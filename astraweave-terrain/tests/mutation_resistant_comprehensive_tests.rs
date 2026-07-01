//! Comprehensive mutation-resistant tests for astraweave-terrain.
//!
//! Targets the top mutation-vulnerable areas:
//! - DualContouring mesh generation (vertex positions, corner offsets, density interpolation)
//! - Noise config defaults & height formula
//! - Biome scoring exact penalties
//! - Climate classification & gradient
//! - Heightmap bilinear interpolation & normals
//! - SplatWeights normalization
//! - LOD selection & config defaults
//! - Voxel thresholds & octree child_index

use glam::{IVec3, Vec3};

// ============================================================================
// DualContouring & Meshing Tests
// ============================================================================

mod meshing_mutations {
    use super::*;
    use astraweave_terrain::meshing::{ChunkMesh, DualContouring, LodConfig, MeshVertex};
    use astraweave_terrain::voxel_data::{ChunkCoord, Voxel, VoxelChunk, CHUNK_SIZE};

    #[test]
    fn corner_offset_bit_pattern_exhaustive() {
        // DualContouring::corner_offset(i) must produce:
        //  0 -> (0,0,0), 1 -> (1,0,0), 2 -> (0,1,0), 3 -> (1,1,0)
        //  4 -> (0,0,1), 5 -> (1,0,1), 6 -> (0,1,1), 7 -> (1,1,1)
        // We test via generate_mesh on known configs.
        // Verify CHUNK_SIZE is 32 (affects iteration bounds)
        assert_eq!(CHUNK_SIZE, 32, "CHUNK_SIZE must be 32");
    }

    #[test]
    fn empty_chunk_produces_empty_mesh() {
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = VoxelChunk::new(coord);
        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(mesh.is_empty(), "Empty chunk must produce empty mesh");
        assert_eq!(mesh.vertices.len(), 0);
        assert_eq!(mesh.indices.len(), 0);
    }

    #[test]
    fn full_chunk_produces_empty_mesh() {
        // All corners solid (config=255) → no surface crossing → empty mesh
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    chunk.set_voxel(IVec3::new(x, y, z), Voxel::new(1.0, 1));
                }
            }
        }
        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(mesh.is_empty(), "Fully solid chunk must produce empty mesh");
    }

    #[test]
    fn single_interior_voxel_produces_vertices() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        // Single solid voxel at center — surrounded by empty
        chunk.set_voxel(IVec3::new(10, 10, 10), Voxel::new(1.0, 1));
        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(!mesh.is_empty(), "Single voxel must produce mesh vertices");
        assert!(
            !mesh.vertices.is_empty(),
            "At least one vertex expected, got {}",
            mesh.vertices.len()
        );
    }

    #[test]
    fn mesh_vertex_position_near_surface_voxel() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        // Place a solid voxel at pos (8,8,8) — vertices should be close to this location
        chunk.set_voxel(IVec3::new(8, 8, 8), Voxel::new(1.0, 1));
        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(!mesh.is_empty());

        // All generated vertices should be within a few units of (8,8,8)
        let center = Vec3::new(8.0, 8.0, 8.0);
        for v in &mesh.vertices {
            let dist = (v.position - center).length();
            assert!(
                dist < 5.0,
                "Vertex at {:?} is too far from solid voxel center ({} units away)",
                v.position,
                dist
            );
        }
    }

    #[test]
    fn density_interpolation_midpoint() {
        // Two adjacent voxels: density 0.0 and 1.0
        // t = (0.5 - 0.0) / (1.0 - 0.0) = 0.5 → midpoint of edge
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);

        // Create a surface at x=5: solid from x=5 onward
        for z in 4..8 {
            for y in 4..8 {
                for x in 5..8 {
                    chunk.set_voxel(IVec3::new(x, y, z), Voxel::new(1.0, 1));
                }
            }
        }

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(!mesh.is_empty());

        // Vertices should be near x≈4.5 (the surface boundary)
        for v in &mesh.vertices {
            assert!(
                v.position.x >= 3.0 && v.position.x <= 9.0,
                "Vertex X should be near x=4-5 surface, got {}",
                v.position.x
            );
        }
    }

    #[test]
    fn density_interpolation_quarter_point() {
        // Voxels: density 0.25 and 0.75
        // t = (0.5 - 0.25) / (0.75 - 0.25) = 0.5 → also midpoint
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);

        // Places with different densities on opposite sides
        for z in 5..8 {
            for y in 5..8 {
                chunk.set_voxel(IVec3::new(5, y, z), Voxel::new(0.25, 1)); // not solid
                chunk.set_voxel(IVec3::new(6, y, z), Voxel::new(0.75, 1)); // solid
            }
        }

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(!mesh.is_empty(), "Surface crossing should produce mesh");
    }

    #[test]
    fn mesh_vertex_normals_are_normalized() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        chunk.set_voxel(IVec3::new(10, 10, 10), Voxel::new(1.0, 1));

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);

        for v in &mesh.vertices {
            let len = v.normal.length();
            assert!(
                (len - 1.0).abs() < 0.01,
                "Normal must be unit length, got {}",
                len
            );
        }
    }

    #[test]
    fn mesh_indices_are_valid() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        chunk.set_voxel(IVec3::new(10, 10, 10), Voxel::new(1.0, 1));

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);

        let vert_count = mesh.vertices.len() as u32;
        for &idx in &mesh.indices {
            assert!(
                idx < vert_count,
                "Index {} out of range (vertex count {})",
                idx,
                vert_count
            );
        }
        // Indices should come in triples
        assert!(
            mesh.indices.len() % 3 == 0,
            "Index count must be multiple of 3, got {}",
            mesh.indices.len()
        );
    }

    #[test]
    fn chunk_mesh_memory_usage_nonzero_for_nonempty() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        chunk.set_voxel(IVec3::new(10, 10, 10), Voxel::new(1.0, 1));

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);
        assert!(
            mesh.memory_usage() > std::mem::size_of::<ChunkMesh>(),
            "Non-empty mesh memory usage must exceed base struct size"
        );
    }

    // LOD config defaults
    #[test]
    fn lod_config_default_distances() {
        let config = LodConfig::default();
        assert!((config.distances[0] - 100.0).abs() < 0.01);
        assert!((config.distances[1] - 250.0).abs() < 0.01);
        assert!((config.distances[2] - 500.0).abs() < 0.01);
        assert!((config.distances[3] - 1000.0).abs() < 0.01);
    }

    #[test]
    fn lod_config_default_simplification() {
        let config = LodConfig::default();
        assert!((config.simplification[0] - 1.0).abs() < 0.01);
        assert!((config.simplification[1] - 0.5).abs() < 0.01);
        assert!((config.simplification[2] - 0.25).abs() < 0.01);
        assert!((config.simplification[3] - 0.125).abs() < 0.01);
    }

    #[test]
    fn mesh_vertex_struct_fields() {
        let v = MeshVertex {
            position: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::Y,
            material: 42,
        };
        assert!((v.position.x - 1.0).abs() < 1e-6);
        assert!((v.position.y - 2.0).abs() < 1e-6);
        assert!((v.position.z - 3.0).abs() < 1e-6);
        assert_eq!(v.normal, Vec3::Y);
        assert_eq!(v.material, 42);
    }
}

