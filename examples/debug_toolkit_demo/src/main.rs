use astraweave_core::{ActionStep, IVec2, PlanIntent, Team, World};
use astraweave_camera::{CameraController, FreeFly as Camera};
use astraweave_render::Renderer;
use aw_debug::{watch_reload_signal, watch_scripts, ChromeTraceGuard, PerfHud};
use std::{path::PathBuf, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct App {
    #[allow(dead_code)]
    world: World,
    #[allow(dead_code)]
    player: u32,
    #[allow(dead_code)]
    comp: u32,
    #[allow(dead_code)]
    enemy: u32,
    #[allow(dead_code)]
    plan: Option<PlanIntent>,

    // Debug toolkit integration
    hud: PerfHud,
    last_update: Instant,
    system_timers: Vec<(String, f32)>,
}

impl App {
    fn new() -> Self {
        let mut world = World::new();
        // wall
        for y in 1..=8 {
            world.obstacles.insert((6, y));
        }
        let player = world.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
        let comp = world.spawn("Comp", IVec2 { x: 2, y: 3 }, Team { id: 1 }, 80, 30);
        let enemy = world.spawn("Enemy", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);
        // trivial plan just to show rendering
        let plan = Some(PlanIntent {
            plan_id: "viz".into(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 4,
                    y: 2,
                    speed: None,
                },
                ActionStep::Throw {
                    item: "smoke".into(),
                    x: 7,
                    y: 2,
                },
                ActionStep::CoverFire {
                    target_id: enemy,
                    duration: 2.0,
                },
            ],
        });

        // Initialize debug HUD
        let mut hud = PerfHud::new();
        hud.entity_count = 3; // player, companion, enemy

        // Example system timers
        let system_timers = vec![
            ("physics".into(), 0.5),
            ("ai_planning".into(), 1.2),
            ("rendering".into(), 2.0),
            ("input".into(), 0.1),
        ];
        hud.systems_snapshot = system_timers.clone();

        // Log initial events
        hud.log_event("system", "Application started");
        hud.log_event("world", "World initialized with 3 entities");

        Self {
            world,
            player,
            comp,
            enemy,
            plan,
            hud,
            last_update: Instant::now(),
            system_timers,
        }
    }

    fn update(&mut self) {
        // Simulate system updates and track timing
        let start = Instant::now();

        // Physics update
        std::thread::sleep(std::time::Duration::from_millis(1));
        self.system_timers[0].1 = start.elapsed().as_secs_f32() * 1000.0;

        // AI planning
        std::thread::sleep(std::time::Duration::from_millis(2));
        self.system_timers[1].1 =
            (start.elapsed().as_secs_f32() * 1000.0) - self.system_timers[0].1;

        // Update HUD with latest system timings
        self.hud.systems_snapshot = self.system_timers.clone();

        // Occasionally log events
        if rand::random::<f32>() < 0.05 {
            let events = [
                ("ai", "Companion evaluating plan options"),
                ("physics", "Collision resolved"),
                ("world", "Entity position updated"),
            ];
            let (category, msg) = events[rand::random::<u32>() as usize % events.len()];
            self.hud.log_event(category, msg);
        }

        // Update frame timing in HUD
        self.hud.frame();
    }
}

struct DemoApp {
    window: Option<std::sync::Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Option<Camera>,
    camera_controller: Option<CameraController>,
    egui_ctx: Option<egui::Context>,
    egui_platform: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    app: Option<App>,

    // Keep guards alive
    _trace_guard: ChromeTraceGuard,
    _script_watcher: Option<aw_debug::notify::RecommendedWatcher>,
    _reload_watcher: Option<aw_debug::notify::RecommendedWatcher>,
}

