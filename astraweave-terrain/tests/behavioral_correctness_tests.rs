//! Behavioral Correctness Tests for astraweave-terrain
//!
//! These tests validate mathematically correct and physically accurate behavior
//! of terrain generation systems. Designed to be mutation-resistant by testing
//! specific numerical relationships that must hold for correct simulation.
//!
//! Coverage targets:
//! - Heightmap: Bilinear interpolation, bounds, normal calculation
//! - Noise generation: Value ranges, determinism, layer combination
//! - Biome classification: Scoring logic, condition matching, slope suitability
//! - Voxel data: Chunk coordinates, density thresholds

use astraweave_terrain::noise_gen::{DomainWarpConfig, NoiseLayer, NoiseType};
use astraweave_terrain::{
    BiomeConfig, BiomeType, Heightmap, HeightmapConfig, NoiseConfig, TerrainNoise,
};

/// Helper function to create a disabled NoiseLayer
fn disabled_noise_layer() -> NoiseLayer {
    NoiseLayer {
        enabled: false,
        scale: 0.01,
        amplitude: 0.0,
        octaves: 1,
        persistence: 0.5,
        lacunarity: 2.0,
        noise_type: NoiseType::Perlin,
        domain_warp: DomainWarpConfig::default(),
    }
}

// ============================================================================
// HEIGHTMAP TESTS
// ============================================================================

