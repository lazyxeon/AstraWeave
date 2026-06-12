//! First GPU-execution and physical-invariant tests for `FluidSystem`.
//!
//! Added in Fluids-Integration F.1 (Must-Fix #9): prior to this file the crate
//! had 2,579 tests and zero that constructed a GPU solver or asserted any
//! physical property across simulation ticks.
//!
//! These tests assert *envelopes*, not exact values: the GPU solver is
//! non-deterministic by construction (atomic neighbor-list ordering × float
//! non-associativity, per the engine's fluids determinism carve-out). The
//! point is catching explosion/collapse/regression classes, not validating
//! PBF physics to research grade (that is T4 work).

use astraweave_fluids::{FluidSystem, Particle};

/// Serializes the GPU tests: running five simultaneous wgpu devices on one
/// adapter distorts frame timing enough to shift the (timing-coupled)
/// adaptive iteration count and occasionally trip the settling envelope.
/// One device at a time gives stable envelopes (observed: mean speed² 20-27
/// serialized vs a >100 outlier under 5-way contention).
static GPU_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu_serial() -> std::sync::MutexGuard<'static, ()> {
    // A panicked test poisons the lock; the poison itself is irrelevant.
    GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const DT: f32 = 1.0 / 60.0;
const PARTICLE_COUNT: u32 = 4096;

/// World bounds hardcoded in `shaders/fluid.wgsl` (integrate fallback clamp).
const BOUNDS_XZ: f32 = 29.5;
const BOUNDS_Y: f32 = 59.5;
const EPS: f32 = 1e-3;

/// Create a wgpu device + queue, or skip LOUDLY.
///
/// Skips when `SKIP_GPU_TESTS` is set or no adapter exists. The skip is
/// printed to stderr so a green run on a headless machine is visibly
/// distinguishable from a run that actually exercised the GPU.
fn try_create_test_device(test_name: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
    if std::env::var("SKIP_GPU_TESTS").is_ok() {
        eprintln!("SKIPPED (SKIP_GPU_TESTS set): {test_name} did NOT exercise the GPU");
        return None;
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("SKIPPED (no GPU adapter): {test_name} did NOT exercise the GPU");
            return None;
        }
    };
    match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("fluids gpu-execution test device"),
        // FluidSystem's SDF pipeline uses ReadOnly/WriteOnly storage textures
        // on Rgba32Float, which needs this native feature (the demo requests
        // the same; see FluidSystem::new docs).
        required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    })) {
        Ok((d, q)) => Some((d, q)),
        Err(e) => {
            eprintln!("SKIPPED (device request failed: {e}): {test_name} did NOT exercise the GPU");
            None
        }
    }
}

/// Apply the demo's canonical simulation parameters
/// (`examples/fluids_demo/src/main.rs:290-298` + laboratory scenario).
///
/// The constructor defaults (`target_density: 12.0`, `viscosity: 10.0`) are
/// NOT a validated configuration: spawn density is ~1.0, so a target of 12
/// keeps the density constraint maximally violated forever, and `viscosity`
/// feeds the vorticity-confinement gain — together they produce permanent
/// violent jitter (measured: mean speed² ≈ 220 after 300 frames). The demo
/// parameter set is the only production-exercised configuration, so the
/// envelope tests pin THAT configuration.
fn apply_demo_params(system: &mut FluidSystem) {
    system.smoothing_radius = 0.5;
    system.target_density = 1.0;
    system.viscosity = 0.01;
    system.surface_tension = 0.1;
    system.gravity = -9.81;
    system.cell_size = 1.2;
    system.grid_width = 64;
    system.grid_height = 64;
    system.grid_depth = 64;
}

/// Run one simulation step under the documented contract: one encoder per
/// step, submitted before the next step call.
fn run_step(system: &mut FluidSystem, device: &wgpu::Device, queue: &wgpu::Queue, dt: f32) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test step"),
    });
    system.step(device, &mut encoder, queue, dt);
    queue.submit([encoder.finish()]);
}

