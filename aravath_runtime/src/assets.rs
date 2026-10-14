//! Where the demo reads models and where it writes saves.
//!
//! The directory that contains the executable wins. A `cargo run` binary
//! lives under `target/`, so that case falls back to the repo assets.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

const REQUIRED: &[&str] = &[
    "models/AnimationLibrary_Godot_Standard.gltf",
    "models/AnimationLibrary_Godot_Standard.bin",
    "models/figurine.glb",
    "models/weapon-sword.glb",
    "models/weapon-shield.glb",
    "models/Textures/colormap.png",
    "models/nature/tree_oak.glb",
    "models/nature/tree_pineDefaultA.glb",
    "models/nature/tree_palmShort.glb",
    "models/nature/rock_largeA.glb",
    "models/nature/grass.glb",
    "models/nature/plant_bush.glb",
    "textures/grass_diff_1k.jpg",
    "textures/dirt_diff_1k.jpg",
    "textures/stone_diff_1k.jpg",
    "textures/sand_diff_1k.jpg",
    "textures/wood_diff_1k.jpg",
];

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn asset_dir() -> PathBuf {
    let beside = exe_dir().join("assets").join("aravath");
    if beside.is_dir() {
        return beside;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let in_target = exe.components().any(|part| part.as_os_str() == "target");
    if in_target {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/aravath");
    }
    beside
}

/// Writable files (saves, logs, crash logs) live in one directory.
///
/// On Windows that directory is `%LOCALAPPDATA%\Aravath`. Elsewhere, and when
/// `LOCALAPPDATA` is unset, it is `aravath_data` beside the executable.
/// Assets and `aravath.cfg` stay next to the executable and are only read.
pub fn user_data_dir() -> PathBuf {
    user_data_dir_from(std::env::var_os("LOCALAPPDATA").as_deref(), &exe_dir())
}

pub fn user_data_dir_from(local_app_data: Option<&std::ffi::OsStr>, exe: &Path) -> PathBuf {
    if cfg!(windows) {
        if let Some(local) = local_app_data {
            let base = PathBuf::from(local);
            if base.is_absolute() && !path_has_parent_dir(&base) {
                return base.join("Aravath");
            }
        }
    }
    exe.join("aravath_data")
}

pub fn save_dir() -> PathBuf {
    user_data_dir().join("saves")
}

pub fn log_dir() -> PathBuf {
    user_data_dir()
}

fn path_has_parent_dir(path: &Path) -> bool {
    path.components()
        .any(|part| matches!(part, Component::ParentDir))
}

pub fn verify() -> Result<PathBuf> {
    let root = asset_dir();
    let mut missing = Vec::new();
    for relative in REQUIRED {
        let path = root.join(relative);
        if !path.is_file() {
            missing.push(relative.to_string());
        }
    }
    if !missing.is_empty() {
        bail!(
            "Aravath assets are incomplete.\nLooked in {}\nExecutable {}\nMissing:\n{}",
            root.display(),
            std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "(unknown)".into()),
            missing.join("\n")
        );
    }
    Ok(root)
}

pub const MAX_TEXTURE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_TEXTURE_EDGE: u32 = 4096;
pub const MAX_MODEL_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_CFG_BYTES: u64 = 16 * 1024;
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn read_texture(file: &str) -> Result<(u32, u32, Vec<u8>)> {
    let root = asset_dir().join("textures");
    let path = confine_relative(&root, file, &["jpg", "jpeg", "png"])?;
    regular_file(&path, MAX_TEXTURE_BYTES)?;
    let mut reader = image::ImageReader::open(&path)
        .with_context(|| format!("open {}", path.display()))?
        .with_guessed_format()
        .context("guess texture format")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_TEXTURE_EDGE);
    limits.max_image_height = Some(MAX_TEXTURE_EDGE);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .with_context(|| format!("decode {}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || width > MAX_TEXTURE_EDGE || height > MAX_TEXTURE_EDGE {
        bail!("texture {file} is {width}x{height}; limit is {MAX_TEXTURE_EDGE}");
    }
    Ok((width, height, image.into_raw()))
}

/// Load a glTF/GLB that lives under the asset directory and whose external
/// URIs are relative names in that same tree.
pub fn import_model(
    path: &Path,
) -> Result<(
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
)> {
    let path = confine_existing(&asset_dir(), path)?;
    validate_model_file(&path)?;
    gltf::import(&path).with_context(|| format!("import {}", path.display()))
}

