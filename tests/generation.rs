use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use akibiyori::{BuildOptions, build, generate};
use serde_json::{Value, json};
use tempfile::TempDir;

fn example_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory")
}

fn generate_example(config: &str, output: &Path) -> akibiyori::GenerationReport {
    generate(&example_root().join(config), output, "")
        .expect("Observatory example generation succeeds")
}

fn file_map(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).expect("read temp tree") {
            let entry = entry.expect("read temp entry");
            let path = entry.path();
            if entry.file_type().expect("file type").is_dir() {
                walk(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .expect("relative path")
                        .to_path_buf(),
                    fs::read(path).expect("read file"),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn retained_control(id: &str) -> Value {
    let inputs: Value = serde_json::from_str(include_str!("fixtures/structural-controls.json"))
        .expect("generic structural controls JSON");
    inputs["cases"]
        .as_array()
        .expect("structural control cases")
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("structural control {id:?} exists"))["spec"]
        .clone()
}

fn generated_source_path(root: &Path, route: &str) -> Option<PathBuf> {
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("_data/akibiyori-manifest.json")).expect("generated manifest"),
    )
    .expect("manifest JSON");
    manifest["content_source_files"][route]
        .as_str()
        .map(|path| root.join(path))
}

fn generated_source(root: &Path, route: &str) -> String {
    fs::read_to_string(generated_source_path(root, route).expect("generated route source mapping"))
        .expect("generated source page")
}

fn concise_schema_view(markdown: &str) -> &str {
    markdown
        .split_once("<section><h2 id=\"source-")
        .map(|(concise, _)| concise)
        .expect("original source JSON section boundary")
}

fn run_cli(config: &Path, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args([
            "generate",
            "--config",
            config.to_str().expect("config path UTF-8"),
            "--output",
            output.to_str().expect("output path UTF-8"),
        ])
        .output()
        .expect("run actual Akibiyori CLI")
}

fn native_program(name: &str, fallback: &str) -> PathBuf {
    std::env::var_os(name).map_or_else(|| PathBuf::from(fallback), PathBuf::from)
}

fn assert_site_routes_exist(site: &Path, manifest: &Value) {
    let routes = manifest["content_source_files"]
        .as_object()
        .expect("content route map");
    assert_eq!(
        routes.len(),
        17,
        "three authored + two operations + nine models + indexes + search"
    );
    for route in routes.keys() {
        let relative = if route == "/" {
            PathBuf::from("index.html")
        } else {
            PathBuf::from(route.trim_start_matches('/')).join("index.html")
        };
        assert!(
            site.join(relative).is_file(),
            "missing rendered route {route}"
        );
    }
}

#[test]
fn real_native_builds_cover_root_and_project_prefix() {
    let temp = TempDir::new().expect("build temp root");
    let config = example_root().join("config.json");
    let mut root_options = BuildOptions::new(&config, temp.path().join("root-build"));
    root_options.cobalt = native_program("COBALT", "cobalt");
    root_options.pagefind = native_program("PAGEFIND", "pagefind");
    let root_report = build(&root_options).expect("real root Cobalt and Pagefind build");
    assert_eq!(root_report.generation.authored_pages, 3);
    assert_eq!(root_report.generation.operation_pages, 2);
    assert_eq!(root_report.generation.model_pages, 9);
    assert_eq!(root_report.site_root, temp.path().join("root-build/site"));
    assert_eq!(root_report.served_dir, root_report.site_root);

    let root_manifest: Value = serde_json::from_slice(
        &fs::read(root_report.source_dir.join("_data/akibiyori-manifest.json"))
            .expect("root manifest"),
    )
    .expect("root manifest JSON");
    assert_site_routes_exist(&root_report.served_dir, &root_manifest);
    let root_home = fs::read_to_string(root_report.served_dir.join("index.html"))
        .expect("built authored home page");
    assert!(root_home.contains("<h1>Observatory Catalog</h1>"));
    assert!(!root_home.contains("# Observatory Catalog"));
    assert!(!root_home.contains("class=\"brand-icon\""));
    assert!(!root_report.served_dir.join("icons/brand.png").exists());
    assert!(root_home.contains("<section class=\"sidebar-group\"><h2>Documentation</h2>"));
    assert!(root_home.contains(
        "<a href=\"/reference/operations/\" data-sidebar-page-link><span>Operations</span></a>"
    ));
    assert!(!root_home.contains("sidebar-icon-initial"));
    assert!(!root_home.contains("class=\"sidebar-link-icon\""));
    assert!(
        root_home
            .contains("<nav class=\"breadcrumbs\" aria-label=\"Breadcrumb\" data-pagefind-ignore>")
    );
    let root_search = fs::read_to_string(root_report.served_dir.join("search/index.html"))
        .expect("built root search page");
    assert!(!root_search.contains("class=\"brand-icon\""));
    assert!(root_search.contains("/pagefind/pagefind-ui.js"));
    assert!(root_search.contains("/pagefind/pagefind-ui.css"));
    assert!(root_search.contains("data-pagefind-ignore"));
    let root_guide = fs::read_to_string(
        root_report
            .served_dir
            .join("docs/reading-an-observation/index.html"),
    )
    .expect("built root guide");
    assert!(root_guide.contains("data-pagefind-body"));
    assert!(
        root_report
            .served_dir
            .join("pagefind/pagefind-ui.js")
            .is_file()
    );
    assert!(
        root_report
            .served_dir
            .join("pagefind/pagefind-ui.css")
            .is_file()
    );
    let pagefind_entry: Value = serde_json::from_slice(
        &fs::read(root_report.served_dir.join("pagefind/pagefind-entry.json"))
            .expect("Pagefind entry metadata"),
    )
    .expect("Pagefind entry JSON");
    assert_eq!(pagefind_entry["version"], "1.5.2");
    assert_eq!(pagefind_entry["languages"]["en"]["page_count"], 16);
    assert!(
        fs::read_to_string(root_report.served_dir.join("pagefind/pagefind-ui.js"))
            .expect("bundled Pagefind UI")
            .contains("PagefindUI")
    );
    assert!(
        fs::read_dir(root_report.served_dir.join("pagefind"))
            .expect("Pagefind output")
            .any(|entry| entry
                .expect("Pagefind entry")
                .file_name()
                .to_string_lossy()
                .starts_with("wasm."))
    );

    let mut prefix_options = BuildOptions::new(&config, temp.path().join("prefix-build"));
    prefix_options.base_path = "/akibiyori-test".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix_report = build(&prefix_options).expect("real prefixed Cobalt and Pagefind build");
    assert_eq!(
        prefix_report.served_dir,
        temp.path().join("prefix-build/site/akibiyori-test")
    );
    let prefix_manifest: Value = serde_json::from_slice(
        &fs::read(
            prefix_report
                .source_dir
                .join("_data/akibiyori-manifest.json"),
        )
        .expect("prefix manifest"),
    )
    .expect("prefix manifest JSON");
    assert_site_routes_exist(&prefix_report.served_dir, &prefix_manifest);
    let prefix_entry: Value = serde_json::from_slice(
        &fs::read(
            prefix_report
                .served_dir
                .join("pagefind/pagefind-entry.json"),
        )
        .expect("prefixed Pagefind metadata"),
    )
    .expect("prefixed Pagefind JSON");
    assert_eq!(prefix_entry["languages"]["en"]["page_count"], 16);
    let prefix_search = fs::read_to_string(prefix_report.served_dir.join("search/index.html"))
        .expect("built prefixed search page");
    assert!(!prefix_search.contains("class=\"brand-icon\""));
    assert!(prefix_search.contains("/akibiyori-test/pagefind/pagefind-ui.js"));
    assert!(prefix_search.contains("data-base-path=\"/akibiyori-test\""));
    assert!(!prefix_search.contains("/akibiyori-test/akibiyori-test/"));
    let prefix_home = fs::read_to_string(prefix_report.served_dir.join("index.html"))
        .expect("built prefixed authored home page");
    assert!(!prefix_home.contains("class=\"brand-icon\""));
    assert!(!prefix_report.served_dir.join("icons/brand.png").exists());
    assert!(prefix_home.contains("<section class=\"sidebar-group\"><h2>Documentation</h2>"));
    assert!(prefix_home.contains("<a href=\"/akibiyori-test/reference/operations/\" data-sidebar-page-link><span>Operations</span></a>"));
    assert!(!prefix_home.contains("sidebar-icon-initial"));
    assert!(!prefix_home.contains("class=\"sidebar-link-icon\""));
    let prefix_guide = fs::read_to_string(
        prefix_report
            .served_dir
            .join("docs/reading-an-observation/index.html"),
    )
    .expect("built prefixed guide");
    assert!(prefix_guide.contains("href=\"/akibiyori-test/reference/\""));
    assert!(prefix_guide.contains("/akibiyori-test/css/akibiyori.css"));
    assert!(!prefix_guide.contains("/akibiyori-test/akibiyori-test/"));
}

