# Monoprint App Store feature

Source: crates.io `gpui-fast-macos` 0.1.10. The existing App Store exclusions
were ported from Monoprint's `gpui-pre-macos` 0.3.8 patch.
Original crate checksum:
`0c9656cc924a624ebbc44bb9f8149f5206fe6e94b262349f662efe692e03d900`.
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
