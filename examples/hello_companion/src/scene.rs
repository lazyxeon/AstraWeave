//! Scene module for hello_companion visual demo
//!
//! Handles loading and rendering of the 3D scene:
//! - Terrain with grass texture
//! - House model
//! - NPC characters (Amber for Arbiter, character-a for LLM/GOAP)
//! - Mode-specific HDRI skyboxes

#![allow(dead_code)] // Scene assets and configs reserved for full 3D rendering

use glam::Vec3;

/// AI mode determines which NPC and skybox to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoMode {
    /// Pure LLM mode - character-a, venice sunset
    PureLlm,
    /// Pure GOAP mode - character-a, kloppenheim clear day
    PureGoap,
    /// Arbiter hybrid mode - Amber, spruit sunrise
    Arbiter,
}

impl DemoMode {
    /// Get HDRI path for this mode
    pub fn hdri_path(&self) -> &'static str {
        match self {
            DemoMode::PureLlm => "assets/hdri/polyhaven/venice_sunset/venice_sunset_2k.hdr",
            DemoMode::PureGoap => "assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr",
            DemoMode::Arbiter => "assets/hdri/polyhaven/spruit_sunrise/spruit_sunrise_2k.hdr",
        }
    }

    /// Get NPC model path for this mode
    pub fn npc_model_path(&self) -> &'static str {
        match self {
            DemoMode::PureLlm | DemoMode::PureGoap => "assets/models/character-a.glb",
            DemoMode::Arbiter => "assets/models/Amber-Npc/Amber.Fbx",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            DemoMode::PureLlm => "Pure LLM (Qwen)",
            DemoMode::PureGoap => "Pure GOAP (Scripted)",
            DemoMode::Arbiter => "Arbiter (GOAP + LLM Hybrid)",
        }
    }

    /// Keyboard shortcut
    pub fn hotkey(&self) -> char {
        match self {
            DemoMode::PureLlm => '1',
            DemoMode::PureGoap => '2',
            DemoMode::Arbiter => '3',
        }
    }
}

/// Scene configuration
pub struct SceneConfig {
    /// House position
    pub house_position: Vec3,
    /// NPC spawn position
    pub npc_position: Vec3,
    /// Camera start position
    pub camera_position: Vec3,
    /// Camera look-at target
    pub camera_target: Vec3,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            house_position: Vec3::new(5.0, 0.0, 5.0),
            npc_position: Vec3::new(3.0, 0.0, 3.0),
            camera_position: Vec3::new(-5.0, 3.0, -5.0),
            camera_target: Vec3::new(3.0, 1.0, 3.0),
        }
    }
}

/// Asset paths
pub mod assets {
    /// House model
    pub const HOUSE: &str = "assets/house1.glb";

    /// Terrain textures
    pub mod terrain {
        pub const GRASS_ALBEDO: &str = "assets/grass.png";
        pub const GRASS_NORMAL: &str = "assets/grass_n.png";
        pub const GRASS_MRA: &str = "assets/grass_mra.png";
    }

    /// Character models
    pub mod characters {
        pub const CHARACTER_A: &str = "assets/models/character-a.glb";
        pub const AMBER: &str = "assets/models/Amber-Npc/Amber.Fbx";
        pub const AMBER_MOTION: &str = "assets/models/Amber-Npc/Amber_Motion.Fbx";
    }

    /// HDRI skyboxes
    pub mod hdri {
        pub const SPRUIT_SUNRISE: &str =
            "assets/hdri/polyhaven/spruit_sunrise/spruit_sunrise_2k.hdr";
        pub const VENICE_SUNSET: &str = "assets/hdri/polyhaven/venice_sunset/venice_sunset_2k.hdr";
        pub const KLOPPENHEIM: &str = "assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr";
    }
}

/// Vertex for terrain rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl TerrainVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Generate a flat terrain mesh
pub fn generate_terrain_mesh(size: f32, subdivisions: u32) -> (Vec<TerrainVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let step = size / subdivisions as f32;
    let half = size / 2.0;
    let uv_scale = size / 4.0; // Repeat texture every 4 units

    for z in 0..=subdivisions {
        for x in 0..=subdivisions {
            let px = -half + x as f32 * step;
            let pz = -half + z as f32 * step;

            vertices.push(TerrainVertex {
                position: [px, 0.0, pz],
                normal: [0.0, 1.0, 0.0],
                uv: [px / uv_scale, pz / uv_scale],
            });
        }
    }

    let row_size = subdivisions + 1;
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let i = z * row_size + x;

            // First triangle
            indices.push(i);
            indices.push(i + row_size);
            indices.push(i + 1);

            // Second triangle
            indices.push(i + 1);
            indices.push(i + row_size);
            indices.push(i + row_size + 1);
        }
    }

    (vertices, indices)
}

/// NPC state for animations and dialogue
#[derive(Debug, Clone)]
pub struct NpcState {
    pub position: Vec3,
    pub rotation: f32,
    pub animation_state: NpcAnimationState,
    pub dialogue_state: NpcDialogueState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcAnimationState {
    Idle,
    Walking,
    Thinking,
    Talking,
    Gesturing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NpcDialogueState {
    /// NPC is idle, no conversation
    Idle,
    /// Waiting for user input
    Listening,
    /// Thinking/processing (filler lines)
    Thinking { started_at: std::time::Instant },
    /// Speaking a response
    Speaking {
        text: String,
        started_at: std::time::Instant,
    },
}

impl Default for NpcState {
    fn default() -> Self {
        Self {
            position: Vec3::new(3.0, 0.0, 3.0),
            rotation: 0.0,
            animation_state: NpcAnimationState::Idle,
            dialogue_state: NpcDialogueState::Idle,
        }
    }
}
