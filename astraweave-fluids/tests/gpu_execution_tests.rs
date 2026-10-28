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

use astraweave_fluids::{FluidSystem, Particle, SecondaryParticle};

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

/// Create a wgpu device + queue, or record a skip, or panic.
///
/// `SKIP_GPU_TESTS` and a `NotFound` adapter while `AW_GPU_SKIP_IF_UNAVAILABLE`
/// is set each append one `SKIP(gpu)` line and return `None`. Any other
/// adapter error panics. A device refusal panics instead of being reported as
/// a missing adapter.
fn try_create_test_device(test_name: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
    if std::env::var_os("SKIP_GPU_TESTS").is_some() {
        record_gpu_skip(&format!(
            "SKIP_GPU_TESTS set: {test_name} did not exercise the GPU"
        ));
        return None;
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(adapter) => adapter,
        Err(err) => {
            let reason = format!("{test_name}: {err}");
            if std::env::var_os("AW_GPU_SKIP_IF_UNAVAILABLE").is_some()
                && matches!(err, wgpu::RequestAdapterError::NotFound { .. })
            {
                record_gpu_skip(&reason);
                return None;
            }
            panic!("Failed to find adapter: {reason}");
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
        Ok((device, queue)) => Some((device, queue)),
        Err(err) => panic!("{test_name}: adapter refused the device: {err}"),
    }
}

fn record_gpu_skip(reason: &str) {
    let current = std::thread::current();
    let test = current.name().unwrap_or("unnamed-test");
    let line = format!(
        "SKIP(gpu): {test}: {}\n",
        reason.replace(['\n', '\r', '%'], " ")
    );
    eprintln!("{}", line.trim_end());
    let target = existing_target_dir()
        .canonicalize()
        .unwrap_or_else(|err| panic!("GPU skip log target is missing: {err}"));
    let path = std::env::var_os("AW_GPU_SKIP_LOG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| target.join("gpu-skip-reasons.txt"));
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("GPU skip log {} has no parent", path.display()))
        .canonicalize()
        .unwrap_or_else(|err| {
            panic!(
                "refusing to create a directory for {}: {err}",
                path.display()
            )
        });
    assert_eq!(
        parent,
        target,
        "GPU skip log {} is outside {}",
        path.display(),
        target.display()
    );
    let mut file = open_append_nofollow(&path)
        .unwrap_or_else(|err| panic!("could not record GPU skip at {}: {err}", path.display()));
    std::io::Write::write_all(&mut file, line.as_bytes())
        .unwrap_or_else(|err| panic!("could not record GPU skip at {}: {err}", path.display()));
}

fn existing_target_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let manifest = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_else(|| std::ffi::OsString::from(".")),
    );
    for ancestor in manifest.ancestors() {
        let candidate = ancestor.join("target");
        if candidate.is_dir() {
            return candidate;
        }
    }
    manifest.join("target")
}

fn open_append_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("GPU skip log {} is a symlink", path.display()),
            ));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
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
    // Retire this submission before the next step. A few hundred unpolled
    // command buffers stop the DX12 software adapter from signaling its
    // fence, and the later `poll(Wait)` in `read_particles` never returns.
    // 120 frames flaked between 58s and the 240s kill; 300 frames never
    // returned. The timeout itself stays 240s.
    // `PollType::Wait` blocks for wgpu-core `CLEANUP_WAIT_MS` (60s) and
    // returns `PollError::Timeout` with the fence still open. That is not
    // the 6.3s `Queue::drop` budget. Dropping the error made `read_particles`
    // block on `recv` for a callback that never ran.
    expect_poll_finished(&device);
}

/// `poll(Wait)` either finished the submission or the step fails here.
fn expect_poll_finished(device: &wgpu::Device) {
    match device.poll(wgpu::PollType::Wait) {
        Ok(status) => assert!(
            status.wait_finished(),
            "poll(Wait) returned {status:?} before the submission finished"
        ),
        Err(wgpu::PollError::Timeout) => panic!(
            "poll(Wait) timed out after 60s (wgpu-core CLEANUP_WAIT_MS). This is not the 6.3s Queue::drop budget; the submission is still running."
        ),
    }
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
    expect_poll_finished(&device);
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
    expect_poll_finished(&device);
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

    // F.4.2: upload a few secondary particles so the `render()` secondary block
    // still draws (the count default is now 0, not the old hardcoded 65536).
    system.set_secondary_particles(
        &queue,
        &[
            SecondaryParticle {
                position: [0.0, 4.0, 0.0, 1.0],
                velocity: [0.0; 4],
                info: [0.0, 1.0, 1.0, 0.5],
            },
            SecondaryParticle {
                position: [1.0, 4.0, 0.0, 1.0],
                velocity: [0.0; 4],
                info: [0.0, 0.0, 1.0, 0.5],
            },
        ],
    );
    assert_eq!(system.secondary_particle_count(), 2);

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
    expect_poll_finished(&device);
    assert!(
        render_err.is_none(),
        "FluidRenderer::render produced a validation error: {render_err:?}"
    );
}

