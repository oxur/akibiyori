use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::config::{Config, validate_base_path, validate_route};
use crate::theme::ThemeSelection;

/// Selects one site from an opt-in showcase manifest.
#[derive(Debug, Clone)]
pub struct ShowcaseOptions {
    /// Strict v1 showcase manifest.
    pub manifest: PathBuf,
    /// Explicit site identifier from the manifest.
    pub site: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    docs: Docs,
    categories: Vec<Category>,
    sites: Vec<Site>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    theme: Option<ThemeSelection>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    theme_scope: Option<String>,
}

fn deserialize_nonnull_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Docs {
    site: String,
    route: String,
    label: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Category {
    id: String,
    label: String,
    site: String,
    route: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Site {
    id: String,
    label: String,
    config: PathBuf,
    mount: String,
    entry_route: String,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ShowcaseData {
    docs: PublicLink,
    categories: Vec<PublicLink>,
    current_label: String,
    is_demo: bool,
    sites: Vec<PublicSite>,
}

pub(crate) struct ShowcaseContext {
    pub data: ShowcaseData,
    pub theme: Option<ThemeSelection>,
    pub theme_key_scope: Option<String>,
    pub composition_root: String,
    pub theme_root: PathBuf,
}
#[derive(Debug, Serialize)]
struct PublicLink {
    label: String,
    href: String,
}
#[derive(Debug, Serialize)]
struct PublicSite {
    id: String,
    label: String,
    href: String,
    current: bool,
}
struct LoadedSite {
    record: Site,
    config_file: PathBuf,
    input_root: PathBuf,
    routes: BTreeSet<String>,
    implicit_theme: bool,
}

pub(crate) fn load_and_project(
    options: &ShowcaseOptions,
    current_config: &Path,
    current_root: &Path,
    base_path: &str,
    expected_sites: Option<&[String]>,
) -> Result<ShowcaseContext, Error> {
    let bytes =
        fs::read(&options.manifest).map_err(|error| Error::read(&options.manifest, error))?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|source| Error::Json {
        path: options.manifest.clone(),
        source,
    })?;
    if manifest.version != 1 {
        return Err(invalid("unsupported showcase manifest version; expected 1"));
    }
    if manifest.theme.is_some() != manifest.theme_scope.is_some() {
        return Err(invalid(
            "showcase theme and theme_scope must be provided together",
        ));
    }
    if let Some(scope) = &manifest.theme_scope {
        crate::theme::validate_theme_id(scope)?;
    }
    if let Some(theme) = &manifest.theme {
        theme.validate()?;
    }
    validate_id(&manifest.docs.site, "Docs site")?;
    validate_label(&manifest.docs.label, "Docs label")?;
    validate_route(&manifest.docs.route)?;
    if manifest.categories.is_empty() || manifest.sites.is_empty() {
        return Err(invalid("showcase categories and sites must not be empty"));
    }

    let manifest_root = manifest_root(&options.manifest)?;
    let mut categories = BTreeMap::new();
    for category in manifest.categories {
        validate_id(&category.id, "category id")?;
        validate_id(&category.site, "category owner site")?;
        validate_label(&category.label, "category label")?;
        validate_route(&category.route)?;
        if categories.insert(category.id.clone(), category).is_some() {
            return Err(invalid("duplicate showcase category id"));
        }
    }

    let mut ids = BTreeSet::new();
    let mut mounts = BTreeSet::new();
    let mut sites = Vec::with_capacity(manifest.sites.len());
    for site in manifest.sites {
        validate_id(&site.id, "site id")?;
        validate_label(&site.label, "site label")?;
        validate_route(&site.entry_route)?;
        validate_base_path(&site.mount)?;
        if !ids.insert(site.id.clone()) {
            return Err(invalid("duplicate showcase site id"));
        }
        if !mounts.insert(site.mount.clone()) {
            return Err(invalid("duplicate showcase site mount"));
        }
        if site
            .category
            .as_deref()
            .is_some_and(|id| !categories.contains_key(id))
        {
            return Err(invalid("showcase site names an unknown category"));
        }
        let config_path = if site.config.is_absolute() {
            site.config.clone()
        } else {
            manifest_root.join(&site.config)
        };
        let (config, root) = Config::load(&config_path)?;
        let config_file =
            fs::canonicalize(&config_path).map_err(|error| Error::read(&config_path, error))?;
        let input_root = canonicalize_input_root(&root)?;
        let routes = config.pages.iter().map(|page| page.route.clone()).collect();
        sites.push(LoadedSite {
            record: site,
            config_file,
            input_root,
            routes,
            implicit_theme: config.theme.is_implicit_default(),
        });
    }

    if let Some(expected) = expected_sites {
        validate_expected_sites(expected, &ids)?;
    }

    if manifest.theme.is_some() {
        for site in &sites {
            if !site.implicit_theme {
                return Err(invalid(format!(
                    "showcase central theme conflicts with site {:?} theme selection",
                    site.record.id
                )));
            }
        }
    }

    let docs_owner = sites
        .iter()
        .find(|site| site.record.id == manifest.docs.site)
        .ok_or_else(|| invalid("showcase Docs owner is not a declared site"))?;
    ensure_owned_route(docs_owner, &manifest.docs.route, "Docs")?;
    for category in categories.values() {
        let owner = sites
            .iter()
            .find(|site| site.record.id == category.site)
            .ok_or_else(|| invalid("showcase category owner is not a declared site"))?;
        ensure_owned_route(owner, &category.route, "category")?;
    }
    let mut used_categories = BTreeSet::new();
    for site in &sites {
        ensure_owned_route(site, &site.record.entry_route, "site entry")?;
        if let Some(category) = &site.record.category {
            used_categories.insert(category.as_str());
        }
    }
    if categories
        .keys()
        .any(|id| !used_categories.contains(id.as_str()))
    {
        return Err(invalid(
            "every showcase category must include at least one site",
        ));
    }

    let current_file =
        fs::canonicalize(current_config).map_err(|error| Error::read(current_config, error))?;
    let current_root = canonicalize_input_root(current_root)?;
    let current = sites
        .iter()
        .find(|site| site.record.id == options.site)
        .ok_or_else(|| invalid("current showcase site is not declared"))?;
    if current.config_file != current_file || current.input_root != current_root {
        return Err(invalid(
            "current config file and input root do not match the selected showcase owner",
        ));
    }

    let (composition_root, derived_base) = composition_root(base_path, &current.record.mount)?;
    if derived_base != base_path {
        return Err(invalid(
            "showcase mount does not match the current base path",
        ));
    }
    let global_href = |mount: &str, route: &str| format!("{composition_root}{mount}{route}");
    let current_label = if let Some(category_id) = current.record.category.as_deref() {
        let category = categories
            .get(category_id)
            .ok_or_else(|| invalid("current showcase category is missing"))?;
        format!("{} · {}", category.label, current.record.label)
    } else {
        current.record.label.clone()
    };
    let docs = PublicLink {
        label: manifest.docs.label,
        href: global_href(&docs_owner.record.mount, &manifest.docs.route),
    };
    let category_links = categories
        .values()
        .map(|category| {
            let owner = sites
                .iter()
                .find(|site| site.record.id == category.site)
                .expect("category owners validated above");
            PublicLink {
                label: category.label.clone(),
                href: global_href(&owner.record.mount, &category.route),
            }
        })
        .collect();
    let public_sites = sites
        .iter()
        .filter(|site| site.record.category.is_some())
        .map(|site| PublicSite {
            id: site.record.id.clone(),
            label: site.record.label.clone(),
            href: global_href(&site.record.mount, &site.record.entry_route),
            current: site.record.id == current.record.id,
        })
        .collect();

    Ok(ShowcaseContext {
        data: ShowcaseData {
            docs,
            categories: category_links,
            current_label,
            is_demo: current.record.category.is_some(),
            sites: public_sites,
        },
        theme: manifest.theme,
        theme_key_scope: manifest.theme_scope,
        composition_root,
        theme_root: manifest_root,
    })
}

fn validate_expected_sites(expected: &[String], actual: &BTreeSet<String>) -> Result<(), Error> {
    if expected.is_empty() {
        return Err(invalid("showcase expected-site set must not be empty"));
    }
    let mut requested = BTreeSet::new();
    for id in expected {
        validate_id(id, "expected site id")?;
        if !requested.insert(id.as_str()) {
            return Err(invalid(format!(
                "duplicate showcase expected site id {id:?}"
            )));
        }
    }
    if requested.len() != actual.len() || actual.iter().any(|id| !requested.contains(id.as_str())) {
        return Err(invalid(
            "showcase expected-site set does not exactly match manifest sites",
        ));
    }
    Ok(())
}

fn manifest_root(path: &Path) -> Result<PathBuf, Error> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::canonicalize(parent).map_err(|error| Error::read(parent, error))
}
fn canonicalize_input_root(path: &Path) -> Result<PathBuf, Error> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    fs::canonicalize(path).map_err(|error| Error::read(path, error))
}
fn composition_root(base_path: &str, mount: &str) -> Result<(String, String), Error> {
    if mount.is_empty() {
        return Ok((base_path.to_owned(), base_path.to_owned()));
    }
    let root = base_path
        .strip_suffix(mount)
        .ok_or_else(|| invalid("showcase mount is not an exact suffix of the base path"))?;
    let root = root.to_owned();
    validate_base_path(&root)?;
    Ok((root.clone(), format!("{root}{mount}")))
}
fn ensure_owned_route(owner: &LoadedSite, route: &str, kind: &str) -> Result<(), Error> {
    if owner.routes.contains(route) {
        Ok(())
    } else {
        Err(invalid(format!(
            "showcase {kind} route {route:?} is not authored by its named owner"
        )))
    }
}
fn validate_id(value: &str, kind: &str) -> Result<(), Error> {
    validate_label(value, kind)
}
fn validate_label(value: &str, kind: &str) -> Result<(), Error> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid(format!(
            "showcase {kind} must be non-empty and contain no controls"
        )));
    }
    Ok(())
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

#[cfg(test)]
mod tests {
    use super::composition_root;

    #[test]
    fn composition_root_removes_only_the_exact_current_mount_suffix() {
        assert_eq!(
            composition_root("/preview/demos/planets", "/demos/planets").unwrap(),
            ("/preview".to_owned(), "/preview/demos/planets".to_owned())
        );
        assert_eq!(
            composition_root("/demos/demos/planets", "/demos/planets").unwrap(),
            ("/demos".to_owned(), "/demos/demos/planets".to_owned())
        );
        assert!(composition_root("/preview/demos/planets-extra", "/demos/planets").is_err());
    }
}