#[test]
fn real_native_reference_specimen_renders_shell_components_and_syntax_tokens() {
    let temp = TempDir::new().expect("reference specimen build root");
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/fumadocs-reference.json");
    let mut options = BuildOptions::new(&config, temp.path().join("reference-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real Cobalt and Pagefind reference build");
    assert_eq!(report.generation.authored_pages, 3);
    assert_eq!(report.generation.operation_pages, 2);
    assert_eq!(report.generation.model_pages, 9);

    let home = fs::read_to_string(report.served_dir.join("index.html"))
        .expect("rendered Quick Start page");
    let brand_icon = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fumadocs-reference/assets/leaves-x64.png");
    let brand_icon_bytes = fs::read(&brand_icon).expect("selected brand icon fixture");
    assert_eq!(brand_icon_bytes.len(), 7_631);
    assert!(home.contains(
        "src=\"/icons/brand.png\" width=\"20\" height=\"20\" alt=\"\" aria-hidden=\"true\""
    ));
    assert_eq!(
        fs::read(report.served_dir.join("icons/brand.png")).expect("emitted brand icon"),
        brand_icon_bytes
    );
    let site_data: Value = serde_json::from_slice(
        &fs::read(report.source_dir.join("_data/akibiyori_site.json")).expect("brand site data"),
    )
    .expect("brand site data JSON");
    assert_eq!(site_data["icon_href"], "/icons/brand.png");
    for expected in [
        "<html lang=\"en\" class=\"dark\">",
        "<body class=\"framework relative flex min-h-screen flex-col\">",
        "id=\"fd-spacious-layout\"",
        "id=\"nd-sidebar\"",
        "id=\"nd-page-panel\"",
        "id=\"nd-page\"",
        "id=\"nd-toc\"",
        "class=\"sidebar-section-summary",
        "data-sidebar-section href=\"/\"",
        "data-sidebar-section href=\"/reference/\"",
        "class=\"lucide lucide-rocket\"",
        "class=\"lucide lucide-circle-help\"",
        "class=\"sidebar-collapse",
        "id=\"sidebar-reopen\"",
        "aria-label=\"Search documentation\"",
        "class=\"prose flex-1 text-fd-foreground/90\"",
        "class=\"flex gap-2 my-4 rounded-xl border bg-fd-card p-3 ps-1 text-sm text-fd-card-foreground\"",
        "class=\"not-prose mb-2 w-fit shadow-md rounded-lg border bg-fd-muted p-1.5 text-fd-muted-foreground [&amp;_svg]:size-4\"",
        "<details>",
        "/css/fumadocs-reference.css",
        "/css/akibiyori-fonts.css",
        "/fonts/geist-latin.woff2",
        "/js/akibiyori-reading.js",
    ] {
        assert!(
            home.contains(expected),
            "missing native reference fragment: {expected}"
        );
    }
    assert_eq!(home.matches("<h1 class=\"document-title").count(), 1);
    assert!(home.contains("Getting Started with Fumadocs"));
    assert!(home.contains("Using an AI agent?"));
    assert!(home.contains("Copy the setup instructions into your agent (Claude Code, Cursor, Codex, etc.), it will follow the recommended way to set up a docs site."));
    assert!(home.contains("apps/docs/components/agent-instructions.md"));
    assert!(home.contains("The source of your content, can be a CMS or local data layers like"));
    assert!(home.contains(
        "<a href=\"/reference/operations/\" data-sidebar-page-link><span>Operations</span></a>"
    ));
    assert!(
        home.contains(
            "<a href=\"/reference/models/\" data-sidebar-page-link><span>Models</span></a>"
        )
    );
    assert!(home.contains("<span class=\"sidebar-icon-initial\">C</span></span><span data-section-label>Catalog API</span>"));
    assert!(home.contains(
        "Read our in-depth <a href=\"docs/what-is-fumadocs/\">What is Fumadocs</a> introduction."
    ));
    let callout_start = home
        .find("<p class=\"font-medium my-0!\">Want to learn more?</p>")
        .expect("authored Quick Start callout");
    let callout_end = home[callout_start..]
        .find("</aside>")
        .map(|offset| callout_start + offset + "</aside>".len())
        .expect("Quick Start callout close");
    let callout = &home[callout_start..callout_end];
    assert!(callout.contains("href=\"docs/what-is-fumadocs/\""));
    assert!(!callout.contains("href=\"/docs/what-is-fumadocs/\""));
    assert!(home.contains("A minimum version of Node.js 22 required."));
    assert!(!home.contains("### For Astro"));
    assert!(home.contains("<svg aria-hidden=\"true\""));
    assert!(report.served_dir.join("icons/search.svg").is_file());

    let reference = fs::read_to_string(report.served_dir.join("reference/index.html"))
        .expect("rendered generated reference overview");
    let cards_start = reference
        .find("<div class=\"cards-grid\">")
        .expect("reference destination cards");
    let cards_end = reference[cards_start..]
        .find("</div>")
        .map(|offset| cards_start + offset)
        .expect("reference cards close");
    let cards = &reference[cards_start..cards_end];
    assert!(cards.contains("href=\"operations/\""));
    assert!(cards.contains("href=\"models/\""));
    assert!(!cards.contains("href=\"/reference/operations/\""));
    assert!(!cards.contains("href=\"/reference/models/\""));
    for target in [
        "docs/what-is-fumadocs/index.html",
        "reference/operations/index.html",
        "reference/models/index.html",
    ] {
        assert!(report.served_dir.join(target).is_file(), "missing {target}");
    }
    assert!(reference.contains("class=\"fd-steps\""));
    assert!(reference.contains("class=\"language-json highlighter-syntect\""));
    assert!(reference.contains("c-string c-quoted c-double c-json"));
    assert!(home.contains("class=\"language-bash highlighter-syntect\""));
    assert!(home.contains("c-variable c-function c-shell"));

    let search = fs::read_to_string(report.served_dir.join("search/index.html"))
        .expect("rendered search page");
    assert!(search.contains("class=\"reference-scroll-row search-scroll-row\""));
    assert!(search.contains("id=\"sidebar-reopen\""));
    assert!(search.contains(
        "src=\"/icons/brand.png\" width=\"20\" height=\"20\" alt=\"\" aria-hidden=\"true\""
    ));
    let shell_css =
        fs::read_to_string(report.served_dir.join("css/akibiyori.css")).expect("shell CSS");
    for expected in [
        "--pagefind-ui-text:",
        "--pagefind-ui-background:",
        "--pagefind-ui-border:",
        "--pagefind-ui-primary:",
        "--pagefind-ui-tag:",
        "--pagefind-ui-font:",
        ".search-scroll-row { min-height: 0; overflow: auto; }",
    ] {
        assert!(
            shell_css.contains(expected),
            "missing search style: {expected}"
        );
    }

    let what = fs::read_to_string(report.served_dir.join("docs/what-is-fumadocs/index.html"))
        .expect("rendered source-derived What page");
    assert_eq!(what.matches("<h1 ").count(), 1);
    assert!(what.contains("Introducing Fumadocs, a docs framework that you can break."));
    assert!(what.contains("href=\"https://github.com/fuma-nama/fumadocs/blob/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef/apps/docs/content/docs/(framework)/what-is-fumadocs.mdx\""));
    assert!(what.contains("<textarea hidden data-markdown-source data-pagefind-ignore>"));
    assert!(what.contains("&#35;&#32;&#87;&#104;&#97;&#116;&#32;&#105;&#115;&#32;&#70;&#117;&#109;&#97;&#100;&#111;&#99;&#115;"));
    let header_end = what.find("</header>").expect("source header boundary");
    let prose_start = what
        .find("<div class=\"prose flex-1 text-fd-foreground/90\">")
        .expect("prose body boundary");
    assert!(header_end < prose_start);
    assert!(what[prose_start..].contains("The <strong>Fumadocs</strong> framework refers to a combination of UI + Core + Content Source"));
    assert!(reference.contains("<h4>External source page</h4>"));
    assert!(reference.contains("https://fumadocs.dev/reference/"));
    assert!(report.served_dir.join("pagefind/pagefind-ui.js").is_file());
    let pagefind: Value = serde_json::from_slice(
        &fs::read(report.served_dir.join("pagefind/pagefind-entry.json"))
            .expect("real Pagefind entry"),
    )
    .expect("Pagefind entry JSON");
    assert_eq!(pagefind["version"], "1.5.2");
    assert_eq!(pagefind["languages"]["en"]["page_count"], 16);

    let mut prefix_options = BuildOptions::new(&config, temp.path().join("reference-prefix-build"));
    prefix_options.base_path = "/fumadocs-reference".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix_report = build(&prefix_options).expect("real prefixed Cobalt and Pagefind build");
    let prefix_home = fs::read_to_string(prefix_report.served_dir.join("index.html"))
        .expect("prefixed Quick Start page");
    assert!(prefix_home.contains(
        "src=\"/fumadocs-reference/icons/brand.png\" width=\"20\" height=\"20\" alt=\"\" aria-hidden=\"true\""
    ));
    assert_eq!(
        fs::read(prefix_report.served_dir.join("icons/brand.png"))
            .expect("prefixed emitted brand icon"),
        brand_icon_bytes
    );
    let prefix_site_data: Value = serde_json::from_slice(
        &fs::read(prefix_report.source_dir.join("_data/akibiyori_site.json"))
            .expect("prefixed brand site data"),
    )
    .expect("prefixed brand site data JSON");
    assert_eq!(
        prefix_site_data["icon_href"],
        "/fumadocs-reference/icons/brand.png"
    );
    let prefix_callout_start = prefix_home
        .find("<p class=\"font-medium my-0!\">Want to learn more?</p>")
        .expect("prefixed authored callout");
    let prefix_callout_end = prefix_home[prefix_callout_start..]
        .find("</aside>")
        .map(|offset| prefix_callout_start + offset + "</aside>".len())
        .expect("prefixed callout close");
    let prefix_callout = &prefix_home[prefix_callout_start..prefix_callout_end];
    assert!(prefix_callout.contains("href=\"docs/what-is-fumadocs/\""));
    assert!(!prefix_callout.contains("href=\"/docs/what-is-fumadocs/\""));
    let prefix_reference =
        fs::read_to_string(prefix_report.served_dir.join("reference/index.html"))
            .expect("prefixed reference overview");
    let prefix_cards_start = prefix_reference
        .find("<div class=\"cards-grid\">")
        .expect("prefixed reference destination cards");
    let prefix_cards_end = prefix_reference[prefix_cards_start..]
        .find("</div>")
        .map(|offset| prefix_cards_start + offset)
        .expect("prefixed reference cards close");
    let prefix_cards = &prefix_reference[prefix_cards_start..prefix_cards_end];
    assert!(prefix_cards.contains("href=\"operations/\""));
    assert!(prefix_cards.contains("href=\"models/\""));
    assert!(!prefix_cards.contains("href=\"/reference/operations/\""));
    assert!(!prefix_cards.contains("href=\"/reference/models/\""));
    for target in [
        "docs/what-is-fumadocs/index.html",
        "reference/operations/index.html",
        "reference/models/index.html",
    ] {
        assert!(
            prefix_report.served_dir.join(target).is_file(),
            "missing {target}"
        );
    }
}

#[test]
fn corrected_planets_fixture_changes_only_the_four_accepted_reference_values() {
    let original = include_bytes!("../examples/fumadocs-reference/scalar.yaml");
    let corrected = include_bytes!("../examples/fumadocs-reference/planets-corrected.yaml");
    assert_eq!(original.len(), 46_552);
    assert_eq!(corrected.len(), 46_656);
    let mut expected = original
        .split_inclusive(|byte| *byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    let changes = [
        (134, b"                    $ref: '#/components/schemas/Planet'".as_slice(), b"                    $ref: 'https://example.invalid/planets.openapi#/components/schemas/Planet'".as_slice()),
        (135, b"                $ref: '#/components/schemas/PaginatedResource'".as_slice(), b"                $ref: 'https://galaxy.scalar.com/schemas/PaginatedResource'".as_slice()),
        (144, b"                    $ref: '#/components/schemas/Planet'".as_slice(), b"                    $ref: 'https://example.invalid/planets.openapi#/components/schemas/Planet'".as_slice()),
        (145, b"                $ref: '#/components/schemas/PaginatedResource'".as_slice(), b"                $ref: 'https://galaxy.scalar.com/schemas/PaginatedResource'".as_slice()),
    ];
    for (line_number, old, new) in changes {
        let line = &mut expected[line_number - 1];
        let newline = line.ends_with(b"\n");
        if newline {
            line.pop();
        }
        assert_eq!(
            line.as_slice(),
            old,
            "unexpected original line {line_number}"
        );
        line.clear();
        line.extend_from_slice(new);
        if newline {
            line.push(b'\n');
        }
    }
    assert_eq!(expected.concat().as_slice(), corrected);
}

#[test]
fn original_planets_contract_fails_at_its_unregistered_resource_reference() {
    let temp = TempDir::new().expect("temporary source root");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/fumadocs-reference/scalar.yaml"),
        temp.path().join("scalar.yaml"),
    )
    .expect("copy exact source fixture into isolated test input");
    fs::write(temp.path().join("reference.md"), "# Reference\n")
        .expect("write minimal authored route");
    fs::write(
        temp.path().join("config.json"),
        serde_json::to_vec(&json!({
            "version": 1,
            "title": "Scalar Galaxy",
            "contracts": [{
                "id": "scalar",
                "label": "Scalar Galaxy API",
                "path": "scalar.yaml",
                "document_uri": "https://example.invalid/planets.openapi"
            }],
            "pages": [{"source": "reference.md", "title": "Reference", "route": "/reference/"}]
        }))
        .expect("serialize isolated input config"),
    )
    .expect("write isolated input config");

    let output = temp.path().join("output");
    let error = generate(&temp.path().join("config.json"), &output, "")
        .expect_err("the preserved source contract has an invalid nearest-resource pointer");
    let message = error.to_string();
    assert!(
        message.contains("#/paths/~1planets/get/responses/200/content/application~1json/schema"),
        "{message}"
    );
    assert!(message.contains("PaginatedResource"), "{message}");
    assert!(
        !output.exists(),
        "validation failure must leave output absent"
    );
}

#[test]
fn real_native_planets_builds_render_both_dynamic_media_and_all_source_routes() {
    let temp = TempDir::new().expect("Planets build temp root");
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/planets.json");
    let mut options = BuildOptions::new(&config, temp.path().join("root-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let root = build(&options).expect("actual root Cobalt and Pagefind Planets build");
    assert_eq!(root.generation.authored_pages, 3);
    assert_eq!(root.generation.operation_pages, 11);
    assert_eq!(root.generation.model_pages, 15);
    for contents in file_map(&root.source_dir).values() {
        let text = String::from_utf8_lossy(contents);
        assert!(
            !text.contains("file://"),
            "local absolute file URI leaked into generated source"
        );
        assert!(
            !text.contains("/Users/"),
            "local absolute path leaked into generated source"
        );
    }

    let manifest: Value = serde_json::from_slice(
        &fs::read(root.source_dir.join("_data/akibiyori-manifest.json"))
            .expect("Planets route manifest"),
    )
    .expect("valid route manifest JSON");
    assert_eq!(manifest["operation_routes"].as_object().unwrap().len(), 11);
    assert_eq!(manifest["model_routes"].as_object().unwrap().len(), 15);
    assert!(manifest["operation_routes"]["webhook-newPlanet-post"].is_string());

    let get_route = manifest["content_source_files"]["/reference/getAllData/"]
        .as_str()
        .expect("GET /planets content mapping");
    let get_source = fs::read_to_string(root.source_dir.join(get_route))
        .expect("generated GET /planets content");
    assert!(
        get_source.contains("https://example.invalid/planets.openapi#/components/schemas/Planet")
    );
    assert!(get_source.contains("https://galaxy.scalar.com/schemas/PaginatedResource"));
    assert!(get_source.contains("Dynamic binding: <code>#itemType</code>"));
    assert!(get_source.contains("application~1json/schema/$defs/itemType"));
    assert!(get_source.contains("application~1xml/schema/$defs/itemType"));
    assert!(get_source.contains("<h5>properties: <code>id</code></h5>"));
    assert!(get_source.contains("<h5>properties: <code>habitabilityIndex</code></h5>"));

    let operation_index_route = manifest["content_source_files"]["/reference/operations/"]
        .as_str()
        .expect("operation index mapping");
    let operation_index = fs::read_to_string(root.source_dir.join(operation_index_route))
        .expect("generated operation index");
    assert!(
        operation_index.find("updatePlanet").unwrap()
            < operation_index.find("deletePlanet").unwrap()
    );
    assert!(
        operation_index.find("uploadImage").unwrap()
            < operation_index.find("webhook-newPlanet-post").unwrap()
    );

    let model_index_route = manifest["content_source_files"]["/reference/models/"]
        .as_str()
        .expect("model index mapping");
    let model_index =
        fs::read_to_string(root.source_dir.join(model_index_route)).expect("generated model index");
    assert!(model_index.find("User").unwrap() < model_index.find("Credentials").unwrap());

    let generic_route =
        manifest["content_source_files"]["/reference/models/scalar/PaginatedResource/"]
            .as_str()
            .expect("generic model content mapping");
    let generic_source = fs::read_to_string(root.source_dir.join(generic_route))
        .expect("generated generic schema content");
    assert!(generic_source.contains("Dynamic reference static fallback"));
    assert!(generic_source.contains("#/components/schemas/PaginatedResource/$defs/itemType"));

    let prefix_dir = temp.path().join("prefix-build");
    let mut prefix_options = BuildOptions::new(&config, &prefix_dir);
    prefix_options.base_path = "/planets-reference".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix = build(&prefix_options).expect("actual prefixed Cobalt and Pagefind Planets build");
    let prefixed_get = prefix.served_dir.join("reference/getAllData/index.html");
    assert!(prefixed_get.is_file());
    let prefixed_html = fs::read_to_string(prefixed_get).expect("prefixed GET /planets HTML");
    assert!(prefixed_html.contains("/planets-reference/reference/models/scalar/Planet/"));
    let prefixed_search = fs::read_to_string(prefix.served_dir.join("search/index.html"))
        .expect("prefixed search page");
    assert!(prefixed_search.contains("/planets-reference/pagefind/pagefind-ui.js"));
    assert!(prefixed_search.contains("/planets-reference/js/akibiyori-search.js"));
}

#[test]
fn real_cobalt_build_maps_dotted_logical_routes_to_directory_indexes() {
    let temp = TempDir::new().expect("dotted route root");
    let spec = json!({
        "openapi":"3.0.0",
        "info":{"title":"Dotted route control","version":"1"},
        "paths":{
            "/thing":{
                "get":{
                    "operationId":"get.v1",
                    "responses":{"200":{"description":"ok"}}
                }
            }
        },
        "components":{"schemas":{"Thing.v1":{"type":"object"}}}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Dotted guide\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_value["pages"]
        .as_array_mut()
        .expect("pages")
        .push(json!({
            "source":"page.md",
            "title":"Dotted guide",
            "route":"/guide.v1/"
        }));
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write dotted route config");

    let mut options = BuildOptions::new(&config, temp.path().join("build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("actual Cobalt/Pagefind dotted route build");
    let manifest: Value = serde_json::from_slice(
        &fs::read(report.source_dir.join("_data/akibiyori-manifest.json"))
            .expect("dotted route manifest"),
    )
    .expect("dotted route manifest JSON");
    assert!(
        manifest["authored_routes"]
            .as_array()
            .expect("authored routes")
            .contains(&json!("/guide.v1/"))
    );
    assert_eq!(manifest["operation_routes"]["get.v1"], "/reference/get.v1/");
    assert_eq!(
        manifest["model_routes"]["test:Thing.v1"],
        "/reference/models/test/Thing.v1/"
    );
    for relative in [
        "guide.v1/index.html",
        "reference/get.v1/index.html",
        "reference/models/test/Thing.v1/index.html",
    ] {
        assert!(
            report.served_dir.join(relative).is_file(),
            "missing native target {relative}"
        );
    }
    assert!(report.served_dir.join("guide.v1").is_dir());
    assert!(report.served_dir.join("guide.v1/index.html").is_file());
}

#[test]
fn native_build_preflights_tools_and_output_without_writing() {
    let temp = TempDir::new().expect("build temp root");
    let config = example_root().join("config.json");
    let output = temp.path().join("missing-pagefind");
    let mut options = BuildOptions::new(&config, &output);
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = temp.path().join("not-an-installed-pagefind");
    assert!(matches!(
        build(&options),
        Err(akibiyori::Error::NativeSpawn {
            stage: "Pagefind version",
            ..
        })
    ));
    assert!(
        !output.exists(),
        "tool preflight failure must leave output absent"
    );

    let mismatch_output = temp.path().join("version-mismatch");
    let mut mismatch = BuildOptions::new(&config, &mismatch_output);
    mismatch.cobalt = native_program("COBALT", "cobalt");
    mismatch.pagefind = mismatch.cobalt.clone();
    assert!(matches!(
        build(&mismatch),
        Err(akibiyori::Error::NativeVersion { .. })
    ));
    assert!(
        !mismatch_output.exists(),
        "version mismatch must leave output absent"
    );

    let nonempty = temp.path().join("nonempty");
    fs::create_dir(&nonempty).expect("create nonempty output");
    fs::write(nonempty.join("keep.txt"), "preserve").expect("write output sentinel");
    let mut nonempty_options = BuildOptions::new(&config, &nonempty);
    nonempty_options.cobalt = native_program("COBALT", "cobalt");
    nonempty_options.pagefind = native_program("PAGEFIND", "pagefind");
    assert!(matches!(
        build(&nonempty_options),
        Err(akibiyori::Error::Invalid(_))
    ));
    assert_eq!(
        fs::read_to_string(nonempty.join("keep.txt")).expect("preserved output sentinel"),
        "preserve"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let real_dir = temp.path().join("real-output");
        fs::create_dir(&real_dir).expect("create real output target");
        let link = temp.path().join("linked-output");
        symlink(&real_dir, &link).expect("create output symlink");
        let mut linked_options = BuildOptions::new(&config, &link);
        linked_options.cobalt = native_program("COBALT", "cobalt");
        linked_options.pagefind = native_program("PAGEFIND", "pagefind");
        assert!(matches!(
            build(&linked_options),
            Err(akibiyori::Error::Invalid(_))
        ));
        assert!(
            fs::read_dir(&real_dir)
                .expect("real output remains empty")
                .next()
                .is_none()
        );
    }
}

#[test]
fn search_route_is_reserved_before_output_writes() {
    let temp = TempDir::new().expect("search collision root");
    let spec = json!({
        "openapi":"3.0.0",
        "info":{"title":"Search collision","version":"1"},
        "paths":{},
        "components":{"schemas":{}}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Search collision\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_value["pages"]
        .as_array_mut()
        .expect("pages")
        .push(json!({
            "source":"page.md",
            "title":"Search collision",
            "route":"/search/"
        }));
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write search collision config");
    let output = temp.path().join("source");
    let error = generate(&config, &output, "").expect_err("reserved search route");
    assert!(error.to_string().contains("search route /search/ collides"));
    assert!(
        !output.exists(),
        "reserved-route failure must precede writes"
    );
}

#[test]
fn authored_routes_cannot_shadow_native_search_assets() {
    let temp = TempDir::new().expect("native asset collision root");
    let spec = json!({
        "openapi":"3.0.0",
        "info":{"title":"Asset collision","version":"1"},
        "paths":{},
        "components":{"schemas":{}}
    });
    for route in [
        "/pagefind/",
        "/pagefind/pagefind.js/",
        "/js/akibiyori-search.js/",
    ] {
        let input = temp.path().join(route.trim_matches('/').replace('/', "-"));
        let config = synthetic_config(&input, &spec, "# Asset collision\n", "/reference/");
        let mut config_value: Value =
            serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
        config_value["pages"]
            .as_array_mut()
            .expect("pages")
            .push(json!({
                "source":"page.md",
                "title":"Asset collision",
                "route":route
            }));
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_value).expect("serialize config"),
        )
        .expect("write asset collision config");
        let output = temp.path().join(format!("out-{}", route.len()));
        let error = generate(&config, &output, "").expect_err("reserved native asset path");
        assert!(
            error
                .to_string()
                .contains("public file/directory collision")
        );
        assert!(
            !output.exists(),
            "asset collision must precede output writes"
        );
    }
}

fn synthetic_config(dir: &Path, spec: &Value, page: &str, route: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("create synthetic input directory");
    fs::write(
        dir.join("spec.json"),
        serde_json::to_vec_pretty(spec).expect("serialize spec"),
    )
    .expect("write spec");
    fs::write(dir.join("page.md"), page).expect("write page");
    let config = json!({
        "version": 1,
        "title": "Synthetic contract reader",
        "contracts": [{"id":"test","label":"Test contract","path":"spec.json"}],
        "pages": [{"source":"page.md","title":"Reference","route":route}],
        "navigation": []
    });
    fs::write(
        dir.join("config.json"),
        serde_json::to_vec_pretty(&config).expect("serialize config"),
    )
    .expect("write config");
    dir.join("config.json")
}

#[test]
fn observatory_example_has_expected_routes_and_deterministic_source() {
    let first = TempDir::new().expect("first output");
    let second = TempDir::new().expect("second output");
    let out_a = first.path().join("source-a");
    let out_b = second.path().join("source-b");
    let report_a = generate_example("config.json", &out_a);
    let report_b = generate_example("config.json", &out_b);

    assert_eq!(report_a.authored_pages, 3);
    assert_eq!(report_a.operation_pages, 2);
    assert_eq!(report_a.model_pages, 9);
    assert_eq!(report_b.operation_pages, report_a.operation_pages);
    assert_eq!(report_b.model_pages, report_a.model_pages);
    assert_eq!(file_map(&out_a), file_map(&out_b));

    let manifest: Value = serde_json::from_slice(
        &fs::read(out_a.join("_data/akibiyori-manifest.json")).expect("manifest"),
    )
    .expect("manifest JSON");
    assert_eq!(
        manifest["operation_routes"]
            .as_object()
            .expect("operations")
            .len(),
        2
    );
    assert_eq!(
        manifest["model_routes"].as_object().expect("models").len(),
        9
    );
    for route in [
        "/reference/read-observation/",
        "/reference/list-instruments/",
        "/reference/models/catalog/ObservationEnvelope/",
        "/reference/models/catalog/Observation/",
        "/reference/models/catalog/ObservationBase/",
        "/reference/models/catalog/PhotometricObservation/",
        "/reference/models/catalog/SpectroscopicObservation/",
        "/reference/models/catalog/Instrument/",
        "/reference/models/catalog/Calibration/",
        "/reference/models/catalog/ScalarValue/",
        "/reference/models/catalog/ObservationGroup/",
        "/docs/reading-an-observation/",
    ] {
        assert!(
            manifest["content_source_files"].get(route).is_some(),
            "missing route {route}"
        );
    }
    let guide = generated_source(&out_a, "/docs/reading-an-observation/");
    for fact in [
        "read-observation",
        "ObservationEnvelope",
        "PhotometricObservation",
        "ObservationBase",
        "Instrument",
        "Calibration",
        "calibration_epoch",
    ] {
        assert!(guide.contains(fact), "guide should explain {fact}");
    }
    let scalar = generated_source(&out_a, "/reference/models/catalog/ScalarValue/");
    let concise = concise_schema_view(&scalar);
    for fact in [
        "Label",
        "Sequence",
        "Signal",
        "Enabled",
        "minLength",
        "minimum",
    ] {
        assert!(concise.contains(fact), "concise alternatives retain {fact}");
    }
}

#[test]
fn synthetic_contract_keeps_composition_refs_and_effective_parameters() {
    let temp = TempDir::new().expect("synthetic root");
    let mut spec = json!({
        "openapi":"3.0.3",
        "security":[{"bearer":[]}],
        "paths":{
            "/thing/{id}":{
                "parameters":[{"$ref":"#/components/parameters/a~1b~0c"}],
                "get":{
                    "operationId":"read-thing",
                    "security":[],
                    "parameters":[{"name":"id","in":"path","required":true,"description":"operation override","schema":{"type":"string"}}],
                    "responses":{
                        "200":{"description":"json","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Combined"}}}},
                        "202":{"description":"text","content":{"text/plain":{"schema":{"type":"string"}}}}
                    }
                },
                "put":{
                    "operationId":"update-thing",
                    "requestBody":{"$ref":"#/components/requestBodies/ThingRequest"},
                    "responses":{"204":{"$ref":"#/components/responses/ThingResponse"}}
                }
            }
        },
        "components":{
            "parameters":{"a/b~c":{"name":"id","in":"path","required":true,"description":"path base","schema":{"type":"string"}}},
            "requestBodies":{"ThingRequest":{"description":"reusable request","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Combined"}}}}},
            "responses":{"ThingResponse":{"description":"reusable response","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Combined"}}}}},
            "securitySchemes":{"bearer":{"type":"http","scheme":"bearer"}},
            "schemas":{
                "Combined":{
                    "type":"object",
                    "properties":{
                        "id":{"type":"string"},
                        "child":{"type":"object","properties":{"id":{"type":"integer"},"a/b~":{"type":"string"}},"required":["id"]},
                        "self":{"$ref":"#/components/schemas/Combined"},
                        "map":{"type":"object","additionalProperties":{"type":"string"}},
                        "list":{"type":"array","items":{"type":"object","properties":{"id":{"type":"integer"}}}}
                    },
                    "required":["id"],
                    "allOf":[{"type":"object","properties":{"own":{"type":"boolean"}}}],
                    "oneOf":[{"type":"string"},{"$ref":"#/components/schemas/Combined"}],
                    "anyOf":[{"type":"integer"},{"type":"null"}],
                    "x-display-note":"retained extension"
                }
            }
        }
    });
    spec["paths"]["/thing/{id}"]["parameters"][0]["$ref"] =
        json!("#/components/parameters/a~1b~0c");
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n\nLiteral: `{{ 4 | plus: 1 }}`\n",
        "/reference/",
    );
    let out = temp.path().join("generated");
    let report = generate(&config, &out, "").expect("synthetic generate");
    assert_eq!(report.operation_pages, 2);
    assert_eq!(report.model_pages, 1);
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].pointer.contains("x-display-note"));

    let root_facts_spec = json!({
        "openapi":"3.0.0","paths":{},"components":{"schemas":{"RootFacts":{
            "type":"string","title":"Root title","description":"Root description",
            "enum":["first","second"],"minLength":2
        }}}
    });
    let root_facts_config = synthetic_config(
        &temp.path().join("root-facts"),
        &root_facts_spec,
        "# Reference\n",
        "/reference/",
    );
    let root_facts_output = temp.path().join("root-facts-source");
    generate(&root_facts_config, &root_facts_output, "").expect("root schema facts");
    let root_facts = generated_source(&root_facts_output, "/reference/models/test/RootFacts/");
    let concise_root = concise_schema_view(&root_facts);
    for fact in [
        "Root title",
        "Root description",
        "enum",
        "first",
        "second",
        "minLength",
    ] {
        assert!(
            concise_root.contains(fact),
            "missing root schema fact {fact}"
        );
    }

    let model = generated_source(&out, "/reference/models/test/Combined/");
    assert!(model.contains("All schemas must apply"));
    assert!(model.contains("Exactly one schema must apply"));
    assert!(model.contains("One or more schemas must apply"));
    assert!(model.contains("retained extension"));
    assert!(model.contains("field-232f636f6d706f6e656e74732f736368656d61732f436f6d62696e65642f70726f706572746965732f6368696c642f70726f706572746965732f6964"));
    assert!(model.contains("Additional properties"));
    assert!(model.contains("Array items"));
    assert!(model.contains("field-232f636f6d706f6e656e74732f736368656d61732f436f6d62696e65642f70726f706572746965732f6368696c642f70726f706572746965732f617e31627e30"));
    let operation = generated_source(&out, "/reference/read-thing/");
    assert!(operation.contains("operation override"));
    assert!(!operation.contains("<p>path base</p>"));
    assert!(operation.contains("Status 200") && operation.contains("Status 202"));
    assert!(operation.contains("application/json") && operation.contains("text/plain"));
    assert!(operation.contains("All schemas must apply") || operation.contains("Combined"));
    assert!(operation.contains("Security alternatives are ORed"));
    assert!(operation.contains("<pre><code>[]</code></pre>"));
    let update = generated_source(&out, "/reference/update-thing/");
    assert!(update.contains("reusable request"));
    assert!(update.contains("Required: <strong>false</strong> (default)"));
    assert!(update.contains("reusable response"));
    assert!(update.contains("Combined"));
    let diagnostics: Value = serde_json::from_slice(
        &fs::read(out.join("_data/akibiyori-diagnostics.json")).expect("diagnostics"),
    )
    .expect("diagnostic JSON");
    assert_eq!(diagnostics.as_array().expect("diagnostic list").len(), 1);
}

