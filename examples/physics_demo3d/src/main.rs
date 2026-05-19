use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_render::{Camera, CameraController, CameraProducer, Instance, Renderer};
use glam::{vec3, Vec2, Vec3};
use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct PhysicsApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    cam_ctl: CameraController,
    phys: PhysicsWorld,
    instances: Vec<Instance>,
    char_id: u64,
    destruct_ids: Vec<u64>,
    water_on: bool,
    wind_on: bool,
    move_dir: Vec3,
    climb_try: bool,
    last_time: Instant,
}

impl PhysicsApp {
    fn new() -> Self {
        let camera = Camera {
            position: vec3(0.0, 8.0, 16.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.45,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 500.0,
        };
        let cam_ctl = CameraController::new(10.0, 0.005);

        let mut phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
        let _ground = phys.create_ground_plane(vec3(100.0, 0.0, 100.0), 1.0);

        let _wall = phys.add_static_trimesh(
            &[
                vec3(5.0, 0.0, 0.0),
                vec3(5.0, 3.0, 0.0),
                vec3(5.0, 0.0, 3.0),
                vec3(5.0, 3.0, 3.0),
                vec3(5.0, 0.0, 3.0),
                vec3(5.0, 3.0, 0.0),
            ],
            &[[0, 1, 2], [3, 2, 1]],
            Layers::CHARACTER | Layers::DEFAULT,
        );

        let char_id = phys.add_character(vec3(-2.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

        let destruct_ids: Vec<u64> = vec![phys.add_destructible_box(
            vec3(-1.0, 1.0, 2.0),
            vec3(0.4, 0.4, 0.4),
            3.0,
            50.0,
            12.0,
        )];

        phys.add_water_aabb(vec3(-2.0, 0.0, -2.0), vec3(2.0, 1.2, 2.0), 1000.0, 0.8);

        Self {
            window: None,
            renderer: None,
            camera,
            cam_ctl,
            phys,
            instances: vec![],
            char_id,
            destruct_ids,
            water_on: true,
            wind_on: false,
            move_dir: Vec3::ZERO,
            climb_try: false,
            last_time: Instant::now(),
        }
    }
}

impl ApplicationHandler for PhysicsApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("AstraWeave Physics Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            let renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();
            self.renderer = Some(renderer);
            self.last_time = Instant::now();
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
                let down = state == ElementState::Pressed;
                match code {
                    KeyCode::KeyW => self.cam_ctl.process_keyboard(code, down),
                    KeyCode::KeyS => self.cam_ctl.process_keyboard(code, down),
                    KeyCode::KeyA => self.cam_ctl.process_keyboard(code, down),
                    KeyCode::KeyD => self.cam_ctl.process_keyboard(code, down),
                    KeyCode::Space => self.cam_ctl.process_keyboard(code, down),
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                        self.cam_ctl.process_keyboard(code, down)
                    }
                    KeyCode::KeyJ => {
                        if down {
                            self.move_dir.x = -2.5;
                        } else {
                            self.move_dir.x = 0.0;
                        }
                    }
                    KeyCode::KeyL => {
                        if down {
                            self.move_dir.x = 2.5;
                        } else {
                            self.move_dir.x = 0.0;
                        }
                    }
                    KeyCode::KeyI => {
                        if down {
                            self.move_dir.z = -2.5;
                        } else {
                            self.move_dir.z = 0.0;
                        }
                    }
                    KeyCode::KeyK => {
                        if down {
                            self.move_dir.z = 2.5;
                        } else {
                            self.move_dir.z = 0.0;
                        }
                    }
                    KeyCode::KeyC => {
                        self.climb_try = down;
                    }
                    KeyCode::KeyT if down => {
                        self.wind_on = !self.wind_on;
                        if self.wind_on {
                            self.phys.set_wind(vec3(1.0, 0.0, 0.2).normalize(), 8.0);
                        } else {
                            self.phys.set_wind(vec3(0.0, 0.0, 0.0), 0.0);
                        }
                        println!("Wind: {}", if self.wind_on { "ON" } else { "OFF" });
                    }
                    KeyCode::KeyG if down => {
                        self.water_on = !self.water_on;
                        if self.water_on {
                            self.phys.add_water_aabb(
                                vec3(-2.0, 0.0, -2.0),
                                vec3(2.0, 1.2, 2.0),
                                1000.0,
                                0.8,
                            );
                        } else {
                            self.phys.clear_water();
                        }
                        println!("Water: {}", if self.water_on { "ON" } else { "OFF" });
                    }
                    KeyCode::KeyF if down => {
                        self.phys.add_dynamic_box(
                            vec3(0.0, 4.0, 0.0),
                            vec3(0.3, 0.3, 0.3),
                            1.0,
                            Layers::DEFAULT,
                        );
                    }
                    KeyCode::KeyB if down => {
                        let _rag = self.phys.add_dynamic_box(
                            vec3(0.0, 1.2, -1.5),
                            vec3(0.2, 0.5, 0.2),
                            70.0,
                            Layers::DEFAULT,
                        );
                        println!("Spawned ragdoll (box placeholder)");
                    }
                    KeyCode::KeyN if down => {
                        let id = self.phys.add_destructible_box(
                            vec3(-0.5, 1.0, -1.0),
                            vec3(0.4, 0.4, 0.4),
                            3.0,
                            60.0,
                            14.0,
                        );
                        self.destruct_ids.push(id);
                        println!("Spawned destructible");
                    }
                    KeyCode::KeyM if down => {
                        if let Some(id) = self.destruct_ids.pop() {
                            self.phys.break_destructible(id);
                            println!("Break destructible");
                        }
                    }
                    _ => {}
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

        let now = Instant::now();
        let dt = (now - self.last_time).as_secs_f32();
        self.last_time = now;

        self.cam_ctl.update_camera(&mut self.camera, dt);

        let desired = vec3(self.move_dir.x, 0.0, self.move_dir.z);
        self.phys
            .control_character(self.char_id, desired, dt, self.climb_try);
        self.phys.step();

        self.instances.clear();
        for (handle, _body) in self.phys.bodies.iter() {
            if let Some(id) = self.phys.id_of(handle) {
                if let Some(m) = self.phys.body_transform(id) {
                    let color = if self.phys.char_map.contains_key(&id) {
                        [0.2, 1.0, 0.4, 1.0]
                    } else {
                        [0.8, 0.8, 0.85, 1.0]
                    };
                    self.instances.push(astraweave_render::Instance {
                        transform: m,
                        color,
                        material_id: 0, // Added material_id
                    });
                }
            }
        }

        renderer.update_view(&self.camera.to_render_view());
        renderer.update_instances(&self.instances);
        if let Err(e) = renderer.render() {
            eprintln!("render error: {e:?}");
        }

        window.request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = PhysicsApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
