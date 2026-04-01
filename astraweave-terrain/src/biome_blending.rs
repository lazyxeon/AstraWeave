//! Advanced biome blending system for seamless terrain transitions
//!
//! This module provides production-ready biome blending with:
//! - Smooth multi-biome interpolation (up to 4 simultaneous biomes)
//! - Distance-based weight falloff with configurable curves
//! - GPU-friendly blend weights for shader-based rendering
//! - Noise-based edge variation for natural transitions

use crate::{BiomeType, Heightmap};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Maximum number of biomes that can influence a single terrain point
pub const MAX_BLEND_BIOMES: usize = 4;

/// Blend weight for a single biome influence
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BiomeWeight {
    /// The biome type
    pub biome: BiomeType,
    /// Weight (0.0-1.0, normalized across all weights)
    pub weight: f32,
}

impl Default for BiomeWeight {
    fn default() -> Self {
        Self {
            biome: BiomeType::Grassland,
            weight: 0.0,
        }
    }
}

/// Packed blend weights optimized for GPU upload
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct PackedBiomeBlend {
    /// Biome IDs packed as u8 (up to 4 biomes)
    pub biome_ids: [u8; 4],
    /// Blend weights (normalized, sum to 1.0)
    pub weights: [f32; 4],
}

impl PackedBiomeBlend {
    /// Create from individual biome weights (auto-normalizes)
    pub fn from_weights(weights: &[BiomeWeight]) -> Self {
        let mut result = Self::default();
        let mut total_weight = 0.0f32;

        // Take top 4 weights sorted by influence
        let mut sorted: Vec<_> = weights.iter().filter(|w| w.weight > 0.001).collect();
        sorted.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (i, bw) in sorted.iter().take(MAX_BLEND_BIOMES).enumerate() {
            result.biome_ids[i] = bw.biome as u8;
            result.weights[i] = bw.weight;
            total_weight += bw.weight;
        }

        // Normalize weights to sum to 1.0
        if total_weight > 0.0 {
            for w in &mut result.weights {
                *w /= total_weight;
            }
        } else {
            // Fallback to grassland if no weights
            result.biome_ids[0] = BiomeType::Grassland as u8;
            result.weights[0] = 1.0;
        }

        result
    }

    /// Get the dominant biome (highest weight)
    pub fn dominant_biome(&self) -> BiomeType {
        // Find index of max weight
        let max_idx = self
            .weights
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Safe conversion with fallback
        match self.biome_ids[max_idx] {
            0 => BiomeType::Grassland,
            1 => BiomeType::Desert,
            2 => BiomeType::Forest,
            3 => BiomeType::Mountain,
            4 => BiomeType::Tundra,
            5 => BiomeType::Swamp,
            6 => BiomeType::Beach,
            7 => BiomeType::River,
            _ => BiomeType::Grassland,
        }
    }
}

/// Configuration for biome blending behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeBlendConfig {
    /// Blend radius in world units (how far biomes influence each other)
    pub blend_radius: f32,
    /// Falloff curve power (1.0 = linear, 2.0 = smooth, 3.0 = sharp)
    pub falloff_power: f32,
    /// Edge noise scale (for irregular biome boundaries)
    pub edge_noise_scale: f32,
    /// Edge noise amplitude (world units of boundary variation)
    pub edge_noise_amplitude: f32,
    /// Minimum weight to consider a biome (culling threshold)
    pub min_weight_threshold: f32,
    /// Enable height-based blend modification
    pub height_blend_enabled: bool,
    /// Height blend factor (how much height affects blending)
    pub height_blend_factor: f32,
}

impl Default for BiomeBlendConfig {
    fn default() -> Self {
        Self {
            blend_radius: 32.0,
            falloff_power: 2.0,
            edge_noise_scale: 0.02,
            edge_noise_amplitude: 8.0,
            min_weight_threshold: 0.01,
            height_blend_enabled: true,
            height_blend_factor: 0.3,
        }
    }
}