#[test]
fn preflight_errors_leave_output_absent_or_unchanged() {
    let cases = [
        (
            "dialect",
            json!({"openapi":"3.2.0","paths":{},"components":{"schemas":{"Thing":{}}}}),
            "/reference/",
            "unsupported OpenAPI version",
        ),
        (
            "external",
            json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{"$ref":"https://example.test/schema.json"}}}}),
            "/reference/",
            "external reference",
        ),
        (
            "missing",
            json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{"$ref":"#/components/schemas/Missing"}}}}),
            "/reference/",
            "has no target",
        ),
        (
            "scalar",
            json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":"string"}}}),
            "/reference/",
            "must be an object",
        ),
        (
            "non-string-ref",
            json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{"$ref":false}}}}),
            "/reference/",
            "$ref at #/components/schemas/Thing must be a string",
        ),
    ];
    for (name, spec, route, expected) in cases {
        let temp = TempDir::new().expect("case root");
        let input = temp.path().join("input");
        let config = synthetic_config(&input, &spec, "# Page\n", route);
        let output = temp.path().join("output");
        let error = generate(&config, &output, "").expect_err("invalid input rejected");
        assert!(error.to_string().contains(expected), "{name}: {error}");
        assert!(!output.exists(), "{name}: preflight created output");
    }

    let temp = TempDir::new().expect("frontmatter root");
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    let config = synthetic_config(temp.path(), &spec, "---\ntitle: broken\n", "/reference/");
    let output = temp.path().join("unchanged");
    fs::create_dir(&output).expect("empty output");
    let error = generate(&config, &output, "").expect_err("unterminated frontmatter rejected");
    assert!(
        error
            .to_string()
            .contains("unterminated leading frontmatter")
    );
    assert_eq!(fs::read_dir(&output).expect("output remains").count(), 0);
}

