//! Windowed Aravath greybox.
//!
//! WASD moves the body. Mouse look orbits the third-person camera. E digs
//! a hole in front of the feet. Escape quits.

use std::sync::Arc;
use std::time::Instant;

use astraweave_input::{Action, Binding, BindingSet, InputContext, InputManager};
use astraweave_render::{Instance, Renderer};
use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use aravath_runtime::ground::{material_tint, ChunkUpload};
use aravath_runtime::Greybox;

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    sim: Greybox,
    input: InputManager,
    last_cursor: Option<(f32, f32)>,
    last_time: Instant,
    player_mesh_ready: bool,
    lock_toggle: bool,
    place_toggle: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            sim: {
                let mut sim = Greybox::island();
                sim.spawn_enemy(Vec3::new(30.0, 14.0, 22.0));
                sim.spawn_enemy(Vec3::new(18.0, 14.0, 30.0));
                sim.spawn_enemy(Vec3::new(34.0, 14.0, 30.0));
                sim
            },
            input: demo_input(),
            last_cursor: None,
            last_time: Instant::now(),
            player_mesh_ready: false,
            lock_toggle: false,
            place_toggle: false,
        }
    }
}

fn chunk_name(x: i32, y: i32, z: i32) -> String {
    format!("chunk_{x}_{y}_{z}")
}

fn box_mesh(hx: f32, hy: f32, hz: f32) -> astraweave_render::mesh::CpuMesh {
    unit_box_scaled(hx, hy, hz)
}

fn unit_box() -> astraweave_render::mesh::CpuMesh {
    unit_box_scaled(0.35, 0.9, 0.35)
}

fn unit_box_scaled(hx: f32, hy: f32, hz: f32) -> astraweave_render::mesh::CpuMesh {
    let corners = [
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(-hx, hy, -hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
    ];
    let faces: [[u32; 6]; 6] = [
        [0, 1, 2, 0, 2, 3],
        [5, 4, 7, 5, 7, 6],
        [4, 0, 3, 4, 3, 7],
        [1, 5, 6, 1, 6, 2],
        [3, 2, 6, 3, 6, 7],
        [4, 5, 1, 4, 1, 0],
    ];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for face in faces {
        let a = corners[face[0] as usize];
        let b = corners[face[1] as usize];
        let c = corners[face[2] as usize];
        let n = (b - a).cross(c - a).normalize_or_zero();
        let helper = if n.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let tangent = n.cross(helper).normalize_or_zero();
        let base = vertices.len() as u32;
        for (offset, corner) in face.into_iter().enumerate() {
            let p = corners[corner as usize];
            vertices.push(astraweave_render::mesh::MeshVertex::new(
                p,
                n,
                glam::Vec4::new(tangent.x, tangent.y, tangent.z, 1.0),
                glam::Vec2::ZERO,
            ));
            indices.push(base + offset as u32);
        }
    }
    astraweave_render::mesh::CpuMesh {
        vertices,
        indices,
        albedo_image: None,
        texture_source_hint: None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Aravath greybox")
                        .with_inner_size(PhysicalSize::new(1280, 720)),
                )
                .expect("create window"),
        );
        let mut renderer = pollster::block_on(Renderer::new(window.clone())).expect("renderer");
        let uploads = std::mem::take(&mut self.sim.initial_uploads);
        upload_chunks(&mut renderer, &uploads);
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.last_time = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = &event
        {
            event_loop.exit();
            return;
        }

        if let WindowEvent::CursorMoved { position, .. } = &event {
            let x = position.x as f32;
            let y = position.y as f32;
            if let Some((px, py)) = self.last_cursor {
                self.sim.camera.apply_look(x - px, y - py, 0.005);
            }
            self.last_cursor = Some((x, y));
        }

        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Tab),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = &event
        {
            self.lock_toggle = true;
        }
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::KeyR),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = &event
        {
            self.place_toggle = true;
        }
        self.input.process_window_event(&event);

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        if let WindowEvent::Resized(size) = &event {
            renderer.resize(size.width, size.height);
            self.sim.camera.aspect = size.width as f32 / size.height.max(1) as f32;
        }
        if let WindowEvent::CloseRequested = &event {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() || self.window.is_none() {
            return;
        }
        let window = self.window.clone().expect("window");

        let now = Instant::now();
        let dt = (now - self.last_time).as_secs_f32().min(0.05);
        self.last_time = now;
        let steps = ((dt / aravath_runtime::sim::FIXED_DT).round() as u32).clamp(1, 4);
        let input = aravath_runtime::motor::MotorInput {
            move_x: axis(
                self.input.is_down(Action::MoveRight),
                self.input.is_down(Action::MoveLeft),
            ),
            move_y: axis(
                self.input.is_down(Action::MoveForward),
                self.input.is_down(Action::MoveBackward),
            ),
            jump: self.input.just_pressed(Action::Jump),
            sprint: self.input.is_down(Action::Sprint),
            crouch: self.input.is_down(Action::Crouch),
            roll: self.input.just_pressed(Action::Ability1),
            fly_toggle: self.input.just_pressed(Action::Ability2),
            feet_locked: false,
        };
        if self.input.just_pressed(Action::AttackLight) {
            self.sim.combat.attack.start();
        }
        self.sim.combat.blocking = self.input.is_down(Action::AttackHeavy);
        if self.lock_toggle {
            self.sim
                .combat
                .toggle_lock(&self.sim.physics, &self.sim.player);
            self.lock_toggle = false;
        }
        for step in 0..steps {
            let mut step_input = input;
            if step > 0 {
                step_input.jump = false;
                step_input.roll = false;
                step_input.fly_toggle = false;
            }
            self.sim.tick_controls(step_input);
        }
        let dig_uploads = if self.input.just_pressed(Action::Interact) {
            Some(self.sim.dig_ahead(2.5))
        } else if self.place_toggle {
            self.place_toggle = false;
            Some(self.sim.place_ahead(1.1))
        } else {
            None
        };
        self.input.clear_frame();

        let distance = self.sim.occluded_camera_distance();
        let view = self.sim.camera.to_render_view_at(distance);
        let pos = self.sim.player_pos();
        let yaw = self.sim.player.facing_yaw;
        let foes = enemy_instances(&self.sim);
        let spawn_player = !self.player_mesh_ready;
        self.player_mesh_ready = true;

        let renderer = self.renderer.as_mut().expect("renderer");
        if let Some(uploads) = dig_uploads.as_deref() {
            upload_chunks(renderer, uploads);
        }
        renderer.update_view(&view);
        let instance = [Instance {
            transform: Mat4::from_translation(pos),
            color: [0.85, 0.55, 0.25, 1.0],
            material_id: 0,
        }];
        if spawn_player {
            let mesh = renderer.create_mesh_from_cpu_mesh(&unit_box());
            renderer.add_model("player", mesh, &instance);
            let sword = renderer.create_mesh_from_cpu_mesh(&box_mesh(0.08, 0.5, 0.08));
            renderer.add_model(
                "sword",
                sword,
                &[Instance {
                    transform: Mat4::IDENTITY,
                    color: [0.75, 0.78, 0.82, 1.0],
                    material_id: 0,
                }],
            );
            let shield = renderer.create_mesh_from_cpu_mesh(&box_mesh(0.4, 0.5, 0.06));
            renderer.add_model(
                "shield",
                shield,
                &[Instance {
                    transform: Mat4::IDENTITY,
                    color: [0.45, 0.28, 0.16, 1.0],
                    material_id: 0,
                }],
            );
            let foe = renderer.create_mesh_from_cpu_mesh(&unit_box());
            renderer.add_model("enemies", foe, &foes);
        } else {
            renderer.update_model_instances("player", &instance);
            renderer.update_model_instances("sword", &[weapon_instance(pos, yaw, 0.9, 0.2)]);
            renderer.update_model_instances("shield", &[weapon_instance(pos, yaw, -0.45, 0.1)]);
            renderer.update_model_instances("enemies", &foes);
        }
        if let Err(err) = renderer.render() {
            eprintln!("render error: {err:?}");
        }
        window.request_redraw();
    }
}

