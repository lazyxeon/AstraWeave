//! Wave 2 Remediation Tests — noise_gen.rs (97 mutants) + noise_simd.rs (71 mutants)
//!
//! Targets: shards 16-17
//! Focus: Default config field values, seed offsets, sample_height arithmetic,
//!        utility functions (normalize, height_curve, island_mask), SIMD loop math.

use astraweave_terrain::noise_gen::NoiseType;
use astraweave_terrain::*;

// ============================================================================
// NoiseConfig default value pinning (many mutants from replacing field values)
// ============================================================================

#[test]
fn noise_config_default_base_elevation_fields() {
    let config = NoiseConfig::default();
    let base = &config.base_elevation;
    assert!(base.enabled, "base_elevation should be enabled");
    assert!((base.scale - 0.005).abs() < 1e-9, "base scale = 0.005");
    assert!(
        (base.amplitude - 50.0).abs() < 1e-6,
        "base amplitude = 50.0"
    );
    assert_eq!(base.octaves, 4, "base octaves = 4");
    assert!(
        (base.persistence - 0.5).abs() < 1e-9,
        "base persistence = 0.5"
    );
    assert!(
        (base.lacunarity - 2.0).abs() < 1e-9,
        "base lacunarity = 2.0"
    );
    assert!(matches!(base.noise_type, NoiseType::Perlin));
}

#[test]
fn noise_config_default_mountains_fields() {
    let config = NoiseConfig::default();
    let mtn = &config.mountains;
    assert!(mtn.enabled, "mountains should be enabled");
    assert!((mtn.scale - 0.002).abs() < 1e-9, "mountain scale = 0.002");
    assert!(
        (mtn.amplitude - 80.0).abs() < 1e-6,
        "mountain amplitude = 80.0"
    );
    assert_eq!(mtn.octaves, 6, "mountain octaves = 6");
    assert!(
        (mtn.persistence - 0.4).abs() < 1e-9,
        "mountain persistence = 0.4"
    );
    assert!(
        (mtn.lacunarity - 2.2).abs() < 1e-9,
        "mountain lacunarity = 2.2"
    );
    assert!(matches!(mtn.noise_type, NoiseType::RidgedNoise));
}

#[test]
fn noise_config_default_detail_fields() {
    let config = NoiseConfig::default();
    let det = &config.detail;
    assert!(det.enabled, "detail should be enabled");
    assert!((det.scale - 0.02).abs() < 1e-9, "detail scale = 0.02");
    assert!((det.amplitude - 5.0).abs() < 1e-6, "detail amplitude = 5.0");
    assert_eq!(det.octaves, 3, "detail octaves = 3");
    assert!(
        (det.persistence - 0.6).abs() < 1e-9,
        "detail persistence = 0.6"
    );
    assert!(
        (det.lacunarity - 2.0).abs() < 1e-9,
        "detail lacunarity = 2.0"
    );
    assert!(matches!(det.noise_type, NoiseType::Billow));
}

#[test]
fn noise_config_default_erosion_fields() {
    let config = NoiseConfig::default();
    assert!(
        config.erosion_enabled,
        "erosion should be enabled by default"
    );
    assert!(
        (config.erosion_strength - 0.3).abs() < 1e-6,
        "erosion_strength = 0.3"
    );
}

// ============================================================================
// TerrainNoise seed offset pinning
// ============================================================================

#[test]
fn terrain_noise_different_seeds_produce_different_heights() {
    let config = NoiseConfig::default();
    let noise_a = TerrainNoise::new(&config, 100);
    let noise_b = TerrainNoise::new(&config, 200);

    let h_a = noise_a.sample_height(50.0, 50.0);
    let h_b = noise_b.sample_height(50.0, 50.0);
    assert_ne!(h_a, h_b, "Different seeds must produce different heights");
}

#[test]
fn terrain_noise_seed_determinism() {
    let config = NoiseConfig::default();
    let a = TerrainNoise::new(&config, 42);
    let b = TerrainNoise::new(&config, 42);
    assert_eq!(
        a.sample_height(123.0, 456.0),
        b.sample_height(123.0, 456.0),
        "Same seed must produce identical heights"
    );
}

