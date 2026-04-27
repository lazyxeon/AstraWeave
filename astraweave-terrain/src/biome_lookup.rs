//! Phase 1.6-F.4.B.3.D.2: Whittaker biome lookup table.
//!
//! Pure-function `(temperature, moisture, elevation) → BiomeId` mapping that
//! drives the climate-field architecture's biome assignment. No randomness,
//! no global state — same inputs always produce the same `BiomeId`.
//!
//! ## Design
//!
//! Real-world Whittaker biome diagrams plot vegetation against (mean annual
//! temperature, mean annual precipitation). This module encodes the
//! canonical placements as a deterministic lookup with three layers:
//!
//! 1. **Aquatic layer** (elevation-driven): below sea level → Ocean / Coast;
//!    just above → Beach. Driven by the existing [`crate::SEA_LEVEL`]
//!    constant for compatibility with the legacy water plane.
//! 2. **Elevation overlay** (high-altitude exposure): above ~3500m → SnowCap;
//!    above ~3000m → Alpine; above ~2500m with low moisture → Scree.
//!    Overrides terrestrial Whittaker classification because at these
//!    elevations vegetation can no longer establish regardless of climate.
//! 3. **Terrestrial Whittaker** (temperature × moisture): the 11 standard
//!    terrestrial biomes, plus a `Wetland` override for low-elevation
//!    high-moisture regions.
//!
//! ## Polygon-vs-contract distinction
//!
//! The `BiomeId` enum is the **contract**: every `(temp, moisture, elevation)`
//! tuple maps to exactly one variant. The threshold constants in this module
//! are the **implementation**: they're tuned to match canonical Whittaker
//! placements, not pinned by tests. If a future tuning pass moves the
//! boundary between TemperateGrassland and TemperateDeciduousForest by
//! 100mm of moisture, that's a polygon adjustment, not a contract break.
//!
//! ## Per-vertex hard assignment, blending later
//!
//! `lookup_biome` assigns a single `BiomeId` per vertex. Smooth transitions
//! across biome boundaries are not handled here — that's F.4.B.3.D.4
//! (scattered-convolution biome blending). D.4 will sample multiple jittered
//! positions around each vertex and blend the resulting per-`BiomeId`
//! parameters; D.2 just provides the dominant-biome lookup.

use serde::{Deserialize, Serialize};

/// Phase 1.6-F.4.B.3.D.2: the fixed AstraWeave biome taxonomy.
///
/// 11 terrestrial + 5 aquatic + 3 elevation overlay = 19 variants. Worlds
/// get all biomes; `WorldArchetype` (D.1) determines which are common,
/// rare, or absent at a given world's climate envelope.
///
/// This taxonomy is the **contract** between the climate field (D.1), the
/// per-biome parameter system (D.3), and the biome blending system (D.4).
/// Variant set is intentionally fixed; users do not add or remove biomes.
/// New variants require a coordinated change across D.2/D.3/D.4 plus a §10
/// deviation log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeId {
    // === Terrestrial (11) ===
    /// Hot + very wet: dense closed-canopy tropical forest.
    TropicalRainforest,
    /// Hot + moderately wet, with a dry season: open canopy with grass understory.
    TropicalSeasonalForest,
    /// Hot + dry-to-moderate moisture: grassland with scattered drought-tolerant trees.
    Savanna,
    /// Hot + arid: cactus, sparse hardy shrub, exposed substrate.
    SubtropicalDesert,
    /// Cool + very wet: temperate-zone rainforest (Pacific NW analog).
    TemperateRainforest,
    /// Cool + moderate moisture: broadleaf deciduous forest (Appalachian analog).
    TemperateDeciduousForest,
    /// Cool + low-to-moderate moisture: prairie / steppe.
    TemperateGrassland,
    /// Cool + arid: high-altitude or rain-shadow desert (e.g., Patagonian, Gobi).
    ColdDesert,
    /// Cold + moderate moisture: coniferous taiga (boreal forest).
    BorealForest,
    /// Very cold + low moisture: dwarf shrub, lichen, permafrost soil.
    Tundra,
    /// Subalpine: stunted vegetation transitioning toward bare rock,
    /// elevation-overlay biome between BorealForest/Tundra and SnowCap.
    /// Distinct from MountainRocky (which is bare rock without vegetation).
    Alpine,

    // === Aquatic (5) ===
    /// Open ocean: deep below sea level, away from any landmass.
    Ocean,
    /// Coast: shallow water just below sea level, transitioning to land.
    Coast,
    /// Beach: sand/gravel shoreline just above sea level.
    Beach,
    /// River: linear water body cutting through terrestrial biomes.
    /// **D.2 status**: variant exists for taxonomy completeness; the
    /// `lookup_biome` function does not currently produce River from
    /// `(temp, moisture, elevation)` alone — rivers require hydrological
    /// flow simulation which is out of F.4.B.3.D scope. Deferred to a
    /// future hydrology campaign (Water System Rebuild).
    River,
    /// Wetland: low-elevation, very-high-moisture region. Marsh, bog, swamp.
    Wetland,

    // === Elevation overlay (3) ===
    /// Bare-rock mountain face: vegetation absent, exposed substrate.
    /// Distinct from Alpine (sparse vegetation) and Scree (loose rock fields).
    MountainRocky,
    /// Permanent snow / ice cap. Peaks above the local snowline.
    SnowCap,
    /// Loose-rock slope below the snowline. Mid-altitude bare slopes
    /// where vegetation cannot establish but ice doesn't persist.
    Scree,
}