#[test]
fn authored_document_rejects_unsafe_source_urls_before_writing() {
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    for source_url in [
        "javascript:alert(1)",
        "https:///missing-authority",
        "https://example.test/with whitespace",
        "https://example.test\\\\path",
    ] {
        let temp = TempDir::new().expect("document source URL root");
        let config = synthetic_config(temp.path(), &spec, "A body.\n", "/reference/");
        let mut config_data: Value =
            serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
        config_data["pages"][0]["document"] = json!({
            "description":"A description",
            "source_url":source_url
        });
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_data).expect("serialize document config"),
        )
        .expect("write document config");
        let output = temp.path().join("generated");
        let error = generate(&config, &output, "").expect_err("unsafe document source URL");
        assert!(error.to_string().contains("document source_url"), "{error}");
        assert!(
            !output.exists(),
            "invalid source URL must fail before writes"
        );
    }
}

#[test]
fn brand_icon_paths_signature_and_output_collisions_fail_before_writes() {
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    for invalid_path in ["../outside.png", "/tmp/brand.png"] {
        let temp = TempDir::new().expect("invalid brand path root");
        let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
        let mut config_value: Value =
            serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config");
        config_value["brand_icon"] = json!(invalid_path);
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_value).expect("serialize config"),
        )
        .expect("write config");
        let output = temp.path().join("output");
        let error = generate(&config, &output, "").expect_err("invalid brand path rejected");
        assert!(error.to_string().contains("brand icon path"), "{error}");
        assert!(!output.exists(), "invalid brand path wrote output");
    }

    let temp = TempDir::new().expect("invalid PNG root");
    let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config");
    config_value["brand_icon"] = json!("not-a-png.bin");
    fs::write(
        config.parent().expect("config root").join("not-a-png.bin"),
        b"not a PNG file",
    )
    .expect("write invalid signature input");
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write config");
    let output = temp.path().join("invalid-png-output");
    let error = generate(&config, &output, "").expect_err("invalid PNG signature rejected");
    assert!(
        error
            .to_string()
            .contains("brand icon input must have the PNG signature")
    );
    assert!(!output.exists(), "invalid PNG wrote output");

    let missing_temp = TempDir::new().expect("missing icon root");
    let missing_config =
        synthetic_config(missing_temp.path(), &spec, "# Reference\n", "/reference/");
    let mut missing_value: Value =
        serde_json::from_slice(&fs::read(&missing_config).expect("read missing icon config"))
            .expect("config JSON");
    missing_value["brand_icon"] = json!("missing.png");
    fs::write(
        &missing_config,
        serde_json::to_vec_pretty(&missing_value).expect("serialize missing icon config"),
    )
    .expect("write missing icon config");
    let missing_output = missing_temp.path().join("missing-icon-output");
    assert!(generate(&missing_config, &missing_output, "").is_err());
    assert!(!missing_output.exists(), "missing PNG wrote output");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().expect("symlink brand icon root");
        let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
        let outside = temp.path().join("outside.png");
        fs::write(&outside, b"not read through a symlink").expect("write outside file");
        let root = config.parent().expect("config root");
        symlink(&outside, root.join("linked.png")).expect("create icon input symlink");
        let mut config_value: Value =
            serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config");
        config_value["brand_icon"] = json!("linked.png");
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_value).expect("serialize config"),
        )
        .expect("write config");
        let output = temp.path().join("symlink-output");
        let error = generate(&config, &output, "").expect_err("symlink input rejected");
        assert!(
            error
                .to_string()
                .contains("must not traverse symbolic links")
        );
        assert!(!output.exists(), "symlink brand path wrote output");
    }

    let collision_temp = TempDir::new().expect("brand target collision root");
    let config = synthetic_config(collision_temp.path(), &spec, "# Reference\n", "/reference/");
    let icon = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fumadocs-reference/assets/leaves-x64.png");
    let config_root = config.parent().expect("config root");
    fs::copy(&icon, config_root.join("brand.png"))
        .expect("copy actual selected PNG for collision control");
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config");
    config_value["brand_icon"] = json!("brand.png");
    config_value["pages"]
        .as_array_mut()
        .expect("authored pages")
        .push(json!({
            "source":"page.md",
            "title":"Reserved icon route",
            "route":"/icons/brand.png/"
        }));
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write config");
    let output = collision_temp.path().join("collision-output");
    let error = generate(&config, &output, "").expect_err("brand public target reserved");
    assert!(
        error
            .to_string()
            .contains("public file/directory collision"),
        "{error}"
    );
    assert!(
        error.to_string().contains("/icons/brand.png asset"),
        "{error}"
    );
    assert!(!output.exists(), "brand target collision wrote output");
}

