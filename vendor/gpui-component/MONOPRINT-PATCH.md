# Monoprint popup fade patch

Source: crates.io `gpui-component` 0.8.0, unchanged except `src/popover.rs`.
The published crate archive is pinned by Cargo's original checksum:
`4097e161cc81e86323b439664be0cdd4f7c56148e867ed6f67fd72b966b6c2d1`.

The dropdown panel already applies animated opacity to its full subtree,
including shadows. Its additional cubic shadow ramp made the ring and shadow
arrive later than the panel. Keep the resting shadow strength so all primitives
fade with the same opacity. Clip the shadow to the outside of the rounded
panel so shadow ink cannot show through its translucent interior. Placement,
duration, easing, focus, search,
dismissal and reduced-motion behavior stay with the upstream component.

Remove this patch when a published GPUI Kit version includes the correction.
