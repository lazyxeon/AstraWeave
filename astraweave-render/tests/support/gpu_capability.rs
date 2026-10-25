//! Headless wgpu probe shared by render unit tests and integration tests.
//!
//! `prefer_fallback` tries a software adapter and then a hardware adapter.
//! That is the order main used. When `AW_GPU_SKIP_IF_UNAVAILABLE` is set,
//! `prefer_fallback` stays on the software adapter and does not continue to
//! hardware, so a macOS runner does not treat Apple Paravirtual Metal as the
//! software probe. `prefer_fallback: false` tries any adapter, then software,
//! with or without that variable. A host with neither records a skip only
//! when the variable is set. Otherwise the probe panics with `Failed to find
//! adapter`.

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

#[allow(dead_code)]
pub struct HeadlessGpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
}

pub fn skip_if_unavailable() -> bool {
    std::env::var_os("AW_GPU_SKIP_IF_UNAVAILABLE").is_some()
}

/// Record `NotFound` and return, or panic.
///
/// The caller returns after this call. `NotFound` writes one skip line when
/// `AW_GPU_SKIP_IF_UNAVAILABLE` is set. Every other adapter error panics, and
/// `NotFound` panics when that variable is unset. This is not a device error.
pub fn skip_missing_adapter(err: &wgpu::RequestAdapterError, reason: &str) {
    if skip_if_unavailable() && matches!(err, wgpu::RequestAdapterError::NotFound { .. }) {
        note_skip(reason);
    } else {
        panic!("Failed to find adapter: {reason}");
    }
}

/// Record one skip reason. Repeated reasons from the same process are kept
/// once. The line is also written to stderr.
pub fn note_skip(reason: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match seen.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let line = skip_line(reason);
    if !guard.insert(line.clone()) {
        return;
    }
    drop(guard);
    write_skip_line(&skip_log_path(), &line).unwrap_or_else(|err| {
        panic!(
            "could not record the GPU skip at {}: {err}",
            skip_log_path().display()
        )
    });
}

fn skip_log_path() -> std::path::PathBuf {
    let target = existing_target_dir();
    let target = target.canonicalize().unwrap_or_else(|err| {
        panic!(
            "GPU skip log stays in the workspace target directory, missing at {}: {err}",
            target.display()
        )
    });
    let path = match std::env::var_os("AW_GPU_SKIP_LOG") {
        Some(raw) => std::path::PathBuf::from(raw),
        None => target.join("gpu-skip-reasons.txt"),
    };
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("GPU skip log {} has no parent", path.display()));
    let parent = parent.canonicalize().unwrap_or_else(|err| {
        panic!(
            "refusing to create a directory for GPU skip log {}: {err}",
            path.display()
        )
    });
    if parent != target {
        panic!(
            "GPU skip log {} is outside {}",
            path.display(),
            target.display()
        );
    }
    path
}

fn existing_target_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let manifest = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_else(|| std::ffi::OsString::from(".")),
    );
    for ancestor in manifest.ancestors() {
        let candidate = ancestor.join("target");
        if candidate.is_dir() {
            return candidate;
        }
    }
    manifest.join("target")
}

fn skip_line(reason: &str) -> String {
    let current = std::thread::current();
    let test = current.name().unwrap_or("unnamed-test");
    format!("{test}: {reason}")
}

fn write_skip_line(path: &std::path::Path, reason: &str) -> std::io::Result<()> {
    let mut file = open_append_nofollow(path)?;
    let line = format!("SKIP(gpu): {}\n", reason.replace(['\n', '\r', '%'], " "));
    file.write_all(line.as_bytes())
}

fn open_append_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("GPU skip log {} is a symlink", path.display()),
            ));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

fn fallback_probes(prefer_fallback: bool, software_only: bool) -> &'static [bool] {
    if prefer_fallback && software_only {
        &[true]
    } else if prefer_fallback {
        &[true, false]
    } else {
        &[false, true]
    }
}

pub async fn request_headless(
    label: &'static str,
    features: wgpu::Features,
    limits: wgpu::Limits,
    power: wgpu::PowerPreference,
    prefer_fallback: bool,
) -> Option<HeadlessGpu> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let mut attempts = Vec::new();
    for &fallback in fallback_probes(prefer_fallback, skip_if_unavailable()) {
        let which = if fallback {
            "software adapter (force_fallback_adapter)"
        } else {
            "any adapter"
        };
        let requested = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power,
                force_fallback_adapter: fallback,
                compatible_surface: None,
            })
            .await;
        let adapter = match requested {
            Ok(adapter) => adapter,
            Err(err) => {
                attempts.push(format!("{which}: {err}"));
                continue;
            }
        };
        match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(label),
                required_features: features,
                required_limits: limits.clone(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
        {
            Ok((device, queue)) => {
                return Some(HeadlessGpu {
                    device,
                    queue,
                    adapter,
                });
            }
            Err(err) => {
                let info = adapter.get_info();
                attempts.push(format!(
                    "{which} '{}' ({:?}) refused the device: {err}",
                    info.name, info.backend
                ));
            }
        }
    }
    let reason = format!(
        "no wgpu adapter for '{label}' (backends=all). {}",
        attempts.join("; ")
    );
    if skip_if_unavailable() {
        note_skip(&reason);
        None
    } else {
        panic!("Failed to find adapter: {reason}");
    }
}

#[cfg(test)]
mod skip_record_tests {
    use super::write_skip_line;

    #[test]
    fn skip_reason_is_written_verbatim() {
        let path =
            std::env::temp_dir().join(format!("aw-gpu-skip-{}-record.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let reason = "no wgpu adapter for 'unit' (backends=all). software adapter: none";
        write_skip_line(&path, reason).expect("skip log write");
        let text = std::fs::read_to_string(&path).expect("skip log");
        let _ = std::fs::remove_file(&path);
        assert_eq!(text, format!("SKIP(gpu): {reason}\n"));
    }

    #[test]
    fn prefer_fallback_keeps_hardware_unless_the_skip_gate_is_set() {
        assert_eq!(super::fallback_probes(true, false), &[true, false]);
        assert_eq!(super::fallback_probes(true, true), &[true]);
        assert_eq!(super::fallback_probes(false, false), &[false, true]);
        assert_eq!(super::fallback_probes(false, true), &[false, true]);
    }

    #[cfg(unix)]
    #[test]
    fn o_nofollow_is_the_libc_constant() {
        assert_ne!(libc::O_NOFOLLOW, 0);
    }

    #[cfg(unix)]
    #[test]
    fn skip_line_refuses_a_symlink() {
        let dir = std::env::temp_dir().join(format!("aw-gpu-skip-note-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let outside = dir.join("outside.txt");
        std::fs::write(&outside, b"outside").expect("outside file");
        let link = dir.join("log.txt");
        std::os::unix::fs::symlink(&outside, &link).expect("symlink");
        let err = super::write_skip_line(&link, "should not land").expect_err("symlink");
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_eq!(std::fs::read(&outside).expect("outside bytes"), b"outside");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_line_names_the_running_test() {
        let line = super::skip_line("no wgpu adapter for 'named'");
        let current = std::thread::current();
        let name = current.name().unwrap_or("unnamed-test");
        assert!(
            line.starts_with(name) && line.contains("no wgpu adapter for 'named'"),
            "{line}"
        );
    }
}
