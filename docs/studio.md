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
[the macOS patch notes](../vendor/gpui-fast-macos/MONOPRINT-PATCH.md)
and [the motion adapter notes](../vendor/gpui-motion-kit/MONOPRINT-PATCH.md).

## Terminal output and diffs

To render terminal colors, choose **Terminal output** in the language picker or
open an `.ans` file. When the code contains color sequences, a notice asks
**This looks like terminal output. Render its colors?**; choose **Use terminal
colors** or **Keep as text**. Studio never switches by itself.

In terminal mode the editor shows the visible text read-only, while the
document keeps the raw capture with its escapes. Paste replaces the capture.
**Terminal output · Edit as plain text**, or choosing another language, asks
before removing the colors and making the text editable. Line numbers are off
unless enabled in **Line numbers**. Comparison and mark controls are
unavailable for terminal output.

The **Diff** inspector turns the current code into a change:

- **Choose previous version…** compares the code with an earlier file. The
  editor holds the new version.
- **Compare with clipboard** uses clipboard text (up to 1 MiB) as the previous
  version.
- **Mark added** and **Mark removed** label the lines touched by the editor
  selection. Marking the same lines again clears them; marking them with the
  other kind switches them. Marks follow edits.
- **Context** cycles through 0, 3, 10 and all unchanged lines.
- **Numbers** cycles through new, old and both gutter columns.
- **Off** removes the change and keeps the code.

Starting a comparison replaces any marks, and marking replaces a comparison.
Opening a file clears the change. In a comparison, line and text highlights are
made on the new version; highlights on lines hidden by the context are omitted
from the image.

When the language is **Diff**, for example after opening a `.patch` or `.diff`
file, **Show as change** renders the patch as a change and selects the
language from its file path. The editor keeps the patch text. **Off** returns
to the patch with Diff highlighting. Patches must contain a single file with
text hunks; otherwise Studio shows the reason and keeps the patch text.

Changed-word emphasis is on for new documents. Studio has no control for it,
but a saved document can keep it off, and Copy command then includes
`--no-diff-words`. To turn it off for an export, add `--no-diff-words` to the
copied command. Split views are not available.

**Copy command** reproduces terminal captures with `--language=ansi` and
bash/zsh ANSI-C quoting (`$'…'`), never raw ESC bytes. Comparisons copy a
`monoprint --patch -` command with a self-contained patch of both versions, so
the CLI keeps full syntax context. The patch goes in a quoted heredoc, or uses
ANSI-C quoting if it contains control characters. Patches and marks
copy their source with `--patch -` or `--added`/`--removed`. See
[terminal output and code changes](terminal-and-changes.md).
