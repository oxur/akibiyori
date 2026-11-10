use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use akibiyori::{
    BuildOptions, Error, GenerationOptions, ShowcaseOptions, ThemePresentation, build,
    build_with_options, generate, generate_with_options, generate_with_showcase,
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture directory");
    for entry in fs::read_dir(source).expect("read source fixture") {
        let entry = entry.expect("source entry");
        let kind = entry.file_type().expect("source type");
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy real fixture file");
        }
    }
}

fn file_map(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("read generated directory") {
            let entry = entry.expect("generated entry");
            let path = entry.path();
            if entry.file_type().expect("generated file type").is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .expect("relative generated path")
                        .into(),
                    fs::read(path).expect("read generated file"),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn emitted_profile_block<'a>(css: &'a str, family: &str, profile: &str, mode: &str) -> &'a str {
    let selector = format!(
        "html[data-aki-theme=\"{family}\"][data-aki-variant=\"{profile}\"][data-aki-mode=\"{mode}\"] body.framework,\nhtml[data-aki-theme=\"{family}\"][data-aki-variant=\"{profile}\"][data-aki-mode=\"{mode}\"] #nd-sidebar {{\n"
    );
    let start = css
        .find(&selector)
        .unwrap_or_else(|| panic!("missing generated owner block for {profile}/{mode}"));
    let block = &css[start..];
    let end = block
        .find("\n}\n")
        .unwrap_or_else(|| panic!("unterminated generated owner block for {profile}/{mode}"));
    let block = &block[..end + 2];
    let sidebar = format!(
        "html[data-aki-theme=\"{family}\"][data-aki-variant=\"{profile}\"][data-aki-mode=\"{mode}\"] #nd-sidebar"
    );
    assert!(
        block.contains(&sidebar),
        "missing sidebar owner for {profile}/{mode}"
    );
    block
}

fn observatory_copy() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("temporary fixture root");
    let root = temp.path().join("observatory");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory"),
        &root,
    );
    (temp, root)
}

fn config_with_theme(root: &Path, theme: Value) -> PathBuf {
    let config_path = root.join("config.json");
    let config: Value = serde_json::from_slice(&fs::read(&config_path).expect("config bytes"))
        .expect("config JSON");
    let mut config = config;
    config["theme"] = theme;
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&config).expect("serialize test config"),
    )
    .expect("write test config");
    config_path
}

fn theme_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("theme parent")).expect("create theme directory");
    fs::write(path, text).expect("write test manifest");
}

fn native_program(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable).map_or_else(|| fallback.into(), PathBuf::from)
}

#[derive(Clone, Copy)]
enum ThemeFailure<'a> {
    Invalid(&'a str),
    Json(&'a str),
}

fn assert_theme_failure(config: &Path, root: &Path, expected: ThemeFailure<'_>) {
    let absent = root.join("failure-absent");
    let empty = root.join("failure-empty");
    for (output, should_exist) in [(&absent, false), (&empty, true)] {
        if should_exist {
            fs::create_dir(output).expect("create empty output directory");
        }
        let error = generate(config, output, "").unwrap_err();
        match (expected, error) {
            (ThemeFailure::Invalid(needle), Error::Invalid(message)) => assert!(
                message.contains(needle),
                "expected Invalid containing {needle:?}, got {message:?}"
            ),
            (ThemeFailure::Json(needle), Error::Json { source, .. }) => assert!(
                source.to_string().contains(needle),
                "expected Json containing {needle:?}, got {source:?}"
            ),
            (ThemeFailure::Invalid(needle), error) => {
                panic!("expected Invalid containing {needle:?}, got {error:?}")
            }
            (ThemeFailure::Json(needle), error) => {
                panic!("expected Json containing {needle:?}, got {error:?}")
            }
        }
        if should_exist {
            assert!(fs::read_dir(output).unwrap().next().is_none());
        } else {
            assert!(!output.exists());
        }
    }
}

fn write_manifest_value(root: &Path, family: &str, manifest: &Value) -> String {
    let relative = format!("themes/{family}/theme.json");
    theme_file(
        &root.join(&relative),
        &serde_json::to_string(manifest).expect("serialize valid manifest fixture"),
    );
    relative
}

fn two_profile_manifest(family: &str, dark: &str, light: &str) -> Value {
    json!({
        "version":1,"id":family,"extends":"default",
        "variants":[
            {"id":dark,"label":"Dark profile","scheme":"dark","colors":{}},
            {"id":light,"label":"Light profile","scheme":"light","colors":{}}
        ]
    })
}

fn theme_catalog(files: &[String], dark: &str, light: &str) -> Value {
    json!({
        "catalog":files.iter().map(|file|json!({"file":file})).collect::<Vec<_>>(),
        "deploy":{"dark":dark,"light":light}
    })
}

fn local_face_manifest(family: &str) -> Value {
    json!({
        "version":1,"id":family,"extends":"default",
        "defaults":{"fonts":{"body":{"face":"local-sans","families":["Arial"],"generic":"sans-serif"}}},
        "variants":[
            {"id":"local-dark","label":"Local Dark","scheme":"dark","colors":{}},
            {"id":"local-light","label":"Local Light","scheme":"light","colors":{}}
        ],
        "font_faces":[{"id":"local-sans","source":"author/geist.woff2","weight":[1,1000],"style":"normal"}]
    })
}

fn prepare_real_local_font(root: &Path, family: &str) -> (PathBuf, PathBuf) {
    let author = root.join("themes").join(family).join("author");
    fs::create_dir_all(&author).unwrap();
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let font_source = source_root.join("theme/vendor/fonts/geist-latin.woff2");
    let license_source = source_root.join("theme/vendor/licenses/geist-OFL.txt");
    let font_target = author.join("geist.woff2");
    let license_target = author.join("geist-OFL.txt");
    fs::copy(&font_source, &font_target).unwrap();
    fs::copy(&license_source, &license_target).unwrap();
    (font_target, license_target)
}

#[test]
fn omitted_and_explicit_builtin_dark_generate_identical_files() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config.json");
    let temp = TempDir::new().expect("temporary outputs");
    let omitted = temp.path().join("omitted");
    let explicit = temp.path().join("explicit");
    generate(&source, &omitted, "").expect("default theme generation");

    let (input, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({"source":{"builtin":"fumadocs"},"mode":"dark"}),
    );
    let _keep_input = input;
    generate(&config, &explicit, "").expect("explicit builtin generation");
    assert_eq!(file_map(&omitted), file_map(&explicit));
    let compiled = fs::read_to_string(omitted.join("css/akibiyori-theme.css")).unwrap();
    assert!(compiled.contains("data-aki-mode=\"light\""));
    assert!(compiled.contains("data-aki-variant=\"fumadocs-dark\""));
    assert_eq!(
        fs::read(omitted.join("js/akibiyori-mode.js")).unwrap(),
        include_bytes!("../theme/js/akibiyori-mode.js")
    );
    let data: Value =
        serde_json::from_slice(&fs::read(omitted.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["default_mode"], "dark");
    assert_eq!(data["mode"], "dark");
    assert_eq!(data["preference_key"], "akibiyori:mode:v1:/");
    let html = fs::read_to_string(omitted.join("_layouts/default.liquid"))
        .expect("generated default layout");
    assert!(html.contains("class=\"{{ site.data.akibiyori_theme.mode | escape }}\""));
    assert!(html.contains("data-aki-theme=\"{{ site.data.akibiyori_theme.id | escape }}\""));
    assert!(html.contains("data-aki-mode=\"{{ site.data.akibiyori_theme.mode | escape }}\""));
    assert!(html.contains(
        "data-aki-default-mode=\"{{ site.data.akibiyori_theme.default_mode | escape }}\""
    ));
    assert!(
        html.contains(
            "data-aki-mode-key=\"{{ site.data.akibiyori_theme.preference_key | escape }}\""
        )
    );
    for layout in ["default.liquid", "search.liquid"] {
        let html = fs::read_to_string(omitted.join("_layouts").join(layout)).unwrap();
        let mode_script = html.find("/js/akibiyori-mode.js").unwrap();
        let first_stylesheet = html.find("<link rel=\"stylesheet\"").unwrap();
        assert!(
            first_stylesheet < mode_script,
            "static stylesheets must be parsed before the mode adapter"
        );
        assert!(mode_script < html.find("</head>").unwrap());
        assert_eq!(html.matches("data-aki-mode-picker").count(), 1);
        assert!(html.contains("<label class=\"mode-control\" data-pagefind-ignore hidden>"));
        assert!(html.contains("<select id=\"aki-mode-picker\" data-aki-mode-picker>"));
    }
}

#[test]
fn mode_preference_metadata_uses_canonical_root_and_prefix_keys() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config.json");
    let temp = TempDir::new().expect("preference-key outputs");
    for (base, expected) in [
        ("", "akibiyori:mode:v1:/"),
        ("/mounted/docs", "akibiyori:mode:v1:/mounted/docs"),
    ] {
        let output = temp
            .path()
            .join(if base.is_empty() { "root" } else { "prefix" });
        generate(&source, &output, base).expect("native theme metadata generation");
        let data: Value =
            serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
                .unwrap();
        assert_eq!(data["preference_key"], expected);
        assert_eq!(data["default_mode"], "dark");
        assert_eq!(data["mode"], "dark");
    }
}

#[test]
fn custom_dark_theme_projects_declared_roles_to_both_owners_and_links_after_adapter() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":"dark"}),
    );
    let output = root.join("output");
    generate(&config, &output, "/night").expect("custom theme generation");

    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(css.contains(
        "html[data-aki-theme=\"neutral-example\"][data-aki-variant=\"neutral-example-dark\"][data-aki-mode=\"dark\"] body.framework"
    ));
    assert!(
        css.contains(
            "html[data-aki-theme=\"neutral-example\"][data-aki-variant=\"neutral-example-dark\"][data-aki-mode=\"dark\"] #nd-sidebar"
        )
    );
    assert!(css.contains("--color-fd-primary: #fafafa;"));
    assert!(css.contains("--aki-color-action: #fafafa;"));
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["stylesheet_href"], "/night/css/akibiyori-theme.css");
    assert_eq!(
        data["author_stylesheet_href"],
        "/night/themes/neutral-example/styles.css"
    );
    assert_eq!(
        fs::read(output.join("themes/neutral-example/styles.css")).unwrap(),
        fs::read(root.join("themes/neutral-example/overrides.css")).unwrap()
    );
    for layout in ["default.liquid", "search.liquid"] {
        let html = fs::read_to_string(output.join("_layouts").join(layout)).unwrap();
        assert!(html.contains("site.data.akibiyori_theme.id | escape"));
        let adapter = html.find("/css/akibiyori.css").unwrap();
        let theme = html.find("akibiyori_theme.stylesheet_href").unwrap();
        let pagefind = html.find("/pagefind/pagefind-ui.css");
        let author = html.find("akibiyori_theme.author_stylesheet_href").unwrap();
        assert!(adapter < theme && theme < author);
        if let Some(pagefind) = pagefind {
            assert!(pagefind < theme && theme < author);
        }
    }
}

