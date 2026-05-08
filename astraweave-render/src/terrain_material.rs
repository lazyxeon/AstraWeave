// Phase PBR-F: Terrain Layering System with Splat Maps and Triplanar Projection
// Rust-side terrain material definitions for multi-layer terrain rendering
//
// Real-Fix.D 2026-05-08: bumped layer count from 8 to 32 per Andrew-gate
// decision (h) Option D-2 (canonical material library). The capacity is now
// driven by `crate::material_library::MAX_TERRAIN_LAYERS` (= 32).

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::material_library::MAX_TERRAIN_LAYERS;

/// GPU representation of a single terrain layer
/// Size: 64 bytes (16-byte aligned)
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TerrainLayerGpu {
    /// Texture indices: [albedo, normal, orm, height]
    pub texture_indices: [u32; 4], // 16 bytes (offset 0)

    /// UV scale for this layer (allows different tiling per layer)
    pub uv_scale: [f32; 2], // 8 bytes (offset 16)

    /// Height blend range: [min_height, max_height]
    /// Used for height-based layer transitions
    pub height_range: [f32; 2], // 8 bytes (offset 24)

    /// Blend sharpness: higher values = sharper transitions
    /// Range: [0.0, 1.0], default 0.5
    pub blend_sharpness: f32, // 4 bytes (offset 32)

    /// Triplanar blend power (steepness threshold)
    /// Higher values = more aggressive triplanar on slopes
    pub triplanar_power: f32, // 4 bytes (offset 36)

    /// Material properties: [metallic, roughness]
    pub material_factors: [f32; 2], // 8 bytes (offset 40)

    /// Padding to reach 64 bytes (48 bytes used, need 16 more)
    pub _pad: [u32; 4], // 16 bytes (offset 48)
}

impl Default for TerrainLayerGpu {
    fn default() -> Self {
        Self {
            texture_indices: [0, 0, 0, 0],
            uv_scale: [1.0, 1.0],
            height_range: [0.0, 100.0],
            blend_sharpness: 0.5,
            triplanar_power: 4.0,
            material_factors: [0.0, 0.5], // metallic=0, roughness=0.5
            _pad: [0, 0, 0, 0],
        }
    }
}

/// Extended terrain material supporting up to 32 layers with splat map blending.
/// Size: 2112 bytes (32 × 64 = 2048 for layers + 64 for common params).
///
/// Real-Fix.D 2026-05-08: bumped from 8 to 32 layers (canonical capacity from
/// `material_library::MAX_TERRAIN_LAYERS`). WGSL shaders
/// (`pbr_terrain.wgsl`, `pbr_terrain_forward.wgsl`) declare matching
/// `array<TerrainLayer, 32>`.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TerrainMaterialGpu {
    /// 32 terrain layers (canonical material slots; see
    /// `material_library::MaterialLibrary`).
    pub layers: [TerrainLayerGpu; MAX_TERRAIN_LAYERS as usize],

    /// Splat map 0 texture index (R=layer0, G=layer1, B=layer2, A=layer3)
    pub splat_map_index_0: u32,

    /// Splat map 1 texture index (R=layer4, G=layer5, B=layer6, A=layer7)
    pub splat_map_index_1: u32,

    /// Global UV scale for splat map sampling
    pub splat_uv_scale: f32,

    /// Triplanar enable flag (0=off, 1=on)
    pub triplanar_enabled: u32,

    /// Normal blend method: 0=Linear, 1=Reoriented Normal Mapping (RNM), 2=UDN
    pub normal_blend_method: u32,

    /// Global triplanar threshold (slope angle in degrees where triplanar kicks in)
    /// Default: 45.0 (typical steep slope threshold)
    pub triplanar_slope_threshold: f32,

    /// Height blend enable (use height maps for smoother transitions)
    pub height_blend_enabled: u32,

    /// Number of active layers (1-8)
    pub active_layer_count: u32,

    /// Padding to complete 64-byte common params block
    pub _pad: [u32; 8],
}

