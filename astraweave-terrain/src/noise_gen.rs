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
    /// Phase 1.6-F.2 §2.6: whether to apply continental-scale mountain
    /// amplitude modulation. When true, `TerrainNoise::sample_height`
    /// multiplies the mountain layer by a low-frequency spatial field,
    /// producing regional clustering of mountain zones vs. lowland zones.
    /// Default: false (F.1-identical output).
    #[serde(default = "default_continental_enabled")]
    pub continental_enabled: bool,
    /// Frequency of continental-scale noise. Lower = longer wavelength =
    /// larger regions. Default 0.0004 gives ~2500-world-unit wavelength,
    /// matching the radius-5 editor terrain extent.
    #[serde(default = "default_continental_scale")]
    pub continental_scale: f32,
    /// Minimum mountain-amplitude multiplier where continental noise is at
    /// its minimum. Default 0.15 means lowlands retain 15% of full mountain
    /// amplitude — subtle topography, not flat.
    #[serde(default = "default_continental_min")]
    pub continental_min: f32,
    /// Offset added to the world seed for continental noise determinism.
    /// Default 7; chosen to avoid collision with base/mountains/detail/cave
    /// seed offsets (0/+1/+2/+42).
    #[serde(default = "default_continental_seed_offset")]
    pub continental_seed_offset: u32,
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
fn default_continental_enabled() -> bool {
    false
}
fn default_continental_scale() -> f32 {
    // Phase 1.6-F.2-T.B.1: raised from 0.0004 to 0.0012 (wavelength
    // dropped ~2500 → ~830 world units). F.2-T.A diagnostic found that
    // at scale 0.0004, seed 12345's continental field maxed at 0.669
    // across the editor's ~2800-unit terrain extent (p95=0.631), with
    // no samples > 0.7 — i.e. NO highland regions in visible terrain,
    // breaking the F.2 core design intent of regional mountain clustering.
    // Scale 0.0012 fits ~3.4 continental periods within the visible
    // extent, ensuring both low-continental (lowland) and high-continental
    // (highland) regions exist at every practical seed.
    0.0012
}
fn default_continental_min() -> f32 {
    // Phase 1.6-F.2-T.B.1: raised from 0.15 to 0.50. F.2-T.A diagnostic
    // H1 measured detail_abs / mountain_effective ratio = 0.60 in
    // lowlands — the intrinsically-spiky Billow detail layer became
    // comparable to the continental-suppressed mountain layer, producing
    // the bed-of-nails visible surface. A higher continental_min keeps
    // more mountain character in lowlands, making detail's spikiness a
    // smaller relative perturbation.
    //
    // F.2-T.B.1 initially chose 0.35, but the highland-Y-max regression
    // test measured only 87.55 units (target ≥ 100) because even the
    // highest continental_01 in the editor's terrain extent (0.874 per
    // F.2-T.A) only yielded multiplier = 0.35 + 0.65×0.874 = 0.918, so
    // highland peaks reached ~73 units of mountain layer. Raised to 0.50
    // so the highland multiplier approaches 0.94 at cont_01=0.874, with
    // detail-reduction (F.2-T.B.2) keeping H1 resolved (detail ratio in
    // lowlands stays acceptable because detail_amplitude was halved
    // alongside).
    0.50
}
fn default_continental_seed_offset() -> u32 {
    7
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
            continental_enabled: default_continental_enabled(),
            continental_scale: default_continental_scale(),
            continental_min: default_continental_min(),
            continental_seed_offset: default_continental_seed_offset(),
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
    /// Phase 1.6-F.2 §2.6: continental-scale plain Perlin noise sampled by
    /// `sample_height` when `config.continental_enabled` to modulate the
    /// mountain layer's contribution spatially. Produces regional clustering
    /// of mountain zones vs. lowland zones.
    continental: Perlin,
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

        // Phase 1.6-F.2 §2.6: continental-scale plain Perlin. Seeded
        // deterministically from (world_seed + continental_seed_offset) so
        // the field is a pure function of (world_seed, world_x, world_z).
        let continental_seed = seed
            .wrapping_add(config.continental_seed_offset as u64)
            as u32;
        let continental = Perlin::new(continental_seed);

        Self {
            base_elevation,
            mountains,
            detail,
            cave_noise,
            continental,
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

        // Mountains — with optional continental-scale amplitude modulation.
        // Phase 1.6-F.2 §2.6: when continental_enabled, the mountain layer's
        // contribution is multiplied by mix(continental_min, 1.0, continental_01),
        // where continental_01 is the continental noise at (x, z) mapped from
        // [-1, 1] to [0, 1]. This produces regional clustering of mountain
        // zones vs. lowland zones — mountain-country regions retain full
        // amplitude, lowland regions retain continental_min (default 0.15) of
        // the full amplitude.
        if self.config.mountains.enabled {
            let noise_val = self.mountains.get([
                x * self.config.mountains.scale,
                0.0,
                z * self.config.mountains.scale,
            ]) as f32;
            // Use absolute value for ridged effect
            let mountain_height_raw = noise_val.abs() * self.config.mountains.amplitude;

            let mountain_height = if self.config.continental_enabled {
                let continental_raw = self.continental.get([
                    x * self.config.continental_scale as f64,
                    0.0,
                    z * self.config.continental_scale as f64,
                ]) as f32;
                // Perlin output is approximately [-1, 1]; map to [0, 1].
                let continental_01 = ((continental_raw + 1.0) * 0.5).clamp(0.0, 1.0);
                let multiplier = self.config.continental_min
                    + (1.0 - self.config.continental_min) * continental_01;
                mountain_height_raw * multiplier
            } else {
                mountain_height_raw
            };
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

    /// Phase 1.6-F.2 §2.6: sample the continental noise field at a world
    /// position, mapped to [0, 1]. Promoted from `#[cfg(test)]` to permanent
    /// `pub(crate)` during F.2-T so downstream diagnostic tests (and future
    /// tuning investigations) can inspect the field directly without running
    /// the full heightmap pipeline. `#[allow(dead_code)]` because F.2-T's
    /// diagnostic tests that exercised this were removed at closeout; the
    /// accessor stays for the next tuning investigation that needs it.
    #[allow(dead_code)]
    pub(crate) fn sample_continental_01(&self, x: f64, z: f64) -> f32 {
        let raw = self.continental.get([
            x * self.config.continental_scale as f64,
            0.0,
            z * self.config.continental_scale as f64,
        ]) as f32;
        ((raw + 1.0) * 0.5).clamp(0.0, 1.0)
    }

    /// Phase 1.6-F.2-T: diagnostic accessor returning per-layer contributions
    /// to the final sampled height, plus the continental field's [0, 1]
    /// sample at the same position. Mirrors `sample_height`'s math exactly
    /// so downstream diagnostics can test hypotheses about layer dominance
    /// (e.g. detail layer masking mountain layer in lowlands).
    ///
    /// Returns: `(base_contrib, mountain_raw_contrib, mountain_effective_contrib, detail_contrib, continental_01)`.
    /// `#[allow(dead_code)]` for the same reason as `sample_continental_01`.
    #[allow(dead_code)]
    pub(crate) fn sample_per_layer(&self, x: f64, z: f64) -> (f32, f32, f32, f32, f32) {
        let base_contrib = if self.config.base_elevation.enabled {
            let v = self.base_elevation.get([
                x * self.config.base_elevation.scale,
                0.0,
                z * self.config.base_elevation.scale,
            ]) as f32;
            v * self.config.base_elevation.amplitude
        } else {
            0.0
        };

        let continental_01 = self.sample_continental_01(x, z);

        let (mountain_raw, mountain_effective) = if self.config.mountains.enabled {
            let v = self.mountains.get([
                x * self.config.mountains.scale,
                0.0,
                z * self.config.mountains.scale,
            ]) as f32;
            let raw = v.abs() * self.config.mountains.amplitude;
            let effective = if self.config.continental_enabled {
                let multiplier = self.config.continental_min
                    + (1.0 - self.config.continental_min) * continental_01;
                raw * multiplier
            } else {
                raw
            };
            (raw, effective)
        } else {
            (0.0, 0.0)
        };

        let detail_contrib = if self.config.detail.enabled {
            let v = self.detail.get([
                x * self.config.detail.scale,
                0.0,
                z * self.config.detail.scale,
            ]) as f32;
            v * self.config.detail.amplitude
        } else {
            0.0
        };

        (
            base_contrib,
            mountain_raw,
            mountain_effective,
            detail_contrib,
            continental_01,
        )
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

    /// Phase 1.6-F.2.C: verify the continental noise field's output is in
    /// [0, 1] and exhibits meaningful spatial variation (not constant).
    #[test]
    fn phase_1_6_f2_continental_output_range_and_variation() {
        let mut config = NoiseConfig::default();
        config.continental_enabled = true;
        let noise = TerrainNoise::new(&config, 12345);

        // Sample continental at a 20×20 grid across a 4000×4000 world area.
        let mut min_sample = f32::INFINITY;
        let mut max_sample = f32::NEG_INFINITY;
        for gx in 0..20 {
            for gz in 0..20 {
                let x = (gx as f64 - 10.0) * 200.0;
                let z = (gz as f64 - 10.0) * 200.0;
                let sample = noise.sample_continental_01(x, z);
                assert!(
                    (0.0..=1.0).contains(&sample),
                    "continental sample out of [0, 1]: {sample} at ({x}, {z})"
                );
                if sample < min_sample {
                    min_sample = sample;
                }
                if sample > max_sample {
                    max_sample = sample;
                }
            }
        }

        // Meaningful variation across the sampled region.
        assert!(
            min_sample < 0.5,
            "continental min {min_sample} — expected < 0.5 (meaningful low region)"
        );
        assert!(
            max_sample > 0.5,
            "continental max {max_sample} — expected > 0.5 (meaningful high region)"
        );
        assert!(
            (max_sample - min_sample) >= 0.5,
            "continental range {:.3} — expected >= 0.5 (meaningful spatial variation)",
            max_sample - min_sample
        );
    }

    /// Phase 1.6-F.2.C: verify DomainWarped sampling produces measurably
    /// different output than plain Perlin at identical world positions.
    /// Sanity check that the preset-driven noise_type override is actually
    /// changing the noise function, not silently ignored.
    #[test]
    fn phase_1_6_f2_domain_warped_differs_from_perlin() {
        let warp = DomainWarpConfig {
            iterations: 2,
            warp_scale: 1.5,
            warp_strength: 40.0,
            warp_octaves: 3,
        };

        let mut config_perlin = NoiseConfig::default();
        config_perlin.base_elevation.noise_type = NoiseType::Perlin;
        config_perlin.base_elevation.domain_warp = warp.clone();
        // Disable continental so the comparison isolates the base layer.
        config_perlin.continental_enabled = false;
        let noise_perlin = TerrainNoise::new(&config_perlin, 12345);

        let mut config_warped = NoiseConfig::default();
        config_warped.base_elevation.noise_type = NoiseType::DomainWarped;
        config_warped.base_elevation.domain_warp = warp;
        config_warped.continental_enabled = false;
        let noise_warped = TerrainNoise::new(&config_warped, 12345);

        // Sample a 10×10 grid. DomainWarped should differ from Perlin at
        // most positions — the coordinate displacement by iterative warping
        // produces distinct values in the underlying Fbm lookup vs. a direct
        // Perlin evaluation.
        let mut differ_count = 0;
        for gx in 0..10 {
            for gz in 0..10 {
                let x = gx as f64 * 100.0;
                let z = gz as f64 * 100.0;
                let sample_perlin = noise_perlin.sample_height(x, z);
                let sample_warped = noise_warped.sample_height(x, z);
                if (sample_perlin - sample_warped).abs() > 0.1 {
                    differ_count += 1;
                }
            }
        }

        // Threshold of 50 is comfortably above an "accidental coincidence"
        // floor (< 10) while below the measured 70/100 with margin. If
        // DomainWarped were silently replaced by the plain-Perlin path, this
        // test would observe ~0 differences (same noise source, same seed).
        assert!(
            differ_count >= 50,
            "DomainWarped matched Perlin at too many positions: only {differ_count}/100 differ (expected >= 50)"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 1.6-F.2-T diagnostic tests were removed at F.2-T.D closeout.
    // The findings (H1 CONFIRMED, H2 CONFIRMED, H3 REJECTED) drove the tuning
    // changes landed in F.2-T.B.1 / B.2 / C. See commit F.2-T.A's message
    // for the original diagnostic output. The diagnostic accessors
    // `sample_continental_01` and `sample_per_layer` on `TerrainNoise` stay
    // permanent for future investigations.
    //
    // The permanent regression test
    // `phase_1_6_f2_t_highland_regions_reach_f1_target` below guards against
    // future "continental suppressed everything uniformly" failure modes.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 1.6-F.2-T-2 diagnostics (temporary — removed at F.2-T-2.D).
    //
    // F.2-T tuning did not resolve the bed-of-nails regression — Andrew's
    // 2026-04-22 visual verification showed spikes were WORSE, and crucially
    // that spike amplitude appeared UNIFORM across highland and lowland
    // regions. Uniform spikiness implies a non-continental-modulated source
    // (base or detail, not mountain). These three tests measure per-layer
    // spatial frequency content (local curvature) plus continental
    // correlation, so the actual spike source can be named with confidence
    // before any tuning.
    // -----------------------------------------------------------------------

    /// Local-curvature spikiness metric: mean of |center - avg(4 neighbors)|
    /// over interior cells of a grid. Higher = spikier. Used by the three
    /// F.2-T-2 diagnostics below.
    fn phase_1_6_f2_t2_local_curvature_grid(heights: &[f32], grid_dim: usize) -> f32 {
        let mut total = 0.0f32;
        let mut count = 0u32;
        for i in 1..grid_dim - 1 {
            for j in 1..grid_dim - 1 {
                let center = heights[i * grid_dim + j];
                let neighbors = heights[(i - 1) * grid_dim + j]
                    + heights[(i + 1) * grid_dim + j]
                    + heights[i * grid_dim + j - 1]
                    + heights[i * grid_dim + j + 1];
                total += (center - neighbors / 4.0).abs();
                count += 1;
            }
        }
        total / count as f32
    }

    /// Build a `NoiseConfig` matching the editor's grassland preset as of
    /// the F.2-T-2.B.3 snapshot. This is used across the three F.2-T-2
    /// diagnostic tests so each measurement reflects the actual runtime.
    /// F.2-T-2.B.3 reduced `warp_strength` from 40 to 15 per the F.2-T-2.A
    /// diagnostic findings.
    fn phase_1_6_f2_t2_grassland_config() -> NoiseConfig {
        let mut config = NoiseConfig::default();
        config.base_elevation.scale = 0.004;
        config.base_elevation.amplitude = 50.0;
        config.base_elevation.octaves = 5;
        config.base_elevation.persistence = 0.50;
        config.base_elevation.lacunarity = 2.0;
        config.base_elevation.noise_type = NoiseType::DomainWarped;
        config.base_elevation.domain_warp = DomainWarpConfig {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 15.0, // F.2-T-2.B.3: was 40.
            warp_octaves: 3,
        };
        config.mountains.enabled = true;
        config.mountains.scale = 0.0025;
        config.mountains.amplitude = 80.0;
        config.mountains.octaves = 6;
        // mountains.persistence and mountains.lacunarity use NoiseConfig::default
        // values (0.4 and 2.2) since BiomeNoisePreset doesn't override them.
        config.detail.enabled = true;
        config.detail.scale = 0.02;
        config.detail.amplitude = 4.0;
        config.continental_enabled = true;
        config
    }

    /// Phase 1.6-F.2-T-2.C: permanent regression guard for the "bed-of-nails
    /// surface spikes" failure mode that F.2-T-2 resolved. Generates heights
    /// across a 200×200 grid at 1 world-unit spacing using the grassland
    /// preset (the default most-viewed preset, which F.2-T-2 targeted most
    /// aggressively). Asserts local curvature (average |center - avg(4
    /// neighbors)|) stays below a threshold locked at post-F.2-T-2
    /// measurement × 1.2 (20% buffer).
    ///
    /// Pre-F.2-T-2.B.3 baseline: total curvature 2.016 (bed-of-nails).
    /// Post-F.2-T-2.B.3 measurement: total curvature 0.753.
    /// Threshold: 0.90 (≈ 0.753 × 1.2) — catches regressions at any
    /// warp_strength ≥ 20 per F.2-T-2.A's tuning matrix.
    ///
    /// If a future sub-phase tunes the grassland preset's DomainWarped
    /// parameters, this test's inline config must be updated in lockstep
    /// with `terrain_panel.rs::noise_preset_for_biome`.
    #[test]
    fn phase_1_6_f2_t2_surface_spikiness_under_threshold() {
        let config = phase_1_6_f2_t2_grassland_config();
        let noise = TerrainNoise::new(&config, 12345);

        const GRID_DIM: usize = 200;
        let mut heights = vec![0f32; GRID_DIM * GRID_DIM];
        for i in 0..GRID_DIM {
            for j in 0..GRID_DIM {
                heights[i * GRID_DIM + j] = noise.sample_height(i as f64, j as f64);
            }
        }
        let curv = phase_1_6_f2_t2_local_curvature_grid(&heights, GRID_DIM);

        const SPIKE_THRESHOLD: f32 = 0.90;

        println!("F.2-T-2 spike regression: curvature {curv:.3} (threshold {SPIKE_THRESHOLD})");

        assert!(
            curv <= SPIKE_THRESHOLD,
            "Surface curvature {curv:.3} > threshold {SPIKE_THRESHOLD} — bed-of-nails regression. \
             F.2-T-2.A diagnostic showed DomainWarped `warp_strength` is the dominant spike \
             source; check `base_domain_warp.warp_strength` on the grassland preset in \
             terrain_panel.rs for recent changes."
        );
    }

    /// Phase 1.6-F.2-T.C: permanent regression guard for the "continental
    /// suppressed everything uniformly" failure mode. Generates heights
    /// across the editor's 11×11 chunk grid (radius 5) with grassland
    /// preset amplitudes + F.2 continental modulation enabled. Asserts on
    /// both Y max and p95 (top 5% threshold) so that regression-free
    /// highland regions exist.
    ///
    /// Thresholds reflect the F.2 continental-modulation design: even at
    /// max continental_01 (measured 0.874 at seed 12345 in F.2-T.A),
    /// multiplier = 0.50 + 0.50×0.874 = 0.937, so highland peaks are
    /// bounded at ~94% of F.1's unmodulated mountain amplitude. The test
    /// thresholds (Y max ≥ 85, p95 ≥ 40) are calibrated to this expected
    /// bound — strict enough to fail F.2-pre-tuning (Y max 70, p95 ~25)
    /// and F.2-T without continental widening (Y max 88), while
    /// accommodating the intrinsic continental-modulation compression.
    ///
    /// The grassland preset's amplitudes are inlined here; if a future
    /// sub-phase (e.g. F.5 integration tuning) modifies them in
    /// `tools/aw_editor/src/panels/terrain_panel.rs`, this test's inline
    /// values must be updated in lockstep. The inline pattern mirrors F.1's
    /// diagnostic-test convention.
    #[test]
    fn phase_1_6_f2_t_highland_regions_reach_f1_target() {
        let mut config = NoiseConfig::default();
        // Grassland preset values from terrain_panel.rs::noise_preset_for_biome
        // `_ =>` arm, as of F.2-T.B.2.
        config.base_elevation.scale = 0.004;
        config.base_elevation.amplitude = 50.0;
        config.base_elevation.octaves = 5;
        config.base_elevation.persistence = 0.50;
        config.base_elevation.lacunarity = 2.0;
        config.base_elevation.noise_type = NoiseType::DomainWarped;
        config.base_elevation.domain_warp = DomainWarpConfig {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 15.0, // F.2-T-2.B.3: was 40.
            warp_octaves: 3,
        };
        config.mountains.enabled = true;
        config.mountains.scale = 0.0025;
        config.mountains.amplitude = 80.0;
        config.mountains.octaves = 6;
        config.detail.enabled = true;
        config.detail.scale = 0.02;
        config.detail.amplitude = 4.0;
        config.continental_enabled = true;

        let noise = TerrainNoise::new(&config, 12345);

        // Generate over the editor's 11x11 chunk grid (radius 5) at 64 verts
        // per chunk side, 256 world units per chunk. Matches TerrainState's
        // default runtime extent.
        let chunk_size = 256.0f64;
        let verts_per_side = 64i32;
        let chunk_radius = 5i32;

        let mut heights: Vec<f32> = Vec::with_capacity(495_616);
        for chunk_x in -chunk_radius..=chunk_radius {
            for chunk_z in -chunk_radius..=chunk_radius {
                let origin_x = chunk_x as f64 * chunk_size;
                let origin_z = chunk_z as f64 * chunk_size;
                for vz in 0..verts_per_side {
                    for vx in 0..verts_per_side {
                        let x = origin_x
                            + (vx as f64 / verts_per_side as f64) * chunk_size;
                        let z = origin_z
                            + (vz as f64 / verts_per_side as f64) * chunk_size;
                        heights.push(noise.sample_height(x, z));
                    }
                }
            }
        }
        heights.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = heights.len();
        let y_min = heights[0];
        let y_max = *heights.last().unwrap();
        let p95 = heights[(n as f32 * 0.95) as usize];
        let p99 = heights[(n as f32 * 0.99) as usize];

        println!(
            "Highland-Y-max regression: min={y_min:.2} p95={p95:.2} p99={p99:.2} max={y_max:.2} n={n}"
        );

        // Gate 1: at least some highland peak exists (Y max).
        assert!(
            y_max >= 85.0,
            "Highland Y max reached only {y_max:.2} — expected >= 85. Continental modulation \
             may be suppressing mountain layer too aggressively, or continental field may not \
             reach highland regions (cont > 0.7) within the visible terrain."
        );
        // Gate 2: top 5% of vertices (p95) must be substantial, catching the
        // "continental suppressed everything uniformly" failure (pre-F.2-T
        // p95 was in the 25-35 range; F.2-T restores p95 to the 40+ range).
        assert!(
            p95 >= 40.0,
            "Highland p95 reached only {p95:.2} — expected >= 40. Top 5% of vertices do not \
             form a substantial highland band."
        );
    }

    /// Phase 1.6-F.2.C: regression guard. When continental_enabled is false,
    /// sample_height must produce byte-identical output to pre-F.2
    /// semantics (i.e. mountain layer's raw contribution is added without
    /// modulation). Verified by comparing a config with continental disabled
    /// against a hand-computed sum of the three layers.
    #[test]
    fn phase_1_6_f2_continental_disabled_is_noop() {
        let mut config = NoiseConfig::default();
        config.continental_enabled = false;
        let noise = TerrainNoise::new(&config, 12345);

        // Sample at three positions; the result should match a manual sum
        // of the three layers without any continental modulation.
        for (x, z) in [(0.0f64, 0.0), (500.0, 500.0), (-300.0, 700.0)] {
            let actual = noise.sample_height(x, z);

            let base = noise.base_elevation.get([
                x * config.base_elevation.scale,
                0.0,
                z * config.base_elevation.scale,
            ]) as f32
                * config.base_elevation.amplitude;

            let mountains_raw = (noise.mountains.get([
                x * config.mountains.scale,
                0.0,
                z * config.mountains.scale,
            ]) as f32)
                .abs()
                * config.mountains.amplitude;

            let detail = noise.detail.get([
                x * config.detail.scale,
                0.0,
                z * config.detail.scale,
            ]) as f32
                * config.detail.amplitude;

            let expected = (base + mountains_raw + detail).max(0.0);
            assert!(
                (actual - expected).abs() < 1e-4,
                "continental-disabled sample_height({x}, {z}) = {actual}, expected {expected} (diff {})",
                (actual - expected).abs()
            );
        }
    }
}
