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

use aravath_runtime::figure::{actor_transform, Figure};
use aravath_runtime::ground::{material_tint, ChunkUpload};
use aravath_runtime::props::{self, figurine_transform, NodeActor, TexturedMesh};
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
    albedo: Option<(u32, u32, Vec<u8>)>,
    figure: Figure,
    sword: TexturedMesh,
    shield: TexturedMesh,
    figurine: NodeActor,
    foe_ready: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            sim: {
                let mut sim = Greybox::open(1);
                sim.spawn_enemy(Vec3::new(28.0, 16.0, 16.0));
                sim.spawn_enemy(Vec3::new(10.0, 16.0, 28.0));
                sim.spawn_enemy(Vec3::new(24.0, 16.0, 30.0));
                sim
            },
            input: demo_input(),
            last_cursor: None,
            last_time: Instant::now(),
            player_mesh_ready: false,
            lock_toggle: false,
            place_toggle: false,
            albedo: load_ground_albedo(),
            figure: Figure::load(&Figure::library_path()).expect("load CC0 character"),
            sword: props::load_rigid(&props::sword_path()).expect("load CC0 sword"),
            shield: props::load_rigid(&props::shield_path()).expect("load CC0 shield"),
            figurine: NodeActor::load(&NodeActor::figurine_path()).expect("load CC0 enemy"),
            foe_ready: false,
        }
    }
}

fn chunk_name(x: i32, y: i32, z: i32) -> String {
    format!("chunk_{x}_{y}_{z}")
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
        upload_chunks(&mut renderer, &uploads, self.albedo.as_ref());
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
        let streamed = std::mem::take(&mut self.sim.pending_uploads);
        self.input.clear_frame();

        let distance = self.sim.occluded_camera_distance();
        let view = self.sim.camera.to_render_view_at(distance);
        let pos = self.sim.player_pos();
        let yaw = self.sim.player.facing_yaw;
        let posed = self.figure.skin("Idle_Loop", 0.0).expect("pose character");
        let (hand_l, hand_r) = self
            .figure
            .hand_frames("Idle_Loop", 0.0)
            .expect("hand joints");
        let player_xform = actor_transform(pos, yaw);
        let mut foe_xforms = Vec::new();
        for enemy in &self.sim.combat.enemies {
            if !enemy.alive() {
                continue;
            }
            if let Some(enemy_pos) =
                aravath_runtime::combat::body_pos(&self.sim.physics, enemy.body)
            {
                let to_player = pos - enemy_pos;
                let yaw = to_player.z.atan2(to_player.x);
                foe_xforms.push(figurine_transform(enemy_pos, yaw));
            }
        }
        let sword_xform = player_xform * props::sword_on_hand(hand_r);
        let shield_xform = player_xform * props::shield_on_hand(hand_l);

        let albedo = self.albedo.clone();
        let renderer = self.renderer.as_mut().expect("renderer");
        if let Some(uploads) = dig_uploads.as_deref() {
            upload_chunks(renderer, uploads, albedo.as_ref());
        }
        upload_chunks(renderer, &streamed, albedo.as_ref());
        renderer.update_view(&view);
        upload_posed(
            renderer,
            "player",
            &posed,
            player_xform,
            [1.0, 1.0, 1.0, 1.0],
        );
        let sword = [Instance {
            transform: sword_xform,
            color: [1.0, 1.0, 1.0, 1.0],
            material_id: 0,
        }];
        let shield = [Instance {
            transform: shield_xform,
            color: [1.0, 1.0, 1.0, 1.0],
            material_id: 0,
        }];
        if !self.player_mesh_ready {
            upload_textured(renderer, "sword", &self.sword, &sword);
            upload_textured(renderer, "shield", &self.shield, &shield);
            self.player_mesh_ready = true;
        } else {
            renderer.update_model_instances("sword", &sword);
            renderer.update_model_instances("shield", &shield);
        }
        let foe_pose = self.figurine.pose("idle", 0.0).expect("pose enemy");
        let foe_instances: Vec<Instance> = foe_xforms
            .iter()
            .map(|transform| Instance {
                transform: *transform,
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: 0,
            })
            .collect();
        if foe_instances.is_empty() {
            renderer.clear_model("enemies");
            self.foe_ready = false;
        } else if !self.foe_ready {
            if let Some(albedo) = &self.figurine.albedo {
                renderer.add_model_with_texture(
                    "enemies",
                    renderer.create_mesh_from_cpu_mesh(&foe_pose),
                    &foe_instances,
                    albedo.width,
                    albedo.height,
                    &albedo.pixels,
                );
            }
            self.foe_ready = true;
        } else {
            renderer.update_model_instances("enemies", &foe_instances);
        }
        if let Err(err) = renderer.render() {
            eprintln!("render error: {err:?}");
        }
        window.request_redraw();
    }
}

fn upload_textured(
    renderer: &mut Renderer,
    name: &str,
    mesh: &TexturedMesh,
    instances: &[Instance],
) {
    let gpu = renderer.create_mesh_from_cpu_mesh(&mesh.mesh);
    if let Some(albedo) = &mesh.albedo {
        renderer.add_model_with_texture(
            name,
            gpu,
            instances,
            albedo.width,
            albedo.height,
            &albedo.pixels,
        );
    } else {
        renderer.add_model(name, gpu, instances);
    }
}

fn upload_posed(
    renderer: &mut Renderer,
    name: &str,
    posed: &[aravath_runtime::figure::PosedPrimitive],
    transform: Mat4,
    tint: [f32; 4],
) {
    for (index, primitive) in posed.iter().enumerate() {
        let gpu = renderer.create_mesh_from_cpu_mesh(&primitive.mesh);
        let color = [
            primitive.color[0] * tint[0],
            primitive.color[1] * tint[1],
            primitive.color[2] * tint[2],
            1.0,
        ];
        renderer.add_model(
            format!("{name}_{index}"),
            gpu,
            &[Instance {
                transform,
                color,
                material_id: 0,
            }],
        );
    }
}

fn upload_chunks(
    renderer: &mut Renderer,
    uploads: &[ChunkUpload],
    albedo: Option<&(u32, u32, Vec<u8>)>,
) {
    for chunk in uploads {
        let name = chunk_name(chunk.coord.x, chunk.coord.y, chunk.coord.z);
        if chunk.removed || chunk.mesh.vertices.is_empty() {
            renderer.clear_model(&name);
            continue;
        }
        let gpu = renderer.create_mesh_from_cpu_mesh(&chunk.mesh);
        let instance = [Instance {
            transform: Mat4::IDENTITY,
            color: material_tint(chunk.material),
            material_id: 0,
        }];
        if let Some((width, height, pixels)) = albedo {
            renderer.add_model_with_texture(name, gpu, &instance, *width, *height, pixels);
        } else {
            renderer.add_model(name, gpu, &instance);
        }
    }
}

fn load_ground_albedo() -> Option<(u32, u32, Vec<u8>)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/aravath/rock_wall_02_diff_1k.jpg");
    let image = image::open(path).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some((width, height, image.into_raw()))
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
