//! Noise generation for terrain heightmaps

use crate::{ChunkId, Heightmap, HeightmapConfig};
use noise::{Billow, Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti};
use serde::{Deserialize, Serialize};

/// Configuration for noise generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseConfig {
    /// Base elevation noise settings
    pub base_elevation: NoiseLayer,
    /// Mountain ridge noise settings
    pub mountains: NoiseLayer,
    /// Detail noise for fine features
    pub detail: NoiseLayer,
    /// Whether to apply erosion
    pub erosion_enabled: bool,
    /// Strength of erosion effect
    pub erosion_strength: f32,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            base_elevation: NoiseLayer {
                enabled: true,
                scale: 0.005,
                amplitude: 50.0,
                octaves: 4,
                persistence: 0.5,
                lacunarity: 2.0,
                noise_type: NoiseType::Perlin,
            },
            mountains: NoiseLayer {
                enabled: true,
                scale: 0.002,
                amplitude: 80.0,
                octaves: 6,
                persistence: 0.4,
                lacunarity: 2.2,
                noise_type: NoiseType::RidgedNoise,
            },
            detail: NoiseLayer {
                enabled: true,
                scale: 0.02,
                amplitude: 5.0,
                octaves: 3,
                persistence: 0.6,
                lacunarity: 2.0,
                noise_type: NoiseType::Billow,
            },
            erosion_enabled: true,
            erosion_strength: 0.3,
        }
    }
}

/// Configuration for a single noise layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseLayer {
    pub enabled: bool,
    pub scale: f64,
    pub amplitude: f32,
    pub octaves: usize,
    pub persistence: f64,
    pub lacunarity: f64,
    pub noise_type: NoiseType,
}

/// Types of noise functions available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NoiseType {
    Perlin,
    RidgedNoise,
    Billow,
    Fbm,
}

/// Terrain noise generator that combines multiple noise layers
pub struct TerrainNoise {
    base_elevation: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    mountains: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    detail: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    config: NoiseConfig,
}

impl std::fmt::Debug for TerrainNoise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerrainNoise")
            .field("config", &self.config)
            .finish()
    }
}

impl TerrainNoise {
    /// Create a new terrain noise generator
    pub fn new(config: &NoiseConfig, seed: u64) -> Self {
        let base_elevation = Self::create_noise_fn(&config.base_elevation, seed);
        let mountains = Self::create_noise_fn(&config.mountains, seed + 1);
        let detail = Self::create_noise_fn(&config.detail, seed + 2);

        Self {
            base_elevation,
            mountains,
            detail,
            config: config.clone(),
        }
    }

    /// Create a noise function based on configuration
    fn create_noise_fn(layer: &NoiseLayer, seed: u64) -> Box<dyn NoiseFn<f64, 3> + Send + Sync> {
        match layer.noise_type {
            NoiseType::Perlin => Box::new(Perlin::new(seed as u32)),
            NoiseType::RidgedNoise => {
                let noise = RidgedMulti::<Perlin>::new(seed as u32)
                    .set_octaves(layer.octaves)
                    .set_persistence(layer.persistence)
                    .set_lacunarity(layer.lacunarity);
                Box::new(noise)
            }
            NoiseType::Billow => {
                let noise = Billow::<Perlin>::new(seed as u32)
                    .set_octaves(layer.octaves)
                    .set_persistence(layer.persistence)
                    .set_lacunarity(layer.lacunarity);
                Box::new(noise)
            }
            NoiseType::Fbm => {
                let noise = Fbm::<Perlin>::new(seed as u32)
                    .set_octaves(layer.octaves)
                    .set_persistence(layer.persistence)
                    .set_lacunarity(layer.lacunarity);
                Box::new(noise)
            }
        }
    }

    /// Generate a heightmap for a terrain chunk
    pub fn generate_heightmap(
        &self,
        chunk_id: ChunkId,
        chunk_size: f32,
        resolution: u32,
    ) -> anyhow::Result<Heightmap> {
        let heightmap_config = HeightmapConfig {
            resolution,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(heightmap_config)?;

        let world_origin = chunk_id.to_world_pos(chunk_size);
        let step = chunk_size / (resolution - 1) as f32;

        for z in 0..resolution {
            for x in 0..resolution {
                let world_x = world_origin.x + x as f32 * step;
                let world_z = world_origin.z + z as f32 * step;

                let height = self.sample_height(world_x as f64, world_z as f64);
                heightmap.set_height(x, z, height);
            }
        }

        Ok(heightmap)
    }

    /// Sample height at a world position
    pub fn sample_height(&self, x: f64, z: f64) -> f32 {
        let mut height = 0.0f32;

        // Base elevation
        if self.config.base_elevation.enabled {
            let noise_val = self.base_elevation.get([
                x * self.config.base_elevation.scale,
                0.0,
                z * self.config.base_elevation.scale,
            ]) as f32;
            height += noise_val * self.config.base_elevation.amplitude;
        }

        // Mountains
        if self.config.mountains.enabled {
            let noise_val = self.mountains.get([
                x * self.config.mountains.scale,
                0.0,
                z * self.config.mountains.scale,
            ]) as f32;
            // Use absolute value for ridged effect
            let mountain_height = noise_val.abs() * self.config.mountains.amplitude;
            height += mountain_height;
        }

        // Detail
        if self.config.detail.enabled {
            let noise_val = self.detail.get([
                x * self.config.detail.scale,
                0.0,
                z * self.config.detail.scale,
            ]) as f32;
            height += noise_val * self.config.detail.amplitude;
        }

        // Ensure non-negative heights
        height.max(0.0)
    }

    /// Generate a density map for cave/overhang generation (future use)
    pub fn sample_density(&self, x: f64, y: f64, z: f64) -> f32 {
        // Use 3D noise for density - this could be used for caves

        self.base_elevation.get([x * 0.01, y * 0.01, z * 0.01]) as f32
    }

    /// Get the configuration
    pub fn config(&self) -> &NoiseConfig {
        &self.config
    }
}

/// Utility functions for noise generation
pub mod utils {
    use super::*;

