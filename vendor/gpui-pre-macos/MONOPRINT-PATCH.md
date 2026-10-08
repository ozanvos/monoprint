# Monoprint App Store feature

Source: crates.io `gpui-pre-macos` 0.3.8, upstream Zed revision
`279fe070bb389b79652e52065b2f001edcc0b11b`.
Original crate checksum:
`0d756813ff1cd741a502af60fd18869e0677795936d7349642b41c49f43120c0`.
Original Apache 2.0 license and source are retained. Changes are limited to
`Cargo.toml`, `Cargo.toml.orig` and `src/window.rs`.

The opt-in `app-store` feature excludes private AppKit selectors, the private
window-restoration class, the undocumented workspace-restoration default, and
native material layer inspection. Without this feature, upstream behavior stays
unchanged.

Store builds use public `mouseDownCanMoveWindow` for custom drag ownership,
`zoom:` for the Fill preference, ordinary `NSKeyedUnarchiver` for restoration,
and public frame-resize cursors on macOS 15+. Older versions use the standard
crosshair for diagonal resize cursors. Native materials retain their default
tint/saturation, and restoration no longer forces the original Space.

This feature addresses the audited API paths only. It does not configure
sandbox entitlements, provisioning, App Store signing, or submission.
Remove this patch when the published backend provides an equivalent feature.
