//! Phase 1.5 (Terrain Material System Campaign) — heightmap-driven
//! multi-biome generation.
//!
//! Maps vertex world-space elevation to normalized 8-slot biome weights
//! that plug into the Phase 1 forward-lit splat pipeline. The
//! `terrain_primary_biome` field becomes a climate bias parameter: the
//! same heightmap produces different biome distributions under different
//! climates.
//!
//! Slot order matches the editor's `TerrainVertex.biome_weights_0/1` packing
//! and the `BiomeType` ordering in this crate:
//!
//! | Slot | Biome      |
//! |------|------------|
//! | 0    | Grassland  |
//! | 1    | Desert     |
//! | 2    | Forest     |
//! | 3    | Mountain   |
//! | 4    | Tundra     |
//! | 5    | Swamp      |
//! | 6    | Beach      |
//! | 7    | River      |
//!
//! Slots 0-3 populate `biome_weights_0`, slots 4-7 populate `biome_weights_1`.

/// Sea level Y in world space. Matches `WaterRenderer`'s hardcoded water
/// plane at Y=2.0 per the water system audit
/// (`docs/audits/water_system_architecture_2026-04-20.md` §3.1). Phase 1.5
/// uses this as the pivot for elevation-to-biome mapping; Beach biome sits
/// just above this value for climates that include a coastal band.
pub const SEA_LEVEL: f32 = 2.0;

/// Climate bias. Selects which elevation→biome mapping is used during
/// chunk generation. The `terrain_primary_biome` string is parsed into a
/// `ClimateBias` via [`ClimateBias::from_primary_biome_str`]; each climate
/// has its own set of biome bands keyed to elevation relative to sea level.
///
/// Replaces the prior interpretation of `terrain_primary_biome` as a
/// single-biome selector.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClimateBias {
    /// Temperate: Beach → Grassland → Forest → Mountain.
    Temperate,
    /// Cold: Beach → Tundra → Mountain (no Forest, no Grassland).
    Cold,
    /// Arid: Beach → Desert → Mountain (no Forest, no Grassland).
    Arid,
    /// Tropical: Beach → Forest (heavy) → Mountain (warm, wet).
    Tropical,
    /// Wetland: Beach → Swamp → Grassland → Forest → Mountain (low focus).
    Wetland,
    /// Highland: Grassland (low) → Mountain (heavy). No Beach.
    Highland,
}

impl ClimateBias {
    /// Parse from the `terrain_primary_biome` string for backward compat.
    /// Maps each biome string to the most appropriate climate bias.
    ///
    /// Unknown, empty, or "grassland"/"beach"/"river" strings default to
    /// [`ClimateBias::Temperate`].
    pub fn from_primary_biome_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tundra" => ClimateBias::Cold,
            "desert" => ClimateBias::Arid,
            "forest" => ClimateBias::Tropical,
            "swamp" => ClimateBias::Wetland,
            "mountain" => ClimateBias::Highland,
            // "grassland" | "beach" | "river" | "" | unknown → Temperate
            _ => ClimateBias::Temperate,
        }
    }

    /// Bands that define this climate's elevation → biome mapping.
    /// Each band contributes to a single slot; weights are summed then
    /// normalized in [`elevation_to_biome_weights`].
    fn bands(self) -> &'static [BiomeBand] {
        match self {
            ClimateBias::Temperate => TEMPERATE_BANDS,
            ClimateBias::Cold => COLD_BANDS,
            ClimateBias::Arid => ARID_BANDS,
            ClimateBias::Tropical => TROPICAL_BANDS,
            ClimateBias::Wetland => WETLAND_BANDS,
            ClimateBias::Highland => HIGHLAND_BANDS,
        }
    }

    /// Slot to use when no band fires (e.g. extreme elevations outside
    /// all configured bands). Keeps the output normalized.
    fn fallback_slot(self) -> usize {
        match self {
            ClimateBias::Cold => 4,     // Tundra
            ClimateBias::Highland => 3, // Mountain
            _ => 6,                      // Beach (sea-level default)
        }
    }
}