#[test]
fn sidebar_groups_require_content_and_resolve_routes_before_output_writes() {
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    let cases = [
        (
            "empty title",
            json!([{"title":"  ","links":[{"label":"Reference","href":"/reference/"}]}]),
            "sidebar group title must not be empty",
        ),
        (
            "empty links",
            json!([{"title":"Documentation","links":[]}]),
            "must contain at least one link",
        ),
        (
            "unsafe external scheme",
            json!([{"title":"Documentation","links":[{"label":"External","href":"javascript:alert(1)"}]}]),
            "http(s) URL",
        ),
        (
            "unknown internal route",
            json!([{"title":"Documentation","links":[{"label":"Missing","href":"/missing/"}]}]),
            "does not match an authored or generated route",
        ),
    ];
    for (name, groups, expected) in cases {
        let temp = TempDir::new().expect("sidebar group root");
        let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
        let mut config_data: Value =
            serde_json::from_slice(&fs::read(&config).expect("config")).expect("config JSON");
        config_data["sidebar_groups"] = groups;
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_data).expect("serialize config"),
        )
        .expect("write config");
        let output = temp.path().join("source");
        let error = generate(&config, &output, "").expect_err("invalid sidebar group");
        assert!(error.to_string().contains(expected), "{name}: {error}");
        assert!(!output.exists(), "{name}: preflight wrote output");
    }
}

