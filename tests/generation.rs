use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use akibiyori::{BuildOptions, Error, ShowcaseOptions, build, generate, generate_with_showcase};
use serde_json::{Value, json};
use tempfile::TempDir;

fn example_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory")
}

fn generate_example(config: &str, output: &Path) -> akibiyori::GenerationReport {
    generate(&example_root().join(config), output, "")
        .expect("Observatory example generation succeeds")
}

fn assert_token_stylesheet_link(html: &str, href: &str) {
    let link = format!("<link rel=\"stylesheet\" href=\"{href}\">");
    assert_eq!(
        html.matches(&link).count(),
        1,
        "one token stylesheet link: {href}"
    );
    let fonts = html
        .find("/css/akibiyori-fonts.css")
        .expect("font stylesheet link");
    let vendor = html
        .find("/css/fumadocs-reference.css")
        .expect("vendor stylesheet link");
    let tokens = html.find(href).expect("token stylesheet link");
    let adapter = html
        .find("/css/akibiyori.css")
        .expect("adapter stylesheet link");
    assert!(
        fonts < vendor && vendor < tokens && tokens < adapter,
        "stylesheet order is fonts, vendor, tokens, adapter"
    );
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

fn assert_model_field_links_resolve(site_root: &Path, base_path: &str) -> usize {
    let files = file_map(site_root);
    let model_prefix = format!("{base_path}/reference/models/");
    let mut links = 0;

    for (source_path, bytes) in &files {
        if source_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("html")
        {
            continue;
        }
        let source = String::from_utf8_lossy(bytes);
        let mut remaining = source.as_ref();
        while let Some(start) = remaining.find("href=\"") {
            let value = &remaining[start + "href=\"".len()..];
            let Some(end) = value.find('"') else {
                break;
            };
            let href = &value[..end];
            remaining = &value[end + 1..];
            let Some((path, fragment)) = href.split_once('#') else {
                continue;
            };
            if !path.starts_with(&model_prefix) || !fragment.starts_with("field-") {
                continue;
            }
            let route = path
                .strip_prefix(base_path)
                .expect("model target retains the requested base path")
                .trim_start_matches('/');
            let target_path = Path::new(route).join("index.html");
            let target = files.get(&target_path).unwrap_or_else(|| {
                panic!("model field link from {source_path:?} has no destination {target_path:?}")
            });
            let target = String::from_utf8_lossy(target);
            assert!(
                target.contains(&format!("id=\"{fragment}\"")),
                "model field link from {source_path:?} points to absent {fragment:?} at {target_path:?}"
            );
            links += 1;
        }
    }

    assert!(links > 0, "native output contains model field links");
    links
}

fn heading_id(html: &str, heading: &str) -> String {
    let end = html
        .find(&format!(">{heading}</h3>"))
        .unwrap_or_else(|| panic!("rendered h3 heading {heading:?} exists"));
    let id_start = html[..end]
        .rfind("id=\"")
        .map(|start| start + "id=\"".len())
        .unwrap_or_else(|| panic!("rendered h3 heading {heading:?} has an id"));
    let id_end = html[id_start..end]
        .find('"')
        .map(|offset| id_start + offset)
        .unwrap_or_else(|| panic!("rendered h3 heading {heading:?} has a closed id attribute"));
    html[id_start..id_end].to_owned()
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

fn response_panel<'a>(html: &'a str, status: &str) -> &'a str {
    let marker = format!("data-choice-label=\"{status}\"");
    let marker_at = html
        .find(&marker)
        .unwrap_or_else(|| panic!("missing response status {status:?}"));
    let start = html[..marker_at]
        .rfind("<section class=\"operation-response\"")
        .expect("response status is inside an operation response panel");
    let end = html[marker_at..]
        .find("<section class=\"operation-response\"")
        .map(|offset| marker_at + offset)
        .unwrap_or(html.len());
    &html[start..end]
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
    let token_css = include_bytes!("../theme/css/akibiyori-tokens.css");
    assert_eq!(
        fs::read(root_report.source_dir.join("css/akibiyori-tokens.css"))
            .expect("generated root token CSS"),
        token_css
    );
    assert_eq!(
        fs::read(root_report.served_dir.join("css/akibiyori-tokens.css"))
            .expect("served root token CSS"),
        token_css
    );
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
    assert_token_stylesheet_link(&root_home, "/css/akibiyori-tokens.css");
    assert_token_stylesheet_link(&root_search, "/css/akibiyori-tokens.css");
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
    assert_eq!(
        fs::read(prefix_report.source_dir.join("css/akibiyori-tokens.css"))
            .expect("generated prefix token CSS"),
        token_css
    );
    assert_eq!(
        fs::read(prefix_report.served_dir.join("css/akibiyori-tokens.css"))
            .expect("served prefix token CSS"),
        token_css
    );
    assert_token_stylesheet_link(&prefix_search, "/akibiyori-test/css/akibiyori-tokens.css");
    assert!(!prefix_search.contains("class=\"brand-icon\""));
    assert!(prefix_search.contains("/akibiyori-test/pagefind/pagefind-ui.js"));
    assert!(prefix_search.contains("data-base-path=\"/akibiyori-test\""));
    assert!(!prefix_search.contains("/akibiyori-test/akibiyori-test/"));
    let prefix_home = fs::read_to_string(prefix_report.served_dir.join("index.html"))
        .expect("built prefixed authored home page");
    assert_token_stylesheet_link(&prefix_home, "/akibiyori-test/css/akibiyori-tokens.css");
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
    let html_start = home.find("<html ").expect("rendered html opening tag");
    let html_end = home[html_start..]
        .find('>')
        .map(|end| html_start + end)
        .expect("closed html opening tag");
    let html_opening = &home[html_start..html_end];
    for expected in [
        "lang=\"en\"",
        "class=\"dark\"",
        "data-aki-theme=\"fumadocs\"",
        "data-aki-mode=\"dark\"",
    ] {
        assert!(
            html_opening.contains(expected),
            "opening html tag lacks {expected}"
        );
    }
    for expected in [
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
    assert!(reference.contains("Catalog API reference"));
    assert!(reference.contains("Observatory Catalog contract"));
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
fn reference_sidebar_is_opt_in_and_groups_multi_contract_tags_without_changing_source_order() {
    let temp = TempDir::new().expect("multi-contract sidebar input root");
    let first_spec = json!({
        "openapi": "3.1.0",
        "info": {"title": "First contract", "version": "1"},
        "tags": [{"name": "records", "x-displayName": "Records", "summary": "Metadata fallback", "parent": "root"}],
        "components": {"schemas": {}},
        "paths": {
            "/first": {
                "post": {"operationId": "postFirst", "summary": "Add first", "tags": ["records", "records"], "responses": {"200": {"description": "ok"}}},
                "get": {"operationId": "getFirst", "summary": "Read first", "tags": ["records"], "responses": {"200": {"description": "ok"}}}
            },
            "/second": {
                "put": {"operationId": "putSecond", "responses": {"200": {"description": "ok"}}}
            }
        },
        "webhooks": {
            "newRecord": {"post": {"tags": ["records"], "responses": {"200": {"description": "ok"}}}},
            "namedRecord": {"post": {"summary": "Published record", "tags": ["records"], "responses": {"200": {"description": "ok"}}}}
        }
    });
    let second_spec = json!({
        "openapi": "3.1.0",
        "info": {"title": "Second contract", "version": "1"},
        "tags": [{"name": "records", "summary": "Record events"}],
        "components": {"schemas": {}},
        "paths": {
            "/third": {"get": {"operationId": "getThird", "tags": ["records"], "responses": {"200": {"description": "ok"}}}}
        }
    });
    fs::write(
        temp.path().join("first.json"),
        serde_json::to_vec(&first_spec).unwrap(),
    )
    .expect("write first spec");
    fs::write(
        temp.path().join("second.json"),
        serde_json::to_vec(&second_spec).unwrap(),
    )
    .expect("write second spec");
    fs::write(temp.path().join("reference.md"), "# Reference\n").expect("write reference page");
    let config = json!({
        "version": 1,
        "title": "Tag navigation control",
        "contracts": [
            {"id": "first", "label": "First API", "path": "first.json"},
            {"id": "second", "label": "Second API", "path": "second.json"}
        ],
        "pages": [{"source": "reference.md", "title": "Reference", "route": "/reference/"}],
        "reference": {"layout": "columns"},
        "reference_sidebar": {"overview_label": "Scalar Example", "untagged_label": "Untagged"}
    });
    let config_path = temp.path().join("config.json");
    fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap()).expect("write config");

    let output = temp.path().join("generated");
    let generated =
        generate(&config_path, &output, "/prefix").expect("generate multi-contract sidebar");
    assert!(generated.diagnostics.iter().any(|diagnostic| {
        diagnostic.pointer == "#/tags/0/parent" && diagnostic.message.contains("does not interpret")
    }));
    let sidebar: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_reference_sidebar.json")).unwrap(),
    )
    .expect("generated sidebar JSON");
    let groups = sidebar["groups"].as_array().unwrap();
    assert_eq!(
        groups
            .iter()
            .map(|group| group["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "First API — Records",
            "First API — Untagged",
            "Second API — Record events"
        ]
    );
    let first_links = groups[0]["links"].as_array().unwrap();
    assert_eq!(
        first_links.len(),
        4,
        "duplicate operation tags collapse within a group"
    );
    assert_eq!(first_links[0]["label"], "Read first");
    assert_eq!(first_links[1]["label"], "Add first");
    assert_eq!(first_links[0]["href"], "/prefix/reference/getFirst/");
    assert_eq!(first_links[2]["label"], "New Record");
    assert_eq!(
        first_links[2]["href"],
        "/prefix/reference/webhook-newRecord-post/"
    );
    assert_eq!(first_links[2]["title"], "newRecord");
    assert!(
        first_links[2]["accessible_label"]
            .as_str()
            .unwrap()
            .contains("newRecord")
    );
    assert_eq!(
        first_links[3]["label"], "Published record",
        "supplied webhook summary wins"
    );
    assert_eq!(groups[1]["links"][0]["label"], "putSecond");
    assert_eq!(groups[2]["links"][0]["label"], "getThird");
    assert_eq!(sidebar["overview"]["href"], "/prefix/reference/");

    let index = generated_source(&output, "/reference/operations/");
    assert!(index.find("postFirst").unwrap() < index.find("getFirst").unwrap());
    let reference = generated_source(&output, "/reference/");
    assert!(reference.contains("reference_sidebar: true"));
    let generated_new_record = generated_source(&output, "/reference/webhook-newRecord-post/");
    assert!(generated_new_record.contains("title: \"New Record\""));
    assert!(generated_new_record.contains("<h1 class=\"operation-title\">New Record</h1>"));
    let generated_named_record = generated_source(&output, "/reference/webhook-namedRecord-post/");
    assert!(generated_named_record.contains("title: \"Published record\""));
    assert!(generated_named_record.contains("<h1 class=\"operation-title\">Published record</h1>"));
    let generated_path_title = generated_source(&output, "/reference/getFirst/");
    assert!(generated_path_title.contains("<h1 class=\"operation-title\">Read first</h1>"));
    let generated_path_id_fallback = generated_source(&output, "/reference/putSecond/");
    assert!(generated_path_id_fallback.contains("<h1 class=\"operation-title\">putSecond</h1>"));

    let default_config = synthetic_config(
        &temp.path().join("default-input"),
        &json!({"openapi":"3.0.0", "info":{"title":"Default", "version":"1"}, "components":{"schemas":{}}, "paths":{}}),
        "# Default\n",
        "/reference/",
    );
    let default_output = temp.path().join("default-generated");
    generate(&default_config, &default_output, "").expect("generate without opt-in");
    assert!(
        !default_output
            .join("_data/akibiyori_reference_sidebar.json")
            .exists()
    );
    assert!(!generated_source(&default_output, "/reference/").contains("reference_sidebar: true"));
}

#[test]
fn compact_field_anchors_distinguish_shared_status_and_branch_occurrences_deterministically() {
    let temp = TempDir::new().expect("field occurrence input root");
    let spec = json!({
        "openapi": "3.1.0",
        "info": {"title": "Repeated schema occurrences", "version": "1"},
        "paths": {
            "/records": {
                "get": {
                    "operationId": "readRecords",
                    "responses": {
                        "200": {"$ref": "#/components/responses/Shared"},
                        "201": {"$ref": "#/components/responses/Shared"}
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "Branch": {"type": "object", "properties": {"name": {"type": "string"}}},
                "Thing": {"anyOf": [{"$ref": "#/components/schemas/Branch"}, {"$ref": "#/components/schemas/Branch"}]}
            },
            "responses": {
                "Shared": {
                    "description": "same schema for both statuses",
                    "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Thing"}}}
                }
            }
        }
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_data: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    config_data["reference"] = json!({"layout":"columns"});
    fs::write(&config, serde_json::to_vec_pretty(&config_data).unwrap()).unwrap();

    let first_output = temp.path().join("first-output");
    let second_output = temp.path().join("second-output");
    generate(&config, &first_output, "").expect("generate first occurrence set");
    generate(&config, &second_output, "").expect("generate second occurrence set");
    let first = generated_source(&first_output, "/reference/readRecords/");
    let second = generated_source(&second_output, "/reference/readRecords/");
    assert_eq!(
        first, second,
        "field identities are deterministic across builds"
    );
    let branch_field_ids = first
        .split("<section class=\"property-card\"")
        .skip(1)
        .filter(|card| card.contains("data-property-name=\"name\""))
        .map(|card| {
            card.split("id=\"api-field-")
                .nth(1)
                .expect("context-qualified field id")
                .split('\"')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        branch_field_ids.len(),
        4,
        "two branches across two statuses"
    );
    assert_eq!(
        branch_field_ids.iter().collect::<BTreeSet<_>>().len(),
        branch_field_ids.len(),
        "shared source pointers retain distinct branch and status identities"
    );
    for id in branch_field_ids {
        assert!(first.contains(&format!("href=\"#api-field-{id}\"")));
    }
}

#[test]
fn reference_sidebar_configuration_rejects_empty_labels_and_unknown_fields() {
    let temp = TempDir::new().expect("sidebar config validation root");
    let spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Config validation", "version": "1"},
        "components": {"schemas": {}},
        "paths": {}
    });
    let base = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let base_value: Value = serde_json::from_slice(&fs::read(&base).unwrap()).unwrap();
    for (label, options) in [
        ("overview", json!({"overview_label":"  "})),
        (
            "untagged",
            json!({"overview_label":"API", "untagged_label":""}),
        ),
        (
            "unknown",
            json!({"overview_label":"API", "unexpected":true}),
        ),
    ] {
        let mut config = base_value.clone();
        config["reference_sidebar"] = options;
        let path = temp.path().join(format!("{label}.json"));
        fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
        let output = temp.path().join(format!("{label}-output"));
        assert!(
            generate(&path, &output, "").is_err(),
            "invalid {label} setting must fail before generation"
        );
        assert!(
            !output.exists(),
            "invalid {label} setting creates no output"
        );
    }
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
    let overview = fs::read_to_string(root.served_dir.join("reference/index.html"))
        .expect("rendered Planets reference overview");
    assert_eq!(overview.matches("<h1>").count(), 1);
    assert!(overview.contains("<h1>Scalar Galaxy API reference</h1>"));
    assert!(overview.contains("ten HTTP path operations, one newPlanet webhook"));
    assert!(overview.contains("fifteen named models"));
    assert!(overview.contains("four documented reference-value corrections"));
    assert!(overview.contains("reference-generated-nav"));
    assert!(overview.contains("Scalar Example"));
    assert!(overview.contains("OpenAPI Example"));
    assert!(overview.contains("href=\"operations/\">operations</a>"));
    assert!(overview.contains("href=\"models/\">models</a>"));
    assert!(overview.contains("schema source sections display the parsed JSON representation"));
    assert!(!overview.contains("Observatory Catalog"));
    assert!(!overview.contains("It is not a copy of Fumadocs"));
    assert!(assert_model_field_links_resolve(&root.served_dir, "") >= 2);
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
    let webhook = fs::read_to_string(
        root.served_dir
            .join("reference/webhook-newPlanet-post/index.html"),
    )
    .expect("native bodyless webhook operation page");
    let webhook_source = generated_source(&root.source_dir, "/reference/webhook-newPlanet-post/");
    assert!(webhook_source.contains("title: \"New Planet\""));
    assert!(webhook.contains("<h1 class=\"operation-title\">New Planet</h1>"));
    assert!(!webhook.contains("<h1 class=\"operation-title\">webhook-newPlanet-post</h1>"));
    assert!(webhook.contains("Return a 200 status to indicate that the data was received"));
    assert!(!webhook.contains("response-examples"));
    let reference_sidebar: Value = serde_json::from_slice(
        &fs::read(
            root.source_dir
                .join("_data/akibiyori_reference_sidebar.json"),
        )
        .expect("generated reference sidebar data"),
    )
    .expect("reference sidebar JSON");
    let sidebar_groups = reference_sidebar["groups"].as_array().unwrap();
    assert_eq!(
        sidebar_groups
            .iter()
            .map(|group| group["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Planets", "Celestial Bodies", "Authentication"]
    );
    let planet_links = sidebar_groups[0]["links"].as_array().unwrap();
    assert_eq!(planet_links.len(), 7);
    assert_eq!(planet_links[0]["label"], "Get all planets");
    assert_eq!(planet_links[1]["label"], "Create a planet");
    assert_eq!(planet_links[3]["label"], "Delete a planet");
    assert_eq!(planet_links[4]["label"], "Update a planet");
    assert_eq!(planet_links[6]["label"], "New Planet");
    assert_eq!(
        planet_links[6]["href"],
        "/reference/webhook-newPlanet-post/"
    );
    assert!(
        planet_links[6]["accessible_label"]
            .as_str()
            .unwrap()
            .contains("newPlanet")
    );
    assert_eq!(reference_sidebar["overview"]["label"], "Scalar Example");
    let guide =
        fs::read_to_string(root.served_dir.join("index.html")).expect("ordinary guide page");
    assert!(guide.contains("class=\"sidebar-group\""));
    assert!(!guide.contains("reference-generated-nav"));

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

    let get_token_html = fs::read_to_string(root.served_dir.join("reference/getToken/index.html"))
        .expect("native getToken operation page");
    let retry_after_response = response_panel(&get_token_html, "429");
    assert!(retry_after_response.contains("class=\"response-contract-source\""));
    assert!(retry_after_response.contains("#/components/responses/TooManyRequests"));
    assert!(retry_after_response.contains("Retry-After"));
    assert!(retry_after_response.contains("The number of seconds to wait before retrying"));
    assert!(retry_after_response.contains("&quot;type&quot;: &quot;integer&quot;"));
    assert!(retry_after_response.contains("&quot;example&quot;: 60"));

    let upload_html = fs::read_to_string(root.served_dir.join("reference/uploadImage/index.html"))
        .expect("native uploadImage operation page");
    let cache_control_response = response_panel(&upload_html, "200");
    assert!(cache_control_response.contains("class=\"response-contract-source\""));
    assert!(cache_control_response.contains("#/components/responses/ImageUploaded"));
    assert!(cache_control_response.contains("X-Cache-Control"));
    assert!(cache_control_response.contains("#/components/headers/X-Cache-Control"));
    assert!(cache_control_response.contains("Cache control directives"));
    assert!(cache_control_response.contains("&quot;type&quot;: &quot;string&quot;"));
    assert!(
        cache_control_response.contains("&quot;example&quot;: &quot;max-age=3600, public&quot;")
    );

    let create_html = fs::read_to_string(root.served_dir.join("reference/createPlanet/index.html"))
        .expect("native Create operation page");
    let create_celestial_html = fs::read_to_string(
        root.served_dir
            .join("reference/createCelestialBody/index.html"),
    )
    .expect("native Create Celestial Body operation page");
    let planet_model = fs::read_to_string(
        root.served_dir
            .join("reference/models/scalar/Planet/index.html"),
    )
    .expect("native Planet model page");
    let temperature_heading = heading_id(&planet_model, "physicalProperties.temperature");
    assert!(create_celestial_html.contains(&format!(
        "href=\"/reference/models/scalar/Planet/#{temperature_heading}\""
    )));
    assert!(
        create_html.contains("<div class=\"operation-body prose flex-1 text-fd-foreground/90\">")
    );
    assert!(create_html.contains("<div class=\"operation-columns text-sm @container\">"));
    assert!(
        create_html
            .contains("<span class=\"method text-blue-400\">POST</span><code>/planets</code>")
    );
    assert!(create_html.contains("<h1 class=\"operation-title\">Create a planet</h1>"));
    assert!(
        create_html.find("operation-title").unwrap()
            < create_html.find("operation-columns").unwrap()
    );
    assert!(!create_html.contains("class=\"breadcrumbs\""));
    assert!(create_html.contains("Example Request"));
    assert!(create_html.contains("Example Response"));
    assert!(
        create_html.contains(
            "<summary class=\"example-origin-badge\" data-example-origin>Derived</summary>"
        )
    );
    assert!(create_html.contains("first declared authorization alternative"));
    assert!(create_html.contains("&lt;token&gt;"));
    assert!(create_html.contains("<summary>Source details</summary>"));
    assert!(!create_html.contains("class=\"example-note\""));
    assert!(create_html.contains("data-choice-label=\"openIdConnect\""));
    let source_start = create_html
        .rfind("<details class=\"operation-source\">")
        .expect("closed operation source disclosure");
    let source_end = source_start
        + create_html[source_start..]
            .find("</details>")
            .expect("closed operation source disclosure end")
        + "</details>".len();
    let operation_source = &create_html[source_start..source_end];
    assert!(operation_source.contains("Effective path item and operation source:"));
    assert!(operation_source.contains("&quot;x-post-response&quot;"));
    assert!(operation_source.contains("&quot;callbacks&quot;"));
    assert!(operation_source.contains("&quot;headers&quot;"));
    assert!(create_html.contains("data-property-filter disabled"));
    assert!(create_html.contains("data-property-name=\"id\""));
    assert!(create_html.contains("data-property-name=\"name\""));
    let anchor_ids = create_html
        .split("id=\"api-field-")
        .skip(1)
        .map(|tail| format!("api-field-{}", tail.split('\"').next().unwrap()))
        .collect::<Vec<_>>();
    let permalink_ids = create_html
        .split("href=\"#api-field-")
        .skip(1)
        .map(|tail| format!("api-field-{}", tail.split('\"').next().unwrap()))
        .collect::<Vec<_>>();
    assert!(
        !anchor_ids.is_empty(),
        "operation fields have context anchors"
    );
    assert_eq!(anchor_ids.len(), permalink_ids.len());
    assert_eq!(
        anchor_ids.iter().collect::<BTreeSet<_>>().len(),
        anchor_ids.len(),
        "different occurrences get distinct fragments"
    );
    assert!(anchor_ids.iter().all(|id| {
        create_html.contains(&format!("id=\"{id}\" tabindex=\"-1\" data-generated-field"))
    }));
    assert!(create_html.contains("aria-label=\"Permalink to id\""));
    assert!(
        create_html.contains("data-property-name=\"id\"")
            && create_html.matches("data-property-name=\"id\"").count() > 1
    );
    assert!(create_html.contains(
        "<code class=\"property-name\">id<span class=\"property-requirement required\">*</span></code>"
    ));
    assert!(create_html.contains(
        "<code class=\"property-name\">description<span class=\"property-requirement optional\">?</span></code>"
    ));
    assert!(create_html.contains("data-media-select disabled"));
    assert!(create_html.contains("data-example-select disabled"));
    assert!(create_html.contains("application/json"));
    assert!(create_html.contains("application/xml"));
    assert!(create_html.contains(
        "class=\"c-string c-quoted c-double c-json\"><span class=\"c-punctuation c-definition c-string c-begin c-json\">&quot;</span>id<span class=\"c-punctuation c-definition c-string c-end c-json\">&quot;</span></span>"
    ));
    assert!(create_html.contains(
        "class=\"c-meta c-structure c-dictionary c-value c-json\"><span class=\"c-punctuation c-separator c-dictionary c-key-value c-json\">:</span> <span class=\"c-string c-quoted c-double c-json\"><span class=\"c-punctuation c-definition c-string c-begin c-json\">&quot;</span>Mars<span class=\"c-punctuation c-definition c-string c-end c-json\">&quot;</span></span>"
    ));
    assert!(create_html.contains(
        "class=\"c-meta c-structure c-dictionary c-value c-json\"><span class=\"c-punctuation c-separator c-dictionary c-key-value c-json\">:</span> <span class=\"c-constant c-numeric c-json\">1</span>"
    ));
    assert!(create_html.contains("XML example unavailable"));
    assert!(create_html.contains("this reader does not invent XML from a JSON schema"));
    let callbacks_start = create_html
        .find("<section class=\"operation-section callback-readers\">")
        .expect("source callback reader section after responses");
    let callbacks_end = create_html
        .rfind("</div></section><details class=\"operation-source\">")
        .expect("parent source disclosure after callback section");
    let callbacks = &create_html[callbacks_start..callbacks_end];
    let callback_names = ["planetCreated", "planetCreationFailed", "planetExploded"];
    let callback_positions = callback_names.map(|name| {
        callbacks
            .find(&format!("data-callback-name=\"{name}\""))
            .unwrap()
    });
    assert!(callback_positions[0] < callback_positions[1]);
    assert!(callback_positions[1] < callback_positions[2]);
    assert!(callbacks.contains("{$request.body#/successCallbackUrl}"));
    assert!(callbacks.contains("{$request.body#/failureCallbackUrl}"));
    assert!(callbacks.contains("Information about which fields failed to validate"));
    assert!(callbacks.contains("Information about the newly exploded planet"));
    assert!(
        callbacks
            .contains("Your server should return this HTTP status code if no longer interested")
    );
    assert!(!callbacks.contains("auth-summary"));
    assert!(callbacks.contains("&quot;security&quot;: []"));
    assert!(callbacks.contains("data-code-label=\"JSON\""));
    assert!(callbacks.contains("callback-media-controls"));
    assert!(callbacks.contains("callback-section-heading\">Request Body</h3>"));
    assert!(callbacks.contains("callback-property-description-group"));
    assert!(callbacks.contains("callback-expression-variable"));
    assert!(!callbacks.contains("response-examples"));
    assert!(callbacks.contains("callback-source-pointer"));
    assert!(!callbacks.contains("data-code-label=\"cURL\""));
    assert_eq!(callbacks.matches("data-callback-reader").count(), 3);
    assert_eq!(manifest["operation_routes"].as_object().unwrap().len(), 11);
    assert!(
        !manifest["operation_routes"]
            .to_string()
            .contains("planetCreated")
    );
    let habitability_pointer = b"#/components/schemas/Planet/properties/habitabilityIndex";
    let habitability_anchor = format!(
        "field-{}",
        habitability_pointer
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let planet_model = fs::read_to_string(
        root.served_dir
            .join("reference/models/scalar/Planet/index.html"),
    )
    .expect("rendered Planet model page");
    assert!(planet_model.contains(&format!("id=\"{habitability_anchor}\">habitabilityIndex")));
    assert!(!planet_model.contains(&format!("id=\"{habitability_anchor}\" tabindex=")));
    assert!(create_html.contains("data-code-label=\"cURL\""));
    assert!(create_html.contains("class=\"c-variable c-function c-shell\">curl"));
    assert!(create_html.contains("class=\"c-variable c-parameter c-option c-shell\""));
    assert!(create_html.contains("</span>request</span> POST"));
    assert!(create_html.contains("https://galaxy.scalar.com/planets"));
    assert!(create_html.contains("class=\"c-meta c-structure c-dictionary c-key c-json\""));
    assert!(create_html.contains("class=\"c-meta c-structure c-dictionary c-value c-json\""));
    assert!(create_html.contains("class=\"c-constant c-numeric c-json\""));
    assert!(create_html.contains("class=\"c-variable c-function c-shell\""));
    assert!(create_html.contains("class=\"c-punctuation c-definition c-parameter c-shell\""));

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
    assert!(generic_source.contains("JSON representation parsed from YAML"));
    assert!(!generic_source.contains("Original schema JSON"));
    assert!(generic_source.contains("Dynamic reference static fallback"));
    assert!(generic_source.contains("#/components/schemas/PaginatedResource/$defs/itemType"));

    let prefix_dir = temp.path().join("prefix-build");
    let mut prefix_options = BuildOptions::new(&config, &prefix_dir);
    prefix_options.base_path = "/planets-reference".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix = build(&prefix_options).expect("actual prefixed Cobalt and Pagefind Planets build");
    let prefixed_overview = fs::read_to_string(prefix.served_dir.join("reference/index.html"))
        .expect("prefixed Planets reference overview");
    assert!(prefixed_overview.contains("href=\"operations/\">operations</a>"));
    assert!(prefixed_overview.contains("href=\"models/\">models</a>"));
    let prefixed_get = prefix.served_dir.join("reference/getAllData/index.html");
    assert!(prefixed_get.is_file());
    let prefixed_html = fs::read_to_string(prefixed_get).expect("prefixed GET /planets HTML");
    assert!(prefixed_html.contains("/planets-reference/reference/models/scalar/Planet/"));
    let prefixed_get_token =
        fs::read_to_string(prefix.served_dir.join("reference/getToken/index.html"))
            .expect("prefixed getToken operation page");
    let prefixed_retry_after = response_panel(&prefixed_get_token, "429");
    assert!(prefixed_retry_after.contains("Retry-After"));
    assert!(prefixed_retry_after.contains("&quot;example&quot;: 60"));
    let prefixed_upload =
        fs::read_to_string(prefix.served_dir.join("reference/uploadImage/index.html"))
            .expect("prefixed uploadImage operation page");
    let prefixed_cache_control = response_panel(&prefixed_upload, "200");
    assert!(prefixed_cache_control.contains("#/components/headers/X-Cache-Control"));
    assert!(
        prefixed_cache_control.contains("&quot;example&quot;: &quot;max-age=3600, public&quot;")
    );
    let prefixed_webhook = fs::read_to_string(
        prefix
            .served_dir
            .join("reference/webhook-newPlanet-post/index.html"),
    )
    .expect("prefixed native newPlanet webhook page");
    assert!(prefixed_webhook.contains("<h1 class=\"operation-title\">New Planet</h1>"));
    let prefixed_sidebar: Value = serde_json::from_slice(
        &fs::read(
            prefix
                .source_dir
                .join("_data/akibiyori_reference_sidebar.json"),
        )
        .expect("prefixed sidebar data"),
    )
    .expect("prefixed sidebar JSON");
    assert_eq!(
        prefixed_sidebar["overview"]["href"],
        "/planets-reference/reference/"
    );
    assert_eq!(
        prefixed_sidebar["groups"][0]["links"][0]["href"],
        "/planets-reference/reference/getAllData/"
    );
    let prefixed_planet_model = fs::read_to_string(
        prefix
            .served_dir
            .join("reference/models/scalar/Planet/index.html"),
    )
    .expect("prefixed Planet model page");
    assert!(
        prefixed_planet_model.contains(&format!("id=\"{habitability_anchor}\">habitabilityIndex"))
    );
    assert!(!prefixed_planet_model.contains(&format!("id=\"{habitability_anchor}\" tabindex=")));
    assert!(assert_model_field_links_resolve(&prefix.served_dir, "/planets-reference") >= 2);
    let prefixed_search = fs::read_to_string(prefix.served_dir.join("search/index.html"))
        .expect("prefixed search page");
    assert!(prefixed_search.contains("/planets-reference/pagefind/pagefind-ui.js"));
    assert!(prefixed_search.contains("/planets-reference/js/akibiyori-search.js"));
}

#[test]
fn callback_components_render_as_distinct_native_readers_without_new_routes() {
    let temp = TempDir::new().expect("independently authored callback fixture");
    let config = temp.path().join("config.json");
    let spec_path = temp.path().join("callbacks.json");
    let page_path = temp.path().join("index.md");
    let reference_page_path = temp.path().join("reference.md");
    let callback_schema = json!({
        "$id":"https://example.invalid/callback-resource",
        "$defs":{"Inner":{"type":"object","description":"JSON callback schema summary","properties":{"binding":{"type":"string","const":"correct resource"}}}},
        "$ref":"#/$defs/Inner"
    });
    let callback_field_schema = json!({
        "type":"object",
        "properties":{"binding":{"type":"string","const":"separate field schema"}}
    });
    let spec = json!({
        "openapi":"3.1.0",
        "info":{"title":"Callback component controls","version":"1"},
        "security":[{"token":[]}],
        "paths":{
            "/parent":{
                "post":{
                    "operationId":"createParent",
                    "callbacks":{
                        "firstDelivery":{"$ref":"#/components/callbacks/Shared"},
                        "secondDelivery":{"$ref":"#/components/callbacks/Shared"},
                        "x-notice":{"$ref":"#/components/callbacks/Shared"}
                    },
                    "responses":{"201":{"description":"Created"}}
                }
            }
        },
        "components":{
            "securitySchemes":{"token":{"type":"http","scheme":"bearer"}},
            "callbacks":{
                "Shared":{
                    "{$request.body#/firstUrl}":{
                        "post":{
                            "security":[],
                            "parameters":[{"name":"trace","in":"header","schema":callback_field_schema.clone()}],
                            "requestBody":{"content":{
                                "application/json":{"schema":callback_schema.clone(),"examples":{"sample":{"value":{"binding":"first"}}}},
                                "application/xml":{"schema":{"$ref":"#/components/schemas/Thing"}}
                            }},
                            "responses":{"200":{"$ref":"#/components/responses/Ack"},"204":{"description":"No content"}}
                        }
                    },
                    "{$request.query#/secondUrl}":{
                        "put":{
                            "security":[],
                            "requestBody":{"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Thing"}}}},
                            "responses":{"202":{"description":"Queued"}}
                        }
                    },
                    "{$request.query#/inlineUrl}":{
                        "patch":{
                            "security":[],
                            "responses":{"200":{
                                "description":"Inline acknowledgement",
                                "content":{"application/json":{
                                    "schema":callback_field_schema.clone(),
                                    "example":{"binding":"inline acknowledgement"}
                                }}
                            }}
                        }
                    },
                    "{$request.query#/unavailableUrl}":{
                        "delete":{
                            "security":[],
                            "responses":{"200":{"$ref":"#/components/responses/NoSample"}}
                        }
                    }
                }
            },
            "responses": {
                "Ack": {
                    "description": "Acknowledged",
                    "headers": {
                        "X<&-Trace": {
                            "description": "Literal header value <script>alert('header')</script>",
                            "schema": {
                                "type": "string",
                                "example": "</code><script>alert('example')</script>"
                            }
                        },
                        "X-Canonical": {
                            "$ref": "#/components/headers/AliasHeader"
                        }
                    },
                    "content": {
                        "application/json": {
                            "schema": callback_field_schema.clone(),
                            "examples": {
                                "confirmed": {
                                    "summary": "Confirmed acknowledgement",
                                    "value": {"binding": "acknowledged"}
                                }
                            }
                        }
                    }
                },
                "NoSample": {
                    "description": "Acknowledgement without a supplied XML sample",
                    "content": {
                        "application/xml": {"schema": {"type": "string"}}
                    }
                }
            },
            "headers": {
                "AliasHeader": {
                    "$ref": "#/components/headers/CanonicalHeader"
                },
                "CanonicalHeader": {
                    "description": "Resolved header description",
                    "schema": {
                        "type": "integer",
                        "example": 37
                    }
                }
            },
            "schemas":{"Thing":{"type":"object","description":"XML callback schema summary","properties":{"label":{"type":"string","examples":["value"]}}}}
        }
    });
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&spec).expect("serialize callback fixture"),
    )
    .expect("write callback fixture");
    fs::write(
        &page_path,
        "# Callback fixture\n\nA small independently authored reader control.\n",
    )
    .expect("write authored page");
    fs::write(&reference_page_path, "# Callback Fixture API\n").expect("write reference overview");
    let config_value = json!({
        "version":1,
        "title":"Callback Reader Fixture",
        "contracts":[{"id":"callback-fixture","label":"Callback Fixture","path":"callbacks.json"}],
        "pages":[
            {"source":"index.md","title":"Callback fixture","route":"/"},
            {"source":"reference.md","title":"Callback Fixture API","route":"/reference/"}
        ],
        "reference":{"layout":"columns"}
    });
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize fixture config"),
    )
    .expect("write fixture config");

    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real Cobalt and Pagefind callback fixture build");
    assert_eq!(report.generation.operation_pages, 1);
    assert_eq!(report.generation.model_pages, 1);
    let manifest: Value = serde_json::from_slice(
        &fs::read(report.source_dir.join("_data/akibiyori-manifest.json"))
            .expect("callback route manifest"),
    )
    .expect("callback manifest JSON");
    assert_eq!(manifest["operation_routes"].as_object().unwrap().len(), 1);
    assert!(
        !manifest["operation_routes"]
            .to_string()
            .contains("Delivery")
    );
    let html = fs::read_to_string(report.served_dir.join("reference/createParent/index.html"))
        .expect("native parent operation");
    let start = html
        .find("callback-readers")
        .expect("callback reader group");
    let end = html
        .rfind("</div></section><details class=\"operation-source\">")
        .expect("parent source disclosure after callback section");
    let callbacks = &html[start..end];
    let expected_order = [
        ("firstDelivery", "{$request.body#/firstUrl}", "POST"),
        ("firstDelivery", "{$request.query#/secondUrl}", "PUT"),
        ("firstDelivery", "{$request.query#/inlineUrl}", "PATCH"),
        (
            "firstDelivery",
            "{$request.query#/unavailableUrl}",
            "DELETE",
        ),
        ("secondDelivery", "{$request.body#/firstUrl}", "POST"),
        ("secondDelivery", "{$request.query#/secondUrl}", "PUT"),
        ("secondDelivery", "{$request.query#/inlineUrl}", "PATCH"),
        (
            "secondDelivery",
            "{$request.query#/unavailableUrl}",
            "DELETE",
        ),
        ("x-notice", "{$request.body#/firstUrl}", "POST"),
        ("x-notice", "{$request.query#/secondUrl}", "PUT"),
        ("x-notice", "{$request.query#/inlineUrl}", "PATCH"),
        ("x-notice", "{$request.query#/unavailableUrl}", "DELETE"),
    ];
    let positions = expected_order.map(|(name, expression, method)| {
        let marker = format!("data-callback-name=\"{name}\" data-callback-expression=\"{expression}\" data-callback-method=\"{method}\"");
        callbacks.find(&marker).unwrap_or_else(|| panic!("native callback marker {marker:?}"))
    });
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(callbacks.contains("data-media-select disabled"));
    assert!(callbacks.contains("data-choice-select disabled"));
    assert!(callbacks.contains("data-example-select disabled"));
    assert!(callbacks.contains("class=\"callback-request-subheader\"><span>Request Body</span>"));
    assert!(callbacks.contains("data-callback-description-slot"));
    assert!(callbacks.contains("data-callback-media-label"));
    assert!(callbacks.contains("class=\"callback-fallback-media\">application/json</p>"));
    assert!(callbacks.contains("class=\"callback-fallback-media\">application/xml</p>"));
    assert!(
        callbacks
            .contains("data-callback-schema-description><p>JSON callback schema summary</p></div>")
    );
    assert!(
        callbacks
            .contains("data-callback-schema-description><p>XML callback schema summary</p></div>")
    );
    assert!(
        callbacks
            .find("class=\"callback-request-subheader\"")
            .unwrap()
            < callbacks
                .find("<section class=\"example-variant\" data-example-panel")
                .unwrap(),
        "request variant panels follow the shared request-body controls"
    );
    let first_request_group = callbacks
        .find("<section class=\"example-card example-group\">")
        .expect("callback request example group");
    let first_request_panel = callbacks[first_request_group..]
        .find("<section class=\"example-variant\" data-example-panel")
        .map(|offset| first_request_group + offset)
        .expect("direct request example panel");
    let request_header = &callbacks[first_request_group..first_request_panel];
    assert_eq!(
        request_header
            .matches("<header class=\"example-card-header\">")
            .count(),
        1
    );
    assert_eq!(
        request_header
            .matches("class=\"callback-request-subheader\"")
            .count(),
        1
    );
    assert_eq!(request_header.matches("data-example-copy type=").count(), 1);
    assert!(
        request_header.find("callback-request-subheader").unwrap()
            < request_header.find("data-example-copy type=").unwrap()
    );
    assert!(
        request_header.ends_with("</div>"),
        "example panels remain direct children of their group"
    );
    assert!(callbacks.contains("data-code-label=\"JSON\""));
    assert!(callbacks.contains("data-media-label=\"application/xml\""));
    assert!(!callbacks.contains("data-code-label=\"cURL\""));
    assert!(!callbacks.contains("auth-summary"));
    assert!(callbacks.contains("No content"));
    assert!(callbacks.contains("firstUrl"));
    assert!(callbacks.contains("secondUrl"));
    assert!(callbacks.contains("data-callback-name=\"x-notice\""));
    assert!(callbacks.contains("https://example.invalid/callback-resource"));
    assert!(callbacks.contains("correct resource"));
    assert!(callbacks.contains("#/components/callbacks/Shared/"));
    assert_eq!(
        callbacks
            .matches("class=\"example-card example-group response-examples\"")
            .count(),
        9,
        "only callback expressions with resolved response media get an example card"
    );
    assert_eq!(
        callbacks
            .matches("data-example-label=\"confirmed\"")
            .count(),
        3
    );
    let ack_status = response_panel(callbacks, "200");
    assert!(ack_status.contains("class=\"response-contract-source\""));
    assert!(
        ack_status.contains("Resolved response source at <code>#/components/responses/Ack</code>")
    );
    assert!(callbacks.contains("&quot;$ref&quot;: &quot;#/components/responses/Ack&quot;"));
    assert!(ack_status.contains("#/components/headers/AliasHeader"));
    assert!(ack_status.contains("#/components/responses/Ack/headers/X&lt;&amp;-Trace"));
    assert!(ack_status.contains("Header <code>X&lt;&amp;-Trace</code>"));
    assert!(
        ack_status
            .contains("Literal header value &lt;script&gt;alert(&#39;header&#39;)&lt;/script&gt;")
    );
    assert!(
        ack_status.contains("&lt;/code&gt;&lt;script&gt;alert(&#39;example&#39;)&lt;/script&gt;")
    );
    assert!(ack_status.contains("#/components/responses/Ack/headers/X-Canonical"));
    assert!(ack_status.contains("&quot;$ref&quot;: &quot;#/components/headers/AliasHeader&quot;"));
    assert!(ack_status.contains("#/components/responses/Ack/headers/X-Canonical → #/components/headers/AliasHeader → #/components/headers/CanonicalHeader"));
    assert!(
        ack_status
            .contains("#/components/headers/AliasHeader → #/components/headers/CanonicalHeader")
    );
    assert!(ack_status.contains("Resolved header description"));
    assert!(ack_status.contains("&quot;example&quot;: 37"));
    assert!(!ack_status.contains("<script>alert('header')"));
    assert!(!ack_status.contains("<script>alert('example')"));
    assert_eq!(
        callbacks.matches("data-example-label=\"Example\"").count(),
        3
    );
    let unavailable_response_tags = callbacks
        .split("<section class=\"example-variant\"")
        .skip(1)
        .filter_map(|variant| variant.split_once('>').map(|(tag, _)| tag))
        .filter(|tag| {
            tag.contains("data-example-label=\"XML example unavailable\"")
                && tag.contains("data-example-unavailable=\"true\"")
                && tag.contains("data-response-example")
        })
        .count();
    assert_eq!(unavailable_response_tags, 3);
    let ids = html
        .split(" id=\"")
        .skip(1)
        .map(|tail| tail.split('"').next().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        ids.iter().collect::<BTreeSet<_>>().len(),
        ids.len(),
        "reused callback components still receive unique DOM IDs"
    );
    let callback_binding_ids = |document: &str| {
        let mut by_name = BTreeMap::<String, BTreeSet<String>>::new();
        let mut offset = 0;
        while let Some(relative) = document[offset..].find("data-property-name=\"binding\"") {
            let property_at = offset + relative;
            let property_tag = document[..property_at]
                .rfind("<section class=\"property-card\"")
                .expect("binding field property row");
            let tag_end = document[property_at..]
                .find('>')
                .expect("property row tag end")
                + property_at;
            let tag = &document[property_tag..tag_end];
            let field_id = tag
                .split("id=\"")
                .nth(1)
                .and_then(|value| value.split('"').next())
                .expect("generated field ID");
            let name_start = document[..property_tag]
                .rfind("data-callback-name=\"")
                .expect("enclosing callback occurrence")
                + "data-callback-name=\"".len();
            let name = document[name_start..]
                .split('"')
                .next()
                .expect("callback name");
            by_name
                .entry(name.to_owned())
                .or_default()
                .insert(field_id.to_owned());
            offset = property_at + "data-property-name=\"binding\"".len();
        }
        by_name
    };
    let original_binding_ids = callback_binding_ids(&html);
    for name in ["firstDelivery", "secondDelivery", "x-notice"] {
        assert_eq!(
            original_binding_ids[name].len(),
            4,
            "parameter, request media, referenced response, and inline response field anchors belong to {name}"
        );
    }
    assert!(
        original_binding_ids["firstDelivery"].is_disjoint(&original_binding_ids["secondDelivery"])
    );
    assert!(original_binding_ids["firstDelivery"].is_disjoint(&original_binding_ids["x-notice"]));
    assert!(original_binding_ids["secondDelivery"].is_disjoint(&original_binding_ids["x-notice"]));

    let mut reversed_spec = spec.clone();
    let callbacks = reversed_spec["paths"]["/parent"]["post"]["callbacks"]
        .as_object_mut()
        .expect("callback map");
    let entries = std::mem::take(callbacks);
    for (name, callback) in entries.into_iter().rev() {
        callbacks.insert(name, callback);
    }
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&reversed_spec).expect("serialize reordered callback map"),
    )
    .expect("write reordered callback map");
    let reversed_output = temp.path().join("reversed-callback-order");
    generate(&config, &reversed_output, "").expect("generate reordered callback map");
    let reversed_html = generated_source(&reversed_output, "/reference/createParent/");
    assert_eq!(
        callback_binding_ids(&reversed_html),
        original_binding_ids,
        "field identity stays attached to callback name when the callback map is reversed"
    );
    assert!(
        html.contains("data-callback-content"),
        "readers remain in static details markup without JavaScript"
    );

    let mut prefix_options = BuildOptions::new(&config, temp.path().join("prefix-native-build"));
    prefix_options.base_path = "/callback-prefix".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix_report = build(&prefix_options).expect("prefixed callback response build");
    let prefix_html = fs::read_to_string(
        prefix_report
            .served_dir
            .join("reference/createParent/index.html"),
    )
    .expect("prefixed callback operation");
    let prefix_callbacks_start = prefix_html
        .find("callback-readers")
        .expect("prefixed callback reader group");
    let prefix_callbacks_end = prefix_html
        .rfind("</div></section><details class=\"operation-source\">")
        .expect("prefixed parent source disclosure");
    let prefix_callbacks = &prefix_html[prefix_callbacks_start..prefix_callbacks_end];
    let prefix_ack = response_panel(prefix_callbacks, "200");
    assert!(
        prefix_ack
            .contains("#/components/headers/AliasHeader → #/components/headers/CanonicalHeader")
    );
    assert!(prefix_ack.contains("Header <code>X&lt;&amp;-Trace</code>"));
    assert!(
        prefix_ack.contains("&lt;/code&gt;&lt;script&gt;alert(&#39;example&#39;)&lt;/script&gt;")
    );
    assert_eq!(
        prefix_callbacks
            .matches("class=\"example-card example-group response-examples\"")
            .count(),
        9
    );
    assert_eq!(
        prefix_callbacks
            .matches("data-example-label=\"confirmed\"")
            .count(),
        3
    );
    assert_eq!(
        prefix_callbacks
            .matches("data-example-label=\"Example\"")
            .count(),
        3
    );
    let prefix_unavailable_responses = prefix_callbacks
        .split("<section class=\"example-variant\"")
        .skip(1)
        .filter_map(|variant| variant.split_once('>').map(|(tag, _)| tag))
        .filter(|tag| {
            tag.contains("data-example-label=\"XML example unavailable\"")
                && tag.contains("data-example-unavailable=\"true\"")
                && tag.contains("data-response-example")
        })
        .count();
    assert_eq!(prefix_unavailable_responses, 3);

    let mut source_config = config_value;
    source_config["reference"]["layout"] = json!("source");
    fs::write(
        &config,
        serde_json::to_vec(&source_config).expect("serialize source-layout config"),
    )
    .expect("write source-layout config");
    let source_output = temp.path().join("source-layout");
    let source_report = generate(&config, &source_output, "").expect("generate source layout");
    assert!(source_report.diagnostics.iter().any(|diagnostic| {
        diagnostic.pointer == "#/paths/~1parent/post/callbacks"
            && diagnostic
                .message
                .contains("not interpreted in the concise view")
    }));
    let source_operation = generated_source(&source_output, "/reference/createParent/");
    assert!(!source_operation.contains("data-callback-reader"));
}

