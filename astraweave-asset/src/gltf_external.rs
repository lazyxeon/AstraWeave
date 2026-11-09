//! Jailed glTF buffer and image loads.
//!
//! gltf 1.4.1 `Scheme::read` percent-decodes a relative URI and, when a base
//! directory is set, opens a `file:` URI with `read_to_end`. That allocates
//! `st_size + 1` and ignores the base. Callers use [`load_jailed_gltf`]
//! instead of `import_buffers` / `import_images`.

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use std::io::Read;
use std::path::{Path, PathBuf};

/// `Ok(None)` is a `data:` URI, which stays in memory.
/// `Ok(Some(path))` is a relative file [`astraweave_security::path::safe_under`]
/// accepted under `base`.
///
/// Any other URI that contains `:`, `%`, `\`, or NUL is rejected before the
/// path check. `safe_under` rejects an absolute path and a `..` component.
/// `\` is rejected on every platform so a backslash does not become a
/// separator on Windows only.
pub fn jail_gltf_uri(base: &Path, uri: &str) -> Result<Option<PathBuf>> {
    if uri.starts_with("data:") {
        return Ok(None);
    }
    if uri.contains(':') || uri.contains('%') || uri.contains('\\') || uri.contains('\0') {
        bail!("path traversal rejected: gltf uri is not a relative file");
    }
    let path_part = uri.split('?').next().unwrap_or(uri);
    if path_part.is_empty() {
        bail!("path traversal rejected: gltf uri is not a relative file");
    }
    let path = astraweave_security::path::safe_under(base, Path::new(path_part))
        .map_err(|err| anyhow!("path traversal rejected: {err}"))?;
    Ok(Some(path))
}

/// Read a jailed file. `metadata().is_file()` runs before `File::open`, then
/// the read stops at `max_bytes`. The allocation follows the bytes read, not
/// `st_size + 1`.
pub fn read_jailed_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if !meta.is_file() {
        bail!("{} is not a regular file", path.display());
    }
    if meta.len() > max_bytes {
        bail!(
            "{} is {} bytes; limit is {max_bytes}",
            path.display(),
            meta.len()
        );
    }
    let mut file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut bytes = Vec::new();
    Read::take(&mut file, max_bytes + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {}", path.display()))?;
    if bytes.len() as u64 > max_bytes {
        bail!("{} exceeds {max_bytes} bytes", path.display());
    }
    Ok(bytes)
}

/// Parse `bytes`, then load buffers and images from `base`.
///
/// A `data:` URI is decoded in memory. A file URI goes through [`jail_gltf_uri`]
/// and [`read_jailed_file`] with `max_external_bytes`.
pub fn load_jailed_gltf(
    base: &Path,
    bytes: &[u8],
    max_external_bytes: u64,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).context("parse gltf")?;
    let mut blob = blob;
    let buffers = read_buffers(&document, base, &mut blob, max_external_bytes)?;
    let images = read_images(&document, base, &buffers, max_external_bytes)?;
    Ok((document, buffers, images))
}

fn uri_bytes(base: &Path, uri: &str, max_bytes: u64) -> Result<Vec<u8>> {
    match jail_gltf_uri(base, uri)? {
        None => decode_data_uri(uri),
        Some(path) => read_jailed_file(&path, max_bytes),
    }
}

fn decode_data_uri(uri: &str) -> Result<Vec<u8>> {
    let rest = uri
        .strip_prefix("data:")
        .context("gltf data uri is missing its prefix")?;
    let encoded = if let Some((_, data)) = rest.split_once(";base64,") {
        data
    } else if let Some((_, data)) = rest.split_once(',') {
        data
    } else {
        rest
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("gltf data uri is not base64")
}

fn read_buffers(
    document: &gltf::Document,
    base: &Path,
    blob: &mut Option<Vec<u8>>,
    max_external_bytes: u64,
) -> Result<Vec<gltf::buffer::Data>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            gltf::buffer::Source::Bin => {
                blob.take().context("GLB buffer is missing its BIN chunk")?
            }
            gltf::buffer::Source::Uri(uri) => uri_bytes(base, uri, max_external_bytes)?,
        };
        if data.len() < buffer.length() {
            bail!(
                "buffer {} is {} bytes; glTF byteLength is {}",
                buffer.index(),
                data.len(),
                buffer.length()
            );
        }
        while data.len() % 4 != 0 {
            data.push(0);
        }
        buffers.push(gltf::buffer::Data(data));
    }
    Ok(buffers)
}

