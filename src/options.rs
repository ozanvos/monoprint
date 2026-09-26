use crate::{Error, Font, InvalidOption, Language, Theme};
use std::{fmt, ops::RangeInclusive};

/// The theme's color variant. Dark is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Appearance {
    /// Dark variant.
    #[default]
    Dark,
    /// Light variant; not available for Stripe.
    Light,
}
impl Appearance {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}
impl fmt::Display for Appearance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A line-number preference, subject to the theme's fixed behavior.
/// Plain text never displays numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineNumbers {
    /// Follow the theme's default.
    #[default]
    Auto,
    /// Show numbers where the theme allows them.
    Show,
    /// Hide numbers where the theme allows it.
    Hide,
}

/// PNG pixel ratio. SVG and HTML always use CSS dimensions.
///
/// Odd scales render at twice this ratio and downsample for accurate half-pixel
/// line heights. Resource limits apply to that larger internal image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Scale {
    /// One output pixel per CSS pixel.
    X1 = 1,
    /// Two output pixels per CSS pixel.
    X2 = 2,
    /// Three output pixels per CSS pixel.
    X3 = 3,
    /// Four output pixels per CSS pixel (default).
    #[default]
    X4 = 4,
    /// Five output pixels per CSS pixel.
    X5 = 5,
    /// Six output pixels per CSS pixel.
    X6 = 6,
}
impl From<Scale> for u32 {
    fn from(scale: Scale) -> u32 {
        scale as u32
    }
}
impl TryFrom<u32> for Scale {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::X1),
            2 => Ok(Self::X2),
            3 => Ok(Self::X3),
            4 => Ok(Self::X4),
            5 => Ok(Self::X5),
            6 => Ok(Self::X6),
            _ => Err(InvalidOption::Scale(value).into()),
        }
    }
}

/// A one-based inclusive source-line selection.
///
/// Accepts `2` or `3..=5` via [`Snippet::highlight`]. Empty, zero-based, and
/// out-of-bounds ranges fail at render time, before expanding any range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineRange(pub(crate) RangeInclusive<usize>);
impl From<usize> for LineRange {
    fn from(line: usize) -> Self {
        Self(line..=line)
    }
}
impl From<RangeInclusive<usize>> for LineRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        Self(range)
    }
}

/// Content for one image, independent of its visual style.
///
/// Owns the source so it can move into a worker thread. CRLF and CR normalize to
/// LF at render time; other text, blank lines, and trailing newlines survive.
/// The empty source has one logical line. A final newline adds a logical line.
#[derive(Clone, Debug, Default)]
pub struct Snippet {
    pub(crate) source: String,
    pub(crate) language: Language,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) highlights: Vec<LineRange>,
}
impl Snippet {
    /// Source text, using plain text highlighting until a language is selected.
    /// Size is checked at render time, before normalization.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            ..Self::default()
        }
    }
    /// Set the highlighting language. Does not infer from the title.
    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
    /// Set a frame label. Themes without a title field omit it.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    /// Set Auth0 subtitle metadata. Its pinned frame displays the language label
    /// instead; this text is not visible. Other themes reject nonempty subtitles.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }
    /// Add a highlighted line or inclusive range. Repeated selections are merged.
    ///
    /// ```
    /// let snippet = monoprint::Snippet::new("a\nb\nc\nd")
    ///     .highlight(1).highlight(3..=4);
    /// ```
    #[must_use]
    pub fn highlight(mut self, lines: impl Into<LineRange>) -> Self {
        self.highlights.push(lines.into());
        self
    }
    /// Remove highlighted-line selections, retaining the source and other content.
    #[must_use]
    pub fn clear_highlights(mut self) -> Self {
        self.highlights.clear();
        self
    }
}

/// A reusable visual style. Clone it to derive variations for other images.
///
/// Defaults: Candy, dark, opaque background, 64px padding, automatic width with
/// a 520px minimum, theme-default line numbers, and the theme's bundled font.
/// Setters are infallible; rendering validates the complete configuration.
#[derive(Clone, Debug)]
pub struct Style {
    pub(crate) theme: Theme,
    pub(crate) appearance: Appearance,
    pub(crate) transparent: bool,
    pub(crate) padding: u32,
    pub(crate) width: Option<u32>,
    pub(crate) line_numbers: LineNumbers,
    pub(crate) font: Option<Font>,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            appearance: Appearance::Dark,
            transparent: false,
            padding: 64,
            width: None,
            line_numbers: LineNumbers::Auto,
            font: None,
        }
    }
}
impl Style {
    /// Use a theme with all other defaults unchanged.
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            ..Self::default()
        }
    }
    /// Change the theme, retaining the other settings.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    /// Select a light or dark variant. Unsupported variants fail when rendered.
    #[must_use]
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }
    /// Remove the decorative background while preserving the code window.
    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
    /// Outer padding in CSS pixels, from 0 through 512.
    #[must_use]
    pub fn padding(mut self, pixels: u32) -> Self {
        self.padding = pixels;
        self
    }
    /// Fixed outer width in CSS pixels, from 80 through 8192. Code wraps to fit.
    /// Must also leave at least 49px after subtracting twice the padding.
    #[must_use]
    pub fn width(mut self, pixels: u32) -> Self {
        self.width = Some(pixels);
        self
    }
    /// Restore automatic width (at least 520px), without wrapping source lines.
    #[must_use]
    pub fn fit_width(mut self) -> Self {
        self.width = None;
        self
    }
    /// Set the line-number preference; fixed partner-theme rules still apply.
    #[must_use]
    pub fn line_numbers(mut self, numbers: LineNumbers) -> Self {
        self.line_numbers = numbers;
        self
    }
    /// Override only the code font. Theme label fonts remain embedded.
    /// Cloning a style shares its font bytes and prepared font data.
    #[must_use]
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }
    /// Restore the selected theme's bundled code font.
    #[must_use]
    pub fn theme_font(mut self) -> Self {
        self.font = None;
        self
    }
}
