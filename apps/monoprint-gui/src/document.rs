use anyhow::{Context, Result, bail, ensure};
use monoprint::{
    Appearance, Background, Font, Language, LineNumberPolicy, LineNumbers, PngOptions, Renderer,
    Scale, Snippet, Style, Theme,
};
use serde::{Deserialize, Serialize};
#[cfg(not(all(target_os = "macos", feature = "app-store")))]
use std::io::Write;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAX_SESSION_BYTES: usize = 8 * 1024 * 1024;

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
    pub diff_base: Option<String>,
    pub diff_patch: bool,
    pub added: String,
    pub removed: String,
    pub diff_context: Option<usize>,
    pub diff_numbers: String,
    pub diff_words: bool,
    pub language: String,
    pub theme: String,
    pub light: bool,
    pub transparent: bool,
    pub background: String,
    /// None follows each theme; Some preserves an explicit choice across themes.
    pub line_numbers: Option<bool>,
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
    /// Persistent read-only access to user-selected fonts in sandboxed builds.
    pub font_bookmark: Option<Vec<u8>>,
    pub comment_font_bookmark: Option<Vec<u8>>,
    pub font_id: Option<String>,
    pub comment_font_id: Option<String>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            source: EXAMPLE.into(),
            diff_base: None,
            diff_patch: false,
            added: String::new(),
            removed: String::new(),
            diff_context: Some(3),
            diff_numbers: "new".into(),
            diff_words: true,
            language: "tsx".into(),
            theme: Theme::default().to_string(),
            light: false,
            transparent: false,
            background: "theme".into(),
            line_numbers: None,
            padding: "32".into(),
            width: "720".into(),
            title: "".into(),
            highlights: "".into(),
            text_highlights: "".into(),
            font_size: "15".into(),
            ligatures: true,
            scale: 2,
            png_compression: "balanced".into(),
            format: Format::Png,
            font_path: None,
            comment_font_path: None,
            font_bookmark: None,
            comment_font_bookmark: None,
            font_id: None,
            comment_font_id: None,
        }
    }
}

pub struct RenderHighlights {
    pub lines: Vec<(usize, usize)>,
    pub text: Vec<monoprint::TextRange>,
}

/// The custom-typeface fields for code or comments.
pub struct FontSlot<'a> {
    pub id: &'a Option<String>,
    pub path: &'a Option<PathBuf>,
    pub bookmark: &'a Option<Vec<u8>>,
}