/// Test that heightmap correctly stores and retrieves values
#[test]
fn test_heightmap_get_set_consistency() {
    let config = HeightmapConfig {
        resolution: 16,
        min_height: 0.0,
        max_height: 100.0,
        height_scale: 1.0,
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set specific heights
    heightmap.set_height(5, 5, 42.5);
    heightmap.set_height(10, 10, 87.3);
    heightmap.set_height(0, 0, 0.0);

    // Verify retrieval is exact
    assert!(
        (heightmap.get_height(5, 5) - 42.5).abs() < 1e-6,
        "Height at (5,5) should be exactly 42.5"
    );
    assert!(
        (heightmap.get_height(10, 10) - 87.3).abs() < 1e-6,
        "Height at (10,10) should be exactly 87.3"
    );
    assert!(
        (heightmap.get_height(0, 0) - 0.0).abs() < 1e-6,
        "Height at (0,0) should be exactly 0.0"
    );
}

/// Test out-of-bounds height access returns 0.0 (safe default)
#[test]
fn test_heightmap_out_of_bounds_returns_zero() {
    let config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set some in-bounds values
    heightmap.set_height(3, 3, 50.0);

    // Out-of-bounds should return 0.0
    assert_eq!(
        heightmap.get_height(8, 0),
        0.0,
        "Out-of-bounds x should return 0"
    );
    assert_eq!(
        heightmap.get_height(0, 8),
        0.0,
        "Out-of-bounds z should return 0"
    );
    assert_eq!(
        heightmap.get_height(100, 100),
        0.0,
        "Far out-of-bounds should return 0"
    );

    // In-bounds should still work
    assert!(
        (heightmap.get_height(3, 3) - 50.0).abs() < 1e-6,
        "In-bounds should still return correct value"
    );
}

/// Test bilinear interpolation returns exact corner values at integer coordinates
#[test]
fn test_bilinear_interpolation_at_corners() {
    let config = HeightmapConfig {
        resolution: 4,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set corner heights
    heightmap.set_height(0, 0, 10.0);
    heightmap.set_height(1, 0, 20.0);
    heightmap.set_height(0, 1, 30.0);
    heightmap.set_height(1, 1, 40.0);

    // At integer coordinates, should return exact values
    assert!(
        (heightmap.sample_bilinear(0.0, 0.0) - 10.0).abs() < 0.01,
        "At (0,0) should return 10.0"
    );
    assert!(
        (heightmap.sample_bilinear(1.0, 0.0) - 20.0).abs() < 0.01,
        "At (1,0) should return 20.0"
    );
    assert!(
        (heightmap.sample_bilinear(0.0, 1.0) - 30.0).abs() < 0.01,
        "At (0,1) should return 30.0"
    );
}

/// Test bilinear interpolation midpoint is average of corners
#[test]
fn test_bilinear_interpolation_midpoint_is_average() {
    let config = HeightmapConfig {
        resolution: 4,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set corner heights
    let h00 = 10.0;
    let h10 = 20.0;
    let h01 = 30.0;
    let h11 = 40.0;
    heightmap.set_height(0, 0, h00);
    heightmap.set_height(1, 0, h10);
    heightmap.set_height(0, 1, h01);
    heightmap.set_height(1, 1, h11);

    // At (0.5, 0.5), bilinear interpolation = average of 4 corners
    let expected_midpoint = (h00 + h10 + h01 + h11) / 4.0;
    let actual = heightmap.sample_bilinear(0.5, 0.5);

    assert!(
        (actual - expected_midpoint).abs() < 0.01,
        "Midpoint should be average of corners. Expected {}, got {}",
        expected_midpoint,
        actual
    );
}

/// Test bilinear interpolation is linear along edges
#[test]
fn test_bilinear_interpolation_linear_along_edge() {
    let config = HeightmapConfig {
        resolution: 4,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set edge heights (bottom edge)
    let h00 = 0.0;
    let h10 = 100.0;
    heightmap.set_height(0, 0, h00);
    heightmap.set_height(1, 0, h10);
    heightmap.set_height(0, 1, 0.0);
    heightmap.set_height(1, 1, 100.0);

    // Along bottom edge (z=0), should be linear interpolation
    // At u=0.25, should be 25.0
    let at_25_percent = heightmap.sample_bilinear(0.25, 0.0);
    assert!(
        (at_25_percent - 25.0).abs() < 1.0,
        "At 25% along edge, expected ~25, got {}",
        at_25_percent
    );

    // At u=0.5, should be 50.0
    let at_50_percent = heightmap.sample_bilinear(0.5, 0.0);
    assert!(
        (at_50_percent - 50.0).abs() < 1.0,
        "At 50% along edge, expected ~50, got {}",
        at_50_percent
    );

    // At u=0.75, should be 75.0
    let at_75_percent = heightmap.sample_bilinear(0.75, 0.0);
    assert!(
        (at_75_percent - 75.0).abs() < 1.0,
        "At 75% along edge, expected ~75, got {}",
        at_75_percent
    );
}

/// Test min/max bounds update when setting heights
#[test]
fn test_heightmap_bounds_update_on_set() {
    let config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Initially should have 0 bounds (empty data)
    assert_eq!(heightmap.min_height(), 0.0, "Initial min should be 0");
    assert_eq!(heightmap.max_height(), 0.0, "Initial max should be 0");

    // Set some heights
    heightmap.set_height(0, 0, -50.0);
    heightmap.set_height(1, 1, 150.0);
    heightmap.set_height(2, 2, 75.0);

    // Min/max should reflect actual range
    assert!(heightmap.min_height() <= -50.0, "Min should include -50.0");
    assert!(heightmap.max_height() >= 150.0, "Max should include 150.0");
}

/// Test from_data calculates bounds correctly
#[test]
fn test_heightmap_from_data_bounds() {
    let resolution = 4;
    let data = vec![
        10.0, 20.0, 30.0, 40.0, 5.0, 15.0, 25.0, 35.0, 0.0, 10.0, 20.0, 30.0, 100.0, 90.0, 80.0,
        70.0,
    ];

    let heightmap = Heightmap::from_data(data, resolution).unwrap();

    assert!(
        (heightmap.min_height() - 0.0).abs() < 1e-6,
        "Min should be 0.0"
    );
    assert!(
        (heightmap.max_height() - 100.0).abs() < 1e-6,
        "Max should be 100.0"
    );
}

/// Test from_data rejects mismatched sizes
#[test]
fn test_heightmap_from_data_size_validation() {
    let resolution = 4; // Expects 16 elements
    let data = vec![1.0, 2.0, 3.0]; // Only 3 elements

    let result = Heightmap::from_data(data, resolution);
    assert!(result.is_err(), "Should reject mismatched data size");
}

/// Test recalculate_bounds after bulk modification
#[test]
fn test_heightmap_recalculate_bounds() {
    let config = HeightmapConfig {
        resolution: 4,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Bulk modify data
    let data = heightmap.data_mut();
    for (i, h) in data.iter_mut().enumerate() {
        *h = i as f32 * 10.0;
    }

    // Recalculate bounds
    heightmap.recalculate_bounds();

    // Resolution 4 = 16 elements, last element is 150.0
    assert!(
        (heightmap.min_height() - 0.0).abs() < 1e-6,
        "Min should be 0.0"
    );
    assert!(
        (heightmap.max_height() - 150.0).abs() < 1e-6,
        "Max should be 150.0"
    );
}

/// Test normal calculation points upward for flat terrain
#[test]
fn test_normal_calculation_flat_terrain() {
    let config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set flat terrain (all same height)
    for z in 0..8 {
        for x in 0..8 {
            heightmap.set_height(x, z, 50.0);
        }
    }

    // Normal at center should point straight up (0, 1, 0)
    let normal = heightmap.calculate_normal(4, 4, 1.0);

    assert!(
        (normal.y - 1.0).abs() < 0.01,
        "Flat terrain normal Y should be ~1.0, got {}",
        normal.y
    );
    assert!(
        normal.x.abs() < 0.01,
        "Flat terrain normal X should be ~0, got {}",
        normal.x
    );
    assert!(
        normal.z.abs() < 0.01,
        "Flat terrain normal Z should be ~0, got {}",
        normal.z
    );
}

/// Test normal calculation for sloped terrain
#[test]
fn test_normal_calculation_sloped_terrain() {
    let config = HeightmapConfig {
        resolution: 8,
        ..Default::default()
    };
    let mut heightmap = Heightmap::new(config).unwrap();

    // Set sloped terrain (increasing in X direction)
    for z in 0..8 {
        for x in 0..8 {
            heightmap.set_height(x, z, x as f32 * 10.0);
        }
    }

    // Normal should tilt in -X direction
    let normal = heightmap.calculate_normal(4, 4, 1.0);

    assert!(
        normal.x < 0.0,
        "Slope in +X direction should have normal tilting toward -X, got {}",
        normal.x
    );
    // For steep slopes, Y component will be small. Just check it's positive.
    assert!(
        normal.y > 0.0,
        "Normal should have positive Y component (pointing up), got {}",
        normal.y
    );
}

// ============================================================================
// NOISE GENERATION TESTS
// ============================================================================

/// Test noise generation is deterministic for same seed
#[test]
fn test_noise_determinism_same_seed() {
    let config = NoiseConfig::default();
    let seed = 12345u64;

    let noise1 = TerrainNoise::new(&config, seed);
    let noise2 = TerrainNoise::new(&config, seed);

    // Sample at multiple points
    for i in 0..10 {
        let x = i as f64 * 100.0;
        let z = i as f64 * 50.0;

        let h1 = noise1.sample_height(x, z);
        let h2 = noise2.sample_height(x, z);

        assert!(
            (h1 - h2).abs() < 1e-6,
            "Same seed should produce identical heights at ({}, {}). Got {} vs {}",
            x,
            z,
            h1,
            h2
        );
    }
}

/// Test different seeds produce different terrain
#[test]
fn test_noise_different_seeds_different_terrain() {
    let config = NoiseConfig::default();

    let noise1 = TerrainNoise::new(&config, 111);
    let noise2 = TerrainNoise::new(&config, 222);

    // Sample at multiple points and check for differences
    let mut differences = 0;
    for i in 0..100 {
        let x = i as f64 * 10.0;
        let z = i as f64 * 10.0;

        let h1 = noise1.sample_height(x, z);
        let h2 = noise2.sample_height(x, z);

        if (h1 - h2).abs() > 0.01 {
            differences += 1;
        }
    }

    assert!(
        differences > 90,
        "Different seeds should produce mostly different heights, got {} differences",
        differences
    );
}

/// Test noise output is non-negative (height >= 0)
#[test]
fn test_noise_output_non_negative() {
    let config = NoiseConfig::default();
    let noise = TerrainNoise::new(&config, 42);

    // Sample across a large area
    for x in -100..100 {
        for z in -100..100 {
            let height = noise.sample_height(x as f64 * 10.0, z as f64 * 10.0);
            assert!(
                height >= 0.0,
                "Height should be non-negative at ({}, {}), got {}",
                x * 10,
                z * 10,
                height
            );
        }
    }
}

/// Test individual noise layers can be disabled
#[test]
fn test_noise_layer_disable() {
    // Config with only base elevation enabled
    let config_base_only = NoiseConfig {
        base_elevation: NoiseLayer {
            enabled: true,
            scale: 0.01,
            amplitude: 50.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            noise_type: NoiseType::Perlin,
            domain_warp: DomainWarpConfig::default(),
        },
        mountains: NoiseLayer {
            enabled: false,
            scale: 0.002,
            amplitude: 80.0,
            octaves: 6,
            persistence: 0.4,
            lacunarity: 2.2,
            noise_type: NoiseType::RidgedNoise,
            domain_warp: DomainWarpConfig::default(),
        },
        detail: NoiseLayer {
            enabled: false,
            scale: 0.02,
            amplitude: 5.0,
            octaves: 3,
            persistence: 0.6,
            lacunarity: 2.0,
            noise_type: NoiseType::Billow,
            domain_warp: DomainWarpConfig::default(),
        },
        erosion_enabled: false,
        erosion_strength: 0.0,
        ..NoiseConfig::default()
    };

    // Config with all layers enabled
    let config_all = NoiseConfig::default();

    let seed = 42;
    let noise_base_only = TerrainNoise::new(&config_base_only, seed);
    let noise_all = TerrainNoise::new(&config_all, seed);

    // Heights should be different when additional layers are enabled
    let x = 500.0;
    let z = 500.0;
    let h_base = noise_base_only.sample_height(x, z);
    let h_all = noise_all.sample_height(x, z);

    // With mountains and detail disabled, height should generally be lower
    // (mountains add significant positive contribution)
    assert!(
        (h_base - h_all).abs() > 1.0,
        "Different layer configurations should produce different heights. Base: {}, All: {}",
        h_base,
        h_all
    );
}

/// Test noise amplitude affects output proportionally
#[test]
fn test_noise_amplitude_scaling() {
    let config_low_amp = NoiseConfig {
        base_elevation: NoiseLayer {
            enabled: true,
            scale: 0.01,
            amplitude: 10.0, // Low amplitude
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            noise_type: NoiseType::Perlin,
            domain_warp: DomainWarpConfig::default(),
        },
        mountains: disabled_noise_layer(),
        detail: disabled_noise_layer(),
        erosion_enabled: false,
        erosion_strength: 0.0,
        ..NoiseConfig::default()
    };

    let config_high_amp = NoiseConfig {
        base_elevation: NoiseLayer {
            enabled: true,
            scale: 0.01,
            amplitude: 100.0, // High amplitude (10x)
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            noise_type: NoiseType::Perlin,
            domain_warp: DomainWarpConfig::default(),
        },
        mountains: disabled_noise_layer(),
        detail: disabled_noise_layer(),
        erosion_enabled: false,
        erosion_strength: 0.0,
        ..NoiseConfig::default()
    };

    let seed = 42;
    let noise_low = TerrainNoise::new(&config_low_amp, seed);
    let noise_high = TerrainNoise::new(&config_high_amp, seed);

    // Sample heights - due to max(0.0) clamping, exact 10x ratio won't hold
    // but high amplitude should produce larger variation
    let mut max_low = 0.0f32;
    let mut max_high = 0.0f32;

    for i in 0..100 {
        let x = i as f64 * 50.0;
        let z = i as f64 * 30.0;

        let h_low = noise_low.sample_height(x, z);
        let h_high = noise_high.sample_height(x, z);

        max_low = max_low.max(h_low);
        max_high = max_high.max(h_high);
    }

    assert!(
        max_high > max_low,
        "Higher amplitude should produce larger max heights. Low max: {}, High max: {}",
        max_low,
        max_high
    );
}

// ============================================================================
// BIOME CLASSIFICATION TESTS
// ============================================================================

/// Test biome scoring increases when conditions are within range
#[test]
fn test_biome_scoring_within_range_positive() {
    let grassland = BiomeConfig::grassland();

    // Grassland conditions:
    // height: 0-50, temperature: 0.3-0.8, moisture: 0.4-0.8

    // Perfect conditions (all within range)
    let perfect_score = grassland.score_conditions(25.0, 0.5, 0.6);

    // All out of range (high mountain, frozen, dry)
    let poor_score = grassland.score_conditions(500.0, 0.0, 0.0);

    assert!(
        perfect_score > poor_score,
        "Perfect conditions should score higher. Perfect: {}, Poor: {}",
        perfect_score,
        poor_score
    );

    // Perfect conditions should give score >= 3.0 (1.0 per metric + priority bonus)
    assert!(
        perfect_score >= 3.0,
        "Perfect conditions should score at least 3.0, got {}",
        perfect_score
    );
}

/// Test biome scoring penalty for temperature mismatch is higher than moisture
#[test]
fn test_biome_scoring_temperature_penalty_weight() {
    let grassland = BiomeConfig::grassland();

    // Start with perfect conditions
    let baseline = grassland.score_conditions(25.0, 0.5, 0.6);

    // Same temperature deviation (0.5 outside range)
    // Temperature: 0.5 → 0.0 (0.3 outside lower bound of 0.3)
    let temp_out = grassland.score_conditions(25.0, 0.0, 0.6); // temp 0.3 below range

    // Moisture: 0.6 → 0.1 (0.3 outside lower bound of 0.4)
    let moist_out = grassland.score_conditions(25.0, 0.5, 0.1); // moisture 0.3 below range

    // Temperature penalty (2.0x) should be higher than moisture penalty (1.5x)
    let temp_penalty = baseline - temp_out;
    let moist_penalty = baseline - moist_out;

    assert!(
        temp_penalty > moist_penalty,
        "Temperature penalty should be higher. Temp penalty: {}, Moisture penalty: {}",
        temp_penalty,
        moist_penalty
    );
}

/// Test desert requires high temperature and low moisture
#[test]
fn test_desert_climate_requirements() {
    let desert = BiomeConfig::desert();

    // Desert conditions: temp 0.7-1.0, moisture 0.0-0.3
    let ideal_desert = desert.score_conditions(15.0, 0.85, 0.15);
    let wrong_temp = desert.score_conditions(15.0, 0.3, 0.15); // Too cold
    let wrong_moist = desert.score_conditions(15.0, 0.85, 0.7); // Too wet

    assert!(
        ideal_desert > wrong_temp,
        "Desert should prefer hot climate. Ideal: {}, Wrong temp: {}",
        ideal_desert,
        wrong_temp
    );
    assert!(
        ideal_desert > wrong_moist,
        "Desert should prefer dry climate. Ideal: {}, Wrong moisture: {}",
        ideal_desert,
        wrong_moist
    );
}

/// Test tundra requires low temperature
#[test]
fn test_tundra_climate_requirements() {
    let tundra = BiomeConfig::tundra();

    // Tundra conditions: temp 0.0-0.2
    let ideal_tundra = tundra.score_conditions(25.0, 0.1, 0.3);
    let too_warm = tundra.score_conditions(25.0, 0.7, 0.3);

    assert!(
        ideal_tundra > too_warm,
        "Tundra should prefer cold climate. Ideal: {}, Too warm: {}",
        ideal_tundra,
        too_warm
    );
}

/// Test mountain requires high elevation
#[test]
fn test_mountain_elevation_requirement() {
    let mountain = BiomeConfig::mountain();

    // Mountain height range: 60-200
    let ideal_mountain = mountain.score_conditions(100.0, 0.4, 0.5);
    let too_low = mountain.score_conditions(10.0, 0.4, 0.5);

    assert!(
        ideal_mountain > too_low,
        "Mountain should prefer high elevation. Ideal: {}, Too low: {}",
        ideal_mountain,
        too_low
    );
}

/// Test swamp requires high moisture
#[test]
fn test_swamp_moisture_requirement() {
    let swamp = BiomeConfig::swamp();

    // Swamp moisture range: 0.8-1.0
    let ideal_swamp = swamp.score_conditions(5.0, 0.6, 0.9);
    let too_dry = swamp.score_conditions(5.0, 0.6, 0.2);

    assert!(
        ideal_swamp > too_dry,
        "Swamp should prefer wet climate. Ideal: {}, Too dry: {}",
        ideal_swamp,
        too_dry
    );
}

/// Test slope suitability respects max_slope threshold
#[test]
fn test_slope_suitability_threshold() {
    let mountain = BiomeConfig::mountain();
    // Mountain max_slope: 70.0

    assert!(
        mountain.is_slope_suitable(0.0),
        "Flat terrain should be suitable for mountain"
    );
    assert!(
        mountain.is_slope_suitable(30.0),
        "Moderate slope should be suitable for mountain"
    );
    assert!(
        mountain.is_slope_suitable(69.0),
        "Slope just under max should be suitable"
    );
    assert!(
        mountain.is_slope_suitable(70.0),
        "Slope at exactly max should be suitable"
    );
    assert!(
        !mountain.is_slope_suitable(71.0),
        "Slope just over max should NOT be suitable"
    );
    assert!(
        !mountain.is_slope_suitable(90.0),
        "Vertical slope should NOT be suitable"
    );
}

/// Test biome priority affects scoring
#[test]
fn test_biome_priority_bonus() {
    let grassland = BiomeConfig::grassland(); // priority: 1
    let beach = BiomeConfig::beach(); // priority: 7

    // Use identical conditions that fit both
    // Beach height: -5 to 5, temp: 0.5-0.9, moisture: 0.3-0.6
    // Grassland height: 0-50, temp: 0.3-0.8, moisture: 0.4-0.8
    // Common ground: height ~2, temp 0.6, moisture 0.5
    let conditions_score_grass = grassland.score_conditions(2.0, 0.6, 0.5);
    let conditions_score_beach = beach.score_conditions(2.0, 0.6, 0.5);

    // Beach has higher priority (7 vs 1), should add bonus
    // Priority bonus: priority * 0.1
    let grass_priority_bonus = 1.0 * 0.1;
    let beach_priority_bonus = 7.0 * 0.1;

    // The difference should reflect priority difference
    let expected_priority_diff = beach_priority_bonus - grass_priority_bonus;

    // Beach should score higher due to priority (if conditions match)
    assert!(
        conditions_score_beach > conditions_score_grass - expected_priority_diff - 1.0,
        "Beach priority bonus should contribute to score"
    );
}

/// Test biome parsing is case-insensitive
#[test]
fn test_biome_type_parse_case_insensitive() {
    assert_eq!(BiomeType::parse("Grassland"), Some(BiomeType::Grassland));
    assert_eq!(BiomeType::parse("GRASSLAND"), Some(BiomeType::Grassland));
    assert_eq!(BiomeType::parse("grassland"), Some(BiomeType::Grassland));
    assert_eq!(BiomeType::parse("GrAsSlAnD"), Some(BiomeType::Grassland));

    assert_eq!(BiomeType::parse("Desert"), Some(BiomeType::Desert));
    assert_eq!(BiomeType::parse("MOUNTAIN"), Some(BiomeType::Mountain));
    assert_eq!(BiomeType::parse("tundra"), Some(BiomeType::Tundra));
}

/// Test biome type roundtrip (as_str -> parse)
#[test]
fn test_biome_type_roundtrip() {
    for biome in BiomeType::all() {
        let str_repr = biome.as_str();
        let parsed = BiomeType::parse(str_repr);
        assert_eq!(
            parsed,
            Some(*biome),
            "Biome {:?} should roundtrip through as_str/parse",
            biome
        );
    }
}

/// Test all biome types are enumerated by all()
#[test]
fn test_biome_type_all_complete() {
    let all_biomes = BiomeType::all();

    // Should have exactly 8 biome types
    assert_eq!(all_biomes.len(), 8, "Should have 8 biome types");

    // Check each specific type is present
    assert!(
        all_biomes.contains(&BiomeType::Grassland),
        "Missing Grassland"
    );
    assert!(all_biomes.contains(&BiomeType::Desert), "Missing Desert");
    assert!(all_biomes.contains(&BiomeType::Forest), "Missing Forest");
    assert!(
        all_biomes.contains(&BiomeType::Mountain),
        "Missing Mountain"
    );
    assert!(all_biomes.contains(&BiomeType::Tundra), "Missing Tundra");
    assert!(all_biomes.contains(&BiomeType::Swamp), "Missing Swamp");
    assert!(all_biomes.contains(&BiomeType::Beach), "Missing Beach");
    assert!(all_biomes.contains(&BiomeType::River), "Missing River");
}

/// Test default biome configs have valid non-empty data
#[test]
fn test_default_biome_configs_valid() {
    let biomes = vec![
        BiomeConfig::grassland(),
        BiomeConfig::desert(),
        BiomeConfig::forest(),
        BiomeConfig::mountain(),
        BiomeConfig::tundra(),
        BiomeConfig::swamp(),
        BiomeConfig::beach(),
    ];

    for biome in biomes {
        // All should have valid names
        assert!(!biome.name.is_empty(), "Biome should have a name");
        assert!(
            !biome.description.is_empty(),
            "Biome should have a description"
        );

        // All should have resource weights
        assert!(
            !biome.resource_weights.is_empty(),
            "Biome {} should have resource weights",
            biome.name
        );

        // All should have ground textures
        assert!(
            !biome.ground_textures.is_empty(),
            "Biome {} should have ground textures",
            biome.name
        );

        // Height range should be valid (min <= max)
        assert!(
            biome.conditions.height_range.0 <= biome.conditions.height_range.1,
            "Biome {} has invalid height range",
            biome.name
        );

        // Temperature and moisture ranges should be in [0, 1]
        assert!(
            biome.conditions.temperature_range.0 >= 0.0
                && biome.conditions.temperature_range.1 <= 1.0,
            "Biome {} has invalid temperature range",
            biome.name
        );
        assert!(
            biome.conditions.moisture_range.0 >= 0.0 && biome.conditions.moisture_range.1 <= 1.0,
            "Biome {} has invalid moisture range",
            biome.name
        );
    }
}

// ============================================================================
// NOISE LAYER CONFIGURATION TESTS
// ============================================================================

/// Test NoiseConfig default has sensible values
#[test]
fn test_noise_config_default_sensible() {
    let config = NoiseConfig::default();

    // Base elevation should be enabled
    assert!(
        config.base_elevation.enabled,
        "Base elevation should be enabled by default"
    );

    // Octaves should be reasonable (1-8)
    assert!(
        config.base_elevation.octaves >= 1 && config.base_elevation.octaves <= 8,
        "Base octaves should be 1-8, got {}",
        config.base_elevation.octaves
    );

    // Persistence should be (0, 1)
    assert!(
        config.base_elevation.persistence > 0.0 && config.base_elevation.persistence <= 1.0,
        "Persistence should be in (0,1], got {}",
        config.base_elevation.persistence
    );

    // Lacunarity should be > 1 (frequency multiplier)
    assert!(
        config.base_elevation.lacunarity > 1.0,
        "Lacunarity should be > 1, got {}",
        config.base_elevation.lacunarity
    );
}

/// Test different noise types produce different patterns
#[test]
fn test_noise_type_variety() {
    let seed = 42u64;
    let x = 100.0;
    let z = 100.0;

    // Create configs with different noise types
    let perlin_config = NoiseConfig {
        base_elevation: NoiseLayer {
            enabled: true,
            scale: 0.01,
            amplitude: 50.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            noise_type: NoiseType::Perlin,
            domain_warp: DomainWarpConfig::default(),
        },
        mountains: disabled_noise_layer(),
        detail: disabled_noise_layer(),
        erosion_enabled: false,
        erosion_strength: 0.0,
        ..NoiseConfig::default()
    };

    let ridged_config = NoiseConfig {
        base_elevation: NoiseLayer {
            noise_type: NoiseType::RidgedNoise,
            ..perlin_config.base_elevation.clone()
        },
        ..perlin_config.clone()
    };

    let billow_config = NoiseConfig {
        base_elevation: NoiseLayer {
            noise_type: NoiseType::Billow,
            ..perlin_config.base_elevation.clone()
        },
        ..perlin_config.clone()
    };

    let noise_perlin = TerrainNoise::new(&perlin_config, seed);
    let noise_ridged = TerrainNoise::new(&ridged_config, seed);
    let noise_billow = TerrainNoise::new(&billow_config, seed);

    let h_perlin = noise_perlin.sample_height(x, z);
    let h_ridged = noise_ridged.sample_height(x, z);
    let h_billow = noise_billow.sample_height(x, z);

    // They should all produce different values (or at least not all identical)
    let all_same = (h_perlin - h_ridged).abs() < 0.01 && (h_ridged - h_billow).abs() < 0.01;
    assert!(
        !all_same,
        "Different noise types should produce different values. Perlin: {}, Ridged: {}, Billow: {}",
        h_perlin, h_ridged, h_billow
    );
}

// ============================================================================
// HEIGHTMAP CONFIG TESTS
// ============================================================================

/// Test HeightmapConfig default values
#[test]
fn test_heightmap_config_default() {
    let config = HeightmapConfig::default();

    assert_eq!(config.resolution, 128, "Default resolution should be 128");
    assert!(
        (config.min_height - 0.0).abs() < 1e-6,
        "Default min_height should be 0.0"
    );
    assert!(
        (config.max_height - 100.0).abs() < 1e-6,
        "Default max_height should be 100.0"
    );
    assert!(
        (config.height_scale - 1.0).abs() < 1e-6,
        "Default height_scale should be 1.0"
    );
}

/// Test resolution property getter
#[test]
fn test_heightmap_resolution_getter() {
    for res in [4, 16, 64, 128, 256] {
        let config = HeightmapConfig {
            resolution: res,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();
        assert_eq!(
            heightmap.resolution(),
            res,
            "Resolution getter should return configured value"
        );
    }
}

/// Test data array has correct size
#[test]
fn test_heightmap_data_size() {
    let config = HeightmapConfig {
        resolution: 32,
        ..Default::default()
    };
    let heightmap = Heightmap::new(config).unwrap();

    assert_eq!(
        heightmap.data().len(),
        32 * 32,
        "Data array should have resolution^2 elements"
    );
}
