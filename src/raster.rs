use crate::PngCompression;
use std::{borrow::Cow, fmt};

/// An owned raster image with tightly packed, row-major RGBA8 pixels.
///
/// Rows run top to bottom with no padding. Alpha is straight (not premultiplied),
/// matching decoded PNG output. Rendering performs the same odd-scale
/// downsampling as PNG; no image encoder or decoder is involved.
pub struct RasterImage(pub(crate) image::RgbaImage);

impl fmt::Debug for RasterImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RasterImage")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

impl RasterImage {
    /// Width in output pixels.
    pub fn width(&self) -> u32 {
        self.0.width()
    }

    /// Height in output pixels.
    pub fn height(&self) -> u32 {
        self.0.height()
    }

    /// Borrow the RGBA8 pixels. Each row occupies `width() * 4` bytes.
    pub fn pixels(&self) -> &[u8] {
        self.0.as_raw()
    }

    /// Take ownership of the pixel allocation without copying it.
    pub fn into_pixels(self) -> Vec<u8> {
        self.0.into_raw()
    }

    /// Take the same allocation as BGRA8, swapping red and blue in place.
    ///
    /// Alpha stays straight. This is useful for native image APIs such as GPUI;
    /// it does not encode, allocate another pixel buffer, or premultiply alpha.
    pub fn into_bgra_pixels(self) -> Vec<u8> {
        let mut pixels = self.into_pixels();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        pixels
    }

    pub(crate) fn encode_png(self, compression: PngCompression) -> anyhow::Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut encoder = png::Encoder::new(&mut data, self.width(), self.height());
        let opaque = crate::pixels::is_opaque(self.pixels());
        let pixels = if opaque {
            Cow::Owned(crate::pixels::to_rgb(self.pixels()))
        } else {
            Cow::Borrowed(self.pixels())
        };
        encoder.set_color(if opaque {
            png::ColorType::Rgb
        } else {
            png::ColorType::Rgba
        });
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(match compression {
            PngCompression::Fast => png::Compression::Fast,
            PngCompression::Balanced => png::Compression::Balanced,
        });
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&pixels)?;
        writer.finish()?;
        Ok(data)
    }
}