impl Document {
    /// The selected theme, falling back to the default for unknown IDs.
    pub fn theme(&self) -> Theme {
        self.theme.parse().unwrap_or_default()
    }
    pub fn appearance(&self) -> Appearance {
        if self.light {
            Appearance::Light
        } else {
            Appearance::Dark
        }
    }
    pub fn numbers_visible(&self) -> bool {
        self.language != Language::PLAIN.id()
            && self.line_numbers.unwrap_or_else(|| {
                if self.terminal() {
                    return false;
                }
                match self.theme().line_numbers() {
                    LineNumberPolicy::Toggleable { default } | LineNumberPolicy::Fixed(default) => {
                        default
                    }
                    _ => false,
                }
            })
    }
    pub fn font_slot(&self, comments: bool) -> FontSlot<'_> {
        if comments {
            FontSlot {
                id: &self.comment_font_id,
                path: &self.comment_font_path,
                bookmark: &self.comment_font_bookmark,
            }
        } else {
            FontSlot {
                id: &self.font_id,
                path: &self.font_path,
                bookmark: &self.font_bookmark,
            }
        }
    }
    /// Replace the code or comment typeface with a bundled ID or a custom file.
    pub fn set_font(
        &mut self,
        comments: bool,
        id: Option<String>,
        path: Option<PathBuf>,
        bookmark: Option<Vec<u8>>,
    ) {
        let (slot_id, slot_path, slot_bookmark) = if comments {
            (
                &mut self.comment_font_id,
                &mut self.comment_font_path,
                &mut self.comment_font_bookmark,
            )
        } else {
            (
                &mut self.font_id,
                &mut self.font_path,
                &mut self.font_bookmark,
            )
        };
        (*slot_id, *slot_path, *slot_bookmark) = (id, path, bookmark);
    }

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
        let slot = self.font_slot(comments);
        slot.id
            .as_deref()
            .unwrap_or(if slot.path.is_some() { "custom" } else { "" })
    }

    /// Select a bundled ID, or an empty ID to restore inheritance.
    pub fn select_typeface(&mut self, comments: bool, id: &str) {
        self.set_font(
            comments,
            (!id.is_empty()).then(|| id.to_owned()),
            None,
            None,
        );
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
        let appearance = self.appearance();
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
            .line_numbers(match self.line_numbers {
                None => LineNumbers::Auto,
                Some(true) => LineNumbers::Show,
                Some(false) => LineNumbers::Hide,
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
        style = style
            .diff_numbers(self.diff_numbers.parse()?)
            .diff_words(self.diff_words);
        let mut snippet = if let Some(diff) = self.diff()? {
            Snippet::diff(diff)
        } else {
            Snippet::new(&self.source)
        }
        .language(self.language.parse::<Language>()?);
        if !self.title.is_empty() {
            snippet = snippet.title(&self.title);
        }
        let highlights = self.render_highlights()?;
        for (start, end) in highlights.lines {
            snippet = snippet.highlight(start..=end);
        }
        for range in highlights.text {
            snippet = snippet.highlight_text(range);
        }
        Ok((snippet, style))
    }
    /// Project editor selections onto the new side of a folded comparison.
    pub fn render_highlights(&self) -> Result<RenderHighlights> {
        if self.highlights.trim().is_empty() && self.text_highlights.trim().is_empty() {
            return Ok(RenderHighlights {
                lines: Vec::new(),
                text: Vec::new(),
            });
        }
        let source = if self.diff_base.is_some() {
            self.source.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            self.visible_source()?
        };
        let lines: Vec<_> = source.split('\n').collect();
        let mut map = std::collections::BTreeMap::new();
        if self.diff_base.is_some()
            && let Some(diff) = self.diff()?
        {
            for (index, row) in diff
                .rows()?
                .iter()
                .filter(|r| !matches!(r.kind, monoprint::RowKind::Fold { .. }))
                .enumerate()
            {
                if let Some(new) = row.new {
                    map.insert(new, index + 1);
                }
            }
        }
        let mut result = RenderHighlights {
            lines: Vec::new(),
            text: Vec::new(),
        };
        for (a, b) in highlight_ranges(&self.highlights, lines.len())? {
            if self.diff_base.is_none() {
                result.lines.push((a, b));
                continue;
            }
            for n in a..=b {
                if let Some(&display) = map.get(&n) {
                    result.lines.push((display, display));
                }
            }
        }
        for text in self
            .text_highlights
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let range: monoprint::TextRange = text.parse()?;
            ensure!(
                crate::selection::resolve(&source, text).is_some(),
                "Text highlight is outside the source."
            );
            if self.diff_base.is_none() {
                result.text.push(range);
                continue;
            }
            for line in range.start.0..=range.end.0 {
                let Some(&display) = map.get(&line) else {
                    continue;
                };
                let start = if line == range.start.0 {
                    range.start.1
                } else {
                    1
                };
                let end = if line == range.end.0 {
                    range.end.1
                } else {
                    lines[line - 1].chars().count() + 1
                };
                if start < end {
                    result
                        .text
                        .push(monoprint::TextRange::new((display, start), (display, end)));
                }
            }
        }
        Ok(result)
    }
    pub fn terminal(&self) -> bool {
        self.language
            .parse::<Language>()
            .is_ok_and(Language::is_terminal)
    }
    pub fn editor_source(&self) -> String {
        if self.terminal() {
            monoprint::ansi::strip(&self.source)
        } else {
            self.source.clone()
        }
    }
    pub fn diff(&self) -> Result<Option<monoprint::Diff>> {
        let diff = if self.diff_patch {
            Some(monoprint::Diff::from_patch(&self.source)?)
        } else if let Some(base) = &self.diff_base {
            Some(monoprint::Diff::compare(base, &self.source)?)
        } else if !self.added.is_empty() || !self.removed.is_empty() {
            let mut diff = monoprint::Diff::marked(&self.source);
            let lines = self.line_count();
            for (a, b) in highlight_ranges(&self.added, lines)? {
                diff = diff.added(a..=b);
            }
            for (a, b) in highlight_ranges(&self.removed, lines)? {
                diff = diff.removed(a..=b);
            }
            Some(diff)
        } else {
            None
        };
        Ok(diff.map(|d| {
            d.context(
                self.diff_context
                    .map(monoprint::Context::Lines)
                    .unwrap_or(monoprint::Context::All),
            )
        }))
    }
    pub fn visible_source(&self) -> Result<String> {
        Ok(self
            .diff()?
            .map(|d| d.displayed_source())
            .unwrap_or_else(|| self.editor_source()))
    }
    /// Lines as the renderer counts them: CRLF, CR and LF each end a line.
    pub fn line_count(&self) -> usize {
        self.source
            .replace("\r\n", "\n")
            .split(['\n', '\r'])
            .count()
    }
    pub fn select_theme(&mut self, theme: Theme) {
        self.theme = theme.canonical().id().into();
        if theme != theme.canonical() || !theme.supports(self.appearance()) {
            self.light = theme.default_appearance() == Appearance::Light;
        }
    }
    /// Suggest a free filename without creating or changing anything on disk.
    pub fn suggested_name(&self, directory: &Path) -> Result<String> {
        let name = Path::new(&self.title)
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("monoprint");
        let extension = self.format.extension();
        let mut number = 1_u64;
        loop {
            let candidate = if number == 1 {
                format!("{name}.{extension}")
            } else {
                format!("{name}-{number}.{extension}")
            };
            // A directory or dangling symlink also occupies the name.
            match fs::symlink_metadata(directory.join(&candidate)) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(candidate);
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "Could not check export filenames in {}",
                            directory.display()
                        )
                    });
                }
                Ok(_) => number += 1,
            }
        }
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
            let line = |value: &str| {
                value
                    .trim()
                    .parse::<usize>()
                    .context("Use line numbers such as 1, 3-5 for highlights.")
            };
            let (first, last) = (line(first)?, line(last)?);
            ensure!(
                first > 0 && first <= last && last <= lines,
                "Highlight lines must be between 1 and {lines}, in ascending ranges."
            );
            Ok((first, last))
        })
        .collect()
}

