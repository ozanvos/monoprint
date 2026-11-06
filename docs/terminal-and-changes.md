# Terminal output and code changes

Monoprint can render colored terminal output and unified code changes, in
addition to ordinary source. Both work in PNG, SVG and HTML, through the CLI,
the Rust and Node.js APIs, and Studio.

## Terminal output

Select the `ansi` language (aliases `terminal` and `ansi-escape`). A `.ans`
filename selects it automatically. Standard input is never detected
automatically, so pass `-l ansi` when piping:

```sh
cargo test --color=always 2>&1 | monoprint -l ansi --title 'cargo test' -o test.png
git log --oneline --color=always -5 | monoprint -l ansi -o log.svg
monoprint fixtures/ansi/compiler.ans -o compiler.html
```

Many tools disable color when writing to a pipe. Use their `--color=always`
option, or set `FORCE_COLOR=1` or `CLICOLOR_FORCE=1` for tools that support it.
Monoprint renders captured text; it does not run commands.

Supported styling:

- SGR colors: the 16 standard and bright colors, the 256-color palette and
  24-bit RGB, for foreground and background, in both `;` and `:` forms.
- SGR attributes: bold, dim, italic, underline, strikethrough, inverse and
  hidden, with their individual resets, default colors (`39`, `49`) and full
  reset (`0`).
- Carriage-return redraws, such as progress bars, and erase-to-end-of-line
  (`ESC[K`). The last redraw of each line remains.
- Backspace overstrikes from man pages and `nroff`: repeated characters become
  bold, and underscores become underlines.

The 16 standard colors come from the selected theme's terminal palette, so
captures follow light and dark appearances. Indexed colors above 15 and RGB
colors are exact. Default text and background use the theme.

Other escape sequences are removed without effect. This includes cursor
movement, screen clearing, window titles and OSC 8 hyperlink targets (the link
text remains). Unsupported attributes, such as blinking and underline colors,
are ignored. Other control characters are dropped; tabs remain. Monoprint is not a terminal emulator. Full-screen programs, cursor
positioning and terminal-width wrapping are not reproduced; capture a
full-screen program's screen first, for example with `tmux capture-pane -e -p`.

Line numbers are hidden by default; `--line-numbers` shows them. Line and text
highlights address the visible text after escape removal, redraws and
overstrikes. HTML exports contain only visible text, never escape bytes. The
1 MiB source limit applies to the raw capture, including escape bytes.

## Code changes

A change is built from one of three inputs:

- **Compare** two complete versions of a file.
- **Patch** renders a single-file unified diff, such as `git diff` output.
- **Marks** label lines of one snippet as added or removed, without comparing.

```sh
monoprint --diff fixtures/diff/before.rs fixtures/diff/after.rs -o change.png
git diff -- src/main.rs | monoprint --patch - -o change.svg
monoprint snippet.rs --added 3-4 --removed 2 --first-line 120 -o marked.html
```

All three retain syntax colors. Compare highlights the complete old and new
versions separately, so removed lines are colored in their original context.
A patch only contains its hunks, so highlighting cannot use code outside them;
a hunk that starts inside a comment or string may be colored incorrectly.

### Language and title

Compare infers the language from the new (second) filename and uses that
filename as the default title; when the new version is read from standard
input, pass `-l` and `--title`. Patch input infers both from the file path in
its `+++` header, or `---` for a deleted file. A bare hunk without headers
uses plain text and no title. Marks infer the language from the input
filename, like an ordinary snippet. An explicit `-l` and `--title` override
inference.

### Context and folds

Compare and patch keep three unchanged lines around each change by default.
`--context 0` shows only changed lines; `--context all` keeps every known line.
Omitted lines become a label such as `⋯ 12 unchanged lines`. In a patch, gaps
between hunks are always labeled, because those lines are not in the patch;
lines before the first hunk and after the last are not shown. Marked snippets
never fold.

Identical compared inputs print a warning. Because nothing changed, the
default context folds the whole file; use `--context all` to show it.

### Numbers, signs and word emphasis

Changed lines receive a `+` or `−` sign and a background. The gutter shows new
line numbers by default. `--diff-numbers old` shows old numbers, and `both`
shows two columns. Added lines have no old number and removed lines no new
number. Numbers follow the usual line-number setting: themes that hide them
by default need `--line-numbers`, and plain text never shows them. Signs
appear either way.

Marked snippets count both old and new numbers from `--first-line`
(default 1). For compare and patch input, numbers come from the files and the
patch hunk headers; `--first-line` has no effect.

When a removed line is paired with a similar added line, the changed words are
emphasized. Lines that differ too much are not emphasized. `--no-diff-words`
turns this off. Marks have no word emphasis.

Signs, numbers and fold labels are not part of the code. They are excluded
from HTML text selection and from highlight positions.

### Highlights

`--highlight` and `--highlight-text` address the displayed code: line 1 is the
first visible row, including removed rows, and fold labels are skipped. For
example, with the default context the first row of the image may be line 40 of
the file but line 1 for highlighting.

### Limits and errors

- Each compared file, a patch, and the displayed code are limited to 1 MiB.
- Only one compare input may read standard input (`-`).
- Patches must contain one file with text hunks. Multi-file, binary,
  rename-only and mode-only patches fail. Hunk line counts must match their
  headers, and hunks must not overlap.
- A line cannot be both added and removed. Marks must lie within the source.
- Compare and patch cannot be combined with each other, with an input file or
  with marks. `--context` and `--no-diff-words` require compare or patch input.
- Changes cannot use the `ansi` language. A patch captured with color, such as
  `git diff --color=always`, can instead be rendered as terminal output.

Split (side-by-side) views are not available; all changes render as one
unified column.

## Other interfaces

- [CLI reference](cli.md#terminal-output-and-changes)
- [Rust API](library.md#terminal-input-and-unified-changes)
- [Node.js API](node-api.md#terminal-output-and-changes)
- [Studio](studio.md#terminal-output-and-diffs)