pub fn validate_model_file(path: &Path) -> Result<()> {
    regular_file(path, MAX_MODEL_BYTES)?;
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() as u64 > MAX_MODEL_BYTES {
        bail!("model {} exceeds {MAX_MODEL_BYTES} bytes", path.display());
    }
    let json = model_json(path, &bytes)?;
    let mut uris = Vec::new();
    collect_uris(&json, &mut uris);
    if uris.len() > 64 {
        bail!("model {} has too many external uris", path.display());
    }
    let parent = path.parent().context("model has no directory")?;
    for uri in uris {
        check_relative_uri(&uri)?;
        let sibling = parent.join(&uri);
        regular_file(&sibling, MAX_MODEL_BYTES).with_context(|| format!("external uri {uri}"))?;
        if !sibling.starts_with(parent) {
            bail!("uri {uri} escapes the model directory");
        }
    }
    Ok(())
}

fn model_json(path: &Path, bytes: &[u8]) -> Result<serde_json::Value> {
    if bytes.starts_with(b"glTF") {
        if bytes.len() < 20 {
            bail!("truncated glb {}", path.display());
        }
        let prefix: [u8; 4] = bytes[12..16]
            .try_into()
            .map_err(|_| anyhow::anyhow!("glb chunk length"))?;
        let chunk_len = u32::from_le_bytes(prefix) as usize;
        let chunk_type = &bytes[16..20];
        if chunk_type != b"JSON" {
            bail!("glb {} is missing its JSON chunk", path.display());
        }
        let end = 20usize.saturating_add(chunk_len);
        if end > bytes.len() {
            bail!("glb {} JSON chunk is truncated", path.display());
        }
        serde_json::from_slice(&bytes[20..end]).context("parse glb json")
    } else {
        serde_json::from_slice(bytes).context("parse gltf json")
    }
}

fn collect_uris(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(uri)) = map.get("uri") {
                out.push(uri.clone());
            }
            for child in map.values() {
                collect_uris(child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                collect_uris(child, out);
            }
        }
        _ => {}
    }
}

fn check_relative_uri(uri: &str) -> Result<()> {
    if uri.is_empty() || uri.len() > 240 {
        bail!("gltf uri length is not allowed");
    }
    let lower = uri.to_ascii_lowercase();
    if lower.contains('%') || lower.contains(':') || lower.contains('\\') || lower.contains('\0') {
        bail!("gltf uri {uri} is not a relative file");
    }
    let path = Path::new(uri);
    let mut parts = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.contains("..") {
                    bail!("gltf uri {uri} escapes the model directory");
                }
                parts += 1;
            }
            Component::CurDir => {}
            _ => bail!("gltf uri {uri} escapes the model directory"),
        }
    }
    if parts == 0 {
        bail!("gltf uri {uri} is empty");
    }
    Ok(())
}

/// Join `relative` onto `base` and, when the file exists, require the
/// canonical path to stay inside `base`.
pub fn confine_relative(base: &Path, relative: &str, extensions: &[&str]) -> Result<PathBuf> {
    if relative.is_empty() || relative.len() > 240 || relative.contains('\0') {
        bail!("asset name is empty or too long");
    }
    let rel = Path::new(relative);
    for component in rel.components() {
        match component {
            Component::Normal(part) => {
                if part.to_string_lossy().contains("..") {
                    bail!("asset name escapes the asset directory");
                }
            }
            _ => bail!("asset name must be a relative file"),
        }
    }
    if !extensions.is_empty() {
        let ext = rel.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        if !extensions
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(ext))
        {
            bail!("asset extension .{ext} is not allowed");
        }
    }
    let joined = base.join(rel);
    if joined.exists() {
        confine_existing(base, &joined)
    } else {
        Ok(joined)
    }
}

pub fn confine_existing(base: &Path, path: &Path) -> Result<PathBuf> {
    let base = base
        .canonicalize()
        .with_context(|| format!("canonicalize {}", base.display()))?;
    let path = path
        .canonicalize()
        .with_context(|| format!("canonicalize {}", path.display()))?;
    if !path.starts_with(&base) {
        bail!("{} escapes {}", path.display(), base.display());
    }
    Ok(path)
}

fn regular_file(path: &Path, max_bytes: u64) -> Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.file_type().is_symlink() {
        bail!("refusing to follow symlink {}", path.display());
    }
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
    Ok(meta)
}

