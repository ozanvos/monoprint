use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub font: String,
    #[serde(default)]
    pub line_numbers: bool,
    pub syntax: BTreeMap<String, BTreeMap<String, String>>,
    pub background: Option<Gradient>,
    pub editor: Option<EditorTheme>,
    #[serde(default)]
    pub original: bool,
    pub surface: Option<String>,
    pub study_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Gradient {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTheme {
    pub family: String,
    pub surface: String,
    pub foreground: String,
    pub gutter: String,
    pub border: String,
    pub solid: String,
}

/// Curated preset identities, separate from the unchanged imported palettes.
#[derive(Debug, Deserialize)]
pub struct ThemePair {
    pub id: String,
    pub name: String,
    pub dark: String,
    pub light: String,
}

pub fn theme_pairs() -> &'static [ThemePair] {
    static DATA: LazyLock<Vec<ThemePair>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/editor/pairs.json"))
            .expect("bundled editor theme pairs")
    });
    &DATA
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grammar {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

pub fn themes() -> &'static BTreeMap<String, Theme> {
    static DATA: LazyLock<BTreeMap<String, Theme>> = LazyLock::new(|| {
        let mut themes: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/removed-frames/themes.json"))
                .expect("bundled legacy frame theme manifest");
        let editor: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/editor/themes.json"))
                .expect("bundled editor theme manifest");
        for (id, theme) in editor {
            assert!(themes.insert(id, theme).is_none(), "duplicate theme ID");
        }
        let originals: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/originals/themes.json"))
                .expect("bundled Monoprint originals");
        for (id, theme) in originals {
            assert!(themes.insert(id, theme).is_none(), "duplicate theme ID");
        }
        let selected: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/selected/themes.json"))
                .expect("bundled selected Monoprint styles");
        for (id, theme) in selected {
            assert!(themes.insert(id, theme).is_none(), "duplicate theme ID");
        }
        themes
    });
    &DATA
}
pub fn grammars() -> &'static [Grammar] {
    static DATA: LazyLock<Vec<Grammar>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/grammar-metadata.json"))
            .expect("bundled grammar manifest")
    });
    &DATA
}
pub fn templates() -> &'static BTreeMap<String, String> {
    static DATA: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/removed-frames/templates.json"))
            .expect("bundled frame templates")
    });
    &DATA
}
pub fn images() -> &'static BTreeMap<String, String> {
    static DATA: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/removed-frames/images.json"))
            .expect("bundled image assets")
    });
    &DATA
}
pub fn styles() -> &'static BTreeMap<String, String> {
    static DATA: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/removed-frames/styles.json"))
            .expect("bundled frame styles")
    });
    &DATA
}

pub fn font(name: &str) -> &'static [u8] {
    match name {
        "jetbrains-mono" => include_bytes!("../assets/fonts/jetbrains-mono.ttf"),
        "geist-mono" => include_bytes!("../assets/fonts/geist-mono.ttf"),
        "ibm-plex-mono" => include_bytes!("../assets/fonts/ibm-plex-mono.ttf"),
        "fira-code" => include_bytes!("../assets/fonts/fira-code.ttf"),
        "roboto-mono" => include_bytes!("../assets/fonts/roboto-mono.ttf"),
        "space-mono" => include_bytes!("../assets/fonts/space-mono.ttf"),
        "source-code-pro" => include_bytes!("../assets/fonts/source-code-pro.ttf"),
        "google-sans-code" => include_bytes!("../assets/fonts/google-sans-code.ttf"),
        "commit-mono" => include_bytes!("../assets/fonts/commit-mono.otf"),
        "inter" => include_bytes!("../assets/fonts/inter.ttf"),
        _ => unreachable!("unrecognized bundled font"),
    }
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
