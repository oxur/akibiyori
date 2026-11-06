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
    Command::new(script_path())
        .args([mode, output.to_str().expect("UTF-8 output path")])
        .env("AKIBIYORI", env!("CARGO_BIN_EXE_akibiyori"))
        .env("COBALT", native_program("COBALT", "cobalt"))
        .env("PAGEFIND", pagefind)
        .current_dir(repository_root())
        .output()
        .expect("run the real demo-site script")
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

fn assert_planets_entry(site: &Path, base_path: &str) {
    let hub_relative = PathBuf::from(base_path.trim_start_matches('/')).join("demos/index.html");
    let hub = fs::read_to_string(site.join(&hub_relative)).expect("read demo hub");
    assert!(hub.contains("href=\"../demos/planets/reference/\">Scalar Galaxy API reference</a>"));
    assert!(hub.contains(
        "href=\"../demos/planets/\">upstream Fumadocs Quick Start prose and visual specimen</a>"
    ));

    let hub_url = source_url_for_html(&hub_relative);
    let api_url = hub_url
        .join("../demos/planets/reference/")
        .expect("resolve primary Planets API route");
    let api_path = target_file(site, &api_url, base_path);
    let api = fs::read_to_string(&api_path).expect("read primary Planets API page");
    assert!(api.contains("<h1>Scalar Galaxy API reference</h1>"));

    let prose_url = hub_url
        .join("../demos/planets/")
        .expect("resolve upstream prose route");
    assert_ne!(api_url, prose_url);
    let prose_path = target_file(site, &prose_url, base_path);
    let prose = fs::read_to_string(&prose_path).expect("read upstream Fumadocs prose page");
    assert!(
        prose.contains("<h1 class=\"document-title text-[1.75em] font-semibold\">Quick Start</h1>")
    );
}

fn assert_petstore_entry(site: &Path, base_path: &str) {
    let hub_relative = PathBuf::from(base_path.trim_start_matches('/')).join("demos/index.html");
    let hub = fs::read_to_string(site.join(&hub_relative)).expect("read demo hub");
    let primary = "href=\"../demos/petstore/reference/\">Swagger Petstore API</a>";
    assert!(
        hub.contains(primary),
        "Petstore's advertised API entry points to its own reference"
    );

    let hub_url = source_url_for_html(&hub_relative);
    let api_url = hub_url
        .join("../demos/petstore/reference/")
        .expect("resolve primary Petstore API route");
    let api_path = target_file(site, &api_url, base_path);
    let api = fs::read_to_string(&api_path).expect("read primary Petstore API page");
    assert!(api.contains("<h1>Petstore API reference</h1>"));
}

fn reject_wrong_petstore_primary_while_other_target_exists(site: &Path, base_path: &str) {
    let hub_relative = PathBuf::from(base_path.trim_start_matches('/')).join("demos/index.html");
    let hub_path = site.join(&hub_relative);
    let original = fs::read(&hub_path).expect("read original demo hub bytes");
    let original_text = String::from_utf8(original.clone()).expect("hub HTML is UTF-8");
    let correct = "href=\"../demos/petstore/reference/\">Swagger Petstore API</a>";
    let wrong = "href=\"../reference/\">Swagger Petstore API</a>";
    assert_eq!(original_text.matches(correct).count(), 1);
    let wrong_text = original_text.replacen(correct, wrong, 1);

    fs::write(&hub_path, wrong_text).expect("write wrong-but-existing Petstore destination");
    let target_count = assert_internal_targets(site, base_path);
    assert!(
        target_count > 0,
        "the assembled site has checked local targets"
    );
    let semantic_result = std::panic::catch_unwind(|| assert_petstore_entry(site, base_path));
    fs::write(&hub_path, &original).expect("restore exact original demo hub bytes");
    assert!(
        semantic_result.is_err(),
        "the Petstore-specific oracle rejects an existing but unrelated reference"
    );
    assert_eq!(
        fs::read(&hub_path).expect("read restored demo hub"),
        original
    );
}

fn assert_project_hub(output: &Path, mode: &str) {
    let site = output.join("site");
    let base_path = if mode == "root" { "" } else { BASE_PATH };
    let expectations = [
        SiteExpectation {
            name: "docs",
            mount: "",
            authored: 15,
            operations: 2,
            models: 9,
            html: 29,
            indexed: 28,
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
    assert_eq!(
        count_html(&site),
        169,
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
    assert_planets_entry(&site, base_path);
    assert_petstore_entry(&site, base_path);
    let petstore_mount = if base_path.is_empty() {
        "demos/petstore"
    } else {
        "akibiyori-docs/demos/petstore"
    };
    assert_petstore_home(&site, petstore_mount);

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
    reject_wrong_petstore_primary_while_other_target_exists(&root.join("site"), "");
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
    reject_wrong_petstore_primary_while_other_target_exists(&prefix.join("site"), BASE_PATH);

    let config = repository_root().join("examples/kubernetes/config.json");
    let standalone_root = temp.path().join("kubernetes-root");
    successful_build(run_cli_build(&config, &standalone_root, ""));
    assert_kubernetes_home(&standalone_root.join("site"), "");
    assert_internal_targets(&standalone_root.join("site"), "");

    let standalone_prefix = temp.path().join("kubernetes-prefix");
    successful_build(run_cli_build(
        &config,
        &standalone_prefix,
        "/kubernetes-reference",
    ));
    assert_kubernetes_home(&standalone_prefix.join("site"), "/kubernetes-reference");
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