#[test]
fn every_documented_color_role_is_accepted_and_projected_when_mapped() {
    const ROLES: [&str; 40] = [
        "--aki-color-text",
        "--aki-color-text-muted",
        "--aki-color-surface-page",
        "--aki-color-surface-panel",
        "--aki-color-text-panel",
        "--aki-color-surface-raised",
        "--aki-color-surface-secondary",
        "--aki-color-border",
        "--aki-color-focus-ring",
        "--aki-color-action",
        "--aki-color-hover",
        "--aki-color-text-hover",
        "--aki-color-method-get",
        "--aki-color-method-post",
        "--aki-color-method-put",
        "--aki-color-method-patch",
        "--aki-color-method-delete",
        "--aki-color-property-required",
        "--aki-color-action-tint",
        "--aki-color-hover-tint",
        "--aki-color-secondary-tint",
        "--aki-color-toc-active-tint",
        "--aki-color-overlay-menu-shadow",
        "--aki-color-overlay-sidebar-shadow",
        "--aki-color-overlay-dialog-shadow",
        "--aki-color-overlay-dialog-backdrop",
        "--aki-color-code-article-foreground",
        "--aki-color-code-article-string",
        "--aki-color-code-article-punctuation",
        "--aki-color-code-article-keyword",
        "--aki-color-code-article-name",
        "--aki-color-code-article-constant",
        "--aki-color-code-article-operator",
        "--aki-color-code-api-foreground",
        "--aki-color-code-api-punctuation",
        "--aki-color-code-api-key-number-option",
        "--aki-color-code-api-string-value",
        "--aki-color-surface-frame",
        "--aki-color-code-api-command-option",
        "--aki-color-code-api-delimiter",
    ];
    let (_temp, root) = observatory_copy();
    let colors: serde_json::Map<String, Value> = ROLES
        .iter()
        .map(|role| ((*role).to_owned(), json!("#abc")))
        .collect();
    theme_file(
        &root.join("themes/neutral-example/theme.json"),
        &serde_json::to_string(&json!({
            "version":1,"id":"neutral-example","extends":"fumadocs",
            "modes":{"dark":{"colors":colors}}
        }))
        .unwrap(),
    );
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"}}),
    );
    let output = root.join("output");
    generate(&config, &output, "").expect("all 40 allowed roles compile");
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    for role in ROLES {
        assert!(css.contains(&format!("{role}: #abc;")), "missing {role}");
    }
    for fd in [
        "--color-fd-foreground",
        "--color-fd-muted-foreground",
        "--color-fd-background",
        "--color-fd-card",
        "--color-fd-card-foreground",
        "--color-fd-popover",
        "--color-fd-secondary",
        "--color-fd-border",
        "--color-fd-ring",
        "--color-fd-primary",
        "--color-fd-accent",
        "--color-fd-accent-foreground",
    ] {
        assert!(
            css.contains(&format!("{fd}: #abc;")),
            "missing projection {fd}"
        );
    }
}

#[test]
fn bundled_house_resolves_all_selected_colors_from_an_external_root() {
    let (_temp, root) = observatory_copy();
    fs::remove_dir_all(root.join("themes"))
        .expect("remove repository theme manifests from author root");
    let config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"house"}],
            "deploy":{"dark":"ember-ink-dark","light":"maple-mist-light"},
            "mode":"system",
            "presentation":"demo",
            "demo_default":"maple-mist-light"
        }),
    );
    let output = root.join("house-output");
    generate(&config, &output, "").expect("embedded House works without repository theme files");

    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["id"], "house");
    assert_eq!(data["dark_variant"], "ember-ink-dark");
    assert_eq!(data["light_variant"], "maple-mist-light");
    assert_eq!(data["presentation"], "demo");
    let profiles = data["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 6);
    let expected = [
        ("maple-mist-light", "Maple & Mist Light", "light"),
        ("maple-mist-dark", "Maple & Mist Dark", "dark"),
        ("golden-hour-light", "Golden Hour Light", "light"),
        ("golden-hour-dark", "Golden Hour Dark", "dark"),
        ("ember-ink-light", "Ember & Ink Light", "light"),
        ("ember-ink-dark", "Ember & Ink Dark", "dark"),
    ];
    for (profile, (id, label, mode)) in profiles.iter().zip(expected) {
        assert_eq!(profile["id"], id);
        assert_eq!(profile["label"], label);
        assert_eq!(profile["mode"], mode);
    }

    let manifest: Value = serde_json::from_slice(include_bytes!("../theme/profiles/house.json"))
        .expect("bundled profile manifest is JSON");
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    for (profile, (id, _, mode)) in manifest["variants"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected)
    {
        assert_eq!(profile["id"], id);
        let block = emitted_profile_block(&css, "house", id, mode);
        let colors = profile["colors"].as_object().unwrap();
        assert_eq!(colors.len(), 32, "{id} has the complete explicit role set");
        for (role, value) in colors {
            let declaration = format!("{role}: {};", value.as_str().unwrap());
            assert!(
                block.contains(&declaration),
                "{id} body/sidebar block misses {declaration}"
            );
        }
    }

    let mut font_files = fs::read_dir(output.join("fonts"))
        .expect("read bundled House font inventory")
        .map(|entry| {
            entry
                .expect("font entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    font_files.sort();
    assert_eq!(
        font_files,
        ["geist-latin.woff2", "jetbrains-mono-latin.woff2"]
    );
    let font_css = fs::read_to_string(output.join("css/akibiyori-fonts.css"))
        .expect("read bundled House font CSS");
    assert_eq!(font_css.matches("url(\"../fonts/").count(), 2);
    assert!(font_css.contains("font-family: Geist;"));
    assert!(font_css.contains("font-family: \"JetBrains Mono\";"));
    for (name, bytes) in [
        (
            "geist-latin.woff2",
            include_bytes!("../theme/vendor/fonts/geist-latin.woff2").as_slice(),
        ),
        (
            "jetbrains-mono-latin.woff2",
            include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2").as_slice(),
        ),
    ] {
        assert_eq!(fs::read(output.join("fonts").join(name)).unwrap(), bytes);
        assert!(font_css.contains(name));
    }
}

#[test]
fn bundled_forest_resolves_four_profiles_and_trusted_css_from_an_external_root() {
    let (_temp, root) = observatory_copy();
    fs::remove_dir_all(root.join("themes")).expect("remove checkout theme inputs");
    let config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"forest"}],
            "deploy":{"dark":"forest-storm","light":"forest-light"},
            "mode":"system",
            "presentation":"demo",
            "demo_default":"forest-light"
        }),
    );
    let output = root.join("forest-output");
    generate(&config, &output, "").expect("embedded Forest works without checkout theme files");
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["id"], "forest");
    let expected = [
        ("forest-light", "Forest Light", "light"),
        ("forest-dusk", "Forest Dusk", "dark"),
        ("forest-storm", "Forest Storm", "dark"),
        ("forest-midnight", "Forest Midnight", "dark"),
    ];
    let profiles = data["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), expected.len());
    for (profile, (id, label, mode)) in profiles.iter().zip(expected) {
        assert_eq!(profile["id"], id);
        assert_eq!(profile["label"], label);
        assert_eq!(profile["mode"], mode);
        assert_eq!(profile["family"], "forest");
    }
    let manifest: Value =
        serde_json::from_slice(include_bytes!("../theme/profiles/forest.json")).unwrap();
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    for (profile, (id, _, mode)) in manifest["variants"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected)
    {
        let block = emitted_profile_block(&css, "forest", id, mode);
        let colors = profile["colors"].as_object().unwrap();
        assert_eq!(colors.len(), 32);
        for (role, value) in colors {
            assert!(
                block.contains(&format!("{role}: {};", value.as_str().unwrap())),
                "{id} missing {role}"
            );
        }
    }
    for (id, _, mode) in expected {
        let block = emitted_profile_block(&css, "forest", id, mode);
        assert!(block.contains("--aki-font-heading: \"Georgia\", \"Times New Roman\", serif;"));
    }
    assert_eq!(
        fs::read(output.join("themes/forest/styles.css")).unwrap(),
        include_bytes!("../theme/profiles/forest.css")
    );
    let family_css = fs::read_to_string(output.join("themes/forest/styles.css")).unwrap();
    assert_eq!(family_css.matches("--aki-callout-").count(), 72);
    for (id, _, mode) in expected {
        assert!(family_css.contains(&format!(
            "html[data-aki-theme=\"forest\"][data-aki-variant=\"{id}\"][data-aki-mode=\"{mode}\"]"
        )));
    }
    let mut font_files = fs::read_dir(output.join("fonts"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    font_files.sort();
    assert_eq!(
        font_files,
        ["geist-latin.woff2", "jetbrains-mono-latin.woff2"]
    );
    assert_eq!(
        fs::read(output.join("fonts/geist-latin.woff2")).unwrap(),
        include_bytes!("../theme/vendor/fonts/geist-latin.woff2")
    );
    assert_eq!(
        fs::read(output.join("fonts/jetbrains-mono-latin.woff2")).unwrap(),
        include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2")
    );
}

#[test]
fn forest_dark_pair_variants_are_independent_and_catalog_stylesheets_follow_selection() {
    let dark_ids = ["forest-dusk", "forest-storm", "forest-midnight"];
    for dark in dark_ids {
        let (_temp, root) = observatory_copy();
        fs::remove_dir_all(root.join("themes")).expect("remove checkout theme inputs");
        let config = config_with_theme(
            &root,
            json!({
                "catalog":[{"builtin":"forest"}],
                "deploy":{"dark":dark,"light":"forest-light"},
                "mode":"dark"
            }),
        );
        let output = root.join("selected");
        generate(&config, &output, "").expect("each Forest dark pairs with Forest Light");
        let data: Value =
            serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
                .unwrap();
        assert_eq!(data["profiles"].as_array().unwrap().len(), 2);
        assert_eq!(data["dark_variant"], dark);
        assert_eq!(data["light_variant"], "forest-light");
        let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
        assert!(css.contains(&format!("data-aki-variant=\"{dark}\"")));
        assert!(css.contains("data-aki-variant=\"forest-light\""));
        for other in dark_ids.into_iter().filter(|id| *id != dark) {
            assert!(!css.contains(&format!("data-aki-variant=\"{other}\"")));
        }
        assert!(output.join("themes/forest/styles.css").is_file());
    }

    let (_temp, root) = observatory_copy();
    fs::remove_dir_all(root.join("themes")).expect("remove checkout theme inputs");
    let config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"forest"},{"builtin":"house"}],
            "deploy":{"dark":"ember-ink-dark","light":"maple-mist-light"},
            "mode":"dark"
        }),
    );
    let output = root.join("house-selected");
    generate(&config, &output, "").expect("House pair resolves without loading unused Forest CSS");
    assert!(!output.join("themes/forest/styles.css").exists());
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert!(
        data["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|profile| profile["family"] == "house")
    );

    let wrong_scheme = config_with_theme(
        &root,
        json!({"catalog":[{"builtin":"forest"}],"deploy":{"dark":"forest-light","light":"forest-dusk"}}),
    );
    assert_theme_failure(
        &wrong_scheme,
        &root,
        ThemeFailure::Invalid("deploy pair must reference"),
    );
    let (_unknown_temp, unknown_root) = observatory_copy();
    let unknown = config_with_theme(
        &unknown_root,
        json!({"catalog":[{"builtin":"forest"}],"deploy":{"dark":"forest-unknown","light":"forest-light"}}),
    );
    assert_theme_failure(
        &unknown,
        &unknown_root,
        ThemeFailure::Invalid("unknown deploy dark variant"),
    );
}

#[test]
fn house_legacy_source_is_rejected_before_output_mutation() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(&root, json!({"source":{"builtin":"house"}}));
    assert_theme_failure(
        &config,
        &root,
        ThemeFailure::Invalid("named catalog family"),
    );
}

#[test]
fn forest_legacy_source_is_rejected_before_output_mutation() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(&root, json!({"source":{"builtin":"forest"}}));
    assert_theme_failure(
        &config,
        &root,
        ThemeFailure::Invalid("named catalog family"),
    );
}

#[test]
fn central_showcase_deploy_override_keeps_the_selected_house_pair() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("akibiyori-docs.json");
    let showcase = root.join("akibiyori-showcase.json");
    let temp = TempDir::new().expect("central House override output");
    let output = temp.path().join("deploy");
    let options = GenerationOptions::default()
        .with_theme_mode(ThemePresentation::Deploy)
        .with_showcase(ShowcaseOptions {
            manifest: showcase,
            site: "docs".to_owned(),
        });
    generate_with_options(&config, &output, "", &options)
        .expect("real composed showcase accepts Deploy override");
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["presentation"], "deploy");
    assert_eq!(data["id"], "house");
    assert_eq!(data["dark_variant"], "ember-ink-dark");
    assert_eq!(data["light_variant"], "maple-mist-light");
    assert_eq!(data["profiles"].as_array().unwrap().len(), 2);
}

