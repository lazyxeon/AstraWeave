//! Veilweaver Visual Renderer — Full 3D windowed mode
//!
//! Renders the Veilweaver game simulation in a real-time 3D window:
//! - Procedural island terrain with heightmap generation + grass texture
//! - Animated ocean with Gerstner waves
//! - HDRI skybox (kloppenheim pure-sky for clean horizon)
//! - Player, companion, and enemy entities as colored spheres
//! - egui HUD overlay: HP bars, zone info, companion status, event log, telemetry
//! - WASD + mouse camera controls
//!
//! Usage: `cargo run -p veilweaver_demo --release --features visual`

use anyhow::{Context, Result};
use glam::{vec3, Vec2, Vec3};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use astraweave_ecs::App;
use astraweave_llm::qwen3_ollama::Qwen3Ollama;
use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_render::{Instance, Mesh, Renderer, Vertex, WaterRenderer};

use crate::telemetry_hud::{TelemetryHud, TelemetryMetrics};
use crate::{Faction, GameState, Health, Position};

// ═══════════════════════════════════════════════════════════════
//  Terrain system — procedural island with heightmap noise
// ═══════════════════════════════════════════════════════════════

struct TerrainSystem {
    island_center: (f32, f32),
    island_radius: f32,
    max_height: f32,
    resolution: u32,
    heightmap: Vec<f32>,
}

impl TerrainSystem {
    fn new() -> Self {
        let resolution = 256;
        let mut ts = Self {
            island_center: (0.0, 0.0),
            island_radius: 150.0,
            max_height: 30.0,
            resolution,
            heightmap: vec![0.0; (resolution * resolution) as usize],
        };
        ts.generate_heightmap();
        ts
    }

    fn generate_heightmap(&mut self) {
        let (cx, cz) = self.island_center;
        let half_size = self.island_radius * 1.05;
        let step = (half_size * 2.0) / self.resolution as f32;

        for z in 0..self.resolution {
            for x in 0..self.resolution {
                let world_x = cx - half_size + x as f32 * step;
                let world_z = cz - half_size + z as f32 * step;
                let height = self.sample_height(world_x, world_z);
                self.heightmap[(z * self.resolution + x) as usize] = height;
            }
        }
    }

    fn sample_height(&self, x: f32, z: f32) -> f32 {
        let (cx, cz) = self.island_center;
        let dx = x - cx;
        let dz = z - cz;
        let dist = (dx * dx + dz * dz).sqrt();
        let normalized_dist = dist / self.island_radius;
        if normalized_dist > 1.0 {
            return -30.0;
        }
        let falloff = 0.5 * (1.0 + (std::f32::consts::PI * normalized_dist).cos());
        // Multi-octave noise scaled for larger island
        let noise1 = (x * 0.04).sin() * (z * 0.05).cos() * 5.0;
        let noise2 = (x * 0.09 + 1.5).sin() * (z * 0.11 + 0.7).cos() * 2.5;
        let noise3 = (x * 0.22 + 3.0).sin() * (z * 0.25 + 2.0).cos() * 1.2;
        let noise4 = (x * 0.5 + 5.5).sin() * (z * 0.6 + 4.2).cos() * 0.5;
        let noise5 = (x * 1.1 + 7.3).sin() * (z * 0.9 + 6.1).cos() * 0.15;
        let base_height = self.max_height * falloff * falloff;
        (base_height + noise1 + noise2 + noise3 + noise4 + noise5).max(0.3)
    }

    fn height_at(&self, x: f32, z: f32) -> f32 {
        self.sample_height(x, z).max(0.0)
    }