impl Default for TerrainMaterialGpu {
    fn default() -> Self {
        Self {
            layers: [TerrainLayerGpu::default(); MAX_TERRAIN_LAYERS as usize],
            splat_map_index_0: 0,
            splat_map_index_1: 0,
            splat_uv_scale: 1.0,
            triplanar_enabled: 1,   // Enable by default
            normal_blend_method: 1, // RNM by default (best quality)
            triplanar_slope_threshold: 45.0,
            height_blend_enabled: 1,
            active_layer_count: 4,
            _pad: [0; 8],
        }
    }
}

/// Rust-side terrain material with TOML support
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainLayerDesc {
    /// Layer name (e.g., "grass", "rock", "sand")
    pub name: String,

    /// Texture paths
    pub albedo: Option<PathBuf>,
    pub normal: Option<PathBuf>,
    pub orm: Option<PathBuf>,
    pub height: Option<PathBuf>,

    /// UV tiling scale
    #[serde(default = "default_uv_scale")]
    pub uv_scale: [f32; 2],

    /// Height range for automatic blending
    #[serde(default)]
    pub height_range: Option<[f32; 2]>,

    /// Blend sharpness (0.0-1.0)
    #[serde(default = "default_blend_sharpness")]
    pub blend_sharpness: f32,

    /// Triplanar power
    #[serde(default = "default_triplanar_power")]
    pub triplanar_power: f32,

    /// Material properties
    #[serde(default)]
    pub metallic: f32,

    #[serde(default = "default_roughness")]
    pub roughness: f32,
}

fn default_uv_scale() -> [f32; 2] {
    [1.0, 1.0]
}

fn default_blend_sharpness() -> f32 {
    0.5
}

fn default_triplanar_power() -> f32 {
    4.0
}

fn default_roughness() -> f32 {
    0.5
}

impl Default for TerrainLayerDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            albedo: None,
            normal: None,
            orm: None,
            height: None,
            uv_scale: default_uv_scale(),
            height_range: None,
            blend_sharpness: default_blend_sharpness(),
            triplanar_power: default_triplanar_power(),
            metallic: 0.0,
            roughness: default_roughness(),
        }
    }
}

/// Complete terrain material pack definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainMaterialDesc {
    /// Material name
    pub name: String,

    /// Biome identifier (grassland, desert, forest, etc.)
    pub biome: String,

    /// Splat map path (RGBA image defining layer weights)
    pub splat_map: Option<PathBuf>,

    /// Global splat UV scale
    #[serde(default = "default_splat_scale")]
    pub splat_uv_scale: f32,

    /// Triplanar settings
    #[serde(default = "default_triplanar_enabled")]
    pub triplanar_enabled: bool,

    #[serde(default = "default_triplanar_threshold")]
    pub triplanar_slope_threshold: f32,

    /// Normal blending: "linear", "rnm" (Reoriented Normal Mapping), "udn" (UDN)
    #[serde(default = "default_normal_blend")]
    pub normal_blend_method: String,

    /// Height-based blending
    #[serde(default = "default_height_blend")]
    pub height_blend_enabled: bool,

    /// Up to 4 layers
    pub layers: Vec<TerrainLayerDesc>,
}

fn default_splat_scale() -> f32 {
    1.0
}

fn default_triplanar_enabled() -> bool {
    true
}

fn default_triplanar_threshold() -> f32 {
    45.0
}

fn default_normal_blend() -> String {
    "rnm".to_string()
}

fn default_height_blend() -> bool {
    true
}

impl Default for TerrainMaterialDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            biome: String::new(),
            splat_map: None,
            splat_uv_scale: default_splat_scale(),
            triplanar_enabled: default_triplanar_enabled(),
            triplanar_slope_threshold: default_triplanar_threshold(),
            normal_blend_method: default_normal_blend(),
            height_blend_enabled: default_height_blend(),
            layers: Vec::new(),
        }
    }
}

