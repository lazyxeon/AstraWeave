//! Particle updates are presentation-only and repeat for the same seed and dt.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use astraweave_render::gpu_particles::{EmitterParams, GpuParticleSystem};

fn params(seed: u32) -> EmitterParams {
    EmitterParams {
        position: [0.0, 1.0, 0.0, 1.0],
        velocity: [0.0, 2.0, 0.0, 0.0],
        emission_rate: 8.0,
        lifetime: 2.0,
        velocity_randomness: 0.25,
        delta_time: 1.0 / 60.0,
        gravity: [0.0, -9.8, 0.0, 0.0],
        particle_count: 8,
        max_particles: 64,
        random_seed: seed,
        _padding: 0,
    }
}

fn fingerprint(seed: u32) -> Option<Vec<u8>> {
    pollster::block_on(async {
        let gpu = gpu_capability::request_headless(
            "particle_present",
            wgpu::Features::empty(),
            wgpu::Limits::default(),
            wgpu::PowerPreference::HighPerformance,
            false,
        )
        .await?;
        let device = gpu.device;
        let queue = gpu.queue;
        let mut system = GpuParticleSystem::new(&device, 64).expect("particles");
        let step = params(seed);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle a"),
        });
        system.update(&queue, &mut encoder, &step);
        system.update(&queue, &mut encoder, &step);
        queue.submit(std::iter::once(encoder.finish()));

        let size = system.particle_buffer().size();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle copy"),
        });
        encoder.copy_buffer_to_buffer(system.particle_buffer(), 0, &staging, 0, size);
        queue.submit(std::iter::once(encoder.finish()));
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).expect("map");
        });
        let _ = device.poll(wgpu::MaintainBase::Wait);
        rx.recv().expect("recv").expect("mapped");
        let bytes = slice.get_mapped_range().to_vec();
        Some(bytes)
    })
}

#[test]
fn same_seed_and_dt_repeat_the_presentation_fingerprint() {
    // Each call builds a new device and runs two updates. The buffers start
    // at zero (`mapped_at_creation`), the emit/update shaders have no
    // atomics, and seed and dt are the only varying uniforms. A second run
    // of the same seed must match bit for bit. Seed 9 must match itself and
    // must differ from seed 7.
    let samples = [
        fingerprint(7),
        fingerprint(7),
        fingerprint(9),
        fingerprint(9),
    ];
    if samples.iter().all(Option::is_none) {
        return;
    }
    let seed_7_a = samples[0]
        .clone()
        .expect("seed 7 missed after another fingerprint created a device");
    let seed_7_b = samples[1]
        .clone()
        .expect("seed 7 second run missed after a device was created");
    assert_eq!(seed_7_a, seed_7_b, "seed 7 did not repeat");
    let seed_9_a = samples[2]
        .clone()
        .expect("seed 9 missed after a device was created");
    let seed_9_b = samples[3]
        .clone()
        .expect("seed 9 second run missed after a device was created");
    assert_eq!(seed_9_a, seed_9_b, "seed 9 did not repeat");
    assert_ne!(
        seed_7_a, seed_9_a,
        "different seeds produced the same buffer"
    );
}

#[test]
fn emitter_draw_into_submits() {
    pollster::block_on(async {
        let Some(mut renderer) = astraweave_render::Renderer::headless_for_tests(64, 64).await
        else {
            return;
        };
        renderer
            .set_particle_emitter(Some(params(7)))
            .expect("emitter");
        let device = renderer.device().clone();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("particle color"),
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
        let queue = renderer.queue().clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle frame"),
        });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw_into");
        queue.submit(std::iter::once(encoder.finish()));
        let _ = device.poll(wgpu::MaintainBase::Wait);
        let err = astraweave_render::gpu_fault::finish_scope(&device);
        assert!(err.is_none(), "particle frame_scope validation={err:?}");
    });
}
