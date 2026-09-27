//! Exact normal-mode source-over for a constant premultiplied color and mask.

use crate::blend::{composite_premultiplied_over, scale_premultiplied_pixel};

pub(super) fn blend(dst: &mut [[u8; 4]], mask: &[u8], color: [u8; 4]) {
  debug_assert_eq!(dst.len(), mask.len());
  #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
  {
    let (runs, tail) = dst.as_chunks_mut::<16>();
    let (masks, mask_tail) = mask.as_chunks::<16>();
    for (run, mask) in runs.iter_mut().zip(masks) {
      neon(run, mask, color);
    }
    scalar(tail, mask_tail, color);
  }
  #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
  scalar(dst, mask, color);
}

fn scalar(dst: &mut [[u8; 4]], mask: &[u8], color: [u8; 4]) {
  for (pixel, &alpha) in dst.iter_mut().zip(mask) {
    if alpha != 0 {
      composite_premultiplied_over(pixel, scale_premultiplied_pixel(color, alpha));
    }
  }
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn neon(dst: &mut [[u8; 4]; 16], mask: &[u8; 16], color: [u8; 4]) {
  use std::arch::aarch64::*;

  // Preserve fast_div_255's exact rounding: (v + 128 + (v >> 8)) >> 8.
  // Products are at most 65025; both additions fit in u16.
  #[inline(always)]
  unsafe fn multiply(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
    unsafe {
      let low = vmull_u8(vget_low_u8(a), vget_low_u8(b));
      let high = vmull_high_u8(a, b);
      let round = vdupq_n_u16(128);
      vcombine_u8(
        vshrn_n_u16::<8>(vaddq_u16(vaddq_u16(low, round), vshrq_n_u16::<8>(low))),
        vshrn_n_u16::<8>(vaddq_u16(vaddq_u16(high, round), vshrq_n_u16::<8>(high))),
      )
    }
  }

  // SAFETY: NEON is baseline on AArch64. Fixed arrays provide exactly the
  // 64 destination bytes and 16 mask bytes loaded/stored, with no aliasing.
  unsafe {
    let coverage = vld1q_u8(mask.as_ptr());
    if vmaxvq_u8(coverage) == 0 {
      return;
    }
    let src_alpha = multiply(vdupq_n_u8(color[3]), coverage);
    let inverse = vsubq_u8(vdupq_n_u8(255), src_alpha);
    let old = vld4q_u8(dst.as_ptr().cast());
    let empty_dst = vceqzq_u8(old.3);
    let empty_src = vceqzq_u8(src_alpha);
    let channel = |old, src| {
      let over = vqaddq_u8(src, multiply(old, inverse));
      vbslq_u8(empty_src, old, vbslq_u8(empty_dst, src, over))
    };
    let output = uint8x16x4_t(
      channel(old.0, multiply(vdupq_n_u8(color[0]), coverage)),
      channel(old.1, multiply(vdupq_n_u8(color[1]), coverage)),
      channel(old.2, multiply(vdupq_n_u8(color[2]), coverage)),
      channel(old.3, src_alpha),
    );
    vst4q_u8(dst.as_mut_ptr().cast(), output);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn all_alpha_pairs_match_scalar_including_zero_and_saturation() {
    let mask: Vec<u8> = (0..=255).collect();
    for alpha in 0..=255 {
      for rgb in [[0, 0, 0], [12, 34, 56], [255, 128, 254]] {
        let color = [rgb[0], rgb[1], rgb[2], alpha];
        for destination_alpha in [0, 113, 255] {
          let mut actual: Vec<[u8; 4]> = (0u8..=255)
            .map(|i| [i, i.wrapping_mul(79), i.wrapping_add(15), destination_alpha])
            .collect();
          let mut expected = actual.clone();
          scalar(&mut expected, &mask, color);
          blend(&mut actual, &mask, color);
          assert_eq!(actual, expected, "color={color:?} destination alpha={destination_alpha}");
        }
      }
    }
  }

  #[test]
  fn mixed_spans_match_scalar_at_every_tail_and_alignment() {
    for offset in 0..16 {
      for len in 0..130 {
        let mut actual = vec![[31, 91, 0, 123]; offset + len + 16];
        let mut expected = actual.clone();
        let mask: Vec<u8> = (0..offset + len).map(|i| (i * 67 + i / 11) as u8).collect();
        let color = [34, 12, 72, 127];
        scalar(&mut expected[offset..offset + len], &mask[offset..], color);
        blend(&mut actual[offset..offset + len], &mask[offset..], color);
        assert_eq!(actual, expected);
      }
    }
  }
}