/// F.4.2: the standalone additive-billboard accent render (`render_accents`).
/// Proves the SSFR-chain split: accents draw WITHOUT the depth/smooth/shade
/// surface passes (no FluidSystem step, no scene/skybox inputs), compositing
/// additively over an HDR target with an external scene-depth view. Also
/// exercises the per-kind tint/shape path (one particle of each kind) and the
/// counter-readback fix (live count, not the old hardcoded 65536).
#[test]
fn gpu_render_accents_smoke() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_render_accents_smoke") else {
        return;
    };
    const W: u32 = 256;
    const H: u32 = 256;
    // HDR format, matching the W-series surface target the accents composite over.
    let format = wgpu::TextureFormat::Rgba16Float;

    // No simulation needed — accents are CPU-produced. A tiny system owns the
    // secondary buffer + the count fix.
    let mut system = FluidSystem::new(&device, 64);
    // Counter-readback fix: zero until a producer uploads (was hardcoded 65536).
    assert_eq!(system.secondary_particle_count(), 0);

    let renderer = astraweave_fluids::FluidRenderer::new(&device, W, H, format);

    // One accent of each kind (Part=0, Raise=1, Freeze=2) → exercises the LUT.
    let accents = [
        SecondaryParticle {
            position: [-2.0, 4.0, 0.0, 1.0],
            velocity: [0.0, 1.0, 0.0, 0.0],
            info: [0.1, 0.0, 0.9, 0.5],
        },
        SecondaryParticle {
            position: [0.0, 4.0, 0.0, 1.0],
            velocity: [0.0, 2.0, 0.0, 0.0],
            info: [0.1, 1.0, 0.9, 0.5],
        },
        SecondaryParticle {
            position: [2.0, 4.0, 0.0, 1.0],
            velocity: [0.0, 0.5, 0.0, 0.0],
            info: [0.1, 2.0, 0.9, 0.5],
        },
    ];
    system.set_secondary_particles(&queue, &accents);
    assert_eq!(
        system.secondary_particle_count(),
        3,
        "live count reflects upload"
    );

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
        "accent target",
        format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    // External scene depth (Depth32Float) — the surface pass's depth in production.
    let scene_depth = make_tex(
        "accent scene depth",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );

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

    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("accent render"),
    });
    // The surface pass writes this depth before accents. A clear-only pass
    // is that write: the DX12 software adapter does not finish a depth
    // attachment that has never been stored.
    {
        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("accent depth clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.create_view(&Default::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &scene_depth.create_view(&Default::default()),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    // ONLY the billboard pass — no depth/smooth/shade SSFR surface render.
    renderer.render_accents(
        &queue,
        &mut encoder,
        &target.create_view(&Default::default()),
        &scene_depth.create_view(&Default::default()),
        system.secondary_particle_buffer(),
        system.secondary_particle_count(),
        camera,
    );
    queue.submit([encoder.finish()]);
    let err = pollster::block_on(device.pop_error_scope());
    expect_poll_finished(&device);
    assert!(
        err.is_none(),
        "render_accents produced a validation error: {err:?}"
    );

    // Zero-accent identity: with no particles, render_accents records no pass.
    system.set_secondary_particles(&queue, &[]);
    assert_eq!(system.secondary_particle_count(), 0);
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("accent render empty"),
    });
    renderer.render_accents(
        &queue,
        &mut encoder2,
        &target.create_view(&Default::default()),
        &scene_depth.create_view(&Default::default()),
        system.secondary_particle_buffer(),
        system.secondary_particle_count(),
        camera,
    );
    queue.submit([encoder2.finish()]);
    let err2 = pollster::block_on(device.pop_error_scope());
    expect_poll_finished(&device);
    assert!(err2.is_none(), "render_accents (empty) error: {err2:?}");
}

