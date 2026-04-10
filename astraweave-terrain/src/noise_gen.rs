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
    /// Frequency of 3D cave noise (lower = larger cave networks)
    #[serde(default = "default_cave_frequency")]
    pub cave_frequency: f64,
    /// Density threshold below which caves form (0.0–1.0, higher = more caves)
    #[serde(default = "default_cave_threshold")]
    pub cave_threshold: f64,
    /// Strength multiplier for cave carving (0.0 = no caves, 1.0 = full strength)
    #[serde(default = "default_cave_strength")]
    pub cave_strength: f64,
}

fn default_cave_frequency() -> f64 {
    0.03
}
fn default_cave_threshold() -> f64 {
    0.35
}
fn default_cave_strength() -> f64 {
    1.0
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
                domain_warp: DomainWarpConfig::default(),
            },
            mountains: NoiseLayer {
                enabled: true,
                scale: 0.002,
                amplitude: 80.0,
                octaves: 6,
                persistence: 0.4,
                lacunarity: 2.2,
                noise_type: NoiseType::RidgedNoise,
                domain_warp: DomainWarpConfig::default(),
            },
            detail: NoiseLayer {
                enabled: true,
                scale: 0.02,
                amplitude: 5.0,
                octaves: 3,
                persistence: 0.6,
                lacunarity: 2.0,
                noise_type: NoiseType::Billow,
                domain_warp: DomainWarpConfig::default(),
            },
            erosion_enabled: true,
            erosion_strength: 0.3,
            cave_frequency: default_cave_frequency(),
            cave_threshold: default_cave_threshold(),
            cave_strength: default_cave_strength(),
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
    /// Domain warp settings (only used when noise_type = DomainWarped).
    #[serde(default)]
    pub domain_warp: DomainWarpConfig,
}

/// Configuration for domain warping (noise-on-noise).
///
/// Domain warping offsets input coordinates with secondary noise fields
/// before evaluating the primary noise. Multiple iterations create
/// increasingly organic, swirled patterns reminiscent of geological
/// formations, marble textures, and coastlines.
///
/// Reference: Inigo Quilez, "Domain Warping" (2002/2019).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainWarpConfig {
    /// Number of warp iterations (1 = simple warp, 2+ = nested warps).
    /// Each iteration feeds the result of the previous warp as input.
    pub iterations: u32,
    /// Scale of the warping noise (relative to the base noise scale).
    pub warp_scale: f64,
    /// Warp strength — how much the coordinates are displaced (world units).
    pub warp_strength: f64,
    /// Octaves for the warping noise.
    pub warp_octaves: usize,
}

impl Default for DomainWarpConfig {
    fn default() -> Self {
        Self {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 40.0,
            warp_octaves: 3,
        }
    }
}

/// Types of noise functions available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NoiseType {
    Perlin,
    RidgedNoise,
    Billow,
    Fbm,
    /// Domain-warped fBM: offsets input coordinates with secondary noise
    /// before evaluating the primary noise, creating organic swirled patterns.
    DomainWarped,
}

/// Domain-warped noise function.
///
/// Displaces input coordinates through one or more iterations of noise-on-noise,
/// producing organic terrain features (swirled ridges, eroded coastlines, marble veins).
///
/// For N iterations:
///   p₀ = input
///   p₁ = p₀ + strength * warp_noise(p₀)
///   p₂ = p₁ + strength * warp_noise(p₁)
///   ...
///   output = primary_noise(pₙ)
struct DomainWarpedNoise {
    /// Primary noise evaluated at the warped coordinates.
    primary: Fbm<Perlin>,
    /// Warp noise fields (one per axis: X, Z). Y stays at 0 for terrain.
    warp_x: Fbm<Perlin>,
    warp_z: Fbm<Perlin>,
    /// How many times to apply warping.
    iterations: u32,
    /// How much the coordinates are displaced per iteration.
    warp_strength: f64,
}

