//! Lossless preparation of RGBA pixels for the PNG encoder.

pub(crate) fn is_opaque(rgba: &[u8]) -> bool {
    assert!(rgba.len().is_multiple_of(4));
    // Bound each scan so transparent images still exit early.
    rgba.chunks(4096).all(opaque_block)
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn opaque_block(rgba: &[u8]) -> bool {
    use std::arch::aarch64::*;
    let (runs, tail) = rgba.as_chunks::<64>();
    // SAFETY: NEON is baseline on AArch64. Each unaligned load reads exactly
    // sixteen RGBA pixels from a complete 64-byte chunk.
    let opaque = unsafe {
        let mut minimum = vdupq_n_u8(255);
        for run in runs {
            minimum = vminq_u8(minimum, vld4q_u8(run.as_ptr()).3);
        }
        vminvq_u8(minimum) == 255
    };
    opaque && opaque_scalar(tail)
}

#[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
fn opaque_block(rgba: &[u8]) -> bool {
    opaque_scalar(rgba)
}

fn opaque_scalar(rgba: &[u8]) -> bool {
    rgba.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255)
}

pub(crate) fn to_rgb(rgba: &[u8]) -> Vec<u8> {
    assert!(rgba.len().is_multiple_of(4));
    let mut rgb = vec![0; rgba.len() / 4 * 3];
    pack(rgba, &mut rgb);
    rgb
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn pack(rgba: &[u8], rgb: &mut [u8]) {
    use std::arch::aarch64::*;
    let (runs, tail) = rgba.as_chunks::<64>();
    let (out, out_tail) = rgb.as_chunks_mut::<48>();
    // SAFETY: matching complete chunks provide 64 readable and 48 writable
    // bytes. NEON supports unaligned loads/stores; the buffers cannot alias.
    unsafe {
        for (input, output) in runs.iter().zip(out) {
            let p = vld4q_u8(input.as_ptr());
            vst3q_u8(output.as_mut_ptr(), uint8x16x3_t(p.0, p.1, p.2));
        }
    }
    pack_scalar(tail, out_tail);
}

#[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
fn pack(rgba: &[u8], rgb: &mut [u8]) {
    pack_scalar(rgba, rgb);
}

fn pack_scalar(rgba: &[u8], rgb: &mut [u8]) {
    for (input, output) in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rgb.as_chunks_mut::<3>().0)
    {
        output.copy_from_slice(&input[..3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_matches_scalar_at_every_tail_and_alignment() {
        for offset in 0..16 {
            for pixels in 0..130 {
                let input: Vec<u8> = (0..offset + pixels * 4)
                    .map(|i| (i * 79 + i / 7) as u8)
                    .collect();
                let rgba = &input[offset..];
                let mut expected = vec![0; pixels * 3];
                pack_scalar(rgba, &mut expected);
                // Guards also catch writes past the output slice.
                let mut actual = vec![0xa5; offset + pixels * 3 + 16];
                pack(rgba, &mut actual[offset..offset + pixels * 3]);
                assert_eq!(&actual[offset..offset + pixels * 3], expected);
                assert!(actual[..offset].iter().all(|&b| b == 0xa5));
                assert!(actual[offset + pixels * 3..].iter().all(|&b| b == 0xa5));
                assert_eq!(to_rgb(rgba), expected);
            }
        }
    }

    #[test]
    fn opacity_checks_every_lane_tail_and_scan_boundary() {
        for offset in 0..16 {
            for pixels in [0, 1, 15, 16, 17, 31, 32, 33, 1023, 1024, 1025, 2051] {
                let mut input = vec![255; offset + pixels * 4];
                assert!(is_opaque(&input[offset..]));
                for pixel in 0..pixels {
                    for alpha in [0, 1, 127, 254] {
                        input[offset + pixel * 4 + 3] = alpha;
                        assert!(!is_opaque(&input[offset..]));
                    }
                    input[offset + pixel * 4 + 3] = 255;
                }
                for pixel in 0..pixels {
                    input[offset + pixel * 4] = 0;
                }
                assert!(is_opaque(&input[offset..]));
                assert!(opaque_scalar(&input[offset..]));
            }
        }
    }
}
