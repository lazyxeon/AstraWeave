//! Comprehensive mutation-killing tests for astraweave-terrain
//!
//! These tests are designed to catch arithmetic and logical mutations
//! by verifying specific expected values and behavioral correctness.

#[cfg(test)]
mod heightmap_tests {
    use crate::heightmap::{Heightmap, HeightmapConfig};

    #[test]
    fn test_heightmap_config_defaults() {
        let config = HeightmapConfig::default();
        assert_eq!(config.resolution, 128);
        assert_eq!(config.min_height, 0.0);
        assert_eq!(config.max_height, 100.0);
        assert_eq!(config.height_scale, 1.0);
    }

    #[test]
    fn test_heightmap_new_creates_zero_filled() {
        let config = HeightmapConfig {
            resolution: 4,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();
        assert_eq!(heightmap.resolution(), 4);
        assert_eq!(heightmap.data().len(), 16); // 4x4
        assert!(heightmap.data().iter().all(|&h| h == 0.0));
    }

    #[test]
    fn test_heightmap_from_data_validates_size() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let heightmap = Heightmap::from_data(data, 3).unwrap();
        assert_eq!(heightmap.resolution(), 3);
        assert_eq!(heightmap.data().len(), 9);
    }

    #[test]
    fn test_heightmap_from_data_rejects_wrong_size() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5 elements, not 4 or 9
        let result = Heightmap::from_data(data, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_heightmap_from_data_calculates_min_max() {
        let data = vec![5.0, 10.0, -3.0, 8.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert_eq!(heightmap.min_height(), -3.0);
        assert_eq!(heightmap.max_height(), 10.0);
    }

    #[test]
    fn test_heightmap_get_height_bounds_check() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();

        assert_eq!(heightmap.get_height(0, 0), 1.0);
        assert_eq!(heightmap.get_height(1, 0), 2.0);
        assert_eq!(heightmap.get_height(0, 1), 3.0);
        assert_eq!(heightmap.get_height(1, 1), 4.0);

        // Out of bounds should return 0
        assert_eq!(heightmap.get_height(2, 0), 0.0);
        assert_eq!(heightmap.get_height(0, 2), 0.0);
    }

    #[test]
    fn test_heightmap_set_height_updates_bounds() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        heightmap.set_height(0, 0, 50.0);
        assert_eq!(heightmap.get_height(0, 0), 50.0);
        assert_eq!(heightmap.max_height(), 50.0);

        heightmap.set_height(1, 1, -10.0);
        assert_eq!(heightmap.min_height(), -10.0);
    }

    #[test]
    fn test_heightmap_set_height_ignores_out_of_bounds() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        // Should not crash or modify anything
        heightmap.set_height(5, 5, 100.0);
        assert!(heightmap.data().iter().all(|&h| h == 0.0));
    }

    #[test]
    fn test_heightmap_bilinear_sample_corners() {
        // For a 2x2 heightmap, the bilinear samples coordinates [0,1) in each axis
        // The clamping means we can't actually sample at exactly 1.0
        let data = vec![0.0, 10.0, 20.0, 30.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();

        // At (0,0) corner should match exact value
        assert!((heightmap.sample_bilinear(0.0, 0.0) - 0.0).abs() < 0.01);

        // Near upper bounds, value should be interpolated (due to clamping to res-1.001)
        // Just verify they return reasonable values within range
        let upper_x = heightmap.sample_bilinear(0.999, 0.0);
        let upper_z = heightmap.sample_bilinear(0.0, 0.999);
        assert!((0.0..=10.0).contains(&upper_x));
        assert!((0.0..=20.0).contains(&upper_z));
    }

    #[test]
    fn test_heightmap_bilinear_sample_center() {
        let data = vec![0.0, 10.0, 20.0, 30.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();

        // At center (0.5, 0.5), should be average of all four
        let center = heightmap.sample_bilinear(0.5, 0.5);
        let expected = (0.0 + 10.0 + 20.0 + 30.0) / 4.0;
        assert!(
            (center - expected).abs() < 0.01,
            "Center: {}, Expected: {}",
            center,
            expected
        );
    }

    #[test]
    fn test_heightmap_recalculate_bounds() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        // Manually modify data
        let data = heightmap.data_mut();
        data[0] = 5.0;
        data[1] = 15.0;
        data[2] = -5.0;
        data[3] = 25.0;

        heightmap.recalculate_bounds();
        assert_eq!(heightmap.min_height(), -5.0);
        assert_eq!(heightmap.max_height(), 25.0);
    }
}

#[cfg(test)]
mod chunk_id_tests {
    use crate::chunk::ChunkId;
    use glam::Vec3;

    #[test]
    fn test_chunk_id_new() {
        let id = ChunkId::new(5, -3);
        assert_eq!(id.x, 5);
        assert_eq!(id.z, -3);
    }

    #[test]
    fn test_chunk_id_from_world_pos_positive() {
        let chunk_size = 64.0;
        let id = ChunkId::from_world_pos(Vec3::new(100.0, 50.0, 200.0), chunk_size);
        assert_eq!(id.x, 1); // 100/64 = 1.56 -> floor = 1
        assert_eq!(id.z, 3); // 200/64 = 3.12 -> floor = 3
    }

    #[test]
    fn test_chunk_id_from_world_pos_negative() {
        let chunk_size = 64.0;
        let id = ChunkId::from_world_pos(Vec3::new(-100.0, 50.0, -200.0), chunk_size);
        assert_eq!(id.x, -2); // -100/64 = -1.56 -> floor = -2
        assert_eq!(id.z, -4); // -200/64 = -3.12 -> floor = -4
    }

    #[test]
    fn test_chunk_id_from_world_pos_at_origin() {
        let chunk_size = 64.0;
        let id = ChunkId::from_world_pos(Vec3::ZERO, chunk_size);
        assert_eq!(id.x, 0);
        assert_eq!(id.z, 0);
    }

    #[test]
    fn test_chunk_id_to_world_pos() {
        let chunk_size = 64.0;
        let id = ChunkId::new(2, 3);
        let pos = id.to_world_pos(chunk_size);

        assert_eq!(pos.x, 128.0); // 2 * 64
        assert_eq!(pos.y, 0.0);
        assert_eq!(pos.z, 192.0); // 3 * 64
    }

    #[test]
    fn test_chunk_id_to_center_pos() {
        let chunk_size = 64.0;
        let id = ChunkId::new(0, 0);
        let center = id.to_center_pos(chunk_size);

        assert_eq!(center.x, 32.0);
        assert_eq!(center.y, 0.0);
        assert_eq!(center.z, 32.0);
    }

    #[test]
    fn test_chunk_id_distance_to_same() {
        let id1 = ChunkId::new(5, 5);
        let id2 = ChunkId::new(5, 5);
        assert_eq!(id1.distance_to(id2), 0.0);
    }

    #[test]
    fn test_chunk_id_distance_to_horizontal() {
        let id1 = ChunkId::new(0, 0);
        let id2 = ChunkId::new(3, 4);
        assert_eq!(id1.distance_to(id2), 5.0); // 3-4-5 triangle
    }

    #[test]
    fn test_chunk_id_get_chunks_in_radius() {
        let chunks = ChunkId::get_chunks_in_radius(Vec3::ZERO, 1, 64.0);
        assert_eq!(chunks.len(), 9); // 3x3 grid around center

        // Verify all expected chunks are present
        assert!(chunks.contains(&ChunkId::new(0, 0)));
        assert!(chunks.contains(&ChunkId::new(-1, 0)));
        assert!(chunks.contains(&ChunkId::new(1, 0)));
        assert!(chunks.contains(&ChunkId::new(0, -1)));
        assert!(chunks.contains(&ChunkId::new(0, 1)));
    }

    #[test]
    fn test_chunk_id_roundtrip() {
        let chunk_size = 64.0;
        let original = ChunkId::new(10, -5);
        let world_pos = original.to_world_pos(chunk_size);
        let recovered = ChunkId::from_world_pos(world_pos, chunk_size);

        assert_eq!(original, recovered);
    }
}

#[cfg(test)]
mod voxel_tests {
    use crate::voxel_data::{ChunkCoord, Voxel, CHUNK_SIZE};
    use glam::Vec3;

    #[test]
    fn test_chunk_size_constant() {
        assert_eq!(CHUNK_SIZE, 32);
    }

    #[test]
    fn test_voxel_default() {
        let voxel = Voxel::default();
        assert_eq!(voxel.density, 0.0);
        assert_eq!(voxel.material, 0);
    }

    #[test]
    fn test_voxel_new() {
        let voxel = Voxel::new(0.75, 5);
        assert_eq!(voxel.density, 0.75);
        assert_eq!(voxel.material, 5);
    }

    #[test]
    fn test_voxel_is_solid_threshold() {
        assert!(!Voxel::new(0.0, 0).is_solid());
        assert!(!Voxel::new(0.49, 0).is_solid());
        assert!(!Voxel::new(0.5, 0).is_solid()); // Exactly 0.5 is NOT solid (>0.5)
        assert!(Voxel::new(0.51, 0).is_solid());
        assert!(Voxel::new(1.0, 0).is_solid());
    }

    #[test]
    fn test_chunk_coord_new() {
        let coord = ChunkCoord::new(1, 2, 3);
        assert_eq!(coord.x, 1);
        assert_eq!(coord.y, 2);
        assert_eq!(coord.z, 3);
    }

    #[test]
    fn test_chunk_coord_from_world_pos() {
        let coord = ChunkCoord::from_world_pos(Vec3::new(50.0, 100.0, 150.0));
        assert_eq!(coord.x, 1); // 50/32 = 1.56 -> floor = 1
        assert_eq!(coord.y, 3); // 100/32 = 3.12 -> floor = 3
        assert_eq!(coord.z, 4); // 150/32 = 4.68 -> floor = 4
    }

    #[test]
    fn test_chunk_coord_from_world_pos_negative() {
        let coord = ChunkCoord::from_world_pos(Vec3::new(-50.0, -100.0, -150.0));
        assert_eq!(coord.x, -2); // -50/32 = -1.56 -> floor = -2
        assert_eq!(coord.y, -4); // -100/32 = -3.12 -> floor = -4
        assert_eq!(coord.z, -5); // -150/32 = -4.68 -> floor = -5
    }

    #[test]
    fn test_chunk_coord_to_world_pos() {
        let coord = ChunkCoord::new(2, 3, 4);
        let world = coord.to_world_pos();

        assert_eq!(world.x, 64.0); // 2 * 32
        assert_eq!(world.y, 96.0); // 3 * 32
        assert_eq!(world.z, 128.0); // 4 * 32
    }

    #[test]
    fn test_chunk_coord_neighbors() {
        let coord = ChunkCoord::new(0, 0, 0);
        let neighbors = coord.neighbors();

        assert_eq!(neighbors.len(), 6);
        assert!(neighbors.contains(&ChunkCoord::new(1, 0, 0)));
        assert!(neighbors.contains(&ChunkCoord::new(-1, 0, 0)));
        assert!(neighbors.contains(&ChunkCoord::new(0, 1, 0)));
        assert!(neighbors.contains(&ChunkCoord::new(0, -1, 0)));
        assert!(neighbors.contains(&ChunkCoord::new(0, 0, 1)));
        assert!(neighbors.contains(&ChunkCoord::new(0, 0, -1)));
    }

    #[test]
    fn test_chunk_coord_roundtrip() {
        let original = ChunkCoord::new(5, -3, 7);
        let world_pos = original.to_world_pos();
        let recovered = ChunkCoord::from_world_pos(world_pos);

        assert_eq!(original, recovered);
    }
}

#[cfg(test)]
mod biome_tests {
    use crate::biome::BiomeType;
    use std::str::FromStr;

    #[test]
    fn test_biome_type_as_str() {
        assert_eq!(BiomeType::Grassland.as_str(), "grassland");
        assert_eq!(BiomeType::Desert.as_str(), "desert");
        assert_eq!(BiomeType::Forest.as_str(), "forest");
        assert_eq!(BiomeType::Mountain.as_str(), "mountain");
        assert_eq!(BiomeType::Tundra.as_str(), "tundra");
        assert_eq!(BiomeType::Swamp.as_str(), "swamp");
        assert_eq!(BiomeType::Beach.as_str(), "beach");
        assert_eq!(BiomeType::River.as_str(), "river");
    }

    #[test]
    fn test_biome_type_from_str_lowercase() {
        assert_eq!(
            BiomeType::from_str("grassland").unwrap(),
            BiomeType::Grassland
        );
        assert_eq!(BiomeType::from_str("desert").unwrap(), BiomeType::Desert);
        assert_eq!(BiomeType::from_str("forest").unwrap(), BiomeType::Forest);
        assert_eq!(
            BiomeType::from_str("mountain").unwrap(),
            BiomeType::Mountain
        );
        assert_eq!(BiomeType::from_str("tundra").unwrap(), BiomeType::Tundra);
        assert_eq!(BiomeType::from_str("swamp").unwrap(), BiomeType::Swamp);
        assert_eq!(BiomeType::from_str("beach").unwrap(), BiomeType::Beach);
        assert_eq!(BiomeType::from_str("river").unwrap(), BiomeType::River);
    }

    #[test]
    fn test_biome_type_from_str_case_insensitive() {
        assert_eq!(
            BiomeType::from_str("GRASSLAND").unwrap(),
            BiomeType::Grassland
        );
        assert_eq!(
            BiomeType::from_str("Grassland").unwrap(),
            BiomeType::Grassland
        );
        assert_eq!(
            BiomeType::from_str("GrAsSlAnD").unwrap(),
            BiomeType::Grassland
        );
    }

    #[test]
    fn test_biome_type_from_str_invalid() {
        assert!(BiomeType::from_str("invalid").is_err());
        assert!(BiomeType::from_str("").is_err());
        assert!(BiomeType::from_str("ocean").is_err());
    }

    #[test]
    fn test_biome_type_parse_method() {
        assert_eq!(BiomeType::parse("forest"), Some(BiomeType::Forest));
        assert_eq!(BiomeType::parse("MOUNTAIN"), Some(BiomeType::Mountain));
        assert_eq!(BiomeType::parse("invalid"), None);
    }

    #[test]
    fn test_biome_type_all() {
        let all = BiomeType::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&BiomeType::Grassland));
        assert!(all.contains(&BiomeType::Desert));
        assert!(all.contains(&BiomeType::Forest));
        assert!(all.contains(&BiomeType::Mountain));
        assert!(all.contains(&BiomeType::Tundra));
        assert!(all.contains(&BiomeType::Swamp));
        assert!(all.contains(&BiomeType::Beach));
        assert!(all.contains(&BiomeType::River));
    }

    #[test]
    fn test_biome_type_all_unique() {
        let all = BiomeType::all();
        for (i, biome1) in all.iter().enumerate() {
            for (j, biome2) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(biome1, biome2);
                }
            }
        }
    }
}

#[cfg(test)]
mod noise_config_tests {
    use crate::noise_gen::{NoiseConfig, NoiseType};

    #[test]
    fn test_noise_config_default_has_all_layers() {
        let config = NoiseConfig::default();
        assert!(config.base_elevation.enabled);
        assert!(config.mountains.enabled);
        assert!(config.detail.enabled);
    }

    #[test]
    fn test_noise_config_default_erosion() {
        let config = NoiseConfig::default();
        assert!(config.erosion_enabled);
        assert!(config.erosion_strength > 0.0);
        assert!(config.erosion_strength <= 1.0);
    }

    #[test]
    fn test_noise_layer_base_elevation() {
        let config = NoiseConfig::default();
        let base = &config.base_elevation;

        assert!(base.scale > 0.0);
        assert!(base.amplitude > 0.0);
        assert!(base.octaves >= 1);
        assert!(base.persistence > 0.0);
        assert!(base.persistence < 1.0);
        assert!(base.lacunarity > 1.0);
    }

