use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn checked(output: Output, what: &str) -> Output {
    assert!(
        output.status.success(),
        "{what} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn cargo() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

fn native_program(env_name: &str, fallback: &str) -> OsString {
    env::var_os(env_name).unwrap_or_else(|| OsString::from(fallback))
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn inventory(root: &Path) -> (usize, u64) {
    let mut files = 0usize;
    let mut bytes = 0u64;
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let (nested_files, nested_bytes) = inventory(&path);
            files += nested_files;
            bytes += nested_bytes;
        } else {
            files += 1;
            bytes += fs::metadata(path).unwrap().len();
        }
    }
    (files, bytes)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap_or_else(|error| {
                panic!("copy {} to {}: {error}", from.display(), to.display())
            });
        }
    }
}

fn command_path(bin: &str, path: &OsStr) -> PathBuf {
    let mut command = Command::new("sh");
    command
        .args(["-c", "command -v akibiyori"])
        .env("PATH", path);
    let output = checked(
        command.output().expect("run command -v"),
        "command -v akibiyori",
    );
    let resolved = String::from_utf8(output.stdout).expect("command -v output UTF-8");
    let resolved = PathBuf::from(resolved.trim());
    assert_eq!(resolved, PathBuf::from(bin));
    resolved
}

fn resolved_native(program: &OsStr, path: &OsStr) -> PathBuf {
    let candidate = Path::new(program);
    if candidate.is_absolute() || candidate.components().count() > 1 {
        return candidate.canonicalize().unwrap_or_else(|error| {
            panic!("resolve native tool {}: {error}", candidate.display())
        });
    }
    let mut command = Command::new("sh");
    command
        .args(["-c", "command -v \"$1\"", "akibiyori-tool"])
        .arg(program)
        .env("PATH", path);
    let output = checked(
        command
            .output()
            .expect("resolve native executable through PATH"),
        "command -v native tool",
    );
    let resolved = String::from_utf8(output.stdout).expect("native command path UTF-8");
    PathBuf::from(resolved.trim())
        .canonicalize()
        .unwrap_or_else(|error| panic!("canonicalize native tool path: {error}"))
}

