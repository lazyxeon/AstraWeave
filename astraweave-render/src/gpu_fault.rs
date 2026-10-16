//! The first wgpu error from this process.
//!
//! Pipeline and bind-group creation run inside [`scope`]. The popped error is
//! logged once, with the `FIRST wgpu error` prefix, so `aravath.log` keeps the
//! cause instead of the per-frame "is invalid" cascade.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static FIRST: Mutex<Option<String>> = Mutex::new(None);
static DEVICE_LOST: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Option<String>> {
    FIRST.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Remember `message` if nothing has been recorded yet.
pub fn record(message: &str) {
    let mut slot = lock();
    if slot.is_none() {
        log::error!("FIRST wgpu error: {message}");
        *slot = Some(message.to_string());
    }
}

/// Take the recorded error, if one was stored.
pub fn take() -> Option<String> {
    lock().take()
}

pub fn device_lost() -> bool {
    DEVICE_LOST.load(Ordering::SeqCst)
}

pub fn mark_device_lost() {
    DEVICE_LOST.store(true, Ordering::SeqCst);
}

/// `Rgba16Float` write-only storage is in the WebGPU baseline. Read-write is not.
pub fn storage_write_supported(device: &wgpu::Device, format: wgpu::TextureFormat) -> bool {
    let features = format.guaranteed_format_features(device.features());
    features
        .flags
        .contains(wgpu::TextureFormatFeatureFlags::STORAGE_WRITE_ONLY)
        && features
            .allowed_usages
            .contains(wgpu::TextureUsages::STORAGE_BINDING)
}

struct ScopeWaker;

impl std::task::Wake for ScopeWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

/// Pop the error scope pushed by [`scope`], polling the device until it resolves.
pub fn finish_scope(device: &wgpu::Device) -> Option<String> {
    let waker = std::task::Waker::from(std::sync::Arc::new(ScopeWaker));
    let mut future = std::pin::pin!(device.pop_error_scope());
    for _ in 0..64 {
        let mut cx = std::task::Context::from_waker(&waker);
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(Some(err)) => {
                let text = err.to_string();
                if error_means_device_lost(&err, &text) {
                    mark_device_lost();
                    log::error!("GPU device loss detected while popping an error scope");
                }
                record(&text);
                return Some(text);
            }
            std::task::Poll::Ready(None) => return None,
            std::task::Poll::Pending => {
                if let Err(err) = device.poll(wgpu::PollType::Wait) {
                    let text =
                        format!("device.poll failed while waiting for an error scope: {err}");
                    mark_device_lost();
                    record(&text);
                    return Some(text);
                }
            }
        }
    }
    let text = "wgpu error scope did not resolve".to_string();
    record(&text);
    Some(text)
}

pub fn error_means_device_lost(error: &wgpu::Error, text: &str) -> bool {
    matches!(
        error,
        wgpu::Error::Internal { .. } | wgpu::Error::OutOfMemory { .. }
    ) || text_means_device_lost(text)
}

pub fn text_means_device_lost(text: &str) -> bool {
    text.contains("device is lost")
        || text.contains("Parent device is lost")
        || text.contains("Device Lost")
        || text.contains("device lost")
}

/// Whether this device can `textureLoad` a `texture_depth_2d`.
///
/// The GL backend rejects that load (`not supported in GLSL`). Vulkan,
/// including lavapipe, accepts it. A probe failure that is not that rejection
/// reports supported, so the real pipeline creation still surfaces it.
/// `true` when `test_name` should return. GL rejects `textureLoad` on depth.
pub fn skip_without_depth_texture_load(device: &wgpu::Device, test_name: &str) -> bool {
    if depth_texture_load_supported(device) {
        return false;
    }
    eprintln!(
        "skip {test_name}: this adapter does not support textureLoad on depth textures. GL rejects that load. The lavapipe job still runs this test."
    );
    true
}

pub fn depth_texture_load_supported(device: &wgpu::Device) -> bool {
    const SHADER: &str = r#"
@group(0) @binding(0) var t_depth: texture_depth_2d;

@compute @workgroup_size(1)
fn main() {
    let _depth = textureLoad(t_depth, vec2<i32>(0, 0), 0);
}
"#;
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("depth_texture_load_probe"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("depth_texture_load_probe"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("depth_texture_load_probe"),
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });
    let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("depth_texture_load_probe"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    match pop_error(device) {
        None => true,
        Some(err) if err.contains("textureLoad") && err.contains("depth") => {
            log::warn!("textureLoad on depth textures is not supported: {err}");
            false
        }
        Some(err) => {
            log::warn!("depth textureLoad probe failed for another reason: {err}");
            true
        }
    }
}

fn pop_error(device: &wgpu::Device) -> Option<String> {
    let waker = std::task::Waker::from(std::sync::Arc::new(ScopeWaker));
    let mut future = std::pin::pin!(device.pop_error_scope());
    for _ in 0..64 {
        let mut cx = std::task::Context::from_waker(&waker);
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(Some(err)) => return Some(err.to_string()),
            std::task::Poll::Ready(None) => return None,
            std::task::Poll::Pending => {
                if device.poll(wgpu::PollType::Wait).is_err() {
                    return None;
                }
            }
        }
    }
    None
}

/// Run `body` inside a validation error scope. The value is dropped when the
/// scope captured an error, so an invalid pipeline is not stored.
pub fn scope<T>(device: &wgpu::Device, label: &str, body: impl FnOnce() -> T) -> Result<T, String> {
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = body();
    match finish_scope(device) {
        Some(err) => Err(format!("{label}: {err}")),
        None => Ok(value),
    }
}