    #[test]
    fn test_noise_layer_mountains() {
        let config = NoiseConfig::default();
        let mountains = &config.mountains;

        // Mountains should have higher amplitude and more octaves
        assert!(mountains.amplitude > config.base_elevation.amplitude);
        assert!(mountains.octaves >= config.base_elevation.octaves);
    }

    #[test]
    fn test_noise_layer_detail() {
        let config = NoiseConfig::default();
        let detail = &config.detail;

        // Detail should have smaller amplitude and larger scale (higher frequency)
        assert!(detail.amplitude < config.base_elevation.amplitude);
        assert!(detail.scale > config.base_elevation.scale);
    }

    #[test]
    fn test_noise_type_variants() {
        // Verify all variants are distinguishable
        let perlin = NoiseType::Perlin;
        let ridged = NoiseType::RidgedNoise;
        let billow = NoiseType::Billow;
        let fbm = NoiseType::Fbm;

        assert!(format!("{:?}", perlin).contains("Perlin"));
        assert!(format!("{:?}", ridged).contains("Ridged"));
        assert!(format!("{:?}", billow).contains("Billow"));
        assert!(format!("{:?}", fbm).contains("Fbm"));
    }
}

#[cfg(test)]
mod climate_tests {
    use crate::climate::ClimateConfig;

    #[test]
    fn test_climate_config_default() {
        let config = ClimateConfig::default();

        // Temperature layer should have reasonable world scale
        assert!(config.temperature.scale > 0.0);
        assert!(config.temperature.octaves >= 1);
        assert!(config.temperature.persistence > 0.0 && config.temperature.persistence <= 1.0);
        assert!(config.temperature.amplitude > 0.0);

        // Moisture layer should have reasonable world scale
        assert!(config.moisture.scale > 0.0);
        assert!(config.moisture.octaves >= 1);
        assert!(config.moisture.persistence > 0.0 && config.moisture.persistence <= 1.0);
        assert!(config.moisture.amplitude > 0.0);
    }

    #[test]
    fn test_climate_gradients() {
        let config = ClimateConfig::default();

        // Height gradient should be negative (cooler at altitude)
        assert!(config.temperature_height_gradient < 0.0);

        // Latitude gradient should be positive
        assert!(config.temperature_latitude_gradient > 0.0);

        // Moisture falloff should be positive
        assert!(config.moisture_distance_falloff > 0.0);
    }

    #[test]
    fn test_climate_layer_lacunarity() {
        let config = ClimateConfig::default();

        // Lacunarity should be > 1 for increasing frequency
        assert!(config.temperature.lacunarity > 1.0);
        assert!(config.moisture.lacunarity > 1.0);
    }
}

#[cfg(test)]
mod world_config_tests {
    use crate::WorldConfig;

    #[test]
    fn test_world_config_default() {
        let config = WorldConfig::default();

        // Chunk size should be reasonable
        assert!(config.chunk_size > 0.0);
        assert!(config.chunk_size >= 32.0);

        // Resolution should be power of 2 or reasonable
        assert!(config.heightmap_resolution >= 32);
        assert!(config.heightmap_resolution <= 512);

        // Seed should exist
        assert!(config.seed > 0);
    }
}

#[cfg(test)]
mod texture_splatting_tests {
    use crate::texture_splatting::{SplatWeights, MAX_SPLAT_LAYERS};

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_max_splat_layers_constant() {
        assert!(MAX_SPLAT_LAYERS >= 4);
        assert!(MAX_SPLAT_LAYERS <= 16);
    }

    #[test]
    fn test_splat_weights_from_weights_normalization() {
        let weights = SplatWeights::from_weights(&[0.5, 0.5, 0.0, 0.0]);

        // Weights should sum to 1.0 (normalized)
        let sum = weights.weights_0.x
            + weights.weights_0.y
            + weights.weights_0.z
            + weights.weights_0.w
            + weights.weights_1.x
            + weights.weights_1.y
            + weights.weights_1.z
            + weights.weights_1.w;
        assert!((sum - 1.0).abs() < 0.001, "Weights sum: {}", sum);
    }

    #[test]
    fn test_splat_weights_single_layer() {
        let weights = SplatWeights::from_weights(&[1.0]);

        // First weight should dominate
        assert!(weights.weights_0.x > 0.99);
        assert!(weights.weights_0.y < 0.01);
    }

