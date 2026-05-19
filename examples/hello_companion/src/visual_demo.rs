//! Hello Companion Visual Demo
//!
//! A visual demo showcasing 3 AI modes with an interactive NPC:
//! - Mode 1 (Pure LLM): Full conversation via Qwen3-8B
//! - Mode 2 (Pure GOAP): Pre-written scripted responses
//! - Mode 3 (Arbiter): GOAP+LLM hybrid with organic filler dialogue
//!
//! Usage:
//!   cargo run -p hello_companion --release --features visual

#![allow(dead_code)] // Scene configs and methods reserved for future 3D expansion

use anyhow::{Context, Result};
use glam::{vec3, Vec2, Vec3};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// Use astraweave-render for full 3D rendering
use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_render::{Instance, Mesh, Renderer, Vertex, WaterRenderer};

// Asset loading for GLB models
use astraweave_asset::gltf_loader;

// LLM integration for real Qwen3-8B chat
use astraweave_llm::qwen3_ollama::Qwen3Ollama;

use crate::chat_ui::{self, ChatUi};
use crate::dialogue_bank::DialogueBank;
use crate::llm_worker::LlmWorker;
use crate::scene::{DemoMode, NpcDialogueState, NpcState, SceneConfig};

// Terrain generation no longer uses voxels - using heightmap approach instead

/// Procedural island terrain system using heightmap generation.
/// Creates a visible island mesh with proper geometry.
struct TerrainSystem {
    /// Island center (x, z).
    island_center: (f32, f32),
    /// Island radius.
    island_radius: f32,
    /// Maximum terrain height.
    max_height: f32,
    /// Grid resolution.
    resolution: u32,
    /// Cached heightmap for asset placement.
    heightmap: Vec<f32>,
}

impl TerrainSystem {
    /// Create a new terrain system with large visible island.
    fn new() -> Self {
        let resolution = 64;
        let mut ts = Self {
            island_center: (0.0, 15.0),
            island_radius: 50.0,
            max_height: 15.0,
            resolution,
            heightmap: vec![0.0; (resolution * resolution) as usize],
        };
        ts.generate_heightmap();
        ts
    }

