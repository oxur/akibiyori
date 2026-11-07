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
    assert_eq!(
        fs::read(omitted.join("css/akibiyori-theme.css")).unwrap(),
        b""
    );
    let html = fs::read_to_string(omitted.join("_layouts/default.liquid"))
        .expect("generated default layout");
    assert!(html.contains("data-aki-theme=\"{{ site.data.akibiyori_theme.id | escape }}\""));
    assert!(html.contains("data-aki-mode=\"{{ site.data.akibiyori_theme.mode | escape }}\""));
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
        "html[data-aki-theme=\"neutral-example\"][data-aki-mode=\"dark\"] body.framework"
    ));
    assert!(
        css.contains(
            "html[data-aki-theme=\"neutral-example\"][data-aki-mode=\"dark\"] #nd-sidebar"
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
    const ROLES: [&str; 37] = [
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
    generate(&config, &output, "").expect("all 37 allowed roles compile");
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
fn theme_rejections_happen_before_absent_or_empty_output_changes() {
    for manifest in [
        r#"{"version":2,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"../bad","extends":"fumadocs","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"other","modes":{"dark":{"colors":{}}}}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-action":"red"}}}}"#,
        r##"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{"--aki-color-unknown":"#abc"}}}}"##,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"stylesheet":null}"#,
        r#"{"version":1,"id":"neutral-example","extends":"fumadocs","modes":{"dark":{"colors":{}}},"light":{"colors":{}}}"#,
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
fn theme_selection_and_theme_ids_are_strict_and_fail_before_writes() {
    for selection in [
        Value::Null,
        json!({"source":{"builtin":"other"},"mode":"dark"}),
        json!({"source":{"builtin":"fumadocs","file":"themes/neutral-example/theme.json"},"mode":"dark"}),
        json!({"source":{"builtin":"fumadocs"},"mode":"light"}),
        json!({"source":{"builtin":"fumadocs"},"mode":"system"}),
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
