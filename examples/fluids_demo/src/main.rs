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
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> Mat4 {
        let view = self.build_view_matrix();
        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
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
    mouse_pos: [f32; 2],
    last_mouse_pos: [f32; 2],
    mouse_left_pressed: bool,
    mouse_right_pressed: bool,
    spawn_burst_size: u32,
    drag_force_strength: f32,

    // Performance controls
    target_particle_count: u32,
    quality_preset: u32, // 0=Low, 1=Medium, 2=High, 3=Ultra
    show_foam: bool,
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

    /// Spawn particles at mouse cursor position
    fn spawn_particles_at_cursor(&mut self) {
        let (origin, dir) = self.screen_to_world_ray(self.mouse_pos[0], self.mouse_pos[1]);

        // Intersect with Y=5 plane (fluid center height)
        if let Some(hit_pos) = self.ray_plane_intersection(origin, dir, 5.0) {
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

                positions.push([hit_pos.x + offset_x, hit_pos.y + 0.5, hit_pos.z + offset_z]);
                velocities.push([0.0, -2.0, 0.0]); // Slight downward velocity
                colors.push([0.3, 0.6, 1.0, 1.0]); // Bright blue
            }

            self.fluid_system
                .spawn_particles(&self.queue, &positions, &velocities, Some(&colors));
            log::info!(
                "Spawned {} particles at ({:.1}, {:.1}, {:.1})",
                count,
                hit_pos.x,
                hit_pos.y,
                hit_pos.z
            );
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

    async fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
        fluid_system.pressure_multiplier = 100.0;
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

        // Scene background texture for refraction
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Setup camera
        let camera = Camera {
            eye: Vec3::new(8.0, 6.0, 8.0),
            target: Vec3::new(3.2, 3.2, 3.2),
            up: Vec3::Y,
            aspect: size.width as f32 / size.height as f32,
            fovy: 45.0,
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
            mouse_right_pressed: false,
            spawn_burst_size: 50,
            drag_force_strength: 10.0,

            // Performance controls
            target_particle_count: 20000,
            quality_preset: 2, // High
            show_foam: true,
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
            self.scene_view = self
                .scene_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.fluid_renderer
                .resize(&self.device, new_size.width, new_size.height);

            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
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

                // Auto-adjust quality preset based on controller tier
                self.quality_preset = result.quality_tier as u32;
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
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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

        // Render to scene texture (Background Pass)
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Background Pass - Clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
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

        // Render skybox to scene texture
        if let Some(ref skybox) = self.skybox_renderer {
            skybox.render(
                &mut encoder,
                &self.scene_view,
                &self.depth_view,
                &self.queue,
                view_proj,
                self.camera.eye,
            );
        }

        // Render current scenario to main view
        if let Some(scenario) = self.scenario_manager.current() {
            let skybox_view = if let Some(ref skybox) = self.skybox_renderer {
                skybox.get_skybox_view()
            } else {
                &self.depth_view // DUMMY
            };

            scenario.render(
                &mut encoder,
                &view,
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
                            ui.label("Viscosity:");
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

                        ui.horizontal(|ui| {
                            ui.label("Drag Force:");
                            ui.add(egui::Slider::new(&mut self.drag_force_strength, 1.0..=50.0));
                        });

                        ui.checkbox(&mut self.show_foam, "Show Foam");

                        ui.add_space(8.0);

                        // Quality Presets
                        ui.heading("🎨 Quality");
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.quality_preset == 0, "Low")
                                .clicked()
                            {
                                self.quality_preset = 0;
                                self.target_particle_count = 5000;
                            }
                            if ui
                                .selectable_label(self.quality_preset == 1, "Med")
                                .clicked()
                            {
                                self.quality_preset = 1;
                                self.target_particle_count = 10000;
                            }
                            if ui
                                .selectable_label(self.quality_preset == 2, "High")
                                .clicked()
                            {
                                self.quality_preset = 2;
                                self.target_particle_count = 20000;
                            }
                            if ui
                                .selectable_label(self.quality_preset == 3, "Ultra")
                                .clicked()
                            {
                                self.quality_preset = 3;
                                self.target_particle_count = 50000;
                            }
                        });

                        ui.label(format!("Target Particles: {}", self.target_particle_count));

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
                        ui.small("Left Click - Spawn particles");
                        ui.small("Right Drag - Apply force");
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
                    view: &view,
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
        output.present();

        Ok(())
    }
}

struct App {
    state: Option<Box<State>>,
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
            self.state = Some(Box::new(pollster::block_on(State::new(window))));
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
                        button,
                        ..
                    } => {
                        match button {
                            winit::event::MouseButton::Left => {
                                let pressed = button_state == ElementState::Pressed;
                                // Spawn particles on left click
                                if pressed && !state.mouse_left_pressed {
                                    state.spawn_particles_at_cursor();
                                }
                                state.mouse_left_pressed = pressed;
                            }
                            winit::event::MouseButton::Right => {
                                state.mouse_right_pressed = button_state == ElementState::Pressed;
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        state.update();
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

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
