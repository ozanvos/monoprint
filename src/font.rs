use crate::{Error, InvalidOption, MAX_FONT_BYTES, RenderError, assets};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};
use takumi::prelude::{FontOverride, FontResource, FontSource, Fonts};

pub(crate) type PreparedFonts = OnceLock<Result<Fonts, String>>;

/// An in-memory TTF or OTF code font, reusable across styles and renderers.
///
/// Clones share bytes and at most two lazily prepared font sets. Dropping all
/// clones releases those resources; custom fonts never enter the renderer's
/// bundled-font cache. No filesystem or system-font lookup takes place.
#[derive(Clone)]
pub struct Font(Arc<Inner>);
struct Inner {
    bytes: Arc<[u8]>,
    prepared: [PreparedFonts; 2],
}
impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("bytes", &self.0.bytes.len())
            .finish_non_exhaustive()
    }
}
impl Font {
    /// Validate a single-face TTF/OTF font (at most 64 MiB).
    ///
    /// Accepts a byte slice, `Vec<u8>`, or `Arc<[u8]>`. A slice is copied;
    /// passing an `Arc` shares its allocation. Registration is deferred to the
    /// first render, and errors from registration are reported there.
    /// Custom code fonts render at weight 400. Collections and WOFF/WOFF2 are
    /// not accepted. To embed a font, pass `&include_bytes!("font.ttf")[..]`.
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Result<Self, Error> {
        let bytes = bytes.into();
        if bytes.len() > MAX_FONT_BYTES {
            return Err(InvalidOption::FontTooLarge { len: bytes.len() }.into());
        }
        if bytes.starts_with(b"ttcf") {
            return Err(Error::InvalidFont);
        }
        ttf_parser::Face::parse(&bytes, 0).map_err(|_| Error::InvalidFont)?;
        Ok(Self(Arc::new(Inner {
            bytes,
            prepared: Default::default(),
        })))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
    pub(crate) fn prepared(&self, secondary: bool) -> Result<&Fonts, Error> {
        cached_fonts(&self.0.prepared[usize::from(secondary)], || {
            prepare(
                FontSource::from_shared(Arc::new(self.0.bytes.clone())),
                secondary,
            )
        })
    }
}

pub(crate) const BUNDLED_NAMES: [&str; 9] = [
    "commit-mono",
    "fira-code",
    "geist-mono",
    "google-sans-code",
    "ibm-plex-mono",
    "jetbrains-mono",
    "roboto-mono",
    "source-code-pro",
    "space-mono",
];

pub(crate) fn cached_fonts(
    cell: &PreparedFonts,
    init: impl FnOnce() -> anyhow::Result<Fonts>,
) -> Result<&Fonts, Error> {
    cell.get_or_init(|| init().map_err(|e| format!("preparing fonts: {e:#}")))
        .as_ref()
        .map_err(|message| Error::Render(RenderError(message.clone())))
}
pub(crate) fn bundled_source(name: &str) -> FontSource<'static> {
    FontSource::from_shared(Arc::new(assets::font(name)))
}
pub(crate) fn prepare(source: FontSource<'_>, secondary: bool) -> anyhow::Result<Fonts> {
    let mut fonts = Fonts::default();
    fonts.register(FontResource::new(source).override_info(FontOverride {
        family_name: Some(Arc::from("CodeMono")),
        ..Default::default()
    }))?;
    fonts.register(
        FontResource::new(bundled_source("inter")).override_info(FontOverride {
            family_name: Some(Arc::from("Inter")),
            ..Default::default()
        }),
    )?;
    if secondary {
        fonts.register(
            FontResource::new(bundled_source("ibm-plex-mono")).override_info(FontOverride {
                family_name: Some(Arc::from("IBM Plex Mono")),
                ..Default::default()
            }),
        )?;
    }
    Ok(fonts)
}