#[test]
fn callback_map_name_with_extension_prefix_is_rendered_as_a_callback() {
    let temp = TempDir::new().expect("callback identifier fixture");
    let config = temp.path().join("config.json");
    let spec_path = temp.path().join("callbacks.json");
    fs::write(temp.path().join("index.md"), "# Reader\n").expect("write home page");
    fs::write(temp.path().join("reference.md"), "# API\n").expect("write reference page");
    let spec = json!({
        "openapi":"3.1.0",
        "info":{"title":"Callback identifier control","version":"1"},
        "paths":{
            "/parent":{
                "post":{
                    "operationId":"createParent",
                    "callbacks":{
                        "x-notice":{
                            "{$request.body#/callbackUrl}":{
                                "post":{
                                    "security":[],
                                    "requestBody":{"content":{"application/json":{
                                        "schema":{"type":"object","properties":{"received":{"type":"string"}}},
                                        "examples":{"delivered":{"value":{"received":"read-only"}}}
                                    }}},
                                    "responses":{"200":{"description":"Accepted"}}
                                }
                            }
                        }
                    }
                }
            }
        },
        "components":{"schemas":{}}
    });
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&spec).expect("serialize extension-prefixed callback name"),
    )
    .expect("write callback contract");
    fs::write(
        &config,
        serde_json::to_vec_pretty(&json!({
            "version":1,
            "title":"Callback Identifier Control",
            "contracts":[{"id":"callback-name","label":"Callback Name","path":"callbacks.json"}],
            "pages":[
                {"source":"index.md","title":"Reader","route":"/"},
                {"source":"reference.md","title":"API","route":"/reference/"}
            ],
            "reference":{"layout":"columns"}
        }))
        .expect("serialize callback config"),
    )
    .expect("write callback config");

    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real native callback identifier build");
    assert_eq!(report.generation.operation_pages, 1);
    let html = fs::read_to_string(report.served_dir.join("reference/createParent/index.html"))
        .expect("native parent operation page");
    assert_eq!(html.matches("data-callback-reader").count(), 1);
    assert!(html.contains("data-callback-name=\"x-notice\""));
    assert!(html.contains("{$request.body#/callbackUrl}"));
    assert!(html.contains("read-only"));
}