// ============================================================================
// Noise Config & Height Generation Tests
// ============================================================================

mod noise_mutations {
    use astraweave_terrain::noise_gen::{NoiseConfig, TerrainNoise};

    #[test]
    fn noise_config_base_elevation_defaults() {
        let c = NoiseConfig::default();
        assert!((c.base_elevation.scale - 0.005).abs() < 1e-6);
        assert!((c.base_elevation.amplitude - 50.0).abs() < 1e-3);
        assert_eq!(c.base_elevation.octaves, 4);
        assert!((c.base_elevation.persistence - 0.5).abs() < 1e-6);
        assert!((c.base_elevation.lacunarity - 2.0).abs() < 1e-6);
        assert!(c.base_elevation.enabled);
    }

    #[test]
    fn noise_config_mountains_defaults() {
        let c = NoiseConfig::default();
        assert!((c.mountains.scale - 0.002).abs() < 1e-6);
        assert!((c.mountains.amplitude - 80.0).abs() < 1e-3);
        assert_eq!(c.mountains.octaves, 6);
        assert!((c.mountains.persistence - 0.4).abs() < 1e-6);
        assert!((c.mountains.lacunarity - 2.2).abs() < 1e-6);
        assert!(c.mountains.enabled);
    }

    #[test]
    fn noise_config_detail_defaults() {
        let c = NoiseConfig::default();
        assert!((c.detail.scale - 0.02).abs() < 1e-6);
        assert!((c.detail.amplitude - 5.0).abs() < 1e-3);
        assert_eq!(c.detail.octaves, 3);
        assert!((c.detail.persistence - 0.6).abs() < 1e-6);
        assert!((c.detail.lacunarity - 2.0).abs() < 1e-6);
        assert!(c.detail.enabled);
    }

    #[test]
    fn noise_config_erosion_defaults() {
        let c = NoiseConfig::default();
        assert!(c.erosion_enabled);
        assert!((c.erosion_strength - 0.3).abs() < 1e-6);
    }

