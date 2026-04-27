//! Phase 1.6-F.4.B.3.D.3: per-biome terrain parameter system.
//!
//! Replaces the legacy `BiomeNoisePreset` (whole-world per-preset
//! configuration) with a per-`BiomeId` parameter table. Each vertex looks
//! up its `BiomeId` from the [`crate::biome_lookup::lookup_biome`] function
//! (D.2), then queries [`BiomeParameters::for_biome`] for the terrain-shape
//! parameters that apply at that vertex.
//!
//! ## Structural change vs F.4.B.3.C and prior
//!
//! Before D.3, the user picked a single "Primary Biome" preset and the
//! engine applied that preset's noise/erosion/scatter parameters to every
//! vertex of the world. After D.3, the user picks a `WorldArchetype`
//! (climate envelope; D.5) which shapes the climate field; the climate
//! field at each vertex resolves to a `BiomeId`; the `BiomeId` resolves
//! to a `BiomeParameters` table entry. The user does not pick parameters
//! directly — biomes emerge from climate.
//!
//! ## Why some fields are stubbed
//!
//! D.3's structural job is the per-vertex parameter lookup. Several
//! fields on `BiomeParameters` are defined here for API completeness but
//! their wiring through downstream subsystems is deferred:
//!
//! - `mountains_amplitude`: WIRED through D.3b's refactor of
//!   `WorldGenerator` + `TerrainNoise::sample_components`. Per-biome
//!   amplitude scaling on the mountain layer.
//! - `ridge_strength`: DEFINED but not yet wired into the noise
//!   pipeline. F.4.B.3.E was demoted to this parameter; ridged-multifractal
//!   integration is a separate noise contribution that requires a follow-up
//!   tuning campaign. The field is set per biome so future wiring sees
//!   correct defaults.
//! - `runevision_config`: DEFINED but not yet wired. F.4.B.3.C's
//!   per-`BiomeNoisePreset.runevision_enabled` flag flowed through a global
//!   `NoiseConfig.runevision: Option<RunevisionConfig>`; the new system
//!   is per-vertex which `TerrainNoise::sample_height` doesn't currently
//!   support. Mountain-character biomes default to `None` per the
//!   F.4.B.3.C REGRESS finding (gradient magnitudes pushed the filter
//!   outside its working range at high amplitudes); this default carries
//!   forward when wiring lands.
//! - `erosion_preset`: DEFINED. The legacy `erosion_preset_for_climate`
//!   maps `ClimateBias` enum values to preset constructors; D.3b will
//!   route per-biome `ErosionPresetId` through to actual erosion
//!   simulation calls during `generate_chunk_with_climate`.
//! - `scatter_density`, `scatter_species_set`, `surface_color_palette`:
//!   DEFINED but consumed by the scatter and rendering subsystems which
//!   are downstream of D.3's terrain-pipeline scope. These are forward-
//!   compatible defaults; D.5+ wires them through.
//!
//! ## Why per-biome defaults are conservative
//!
//! All 19 biomes default `runevision_config: None`. The F.4.B.3.C lesson
//! is that opt-in (per-biome) is the safe default — runevision composes
//! badly with high-amplitude mountain biomes, and bespoke per-biome
//! tuning to find safe parameters is an investigation campaign, not a
//! D.3 task. The deferred per-biome runevision tuning (in §4 of the
//! reframe doc) is the right home for finding biome-specific safe
//! parameters; until then, every biome opts out.
//!
//! The other defaults (mountains_amplitude, ridge_strength) start from
//! mid-range conservative values that produce visually plausible terrain
//! across the full archetype × biome cross-product. D.6's Andrew-gate
//! will identify which defaults need tuning.

use crate::biome_lookup::BiomeId;
use crate::runevision_erosion::RunevisionConfig;
use serde::{Deserialize, Serialize};

