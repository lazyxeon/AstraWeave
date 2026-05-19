use std::fs;
use std::time::Instant;

use glam::{vec3, Vec2};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use astraweave_audio::AudioEngine;
use astraweave_npc::{
    llm::MockLlm, load_profile_from_toml_str, EngineCommandSink, NpcManager, NpcWorldView,
};
use astraweave_physics::PhysicsWorld;
use astraweave_render::{Camera, CameraController, CameraProducer, Instance, Renderer};

struct App {
    window: Option<std::sync::Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    cam_ctl: CameraController,
    phys: PhysicsWorld,
    audio: AudioEngine,
    npcs: NpcManager,
    merchant_id: u64,
    guard_id: u64,
    utter_hello: bool,
    utter_buy: bool,
    utter_danger: bool,
    last: Instant,
    instances: Vec<Instance>,
}

impl App {
    fn new() -> anyhow::Result<Self> {
        // Physics + audio
        let mut phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
        phys.create_ground_plane(vec3(100.0, 0.0, 100.0), 1.0);
        let mut audio = AudioEngine::new()?;
        audio.set_master_volume(1.0);

        // NPC Manager + profiles
        let mut npcs = NpcManager::new(Box::new(MockLlm));

        let merchant_toml = fs::read_to_string("assets/npc/merchant.toml")?;
        let guard_toml = fs::read_to_string("assets/npc/guard.toml")?;
        let merchant = load_profile_from_toml_str(&merchant_toml)?;
        let guard = load_profile_from_toml_str(&guard_toml)?;

        let merchant_id = npcs.spawn_from_profile(&mut phys, merchant);
        let guard_id = npcs.spawn_from_profile(&mut phys, guard);

        Ok(Self {
            window: None,
            renderer: None,
            camera: Camera {
                position: vec3(0.0, 6.0, 14.0),
                yaw: -1.57,
                pitch: -0.35,
                fovy: 60f32.to_radians(),
                aspect: 16.0 / 9.0,
                znear: 0.1,
                zfar: 300.0,
            },
            cam_ctl: CameraController::new(12.0, 0.005),
            phys,
            audio,
            npcs,
            merchant_id,
            guard_id,
            utter_hello: true,
            utter_buy: false,
            utter_danger: false,
            last: Instant::now(),
            instances: Vec::new(),
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("NPC Town Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let window = std::sync::Arc::new(window);
                    match pollster::block_on(Renderer::new(window.clone())) {
                        Ok(renderer) => {
                            self.window = Some(window);
                            self.renderer = Some(renderer);
                        }
                        Err(e) => {
                            eprintln!("Failed to create renderer: {:?}", e);
                            event_loop.exit();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create window: {:?}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(_window) = self.window.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => {
                renderer.resize(s.width, s.height);
                self.camera.aspect = s.width as f32 / s.height.max(1) as f32;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                let down = state == ElementState::Pressed;
                self.cam_ctl.process_keyboard(code, down);

                if down {
                    match code {
                        KeyCode::KeyH => {
                            self.utter_hello = true;
                            self.utter_buy = false;
                            self.utter_danger = false;
                            println!("Utterance: hello");
                        }
                        KeyCode::KeyB => {
                            self.utter_hello = false;
                            self.utter_buy = true;
                            self.utter_danger = false;
                            println!("Utterance: buy/shop");
                        }
                        KeyCode::KeyD => {
                            self.utter_hello = false;
                            self.utter_buy = false;
                            self.utter_danger = true;
                            println!("Utterance: danger/help");
                        }
                        KeyCode::KeyE => {
                            // talk to nearest (merchant or guard). For demo, alternate:
                            let _glue = EngineCommandSink {
                                phys: &mut self.phys,
                                audio: &mut self.audio,
                            };
                            let view_merchant = NpcWorldView {
                                time_of_day: 12.0,
                                self_pos: vec3(0.0, 1.0, 0.0),
                                player_pos: Some(self.camera.position),
                                player_dist: Some(
                                    self.camera.position.distance(vec3(0.0, 1.0, 0.0)),
                                ),
                                nearby_threat: self.utter_danger,
                                location_tag: Some("market".into()),
                            };
                            let view_guard = NpcWorldView {
                                time_of_day: 12.0,
                                self_pos: vec3(3.0, 1.0, 0.0),
                                player_pos: Some(self.camera.position),
                                player_dist: Some(
                                    self.camera.position.distance(vec3(3.0, 1.0, 0.0)),
                                ),
                                nearby_threat: self.utter_danger,
                                location_tag: Some("gate".into()),
                            };
                            let utter = if self.utter_hello {
                                "hello"
                            } else if self.utter_buy {
                                "buy"
                            } else {
                                "danger"
                            };
                            let _ = self.npcs.handle_player_utterance(
                                self.merchant_id,
                                &view_merchant,
                                utter,
                            );
                            let _ = self.npcs.handle_player_utterance(
                                self.guard_id,
                                &view_guard,
                                utter,
                            );
                            // glue is dropped automatically at end of scope
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.cam_ctl
                        .process_mouse_button(MouseButton::Right, state == ElementState::Pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cam_ctl.process_mouse_move(
                    &mut self.camera,
                    Vec2::new(position.x as f32, position.y as f32),
                );
            }
            WindowEvent::RedrawRequested => {
                let dt = (Instant::now() - self.last).as_secs_f32();
                self.last = Instant::now();
                self.cam_ctl.update_camera(&mut self.camera, dt);
                renderer.update_view(&self.camera.to_render_view());

                // Tick NPC manager with current views (simplified)
                let views = std::iter::once((
                    self.merchant_id,
                    NpcWorldView {
                        time_of_day: 12.0,
                        self_pos: vec3(0.0, 1.0, 0.0),
                        player_pos: Some(self.camera.position),
                        player_dist: Some(self.camera.position.distance(vec3(0.0, 1.0, 0.0))),
                        nearby_threat: self.utter_danger,
                        location_tag: Some("market".into()),
                    },
                ))
                .chain(std::iter::once((
                    self.guard_id,
                    NpcWorldView {
                        time_of_day: 12.0,
                        self_pos: vec3(3.0, 1.0, 0.0),
                        player_pos: Some(self.camera.position),
                        player_dist: Some(self.camera.position.distance(vec3(3.0, 1.0, 0.0))),
                        nearby_threat: self.utter_danger,
                        location_tag: Some("gate".into()),
                    },
                )))
                .collect();

                let mut glue = EngineCommandSink {
                    phys: &mut self.phys,
                    audio: &mut self.audio,
                };
                self.npcs.update(dt, &mut glue, &views);

                // Render simple cubes for "town" + NPCs
                self.instances.clear();
                // ground is drawn by renderer; add NPC markers:
                self.instances.push(Instance::from_pos_scale_color(
                    vec3(0.0, 0.5, 0.0),
                    vec3(0.6, 1.0, 0.6),
                    [0.2, 1.0, 0.4, 1.0],
                )); // merchant
                self.instances.push(Instance::from_pos_scale_color(
                    vec3(3.0, 0.5, 0.0),
                    vec3(0.6, 1.0, 0.6),
                    [0.2, 0.6, 1.0, 1.0],
                )); // guard

                renderer.update_instances(&self.instances);
                if let Err(e) = renderer.render() {
                    eprintln!("{e:?}");
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
