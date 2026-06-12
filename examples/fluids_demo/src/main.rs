use astraweave_physics::PhysicsWorld;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use glam::{Mat4, Vec3};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod ocean_renderer;
mod scenarios;
mod skybox_renderer;

use astraweave_fluids::renderer::CameraUniform;
use astraweave_fluids::{
    FluidLodConfig, FluidLodManager, FluidOptimizationController, FluidRenderer, FluidSystem,
};

use scenarios::{LaboratoryScenario, OceanScenario, ScenarioManager};
use skybox_renderer::SkyboxRenderer;

struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    aspect: f32,
    /// Vertical field of view in **radians** post-C.6.D
    /// (Unified Camera campaign). Pre-C.6.D the field stored degrees with
    /// `.to_radians()` conversion at the projection site (mismatched with
    /// the name; per C.5 audit L.5.11). The semantic conversion is
    /// equivalent — same downstream matrix — but the field-name and unit
    /// now agree per CAMERA_CONVENTIONS.md §2.1.
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> Mat4 {
        let view = self.build_view_matrix();
        // C.6.D: `self.fovy` now stores radians directly (no boundary
        // conversion). Identical result vs pre-C.6.D `self.fovy.to_radians()`
        // when paired with the constructor's unit conversion.
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }

    fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }
}

// RenderMode is now handled by ScenarioManager

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: std::sync::Arc<winit::window::Window>,

    physics_world: PhysicsWorld,
    fluid_system: FluidSystem,
    fluid_renderer: FluidRenderer,
    scenario_manager: ScenarioManager,
    skybox_renderer: Option<SkyboxRenderer>,

    camera: Camera,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,

    egui_ctx: egui::Context,
    egui_state: EguiState,
    egui_renderer: EguiRenderer,

    start_time: Instant,
    last_update: Instant,

    // Camera controls
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
    keys_pressed: std::collections::HashSet<KeyCode>,
    #[allow(dead_code)]
    mouse_captured: bool,

    // Performance tracking
    frame_times: Vec<f32>,
    lod_manager: FluidLodManager,

    // Optimization Controller (NEW)
    optimization_controller: FluidOptimizationController,
    show_optimization_overlay: bool,
    use_controller_stepping: bool,
    last_frame_time_ms: f32,

    // Simulation parameters (egui controlled)
    sim_speed: f32,
    show_debug_panel: bool,

    // Mouse interaction
    // (F.1 removed dead UI state: right-drag force, target_particle_count
    // quality buttons, and show_foam were settable but never read by any
    // update path — audit findings, Integration Completeness #3.)
    mouse_pos: [f32; 2],
    last_mouse_pos: [f32; 2],
    mouse_left_pressed: bool,
    spawn_burst_size: u32,

    // F.1.2: frame capture + scripted-exercise driver
    frame_counter: u64,
    capture_frames: Vec<u64>,
    capture_requested: bool,
    exercise: bool,

    // F.1.3: transient on-screen notice (message, visible-until-frame).
    // Silent dead input is the defect class this campaign keeps paying for;
    // anything that swallows a click must say so on screen.
    notice: Option<(String, u64)>,
}

impl State {
    /// Convert screen coordinates to a ray in world space
    fn screen_to_world_ray(&self, screen_x: f32, screen_y: f32) -> (Vec3, Vec3) {
        let ndc_x = (2.0 * screen_x / self.size.width as f32) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / self.size.height as f32);

        let inv_view_proj = self.camera.build_view_projection_matrix().inverse();

        let near_ndc = glam::Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world = inv_view_proj * near_ndc;
        let far_world = inv_view_proj * far_ndc;

        let near_pos = Vec3::new(near_world.x, near_world.y, near_world.z) / near_world.w;
        let far_pos = Vec3::new(far_world.x, far_world.y, far_world.z) / far_world.w;