fn native_version(program: &Path) -> String {
    let output = checked(
        Command::new(program)
            .arg("--version")
            .output()
            .unwrap_or_else(|error| panic!("run {} --version: {error}", program.display())),
        "native tool --version",
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn native_arg(value: &OsStr) -> &OsStr {
    value
}

#[test]
fn packaged_archive_installs_and_serves_an_external_consumer() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let clean_status = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .current_dir(&manifest_dir)
        .output()
        .expect("inspect candidate worktree before packaging");
    let clean_status = checked(clean_status, "git status before cargo package");
    assert!(
        clean_status.stdout.is_empty(),
        "cargo package evidence must come from a clean committed candidate; status was:\n{}",
        String::from_utf8_lossy(&clean_status.stdout)
    );
    let cargo_target = manifest_dir.join("target/package-check/cargo");
    fs::create_dir_all(&cargo_target).expect("package-check target directory");

    let status = Command::new(cargo())
        .args(["package", "--manifest-path"])
        .arg(manifest_dir.join("Cargo.toml"))
        .args(["--locked", "--offline"])
        .env("CARGO_TARGET_DIR", &cargo_target)
        .current_dir(&manifest_dir)
        .output()
        .expect("spawn actual cargo package");
    checked(status, "cargo package --locked --offline");

    let archive = cargo_target.join("package/akibiyori-0.1.0.crate");
    assert!(
        archive.is_file(),
        "Cargo package archive exists: {}",
        archive.display()
    );
    let archive_size = fs::metadata(&archive).expect("archive metadata").len();

    let run_root = tempfile::Builder::new()
        .prefix("akibiyori-package-check-")
        .tempdir()
        .expect("create unique system temporary package run")
        .keep();
    let candidate_archive = run_root.join("candidate/akibiyori-0.1.0.crate");
    fs::create_dir_all(candidate_archive.parent().unwrap()).expect("frozen candidate directory");
    fs::copy(&archive, &candidate_archive).expect("preserve package archive for review");
    let extract_root = run_root.join("extract");
    fs::create_dir_all(&extract_root).expect("extract root");
    let tar_list = Command::new("tar")
        .args(["-tzf"])
        .arg(&archive)
        .output()
        .expect("run system tar to list package");
    let tar_list = checked(tar_list, "tar -tzf package");
    let members = String::from_utf8(tar_list.stdout).expect("tar listing UTF-8");
    for excluded in ["AGENTS.md", ".github/", "assets/images/drafts/"] {
        assert!(
            !members.lines().any(|member| member.contains(excluded)),
            "archive excludes {excluded}"
        );
    }
    assert!(
        members.lines().any(|member| member.ends_with("LICENSE")),
        "first-party license packaged"
    );
    assert!(
        members
            .lines()
            .any(|member| member.ends_with("THIRD_PARTY_NOTICES.md"))
    );
    assert!(
        members
            .lines()
            .any(|member| member.contains("examples/starter-site/config.json"))
    );
    for required in [
        "theme/_layouts/default.liquid",
        "theme/css/akibiyori.css",
        "theme/vendor/fumadocs-reference.css",
        "theme/js/akibiyori-mode.js",
        "theme/profiles/botanist.json",
        "examples/observatory/themes/neutral-example/theme.json",
    ] {
        assert!(
            members.lines().any(|member| member.ends_with(required)),
            "archive contains {required}"
        );
    }

    checked(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&archive)
            .arg("-C")
            .arg(&extract_root)
            .output()
            .expect("extract actual crate with system tar"),
        "tar -xzf package",
    );
    let package_source = extract_root.join("akibiyori-0.1.0");
    assert!(package_source.join("LICENSE").is_file());
    let package_inventory = inventory(&package_source);
    let license = fs::read_to_string(package_source.join("LICENSE")).expect("read package license");
    assert!(license.contains("Permission is hereby granted"));
    assert!(license.contains("THE SOFTWARE IS PROVIDED"));
    let notices = fs::read_to_string(package_source.join("THIRD_PARTY_NOTICES.md"))
        .expect("read third-party notices");
    for retained_notice in ["Fumadocs", "Fonts", "Lucide", "Pacific", "Swagger Petstore"] {
        assert!(
            notices.contains(retained_notice),
            "notices include {retained_notice}"
        );
    }

    let install_root = run_root.join("cargo-install-root");
    let install_target = cargo_target.join("install");
    checked(
        Command::new(cargo())
            .args(["install", "--path"])
            .arg(&package_source)
            .args(["--locked", "--offline", "--profile", "release", "--root"])
            .arg(&install_root)
            .env("CARGO_TARGET_DIR", &install_target)
            .current_dir(&run_root)
            .output()
            .expect("spawn actual cargo install from extracted archive"),
        "cargo install from extracted archive",
    );
    let installed_bin = install_root.join("bin/akibiyori");
    assert!(installed_bin.is_file(), "installed CLI exists");
    let bin_path = install_root.join("bin");
    let mut installed_paths = vec![bin_path.clone()];
    installed_paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let installed_path = env::join_paths(installed_paths).expect("installed PATH");
    let command_resolved = command_path(
        installed_bin.to_str().expect("installed binary path UTF-8"),
        &installed_path,
    );
    let version = Command::new("akibiyori")
        .arg("--version")
        .env("PATH", &installed_path)
        .current_dir(&run_root)
        .output()
        .expect("run installed CLI version");
    let version = checked(version, "installed akibiyori --version");
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "akibiyori 0.1.0"
    );

    let native_path = env::var_os("PATH").unwrap_or_default();
    let mut executable_paths = vec![bin_path.clone()];
    executable_paths.extend(env::split_paths(&native_path));
    let cli_path = env::join_paths(&executable_paths).expect("external CLI PATH");
    let cobalt = native_program("COBALT", "cobalt");
    let pagefind = native_program("PAGEFIND", "pagefind");
    let cobalt_resolved = resolved_native(&cobalt, &cli_path);
    let pagefind_resolved = resolved_native(&pagefind, &cli_path);
    let cobalt_version = native_version(&cobalt_resolved);
    let pagefind_version = native_version(&pagefind_resolved);
    let external = run_root.join("consumer");
    let site_source = external.join("input");
    fs::create_dir_all(&site_source).expect("consumer directory");
    let checkout_real = manifest_dir
        .canonicalize()
        .expect("canonical source checkout");
    let package_source_real = package_source
        .canonicalize()
        .expect("canonical extracted package source");
    let external_real = external
        .canonicalize()
        .expect("canonical external consumer");
    assert!(!external_real.starts_with(&checkout_real));
    assert!(!external_real.starts_with(&package_source_real));
    copy_tree(&package_source.join("examples/starter-site"), &site_source);
    let mut build_inventories = Vec::new();
    for (name, base_path) in [("root", ""), ("prefix", "/consumer-preview")] {
        let output = external.join("target").join(format!("{name}-build"));
        let mut command = Command::new("akibiyori");
        command
            .args(["build", "--config"])
            .arg(site_source.join("config.json"))
            .args(["--output"])
            .arg(&output);
        if !base_path.is_empty() {
            command.args(["--base-path", base_path]);
        }
        command
            .arg("--cobalt")
            .arg(native_arg(&cobalt))
            .arg("--pagefind")
            .arg(native_arg(&pagefind))
            .env("PATH", &cli_path)
            .env("COBALT", &cobalt)
            .env("PAGEFIND", &pagefind)
            .current_dir(&external);
        let build_result = checked(
            command
                .output()
                .expect("run installed CLI for external site"),
            "installed CLI external build",
        );
        assert!(
            String::from_utf8_lossy(&build_result.stdout)
                .contains("built 3 authored pages, 1 operation pages, 1 model pages"),
            "installed CLI reports the expected external consumer counts"
        );
        let served = if base_path.is_empty() {
            output.join("site")
        } else {
            output.join("site/consumer-preview")
        };
        assert!(served.join("index.html").is_file());
        assert!(served.join("guide/reading-a-reading/index.html").is_file());
        assert!(served.join("reference/index.html").is_file());
        assert!(served.join("reference/readReading/index.html").is_file());
        assert!(
            served
                .join("reference/models/weather/Reading/index.html")
                .is_file()
        );
        assert!(served.join("pagefind/pagefind-entry.json").is_file());
        let homepage = fs::read_to_string(served.join("index.html")).expect("read built homepage");
        assert!(homepage.contains("Weather Station"));
        assert!(homepage.contains("data-aki-theme=\"botanist\""));
        assert!(homepage.contains("data-aki-variant=\"botanist-night\""));
        assert!(homepage.contains("<h2>Documentation"));
        assert!(homepage.contains("<h2>API Reference"));
        assert!(homepage.contains("id=\"aki-mode-picker\""));
        assert!(homepage.contains("<option value=\"system\">System"));
        assert!(homepage.contains("<option value=\"light\">Light"));
        assert!(homepage.contains("<option value=\"dark\">Dark"));
        let prefix = if base_path.is_empty() { "" } else { base_path };
        for asset_url in [
            "css/akibiyori-fonts.css",
            "css/fumadocs-reference.css",
            "css/akibiyori-tokens.css",
            "css/akibiyori.css",
            "css/akibiyori-theme.css",
            "themes/botanist/styles.css",
            "js/akibiyori-mode.js",
            "js/akibiyori-reading.js",
            "js/akibiyori-search.js",
        ] {
            assert!(
                homepage.contains(&format!("href=\"{prefix}/{asset_url}\""))
                    || homepage.contains(&format!("src=\"{prefix}/{asset_url}\"")),
                "page links local asset at {prefix}/{asset_url}"
            );
            assert!(
                served.join(asset_url).is_file(),
                "served asset exists: {asset_url}"
            );
        }
        for font in [
            "fonts/geist-latin.woff2",
            "fonts/jetbrains-mono-latin.woff2",
        ] {
            assert!(served.join(font).is_file(), "served font exists: {font}");
        }
        let operation = fs::read_to_string(served.join("reference/readReading/index.html"))
            .expect("read operation page");
        assert!(operation.contains("Read a station measurement"));
        assert!(operation.contains("readingId"));
        assert!(operation.contains("18.5"));
        assert!(operation.contains(&format!(
            "href=\"{prefix}/reference/models/weather/Reading/\""
        )));
        let model = fs::read_to_string(served.join("reference/models/weather/Reading/index.html"))
            .expect("read model page");
        assert!(model.contains("<h1>Reading</h1>"));
        assert!(model.contains("temperature in degrees Celsius"));
        let manifest: Value = serde_json::from_slice(
            &fs::read(output.join("source/_data/akibiyori-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["authored_routes"].as_array().unwrap().len(), 3);
        assert_eq!(manifest["operation_routes"].as_object().unwrap().len(), 1);
        assert_eq!(manifest["model_routes"].as_object().unwrap().len(), 1);
        let theme: Value = serde_json::from_slice(
            &fs::read(output.join("source/_data/akibiyori_theme.json")).unwrap(),
        )
        .unwrap();
        let profiles = theme["profiles"].as_array().unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(
            profiles
                .iter()
                .any(|profile| profile["id"] == "botanist-night")
        );
        assert!(
            profiles
                .iter()
                .any(|profile| profile["id"] == "botanist-day")
        );
        let (source_files, source_bytes) = inventory(&output.join("source"));
        let (site_files, site_bytes) = inventory(&served);
        assert!(source_files >= 20);
        assert!(site_files >= 20);
        println!(
            "consumer-{name}: source_files={source_files} source_bytes={source_bytes} site_files={site_files} site_bytes={site_bytes}"
        );
        build_inventories.push(json!({
            "kind": name,
            "source": output.join("source"),
            "source_file_count": source_files,
            "source_bytes": source_bytes,
            "site": served,
            "site_file_count": site_files,
            "site_bytes": site_bytes,
        }));
    }

    let demo_config_path = site_source.join("config-all-themes.json");
    let mut demo_config: Value =
        serde_json::from_slice(&fs::read(site_source.join("config.json")).unwrap()).unwrap();
    demo_config["theme"] = json!({
        "catalog": [
            {"builtin":"fumadocs"}, {"builtin":"neutral-example"}, {"builtin":"house"},
            {"builtin":"forest"}, {"builtin":"botanist"}, {"builtin":"pacific"}
        ],
        "deploy": {"dark":"botanist-night", "light":"botanist-day"},
        "presentation":"demo", "demo_default":"fumadocs-dark", "mode":"system"
    });
    fs::write(
        &demo_config_path,
        serde_json::to_vec_pretty(&demo_config).unwrap(),
    )
    .unwrap();
    let all_families_source = external.join("target/all-families-source");
    let mut generate = Command::new("akibiyori");
    generate
        .args(["generate", "--config"])
        .arg(&demo_config_path)
        .args(["--output"])
        .arg(&all_families_source)
        .args(["--theme-mode", "demo"])
        .env("PATH", &cli_path)
        .current_dir(&external);
    checked(
        generate
            .output()
            .expect("generate installed all-family source"),
        "installed CLI all-family generation",
    );
    let theme_data: Value = serde_json::from_slice(
        &fs::read(all_families_source.join("_data/akibiyori_theme.json")).unwrap(),
    )
    .unwrap();
    let profiles = theme_data["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), 18);
    let families: BTreeSet<&str> = profiles
        .iter()
        .map(|profile| profile["family"].as_str().unwrap())
        .collect();
    assert_eq!(
        families,
        BTreeSet::from([
            "fumadocs",
            "neutral-example",
            "house",
            "forest",
            "botanist",
            "pacific"
        ])
    );
    let theme_css = fs::read_to_string(all_families_source.join("css/akibiyori-theme.css"))
        .expect("read generated profile CSS");
    for profile in [
        "maple-mist-light",
        "maple-mist-dark",
        "golden-hour-light",
        "golden-hour-dark",
        "ember-ink-light",
        "ember-ink-dark",
    ] {
        assert!(
            theme_css.contains(profile),
            "generated CSS includes House profile {profile}"
        );
    }
    for asset in [
        "css/akibiyori-theme.css",
        "themes/neutral-example/styles.css",
        "themes/forest/styles.css",
        "themes/botanist/styles.css",
        "themes/pacific/styles.css",
        "themes/pacific/fonts/red-hat-display-normal-300-900.ttf",
        "themes/pacific/notices/red-hat-display-OFL.txt",
    ] {
        assert!(
            all_families_source.join(asset).is_file(),
            "all-family source includes {asset}"
        );
    }

    let library_consumer = run_root.join("library-consumer");
    fs::create_dir_all(library_consumer.join("src")).unwrap();
    let package_path = package_source
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    fs::write(
        library_consumer.join("Cargo.toml"),
        format!("[package]\nname = \"packaged-library-consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nakibiyori = {{ path = \"{package_path}\" }}\n"),
    )
    .unwrap();
    fs::write(
        library_consumer.join("src/main.rs"),
        "use std::path::Path;\nfn main() { let report = akibiyori::generate(Path::new(std::env::args().nth(1).as_deref().unwrap()), Path::new(std::env::args().nth(2).as_deref().unwrap()), \"\").unwrap(); assert_eq!(report.authored_pages, 3); assert_eq!(report.operation_pages, 1); assert_eq!(report.model_pages, 1); println!(\"{} {} {}\", report.authored_pages, report.operation_pages, report.model_pages); }\n",
    )
    .unwrap();
    checked(
        Command::new(cargo())
            .args(["generate-lockfile", "--offline"])
            .current_dir(&library_consumer)
            .env("CARGO_TARGET_DIR", &cargo_target.join("library-consumer"))
            .output()
            .expect("generate real external library lockfile"),
        "cargo generate-lockfile --offline for library consumer",
    );
    let library_output = library_consumer.join("generated-source");
    let mut library_run = Command::new(cargo());
    library_run
        .args(["run", "--locked", "--offline", "--"])
        .arg(site_source.join("config.json"))
        .arg(&library_output)
        .current_dir(&library_consumer)
        .env("CARGO_TARGET_DIR", cargo_target.join("library-consumer"));
    let library_result = checked(
        library_run
            .output()
            .expect("run external Rust library consumer"),
        "external Rust library consumer",
    );
    assert_eq!(
        String::from_utf8_lossy(&library_result.stdout).trim(),
        "3 1 1"
    );
    assert!(
        library_output
            .join("pages/2f7265666572656e63652f.md")
            .is_file()
    );
    assert!(library_consumer.join("Cargo.lock").is_file());

    let moved_source = extract_root.join("akibiyori-0.1.0-unavailable");
    fs::rename(&package_source, &moved_source)
        .expect("move only test-owned extracted package tree");
    let independence_output = external.join("target/after-extracted-source-moved");
    let mut independent_build = Command::new("akibiyori");
    independent_build
        .args(["build", "--config"])
        .arg(site_source.join("config.json"))
        .args(["--output"])
        .arg(&independence_output)
        .arg("--cobalt")
        .arg(&cobalt)
        .arg("--pagefind")
        .arg(&pagefind)
        .env("PATH", &cli_path)
        .current_dir(&external);
    checked(
        independent_build
            .output()
            .expect("run installed CLI after extracted source moved"),
        "installed CLI after extracted source moved",
    );
    assert!(independence_output.join("site/index.html").is_file());

    let mut receipt =
        fs::File::create(run_root.join("receipt.json")).expect("create package consumer receipt");
    writeln!(
        receipt,
        "{}",
        serde_json::to_string_pretty(&json!({
            "archive": archive,
            "frozen_archive": candidate_archive,
            "archive_sha256": sha256(&archive),
            "archive_compressed_bytes": archive_size,
            "archive_file_count": members.lines().count(),
            "archive_unpacked_file_count_and_bytes": package_inventory,
            "installed_binary": command_resolved,
            "installed_binary_sha256": sha256(&installed_bin),
            "version": String::from_utf8_lossy(&version.stdout).trim(),
            "cobalt": {
                "path": cobalt_resolved,
                "version": cobalt_version,
                "sha256": sha256(&cobalt_resolved),
            },
            "pagefind": {
                "path": pagefind_resolved,
                "version": pagefind_version,
                "sha256": sha256(&pagefind_resolved),
            },
            "run_root": run_root,
            "external_consumer": external,
            "external_consumer_outside_checkout": !external_real.starts_with(&checkout_real),
            "external_consumer_outside_extracted_package": !external_real.starts_with(&package_source_real),
            "build_inventories": build_inventories,
            "all_families_source": all_families_source,
            "library_consumer": library_consumer,
            "source_independence_output": independence_output,
        }))
        .expect("serialize receipt")
    )
    .unwrap();
    println!(
        "packaged-consumer-receipt: {}",
        run_root.join("receipt.json").display()
    );
}