    /// Generate heightmap data for the island.
    fn generate_heightmap(&mut self) {
        let (cx, cz) = self.island_center;
        let half_size = self.island_radius * 1.5;
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

    /// Sample terrain height at (x, z) using island shape with noise.
    fn sample_height(&self, x: f32, z: f32) -> f32 {
        let (cx, cz) = self.island_center;
        let dx = x - cx;
        let dz = z - cz;
        let dist = (dx * dx + dz * dz).sqrt();

        // Circular island falloff with smoother edges
        let normalized_dist = dist / self.island_radius;
        if normalized_dist > 1.0 {
            return -0.5; // Below water level
        }

        // Smooth falloff curve (raised cosine)
        let falloff = 0.5 * (1.0 + (std::f32::consts::PI * normalized_dist).cos());

        // Multi-octave noise for natural variation
        let noise1 = (x * 0.1).sin() * (z * 0.12).cos() * 2.0;
        let noise2 = (x * 0.25 + 1.5).sin() * (z * 0.3 + 0.7).cos() * 0.8;
        let noise3 = (x * 0.5 + 3.0).sin() * (z * 0.6 + 2.0).cos() * 0.3;
        let noise = noise1 + noise2 + noise3;

        // Base height with noise
        let base_height = self.max_height * falloff * falloff;
        (base_height + noise).max(0.0)
    }

    /// Get terrain height at any world position (for asset placement).
    fn height_at(&self, x: f32, z: f32) -> f32 {
        self.sample_height(x, z).max(0.0)
    }

    /// Generate terrain mesh and upload to renderer.
    fn upload_to_renderer(&self, renderer: &mut Renderer) {
        let (cx, cz) = self.island_center;
        let half_size = self.island_radius * 1.5;
        let step = (half_size * 2.0) / self.resolution as f32;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for z in 0..self.resolution {
            for x in 0..self.resolution {
                let world_x = cx - half_size + x as f32 * step;
                let world_z = cz - half_size + z as f32 * step;
                let height = self.heightmap[(z * self.resolution + x) as usize];

                // Calculate normal using central differences
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

                // UV coordinates
                let u = x as f32 / self.resolution as f32;
                let v = z as f32 / self.resolution as f32;

                // Color based on height (green grass to brown rock)
                // Color based on height (green grass to brown rock)
                // Use white to allow texture to show clearly
                let color = [1.0, 1.0, 1.0, 1.0];

                vertices.push(Vertex {
                    position: [world_x, height, world_z],
                    normal: [normal.x, normal.y, normal.z],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv: [u * 10.0, v * 10.0], // Tile UVs
                });
            }
        }

        // Generate indices
        for z in 0..(self.resolution - 1) {
            for x in 0..(self.resolution - 1) {
                let top_left = z * self.resolution + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * self.resolution + x;
                let bottom_right = bottom_left + 1;

                // First triangle
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Second triangle
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        println!(
            "TerrainSystem: Generated {} vertices, {} triangles",
            vertices.len(),
            indices.len() / 3
        );

        // Create GPU mesh
        use wgpu::util::DeviceExt;
        let vertex_buf = renderer
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain-vertex"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buf = renderer
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain-index"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let terrain_mesh = Mesh {
            vertex_buf,
            index_buf,
            index_count: indices.len() as u32,
        };

        // Add as named model with single identity instance
        let terrain_instance = Instance::from_pos_scale_color(
            vec3(0.0, 0.0, 0.0),
            Vec3::ONE,
            [1.0, 1.0, 1.0, 1.0], // White (let texture provide color)
        );
        renderer.add_model("terrain", terrain_mesh, &[terrain_instance]);
    }
}

/// Load a GLB model and convert it to a GPU-ready Mesh
/// Uses load_all_meshes_merged to combine all mesh parts (e.g., trunk + foliage)
fn load_glb_mesh(device: &wgpu::Device, path: &str) -> Result<Mesh> {
    use wgpu::util::DeviceExt;

    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read GLB file: {}", path))?;

    let mesh_data = gltf_loader::load_all_meshes_merged(&bytes)
        .with_context(|| format!("Failed to parse GLB: {}", path))?;

    // Convert to Vertex format (position, normal, tangent, uv)
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

/// Session memory for user preferences (persists across conversation).
#[derive(Debug, Clone, Default)]
struct SessionMemory {
    /// User's name if provided.
    user_name: Option<String>,
    /// Preferred response style: "brief", "detailed", or none.
    preferred_style: Option<String>,
    /// Topics the user has shown interest in.
    interests: Vec<String>,
    /// Number of exchanges in this session.
    exchange_count: u32,
    /// Recent conversation turns for multi-turn reasoning (user, assistant pairs).
    recent_turns: Vec<(String, String)>,
    /// Last detected intent (for follow-up questions).
    last_intent: Option<String>,
    /// Unresolved question awaiting answer.
    pending_question: Option<String>,
    /// Detected user mood (calm, curious, frustrated, excited, tired, anxious).
    user_mood: Option<String>,
    /// Rapport level (0-100) - increases with positive interactions.
    rapport: u32,
    /// Topics user explicitly said to avoid.
    avoid_topics: Vec<String>,
}

/// Turn entry for conversation history.
#[derive(Debug, Clone)]
struct ConversationTurn {
    user_input: String,
    assistant_response: String,
}

impl SessionMemory {
    /// Maximum recent turns to remember.
    const MAX_TURNS: usize = 5;

    fn update_from_input(&mut self, input: &str) {
        let lower = input.to_lowercase();

        // Parse name if provided.
        if let Some(rest) = lower.strip_prefix("my name is ") {
            if let Some(name) = rest.split_whitespace().next() {
                self.user_name = Some(name.to_string());
                self.rapport = self.rapport.saturating_add(10); // Name sharing builds rapport
            }
        } else if let Some(rest) = lower.strip_prefix("call me ") {
            if let Some(name) = rest.split_whitespace().next() {
                self.user_name = Some(name.to_string());
                self.rapport = self.rapport.saturating_add(10);
            }
        } else if let Some(rest) = lower.strip_prefix("i'm ") {
            if let Some(name) = rest.split_whitespace().next() {
                if name.len() > 1 && name.chars().all(|c| c.is_alphabetic()) {
                    self.user_name = Some(name.to_string());
                    self.rapport = self.rapport.saturating_add(10);
                }
            }
        }

        // Parse style preference.
        if lower.contains("brief")
            || lower.contains("short answers")
            || lower.contains("keep it short")
        {
            self.preferred_style = Some("brief".to_string());
        } else if lower.contains("detailed")
            || lower.contains("thorough")
            || lower.contains("elaborate")
        {
            self.preferred_style = Some("detailed".to_string());
        }

        // Track interests.
        let topics = [
            "combat",
            "stealth",
            "exploration",
            "story",
            "puzzle",
            "quest",
            "crafting",
        ];
        for topic in topics {
            if lower.contains(topic) && !self.interests.contains(&topic.to_string()) {
                self.interests.push(topic.to_string());
            }
        }

        // Detect and track user mood.
        self.user_mood = Self::detect_mood(&lower);

        // Track avoid topics.
        if lower.contains("don't mention") || lower.contains("avoid") || lower.contains("skip") {
            for topic in &["combat", "violence", "death", "spoilers", "hints"] {
                if lower.contains(topic) && !self.avoid_topics.contains(&topic.to_string()) {
                    self.avoid_topics.push(topic.to_string());
                }
            }
        }

        // Update rapport based on interaction quality.
        self.update_rapport(&lower);

        // Detect intent for follow-up.
        self.last_intent = Self::detect_intent(&lower);

        // Track pending questions.
        if lower.ends_with('?')
            || lower.starts_with("what")
            || lower.starts_with("how")
            || lower.starts_with("why")
            || lower.starts_with("when")
            || lower.starts_with("where")
        {
            self.pending_question = Some(input.to_string());
        }

        self.exchange_count += 1;
    }

    /// Detect user mood from input.
    fn detect_mood(lower: &str) -> Option<String> {
        // Negative moods
        if lower.contains("frustrated") || lower.contains("annoyed") || lower.contains("angry") {
            return Some("frustrated".to_string());
        }
        if lower.contains("tired") || lower.contains("exhausted") || lower.contains("sleepy") {
            return Some("tired".to_string());
        }
        if lower.contains("anxious")
            || lower.contains("worried")
            || lower.contains("nervous")
            || lower.contains("scared")
        {
            return Some("anxious".to_string());
        }
        if lower.contains("confused") || lower.contains("lost") || lower.contains("stuck") {
            return Some("confused".to_string());
        }
        if lower.contains("sad") || lower.contains("down") || lower.contains("lonely") {
            return Some("sad".to_string());
        }
        // Positive moods
        if lower.contains("excited")
            || lower.contains("pumped")
            || lower.contains("ready")
            || lower.contains("eager")
        {
            return Some("excited".to_string());
        }
        if lower.contains("happy") || lower.contains("glad") || lower.contains("great") {
            return Some("happy".to_string());
        }
        if lower.contains("curious") || lower.contains("wonder") || lower.contains("interested") {
            return Some("curious".to_string());
        }
        if lower.contains("calm") || lower.contains("relaxed") || lower.contains("peaceful") {
            return Some("calm".to_string());
        }
        // Implicit mood from punctuation and words
        if lower.contains('!') && lower.len() < 50 {
            return Some("energetic".to_string());
        }
        if lower.contains("please") || lower.contains("thank") {
            return Some("polite".to_string());
        }
        None
    }

    /// Update rapport based on interaction signals.
    fn update_rapport(&mut self, lower: &str) {
        // Positive signals increase rapport
        if lower.contains("thank") || lower.contains("thanks") || lower.contains("appreciate") {
            self.rapport = self.rapport.saturating_add(5).min(100);
        }
        if lower.contains("great")
            || lower.contains("helpful")
            || lower.contains("perfect")
            || lower.contains("awesome")
        {
            self.rapport = self.rapport.saturating_add(8).min(100);
        }
        if lower.contains("please") {
            self.rapport = self.rapport.saturating_add(2).min(100);
        }
        // Longer, engaged messages suggest rapport
        if lower.len() > 100 {
            self.rapport = self.rapport.saturating_add(3).min(100);
        }
        // Negative signals decrease rapport slightly
        if lower.contains("useless") || lower.contains("stupid") || lower.contains("wrong") {
            self.rapport = self.rapport.saturating_sub(5);
        }
        // Natural rapport growth from continued conversation
        if self.exchange_count > 0 && self.exchange_count % 5 == 0 {
            self.rapport = self.rapport.saturating_add(2).min(100);
        }
    }

    /// Get rapport tier for adaptive responses.
    fn rapport_tier(&self) -> &'static str {
        match self.rapport {
            0..=20 => "stranger",
            21..=50 => "acquaintance",
            51..=80 => "friend",
            _ => "trusted",
        }
    }

    /// Record a completed exchange (user input → assistant response).
    fn record_turn(&mut self, user_input: &str, response: &str) {
        self.recent_turns
            .push((user_input.to_string(), response.to_string()));
        if self.recent_turns.len() > Self::MAX_TURNS {
            self.recent_turns.remove(0);
        }
        // Clear pending question if answered.
        self.pending_question = None;
    }

    /// Detect the primary intent from user input.
    fn detect_intent(lower: &str) -> Option<String> {
        if lower.contains("help") || lower.contains("assist") {
            Some("help".to_string())
        } else if lower.contains("explain") || lower.contains("what is") {
            Some("explanation".to_string())
        } else if lower.contains("how do") || lower.contains("how can") {
            Some("instruction".to_string())
        } else if lower.contains("compare") || lower.contains("difference") {
            Some("comparison".to_string())
        } else if lower.contains("recommend") || lower.contains("suggest") {
            Some("recommendation".to_string())
        } else if lower.contains("plan") || lower.contains("goal") {
            Some("planning".to_string())
        } else {
            None
        }
    }

    /// Check if this is a follow-up to the previous turn.
    fn is_followup(&self, input: &str) -> bool {
        let lower = input.to_lowercase();
        // Short inputs that reference previous context.
        if lower.len() < 30
            && (lower.starts_with("and ")
                || lower.starts_with("also ")
                || lower.starts_with("what about")
                || lower.starts_with("how about")
                || lower == "why"
                || lower == "how"
                || lower == "more"
                || lower.contains("that")
                || lower.contains("this")
                || lower.contains("the same")
                || lower.contains("again"))
        {
            return true;
        }
        false
    }

    /// Get a summary of recent conversation for context.
    fn recent_context_summary(&self) -> Option<String> {
        if self.recent_turns.is_empty() {
            return None;
        }
        let last = self.recent_turns.last()?;
        Some(format!(
            "Last exchange: User asked '{}' and I replied '{}'",
            truncate_str(&last.0, 50),
            truncate_str(&last.1, 80)
        ))
    }

    /// Build context prefix for LLM based on memory.
    fn to_context_prefix(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref name) = self.user_name {
            parts.push(format!("The user's name is {}.", name));
        }
        if let Some(ref style) = self.preferred_style {
            parts.push(format!("The user prefers {} responses.", style));
        }
        if let Some(ref mood) = self.user_mood {
            parts.push(format!("The user seems {} right now.", mood));
        }
        if !self.interests.is_empty() {
            parts.push(format!(
                "The user has shown interest in: {}.",
                self.interests.join(", ")
            ));
        }
        if !self.avoid_topics.is_empty() {
            parts.push(format!(
                "Avoid mentioning: {}.",
                self.avoid_topics.join(", ")
            ));
        }
        // Rapport-based tone guidance
        match self.rapport_tier() {
            "stranger" => parts.push("Be welcoming but professional.".to_string()),
            "acquaintance" => parts.push("Be friendly and helpful.".to_string()),
            "friend" => parts.push("Be warm and personable.".to_string()),
            "trusted" => parts.push("Be open and supportive like a close friend.".to_string()),
            _ => {}
        }
        if self.exchange_count > 5 {
            parts.push("This is an ongoing conversation; maintain continuity.".to_string());
        }
        if let Some(ref context) = self.recent_context_summary() {
            parts.push(context.clone());
        }
        parts.join(" ")
    }
}

/// Truncate a string to max_len characters with "..." suffix.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Main application state
struct CompanionApp {
    // Window and Renderer
    window: Arc<Window>,
    renderer: Renderer,

