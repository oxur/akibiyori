//! Generate deterministic Cobalt source pages from OpenAPI documents and Markdown.

mod build;
mod config;
mod error;
mod examples;
mod icons;
mod input;
mod markdown;
mod openapi;
mod render;
mod schema_reading;
mod schema_resources;
mod showcase;
mod theme;

pub use build::{BuildOptions, BuildReport, build, build_with_showcase};
pub use error::Error;
pub use showcase::ShowcaseOptions;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use config::{
    AuthoredPage, Config, DescriptionMode, ReferenceLayout, check_output, validate_base_path,
    validate_route,
};
use openapi::{Contract, ensure_unique_operations};
use render::{
    RenderContext, generated_source_file, navigation_data, reference_sidebar_data, render_contract,
    render_document_page, render_indexes, render_navigation, render_page, route_target,
    section_data, sidebar_data,
};
use serde::Serialize;

/// Summary of a completed generation run.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationReport {
    /// Number of authored pages copied into the output source tree.
    pub authored_pages: usize,
    /// Number of generated operation reference pages.
    pub operation_pages: usize,
    /// Number of generated named schema pages.
    pub model_pages: usize,
    /// Non-fatal presentation limitations. Raw contract JSON remains visible.
    pub diagnostics: Vec<Diagnostic>,
}

/// A visible limitation recorded during structural rendering.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Contract identifier that owns the feature.
    pub contract: String,
    /// JSON pointer to the original contract element.
    pub pointer: String,
    /// Explanation of the display limitation.
    pub message: String,
}

struct PendingFile {
    path: PathBuf,
    contents: Vec<u8>,
}

/// Generate a deterministic Cobalt source tree from a strict JSON config.
///
/// The output must be absent or an empty real directory. All inputs are parsed,
/// validated, and rendered before the first output write. Input and validation
/// failures therefore leave the output absent or unchanged. A filesystem error
/// during writing can leave a partial output tree.
///
/// # Errors
/// Returns [`Error`] for malformed input, unsupported references, unsafe routes,
/// collisions, invalid output state, or filesystem failures.
pub fn generate(
    config_path: &Path,
    output: &Path,
    base_path: &str,
) -> Result<GenerationReport, Error> {
    generate_internal(config_path, output, base_path, None)
}

/// Generate a Cobalt source tree with an opt-in, owner-validated showcase context.
///
/// The manifest and all named owner configs are validated before output writes.
pub fn generate_with_showcase(
    config_path: &Path,
    output: &Path,
    base_path: &str,
    showcase: &ShowcaseOptions,
) -> Result<GenerationReport, Error> {
    generate_internal(config_path, output, base_path, Some(showcase))
}

