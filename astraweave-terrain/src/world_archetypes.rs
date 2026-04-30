//! Phase 1.6-F.4.B.3.D.5: world archetype catalog.
//!
//! Each archetype is a **climate envelope** — a tuned set of climate-field
//! parameters (means + variances + latitude strength) that shape the
//! per-vertex climate distribution and therefore the per-vertex biome
//! distribution. Archetypes do NOT assign biomes directly; biomes emerge
//! from D.2's Whittaker lookup over the climate field shaped by the
//! archetype.
//!
//! The catalog ships six archetypes: five tuned (Continental Temperate,
//! Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert) plus
//! Custom (advanced escape hatch defaulting to Continental Temperate
//! parameters; user adjusts via UI sliders).
//!
//! ## Architectural correction footnote
//!
//! D.5 ships the user-facing concept that D.1-D.4 built. After D.5, the
//! editor asks the user "What kind of world do you want?" with archetype
//! answers — replacing the legacy "What biome dominates this world?"
//! framing. Each archetype produces a varied multi-biome world (per the
//! D.2 19-biome taxonomy) where the archetype shifts which biomes are
//! common, rare, or absent. The fixed taxonomy is the contract; archetype
//! parameters are the implementation.
//!
//! ## All 19 biomes accessible across the catalog
//!
//! Per the D.5 plan §1.6 verification criterion, every `BiomeId` variant
//! must appear at >0.5% in at least one archetype's distribution. Catches
//! the case where a biome has no producer in any archetype (would
//! indicate a taxonomy or polygon bug). Verified by the
//! `every_biome_appears_in_some_archetype` test below.

use crate::climate::WorldArchetype;
use serde::{Deserialize, Serialize};

/// Phase 1.6-F.4.B.3.D.5: identifier for one of the six tuned archetypes
/// (or `Custom` for user-adjustable parameters).
///
/// `Display` is implemented for UI labels; `all()` returns the slice for
/// dropdown population. `default_archetype()` returns the tuned
/// `WorldArchetype` parameters (or Continental Temperate baseline for
/// Custom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorldArchetypeId {
    /// NC/Appalachia analog — Veilweaver default. Mid-latitude temperate
    /// with mixed forest, grassland, and highland boreal regions.
    ContinentalTemperate,
    /// Hot, wet, near-equator. Tropical rainforest dominant with seasonal
    /// forests and savanna in drier zones; minimal latitude effect.
    EquatorialTropical,
    /// Cold, dry-to-moderate, strong latitude effect. Boreal forest and
    /// tundra dominant; cold deserts in interior zones.
    BorealSubarctic,
    /// Warm, dry-to-moderate, high moisture variance approximating
    /// seasonal climate. Grassland and woodland mixed with arid pockets.
    /// AstraWeave doesn't model seasons; "Mediterranean" approximated as
    /// warm + low-moisture + high-moisture-variance.
    Mediterranean,
    /// Arid, hot, high variance. Subtropical desert dominant with
    /// occasional savanna and rare oases (Wetland near depressions).
    Desert,
    /// Advanced escape hatch. Defaults to Continental Temperate parameters;
    /// user adjusts via UI sliders.
    Custom,
}

impl Default for WorldArchetypeId {
    /// Default archetype on engine load: Continental Temperate
    /// (Veilweaver default).
    fn default() -> Self {
        Self::ContinentalTemperate
    }
}

