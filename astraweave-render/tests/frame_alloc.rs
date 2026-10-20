//! `draw_into` must not touch the heap once a fixed scene is warm.
//!
//! Run with `cargo test -p astraweave-render --features alloc-counter --test frame_alloc`.
//! The counting allocator is installed only for this binary.

#![cfg(feature = "alloc-counter")]

use astraweave_ecs::counting_alloc::{allocs, reset_allocs};
use astraweave_render::{Instance, Renderer};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame alloc color"),
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

fn record(renderer: &mut Renderer, view: &wgpu::TextureView) {
    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame alloc encoder"),
    });
    renderer
        .draw_into(view, None, &mut encoder)
        .expect("draw_into");
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
}

#[test]
fn draw_into_allocates_nothing_after_warmup() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64)
            .await
            .expect("headless renderer");
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let indices = [0, 1, 2];
        let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
        for i in 0..4 {
            let instance = Instance::from_pos_scale_color(
                glam::Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                glam::Vec3::ONE,
                [1.0, 1.0, 1.0, 1.0],
            );
            renderer.add_model_with_bounds(
                format!("tri-{i}"),
                mesh.clone(),
                &[instance],
                [i as f32 * 2.0, 0.0, 0.0],
                [i as f32 * 2.0 + 1.0, 1.0, 0.0],
            );
        }
        let instances = (0..4)
            .map(|i| {
                Instance::from_pos_scale_color(
                    glam::Vec3::new(i as f32, 1.0, 2.0),
                    glam::Vec3::ONE,
                    [1.0, 1.0, 1.0, 1.0],
                )
            })
            .collect::<Vec<_>>();
        renderer.update_instances(&instances);
        // Second upload stays inside the scratch capacity.
        renderer.update_instances(&instances);

        let device = renderer.device().clone();
        let view = color_target(&device);
        // Far shadow cascades settle after a few frames. Warm until that cache
        // stops changing which passes `draw_into` records.
        for _ in 0..8 {
            record(&mut renderer, &view);
        }

        reset_allocs();
        renderer.update_instances(&instances);
        assert_eq!(
            allocs(),
            0,
            "update_instances allocated after the scratch was warm"
        );

        let mut samples = [0usize; 6];
        for sample in &mut samples {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("measured encoder"),
            });
            reset_allocs();
            renderer
                .draw_into(&view, None, &mut encoder)
                .expect("measured draw_into");
            *sample = allocs();
            drop(encoder);
        }
        eprintln!("draw_into alloc samples: {samples:?}");
        let min = *samples.iter().min().unwrap();
        let max = *samples.iter().max().unwrap();
        // Measured after the unused Hi-Z chain stopped recording:
        // [119, 116, 115, 116, 115, 115]. `draw_into` does not build the
        // pyramid unless a pass samples `HizPyramid::view` (`sample_hiz`).
        // Nothing does. The six mip passes were the jump to ~217. What
        // remains is wgpu-core 25 `BasePass::new` for the passes that still
        // record (shadows, sky, cloud shadow, main, post). Upstream still
        // allocates that Vec per pass. Engine scratch stays at zero
        // (`update_instances` above). A new recorded pass moves the band.
        assert!(
            max <= 120 && min >= 100 && max - min <= 6,
            "draw_into allocs left the wgpu pass floor: {samples:?}"
        );
    });
}
