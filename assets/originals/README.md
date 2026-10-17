# Monoprint Originals

The default Monoprint preset has light and dark appearances. Its frame lives in
`src/monoprint_theme.rs` and `monoprint.css`. The additional presets use
`src/originals.rs` and the CSS files in this directory.

`themes.json` defines the catalog, fonts, and colors. Matching syntax palettes
live in `assets/themes/original-*.json`. Keep them in sync when editing colors.
Rebuild the syntax registry and regenerate Studio thumbnails as described in
[maintenance](../../docs/maintenance.md).

Frames flow around real code and metadata. Highlighter Pass and Proof Marks
follow selected source lines. Wayfinding marks nonblank source groups.
PNG, SVG, and HTML use the same native layout.
