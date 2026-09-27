//! The exact 2:1 RGBA8 Lanczos3 operation used for odd export scales.
//!
//! Keep image 0.25's vertical-then-horizontal f32 accumulation and rounding,
//! but stream one float scanline instead of allocating a full float image and
//! walking it column-first. Contiguous channels permit compiler SIMD.
use image::RgbaImage;
use rayon::prelude::*;
use std::sync::OnceLock;

struct Weights {
    first: usize,
    values: [f32; 12],
    len: usize,
}

fn weights(input: u32) -> Vec<Weights> {
    (0..input / 2)
        .map(|out| {
            let center = (out as f32 + 0.5) * 2.0;
            let first = ((center - 6.0).floor() as i64).clamp(0, i64::from(input) - 1);
            let end = ((center + 6.0).ceil() as i64).clamp(first + 1, i64::from(input));
            let mut values = [0.0; 12];
            let mut sum = 0.0;
            for (i, value) in values[..(end - first) as usize].iter_mut().enumerate() {
                let x = ((first as usize + i) as f32 - (center - 0.5)) / 2.0;
                *value = if x.abs() < 3.0 {
                    sinc(x) * sinc(x / 3.0)
                } else {
                    0.0
                };
                sum += *value;
            }
            for value in &mut values[..(end - first) as usize] {
                *value /= sum;
            }
            Weights {
                first: first as usize,
                values,
                len: (end - first) as usize,
            }
        })
        .collect()
}

fn sinc(x: f32) -> f32 {
    let angle = x * std::f32::consts::PI;
    if x == 0.0 { 1.0 } else { angle.sin() / angle }
}

pub(crate) fn half(image: &RgbaImage) -> RgbaImage {
    let (width, height) = image.dimensions();
    // Internal odd-scale canvases are even-sized; preserve the generic fallback
    // if this helper ever receives another shape.
    if width < 2 || height < 2 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return image::imageops::resize(
            image,
            width / 2,
            height / 2,
            image::imageops::FilterType::Lanczos3,
        );
    }
    let horizontal = weights(width);
    let vertical = weights(height);
    let stride = width as usize * 4;
    let mut output = RgbaImage::new(width / 2, height / 2);
    // One process-wide pool bounds worker count across simultaneous Rust/Node
    // calls. A band needs only one float scanline of scratch memory.
    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    if u64::from(width) * u64::from(height) >= 256 * 1024
        && let Some(pool) = POOL.get_or_init(|| {
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(4));
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .thread_name(|i| format!("monoprint-resize-{i}"))
                .build()
                .ok()
        })
    {
        let rows = vertical.len().div_ceil(pool.current_num_threads());
        pool.install(|| {
            output
                .as_mut()
                .par_chunks_mut(stride / 2 * rows)
                .enumerate()
                .for_each(|(band, destination)| {
                    sample_rows(
                        image.as_raw(),
                        stride,
                        &horizontal,
                        &vertical[band * rows..],
                        destination,
                    );
                });
        });
    } else {
        sample_rows(
            image.as_raw(),
            stride,
            &horizontal,
            &vertical,
            output.as_mut(),
        );
    }
    output
}

fn sample_rows(
    source: &[u8],
    stride: usize,
    horizontal: &[Weights],
    vertical: &[Weights],
    output: &mut [u8],
) {
    let mut row = vec![0.0f32; stride];
    for (weights, destination) in vertical.iter().zip(output.chunks_exact_mut(stride / 2)) {
        row.fill(0.0);
        for (offset, weight) in weights.values[..weights.len].iter().enumerate() {
            let input = &source[(weights.first + offset) * stride..][..stride];
            for (value, byte) in row.iter_mut().zip(input) {
                *value += f32::from(*byte) * weight;
            }
        }
        for (weights, pixel) in horizontal
            .iter()
            .zip(destination.as_chunks_mut::<4>().0.iter_mut())
        {
            let mut channels = [0.0f32; 4];
            for (offset, weight) in weights.values[..weights.len].iter().enumerate() {
                let source = &row[(weights.first + offset) * 4..][..4];
                for channel in 0..4 {
                    channels[channel] += source[channel] * weight;
                }
            }
            for (byte, value) in pixel.iter_mut().zip(channels) {
                *byte = value.clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_lanczos_matches_generic_resampling_exactly() {
        let mut seed = 0x3489_221bu32;
        for (width, height) in [
            (2, 2),
            (2, 30),
            (34, 2),
            (6, 8),
            (14, 14),
            (128, 90),
            (358, 246),
            (1280, 800),
        ] {
            let image = RgbaImage::from_fn(width, height, |_, _| {
                image::Rgba(std::array::from_fn(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    seed as u8
                }))
            });
            assert_eq!(
                half(&image),
                image::imageops::resize(
                    &image,
                    width / 2,
                    height / 2,
                    image::imageops::FilterType::Lanczos3
                ),
                "{width}x{height}"
            );
        }
    }
}