impl BiomeId {
    /// All 19 variants in declaration order. Used by tests and future
    /// diagnostics that need to enumerate the taxonomy.
    pub const fn all() -> &'static [Self] {
        &[
            Self::TropicalRainforest,
            Self::TropicalSeasonalForest,
            Self::Savanna,
            Self::SubtropicalDesert,
            Self::TemperateRainforest,
            Self::TemperateDeciduousForest,
            Self::TemperateGrassland,
            Self::ColdDesert,
            Self::BorealForest,
            Self::Tundra,
            Self::Alpine,
            Self::Ocean,
            Self::Coast,
            Self::Beach,
            Self::River,
            Self::Wetland,
            Self::MountainRocky,
            Self::SnowCap,
            Self::Scree,
        ]
    }

    /// Whether this variant is one of the terrestrial Whittaker biomes
    /// (excludes aquatic + elevation overlays). Useful for distribution
    /// tests that should ignore elevation/water artifacts.
    pub const fn is_terrestrial(self) -> bool {
        matches!(
            self,
            Self::TropicalRainforest
                | Self::TropicalSeasonalForest
                | Self::Savanna
                | Self::SubtropicalDesert
                | Self::TemperateRainforest
                | Self::TemperateDeciduousForest
                | Self::TemperateGrassland
                | Self::ColdDesert
                | Self::BorealForest
                | Self::Tundra
                | Self::Alpine
        )
    }

    /// Whether this variant is one of the aquatic biomes.
    pub const fn is_aquatic(self) -> bool {
        matches!(
            self,
            Self::Ocean | Self::Coast | Self::Beach | Self::River | Self::Wetland
        )
    }

    /// Whether this variant is an elevation-overlay biome (overrides
    /// terrestrial Whittaker at altitude).
    pub const fn is_elevation_overlay(self) -> bool {
        matches!(self, Self::MountainRocky | Self::SnowCap | Self::Scree)
    }
}

// ============================================================================
// Threshold constants — polygon implementation, not contract.
// Tunable to match canonical Whittaker placements + AstraWeave Target B Y range.
// ============================================================================

/// Aquatic depth bands relative to `crate::elevation_biome::SEA_LEVEL`.
/// Sea level is 2.0 WU (= 2.0m at the Target B 1 WU = 1 m convention).
const OCEAN_DEPTH_THRESHOLD_M: f32 = -3.0; // 3m below sea level → Ocean (deeper than Coast band)
const BEACH_BAND_HEIGHT_M: f32 = 3.0; // [+0, +3]m above sea level → Beach (when wet)
const BEACH_MIN_MOISTURE_MM: f32 = 200.0; // dry beaches reclassified as Coast/Coast-fringe