#[test]
fn house_catalog_deploy_supports_an_independent_dark_light_pair() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"house"}],
            "deploy":{"dark":"maple-mist-dark","light":"golden-hour-light"},
            "mode":"system"
        }),
    );
    let output = root.join("alternate-house-deploy");
    generate(&config, &output, "").expect("House accepts an independent valid Deploy pair");
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["presentation"], "deploy");
    assert_eq!(data["dark_variant"], "maple-mist-dark");
    assert_eq!(data["light_variant"], "golden-hour-light");
    assert_eq!(data["profiles"].as_array().unwrap().len(), 2);
    assert_eq!(data["profiles"][0]["id"], "maple-mist-dark");
    assert_eq!(data["profiles"][1]["id"], "golden-hour-light");
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    for id in ["maple-mist-dark", "golden-hour-light"] {
        assert!(css.contains(&format!("data-aki-variant=\"{id}\"")));
    }
    for id in [
        "maple-mist-light",
        "golden-hour-dark",
        "ember-ink-light",
        "ember-ink-dark",
    ] {
        assert!(!css.contains(&format!("data-aki-variant=\"{id}\"")));
    }
    let mut font_files = fs::read_dir(output.join("fonts"))
        .expect("read alternate Deploy font inventory")
        .map(|entry| {
            entry
                .expect("font entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    font_files.sort();
    assert_eq!(
        font_files,
        ["geist-latin.woff2", "jetbrains-mono-latin.woff2"]
    );
}

#[test]
fn light_mode_uses_builtin_palette_inherits_and_applies_only_selected_overrides() {
    let (_temp, root) = observatory_copy();
    let builtin_config = config_with_theme(
        &root,
        json!({"source":{"builtin":"fumadocs"},"mode":"light"}),
    );
    let builtin_output = root.join("builtin-light");
    generate(&builtin_config, &builtin_output, "").expect("built-in Light generation");
    let builtin_css = fs::read_to_string(builtin_output.join("css/akibiyori-theme.css")).unwrap();
    assert!(builtin_css.contains(
        "html[data-aki-theme=\"fumadocs\"][data-aki-variant=\"fumadocs-light\"][data-aki-mode=\"light\"] { color-scheme: light; }"
    ));
    assert!(builtin_css.contains("--color-fd-background: #f5f5f5;"));
    assert!(builtin_css.contains("--color-fd-primary: #cc8b00;"));
    assert!(builtin_css.contains("data-aki-mode=\"light\""));
    assert!(builtin_css.contains("data-aki-variant=\"fumadocs-dark\""));

    let manifest_path = root.join("themes/neutral-example/theme.json");
    theme_file(
        &manifest_path,
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"#112233"}}}}"##,
    );
    let inherited_config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":"light"}),
    );
    let inherited_output = root.join("inherited-light");
    generate(&inherited_config, &inherited_output, "").expect("inherited Light generation");
    let inherited_css =
        fs::read_to_string(inherited_output.join("css/akibiyori-theme.css")).unwrap();
    let (dark_block, light_block) = inherited_css
        .split_once("html[data-aki-theme=\"neutral-example\"][data-aki-variant=\"neutral-example-light\"][data-aki-mode=\"light\"]")
        .expect("compiled Light block follows Dark block");
    assert!(dark_block.contains("--aki-color-action: #112233;"));
    assert!(dark_block.contains("--color-fd-primary: #112233;"));
    assert!(light_block.contains("--color-fd-primary: #cc8b00;"));
    assert!(!light_block.contains("--aki-color-action: #112233;"));
    assert!(!light_block.contains("--color-fd-primary: #112233;"));

    theme_file(
        &manifest_path,
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"#112233"}},"light":{"colors":{"--aki-color-action":"#abcdef","--aki-color-code-api-command-option":"#123456","--aki-color-code-api-delimiter":"#654321","--aki-color-surface-frame":"#fedcba"}}}}"##,
    );
    let override_config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":"light"}),
    );
    let override_output = root.join("override-light");
    generate(&override_config, &override_output, "").expect("Light overrides generation");
    let override_css = fs::read_to_string(override_output.join("css/akibiyori-theme.css")).unwrap();
    assert!(override_css.contains("--color-fd-primary: #abcdef;"));
    assert!(override_css.contains("--aki-color-action: #abcdef;"));
    assert!(override_css.contains("--aki-color-code-api-command-option: #123456;"));
    assert!(override_css.contains("--aki-color-code-api-delimiter: #654321;"));
    assert!(override_css.contains("--aki-color-surface-frame: #fedcba;"));
    assert!(override_css.contains("data-aki-mode=\"dark\""));
    assert!(override_css.contains("--aki-color-action: #112233;"));
    assert!(override_css.contains("data-aki-mode=\"light\""));
    assert!(
        override_css.find("data-aki-mode=\"dark\"").unwrap()
            < override_css.find("data-aki-mode=\"light\"").unwrap()
    );

    for layout in ["default.liquid", "search.liquid"] {
        let html = fs::read_to_string(override_output.join("_layouts").join(layout)).unwrap();
        assert!(html.contains("class=\"{{ site.data.akibiyori_theme.mode | escape }}\""));
        assert!(!html.contains("class=\"dark\""));
    }

    let adapter =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("theme/css/akibiyori.css"))
            .unwrap();
    assert!(
        adapter
            .contains(".c-punctuation.c-section { color: var(--aki-color-code-api-delimiter); }")
    );
    assert!(adapter.contains(
        ".c-variable.c-function.c-shell { color: var(--aki-color-code-api-command-option); }"
    ));
    assert!(adapter.contains(".c-punctuation { color: var(--aki-color-code-api-punctuation); }"));
    assert!(adapter.contains(
        "html[data-aki-theme=\"house\"] .search-ui .pagefind-ui__search-input::placeholder,\nhtml[data-aki-theme=\"house\"] .property-filter::placeholder,\nhtml[data-aki-theme=\"forest\"] .search-ui .pagefind-ui__search-input::placeholder,\nhtml[data-aki-theme=\"forest\"] .property-filter::placeholder { color: var(--aki-color-text-muted); opacity: 1; }"
    ));
}

#[test]
fn light_mode_manifest_is_strict_and_null_is_not_omission() {
    for manifest in [
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"light":null}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"light":{}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"light":{"colors":null}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"light":{"colors":{},"unknown":true}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"light":{"colors":{},"colors":{}}}}"#,
    ] {
        let (_temp, root) = observatory_copy();
        theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":"light"}),
        );
        let output = root.join("output");
        assert!(
            generate(&config, &output, "").is_err(),
            "manifest {manifest}"
        );
        assert!(
            !output.exists(),
            "invalid Light manifest must fail before output"
        );
    }
}

#[test]
fn theme_rejections_happen_before_absent_or_empty_output_changes() {
    for manifest in [
        r#"{"version":2,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"../bad","extends":"fumadocs","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"other","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"red"}}}}"#,
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-unknown":"#abc"}}}}"##,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"stylesheet":null}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"light":{"colors":{}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}},"system":{"colors":{}}}}"#,
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"#abc","--aki-color-action":"#def"}}}}"##,
    ] {
        let (_temp, root) = observatory_copy();
        theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"}}),
        );
        let absent = root.join("absent");
        assert!(generate(&config, &absent, "").is_err());
        assert!(
            !absent.exists(),
            "invalid input leaves absent output absent"
        );
        let empty = root.join("empty");
        fs::create_dir(&empty).unwrap();
        assert!(generate(&config, &empty, "").is_err());
        assert!(fs::read_dir(&empty).unwrap().next().is_none());
    }
}

#[test]
fn duplicate_typed_map_keys_fail_through_the_real_generator_before_writes() {
    let manifests = [
        r##"{
          "version":1,"id":"neutral-example","extends":"fumadocs",
          "modes":{"dark":{"colors":{}}},
          "defaults":{"fonts":{
            "body":{"families":["Arial"],"generic":"sans-serif"},
            "body":{"families":["Georgia"],"generic":"serif"}
          }}
        }"##,
        r##"{
          "version":1,"id":"neutral-example","extends":"fumadocs",
          "modes":{"dark":{"colors":{}}},
          "defaults":{"geometry":{"radii":{
            "panel":{"value":1,"unit":"rem"},
            "panel":{"value":2,"unit":"rem"}
          }}}
        }"##,
        r##"{
          "version":1,"id":"neutral-example","extends":"fumadocs",
          "modes":{"dark":{"colors":{}}},
          "defaults":{"geometry":{"shadows":{
            "menu":{"x":{"value":0,"unit":"px"},"y":{"value":1,"unit":"px"},"blur":{"value":2,"unit":"px"},"spread":{"value":0,"unit":"px"}},
            "menu":{"x":{"value":0,"unit":"px"},"y":{"value":2,"unit":"px"},"blur":{"value":3,"unit":"px"},"spread":{"value":0,"unit":"px"}}
          }}}
        }"##,
    ];
    for manifest in manifests {
        let (_temp, root) = observatory_copy();
        theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"}}),
        );
        let absent = root.join("absent");
        let error = generate(&config, &absent, "").unwrap_err();
        assert!(
            format!("{error:?}").contains("duplicate map key"),
            "wrong rejection for {manifest}: {error:?}"
        );
        assert!(!absent.exists(), "invalid map leaves absent output absent");
        let empty = root.join("empty");
        fs::create_dir(&empty).unwrap();
        let error = generate(&config, &empty, "").unwrap_err();
        assert!(
            format!("{error:?}").contains("duplicate map key"),
            "wrong rejection for {manifest}: {error:?}"
        );
        assert!(
            fs::read_dir(&empty).unwrap().next().is_none(),
            "invalid map leaves empty output untouched"
        );
    }

    let (_temp, root) = observatory_copy();
    let named = r##"{
      "version":1,"id":"neutral-example","extends":"default",
      "variants":[
        {"id":"neutral-dark","label":"Neutral Dark","scheme":"dark","colors":{},"fonts":{
          "code":{"families":["Mono"],"generic":"monospace"},
          "code":{"families":["Other Mono"],"generic":"monospace"}
        }},
        {"id":"neutral-light","label":"Neutral Light","scheme":"light","colors":{}}
      ]
    }"##;
    theme_file(&root.join("themes/neutral-example/theme.json"), named);
    let config = config_with_theme(
        &root,
        json!({"catalog":[{"file":"themes/neutral-example/theme.json"}],"deploy":{"dark":"neutral-dark","light":"neutral-light"}}),
    );
    let output = root.join("output");
    let error = generate(&config, &output, "").unwrap_err();
    assert!(
        format!("{error:?}").contains("duplicate map key"),
        "wrong named font rejection: {error:?}"
    );
    assert!(
        !output.exists(),
        "invalid named font map fails before output"
    );
}

