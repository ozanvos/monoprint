//! Offline code images with native Rust rendering.
//!
//! ```
//! use monoprint::{Renderer, Scale, Snippet, Style};
//!
//! # fn main() -> Result<(), monoprint::Error> {
//! let png = Renderer::new().png(
//!     &Snippet::new("fn main() {}\n").language("rust".parse()?).title("main.rs"),
//!     &Style::default(),
//!     Scale::X2,
//! )?;
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok(()) }
//! ```
//!
//! Reuse a [`Renderer`] and [`Style`] for batches or live previews. SVG and HTML
//! return `String`; PNG returns `Vec<u8>`. File IO belongs to the caller.
//!
//! ```
//! use monoprint::{Appearance, Language, Renderer, Snippet, Style};
//!
//! # fn main() -> Result<(), monoprint::Error> {
//! let renderer = Renderer::new();
//! let style = Style::new("removed-frame".parse()?)
//!     .appearance(Appearance::Light).padding(32).width(640);
//! for (name, source) in [("a.rs", "let a = 1;"), ("b.py", "b = 2")] {
//!     let snippet = Snippet::new(source).title(name).highlight(1)
//!         .language(Language::for_file_name(name).unwrap_or_default());
//!     let html = renderer.html(&snippet, &style)?;
//!     assert!(html.starts_with("<!doctype html>"));
//! }
//! # Ok(()) }
//! ```
//!
//! No browser, network, filesystem, installed fonts, or async runtime is used.
//! Calls are synchronous CPU work; interactive/async apps should use a worker or
//! their runtime's blocking pool. [`Renderer`], [`Style`], [`Snippet`], and
//! [`Font`] can be sent and shared across threads. Rendering does not support
//! cancellation; preview apps should coalesce edits before dispatching work.
//!
//! # Validation and limits
//!
//! Theme/language parsing and [`Font::from_bytes`] validate immediately. Style
//! and snippet setters validate at render time. Input is capped at 1 MiB before
//! newline normalization and fonts at 64 MiB. PNG allows 16,777,216 internal
//! pixels, SVG 64 million; both allow 32,768 pixels per edge. Missing glyphs
//! produce an explicit error.
//!
//! # Cargo features
//!
//! `cli` is enabled by default for `cargo install`. Library consumers can disable
//! default features to omit clap and the executable. All formats remain available.
#![warn(missing_docs)]

mod assets;
mod background;
mod catalog;
mod editor_theme;
mod effects;
mod error;
mod fallback;
mod font;
mod highlight;
mod options;
mod pixels;
mod raster;
mod render;
mod resample;

pub use background::Background;
pub use catalog::{Language, LineNumberPolicy, Theme};
pub use error::{Error, InvalidOption, RenderError, TextRole};
pub use font::{BundledFont, Font};
pub use options::{
    Appearance, LineNumbers, LineRange, PngCompression, PngOptions, Scale, Snippet, Style,
    TextRange,
};
pub use raster::{BgraImage, RasterImage};

use giallo::Registry;
use std::{
    borrow::Cow,
    collections::BTreeSet,
    fmt,
    sync::{Arc, OnceLock},
};
use takumi::prelude::Fonts;

/// Maximum source size in bytes, checked before CRLF/CR normalization.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
/// Maximum size of a custom font in bytes.
pub const MAX_FONT_BYTES: usize = 64 * 1024 * 1024;