    fn upload_to_renderer(&self, renderer: &mut Renderer) {
        let (cx, cz) = self.island_center;
        let half_size = self.island_radius * 1.05;
        let step = (half_size * 2.0) / self.resolution as f32;

        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for z in 0..self.resolution {
            for x in 0..self.resolution {
                let world_x = cx - half_size + x as f32 * step;
                let world_z = cz - half_size + z as f32 * step;
                let height = self.heightmap[(z * self.resolution + x) as usize];

                let h_left = if x > 0 {
                    self.heightmap[(z * self.resolution + x - 1) as usize]
                } else {
                    height
                };
                let h_right = if x < self.resolution - 1 {
                    self.heightmap[(z * self.resolution + x + 1) as usize]
                } else {
                    height
                };
                let h_back = if z > 0 {
                    self.heightmap[((z - 1) * self.resolution + x) as usize]
                } else {
                    height
                };
                let h_front = if z < self.resolution - 1 {
                    self.heightmap[((z + 1) * self.resolution + x) as usize]
                } else {
                    height
                };

                let normal = vec3(h_left - h_right, 2.0 * step, h_back - h_front).normalize();
                let u = x as f32 / self.resolution as f32;
                let v = z as f32 / self.resolution as f32;

                vertices.push(Vertex {
                    position: [world_x, height, world_z],
                    normal: [normal.x, normal.y, normal.z],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [u * 60.0, v * 60.0],
                });
            }
        }

        for z in 0..(self.resolution - 1) {
            for x in 0..(self.resolution - 1) {
                let tl = z * self.resolution + x;
                let tr = tl + 1;
                let bl = (z + 1) * self.resolution + x;
                let br = bl + 1;
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }

        use wgpu::util::DeviceExt;
        let vertex_buf = renderer
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vw-terrain-vertex"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buf = renderer
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vw-terrain-index"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let terrain_mesh = Mesh {
            vertex_buf,
            index_buf,
            index_count: indices.len() as u32,
        };

        let terrain_instance =
            Instance::from_pos_scale_color(vec3(0.0, 0.0, 0.0), Vec3::ONE, [0.45, 0.65, 0.3, 1.0]);
        renderer.add_model("terrain", terrain_mesh, &[terrain_instance]);
    }
}

// ═══════════════════════════════════════════════════════════════
//  Entity visuals — map ECS entities to 3D instances
// ═══════════════════════════════════════════════════════════════

/// Colors for factions
const PLAYER_COLOR: [f32; 4] = [0.2, 0.6, 1.0, 1.0]; // Blue
const COMPANION_COLORS: [[f32; 4]; 3] = [
    [0.3, 0.9, 0.4, 1.0], // Aria — green
    [0.9, 0.7, 0.2, 1.0], // Lyra — gold
    [0.7, 0.3, 0.9, 1.0], // Kael — purple
];
const ENEMY_COLOR: [f32; 4] = [0.9, 0.2, 0.2, 1.0]; // Red
const DEAD_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 0.5]; // Grey

fn build_entity_instances(app: &App, state: &GameState, terrain: &TerrainSystem) -> Vec<Instance> {
    let mut instances = Vec::new();

    // Player sphere
    if let Some(pe) = state.player_entity {
        if let Some(pos) = app.world.get::<Position>(pe) {
            let y = terrain.height_at(pos.value.x, pos.value.z) + 1.0;
            instances.push(Instance::from_pos_scale_color(
                vec3(pos.value.x, y, pos.value.z),
                Vec3::splat(2.5),
                PLAYER_COLOR,
            ));
        }
    }

    // Companion spheres
    for (i, &ce) in state.companion_entities.iter().enumerate() {
        if let Some(pos) = app.world.get::<Position>(ce) {
            let alive = app.world.get::<Health>(ce).map_or(false, |h| h.is_alive());
            let color = if alive {
                COMPANION_COLORS[i % COMPANION_COLORS.len()]
            } else {
                DEAD_COLOR
            };
            let y = terrain.height_at(pos.value.x, pos.value.z) + 0.8;
            instances.push(Instance::from_pos_scale_color(
                vec3(pos.value.x, y, pos.value.z),
                Vec3::splat(2.0),
                color,
            ));
        }
    }

    // Enemy spheres
    for &ee in &state.enemy_entities {
        if let Some(pos) = app.world.get::<Position>(ee) {
            let alive = app.world.get::<Health>(ee).map_or(false, |h| h.is_alive());
            let color = if alive { ENEMY_COLOR } else { DEAD_COLOR };
            let scale = if alive { 1.8 } else { 0.8 };
            let y = terrain.height_at(pos.value.x, pos.value.z) + 0.6;
            instances.push(Instance::from_pos_scale_color(
                vec3(pos.value.x, y, pos.value.z),
                Vec3::splat(scale),
                color,
            ));
        }
    }

    instances
}