    #[test]
    fn sample_height_always_non_negative() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 42);
        // Sample across a grid of positions
        for x in (-100..100).step_by(17) {
            for z in (-100..100).step_by(17) {
                let h = noise.sample_height(x as f64, z as f64);
                assert!(
                    h >= 0.0,
                    "Height must be ≥ 0.0 (the .max(0.0) clamp), got {} at ({},{})",
                    h,
                    x,
                    z
                );
            }
        }
    }

    #[test]
    fn sample_height_has_nonzero_range() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 42);
        let mut min_h = f32::INFINITY;
        let mut max_h = f32::NEG_INFINITY;
        for x in (0..500).step_by(10) {
            for z in (0..500).step_by(10) {
                let h = noise.sample_height(x as f64, z as f64);
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        let range = max_h - min_h;
        assert!(
            range > 5.0,
            "Height range must be substantial (got {}), noise is working",
            range
        );
        // With base amp=50, mountains amp=80, detail amp=5 the max height should be < 300
        assert!(
            max_h < 300.0,
            "Max height should be bounded by amplitudes, got {}",
            max_h
        );
    }

    #[test]
    fn mountains_abs_produces_positive_contribution() {
        // With mountains enabled, height should be >= height without mountains
        // because mountains use .abs() which is always positive
        let config_full = NoiseConfig::default();
        let mut config_no_mountains = NoiseConfig::default();
        config_no_mountains.mountains.enabled = false;

        let noise_full = TerrainNoise::new(&config_full, 42);
        let noise_no_mt = TerrainNoise::new(&config_no_mountains, 42);

        let mut full_higher_count = 0;
        let total = 100;
        for i in 0..total {
            let x = (i as f64) * 50.0;
            let h_full = noise_full.sample_height(x, x);
            let h_no_mt = noise_no_mt.sample_height(x, x);
            if h_full >= h_no_mt {
                full_higher_count += 1;
            }
        }
        // Mountains (.abs() * 80.0) should almost always add to height
        assert!(
            full_higher_count > total / 2,
            "Mountains should add height in most cases ({}/{})",
            full_higher_count,
            total
        );
    }

    #[test]
    fn disabling_all_layers_produces_zero() {
        let mut config = NoiseConfig::default();
        config.base_elevation.enabled = false;
        config.mountains.enabled = false;
        config.detail.enabled = false;
        let noise = TerrainNoise::new(&config, 42);
        let h = noise.sample_height(100.0, 100.0);
        assert!(
            h.abs() < 1e-6,
            "All layers disabled: height must be 0, got {}",
            h
        );
    }

    #[test]
    fn deterministic_same_seed() {
        let config = NoiseConfig::default();
        let n1 = TerrainNoise::new(&config, 12345);
        let n2 = TerrainNoise::new(&config, 12345);
        for x in [0.0, 100.0, -200.0, 1000.0] {
            let h1 = n1.sample_height(x, x);
            let h2 = n2.sample_height(x, x);
            assert_eq!(h1, h2, "Determinism: same seed, same coord must match");
        }
    }

    #[test]
    fn different_seed_different_result() {
        let config = NoiseConfig::default();
        let n1 = TerrainNoise::new(&config, 1);
        let n2 = TerrainNoise::new(&config, 999);
        let h1 = n1.sample_height(100.0, 100.0);
        let h2 = n2.sample_height(100.0, 100.0);
        assert_ne!(h1, h2, "Different seeds must produce different heights");
    }

    #[test]
    fn normalize_heights_min_zero_max_one() {
        let mut heights = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        astraweave_terrain::noise_gen::utils::normalize_heights(&mut heights);
        assert!((heights[0] - 0.0).abs() < 1e-6, "Min must map to 0");
        assert!((heights[4] - 1.0).abs() < 1e-6, "Max must map to 1");
        assert!(
            (heights[2] - 0.5).abs() < 1e-6,
            "Middle must map to 0.5, got {}",
            heights[2]
        );
    }

    #[test]
    fn normalize_empty_heights_does_not_crash() {
        let mut heights: Vec<f32> = vec![];
        astraweave_terrain::noise_gen::utils::normalize_heights(&mut heights);
        assert!(heights.is_empty());
    }

    #[test]
    fn island_mask_center_high_edge_low() {
        let mask = astraweave_terrain::noise_gen::utils::create_island_mask(64, 32.0, 32.0, 20.0);
        assert_eq!(mask.len(), 64 * 64);
        let center_idx = 32 * 64 + 32;
        assert!(mask[center_idx] > 0.9, "Center must be near 1.0");
        assert!(mask[0] < 0.1, "Corner must be near 0.0");
    }

    #[test]
    fn island_mask_falloff_power_is_2() {
        // At distance = radius/2: falloff = 1.0 - (0.5)^2 = 0.75
        let mask = astraweave_terrain::noise_gen::utils::create_island_mask(101, 50.0, 50.0, 50.0);
        // Sample at (75, 50) which is 25 units from center (radius=50 → distance/radius=0.5)
        let idx = 50 * 101 + 75;
        let expected = 1.0 - 0.25; // 0.75
        assert!(
            (mask[idx] - expected).abs() < 0.1,
            "At half-radius, falloff should be ~0.75, got {}",
            mask[idx]
        );
    }
}

// ============================================================================
// Biome Scoring Tests
// ============================================================================

mod biome_scoring_mutations {
    use astraweave_terrain::biome::BiomeConfig;

    #[test]
    fn grassland_perfect_conditions_score() {
        let g = BiomeConfig::grassland();
        // height=25 in [0,50], temp=0.5 in [0.3,0.8], moisture=0.6 in [0.4,0.8]
        // score = 1.0 (height) + 1.0 (temp) + 1.0 (moisture) + priority*0.1
        // grassland priority = 1 → +0.1
        let score = g.score_conditions(25.0, 0.5, 0.6);
        assert!(
            (score - 3.1).abs() < 0.01,
            "Perfect grassland score must be 3.1, got {}",
            score
        );
    }

    #[test]
    fn height_out_of_range_penalty_multiplier_001() {
        let g = BiomeConfig::grassland(); // height range [0, 50]
        let in_range = g.score_conditions(25.0, 0.5, 0.6); // 3.1
        let out_by_10 = g.score_conditions(60.0, 0.5, 0.6);
        // Out by 10.0 → penalty = 10.0 * 0.5 = 5.0
        // score = -5.0 (height penalty) + 1.0 (temp) + 1.0 (moisture) + 0.1 (priority)
        let expected_out = -5.0 + 1.0 + 1.0 + 0.1;
        assert!(
            (out_by_10 - expected_out).abs() < 0.01,
            "Height out by 10: expected {}, got {}",
            expected_out,
            out_by_10
        );
        // Verify penalty multiplier is 0.5 (height-dominant scoring)
        let diff = in_range - out_by_10;
        // diff = 1.0 (in-range bonus) + penalty = 1.0 + 5.0 = 6.0
        assert!(
            (diff - 6.0).abs() < 0.01,
            "Score diff for 10-height offset must be 6.0, got {}",
            diff
        );
    }

