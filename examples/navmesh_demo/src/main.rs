use astraweave_nav::{NavMesh, Triangle};
use astraweave_render::{Camera, CameraController, CameraProducer, Instance, Renderer};
use glam::{vec3, Vec2};
use std::sync::Arc;
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
    instances: Vec<Instance>,
}

impl App {
    fn new() -> Self {
        let camera = Camera {
            position: vec3(0.0, 10.0, 16.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.5,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 500.0,
        };
        let cam_ctl = CameraController::new(10.0, 0.005);

        // Make a small "heightfield" of walkable tris with a ramp:
        let tris = vec![
            tri(
                vec3(-4.0, 0.0, -4.0),
                vec3(4.0, 0.0, -4.0),
                vec3(4.0, 0.0, 4.0),
            ),
            tri(
                vec3(-4.0, 0.0, -4.0),
                vec3(4.0, 0.0, 4.0),
                vec3(-4.0, 0.0, 4.0),
            ),
            // ramp up
            tri(
                vec3(4.0, 0.0, -1.0),
                vec3(8.0, 0.8, -1.0),
                vec3(8.0, 0.8, 1.0),
            ),
            tri(
                vec3(4.0, 0.0, -1.0),
                vec3(8.0, 0.8, 1.0),
                vec3(4.0, 0.0, 1.0),
            ),
            // plateau
            tri(
                vec3(8.0, 0.8, -1.0),
                vec3(12.0, 0.8, -1.0),
                vec3(12.0, 0.8, 1.0),
            ),
            tri(
                vec3(8.0, 0.8, -1.0),
                vec3(12.0, 0.8, 1.0),
                vec3(8.0, 0.8, 1.0),
            ),
        ];
        let nav = NavMesh::bake(&tris, 0.4, 50.0); // 50° slope allowed

        let start = vec3(-3.5, 0.0, -3.5);
        let goal = vec3(11.5, 0.8, 0.0);
        let path = nav.find_path(start, goal);

        let mut instances = vec![];

        // visualize tri centers
        for t in &nav.tris {
            instances.push(Instance::from_pos_scale_color(
                t.center + vec3(0.0, 0.05, 0.0),
                vec3(0.1, 0.1, 0.1),
                [0.7, 0.7, 0.3, 1.0],
            ));
        }
        // visualize path
        for p in &path {
            instances.push(Instance::from_pos_scale_color(
                *p + vec3(0.0, 0.08, 0.0),
                vec3(0.12, 0.12, 0.12),
                [0.2, 1.0, 0.4, 1.0],
            ));
        }

        Self {
            window: None,
            renderer: None,
            camera,
            cam_ctl,
            instances,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("NavMesh Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let mut renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();
            renderer.update_instances(&self.instances);
            renderer.update_view(&self.camera.to_render_view());
            self.renderer = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.is_none() {
            return;
        }
        match event {
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                if code == KeyCode::Escape {
                    event_loop.exit();
                }
                self.cam_ctl
                    .process_keyboard(code, state == ElementState::Pressed);
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
                if let Some(renderer) = self.renderer.as_mut() {
                    self.cam_ctl.update_camera(&mut self.camera, 1.0 / 60.0);
                    renderer.update_view(&self.camera.to_render_view());
                    if let Err(e) = renderer.render() {
                        eprintln!("{e:?}");
                    }
                }
                self.window.as_ref().unwrap().request_redraw();
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
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[inline]
fn tri(a: glam::Vec3, b: glam::Vec3, c: glam::Vec3) -> Triangle {
    Triangle { a, b, c }
}