/// Phase 1.6-F.4.B.3.D.3: erosion preset identifier per biome.
///
/// Selects one of the existing `advanced_erosion::ErosionPreset`
/// constructors. The actual `ErosionPreset` struct is large and not
/// `Copy`-friendly, so `BiomeParameters` stores an ID and resolves to
/// the full preset only when erosion runs (per-chunk, not per-vertex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErosionPresetId {
    /// Default 25K-droplet preset for moderate-erosion temperate climates.
    DefaultBalanced,
    /// 35K-droplet preset for mountain-character climates needing more
    /// erosion to carve dramatic relief.
    MountainBalanced,
    /// Arid-climate preset.
    Desert,
    /// Wet-climate / coastal preset.
    Coastal,
    /// Legacy 100K-droplet aggressive mountain preset (slow; rarely used).
    Mountain,
}

impl ErosionPresetId {
    /// Resolve the ID to a full `ErosionPreset`. Calls the corresponding
    /// constructor on `advanced_erosion::ErosionPreset`.
    pub fn resolve(self) -> crate::advanced_erosion::ErosionPreset {
        match self {
            Self::DefaultBalanced => crate::advanced_erosion::ErosionPreset::default_balanced(),
            Self::MountainBalanced => crate::advanced_erosion::ErosionPreset::mountain_balanced(),
            Self::Desert => crate::advanced_erosion::ErosionPreset::desert(),
            Self::Coastal => crate::advanced_erosion::ErosionPreset::coastal(),
            Self::Mountain => crate::advanced_erosion::ErosionPreset::mountain(),
        }
    }
}

/// Phase 1.6-F.4.B.3.D.3: scatter species set selector per biome.
///
/// Identifies which scatter assets are valid for this biome. Concrete
/// asset wiring (mapping `ScatterSpeciesSet` to specific .glb files) is
/// deferred to D.5+. For D.3, this is forward-compatible metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScatterSpeciesSet {
    /// No terrestrial scatter (aquatic biomes).
    None,
    /// Grasses, herbs, occasional shrubs.
    Grassland,
    /// Broadleaf trees, ferns, mushrooms.
    Forest,
    /// Conifers, lichens (taiga / boreal).
    Boreal,
    /// Dwarf shrubs, mosses, lichens (cold).
    Tundra,
    /// Succulents, hardy grasses (arid).
    Desert,
    /// Palms, vines, broadleaf trees (warm + wet).
    Tropical,
    /// Sparse trees, drought-tolerant grasses (warm + moderate).
    Savanna,
    /// Reeds, mangroves, mosses (low-elevation wet).
    Wetland,
    /// Dwarf shrubs at altitude (subalpine).
    Alpine,
    /// Sparse lithophytes / no scatter (bare rock + snow).
    BareRock,
}

/// Phase 1.6-F.4.B.3.D.3: surface color palette selector per biome.
///
/// Identifies which palette this biome's splat-texturing uses. Palette
/// content (concrete RGB values, splat textures) is defined elsewhere
/// in the rendering subsystem. For D.3, this is forward-compatible
/// metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceColorPalette {
    /// Open ocean / deep water.
    OceanWater,
    /// Sand / beach.
    Sand,
    /// Lush green grass.
    Grass,
    /// Dry / yellow grass (savanna, dry temperate).
    DryGrass,
    /// Temperate forest floor (mossy, leaf-littered).
    Forest,
    /// Boreal forest floor (needles, lichen).
    Boreal,
    /// Tundra (low vegetation, exposed soil).
    Tundra,
    /// Mud / wetland soil.
    Mud,
    /// Bare rock (alpine + scree + mountain).
    Rock,
    /// Snow / ice (snowcap).
    Snow,
}