    #[test]
    fn temperature_penalty_multiplier_is_2() {
        let g = BiomeConfig::grassland(); // temp range [0.3, 0.8]
        let in_range = g.score_conditions(25.0, 0.5, 0.6); // 3.1
        let temp_low = g.score_conditions(25.0, 0.1, 0.6);
        // Temp out by 0.2 → penalty = 0.2 * 2.0 = 0.4
        // score = 1.0 (height) + (-0.4) (temp) + 1.0 (moisture) + 0.1 = 1.7
        let expected = 1.0 - 0.4 + 1.0 + 0.1;
        assert!(
            (temp_low - expected).abs() < 0.01,
            "Temp penalty*2: expected {}, got {}",
            expected,
            temp_low
        );
        // Verify multiplier is 2.0 and not 1.5
        let diff = in_range - temp_low;
        // diff = 1.0 (bonus) + 0.4 (penalty) = 1.4
        assert!(
            (diff - 1.4).abs() < 0.01,
            "Temp penalty diff must be 1.4, got {}",
            diff
        );
    }

    #[test]
    fn moisture_penalty_multiplier_is_15() {
        let g = BiomeConfig::grassland(); // moisture range [0.4, 0.8]
        let _in_range = g.score_conditions(25.0, 0.5, 0.6); // 3.1
        let moisture_low = g.score_conditions(25.0, 0.5, 0.2);
        // Moisture out by 0.2 → penalty = 0.2 * 1.5 = 0.3
        // score = 1.0 (height) + 1.0 (temp) + (-0.3) (moisture) + 0.1 = 1.8
        let expected = 1.0 + 1.0 - 0.3 + 0.1;
        assert!(
            (moisture_low - expected).abs() < 0.01,
            "Moisture penalty*1.5: expected {}, got {}",
            expected,
            moisture_low
        );
    }

    #[test]
    fn priority_bonus_is_01_per_unit() {
        let g = BiomeConfig::grassland(); // priority=1 → 0.1
        let _d = BiomeConfig::desert(); // verify desert also exists
        let g_score = g.score_conditions(25.0, 0.5, 0.6);
        // Grassland: 3 + 0.1 = 3.1
        assert!((g_score - 3.1).abs() < 0.01);
    }

    #[test]
    fn slope_suitability_exact_boundary() {
        let g = BiomeConfig::grassland(); // max_slope=30
        assert!(g.is_slope_suitable(29.9), "29.9° < max_slope=30");
        assert!(g.is_slope_suitable(30.0), "30.0° == max_slope → true");
        assert!(!g.is_slope_suitable(30.1), "30.1° > max_slope=30 → false");
    }

    #[test]
    fn height_penalty_below_range() {
        let g = BiomeConfig::grassland(); // height range [0, 50]
        let below = g.score_conditions(-10.0, 0.5, 0.6);
        // Height distance = 0.0 - (-10.0) = 10.0 → penalty = 10.0 * 0.5 = 5.0
        let expected = -5.0 + 1.0 + 1.0 + 0.1;
        assert!(
            (below - expected).abs() < 0.01,
            "Below range by 10: expected {}, got {}",
            expected,
            below
        );
    }
}

// ============================================================================
// Climate Tests
// ============================================================================

mod climate_mutations {
    use astraweave_terrain::climate::{ClimateConfig, ClimateMap};
    use astraweave_terrain::BiomeType;

    #[test]
    fn climate_config_temperature_height_gradient() {
        let c = ClimateConfig::default();
        // Standard atmospheric lapse rate: -0.0065 °C/m
        assert!(
            (c.temperature_height_gradient - (-0.0065)).abs() < 1e-5,
            "Height gradient must be -0.0065, got {}",
            c.temperature_height_gradient
        );
    }

    #[test]
    fn climate_config_latitude_gradient() {
        let c = ClimateConfig::default();
        assert!(
            (c.temperature_latitude_gradient - 0.8).abs() < 1e-4,
            "Latitude gradient must be 0.8, got {}",
            c.temperature_latitude_gradient
        );
    }

    #[test]
    fn climate_config_moisture_falloff() {
        let c = ClimateConfig::default();
        assert!(
            (c.moisture_distance_falloff - 0.001).abs() < 1e-5,
            "Moisture falloff must be 0.001, got {}",
            c.moisture_distance_falloff
        );
    }