#[test]
fn sidebar_sections_require_unique_internal_resolved_routes_and_known_icons() {
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    let cases = [
        (
            "empty label",
            json!([{"label":"  ","href":"/reference/"}]),
            "label and href must not be empty",
        ),
        (
            "external route",
            json!([{"label":"External","href":"https://example.test/"}]),
            "must be an internal route",
        ),
        (
            "duplicate route",
            json!([
                {"label":"First","href":"/reference/"},
                {"label":"Second","href":"/reference/"}
            ]),
            "duplicate sidebar section route",
        ),
        (
            "unresolved route",
            json!([{"label":"Missing","href":"/missing/"}]),
            "does not match an authored or generated route",
        ),
        (
            "unknown icon",
            json!([{"label":"Reference","href":"/reference/","icon":"sparkles"}]),
            "unknown variant",
        ),
    ];
    for (name, sections, expected) in cases {
        let temp = TempDir::new().expect("sidebar section root");
        let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
        let mut config_data: Value =
            serde_json::from_slice(&fs::read(&config).expect("config")).expect("config JSON");
        config_data["sidebar_sections"] = sections;
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_data).expect("serialize config"),
        )
        .expect("write config");
        let output = temp.path().join("source");
        let error = generate(&config, &output, "").expect_err("invalid sidebar section");
        assert!(error.to_string().contains(expected), "{name}: {error}");
        assert!(!output.exists(), "{name}: invalid section wrote output");
    }
}

#[test]
fn unsafe_routes_collisions_and_nonempty_output_fail_without_writes() {
    let spec = json!({"openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}});
    for route in [
        "/../reference/",
        "/bad%2fpath/",
        "/reference/operations/",
        "/double//slash/",
    ] {
        let temp = TempDir::new().expect("route root");
        let config = synthetic_config(temp.path(), &spec, "# Page\n", route);
        let output = temp.path().join("output");
        let error = generate(&config, &output, "").expect_err("unsafe or colliding route rejected");
        assert!(error.to_string().contains("route") || error.to_string().contains("collides"));
        assert!(!output.exists());
    }

    let temp = TempDir::new().expect("output root");
    let config = synthetic_config(temp.path(), &spec, "# Page\n", "/reference/");
    let output = temp.path().join("output");
    fs::create_dir(&output).expect("existing output");
    fs::write(output.join("keep.txt"), "preserve").expect("sentinel");
    assert!(generate(&config, &output, "").is_err());
    assert_eq!(
        fs::read_to_string(output.join("keep.txt")).expect("sentinel survives"),
        "preserve"
    );
    assert!(generate(&config, &temp.path().join("other"), "/docs/%2e%2e").is_err());
}

#[test]
fn duplicate_ids_and_authored_routes_fail_during_preflight() {
    let temp = TempDir::new().expect("duplicate root");
    let spec = json!({
        "openapi":"3.0.0",
        "paths":{"/x":{"get":{"operationId":"read-x","responses":{"200":{"description":"ok"}}}}},
        "components":{"schemas":{"Thing":{}}}
    });
    let config = synthetic_config(temp.path(), &spec, "# Page\n", "/reference/");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&config).expect("config")).expect("JSON");

    let first_contract = value["contracts"][0].clone();
    value["contracts"]
        .as_array_mut()
        .expect("contracts")
        .push(first_contract);
    fs::write(
        &config,
        serde_json::to_vec_pretty(&value).expect("serialize duplicate ids"),
    )
    .expect("write duplicate ids");
    let duplicate_id_output = temp.path().join("duplicate-id-output");
    let error = generate(&config, &duplicate_id_output, "").expect_err("duplicate contract id");
    assert!(error.to_string().contains("duplicate contract id"));
    assert!(!duplicate_id_output.exists());

    let config = synthetic_config(temp.path(), &spec, "# Page\n", "/reference/");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&config).expect("config")).expect("JSON");
    value["contracts"][0]["id"] = json!("other");
    let first_contract = value["contracts"][0].clone();
    value["contracts"]
        .as_array_mut()
        .expect("contracts")
        .push(first_contract);
    let second = value["contracts"]
        .as_array_mut()
        .expect("contracts")
        .get_mut(1)
        .expect("second");
    second["id"] = json!("second");
    fs::write(
        &config,
        serde_json::to_vec_pretty(&value).expect("serialize duplicate op ids"),
    )
    .expect("write duplicate operation IDs");
    let duplicate_operation_output = temp.path().join("duplicate-operation-output");
    let error = generate(&config, &duplicate_operation_output, "")
        .expect_err("duplicate operationId across contracts");
    assert!(error.to_string().contains("duplicate operationId"));
    assert!(!duplicate_operation_output.exists());

    let config = synthetic_config(temp.path(), &spec, "# Page\n", "/reference/");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&config).expect("config")).expect("JSON");
    let first_page = value["pages"][0].clone();
    value["pages"]
        .as_array_mut()
        .expect("pages")
        .push(first_page);
    fs::write(
        &config,
        serde_json::to_vec_pretty(&value).expect("serialize duplicate routes"),
    )
    .expect("write duplicate routes");
    let duplicate_route_output = temp.path().join("duplicate-route-output");
    let error =
        generate(&config, &duplicate_route_output, "").expect_err("duplicate authored route");
    assert!(error.to_string().contains("duplicate authored route"));
    assert!(!duplicate_route_output.exists());
}

