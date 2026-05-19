use astraweave_gameplay::cutscenes::*;
use astraweave_render::{Camera, CameraController, CameraProducer, Renderer};
use glam::{vec3, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

struct CutsceneApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    ctl: CameraController,
    tl: Timeline,
    cs: CutsceneState,
    t: f32,
    last: Instant,
}

impl CutsceneApp {
    fn new() -> Self {
        let camera = Camera {
            position: vec3(-3.0, 5.0, 10.0),
            yaw: -1.57,
            pitch: -0.4,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 500.0,
        };
        let ctl = CameraController::new(10.0, 0.005);

        let tl = Timeline {
            cues: vec![
                Cue::Title {
                    text: "Veilweaver".into(),
                    time: 1.5,
                },
                Cue::Wait { time: 0.5 },
                Cue::CameraTo {
                    pos: vec3(0.0, 6.0, 12.0),
                    yaw: -1.57,
                    pitch: -0.35,
                    time: 2.0,
                },
                Cue::CameraTo {
                    pos: vec3(2.0, 4.0, 8.0),
                    yaw: -1.40,
                    pitch: -0.45,
                    time: 2.0,
                },
            ],
        };
        let cs = CutsceneState::new();

        Self {
            window: None,
            renderer: None,
            camera,
            ctl,
            tl,
            cs,
            t: 0.0,
            last: Instant::now(),
        }
    }
}

impl ApplicationHandler for CutsceneApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Cutscene Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            let renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();
            self.renderer = Some(renderer);
            self.last = Instant::now();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };
        let _window = match self.window.as_ref() {
            Some(w) => w,
            None => return,
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
                self.ctl
                    .process_keyboard(code, state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.ctl
                        .process_mouse_button(MouseButton::Right, state == ElementState::Pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.ctl.process_mouse_move(
                &mut self.camera,
                Vec2::new(position.x as f32, position.y as f32),
            ),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };
        let window = match self.window.as_ref() {
            Some(w) => w,
            None => return,
        };

        let dt = (Instant::now() - self.last).as_secs_f32();
        self.last = Instant::now();
        self.t += dt;

        let (cam, _title, _done) = self.cs.tick(dt, &self.tl);
        if let Some((pos, yaw, pitch)) = cam {
            self.camera.position = pos;
            self.camera.yaw = yaw;
            self.camera.pitch = pitch;
        } else {
            self.ctl.update_camera(&mut self.camera, dt);
        }

        renderer.update_view(&self.camera.to_render_view());
        if let Err(e) = renderer.render() {
            eprintln!("{e:?}");
        }
        window.request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CutsceneApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
