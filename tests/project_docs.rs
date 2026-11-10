use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;
use url::Url;

const BASE_PATH: &str = "/akibiyori-docs";
const LOCAL_HOST: &str = "akibiyori.invalid";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn script_path() -> PathBuf {
    repository_root().join("scripts/build-demo-site.sh")
}

fn native_program(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn run_script(mode: &str, output: &Path, pagefind: &str) -> Output {
    run_script_with_manifest(mode, output, pagefind, None)
}

fn run_script_with_manifest(
    mode: &str,
    output: &Path,
    pagefind: &str,
    showcase_manifest: Option<&Path>,
) -> Output {
    let mut command = Command::new(script_path());
    command
        .args([mode, output.to_str().expect("UTF-8 output path")])
        .env("AKIBIYORI", env!("CARGO_BIN_EXE_akibiyori"))
        .env("COBALT", native_program("COBALT", "cobalt"))
        .env("PAGEFIND", pagefind)
        .current_dir(repository_root());
    if let Some(manifest) = showcase_manifest {
        command.env("SHOWCASE_MANIFEST", manifest);
    }
    command.output().expect("run the real demo-site script")
}

fn successful_build(output: Output) {
    assert!(
        output.status.success(),
        "native site build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_cli_build(config: &Path, output: &Path, base_path: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_akibiyori"));
    command
        .args([
            "build",
            "--config",
            config.to_str().expect("UTF-8 config path"),
            "--output",
            output.to_str().expect("UTF-8 output path"),
            "--cobalt",
            &native_program("COBALT", "cobalt"),
            "--pagefind",
            &native_program("PAGEFIND", "pagefind"),
        ])
        .current_dir(repository_root());
    if !base_path.is_empty() {
        command.args(["--base-path", base_path]);
    }
    command.output().expect("run the actual Akibiyori CLI")
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("parse {path:?}: {error}"))
}

fn count_html(root: &Path) -> usize {
    fn visit(path: &Path, count: &mut usize) {
        for entry in fs::read_dir(path).unwrap_or_else(|error| panic!("read {path:?}: {error}")) {
            let entry = entry.expect("read directory entry");
            let entry_path = entry.path();
            let file_type = entry.file_type().expect("read file type");
            assert!(
                !file_type.is_symlink(),
                "native site contains a symlink: {entry_path:?}"
            );
            if file_type.is_dir() {
                visit(&entry_path, count);
            } else if entry_path
                .extension()
                .is_some_and(|extension| extension == "html")
            {
                *count += 1;
            }
        }
    }

    let mut count = 0;
    visit(root, &mut count);
    count
}

