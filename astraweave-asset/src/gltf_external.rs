//! Jailed glTF buffer and image loads.
//!
//! Callers use [`load_jailed_gltf`] instead of `import_buffers` / `import_images`.
//! The model directory is opened once per load, and every file URI is read
//! from that handle, one component at a time.
//! Buffer and image URI bytes share one total, and the URI count is capped.

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use std::path::{Component, Path, PathBuf};

/// External buffer and image URIs, including `data:` URIs.
pub const MAX_EXTERNAL_URIS: usize = 64;

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
    let relative = relative_file_uri(uri)?;
    let path = astraweave_security::path::safe_under(base, &relative)
        .map_err(|err| anyhow!("path traversal rejected: {err}"))?;
    Ok(Some(path))
}

/// Read a jailed file without resolving any component.
///
/// The parent is opened component by component from the filesystem root.
/// A link in a parent directory is refused, and a link at the file is refused.
/// The model file uses `read_path`, which resolves the caller-supplied path once.
///
/// The read stops at `max_bytes`. The allocation follows the bytes read.
pub fn read_jailed_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    crate::bounded_open::read_unresolved(path, max_bytes, "")
}

pub(crate) fn relative_file_uri(uri: &str) -> Result<PathBuf> {
    if uri.contains(':') || uri.contains('%') || uri.contains('\\') || uri.contains('\0') {
        bail!("path traversal rejected: gltf uri is not a relative file");
    }
    let path_part = uri.split('?').next().unwrap_or(uri);
    if path_part.is_empty() {
        bail!("path traversal rejected: gltf uri is not a relative file");
    }
    let path = PathBuf::from(path_part);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("path traversal rejected: gltf uri is not a relative file");
    }
    Ok(path)
}

/// Refuse a document with more external buffer and image URIs than
/// [`MAX_EXTERNAL_URIS`].
pub fn reject_external_uri_count(document: &gltf::Document) -> Result<()> {
    if external_uri_count(document) > MAX_EXTERNAL_URIS {
        bail!("too many external uris");
    }
    Ok(())
}

fn external_uri_count(document: &gltf::Document) -> usize {
    let buffers = document
        .buffers()
        .filter(|buffer| matches!(buffer.source(), gltf::buffer::Source::Uri(_)))
        .count();
    let images = document
        .images()
        .filter(|image| matches!(image.source(), gltf::image::Source::Uri { .. }))
        .count();
    buffers + images
}

/// Parse `bytes`, then load buffers and images from `base`.
///
/// A `data:` URI is decoded in memory. A file URI is read from `base`.
/// Every URI's bytes count toward `max_external_bytes`, and the URI count
/// stops at [`MAX_EXTERNAL_URIS`].
pub fn load_jailed_gltf(
    base: &Path,
    bytes: &[u8],
    max_external_bytes: u64,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    crate::gltf_loader::reject_short_glb_header(bytes)?;
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).context("parse gltf")?;
    reject_external_uri_count(&document)?;
    let root = open_model_dir(base)?;
    let mut budget = Budget {
        cap: max_external_bytes,
        spent: 0,
    };
    let mut blob = blob;
    let buffers = read_buffers(&document, &root, &mut blob, &mut budget)?;
    let images = read_images(&document, &root, &buffers, &mut budget)?;
    Ok((document, buffers, images))
}

fn open_model_dir(base: &Path) -> Result<crate::bounded_open::OpenedRoot> {
    match crate::bounded_open::OpenedRoot::open(base) {
        Ok(root) => Ok(root),
        Err(err)
            if err.kind() == std::io::ErrorKind::InvalidInput
                && err.to_string().contains("not a regular file") =>
        {
            bail!("{} is not a regular file", base.display())
        }
        Err(err) => Err(err).with_context(|| format!("open {}", base.display())),
    }
}

/// Read one relative file URI from `base`, opening that directory once.
pub(crate) fn read_opened_uri(base: &Path, uri: &str, max_bytes: u64) -> Result<Vec<u8>> {
    let root = open_model_dir(base)?;
    let mut budget = Budget {
        cap: max_bytes,
        spent: 0,
    };
    uri_bytes(&root, uri, &mut budget)
}

