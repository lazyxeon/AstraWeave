//! Phase 1.X-F.3.C: F.3 regression + smoke tests.
//!
//! Verifies F.3.A's `sample_height_with_params` method and F.3.B's
//! `WorldGenerator` integration meet the F.3 prompt §2.3 specification:
//!
//! 1. **F.4.B.3.D.5-fix Path B byte-identity** — Continental Temperate
//!    seed 12345 produces byte-identical output through the F.3-wired
//!    pipeline. Load-bearing regression contract per F.3 prompt §6.3.
//! 2. **Spline toggling falsifiable smoke test** — proves splines are
//!    actually wired by doubling `mountains_amplitude` and observing a
//!    measurable height increase at a position with substantial mountain
//!    contribution.
//! 3. **Median climate sample produces baseline `BootstrapParams`** —
//!    end-to-end sanity that F.3's wiring evaluates archetype splines
//!    correctly per the F.4.B.3.D.5-fix baseline contract.
//!
//! Per F.3 prompt §3 scope discipline: tests only; no production code
//! changes. Per F.3 prompt §0: byte-identity contract is load-bearing;
//! F.3.D Andrew-gate is the authoritative completion signal.

use astraweave_terrain::climate::ClimateSample;
use astraweave_terrain::spline_types::{
    bootstrap_splines_continental_temperate, BootstrapParams,
    D5FIX_BASELINE_BASE_ELEVATION_AMPLITUDE, D5FIX_BASELINE_CONTINENTAL_SCALE,
    D5FIX_BASELINE_MOUNTAINS_AMPLITUDE, D5FIX_BASELINE_MOUNTAINS_SCALE,
};
use astraweave_terrain::{NoiseConfig, TerrainNoise};

// =============================================================================
// §2.3 Test 1: F.4.B.3.D.5-fix Path B byte-identity
// =============================================================================

/// Build the F.4.B.3.D.5-fix baseline `BootstrapParams` from the
/// `D5FIX_BASELINE_*` consts. Tests assert that a spline-driven path at
/// these baseline values produces byte-identical output to the legacy
/// `NoiseConfig::default()`-driven path.
fn baseline_bootstrap_params() -> BootstrapParams {
    BootstrapParams {
        mountains_amplitude: D5FIX_BASELINE_MOUNTAINS_AMPLITUDE,
        mountains_scale: D5FIX_BASELINE_MOUNTAINS_SCALE,
        continental_scale: D5FIX_BASELINE_CONTINENTAL_SCALE,
        base_elevation_amplitude: D5FIX_BASELINE_BASE_ELEVATION_AMPLITUDE,
    }
}

/// Continental Temperate at seed 12345 produces byte-identical heights
/// through the F.3-wired path (`sample_height_with_params` with baseline
/// params + multiplier 1.0) vs the legacy path (`sample_height`). 100
/// sample positions across the Target B world; max divergence must be
/// exactly 0.0.
///
/// **F.3 prompt §6.3 contract**: byte-identical was the primary
/// assertion; F.3.A initially failed it (~60-ulp drift) but F.3.B's
/// `mountains_scale: f32 → f64` retrofit restored byte-identity. If a
/// future change reintroduces drift, this test surfaces the regression.
#[test]
fn phase_1_x_f3_continental_temperate_byte_identical_to_f4b3d_d5fix() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 12345);
    let params = baseline_bootstrap_params();

    let mut max_diff = 0.0f32;
    for i in 0..10 {
        for j in 0..10 {
            let x = -5000.0 + i as f64 * 1000.0;
            let z = -5000.0 + j as f64 * 1000.0;
            let legacy = noise.sample_height(x, z);
            let new = noise.sample_height_with_params(&params, x, z, 1.0);
            let diff = (legacy - new).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }
    }

    assert_eq!(
        max_diff, 0.0,
        "F.3-wired path must produce byte-identical output to F.4.B.3.D.5-fix \
         Path B baseline at Continental Temperate seed 12345; max divergence={:.9}m",
        max_diff
    );
}

// =============================================================================
// §2.3 Test 2: Spline toggling falsifiable smoke test
// =============================================================================

