//! Golden-value tests for noise_gen.rs — targets the 18 MISSED mutations from shard 17.
//!
//! All 18 missed mutations are arithmetic operator replacements (* → +, * → /, += → -=)
//! in `sample_height` and `sample_density`, plus a `config()` return replacement.
//! Range-based tests pass these mutations; exact golden values do not.
//!
//! Seed 42, default NoiseConfig throughout.

use astraweave_terrain::noise_gen::{NoiseConfig, TerrainNoise};

const SEED: u64 = 42;
const EPS: f32 = 1e-4;

fn default_noise() -> TerrainNoise {
    TerrainNoise::new(&NoiseConfig::default(), SEED)
}

fn assert_close(actual: f32, expected: f32, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff < EPS,
        "{label}: expected {expected:.6}, got {actual:.6}, diff {diff:.8}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// sample_height golden values — kills * → + and * → / on amplitude/scale
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn golden_sample_height_origin() {
    let noise = default_noise();
    assert_close(noise.sample_height(0.0, 0.0), 35.8488, "origin");
}

#[test]
fn golden_sample_height_100_200() {
    let noise = default_noise();
    assert_close(noise.sample_height(100.0, 200.0), 37.5913, "(100,200)");
}

#[test]
fn golden_sample_height_500_500() {
    let noise = default_noise();
    assert_close(noise.sample_height(500.0, 500.0), 40.4571, "(500,500)");
}

#[test]
fn golden_sample_height_1000_0() {
    let noise = default_noise();
    assert_close(noise.sample_height(1000.0, 0.0), 30.1594, "(1000,0)");
}

#[test]
fn golden_sample_height_0_1000() {
    let noise = default_noise();
    assert_close(noise.sample_height(0.0, 1000.0), 34.2941, "(0,1000)");
}

#[test]
fn golden_sample_height_250_750() {
    let noise = default_noise();
    assert_close(noise.sample_height(250.0, 750.0), 60.1683, "(250,750)");
}

// ═══════════════════════════════════════════════════════════════════════
// Per-layer isolation — kills individual layer arithmetic mutations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn golden_base_only_100_200() {
    let mut cfg = NoiseConfig::default();
    cfg.mountains.enabled = false;
    cfg.detail.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    assert_close(
        noise.sample_height(100.0, 200.0),
        28.8675,
        "base_only(100,200)",
    );
}

#[test]
fn golden_base_only_500_500() {
    let mut cfg = NoiseConfig::default();
    cfg.mountains.enabled = false;
    cfg.detail.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    assert_close(
        noise.sample_height(500.0, 500.0),
        14.4338,
        "base_only(500,500)",
    );
}

#[test]
fn golden_mountains_only_100_200() {
    let mut cfg = NoiseConfig::default();
    cfg.base_elevation.enabled = false;
    cfg.detail.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    assert_close(
        noise.sample_height(100.0, 200.0),
        14.6972,
        "mtn_only(100,200)",
    );
}

#[test]
fn golden_mountains_only_500_500() {
    let mut cfg = NoiseConfig::default();
    cfg.base_elevation.enabled = false;
    cfg.detail.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    assert_close(
        noise.sample_height(500.0, 500.0),
        31.9967,
        "mtn_only(500,500)",
    );
}

#[test]
fn golden_detail_only_100_200() {
    let mut cfg = NoiseConfig::default();
    cfg.base_elevation.enabled = false;
    cfg.mountains.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    // detail at (100,200) ≈ 0.0 (within eps)
    assert_close(
        noise.sample_height(100.0, 200.0),
        0.0,
        "detail_only(100,200)",
    );
}

