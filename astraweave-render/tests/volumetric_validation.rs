//! One real-device frame with volumetrics and light shafts on.
//!
//! Lavapipe (Linux) and the Windows runner both execute this. Any wgpu
//! validation error fails the test, including a bad storage access, a depth
//! binding, or a copy into a mapped profiler buffer.

use astraweave_render::{GodRayConfig, Renderer};

#[test]
fn volumetrics_on_submits_two_frames_without_validation_errors() {
    std::env::set_var("WGPU_VALIDATION", "1");
    let mut renderer = pollster::block_on(Renderer::new_headless(64, 64))
        .expect("headless renderer with validation");
    if !astraweave_render::gpu_fault::depth_texture_load_supported(renderer.device()) {
        eprintln!(
            "skip volumetrics_on_submits_two_frames_without_validation_errors: this adapter does not support textureLoad on depth textures. GL rejects that load. The lavapipe job still runs this test."
        );
        return;
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