#[test]
fn terrain_noise_config_accessor() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 1);
    let returned = noise.config();
    assert!(returned.erosion_enabled);
    assert!((returned.erosion_strength - 0.3).abs() < 1e-6);
}

// ============================================================================
// sample_height: layer contribution, abs(), max(0.0)
// ============================================================================

#[test]
fn sample_height_non_negative() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 12345);

    // Sample many positions — all should be >= 0 due to .max(0.0)
    for x in (0..500).step_by(37) {
        for z in (0..500).step_by(41) {
            let h = noise.sample_height(x as f64, z as f64);
            assert!(h >= 0.0, "Height at ({x},{z}) = {h} should be >= 0");
        }
    }
}

#[test]
fn sample_height_with_only_base_elevation() {
    let mut config = NoiseConfig::default();
    config.mountains.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    // With only base elevation (amplitude 50.0), height should be moderate
    assert!(h < 100.0, "Base-only height should be < 100");
}

#[test]
fn sample_height_with_only_mountains() {
    let mut config = NoiseConfig::default();
    config.base_elevation.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    // Mountains use absolute value, so height >= 0
    assert!(h >= 0.0, "Mountain-only height >= 0 from abs()");
}

#[test]
fn sample_height_with_only_detail() {
    let mut config = NoiseConfig::default();
    config.base_elevation.enabled = false;
    config.mountains.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    // Detail amplitude = 5.0, so |h| should be modest
    // But .max(0.0) ensures >= 0
    assert!(h >= 0.0, "Detail-only height >= 0");
    assert!(h < 20.0, "Detail-only height should be small");
}

#[test]
fn sample_height_all_disabled_gives_zero() {
    let mut config = NoiseConfig::default();
    config.base_elevation.enabled = false;
    config.mountains.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    assert!((h - 0.0).abs() < 1e-6, "All layers disabled → height = 0.0");
}

#[test]
fn sample_height_varies_with_position() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 12345);

    let h1 = noise.sample_height(0.0, 0.0);
    let h2 = noise.sample_height(500.0, 500.0);
    // Terrain should vary across large distance
    assert_ne!(h1, h2, "Height should vary across positions");
}

// ============================================================================
// sample_density
// ============================================================================

#[test]
fn sample_density_varies_with_y() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);

    // Sample many Y values to find variation — noise may be zero at specific coords
    let mut values = Vec::new();
    for y in (0..1000).step_by(100) {
        values.push(noise.sample_density(37.0, y as f64, 53.0));
    }
    let any_diff = values.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-6);
    assert!(any_diff, "Density should vary with y-coordinate");
}

#[test]
fn sample_density_deterministic() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let d1 = noise.sample_density(10.0, 20.0, 30.0);
    let d2 = noise.sample_density(10.0, 20.0, 30.0);
    assert_eq!(d1, d2, "Density must be deterministic");
}

// ============================================================================
// generate_heightmap
// ============================================================================

#[test]
fn generate_heightmap_resolution_correct() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let chunk = ChunkId::new(0, 0);
    let hm = noise.generate_heightmap(chunk, 64.0, 16).unwrap();
    assert_eq!(hm.resolution(), 16);
}

#[test]
fn generate_heightmap_different_chunks_different() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);

    let hm0 = noise
        .generate_heightmap(ChunkId::new(0, 0), 256.0, 16)
        .unwrap();
    let hm1 = noise
        .generate_heightmap(ChunkId::new(5, 5), 256.0, 16)
        .unwrap();

    // At least some heights should differ between distant chunks
    let mut any_diff = false;
    for x in 0..16 {
        for z in 0..16 {
            if (hm0.get_height(x, z) - hm1.get_height(x, z)).abs() > 0.01 {
                any_diff = true;
            }
        }
    }
    assert!(
        any_diff,
        "Different chunks should produce different heights"
    );
}

// ============================================================================
// utils::normalize_heights
// ============================================================================

