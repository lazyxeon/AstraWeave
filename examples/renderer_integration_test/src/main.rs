use anyhow::Result;
use astraweave_camera::{CameraProducer, FreeFly as Camera};
use astraweave_render::Renderer;
use glam::Vec3;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct App {
    renderer: Renderer,
    camera: Camera,
    // Input state
    movement: [bool; 6], // W, A, S, D, Space, Shift
    mouse_pressed: bool,
    mouse_delta: (f32, f32),
}

impl App {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let mut renderer = Renderer::new(window.clone()).await?;

        // TRIGGER VISUAL SMOKE TEST
        renderer.set_smoke_test_texture("assets/test_texture.png");

        // Initialize camera
        let size = window.inner_size();
        let camera = Camera {
            position: Vec3::new(0.0, 5.0, 10.0),
            yaw: -90.0_f32.to_radians(),
            pitch: -20.0_f32.to_radians(),
            fovy: 45.0_f32.to_radians(),
            aspect: size.width as f32 / size.height as f32,
            znear: 0.1,
            zfar: 100.0,
        };

        Ok(Self {
            renderer,
            camera,
            movement: [false; 6],
            mouse_pressed: false,
            mouse_delta: (0.0, 0.0),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.renderer.resize(new_size.width, new_size.height);
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
    }

    fn update(&mut self) {
        // Update camera
        let speed = 0.1;
        let forward = Camera::dir(self.camera.yaw, self.camera.pitch);
        let right = forward.cross(Vec3::Y).normalize();

        if self.movement[0] {
            self.camera.position += forward * speed;
        } // W
        if self.movement[2] {
            self.camera.position -= forward * speed;
        } // S
        if self.movement[1] {
            self.camera.position -= right * speed;
        } // A
        if self.movement[3] {
            self.camera.position += right * speed;
        } // D
        if self.movement[4] {
            self.camera.position.y += speed;
        } // Space
        if self.movement[5] {
            self.camera.position.y -= speed;
        } // Shift

        // Mouse look
        if self.mouse_pressed {
            let sensitivity = 0.003;
            self.camera.yaw += self.mouse_delta.0 * sensitivity;
            self.camera.pitch -= self.mouse_delta.1 * sensitivity;
            self.camera.pitch = self
                .camera
                .pitch
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }
        self.mouse_delta = (0.0, 0.0);

        // Update renderer camera
        self.renderer.update_view(&self.camera.to_render_view());

        // Update environment (sky, weather)
        self.renderer.tick_environment(0.016); // Fixed dt for now
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        match self.renderer.render() {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Render error: {:?}", e);
                // Map anyhow error to SurfaceError if possible, or just return Lost
                Err(wgpu::SurfaceError::Lost)
            }
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW => self.movement[0] = pressed,
                    KeyCode::KeyA => self.movement[1] = pressed,
                    KeyCode::KeyS => self.movement[2] = pressed,
                    KeyCode::KeyD => self.movement[3] = pressed,
                    KeyCode::Space => self.movement[4] = pressed,
                    KeyCode::ShiftLeft => self.movement[5] = pressed,
                    _ => return false,
                }
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    self.mouse_pressed = *state == ElementState::Pressed;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn mouse_motion(&mut self, delta: (f64, f64)) {
        self.mouse_delta.0 += delta.0 as f32;
        self.mouse_delta.1 += delta.1 as f32;
    }
}

struct TestApp {
    app: Option<App>,
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for TestApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Renderer Integration Test")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            match pollster::block_on(App::new(window)) {
                Ok(app) => {
                    self.app = Some(app);
                }
                Err(e) => {
                    eprintln!("Failed to create app: {}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window.as_ref().map(|w| w.id()) {
            return;
        }

        if let Some(app) = &mut self.app {
            if !app.input(&event) {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::Resized(physical_size) => app.resize(physical_size),
                    WindowEvent::RedrawRequested => {
                        app.update();
                        if let Err(wgpu::SurfaceError::OutOfMemory) = app.render() {
                            event_loop.exit();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(app) = &mut self.app {
                app.mouse_motion(delta);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut test_app = TestApp {
        app: None,
        window: None,
    };

    event_loop.run_app(&mut test_app)?;

    Ok(())
}
