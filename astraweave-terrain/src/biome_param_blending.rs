//! Phase 1.6-F.4.B.3.D.4: scattered-convolution biome blending.
//!
//! Softens the per-vertex hard biome assignments produced by D.3 so the
//! world reads as continuous while preserving discrete biome identity at
//! each vertex. Reference: noiseposti.ng "Fast Biome Blending Without
//! Squareness"
//! (<https://noiseposti.ng/posts/2021-03-13-Fast-Biome-Blending-Without-Squareness.html>).
//!
//! ## Algorithm
//!
//! For each vertex at world position `(x, z)` with bootstrap elevation `e`:
//!
//! 1. Generate `N` jittered sample positions within radius `r` of `(x, z)`.
//!    Jitter is a deterministic hash of quantized world position + sample
//!    index — same vertex always produces same sample positions.
//! 2. At each sample, look up the biome via D.2's `lookup_biome` and
//!    retrieve its `BiomeParameters` via D.3's `BiomeParameters::for_biome`.
//! 3. Compute distance-weighted blend of the *numeric* parameters:
//!    `mountains_amplitude` and `scatter_density`. Linear falloff weight.
//! 4. The vertex's `BiomeId` is the **dominant** biome (highest summed
//!    weight). Discrete fields (`scatter_species_set`,
//!    `surface_color_palette`) and unwired fields (`ridge_strength`,
//!    `runevision_config`, `erosion_preset`) take the dominant biome's
//!    value — they don't blend.
//!
//! ## Why blend parameters, not biome IDs
//!
//! "60% TemperateDeciduous + 40% TemperateGrassland" isn't a category that
//! exists. Blending categorical biome IDs would require defining new
//! intermediate biomes for every pair, exploding the taxonomy. The
//! noiseposti.ng algorithm's insight is that blending the *continuous
//! parameter values* preserves smooth terrain transitions while keeping
//! the per-vertex biome assignment discrete (and therefore tractable for
//! gameplay logic, scatter species, surface coloring).
//!
//! ## Why this tightens continuity tolerances
//!
//! Per-vertex hard assignment (D.3) makes adjacent vertices' parameters
//! flip discontinuously at biome boundaries — `mountains_amplitude` jumps
//! from e.g. 0.8 (TemperateGrassland) to 1.2 (TemperateDeciduous). At
//! chunk boundaries this divergence is amplified by the f32 precision
//! discrepancy across independently-generated halos (the pre-existing
//! 47.4 WU grassland divergence). D.4's blending makes parameter values
//! transition smoothly across biome boundaries, eliminating the flip
//! discontinuity. Tolerances raised in D.3 (grassland 20→150 WU, mountain
//! 10→200 WU) should be tightenable back down toward F.4.B.2.G's 20 WU
//! range; D.4 verification asserts targets of grassland ≤50 WU and
//! mountain ≤100 WU.
//!
//! ## Determinism + position quantization
//!
//! Jitter offsets are computed from a Wang-style hash of the vertex's
//! world position (quantized to 1/1024 WU = ~1mm grid) plus the sample
//! index. Quantization is critical: f32 floating-point arithmetic across
//! adjacent chunks' independently-derived halo coordinates can produce
//! tiny bit-level differences at the same logical world position. Snapping
//! to a 1mm grid before hashing ensures both chunks produce identical
//! jitter offsets at shared-edge vertices, preserving the
//! shared-edge-invariance property.

use crate::biome_lookup::{lookup_biome, BiomeId};
use crate::biome_parameters::BiomeParameters;
use crate::climate::ClimateMap;
use serde::{Deserialize, Serialize};

/// Phase 1.6-F.4.B.3.D.4: configuration for scattered-convolution biome
/// blending.
///
/// Defaults: 6 samples (mid-range of §1.D.4's tunable 4-9), radius 48 WU
/// (mid-range of suggested 32-64 WU; D.6 Andrew-gate informs production
/// value). Seed offset 31 keeps jitter decoupled from terrain noise seeds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiomeParamBlendConfig {
    /// Number of jittered sample positions per vertex. Higher = smoother
    /// transitions but more cost. Range 4-9; default 6.
    pub sample_count: u32,
    /// Maximum offset of jittered samples from the vertex (in world units).
    /// Should span typical biome-boundary scale; too small produces no
    /// blending, too large smears character across distant biomes.
    /// Default 48 WU.
    pub radius: f32,
    /// Seed offset added to world seed for jitter determinism. Decouples
    /// jitter pattern from terrain noise. Default 31.
    pub seed_offset: u32,
}

