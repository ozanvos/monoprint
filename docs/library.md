# Rust library

Monoprint renders code to PNG bytes, SVG text, or standalone HTML. `Renderer` owns reusable resources; `Snippet` holds content; `Style` holds the appearance. The CLI uses this same API.

## Use the local crate

```toml
[dependencies]
monoprint = { path = "../monoprint", default-features = false }
```

Adjust the path to your checkout. Rust 1.97+ and a C compiler are required. Disabling default features removes clap and the executable, while retaining all themes, grammars, and formats.

The default style is **Monoprint**, a bordered Masthead composition with restrained oxide and umber syntax. It supports both dark (default) and light appearances under the `monoprint` ID. Use `Style::default().appearance(Appearance::Light)` for its light counterpart.

The library is usable as a path dependency. **Registry publication is disabled** until the vendored raster and SVG changes are available in published dependencies (upstream or maintained forks). Monoprint directly depends on its patched backends; callers do not need a `[patch]` section. A root-only Cargo patch would silently lose the fixes downstream. Git distribution should also retain these path dependencies, but has not been tested against a published remote.

## One image

```rust
use monoprint::{Renderer, Scale, Snippet, Style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let png = Renderer::new().png(
        &Snippet::new("fn main() {}\n")
            .language("rust".parse()?)
            .title("main.rs"),
        &Style::default(),
        Scale::X2,
    )?;
    std::fs::write("snippet.png", png)?;
    Ok(())
}
```

The default style is Monoprint dark, 32px padding, automatic width (at least 520px), the theme's font at a nominal 15px size, and its default line-number behavior. PNG scale is explicit; `Scale::default()` is 4×. `svg()` and `html()` accept only the snippet and style and return `String` at CSS dimensions.

Passing a `Scale` uses Fast lossless compression, preserving existing output.
For smaller final exports, pass `PngOptions` instead:

```rust
use monoprint::{PngCompression, PngOptions, Renderer, Scale, Snippet, Style};

fn main() -> Result<(), monoprint::Error> {
    let options = PngOptions::new(Scale::X2).compression(PngCompression::Balanced);
    let png = Renderer::new().png(&Snippet::new("let answer = 42;"), &Style::default(), options)?;
    std::fs::write("snippet.png", png).unwrap();
    Ok(())
}
```

Both modes preserve exactly the same pixels. Balanced spends more time encoding
to reduce file size; use Fast when a preview needs encoded PNG. `PngOptions::default()` uses
4× and Fast. Compression is an output setting, independent of `Style`.
Balanced compression preserves pixels while trading more encoding time for smaller PNGs.

## Batch rendering

Reuse one renderer to retain native lexer arenas, the fallback grammar registry, compiled grammar patterns, and prepared bundled fonts. Clone a style to derive a variation without changing the original.