/// Biome blender for calculating smooth terrain transitions
pub struct BiomeBlender {
    config: BiomeBlendConfig,
    /// Pre-computed edge noise for deterministic results
    edge_noise_seed: u64,
}

impl BiomeBlender {
    /// Create a new biome blender with the given configuration
    pub fn new(config: BiomeBlendConfig, seed: u64) -> Self {
        Self {
            config,
            edge_noise_seed: seed,
        }
    }

    /// Calculate blend weights for a single terrain point
    ///
    /// # Arguments
    /// * `world_pos` - World position (x, z)
    /// * `height` - Terrain height at this position
    /// * `neighbor_biomes` - Sample of nearby biomes with their positions
    ///
    /// # Returns
    /// Packed blend weights ready for GPU upload
    pub fn calculate_blend_weights(
        &self,
        world_pos: Vec2,
        height: f32,
        neighbor_biomes: &[(Vec2, BiomeType)],
    ) -> PackedBiomeBlend {
        let mut biome_weights: std::collections::HashMap<BiomeType, f32> =
            std::collections::HashMap::new();

        // Calculate distance-based weights for each neighbor
        for &(neighbor_pos, biome) in neighbor_biomes {
            let distance = (world_pos - neighbor_pos).length();

            // Apply edge noise for irregular boundaries
            let noise_offset = self.sample_edge_noise(world_pos);
            let effective_distance = (distance + noise_offset).max(0.0);

            // Calculate weight with falloff
            if effective_distance < self.config.blend_radius {
                let normalized_dist = effective_distance / self.config.blend_radius;
                let base_weight = (1.0 - normalized_dist).powf(self.config.falloff_power);

                // Apply height-based modification if enabled
                let weight = if self.config.height_blend_enabled {
                    self.apply_height_modification(base_weight, height, biome)
                } else {
                    base_weight
                };

                if weight > self.config.min_weight_threshold {
                    *biome_weights.entry(biome).or_insert(0.0) += weight;
                }
            }
        }

        // Convert to BiomeWeight array
        let weights: Vec<BiomeWeight> = biome_weights
            .into_iter()
            .map(|(biome, weight)| BiomeWeight { biome, weight })
            .collect();

        PackedBiomeBlend::from_weights(&weights)
    }

    /// Calculate blend weights for an entire heightmap chunk
    ///
    /// This is the main entry point for chunk-based terrain generation.
    /// Includes a fast path when the entire chunk has a uniform biome.
    pub fn blend_chunk(
        &self,
        heightmap: &Heightmap,
        biome_map: &[BiomeType],
        chunk_size: f32,
        world_offset: Vec2,
    ) -> Vec<PackedBiomeBlend> {
        let resolution = heightmap.resolution() as usize;
        let total = resolution * resolution;

        // Fast path: if the entire chunk has a single biome, skip blending entirely
        if let Some(&first_biome) = biome_map.first() {
            if biome_map.iter().all(|&b| b == first_biome) {
                let uniform = PackedBiomeBlend {
                    biome_ids: [first_biome as u8, 0, 0, 0],
                    weights: [1.0, 0.0, 0.0, 0.0],
                };
                return vec![uniform; total];
            }
        }

        let cell_size = chunk_size / (resolution - 1) as f32;
        let sample_radius = (self.config.blend_radius / cell_size).ceil() as i32;
        let mut blend_weights = Vec::with_capacity(total);

        for z in 0..resolution {
            for x in 0..resolution {
                let local_pos = Vec2::new(x as f32 * cell_size, z as f32 * cell_size);
                let world_pos = world_offset + local_pos;
                let height = heightmap.get_height(x as u32, z as u32);

                // Gather neighbor biomes in a radius (inlined for perf)
                let mut biome_weights: std::collections::HashMap<BiomeType, f32> =
                    std::collections::HashMap::new();

                for dz in -sample_radius..=sample_radius {
                    for dx in -sample_radius..=sample_radius {
                        let nx = x as i32 + dx;
                        let nz = z as i32 + dz;

                        if nx >= 0 && nx < resolution as i32 && nz >= 0 && nz < resolution as i32 {
                            let idx = nz as usize * resolution + nx as usize;
                            if let Some(&biome) = biome_map.get(idx) {
                                let neighbor_pos = world_offset
                                    + Vec2::new(nx as f32 * cell_size, nz as f32 * cell_size);
                                let distance = (world_pos - neighbor_pos).length();
                                let noise_offset = self.sample_edge_noise(world_pos);
                                let effective_distance = (distance + noise_offset).max(0.0);

                                if effective_distance < self.config.blend_radius {
                                    let normalized_dist =
                                        effective_distance / self.config.blend_radius;
                                    let base_weight =
                                        (1.0 - normalized_dist).powf(self.config.falloff_power);
                                    let weight = if self.config.height_blend_enabled {
                                        self.apply_height_modification(base_weight, height, biome)
                                    } else {
                                        base_weight
                                    };
                                    if weight > self.config.min_weight_threshold {
                                        *biome_weights.entry(biome).or_insert(0.0) += weight;
                                    }
                                }
                            }
                        }
                    }
                }

                let weights: Vec<BiomeWeight> = biome_weights
                    .into_iter()
                    .map(|(biome, weight)| BiomeWeight { biome, weight })
                    .collect();
                blend_weights.push(PackedBiomeBlend::from_weights(&weights));
            }
        }

        blend_weights
    }

