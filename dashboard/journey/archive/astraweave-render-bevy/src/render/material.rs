// Material module - PBR materials (albedo, normal, MRA)
// Day 3 implementation

//! PBR material system

use glam::Vec4;

/// Standard PBR material (from Bevy)
#[derive(Debug, Clone)]
pub struct StandardMaterial {
    /// Base color (RGBA)
    pub base_color: Vec4,

    /// Base color texture handle
    pub base_color_texture: Option<TextureHandle>,

    /// Normal map texture handle
    pub normal_map_texture: Option<TextureHandle>,

    /// Metallic-roughness-ambient occlusion texture handle
    pub metallic_roughness_texture: Option<TextureHandle>,

    /// Metallic factor (0.0 = dielectric, 1.0 = conductor)
    pub metallic: f32,

    /// Perceptual roughness (0.0 = smooth, 1.0 = rough)
    pub perceptual_roughness: f32,

    /// Reflectance (F0 for dielectrics)
    pub reflectance: f32,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        Self {
            base_color: Vec4::ONE,
            base_color_texture: None,
            normal_map_texture: None,
            metallic_roughness_texture: None,
            metallic: 0.0,
            perceptual_roughness: 0.5,
            reflectance: 0.5,
        }
    }
}

/// Texture handle (placeholder)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

// Day 3: Add
// - Material GPU binding
// - Texture array management
// - Material batching