#[test]
fn catalog_selection_and_manifest_guards_report_categories_before_writes() {
    let selection_cases = [
        (
            json!({"source":{"builtin":"fumadocs"},"catalog":[{"builtin":"fumadocs"}],"deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"}}),
            ThemeFailure::Invalid("mutually exclusive"),
        ),
        (
            json!({"catalog":[],"deploy":{"dark":"dark-one","light":"light-one"}}),
            ThemeFailure::Invalid("at least one family"),
        ),
        (
            json!({"catalog":[{"builtin":"fumadocs"}]}),
            ThemeFailure::Invalid("explicit theme.deploy"),
        ),
        (
            json!({"catalog":[{"builtin":"fumadocs"}],"deploy":{"dark":"missing-dark","light":"fumadocs-light"}}),
            ThemeFailure::Invalid("unknown deploy dark variant"),
        ),
        (
            json!({"catalog":[{"builtin":"fumadocs"}],"deploy":{"dark":"fumadocs-light","light":"fumadocs-light"}}),
            ThemeFailure::Invalid("dark variant and a light variant"),
        ),
        (
            json!({"catalog":[{"builtin":"fumadocs"},{"builtin":"fumadocs"}],"deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"}}),
            ThemeFailure::Invalid("duplicate theme family id"),
        ),
    ];
    for (theme, expected) in selection_cases {
        let (_temp, root) = observatory_copy();
        let config = config_with_theme(&root, theme);
        assert_theme_failure(&config, &root, expected);
    }

    let (_temp, root) = observatory_copy();
    let first = write_manifest_value(
        &root,
        "family-one",
        &two_profile_manifest("family-one", "shared-dark", "one-light"),
    );
    let second = write_manifest_value(
        &root,
        "family-two",
        &two_profile_manifest("family-two", "shared-dark", "two-light"),
    );
    let config = config_with_theme(
        &root,
        theme_catalog(&[first, second], "shared-dark", "one-light"),
    );
    assert_theme_failure(
        &config,
        &root,
        ThemeFailure::Invalid("duplicate global theme variant id"),
    );

    for (manifest, expected) in [
        (
            json!({"version":1,"id":"local-family","extends":"default","modes":{"dark":{"colors":{}}},"variants":[{"id":"local-dark","label":"Dark","scheme":"dark","colors":{}}]}),
            ThemeFailure::Invalid("exactly one of modes or variants"),
        ),
        (
            json!({"version":1,"id":"local-family","extends":"default","variants":[]}),
            ThemeFailure::Invalid("theme variants must not be empty"),
        ),
        (
            json!({"version":1,"id":"local-family","extends":"default","modes":{"dark":{"colors":{}}},"defaults":null}),
            ThemeFailure::Json("invalid type: null"),
        ),
        (
            json!({"version":1,"id":"local-family","extends":"default","modes":{"dark":{"colors":{}}},"defaults":{"unexpected":true}}),
            ThemeFailure::Json("unknown field"),
        ),
    ] {
        let (_temp, root) = observatory_copy();
        let file = write_manifest_value(&root, "local-family", &manifest);
        let config = config_with_theme(
            &root,
            json!({"catalog":[{"file":file}],"deploy":{"dark":"local-dark","light":"local-light"}}),
        );
        assert_theme_failure(&config, &root, expected);
    }
}

#[test]
fn typed_geometry_and_font_range_guards_reject_before_writes() {
    let geometry_cases = [
        (
            "border_width",
            json!({"value":1,"unit":"em"}),
            ThemeFailure::Invalid("invalid border width length"),
        ),
        (
            "radius",
            json!({"value":-0.1,"unit":"rem"}),
            ThemeFailure::Invalid("invalid radius length"),
        ),
        (
            "shadow",
            json!({"x":{"value":0,"unit":"px"},"y":{"value":0,"unit":"px"},"blur":{"value":-1,"unit":"px"},"spread":{"value":0,"unit":"px"}}),
            ThemeFailure::Invalid("invalid shadow blur length"),
        ),
        (
            "unknown_radius",
            json!({"panel-new":{"value":1,"unit":"rem"}}),
            ThemeFailure::Invalid("unknown geometry radius"),
        ),
        (
            "unknown_shadow",
            json!({"popover":{"x":{"value":0,"unit":"px"},"y":{"value":0,"unit":"px"},"blur":{"value":1,"unit":"px"},"spread":{"value":0,"unit":"px"}}}),
            ThemeFailure::Invalid("unknown geometry shadow"),
        ),
    ];
    for (kind, value, expected) in geometry_cases {
        let (_temp, root) = observatory_copy();
        let mut manifest =
            two_profile_manifest("geometry-family", "geometry-dark", "geometry-light");
        if manifest.get("defaults").is_none() {
            manifest["defaults"] = json!({});
        }
        match kind {
            "border_width" => manifest["defaults"]["geometry"]["border_width"] = value,
            "radius" => manifest["defaults"]["geometry"]["radii"] = json!({"panel":value}),
            "shadow" => manifest["defaults"]["geometry"]["shadows"] = json!({"menu":value}),
            "unknown_radius" => manifest["defaults"]["geometry"]["radii"] = value,
            "unknown_shadow" => manifest["defaults"]["geometry"]["shadows"] = value,
            _ => unreachable!(),
        }
        let file = write_manifest_value(&root, "geometry-family", &manifest);
        let config = config_with_theme(
            &root,
            theme_catalog(&[file], "geometry-dark", "geometry-light"),
        );
        assert_theme_failure(&config, &root, expected);
    }

    let face_cases = [
        (
            "unknown_reference",
            json!({"id":"missing-face","source":"author/geist.woff2","weight":[1,1000],"style":"normal"}),
            ThemeFailure::Invalid("undeclared face family"),
        ),
        (
            "zero_weight",
            json!({"id":"local-sans","source":"author/geist.woff2","weight":[0,1000],"style":"normal"}),
            ThemeFailure::Invalid("invalid font face range/style"),
        ),
        (
            "too_high",
            json!({"id":"local-sans","source":"author/geist.woff2","weight":[1,1001],"style":"normal"}),
            ThemeFailure::Invalid("invalid font face range/style"),
        ),
        (
            "reversed",
            json!({"id":"local-sans","source":"author/geist.woff2","weight":[900,400],"style":"normal"}),
            ThemeFailure::Invalid("invalid font face range/style"),
        ),
        (
            "unsupported_style",
            json!({"id":"local-sans","source":"author/geist.woff2","weight":[1,1000],"style":"oblique"}),
            ThemeFailure::Invalid("invalid font face range/style"),
        ),
    ];
    for (kind, face, expected) in face_cases {
        let (_temp, root) = observatory_copy();
        prepare_real_local_font(&root, "local-family");
        let mut manifest = local_face_manifest("local-family");
        manifest["font_faces"] = json!([face]);
        if kind == "unknown_reference" {
            manifest["defaults"]["fonts"]["body"]["face"] = json!("undeclared-face");
        }
        let file = write_manifest_value(&root, "local-family", &manifest);
        let config = config_with_theme(&root, theme_catalog(&[file], "local-dark", "local-light"));
        assert_theme_failure(&config, &root, expected);
    }

    for (first, second) in [
        ([1, 1000], [1, 1000]),
        ([1, 600], [500, 1000]),
        ([1, 500], [500, 1000]),
    ] {
        let (_temp, root) = observatory_copy();
        prepare_real_local_font(&root, "local-family");
        let mut manifest = local_face_manifest("local-family");
        manifest["font_faces"][0]["weight"] = json!(first);
        let mut second_face = manifest["font_faces"][0].clone();
        second_face["weight"] = json!(second);
        manifest["font_faces"]
            .as_array_mut()
            .unwrap()
            .push(second_face);
        let file = write_manifest_value(&root, "local-family", &manifest);
        let config = config_with_theme(&root, theme_catalog(&[file], "local-dark", "local-light"));
        assert_theme_failure(
            &config,
            &root,
            ThemeFailure::Invalid("overlapping font face range"),
        );
    }
}

#[test]
fn font_source_and_generated_target_guards_reject_real_inputs_before_writes() {
    let (_temp, root) = observatory_copy();
    prepare_real_local_font(&root, "local-family");
    let mut unsafe_source = local_face_manifest("local-family");
    unsafe_source["font_faces"][0]["source"] = json!("../escape.woff2");
    let file = write_manifest_value(&root, "local-family", &unsafe_source);
    let config = config_with_theme(&root, theme_catalog(&[file], "local-dark", "local-light"));
    assert_theme_failure(
        &config,
        &root,
        ThemeFailure::Invalid("relative path without dot segments"),
    );

    for target in [
        "fonts/local-sans-normal-1-1000.woff2",
        "fonts/local-sans-normal-1-1000.woff2/child.txt",
    ] {
        let (_temp, root) = observatory_copy();
        prepare_real_local_font(&root, "local-family");
        let family_dir = root.join("themes/local-family");
        fs::write(family_dir.join("author/notice.txt"), b"real notice bytes").unwrap();
        fs::write(family_dir.join("author/child.txt"), b"real child bytes").unwrap();
        let mut manifest = local_face_manifest("local-family");
        manifest["assets"] = json!([{"source":"author/notice.txt","target":target}]);
        if target.ends_with("/child.txt") {
            manifest["assets"] = json!([{"source":"author/child.txt","target":target}]);
        }
        let file = write_manifest_value(&root, "local-family", &manifest);
        let config = config_with_theme(&root, theme_catalog(&[file], "local-dark", "local-light"));
        assert_theme_failure(
            &config,
            &root,
            ThemeFailure::Invalid("font face target collision"),
        );
    }

    #[cfg(unix)]
    {
        let (_temp, root) = observatory_copy();
        let real_font = prepare_real_local_font(&root, "local-family").0;
        let symlink = root.join("themes/local-family/author/symlink.woff2");
        std::os::unix::fs::symlink(&real_font, &symlink).unwrap();
        let mut manifest = local_face_manifest("local-family");
        manifest["font_faces"][0]["source"] = json!("author/symlink.woff2");
        let file = write_manifest_value(&root, "local-family", &manifest);
        let config = config_with_theme(&root, theme_catalog(&[file], "local-dark", "local-light"));
        assert_theme_failure(
            &config,
            &root,
            ThemeFailure::Invalid("must not traverse symbolic links"),
        );
    }
}

#[test]
fn selected_local_pair_accepts_zero_and_weight_boundaries_and_omits_other_dependencies() {
    let (_temp, root) = observatory_copy();
    let (font_source, license_source) = prepare_real_local_font(&root, "selected-theme");
    let selected_dir = root.join("themes/selected-theme");
    let unused_dir = root.join("themes/unused-theme");
    fs::create_dir_all(unused_dir.join("author")).unwrap();
    fs::write(
        unused_dir.join("author/unused.txt"),
        b"unselected family data",
    )
    .unwrap();
    let mut selected = local_face_manifest("selected-theme");
    selected["defaults"]["fonts"] = json!({
        "body":{"face":"local-sans","families":["Quoted \" Font Family"],"generic":"sans-serif"},
        "heading":{"families":["Georgia"],"generic":"serif"},
        "code":{"families":["Courier New"],"generic":"monospace"}
    });
    selected["defaults"]["geometry"] = json!({
        "border_width":{"value":0,"unit":"px"},
        "radii":{"panel":{"value":0,"unit":"rem"}},
        "shadows":{"menu":{"x":{"value":0,"unit":"px"},"y":{"value":0,"unit":"px"},"blur":{"value":0,"unit":"px"},"spread":{"value":0,"unit":"px"}}}
    });
    selected["assets"] =
        json!([{"source":"author/geist-OFL.txt","target":"licenses/geist-OFL.txt"}]);
    let mut unused = two_profile_manifest("unused-theme", "unused-dark", "unused-light");
    unused["assets"] = json!([{"source":"author/unused.txt","target":"files/unused.txt"}]);
    let selected_file = write_manifest_value(&root, "selected-theme", &selected);
    let unused_file = write_manifest_value(&root, "unused-theme", &unused);
    let config = config_with_theme(
        &root,
        theme_catalog(&[selected_file, unused_file], "local-dark", "local-light"),
    );
    let output = root.join("output");
    generate(&config, &output, "").expect("valid boundary pair generates");

    let family_css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(family_css.contains("font-family: \"aki-14-selected-theme-local-sans\";"));
    assert!(family_css.contains("font-weight: 1 1000;"));
    assert!(family_css.contains("font-style: normal;"));
    assert!(!output.join("fonts/geist-latin.woff2").exists());
    assert!(!output.join("fonts/jetbrains-mono-latin.woff2").exists());
    let profile_css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(profile_css.contains(
        r#"--aki-font-body: aki-14-selected-theme-local-sans, "Quoted \" Font Family", sans-serif;"#
    ));
    assert!(profile_css.contains("--aki-font-heading: \"Georgia\", serif;"));
    assert!(profile_css.contains("--aki-font-code: \"Courier New\", monospace;"));
    assert!(profile_css.contains("--aki-border-default: 0px;"));
    assert!(profile_css.contains("--aki-radius-panel: 0rem;"));
    assert!(
        profile_css
            .contains("--aki-shadow-menu: 0px 0px 0px 0px var(--aki-color-overlay-menu-shadow);")
    );
    assert_eq!(
        fs::read(output.join("themes/selected-theme/fonts/local-sans-normal-1-1000.woff2"))
            .unwrap(),
        fs::read(&font_source).unwrap()
    );
    assert_eq!(
        fs::read(output.join("themes/selected-theme/licenses/geist-OFL.txt")).unwrap(),
        fs::read(&license_source).unwrap()
    );
    assert!(!output.join("themes/unused-theme/files/unused.txt").exists());
    assert!(!output.join("themes/unused-theme/styles.css").exists());
    assert!(selected_dir.is_dir());
    assert!(unused_dir.is_dir());
}