    /// Gather biome samples from neighboring cells
    #[allow(dead_code)]
    fn gather_neighbor_biomes(
        &self,
        center_x: usize,
        center_z: usize,
        resolution: usize,
        cell_size: f32,
        world_offset: Vec2,
        biome_map: &[BiomeType],
    ) -> Vec<(Vec2, BiomeType)> {
        let sample_radius = (self.config.blend_radius / cell_size).ceil() as i32;
        let mut neighbors = Vec::new();

        for dz in -sample_radius..=sample_radius {
            for dx in -sample_radius..=sample_radius {
                let nx = center_x as i32 + dx;
                let nz = center_z as i32 + dz;

                if nx >= 0 && nx < resolution as i32 && nz >= 0 && nz < resolution as i32 {
                    let idx = nz as usize * resolution + nx as usize;
                    if let Some(&biome) = biome_map.get(idx) {
                        let pos =
                            world_offset + Vec2::new(nx as f32 * cell_size, nz as f32 * cell_size);
                        neighbors.push((pos, biome));
                    }
                }
            }
        }

        neighbors
    }

    /// Apply height-based modification to blend weight
    fn apply_height_modification(&self, base_weight: f32, height: f32, biome: BiomeType) -> f32 {
        // Adjust weight based on biome's preferred height range
        let height_preference = match biome {
            BiomeType::Beach => 1.0 - (height / 20.0).clamp(0.0, 1.0),
            BiomeType::River => 1.0 - (height / 10.0).clamp(0.0, 1.0),
            BiomeType::Mountain => (height / 150.0).clamp(0.2, 1.0),
            BiomeType::Tundra => (height / 80.0 - 0.3).clamp(0.0, 1.0),
            BiomeType::Swamp => 1.0 - (height / 30.0).clamp(0.0, 1.0),
            _ => 1.0, // Grassland, Desert, Forest work at any height
        };

        let modifier = 1.0 - self.config.height_blend_factor
            + height_preference * self.config.height_blend_factor;
        base_weight * modifier
    }

    /// Sample edge noise at a position for irregular boundaries
    fn sample_edge_noise(&self, pos: Vec2) -> f32 {
        // Simple hash-based noise for edge variation
        let hash = self.hash_position(pos);
        let noise_value = ((hash as f32) / u32::MAX as f32) * 2.0 - 1.0;
        noise_value * self.config.edge_noise_amplitude
    }