#[test]
fn normalize_heights_empty_noop() {
    let mut heights: Vec<f32> = vec![];
    noise_gen::utils::normalize_heights(&mut heights);
    assert!(heights.is_empty());
}

#[test]
fn normalize_heights_single_value() {
    let mut heights = vec![42.0];
    noise_gen::utils::normalize_heights(&mut heights);
    // Single value: range=0, so no normalization applied
    assert_eq!(heights[0], 42.0);
}

#[test]
fn normalize_heights_two_values() {
    let mut heights = vec![10.0, 20.0];
    noise_gen::utils::normalize_heights(&mut heights);
    assert!((heights[0] - 0.0).abs() < 1e-6, "Min should normalize to 0");
    assert!((heights[1] - 1.0).abs() < 1e-6, "Max should normalize to 1");
}

#[test]
fn normalize_heights_preserves_order() {
    let mut heights = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    noise_gen::utils::normalize_heights(&mut heights);
    assert!((heights[0] - 0.0).abs() < 1e-6);
    assert!((heights[4] - 1.0).abs() < 1e-6);
    assert!(heights[1] < heights[2]);
    assert!(heights[2] < heights[3]);
    // Midpoint (30) should normalize to 0.5
    assert!((heights[2] - 0.5).abs() < 1e-6);
}

#[test]
fn normalize_heights_all_same() {
    let mut heights = vec![5.0, 5.0, 5.0];
    noise_gen::utils::normalize_heights(&mut heights);
    // Range = 0, so values unchanged
    assert!(heights.iter().all(|&h| (h - 5.0).abs() < 1e-6));
}

// ============================================================================
// utils::apply_height_curve
// ============================================================================

#[test]
fn apply_height_curve_power_one() {
    let mut heights = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    noise_gen::utils::apply_height_curve(&mut heights, 1.0);
    // powf(1.0) * 100.0 = value * 100.0
    assert!((heights[0] - 0.0).abs() < 1e-6);
    assert!((heights[2] - 50.0).abs() < 1e-4);
    assert!((heights[4] - 100.0).abs() < 1e-4);
}

#[test]
fn apply_height_curve_power_two() {
    let mut heights = vec![0.0, 0.5, 1.0];
    noise_gen::utils::apply_height_curve(&mut heights, 2.0);
    // 0.0^2 * 100 = 0
    // 0.5^2 * 100 = 25
    // 1.0^2 * 100 = 100
    assert!((heights[0] - 0.0).abs() < 1e-6);
    assert!((heights[1] - 25.0).abs() < 1e-4);
    assert!((heights[2] - 100.0).abs() < 1e-4);
}

#[test]
fn apply_height_curve_clamps_input() {
    let mut heights = vec![-0.5, 1.5];
    noise_gen::utils::apply_height_curve(&mut heights, 1.0);
    // Negative clamped to 0.0, above 1.0 clamped to 1.0
    assert!(
        (heights[0] - 0.0).abs() < 1e-6,
        "Negative input clamped to 0"
    );
    assert!(
        (heights[1] - 100.0).abs() < 1e-4,
        ">1.0 input clamped to 1.0 → 100.0"
    );
}

// ============================================================================
// utils::create_island_mask
// ============================================================================

#[test]
fn create_island_mask_size() {
    let mask = noise_gen::utils::create_island_mask(32, 16.0, 16.0, 10.0);
    assert_eq!(mask.len(), 32 * 32);
}

#[test]
fn create_island_mask_center_is_one() {
    let mask = noise_gen::utils::create_island_mask(64, 32.0, 32.0, 20.0);
    let center_idx = 32 * 64 + 32;
    assert!(
        (mask[center_idx] - 1.0).abs() < 0.01,
        "Center should be ~1.0, got {}",
        mask[center_idx]
    );
}

#[test]
fn create_island_mask_far_corner_is_zero() {
    let mask = noise_gen::utils::create_island_mask(64, 32.0, 32.0, 10.0);
    // Corner (0,0) is far from center (32,32)
    assert!(mask[0] < 0.01, "Far corner should be ~0.0, got {}", mask[0]);
}