/// Shape of a biome band along the elevation axis.
#[derive(Copy, Clone, Debug)]
enum BandShape {
    /// Triangular pulse with smoothstep falloff. Peaks at `peak` with
    /// weight 1.0; decays to zero at `peak ± width`.
    Pulse { peak: f32, width: f32 },
    /// High-pass. Zero at `start`, ramps smoothly to 1.0 at
    /// `start + ramp`, then plateaus. Used for mountain-type biomes
    /// that should dominate at any elevation above a threshold.
    HighPass { start: f32, ramp: f32 },
}

/// A single band contributing to one biome slot.
#[derive(Copy, Clone, Debug)]
struct BiomeBand {
    slot: usize,
    shape: BandShape,
}

// Band tables. All `peak` / `start` values are elevations *relative to
// sea level* (not absolute world Y) so climates remain valid regardless
// of SEA_LEVEL adjustments.

const TEMPERATE_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 6, shape: BandShape::Pulse { peak: 1.0, width: 3.0 } },     // Beach
    BiomeBand { slot: 0, shape: BandShape::Pulse { peak: 8.0, width: 6.0 } },     // Grassland
    BiomeBand { slot: 2, shape: BandShape::Pulse { peak: 22.0, width: 10.0 } },   // Forest
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 30.0, ramp: 30.0 } }, // Mountain
];

const COLD_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 6, shape: BandShape::Pulse { peak: 1.0, width: 3.0 } },     // Beach
    BiomeBand { slot: 4, shape: BandShape::Pulse { peak: 10.0, width: 25.0 } },   // Tundra
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 30.0, ramp: 30.0 } }, // Mountain
];

const ARID_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 6, shape: BandShape::Pulse { peak: 1.0, width: 3.0 } },     // Beach
    BiomeBand { slot: 1, shape: BandShape::Pulse { peak: 10.0, width: 25.0 } },   // Desert
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 30.0, ramp: 30.0 } }, // Mountain
];

const TROPICAL_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 6, shape: BandShape::Pulse { peak: 1.0, width: 3.0 } },     // Beach
    BiomeBand { slot: 2, shape: BandShape::Pulse { peak: 15.0, width: 30.0 } },   // Forest (heavy)
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 50.0, ramp: 30.0 } }, // Mountain
];

const WETLAND_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 6, shape: BandShape::Pulse { peak: 1.0, width: 3.0 } },     // Beach
    BiomeBand { slot: 5, shape: BandShape::Pulse { peak: 5.0, width: 10.0 } },    // Swamp (lowland)
    BiomeBand { slot: 0, shape: BandShape::Pulse { peak: 15.0, width: 10.0 } },   // Grassland
    BiomeBand { slot: 2, shape: BandShape::Pulse { peak: 35.0, width: 20.0 } },   // Forest
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 50.0, ramp: 30.0 } }, // Mountain
];

const HIGHLAND_BANDS: &[BiomeBand] = &[
    BiomeBand { slot: 0, shape: BandShape::Pulse { peak: 3.0, width: 15.0 } },    // Grassland (low)
    BiomeBand { slot: 3, shape: BandShape::HighPass { start: 5.0, ramp: 40.0 } }, // Mountain (dominant)
];

/// Smoothstep function `3t² - 2t³` clamped to `[0, 1]`.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn evaluate_band(rel_y: f32, shape: &BandShape) -> f32 {
    match *shape {
        BandShape::Pulse { peak, width } => {
            if width <= 0.0 {
                return 0.0;
            }
            let d = (rel_y - peak).abs();
            if d >= width {
                0.0
            } else {
                smoothstep(1.0 - d / width)
            }
        }
        BandShape::HighPass { start, ramp } => {
            if ramp <= 0.0 {
                return if rel_y >= start { 1.0 } else { 0.0 };
            }
            if rel_y <= start {
                0.0
            } else if rel_y >= start + ramp {
                1.0
            } else {
                smoothstep((rel_y - start) / ramp)
            }
        }
    }
}