#[test]
fn catalog_mixed_pair_and_same_family_dark_profiles_keep_variant_scopes_independent() {
    let pair_config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config-theme-pair.json");
    let pair_output = TempDir::new().unwrap();
    generate(&pair_config, &pair_output.path().join("mixed"), "").unwrap();
    let css = fs::read_to_string(pair_output.path().join("mixed/css/akibiyori-theme.css")).unwrap();
    assert!(css.contains("data-aki-theme=\"neutral-profile\"][data-aki-variant=\"neutral-night\"][data-aki-mode=\"dark\"]"));
    assert!(css.contains(
        "data-aki-theme=\"fumadocs\"][data-aki-variant=\"fumadocs-light\"][data-aki-mode=\"light\"]"
    ));
    assert!(
        !css.contains("data-aki-variant=\"neutral-day\""),
        "unselected catalog profiles are not packaged"
    );
    let data: Value = serde_json::from_slice(
        &fs::read(pair_output.path().join("mixed/_data/akibiyori_theme.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(data["dark_variant"], "neutral-night");
    assert_eq!(data["light_variant"], "fumadocs-light");
    assert_eq!(data["profiles"].as_array().unwrap().len(), 2);
    assert!(css.contains("--aki-font-body: \"Arial\", sans-serif;"));
    assert!(css.contains("--aki-font-heading: \"Georgia\", serif;"));
    assert!(css.contains("--aki-radius-copy-code: 0.5rem;"));

    let (_temp, root) = observatory_copy();
    let variants = r##"{
      "version":1,"id":"neutral-example","extends":"default",
      "defaults":{"fonts":{"body":{"families":["Arial"],"generic":"sans-serif"},"code":{"families":["Mono"],"generic":"monospace"}},"geometry":{"radii":{"panel":{"value":0.5,"unit":"rem"}}}},
      "variants":[
        {"id":"night-one","label":"Night One","scheme":"dark","colors":{"--aki-color-action":"#112233"},"fonts":{"body":{"families":["Courier New"],"generic":"sans-serif"}},"geometry":{"radii":{"panel":{"value":0.75,"unit":"rem"}}}},
        {"id":"night-two","label":"Night Two","scheme":"dark","colors":{"--aki-color-action":"#aabbcc"}},
        {"id":"day-one","label":"Day One","scheme":"light","colors":{}}
      ]
    }"##;
    theme_file(&root.join("themes/neutral-example/theme.json"), variants);
    for (dark, other_color) in [("night-one", "#aabbcc"), ("night-two", "#112233")] {
        let config = config_with_theme(
            &root,
            json!({"catalog":[{"file":"themes/neutral-example/theme.json"}],"deploy":{"dark":dark,"light":"day-one"}}),
        );
        let output = root.join(dark);
        generate(&config, &output, "").unwrap();
        let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
        assert!(css.contains(&format!("data-aki-variant=\"{dark}\"")));
        assert!(!css.contains(&format!(
            "data-aki-variant=\"{}\"",
            if dark == "night-one" {
                "night-two"
            } else {
                "night-one"
            }
        )));
        assert!(css.contains(&format!(
            "--aki-color-action: {}",
            if dark == "night-one" {
                "#112233"
            } else {
                "#aabbcc"
            }
        )));
        assert!(!css.contains(other_color));
        assert!(css.contains("--aki-font-code: \"Mono\", monospace;"));
        if dark == "night-one" {
            assert!(css.contains("--aki-font-body: \"Courier New\", sans-serif;"));
            assert!(css.contains("--aki-radius-panel: 0.75rem;"));
        } else {
            assert!(css.contains("--aki-font-body: \"Arial\", sans-serif;"));
            assert!(css.contains("--aki-radius-panel: 0.5rem;"));
        }
    }
}

#[test]
fn bundled_font_literals_are_case_insensitive_dependencies_in_every_role() {
    let (_temp, root) = observatory_copy();
    let manifest = r##"{
      "version":1,"id":"neutral-example","extends":"default",
      "defaults":{"fonts":{
        "body":{"families":["Arial"],"generic":"sans-serif"},
        "heading":{"families":["gEiSt"],"generic":"serif"},
        "code":{"families":["jEtBrAiNs MoNo"],"generic":"monospace"}
      }},
      "variants":[
        {"id":"neutral-dark","label":"Neutral Dark","scheme":"dark","colors":{}},
        {"id":"neutral-light","label":"Neutral Light","scheme":"light","colors":{}}
      ]
    }"##;
    theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
    let config = config_with_theme(
        &root,
        json!({"catalog":[{"file":"themes/neutral-example/theme.json"}],"deploy":{"dark":"neutral-dark","light":"neutral-light"}}),
    );
    let output = root.join("output");
    generate(&config, &output, "").unwrap();
    let face_css = fs::read_to_string(output.join("css/akibiyori-fonts.css")).unwrap();
    assert!(face_css.contains("font-family: Geist;"));
    assert!(face_css.contains("font-family: \"JetBrains Mono\";"));
    assert!(
        fs::read(output.join("fonts/geist-latin.woff2"))
            .unwrap()
            .as_slice()
            == include_bytes!("../theme/vendor/fonts/geist-latin.woff2")
    );
    assert!(
        fs::read(output.join("fonts/jetbrains-mono-latin.woff2"))
            .unwrap()
            .as_slice()
            == include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2")
    );
    let profile_css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(profile_css.contains(r#"--aki-font-heading: "gEiSt", serif;"#));
    assert!(profile_css.contains(r#"--aki-font-code: "jEtBrAiNs MoNo", monospace;"#));
}

#[test]
fn native_cobalt_keeps_profile_labels_as_data_not_template_source() {
    let (_temp, root) = observatory_copy();
    let manifest = r##"{
      "version":1,"id":"neutral-example","extends":"default",
      "variants":[
        {"id":"neutral-dark","label":"Dark {{ site.title }} {% endif %}","scheme":"dark","colors":{}},
        {"id":"neutral-light","label":"Light profile","scheme":"light","colors":{}}
      ]
    }"##;
    theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
    let config = config_with_theme(
        &root,
        json!({"catalog":[{"file":"themes/neutral-example/theme.json"}],"deploy":{"dark":"neutral-dark","light":"neutral-light"}}),
    );
    let output = root.join("output");
    let mut options = BuildOptions::new(&config, &output);
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real Cobalt and Pagefind build");
    let metadata =
        fs::read_to_string(report.source_dir.join("_data/akibiyori_theme.json")).unwrap();
    assert!(metadata.contains("Dark {{ site.title }} {% endif %}"));
    let html = fs::read_to_string(report.served_dir.join("index.html")).unwrap();
    assert!(
        !html.contains("Dark \""),
        "profile labels remain data and are not injected into template source"
    );
}

#[test]
fn local_grouped_font_faces_emit_only_selected_family_bytes_and_resolve_heading_at_owner() {
    let (_temp, root) = observatory_copy();
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fonts = root.join("themes/neutral-example/fonts-src");
    fs::create_dir_all(&fonts).unwrap();
    let original = source_root.join("theme/vendor/fonts/geist-latin.woff2");
    for name in [
        "body-normal-light.woff2",
        "body-normal-heavy.woff2",
        "body-italic.woff2",
    ] {
        fs::copy(&original, fonts.join(name)).unwrap();
    }
    let manifest = r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"defaults":{"fonts":{"body":{"face":"author-sans","families":["Arial"],"generic":"sans-serif"}}},"font_faces":[{"id":"author-sans","source":"fonts-src/body-normal-light.woff2","weight":[100,400],"style":"normal"},{"id":"author-sans","source":"fonts-src/body-normal-heavy.woff2","weight":[401,900],"style":"normal"},{"id":"author-sans","source":"fonts-src/body-italic.woff2","weight":[100,900],"style":"italic"}]}"##;
    theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
    let config = config_with_theme(
        &root,
        json!({"catalog":[{"builtin":"fumadocs"},{"file":"themes/neutral-example/theme.json"}],"deploy":{"dark":"neutral-example-dark","light":"fumadocs-light"}}),
    );
    let output = root.join("output");
    generate(&config, &output, "").unwrap();
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(
        css.contains("--aki-font-body: aki-15-neutral-example-author-sans, \"Arial\", sans-serif;")
    );
    for file in [
        "author-sans-normal-100-400.woff2",
        "author-sans-normal-401-900.woff2",
        "author-sans-italic-100-900.woff2",
    ] {
        let target = output.join("themes/neutral-example/fonts").join(file);
        assert_eq!(fs::read(&target).unwrap(), fs::read(&original).unwrap());
        assert!(css.contains(&format!("../themes/neutral-example/fonts/{file}")));
    }
    assert_eq!(
        fs::read(output.join("fonts/geist-latin.woff2")).unwrap(),
        fs::read(source_root.join("theme/vendor/fonts/geist-latin.woff2")).unwrap(),
        "the selected Fumadocs Light profile still requires its bundled body face"
    );
    let owner_css =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("theme/css/akibiyori.css"))
            .unwrap();
    assert!(
        owner_css
            .contains("body.framework, #nd-sidebar { --aki-font-heading: var(--aki-font-body)")
    );
}

#[test]
fn legacy_file_theme_consumes_additive_typed_defaults_and_keeps_undeclared_css_fallbacks() {
    let (_temp, root) = observatory_copy();
    let manifest = r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"defaults":{"fonts":{"body":{"families":["Arial"],"generic":"sans-serif"}},"geometry":{"border_width":{"value":1,"unit":"px"},"radii":{"panel":{"value":0.75,"unit":"rem"}}}},"stylesheet":"overrides.css"}"##;
    let trusted_css = b"body.framework { --aki-radius-copy-code: .5rem; }\n";
    theme_file(&root.join("themes/neutral-example/theme.json"), manifest);
    fs::write(
        root.join("themes/neutral-example/overrides.css"),
        trusted_css,
    )
    .unwrap();
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":"dark"}),
    );
    let output = root.join("output");
    generate(&config, &output, "").unwrap();
    let css = fs::read_to_string(output.join("css/akibiyori-theme.css")).unwrap();
    assert!(css.contains("--aki-font-body: \"Arial\", sans-serif;"));
    assert!(css.contains("--aki-border-default: 1px;"));
    assert!(css.contains("--aki-radius-panel: 0.75rem;"));
    assert!(
        !css.contains("--aki-radius-copy-code:"),
        "undeclared typed fallback must not mask trusted CSS"
    );
    assert_eq!(
        fs::read(output.join("themes/neutral-example/styles.css")).unwrap(),
        trusted_css
    );
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["id"], "neutral-example");
    assert_eq!(data["dark_variant"], "neutral-example-dark");
    assert_eq!(data["profiles"][0]["label"], "neutral-example Dark");
}

