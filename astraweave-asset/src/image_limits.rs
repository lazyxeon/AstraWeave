//! One image decoder. Checked-in CC0 maps are at most 1024².
//! A 4096-pixel edge is four times that. 4096² RGBA8 is 64 MiB, which is
//! also the allocation cap.

use anyhow::{bail, Context, Result};
use std::io::{Cursor, Read};
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

/// Read a file without exceeding `cap`.
///
/// A regular file longer than `cap` is rejected from its metadata before the
/// read. A non-regular file stops at `cap`.
pub fn read_capped_file(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path).with_context(|| format!("open {}", path.display()))?;
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    if meta.is_file() {
        if meta.len() > cap {
            bail!("{} is {} bytes, cap is {cap}", path.display(), meta.len());
        }
        let mut bytes = Vec::new();
        file.take(meta.len())
            .read_to_end(&mut bytes)
            .with_context(|| format!("read {}", path.display()))?;
        return Ok(bytes);
    }
    let mut bytes = Vec::new();
    file.take(cap)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {}", path.display()))?;
    if bytes.len() as u64 == cap {
        bail!("{} exceeds {cap} bytes", path.display());
    }
    Ok(bytes)
}

/// Read an image file without exceeding [`MAX_IMAGE_ALLOC`].
///
/// A regular file longer than the cap is rejected from its metadata, before
/// any byte is read, and is then read only up to that length. A fifo or other
/// non-regular file stops at the cap. It is not probed one byte past the cap,
/// which would block if the writer stalled after exactly that many bytes.
pub fn read_limited_image_bytes(path: &Path) -> Result<Vec<u8>> {
    let cap = MAX_IMAGE_ALLOC;
    let meta = std::fs::metadata(path).with_context(|| format!("open image {}", path.display()))?;
    let file =
        std::fs::File::open(path).with_context(|| format!("open image {}", path.display()))?;
    if meta.is_file() {
        if meta.len() > cap {
            bail!("image is {} bytes, cap is {cap}", meta.len());
        }
        let mut bytes = Vec::new();
        file.take(meta.len())
            .read_to_end(&mut bytes)
            .with_context(|| format!("read image {}", path.display()))?;
        return Ok(bytes);
    }
    let mut bytes = Vec::new();
    file.take(cap)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read image {}", path.display()))?;
    if bytes.len() as u64 == cap {
        bail!("image exceeds {cap} bytes");
    }
    Ok(bytes)
}

/// Open an image file with the same caps as [`decode_limited_image`].
pub fn open_limited_image(path: impl AsRef<Path>) -> Result<image::DynamicImage> {
    let path = path.as_ref();
    let bytes = read_limited_image_bytes(path)?;
    decode_limited_image(&bytes).with_context(|| format!("decode image {}", path.display()))
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

    #[test]
    fn open_limited_image_rejects_a_file_over_64_mib() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("huge.bin");
        let file = std::fs::File::create(&path).expect("create");
        let cap = MAX_IMAGE_ALLOC;
        file.set_len(cap + 1).expect("set_len");
        drop(file);
        let err = open_limited_image(&path).expect_err("oversized file must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&cap.to_string()) && msg.contains(&(cap + 1).to_string()),
            "{msg}"
        );
    }
}
