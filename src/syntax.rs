//! Shared native syntax adapter for image rendering and the Studio source editor.
use crate::{Appearance, Error, InvalidOption, Language, MAX_SOURCE_BYTES, Renderer};
use anyhow::{Context, Result};
use giallo::{Color, FontStyle, HighlightOptions, Style, ThemeVariant};
use pierre_highlights_rs::{Highlighter, Language as PierreLanguage};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    ops::Range,
    sync::{LazyLock, Mutex},
};

/// Syntax backend policy. The language catalog is identical in either mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HighlightBackend {
    /// Native Pierre for supported languages, Giallo for the remaining grammars.
    #[default]
    Auto,
    /// Use the original TextMate backend for every language.
    Giallo,
}

/// A styled UTF-8 byte range for a native source editor, independent of GUI types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxSpan {
    /// Nonempty byte range on UTF-8 character boundaries.
    pub range: Range<usize>,
    /// CSS RGBA hex color from the selected GitHub editor palette.
    pub foreground: String,
    /// Use a bold font weight.
    pub bold: bool,
    /// Use an italic font face.
    pub italic: bool,
    /// Draw an underline.
    pub underline: bool,
    /// Draw a strikethrough.
    pub strikethrough: bool,
}

impl Renderer {
    /// Select syntax policy while retaining this renderer's shared caches.
    pub fn highlight_backend(mut self, backend: HighlightBackend) -> Self {
        self.1 = backend;
        self
    }

    /// Color an editor buffer without rendering an image. Offsets address the
    /// original UTF-8 input, including CRLF and lone CR line endings.
    pub fn highlight_source(
        &self,
        source: &str,
        language: Language,
        appearance: Appearance,
    ) -> Result<Vec<SyntaxSpan>, Error> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(InvalidOption::SourceTooLarge { len: source.len() }.into());
        }
        let theme = match appearance {
            Appearance::Light => "editor-github-light-default",
            Appearance::Dark => "editor-github-dark-default",
        };
        let result = highlight(self, language, source, theme, false).map_err(Error::backend)?;
        Ok(result
            .runs
            .into_iter()
            .map(|run| {
                let font = run.style.font_style;
                SyntaxSpan {
                    range: run.range,
                    foreground: run.style.foreground.as_hex(),
                    bold: font.contains(FontStyle::BOLD),
                    italic: font.contains(FontStyle::ITALIC),
                    underline: font.contains(FontStyle::UNDERLINE),
                    strikethrough: font.contains(FontStyle::STRIKETHROUGH),
                }
            })
            .collect())
    }
}

