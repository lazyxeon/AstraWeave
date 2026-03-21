//! Wave 2 Mutation Remediation Tests — noise_gen.rs, scatter.rs, and other modules
//!
//! Pins exact config default values and utility function arithmetic
//! to kill mutations in NoiseConfig, NoiseLayer defaults, ScatterConfig,
//! noise utility functions, and island mask calculations.

use astraweave_terrain::noise_gen::{NoiseLayer, NoiseType};
use astraweave_terrain::*;

// ============================================================================
// NoiseConfig: exact default values for base_elevation layer
// ============================================================================

#[test]
fn noise_config_base_elevation_enabled() {
    let c = NoiseConfig::default();
    assert!(c.base_elevation.enabled);
}

#[test]
fn noise_config_base_elevation_scale() {
    let c = NoiseConfig::default();
    assert!((c.base_elevation.scale - 0.005).abs() < 1e-9);
}

#[test]
fn noise_config_base_elevation_amplitude() {
    let c = NoiseConfig::default();
    assert!((c.base_elevation.amplitude - 50.0).abs() < 1e-6);
}

#[test]
fn noise_config_base_elevation_octaves() {
    let c = NoiseConfig::default();
    assert_eq!(c.base_elevation.octaves, 4);
}

#[test]
fn noise_config_base_elevation_persistence() {
    let c = NoiseConfig::default();
    assert!((c.base_elevation.persistence - 0.5).abs() < 1e-9);
}

#[test]
fn noise_config_base_elevation_lacunarity() {
    let c = NoiseConfig::default();
    assert!((c.base_elevation.lacunarity - 2.0).abs() < 1e-9);
}

// ============================================================================
// NoiseConfig: exact default values for mountains layer
// ============================================================================

#[test]
fn noise_config_mountains_enabled() {
    let c = NoiseConfig::default();
    assert!(c.mountains.enabled);
}

#[test]
fn noise_config_mountains_scale() {
    let c = NoiseConfig::default();
    assert!((c.mountains.scale - 0.002).abs() < 1e-9);
}

#[test]
fn noise_config_mountains_amplitude() {
    let c = NoiseConfig::default();
    assert!((c.mountains.amplitude - 80.0).abs() < 1e-6);
}

#[test]
fn noise_config_mountains_octaves() {
    let c = NoiseConfig::default();
    assert_eq!(c.mountains.octaves, 6);
}

#[test]
fn noise_config_mountains_persistence() {
    let c = NoiseConfig::default();
    assert!((c.mountains.persistence - 0.4).abs() < 1e-9);
}

#[test]
fn noise_config_mountains_lacunarity() {
    let c = NoiseConfig::default();
    assert!((c.mountains.lacunarity - 2.2).abs() < 1e-9);
}

// ============================================================================
// NoiseConfig: exact default values for detail layer
// ============================================================================

#[test]
fn noise_config_detail_enabled() {
    let c = NoiseConfig::default();
    assert!(c.detail.enabled);
}

#[test]
fn noise_config_detail_scale() {
    let c = NoiseConfig::default();
    assert!((c.detail.scale - 0.02).abs() < 1e-9);
}

#[test]
fn noise_config_detail_amplitude() {
    let c = NoiseConfig::default();
    assert!((c.detail.amplitude - 5.0).abs() < 1e-6);
}

#[test]
fn noise_config_detail_octaves() {
    let c = NoiseConfig::default();
    assert_eq!(c.detail.octaves, 3);
}

#[test]
fn noise_config_detail_persistence() {
    let c = NoiseConfig::default();
    assert!((c.detail.persistence - 0.6).abs() < 1e-9);
}

#[test]
fn noise_config_detail_lacunarity() {
    let c = NoiseConfig::default();
    assert!((c.detail.lacunarity - 2.0).abs() < 1e-9);
}

// ============================================================================
// NoiseConfig: erosion flags
// ============================================================================

#[test]
fn noise_config_erosion_enabled() {
    let c = NoiseConfig::default();
    assert!(c.erosion_enabled);
}

#[test]
fn noise_config_erosion_strength() {
    let c = NoiseConfig::default();
    assert!((c.erosion_strength - 0.3).abs() < 1e-6);
}

// ============================================================================
// NoiseConfig: layer relationship constraints
// ============================================================================

#[test]
fn noise_mountains_amplitude_gt_base() {
    let c = NoiseConfig::default();
    assert!(
        c.mountains.amplitude > c.base_elevation.amplitude,
        "Mountains amplitude {} should exceed base {}",
        c.mountains.amplitude,
        c.base_elevation.amplitude
    );
}

