use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

use clap::ValueEnum;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{Error, read_icon_input};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThemeSelection {
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    source: Option<ThemeSource>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    catalog: Option<Vec<ThemeSource>>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    deploy: Option<DeployPair>,
    #[serde(default)]
    presentation: ThemePresentation,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    demo_default: Option<String>,
    #[serde(default, rename = "mode")]
    default_mode: ThemePreference,
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self {
            source: None,
            catalog: None,
            deploy: None,
            presentation: ThemePresentation::Deploy,
            demo_default: None,
            default_mode: ThemePreference::Dark,
        }
    }
}

fn deserialize_nonnull_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeployPair {
    dark: String,
    light: String,
}

#[derive(Debug, Clone, Deserialize)]
enum ThemeSource {
    #[serde(rename = "builtin")]
    Builtin(String),
    #[serde(rename = "file")]
    File(PathBuf),
}

impl Default for ThemeSource {
    fn default() -> Self {
        Self::Builtin("fumadocs".to_owned())
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ThemePreference {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemePreference {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    fn effective_mode(self) -> ThemeMode {
        match self {
            Self::Light => ThemeMode::Light,
            Self::Dark | Self::System => ThemeMode::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Selects the visitor-facing theme control and profile population.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ThemePresentation {
    /// Publish the mapped Dark and Light profiles with the existing Mode control.
    #[default]
    Deploy,
    /// Publish the complete catalog with the named Theme control.
    Demo,
}

impl ThemePresentation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Deploy => "deploy",
            Self::Demo => "demo",
        }
    }
}

impl ThemeSelection {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.source.is_some() && self.catalog.is_some() {
            return Err(Error::Invalid(
                "theme.source and theme.catalog are mutually exclusive".to_owned(),
            ));
        }
        if self.catalog.is_some() != self.deploy.is_some() {
            return Err(Error::Invalid(
                "theme.catalog requires an explicit theme.deploy dark/light pair".to_owned(),
            ));
        }
        if let Some(catalog) = &self.catalog {
            if catalog.is_empty() {
                return Err(Error::Invalid(
                    "theme.catalog must contain at least one family".to_owned(),
                ));
            }
            for source in catalog {
                validate_source(source)?;
            }
        }
        if let Some(source) = &self.source {
            validate_source(source)?;
        }
        if let Some(default) = &self.demo_default {
            validate_theme_id(default)?;
        }
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        config_root: &Path,
        base_path: &str,
        presentation_override: Option<ThemePresentation>,
    ) -> Result<ResolvedTheme, Error> {
        self.validate()?;
        if self.catalog.is_some() {
            return self.resolve_catalog(config_root, base_path, presentation_override);
        }
        self.resolve_source(
            config_root,
            base_path,
            self.source
                .as_ref()
                .unwrap_or(&ThemeSource::Builtin("fumadocs".to_owned())),
            presentation_override,
        )
    }

    fn resolve_source(
        &self,
        config_root: &Path,
        base_path: &str,
        source: &ThemeSource,
        presentation_override: Option<ThemePresentation>,
    ) -> Result<ResolvedTheme, Error> {
        let family_id = match source {
            ThemeSource::Builtin(name) if name == "fumadocs" => "fumadocs".to_owned(),
            ThemeSource::Builtin(name) if name == "neutral-example" => {
                let bytes =
                    include_bytes!("../examples/observatory/themes/neutral-example/theme.json");
                let manifest: ThemeManifest = serde_json::from_slice(bytes).map_err(|source| {
                    Error::Invalid(format!(
                        "invalid bundled neutral-example manifest: {source}"
                    ))
                })?;
                manifest.validate()?;
                manifest.id
            }
            ThemeSource::Builtin(name) => {
                return Err(Error::Invalid(format!(
                    "unsupported built-in theme {name:?}; expected \"fumadocs\" or \"neutral-example\""
                )));
            }
            ThemeSource::File(manifest_path) => {
                validate_source_file(manifest_path, "theme manifest", &["json"])?;
                let bytes = read_icon_input(config_root, manifest_path, "theme manifest")?;
                let manifest: ThemeManifest =
                    serde_json::from_slice(&bytes).map_err(|source| Error::Json {
                        path: config_root.join(manifest_path),
                        source,
                    })?;
                manifest.validate()?;
                if manifest.modes.is_none() {
                    return Err(Error::Invalid(
                        "named variant manifests require theme.catalog selection".to_owned(),
                    ));
                }
                manifest.id
            }
        };
        let normalized = ThemeSelection {
            source: None,
            catalog: Some(vec![source.clone()]),
            deploy: Some(DeployPair {
                dark: format!("{family_id}-dark"),
                light: format!("{family_id}-light"),
            }),
            presentation: self.presentation,
            demo_default: self.demo_default.clone(),
            default_mode: self.default_mode,
        };
        normalized.resolve_catalog(config_root, base_path, presentation_override)
    }

    fn resolve_catalog(
        &self,
        config_root: &Path,
        base_path: &str,
        presentation_override: Option<ThemePresentation>,
    ) -> Result<ResolvedTheme, Error> {
        let catalog = self.catalog.as_ref().expect("validated catalog");
        let deploy = self.deploy.as_ref().expect("validated deploy pair");
        let mut family_ids = BTreeSet::new();
        let mut profiles = BTreeMap::new();
        let mut ordered_profiles = Vec::new();
        let mut stylesheet_families = BTreeSet::new();
        let mut family_files: BTreeMap<String, ThemeFamilyFiles> = BTreeMap::new();
        let mut family_faces: BTreeMap<String, Vec<FontRecord>> = BTreeMap::new();
        for source in catalog {
            let (family_id, manifest, stylesheet, assets, faces) =
                load_catalog_family(config_root, source)?;
            if !family_ids.insert(family_id.clone()) {
                return Err(Error::Invalid(format!(
                    "duplicate theme family id {family_id:?}"
                )));
            }
            if stylesheet.is_some() {
                stylesheet_families.insert(family_id.clone());
            }
            family_files.insert(family_id.clone(), (stylesheet, assets));
            family_faces.insert(family_id.clone(), faces);
            let variants = match manifest {
                None => vec![
                    NamedVariant {
                        id: "fumadocs-dark".into(),
                        label: "Fuma Dark".into(),
                        scheme: ThemeMode::Dark,
                        colors: Colors::default(),
                        fonts: BTreeMap::new(),
                        geometry: Geometry::default(),
                    },
                    NamedVariant {
                        id: "fumadocs-light".into(),
                        label: "Fuma Light".into(),
                        scheme: ThemeMode::Light,
                        colors: Colors::default(),
                        fonts: BTreeMap::new(),
                        geometry: Geometry::default(),
                    },
                ],
                Some(manifest) => {
                    if let Some(modes) = manifest.modes {
                        let defaults = manifest.defaults.unwrap_or_default();
                        let mut dark = defaults.colors.clone();
                        dark.merge(modes.dark.colors);
                        let mut light = defaults.colors.clone();
                        if let Some(light_mode) = modes.light {
                            light.merge(light_mode.colors);
                        }
                        vec![
                            NamedVariant {
                                id: format!("{}-dark", family_id),
                                label: format!("{} Dark", family_id),
                                scheme: ThemeMode::Dark,
                                colors: dark,
                                fonts: defaults.fonts.clone(),
                                geometry: defaults.geometry.clone(),
                            },
                            NamedVariant {
                                id: format!("{}-light", family_id),
                                label: format!("{} Light", family_id),
                                scheme: ThemeMode::Light,
                                colors: light,
                                fonts: defaults.fonts,
                                geometry: defaults.geometry,
                            },
                        ]
                    } else {
                        let defaults = manifest.defaults.unwrap_or_default();
                        manifest
                            .variants
                            .unwrap_or_default()
                            .into_iter()
                            .map(|mut v| {
                                let mut colors = defaults.colors.clone();
                                colors.merge(v.colors);
                                v.colors = colors;
                                let mut fonts = defaults.fonts.clone();
                                fonts.extend(v.fonts);
                                v.fonts = fonts;
                                v.geometry = merge_geometry(defaults.geometry.clone(), v.geometry);
                                v
                            })
                            .collect()
                    }
                }
            };
            for variant in variants {
                validate_theme_id(&variant.id)?;
                if profiles.contains_key(&variant.id) {
                    return Err(Error::Invalid(
                        "duplicate global theme variant id".to_owned(),
                    ));
                }
                profiles.insert(variant.id.clone(), (family_id.clone(), variant.clone()));
                ordered_profiles.push((family_id.clone(), variant));
            }
        }
        let (dark_family, dark) = profiles.get(&deploy.dark).ok_or_else(|| {
            Error::Invalid(format!("unknown deploy dark variant {:?}", deploy.dark))
        })?;
        let (light_family, light) = profiles.get(&deploy.light).ok_or_else(|| {
            Error::Invalid(format!("unknown deploy light variant {:?}", deploy.light))
        })?;
        if dark.scheme != ThemeMode::Dark || light.scheme != ThemeMode::Light {
            return Err(Error::Invalid(
                "deploy pair must reference a dark variant and a light variant".to_owned(),
            ));
        }
        let demo_default_id = self.demo_default.as_deref().unwrap_or(&deploy.dark);
        let (demo_family, demo_profile) = profiles.get(demo_default_id).ok_or_else(|| {
            Error::Invalid(format!(
                "unknown theme.demo_default variant {demo_default_id:?}"
            ))
        })?;
        let presentation = presentation_override.unwrap_or(self.presentation);
        let selected_profiles: Vec<(&String, &NamedVariant)> = match presentation {
            ThemePresentation::Deploy => vec![(dark_family, dark), (light_family, light)],
            ThemePresentation::Demo => ordered_profiles
                .iter()
                .map(|(family, profile)| (family, profile))
                .collect(),
        };
        let mut files = Vec::new();
        let mut selected_families = BTreeSet::new();
        for (family, _) in selected_profiles.iter().copied() {
            if !selected_families.insert(family.clone()) {
                continue;
            }
            if let Some((stylesheet, assets)) = family_files.remove(family) {
                if let Some(contents) = stylesheet {
                    files.push(ThemeFile {
                        path: PathBuf::from(format!("themes/{family}/styles.css")),
                        contents,
                    });
                }
                for (target, contents) in assets {
                    validate_target(&target)?;
                    files.push(ThemeFile {
                        path: PathBuf::from(format!(
                            "themes/{family}/{}",
                            target.to_string_lossy()
                        )),
                        contents,
                    });
                }
            }
        }
        let has_family = |profile: &NamedVariant, family: &str| {
            profile.fonts.values().any(|stack| {
                stack
                    .families
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(family))
            })
        };
        let need_geist = selected_profiles
            .iter()
            .any(|(_, profile)| !profile.fonts.contains_key(&FontRole::Body))
            || selected_profiles
                .iter()
                .any(|(_, profile)| has_family(profile, "Geist"));
        let need_jetbrains = selected_profiles
            .iter()
            .any(|(_, profile)| !profile.fonts.contains_key(&FontRole::Code))
            || selected_profiles
                .iter()
                .any(|(_, profile)| has_family(profile, "JetBrains Mono"));
        let mut font_css = String::new();
        if need_geist {
            font_css.push_str(GEIST_FONT_FACE);
        }
        if need_jetbrains {
            font_css.push_str(JETBRAINS_FONT_FACE);
        }
        files.push(ThemeFile {
            path: "css/akibiyori-fonts.css".into(),
            contents: font_css.into_bytes(),
        });
        let mut bundled_paths = vec!["css/akibiyori-fonts.css".to_owned()];
        if need_geist {
            files.push(ThemeFile {
                path: "fonts/geist-latin.woff2".into(),
                contents: include_bytes!("../theme/vendor/fonts/geist-latin.woff2").to_vec(),
            });
            bundled_paths.push("fonts/geist-latin.woff2".into());
        }
        if need_jetbrains {
            files.push(ThemeFile {
                path: "fonts/jetbrains-mono-latin.woff2".into(),
                contents: include_bytes!("../theme/vendor/fonts/jetbrains-mono-latin.woff2")
                    .to_vec(),
            });
            bundled_paths.push("fonts/jetbrains-mono-latin.woff2".into());
        }
        let mut css = String::new();
        let mut used_faces: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (family, profile) in selected_profiles.iter().copied() {
            for stack in profile.fonts.values() {
                if let Some(face) = &stack.face {
                    used_faces
                        .entry(family.clone())
                        .or_default()
                        .insert(face.clone());
                }
            }
        }
        for (family, ids) in &used_faces {
            if let Some(records) = family_faces.get(family) {
                for record in records.iter().filter(|record| ids.contains(&record.id)) {
                    css.push_str(&compile_font_face(family, record));
                    let path = format!("themes/{family}/fonts/{}", record.filename());
                    files.push(ThemeFile {
                        path: path.into(),
                        contents: record.bytes.clone(),
                    });
                }
            }
        }
        for (family, profile) in selected_profiles.iter().copied() {
            css.push_str(&compile_profile(family, profile));
        }
        let mut public_paths = vec![
            "css/akibiyori-theme.css".to_owned(),
            "_data/akibiyori_theme.json".to_owned(),
        ];
        for file in &files {
            public_paths.push(file.path.to_string_lossy().into_owned());
        }
        files.push(ThemeFile {
            path: "css/akibiyori-theme.css".into(),
            contents: css.into_bytes(),
        });
        public_paths.sort();
        let (current_family, current_profile, variant_id) = match presentation {
            ThemePresentation::Deploy => {
                if self.default_mode.effective_mode() == ThemeMode::Dark {
                    (dark_family, dark, deploy.dark.clone())
                } else {
                    (light_family, light, deploy.light.clone())
                }
            }
            ThemePresentation::Demo => (demo_family, demo_profile, demo_default_id.to_owned()),
        };
        let mode = current_profile.scheme;
        let profiles = selected_profiles
            .iter()
            .map(|(family, profile)| ResolvedProfileMetadata {
                id: profile.id.clone(),
                label: profile.label.clone(),
                family: (*family).clone(),
                mode: profile.scheme,
                author_stylesheet_href: stylesheet_families
                    .contains(*family)
                    .then(|| format!("{base_path}/themes/{family}/styles.css")),
            })
            .collect();
        let author_stylesheet_href = stylesheet_families
            .contains(current_family)
            .then(|| format!("{base_path}/themes/{current_family}/styles.css"));
        Ok(ResolvedTheme {
            id: current_family.clone(),
            mode,
            variant_id,
            dark_variant: deploy.dark.clone(),
            light_variant: deploy.light.clone(),
            dark_family: dark_family.clone(),
            light_family: light_family.clone(),
            profiles,
            default_mode: self.default_mode,
            presentation,
            demo_default: demo_default_id.to_owned(),
            preference_key: format!(
                "akibiyori:mode:v1:{}",
                if base_path.is_empty() { "/" } else { base_path }
            ),
            theme_key: format!(
                "akibiyori:theme:v1:{}",
                if base_path.is_empty() { "/" } else { base_path }
            ),
            stylesheet_href: format!("{base_path}/css/akibiyori-theme.css"),
            author_stylesheet_href,
            files,
            public_paths,
        })
    }
}

const GEIST_FONT_FACE: &str = "@font-face {\n  font-family: Geist;\n  font-style: normal;\n  font-weight: 100 900;\n  font-display: swap;\n  src: url(\"../fonts/geist-latin.woff2\") format(\"woff2\");\n  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;\n}\n@font-face {\n  font-family: \"Geist Fallback\";\n  src: local(Arial);\n  ascent-override: 95.94%;\n  descent-override: 28.16%;\n  line-gap-override: 0%;\n  size-adjust: 104.76%;\n}\n";
const JETBRAINS_FONT_FACE: &str = "@font-face {\n  font-family: \"JetBrains Mono\";\n  font-style: normal;\n  font-weight: 100 800;\n  font-display: swap;\n  src: url(\"../fonts/jetbrains-mono-latin.woff2\") format(\"woff2\");\n  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;\n}\n@font-face {\n  font-family: \"JetBrains Mono Fallback\";\n  src: local(Arial);\n  ascent-override: 75.79%;\n  descent-override: 22.29%;\n  line-gap-override: 0%;\n  size-adjust: 134.59%;\n}\n";
fn load_catalog_family(root: &Path, source: &ThemeSource) -> Result<LoadedThemeFamily, Error> {
    match source {
        ThemeSource::Builtin(name) if name == "fumadocs" => {
            Ok(("fumadocs".into(), None, None, Vec::new(), Vec::new()))
        }
        ThemeSource::Builtin(name) if name == "neutral-example" => load_catalog_manifest(
            root,
            Path::new("examples/observatory/themes/neutral-example/theme.json"),
            true,
        ),
        ThemeSource::Builtin(name) => Err(Error::Invalid(format!(
            "unsupported built-in theme {name:?}"
        ))),
        ThemeSource::File(path) => load_catalog_manifest(root, path, false),
    }
}
fn load_catalog_manifest(
    root: &Path,
    path: &Path,
    bundled: bool,
) -> Result<LoadedThemeFamily, Error> {
    let bytes = if bundled {
        include_bytes!("../examples/observatory/themes/neutral-example/theme.json").to_vec()
    } else {
        read_icon_input(root, path, "theme manifest")?
    };
    let manifest: ThemeManifest = serde_json::from_slice(&bytes).map_err(|source| {
        if bundled {
            Error::Invalid(format!("invalid bundled theme manifest: {source}"))
        } else {
            Error::Json {
                path: root.join(path),
                source,
            }
        }
    })?;
    manifest.validate()?;
    let id = manifest.id.clone();
    let parent = if bundled {
        Path::new("examples/observatory/themes/neutral-example")
    } else {
        path.parent().unwrap_or_else(|| Path::new(""))
    };
    let css = manifest
        .stylesheet
        .as_deref()
        .map(|p| {
            if bundled {
                Ok(
                    include_bytes!("../examples/observatory/themes/neutral-example/overrides.css")
                        .to_vec(),
                )
            } else {
                read_icon_input(root, &parent.join(p), "theme stylesheet")
            }
        })
        .transpose()?;
    let mut assets = Vec::new();
    for asset in &manifest.assets {
        assets.push((
            asset.target.clone(),
            if bundled {
                return Err(Error::Invalid(
                    "bundled theme assets are not supported".into(),
                ));
            } else {
                read_icon_input(root, &parent.join(&asset.source), "theme asset")?
            },
        ));
    }
    let mut faces = Vec::new();
    for face in &manifest.font_faces {
        let source_bytes = read_icon_input(root, &parent.join(&face.source), "font face")?;
        let extension = face
            .source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        faces.push(FontRecord {
            id: face.id.clone(),
            style: face.style.clone(),
            weight: face.weight,
            extension,
            bytes: source_bytes,
        });
    }
    Ok((id, Some(manifest), css, assets, faces))
}
fn compile_font_face(family: &str, face: &FontRecord) -> String {
    let family_name = css_family(family, &face.id);
    let path = format!("../themes/{family}/fonts/{}", face.filename());
    let format = match face.extension.as_str() {
        "woff2" => "woff2",
        "woff" => "woff",
        "ttf" => "truetype",
        "otf" => "opentype",
        _ => "woff2",
    };
    format!(
        "@font-face {{\n  font-family: \"{family_name}\";\n  font-style: {};\n  font-weight: {} {};\n  font-display: swap;\n  src: url(\"{path}\") format(\"{format}\");\n}}\n",
        face.style, face.weight[0], face.weight[1]
    )
}
fn compile_profile(family: &str, profile: &NamedVariant) -> String {
    let mut out = String::new();
    if profile.scheme == ThemeMode::Light {
        out.push_str(&format!("html[data-aki-theme=\"{family}\"][data-aki-variant=\"{}\"][data-aki-mode=\"light\"] {{ color-scheme: light; }}\n",profile.id));
    }
    out.push_str(&format!("html[data-aki-theme=\"{family}\"][data-aki-variant=\"{}\"][data-aki-mode=\"{}\"] body.framework,\nhtml[data-aki-theme=\"{family}\"][data-aki-variant=\"{}\"][data-aki-mode=\"{}\"] #nd-sidebar {{\n",profile.id,profile.scheme.as_str(),profile.id,profile.scheme.as_str()));
    if profile.scheme == ThemeMode::Light {
        for (name, value) in LIGHT_FUMADOCS_INPUTS {
            out.push_str(&format!("  {name}: {value};\n"));
        }
    }
    for (role, color) in &profile.colors.0 {
        if let Some(fd) = role.fd_name() {
            out.push_str(&format!("  {fd}: {};\n", color.0));
        }
        out.push_str(&format!("  {}: {};\n", role.as_str(), color.0));
    }
    for (role, stack) in &profile.fonts {
        let name = match role {
            FontRole::Body => "body",
            FontRole::Heading => "heading",
            FontRole::Code => "code",
        };
        let mut entries = Vec::new();
        if let Some(face) = &stack.face {
            entries.push(css_family(family, face));
        }
        entries.extend(
            stack
                .families
                .iter()
                .map(|value| format!("\"{}\"", css_escape(value))),
        );
        entries.push(stack.generic.as_css().to_owned());
        out.push_str(&format!("  --aki-font-{name}: {};\n", entries.join(", ")));
    }
    if let Some(width) = &profile.geometry.border_width {
        out.push_str(&format!(
            "  --aki-border-default: {};\n",
            format_length(width)
        ));
    }
    for (name, length) in &profile.geometry.radii {
        out.push_str(&format!(
            "  --aki-radius-{name}: {};\n",
            format_length(length)
        ));
    }
    for (name, shadow) in &profile.geometry.shadows {
        let color = shadow
            .color
            .as_ref()
            .map(|c| c.0.as_str())
            .unwrap_or(match name.as_str() {
                "menu" => "var(--aki-color-overlay-menu-shadow)",
                "sidebar" => "var(--aki-color-overlay-sidebar-shadow)",
                _ => "var(--aki-color-overlay-dialog-shadow)",
            });
        out.push_str(&format!(
            "  --aki-shadow-{name}: {} {} {} {} {};\n",
            format_length(&shadow.x),
            format_length(&shadow.y),
            format_length(&shadow.blur),
            format_length(&shadow.spread),
            color
        ));
    }
    out.push_str("}\n");
    out
}
fn css_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}
fn css_family(family: &str, face: &str) -> String {
    format!("aki-{}-{family}-{face}", family.len())
}
fn format_length(value: &Length) -> String {
    format!("{}{}", value.value, value.unit)
}
fn merge_geometry(mut base: Geometry, over: Geometry) -> Geometry {
    if over.border_width.is_some() {
        base.border_width = over.border_width;
    }
    base.radii.extend(over.radii);
    base.shadows.extend(over.shadows);
    base
}

