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
    pub reference: ReferenceOptions,
    #[serde(default)]
    pub reference_sidebar: Option<ReferenceSidebarOptions>,
    #[serde(default)]
    pub brand_icon: Option<PathBuf>,
    #[serde(default)]
    pub icon_pack: Option<IconPack>,
    #[serde(default)]
    pub theme: crate::theme::ThemeSelection,
    #[serde(default)]
    pub navigation: Vec<NavigationLink>,
    #[serde(default)]
    pub sidebar_groups: Vec<SidebarGroup>,
    #[serde(default)]
    pub sidebar_sections: Vec<NavigationLink>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IconPack {
    pub directory: PathBuf,
    #[serde(default = "icon_background")]
    pub theme_color: String,
    #[serde(default = "icon_background")]
    pub background_color: String,
}

fn icon_background() -> String {
    // Match the supplied pack's opaque Apple/maskable canvas.
    "#111111".to_owned()
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReferenceSidebarPresentation {
    #[default]
    Flat,
    ContractAccordion,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferenceSidebarOptions {
    pub overview_label: String,
    #[serde(default)]
    pub untagged_label: Option<String>,
    #[serde(default)]
    pub presentation: ReferenceSidebarPresentation,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferenceOptions {
    #[serde(default)]
    pub layout: ReferenceLayout,
    #[serde(default)]
    pub descriptions: DescriptionMode,
    #[serde(default)]
    pub images: ImageMode,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReferenceLayout {
    #[default]
    Source,
    Columns,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DescriptionMode {
    #[default]
    Plain,
    Markdown,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ImageMode {
    #[default]
    Link,
    Https,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContractInput {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    #[serde(default)]
    pub document_uri: Option<String>,
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
    #[serde(default)]
    pub icon: Option<NavigationIcon>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderSection {
    Documentation,
    Reference,
}

pub(crate) fn active_sidebar_section<'a>(
    sections: &'a [NavigationLink],
    route: &str,
) -> Option<&'a NavigationLink> {
    let matches = sections
        .iter()
        .filter(|section| route_matches(&section.href, route))
        .max_by_key(|section| section.href.len());
    matches
        .or_else(|| {
            sections
                .iter()
                .find(|section| !section.href.starts_with("/reference/"))
        })
        .or_else(|| sections.first())
}

fn route_matches(section: &str, route: &str) -> bool {
    section == "/" || route == section || route.starts_with(section)
}

pub(crate) fn reader_section(section: &NavigationLink) -> ReaderSection {
    if section.href.starts_with("/reference/") {
        ReaderSection::Reference
    } else {
        ReaderSection::Documentation
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum NavigationIcon {
    Rocket,
    CircleHelp,
    BookOpen,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SidebarGroup {
    pub title: String,
    pub links: Vec<NavigationLink>,
}

impl Config {
    pub(crate) fn documentation_home(&self) -> Option<&NavigationLink> {
        self.sidebar_sections.iter().find(|section| {
            !section.href.starts_with("/reference/")
                && self.pages.iter().any(|page| page.route == section.href)
        })
    }

    pub(crate) fn reference_section(&self) -> Option<&NavigationLink> {
        self.sidebar_sections
            .iter()
            .find(|section| section.href == "/reference/")
    }

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
        if let Some(reference_sidebar) = &self.reference_sidebar {
            if reference_sidebar.overview_label.trim().is_empty() {
                return Err(Error::Invalid(
                    "reference_sidebar overview_label must not be empty".to_owned(),
                ));
            }
            if reference_sidebar
                .untagged_label
                .as_deref()
                .is_some_and(|label| label.trim().is_empty())
            {
                return Err(Error::Invalid(
                    "reference_sidebar untagged_label must not be empty when provided".to_owned(),
                ));
            }
        }
        if let Some(brand_icon) = &self.brand_icon {
            validate_relative_input(brand_icon, "brand icon")?;
        }
        if let Some(pack) = &self.icon_pack {
            validate_relative_input(&pack.directory, "icon pack")?;
            for color in [&pack.theme_color, &pack.background_color] {
                if color.len() != 7
                    || !color.starts_with('#')
                    || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
                {
                    return Err(Error::Invalid(
                        "icon pack colors must use #RRGGBB hexadecimal notation".to_owned(),
                    ));
                }
            }
        }
        self.theme.validate()?;
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
        let mut section_routes = std::collections::BTreeSet::new();
        for section in &self.sidebar_sections {
            if !section.href.starts_with('/') {
                return Err(Error::Invalid(format!(
                    "sidebar section href {:?} must be an internal route",
                    section.href
                )));
            }
            validate_navigation_link(section)?;
            if !section_routes.insert(section.href.as_str()) {
                return Err(Error::Invalid(format!(
                    "duplicate sidebar section route {:?}",
                    section.href
                )));
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

#[cfg(test)]
mod reference_option_tests {
    use super::{DescriptionMode, ImageMode, ReferenceLayout, ReferenceOptions};

    #[test]
    fn reference_options_default_independently_and_reject_unknown_fields() {
        let defaults: ReferenceOptions = serde_json::from_str("{}").expect("empty options");
        assert_eq!(defaults.layout, ReferenceLayout::Source);
        assert_eq!(defaults.descriptions, DescriptionMode::Plain);
        assert_eq!(defaults.images, ImageMode::Link);

        let markdown: ReferenceOptions = serde_json::from_str(r#"{"descriptions":"markdown"}"#)
            .expect("independent Markdown option");
        assert_eq!(markdown.layout, ReferenceLayout::Source);
        assert_eq!(markdown.descriptions, DescriptionMode::Markdown);
        assert_eq!(markdown.images, ImageMode::Link);

        let columns: ReferenceOptions =
            serde_json::from_str(r#"{"layout":"columns"}"#).expect("independent layout option");
        assert_eq!(columns.layout, ReferenceLayout::Columns);
        assert_eq!(columns.descriptions, DescriptionMode::Plain);
        assert_eq!(columns.images, ImageMode::Link);

        assert!(serde_json::from_str::<ReferenceOptions>(r#"{"unknown":true}"#).is_err());
    }
}
