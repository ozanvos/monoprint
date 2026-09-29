# Bundled glyph fallback fonts

Monoprint's automatic fallback inventory contains eight unmodified static Noto
fonts under SIL OFL-1.1. It is separate from the ten selectable code typefaces:
fallback faces supply missing glyphs, rather than becoming additional choices
in the typeface picker. The files add **28,917,868 bytes (27.58 MiB)** of raw
assets. This is not a measured executable-size or memory increase.

## Rendering behavior

Fallback is automatic for code, separate comment fonts, and visible frame titles.
The requested typeface wins wherever it supplies a character. Missing characters
use the smallest required set of embedded fallback faces; no installed fonts or
network access are needed. A character absent from every face still returns
`UnsupportedGlyph` with its code point and text role.

PNG renders color emoji directly. SVG keeps ordinary text as outlines and embeds
emoji bitmap glyphs; it does not use `foreignObject`. HTML embeds the required
font bytes and preserves the source text, so multilingual HTML files can grow by
tens of megabytes. Browser shaping and antialiasing may differ from native output.
Emoji are supported with coding ligatures both enabled and disabled, including
skin-tone modifiers, ZWJ sequences, flags and keycaps supported by the pinned font.
Explicit text (VS15) and emoji (VS16) presentation selectors are respected.
Unqualified symbols can remain monochrome when the selected typeface or another
required text fallback covers them. For example, `⌚ ❤︎` can use text outlines,
whereas `⌚️ ❤︎` requests a color watch and a text heart.

Automatic width uses the renderer's text shaper for fallback and non-ASCII text,
including combining marks and joined emoji. The original ASCII path is retained.
Font collections are prepared lazily and cached in an eight-entry LRU per base
font set; font bytes are shared. The fallback fonts are embedded in the executable
even when a particular render does not use them.

## Assets and coverage

| Face | Raw bytes | Unicode cmap entries | Intended coverage |
| --- | ---: | ---: | --- |
| Noto Color Emoji | 10,730,124 | 1,508 | Color emoji; GSUB contains supported emoji sequence substitutions |
| Noto Sans CJK SC Regular | 16,437,364 | 44,810 | Han, Japanese kana and Korean Hangul |
| Noto Sans Regular | 569,208 | 2,838 | Latin, Greek and Cyrillic |
| Noto Sans Symbols2 Regular | 656,852 | 2,623 | Supplementary symbols |
| Noto Sans Arabic Regular | 240,456 | 1,159 | Arabic script |
| Noto Sans Devanagari Regular | 219,212 | 270 | Devanagari script |
| Noto Sans Hebrew Regular | 26,900 | 143 | Hebrew script |
| Noto Sans Thai Regular | 37,752 | 99 | Thai script |

Counts are distinct mapped Unicode scalar values reported by Fontconfig for the
exact files, not glyph counts or counts of supported emoji sequences. Cmap
coverage alone does not prove correct shaping, bidirectional layout or every
combining-mark sequence. This is a useful fallback set, not all of Unicode:
additional scripts and rare ideographs can still be absent.

The full CJK font was chosen over the smaller 8,331,336-byte Noto Sans SC regional
subset. Both cover the Chinese and Japanese sample strings, but the subset lacks
the Korean sample. The bundled full font covers `你好世界繁體中文`,
`日本語こんにちはカタカナ`, and `한국어 안녕하세요`. Its default Han forms are
Simplified Chinese; sharing a code point does not make a glyph's regional shape
identical in Chinese, Japanese and Korean. The upstream
[CJK deployment guide](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/README.md)
explains language-specific fonts and localized forms.

The other sample strings checked against the font maps are
`İstanbul naïve café ǽ ẞ`, `Ελληνικά русский`, `مرحبا بالعالم`, `नमस्ते दुनिया`,
`שלום עולם`, and `สวัสดีโลก`. Symbols2 is not a complete math or emoji font;
symbol coverage is shared across the selected code face and fallback inventory.
Emoji mapping checks include `😀🚀👍🏽👩💻🇳🇱`; shaping a modifier, flag or ZWJ
sequence remains a renderer-level responsibility.