/// `Ok(None)` means the file is absent or does not mention volumetrics.
/// `Ok(Some(bool))` is the last `volumetrics=on|off` line.
pub fn read_volumetrics_cfg(path: &Path) -> Result<Option<bool>> {
    if !path.exists() {
        return Ok(None);
    }
    regular_file(path, MAX_CFG_BYTES)?;
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let text = String::from_utf8(bytes).context("aravath.cfg is not UTF-8")?;
    parse_volumetrics_cfg(&text)
}

pub fn parse_volumetrics_cfg(text: &str) -> Result<Option<bool>> {
    if text.len() > MAX_CFG_BYTES as usize {
        bail!("aravath.cfg exceeds {MAX_CFG_BYTES} bytes");
    }
    let mut volumetrics = None;
    for (index, line) in text.lines().enumerate() {
        if index > 200 {
            bail!("aravath.cfg has too many lines");
        }
        if line.len() > 256 {
            bail!("aravath.cfg line {} is too long", index + 1);
        }
        if line.chars().any(|c| c.is_control() && c != '\t') {
            bail!("aravath.cfg line {} has a control character", index + 1);
        }
        let key = line.split('#').next().unwrap_or("").replace(' ', "");
        if key.is_empty() {
            continue;
        }
        match key.as_str() {
            "volumetrics=off" => volumetrics = Some(false),
            "volumetrics=on" => volumetrics = Some(true),
            _ => {}
        }
    }
    Ok(volumetrics)
}

pub fn bounded_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// Append to `name` inside `dir`, truncating when the file is already over `max_bytes`.
pub fn open_capped_log(dir: &Path, name: &str, max_bytes: u64) -> io::Result<File> {
    if name.is_empty()
        || name.len() > 64
        || name.contains(['/', '\\', ':', '\0'])
        || name.contains("..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log name must be a single file component",
        ));
    }
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "refusing to append to a symlinked log",
            ));
        }
        if meta.len() > max_bytes {
            return OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path);
        }
    }
    OpenOptions::new().create(true).append(true).open(path)
}

