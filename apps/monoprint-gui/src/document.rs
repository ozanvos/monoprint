use anyhow::{Context, Result, bail, ensure};
use monoprint::{
    Appearance, Background, Font, Language, LineNumbers, PngOptions, Renderer, Scale, Snippet,
    Style, Theme,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub const EXAMPLE: &str = "// A little code. A lasting impression.\ntype Theme = { name: string; dark: boolean };\n\nexport function print(theme: Theme): string {\n  const scale = 2;\n  return `${theme.name} at ${scale}×`;\n}\n\nprint({ name: \"Monoprint\", dark: true });";

/// Valid outer widths after padding has been validated (0–512 px).
pub fn width_range(padding: u32) -> std::ops::RangeInclusive<u32> {
    (2 * padding + 49).max(80)..=8192
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    #[default]
    Png,
    Svg,
    Html,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Html => "html",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Svg => "SVG",
            Self::Html => "HTML",
        }
    }
}

/// Serializable editing state. Text fields retain incomplete input until validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    pub source: String,
    pub language: String,
    pub theme: String,
    pub light: bool,
    pub transparent: bool,
    pub background: String,
    pub numbers: bool,
    pub padding: String,
    pub width: String,
    pub title: String,
    pub highlights: String,
    pub text_highlights: String,
    pub font_size: String,
    pub ligatures: bool,
    pub scale: u32,
    pub png_compression: String,
    pub format: Format,
    pub font_path: Option<PathBuf>,
    pub comment_font_path: Option<PathBuf>,
    pub font_id: Option<String>,
    pub comment_font_id: Option<String>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            source: EXAMPLE.into(),
            language: "tsx".into(),
            theme: "removed-frame".into(),
            light: false,
            transparent: false,
            background: "theme".into(),
            numbers: false,
            padding: "64".into(),
            width: "720".into(),
            title: "".into(),
            highlights: "".into(),
            text_highlights: "".into(),
            font_size: "15".into(),
            ligatures: false,
            scale: 2,
            png_compression: "balanced".into(),
            format: Format::Png,
            font_path: None,
            comment_font_path: None,
            font_id: None,
            comment_font_id: None,
        }
    }
}

impl Document {
    pub fn background(&self) -> Result<Background> {
        // Older saved documents use the transparency flag.
        Ok(if self.transparent {
            Background::None
        } else {
            self.background.parse()?
        })
    }
    pub fn select_background(&mut self, background: Background) {
        self.transparent = false;
        self.background = background.to_string();
    }
    pub fn typeface_selection(&self, comments: bool) -> &str {
        let (id, path) = if comments {
            (&self.comment_font_id, &self.comment_font_path)
        } else {
            (&self.font_id, &self.font_path)
        };
        id.as_deref()
            .unwrap_or(if path.is_some() { "custom" } else { "" })
    }

    /// Select a bundled ID, or an empty ID to restore inheritance.
    pub fn select_typeface(&mut self, comments: bool, id: &str) {
        let id = (!id.is_empty()).then(|| id.to_owned());
        if comments {
            self.comment_font_id = id;
            self.comment_font_path = None;
        } else {
            self.font_id = id;
            self.font_path = None;
        }
    }

