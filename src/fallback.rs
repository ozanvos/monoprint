//! Deterministic, embedded coverage fallbacks, separate from the typeface picker.
use crate::{Error, TextRole};
use icu_properties::{CodePointSetData, props::EmojiPresentation};
use std::sync::{Arc, OnceLock};
use takumi::prelude::{FontOverride, FontResource, FontSource, Fonts, GenericFamily};

struct Entry {
    family: &'static str,
    bytes: &'static [u8],
    face: OnceLock<ttf_parser::Face<'static>>,
    emoji: bool,
}

impl Entry {
    fn face(&self) -> &ttf_parser::Face<'static> {
        self.face
            .get_or_init(|| ttf_parser::Face::parse(self.bytes, 0).expect("valid bundled fallback"))
    }
}

macro_rules! entry {
    ($family:literal, $file:literal, $emoji:literal) => {
        Entry {
            family: $family,
            bytes: include_bytes!(concat!("../assets/fonts/fallback/", $file)),
            face: OnceLock::new(),
            emoji: $emoji,
        }
    };
}

// The bit positions are stable; presentation-aware selection below decides
// between monochrome symbols and color glyphs without changing the source.
static ENTRIES: [Entry; 8] = [
    entry!("MonoprintSans", "noto-sans.ttf", false),
    entry!("MonoprintEmoji", "noto-color-emoji.ttf", true),
    entry!("MonoprintCjk", "noto-sans-cjk-sc.otf", false),
    entry!("MonoprintSymbols", "noto-sans-symbols2.ttf", false),
    entry!("MonoprintArabic", "noto-sans-arabic.ttf", false),
    entry!("MonoprintDevanagari", "noto-sans-devanagari.ttf", false),
    entry!("MonoprintHebrew", "noto-sans-hebrew.ttf", false),
    entry!("MonoprintThai", "noto-sans-thai.ttf", false),
];
const EMOJI: u8 = 1 << 1;

/// Only the coverage fonts required by a render request.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fallbacks(u8);

/// Format controls participate in shaping but do not need their own glyph.
pub(crate) fn is_shaping_control(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r' | '\t' | '\u{00ad}' | '\u{061c}' | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}'
            | '\u{fe00}'..='\u{fe0f}' | '\u{feff}' | '\u{e0020}'..='\u{e007f}'
            | '\u{e0100}'..='\u{e01ef}'
    )
}

impl Fallbacks {
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Validate coverage and retain the smallest ordered set needed by the text.
    /// Shaping still belongs to Takumi/Parley, which selects whole graphemes.
    pub(crate) fn include(
        &mut self,
        primary: &[u8],
        text: &str,
        role: TextRole,
        code_font: bool,
    ) -> Result<(), Error> {
        let primary = ttf_parser::Face::parse(primary, 0).map_err(|_| Error::InvalidFont)?;
        let mut characters = text.chars().peekable();
        let emoji_presentation = CodePointSetData::new::<EmojiPresentation>();
        while let Some(character) = characters.next() {
            if character == '\u{fe0f}' {
                self.0 |= EMOJI;
            }
            if is_shaping_control(character)
                || primary
                    .glyph_index(character)
                    .is_some_and(|glyph| glyph.0 != 0)
            {
                continue;
            }
            let prefer_emoji = character == '\u{20e3}'
                || characters.peek() == Some(&'\u{fe0f}')
                || (characters.peek() != Some(&'\u{fe0e}')
                    && emoji_presentation.contains(character));
            let preferred =
                (0..ENTRIES.len()).filter(|&index| ENTRIES[index].emoji == prefer_emoji);
            let remaining =
                (0..ENTRIES.len()).filter(|&index| ENTRIES[index].emoji != prefer_emoji);
            let Some(index) = preferred.chain(remaining).find(|&index| {
                ENTRIES[index]
                    .face()
                    .glyph_index(character)
                    .is_some_and(|glyph| glyph.0 != 0)
            }) else {
                return Err(Error::UnsupportedGlyph {
                    text: role,
                    character,
                    code_font,
                });
            };
            self.0 |= 1 << index;
        }
        Ok(())
    }