#[test]
fn callback_reference_and_shape_failures_are_contextual_and_write_nothing() {
    let temp = TempDir::new().expect("callback failure controls");
    let config = temp.path().join("config.json");
    let spec_path = temp.path().join("callbacks.json");
    fs::write(temp.path().join("index.md"), "# Reader\n").expect("write home page");
    fs::write(temp.path().join("reference.md"), "# API\n").expect("write reference page");
    fs::write(
        &config,
        serde_json::to_vec(&json!({
            "version":1,
            "title":"Callback Failure Controls",
            "contracts":[{"id":"callback-failure","label":"Callback Failure","path":"callbacks.json"}],
            "pages":[
                {"source":"index.md","title":"Reader","route":"/"},
                {"source":"reference.md","title":"API","route":"/reference/"}
            ],
            "reference":{"layout":"columns"}
        }))
        .expect("serialize callback failure config"),
    )
    .expect("write callback failure config");
    let base_spec = json!({
        "openapi":"3.1.0",
        "info":{"title":"Callback failure controls","version":"1"},
        "paths":{"/parent":{"post":{
            "operationId":"createParent",
            "callbacks":{"delivery":{"$ref":"#/components/callbacks/Callback"}},
            "responses":{"201":{"description":"Created"}}
        }}},
        "components":{
            "callbacks":{"Callback":{"{$request.body#/callbackUrl}":{
                "post":{"security":[],"responses":{"200":{"description":"Accepted","headers":{"X-Declared":{"description":"Header value","schema":{"type":"string","example":"ok"}}}}}}
            }}},
            "headers":{"Present":{"description":"Present header","schema":{"type":"string"}}},
            "schemas":{"Thing":{"type":"object"}}
        }
    });
    let assert_failure = |case: &str, spec: Value, expected: &str| {
        fs::write(
            &spec_path,
            serde_json::to_vec(&spec).expect("serialize failing callback spec"),
        )
        .expect("write failing callback spec");
        let output = temp.path().join(format!("out-{case}"));
        let error =
            generate(&config, &output, "").expect_err("invalid callback must fail generation");
        assert!(
            error.to_string().contains(expected),
            "{case} returned {error}"
        );
        assert!(
            !output.exists(),
            "{case} must fail before generated output is written"
        );
    };

    let mut missing = base_spec.clone();
    missing["paths"]["/parent"]["post"]["callbacks"]["delivery"] =
        json!({"$ref":"#/components/callbacks/Missing"});
    assert_failure("missing", missing, "has no target");

    let mut wrong_category = base_spec.clone();
    wrong_category["paths"]["/parent"]["post"]["callbacks"]["delivery"] =
        json!({"$ref":"#/components/schemas/Thing"});
    assert_failure(
        "wrong-category",
        wrong_category,
        "targets an unsupported object",
    );

    let mut cyclic = base_spec.clone();
    cyclic["components"]["callbacks"]["First"] = json!({"$ref":"#/components/callbacks/Second"});
    cyclic["components"]["callbacks"]["Second"] = json!({"$ref":"#/components/callbacks/First"});
    cyclic["paths"]["/parent"]["post"]["callbacks"]["delivery"] =
        json!({"$ref":"#/components/callbacks/First"});
    assert_failure("cyclic", cyclic, "cyclic callbacks reference chain");

    let mut missing_header = base_spec.clone();
    missing_header["components"]["callbacks"]["Callback"]["{$request.body#/callbackUrl}"]["post"]
        ["responses"]["200"]["headers"]["X-Declared"] =
        json!({"$ref":"#/components/headers/Missing"});
    assert_failure("missing-header", missing_header, "has no target");

    let mut wrong_category_header = base_spec.clone();
    wrong_category_header["components"]["callbacks"]["Callback"]["{$request.body#/callbackUrl}"]
        ["post"]["responses"]["200"]["headers"]["X-Declared"] =
        json!({"$ref":"#/components/schemas/Thing"});
    assert_failure(
        "wrong-category-header",
        wrong_category_header,
        "targets an unsupported object",
    );

    let mut cyclic_header = base_spec.clone();
    cyclic_header["components"]["headers"]["First"] = json!({"$ref":"#/components/headers/Second"});
    cyclic_header["components"]["headers"]["Second"] = json!({"$ref":"#/components/headers/First"});
    cyclic_header["components"]["callbacks"]["Callback"]["{$request.body#/callbackUrl}"]["post"]
        ["responses"]["200"]["headers"]["X-Declared"] =
        json!({"$ref":"#/components/headers/First"});
    assert_failure(
        "cyclic-header",
        cyclic_header,
        "cyclic headers reference chain",
    );

    let mut malformed_path_item = base_spec.clone();
    malformed_path_item["components"]["callbacks"]["Callback"] =
        json!({"{$request.body#/callbackUrl}":42});
    assert_failure("malformed-path-item", malformed_path_item, "path item at");

    let mut unsupported_path_item_ref = base_spec.clone();
    unsupported_path_item_ref["components"]["callbacks"]["Callback"] =
        json!({"{$request.body#/callbackUrl}":{"$ref":"#/paths/~1parent"}});
    assert_failure(
        "unsupported-path-item-ref",
        unsupported_path_item_ref,
        "path-item $ref is unsupported",
    );
}