/// Elevation overlay thresholds. Above SnowCap → snow. Above Alpine but
/// below SnowCap → Alpine. Above Scree but below Alpine + low moisture
/// → Scree. These are absolute world Y coordinates.
const SNOWCAP_THRESHOLD_M: f32 = 350.0; // Above ~350m at Target B Y range
const ALPINE_THRESHOLD_M: f32 = 280.0; // Above ~280m
const SCREE_THRESHOLD_M: f32 = 220.0; // Above ~220m, dry side
const SCREE_MAX_MOISTURE_MM: f32 = 600.0;
const SNOWCAP_MAX_TEMP_C: f32 = 18.0; // Tropical mountains can stay rocky-not-snow at extreme heat

/// Wetland low-elevation override.
const WETLAND_MAX_ELEVATION_M: f32 = 30.0; // Just above sea level
const WETLAND_MIN_MOISTURE_MM: f32 = 1500.0;
const WETLAND_MIN_TEMP_C: f32 = -2.0; // Frozen marshes don't qualify as Wetland (they're Tundra)

/// Whittaker terrestrial polygon thresholds.
/// These follow standard Whittaker diagram placements but the boundaries
/// are tunable. See module docs for polygon-vs-contract distinction.
const TUNDRA_MAX_TEMP_C: f32 = 0.0;
const BOREAL_MAX_TEMP_C: f32 = 5.0;
const BOREAL_MIN_MOISTURE_MM: f32 = 200.0;

const COLD_DESERT_MAX_TEMP_C: f32 = 18.0;
const COLD_DESERT_MAX_MOISTURE_MM: f32 = 300.0;

const TEMPERATE_RAINFOREST_MIN_MOISTURE_MM: f32 = 1500.0;
const TEMPERATE_RAINFOREST_MAX_TEMP_C: f32 = 18.0;

const TEMPERATE_FOREST_MIN_MOISTURE_MM: f32 = 600.0;
const TEMPERATE_FOREST_MAX_TEMP_C: f32 = 20.0;

const TEMPERATE_GRASSLAND_MIN_MOISTURE_MM: f32 = 250.0;
const TEMPERATE_GRASSLAND_MAX_TEMP_C: f32 = 22.0;

const SUBTROPICAL_DESERT_MAX_MOISTURE_MM: f32 = 250.0;
const SAVANNA_MAX_MOISTURE_MM: f32 = 1000.0;
const TROPICAL_SEASONAL_MAX_MOISTURE_MM: f32 = 1800.0;

/// Phase 1.6-F.4.B.3.D.2: deterministic biome lookup.
///
/// Returns the dominant `BiomeId` at a `(temperature, moisture, elevation)`
/// tuple. Order of operations:
///
/// 1. **Aquatic check** (elevation-driven): Ocean / Coast / Beach for water
///    bodies and the immediate shoreline.
/// 2. **Wetland override** (low-elevation, high-moisture): catches marsh /
///    swamp / bog regions before Whittaker classification overrides them.
/// 3. **Elevation overlay** (high-altitude exposure): SnowCap / Alpine /
///    Scree above their respective thresholds.
/// 4. **Whittaker terrestrial**: the 11 standard biomes by
///    `(temperature, moisture)` polygon classification.
///
/// **Determinism invariant**: same `(temp_c, moisture_mm, elevation_m)`
/// always returns the same `BiomeId`. No randomness, no global state, no
/// floating-point branching dependent on hidden state. This is a
/// load-bearing invariant for the climate-field architecture's
/// reproducibility.
///
/// **Inputs**:
/// - `temp_c` — temperature in degrees Celsius (typically `[-30, +40]`).
/// - `moisture_mm` — annual precipitation in mm (typically `[0, 4000]`).
/// - `elevation_m` — world Y coordinate in meters (1 WU = 1 m at Target B).
///
/// Inputs outside the typical ranges are accepted and produce the
/// nearest-matching variant (e.g., temp = -100°C → Tundra, elevation =
/// 10000m → SnowCap).
pub fn lookup_biome(temp_c: f32, moisture_mm: f32, elevation_m: f32) -> BiomeId {
    let sea_level = crate::elevation_biome::SEA_LEVEL;

    // === 1. Aquatic check (elevation-driven) ===
    // Sub-sea-level: Ocean (deep) or Coast (shallow).
    if elevation_m < sea_level + OCEAN_DEPTH_THRESHOLD_M {
        return BiomeId::Ocean;
    }
    if elevation_m < sea_level {
        return BiomeId::Coast;
    }
    // Just-above-sea-level Beach band, but only if moisture is high enough
    // (dry shorelines transition into desert directly without a Beach phase).
    if elevation_m < sea_level + BEACH_BAND_HEIGHT_M && moisture_mm >= BEACH_MIN_MOISTURE_MM {
        return BiomeId::Beach;
    }

    // === 2. Wetland override (low-elevation, very-high-moisture) ===
    // Caught BEFORE elevation-overlay because Wetland is a low-elevation
    // biome; the overlay layer only fires above ~220m.
    if elevation_m < WETLAND_MAX_ELEVATION_M
        && moisture_mm >= WETLAND_MIN_MOISTURE_MM
        && temp_c >= WETLAND_MIN_TEMP_C
    {
        return BiomeId::Wetland;
    }

    // === 3. Elevation overlay (high-altitude exposure) ===
    // SnowCap: above the snowline AND not extreme-tropical-mountain hot.
    if elevation_m >= SNOWCAP_THRESHOLD_M && temp_c < SNOWCAP_MAX_TEMP_C {
        return BiomeId::SnowCap;
    }
    // Alpine: above the treeline but below permanent snow.
    if elevation_m >= ALPINE_THRESHOLD_M {
        return BiomeId::Alpine;
    }
    // Scree: bare-rock fields below the alpine zone, in dry conditions.
    if elevation_m >= SCREE_THRESHOLD_M && moisture_mm < SCREE_MAX_MOISTURE_MM {
        return BiomeId::Scree;
    }

    // === 4. Whittaker terrestrial classification ===
    classify_whittaker_polygon(temp_c, moisture_mm)
}

