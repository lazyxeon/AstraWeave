//! `draw_into` must not touch the heap once a fixed scene is warm.
//!
//! Run with `RUST_LIB_BACKTRACE=0 cargo test -p astraweave-render --features alloc-counter --test frame_alloc`.
//! The counting allocator is installed only for this binary.

#![cfg(feature = "alloc-counter")]

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[path = "support/lib_backtrace.rs"]
mod lib_backtrace;

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
    lib_backtrace::require_library_backtraces_off();
    pollster::block_on(async {
        let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
            return;
        };
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
        let features = renderer.gpu_features();
        let interesting = [
            ("timestamp", wgpu::Features::TIMESTAMP_QUERY),
            (
                "timestamp_inside_encoders",
                wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
            ),
            (
                "timestamp_inside_passes",
                wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
            ),
            ("pipeline_cache", wgpu::Features::PIPELINE_CACHE),
        ];
        let feature_bits: Vec<&str> = interesting
            .iter()
            .filter(|(_, bit)| features.contains(*bit))
            .map(|(name, _)| *name)
            .collect();
        let header = format!(
            "draw_into alloc adapter={} backend={} features={:?} color=Rgba8UnormSrgb atmosphere={} samples={samples:?}\n",
            renderer.gpu_adapter_name(),
            renderer.gpu_backend(),
            feature_bits,
            renderer.atmosphere_enabled(),
        );
        eprintln!("{header}");
        let adapter = renderer.gpu_adapter_name();
        assert!(
            adapter.to_lowercase().contains("llvmpipe"),
            "draw_into floor is the lavapipe count, adapter={adapter}"
        );
        let min = *samples.iter().min().unwrap();
        let max = *samples.iter().max().unwrap();
        // Lavapipe on Mesa 25.2 and on Mesa 26.0.8-1ubuntu0.3 (amd64 and
        // amd64v3), validation layers installed or not, rayon 1/4/16:
        // alloc-counter only is [119, 116, 115, 116, 115, 115].
        // `--all-features` is [125, 122, 121, 122, 121, 121].
        // The GL backend is [196, 190, 188, 190, 188, 188].
        // PBR CI on 8fc15bd9 printed [173, 170, 169, 170, 169, 169]. That is
        // the `--all-features` shape plus 48. The workflow sets
        // `RUST_BACKTRACE=1`, and wgpu-core's debug snatch lock then calls
        // `Backtrace::capture` on every acquire. Those 48 allocations are
        // that capture (`LockTrace::enter`), not another pass and not the
        // adapter. The process must be started with `RUST_LIB_BACKTRACE=0`
        // so capture stays off and the ceiling stays 130. A per-site
        // breakdown is the manual heaptrack workflow, not an assert here.
        // Spread inside a run is 4.
        // The constructor leaves Bruneton off. A sky-only Bruneton frame
        // was [150, 147, 147, 146, 146, 147] and is not this test.
        // `draw_into` does not build the pyramid unless `sample_hiz` is set.
        // The six mip passes were the jump to ~217 (about 18 allocations
        // each). What remains is wgpu-core 25 `BasePass::new` for the passes
        // that still record. Engine scratch stays at zero (`update_instances`
        // above). The PBR step builds `--all-features`. One pass is about 18,
        // and `121 - 18 = 103` still passes `min >= 100`. `min >= 110` keeps
        // the steady lows (115 and 121) and fails that drop (`103` or `97`).
        // Ceiling 130 is 5 above the all-features high sample. `125 + 18 = 143`
        // still fails.
        assert!(
            max <= 130 && min >= 110 && max - min <= 4,
            "draw_into allocs left the wgpu pass floor: {samples:?} adapter={} backend={} features={feature_bits:?}",
            renderer.gpu_adapter_name(),
            renderer.gpu_backend(),
        );
    });
}