    pub fn prepare(
        &self,
        font: Option<Font>,
        comment_font: Option<Font>,
    ) -> Result<(Snippet, Style)> {
        ensure!(
            self.source.len() <= monoprint::MAX_SOURCE_BYTES,
            "Code is too large. Use a snippet smaller than 1 MiB."
        );
        let theme: Theme = self.theme.parse()?;
        let appearance = if self.light {
            Appearance::Light
        } else {
            Appearance::Dark
        };
        ensure!(
            theme.supports(appearance),
            "{} does not support {appearance} appearance.",
            theme.name()
        );
        let padding: u32 = self
            .padding
            .trim()
            .parse()
            .context("Padding must be a whole number from 0 to 512.")?;
        ensure!(padding <= 512, "Padding must be between 0 and 512.");
        let font_size: u32 = self
            .font_size
            .trim()
            .parse()
            .context("Font size must be a whole number from 8 to 72 px.")?;
        ensure!(
            (8..=72).contains(&font_size),
            "Font size must be between 8 and 72 px."
        );
        let mut style = Style::new(theme)
            .appearance(appearance)
            .padding(padding)
            .background(self.background()?)
            .font_size(font_size)
            .ligatures(self.ligatures)
            .line_numbers(if self.numbers {
                LineNumbers::Show
            } else {
                LineNumbers::Hide
            });
        if !self.width.trim().is_empty() {
            let width: u32 = self
                .width
                .trim()
                .parse()
                .context("Width must be a whole number, or leave it empty for auto-fit.")?;
            ensure!(
                width_range(padding).contains(&width),
                "Width must be {}–8192 px at this padding.",
                width_range(padding).start()
            );
            style = style.width(width);
        }
        if let Some(font) = font {
            style = style.font(font);
        }
        if let Some(font) = comment_font {
            style = style.comment_font(font);
        }
        let mut snippet = Snippet::new(&self.source).language(self.language.parse::<Language>()?);
        if !self.title.is_empty() {
            snippet = snippet.title(&self.title);
        }
        let lines = self
            .source
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .split('\n')
            .count();
        for (start, end) in highlight_ranges(&self.highlights, lines)? {
            snippet = snippet.highlight(start..=end);
        }
        for range in self
            .text_highlights
            .split(',')
            .filter(|value| !value.trim().is_empty())
        {
            snippet = snippet.highlight_text(range.parse()?);
        }
        Ok((snippet, style))
    }
    pub fn select_theme(&mut self, theme: Theme) {
        self.theme = theme.canonical().id().into();
        if theme != theme.canonical()
            || !theme.supports(if self.light {
                Appearance::Light
            } else {
                Appearance::Dark
            })
        {
            self.light = theme.default_appearance() == Appearance::Light;
        }
    }
    pub fn suggested_name(&self) -> String {
        let name = Path::new(&self.title)
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("monoprint");
        format!("{name}.{}", self.format.extension())
    }
}

pub(crate) fn highlight_ranges(value: &str, lines: usize) -> Result<Vec<(usize, usize)>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|part| {
            let part = part.trim();
            let (first, last) = part.split_once('-').unwrap_or((part, part));
            let first: usize = first
                .trim()
                .parse()
                .context("Use line numbers such as 1, 3-5 for highlights.")?;
            let last: usize = last
                .trim()
                .parse()
                .context("Use line numbers such as 1, 3-5 for highlights.")?;
            ensure!(
                first > 0 && first <= last && last <= lines,
                "Highlight lines must be between 1 and {lines}, in ascending ranges."
            );
            Ok((first, last))
        })
        .collect()
}

pub fn read_source(path: &Path) -> Result<String> {
    let mut source = String::new();
    fs::File::open(path)
        .with_context(|| format!("Could not open {}", path.display()))?
        .take(monoprint::MAX_SOURCE_BYTES as u64 + 1)
        .read_to_string(&mut source)
        .context("Choose a UTF-8 source file.")?;
    ensure!(
        source.len() <= monoprint::MAX_SOURCE_BYTES,
        "Choose a file smaller than 1 MiB."
    );
    Ok(source)
}

