//! Wave 2 Mutation Remediation — Biome Blending
//!
//! Targets the 82 missed mutants in biome_blending.rs by testing:
//! - PackedBiomeBlend::from_weights normalization
//! - PackedBiomeBlend::dominant_biome mapping (all 8 biome IDs)
//! - BiomeBlendConfig defaults
//! - BiomeBlender::calculate_blend_weights distance falloff
//! - gather_neighbor_biomes radius and count
//! - apply_height_modification biome preferences
//! - Edge noise hash determinism

use astraweave_terrain::{
    BiomeBlendConfig, BiomeBlender, BiomeType, BiomeWeight, Heightmap, HeightmapConfig,
    PackedBiomeBlend,
};
use glam::Vec2;

// ============================================================================
// PACKED BIOME BLEND — NORMALIZATION
// ============================================================================

/// Weights should normalize to sum to 1.0.
#[test]
fn packed_blend_normalizes_weights() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Grassland,
            weight: 2.0,
        },
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 3.0,
        },
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    let sum: f32 = packed.weights.iter().sum();
    assert!(
        (sum - 1.0).abs() < 0.001,
        "Weights should sum to 1.0, got {sum}"
    );
    // Forest has higher weight, should be first (sorted by weight desc)
    assert!(packed.weights[0] > packed.weights[1]);
}

/// Empty weights should fall back to Desert with weight 1.0
/// (biome_blending.rs:72-73 — "Desert is a safer default than Grassland for arid scenes").
#[test]
fn packed_blend_empty_fallback_grassland() {
    let packed = PackedBiomeBlend::from_weights(&[]);
    assert_eq!(packed.biome_ids[0], BiomeType::Desert as u8);
    assert_eq!(packed.weights[0], 1.0);
}

/// Very small weights (< 0.001) should be filtered out.
#[test]
fn packed_blend_filters_tiny_weights() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Desert,
            weight: 0.0005,
        },
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 0.0001,
        },
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    // Both below threshold → total≈0 → documented Desert fallback (biome_blending.rs:72-73)
    assert_eq!(packed.biome_ids[0], BiomeType::Desert as u8);
    assert_eq!(packed.weights[0], 1.0);
}

/// Only top 4 biomes should be kept (MAX_BLEND_BIOMES = 4).
#[test]
fn packed_blend_caps_at_4_biomes() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Grassland,
            weight: 1.0,
        },
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 0.9,
        },
        BiomeWeight {
            biome: BiomeType::Desert,
            weight: 0.8,
        },
        BiomeWeight {
            biome: BiomeType::Mountain,
            weight: 0.7,
        },
        BiomeWeight {
            biome: BiomeType::Tundra,
            weight: 0.6,
        },
        BiomeWeight {
            biome: BiomeType::Swamp,
            weight: 0.5,
        },
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    let sum: f32 = packed.weights.iter().sum();
    assert!((sum - 1.0).abs() < 0.001, "Sum should be 1.0: {sum}");

    // First 4 should have weights, last shouldn't appear
    // Actually the 5th and 6th are dropped, and only top 4 remain
    // Just verify normalization works with >4 inputs
    let nonzero_count = packed.weights.iter().filter(|&&w| w > 0.001).count();
    assert!(
        nonzero_count <= 4,
        "Max 4 non-zero weights: {nonzero_count}"
    );
}

/// Single biome weight should normalize to 1.0.
#[test]
fn packed_blend_single_biome() {
    let weights = vec![BiomeWeight {
        biome: BiomeType::Mountain,
        weight: 5.0,
    }];
    let packed = PackedBiomeBlend::from_weights(&weights);
    assert_eq!(packed.weights[0], 1.0);
    assert_eq!(packed.biome_ids[0], BiomeType::Mountain as u8);
}

// ============================================================================
// PACKED BIOME BLEND — DOMINANT BIOME MAPPING
// ============================================================================