fn generate_internal(
    config_path: &Path,
    output: &Path,
    base_path: &str,
    showcase: Option<&ShowcaseOptions>,
) -> Result<GenerationReport, Error> {
    validate_base_path(base_path)?;
    check_output(output)?;
    let (config, config_root) = Config::load(config_path)?;
    let showcase_data = showcase
        .map(|options| showcase::load_and_project(options, config_path, &config_root, base_path))
        .transpose()?;
    let theme = config.theme.resolve(&config_root, base_path)?;
    let theme_data = theme.data_json()?;
    let brand_icon = config
        .brand_icon
        .as_deref()
        .map(|path| read_brand_icon(&config_root, path))
        .transpose()?;
    let contracts = config
        .contracts
        .iter()
        .map(|contract| Contract::load(contract, &config_root))
        .collect::<Result<Vec<_>, _>>()?;
    ensure_unique_operations(&contracts)?;

    let mut files = Vec::new();
    let mut routes = BTreeSet::new();
    let description_renderer = (config.reference.descriptions == DescriptionMode::Markdown
        || config.reference.layout == ReferenceLayout::Columns)
        .then(markdown::DescriptionRenderer::new);
    let context = RenderContext {
        base_path,
        has_home: config.pages.iter().any(|page| page.route == "/"),
        reference: config.reference,
        reference_sidebar: config.reference_sidebar.is_some(),
        markdown: description_renderer.as_ref(),
        model_field_targets: None,
    };
    for page in &config.pages {
        validate_route(&page.route)?;
        if !routes.insert(page.route.clone()) {
            return Err(Error::Invalid(format!(
                "duplicate authored route {:?}",
                page.route
            )));
        }
        let source_path = config_root.join(&page.source);
        let bytes =
            std::fs::read(&source_path).map_err(|error| Error::read(&source_path, error))?;
        let body = std::str::from_utf8(&bytes).map_err(|error| {
            Error::Invalid(format!(
                "authored page {} is not UTF-8: {error}",
                source_path.display()
            ))
        })?;
        let body = strip_frontmatter(body, &source_path)?;
        let rendered = match &page.document {
            Some(document) => {
                render_document_page(&page.title, &page.route, body, document, &context)
            }
            None => render_page(&page.title, &page.route, body, "default.liquid", &context),
        };
        files.push(PendingFile {
            path: generated_source_file(&page.route),
            contents: rendered.into_bytes(),
        });
    }

    if !routes.contains("/reference/") {
        return Err(Error::Invalid(
            "config must author the /reference/ overview route".to_owned(),
        ));
    }
    let mut operation_ids = BTreeSet::new();
    let mut model_routes = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let reference_sidebar = config
        .reference_sidebar
        .as_ref()
        .map(|options| reference_sidebar_data(&contracts, options, base_path, &mut diagnostics))
        .transpose()?;
    for contract in &contracts {
        for model_name in contract.schemas.keys() {
            let route = format!("/reference/models/{}/{model_name}/", contract.id);
            if !routes.insert(route.clone()) {
                return Err(Error::Invalid(format!(
                    "generated model route {route:?} collides with another page"
                )));
            }
            model_routes.insert(route);
        }
        for operation in &contract.operations {
            let route = format!("/reference/{}/", operation.id);
            if !routes.insert(route.clone()) {
                return Err(Error::Invalid(format!(
                    "generated operation route {route:?} collides with another page"
                )));
            }
            operation_ids.insert(operation.id.as_str());
        }
        render_contract(contract, &context, &mut files, &mut diagnostics)?;
    }
    for index_route in ["/reference/operations/", "/reference/models/"] {
        if !routes.insert(index_route.to_owned()) {
            return Err(Error::Invalid(format!(
                "generated index route {index_route:?} collides with another page"
            )));
        }
    }
    const SEARCH_ROUTE: &str = "/search/";
    if !routes.insert(SEARCH_ROUTE.to_owned()) {
        return Err(Error::Invalid(
            "generated search route /search/ collides with another page".to_owned(),
        ));
    }
    files.extend(render_indexes(&contracts, &context)?);
    files.push(PendingFile {
        path: generated_source_file(SEARCH_ROUTE),
        contents: render_page(
            "Search",
            SEARCH_ROUTE,
            &format!(
                "<h1>Search</h1>\n<p>Search guides and API reference pages. Select a result to open its page.</p>\n<div id=\"pagefind\" class=\"search-ui\" data-base-path=\"{}\">\n  <p role=\"status\">Loading search…</p>\n  <noscript>Search needs JavaScript. Browse <a href=\"{}\">operations</a> or <a href=\"{}\">models</a>.</noscript>\n</div>\n<p class=\"search-fallback\">Browse <a href=\"{}\">all operations</a> or <a href=\"{}\">all models</a>.</p>",
                base_path,
                context.href("/reference/operations/"),
                context.href("/reference/models/"),
                context.href("/reference/operations/"),
                context.href("/reference/models/")
            ),
            "search.liquid",
            &context,
        )
        .into_bytes(),
    });
    validate_public_targets(
        &routes,
        brand_icon.is_some(),
        config.icon_pack.as_ref(),
        &theme.public_paths,
    )?;
    for link in config
        .navigation
        .iter()
        .chain(config.sidebar_groups.iter().flat_map(|group| &group.links))
        .chain(config.sidebar_sections.iter())
    {
        if link.href.starts_with('/') && !routes.contains(&link.href) {
            return Err(Error::Invalid(format!(
                "navigation href {:?} does not match an authored or generated route",
                link.href
            )));
        }
    }
    if let Some(pack) = &config.icon_pack {
        files.extend(icons::files(
            &config_root,
            pack,
            &config.title,
            base_path,
            context.has_home,
        )?);
    }
    files.extend(theme.files.into_iter().map(|file| PendingFile {
        path: file.path,
        contents: file.contents,
    }));
    files.push(PendingFile {
        path: PathBuf::from("_includes/icons.liquid"),
        contents: String::from_utf8_lossy(include_bytes!("../theme/_includes/icons.liquid"))
            .replace("__AKIBIYORI_BASE_PATH__", base_path)
            .into_bytes(),
    });
    let layout = String::from_utf8_lossy(include_bytes!("../theme/_layouts/default.liquid"))
        .replace("__AKIBIYORI_BASE_PATH__", base_path);
    files.push(PendingFile {
        path: PathBuf::from("_layouts/default.liquid"),
        contents: layout.into_bytes(),
    });
    files.push(PendingFile {
        path: PathBuf::from("_layouts/search.liquid"),
        contents: String::from_utf8_lossy(include_bytes!("../theme/_layouts/search.liquid"))
            .replace("__AKIBIYORI_BASE_PATH__", base_path)
            .into_bytes(),
    });
    files.push(PendingFile {
        path: PathBuf::from("css/akibiyori.css"),
        contents: include_bytes!("../theme/css/akibiyori.css").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("css/fumadocs-reference.css"),
        contents: include_bytes!("../theme/vendor/fumadocs-reference.css").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("css/akibiyori-tokens.css"),
        contents: include_bytes!("../theme/css/akibiyori-tokens.css").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("css/akibiyori-fonts.css"),
        contents: include_bytes!("../theme/css/akibiyori-fonts.css").to_vec(),
    });
    for (path, contents) in [
        (
            "fonts/geist-latin.woff2",
            include_bytes!("../theme/vendor/fonts/geist-latin.woff2").as_slice(),
        ),
        (
            "fonts/jetbrains-mono-latin.woff2",
            include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2").as_slice(),
        ),
        (
            "icons/search.svg",
            include_bytes!("../theme/vendor/icons/search.svg").as_slice(),
        ),
    ] {
        files.push(PendingFile {
            path: PathBuf::from(path),
            contents: contents.to_vec(),
        });
    }
    if let Some(contents) = &brand_icon {
        files.push(PendingFile {
            path: PathBuf::from("icons/brand.png"),
            contents: contents.clone(),
        });
    }
    files.push(PendingFile {
        path: PathBuf::from("js/akibiyori-mode.js"),
        contents: include_bytes!("../theme/js/akibiyori-mode.js").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("js/akibiyori-search.js"),
        contents: include_bytes!("../theme/js/akibiyori-search.js").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("js/akibiyori-reading.js"),
        contents: include_bytes!("../theme/js/akibiyori-reading.js").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("_cobalt.yml"),
        contents: format!("source: .\ndestination: _site\nsite:\n  title: {}\nsyntax_highlight:\n  enabled: true\n  theme: css\n", yaml_quote(&config.title)).into_bytes(),
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_navigation.json"),
        contents: serialize_json(&navigation_data(&config.navigation, base_path))?,
    });
    let (home_href, home_label) = if config.pages.iter().any(|page| page.route == "/") {
        (context.href("/"), "Home")
    } else {
        (context.href("/reference/"), "API reference")
    };
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_site.json"),
        contents: serialize_json(&serde_json::json!({
            "home_href": home_href,
            "home_label": home_label,
            "search_href": format!("{base_path}/search/"),
            "search_icon": format!("{base_path}/icons/search.svg"),
            "icon_href": brand_icon.as_ref().map(|_| format!("{base_path}/icons/brand.png")),
            "base_path": base_path,
            "icon_pack": config.icon_pack.as_ref().map(|pack| serde_json::json!({
                "theme_color": pack.theme_color,
            })),
        }))?,
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_theme.json"),
        contents: theme_data,
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_sidebar.json"),
        contents: serialize_json(&sidebar_data(
            &config.sidebar_groups,
            &config.navigation,
            base_path,
        ))?,
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_sections.json"),
        contents: serialize_json(&section_data(&config.sidebar_sections, base_path))?,
    });
    if let Some(reference_sidebar) = reference_sidebar {
        files.push(PendingFile {
            path: PathBuf::from("_data/akibiyori_reference_sidebar.json"),
            contents: serialize_json(&reference_sidebar)?,
        });
    }
    if let Some(showcase) = showcase_data {
        files.push(PendingFile {
            path: PathBuf::from("_data/akibiyori_showcase.json"),
            contents: serialize_json(&showcase)?,
        });
    }
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori-diagnostics.json"),
        contents: serialize_json(&diagnostics)?,
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori-manifest.json"),
        contents: serialize_json(&route_manifest(&contracts, &config.pages, &context))?,
    });
    let navigation = render_navigation();
    files.push(PendingFile {
        path: PathBuf::from("_includes/navigation.liquid"),
        contents: navigation.into_bytes(),
    });
    validate_file_tree(&files)?;

    let report = GenerationReport {
        authored_pages: config.pages.len(),
        operation_pages: operation_ids.len(),
        model_pages: model_routes.len(),
        diagnostics,
    };
    write_all(output, &files)?;
    Ok(report)
}