pub fn read_source(path: &Path) -> Result<String> {
    crate::sandbox::with_file_access(path, None, read_source_contents)
}

fn read_source_contents(path: &Path) -> Result<String> {
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
    pub fn load_document(&mut self, document: &Document, comments: bool) -> Result<Option<Font>> {
        let FontSlot { id, path, bookmark } = document.font_slot(comments);
        if id.is_some() || path.is_none() {
            return self.load_selection(id.as_deref(), path.as_deref());
        }
        crate::sandbox::with_file_access(path.as_deref().unwrap(), bookmark.as_deref(), |path| {
            self.load(Some(path))
        })
    }
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
    let font = FontCache::default().load_document(document, false)?;
    let comments = FontCache::default().load_document(document, true)?;
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
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    {
        crate::sandbox::write_atomic(path, bytes)
    }
    #[cfg(not(all(target_os = "macos", feature = "app-store")))]
    {
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
}

pub fn session_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MONOPRINT_SESSION") {
        return (path != "off").then(|| PathBuf::from(path));
    }
    Some(data_directory()?.join("studio.json"))
}

pub fn data_directory() -> Option<PathBuf> {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    let home = crate::sandbox::home_directory();
    #[cfg(all(
        not(target_os = "windows"),
        not(all(target_os = "macos", feature = "app-store"))
    ))]
    let home = PathBuf::from(std::env::var_os("HOME")?);
    #[cfg(target_os = "macos")]
    let dir = home.join("Library/Application Support/Monoprint");
    #[cfg(target_os = "windows")]
    let dir = PathBuf::from(std::env::var_os("APPDATA")?).join("Monoprint");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let dir = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".local/state"))
        .join("monoprint");
    Some(dir)
}