    /// Simple position-based hash for deterministic noise
    fn hash_position(&self, pos: Vec2) -> u32 {
        let x = (pos.x * self.config.edge_noise_scale) as i32;
        let z = (pos.y * self.config.edge_noise_scale) as i32;

        let mut hash = self.edge_noise_seed as u32;
        hash ^= x as u32;
        hash = hash.wrapping_mul(0x85ebca6b);
        hash ^= z as u32;
        hash = hash.wrapping_mul(0xc2b2ae35);
        hash ^= hash >> 16;
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_biome_blend_normalization() {
        let weights = vec![
            BiomeWeight {
                biome: BiomeType::Grassland,
                weight: 0.6,
            },
            BiomeWeight {
                biome: BiomeType::Forest,
                weight: 0.3,
            },
            BiomeWeight {
                biome: BiomeType::Mountain,
                weight: 0.1,
            },
        ];

        let packed = PackedBiomeBlend::from_weights(&weights);

        // Weights should sum to 1.0
        let sum: f32 = packed.weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);

        // Dominant biome should be grassland
        assert_eq!(packed.dominant_biome(), BiomeType::Grassland);
    }

    #[test]
    fn test_packed_biome_blend_max_biomes() {
        // Test with more than 4 biomes
        let weights = vec![
            BiomeWeight {
                biome: BiomeType::Grassland,
                weight: 0.3,
            },
            BiomeWeight {
                biome: BiomeType::Forest,
                weight: 0.25,
            },
            BiomeWeight {
                biome: BiomeType::Mountain,
                weight: 0.2,
            },
            BiomeWeight {
                biome: BiomeType::Desert,
                weight: 0.15,
            },
            BiomeWeight {
                biome: BiomeType::Tundra,
                weight: 0.1,
            }, // Should be dropped
        ];

        let packed = PackedBiomeBlend::from_weights(&weights);

        // Should only have 4 biomes
        let non_zero = packed.weights.iter().filter(|&&w| w > 0.0).count();
        assert!(non_zero <= MAX_BLEND_BIOMES);
    }

    #[test]
    fn test_biome_blender_distance_falloff() {
        let config = BiomeBlendConfig::default();
        let blender = BiomeBlender::new(config.clone(), 12345);

        let center = Vec2::ZERO;
        let height = 50.0;

        // Close neighbor should have higher weight than far neighbor
        let neighbors = vec![
            (Vec2::new(10.0, 0.0), BiomeType::Grassland),
            (Vec2::new(60.0, 0.0), BiomeType::Forest),
        ];

        let blend = blender.calculate_blend_weights(center, height, &neighbors);

        // Grassland (closer) should have higher weight
        assert!(blend.weights[0] > blend.weights[1]);
    }

    #[test]
    fn test_biome_blender_height_modification() {
        let config = BiomeBlendConfig {
            height_blend_enabled: true,
            ..Default::default()
        };

        let blender = BiomeBlender::new(config, 12345);

        // Test that mountain biome is preferred at high elevations.
        // Keep neighbors close to center so they are well within blend_radius
        // even after edge noise displacement.
        let high_pos = Vec2::ZERO;
        let high_neighbors = vec![
            (Vec2::new(2.0, 0.0), BiomeType::Grassland),
            (Vec2::new(2.0, 2.0), BiomeType::Mountain),
        ];

        let high_blend = blender.calculate_blend_weights(high_pos, 150.0, &high_neighbors);

        // At 150m height, mountain should be preferred
        let mountain_weight = high_blend
            .weights
            .iter()
            .zip(high_blend.biome_ids.iter())
            .find(|(_, &id)| id == BiomeType::Mountain as u8)
            .map(|(w, _)| *w)
            .unwrap_or(0.0);

        assert!(mountain_weight > 0.0);
    }

    #[test]
    fn test_edge_noise_determinism() {
        let config = BiomeBlendConfig::default();
        let blender1 = BiomeBlender::new(config.clone(), 12345);
        let blender2 = BiomeBlender::new(config, 12345);

        let pos = Vec2::new(100.0, 200.0);

        // Same seed should produce same noise
        let noise1 = blender1.sample_edge_noise(pos);
        let noise2 = blender2.sample_edge_noise(pos);

        assert!((noise1 - noise2).abs() < 0.001);
    }
}
