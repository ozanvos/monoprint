# Local rendering fix

Inline text shadow fix: `src/text.rs` reads each glyph run's resolved source-span
style instead of applying only the container's shadows. This enables SynthWave's
per-token glow and explicit `text-shadow:none` overrides without changing text
layout. Pixel coverage lives in `tests/editor_themes.rs`.

Based on unmodified crates.io `takumi-svg` 0.3.22, Git revision recorded in `.cargo_vcs_info.json`. Original MIT/Apache notices are included.

Changes in `src/render.rs`:

- Paint outer box shadows in reverse CSS-list order for SVG's source-over compositing.
- Clip the blurred outer shadow to the exterior of the rounded border box. Translucent windows must not reveal a solid shadow underneath their interior.
- Give Gaussian blur filters sufficient bounds so broad shadows do not acquire rectangular clipping edges (`src/lib.rs`).
- Skip box decorations on anonymous inline-layout containers, matching the raster backend. Their inherited background otherwise paints translucent line highlights twice over the code area.

This fixes the dark/opaque windows observed in the SVG-versus-PNG comparison. The browser export matrix in `scripts/verify-exports.mjs` verifies the resulting images. Remove this patch once an upstream release supplies the same behavior and passes the matrix.

Library packaging: `Cargo.toml` pins `takumi-core` to exactly 0.25.0, the core
version validated with these changes. Monoprint depends on this directory
and calls its renderer directly, so downstream path/git users retain the fixes
without adding a root-level `[patch]`.

The core dependency requests only SVG support rather than enabling all core
defaults. Other upstream dependencies still enable several decoders through
Cargo feature unification; Monoprint accepts only single-face TTF/OTF fonts.