fn upload_chunks(renderer: &mut Renderer, uploads: &[ChunkUpload]) {
    for chunk in uploads {
        let name = chunk_name(chunk.coord.x, chunk.coord.y, chunk.coord.z);
        if chunk.removed || chunk.mesh.vertices.is_empty() {
            renderer.clear_model(&name);
            continue;
        }
        let gpu = renderer.create_mesh_from_cpu_mesh(&chunk.mesh);
        renderer.add_model(
            name,
            gpu,
            &[Instance {
                transform: Mat4::IDENTITY,
                color: material_tint(chunk.material),
                material_id: 0,
            }],
        );
    }
}

fn weapon_instance(pos: Vec3, yaw: f32, side: f32, forward: f32) -> Instance {
    let (fwd, right) = aravath_runtime::player::flat_basis(yaw);
    Instance {
        transform: Mat4::from_translation(pos + right * side + fwd * forward + Vec3::Y * 0.2),
        color: [0.8, 0.8, 0.85, 1.0],
        material_id: 0,
    }
}

fn enemy_instances(sim: &Greybox) -> Vec<Instance> {
    sim.combat
        .enemies
        .iter()
        .filter(|enemy| enemy.alive())
        .filter_map(|enemy| aravath_runtime::combat::body_pos(&sim.physics, enemy.body))
        .map(|pos| Instance {
            transform: Mat4::from_translation(pos),
            color: [0.75, 0.2, 0.2, 1.0],
            material_id: 0,
        })
        .collect()
}

fn demo_input() -> InputManager {
    let mut bindings = BindingSet::default();
    bindings.actions.insert(
        Action::Ability1,
        Binding {
            key: Some(winit::keyboard::KeyCode::KeyQ),
            ..Binding::default()
        },
    );
    bindings.actions.insert(
        Action::Ability2,
        Binding {
            key: Some(winit::keyboard::KeyCode::KeyF),
            ..Binding::default()
        },
    );
    InputManager::new(InputContext::Gameplay, bindings)
}

fn axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
