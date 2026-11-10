//! Jailed glTF buffer and image loads.
//!
//! Callers use [`load_jailed_gltf`] instead of `import_buffers` / `import_images`.
//! One load opens the model directory once, and that handle is used for the
//! container read, every sibling URI, and every image URI. Each URI component
//! is read from that handle.
//! Buffer and image URI bytes share one total, and the URI count is capped.

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use std::path::{Component, Path, PathBuf};

/// External buffer and image URIs, including `data:` URIs.
pub const MAX_EXTERNAL_URIS: usize = 64;

/// `Ok(None)` is a `data:` URI, which stays in memory.
/// `Ok(Some(path))` is a relative file name under `base`.
///
/// The component check rejects `:`, `%`, `\`, NUL, an empty path, and a parent,
/// root, or prefix component. The URI read uses the model directory handle.
/// `safe_under` is not that read. `\` is rejected on every platform so a
/// backslash does not become a separator on Windows only.
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
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn read_jailed_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
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

/// The model directory opened once for a load.
///
/// One load opens the model directory once, and that handle is used for the
/// container read, every sibling URI, and every image URI.
#[derive(Debug)]
pub struct ModelRoot {
    inner: crate::bounded_open::OpenedRoot,
}

impl ModelRoot {
    pub(crate) fn from_opened(inner: crate::bounded_open::OpenedRoot) -> Self {
        Self { inner }
    }

    /// Open `base`. An empty path is the current directory.
    pub fn open(base: &Path) -> Result<Self> {
        open_model_dir(base).map(Self::from_opened)
    }

    pub(crate) fn read(
        &self,
        relative: &Path,
        max_bytes: u64,
        noun: &str,
        display: &Path,
    ) -> Result<Vec<u8>> {
        self.inner.read(relative, max_bytes, noun, display)
    }
}

/// Shared byte total for every buffer and image URI in one load.
pub(crate) struct ExternalBudget {
    cap: u64,
    spent: u64,
}

impl ExternalBudget {
    pub(crate) fn new(cap: u64) -> Self {
        Self { cap, spent: 0 }
    }

    /// Bytes still under `cap`.
    ///
    /// A read limit that reports the full cap after earlier reads is still
    /// refused by [`Self::charge`], which rejects a total past `cap`.
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

/// Parse `bytes`, then load buffers and images from `base`.
///
/// An empty `base` is the current directory. A `data:` URI is decoded in
/// memory. A file URI is read from the directory handle. Every URI's bytes
/// count toward `max_external_bytes`, and the URI count stops at
/// [`MAX_EXTERNAL_URIS`].
pub fn load_jailed_gltf(
    base: &Path,
    bytes: &[u8],
    max_external_bytes: u64,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    let root = ModelRoot::open(base)?;
    load_jailed_gltf_with_root(&root, bytes, max_external_bytes)
}

/// Parse `bytes` and read every sibling from `root`.
///
/// One load opens the model directory once, and that handle is used for the
/// container read, every sibling URI, and every image URI. This function does
/// not open the directory again.
pub fn load_jailed_gltf_with_root(
    root: &ModelRoot,
    bytes: &[u8],
    max_external_bytes: u64,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    let mut budget = ExternalBudget::new(max_external_bytes);
    load_with_budget(root, bytes, &mut budget)
}

pub(crate) fn load_with_budget(
    root: &ModelRoot,
    bytes: &[u8],
    budget: &mut ExternalBudget,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    crate::gltf_loader::reject_short_glb_header(bytes)?;
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).context("parse gltf")?;
    reject_external_uri_count(&document)?;
    let mut blob = blob;
    let buffers = read_buffers(&document, root, &mut blob, budget)?;
    let images = read_images(&document, root, &buffers, budget)?;
    Ok((document, buffers, images))
}

fn model_base(base: &Path) -> &Path {
    if base.as_os_str().is_empty() {
        Path::new(".")
    } else {
        base
    }
}