#[test]
fn real_native_model_reference_links_use_emitted_ids_and_root_fallbacks() {
    let temp = TempDir::new().expect("synthetic native link root");
    let spec = json!({
        "openapi":"3.1.0",
        "info":{"title":"Canonical link controls","version":"1"},
        "paths":{
            "/references":{
                "get":{
                    "operationId":"readReferences",
                    "responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Z_References"}}}}}
                }
            }
        },
        "components":{"schemas":{
            "A_DisplayTarget":{
                "type":"object",
                "properties":{
                    "display name":{"type":"string"},
                    "list":{"type":"array","items":{"type":"object","properties":{"value":{"type":"string"}}}}
                },
                "$defs":{"N":{"type":"object","properties":{"value":{"type":"string"}}}}
            },
            "Z_References":{
                "type":"object",
                "properties":{
                    "encoded":{"$ref":"#/components/schemas/A_DisplayTarget/properties/display%20name"},
                    "item schema":{"$ref":"#/components/schemas/A_DisplayTarget/properties/list/items"},
                    "definition property":{"$ref":"#/components/schemas/A_DisplayTarget/$defs/N/properties/value"}
                }
            }
        }}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Canonical links\n",
        "/reference/",
    );

    let mut root_options = BuildOptions::new(&config, temp.path().join("root-build"));
    root_options.cobalt = native_program("COBALT", "cobalt");
    root_options.pagefind = native_program("PAGEFIND", "pagefind");
    let root = build(&root_options).expect("real root Cobalt and Pagefind build");
    let target = fs::read_to_string(
        root.served_dir
            .join("reference/models/test/A_DisplayTarget/index.html"),
    )
    .expect("rendered target model page");
    let references =
        fs::read_to_string(root.served_dir.join("reference/readReferences/index.html"))
            .expect("rendered operation using the referring model");
    let display_name_id = heading_id(&target, "display name");
    assert!(references.contains(&format!(
        "Schema reference: <code>#/components/schemas/A_DisplayTarget/properties/display%20name</code> → <a href=\"/reference/models/test/A_DisplayTarget/#{display_name_id}\">A_DisplayTarget</a> <code>#/components/schemas/A_DisplayTarget/properties/display name</code>"
    )));
    for canonical_pointer in [
        "#/components/schemas/A_DisplayTarget/properties/list/items",
        "#/components/schemas/A_DisplayTarget/$defs/N/properties/value",
    ] {
        assert!(references.contains(&format!(
            "Schema reference: <code>{canonical_pointer}</code> → <a href=\"/reference/models/test/A_DisplayTarget/\">A_DisplayTarget</a> <code>{canonical_pointer}</code>"
        )));
    }
    assert!(assert_model_field_links_resolve(&root.served_dir, "") >= 1);

    let mut prefix_options = BuildOptions::new(&config, temp.path().join("prefix-build"));
    prefix_options.base_path = "/canonical-links".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix = build(&prefix_options).expect("real prefixed Cobalt and Pagefind build");
    let prefix_target = fs::read_to_string(
        prefix
            .served_dir
            .join("reference/models/test/A_DisplayTarget/index.html"),
    )
    .expect("rendered prefixed target model page");
    let prefix_references = fs::read_to_string(
        prefix
            .served_dir
            .join("reference/readReferences/index.html"),
    )
    .expect("rendered prefixed operation using the referring model");
    let prefix_display_name_id = heading_id(&prefix_target, "display name");
    assert!(prefix_references.contains(&format!(
        "Schema reference: <code>#/components/schemas/A_DisplayTarget/properties/display%20name</code> → <a href=\"/canonical-links/reference/models/test/A_DisplayTarget/#{prefix_display_name_id}\">A_DisplayTarget</a> <code>#/components/schemas/A_DisplayTarget/properties/display name</code>"
    )));
    assert!(!prefix_references.contains("/canonical-links/canonical-links/"));
    for canonical_pointer in [
        "#/components/schemas/A_DisplayTarget/properties/list/items",
        "#/components/schemas/A_DisplayTarget/$defs/N/properties/value",
    ] {
        assert!(prefix_references.contains(&format!(
            "Schema reference: <code>{canonical_pointer}</code> → <a href=\"/canonical-links/reference/models/test/A_DisplayTarget/\">A_DisplayTarget</a> <code>{canonical_pointer}</code>"
        )));
    }
    assert!(assert_model_field_links_resolve(&prefix.served_dir, "/canonical-links") >= 1);
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
        "/css/akibiyori-tokens.css/",
        "/CSS/AKIBIYORI-TOKENS.CSS/",
        "/css/akibiyori-tokens.css/child/",
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

