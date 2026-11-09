use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use akibiyori::{BuildOptions, Error, build, generate};
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
