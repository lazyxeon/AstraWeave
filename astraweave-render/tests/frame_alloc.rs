//! `draw_into` must not touch the heap once a fixed scene is warm.
//!
//! Run with `cargo test -p astraweave-render --features alloc-counter --test frame_alloc`.
//! The counting allocator is installed only for this binary.

#![cfg(feature = "alloc-counter")]

#[path = "support/lib_backtrace.rs"]
mod lib_backtrace;

use astraweave_ecs::counting_alloc::{allocs, begin_alloc_trace, end_alloc_trace, reset_allocs};
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
    lib_backtrace::disable_library_backtraces();
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

        // Warm the trace table off the measured window. The first
        // `thread_local` access must not land inside a sample.
        begin_alloc_trace();
        let _ = end_alloc_trace();

        let mut samples = [0usize; 6];
        let mut trace = None;
        for (index, sample) in samples.iter_mut().enumerate() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("measured encoder"),
            });
            reset_allocs();
            if index == 0 {
                begin_alloc_trace();
            }
            renderer
                .draw_into(&view, None, &mut encoder)
                .expect("measured draw_into");
            if index == 0 {
                let recorded = end_alloc_trace();
                *sample = recorded.total;
                trace = Some(recorded);
            } else {
                *sample = allocs();
            }
            drop(encoder);
        }
        let trace = trace.expect("first frame was traced");
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
        eprint!("{}", trace.report);
        let trace_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/frame-alloc-trace.txt");
        if let Some(parent) = trace_path.parent() {
            std::fs::create_dir_all(parent).expect("create trace directory");
        }
        std::fs::write(&trace_path, format!("{header}{}", trace.report))
            .expect("write alloc site trace");
        eprintln!("alloc-sites file={}", trace_path.display());
        if trace.report.starts_with("alloc-sites total=") {
            assert_eq!(
                trace.total, samples[0],
                "first-frame snapshot diverged from the reported total\n{}",
                trace.report
            );
            assert!(
                trace.report.contains("draw_into") || trace.report.contains("Renderer"),
                "frame-pointer capture did not reach draw_into\n{}",
                trace.report
            );
        }
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
        // adapter. `disable_library_backtraces` runs before the device
        // exists so capture stays off and the ceiling stays 130. The first
        // sample's call sites are `target/frame-alloc-trace.txt`.
        // Spread inside a run is 4.
        // The constructor leaves Bruneton off. A sky-only Bruneton frame
        // was [150, 147, 147, 146, 146, 147] and is not this test.
        // `draw_into` does not build the pyramid unless `sample_hiz` is set.
        // The six mip passes were the jump to ~217 (about 18 allocations
        // each). What remains is wgpu-core 25 `BasePass::new` for the passes
        // that still record. Engine scratch stays at zero (`update_instances`
        // above). Ceiling 130 is 11 above the high sample and still under
        // one extra pass (119 + 18). Floor 100 is 15 under the low sample,
        // so a dropped pass fails too.
        assert!(
            max <= 130 && min >= 100 && max - min <= 4,
            "draw_into allocs left the wgpu pass floor: {samples:?} adapter={} backend={} features={feature_bits:?}",
            renderer.gpu_adapter_name(),
            renderer.gpu_backend(),
        );
    });
}