impl TerrainMaterialDesc {
    /// Create a grassland terrain material (grass base + dirt + rock + sparse grass)
    pub fn grassland() -> Self {
        Self {
            name: "grassland_terrain".to_string(),
            biome: "grassland".to_string(),
            splat_map: Some(PathBuf::from("grassland_splat.png")),
            splat_uv_scale: 0.5,
            triplanar_enabled: true,
            triplanar_slope_threshold: 35.0,
            normal_blend_method: "rnm".to_string(),
            height_blend_enabled: true,
            layers: vec![
                TerrainLayerDesc {
                    name: "grass".to_string(),
                    albedo: Some(PathBuf::from("grass_albedo.png")),
                    normal: Some(PathBuf::from("grass_normal.png")),
                    orm: Some(PathBuf::from("grass_orm.png")),
                    height: Some(PathBuf::from("grass_height.png")),
                    uv_scale: [8.0, 8.0],
                    height_range: Some([0.0, 50.0]),
                    blend_sharpness: 0.6,
                    triplanar_power: 3.0,
                    metallic: 0.0,
                    roughness: 0.9,
                },
                TerrainLayerDesc {
                    name: "dirt".to_string(),
                    albedo: Some(PathBuf::from("dirt_albedo.png")),
                    normal: Some(PathBuf::from("dirt_normal.png")),
                    orm: Some(PathBuf::from("dirt_orm.png")),
                    height: Some(PathBuf::from("dirt_height.png")),
                    uv_scale: [6.0, 6.0],
                    height_range: Some([0.0, 100.0]),
                    blend_sharpness: 0.5,
                    triplanar_power: 4.0,
                    metallic: 0.0,
                    roughness: 0.8,
                },
                TerrainLayerDesc {
                    name: "rock".to_string(),
                    albedo: Some(PathBuf::from("rock_albedo.png")),
                    normal: Some(PathBuf::from("rock_normal.png")),
                    orm: Some(PathBuf::from("rock_orm.png")),
                    height: Some(PathBuf::from("rock_height.png")),
                    uv_scale: [4.0, 4.0],
                    height_range: Some([40.0, 100.0]),
                    blend_sharpness: 0.7,
                    triplanar_power: 5.0,
                    metallic: 0.0,
                    roughness: 0.7,
                },
                TerrainLayerDesc {
                    name: "sparse_grass".to_string(),
                    albedo: Some(PathBuf::from("sparse_grass_albedo.png")),
                    normal: Some(PathBuf::from("sparse_grass_normal.png")),
                    orm: Some(PathBuf::from("sparse_grass_orm.png")),
                    height: Some(PathBuf::from("sparse_grass_height.png")),
                    uv_scale: [10.0, 10.0],
                    height_range: Some([0.0, 30.0]),
                    blend_sharpness: 0.4,
                    triplanar_power: 2.0,
                    metallic: 0.0,
                    roughness: 0.95,
                },
            ],
        }
    }

    /// Create a desert terrain material (sand base + red sand + rock + cracked ground)
    pub fn desert() -> Self {
        Self {
            name: "desert_terrain".to_string(),
            biome: "desert".to_string(),
            splat_map: Some(PathBuf::from("desert_splat.png")),
            splat_uv_scale: 0.4,
            triplanar_enabled: true,
            triplanar_slope_threshold: 40.0,
            normal_blend_method: "rnm".to_string(),
            height_blend_enabled: true,
            layers: vec![
                TerrainLayerDesc {
                    name: "sand".to_string(),
                    albedo: Some(PathBuf::from("sand_albedo.png")),
                    normal: Some(PathBuf::from("sand_normal.png")),
                    orm: Some(PathBuf::from("sand_orm.png")),
                    height: Some(PathBuf::from("sand_height.png")),
                    uv_scale: [12.0, 12.0],
                    height_range: Some([0.0, 60.0]),
                    blend_sharpness: 0.3,
                    triplanar_power: 2.5,
                    metallic: 0.0,
                    roughness: 0.95,
                },
                TerrainLayerDesc {
                    name: "red_sand".to_string(),
                    albedo: Some(PathBuf::from("red_sand_albedo.png")),
                    normal: Some(PathBuf::from("red_sand_normal.png")),
                    orm: Some(PathBuf::from("red_sand_orm.png")),
                    height: Some(PathBuf::from("red_sand_height.png")),
                    uv_scale: [10.0, 10.0],
                    height_range: Some([0.0, 40.0]),
                    blend_sharpness: 0.4,
                    triplanar_power: 3.0,
                    metallic: 0.0,
                    roughness: 0.9,
                },
                TerrainLayerDesc {
                    name: "desert_rock".to_string(),
                    albedo: Some(PathBuf::from("desert_rock_albedo.png")),
                    normal: Some(PathBuf::from("desert_rock_normal.png")),
                    orm: Some(PathBuf::from("desert_rock_orm.png")),
                    height: Some(PathBuf::from("desert_rock_height.png")),
                    uv_scale: [5.0, 5.0],
                    height_range: Some([50.0, 100.0]),
                    blend_sharpness: 0.8,
                    triplanar_power: 6.0,
                    metallic: 0.0,
                    roughness: 0.6,
                },
                TerrainLayerDesc {
                    name: "cracked_ground".to_string(),
                    albedo: Some(PathBuf::from("cracked_albedo.png")),
                    normal: Some(PathBuf::from("cracked_normal.png")),
                    orm: Some(PathBuf::from("cracked_orm.png")),
                    height: Some(PathBuf::from("cracked_height.png")),
                    uv_scale: [8.0, 8.0],
                    height_range: Some([0.0, 20.0]),
                    blend_sharpness: 0.6,
                    triplanar_power: 4.0,
                    metallic: 0.0,
                    roughness: 0.85,
                },
            ],
        }
    }