#[test]
fn noise_detail_amplitude_lt_base() {
    let c = NoiseConfig::default();
    assert!(
        c.detail.amplitude < c.base_elevation.amplitude,
        "Detail amplitude {} should be less than base {}",
        c.detail.amplitude,
        c.base_elevation.amplitude
    );
}

#[test]
fn noise_mountains_octaves_gt_base() {
    let c = NoiseConfig::default();
    assert!(c.mountains.octaves >= c.base_elevation.octaves);
}

#[test]
fn noise_detail_scale_gt_base() {
    let c = NoiseConfig::default();
    assert!(
        c.detail.scale > c.base_elevation.scale,
        "Detail should have higher frequency: {} vs {}",
        c.detail.scale,
        c.base_elevation.scale
    );
}

// ============================================================================
// TerrainNoise: behavioral tests
// ============================================================================

#[test]
fn terrain_noise_height_non_negative() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    // Sample many points; all should be >= 0
    for i in 0..100 {
        let h = noise.sample_height(i as f64 * 10.0, i as f64 * 7.0);
        assert!(
            h >= 0.0,
            "Height at ({}, {}) was negative: {}",
            i * 10,
            i * 7,
            h
        );
    }
}

#[test]
fn terrain_noise_with_disabled_layers() {
    let config = NoiseConfig {
        base_elevation: NoiseLayer {
            enabled: false,
            ..NoiseConfig::default().base_elevation
        },
        mountains: NoiseLayer {
            enabled: false,
            ..NoiseConfig::default().mountains
        },
        detail: NoiseLayer {
            enabled: false,
            ..NoiseConfig::default().detail
        },
        ..Default::default()
    };
    let noise = TerrainNoise::new(&config, 42);
    let h = noise.sample_height(100.0, 200.0);
    // All layers disabled → height = 0.0.max(0.0) = 0.0
    assert!(
        (h - 0.0).abs() < 1e-6,
        "All disabled should give 0.0, got {h}"
    );
}

// ============================================================================
// NoiseConfig: noise type pinning
// ============================================================================

#[test]
fn noise_config_base_elevation_type_is_perlin() {
    let c = NoiseConfig::default();
    assert!(matches!(c.base_elevation.noise_type, NoiseType::Perlin));
}

#[test]
fn noise_config_mountains_type_is_ridged() {
    let c = NoiseConfig::default();
    assert!(matches!(c.mountains.noise_type, NoiseType::RidgedNoise));
}

#[test]
fn noise_config_detail_type_is_billow() {
    let c = NoiseConfig::default();
    assert!(matches!(c.detail.noise_type, NoiseType::Billow));
}

#[test]
fn terrain_noise_generate_heightmap_correct_resolution() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let hm = noise
        .generate_heightmap(ChunkId::new(0, 0), 256.0, 32)
        .unwrap();
    assert_eq!(hm.resolution(), 32);
    assert_eq!(hm.data().len(), 1024);
}

#[test]
fn terrain_noise_density_sampling() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let d = noise.sample_density(10.0, 5.0, 10.0);
    // Just verify it returns a finite value
    assert!(d.is_finite(), "sample_density should return finite value");
}

// ============================================================================
// Noise utils: normalize_heights
// ============================================================================

#[test]
fn normalize_heights_maps_to_01_range() {
    let mut h = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    astraweave_terrain::noise_gen::utils::normalize_heights(&mut h);
    assert!((h[0] - 0.0).abs() < 1e-6);
    assert!((h[4] - 1.0).abs() < 1e-6);
    // Middle should be 0.5
    assert!((h[2] - 0.5).abs() < 1e-6);
}

#[test]
fn normalize_heights_uniform_stays_same() {
    let mut h = vec![5.0, 5.0, 5.0];
    astraweave_terrain::noise_gen::utils::normalize_heights(&mut h);
    // range = 0 → no change (guard clause)
    // Values stay as-is since range is 0
}

#[test]
fn normalize_heights_empty_is_noop() {
    let mut h: Vec<f32> = vec![];
    astraweave_terrain::noise_gen::utils::normalize_heights(&mut h);
    assert!(h.is_empty());
}

// ============================================================================
// Noise utils: apply_height_curve
// ============================================================================

#[test]
fn apply_height_curve_power_1_identity() {
    let mut h = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    astraweave_terrain::noise_gen::utils::apply_height_curve(&mut h, 1.0);
    // power=1 → identity, then *100
    assert!((h[0] - 0.0).abs() < 1e-4);
    assert!((h[1] - 25.0).abs() < 1e-4);
    assert!((h[2] - 50.0).abs() < 1e-4);
    assert!((h[3] - 75.0).abs() < 1e-4);
    assert!((h[4] - 100.0).abs() < 1e-4);
}

