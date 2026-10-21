//! Optional presentation passes submit under lavapipe with a real depth view.
//! They stay off in `draw_into` unless a setter enables them, so the allocation
//! gate does not count their passes.

use astraweave_render::Renderer;

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gated color"),
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
    target.create_view(&wgpu::TextureViewDescriptor::default())
}

fn submit(renderer: &mut Renderer, view: &wgpu::TextureView) {
    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gated"),
    });
    renderer
        .draw_into(view, None, &mut encoder)
        .expect("draw_into");
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
}

#[test]
fn atmosphere_stays_off_until_measured() {
    pollster::block_on(async {
        let renderer = Renderer::new_headless(64, 64).await.expect("headless");
        assert!(!renderer.atmosphere_enabled());
        assert!(!astraweave_render::PresentationQuality::default().atmosphere());
        assert!(astraweave_render::PresentationQuality::Atmosphere.atmosphere());
    });
}

#[test]
fn gated_passes_submit_with_scene_depth() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("headless");
        renderer.set_atmosphere_enabled(true);
        renderer.set_cloud_raymarch_enabled(true);
        renderer.set_ssgi_enabled(true);
        renderer.set_taa_enabled(true);
        renderer.set_temporal_upscale_enabled(true);
        renderer.set_dof_enabled(true);
        let device = renderer.device().clone();
        let view = color_target(&device);
        submit(&mut renderer, &view);
        submit(&mut renderer, &view);
    });
}