fn read_images(
    document: &gltf::Document,
    base: &Path,
    buffers: &[gltf::buffer::Data],
    max_external_bytes: u64,
) -> Result<Vec<gltf::image::Data>> {
    let mut images = Vec::new();
    for image in document.images() {
        let encoded = match image.source() {
            gltf::image::Source::Uri { uri, .. } => uri_bytes(base, uri, max_external_bytes)?,
            gltf::image::Source::View { view, .. } => {
                let data = buffers
                    .get(view.buffer().index())
                    .context("image buffer view has no buffer")?;
                let start = view.offset();
                let end = start
                    .checked_add(view.length())
                    .context("image view length overflows")?;
                data.0
                    .get(start..end)
                    .context("image view is outside its buffer")?
                    .to_vec()
            }
        };
        images.push(decode_gltf_image(&encoded)?);
    }
    Ok(images)
}

fn decode_gltf_image(bytes: &[u8]) -> Result<gltf::image::Data> {
    let image = image::load_from_memory(bytes).context("decode gltf image")?;
    let format = match image.color() {
        image::ColorType::L8 => gltf::image::Format::R8,
        image::ColorType::La8 => gltf::image::Format::R8G8,
        image::ColorType::Rgb8 => gltf::image::Format::R8G8B8,
        image::ColorType::Rgba8 => gltf::image::Format::R8G8B8A8,
        image::ColorType::L16 => gltf::image::Format::R16,
        image::ColorType::La16 => gltf::image::Format::R16G16,
        image::ColorType::Rgb16 => gltf::image::Format::R16G16B16,
        image::ColorType::Rgba16 => gltf::image::Format::R16G16B16A16,
        image::ColorType::Rgb32F => gltf::image::Format::R32G32B32FLOAT,
        image::ColorType::Rgba32F => gltf::image::Format::R32G32B32A32FLOAT,
        other => bail!("unsupported gltf image format {other:?}"),
    };
    let width = image.width();
    let height = image.height();
    Ok(gltf::image::Data {
        pixels: image.into_bytes(),
        format,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTILE: &[&str] = &[
        "file:///etc/passwd",
        "file:/%2e%2e/x",
        "%2e%2e/x",
        "../x",
        "/abs",
        r"C:\x",
    ];

    #[test]
    fn hostile_uris_are_rejected_and_a_relative_uri_passes() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        for uri in HOSTILE {
            let err = jail_gltf_uri(dir.path(), uri).expect_err(uri);
            let msg = format!("{err:#}").to_lowercase();
            assert!(
                msg.contains("traversal") || msg.contains("rejected"),
                "{uri}: {msg}"
            );
        }
        let path = jail_gltf_uri(dir.path(), "box.bin")
            .expect("relative")
            .expect("file uri");
        let base = dir.path().canonicalize().expect("base");
        assert!(path.starts_with(&base), "{path:?}");
        assert_eq!(read_jailed_file(&path, 4).expect("read"), b"box!");
    }

    #[test]
    fn data_uri_stays_in_memory() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            jail_gltf_uri(dir.path(), "data:application/octet-stream;base64,Yg==")
                .expect("data")
                .is_none()
        );
    }

    #[test]
    fn read_stops_at_the_cap_before_the_payload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.bin");
        let file = std::fs::File::create(&path).expect("create");
        file.set_len(9).expect("len");
        let err = read_jailed_file(&path, 8).expect_err("cap");
        assert!(err.to_string().contains("limit is 8"), "{err}");
    }

    #[test]
    fn load_rejects_a_sibling_past_the_cap_and_keeps_a_small_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "buffers": [{"uri": "box.bin", "byteLength": 4}]
        }"#;
        let (_doc, buffers, images) = load_jailed_gltf(dir.path(), gltf, 4).expect("within cap");
        assert_eq!(&buffers[0].0[..], b"box!");
        assert!(images.is_empty());
        let err = load_jailed_gltf(dir.path(), gltf, 3).expect_err("past cap");
        assert!(err.to_string().contains("limit is 3"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn directory_symlink_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("loot.bin"), b"loot").expect("loot");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).expect("symlink");
        let err = jail_gltf_uri(dir.path(), "sub/loot.bin").expect_err("symlink dir");
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("traversal") || msg.contains("escapes") || msg.contains("rejected"),
            "{msg}"
        );
    }
}