## Why the bitmap emoji font

The selected Noto Color Emoji is version 2.057, with CBDT/CBLC color bitmap data,
GSUB substitutions and normal OpenType metrics. Its bitmap strike is 109 ppem
at 32-bit color. It has no monochrome glyph outlines. Large exports therefore
scale bitmap emoji rather than obtaining infinitely sharp vector artwork.

Noto-COLRv1 at the same upstream revision is smaller (5,038,236 bytes), but the
current renderer's color-font path supports bitmap glyphs and COLRv0 rather
than COLRv1. It was not bundled. The approved asset keeps the original font
bytes instead of converting, subsetting or manufacturing a new font. The
[upstream Noto Emoji project](https://github.com/googlefonts/noto-emoji/tree/e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e)
supplies both versions; the pinned bitmap file is under `2D/fonts/`.

## Provenance and licensing

The [separate manifest](../assets/fonts/fallback/manifest.json) records every
immutable download URL, Git revision and blob ID, SHA-256, byte count, font
version, original copyright, and exact license URL/hash. It pins:

- Noto Emoji to `googlefonts/noto-emoji` commit
  `e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e`.
- Noto Sans CJK to `notofonts/noto-cjk` commit
  `f8d157532fbfaeda587e826d4cd5b21a49186f7c` (font version 2.004).
- The six smaller script/symbol fonts to the official static builds in
  `notofonts/noto-fonts` commit `ffebf8c1ee449e544955a7e813c54f9b73848eac`.
  This is the older 2023 aggregate repository; these are deliberately pinned
  stable files, not a claim to use the latest release of every Noto family.

The files are copied exactly: no subsetting, internal renaming or format
conversion. Keep [NOTICE.txt](../assets/fonts/fallback/NOTICE.txt), the three
full `*-OFL.txt` files, and the manifest with software distributions. Copyright
and license metadata inside every font remains intact.

OFL font use does not require a watermark or visible attribution in an exported
image, and it does not impose the font license on the generated document. The
OFL's author explains document embedding, metadata and redistribution separately
in [FAQ 1.10–1.15](https://openfontlicense.org/ofl-faq/). Distribution notices
belong with the project and packaged software; this inventory does not add
visible attribution to code images.

## Reproduce and verify

The maintenance importer does not discover newer revisions. It reads the
reviewed manifest and refuses downloaded bytes that differ from its checksums:

```sh
# Network access: restore the exact fonts and licenses from pinned sources.
python3 scripts/fetch-fallback-fonts.py

# Offline: verify font, license, Git blob and notice bytes.
python3 scripts/fetch-fallback-fonts.py --check
```

Normal rendering and builds do not run the downloader. The importer is separate
from `fetch-fonts.py`, which manages selectable typefaces. A future font update
requires explicit manifest/license review and renderer-level coverage checks;
hash equality only establishes that the reviewed assets have not changed.

## Local acceptance and size

On macOS arm64, the stripped release CLI grew from **17.03 MiB to 48.54 MiB**
with these fonts and the shaper's CJK/Thai segmentation data enabled. The native
Node addon is **48.08 MiB**. These are executable sizes from the 2026-09-29 local
build, not compressed download sizes or memory measurements.

The fallback regression suite covers color/ZWJ/modifier/flag/keycap sequences,
text and emoji selectors, mixed scripts and title/comment roles, wrapping,
selection geometry, combining marks, cache isolation and unsupported characters.
Local acceptance also checked the Node/CLI byte contract, a clean npm tarball
consumer, and the native GUI at 1440×900 and 1100×700. A self-contained multilingual
HTML export was inspected in Chrome 153; other browser engines remain unverified.
Existing ASCII fixtures remained byte-identical across 116 exports covering all
96 theme appearances.

A small interleaved fresh-process ASCII PNG check (four Rust lines, 720px width,
2× scale; two warmups and 21 measured runs per binary) measured **43.52 ms before**
and **43.90 ms after**. This local sanity check does not replace the broader
competitor benchmark or establish performance on other machines.

```sh
cargo test --locked --test glyph_fallback
python3 scripts/fetch-fallback-fonts.py --check
```