fn validate_source(source: &ThemeSource) -> Result<(), Error> {
    match source {
        ThemeSource::Builtin(name) if matches!(name.as_str(), "fumadocs" | "neutral-example") => {
            Ok(())
        }
        ThemeSource::Builtin(name) => Err(Error::Invalid(format!(
            "unsupported built-in theme {name:?}; expected \"fumadocs\" or \"neutral-example\""
        ))),
        ThemeSource::File(path) => validate_source_file(path, "theme manifest", &["json"]),
    }
}

#[derive(Debug)]
pub(crate) struct ThemeFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct ResolvedTheme {
    pub id: String,
    mode: ThemeMode,
    variant_id: String,
    dark_variant: String,
    light_variant: String,
    dark_family: String,
    light_family: String,
    profiles: Vec<ResolvedProfileMetadata>,
    default_mode: ThemePreference,
    presentation: ThemePresentation,
    demo_default: String,
    preference_key: String,
    theme_key: String,
    pub stylesheet_href: String,
    pub author_stylesheet_href: Option<String>,
    pub files: Vec<ThemeFile>,
    pub public_paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolvedProfileMetadata {
    id: String,
    label: String,
    family: String,
    mode: ThemeMode,
    author_stylesheet_href: Option<String>,
}

impl ResolvedTheme {
    pub(crate) fn data_json(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "id": self.id,
            "variant_id": self.variant_id,
            "dark_variant": self.dark_variant,
            "light_variant": self.light_variant,
            "dark_family": self.dark_family,
            "light_family": self.light_family,
            "profiles": self.profiles.iter().map(|profile| serde_json::json!({"id":profile.id,"label":profile.label,"family":profile.family,"mode":profile.mode.as_str(),"author_stylesheet_href":profile.author_stylesheet_href})).collect::<Vec<_>>(),
            "mode": self.mode.as_str(),
            "default_mode": self.default_mode.as_str(),
            "presentation": self.presentation.as_str(),
            "demo_default": self.demo_default,
            "preference_key": self.preference_key,
            "theme_key": self.theme_key,
            "stylesheet_href": self.stylesheet_href,
            "author_stylesheet_href": self.author_stylesheet_href,
        }))
        .map_err(|source| Error::Invalid(format!("failed to serialize theme data: {source}")))
    }
}