/// Phase 1.6-F.4.B.3.D.3: per-biome terrain parameters.
///
/// Looked up per-vertex via [`BiomeParameters::for_biome`] after the
/// climate field + Whittaker lookup determine the dominant `BiomeId`
/// at the vertex. Replaces the legacy `BiomeNoisePreset` whole-world
/// configuration.
///
/// All 19 `BiomeId` variants resolve to a populated struct (no panics,
/// no fallback). See module docs for which fields are wired vs stubbed
/// in D.3.
#[derive(Debug, Clone)]
pub struct BiomeParameters {
    /// Per-biome multiplier on the mountain layer's noise contribution.
    /// `1.0` = baseline (use `NoiseConfig::default()`'s mountain
    /// amplitude as-is); `< 1.0` = damped (rolling biomes); `> 1.0` =
    /// boosted (alpine biomes). Multiplies the mountain layer's
    /// per-vertex contribution AFTER continental modulation.
    ///
    /// Wired through `TerrainNoise::sample_components` + per-vertex
    /// combine in `WorldGenerator` (D.3b).
    pub mountains_amplitude: f64,

    /// Strength of ridged-multifractal noise contribution added to the
    /// base fBm. `0.0` = no ridges (rolling terrain); `1.0` = full
    /// ridge contribution. Absorbs F.4.B.3.E (originally a separate
    /// sub-phase, now a per-biome parameter).
    ///
    /// **D.3 status**: DEFINED but not yet wired. Wiring requires adding
    /// a ridged-multifractal noise source to `TerrainNoise` and routing
    /// per-vertex contribution through `sample_components`. Defer to
    /// follow-up tuning campaign.
    pub ridge_strength: f64,

    /// Per-biome runevision filter configuration. `None` = filter off
    /// for this biome; `Some` = filter on with these parameters.
    /// Mountain-character biomes (Alpine, MountainRocky, SnowCap, Scree)
    /// default to `None` per the F.4.B.3.C REGRESS finding.
    ///
    /// **D.3 status**: DEFINED but not yet wired. F.4.B.3.C's flag
    /// flowed through a global `NoiseConfig.runevision`; per-vertex
    /// wiring requires `TerrainNoise::sample_height` to accept per-call
    /// runevision config. Defer to follow-up tuning campaign that also
    /// finds safe per-biome runevision parameters.
    pub runevision_config: Option<RunevisionConfig>,

    /// Erosion preset identifier. Resolves to a full
    /// `advanced_erosion::ErosionPreset` via [`ErosionPresetId::resolve`].
    ///
    /// **D.3 status**: DEFINED but legacy `erosion_preset_for_climate`
    /// path still drives `WorldGenerator::generate_chunk_with_climate`'s
    /// erosion call. D.3b retains the legacy mapping (it's keyed by
    /// `ClimateBias` not `BiomeId`); future per-biome erosion routing
    /// can use this field.
    pub erosion_preset: ErosionPresetId,

    /// Per-biome vegetation density multiplier. `0.0` = barren; higher =
    /// denser scatter. Multiplies the global scatter density at scatter
    /// time.
    ///
    /// **D.3 status**: DEFINED but consumed by scatter subsystem
    /// downstream of D.3's terrain-pipeline scope.
    pub scatter_density: f64,

    /// Per-biome scatter species set selector.
    ///
    /// **D.3 status**: DEFINED, see field docs above.
    pub scatter_species_set: ScatterSpeciesSet,

    /// Per-biome surface color palette selector.
    ///
    /// **D.3 status**: DEFINED, consumed by splat-texturing in the
    /// rendering subsystem.
    pub surface_color_palette: SurfaceColorPalette,
}