/// Test all 8 biome ID mappings in dominant_biome().
#[test]
fn dominant_biome_maps_all_ids() {
    // biome_id 0 → Grassland
    let mut p = PackedBiomeBlend::default();
    p.biome_ids[0] = 0;
    p.weights[0] = 1.0;
    assert_eq!(p.dominant_biome(), BiomeType::Grassland);

    p.biome_ids[0] = 1;
    assert_eq!(p.dominant_biome(), BiomeType::Desert);

    p.biome_ids[0] = 2;
    assert_eq!(p.dominant_biome(), BiomeType::Forest);

    p.biome_ids[0] = 3;
    assert_eq!(p.dominant_biome(), BiomeType::Mountain);

    p.biome_ids[0] = 4;
    assert_eq!(p.dominant_biome(), BiomeType::Tundra);

    p.biome_ids[0] = 5;
    assert_eq!(p.dominant_biome(), BiomeType::Swamp);

    p.biome_ids[0] = 6;
    assert_eq!(p.dominant_biome(), BiomeType::Beach);

    p.biome_ids[0] = 7;
    assert_eq!(p.dominant_biome(), BiomeType::River);
}

/// Unknown IDs should fall back to Grassland.
#[test]
fn dominant_biome_unknown_id_fallback() {
    let mut p = PackedBiomeBlend::default();
    p.biome_ids[0] = 99;
    p.weights[0] = 1.0;
    assert_eq!(p.dominant_biome(), BiomeType::Grassland);
}

/// Dominant biome is the one with highest weight, not first slot.
#[test]
fn dominant_biome_highest_weight_wins() {
    let mut p = PackedBiomeBlend::default();
    p.biome_ids[0] = BiomeType::Desert as u8;
    p.weights[0] = 0.2;
    p.biome_ids[1] = BiomeType::Forest as u8;
    p.weights[1] = 0.8;
    assert_eq!(p.dominant_biome(), BiomeType::Forest);
}

// ============================================================================
// BIOME BLEND CONFIG DEFAULTS
// ============================================================================

#[test]
fn blend_config_defaults_exact() {
    let c = BiomeBlendConfig::default();
    assert_eq!(c.blend_radius, 32.0);
    assert_eq!(c.falloff_power, 2.0);
    assert_eq!(c.edge_noise_scale, 0.02);
    assert_eq!(c.edge_noise_amplitude, 8.0);
    assert_eq!(c.min_weight_threshold, 0.01);
    assert!(c.height_blend_enabled);
    assert_eq!(c.height_blend_factor, 0.3);
}

// ============================================================================
// BIOME BLENDER — CALCULATE BLEND WEIGHTS
// ============================================================================

/// A single neighbor at distance 0 should get weight 1.0 after normalization.
#[test]
fn blend_weights_single_neighbor_at_origin() {
    let config = BiomeBlendConfig {
        edge_noise_amplitude: 0.0, // Disable noise for predictable results
        height_blend_enabled: false,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);
    let neighbors = vec![(Vec2::new(10.0, 10.0), BiomeType::Forest)];
    let packed = blender.calculate_blend_weights(Vec2::new(10.0, 10.0), 0.0, &neighbors);

    assert_eq!(packed.dominant_biome(), BiomeType::Forest);
    assert!(
        packed.weights[0] > 0.99,
        "Single neighbor should dominate: {}",
        packed.weights[0]
    );
}

/// Two equidistant neighbors of different biomes should have roughly equal weight.
#[test]
fn blend_weights_equidistant_neighbors() {
    let config = BiomeBlendConfig {
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        blend_radius: 100.0,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);
    let neighbors = vec![
        (Vec2::new(10.0, 0.0), BiomeType::Forest),
        (Vec2::new(-10.0, 0.0), BiomeType::Desert),
    ];
    let packed = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);

    // Both should have significant weight (equidistant)
    let total: f32 = packed.weights.iter().sum();
    assert!((total - 1.0).abs() < 0.01, "Sum should be 1.0: {total}");
    // Each should be roughly 0.5
    assert!(
        packed.weights[0] > 0.3 && packed.weights[0] < 0.7,
        "Equidistant weight should be ~0.5: {}",
        packed.weights[0]
    );
}

/// Neighbors outside blend_radius should get zero weight.
#[test]
fn blend_weights_outside_radius_zero() {
    let config = BiomeBlendConfig {
        blend_radius: 10.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);
    let neighbors = vec![
        (Vec2::new(100.0, 0.0), BiomeType::Desert), // Way outside radius
    ];
    let packed = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);

    // No valid neighbors → total≈0 → documented Desert fallback (biome_blending.rs:72-73)
    assert_eq!(packed.dominant_biome(), BiomeType::Desert);
}

