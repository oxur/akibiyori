use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub version: u32,
    pub title: String,
    pub contracts: Vec<ContractInput>,
    pub pages: Vec<AuthoredPage>,
    #[serde(default)]
    pub navigation: Vec<NavigationLink>,
    #[serde(default)]
    pub sidebar_groups: Vec<SidebarGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContractInput {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthoredPage {
    pub source: PathBuf,
    pub title: String,
    pub route: String,
    #[serde(default)]
    pub document: Option<DocumentHeader>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocumentHeader {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NavigationLink {
    pub label: String,
    pub href: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SidebarGroup {
    pub title: String,
    pub links: Vec<NavigationLink>,
}

impl Config {
    pub fn load(path: &Path) -> Result<(Self, PathBuf), Error> {
        let bytes = std::fs::read(path).map_err(|error| Error::read(path, error))?;
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            Error::Invalid(format!("config {} is not UTF-8: {error}", path.display()))
        })?;
        let config: Self = serde_json::from_str(text).map_err(|source| Error::Json {
            path: path.to_path_buf(),
            source,
        })?;
        config.validate()?;
        let root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok((config, root))
    }

    fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid(format!(
                "unsupported config version {}; expected 1",
                self.version
            )));
        }
        if self.title.trim().is_empty() {
            return Err(Error::Invalid("site title must not be empty".to_owned()));
        }
        if self.contracts.is_empty() || self.pages.is_empty() {
            return Err(Error::Invalid(
                "config must include at least one contract and one authored page".to_owned(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for contract in &self.contracts {
            validate_identifier(&contract.id, "contract id")?;
            if contract.label.trim().is_empty() {
                return Err(Error::Invalid(format!(
                    "contract {} has an empty label",
                    contract.id
                )));
            }
            if !ids.insert(&contract.id) {
                return Err(Error::Invalid(format!(
                    "duplicate contract id {:?}",
                    contract.id
                )));
            }
            validate_relative_input(&contract.path, "contract")?;
        }
        let mut routes = std::collections::BTreeSet::new();
        for page in &self.pages {
            validate_route(&page.route)?;
            if page.title.trim().is_empty() {
                return Err(Error::Invalid(format!(
                    "page at {} has an empty title",
                    page.route
                )));
            }
            if !routes.insert(page.route.as_str()) {
                return Err(Error::Invalid(format!(
                    "duplicate authored route {:?}",
                    page.route
                )));
            }
            validate_relative_input(&page.source, "authored page")?;
            if let Some(document) = &page.document
                && let Some(source_url) = &document.source_url
            {
                validate_source_url(source_url)?;
            }
        }
        for link in &self.navigation {
            validate_navigation_link(link)?;
        }
        for group in &self.sidebar_groups {
            if group.title.trim().is_empty() {
                return Err(Error::Invalid(
                    "sidebar group title must not be empty".to_owned(),
                ));
            }
            if group.links.is_empty() {
                return Err(Error::Invalid(format!(
                    "sidebar group {:?} must contain at least one link",
                    group.title
                )));
            }
            for link in &group.links {
                validate_navigation_link(link)?;
            }
        }
        Ok(())
    }
}

fn validate_source_url(value: &str) -> Result<(), Error> {
    let Some((scheme, remainder)) = value.split_once("://") else {
        return Err(Error::Invalid(
            "document source_url must be an absolute http(s) URL".to_owned(),
        ));
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
        || value.chars().any(|character| {
            character.is_whitespace() || character.is_control() || character == '\\'
        })
    {
        return Err(Error::Invalid(
            "document source_url must be an absolute http(s) URL without whitespace or backslashes"
                .to_owned(),
        ));
    }
    let authority = remainder.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() {
        return Err(Error::Invalid(
            "document source_url must have a non-empty authority".to_owned(),
        ));
    }
    Ok(())
}

fn validate_navigation_link(link: &NavigationLink) -> Result<(), Error> {
    if link.label.trim().is_empty() || link.href.trim().is_empty() {
        return Err(Error::Invalid(
            "navigation label and href must not be empty".to_owned(),
        ));
    }
    if link.href.starts_with('/') {
        validate_route(&link.href)
    } else if link.href.starts_with("https://") || link.href.starts_with("http://") {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "navigation href {:?} must be an internal route or an http(s) URL",
            link.href
        )))
    }
}

pub(crate) fn validate_identifier(value: &str, kind: &str) -> Result<(), Error> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte));
    if !valid {
        return Err(Error::Invalid(format!(
            "unsafe {kind} {value:?}; use ASCII letters, digits, '-', '_', or '.'"
        )));
    }
    Ok(())
}

fn validate_relative_input(path: &Path, kind: &str) -> Result<(), Error> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!(
            "{kind} path {:?} must be a non-empty relative path without dot segments",
            path
        )));
    }
    Ok(())
}

pub(crate) fn validate_route(route: &str) -> Result<(), Error> {
    if !route.starts_with('/') || route.contains(['?', '#', '%', '\\']) {
        return Err(Error::Invalid(format!("unsafe route {route:?}")));
    }
    if route != "/" && !route.ends_with('/') {
        return Err(Error::Invalid(format!("route {route:?} must end with '/'")));
    }
    if route == "/" {
        return Ok(());
    }
    let path = &route[1..route.len() - 1];
    for segment in path.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte))
        {
            return Err(Error::Invalid(format!("unsafe route segment in {route:?}")));
        }
    }
    Ok(())
}

pub(crate) fn validate_base_path(prefix: &str) -> Result<(), Error> {
    if prefix.is_empty() {
        return Ok(());
    }
    if !prefix.starts_with('/') || prefix.ends_with('/') {
        return Err(Error::Invalid(
            "base path must be empty or start with '/' and have no trailing slash".to_owned(),
        ));
    }
    validate_route(&format!("{prefix}/"))
}

pub(crate) fn check_output(output: &Path) -> Result<(), Error> {
    match std::fs::symlink_metadata(output) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(Error::Invalid(format!(
                    "output {} must be a real directory, not a symlink or file",
                    output.display()
                )));
            }
            let mut entries =
                std::fs::read_dir(output).map_err(|error| Error::read(output, error))?;
            if entries.next().is_some() {
                return Err(Error::Invalid(format!(
                    "output directory {} must be empty",
                    output.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::read(output, error)),
    }
    Ok(())
}
