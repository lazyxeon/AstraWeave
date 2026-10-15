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
///
/// Real-Fix.C 2026-05-08: unified `biome_weights_0/1` and `material_ids/
/// material_weights` into a single canonical material attribute set
/// (Option C per Andrew-gate decision). Resolves §7.7 sibling-attribute
/// drift trap at texture-data layer (Round 7 evidence). Splat textures are
/// rebuilt directly from `material_ids/material_weights`; biome blending
/// at higher abstraction layers (astraweave-terrain) preserved per Model A.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Material texture layer indices (0-7 valid; mapped to 8-channel splat)
    /// packed as f32 for vertex attribute compatibility.
    pub material_ids: [f32; 4],
    /// Blend weights for each material slot (sum to 1.0).
    pub material_weights: [f32; 4],
}

impl TerrainVertex {
    /// Convert to engine-compatible vertex by extracting the dominant material
    /// slot. The engine uses a single biome_id per vertex; post-Real-Fix.C
    /// this is the material_id of the highest-weight slot.
    pub fn to_engine_vertex(self) -> astraweave_render::TerrainVertex {
        let mut best_idx = 0usize;
        let mut best_weight = self.material_weights[0];
        for i in 1..4 {
            if self.material_weights[i] > best_weight {
                best_weight = self.material_weights[i];
                best_idx = i;
            }
        }

        astraweave_render::TerrainVertex {
            position: self.position,
            normal: self.normal,
            uv: self.uv,
            biome_id: self.material_ids[best_idx] as u32,
        }
    }
}

// ─── Fog Parameters ──────────────────────────────────────────────────────────

/// Fog and weather parameters passed to terrain/scene shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainFogParams {
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_color: [f32; 3],
    pub weather_type: u32,
    /// Optional override for particle count (None = use default for weather type)
    pub particle_count_override: Option<u32>,
}

impl Default for TerrainFogParams {
    fn default() -> Self {
        Self {
            fog_enabled: false,
            fog_density: 0.0,
            fog_start: 800.0,
            fog_end: 1800.0,
            particle_count_override: None,
            fog_color: [0.6, 0.6, 0.62],
            weather_type: 0,
        }
    }
}

// ─── Lighting Parameters ─────────────────────────────────────────────────────

// L.1 pinned lighting defaults — THE DELIVERED STATE, single source of truth.
//
// Pre-L.1 the World panel displayed one set of values (sun 2.2×) while the
// renderer delivered another (sun 1.0×), because the frame-1 push was
// silently dropped in the adapter-init race (T2F_LIGHTING_RECON.md §3).
// These constants pin the panel, `TerrainLightingParams::default()`, and the
// engine adapter's terrain-upload block to the state the renderer actually
// holds at defaults, so the UI tells the truth and pushing the defaults is
// visually neutral (proven byte-identical by `l1_proof.rs`).

/// Direction TO the sun. Negated + normalized this reproduces the terrain-
/// upload block's hardcoded light direction `normalize(-0.5, -0.6, -0.4)`
/// bit-for-bit (same `Vec3::normalize` on exactly negated components).
pub const DEFAULT_SUN_DIR: [f32; 3] = [0.5, 0.6, 0.4];
/// `SceneEnvironment::default()`'s sun colour (astraweave-render).
pub const DEFAULT_SUN_COLOR: [f32; 3] = [1.0, 0.98, 0.9];
/// `SceneEnvironment::default()`'s sun intensity.
pub const DEFAULT_SUN_INTENSITY: f32 = 1.0;
/// The terrain-upload block's ambient (engine_adapter.rs; the value every
/// T-series judgment was made under).
pub const DEFAULT_AMBIENT_COLOR: [f32; 3] = [0.45, 0.50, 0.55];
pub const DEFAULT_AMBIENT_INTENSITY: f32 = 0.35;
/// Mirrors `astraweave_render::scene_environment::DEFAULT_EXPOSURE` (the
/// pre-L.1 POST-shader hardcode); equality is asserted by test.
pub const DEFAULT_EXPOSURE: f32 = 1.35;
/// `DEFAULT_SUN_DIR` expressed in the World panel's elevation/azimuth
/// degrees (asin(0.6/|v|) resp. atan2(0.4, 0.5)). The panel's trig
/// reconstruction lands within <0.1° of the hardcoded direction; the
/// delivered direction stays the terrain-upload hardcode until a slider is
/// touched.
pub const DEFAULT_SUN_ELEVATION_DEG: f32 = 43.15;
pub const DEFAULT_SUN_AZIMUTH_DEG: f32 = 38.66;

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
            sun_dir: DEFAULT_SUN_DIR,
            sun_color: DEFAULT_SUN_COLOR,
            sun_intensity: DEFAULT_SUN_INTENSITY,
            ambient_color: DEFAULT_AMBIENT_COLOR,
            ambient_intensity: DEFAULT_AMBIENT_INTENSITY,
            exposure: DEFAULT_EXPOSURE,
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