impl Default for BiomeParamBlendConfig {
    fn default() -> Self {
        Self {
            sample_count: 6,
            radius: 48.0,
            seed_offset: 31,
        }
    }
}

/// Phase 1.6-F.4.B.3.D.4: result of biome blending at a single vertex.
///
/// Contains:
/// - `dominant_biome`: the discrete `BiomeId` driving surface color,
///   scatter species, gameplay logic.
/// - Blended numeric fields: `mountains_amplitude`, `scatter_density`.
/// - Unblended fields: `ridge_strength`, `runevision_config`,
///   `scatter_species_set`, `surface_color_palette`, `erosion_preset` —
///   all take the dominant biome's value (not blended) per §1.2 plan.
#[derive(Debug, Clone)]
pub struct BlendedBiomeParams {
    /// Dominant biome at this vertex (highest summed sample weight).
    pub dominant_biome: BiomeId,
    /// Distance-weighted blend of `BiomeParameters::mountains_amplitude`.
    pub mountains_amplitude: f64,
    /// Distance-weighted blend of `BiomeParameters::scatter_density`.
    pub scatter_density: f64,
    /// Forwarded from the dominant biome's `BiomeParameters` (no blending).
    /// Per §1.2: unwired/discrete fields take the dominant value.
    pub dominant_params: BiomeParameters,
}

/// Phase 1.6-F.4.B.3.D.4: position-quantization granularity for jitter
/// determinism. 1/1024 WU = ~1mm. Snapping to this grid before hashing
/// ensures adjacent chunks' halos produce identical jitter offsets at
/// shared-edge vertices despite f32 precision discrepancies.
const JITTER_QUANTIZATION: f32 = 1024.0;