    /// Generate a preview heightmap for visualization
    pub fn generate_preview(noise: &TerrainNoise, size: u32, scale: f32) -> Vec<f32> {
        let mut heights = Vec::with_capacity((size * size) as usize);
        let step = scale / size as f32;

        for z in 0..size {
            for x in 0..size {
                let world_x = x as f32 * step;
                let world_z = z as f32 * step;
                let height = noise.sample_height(world_x as f64, world_z as f64);
                heights.push(height);
            }
        }

        heights
    }

    /// Normalize a height array to 0-1 range
    pub fn normalize_heights(heights: &mut [f32]) {
        if heights.is_empty() {
            return;
        }

        let min_height = heights.iter().copied().fold(f32::INFINITY, f32::min);
        let max_height = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = max_height - min_height;

        if range > 0.0 {
            for height in heights.iter_mut() {
                *height = (*height - min_height) / range;
            }
        }
    }

    /// Apply a curve to height values for more interesting terrain
    pub fn apply_height_curve(heights: &mut [f32], curve_power: f32) {
        for height in heights.iter_mut() {
            let normalized = (*height).clamp(0.0, 1.0);
            *height = normalized.powf(curve_power) * 100.0; // Scale back up
        }
    }

    /// Create a falloff mask for island generation
    pub fn create_island_mask(size: u32, center_x: f32, center_z: f32, radius: f32) -> Vec<f32> {
        let mut mask = Vec::with_capacity((size * size) as usize);

        for z in 0..size {
            for x in 0..size {
                let dx = x as f32 - center_x;
                let dz = z as f32 - center_z;
                let distance = (dx * dx + dz * dz).sqrt();

                let falloff = if distance < radius {
                    1.0 - (distance / radius).powf(2.0)
                } else {
                    0.0
                };

                mask.push(falloff.clamp(0.0, 1.0));
            }
        }

        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_config_default() {
        let config = NoiseConfig::default();
        assert!(config.base_elevation.enabled);
        assert!(config.mountains.enabled);
        assert!(config.detail.enabled);
    }

    #[test]
    fn test_terrain_noise_creation() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 12345);

        let height = noise.sample_height(100.0, 100.0);
        assert!(height >= 0.0); // Should be non-negative
    }

    #[test]
    fn test_heightmap_generation() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 12345);

        let chunk_id = ChunkId::new(0, 0);
        let heightmap = noise.generate_heightmap(chunk_id, 256.0, 64).unwrap();

        assert_eq!(heightmap.resolution(), 64);
        assert!(heightmap.max_height() >= heightmap.min_height());
    }

    #[test]
    fn test_deterministic_generation() {
        let config = NoiseConfig::default();
        let noise1 = TerrainNoise::new(&config, 12345);
        let noise2 = TerrainNoise::new(&config, 12345);

        let height1 = noise1.sample_height(100.0, 100.0);
        let height2 = noise2.sample_height(100.0, 100.0);

        assert_eq!(height1, height2); // Should be deterministic
    }

    #[test]
    fn test_different_seeds() {
        let config = NoiseConfig::default();
        let noise1 = TerrainNoise::new(&config, 12345);
        let noise2 = TerrainNoise::new(&config, 54321);

        let height1 = noise1.sample_height(100.0, 100.0);
        let height2 = noise2.sample_height(100.0, 100.0);

        assert_ne!(height1, height2); // Different seeds should give different results
    }

    #[test]
    fn test_preview_generation() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 12345);

        let preview = utils::generate_preview(&noise, 32, 256.0);
        assert_eq!(preview.len(), 32 * 32);
    }

    #[test]
    fn test_height_normalization() {
        let mut heights = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        utils::normalize_heights(&mut heights);

        assert_eq!(heights[0], 0.0);
        assert_eq!(heights[4], 1.0);
        assert!(heights[2] > 0.0 && heights[2] < 1.0);
    }

    #[test]
    fn test_island_mask() {
        let mask = utils::create_island_mask(64, 32.0, 32.0, 20.0);
        assert_eq!(mask.len(), 64 * 64);

        // Center should have high value
        let center_idx = 32 * 64 + 32;
        assert!(mask[center_idx] > 0.8);

        // Edges should have low value
        assert!(mask[0] < 0.2);
    }
}