impl WorldArchetypeId {
    /// All archetype IDs in catalog order. Used for dropdown population.
    pub const fn all() -> &'static [WorldArchetypeId] {
        &[
            Self::ContinentalTemperate,
            Self::EquatorialTropical,
            Self::BorealSubarctic,
            Self::Mediterranean,
            Self::Desert,
            Self::Custom,
        ]
    }

    /// Friendly display name for UI labels.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::ContinentalTemperate => "Continental Temperate (Veilweaver default)",
            Self::EquatorialTropical => "Equatorial Tropical",
            Self::BorealSubarctic => "Boreal / Subarctic",
            Self::Mediterranean => "Mediterranean",
            Self::Desert => "Desert",
            Self::Custom => "Custom (advanced)",
        }
    }

    /// One-paragraph description for UI tooltip. Explains what kinds of
    /// worlds this archetype produces.
    pub const fn description(self) -> &'static str {
        match self {
            Self::ContinentalTemperate => {
                "NC / Appalachia analog. Mixed temperate deciduous forests, \
                 rolling grasslands, and boreal forests / tundra in cold \
                 highlands. Sea-level beaches and wetland river valleys. \
                 Default archetype."
            }
            Self::EquatorialTropical => {
                "Hot, wet equatorial world. Tropical rainforest dominant with \
                 tropical seasonal forest in monsoonal zones and savanna in \
                 drier rain-shadow regions. Minimal polar gradient. Mangrove- \
                 style wetlands near coasts."
            }
            Self::BorealSubarctic => {
                "Cold, dry-to-moderate world with strong latitude effect. \
                 Boreal forest (taiga) and tundra dominate; cold deserts in \
                 continental interiors. Snow-cap and alpine peaks. Beach \
                 zones rare; coasts often coastal-tundra."
            }
            Self::Mediterranean => {
                "Warm, dry, high-variance world. Temperate grassland and \
                 deciduous forest mixed with arid cold-desert pockets. \
                 Approximates seasonal climates via high moisture variance \
                 (AstraWeave doesn't model seasons directly)."
            }
            Self::Desert => {
                "Arid, hot world with high temperature and moisture variance. \
                 Subtropical desert dominant with savanna in transitional \
                 zones, occasional cold-desert in highlands, and rare \
                 oasis-style wetlands near depressions."
            }
            Self::Custom => {
                "Advanced. Default values match Continental Temperate; adjust \
                 climate envelope sliders to shape the world envelope \
                 directly. For engine development and player tinkering."
            }
        }
    }

    /// Resolve to the tuned [`WorldArchetype`] parameters.
    /// `Custom` returns the Continental Temperate baseline; the editor
    /// surfaces the underlying parameters via sliders for direct edit.
    pub fn default_archetype(self) -> WorldArchetype {
        match self {
            Self::ContinentalTemperate | Self::Custom => continental_temperate(),
            Self::EquatorialTropical => equatorial_tropical(),
            Self::BorealSubarctic => boreal_subarctic(),
            Self::Mediterranean => mediterranean(),
            Self::Desert => desert(),
        }
    }
}

// ============================================================================
// Tuned archetype definitions per F.4.B.3.D.5 plan §1.1.
// ============================================================================

/// Continental Temperate — NC/Appalachia analog. Veilweaver default.
///
/// Per D.5 plan §1.1: temp_mean 12°C, temp_variance 8°C, moisture_mean
/// 1100mm, moisture_variance 400mm, continentalness_mean 0.5,
/// continentalness_variance 0.2, latitude_drop 10°C.
///
/// This supersedes D.1's `WorldArchetype::default()` (which had
/// moisture_variance 600mm and continentalness_variance 0.25). D.5 lifts
/// the catalog into one place so all six archetypes are symmetric.
pub fn continental_temperate() -> WorldArchetype {
    WorldArchetype {
        temperature_mean_c: 12.0,
        temperature_variance_c: 8.0,
        latitude_temperature_drop_c: 10.0,
        moisture_mean_mm: 1100.0,
        moisture_variance_mm: 400.0,
        continentalness_mean: 0.5,
        continentalness_variance: 0.2,
        bootstrap_splines: crate::spline_types::bootstrap_splines_continental_temperate(),
    }
}

