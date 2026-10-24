//! Headless wgpu probe shared by render unit tests and integration tests.
//!
//! Software adapter first when the caller asks for it, then any adapter.
//! A host with neither (macOS GitHub runners) must not panic and must not
//! look like a GPU pass: the skip reason is printed and appended to
//! `target/gpu-skip-reasons.txt`. That path is a skip only when
//! `AW_GPU_SKIP_IF_UNAVAILABLE` is set. Otherwise the probe still panics
//! with `Failed to find adapter`, which is what a machine that should have
//! lavapipe or a GPU is required to do.

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
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = format!("SKIP(gpu): {}\n", reason.replace(['\n', '\r', '%'], " "));
    file.write_all(line.as_bytes())
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
    let order = if prefer_fallback {
        [true, false]
    } else {
        [false, true]
    };
    let mut attempts = Vec::new();
    for fallback in order {
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
