use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::{Error, read_icon_input};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThemeSelection {
    #[serde(default)]
    source: ThemeSource,
    #[serde(default, rename = "mode")]
    default_mode: ThemePreference,
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self {
            source: ThemeSource::default(),
            default_mode: ThemePreference::Dark,
        }
    }
}

#[derive(Debug, Deserialize)]
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

impl ThemeSelection {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let ThemeSource::Builtin(name) = &self.source
            && name != "fumadocs"
        {
            return Err(Error::Invalid(format!(
                "unsupported built-in theme {name:?}; expected \"fumadocs\""
            )));
        }
        if let ThemeSource::File(path) = &self.source {
            validate_relative(path, "theme manifest")?;
        }
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        config_root: &Path,
        base_path: &str,
    ) -> Result<ResolvedTheme, Error> {
        self.validate()?;
        let (id, dark_colors, light_colors, stylesheet, assets) = match &self.source {
            ThemeSource::Builtin(_) => (
                "fumadocs".to_owned(),
                Colors::default(),
                Colors::default(),
                None,
                Vec::new(),
            ),
            ThemeSource::File(manifest_path) => {
                validate_source_file(manifest_path, "theme manifest", &["json"])?;
                let manifest_bytes = read_icon_input(config_root, manifest_path, "theme manifest")?;
                let manifest_full_path = config_root.join(manifest_path);
                let manifest: ThemeManifest =
                    serde_json::from_slice(&manifest_bytes).map_err(|source| Error::Json {
                        path: manifest_full_path.clone(),
                        source,
                    })?;
                manifest.validate()?;
                let manifest_parent = manifest_path.parent().unwrap_or_else(|| Path::new(""));
                let dark_colors = manifest.modes.dark.colors.clone();
                let light_colors = manifest
                    .modes
                    .light
                    .as_ref()
                    .map(|light| light.colors.clone())
                    .unwrap_or_default();
                let stylesheet = manifest
                    .stylesheet
                    .as_deref()
                    .map(|path| {
                        validate_source_file(path, "theme stylesheet", &["css"])?;
                        read_icon_input(
                            config_root,
                            &manifest_parent.join(path),
                            "theme stylesheet",
                        )
                    })
                    .transpose()?;
                let mut assets = Vec::with_capacity(manifest.assets.len());
                for asset in manifest.assets {
                    validate_target(&asset.target)?;
                    validate_source_file(&asset.source, "theme asset", ASSET_EXTENSIONS)?;
                    let source = read_icon_input(
                        config_root,
                        &manifest_parent.join(&asset.source),
                        "theme asset",
                    )?;
                    assets.push((asset.target, source));
                }
                (manifest.id, dark_colors, light_colors, stylesheet, assets)
            }
        };

        let stylesheet_href = format!("{base_path}/css/akibiyori-theme.css");
        let author_stylesheet_href = stylesheet
            .as_ref()
            .map(|_| format!("{base_path}/themes/{id}/styles.css"));
        let preference_key = format!(
            "akibiyori:mode:v1:{}",
            if base_path.is_empty() { "/" } else { base_path }
        );
        let effective_mode = self.default_mode.effective_mode();
        let compiled_colors = format!(
            "{}{}",
            compile_colors(&id, ThemeMode::Dark, &dark_colors.0),
            compile_colors(&id, ThemeMode::Light, &light_colors.0),
        );
        let mut files = vec![ThemeFile {
            path: PathBuf::from("css/akibiyori-theme.css"),
            contents: compiled_colors.into_bytes(),
        }];
        let mut public_paths = vec![
            "css/akibiyori-theme.css".to_owned(),
            "_data/akibiyori_theme.json".to_owned(),
        ];
        let mut targets = BTreeSet::new();
        if let Some(stylesheet) = stylesheet {
            let path = format!("themes/{id}/styles.css");
            targets.insert(path.to_ascii_lowercase());
            public_paths.push(path.clone());
            files.push(ThemeFile {
                path: PathBuf::from(path),
                contents: stylesheet,
            });
        }
        for (target, contents) in assets {
            if target
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("styles.css"))
            {
                return Err(Error::Invalid(
                    "theme asset target styles.css is reserved for the optional stylesheet"
                        .to_owned(),
                ));
            }
            let path = format!("themes/{id}/{}", target.to_string_lossy());
            let folded = path.to_ascii_lowercase();
            if !targets.insert(folded.clone()) {
                return Err(Error::Invalid(format!(
                    "duplicate theme asset target {path:?}"
                )));
            }
            if targets.iter().any(|candidate| {
                candidate != &folded
                    && (candidate.starts_with(&(folded.clone() + "/"))
                        || folded.starts_with(&(candidate.clone() + "/")))
            }) {
                return Err(Error::Invalid(format!(
                    "theme targets contain a file/directory collision at {path:?}"
                )));
            }
            public_paths.push(path.clone());
            files.push(ThemeFile {
                path: PathBuf::from(path),
                contents,
            });
        }
        public_paths.sort();
        Ok(ResolvedTheme {
            id,
            mode: effective_mode,
            default_mode: self.default_mode,
            preference_key,
            stylesheet_href,
            author_stylesheet_href,
            files,
            public_paths,
        })
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
    default_mode: ThemePreference,
    preference_key: String,
    pub stylesheet_href: String,
    pub author_stylesheet_href: Option<String>,
    pub files: Vec<ThemeFile>,
    pub public_paths: Vec<String>,
}

impl ResolvedTheme {
    pub(crate) fn data_json(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "id": self.id,
            "mode": self.mode.as_str(),
            "default_mode": self.default_mode.as_str(),
            "preference_key": self.preference_key,
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
    modes: ThemeModes,
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
        if self.extends != "fumadocs" {
            return Err(Error::Invalid(format!(
                "unsupported theme base {:?}; expected \"fumadocs\"",
                self.extends
            )));
        }
        validate_theme_id(&self.id)?;
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
        Ok(())
    }
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

fn compile_colors(id: &str, mode: ThemeMode, colors: &BTreeMap<ColorRole, HexColor>) -> String {
    if mode == ThemeMode::Dark && colors.is_empty() {
        return String::new();
    }
    let mode_name = mode.as_str();
    let mut output = String::new();
    if mode == ThemeMode::Light {
        output.push_str(&format!(
            "html[data-aki-theme=\"{id}\"][data-aki-mode=\"{mode_name}\"] {{ color-scheme: light; }}\n"
        ));
    }
    output.push_str(&format!(
        "html[data-aki-theme=\"{id}\"][data-aki-mode=\"{mode_name}\"] body.framework,\nhtml[data-aki-theme=\"{id}\"][data-aki-mode=\"{mode_name}\"] #nd-sidebar {{\n"
    ));
    if mode == ThemeMode::Light {
        for (name, value) in LIGHT_FUMADOCS_INPUTS {
            output.push_str("  ");
            output.push_str(name);
            output.push_str(": ");
            output.push_str(value);
            output.push_str(";\n");
        }
    }
    for (role, color) in colors {
        if let Some(fd_name) = role.fd_name() {
            output.push_str("  ");
            output.push_str(fd_name);
            output.push_str(": ");
            output.push_str(&color.0);
            output.push_str(";\n");
        }
        output.push_str("  ");
        output.push_str(role.as_str());
        output.push_str(": ");
        output.push_str(&color.0);
        output.push_str(";\n");
    }
    output.push_str("}\n");
    output
}