impl WaterStyle {
    /// Parse the style name stored in a `WaterVolume` entity component
    /// (TW2). Unknown/missing strings fall back to `Lake` — the calm
    /// authored-volume default.
    pub fn from_component_str(s: &str) -> Self {
        match s {
            "Ocean" => WaterStyle::Ocean,
            "River" => WaterStyle::River,
            "Swamp" => WaterStyle::Swamp,
            _ => WaterStyle::Lake,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            WaterStyle::Ocean => "Ocean",
            WaterStyle::River => "River",
            WaterStyle::Lake => "Lake",
            WaterStyle::Swamp => "Swamp",
        }
    }
}

/// TW2: an authored water volume collected from a `WaterVolume` entity —
/// the editor-side spec the engine adapter resolves (style → colors/wave
/// params) into `astraweave_render::WaterVolumeDesc`. The entity's world
/// position carries the surface: its Y is the volume's water level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterVolumeSpec {
    /// Entity world position; `[1]` (Y) is the volume's surface level.
    pub position: [f32; 3],
    pub half_extent_x: f32,
    pub half_extent_z: f32,
    pub style: WaterStyle,
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
//
// Real-Fix.D 2026-05-08: re-exported from `astraweave_render::material_library`
// per Andrew-gate decision (h) Option D-2 (canonical material library).
// Identity unified at the UI/renderer boundary; both UI panel material list
// AND renderer texture-array allocation derive from the same canonical
// source. Use `astraweave_render::MaterialLibrary` for new code.

pub use astraweave_render::{MATERIAL_DISPLAY_NAMES, MATERIAL_NAMES};

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

// ─── L.1 defaults-honesty tests ──────────────────────────────────────────────

#[cfg(test)]
mod l1_lighting_defaults_tests {
    use super::*;

    /// The pinned sun direction must reproduce the terrain-upload block's
    /// hardcoded light direction BIT-FOR-BIT through the same
    /// `set_lighting_params` math (negate, then `Vec3::normalize`).
    #[test]
    fn l1_pinned_sun_dir_matches_upload_hardcode_bitwise() {
        let via_params = (-glam::Vec3::from(DEFAULT_SUN_DIR)).normalize();
        let upload_hardcode = glam::Vec3::new(-0.5, -0.6, -0.4).normalize();
        assert_eq!(via_params.to_array(), upload_hardcode.to_array());
    }

    /// The editor-side exposure default must equal the render crate's
    /// canonical constant (the pre-L.1 POST-shader hardcode).
    #[test]
    fn l1_exposure_default_matches_render_crate() {
        assert_eq!(
            DEFAULT_EXPOSURE,
            astraweave_render::scene_environment::DEFAULT_EXPOSURE
        );
    }

    /// `TerrainLightingParams::default()` is the pinned delivered state —
    /// pre-L.1 it advertised sun 1.8× / exposure 1.1, values no path
    /// delivered (T2F §1.2 row 1, §3).
    #[test]
    fn l1_default_params_are_the_delivered_state() {
        let d = TerrainLightingParams::default();
        assert_eq!(d.sun_dir, DEFAULT_SUN_DIR);
        assert_eq!(d.sun_color, [1.0, 0.98, 0.9]);
        assert_eq!(d.sun_intensity, 1.0);
        assert_eq!(d.ambient_color, [0.45, 0.50, 0.55]);
        assert_eq!(d.ambient_intensity, 0.35);
        assert_eq!(d.exposure, 1.35);
    }

    /// The panel's elevation/azimuth defaults reconstruct the pinned sun
    /// direction to within 0.1° (the panel works in whole-degree sliders;
    /// the delivered direction remains the upload hardcode until touched).
    #[test]
    fn l1_elevation_azimuth_round_trip() {
        let elev = DEFAULT_SUN_ELEVATION_DEG.to_radians();
        let azim = DEFAULT_SUN_AZIMUTH_DEG.to_radians();
        let dir = glam::Vec3::new(elev.cos() * azim.cos(), elev.sin(), elev.cos() * azim.sin())
            .normalize();
        let pinned = glam::Vec3::from(DEFAULT_SUN_DIR).normalize();
        let angle_deg = dir.dot(pinned).clamp(-1.0, 1.0).acos().to_degrees();
        assert!(
            angle_deg < 0.1,
            "panel-derived direction is {angle_deg:.4}° off the pinned direction"
        );
    }
}