#[derive(Serialize)]
struct Manifest<'a> {
    authored_routes: Vec<&'a str>,
    operation_routes: BTreeMap<&'a str, String>,
    model_routes: BTreeMap<String, String>,
    content_source_files: BTreeMap<String, String>,
    contract_sources: BTreeMap<&'a str, &'a str>,
}

fn route_manifest<'a>(
    contracts: &'a [Contract],
    pages: &'a [AuthoredPage],
    context: &RenderContext<'_>,
) -> Manifest<'a> {
    let mut operations = BTreeMap::new();
    let mut models = BTreeMap::new();
    let mut content_source_files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for page in pages {
        content_source_files.insert(
            page.route.clone(),
            generated_source_file(&page.route).display().to_string(),
        );
    }
    for contract in contracts {
        sources.insert(contract.id.as_str(), contract.source.as_str());
        for operation in &contract.operations {
            let route = format!("/reference/{}/", operation.id);
            operations.insert(operation.id.as_str(), context.href(&route));
            content_source_files.insert(
                route.clone(),
                generated_source_file(&route).display().to_string(),
            );
        }
        for name in contract.schemas.keys() {
            let key = format!("{}:{name}", contract.id);
            let route = format!("/reference/models/{}/{name}/", contract.id);
            models.insert(key.clone(), context.href(&route));
            content_source_files.insert(
                route.clone(),
                generated_source_file(&route).display().to_string(),
            );
        }
    }
    for route in ["/reference/operations/", "/reference/models/"] {
        content_source_files.insert(
            route.to_owned(),
            generated_source_file(route).display().to_string(),
        );
    }
    content_source_files.insert(
        "/search/".to_owned(),
        generated_source_file("/search/").display().to_string(),
    );
    Manifest {
        authored_routes: pages.iter().map(|page| page.route.as_str()).collect(),
        operation_routes: operations,
        model_routes: models,
        content_source_files,
        contract_sources: sources,
    }
}

