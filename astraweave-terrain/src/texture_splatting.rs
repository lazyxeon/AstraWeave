//! Terrain Texture Splatting System - Production Ready
//!
//! This module provides GPU-ready texture splatting for terrain materials including:
//! - Multi-layer texture blending with triplanar projection
//! - Height-based blending for material transitions
//! - Slope-based material masking
//! - Procedural detail mapping
//! - GPU compute shader data structures

use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Maximum number of texture layers per terrain tile
pub const MAX_SPLAT_LAYERS: usize = 8;

/// Terrain material definition for a single texture layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainMaterial {
    /// Unique material ID
    pub id: u32,
    /// Human-readable name
    pub name: String,
    /// Albedo texture path (or array index if using texture arrays)
    pub albedo_index: u32,
    /// Normal map texture index
    pub normal_index: u32,
    /// Roughness/Metallic/AO (packed) texture index
    pub pbr_index: u32,
    /// Height map texture index (for parallax/displacement)
    pub height_index: u32,
    /// UV scale for this material
    pub uv_scale: f32,
    /// Height blend sharpness (for height-based blending)
    pub blend_sharpness: f32,
    /// Triplanar blend falloff
    pub triplanar_sharpness: f32,
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Default".to_string(),
            albedo_index: 0,
            normal_index: 0,
            pbr_index: 0,
            height_index: 0,
            uv_scale: 1.0,
            blend_sharpness: 2.0,
            triplanar_sharpness: 4.0,
        }
    }
}

impl TerrainMaterial {
    /// Create a grass material preset
    pub fn grass() -> Self {
        Self {
            id: 0,
            name: "Grass".to_string(),
            albedo_index: 0,
            normal_index: 0,
            pbr_index: 0,
            height_index: 0,
            uv_scale: 4.0,
            blend_sharpness: 2.0,
            triplanar_sharpness: 4.0,
        }
    }

    /// Create a rock material preset
    pub fn rock() -> Self {
        Self {
            id: 1,
            name: "Rock".to_string(),
            albedo_index: 1,
            normal_index: 1,
            pbr_index: 1,
            height_index: 1,
            uv_scale: 2.0,
            blend_sharpness: 4.0,
            triplanar_sharpness: 8.0,
        }
    }

    /// Create a sand material preset
    pub fn sand() -> Self {
        Self {
            id: 2,
            name: "Sand".to_string(),
            albedo_index: 2,
            normal_index: 2,
            pbr_index: 2,
            height_index: 2,
            uv_scale: 8.0,
            blend_sharpness: 1.5,
            triplanar_sharpness: 2.0,
        }
    }

    /// Create a snow material preset
    pub fn snow() -> Self {
        Self {
            id: 3,
            name: "Snow".to_string(),
            albedo_index: 3,
            normal_index: 3,
            pbr_index: 3,
            height_index: 3,
            uv_scale: 6.0,
            blend_sharpness: 1.0,
            triplanar_sharpness: 2.0,
        }
    }

    /// Create a dirt material preset
    pub fn dirt() -> Self {
        Self {
            id: 4,
            name: "Dirt".to_string(),
            albedo_index: 4,
            normal_index: 4,
            pbr_index: 4,
            height_index: 4,
            uv_scale: 4.0,
            blend_sharpness: 2.5,
            triplanar_sharpness: 4.0,
        }
    }
}

/// Splat map layer weights (packed for GPU)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SplatWeights {
    /// Weights for layers 0-3 (RGBA)
    pub weights_0: Vec4,
    /// Weights for layers 4-7 (RGBA)
    pub weights_1: Vec4,
}

impl SplatWeights {
    /// Create splat weights from an array of layer weights
    pub fn from_weights(weights: &[f32]) -> Self {
        let mut result = Self::default();

        for (i, &w) in weights.iter().take(MAX_SPLAT_LAYERS).enumerate() {
            if i < 4 {
                result.weights_0[i] = w;
            } else {
                result.weights_1[i - 4] = w;
            }
        }

        // Normalize
        let total: f32 = result.weights_0.x
            + result.weights_0.y
            + result.weights_0.z
            + result.weights_0.w
            + result.weights_1.x
            + result.weights_1.y
            + result.weights_1.z
            + result.weights_1.w;

        if total > 0.0001 {
            result.weights_0 /= total;
            result.weights_1 /= total;
        } else {
            // Fallback to sand (layer 1) — avoids grass bleed in non-grassland biomes
            result.weights_0.y = 1.0;
        }

        result
    }