/// F.1.3 (H-1): respawn reactivation — the path that despawn-freezing's
/// test-proven machinery never covered. Spawn-side reuse of freed slots must
/// flip the GPU flag back to 1, land the position write, and produce
/// particles that actually SIMULATE (fall under gravity), not just CPU
/// bookkeeping that logs success. (F.1.2 verified click-spawn with a CPU log
/// line — bookkeeping, not evidence; this test is the corrected standard.)
#[test]
fn gpu_respawn_reactivates_particles() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_respawn_reactivates_particles") else {
        return;
    };
    let mut system = FluidSystem::new(&device, PARTICLE_COUNT);
    apply_demo_params(&mut system);

    // Settle the constructor lattice a few frames first.
    for _ in 0..5 {
        run_step(&mut system, &device, &queue, DT);
    }

    // Despawn a region (the constructor block spans x,z in [-5, ...), y >= 2;
    // take the x < 0 half). Processed by the next step.
    let before = system.active_count;
    system.despawn_region(&queue, [-100.0, -100.0, -100.0], [0.0, 100.0, 100.0]);
    run_step(&mut system, &device, &queue, DT);
    let after_despawn = system.active_count;
    let despawned = before - after_despawn;
    assert!(
        despawned > 100,
        "test setup: expected a substantial despawn, got {despawned}"
    );

    // Respawn M particles into freed slots, high above the floor, marked by
    // a unique color so readback can identify them unambiguously.
    const M: usize = 64;
    const SPAWN_Y: f32 = 40.0;
    let positions: Vec<[f32; 3]> = (0..M)
        .map(|i| {
            [
                10.0 + (i % 8) as f32 * 0.4,
                SPAWN_Y,
                10.0 + (i / 8) as f32 * 0.4,
            ]
        })
        .collect();
    let velocities = vec![[0.0f32, 0.0, 0.0]; M];
    let colors = vec![[9.0f32, 0.0, 9.0, 1.0]; M]; // sentinel color
    let spawned = system.spawn_particles(&queue, &positions, &velocities, Some(&colors));
    assert_eq!(spawned, M, "spawn_particles should fill {M} freed slots");

    // Step 30 frames: if reactivation works, the respawned particles free-fall.
    for _ in 0..30 {
        run_step(&mut system, &device, &queue, DT);
    }
    let particles = read_particles(&system, &device, &queue);

    let respawned: Vec<&Particle> = particles
        .iter()
        .filter(|p| p.color == [9.0, 0.0, 9.0, 1.0])
        .collect();
    assert_eq!(
        respawned.len(),
        M,
        "expected {M} sentinel-colored particles in the buffer, found {}",
        respawned.len()
    );
    for (i, p) in respawned.iter().enumerate() {
        let y = p.position[1];
        assert!(
            y.is_finite() && y > -1000.0,
            "respawned particle {i} still parked (y = {y}): flag never reactivated?"
        );
        // 30 frames of free fall from y=40: ~0.5*9.81*(0.5s)^2 ~ 1.2 units;
        // assert a generous envelope (moved down measurably, still in-world).
        assert!(
            y < SPAWN_Y - 0.5,
            "respawned particle {i} did not fall (y = {y} vs spawn {SPAWN_Y}): not simulating"
        );
        assert!(
            y >= -EPS,
            "respawned particle {i} fell out of the world (y = {y})"
        );
    }
}

fn kernel_w(r: f32, h: f32) -> f32 {
    let q = r / h;
    if q >= 1.0 {
        return 0.0;
    }
    let alpha = 3.0 / (2.0 * std::f32::consts::PI * h * h * h);
    if q < 0.5 {
        alpha * (2.0 * (1.0 - q).powi(3) - (1.0 - 2.0 * q).powi(3))
    } else {
        alpha * (1.0 - q).powi(3)
    }
}

fn plant_particle(at: [f32; 3]) -> Particle {
    Particle {
        position: [at[0], at[1], at[2], 1.0],
        velocity: [0.0; 4],
        predicted_position: [at[0], at[1], at[2], 1.0],
        lambda: 0.0,
        density: 0.0,
        phase: 0,
        temperature: 293.0,
        color: [0.0, 0.0, 1.0, 1.0],
    }
}

fn dispatch_planted_lambda(
    system: &mut FluidSystem,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    head: &[i32],
    next: &[i32],
) -> Vec<Particle> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("planted lambda"),
    });
    system.lambda_on_planted_neighbors(device, queue, &mut encoder, head, next);
    queue.submit([encoder.finish()]);
    expect_poll_finished(&device);
    read_particles(system, device, queue)
}

