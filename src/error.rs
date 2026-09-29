use crate::{Appearance, TextRange, Theme};
use std::fmt;

/// An error callers can inspect without depending on a rendering backend.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// No bundled theme matches this ID.
    UnknownTheme(String),
    /// No bundled language or alias matches this ID.
    UnknownLanguage(String),
    /// No bundled typeface matches this ID.
    UnknownFont(String),
    /// A request violates a documented option or resource limit.
    InvalidOption(InvalidOption),
    /// The supplied bytes are not a supported TTF or OTF face.
    InvalidFont,
    /// A visible character is missing from its font and bundled fallbacks.
    UnsupportedGlyph {
        /// The text containing the missing character.
        text: TextRole,
        /// The unsupported Unicode scalar value.
        character: char,
        /// Whether a custom code font can supply this glyph.
        code_font: bool,
    },
    /// The measured image exceeds its internal pixel budget (16,777,216 for
    /// PNG/raw images, 64 million for SVG), or 32,768 pixels per edge.
    ImageTooLarge {
        /// Internal width, before downsampling.
        width: u32,
        /// Internal height, before downsampling.
        height: u32,
        /// An odd raster scale requires rendering at twice the requested scale.
        supersampled: bool,
    },
    /// An embedded resource or rendering backend failed.
    Render(RenderError),
}

/// Invalid configuration, reported before highlighting or image allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidOption {
    /// Source exceeds [`crate::MAX_SOURCE_BYTES`], before newline normalization.
    SourceTooLarge {
        /// Supplied byte length.
        len: usize,
    },
    /// Font exceeds [`crate::MAX_FONT_BYTES`].
    FontTooLarge {
        /// Supplied byte length.
        len: usize,
    },
    /// Padding is outside 0..=512 CSS pixels.
    Padding(u32),
    /// Code font size is outside 8..=72 CSS pixels.
    FontSize(u32),
    /// Width must satisfy both the outer bounds and the space needed by padding.
    Width {
        /// Requested CSS width.
        width: u32,
        /// Minimum width for this padding.
        min: u32,
        /// Maximum supported CSS width.
        max: u32,
    },
    /// PNG scale is outside 1..=6.
    Scale(u32),
    /// Unknown PNG compression mode.
    PngCompression(String),
    /// A background name is not one of the five supported treatments.
    Background(String),
    /// This theme does not provide the selected appearance.
    Appearance {
        /// Selected theme.
        theme: Theme,
        /// Unavailable variant.
        appearance: Appearance,
    },
    /// A one-based inclusive range is empty or outside the normalized source.
    HighlightedLines {
        /// Inclusive first line, as supplied.
        start: usize,
        /// Inclusive last line, as supplied.
        end: usize,
        /// Logical source lines after normalization.
        line_count: usize,
    },
    /// An exact selection is empty, reversed, or outside the normalized source.
    HighlightedText(TextRange),
    /// Text selections must use `line:column-line:column` syntax.
    TextRangeSyntax(String),
    /// Subtitle metadata is supported only by Auth0.
    Subtitle {
        /// Theme that cannot accept subtitle metadata.
        theme: Theme,
    },
}

/// The visible text whose font cannot represent a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    /// Source code.
    Code,
    /// A grammar-recognized comment with a separate typeface.
    Comment,
    /// The frame's title.
    Title,
}

/// Diagnostic details of an unexpected backend failure.
///
/// Its display text is for people; match [`Error`] for programmatic handling.
#[derive(Debug)]
pub struct RenderError(pub(crate) String);
impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTheme(id) => write!(f, "unknown theme {id:?}"),
            Self::UnknownLanguage(id) => write!(f, "unknown language {id:?}"),
            Self::UnknownFont(id) => write!(f, "unknown bundled font {id:?}"),
            Self::InvalidOption(option) => option.fmt(f),
            Self::InvalidFont => f.write_str("font must be a valid TTF or OTF file"),
            Self::UnsupportedGlyph {
                text,
                character,
                code_font,
            } => {
                let label = match text {
                    TextRole::Code => "code",
                    TextRole::Comment => "comment",
                    TextRole::Title => "title",
                };
                write!(
                    f,
                    "{label} font and bundled fallbacks cannot display {character:?} (U+{:04X}); {}",
                    *character as u32,
                    if *text == TextRole::Comment {
                        "supply a comment font containing this glyph"
                    } else if *code_font {
                        "supply a code font containing this glyph"
                    } else {
                        "use text supported by this theme's label font or bundled fallbacks (a custom font only changes the code font)"
                    }
                )
            }
            Self::ImageTooLarge {
                width,
                height,
                supersampled,
            } => write!(
                f,
                "image is too large ({width}x{height} internal render pixels{}); reduce the scale, width, or snippet length",
                if *supersampled {
                    "; odd PNG scales render at 2x the requested scale before downsampling"
                } else {
                    ""
                }
            ),
            Self::Render(error) => error.fmt(f),
        }
    }
}
impl fmt::Display for InvalidOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { .. } => {
                f.write_str("source exceeds 1 MiB; select a smaller snippet")
            }
            Self::FontTooLarge { .. } => {
                f.write_str("font exceeds 64 MiB; use a smaller TTF or OTF file")
            }
            Self::Padding(value) => write!(f, "padding {value} must be between 0 and 512px"),
            Self::FontSize(value) => write!(f, "font size {value} must be between 8 and 72px"),
            Self::Width { width, min, max } => write!(
                f,
                "width {width} must be between {min} and {max}px and leave room for the code after padding"
            ),
            Self::Scale(value) => write!(f, "PNG scale {value} must be between 1 and 6"),
            Self::PngCompression(value) => {
                write!(f, "unknown PNG compression {value:?}; use fast or balanced")
            }
            Self::Background(value) => write!(
                f,
                "unknown background {value:?}; use theme, solid, lattice, halftone, or none"
            ),
            Self::Appearance { theme, appearance } => {
                write!(f, "{} has no {appearance} variant", theme.name())
            }
            Self::HighlightedLines {
                start,
                end,
                line_count,
            } => write!(
                f,
                "highlight range {start}-{end} is empty or outside the {line_count} source lines"
            ),
            Self::Subtitle { .. } => f.write_str("subtitle is supported by the Auth0 theme"),
            Self::HighlightedText(range) => write!(
                f,
                "text highlight {range} is empty or outside the source; use one-based lines and character columns with an exclusive end"
            ),
            Self::TextRangeSyntax(value) => write!(
                f,
                "invalid text highlight {value:?}; use line:column-line:column, e.g. 2:1-2:8"
            ),
        }
    }
}
impl std::error::Error for InvalidOption {}
impl std::error::Error for Error {}
impl From<InvalidOption> for Error {
    fn from(value: InvalidOption) -> Self {
        Self::InvalidOption(value)
    }
}
impl Error {
    pub(crate) fn backend(error: anyhow::Error) -> Self {
        match error.downcast::<Self>() {
            Ok(error) => error,
            Err(error) => Self::Render(RenderError(format!("{error:#}"))),
        }
    }
}