fn open_model_dir(base: &Path) -> Result<crate::bounded_open::OpenedRoot> {
    let base = model_base(base);
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

/// Read one URI from `root`, charging `budget`.
///
/// `file_cap` limits one image file read. The read uses the smaller of that
/// cap and the budget still remaining. A `data:` URI charges its decoded
/// length before the bytes are decoded.
pub(crate) fn read_charged_uri(
    root: &ModelRoot,
    uri: &str,
    budget: &mut ExternalBudget,
    file_cap: Option<u64>,
) -> Result<Vec<u8>> {
    uri_bytes(root, uri, budget, file_cap)
}

fn uri_bytes(
    root: &ModelRoot,
    uri: &str,
    budget: &mut ExternalBudget,
    file_cap: Option<u64>,
) -> Result<Vec<u8>> {
    if uri.starts_with("data:") {
        let encoded = data_uri_encoded(uri)?;
        let decoded_len = decoded_base64_len(encoded)?;
        budget.charge(decoded_len)?;
        return decode_data_uri(uri);
    }
    let relative = relative_file_uri(uri)?;
    let limit = match file_cap {
        Some(cap) => budget.remaining().min(cap),
        None => budget.remaining(),
    };
    let bytes = root.read(&relative, limit, "", &relative)?;
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
    root: &ModelRoot,
    blob: &mut Option<Vec<u8>>,
    budget: &mut ExternalBudget,
) -> Result<Vec<gltf::buffer::Data>> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            gltf::buffer::Source::Bin => {
                blob.take().context("GLB buffer is missing its BIN chunk")?
            }
            gltf::buffer::Source::Uri(uri) => uri_bytes(root, uri, budget, None)?,
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
    root: &ModelRoot,
    buffers: &[gltf::buffer::Data],
    budget: &mut ExternalBudget,
) -> Result<Vec<gltf::image::Data>> {
    let mut images = Vec::new();
    for image in document.images() {
        let encoded = match image.source() {
            gltf::image::Source::Uri { uri, .. } => uri_bytes(
                root,
                uri,
                budget,
                Some(crate::image_limits::MAX_IMAGE_ALLOC),
            )?,
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
        let base = std::fs::canonicalize(dir.path()).expect("canonical");
        let path = base.join("big.bin");
        let file = std::fs::File::create(&path).expect("create");
        file.set_len(9).expect("len");
        let err = read_jailed_file(&path, 8).expect_err("cap");
        assert!(err.to_string().contains("limit is 8"), "{err}");
    }

    #[test]
    fn image_file_read_stops_at_its_own_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cap: u64 = 64 * 1024 * 1024;
        let file = std::fs::File::create(dir.path().join("big.png")).expect("create");
        file.set_len(cap + 1).expect("len");
        drop(file);
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "images": [{"uri": "big.png"}]
        }"#;
        let err = load_jailed_gltf(
            dir.path(),
            gltf,
            crate::gltf_loader::MAX_GLB_CONTAINER_BYTES,
        )
        .expect_err("image cap");
        let msg = err.to_string();
        assert!(msg.contains(&format!("limit is {cap}")), "{msg}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn load_under_an_execute_only_ancestor() {
        use std::os::unix::fs::PermissionsExt;
        struct Restore(std::path::PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let ancestor = dir.path().join("ancestor");
        let model = ancestor.join("model");
        std::fs::create_dir_all(&model).expect("model");
        std::fs::write(model.join("mesh.bin"), b"mesh").expect("bin");
        let gltf = model.join("scene.gltf");
        std::fs::write(
            &gltf,
            br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":4}]}"#,
        )
        .expect("gltf");
        let _restore = Restore(ancestor.clone());
        std::fs::set_permissions(&ancestor, std::fs::Permissions::from_mode(0o311)).expect("mode");
        let (bytes, root) =
            crate::gltf_loader::read_gltf_container_with_root(&gltf).expect("container");
        let (_doc, buffers, _) = load_jailed_gltf_with_root(&root, &bytes, 1024).expect("load");
        assert_eq!(&buffers[0].0[..4], b"mesh");
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

    fn one_pixel_png() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([9, 8, 7, 6]));
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("png");
        png
    }

    #[test]
    fn empty_base_reads_a_bare_file_name() {
        let name = format!(
            "aw-empty-base-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        struct Guard(std::path::PathBuf);
        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _guard = Guard(std::path::PathBuf::from(&name));
        std::fs::write(&name, b"bare").expect("write");
        let gltf = format!(
            r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":"{name}","byteLength":4}}]}}"#
        );
        let base = std::path::Path::new("model.gltf").parent().expect("parent");
        assert!(base.as_os_str().is_empty());
        let (_doc, buffers, _) = load_jailed_gltf(base, gltf.as_bytes(), 4).expect("empty base");
        assert_eq!(&buffers[0].0[..], b"bare");
    }

    #[test]
    fn buffer_and_image_share_one_byte_budget() {
        let png = one_pixel_png();
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        std::fs::write(dir.path().join("pixel.png"), &png).expect("png");
        let gltf = br#"{
            "asset": {"version": "2.0"},
            "buffers": [{"uri": "box.bin", "byteLength": 4}],
            "images": [{"uri": "pixel.png"}]
        }"#;
        let total = 4 + png.len() as u64;
        let (_doc, buffers, images) =
            load_jailed_gltf(dir.path(), gltf, total).expect("exact total");
        assert_eq!(&buffers[0].0[..], b"box!");
        assert_eq!(images.len(), 1);
        let err = load_jailed_gltf(dir.path(), gltf, total - 1).expect_err("one past");
        let msg = err.to_string();
        assert!(msg.contains("limit") || msg.contains("exceed"), "{msg}");
    }

    #[test]
    fn image_uris_count_toward_the_uri_cap() {
        let png = one_pixel_png();
        let dir = tempfile::tempdir().expect("tempdir");
        let mut images = Vec::new();
        for index in 0..MAX_EXTERNAL_URIS {
            let name = format!("i{index}.png");
            std::fs::write(dir.path().join(&name), &png).expect("png");
            images.push(serde_json::json!({ "uri": name }));
        }
        let within = serde_json::json!({
            "asset": { "version": "2.0" },
            "images": images,
        });
        let bytes = serde_json::to_vec(&within).expect("json");
        let cap = png.len() as u64 * MAX_EXTERNAL_URIS as u64;
        let (_doc, _, loaded) = load_jailed_gltf(dir.path(), &bytes, cap).expect("64 images");
        assert_eq!(loaded.len(), MAX_EXTERNAL_URIS);

        let name = "extra.png";
        std::fs::write(dir.path().join(name), &png).expect("extra");
        images.push(serde_json::json!({ "uri": name }));
        let over = serde_json::json!({
            "asset": { "version": "2.0" },
            "images": images,
        });
        let bytes = serde_json::to_vec(&over).expect("json");
        let err = load_jailed_gltf(dir.path(), &bytes, cap + png.len() as u64).expect_err("65");
        assert!(err.to_string().contains("too many"), "{err}");
    }

    struct ClearRootHook;
    impl Drop for ClearRootHook {
        fn drop(&mut self) {
            crate::bounded_open::reset_root_opens();
        }
    }

    #[cfg(unix)]
    #[test]
    fn one_directory_handle_serves_the_container_and_its_siblings() {
        let _guard = ClearRootHook;
        crate::bounded_open::reset_root_opens();
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        let png = one_pixel_png();
        let model_json = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"box.bin","byteLength":4}],"images":[{"uri":"pixel.png"}]}"#;
        std::fs::write(jail.join("model.gltf"), model_json).expect("model");
        std::fs::write(jail.join("box.bin"), b"box!").expect("bin");
        std::fs::write(jail.join("pixel.png"), &png).expect("png");
        let outside = parent.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        std::fs::write(outside.join("model.gltf"), b"other").expect("other model");
        std::fs::write(outside.join("box.bin"), b"xxxx").expect("other bin");
        std::fs::write(outside.join("pixel.png"), &png).expect("other png");
        let jail_hook = jail.clone();
        let outside_hook = outside.clone();
        let parked = parent.path().join("parked");
        crate::bounded_open::after_root_open(Box::new(move || {
            std::fs::rename(&jail_hook, &parked).expect("park");
            std::os::unix::fs::symlink(&outside_hook, &jail_hook).expect("swap");
        }));
        let (bytes, root) =
            crate::gltf_loader::read_gltf_container_with_root(&jail.join("model.gltf"))
                .expect("container");
        assert!(bytes.starts_with(b"{"));
        let (_doc, buffers, images) =
            load_jailed_gltf_with_root(&root, &bytes, crate::gltf_loader::MAX_GLB_CONTAINER_BYTES)
                .expect("siblings");
        assert_eq!(&buffers[0].0[..], b"box!");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].width, 1);
        assert_eq!(crate::bounded_open::root_open_count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn one_directory_open_reads_the_container_and_its_siblings() {
        let _guard = ClearRootHook;
        crate::bounded_open::reset_root_opens();
        let dir = tempfile::tempdir().expect("tempdir");
        let png = one_pixel_png();
        let model_json = br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"box.bin","byteLength":4}],"images":[{"uri":"pixel.png"}]}"#;
        std::fs::write(dir.path().join("model.gltf"), model_json).expect("model");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        std::fs::write(dir.path().join("pixel.png"), &png).expect("png");
        let (bytes, root) =
            crate::gltf_loader::read_gltf_container_with_root(&dir.path().join("model.gltf"))
                .expect("container");
        let (_doc, buffers, images) =
            load_jailed_gltf_with_root(&root, &bytes, crate::gltf_loader::MAX_GLB_CONTAINER_BYTES)
                .expect("siblings");
        assert_eq!(&buffers[0].0[..], b"box!");
        assert_eq!(images.len(), 1);
        assert_eq!(crate::bounded_open::root_open_count(), 1);
    }
}