/// Doubling `params.mountains_amplitude` produces a measurable height
/// increase at a position with substantial mountain contribution.
/// Falsifiable smoke test that splines actually drive the mountain
/// layer end-to-end.
///
/// **Falsifiability contract**:
/// - Splines wired correctly: doubling mountains_amplitude approximately
///   doubles the mountain layer's contribution; delta is positive and
///   substantial (> 50m at this position).
/// - Splines NOT wired (sample_height_with_params ignores params):
///   delta ≈ 0.
/// - Wrong field doubled (e.g., base_elevation_amplitude instead):
///   delta is small or wrong sign.
///
/// Position selection: scan candidate positions at Continental Temperate
/// seed 12345; pick the first position with baseline pre-erosion height
/// > 100m, ensuring mountain layer contribution is dominant. Documented
/// inline.
#[test]
fn phase_1_x_f3_spline_toggling_changes_terrain() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 12345);
    let baseline = baseline_bootstrap_params();
    let mut doubled = baseline;
    doubled.mountains_amplitude *= 2.0;

    // Search for a position with substantial mountain contribution.
    let mut chosen: Option<(f64, f64, f32)> = None;
    'outer: for i in 0..15 {
        for j in 0..15 {
            let x = -3500.0 + i as f64 * 500.0;
            let z = -3500.0 + j as f64 * 500.0;
            let h = noise.sample_height_with_params(&baseline, x, z, 1.0);
            if h > 100.0 {
                chosen = Some((x, z, h));
                break 'outer;
            }
        }
    }
    let (x, z, baseline_height) = chosen
        .expect("expected at least one Continental Temperate position with \
             pre-erosion height > 100m at seed 12345");
    let doubled_height = noise.sample_height_with_params(&doubled, x, z, 1.0);
    let delta = doubled_height - baseline_height;

    assert!(
        delta > 50.0,
        "doubling mountains_amplitude at ({}, {}) should produce >50m height \
         increase; baseline={:.1}m, doubled={:.1}m, delta={:.1}m. If delta is \
         near zero, splines are not wired into the noise pipeline.",
        x,
        z,
        baseline_height,
        doubled_height,
        delta
    );
}

// =============================================================================
// §2.3 Test 3: Median climate sample produces baseline BootstrapParams
// =============================================================================

/// At median climate, Continental Temperate's `BootstrapSplineSet`
/// produces `BootstrapParams` that byte-identically match the
/// `D5FIX_BASELINE_*` consts. Verifies F.3's spline evaluation routes
/// through F.2's catalog factory output correctly end-to-end.
///
/// Median climate: weirdness=1.0 → pv=0.0 (mid); other fields at
/// archetype-default-near values. F.2.B's catalog factories produce
/// single-control-point splines at the F.4.B.3.D.5-fix baseline, so
/// any climate sample yields the baseline output.
#[test]
fn phase_1_x_f3_continental_temperate_at_median_climate_matches_baseline() {
    let climate_sample = ClimateSample {
        temperature_c: 12.0,
        moisture_mm: 800.0,
        continentalness: 0.5,
        erosion: 0.0,
        weirdness: 1.0,
    };
    let splines = bootstrap_splines_continental_temperate();
    let params = splines.evaluate(&climate_sample);

    assert_eq!(
        params.mountains_amplitude.to_bits(),
        D5FIX_BASELINE_MOUNTAINS_AMPLITUDE.to_bits(),
        "mountains_amplitude must byte-identical match baseline (480.0)"
    );
    assert_eq!(
        params.mountains_scale.to_bits(),
        D5FIX_BASELINE_MOUNTAINS_SCALE.to_bits(),
        "mountains_scale must byte-identical match baseline (0.002 f64)"
    );
    assert_eq!(
        params.continental_scale.to_bits(),
        D5FIX_BASELINE_CONTINENTAL_SCALE.to_bits(),
        "continental_scale must byte-identical match baseline (0.0003 f32)"
    );
    assert_eq!(
        params.base_elevation_amplitude.to_bits(),
        D5FIX_BASELINE_BASE_ELEVATION_AMPLITUDE.to_bits(),
        "base_elevation_amplitude must byte-identical match baseline (150.0)"
    );
}

// =============================================================================
// §2.3 Test 4: Phase-2 continuity preservation via cross-reference
// =============================================================================

/// Documents that the predecessor campaign's phase-2 continuity test
/// (`phase_1_6_f3_phase_2_continuity::adjacent_chunks_share_edges_under_real_erosion_grassland`)
/// passes at the F.4.B.3.D.5-fix-tightened 80 WU threshold under F.3-wired
/// generation.
///
/// This test does NOT re-run the full phase-2 continuity check (which
/// takes ~45s due to erosion). Instead it serves as a documentation
/// landmark that the phase-2 continuity test continues to be the
/// load-bearing chunk-edge regression check post-F.3 wiring.
///
/// Manual verification: `cargo test -p astraweave-terrain --test
/// phase_1_6_f3_phase_2_continuity adjacent_chunks_share_edges_under_real_erosion_grassland`.
/// At F.3.B close, this test passed at 80 WU grassland / 10 WU mountain
/// thresholds (per F.3.B retrofit + commit `447367c15`).
#[test]
fn phase_1_x_f3_phase_2_continuity_preserved_documentation() {
    // No-op test; documentation only. The actual phase-2 continuity test
    // lives at astraweave-terrain/tests/phase_1_6_f3_phase_2_continuity.rs
    // and is run as part of the standard `cargo test -p astraweave-terrain`
    // pipeline.
    //
    // F.3.B retrofit (mountains_scale: f32 → f64) was specifically
    // motivated by initial F.3.B integration failing this test at 103 WU
    // divergence pre-retrofit. Post-retrofit: byte-identity restored,
    // phase-2 continuity passes at 80 WU.
}
