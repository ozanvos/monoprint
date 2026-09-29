use crate::{Error, InvalidOption, MAX_FONT_BYTES, RenderError, assets, fallback::Fallbacks};
use std::{
    collections::VecDeque,
    fmt,
    str::FromStr,
    sync::{Arc, Mutex, OnceLock},
};
use takumi::prelude::{FontOverride, FontResource, FontSource, Fonts};

/// One lazily prepared base set and a small LRU of requested coverage subsets.
#[derive(Default)]
pub(crate) struct PreparedFonts {
    base: OnceLock<Result<Arc<Fonts>, String>>,
    fallbacks: Mutex<VecDeque<(Fallbacks, Arc<Fonts>)>>,
}

const TYPEFACES: [(&str, &str); 10] = [
    ("commit-mono", "Commit Mono"),
    ("fira-code", "Fira Code"),
    ("geist-mono", "Geist Mono"),
    ("google-sans-code", "Google Sans Code"),
    ("ibm-plex-mono", "IBM Plex Mono"),
    ("inter", "Inter"),
    ("jetbrains-mono", "JetBrains Mono"),
    ("roboto-mono", "Roboto Mono"),
    ("source-code-pro", "Source Code Pro"),
    ("space-mono", "Space Mono"),
];

/// A shipped typeface. Parse an ID or discover choices with [`BundledFont::all`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BundledFont(usize);
impl BundledFont {
    /// All ten bundled typefaces, ordered by ID; no font data is loaded.
    pub fn all() -> impl ExactSizeIterator<Item = Self> {
        (0..TYPEFACES.len()).map(Self)
    }
    /// Stable lowercase ID, such as `fira-code`.
    pub fn id(self) -> &'static str {
        TYPEFACES[self.0].0
    }
    /// Human-readable family name.
    pub fn name(self) -> &'static str {
        TYPEFACES[self.0].1
    }
    /// Load a reusable in-memory font for [`crate::Style::font`] or
    /// [`crate::Style::comment_font`]. Repeated calls share font bytes and caches.
    pub fn font(self) -> Font {
        static FONTS: [OnceLock<Font>; TYPEFACES.len()] =
            [const { OnceLock::new() }; TYPEFACES.len()];
        FONTS[self.0]
            .get_or_init(|| {
                Font::from_bytes(assets::font(self.id())).expect("valid bundled typeface")
            })
            .clone()
    }
}
impl FromStr for BundledFont {
    type Err = Error;
    fn from_str(id: &str) -> Result<Self, Error> {
        Self::all()
            .find(|font| font.id().eq_ignore_ascii_case(id))
            .ok_or_else(|| Error::UnknownFont(id.to_owned()))
    }
}
impl fmt::Display for BundledFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// An in-memory TTF or OTF typeface, reusable for code or comments.
///
/// Clones share bytes and two lazily prepared base font sets, each caching at
/// most eight recently used fallback combinations. Dropping all
/// clones releases those resources; custom fonts never enter the renderer's
/// bundled-font cache. No filesystem or system-font lookup takes place.
/// Styles with a separate comment typeface also cache their combined font set.
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
    pub(crate) fn register_as(&self, fonts: &mut Fonts, name: &str) -> anyhow::Result<()> {
        fonts.register(
            FontResource::new(FontSource::from_shared(Arc::new(self.0.bytes.clone())))
                .override_info(FontOverride {
                    family_name: Some(Arc::from(name)),
                    ..Default::default()
                }),
        )?;
        Ok(())
    }
    pub(crate) fn prepared(
        &self,
        secondary: bool,
        fallbacks: Fallbacks,
    ) -> Result<Arc<Fonts>, Error> {
        cached_fonts(&self.0.prepared[usize::from(secondary)], fallbacks, || {
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
    fallbacks: Fallbacks,
    init: impl FnOnce() -> anyhow::Result<Fonts>,
) -> Result<Arc<Fonts>, Error> {
    let base = cell
        .base
        .get_or_init(|| {
            init()
                .map(Arc::new)
                .map_err(|e| format!("preparing fonts: {e:#}"))
        })
        .as_ref()
        .map_err(|message| Error::Render(RenderError(message.clone())))?;
    if fallbacks.is_empty() {
        return Ok(base.clone());
    }
    let mut cache = cell
        .fallbacks
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if let Some(index) = cache.iter().position(|(key, _)| *key == fallbacks) {
        let entry = cache.remove(index).expect("located cached fonts");
        let fonts = entry.1.clone();
        cache.push_back(entry);
        return Ok(fonts);
    }
    let mut fonts = base.as_ref().clone();
    fallbacks.register(&mut fonts).map_err(|error| {
        Error::Render(RenderError(format!("preparing fallback fonts: {error:#}")))
    })?;
    let fonts = Arc::new(fonts);
    if cache.len() == 8 {
        cache.pop_front();
    }
    cache.push_back((fallbacks, fonts.clone()));
    Ok(fonts)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextRole;

    #[test]
    fn fallback_cache_is_bounded_and_keeps_in_flight_font_sets_alive() {
        let cache = PreparedFonts::default();
        let base = cached_fonts(&cache, Fallbacks::default(), || Ok(Fonts::default())).unwrap();
        let base_again = cached_fonts(&cache, Fallbacks::default(), || unreachable!()).unwrap();
        assert!(Arc::ptr_eq(&base, &base_again));
        assert!(cache.fallbacks.lock().unwrap().is_empty());

        let mut keys = Vec::new();
        for text in [
            "Привет",
            "🦀",
            "中文",
            "❤\u{fe0e}",
            "مرحبا",
            "नमस्ते",
            "שלום",
            "สวัสดี",
            "Привет 🦀",
        ] {
            let mut key = Fallbacks::default();
            key.include(assets::font("space-mono"), text, TextRole::Code, true)
                .unwrap();
            assert!(
                !keys.contains(&key),
                "test inputs must exercise nine distinct subsets"
            );
            keys.push(key);
        }
        let held = cached_fonts(&cache, keys[0], || unreachable!()).unwrap();
        for key in &keys[1..] {
            cached_fonts(&cache, *key, || unreachable!()).unwrap();
        }
        assert_eq!(cache.fallbacks.lock().unwrap().len(), 8);
        let rebuilt = cached_fonts(&cache, keys[0], || unreachable!()).unwrap();
        assert!(!Arc::ptr_eq(&held, &rebuilt));
        // Eviction removes only the cache's Arc; the ongoing render still owns
        // a usable font registry and can take another immutable snapshot.
        let _ = held.snapshot();
        assert_eq!(cache.fallbacks.lock().unwrap().len(), 8);
        let reused = cached_fonts(&cache, keys[0], || unreachable!()).unwrap();
        assert!(Arc::ptr_eq(&rebuilt, &reused));
    }
}