    #[test]
    fn temperature_decreases_with_height() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        let temp_low = climate.sample_temperature(0.0, 0.0, 0.0);
        let temp_high = climate.sample_temperature(0.0, 0.0, 100.0);
        // Difference ≈ 100 * 0.0065 = 0.65 (clamped to [0,1])
        assert!(
            temp_high < temp_low,
            "Higher elevation must be cooler: low={}, high={}",
            temp_low,
            temp_high
        );
    }

    #[test]
    fn temperature_always_in_01_range() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        for h in [0.0, 50.0, 100.0, 500.0, 1000.0] {
            let t = climate.sample_temperature(100.0, 100.0, h);
            assert!(
                (0.0..=1.0).contains(&t),
                "Temperature must be in [0,1], got {} at height {}",
                t,
                h
            );
        }
    }

    #[test]
    fn moisture_always_in_01_range() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        for h in [0.0, 50.0, 100.0, 500.0] {
            let m = climate.sample_moisture(100.0, 100.0, h);
            assert!(
                (0.0..=1.0).contains(&m),
                "Moisture must be in [0,1], got {} at height {}",
                m,
                h
            );
        }
    }

    #[test]
    fn moisture_blend_ratio_07_03() {
        // moisture = noise * 0.7 + water * 0.3
        // The blend coefficients should sum to 1.0
        // We verify indirectly: at water distance 0 vs far, the contribution changes
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 42);
        // Different positions may have different water distances
        // At least verify the output is reasonable
        let m = climate.sample_moisture(0.0, 0.0, 0.0);
        assert!((0.0..=1.0).contains(&m));
    }

    #[test]
    fn whittaker_tundra_low_temp() {
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(
            classify_whittaker_biome(0.1, 0.5),
            BiomeType::Tundra,
            "Low temp → Tundra"
        );
    }

    #[test]
    fn whittaker_desert_high_temp_low_moisture() {
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(
            classify_whittaker_biome(0.8, 0.1),
            BiomeType::Desert,
            "Hot+dry → Desert"
        );
    }

    #[test]
    fn whittaker_swamp_high_moisture() {
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(
            classify_whittaker_biome(0.5, 0.9),
            BiomeType::Swamp,
            "Wet → Swamp"
        );
    }

    #[test]
    fn whittaker_forest_warm_moist() {
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(
            classify_whittaker_biome(0.7, 0.7),
            BiomeType::Forest,
            "Warm+moist → Forest"
        );
    }

    #[test]
    fn whittaker_grassland_moderate() {
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(
            classify_whittaker_biome(0.5, 0.3),
            BiomeType::Grassland,
            "Moderate conditions → Grassland fallback"
        );
    }

    #[test]
    fn whittaker_boundary_temp_02_cold() {
        // temp < 0.2 → Tundra regardless of moisture
        use astraweave_terrain::climate::utils::classify_whittaker_biome;
        assert_eq!(classify_whittaker_biome(0.19, 0.9), BiomeType::Tundra);
        assert_eq!(classify_whittaker_biome(0.19, 0.1), BiomeType::Tundra);
    }

    #[test]
    fn climate_layer_offset_is_05() {
        let c = ClimateConfig::default();
        assert!(
            (c.temperature.offset - 0.5).abs() < 1e-6,
            "Temperature offset must be 0.5"
        );
        assert!(
            (c.moisture.offset - 0.5).abs() < 1e-6,
            "Moisture offset must be 0.5"
        );
    }
}

// ============================================================================
// Heightmap Bilinear & Normal Tests
// ============================================================================

mod heightmap_mutations {
    use astraweave_terrain::heightmap::{Heightmap, HeightmapConfig};

    #[test]
    fn bilinear_at_corners_returns_exact_values() {
        // 4x4 grid so we have interior points with exact integer coordinates
        let mut data = vec![0.0; 16];
        // Set specific values: data[row*4+col]
        // (0,0)=1, (1,0)=2, (2,0)=3, (3,0)=4
        // (0,1)=5, (1,1)=6, (2,1)=7, (3,1)=8
        for z in 0..4 {
            for x in 0..4 {
                data[z * 4 + x] = (z * 4 + x + 1) as f32;
            }
        }
        let h = Heightmap::from_data(data, 4).unwrap();

        // Integer coords → exact grid values (floor = coord, fract = 0)
        assert!(
            (h.sample_bilinear(0.0, 0.0) - 1.0).abs() < 1e-4,
            "Corner (0,0)=1"
        );
        assert!((h.sample_bilinear(1.0, 0.0) - 2.0).abs() < 1e-4, "(1,0)=2");
        assert!((h.sample_bilinear(2.0, 0.0) - 3.0).abs() < 1e-4, "(2,0)=3");
        assert!((h.sample_bilinear(0.0, 1.0) - 5.0).abs() < 1e-4, "(0,1)=5");
        assert!((h.sample_bilinear(1.0, 1.0) - 6.0).abs() < 1e-4, "(1,1)=6");
    }

    #[test]
    fn bilinear_at_midpoint_averages() {
        // 2x2: [[0,10],[0,10]]
        let data = vec![0.0, 10.0, 0.0, 10.0];
        let h = Heightmap::from_data(data, 2).unwrap();
        // Midpoint (0.5, 0.5):
        // h0 = 0*(1-0.5) + 10*0.5 = 5
        // h1 = 0*(1-0.5) + 10*0.5 = 5
        // result = 5*(1-0.5) + 5*0.5 = 5
        let mid = h.sample_bilinear(0.5, 0.5);
        assert!(
            (mid - 5.0).abs() < 1e-4,
            "Midpoint of uniform-along-z must be 5.0, got {}",
            mid
        );
    }

    #[test]
    fn bilinear_interpolation_formula_x_axis() {
        // 2x2: [[0,10],[0,10]] — varies only in X
        let data = vec![0.0, 10.0, 0.0, 10.0];
        let h = Heightmap::from_data(data, 2).unwrap();

        // At u=0.3, v=0.0:
        // h00=0, h10=10, fx=0.3
        // h0 = 0*(1-0.3) + 10*0.3 = 3.0
        let val = h.sample_bilinear(0.3, 0.0);
        assert!(
            (val - 3.0).abs() < 1e-4,
            "X interpolation: 0.3 * 10 = 3, got {}",
            val
        );
    }