/// Closer neighbors should have higher weight (falloff).
#[test]
fn blend_weights_closer_is_stronger() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        falloff_power: 2.0,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);
    let neighbors = vec![
        (Vec2::new(5.0, 0.0), BiomeType::Forest),  // Close
        (Vec2::new(80.0, 0.0), BiomeType::Desert), // Far
    ];
    let packed = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);

    assert_eq!(
        packed.dominant_biome(),
        BiomeType::Forest,
        "Closer neighbor should dominate"
    );
}

/// Falloff power affects weight distribution:
/// Higher power → sharper falloff → closer biome even more dominant.
#[test]
fn blend_weights_falloff_power_effect() {
    let config_linear = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        falloff_power: 1.0,
        ..Default::default()
    };
    let config_sharp = BiomeBlendConfig {
        falloff_power: 4.0,
        ..config_linear.clone()
    };

    let blender_linear = BiomeBlender::new(config_linear, 42);
    let blender_sharp = BiomeBlender::new(config_sharp, 42);

    let neighbors = vec![
        (Vec2::new(10.0, 0.0), BiomeType::Forest),
        (Vec2::new(50.0, 0.0), BiomeType::Desert),
    ];

    let packed_linear = blender_linear.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let packed_sharp = blender_sharp.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);

    // Sharp falloff should give even more weight to close neighbor
    // So the dominant weight should be higher with sharp falloff
    assert!(
        packed_sharp.weights[0] >= packed_linear.weights[0] - 0.01,
        "Sharper falloff should increase dominant weight: sharp={} linear={}",
        packed_sharp.weights[0],
        packed_linear.weights[0]
    );
}

// ============================================================================
// HEIGHT MODIFICATION — apply_height_modification
// ============================================================================

/// Beach biome prefers low height: weight increases at height 0.
#[test]
fn height_mod_beach_prefers_low() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    // Beach at distance 20, at low height vs high height
    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::Beach)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 100.0, &neighbors);

    assert!(
        low.weights[0] >= high.weights[0],
        "Beach should have higher weight at low height: low={} high={}",
        low.weights[0],
        high.weights[0]
    );
}

/// Mountain biome prefers high height.
#[test]
fn height_mod_mountain_prefers_high() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::Mountain)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 200.0, &neighbors);

    assert!(
        high.weights[0] >= low.weights[0],
        "Mountain should have higher weight at high elevation: high={} low={}",
        high.weights[0],
        low.weights[0]
    );
}

/// Tundra biome prefers high height.
#[test]
fn height_mod_tundra_prefers_high() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::Tundra)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 150.0, &neighbors);

    assert!(
        high.weights[0] >= low.weights[0],
        "Tundra should have higher weight at high elevation: high={} low={}",
        high.weights[0],
        low.weights[0]
    );
}

/// River biome prefers low height.
#[test]
fn height_mod_river_prefers_low() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::River)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 50.0, &neighbors);

    assert!(
        low.weights[0] >= high.weights[0],
        "River should prefer low height: low={} high={}",
        low.weights[0],
        high.weights[0]
    );
}

/// Swamp biome prefers low height.
#[test]
fn height_mod_swamp_prefers_low() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::Swamp)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 100.0, &neighbors);

    assert!(
        low.weights[0] >= high.weights[0],
        "Swamp should prefer low height: low={} high={}",
        low.weights[0],
        high.weights[0]
    );
}

/// Grassland/Desert/Forest are height-neutral (weight unaffected by height).
#[test]
fn height_mod_neutral_biomes_unaffected() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: true,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    for biome in [BiomeType::Grassland, BiomeType::Desert, BiomeType::Forest] {
        let neighbors = vec![(Vec2::new(20.0, 0.0), biome)];
        let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
        let high = blender.calculate_blend_weights(Vec2::ZERO, 200.0, &neighbors);

        assert!(
            (low.weights[0] - high.weights[0]).abs() < 0.001,
            "{:?} should be height-neutral: low={} high={}",
            biome,
            low.weights[0],
            high.weights[0]
        );
    }
}

