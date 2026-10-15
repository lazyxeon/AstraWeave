//! One image decoder. Checked-in CC0 maps are at most 1024².
//! A 4096-pixel edge is four times that. 4096² RGBA8 is 64 MiB, which is
//! also the allocation cap.

use anyhow::{Context, Result};
use std::io::Cursor;
use std::path::Path;

/// Longest edge `open_limited_image` and `decode_limited_image` accept.
pub const MAX_IMAGE_EDGE: u32 = 4096;
/// Decoded-buffer cap, equal to a 4096² RGBA8 image.
pub const MAX_IMAGE_ALLOC: u64 = 64 * 1024 * 1024;

/// Limits shared by every image decode.
pub fn capped_image_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_EDGE);
    limits.max_image_height = Some(MAX_IMAGE_EDGE);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC);
    limits
}

/// Decode image bytes. Hostile dimensions never become a pixel buffer.
pub fn decode_limited_image(bytes: &[u8]) -> Result<image::DynamicImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("guess image format")?;
    reader.limits(capped_image_limits());
    reader.decode().context("decode image")
}

/// Open an image file with the same caps as [`decode_limited_image`].
pub fn open_limited_image(path: impl AsRef<Path>) -> Result<image::DynamicImage> {
    let path = path.as_ref();
    let mut reader =
        image::ImageReader::open(path).with_context(|| format!("open image {}", path.display()))?;
    reader.limits(capped_image_limits());
    reader
        .decode()
        .with_context(|| format!("decode image {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_limited_image_rejects_a_wide_png() {
        let image = image::RgbImage::from_pixel(5_000, 1, image::Rgb([0, 0, 0]));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode png");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wide.png");
        std::fs::write(&path, &bytes).expect("write png");
        let err = open_limited_image(&path).expect_err("wide image must be rejected");
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("limit") || msg.contains("dimension") || msg.contains("4096"),
            "{msg}"
        );
    }
}
