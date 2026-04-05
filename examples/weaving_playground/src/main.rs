use astraweave_core::{IVec2, Team, World};
use astraweave_gameplay::biome::generate_island_room;
use astraweave_gameplay::*;
use astraweave_nav::{NavMesh, Triangle};
use astraweave_physics::PhysicsWorld;
use astraweave_render::TerrainRenderer as RenderTerrainRenderer; // rename to avoid conflict
use astraweave_render::{Camera, CameraController, Instance, Renderer};
use astraweave_terrain::{ChunkId, TerrainChunk, WorldConfig};
use glam::{vec3, Vec2};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct WeavingApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    cam_ctl: CameraController,
    speed_scale: f32,
    world: World,
    phys: PhysicsWorld,
    tris: Vec<Triangle>,
    terr_renderer: RenderTerrainRenderer,
    current_chunk: Option<TerrainChunk>,
    budget: WeaveBudget,
    instances: Vec<Instance>,
    last_time: Instant,
    terr_cfg: WorldConfig,
}

impl WeavingApp {
    fn new() -> Self {
        let camera = Camera {
            position: vec3(-4.0, 7.0, 12.0),
            yaw: -std::f32::consts::PI / 2.1,
            pitch: -0.55,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.01,
            zfar: 400.0,
        };
        let base_cam_speed = 3.0f32;
        let cam_ctl = CameraController::new(base_cam_speed, 0.0015);
        let speed_scale = 1.0f32;

        let mut w = World::new();
        let phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));

        let _player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
        let _comp = w.spawn("Comp", IVec2 { x: 3, y: 2 }, Team { id: 1 }, 80, 30);
        let _enemy = w.spawn("Enemy", IVec2 { x: 10, y: 2 }, Team { id: 2 }, 60, 0);

        let tris = generate_island_room();
        let _nav = NavMesh::bake(&tris, 0.5, 55.0);

        let terr_cfg = WorldConfig {
            chunk_size: 128.0,
            heightmap_resolution: 64,
            ..Default::default()
        };
        let terr_renderer = RenderTerrainRenderer::new(terr_cfg.clone());

        let budget = WeaveBudget {
            terrain_edits: 3,
            weather_ops: 2,
        };

        Self {
            window: None,
            renderer: None,
            camera,
            cam_ctl,
            speed_scale,
            world: w,
            phys,
            tris,
            terr_renderer,
            current_chunk: None,
            budget,
            instances: vec![],
            last_time: Instant::now(),
            terr_cfg,
        }
    }
}