/// Read the full particle buffer back to the CPU (blocking).
fn read_particles(
    system: &FluidSystem,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Vec<Particle> {
    let size = (system.particle_count as usize * std::mem::size_of::<Particle>()) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("particle readback staging"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("readback copy"),
    });
    encoder.copy_buffer_to_buffer(system.get_particle_buffer(), 0, &staging, 0, size);
    queue.submit([encoder.finish()]);

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("map_async channel send");
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv()
        .expect("map_async channel recv")
        .expect("particle readback mapping failed");

    let data = slice.get_mapped_range();
    let particles: Vec<Particle> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging.unmap();
    particles
}

fn mean_speed_sq(particles: &[Particle], active: impl Fn(usize) -> bool) -> f32 {
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for (i, p) in particles.iter().enumerate() {
        if active(i) {
            sum += p.velocity[0] * p.velocity[0]
                + p.velocity[1] * p.velocity[1]
                + p.velocity[2] * p.velocity[2];
            n += 1;
        }
    }
    sum / n.max(1) as f32
}

fn mean_y(particles: &[Particle], active: impl Fn(usize) -> bool) -> f32 {
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for (i, p) in particles.iter().enumerate() {
        if active(i) {
            sum += p.position[1];
            n += 1;
        }
    }
    sum / n.max(1) as f32
}

/// GPU smoke: construct + step N frames without device validation errors.
/// `FluidSystem::new` had never been called by any test before F.1.
#[test]
fn gpu_smoke_construct_and_step() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_smoke_construct_and_step") else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);
    for _ in 0..10 {
        run_step(&mut system, &device, &queue, DT);
    }
    let _ = device.poll(wgpu::MaintainBase::Wait);
    // Reaching here without a wgpu validation panic is the assertion; also
    // verify state is readable and finite.
    let particles = read_particles(&system, &device, &queue);
    assert_eq!(particles.len(), PARTICLE_COUNT as usize);
    assert!(particles
        .iter()
        .all(|p| p.position.iter().all(|v| v.is_finite())));
}

/// Containment invariant: after 120 fixed-dt frames, every active particle
/// remains inside the shader's world box and every float is finite.
#[test]
fn gpu_containment_invariant_120_frames() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_containment_invariant_120_frames")
    else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);
    for _ in 0..120 {
        run_step(&mut system, &device, &queue, DT);
    }
    let particles = read_particles(&system, &device, &queue);

    for (i, p) in particles.iter().enumerate() {
        for (c, v) in p.position.iter().enumerate() {
            assert!(
                v.is_finite(),
                "particle {i} position component {c} is not finite: {v}"
            );
        }
        for (c, v) in p.velocity.iter().enumerate() {
            assert!(
                v.is_finite(),
                "particle {i} velocity component {c} is not finite: {v}"
            );
        }
        let [x, y, z, _] = p.position;
        assert!(
            (-BOUNDS_XZ - EPS..=BOUNDS_XZ + EPS).contains(&x),
            "particle {i} escaped X bounds: {x}"
        );
        assert!(
            (-EPS..=BOUNDS_Y + EPS).contains(&y),
            "particle {i} escaped Y bounds: {y}"
        );
        assert!(
            (-BOUNDS_XZ - EPS..=BOUNDS_XZ + EPS).contains(&z),
            "particle {i} escaped Z bounds: {z}"
        );
    }
}

