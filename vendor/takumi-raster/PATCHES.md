# Local exact-pixel performance changes

Based on crates.io `takumi-raster` 0.5.1, upstream revision
`4180970140a2f1a811e5dbb3b33343c74787bebc` (also recorded in
`.cargo_vcs_info.json`). Original MIT/Apache licenses are preserved.

Backported upstream changes:

- [93117787](https://github.com/kane50613/takumi/commit/93117787ac09908e91b0f929014264c436093ef0): SIMD demultiply skips transparent and opaque runs.
- [ae7a6633](https://github.com/kane50613/takumi/commit/ae7a6633a8043d0b4ad6e6fa60b14ac44a4815c5): four-row SIMD for angled gradients. Only the import context was adapted to this crate version.
- [08714406](https://github.com/kane50613/takumi/commit/0871440617c99051cbda163a68c62ba3eb93a97e): horizontal alpha blur uses a four-pixel SIMD prefix sum.

Monoprint changes in `src/components/blur.rs` and `src/components/blur/`:

- Detect globally identical rows/columns in the actual rasterized alpha mask.
  Retain `6*r+1` samples of a repeated run for the three box passes of radius
  `r`, blur the smaller mask, then expand its blurred center bytes. Preserve
  pass order, integer quantization, edge clamping, cutouts and compositing.
  Masks below 4096 bytes or saving less than 25% of area use the general path.
- Reuse blurred compact masks through a process-wide cache capped at 64 entries
  and 16 MiB of retained input/output pixel data, plus bounded entry overhead.
  Lookup checks dimensions, radius and exact input bytes after hashing. Misses
  compute outside the lock; poisoning or oversized masks bypass the cache.
  Active renders may temporarily retain evicted buffers. Only alpha masks are
  retained, not source text or finished exports.
- Pin `takumi-core` to `=0.25.0`. Monoprint calls this path dependency directly,
  preserving changes for downstream path/git consumers without a root patch.

The original canvas limit and PNG compression policy are unchanged. Unit tests
compare compact and full six-pass output byte-for-byte and test cache collisions,
geometry and eviction. See `docs/shadow-optimization-design.md` for the support
argument and the performance report for full-image verification. Remove local
patches only after replacement dependencies pass the same pixel comparisons.

In `src/canvas/mask.rs`, branch-free cutout attenuation enables compiler SIMD
while preserving all 65,536 input-alpha pairs. In `src/canvas/composite.rs`,
64-byte all-zero alpha runs skip per-pixel compositing; mixed runs and tails
keep the original arithmetic. Tests compare clipped and blended results to a
scalar reference, including chunk boundaries and negative mask placement.

Additional exact-output optimizations:

- `measure_size` computes the same root layout as `measure`, without building
  the unused descendant measurement tree or shaping text again for its runs.
  The render still computes layout for its final, rounded viewport.
- Constraint masks are intersected in their owned incoming buffer, preserving
  the parent mask and the existing integer formula. This follows the approach
  of upstream [fd2cf7f1](https://github.com/kane50613/takumi/commit/fd2cf7f1f5ce88e0aa1cbe5344d08c03b10ffdb1).
- `canvas/composite/constant_span.rs` blends sixteen pixels at once with NEON
  for normal-mode constant fills without a clipping mask. Both mask scaling
  and source-over keep the original `fast_div_255` rounding, saturation and
  transparent-pixel rules. Scalar tails and non-AArch64 fallback are retained.
  Tests cover all source-alpha/mask-alpha pairs, zero destination alpha,
  saturation, chunk tails and alignment against the scalar reference.