#[test]
fn base_path_and_cli_exit_contract_are_applied() {
    let temp = TempDir::new().expect("prefix root");
    let spec = json!({
        "openapi":"3.0.0",
        "paths":{
            "/thing":{
                "get":{
                    "operationId":"read-thing",
                    "responses":{
                        "200":{
                            "description":"ok",
                            "content":{
                                "application/json":{
                                    "schema":{"$ref":"#/components/schemas/Thing"}
                                }
                            }
                        }
                    }
                }
            }
        },
        "components":{
            "schemas":{
                "Thing":{
                    "type":"object",
                    "properties":{"self":{"$ref":"#/components/schemas/Thing"}}
                }
            }
        }
    });
    let config = synthetic_config(temp.path(), &spec, "# Reference\n", "/reference/");
    let mut config_data: Value =
        serde_json::from_slice(&fs::read(&config).expect("config")).expect("config JSON");
    config_data["navigation"] = json!([
        {"label":"Reference","href":"/reference/"},
        {"label":"External","href":"https://example.test/help/"}
    ]);
    config_data["sidebar_groups"] = json!([
        {"title":"API","links":[
            {"label":"Reference","href":"/reference/"},
            {"label":"External","href":"https://example.test/help/"}
        ]}
    ]);
    config_data["sidebar_sections"] = json!([
        {"label":"Reference","href":"/reference/","icon":"book-open"}
    ]);
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_data).expect("serialize config"),
    )
    .expect("write config");
    let output = temp.path().join("prefixed-source");
    generate(&config, &output, "/preview").expect("generate with base path");
    let operation = generated_source(&output, "/reference/read-thing/");
    let model = generated_source(&output, "/reference/models/test/Thing/");
    let navigation_data: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_navigation.json")).expect("navigation data"),
    )
    .expect("navigation data JSON");
    let sidebar_data: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_sidebar.json")).expect("sidebar data"),
    )
    .expect("sidebar data JSON");
    let sections_data: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_sections.json")).expect("sections data"),
    )
    .expect("sections data JSON");
    let layout = fs::read_to_string(output.join("_layouts/default.liquid")).expect("layout");
    assert!(operation.contains("href=\"/preview/reference/models/test/Thing/\""));
    assert!(model.contains("href=\"/preview/reference/models/test/Thing/\""));
    assert_eq!(navigation_data[0]["href"], "/preview/reference/");
    assert_eq!(navigation_data[1]["href"], "https://example.test/help/");
    assert_eq!(sidebar_data[0]["title"], "API");
    assert_eq!(sidebar_data[0]["links"][0]["href"], "/preview/reference/");
    assert_eq!(
        sidebar_data[0]["links"][1]["href"],
        "https://example.test/help/"
    );
    assert_eq!(sections_data[0]["label"], "Reference");
    assert_eq!(sections_data[0]["href"], "/preview/reference/");
    assert_eq!(sections_data[0]["icon"], "book-open");
    assert_eq!(sections_data[0]["initial"], "R");
    assert!(layout.contains("/preview/css/akibiyori.css"));

    let binary = env!("CARGO_BIN_EXE_akibiyori");
    assert!(
        Command::new(binary)
            .arg("--help")
            .status()
            .expect("help")
            .success()
    );
    assert!(
        Command::new(binary)
            .arg("--version")
            .status()
            .expect("version")
            .success()
    );
    assert_eq!(
        Command::new(binary)
            .arg("--unknown")
            .status()
            .expect("usage error")
            .code(),
        Some(2)
    );
    assert_eq!(
        Command::new(binary)
            .args([
                "generate",
                "--config",
                "missing.json",
                "--output",
                "unused-output"
            ])
            .status()
            .expect("processing error")
            .code(),
        Some(1)
    );
}

#[test]
fn retained_case_collision_fails_before_output_through_cli() {
    let temp = TempDir::new().expect("collision root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("case-collision"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("portable public route collision"));
    assert!(!output.exists());
}

#[test]
fn retained_diagnostic_pointers_and_resolved_response_source_are_exact() {
    let temp = TempDir::new().expect("diagnostic root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("diagnostic-pointer"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let diagnostics: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori-diagnostics.json")).expect("diagnostics"),
    )
    .expect("diagnostics JSON");
    let pointers: std::collections::BTreeSet<&str> = diagnostics
        .as_array()
        .expect("diagnostic array")
        .iter()
        .filter_map(|diagnostic| diagnostic["pointer"].as_str())
        .collect();
    for pointer in [
        "#/paths/~1x/get/parameters/0/style",
        "#/components/responses/Reply/headers",
    ] {
        assert!(
            pointers.contains(pointer),
            "missing exact source pointer {pointer}: {pointers:?}"
        );
        assert!(
            retained_control("diagnostic-pointer")
                .pointer(pointer.strip_prefix('#').expect("fragment pointer"))
                .is_some(),
            "pointer resolves against the actual source contract: {pointer}"
        );
    }
    let operation = generated_source(&output, "/reference/read-x/");
    assert!(
        operation.contains("Resolved response source at <code>#/components/responses/Reply</code>")
    );
    assert!(operation.contains("X-Value"));
    assert!(operation.contains("&quot;headers&quot;"));
}

#[test]
fn retained_hidden_identifiers_render_at_exact_public_routes_in_cobalt() {
    let temp = TempDir::new().expect("hidden identifier root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("hidden-identifiers"),
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("config")).expect("config JSON");
    config_value["pages"]
        .as_array_mut()
        .expect("authored pages")
        .push(json!({
            "source":"page.md","title":"Hidden guide","route":"/_guide/"
        }));
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize hidden authored route"),
    )
    .expect("write hidden authored route");
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout)
            .contains("2 authored pages, 1 operation pages, 1 model pages")
    );
    assert!(
        generated_source_path(&output, "/reference/_read-x/")
            .expect("operation source")
            .is_file()
    );
    assert!(
        generated_source_path(&output, "/reference/models/test/_Thing/")
            .expect("model source")
            .is_file()
    );
    let guide_source_path =
        generated_source_path(&output, "/_guide/").expect("authored source map");
    assert!(guide_source_path.starts_with(output.join("pages")));
    let guide_source = fs::read_to_string(&guide_source_path).expect("authored Cobalt source");
    assert!(guide_source.contains("title: \"Hidden guide\""));
    assert!(guide_source.contains("# Reference"));
    let cobalt = Command::new("cobalt")
        .current_dir(&output)
        .args(["build", "--config", "_cobalt.yml", "--destination", "_site"])
        .output()
        .expect("actual Cobalt CLI");
    assert!(
        cobalt.status.success(),
        "{}",
        String::from_utf8_lossy(&cobalt.stderr)
    );
    assert!(output.join("_site/reference/_read-x/index.html").is_file());
    assert!(
        output
            .join("_site/reference/models/test/_Thing/index.html")
            .is_file()
    );
    let guide_html = fs::read_to_string(output.join("_site/_guide/index.html"))
        .expect("authored hidden route HTML");
    assert!(guide_html.contains("<title>Hidden guide · Synthetic contract reader</title>"));
    assert!(guide_html.contains("<h1>Reference</h1>"));
}