/// Settling envelope: particles dropped under gravity reach a quasi-steady
/// state — no explosion (bounded speeds), no levitation (mean height drops),
/// no NaN collapse. Thresholds are deliberately generous regression envelopes
/// chosen from an observed run on a GTX 1660 Ti (recorded in the F.1 report);
/// they catch blow-up/collapse classes, not solver-accuracy drift.
#[test]
fn gpu_settling_envelope_300_frames() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_settling_envelope_300_frames") else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);

    let initial = read_particles(&system, &device, &queue);
    let initial_mean_y = mean_y(&initial, |_| true);

    for _ in 0..300 {
        run_step(&mut system, &device, &queue, DT);
    }
    let settled = read_particles(&system, &device, &queue);
    let settled_mean_y = mean_y(&settled, |_| true);
    let settled_speed_sq = mean_speed_sq(&settled, |_| true);

    eprintln!(
        "settling metrics: mean_y {initial_mean_y:.3} -> {settled_mean_y:.3}, \
         mean speed^2 at frame 300 = {settled_speed_sq:.4}"
    );

    // Gravity acted: the block (spawned at y >= 2.0) must have dropped.
    assert!(
        settled_mean_y < initial_mean_y,
        "mean height did not decrease: {initial_mean_y} -> {settled_mean_y}"
    );
    // No explosion: settled mean squared speed stays within a generous bound.
    // Empirical basis (GTX 1660 Ti Max-Q, 2026-06-11, demo params, 4096
    // particles, 300 frames): observed quasi-steady mean speed² ≈ 27 (the
    // pile keeps sloshing — XSPH at 0.01 damps slowly). Explosion classes
    // measured during F.1 debugging produced 10⁵-10⁷. 100.0 gives ~4×
    // headroom over observed steady state while sitting 3+ orders of
    // magnitude below the failure class this guards against.
    assert!(
        settled_speed_sq < 100.0,
        "fluid did not settle: mean speed^2 = {settled_speed_sq}"
    );
    // Quasi-steady state is still a *fluid*, not a frozen or collapsed state.
    assert!(settled
        .iter()
        .all(|p| p.position.iter().all(|v| v.is_finite())));
}

/// Despawn honesty (WI-2): despawned particles get flag 0, are parked far
/// below the world, and STAY parked across further simulation steps — proving
/// every kernel actually skips them (if `predict` ran on them, gravity would
/// move the parked position).
#[test]
fn gpu_despawn_removes_particles_from_simulation() {
    let _gpu = gpu_serial();
    let Some((device, queue)) =
        try_create_test_device("gpu_despawn_removes_particles_from_simulation")
    else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);
    let before = system.active_count;

    // Despawn before the first step: the CPU position cache is exact at
    // construction time, so region membership is precise here.
    // The constructor spawns a cubic grid around x,z in [-5, ...), y >= 2.
    system.despawn_region(&queue, [-100.0, -100.0, -100.0], [0.0, 100.0, 100.0]);
    run_step(&mut system, &device, &queue, DT); // processes the pending region
    let after = system.active_count;
    assert!(
        after < before,
        "despawn did not reduce active_count: {before} -> {after}"
    );

    // Step many more frames: parked particles must not move.
    for _ in 0..30 {
        run_step(&mut system, &device, &queue, DT);
    }
    let particles = read_particles(&system, &device, &queue);
    let mut parked = 0usize;
    for p in particles.iter() {
        if p.position[1] < -1000.0 {
            parked += 1;
            assert!(
                (p.position[1] - (-10_000.0)).abs() < 1e-3,
                "parked particle moved: y = {}",
                p.position[1]
            );
            assert_eq!(p.velocity, [0.0; 4], "parked particle has velocity");
        }
    }
    assert_eq!(
        parked,
        (before - after) as usize,
        "parked-particle count does not match despawned count"
    );
    // Active particles remain inside the world (not parked, not exploded).
    let active = particles.iter().filter(|p| p.position[1] >= -1.0).count();
    assert_eq!(active, after as usize);
}

/// Regression guard for the F.0 Must-Fix #1 class (ping-pong divergence):
/// the buffer returned by `get_particle_buffer()` must advance EVERY frame.
/// Under the removed ping-pong defect, the visible buffer alternated between
/// two half-rate states, so a free-falling particle's height would not
/// decrease monotonically across three consecutive single-step readbacks.
#[test]
fn gpu_visible_state_advances_every_frame() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_visible_state_advances_every_frame")
    else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);

    // Track the topmost particle (longest free-fall before floor contact).
    let initial = read_particles(&system, &device, &queue);
    let (track_idx, _) = initial
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.position[1].total_cmp(&b.1.position[1]))
        .expect("non-empty particle set");

    let mut last_y = initial[track_idx].position[1];
    for frame in 0..5 {
        run_step(&mut system, &device, &queue, DT);
        let particles = read_particles(&system, &device, &queue);
        let y = particles[track_idx].position[1];
        assert!(
            y < last_y - 1e-6,
            "frame {frame}: tracked particle did not fall ({last_y} -> {y}); \
             visible state is not advancing every frame"
        );
        last_y = y;
    }
}