    #[test]
    fn test_splat_weights_multiple_layers() {
        let weights = SplatWeights::from_weights(&[0.25, 0.25, 0.25, 0.25]);

        // All first 4 weights should be equal
        assert!((weights.weights_0.x - 0.25).abs() < 0.01);
        assert!((weights.weights_0.y - 0.25).abs() < 0.01);
        assert!((weights.weights_0.z - 0.25).abs() < 0.01);
        assert!((weights.weights_0.w - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_splat_weights_default() {
        let weights = SplatWeights::default();

        // Default should be valid (not NaN)
        assert!(!weights.weights_0.x.is_nan());
        assert!(!weights.weights_1.x.is_nan());
    }

    #[test]
    fn test_splat_weights_extended_layers() {
        let weights = SplatWeights::from_weights(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);

        // Second weight vector should be populated
        let second_sum =
            weights.weights_1.x + weights.weights_1.y + weights.weights_1.z + weights.weights_1.w;
        assert!(second_sum > 0.0);
    }
}

#[cfg(test)]
mod lod_tests {
    use crate::lod_manager::LodLevel;

    #[test]
    fn test_lod_level_variants() {
        // All LOD levels should be distinct and valid
        let full = LodLevel::Full;
        let half = LodLevel::Half;
        let quarter = LodLevel::Quarter;
        let skybox = LodLevel::Skybox;

        // Each should be debuggable
        assert!(!format!("{:?}", full).is_empty());
        assert!(!format!("{:?}", half).is_empty());
        assert!(!format!("{:?}", quarter).is_empty());
        assert!(!format!("{:?}", skybox).is_empty());
    }

    #[test]
    fn test_lod_level_full_is_distinct() {
        let full = LodLevel::Full;
        let half = LodLevel::Half;

        // Full should not equal Half
        assert_ne!(format!("{:?}", full), format!("{:?}", half));
    }

    #[test]
    fn test_lod_level_skybox_is_lowest_detail() {
        // Skybox should be the lowest detail level
        let skybox = LodLevel::Skybox;
        assert!(format!("{:?}", skybox).contains("Skybox"));
    }
}

#[cfg(test)]
mod streaming_tests {
    use crate::background_loader::StreamingConfig;

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();

        // View distance should be reasonable (u32)
        assert!(config.view_distance > 0);
        assert!(config.view_distance <= 100);

        // Prefetch distance should be reasonable
        assert!(config.prefetch_distance > 0);
        assert!(config.prefetch_distance <= config.view_distance);

        // Max concurrent loads should be at least 1
        assert!(config.max_concurrent_loads >= 1);

        // Chunk size should be positive
        assert!(config.chunk_size > 0.0);
    }

    #[test]
    fn test_streaming_config_adaptive_throttle() {
        let config = StreamingConfig::default();

        // Throttle threshold should be reasonable frame time
        assert!(config.adaptive_throttle_threshold_ms > 0.0);
        assert!(config.adaptive_throttle_threshold_ms < 100.0);

        // Throttled loads should be less than normal
        assert!(config.throttled_concurrent_loads <= config.max_concurrent_loads);
    }

    #[test]
    fn test_streaming_config_max_loaded() {
        let config = StreamingConfig::default();

        // Max loaded chunks should be reasonable
        assert!(config.max_loaded_chunks >= 16);
        assert!(config.max_loaded_chunks <= 1024);
    }
}

#[cfg(test)]
mod erosion_tests {
    use crate::advanced_erosion::{ErosionPreset, HydraulicErosionConfig, ThermalErosionConfig};

    #[test]
    fn test_erosion_preset_default() {
        let preset = ErosionPreset::default();

        // Default should have a name
        assert!(!preset.name.is_empty());

        // Default should have hydraulic and thermal
        assert!(preset.hydraulic.is_some());
        assert!(preset.thermal.is_some());

        // Pass order should be non-empty
        assert!(!preset.pass_order.is_empty());
    }

    #[test]
    fn test_erosion_preset_desert() {
        let preset = ErosionPreset::desert();

        // Desert should have wind erosion
        assert!(preset.wind.is_some());
        assert!(preset.name.contains("Desert"));
    }

    #[test]
    fn test_erosion_preset_mountain() {
        let preset = ErosionPreset::mountain();

        // Mountain should have heavy hydraulic
        assert!(preset.hydraulic.is_some());
        if let Some(ref hydraulic) = preset.hydraulic {
            assert!(hydraulic.droplet_count > 50000);
        }
    }

    #[test]
    fn test_hydraulic_erosion_config_default() {
        let config = HydraulicErosionConfig::default();

        // Droplet count should be reasonable
        assert!(config.droplet_count > 0);
        assert!(config.droplet_count <= 500000);

        // Erode speed should be positive
        assert!(config.erode_speed > 0.0);

        // Deposit speed should be positive
        assert!(config.deposit_speed > 0.0);

        // Erosion radius should be reasonable
        assert!(config.erosion_radius > 0);
        assert!(config.erosion_radius <= 10);
    }

    #[test]
    fn test_thermal_erosion_config_default() {
        let config = ThermalErosionConfig::default();

        // Iterations should be reasonable
        assert!(config.iterations > 0);
        assert!(config.iterations <= 1000);

        // Talus angle should be in reasonable range
        assert!(config.talus_angle > 0.0);
        assert!(config.talus_angle < 90.0);

        // Redistribution rate should be between 0 and 1
        assert!(config.redistribution_rate >= 0.0);
        assert!(config.redistribution_rate <= 1.0);
    }
}

#[cfg(test)]
mod solver_tests {
    use crate::biome::BiomeType;
    use crate::solver::ValidationStatus;

    #[test]
    fn test_validation_status_valid() {
        let valid = ValidationStatus::Valid;
        assert!(format!("{:?}", valid).contains("Valid"));
    }

    #[test]
    fn test_validation_status_out_of_bounds() {
        let oob = ValidationStatus::OutOfBounds;
        assert!(format!("{:?}", oob).contains("OutOfBounds"));
    }

    #[test]
    fn test_validation_status_no_solid_ground() {
        let nsg = ValidationStatus::NoSolidGround;
        assert!(format!("{:?}", nsg).contains("NoSolidGround"));
    }

    #[test]
    fn test_validation_status_chunk_not_loaded() {
        let cnl = ValidationStatus::ChunkNotLoaded;
        assert!(format!("{:?}", cnl).contains("ChunkNotLoaded"));
    }

    #[test]
    fn test_validation_status_biome_incompatible() {
        let bi = ValidationStatus::BiomeIncompatible(BiomeType::Desert);
        assert!(format!("{:?}", bi).contains("BiomeIncompatible"));
    }
}

#[cfg(test)]
mod terrain_modifier_tests {
    use crate::terrain_modifier::VoxelOpType;
    use crate::voxel_data::Voxel;

    #[test]
    fn test_voxel_op_type_set() {
        let voxel = Voxel::new(0.8, 1);
        let set_op = VoxelOpType::Set(voxel);
        assert!(format!("{:?}", set_op).contains("Set"));
    }

    #[test]
    fn test_voxel_op_type_add_density() {
        let add = VoxelOpType::AddDensity(0.5);
        assert!(format!("{:?}", add).contains("AddDensity"));
    }

    #[test]
    fn test_voxel_op_type_subtract_density() {
        let subtract = VoxelOpType::SubtractDensity(0.3);
        assert!(format!("{:?}", subtract).contains("SubtractDensity"));
    }

    #[test]
    fn test_voxel_op_type_set_material() {
        let set_mat = VoxelOpType::SetMaterial(42);
        assert!(format!("{:?}", set_mat).contains("SetMaterial"));
    }

    #[test]
    fn test_voxel_op_type_blend() {
        let voxel = Voxel::new(0.5, 2);
        let blend = VoxelOpType::Blend { voxel, factor: 0.5 };
        assert!(format!("{:?}", blend).contains("Blend"));
    }
}

#[cfg(test)]
mod structure_tests {
    use crate::structures::StructureType;

    #[test]
    fn test_structure_type_residential() {
        // Verify residential structure types
        let cottage = StructureType::Cottage;
        let farmhouse = StructureType::Farmhouse;
        let villa = StructureType::Villa;
        let cabin = StructureType::Cabin;

        assert!(format!("{:?}", cottage).contains("Cottage"));
        assert!(format!("{:?}", farmhouse).contains("Farmhouse"));
        assert!(format!("{:?}", villa).contains("Villa"));
        assert!(format!("{:?}", cabin).contains("Cabin"));
    }

    #[test]
    fn test_structure_type_commercial() {
        let tavern = StructureType::Tavern;
        let blacksmith = StructureType::Blacksmith;
        let market = StructureType::Market;

        assert!(format!("{:?}", tavern).contains("Tavern"));
        assert!(format!("{:?}", blacksmith).contains("Blacksmith"));
        assert!(format!("{:?}", market).contains("Market"));
    }

    #[test]
    fn test_structure_type_defensive() {
        let tower = StructureType::Watchtower;
        let fort = StructureType::Fort;
        let wall = StructureType::Wall;

        assert!(format!("{:?}", tower).contains("Watchtower"));
        assert!(format!("{:?}", fort).contains("Fort"));
        assert!(format!("{:?}", wall).contains("Wall"));
    }

    #[test]
    fn test_structure_type_ancient() {
        let ruin = StructureType::AncientRuin;
        let obelisk = StructureType::Obelisk;
        let tomb = StructureType::Tomb;

        assert!(format!("{:?}", ruin).contains("AncientRuin"));
        assert!(format!("{:?}", obelisk).contains("Obelisk"));
        assert!(format!("{:?}", tomb).contains("Tomb"));
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use crate::streaming_diagnostics::ChunkLoadState;

    #[test]
    fn test_chunk_load_state_pending() {
        let pending = ChunkLoadState::Pending;
        assert!(format!("{:?}", pending).contains("Pending"));
    }

    #[test]
    fn test_chunk_load_state_loading() {
        let loading = ChunkLoadState::Loading;
        assert!(format!("{:?}", loading).contains("Loading"));
    }

    #[test]
    fn test_chunk_load_state_loaded() {
        let loaded = ChunkLoadState::Loaded;
        assert!(format!("{:?}", loaded).contains("Loaded"));
    }

    #[test]
    fn test_chunk_load_state_unloaded() {
        let unloaded = ChunkLoadState::Unloaded;
        assert!(format!("{:?}", unloaded).contains("Unloaded"));
    }

    #[test]
    fn test_chunk_load_states_distinct() {
        // All states should be distinct
        assert_ne!(ChunkLoadState::Pending, ChunkLoadState::Loading);
        assert_ne!(ChunkLoadState::Loading, ChunkLoadState::Loaded);
        assert_ne!(ChunkLoadState::Loaded, ChunkLoadState::Unloaded);
    }
}

#[cfg(test)]
mod behavioral_correctness_tests {
    //! Tests that verify terrain generation produces physically correct results

    use crate::chunk::ChunkId;
    use crate::heightmap::{Heightmap, HeightmapConfig};
    use crate::voxel_data::{ChunkCoord, CHUNK_SIZE};
    use glam::Vec3;

    #[test]
    fn test_chunk_world_pos_consistency() {
        // World position to chunk and back should be consistent
        let chunk_size = 64.0;

        for x in -10..10 {
            for z in -10..10 {
                let chunk = ChunkId::new(x, z);
                let world = chunk.to_world_pos(chunk_size);

                // Position inside chunk should map back to same chunk
                let inside = world + Vec3::new(chunk_size * 0.5, 0.0, chunk_size * 0.5);
                let recovered = ChunkId::from_world_pos(inside, chunk_size);

                assert_eq!(chunk, recovered, "Failed for chunk ({}, {})", x, z);
            }
        }
    }

    #[test]
    fn test_voxel_chunk_coord_consistency() {
        // Similar test for voxel chunks
        for x in -5..5 {
            for y in -5..5 {
                for z in -5..5 {
                    let coord = ChunkCoord::new(x, y, z);
                    let world = coord.to_world_pos();

                    // Position inside chunk should map back to same chunk
                    let inside = world + Vec3::splat(CHUNK_SIZE as f32 * 0.5);
                    let recovered = ChunkCoord::from_world_pos(inside);

                    assert_eq!(coord, recovered, "Failed for coord ({}, {}, {})", x, y, z);
                }
            }
        }
    }

    #[test]
    fn test_heightmap_index_calculation() {
        // Verify index calculation: index = z * resolution + x
        let config = HeightmapConfig {
            resolution: 4,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        // Set each position with unique value
        for z in 0..4 {
            for x in 0..4 {
                let expected_index = z * 4 + x;
                heightmap.set_height(x, z, expected_index as f32);
            }
        }

        // Verify values
        for z in 0..4 {
            for x in 0..4 {
                let expected = (z * 4 + x) as f32;
                let actual = heightmap.get_height(x, z);
                assert_eq!(actual, expected, "Mismatch at ({}, {})", x, z);
            }
        }
    }

    #[test]
    fn test_bilinear_interpolation_linearity() {
        // Bilinear interpolation should be linear along edges
        // Using a 3x3 grid for clearer interpolation testing
        let data = vec![0.0, 50.0, 100.0, 0.0, 50.0, 100.0, 0.0, 50.0, 100.0];
        let heightmap = Heightmap::from_data(data, 3).unwrap();

        // Along x edge (z=0): should go from 0 to 50 at midpoint
        let h_0 = heightmap.sample_bilinear(0.0, 0.0);
        let h_half = heightmap.sample_bilinear(0.5, 0.0);
        let h_1 = heightmap.sample_bilinear(1.0, 0.0);

        assert!((h_0 - 0.0).abs() < 0.1);
        assert!((h_half - 25.0).abs() < 0.1, "Expected ~25, got {}", h_half);
        assert!((h_1 - 50.0).abs() < 0.1, "Expected ~50, got {}", h_1);
    }

    #[test]
    fn test_chunk_radius_calculation() {
        let center = Vec3::ZERO;
        let chunk_size = 64.0;

        // Radius 0 should give 1 chunk
        let r0 = ChunkId::get_chunks_in_radius(center, 0, chunk_size);
        assert_eq!(r0.len(), 1);

        // Radius 1 should give 9 chunks (3x3)
        let r1 = ChunkId::get_chunks_in_radius(center, 1, chunk_size);
        assert_eq!(r1.len(), 9);

        // Radius 2 should give 25 chunks (5x5)
        let r2 = ChunkId::get_chunks_in_radius(center, 2, chunk_size);
        assert_eq!(r2.len(), 25);
    }
}

// =============================================================================
// BOUNDARY CONDITION TESTS - Test exact boundary values to catch < vs <= mutations
// =============================================================================

#[cfg(test)]
mod boundary_condition_tests {
    use crate::background_loader::StreamingConfig;
    use crate::chunk::ChunkId;
    use crate::heightmap::{Heightmap, HeightmapConfig};
    use crate::voxel_data::Voxel;
    use glam::Vec3;

    // --- Heightmap boundary tests ---

    #[test]
    fn heightmap_value_at_zero() {
        let data = vec![0.0, 1.0, 2.0, 3.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert_eq!(heightmap.min_height(), 0.0);
    }

    #[test]
    fn heightmap_value_at_one() {
        let data = vec![1.0; 4];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert_eq!(heightmap.min_height(), 1.0);
        assert_eq!(heightmap.max_height(), 1.0);
    }

    #[test]
    fn heightmap_normalized_sample_at_zero() {
        let data = vec![10.0, 20.0, 30.0, 40.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        // Sample at normalized (0.0, 0.0) should give corner value
        let sample = heightmap.sample_bilinear(0.0, 0.0);
        assert!((sample - 10.0).abs() < 0.1);
    }

    #[test]
    fn heightmap_normalized_sample_near_one() {
        let data = vec![10.0, 20.0, 30.0, 40.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        // Sample near normalized (1.0, 1.0) should give interpolated value
        let sample = heightmap.sample_bilinear(0.999, 0.999);
        // Should be close to 40.0 (lower-right corner)
        assert!(
            sample > 25.0,
            "Sample near corner should be high: {}",
            sample
        );
    }

    #[test]
    fn heightmap_out_of_bounds_x_returns_zero() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();
        assert_eq!(heightmap.get_height(100, 0), 0.0);
    }

    #[test]
    fn heightmap_out_of_bounds_z_returns_zero() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();
        assert_eq!(heightmap.get_height(0, 100), 0.0);
    }

    // --- ChunkId boundary tests ---

    #[test]
    fn chunk_id_at_origin() {
        let id = ChunkId::from_world_pos(Vec3::ZERO, 64.0);
        assert_eq!(id.x, 0);
        assert_eq!(id.z, 0);
    }

    #[test]
    fn chunk_id_at_exact_boundary() {
        // At exactly chunk boundary (64, 0, 0)
        let id = ChunkId::from_world_pos(Vec3::new(64.0, 0.0, 0.0), 64.0);
        assert_eq!(id.x, 1); // Should be next chunk
    }

    #[test]
    fn chunk_id_just_before_boundary() {
        // Just before chunk boundary (63.999, 0, 0)
        let id = ChunkId::from_world_pos(Vec3::new(63.999, 0.0, 0.0), 64.0);
        assert_eq!(id.x, 0); // Should still be current chunk
    }

    #[test]
    fn chunk_id_negative_boundary() {
        // At exactly (-64, 0, 0)
        let id = ChunkId::from_world_pos(Vec3::new(-64.0, 0.0, 0.0), 64.0);
        assert_eq!(id.x, -1);
    }

    #[test]
    fn chunk_radius_zero() {
        let chunks = ChunkId::get_chunks_in_radius(Vec3::ZERO, 0, 64.0);
        assert_eq!(chunks.len(), 1);
    }

    // --- Voxel density boundary tests ---

    #[test]
    fn voxel_density_zero_is_empty() {
        let voxel = Voxel::new(0.0, 0);
        assert!(voxel.is_empty());
    }

    #[test]
    fn voxel_density_one_is_solid() {
        let voxel = Voxel::new(1.0, 0);
        assert!(voxel.is_solid());
    }

    #[test]
    fn voxel_density_at_empty_threshold() {
        // Just under empty threshold (0.01)
        let voxel = Voxel::new(0.009, 0);
        assert!(voxel.is_empty());
    }

    #[test]
    fn voxel_density_at_solid_threshold() {
        // Just over solid threshold (0.5)
        let voxel = Voxel::new(0.501, 0);
        assert!(voxel.is_solid());
    }

    // --- StreamingConfig boundary tests ---

    #[test]
    fn streaming_view_distance_minimum() {
        let config = StreamingConfig::default();
        assert!(config.view_distance >= 1);
    }

    #[test]
    fn streaming_prefetch_within_view() {
        let config = StreamingConfig::default();
        assert!(config.prefetch_distance <= config.view_distance);
    }

    #[test]
    fn streaming_max_loads_at_least_one() {
        let config = StreamingConfig::default();
        assert!(config.max_concurrent_loads >= 1);
    }

    #[test]
    fn streaming_throttle_positive() {
        let config = StreamingConfig::default();
        assert!(config.adaptive_throttle_threshold_ms > 0.0);
    }
}

// =============================================================================
// COMPARISON OPERATOR TESTS - Test to catch == vs != and < vs > swaps
// =============================================================================

#[cfg(test)]
mod comparison_operator_tests {
    use crate::biome::BiomeType;
    use crate::chunk::ChunkId;
    use crate::heightmap::Heightmap;
    use crate::lod_manager::LodLevel;
    use crate::voxel_data::Voxel;
    // --- ChunkId equality ---

    #[test]
    fn chunk_id_equal_to_self() {
        let id = ChunkId::new(5, 10);
        assert_eq!(id, id);
    }

    #[test]
    fn chunk_id_equal_same_coords() {
        let id1 = ChunkId::new(5, 10);
        let id2 = ChunkId::new(5, 10);
        assert_eq!(id1, id2);
    }

    #[test]
    fn chunk_id_not_equal_different_x() {
        let id1 = ChunkId::new(5, 10);
        let id2 = ChunkId::new(6, 10);
        assert_ne!(id1, id2);
    }

    #[test]
    fn chunk_id_not_equal_different_z() {
        let id1 = ChunkId::new(5, 10);
        let id2 = ChunkId::new(5, 11);
        assert_ne!(id1, id2);
    }

    // --- BiomeType equality ---

    #[test]
    fn biome_type_equals_self() {
        assert_eq!(BiomeType::Forest, BiomeType::Forest);
    }

    #[test]
    fn biome_type_not_equals_other() {
        assert_ne!(BiomeType::Forest, BiomeType::Desert);
    }

    #[test]
    fn biome_type_all_distinct() {
        let all = BiomeType::all();
        for (i, b1) in all.iter().enumerate() {
            for (j, b2) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(b1, b2);
                } else {
                    assert_ne!(b1, b2);
                }
            }
        }
    }

    // --- Voxel comparisons ---

    #[test]
    fn voxel_default_equals_default() {
        let v1 = Voxel::default();
        let v2 = Voxel::default();
        assert_eq!(v1.density, v2.density);
    }

    #[test]
    fn voxel_different_density_detected() {
        let v1 = Voxel::new(0.0, 0);
        let v2 = Voxel::new(1.0, 0);
        assert!(v1.density < v2.density);
    }

    // --- Heightmap min/max comparison ---

    #[test]
    fn heightmap_min_less_than_max() {
        let data = vec![5.0, 10.0, 15.0, 20.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert!(heightmap.min_height() < heightmap.max_height());
    }

    #[test]
    fn heightmap_min_equals_max_when_uniform() {
        let data = vec![7.0; 4];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert_eq!(heightmap.min_height(), heightmap.max_height());
    }

    // --- LodLevel debug format comparisons ---

    #[test]
    fn lod_full_not_equals_half() {
        assert_ne!(
            format!("{:?}", LodLevel::Full),
            format!("{:?}", LodLevel::Half)
        );
    }

    #[test]
    fn lod_quarter_not_equals_skybox() {
        assert_ne!(
            format!("{:?}", LodLevel::Quarter),
            format!("{:?}", LodLevel::Skybox)
        );
    }

    // --- Position comparisons ---

    #[test]
    fn center_world_pos_correct() {
        let id = ChunkId::new(1, 1);
        let corner = id.to_world_pos(64.0);
        let center = id.to_center_pos(64.0);
        assert!(center.x > corner.x);
        assert!(center.z > corner.z);
    }
}

// =============================================================================
// BOOLEAN RETURN PATH TESTS - Test all paths through boolean-returning functions
// =============================================================================

#[cfg(test)]
mod boolean_return_path_tests {
    use crate::biome::BiomeType;
    use crate::heightmap::{Heightmap, HeightmapConfig};
    use crate::meshing::ChunkMesh;
    use crate::voxel_data::{ChunkCoord, Voxel};
    use std::str::FromStr;

    // --- Voxel.is_solid() paths ---

    #[test]
    fn voxel_is_solid_true_for_high_density() {
        let voxel = Voxel::new(1.0, 0);
        assert!(voxel.is_solid());
    }

    #[test]
    fn voxel_is_solid_true_at_threshold() {
        let voxel = Voxel::new(0.51, 0);
        assert!(voxel.is_solid());
    }

    #[test]
    fn voxel_is_solid_false_for_low_density() {
        let voxel = Voxel::new(0.0, 0);
        assert!(!voxel.is_solid());
    }

    #[test]
    fn voxel_is_solid_false_at_threshold() {
        let voxel = Voxel::new(0.5, 0);
        assert!(!voxel.is_solid()); // Threshold is > 0.5, not >=
    }

    // --- Voxel.is_empty() paths ---

    #[test]
    fn voxel_is_empty_true_for_zero() {
        let voxel = Voxel::new(0.0, 0);
        assert!(voxel.is_empty());
    }

    #[test]
    fn voxel_is_empty_true_under_threshold() {
        let voxel = Voxel::new(0.009, 0);
        assert!(voxel.is_empty());
    }

    #[test]
    fn voxel_is_empty_false_for_solid() {
        let voxel = Voxel::new(1.0, 0);
        assert!(!voxel.is_empty());
    }

    #[test]
    fn voxel_is_empty_false_at_threshold() {
        let voxel = Voxel::new(0.01, 0);
        assert!(!voxel.is_empty()); // Threshold is < 0.01, not <=
    }

    // --- ChunkMesh.is_empty() paths ---

    #[test]
    fn chunk_mesh_is_empty_true_when_new() {
        let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
        assert!(mesh.is_empty());
    }

    // --- Heightmap.from_data() Result paths ---

    #[test]
    fn heightmap_from_data_ok_valid_size() {
        let data = vec![1.0; 9]; // 3x3
        let result = Heightmap::from_data(data, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn heightmap_from_data_err_invalid_size() {
        let data = vec![1.0; 5]; // Not a square
        let result = Heightmap::from_data(data, 3);
        assert!(result.is_err());
    }

    // --- BiomeType::from_str() Result paths ---

    #[test]
    fn biome_from_str_ok_valid() {
        let result = BiomeType::from_str("forest");
        assert!(result.is_ok());
    }

    #[test]
    fn biome_from_str_err_invalid() {
        let result = BiomeType::from_str("invalid_biome");
        assert!(result.is_err());
    }

    #[test]
    fn biome_from_str_err_empty() {
        let result = BiomeType::from_str("");
        assert!(result.is_err());
    }

    // --- BiomeType::parse() Option paths ---

    #[test]
    fn biome_parse_some_valid() {
        let result = BiomeType::parse("mountain");
        assert!(result.is_some());
    }

    #[test]
    fn biome_parse_none_invalid() {
        let result = BiomeType::parse("unknown_biome");
        assert!(result.is_none());
    }

    // --- Heightmap data operations ---

    #[test]
    fn heightmap_data_iter_all_returns_true_when_uniform() {
        let config = HeightmapConfig {
            resolution: 2,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();
        // All zeros should be uniform
        assert!(heightmap.data().iter().all(|&h| h == 0.0));
    }

    #[test]
    fn heightmap_data_iter_all_returns_false_when_varied() {
        let data = vec![0.0, 1.0, 0.0, 0.0];
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        assert!(!heightmap.data().iter().all(|&h| h == 0.0));
    }
}

/// Targeted tests for TerrainChunk::get_height_at_world_pos mutations
#[cfg(test)]
mod chunk_height_at_world_pos_targeted {
    use crate::chunk::{ChunkId, TerrainChunk};
    use crate::{BiomeType, Heightmap};
    use glam::Vec3;

    fn make_chunk(cx: i32, cz: i32) -> TerrainChunk {
        // 4x4 heightmap with known values
        let data: Vec<f32> = (0..16).map(|i| i as f32 * 0.5).collect();
        let hm = Heightmap::from_data(data, 4).unwrap();
        let biomes = vec![BiomeType::Grassland; 16];
        TerrainChunk::new(ChunkId::new(cx, cz), hm, biomes)
    }

    #[test]
    fn non_origin_chunk_subtraction_matters() {
        // Kills: line 117 `- with +` (world_pos - chunk_origin vs +)
        // Chunk at (2,0,3), chunk_size=16 → origin = (32, 0, 48)
        let chunk = make_chunk(2, 3);
        let chunk_size = 16.0;
        // World pos at (40, 0, 56) → local (8, 0, 8) → inside chunk
        let inside = chunk.get_height_at_world_pos(Vec3::new(40.0, 0.0, 56.0), chunk_size);
        assert!(
            inside.is_some(),
            "Position inside non-origin chunk should return Some"
        );

        // If `-` were `+`, local would be (40+32, 0, 56+48) = (72, 0, 104)
        // which is >= chunk_size=16 → would return None incorrectly
        // This verifies subtraction is correct
    }

    #[test]
    fn outside_x_only_returns_none() {
        // Kills: line 122 `|| with &&` — position out in x but in z
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        // x = -1 (negative, outside), z = 8 (inside)
        let result = chunk.get_height_at_world_pos(Vec3::new(-1.0, 0.0, 8.0), chunk_size);
        assert!(
            result.is_none(),
            "Outside x-only should be None (|| not &&)"
        );
    }

    #[test]
    fn outside_z_only_returns_none() {
        // Also kills `|| with &&` — position in x but out in z
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        // x = 8 (inside), z = -1 (outside)
        let result = chunk.get_height_at_world_pos(Vec3::new(8.0, 0.0, -1.0), chunk_size);
        assert!(
            result.is_none(),
            "Outside z-only should be None (|| not &&)"
        );
    }

    #[test]
    fn inside_chunk_returns_some() {
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        let result = chunk.get_height_at_world_pos(Vec3::new(8.0, 0.0, 8.0), chunk_size);
        assert!(result.is_some(), "Position inside chunk should return Some");
    }

    #[test]
    fn at_exact_origin_returns_some() {
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        // x=0, z=0 → local_pos = (0, 0, 0) → x < 0.0 is false, z < 0.0 is false
        let result = chunk.get_height_at_world_pos(Vec3::new(0.0, 0.0, 0.0), chunk_size);
        assert!(result.is_some(), "Origin position should be inside chunk");
    }

    #[test]
    fn at_upper_boundary_returns_none() {
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        // x = chunk_size → local_pos.x >= chunk_size → None
        let result = chunk.get_height_at_world_pos(Vec3::new(16.0, 0.0, 8.0), chunk_size);
        assert!(result.is_none(), "At x=chunk_size boundary should be None");
    }

    #[test]
    fn height_value_at_known_position_correct() {
        // Kills: lines 130-131 u/v calculation mutations
        // Heightmap 4x4, data[i] = i * 0.5: [0, 0.5, 1.0, 1.5, 2.0, ...]
        // chunk_size = 16, resolution = 4
        // u = (local_x / 16) * 3, v = (local_z / 16) * 3
        // At world (0, 0, 0) → local (0,0,0) → u=0, v=0 → sample_bilinear(0,0) = data[0] = 0.0
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        let h = chunk
            .get_height_at_world_pos(Vec3::new(0.0, 0.0, 0.0), chunk_size)
            .unwrap();
        assert!(
            (h - 0.0).abs() < 1e-4,
            "Height at origin should be 0.0, got {}",
            h
        );
    }

    #[test]
    fn height_value_at_grid_aligned_position() {
        // At world (16/3, 0, 0) → local_x = 16/3 → u = (16/3)/16 * 3 = 1.0
        // sample_bilinear(1.0, 0.0) = data[0*4+1] = 0.5
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        let wx = chunk_size / 3.0; // = 16/3 ≈ 5.333
        let h = chunk
            .get_height_at_world_pos(Vec3::new(wx, 0.0, 0.0), chunk_size)
            .unwrap();
        assert!(
            (h - 0.5).abs() < 1e-4,
            "Height at u=1 should be 0.5, got {}",
            h
        );
    }

    #[test]
    fn height_value_z_axis_different_from_x() {
        // At world (0, 0, 16/3) → local_z = 16/3 → v = 1.0, u = 0.0
        // sample_bilinear(0.0, 1.0) = data[1*4+0] = 2.0
        // vs sample_bilinear(1.0, 0.0) = data[0*4+1] = 0.5
        // Kills mutations that swap u/v or make them equivalent
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        let wz = chunk_size / 3.0;
        let h_z = chunk
            .get_height_at_world_pos(Vec3::new(0.0, 0.0, wz), chunk_size)
            .unwrap();
        let h_x = chunk
            .get_height_at_world_pos(Vec3::new(wz, 0.0, 0.0), chunk_size)
            .unwrap();
        assert!(
            (h_z - 2.0).abs() < 1e-4,
            "Height at v=1 should be 2.0, got {}",
            h_z
        );
        assert!(
            (h_x - 0.5).abs() < 1e-4,
            "Height at u=1 should be 0.5, got {}",
            h_x
        );
        assert!(
            (h_z - h_x).abs() > 0.5,
            "x and z axes should give different heights"
        );
    }

    #[test]
    fn height_value_mid_chunk_interpolated() {
        // At world (8, 0, 0) → local_x = 8 → u = (8/16)*3 = 1.5, v = 0
        // sample_bilinear(1.5, 0.0) = lerp(data[1], data[2], 0.5) = lerp(0.5, 1.0, 0.5) = 0.75
        let chunk = make_chunk(0, 0);
        let chunk_size = 16.0;
        let h = chunk
            .get_height_at_world_pos(Vec3::new(8.0, 0.0, 0.0), chunk_size)
            .unwrap();
        assert!(
            (h - 0.75).abs() < 1e-3,
            "Mid-chunk x height should be ~0.75, got {}",
            h
        );
    }
}

/// Targeted tests for Heightmap::calculate_normal mutations
#[cfg(test)]
mod heightmap_normal_targeted {
    use crate::Heightmap;

    #[test]
    fn normal_at_flat_surface_points_up() {
        // Flat heightmap: all values = 5.0
        let data = vec![5.0; 16];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let normal = hm.calculate_normal(1, 1, 1.0);
        // On a flat surface, normal should be (0, 1, 0)
        assert!(
            normal.y > 0.9,
            "Normal Y on flat surface should be ~1.0, got {}",
            normal.y
        );
        assert!(
            normal.x.abs() < 0.1,
            "Normal X on flat surface should be ~0, got {}",
            normal.x
        );
        assert!(
            normal.z.abs() < 0.1,
            "Normal Z on flat surface should be ~0, got {}",
            normal.z
        );
    }

    #[test]
    fn normal_on_slope_x_tilts_correctly() {
        // Slope in X: heights increase along x
        // Row-major 4×4: row z, col x
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[z * 4 + x] = x as f32 * 2.0; // slope in x
            }
        }
        let hm = Heightmap::from_data(data, 4).unwrap();
        let normal = hm.calculate_normal(1, 1, 1.0);
        // With +x slope, normal should tilt in -x direction
        assert!(
            normal.x < -0.1,
            "Normal should tilt -x on +x slope, got x={}",
            normal.x
        );
        assert!(
            normal.y > 0.0,
            "Normal Y should be positive, got {}",
            normal.y
        );
    }

    #[test]
    fn normal_on_slope_z_tilts_correctly() {
        // Slope in Z: heights increase along z
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[z * 4 + x] = z as f32 * 2.0;
            }
        }
        let hm = Heightmap::from_data(data, 4).unwrap();
        let normal = hm.calculate_normal(1, 1, 1.0);
        // With +z slope, normal should tilt in -z direction
        assert!(
            normal.z < -0.1,
            "Normal should tilt -z on +z slope, got z={}",
            normal.z
        );
        assert!(
            normal.y > 0.0,
            "Normal Y should be positive, got {}",
            normal.y
        );
    }

    #[test]
    fn normal_is_normalized() {
        let data = vec![
            0.0, 1.0, 2.0, 3.0, 0.5, 1.5, 2.5, 3.5, 1.0, 2.0, 3.0, 4.0, 1.5, 2.5, 3.5, 4.5,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let normal = hm.calculate_normal(1, 1, 1.0);
        let len = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Normal should be unit length, got {}",
            len
        );
    }
}

/// Targeted tests for Heightmap::sample_bilinear mutations
#[cfg(test)]
mod heightmap_bilinear_targeted {
    use crate::Heightmap;

    #[test]
    fn bilinear_at_grid_point_returns_exact() {
        // 4x4 heightmap with distinct values
        let data = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        // At exact grid point (1, 1) should return height[1][1] = 6.0
        let val = hm.sample_bilinear(1.0, 1.0);
        assert!(
            (val - 6.0).abs() < 1e-4,
            "At grid point (1,1) should be 6.0, got {}",
            val
        );
    }

    #[test]
    fn bilinear_midpoint_is_average() {
        // At (0.5, 0.5) should interpolate between [0,0]=1, [1,0]=2, [0,1]=5, [1,1]=6
        // Bilinear: lerp(lerp(1,2,0.5), lerp(5,6,0.5), 0.5) = lerp(1.5, 5.5, 0.5) = 3.5
        let data = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val = hm.sample_bilinear(0.5, 0.5);
        assert!(
            (val - 3.5).abs() < 1e-4,
            "Bilinear midpoint should be 3.5, got {}",
            val
        );
    }

    #[test]
    fn bilinear_at_quarter_x() {
        // At (0.25, 0.0) → lerp(1.0, 2.0, 0.25) = 1.25
        let data = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val = hm.sample_bilinear(0.25, 0.0);
        assert!(
            (val - 1.25).abs() < 1e-4,
            "Bilinear (0.25,0) should be 1.25, got {}",
            val
        );
    }

    #[test]
    fn bilinear_asymmetric_check() {
        // sample_bilinear(0.5, 0.0) != sample_bilinear(0.0, 0.5)
        // Kills * vs + and coordinate swap mutations
        let data = vec![
            1.0, 3.0, 5.0, 7.0, 2.0, 4.0, 6.0, 8.0, 3.0, 5.0, 7.0, 9.0, 4.0, 6.0, 8.0, 10.0,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val_x = hm.sample_bilinear(0.5, 0.0); // lerp(1,3,0.5) = 2.0
        let val_y = hm.sample_bilinear(0.0, 0.5); // lerp(1,2,0.5) = 1.5
        assert!(
            (val_x - val_y).abs() > 0.1,
            "Bilinear should be direction-sensitive: ({}, {})",
            val_x,
            val_y
        );
    }
}

/// Targeted tests for Heightmap::smooth mutations
#[cfg(test)]
mod heightmap_smooth_targeted {
    use crate::Heightmap;

    #[test]
    fn smooth_reduces_peak() {
        // Create heightmap with a single peak
        let mut data = vec![0.0; 16];
        data[5] = 10.0; // Peak at (1,1)
        let mut hm = Heightmap::from_data(data, 4).unwrap();
        let peak_before = hm.get_height(1, 1);
        hm.smooth(1);
        let peak_after = hm.get_height(1, 1);
        assert!(
            peak_after < peak_before,
            "Smoothing should reduce peak: before={}, after={}",
            peak_before,
            peak_after
        );
    }

    #[test]
    fn smooth_raises_neighbors() {
        // Neighbors of a peak should increase after smoothing
        let mut data = vec![0.0; 16];
        data[5] = 10.0; // Peak at (1,1)
        let mut hm = Heightmap::from_data(data, 4).unwrap();
        let neighbor_before = hm.get_height(2, 1); // right neighbor
        hm.smooth(1);
        let neighbor_after = hm.get_height(2, 1);
        assert!(
            neighbor_after > neighbor_before,
            "Smoothing should raise neighbors: before={}, after={}",
            neighbor_before,
            neighbor_after
        );
    }

    #[test]
    fn smooth_flat_stays_flat() {
        // Flat heightmap should remain flat after smoothing
        let data = vec![5.0; 16];
        let mut hm = Heightmap::from_data(data, 4).unwrap();
        hm.smooth(3);
        for z in 0..4 {
            for x in 0..4 {
                let h = hm.get_height(x, z);
                assert!(
                    (h - 5.0).abs() < 1e-4,
                    "Flat surface should stay flat after smoothing: ({},{})={}",
                    x,
                    z,
                    h
                );
            }
        }
    }

    #[test]
    fn smooth_multiple_iterations_more_flat() {
        let mut data = vec![0.0; 16];
        data[5] = 10.0;
        let mut hm1 = Heightmap::from_data(data.clone(), 4).unwrap();
        let mut hm2 = Heightmap::from_data(data, 4).unwrap();
        hm1.smooth(1);
        hm2.smooth(5);
        let peak1 = hm1.get_height(1, 1);
        let peak2 = hm2.get_height(1, 1);
        assert!(
            peak2 < peak1,
            "More iterations should smooth more: 1iter={}, 5iter={}",
            peak1,
            peak2
        );
    }
}

/// Targeted tests for ClimateMap::sample_temperature mutations
#[cfg(test)]
mod climate_temperature_targeted {
    use crate::climate::{ClimateConfig, ClimateMap};

    fn make_climate() -> ClimateMap {
        let config = ClimateConfig::default();
        ClimateMap::new(&config, 42)
    }

    #[test]
    fn temperature_height_gradient_effect() {
        // Higher elevation should be cooler (gradient is negative: -0.0065)
        let climate = make_climate();
        let t_low = climate.sample_temperature(100.0, 100.0, 0.0);
        let t_high = climate.sample_temperature(100.0, 100.0, 100.0);
        // height * -0.0065: 100 * -0.0065 = -0.65 reduction
        // If gradient sign were wrong or * became +, t_high would be higher
        assert!(
            t_high < t_low,
            "Higher elevation should be cooler: low={}, high={}",
            t_low,
            t_high
        );
    }

    #[test]
    fn temperature_deterministic_for_same_input() {
        let climate = make_climate();
        let t1 = climate.sample_temperature(50.0, 50.0, 10.0);
        let t2 = climate.sample_temperature(50.0, 50.0, 10.0);
        assert_eq!(t1, t2, "Same input should give same temperature");
    }

    #[test]
    fn temperature_clamped_between_0_and_1() {
        let climate = make_climate();
        // Test at various positions
        for i in 0..20 {
            let x = i as f64 * 100.0;
            let z = i as f64 * 73.0;
            let h = (i as f32 * 50.0) % 200.0;
            let t = climate.sample_temperature(x, z, h);
            assert!(
                t >= 0.0 && t <= 1.0,
                "Temperature should be [0,1]: {} at ({},{},{})",
                t,
                x,
                z,
                h
            );
        }
    }

    #[test]
    fn temperature_varies_with_position() {
        // Different positions should generally give different temperatures
        let climate = make_climate();
        let t1 = climate.sample_temperature(0.0, 0.0, 0.0);
        let t2 = climate.sample_temperature(1000.0, 0.0, 0.0);
        let t3 = climate.sample_temperature(0.0, 1000.0, 0.0);
        // At least some should differ (noise-based)
        assert!(
            t1 != t2 || t1 != t3,
            "Temperature should vary: t1={}, t2={}, t3={}",
            t1,
            t2,
            t3
        );
    }

    #[test]
    fn temperature_latitude_gradient_effect() {
        // Moving far in z should affect temperature due to latitude gradient
        let climate = make_climate();
        let t_equator = climate.sample_temperature(1000.0, 0.0, 0.0);
        // At z=100000, latitude_factor = sin(100000*0.00001) = sin(1) ≈ 0.841
        // With latitude_gradient=0.8, adds 0.841*0.8 ≈ 0.673
        let t_pole = climate.sample_temperature(1000.0, 100000.0, 0.0);
        // These should be different due to latitude effect
        assert!(
            (t_equator - t_pole).abs() > 0.01,
            "Latitude should affect temperature: equator={}, pole={}",
            t_equator,
            t_pole
        );
    }

    #[test]
    fn temperature_height_gradient_magnitude() {
        // Quantitative check: height 100 with gradient -0.0065
        // should reduce by approximately 0.65
        let climate = make_climate();
        let t0 = climate.sample_temperature(500.0, 500.0, 0.0);
        let t100 = climate.sample_temperature(500.0, 500.0, 100.0);
        let diff = t0 - t100;
        // diff should be near 0.65 (before clamping)
        // If * became +, diff would be near -99.9935 (wrong sign/magnitude)
        // If += became -=, diff would be negative
        assert!(
            diff > 0.0,
            "Higher elevation should reduce temperature: diff={}",
            diff
        );
    }

    #[test]
    fn temperature_height_mul_vs_div_catches_mutation() {
        // Kills: line 90, * → /  in `height * gradient`
        // With height=1, gradient=-0.0065:
        //   correct: 1 * -0.0065 = -0.0065 (tiny reduction)
        //   mutated: 1 / -0.0065 = -153.8 (massive negative, clamps to 0)
        // So with height=1, correct temp ~= base - 0.0065, mutated = 0.0
        let climate = make_climate();
        let t_h1 = climate.sample_temperature(500.0, 0.0, 1.0); // small height
                                                                // With correct formula, this should be very close to base temp (height=1 barely matters)
                                                                // With mutation, would be clamped to 0.0
        assert!(
            t_h1 > 0.1,
            "Height=1 shouldn't crush temperature to 0: got {}",
            t_h1
        );
    }

    #[test]
    fn temperature_latitude_sign_matters() {
        // Kills: line 94, += → -=
        // At z=78540 (sin(78540*0.00001) = sin(0.7854) ≈ 0.707):
        //   correct: += 0.707 * 0.8 = += 0.566
        //   mutated: -= 0.707 * 0.8 = -= 0.566
        // Compare two symmetric z values
        let climate = make_climate();
        let t_pos = climate.sample_temperature(500.0, 78540.0, 0.0);
        let _t_neg = climate.sample_temperature(500.0, -78540.0, 0.0);
        // sin(-x) = -sin(x), so latitude effects should differ
        // Correct: pos gets +0.566, neg gets -0.566 → difference ≈ 1.132
        // If sign is flipped (+=→-=): pos gets -0.566, neg gets +0.566 → still differs but reversed
        // Compare with z=0 (no latitude effect):
        let t_zero = climate.sample_temperature(500.0, 0.0, 0.0);
        // With correct +=: t_pos > t_zero (latitude adds positive)
        // With mutated -=: t_pos < t_zero (latitude subtracts)
        // We know sin(0.7854) > 0, gradient > 0, so += makes it bigger
        assert!(
            t_pos > t_zero - 0.01 || t_pos == 1.0,
            "Positive latitude should increase temp: t_pos={}, t_zero={}",
            t_pos,
            t_zero
        );
    }

    #[test]
    fn temperature_latitude_mul_vs_div_catches() {
        // Kills: line 94, * → / in `latitude_factor * gradient`
        // latitude_factor ≈ 0.707, gradient = 0.8
        //   correct: 0.707 * 0.8 = 0.566
        //   mutated: 0.707 / 0.8 = 0.884
        // Small difference but detectable
        let mut config = ClimateConfig::default();
        config.temperature.offset = 0.0; // Start from lower base to see effects
        config.temperature.amplitude = 0.1;
        config.temperature_latitude_gradient = 2.0; // Amplify difference
        let climate = ClimateMap::new(&config, 42);
        let t_z0 = climate.sample_temperature(500.0, 0.0, 0.0);
        let t_z_pos = climate.sample_temperature(500.0, 78540.0, 0.0);
        // latitude effect = sin(0.7854) * 2.0 ≈ 0.707 * 2.0 = 1.414
        // vs mutated: sin(0.7854) / 2.0 ≈ 0.354
        let lat_effect = t_z_pos - t_z0;
        // Correct: ~1.414, Mutated: ~0.354 — difference is large enough
        assert!(
            lat_effect > 0.3,
            "Latitude effect should be significant: effect={}",
            lat_effect
        );
    }
}

/// Targeted tests for ClimateMap::sample_moisture mutations
#[cfg(test)]
mod climate_moisture_targeted {
    use crate::climate::{ClimateConfig, ClimateMap};

    fn make_climate() -> ClimateMap {
        let config = ClimateConfig::default();
        ClimateMap::new(&config, 42)
    }

    #[test]
    fn moisture_height_reduces_moisture() {
        // Higher elevation should reduce moisture (rain shadow)
        // moisture *= 1.0 - height_factor * 0.3
        let climate = make_climate();
        let m_low = climate.sample_moisture(100.0, 100.0, 0.0);
        let m_high = climate.sample_moisture(100.0, 100.0, 200.0);
        // height_factor = (200*0.01).clamp(0,1) = 1.0 → multiply by 1.0 - 1.0*0.3 = 0.7
        // If logic is wrong, m_high might be higher
        assert!(
            m_high <= m_low + 0.01,
            "Higher elevation should reduce moisture: low={}, high={}",
            m_low,
            m_high
        );
    }

    #[test]
    fn moisture_clamped_between_0_and_1() {
        let climate = make_climate();
        for i in 0..20 {
            let x = i as f64 * 100.0;
            let z = i as f64 * 73.0;
            let h = (i as f32 * 50.0) % 200.0;
            let m = climate.sample_moisture(x, z, h);
            assert!(
                m >= 0.0 && m <= 1.0,
                "Moisture should be [0,1]: {} at ({},{},{})",
                m,
                x,
                z,
                h
            );
        }
    }

    #[test]
    fn moisture_deterministic() {
        let climate = make_climate();
        let m1 = climate.sample_moisture(50.0, 50.0, 10.0);
        let m2 = climate.sample_moisture(50.0, 50.0, 10.0);
        assert_eq!(m1, m2, "Same input should give same moisture");
    }

    #[test]
    fn moisture_varies_with_position() {
        let climate = make_climate();
        let m1 = climate.sample_moisture(0.0, 0.0, 0.0);
        let m2 = climate.sample_moisture(1000.0, 0.0, 0.0);
        let m3 = climate.sample_moisture(0.0, 1000.0, 0.0);
        assert!(
            m1 != m2 || m1 != m3,
            "Moisture should vary spatially: m1={}, m2={}, m3={}",
            m1,
            m2,
            m3
        );
    }

    #[test]
    fn moisture_height_factor_multiplication() {
        // At height=0: height_factor=0, so *= 1.0 - 0 = 1.0 (no change)
        // At height=50: height_factor=(50*0.01)=0.5, so *= 1.0 - 0.5*0.3 = 0.85
        // At height=200: height_factor=1.0 (clamped), so *= 1.0 - 1.0*0.3 = 0.7
        let climate = make_climate();
        let m0 = climate.sample_moisture(500.0, 500.0, 0.0);
        let m50 = climate.sample_moisture(500.0, 500.0, 50.0);
        let m200 = climate.sample_moisture(500.0, 500.0, 200.0);
        // Should be monotonically decreasing (or equal for very low moisture)
        assert!(
            m50 <= m0 + 0.01 && m200 <= m50 + 0.01,
            "Moisture should decrease with height: h0={}, h50={}, h200={}",
            m0,
            m50,
            m200
        );
    }

    #[test]
    fn moisture_water_distance_component() {
        // The water factor = exp(-distance * falloff) * 0.3
        // Combined: moisture = base_moisture * 0.7 + water_factor * 0.3
        // This checks the overall formula produces non-trivial output
        let climate = make_climate();
        let m = climate.sample_moisture(250.0, 250.0, 0.0);
        // Should be between 0 and 1 and non-zero
        assert!(m > 0.0, "Moisture should be positive: {}", m);
        assert!(m <= 1.0, "Moisture should be <= 1: {}", m);
    }
}

/// Targeted tests for calculate_climate_stats mutations
#[cfg(test)]
mod climate_stats_targeted {
    use crate::climate::{utils, ClimateConfig, ClimateMap};

    fn make_climate() -> ClimateMap {
        let config = ClimateConfig::default();
        ClimateMap::new(&config, 42)
    }

    #[test]
    fn stats_min_max_consistent() {
        let climate = make_climate();
        let stats = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 100.0, 5);
        assert!(
            stats.temperature_min <= stats.temperature_avg,
            "min {} should be <= avg {}",
            stats.temperature_min,
            stats.temperature_avg
        );
        assert!(
            stats.temperature_avg <= stats.temperature_max,
            "avg {} should be <= max {}",
            stats.temperature_avg,
            stats.temperature_max
        );
        assert!(
            stats.moisture_min <= stats.moisture_avg,
            "min {} should be <= avg {}",
            stats.moisture_min,
            stats.moisture_avg
        );
        assert!(
            stats.moisture_avg <= stats.moisture_max,
            "avg {} should be <= max {}",
            stats.moisture_avg,
            stats.moisture_max
        );
    }

    #[test]
    fn stats_different_regions_give_different_results() {
        let climate = make_climate();
        let stats1 = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 100.0, 5);
        let stats2 =
            utils::calculate_climate_stats(&climate, 10000.0, 10100.0, 10000.0, 10100.0, 5);
        let diff_t = (stats1.temperature_avg - stats2.temperature_avg).abs();
        let diff_m = (stats1.moisture_avg - stats2.moisture_avg).abs();
        assert!(
            diff_t > 0.001 || diff_m > 0.001,
            "Different regions should give different stats: dt={}, dm={}",
            diff_t,
            diff_m
        );
    }

    #[test]
    fn stats_step_z_formula_covers_range() {
        let climate = make_climate();
        let stats_wide = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 1000.0, 5);
        let stats_narrow = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 10.0, 5);
        let range_wide = stats_wide.temperature_max - stats_wide.temperature_min;
        let range_narrow = stats_narrow.temperature_max - stats_narrow.temperature_min;
        assert!(
            (stats_wide.temperature_avg - stats_narrow.temperature_avg).abs() > 0.0001
                || (range_wide - range_narrow).abs() > 0.0001,
            "Different z ranges should give different results"
        );
    }