pub(crate) fn decode_embedded_uri(uri: &str) -> Result<Vec<u8>> {
    decode_data_uri(uri)
}

struct Budget {
    cap: u64,
    spent: u64,
}

impl Budget {
    fn remaining(&self) -> u64 {
        self.cap.saturating_sub(self.spent)
    }

    fn charge(&mut self, nbytes: u64) -> Result<()> {
        let next = self
            .spent
            .checked_add(nbytes)
            .context("external byte total")?;
        if next > self.cap {
            bail!("external bytes exceed {}", self.cap);
        }
        self.spent = next;
        Ok(())
    }
}

fn uri_bytes(
    root: &crate::bounded_open::OpenedRoot,
    uri: &str,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    if uri.starts_with("data:") {
        let encoded = data_uri_encoded(uri)?;
        let decoded_len = decoded_base64_len(encoded)?;
        budget.charge(decoded_len)?;
        return decode_data_uri(uri);
    }
    let relative = relative_file_uri(uri)?;
    let bytes = root.read(&relative, budget.remaining(), "", &relative)?;
    budget.charge(bytes.len() as u64)?;
    Ok(bytes)
}

fn data_uri_encoded(uri: &str) -> Result<&str> {
    let rest = uri
        .strip_prefix("data:")
        .context("gltf data uri is missing its prefix")?;
    if let Some((_, data)) = rest.split_once(";base64,") {
        Ok(data)
    } else if let Some((_, data)) = rest.split_once(',') {
        Ok(data)
    } else {
        Ok(rest)
    }
}

fn decoded_base64_len(encoded: &str) -> Result<u64> {
    let bytes = encoded.as_bytes();
    let pad = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if pad > 2 {
        bail!("gltf data uri is not base64");
    }
    let len = bytes.len() as u64;
    let pad = pad as u64;
    if len % 4 != 0 {
        return match len % 4 {
            2 if pad == 0 => Ok(len / 4 * 3 + 1),
            3 if pad == 0 => Ok(len / 4 * 3 + 2),
            _ => bail!("gltf data uri is not base64"),
        };
    }
    Ok(len / 4 * 3 - pad)
}

fn decode_data_uri(uri: &str) -> Result<Vec<u8>> {
    let encoded = data_uri_encoded(uri)?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("gltf data uri is not base64")
}