    /// Get weight for a specific layer index
    pub fn get_weight(&self, layer: usize) -> f32 {
        match layer {
            0 => self.weights_0.x,
            1 => self.weights_0.y,
            2 => self.weights_0.z,
            3 => self.weights_0.w,
            4 => self.weights_1.x,
            5 => self.weights_1.y,
            6 => self.weights_1.z,
            7 => self.weights_1.w,
            _ => 0.0,
        }
    }

    /// Get the dominant layer index
    pub fn dominant_layer(&self) -> usize {
        let mut max_weight = 0.0;
        let mut max_idx = 0;

        for i in 0..MAX_SPLAT_LAYERS {
            let w = self.get_weight(i);
            if w > max_weight {
                max_weight = w;
                max_idx = i;
            }
        }

        max_idx
    }
}

/// Configuration for the terrain splatting system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplatConfig {
    /// Enable triplanar projection for steep slopes
    pub triplanar_enabled: bool,
    /// Slope angle (degrees) above which triplanar kicks in
    pub triplanar_slope_threshold: f32,
    /// Enable height-based blending between materials
    pub height_blending_enabled: bool,
    /// Height blend contrast (higher = sharper transitions)
    pub height_blend_contrast: f32,
    /// Slope angle (degrees) for rock material activation
    pub rock_slope_threshold: f32,
    /// Enable detail normal mapping
    pub detail_normal_enabled: bool,
    /// Detail normal UV scale multiplier
    pub detail_uv_scale: f32,
    /// Snow coverage height threshold
    pub snow_height_threshold: f32,
    /// Snow slope fade (degrees from vertical)
    pub snow_slope_fade: f32,
}

impl Default for SplatConfig {
    fn default() -> Self {
        Self {
            triplanar_enabled: true,
            triplanar_slope_threshold: 45.0,
            height_blending_enabled: true,
            height_blend_contrast: 8.0,
            rock_slope_threshold: 35.0,
            detail_normal_enabled: true,
            detail_uv_scale: 16.0,
            snow_height_threshold: 150.0,
            snow_slope_fade: 30.0,
        }
    }
}

/// Splat rule for automatic material assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplatRule {
    /// Target material ID
    pub material_id: u32,
    /// Minimum height (or f32::MIN for no minimum)
    pub min_height: f32,
    /// Maximum height (or f32::MAX for no maximum)
    pub max_height: f32,
    /// Minimum slope angle (degrees)
    pub min_slope: f32,
    /// Maximum slope angle (degrees)
    pub max_slope: f32,
    /// Rule priority (higher = checked first)
    pub priority: u32,
    /// Base weight when conditions match
    pub weight: f32,
    /// Height falloff (how fast weight decreases outside range)
    pub height_falloff: f32,
    /// Slope falloff (how fast weight decreases outside range)
    pub slope_falloff: f32,
}

impl Default for SplatRule {
    fn default() -> Self {
        Self {
            material_id: 0,
            min_height: f32::MIN,
            max_height: f32::MAX,
            min_slope: 0.0,
            max_slope: 90.0,
            priority: 0,
            weight: 1.0,
            height_falloff: 0.01,
            slope_falloff: 0.05,
        }
    }
}

impl SplatRule {
    /// Create a grass rule (flat lowlands)
    pub fn grass() -> Self {
        Self {
            material_id: 0,
            min_height: 0.0,
            max_height: 100.0,
            min_slope: 0.0,
            max_slope: 30.0,
            priority: 10,
            weight: 1.0,
            height_falloff: 0.02,
            slope_falloff: 0.05,
        }
    }

    /// Create a rock rule (steep slopes)
    pub fn rock() -> Self {
        Self {
            material_id: 7,
            min_height: f32::MIN,
            max_height: f32::MAX,
            min_slope: 35.0,
            max_slope: 90.0,
            priority: 20, // Higher priority overrides grass
            weight: 1.0,
            height_falloff: 0.0,
            slope_falloff: 0.1,
        }
    }

    /// Create a sand rule (beaches/deserts)
    pub fn sand() -> Self {
        Self {
            material_id: 1,
            min_height: -5.0,
            max_height: 8.0,
            min_slope: 0.0,
            max_slope: 25.0,
            priority: 15,
            weight: 2.0, // Higher base weight to dominate in beach zones
            height_falloff: 0.3,
            slope_falloff: 0.05,
        }
    }

    /// Create a snow rule (high elevations)
    pub fn snow() -> Self {
        Self {
            material_id: 4,
            min_height: 120.0,
            max_height: f32::MAX,
            min_slope: 0.0,
            max_slope: 45.0,
            priority: 25, // Highest priority
            weight: 1.0,
            height_falloff: 0.03,
            slope_falloff: 0.08,
        }
    }