pub fn load_session(path: &Path) -> Result<Document> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_SESSION_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_SESSION_BYTES,
        "Saved session is too large."
    );
    let mut saved: serde_json::Value = serde_json::from_slice(&bytes)?;
    if let Some(fields) = saved.as_object_mut()
        && !fields.contains_key("line_numbers")
    {
        // Old drafts used false for both the initial state and an explicit hide,
        // and themes could ignore it. Inherit by default; preserve an opt-in.
        if fields.get("numbers").and_then(|value| value.as_bool()) == Some(true) {
            fields.insert("line_numbers".into(), true.into());
        }
    }
    let mut doc: Document = serde_json::from_value(saved)?;
    let retired: &[&str] =
        &serde_json::from_str::<Vec<&str>>(include_str!("../../../assets/retired-themes.json"))?;
    if retired.iter().any(|id| id.eq_ignore_ascii_case(&doc.theme)) {
        doc.select_theme(Theme::default());
    }
    doc.theme.parse::<Theme>()?;
    doc.language.parse::<Language>()?;
    Scale::try_from(doc.scale)?;
    ensure!(
        doc.source.len() <= monoprint::MAX_SOURCE_BYTES,
        "Saved code is too large."
    );
    ensure!(
        doc.diff_base
            .as_ref()
            .is_none_or(|s| s.len() <= monoprint::MAX_SOURCE_BYTES),
        "Saved previous version is too large."
    );
    Ok(doc)
}