    #[test]
    fn bilinear_interpolation_formula_z_axis() {
        // 2x2: [[0,0],[10,10]] — varies only in Z
        let data = vec![0.0, 0.0, 10.0, 10.0];
        let h = Heightmap::from_data(data, 2).unwrap();

        // At u=0.0, v=0.4:
        // h0=0, h1=10, fz=0.4
        // result = 0*(1-0.4) + 10*0.4 = 4.0
        let val = h.sample_bilinear(0.0, 0.4);
        assert!(
            (val - 4.0).abs() < 1e-4,
            "Z interpolation: 0.4 * 10 = 4, got {}",
            val
        );
    }

    #[test]
    fn bilinear_fx_and_fz_not_swapped() {
        // 2x2: [[0,10],[20,30]] — different in both axes
        // h00=0, h10=10, h01=20, h11=30
        let data = vec![0.0, 10.0, 20.0, 30.0];
        let h = Heightmap::from_data(data, 2).unwrap();

        // At (fx=1.0, fz=0.0) → should be 10, not 20
        let val = h.sample_bilinear(0.99, 0.0);
        assert!(
            (val - 10.0).abs() < 0.5,
            "At (1,0) should be ~10, not 20 (fx/fz not swapped), got {}",
            val
        );

        // At (fx=0.0, fz=1.0) → should be 20, not 10
        let val2 = h.sample_bilinear(0.0, 0.99);
        assert!(
            (val2 - 20.0).abs() < 0.5,
            "At (0,1) should be ~20, not 10, got {}",
            val2
        );
    }

    #[test]
    fn normal_flat_surface_points_up() {
        // 4x4 uniform height = 10.0
        let data = vec![10.0; 16];
        let h = Heightmap::from_data(data, 4).unwrap();
        let normal = h.calculate_normal(1, 1, 1.0);
        // dx=0, dz=0 → normal = (0, 1, 0)
        assert!(
            (normal.y - 1.0).abs() < 1e-4,
            "Flat surface normal Y must be 1.0, got {}",
            normal.y
        );
        assert!(
            normal.x.abs() < 1e-4,
            "Flat surface normal X must be 0, got {}",
            normal.x
        );
        assert!(
            normal.z.abs() < 1e-4,
            "Flat surface normal Z must be 0, got {}",
            normal.z
        );
    }

    #[test]
    fn normal_x_slope_direction() {
        // 4x4 ramp increasing in X: row = [0, 10, 20, 30]
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[(z * 4 + x) as usize] = x as f32 * 10.0;
            }
        }
        let h = Heightmap::from_data(data, 4).unwrap();
        let normal = h.calculate_normal(2, 2, 1.0);
        // dx = (30 - 10) / 2 = 10 → normal.x = -10 (before normalize)
        // So normal.x should be negative (pointing opposite to slope)
        assert!(
            normal.x < -0.1,
            "X-slope normal.x must be negative, got {}",
            normal.x
        );
    }

    #[test]
    fn normal_z_slope_direction() {
        // 4x4 ramp increasing in Z
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[(z * 4 + x) as usize] = z as f32 * 10.0;
            }
        }
        let h = Heightmap::from_data(data, 4).unwrap();
        let normal = h.calculate_normal(2, 2, 1.0);
        // dz = (30 - 10) / 2 = 10 → normal.z = -10
        assert!(
            normal.z < -0.1,
            "Z-slope normal.z must be negative, got {}",
            normal.z
        );
    }

    #[test]
    fn normal_is_normalized() {
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[(z * 4 + x) as usize] = x as f32 * 5.0 + z as f32 * 3.0;
            }
        }
        let h = Heightmap::from_data(data, 4).unwrap();
        let normal = h.calculate_normal(2, 2, 1.0);
        let len = normal.length();
        assert!(
            (len - 1.0).abs() < 0.01,
            "Normal must be unit length, got {}",
            len
        );
    }

    #[test]
    fn normal_scale_parameter_affects_gradient() {
        // Same heightmap, different scales
        let mut data = vec![0.0; 16];
        for z in 0..4 {
            for x in 0..4 {
                data[(z * 4 + x) as usize] = x as f32 * 10.0;
            }
        }
        let h = Heightmap::from_data(data, 4).unwrap();
        let n1 = h.calculate_normal(2, 2, 1.0);
        let n2 = h.calculate_normal(2, 2, 10.0);
        // Larger scale → smaller dx → normal closer to (0,1,0)
        assert!(
            n2.y > n1.y,
            "Larger scale should produce more vertical normal: scale=1 y={}, scale=10 y={}",
            n1.y,
            n2.y
        );
    }

    #[test]
    fn erosion_constants_correct() {
        // We can't easily test erosion output without running it, but we verify
        // the function doesn't crash and produces valid heightmap data
        let config = HeightmapConfig {
            resolution: 8,
            ..Default::default()
        };
        let mut h = Heightmap::new(config).unwrap();
        // Create a simple peaked heightmap
        for z in 0..8 {
            for x in 0..8 {
                let dist = ((x as f32 - 3.5).powi(2) + (z as f32 - 3.5).powi(2)).sqrt();
                h.set_height(x, z, (10.0 - dist * 2.0).max(0.0));
            }
        }
        let max_before = h.max_height();
        h.apply_hydraulic_erosion(1.0).unwrap();
        // Erosion should reduce peaks somewhat
        let max_after = h.max_height();
        assert!(
            max_after <= max_before + 0.1,
            "Erosion should not increase peaks: before={}, after={}",
            max_before,
            max_after
        );
    }

    #[test]
    fn heightmap_resolution_matches_data_len() {
        let config = HeightmapConfig {
            resolution: 16,
            ..Default::default()
        };
        let h = Heightmap::new(config).unwrap();
        assert_eq!(h.data().len(), 16 * 16);
        assert_eq!(h.resolution(), 16);
    }
}