    /// Evaluate this rule at a given height and slope
    pub fn evaluate(&self, height: f32, slope_degrees: f32) -> f32 {
        let mut weight = self.weight;

        // Height contribution
        if height < self.min_height {
            weight *= (1.0 - (self.min_height - height) * self.height_falloff).max(0.0);
        } else if height > self.max_height {
            weight *= (1.0 - (height - self.max_height) * self.height_falloff).max(0.0);
        }

        // Slope contribution
        if slope_degrees < self.min_slope {
            weight *= (1.0 - (self.min_slope - slope_degrees) * self.slope_falloff).max(0.0);
        } else if slope_degrees > self.max_slope {
            weight *= (1.0 - (slope_degrees - self.max_slope) * self.slope_falloff).max(0.0);
        }

        weight
    }
}

/// Terrain splat map generator
pub struct SplatMapGenerator {
    /// Configuration
    #[allow(dead_code)]
    config: SplatConfig,
    /// Material rules sorted by priority (descending)
    rules: Vec<SplatRule>,
    /// Noise seed for variation
    seed: u64,
}

impl SplatMapGenerator {
    /// Create a new splat map generator
    pub fn new(config: SplatConfig, seed: u64) -> Self {
        Self {
            config,
            rules: Vec::new(),
            seed,
        }
    }

    /// Mutable access to rules for parameter override (e.g., editor UI adjustments).
    pub fn rules_mut(&mut self) -> &mut Vec<SplatRule> {
        &mut self.rules
    }