// ═══════════════════════════════════════════════════════════════
//  egui HUD overlay
// ═══════════════════════════════════════════════════════════════

// ── Theme constants ──────────────────────────────────────────────────
const HUD_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(10, 10, 18, 210);
const HUD_BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(90, 60, 180, 120);
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(160, 100, 255);
const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(80, 200, 255);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(60, 220, 120);
const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);
const ACCENT_GOLD: egui::Color32 = egui::Color32::from_rgb(245, 200, 60);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(140, 140, 160);
const BAR_BG: egui::Color32 = egui::Color32::from_rgb(30, 28, 38);

/// Helper: draw a themed panel frame with a subtle border accent.
fn themed_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(HUD_BG)
        .corner_radius(6.0)
        .inner_margin(14.0)
        .stroke(egui::Stroke::new(1.0, HUD_BORDER))
}

/// Helper: draw a smooth gradient HP/status bar.
fn draw_bar(ui: &mut egui::Ui, width: f32, height: f32, frac: f32, color: egui::Color32) {
    let (_, bar_rect) = ui.allocate_space(egui::vec2(width, height));
    ui.painter().rect_filled(bar_rect, 3.0, BAR_BG);
    let filled = egui::Rect::from_min_max(
        bar_rect.left_top(),
        egui::pos2(
            bar_rect.left() + bar_rect.width() * frac.clamp(0.0, 1.0),
            bar_rect.bottom(),
        ),
    );
    ui.painter().rect_filled(filled, 3.0, color);
    // Bright edge highlight on the filled portion
    let edge = egui::Rect::from_min_max(
        egui::pos2(filled.right() - 2.0, filled.top()),
        filled.right_bottom(),
    );
    ui.painter()
        .rect_filled(edge, 0.0, egui::Color32::from_white_alpha(40));
}