fn strip_frontmatter<'a>(body: &'a str, path: &Path) -> Result<&'a str, Error> {
    let Some(first_line_end) = body.find('\n') else {
        if body.trim_end_matches('\r') == "---" {
            return Err(Error::Invalid(format!(
                "authored page {} has unterminated leading frontmatter",
                path.display()
            )));
        }
        return Ok(body);
    };
    let (first_line, rest_with_line) = body.split_at(first_line_end);
    if first_line.trim_end_matches('\r') != "---" {
        return Ok(body);
    }
    let rest = &rest_with_line[1..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok(&rest[offset..]);
        }
    }
    Err(Error::Invalid(format!(
        "authored page {} has unterminated leading frontmatter",
        path.display()
    )))
}

fn read_brand_icon(root: &Path, relative: &Path) -> Result<Vec<u8>, Error> {
    let contents = read_icon_input(root, relative, "brand icon")?;
    if !contents.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::Invalid(
            "brand icon input must have the PNG signature".to_owned(),
        ));
    }
    Ok(contents)
}

fn read_icon_input(root: &Path, relative: &Path, label: &str) -> Result<Vec<u8>, Error> {
    let root = if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    };
    let canonical_root = root
        .canonicalize()
        .map_err(|error| Error::read(root, error))?;
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(Error::Invalid(format!(
                "{label} path must be a relative path without dot segments"
            )));
        };
        path.push(name);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|error| Error::read(&path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Invalid(format!(
                "{label} path must not traverse symbolic links"
            )));
        }
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|error| Error::read(&path, error))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(Error::Invalid(format!(
            "{label} path must remain within the configuration directory"
        )));
    }
    if !canonical_path.is_file() {
        return Err(Error::Invalid(format!(
            "{label} input must be a regular file"
        )));
    }
    let contents =
        std::fs::read(&canonical_path).map_err(|error| Error::read(&canonical_path, error))?;
    Ok(contents)
}