fn synthetic_yaml_config(dir: &Path, spec: &str, page: &str, route: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("create synthetic YAML input directory");
    fs::write(dir.join("spec.yaml"), spec).expect("write YAML contract");
    fs::write(dir.join("page.md"), page).expect("write authored page");
    let config = json!({
        "version": 1,
        "title": "Synthetic YAML contract reader",
        "contracts": [{"id":"test","label":"Test contract","path":"spec.yaml"}],
        "pages": [{"source":"page.md","title":"Reference","route":route}],
        "navigation": []
    });
    fs::write(
        dir.join("config.json"),
        serde_json::to_vec_pretty(&config).expect("serialize YAML input config"),
    )
    .expect("write YAML input config");
    dir.join("config.json")
}

#[test]
fn malformed_and_trailing_json_keep_the_exported_typed_error() {
    let valid_spec = json!({
        "openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}
    });
    for source in ["{\n  \"openapi\":", "{\"openapi\":\"3.0.0\"} trailing"] {
        let temp = TempDir::new().expect("malformed JSON root");
        let input = temp.path().join("input");
        let config = synthetic_config(&input, &valid_spec, "# Reference\n", "/reference/");
        let contract_path = input.join("spec.json");
        fs::write(&contract_path, source).expect("write malformed contract JSON");
        let output = temp.path().join("output");
        let error = generate(&config, &output, "").expect_err("invalid JSON fails preflight");
        let Error::Json {
            path,
            source: json_error,
        } = &error
        else {
            panic!("expected typed JSON error, got {error:?}");
        };
        assert_eq!(path, &contract_path);
        assert!(json_error.line() >= 1);
        assert!(json_error.column() >= 1);
        assert!(std::error::Error::source(&error).is_some());
        assert!(!output.exists(), "JSON error must leave output absent");
    }
}

#[test]
fn nested_json_decode_error_keeps_the_actual_serde_source() {
    let multiline_value = r#"{
  "openapi": "3.0.3",
  "paths": {},
  "components": {
    "schemas": {
      "Thing": {
        "properties": {
          "value": {
            "type": "string",
            "description": "\uD800"
          }
        }
      }
    }
  }
}"#;
    let root_key = r#"{"openapi":"3.0.3","paths":{},"components":{"schemas":{}},"\uD800":true}"#;
    let nested_key = r#"{"openapi":"3.0.3","paths":{},"components":{"schemas":{"Thing":{"properties":{"\uD800":{"type":"string"}}}}}}"#;
    let cases = [
        (
            multiline_value,
            serde_json::from_str::<String>(r#""\uD800""#)
                .expect_err("the real scalar decoder rejects a lone surrogate"),
            true,
        ),
        (
            root_key,
            serde_json::from_str::<Value>(root_key)
                .expect_err("the real document decoder rejects a lone surrogate key"),
            false,
        ),
        (
            nested_key,
            serde_json::from_str::<Value>(r#"{"\uD800":{"type":"string"}}"#)
                .expect_err("the real nested-object decoder rejects a lone surrogate key"),
            false,
        ),
    ];

    for (contract, expected_source, value_relative) in cases {
        let temp = TempDir::new().expect("late JSON decode error root");
        let input = temp.path().join("input");
        let valid_spec = json!({
            "openapi":"3.0.0","paths":{},"components":{"schemas":{"Thing":{}}}
        });
        let config = synthetic_config(&input, &valid_spec, "# Reference\n", "/reference/");
        let contract_path = input.join("spec.json");
        fs::write(&contract_path, contract).expect("write escaped surrogate contract");

        let output = temp.path().join("output");
        let error = generate(&config, &output, "")
            .expect_err("invalid decoded Unicode scalar fails preflight");
        let Error::Json {
            path,
            source: json_error,
        } = &error
        else {
            panic!("expected typed JSON error, got {error:?}");
        };
        assert_eq!(path, &contract_path);
        assert_eq!(json_error.classify(), expected_source.classify());
        assert_eq!(json_error.line(), expected_source.line());
        assert_eq!(json_error.column(), expected_source.column());
        if value_relative {
            assert_eq!(
                json_error.line(),
                1,
                "position is relative to the raw value"
            );
        }
        let source = std::error::Error::source(&error).expect("preserve serde error as source");
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
        assert!(!output.exists(), "JSON error must leave output absent");
    }
}

#[test]
fn dynamic_scope_follows_reference_traversal_not_lexical_ancestors_and_includes_current_resource() {
    let lexical_leak = json!({
        "openapi":"3.1.1","paths":{},
        "components":{"schemas":{
            "Use":{"$id":"https://example.test/Use","$ref":"https://example.test/B"},
            "ContainerA":{"$id":"https://example.test/A","$defs":{
                "outerT":{"$dynamicAnchor":"T","type":"string"},
                "B":{"$id":"https://example.test/B","$defs":{"fallback":{"$dynamicAnchor":"T","type":"number"}},"properties":{"value":{"$dynamicRef":"#T"}}}
            }}
        }}
    });
    let temp = TempDir::new().expect("lexical scope control root");
    let config = synthetic_config(
        &temp.path().join("input"),
        &lexical_leak,
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("lexical-source");
    generate(&config, &output, "").expect("reference traversal without ContainerA");
    let use_page = generated_source(&output, "/reference/models/test/Use/");
    let expected_static = "Dynamic reference static fallback: <code>#T</code> → <a href=\"/reference/models/test/ContainerA/\">ContainerA</a> <code>#/components/schemas/ContainerA/$defs/B/$defs/fallback</code>";
    assert!(
        use_page.contains(expected_static),
        "actual Use binding was not the B fallback: {use_page}"
    );
    assert!(!use_page.contains("Dynamic binding: <code>#T</code> → <a href=\"/reference/models/test/ContainerA/\">ContainerA</a> <code>#/components/schemas/ContainerA/$defs/outerT</code>"));

    let current_resource = json!({
        "openapi":"3.1.1","paths":{},
        "components":{"schemas":{
            "A":{"$id":"https://example.test/A","$defs":{"own":{"$dynamicAnchor":"T","type":"string"}},"properties":{"value":{"$dynamicRef":"https://example.test/B#T"}}},
            "B":{"$id":"https://example.test/B","$defs":{"fallback":{"$dynamicAnchor":"T","type":"number"}}}
        }}
    });
    let current_config = synthetic_config(
        &temp.path().join("current-input"),
        &current_resource,
        "# Reference\n",
        "/reference/",
    );
    let current_output = temp.path().join("current-source");
    generate(&current_config, &current_output, "").expect("current-resource dynamic binding");
    let a_page = generated_source(&current_output, "/reference/models/test/A/");
    let expected_current = "Dynamic binding: <code>https://example.test/B#T</code> → <a href=\"/reference/models/test/A/\">A</a> <code>#/components/schemas/A/$defs/own</code> (outer dynamic anchor)";
    assert!(
        a_page.contains(expected_current),
        "current A resource did not bind before B fallback: {a_page}"
    );
}

#[test]
fn openapi_30_ref_ignores_nested_sibling_schemas_but_31_checks_them() {
    for (version, should_fail) in [("3.0.3", false), ("3.1.1", true)] {
        let spec = json!({
            "openapi":version,"paths":{},
            "components":{"schemas":{
                "A":{"$ref":"#/components/schemas/B","properties":{"ignored":{"$ref":"#/components/schemas/Missing"}}},
                "B":{"type":"string"}
            }}
        });
        let temp = TempDir::new().expect("legacy sibling root");
        let config = synthetic_config(
            &temp.path().join("input"),
            &spec,
            "# Reference\n",
            "/reference/",
        );
        let output = temp.path().join("source");
        let result = generate(&config, &output, "");
        if should_fail {
            let error =
                result.expect_err("3.1 schema siblings participate in reference validation");
            assert!(error.to_string().contains("Missing"));
            assert!(
                !output.exists(),
                "3.1 missing sibling target fails before writes"
            );
        } else {
            result.expect("3.0 ignores siblings when the schema itself is a Reference Object");
            let source = generated_source(&output, "/reference/models/test/A/");
            assert!(source.contains("Schema reference"));
            assert!(
                source.contains("Missing"),
                "ignored sibling remains in escaped source JSON"
            );
            assert!(source.contains("Original schema JSON"));
        }
    }
}

#[test]
fn signed_yaml_radix_values_remain_strings_and_unsigned_forms_fail_before_output() {
    let temp = TempDir::new().expect("YAML radix cases");
    let accepted = "openapi: 3.1.1\npaths: {}\ncomponents:\n  schemas:\n    Thing:\n      description: -0xFF\n      example: +0o77\n";
    let config = synthetic_yaml_config(
        &temp.path().join("accepted"),
        accepted,
        "# Reference\n",
        "/reference/",
    );
    let output = temp.path().join("accepted-source");
    generate(&config, &output, "").expect("signed radix values are plain strings");
    let model = generated_source(&output, "/reference/models/test/Thing/");
    assert!(model.contains("-0xFF"));
    assert!(model.contains("+0o77"));

    for number in ["0xFF", "0o77"] {
        let spec = format!(
            "openapi: 3.1.1\npaths: {{}}\ncomponents:\n  schemas:\n    Thing:\n      description: {number}\n"
        );
        let config = synthetic_yaml_config(
            &temp.path().join(format!("unsupported-{number}")),
            &spec,
            "# Reference\n",
            "/reference/",
        );
        let output = temp.path().join(format!("unsupported-{number}-source"));
        let error = generate(&config, &output, "").expect_err("unsigned radix number unsupported");
        assert!(
            error
                .to_string()
                .contains("unsupported or invalid plain YAML number")
        );
        assert!(
            !output.exists(),
            "unsupported number must fail before writes"
        );
    }
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
fn real_native_columns_keep_parameters_auth_choices_schema_context_and_sample_limits() {
    let temp = TempDir::new().expect("columns controls temp root");
    let spec = json!({
        "openapi":"3.0.3",
        "security":[{"bearer":[]},{"key":[]}],
        "paths":{
            "/thing/{id}":{
                "parameters":[{"name":"id","in":"path","required":true,"description":"path base value","schema":{"type":"string"}}],
                "get":{
                    "operationId":"read-thing",
                    "security":[],
                    "parameters":[{"name":"id","in":"path","required":true,"description":"operation override value","schema":{"type":"string"}},{"name":"filter","in":"query","description":"filter description","schema":{"type":"string"}}],
                    "responses":{
                        "200":{"description":"success","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ReadThing"}},"text/plain":{"schema":{"type":"string"}}}},
                        "202":{"description":"accepted","content":{"application/json":{"schema":{"type":"string"}}}}
                    }
                }
            },
            "/thing":{
                "post":{
                    "operationId":"create-thing",
                    "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/WriteThing"}}}},
                    "responses":{"201":{"description":"created","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ReadThing"}}}}}
                }
            }
        },
        "webhooks":{"events":{
                "post":{
                    "operationId":"event-hook",
                    "requestBody":{"content":{"application/json":{"schema":{"type":"object","properties":{"event":{"type":"string"}}}}}},
                    "responses":{"204":{"description":"received"}}
                }
            }
        },
        "components":{
            "securitySchemes":{"bearer":{"type":"http","scheme":"bearer","bearerFormat":"opaque"},"key":{"type":"apiKey","in":"header","name":"X-Reading-Key"}},
            "schemas":{
                "Label":{"type":"string","description":"A referenced label"},
                "ReadThing":{"type":"object","required":["id"],"properties":{"id":{"type":"integer","minimum":5},"label":{"$ref":"#/components/schemas/Label"},"secret":{"type":"string","writeOnly":true},"choice":{"oneOf":[{"type":"string","title":"Text choice"},{"type":"integer","title":"Integer choice"}]},"children":{"type":"array","items":{"$ref":"#/components/schemas/Label"}},"attributes":{"type":"object","additionalProperties":{"type":"string"}}}},
                "WriteThing":{"type":"object","required":["name","secret"],"properties":{"name":{"type":"string"},"createdAt":{"type":"string","readOnly":true},"secret":{"type":"string","writeOnly":true},"label":{"$ref":"#/components/schemas/Label"},"extra":{"type":"object","additionalProperties":{"type":"string"}}}}
            }
        }
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize columns config"),
    )
    .expect("write columns config");
    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("real Cobalt and Pagefind columns output");

    let read = fs::read_to_string(report.served_dir.join("reference/read-thing/index.html"))
        .expect("read GET operation");
    let parameter_start = read
        .find("<section class=\"operation-section\"><h2>Parameters</h2>")
        .unwrap();
    let parameter_end = parameter_start
        + read[parameter_start..]
            .find("</section></section>")
            .unwrap()
        + "</section></section>".len();
    let parameter_cards = &read[parameter_start..parameter_end];
    assert!(parameter_cards.contains("operation override value"));
    assert!(!parameter_cards.contains("path base value"));
    assert!(parameter_cards.contains("filter description"));
    assert!(read.contains("No authentication is required;"));
    let read_auth = &read[..read
        .find("<section class=\"operation-section\"><h2>Request Body")
        .unwrap_or(read.len())];
    assert!(!read_auth.contains("aria-label=\"Authentication alternative\""));
    assert!(read.contains("data-choice-label=\"200\""));
    assert!(read.contains("data-choice-label=\"202\""));
    assert!(read.contains("aria-label=\"Response status\""));
    assert!(read.contains("data-media-label=\"text/plain\""));
    assert!(read.contains("oneOf (2)"));
    assert!(read.contains("oneOf alternative 1"));
    assert!(read.contains("A referenced label"));
    assert!(read.contains("Additional property values"));
    assert!(
        read.contains("class=\"schema-nested property-nested-inline\"")
            && read.contains("<span class=\"property-type\">array</span>"),
        "{read}"
    );
    assert!(!read.contains("data-property-name=\"secret\""));
    assert!(read.contains("Example unavailable"));
    assert!(read.contains("minimum=5"), "{read}");
    assert!(
        read.contains("#/components/schemas/ReadThing/properties/id/minimum"),
        "{read}"
    );
    assert!(!read.contains("&quot;id&quot;: 0"));

    let create = fs::read_to_string(report.served_dir.join("reference/create-thing/index.html"))
        .expect("read POST operation");
    assert!(create.contains("data-choice-label=\"bearer\""));
    assert!(create.contains("data-choice-label=\"key\""));
    assert!(create.contains("aria-label=\"Authorization requirement\""));
    assert!(create.contains("Authorization: Bearer &lt;token&gt;"));
    assert!(create.contains("bearerFormat"));
    assert!(create.contains("X-Reading-Key"));
    assert!(create.contains("data-property-name=\"secret\""));
    assert!(!create.contains("data-property-name=\"createdAt\""));
    assert!(create.contains("data-property-name=\"label\""));
    assert!(!create.contains("data-media-select"));
    assert!(!create.contains("aria-label=\"Response status\""));

    let webhook = fs::read_to_string(report.served_dir.join("reference/event-hook/index.html"))
        .expect("read webhook operation");
    assert!(webhook.contains("caller-configured destination"));
    assert!(webhook.contains("Webhook: events"));
    assert!(!webhook.contains("curl --request"));
}