impl ThemeMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeManifest {
    version: u32,
    id: String,
    extends: String,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    modes: Option<ThemeModes>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    variants: Option<Vec<NamedVariant>>,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    defaults: Option<ThemeDefaults>,
    #[serde(default)]
    font_faces: Vec<FontFace>,
    #[serde(default, deserialize_with = "deserialize_optional_path")]
    stylesheet: Option<PathBuf>,
    #[serde(default)]
    assets: Vec<ThemeAsset>,
}

fn deserialize_optional_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    PathBuf::deserialize(deserializer).map(Some)
}

impl ThemeManifest {
    fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid(format!(
                "unsupported theme manifest version {}; expected 1",
                self.version
            )));
        }
        if !matches!(self.extends.as_str(), "fumadocs" | "default") {
            return Err(Error::Invalid(format!(
                "unsupported theme base {:?}; expected \"fumadocs\" or \"default\"",
                self.extends
            )));
        }
        if self.modes.is_some() == self.variants.is_some() {
            return Err(Error::Invalid(
                "theme manifest must contain exactly one of modes or variants".to_owned(),
            ));
        }
        if let Some(variants) = &self.variants {
            if variants.is_empty() {
                return Err(Error::Invalid(
                    "theme variants must not be empty".to_owned(),
                ));
            }
            let mut ids = BTreeSet::new();
            for variant in variants {
                validate_theme_id(&variant.id)?;
                if variant.label.trim().is_empty() || variant.label.chars().any(char::is_control) {
                    return Err(Error::Invalid(format!(
                        "invalid label for variant {:?}",
                        variant.id
                    )));
                }
                if !ids.insert(&variant.id) {
                    return Err(Error::Invalid(format!(
                        "duplicate variant id {:?}",
                        variant.id
                    )));
                }
                validate_profile_fields(&variant.fonts, &variant.geometry)?;
            }
        }
        if let Some(defaults) = &self.defaults {
            validate_profile_fields(&defaults.fonts, &defaults.geometry)?;
        }
        let face_ids: BTreeSet<&str> = self
            .font_faces
            .iter()
            .map(|face| face.id.as_str())
            .collect();
        let validate_faces = |fonts: &BTreeMap<FontRole, FontStack>| -> Result<(), Error> {
            for stack in fonts.values() {
                if let Some(face) = &stack.face
                    && !face_ids.contains(face.as_str())
                {
                    return Err(Error::Invalid(format!(
                        "font stack references undeclared face family {face:?}"
                    )));
                }
            }
            Ok(())
        };
        if let Some(defaults) = &self.defaults {
            validate_faces(&defaults.fonts)?;
        }
        if let Some(variants) = &self.variants {
            for variant in variants {
                validate_faces(&variant.fonts)?;
            }
        }
        validate_theme_id(&self.id)?;
        let mut face_groups: BTreeMap<(&str, &str), Vec<(u16, u16)>> = BTreeMap::new();
        for face in &self.font_faces {
            validate_theme_id(&face.id)?;
            validate_source_file(&face.source, "font face", &["woff2", "woff", "ttf", "otf"])?;
            if !matches!(face.style.as_str(), "normal" | "italic")
                || face.weight[0] == 0
                || face.weight[1] > 1000
                || face.weight[0] > face.weight[1]
            {
                return Err(Error::Invalid(format!(
                    "invalid font face range/style for {:?}",
                    face.id
                )));
            }
            let ranges = face_groups.entry((&face.id, &face.style)).or_default();
            if ranges
                .iter()
                .any(|(a, b)| face.weight[0] <= *b && *a <= face.weight[1])
            {
                return Err(Error::Invalid(format!(
                    "overlapping font face range for {:?}",
                    face.id
                )));
            }
            ranges.push((face.weight[0], face.weight[1]));
        }
        if let Some(path) = &self.stylesheet {
            validate_source_file(path, "theme stylesheet", &["css"])?;
        }
        let mut targets = BTreeSet::new();
        for asset in &self.assets {
            validate_source_file(&asset.source, "theme asset", ASSET_EXTENSIONS)?;
            validate_target(&asset.target)?;
            if asset
                .target
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("styles.css"))
            {
                return Err(Error::Invalid(
                    "theme asset target styles.css is reserved for the optional stylesheet"
                        .to_owned(),
                ));
            }
            let folded = asset.target.to_string_lossy().to_ascii_lowercase();
            if !targets.insert(folded.clone()) {
                return Err(Error::Invalid(format!(
                    "duplicate theme asset target {:?}",
                    asset.target
                )));
            }
            if targets.iter().any(|candidate| {
                candidate != &folded
                    && (candidate.starts_with(&(folded.clone() + "/"))
                        || folded.starts_with(&(candidate.clone() + "/")))
            }) {
                return Err(Error::Invalid(format!(
                    "theme targets contain a file/directory collision at {:?}",
                    asset.target
                )));
            }
        }
        for face in &self.font_faces {
            let extension = face
                .source
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let generated = PathBuf::from(format!(
                "fonts/{}-{}-{}-{}.{}",
                face.id, face.style, face.weight[0], face.weight[1], extension
            ));
            let folded = generated.to_string_lossy().to_ascii_lowercase();
            if !targets.insert(folded.clone())
                || targets.iter().any(|candidate| {
                    candidate != &folded
                        && (candidate.starts_with(&(folded.clone() + "/"))
                            || folded.starts_with(&(candidate.clone() + "/")))
                })
            {
                return Err(Error::Invalid(format!(
                    "font face target collision at {generated:?}"
                )));
            }
        }
        Ok(())
    }
}