/// F.1.1 coverage-gap closure: `FluidRenderer` had never been constructed by
/// any test, so a VERTEX-only visibility flag on a fragment-read camera
/// uniform (and a depth-texture × filtering-sampler pair in the shade
/// shader) shipped as startup panics on every demo launch. This smoke test
/// constructs the renderer headless (offscreen target — no surface) and
/// renders one frame against a live FluidSystem, under explicit validation
/// error scopes.
#[test]
fn gpu_renderer_smoke() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_renderer_smoke") else {
        return;
    };
    const W: u32 = 256;
    const H: u32 = 256;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // A small live particle system, stepped once so the buffer holds real
    // simulation state.
    let mut system = FluidSystem::new(&device, 1024);
    apply_demo_params(&mut system);
    run_step(&mut system, &device, &queue, DT);

    // Renderer construction under a validation scope (this is where the
    // F.1.1 startup panics fired).
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = astraweave_fluids::FluidRenderer::new(&device, W, H, format);
    let construct_err = pollster::block_on(device.pop_error_scope());
    assert!(
        construct_err.is_none(),
        "FluidRenderer construction produced a validation error: {construct_err:?}"
    );

    // Offscreen inputs (no swapchain surface anywhere).
    let make_tex = |label: &str, fmt: wgpu::TextureFormat, usage: wgpu::TextureUsages| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage,
            view_formats: &[],
        })
    };
    let target = make_tex(
        "smoke target",
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let scene = make_tex("smoke scene", format, wgpu::TextureUsages::TEXTURE_BINDING);
    let scene_depth = make_tex(
        "smoke scene depth",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let skybox = make_tex("smoke skybox", format, wgpu::TextureUsages::TEXTURE_BINDING);

    // Minimal but well-formed camera looking at the spawn region.
    let eye = glam::Vec3::new(0.0, 8.0, 25.0);
    let view = glam::Mat4::look_at_rh(eye, glam::Vec3::new(0.0, 4.0, 0.0), glam::Vec3::Y);
    let proj = glam::Mat4::perspective_rh(1.0, W as f32 / H as f32, 0.1, 200.0);
    let view_proj = proj * view;
    let camera = astraweave_fluids::renderer::CameraUniform {
        view_proj: view_proj.to_cols_array_2d(),
        inv_view_proj: view_proj.inverse().to_cols_array_2d(),
        view_inv: view.inverse().to_cols_array_2d(),
        cam_pos: [eye.x, eye.y, eye.z, 1.0],
        light_dir: [0.3, 0.9, 0.2, 0.0],
        time: 0.0,
        padding: [0.0; 19],
    };

    // One full render (depth -> smooth -> shade -> secondary) under a scope.
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("smoke render"),
    });
    renderer.render(
        &mut encoder,
        &target.create_view(&Default::default()),
        &scene.create_view(&Default::default()),
        &scene_depth.create_view(&Default::default()),
        &skybox.create_view(&Default::default()),
        system.get_particle_buffer(),
        system.particle_count,
        system.secondary_particle_buffer(),
        system.secondary_particle_count(),
        camera,
        &queue,
        &device,
    );
    queue.submit([encoder.finish()]);
    let render_err = pollster::block_on(device.pop_error_scope());
    let _ = device.poll(wgpu::MaintainBase::Wait);
    assert!(
        render_err.is_none(),
        "FluidRenderer::render produced a validation error: {render_err:?}"
    );
}
