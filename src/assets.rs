use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub font: String,
    #[serde(default)]
    pub partner: bool,
    #[serde(default)]
    pub line_numbers: bool,
    #[serde(default)]
    pub line_numbers_toggleable: bool,
    pub syntax: BTreeMap<String, BTreeMap<String, String>>,
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
        serde_json::from_str(include_str!("../assets/removed-frames/themes.json"))
            .expect("bundled theme manifest")
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