impl DomainWarpedNoise {
    fn new(layer: &NoiseLayer, seed: u64) -> Self {
        let dw = &layer.domain_warp;

        let primary = Fbm::<Perlin>::new(seed as u32)
            .set_octaves(layer.octaves)
            .set_persistence(layer.persistence)
            .set_lacunarity(layer.lacunarity);

        // Use different seeds for each warp axis to decorrelate them.
        let warp_x = Fbm::<Perlin>::new(seed as u32 + 100)
            .set_octaves(dw.warp_octaves)
            .set_persistence(0.5)
            .set_lacunarity(2.0);

        let warp_z = Fbm::<Perlin>::new(seed as u32 + 200)
            .set_octaves(dw.warp_octaves)
            .set_persistence(0.5)
            .set_lacunarity(2.0);

        Self {
            primary,
            warp_x,
            warp_z,
            iterations: dw.iterations.max(1),
            warp_strength: dw.warp_strength,
        }
    }
}

impl NoiseFn<f64, 3> for DomainWarpedNoise {
    fn get(&self, point: [f64; 3]) -> f64 {
        let mut x = point[0];
        let y = point[1];
        let mut z = point[2];

        for _ in 0..self.iterations {
            let dx = self.warp_x.get([x, y, z]) * self.warp_strength;
            let dz = self.warp_z.get([x, y, z]) * self.warp_strength;
            x += dx;
            z += dz;
        }

        self.primary.get([x, y, z])
    }
}

/// Terrain noise generator that combines multiple noise layers
pub struct TerrainNoise {
    base_elevation: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    mountains: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    detail: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
    /// 3D ridged-multi noise for cave networks
    cave_noise: Box<dyn NoiseFn<f64, 3> + Send + Sync>,
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

        // 3D ridged-multi noise for cave carving.
        // Ridged noise produces narrow, connected ridges — when thresholded,
        // these become tunnel-like cave networks.
        let cave_noise: Box<dyn NoiseFn<f64, 3> + Send + Sync> = Box::new(
            RidgedMulti::<Perlin>::new(seed as u32 + 42)
                .set_octaves(4)
                .set_persistence(0.5)
                .set_lacunarity(2.0),
        );