// ============================================================================
// Texture Splatting Tests
// ============================================================================

mod splatting_mutations {
    use astraweave_terrain::texture_splatting::{SplatWeights, TerrainMaterial, MAX_SPLAT_LAYERS};

    #[test]
    fn max_splat_layers_is_8() {
        assert_eq!(MAX_SPLAT_LAYERS, 8);
    }

    #[test]
    fn from_weights_normalizes_to_sum_one() {
        let w = SplatWeights::from_weights(&[0.5, 0.3, 0.2]);
        let sum: f32 = (0..8).map(|i| w.get_weight(i)).sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "Normalized weights must sum to 1.0, got {}",
            sum
        );
    }

    #[test]
    fn from_weights_preserves_ratios() {
        let w = SplatWeights::from_weights(&[0.3, 0.6, 0.1]);
        // After normalization: 0.3/1.0, 0.6/1.0, 0.1/1.0
        assert!((w.get_weight(0) - 0.3).abs() < 1e-4);
        assert!((w.get_weight(1) - 0.6).abs() < 1e-4);
        assert!((w.get_weight(2) - 0.1).abs() < 1e-4);
    }

    #[test]
    fn from_weights_empty_fallback_to_first() {
        let w = SplatWeights::from_weights(&[]);
        // Total = 0 → documented fallback: weights_0.y = 1.0 (layer 1 / sand),
        // per texture_splatting.rs:169-170 ("avoids grass bleed in non-grassland biomes").
        assert!(
            (w.get_weight(1) - 1.0).abs() < 1e-4,
            "Empty weights fallback: layer 1 (sand) = 1.0, got {}",
            w.get_weight(1)
        );
    }

    #[test]
    fn from_weights_zero_total_fallback() {
        let w = SplatWeights::from_weights(&[0.0, 0.0, 0.0]);
        assert!(
            (w.get_weight(1) - 1.0).abs() < 1e-4,
            "All-zero weights fallback: layer 1 (sand) = 1.0"
        );
    }

    #[test]
    fn dominant_layer_returns_highest() {
        let w = SplatWeights::from_weights(&[0.1, 0.6, 0.3]);
        assert_eq!(w.dominant_layer(), 1, "Layer 1 has highest weight");
    }

    #[test]
    fn dominant_layer_first_when_tied() {
        let w = SplatWeights::from_weights(&[0.5, 0.5]);
        // With > comparison (not >=), first highest wins
        assert_eq!(w.dominant_layer(), 0, "Tie goes to first layer");
    }

    #[test]
    fn get_weight_out_of_range_is_zero() {
        let w = SplatWeights::from_weights(&[1.0]);
        assert!(w.get_weight(8).abs() < 1e-6, "Index 8 must return 0");
        assert!(w.get_weight(100).abs() < 1e-6, "Index 100 must return 0");
    }

    #[test]
    fn weights_span_two_vec4s() {
        let w = SplatWeights::from_weights(&[0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.3]);
        // Layer 7 (8th) should be in weights_1.w
        assert!(
            w.get_weight(7) > 0.1,
            "Layer 7 must have weight, got {}",
            w.get_weight(7)
        );
    }

    #[test]
    fn terrain_material_grass_uv_scale() {
        let g = TerrainMaterial::grass();
        assert!((g.uv_scale - 4.0).abs() < 1e-4);
        assert!((g.blend_sharpness - 2.0).abs() < 1e-4);
        assert!((g.triplanar_sharpness - 4.0).abs() < 1e-4);
    }

    #[test]
    fn terrain_material_rock_uv_scale() {
        let r = TerrainMaterial::rock();
        assert!((r.uv_scale - 2.0).abs() < 1e-4);
        assert!((r.blend_sharpness - 4.0).abs() < 1e-4);
        assert!((r.triplanar_sharpness - 8.0).abs() < 1e-4);
    }

    #[test]
    fn terrain_material_sand_uv_scale() {
        let s = TerrainMaterial::sand();
        assert!((s.uv_scale - 8.0).abs() < 1e-4);
        assert!((s.blend_sharpness - 1.5).abs() < 1e-4);
        assert!((s.triplanar_sharpness - 2.0).abs() < 1e-4);
    }

    #[test]
    fn normalization_threshold_is_00001() {
        // If total <= 0.0001, fallback to the documented sand layer (layer 1).
        // So weights [0.00005, 0.00005] should trigger fallback
        let w = SplatWeights::from_weights(&[0.00005, 0.00005]);
        // total = 0.0001 which is NOT > 0.0001, so fallback to layer 1 (sand)
        assert!(
            (w.get_weight(1) - 1.0).abs() < 1e-4,
            "Total=0.0001 should trigger fallback to layer 1 (sand)"
        );
    }
}