impl BiomeParameters {
    /// Phase 1.6-F.4.B.3.D.3: resolve a `BiomeId` to its default
    /// per-biome parameters. Total over all 19 variants — every input
    /// returns a populated struct, no panics, no fallback.
    ///
    /// Defaults are conservative starting values. Mountain-character
    /// biomes default `runevision_config: None` per the F.4.B.3.C REGRESS
    /// finding. Other biome defaults are mid-range; D.6's Andrew-gate
    /// will identify which need tuning. Future per-biome runevision
    /// tuning campaign can change individual biome defaults to opt in
    /// with calibrated parameters.
    pub fn for_biome(biome: BiomeId) -> Self {
        match biome {
            // === Tropical (warm + wet) ===
            BiomeId::TropicalRainforest => Self {
                mountains_amplitude: 1.4,
                ridge_strength: 0.3,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 1.6,
                scatter_species_set: ScatterSpeciesSet::Tropical,
                surface_color_palette: SurfaceColorPalette::Forest,
            },
            BiomeId::TropicalSeasonalForest => Self {
                mountains_amplitude: 1.2,
                ridge_strength: 0.2,
                runevision_config: None,
                erosion_preset: ErosionPresetId::DefaultBalanced,
                scatter_density: 1.2,
                scatter_species_set: ScatterSpeciesSet::Tropical,
                surface_color_palette: SurfaceColorPalette::Forest,
            },
            BiomeId::Savanna => Self {
                mountains_amplitude: 0.8,
                ridge_strength: 0.1,
                runevision_config: None,
                erosion_preset: ErosionPresetId::DefaultBalanced,
                scatter_density: 0.6,
                scatter_species_set: ScatterSpeciesSet::Savanna,
                surface_color_palette: SurfaceColorPalette::DryGrass,
            },
            BiomeId::SubtropicalDesert => Self {
                mountains_amplitude: 1.0,
                ridge_strength: 0.1,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Desert,
                scatter_density: 0.2,
                scatter_species_set: ScatterSpeciesSet::Desert,
                surface_color_palette: SurfaceColorPalette::Sand,
            },

            // === Temperate (cool + moderate) ===
            BiomeId::TemperateRainforest => Self {
                mountains_amplitude: 1.4,
                ridge_strength: 0.3,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 1.5,
                scatter_species_set: ScatterSpeciesSet::Forest,
                surface_color_palette: SurfaceColorPalette::Forest,
            },
            BiomeId::TemperateDeciduousForest => Self {
                mountains_amplitude: 1.2,
                ridge_strength: 0.2,
                runevision_config: None,
                erosion_preset: ErosionPresetId::DefaultBalanced,
                scatter_density: 1.2,
                scatter_species_set: ScatterSpeciesSet::Forest,
                surface_color_palette: SurfaceColorPalette::Forest,
            },
            BiomeId::TemperateGrassland => Self {
                mountains_amplitude: 0.8,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::DefaultBalanced,
                scatter_density: 0.8,
                scatter_species_set: ScatterSpeciesSet::Grassland,
                surface_color_palette: SurfaceColorPalette::Grass,
            },
            BiomeId::ColdDesert => Self {
                mountains_amplitude: 1.0,
                ridge_strength: 0.2,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Desert,
                scatter_density: 0.2,
                scatter_species_set: ScatterSpeciesSet::Desert,
                surface_color_palette: SurfaceColorPalette::DryGrass,
            },

            // === Cold ===
            BiomeId::BorealForest => Self {
                mountains_amplitude: 1.5,
                ridge_strength: 0.4,
                runevision_config: None,
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 1.0,
                scatter_species_set: ScatterSpeciesSet::Boreal,
                surface_color_palette: SurfaceColorPalette::Boreal,
            },
            BiomeId::Tundra => Self {
                mountains_amplitude: 1.5,
                ridge_strength: 0.4,
                runevision_config: None,
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 0.3,
                scatter_species_set: ScatterSpeciesSet::Tundra,
                surface_color_palette: SurfaceColorPalette::Tundra,
            },
            BiomeId::Alpine => Self {
                mountains_amplitude: 2.5,
                ridge_strength: 0.6,
                runevision_config: None, // F.4.B.3.C: mountain-character biomes opt out
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 0.4,
                scatter_species_set: ScatterSpeciesSet::Alpine,
                surface_color_palette: SurfaceColorPalette::Rock,
            },

            // === Aquatic ===
            BiomeId::Ocean => Self {
                mountains_amplitude: 0.0,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 0.0,
                scatter_species_set: ScatterSpeciesSet::None,
                surface_color_palette: SurfaceColorPalette::OceanWater,
            },
            BiomeId::Coast => Self {
                mountains_amplitude: 0.0,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 0.0,
                scatter_species_set: ScatterSpeciesSet::None,
                surface_color_palette: SurfaceColorPalette::OceanWater,
            },
            BiomeId::Beach => Self {
                mountains_amplitude: 0.2,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 0.1,
                scatter_species_set: ScatterSpeciesSet::None,
                surface_color_palette: SurfaceColorPalette::Sand,
            },
            BiomeId::River => Self {
                // River variant has no producer in lookup_biome (deferred
                // to Water System Rebuild). These defaults are
                // forward-compatible.
                mountains_amplitude: 0.2,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 0.4,
                scatter_species_set: ScatterSpeciesSet::Wetland,
                surface_color_palette: SurfaceColorPalette::Mud,
            },
            BiomeId::Wetland => Self {
                mountains_amplitude: 0.2,
                ridge_strength: 0.0,
                runevision_config: None,
                erosion_preset: ErosionPresetId::Coastal,
                scatter_density: 1.0,
                scatter_species_set: ScatterSpeciesSet::Wetland,
                surface_color_palette: SurfaceColorPalette::Mud,
            },

            // === Elevation overlays ===
            BiomeId::MountainRocky => Self {
                mountains_amplitude: 3.0,
                ridge_strength: 0.7,
                runevision_config: None, // F.4.B.3.C: mountain-character biomes opt out
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 0.0,
                scatter_species_set: ScatterSpeciesSet::BareRock,
                surface_color_palette: SurfaceColorPalette::Rock,
            },
            BiomeId::SnowCap => Self {
                mountains_amplitude: 2.5,
                ridge_strength: 0.5,
                runevision_config: None, // F.4.B.3.C: mountain-character biomes opt out
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 0.0,
                scatter_species_set: ScatterSpeciesSet::BareRock,
                surface_color_palette: SurfaceColorPalette::Snow,
            },
            BiomeId::Scree => Self {
                mountains_amplitude: 2.0,
                ridge_strength: 0.5,
                runevision_config: None, // F.4.B.3.C: mountain-character biomes opt out
                erosion_preset: ErosionPresetId::MountainBalanced,
                scatter_density: 0.0,
                scatter_species_set: ScatterSpeciesSet::BareRock,
                surface_color_palette: SurfaceColorPalette::Rock,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_1_6_f4_b_3_d_3_for_biome_total_over_all_variants() {
        // Every BiomeId resolves to a populated struct. No panic, no
        // fallback to a "default" branch — the match is exhaustive.
        for &biome in BiomeId::all() {
            let _params = BiomeParameters::for_biome(biome);
            // No assert — the call must not panic.
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_mountain_character_biomes_disable_runevision() {
        // F.4.B.3.C REGRESS finding: runevision composes badly with
        // high-amplitude mountain biomes. Mountain-character biomes
        // default to None in D.3.
        for &biome in &[
            BiomeId::Alpine,
            BiomeId::MountainRocky,
            BiomeId::SnowCap,
            BiomeId::Scree,
        ] {
            let params = BiomeParameters::for_biome(biome);
            assert!(
                params.runevision_config.is_none(),
                "{:?} must default runevision_config to None per F.4.B.3.C",
                biome
            );
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_aquatic_biomes_have_zero_mountains() {
        // Water doesn't have mountain contributions.
        for &biome in &[BiomeId::Ocean, BiomeId::Coast] {
            let params = BiomeParameters::for_biome(biome);
            assert_eq!(
                params.mountains_amplitude, 0.0,
                "{:?} must have mountains_amplitude=0.0 (water has no mountain contribution)",
                biome
            );
            assert_eq!(
                params.ridge_strength, 0.0,
                "{:?} must have ridge_strength=0.0",
                biome
            );
            assert!(matches!(
                params.scatter_species_set,
                ScatterSpeciesSet::None
            ));
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_alpine_biomes_have_dramatic_mountains() {
        // High-altitude biomes get strong mountain amplitude.
        for &biome in &[
            BiomeId::Alpine,
            BiomeId::MountainRocky,
            BiomeId::SnowCap,
            BiomeId::Scree,
        ] {
            let params = BiomeParameters::for_biome(biome);
            assert!(
                params.mountains_amplitude >= 2.0,
                "{:?} must have mountains_amplitude >= 2.0; got {}",
                biome,
                params.mountains_amplitude
            );
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_grassland_has_low_ridge_strength() {
        // Rolling biomes (grasslands, savanna) have minimal ridges.
        let grassland = BiomeParameters::for_biome(BiomeId::TemperateGrassland);
        assert!(
            grassland.ridge_strength < 0.2,
            "TemperateGrassland ridge_strength should be low; got {}",
            grassland.ridge_strength
        );
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_for_biome_is_pure_function() {
        // Same input always returns equivalent output (no global state).
        let a = BiomeParameters::for_biome(BiomeId::TemperateDeciduousForest);
        let b = BiomeParameters::for_biome(BiomeId::TemperateDeciduousForest);
        assert_eq!(a.mountains_amplitude, b.mountains_amplitude);
        assert_eq!(a.ridge_strength, b.ridge_strength);
        assert_eq!(a.erosion_preset, b.erosion_preset);
        assert_eq!(a.scatter_species_set, b.scatter_species_set);
        assert_eq!(a.surface_color_palette, b.surface_color_palette);
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_erosion_preset_id_resolves_to_full_preset() {
        // Each ID resolves to a non-panicking ErosionPreset.
        for id in [
            ErosionPresetId::DefaultBalanced,
            ErosionPresetId::MountainBalanced,
            ErosionPresetId::Desert,
            ErosionPresetId::Coastal,
            ErosionPresetId::Mountain,
        ] {
            let _preset = id.resolve();
            // No assert — call must not panic.
        }
    }

    #[test]
    fn phase_1_6_f4_b_3_d_3_spot_checks_six_diverse_biomes() {
        // §1.6 requires spot-checking ~6 biomes. Verify their defaults
        // match the documented intent.
        let trf = BiomeParameters::for_biome(BiomeId::TropicalRainforest);
        assert!(matches!(
            trf.scatter_species_set,
            ScatterSpeciesSet::Tropical
        ));
        assert!(matches!(trf.surface_color_palette, SurfaceColorPalette::Forest));

        let tdf = BiomeParameters::for_biome(BiomeId::TemperateDeciduousForest);
        assert!(matches!(tdf.scatter_species_set, ScatterSpeciesSet::Forest));

        let tundra = BiomeParameters::for_biome(BiomeId::Tundra);
        assert!(matches!(tundra.scatter_species_set, ScatterSpeciesSet::Tundra));
        assert!(matches!(
            tundra.surface_color_palette,
            SurfaceColorPalette::Tundra
        ));

        let alpine = BiomeParameters::for_biome(BiomeId::Alpine);
        assert!(matches!(alpine.scatter_species_set, ScatterSpeciesSet::Alpine));
        assert!(matches!(alpine.surface_color_palette, SurfaceColorPalette::Rock));
        assert!(alpine.runevision_config.is_none()); // mountain-character

        let rocky = BiomeParameters::for_biome(BiomeId::MountainRocky);
        assert!(matches!(rocky.scatter_species_set, ScatterSpeciesSet::BareRock));
        assert_eq!(rocky.scatter_density, 0.0);

        let ocean = BiomeParameters::for_biome(BiomeId::Ocean);
        assert!(matches!(ocean.scatter_species_set, ScatterSpeciesSet::None));
        assert!(matches!(ocean.surface_color_palette, SurfaceColorPalette::OceanWater));
    }
}