#[test]
fn create_island_mask_values_in_01_range() {
    let mask = noise_gen::utils::create_island_mask(32, 16.0, 16.0, 15.0);
    for &v in &mask {
        assert!(v >= 0.0 && v <= 1.0, "Mask value {v} outside [0,1]");
    }
}

#[test]
fn create_island_mask_monotonic_from_center() {
    let size = 64u32;
    let center = 32.0;
    let radius = 20.0;
    let mask = noise_gen::utils::create_island_mask(size, center, center, radius);

    // Walking from center outward along X, values should be non-increasing
    let z = center as u32;
    let center_val = mask[(z * size + center as u32) as usize];
    let edge_val = mask[(z * size + (center as u32 + radius as u32 - 1)) as usize];
    assert!(
        center_val >= edge_val,
        "Center {center_val} should be >= near-edge {edge_val}"
    );
}

// ============================================================================
// utils::generate_preview
// ============================================================================

#[test]
fn generate_preview_size_correct() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = noise_gen::utils::generate_preview(&noise, 16, 128.0);
    assert_eq!(preview.len(), 16 * 16);
}

#[test]
fn generate_preview_non_negative() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = noise_gen::utils::generate_preview(&noise, 32, 256.0);
    for &h in &preview {
        assert!(h >= 0.0, "Preview height should be non-negative, got {h}");
    }
}

// ============================================================================
// SimdHeightmapGenerator
// ============================================================================

#[test]
fn simd_heightmap_resolution_correct() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let hm = SimdHeightmapGenerator::generate_heightmap_simd(&noise, ChunkId::new(0, 0), 64.0, 16)
        .unwrap();
    assert_eq!(hm.resolution(), 16);
}

#[test]
fn simd_heightmap_matches_scalar() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let chunk = ChunkId::new(1, 1);

    let scalar = noise.generate_heightmap(chunk, 128.0, 32).unwrap();
    let simd = SimdHeightmapGenerator::generate_heightmap_simd(&noise, chunk, 128.0, 32).unwrap();

    let mut max_diff = 0.0f32;
    for x in 0..32 {
        for z in 0..32 {
            let diff = (scalar.get_height(x, z) - simd.get_height(x, z)).abs();
            max_diff = max_diff.max(diff);
        }
    }
    assert!(
        max_diff < 0.01,
        "SIMD vs scalar max diff {max_diff} should be < 0.01"
    );
}

#[test]
fn simd_heightmap_non_multiple_of_4_resolution() {
    // Resolution 17 → 4 full unrolled iterations + 1 scalar fallback
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let hm = SimdHeightmapGenerator::generate_heightmap_simd(&noise, ChunkId::new(0, 0), 64.0, 17)
        .unwrap();
    assert_eq!(hm.resolution(), 17);
}

#[test]
fn simd_heightmap_resolution_5() {
    // Small non-multiple-of-4
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let hm = SimdHeightmapGenerator::generate_heightmap_simd(&noise, ChunkId::new(0, 0), 64.0, 5)
        .unwrap();
    assert_eq!(hm.resolution(), 5);
}

#[test]
fn simd_heightmap_deterministic() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let chunk = ChunkId::new(2, 3);

    let a = SimdHeightmapGenerator::generate_heightmap_simd(&noise, chunk, 128.0, 16).unwrap();
    let b = SimdHeightmapGenerator::generate_heightmap_simd(&noise, chunk, 128.0, 16).unwrap();

    for x in 0..16 {
        for z in 0..16 {
            assert_eq!(
                a.get_height(x, z),
                b.get_height(x, z),
                "SIMD should be deterministic"
            );
        }
    }
}

#[test]
fn simd_preview_size_correct() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = SimdHeightmapGenerator::generate_preview_simd(&noise, 16, 128.0);
    assert_eq!(preview.len(), 16 * 16);
}

