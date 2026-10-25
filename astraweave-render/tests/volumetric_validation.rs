//! One real-device frame with volumetrics and light shafts on.
//!
//! Lavapipe (Linux) and the Windows runner both execute this. Any wgpu
//! validation error fails the test, including a bad storage access, a depth
//! binding, or a copy into a mapped profiler buffer.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::{GodRayConfig, Renderer};

#[test]
fn volumetrics_on_submits_two_frames_without_validation_errors() {
    std::env::set_var("WGPU_VALIDATION", "1");
    let Some(mut renderer) = pollster::block_on(Renderer::headless_for_tests(64, 64)) else {
        return;
    };
    if !astraweave_render::gpu_fault::depth_texture_load_supported(renderer.device()) {
        let reason = format!(
            "volumetrics_on_submits_two_frames_without_validation_errors: depth textureLoad is absent on adapter={} backend={}",
            renderer.gpu_adapter_name(),
            renderer.gpu_backend()
        );
        if std::env::var_os("AW_GPU_SKIP_IF_UNAVAILABLE").is_some() {
            astraweave_render::renderer::record_gpu_skip_for_tests(&reason);
            return;
        }
        panic!("{reason}");
    }
    renderer.set_volumetric_fog_config(
        astraweave_render::volumetric_fog::VolumetricFogPass::gameplay_config(),
    );
    renderer.set_god_ray_config(GodRayConfig {
        enabled: true,
        half_res: true,
        num_samples: 32,
        exposure: 0.22,
        ..GodRayConfig::default()
    });
    assert!(
        renderer.volumetric_fog_enabled(),
        "volumetric fog did not stay enabled"
    );
    assert!(
        renderer.god_rays_enabled(),
        "light shafts did not stay enabled"
    );
    renderer
        .submit_headless_frame()
        .expect("first volumetrics frame");
    renderer
        .submit_headless_frame()
        .expect("second volumetrics frame");
    assert!(renderer.volumetric_fog_enabled());
    assert!(renderer.god_rays_enabled());
    assert!(!renderer.is_device_lost());
}