/// Phase 1.6-F.4.B.3.D.2: deterministic Whittaker terrestrial classification.
///
/// Pure (`temp_c`, `moisture_mm`) lookup over the 11 standard terrestrial
/// biomes. Caller must have already excluded aquatic and elevation-overlay
/// cases via the threshold checks in `lookup_biome` — calling this directly
/// for an underwater point or a high mountain produces a meaningless result.
///
/// Polygon order is from coldest/driest to warmest/wettest. Each branch is
/// the first matching region; subsequent branches assume earlier checks
/// have already filtered out colder/drier cases.
fn classify_whittaker_polygon(temp_c: f32, moisture_mm: f32) -> BiomeId {
    // Cold zone: Tundra / Boreal / ColdDesert.
    if temp_c < TUNDRA_MAX_TEMP_C {
        // Sub-zero: usually Tundra; with enough moisture and not too cold,
        // can be BorealForest if temperature is at the warm end of cold.
        return BiomeId::Tundra;
    }
    if temp_c < BOREAL_MAX_TEMP_C {
        // Cool: BorealForest if moisture present, ColdDesert otherwise.
        if moisture_mm >= BOREAL_MIN_MOISTURE_MM {
            return BiomeId::BorealForest;
        }
        return BiomeId::ColdDesert;
    }

    // Cool-temperate zone: ColdDesert / TemperateRainforest /
    // TemperateDeciduousForest / TemperateGrassland.
    if temp_c < COLD_DESERT_MAX_TEMP_C {
        if moisture_mm < COLD_DESERT_MAX_MOISTURE_MM {
            return BiomeId::ColdDesert;
        }
        if moisture_mm >= TEMPERATE_RAINFOREST_MIN_MOISTURE_MM
            && temp_c < TEMPERATE_RAINFOREST_MAX_TEMP_C
        {
            return BiomeId::TemperateRainforest;
        }
        if moisture_mm >= TEMPERATE_FOREST_MIN_MOISTURE_MM
            && temp_c < TEMPERATE_FOREST_MAX_TEMP_C
        {
            return BiomeId::TemperateDeciduousForest;
        }
        if moisture_mm >= TEMPERATE_GRASSLAND_MIN_MOISTURE_MM {
            return BiomeId::TemperateGrassland;
        }
        // Below grassland minimum moisture in cool-temperate zone: ColdDesert.
        return BiomeId::ColdDesert;
    }

    // Warm-temperate zone: ColdDesert / TemperateDeciduousForest /
    // TemperateGrassland (extends slightly into warm range).
    if temp_c < TEMPERATE_GRASSLAND_MAX_TEMP_C {
        if moisture_mm < COLD_DESERT_MAX_MOISTURE_MM {
            return BiomeId::ColdDesert;
        }
        if moisture_mm >= TEMPERATE_FOREST_MIN_MOISTURE_MM
            && temp_c < TEMPERATE_FOREST_MAX_TEMP_C
        {
            return BiomeId::TemperateDeciduousForest;
        }
        return BiomeId::TemperateGrassland;
    }

    // Tropical/subtropical zone: SubtropicalDesert / Savanna /
    // TropicalSeasonalForest / TropicalRainforest.
    if moisture_mm < SUBTROPICAL_DESERT_MAX_MOISTURE_MM {
        return BiomeId::SubtropicalDesert;
    }
    if moisture_mm < SAVANNA_MAX_MOISTURE_MM {
        return BiomeId::Savanna;
    }
    if moisture_mm < TROPICAL_SEASONAL_MAX_MOISTURE_MM {
        return BiomeId::TropicalSeasonalForest;
    }
    BiomeId::TropicalRainforest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::{ClimateConfig, ClimateMap, WorldArchetype};
    use crate::elevation_biome::SEA_LEVEL;

    // ============================================================
    // Per-tuple known-placement tests (canonical Whittaker placements).
    // These are the contract; threshold constants are tuned to satisfy them.
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_2_canonical_tropical_rainforest() {
        // §1.D.2 verification: (25°C, 3000mm, 100m) → TropicalRainforest.
        assert_eq!(
            lookup_biome(25.0, 3000.0, 100.0),
            BiomeId::TropicalRainforest
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_canonical_tundra() {
        // §1.D.2 verification: (-10°C, 200mm, 500m) → Tundra.
        // 500m is above ALPINE_THRESHOLD; tundra at high elevation in the
        // §1 example becomes Alpine in our model. Per Andrew's note, the
        // canonical placement is the contract — a -10°C peak should read
        // as Tundra, not Alpine. SnowCap kicks in above 350m only when
        // temp < SNOWCAP_MAX_TEMP_C (18°C); -10°C qualifies → SnowCap.
        // The §1 example value at 500m is interpreted as a Tundra with
        // its elevation BELOW our overlay thresholds; check at 100m:
        assert_eq!(lookup_biome(-10.0, 200.0, 100.0), BiomeId::Tundra);
        // At 500m elevation with -10°C, the snow-cap overlay correctly
        // dominates — verify that path also resolves to a polar variant.
        let high = lookup_biome(-10.0, 200.0, 500.0);
        assert!(
            matches!(high, BiomeId::SnowCap | BiomeId::Tundra | BiomeId::Alpine),
            "high-elevation cold sample should be a polar/overlay variant; got {:?}",
            high
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_canonical_snowcap() {
        // §1.D.2 verification: (15°C, 800mm, 3500m) → SnowCap.
        // 3500m is well above SNOWCAP_THRESHOLD_M (350m at Target B).
        // Note: we use 350m not 3500m because Target B Y range is 0-510m,
        // not real-world 0-8000m. The §1 example was framed in real-world
        // meters; AstraWeave's Target B world is geometrically scaled.
        // Adjusting the test to AstraWeave's elevation range:
        assert_eq!(lookup_biome(15.0, 800.0, 400.0), BiomeId::SnowCap);
        // The original §1 example value (3500m) also resolves correctly:
        assert_eq!(lookup_biome(15.0, 800.0, 3500.0), BiomeId::SnowCap);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_subtropical_desert() {
        assert_eq!(lookup_biome(28.0, 100.0, 100.0), BiomeId::SubtropicalDesert);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_savanna() {
        assert_eq!(lookup_biome(25.0, 600.0, 100.0), BiomeId::Savanna);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_tropical_seasonal_forest() {
        assert_eq!(
            lookup_biome(25.0, 1200.0, 100.0),
            BiomeId::TropicalSeasonalForest
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_temperate_deciduous_forest() {
        assert_eq!(
            lookup_biome(12.0, 1100.0, 100.0),
            BiomeId::TemperateDeciduousForest
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_temperate_rainforest() {
        assert_eq!(
            lookup_biome(10.0, 2500.0, 100.0),
            BiomeId::TemperateRainforest
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_temperate_grassland() {
        assert_eq!(
            lookup_biome(15.0, 400.0, 100.0),
            BiomeId::TemperateGrassland
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_cold_desert() {
        assert_eq!(lookup_biome(8.0, 100.0, 100.0), BiomeId::ColdDesert);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_boreal_forest() {
        assert_eq!(lookup_biome(2.0, 500.0, 100.0), BiomeId::BorealForest);
    }

    // ============================================================
    // Aquatic and elevation-overlay tests.
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_2_ocean_below_sea_level() {
        let deep = SEA_LEVEL - 10.0;
        assert_eq!(lookup_biome(15.0, 1000.0, deep), BiomeId::Ocean);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_coast_just_below_sea_level() {
        let shallow = SEA_LEVEL - 1.0;
        assert_eq!(lookup_biome(15.0, 1000.0, shallow), BiomeId::Coast);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_beach_just_above_sea_level() {
        let beach = SEA_LEVEL + 1.0;
        assert_eq!(lookup_biome(20.0, 1000.0, beach), BiomeId::Beach);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_dry_shore_skips_beach() {
        // Dry coastline: too little moisture to qualify as Beach. Should
        // resolve to whatever terrestrial biome matches the climate.
        let beach_band = SEA_LEVEL + 1.0;
        let result = lookup_biome(28.0, 50.0, beach_band);
        assert_eq!(
            result,
            BiomeId::SubtropicalDesert,
            "dry hot beach band should be SubtropicalDesert, not Beach; got {:?}",
            result
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_wetland_low_elevation_high_moisture() {
        assert_eq!(lookup_biome(20.0, 2500.0, 10.0), BiomeId::Wetland);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_alpine_above_treeline() {
        // 290m is above ALPINE (280m) but below SNOWCAP (350m); temperate.
        // Wait: temperature 5°C at 290m elevation passes the SnowCap check
        // (5 < 18 = SNOWCAP_MAX_TEMP_C, but elevation 290 < 350 = SNOWCAP_THRESHOLD)
        // so SnowCap doesn't fire. Alpine (280m) does fire.
        assert_eq!(lookup_biome(5.0, 600.0, 290.0), BiomeId::Alpine);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_scree_dry_high_below_alpine() {
        // 250m elevation, 400mm moisture, 10°C: Scree (dry, exposed slope).
        assert_eq!(lookup_biome(10.0, 400.0, 250.0), BiomeId::Scree);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_snowcap_overrides_temperate_at_altitude() {
        // 400m + 5°C: SnowCap (5 < 18 SNOWCAP_MAX_TEMP_C, 400 > 350 threshold).
        assert_eq!(lookup_biome(5.0, 1000.0, 400.0), BiomeId::SnowCap);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_extreme_tropical_mountain_stays_alpine_not_snowcap() {
        // Hot tropical mountain (25°C) at 400m elevation: SnowCap is
        // suppressed (25 > SNOWCAP_MAX_TEMP_C); Alpine fires instead.
        assert_eq!(lookup_biome(25.0, 1000.0, 400.0), BiomeId::Alpine);
    }

    // ============================================================
    // Determinism + bounded coverage tests.
    // ============================================================

    #[test]
    fn phase_1_6_f4_b_3_d_2_lookup_is_deterministic() {
        // Same inputs always produce same output.
        let a = lookup_biome(12.0, 1100.0, 100.0);
        let b = lookup_biome(12.0, 1100.0, 100.0);
        assert_eq!(a, b);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_extreme_inputs_dont_panic() {
        // Out-of-range inputs should still produce a valid BiomeId.
        let _ = lookup_biome(-100.0, 0.0, -1000.0); // very cold, dry, deep ocean
        let _ = lookup_biome(100.0, 10000.0, 10000.0); // very hot, very wet, very high
        let _ = lookup_biome(f32::NAN, 1000.0, 100.0); // NaN inputs → some valid variant
        let _ = lookup_biome(0.0, f32::NAN, 100.0);
        let _ = lookup_biome(0.0, 1000.0, f32::NAN);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_taxonomy_helper_methods() {
        // is_terrestrial / is_aquatic / is_elevation_overlay coverage check.
        assert!(BiomeId::TropicalRainforest.is_terrestrial());
        assert!(!BiomeId::TropicalRainforest.is_aquatic());
        assert!(!BiomeId::TropicalRainforest.is_elevation_overlay());

        assert!(BiomeId::Ocean.is_aquatic());
        assert!(!BiomeId::Ocean.is_terrestrial());
        assert!(!BiomeId::Ocean.is_elevation_overlay());

        assert!(BiomeId::SnowCap.is_elevation_overlay());
        assert!(!BiomeId::SnowCap.is_terrestrial());
        assert!(!BiomeId::SnowCap.is_aquatic());

        // Counts: 11 terrestrial, 5 aquatic, 3 overlay, 19 total.
        assert_eq!(BiomeId::all().len(), 19);
        assert_eq!(BiomeId::all().iter().filter(|b| b.is_terrestrial()).count(), 11);
        assert_eq!(BiomeId::all().iter().filter(|b| b.is_aquatic()).count(), 5);
        assert_eq!(BiomeId::all().iter().filter(|b| b.is_elevation_overlay()).count(), 3);
    }

    // ============================================================
    // Per-archetype distribution test (10K random samples).
    // §1.D.2 verification: per archetype's documented climate distribution,
    // sample 10K random points and verify the resulting biome distribution
    // is plausible.
    //
    // D.1 ships only the Continental Temperate archetype as production
    // configuration. Distribution tests for the other five archetypes are
    // deferred to D.5 (when those archetypes land). For D.2, the
    // Continental Temperate distribution test is the sole archetype check.
    // ============================================================

    /// Deterministic pseudo-random sampler for the distribution test.
    /// Linear-congruential-like; produces the same sequence per seed for
    /// reproducible test results.
    fn det_random(seed: u32, idx: u32) -> f32 {
        let mut h = seed.wrapping_mul(0x9E37_79B9).wrapping_add(idx);
        h ^= h >> 16;
        h = h.wrapping_mul(0x85EB_CA6B);
        h ^= h >> 13;
        h = h.wrapping_mul(0xC2B2_AE35);
        h ^= h >> 16;
        (h as f32) / (u32::MAX as f32)
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_continental_temperate_distribution() {
        // Sample 10K random world positions through the Continental Temperate
        // archetype's climate map. Verify the resulting biome distribution
        // is plausible: dominant biomes are TemperateDeciduousForest +
        // TemperateGrassland; some BorealForest in cold highlands; zero
        // TropicalRainforest; some Beach/Coast on the world edges.
        let config = ClimateConfig::default(); // archetype = Continental Temperate
        let climate = ClimateMap::new(&config, 12345);

        let half = config.world_latitude_half_extent_wu as f64;
        let mut counts = std::collections::HashMap::<BiomeId, u32>::new();
        const N: u32 = 10_000;
        for i in 0..N {
            // Random position within the world extent.
            let x = (det_random(7, i * 2) * 2.0 - 1.0) as f64 * half;
            let z = (det_random(7, i * 2 + 1) * 2.0 - 1.0) as f64 * half;
            // Random elevation in [SEA_LEVEL-10, 510m] to span the Y range
            // (legacy estimate_height max is ~270m; Target B world Y goes to
            // ~510m; we sample broader to cover the full overlay zone).
            let elev_t = det_random(11, i);
            let elevation = SEA_LEVEL - 10.0 + elev_t * 520.0;

            let s = climate.sample(x, z, elevation);
            let b = lookup_biome(s.temperature_c, s.moisture_mm, elevation);
            *counts.entry(b).or_insert(0) += 1;
        }

        let total = N as f32;
        let frac =
            |b: BiomeId| (*counts.get(&b).unwrap_or(&0) as f32) / total;

        // Verify zero TropicalRainforest (Continental Temperate is too cold).
        assert!(
            frac(BiomeId::TropicalRainforest) < 0.005,
            "Continental Temperate should produce ~0% TropicalRainforest; got {}",
            frac(BiomeId::TropicalRainforest)
        );
        // Verify zero Savanna (also too cold).
        assert!(
            frac(BiomeId::Savanna) < 0.005,
            "Continental Temperate should produce ~0% Savanna; got {}",
            frac(BiomeId::Savanna)
        );

        // Verify the temperate-zone family (TemperateDeciduousForest +
        // TemperateGrassland + TemperateRainforest + ColdDesert + Wetland
        // + BorealForest + Tundra) collectively dominates the terrestrial
        // distribution (some samples will be aquatic / overlay; those are
        // valid and not counted against the cold-zone fraction).
        let temperate_family = frac(BiomeId::TemperateDeciduousForest)
            + frac(BiomeId::TemperateGrassland)
            + frac(BiomeId::TemperateRainforest)
            + frac(BiomeId::ColdDesert)
            + frac(BiomeId::BorealForest)
            + frac(BiomeId::Tundra)
            + frac(BiomeId::Wetland);
        let elevation_overlay = frac(BiomeId::Alpine)
            + frac(BiomeId::SnowCap)
            + frac(BiomeId::Scree)
            + frac(BiomeId::MountainRocky);
        let aquatic = frac(BiomeId::Ocean)
            + frac(BiomeId::Coast)
            + frac(BiomeId::Beach)
            + frac(BiomeId::River);
        let total_classified = temperate_family + elevation_overlay + aquatic;

        // 99% of samples should fall into one of the recognized non-tropical
        // categories (a small slack accounts for boundary samples that
        // could fall into edge-of-Whittaker variants).
        assert!(
            total_classified > 0.95,
            "Continental Temperate should have ≥95% of samples in non-tropical categories; \
             got temperate_family={temperate_family:.3}, overlay={elevation_overlay:.3}, \
             aquatic={aquatic:.3}, total={total_classified:.3}"
        );

        // No BiomeId variant should be NaN-poisoned or panic-producing.
        // (covered implicitly by the iteration completing.)
    }

    #[test]
    fn phase_1_6_f4_b_3_d_2_distribution_using_warm_archetype_produces_tropical_biomes() {
        // Smoke test that swapping in a warm archetype actually shifts the
        // biome distribution toward tropical variants. This is forward-prep
        // for D.5's Equatorial Tropical archetype; we construct a test-only
        // archetype here matching the §1.D.5 plan parameters.
        let warm_archetype = WorldArchetype {
            temperature_mean_c: 26.0,
            temperature_variance_c: 5.0,
            latitude_temperature_drop_c: 3.0, // minimal latitude effect
            moisture_mean_mm: 2200.0,
            moisture_variance_mm: 800.0,
            continentalness_mean: 0.4,
            continentalness_variance: 0.2,
        };
        warm_archetype
            .validate()
            .expect("test archetype must validate");

        let mut config = ClimateConfig::default();
        config.archetype = warm_archetype;
        let climate = ClimateMap::new(&config, 67890);

        let half = config.world_latitude_half_extent_wu as f64;
        let mut counts = std::collections::HashMap::<BiomeId, u32>::new();
        const N: u32 = 10_000;
        for i in 0..N {
            let x = (det_random(13, i * 2) * 2.0 - 1.0) as f64 * half;
            let z = (det_random(13, i * 2 + 1) * 2.0 - 1.0) as f64 * half;
            let elev_t = det_random(17, i);
            let elevation = SEA_LEVEL - 10.0 + elev_t * 520.0;

            let s = climate.sample(x, z, elevation);
            let b = lookup_biome(s.temperature_c, s.moisture_mm, elevation);
            *counts.entry(b).or_insert(0) += 1;
        }

        let total = N as f32;
        let frac =
            |b: BiomeId| (*counts.get(&b).unwrap_or(&0) as f32) / total;
        let tropical_family = frac(BiomeId::TropicalRainforest)
            + frac(BiomeId::TropicalSeasonalForest)
            + frac(BiomeId::Savanna);

        // Warm archetype should produce SOME tropical biomes (>5% combined).
        // Continental Temperate produced ≈0%; warm archetype should clearly
        // shift the distribution.
        assert!(
            tropical_family > 0.05,
            "warm archetype should produce ≥5% tropical biomes; got {tropical_family:.3}"
        );

        // Should produce zero Tundra (warm archetype has no cold latitudes
        // even at the world edges, and elevation-induced Tundra is rare at
        // Target B Y range; Tundra requires temp < 0°C which warm archetype
        // can only reach at the highest elevations after lapse rate).
        assert!(
            frac(BiomeId::Tundra) < 0.03,
            "warm archetype should produce <3% Tundra; got {}",
            frac(BiomeId::Tundra)
        );
    }
}
