# Node API reference

| Method | Result | Execution |
| --- | --- | --- |
| `png(source, options?)` | `Promise<Buffer>` | Native worker |
| `svg(source, options?)` | `Promise<string>` | Native worker |
| `html(source, options?)` | `Promise<string>` | Native worker |
| `pngSync`, `svgSync`, `htmlSync` | `Buffer` / `string` | Blocks calling thread |

The convenience functions share a lazy renderer. Use `new Renderer(style?)` to own a reusable cache and style defaults. It has the same six methods. Style defaults are snapshotted at construction; per-render options override them without mutating the renderer.

```ts
import { Renderer, languageForFileName } from 'monoprint';

const renderer = new Renderer({ theme: 'monoprint', padding: 24, width: 640 });
const filename = 'main.rs';
const svg = await renderer.svg('fn main() {}', {
  language: languageForFileName(filename) ?? 'plain',
  title: filename,
  highlight: [1],
});
const html = await renderer.html('let a = 1;\nlet b = 2;', {
  language: 'rust',
  highlight: [[1, 2]], // One-based, inclusive.
  width: 'auto',      // Clear the inherited fixed width.
});
```

Defaults are Monoprint, dark, 15px code size, 32px padding, automatic width, plain-text highlighting, theme-default line numbers, and 4× PNG scale. `language` accepts aliases such as `ts`, `rs`, and `py`; TypeScript and TSX are separate. Titles are labels and do not imply a language. Filesystem access belongs to the caller.

`Style` contains `theme`, `appearance`, `padding`, `fontSize`, `width`, `background`, `transparent`, `lineNumbers`, and `font`. Render options additionally accept `language`, `title`, `subtitle`, and `highlight`; PNG alone accepts `scale` (1–6). `fontSize` is an integer from 8–72 CSS pixels; it changes code, comments, line numbers, and layout in every format, independently of PNG `scale`. `lineNumbers` accepts `true` or `false` to override any theme, or `'auto'` (the default) to follow the theme. Plain text never shows numbers. `subtitle` is retained for compatibility; current themes reject nonempty values. Unknown options and invalid numbers are rejected.

PNG also accepts `compression: 'fast' | 'balanced'`. The default is `'fast'`,
Choose `'balanced'` for smaller final exports
at the cost of more encoding time; both modes preserve every pixel. Compression
belongs to each PNG call, not to the reusable renderer style or SVG/HTML options.

```ts
const image = await renderer.png('const answer = 42;', {
  language: 'ts', scale: 2, compression: 'balanced',
});
```

## Discover themes and languages

```ts
import { getThemes, getLanguages } from 'monoprint';

const lightThemes = getThemes().filter(t => t.appearances.includes('light'));
const languages = getLanguages();
```

The immutable catalogs include names, canonical IDs, legacy theme aliases, language aliases, supported appearances, and theme line-number policies. `getThemes()` lists 69 curated presets, pairing matching light/dark palettes. Use `{ theme: 'ayu', appearance: 'light' }` or the compatible `{ theme: 'ayu-light' }` alias. They contain 232 named grammars represented as 231 selectable highlighting languages plus plain text. The [grammar review](grammar-licenses.md) lists intentionally excluded source grammars.

## Fonts and errors

```ts
import { readFile } from 'node:fs/promises';
import { Font, Renderer, MonoprintError } from 'monoprint';

const font = Font.fromBytes(await readFile('MyMono.ttf'));
const renderer = new Renderer({ font });
try {
  const image = await renderer.png('hello', { scale: 2 });
} catch (error) {
  if (error instanceof MonoprintError) {
    console.error(error.code, error.message, error.details);
  } else throw error;
}
```

`Font.fromBytes(Buffer | Uint8Array)` copies and validates a single-face TTF/OTF, up to 64 MiB. Reuse the handle across renderers to share preparation. Collections and WOFF/WOFF2 are unsupported. Custom fonts affect code, not theme labels; pass `font: null` to restore the theme font. Supply fonts you have rights to embed in your chosen format.

Errors are `MonoprintError` instances with stable `code` and `details` fields: `INVALID_ARGUMENT`, `INVALID_OPTION`, `UNKNOWN_THEME`, `UNKNOWN_LANGUAGE`, `INVALID_FONT`, `UNSUPPORTED_GLYPH`, `IMAGE_TOO_LARGE`, `RENDER_FAILED`, or `NATIVE_BINDING_UNAVAILABLE`. Async calls reject promises, including for invalid options; sync calls throw. Source is limited to 1 MiB before newline normalization. PNG allows 16,777,216 internal pixels; SVG allows 64 million. Both allow 32,768 pixels per edge. Odd PNG scales render at twice the requested scale before downsampling, so those limits apply to the larger internal image.

Emoji and missing glyphs automatically use embedded Noto fonts, with no system-font or network dependency. PNG and SVG embed color emoji; HTML embeds the required fallback fonts. Coverage includes CJK, Arabic, Devanagari, Hebrew, Thai and additional symbols, but not all Unicode. The native binary includes about 28 MiB of fallback font assets, and HTML grows with the fonts it needs. See [coverage and limitations](font-fallback.md).

Async rendering uses libuv's worker pool and shares it with other Node operations. Bound concurrency for large batch jobs. A queued request owns its source/options and retains the native renderer/font, even if the caller drops its references. Running renders cannot be canceled; coalesce preview edits before dispatching. Rust backend thread-local scratch and bounded process-wide glyph/shadow-mask caches can outlive a renderer. PNG downsampling shares at most four additional worker threads across the process; the shadow cache retains at most 16 MiB of pixel data plus entry overhead.

## Runtimes and bundling

The release matrix targets **Node.js 22+** on macOS arm64/x64, Linux glibc arm64/x64, and Windows x64. Linux release builds use Ubuntu 22.04 (glibc 2.35); macOS builds target macOS 11+. Alpine/musl, Windows arm64, browser bundles, and edge runtimes are outside v1's matrix. Native binaries are platform-specific even though Node-API avoids per-Node-version builds. Run build and runtime tests on each target before publishing.

Install normally with optional dependencies enabled. npm selects the matching native package, such as `monoprint-darwin-arm64`; there is no install script or compiler fallback. Deploy dependencies for the runtime platform, not a copied macOS `node_modules` directory on Linux.

Keep the package external in a server bundle, and deploy its production dependencies:

```ts
// esbuild
await build({
  entryPoints: ['server.ts'], bundle: true, platform: 'node', format: 'esm',
  outfile: 'dist/server.mjs', external: ['monoprint', 'monoprint-*'],
});
```

For Next.js use `serverExternalPackages: ['monoprint']` and the Node runtime. For Vite SSR use `ssr.external: ['monoprint']` and `optimizeDeps.exclude: ['monoprint']`. The `.node` addon is a runtime file, not JavaScript to inline into a browser bundle. Both ESM imports and CommonJS `require()` share the same classes and caches. Electron packaging additionally needs native addons outside ASAR compression; Electron is not part of this package's tested runtime matrix.

## Backgrounds

Editor palettes use their supported appearance automatically (for example, `catppuccin-latte` is light). `getThemes()` also exposes `family` and `defaultAppearance`. Set `background` to `theme`, `solid`, `lattice`, `halftone`, or `none`; matching colors come from the palette. `transparent: true` is the legacy alias for `background: "none"`. Do not supply both in one options object; a per-render background replaces a renderer-default transparency option, and vice versa.

```ts
const renderer = new Renderer({ theme: "catppuccin-mocha", background: "lattice" });
const image = await renderer.png("const answer = 42;", { language: "ts", scale: 2 });
```