#[test]
fn real_native_kubernetes_specimen_preserves_source_population_and_data_fields() {
    let temp = TempDir::new().expect("Kubernetes specimen build root");
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/kubernetes/config.json");
    let mut options = BuildOptions::new(&config, temp.path().join("kubernetes-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("actual Kubernetes Cobalt and Pagefind build");

    assert_eq!(report.generation.authored_pages, 3);
    assert_eq!(report.generation.operation_pages, 13);
    assert_eq!(report.generation.model_pages, 41);
    assert!(assert_model_field_links_resolve(&report.served_dir, "") >= 1);
    let manifest: Value = serde_json::from_slice(
        &fs::read(report.source_dir.join("_data/akibiyori-manifest.json"))
            .expect("Kubernetes manifest"),
    )
    .expect("valid Kubernetes manifest JSON");
    let operation_routes = manifest["operation_routes"].as_object().unwrap();
    assert_eq!(operation_routes.len(), 13);
    assert_eq!(manifest["model_routes"].as_object().unwrap().len(), 41);
    let mut operation_html = String::new();
    for route in operation_routes.values().filter_map(Value::as_str) {
        operation_html.push_str(
            &fs::read_to_string(
                report
                    .served_dir
                    .join(route.trim_start_matches('/'))
                    .join("index.html"),
            )
            .expect("rendered operation page"),
        );
    }
    for media_type in [
        "*/*",
        "application/apply-patch+cbor",
        "application/apply-patch+yaml",
        "application/cbor",
        "application/cbor-seq",
        "application/json",
        "application/json-patch+json",
        "application/json;stream=watch",
        "application/merge-patch+json",
        "application/strategic-merge-patch+json",
        "application/vnd.kubernetes.protobuf",
        "application/vnd.kubernetes.protobuf;stream=watch",
        "application/yaml",
    ] {
        assert!(
            operation_html.contains(&format!("data-media-label=\"{media_type}\"")),
            "source media label {media_type:?} remains visible"
        );
    }
    for status in ["200", "201", "202", "401"] {
        assert!(
            operation_html.contains(&format!("data-choice-label=\"{status}\"")),
            "source response status {status} remains visible"
        );
    }

    let create_route =
        manifest["operation_routes"]["createApiextensionsV1CustomResourceDefinition"]
            .as_str()
            .expect("Create CustomResourceDefinition route");
    let create = fs::read_to_string(
        report
            .served_dir
            .join(create_route.trim_start_matches('/'))
            .join("index.html"),
    )
    .expect("rendered Create CustomResourceDefinition page");
    let wildcard = example_variant_for_media(&create, "*/*");
    assert!(wildcard.contains("data-example-unavailable=\"true\""));
    assert!(wildcard.contains("does not encode JSON schema values as that media type"));
    assert!(!wildcard.contains("data-code-label=\"cURL\""));
    assert!(!wildcard.contains("--header 'Content-Type: */*'"));
    assert!(create.contains("data-media-label=\"application/json\""));
    assert!(create.contains("data-media-label=\"application/cbor\""));
    assert!(create.contains("data-media-label=\"application/vnd.kubernetes.protobuf\""));
    assert!(create.contains("data-media-label=\"application/yaml\""));

    let model_route = manifest["model_routes"][
        "apiextensions:io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.JSONSchemaProps"
    ]
    .as_str()
    .expect("JSONSchemaProps model route");
    let model = fs::read_to_string(
        report
            .served_dir
            .join(model_route.trim_start_matches('/'))
            .join("index.html"),
    )
    .expect("rendered JSONSchemaProps model page");
    for field in [
        "$ref",
        "$schema",
        "oneOf",
        "anyOf",
        "not",
        "allOf",
        "properties",
        "default",
    ] {
        assert!(
            model.contains(&format!("data-property-name=\"{field}\"")),
            "JSONSchemaProps data field {field:?} remains a field"
        );
    }
    assert!(model.contains("Specification Draft 4"));
    assert!(model.contains("Original schema JSON"));

    let validation_route = manifest["model_routes"]["apiextensions:io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.CustomResourceValidation"]
        .as_str()
        .expect("CustomResourceValidation model route");
    let validation = fs::read_to_string(
        report
            .served_dir
            .join(validation_route.trim_start_matches('/'))
            .join("index.html"),
    )
    .expect("rendered CustomResourceValidation model page");
    assert!(validation.contains("data-property-name=\"openAPIV3Schema\""));
    assert!(validation.contains("All schemas must apply"));
    assert!(validation.contains(
        "#/components/schemas/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.CustomResourceValidation/properties/openAPIV3Schema/allOf/0"
    ));
    assert!(validation.contains(
        "href=\"/reference/models/apiextensions/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.JSONSchemaProps/\""
    ));

    for (property, source_pointer) in [
        (
            "definitions",
            "#/components/schemas/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.JSONSchemaProps/properties/definitions/additionalProperties",
        ),
        (
            "properties",
            "#/components/schemas/io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.JSONSchemaProps/properties/properties/additionalProperties",
        ),
    ] {
        let start = model
            .find(&format!("data-property-name=\"{property}\""))
            .expect("map field is rendered");
        let next = model[start + 1..]
            .find("data-property-name=\"")
            .map(|offset| start + 1 + offset)
            .unwrap_or(model.len());
        let field = &model[start..next];
        assert!(
            field.contains("Additional properties"),
            "{property}: {field}"
        );
        assert!(field.contains(source_pointer), "{property}: {field}");
        assert!(field.contains("JSONSchemaProps"), "{property}: {field}");
    }
    let not_start = model
        .find("data-property-name=\"not\"")
        .expect("not data field is rendered");
    let not_end = model[not_start + 1..]
        .find("data-property-name=\"")
        .map(|offset| not_start + 1 + offset)
        .unwrap_or(model.len());
    let not_field = &model[not_start..not_end];
    assert!(not_field.contains("JSONSchemaProps"), "{not_field}");
    assert!(not_field.contains("/properties/not"), "{not_field}");
    assert!(!not_field.contains("schema-composition"), "{not_field}");
    assert!(model.contains("Schema reading stopped at its bounded depth, node, or cycle guard"));
    assert!(model.contains("schema-expansion-note"));

    assert!(create.contains("class=\"schema-composition\""));
    assert!(create.contains("Additional property values"));
    assert!(create.contains("Schema reading stopped at its bounded depth, node, or cycle guard"));

    let mut prefix_options =
        BuildOptions::new(&config, temp.path().join("kubernetes-prefix-build"));
    prefix_options.base_path = "/kubernetes-reference".to_owned();
    prefix_options.cobalt = native_program("COBALT", "cobalt");
    prefix_options.pagefind = native_program("PAGEFIND", "pagefind");
    let prefix =
        build(&prefix_options).expect("actual Kubernetes prefix Cobalt and Pagefind build");
    assert_eq!(prefix.generation.operation_pages, 13);
    assert_eq!(prefix.generation.model_pages, 41);
    assert!(assert_model_field_links_resolve(&prefix.served_dir, "/kubernetes-reference") >= 1);
}

fn example_variant_for_media<'a>(html: &'a str, media_type: &str) -> &'a str {
    let marker = format!("data-media-type=\"{media_type}\"");
    let at = html
        .find(&marker)
        .unwrap_or_else(|| panic!("example variant for {media_type:?} is rendered"));
    let start = html[..at]
        .rfind("<section class=\"example-variant\"")
        .expect("example variant start");
    let mut cursor = start;
    let mut depth = 0usize;
    loop {
        let next_open = html[cursor..]
            .find("<section")
            .map(|offset| cursor + offset);
        let next_close = html[cursor..]
            .find("</section>")
            .map(|offset| cursor + offset);
        match (next_open, next_close) {
            (Some(open), Some(close)) if open < close => {
                depth += 1;
                cursor = open + "<section".len();
            }
            (_, Some(close)) => {
                depth -= 1;
                cursor = close + "</section>".len();
                if depth == 0 {
                    return &html[start..cursor];
                }
            }
            _ => panic!("example variant closes"),
        }
    }
}

