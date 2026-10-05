# Monoprint popup fade patch

Source: crates.io `gpui-component` 0.6.6, unchanged except `src/popover.rs`.
The published crate archive is pinned by Cargo's original checksum:
`0cf6c72516b44bb2e66a1bd2ccf77a122ecd1b54a75559ac299bb296bfcb5bb7`.

The dropdown panel already applies animated opacity to its full subtree,
including shadows. Its additional cubic shadow ramp made the ring and shadow
arrive later than the panel. Keep the resting shadow strength so all primitives
fade with the same opacity. Clip the shadow to the outside of the rounded
panel so shadow ink cannot show through its translucent interior. Placement,
duration, easing, focus, search,
dismissal and reduced-motion behavior stay with the upstream component.

Remove this patch when a published GPUI Kit version includes the correction.
