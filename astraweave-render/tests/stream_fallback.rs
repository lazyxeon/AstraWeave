//! A streamed albedo stays on the 1×1 fallback until the worker finishes.
//! The frame that still binds that fallback does not allocate past the
//! wgpu recording floor.
//!
//! `cargo test -p astraweave-render --features alloc-counter --test stream_fallback`

#![cfg(feature = "alloc-counter")]

#[path = "support/lib_backtrace.rs"]
mod lib_backtrace;

use astraweave_ecs::counting_alloc::{allocs, reset_allocs};
use astraweave_render::{Instance, Renderer};
use std::io::Write;

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0,
    0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 0, 3, 1,
    1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stream fallback color"),
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
        label: Some("stream fallback encoder"),
    });
    renderer
        .draw_into(view, None, &mut encoder)
        .expect("draw_into");
    queue.submit(std::iter::once(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
}

#[test]
fn fallback_frame_does_not_allocate_before_the_texture_is_resident() {
    lib_backtrace::disable_library_backtraces();
    pollster::block_on(async {
        let path = format!("/tmp/aw-stream-{}.png", std::process::id());
        let _ = std::fs::remove_file(&path);
        let made = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .expect("mkfifo");
        assert!(made.success(), "mkfifo");
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let writer_path = path.clone();
        let writer = std::thread::spawn(move || {
            if go_rx.recv().is_err() {
                return;
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&writer_path)
                .expect("open fifo");
            file.write_all(PNG).expect("write png");
        });

        let mut renderer = Renderer::new_headless(64, 64)
            .await
            .expect("headless renderer");
        // An assert before the explicit release still opens the fifo, so the
        // decode read is not stuck in `open` when the renderer drops.
        let mut release = ReleaseWriter { tx: Some(go_tx) };
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let indices = [0, 1, 2];
        let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
        let instance =
            Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
        renderer.add_model_with_bounds(
            "plate",
            mesh,
            &[instance],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        );
        renderer
            .request_streamed_albedo("plate", &path, glam::Vec3::new(2.0, 0.0, 0.0))
            .expect("queue stream");

        let device = renderer.device().clone();
        let view = color_target(&device);
        for _ in 0..8 {
            record(&mut renderer, &view);
        }
        assert!(
            renderer.streamed_albedo_is_fallback("plate"),
            "warmup should still be on the 1x1 fallback, failure={:?}",
            renderer.stream_failure(&path)
        );

        let mut samples = [0usize; 4];
        for sample in &mut samples {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("measured stream encoder"),
            });
            reset_allocs();
            renderer
                .draw_into(&view, None, &mut encoder)
                .expect("measured draw_into");
            *sample = allocs();
            drop(encoder);
            assert!(
                renderer.streamed_albedo_is_fallback("plate"),
                "measured frame left the fallback"
            );
        }
        eprintln!("stream fallback draw_into alloc samples: {samples:?}");
        let min = *samples.iter().min().unwrap();
        let max = *samples.iter().max().unwrap();
        // Same floor as `frame_alloc`: high sample 119, spread 4.
        // Ceiling 130 leaves 11 of room and still fails one extra pass.
        assert!(
            max <= 130 && min >= 100 && max - min <= 4,
            "fallback frame left the wgpu pass floor: {samples:?}"
        );

        release.release();
        let mut resident = false;
        for _ in 0..100 {
            record(&mut renderer, &view);
            if !renderer.streamed_albedo_is_fallback("plate") {
                resident = true;
                break;
            }
            if let Some(err) = renderer.stream_failure(&path) {
                panic!("stream failed: {err}");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        writer.join().expect("writer");
        let _ = std::fs::remove_file(&path);
        assert!(resident, "texture never became resident");
    });
}

/// Sends on drop if `release` was not called. Declared after the renderer so
/// this drops first and the writer is connected before `DecodeRuntime` joins.
struct ReleaseWriter {
    tx: Option<std::sync::mpsc::Sender<()>>,
}

impl ReleaseWriter {
    fn release(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ReleaseWriter {
    fn drop(&mut self) {
        self.release();
    }
}