/// A small reusable pool avoids serializing concurrent preview/export calls.
#[derive(Default)]
pub(crate) struct NativePool(Mutex<Vec<Highlighter>>);
impl NativePool {
    fn take(&self) -> Highlighter {
        self.0.lock().unwrap().pop().unwrap_or_default()
    }
    fn put(&self, highlighter: Highlighter) {
        let mut pool = self.0.lock().unwrap();
        if pool.len() < 4 {
            pool.push(highlighter);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub range: Range<usize>,
    pub style: Style,
    pub comment: bool,
}
pub(crate) struct StyledSource {
    pub runs: Vec<Run>,
    pub background: Color,
}

#[derive(Deserialize)]
struct RawPalette {
    background: String,
    styles: Vec<(String, String, FontStyle)>,
}
struct Palette {
    background: Color,
    styles: Vec<Style>,
}
fn palettes() -> &'static BTreeMap<String, Palette> {
    static PALETTES: LazyLock<BTreeMap<String, Palette>> = LazyLock::new(|| {
        let raw: BTreeMap<String, RawPalette> =
            serde_json::from_str(include_str!("../assets/pierre-palettes.json"))
                .expect("generated Pierre palettes");
        raw.into_iter()
            .map(|(name, raw)| {
                assert_eq!(raw.styles.len(), 73);
                (
                    name,
                    Palette {
                        background: Color::from_hex(&raw.background).unwrap(),
                        styles: raw
                            .styles
                            .into_iter()
                            .map(|(foreground, background, font_style)| Style {
                                foreground: Color::from_hex(&foreground).unwrap(),
                                background: Color::from_hex(&background).unwrap(),
                                font_style,
                            })
                            .collect(),
                    },
                )
            })
            .collect()
    });
    &PALETTES
}

pub(crate) fn native_language(language: Language) -> Option<PierreLanguage> {
    // A few TextMate canonical IDs differ from Pierre's names; keep this
    // explicit so a vaguely related alias never silently selects another lexer.
    let id = match language.id() {
        "make" => "makefile",
        "common-lisp" => "lisp",
        other => other,
    };
    PierreLanguage::from_name(id)
}

pub(crate) fn highlight(
    renderer: &Renderer,
    language: Language,
    source: &str,
    theme: &str,
    comments: bool,
) -> Result<StyledSource> {
    // Pierre treats Python docstrings as ordinary strings. Keep TextMate's
    // docstring classification when a separate comment typeface needs it.
    let needs_docstrings = comments && language.id() == "python";
    if renderer.1 == HighlightBackend::Auto
        && !needs_docstrings
        && let Some(language) = native_language(language)
    {
        let palette = palettes()
            .get(theme)
            .with_context(|| format!("missing native palette {theme}"))?;
        let mut highlighter = renderer.0.native.take();
        let records = highlighter.highlight(language, source);
        let mut runs = Vec::with_capacity(records.len());
        for token in records.iter() {
            let style = palette.styles[token.kind.id() as usize];
            let comment = comments
                && matches!(
                    token.kind.name(),
                    "comment" | "comment.doc" | "keyword.jsdoc" | "type.jsdoc" | "variable.jsdoc"
                );
            // Pierre attaches trailing whitespace to the preceding token. Do
            // not let a comment font consume the next code line's indentation.
            let end = if comment {
                token.range.start
                    + source[token.range.clone()]
                        .trim_end_matches(char::is_whitespace)
                        .len()
            } else {
                token.range.end
            };
            push_run(&mut runs, token.range.start..end, style, comment);
            push_run(&mut runs, end..token.range.end, palette.styles[0], false);
        }
        renderer.0.native.put(highlighter);
        return Ok(StyledSource {
            runs,
            background: palette.background,
        });
    }
    let themes = if comments {
        ThemeVariant::Dual {
            light: theme,
            dark: "monoprint-comments",
        }
    } else {
        ThemeVariant::Single(theme)
    };
    let code = renderer.registry()?.highlight(
        source,
        &HighlightOptions::new(language.id(), themes).merge_whitespace(false),
    )?;
    let background = match code.theme {
        ThemeVariant::Single(theme) => theme.default_style.background,
        ThemeVariant::Dual { light, .. } => light.default_style.background,
    };
    let marker = Color::from_hex("#ffffff")?;
    let mut offset = 0;
    let mut runs = Vec::new();
    for line in code.tokens {
        for token in line {
            let (style, comment) = match token.style {
                ThemeVariant::Single(style) => (style, false),
                ThemeVariant::Dual { light, dark } => (light, dark.foreground == marker),
            };
            let end = offset + token.text.len();
            push_run(&mut runs, offset..end, style, comment);
            offset = end;
        }
        if source.as_bytes().get(offset) == Some(&b'\r') {
            offset += 1;
        }
        if source.as_bytes().get(offset) == Some(&b'\n') {
            offset += 1;
        }
    }
    Ok(StyledSource { runs, background })
}

fn push_run(runs: &mut Vec<Run>, range: Range<usize>, style: Style, comment: bool) {
    if range.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut()
        && previous.range.end == range.start
        && previous.style == style
        && previous.comment == comment
    {
        previous.range.end = range.end;
    } else {
        runs.push(Run {
            range,
            style,
            comment,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_path_does_not_load_giallo_and_all_unsupported_languages_fall_back() {
        let auto = Renderer::new();
        let baseline = auto.clone().highlight_backend(HighlightBackend::Giallo);
        let mut native = 0;
        for language in Language::all().filter(|&l| native_language(l).is_some()) {
            highlight(
                &auto,
                language,
                "// sample\nvalue = 42",
                "original-monoprint-dark",
                false,
            )
            .unwrap();
            native += 1;
        }
        assert!(native >= 80);
        assert!(
            auto.0.registry.get().is_none(),
            "supported lexers must not initialize Giallo"
        );
        for language in Language::all().filter(|&l| native_language(l).is_none()) {
            let actual = highlight(
                &auto,
                language,
                "// sample\nvalue = 42",
                "original-monoprint-dark",
                true,
            )
            .unwrap();
            let expected = highlight(
                &baseline,
                language,
                "// sample\nvalue = 42",
                "original-monoprint-dark",
                true,
            )
            .unwrap();
            assert_eq!(actual.runs, expected.runs, "{language}");
            assert_eq!(actual.background, expected.background);
        }
        assert!(auto.0.registry.get().is_some());
    }

    #[test]
    fn native_comments_do_not_style_the_next_lines_indentation() {
        let r = Renderer::new();
        let source = "// café\n    let x = \"// string\"; /* block */\n    x";
        let result = highlight(
            &r,
            "rust".parse().unwrap(),
            source,
            "original-monoprint-dark",
            true,
        )
        .unwrap();
        let comments = result
            .runs
            .iter()
            .filter(|run| run.comment)
            .map(|run| &source[run.range.clone()])
            .collect::<Vec<_>>();
        assert_eq!(comments, ["// café", "/* block */"]);
        assert!(r.0.registry.get().is_none());
    }

    #[test]
    fn python_comment_fonts_preserve_docstrings_through_giallo() {
        let auto = Renderer::new();
        let baseline = auto.clone().highlight_backend(HighlightBackend::Giallo);
        let source = "# comment\n\"\"\"docstring\"\"\"\nx = '# string'";
        let language = "python".parse().unwrap();
        let actual = highlight(&auto, language, source, "original-monoprint-dark", true).unwrap();
        let expected =
            highlight(&baseline, language, source, "original-monoprint-dark", true).unwrap();
        assert_eq!(actual.runs, expected.runs);
        assert!(
            actual
                .runs
                .iter()
                .any(|run| run.comment && source[run.range.clone()].contains("docstring"))
        );
    }
}
