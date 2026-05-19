use astraweave_audio::{AudioEngine, ListenerPose, MusicTrack};
use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_render::Renderer;
use glam::{vec3, Vec2};
use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    cam_ctl: CameraController,
    audio: AudioEngine,
    last: Instant,
    _emitter_id: u64,
}

impl App {
    fn new() -> anyhow::Result<Self> {
        let mut audio = AudioEngine::new()?;
        audio.set_master_volume(1.0);

        // Try to play BGM (will error if file missing; safe to comment or replace)
        let _ = audio.play_music(
            MusicTrack {
                path: "assets/audio/bgm.ogg".into(),
                looped: true,
            },
            1.0,
        );

        Ok(Self {
            window: None,
            renderer: None,
            camera: Camera {
                position: vec3(0.0, 2.0, 6.0),
                yaw: -1.57,
                pitch: -0.2,
                fovy: 60f32.to_radians(),
                aspect: 16.0 / 9.0,
                znear: 0.1,
                zfar: 300.0,
            },
            cam_ctl: CameraController::new(12.0, 0.005),
            audio,
            last: Instant::now(),
            _emitter_id: 1,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Audio Spatial Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let window = Arc::new(window);
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
                self.cam_ctl
                    .process_keyboard(code, state == ElementState::Pressed);
                if state == ElementState::Pressed {
                    match code {
                        KeyCode::Digit1 => {
                            // center beep
                            let _ = self.audio.play_sfx_3d_beep(
                                100,
                                vec3(0.0, 1.0, 0.0),
                                880.0,
                                0.25,
                                0.5,
                            );
                        }
                        KeyCode::Digit2 => {
                            // left beep
                            let _ = self.audio.play_sfx_3d_beep(
                                101,
                                vec3(-3.0, 1.0, 0.0),
                                660.0,
                                0.25,
                                0.5,
                            );
                        }
                        KeyCode::Digit3 => {
                            // right beep
                            let _ = self.audio.play_sfx_3d_beep(
                                102,
                                vec3(3.0, 1.0, 0.0),
                                440.0,
                                0.25,
                                0.5,
                            );
                        }
                        KeyCode::KeyM => {
                            // switch music (crossfade)
                            let _ = self.audio.play_music(
                                MusicTrack {
                                    path: "assets/audio/bgm_alt.ogg".into(),
                                    looped: true,
                                },
                                1.25,
                            );
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
            WindowEvent::CursorMoved { position, .. } => self.cam_ctl.process_mouse_move(
                &mut self.camera,
                Vec2::new(position.x as f32, position.y as f32),
            ),
            WindowEvent::RedrawRequested => {
                let dt = (Instant::now() - self.last).as_secs_f32();
                self.last = Instant::now();
                self.cam_ctl.update_camera(&mut self.camera, dt);
                renderer.update_view(&self.camera.to_render_view());

                // update listener from camera (Y-up, forward from yaw/pitch)
                let forward = glam::Quat::from_euler(
                    glam::EulerRot::YXZ,
                    self.camera.yaw,
                    self.camera.pitch,
                    0.0,
                ) * vec3(0.0, 0.0, -1.0);
                self.audio.update_listener(ListenerPose {
                    position: self.camera.position,
                    forward,
                    up: vec3(0.0, 1.0, 0.0),
                });
                self.audio.tick(dt);

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
    event_loop
        .run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("Event loop error: {}", e))
}