/// A reusable native renderer with lazy grammar and bundled-font caches.
///
/// Construction is cheap and infallible. The first render loads the embedded
/// grammar registry; fonts prepare on first use. Clones share these caches.
/// Render methods accept `&self` and may run concurrently, including with
/// different fonts. Drop all clones to release the per-renderer caches.
///
/// Custom-font preparation belongs to the [`Font`] handle. Takumi also keeps
/// bounded process-wide glyph/shadow-mask caches and per-thread layout scratch
/// buffers. PNG downsampling shares a pool of at most four worker threads.
/// Dropping a renderer does not flush those; scratch buffers live until their
/// rendering thread exits. Applications should bound concurrent work.
#[derive(Clone, Default)]
pub struct Renderer(Arc<Inner>);
#[derive(Default)]
struct Inner {
    registry: OnceLock<Result<Registry, String>>,
    fonts: [font::PreparedFonts; font::BUNDLED_NAMES.len() * 2],
}
impl fmt::Debug for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Renderer")
            .field("initialized", &self.0.registry.get().is_some())
            .finish_non_exhaustive()
    }
}
impl Renderer {
    /// Create a renderer; no fonts or grammars are loaded until needed.
    pub fn new() -> Self {
        Self::default()
    }
    /// Render a lossless PNG. Accepts a [`Scale`] for Fast compression, or
    /// [`PngOptions`] to choose compression independently of the visual style.
    ///
    /// ```
    /// use monoprint::{PngCompression, PngOptions, Renderer, Scale, Snippet, Style};
    /// let options = PngOptions::new(Scale::X2).compression(PngCompression::Balanced);
    /// let png = Renderer::new().png(&Snippet::new("hello"), &Style::default(), options)?;
    /// # Ok::<(), monoprint::Error>(())
    /// ```
    pub fn png(
        &self,
        snippet: &Snippet,
        style: &Style,
        options: impl Into<PngOptions>,
    ) -> Result<Vec<u8>, Error> {
        let options = options.into();
        self.rgba(snippet, style, options.scale)?
            .encode_png(options.compression)
            .map_err(Error::backend)
    }
    /// Render owned, tightly packed RGBA8 pixels without image encoding.
    ///
    /// Uses the same layout, effects, scale and pixel limits as [`Self::png`].
    /// Alpha is straight (not premultiplied); pixels are row-major, top to bottom.
    /// This is suitable for native previews and image-processing pipelines.
    ///
    /// ```
    /// use monoprint::{Renderer, Scale, Snippet, Style};
    /// let image = Renderer::new().rgba(&Snippet::new("hello"), &Style::default(), Scale::X2)?;
    /// let (width, height) = (image.width(), image.height());
    /// let pixels = image.into_pixels(); // Moves the allocation; no pixel copy.
    /// assert_eq!(pixels.len(), width as usize * height as usize * 4);
    /// # Ok::<(), monoprint::Error>(())
    /// ```
    pub fn rgba(
        &self,
        snippet: &Snippet,
        style: &Style,
        scale: Scale,
    ) -> Result<RasterImage, Error> {
        match self.render(snippet, style, render::Format::Rgba, scale)? {
            render::Output::Raster(image) => Ok(RasterImage(image)),
            render::Output::Text(_) => unreachable!("raster output requested"),
        }
    }
    /// Render straight-alpha BGRA8 for native previews without a separate
    /// channel-swap pass. Uses the same scale, layout and limits as [`Self::rgba`].
    ///
    /// The channel order is produced during raster finalization. Take the
    /// allocation with [`BgraImage::into_pixels`] to hand it to the native UI.
    pub fn bgra(&self, snippet: &Snippet, style: &Style, scale: Scale) -> Result<BgraImage, Error> {
        match self.render(snippet, style, render::Format::Bgra, scale)? {
            render::Output::Raster(image) => Ok(BgraImage(image)),
            render::Output::Text(_) => unreachable!("raster output requested"),
        }
    }
    /// Render portable SVG with outlined text, CSS-pixel dimensions, and embedded
    /// textures where needed. No external resources, text, or `foreignObject`.
    pub fn svg(&self, snippet: &Snippet, style: &Style) -> Result<String, Error> {
        self.text_export(snippet, style, render::Format::Svg)
    }
    /// Render a standalone HTML document with selectable text and embedded fonts
    /// and images. Contains no JavaScript or external resources.
    pub fn html(&self, snippet: &Snippet, style: &Style) -> Result<String, Error> {
        self.text_export(snippet, style, render::Format::Html)
    }
    fn text_export(
        &self,
        snippet: &Snippet,
        style: &Style,
        format: render::Format,
    ) -> Result<String, Error> {
        match self.render(snippet, style, format, Scale::X2)? {
            render::Output::Text(text) => Ok(text),
            render::Output::Raster(_) => unreachable!("text output requested"),
        }
    }
    fn render(
        &self,
        snippet: &Snippet,
        style: &Style,
        format: render::Format,
        scale: Scale,
    ) -> Result<render::Output, Error> {
        let source = validate(snippet, style)?;
        render::render(self, snippet, style, &source, format, scale).map_err(Error::backend)
    }
    fn registry(&self) -> Result<&Registry, Error> {
        self.0
            .registry
            .get_or_init(|| {
                Registry::load(include_bytes!("../assets/registry.zst"))
                    .map_err(|e| format!("loading bundled grammars: {e}"))
            })
            .as_ref()
            .map_err(|e| Error::Render(RenderError(e.clone())))
    }
    fn fonts(
        &self,
        style: &Style,
        secondary: bool,
        fallbacks: fallback::Fallbacks,
    ) -> Result<Arc<Fonts>, Error> {
        if let (Some(comment_font), Some(cache)) = (&style.comment_font, &style.mixed_fonts) {
            return font::cached_fonts(cache, fallbacks, || {
                let mut fonts = self
                    .code_fonts(style, secondary, fallback::Fallbacks::default())?
                    .as_ref()
                    .clone();
                comment_font.register_as(&mut fonts, "CommentFont")?;
                Ok(fonts)
            });
        }
        self.code_fonts(style, secondary, fallbacks)
    }
    fn code_fonts(
        &self,
        style: &Style,
        secondary: bool,
        fallbacks: fallback::Fallbacks,
    ) -> Result<Arc<Fonts>, Error> {
        if let Some(font) = &style.font {
            return font.prepared(secondary, fallbacks);
        }
        let name = &style.theme.data().font;
        let index = font::BUNDLED_NAMES
            .iter()
            .position(|candidate| candidate == name)
            .expect("bundled theme font");
        font::cached_fonts(
            &self.0.fonts[index * 2 + usize::from(secondary)],
            fallbacks,
            || font::prepare(font::bundled_source(name), secondary),
        )
    }
}