fn draw_hud(ctx: &egui::Context, state: &GameState, app: &App, metrics: &TelemetryMetrics) {
    // ── Top-center: Title banner with glow ──────────────────────────────
    egui::Area::new(egui::Id::new("title_banner"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("V E I L W E A V E R")
                        .size(20.0)
                        .color(ACCENT_PURPLE)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("AI-Native Vertical Slice")
                        .size(10.0)
                        .color(TEXT_DIM),
                );
            });
        });

    // ── Top-left: Zone / Boss panel ─────────────────────────────────────
    egui::Area::new(egui::Id::new("zone_panel"))
        .fixed_pos(egui::pos2(16.0, 50.0))
        .show(ctx, |ui| {
            themed_frame().show(ui, |ui| {
                ui.set_min_width(220.0);
                ui.label(egui::RichText::new("◈  ZONE").color(TEXT_DIM).size(11.0));
                ui.label(
                    egui::RichText::new(&state.current_zone)
                        .color(ACCENT_CYAN)
                        .size(16.0)
                        .strong(),
                );
                if let Some(ref phase) = state.boss_phase {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("⚠  {phase}"))
                            .color(ACCENT_RED)
                            .size(13.0)
                            .strong(),
                    );
                }
            });
        });

    // ── Top-right: Engine telemetry ─────────────────────────────────────
    egui::Area::new(egui::Id::new("telemetry_panel"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 50.0))
        .show(ctx, |ui| {
            themed_frame().show(ui, |ui| {
                let fps = metrics.fps;
                let fps_color = if fps >= 58.0 {
                    ACCENT_GREEN
                } else if fps >= 30.0 {
                    ACCENT_GOLD
                } else {
                    ACCENT_RED
                };
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{fps:.0}"))
                            .color(fps_color)
                            .size(18.0)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("FPS").color(TEXT_DIM).size(11.0));
                });
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} ms  ·  tick {}",
                        metrics.frame_time.as_secs_f32() * 1000.0,
                        state.frame_count,
                    ))
                    .color(TEXT_DIM)
                    .size(11.0),
                );
            });
        });

    // ── Bottom-left: Player + companion status ──────────────────────────
    egui::Area::new(egui::Id::new("status_panel"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -16.0))
        .show(ctx, |ui| {
            themed_frame().show(ui, |ui| {
                ui.set_min_width(240.0);

                // Player HP
                let (hp, max_hp) = state
                    .player_entity
                    .and_then(|e| app.world.get::<Health>(e))
                    .map(|h| (h.current, h.max))
                    .unwrap_or((0.0, 100.0));
                let frac = hp / max_hp;
                let hp_color = if frac > 0.6 {
                    ACCENT_GREEN
                } else if frac > 0.3 {
                    ACCENT_GOLD
                } else {
                    ACCENT_RED
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HP").color(TEXT_DIM).size(11.0));
                    ui.label(
                        egui::RichText::new(format!("{hp:.0}"))
                            .color(hp_color)
                            .size(14.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!("/ {max_hp:.0}"))
                            .color(TEXT_DIM)
                            .size(11.0),
                    );
                });
                draw_bar(ui, 220.0, 10.0, frac, hp_color);

                ui.add_space(6.0);

                // Thread stability
                let stab = state.thread_stability / 100.0;
                let stab_color = if stab > 0.6 {
                    ACCENT_CYAN
                } else if stab > 0.3 {
                    ACCENT_GOLD
                } else {
                    ACCENT_RED
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("THREAD").color(TEXT_DIM).size(11.0));
                    ui.label(
                        egui::RichText::new(format!("{:.0}%", state.thread_stability))
                            .color(stab_color)
                            .size(13.0)
                            .strong(),
                    );
                });
                draw_bar(ui, 220.0, 6.0, stab, stab_color);

                ui.add_space(8.0);

                // Counts row
                let enemy_count = state
                    .enemy_entities
                    .iter()
                    .filter(|&&e| app.world.get::<Health>(e).map_or(false, |h| h.is_alive()))
                    .count();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("● {} Allies", state.companions_active))
                            .color(ACCENT_GREEN)
                            .size(12.0),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("▲ {} Hostiles", enemy_count))
                            .color(ACCENT_RED)
                            .size(12.0),
                    );
                });
                ui.label(
                    egui::RichText::new(format!("◆ {} Echoes", state.echoes_collected))
                        .color(ACCENT_PURPLE)
                        .size(12.0),
                );
            });
        });

    // ── Bottom-right: Event log ─────────────────────────────────────────
    egui::Area::new(egui::Id::new("event_log"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
        .show(ctx, |ui| {
            themed_frame().show(ui, |ui| {
                ui.set_min_width(280.0);
                ui.label(
                    egui::RichText::new("◉  EVENT LOG")
                        .color(TEXT_DIM)
                        .size(11.0),
                );
                ui.add_space(2.0);
                // Thin separator line
                let (_, sep_rect) = ui.allocate_space(egui::vec2(260.0, 1.0));
                ui.painter().rect_filled(sep_rect, 0.0, HUD_BORDER);
                ui.add_space(2.0);

                let start = state.events.len().saturating_sub(6);
                for ev in &state.events[start..] {
                    let clean = strip_ansi(ev);
                    ui.label(
                        egui::RichText::new(clean)
                            .color(egui::Color32::from_rgb(190, 190, 200))
                            .size(11.0),
                    );
                }
                if state.events.is_empty() {
                    ui.label(
                        egui::RichText::new("Awaiting first event...")
                            .color(TEXT_DIM)
                            .italics()
                            .size(11.0),
                    );
                }
            });
        });
}

/// Strip ANSI escape sequences from a string for clean egui display.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm'
            for inner in chars.by_ref() {
                if inner == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════
//  Main application struct
// ═══════════════════════════════════════════════════════════════

struct VeilweaverApp {
    window: Arc<Window>,
    renderer: Renderer,

    camera: Camera,
    camera_controller: CameraController,

    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: std::cell::RefCell<egui_wgpu::Renderer>,

    app: App,
    game_state: GameState,
    terrain: TerrainSystem,

    hud: TelemetryHud,
    metrics: TelemetryMetrics,

    _llm_client: Qwen3Ollama,

    start_time: Instant,
    last_frame: Instant,
    mouse_captured: bool,

    sim_duration: Duration,
    finished: bool,
}

impl VeilweaverApp {
    async fn new(window: Arc<Window>) -> Result<Self> {
        // Renderer
        let mut renderer = Renderer::new(window.clone())
            .await
            .context("Failed to create renderer")?;

        // Freeze time-of-day to morning
        {
            let tod = renderer.time_of_day_mut();
            tod.current_time = 9.0;
            tod.time_scale = 0.0;
        }

        // HDRI skybox — clean pure-sky for uncluttered horizon
        {
            renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::HdrPath {
                biome: "default".to_string(),
                path: "assets/hdri/polyhaven/kloppenheim/kloppenheim_06_puresky_2k.hdr".to_string(),
            };
            if let Err(e) = renderer.bake_environment(astraweave_render::ibl::IblQuality::Medium) {
                eprintln!("HDRI bake failed: {e:?} — falling back to procedural sky");
                renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::Procedural {
                    last_capture_time: 0.0,
                    recapture_interval: 0.0,
                };
            }
        }

        // Terrain
        let terrain = TerrainSystem::new();
        terrain.upload_to_renderer(&mut renderer);

        // Material — neutral white so instance color × texture show through cleanly
        renderer.set_material_params([1.0, 1.0, 1.0, 1.0], 0.0, 0.85);

        // Load grass texture for terrain detail
        let grass_path = "assets/textures/grass_bermuda_01_diff_1k.jpg";
        if std::path::Path::new(grass_path).exists() {
            renderer.set_smoke_test_texture(grass_path);
        }

        // ── Scene Environment ────────────────────────────────────────────
        // Configure fog for atmospheric perspective (distant edges fade to sky)
        {
            let env = renderer.scene_environment_mut();
            env.visuals.fog_color = glam::Vec3::new(0.72, 0.78, 0.85);
            env.visuals.fog_density = 0.004;
            env.visuals.fog_start = 80.0;
            env.visuals.fog_end = 400.0;
            // Bright ambient to match the HDRI sky lighting
            env.visuals.ambient_color = glam::Vec3::new(0.55, 0.6, 0.7);
            env.visuals.ambient_intensity = 0.65;

            // Tropical island water — vibrant teal/cyan tones
            env.visuals.water_deep = glam::Vec3::new(0.04, 0.15, 0.35);
            env.visuals.water_shallow = glam::Vec3::new(0.15, 0.55, 0.6);
            env.visuals.water_foam = glam::Vec3::new(1.0, 1.0, 1.0);
        }

        // Shadow tuning — soft PCF, low bias for terrain detail
        renderer.set_shadow_filter(2.0, 0.0005, 0.0);

        // Apply time-of-day lighting ONCE at init.
        // DO NOT call tick_environment() per frame — apply_time_of_day()
        // compounds ambient_intensity multiplicatively, causing exponential
        // lighting decay that produces a dark / negative-filter appearance.
        renderer.tick_environment(0.01);

        // Water — use Rgba16Float to match the HDR render pass when bloom is enabled
        let mut water = WaterRenderer::new(
            renderer.device(),
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Depth32Float,
        );
        // Apply the island water colors directly so the first frame is correct
        water.set_water_colors(
            glam::Vec3::new(0.04, 0.15, 0.35), // deep
            glam::Vec3::new(0.15, 0.55, 0.6),  // shallow
            glam::Vec3::new(1.0, 1.0, 1.0),    // foam
        );
        renderer.set_water_renderer(water);

        // Try loading trees (optional — graceful fallback)
        if let Ok(tree_mesh) = load_glb_mesh(renderer.device(), "assets/models/tree_default.glb") {
            renderer.set_external_mesh(tree_mesh);
            // Procedurally scatter ~80 trees across the island using golden-angle
            // spiral, then jitter for a natural look. Only place above waterline.
            let mut tree_instances: Vec<Instance> = Vec::with_capacity(80);
            let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt()); // ~2.399 rad
            let max_trees = 80u32;
            let island_r = 130.0_f32; // stay within island radius (150 - fringe)
            for i in 0..max_trees {
                let t = i as f32 / max_trees as f32;
                let r = island_r * t.sqrt(); // sqrt for uniform area distribution
                let theta = golden_angle * i as f32;
                // Deterministic jitter using simple hash
                let jx = ((i * 7 + 13) % 37) as f32 / 37.0 * 8.0 - 4.0;
                let jz = ((i * 11 + 3) % 41) as f32 / 41.0 * 8.0 - 4.0;
                let x = r * theta.cos() + jx;
                let z = r * theta.sin() + jz;
                let y = terrain.height_at(x, z);
                // Skip trees below waterline or too close to the player spawn
                if y < 3.0 || (x * x + z * z) < 100.0 {
                    continue;
                }
                // Size variation: smaller near edges, bigger in mid-range
                let base_scale = 3.5 + t * 3.0; // 3.5..6.5
                let size_jitter = ((i * 17 + 5) % 29) as f32 / 29.0 * 1.5;
                let scale = base_scale + size_jitter;
                // Slight colour variation per tree (warm/cool greens)
                let g_var = 0.85 + ((i * 23 + 7) % 19) as f32 / 19.0 * 0.3; // 0.85..1.15
                let r_var = 0.6 + ((i * 31 + 2) % 13) as f32 / 13.0 * 0.2; // 0.6..0.8
                tree_instances.push(Instance::from_pos_scale_color(
                    vec3(x, y, z),
                    Vec3::splat(scale),
                    [r_var, g_var, 0.5, 1.0],
                ));
            }
            renderer.set_external_instances(&tree_instances);
        }

        // Camera — overlooking the island from a scenic angle
        let camera = Camera {
            position: vec3(0.0, 35.0, -100.0),
            yaw: std::f32::consts::FRAC_PI_2,
            pitch: 0.25,
            fovy: 60f32.to_radians(),
            aspect: window.inner_size().width as f32 / window.inner_size().height.max(1) as f32,
            znear: 0.1,
            zfar: 2000.0,
        };
        let mut camera_controller = CameraController::new(40.0, 0.004);
        camera_controller.mouse_smooth = 50.0;
        camera_controller.mouse_deadzone = 0.5;

        // egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            Some(winit::window::Theme::Dark),
            Some(renderer.device().limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = std::cell::RefCell::new(egui_wgpu::Renderer::new(
            renderer.device(),
            renderer.surface_format(),
            None,
            1,
            false,
        ));

        // ECS + game state
        let mut ecs_app = App::new();
        let mut game_state = GameState::default();

        // Spawn player at island center
        let player = ecs_app.world.spawn();
        ecs_app.world.insert(player, Position { value: Vec3::ZERO });
        ecs_app.world.insert(player, Health::new(100.0));
        ecs_app.world.insert(player, Faction::Player);
        game_state.player_entity = Some(player);

        // Spawn companions near player (spread across island)
        let companion_positions = [
            Vec3::new(-8.0, 0.0, 6.0),
            Vec3::new(8.0, 0.0, 6.0),
            Vec3::new(0.0, 0.0, 14.0),
        ];
        for pos in &companion_positions {
            let c = ecs_app.world.spawn();
            ecs_app.world.insert(c, Position { value: *pos });
            ecs_app.world.insert(c, Health::new(100.0));
            ecs_app.world.insert(c, Faction::Companion);
            game_state.companion_entities.push(c);
        }

        // Spawn enemies in a wide circle across the island
        for i in 0..5 {
            let angle = (i as f32) * std::f32::consts::TAU / 5.0;
            let radius = 55.0;
            let spawn_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            let e = ecs_app.world.spawn();
            ecs_app.world.insert(e, Position { value: spawn_pos });
            ecs_app.world.insert(e, Health::new(80.0));
            ecs_app.world.insert(e, Faction::Enemy);
            game_state.enemy_entities.push(e);
        }

        let llm_client = Qwen3Ollama::fast();

        Ok(Self {
            window,
            renderer,
            camera,
            camera_controller,
            egui_ctx,
            egui_state,
            egui_renderer,
            app: ecs_app,
            game_state,
            terrain,
            hud: TelemetryHud::new(),
            metrics: TelemetryMetrics::default(),
            _llm_client: llm_client,
            start_time: Instant::now(),
            last_frame: Instant::now(),
            mouse_captured: false,
            sim_duration: Duration::from_secs(1800),
            finished: false,
        })
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.renderer.resize(size.width, size.height);
            self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
        }
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        let egui_response = self.egui_state.on_window_event(&self.window, event);
        if egui_response.consumed {
            return true;
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;

                    if pressed {
                        match keycode {
                            KeyCode::Escape => {
                                self.mouse_captured = !self.mouse_captured;
                                let _ = self.window.set_cursor_grab(if self.mouse_captured {
                                    winit::window::CursorGrabMode::Confined
                                } else {
                                    winit::window::CursorGrabMode::None
                                });
                                self.window.set_cursor_visible(!self.mouse_captured);
                            }
                            _ => {}
                        }
                    }
                    self.camera_controller.process_keyboard(keycode, pressed);
                }
                true
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.camera_controller
                    .process_mouse_button(*button, *state == ElementState::Pressed);
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.camera_controller.process_mouse_move(
                    &mut self.camera,
                    Vec2::new(position.x as f32, position.y as f32),
                );
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: f32) {
        // Camera ALWAYS updates — even after simulation finishes —
        // so the player can keep flying around the scene.
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.renderer.update_view(&self.camera.to_render_view());

        // Water animation continues regardless of sim state
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let view_proj = self.camera.vp();
        self.renderer
            .update_water(view_proj, self.camera.position, elapsed);
        // NOTE: DO NOT call tick_environment() here — it compounds
        // ambient_intensity every frame. Called once at init instead.

        if self.finished {
            return;
        }

        self.game_state.elapsed = elapsed;
        self.game_state.frame_count += 1;

        // Scale 30-minute real-time → 30s event timeline so the scripted
        // gameplay paces itself across the full vertical-slice duration.
        let sim_time = elapsed * (30.0 / self.sim_duration.as_secs_f32());
        crate::simulate_events(&mut self.game_state, sim_time, &mut self.app);

        // Check if simulation done
        if self.start_time.elapsed() >= self.sim_duration {
            self.finished = true;
        }

        // Rebuild entity instances
        let instances = build_entity_instances(&self.app, &self.game_state, &self.terrain);
        self.renderer.update_instances(&instances);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.update_view(&self.camera.to_render_view());

        let elapsed = self.start_time.elapsed().as_secs_f32();
        self.renderer
            .update_water(self.camera.vp(), self.camera.position, elapsed);

        // egui
        let raw_input = self.egui_state.take_egui_input(&self.window);

        let game_state = &self.game_state;
        let ecs_app = &self.app;
        let metrics = &self.metrics;
        let finished = self.finished;

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            draw_hud(ctx, game_state, ecs_app, metrics);

            if finished {
                egui::Area::new(egui::Id::new("finished_banner"))
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(200))
                            .corner_radius(12.0)
                            .inner_margin(24.0)
                            .show(ui, |ui| {
                                ui.heading(
                                    egui::RichText::new("SIMULATION COMPLETE")
                                        .color(egui::Color32::from_rgb(180, 120, 255))
                                        .size(28.0),
                                );
                                ui.label(
                                    egui::RichText::new("Press ESC to exit • Camera still active")
                                        .color(egui::Color32::GRAY),
                                );
                            });
                    });
            }
        });

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let size = self.window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.borrow_mut().update_texture(
                self.renderer.device(),
                self.renderer.queue(),
                *id,
                image_delta,
            );
        }

        let egui_renderer = &self.egui_renderer;

        let result = self
            .renderer
            .render_with(|view, encoder, device, queue, _size| {
                egui_renderer.borrow_mut().update_buffers(
                    device,
                    queue,
                    encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui_pass"),
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

                    // SAFETY: The render pass doesn't outlive the encoder scope.
                    let render_pass_static: &mut wgpu::RenderPass<'static> =
                        unsafe { std::mem::transmute(&mut render_pass) };
                    egui_renderer.borrow_mut().render(
                        render_pass_static,
                        &paint_jobs,
                        &screen_descriptor,
                    );
                }
            });

        for id in &full_output.textures_delta.free {
            self.egui_renderer.borrow_mut().free_texture(id);
        }

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("Render error: {e:?}");
                Err(wgpu::SurfaceError::Lost)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  GLB model loader (same as hello_companion)