fn read_buffers(
    document: &gltf::Document,
    root: &crate::bounded_open::OpenedRoot,
    blob: &mut Option<Vec<u8>>,
    budget: &mut Budget,
) -> Result<Vec<gltf::buffer::Data>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            gltf::buffer::Source::Bin => {
                blob.take().context("GLB buffer is missing its BIN chunk")?
            }
            gltf::buffer::Source::Uri(uri) => uri_bytes(root, uri, budget)?,
        };
        // Compare the bytes on hand before the 4-byte pad. A buffer one to
        // three bytes short would otherwise grow to `byteLength` and pass.
        let declared = buffer.length();
        if data.len() < declared {
            bail!(
                "buffer {} is {} bytes; byteLength is {declared}",
                buffer.index(),
                data.len()
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
    root: &crate::bounded_open::OpenedRoot,
    buffers: &[gltf::buffer::Data],
    budget: &mut Budget,
) -> Result<Vec<gltf::image::Data>> {
    let mut images = Vec::new();
    for image in document.images() {
        let encoded = match image.source() {
            gltf::image::Source::Uri { uri, .. } => uri_bytes(root, uri, budget)?,
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
    let image = crate::decode_limited_image(bytes).context("decode gltf image")?;
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

    #[test]
    fn external_reads_respect_total_budget() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.bin"), b"aaaa").expect("a");
        std::fs::write(dir.path().join("b.bin"), b"bbbb").expect("b");
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "buffers": [
                {"uri": "a.bin", "byteLength": 4},
                {"uri": "b.bin", "byteLength": 4}
            ]
        }"#;
        let (_doc, buffers, _) = load_jailed_gltf(dir.path(), gltf, 8).expect("within total");
        assert_eq!(buffers.len(), 2);
        let err = load_jailed_gltf(dir.path(), gltf, 6).expect_err("over total");
        let msg = err.to_string();
        assert!(msg.contains("limit is") || msg.contains("exceed"), "{msg}");
    }

    #[test]
    fn external_reads_respect_uri_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut buffers = Vec::new();
        for index in 0..MAX_EXTERNAL_URIS {
            let name = format!("b{index}.bin");
            std::fs::write(dir.path().join(&name), b"x").expect("bin");
            buffers.push(serde_json::json!({ "uri": name, "byteLength": 1 }));
        }
        let within = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": buffers,
        });
        let bytes = serde_json::to_vec(&within).expect("json");
        let (_doc, loaded, _) = load_jailed_gltf(dir.path(), &bytes, 1024).expect("uri count");
        assert_eq!(loaded.len(), MAX_EXTERNAL_URIS);

        let name = "extra.bin";
        std::fs::write(dir.path().join(name), b"x").expect("extra");
        buffers.push(serde_json::json!({ "uri": name, "byteLength": 1 }));
        let over = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": buffers,
        });
        let bytes = serde_json::to_vec(&over).expect("json");
        let err = load_jailed_gltf(dir.path(), &bytes, 1024).expect_err("uri count");
        assert!(err.to_string().contains("too many"), "{err}");
    }

    #[test]
    fn external_reads_respect_image_limits() {
        let image = image::RgbImage::from_pixel(5_000, 1, image::Rgb([0, 0, 0]));
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("png");
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("wide.png"), &png).expect("write");
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "images": [{"uri": "wide.png"}]
        }"#;
        let err = load_jailed_gltf(dir.path(), gltf, 1024 * 1024).expect_err("wide");
        let msg = format!("{err:#}").to_lowercase();
        assert!(
            msg.contains("limit") || msg.contains("dimension") || msg.contains("4096"),
            "{msg}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn post_read_check_catches_understated_procfs_size() {
        let proc_cmdline =
            std::path::PathBuf::from(format!("/proc/{}/cmdline", std::process::id()));
        let err = read_jailed_file(&proc_cmdline, 8).expect_err("post-read cap");
        assert!(err.to_string().contains("exceeds 8 bytes"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn external_reads_reject_a_non_regular_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pipe = dir.path().join("pipe.bin");
        let status = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo");
        assert!(status.success(), "mkfifo");
        let started = std::time::Instant::now();
        let err = read_jailed_file(&pipe, 64).expect_err("pipe");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "read_jailed_file blocked"
        );
        assert!(err.to_string().contains("not a regular file"), "{err}");

        let real = dir.path().join("real.bin");
        std::fs::write(&real, b"real").expect("real");
        let link = dir.path().join("link.bin");
        std::os::unix::fs::symlink(&real, &link).expect("link");
        let err = read_jailed_file(&link, 64).expect_err("link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
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

    #[test]
    fn backslash_uri_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = jail_gltf_uri(dir.path(), r"a\b").expect_err("slash");
        assert!(
            err.to_string().contains("gltf uri is not a relative file"),
            "{err}"
        );
    }

    #[test]
    fn nul_uri_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = jail_gltf_uri(dir.path(), "a\0b").expect_err("nul");
        assert!(
            err.to_string().contains("gltf uri is not a relative file"),
            "{err}"
        );
    }

    #[test]
    fn data_uri_respects_total_budget() {
        let dir = tempfile::tempdir().expect("tempdir");
        let over = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"data:application/octet-stream;base64,!!!!","byteLength":3}]}"#;
        let err = load_jailed_gltf(dir.path(), over, 1).expect_err("budget");
        let msg = format!("{err:#}");
        assert!(msg.contains("exceed"), "{msg}");
        assert!(!msg.contains("not base64"), "{msg}");

        let fits = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"data:application/octet-stream;base64,YWI=","byteLength":2}]}"#;
        let (_doc, buffers, _) = load_jailed_gltf(dir.path(), fits, 2).expect("fits");
        assert_eq!(&buffers[0].0[..2], b"ab");
    }

    #[test]
    fn short_buffer_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let gltf = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"data:application/octet-stream;base64,YWI=","byteLength":8}]}"#;
        let err = load_jailed_gltf(dir.path(), gltf, 64).expect_err("short");
        assert!(err.to_string().contains("byteLength"), "{err}");
    }

    #[test]
    fn buffer_one_byte_under_a_multiple_of_four_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Three payload bytes and byteLength 4. Padding to 4 would hide the gap.
        let gltf = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"data:application/octet-stream;base64,YWJj","byteLength":4}]}"#;
        let err = load_jailed_gltf(dir.path(), gltf, 64).expect_err("short of 4");
        let msg = err.to_string();
        assert!(msg.contains("is 3 bytes"), "{msg}");
        assert!(msg.contains("byteLength is 4"), "{msg}");
    }

    #[test]
    fn buffer_at_byte_length_is_padded_to_four() {
        let dir = tempfile::tempdir().expect("tempdir");
        let gltf = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"data:application/octet-stream;base64,YWJj","byteLength":3}]}"#;
        let (_doc, buffers, _) = load_jailed_gltf(dir.path(), gltf, 64).expect("exact");
        assert_eq!(buffers[0].0.as_slice(), b"abc\0");
    }

    #[test]
    fn json_external_reads_use_the_given_base() {
        let dir = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("other");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "buffers": [{"uri": "box.bin", "byteLength": 4}]
        }"#;
        let (_doc, buffers, _) = load_jailed_gltf(dir.path(), gltf, 64).expect("base");
        assert_eq!(&buffers[0].0[..], b"box!");
        let err = load_jailed_gltf(other.path(), gltf, 64).expect_err("other base");
        let msg = format!("{err:#}");
        assert!(msg.contains("box.bin") || msg.contains("open"), "{msg}");
        let mesh_err = crate::gltf_loader::load_first_mesh_and_material(dir.path(), gltf)
            .expect_err("no mesh");
        assert!(mesh_err.to_string().contains("No meshes"), "{mesh_err}");
        let other_err = crate::gltf_loader::load_first_mesh_and_material(other.path(), gltf)
            .expect_err("missing sibling");
        assert!(!other_err.to_string().contains("No meshes"), "{other_err}");
    }

    #[test]
    fn nested_relative_uri_loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).expect("sub");
        std::fs::write(sub.join("box.bin"), b"nest").expect("bin");
        let gltf =
            br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"sub/box.bin","byteLength":4}]}"#;
        let (_doc, buffers, _) = load_jailed_gltf(dir.path(), gltf, 64).expect("nested");
        assert_eq!(&buffers[0].0[..4], b"nest");
    }

    #[cfg(unix)]
    #[test]
    fn uri_parent_directory_link_is_not_followed() {
        let dir = tempfile::tempdir().expect("dir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("loot.bin"), b"loot").expect("loot");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).expect("link");
        let path = dir.path().join("sub").join("loot.bin");
        let err = read_jailed_file(&path, 64).expect_err("parent link");
        assert!(err.to_string().contains("not a regular file"), "{err}");

        let gltf =
            br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"sub/loot.bin","byteLength":4}]}"#;
        let err = load_jailed_gltf(dir.path(), gltf, 64).expect_err("load parent link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn directory_replaced_between_jail_and_read_is_refused() {
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        std::fs::write(jail.join("box.bin"), b"good").expect("good");
        let outside = parent.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        std::fs::write(outside.join("box.bin"), b"evil").expect("evil");

        let checked = jail_gltf_uri(&jail, "box.bin")
            .expect("jail")
            .expect("file");
        let parked = parent.path().join("parked");
        std::fs::rename(&jail, &parked).expect("park");
        std::os::unix::fs::symlink(&outside, &jail).expect("swap");

        let err = read_jailed_file(&checked, 64).expect_err("replaced directory");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }
}
