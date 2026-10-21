mod break_box;

use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_render::{Instance, Renderer};
use break_box::{fixed_to_f32, BreakStage, IntactBox};
use glam::{vec3, Mat4, Vec2, Vec3};
use std::collections::HashMap;
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
    breaks: BreakStage,
    instances: Vec<Instance>,
    char_id: u64,
    intact: Vec<Shown>,
    piece_half: HashMap<u64, Vec3>,
    water_on: bool,
    wind_on: bool,
    move_dir: Vec3,
    climb_try: bool,
    last_time: Instant,
}

struct Shown {
    sim: IntactBox,
    body: u64,
}

impl PhysicsApp {
    fn new() -> anyhow::Result<Self> {
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
        let mut breaks = BreakStage::new(1)
            .map_err(|err| anyhow::anyhow!("anchor command rejected: {err:?}"))?;
        let intact = vec![spawn_shown(
            &mut phys,
            &mut breaks,
            [-1, 1, 2],
            [1, 1, 1],
            3,
            50,
            50,
        )?];

        phys.add_water_aabb(vec3(-2.0, 0.0, -2.0), vec3(2.0, 1.2, 2.0), 1000.0, 0.8);

        Ok(Self {
            window: None,
            renderer: None,
            camera,
            cam_ctl,
            phys,
            breaks,
            instances: vec![],
            char_id,
            intact,
            piece_half: HashMap::new(),
            water_on: true,
            wind_on: false,
            move_dir: Vec3::ZERO,
            climb_try: false,
            last_time: Instant::now(),
        })
    }
}

fn spawn_shown(
    phys: &mut PhysicsWorld,
    breaks: &mut BreakStage,
    pos: [i64; 3],
    half: [i64; 3],
    mass: i64,
    health: i64,
    impulse: i64,
) -> anyhow::Result<Shown> {
    let sim = breaks
        .spawn_box(pos, half, mass, health, impulse)
        .map_err(|err| anyhow::anyhow!("spawn command rejected: {err:?}"))?;
    let body = phys.add_dynamic_box(
        vec3(pos[0] as f32, pos[1] as f32, pos[2] as f32),
        vec3(sim.half[0] as f32, sim.half[1] as f32, sim.half[2] as f32),
        mass as f32,
        Layers::DEFAULT,
    );
    Ok(Shown { sim, body })
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
                    KeyCode::KeyN if down => match spawn_shown(
                        &mut self.phys,
                        &mut self.breaks,
                        [0, 1, -1],
                        [1, 1, 1],
                        3,
                        60,
                        60,
                    ) {
                        Ok(shown) => {
                            println!(
                                "Spawned destructible health {} impulse {}",
                                shown.sim.health, shown.sim.impulse
                            );
                            self.intact.push(shown);
                        }
                        Err(err) => eprintln!("spawn rejected: {err:#}"),
                    },
                    KeyCode::KeyM if down => {
                        if let Some(shown) = self.intact.pop() {
                            match self.breaks.strike(shown.sim.node_id, shown.sim.impulse) {
                                Ok(hit) if !hit.pieces.is_empty() => {
                                    self.phys.remove_body(shown.body);
                                    for piece in hit.pieces {
                                        let pos = vec3(
                                            fixed_to_f32(piece.pos_raw[0]),
                                            fixed_to_f32(piece.pos_raw[1]),
                                            fixed_to_f32(piece.pos_raw[2]),
                                        );
                                        let half = vec3(
                                            fixed_to_f32(piece.half_raw[0]),
                                            fixed_to_f32(piece.half_raw[1]),
                                            fixed_to_f32(piece.half_raw[2]),
                                        );
                                        let mass = fixed_to_f32(piece.mass_raw).max(0.1);
                                        let id = self.phys.add_dynamic_box(
                                            pos,
                                            half,
                                            mass,
                                            Layers::DEFAULT,
                                        );
                                        self.piece_half.insert(id, half);
                                    }
                                    println!("Break command {} bytes", hit.log_bytes.len());
                                }
                                Ok(_) => {
                                    self.intact.push(shown);
                                    println!("Impact recorded; the box still holds");
                                }
                                Err(err) => {
                                    self.intact.push(shown);
                                    eprintln!("break command rejected: {err:?}");
                                }
                            }
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
                    } else if self.piece_half.contains_key(&id) {
                        [0.95, 0.45, 0.15, 1.0]
                    } else {
                        [0.8, 0.8, 0.85, 1.0]
                    };
                    let (_, rot, pos) = m.to_scale_rotation_translation();
                    let scale = self.piece_half.get(&id).copied().unwrap_or(Vec3::ONE);
                    self.instances.push(Instance {
                        transform: Mat4::from_scale_rotation_translation(scale, rot, pos),
                        color,
                        material_id: 0,
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
    let mut app = PhysicsApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
