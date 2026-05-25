use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_gameplay::cutscenes::*;
use astraweave_render::Renderer;
use glam::{vec3, Vec2, Vec3};
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

        // C.7.A (Unified Camera campaign): `Cue::CameraTo` migrated from
        // yaw/pitch to look_at storage. Helper computes the equivalent
        // look_at from the original yaw/pitch values via the canonical
        // spherical-to-cartesian forward direction
        // (`forward = Vec3::new(yaw.cos() * pitch.cos(), pitch.sin(),
        //  yaw.sin() * pitch.cos())`), matching `FreeFly::dir`'s convention
        // at `astraweave-camera/src/freefly.rs:55-62`. The visual framing
        // of each cue is preserved exactly: `look_at = pos + forward`.
        let forward = |yaw: f32, pitch: f32| -> Vec3 {
            Vec3::new(
                yaw.cos() * pitch.cos(),
                pitch.sin(),
                yaw.sin() * pitch.cos(),
            )
        };
        let pos1 = vec3(0.0, 6.0, 12.0);
        let pos2 = vec3(2.0, 4.0, 8.0);
        let tl = Timeline {
            cues: vec![
                Cue::Title {
                    text: "Veilweaver".into(),
                    time: 1.5,
                },
                Cue::Wait { time: 0.5 },
                Cue::CameraTo {
                    pos: pos1,
                    look_at: pos1 + forward(-1.57, -0.35),
                    fov_deg: 60.0,
                    time: 2.0,
                },
                Cue::CameraTo {
                    pos: pos2,
                    look_at: pos2 + forward(-1.40, -0.45),
                    fov_deg: 60.0,
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

        // C.7.A bridge: inlines `apply_camera_key`-equivalent conversion
        // locally so the demo continues to function between C.7.A (this
        // sub-phase) and C.7.B. C.7.B will replace this match block with
        // `Renderer::tick_cinematics(dt, &mut self.camera)`, making the
        // demo the first production caller of the canonical cinematics
        // path. Until then, the bridge keeps the demo's runtime behavior
        // equivalent to its pre-C.7.A state: a `CameraTo` cue moves the
        // camera; other cues fall through to the free-fly controller.
        match self.cs.tick(dt, &self.tl) {
            CutsceneTickEvent::Camera(key) => {
                // Mirrors `astraweave_render::Renderer::apply_camera_key`
                // (`astraweave-render/src/renderer.rs:3371-3381`).
                let pos = vec3(key.pos.0, key.pos.1, key.pos.2);
                let look = vec3(key.look_at.0, key.look_at.1, key.look_at.2);
                let dir = (look - pos).normalize_or_zero();
                self.camera.position = pos;
                self.camera.yaw = dir.z.atan2(dir.x);
                self.camera.pitch = dir.y.clamp(-1.0, 1.0).asin();
                self.camera.fovy = key.fov_deg.to_radians();
            }
            CutsceneTickEvent::Title(_)
            | CutsceneTickEvent::Continue
            | CutsceneTickEvent::Done => {
                self.ctl.update_camera(&mut self.camera, dt);
            }
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