fn validate<'a>(snippet: &'a Snippet, style: &Style) -> Result<Cow<'a, str>, Error> {
    if snippet.source.len() > MAX_SOURCE_BYTES {
        return Err(InvalidOption::SourceTooLarge {
            len: snippet.source.len(),
        }
        .into());
    }
    if style.padding > 512 {
        return Err(InvalidOption::Padding(style.padding).into());
    }
    if !(8..=72).contains(&style.font_size) {
        return Err(InvalidOption::FontSize(style.font_size).into());
    }
    if let Some(width) = style.width {
        let min = 80.max(style.padding * 2 + 49);
        if width < min || width > 8192 {
            return Err(InvalidOption::Width {
                width,
                min,
                max: 8192,
            }
            .into());
        }
    }
    if !style.theme.supports(style.appearance) {
        return Err(InvalidOption::Appearance {
            theme: style.theme,
            appearance: style.appearance,
        }
        .into());
    }
    if !snippet.subtitle.is_empty() && style.theme.id() != "removed-frame" {
        return Err(InvalidOption::Subtitle { theme: style.theme }.into());
    }
    let source = if snippet.source.contains('\r') {
        Cow::Owned(snippet.source.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(snippet.source.as_str())
    };
    let line_count = source.split('\n').count();
    for range in &snippet.highlights {
        let (start, end) = (*range.0.start(), *range.0.end());
        if start == 0 || range.0.is_empty() || end > line_count {
            return Err(InvalidOption::HighlightedLines {
                start,
                end,
                line_count,
            }
            .into());
        }
    }
    for range in &snippet.text_highlights {
        range.resolve(&source)?;
    }
    Ok(source)
}
fn highlighted_lines(snippet: &Snippet) -> BTreeSet<usize> {
    snippet
        .highlights
        .iter()
        .flat_map(|range| range.0.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_languages_resolve_in_the_embedded_registry() {
        let renderer = Renderer::new();
        let registry = renderer.registry().unwrap();
        for language in Language::all() {
            assert!(registry.contains_grammar(language.id()), "{language}");
            for alias in language.aliases() {
                assert!(
                    registry.contains_grammar(alias.parse::<Language>().unwrap().id()),
                    "{alias}"
                );
            }
        }
    }

    #[test]
    fn hem_keywords_use_their_own_grammar_scopes() {
        let renderer = Renderer::new();
        let registry = renderer.registry().unwrap();
        let code = registry
            .highlight(
                "effect resource use defer raise match greeting string \"hello\" // café",
                &giallo::HighlightOptions::new(
                    "hem",
                    giallo::ThemeVariant::Single("editor-github-dark-default"),
                )
                .merge_whitespace(false),
            )
            .unwrap();
        let foreground = |word: &str| {
            let token = code
                .tokens
                .iter()
                .flatten()
                .find(|token| token.text.trim() == word)
                .unwrap_or_else(|| panic!("missing Hem token {word}"));
            let giallo::ThemeVariant::Single(style) = token.style else {
                panic!("expected a single theme")
            };
            style.foreground
        };
        let keyword = foreground("effect");
        for word in ["resource", "use", "defer", "raise", "match"] {
            assert_eq!(foreground(word), keyword, "{word}");
        }
        assert_ne!(keyword, foreground("greeting"));
        assert_ne!(keyword, foreground("\"hello\""));
        assert_ne!(keyword, foreground("// café"));
    }

    #[test]
    fn embedded_registry_obeys_the_grammar_license_policy() {
        let renderer = Renderer::new();
        let registry = renderer.registry().unwrap();
        let policy: serde_json::Value =
            serde_json::from_str(include_str!("../licenses/grammar-policy.json")).unwrap();
        for entry in policy["entries"].as_array().unwrap() {
            let name = entry["name"].as_str().unwrap();
            let included = entry["status"] == "included";
            assert_eq!(registry.contains_grammar(name), included, "{name}");
            if !included {
                assert!(name.parse::<Language>().is_err(), "{name}");
            }
        }
    }
}