/// A self-loop is one visit. A tail into another cycle is discarded, not
/// applied `particle_count` times. An index past `particle_count` is not loaded.
#[test]
fn gpu_cyclic_neighbor_is_not_applied() {
    let _gpu = gpu_serial();
    let Some((device, queue)) = try_create_test_device("gpu_cyclic_neighbor_is_not_applied") else {
        return;
    };
    const N: u32 = 8;
    let mut system = FluidSystem::new(&device, N);
    let h = system.smoothing_radius;
    let w0 = kernel_w(0.0, h);
    let cells = (system.grid_width * system.grid_height * system.grid_depth) as usize;
    let gx = i32::try_from(system.grid_width).expect("grid width");
    let gy = i32::try_from(system.grid_height).expect("grid height");
    // (0, 1, 0) with the constructor cell size 1.2 and the shader's +30 offset.
    let cell = 25 + 25 * gx * gy;
    let cell = usize::try_from(cell).expect("cell");
    assert!(cell < cells);

    let here = [0.0, 1.0, 0.0];
    let far = [1.0e6, 1.0e6, 1.0e6];
    let mut particles = vec![plant_particle(far); N as usize];
    particles[0] = plant_particle(here);
    queue.write_buffer(
        system.get_particle_buffer(),
        0,
        bytemuck::cast_slice(&particles),
    );

    let mut head = vec![-1i32; cells];
    head[cell] = 0;
    let mut next = vec![-1i32; N as usize];
    next[0] = 0; // self-loop
    let got = dispatch_planted_lambda(&mut system, &device, &queue, &head, &next);
    let density = got[0].density;
    assert!(
        (density - w0).abs() < w0 * 0.02,
        "self-loop density {density} is not one kernel visit {w0}"
    );
    assert!(
        density < w0 * 2.0,
        "self-loop was applied more than once ({density} vs {w0})"
    );

    // 0 -> 1 -> 2 -> 1 ... never returns to the entry id. The hop cap must
    // discard the cell instead of storing the repeated samples.
    particles[1] = plant_particle(here);
    particles[2] = plant_particle(here);
    queue.write_buffer(
        system.get_particle_buffer(),
        0,
        bytemuck::cast_slice(&particles),
    );
    next[0] = 1;
    next[1] = 2;
    next[2] = 1;
    let got = dispatch_planted_lambda(&mut system, &device, &queue, &head, &next);
    assert_eq!(
        got[0].density, 0.0,
        "a tail cycle was stored (density {})",
        got[0].density
    );

    // The id past particle_count is not loaded. The one in-range visit remains.
    particles[1] = plant_particle(far);
    particles[2] = plant_particle(far);
    queue.write_buffer(
        system.get_particle_buffer(),
        0,
        bytemuck::cast_slice(&particles),
    );
    next = vec![-1i32; N as usize];
    next[0] = 1_000_000;
    let got = dispatch_planted_lambda(&mut system, &device, &queue, &head, &next);
    let density = got[0].density;
    assert!(
        density.is_finite() && (density - w0).abs() < w0 * 0.02,
        "out-of-range neighbor changed the one in-range visit: {density} vs {w0}"
    );
}

fn read_sdf_r(system: &FluidSystem, device: &wgpu::Device, queue: &wgpu::Queue) -> f32 {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sdf voxel readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sdf voxel copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &system.sdf_system.texture_a,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("sdf map send");
    });
    expect_poll_finished(&device);
    rx.recv().expect("sdf map recv").expect("sdf map");
    let data = slice.get_mapped_range();
    let r = f32::from_le_bytes(data[0..4].try_into().expect("sdf r"));
    drop(data);
    staging.unmap();
    r
}

/// Dropping the caller encoder must not arm the empty-SDF reuse. The empty
/// field is submitted on its own encoder and polled before the flag is set,
/// so voxel 0 is `world_size` (60) even though the step encoder was dropped.
#[test]
fn gpu_dropped_step_encoder_still_fills_empty_sdf() {
    let _gpu = gpu_serial();
    let Some((device, queue)) =
        try_create_test_device("gpu_dropped_step_encoder_still_fills_empty_sdf")
    else {
        return;
    };
    let mut system = FluidSystem::new(&device, 64);
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("dropped step"),
        });
        system.step(&device, &mut encoder, &queue, DT);
        drop(encoder);
    }
    let r = read_sdf_r(&system, &device, &queue);
    assert!(
        (r - 60.0).abs() < 1.0e-3,
        "empty SDF voxel 0 is {r}; a dropped encoder left the zero field in place"
    );
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("dropped step reuse"),
        });
        system.step(&device, &mut encoder, &queue, DT);
        drop(encoder);
    }
    let again = read_sdf_r(&system, &device, &queue);
    assert!(
        (again - 60.0).abs() < 1.0e-3,
        "reused empty SDF voxel 0 is {again}"
    );
}