    pub(crate) fn families(self) -> String {
        self.entries()
            .map(|(family, _)| family)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Bytes are shared with native registration and self-contained HTML.
    pub(crate) fn entries(self) -> impl Iterator<Item = (&'static str, &'static [u8])> {
        // Plain text symbols retain text presentation when another part of the
        // request also needs Emoji. Explicit VS15/VS16 are resolved by Takumi.
        ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.emoji)
            .chain(ENTRIES.iter().enumerate().filter(|(_, entry)| entry.emoji))
            .filter(move |(index, _)| self.0 & (1 << index) != 0)
            .map(|(_, entry)| (entry.family, entry.bytes))
    }

    pub(crate) fn register(self, fonts: &mut Fonts) -> anyhow::Result<()> {
        for (index, entry) in ENTRIES.iter().enumerate() {
            if self.0 & (1 << index) == 0 {
                continue;
            }
            let resource = FontResource::new(FontSource::from_static(entry.bytes)).override_info(
                FontOverride {
                    family_name: Some(Arc::from(entry.family)),
                    ..Default::default()
                },
            );
            let resource = if entry.emoji {
                resource.generic_family(GenericFamily::EMOJI)
            } else {
                resource
            };
            fonts.register(resource)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_is_lazy_role_aware_and_handles_sequence_controls() {
        let primary = crate::assets::font("space-mono");
        let mut fallback = Fallbacks::default();
        fallback
            .include(primary, "ordinary code", TextRole::Code, true)
            .unwrap();
        assert!(fallback.is_empty());
        fallback
            .include(primary, "👩🏽‍💻 🇳🇱 1️⃣", TextRole::Comment, false)
            .unwrap();
        assert_eq!(fallback.families(), "MonoprintEmoji");
        fallback
            .include(primary, "Привет", TextRole::Code, true)
            .unwrap();
        assert_eq!(fallback.families(), "MonoprintSans,MonoprintEmoji");
        assert!(matches!(
            fallback.include(primary, "\u{10ffff}", TextRole::Title, false),
            Err(Error::UnsupportedGlyph {
                text: TextRole::Title,
                character: '\u{10ffff}',
                code_font: false
            })
        ));
    }

    #[test]
    fn requested_emoji_presentation_loads_color_font_even_for_covered_base() {
        let primary = crate::assets::font("jetbrains-mono");
        let mut fallback = Fallbacks::default();
        fallback
            .include(primary, "1\u{fe0f}\u{20e3}", TextRole::Code, true)
            .unwrap();
        assert_eq!(fallback.families(), "MonoprintEmoji");
    }

    #[test]
    fn notdef_cmap_mappings_do_not_count_as_coverage() {
        let primary = crate::assets::font("jetbrains-mono");
        // Format 4 cmap sentinel entries may explicitly map U+FFFF to glyph 0.
        // Both the selected face and fallback faces must reject .notdef.
        assert_eq!(
            ttf_parser::Face::parse(primary, 0)
                .unwrap()
                .glyph_index('\u{ffff}'),
            Some(ttf_parser::GlyphId(0))
        );
        assert!(
            ENTRIES.iter().any(|entry| {
                entry.face().glyph_index('\u{ffff}') == Some(ttf_parser::GlyphId(0))
            })
        );
        assert!(matches!(
            Fallbacks::default().include(primary, "\u{ffff}", TextRole::Comment, false),
            Err(Error::UnsupportedGlyph {
                text: TextRole::Comment,
                character: '\u{ffff}',
                code_font: false,
            })
        ));
    }

    #[test]
    fn selectors_choose_available_text_or_color_faces_without_replacing_source() {
        let primary = crate::assets::font("space-mono");
        let mut plain = Fallbacks::default();
        plain
            .include(primary, "❤\u{fe0e}", TextRole::Code, true)
            .unwrap();
        assert_eq!(plain.families(), "MonoprintSymbols");
        let mut color = Fallbacks::default();
        color.include(primary, "❤️", TextRole::Code, true).unwrap();
        assert_eq!(color.families(), "MonoprintEmoji");
        color
            .include(primary, "❤\u{fe0e}", TextRole::Code, true)
            .unwrap();
        assert_eq!(color.families(), "MonoprintSymbols,MonoprintEmoji");
        let mut defaults = Fallbacks::default();
        defaults
            .include(primary, "⌚", TextRole::Code, true)
            .unwrap();
        assert_eq!(defaults.families(), "MonoprintEmoji");
    }
}
