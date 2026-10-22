# macOS file handling and Shortcuts

## Files and drag-out export

Drop one UTF-8 source file into the window or onto the Dock icon, or choose
Open With → Monoprint in Finder. The extension determines the language.
Opening a file replaces the source and highlights while keeping image settings.
Invalid, oversized or multiple-file inputs leave the source intact. Monoprint
registers as an alternative viewer without changing default file associations.

To drag an image into Finder or another app, hover over the preview's top-left
corner and drag its document button. A spinner appears while the PNG is prepared
at the selected scale and compression. Ordinary canvas dragging pans the preview.
The temporary PNG remains readable after editing or quitting.

## Shortcuts

Ordinary macOS bundles include two signed shortcuts and a renderer bridge.
App Store bundles omit them and the CLI.

- Monoprint Clipboard to PNG reads the clipboard. Its default language is
  TypeScript; change `typescript` in its Run Shell Script action to another CLI ID.
- Monoprint Source File to PNG accepts one file and can run as a Finder Quick Action.

Open the `.shortcut` files in `Monoprint.app/Contents/Resources/Shortcuts/`
to install them. Both save unique PNGs in `~/Pictures/Monoprint` using Monoprint,
automatic width and 2× Balanced compression. They run independently of the
Studio draft and require Shortcuts script access. Assign keyboard shortcuts
in Shortcuts' Details panel.

The bridge also accepts stdin:

```sh
printf 'const answer = 42;\n' | \
  /Applications/Monoprint.app/Contents/MacOS/monoprint-shortcut text typescript
```

It prints the saved path. `files /absolute/path/to/source.rs` detects the language
from the extension. `MONOPRINT_OUTPUT_DIR` changes the destination.

Regenerate the signed shortcuts after editing their definitions:

```sh
python3 apps/monoprint-gui/scripts/build-shortcuts.py \
  --output apps/monoprint-gui/assets/shortcuts --sign
```

This uses Apple's `shortcuts sign --mode anyone` service. Normal builds copy
and verify the existing signed files.

## Preview and editor

Use **>_ CLI** at the left of the preview toolbar to preview the current command, then
**Copy command** to include the source code and render settings. Paste into bash
or zsh with `monoprint` on your PATH. The command saves `monoprint.png` (or the
selected SVG/HTML format) in the current folder and refuses to overwrite an
existing file. Custom font paths refer to files on your machine.

Previews render at 2× on a background worker and pass raw BGRA pixels to GPUI.
Rapid edits are coalesced; stale results are discarded. Invalid input keeps the
previous preview and shows an error. Export renders the current document, so
large PNG scales can exceed the pixel limit even when the preview fits.

Editor syntax colors follow the interface's GitHub light/dark palette, separately
from the export theme. Updated tokens are computed after a short typing delay;
existing colors follow edits in the meantime. Oversized edits remain editable
without syntax coloring. The language picker offers 109 selected languages.
Saved and file-inferred languages outside that set still render.

Text highlights follow insertions and deletions. Line highlights use fixed line
numbers; marks beyond the end are removed when the source is shortened. Opening
or pasting replacement source clears both kinds. Clear all highlights removes
them without changing the code.

Comment fonts follow the selected grammar's comment/docstring scopes. Plain text
has no comment classification. Ligatures are off by default. Missing glyphs use
[bundled fallback fonts](font-fallback.md), including color emoji and CJK.
Coverage is limited to those fonts; CJK uses Simplified Chinese default forms.

## UI assets

Regenerate theme thumbnails after changing palettes or rendering:

```sh
cargo run --locked --release --example gui_thumbnails
```

The app icon source is `apps/monoprint-gui/assets/Monoprint.icon`. Bundling uses
`actool` to compile it into `Assets.car` and an `.icns` fallback. To compile it
separately:

```sh
python3 apps/monoprint-gui/scripts/build-icon.py
```

Toolkit patches are documented in
[the popup patch notes](../vendor/gpui-component/MONOPRINT-PATCH.md)
and [the macOS patch notes](../vendor/gpui-pre-macos/MONOPRINT-PATCH.md).