fn visible_text(fragment: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for character in fragment.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[test]
fn native_examples_only_derive_and_offer_curl_for_concrete_json_media() {
    let temp = TempDir::new().expect("media eligibility temp root");
    let spec = json!({
        "openapi":"3.0.0",
        "info":{"title":"Media eligibility","version":"1"},
        "components":{"schemas":{}},
        "paths":{
            "/json":{"post":{"operationId":"createJson","requestBody":{"required":true,"content":{
                "application/json":{"schema":{"type":"object","properties":{"name":{"type":"string"}}}},
                "model/gltf+json":{"schema":{"type":"object","properties":{"asset":{"type":"string"}}}}
            }},"responses":{"204":{"description":"accepted"}}}},
            "/cbor-derived":{"post":{"operationId":"createCborDerived","requestBody":{"required":true,"content":{
                "application/cbor":{"schema":{"type":"object","properties":{"name":{"type":"string"}}}}
            }},"responses":{"204":{"description":"accepted"}}}},
            "/cbor-provided":{"post":{"operationId":"createCborProvided","requestBody":{"required":true,"content":{
                "application/cbor":{"schema":{"type":"object","properties":{"name":{"type":"string"}}},"example":{"name":"source-value"}}
            }},"responses":{"204":{"description":"accepted"}}}},
            "/wildcard":{"post":{"operationId":"createWildcard","requestBody":{"required":true,"content":{
                "*/*":{"schema":{"type":"object","properties":{"name":{"type":"string"}}}},
                "application/*+json":{"schema":{"type":"object","properties":{"name":{"type":"string"}}}}
            }},"responses":{"204":{"description":"accepted"}}}}
        }
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write columns config");

    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("actual native media eligibility build");

    let json_html = fs::read_to_string(report.served_dir.join("reference/createJson/index.html"))
        .expect("read JSON operation");
    assert!(json_html.contains("data-media-label=\"application/json\""));
    assert!(json_html.contains("data-code-label=\"cURL\""));
    assert!(json_html.contains("data-media-label=\"model/gltf+json\""));
    assert!(json_html.contains("data-code-label=\"cURL\""));
    assert!(json_html.contains("Content-Type"));
    assert!(json_html.contains("model/gltf+json"));

    let cbor_derived = fs::read_to_string(
        report
            .served_dir
            .join("reference/createCborDerived/index.html"),
    )
    .expect("read schema-only CBOR operation");
    assert!(cbor_derived.contains("data-media-label=\"application/cbor\""));
    assert!(cbor_derived.contains("data-example-unavailable=\"true\""));
    assert!(cbor_derived.contains("does not encode JSON schema values as that media type"));
    assert!(cbor_derived.contains("/requestBody/content/application~1cbor/schema"));
    assert!(!cbor_derived.contains("data-code-label=\"cURL\""));
    assert!(!cbor_derived.contains("data-code-panel"));

    let cbor_provided = fs::read_to_string(
        report
            .served_dir
            .join("reference/createCborProvided/index.html"),
    )
    .expect("read source-provided CBOR operation");
    assert!(cbor_provided.contains("data-example-provided=\"true\""));
    assert!(cbor_provided.contains("data-code-label=\"Source value (JSON)\""));
    assert!(cbor_provided.contains("aria-label=\"Source value (JSON) example code\""));
    assert!(cbor_provided.contains("JSON representation of the source value"));
    assert!(cbor_provided.contains("source-value"));
    assert!(!cbor_provided.contains("data-code-label=\"cURL\""));
    assert!(!cbor_provided.contains("--header 'Content-Type: application/cbor'"));

    let wildcard = fs::read_to_string(
        report
            .served_dir
            .join("reference/createWildcard/index.html"),
    )
    .expect("read wildcard operations");
    assert!(wildcard.contains("data-media-label=\"*/*\""));
    assert!(wildcard.contains("data-media-label=\"application/*+json\""));
    assert_eq!(
        wildcard
            .matches("data-example-unavailable=\"true\"")
            .count(),
        2
    );
    assert!(!wildcard.contains("data-code-label=\"cURL\""));
}

#[test]
fn native_response_examples_keep_raw_media_separate_from_status_panel_keys() {
    let temp = TempDir::new().expect("response media temp root");
    let spec = json!({
        "openapi":"3.0.0",
        "info":{"title":"Response media identity","version":"1"},
        "components":{"schemas":{}},
        "paths":{
            "/read-response-media":{"get":{
                "operationId":"readResponseMedia",
                "responses":{
                    "200":{"description":"JSON response","content":{
                        "application/json; charset=utf-8":{"example":{"kind":"json"}}
                    }},
                    "201":{"description":"JSON-based response","content":{
                        "model/gltf+json":{"example":{"kind":"gltf"}}
                    }},
                    "202":{"description":"Opaque response","content":{
                        "application/cbor; version=1":{"example":{"kind":"cbor"}}
                    }}
                }
            }}
        }
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize config"),
    )
    .expect("write columns config");

    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("actual native response media build");
    let html = fs::read_to_string(
        report
            .served_dir
            .join("reference/readResponseMedia/index.html"),
    )
    .expect("read response media operation");

    let json = example_variant_for_media(&html, "200:application/json; charset=utf-8");
    let json_text = visible_text(json);
    assert!(json.contains("data-code-label=\"JSON\""), "{json}");
    assert!(json_text.contains("\"kind\": \"json\""), "{json_text}");
    assert!(!json.contains("Source value (JSON)"), "{json}");
    assert!(
        !json.contains("JSON representation of the source value"),
        "{json}"
    );

    let gltf = example_variant_for_media(&html, "201:model/gltf+json");
    let gltf_text = visible_text(gltf);
    assert!(gltf.contains("data-code-label=\"JSON\""), "{gltf}");
    assert!(gltf_text.contains("\"kind\": \"gltf\""), "{gltf_text}");
    assert!(!gltf.contains("Source value (JSON)"), "{gltf}");
    assert!(
        !gltf.contains("JSON representation of the source value"),
        "{gltf}"
    );

    let cbor = example_variant_for_media(&html, "202:application/cbor; version=1");
    let cbor_text = visible_text(cbor);
    assert!(
        cbor.contains("data-code-label=\"Source value (JSON)\""),
        "{cbor}"
    );
    assert!(cbor_text.contains("\"kind\": \"cbor\""), "{cbor_text}");
    assert!(
        cbor_text.contains("JSON representation of the source value, not an encoding of application/cbor; version=1"),
        "{cbor_text}"
    );
    assert!(!cbor_text.contains("202:application/cbor"), "{cbor_text}");
    assert!(!html.contains("data-code-label=\"cURL\""), "{html}");
}

#[test]
fn native_samples_apply_direction_across_reference_chains_and_keep_media_examples_opaque() {
    let temp = TempDir::new().expect("direction-chain temp root");
    let spec = json!({
        "openapi":"3.1.1",
        "paths":{
            "/direction":{
                "post":{
                    "operationId":"write-direction",
                    "description":"See [authority](https:///example.test/x).",
                    "requestBody":{"required":true,"description":"Write a direction value","content":{"application/json":{"schema":{"$ref":"#/components/schemas/WriteEntity"}}}},
                    "responses":{"200":{"description":"Read a direction value","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ReadEntity"}}}}}
                }
            },
            "/opaque":{
                "post":{
                    "operationId":"write-opaque-example",
                    "requestBody":{"required":true,"content":{"application/json":{
                        "schema":{"type":"object","properties":{"directRead":{"type":"string","readOnly":true},"directWrite":{"type":"string","writeOnly":true}}},
                        "example":{"directRead":"opaque-read","directWrite":"opaque-write"}
                    }}},
                    "responses":{"204":{"description":"Accepted"}}
                }
            },
            "/bounded":{
                "post":{
                    "operationId":"bounded-sample",
                    "requestBody":{"required":true,"content":{"application/json":{"schema":{
                        "type":"object","required":["value"],"properties":{"value":{"$ref":"#/components/schemas/BoundRoot"}}
                    }}}},
                    "responses":{"204":{"description":"Accepted"}}
                }
            }
        },
        "components":{"schemas":{
            "ReadRoot":{"$ref":"#/components/schemas/ReadMiddle"},
            "ReadMiddle":{"$ref":"#/components/schemas/ReadLeaf","readOnly":true},
            "ReadLeaf":{"type":"string"},
            "WriteRoot":{"$ref":"#/components/schemas/WriteMiddle"},
            "WriteMiddle":{"$ref":"#/components/schemas/WriteLeaf","writeOnly":true},
            "WriteLeaf":{"type":"string"},
            "BoundRoot":{"$ref":"#/components/schemas/BoundLeaf","minimum":5},
            "BoundLeaf":{"type":"integer"},
            "WriteEntity":{"type":"object","required":["directRead","chainedRead","directWrite","chainedWrite"],"properties":{
                "directRead":{"type":"string","readOnly":true,"example":"ignored-direct-read"},
                "chainedRead":{"$ref":"#/components/schemas/ReadRoot","example":"ignored-chained-read"},
                "directWrite":{"type":"string","writeOnly":true,"example":"included-direct-write"},
                "chainedWrite":{"$ref":"#/components/schemas/WriteRoot"}
            }},
            "ReadEntity":{"type":"object","properties":{
                "directRead":{"type":"string","readOnly":true,"example":"included-direct-read"},
                "chainedRead":{"$ref":"#/components/schemas/ReadRoot"},
                "directWrite":{"type":"string","writeOnly":true,"example":"ignored-direct-write"},
                "chainedWrite":{"$ref":"#/components/schemas/WriteRoot"}
            }}
        }}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read direction config"))
            .expect("direction config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize direction config"),
    )
    .expect("write direction config");
    let mut options = BuildOptions::new(&config, temp.path().join("native-build"));
    options.cobalt = native_program("COBALT", "cobalt");
    options.pagefind = native_program("PAGEFIND", "pagefind");
    let report = build(&options).expect("actual native directional examples build");

    let create = fs::read_to_string(
        report
            .served_dir
            .join("reference/write-direction/index.html"),
    )
    .expect("read directional Create HTML");
    assert!(create.contains("authority"), "{create}");
    assert!(
        !create.contains("href=\"https:///example.test/x\""),
        "{create}"
    );
    let request_start = create
        .find("<section class=\"operation-section\"><h2>Request Body</h2>")
        .expect("request body section");
    let response_start = create[request_start..]
        .find("<section class=\"operation-section\"><h2>Response Body</h2>")
        .map(|offset| request_start + offset)
        .expect("response section after request body");
    let request_schema = &create[request_start..response_start];
    let response_schema = &create[response_start..];
    assert!(request_schema.contains("data-property-name=\"directWrite\""));
    assert!(request_schema.contains("data-property-name=\"chainedWrite\""));
    assert!(!request_schema.contains("data-property-name=\"directRead\""));
    assert!(!request_schema.contains("data-property-name=\"chainedRead\""));
    assert!(!request_schema.contains("class=\"property-examples\""));
    assert!(response_schema.contains("data-property-name=\"directRead\""));
    assert!(response_schema.contains("data-property-name=\"chainedRead\""));
    assert!(!response_schema.contains("data-property-name=\"directWrite\""));
    assert!(!response_schema.contains("data-property-name=\"chainedWrite\""));
    let request_json = example_json_panel(&create, "Example Request");
    assert!(request_json.contains("directWrite"), "{request_json}");
    assert!(request_json.contains("chainedWrite"), "{request_json}");
    assert!(
        request_json.contains("included-direct-write"),
        "{request_json}"
    );
    assert!(!request_json.contains("directRead"), "{request_json}");
    assert!(!request_json.contains("chainedRead"), "{request_json}");
    let response_json = example_json_panel(&create, "Example Response");
    assert!(response_json.contains("directRead"), "{response_json}");
    assert!(response_json.contains("chainedRead"), "{response_json}");
    assert!(
        response_json.contains("included-direct-read"),
        "{response_json}"
    );
    assert!(!response_json.contains("directWrite"), "{response_json}");
    assert!(!response_json.contains("chainedWrite"), "{response_json}");

    let opaque = fs::read_to_string(
        report
            .served_dir
            .join("reference/write-opaque-example/index.html"),
    )
    .expect("read opaque example HTML");
    let opaque_json = example_json_panel(&opaque, "Example Request");
    assert!(opaque_json.contains("opaque-read"), "{opaque_json}");
    assert!(opaque_json.contains("opaque-write"), "{opaque_json}");
    assert!(opaque.contains("data-example-provided=\"true\""));

    let bounded = fs::read_to_string(
        report
            .served_dir
            .join("reference/bounded-sample/index.html"),
    )
    .expect("read bounded sample HTML");
    assert!(
        bounded.contains("data-property-name=\"value\""),
        "{bounded}"
    );
    assert!(
        bounded.contains("Cannot derive a sample satisfying minimum=5 without a source value."),
        "{bounded}"
    );
    assert!(
        bounded.contains("#/components/schemas/BoundRoot/minimum"),
        "{bounded}"
    );
    assert!(bounded.contains("data-example-origin>Unavailable</summary>"));
    assert!(bounded.contains("data-example-unavailable=\"true\""));
    assert!(!bounded.contains("data-code-panel"), "{bounded}");
}

fn example_json_panel<'a>(html: &'a str, card_label: &str) -> &'a str {
    let card = html
        .find(card_label)
        .unwrap_or_else(|| panic!("example card {card_label:?} is present"));
    let panel = html[card..]
        .find("data-code-label=\"JSON\"")
        .map(|offset| card + offset)
        .unwrap_or_else(|| panic!("JSON code panel for {card_label:?} is present"));
    let end = html[panel..]
        .find("</section>")
        .map(|offset| panel + offset)
        .unwrap_or_else(|| panic!("JSON code panel for {card_label:?} closes"));
    &html[panel..end]
}

#[test]
fn compact_operation_schema_distinguishes_ref_dialects_and_branch_only_examples() {
    for (version, has_sibling) in [("3.0.3", false), ("3.1.1", true)] {
        let temp = TempDir::new().expect("compact sibling temp root");
        let spec = json!({
            "openapi":version,
            "paths":{"/thing":{"post":{
                "operationId":"create-thing",
                "requestBody":{"required":true,"content":{"application/json":{"schema":{
                    "$ref":"#/components/schemas/Base",
                    "description":"LOCAL REF SIBLING NOTE"
                }}}},
                "responses":{"200":{"description":"ok"}}
            }}},
            "components":{"schemas":{"Base":{"type":"object","properties":{"name":{"type":"string"}}}}}
        });
        let config = synthetic_config(
            &temp.path().join("input"),
            &spec,
            "# Reference\n",
            "/reference/",
        );
        let mut config_value: Value =
            serde_json::from_slice(&fs::read(&config).expect("read config")).expect("config JSON");
        config_value["reference"] = json!({"layout":"columns"});
        fs::write(
            &config,
            serde_json::to_vec_pretty(&config_value).expect("serialize columns config"),
        )
        .expect("write columns config");
        let output = temp.path().join("generated");
        generate(&config, &output, "").expect("generate compact reference source");
        let page = generated_source(&output, "/reference/create-thing/");
        if has_sibling {
            assert!(page.contains("LOCAL REF SIBLING NOTE"), "{page}");
            assert!(
                page.contains("class=\"schema-reference-siblings\""),
                "{page}"
            );
        } else {
            assert!(
                !page.contains("class=\"schema-card-description\">LOCAL REF SIBLING NOTE"),
                "{page}"
            );
            assert!(
                !page.contains("class=\"schema-reference-siblings\""),
                "{page}"
            );
        }
    }

    let temp = TempDir::new().expect("branch sample temp root");
    let spec = json!({
        "openapi":"3.1.1",
        "paths":{"/thing":{"post":{
            "operationId":"create-branch",
            "requestBody":{"required":true,"content":{"application/json":{"schema":{
                "oneOf":[
                    {"type":"object","title":"Selected branch","required":["name"],"properties":{"name":{"type":"string"}}},
                    {"type":"object","title":"Unselected branch","required":["id"],"properties":{"id":{"type":"integer","minimum":5}}}
                ]
            }}}},
            "responses":{"200":{"description":"ok"}}
        }}},
        "components":{"schemas":{}}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read branch config"))
            .expect("config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize branch config"),
    )
    .expect("write branch config");
    let output = temp.path().join("generated");
    generate(&config, &output, "").expect("generate branch-only sample");
    let page = generated_source(&output, "/reference/create-branch/");
    assert!(
        page.contains("oneOf/0: selected oneOf alternative 1 as a reading example"),
        "{page}"
    );
    assert!(
        page.contains("&quot;name&quot;: &quot;string&quot;"),
        "{page}"
    );
    assert!(!page.contains("Example unavailable"), "{page}");

    let temp = TempDir::new().expect("dynamic compact temp root");
    let dynamic_spec = json!({
        "openapi":"3.1.1",
        "paths":{"/thing":{"post":{
            "operationId":"create-dynamic",
            "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Root"}}}},
            "responses":{"200":{"description":"ok"}}
        }}},
        "components":{"schemas":{
            "Root":{"$id":"https://example.test/Root","$dynamicAnchor":"T","type":"string","required":["payload"],"properties":{"payload":{"$ref":"https://example.test/Inner"}}},
            "Inner":{"$id":"https://example.test/Inner","$defs":{"fallback":{"$dynamicAnchor":"T","type":"integer"}},"type":"object","required":["value"],"properties":{"value":{"$dynamicRef":"#T"}}}
        }}
    });
    let config = synthetic_config(
        &temp.path().join("input"),
        &dynamic_spec,
        "# Reference\n",
        "/reference/",
    );
    let mut config_value: Value =
        serde_json::from_slice(&fs::read(&config).expect("read dynamic config"))
            .expect("dynamic config JSON");
    config_value["reference"] = json!({"layout":"columns"});
    fs::write(
        &config,
        serde_json::to_vec_pretty(&config_value).expect("serialize dynamic config"),
    )
    .expect("write dynamic config");
    let output = temp.path().join("generated");
    generate(&config, &output, "").expect("generate compact dynamic resource traversal");
    let page = generated_source(&output, "/reference/create-dynamic/");
    let value_card_start = page
        .find("data-property-name=\"value\"")
        .expect("compact dynamic value row");
    let value_card = &page
        [value_card_start..page[value_card_start..].find("</section>").unwrap() + value_card_start];
    assert!(
        value_card.contains("<span class=\"property-type\">string</span>"),
        "{value_card}"
    );
    assert!(
        !value_card.contains("<span class=\"property-type\">integer</span>"),
        "{value_card}"
    );
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
fn native_cobalt_keeps_generated_sidebar_labels_as_literal_liquid_data() {
    let temp = TempDir::new().expect("literal sidebar Cobalt root");
    let input = temp.path().join("input");
    let spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Literal tags", "version": "1"},
        "tags": [{"name": "records", "x-displayName": "Records {{ 7 | plus: 1 }}"}],
        "paths": {
            "/records": {
                "get": {
                    "operationId": "read-records",
                    "summary": "Read {{ 8 | plus: 1 }}",
                    "tags": ["records"],
                    "responses": {"200": {"description": "ok"}}
                }
            }
        },
        "components": {"schemas": {}}
    });
    let config = synthetic_config(&input, &spec, "# Overview\n", "/reference/");
    let mut data: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    data["reference_sidebar"] = json!({"overview_label": "Scalar {{ 2 | plus: 3 }}"});
    fs::write(&config, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    let source = temp.path().join("source");
    generate(&config, &source, "").expect("generate literal sidebar source");
    let result = Command::new(native_program("COBALT", "cobalt"))
        .current_dir(&source)
        .args(["build", "--config", "_cobalt.yml", "--destination", "_site"])
        .output()
        .expect("run native Cobalt with generated navigation");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(source.join("_site/reference/index.html"))
        .expect("Cobalt generated reference navigation");
    assert!(html.contains("Scalar {{ 2 | plus: 3 }}"));
    assert!(html.contains("Records {{ 7 | plus: 1 }}"));
    assert!(html.contains("Read {{ 8 | plus: 1 }}"));
    assert!(!html.contains("Scalar 5"));
    assert!(!html.contains("Records 8"));
    assert!(!html.contains("Read 9"));
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

fn showcase_manifest_with_absolute_configs(root: &Path, path: &Path) -> Value {
    let mut manifest: Value = serde_json::from_slice(
        &fs::read(root.join("akibiyori-showcase.json")).expect("read showcase manifest"),
    )
    .expect("parse showcase manifest");
    for site in manifest["sites"].as_array_mut().expect("sites array") {
        let config = site["config"].as_str().expect("config path");
        site["config"] = json!(root.join(config).display().to_string());
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("serialize test manifest"),
    )
    .expect("write test manifest");
    manifest
}

#[test]
fn showcase_context_is_opt_in_public_only_and_composes_global_urls() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("akibiyori-docs.json");
    let manifest = root.join("akibiyori-showcase.json");
    let temp = TempDir::new().expect("temporary showcase generation");
    let output = temp.path().join("showcase");
    let options = ShowcaseOptions {
        manifest,
        site: "docs".to_owned(),
    };

    generate_with_showcase(&config, &output, "/preview", &options)
        .expect("generate Docs with showcase context");
    let projection: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_showcase.json")).expect("showcase data"),
    )
    .expect("parse showcase projection");
    assert_eq!(projection["docs"]["href"], "/preview/docs/");
    assert_eq!(
        projection["categories"][0]["href"],
        "/preview/demos/openapi/"
    );
    assert_eq!(projection["current_label"], "Akibiyori Docs");
    let sites = projection["sites"].as_array().expect("API sites");
    assert_eq!(sites.len(), 4);
    assert_eq!(sites[1]["href"], "/preview/demos/planets/reference/");
    assert!(sites.iter().all(|site| site.get("config").is_none()));
    let serialized = serde_json::to_string(&projection).unwrap();
    assert!(!serialized.contains(&root.display().to_string()));
    assert!(!serialized.contains("akibiyori-showcase.json"));

    let legacy_output = temp.path().join("without-context");
    generate(&config, &legacy_output, "/preview").expect("legacy generation remains available");
    assert!(!legacy_output.join("_data/akibiyori_showcase.json").exists());
}