#[derive(Default)]
pub struct FontCache {
    current: Option<(PathBuf, std::time::SystemTime, u64, Font)>,
}
impl FontCache {
    pub fn load_selection(
        &mut self,
        id: Option<&str>,
        path: Option<&Path>,
    ) -> Result<Option<Font>> {
        if let Some(id) = id {
            return Ok(Some(id.parse::<monoprint::BundledFont>()?.font()));
        }
        self.load(path)
    }
    pub fn load(&mut self, path: Option<&Path>) -> Result<Option<Font>> {
        let Some(path) = path else { return Ok(None) };
        let metadata = fs::metadata(path)
            .context("Custom font is unavailable. Choose the font again or use Theme font.")?;
        let modified = metadata.modified()?;
        if let Some((cached, time, len, font)) = &self.current
            && cached == path
            && *time == modified
            && *len == metadata.len()
        {
            return Ok(Some(font.clone()));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .context("Custom font is unavailable. Choose the font again or use Theme font.")?
            .take(monoprint::MAX_FONT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let font = Font::from_bytes(bytes)?;
        self.current = Some((path.to_owned(), modified, metadata.len(), font.clone()));
        Ok(Some(font))
    }
}

pub fn export(renderer: &Renderer, document: &Document, format: Format) -> Result<Vec<u8>> {
    let font = FontCache::default()
        .load_selection(document.font_id.as_deref(), document.font_path.as_deref())?;
    let comments = FontCache::default().load_selection(
        document.comment_font_id.as_deref(),
        document.comment_font_path.as_deref(),
    )?;
    let (snippet, style) = document.prepare(font, comments)?;
    Ok(match format {
        Format::Png => renderer.png(
            &snippet,
            &style,
            PngOptions::new(Scale::try_from(document.scale)?)
                .compression(document.png_compression.parse()?),
        )?,
        Format::Svg => renderer.svg(&snippet, &style)?.into_bytes(),
        Format::Html => renderer.html(&snippet, &style)?.into_bytes(),
    })
}

/// A name typed in the native save panel must agree with the selected format.
/// Never silently change an extension and overwrite a different, unapproved path.
pub fn export_path(mut path: PathBuf, format: Format) -> Result<PathBuf> {
    match path.extension().and_then(|s| s.to_str()) {
        None => {
            path.set_extension(format.extension());
            ensure!(
                !path.exists(),
                "That file already exists. Choose its full name in the save dialog to replace it."
            );
        }
        Some(ext) if ext.eq_ignore_ascii_case(format.extension()) => {}
        _ => bail!(
            "Use a .{} filename, or change the export format.",
            format.extension()
        ),
    }
    Ok(path)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Could not save {}", path.display()))?;
    Ok(())
}

pub fn session_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MONOPRINT_SESSION") {
        return (path != "off").then(|| PathBuf::from(path));
    }
    let home = std::env::var_os("HOME")?;
    #[cfg(target_os = "macos")]
    let dir = PathBuf::from(home).join("Library/Application Support/Monoprint");
    #[cfg(target_os = "windows")]
    let dir = PathBuf::from(std::env::var_os("APPDATA").unwrap_or(home)).join("Monoprint");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let dir = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home).join(".local/state"))
        .join("monoprint");
    Some(dir.join("studio.json"))
}

pub fn load_session(path: &Path) -> Result<Document> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Saved session is too large."
    );
    let doc: Document = serde_json::from_slice(&bytes)?;
    doc.theme.parse::<Theme>()?;
    doc.language.parse::<Language>()?;
    Scale::try_from(doc.scale)?;
    ensure!(
        doc.source.len() <= monoprint::MAX_SOURCE_BYTES,
        "Saved code is too large."
    );
    Ok(doc)
}

pub fn save_session(path: &Path, document: &Document) -> Result<()> {
    ensure!(
        document.source.len() <= monoprint::MAX_SOURCE_BYTES,
        "Code exceeds the 1 MiB session limit; the previous session was retained."
    );
    let bytes = serde_json::to_vec(document)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Session exceeds 8 MiB; the previous session was retained."
    );
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    write_atomic(path, &bytes)
}

