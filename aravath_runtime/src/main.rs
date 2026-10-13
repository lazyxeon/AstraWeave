//! Windowed Aravath greybox.
//!
//! WASD moves the body. Mouse look orbits the third-person camera. E digs
//! a hole in front of the feet. Escape quits.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use astraweave_input::{Action, Binding, BindingSet, InputContext, InputManager};
use astraweave_render::{Instance, Renderer, SkyConfig};
use astraweave_terrain::BiomeType;
use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use aravath_runtime::combat::EnemyPose;
use aravath_runtime::figure::{actor_transform, Figure};
use aravath_runtime::ground::{ChunkPart, ChunkUpload, PropKind, PropSpot};
use aravath_runtime::props::{
    self, figurine_transform, ColoredMesh, NodeActor, Rgba, TexturedMesh,
};
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
    walk_held: bool,
    terrain: TerrainMaps,
    figure: Figure,
    sword: TexturedMesh,
    shield: TexturedMesh,
    figurine: NodeActor,
    foliage: Foliage,
    props: HashMap<(i32, i32, i32), Vec<PropSpot>>,
    foliage_ready: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            sim: {
                let mut sim = Greybox::open(aravath_runtime::sim::VIEW_RADIUS);
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
            walk_held: false,
            terrain: TerrainMaps::load(),
            figure: Figure::load(&Figure::library_path()).expect("load CC0 character"),
            sword: props::load_rigid(&props::sword_path()).expect("load CC0 sword"),
            shield: props::load_rigid(&props::shield_path()).expect("load CC0 shield"),
            figurine: NodeActor::load(&NodeActor::figurine_path()).expect("load CC0 enemy"),
            foliage: Foliage::load(),
            props: HashMap::new(),
            foliage_ready: false,
        }
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
        light_the_world(&mut renderer);
        let uploads = std::mem::take(&mut self.sim.initial_uploads);
        upload_chunks(&mut renderer, &uploads, &self.terrain);
        note_props(&mut self.props, &uploads);
        sync_foliage(
            &mut renderer,
            &self.foliage,
            &self.props,
            &mut self.foliage_ready,
        );
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
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                },
            ..
        } = &event
        {
            if matches!(code, KeyCode::AltLeft | KeyCode::AltRight) {
                self.walk_held = *state == ElementState::Pressed;
            }
            if *code == KeyCode::Tab && *state == ElementState::Pressed {
                self.lock_toggle = true;
            }
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
            walk: self.walk_held,
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
        let clip = aravath_runtime::locomotion::clip_name(self.sim.graph.state);
        let duration = self.figure.duration(clip).unwrap_or(1.0);
        let (clip, time, looping) = self.sim.player_visual(duration);
        let posed = self
            .figure
            .skin_at(clip, time, looping)
            .expect("pose character");
        let (hand_l, hand_r) = self
            .figure
            .hand_frames_at(clip, time, looping)
            .expect("hand joints");
        let player_xform = actor_transform(pos, yaw);
        let sword_xform = player_xform * props::sword_on_hand(hand_r);
        let shield_xform = player_xform * props::shield_on_hand(hand_l);

        let renderer = self.renderer.as_mut().expect("renderer");
        if let Some(uploads) = dig_uploads.as_deref() {
            upload_chunks(renderer, uploads, &self.terrain);
            note_props(&mut self.props, uploads);
        }
        upload_chunks(renderer, &streamed, &self.terrain);
        note_props(&mut self.props, &streamed);
        if dig_uploads.is_some() || !streamed.is_empty() {
            sync_foliage(
                renderer,
                &self.foliage,
                &self.props,
                &mut self.foliage_ready,
            );
        }
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
        for (index, enemy) in self.sim.combat.enemies.iter().enumerate() {
            let Some(enemy_pos) = aravath_runtime::combat::body_pos(&self.sim.physics, enemy.body)
            else {
                continue;
            };
            let to_player = pos - enemy_pos;
            let foe_yaw = to_player.z.atan2(to_player.x);
            let (foe_clip, foe_time, foe_loop) = sample_enemy(&self.figurine, enemy);
            let foe_pose = self
                .figurine
                .pose_at(foe_clip, foe_time, foe_loop)
                .expect("pose enemy");
            let instance = [Instance {
                transform: figurine_transform(enemy_pos, foe_yaw),
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: 0,
            }];
            let name = format!("enemy{index}");
            if let Some(albedo) = &self.figurine.albedo {
                renderer.add_model_with_texture(
                    name,
                    renderer.create_mesh_from_cpu_mesh(&foe_pose),
                    &instance,
                    albedo.width,
                    albedo.height,
                    &albedo.pixels,
                );
            } else {
                renderer.add_model(
                    name,
                    renderer.create_mesh_from_cpu_mesh(&foe_pose),
                    &instance,
                );
            }
        }
        if let Err(err) = renderer.render() {
            eprintln!("render error: {err:?}");
        }
        window.request_redraw();
    }
}

