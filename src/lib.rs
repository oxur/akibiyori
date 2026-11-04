//! Generate deterministic Cobalt source pages from OpenAPI documents and Markdown.

mod config;
mod error;
mod openapi;
mod render;

pub use error::Error;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use config::{AuthoredPage, Config, check_output, validate_base_path, validate_route};
use openapi::{Contract, ensure_unique_operations};
use render::{
    RenderContext, generated_source_file, navigation_data, render_contract, render_indexes,
    render_navigation, render_page,
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
    validate_base_path(base_path)?;
    check_output(output)?;
    let (config, config_root) = Config::load(config_path)?;
    let contracts = config
        .contracts
        .iter()
        .map(|contract| Contract::load(contract, &config_root))
        .collect::<Result<Vec<_>, _>>()?;
    ensure_unique_operations(&contracts)?;

    let mut files = Vec::new();
    let mut routes = BTreeSet::new();
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
        files.push(PendingFile {
            path: generated_source_file(&page.route),
            contents: render_page(&page.title, &page.route, body).into_bytes(),
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
    let context = RenderContext { base_path };
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
    files.extend(render_indexes(&contracts, &context)?);
    validate_public_targets(&routes)?;
    for link in &config.navigation {
        if link.href.starts_with('/') && !routes.contains(&link.href) {
            return Err(Error::Invalid(format!(
                "navigation href {:?} does not match an authored or generated route",
                link.href
            )));
        }
    }
    let layout = String::from_utf8_lossy(include_bytes!("../theme/_layouts/default.liquid"))
        .replace("__AKIBIYORI_BASE_PATH__", base_path);
    files.push(PendingFile {
        path: PathBuf::from("_layouts/default.liquid"),
        contents: layout.into_bytes(),
    });
    files.push(PendingFile {
        path: PathBuf::from("css/akibiyori.css"),
        contents: include_bytes!("../theme/css/akibiyori.css").to_vec(),
    });
    files.push(PendingFile {
        path: PathBuf::from("_cobalt.yml"),
        contents: format!("source: .\ndestination: _site\nsite:\n  title: {}\nsyntax_highlight:\n  enabled: false\n", yaml_quote(&config.title)).into_bytes(),
    });
    files.push(PendingFile {
        path: PathBuf::from("_data/akibiyori_navigation.json"),
        contents: serialize_json(&navigation_data(&config.navigation, base_path))?,
    });
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

fn validate_public_targets(routes: &BTreeSet<String>) -> Result<(), Error> {
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
    const ASSET: &str = "css/akibiyori.css";
    let folded_asset = ASSET.to_ascii_lowercase();
    if targets
        .insert(folded_asset, "/css/akibiyori.css".to_owned())
        .is_some()
    {
        return Err(Error::Invalid(
            "public route collides with generated CSS asset /css/akibiyori.css".to_owned(),
        ));
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

fn route_target(route: &str) -> String {
    if route == "/" {
        "index.html".to_owned()
    } else {
        format!("{}index.html", route.trim_start_matches('/'))
    }
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