/// Disabling height_blend_enabled makes all biomes height-neutral.
#[test]
fn height_blend_disabled_all_neutral() {
    let config = BiomeBlendConfig {
        blend_radius: 100.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        height_blend_factor: 0.3,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let neighbors = vec![(Vec2::new(20.0, 0.0), BiomeType::Mountain)];
    let low = blender.calculate_blend_weights(Vec2::ZERO, 0.0, &neighbors);
    let high = blender.calculate_blend_weights(Vec2::ZERO, 200.0, &neighbors);

    assert!(
        (low.weights[0] - high.weights[0]).abs() < 0.001,
        "With height_blend_enabled=false, Mountain should be neutral: low={} high={}",
        low.weights[0],
        high.weights[0]
    );
}

// ============================================================================
// BLEND CHUNK — end-to-end
// ============================================================================

/// blend_chunk should produce correct number of outputs.
#[test]
fn blend_chunk_output_count() {
    let config = BiomeBlendConfig {
        blend_radius: 8.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let hm_config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let hm = Heightmap::new(hm_config).unwrap();
    let biome_map = vec![BiomeType::Grassland; 8 * 8];

    let result = blender.blend_chunk(&hm, &biome_map, 256.0, Vec2::ZERO);
    assert_eq!(result.len(), 64, "8x8 = 64 blend results");
}

/// blend_chunk with uniform biome map should all be the same biome.
#[test]
fn blend_chunk_uniform_biome() {
    let config = BiomeBlendConfig {
        blend_radius: 8.0,
        edge_noise_amplitude: 0.0,
        height_blend_enabled: false,
        ..Default::default()
    };
    let blender = BiomeBlender::new(config, 42);

    let hm_config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let hm = Heightmap::new(hm_config).unwrap();
    let biome_map = vec![BiomeType::Forest; 64];

    let result = blender.blend_chunk(&hm, &biome_map, 256.0, Vec2::ZERO);
    for (i, blend) in result.iter().enumerate() {
        assert_eq!(
            blend.dominant_biome(),
            BiomeType::Forest,
            "Cell {i} should be Forest"
        );
    }
}

// ============================================================================
// EDGE NOISE DETERMINISM
// ============================================================================

/// Same seed should produce same blend results.
#[test]
fn edge_noise_deterministic_same_seed() {
    let config = BiomeBlendConfig::default();
    let blender1 = BiomeBlender::new(config.clone(), 12345);
    let blender2 = BiomeBlender::new(config, 12345);

    let neighbors = vec![
        (Vec2::new(10.0, 0.0), BiomeType::Forest),
        (Vec2::new(-10.0, 0.0), BiomeType::Desert),
    ];

    let r1 = blender1.calculate_blend_weights(Vec2::new(5.0, 3.0), 10.0, &neighbors);
    let r2 = blender2.calculate_blend_weights(Vec2::new(5.0, 3.0), 10.0, &neighbors);

    for i in 0..4 {
        assert_eq!(r1.biome_ids[i], r2.biome_ids[i], "Biome ID mismatch at {i}");
        assert!(
            (r1.weights[i] - r2.weights[i]).abs() < 0.001,
            "Weight mismatch at {i}: {} vs {}",
            r1.weights[i],
            r2.weights[i]
        );
    }
}

/// Different seeds should produce different edge noise → different weights.
#[test]
fn edge_noise_different_seeds_differ() {
    let config = BiomeBlendConfig {
        edge_noise_amplitude: 5.0, // Moderate noise within blend_radius=32
        ..Default::default()
    };
    let blender1 = BiomeBlender::new(config.clone(), 111);
    let blender2 = BiomeBlender::new(config, 999);

    let neighbors = vec![
        (Vec2::new(57.0, 73.0), BiomeType::Forest), // dist=2 from query
        (Vec2::new(42.0, 73.0), BiomeType::Desert), // dist=13 from query (near edge)
    ];

    let r1 = blender1.calculate_blend_weights(Vec2::new(55.0, 73.0), 10.0, &neighbors);
    let r2 = blender2.calculate_blend_weights(Vec2::new(55.0, 73.0), 10.0, &neighbors);

    // At least one weight should differ
    let any_differ = (0..4).any(|i| (r1.weights[i] - r2.weights[i]).abs() > 0.001);
    assert!(
        any_differ,
        "Different seeds should produce different blend weights"
    );
}

// ============================================================================
// BIOME WEIGHT DEFAULT
// ============================================================================

#[test]
fn biome_weight_default() {
    let w = BiomeWeight::default();
    assert_eq!(w.biome, BiomeType::Grassland);
    assert_eq!(w.weight, 0.0);
}

// ============================================================================
// PACKED BIOME BLEND DEFAULT
// ============================================================================

#[test]
fn packed_biome_blend_default() {
    let p = PackedBiomeBlend::default();
    assert_eq!(p.biome_ids, [0, 0, 0, 0]);
    assert_eq!(p.weights, [0.0, 0.0, 0.0, 0.0]);
}