Monoprint uses [pierre-highlights-rs 0.1.0](https://crates.io/crates/pierre-highlights-rs/0.1.0)
from crates.io. The highlighter is maintained in its
[own repository](https://github.com/ozanvos/pierre-highlights-rs).

Syntax highlighting defaults to native Pierre for supported languages, with Giallo
for the remaining catalog entries. The full language picker and aliases remain
available. Python with a separate comment font also uses Giallo to preserve
TextMate's docstring classification. Existing theme palettes are mapped to Pierre's
token kinds; syntax classification can differ from TextMate.

Use `Renderer::new().highlight_backend(HighlightBackend::Giallo)` to retain the
original backend explicitly. Clones share caches, while each handle retains its
backend policy. `Renderer::highlight_source(source, language, appearance)` exposes
styled UTF-8 byte ranges for native source editors using the GitHub light/dark
palettes; it preserves original CRLF/CR offsets and enforces the source-size limit.

After regenerating `assets/registry.zst`, run
`cargo run --release --example compile_pierre_palettes` to update the native tables.
The `--check` option verifies all 80 palettes against Giallo's theme matcher and the
reviewed mapping in `assets/pierre-token-scopes.json`.

```rust
use monoprint::{Appearance, Language, LineNumbers, Renderer, Snippet, Style};

fn main() -> Result<(), monoprint::Error> {
    let renderer = Renderer::new();
    let style = Style::new("monoprint".parse()?)
        .appearance(Appearance::Light)
        .padding(32)
        .width(720)
        .line_numbers(LineNumbers::Show);

    for (name, source) in [("a.rs", "let a = 1;"), ("b.py", "b = 2")] {
        let snippet = Snippet::new(source)
            .language(Language::for_file_name(name).unwrap_or_default())
            .title(name)
            .highlight(1);
        let svg = renderer.svg(&snippet, &style)?;
        // Save or return `svg` here.
        assert!(svg.contains("<svg"));
    }
    Ok(())
}
```

`width()` wraps code to a fixed outer width. `fit_width()` restores automatic sizing. `highlight(2)` and `highlight(3..=5)` add one-based logical source lines; repeated selections merge. `clear_highlights()` resets a selection while retaining the source. A final newline adds a logical line. Invalid or reversed ranges fail before expansion. Plain text never shows line numbers. Every theme permits `LineNumbers::Show` and `LineNumbers::Hide`; `Auto` follows the theme's default, exposed by `Theme::line_numbers()`.

Use `highlight_text(TextRange::new((2, 5), (2, 12)))` for characters 5 through 11
of line 2, or parse `"2:5-2:12"` as a `TextRange`. Lines and columns are one-based;
columns count Unicode scalar characters, including each tab as one character.
CRLF/CR normalize to LF first. Selections may span lines, overlapping selections
merge, and empty/reversed/out-of-bounds coordinates fail when rendered.
`clear_highlights()` removes both line and text highlights.

The title is a label, not a filename hint. Use `Language::for_file_name()` explicitly; it does not read the file. Unknown filenames return `None`; parsing an unknown explicit language returns an error. TypeScript aliases (`ts`, `mts`, `cts`) select TypeScript; TSX remains separate. TeX selects LaTeX. `Language::all()` lists these unique selections (231 highlighting languages plus plain text, backed by 232 bundled named grammars).

`Theme::all()` lists 69 curated presets; matching light/dark appearances appear once.
For example, `Style::new("ayu".parse()?).appearance(Appearance::Light)` selects
Ayu Light, including its frame and background. Original IDs still work:
`Style::new("ayu-light".parse()?)` keeps its light default. `Theme::aliases()`
lists legacy palette IDs; `Theme::canonical()` finds the paired preset. Preserve
the appearance separately when canonicalizing saved settings. Distinct dark-only
flavors remain separate presets, and unsupported appearances still fail.

## Embedded previews

`html()` produces a complete document with selectable text, embedded fonts and assets, and no scripts or external requests. Pass it to your webview/document loading API. Use `png()` for an image-based preview or `svg()` when selectable text is unnecessary.

Native previews can skip encoding and decoding with `Renderer::rgba()`:

```rust
use monoprint::{Renderer, Scale, Snippet, Style};

fn main() -> Result<(), monoprint::Error> {
    let image = Renderer::new().rgba(&Snippet::new("let answer = 42;"), &Style::default(), Scale::X2)?;
    let (width, height) = (image.width(), image.height());
    let pixels = image.into_pixels();
    assert_eq!(pixels.len(), width as usize * height as usize * 4);
    Ok(())
}
```

`RasterImage` owns tightly packed RGBA8 pixels, with straight alpha, top-to-bottom
rows and no row padding. `pixels()` borrows the bytes; `into_pixels()` moves the
allocation without copying. Layout, effects, odd-scale downsampling and limits
match PNG exactly. When a native API expects BGRA, call `Renderer::bgra()` instead:
it returns a distinct `BgraImage` with the same width, height, pixels and
`into_pixels()` methods. The rasterizer combines alpha conversion and channel
order in one finalization pass. Studio transfers this finished allocation to
GPUI without another pixel traversal. Odd-scale resampling preserves BGRA order.

`RasterImage::into_bgra_pixels()` remains available for callers that already have
RGBA data; that method performs a separate in-place swap. Both APIs retain
straight alpha. GPUI still uploads the buffer to its texture atlas; this is not
zero-copy GPU presentation. Compression is only needed when producing a PNG file.

Rendering is synchronous CPU work. Keep it off a UI/event-loop thread, and reuse a renderer on the worker. `Renderer`, `Snippet`, `Style`, and `Font` are `Send + Sync`; cloning a renderer shares its caches, and concurrent calls are supported.

PNG downsampling uses a process-wide pool of at most four workers. The backend also shares a shadow-mask cache capped at 64 entries and 16 MiB of retained pixel data, plus entry overhead. These resources can outlive a renderer; active calls may briefly retain evicted masks. Bound application-level concurrency for large exports.

```rust
use monoprint::{Renderer, Snippet, Style};

fn main() -> Result<(), monoprint::Error> {
    let renderer = Renderer::new();
    let style = Style::default().width(640);
    let snippet = Snippet::new("const answer = 42;").language("ts".parse()?);
    let worker_renderer = renderer.clone();
    let worker = std::thread::spawn(move || worker_renderer.html(&snippet, &style));
    let html = worker.join().expect("preview worker panicked")?;
    // Your app loads `html` into its preview. Join away from the UI thread.
    assert!(html.starts_with("<!doctype html>"));
    Ok(())
}
```

For repeated edits, keep the worker alive, coalesce pending edits, and discard stale results using your app's request IDs. A render in progress cannot be cancelled. HTML embeds its assets on every call; this API does not promise a particular interactive frame rate or incremental DOM updates.

## Custom fonts and discovery

```rust
use monoprint::{Appearance, Font, Language, Renderer, Snippet, Style, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font = Font::from_bytes(std::fs::read("my-mono.ttf")?)?;
    let style = Style::default().font(font).font_size(18);
    let html = Renderer::new().html(&Snippet::new("hello"), &style)?;
    std::fs::write("custom.html", html)?;

    for theme in Theme::all() {
        println!("{}: {} (light: {})", theme.id(), theme.name(), theme.supports(Appearance::Light));
    }
    for language in Language::all() {
        println!("{}: {}", language.id(), language.name());
    }
    Ok(())
}
```

`Font::from_bytes()` accepts a slice, `Vec<u8>`, or `Arc<[u8]>`. Clones share bytes and prepared font sets; custom fonts do not accumulate in the renderer. Restore the bundled font with `style.theme_font()`. Overrides apply to the code font at weight 400, including titles in frames that use that font. Other label fonts stay unchanged. Single-face TTF/OTF are accepted; font collections and WOFF/WOFF2 are not. Missing glyphs automatically use the [bundled fallback fonts](font-fallback.md); characters absent from the entire fallback inventory return `Error::UnsupportedGlyph`.

`font_size(18)` sets the nominal code size in CSS pixels (integer 8–72, default 15).
Code, comments, line numbers, and line spacing scale together; frame labels retain
their theme styling. Automatic width and wrapping use the selected size. This
changes layout in all three formats; PNG `Scale` only changes output resolution.

```rust
use monoprint::{BundledFont, Renderer, Scale, Snippet, Style, TextRange};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = "fira-code".parse::<BundledFont>()?.font();
    let comments = "space-mono".parse::<BundledFont>()?.font();
    let style = Style::default().font(code).comment_font(comments).ligatures(true);
    let snippet = Snippet::new("let café = 1; // a comment")
        .language("rust".parse()?)
        .highlight_text(TextRange::new((1, 5), (1, 9)));
    let png = Renderer::new().png(&snippet, &style, Scale::X2)?;
    std::fs::write("example.png", png)?;
    Ok(())
}
```

Comments normally inherit the code font. `comment_font()` overrides only
grammar-recognized comments and docstrings; other strings, line numbers and
labels retain their fonts. Plain text has no comment classification.
`comments_use_code_font()` removes the override. Styles and their clones share
one lazily prepared mixed font set; changing either font or the theme starts a
new set. Each base font set retains up to eight fallback combinations; unchanged ASCII requests use the original cached set. Missing comment glyphs use the same automatic fallback chain, then report `TextRole::Comment` if no font covers them.

`BundledFont::all()` lists all ten shipped typefaces, with `id()` and `name()`.
IDs parse case-insensitively; an unknown ID returns `Error::UnknownFont`.
`font()` returns a `Font` backed by a bounded process-wide cache, sharing bytes
and prepared data across repeated choices. Explicit bundled selections use the
same override behavior as custom fonts; `theme_font()` restores the theme's
original font and weight.

`ligatures(true)` enables the font's `liga`, `clig` and `calt` features while preserving
code letter spacing. Text without substitutions keeps the same width and wrapping.
Availability depends on the font. The default is on, and
`ligatures(false)` disables these substitutions. Both options work
with PNG, SVG and embedded HTML; exact selections may split a ligature at the
selection boundary.

## Errors and limits

Theme/language parsing, scale conversion, and font construction validate immediately. Style and snippet builders remain infallible; rendering validates their combined options. `Error` and `InvalidOption` are non-exhaustive, so matches need a fallback arm. Backend failures preserve a readable diagnostic without exposing dependency types.

```rust
use monoprint::{Error, InvalidOption, Renderer, Snippet, Style};

let result = Renderer::new().svg(&Snippet::new("hello"), &Style::default().width(100));
match result {
    Err(Error::InvalidOption(InvalidOption::Width { min, .. })) => {
        assert_eq!(min, 113); // 2 × 32px padding + 49px for code
    }
    other => panic!("unexpected result: {other:?}"),
}
```

- Source: at most `MAX_SOURCE_BYTES` (1 MiB), checked before CRLF/CR normalize to LF. Other source text is preserved.
- Font: at most `MAX_FONT_BYTES` (64 MiB).
- Padding: 0–512 CSS pixels. Width: 80–8192px, also greater than twice padding + 48px.
- Nominal font size: integer 8–72 CSS pixels, default 15.
- PNG/raw image scale: 1–6. Odd scales render at twice the requested ratio, then downsample.
- PNG/raw images: at most 16,777,216 internal render pixels (the pinned raster backend’s limit). SVG: at most 64 million. Both: at most 32,768px per edge, checked after measurement and before rasterization. SVG uses 2× internally while retaining CSS output dimensions.
- Nonempty legacy subtitle metadata is rejected. Current themes do not use it.

Takumi retains bounded process-wide glyph data and per-thread layout scratch buffers. The latter stay allocated until the rendering thread exits, even after the renderer is dropped.

These limits are per call, not a process-wide memory or CPU quota. Applications control concurrency. Monoprint never performs file IO, looks up system fonts, launches processes, prints, or accesses the network. Rendering is deterministic for the pinned assets and dependency versions; antialiasing can differ from browser exports.

Run `cargo doc --no-default-features --no-deps --open` for the API reference and `cargo run --release --example library --no-default-features` for executable one-off, batch, custom-font, and worker examples.

## Editor palettes and backgrounds

`Theme::all()` includes the Monoprint default, the Originals collection, bundled frame presets and curated editor palettes. `family()` groups related palettes; `default_appearance()` identifies a supported default. `Style::new(theme)` uses that default. `style.theme(theme)` preserves the appearance when supported, otherwise switches to the new theme’s preferred appearance; an explicit unsupported `appearance()` still fails on render.

```rust
use monoprint::{Background, Style, Theme};
let theme: Theme = "catppuccin-latte".parse().unwrap();
let style = Style::new(theme).background(Background::Halftone);
```

`Background::{Theme, Solid, Lattice, Halftone, None}` controls the outer canvas. Colors follow the palette; the code window is retained. The Monoprint default uses halftone in both light and dark appearances. `transparent(true)` selects `None`, and `transparent(false)` restores `Theme`. These setters replace one another, so the last call wins. Imported themes preserve TextMate rules; language-server semantic coloring is not available. SynthWave ’84 includes its token-specific Neon Dreams glow.