#[test]
fn theme_selection_and_theme_ids_are_strict_and_fail_before_writes() {
    for selection in [
        Value::Null,
        json!({"source":{"builtin":"other"},"mode":"dark"}),
        json!({"source":{"builtin":"fumadocs","file":"themes/neutral-example/theme.json"},"mode":"dark"}),
        json!({"source":{"builtin":"fumadocs"},"mode":"sepia"}),
        json!({"source":{"builtin":"fumadocs"},"mode":null}),
        json!({"source":{"builtin":"fumadocs"},"mode":"dark","unknown":true}),
    ] {
        let (_temp, root) = observatory_copy();
        let config = config_with_theme(&root, selection);
        let output = root.join("output");
        assert!(generate(&config, &output, "").is_err());
        assert!(!output.exists());
    }

    for id in ["Uppercase", "_hidden", ".hidden", "1numeric", "bad_name"] {
        let (_temp, root) = observatory_copy();
        let manifest = json!({
            "version":1,"id":id,"extends":"fumadocs","modes":{"dark":{"colors":{}}}
        });
        theme_file(
            &root.join("themes/neutral-example/theme.json"),
            &serde_json::to_string(&manifest).unwrap(),
        );
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"}}),
        );
        let output = root.join("output");
        assert!(generate(&config, &output, "").is_err(), "theme id {id}");
        assert!(!output.exists());
    }
}

#[test]
fn system_mode_keeps_author_preference_and_uses_dark_native_fallback() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({"source":{"builtin":"fumadocs"},"mode":"system"}),
    );
    let output = root.join("output");
    generate(&config, &output, "/mounted").expect("System preference generation");
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(data["default_mode"], "system");
    assert_eq!(data["mode"], "dark");
    assert_eq!(data["preference_key"], "akibiyori:mode:v1:/mounted");
    for layout in ["default.liquid", "search.liquid"] {
        let html = fs::read_to_string(output.join("_layouts").join(layout)).unwrap();
        assert!(html.contains(
            "data-aki-default-mode=\"{{ site.data.akibiyori_theme.default_mode | escape }}\""
        ));
        assert!(html.contains(
            "data-aki-mode-key=\"{{ site.data.akibiyori_theme.preference_key | escape }}\""
        ));
        assert!(html.contains("class=\"{{ site.data.akibiyori_theme.mode | escape }}\""));
    }
}

#[test]
fn theme_sources_preserve_typed_errors_and_public_collisions_fail_before_writes() {
    let (_temp, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"}}),
    );
    let manifest_path = root.join("themes/neutral-example/theme.json");
    theme_file(&manifest_path, "{ broken json");
    let malformed_output = root.join("malformed");
    assert!(matches!(
        generate(&config, &malformed_output, ""),
        Err(Error::Json { .. })
    ));
    assert!(!malformed_output.exists());

    theme_file(
        &manifest_path,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"assets":[{"source":"missing/file.txt","target":"files/file.txt"}]}"#,
    );
    let missing_output = root.join("missing-asset");
    assert!(matches!(
        generate(&config, &missing_output, ""),
        Err(Error::Read { .. })
    ));
    assert!(!missing_output.exists());

    for route in [
        "/css/akibiyori-theme.css/",
        "/_data/akibiyori_theme.json/",
        "/js/akibiyori-mode.js/",
        "/themes/neutral-example/files/data.txt/",
    ] {
        let (_temp, root) = observatory_copy();
        let manifest_path = root.join("themes/neutral-example/theme.json");
        theme_file(
            &manifest_path,
            r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"assets":[{"source":"author.txt","target":"files/data.txt"}]}"#,
        );
        fs::write(
            root.join("themes/neutral-example/author.txt"),
            b"author asset",
        )
        .expect("write real asset input");
        let config_path = root.join("config.json");
        let config: Value = serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
        let mut config = config;
        config["theme"] = json!({"source":{"file":"themes/neutral-example/theme.json"}});
        config["pages"].as_array_mut().unwrap().push(json!({
            "source":"content/reference.md","title":"Collision","route":route
        }));
        fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
        let output = root.join("route-collision");
        assert!(
            generate(&config_path, &output, "").is_err(),
            "route {route}"
        );
        assert!(!output.exists(), "route collision {route} writes nothing");
    }
}

#[test]
fn theme_asset_targets_reject_case_distinct_duplicates_before_writes() {
    let (_temp, root) = observatory_copy();
    let theme_dir = root.join("themes/neutral-example");
    fs::create_dir_all(theme_dir.join("author")).unwrap();
    fs::write(theme_dir.join("author/upper.txt"), b"upper bytes").unwrap();
    fs::write(theme_dir.join("author/lower.txt"), b"lower bytes").unwrap();
    theme_file(
        &theme_dir.join("theme.json"),
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"assets":[{"source":"author/upper.txt","target":"files/Alpha.txt"},{"source":"author/lower.txt","target":"files/alpha.txt"}]}"#,
    );
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"}}),
    );

    let absent_output = root.join("case-collision-absent");
    assert!(matches!(
        generate(&config, &absent_output, ""),
        Err(Error::Invalid(_))
    ));
    assert!(!absent_output.exists(), "collision created absent output");

    let empty_output = root.join("case-collision-empty");
    fs::create_dir(&empty_output).unwrap();
    assert!(matches!(
        generate(&config, &empty_output, ""),
        Err(Error::Invalid(_))
    ));
    assert!(
        fs::read_dir(&empty_output).unwrap().next().is_none(),
        "collision changed pre-existing empty output"
    );
}

#[test]
fn theme_css_file_and_author_assets_preserve_real_bytes_in_root_and_prefix_builds() {
    let (_temp, root) = observatory_copy();
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let author_dir = root.join("themes/neutral-example");
    fs::create_dir_all(author_dir.join("author")).unwrap();
    let font = source_root.join("theme/vendor/fonts/geist-latin.woff2");
    let license = source_root.join("theme/vendor/licenses/geist-OFL.txt");
    fs::copy(&font, author_dir.join("author/font.woff2")).unwrap();
    fs::copy(&license, author_dir.join("author/license.txt")).unwrap();
    let stylesheet = b"/* Liquid stays literal: {{ literal }} {% literal %} */\n@font-face { font-family: AuthorExample; src: url(\"fonts/author.woff2\") format(\"woff2\"); }\nbody.framework { --aki-radius-copy-code: .5rem; }\n";
    fs::write(author_dir.join("overrides.css"), stylesheet).unwrap();
    theme_file(
        &author_dir.join("theme.json"),
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"#fafafa"}}},"stylesheet":"overrides.css","assets":[{"source":"author/font.woff2","target":"fonts/author.woff2"},{"source":"author/license.txt","target":"licenses/OFL.txt"}]}"##,
    );
    let config = config_with_theme(
        &root,
        json!({"source":{"file":"themes/neutral-example/theme.json"}}),
    );
    for (label, prefix) in [("root", ""), ("prefix", "/theme-test")] {
        let output = root.join(label);
        let mut options = BuildOptions::new(&config, &output);
        options.base_path = prefix.to_owned();
        options.cobalt = native_program("COBALT", "cobalt");
        options.pagefind = native_program("PAGEFIND", "pagefind");
        let report = build(&options).expect("real Cobalt and Pagefind theme build");
        let served = report.served_dir;
        assert_eq!(
            fs::read(served.join("themes/neutral-example/styles.css")).unwrap(),
            stylesheet
        );
        assert_eq!(
            fs::read(served.join("themes/neutral-example/fonts/author.woff2")).unwrap(),
            fs::read(&font).unwrap()
        );
        assert_eq!(
            fs::read(served.join("themes/neutral-example/licenses/OFL.txt")).unwrap(),
            fs::read(&license).unwrap()
        );
        for page in ["index.html", "search/index.html"] {
            let html = fs::read_to_string(served.join(page)).unwrap();
            assert!(html.contains(&format!("href=\"{prefix}/css/akibiyori-theme.css\"")));
            assert!(html.contains(&format!(
                "href=\"{prefix}/themes/neutral-example/styles.css\""
            )));
            assert!(html.contains("data-aki-theme=\"neutral-example\""));
        }
        let generated_css = fs::read_to_string(served.join("css/akibiyori-theme.css")).unwrap();
        assert!(generated_css.contains("--aki-color-action: #fafafa;"));
    }
}

#[test]
fn theme_assets_reject_symlink_and_hidden_or_colliding_targets_before_write() {
    let (_temp, root) = observatory_copy();
    let theme_dir = root.join("themes/neutral-example");
    fs::create_dir_all(theme_dir.join("author")).unwrap();
    fs::write(theme_dir.join("author/source.txt"), b"author bytes").unwrap();
    for target in [
        "../outside.txt",
        "_hidden/file.txt",
        ".hidden/file.txt",
        "styles.css",
    ] {
        let manifest = json!({
            "version":1,"id":"neutral-example","extends":"fumadocs",
            "modes":{"dark":{"colors":{}}},
            "assets":[{"source":"author/source.txt","target":target}]
        });
        theme_file(
            &theme_dir.join("theme.json"),
            &serde_json::to_string(&manifest).unwrap(),
        );
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"}}),
        );
        let output = root.join("bad-target");
        assert!(generate(&config, &output, "").is_err(), "target {target}");
        assert!(!output.exists());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let real = theme_dir.join("author/real.txt");
        fs::write(&real, b"bytes").unwrap();
        symlink(&real, theme_dir.join("author/link.txt")).unwrap();
        let manifest = json!({
            "version":1,"id":"neutral-example","extends":"fumadocs",
            "modes":{"dark":{"colors":{}}},
            "assets":[{"source":"author/link.txt","target":"files/link.txt"}]
        });
        theme_file(
            &theme_dir.join("theme.json"),
            &serde_json::to_string(&manifest).unwrap(),
        );
        let config = config_with_theme(
            &root,
            json!({"source":{"file":"themes/neutral-example/theme.json"}}),
        );
        let output = root.join("symlink-output");
        assert!(matches!(
            generate(&config, &output, ""),
            Err(Error::Invalid(_))
        ));
        assert!(!output.exists());
    }
}