    /// Create a forest terrain material (moss + dirt + rock + leaves)
    pub fn forest() -> Self {
        Self {
            name: "forest_terrain".to_string(),
            biome: "forest".to_string(),
            splat_map: Some(PathBuf::from("forest_splat.png")),
            splat_uv_scale: 0.6,
            triplanar_enabled: true,
            triplanar_slope_threshold: 30.0,
            normal_blend_method: "rnm".to_string(),
            height_blend_enabled: true,
            layers: vec![
                TerrainLayerDesc {
                    name: "moss".to_string(),
                    albedo: Some(PathBuf::from("moss_albedo.png")),
                    normal: Some(PathBuf::from("moss_normal.png")),
                    orm: Some(PathBuf::from("moss_orm.png")),
                    height: Some(PathBuf::from("moss_height.png")),
                    uv_scale: [10.0, 10.0],
                    height_range: Some([0.0, 40.0]),
                    blend_sharpness: 0.5,
                    triplanar_power: 2.0,
                    metallic: 0.0,
                    roughness: 0.85,
                },
                TerrainLayerDesc {
                    name: "forest_dirt".to_string(),
                    albedo: Some(PathBuf::from("forest_dirt_albedo.png")),
                    normal: Some(PathBuf::from("forest_dirt_normal.png")),
                    orm: Some(PathBuf::from("forest_dirt_orm.png")),
                    height: Some(PathBuf::from("forest_dirt_height.png")),
                    uv_scale: [7.0, 7.0],
                    height_range: Some([0.0, 70.0]),
                    blend_sharpness: 0.6,
                    triplanar_power: 3.5,
                    metallic: 0.0,
                    roughness: 0.9,
                },
                TerrainLayerDesc {
                    name: "forest_rock".to_string(),
                    albedo: Some(PathBuf::from("forest_rock_albedo.png")),
                    normal: Some(PathBuf::from("forest_rock_normal.png")),
                    orm: Some(PathBuf::from("forest_rock_orm.png")),
                    height: Some(PathBuf::from("forest_rock_height.png")),
                    uv_scale: [5.0, 5.0],
                    height_range: Some([60.0, 100.0]),
                    blend_sharpness: 0.75,
                    triplanar_power: 5.5,
                    metallic: 0.0,
                    roughness: 0.65,
                },
                TerrainLayerDesc {
                    name: "leaf_litter".to_string(),
                    albedo: Some(PathBuf::from("leaves_albedo.png")),
                    normal: Some(PathBuf::from("leaves_normal.png")),
                    orm: Some(PathBuf::from("leaves_orm.png")),
                    height: Some(PathBuf::from("leaves_height.png")),
                    uv_scale: [12.0, 12.0],
                    height_range: Some([0.0, 30.0]),
                    blend_sharpness: 0.4,
                    triplanar_power: 2.5,
                    metallic: 0.0,
                    roughness: 0.95,
                },
            ],
        }
    }

