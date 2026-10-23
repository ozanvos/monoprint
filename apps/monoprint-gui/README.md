# Monoprint Studio

A desktop code-image editor built with [GPUI Kit](https://gpui-kit.com/).
It uses the same renderer as the Monoprint CLI and library.

## Run

From the repository root:

```sh
cargo run --locked --release --manifest-path apps/monoprint-gui/Cargo.toml
# Open a file at launch:
cargo run --locked --release --manifest-path apps/monoprint-gui/Cargo.toml -- example.rs
```

Studio has its own Cargo workspace. It needs a graphical session and GPU;
see [GPUI Kit's prerequisites](https://gpui-kit.com/docs/getting-started/).
macOS Apple Silicon is tested. Windows and Linux still need native validation.

## Use

- Open a UTF-8 file, paste code or type in the editor. Choose its language above
  the editor. TypeScript and TSX are separate languages.
- Click the theme thumbnail to browse themes. Search and filter by appearance.
  The theme's light/dark switch changes the image; the top-bar switch changes
  the app interface.
- New drafts start with **Monoprint**, the Masthead frame with Oxide syntax.
  Its light and dark appearances share one theme and lead the theme browser.
- Twenty code-only Originals add open fields, edge-attached panels, and quiet
  frames, from Ground through Trace. They need no filename or other metadata.
  Aside uses Inter for comments by default; select a comment typeface to override it.
- Set the width, padding, title, background and line numbers. Leave Width blank
  for automatic sizing. Choose None for a transparent outer background.
- Select text to highlight it, or click line numbers to highlight whole lines.
- Choose code and comment typefaces, a size from 8 to 72 px, and ligatures enabled by default.
  Bundled fonts work offline; custom fonts must be single-face TTF/OTF files.
- Scroll or drag to pan, pinch to zoom, and use the image's side handles to change
  export width. Preview zoom does not change the export.
- Save PNG at 1–6×, SVG or HTML. Copy exports a PNG to the clipboard. Fast and
  Balanced PNG compression produce the same pixels; Balanced makes smaller files.

See [macOS file handling and Shortcuts](../../docs/studio.md) for drag-out export,
Finder integration and automation.

## Keyboard shortcuts

| Action | macOS |
| --- | --- |
| Open source | Command-O |
| Export | Command-S |
| Copy image | Command-Shift-C |
| Switch interface appearance | Command-Shift-L |
| Move by character / line | Control-B/F / Control-P/N |
| Delete backward / forward | Control-H/D |

Shift extends cursor movement into a selection. Command and Option arrow
navigation are also supported. On other platforms, Control replaces Command.

## Saved draft

Studio saves the latest source and export settings locally. Opening or pasting
another source replaces the draft. Code is limited to 1 MiB; an oversized draft
leaves the previous saved file intact.

- macOS: `~/Library/Application Support/Monoprint/studio.json`
- Sandboxed macOS: `~/Library/Containers/app.monoprint.studio/Data/Library/Application Support/Monoprint/studio.json`
- Linux: `$XDG_STATE_HOME/monoprint/studio.json`, or `~/.local/state/monoprint/studio.json`
- Windows: `%APPDATA%/Monoprint/studio.json`

Set `MONOPRINT_SESSION=off` to disable saving, or set it to a file path for a
separate draft. Interface appearance, preview zoom and catalog filters reset
at launch. Store builds retain custom-font access with security-scoped bookmarks.

## Build and check

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py
open target/gui/Monoprint.app

cargo fmt --manifest-path apps/monoprint-gui/Cargo.toml -- --check
cargo clippy --locked --manifest-path apps/monoprint-gui/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path apps/monoprint-gui/Cargo.toml
```

Bundling requires Python 3.11+ and Xcode 26+. Use `--debug` for a development
bundle. The default signature is ad-hoc. See [release packaging](../../docs/release.md)
for Developer ID and App Store signing, and [desktop updates](../../docs/desktop-updates.md)
for Sparkle and Velopack packages. Keep the packaged licenses and source archives.

To save native test screenshots, set
`MONOPRINT_GUI_SCREENSHOTS="$PWD/output/gui-validation"` when running the
`studio` integration test. Native dialogs, clipboard delivery and installed
update behavior also need manual testing on each platform.
