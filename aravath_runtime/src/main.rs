//! Windowed Aravath greybox.
//!
//! WASD moves the body. Mouse look orbits the third-person camera. E digs
//! a hole in front of the feet. Escape quits.

mod startup;

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
use aravath_runtime::hud::{self, HudModel};
use aravath_runtime::props::{
    self, figurine_transform, ColoredMesh, NodeActor, Rgba, TexturedMesh,
};
use aravath_runtime::sim::{FrameLatch, TickInput};
use aravath_runtime::Greybox;

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    sim: Greybox,
    input: InputManager,
    last_cursor: Option<(f32, f32)>,
    look_dx: f32,
    look_dy: f32,
    latch: FrameLatch,
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
    selected: u16,
    notice: Option<(String, f32)>,
    save_toggle: bool,
    load_toggle: bool,
    respawn_toggle: bool,
    hud: Option<EguiHud>,
    volumetrics: bool,
}

impl App {
    fn new(volumetrics: bool) -> anyhow::Result<Self> {
        startup::stage("mesh world");
        let sim = {
            let mut sim = Greybox::open(aravath_runtime::sim::VIEW_RADIUS);
            sim.spawn_enemy(Vec3::new(28.0, 16.0, 16.0));
            sim.spawn_enemy(Vec3::new(10.0, 16.0, 28.0));
            sim.spawn_enemy(Vec3::new(24.0, 16.0, 30.0));
            sim
        };
        startup::stage("world meshed");
        startup::stage("load assets");
        Ok(Self {
            window: None,
            renderer: None,
            sim,
            input: demo_input(),
            last_cursor: None,
            look_dx: 0.0,
            look_dy: 0.0,
            latch: FrameLatch::default(),
            last_time: Instant::now(),
            player_mesh_ready: false,
            lock_toggle: false,
            place_toggle: false,
            walk_held: false,
            terrain: TerrainMaps::load()?,
            figure: Figure::load(&Figure::library_path())?,
            sword: props::load_rigid(&props::sword_path())?,
            shield: props::load_rigid(&props::shield_path())?,
            figurine: NodeActor::load(&NodeActor::figurine_path())?,
            foliage: Foliage::load()?,
            props: HashMap::new(),
            foliage_ready: false,
            selected: 1,
            notice: None,
            save_toggle: false,
            load_toggle: false,
            respawn_toggle: false,
            hud: None,
            volumetrics,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Aravath")
                .with_inner_size(PhysicalSize::new(1280, 720)),
        ) {
            Ok(window) => Arc::new(window),
            Err(err) => startup::fatal("Aravath", &format!("Could not open a window: {err}")),
        };
        startup::paint_loading(&window, "Loading Aravath… compiling shaders");
        startup::stage("window shown");
        let mut renderer = match pollster::block_on(Renderer::new(window.clone())) {
            Ok(renderer) => renderer,
            Err(err) => startup::fatal("Aravath", &format!("Renderer failed to start:\n{err:#}")),
        };
        startup::stage("renderer ready");
        light_the_world(&mut renderer, self.volumetrics);
        startup::stage("lighting configured");
        let uploads = std::mem::take(&mut self.sim.initial_uploads);
        upload_chunks(&mut renderer, &uploads, &self.terrain);
        note_props(&mut self.props, &uploads);
        sync_foliage(
            &mut renderer,
            &self.foliage,
            &self.props,
            &mut self.foliage_ready,
        );
        let ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::default(),
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(renderer.device(), renderer.surface_format(), None, 1, false);
        self.hud = Some(EguiHud {
            ctx,
            state: egui_state,
            renderer: egui_renderer,
        });
        capture_mouse(&window);
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

        if let (Some(hud), Some(window)) = (self.hud.as_mut(), self.window.as_ref()) {
            let _ = hud.state.on_window_event(window, &event);
        }

        if let WindowEvent::CursorMoved { position, .. } = &event {
            let x = position.x as f32;
            let y = position.y as f32;
            if let Some((px, py)) = self.last_cursor {
                self.look_dx += x - px;
                self.look_dy += y - py;
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
            if *state == ElementState::Pressed {
                match code {
                    KeyCode::Tab => self.lock_toggle = true,
                    KeyCode::F5 => self.save_toggle = true,
                    KeyCode::F9 => self.load_toggle = true,
                    KeyCode::Enter => self.respawn_toggle = true,
                    KeyCode::Digit1 => self.selected = 1,
                    KeyCode::Digit2 => self.selected = 2,
                    KeyCode::Digit3 => self.selected = 3,
                    KeyCode::Digit4 => self.selected = 4,
                    KeyCode::Digit5 => self.selected = 5,
                    _ => {}
                }
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
        let frame_dt = (now - self.last_time).as_secs_f32();
        self.last_time = now;
        let present_dt = frame_dt.clamp(0.0, 0.05);
        if let Some((_, left)) = self.notice.as_mut() {
            *left -= present_dt;
        }
        if self.notice.as_ref().is_some_and(|(_, left)| *left <= 0.0) {
            self.notice = None;
        }
        let lock_on = self.lock_toggle;
        self.lock_toggle = false;
        let place = self.place_toggle;
        self.place_toggle = false;
        let respawn = self.respawn_toggle;
        self.respawn_toggle = false;
        self.latch.sample(TickInput {
            motor: aravath_runtime::motor::MotorInput {
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
            },
            attack: self.input.just_pressed(Action::AttackLight),
            block: self.input.is_down(Action::AttackHeavy),
            lock_on,
            dig: self.input.just_pressed(Action::Interact),
            place,
            place_material: self.selected,
            look_dx: self.look_dx,
            look_dy: self.look_dy,
            respawn,
        });
        self.look_dx = 0.0;
        self.look_dy = 0.0;
        let caught = self.latch.catch_up(&mut self.sim, frame_dt);
        if caught.respawned {
            self.notice = Some(("Respawned".into(), 2.5));
        }
        if caught.steps > 0 && self.save_toggle {
            self.save_toggle = false;
            self.notice = Some(
                match aravath_runtime::save::save_checkpoint(&save_root(), &self.sim.checkpoint()) {
                    Ok(()) => ("Saved".into(), 2.5),
                    Err(err) => (format!("Save failed: {err}"), 3.5),
                },
            );
        }
        let restore_uploads = if caught.steps > 0 && self.load_toggle {
            self.load_toggle = false;
            match aravath_runtime::save::load_checkpoint(&save_root()) {
                Ok(checkpoint) => {
                    self.notice = Some(("Loaded".into(), 2.5));
                    Some(self.sim.restore(checkpoint))
                }
                Err(err) => {
                    self.notice = Some((format!("Load failed: {err}"), 3.5));
                    None
                }
            }
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
        upload_chunks(renderer, &streamed, &self.terrain);
        note_props(&mut self.props, &streamed);
        if let Some(uploads) = restore_uploads.as_deref() {
            upload_chunks(renderer, uploads, &self.terrain);
            note_props(&mut self.props, uploads);
        }
        if !streamed.is_empty() || restore_uploads.is_some() {
            sync_foliage(
                renderer,
                &self.foliage,
                &self.props,
                &mut self.foliage_ready,
            );
        }
        renderer.tick_environment(present_dt);
        renderer.update_view(&view);
        if self.volumetrics {
            let focus = aravath_runtime::combat::world_metres(pos);
            let baked = self.sim.scars.bake_gpu_volume(focus);
            let volume = astraweave_render::ScarFogVolume {
                origin_metres: baked.origin_metres,
                extent_metres: baked.extent_metres,
                dims: baked.dims,
                rgba: baked.rgba,
                any: baked.any,
            };
            if let Err(err) = renderer.upload_veil_scar_volume(&volume) {
                startup::line(&format!("scar upload failed: {err:#}"));
            }
        }
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
            let foe_yaw = aravath_runtime::detmath::atan2(to_player.z, to_player.x);
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
        let model = HudModel {
            health: self.sim.combat.player_hp,
            health_max: 100.0,
            stamina: self.sim.motor.stamina,
            stamina_max: self.sim.motor.stamina_max,
            locked: self.sim.combat.lock.is_some(),
            block: hud::block_name(self.selected),
            notice: self.notice.as_ref().map(|(text, _)| text.clone()),
            downed: self.sim.combat.player_hp <= 0.0,
        };
        let mut hud = self.hud.take();
        let scale = window.scale_factor() as f32;
        if let Some(renderer) = self.renderer.as_mut() {
            if renderer.is_device_lost() {
                startup::fatal(
                    "Aravath",
                    "The GPU device was lost. Details are in aravath.log next to the exe.",
                );
            }
            if let Err(err) = renderer.render_with(|view, encoder, device, queue, size| {
                if let Some(hud) = hud.as_mut() {
                    paint_hud(
                        hud,
                        HudFrame {
                            window: &window,
                            view,
                            encoder,
                            device,
                            queue,
                            size,
                            scale,
                            model: &model,
                        },
                    );
                }
            }) {
                startup::line(&format!("render error: {err:#}"));
                if renderer.is_device_lost() {
                    startup::fatal(
                        "Aravath",
                        &format!("Render failed and the GPU device was lost:\n{err:#}"),
                    );
                }
            }
        }
        self.hud = hud;
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
    fn load() -> anyhow::Result<Self> {
        Ok(Self {
            stone: load_jpg("stone_diff_1k.jpg")?,
            dirt: load_jpg("dirt_diff_1k.jpg")?,
            grass: load_jpg("grass_diff_1k.jpg")?,
            sand: load_jpg("sand_diff_1k.jpg")?,
            wood: load_jpg("wood_diff_1k.jpg")?,
        })
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

fn load_jpg(file: &str) -> anyhow::Result<Rgba> {
    let (width, height, pixels) = aravath_runtime::assets::read_texture(file)?;
    Ok(Rgba {
        width,
        height,
        pixels,
    })
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
    fn load() -> anyhow::Result<Self> {
        let path = |name: &str| {
            aravath_runtime::assets::asset_dir()
                .join("models/nature")
                .join(name)
        };
        Ok(Self {
            oak: props::load_colored(&path("tree_oak.glb"))?,
            pine: props::load_colored(&path("tree_pineDefaultA.glb"))?,
            palm: props::load_colored(&path("tree_palmShort.glb"))?,
            rock: props::load_colored(&path("rock_largeA.glb"))?,
            grass: props::load_colored(&path("grass.glb"))?,
            bush: props::load_colored(&path("plant_bush.glb"))?,
        })
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

struct EguiHud {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

fn capture_mouse(window: &Window) {
    window.set_cursor_visible(false);
    match window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
        Ok(()) => {}
        Err(locked) => match window.set_cursor_grab(winit::window::CursorGrabMode::Confined) {
            Ok(()) => {}
            Err(confined) => {
                eprintln!("cursor stays free: locked {locked}; confined {confined}");
            }
        },
    }
}

fn save_root() -> std::path::PathBuf {
    aravath_runtime::assets::save_dir()
}

struct HudFrame<'a> {
    window: &'a Window,
    view: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    size: (u32, u32),
    scale: f32,
    model: &'a HudModel,
}

fn paint_hud(hud: &mut EguiHud, frame: HudFrame<'_>) {
    let HudFrame {
        window,
        view,
        encoder,
        device,
        queue,
        size,
        scale,
        model,
    } = frame;
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [size.0, size.1],
        pixels_per_point: scale,
    };
    let input = hud.state.take_egui_input(window);
    hud.ctx.begin_pass(input);
    egui::Area::new(egui::Id::new("vitals"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -16.0))
        .show(&hud.ctx, |ui| {
            ui.label(format!("Health {:.0}", model.health.max(0.0)));
            ui.add(
                egui::ProgressBar::new(hud::health_fraction(model))
                    .fill(egui::Color32::from_rgb(176, 48, 42)),
            );
            ui.label(format!("Stamina {:.0}", model.stamina.max(0.0)));
            ui.add(
                egui::ProgressBar::new(hud::stamina_fraction(model))
                    .fill(egui::Color32::from_rgb(46, 140, 72)),
            );
        });
    egui::Area::new(egui::Id::new("block"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
        .show(&hud.ctx, |ui| {
            ui.label(format!("Block: {}  (1-5)", model.block));
        });
    if let Some(hint) = hud::respawn_hint(model.downed) {
        egui::Area::new(egui::Id::new("downed"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 48.0))
            .show(&hud.ctx, |ui| {
                ui.label(hint);
            });
    }
    if let Some(text) = &model.notice {
        egui::Area::new(egui::Id::new("notice"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 24.0))
            .show(&hud.ctx, |ui| {
                ui.label(text);
            });
    }
    let center = egui::pos2(size.0 as f32 / scale * 0.5, size.1 as f32 / scale * 0.5);
    let painter = hud.ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("reticle"),
    ));
    let color = if model.locked {
        egui::Color32::from_rgb(220, 70, 60)
    } else {
        egui::Color32::from_rgba_premultiplied(255, 255, 255, 180)
    };
    let radius = if model.locked { 16.0 } else { 6.0 };
    painter.circle_stroke(center, radius, egui::Stroke::new(2.0, color));
    if model.locked {
        painter.text(
            center + egui::vec2(22.0, -8.0),
            egui::Align2::LEFT_CENTER,
            hud::reticle_label(true),
            egui::FontId::proportional(14.0),
            color,
        );
    }
    let output = hud.ctx.end_pass();
    let clipped = hud.ctx.tessellate(output.shapes, output.pixels_per_point);
    for (id, image) in &output.textures_delta.set {
        hud.renderer.update_texture(device, queue, *id, image);
    }
    let _ = hud
        .renderer
        .update_buffers(device, queue, encoder, &clipped, &screen);
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aravath hud"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // SAFETY: egui-wgpu 0.32 takes a 'static render pass. The pass is not
        // used after this call, and it does not outlive `encoder`.
        let pass: &mut wgpu::RenderPass<'static> = unsafe { std::mem::transmute(&mut pass) };
        hud.renderer.render(pass, &clipped, &screen);
    }
    for id in &output.textures_delta.free {
        hud.renderer.free_texture(id);
    }
}

fn light_the_world(renderer: &mut Renderer, volumetrics: bool) {
    renderer.set_shadows_enabled(true);
    renderer.set_cloud_shadows_enabled(false);
    renderer
        .scene_environment_mut()
        .set_biome(BiomeType::Grassland);
    {
        let env = renderer.scene_environment_mut();
        env.visuals.fog_density = 0.0012;
        env.visuals.fog_color = Vec3::new(0.64, 0.74, 0.86);
        env.visuals.ambient_intensity = 0.45;
    }
    let clock = renderer.time_of_day_mut();
    clock.current_time = 16.4;
    clock.time_scale = 90.0;
    renderer.set_sky_config(SkyConfig {
        cloud_coverage: 0.28,
        day_color_top: Vec3::new(0.25, 0.48, 0.92),
        day_color_horizon: Vec3::new(0.95, 0.62, 0.38),
        ..SkyConfig::default()
    });
    if !volumetrics {
        startup::line("volumetrics off (--no-volumetrics, --safe, or aravath.cfg)");
        return;
    }
    startup::stage("build volumetrics");
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.set_volumetric_fog_config(
            astraweave_render::volumetric_fog::VolumetricFogPass::gameplay_config(),
        );
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: true,
            num_samples: 32,
            exposure: 0.22,
            ..astraweave_render::GodRayConfig::default()
        });
    }));
    if built.is_err() {
        startup::line(
            "volumetrics panicked during setup; continuing without fog and light shafts. Relaunch with --no-volumetrics.",
        );
    } else {
        startup::stage("volumetrics ready");
    }
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

struct Launch {
    smoke: bool,
    volumetrics: bool,
}

fn launch_options() -> Launch {
    let mut volumetrics = true;
    let cfg = startup::exe_dir().join("aravath.cfg");
    match aravath_runtime::assets::read_volumetrics_cfg(&cfg) {
        Ok(Some(enabled)) => {
            startup::line(&format!("read {}", cfg.display()));
            volumetrics = enabled;
        }
        Ok(None) => {}
        Err(err) => startup::fatal("Aravath config", &format!("{err:#}")),
    }
    let mut smoke = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--smoke-test" => smoke = true,
            "--no-volumetrics" | "--safe" => volumetrics = false,
            "--volumetrics" => volumetrics = true,
            other => startup::line(&format!(
                "ignored argument {}",
                aravath_runtime::assets::bounded_text(other, 80)
            )),
        }
    }
    Launch { smoke, volumetrics }
}

fn smoke_test(volumetrics: bool) -> anyhow::Result<()> {
    startup::stage("verify assets");
    let root = aravath_runtime::assets::verify()?;
    startup::line(&format!("assets {}", root.display()));
    startup::stage("load models");
    let _figure = Figure::load(&Figure::library_path())?;
    let _sword = props::load_rigid(&props::sword_path())?;
    let _shield = props::load_rigid(&props::shield_path())?;
    let _figurine = NodeActor::load(&NodeActor::figurine_path())?;
    let _foliage = Foliage::load()?;
    let _terrain = TerrainMaps::load()?;
    startup::stage("mesh one chunk");
    let _sim = Greybox::open(0);
    std::env::set_var("ARAVATH_WARP", "1");
    startup::stage("headless renderer");
    let mut renderer = pollster::block_on(Renderer::new_headless(64, 64))?;
    startup::stage("headless renderer ready");
    if volumetrics {
        startup::stage("volumetrics on warp");
        renderer.set_volumetric_fog_config(
            astraweave_render::volumetric_fog::VolumetricFogPass::gameplay_config(),
        );
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: true,
            num_samples: 32,
            exposure: 0.22,
            ..astraweave_render::GodRayConfig::default()
        });
        startup::stage("volumetrics ready");
    }
    startup::stage("smoke ok");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    startup::install();
    let launch = launch_options();
    startup::line(&format!(
        "exe {} volumetrics {}",
        std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "(unknown)".into()),
        launch.volumetrics
    ));
    if let Err(err) = aravath_runtime::assets::verify() {
        startup::fatal("Aravath assets", &format!("{err:#}"));
    }
    startup::stage("assets verified");
    if launch.smoke {
        return smoke_test(launch.volumetrics);
    }
    let event_loop = EventLoop::new()?;
    let mut app = App::new(launch.volumetrics)?;
    startup::stage("enter event loop");
    event_loop.run_app(&mut app)?;
    Ok(())
}