fn sample_enemy(
    actor: &NodeActor,
    enemy: &aravath_runtime::combat::Enemy,
) -> (&'static str, f32, bool) {
    let duration = |name| actor.duration(name).unwrap_or(1.0);
    match enemy.pose() {
        EnemyPose::Loop { clip, time } => (clip, time, true),
        EnemyPose::Fraction { clip, phase } => {
            (clip, phase.clamp(0.0, 1.0) * duration(clip), false)
        }
        EnemyPose::Hold { clip, time } => (clip, time, false),
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

fn upload_chunks(renderer: &mut Renderer, uploads: &[ChunkUpload], terrain: &TerrainMaps) {
    for chunk in uploads {
        for material in [1, 2, 3, 4, 5] {
            renderer.clear_model(&chunk_name(
                chunk.coord.x,
                chunk.coord.y,
                chunk.coord.z,
                material,
            ));
        }
        if chunk.removed {
            continue;
        }
        for part in &chunk.parts {
            upload_part(renderer, chunk, part, terrain);
        }
    }
}

fn upload_part(
    renderer: &mut Renderer,
    chunk: &ChunkUpload,
    part: &ChunkPart,
    terrain: &TerrainMaps,
) {
    if part.mesh.vertices.is_empty() {
        return;
    }
    let name = chunk_name(chunk.coord.x, chunk.coord.y, chunk.coord.z, part.material);
    let gpu = renderer.create_mesh_from_cpu_mesh(&part.mesh);
    let instance = [Instance {
        transform: Mat4::IDENTITY,
        color: [1.0, 1.0, 1.0, 1.0],
        material_id: 0,
    }];
    let albedo = terrain.for_material(part.material);
    renderer.add_model_with_texture(
        name,
        gpu,
        &instance,
        albedo.width,
        albedo.height,
        &albedo.pixels,
    );
}

fn chunk_name(x: i32, y: i32, z: i32, material: u16) -> String {
    format!("chunk_{x}_{y}_{z}_{material}")
}

struct TerrainMaps {
    stone: Rgba,
    dirt: Rgba,
    grass: Rgba,
    sand: Rgba,
    wood: Rgba,
}

impl TerrainMaps {
    fn load() -> Self {
        Self {
            stone: load_jpg("stone_diff_1k.jpg"),
            dirt: load_jpg("dirt_diff_1k.jpg"),
            grass: load_jpg("grass_diff_1k.jpg"),
            sand: load_jpg("sand_diff_1k.jpg"),
            wood: load_jpg("wood_diff_1k.jpg"),
        }
    }

    fn for_material(&self, material: u16) -> &Rgba {
        match material {
            2 => &self.dirt,
            3 => &self.grass,
            4 => &self.sand,
            5 => &self.wood,
            _ => &self.stone,
        }
    }
}

fn load_jpg(file: &str) -> Rgba {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/aravath/textures")
        .join(file);
    let image = image::open(&path)
        .unwrap_or_else(|err| panic!("load {path:?}: {err}"))
        .to_rgba8();
    let (width, height) = image.dimensions();
    Rgba {
        width,
        height,
        pixels: image.into_raw(),
    }
}

struct Foliage {
    oak: Vec<ColoredMesh>,
    pine: Vec<ColoredMesh>,
    palm: Vec<ColoredMesh>,
    rock: Vec<ColoredMesh>,
    grass: Vec<ColoredMesh>,
    bush: Vec<ColoredMesh>,
}

impl Foliage {
    fn load() -> Self {
        let path = |name: &str| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../assets/aravath/models/nature")
                .join(name)
        };
        Self {
            oak: props::load_colored(&path("tree_oak.glb")).expect("oak"),
            pine: props::load_colored(&path("tree_pineDefaultA.glb")).expect("pine"),
            palm: props::load_colored(&path("tree_palmShort.glb")).expect("palm"),
            rock: props::load_colored(&path("rock_largeA.glb")).expect("rock"),
            grass: props::load_colored(&path("grass.glb")).expect("grass"),
            bush: props::load_colored(&path("plant_bush.glb")).expect("bush"),
        }
    }

    fn parts(&self, kind: PropKind) -> &[ColoredMesh] {
        match kind {
            PropKind::Oak => &self.oak,
            PropKind::Pine => &self.pine,
            PropKind::Palm => &self.palm,
            PropKind::Rock => &self.rock,
            PropKind::Grass => &self.grass,
            PropKind::Bush => &self.bush,
        }
    }
}

fn note_props(map: &mut HashMap<(i32, i32, i32), Vec<PropSpot>>, uploads: &[ChunkUpload]) {
    for chunk in uploads {
        let key = (chunk.coord.x, chunk.coord.y, chunk.coord.z);
        if chunk.removed {
            map.remove(&key);
        } else {
            map.insert(key, chunk.props.clone());
        }
    }
}

fn sync_foliage(
    renderer: &mut Renderer,
    foliage: &Foliage,
    props: &HashMap<(i32, i32, i32), Vec<PropSpot>>,
    ready: &mut bool,
) {
    let kinds = [
        PropKind::Oak,
        PropKind::Pine,
        PropKind::Palm,
        PropKind::Rock,
        PropKind::Grass,
        PropKind::Bush,
    ];
    for kind in kinds {
        let instances: Vec<Instance> = props
            .values()
            .flatten()
            .filter(|spot| spot.kind == kind)
            .map(|spot| Instance {
                transform: Mat4::from_translation(spot.position)
                    * Mat4::from_rotation_y(spot.yaw)
                    * Mat4::from_scale(Vec3::splat(spot.scale)),
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: 0,
            })
            .collect();
        for (index, part) in foliage.parts(kind).iter().enumerate() {
            let name = format!("{kind:?}{index}");
            let colored: Vec<Instance> = instances
                .iter()
                .cloned()
                .map(|mut instance| {
                    instance.color = part.color;
                    instance
                })
                .collect();
            if !*ready {
                renderer.add_model(
                    name,
                    renderer.create_mesh_from_cpu_mesh(&part.mesh),
                    &colored,
                );
            } else {
                renderer.update_model_instances(&name, &colored);
            }
        }
    }
    *ready = true;
}

fn light_the_world(renderer: &mut Renderer) {
    renderer
        .scene_environment_mut()
        .set_biome(BiomeType::Grassland);
    {
        let env = renderer.scene_environment_mut();
        env.visuals.fog_density = 0.005;
        env.visuals.fog_color = Vec3::new(0.64, 0.76, 0.88);
        env.visuals.ambient_intensity = 0.5;
        env.sun_intensity = 2.4;
    }
    renderer.set_light_direction_override(Vec3::new(-0.35, 0.82, 0.28).normalize(), 2.4);
    let clock = renderer.time_of_day_mut();
    clock.current_time = 11.0;
    clock.time_scale = 0.0;
    renderer.set_sky_config(SkyConfig {
        cloud_coverage: 0.32,
        day_color_top: Vec3::new(0.28, 0.52, 0.92),
        day_color_horizon: Vec3::new(0.72, 0.82, 0.92),
        ..SkyConfig::default()
    });
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