fn deserialize_unique_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + fmt::Debug,
    V: Deserialize<'de>,
{
    struct UniqueMapVisitor<K, V>(std::marker::PhantomData<(K, V)>);
    impl<'de, K, V> Visitor<'de> for UniqueMapVisitor<K, V>
    where
        K: Deserialize<'de> + Ord + fmt::Debug,
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<K, V>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a map with unique keys")
        }
        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<K, V>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate map key"));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueMapVisitor(std::marker::PhantomData))
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeDefaults {
    #[serde(default)]
    colors: Colors,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    fonts: BTreeMap<FontRole, FontStack>,
    #[serde(default)]
    geometry: Geometry,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedVariant {
    id: String,
    label: String,
    scheme: ThemeMode,
    colors: Colors,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    fonts: BTreeMap<FontRole, FontStack>,
    #[serde(default)]
    geometry: Geometry,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum FontRole {
    Body,
    Heading,
    Code,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontStack {
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    face: Option<String>,
    #[serde(default)]
    families: Vec<String>,
    generic: GenericFont,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum GenericFont {
    SansSerif,
    Serif,
    Monospace,
    SystemUi,
}
impl GenericFont {
    fn as_css(&self) -> &'static str {
        match self {
            Self::SansSerif => "sans-serif",
            Self::Serif => "serif",
            Self::Monospace => "monospace",
            Self::SystemUi => "system-ui",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Geometry {
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    border_width: Option<Length>,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    radii: BTreeMap<String, Length>,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    shadows: BTreeMap<String, Shadow>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Length {
    value: f64,
    unit: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Shadow {
    x: Length,
    y: Length,
    blur: Length,
    spread: Length,
    #[serde(default, deserialize_with = "deserialize_nonnull_option")]
    color: Option<HexColor>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontFace {
    id: String,
    source: PathBuf,
    weight: [u16; 2],
    style: String,
}
#[derive(Debug)]
struct FontRecord {
    id: String,
    style: String,
    weight: [u16; 2],
    extension: String,
    bytes: Vec<u8>,
}
type ThemeAssetBytes = Vec<(PathBuf, Vec<u8>)>;
type ThemeFamilyFiles = (Option<Vec<u8>>, ThemeAssetBytes);
type LoadedThemeFamily = (
    String,
    Option<ThemeManifest>,
    Option<Vec<u8>>,
    ThemeAssetBytes,
    Vec<FontRecord>,
);
impl FontRecord {
    fn filename(&self) -> String {
        format!(
            "{}-{}-{}-{}.{}",
            self.id, self.style, self.weight[0], self.weight[1], self.extension
        )
    }
}

fn validate_profile_fields(
    fonts: &BTreeMap<FontRole, FontStack>,
    geometry: &Geometry,
) -> Result<(), Error> {
    for (role, stack) in fonts {
        if stack
            .families
            .iter()
            .any(|f| f.is_empty() || f.chars().any(char::is_control))
            || (stack.families.is_empty() && stack.face.is_none())
        {
            return Err(Error::Invalid(format!("invalid font stack for {role:?}")));
        }
        if let Some(face) = &stack.face {
            validate_theme_id(face)?;
        }
    }
    if let Some(width) = &geometry.border_width {
        validate_length(width, false, "border width")?;
    }
    const RADII: [&str; 10] = [
        "panel",
        "navigation",
        "selector-menu",
        "endpoint",
        "example-card",
        "schema-card",
        "small-control",
        "schema-chip",
        "copy-code",
        "callback-dialog",
    ];
    for (key, value) in &geometry.radii {
        if !RADII.contains(&key.as_str()) {
            return Err(Error::Invalid(format!("unknown geometry radius {key:?}")));
        }
        validate_length(value, false, "radius")?;
    }
    const SHADOWS: [&str; 3] = ["menu", "sidebar", "dialog"];
    for (key, shadow) in &geometry.shadows {
        if !SHADOWS.contains(&key.as_str()) {
            return Err(Error::Invalid(format!("unknown geometry shadow {key:?}")));
        }
        validate_length(&shadow.x, true, "shadow x")?;
        validate_length(&shadow.y, true, "shadow y")?;
        validate_length(&shadow.blur, false, "shadow blur")?;
        validate_length(&shadow.spread, true, "shadow spread")?;
    }
    Ok(())
}
fn validate_length(v: &Length, signed: bool, label: &str) -> Result<(), Error> {
    if !v.value.is_finite()
        || (!signed && v.value < 0.0)
        || !matches!(v.unit.as_str(), "px" | "rem")
    {
        return Err(Error::Invalid(format!("invalid {label} length")));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeModes {
    dark: DarkMode,
    #[serde(default, deserialize_with = "deserialize_optional_light")]
    light: Option<LightMode>,
}

fn deserialize_optional_light<'de, D>(deserializer: D) -> Result<Option<LightMode>, D::Error>
where
    D: Deserializer<'de>,
{
    LightMode::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DarkMode {
    colors: Colors,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LightMode {
    colors: Colors,
}

#[derive(Debug, Default, Clone)]
struct Colors(BTreeMap<ColorRole, HexColor>);

impl Colors {
    fn merge(&mut self, overrides: Colors) {
        self.0.extend(overrides.0);
    }
}

impl<'de> Deserialize<'de> for Colors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorsVisitor;

        impl<'de> Visitor<'de> for ColorsVisitor {
            type Value = Colors;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map of supported Akibiyori colour roles to hex colours")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut colors = BTreeMap::new();
                while let Some((name, value)) = map.next_entry::<String, HexColor>()? {
                    let role = ColorRole::parse(&name).ok_or_else(|| {
                        serde::de::Error::custom(format!("unknown theme colour role {name:?}"))
                    })?;
                    if colors.insert(role, value).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate theme colour role {name:?}"
                        )));
                    }
                }
                Ok(Colors(colors))
            }
        }

        deserializer.deserialize_map(ColorsVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ColorRole {
    Text,
    TextMuted,
    SurfacePage,
    SurfacePanel,
    TextPanel,
    SurfaceRaised,
    SurfaceSecondary,
    Border,
    FocusRing,
    Action,
    Hover,
    TextHover,
    MethodGet,
    MethodPost,
    MethodPut,
    MethodPatch,
    MethodDelete,
    PropertyRequired,
    ActionTint,
    HoverTint,
    SecondaryTint,
    TocActiveTint,
    OverlayMenuShadow,
    OverlaySidebarShadow,
    OverlayDialogShadow,
    OverlayDialogBackdrop,
    CodeArticleForeground,
    CodeArticleString,
    CodeArticlePunctuation,
    CodeArticleKeyword,
    CodeArticleName,
    CodeArticleConstant,
    CodeArticleOperator,
    CodeApiForeground,
    CodeApiPunctuation,
    CodeApiKeyNumberOption,
    CodeApiStringValue,
    SurfaceFrame,
    CodeApiCommandOption,
    CodeApiDelimiter,
}

impl ColorRole {
    const ALL: [Self; 40] = [
        Self::Text,
        Self::TextMuted,
        Self::SurfacePage,
        Self::SurfacePanel,
        Self::TextPanel,
        Self::SurfaceRaised,
        Self::SurfaceSecondary,
        Self::Border,
        Self::FocusRing,
        Self::Action,
        Self::Hover,
        Self::TextHover,
        Self::MethodGet,
        Self::MethodPost,
        Self::MethodPut,
        Self::MethodPatch,
        Self::MethodDelete,
        Self::PropertyRequired,
        Self::ActionTint,
        Self::HoverTint,
        Self::SecondaryTint,
        Self::TocActiveTint,
        Self::OverlayMenuShadow,
        Self::OverlaySidebarShadow,
        Self::OverlayDialogShadow,
        Self::OverlayDialogBackdrop,
        Self::CodeArticleForeground,
        Self::CodeArticleString,
        Self::CodeArticlePunctuation,
        Self::CodeArticleKeyword,
        Self::CodeArticleName,
        Self::CodeArticleConstant,
        Self::CodeArticleOperator,
        Self::CodeApiForeground,
        Self::CodeApiPunctuation,
        Self::CodeApiKeyNumberOption,
        Self::CodeApiStringValue,
        Self::SurfaceFrame,
        Self::CodeApiCommandOption,
        Self::CodeApiDelimiter,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "--aki-color-text",
            Self::TextMuted => "--aki-color-text-muted",
            Self::SurfacePage => "--aki-color-surface-page",
            Self::SurfacePanel => "--aki-color-surface-panel",
            Self::TextPanel => "--aki-color-text-panel",
            Self::SurfaceRaised => "--aki-color-surface-raised",
            Self::SurfaceSecondary => "--aki-color-surface-secondary",
            Self::Border => "--aki-color-border",
            Self::FocusRing => "--aki-color-focus-ring",
            Self::Action => "--aki-color-action",
            Self::Hover => "--aki-color-hover",
            Self::TextHover => "--aki-color-text-hover",
            Self::MethodGet => "--aki-color-method-get",
            Self::MethodPost => "--aki-color-method-post",
            Self::MethodPut => "--aki-color-method-put",
            Self::MethodPatch => "--aki-color-method-patch",
            Self::MethodDelete => "--aki-color-method-delete",
            Self::PropertyRequired => "--aki-color-property-required",
            Self::ActionTint => "--aki-color-action-tint",
            Self::HoverTint => "--aki-color-hover-tint",
            Self::SecondaryTint => "--aki-color-secondary-tint",
            Self::TocActiveTint => "--aki-color-toc-active-tint",
            Self::OverlayMenuShadow => "--aki-color-overlay-menu-shadow",
            Self::OverlaySidebarShadow => "--aki-color-overlay-sidebar-shadow",
            Self::OverlayDialogShadow => "--aki-color-overlay-dialog-shadow",
            Self::OverlayDialogBackdrop => "--aki-color-overlay-dialog-backdrop",
            Self::CodeArticleForeground => "--aki-color-code-article-foreground",
            Self::CodeArticleString => "--aki-color-code-article-string",
            Self::CodeArticlePunctuation => "--aki-color-code-article-punctuation",
            Self::CodeArticleKeyword => "--aki-color-code-article-keyword",
            Self::CodeArticleName => "--aki-color-code-article-name",
            Self::CodeArticleConstant => "--aki-color-code-article-constant",
            Self::CodeArticleOperator => "--aki-color-code-article-operator",
            Self::CodeApiForeground => "--aki-color-code-api-foreground",
            Self::CodeApiPunctuation => "--aki-color-code-api-punctuation",
            Self::CodeApiKeyNumberOption => "--aki-color-code-api-key-number-option",
            Self::CodeApiStringValue => "--aki-color-code-api-string-value",
            Self::SurfaceFrame => "--aki-color-surface-frame",
            Self::CodeApiCommandOption => "--aki-color-code-api-command-option",
            Self::CodeApiDelimiter => "--aki-color-code-api-delimiter",
        }
    }

    fn fd_name(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("--color-fd-foreground"),
            Self::TextMuted => Some("--color-fd-muted-foreground"),
            Self::SurfacePage => Some("--color-fd-background"),
            Self::SurfacePanel => Some("--color-fd-card"),
            Self::TextPanel => Some("--color-fd-card-foreground"),
            Self::SurfaceRaised => Some("--color-fd-popover"),
            Self::SurfaceSecondary => Some("--color-fd-secondary"),
            Self::Border => Some("--color-fd-border"),
            Self::FocusRing => Some("--color-fd-ring"),
            Self::Action => Some("--color-fd-primary"),
            Self::Hover => Some("--color-fd-accent"),
            Self::TextHover => Some("--color-fd-accent-foreground"),
            _ => None,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == value)
    }
}

#[derive(Debug, Clone)]
struct HexColor(String);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let valid_len = matches!(value.len(), 4 | 5 | 7 | 9);
        if !valid_len
            || !value.starts_with('#')
            || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err(serde::de::Error::custom(
                "theme colours must use #RGB, #RGBA, #RRGGBB, or #RRGGBBAA hex notation",
            ));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeAsset {
    source: PathBuf,
    target: PathBuf,
}

const ASSET_EXTENSIONS: &[&str] = &["woff2", "woff", "ttf", "otf", "txt"];

fn validate_theme_id(value: &str) -> Result<(), Error> {
    let bytes = value.as_bytes();
    let valid = bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if !valid {
        return Err(Error::Invalid(format!(
            "unsafe theme id {value:?}; use lowercase ASCII letters, digits, and hyphens, starting with a letter"
        )));
    }
    Ok(())
}

fn validate_relative(path: &Path, label: &str) -> Result<(), Error> {
    let Some(text) = path.to_str() else {
        return Err(Error::Invalid(format!(
            "{label} path must use UTF-8 path segments"
        )));
    };
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || text.contains('\\')
        || text
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!(
            "{label} path must be a non-empty relative path without dot segments"
        )));
    }
    Ok(())
}

fn validate_source_file(path: &Path, label: &str, extensions: &[&str]) -> Result<(), Error> {
    validate_relative(path, label)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extensions
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
    {
        return Err(Error::Invalid(format!(
            "{label} path must have one of these extensions: {}",
            extensions.join(", ")
        )));
    }
    Ok(())
}

fn validate_target(path: &Path) -> Result<(), Error> {
    validate_relative(path, "theme asset target")?;
    for component in path.components() {
        let Component::Normal(name) = component else {
            unreachable!("relative path validation checked components")
        };
        let Some(segment) = name.to_str() else {
            return Err(Error::Invalid(
                "theme target must use portable UTF-8 segments".to_owned(),
            ));
        };
        validate_target_segment(segment, "theme target segment")?;
    }
    Ok(())
}

fn validate_target_segment(value: &str, label: &str) -> Result<(), Error> {
    let valid = !value.is_empty()
        && !value.starts_with(['.', '_'])
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !valid {
        return Err(Error::Invalid(format!(
            "unsafe {label} {value:?}; use non-hidden ASCII letters, digits, '-', '_', or '.' segments"
        )));
    }
    Ok(())
}

const LIGHT_FUMADOCS_INPUTS: [(&str, &str); 16] = [
    ("--color-fd-background", "#f5f5f5"),
    ("--color-fd-foreground", "#0a0a0a"),
    ("--color-fd-muted", "#f5f5f5"),
    ("--color-fd-muted-foreground", "#737373"),
    ("--color-fd-popover", "#fafafa"),
    ("--color-fd-popover-foreground", "#272727"),
    ("--color-fd-card", "#f1f1f1"),
    ("--color-fd-card-foreground", "#0a0a0a"),
    ("--color-fd-border", "#cccccc80"),
    ("--color-fd-secondary", "#ededed"),
    ("--color-fd-secondary-foreground", "#171717"),
    ("--color-fd-accent", "#d1d1d180"),
    ("--color-fd-accent-foreground", "#171717"),
    ("--color-fd-ring", "#a3a3a3"),
    ("--color-fd-primary", "#cc8b00"),
    ("--color-fd-primary-foreground", "#fafafa"),
];