#[test]
fn bundled_neutral_example_matches_its_file_theme_and_needs_no_caller_theme_files() {
    for (mode, base_path) in [("dark", ""), ("light", "/mounted"), ("system", "/mounted")] {
        let (_builtin_temp, builtin_root) = observatory_copy();
        let builtin_config = config_with_theme(
            &builtin_root,
            json!({"source":{"builtin":"neutral-example"},"mode":mode}),
        );
        fs::remove_dir_all(builtin_root.join("themes/neutral-example"))
            .expect("remove caller theme files before bundled generation");
        let builtin_output = builtin_root.join("output");
        generate(&builtin_config, &builtin_output, base_path)
            .expect("bundled theme does not depend on caller theme files");

        let (_file_temp, file_root) = observatory_copy();
        let file_config = config_with_theme(
            &file_root,
            json!({"source":{"file":"themes/neutral-example/theme.json"},"mode":mode}),
        );
        let file_output = file_root.join("output");
        generate(&file_config, &file_output, base_path).expect("canonical file theme generation");

        assert_eq!(
            file_map(&builtin_output),
            file_map(&file_output),
            "bundled and file theme outputs match for mode={mode} base={base_path}"
        );
        let builtin_metadata: Value = serde_json::from_slice(
            &fs::read(builtin_output.join("_data/akibiyori_theme.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(builtin_metadata["id"], "neutral-example");
        assert_eq!(builtin_metadata["default_mode"], mode);
        assert_eq!(
            builtin_metadata["mode"],
            if mode == "light" { "light" } else { "dark" }
        );
        let css = fs::read_to_string(builtin_output.join("css/akibiyori-theme.css")).unwrap();
        assert!(css.contains("--aki-color-action: #fafafa;"));
        assert!(css.contains("--aki-color-action: #171717;"));
        assert_eq!(
            fs::read(builtin_output.join("themes/neutral-example/styles.css")).unwrap(),
            include_bytes!("../examples/observatory/themes/neutral-example/overrides.css")
        );
    }
}

#[test]
fn unknown_builtin_fails_before_absent_or_empty_output_is_changed() {
    for empty in [false, true] {
        let (_temp, root) = observatory_copy();
        let config = config_with_theme(
            &root,
            json!({"source":{"builtin":"unknown-theme"},"mode":"dark"}),
        );
        let output = root.join(if empty {
            "empty-output"
        } else {
            "absent-output"
        });
        if empty {
            fs::create_dir(&output).expect("make empty output directory");
        }
        let error = generate(&config, &output, "").expect_err("unknown builtin is rejected");
        assert!(
            matches!(error, Error::Invalid(message) if message.contains("unsupported built-in theme"))
        );
        if empty {
            assert!(fs::read_dir(&output).unwrap().next().is_none());
        } else {
            assert!(!output.exists());
        }
    }

    let (_temp, root) = observatory_copy();
    let config = config_with_theme(
        &root,
        json!({"source":{"builtin":"unknown-theme"},"mode":"dark"}),
    );
    let output = root.join("nonempty-output");
    fs::create_dir(&output).expect("make nonempty sentinel output");
    fs::write(output.join("sentinel"), b"keep").expect("write sentinel");
    let before = file_map(&output);
    assert!(generate(&config, &output, "").is_err());
    assert_eq!(
        file_map(&output),
        before,
        "output remains byte-for-byte unchanged"
    );
}

#[test]
fn demo_presentation_and_generation_override_select_full_catalog_or_deploy_pair() {
    let config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config-theme-demo.json");
    let temp = TempDir::new().expect("presentation outputs");
    let demo = temp.path().join("demo");
    let deploy = temp.path().join("deploy");
    generate(&config, &demo, "").expect("configured Demo generation");
    generate_with_options(
        &config,
        &deploy,
        "",
        &GenerationOptions::default().with_theme_mode(ThemePresentation::Deploy),
    )
    .expect("API Deploy override");

    let demo_data: Value =
        serde_json::from_slice(&fs::read(demo.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(demo_data["presentation"], "demo");
    assert_eq!(demo_data["demo_default"], "neutral-day");
    assert_eq!(demo_data["mode"], "light");
    assert_eq!(demo_data["theme_key"], "akibiyori:theme:v1:/");
    let profiles = demo_data["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 6);
    assert_eq!(profiles[0]["id"], "fumadocs-dark");
    assert_eq!(profiles[1]["id"], "fumadocs-light");
    assert_eq!(profiles[2]["id"], "neutral-example-dark");
    assert_eq!(profiles[3]["id"], "neutral-example-light");
    assert_eq!(profiles[4]["id"], "neutral-night");
    assert_eq!(profiles[5]["id"], "neutral-day");
    assert_eq!(
        profiles[5]["label"],
        "Neutral Day with an intentionally longer display name"
    );
    let demo_layout = fs::read_to_string(demo.join("_layouts/default.liquid")).unwrap();
    assert!(demo_layout.contains("data-aki-presentation"));
    assert!(demo_layout.contains("data-aki-default-variant"));
    assert!(demo_layout.contains("data-aki-theme-key"));
    assert!(demo_layout.contains("data-aki-theme-picker"));
    assert!(
        demo_layout.contains("<article id=\"nd-page\" class=\"reference-article")
            && demo_layout.find("<article id=\"nd-page\"").unwrap()
                < demo_layout.find("data-aki-theme-picker").unwrap()
    );

    let deploy_data: Value =
        serde_json::from_slice(&fs::read(deploy.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(deploy_data["presentation"], "deploy");
    assert_eq!(deploy_data["profiles"].as_array().unwrap().len(), 2);
    assert_eq!(deploy_data["profiles"][0]["id"], "neutral-night");
    assert_eq!(deploy_data["profiles"][1]["id"], "fumadocs-light");

    let cli_output = temp.path().join("cli-deploy");
    let cli = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args(["generate", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(&cli_output)
        .args(["--theme-mode", "deploy"])
        .output()
        .expect("run real Akibiyori CLI");
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let cli_data: Value =
        serde_json::from_slice(&fs::read(cli_output.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    assert_eq!(cli_data["presentation"], "deploy");
    assert_eq!(cli_data["profiles"].as_array().unwrap().len(), 2);

    let invalid_output = temp.path().join("cli-invalid");
    let invalid = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args(["generate", "--config"])
        .arg(&config)
        .arg("--output")
        .arg(&invalid_output)
        .args(["--theme-mode", "preview"])
        .output()
        .expect("run CLI with invalid enum value");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(!invalid_output.exists());
}

#[test]
fn demo_default_is_validated_before_deploy_override_and_before_output_writes() {
    let (_temp, root) = observatory_copy();
    for (index, presentation) in [json!(null), json!("preview")].into_iter().enumerate() {
        let config = config_with_theme(
            &root,
            json!({
                "catalog":[{"builtin":"fumadocs"}],
                "deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"},
                "presentation":presentation
            }),
        );
        for (suffix, should_exist) in [("absent", false), ("empty", true)] {
            let output = root.join(format!("bad-presentation-{index}-{suffix}"));
            if should_exist {
                fs::create_dir(&output).unwrap();
            }
            let error = generate(&config, &output, "").unwrap_err();
            assert!(matches!(error, Error::Json { .. }), "{error:?}");
            if should_exist {
                assert!(fs::read_dir(output).unwrap().next().is_none());
            } else {
                assert!(!output.exists());
            }
        }
    }
    let fallback_config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"fumadocs"}],
            "deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"},
            "presentation":"demo"
        }),
    );
    let fallback_output = root.join("demo-default-fallback");
    generate(&fallback_config, &fallback_output, "")
        .expect("omitted demo_default uses mapped Dark");
    let fallback_data: Value = serde_json::from_slice(
        &fs::read(fallback_output.join("_data/akibiyori_theme.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(fallback_data["demo_default"], "fumadocs-dark");
    assert_eq!(fallback_data["variant_id"], "fumadocs-dark");
    for (index, invalid_default) in [json!(null), json!("unknown-profile"), json!(3)]
        .into_iter()
        .enumerate()
    {
        let config = config_with_theme(
            &root,
            json!({
                "catalog":[{"builtin":"fumadocs"}],
                "deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"},
                "presentation":"demo",
                "demo_default":invalid_default
            }),
        );
        for (suffix, should_exist) in [("absent", false), ("empty", true)] {
            let output = root.join(format!("bad-default-{index}-{suffix}"));
            if should_exist {
                fs::create_dir(&output).unwrap();
            }
            let error = generate_with_options(
                &config,
                &output,
                "",
                &GenerationOptions::default().with_theme_mode(ThemePresentation::Deploy),
            )
            .unwrap_err();
            match (&invalid_default, error) {
                (Value::String(_), Error::Invalid(message)) => {
                    assert!(message.contains("demo_default"))
                }
                (_, Error::Json { source, .. }) => assert!(source.to_string().contains("string")),
                (value, error) => panic!("unexpected error for {value}: {error:?}"),
            }
            if should_exist {
                assert!(fs::read_dir(output).unwrap().next().is_none());
            } else {
                assert!(!output.exists());
            }
        }
    }
}

#[test]
fn build_with_options_builds_demo_override_with_native_tools() {
    let config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config-theme-pair.json");
    let temp = TempDir::new().expect("native Demo build root");
    let mut build_options = BuildOptions::new(&config, temp.path().join("build"));
    build_options.cobalt = native_program("COBALT", "cobalt");
    build_options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build_with_options(
        &build_options,
        &GenerationOptions::default().with_theme_mode(ThemePresentation::Demo),
    )
    .expect("real Cobalt and Pagefind Demo override");
    assert_eq!(report.generation.authored_pages, 3);
    let data: Value = serde_json::from_slice(
        &fs::read(report.source_dir.join("_data/akibiyori_theme.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(data["presentation"], "demo");
    assert_eq!(data["profiles"].as_array().unwrap().len(), 4);
    assert!(report.served_dir.join("index.html").is_file());
    assert!(
        report
            .served_dir
            .join("pagefind/pagefind-entry.json")
            .is_file()
    );
}

#[test]
fn options_preserve_default_wrapper_bytes_and_compose_with_showcase() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config.json");
    let temp = TempDir::new().expect("default option outputs");
    let wrapped = temp.path().join("wrapped");
    let explicit = temp.path().join("explicit");
    generate(&source, &wrapped, "").expect("legacy wrapper generation");
    generate_with_options(&source, &explicit, "", &GenerationOptions::default())
        .expect("default options generation");
    assert_eq!(file_map(&wrapped), file_map(&explicit));

    let (_input, root) = observatory_copy();
    let showcase = root.join("showcase.json");
    fs::write(
        &showcase,
        serde_json::to_vec(&json!({
            "version":1,
            "docs":{"site":"observatory","route":"/","label":"Docs"},
            "categories":[{"id":"apis","label":"APIs","site":"observatory","route":"/reference/"}],
            "sites":[{"id":"observatory","label":"Observatory","config":"config.json","mount":"","entry_route":"/","category":"apis"}]
        }))
        .unwrap(),
    )
    .unwrap();
    let demo_config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"fumadocs"},{"builtin":"neutral-example"},{"file":"themes/neutral-profile/theme.json"}],
            "deploy":{"dark":"neutral-night","light":"fumadocs-light"},
            "presentation":"demo","demo_default":"neutral-day"
        }),
    );
    let composed = root.join("composed");
    let options = GenerationOptions::default()
        .with_theme_mode(ThemePresentation::Demo)
        .with_showcase(ShowcaseOptions {
            manifest: showcase,
            site: "observatory".to_owned(),
        });
    generate_with_options(&demo_config, &composed, "", &options)
        .expect("presentation and existing showcase options compose");
    let theme: Value =
        serde_json::from_slice(&fs::read(composed.join("_data/akibiyori_theme.json")).unwrap())
            .unwrap();
    let navigation: Value =
        serde_json::from_slice(&fs::read(composed.join("_data/akibiyori_showcase.json")).unwrap())
            .unwrap();
    assert_eq!(theme["presentation"], "demo");
    assert_eq!(theme["profiles"].as_array().unwrap().len(), 6);
    assert_eq!(navigation["current_label"], "APIs · Observatory");
}

fn single_owner_showcase(root: &Path) -> PathBuf {
    let path = root.join("showcase.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({
            "version":1,
            "docs":{"site":"observatory","route":"/","label":"Docs"},
            "categories":[{"id":"apis","label":"APIs","site":"observatory","route":"/reference/"}],
            "sites":[{"id":"observatory","label":"Observatory","config":"config.json","mount":"","entry_route":"/","category":"apis"}]
        }))
        .expect("serialize one-owner showcase"),
    )
    .expect("write one-owner showcase");
    path
}

#[test]
fn showcase_expected_site_constraint_uses_manifest_owners_and_fails_before_writes() {
    let (_temp, root) = observatory_copy();
    let config = root.join("config.json");
    let manifest = single_owner_showcase(&root);
    let context = ShowcaseOptions {
        manifest: manifest.clone(),
        site: "observatory".to_owned(),
    };
    let valid = root.join("expected-valid");
    generate_with_options(
        &config,
        &valid,
        "",
        &GenerationOptions::default()
            .with_showcase(context.clone())
            .with_showcase_expected_sites(vec!["observatory".to_owned()]),
    )
    .expect("exact owner set is accepted");
    assert!(valid.join("_data/akibiyori_showcase.json").is_file());

    for (label, expected, diagnostic) in [
        ("empty", Vec::new(), "must not be empty"),
        (
            "duplicate",
            vec!["observatory".to_owned(), "observatory".to_owned()],
            "duplicate showcase expected site id",
        ),
        (
            "missing-owner",
            vec!["different-site".to_owned()],
            "does not exactly match manifest sites",
        ),
        (
            "empty-id",
            vec![String::new()],
            "non-empty and contain no controls",
        ),
    ] {
        let options = GenerationOptions::default()
            .with_showcase(context.clone())
            .with_showcase_expected_sites(expected);
        for (suffix, exists) in [("absent", false), ("empty-dir", true)] {
            let output = root.join(format!("failure-{label}-{suffix}"));
            if exists {
                fs::create_dir(&output).expect("create empty output directory");
            }
            let error = generate_with_options(&config, &output, "", &options).unwrap_err();
            assert!(
                matches!(&error, Error::Invalid(message) if message.contains(diagnostic)),
                "{label}: {error:?}"
            );
            if exists {
                assert!(fs::read_dir(&output).unwrap().next().is_none());
            } else {
                assert!(!output.exists());
            }
        }
    }

    for (suffix, exists) in [("absent", false), ("empty", true)] {
        let no_context = root.join(format!("failure-no-context-{suffix}"));
        if exists {
            fs::create_dir(&no_context).expect("create empty no-context output");
        }
        let error = generate_with_options(
            &config,
            &no_context,
            "",
            &GenerationOptions::default().with_showcase_expected_sites(vec!["observatory".into()]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, Error::Invalid(message) if message.contains("requires a showcase context"))
        );
        if exists {
            assert!(fs::read_dir(no_context).unwrap().next().is_none());
        } else {
            assert!(!no_context.exists());
        }
    }
}

#[test]
fn expected_showcase_sites_are_available_on_generate_and_build_cli_commands() {
    let (_temp, root) = observatory_copy();
    let config = root.join("config.json");
    let manifest = single_owner_showcase(&root);
    let generated = root.join("cli-generated");
    let generate = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args([
            "generate",
            "--config",
            config.to_str().unwrap(),
            "--output",
            generated.to_str().unwrap(),
            "--showcase-manifest",
            manifest.to_str().unwrap(),
            "--showcase-site",
            "observatory",
            "--showcase-expected-sites",
            "observatory",
        ])
        .output()
        .expect("run real generate CLI");
    assert!(
        generate.status.success(),
        "{}",
        String::from_utf8_lossy(&generate.stderr)
    );
    assert!(generated.join("_data/akibiyori_showcase.json").is_file());

    let built = root.join("cli-build-rejected");
    let build = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args([
            "build",
            "--config",
            config.to_str().unwrap(),
            "--output",
            built.to_str().unwrap(),
            "--showcase-manifest",
            manifest.to_str().unwrap(),
            "--showcase-site",
            "observatory",
            "--cobalt",
            native_program("COBALT", "cobalt").to_str().unwrap(),
            "--pagefind",
            native_program("PAGEFIND", "pagefind").to_str().unwrap(),
            "--showcase-expected-sites",
            "different-site",
        ])
        .output()
        .expect("run real build CLI");
    assert!(!build.status.success());
    assert!(
        String::from_utf8_lossy(&build.stderr).contains("does not exactly match manifest sites")
    );
    assert!(
        !built.exists(),
        "build rejection occurs before output writes"
    );
}

#[test]
fn central_showcase_theme_fields_scope_and_assets_are_strict_before_writes() {
    let (_temp, root) = observatory_copy();
    let config = root.join("config.json");
    let manifest_path = single_owner_showcase(&root);
    let base: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let valid_theme = json!({"source":{"builtin":"fumadocs"}});
    let cases = [
        (
            "theme-without-scope",
            {
                let mut value = base.clone();
                value["theme"] = valid_theme.clone();
                value
            },
            "theme and theme_scope must be provided together",
            false,
        ),
        (
            "scope-without-theme",
            {
                let mut value = base.clone();
                value["theme_scope"] = json!("scope-a");
                value
            },
            "theme and theme_scope must be provided together",
            false,
        ),
        (
            "null-theme",
            {
                let mut value = base.clone();
                value["theme"] = Value::Null;
                value["theme_scope"] = json!("scope-a");
                value
            },
            "ThemeSelection",
            true,
        ),
        (
            "null-scope",
            {
                let mut value = base.clone();
                value["theme"] = valid_theme.clone();
                value["theme_scope"] = Value::Null;
                value
            },
            "expected a string",
            true,
        ),
        (
            "unsafe-scope",
            {
                let mut value = base.clone();
                value["theme_scope"] = json!("Scope_A");
                value["theme"] = valid_theme.clone();
                value
            },
            "unsafe theme id",
            false,
        ),
        (
            "unsafe-theme-path",
            {
                let mut value = base.clone();
                value["theme_scope"] = json!("scope-a");
                value["theme"] = json!({
                    "catalog":[{"file":"../outside/theme.json"}],
                    "deploy":{"dark":"outside-dark","light":"outside-light"}
                });
                value
            },
            "theme manifest path",
            false,
        ),
    ];
    for (label, value, diagnostic, is_json) in cases {
        let bytes = serde_json::to_vec(&value).expect("serialize central theme case");
        for (suffix, exists) in [("absent", false), ("empty", true)] {
            let manifest = root.join(format!("{label}-{suffix}.json"));
            fs::write(&manifest, &bytes).expect("write invalid central manifest");
            let output = root.join(format!("{label}-{suffix}-output"));
            if exists {
                fs::create_dir(&output).expect("create empty output");
            }
            let error = generate_with_showcase(
                &config,
                &output,
                "",
                &ShowcaseOptions {
                    manifest,
                    site: "observatory".to_owned(),
                },
            )
            .unwrap_err();
            if is_json {
                assert!(
                    matches!(&error, Error::Json { source, .. } if source.to_string().contains(diagnostic)),
                    "{label}: {error:?}"
                );
            } else {
                assert!(
                    matches!(&error, Error::Invalid(message) if message.contains(diagnostic)),
                    "{label}: {error:?}"
                );
            }
            if exists {
                assert!(fs::read_dir(output).unwrap().next().is_none());
            } else {
                assert!(!output.exists());
            }
        }
    }

    let mut paired = base.clone();
    paired["theme_scope"] = json!("scope-a");
    paired["theme"] = valid_theme.clone();
    let valid = serde_json::to_string(&paired).unwrap();
    let duplicate_scope = valid.replacen(
        "\"theme_scope\":\"scope-a\"",
        "\"theme_scope\":\"scope-a\",\"theme_scope\":\"scope-b\"",
        1,
    );
    for (label, raw, expected) in [
        (
            "duplicate-theme",
            valid.replacen("\"theme\":", "\"theme\":{},\"theme\":", 1),
            "duplicate field `theme`",
        ),
        (
            "duplicate-scope",
            duplicate_scope,
            "duplicate field `theme_scope`",
        ),
    ] {
        for (suffix, exists) in [("absent", false), ("empty", true)] {
            let manifest = root.join(format!("{label}-{suffix}.json"));
            fs::write(&manifest, raw.as_bytes()).expect("write duplicate-key JSON bytes");
            let output = root.join(format!("{label}-{suffix}-output"));
            if exists {
                fs::create_dir(&output).expect("create empty output");
            }
            let error = generate_with_showcase(
                &config,
                &output,
                "",
                &ShowcaseOptions {
                    manifest,
                    site: "observatory".to_owned(),
                },
            )
            .unwrap_err();
            assert!(
                matches!(&error, Error::Json { source, .. } if source.to_string().contains(expected)),
                "{label}: {error:?}"
            );
            if exists {
                assert!(fs::read_dir(output).unwrap().next().is_none());
            } else {
                assert!(!output.exists());
            }
        }
    }

    let mut two_site = paired;
    let second_config = root.join("config-second.json");
    let mut second: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    second["theme"] = json!({"mode":"system"});
    fs::write(&second_config, serde_json::to_vec(&second).unwrap()).unwrap();
    let mut second_site = two_site["sites"][0].clone();
    second_site["id"] = json!("second");
    second_site["label"] = json!("Second owner");
    second_site["config"] = json!("config-second.json");
    second_site["mount"] = json!("/second");
    second_site.as_object_mut().unwrap().remove("category");
    two_site["sites"].as_array_mut().unwrap().push(second_site);
    let noncurrent = root.join("noncurrent-conflict.json");
    fs::write(&noncurrent, serde_json::to_vec(&two_site).unwrap()).unwrap();
    for (suffix, exists) in [("absent", false), ("empty", true)] {
        let output = root.join(format!("noncurrent-conflict-{suffix}-output"));
        if exists {
            fs::create_dir(&output).expect("create empty output");
        }
        let error = generate_with_showcase(
            &config,
            &output,
            "",
            &ShowcaseOptions {
                manifest: noncurrent.clone(),
                site: "observatory".to_owned(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(&error, Error::Invalid(message) if message.contains("central theme conflicts with site \"second\""))
        );
        if exists {
            assert!(fs::read_dir(output).unwrap().next().is_none());
        } else {
            assert!(!output.exists());
        }
    }
}

#[test]
fn cli_build_override_and_demo_local_face_packaging_follow_selected_presentation() {
    let (_temp, root) = observatory_copy();
    let (font_input, _license) = prepare_real_local_font(&root, "local-theme");
    let local_manifest = local_face_manifest("local-theme");
    write_manifest_value(&root, "local-theme", &local_manifest);
    let config = config_with_theme(
        &root,
        json!({
            "catalog":[{"builtin":"fumadocs"},{"file":"themes/local-theme/theme.json"}],
            "deploy":{"dark":"fumadocs-dark","light":"fumadocs-light"},
            "presentation":"deploy","demo_default":"local-light"
        }),
    );
    let temp = TempDir::new().expect("local face profile outputs");
    let demo_output = temp.path().join("api-demo");
    generate_with_options(
        &config,
        &demo_output,
        "",
        &GenerationOptions::default().with_theme_mode(ThemePresentation::Demo),
    )
    .expect("Demo override includes non-deploy face family");
    let generated_face =
        demo_output.join("themes/local-theme/fonts/local-sans-normal-1-1000.woff2");
    assert_eq!(
        fs::read(&generated_face).unwrap(),
        fs::read(&font_input).unwrap()
    );
    let fonts_css = fs::read_to_string(demo_output.join("css/akibiyori-theme.css")).unwrap();
    assert!(fonts_css.contains("../themes/local-theme/fonts/local-sans-normal-1-1000.woff2"));
    let deploy_output = temp.path().join("api-deploy");
    generate_with_options(
        &config,
        &deploy_output,
        "",
        &GenerationOptions::default().with_theme_mode(ThemePresentation::Deploy),
    )
    .expect("Deploy override excludes non-deploy face family");
    assert!(!deploy_output.join("themes/local-theme/fonts").exists());

    let cli_build_config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/config-theme-demo.json");
    let cli_output = temp.path().join("cli-build");
    let command = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args(["build", "--config"])
        .arg(cli_build_config)
        .arg("--output")
        .arg(&cli_output)
        .args(["--theme-mode", "deploy", "--cobalt"])
        .arg(native_program("COBALT", "cobalt"))
        .arg("--pagefind")
        .arg(native_program("PAGEFIND", "pagefind"))
        .output()
        .expect("run native CLI build with Deploy override");
    assert!(
        command.status.success(),
        "{}",
        String::from_utf8_lossy(&command.stderr)
    );
    let cli_data: Value = serde_json::from_slice(
        &fs::read(cli_output.join("source/_data/akibiyori_theme.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cli_data["presentation"], "deploy");
    assert_eq!(cli_data["profiles"].as_array().unwrap().len(), 2);
    assert!(cli_output.join("site/index.html").is_file());
}
