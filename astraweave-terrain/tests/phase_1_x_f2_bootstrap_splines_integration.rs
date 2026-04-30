//! Phase 1.X-F.2.C: integration tests for `WorldArchetype.bootstrap_splines`.
//!
//! Verifies F.2.C's `WorldArchetype` extension wires correctly with F.2.B's
//! catalog factory functions:
//!
//! 1. Continental Temperate from D.5 catalog matches F.2.B's
//!    `bootstrap_splines_continental_temperate()` output at median climate.
//! 2. All 6 catalog archetypes have `bootstrap_splines` populated and
//!    produce baseline `BootstrapParams` byte-identical at median climate.
//! 3. Existing D.5 climate envelope fields (temperature_mean,
//!    moisture_mean, etc.) unchanged from D.5 documented values.
//! 4. F.4.B.3.D.5-fix Path B regression preserved — F.2.C only adds a
//!    struct field; doesn't modify existing terrain output paths.
//!
//! Per F.2 prompt §3 scope discipline: tests only; no production code
//! changes.

use astraweave_terrain::climate::ClimateSample;
use astraweave_terrain::spline_types::{
    bootstrap_splines_continental_temperate, BootstrapParams,
    D5FIX_BASELINE_BASE_ELEVATION_AMPLITUDE, D5FIX_BASELINE_CONTINENTAL_SCALE,
    D5FIX_BASELINE_MOUNTAINS_AMPLITUDE, D5FIX_BASELINE_MOUNTAINS_SCALE,
};
use astraweave_terrain::world_archetypes::{self, WorldArchetypeId};

/// Median climate sample used for all F.2 archetype-equivalence tests.
/// weirdness=1.0 → pv=0.0 (mid); other fields at archetype-default-near
/// values. Same construction as F.2.B's `median_climate_sample` helper.
fn median_climate_sample() -> ClimateSample {
    ClimateSample {
        temperature_c: 12.0,
        moisture_mm: 800.0,
        continentalness: 0.5,
        erosion: 0.0,
        weirdness: 1.0,
    }
}

/// D.5 catalog Continental Temperate archetype's `bootstrap_splines`
/// produces the same `BootstrapParams` at median climate as F.2.B's
/// `bootstrap_splines_continental_temperate()` factory output. Verifies
/// the catalog wiring routes through the correct factory.
#[test]
fn world_archetype_continental_temperate_bootstrap_splines_match_factory() {
    let archetype = world_archetypes::continental_temperate();
    let factory_set = bootstrap_splines_continental_temperate();
    let sample = median_climate_sample();

    let from_archetype = archetype.bootstrap_splines.evaluate(&sample);
    let from_factory = factory_set.evaluate(&sample);

    assert_eq!(
        from_archetype, from_factory,
        "WorldArchetype catalog Continental Temperate's bootstrap_splines \
         must produce same BootstrapParams as factory output"
    );
}

/// All 6 catalog archetypes have `bootstrap_splines` populated and
/// produce F.4.B.3.D.5-fix baseline byte-identical at median climate.
/// F.2's load-bearing regression contract: catalog archetypes ship at
/// baseline; F.7 differentiates.
#[test]
fn world_archetype_six_catalog_archetypes_have_bootstrap_splines_field() {
    let sample = median_climate_sample();
    let expected = BootstrapParams {
        mountains_amplitude: D5FIX_BASELINE_MOUNTAINS_AMPLITUDE,
        mountains_scale: D5FIX_BASELINE_MOUNTAINS_SCALE,
        continental_scale: D5FIX_BASELINE_CONTINENTAL_SCALE,
        base_elevation_amplitude: D5FIX_BASELINE_BASE_ELEVATION_AMPLITUDE,
    };

    for id in WorldArchetypeId::all() {
        let archetype = id.default_archetype();
        let params = archetype.bootstrap_splines.evaluate(&sample);
        assert_eq!(
            params, expected,
            "{:?}: bootstrap_splines.evaluate must produce baseline at \
             median climate",
            id
        );
    }
}

/// Existing D.5 climate envelope fields (temperature_mean,
/// moisture_mean, latitude_drop, etc.) unchanged from D.5 documented
/// values. Catches the failure mode where F.2.C accidentally drifted
/// existing values when adding the new field.
///
/// Reference values per D.5 plan §1.1 and `world_archetypes.rs` doc
/// comments at the time of F.2.C landing.
#[test]
fn world_archetype_existing_climate_envelope_unchanged() {
    let ct = world_archetypes::continental_temperate();
    assert_eq!(ct.temperature_mean_c, 12.0);
    assert_eq!(ct.temperature_variance_c, 8.0);
    assert_eq!(ct.latitude_temperature_drop_c, 10.0);
    assert_eq!(ct.moisture_mean_mm, 1100.0);
    assert_eq!(ct.moisture_variance_mm, 400.0);
    assert_eq!(ct.continentalness_mean, 0.5);
    assert_eq!(ct.continentalness_variance, 0.2);

    let et = world_archetypes::equatorial_tropical();
    assert_eq!(et.temperature_mean_c, 26.0);
    assert_eq!(et.moisture_mean_mm, 1900.0);
    assert_eq!(et.latitude_temperature_drop_c, 3.0);

    let bs = world_archetypes::boreal_subarctic();
    assert_eq!(bs.temperature_mean_c, -3.0);
    assert_eq!(bs.latitude_temperature_drop_c, 15.0);

    let med = world_archetypes::mediterranean();
    assert_eq!(med.temperature_mean_c, 17.0);
    assert_eq!(med.moisture_variance_mm, 350.0);

    let des = world_archetypes::desert();
    assert_eq!(des.temperature_mean_c, 25.0);
    assert_eq!(des.continentalness_mean, 0.7);
    assert_eq!(des.continentalness_variance, 0.3);
}

/// Custom archetype routes through Continental Temperate baseline (per
/// D.5b's "Custom defaults to CT" pattern). Verify both the climate
/// envelope and the bootstrap_splines match.
#[test]
fn world_archetype_custom_defaults_to_continental_temperate() {
    let custom = WorldArchetypeId::Custom.default_archetype();
    let ct = world_archetypes::continental_temperate();
    assert_eq!(custom.temperature_mean_c, ct.temperature_mean_c);
    assert_eq!(custom.moisture_mean_mm, ct.moisture_mean_mm);
    assert_eq!(custom.bootstrap_splines, ct.bootstrap_splines);
}

/// `WorldArchetype::default()` (the trait impl in climate.rs) delegates
/// to Continental Temperate factory. Both should produce identical
/// archetypes including `bootstrap_splines`.
#[test]
fn world_archetype_default_matches_continental_temperate() {
    let default_arch = astraweave_terrain::climate::WorldArchetype::default();
    let factory_ct = world_archetypes::continental_temperate();
    assert_eq!(default_arch, factory_ct);
}

/// `BootstrapSplineSet::default()` produces F.4.B.3.D.5-fix baseline.
/// Used by `#[serde(default)]` on `WorldArchetype.bootstrap_splines` so
/// worlds serialized before F.2.C deserialize cleanly with baseline
/// fallback.
#[test]
fn bootstrap_spline_set_default_is_d5fix_baseline() {
    let default_set = astraweave_terrain::spline_types::BootstrapSplineSet::default();
    let factory_ct = bootstrap_splines_continental_temperate();
    let sample = median_climate_sample();
    assert_eq!(default_set.evaluate(&sample), factory_ct.evaluate(&sample));
}