        let direction = (far_pos - near_pos).normalize();
        (near_pos, direction)
    }

    /// Intersect ray with horizontal plane at given Y height
    fn ray_plane_intersection(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        plane_y: f32,
    ) -> Option<Vec3> {
        if ray_dir.y.abs() < 0.0001 {
            return None;
        }
        let t = (plane_y - ray_origin.y) / ray_dir.y;
        if t < 0.0 {
            return None;
        }
        Some(ray_origin + ray_dir * t)
    }

    /// Spawn particles at mouse cursor position.
    ///
    /// F.1.3: crate-level respawn was proven working by GPU readback
    /// (`gpu_respawn_reactivates_particles`); the "click does nothing" report
    /// was demo-side UX — invisible-by-similarity spawns (same blue as 18k
    /// existing particles), silent sky-miss returns, and undefined behavior
    /// in the ocean scenario (which doesn't render particles at all). All
    /// three are addressed here: distinct orange burst, ray fallback, and a
    /// visible notice instead of silently swallowing the click.
    fn spawn_particles_at_cursor(&mut self) {
        // The ocean scenario never draws the particle system; spawning there
        // would silently drain the pool with zero visible effect.
        if self.scenario_manager.current_index() != 0 {
            self.notice = Some((
                "Particle spawning is Laboratory-only (this scenario doesn't render particles)"
                    .to_string(),
                self.frame_counter + 180,
            ));
            return;
        }

        let (origin, dir) = self.screen_to_world_ray(self.mouse_pos[0], self.mouse_pos[1]);

        // Intersect with the Y=5 plane; when the click aims at the sky (no
        // forward intersection), fall back to a point 25 units along the ray
        // so a click ALWAYS produces a visible burst rather than silence.
        let hit = self
            .ray_plane_intersection(origin, dir, 5.0)
            .unwrap_or_else(|| origin + dir * 25.0);
        // Cap the spawn distance: a plane hit far across the domain would
        // make the burst a few pixels tall — keep it near enough to read.
        let hit = if (hit - origin).length() > 30.0 {
            origin + dir * 25.0
        } else {
            hit
        };
        {
            // F.1.3: spawn ABOVE the foam line (+10), not inside it. The SSFR
            // pipeline renders every particle as the same water material
            // (Particle.color is never sampled by the shade pass — ledgered
            // for F.4), so a burst materializing INSIDE the existing 18k-
            // particle foam is invisible by indistinguishability: capture-
            // verified, and the root of the persistent "click does nothing"
            // report despite GPU-readback-proven respawn. Dropping the burst
            // in from the sky makes every click unambiguous.
            let hit_pos = Vec3::new(
                hit.x.clamp(-29.0, 29.0),
                (hit.y + 10.0).clamp(0.5, 55.0),
                hit.z.clamp(-29.0, 29.0),
            );
            let count = self.spawn_burst_size as usize;
            let mut positions = Vec::with_capacity(count);
            let mut velocities = Vec::with_capacity(count);
            let mut colors = Vec::with_capacity(count);

            for i in 0..count {
                // Random spread around hit position
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let radius = (i as f32 * 0.1).sin() * 0.5;
                let offset_x = angle.cos() * radius;
                let offset_z = angle.sin() * radius;

                // Clamp to the shader's hardcoded sim domain
                // (|x|,|z| <= 29.5, 0 <= y <= 59.5).
                positions.push([
                    (hit_pos.x + offset_x).clamp(-29.0, 29.0),
                    (hit_pos.y + 0.5).clamp(0.5, 59.0),
                    (hit_pos.z + offset_z).clamp(-29.0, 29.0),
                ]);
                velocities.push([0.0, -2.0, 0.0]); // Slight downward velocity
                                                   // F.1.3: distinct ORANGE so spawns are visible against the
                                                   // 18k blue particles (the old blue-on-blue bursts were
                                                   // unspottable — one root of "click does nothing").
                colors.push([1.0, 0.45, 0.1, 1.0]);
            }

            let spawned = self.fluid_system.spawn_particles(
                &self.queue,
                &positions,
                &velocities,
                Some(&colors),
            );
            // Safe at the max_particles cap by construction: spawn_particles
            // draws min(requested, free_list.len()) — zero spawns when the
            // reserve pool is exhausted, never a panic or wrap.
            log::info!(
                "Spawned {}/{} particles at ({:.1}, {:.1}, {:.1}); free pool now {}",
                spawned,
                count,
                hit_pos.x,
                hit_pos.y,
                hit_pos.z,
                self.fluid_system.max_particles - self.fluid_system.active_count
            );
            if spawned == 0 {
                self.notice = Some((
                    "Spawn pool exhausted — switch scenarios (SPACE twice) to refill".to_string(),
                    self.frame_counter + 180,
                ));
            }
        }
    }

    fn toggle_render_mode(&mut self) {
        self.scenario_manager.next();
        if let Some(scenario) = self.scenario_manager.current() {
            log::info!("Switching to scenario: {}", scenario.name());
            scenario.init(
                &self.device,
                &self.queue,
                &mut self.fluid_system,
                &mut self.physics_world,
            );
        }
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        self.egui_state
            .on_window_event(&self.window, event)
            .consumed
    }

    async fn new(window: std::sync::Arc<winit::window::Window>, opts: DemoOptions) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            // COPY_SRC: the F.1.2 frame-capture path (F12 / --capture-frames)
            // copies the pre-present swapchain texture to a readback buffer.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Initialize physics world with gravity
        let mut physics_world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        // Initialize fluid system with 20000 particles
        let particle_count = 20000;
        let mut fluid_system = FluidSystem::new(&device, particle_count);

        // The fluid system parameters are public fields, so we can set them directly
        fluid_system.smoothing_radius = 0.5;
        fluid_system.target_density = 1.0;
        fluid_system.viscosity = 0.01;
        fluid_system.gravity = -9.81;
        fluid_system.cell_size = 1.2;
        fluid_system.grid_width = 64;
        fluid_system.grid_height = 64;
        fluid_system.grid_depth = 64;

        log::info!("Initialized fluid system with {} particles", particle_count);

        // Initialize scenarios
        let mut scenario_manager = ScenarioManager::new();
        scenario_manager.add_scenario(Box::new(LaboratoryScenario::new()));
        scenario_manager.add_scenario(Box::new(OceanScenario::new(
            &device,
            &queue,
            surface_format,
        )));

        if let Some(scenario) = scenario_manager.current() {
            scenario.init(&device, &queue, &mut fluid_system, &mut physics_world);
        }
        if let Some(st) = opts.surface_tension {
            fluid_system.surface_tension = st;
        }

        // Initialize skybox renderer
        let skybox_path = "assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr";
        let skybox_renderer = if std::path::Path::new(skybox_path).exists() {
            Some(SkyboxRenderer::new(
                &device,
                &queue,
                surface_format,
                skybox_path,
            ))
        } else {
            None
        };

        // Initialize fluid renderer
        let fluid_renderer = FluidRenderer::new(&device, size.width, size.height, surface_format);

        // Scene background texture for refraction (filled per frame by a
        // swapchain copy after the background passes — F.1.2 H-6b)
        let scene_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Background Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Setup camera
        let camera = Camera {
            eye: Vec3::new(8.0, 6.0, 8.0),
            target: Vec3::new(3.2, 3.2, 3.2),
            up: Vec3::Y,
            // C.6.D: `.max(0.01)` aspect guard per CAMERA_CONVENTIONS.md §2.3.
            aspect: (size.width as f32 / size.height as f32).max(0.01),
            // C.6.D: fovy stores radians directly (was degrees pre-C.6.D).
            fovy: 45_f32.to_radians(),
            znear: 0.1,
            zfar: 100.0,
        };

        // Initialize Egui
        let egui_ctx = egui::Context::default();
        let egui_state = EguiState::new(
            egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let egui_renderer = EguiRenderer::new(&device, surface_format, None, 1, false);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            physics_world,
            fluid_system,
            fluid_renderer,
            scenario_manager,
            skybox_renderer,
            camera,
            depth_texture,
            depth_view,
            scene_texture,
            scene_view,

            // Egui
            egui_ctx,
            egui_state,
            egui_renderer,

            start_time: Instant::now(),
            last_update: Instant::now(),

            // Camera controls
            camera_yaw: 0.0,
            camera_pitch: 0.3,
            camera_distance: 30.0,
            keys_pressed: std::collections::HashSet::new(),
            mouse_captured: false,

            // Performance tracking
            frame_times: Vec::with_capacity(60),
            lod_manager: FluidLodManager::new(FluidLodConfig::default()),

            // Optimization Controller
            optimization_controller: {
                let mut controller = FluidOptimizationController::new();
                controller.set_target_framerate(60.0);
                controller.enable_lod([0.0, 5.0, 0.0]);
                controller.set_auto_tune(true);
                controller
            },
            show_optimization_overlay: false,
            use_controller_stepping: false,
            last_frame_time_ms: 16.67,

            // Simulation parameters
            sim_speed: 1.0,
            show_debug_panel: true,

            // Mouse interaction
            mouse_pos: [0.0, 0.0],
            last_mouse_pos: [0.0, 0.0],
            mouse_left_pressed: false,
            spawn_burst_size: 50,

            frame_counter: 0,
            capture_frames: opts.capture_frames,
            capture_requested: false,
            exercise: opts.exercise,
            notice: None,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate depth texture
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            // F.1.2 (H-1): the view must be recreated too — pre-fix this kept
            // viewing the ORIGINAL startup-sized texture, so after maximize
            // every pass pairing depth_view with a swapchain attachment
            // panicked with "Attachments have differing sizes" (the owner's
            // captured crash, via the ocean scenario's swapchain+depth pass).
            self.depth_view = self
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // F.1.2 (H-1): scene_texture itself was never recreated either —
            // only its view was refreshed, still pointing at the startup-sized
            // texture (stale refraction source after resize).
            self.scene_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Scene Background Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.scene_view = self
                .scene_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.fluid_renderer
                .resize(&self.device, new_size.width, new_size.height);

            // C.6.D: `.max(0.01)` aspect guard at resize per
            // CAMERA_CONVENTIONS.md §2.3.
            self.camera.aspect = (new_size.width as f32 / new_size.height as f32).max(0.01);
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let raw_dt = (now - self.last_update).as_secs_f32();
        self.last_update = now;

        // Frame time averaging
        if self.frame_times.len() >= 60 {
            self.frame_times.remove(0);
        }
        self.frame_times.push(raw_dt);

        // Cap dt and apply simulation speed
        let dt = (raw_dt.min(0.016) * self.sim_speed).max(0.0001);

        // Camera orbit controls (WASD + scroll)
        let move_speed = 2.0 * raw_dt;
        let orbit_speed = 1.5 * raw_dt;

        if self.keys_pressed.contains(&KeyCode::KeyW) {
            self.camera_pitch += orbit_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyS) {
            self.camera_pitch -= orbit_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyA) {
            self.camera_yaw -= orbit_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyD) {
            self.camera_yaw += orbit_speed;
        }
        if self.keys_pressed.contains(&KeyCode::KeyQ) {
            self.camera_distance += move_speed * 10.0;
        }
        if self.keys_pressed.contains(&KeyCode::KeyE) {
            self.camera_distance -= move_speed * 10.0;
        }

        // Clamp camera values
        self.camera_pitch = self.camera_pitch.clamp(-1.4, 1.4);
        self.camera_distance = self.camera_distance.clamp(5.0, 100.0);

        // Update camera position from orbit
        let target = Vec3::new(0.0, 5.0, 0.0);
        let x = self.camera_distance * self.camera_yaw.cos() * self.camera_pitch.cos();
        let y = self.camera_distance * self.camera_pitch.sin();
        let z = self.camera_distance * self.camera_yaw.sin() * self.camera_pitch.cos();
        self.camera.eye = target + Vec3::new(x, y, z);
        self.camera.target = target;

        // LOD check
        let fluid_center = [0.0_f32, 5.0_f32, 0.0_f32];
        let camera_pos = [self.camera.eye.x, self.camera.eye.y, self.camera.eye.z];
        let should_simulate = self.lod_manager.update(camera_pos, fluid_center);

        // Create encoder for fluid simulation
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Fluid Update Encoder"),
            });

        // Update fluid simulation (only if LOD allows and sim_speed > 0)
        if should_simulate && self.sim_speed > 0.0 {
            if self.use_controller_stepping {
                // Use FluidOptimizationController for auto-tuned stepping
                let step_start = Instant::now();
                let result = self.optimization_controller.step_with_budget(
                    &mut self.fluid_system,
                    &self.device,
                    &mut encoder,
                    &self.queue,
                    dt,
                    self.last_frame_time_ms,
                    camera_pos,
                );
                self.last_frame_time_ms = step_start.elapsed().as_secs_f32() * 1000.0;
                // (Auto-tune quality tier is surfaced via the controller
                // status label; the former quality_preset mirror field was
                // dead UI state, removed in F.1.)
                let _ = result.quality_tier;
            } else {
                // Traditional direct stepping
                self.fluid_system
                    .step(&self.device, &mut encoder, &self.queue, dt);

                // Still record frame for metrics display
                self.optimization_controller.record_frame(raw_dt * 1000.0);
            }
        }

        // Update current scenario
        if let Some(scenario) = self.scenario_manager.current() {
            scenario.update(
                dt,
                &mut self.fluid_system,
                &mut self.physics_world,
                self.camera.eye,
                &self.device,
                &self.queue,
            );
        }

        // Submit encoder
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // F.1.2 (H-1): every screen-sized target must track the swapchain.
        // This names the failure class at its source instead of a generic
        // "Attachments have differing sizes" panic deep inside a pass.
        debug_assert!(
            self.depth_texture.width() == self.config.width
                && self.depth_texture.height() == self.config.height,
            "demo depth target {}x{} out of sync with swapchain {}x{} — resize() missed it",
            self.depth_texture.width(),
            self.depth_texture.height(),
            self.config.width,
            self.config.height,
        );
        debug_assert!(
            self.scene_texture.width() == self.config.width
                && self.scene_texture.height() == self.config.height,
            "scene target {}x{} out of sync with swapchain {}x{} — resize() missed it",
            self.scene_texture.width(),
            self.scene_texture.height(),
            self.config.width,
            self.config.height,
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // F.1.2 (H-4): contain frame-encode panics. Pre-fix, a panic during
        // encoding unwound through the live SurfaceTexture, whose teardown
        // assert ("SurfaceSemaphores still in use") panicked AGAIN during
        // unwind -> STATUS_STACK_BUFFER_OVERRUN abort burying the real error.
        // Catching here lets us drop the SurfaceTexture in a controlled order
        // and exit with exactly one diagnostic.
        let frame = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.encode_frame(&view, &output.texture);
        }));
        match frame {
            Ok(()) => {
                self.capture_frame_if_requested(&output.texture);
                output.present();
                self.frame_counter += 1;
                Ok(())
            }
            Err(payload) => {
                drop(output);
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                eprintln!("FATAL: frame encode panicked: {msg}");
                std::process::exit(1);
            }
        }
    }

    fn encode_frame(&mut self, view: &wgpu::TextureView, surface_texture: &wgpu::Texture) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Build Camera Uniform
        let view_proj = self.camera.build_view_projection_matrix();
        let inv_view_proj = view_proj.inverse();
        let view_inv = self.camera.build_view_matrix().inverse();
        let light_dir = glam::Vec3::new(0.5, 1.0, 0.2).normalize();
        let camera_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_inv: view_inv.to_cols_array_2d(),
            cam_pos: [self.camera.eye.x, self.camera.eye.y, self.camera.eye.z, 1.0],
            light_dir: [light_dir.x, light_dir.y, light_dir.z, 1.0],
            time: self.start_time.elapsed().as_secs_f32(),
            padding: [0.0; 19],
        };

        // F.1.2 (H-6b): render the background to the SWAPCHAIN, then copy it
        // into scene_texture as the refraction source. Pre-fix, the clear +
        // skybox went ONLY into scene_texture and nothing ever drew a
        // background on the swapchain — the SSFR shade pass discards where
        // there is no fluid, so the visible background was zero-init black
        // ("pearls in a void") in both scenarios, and the fluid could only
        // ever composite against blackness.
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Background Pass - Clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            // Drop _rpass here
        }

        // Render skybox to the swapchain (visible background)
        if let Some(ref skybox) = self.skybox_renderer {
            skybox.render(
                &mut encoder,
                view,
                &self.depth_view,
                &self.queue,
                view_proj,
                self.camera.eye,
            );
        }

        // Snapshot the background into scene_texture: the SSFR shade pass
        // samples it for refraction, so it must contain what is actually
        // behind the fluid on screen.
        encoder.copy_texture_to_texture(
            surface_texture.as_image_copy(),
            self.scene_texture.as_image_copy(),
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );

        // Render current scenario to main view
        if let Some(scenario) = self.scenario_manager.current() {
            let skybox_view = if let Some(ref skybox) = self.skybox_renderer {
                skybox.get_skybox_view()
            } else {
                &self.depth_view // DUMMY
            };

            scenario.render(
                &mut encoder,
                view,
                &self.scene_view,
                &self.depth_view, // Scene depth
                &self.depth_view, // Fluid raw depth target
                &self.device,
                &self.queue,
                &self.fluid_system,
                &self.fluid_renderer,
                camera_uniform,
                skybox_view,
            );
        }

        // --- Egui Performance Overlay ---
        let raw_input = self.egui_state.take_egui_input(&self.window);

        // Calculate average FPS from frame times
        let avg_dt = if !self.frame_times.is_empty() {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        } else {
            0.016
        };
        let avg_fps = 1.0 / avg_dt;
        let min_fps = 1.0
            / self
                .frame_times
                .iter()
                .cloned()
                .fold(0.0f32, f32::max)
                .max(0.001);
        let _max_fps = 1.0
            / self
                .frame_times
                .iter()
                .cloned()
                .fold(f32::INFINITY, f32::min)
                .max(0.001);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // F.1.3: transient notice (e.g. "spawning is Laboratory-only").
            if let Some((msg, until)) = &self.notice {
                if self.frame_counter < *until {
                    egui::Area::new(egui::Id::new("f13_notice"))
                        .anchor(egui::Align2::CENTER_TOP, [0.0, 24.0])
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.colored_label(egui::Color32::YELLOW, msg);
                            });
                        });
                } else {
                    self.notice = None;
                }
            }

            if self.show_debug_panel {
                egui::Window::new("🎮 Fluids Demo")
                    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
                    .resizable(false)
                    .collapsible(true)
                    .show(ctx, |ui| {
                        // Performance Section
                        ui.heading("📊 Performance");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("FPS:");
                            ui.label(format!("{:.0} avg / {:.0} min", avg_fps, min_fps));
                        });
                        ui.label(format!("Frame: {:.2}ms", avg_dt * 1000.0));
                        ui.label(format!("Particles: {}", self.fluid_system.particle_count));
                        ui.label(format!("LOD: {:?}", self.lod_manager.current_lod()));

                        ui.add_space(8.0);

                        // Simulation Controls
                        ui.heading("⚙️ Simulation");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Speed:");
                            ui.add(egui::Slider::new(&mut self.sim_speed, 0.0..=2.0).suffix("x"));
                        });

                        ui.horizontal(|ui| {
                            // Honest label (F.1): this parameter scales the
                            // vorticity-confinement gain in the shader; the
                            // XSPH viscosity blend is hardcoded at 0.01.
                            ui.label("Vorticity (\"viscosity\"):").on_hover_text(
                                "Scales vorticity confinement. The XSPH viscosity \
                                     blend itself is hardcoded in fluid.wgsl.",
                            );
                            ui.add(egui::Slider::new(
                                &mut self.fluid_system.viscosity,
                                0.0..=100.0,
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Surface Tension:");
                            ui.add(egui::Slider::new(
                                &mut self.fluid_system.surface_tension,
                                0.0..=1.0,
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Gravity:");
                            ui.add(egui::Slider::new(
                                &mut self.fluid_system.gravity,
                                -20.0..=0.0,
                            ));
                        });

                        ui.add_space(8.0);

                        // Camera Info
                        ui.heading("📷 Camera");
                        ui.separator();
                        ui.label(format!("Distance: {:.1}", self.camera_distance));
                        ui.label(format!("Pitch: {:.2}rad", self.camera_pitch));

                        // Interactive Controls
                        ui.heading("🖱️ Interaction");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Spawn Burst:");
                            ui.add(egui::Slider::new(&mut self.spawn_burst_size, 10..=200));
                        });

                        // (F.1 removed the "Drag Force" slider, "Show Foam"
                        // checkbox, and the quality-preset buttons: none of
                        // them were read by any update path — the buttons set
                        // target_particle_count which was never applied.)

                        ui.add_space(8.0);

                        // Optimization Controller Section
                        ui.heading("⚡ Optimization");
                        ui.separator();

                        ui.checkbox(&mut self.use_controller_stepping, "Auto-Tune Mode");
                        ui.checkbox(&mut self.show_optimization_overlay, "Show Details (F2)");

                        if self.use_controller_stepping {
                            let status = self.optimization_controller.status();
                            ui.label(format!("Quality Tier: {}", status.quality_tier));
                            ui.label(format!(
                                "Iterations: {}",
                                self.optimization_controller.recommended_iterations()
                            ));

                            let headroom = self.optimization_controller.budget_headroom();
                            let headroom_color = if headroom > 30.0 {
                                egui::Color32::GREEN
                            } else if headroom > 10.0 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(headroom_color, format!("Headroom: {:.1}%", headroom));
                        }

                        ui.add_space(8.0);

                        // Controls Help
                        ui.heading("🎮 Controls");
                        ui.separator();
                        ui.small("WASD - Orbit camera");
                        ui.small("Q/E - Zoom in/out");
                        ui.small("SPACE - Switch scenario");
                        ui.small("Left Click - Spawn particles (Laboratory only)");
                        ui.small("F1 - Toggle this panel");
                        ui.small("F2 - Toggle optimization overlay");
                        ui.small("ESC - Exit");
                    });
            }

            // Optimization Overlay (separate window)
            if self.show_optimization_overlay {
                egui::Window::new("⚡ Optimization Details")
                    .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
                    .resizable(false)
                    .collapsible(true)
                    .show(ctx, |ui| {
                        let status = self.optimization_controller.status();

                        ui.heading("Performance Budget");
                        ui.separator();

                        // Progress bar for frame budget
                        let budget_usage = if status.target_frame_time_ms > 0.0 {
                            status.avg_frame_time_ms / status.target_frame_time_ms
                        } else {
                            0.0
                        };
                        ui.add(
                            egui::ProgressBar::new(budget_usage.min(1.5) / 1.5).text(format!(
                                "{:.2}ms / {:.2}ms",
                                status.avg_frame_time_ms, status.target_frame_time_ms
                            )),
                        );

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Within Budget:");
                            if status.within_budget {
                                ui.colored_label(egui::Color32::GREEN, "✓ Yes");
                            } else {
                                ui.colored_label(egui::Color32::RED, "✗ No");
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Auto-Tune:");
                            if status.auto_tune_enabled {
                                ui.colored_label(egui::Color32::GREEN, "Enabled");
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "Disabled");
                            }
                        });

                        ui.add_space(8.0);
                        ui.heading("Quality Settings");
                        ui.separator();

                        let tier_names = ["Ultra", "High", "Medium", "Low", "Potato"];
                        let tier_name = tier_names
                            .get(status.quality_tier as usize)
                            .unwrap_or(&"Unknown");
                        ui.label(format!("Tier: {} ({})", status.quality_tier, tier_name));
                        ui.label(format!(
                            "Iterations: {}",
                            self.optimization_controller.recommended_iterations()
                        ));
                        ui.label(format!("Frames Recorded: {}", status.frames_recorded));

                        ui.add_space(8.0);
                        ui.heading("Manual Controls");
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("⬆ Increase Quality").clicked() {
                                let current = self.optimization_controller.quality_tier();
                                if current > 0 {
                                    self.optimization_controller.set_quality_tier(current - 1);
                                }
                            }
                            if ui.button("⬇ Decrease Quality").clicked() {
                                let current = self.optimization_controller.quality_tier();
                                if current < 4 {
                                    self.optimization_controller.set_quality_tier(current + 1);
                                }
                            }
                        });

                        if ui.button("Reset Metrics").clicked() {
                            self.optimization_controller.reset_metrics();
                        }

                        ui.add_space(8.0);
                        ui.heading("Target Framerate");
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(status.target_frame_time_ms > 30.0, "30")
                                .clicked()
                            {
                                self.optimization_controller.set_target_framerate(30.0);
                            }
                            if ui
                                .selectable_label(
                                    (status.target_frame_time_ms - 16.67).abs() < 1.0,
                                    "60",
                                )
                                .clicked()
                            {
                                self.optimization_controller.set_target_framerate(60.0);
                            }
                            if ui
                                .selectable_label(
                                    (status.target_frame_time_ms - 8.33).abs() < 1.0,
                                    "120",
                                )
                                .clicked()
                            {
                                self.optimization_controller.set_target_framerate(120.0);
                            }
                            if ui
                                .selectable_label(status.target_frame_time_ms < 7.0, "144")
                                .clicked()
                            {
                                self.optimization_controller.set_target_framerate(144.0);
                            }
                        });
                    });
            }
        });

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let rpass_static: &mut wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute(&mut rpass) };
            self.egui_renderer
                .render(rpass_static, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// F.1.2 (H-5): write the pre-present swapchain image to
    /// `examples/fluids_demo/captures/` as PNG when this frame is in the
    /// `--capture-frames` list or F12 was pressed. Blocking readback —
    /// capture frames stutter by design.
    fn capture_frame_if_requested(&mut self, texture: &wgpu::Texture) {
        let due = self.capture_requested || self.capture_frames.contains(&self.frame_counter);
        if !due {
            return;
        }
        self.capture_requested = false;

        let (w, h) = (texture.width(), texture.height());
        let bytes_per_pixel = 4u32;
        let unpadded = w * bytes_per_pixel;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame capture readback"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame capture"),
            });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        if rx.recv().map(|r| r.is_err()).unwrap_or(true) {
            log::error!("frame capture: readback mapping failed");
            return;
        }

        let data = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((unpadded * h) as usize);
        let bgra = matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        for row in 0..h {
            let start = (row * padded) as usize;
            let row_bytes = &data[start..start + unpadded as usize];
            if bgra {
                for px in row_bytes.chunks_exact(4) {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
                }
            } else {
                for px in row_bytes.chunks_exact(4) {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        drop(data);
        buffer.unmap();

        let dir = std::path::Path::new("examples/fluids_demo/captures");
        let dir = if std::path::Path::new("examples/fluids_demo").exists() {
            dir.to_path_buf()
        } else {
            std::path::PathBuf::from("captures") // launched from the demo dir
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("frame capture: cannot create {dir:?}: {e}");
            return;
        }
        let path = dir.join(format!("f{:04}_{}x{}.png", self.frame_counter, w, h));
        match image::RgbaImage::from_raw(w, h, rgba) {
            Some(img) => match img.save(&path) {
                Ok(()) => log::info!("frame capture written: {path:?}"),
                Err(e) => log::error!("frame capture: PNG write failed: {e}"),
            },
            None => log::error!("frame capture: buffer size mismatch"),
        }
    }

    /// F.1.2 (H-2 driver): scripted resize/scenario-switch/click sequence,
    /// exits 0 at the end. Frames: 80 maximize-ish resize, 140 -> ocean,
    /// 200 -> back to lab, 230 second resize, 260 center click-spawn,
    /// 440 exit.
    fn run_exercise_step(&mut self, event_loop: &ActiveEventLoop) {
        match self.frame_counter {
            80 => {
                let _ = self
                    .window
                    .request_inner_size(winit::dpi::PhysicalSize::new(1600u32, 900u32));
            }
            140 => self.toggle_render_mode(),
            200 => self.toggle_render_mode(),
            230 => {
                let _ = self
                    .window
                    .request_inner_size(winit::dpi::PhysicalSize::new(1100u32, 700u32));
            }
            // Click-spawns happen LATE, once the re-initialized dam (frame
            // 200) has opened the view: a sky-aimed burst against open
            // background is the unambiguous capture evidence H-1 requires.
            380 => {
                self.spawn_burst_size = 150;
                self.mouse_pos = [self.size.width as f32 * 0.3, self.size.height as f32 * 0.18];
                self.spawn_particles_at_cursor();
            }
            450 => {
                let _ = self
                    .window
                    .request_inner_size(winit::dpi::PhysicalSize::new(800u32, 600u32));
            }
            480 => {
                self.mouse_pos = [self.size.width as f32 * 0.3, self.size.height as f32 * 0.18];
                self.spawn_particles_at_cursor();
            }
            660 => {
                eprintln!("EXERCISE COMPLETE: clean exit");
                event_loop.exit();
            }
            _ => {}
        }
    }
}

struct App {
    state: Option<Box<State>>,
    opts: DemoOptions,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window = std::sync::Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes().with_title("AstraWeave Fluids Demo"),
                    )
                    .unwrap(),
            );
            self.state = Some(Box::new(pollster::block_on(State::new(
                window,
                self.opts.clone(),
            ))));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = &mut self.state {
            if window_id == state.window.id() {
                if state.handle_window_event(&event) {
                    return;
                }
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => event_loop.exit(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Space),
                                ..
                            },
                        ..
                    } => {
                        state.toggle_render_mode();
                    }
                    // F1 - Toggle debug panel
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::F1),
                                ..
                            },
                        ..
                    } => {
                        state.show_debug_panel = !state.show_debug_panel;
                    }
                    // F2 - Toggle optimization overlay
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::F2),
                                ..
                            },
                        ..
                    } => {
                        state.show_optimization_overlay = !state.show_optimization_overlay;
                    }
                    // F12 - Capture next frame to captures/ (F.1.2)
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::F12),
                                ..
                            },
                        ..
                    } => {
                        state.capture_requested = true;
                    }
                    // R - Reset camera
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::KeyR),
                                ..
                            },
                        ..
                    } => {
                        state.camera_yaw = 0.0;
                        state.camera_pitch = 0.3;
                        state.camera_distance = 30.0;
                    }
                    // Track key presses for camera movement
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: key_state,
                                physical_key: PhysicalKey::Code(code),
                                ..
                            },
                        ..
                    } => match key_state {
                        ElementState::Pressed => {
                            state.keys_pressed.insert(code);
                        }
                        ElementState::Released => {
                            state.keys_pressed.remove(&code);
                        }
                    },
                    // Mouse cursor tracking
                    WindowEvent::CursorMoved { position, .. } => {
                        state.last_mouse_pos = state.mouse_pos;
                        state.mouse_pos = [position.x as f32, position.y as f32];
                    }
                    // Mouse button handling
                    WindowEvent::MouseInput {
                        state: button_state,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        let pressed = button_state == ElementState::Pressed;
                        // Spawn particles on left click
                        if pressed && !state.mouse_left_pressed {
                            state.spawn_particles_at_cursor();
                        }
                        state.mouse_left_pressed = pressed;
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        state.update();
                        if state.exercise {
                            state.run_exercise_step(event_loop);
                            if !event_loop.exiting() {
                                // fall through to render
                            } else {
                                return;
                            }
                        }
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

/// Demo launch options (F.1.2).
///
/// `--capture-frames N,M,...` writes PNGs of those frame indices to
/// `examples/fluids_demo/captures/` (also bindable at runtime via F12,
/// which captures the next frame). `--exercise` drives a scripted
/// resize/scenario-switch/click sequence and exits with code 0 — the
/// headless-ish regression driver used by the F.1.2 verification gate.
#[derive(Clone, Default)]
struct DemoOptions {
    capture_frames: Vec<u64>,
    exercise: bool,
    /// Override the scenario's surface-tension default at startup (used to
    /// produce the H-6d low/high comparison capture pair).
    surface_tension: Option<f32>,
}

fn parse_options() -> DemoOptions {
    let mut opts = DemoOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(list) = arg.strip_prefix("--capture-frames=") {
            opts.capture_frames = list
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        } else if arg == "--capture-frames" {
            if let Some(list) = args.next() {
                opts.capture_frames = list
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
            }
        } else if arg == "--exercise" {
            opts.exercise = true;
        } else if let Some(v) = arg.strip_prefix("--surface-tension=") {
            opts.surface_tension = v.trim().parse().ok();
        }
    }
    // The exercise gate wants eyes at fixed points: startup, post-maximize,
    // ocean scenario, back in lab post-click, second resize, settled.
    if opts.exercise && opts.capture_frames.is_empty() {
        // 30 baseline; 120 post-resize aspect check; 170 ocean;
        // 381/440 click-spawn pair @1100x700 (click@380, sky-aimed, after the
        // scene opens up); 481/540 second pair @800x600 (resize@450,
        // click@480); 620 settled basin.
        opts.capture_frames = vec![30, 120, 170, 381, 440, 481, 540, 620];
    }
    opts
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        state: None,
        opts: parse_options(),
    };
    event_loop.run_app(&mut app).unwrap();
}