pub fn save_session(path: &Path, document: &Document) -> Result<()> {
    ensure!(
        document.source.len() <= monoprint::MAX_SOURCE_BYTES,
        "Code exceeds the 1 MiB session limit; the previous session was retained."
    );
    ensure!(
        document
            .diff_base
            .as_ref()
            .is_none_or(|s| s.len() <= monoprint::MAX_SOURCE_BYTES),
        "Previous version exceeds the 1 MiB session limit."
    );
    let bytes = serde_json::to_vec(document)?;
    ensure!(
        bytes.len() <= MAX_SESSION_BYTES,
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
    saved_revision: Mutex<(u64, bool)>,
}
impl SessionStore {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            saved_revision: Mutex::new((0, true)),
        }
    }
    pub fn save(&self, revision: u64, document: &Document) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let mut saved = self.saved_revision.lock().unwrap();
        if !saved.1 || revision <= saved.0 {
            return Ok(());
        }
        save_session(path, document)?;
        saved.0 = revision;
        Ok(())
    }

    #[cfg(any(target_os = "linux", all(test, unix)))]
    pub fn set_saving_enabled(&self, enabled: bool) {
        // Wait for an in-flight preview save before removing saved user data.
        self.saved_revision.lock().unwrap().1 = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_numbers_follow_theme_defaults_until_overridden() {
        let renderer = Renderer::new();
        let mut doc = Document::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        for preference in [None, Some(false), Some(true), None] {
            doc.line_numbers = preference;
            for (theme, default) in [("ordinal", true), ("monoprint", false), ("ground", false)] {
                doc.select_theme(theme.parse().unwrap());
                assert_eq!(doc.line_numbers, preference);
                assert_eq!(doc.numbers_visible(), preference.unwrap_or(default));
                let (snippet, style) = doc.prepare(None, None).unwrap();
                let html = renderer.html(&snippet, &style).unwrap();
                assert_eq!(
                    html.contains("class=\"Editor__editor Editor__showLineNumbers"),
                    doc.numbers_visible(),
                );
                save_session(&path, &doc).unwrap();
                doc = load_session(&path).unwrap();
                assert_eq!(doc.line_numbers, preference);
            }
        }
        doc.line_numbers = Some(true);
        doc.language = "plain".into();
        assert!(!doc.numbers_visible());
        doc.language = "rust".into();
        assert!(doc.numbers_visible());
    }

    #[test]
    fn retired_theme_drafts_migrate_without_changing_other_fields() {
        let retired: Vec<&str> =
            serde_json::from_str(include_str!("../../../assets/retired-themes.json")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        for id in retired {
            for light in [false, true] {
                let original = Document {
                    theme: id.to_uppercase(),
                    source: "// Saved work\nconst answer = 42;".into(),
                    light,
                    padding: "24".into(),
                    width: "580".into(),
                    line_numbers: Some(true),
                    font_size: "18".into(),
                    highlights: "2".into(),
                    background: "solid".into(),
                    ..Default::default()
                };
                save_session(&path, &original).unwrap();
                let saved_bytes = fs::read(&path).unwrap();
                let migrated = load_session(&path).unwrap();
                let mut expected = serde_json::to_value(&original).unwrap();
                expected["theme"] = "monoprint".into();
                assert_eq!(serde_json::to_value(&migrated).unwrap(), expected, "{id}");
                assert_eq!(
                    fs::read(&path).unwrap(),
                    saved_bytes,
                    "loading must not overwrite a draft"
                );
                migrated.prepare(None, None).unwrap();
            }
        }
        fs::write(&path, r#"{"theme":"not-a-known-theme"}"#).unwrap();
        assert!(load_session(&path).is_err());
    }

    #[test]
    fn legacy_drafts_inherit_numbers_unless_explicitly_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        for (json, preference) in [
            (r#"{"theme":"supabase","numbers":false}"#, None),
            (r#"{"theme":"supabase","numbers":true}"#, Some(true)),
            (
                r#"{"theme":"supabase","numbers":true,"line_numbers":null}"#,
                None,
            ),
            (
                r#"{"theme":"supabase","numbers":true,"line_numbers":false}"#,
                Some(false),
            ),
        ] {
            fs::write(&path, json).unwrap();
            assert_eq!(load_session(&path).unwrap().line_numbers, preference);
        }
    }

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
        d.select_theme("nord".parse().unwrap());
        assert!(!d.light);
        d.width = "100".into();
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
    fn export_suggestions_keep_previous_files_and_follow_title_and_format() {
        let dir = tempfile::tempdir().unwrap();
        for (title, stem) in [("", "monoprint"), ("src/example.test.ts", "example.test")] {
            for format in [Format::Png, Format::Svg, Format::Html] {
                let doc = Document {
                    title: title.into(),
                    format,
                    ..Default::default()
                };
                let extension = format.extension();
                for suffix in ["", "-2", "-3"] {
                    let expected = format!("{stem}{suffix}.{extension}");
                    assert_eq!(doc.suggested_name(dir.path()).unwrap(), expected);
                    // Cancelling the panel doesn't reserve or consume a name.
                    assert_eq!(doc.suggested_name(dir.path()).unwrap(), expected);
                    assert!(!dir.path().join(&expected).exists());
                    write_atomic(&dir.path().join(expected), b"previous export").unwrap();
                }
                assert_eq!(
                    fs::read(dir.path().join(format!("{stem}.{extension}"))).unwrap(),
                    b"previous export"
                );
                let other = tempfile::tempdir().unwrap();
                assert_eq!(
                    doc.suggested_name(other.path()).unwrap(),
                    format!("{stem}.{extension}")
                );
            }
        }
    }

    #[test]
    fn export_suggestions_skip_directories_and_dangling_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("monoprint.png")).unwrap();
        let doc = Document::default();
        assert_eq!(doc.suggested_name(dir.path()).unwrap(), "monoprint-2.png");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("missing", dir.path().join("monoprint-2.png")).unwrap();
            assert_eq!(doc.suggested_name(dir.path()).unwrap(), "monoprint-3.png");
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
            line_numbers: Some(true),
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
            font_bookmark: Some(vec![0, 1, 255]),
            comment_font_bookmark: Some(vec![2, 0, 128]),
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
        assert_eq!(restored.font_bookmark, d.font_bookmark);
        assert_eq!(restored.comment_font_bookmark, d.comment_font_bookmark);
        let old: Document = serde_json::from_str(r#"{"source":"old draft"}"#).unwrap();
        assert_eq!(old.font_size, "15");
        assert_eq!(old.png_compression, "balanced");
        assert!(old.ligatures);
        let explicit_off: Document = serde_json::from_str(r#"{"ligatures":false}"#).unwrap();
        assert!(!explicit_off.ligatures);
        assert!(old.comment_font_path.is_none());
        assert!(old.text_highlights.is_empty());
        assert!(old.font_id.is_none());
        assert!(old.comment_font_id.is_none());
        assert!(old.font_bookmark.is_none());
        assert!(old.comment_font_bookmark.is_none());
        let mut named = restored;
        named.select_typeface(false, "fira-code");
        named.select_typeface(true, "inter");
        assert!(named.font_path.is_none());
        assert!(named.comment_font_path.is_none());
        assert!(named.font_bookmark.is_none());
        assert!(named.comment_font_bookmark.is_none());
        save_session(&path, &named).unwrap();
        let named = load_session(&path).unwrap();
        assert_eq!(named.typeface_selection(false), "fira-code");
        assert_eq!(named.typeface_selection(true), "inter");
        assert!(export(&Renderer::new(), &named, Format::Png).is_ok());
    }
}
