use astraweave_core::{IVec2, Team, World};
use astraweave_render::types::SkinnedVertex;
use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_render::{Instance, Renderer};
use egui::{Context as EguiContext, Slider};
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiWinitState;
use glam::{vec3, Vec2};
use serde::Deserialize;
use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{self, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

fn world_to_instances(world: &World, scale: f32) -> Vec<Instance> {
    let mut v = Vec::new();
    for (x, y) in world.obstacles.iter() {
        let pos = vec3(*x as f32 * scale, 0.5, *y as f32 * scale);
        v.push(Instance::from_pos_scale_color(
            pos,
            vec3(0.9, 1.0, 0.9) * 0.9,
            [0.6, 0.6, 0.7, 1.0],
        ));
    }
    for e in world.all_of_team(0) {
        let p = world.pos_of(e).unwrap();
        v.push(Instance::from_pos_scale_color(
            vec3(p.x as f32 * scale, 0.5, p.y as f32 * scale),
            vec3(0.7, 1.0, 0.7),
            [0.2, 0.4, 1.0, 1.0],
        ));
    }
    for e in world.all_of_team(1) {
        let p = world.pos_of(e).unwrap();
        v.push(Instance::from_pos_scale_color(
            vec3(p.x as f32 * scale, 0.5, p.y as f32 * scale),
            vec3(0.7, 1.0, 0.7),
            [0.2, 1.0, 0.4, 1.0],
        ));
    }
    for e in world.all_of_team(2) {
        let p = world.pos_of(e).unwrap();
        v.push(Instance::from_pos_scale_color(
            vec3(p.x as f32 * scale, 0.5, p.y as f32 * scale),
            vec3(0.7, 1.0, 0.7),
            [1.0, 0.2, 0.2, 1.0],
        ));
    }
    v
}

fn validate_textures() -> anyhow::Result<()> {
    #[cfg(feature = "textures")]
    {
        let texture_files = [
            "assets/grass.png",
            "assets/dirt.png",
            "assets/stone.png",
            "assets/grass_n.png",
            "assets/dirt_n.png",
            "assets/stone_n.png",
            "assets/default_n.png",
        ];
        astraweave_render::texture::validate_texture_assets(&texture_files)
    }
    #[cfg(not(feature = "textures"))]
    {
        println!("🎨 Texture validation skipped (textures feature not enabled)");
        println!("✅ Visual demo will use basic colored primitives");
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MaterialLiveDoc {
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    texture_path: Option<String>,
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    camera_controller: CameraController,
    last_frame: Instant,
    world: World,
    grid_scale: f32,

    // Egui state
    egui_ctx: EguiContext,
    egui_state: Option<EguiWinitState>,
    egui_renderer: Option<EguiRenderer>,

    // UI state
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    use_checkerboard: bool,
    last_checkerboard: bool,
    load_mesh: bool,
    mesh_loaded: bool,
    skinned_demo: bool,
    skinned_setup_done: bool,
    load_skinned_gltf: bool,
    skinned_gltf_loaded: bool,
    skinned_gltf_clip: Option<(Vec<f32>, Vec<[f32; 4]>)>,
    skinned_gltf_joint_count: usize,
}

impl App {
    fn new() -> Self {
        let world = {
            let mut world = World::new();
            for y in 1..=8 {
                world.obstacles.insert((6, y));
            }
            let _player = world.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
            let _comp = world.spawn("Companion", IVec2 { x: 3, y: 2 }, Team { id: 1 }, 80, 30);
            let _enemy = world.spawn("Enemy", IVec2 { x: 12, y: 2 }, Team { id: 2 }, 60, 0);
            world
        };

        let camera = Camera {
            position: vec3(0.0, 8.0, 12.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.6,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 200.0,
        };

        Self {
            window: None,
            renderer: None,
            camera,
            camera_controller: CameraController::new(4.0, 0.4),
            last_frame: Instant::now(),
            world,
            grid_scale: 1.5,
            egui_ctx: EguiContext::default(),
            egui_state: None,
            egui_renderer: None,
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.1,
            roughness: 0.6,
            use_checkerboard: false,
            last_checkerboard: false,
            load_mesh: false,
            mesh_loaded: false,
            skinned_demo: false,
            skinned_setup_done: false,
            load_skinned_gltf: false,
            skinned_gltf_loaded: false,
            skinned_gltf_clip: None,
            skinned_gltf_joint_count: 0,
        }
    }

    fn update_scene(&mut self) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.update_instances(&world_to_instances(&self.world, self.grid_scale));
            renderer.update_view(&self.camera.to_render_view());
            renderer.set_material_params(self.base_color, self.metallic, self.roughness);
        }
    }

    fn render(&mut self) {
        // Update UI logic first before borrowing window
        self.handle_ui_logic();

        // Now get window reference
        let window = self.window.as_ref().unwrap();

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.camera_controller.update_camera(&mut self.camera, dt);

        // Prepare Egui input and UI declaration
        let egui_state = self.egui_state.as_mut().unwrap();
        let raw_input = egui_state.take_egui_input(window);
        let egui_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("AstraWeave Visual 3D Controls");
                    if ui.button("Reset Time").clicked() {
                        if let Some(renderer) = self.renderer.as_mut() {
                            renderer.time_of_day_mut().current_time = 12.0;
                        }
                    }
                });
            });
            egui::Window::new("Environment").show(ctx, |ui| {
                if let Some(renderer) = self.renderer.as_mut() {
                    let mut hour = renderer.time_of_day_mut().current_time;
                    let mut sky_cfg = renderer.sky_config();
                    ui.add(Slider::new(&mut hour, 0.0..=24.0).text("Hour"));
                    ui.add(Slider::new(&mut sky_cfg.cloud_coverage, 0.0..=1.0).text("Clouds"));
                    ui.add(Slider::new(&mut sky_cfg.cloud_speed, 0.0..=5.0).text("Cloud Speed"));
                    renderer.time_of_day_mut().current_time = hour;
                    renderer.set_sky_config(sky_cfg);
                }
            });
            egui::Window::new("Material").show(ctx, |ui| {
                ui.label("Base Color (RGB)");
                ui.add(Slider::new(&mut self.base_color[0], 0.0..=1.0).text("R"));
                ui.add(Slider::new(&mut self.base_color[1], 0.0..=1.0).text("G"));
                ui.add(Slider::new(&mut self.base_color[2], 0.0..=1.0).text("B"));
                ui.add(Slider::new(&mut self.metallic, 0.0..=1.0).text("Metallic"));
                ui.add(Slider::new(&mut self.roughness, 0.04..=1.0).text("Roughness"));
                ui.checkbox(&mut self.use_checkerboard, "Checkerboard Albedo");
                ui.separator();
                ui.checkbox(&mut self.load_mesh, "Load demo glTF plane");
                ui.separator();
                ui.checkbox(&mut self.skinned_demo, "Skinned quad (idle) demo");
                ui.separator();
                ui.checkbox(&mut self.load_skinned_gltf, "Load skinned glTF (idle)");
            });
        });

        egui_state.handle_platform_output(window, egui_output.platform_output.clone());

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.tick_environment(dt);
            renderer.update_view(&self.camera.to_render_view());
            renderer.set_material_params(self.base_color, self.metallic, self.roughness);

            // Simply call render() - it handles the 3D scene
            // TODO: Integrate egui rendering properly when the render API supports it
            let _ = renderer.render();
        }
    }

    fn handle_ui_logic(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        if self.use_checkerboard != self.last_checkerboard {
            if self.use_checkerboard {
                // Generate a simple 256x256 checkerboard (8x8 tiles)
                let w: u32 = 256;
                let h: u32 = 256;
                let tile: u32 = 32;
                let mut data = vec![0u8; (w * h * 4) as usize];
                for y in 0..h {
                    for x in 0..w {
                        let tx = (x / tile) % 2;
                        let ty = (y / tile) % 2;
                        let c: u8 = if (tx ^ ty) == 0 { 220 } else { 40 };
                        let idx = ((y * w + x) * 4) as usize;
                        data[idx] = c;
                        data[idx + 1] = c;
                        data[idx + 2] = c;
                        data[idx + 3] = 255;
                    }
                }
                renderer.set_albedo_from_rgba8(w, h, &data);
            } else {
                renderer.set_albedo_from_rgba8(1, 1, &[255, 255, 255, 255]);
            }
            self.last_checkerboard = self.use_checkerboard;
        }

        if self.load_mesh && !self.mesh_loaded {
            use astraweave_asset::gltf_loader as gl;
            if let Ok(bytes) = std::fs::read("assets/demo_plane.gltf") {
                if let Ok((mesh, mat)) = gl::load_first_mesh_and_material(&bytes) {
                    // Use full vertex data (positions, normals, tangents, uvs)
                    let gpu_mesh = renderer.create_mesh_from_full_arrays(
                        &mesh.positions,
                        &mesh.normals,
                        &mesh.tangents,
                        &mesh.texcoords,
                        &mesh.indices,
                    );
                    renderer.set_external_mesh(gpu_mesh);
                    renderer.set_material_params(
                        mat.base_color_factor,
                        mat.metallic_factor,
                        mat.roughness_factor,
                    );
                    if let Some(img) = mat.base_color_texture {
                        renderer.set_albedo_from_rgba8(img.width, img.height, &img.rgba8);
                    }
                    if let Some(img) = mat.metallic_roughness_texture {
                        renderer
                            .set_metallic_roughness_from_rgba8(img.width, img.height, &img.rgba8);
                    }
                    if let Some(img) = mat.normal_texture {
                        renderer.set_normal_from_rgba8(img.width, img.height, &img.rgba8);
                    }
                    self.mesh_loaded = true;
                } else {
                    eprintln!("Failed to parse demo_plane.gltf");
                }
            } else {
                eprintln!("Missing assets/demo_plane.gltf");
            }
        }

        // Simple skinned quad demo: a 2-triangle plane with 1 bone influencing all verts
        if self.skinned_demo && !self.skinned_setup_done {
            // Build a unit quad in XZ plane centered at origin
            let verts = [
                SkinnedVertex {
                    position: [-0.5, 0.0, -0.5],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                    joints: [0, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                },
                SkinnedVertex {
                    position: [0.5, 0.0, -0.5],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [1.0, 0.0],
                    joints: [0, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                },
                SkinnedVertex {
                    position: [0.5, 0.0, 0.5],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [1.0, 1.0],
                    joints: [0, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                },
                SkinnedVertex {
                    position: [-0.5, 0.0, 0.5],
                    normal: [0.0, 1.0, 0.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                    joints: [0, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                },
            ];
            let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
            renderer.set_skinned_mesh(&verts, &indices);
            // Set a colored material for visibility
            renderer.set_material_params([0.9, 0.2, 0.2, 1.0], 0.0, 0.8);
            self.skinned_setup_done = true;
        }

        if self.skinned_demo && self.skinned_setup_done {
            let t = self.last_frame.elapsed().as_secs_f32();
            let angle = (t * 1.2).sin() * 0.3; // radians
            let rot = glam::Mat4::from_rotation_z(angle);
            let trans = glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0));
            let m = trans * rot; // simple idle motion
            renderer.update_skin_palette(&[m]);
        }

        // Load a skinned glTF (first skinned node) and an optional idle clip
        if self.load_skinned_gltf && !self.skinned_gltf_loaded {
            use astraweave_asset::gltf_loader as gl;
            let path = "assets/skinned_demo.gltf"; // expected to be placed by user in assets/
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok((mesh, _skeleton, animations, mat_opt)) =
                    gl::load_skinned_mesh_complete(&bytes)
                {
                    // Convert to renderer format
                    let verts: Vec<SkinnedVertex> = mesh
                        .vertices
                        .into_iter()
                        .map(|v| SkinnedVertex {
                            position: v.position,
                            normal: v.normal,
                            tangent: v.tangent,
                            uv: v.uv,
                            joints: v.joints,
                            weights: v.weights,
                        })
                        .collect();
                    renderer.set_skinned_mesh(&verts, &mesh.indices);
                    if let Some(mat) = mat_opt {
                        renderer.set_material_params(
                            mat.base_color_factor,
                            mat.metallic_factor,
                            mat.roughness_factor,
                        );
                        if let Some(img) = mat.base_color_texture {
                            renderer.set_albedo_from_rgba8(img.width, img.height, &img.rgba8);
                        }
                        if let Some(img) = mat.metallic_roughness_texture {
                            renderer.set_metallic_roughness_from_rgba8(
                                img.width, img.height, &img.rgba8,
                            );
                        }
                        if let Some(img) = mat.normal_texture {
                            renderer.set_normal_from_rgba8(img.width, img.height, &img.rgba8);
                        }
                    }
                    // Default palette: identity for all joints
                    self.skinned_gltf_joint_count = mesh.joint_count as usize;
                    let mut mats = Vec::with_capacity(self.skinned_gltf_joint_count);
                    for _ in 0..self.skinned_gltf_joint_count {
                        mats.push(glam::Mat4::IDENTITY);
                    }
                    renderer.update_skin_palette(&mats);
                    // Store clip if present - extract first rotation channel from first animation
                    if let Some(clip) = animations.first() {
                        // Find first rotation channel
                        if let Some(channel) = clip
                            .channels
                            .iter()
                            .find(|ch| matches!(ch.data, gl::ChannelData::Rotation(_)))
                        {
                            if let gl::ChannelData::Rotation(ref rotations) = channel.data {
                                self.skinned_gltf_clip =
                                    Some((channel.times.clone(), rotations.clone()));
                            }
                        }
                    }
                    self.skinned_gltf_loaded = true;
                } else {
                    eprintln!("Failed to parse skinned glTF: {path}");
                }
            } else {
                eprintln!("Missing {path}");
            }
        }

        // Animate first joint from clip if present
        if self.skinned_gltf_loaded {
            if let Some((times, rotations)) = &self.skinned_gltf_clip {
                if !times.is_empty() && !rotations.is_empty() {
                    // Loop time in clip range
                    let t = self.last_frame.elapsed().as_secs_f32();
                    let max_t = *times.last().unwrap();
                    let lt = (t % max_t.max(0.0001)).max(0.0);
                    // Find the first time >= lt
                    let mut idx = 0usize;
                    while idx + 1 < times.len() && times[idx + 1] < lt {
                        idx += 1;
                    }
                    let i0 = idx;
                    let i1 = (idx + 1).min(times.len() - 1);
                    let t0 = times[i0];
                    let t1 = times[i1].max(t0 + 1e-5);
                    let a = ((lt - t0) / (t1 - t0)).clamp(0.0, 1.0);
                    let q0 = glam::Quat::from_xyzw(
                        rotations[i0][0],
                        rotations[i0][1],
                        rotations[i0][2],
                        rotations[i0][3],
                    );
                    let q1 = glam::Quat::from_xyzw(
                        rotations[i1][0],
                        rotations[i1][1],
                        rotations[i1][2],
                        rotations[i1][3],
                    );
                    let q = q0.slerp(q1, a);
                    // Build palette: rotate first joint; rest identity
                    let mut mats = Vec::with_capacity(self.skinned_gltf_joint_count.max(1));
                    if self.skinned_gltf_joint_count == 0 {
                        mats.push(glam::Mat4::from_quat(q));
                    } else {
                        mats.push(glam::Mat4::from_quat(q));
                        for _ in 1..self.skinned_gltf_joint_count {
                            mats.push(glam::Mat4::IDENTITY);
                        }
                    }
                    renderer.update_skin_palette(&mats);
                }
            }
        }

        // Apply live material if available
        if let Ok(txt) = std::fs::read_to_string("assets/material_live.json") {
            if let Ok(doc) = serde_json::from_str::<MaterialLiveDoc>(&txt) {
                renderer.set_material_params(doc.base_color, doc.metallic, doc.roughness);
                if let Some(path) = doc.texture_path {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            let img = img.to_rgba8();
                            let (w, h) = img.dimensions();
                            renderer.set_albedo_from_rgba8(w, h, &img);
                        }
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("AstraWeave Visual 3D")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();

            // Correct arguments for egui-winit v0.32
            self.egui_state = Some(EguiWinitState::new(
                self.egui_ctx.clone(),
                egui::ViewportId::default(),
                &window,
                None, // initial_pixels_per_point
                None, // theme
                Some(renderer.device().limits().max_texture_dimension_2d as usize), // max_texture_side
            ));
            // Correct arguments for egui-wgpu v0.32
            self.egui_renderer = Some(EguiRenderer::new(
                renderer.device(),
                renderer.surface_format(),
                None,  // depth_format
                1,     // msaa_samples
                false, // prefer_srgb
            ));

            self.renderer = Some(renderer);
            self.update_scene();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.is_none() {
            return;
        }
        let window = self.window.as_ref().unwrap();
        let egui_state = self.egui_state.as_mut().unwrap();

        let _ = egui_state.on_window_event(window, &event);
        match event {
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                self.camera.aspect = (size.width as f32 / size.height.max(1) as f32).max(0.1);
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                if key_code == KeyCode::Escape {
                    event_loop.exit();
                }
                self.camera_controller
                    .process_keyboard(key_code, state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.camera_controller
                    .process_mouse_button(button, state == ElementState::Pressed);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.camera_controller.process_mouse_move(
                    &mut self.camera,
                    Vec2::new(position.x as f32, position.y as f32),
                );
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            self.window.as_ref().unwrap().request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    println!("🎮 AstraWeave Visual 3D Demo - with UI controls");
    validate_textures()?;

    let event_loop = EventLoop::new()?;
    let mut app = App::new();

    event_loop.run_app(&mut app)?;
    Ok(())
}