#[test]
fn simd_preview_matches_scalar() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);

    let scalar = noise_gen::utils::generate_preview(&noise, 24, 200.0);
    let simd = SimdHeightmapGenerator::generate_preview_simd(&noise, 24, 200.0);

    assert_eq!(scalar.len(), simd.len());
    let max_diff: f32 = scalar
        .iter()
        .zip(simd.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 0.01,
        "Preview SIMD vs scalar max diff {max_diff} should be < 0.01"
    );
}

#[test]
fn simd_preview_non_multiple_of_4() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = SimdHeightmapGenerator::generate_preview_simd(&noise, 13, 100.0);
    assert_eq!(preview.len(), 13 * 13);
}

#[test]
fn simd_heightmap_different_chunks_differ() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);

    let a = SimdHeightmapGenerator::generate_heightmap_simd(&noise, ChunkId::new(0, 0), 256.0, 8)
        .unwrap();
    let b = SimdHeightmapGenerator::generate_heightmap_simd(&noise, ChunkId::new(10, 10), 256.0, 8)
        .unwrap();

    let mut any_diff = false;
    for x in 0..8 {
        for z in 0..8 {
            if (a.get_height(x, z) - b.get_height(x, z)).abs() > 0.01 {
                any_diff = true;
            }
        }
    }
    assert!(any_diff, "Different chunks should have different heights");
}

// ============================================================================
// NoiseType enum variant matching (create_noise_fn)
// ============================================================================

#[test]
fn noise_type_fbm_produces_output() {
    let mut config = NoiseConfig::default();
    config.base_elevation.noise_type = NoiseType::Fbm;
    config.mountains.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    // Fbm should produce non-trivial output
    // Can't predict exact value, but should work
    assert!(h >= 0.0);
}

#[test]
fn noise_type_ridged_produces_output() {
    let mut config = NoiseConfig::default();
    config.base_elevation.noise_type = NoiseType::RidgedNoise;
    config.mountains.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    assert!(h >= 0.0);
}

#[test]
fn noise_type_billow_produces_output() {
    let mut config = NoiseConfig::default();
    config.base_elevation.noise_type = NoiseType::Billow;
    config.mountains.enabled = false;
    config.detail.enabled = false;

    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 100.0);
    assert!(h >= 0.0);
}

// ============================================================================
// Amplitude pinning: verify that changing amplitude changes output
// ============================================================================

#[test]
fn amplitude_affects_height_range() {
    let mut config_low = NoiseConfig::default();
    config_low.base_elevation.amplitude = 10.0;
    config_low.mountains.enabled = false;
    config_low.detail.enabled = false;

    let mut config_high = NoiseConfig::default();
    config_high.base_elevation.amplitude = 200.0;
    config_high.mountains.enabled = false;
    config_high.detail.enabled = false;

    let noise_low = TerrainNoise::new(&config_low, 42);
    let noise_high = TerrainNoise::new(&config_high, 42);

    let h_low = noise_low.sample_height(100.0, 100.0);
    let h_high = noise_high.sample_height(100.0, 100.0);

    // Higher amplitude should produce larger height (at same position)
    // The noise value is the same, just scaled differently
    if h_low > 0.0 {
        assert!(
            h_high > h_low,
            "Higher amplitude should produce larger height"
        );
    }
}

// ============================================================================
// Mountain layer abs() test
// ============================================================================

#[test]
fn mountain_layer_uses_abs() {
    // Mountains use noise_val.abs() * amplitude
    // This means negative noise values still contribute positive height
    let mut config = NoiseConfig::default();
    config.base_elevation.enabled = false;
    config.detail.enabled = false;
    // Mountains enabled by default

    let noise = TerrainNoise::new(&config, 42);

    // Sample many positions — mountain contribution should always be >= 0
    let mut any_positive = false;
    for x in (0..1000).step_by(50) {
        for z in (0..1000).step_by(50) {
            let h = noise.sample_height(x as f64, z as f64);
            assert!(h >= 0.0, "Mountain height at ({x},{z}) must be >= 0");
            if h > 0.0 {
                any_positive = true;
            }
        }
    }
    assert!(any_positive, "At least some mountain heights should be > 0");
}