impl DemoApp {
    fn new() -> Self {
        // Initialize Chrome tracing
        let _trace_guard = ChromeTraceGuard::init("astraweave_demo_trace.json");

        // Set up content directory watchers
        let content_dir = PathBuf::from("content");
        std::fs::create_dir_all(&content_dir).ok();

        let _script_watcher = watch_scripts(content_dir.join("encounters"), || {
            println!("Script changed, reloading...");
        })
        .ok();

        let _reload_watcher = watch_reload_signal(content_dir.clone(), || {
            println!("Reload signal detected, reloading level...");
        })
        .ok();

        Self {
            window: None,
            renderer: None,
            camera: None,
            camera_controller: None,
            egui_ctx: None,
            egui_platform: None,
            egui_renderer: None,
            app: None,
            _trace_guard,
            _script_watcher,
            _reload_watcher,
        }
    }
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win = std::sync::Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("AstraWeave Debug Toolkit Demo")
                            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0)),
                    )
                    .unwrap(),
            );
            self.window = Some(win.clone());

            // Initialize renderer
            let r = pollster::block_on(Renderer::new(win.clone())).unwrap();

            // Set up camera
            let c = Camera {
                position: glam::Vec3::new(0.0, 5.0, 10.0),
                yaw: -std::f32::consts::PI / 2.0,
                pitch: -0.6,
                fovy: 60f32.to_radians(),
                aspect: win.inner_size().width as f32 / win.inner_size().height as f32,
                znear: 0.1,
                zfar: 200.0,
            };
            self.camera = Some(c);
            self.camera_controller = Some(CameraController::new(0.2, 0.005));

            // Set up egui integration
            let ctx = egui::Context::default();
            let platform = egui_winit::State::new(
                ctx.clone(),
                egui::ViewportId::default(),
                &win,
                None,
                None,
                None, // Added argument
            );
            let er = egui_wgpu::Renderer::new(r.device(), r.surface_format(), None, 1, false); // Added argument

            self.egui_ctx = Some(ctx);
            self.egui_platform = Some(platform);
            self.egui_renderer = Some(er);
            self.renderer = Some(r);
            self.app = Some(App::new());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(egui_platform), Some(window)) = (&mut self.egui_platform, &self.window) {
            let _ = egui_platform.on_window_event(window, &event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(renderer), Some(camera)) = (&mut self.renderer, &mut self.camera) {
                    if size.width > 0 && size.height > 0 {
                        renderer.resize(size.width, size.height);
                        camera.aspect = size.width as f32 / size.height as f32;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (
                    Some(app),
                    Some(renderer),
                    Some(camera),
                    Some(camera_controller),
                    Some(window),
                    Some(egui_ctx),
                    Some(egui_platform),
                    Some(egui_renderer),
                ) = (
                    &mut self.app,
                    &mut self.renderer,
                    &mut self.camera,
                    &mut self.camera_controller,
                    &self.window,
                    &self.egui_ctx,
                    &mut self.egui_platform,
                    &mut self.egui_renderer,
                ) {
                    // Update app state
                    app.update();

                    // Update camera
                    let dt = app.last_update.elapsed().as_secs_f32();
                    camera_controller.update_camera(camera, dt);

                    // Render manually
                    let surface = renderer.surface();
                    let device = renderer.device();
                    let queue = renderer.queue();
                    let config = renderer.config();
                    let width = config.width;
                    let height = config.height;

                    if let Some(surface) = surface {
                        if let Ok(frame) = surface.get_current_texture() {
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                });

                            // Clear the screen
                            {
                                let _render_pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Main Render Pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                                        r: 0.1,
                                                        g: 0.2,
                                                        b: 0.3,
                                                        a: 1.0,
                                                    }),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });
                            }

                            // Render egui
                            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                                size_in_pixels: [width, height],
                                pixels_per_point: window.scale_factor() as f32,
                            };

                            let egui_input = egui_platform.take_egui_input(window);
                            egui_ctx.begin_pass(egui_input);

                            egui::Window::new("Debug HUD")
                                .default_pos([10.0, 10.0])
                                .default_width(350.0)
                                .show(egui_ctx, |ui| {
                                    app.hud.ui(ui);
                                });

                            let egui_output = egui_ctx.end_pass();
                            let clipped_primitives = egui_ctx
                                .tessellate(egui_output.shapes, egui_output.pixels_per_point);

                            for (id, image_delta) in &egui_output.textures_delta.set {
                                egui_renderer.update_texture(device, queue, *id, image_delta);
                            }

                            {
                                let mut render_pass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Egui Render Pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Load,
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });

                                // SAFETY: The render pass doesn't outlive the encoder scope,
                                // so extending the lifetime to 'static is safe here.
                                // This is required due to egui-wgpu's API in version 0.32.
                                let render_pass_static: &mut wgpu::RenderPass<'static> =
                                    unsafe { std::mem::transmute(&mut render_pass) };
                                egui_renderer.render(
                                    render_pass_static,
                                    &clipped_primitives,
                                    &screen_descriptor,
                                );
                            }

                            for id in &egui_output.textures_delta.free {
                                egui_renderer.free_texture(id);
                            }

                            queue.submit(Some(encoder.finish()));
                            frame.present();
                        }
                    }

                    // Update system render time
                    app.system_timers[2].1 = app.last_update.elapsed().as_secs_f32() * 1000.0;
                    app.last_update = Instant::now();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state,
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                if let Some(camera_controller) = &mut self.camera_controller {
                    camera_controller
                        .process_keyboard(code, state == winit::event::ElementState::Pressed);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(camera_controller) = &mut self.camera_controller {
                    camera_controller
                        .process_mouse_button(button, state == winit::event::ElementState::Pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(camera_controller), Some(camera)) =
                    (&mut self.camera_controller, &mut self.camera)
                {
                    camera_controller.process_mouse_move(
                        camera,
                        glam::Vec2::new(position.x as f32, position.y as f32),
                    );
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = DemoApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
