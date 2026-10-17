# Selected styles

Seventeen Monoprint presets with content-sized frames.

- `themes.json` defines palettes, fonts, and supported appearances.
- `frames.json` contains frame markup and material treatments.
- `frame.css` provides wrapping, line-number, and text-selection behavior.
- `src/selected_styles.rs` applies font scaling and layout settings.

Edit these assets directly. After palette changes, rebuild the syntax registry;
after visual changes, regenerate Studio thumbnails. See [maintenance](../../docs/maintenance.md).

Set `MONOPRINT_SELECTED_SAMPLES` to an output directory when running
`cargo test --test selected_styles` to inspect PNG, SVG, and HTML samples.
