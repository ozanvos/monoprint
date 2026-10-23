use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub font: String,
    pub comment_font: Option<String>,
    pub syntax_weight: Option<u32>,
    #[serde(default)]
    pub line_numbers: bool,
    pub syntax: BTreeMap<String, BTreeMap<String, String>>,
    pub background: Option<Gradient>,
    pub editor: Option<EditorTheme>,
    #[serde(default)]
    pub original: bool,
    pub surface: Option<String>,
    /// Selected and code-only catalogs share their frame and line markup.
    /// Derive routing from the owning catalog, not optional palette metadata.
    #[serde(skip)]
    pub selected_frame: bool,
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
            serde_json::from_str(include_str!("../assets/editor/themes.json"))
                .expect("bundled editor theme manifest");
        let originals: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/originals/themes.json"))
                .expect("bundled Monoprint originals");
        for (id, theme) in originals {
            assert!(themes.insert(id, theme).is_none(), "duplicate theme ID");
        }
        let selected: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/selected/themes.json"))
                .expect("bundled selected Monoprint styles");
        for (id, mut theme) in selected {
            theme.selected_frame = true;
            assert!(themes.insert(id, theme).is_none(), "duplicate theme ID");
        }
        let code_styles: BTreeMap<String, Theme> =
            serde_json::from_str(include_str!("../assets/code-styles/themes.json"))
                .expect("bundled code-only Monoprint styles");
        for (id, mut theme) in code_styles {
            theme.selected_frame = true;
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
