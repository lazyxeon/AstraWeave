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
use std::path::PathBuf;
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

fn skip_log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("AW_GPU_SKIP_LOG") {
        return PathBuf::from(path);
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest)
        .join("..")
        .join("target")
        .join("gpu-skip-reasons.txt")
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
    if !guard.insert(reason.to_string()) {
        return;
    }
    eprintln!("SKIP(gpu): {reason}");
    write_skip_line(&skip_log_path(), reason);
}

fn write_skip_line(path: &std::path::Path, reason: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            let _ = writeln!(file, "SKIP(gpu): {reason}");
        }
        Err(err) => {
            eprintln!(
                "SKIP(gpu): could not record the skip at {}: {err}",
                path.display()
            );
        }
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
        write_skip_line(&path, reason);
        let text = std::fs::read_to_string(&path).expect("skip log");
        let _ = std::fs::remove_file(&path);
        assert_eq!(text, format!("SKIP(gpu): {reason}\n"));
    }
}