    /// Parse normal blend method string to GPU constant
    pub fn normal_blend_to_gpu(&self) -> u32 {
        match self.normal_blend_method.to_lowercase().as_str() {
            "linear" => 0,
            "rnm" => 1,
            "udn" => 2,
            _ => 1, // Default to RNM
        }
    }

    /// Convert to GPU representation (requires texture index mapping)
    pub fn to_gpu(&self, texture_resolver: &dyn Fn(&PathBuf) -> u32) -> TerrainMaterialGpu {
        let mut gpu_material = TerrainMaterialGpu::default();

        // Splat map
        if let Some(splat_path) = &self.splat_map {
            gpu_material.splat_map_index_0 = texture_resolver(splat_path);
        }

        gpu_material.splat_uv_scale = self.splat_uv_scale;
        gpu_material.triplanar_enabled = if self.triplanar_enabled { 1 } else { 0 };
        gpu_material.normal_blend_method = self.normal_blend_to_gpu();
        gpu_material.triplanar_slope_threshold = self.triplanar_slope_threshold;
        gpu_material.height_blend_enabled = if self.height_blend_enabled { 1 } else { 0 };
        gpu_material.active_layer_count = self.layers.len().min(MAX_TERRAIN_LAYERS as usize) as u32;

        // Convert up to MAX_TERRAIN_LAYERS layers (Real-Fix.D 2026-05-08: 32)
        for (i, layer_desc) in self
            .layers
            .iter()
            .take(MAX_TERRAIN_LAYERS as usize)
            .enumerate()
        {
            let layer = &mut gpu_material.layers[i];

            // Texture indices
            if let Some(albedo) = &layer_desc.albedo {
                layer.texture_indices[0] = texture_resolver(albedo);
            }
            if let Some(normal) = &layer_desc.normal {
                layer.texture_indices[1] = texture_resolver(normal);
            }
            if let Some(orm) = &layer_desc.orm {
                layer.texture_indices[2] = texture_resolver(orm);
            }
            if let Some(height) = &layer_desc.height {
                layer.texture_indices[3] = texture_resolver(height);
            }

            layer.uv_scale = layer_desc.uv_scale;

            if let Some(height_range) = layer_desc.height_range {
                layer.height_range = height_range;
            }

            layer.blend_sharpness = layer_desc.blend_sharpness;
            layer.triplanar_power = layer_desc.triplanar_power;
            layer.material_factors = [layer_desc.metallic, layer_desc.roughness];
        }

        gpu_material
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_layer_size() {
        // Verify TerrainLayerGpu is exactly 64 bytes
        assert_eq!(std::mem::size_of::<TerrainLayerGpu>(), 64);
        assert_eq!(std::mem::align_of::<TerrainLayerGpu>(), 16);
    }

    #[test]
    fn test_terrain_material_size() {
        // Verify TerrainMaterialGpu is exactly 2112 bytes (32*64 + 64)
        // Real-Fix.D 2026-05-08: was 576 bytes (8*64+64); canonical
        // MAX_TERRAIN_LAYERS=32 grows the layer array.
        assert_eq!(std::mem::size_of::<TerrainMaterialGpu>(), 2112);
        assert_eq!(std::mem::align_of::<TerrainMaterialGpu>(), 16);
    }

    #[test]
    fn test_default_terrain_layer() {
        let layer = TerrainLayerGpu::default();
        assert_eq!(layer.uv_scale, [1.0, 1.0]);
        assert_eq!(layer.blend_sharpness, 0.5);
        assert_eq!(layer.triplanar_power, 4.0);
    }

    #[test]
    fn test_default_terrain_material() {
        let material = TerrainMaterialGpu::default();
        assert_eq!(material.triplanar_enabled, 1);
        assert_eq!(material.normal_blend_method, 1); // RNM
        assert_eq!(material.triplanar_slope_threshold, 45.0);
        assert_eq!(material.height_blend_enabled, 1);
    }

    #[test]
    fn test_grassland_factory() {
        let grassland = TerrainMaterialDesc::grassland();
        assert_eq!(grassland.biome, "grassland");
        assert_eq!(grassland.layers.len(), 4);
        assert_eq!(grassland.layers[0].name, "grass");
        assert_eq!(grassland.layers[1].name, "dirt");
        assert_eq!(grassland.layers[2].name, "rock");
        assert_eq!(grassland.layers[3].name, "sparse_grass");
    }

    #[test]
    fn test_desert_factory() {
        let desert = TerrainMaterialDesc::desert();
        assert_eq!(desert.biome, "desert");
        assert_eq!(desert.layers.len(), 4);
        assert_eq!(desert.layers[0].name, "sand");
        assert_eq!(desert.layers[2].name, "desert_rock");
    }

    #[test]
    fn test_forest_factory() {
        let forest = TerrainMaterialDesc::forest();
        assert_eq!(forest.biome, "forest");
        assert_eq!(forest.layers.len(), 4);
        assert_eq!(forest.layers[0].name, "moss");
        assert_eq!(forest.layers[3].name, "leaf_litter");
    }

    #[test]
    fn test_normal_blend_parsing() {
        let linear = TerrainMaterialDesc {
            normal_blend_method: "linear".to_string(),
            ..Default::default()
        };
        assert_eq!(linear.normal_blend_to_gpu(), 0);

        let rnm = TerrainMaterialDesc {
            normal_blend_method: "rnm".to_string(),
            ..Default::default()
        };
        assert_eq!(rnm.normal_blend_to_gpu(), 1);

        let udn = TerrainMaterialDesc {
            normal_blend_method: "udn".to_string(),
            ..Default::default()
        };
        assert_eq!(udn.normal_blend_to_gpu(), 2);

        let invalid = TerrainMaterialDesc {
            normal_blend_method: "invalid".to_string(),
            ..Default::default()
        };
        assert_eq!(invalid.normal_blend_to_gpu(), 1); // Falls back to RNM
    }

    #[test]
    fn test_to_gpu_conversion() {
        let desc = TerrainMaterialDesc::grassland();

        // Mock texture resolver (returns sequential indices)
        use std::cell::Cell;
        let counter = Cell::new(0u32);
        let resolver = |_: &PathBuf| -> u32 {
            let val = counter.get();
            counter.set(val + 1);
            val
        };

        let gpu = desc.to_gpu(&resolver);

        // Check basic properties transferred
        assert_eq!(gpu.splat_uv_scale, 0.5);
        assert_eq!(gpu.triplanar_enabled, 1);
        assert_eq!(gpu.normal_blend_method, 1); // RNM
        assert_eq!(gpu.triplanar_slope_threshold, 35.0);

        // Check first layer got texture indices
        assert!(gpu.layers[0].texture_indices[0] < 100); // albedo index assigned
        assert_eq!(gpu.layers[0].uv_scale, [8.0, 8.0]);
        assert_eq!(gpu.layers[0].blend_sharpness, 0.6);
    }

    #[test]
    fn test_pod_zeroable_terrain_layer() {
        // Verify we can create from bytes (Pod requirement)
        let bytes = [0u8; 64];
        let layer: TerrainLayerGpu = bytemuck::cast(bytes);
        assert_eq!(layer.uv_scale, [0.0, 0.0]);
    }

    #[test]
    fn test_pod_zeroable_terrain_material() {
        // Verify we can create from bytes (Pod requirement)
        // Real-Fix.D 2026-05-08: 2112 bytes (was 576).
        let bytes = [0u8; 2112];
        let material: TerrainMaterialGpu = bytemuck::cast(bytes);
        assert_eq!(material.splat_uv_scale, 0.0);
    }

    #[test]
    fn test_blend_mode_edge_cases() {
        // EDGE CASE: Mixed case, empty string, special characters
        let linear = TerrainMaterialDesc {
            normal_blend_method: "LINEAR".to_string(),
            ..Default::default()
        };
        assert_eq!(linear.normal_blend_to_gpu(), 0);

        let rnm = TerrainMaterialDesc {
            normal_blend_method: "RnM".to_string(),
            ..Default::default()
        };
        assert_eq!(rnm.normal_blend_to_gpu(), 1);

        let whitespace = TerrainMaterialDesc {
            normal_blend_method: "  udn  ".to_string(), // With whitespace
            ..Default::default()
        };
        assert_eq!(whitespace.normal_blend_to_gpu(), 1); // Falls back to RNM (no trim)

        let empty = TerrainMaterialDesc {
            normal_blend_method: "".to_string(),
            ..Default::default()
        };
        assert_eq!(empty.normal_blend_to_gpu(), 1); // Empty falls back to RNM
    }

    #[test]
    fn test_empty_layer_list() {
        // EDGE CASE: TerrainMaterialDesc with zero layers
        let desc = TerrainMaterialDesc {
            name: "empty_terrain".to_string(),
            biome: "void".to_string(),
            splat_map: None,
            splat_uv_scale: 1.0,
            triplanar_enabled: false,
            triplanar_slope_threshold: 45.0,
            normal_blend_method: "linear".to_string(),
            height_blend_enabled: false,
            layers: vec![], // Empty layer list
        };

        // Mock texture resolver
        let resolver = |_: &PathBuf| -> u32 { 0 };

        let gpu = desc.to_gpu(&resolver);

        // Should not crash, GPU material should have default layers
        assert_eq!(gpu.splat_uv_scale, 1.0);
        assert_eq!(gpu.triplanar_enabled, 0);

        // All layers should be default (zero indices)
        for layer in &gpu.layers {
            assert_eq!(layer.texture_indices, [0, 0, 0, 0]);
        }
    }

    #[test]
    fn test_more_than_max_layers_truncates() {
        // EDGE CASE: More than MAX_TERRAIN_LAYERS (=32) layers should truncate.
        // Real-Fix.D 2026-05-08: was 8; canonical MAX_TERRAIN_LAYERS=32.
        let mut layers = Vec::new();
        for _ in 0..(MAX_TERRAIN_LAYERS as usize + 4) {
            layers.push(TerrainLayerDesc::default());
        }
        let desc = TerrainMaterialDesc {
            name: "many_layers".to_string(),
            biome: "complex".to_string(),
            splat_map: None,
            splat_uv_scale: 1.0,
            triplanar_enabled: true,
            triplanar_slope_threshold: 45.0,
            normal_blend_method: "rnm".to_string(),
            height_blend_enabled: true,
            layers,
        };

        let resolver = |_: &PathBuf| -> u32 { 0 };
        let gpu = desc.to_gpu(&resolver);

        // Only MAX_TERRAIN_LAYERS layers should be in GPU struct.
        assert_eq!(gpu.layers.len(), MAX_TERRAIN_LAYERS as usize);
        assert_eq!(gpu.active_layer_count, MAX_TERRAIN_LAYERS);
    }

    #[test]
    fn test_extreme_uv_scales() {
        // EDGE CASE: Very large/small UV scales
        let extreme = TerrainLayerDesc {
            uv_scale: [1000.0, 0.001], // Extreme values
            ..Default::default()
        };
        assert_eq!(extreme.uv_scale, [1000.0, 0.001]);

        // Negative UV scales (flips texture)
        let negative = TerrainLayerDesc {
            uv_scale: [-1.0, -1.0],
            ..Default::default()
        };
        assert_eq!(negative.uv_scale, [-1.0, -1.0]);
    }

    #[test]
    fn test_blend_sharpness_extremes() {
        // EDGE CASE: Blend sharpness at extremes
        let smooth = TerrainLayerDesc {
            blend_sharpness: 0.0, // Completely smooth blend
            ..Default::default()
        };
        assert_eq!(smooth.blend_sharpness, 0.0);

        let sharp = TerrainLayerDesc {
            blend_sharpness: 10.0, // Very sharp blend
            ..Default::default()
        };
        assert_eq!(sharp.blend_sharpness, 10.0);

        let negative = TerrainLayerDesc {
            blend_sharpness: -0.5, // Negative (invalid but should not crash)
            ..Default::default()
        };
        assert_eq!(negative.blend_sharpness, -0.5);
    }
}
