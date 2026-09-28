use crate::{Appearance, Error, assets};
use std::{fmt, path::Path, str::FromStr};

/// A bundled code-image theme. Parse an ID or discover themes with [`Theme::all`].
///
/// IDs are case-insensitive when parsed. The default is Candy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Theme(pub(crate) &'static str);
impl Default for Theme {
    fn default() -> Self {
        Self("removed-frame")
    }
}
impl Theme {
    /// Every bundled theme, ordered by ID. No renderer is initialized.
    pub fn all() -> impl ExactSizeIterator<Item = Self> {
        assets::themes().values().map(|theme| Self(&theme.id))
    }
    /// Stable lowercase theme ID.
    pub fn id(self) -> &'static str {
        self.0
    }
    /// Human-readable theme name.
    pub fn name(self) -> &'static str {
        &self.data().name
    }
    /// Palette family, or `Monoprint` for the bundled frame presets.
    pub fn family(self) -> &'static str {
        self.data()
            .editor
            .as_ref()
            .map_or("Monoprint", |e| e.family.as_str())
    }
    /// Preferred appearance: dark when available, otherwise light.
    pub fn default_appearance(self) -> Appearance {
        if self.supports(Appearance::Dark) {
            Appearance::Dark
        } else {
            Appearance::Light
        }
    }
    /// Whether the theme provides this light or dark variant.
    pub fn supports(self, appearance: Appearance) -> bool {
        self.data().syntax.contains_key(appearance.as_str())
    }
    /// How the frame handles line numbers. Plain text always hides numbers.
    pub fn line_numbers(self) -> LineNumberPolicy {
        let theme = self.data();
        if theme.partner && !theme.line_numbers_toggleable {
            LineNumberPolicy::Fixed(theme.line_numbers)
        } else {
            LineNumberPolicy::Toggleable {
                default: theme.line_numbers,
            }
        }
    }
    pub(crate) fn data(self) -> &'static assets::Theme {
        &assets::themes()[self.0]
    }
}
impl FromStr for Theme {
    type Err = Error;
    fn from_str(id: &str) -> Result<Self, Error> {
        assets::themes()
            .get(&id.to_ascii_lowercase())
            .map(|theme| Self(&theme.id))
            .ok_or_else(|| Error::UnknownTheme(id.to_owned()))
    }
}
impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A bundled language, independent of Giallo's internal grammar types.
///
/// Parse an ID or alias, or infer from a filename. TypeScript aliases select
/// TSX, matching legacy frames. Unspecified input defaults to [`Language::PLAIN`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Language(pub(crate) &'static str);
impl Default for Language {
    fn default() -> Self {
        Self::PLAIN
    }
}
impl Language {
    /// Unhighlighted plain text; line numbers are always hidden.
    pub const PLAIN: Self = Self("plain");
    /// Every selectable language and plain text. No renderer is initialized.
    /// TypeScript and TeX are represented by TSX and LaTeX respectively,
    /// matching the CLI-compatible parsing aliases.
    pub fn all() -> impl Iterator<Item = Self> {
        assets::grammars()
            .iter()
            .filter(|g| !matches!(g.name.as_str(), "typescript" | "tex"))
            .map(|g| Self(&g.name))
            .chain([Self::PLAIN])
    }
    /// Canonical grammar ID.
    pub fn id(self) -> &'static str {
        self.0
    }
    /// Human-readable language name.
    pub fn name(self) -> &'static str {
        self.data()
            .map(|g| g.display_name.as_str())
            .unwrap_or("Plain Text")
    }
    /// Aliases provided by the bundled grammar catalog.
    ///
    /// Parsing also accepts conventional file extensions such as `rs` and `mts`.
    pub fn aliases(self) -> impl Iterator<Item = &'static str> {
        let extra: &[&str] = if self.0 == "tsx" {
            &["typescript", "ts", "mts", "cts"]
        } else if self.0 == "latex" {
            &["tex"]
        } else {
            &[]
        };
        self.data()
            .map(|g| g.aliases.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(String::as_str)
            .chain(extra.iter().copied())
    }
    /// Infer a language from a path's filename without reading the filesystem.
    ///
    /// Recognizes extensions, Dockerfile/Containerfile, and Makefile/GNUmakefile.
    /// Unknown or non-UTF-8 filenames return `None`.
    pub fn for_file_name(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        let name = path.file_name()?.to_str()?;
        let id = match name {
            "Dockerfile" | "Containerfile" => "dockerfile",
            "Makefile" | "GNUmakefile" => "make",
            _ => path.extension()?.to_str()?,
        };
        id.parse().ok()
    }
    fn data(self) -> Option<&'static assets::Grammar> {
        assets::grammars().iter().find(|g| g.name == self.0)
    }
}
impl FromStr for Language {
    type Err = Error;
    fn from_str(id: &str) -> Result<Self, Error> {
        let lower = id.to_ascii_lowercase();
        let candidate = match lower.as_str() {
            "typescript" | "ts" | "mts" | "cts" => "tsx",
            "js" => "javascript",
            "rs" => "rust",
            "py" => "python",
            "sh" | "bash" | "zsh" => "shellscript",
            "yml" => "yaml",
            "md" => "markdown",
            "h" => "c",
            "cs" => "csharp",
            "kt" => "kotlin",
            "ex" | "exs" => "elixir",
            "rb" => "ruby",
            "tex" => "latex",
            "txt" | "text" | "plaintext" => "plain",
            other => other,
        };
        if candidate == "plain" {
            return Ok(Self::PLAIN);
        }
        let grammars = assets::grammars();
        grammars
            .iter()
            .find(|g| g.name == candidate)
            .or_else(|| {
                grammars.iter().find(|g| {
                    g.aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(candidate))
                })
            })
            .map(|g| Self(&g.name))
            .ok_or_else(|| Error::UnknownLanguage(id.to_owned()))
    }
}
impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A theme's line-number behavior, before plain-text suppression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineNumberPolicy {
    /// The frame always shows (`true`) or hides (`false`) line numbers.
    Fixed(bool),
    /// Callers may override the theme's default.
    Toggleable {
        /// Whether numbers appear when the preference is `Auto`.
        default: bool,
    },
}