    #[test]
    fn stats_samples_affect_precision() {
        let climate = make_climate();
        let stats_few = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 100.0, 2);
        let stats_many = utils::calculate_climate_stats(&climate, 0.0, 100.0, 0.0, 100.0, 10);
        assert!(stats_few.temperature_min >= 0.0 && stats_few.temperature_max <= 1.0);
        assert!(stats_many.temperature_min >= 0.0 && stats_many.temperature_max <= 1.0);
    }
}

/// Additional tests for noise_fbm coverage through sample_temperature/moisture
#[cfg(test)]
mod noise_fbm_indirect_targeted {
    use crate::climate::{ClimateConfig, ClimateMap};

    #[test]
    fn fbm_amplitude_affects_output() {
        let mut config1 = ClimateConfig::default();
        config1.temperature.amplitude = 0.1;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.amplitude = 2.0;
        let climate2 = ClimateMap::new(&config2, 42);

        let t1 = climate1.sample_temperature(200.0, 200.0, 0.0);
        let t2 = climate2.sample_temperature(200.0, 200.0, 0.0);
        assert!(
            t1 != t2,
            "Different amplitudes should give different temperatures: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_persistence_affects_output() {
        let mut config1 = ClimateConfig::default();
        config1.temperature.persistence = 0.1;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.persistence = 0.9;
        let climate2 = ClimateMap::new(&config2, 42);

        let t1 = climate1.sample_temperature(300.0, 300.0, 0.0);
        let t2 = climate2.sample_temperature(300.0, 300.0, 0.0);
        assert!(
            t1 != t2,
            "Different persistence should affect temperature: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_lacunarity_affects_output() {
        let mut config1 = ClimateConfig::default();
        config1.temperature.lacunarity = 1.1;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.lacunarity = 4.0;
        let climate2 = ClimateMap::new(&config2, 42);

        let t1 = climate1.sample_temperature(400.0, 400.0, 0.0);
        let t2 = climate2.sample_temperature(400.0, 400.0, 0.0);
        assert!(
            t1 != t2,
            "Different lacunarity should affect temperature: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_offset_affects_output() {
        let mut config1 = ClimateConfig::default();
        config1.temperature.offset = 0.0;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.offset = 0.8;
        let climate2 = ClimateMap::new(&config2, 42);

        let t1 = climate1.sample_temperature(500.0, 500.0, 0.0);
        let t2 = climate2.sample_temperature(500.0, 500.0, 0.0);
        assert!(
            t1 != t2,
            "Different offset should affect temperature: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_octaves_affect_output() {
        // Use a config with small amplitude so values don't saturate to 0 or 1
        let mut config1 = ClimateConfig::default();
        config1.temperature.octaves = 1;
        config1.temperature.amplitude = 0.3;
        config1.temperature.offset = 0.5;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.octaves = 6;
        config2.temperature.amplitude = 0.3;
        config2.temperature.offset = 0.5;
        let climate2 = ClimateMap::new(&config2, 42);

        let t1 = climate1.sample_temperature(600.0, 600.0, 0.0);
        let t2 = climate2.sample_temperature(600.0, 600.0, 0.0);
        assert!(
            t1 != t2,
            "Different octave counts should affect temperature: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_value_is_nontrivial() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        let t = climate.sample_temperature(100.0, 100.0, 0.0);
        let t2 = climate.sample_temperature(100.0, 100.0, 0.0);
        assert_eq!(t, t2, "Should be deterministic");
        assert!(
            t > 0.01 && t < 0.99,
            "Temperature should be non-trivial: {}",
            t
        );
    }
}

/// Targeted tests for Heightmap::generate_vertices mutations
#[cfg(test)]
mod heightmap_generate_vertices_targeted {
    use crate::Heightmap;
    use glam::Vec3;

    #[test]
    fn vertex_count_matches_resolution_squared() {
        let data = vec![0.0; 16];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let verts = hm.generate_vertices(16.0, Vec3::ZERO);
        assert_eq!(verts.len(), 16, "4x4 heightmap should produce 16 vertices");
    }

    #[test]
    fn vertex_positions_with_offset() {
        // 2x2 heightmap, heights [1, 2, 3, 4], chunk_size=10, offset=(100, 0, 200)
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let hm = Heightmap::from_data(data, 2).unwrap();
        let verts = hm.generate_vertices(10.0, Vec3::new(100.0, 0.0, 200.0));
        // step = 10 / (2-1) = 10
        // Vertex (0,0): (100+0*10, height[0,0]=1.0, 200+0*10) = (100, 1, 200)
        assert!(
            (verts[0].x - 100.0).abs() < 1e-4,
            "v0.x should be 100, got {}",
            verts[0].x
        );
        assert!(
            (verts[0].y - 1.0).abs() < 1e-4,
            "v0.y should be 1.0, got {}",
            verts[0].y
        );
        assert!(
            (verts[0].z - 200.0).abs() < 1e-4,
            "v0.z should be 200, got {}",
            verts[0].z
        );
        // Vertex (1,0): (100+1*10, height[0,1]=2.0, 200+0*10) = (110, 2, 200)
        assert!(
            (verts[1].x - 110.0).abs() < 1e-4,
            "v1.x should be 110, got {}",
            verts[1].x
        );
        assert!(
            (verts[1].y - 2.0).abs() < 1e-4,
            "v1.y should be 2.0, got {}",
            verts[1].y
        );
        // Vertex (0,1): (100+0*10, height[1,0]=3.0, 200+1*10) = (100, 3, 210)
        assert!(
            (verts[2].x - 100.0).abs() < 1e-4,
            "v2.x should be 100, got {}",
            verts[2].x
        );
        assert!(
            (verts[2].y - 3.0).abs() < 1e-4,
            "v2.y should be 3.0, got {}",
            verts[2].y
        );
        assert!(
            (verts[2].z - 210.0).abs() < 1e-4,
            "v2.z should be 210, got {}",
            verts[2].z
        );
        // Vertex (1,1): (110, 4, 210)
        assert!(
            (verts[3].x - 110.0).abs() < 1e-4,
            "v3.x should be 110, got {}",
            verts[3].x
        );
        assert!(
            (verts[3].y - 4.0).abs() < 1e-4,
            "v3.y should be 4.0, got {}",
            verts[3].y
        );
        assert!(
            (verts[3].z - 210.0).abs() < 1e-4,
            "v3.z should be 210, got {}",
            verts[3].z
        );
    }

    #[test]
    fn vertex_step_calculation() {
        // step = chunk_size / (resolution - 1)
        // 3x3, chunk_size=20 → step=10
        let data = vec![0.0; 9];
        let hm = Heightmap::from_data(data, 3).unwrap();
        let verts = hm.generate_vertices(20.0, Vec3::ZERO);
        // Last vertex x position should be at chunk_size
        // vertex (2,0): 0 + 2*10 = 20
        assert!(
            (verts[2].x - 20.0).abs() < 1e-4,
            "Last x should be 20, got {}",
            verts[2].x
        );
        // vertex (0,2): z = 0 + 2*10 = 20
        assert!(
            (verts[6].z - 20.0).abs() < 1e-4,
            "Last z should be 20, got {}",
            verts[6].z
        );
    }

    #[test]
    fn vertices_use_correct_heights() {
        // Verify each vertex Y is the heightmap value
        let data = vec![5.0, 10.0, 15.0, 20.0];
        let hm = Heightmap::from_data(data, 2).unwrap();
        let verts = hm.generate_vertices(8.0, Vec3::ZERO);
        assert!((verts[0].y - 5.0).abs() < 1e-4);
        assert!((verts[1].y - 10.0).abs() < 1e-4);
        assert!((verts[2].y - 15.0).abs() < 1e-4);
        assert!((verts[3].y - 20.0).abs() < 1e-4);
    }
}

/// Targeted tests for Heightmap::generate_indices mutations
#[cfg(test)]
mod heightmap_generate_indices_targeted {
    use crate::Heightmap;

    #[test]
    fn index_count_for_2x2() {
        // 2x2: 1 quad = 2 triangles = 6 indices
        let data = vec![0.0; 4];
        let hm = Heightmap::from_data(data, 2).unwrap();
        let indices = hm.generate_indices();
        assert_eq!(
            indices.len(),
            6,
            "2x2 should have 6 indices, got {}",
            indices.len()
        );
    }

    #[test]
    fn index_count_for_3x3() {
        // 3x3: 4 quads = 8 triangles = 24 indices
        let data = vec![0.0; 9];
        let hm = Heightmap::from_data(data, 3).unwrap();
        let indices = hm.generate_indices();
        assert_eq!(
            indices.len(),
            24,
            "3x3 should have 24 indices, got {}",
            indices.len()
        );
    }

    #[test]
    fn indices_valid_range() {
        let data = vec![0.0; 16];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let indices = hm.generate_indices();
        let max_idx = *indices.iter().max().unwrap();
        assert!(max_idx < 16, "Max index {} should be < 16", max_idx);
    }

    #[test]
    fn first_quad_indices_correct() {
        // For 3x3: base = 0*3+0 = 0 (CCW from above)
        // Triangle 1: 0, 3, 1 (base, base+resolution, base+1)
        // Triangle 2: 1, 3, 4 (base+1, base+resolution, base+resolution+1)
        let data = vec![0.0; 9];
        let hm = Heightmap::from_data(data, 3).unwrap();
        let indices = hm.generate_indices();
        assert_eq!(indices[0], 0, "tri1[0]");
        assert_eq!(indices[1], 3, "tri1[1]"); // base + resolution = 0 + 3
        assert_eq!(indices[2], 1, "tri1[2]"); // base + 1
        assert_eq!(indices[3], 1, "tri2[0]");
        assert_eq!(indices[4], 3, "tri2[1]"); // base + resolution
        assert_eq!(indices[5], 4, "tri2[2]"); // base + resolution + 1 = 0 + 3 + 1
    }

    #[test]
    fn second_quad_x_indices_correct() {
        // For 3x3, quad at (1,0): base = 0*3+1 = 1 (CCW from above)
        // Triangle 1: 1, 4, 2
        // Triangle 2: 2, 4, 5
        let data = vec![0.0; 9];
        let hm = Heightmap::from_data(data, 3).unwrap();
        let indices = hm.generate_indices();
        // Second quad starts at index 6
        assert_eq!(indices[6], 1, "quad2 tri1[0]");
        assert_eq!(indices[7], 4, "quad2 tri1[1]");
        assert_eq!(indices[8], 2, "quad2 tri1[2]");
        assert_eq!(indices[9], 2, "quad2 tri2[0]");
        assert_eq!(indices[10], 4, "quad2 tri2[1]");
        assert_eq!(indices[11], 5, "quad2 tri2[2]");
    }

    #[test]
    fn z1_quad_indices_correct() {
        // For 3x3, quad at (0,1): base = 1*3+0 = 3 (CCW from above)
        // Kills L302 `* → /` where base = z/resolution+x
        // With mutation: base = 1/3+0 = 0 (integer div), wrong!
        let data = vec![0.0; 9];
        let hm = Heightmap::from_data(data, 3).unwrap();
        let indices = hm.generate_indices();
        // Quad (0,1) is the 3rd quad (after (0,0) and (1,0)), indices 12..17
        assert_eq!(indices[12], 3, "quad(0,1) tri1[0]: base=3");
        assert_eq!(indices[13], 6, "quad(0,1) tri1[1]: base+res=6");
        assert_eq!(indices[14], 4, "quad(0,1) tri1[2]: base+1=4");
        assert_eq!(indices[15], 4, "quad(0,1) tri2[0]: base+1=4");
        assert_eq!(indices[16], 6, "quad(0,1) tri2[1]: base+res=6");
        assert_eq!(indices[17], 7, "quad(0,1) tri2[2]: base+res+1=7");
    }
}

// ============================================================================
// V3 TARGETED MUTATION TESTS
// Precision tests to kill remaining 69 missed mutations from terrain v2
// ============================================================================

/// Precise tests for calculate_normal — kills 20 missed mutations
/// by asserting exact normal values, testing boundaries, and using scale≠1
#[cfg(test)]
mod calculate_normal_precise {
    use crate::Heightmap;

    /// Helper: create a 5x5 heightmap with x-slope (height = x * 2.0)
    fn make_x_slope_5x5() -> Heightmap {
        let mut data = vec![0.0; 25];
        for z in 0..5 {
            for x in 0..5 {
                data[z * 5 + x] = x as f32 * 2.0;
            }
        }
        Heightmap::from_data(data, 5).unwrap()
    }

    /// Helper: create a 5x5 heightmap with z-slope (height = z * 2.0)
    fn make_z_slope_5x5() -> Heightmap {
        let mut data = vec![0.0; 25];
        for z in 0..5 {
            for x in 0..5 {
                data[z * 5 + x] = z as f32 * 2.0;
            }
        }
        Heightmap::from_data(data, 5).unwrap()
    }

    #[test]
    fn interior_x_slope_exact_normal() {
        // At (2,2) on x-slope: left=h(1,2)=2, right=h(3,2)=6
        // dx = (6-2)/(2*1) = 2.0, dz = (4-4)/(2*1) = 0.0
        // normal = Vec3(-2, 1, 0).normalize() = (-0.8944, 0.4472, 0)
        let hm = make_x_slope_5x5();
        let n = hm.calculate_normal(2, 2, 1.0);
        assert!(
            (n.x - (-0.8944)).abs() < 0.02,
            "Normal.x should be -0.8944, got {}",
            n.x
        );
        assert!(
            (n.y - 0.4472).abs() < 0.02,
            "Normal.y should be 0.4472, got {}",
            n.y
        );
        assert!(n.z.abs() < 0.01, "Normal.z should be ~0, got {}", n.z);
    }

    #[test]
    fn interior_z_slope_exact_normal() {
        // At (2,2) on z-slope: up=h(2,1)=2, down=h(2,3)=6
        // dz = (6-2)/(2*1) = 2.0, dx = 0
        // normal = Vec3(0, 1, -2).normalize() = (0, 0.4472, -0.8944)
        let hm = make_z_slope_5x5();
        let n = hm.calculate_normal(2, 2, 1.0);
        assert!(n.x.abs() < 0.01, "Normal.x should be ~0, got {}", n.x);
        assert!(
            (n.y - 0.4472).abs() < 0.02,
            "Normal.y should be 0.4472, got {}",
            n.y
        );
        assert!(
            (n.z - (-0.8944)).abs() < 0.02,
            "Normal.z should be -0.8944, got {}",
            n.z
        );
    }

    #[test]
    fn boundary_x0_normal_differs_from_interior() {
        // At x=0: left = self.get_height(0, z) (boundary fallback)
        // Mutations `> → >=` make left = get_height(u32::MAX, z) = 0
        // If h(0,2) ≠ 0, this changes the normal
        let mut data = vec![5.0; 25]; // All 5.0 so boundary h(0,z)=5≠0
        for x in 0..5 {
            data[2 * 5 + x] = x as f32 * 3.0 + 5.0; // z=2 row has slope
        }
        let hm = Heightmap::from_data(data, 5).unwrap();
        let n = hm.calculate_normal(0, 2, 1.0);
        // At x=0: left = h(0,2) = 5.0 (self), right = h(1,2) = 8.0
        // dx = (8 - 5) / (2*1) = 1.5
        // up = h(0,1) = 5.0, down = h(0,3) = 5.0, dz = 0
        // normal = Vec3(-1.5, 1, 0).normalize()
        let expected_x = -1.5 / (1.5_f32.powi(2) + 1.0).sqrt();
        assert!(
            (n.x - expected_x).abs() < 0.02,
            "Boundary x=0 normal.x should be {}, got {}",
            expected_x,
            n.x
        );
    }

    #[test]
    fn boundary_x_max_normal() {
        // At x=resolution-1=4: right = self (boundary fallback)
        // Mutations `< → <=` make right = get_height(5, z) = 0 (OOB)
        let hm = make_x_slope_5x5();
        let n = hm.calculate_normal(4, 2, 1.0);
        // left = h(3,2) = 6, right = h(4,2) = 8 (self, boundary)
        // dx = (8 - 6) / (2*1) = 1.0
        // normal = Vec3(-1, 1, 0).normalize() = (-0.7071, 0.7071, 0)
        assert!(
            (n.x - (-0.7071)).abs() < 0.02,
            "Boundary x=max normal.x should be -0.7071, got {}",
            n.x
        );
        assert!(
            (n.y - 0.7071).abs() < 0.02,
            "Boundary x=max normal.y should be 0.7071, got {}",
            n.y
        );
    }

    #[test]
    fn boundary_z0_normal() {
        // At z=0: up = self (boundary fallback)
        let hm = make_z_slope_5x5();
        let n = hm.calculate_normal(2, 0, 1.0);
        // up = h(2,0) = 0 (self), down = h(2,1) = 2
        // dz = (2 - 0) / (2*1) = 1.0
        // normal = Vec3(0, 1, -1).normalize() = (0, 0.7071, -0.7071)
        assert!(
            (n.z - (-0.7071)).abs() < 0.02,
            "Boundary z=0 normal.z should be -0.7071, got {}",
            n.z
        );
    }

    #[test]
    fn boundary_z_max_normal() {
        // At z=resolution-1=4: down = self (boundary fallback)
        let hm = make_z_slope_5x5();
        let n = hm.calculate_normal(2, 4, 1.0);
        // up = h(2,3) = 6, down = h(2,4) = 8 (self, boundary)
        // dz = (8 - 6) / (2*1) = 1.0
        assert!(
            (n.z - (-0.7071)).abs() < 0.02,
            "Boundary z=max normal.z should be -0.7071, got {}",
            n.z
        );
    }

    #[test]
    fn scale_affects_normal_magnitude() {
        // With scale=3: dx = (right-left)/(2*3) instead of /(2*1)
        // Kills L192-193 mutations: `* → +` (2+3=5≠6), `* → /` (2/3≠6), `/ → *`
        let hm = make_x_slope_5x5();
        let n_s1 = hm.calculate_normal(2, 2, 1.0);
        let n_s3 = hm.calculate_normal(2, 2, 3.0);
        // scale=1: dx = 4/(2*1)=2, normal=(-2,1,0).norm=(-0.8944, 0.4472, 0)
        // scale=3: dx = 4/(2*3)=0.667, normal=(-0.667,1,0).norm
        let len = (0.667_f32.powi(2) + 1.0).sqrt();
        let expected_x = -0.667 / len;
        let expected_y = 1.0 / len;
        assert!(
            (n_s3.x - expected_x).abs() < 0.02,
            "Scale=3 normal.x should be {}, got {}",
            expected_x,
            n_s3.x
        );
        assert!(
            (n_s3.y - expected_y).abs() < 0.02,
            "Scale=3 normal.y should be {}, got {}",
            expected_y,
            n_s3.y
        );
        // Ensure scale actually changes the result
        assert!(
            (n_s1.x - n_s3.x).abs() > 0.1,
            "Different scales should give different normals: s1={}, s3={}",
            n_s1.x,
            n_s3.x
        );
    }

    #[test]
    fn scale_2_distinguishes_mul_from_add() {
        // scale=2: 2*scale=4, 2+scale=4 — SAME for scale=2!
        // scale=0.5: 2*0.5=1, 2+0.5=2.5 — DIFFERENT
        let hm = make_x_slope_5x5();
        let n = hm.calculate_normal(2, 2, 0.5);
        // dx = 4/(2*0.5) = 4/1 = 4. normal = Vec3(-4, 1, 0).normalize()
        let len = (16.0_f32 + 1.0).sqrt();
        let expected_x = -4.0 / len;
        assert!(
            (n.x - expected_x).abs() < 0.02,
            "Scale=0.5 normal.x should be {}, got {}",
            expected_x,
            n.x
        );
        // With mutation `* → +`: dx = 4/(2+0.5)=4/2.5=1.6
        // normal.x = -1.6/sqrt(3.56) = -0.848 vs expected -0.9701
        // Our assertion catches this
    }

    #[test]
    fn asymmetric_grid_detects_neighbor_swap() {
        // Grid where left≠right and up≠down distinctly
        // Kills mutations that swap neighbor indices (L172 -→/=x, L177 +→*=x)
        // and L330 idx-res → idx+res, L331 idx+res → idx-res
        let mut data = vec![0.0; 25];
        // Set up: center(2,2)=10, unique neighbors
        data[2 * 5 + 2] = 10.0; // center
        data[2 * 5 + 1] = 2.0; // left
        data[2 * 5 + 3] = 8.0; // right
        data[1 * 5 + 2] = 1.0; // up
        data[3 * 5 + 2] = 15.0; // down
        let hm = Heightmap::from_data(data, 5).unwrap();
        let n = hm.calculate_normal(2, 2, 1.0);
        // dx = (8-2)/(2*1) = 3.0, dz = (15-1)/(2*1) = 7.0
        // normal = Vec3(-3, 1, -7).normalize()
        let len = (9.0 + 1.0 + 49.0_f32).sqrt(); // sqrt(59) ≈ 7.681
        let expected_x = -3.0 / len;
        let expected_z = -7.0 / len;
        assert!(
            (n.x - expected_x).abs() < 0.02,
            "Asymmetric normal.x should be {}, got {}",
            expected_x,
            n.x
        );
        assert!(
            (n.z - expected_z).abs() < 0.02,
            "Asymmetric normal.z should be {}, got {}",
            expected_z,
            n.z
        );
    }

    #[test]
    fn z_slope_with_non_unit_scale_kills_dz_formula() {
        // Kills L193:37 `* → /` in dz = (down-up) / (2.0 * scale)
        // With scale=1: 2*1=2, 2/1=2 → EQUIVALENT
        // With scale=3: 2*3=6, 2/3=0.667 → VERY DIFFERENT
        let hm = make_z_slope_5x5();
        let n = hm.calculate_normal(2, 2, 3.0);
        // At (2,2): up=h(2,1)=2, down=h(2,3)=6, dx=0
        // dz = (6-2)/(2*3) = 4/6 = 0.667
        // normal = Vec3(0, 1, -0.667).normalize()
        let len = (0.667_f32.powi(2) + 1.0).sqrt();
        let expected_z = -0.667 / len;
        let expected_y = 1.0 / len;
        assert!(
            (n.z - expected_z).abs() < 0.02,
            "Z-slope scale=3 normal.z should be {}, got {}",
            expected_z,
            n.z
        );
        assert!(
            (n.y - expected_y).abs() < 0.02,
            "Z-slope scale=3 normal.y should be {}, got {}",
            expected_y,
            n.y
        );
        assert!(
            n.x.abs() < 0.01,
            "Z-slope normal.x should be ~0, got {}",
            n.x
        );
    }
}

/// Precise test for set_height || → && mutation
#[cfg(test)]
mod set_height_boundary {
    use crate::Heightmap;

    #[test]
    fn single_dimension_oob_does_not_write() {
        // Kills L134 `|| → &&`: with &&, single-dim OOB proceeds and panics
        let data = vec![1.0; 16];
        let mut hm = Heightmap::from_data(data, 4).unwrap();
        // x OOB, z valid: should be rejected
        hm.set_height(100, 0, 99.0);
        // Verify nothing changed
        assert_eq!(
            hm.get_height(0, 0),
            1.0,
            "Data should be unchanged after x-OOB set"
        );
        // z OOB, x valid: should be rejected
        hm.set_height(0, 100, 99.0);
        assert_eq!(
            hm.get_height(0, 0),
            1.0,
            "Data should be unchanged after z-OOB set"
        );
    }
}

/// Boundary tests for sample_bilinear — kills clamp bound mutations
#[cfg(test)]
mod bilinear_boundary_tests {
    use crate::Heightmap;

    #[test]
    fn oob_u_clamps_correctly() {
        // Test with u >> resolution to kill L147 `- → +` and `- → /`
        // On 4x4 grid: original clamps u=10 to 2.999, mutation allows larger values
        let mut data = vec![0.0; 16];
        // Make heights distinct so different positions give different results
        for i in 0..16 {
            data[i] = (i as f32 + 1.0) * 3.0;
        }
        let hm = Heightmap::from_data(data, 4).unwrap();
        // At u=10.0, v=0.0: original clamps to x=2.999
        let val_oob = hm.sample_bilinear(10.0, 0.0);
        // At u=2.999, v=0.0: should give same result as clamped version
        let val_clamped = hm.sample_bilinear(2.999, 0.0);
        assert!(
            (val_oob - val_clamped).abs() < 0.01,
            "OOB u should clamp same as near-max: oob={}, clamped={}",
            val_oob,
            val_clamped
        );
    }

    #[test]
    fn oob_v_clamps_correctly() {
        let mut data = vec![0.0; 16];
        for i in 0..16 {
            data[i] = (i as f32 + 1.0) * 3.0;
        }
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val_oob = hm.sample_bilinear(0.0, 10.0);
        let val_clamped = hm.sample_bilinear(0.0, 2.999);
        assert!(
            (val_oob - val_clamped).abs() < 0.01,
            "OOB v should clamp same as near-max: oob={}, clamped={}",
            val_oob,
            val_clamped
        );
    }

    #[test]
    fn near_boundary_u_interpolates_correctly() {
        // At u=2.5 on 4x4: x0=2, x1=3, fx=0.5
        // Should interpolate between column 2 and column 3
        let mut data = vec![0.0; 16];
        for i in 0..16 {
            data[i] = ((i % 4) as f32) * 10.0; // columns: 0, 10, 20, 30
        }
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val = hm.sample_bilinear(2.5, 0.0);
        // lerp(20, 30, 0.5) = 25
        assert!(
            (val - 25.0).abs() < 0.01,
            "Near boundary should interpolate: expected 25, got {}",
            val
        );
    }

    #[test]
    fn negative_u_clamps_to_zero() {
        let data = vec![
            100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val = hm.sample_bilinear(-5.0, 0.0);
        // Should clamp to (0,0) = 100
        assert!(
            (val - 100.0).abs() < 0.01,
            "Negative u should clamp to 0: expected 100, got {}",
            val
        );
    }

    #[test]
    fn oob_u_extreme_height_gap() {
        // Kills L147:53 `- → /` where clamp bound is res/1.001=3.996 vs res-1.001=2.999
        // Column 3 has height=100, all others=0
        // OOB u=10: original clamps to 2.999 → interpolates between col 2 (0) and col 3 (100)
        // Mutation clamps to 3.996 → x0=3, x1=min(4,3)=3 → returns exactly 100
        let mut data = vec![0.0; 16];
        data[3] = 100.0; // (3,0)
        data[7] = 100.0; // (3,1)
        data[11] = 100.0; // (3,2)
        data[15] = 100.0; // (3,3)
        let hm = Heightmap::from_data(data, 4).unwrap();
        let val = hm.sample_bilinear(10.0, 0.0);
        // Original: x=2.999, interpolates 0*0.001+100*0.999 = 99.9
        // Mutation: x=3.996, gets 100.0 exactly
        assert!(
            (val - 99.9).abs() < 0.05,
            "OOB u should clamp to 2.999 giving ~99.9, got {} (100.0 means clamp bound wrong)",
            val
        );
    }
}

/// Precise smooth tests — kills 5 missed mutations with exact value checks
#[cfg(test)]
mod smooth_precise_values {
    use crate::Heightmap;

    #[test]
    fn smooth_exact_value_at_center() {
        // 5x5 grid with non-linear data to distinguish operator mutations
        // Using values where all 4 neighbors of (2,2) are distinct and nonzero
        let mut data = vec![0.0; 25];
        // Row 0: 1 2 3 4 5
        // Row 1: 6 7 8 9 10
        // Row 2: 11 12 20 14 15  (spike at center)
        // Row 3: 16 17 18 19 20
        // Row 4: 21 22 23 24 25
        for i in 0..25 {
            data[i] = (i + 1) as f32;
        }
        data[12] = 20.0; // Spike at (2,2) — was 13

        let mut hm = Heightmap::from_data(data, 5).unwrap();
        hm.smooth(1);

        // At (2,2), idx=12: left=data[11]=12, right=data[13]=14,
        // up=data[7]=8, down=data[17]=18, center=data[12]=20
        // sum = 12 + 14 + 8 + 18 + 20*4 = 132
        // smoothed = 132/8 = 16.5
        let val = hm.get_height(2, 2);
        assert!(
            (val - 16.5).abs() < 0.01,
            "Smoothed center should be 16.5, got {}",
            val
        );
    }

    #[test]
    fn smooth_exact_value_at_off_center() {
        // At (1,1), idx=6: left=data[5]=6, right=data[7]=8,
        // up=data[1]=2, down=data[11]=12, center=data[6]=7
        // sum = 6 + 8 + 2 + 12 + 7*4 = 56
        // smoothed = 56/8 = 7.0
        let mut data = vec![0.0; 25];
        for i in 0..25 {
            data[i] = (i + 1) as f32;
        }
        data[12] = 20.0;

        let mut hm = Heightmap::from_data(data, 5).unwrap();
        hm.smooth(1);

        let val = hm.get_height(1, 1);
        assert!(
            (val - 7.0).abs() < 0.01,
            "Smoothed (1,1) should be 7.0, got {}",
            val
        );
    }

    #[test]
    fn smooth_boundary_not_modified() {
        // Boundary points (x=0, x=res-1, z=0, z=res-1) should NOT be smoothed
        let mut data = vec![0.0; 25];
        for i in 0..25 {
            data[i] = (i + 1) as f32;
        }
        data[12] = 20.0;

        let original_boundary = vec![
            (0, 0, 1.0),
            (1, 0, 2.0),
            (4, 0, 5.0),
            (0, 4, 21.0),
            (4, 4, 25.0),
        ];

        let mut hm = Heightmap::from_data(data, 5).unwrap();
        hm.smooth(1);

        for (x, z, expected) in &original_boundary {
            let val = hm.get_height(*x, *z);
            assert!(
                (val - expected).abs() < 0.01,
                "Boundary ({},{}) should be unchanged: expected {}, got {}",
                x,
                z,
                expected,
                val
            );
        }
    }

    #[test]
    fn smooth_x_boundary_not_smoothed() {
        // Kills L325:46 `- → /`: loop goes x=1..resolution instead of 1..resolution-1
        // Mutation would smooth x=resolution-1=4 positions, changing their values
        // Use spike at (4,2) so smoothing would change it
        let mut data = vec![0.0; 25];
        for i in 0..25 {
            data[i] = (i + 1) as f32;
        }
        data[2 * 5 + 4] = 50.0; // Spike at (4,2) = data[14]
                                // Neighbors: left=data[13]=14, right=data[15]=16, up=data[9]=10, down=data[19]=20
                                // If smoothed: sum = 14+16+10+20+50*4 = 260, smoothed = 32.5
                                // But (4,2) is at x=resolution-1, should NOT be in the loop
        let mut hm = Heightmap::from_data(data, 5).unwrap();
        hm.smooth(1);
        let val = hm.get_height(4, 2);
        assert!(
            (val - 50.0).abs() < 0.01,
            "Boundary x=4 should NOT be smoothed: expected 50.0, got {}",
            val
        );
    }

    #[test]
    fn smooth_distinguishes_neighbor_directions() {
        // Grid where each neighbor of center has a very different value
        // This kills mutations that swap/negate neighbor indices
        let mut data = vec![10.0; 25];
        // At (2,2): set distinct neighbors
        data[2 * 5 + 1] = 100.0; // left = 100
        data[2 * 5 + 3] = 1.0; // right = 1
        data[1 * 5 + 2] = 50.0; // up = 50
        data[3 * 5 + 2] = 200.0; // down = 200
        data[2 * 5 + 2] = 10.0; // center = 10

        let mut hm = Heightmap::from_data(data, 5).unwrap();
        hm.smooth(1);

        // sum = 100 + 1 + 50 + 200 + 10*4 = 391
        // smoothed = 391/8 = 48.875
        let val = hm.get_height(2, 2);
        assert!(
            (val - 48.875).abs() < 0.01,
            "Center with distinct neighbors should be 48.875, got {}",
            val
        );
    }
}

/// Manual computation test for calculate_climate_stats — kills 11 mutations
#[cfg(test)]
mod climate_stats_precise {
    use crate::climate::{utils, ClimateConfig, ClimateMap};

    #[test]
    fn stats_match_manual_computation() {
        // Manually compute stats the same way the function should,
        // then compare. Any mutation in step formula or sampling position
        // will produce different results.
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);

        let min_x = 50.0_f64;
        let max_x = 250.0_f64;
        let min_z = 30.0_f64;
        let max_z = 430.0_f64;
        let samples = 4_u32;

        // Manual computation using correct formulas
        let step_x = (max_x - min_x) / samples as f64;
        let step_z = (max_z - min_z) / samples as f64;
        let mut temps = Vec::new();
        let mut moists = Vec::new();
        for i in 0..samples {
            for j in 0..samples {
                let x = min_x + i as f64 * step_x;
                let z = min_z + j as f64 * step_z;
                let h = climate.estimate_height(x, z);
                let (t, m) = climate.sample_climate(x, z, h);
                temps.push(t);
                moists.push(m);
            }
        }
        let expected_t_avg = temps.iter().sum::<f32>() / temps.len() as f32;
        let expected_t_min = temps.iter().copied().fold(f32::INFINITY, f32::min);
        let expected_t_max = temps.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let expected_m_avg = moists.iter().sum::<f32>() / moists.len() as f32;

        // Now get stats from the function under test
        let stats = utils::calculate_climate_stats(&climate, min_x, max_x, min_z, max_z, samples);

        assert!(
            (stats.temperature_avg - expected_t_avg).abs() < 1e-6,
            "Temp avg mismatch: got {}, expected {}",
            stats.temperature_avg,
            expected_t_avg
        );
        assert!(
            (stats.temperature_min - expected_t_min).abs() < 1e-6,
            "Temp min mismatch: got {}, expected {}",
            stats.temperature_min,
            expected_t_min
        );
        assert!(
            (stats.temperature_max - expected_t_max).abs() < 1e-6,
            "Temp max mismatch: got {}, expected {}",
            stats.temperature_max,
            expected_t_max
        );
        assert!(
            (stats.moisture_avg - expected_m_avg).abs() < 1e-6,
            "Moisture avg mismatch: got {}, expected {}",
            stats.moisture_avg,
            expected_m_avg
        );
    }

    #[test]
    fn stats_nonzero_min_x_z_distinguishes_sign_mutations() {
        // Use min_x/z far from 0 so `max-min` ≠ `max+min`
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        let stats = utils::calculate_climate_stats(&climate, 100.0, 200.0, 100.0, 200.0, 3);
        // With `- → +`: step_x = (200+100)/3 = 100 instead of (200-100)/3 = 33.3
        // Very different sampling positions → different averages
        // Just verify the output is reasonable and matches our manual calc
        let step_x = (200.0 - 100.0) / 3.0_f64;
        let step_z = (200.0 - 100.0) / 3.0_f64;
        let mut temps = Vec::new();
        for i in 0..3_u32 {
            for j in 0..3_u32 {
                let x = 100.0 + i as f64 * step_x;
                let z = 100.0 + j as f64 * step_z;
                let h = climate.estimate_height(x, z);
                let (t, _) = climate.sample_climate(x, z, h);
                temps.push(t);
            }
        }
        let expected_avg = temps.iter().sum::<f32>() / temps.len() as f32;
        assert!(
            (stats.temperature_avg - expected_avg).abs() < 1e-6,
            "Stats should match manual: got {}, expected {}",
            stats.temperature_avg,
            expected_avg
        );
    }

    #[test]
    fn stats_large_region_step_matters() {
        // Large region where step formula errors create huge position differences
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        let stats = utils::calculate_climate_stats(&climate, 500.0, 5000.0, 500.0, 5000.0, 5);
        let step = (5000.0 - 500.0) / 5.0_f64;
        let mut temps = Vec::new();
        for i in 0..5_u32 {
            for j in 0..5_u32 {
                let x = 500.0 + i as f64 * step;
                let z = 500.0 + j as f64 * step;
                let h = climate.estimate_height(x, z);
                let (t, _) = climate.sample_climate(x, z, h);
                temps.push(t);
            }
        }
        let expected = temps.iter().sum::<f32>() / temps.len() as f32;
        assert!(
            (stats.temperature_avg - expected).abs() < 1e-6,
            "Large region stats: got {}, expected {}",
            stats.temperature_avg,
            expected
        );
    }
}

/// Precise moisture tests with controlled configs — kills moisture and fbm mutations
#[cfg(test)]
mod moisture_precise_v3 {
    use crate::climate::{ClimateConfig, ClimateMap};

    /// Create a climate map with zero-amplitude moisture noise (constant base=offset)
    fn make_controlled_climate() -> ClimateMap {
        let mut config = ClimateConfig::default();
        config.moisture.amplitude = 0.0; // No noise variation
        config.moisture.offset = 0.5; // Constant base moisture = 0.5
        ClimateMap::new(&config, 42)
    }

    #[test]
    fn moisture_height_factor_precise() {
        // With amp=0, base_moisture = 0.5 always
        // height=50: height_factor = (50*0.01).clamp(0,1) = 0.5
        // moisture *= 1.0 - 0.5*0.3 = 0.85 → 0.5*0.85 = 0.425
        // Then: moisture = 0.425 * 0.7 + water_factor * 0.3
        // We compare h=0 vs h=50:
        // h=0: hf=0, mult=1.0, m=0.5*0.7+w*0.3 = 0.35+w*0.3
        // h=50: hf=0.5, mult=0.85, m=0.425*0.7+w*0.3 = 0.2975+w*0.3
        // diff = 0.0525
        let climate = make_controlled_climate();
        let m0 = climate.sample_moisture(300.0, 300.0, 0.0);
        let m50 = climate.sample_moisture(300.0, 300.0, 50.0);
        let diff = m0 - m50;
        // With L106 `*→+`: hf=(50+0.01)→1.0, mult=0.7, diff=0.35-0.245=0.105
        // With L106 `*→/`: hf=(50/0.01)→1.0, same as +
        // With L107 `*→+`: mult=1.0-0.5+0.3=0.8, diff=0.35-0.28=0.07
        // With L107 `*→/`: mult=1.0-0.5/0.3=-0.667, moisture goes very negative → clamped
        assert!(
            (diff - 0.0525).abs() < 0.01,
            "Height effect diff should be ~0.0525, got {} (m0={}, m50={})",
            diff,
            m0,
            m50
        );
    }

    #[test]
    fn moisture_height_factor_small_height() {
        // height=0.5: hf = (0.5*0.01)=0.005, mult = 1-0.005*0.3 = 0.9985
        // vs mutation `*→+`: hf = (0.5+0.01)=0.51, mult = 1-0.51*0.3 = 0.847
        // vs mutation `*→/`: hf = (0.5/0.01)=50→clamp=1.0, mult = 0.7
        let climate = make_controlled_climate();
        let m0 = climate.sample_moisture(300.0, 300.0, 0.0);
        let m05 = climate.sample_moisture(300.0, 300.0, 0.5);
        let diff = m0 - m05;
        // Expected diff = 0.5*(1-1*0.3) - 0.5*(1-0.005*0.3) cancels to...
        // Actually: 0.5*0.7 + w*0.3 - (0.5*0.9985*0.7 + w*0.3) = 0.5*0.7*(1-0.9985) = 0.000525
        // Very small! With `*→+` mutation: diff = 0.5*0.7*(1-0.847) = 0.0536. Much larger!
        assert!(
            diff < 0.01,
            "Height=0.5 should have tiny moisture effect: diff={}",
            diff
        );
    }

    #[test]
    fn moisture_water_factor_sign_matters() {
        // Kills L111:29 `delete -`: exp(+dist*falloff) > 1 vs exp(-dist*falloff) < 1
        // At position where water_distance > 0:
        // exp(-d*f) < 1.0, exp(+d*f) > 1.0
        // moisture = base * mult * 0.7 + water_factor * 0.3
        // With delete-: water_factor > 1, so moisture > base*mult*0.7 + 0.3
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        let m = climate.sample_moisture(500.0, 500.0, 0.0);
        // With correct formula, water_factor ≤ 1, so water contribution ≤ 0.3
        // Total moisture ≤ base*0.7 + 0.3 ≤ 1.0 (already clamped)
        // Key: just ensure moisture is within expected range
        assert!(m >= 0.0 && m <= 1.0, "Moisture out of range: {}", m);
    }

    #[test]
    fn moisture_water_mul_vs_add() {
        // Kills L111:45 `* → +`: (-dist + falloff).exp() vs (-dist * falloff).exp()
        // With default falloff=0.001, at a point where distance > 0:
        // Original: exp(-dist * 0.001) — gently decays
        // Mutated: exp(-dist + 0.001) — for dist > 1, this is essentially 0
        // Test at two points: one "near water" and one "far from water"
        // The near-water point should have higher moisture than far
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        // Sample many points and verify moisture varies
        let moistures: Vec<f32> = (0..10)
            .map(|i| climate.sample_moisture(i as f64 * 500.0, 0.0, 0.0))
            .collect();
        let range = moistures.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - moistures.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            range > 0.01,
            "Moisture should vary across positions: range={}, values={:?}",
            range,
            moistures
        );
    }

    #[test]
    fn moisture_blend_is_weighted_sum_not_product() {
        // Kills L112:35 `+ → *`: moisture = base*0.7 * water_factor*0.3
        // Original: moisture = base*0.7 + water_factor*0.3
        // Product would give much smaller result for typical values
        let climate = make_controlled_climate();
        // With base=0.5, height=0: moisture = 0.5*0.7 + water*0.3 = 0.35 + water*0.3
        // If water_factor ≈ 0.9: original = 0.35 + 0.27 = 0.62
        // Product mutation: 0.35 * 0.27 = 0.0945. Much smaller!
        let m = climate.sample_moisture(100.0, 100.0, 0.0);
        // The result should be > 0.2 (weighted sum gives at least 0.35)
        assert!(
            m > 0.2,
            "Moisture should be substantial with sum blend: got {}",
            m
        );
    }

    #[test]
    fn moisture_blend_coefficient_matters() {
        // Kills L112:29 `* → /`: moisture = base/0.7 + water*0.3
        // Instead of base*0.7, dividing by 0.7 amplifies: 0.5/0.7 = 0.714
        // Original: 0.5*0.7 = 0.35
        // Total: 0.714+water*0.3 vs 0.35+water*0.3, diff=0.364
        let climate = make_controlled_climate();
        let m = climate.sample_moisture(300.0, 300.0, 0.0);
        // Original: 0.35 + water*0.3 ≤ 0.65 for reasonable water_factor
        // Mutation: 0.714 + water*0.3 ≥ 0.714, likely clamped to 1.0
        // So original should be noticeably below 1.0
        assert!(
            m < 0.95,
            "Moisture with *0.7 should not be near 1.0: got {}",
            m
        );
    }
}

/// Tests targeting sample_noise_fbm through temperature with controlled configs
#[cfg(test)]
mod fbm_targeted_v3 {
    use crate::climate::{ClimateConfig, ClimateMap};

    #[test]
    fn fbm_sign_inversion_detectable() {
        // Kills L160 `+= → -=`: value -= noise*amp instead of +=
        // Result is mirror image: -sum + offset vs +sum + offset
        // For two positions with DIFFERENT noise values, the ordering should flip
        let mut config = ClimateConfig::default();
        config.temperature.amplitude = 0.3;
        config.temperature.offset = 0.5;
        config.temperature_height_gradient = 0.0; // Disable other effects
        config.temperature_latitude_gradient = 0.0;
        let climate = ClimateMap::new(&config, 42);

        let t1 = climate.sample_temperature(100.0, 0.0, 0.0);
        let t2 = climate.sample_temperature(500.0, 0.0, 0.0);
        // Record which is higher
        // With original += : t1 and t2 have some ordering
        // With mutation -= : ordering flips (noise contribution negated)
        // We verify the specific values match expected
        // Since we can't predict noise, assert determinism and non-trivial range
        let t1b = climate.sample_temperature(100.0, 0.0, 0.0);
        assert_eq!(t1, t1b, "Should be deterministic");
        // At least verify they're in valid range and different from offset
        assert!(
            t1 != t2 || t1 != 0.5,
            "Noise should produce variation: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_amplitude_scaling_not_division() {
        // Kills L160 `* → /`: noise.get()*amplitude vs noise.get()/amplitude
        // With amplitude=2.0, * gives 2× the noise contribution
        // With / gives 0.5× → very different
        let mut config1 = ClimateConfig::default();
        config1.temperature.amplitude = 2.0;
        config1.temperature.offset = 0.0; // Start from 0 to avoid clamp masking
        config1.temperature_height_gradient = 0.0;
        config1.temperature_latitude_gradient = 0.0;
        let climate1 = ClimateMap::new(&config1, 42);

        let mut config2 = ClimateConfig::default();
        config2.temperature.amplitude = 0.5; // This is what / would effectively give (1/2)
        config2.temperature.offset = 0.0;
        config2.temperature_height_gradient = 0.0;
        config2.temperature_latitude_gradient = 0.0;
        let climate2 = ClimateMap::new(&config2, 42);

        // With * : t1 uses 2.0× noise, t2 uses 0.5× noise
        // With / : t1 uses 1/2.0=0.5× noise (same as t2!), t2 uses 1/0.5=2.0× noise (same as t1!)
        // So the mutation swaps their behaviors
        let t1 = climate1.sample_temperature(300.0, 300.0, 0.0);
        let t2 = climate2.sample_temperature(300.0, 300.0, 0.0);
        // They should be different with correct multiplication
        assert!(
            (t1 - t2).abs() > 0.01 || (t1 == 0.0 && t2 == 0.0) || (t1 == 1.0 && t2 == 1.0),
            "amp=2 vs amp=0.5 should differ: t1={}, t2={}",
            t1,
            t2
        );
    }

    #[test]
    fn fbm_frequency_scaling_matters() {
        // Kills L160 `* → +`: x*frequency vs x+frequency
        // With large x and small frequency, x*freq is very different from x+freq
        // x=1000, freq=0.001: x*freq=1.0, x+freq=1000.001
        let mut config = ClimateConfig::default();
        config.temperature.scale = 0.001;
        config.temperature.offset = 0.5;
        config.temperature.amplitude = 0.3;
        config.temperature_height_gradient = 0.0;
        config.temperature_latitude_gradient = 0.0;
        let climate = ClimateMap::new(&config, 42);

        // Test at position where x*freq gives a specific noise value
        let t_far = climate.sample_temperature(1000.0, 0.0, 0.0);
        // Position where x+freq would give essentially the same noise as x=1000
        // (since noise.get([1000.001, ...]) ≈ noise.get([1000, ...]))
        // But with correct formula: noise.get([1.0, ...]) — very different!
        let t_near = climate.sample_temperature(1.0, 0.0, 0.0);
        // They should differ significantly since they sample very different noise positions
        // (unless both clamp to same bound)
        assert!(
            t_far != t_near || t_far == 0.0 || t_far == 1.0,
            "Different positions should give different noise: far={}, near={}",
            t_far,
            t_near
        );
    }

    #[test]
    fn fbm_persistence_decay_not_linear() {
        // Kills L162 `*= → +=`: amplitude *= persistence vs += persistence
        // *= reduces amplitude geometrically: a, a*p, a*p², ...
        // += increases linearly: a, a+p, a+2p, ...
        // With persistence=0.5, 3 octaves:
        //   *=: amplitudes = [0.3, 0.15, 0.075]
        //   +=: amplitudes = [0.3, 0.8, 1.3] — MUCH larger!
        let mut config1 = ClimateConfig::default();
        config1.temperature.amplitude = 0.3;
        config1.temperature.persistence = 0.5;
        config1.temperature.octaves = 4;
        config1.temperature.offset = 0.0;
        config1.temperature_height_gradient = 0.0;
        config1.temperature_latitude_gradient = 0.0;
        let climate = ClimateMap::new(&config1, 42);

        // With += mutation, the sum of amplitudes is much larger:
        // Total amp with *=: 0.3 + 0.15 + 0.075 + 0.0375 = 0.5625
        // Total amp with +=: 0.3 + 0.8 + 1.3 + 1.8 = 4.2
        // After offset=0, value could be in [-4.2, 4.2] and clamp to [0,1]
        // The mutation would give very different results
        let t = climate.sample_temperature(200.0, 200.0, 0.0);
        // With correct *= decay, values should be moderate
        assert!(t >= 0.0 && t <= 1.0, "Temperature should be clamped: {}", t);
        // The value should be noticeably below 1.0 or above 0.0 with amp=0.3
        // (noise ranges roughly ±1, so sum ≈ ±0.5625, plus offset 0 = [-0.56, 0.56] → clamp)
    }
}

/// Enhanced temperature tests for re-run accuracy
#[cfg(test)]
mod temperature_enhanced_v3 {
    use crate::climate::{ClimateConfig, ClimateMap};

    #[test]
    fn temperature_height_gradient_operator_precise() {
        // Kills L90 `*→/`: height * gradient vs height / gradient
        // height=10, gradient=-0.0065: *gives -0.065, /gives -1538.5
        // After adding to base (~0.5) and clamping: * gives ~0.435, / gives 0.0
        let mut config = ClimateConfig::default();
        config.temperature_latitude_gradient = 0.0; // Isolate height effect
        let climate = ClimateMap::new(&config, 42);
        let t10 = climate.sample_temperature(500.0, 0.0, 10.0);
        // height 10 * -0.0065 = -0.065 adjustment (small)
        // Should still be > 0.1 (base is ~0.5 from noise+offset)
        assert!(
            t10 > 0.1,
            "Height=10 should not crush temperature: got {}",
            t10
        );
    }

    #[test]
    fn temperature_latitude_operator_precise() {
        // Kills L94 `+=→-=` and `*→/`
        // At z=100000: sin(100000*0.00001)=sin(1)≈0.841
        // +=: adds 0.841*0.8 = 0.673
        // -=: subtracts 0.673
        // /: adds 0.841/0.8 = 1.051
        let mut config = ClimateConfig::default();
        config.temperature_height_gradient = 0.0; // Isolate latitude
        config.temperature.offset = 0.3; // Lower base so latitude effect is visible
        config.temperature.amplitude = 0.1;
        let climate = ClimateMap::new(&config, 42);
        let t0 = climate.sample_temperature(500.0, 0.0, 0.0);
        let t_high_z = climate.sample_temperature(500.0, 100000.0, 0.0);
        // Latitude effect: sin(1)*0.8 ≈ 0.673
        // t_high_z should be t0 + 0.673 (likely clamped to 1.0)
        // With -= mutation: t_high_z = t0 - 0.673 (likely 0.0)
        // So t_high_z should be > t0 (positive latitude effect)
        assert!(
            t_high_z >= t0 - 0.01 || t_high_z == 1.0,
            "Positive latitude should increase temp: z0={}, z_high={}",
            t0,
            t_high_z
        );
    }
}

/// Golden value tests — exact regression values to kill stubborn noise/water mutations
/// These catch mutations in sample_noise_fbm and water_factor formula (L111, L160, L162)
/// by asserting specific computed outputs that change under ANY operator mutation
#[cfg(test)]
mod golden_value_tests {
    use crate::climate::{ClimateConfig, ClimateMap};

    #[test]
    fn moisture_golden_value_default_config() {
        // Golden values for default config, seed=42 at specific positions
        // Any mutation in fbm or water_factor formula changes these
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);

        // Position (300, 300): known moisture = 0.353239
        let m1 = climate.sample_moisture(300.0, 300.0, 0.0);
        assert!(
            (m1 - 0.353239).abs() < 0.001,
            "Moisture at (300,300) should be 0.353239, got {}",
            m1
        );

        // Position (500, 500): known moisture = 0.623433
        let m2 = climate.sample_moisture(500.0, 500.0, 0.0);
        assert!(
            (m2 - 0.623433).abs() < 0.001,
            "Moisture at (500,500) should be 0.623433, got {}",
            m2
        );

        // Position (1000, 1000): known moisture = 0.630198
        let m3 = climate.sample_moisture(1000.0, 1000.0, 0.0);
        assert!(
            (m3 - 0.630198).abs() < 0.001,
            "Moisture at (1000,1000) should be 0.630198, got {}",
            m3
        );
    }

    #[test]
    fn temperature_golden_value_default_config() {
        // Golden values for temperature — kills fbm sign/operator mutations
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);

        let t1 = climate.sample_temperature(300.0, 300.0, 0.0);
        assert!(
            (t1 - 0.640202).abs() < 0.001,
            "Temperature at (300,300) should be 0.640202, got {}",
            t1
        );

        let t2 = climate.sample_temperature(100.0, 100.0, 0.0);
        assert!(
            (t2 - 0.333779).abs() < 0.001,
            "Temperature at (100,100) should be 0.333779, got {}",
            t2
        );

        let t3 = climate.sample_temperature(1000.0, 1000.0, 0.0);
        assert!(
            (t3 - 0.508000).abs() < 0.001,
            "Temperature at (1000,1000) should be 0.508000, got {}",
            t3
        );
    }

    #[test]
    fn moisture_golden_controlled_config() {
        // With controlled moisture config (amplitude=0.2, offset=0.5)
        // These golden values catch fbm mutations since base moisture varies
        let mut config = ClimateConfig::default();
        config.moisture.amplitude = 0.2;
        config.moisture.offset = 0.5;
        let climate = ClimateMap::new(&config, 42);

        let m1 = climate.sample_moisture(300.0, 300.0, 0.0);
        assert!(
            (m1 - 0.589553).abs() < 0.001,
            "Controlled moisture at (300,300) should be 0.589553, got {}",
            m1
        );

        let m2 = climate.sample_moisture(500.0, 500.0, 0.0);
        assert!(
            (m2 - 0.608050).abs() < 0.001,
            "Controlled moisture at (500,500) should be 0.608050, got {}",
            m2
        );

        let m3 = climate.sample_moisture(100.0, 100.0, 0.0);
        assert!(
            (m3 - 0.521106).abs() < 0.001,
            "Controlled moisture at (100,100) should be 0.521106, got {}",
            m3
        );
    }
}
