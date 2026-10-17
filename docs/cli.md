# CLI reference

Defaults: Monoprint dark, 15px code size, 32px outer padding, automatic width, 4× PNG, no title. Use `--light` for its paired light appearance.

| Option | Behavior |
| --- | --- |
| `-o, --output image.png` | Required. Infer PNG, SVG, or HTML from the extension. |
| `-o - --format svg` | Write to stdout; an explicit format is required. |
| `--theme monoprint` | Choose a theme ID (Monoprint is the default); `--list-themes` lists all variants. |
| `--light`, `--dark` | Override the preferred appearance; unsupported variants fail. Light-only palettes select light automatically. |
| `--background theme` | Outer canvas: `theme`, `solid`, `lattice`, `halftone`, or `none`. Colors follow the selected palette. |
| `--transparent` | Alias for `--background none`; cannot be combined with `--background`. Retains the code window. |
| `--padding 32` | Outer padding in CSS pixels, from 0 to 512. |
| `--width 640` | Outer width in CSS pixels. Wrap code to fit. |
| `--scale 2` | PNG pixel ratio, from 1 to 6. SVG/HTML retain CSS dimensions. |
| `--png-compression balanced` | Lossless PNG compression: `fast` (default) or `balanced` (smaller files, more encoding time). PNG only. |
| `--title example.rs` | Label where the theme has a title field. |
| `--subtitle 'An example'` | Auth0 subtitle metadata; the frame displays the language label, so this does not change the visible image. |
| `--line-numbers`, `--no-line-numbers` | Override line numbers for any theme; omit both flags to use its default. Plain text has no numbers. |
| `--highlight 1,3-5` | Highlight source lines. |
| `--highlight-text 2:5-2:12` | Highlight exact text; repeat for multiple ranges. End is exclusive. |
| `-l, --language rust` | Override filename detection; `--list-languages` lists IDs. |
| `--font fira-code` | Select a bundled code typeface; `--list-fonts` lists all ten. Defaults to the theme's typeface. |
| `--font-size 18` | Nominal code size in CSS pixels, an integer from 8 to 72. Defaults to 15. Changes text layout independently of PNG scale. |
| `--comment-font space-mono` | Select a bundled comment typeface. Defaults to the code typeface. |
| `--font-file mono.ttf` | Supply a TTF/OTF code font. HTML embeds it. |
| `--comment-font-file comments.otf` | Separate typeface for grammar-recognized comments and docstrings. Defaults to the code font. |
| `--ligatures`, `--no-ligatures` | Toggle coding ligatures supported by the font. Off by default. |
| `--force` | Replace an existing output. Otherwise overwriting is rejected. |

Omit the input filename, or use `-`, to read UTF-8 stdin. Unknown filenames and unspecified stdin use plain text; an unknown explicit language is an error. TypeScript and TSX are separate; `.ts`, `.mts` and `.cts` select TypeScript. CRLF/CR are normalized to LF; all other source text, including blank lines and trailing newlines, is preserved in selectable HTML.

Text ranges use one-based lines and Unicode-character columns: `2:5-2:12` selects
characters 5 through 11 on line 2. Tabs count as one character. Ranges may cross
lines; overlaps merge, and invalid or empty ranges fail before export. Both kinds
of highlight can be combined. Separate comment fonts use the selected language's
grammar; plain text has no comment classification. Ligatures enable the font's
`liga`, `clig` and `calt` features while preserving code letter spacing. These options
apply to PNG, SVG and HTML, and are available in Studio's **Typeface** section and source highlight controls.

```sh
monoprint example.rs -o example.png \
  --font fira-code --comment-font space-mono \
  --ligatures --highlight 3 --highlight-text 5:5-5:18
```

Bundled choices are Commit Mono, Fira Code, Geist Mono, Google Sans Code,
IBM Plex Mono, Inter, JetBrains Mono, Roboto Mono, Source Code Pro and Space Mono.
Use `--font-file` or `--comment-font-file` for a custom TTF/OTF instead; each
role accepts either a bundled ID or a file, not both.

Source reads stop at 1 MiB; custom fonts are limited to 64 MiB. PNG layout is limited to 16,777,216 internal render pixels by the raster backend; SVG allows 64 million. Both allow at most 32,768 pixels per edge. Odd PNG scales (1×, 3×, 5×) render at twice the requested scale before downsampling; SVG uses 2× internally. The limits apply before downsampling, so an odd scale can hit the limit before the next even scale. Lower the scale or shorten the snippet for larger inputs. Color emoji and missing glyphs use bundled, offline Noto fallbacks in PNG, SVG and HTML, including CJK and several additional scripts. The selected typeface is preserved wherever it has coverage. Characters missing from every bundled fallback still fail with their code point. See [font coverage and limitations](font-fallback.md).

## Formats

- **PNG:** native raster rendering with Fast or Balanced lossless compression.
- **SVG:** outlined text and vector geometry, with embedded raster textures/effects where needed. No external fonts or `foreignObject`.
- **HTML:** selectable highlighted text, embedded fonts/assets, no JavaScript or external resource requests.

See [validation](validation.md) for rendering checks and known limitations.