    /// Add a splat rule
    pub fn add_rule(&mut self, rule: SplatRule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Create with default terrain rules
    pub fn with_default_rules(config: SplatConfig, seed: u64) -> Self {
        let mut gen = Self::new(config, seed);
        gen.add_rule(SplatRule::grass());
        gen.add_rule(SplatRule::rock());
        gen.add_rule(SplatRule::sand());
        gen.add_rule(SplatRule::snow());
        gen
    }

    /// Calculate splat weights for a single terrain point
    pub fn calculate_weights(&self, height: f32, normal: Vec3) -> SplatWeights {
        // Calculate slope from normal (assumes Y-up)
        let slope_cos = normal.y.abs();
        let slope_degrees = slope_cos.acos().to_degrees();

        // Evaluate all rules
        let mut layer_weights = [0.0f32; MAX_SPLAT_LAYERS];

        for rule in &self.rules {
            let weight = rule.evaluate(height, slope_degrees);
            if weight > 0.0 && (rule.material_id as usize) < MAX_SPLAT_LAYERS {
                layer_weights[rule.material_id as usize] += weight;
            }
        }

        // Add noise-based variation
        let noise_offset = self.sample_noise(height);
        for weight in &mut layer_weights {
            *weight *= 1.0 + noise_offset * 0.1;
        }

        SplatWeights::from_weights(&layer_weights)
    }

    /// Generate a full splat map for a heightmap chunk
    pub fn generate_splat_map(
        &self,
        heights: &[f32],
        normals: &[Vec3],
        _resolution: u32,
    ) -> Vec<SplatWeights> {
        let mut splat_map = Vec::with_capacity(heights.len());

        for (i, &height) in heights.iter().enumerate() {
            let normal = normals.get(i).copied().unwrap_or(Vec3::Y);
            let weights = self.calculate_weights(height, normal);
            splat_map.push(weights);
        }

        splat_map
    }

    /// Sample simple noise for variation
    fn sample_noise(&self, height: f32) -> f32 {
        // Simple hash-based noise for variation
        let hash = self.hash_value(height);
        (hash as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn hash_value(&self, value: f32) -> u32 {
        let bits = value.to_bits();
        let mut hash = self.seed as u32;
        hash ^= bits;
        hash = hash.wrapping_mul(0x85ebca6b);
        hash ^= hash >> 16;
        hash
    }
}

/// Triplanar projection weights for a surface normal
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TriplanarWeights {
    /// X-axis projection weight
    pub x: f32,
    /// Y-axis projection weight
    pub y: f32,
    /// Z-axis projection weight
    pub z: f32,
    /// Padding for GPU alignment
    pub _padding: f32,
}

impl TriplanarWeights {
    /// Calculate triplanar weights from a surface normal
    pub fn from_normal(normal: Vec3, sharpness: f32) -> Self {
        let abs_normal = normal.abs();
        let sum = (abs_normal.x.powf(sharpness)
            + abs_normal.y.powf(sharpness)
            + abs_normal.z.powf(sharpness))
        .max(0.0001);

        Self {
            x: abs_normal.x.powf(sharpness) / sum,
            y: abs_normal.y.powf(sharpness) / sum,
            z: abs_normal.z.powf(sharpness) / sum,
            _padding: 0.0,
        }
    }

    /// Check if triplanar projection should be used (steep surface)
    pub fn should_use_triplanar(&self, threshold: f32) -> bool {
        // Use triplanar if Y weight is below threshold
        self.y < threshold
    }
}

/// GPU-ready terrain vertex with splat data
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TerrainSplatVertex {
    /// World position
    pub position: Vec3,
    /// Surface normal
    pub normal: Vec3,
    /// UV coordinates
    pub uv: Vec2,
    /// Splat weights (first 4 layers)
    pub splat_0: Vec4,
    /// Splat weights (layers 4-7)
    pub splat_1: Vec4,
    /// Triplanar blend weights
    pub triplanar: TriplanarWeights,
}

impl TerrainSplatVertex {
    /// Create a new terrain vertex with calculated splat data
    pub fn new(
        position: Vec3,
        normal: Vec3,
        uv: Vec2,
        splat: SplatWeights,
        triplanar_sharpness: f32,
    ) -> Self {
        Self {
            position,
            normal,
            uv,
            splat_0: splat.weights_0,
            splat_1: splat.weights_1,
            triplanar: TriplanarWeights::from_normal(normal, triplanar_sharpness),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splat_weights_normalization() {
        let weights = SplatWeights::from_weights(&[0.5, 0.3, 0.2]);

        let total =
            weights.weights_0.x + weights.weights_0.y + weights.weights_0.z + weights.weights_0.w;

        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_splat_weights_dominant() {
        let weights = SplatWeights::from_weights(&[0.1, 0.6, 0.3]);
        assert_eq!(weights.dominant_layer(), 1);
    }

    #[test]
    fn test_splat_rule_evaluation() {
        let rule = SplatRule::grass();

        // Perfect match
        let weight_perfect = rule.evaluate(50.0, 15.0);
        assert!(weight_perfect > 0.9);

        // Too high
        let weight_high = rule.evaluate(150.0, 15.0);
        assert!(weight_high < weight_perfect);

        // Too steep
        let weight_steep = rule.evaluate(50.0, 60.0);
        assert!(weight_steep < weight_perfect);
    }

    #[test]
    fn test_triplanar_weights() {
        // Flat surface (Y-up)
        let flat = TriplanarWeights::from_normal(Vec3::Y, 4.0);
        assert!(flat.y > 0.9);

        // Vertical cliff (X-facing)
        let cliff = TriplanarWeights::from_normal(Vec3::X, 4.0);
        assert!(cliff.x > 0.9);

        // 45-degree slope
        let slope = TriplanarWeights::from_normal(Vec3::new(0.707, 0.707, 0.0), 4.0);
        assert!((slope.x - slope.y).abs() < 0.1);
    }

    #[test]
    fn test_splat_generator_default_rules() {
        let config = SplatConfig::default();
        let generator = SplatMapGenerator::with_default_rules(config, 12345);

        // Flat grass area
        let grass_weights = generator.calculate_weights(50.0, Vec3::Y);
        assert_eq!(grass_weights.dominant_layer(), 0); // Grass

        // High steep mountain
        let rock_weights = generator.calculate_weights(50.0, Vec3::new(0.3, 0.3, 0.9).normalize());
        assert_eq!(rock_weights.dominant_layer(), 7); // Rock (stone)

        // Beach
        let sand_weights = generator.calculate_weights(2.0, Vec3::Y);
        assert_eq!(sand_weights.dominant_layer(), 1); // Sand

        // Snow peak
        let snow_weights = generator.calculate_weights(180.0, Vec3::Y);
        assert_eq!(snow_weights.dominant_layer(), 4); // Snow
    }

    #[test]
    fn test_terrain_material_presets() {
        let grass = TerrainMaterial::grass();
        assert_eq!(grass.name, "Grass");

        let rock = TerrainMaterial::rock();
        assert_eq!(rock.id, 1);

        let snow = TerrainMaterial::snow();
        assert!(snow.uv_scale > 1.0);
    }

    #[test]
    fn test_splat_map_generation() {
        let config = SplatConfig::default();
        let generator = SplatMapGenerator::with_default_rules(config, 12345);

        let heights = vec![50.0, 60.0, 70.0, 80.0];
        let normals = vec![Vec3::Y, Vec3::Y, Vec3::Y, Vec3::Y];

        let splat_map = generator.generate_splat_map(&heights, &normals, 2);

        assert_eq!(splat_map.len(), 4);
        for weights in &splat_map {
            let total = weights.weights_0.x
                + weights.weights_0.y
                + weights.weights_0.z
                + weights.weights_0.w
                + weights.weights_1.x
                + weights.weights_1.y
                + weights.weights_1.z
                + weights.weights_1.w;
            assert!((total - 1.0).abs() < 0.001);
        }
    }
}
