//! Shared viewport types used by both the engine adapter and editor panels.
//!
//! These types were extracted from the individual editor renderers so they
//! can be referenced independently of any particular rendering backend.

use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};

// ─── Terrain Vertex (GPU format) ─────────────────────────────────────────────

/// Terrain vertex in the editor's GPU format.
///
/// This is the *viewport* vertex layout consumed by both the engine adapter
/// (which converts it to engine meshes) and the legacy terrain renderer.
/// A separate `terrain_integration::TerrainVertex` exists for the CPU side.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_weights_0: [f32; 4],
    pub biome_weights_1: [f32; 4],
    /// Material texture layer indices (0-21) packed as f32 for vertex attr compat
    pub material_ids: [f32; 4],
    /// Blend weights for each material slot (sum to 1.0)
    pub material_weights: [f32; 4],
}

// ─── Fog Parameters ──────────────────────────────────────────────────────────

/// Fog and weather parameters passed to terrain/scene shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainFogParams {
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_color: [f32; 3],
    pub weather_type: u32,
    /// Optional override for particle count (None = use default for weather type)
    pub particle_count_override: Option<u32>,
}

impl Default for TerrainFogParams {
    fn default() -> Self {
        Self {
            fog_enabled: false,
            fog_density: 0.01,
            particle_count_override: None,
            fog_color: [0.6, 0.6, 0.62],
            weather_type: 0,
        }
    }
}

// ─── Lighting Parameters ─────────────────────────────────────────────────────

/// Lighting parameters passed to terrain/scene shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainLightingParams {
    pub sun_dir: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub exposure: f32,
}

impl Default for TerrainLightingParams {
    fn default() -> Self {
        Self {
            sun_dir: [0.5, 0.7, 0.35],
            sun_color: [1.0, 0.95, 0.85],
            sun_intensity: 1.8,
            ambient_color: [0.55, 0.52, 0.48],
            ambient_intensity: 0.35,
            exposure: 1.1,
        }
    }
}

// ─── Water Style ─────────────────────────────────────────────────────────────

/// Water style presets for different biome types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaterStyle {
    Ocean,
    River,
    Lake,
    Swamp,
}

// ─── Weather Kind ────────────────────────────────────────────────────────────

/// Weather type constants for the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WeatherKind {
    None = 0,
    Rain = 1,
    Snow = 2,
    Hail = 3,
    Sandstorm = 4,
    Blizzard = 5,
}

impl WeatherKind {
    pub fn from_weather_type(weather_type: u32) -> Self {
        match weather_type {
            0 | 1 | 5 => WeatherKind::None, // Clear, Cloudy, Fog
            2 => WeatherKind::Rain,         // Rain
            3 => WeatherKind::Rain,         // Storm (heavy rain)
            4 => WeatherKind::Snow,         // Snow
            6 => WeatherKind::Sandstorm,    // Sandstorm
            _ => WeatherKind::None,
        }
    }

    /// Map the 11-type world_panel weather to WeatherKind
    pub fn from_world_panel(weather_type: u32) -> Self {
        match weather_type {
            0..=2 | 8 => WeatherKind::None, // Clear, Cloudy, Overcast, Fog
            3 => WeatherKind::Rain,         // LightRain
            4 | 5 => WeatherKind::Rain,     // HeavyRain, Thunderstorm
            6 => WeatherKind::Snow,         // Snow
            7 => WeatherKind::Blizzard,     // Blizzard
            9 => WeatherKind::Sandstorm,    // Sandstorm
            10 => WeatherKind::Hail,        // Hail
            _ => WeatherKind::None,
        }
    }
}

// ─── Scatter Placement (re-export) ───────────────────────────────────────────

pub use crate::terrain_integration::ScatterPlacement;

// ─── Material Constants ──────────────────────────────────────────────────────

/// Canonical material names matching texture array layer ordering.
/// Layers 0-7 match biome indices for backward compatibility.
pub const MATERIAL_NAMES: [&str; 22] = [
    "grass",         //  0: Grassland biome
    "sand",          //  1: Desert biome (also Beach via remap)
    "forest_floor",  //  2: Forest biome
    "mountain_rock", //  3: Mountain biome
    "snow",          //  4: Tundra biome
    "mud",           //  5: Swamp biome
    "wood_planks",   //  6: (was sand dupe; Beach biome remapped to 1)
    "stone",         //  7: River biome
    "rock_slate",    //  8: steep rock
    "dirt",          //  9: dirt breakup
    "cobblestone",   // 10
    "cloth",         // 11
    "default",       // 12
    "gravel",        // 13
    "ice",           // 14
    "metal_rusted",  // 15
    "moss",          // 16
    "plaster",       // 17
    "rock_lichen",   // 18
    "roof_tile",     // 19
    "tree_bark",     // 20
    "tree_leaves",   // 21
];