/// Serializes background saves and the quit flush. An older preview must never
/// replace the final editing snapshot after application shutdown has begun.
pub struct SessionStore {
    path: Option<PathBuf>,
    saved_revision: Mutex<u64>,
}
impl SessionStore {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            saved_revision: Mutex::new(0),
        }
    }
    pub fn save(&self, revision: u64, document: &Document) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let mut saved = self.saved_revision.lock().unwrap();
        if revision <= *saved {
            return Ok(());
        }
        save_session(path, document)?;
        *saved = revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_highlights_without_expanding_unbounded_ranges() {
        assert_eq!(highlight_ranges("1, 3-5", 5).unwrap(), vec![(1, 1), (3, 5)]);
        for bad in ["0", "3-2", "1-999999999999999999", "1,", "six"] {
            assert!(highlight_ranges(bad, 5).is_err(), "{bad}");
        }
    }
    #[test]
    fn theme_switch_and_geometry_are_validated() {
        let mut d = Document {
            light: true,
            ..Default::default()
        };
        d.select_theme("removed-frame".parse().unwrap());
        assert!(!d.light);
        d.width = "150".into();
        assert!(d.prepare(None, None).is_err());
        d.width.clear();
        assert!(d.prepare(None, None).is_ok());
    }

    #[test]
    fn font_size_accepts_exact_pixels_and_retains_incomplete_input() {
        let mut doc = Document::default();
        for value in ["8", "15", "24", "72", " 18 "] {
            doc.font_size = value.into();
            assert!(doc.prepare(None, None).is_ok(), "{value}");
        }
        for value in ["", "7", "73", "-1", "15.5", "large"] {
            doc.font_size = value.into();
            let error = doc.prepare(None, None).err().unwrap();
            assert!(error.to_string().contains("Font size"), "{value}: {error}");
            let restored: Document =
                serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
            assert_eq!(restored.font_size, value);
        }
    }

    #[test]
    fn editor_palettes_backgrounds_and_legacy_sessions() {
        let mut doc = Document {
            transparent: true,
            ..Document::default()
        };
        assert_eq!(doc.background().unwrap(), Background::None);
        doc.select_theme("catppuccin-latte".parse().unwrap());
        assert_eq!(doc.theme, "catppuccin");
        assert!(doc.light);
        for background in Background::ALL {
            doc.select_background(background);
            assert_eq!(doc.background().unwrap(), background);
            assert!(doc.prepare(None, None).is_ok());
            let restored: Document =
                serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
            assert_eq!(restored.background().unwrap(), background);
        }
        doc.select_theme("nord".parse().unwrap());
        assert!(!doc.light);
    }
    #[test]
    fn paired_theme_selection_preserves_appearance_and_legacy_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        for (id, light) in [("ayu-light", true), ("ayu-dark", false)] {
            let mut doc = Document {
                theme: id.into(),
                light,
                ..Default::default()
            };
            save_session(&path, &doc).unwrap();
            doc = load_session(&path).unwrap();
            doc.select_theme(doc.theme.parse().unwrap());
            assert_eq!(doc.theme, "ayu");
            assert_eq!(doc.light, light);
            for light in [false, true, false] {
                doc.light = light;
                assert!(doc.prepare(None, None).is_ok());
                save_session(&path, &doc).unwrap();
                let mut restored = load_session(&path).unwrap();
                restored.select_theme(restored.theme.parse().unwrap());
                assert_eq!(restored.theme, "ayu");
                assert_eq!(restored.light, light);
            }
        }
    }
    #[test]
    fn output_extension_never_redirects_an_approved_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("art.png");
        fs::write(&path, b"keep").unwrap();
        assert!(export_path(dir.path().join("art"), Format::Png).is_err());
        assert!(export_path(path.clone(), Format::Svg).is_err());
        assert_eq!(export_path(path.clone(), Format::Png).unwrap(), path);
        assert_eq!(fs::read(path).unwrap(), b"keep");
    }
    #[test]
    fn stale_background_save_cannot_replace_quit_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let store = SessionStore::new(Some(path.clone()));
        store
            .save(
                2,
                &Document {
                    source: "latest".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        store.save(1, &Document::default()).unwrap();
        assert_eq!(load_session(&path).unwrap().source, "latest");
        assert!(
            store
                .save(
                    3,
                    &Document {
                        source: "x".repeat(monoprint::MAX_SOURCE_BYTES + 1),
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert_eq!(load_session(&path).unwrap().source, "latest");
    }
    #[test]
    fn exports_match_the_public_renderer_in_all_formats() {
        let renderer = Renderer::new();
        let d = Document {
            title: "example.ts".into(),
            transparent: true,
            numbers: true,
            highlights: "2,4-6".into(),
            text_highlights: "1:4-1:10, 2:6-2:11".into(),
            font_size: "21".into(),
            ligatures: true,
            font_path: Some(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/fira-code.ttf"),
            ),
            comment_font_path: Some(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/space-mono.ttf"),
            ),
            ..Default::default()
        };
        let (snippet, style) = d
            .prepare(
                FontCache::default().load(d.font_path.as_deref()).unwrap(),
                FontCache::default()
                    .load(d.comment_font_path.as_deref())
                    .unwrap(),
            )
            .unwrap();
        for mode in ["balanced", "fast"] {
            let mut png_document = d.clone();
            png_document.png_compression = mode.into();
            assert_eq!(
                export(&renderer, &png_document, Format::Png).unwrap(),
                renderer
                    .png(
                        &snippet,
                        &style,
                        PngOptions::new(Scale::X2).compression(mode.parse().unwrap()),
                    )
                    .unwrap()
            );
        }
        assert_eq!(
            export(&renderer, &d, Format::Svg).unwrap(),
            renderer
                .svg(&snippet, &style.clone().font_size(21))
                .unwrap()
                .as_bytes()
        );
        assert_eq!(
            export(&renderer, &d, Format::Html).unwrap(),
            renderer
                .html(&snippet, &style.font_size(21))
                .unwrap()
                .as_bytes()
        );
    }

    #[test]
    fn automatic_glyph_fallback_exports_emoji_and_cjk_without_changing_the_document() {
        let renderer = Renderer::new();
        let doc = Document {
            source: "// 中文 日本語 한국어 — 👩🏽‍💻 👨‍👩‍👧‍👦 🇳🇱 ❤️\nlet greeting = \"你好 👋🏽 ☀️\";".into(),
            title: "示例・예제 🇯🇵.rs".into(),
            language: "rust".into(),
            font_id: Some("fira-code".into()),
            comment_font_id: Some("space-mono".into()),
            font_size: "24".into(),
            text_highlights: "1:4-1:6".into(),
            ..Default::default()
        };
        let before = serde_json::to_string(&doc).unwrap();
        for format in [Format::Png, Format::Svg, Format::Html] {
            let output = export(&renderer, &doc, format)
                .unwrap_or_else(|error| panic!("{} fallback export: {error:#}", format.label()));
            assert!(!output.is_empty());
            assert_eq!(serde_json::to_string(&doc).unwrap(), before);
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("studio.json");
        save_session(&path, &doc).unwrap();
        assert_eq!(
            serde_json::to_string(&load_session(&path).unwrap()).unwrap(),
            before
        );
    }

    #[test]
    fn session_round_trip_preserves_source_and_options() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let d = Document {
            source: "\tfn main() {}\n\n".into(),
            transparent: true,
            highlights: "1-2".into(),
            text_highlights: "1:2-1:4".into(),
            font_size: "23".into(),
            png_compression: "fast".into(),
            ligatures: true,
            font_path: Some("code.ttf".into()),
            comment_font_path: Some("comments.otf".into()),
            ..Default::default()
        };
        save_session(&path, &d).unwrap();
        let restored = load_session(&path).unwrap();
        assert_eq!(restored.source, d.source);
        assert!(restored.transparent);
        assert_eq!(restored.highlights, "1-2");
        assert_eq!(restored.text_highlights, d.text_highlights);
        assert_eq!(restored.font_size, d.font_size);
        assert_eq!(restored.png_compression, "fast");
        assert!(restored.ligatures);
        assert_eq!(restored.font_path, d.font_path);
        assert_eq!(restored.comment_font_path, d.comment_font_path);
        let old: Document = serde_json::from_str(r#"{"source":"old draft"}"#).unwrap();
        assert_eq!(old.font_size, "15");
        assert_eq!(old.png_compression, "balanced");
        assert!(!old.ligatures);
        assert!(old.comment_font_path.is_none());
        assert!(old.text_highlights.is_empty());
        assert!(old.font_id.is_none());
        assert!(old.comment_font_id.is_none());
        let mut named = restored;
        named.select_typeface(false, "fira-code");
        named.select_typeface(true, "inter");
        assert!(named.font_path.is_none());
        assert!(named.comment_font_path.is_none());
        save_session(&path, &named).unwrap();
        let named = load_session(&path).unwrap();
        assert_eq!(named.typeface_selection(false), "fira-code");
        assert_eq!(named.typeface_selection(true), "inter");
        assert!(export(&Renderer::new(), &named, Format::Png).is_ok());
    }
}