#[test]
fn retained_unknown_keywords_are_reported_at_each_schema_node() {
    let temp = TempDir::new().expect("unknown schema root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("leaf-unknown"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let model = generated_source(&output, "/reference/models/test/Thing/");
    assert!(model.contains("x-leaf-note") && model.contains("leaf note"));
    assert!(model.contains("&quot;not&quot;") && model.contains("&quot;integer&quot;"));
    let diagnostics: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori-diagnostics.json")).expect("diagnostics"),
    )
    .expect("diagnostics JSON");
    let pointers: std::collections::BTreeSet<&str> = diagnostics
        .as_array()
        .expect("diagnostic array")
        .iter()
        .filter_map(|item| item["pointer"].as_str())
        .collect();
    assert!(pointers.contains("#/components/schemas/Thing/properties/field/x-leaf-note"));
    assert!(pointers.contains("#/components/schemas/Thing/not"));

    let branch_spec = json!({
        "openapi":"3.0.0","paths":{},"components":{"schemas":{"Branch":{
            "type":"object","allOf":[{"type":"string","title":"Branch alternative","x-branch-note":"branch note"}]
        }}}
    });
    let branch_config = synthetic_config(
        &temp.path().join("branch"),
        &branch_spec,
        "# Reference\n",
        "/reference/",
    );
    let branch_output = temp.path().join("branch-source");
    generate(&branch_config, &branch_output, "").expect("branch schema");
    let branch = generated_source(&branch_output, "/reference/models/test/Branch/");
    assert!(concise_schema_view(&branch).contains("Branch alternative"));
    let branch_diagnostics: Value = serde_json::from_slice(
        &fs::read(branch_output.join("_data/akibiyori-diagnostics.json"))
            .expect("branch diagnostics"),
    )
    .expect("branch diagnostics JSON");
    assert!(
        branch_diagnostics
            .as_array()
            .expect("array")
            .iter()
            .any(|item| item["pointer"] == "#/components/schemas/Branch/allOf/0/x-branch-note")
    );
}

#[test]
fn literal_example_ref_values_are_not_structural_references() {
    let temp = TempDir::new().expect("literal example root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("literal-example-ref"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let model = generated_source(&output, "/reference/models/test/Thing/");
    assert!(model.contains("https://example.test/literal-value"));
    assert!(!model.contains("Schema reference:"));

    let mut missing_property_ref = retained_control("literal-example-ref");
    missing_property_ref["components"]["schemas"]["Thing"]["properties"]["example"] =
        json!({"$ref":"#/components/schemas/Missing"});
    let config = synthetic_config(
        &temp.path().join("missing-property-ref"),
        &missing_property_ref,
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("missing-property-source");
    let result = run_cli(&config, &output);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("#/components/schemas/Thing/properties/example")
    );
    assert!(!output.exists());
}

#[test]
fn reusable_parameter_aliases_resolve_with_actual_provenance_and_cycles_fail() {
    let temp = TempDir::new().expect("parameter chain root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("parameter-chain"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let operation = generated_source(&output, "/reference/read-x/");
    assert!(operation.contains("unique-query"));
    assert!(operation.contains("unique resolved fact"));
    assert!(!operation.contains("(unnamed)"));
    assert!(operation.contains("#/components/parameters/Alias → #/components/parameters/Actual"));
    let input: Value =
        serde_json::from_str(include_str!("fixtures/structural-controls.json")).expect("fixture");
    let source_spec = input["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["id"] == "parameter-chain")
        .expect("case")["spec"]
        .clone();
    let value: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori-manifest.json")).expect("manifest"),
    )
    .expect("manifest JSON");
    let _ = value;
    let parameter = source_spec
        .pointer("/components/parameters/Actual")
        .expect("actual source target");
    assert_eq!(parameter["name"], "unique-query");

    let wrong_shape_spec = json!({
        "openapi":"3.0.0","paths":{"/x":{"get":{"operationId":"read-x","parameters":[{"$ref":"#/components/responses/NotParameter"}],"responses":{"200":{"description":"ok"}}}}},
        "components":{"responses":{"NotParameter":{"description":"not a parameter"}},"schemas":{"Thing":{}}}
    });
    let wrong_shape_config = synthetic_config(
        &temp.path().join("wrong-shape"),
        &wrong_shape_spec,
        "# Reference\n",
        "/reference/",
    );
    let wrong_shape_output = temp.path().join("wrong-shape-source");
    let error = generate(&wrong_shape_config, &wrong_shape_output, "")
        .expect_err("wrong reusable object shape fails");
    assert!(error.to_string().contains("targets an unsupported object"));
    assert!(!wrong_shape_output.exists());

    let cycle_spec = json!({
        "openapi":"3.0.0","paths":{"/x":{"get":{"operationId":"read-x","parameters":[{"$ref":"#/components/parameters/A"}],"responses":{"200":{"description":"ok"}}}}},
        "components":{"parameters":{"A":{"$ref":"#/components/parameters/B"},"B":{"$ref":"#/components/parameters/A"},"Unused":{"name":"x","in":"query","schema":{"type":"string"}}},"schemas":{"Thing":{}}}
    });
    let cycle_config = synthetic_config(
        &temp.path().join("cycle"),
        &cycle_spec,
        "# Reference\n",
        "/reference/",
    );
    let cycle_output = temp.path().join("cycle-source");
    let error =
        generate(&cycle_config, &cycle_output, "").expect_err("cyclic reusable parameters fail");
    assert!(
        error
            .to_string()
            .contains("cyclic parameters reference chain")
    );
    assert!(!cycle_output.exists());
}

#[test]
fn retained_path_item_reference_is_explicitly_unsupported_before_output() {
    let temp = TempDir::new().expect("path-item ref root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &retained_control("path-item-ref"),
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("source");
    let result = run_cli(&config, &output);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("path-item $ref is unsupported at #/paths/~1x")
    );
    assert!(!output.exists());
}

#[test]
fn cobalt_renders_escaped_contract_text_and_preserves_literal_liquid() {
    let version = Command::new("cobalt")
        .arg("--version")
        .output()
        .expect("installed Cobalt required by make check");
    assert!(version.status.success());
    let temp = TempDir::new().expect("Cobalt root");
    let input = temp.path().join("input");
    let spec = json!({
        "openapi":"3.0.0",
        "paths":{"/unsafe":{"get":{"operationId":"read-unsafe","summary":"\"<script>alert('x')</script>","responses":{"200":{"description":"ok"}}}}},
        "components":{"schemas":{"Thing":{"type":"object","description":"<img src=x onerror=alert(1)>","properties":{"value":{"type":"string","description":"<script>field()</script> & \"quoted\""}}}}}
    });
    let config = synthetic_config(
        &input,
        &spec,
        "# Page\n\nLiteral Liquid: `{{ 4 | plus: 1 }}` and `{% assign x = 2 %}`\n",
        "/reference/",
    );
    let mut config_data: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_data["title"] = json!("\"<script>site()</script>");
    config_data["pages"][0]["title"] = json!("\"<script>page()</script>");
    config_data["navigation"] = json!([
        {"label":"{{ 4 | plus: 1 }}","href":"/reference/"},
        {"label":"{% assign x = 2 %}","href":"/reference/"},
        {"label":"<script>nav()</script> & \"quoted\"","href":"/reference/"}
    ]);
    config_data["sidebar_groups"] = json!([
        {"title":"Guides {{ 7 | plus: 1 }}","links":[
            {"label":"{{ 7 | plus: 1 }}","href":"/reference/"},
            {"label":"{% if %}","href":"/reference/"},
            {"label":"<script>nav()</script> & \"quoted\"","href":"/reference/"}
        ]}
    ]);
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_data).expect("serialize hostile config"),
    )
    .expect("write hostile config");
    let output = temp.path().join("source");
    generate(&config, &output, "").expect("generate");
    let result = Command::new("cobalt")
        .current_dir(&output)
        .args(["build", "--config", "_cobalt.yml", "--destination", "_site"])
        .output()
        .expect("run native Cobalt");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(output.join("_site/reference/models/test/Thing/index.html"))
        .expect("rendered model HTML");
    assert!(html.contains("&lt;script&gt;field()&lt;/script&gt;"));
    let reference = fs::read_to_string(output.join("_site/reference/index.html"))
        .expect("authored reference HTML");
    assert!(
        reference.contains("{{ 4 | plus: 1 }}"),
        "literal output: {reference}"
    );
    assert!(
        reference.contains("{% assign x = 2 %}"),
        "literal output: {reference}"
    );
    assert!(reference.contains("Guides {{ 7 | plus: 1 }}"));
    assert!(reference.contains("{% if %}"));
    assert!(reference.contains("&lt;script&gt;nav()&lt;/script&gt; &amp; &quot;quoted&quot;"));
    assert!(!reference.contains("Guides 8"));
    assert!(!reference.contains("Literal Liquid: <code>5</code>"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));
    let operation = fs::read_to_string(output.join("_site/reference/read-unsafe/index.html"))
        .expect("operation HTML");
    assert!(operation.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));
    assert!(!operation.contains("<script>alert('x')</script>"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));
    let title = reference
        .lines()
        .find(|line| line.contains("<title>"))
        .expect("title line");
    assert!(
        title.contains("&lt;script&gt;page()&lt;/script&gt;"),
        "{title}"
    );
    assert!(
        title.contains("&lt;script&gt;site()&lt;/script&gt;"),
        "{title}"
    );
}