const DEPTH_SAMPLE_WGSL: &str = r#"
@group(0) @binding(0) var depth_tex: texture_depth_2d;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = p[vi];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = xy * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(depth_tex));
    let coord = vec2<i32>(in.uv * dims);
    let d = textureLoad(depth_tex, coord, 0);
    return vec4<f32>(d, 0.0, 0.0, 1.0);
}
"#;

fn read_rgba16(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> f32 {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("accent color readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("accent color copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("color map send");
    });
    expect_poll_finished(&device);
    rx.recv().expect("color map recv").expect("color map");
    let data = slice.get_mapped_range();
    let bits = u16::from_le_bytes(data[0..2].try_into().expect("f16"));
    drop(data);
    staging.unmap();
    half_to_f32(bits)
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1f) as u32;
    let frac = (bits & 0x3ff) as u32;
    let out = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            let mut e = 127 - 15 + 1;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            ((sign << 31) | (e << 23) | ((f & 0x3ff) << 13)) as u32
        }
    } else if exp == 31 {
        (sign << 31) | 0x7f80_0000 | (frac << 13)
    } else {
        (sign << 31) | ((exp + (127 - 15)) << 23) | (frac << 13)
    };
    f32::from_bits(out)
}

/// The accent store keeps the depth the preceding pass stored, including a
/// frame whose water-style read-only pass sits between that store and the
/// accent load. A later sample of the same texture sees that depth, and the
/// `Depth32Float` copy matches it.
#[test]
fn gpu_accent_depth_store_matches_later_sample() {
    let _gpu = gpu_serial();
    let Some((device, queue)) =
        try_create_test_device("gpu_accent_depth_store_matches_later_sample")
    else {
        return;
    };
    const W: u32 = 8;
    const H: u32 = 8;
    const STORED: f32 = 0.4;
    let format = wgpu::TextureFormat::Rgba16Float;
    let mut system = FluidSystem::new(&device, 64);
    let renderer = astraweave_fluids::FluidRenderer::new(&device, W, H, format);
    let accents = [SecondaryParticle {
        position: [0.0, 0.0, 0.0, 1.0],
        velocity: [0.0, 1.0, 0.0, 0.0],
        info: [0.1, 0.0, 0.9, 0.5],
    }];
    system.set_secondary_particles(&queue, &accents);

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("accent measured depth"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("accent measured color"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("depth sample"),
        source: wgpu::ShaderSource::Wgsl(DEPTH_SAMPLE_WGSL.into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("depth sample bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("depth sample pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("depth sample pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    let depth_sample_view = depth.create_view(&Default::default());
    let depth_attach_view = depth.create_view(&Default::default());
    let color_view = color.create_view(&Default::default());
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("depth sample bg"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&depth_sample_view),
        }],
    });

    let eye = glam::Vec3::new(0.0, 2.0, 6.0);
    let view = glam::Mat4::look_at_rh(eye, glam::Vec3::ZERO, glam::Vec3::Y);
    let proj = glam::Mat4::perspective_rh(1.0, 1.0, 0.1, 50.0);
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

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("accent depth measure"),
    });
    {
        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("store depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_attach_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(STORED),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    // Water's pass: the same depth texture is a read-only attachment and a sample.
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("water-style depth sample"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_attach_view,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.draw(0..3, 0..1);
    }
    renderer.render_accents(
        &queue,
        &mut encoder,
        &color_view,
        &depth_attach_view,
        system.secondary_particle_buffer(),
        system.secondary_particle_count(),
        camera,
    );
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("later depth sample"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_attach_view,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.draw(0..3, 0..1);
    }
    let bytes_per_row = 256u32;
    let depth_read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("depth32 readback"),
        size: u64::from(bytes_per_row) * u64::from(H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &depth,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &depth_read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = depth_read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("depth map send");
    });
    expect_poll_finished(&device);
    rx.recv().expect("depth map recv").expect("depth map");
    let data = slice.get_mapped_range();
    let stored_bits = STORED.to_bits();
    for y in 0..H {
        for x in 0..W {
            let offset = (y * bytes_per_row + x * 4) as usize;
            let bits = u32::from_le_bytes(data[offset..offset + 4].try_into().expect("depth"));
            assert_eq!(
                bits,
                stored_bits,
                "depth at ({x},{y}) is {:e}, the store was {STORED}",
                f32::from_bits(bits)
            );
        }
    }
    drop(data);
    depth_read.unmap();

    let sampled = read_rgba16(&device, &queue, &color);
    assert!(
        (sampled - STORED).abs() < 0.01,
        "later sample saw {sampled}, the stored depth is {STORED}"
    );
}