impl ApplicationHandler for WeavingApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Weaving Playground")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let mut renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();
            renderer.time_of_day_mut().current_time = 10.0;
            renderer.time_of_day_mut().time_scale = 0.0;

            // Generate chunk and mesh
            let center_chunk_id = ChunkId::new(0, 0);
            let chunk = self
                .terr_renderer
                .world_generator_mut()
                .generate_chunk(center_chunk_id)
                .unwrap();

            let (_terrain_mesh, terrain_gpu_init) =
                build_and_upload_terrain_mesh(&mut self.terr_renderer, &chunk, &renderer).unwrap();
            renderer.set_external_mesh(terrain_gpu_init);

            self.current_chunk = Some(chunk);
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
                self.cam_ctl.process_keyboard(code, down);
                if down {
                    let mut log = |s: String| println!("{}", s);
                    match code {
                        KeyCode::BracketLeft => {
                            renderer.time_of_day_mut().current_time =
                                (renderer.time_of_day_mut().current_time - 0.5 + 24.0) % 24.0;
                            println!("Time: {:.2}h", renderer.time_of_day_mut().current_time);
                        }
                        KeyCode::BracketRight => {
                            renderer.time_of_day_mut().current_time =
                                (renderer.time_of_day_mut().current_time + 0.5) % 24.0;
                            println!("Time: {:.2}h", renderer.time_of_day_mut().current_time);
                        }
                        KeyCode::Minus => {
                            self.speed_scale = (self.speed_scale * 0.9).max(0.05);
                            self.cam_ctl.speed = 3.0 * self.speed_scale;
                            println!("Speed: {:.2}", self.cam_ctl.speed);
                        }
                        KeyCode::Equal => {
                            self.speed_scale = (self.speed_scale * 1.1).min(20.0);
                            self.cam_ctl.speed = 3.0 * self.speed_scale;
                            println!("Speed: {:.2}", self.cam_ctl.speed);
                        }
                        KeyCode::Digit1 => {
                            if let Some(ref mut current_chunk) = self.current_chunk {
                                let op = WeaveOp {
                                    kind: WeaveOpKind::ReinforcePath,
                                    a: vec3(2.0, 0.0, 2.0),
                                    b: None,
                                    budget_cost: 1,
                                };
                                if let Ok(cons) = apply_weave_op(
                                    &mut self.world,
                                    &mut self.phys,
                                    &self.tris,
                                    &mut self.budget,
                                    &op,
                                    &mut log,
                                ) {
                                    println!(
                                        "Consequence: drop x{}, faction {}",
                                        cons.drop_multiplier, cons.faction_disposition
                                    );
                                    apply_height_edit(
                                        current_chunk,
                                        op.a,
                                        3.0,
                                        1.5,
                                        self.terr_cfg.chunk_size,
                                    );
                                    match build_and_upload_terrain_mesh(
                                        &mut self.terr_renderer,
                                        current_chunk,
                                        renderer,
                                    ) {
                                        Ok((_cpu, mesh_gpu)) => {
                                            renderer.set_external_mesh(mesh_gpu)
                                        }
                                        Err(e) => eprintln!("terrain rebuild failed: {}", e),
                                    }
                                }
                            }
                        }
                        KeyCode::Digit2 => {
                            if let Some(ref mut current_chunk) = self.current_chunk {
                                let op = WeaveOp {
                                    kind: WeaveOpKind::CollapseBridge,
                                    a: vec3(1.0, 0.0, -1.0),
                                    b: Some(vec3(6.0, 0.0, -1.0)),
                                    budget_cost: 1,
                                };
                                let _ = apply_weave_op(
                                    &mut self.world,
                                    &mut self.phys,
                                    &self.tris,
                                    &mut self.budget,
                                    &op,
                                    &mut log,
                                );
                                if let Some(b) = op.b {
                                    apply_line_height_edit(
                                        current_chunk,
                                        op.a,
                                        b,
                                        2.0,
                                        -1.2,
                                        self.terr_cfg.chunk_size,
                                    );
                                    match build_and_upload_terrain_mesh(
                                        &mut self.terr_renderer,
                                        current_chunk,
                                        renderer,
                                    ) {
                                        Ok((_cpu, mesh_gpu)) => {
                                            renderer.set_external_mesh(mesh_gpu)
                                        }
                                        Err(e) => eprintln!("terrain rebuild failed: {}", e),
                                    }
                                }
                            }
                        }
                        KeyCode::Digit3 => {
                            let op = WeaveOp {
                                kind: WeaveOpKind::RedirectWind,
                                a: vec3(0.0, 0.0, 0.0),
                                b: Some(vec3(1.0, 0.0, 0.2)),
                                budget_cost: 1,
                            };
                            let mut log = |s: String| println!("{}", s);
                            let _ = apply_weave_op(
                                &mut self.world,
                                &mut self.phys,
                                &self.tris,
                                &mut self.budget,
                                &op,
                                &mut log,
                            );
                        }
                        KeyCode::Digit4 => {
                            if let Some(ref mut current_chunk) = self.current_chunk {
                                let op = WeaveOp {
                                    kind: WeaveOpKind::LowerWater,
                                    a: vec3(0.0, 0.0, 0.0),
                                    b: None,
                                    budget_cost: 1,
                                };
                                let _ = apply_weave_op(
                                    &mut self.world,
                                    &mut self.phys,
                                    &self.tris,
                                    &mut self.budget,
                                    &op,
                                    &mut log,
                                );
                                apply_height_edit(
                                    current_chunk,
                                    op.a,
                                    5.0,
                                    -1.0,
                                    self.terr_cfg.chunk_size,
                                );
                                match build_and_upload_terrain_mesh(
                                    &mut self.terr_renderer,
                                    current_chunk,
                                    renderer,
                                ) {
                                    Ok((_cpu, mesh_gpu)) => renderer.set_external_mesh(mesh_gpu),
                                    Err(e) => eprintln!("terrain rebuild failed: {}", e),
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    let pressed = state == ElementState::Pressed;
                    self.cam_ctl
                        .process_mouse_button(MouseButton::Right, pressed);
                    if pressed {
                        let _ = _window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
                        _window.set_cursor_visible(false);
                    } else {
                        let _ = _window.set_cursor_grab(winit::window::CursorGrabMode::None);
                        _window.set_cursor_visible(true);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 120.0,
                };
                self.cam_ctl.process_scroll(&mut self.camera, scroll);
            }
            WindowEvent::CursorMoved { position, .. } => {
                if !self.cam_ctl.is_dragging() {
                    self.cam_ctl.process_mouse_move(
                        &mut self.camera,
                        Vec2::new(position.x as f32, position.y as f32),
                    );
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            self.cam_ctl
                .process_mouse_delta(&mut self.camera, Vec2::new(delta.0 as f32, delta.1 as f32));
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

        self.cam_ctl.begin_frame();
        let dt_raw = (Instant::now() - self.last_time).as_secs_f32();
        let dt = dt_raw.clamp(0.0, 0.04);
        self.last_time += std::time::Duration::from_secs_f32(dt);
        self.cam_ctl.update_camera(&mut self.camera, dt);
        self.phys.step();

        self.instances.clear();
        for (x, y) in self.world.obstacles.iter() {
            self.instances.push(Instance::from_pos_scale_color(
                glam::vec3(*x as f32, 0.5, *y as f32),
                glam::vec3(0.9, 1.0, 0.9),
                [0.45, 0.45, 0.45, 1.0],
            ));
        }
        for e in self.world.all_of_team(0) {
            let p = self.world.pos_of(e).unwrap();
            self.instances.push(Instance::from_pos_scale_color(
                glam::vec3(p.x as f32, 0.5, p.y as f32),
                glam::vec3(0.7, 1.0, 0.7),
                [0.2, 0.4, 1.0, 1.0],
            ));
        }
        for e in self.world.all_of_team(1) {
            let p = self.world.pos_of(e).unwrap();
            self.instances.push(Instance::from_pos_scale_color(
                glam::vec3(p.x as f32, 0.5, p.y as f32),
                glam::vec3(0.7, 1.0, 0.7),
                [0.2, 1.0, 0.4, 1.0],
            ));
        }
        for e in self.world.all_of_team(2) {
            let p = self.world.pos_of(e).unwrap();
            self.instances.push(Instance::from_pos_scale_color(
                glam::vec3(p.x as f32, 0.5, p.y as f32),
                glam::vec3(0.7, 1.0, 0.7),
                [1.0, 0.2, 0.2, 1.0],
            ));
        }
        renderer.update_instances(&self.instances);
        renderer.update_camera(&self.camera);
        let _ = renderer.render();
        window.request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = WeavingApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

// Convert a generated terrain chunk into a renderer Mesh and upload to GPU
fn build_and_upload_terrain_mesh(
    terr_renderer: &mut RenderTerrainRenderer,
    chunk: &TerrainChunk,
    renderer: &Renderer,
) -> anyhow::Result<(
    astraweave_render::TerrainMesh,
    astraweave_render::types::Mesh,
)> {
    // Build CPU mesh using TerrainRenderer utilities
    let cpu_mesh = {
        // Recreate using internal helper: create_terrain_mesh is private, so rebuild equivalent
        // Use heightmap vertices/normals and indices from terrain crate
        let hm = chunk.heightmap();
        let res = hm.resolution();
        let step = terr_renderer.world_generator().config().chunk_size / (res - 1) as f32;
        let origin = chunk
            .id()
            .to_world_pos(terr_renderer.world_generator().config().chunk_size);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity((res * res) as usize);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity((res * res) as usize);
        let mut tangents: Vec<[f32; 4]> = Vec::with_capacity((res * res) as usize);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((res * res) as usize);
        for z in 0..res {
            for x in 0..res {
                let world_x = origin.x + x as f32 * step;
                let world_z = origin.z + z as f32 * step;
                let h = hm.get_height(x, z);
                positions.push([world_x, h, world_z]);
                let n = hm.calculate_normal(x, z, step);
                normals.push([n.x, n.y, n.z]);
                // Approx tangent along +X
                tangents.push([1.0, 0.0, 0.0, 1.0]);
                let u = x as f32 / (res - 1) as f32;
                let v = z as f32 / (res - 1) as f32;
                uvs.push([u, v]);
            }
        }
        let indices = hm.generate_indices();
        // Upload via renderer helper
        let gpu =
            renderer.create_mesh_from_full_arrays(&positions, &normals, &tangents, &uvs, &indices);
        // Return a lightweight TerrainMesh placeholder (not used by renderer directly here)
        (
            astraweave_render::TerrainMesh {
                vertices: vec![],
                indices,
                chunk_id: chunk.id(),
            },
            gpu,
        )
    };
    Ok(cpu_mesh)
}

// Apply a radial height delta around a world-space center on the given chunk
fn apply_height_edit(
    chunk: &mut TerrainChunk,
    center: glam::Vec3,
    radius: f32,
    delta: f32,
    chunk_size: f32,
) {
    let hm_res = chunk.heightmap().resolution();
    let origin = chunk.id().to_world_pos(chunk_size);
    let step = chunk_size / (hm_res - 1) as f32;
    let cx = ((center.x - origin.x) / step).floor() as i32;
    let cz = ((center.z - origin.z) / step).floor() as i32;
    let r = (radius / step).max(1.0) as i32;
    let (w, h) = (hm_res as i32, hm_res as i32);
    // Mutable access to heightmap via local copy then set back
    let mut hm = chunk.heightmap().clone();
    for dz in -r..=r {
        for dx in -r..=r {
            let x = cx + dx;
            let z = cz + dz;
            if x < 0 || x >= w || z < 0 || z >= h {
                continue;
            }
            let dist = ((dx * dx + dz * dz) as f32).sqrt();
            if dist <= r as f32 {
                let falloff = 1.0 - (dist / r as f32);
                let h0 = hm.get_height(x as u32, z as u32);
                hm.set_height(x as u32, z as u32, h0 + delta * falloff);
            }
        }
    }
    // Replace chunk's heightmap by constructing a new chunk with same id/biome
    let id = chunk.id();
    let biome = chunk.biome_map().to_vec();
    *chunk = TerrainChunk::new(id, hm, biome);
}

// Apply a line-based height delta along segment A->B on the chunk
fn apply_line_height_edit(
    chunk: &mut TerrainChunk,
    a: glam::Vec3,
    b: glam::Vec3,
    radius: f32,
    delta: f32,
    chunk_size: f32,
) {
    let hm_res = chunk.heightmap().resolution();
    let origin = chunk.id().to_world_pos(chunk_size);
    let step = chunk_size / (hm_res - 1) as f32;
    let mut hm = chunk.heightmap().clone();
    // bounds not required here; we iterate valid grid indices
    let ra = radius.max(step);
    let ra2 = ra * ra;
    // Iterate grid, move points near the infinite line segment
    for z in 0..hm_res {
        for x in 0..hm_res {
            let wx = origin.x + x as f32 * step;
            let wz = origin.z + z as f32 * step;
            let p = glam::Vec2::new(wx, wz);
            let a2 = glam::Vec2::new(a.x, a.z);
            let b2 = glam::Vec2::new(b.x, b.z);
            let ab = b2 - a2;
            let ab_len2 = ab.length_squared().max(1e-4);
            let t = ((p - a2).dot(ab) / ab_len2).clamp(0.0, 1.0);
            let proj = a2 + ab * t;
            let d2 = (p - proj).length_squared();
            if d2 <= ra2 {
                // within influence
                let falloff = 1.0 - (d2 / ra2);
                let h0 = hm.get_height(x, z);
                hm.set_height(x, z, h0 + delta * falloff);
            }
        }
    }
    let id = chunk.id();
    let biome = chunk.biome_map().to_vec();
    *chunk = TerrainChunk::new(id, hm, biome);
}
