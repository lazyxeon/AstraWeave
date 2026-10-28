//! Enabling `sample_hiz` rebuilds the pyramid against that frame's depth view.
//!
//! The copy bind group is fixed at `HizPyramid` creation. A later `draw_into`
//! with a different view of the same size must not keep the view captured
//! earlier.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::Renderer;

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hiz rebind color"),
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

fn depth_target(device: &wgpu::Device, label: &str) -> (wgpu::Texture, wgpu::TextureView) {
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    (depth, view)
}

fn draw(renderer: &mut Renderer, color: &wgpu::TextureView, depth: Option<&wgpu::TextureView>) {
    let device = renderer.device().clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hiz rebind"),
    });
    renderer
        .draw_into(color, depth, &mut encoder)
        .expect("draw_into");
    drop(encoder);
}

#[test]
fn enabling_sample_hiz_rebinds_to_the_current_depth_view() {
    pollster::block_on(async {
        let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
            return;
        };
        if astraweave_render::gpu_fault::skip_or_panic_without_depth_texture_load(
            renderer.device(),
            "enabling_sample_hiz_rebinds_to_the_current_depth_view",
        ) {
            return;
        }
        let device = renderer.device().clone();
        let color = color_target(&device);
        let (_depth_a, view_a) = depth_target(&device, "depth a");
        let (_depth_b, view_b) = depth_target(&device, "depth b");

        draw(&mut renderer, &color, Some(&view_a));
        assert!(
            renderer.hiz_pyramid().is_none(),
            "sample_hiz stays off until set, so no pyramid is captured at creation"
        );

        renderer.set_sample_hiz(true);
        draw(&mut renderer, &color, Some(&view_a));
        let pyramid = renderer
            .hiz_pyramid()
            .expect("enabling sample_hiz builds a pyramid");
        assert!(
            pyramid.copies_from(&view_a),
            "the first enabled frame copies the depth view from that draw_into"
        );
        assert!(!pyramid.copies_from(&view_b));

        draw(&mut renderer, &color, Some(&view_b));
        let pyramid = renderer
            .hiz_pyramid()
            .expect("a same-size view still rebuilds");
        assert!(
            pyramid.copies_from(&view_b),
            "a different depth view of the same size replaces the copy source"
        );
        assert!(!pyramid.copies_from(&view_a));

        renderer.set_sample_hiz(false);
        draw(&mut renderer, &color, Some(&view_a));
        assert!(
            renderer.hiz_pyramid().is_none(),
            "turning sample_hiz off drops the pyramid"
        );

        renderer.set_sample_hiz(true);
        draw(&mut renderer, &color, Some(&view_a));
        let pyramid = renderer
            .hiz_pyramid()
            .expect("enabling again builds against the current view");
        assert!(pyramid.copies_from(&view_a));
        assert!(!pyramid.copies_from(&view_b));
    });
}