// ═══════════════════════════════════════════════════════════════

fn load_glb_mesh(device: &wgpu::Device, path: &str) -> Result<Mesh> {
    use wgpu::util::DeviceExt;

    let bytes = std::fs::read(path).with_context(|| format!("Failed to read GLB: {path}"))?;
    let mesh_data = astraweave_asset::gltf_loader::load_all_meshes_merged(&bytes)
        .with_context(|| format!("Failed to parse GLB: {path}"))?;

    let vertices: Vec<Vertex> = mesh_data
        .positions
        .iter()
        .zip(&mesh_data.normals)
        .zip(&mesh_data.tangents)
        .zip(&mesh_data.texcoords)
        .map(|(((pos, nor), tan), uv)| Vertex {
            position: *pos,
            normal: *nor,
            tangent: *tan,
            uv: *uv,
        })
        .collect();

    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("glb-vertex"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("glb-index"),
        contents: bytemuck::cast_slice(&mesh_data.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Ok(Mesh {
        vertex_buf,
        index_buf,
        index_count: mesh_data.indices.len() as u32,
    })
}

// ═══════════════════════════════════════════════════════════════
//  winit Application Handler
// ═══════════════════════════════════════════════════════════════

struct VeilweaverHandler {
    app: Option<VeilweaverApp>,
}

impl ApplicationHandler for VeilweaverHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("Veilweaver — AI-Native Game Engine")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create window"),
            );

            self.app = Some(pollster::block_on(VeilweaverApp::new(window)).unwrap());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(app) = &mut self.app else { return };

        match &event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => app.resize(*size),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - app.last_frame).as_secs_f32();
                app.last_frame = now;

                app.update(dt);

                // Update telemetry metrics
                let frame_time = Duration::from_secs_f32(dt);
                app.metrics.frame_time = frame_time;
                app.metrics.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
                app.hud.update(&app.metrics);

                match app.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => app.resize(app.window.inner_size()),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("Render error: {e:?}"),
                }

                app.window.request_redraw();
            }
            _ => {
                app.handle_input(&event);
            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(app) = &mut self.app {
            if let DeviceEvent::MouseMotion { delta } = event {
                if app.mouse_captured {
                    let sens = 0.004;
                    app.camera.yaw -= delta.0 as f32 * sens;
                    app.camera.pitch = (app.camera.pitch + delta.1 as f32 * sens).clamp(-1.5, 1.5);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Public entry point
// ═══════════════════════════════════════════════════════════════

/// Launch the full 3D windowed Veilweaver demo.
pub fn run_visual_demo() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   Veilweaver — AI-Native Game Engine (Visual Mode)         ║");
    println!("║   WASD: move camera · Mouse: look · ESC: toggle grab       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut handler = VeilweaverHandler { app: None };
    event_loop
        .run_app(&mut handler)
        .context("Event loop error")?;

    Ok(())
}