        Self {
            base_elevation,
            mountains,
            detail,
            cave_noise,
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
            NoiseType::DomainWarped => Box::new(DomainWarpedNoise::new(layer, seed)),
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

    /// Sample 3D density for isosurface extraction (caves, overhangs).
    ///
    /// Returns a signed density value:
    /// - **Positive** = solid terrain (inside surface)
    /// - **Negative** = air / void (outside surface)
    ///
    /// The density combines:
    /// 1. A height-relative base: `surface_height - y` (positive underground)
    /// 2. 3D ridged-multi noise subtracted as a cave mask, producing
    ///    tunnel-like voids when the noise exceeds `cave_threshold`
    ///
    /// Cave networks form where ridged noise creates narrow high-value ridges
    /// in 3D space. The `cave_strength` parameter controls carving intensity.
    pub fn sample_density(&self, x: f64, y: f64, z: f64) -> f32 {
        // Base density: above terrain surface = negative (air), below = positive (solid)
        let surface_height = self.sample_height(x, z) as f64;
        let base_density = surface_height - y;

        // 3D cave noise at cave_frequency scale
        let freq = self.config.cave_frequency;
        let cave_sample = self.cave_noise.get([x * freq, y * freq, z * freq]);

        // Ridged-multi outputs roughly [−1, 1]. We map to [0, 1] and threshold.
        let cave_val = (cave_sample * 0.5 + 0.5).clamp(0.0, 1.0);

        // Carve caves where noise exceeds threshold (narrow ridged peaks = tunnels)
        let cave_mask = if cave_val > self.config.cave_threshold {
            // Smooth falloff above threshold
            let excess = (cave_val - self.config.cave_threshold)
                / (1.0 - self.config.cave_threshold + 1e-10);
            excess * excess * self.config.cave_strength
        } else {
            0.0
        };

        // Depth attenuation: caves weaken near the surface to prevent sky holes,
        // and taper off at extreme depth to keep a solid floor.
        let depth_below_surface = (surface_height - y).max(0.0);
        let surface_guard = (depth_below_surface / 10.0).clamp(0.0, 1.0); // ramp over 10 units
        let deep_guard = 1.0 - ((depth_below_surface - 200.0) / 50.0).clamp(0.0, 1.0); // taper after 200
        let depth_factor = surface_guard * deep_guard;

        // Final density: base minus cave carving
        let carve_strength = 80.0; // world-unit carving radius
        let density = base_density - cave_mask * carve_strength * depth_factor;

        density as f32
    }

    /// Get the configuration
    pub fn config(&self) -> &NoiseConfig {
        &self.config
    }
}

/// Sample domain-warped fBM noise at a single point.
///
/// This is a standalone convenience function for use outside `TerrainNoise`.
/// It creates temporary noise sources — for bulk generation, use `TerrainNoise`
/// with `NoiseType::DomainWarped` instead.
///
/// `scale` — base coordinate scale (smaller = larger features).
/// `warp_config` — warping parameters.
pub fn domain_warped_fbm(
    x: f64,
    z: f64,
    scale: f64,
    seed: u64,
    warp_config: &DomainWarpConfig,
) -> f64 {
    let layer = NoiseLayer {
        enabled: true,
        scale,
        amplitude: 1.0,
        octaves: 4,
        persistence: 0.5,
        lacunarity: 2.0,
        noise_type: NoiseType::DomainWarped,
        domain_warp: warp_config.clone(),
    };
    let noise = DomainWarpedNoise::new(&layer, seed);
    noise.get([x * scale, 0.0, z * scale])
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

    #[test]
    fn test_domain_warp_config_default() {
        let cfg = DomainWarpConfig::default();
        assert_eq!(cfg.iterations, 1);
        assert!((cfg.warp_scale - 1.5).abs() < 1e-5);
        assert!((cfg.warp_strength - 40.0).abs() < 1e-5);
        assert_eq!(cfg.warp_octaves, 3);
    }

    #[test]
    fn test_domain_warped_noise_type() {
        let mut config = NoiseConfig::default();
        config.base_elevation.noise_type = NoiseType::DomainWarped;
        config.base_elevation.domain_warp = DomainWarpConfig {
            iterations: 2,
            warp_scale: 1.5,
            warp_strength: 30.0,
            warp_octaves: 3,
        };

        let noise = TerrainNoise::new(&config, 42);
        let h = noise.sample_height(100.0, 100.0);
        assert!(h >= 0.0);
    }

    #[test]
    fn test_domain_warped_deterministic() {
        let warp = DomainWarpConfig {
            iterations: 2,
            warp_scale: 1.5,
            warp_strength: 50.0,
            warp_octaves: 3,
        };

        let h1 = domain_warped_fbm(100.0, 200.0, 0.005, 42, &warp);
        let h2 = domain_warped_fbm(100.0, 200.0, 0.005, 42, &warp);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_domain_warped_differs_from_plain_fbm() {
        let warp = DomainWarpConfig {
            iterations: 2,
            warp_scale: 1.5,
            warp_strength: 50.0,
            warp_octaves: 3,
        };

        let warped = domain_warped_fbm(100.0, 200.0, 0.005, 42, &warp);

        // Plain fBM at the same coordinates should differ
        let plain_noise = Fbm::<Perlin>::new(42);
        let plain = plain_noise.get([100.0 * 0.005, 0.0, 200.0 * 0.005]);

        // Domain warping should produce a different value
        assert!((warped - plain).abs() > 1e-10);
    }

    #[test]
    fn test_domain_warped_iterations_matter() {
        let warp1 = DomainWarpConfig {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 50.0,
            warp_octaves: 3,
        };
        let warp3 = DomainWarpConfig {
            iterations: 3,
            warp_scale: 1.5,
            warp_strength: 50.0,
            warp_octaves: 3,
        };

        let h1 = domain_warped_fbm(100.0, 200.0, 0.005, 42, &warp1);
        let h3 = domain_warped_fbm(100.0, 200.0, 0.005, 42, &warp3);

        // More iterations should produce a different result
        assert!((h1 - h3).abs() > 1e-10);
    }
}