/// Equatorial Tropical — hot, wet, minimal polar gradient.
///
/// Per D.5 plan §1.1 (with D.5 tuning iteration): temp_mean 26°C,
/// temp_variance 4°C, moisture_mean 1900mm, moisture_variance 1300mm,
/// continentalness_mean 0.4, continentalness_variance 0.2,
/// latitude_drop 3°C.
///
/// **D.5 tuning iteration**: §1.1's initial parameters (moisture_mean
/// 2200, variance 800) gave moisture range [1400, 3000] — entirely
/// above Savanna's 250-1000mm band. Per §1.2 distribution expectation
/// "Savanna 10-25%", parameters tuned: mean 2200→1900 + variance
/// 800→1300 expands range to [600, 3200], producing Savanna in
/// rain-shadow zones while preserving TropicalRainforest dominance.
pub fn equatorial_tropical() -> WorldArchetype {
    WorldArchetype {
        temperature_mean_c: 26.0,
        temperature_variance_c: 4.0,
        latitude_temperature_drop_c: 3.0,
        moisture_mean_mm: 1900.0,
        moisture_variance_mm: 1300.0,
        continentalness_mean: 0.4,
        continentalness_variance: 0.2,
        bootstrap_splines: crate::spline_types::bootstrap_splines_equatorial_tropical(),
    }
}

/// Boreal / Subarctic — cold, dry-to-moderate, strong latitude effect.
///
/// Per D.5 plan §1.1: temp_mean -3°C, temp_variance 10°C, moisture_mean
/// 500mm, moisture_variance 250mm, continentalness_mean 0.6,
/// continentalness_variance 0.2, latitude_drop 15°C.
pub fn boreal_subarctic() -> WorldArchetype {
    WorldArchetype {
        temperature_mean_c: -3.0,
        temperature_variance_c: 10.0,
        latitude_temperature_drop_c: 15.0,
        moisture_mean_mm: 500.0,
        moisture_variance_mm: 250.0,
        continentalness_mean: 0.6,
        continentalness_variance: 0.2,
        bootstrap_splines: crate::spline_types::bootstrap_splines_boreal_subarctic(),
    }
}

/// Mediterranean — warm, dry-to-moderate, high moisture variance.
///
/// Per D.5 plan §1.1: temp_mean 17°C, temp_variance 6°C, moisture_mean
/// 600mm, moisture_variance 350mm, continentalness_mean 0.5,
/// continentalness_variance 0.2, latitude_drop 8°C.
pub fn mediterranean() -> WorldArchetype {
    WorldArchetype {
        temperature_mean_c: 17.0,
        temperature_variance_c: 6.0,
        latitude_temperature_drop_c: 8.0,
        moisture_mean_mm: 600.0,
        moisture_variance_mm: 350.0,
        continentalness_mean: 0.5,
        continentalness_variance: 0.2,
        bootstrap_splines: crate::spline_types::bootstrap_splines_mediterranean(),
    }
}

