use std::fs;
use std::path::{Path, PathBuf};

use akibiyori::{BuildOptions, build, generate};
use serde_json::{Value, json};
use tempfile::TempDir;

const PACK_FILES: [&str; 8] = [
    "favicon.ico",
    "favicon-16x16.png",
    "favicon-32x32.png",
    "favicon-48x48.png",
    "apple-touch-icon.png",
    "icon-192x192.png",
    "icon-512x512.png",
    "icon-maskable-512x512.png",
];

fn input(root: &Path, route: &str) -> PathBuf {
    let pack = root.join("icons");
    fs::create_dir_all(&pack).expect("icon input directory");
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
    for name in PACK_FILES {
        fs::copy(original.join(name), pack.join(name)).expect("copy real generated icon");
    }
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/observatory/contracts/catalog.json"),
        root.join("contract.json"),
    )
    .expect("copy actual example contract");
    fs::write(root.join("page.md"), "# Icon pack example\n").expect("write authored page");
    let config = json!({
        "version": 1,
        "title": "Autumn & Observations",
        "contracts": [{"id":"catalog", "label":"Catalog", "path":"contract.json"}],
        "pages": [{"source":"page.md", "title":"Welcome", "route":route}],
        "icon_pack": {"directory":"icons"}
    });
    let mut config = config;
    if route != "/reference/" {
        config["pages"].as_array_mut().unwrap().push(json!({
            "source":"page.md", "title":"API reference", "route":"/reference/"
        }));
    }
    let path = root.join("config.json");
    fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();
    path
}