fn validate_public_targets(
    routes: &BTreeSet<String>,
    has_brand_icon: bool,
    icon_pack: Option<&config::IconPack>,
    theme_paths: &[String],
) -> Result<(), Error> {
    let mut targets = BTreeMap::<String, String>::new();
    for route in routes {
        let target = route_target(route);
        let folded = target.to_ascii_lowercase();
        if let Some(previous) = targets.insert(folded, route.clone()) {
            return Err(Error::Invalid(format!(
                "portable public route collision between {previous:?} and {route:?}"
            )));
        }
    }
    let mut assets = vec![
        ("css/akibiyori.css", "/css/akibiyori.css asset"),
        (
            "css/fumadocs-reference.css",
            "/css/fumadocs-reference.css asset",
        ),
        (
            "css/akibiyori-tokens.css",
            "/css/akibiyori-tokens.css asset",
        ),
        ("css/akibiyori-fonts.css", "/css/akibiyori-fonts.css asset"),
        ("js/akibiyori-mode.js", "/js/akibiyori-mode.js asset"),
        ("js/akibiyori-search.js", "/js/akibiyori-search.js asset"),
        ("js/akibiyori-reading.js", "/js/akibiyori-reading.js asset"),
        ("fonts/geist-latin.woff2", "/fonts/geist-latin.woff2 asset"),
        (
            "fonts/jetbrains-mono-latin.woff2",
            "/fonts/jetbrains-mono-latin.woff2 asset",
        ),
        ("icons/search.svg", "/icons/search.svg asset"),
        ("pagefind", "/pagefind/ Pagefind bundle"),
    ];
    if has_brand_icon {
        assets.push(("icons/brand.png", "/icons/brand.png asset"));
    }
    assets.extend(
        theme_paths
            .iter()
            .map(|path| (path.as_str(), "theme asset")),
    );
    for (asset_path, route_label) in assets {
        let folded_asset = asset_path.to_ascii_lowercase();
        if targets
            .insert(folded_asset, route_label.to_owned())
            .is_some()
        {
            return Err(Error::Invalid(format!(
                "public route collides with generated asset {route_label}"
            )));
        }
    }
    if icon_pack.is_some() {
        for path in icons::public_paths() {
            if targets
                .insert(
                    path.to_ascii_lowercase(),
                    format!("/{path} icon pack asset"),
                )
                .is_some()
            {
                return Err(Error::Invalid(format!(
                    "public route collides with icon pack asset {path}"
                )));
            }
        }
    }
    let entries: Vec<(String, String)> = targets.into_iter().collect();
    for (file, route) in &entries {
        if let Some((nested, nested_route)) = entries.iter().find(|(candidate, _)| {
            candidate.starts_with(file) && candidate.as_bytes().get(file.len()) == Some(&b'/')
        }) {
            return Err(Error::Invalid(format!(
                "public file/directory collision between {route:?} ({file}) and {nested_route:?} ({nested})"
            )));
        }
    }
    Ok(())
}

fn yaml_quote(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04X}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    format!("\"{escaped}\"")
}

fn serialize_json(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    serde_json::to_vec_pretty(value)
        .map_err(|error| Error::Invalid(format!("failed to serialize generated JSON: {error}")))
}

fn write_all(output: &Path, files: &[PendingFile]) -> Result<(), Error> {
    if !output.exists() {
        std::fs::create_dir_all(output).map_err(|error| Error::write(output, error))?;
    }
    for file in files {
        let destination = output.join(&file.path);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| Error::write(parent, error))?;
        }
        std::fs::write(&destination, &file.contents)
            .map_err(|error| Error::write(&destination, error))?;
    }
    Ok(())
}

fn validate_file_tree(files: &[PendingFile]) -> Result<(), Error> {
    let mut paths = BTreeSet::new();
    for file in files {
        if !paths.insert(file.path.clone()) {
            return Err(Error::Invalid(format!(
                "multiple generated inputs target {}",
                file.path.display()
            )));
        }
    }
    for path in &paths {
        if let Some(parent) = paths
            .iter()
            .find(|other| *other != path && path.starts_with(other))
        {
            return Err(Error::Invalid(format!(
                "generated file {} conflicts with nested output {}",
                parent.display(),
                path.display()
            )));
        }
    }
    Ok(())
}