#[test]
fn showcase_validation_rejects_wrong_owner_before_output_writes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("akibiyori-docs.json");
    let temp = TempDir::new().expect("temporary invalid showcase");
    let manifest_path = temp.path().join("showcase.json");
    let mut manifest = showcase_manifest_with_absolute_configs(root, &manifest_path);
    manifest["categories"][0]["site"] = json!("planets");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let output = temp.path().join("must-remain-absent");
    let options = ShowcaseOptions {
        manifest: manifest_path,
        site: "docs".to_owned(),
    };
    let error = generate_with_showcase(&config, &output, "", &options)
        .expect_err("category route belongs to Docs, not the existing Planets site");
    assert!(
        error
            .to_string()
            .contains("not authored by its named owner")
    );
    assert!(
        !output.exists(),
        "owner failure happens before output writes"
    );
}

#[test]
fn showcase_manifest_rejects_invalid_graphs_without_mutating_outputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("akibiyori-docs.json");
    let temp = TempDir::new().expect("temporary invalid manifests");

    fn unknown_field(value: &mut Value) {
        value["unexpected"] = json!(true);
    }
    fn wrong_version(value: &mut Value) {
        value["version"] = json!(2);
    }
    fn duplicate_site(value: &mut Value) {
        value["sites"][1]["id"] = value["sites"][0]["id"].clone();
    }
    fn duplicate_mount(value: &mut Value) {
        value["sites"][1]["mount"] = value["sites"][0]["mount"].clone();
    }
    fn unknown_category(value: &mut Value) {
        value["sites"][1]["category"] = json!("missing");
    }
    fn empty_categories(value: &mut Value) {
        value["categories"] = json!([]);
    }
    fn control_label(value: &mut Value) {
        value["sites"][1]["label"] = json!("bad\nlabel");
    }
    fn unused_category(value: &mut Value) {
        for site in value["sites"].as_array_mut().unwrap().iter_mut().skip(1) {
            site.as_object_mut().unwrap().remove("category");
        }
    }
    fn missing_required_docs_site(value: &mut Value) {
        value["docs"].as_object_mut().unwrap().remove("site");
    }
    fn null_required_docs(value: &mut Value) {
        value["docs"] = Value::Null;
    }
    fn unchanged(_: &mut Value) {}

    struct InvalidCase {
        name: &'static str,
        mutate: fn(&mut Value),
        current_site: &'static str,
    }
    let cases = [
        InvalidCase {
            name: "unknown-field",
            mutate: unknown_field,
            current_site: "docs",
        },
        InvalidCase {
            name: "wrong-version",
            mutate: wrong_version,
            current_site: "docs",
        },
        InvalidCase {
            name: "duplicate-site",
            mutate: duplicate_site,
            current_site: "docs",
        },
        InvalidCase {
            name: "duplicate-mount",
            mutate: duplicate_mount,
            current_site: "docs",
        },
        InvalidCase {
            name: "unknown-category",
            mutate: unknown_category,
            current_site: "docs",
        },
        InvalidCase {
            name: "empty-categories",
            mutate: empty_categories,
            current_site: "docs",
        },
        InvalidCase {
            name: "control-label",
            mutate: control_label,
            current_site: "docs",
        },
        InvalidCase {
            name: "unused-category",
            mutate: unused_category,
            current_site: "docs",
        },
        InvalidCase {
            name: "missing-required-docs-site",
            mutate: missing_required_docs_site,
            current_site: "docs",
        },
        InvalidCase {
            name: "null-required-docs",
            mutate: null_required_docs,
            current_site: "docs",
        },
        InvalidCase {
            name: "unknown-current-site",
            mutate: unchanged,
            current_site: "missing",
        },
    ];
    for case in cases {
        let manifest_path = temp.path().join(format!("{}.json", case.name));
        let mut manifest = showcase_manifest_with_absolute_configs(root, &manifest_path);
        (case.mutate)(&mut manifest);
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let output = temp.path().join(format!("{}-output", case.name));
        let options = ShowcaseOptions {
            manifest: manifest_path,
            site: case.current_site.to_owned(),
        };
        assert!(
            generate_with_showcase(&config, &output, "", &options).is_err(),
            "invalid manifest case {} must fail",
            case.name
        );
        assert!(
            !output.exists(),
            "{} failure leaves absent output unchanged",
            case.name
        );
    }

    let manifest_path = temp.path().join("invalid-empty-output.json");
    let mut manifest = showcase_manifest_with_absolute_configs(root, &manifest_path);
    manifest["categories"] = json!([]);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = temp.path().join("preexisting-empty-output");
    fs::create_dir(&output).unwrap();
    let options = ShowcaseOptions {
        manifest: manifest_path,
        site: "docs".to_owned(),
    };
    assert!(generate_with_showcase(&config, &output, "", &options).is_err());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn showcase_accepts_optional_category_and_same_config_at_distinct_mounts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("akibiyori-docs.json");
    let temp = TempDir::new().expect("temporary valid manifest variants");
    let manifest_path = temp.path().join("valid-variants.json");
    let mut manifest = showcase_manifest_with_absolute_configs(root, &manifest_path);
    manifest["sites"][1]["category"] = Value::Null;
    manifest["sites"][2]
        .as_object_mut()
        .unwrap()
        .remove("category");
    let mut planets_copy = manifest["sites"][2].clone();
    planets_copy["id"] = json!("planets-copy");
    planets_copy["label"] = json!("Planet API copy");
    planets_copy["mount"] = json!("/demos/planets-copy");
    planets_copy["category"] = json!("openapi");
    manifest["sites"].as_array_mut().unwrap().push(planets_copy);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let output = temp.path().join("valid-output");
    let options = ShowcaseOptions {
        manifest: manifest_path,
        site: "docs".to_owned(),
    };
    generate_with_showcase(&config, &output, "/preview", &options)
        .expect("optional category values and reused config at a distinct mount are valid");
    let data: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_showcase.json")).unwrap())
            .unwrap();
    assert_eq!(data["sites"].as_array().unwrap().len(), 3);
    assert!(data["sites"].as_array().unwrap().iter().any(|site| {
        site["id"] == "planets-copy" && site["href"] == "/preview/demos/planets-copy/reference/"
    }));
}

#[test]
fn legacy_build_options_literal_and_cli_showcase_pair_remain_supported() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let options = BuildOptions {
        config: root.join("akibiyori-docs.json"),
        output: root.join("target/s3-compatibility-never-built"),
        base_path: String::new(),
        cobalt: native_program("COBALT", "cobalt"),
        pagefind: native_program("PAGEFIND", "pagefind"),
    };
    let _ = options;

    let temp = TempDir::new().expect("temporary CLI outputs");
    let output = temp.path().join("paired");
    let library_output = temp.path().join("library");
    let library_options = ShowcaseOptions {
        manifest: root.join("akibiyori-showcase.json"),
        site: "docs".to_owned(),
    };
    generate_with_showcase(
        &root.join("akibiyori-docs.json"),
        &library_output,
        "",
        &library_options,
    )
    .expect("direct library projection succeeds");
    let result = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args([
            "generate",
            "--config",
            root.join("akibiyori-docs.json").to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--showcase-manifest",
            root.join("akibiyori-showcase.json").to_str().unwrap(),
            "--showcase-site",
            "docs",
        ])
        .current_dir(root)
        .output()
        .expect("run actual paired CLI");
    assert!(
        result.status.success(),
        "paired CLI succeeds: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let cli_projection: Value = serde_json::from_slice(
        &fs::read(output.join("_data/akibiyori_showcase.json")).expect("read CLI projection"),
    )
    .expect("CLI projection is JSON");
    let library_projection: Value = serde_json::from_slice(
        &fs::read(library_output.join("_data/akibiyori_showcase.json"))
            .expect("read library projection"),
    )
    .expect("library projection is JSON");
    assert_eq!(cli_projection, library_projection);

    let invalid_output = temp.path().join("unpaired");
    let result = Command::new(env!("CARGO_BIN_EXE_akibiyori"))
        .args([
            "generate",
            "--config",
            root.join("akibiyori-docs.json").to_str().unwrap(),
            "--output",
            invalid_output.to_str().unwrap(),
            "--showcase-manifest",
            root.join("akibiyori-showcase.json").to_str().unwrap(),
        ])
        .current_dir(root)
        .output()
        .expect("run actual unpaired CLI");
    assert!(!result.status.success());
    assert!(!invalid_output.exists());
}

#[cfg(unix)]
#[test]
fn showcase_owner_compares_canonical_config_and_provided_root() {
    use std::os::unix::fs::symlink;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = TempDir::new().expect("temporary config aliases");
    let alias_root = temp.path().join("file-alias");
    fs::create_dir(&alias_root).unwrap();
    let docs_alias = alias_root.join("akibiyori-docs.json");
    symlink(root.join("akibiyori-docs.json"), &docs_alias).unwrap();

    let manifest_path = temp.path().join("file-alias-manifest.json");
    let mut manifest = showcase_manifest_with_absolute_configs(root, &manifest_path);
    manifest["sites"][0]["config"] = json!(docs_alias.display().to_string());
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    let options = ShowcaseOptions {
        manifest: manifest_path,
        site: "docs".to_owned(),
    };
    let output = temp.path().join("file-alias-output");
    let error = generate_with_showcase(&root.join("akibiyori-docs.json"), &output, "", &options)
        .expect_err("same canonical file with a different provided-input root is rejected");
    assert!(error.to_string().contains("config file and input root"));
    assert!(!output.exists());

    let planets_config = root.join("examples/planets.json");
    let wrong_current_output = temp.path().join("wrong-current-owner");
    let error = generate_with_showcase(
        &planets_config,
        &wrong_current_output,
        "/demos/planets",
        &options,
    )
    .expect_err("the selected current owner must match the provided config");
    assert!(error.to_string().contains("config file and input root"));
    assert!(!wrong_current_output.exists());

    let planets_manifest = temp.path().join("planets-manifest.json");
    showcase_manifest_with_absolute_configs(root, &planets_manifest);
    let planets_options = ShowcaseOptions {
        manifest: planets_manifest,
        site: "planets".to_owned(),
    };
    let wrong_base_output = temp.path().join("wrong-base");
    let error = generate_with_showcase(
        &planets_config,
        &wrong_base_output,
        "/unrelated",
        &planets_options,
    )
    .expect_err("the current base must end in the selected owner's mount");
    assert!(error.to_string().contains("exact suffix"));
    assert!(!wrong_base_output.exists());

    let directory_alias = temp.path().join("directory-alias");
    symlink(root, &directory_alias).unwrap();
    let aliased_manifest = directory_alias.join("akibiyori-showcase.json");
    let accepted_output = temp.path().join("directory-alias-output");
    let options = ShowcaseOptions {
        manifest: aliased_manifest,
        site: "docs".to_owned(),
    };
    generate_with_showcase(
        &directory_alias.join("akibiyori-docs.json"),
        &accepted_output,
        "",
        &options,
    )
    .expect("directory symlink and relative manifest paths preserve the same canonical root");
}