pub fn write_log_line(file: &mut File, text: &str) -> io::Result<()> {
    let text = bounded_text(text, 16 * 1024);
    file.write_all(text.as_bytes())?;
    if !text.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_dir_finds_the_character_library() {
        let path = asset_dir().join("models/AnimationLibrary_Godot_Standard.gltf");
        assert!(path.is_file(), "missing {path:?}");
        assert!(verify().is_ok(), "{}", verify().unwrap_err());
        assert!(
            validate_model_file(&path).is_ok(),
            "{}",
            validate_model_file(&path).unwrap_err()
        );
    }

    #[test]
    fn texture_names_cannot_leave_the_texture_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("textures")).unwrap();
        let err = confine_relative(root.path(), "../secret.jpg", &["jpg"]).unwrap_err();
        assert!(
            err.to_string().contains("relative") || err.to_string().contains("escapes"),
            "{err}"
        );
        let err = confine_relative(root.path(), "/etc/passwd", &["jpg"]).unwrap_err();
        assert!(
            err.to_string().contains("relative") || err.to_string().contains("escapes"),
            "{err}"
        );
        let err = confine_relative(root.path(), "notes.txt", &["jpg"]).unwrap_err();
        assert!(err.to_string().contains("extension"), "{err}");
    }

    #[test]
    fn gltf_uris_must_stay_beside_the_model() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hero.gltf");
        fs::write(&path, r#"{"buffers":[{"uri":"../secret.bin"}]}"#).unwrap();
        let err = validate_model_file(&path).unwrap_err();
        assert!(
            err.to_string().contains("escapes") || err.to_string().contains("uri"),
            "{err}"
        );

        fs::write(&path, r#"{"buffers":[{"uri":"hero.bin"}]}"#).unwrap();
        let err = validate_model_file(&path).unwrap_err();
        assert!(
            err.to_string().contains("hero.bin") || err.to_string().contains("stat"),
            "{err}"
        );

        fs::write(dir.path().join("hero.bin"), b"glb").unwrap();
        assert!(
            validate_model_file(&path).is_ok(),
            "{}",
            validate_model_file(&path).unwrap_err()
        );
    }

    #[test]
    fn cfg_is_size_limited_and_only_toggles_volumetrics() {
        assert_eq!(parse_volumetrics_cfg("").unwrap(), None);
        assert_eq!(
            parse_volumetrics_cfg("# note\nvolumetrics=off\n").unwrap(),
            Some(false)
        );
        assert_eq!(
            parse_volumetrics_cfg("volumetrics = on\nunknown=1\n").unwrap(),
            Some(true)
        );
        let huge = "a".repeat(MAX_CFG_BYTES as usize + 1);
        assert!(parse_volumetrics_cfg(&huge).is_err());

        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("aravath.cfg");
        fs::write(&cfg, "volumetrics=off\n").unwrap();
        assert_eq!(read_volumetrics_cfg(&cfg).unwrap(), Some(false));
        let missing = dir.path().join("missing.cfg");
        assert_eq!(read_volumetrics_cfg(&missing).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn cfg_symlink_is_refused() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.txt");
        fs::write(&real, "volumetrics=off\n").unwrap();
        let cfg = dir.path().join("aravath.cfg");
        symlink(&real, &cfg).unwrap();
        let err = read_volumetrics_cfg(&cfg).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
    }

    #[test]
    fn user_data_stays_under_localappdata_on_windows() {
        let exe = PathBuf::from("/games/Aravath");
        let dir = user_data_dir_from(
            Some(std::ffi::OsStr::new(r"C:\Users\ada\AppData\Local")),
            &exe,
        );
        if cfg!(windows) {
            assert!(dir.ends_with("Aravath"));
            assert!(dir.starts_with(r"C:\Users\ada\AppData\Local"));
        } else {
            assert!(dir.ends_with("aravath_data"));
        }
        let escaped =
            user_data_dir_from(Some(std::ffi::OsStr::new(r"C:\Users\ada\..\Windows")), &exe);
        if cfg!(windows) {
            assert!(escaped.ends_with("aravath_data"), "{escaped:?}");
        }
    }

    #[test]
    fn logs_truncate_instead_of_growing_without_bound() {
        let dir = tempfile::tempdir().unwrap();
        let mut file = open_capped_log(dir.path(), "aravath.log", 32).unwrap();
        write_log_line(&mut file, "hello").unwrap();
        drop(file);
        let mut file = open_capped_log(dir.path(), "aravath.log", 32).unwrap();
        write_log_line(&mut file, &"x".repeat(80)).unwrap();
        drop(file);
        let again = open_capped_log(dir.path(), "aravath.log", 32).unwrap();
        drop(again);
        let len = fs::metadata(dir.path().join("aravath.log")).unwrap().len();
        assert!(len <= 32, "log grew to {len}");
    }

    #[test]
    fn bounded_text_does_not_split_a_char() {
        let text = bounded_text("héllo", 2);
        assert!(text.ends_with('…'));
        assert!(text.starts_with('h'));
    }

    #[test]
    fn opening_a_log_creates_the_directory_and_flushes_the_line() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("Aravath");
        let mut file = open_capped_log(&nested, "aravath.log", 4096).unwrap();
        write_log_line(&mut file, "created").unwrap();
        file.sync_all().unwrap();
        let text = fs::read_to_string(nested.join("aravath.log")).unwrap();
        assert!(nested.is_dir());
        assert!(text.contains("created"));
    }

    #[test]
    fn discrete_gpu_export_is_requested_for_the_windows_exe() {
        let build = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs"));
        assert!(build.contains("/EXPORT:NvOptimusEnablement,DATA"));
        assert!(build.contains("/EXPORT:AmdPowerXpressRequestHighPerformance,DATA"));
        let prefs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/gpu_preference.rs"
        ));
        assert!(prefs.contains("static NvOptimusEnablement: u32 = 1"));
        assert!(prefs.contains("static AmdPowerXpressRequestHighPerformance: i32 = 1"));
    }

    #[test]
    fn windows_manifest_requests_as_invoker() {
        let manifest =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("windows/aravath.exe.manifest");
        let text = fs::read_to_string(&manifest).unwrap();
        assert!(text.contains("asInvoker"));
        assert!(text.contains("uiAccess=\"false\""));
        assert!(text.contains("Aravath"));
        let rc = fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("windows/aravath.rc"),
        )
        .unwrap();
        assert!(rc.contains("ProductName"));
        assert!(rc.contains("Aravath greybox demo"));
        assert!(rc.contains("AstraWeave"));
    }
}