    // Camera
    camera: Camera,
    camera_controller: CameraController,

    // UI (egui overlay on top of 3D scene)
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: std::cell::RefCell<egui_wgpu::Renderer>,
    chat_ui: ChatUi,

    // Scene
    npc_state: NpcState,
    dialogue_bank: DialogueBank,
    scene_config: SceneConfig,

    // Input
    mouse_captured: bool,
    keys_held: std::collections::HashSet<KeyCode>,

    // Timing
    last_frame: Instant,
    frame_count: u64,

    // LLM state (for Arbiter mode)
    llm_pending: bool,
    llm_request_time: Option<Instant>,
    filler_index: usize,
    current_fillers: Vec<String>,

    // Pending chat input from UI (stored for LLM context)
    pending_chat_input: Option<String>,
    last_user_input: String,

    // Demo scene instances (visible spheres)
    demo_instances: Vec<Instance>,

    // LLM client for real Qwen3-8B integration
    llm_worker: LlmWorker,
    llm_response_receiver: Option<tokio::sync::oneshot::Receiver<String>>,

    // Streaming partial output (Arbiter mode).
    llm_stream_receiver: Option<std::sync::mpsc::Receiver<String>>,
    last_streamed_text: String,

    // Session memory for personalization.
    session_memory: SessionMemory,