fn update_config(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    edit(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn icon_pack_real_native_builds_resolve_root_and_prefix_head_links() {
    let temp = TempDir::new().unwrap();
    let config = input(&temp.path().join("input"), "/");
    for (label, prefix) in [("root", ""), ("prefix", "/autumn")] {
        let mut options = BuildOptions::new(&config, temp.path().join(label));
        options.base_path = prefix.into();
        options.cobalt = std::env::var_os("COBALT").map_or_else(|| "cobalt".into(), PathBuf::from);
        options.pagefind =
            std::env::var_os("PAGEFIND").map_or_else(|| "pagefind".into(), PathBuf::from);
        let report = build(&options).expect("real Cobalt + Pagefind build");
        for page in [
            "index.html",
            "search/index.html",
            "reference/operations/index.html",
        ] {
            let html = fs::read_to_string(report.served_dir.join(page)).unwrap();
            let head = html.split_once("</head>").expect("rendered head").0;
            for (rel, href) in [
                ("icon", "favicon.ico"),
                ("icon", "icons/favicon-16x16.png"),
                ("icon", "icons/favicon-32x32.png"),
                ("icon", "icons/favicon-48x48.png"),
                ("apple-touch-icon", "icons/apple-touch-icon.png"),
                ("manifest", "site.webmanifest"),
            ] {
                let expected = format!("rel=\"{rel}\" href=\"{prefix}/{href}\"");
                assert!(head.contains(&expected), "{page} lacks {expected}");
                assert!(report.served_dir.join(href).is_file());
            }
            assert!(head.contains("name=\"theme-color\" content=\"#111111\""));
            assert!(head.contains("content=\"Autumn &amp; Observations\""));
            assert!(!head.contains("__AKIBIYORI_BASE_PATH__"));
            assert!(!head.contains("apple-mobile-web-app-capable"));
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(report.served_dir.join("site.webmanifest")).unwrap())
                .unwrap();
        assert_eq!(manifest["name"], "Autumn & Observations");
        assert_eq!(manifest["scope"], format!("{prefix}/"));
        assert_eq!(manifest["start_url"], format!("{prefix}/"));
        assert_eq!(manifest["id"], format!("{prefix}/"));
        assert_eq!(manifest["display"], "browser");
        assert_eq!(manifest["icons"].as_array().unwrap().len(), 3);
        for icon in manifest["icons"].as_array().unwrap() {
            let src = icon["src"].as_str().unwrap();
            assert!(
                !src.starts_with('/'),
                "manifest URLs are relative to its own root"
            );
            assert!(report.served_dir.join(src).is_file());
            let name = Path::new(src).file_name().unwrap();
            assert_eq!(
                fs::read(report.served_dir.join(src)).unwrap(),
                fs::read(config.parent().unwrap().join("icons").join(name)).unwrap()
            );
        }
        assert_eq!(manifest["icons"][2]["purpose"], "maskable");
    }
}

#[test]
fn icon_pack_is_opt_in_and_launches_existing_reference_without_home() {
    let temp = TempDir::new().unwrap();
    let config = input(&temp.path().join("input"), "/reference/");
    update_config(&config, |c| {
        c["icon_pack"]["theme_color"] = json!("#123456");
        c["icon_pack"]["background_color"] = json!("#abcdef");
    });
    let output = temp.path().join("enabled");
    generate(&config, &output, "/autumn").unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(output.join("site.webmanifest")).unwrap()).unwrap();
    assert_eq!(manifest["start_url"], "/autumn/reference/");
    assert_eq!(manifest["theme_color"], "#123456");
    assert_eq!(manifest["background_color"], "#abcdef");
    update_config(&config, |c| {
        c.as_object_mut().unwrap().remove("icon_pack");
    });
    let output = temp.path().join("disabled");
    generate(&config, &output, "").unwrap();
    assert!(!output.join("favicon.ico").exists());
    assert!(!output.join("site.webmanifest").exists());
    let site: Value =
        serde_json::from_slice(&fs::read(output.join("_data/akibiyori_site.json")).unwrap())
            .unwrap();
    assert!(site["icon_pack"].is_null());
}

#[test]
fn icon_pack_rejects_bad_inputs_and_public_collisions_before_writing() {
    let temp = TempDir::new().unwrap();
    for case in [
        "missing",
        "wrong-size",
        "ico",
        "escape",
        "absolute",
        "color",
        "empty",
        "favicon-route",
        "manifest-route",
        "png-route",
    ] {
        let root = temp.path().join(case);
        let config = input(&root, "/reference/");
        match case {
            "missing" => fs::remove_file(root.join("icons/icon-192x192.png")).unwrap(),
            "wrong-size" => {
                fs::copy(
                    root.join("icons/icon-192x192.png"),
                    root.join("icons/apple-touch-icon.png"),
                )
                .unwrap();
            }
            "ico" => fs::write(root.join("icons/favicon.ico"), b"not an ICO").unwrap(),
            "escape" => update_config(&config, |c| c["icon_pack"]["directory"] = json!("../icons")),
            "absolute" => update_config(&config, |c| {
                c["icon_pack"]["directory"] = json!(root.join("icons"))
            }),
            "empty" => update_config(&config, |c| c["icon_pack"]["directory"] = json!("")),
            "color" => update_config(&config, |c| {
                c["icon_pack"]["theme_color"] = json!("red\" bad")
            }),
            _ => update_config(&config, |c| {
                let route = match case {
                    "favicon-route" => "/favicon.ico/",
                    "manifest-route" => "/SITE.webmanifest/",
                    _ => "/icons/icon-192x192.png/",
                };
                c["pages"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"source":"page.md", "title":"Collision", "route":route}));
            }),
        }
        let output = temp.path().join(format!("out-{case}"));
        generate(&config, &output, "").expect_err(case);
        assert!(!output.exists(), "{case} must fail before writes");
    }
}

#[cfg(unix)]
#[test]
fn icon_pack_rejects_symlinked_inputs() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("input");
    let config = input(&root, "/reference/");
    let icon = root.join("icons/icon-192x192.png");
    let saved = root.join("saved.png");
    fs::rename(&icon, &saved).unwrap();
    std::os::unix::fs::symlink(&saved, &icon).unwrap();
    let output = temp.path().join("out");
    let error = generate(&config, &output, "").unwrap_err();
    assert!(error.to_string().contains("symbolic links"));
    assert!(!output.exists());
}