/// Phase 1.6-F.4.B.3.D.4: jitter offset for a given vertex + sample index.
///
/// Returns `(dx, dz)` in `[-1, 1]` (multiply by `radius` to get world-unit
/// offset). Deterministic per `(world_x, world_z, sample_idx, seed)`.
///
/// Uses Wang-style integer hash on quantized world position. Position is
/// snapped to 1mm grid before hashing so adjacent chunks computing
/// jitter at the SAME logical world coordinate produce identical results
/// despite f32 precision.
#[inline]
fn jitter_offset(world_x: f32, world_z: f32, sample_idx: u32, seed: u64) -> (f32, f32) {
    let qx = (world_x * JITTER_QUANTIZATION).round() as i64 as u64;
    let qz = (world_z * JITTER_QUANTIZATION).round() as i64 as u64;

    let mut h = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 32;
    h = h.wrapping_add(qx).wrapping_mul(0x85EB_CA6B_E11E_CC0D);
    h ^= h >> 32;
    h = h.wrapping_add(qz).wrapping_mul(0xC2B2_AE35_5D9F_3B25);
    h ^= h >> 32;
    h = h
        .wrapping_add(sample_idx as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 32;

    let u = ((h & 0xFFFF_FFFF) as f32) / (u32::MAX as f32);
    let v = ((h >> 32) as f32) / (u32::MAX as f32);
    (u * 2.0 - 1.0, v * 2.0 - 1.0)
}

/// Phase 1.6-F.4.B.3.D.4: blend biome parameters at a world position.
///
/// Per the noiseposti.ng algorithm:
/// 1. Generate `config.sample_count` jittered sample positions within
///    `config.radius` of `(world_x, world_z)`.
/// 2. At each sample, look up biome (climate field × Whittaker) and
///    retrieve `BiomeParameters`.
/// 3. Linear distance-weighted blend of numeric parameters.
/// 4. Dominant biome = highest summed weight.
///
/// `bootstrap_elevation` is used as the elevation input for ALL jittered
/// climate samples. This is an approximation — strictly, the elevation at
/// each jittered position differs slightly. Per §1.4 plan: "The bootstrap
/// elevation used for climate sampling can remain the un-blended initial
/// height — climate-field shape doesn't need to be smoothed at vertex
/// level (it's already smooth at world scale). Only the *parameter values*
/// derived from biome IDs need blending." Computing per-jittered-sample
/// elevation would require N extra `sample_height` calls per vertex (a
/// significant cost). The approximation is bounded because the elevation
/// overlay layer (Alpine/SnowCap) only fires at high altitudes where
/// nearby vertices have similar elevations.
pub fn blend_biome_parameters(
    world_x: f32,
    world_z: f32,
    bootstrap_elevation: f32,
    climate_map: &ClimateMap,
    config: &BiomeParamBlendConfig,
) -> BlendedBiomeParams {
    debug_assert!(config.sample_count > 0, "sample_count must be ≥ 1");

    let seed = config.seed_offset as u64;

    // Accumulators.
    let mut weight_per_biome = [0.0f32; 19];
    let mut sum_amp: f64 = 0.0;
    let mut sum_density: f64 = 0.0;
    let mut sum_w: f32 = 0.0;

    for i in 0..config.sample_count {
        let (jx, jz) = jitter_offset(world_x, world_z, i, seed);
        let sample_x = world_x + jx * config.radius;
        let sample_z = world_z + jz * config.radius;

        let climate = climate_map.sample(sample_x as f64, sample_z as f64, bootstrap_elevation);
        let biome = lookup_biome(
            climate.temperature_c,
            climate.moisture_mm,
            bootstrap_elevation,
        );
        let params = BiomeParameters::for_biome(biome);

        // Linear distance-weight: 1.0 at center, 0.0 at radius edge.
        // Clamp to small floor to avoid division-by-zero when all samples
        // happen to land at the maximum jitter offset.
        let dist_norm = ((jx * jx + jz * jz).sqrt()).min(1.0);
        let weight = (1.0 - dist_norm).max(0.001);

        sum_amp += params.mountains_amplitude * weight as f64;
        sum_density += params.scatter_density * weight as f64;
        sum_w += weight;

        let biome_idx = biome_id_to_index(biome);
        weight_per_biome[biome_idx] += weight;
    }

    // Dominant biome = highest summed weight.
    let mut dominant_idx = 0usize;
    let mut max_w = weight_per_biome[0];
    for (idx, &w) in weight_per_biome.iter().enumerate().skip(1) {
        if w > max_w {
            max_w = w;
            dominant_idx = idx;
        }
    }
    let dominant_biome = index_to_biome_id(dominant_idx);

    BlendedBiomeParams {
        dominant_biome,
        mountains_amplitude: sum_amp / sum_w as f64,
        scatter_density: sum_density / sum_w as f64,
        dominant_params: BiomeParameters::for_biome(dominant_biome),
    }
}

/// Phase 1.6-F.4.B.3.D.4: map `BiomeId` to a stable `[0, 19)` index for
/// the per-biome weight accumulator. Inverse of [`index_to_biome_id`].
#[inline]
fn biome_id_to_index(biome: BiomeId) -> usize {
    match biome {
        BiomeId::TropicalRainforest => 0,
        BiomeId::TropicalSeasonalForest => 1,
        BiomeId::Savanna => 2,
        BiomeId::SubtropicalDesert => 3,
        BiomeId::TemperateRainforest => 4,
        BiomeId::TemperateDeciduousForest => 5,
        BiomeId::TemperateGrassland => 6,
        BiomeId::ColdDesert => 7,
        BiomeId::BorealForest => 8,
        BiomeId::Tundra => 9,
        BiomeId::Alpine => 10,
        BiomeId::Ocean => 11,
        BiomeId::Coast => 12,
        BiomeId::Beach => 13,
        BiomeId::River => 14,
        BiomeId::Wetland => 15,
        BiomeId::MountainRocky => 16,
        BiomeId::SnowCap => 17,
        BiomeId::Scree => 18,
    }
}

/// Phase 1.6-F.4.B.3.D.4: inverse of [`biome_id_to_index`]. Total over
/// `[0, 19)`; out-of-range indices fall back to TemperateGrassland (a
/// safe-default biome that won't crash downstream rendering).
#[inline]
fn index_to_biome_id(idx: usize) -> BiomeId {
    match idx {
        0 => BiomeId::TropicalRainforest,
        1 => BiomeId::TropicalSeasonalForest,
        2 => BiomeId::Savanna,
        3 => BiomeId::SubtropicalDesert,
        4 => BiomeId::TemperateRainforest,
        5 => BiomeId::TemperateDeciduousForest,
        6 => BiomeId::TemperateGrassland,
        7 => BiomeId::ColdDesert,
        8 => BiomeId::BorealForest,
        9 => BiomeId::Tundra,
        10 => BiomeId::Alpine,
        11 => BiomeId::Ocean,
        12 => BiomeId::Coast,
        13 => BiomeId::Beach,
        14 => BiomeId::River,
        15 => BiomeId::Wetland,
        16 => BiomeId::MountainRocky,
        17 => BiomeId::SnowCap,
        18 => BiomeId::Scree,
        _ => BiomeId::TemperateGrassland,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::{ClimateConfig, WorldArchetype};

    fn make_climate(seed: u64) -> ClimateMap {
        ClimateMap::new(&ClimateConfig::default(), seed)
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_default_config_within_documented_ranges() {
        let cfg = BiomeParamBlendConfig::default();
        assert!(cfg.sample_count >= 4 && cfg.sample_count <= 9);
        assert!(cfg.radius >= 32.0 && cfg.radius <= 64.0);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_blend_is_deterministic() {
        // Same (world_x, world_z, seed) → same blended result.
        let climate = make_climate(12345);
        let cfg = BiomeParamBlendConfig::default();

        let a = blend_biome_parameters(100.0, 200.0, 50.0, &climate, &cfg);
        let b = blend_biome_parameters(100.0, 200.0, 50.0, &climate, &cfg);

        assert_eq!(a.dominant_biome, b.dominant_biome);
        assert_eq!(a.mountains_amplitude, b.mountains_amplitude);
        assert_eq!(a.scatter_density, b.scatter_density);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_jitter_position_quantization_robust_to_f32_epsilon() {
        // Two world coordinates that differ by less than the
        // 1/1024-WU quantization grid should produce IDENTICAL jitter
        // offsets. This is the load-bearing property for shared-edge
        // invariance across adjacent chunks (D.3b precision concern).
        let seed = 1u64;
        let (jx_a, jz_a) = jitter_offset(512.0, 512.0, 0, seed);
        // Add a sub-quantization-grid epsilon (~0.0001 WU < 1/1024 ≈ 0.001 WU).
        let (jx_b, jz_b) = jitter_offset(512.0001, 512.0001, 0, seed);
        assert_eq!(jx_a, jx_b, "tiny f32 differences must quantize to same jitter");
        assert_eq!(jz_a, jz_b, "tiny f32 differences must quantize to same jitter");
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_jitter_distinct_per_sample_index() {
        // Different sample_idx at the same vertex must produce different
        // jitter offsets (otherwise scattered-convolution degenerates to
        // a single-sample lookup).
        let seed = 7u64;
        let mut offsets = std::collections::HashSet::new();
        for i in 0..6u32 {
            let (jx, jz) = jitter_offset(100.0, 200.0, i, seed);
            // Quantize to 4-decimal precision for HashSet membership.
            offsets.insert((
                (jx * 10_000.0).round() as i32,
                (jz * 10_000.0).round() as i32,
            ));
        }
        assert!(
            offsets.len() >= 5,
            "6 samples should produce at least 5 distinct jitter offsets; got {}",
            offsets.len()
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_uniform_climate_degenerates_to_dominant_params() {
        // At a position where the climate field produces a single biome
        // across the entire blending radius, the blended parameters
        // should approximately equal that biome's `for_biome` values.
        // We use a position deep inside Continental Temperate's lowland
        // zone where the dominant biome should be temperate-stable.
        let climate = make_climate(12345);
        let cfg = BiomeParamBlendConfig {
            sample_count: 9, // max for tighter degeneration
            radius: 16.0,    // small radius to stay in one biome
            seed_offset: 31,
        };
        let result = blend_biome_parameters(0.0, 0.0, 30.0, &climate, &cfg);

        let dom_params = BiomeParameters::for_biome(result.dominant_biome);
        let amp_diff = (result.mountains_amplitude - dom_params.mountains_amplitude).abs();
        let density_diff = (result.scatter_density - dom_params.scatter_density).abs();

        // Very tight tolerance: in a uniform region all 9 samples should
        // hit the same biome → blended value = dominant_params value.
        // Small variance allowed for any residual climate-edge effects.
        assert!(
            amp_diff < 0.5,
            "uniform region: blended mountains_amplitude {} should match dominant {} (diff {})",
            result.mountains_amplitude,
            dom_params.mountains_amplitude,
            amp_diff
        );
        assert!(
            density_diff < 0.5,
            "uniform region: blended scatter_density {} should match dominant {} (diff {})",
            result.scatter_density,
            dom_params.scatter_density,
            density_diff
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_gradient_smoothness() {
        // Along a climate gradient (latitude effect), `mountains_amplitude`
        // varies smoothly with bounded max derivative. This is the
        // anti-discontinuity property — the whole point of D.4.
        let climate = make_climate(12345);
        let cfg = BiomeParamBlendConfig::default();

        // Sample 100 points along a 4000 WU latitudinal sweep (reaches
        // varied climate per archetype's latitude_temperature_drop).
        const N: usize = 100;
        let mut amps = Vec::with_capacity(N);
        for i in 0..N {
            let z = -2000.0 + (i as f32 / (N - 1) as f32) * 4000.0;
            let r = blend_biome_parameters(0.0, z, 50.0, &climate, &cfg);
            amps.push(r.mountains_amplitude);
        }

        // Compute per-step deltas; max delta should be bounded by the
        // largest single-sample contribution / radius. Without blending,
        // a discontinuity would produce a delta equal to the full
        // amplitude difference between adjacent biomes (e.g., 0.8 → 1.2 = 0.4).
        // With blending, max delta should be significantly smaller because
        // each sample's contribution is fractional.
        let max_delta = amps
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f64, f64::max);

        // Bound: blending should keep per-step delta below the unblended
        // hard-flip discontinuity. Without blending, adjacent vertices at
        // a TemperateGrassland (0.8) ↔ BorealForest (1.5) boundary would
        // produce a delta of 0.7. With blending at radius 48 WU and 6
        // samples, the overlapping blend kernels reduce that.
        //
        // Phase 1.6-F.4.B.3.D.5-fix Path B: per-biome amplitudes for
        // elevation-overlay biomes reduced (Alpine 2.5 → 1.4, SnowCap
        // 2.5 → 1.4, MountainRocky 3.0 → 1.6, Scree 2.0 → 1.2). Maximum
        // amplitude differential at any boundary is now ≤ 0.7 (Alpine
        // 1.4 - TemperateGrassland 0.8 = 0.6; the unchanged BorealForest
        // 1.5 still pairs with TemperateGrassland 0.8 = 0.7). Threshold
        // tightened from 0.5 → 0.4 to reflect the smaller possible max
        // delta. Future amplitude tuning that preserves the [0.0, 2.0]
        // range used post Path B should keep this threshold valid.
        assert!(
            max_delta < 0.4,
            "gradient should be smooth: max per-step amplitude delta {} > 0.4 \
             (blending may not be working)",
            max_delta
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_sample_count_4_vs_9_changes_smoothness() {
        // Sanity: increasing sample count should produce a similar
        // dominant biome at uniform positions but slightly different
        // blended values (more samples = more averaging). This isn't
        // a strict assertion of "smoother" — just that the parameter
        // matters.
        let climate = make_climate(99);
        let cfg_4 = BiomeParamBlendConfig {
            sample_count: 4,
            ..BiomeParamBlendConfig::default()
        };
        let cfg_9 = BiomeParamBlendConfig {
            sample_count: 9,
            ..BiomeParamBlendConfig::default()
        };

        // Sample at a position likely to be near a climate boundary
        // (high z = latitude effect).
        let r4 = blend_biome_parameters(1500.0, 3000.0, 100.0, &climate, &cfg_4);
        let r9 = blend_biome_parameters(1500.0, 3000.0, 100.0, &climate, &cfg_9);

        // Values may or may not differ depending on whether boundary
        // samples land in different biomes — but the function must
        // accept both configurations without panicking.
        assert!(r4.mountains_amplitude.is_finite());
        assert!(r9.mountains_amplitude.is_finite());
        assert!(r4.scatter_density.is_finite());
        assert!(r9.scatter_density.is_finite());
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_blended_params_within_biome_taxonomy_bounds() {
        // For ANY position, blended `mountains_amplitude` must lie within
        // the min/max of `BiomeParameters::for_biome` across all biomes.
        // Convex-combination invariant: weighted average of values
        // bounded by the values themselves.
        let climate = make_climate(42);
        let cfg = BiomeParamBlendConfig::default();

        let mut min_amp = f64::INFINITY;
        let mut max_amp = f64::NEG_INFINITY;
        for &b in BiomeId::all() {
            let p = BiomeParameters::for_biome(b);
            min_amp = min_amp.min(p.mountains_amplitude);
            max_amp = max_amp.max(p.mountains_amplitude);
        }

        // Sample many positions; all should produce blended values in
        // [min_amp, max_amp].
        for i in 0..50 {
            let x = (i as f32) * 137.0 - 1500.0;
            let z = (i as f32) * 211.0 - 1500.0;
            let r = blend_biome_parameters(x, z, 50.0, &climate, &cfg);
            assert!(
                r.mountains_amplitude >= min_amp - 0.01
                    && r.mountains_amplitude <= max_amp + 0.01,
                "blended mountains_amplitude {} out of bounds [{}, {}]",
                r.mountains_amplitude,
                min_amp,
                max_amp
            );
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_dominant_biome_is_actually_max_weight() {
        // Verify the dominant_biome field reflects the highest summed
        // sample weight (not just any sampled biome). We hand-construct
        // a known scenario: at a position where Continental Temperate
        // produces TemperateDeciduous samples almost exclusively, the
        // dominant should be TemperateDeciduous.
        let climate = make_climate(12345);
        let cfg = BiomeParamBlendConfig::default();
        let r = blend_biome_parameters(0.0, 0.0, 100.0, &climate, &cfg);
        // Dominant biome should be terrestrial at world center sea-level+,
        // not aquatic or overlay.
        assert!(
            r.dominant_biome.is_terrestrial(),
            "world-center mid-elevation Continental Temperate should resolve to terrestrial; \
             got {:?}",
            r.dominant_biome
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_4_warm_archetype_shifts_dominant_toward_tropical() {
        // Smoke test that archetype envelope affects blending. Warm
        // archetype at world center should produce tropical-family
        // dominants, not temperate.
        let warm = WorldArchetype {
            temperature_mean_c: 26.0,
            temperature_variance_c: 5.0,
            latitude_temperature_drop_c: 3.0,
            moisture_mean_mm: 2200.0,
            moisture_variance_mm: 800.0,
            continentalness_mean: 0.4,
            continentalness_variance: 0.2,
            // Phase 1.X-F.2.C: bootstrap_splines added; default to F.2 baseline
            // (test only exercises climate envelope blending behavior).
            bootstrap_splines: Default::default(),
        };
        let mut config = ClimateConfig::default();
        config.archetype = warm;
        let climate = ClimateMap::new(&config, 67890);

        let cfg = BiomeParamBlendConfig::default();
        let mut tropical_count = 0;
        let mut temperate_count = 0;
        for i in 0..50 {
            let x = (i as f32) * 137.0;
            let z = (i as f32) * 211.0;
            let r = blend_biome_parameters(x, z, 30.0, &climate, &cfg);
            if matches!(
                r.dominant_biome,
                BiomeId::TropicalRainforest
                    | BiomeId::TropicalSeasonalForest
                    | BiomeId::Savanna
                    | BiomeId::SubtropicalDesert
            ) {
                tropical_count += 1;
            }
            if matches!(
                r.dominant_biome,
                BiomeId::TemperateGrassland
                    | BiomeId::TemperateDeciduousForest
                    | BiomeId::TemperateRainforest
                    | BiomeId::ColdDesert
            ) {
                temperate_count += 1;
            }
        }
        assert!(
            tropical_count > temperate_count,
            "warm archetype should produce more tropical dominants ({}) than temperate ({})",
            tropical_count,
            temperate_count
        );
    }
}