    // Start time for animation
    start_time: Instant,
}

impl CompanionApp {
    async fn new(window: Arc<Window>) -> Result<Self> {
        // Create renderer (handles all wgpu setup internally)
        let mut renderer = Renderer::new(window.clone())
            .await
            .context("Failed to create renderer")?;

        // Set time to morning (10:00) for good lighting angle and pause it
        {
            let tod = renderer.time_of_day_mut();
            tod.current_time = 10.0;
            tod.time_scale = 0.0; // Freeze time for consistent lighting
        }

        // Initialize HDRI environment
        {
            println!("Initializing HDRI environment...");
            // Set to HDRI mode
            renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::HdrPath {
                biome: "default".to_string(),
                path: "assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr".to_string(),
            };

            // Bake environment (offline process step)
            if let Err(e) = renderer.bake_environment(astraweave_render::ibl::IblQuality::Medium) {
                eprintln!(
                    "Failed to bake environment: {:?} (falling back to procedural)",
                    e
                );
                // Switch back to procedural if failed
                renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::Procedural {
                    last_capture_time: 0.0,
                    recapture_interval: 0.0,
                };
            } else {
                println!("✓ HDRI environment baked successfully");
            }
        }

        // Generate and upload procedural terrain
        println!("Generating procedural terrain...");
        let terrain_system = TerrainSystem::new();
        terrain_system.upload_to_renderer(&mut renderer);
        println!("✓ Terrain generated and uploaded");

        // Load forest floor texture for terrain
        println!("Loading terrain texture...");
        renderer.set_smoke_test_texture("assets/dirt.ktx2");
        // Ensure material is not black (default might be 0,0,0)
        renderer.set_material_params([1.0, 1.0, 1.0, 1.0], 0.0, 0.8);
        println!("✓ Forest floor texture loaded");

        // Create water renderer for ocean and pass to Renderer
        // Use Rgba16Float to match the HDR render pass (bloom feature is enabled)
        println!("Initializing ocean...");
        let water_renderer = WaterRenderer::new(
            renderer.device(),
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Depth32Float,
        );
        renderer.set_water_renderer(water_renderer);
        println!("✓ Ocean initialized with Gerstner waves");

        // Try to load a GLB model as external mesh (optional - continue if fails)
        if let Ok(tree_mesh) = load_glb_mesh(renderer.device(), "assets/models/tree_default.glb") {
            println!(
                "✓ Loaded tree_default.glb: {} indices",
                tree_mesh.index_count
            );
            renderer.set_external_mesh(tree_mesh);

            // Tree positions - snap to terrain height using height_at
            let tree_positions = [
                (5.0_f32, 8.0_f32, 2.0_f32, [0.3, 0.7, 0.3, 1.0]), // Front right
                (-7.0, 10.0, 1.8, [0.35, 0.65, 0.25, 1.0]),        // Left cluster
                (-5.0, 12.0, 2.2, [0.25, 0.6, 0.3, 1.0]),
                (-9.0, 8.0, 1.5, [0.4, 0.75, 0.35, 1.0]),
                (8.0, 12.0, 2.5, [0.2, 0.55, 0.25, 1.0]), // Right cluster
                (10.0, 10.0, 1.7, [0.38, 0.68, 0.28, 1.0]),
                (0.0, 18.0, 3.0, [0.15, 0.45, 0.2, 1.0]), // Background
                (-4.0, 20.0, 2.8, [0.18, 0.5, 0.22, 1.0]),
                (4.0, 22.0, 3.2, [0.12, 0.42, 0.18, 1.0]),
            ];

            let tree_instances: Vec<Instance> = tree_positions
                .iter()
                .map(|(x, z, scale, color)| {
                    let y = terrain_system.height_at(*x, *z);
                    Instance::from_pos_scale_color(vec3(*x, y, *z), Vec3::splat(*scale), *color)
                })
                .collect();

            renderer.set_external_instances(&tree_instances);
            println!(
                "  External mesh set: {} - {} tree instances snapped to terrain",
                renderer.has_external_mesh(),
                tree_instances.len()
            );
        } else {
            println!("Note: Could not load tree_default.glb, using default spheres only");
        }

        // Setup camera looking at the island center
        // Camera positioned to overlook the terrain
        let camera = Camera {
            position: vec3(0.0, 20.0, -15.0), // Higher up, behind island
            yaw: std::f32::consts::FRAC_PI_2, // Face +Z direction (toward island)
            pitch: 0.25,                      // Look down at terrain
            fovy: 60f32.to_radians(),
            aspect: window.inner_size().width as f32 / window.inner_size().height.max(1) as f32,
            znear: 0.1,
            zfar: 500.0, // Increased for larger scene
        };
        // Fast camera with high sensitivity
        let mut camera_controller = CameraController::new(8.0, 0.004);
        camera_controller.mouse_smooth = 50.0; // Very high = nearly instant (no drift)
        camera_controller.mouse_deadzone = 0.5; // Small deadzone

        // Setup egui using renderer's device and format
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

        // Initialize chat
        let mut chat_ui = ChatUi::new();
        chat_ui.add_system_message("Welcome to Hello Companion Demo!");
        chat_ui.add_system_message("Press 1/2/3 to switch AI modes. WASD to move camera.");

        // Scene config
        let scene_config = SceneConfig::default();

        // Only keep NPC companion as a sphere - terrain and trees come from proper meshes
        let demo_instances = vec![
            // NPC COMPANION (purple sphere, central focal point on terrain)
            Instance::from_pos_scale_color(
                vec3(0.0, terrain_system.height_at(0.0, 15.0) + 1.5, 15.0),
                Vec3::splat(1.5),
                [0.7, 0.3, 0.9, 1.0], // Vibrant purple
            ),
        ];

        Ok(Self {
            window,
            renderer,
            camera,
            camera_controller,
            egui_ctx,
            egui_state,
            egui_renderer,
            chat_ui,
            npc_state: NpcState::default(),
            dialogue_bank: DialogueBank::new(),
            scene_config,
            mouse_captured: false,
            keys_held: std::collections::HashSet::new(),
            last_frame: Instant::now(),
            frame_count: 0,
            llm_pending: false,
            llm_request_time: None,
            filler_index: 0,
            current_fillers: Vec::new(),
            pending_chat_input: None,
            last_user_input: String::new(),
            demo_instances,
            // Initialize LLM worker (reuses a single runtime + caches prompts)
            llm_worker: LlmWorker::new(
                Qwen3Ollama::new(
                    "http://localhost:11434",
                    "qwen3:8b",
                )
                .with_temperature(0.6)
                .with_max_tokens(192)
                .with_system_prompt(
                    "You are a friendly NPC companion in a peaceful game world. \
                     Respond naturally and conversationally. Be helpful, warm, and engaging. \
                     Keep responses concise (2-4 sentences). Use occasional emotes like *smiles* or *nods*. \
                     You can discuss the weather, adventures, stories, and help the player. \
                     Never break character or mention being an AI.",
                ),
            ),
            llm_response_receiver: None,
            llm_stream_receiver: None,
            last_streamed_text: String::new(),
            session_memory: SessionMemory::default(),
            start_time: Instant::now(),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.renderer.resize(new_size.width, new_size.height);
            self.camera.aspect = new_size.width as f32 / new_size.height.max(1) as f32;
        }
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        // Let egui handle input first
        let egui_response = self.egui_state.on_window_event(&self.window, event);
        if egui_response.consumed {
            return true;
        }

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;

                    if pressed {
                        self.keys_held.insert(keycode);

                        // Mode switching
                        match keycode {
                            KeyCode::Digit1 => self.switch_mode(DemoMode::PureLlm),
                            KeyCode::Digit2 => self.switch_mode(DemoMode::PureGoap),
                            KeyCode::Digit3 => self.switch_mode(DemoMode::Arbiter),
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
                    } else {
                        self.keys_held.remove(&keycode);
                    }

                    // Forward to camera controller
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

    fn switch_mode(&mut self, mode: DemoMode) {
        self.chat_ui.switch_mode(mode);

        // Add a greeting when switching to a new mode
        let greeting = self.dialogue_bank.greeting_for_mode(mode);
        self.chat_ui.add_npc_message(greeting);
    }

    /// Check for demo choreography commands (special triggers that showcase the system).
    fn check_choreography(&mut self, text: &str) -> bool {
        let lower = text.to_lowercase();

        // "compare modes" / "show me modes" - run the same prompt across all modes.
        if lower.contains("compare mode")
            || lower.contains("show me mode")
            || lower.contains("demonstrate mode")
        {
            self.chat_ui.add_system_message(
                "🎬 Demo Choreography: Comparing all 3 modes with 'What should I do next?'",
            );
            let test_prompt = "What should I do next?";

            // Mode 1: GOAP (instant)
            self.chat_ui
                .add_system_message("─── Mode 2 (Pure GOAP) ───");
            let goap_response = self
                .dialogue_bank
                .goap_response_string(test_prompt)
                .unwrap_or_else(|| {
                    self.dialogue_bank
                        .goap_fallback_contextual(test_prompt)
                        .to_string()
                });
            self.chat_ui
                .add_npc_message(&format!("[GOAP] {}", goap_response));

            // Mode 2: Note about LLM
            self.chat_ui.add_system_message("─── Mode 1 (Pure LLM) ───");
            self.chat_ui.add_npc_message(
                "[LLM] *thinking...* (switch to Mode 1 and ask to see the full response)",
            );

            // Mode 3: Note about Arbiter
            self.chat_ui.add_system_message("─── Mode 3 (Arbiter) ───");
            self.chat_ui
                .add_npc_message("[Arbiter] Quick take first, then deeper answer. Best of both!");

            self.chat_ui.add_system_message(
                "🎬 Try asking the same question in each mode (press 1/2/3 to switch).",
            );
            return true;
        }

        // "show me streaming" / "test streaming" - demonstrate the streaming feature.
        if lower.contains("show streaming")
            || lower.contains("test streaming")
            || lower.contains("demonstrate streaming")
        {
            self.chat_ui.add_system_message("🎬 Streaming Demo: Switch to Mode 3 (Arbiter) and ask a question to see partial output appear as it generates.");
            return true;
        }

        // "who am i" / "what do you know about me" - show session memory.
        if lower.contains("who am i")
            || lower.contains("what do you know about me")
            || lower.contains("my preferences")
        {
            let mut info = Vec::new();
            if let Some(ref name) = self.session_memory.user_name {
                info.push(format!("Your name: {}", name));
            }
            if let Some(ref style) = self.session_memory.preferred_style {
                info.push(format!("Preferred style: {}", style));
            }
            if let Some(ref mood) = self.session_memory.user_mood {
                info.push(format!("Current mood: {}", mood));
            }
            if !self.session_memory.interests.is_empty() {
                info.push(format!(
                    "Interests: {}",
                    self.session_memory.interests.join(", ")
                ));
            }
            if !self.session_memory.avoid_topics.is_empty() {
                info.push(format!(
                    "Topics to avoid: {}",
                    self.session_memory.avoid_topics.join(", ")
                ));
            }
            info.push(format!(
                "Rapport level: {} ({}%)",
                self.session_memory.rapport_tier(),
                self.session_memory.rapport
            ));
            info.push(format!(
                "Exchanges this session: {}",
                self.session_memory.exchange_count
            ));
            if !self.session_memory.recent_turns.is_empty() {
                info.push(format!(
                    "Conversation turns tracked: {}",
                    self.session_memory.recent_turns.len()
                ));
            }

            if info.len() > 2 {
                let rapport_comment = match self.session_memory.rapport_tier() {
                    "stranger" => "We're just getting started!",
                    "acquaintance" => "We're building a connection.",
                    "friend" => "We've got a good rapport going.",
                    "trusted" => "I feel like we really understand each other.",
                    _ => "",
                };
                self.chat_ui.add_npc_message(&format!(
                    "Here's what I remember about you:\n• {}\n\n{}",
                    info.join("\n• "),
                    rapport_comment
                ));
            } else {
                self.chat_ui.add_npc_message("I don't know much about you yet. Tell me your name, how you're feeling, or what you're interested in!");
            }
            return true;
        }

        // "reset memory" / "forget me" - clear session memory.
        if lower.contains("reset memory")
            || lower.contains("forget me")
            || lower.contains("start fresh")
        {
            self.session_memory = SessionMemory::default();
            self.chat_ui
                .add_npc_message("Done — I've cleared my memory of our conversation. Fresh start!");
            return true;
        }

        false
    }

    fn process_user_input(&mut self, text: String) {
        // Update session memory with any info from this input.
        self.session_memory.update_from_input(&text);

        // Store the user's input for contextual response generation
        self.last_user_input = text.clone();

        // Check for demo choreography commands first.
        if self.check_choreography(&text) {
            return;
        }

        match self.chat_ui.mode {
            DemoMode::PureGoap => {
                // Check if this is a follow-up question.
                let is_followup = self.session_memory.is_followup(&text);

                // Instant GOAP response
                let mut response = self
                    .dialogue_bank
                    .goap_response_string(&text)
                    .unwrap_or_else(|| {
                        self.dialogue_bank
                            .goap_fallback_contextual(&text)
                            .to_string()
                    });

                // If follow-up and we have context, acknowledge it.
                if is_followup {
                    if self.session_memory.recent_turns.last().is_some() {
                        // Reference previous context.
                        let prefix = if text.to_lowercase().contains("why") {
                            "Building on that — "
                        } else if text.to_lowercase().contains("how") {
                            "To elaborate — "
                        } else if text.to_lowercase().contains("more") {
                            "Adding more detail — "
                        } else {
                            "Continuing from before — "
                        };
                        response = format!("{}{}", prefix, response);
                    }
                }

                // Add mood-aware prefix for emotional attunement.
                let response = if let Some(ref mood) = self.session_memory.user_mood {
                    let prefix = match mood.as_str() {
                        "frustrated" => "I hear your frustration. ",
                        "tired" => "Let's keep this simple. ",
                        "anxious" => "It's okay, we'll take it slow. ",
                        "confused" => "Let me clarify: ",
                        "sad" => "I'm here with you. ",
                        "excited" => "Love the energy! ",
                        "happy" => "Great to hear! ",
                        "curious" => "Good question! ",
                        "calm" => "",
                        "energetic" => "Alright! ",
                        "polite" => "",
                        _ => "",
                    };
                    if !prefix.is_empty() && self.session_memory.exchange_count % 2 == 0 {
                        format!("{}{}", prefix, response)
                    } else {
                        response
                    }
                } else {
                    response
                };

                // Personalize if we know the user's name.
                let response = if let Some(ref name) = self.session_memory.user_name {
                    if !response.contains(name) && self.session_memory.exchange_count % 3 == 0 {
                        format!(
                            "{}, {}",
                            name,
                            response
                                .chars()
                                .next()
                                .unwrap_or(' ')
                                .to_lowercase()
                                .to_string()
                                + &response[1..]
                        )
                    } else {
                        response
                    }
                } else {
                    response
                };

                // Add rapport-based warmth.
                let response = match self.session_memory.rapport_tier() {
                    "trusted" if self.session_memory.exchange_count % 4 == 0 => {
                        format!("{} (You know I've got your back.)", response)
                    }
                    "friend" if self.session_memory.exchange_count % 5 == 0 => {
                        format!("{} Happy to help anytime.", response)
                    }
                    _ => response,
                };

                // Record this turn for multi-turn context.
                self.session_memory.record_turn(&text, &response);

                self.chat_ui.add_npc_message(&response);
                self.npc_state.dialogue_state = NpcDialogueState::Speaking {
                    text: response,
                    started_at: Instant::now(),
                };
            }
            DemoMode::PureLlm => {
                // Start real LLM request via Qwen3-8B
                self.llm_pending = true;
                self.llm_request_time = Some(Instant::now());
                self.chat_ui.set_npc_typing(true);
                self.npc_state.dialogue_state = NpcDialogueState::Thinking {
                    started_at: Instant::now(),
                };
                // Launch async LLM request with modifiers.
                self.start_llm_request(&text);
            }
            DemoMode::Arbiter => {
                // Arbiter: Immediate contextual filler + background LLM with streaming.
                self.llm_pending = true;
                self.llm_request_time = Some(Instant::now());

                // Generate CONTEXTUAL thinking fillers based on user input
                self.current_fillers = self.dialogue_bank.thinking_sequence_contextual_for_mode(
                    DemoMode::Arbiter,
                    10.0,
                    &text,
                );

                // If GOAP can confidently answer quickly, prepend that as the first filler.
                if let Some(quick) = self.dialogue_bank.goap_response_string(&text) {
                    self.current_fillers
                        .insert(0, format!("Quick take: {}", quick));
                }
                self.filler_index = 0;

                // Show first contextual filler immediately
                if let Some(filler) = self.current_fillers.first() {
                    self.chat_ui.add_npc_message(filler);
                    self.filler_index = 1;
                }

                self.npc_state.dialogue_state = NpcDialogueState::Thinking {
                    started_at: Instant::now(),
                };
                // Launch async LLM request with streaming support.
                self.start_llm_request_streaming(&text);
            }
        }
    }

    fn update(&mut self, dt: f32) {
        // Update camera position and view matrix in renderer
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.renderer.update_view(&self.camera.to_render_view());

        // Update visible instances each frame (spheres in scene)
        self.renderer.update_instances(&self.demo_instances);

        // Update water renderer for animated ocean
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let view_proj = self.camera.vp();
        self.renderer
            .update_water(view_proj, self.camera.position, elapsed);

        // Check for LLM response from async task
        if self.llm_pending {
            if let Some(ref mut receiver) = self.llm_response_receiver {
                // Try to receive without blocking
                match receiver.try_recv() {
                    Ok(response) => {
                        // LLM response received!
                        self.llm_pending = false;
                        self.llm_request_time = None;
                        self.chat_ui.set_npc_typing(false);
                        self.llm_response_receiver = None;

                        // Record turn for multi-turn context.
                        self.session_memory
                            .record_turn(&self.last_user_input, &response);

                        self.chat_ui.add_npc_message(&response);
                        self.npc_state.dialogue_state = NpcDialogueState::Speaking {
                            text: response,
                            started_at: Instant::now(),
                        };
                        self.current_fillers.clear();
                        self.filler_index = 0;
                    }
                    Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                        // Still waiting for response - show filler for Arbiter mode
                        if self.chat_ui.mode == DemoMode::Arbiter {
                            if let Some(start_time) = self.llm_request_time {
                                let elapsed = start_time.elapsed().as_secs_f32();
                                let expected_filler_index =
                                    ((elapsed / 3.0) as usize).min(self.current_fillers.len());
                                while self.filler_index < expected_filler_index {
                                    if let Some(filler) =
                                        self.current_fillers.get(self.filler_index)
                                    {
                                        self.chat_ui.add_npc_message(filler);
                                    }
                                    self.filler_index += 1;
                                }
                            }
                        }
                    }
                    Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                        // Channel closed - LLM request failed, use fallback
                        self.llm_pending = false;
                        self.llm_request_time = None;
                        self.chat_ui.set_npc_typing(false);
                        self.llm_response_receiver = None;
                        self.llm_stream_receiver = None;

                        let fallback = self
                            .dialogue_bank
                            .fallback_response_for_mode(self.chat_ui.mode, &self.last_user_input);
                        self.chat_ui.add_npc_message(&fallback);
                        self.npc_state.dialogue_state = NpcDialogueState::Speaking {
                            text: fallback,
                            started_at: Instant::now(),
                        };
                        self.current_fillers.clear();
                        self.filler_index = 0;
                        self.last_streamed_text.clear();
                    }
                }
            }

            // Check for streaming partial output (Arbiter mode shows this progressively).
            if self.chat_ui.mode == DemoMode::Arbiter {
                if let Some(ref stream_rx) = self.llm_stream_receiver {
                    // Drain all available partial updates.
                    while let Ok(partial) = stream_rx.try_recv() {
                        // Only update if the partial is longer than what we've shown.
                        if partial.len() > self.last_streamed_text.len() + 10 {
                            // Show a "typing" indicator with partial text.
                            let preview = if partial.len() > 80 {
                                format!("{}...", &partial[..77])
                            } else {
                                partial.clone()
                            };
                            self.chat_ui.set_typing_preview(&preview);
                            self.last_streamed_text = partial;
                        }
                    }
                }
            }
        }

        // Update environment (sky, time of day)
        self.renderer.tick_environment(dt);
    }

    /// Start an async LLM request using Qwen3-8B (with modifiers from input).
    fn start_llm_request(&mut self, user_input: &str) {
        // Build prompt with session context.
        let context = self.session_memory.to_context_prefix();
        let prompt = if context.is_empty() {
            user_input.to_string()
        } else {
            format!("[Context: {}]\n\n{}", context, user_input)
        };
        self.llm_response_receiver =
            Some(self.llm_worker.request_with_modifiers(prompt, user_input));
    }

    /// Start an async LLM request with streaming support (for Arbiter mode).
    fn start_llm_request_streaming(&mut self, user_input: &str) {
        // Build prompt with session context.
        let context = self.session_memory.to_context_prefix();
        let prompt = if context.is_empty() {
            user_input.to_string()
        } else {
            format!("[Context: {}]\n\n{}", context, user_input)
        };
        let (final_rx, stream_rx) = self.llm_worker.request_streaming(prompt, user_input);
        self.llm_response_receiver = Some(final_rx);
        self.llm_stream_receiver = Some(stream_rx);
        self.last_streamed_text.clear();
    }

    fn complete_llm_response(&mut self) {
        self.llm_pending = false;
        self.llm_request_time = None;
        self.chat_ui.set_npc_typing(false);

        // Generate contextual response based on user input
        let response = self
            .dialogue_bank
            .fallback_response_for_mode(self.chat_ui.mode, &self.last_user_input);

        self.chat_ui.add_npc_message(&response);
        self.npc_state.dialogue_state = NpcDialogueState::Speaking {
            text: response.clone(),
            started_at: Instant::now(),
        };
        self.current_fillers.clear();
        self.filler_index = 0;
    }

    /// Generate a contextual response based on user input
    /// This simulates intelligent LLM behavior by analyzing keywords
    fn generate_contextual_response(&self, input: &str) -> String {
        self.dialogue_bank
            .fallback_response_for_mode(self.chat_ui.mode, input)
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Update camera and water in renderer
        self.renderer.update_view(&self.camera.to_render_view());
        self.renderer.update_water(
            self.camera.vp(),
            self.camera.position,
            self.start_time.elapsed().as_secs_f32(),
        );

        // Prepare egui input and run UI logic
        let raw_input = self.egui_state.take_egui_input(&self.window);

        // Use Cell to capture chat input from egui closure
        let pending_input = std::cell::RefCell::new(None);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Draw our chat UI and capture any submitted text
            if let Some(text) = self.chat_ui.draw(ctx) {
                *pending_input.borrow_mut() = Some(text);
            }
            chat_ui::draw_help_overlay(ctx);
        });