/// World-elevation-to-biome-weights mapping.
///
/// Returns 8 weights summing to ~1.0 in canonical slot order (see module
/// docs). The mapping is climate-bias-dependent: each climate has its own
/// elevation → biome curves with smooth smoothstep-falloff transitions at
/// band boundaries so adjacent vertices produce blendable splat weights,
/// not hard cutoffs.
///
/// - `world_y`: vertex Y in world space.
/// - `sea_level`: Y coordinate of sea level (use [`SEA_LEVEL`] to match
///   the water system's hardcoded plane).
/// - `climate`: which elevation → biome mapping to apply.
///
/// If no configured band fires at the given elevation (e.g. far below sea
/// level or far above any mountain plateau), the climate's fallback slot
/// receives weight 1.0 so the output is always normalized.
pub fn elevation_to_biome_weights(
    world_y: f32,
    sea_level: f32,
    climate: ClimateBias,
) -> [f32; 8] {
    let rel = world_y - sea_level;
    let mut weights = [0.0f32; 8];

    for band in climate.bands() {
        let w = evaluate_band(rel, &band.shape);
        if w > 0.0 {
            weights[band.slot] += w;
        }
    }

    let total: f32 = weights.iter().sum();
    if total > 1e-4 {
        let inv = 1.0 / total;
        for w in &mut weights {
            *w *= inv;
        }
    } else {
        weights[climate.fallback_slot()] = 1.0;
    }

    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-3;

    /// Test 1: sum-to-one invariant. For every climate across a wide
    /// elevation sweep, output weights sum to within 0.001 of 1.0.
    #[test]
    fn weights_sum_to_one_across_elevation_sweep() {
        let climates = [
            ClimateBias::Temperate,
            ClimateBias::Cold,
            ClimateBias::Arid,
            ClimateBias::Tropical,
            ClimateBias::Wetland,
            ClimateBias::Highland,
        ];
        let sea_level = SEA_LEVEL;

        for climate in climates {
            let mut rel = -10.0;
            while rel <= 200.0 {
                let world_y = sea_level + rel;
                let w = elevation_to_biome_weights(world_y, sea_level, climate);
                let sum: f32 = w.iter().sum();
                assert!(
                    (sum - 1.0).abs() < EPS,
                    "climate {:?} at rel {}: sum = {}, weights = {:?}",
                    climate,
                    rel,
                    sum,
                    w
                );
                // Every individual weight is in [0, 1].
                for (i, &v) in w.iter().enumerate() {
                    assert!(
                        (0.0..=1.0 + EPS).contains(&v),
                        "climate {:?} at rel {}: slot {} = {}",
                        climate,
                        rel,
                        i,
                        v
                    );
                }
                rel += 1.0;
            }
        }
    }

    /// Test 2: Beach band exists for non-Highland climates. At
    /// `sea_level + 0.5` the Beach slot (6) dominates for Temperate, Cold,
    /// Arid, Tropical, and Wetland. Highland has no Beach.
    #[test]
    fn beach_dominates_near_sea_level_for_coastal_climates() {
        let sea_level = SEA_LEVEL;
        let y = sea_level + 0.5;

        for climate in [
            ClimateBias::Temperate,
            ClimateBias::Cold,
            ClimateBias::Arid,
            ClimateBias::Tropical,
            ClimateBias::Wetland,
        ] {
            let w = elevation_to_biome_weights(y, sea_level, climate);
            let (dom_slot, _dom_w) = dominant(&w);
            assert_eq!(
                dom_slot, 6,
                "climate {:?}: expected Beach (slot 6) dominant at rel=0.5, got slot {} with weights {:?}",
                climate, dom_slot, w
            );
        }

        // Highland: Beach slot (6) must be zero.
        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Highland);
        assert!(
            w[6].abs() < EPS,
            "Highland must not have Beach weight, got {:?}",
            w
        );
    }

    /// Test 3: Mountain (slot 3) dominates at high elevation
    /// (sea_level + 100.0) for all climates.
    #[test]
    fn mountain_dominates_at_high_elevation() {
        let sea_level = SEA_LEVEL;
        let y = sea_level + 100.0;
        let climates = [
            ClimateBias::Temperate,
            ClimateBias::Cold,
            ClimateBias::Arid,
            ClimateBias::Tropical,
            ClimateBias::Wetland,
            ClimateBias::Highland,
        ];

        for climate in climates {
            let w = elevation_to_biome_weights(y, sea_level, climate);
            let (dom_slot, _) = dominant(&w);
            assert_eq!(
                dom_slot, 3,
                "climate {:?}: expected Mountain (slot 3) dominant at rel=100, got slot {} with weights {:?}",
                climate, dom_slot, w
            );
        }
    }

    /// Test 4: mid-elevation biomes vary by climate. At rel=15 each
    /// climate produces its climate-distinct biome as the dominant slot.
    #[test]
    fn mid_elevation_dominant_biome_varies_by_climate() {
        let sea_level = SEA_LEVEL;
        let y = sea_level + 15.0;

        // Temperate: Grassland (0) or Forest (2).
        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Temperate);
        let (dom, _) = dominant(&w);
        assert!(
            dom == 0 || dom == 2,
            "Temperate mid-elevation dominant = slot {}, expected 0 or 2; weights = {:?}",
            dom,
            w
        );

        // Cold: Tundra (4).
        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Cold);
        let (dom, _) = dominant(&w);
        assert_eq!(dom, 4, "Cold mid-elevation: weights {:?}", w);

        // Arid: Desert (1).
        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Arid);
        let (dom, _) = dominant(&w);
        assert_eq!(dom, 1, "Arid mid-elevation: weights {:?}", w);

        // Tropical: Forest (2).
        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Tropical);
        let (dom, _) = dominant(&w);
        assert_eq!(dom, 2, "Tropical mid-elevation: weights {:?}", w);
    }

    /// Test 5: from_primary_biome_str maps legacy strings to the right
    /// climate bias; unknown/empty/"grassland" → Temperate.
    #[test]
    fn from_primary_biome_str_maps_strings_correctly() {
        assert_eq!(
            ClimateBias::from_primary_biome_str("grassland"),
            ClimateBias::Temperate
        );
        assert_eq!(
            ClimateBias::from_primary_biome_str("GRASSLAND"),
            ClimateBias::Temperate
        );
        assert_eq!(ClimateBias::from_primary_biome_str("tundra"), ClimateBias::Cold);
        assert_eq!(ClimateBias::from_primary_biome_str("desert"), ClimateBias::Arid);
        assert_eq!(
            ClimateBias::from_primary_biome_str("forest"),
            ClimateBias::Tropical
        );
        assert_eq!(
            ClimateBias::from_primary_biome_str("swamp"),
            ClimateBias::Wetland
        );
        assert_eq!(
            ClimateBias::from_primary_biome_str("mountain"),
            ClimateBias::Highland
        );

        // Strings that map to Temperate fallback.
        assert_eq!(ClimateBias::from_primary_biome_str(""), ClimateBias::Temperate);
        assert_eq!(
            ClimateBias::from_primary_biome_str("beach"),
            ClimateBias::Temperate
        );
        assert_eq!(
            ClimateBias::from_primary_biome_str("river"),
            ClimateBias::Temperate
        );
        assert_eq!(
            ClimateBias::from_primary_biome_str("unknown_biome_xyz"),
            ClimateBias::Temperate
        );
    }

    /// Smoothstep helper sanity: at t=0 → 0, t=0.5 → 0.5, t=1 → 1.
    #[test]
    fn smoothstep_endpoints_and_midpoint() {
        assert!((smoothstep(0.0) - 0.0).abs() < EPS);
        assert!((smoothstep(0.5) - 0.5).abs() < EPS);
        assert!((smoothstep(1.0) - 1.0).abs() < EPS);
        // Clamping
        assert!((smoothstep(-1.0) - 0.0).abs() < EPS);
        assert!((smoothstep(2.0) - 1.0).abs() < EPS);
    }

    /// Far-below-sea-level case uses the climate's fallback slot. Output
    /// is still normalized.
    #[test]
    fn below_sea_level_falls_back_cleanly() {
        let sea_level = SEA_LEVEL;
        let y = sea_level - 50.0;

        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Temperate);
        let sum: f32 = w.iter().sum();
        assert!((sum - 1.0).abs() < EPS, "weights: {:?}", w);
        assert_eq!(dominant(&w).0, 6, "Temperate below sea level → Beach fallback: {:?}", w);

        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Cold);
        assert_eq!(dominant(&w).0, 4, "Cold below sea level → Tundra fallback: {:?}", w);

        let w = elevation_to_biome_weights(y, sea_level, ClimateBias::Highland);
        assert_eq!(dominant(&w).0, 3, "Highland below sea level → Mountain fallback: {:?}", w);
    }

    fn dominant(w: &[f32; 8]) -> (usize, f32) {
        let mut best = (0, w[0]);
        for (i, &v) in w.iter().enumerate().skip(1) {
            if v > best.1 {
                best = (i, v);
            }
        }
        best
    }
}
