# Monoprint popup fade patch

Source: crates.io `gpui-component` 0.7.1, unchanged except `src/popover.rs`.
The published crate archive is pinned by Cargo's original checksum:
`bc8b7def8e74b0d822d3e96f6b28fb5a2c20eb7a83d30d22fa59ca8088d1159b`.

The dropdown panel already applies animated opacity to its full subtree,
including shadows. Its additional cubic shadow ramp made the ring and shadow
arrive later than the panel. Keep the resting shadow strength so all primitives
fade with the same opacity. Clip the shadow to the outside of the rounded
panel so shadow ink cannot show through its translucent interior. Placement,
duration, easing, focus, search,
dismissal and reduced-motion behavior stay with the upstream component.

Remove this patch when a published GPUI Kit version includes the correction.