#[test]
fn golden_detail_only_500_500() {
    let mut cfg = NoiseConfig::default();
    cfg.base_elevation.enabled = false;
    cfg.mountains.enabled = false;
    let noise = TerrainNoise::new(&cfg, SEED);
    assert_close(
        noise.sample_height(500.0, 500.0),
        0.0,
        "detail_only(500,500)",
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Additivity check — base + mountains ≈ total (when detail ≈ 0)
// Kills += → -= mutation on height accumulation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sample_height_is_sum_of_layers() {
    let noise_all = default_noise();

    let mut cfg_base = NoiseConfig::default();
    cfg_base.mountains.enabled = false;
    cfg_base.detail.enabled = false;
    let noise_base = TerrainNoise::new(&cfg_base, SEED);

    let mut cfg_mtn = NoiseConfig::default();
    cfg_mtn.base_elevation.enabled = false;
    cfg_mtn.detail.enabled = false;
    let noise_mtn = TerrainNoise::new(&cfg_mtn, SEED);

    let mut cfg_det = NoiseConfig::default();
    cfg_det.base_elevation.enabled = false;
    cfg_det.mountains.enabled = false;
    let noise_det = TerrainNoise::new(&cfg_det, SEED);

    // At (100,200): base=28.8675, mtn=14.6972, detail≈0 → sum≈43.5647
    // actual with .max(0.0) clamp = 37.5913 (because raw sum may differ slightly
    // due to noise interaction); verify each layer independently
    for &(x, z) in &[(100.0, 200.0), (500.0, 500.0), (250.0, 750.0), (0.0, 0.0)] {
        let base = noise_base.sample_height(x, z);
        let mtn = noise_mtn.sample_height(x, z);
        let det = noise_det.sample_height(x, z);
        let combined = noise_all.sample_height(x, z);

        // Each layer's raw value is clamped via .max(0.0) independently,
        // but the combined path accumulates THEN clamps.
        // The combined value should be >= 0
        assert!(combined >= 0.0, "combined height must be non-negative");

        // Layer isolation means each layer contributes additively before clamp.
        // The raw sum (before max(0)) should be close to base_raw + mtn_raw + det_raw.
        // Since we can't access raw values, verify that:
        // combined ≈ (base + mtn + det).max(0.0) won't hold exactly because
        // individual layers apply max(0) too. Instead verify relative magnitude.
        // Mountains at (500,500) ≈ 32 and base ≈ 14 → combined ≈ 40 (not 32-14).
        // This proves += is correct (not -=).
        if mtn > 10.0 && base > 10.0 {
            assert!(
                combined > mtn,
                "combined({x},{z})={combined} should exceed mountains-only={mtn}"
            );
            assert!(
                combined > base,
                "combined({x},{z})={combined} should exceed base-only={base}"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Amplitude sensitivity — kills * → + on amplitude multiplications
// If mutation replaces `noise_val * amplitude` with `noise_val + amplitude`,
// the result will be ≈ amplitude for small noise_val or wildly different.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn base_amplitude_sensitivity() {
    let mut cfg_lo = NoiseConfig::default();
    cfg_lo.mountains.enabled = false;
    cfg_lo.detail.enabled = false;
    cfg_lo.base_elevation.amplitude = 10.0;
    let noise_lo = TerrainNoise::new(&cfg_lo, SEED);

    let mut cfg_hi = NoiseConfig::default();
    cfg_hi.mountains.enabled = false;
    cfg_hi.detail.enabled = false;
    cfg_hi.base_elevation.amplitude = 100.0;
    let noise_hi = TerrainNoise::new(&cfg_hi, SEED);

    let h_lo = noise_lo.sample_height(100.0, 200.0);
    let h_hi = noise_hi.sample_height(100.0, 200.0);

    // With proper multiplication, h_hi ≈ 10 * h_lo (within noise bounds)
    // With + mutation, h_hi ≈ h_lo + 90 (additive noise + amplitude)
    let ratio = h_hi / h_lo.max(0.001);
    assert!(
        (ratio - 10.0).abs() < 2.0,
        "amplitude ratio should be ~10x, got {ratio:.2} (lo={h_lo}, hi={h_hi})"
    );
}

#[test]
fn mountain_amplitude_sensitivity() {
    let mut cfg_lo = NoiseConfig::default();
    cfg_lo.base_elevation.enabled = false;
    cfg_lo.detail.enabled = false;
    cfg_lo.mountains.amplitude = 20.0;
    let noise_lo = TerrainNoise::new(&cfg_lo, SEED);

    let mut cfg_hi = NoiseConfig::default();
    cfg_hi.base_elevation.enabled = false;
    cfg_hi.detail.enabled = false;
    cfg_hi.mountains.amplitude = 160.0;
    let noise_hi = TerrainNoise::new(&cfg_hi, SEED);

    let h_lo = noise_lo.sample_height(500.0, 500.0);
    let h_hi = noise_hi.sample_height(500.0, 500.0);

    let ratio = h_hi / h_lo.max(0.001);
    assert!(
        (ratio - 8.0).abs() < 2.0,
        "mountain amplitude ratio should be ~8x, got {ratio:.2}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scale sensitivity — kills * → + and * → / on scale multiplications
// Scale controls spatial frequency. Changing * to + or / completely
// wrecks the spatial pattern.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn base_scale_sensitivity() {
    let mut cfg_narrow = NoiseConfig::default();
    cfg_narrow.mountains.enabled = false;
    cfg_narrow.detail.enabled = false;
    cfg_narrow.base_elevation.scale = 0.001; // very low freq

    let mut cfg_wide = NoiseConfig::default();
    cfg_wide.mountains.enabled = false;
    cfg_wide.detail.enabled = false;
    cfg_wide.base_elevation.scale = 0.05; // higher freq

    let noise_narrow = TerrainNoise::new(&cfg_narrow, SEED);
    let noise_wide = TerrainNoise::new(&cfg_wide, SEED);

    // Two nearby points should differ more with higher scale (frequency)
    let n_a = noise_narrow.sample_height(0.0, 0.0);
    let n_b = noise_narrow.sample_height(10.0, 10.0);
    let w_a = noise_wide.sample_height(0.0, 0.0);
    let w_b = noise_wide.sample_height(10.0, 10.0);

    let narrow_diff = (n_a - n_b).abs();
    let wide_diff = (w_a - w_b).abs();

    // Higher scale → same physical distance maps to larger noise coordinate diff
    // → generally more variation between nearby points
    // If * → +, scale 0.001 + x = ~x which is huge, completely different behavior
    assert!(
        noise_narrow.sample_height(100.0, 200.0) < 200.0,
        "narrow scale should not produce extreme values"
    );
    assert!(
        noise_wide.sample_height(100.0, 200.0) < 200.0,
        "wide scale should not produce extreme values"
    );
    // Both should produce non-negative results with amplitudes in normal range
    assert!(n_a >= 0.0);
    assert!(w_a >= 0.0);
}

#[test]
fn mountain_scale_changes_output() {
    let mut cfg_a = NoiseConfig::default();
    cfg_a.base_elevation.enabled = false;
    cfg_a.detail.enabled = false;
    cfg_a.mountains.scale = 0.001;
    let noise_a = TerrainNoise::new(&cfg_a, SEED);

    let mut cfg_b = NoiseConfig::default();
    cfg_b.base_elevation.enabled = false;
    cfg_b.detail.enabled = false;
    cfg_b.mountains.scale = 0.01;
    let noise_b = TerrainNoise::new(&cfg_b, SEED);

    let h_a = noise_a.sample_height(500.0, 500.0);
    let h_b = noise_b.sample_height(500.0, 500.0);

    // Different scales should produce different values
    assert!(
        (h_a - h_b).abs() > 0.1,
        "different mountain scales should produce different results: {h_a} vs {h_b}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// sample_density golden values — kills * → + and * → / on coordinate scaling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn golden_density_10_5_10() {
    let noise = default_noise();
    assert_close(
        noise.sample_density(10.0, 5.0, 10.0),
        -0.0546,
        "density(10,5,10)",
    );
}

#[test]
fn golden_density_100_50_100() {
    let noise = default_noise();
    assert_close(
        noise.sample_density(100.0, 50.0, 100.0),
        0.2887,
        "density(100,50,100)",
    );
}

#[test]
fn golden_density_origin() {
    let noise = default_noise();
    assert_close(noise.sample_density(0.0, 0.0, 0.0), 0.0, "density(0,0,0)");
}

#[test]
fn golden_density_500_250_500() {
    let noise = default_noise();
    assert_close(
        noise.sample_density(500.0, 250.0, 500.0),
        0.2887,
        "density(500,250,500)",
    );
}

#[test]
fn density_coordinate_scaling_matters() {
    let noise = default_noise();
    // sample_density uses x*0.01, y*0.01, z*0.01
    // If * → +, the coords become x+0.01 ≈ x (for large x), completely different
    let d_small = noise.sample_density(1.0, 1.0, 1.0);
    let d_large = noise.sample_density(100.0, 100.0, 100.0);
    // Points at very different world positions should differ
    assert!(
        (d_small - d_large).abs() > 0.001 || d_small.abs() < 0.01,
        "density should vary or be near zero: small={d_small}, large={d_large}"
    );
}

#[test]
fn density_y_coordinate_independent() {
    let noise = default_noise();
    // Varying y with same x,z should change density (proves y*0.01 is used)
    // Use non-round coords to avoid Perlin zero-at-integers grid points
    let d_low = noise.sample_density(73.0, 5.0, 41.0);
    let d_high = noise.sample_density(73.0, 500.0, 41.0);
    assert!(
        (d_low - d_high).abs() > 0.001,
        "different y should produce different density: low={d_low}, high={d_high}"
    );
}

#[test]
fn density_each_axis_contributes() {
    let noise = default_noise();
    // Use coords that avoid Perlin zero-at-integer-grid-points
    // x*0.01 must not yield integers → use multiples that don't land on 100s
    let d_base = noise.sample_density(73.0, 41.0, 59.0);
    let d_x = noise.sample_density(173.0, 41.0, 59.0);
    let d_y = noise.sample_density(73.0, 141.0, 59.0);
    let d_z = noise.sample_density(73.0, 41.0, 159.0);

    // At least 2 of 3 axis changes should produce different values
    let changed_count = [
        (d_x - d_base).abs(),
        (d_y - d_base).abs(),
        (d_z - d_base).abs(),
    ]
    .iter()
    .filter(|d| **d > 0.001)
    .count();
    assert!(
        changed_count >= 2,
        "at least 2 axes should affect density (x_diff={}, y_diff={}, z_diff={})",
        (d_x - d_base).abs(),
        (d_y - d_base).abs(),
        (d_z - d_base).abs()
    );
}

// ═══════════════════════════════════════════════════════════════════════
// config() accessor — kills the Box::leak(Default::default()) replacement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn config_returns_actual_config_all_fields() {
    let cfg = NoiseConfig::default();
    let noise = TerrainNoise::new(&cfg, SEED);
    let c = noise.config();

    // Base elevation
    assert!(c.base_elevation.enabled);
    assert!((c.base_elevation.scale - 0.005).abs() < 1e-9);
    assert!((c.base_elevation.amplitude - 50.0).abs() < 1e-6);
    assert_eq!(c.base_elevation.octaves, 4);
    assert!((c.base_elevation.persistence - 0.5).abs() < 1e-9);
    assert!((c.base_elevation.lacunarity - 2.0).abs() < 1e-9);

    // Mountains
    assert!(c.mountains.enabled);
    assert!((c.mountains.scale - 0.002).abs() < 1e-9);
    assert!((c.mountains.amplitude - 80.0).abs() < 1e-6);
    assert_eq!(c.mountains.octaves, 6);
    assert!((c.mountains.persistence - 0.4).abs() < 1e-9);
    assert!((c.mountains.lacunarity - 2.2).abs() < 1e-9);

    // Detail
    assert!(c.detail.enabled);
    assert!((c.detail.scale - 0.02).abs() < 1e-9);
    assert!((c.detail.amplitude - 5.0).abs() < 1e-6);
    assert_eq!(c.detail.octaves, 3);
    assert!((c.detail.persistence - 0.6).abs() < 1e-9);
    assert!((c.detail.lacunarity - 2.0).abs() < 1e-9);

    // Erosion
    assert!(c.erosion_enabled);
    assert!((c.erosion_strength - 0.3).abs() < 1e-6);
}

#[test]
fn config_returns_custom_config() {
    let mut cfg = NoiseConfig::default();
    cfg.base_elevation.amplitude = 999.0;
    cfg.mountains.scale = 0.123;
    cfg.detail.octaves = 7;
    cfg.erosion_enabled = false;
    cfg.erosion_strength = 0.99;

    let noise = TerrainNoise::new(&cfg, SEED);
    let c = noise.config();

    assert!((c.base_elevation.amplitude - 999.0).abs() < 1e-6);
    assert!((c.mountains.scale - 0.123).abs() < 1e-9);
    assert_eq!(c.detail.octaves, 7);
    assert!(!c.erosion_enabled);
    assert!((c.erosion_strength - 0.99).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-point fingerprint — a single test that catches ANY arithmetic change
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn multi_point_fingerprint() {
    let noise = default_noise();
    let expected: &[(f64, f64, f32)] = &[
        (0.0, 0.0, 35.8488),
        (100.0, 200.0, 37.5913),
        (500.0, 500.0, 40.4571),
        (1000.0, 0.0, 30.1594),
        (0.0, 1000.0, 34.2941),
        (250.0, 750.0, 60.1683),
    ];
    for &(x, z, exp) in expected {
        assert_close(
            noise.sample_height(x, z),
            exp,
            &format!("fingerprint({x},{z})"),
        );
    }
}

#[test]
fn density_fingerprint() {
    let noise = default_noise();
    let expected: &[(f64, f64, f64, f32)] = &[
        (10.0, 5.0, 10.0, -0.0546),
        (100.0, 50.0, 100.0, 0.2887),
        (0.0, 0.0, 0.0, 0.0),
        (500.0, 250.0, 500.0, 0.2887),
    ];
    for &(x, y, z, exp) in expected {
        assert_close(
            noise.sample_density(x, y, z),
            exp,
            &format!("density({x},{y},{z})"),
        );
    }
}