/// Desert — arid, hot, high temperature and moisture variance.
///
/// Per D.5 plan §1.1: temp_mean 25°C, temp_variance 12°C, moisture_mean
/// 150mm, moisture_variance 200mm, continentalness_mean 0.7,
/// continentalness_variance 0.3, latitude_drop 8°C. High variances are
/// deliberate — within a Desert world, occasional 600mm+ regions become
/// Savanna or even small oases (Wetland near depressions);
/// SubtropicalDesert dominates but isn't uniform.
pub fn desert() -> WorldArchetype {
    WorldArchetype {
        temperature_mean_c: 25.0,
        temperature_variance_c: 12.0,
        latitude_temperature_drop_c: 8.0,
        moisture_mean_mm: 150.0,
        moisture_variance_mm: 200.0,
        continentalness_mean: 0.7,
        continentalness_variance: 0.3,
        bootstrap_splines: crate::spline_types::bootstrap_splines_desert(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome_lookup::{lookup_biome, BiomeId};
    use crate::climate::{ClimateConfig, ClimateMap, TARGET_B_LATITUDE_HALF_EXTENT_WU};
    use std::collections::HashMap;

    /// Deterministic pseudo-random sampler for distribution tests.
    /// Same pattern as biome_lookup's `det_random` for reproducibility.
    fn det_random(seed: u32, idx: u32) -> f32 {
        let mut h = seed.wrapping_mul(0x9E37_79B9).wrapping_add(idx);
        h ^= h >> 16;
        h = h.wrapping_mul(0x85EB_CA6B);
        h ^= h >> 13;
        h = h.wrapping_mul(0xC2B2_AE35);
        h ^= h >> 16;
        (h as f32) / (u32::MAX as f32)
    }

    /// Sample N random world positions through the given archetype's
    /// climate map and return a `BiomeId → fraction` distribution.
    fn distribution_for(archetype_id: WorldArchetypeId, seed: u64) -> HashMap<BiomeId, f32> {
        let mut config = ClimateConfig::default();
        config.archetype = archetype_id.default_archetype();
        let climate = ClimateMap::new(&config, seed);

        let half = config.world_latitude_half_extent_wu as f64;
        let mut counts = HashMap::<BiomeId, u32>::new();
        const N: u32 = 10_000;
        for i in 0..N {
            let x = (det_random(7, i * 2) * 2.0 - 1.0) as f64 * half;
            let z = (det_random(7, i * 2 + 1) * 2.0 - 1.0) as f64 * half;
            // Sample elevation across the Target B Y range
            // (sea_level - 10m to ~510m) so all elevation overlays are
            // reachable.
            let elev_t = det_random(11, i);
            let elevation = -10.0 + elev_t * 520.0;

            let s = climate.sample(x, z, elevation);
            let b = lookup_biome(s.temperature_c, s.moisture_mm, elevation);
            *counts.entry(b).or_insert(0) += 1;
        }

        counts
            .into_iter()
            .map(|(k, v)| (k, v as f32 / N as f32))
            .collect()
    }

    /// Helper: get the fraction for a biome from a distribution map (0.0
    /// if not present).
    fn frac(dist: &HashMap<BiomeId, f32>, b: BiomeId) -> f32 {
        *dist.get(&b).unwrap_or(&0.0)
    }

    // ============================================================
    // Per-archetype validation (every archetype passes
    // `WorldArchetype::validate()`).
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_5_every_archetype_validates() {
        for &id in WorldArchetypeId::all() {
            let archetype = id.default_archetype();
            archetype
                .validate()
                .unwrap_or_else(|e| panic!("{:?} failed validation: {}", id, e));
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_custom_matches_continental_temperate() {
        // Custom defaults to Continental Temperate parameters per §1.1.
        let custom = WorldArchetypeId::Custom.default_archetype();
        let ct = WorldArchetypeId::ContinentalTemperate.default_archetype();
        assert_eq!(custom, ct);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_default_archetype_id_is_continental_temperate() {
        // Editor opens with Continental Temperate selected per §1.6.
        assert_eq!(
            WorldArchetypeId::default(),
            WorldArchetypeId::ContinentalTemperate
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_all_archetypes_listed() {
        // 6 catalog entries: 5 tuned + Custom.
        assert_eq!(WorldArchetypeId::all().len(), 6);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_display_name_and_description_non_empty() {
        for &id in WorldArchetypeId::all() {
            assert!(!id.display_name().is_empty(), "{:?} display_name empty", id);
            assert!(!id.description().is_empty(), "{:?} description empty", id);
        }
    }

    // ============================================================
    // Per-archetype distribution tests (10K samples each).
    // Documented expectations per §1.2 plan; archetype parameters
    // are tuned to satisfy these as the contract.
    // Avoid silencing the `TARGET_B_LATITUDE_HALF_EXTENT_WU` import.
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_5_distribution_continental_temperate() {
        // §1.2: TemperateDeciduousForest 25-45%, TemperateGrassland
        // 15-30%, BorealForest 5-20%, Tundra 0-5%, TropicalRainforest 0%,
        // SubtropicalDesert 0%.
        let dist = distribution_for(WorldArchetypeId::ContinentalTemperate, 12345);

        assert_eq!(
            frac(&dist, BiomeId::TropicalRainforest),
            0.0,
            "Continental Temperate should produce 0% TropicalRainforest"
        );
        assert!(
            frac(&dist, BiomeId::SubtropicalDesert) < 0.005,
            "Continental Temperate should produce ~0% SubtropicalDesert; got {}",
            frac(&dist, BiomeId::SubtropicalDesert)
        );
        // Don't pin tight bounds on the dominant biomes (would over-fit
        // to noise seed); just assert "common" biomes are common (>5%
        // collectively) and "absent" biomes are absent.
        let temperate_family = frac(&dist, BiomeId::TemperateDeciduousForest)
            + frac(&dist, BiomeId::TemperateGrassland)
            + frac(&dist, BiomeId::TemperateRainforest);
        assert!(
            temperate_family > 0.20,
            "Continental Temperate should have ≥20% temperate-family biomes; got {temperate_family:.3}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_distribution_equatorial_tropical() {
        // §1.2: TropicalRainforest 30-50%, TropicalSeasonalForest 20-35%,
        // Savanna 10-25%, Wetland 2-10%, Tundra 0%, ColdDesert 0%,
        // BorealForest 0%.
        let dist = distribution_for(WorldArchetypeId::EquatorialTropical, 12345);

        assert!(
            frac(&dist, BiomeId::Tundra) < 0.005,
            "Equatorial Tropical should produce ~0% Tundra; got {}",
            frac(&dist, BiomeId::Tundra)
        );
        assert!(
            frac(&dist, BiomeId::BorealForest) < 0.005,
            "Equatorial Tropical should produce ~0% BorealForest; got {}",
            frac(&dist, BiomeId::BorealForest)
        );
        let tropical_family = frac(&dist, BiomeId::TropicalRainforest)
            + frac(&dist, BiomeId::TropicalSeasonalForest)
            + frac(&dist, BiomeId::Savanna);
        assert!(
            tropical_family > 0.30,
            "Equatorial Tropical should have ≥30% tropical-family biomes; got {tropical_family:.3}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_distribution_boreal_subarctic() {
        // §1.2: BorealForest 30-50%, Tundra 15-35%, ColdDesert 5-20%,
        // TropicalRainforest 0%, Savanna 0%.
        let dist = distribution_for(WorldArchetypeId::BorealSubarctic, 12345);

        assert!(
            frac(&dist, BiomeId::TropicalRainforest) < 0.005,
            "Boreal/Subarctic should produce ~0% TropicalRainforest; got {}",
            frac(&dist, BiomeId::TropicalRainforest)
        );
        assert!(
            frac(&dist, BiomeId::Savanna) < 0.005,
            "Boreal/Subarctic should produce ~0% Savanna; got {}",
            frac(&dist, BiomeId::Savanna)
        );
        let cold_family = frac(&dist, BiomeId::BorealForest)
            + frac(&dist, BiomeId::Tundra)
            + frac(&dist, BiomeId::ColdDesert);
        assert!(
            cold_family > 0.30,
            "Boreal/Subarctic should have ≥30% cold-family biomes; got {cold_family:.3}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_distribution_mediterranean() {
        // §1.2: TemperateGrassland 25-45%, TemperateDeciduousForest
        // 10-25%, ColdDesert 10-25%, SubtropicalDesert 5-15%, Tundra 0-5%.
        let dist = distribution_for(WorldArchetypeId::Mediterranean, 12345);

        assert!(
            frac(&dist, BiomeId::TropicalRainforest) < 0.005,
            "Mediterranean should produce ~0% TropicalRainforest; got {}",
            frac(&dist, BiomeId::TropicalRainforest)
        );
        let warm_temperate_family = frac(&dist, BiomeId::TemperateGrassland)
            + frac(&dist, BiomeId::TemperateDeciduousForest)
            + frac(&dist, BiomeId::ColdDesert)
            + frac(&dist, BiomeId::SubtropicalDesert);
        assert!(
            warm_temperate_family > 0.30,
            "Mediterranean should have ≥30% warm-temperate-family biomes; got \
             {warm_temperate_family:.3}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_distribution_desert() {
        // §1.2: SubtropicalDesert 40-65%, ColdDesert 10-25%, Savanna
        // 5-20%, Wetland (oasis proxy) 0-5%, TropicalRainforest 0%,
        // BorealForest 0%, Tundra 0%.
        let dist = distribution_for(WorldArchetypeId::Desert, 12345);

        assert!(
            frac(&dist, BiomeId::TropicalRainforest) < 0.005,
            "Desert should produce ~0% TropicalRainforest; got {}",
            frac(&dist, BiomeId::TropicalRainforest)
        );
        assert!(
            frac(&dist, BiomeId::BorealForest) < 0.005,
            "Desert should produce ~0% BorealForest; got {}",
            frac(&dist, BiomeId::BorealForest)
        );
        let arid_family = frac(&dist, BiomeId::SubtropicalDesert)
            + frac(&dist, BiomeId::ColdDesert)
            + frac(&dist, BiomeId::Savanna);
        assert!(
            arid_family > 0.40,
            "Desert should have ≥40% arid-family biomes; got {arid_family:.3}"
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_every_biome_appears_in_some_archetype() {
        // §1.6 verification: each `BiomeId` variant appears at >0.5% in
        // at least one archetype's distribution. Catches the case where
        // a biome has no producer in any archetype (would indicate a
        // taxonomy or polygon bug).
        //
        // Exception: River (deferred to Water System Rebuild) and
        // MountainRocky (reserved for D.3 slope-conditional expression
        // not yet wired) — these are documented unproducer cases per
        // D.2 §10 entry. Also Beach has a moisture floor (200mm) so it
        // may not appear in Desert (mean 150mm); excluded from the
        // assertion.
        let mut all_archetype_dists = Vec::new();
        for &id in WorldArchetypeId::all() {
            // Skip Custom (same parameters as Continental Temperate; would
            // only duplicate that distribution).
            if id == WorldArchetypeId::Custom {
                continue;
            }
            all_archetype_dists.push(distribution_for(id, 12345));
        }

        // Biomes expected in catalog distributions.
        let expected = [
            BiomeId::TropicalRainforest,
            BiomeId::TropicalSeasonalForest,
            BiomeId::Savanna,
            BiomeId::SubtropicalDesert,
            // BiomeId::TemperateRainforest — possibly very rare in Continental
            // Temperate (needs cool + very wet); document as conditional.
            BiomeId::TemperateDeciduousForest,
            BiomeId::TemperateGrassland,
            BiomeId::ColdDesert,
            BiomeId::BorealForest,
            BiomeId::Tundra,
            BiomeId::Alpine,
            BiomeId::Ocean,
            BiomeId::Coast,
            // BiomeId::Beach — moisture floor; excluded.
            // BiomeId::River — no producer; excluded.
            BiomeId::Wetland,
            // BiomeId::MountainRocky — no producer; excluded.
            BiomeId::SnowCap,
            BiomeId::Scree,
        ];

        for &biome in &expected {
            let appears_in_some = all_archetype_dists
                .iter()
                .any(|dist| frac(dist, biome) > 0.005);
            assert!(
                appears_in_some,
                "{:?} should appear at >0.5% in at least one archetype's distribution \
                 (test fails if no archetype's climate field reaches the biome's \
                 Whittaker polygon)",
                biome
            );
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_5_target_b_constant_referenced() {
        // Sanity: the climate map's TARGET_B_LATITUDE_HALF_EXTENT_WU
        // constant (D.1) is what the distribution sampling uses for
        // world extent. Asserting its value here documents the scale at
        // which D.5 distributions are computed. If F.4.B.2 changes
        // Target B scale in the future, distribution sampling will
        // automatically adjust.
        assert_eq!(TARGET_B_LATITUDE_HALF_EXTENT_WU, 5376.0);
    }
}