        // Process any pending chat input
        if let Some(text) = pending_input.into_inner() {
            self.process_user_input(text);
        }

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        // Tessellate egui (prepare before render_with)
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let size = self.window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        // Update egui textures before render_with
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.borrow_mut().update_texture(
                self.renderer.device(),
                self.renderer.queue(),
                *id,
                image_delta,
            );
        }

        // Get reference to egui_renderer for use in closure
        let egui_renderer = &self.egui_renderer;

        // Use render_with to test minimal draw_into (debugging)
        let result = self
            .renderer
            .render_with(|view, encoder, device, queue, _size| {
                // Update egui buffers within the render call
                egui_renderer.borrow_mut().update_buffers(
                    device,
                    queue,
                    encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                // Render egui on top of the 3D scene
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Egui Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Load the 3D scene
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

        // Free egui textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.borrow_mut().free_texture(id);
        }

        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Render error: {e:?}");
                return Err(wgpu::SurfaceError::Lost);
            }
        }

        self.frame_count += 1;
        Ok(())
    }
}

/// Application handler for winit
struct AppHandler {
    app: Option<CompanionApp>,
    pending_text: Option<String>,
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("Hello Companion - AI Demo")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

            self.app = Some(pollster::block_on(CompanionApp::new(window)).unwrap());
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

                // Process any pending input
                if let Some(text) = self.pending_text.take() {
                    app.process_user_input(text);
                }

                app.update(dt);

                match app.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => app.resize(app.window.inner_size()),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("Render error: {:?}", e),
                }

                app.window.request_redraw();
            }
            _ => {
                if !app.handle_input(&event) {
                    // Event not consumed
                }
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
                    // Direct camera manipulation - no smoothing = no drift
                    let sens = 0.004;
                    // With flipped up vector (-Y), negate yaw: mouse right = turn right
                    app.camera.yaw -= delta.0 as f32 * sens;
                    // Inverted: mouse up = look up (positive pitch change)
                    app.camera.pitch = (app.camera.pitch + delta.1 as f32 * sens).clamp(-1.5, 1.5);
                }
            }
        }
    }
}

/// Main entry point for visual demo
pub fn run_visual_demo() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   Hello Companion - Visual AI Demo                         ║");
    println!("║   Press 1/2/3 to switch AI modes                           ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut handler = AppHandler {
        app: None,
        pending_text: None,
    };

    event_loop
        .run_app(&mut handler)
        .context("Event loop error")?;
    Ok(())
}
