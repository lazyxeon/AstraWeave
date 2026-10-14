//! Submit one frame from a freshly built `Renderer` under wgpu validation.
//!
//! `Renderer::new_headless` creates every pass the device owns at startup,
//! including `CloudShadowPass` (write-only `Rgba16Float` storage). This is the
//! startup set, not the volumetric fog / god-ray passes owned by the Aravath
//! demo's `volumetric_validation` test.

use anyhow::Context;
use astraweave_render::Renderer;

#[test]
fn startup_passes_submit_under_validation() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64)
            .await
            .context("headless renderer")
            .expect("adapter");
        let device = renderer.device().clone();
        let queue = renderer.queue().clone();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("validation color"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("validation encoder"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("record startup passes");
        queue.submit(std::iter::once(encoder.finish()));
        let error = device.pop_error_scope().await;
        assert!(
            error.is_none(),
            "startup passes raised a validation error: {error:?}"
        );
        assert!(!renderer.is_device_lost());
    });
}