pub const MATERIAL_DISPLAY_NAMES: [&str; 22] = [
    "Grass",
    "Sand",
    "Forest Floor",
    "Mountain Rock",
    "Snow",
    "Mud",
    "Wood Planks",
    "Stone",
    "Rock Slate",
    "Dirt",
    "Cobblestone",
    "Cloth",
    "Default",
    "Gravel",
    "Ice",
    "Metal Rusted",
    "Moss",
    "Plaster",
    "Rock Lichen",
    "Roof Tile",
    "Tree Bark",
    "Tree Leaves",
];

// ─── Asset Directory Discovery ───────────────────────────────────────────────

/// Locate the project `assets/` directory by searching from CWD and walking up
/// from the executable location.
pub fn find_assets_dir() -> PathBuf {
    // Try working directory first
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("assets/materials/grass.png").exists() {
        tracing::info!(
            "[terrain] Assets dir resolved via CWD: {:?}",
            cwd.join("assets")
        );
        return cwd.join("assets");
    }
    // Walk up from executable location
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("assets/materials/grass.png").exists() {
                tracing::info!(
                    "[terrain] Assets dir resolved via exe walk-up: {:?}",
                    d.join("assets")
                );
                return d.join("assets");
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    // Fallback — warn loudly since textures will likely fail
    tracing::warn!(
        "[terrain] Could not locate assets directory! \
         Checked CWD ({:?}) and walked up from executable. \
         Falling back to relative 'assets/' — textures will likely fail to load.",
        cwd,
    );
    PathBuf::from("assets")
}

// ─── Scene Light ────────────────────────────────────────────────────────────

/// A point light extracted from entity components for scene lighting.
///
/// Previously defined in `entity_renderer.rs`; moved here so it can be shared
/// across the editor without depending on the legacy entity renderer.
#[derive(Clone, Debug)]
pub struct SceneLight {
    pub position: [f32; 3],
    pub range: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

// ─── GLTF Animation Types ───────────────────────────────────────────────────

/// A single joint in a skeleton hierarchy (extracted from glTF skin data).
#[derive(Clone, Debug)]
pub struct GltfJoint {
    /// Joint name (from the glTF node).
    pub name: String,
    /// Parent joint index in the skeleton's `joints` array, or `None` for roots.
    pub parent_index: Option<usize>,
    /// Inverse bind matrix (transforms from mesh space to joint-local space).
    pub inverse_bind_matrix: glam::Mat4,
    /// Local (rest-pose) transform of the joint.
    pub local_transform: glam::Mat4,
}

/// A skeleton extracted from a glTF skin.
#[derive(Clone, Debug)]
pub struct GltfSkeleton {
    /// Ordered joint list (index matches glTF skin joint order).
    pub joints: Vec<GltfJoint>,
    /// Indices of root joints (joints with no parent).
    pub root_indices: Vec<usize>,
}

/// Keyframe interpolation mode.
#[derive(Clone, Copy, Debug)]
pub enum GltfInterpolation {
    Linear,
    Step,
    CubicSpline,
}

/// Channel target property being animated.
#[derive(Clone, Copy, Debug)]
pub enum GltfChannelProperty {
    Translation,
    Rotation,
    Scale,
}

/// A single animation channel targeting one joint.
#[derive(Clone, Debug)]
pub struct GltfAnimChannel {
    /// Joint index in the skeleton.
    pub joint_index: usize,
    /// Property being animated.
    pub property: GltfChannelProperty,
    /// Keyframe timestamps in seconds.
    pub times: Vec<f32>,
    /// Keyframe values (3 floats for translation/scale, 4 for rotation quaternion).
    pub values: Vec<Vec<f32>>,
    /// Interpolation mode.
    pub interpolation: GltfInterpolation,
}

/// An animation clip extracted from a glTF animation.
#[derive(Clone, Debug)]
pub struct GltfAnimationClip {
    /// Clip name.
    pub name: String,
    /// Duration in seconds.
    pub duration: f32,
    /// Animation channels.
    pub channels: Vec<GltfAnimChannel>,
}