fn collect_files(root: &Path) -> BTreeMap<PathBuf, PathBuf> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, PathBuf>) {
        for entry in fs::read_dir(path).unwrap_or_else(|error| panic!("read {path:?}: {error}")) {
            let entry = entry.expect("read directory entry");
            let entry_path = entry.path();
            let file_type = entry.file_type().expect("read file type");
            assert!(
                !file_type.is_symlink(),
                "native site contains a symlink: {entry_path:?}"
            );
            if file_type.is_dir() {
                visit(root, &entry_path, files);
            } else {
                let relative = entry_path
                    .strip_prefix(root)
                    .expect("file remains within assembled site")
                    .to_path_buf();
                files.insert(relative, entry_path);
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn snapshot_directory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    collect_files(root)
        .into_iter()
        .map(|(relative, path)| (relative, fs::read(path).expect("read snapshot file")))
        .collect()
}

fn html_attributes(document: &str, wanted: &str) -> Vec<String> {
    let marker_double = format!("{wanted}=\"");
    let marker_single = format!("{wanted}='");
    let mut attributes = Vec::new();
    let mut cursor = 0;
    while cursor < document.len() {
        let Some(open) = document[cursor..].find('<').map(|offset| cursor + offset) else {
            break;
        };
        let bytes = document.as_bytes();
        let mut end = open + 1;
        let mut quote = None;
        while end < bytes.len() {
            let byte = bytes[end];
            if let Some(current_quote) = quote {
                if byte == current_quote {
                    quote = None;
                }
            } else if byte == b'"' || byte == b'\'' {
                quote = Some(byte);
            } else if byte == b'>' {
                break;
            }
            end += 1;
        }
        if end == bytes.len() {
            break;
        }
        let tag = &document[open + 1..end];
        let mut search = 0;
        while search < tag.len() {
            let double = tag[search..]
                .find(&marker_double)
                .map(|offset| (search + offset, &marker_double));
            let single = tag[search..]
                .find(&marker_single)
                .map(|offset| (search + offset, &marker_single));
            let next = match (double, single) {
                (Some((left, marker)), Some((right, _))) if left <= right => (left, marker),
                (Some(_), Some((right, marker))) => (right, marker),
                (Some((left, marker)), None) => (left, marker),
                (None, Some((right, marker))) => (right, marker),
                (None, None) => break,
            };
            let attribute_start = next.0;
            let absolute_start = open + 1 + attribute_start;
            let boundary = document[..absolute_start].chars().next_back();
            if !boundary.is_some_and(|character| character.is_ascii_whitespace()) {
                search = attribute_start + next.1.len();
                continue;
            }
            let value_start = attribute_start + next.1.len();
            let delimiter = if next.1.as_str() == marker_double.as_str() {
                '"'
            } else {
                '\''
            };
            if let Some(value_end) = tag[value_start..].find(delimiter) {
                attributes.push(tag[value_start..value_start + value_end].to_owned());
                search = value_start + value_end + 1;
            } else {
                break;
            }
        }
        cursor = end + 1;
    }
    attributes
}

fn source_url_for_html(relative_file: &Path) -> Url {
    let physical = relative_file
        .to_str()
        .expect("native relative path is UTF-8")
        .replace('\\', "/");
    let route = physical.strip_suffix("index.html").map_or_else(
        || format!("/{physical}"),
        |directory| format!("/{directory}"),
    );
    let path = if route == "/" || route.ends_with('/') {
        route
    } else {
        format!("{route}/")
    };
    Url::parse(&format!("https://{LOCAL_HOST}{path}"))
        .expect("assembled source route is a valid URL")
}

fn target_file(site: &Path, url: &Url, base_path: &str) -> PathBuf {
    let url_path = url.path();
    if !base_path.is_empty() {
        assert!(
            url_path == base_path || url_path.starts_with(&format!("{base_path}/")),
            "internal URL {:?} escapes base path {base_path:?}",
            url
        );
    }
    let relative = url_path.trim_start_matches('/');
    let mut path = site.to_path_buf();
    if relative.is_empty() || url_path.ends_with('/') {
        if !relative.is_empty() {
            path.push(relative);
        }
        path.push("index.html");
    } else {
        path.push(relative);
        if !path.is_file() {
            path.push("index.html");
        }
    }
    path
}

fn assert_mount_copied(source: &Path, assembled: &Path) {
    let source_files = collect_files(source);
    for (relative, source_path) in source_files {
        let assembled_path = assembled.join(&relative);
        assert!(
            assembled_path.is_file(),
            "assembled site omits {relative:?}"
        );
        assert_eq!(
            fs::read(&assembled_path).expect("read assembled copy"),
            fs::read(&source_path).expect("read native source file"),
            "native file changed during assembly: {relative:?}"
        );
    }
}

fn assert_internal_targets(site: &Path, base_path: &str) -> usize {
    let files = collect_files(site);
    let mut checked = 0;
    for (relative, path) in files.iter().filter(|(relative, _)| {
        relative
            .extension()
            .is_some_and(|extension| extension == "html")
    }) {
        let document =
            fs::read_to_string(path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
        let source_url = source_url_for_html(relative);
        for value in html_attributes(&document, "href")
            .into_iter()
            .chain(html_attributes(&document, "src"))
            .chain(html_attributes(&document, "poster"))
        {
            let target_url = source_url
                .join(&value)
                .unwrap_or_else(|error| panic!("resolve {value:?} from {source_url}: {error}"));
            if target_url.host_str() != Some(LOCAL_HOST) {
                continue;
            }
            let target = target_file(site, &target_url, base_path);
            assert!(
                target.is_file(),
                "internal target {value:?} from {relative:?} has no file at {target:?}"
            );
            if let Some(fragment) = target_url
                .fragment()
                .filter(|fragment| !fragment.is_empty())
            {
                let target_document = fs::read_to_string(&target).unwrap_or_else(|error| {
                    panic!("read fragment destination {target:?}: {error}")
                });
                assert!(
                    target_document.contains(&format!("id=\"{fragment}\""))
                        || target_document.contains(&format!("id='{fragment}'"))
                        || target_document.contains(&format!("name=\"{fragment}\"")),
                    "internal fragment {fragment:?} from {relative:?} has no matching id in {target:?}"
                );
            }
            checked += 1;
        }
    }
    checked
}

struct SiteExpectation {
    name: &'static str,
    mount: &'static str,
    authored: usize,
    operations: usize,
    models: usize,
    html: usize,
    indexed: u64,
}

fn assert_site_population(output: &Path, site: &Path, base_path: &str, expected: SiteExpectation) {
    let build_root = output.join("builds").join(expected.name);
    let manifest = read_json(&build_root.join("source/_data/akibiyori-manifest.json"));
    assert_eq!(
        manifest["authored_routes"].as_array().unwrap().len(),
        expected.authored
    );
    assert_eq!(
        manifest["operation_routes"].as_object().unwrap().len(),
        expected.operations
    );
    assert_eq!(
        manifest["model_routes"].as_object().unwrap().len(),
        expected.models
    );
    let served_root = site
        .join(base_path.trim_start_matches('/'))
        .join(expected.mount.trim_start_matches('/'));
    let native_root = build_root
        .join("site")
        .join(base_path.trim_start_matches('/'))
        .join(expected.mount.trim_start_matches('/'));
    assert!(
        served_root.join("index.html").is_file(),
        "missing {} overview at {served_root:?}",
        expected.name
    );
    assert!(
        native_root.join("index.html").is_file(),
        "native {} mount is missing at {native_root:?}",
        expected.name
    );
    assert_eq!(
        count_html(&native_root),
        expected.html,
        "unexpected {} HTML route count",
        expected.name
    );
    assert_mount_copied(&native_root, &served_root);
    let search = read_json(&served_root.join("pagefind/pagefind-entry.json"));
    assert_eq!(
        search["languages"]["en"]["page_count"].as_u64(),
        Some(expected.indexed)
    );
}

fn assert_kubernetes_home(site: &Path, mount: &str) {
    let root = site.join(mount.trim_start_matches('/'));
    let overview = fs::read_to_string(root.join("index.html")).expect("read Kubernetes overview");
    assert!(overview.contains("href=\"guide/\""));
    assert!(overview.contains("href=\"reference/\""));
    let guide = fs::read_to_string(root.join("guide/index.html")).expect("read Kubernetes guide");
    let reference = fs::read_to_string(root.join("reference/index.html"))
        .expect("read Kubernetes reference overview");
    assert!(guide.contains("<h1>Reading this specimen</h1>"));
    assert!(reference.contains("<h1>Kubernetes API Extensions Reference</h1>"));
}

fn assert_petstore_home(site: &Path, mount: &str) {
    let root = site.join(mount.trim_start_matches('/'));
    let overview = fs::read_to_string(root.join("index.html")).expect("read Petstore overview");
    assert!(overview.contains("<h1>Swagger Petstore</h1>"));
    assert!(overview.contains("href=\"guide/\">reading guide</a>"));
    assert!(overview.contains("href=\"reference/\">API reference</a>"));

    let guide = fs::read_to_string(root.join("guide/index.html")).expect("read Petstore guide");
    assert!(guide.contains("<h1>Read the Petstore contract</h1>"));
    assert!(guide.contains("href=\"../reference/getPetById/\"><code>getPetById</code></a>"));
    assert!(guide.contains("href=\"../reference/models/petstore/Pet/\"><code>Pet</code></a>"));
    assert!(guide.contains("href=\"../reference/models/petstore/Order/\"><code>Order</code></a>"));

    let reference = fs::read_to_string(root.join("reference/index.html"))
        .expect("read Petstore reference overview");
    assert!(reference.contains("<h1>Petstore API reference</h1>"));
    assert!(reference.contains("href=\"../guide/\">reading guide</a>"));
    let operation = fs::read_to_string(root.join("reference/getPetById/index.html"))
        .expect("read Petstore operation page");
    assert!(operation.contains("data-choice-label=\"api_key\""));
    assert!(operation.contains("data-choice-label=\"petstore_auth\""));
    let pet = fs::read_to_string(root.join("reference/models/petstore/Pet/index.html"))
        .expect("read Pet model page");
    assert!(pet.contains("data-property-name=\"photoUrls\""));
    assert!(pet.contains("id=\"field-232f636f6d706f6e656e74732f736368656d61732f5065742f70726f706572746965732f70686f746f55726c73\""));
}

fn assert_openapi_catalogue(site: &Path, build_root: &Path, base_path: &str) {
    let mount = base_path.trim_start_matches('/');
    let hub = PathBuf::from(mount).join("demos/index.html");
    let catalogue = PathBuf::from(mount).join("demos/openapi/index.html");
    assert_owned_document_link(
        site,
        base_path,
        &hub,
        "openapi/",
        "OpenAPI demos",
        "OpenAPI demos",
    );

    for (href, label, heading) in [
        (
            "../observatory/",
            "Observatory tutorial catalog",
            "Observatory Catalog",
        ),
        (
            "../planets/reference/",
            "Scalar Galaxy API reference",
            "Scalar Galaxy API reference",
        ),
        (
            "../kubernetes/",
            "Kubernetes API Extensions",
            "Kubernetes API Extensions",
        ),
        (
            "../petstore/reference/",
            "Swagger Petstore API",
            "Petstore API reference",
        ),
    ] {
        assert_owned_document_link(site, base_path, &catalogue, href, label, heading);
    }
    for (href, label, heading) in [
        (
            "../observatory/docs/reading-an-observation/",
            "observation walkthrough",
            "Reading an observation",
        ),
        (
            "../kubernetes/guide/",
            "reading guide",
            "Reading this specimen",
        ),
        (
            "../petstore/guide/",
            "reading guide",
            "Read the Petstore contract",
        ),
        ("../", "Public demos", "Public demos"),
        ("../../docs/", "Akibiyori Docs", "Akibiyori Docs"),
        (
            "../../docs/public-fixtures/",
            "fixture provenance and bounds",
            "Public fixture provenance and bounds",
        ),
        ("../../docs/credits/", "Credits", "Third-party notices"),
    ] {
        assert_owned_document_link(site, base_path, &catalogue, href, label, heading);
    }

    let catalogue_body = fs::read_to_string(site.join(&catalogue)).expect("read OpenAPI catalogue");
    let upstream_label =
        "href=\"../planets/\">upstream Fumadocs Quick Start prose and visual specimen</a>";
    assert!(
        catalogue_body.contains(upstream_label),
        "the upstream prose is a separately labelled catalogue destination"
    );
    let catalogue_url = source_url_for_html(&catalogue);
    let api_url = catalogue_url
        .join("../planets/reference/")
        .expect("resolve primary Scalar Galaxy API route");
    let upstream_url = catalogue_url
        .join("../planets/")
        .expect("resolve separate upstream Fumadocs route");
    assert_ne!(api_url, upstream_url);
    let upstream = fs::read_to_string(target_file(site, &upstream_url, base_path))
        .expect("read upstream Fumadocs prose page");
    assert!(
        upstream
            .contains("<h1 class=\"document-title text-[1.75em] font-semibold\">Quick Start</h1>")
    );

    let config = read_json(&repository_root().join("akibiyori-docs.json"));
    let pages = config["pages"]
        .as_array()
        .expect("project pages are an array");
    let demos_position = pages
        .iter()
        .position(|page| page["route"] == "/demos/")
        .expect("Demos page is registered");
    assert_eq!(
        pages.get(demos_position + 1).unwrap()["route"],
        "/demos/openapi/",
        "the OpenAPI category follows the Demos landing in authored page order"
    );

    let prefix = if base_path.is_empty() {
        String::new()
    } else {
        base_path.to_owned()
    };
    let navigation = read_json(&build_root.join("source/_data/akibiyori_navigation.json"));
    let navigation = navigation
        .as_array()
        .expect("native navigation is an array");
    assert_eq!(
        navigation
            .iter()
            .filter(|link| link["label"] == "Demos")
            .count(),
        1
    );
    assert!(
        navigation
            .iter()
            .any(|link| { link["label"] == "Demos" && link["href"] == format!("{prefix}/demos/") })
    );

    let sidebar = read_json(&build_root.join("source/_data/akibiyori_sidebar.json"));
    let groups = sidebar.as_array().expect("native sidebar is an array");
    let group_position = |title: &str| {
        groups
            .iter()
            .position(|group| group["title"] == title)
            .unwrap_or_else(|| panic!("missing sidebar group {title:?}"))
    };
    let guides_position = group_position("Guides");
    let demos_position = group_position("Demos");
    let observatory_position = group_position("Observatory tutorial API");
    assert_eq!(demos_position, guides_position + 1);
    assert_eq!(observatory_position, demos_position + 1);
    assert_eq!(
        groups
            .iter()
            .filter(|group| group["title"] == "Demos")
            .count(),
        1
    );
    let demo_links = groups[demos_position]["links"]
        .as_array()
        .expect("Demos sidebar links are an array");
    assert_eq!(demo_links.len(), 1);
    assert_eq!(demo_links[0]["label"], "OpenAPI demos");
    assert_eq!(demo_links[0]["href"], format!("{prefix}/demos/openapi/"));
}

fn assert_petstore_entry(site: &Path, base_path: &str) {
    let catalogue =
        PathBuf::from(base_path.trim_start_matches('/')).join("demos/openapi/index.html");
    assert_owned_document_link(
        site,
        base_path,
        &catalogue,
        "../petstore/reference/",
        "Swagger Petstore API",
        "Petstore API reference",
    );
}

fn assert_owned_document_link(
    site: &Path,
    base_path: &str,
    source_relative: &Path,
    href: &str,
    link_label: &str,
    target_heading: &str,
) {
    let source = fs::read_to_string(site.join(source_relative)).expect("read authored source page");
    let anchor = format!("href=\"{href}\">{link_label}</a>");
    assert!(
        source.contains(&anchor),
        "missing semantic anchor {anchor:?} from {source_relative:?}"
    );
    let source_url = source_url_for_html(source_relative);
    let target_url = source_url.join(href).expect("resolve authored page link");
    let target = target_file(site, &target_url, base_path);
    let target_body = fs::read_to_string(&target).expect("read semantic destination page");
    assert!(
        target_body.contains(&format!("<h1>{target_heading}</h1>")),
        "{href:?} from {source_relative:?} must identify the {target_heading:?} page, got {target:?}"
    );
}

struct GeneratedIndexExpectation<'a> {
    title: &'a str,
    body_marker: &'a str,
    manifest_routes: &'a Value,
}

fn assert_owned_generated_index_link(
    site: &Path,
    base_path: &str,
    source_relative: &Path,
    href: &str,
    link_label: &str,
    expectation: GeneratedIndexExpectation<'_>,
) {
    let source = fs::read_to_string(site.join(source_relative)).expect("read authored source page");
    let anchor = format!("href=\"{href}\">{link_label}</a>");
    assert!(
        source.contains(&anchor),
        "missing semantic anchor {anchor:?} from {source_relative:?}"
    );
    let source_url = source_url_for_html(source_relative);
    let target_url = source_url.join(href).expect("resolve authored page link");
    let expected_owner_prefix = format!("{base_path}/demos/observatory/reference/");
    assert!(
        target_url.path().starts_with(&expected_owner_prefix),
        "generated index URL {:?} escapes the Observatory owner",
        target_url
    );
    let expected_route = format!("{expected_owner_prefix}{href}");
    assert_eq!(target_url.path(), expected_route);

    let target = target_file(site, &target_url, base_path);
    let target_body = fs::read_to_string(&target).expect("read generated index destination");
    assert!(
        target_body.contains(&format!("<title>{}</title>", expectation.title)),
        "{href:?} from {source_relative:?} must identify the {:?} index, got {target:?}",
        expectation.title
    );
    assert!(
        target_body.contains(expectation.body_marker),
        "{href:?} from {source_relative:?} must contain its native index marker {:?}",
        expectation.body_marker
    );

    let article_start = target_body
        .find("<article id=\"nd-page\"")
        .expect("generated index has its native article");
    let article_end = target_body[article_start..]
        .find("</article>")
        .map(|offset| article_start + offset)
        .expect("generated index article closes");
    let article = &target_body[article_start..article_end];
    let index_content_start = article
        .find(expectation.body_marker)
        .expect("generated index marker is inside its article");
    let index_content = &article[index_content_start..];
    let routes = expectation
        .manifest_routes
        .as_object()
        .expect("owner route manifest is an object");
    assert_eq!(
        index_content.matches("<li><a href=\"").count(),
        routes.len(),
        "generated index body list is complete without extra entries"
    );
    for (identity, route) in routes {
        let route = route
            .as_str()
            .unwrap_or_else(|| panic!("manifest route for {identity:?} is a string"));
        let expected_anchor = format!("<li><a href=\"{route}\">");
        assert_eq!(
            index_content.matches(&expected_anchor).count(),
            1,
            "generated index contains exactly one entry for {identity:?} at {route:?}"
        );
    }
}

fn assert_theme_doc_destinations(site: &Path, base_path: &str) {
    let mount = base_path.trim_start_matches('/');
    let theme_page = PathBuf::from(mount).join("docs/themes/index.html");
    let design_page = PathBuf::from(mount).join("docs/design-system/index.html");
    assert_owned_document_link(
        site,
        base_path,
        &theme_page,
        "../design-system/",
        "canonical reader design system",
        "Reader design system",
    );
    assert_owned_document_link(
        site,
        base_path,
        &design_page,
        "../themes/",
        "theme authoring",
        "Theme authoring",
    );
    assert_owned_document_link(
        site,
        base_path,
        &theme_page,
        "../credits/",
        "Credits",
        "Third-party notices",
    );
    assert_owned_document_link(
        site,
        base_path,
        &design_page,
        "../credits/",
        "Credits",
        "Third-party notices",
    );
}

fn assert_design_guidelines(site: &Path, build_root: &Path, base_path: &str) {
    let mount = base_path.trim_start_matches('/');
    let docs = PathBuf::from(mount).join("docs/index.html");
    assert_owned_document_link(
        site,
        base_path,
        &docs,
        "design/",
        "Design guidelines",
        "Design guidelines",
    );

    for (route, heading) in [
        ("", "Design guidelines"),
        ("colour/", "Colour"),
        ("typography/", "Typography"),
        ("layout/", "Layout"),
        ("specimens/", "Shared specimens"),
        ("house/", "House colour themes"),
    ] {
        let relative = PathBuf::from(mount)
            .join("docs/design")
            .join(route)
            .join("index.html");
        let body = fs::read_to_string(site.join(relative)).expect("read generated design page");
        assert!(
            body.contains(&format!("<h1>{heading}</h1>")),
            "generated design route must have heading {heading:?}"
        );
    }

    let sidebar = read_json(&build_root.join("source/_data/akibiyori_sidebar.json"));
    let groups = sidebar.as_array().expect("native sidebar is an array");
    let group = groups
        .iter()
        .find(|group| group["title"] == "Design guidelines")
        .expect("distinct Design guidelines sidebar group");
    let links = group["links"]
        .as_array()
        .expect("design links are an array");
    let prefix = if base_path.is_empty() {
        String::new()
    } else {
        base_path.to_owned()
    };
    for (label, path) in [
        ("Overview", "design/"),
        ("Colour", "design/colour/"),
        ("Typography", "design/typography/"),
        ("Layout", "design/layout/"),
        ("Shared specimens", "design/specimens/"),
        ("House colour themes", "design/house/"),
    ] {
        let href = format!("{prefix}/docs/{path}");
        assert!(
            links
                .iter()
                .any(|link| link["label"] == label && link["href"] == href),
            "sidebar destination {label:?} resolves to {href:?}"
        );
    }

    let house = PathBuf::from(mount).join("docs/design/house/index.html");
    let house_body = fs::read_to_string(site.join(&house)).expect("read House colour guide");
    for phrase in [
        "Maple &amp; Mist Light",
        "Ember &amp; Ink Dark",
        "House colour themes",
    ] {
        assert!(
            house_body.contains(phrase),
            "House page includes {phrase:?}"
        );
    }
    assert_owned_document_link(
        site,
        base_path,
        &house,
        "../../design-system/",
        "reader design system",
        "Reader design system",
    );
    assert_owned_document_link(
        site,
        base_path,
        &house,
        "../../themes/",
        "theme authoring guide",
        "Theme authoring",
    );
    assert_owned_document_link(
        site,
        base_path,
        &house,
        "../colour/",
        "colour guide",
        "Colour",
    );
    assert_owned_document_link(
        site,
        base_path,
        &house,
        "../specimens/",
        "shared specimens",
        "Shared specimens",
    );
    assert!(house_body.contains("config-theme-house.json"));

    let colour = PathBuf::from(mount).join("docs/design/colour/index.html");
    let colour_body =
        fs::read_to_string(site.join(&colour)).expect("read generated colour guidance");
    for marker in [
        "background:var(--aki-color-surface-page)",
        "color:var(--aki-color-text)",
        "background:var(--aki-color-surface-panel)",
        "color:var(--aki-color-text-panel)",
        "background:var(--aki-color-surface-page);color:var(--aki-color-action)",
    ] {
        assert!(
            colour_body.contains(marker),
            "generated role swatches preserve {marker:?}"
        );
    }

    let specimens = PathBuf::from(mount).join("docs/design/specimens/index.html");
    let body = fs::read_to_string(site.join(&specimens)).expect("read generated specimens");
    for marker in [
        "<details>",
        "class=\"operation-endpoint\"",
        "class=\"method text-green-400\"",
        "class=\"schema-card\"",
        "class=\"property-name\"",
        "class=\"property-requirement required\"",
        "language-json",
        "language-sh",
        "--config",
        "examples/observatory/config.json",
        "--output",
        "target/observatory-design-example",
        "--base-path",
        "/observatory",
    ] {
        assert!(
            body.contains(marker),
            "generated shared specimens preserve {marker:?}"
        );
    }
    assert_owned_document_link(
        site,
        base_path,
        &specimens,
        "../../../reference/read-observation/",
        "read observation operation",
        "read-observation",
    );
    assert_owned_document_link(
        site,
        base_path,
        &specimens,
        "../../../reference/models/catalog/ObservationEnvelope/",
        "ObservationEnvelope model",
        "ObservationEnvelope",
    );
}

fn assert_docs_entry(site: &Path, build_root: &Path, base_path: &str) {
    let mount = base_path.trim_start_matches('/');
    let home = PathBuf::from(mount).join("index.html");
    let docs = PathBuf::from(mount).join("docs/index.html");
    assert_owned_document_link(
        site,
        base_path,
        &home,
        "docs/",
        "Akibiyori Docs",
        "Akibiyori Docs",
    );

    for (href, label, heading) in [
        ("install/", "Install", "Install"),
        ("quickstart/", "Quick start", "Quick start"),
        ("authoring/", "Author Markdown", "Author Markdown"),
        ("configuration/", "Configure a site", "Configure a site"),
        ("themes/", "Theme authoring", "Theme authoring"),
        ("reference-reader/", "Reference reader", "Reference reader"),
        ("maintenance/", "Build and maintain", "Build and maintain"),
        (
            "public-fixtures/",
            "Public fixture provenance and bounds",
            "Public fixture provenance and bounds",
        ),
        ("credits/", "Third-party notices", "Third-party notices"),
        ("design/", "Design guidelines", "Design guidelines"),
    ] {
        assert_owned_document_link(site, base_path, &docs, href, label, heading);
    }
    assert_owned_document_link(
        site,
        base_path,
        &docs,
        "../demos/",
        "demo hub",
        "Public demos",
    );

    let prefix = if base_path.is_empty() {
        String::new()
    } else {
        base_path.to_owned()
    };
    let navigation = read_json(&build_root.join("source/_data/akibiyori_navigation.json"));
    let navigation = navigation
        .as_array()
        .expect("native navigation is an array");
    assert_eq!(navigation.first().unwrap()["label"], "Docs");
    assert_eq!(
        navigation.first().unwrap()["href"],
        format!("{prefix}/docs/")
    );
    assert_eq!(
        navigation
            .iter()
            .filter(|link| link["label"] == "Docs")
            .count(),
        1,
        "the generated navigation has exactly one Docs entry"
    );

    let sidebar = read_json(&build_root.join("source/_data/akibiyori_sidebar.json"));
    let groups = sidebar.as_array().expect("native sidebar is an array");
    assert_eq!(groups.first().unwrap()["title"], "Start here");
    let start_links = groups.first().unwrap()["links"]
        .as_array()
        .expect("Start here links are an array");
    assert_eq!(start_links.first().unwrap()["label"], "Akibiyori Docs");
    assert_eq!(
        start_links.first().unwrap()["href"],
        format!("{prefix}/docs/")
    );
}

fn reject_wrong_docs_home_while_quickstart_exists(site: &Path, build_root: &Path, base_path: &str) {
    let relative = PathBuf::from(base_path.trim_start_matches('/')).join("index.html");
    let path = site.join(&relative);
    let original = fs::read(&path).expect("read original home bytes");
    let original_text = String::from_utf8(original.clone()).expect("home HTML is UTF-8");
    let correct = "href=\"docs/\">Akibiyori Docs</a>";
    let wrong = "href=\"docs/quickstart/\">Akibiyori Docs</a>";
    assert_eq!(original_text.matches(correct).count(), 1);
    let wrong_text = original_text.replacen(correct, wrong, 1);

    fs::write(&path, wrong_text).expect("write wrong-but-existing Docs destination");
    let quickstart_url = source_url_for_html(&relative)
        .join("docs/quickstart/")
        .expect("resolve existing Quick start route");
    assert!(target_file(site, &quickstart_url, base_path).is_file());
    assert!(assert_internal_targets(site, base_path) > 0);
    let semantic_result =
        std::panic::catch_unwind(|| assert_docs_entry(site, build_root, base_path));
    fs::write(&path, &original).expect("restore exact original home bytes");
    assert!(
        semantic_result.is_err(),
        "the Docs-specific oracle rejects an existing but unrelated route"
    );
    assert_eq!(fs::read(&path).expect("read restored home"), original);
}

fn reject_wrong_theme_guide_while_other_target_exists(site: &Path, base_path: &str) {
    let relative = PathBuf::from(base_path.trim_start_matches('/')).join("docs/themes/index.html");
    let path = site.join(&relative);
    let original = fs::read(&path).expect("read original theme guide bytes");
    let original_text = String::from_utf8(original.clone()).expect("theme guide HTML is UTF-8");
    let correct = "href=\"../design-system/\">canonical reader design system</a>";
    let wrong = "href=\"../themes/\">canonical reader design system</a>";
    assert_eq!(original_text.matches(correct).count(), 1);
    let wrong_text = original_text.replacen(correct, wrong, 1);

    fs::write(&path, wrong_text).expect("write wrong-but-existing role-map destination");
    let checked = assert_internal_targets(site, base_path);
    assert!(checked > 0, "the assembled site has checked local targets");
    let semantic_result =
        std::panic::catch_unwind(|| assert_theme_doc_destinations(site, base_path));
    fs::write(&path, &original).expect("restore exact original theme guide bytes");
    assert!(
        semantic_result.is_err(),
        "the theme-doc oracle rejects an existing but unrelated guide destination"
    );
    assert_eq!(
        fs::read(&path).expect("read restored theme guide"),
        original
    );
}

fn reject_wrong_petstore_primary_while_other_target_exists(site: &Path, base_path: &str) {
    let catalogue_relative =
        PathBuf::from(base_path.trim_start_matches('/')).join("demos/openapi/index.html");
    let catalogue_path = site.join(&catalogue_relative);
    let original = fs::read(&catalogue_path).expect("read original OpenAPI catalogue bytes");
    let original_text = String::from_utf8(original.clone()).expect("catalogue HTML is UTF-8");
    let correct = "href=\"../petstore/reference/\">Swagger Petstore API</a>";
    let wrong = "href=\"../../reference/\">Swagger Petstore API</a>";
    assert_eq!(original_text.matches(correct).count(), 1);
    let wrong_text = original_text.replacen(correct, wrong, 1);

    fs::write(&catalogue_path, wrong_text).expect("write wrong-but-existing Petstore destination");
    let target_count = assert_internal_targets(site, base_path);
    assert!(
        target_count > 0,
        "the assembled site has checked local targets"
    );
    let semantic_result = std::panic::catch_unwind(|| assert_petstore_entry(site, base_path));
    fs::write(&catalogue_path, &original).expect("restore exact original catalogue bytes");
    assert!(
        semantic_result.is_err(),
        "the Petstore-specific oracle rejects an existing but unrelated reference"
    );
    assert_eq!(
        fs::read(&catalogue_path).expect("read restored catalogue"),
        original
    );
}

fn showcase_docs_link_is_owned_by_docs(
    site: &Path,
    base_path: &str,
    source_relative: &Path,
    html: &str,
    target_headings: &mut BTreeMap<PathBuf, bool>,
) -> bool {
    let Some((_, anchor_tail)) = html.split_once("<a class=\"showcase-context-link\" href=\"")
    else {
        return false;
    };
    let Some((href, label_tail)) = anchor_tail.split_once("\">") else {
        return false;
    };
    if !label_tail.starts_with("Akibiyori Docs</a>") {
        return false;
    }

    let Ok(target_url) = source_url_for_html(source_relative).join(href) else {
        return false;
    };
    let target = target_file(site, &target_url, base_path);
    *target_headings.entry(target.clone()).or_insert_with(|| {
        fs::read_to_string(&target).is_ok_and(|body| body.contains("<h1>Akibiyori Docs</h1>"))
    })
}

fn assert_showcase_context(output: &Path, site: &Path, base_path: &str) {
    let prefix = base_path;
    let expected_sites = [
        (
            "observatory",
            "Observatory tutorial API",
            "/demos/observatory/",
        ),
        ("planets", "Scalar Galaxy API", "/demos/planets/reference/"),
        (
            "kubernetes",
            "Kubernetes API Extensions",
            "/demos/kubernetes/",
        ),
        (
            "petstore",
            "Swagger Petstore API",
            "/demos/petstore/reference/",
        ),
    ];
    for (owner, expected_current) in [
        ("docs", "Akibiyori Docs"),
        ("observatory", "OpenAPI demos · Observatory tutorial API"),
        ("planets", "OpenAPI demos · Scalar Galaxy API"),
        ("kubernetes", "OpenAPI demos · Kubernetes API Extensions"),
        ("petstore", "OpenAPI demos · Swagger Petstore API"),
    ] {
        let data = read_json(&output.join(format!(
            "builds/{owner}/source/_data/akibiyori_showcase.json"
        )));
        assert_eq!(data["current_label"], expected_current);
        assert_eq!(data["sites"].as_array().unwrap().len(), 4);
        assert!(
            !data
                .to_string()
                .contains(&repository_root().display().to_string())
        );
        for (id, label, route) in expected_sites {
            let entry = data["sites"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["id"] == id)
                .unwrap_or_else(|| panic!("missing showcase site {id}"));
            assert_eq!(entry["label"], label);
            assert_eq!(entry["href"], format!("{prefix}{route}"));
            assert_eq!(entry["current"], id == owner);
        }
        assert_eq!(data["docs"]["href"], format!("{prefix}/docs/"));
        assert_eq!(
            data["categories"][0]["href"],
            format!("{prefix}/demos/openapi/")
        );

        let theme =
            read_json(&output.join(format!("builds/{owner}/source/_data/akibiyori_theme.json")));
        assert_eq!(theme["presentation"], "demo");
        assert_eq!(theme["demo_default"], "maple-mist-light");
        assert_eq!(theme["variant_id"], "maple-mist-light");
        assert_eq!(theme["mode"], "light");
        assert_eq!(theme["default_mode"], "system");
        assert_eq!(theme["profiles"].as_array().unwrap().len(), 8);
        assert_eq!(
            theme["theme_key"],
            format!(
                "akibiyori:theme:v1:showcase:akibiyori-showcase:{}",
                if prefix.is_empty() { "/" } else { prefix }
            )
        );
        let profile_ids = theme["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|profile| profile["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            profile_ids,
            [
                "fumadocs-dark",
                "fumadocs-light",
                "neutral-example-dark",
                "neutral-example-light",
                "neutral-night",
                "neutral-day",
                "maple-mist-light",
                "ember-ink-dark",
            ]
        );
        let font_css = fs::read_to_string(
            output.join(format!("builds/{owner}/source/css/akibiyori-fonts.css")),
        )
        .expect("read shared theme font CSS");
        assert!(font_css.contains("geist-latin.woff2"));
        assert!(font_css.contains("jetbrains-mono-latin.woff2"));
        for (font, bytes) in [
            (
                "geist-latin.woff2",
                include_bytes!("../theme/vendor/fonts/geist-latin.woff2").as_slice(),
            ),
            (
                "jetbrains-mono-latin.woff2",
                include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2").as_slice(),
            ),
        ] {
            assert_eq!(
                fs::read(output.join(format!("builds/{owner}/source/fonts/{font}")))
                    .expect("read native build font bytes"),
                bytes,
                "shared font bytes for {owner}/{font}"
            );
            let mount = if owner == "docs" {
                PathBuf::new()
            } else {
                PathBuf::from("demos").join(owner)
            };
            assert_eq!(
                fs::read(
                    site.join(prefix.trim_start_matches('/'))
                        .join(&mount)
                        .join("fonts")
                        .join(font)
                )
                .expect("read assembled shared font bytes"),
                bytes,
                "assembled font bytes for {owner}/{font}"
            );
        }
    }

    let files = collect_files(site);
    let mut html_count = 0;
    let mut docs_target_headings = BTreeMap::new();
    for relative in files
        .keys()
        .filter(|path| path.extension().is_some_and(|ext| ext == "html"))
    {
        html_count += 1;
        let html = fs::read_to_string(site.join(relative))
            .unwrap_or_else(|error| panic!("read assembled HTML {relative:?}: {error}"));
        let start = html
            .find("<nav class=\"showcase-context\"")
            .unwrap_or_else(|| panic!("missing showcase nav in {relative:?}"));
        let rest = &html[start..];
        let end = rest.find("</nav>").expect("showcase nav closes");
        let nav = &rest[..end + "</nav>".len()];
        let opening_end = nav.find('>').expect("showcase nav opening tag closes");
        let opening = &nav[..=opening_end];
        assert!(
            opening.contains("aria-label=\"Showcase context\""),
            "{relative:?}"
        );
        assert!(opening.contains("data-pagefind-ignore"), "{relative:?}");
        assert!(nav.contains("Switch demo"), "{relative:?}");
        assert!(nav.matches("class=\"showcase-context-link\"").count() >= 6);
        let normalized = relative
            .strip_prefix(prefix.trim_start_matches('/'))
            .unwrap_or(relative);
        let owner = normalized
            .components()
            .next()
            .and_then(|component| component.as_os_str().to_str());
        let current_label = match owner {
            Some("demos") => match normalized
                .components()
                .nth(1)
                .and_then(|component| component.as_os_str().to_str())
            {
                Some("observatory") => "Current: OpenAPI demos · Observatory tutorial API",
                Some("planets") => "Current: OpenAPI demos · Scalar Galaxy API",
                Some("kubernetes") => "Current: OpenAPI demos · Kubernetes API Extensions",
                Some("petstore") => "Current: OpenAPI demos · Swagger Petstore API",
                _ => "Current: Akibiyori Docs",
            },
            _ => "Current: Akibiyori Docs",
        };
        assert!(
            nav.contains(current_label),
            "wrong current owner in {relative:?}"
        );
        assert!(
            showcase_docs_link_is_owned_by_docs(
                site,
                base_path,
                relative,
                nav,
                &mut docs_target_headings,
            ),
            "Docs navigation from {relative:?} resolves to the Docs H1 owner"
        );
    }
    assert_eq!(
        html_count, 179,
        "showcase appears across every native HTML route"
    );
}

fn reject_wrong_showcase_docs_target_while_other_target_exists(site: &Path, base_path: &str) {
    let docs_home = site
        .join(base_path.trim_start_matches('/'))
        .join("docs/index.html");
    let original = fs::read(&docs_home).expect("read Docs homepage");
    let original_text = String::from_utf8(original.clone()).expect("Docs homepage UTF-8");
    let relative = PathBuf::from(base_path.trim_start_matches('/')).join("docs/index.html");
    let mut target_headings = BTreeMap::new();
    assert!(showcase_docs_link_is_owned_by_docs(
        site,
        base_path,
        &relative,
        &original_text,
        &mut target_headings,
    ));
    let wrong = format!(
        "<a class=\"showcase-context-link\" href=\"{base_path}/demos/planets/reference/\">Akibiyori Docs</a>"
    );
    let correct =
        format!("<a class=\"showcase-context-link\" href=\"{base_path}/docs/\">Akibiyori Docs</a>");
    let wrong_text = original_text.replacen(&correct, &wrong, 1);
    assert_ne!(wrong_text, original_text);
    fs::write(&docs_home, &wrong_text).expect("write wrong-but-existing showcase destination");
    let alternate = site
        .join(base_path.trim_start_matches('/'))
        .join("demos/planets/reference/index.html");
    assert!(
        alternate.is_file(),
        "wrong destination exists under another owner"
    );
    assert!(assert_internal_targets(site, base_path) > 0);
    assert!(
        !showcase_docs_link_is_owned_by_docs(
            site,
            base_path,
            &relative,
            &wrong_text,
            &mut target_headings,
        ),
        "the semantic owner predicate rejects the wrong-but-existing target"
    );
    fs::write(&docs_home, &original).expect("restore exact Docs homepage bytes");
    assert_eq!(fs::read(&docs_home).unwrap(), original);
}

fn invalid_central_theme_preflight_preserves_owned_output(output: &Path, temp: &Path) {
    let mut manifest = read_json(&repository_root().join("akibiyori-showcase.json"));
    manifest["theme"]["catalog"] = serde_json::json!([
        {"builtin":"fumadocs"},
        {"builtin":"neutral-example"},
        {"builtin":"house"}
    ]);
    manifest["theme"]["demo_default"] = serde_json::json!("unknown-central-profile");
    for site in manifest["sites"].as_array_mut().expect("manifest sites") {
        let config = site["config"].as_str().expect("config path");
        site["config"] = serde_json::json!(repository_root().join(config));
    }
    let alternate = temp.join("invalid-central-theme.json");
    fs::write(
        &alternate,
        serde_json::to_vec_pretty(&manifest).expect("serialize invalid selection manifest"),
    )
    .expect("write alternate manifest");

    let marker = output.with_file_name(format!(
        "{}.owned",
        output.file_name().unwrap().to_string_lossy()
    ));
    let output_before = snapshot_directory(output);
    let marker_before = fs::read(&marker).expect("read output ownership marker");
    let failure = run_script_with_manifest(
        "root",
        output,
        &native_program("PAGEFIND", "pagefind"),
        Some(&alternate),
    );
    assert!(
        !failure.status.success(),
        "invalid shared theme must fail preflight"
    );
    let stderr = String::from_utf8_lossy(&failure.stderr);
    assert!(stderr.contains("unknown theme.demo_default"), "{stderr}");
    assert_eq!(snapshot_directory(output), output_before);
    assert_eq!(
        fs::read(&marker).expect("read preserved ownership marker"),
        marker_before
    );
}

fn accordion_sidebar_markup_matches(
    sidebar: &Value,
    html: &str,
    site: &Path,
    base_path: &str,
    owner: &str,
    contract_title: &str,
    expected_groups: &[(&str, usize)],
) -> bool {
    let Some((_, nav_tail)) = html.split_once("<nav class=\"reference-generated-nav\"") else {
        return false;
    };
    let Some(nav_end) = nav_tail.find("</nav>") else {
        return false;
    };
    let nav = &nav_tail[..nav_end];
    if sidebar["groups"] != serde_json::json!([]) {
        return false;
    }
    let Some(contracts) = sidebar["contracts"].as_array() else {
        return false;
    };
    if contracts.len() != 1 || contracts[0]["title"] != contract_title {
        return false;
    }
    if nav
        .matches("<section class=\"reference-contract-section\">")
        .count()
        != 1
        || !nav.contains(&format!(
            "<h2 class=\"reference-contract-heading\">{contract_title}</h2>"
        ))
    {
        return false;
    }
    let Some(groups) = contracts[0]["groups"].as_array() else {
        return false;
    };
    if groups.len() != expected_groups.len() {
        return false;
    }
    if nav
        .matches("<details class=\"reference-tag-group\" data-reference-tag-group>")
        .count()
        != groups.len()
        || nav
            .matches("<summary class=\"reference-tag-summary\">")
            .count()
            != groups.len()
    {
        return false;
    }
    let owner_prefix = format!("{base_path}/demos/{owner}/reference/");
    for (group, (expected_title, expected_count)) in groups.iter().zip(expected_groups) {
        if group["title"] != *expected_title {
            return false;
        }
        let expected_group_start = format!(
            "<details class=\"reference-tag-group\" data-reference-tag-group><summary class=\"reference-tag-summary\">{expected_title}</summary>"
        );
        let Some(group_start) = nav.find(&expected_group_start) else {
            return false;
        };
        let Some(group_end) = nav[group_start..].find("</details>") else {
            return false;
        };
        let group_markup = &nav[group_start..group_start + group_end];
        let Some(links) = group["links"].as_array() else {
            return false;
        };
        if links.len() != *expected_count {
            return false;
        }
        for link in links {
            let Some(href) = link["href"].as_str() else {
                return false;
            };
            if !href.starts_with(&owner_prefix)
                || !group_markup.contains(&format!("href=\"{href}\""))
                || !site
                    .join(href.trim_start_matches('/'))
                    .join("index.html")
                    .is_file()
            {
                return false;
            }
            let Some(label) = link["label"].as_str() else {
                return false;
            };
            let Some(accessible_label) = link["accessible_label"].as_str() else {
                return false;
            };
            let Some(anchor_start) =
                group_markup.find(&format!("<a class=\"reference-nav-link\" href=\"{href}\""))
            else {
                return false;
            };
            let Some(anchor_end) = group_markup[anchor_start..].find("</a>") else {
                return false;
            };
            let anchor = &group_markup[anchor_start..anchor_start + anchor_end];
            if !anchor.contains(&format!("aria-label=\"{accessible_label}\""))
                || !anchor.contains(&format!(
                    "<span class=\"reference-nav-label\">{label}</span>"
                ))
            {
                return false;
            }
            match link["badge"]["kind"].as_str() {
                Some("method") => {
                    let Some(method) = link["badge"]["value"].as_str() else {
                        return false;
                    };
                    let Some(badge_start) = anchor.find("class=\"reference-nav-badge") else {
                        return false;
                    };
                    let Some(badge_end) = anchor[badge_start..].find("</span>") else {
                        return false;
                    };
                    if !anchor[badge_start..badge_start + badge_end].contains(&format!(">{method}"))
                    {
                        return false;
                    }
                }
                Some("webhook") => {
                    let Some(badge_start) = anchor.find("class=\"reference-nav-badge") else {
                        return false;
                    };
                    let Some(badge_end) = anchor[badge_start..].find("</span>") else {
                        return false;
                    };
                    if !anchor[badge_start..badge_start + badge_end].contains(">Webhook") {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
    true
}

fn assert_project_accordion_sidebars(output: &Path, site: &Path, base_path: &str) {
    let owners = [
        (
            "planets",
            "Scalar Galaxy API",
            [
                ("Planets", 7),
                ("Celestial Bodies", 1),
                ("Authentication", 3),
            ]
            .as_slice(),
        ),
        (
            "kubernetes",
            "Kubernetes API Extensions",
            [("apiextensions_v1", 13)].as_slice(),
        ),
        (
            "petstore",
            "Swagger Petstore",
            [("pet", 8), ("store", 4), ("user", 7)].as_slice(),
        ),
        (
            "observatory",
            "Catalog contract",
            [("Catalog operations", 2)].as_slice(),
        ),
    ];
    for (owner, contract_title, expected_groups) in owners {
        let sidebar = read_json(&output.join(format!(
            "builds/{owner}/source/_data/akibiyori_reference_sidebar.json"
        )));
        let reference = site
            .join(base_path.trim_start_matches('/'))
            .join(format!("demos/{owner}/reference/index.html"));
        let html = fs::read_to_string(&reference).unwrap_or_else(|error| {
            panic!("read {owner} generated reference {reference:?}: {error}")
        });
        assert!(
            accordion_sidebar_markup_matches(
                &sidebar,
                &html,
                site,
                base_path,
                owner,
                contract_title,
                expected_groups,
            ),
            "{owner} accordion data, owning links, badges, and native markup agree"
        );
    }

    let observatory_reference = PathBuf::from(base_path.trim_start_matches('/'))
        .join("demos/observatory/reference/index.html");
    let observatory_manifest =
        read_json(&output.join("builds/observatory/source/_data/akibiyori-manifest.json"));
    assert_owned_generated_index_link(
        site,
        base_path,
        &observatory_reference,
        "operations/",
        "All operations",
        GeneratedIndexExpectation {
            title: "Operations · Observatory Catalog",
            body_marker: "Operations in the selected contracts.",
            manifest_routes: &observatory_manifest["operation_routes"],
        },
    );
    assert_owned_generated_index_link(
        site,
        base_path,
        &observatory_reference,
        "models/",
        "All models",
        GeneratedIndexExpectation {
            title: "Models · Observatory Catalog",
            body_marker: "Named models are grouped by contract identifier.",
            manifest_routes: &observatory_manifest["model_routes"],
        },
    );
    assert_owned_document_link(
        site,
        base_path,
        &observatory_reference,
        "../docs/reading-an-observation/",
        "Reading an observation",
        "Reading an observation",
    );

    for (owner, contract_title, expected_groups) in [
        (
            "planets",
            "Scalar Galaxy API",
            [
                ("Planets", 7),
                ("Celestial Bodies", 1),
                ("Authentication", 3),
            ]
            .as_slice(),
        ),
        (
            "kubernetes",
            "Kubernetes API Extensions",
            [("apiextensions_v1", 13)].as_slice(),
        ),
        (
            "petstore",
            "Swagger Petstore",
            [("pet", 8), ("store", 4), ("user", 7)].as_slice(),
        ),
        (
            "observatory",
            "Catalog contract",
            [("Catalog operations", 2)].as_slice(),
        ),
    ] {
        let sidebar = read_json(&output.join(format!(
            "builds/{owner}/source/_data/akibiyori_reference_sidebar.json"
        )));
        let manifest = read_json(&output.join(format!(
            "builds/{owner}/source/_data/akibiyori-manifest.json"
        )));
        for routes in ["operation_routes", "model_routes"] {
            let route = manifest[routes]
                .as_object()
                .and_then(|routes| routes.values().next())
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{owner} has a real {routes} entry"));
            let page = site.join(route.trim_start_matches('/')).join("index.html");
            let html = fs::read_to_string(&page).unwrap_or_else(|error| {
                panic!("read {owner} accordion consumer {page:?}: {error}")
            });
            assert!(accordion_sidebar_markup_matches(
                &sidebar,
                &html,
                site,
                base_path,
                owner,
                contract_title,
                expected_groups,
            ));
        }
    }

    // This real temporary assembled output is never served. The same owner
    // predicate must reject a link to another existing demo route, then accept
    // the byte-restored native page.
    let sidebar =
        read_json(&output.join("builds/planets/source/_data/akibiyori_reference_sidebar.json"));
    let reference = site
        .join(base_path.trim_start_matches('/'))
        .join("demos/planets/reference/index.html");
    let original = fs::read(&reference).expect("snapshot temporary Planets reference");
    let text = String::from_utf8(original.clone()).expect("native HTML is UTF-8");
    let expected_href = format!("{base_path}/demos/planets/reference/getAllData/");
    let wrong_existing_href =
        format!("{base_path}/demos/kubernetes/reference/getApiextensionsV1APIResources/");
    assert!(
        site.join(wrong_existing_href.trim_start_matches('/'))
            .join("index.html")
            .is_file()
    );
    let mutated = text.replacen(
        &format!("href=\"{expected_href}\""),
        &format!("href=\"{wrong_existing_href}\""),
        1,
    );
    assert_ne!(mutated, text, "the expected Planets target is present");
    fs::write(&reference, mutated.as_bytes()).expect("mutate only temporary native HTML");
    let rejected = !accordion_sidebar_markup_matches(
        &sidebar,
        &mutated,
        site,
        base_path,
        "planets",
        "Scalar Galaxy API",
        &[
            ("Planets", 7),
            ("Celestial Bodies", 1),
            ("Authentication", 3),
        ],
    );
    fs::write(&reference, &original).expect("restore exact temporary native HTML");
    assert_eq!(fs::read(&reference).expect("read restored HTML"), original);
    assert!(
        rejected,
        "the same owner predicate rejects a wrong existing target"
    );
    let restored = String::from_utf8(original).expect("restored HTML is UTF-8");
    assert!(accordion_sidebar_markup_matches(
        &sidebar,
        &restored,
        site,
        base_path,
        "planets",
        "Scalar Galaxy API",
        &[
            ("Planets", 7),
            ("Celestial Bodies", 1),
            ("Authentication", 3)
        ],
    ));
}

fn assert_project_hub(output: &Path, mode: &str) {
    let site = output.join("site");
    let base_path = if mode == "root" { "" } else { BASE_PATH };
    let expectations = [
        SiteExpectation {
            name: "docs",
            mount: "",
            authored: 25,
            operations: 2,
            models: 9,
            html: 39,
            indexed: 38,
        },
        SiteExpectation {
            name: "observatory",
            mount: "/demos/observatory",
            authored: 3,
            operations: 2,
            models: 9,
            html: 17,
            indexed: 16,
        },
        SiteExpectation {
            name: "planets",
            mount: "/demos/planets",
            authored: 3,
            operations: 11,
            models: 15,
            html: 32,
            indexed: 31,
        },
        SiteExpectation {
            name: "kubernetes",
            mount: "/demos/kubernetes",
            authored: 3,
            operations: 13,
            models: 41,
            html: 60,
            indexed: 59,
        },
        SiteExpectation {
            name: "petstore",
            mount: "/demos/petstore",
            authored: 3,
            operations: 19,
            models: 6,
            html: 31,
            indexed: 30,
        },
    ];
    for expected in expectations {
        assert_site_population(output, &site, base_path, expected);
    }
    assert_docs_entry(&site, &output.join("builds/docs"), base_path);
    assert_showcase_context(output, &site, base_path);
    assert_project_accordion_sidebars(output, &site, base_path);
    reject_wrong_showcase_docs_target_while_other_target_exists(&site, base_path);
    assert_eq!(
        count_html(&site),
        179,
        "assembled hub keeps every separate route"
    );
    let checked = assert_internal_targets(&site, base_path);
    assert!(
        checked > 0,
        "native hub contains internal page and asset targets"
    );
    let kube_mount = if base_path.is_empty() {
        "demos/kubernetes"
    } else {
        "akibiyori-docs/demos/kubernetes"
    };
    assert_kubernetes_home(&site, kube_mount);
    assert_openapi_catalogue(&site, &output.join("builds/docs"), base_path);
    assert_petstore_entry(&site, base_path);
    let petstore_mount = if base_path.is_empty() {
        "demos/petstore"
    } else {
        "akibiyori-docs/demos/petstore"
    };
    assert_petstore_home(&site, petstore_mount);
    assert_theme_doc_destinations(&site, base_path);
    assert_design_guidelines(&site, &output.join("builds/docs"), base_path);

    let walkthrough = site
        .join(base_path.trim_start_matches('/'))
        .join("demos/observatory/docs/reading-an-observation/index.html");
    let body = fs::read_to_string(&walkthrough).expect("read Observatory walkthrough");
    assert!(body.contains("href=\"../../reference/models/catalog/Calibration/\">Calibration</a>"));
    assert!(body.contains("<code>calibration_epoch</code>"));
    let calibration = site
        .join(base_path.trim_start_matches('/'))
        .join("demos/observatory/reference/models/catalog/Calibration/index.html");
    let calibration_body = fs::read_to_string(&calibration).expect("read Calibration model");
    let field_at = calibration_body
        .find(">calibration_epoch")
        .expect("Calibration exposes its calibration_epoch heading");
    let heading_start = calibration_body[..field_at]
        .rfind("<h3 id=\"")
        .expect("the field is a generated heading with a fragment target");
    let id_start = heading_start + "<h3 id=\"".len();
    let id_end = calibration_body[id_start..]
        .find('\"')
        .map(|offset| id_start + offset)
        .expect("the field heading id is closed");
    assert!(!calibration_body[id_start..id_end].is_empty());
}

#[test]
fn native_root_and_prefix_hubs_keep_routes_assets_and_search_separate() {
    let temp = TempDir::new().expect("temporary hub outputs");
    let root = temp.path().join("root");
    successful_build(run_script(
        "root",
        &root,
        &native_program("PAGEFIND", "pagefind"),
    ));
    assert_project_hub(&root, "root");
    invalid_central_theme_preflight_preserves_owned_output(&root, temp.path());
    reject_wrong_docs_home_while_quickstart_exists(
        &root.join("site"),
        &root.join("builds/docs"),
        "",
    );
    reject_wrong_petstore_primary_while_other_target_exists(&root.join("site"), "");
    reject_wrong_theme_guide_while_other_target_exists(&root.join("site"), "");
    successful_build(run_script(
        "root",
        &root,
        &native_program("PAGEFIND", "pagefind"),
    ));
    assert_project_hub(&root, "root");

    let prefix = temp.path().join("prefix");
    successful_build(run_script(
        "prefix",
        &prefix,
        &native_program("PAGEFIND", "pagefind"),
    ));
    assert_project_hub(&prefix, "prefix");
    reject_wrong_docs_home_while_quickstart_exists(
        &prefix.join("site"),
        &prefix.join("builds/docs"),
        BASE_PATH,
    );
    reject_wrong_petstore_primary_while_other_target_exists(&prefix.join("site"), BASE_PATH);
    reject_wrong_theme_guide_while_other_target_exists(&prefix.join("site"), BASE_PATH);

    let config = repository_root().join("examples/kubernetes/config.json");
    let standalone_root = temp.path().join("kubernetes-root");
    successful_build(run_cli_build(&config, &standalone_root, ""));
    assert_kubernetes_home(&standalone_root.join("site"), "");
    assert!(
        !standalone_root
            .join("source/_data/akibiyori_showcase.json")
            .exists()
    );
    assert_internal_targets(&standalone_root.join("site"), "");

    let standalone_prefix = temp.path().join("kubernetes-prefix");
    successful_build(run_cli_build(
        &config,
        &standalone_prefix,
        "/kubernetes-reference",
    ));
    assert_kubernetes_home(&standalone_prefix.join("site"), "/kubernetes-reference");
    assert!(
        !standalone_prefix
            .join("source/_data/akibiyori_showcase.json")
            .exists()
    );
    assert_internal_targets(&standalone_prefix.join("site"), "/kubernetes-reference");

    let petstore_config = repository_root().join("examples/petstore/config.json");
    let petstore_root = temp.path().join("petstore-root");
    successful_build(run_cli_build(&petstore_config, &petstore_root, ""));
    assert_petstore_home(&petstore_root.join("site"), "");
    assert_internal_targets(&petstore_root.join("site"), "");

    let petstore_prefix = temp.path().join("petstore-prefix");
    successful_build(run_cli_build(
        &petstore_config,
        &petstore_prefix,
        "/petstore-reference",
    ));
    assert_petstore_home(&petstore_prefix.join("site"), "/petstore-reference");
    assert_internal_targets(&petstore_prefix.join("site"), "/petstore-reference");
}

#[test]
fn demo_site_refuses_unowned_symlink_extra_marker_and_source_outputs() {
    let temp = TempDir::new().expect("temporary ownership controls");
    let pagefind = native_program("PAGEFIND", "pagefind");

    let unowned = temp.path().join("unowned");
    fs::create_dir(&unowned).expect("create unowned directory");
    fs::write(unowned.join("keep.txt"), "sentinel").expect("write sentinel");
    let result = run_script("root", &unowned, &pagefind);
    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(unowned.join("keep.txt")).unwrap(),
        "sentinel"
    );

    let extra_marker = temp.path().join("extra-marker");
    fs::create_dir(&extra_marker).expect("create marked directory");
    fs::write(extra_marker.join("keep.txt"), "sentinel").expect("write marked sentinel");
    fs::write(
        temp.path().join("extra-marker.owned"),
        "akibiyori:demo-site:root:v1\nforeign\n",
    )
    .expect("write non-exact ownership marker");
    let result = run_script("root", &extra_marker, &pagefind);
    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(extra_marker.join("keep.txt")).unwrap(),
        "sentinel"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let target = temp.path().join("symlink-target");
        fs::create_dir(&target).expect("create symlink target");
        fs::write(target.join("keep.txt"), "sentinel").expect("write symlink sentinel");
        let output = temp.path().join("symlink-output");
        symlink(&target, &output).expect("create output symlink");
        let result = run_script("root", &output, &pagefind);
        assert!(!result.status.success());
        assert!(
            fs::symlink_metadata(&output)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(target.join("keep.txt")).unwrap(),
            "sentinel"
        );

        let source_docs = repository_root().join("docs");
        let alias = temp.path().join("docs-alias");
        symlink(&source_docs, &alias).expect("create source docs alias");
        let physical_parent = source_docs.join("output-parent-alias-check");
        let aliased_output = alias.join("output-parent-alias-check/site");
        assert!(
            !physical_parent.exists(),
            "aliased source parent starts absent"
        );
        let result = run_script("root", &aliased_output, &pagefind);
        assert!(
            !result.status.success(),
            "physical source alias is rejected"
        );
        assert!(
            !physical_parent.exists(),
            "rejected physical source path does not create parent directories"
        );
        assert!(
            !physical_parent.join("site.owned").exists(),
            "rejected physical source path does not create a success marker"
        );
    }

    let source_descendant =
        repository_root().join("docs/demo-output-safety-probe/new-parent/output");
    let parent = source_descendant.parent().unwrap();
    assert!(!parent.exists(), "safety test parent starts absent");
    let result = run_script("root", &source_descendant, &pagefind);
    assert!(!result.status.success());
    assert!(
        !parent.exists(),
        "rejected source path does not create parent directories"
    );
}

#[test]
fn failed_native_build_leaves_partial_output_without_success_marker() {
    let temp = TempDir::new().expect("temporary failed output");
    let output = temp.path().join("failed");
    let missing_pagefind = temp.path().join("pagefind-does-not-exist");
    let result = run_script("root", &output, missing_pagefind.to_str().unwrap());
    assert!(!result.status.success());
    assert!(
        output.is_dir(),
        "script partial output remains available for inspection"
    );
    assert!(!temp.path().join("failed.owned").exists());
}
