# Monoprint App Store feature

Source: crates.io `gpui-pre-macos` 0.3.6, upstream Zed revision
`bcf6582ce3500df93a8a39366640173e6786cea6`.
Original crate checksum:
`951d17e41a72067ad2d35c60e08a62e8ad7fa9400b41c802e26cfb459f6c05da`.
Original Apache 2.0 license and source are retained. Changes are limited to
`Cargo.toml` and `src/window.rs`.

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