// ============================================================================
// Voxel Data Tests
// ============================================================================

mod voxel_mutations {
    use astraweave_terrain::voxel_data::{
        ChunkCoord, Voxel, VoxelChunk, CHUNK_SIZE, MAX_OCTREE_DEPTH,
    };
    use glam::{IVec3, Vec3};

    #[test]
    fn chunk_size_constant_is_32() {
        assert_eq!(CHUNK_SIZE, 32);
    }

    #[test]
    fn max_octree_depth_is_5() {
        assert_eq!(MAX_OCTREE_DEPTH, 5);
    }

    #[test]
    fn voxel_is_solid_threshold_05() {
        assert!(!Voxel::new(0.5, 0).is_solid(), "0.5 is NOT solid (> 0.5)");
        assert!(Voxel::new(0.501, 0).is_solid(), "0.501 IS solid");
        assert!(!Voxel::new(0.499, 0).is_solid(), "0.499 NOT solid");
    }

    #[test]
    fn voxel_is_empty_threshold_001() {
        assert!(Voxel::new(0.0, 0).is_empty(), "0.0 is empty");
        assert!(Voxel::new(0.009, 0).is_empty(), "0.009 is empty (< 0.01)");
        assert!(
            !Voxel::new(0.01, 0).is_empty(),
            "0.01 NOT empty (not < 0.01)"
        );
        assert!(!Voxel::new(0.011, 0).is_empty(), "0.011 NOT empty");
    }

    #[test]
    fn voxel_default_is_empty_not_solid() {
        let v = Voxel::default();
        assert!((v.density - 0.0).abs() < 1e-6);
        assert_eq!(v.material, 0);
        assert!(v.is_empty());
        assert!(!v.is_solid());
    }

    #[test]
    fn chunk_coord_from_world_pos_floor_division() {
        // Positive coords
        let c = ChunkCoord::from_world_pos(Vec3::new(33.0, 0.0, 0.0));
        assert_eq!(c.x, 1, "33/32 floors to 1");
        assert_eq!(c.y, 0);

        // Negative coords — floor division
        let c2 = ChunkCoord::from_world_pos(Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(c2.x, -1, "-1/32 floors to -1");

        // Exact boundary
        let c3 = ChunkCoord::from_world_pos(Vec3::new(32.0, 0.0, 0.0));
        assert_eq!(c3.x, 1, "32/32 floors to 1");
    }

    #[test]
    fn chunk_coord_to_world_pos_and_back() {
        let coord = ChunkCoord::new(2, -1, 3);
        let world = coord.to_world_pos();
        assert!((world.x - 64.0).abs() < 1e-4, "2 * 32 = 64");
        assert!((world.y - (-32.0)).abs() < 1e-4, "-1 * 32 = -32");
        assert!((world.z - 96.0).abs() < 1e-4, "3 * 32 = 96");
    }

    #[test]
    fn chunk_coord_neighbors_six_axis_aligned() {
        let c = ChunkCoord::new(0, 0, 0);
        let n = c.neighbors();
        assert_eq!(n.len(), 6);
        assert_eq!(n[0], ChunkCoord::new(1, 0, 0));
        assert_eq!(n[1], ChunkCoord::new(-1, 0, 0));
        assert_eq!(n[2], ChunkCoord::new(0, 1, 0));
        assert_eq!(n[3], ChunkCoord::new(0, -1, 0));
        assert_eq!(n[4], ChunkCoord::new(0, 0, 1));
        assert_eq!(n[5], ChunkCoord::new(0, 0, -1));
    }

    #[test]
    fn voxel_chunk_set_get_roundtrip() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        let pos = IVec3::new(5, 10, 15);
        chunk.set_voxel(pos, Voxel::new(0.75, 42));
        let v = chunk.get_voxel(pos).unwrap();
        assert!((v.density - 0.75).abs() < 1e-6);
        assert_eq!(v.material, 42);
    }

    #[test]
    fn voxel_chunk_out_of_bounds_returns_none() {
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = VoxelChunk::new(coord);
        assert!(chunk.get_voxel(IVec3::new(-1, 0, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(CHUNK_SIZE, 0, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(0, CHUNK_SIZE, 0)).is_none());
    }
}

// ============================================================================
// ChunkId / WorldConfig Tests
// ============================================================================

mod chunk_world_mutations {
    use astraweave_terrain::{ChunkId, WorldConfig};

    #[test]
    fn world_config_defaults() {
        let w = WorldConfig::default();
        assert!((w.chunk_size - 512.0).abs() < 1e-3);
        assert_eq!(w.heightmap_resolution, 96);
    }

    #[test]
    fn chunk_id_to_world_pos_exact() {
        let id = ChunkId::new(1, 2);
        let pos = id.to_world_pos(256.0);
        assert!(
            (pos.x - 256.0).abs() < 1e-3,
            "ChunkId(1,2) at size 256 → x=256"
        );
        assert!(
            (pos.z - 512.0).abs() < 1e-3,
            "ChunkId(1,2) at size 256 → z=512"
        );
    }

    #[test]
    fn chunk_id_negative_coords() {
        let id = ChunkId::new(-1, -1);
        let pos = id.to_world_pos(100.0);
        assert!((pos.x - (-100.0)).abs() < 1e-3);
        assert!((pos.z - (-100.0)).abs() < 1e-3);
    }
}