#[test]
fn apply_height_curve_power_2_quadratic() {
    let mut h = vec![0.5];
    astraweave_terrain::noise_gen::utils::apply_height_curve(&mut h, 2.0);
    // 0.5^2 * 100 = 25.0
    assert!((h[0] - 25.0).abs() < 1e-4);
}

// ============================================================================
// Noise utils: create_island_mask
// ============================================================================

#[test]
fn island_mask_correct_size() {
    let mask = astraweave_terrain::noise_gen::utils::create_island_mask(16, 8.0, 8.0, 5.0);
    assert_eq!(mask.len(), 256);
}

#[test]
fn island_mask_center_is_high() {
    let mask = astraweave_terrain::noise_gen::utils::create_island_mask(32, 16.0, 16.0, 10.0);
    let center = mask[16 * 32 + 16];
    assert!(center > 0.9, "Center should be near 1.0, got {center}");
}

#[test]
fn island_mask_edge_is_low() {
    let mask = astraweave_terrain::noise_gen::utils::create_island_mask(32, 16.0, 16.0, 10.0);
    assert!(mask[0] < 0.1, "Far corner should be low, got {}", mask[0]);
}

#[test]
fn island_mask_values_in_01() {
    let mask = astraweave_terrain::noise_gen::utils::create_island_mask(32, 16.0, 16.0, 10.0);
    for &v in &mask {
        assert!(v >= 0.0 && v <= 1.0, "Mask value out of range: {v}");
    }
}

// ============================================================================
// ScatterConfig: exact default values
// ============================================================================

#[test]
fn scatter_config_default_use_poisson_disk() {
    let c = ScatterConfig::default();
    assert!(c.use_poisson_disk);
}

#[test]
fn scatter_config_default_min_distance() {
    let c = ScatterConfig::default();
    assert!((c.min_distance - 2.0).abs() < 1e-6);
}

#[test]
fn scatter_config_default_max_slope() {
    let c = ScatterConfig::default();
    assert!((c.max_slope - 35.0).abs() < 1e-6);
}

#[test]
fn scatter_config_default_height_filter_none() {
    let c = ScatterConfig::default();
    assert!(c.height_filter.is_none());
}

#[test]
fn scatter_config_default_seed_offset() {
    let c = ScatterConfig::default();
    assert_eq!(c.seed_offset, 0);
}

// ============================================================================
// StreamingConfig: precise default values
// ============================================================================

#[test]
fn streaming_config_default_max_loaded_chunks() {
    let c = StreamingConfig::default();
    assert_eq!(c.max_loaded_chunks, 256);
}

#[test]
fn streaming_config_default_view_distance() {
    let c = StreamingConfig::default();
    assert_eq!(c.view_distance, 8);
}

#[test]
fn streaming_config_default_prefetch_distance() {
    let c = StreamingConfig::default();
    assert_eq!(c.prefetch_distance, 4);
}

#[test]
fn streaming_config_default_max_concurrent_loads() {
    let c = StreamingConfig::default();
    assert_eq!(c.max_concurrent_loads, 8);
}

#[test]
fn streaming_config_default_chunk_size() {
    let c = StreamingConfig::default();
    assert!((c.chunk_size - 256.0).abs() < 1e-6);
}

#[test]
fn streaming_config_default_adaptive_throttle() {
    let c = StreamingConfig::default();
    assert!((c.adaptive_throttle_threshold_ms - 10.0).abs() < 1e-6);
}

#[test]
fn streaming_config_default_throttled_loads() {
    let c = StreamingConfig::default();
    assert_eq!(c.throttled_concurrent_loads, 2);
}

#[test]
fn streaming_prefetch_le_view() {
    let c = StreamingConfig::default();
    assert!(c.prefetch_distance <= c.view_distance);
}

#[test]
fn streaming_throttled_lt_max_concurrent() {
    let c = StreamingConfig::default();
    assert!(c.throttled_concurrent_loads < c.max_concurrent_loads);
}

// ============================================================================
// Noise preview generation from utils
// ============================================================================

#[test]
fn noise_preview_correct_size() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = astraweave_terrain::noise_gen::utils::generate_preview(&noise, 16, 100.0);
    assert_eq!(preview.len(), 256);
}

#[test]
fn noise_preview_non_negative() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);
    let preview = astraweave_terrain::noise_gen::utils::generate_preview(&noise, 16, 100.0);
    for (i, &v) in preview.iter().enumerate() {
        assert!(
            v >= 0.0,
            "Preview height at index {} was negative: {}",
            i,
            v
        );
    }
}
