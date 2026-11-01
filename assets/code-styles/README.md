# Code-only styles

Ten Monoprint presets that frame source code without title or language labels.
Each pairs light and dark; `#capture[data-theme]` rules recolor the frame.
`themes.json` defines palettes and fonts; `frames.json` defines content-sized
surfaces consumed by `src/selected_styles.rs`.

Ordinal enables line numbers by default. Aside uses Inter for comments;
explicit comment-font choices override that default.

Edit these assets directly. Rebuild the syntax registry after palette changes
and regenerate Studio thumbnails after visual changes. See [maintenance](../../docs/maintenance.md).

Set `MONOPRINT_CODE_SAMPLES` to an output directory when running
`cargo test --test code_styles` to inspect PNG, SVG, and HTML samples.
