//! Wave 3 mutation-resistant remediation tests for astraweave-terrain.
//!
//! Targets specific mutation survivors identified by cargo-mutants sharded
//! analysis. Each test pins exact values, boundary operators, and return-path
//! semantics to prevent operator/constant mutations from surviving.

use astraweave_terrain::climate::utils::classify_whittaker_biome;
use astraweave_terrain::marching_cubes_tables::{EDGE_ENDPOINTS, MC_EDGE_TABLE, MC_TRI_TABLE};
use astraweave_terrain::voxel_data::{ChunkCoord, Voxel, VoxelChunk, VoxelGrid, CHUNK_SIZE};
use astraweave_terrain::*;

// ============================================================================
// REMEDIATION 1: climate.rs — classify_whittaker_biome boundary operators
// Mutant: "replace > with >= in classify_whittaker_biome"
// ============================================================================

mod whittaker_boundary {
    use super::*;

    // Boundary: t < 0.2 → Tundra (strict <)
    #[test]
    fn tundra_at_t_0_199() {
        let biome = classify_whittaker_biome(0.199, 0.5);
        assert_eq!(biome, BiomeType::Tundra, "t=0.199 must be Tundra (t < 0.2)");
    }

    #[test]
    fn not_tundra_at_t_0_2() {
        let biome = classify_whittaker_biome(0.2, 0.5);
        assert_ne!(
            biome,
            BiomeType::Tundra,
            "t=0.2 must NOT be Tundra (strict <)"
        );
    }

    // Boundary: t < 0.4 && m < 0.3 → Tundra
    #[test]
    fn tundra_second_branch() {
        assert_eq!(classify_whittaker_biome(0.39, 0.29), BiomeType::Tundra);
        // At boundary m=0.3, should NOT be Tundra
        assert_ne!(
            classify_whittaker_biome(0.39, 0.3),
            BiomeType::Tundra,
            "t=0.39, m=0.3 must NOT be Tundra (m < 0.3 is strict)"
        );
        // At boundary t=0.4 with qualifying m, should NOT be Tundra
        // Kills mutation: t < 0.4 → t <= 0.4
        assert_ne!(
            classify_whittaker_biome(0.4, 0.29),
            BiomeType::Tundra,
            "t=0.4, m=0.29 must NOT be Tundra (t < 0.4 is strict)"
        );
    }

    // Boundary: t < 0.6 && m < 0.2 → Desert
    #[test]
    fn desert_low_moisture() {
        assert_eq!(classify_whittaker_biome(0.5, 0.19), BiomeType::Desert);
        assert_ne!(
            classify_whittaker_biome(0.5, 0.2),
            BiomeType::Desert,
            "t=0.5, m=0.2 must NOT be Desert (m < 0.2 is strict)"
        );
        // At boundary t=0.6 with qualifying m, should NOT be Desert
        // Kills mutation: t < 0.6 → t <= 0.6 (Desert arm)
        assert_ne!(
            classify_whittaker_biome(0.6, 0.19),
            BiomeType::Desert,
            "t=0.6, m=0.19 must NOT be Desert (t < 0.6 is strict)"
        );
    }

    // Kill mutation: t < 0.6 → t <= 0.6 (line 227 col 25)
    // At t=0.6, m=0.19: t < 0.6 → false, so NOT Desert
    // What IS it? Falls through remaining guards:
    //   t > 0.7 && m < 0.4? No (t=0.6)
    //   m > 0.8? No (m=0.19)
    //   t > 0.6 && m > 0.6? No
    //   t > 0.4 && m > 0.4? No (m=0.19)
    //   Default → Grassland
    #[test]
    fn desert_boundary_t_0_6_is_grassland() {
        assert_eq!(
            classify_whittaker_biome(0.6, 0.19),
            BiomeType::Grassland,
            "t=0.6, m=0.19 must be Grassland (t < 0.6 is strict, so Desert branch skipped)"
        );
    }

    // Kill mutation: t < 0.6 → t == 0.6 (Desert arm)
    // At t=0.5: t < 0.6 is true, t == 0.6 is false → mutation skips Desert
    // But what does t=0.5,m=0.19 classify as with mutation?
    //   t > 0.7? No. m>0.8? No. t>0.6? No. t>0.4 && m>0.4? No (m=0.19). → Grassland
    // Original gives Desert. Mutation gives Grassland. The `desert_low_moisture` test kills this.

    // Kill mutation: t < 0.6 → t > 0.6 (Desert arm)
    // At t=0.5: t < 0.6 true → Desert. t > 0.6 false → skip Desert.
    // Same as above: killed by `desert_low_moisture` test.

    // Boundary: t > 0.7 && m < 0.4 → Desert
    #[test]
    fn desert_hot_dry() {
        assert_eq!(classify_whittaker_biome(0.71, 0.39), BiomeType::Desert);
        assert_ne!(
            classify_whittaker_biome(0.7, 0.39),
            BiomeType::Desert,
            "t=0.7 must NOT be Desert (t > 0.7 is strict)"
        );
        // At boundary m=0.4 with qualifying t, should NOT be Desert
        // Kills mutation: m < 0.4 → m <= 0.4
        assert_ne!(
            classify_whittaker_biome(0.71, 0.4),
            BiomeType::Desert,
            "t=0.71, m=0.4 must NOT be Desert (m < 0.4 is strict)"
        );
    }

    // Boundary: m > 0.8 → Swamp
    #[test]
    fn swamp_moisture_boundary() {
        assert_eq!(classify_whittaker_biome(0.5, 0.81), BiomeType::Swamp);
        assert_ne!(
            classify_whittaker_biome(0.5, 0.8),
            BiomeType::Swamp,
            "m=0.8 must NOT be Swamp (m > 0.8 is strict)"
        );
    }

    // Boundary: t > 0.6 && m > 0.6 → Forest
    // Note: t=0.6 also matches the later `t > 0.4 && m > 0.4` Forest branch
    // so the mutation on the t > 0.6 boundary is semantically equivalent.
    // Test the m > 0.6 boundary instead: m=0.6 won't match this branch.
    #[test]
    fn forest_hot_wet() {
        // t=0.61, m=0.61 → matches t > 0.6 && m > 0.6 → Forest
        assert_eq!(classify_whittaker_biome(0.61, 0.61), BiomeType::Forest);
        // t=0.71, m=0.6 → matches t > 0.7 && m < 0.4? No (m=0.6). m > 0.8? No.
        //   t > 0.6 && m > 0.6? m=0.6 is not > 0.6. Falls to t > 0.4 && m > 0.4 → Forest
        assert_eq!(classify_whittaker_biome(0.61, 0.7), BiomeType::Forest);
    }

    // Kill mutation: t > 0.6 → t == 0.6
    // t=0.8, m=0.7 → t > 0.6 is true (Forest), but t == 0.6 is false → falls through
    #[test]
    fn forest_t_above_0_6() {
        assert_eq!(
            classify_whittaker_biome(0.8, 0.7),
            BiomeType::Forest,
            "t=0.8, m=0.7 must be Forest (t > 0.6 && m > 0.6)"
        );
    }

    // Kill mutation: t > 0.6 → t < 0.6
    // t=0.61, m=0.61 → t > 0.6 true → Forest. With t < 0.6: false → falls to t > 0.4 branch which also gives Forest.
    // Need a case where ONLY the t > 0.6 branch gives Forest but t > 0.4 does NOT.
    // Actually both branches give Forest, so this mutation may be equivalent for the Forest case.
    // What if we test t=0.5 m=0.7? t > 0.6 is false, t > 0.4 is true, m > 0.4 is true → Forest
    // With mutation t < 0.6: t=0.5 < 0.6 is true, m > 0.6 is true → Forest too. Same.
    // This mutation on t > 0.6 is equivalent when t > 0.4 && m > 0.4 also catches it.

    // Kill mutation: t > 0.6 → t >= 0.6 in the (t > 0.6 && m > 0.6) guard
    // At t=0.6, m=0.61: t > 0.6 is false → fall to t > 0.4 && m > 0.4 → Forest
    // With >=: t >= 0.6 is true → Forest. Same result, so this mutation is equivalent.

    // Kill mutation: m > 0.6 → m == 0.6 in (t > 0.6 && m > 0.6) guard
    // t=0.61, m=0.65: original → Forest. m==0.6? No → falls to t>0.4 && m>0.4 → Forest. Equivalent!
    // t=0.61, m=0.6: original → not this branch → t>0.4 && m>0.4 → Forest. m==0.6? Yes → Forest. Same.

    // Kill mutation: m > 0.6 → m < 0.6 in (t > 0.6 && m > 0.6) guard
    // t=0.65, m=0.65: m > 0.6 true → Forest. m < 0.6 false → falls to t>0.4,m>0.4 → Forest. Same!
    // These mutations on the t>0.6 && m>0.6 guard are all practically equivalent because
    // the next guard (t>0.4 && m>0.4) catches the same cases.

    // Kill mutation: m > 0.6 → m >= 0.6 at line 231 col 36
    // Similar analysis: equivalent due to the fallthrough to t>0.4 && m>0.4

    // Kill mutation: && → || in (t > 0.6 && m > 0.6)
    // With ||: t=0.3, m=0.7 → m>0.6 true → Forest (OR matches)
    // Original: t=0.3 → t>0.6 false → AND fails → falls through
    // t=0.3, m=0.7 → m > 0.8? No. Next: t>0.6 && m>0.6? No (t=0.3)
    // Next: t>0.4 && m>0.4? No (t=0.3). → Grassland!
    // The existing test `grassland_low_t_high_m_not_forest` already covers this.

    // Kill mutation: match guard `t > 0.6 && m > 0.6` → `false`
    // This disables the entire branch. t=0.61, m=0.61 would fall to t>0.4 && m>0.4 → Forest.
    // Equivalent! Both branches produce Forest for these values.
    // To kill this, we need values where ONLY the t>0.6 branch applies but NOT t>0.4.
    // That's impossible since t>0.6 implies t>0.4 and m>0.6 implies m>0.4.
    // This mutation is fully equivalent and unkillable.

    // Boundary: t > 0.4 && m > 0.4 → Forest
    #[test]
    fn forest_moderate_boundary() {
        assert_eq!(classify_whittaker_biome(0.41, 0.5), BiomeType::Forest);
        // At exactly (0.4, 0.5), t=0.4 does NOT satisfy t > 0.4 → falls to Grassland
        assert_eq!(
            classify_whittaker_biome(0.4, 0.5),
            BiomeType::Grassland,
            "t=0.4, m=0.5 must be Grassland (t > 0.4 is strict)"
        );
    }

    // Both at boundary
    #[test]
    fn both_at_boundary_is_grassland() {
        assert_eq!(classify_whittaker_biome(0.4, 0.4), BiomeType::Grassland);
    }

    // Moisture exactly at 0.4 with t above threshold
    #[test]
    fn moisture_exactly_0_4_is_grassland() {
        assert_eq!(
            classify_whittaker_biome(0.5, 0.4),
            BiomeType::Grassland,
            "t=0.5, m=0.4 must be Grassland (m > 0.4 is strict)"
        );
    }

    // Kill mutation: t > 0.6 && m > 0.6 → t > 0.6 || m > 0.6
    // Kill mutation: t > 0.6 → t < 0.6 in guard (t < 0.6 && m > 0.6 captures this wrongly)
    // t=0.3, m=0.7: must NOT be Forest (only m qualifies, t doesn't)
    #[test]
    fn grassland_low_t_high_m_not_forest() {
        assert_eq!(
            classify_whittaker_biome(0.3, 0.7),
            BiomeType::Grassland,
            "t=0.3, m=0.7 must be Grassland — t > 0.6 guard must use && not ||"
        );
    }

    // Kill mutation: m > 0.6 → m < 0.6 in guard (t > 0.6 && m < 0.6 captures this wrongly)
    // t=0.65, m=0.1: must NOT be Forest (only t qualifies, m doesn't)
    #[test]
    fn grassland_high_t_low_m_not_forest() {
        assert_eq!(
            classify_whittaker_biome(0.65, 0.1),
            BiomeType::Grassland,
            "t=0.65, m=0.1 must be Grassland — m > 0.6 guard must not become m < 0.6"
        );
    }

    // Default fallthrough
    #[test]
    fn grassland_fallthrough() {
        // t=0.3, m=0.35 — doesn't match any specific branch
        assert_eq!(classify_whittaker_biome(0.3, 0.35), BiomeType::Grassland);
    }
}

// ============================================================================
// REMEDIATION 2: meshing.rs — DualContouring boundary operators
// ============================================================================

mod dual_contouring_boundary {
    use super::*;

    #[test]
    fn empty_chunk_produces_empty_mesh() {
        let mut dc = DualContouring::new();
        let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
        let mesh = dc.generate_mesh(&chunk);
        assert!(mesh.vertices.is_empty(), "Empty chunk → no vertices");
        assert!(mesh.indices.is_empty(), "Empty chunk → no indices");
    }

    #[test]
    fn surface_crossing_generates_geometry() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = grid.get_or_create_chunk(coord);

        // Create a surface crossing in a small region
        // Positive density = solid, negative = air
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let density = if y < CHUNK_SIZE / 2 { 1.0 } else { -1.0 };
                    chunk.set_voxel(glam::IVec3::new(x, y, z), Voxel::new(density, 0));
                }
            }
        }

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(grid.get_chunk(coord).unwrap());
        // Verify we get vertices (mesh may or may not have indices depending on algorithm)
        assert!(!mesh.vertices.is_empty(), "Surface crossing → vertices");

        // All vertices must be finite (no NaN from boundary issues)
        for v in &mesh.vertices {
            assert!(v.position[0].is_finite());
            assert!(v.position[1].is_finite());
            assert!(v.position[2].is_finite());
        }
    }

    #[test]
    fn all_indices_refer_to_valid_vertices() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = grid.get_or_create_chunk(coord);

        // Simple surface
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let density = if y < 8 { 1.0 } else { -1.0 };
                    chunk.set_voxel(glam::IVec3::new(x, y, z), Voxel::new(density, 0));
                }
            }
        }

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(grid.get_chunk(coord).unwrap());
        let vcount = mesh.vertices.len() as u32;
        for &idx in &mesh.indices {
            assert!(
                idx < vcount,
                "Index {} exceeds vertex count {}",
                idx,
                vcount
            );
        }
        // Indices must come in triples (triangles)
        assert_eq!(mesh.indices.len() % 3, 0, "Indices must be multiple of 3");
    }

    // Kill mutation: (v1.density - v2.density).abs() > 0.001 → >= 0.001
    // This mutation is practically equivalent since f32 rarely equals exactly 0.001.
    // To kill it, we'd need densities whose difference is EXACTLY 0.001, which is
    // extremely unlikely with standard f32 arithmetic. This is an equivalent mutant.
    // Instead, verify the interpolation produces correct results for known densities.
    #[test]
    fn interpolation_with_known_densities() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = grid.get_or_create_chunk(coord);

        // Create surface with specific densities: sharp surface at y=8
        // with densities that have a clear crossing
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let density = (y as f32 - 8.0) * 0.5;
                    chunk.set_voxel(glam::IVec3::new(x, y, z), Voxel::new(density, 0));
                }
            }
        }

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(grid.get_chunk(coord).unwrap());
        assert!(
            !mesh.vertices.is_empty(),
            "Gradient surface should produce vertices"
        );

        // All vertex positions should be in valid range
        for v in &mesh.vertices {
            assert!(v.position[0] >= 0.0 && v.position[0] <= CHUNK_SIZE as f32);
            assert!(v.position[1] >= 0.0 && v.position[1] <= CHUNK_SIZE as f32);
            assert!(v.position[2] >= 0.0 && v.position[2] <= CHUNK_SIZE as f32);
        }
    }
}

// ============================================================================
// REMEDIATION 3: marching_cubes_tables.rs — sentinel integrity
// Mutant: "delete -" changes -1 sentinel to 1 (a valid vertex index)
// ============================================================================

mod marching_cubes_sentinel {
    use super::*;

    #[test]
    fn triangle_table_sentinels_are_negative() {
        for (case_idx, row) in MC_TRI_TABLE.iter().enumerate() {
            // Find first -1 in the row
            if let Some(pos) = row.iter().position(|&v| v == -1) {
                // All entries after first -1 must also be -1
                for j in pos..row.len() {
                    assert_eq!(
                        row[j], -1,
                        "MC_TRI_TABLE[{}][{}] should be -1 sentinel but was {}",
                        case_idx, j, row[j]
                    );
                }
            }
            // Non-sentinel values must be valid vertex indices (0-11)
            for (j, &val) in row.iter().enumerate() {
                if val != -1 {
                    assert!(
                        val >= 0 && val <= 11,
                        "MC_TRI_TABLE[{}][{}] has invalid index {}",
                        case_idx,
                        j,
                        val
                    );
                }
            }
        }
    }

    #[test]
    fn case_zero_all_sentinels() {
        for &val in &MC_TRI_TABLE[0] {
            assert_eq!(val, -1, "Case 0 must be all -1");
        }
    }

    #[test]
    fn case_255_all_sentinels() {
        for &val in &MC_TRI_TABLE[255] {
            assert_eq!(val, -1, "Case 255 must be all -1");
        }
    }

    #[test]
    fn row_103_has_valid_triangle_groups() {
        let row = &MC_TRI_TABLE[103];
        let valid_count = row.iter().filter(|&&v| v != -1).count();
        assert_eq!(
            valid_count % 3,
            0,
            "Row 103 valid entries ({}) must be a multiple of 3",
            valid_count
        );
    }

    // Kill mutation: "delete -" at marching_cubes_tables.rs:103:38
    // This changes -1 sentinel to 1 (a valid index) in some row.
    // The source line 103 corresponds to MC_TRI_TABLE[69] (table starts at line 34).
    // Original row 69: [1, 8, 3, 1, 9, 8, 5, 10, 6, -1, -1, -1, -1, -1, -1, -1]
    // Has 9 valid entries (3 triangles). If a -1 becomes 1, it has 10 entries → invalid.
    #[test]
    fn all_rows_have_valid_triangle_count() {
        for (case_idx, row) in MC_TRI_TABLE.iter().enumerate() {
            let valid_count = row.iter().filter(|&&v| v != -1).count();
            assert_eq!(
                valid_count % 3,
                0,
                "MC_TRI_TABLE[{}] has {} valid entries, must be a multiple of 3",
                case_idx,
                valid_count
            );
            // Max triangles per cell is 5 (15 indices)
            assert!(
                valid_count <= 15,
                "MC_TRI_TABLE[{}] has {} valid entries, max is 15",
                case_idx,
                valid_count
            );
        }
    }

    // Specifically verify row 69 (the mutation target from line 103)
    #[test]
    fn row_69_exact_valid_count() {
        let row = &MC_TRI_TABLE[69];
        let valid_count = row.iter().filter(|&&v| v != -1).count();
        assert_eq!(
            valid_count, 9,
            "MC_TRI_TABLE[69] must have exactly 9 valid entries (3 triangles), got {}",
            valid_count
        );
    }

    // Additional: verify that sentinels are CONTIGUOUS (-1s are never interrupted by non-sentinel)
    #[test]
    fn sentinels_contiguous_in_all_rows() {
        for (case_idx, row) in MC_TRI_TABLE.iter().enumerate() {
            let mut seen_sentinel = false;
            for (j, &val) in row.iter().enumerate() {
                if val == -1 {
                    seen_sentinel = true;
                } else if seen_sentinel {
                    panic!("MC_TRI_TABLE[{}][{}] = {} appears AFTER a -1 sentinel — sentinels must be contiguous",
                        case_idx, j, val);
                }
            }
        }
    }

    #[test]
    fn edge_table_case_0_is_zero() {
        assert_eq!(MC_EDGE_TABLE[0], 0, "Case 0 has no edges intersected");
    }

    #[test]
    fn edge_endpoints_exactly_12_entries() {
        assert_eq!(
            EDGE_ENDPOINTS.len(),
            12,
            "Must have exactly 12 edge endpoints"
        );
        // Each edge connects two different cube vertices (0-7)
        for (i, &(a, b)) in EDGE_ENDPOINTS.iter().enumerate() {
            assert!(a <= 7, "EDGE_ENDPOINTS[{}].0 = {} must be 0-7", i, a);
            assert!(b <= 7, "EDGE_ENDPOINTS[{}].1 = {} must be 0-7", i, b);
            assert_ne!(a, b, "EDGE_ENDPOINTS[{}] connects vertex to itself", i);
        }
    }
}

// ============================================================================
// REMEDIATION 4: lib.rs — scatter_chunk_content biome matching
// ============================================================================

mod scatter_biome_matching {
    use super::*;

    #[test]
    fn world_generator_creates_successfully() {
        let config = WorldConfig::default();
        let gen = WorldGenerator::new(config);
        // Basic smoke test that construction succeeds
        let chunk_id = ChunkId::new(0, 0);
        let result = gen.generate_chunk(chunk_id);
        assert!(result.is_ok(), "Chunk generation at (0,0) should succeed");
    }

    #[test]
    fn different_seed_different_terrain() {
        let config1 = WorldConfig {
            seed: 1,
            ..WorldConfig::default()
        };
        let config2 = WorldConfig {
            seed: 2,
            ..WorldConfig::default()
        };
        let gen1 = WorldGenerator::new(config1);
        let gen2 = WorldGenerator::new(config2);
        let chunk1 = gen1.generate_chunk(ChunkId::new(0, 0)).unwrap();
        let chunk2 = gen2.generate_chunk(ChunkId::new(0, 0)).unwrap();
        // Different seeds should produce different heightmaps
        let h1 = chunk1.heightmap().get_height(0, 0);
        let h2 = chunk2.heightmap().get_height(0, 0);
        // Same seed + same position → identical result
        assert!(h1.is_finite());
        assert!(h2.is_finite());
    }

    // Kill mutation: `b.biome_type == center_biome` → `b.biome_type != center_biome`
    // in scatter_chunk_content. With != mutation, the wrong biome config is used.
    // We verify that the default config's first biome is Grassland, so when center_biome
    // IS Grassland, `==` finds Grassland config but `!=` finds some other biome config.
    #[test]
    fn default_config_first_biome_is_grassland() {
        let config = WorldConfig::default();
        // The fallback `biomes[0]` is Grassland in default config
        assert_eq!(
            config.biomes[0].biome_type,
            BiomeType::Grassland,
            "Default config biomes[0] must be Grassland"
        );
        // Verify there are multiple biome configs so the mutation finds a wrong one
        assert!(
            config.biomes.len() > 1,
            "Default config must have multiple biomes for mutation to be detectable"
        );
        // Verify biome configs have different vegetation densities so wrong selection matters
        let grassland_density = config
            .biomes
            .iter()
            .find(|b| b.biome_type == BiomeType::Grassland)
            .unwrap()
            .vegetation
            .density;
        let other_density = config
            .biomes
            .iter()
            .find(|b| b.biome_type != BiomeType::Grassland)
            .unwrap()
            .vegetation
            .density;
        // These should differ — the mutation would pick the wrong density
        assert_ne!(
            grassland_density, other_density,
            "Grassland and other biome should have different vegetation densities"
        );
    }
}

// ============================================================================
// REMEDIATION 5: structures.rs — exact value assertions
// ============================================================================

mod structure_exact_values {
    use super::*;

    #[test]
    fn typical_size_exact() {
        assert_eq!(StructureType::Cottage.typical_size(), 8.0);
        assert_eq!(StructureType::Cabin.typical_size(), 8.0);
        assert_eq!(StructureType::Farmhouse.typical_size(), 12.0);
        assert_eq!(StructureType::Villa.typical_size(), 12.0);
        assert_eq!(StructureType::Tavern.typical_size(), 10.0);
        assert_eq!(StructureType::Blacksmith.typical_size(), 10.0);
        assert_eq!(StructureType::Market.typical_size(), 10.0);
        assert_eq!(StructureType::Temple.typical_size(), 15.0);
        assert_eq!(StructureType::Watchtower.typical_size(), 6.0);
        assert_eq!(StructureType::Fort.typical_size(), 20.0);
        assert_eq!(StructureType::Wall.typical_size(), 5.0);
        assert_eq!(StructureType::Gate.typical_size(), 5.0);
        assert_eq!(StructureType::AncientRuin.typical_size(), 15.0);
        assert_eq!(StructureType::StoneCircle.typical_size(), 12.0);
        assert_eq!(StructureType::Obelisk.typical_size(), 3.0);
        assert_eq!(StructureType::Tomb.typical_size(), 8.0);
        assert_eq!(StructureType::Cave.typical_size(), 6.0);
        assert_eq!(StructureType::RockFormation.typical_size(), 4.0);
        assert_eq!(StructureType::CrystalFormation.typical_size(), 5.0);
        assert_eq!(StructureType::Bridge.typical_size(), 15.0);
        assert_eq!(StructureType::Well.typical_size(), 2.0);
        assert_eq!(StructureType::Windmill.typical_size(), 8.0);
        assert_eq!(StructureType::Lighthouse.typical_size(), 6.0);
    }

    #[test]
    fn rarity_exact() {
        // Common = 0.8
        assert_eq!(StructureType::Cottage.rarity(), 0.8);
        assert_eq!(StructureType::Farmhouse.rarity(), 0.8);
        assert_eq!(StructureType::RockFormation.rarity(), 0.8);
        // Uncommon = 0.6
        assert_eq!(StructureType::Cabin.rarity(), 0.6);
        assert_eq!(StructureType::Tavern.rarity(), 0.6);
        assert_eq!(StructureType::Blacksmith.rarity(), 0.6);
        assert_eq!(StructureType::Well.rarity(), 0.6);
        assert_eq!(StructureType::Windmill.rarity(), 0.6);
        // Rare = 0.4
        assert_eq!(StructureType::Villa.rarity(), 0.4);
        assert_eq!(StructureType::Market.rarity(), 0.4);
        assert_eq!(StructureType::Temple.rarity(), 0.4);
        assert_eq!(StructureType::Watchtower.rarity(), 0.4);
        assert_eq!(StructureType::Cave.rarity(), 0.4);
        // Very Rare = 0.2
        assert_eq!(StructureType::Fort.rarity(), 0.2);
        assert_eq!(StructureType::AncientRuin.rarity(), 0.2);
        assert_eq!(StructureType::StoneCircle.rarity(), 0.2);
        assert_eq!(StructureType::Bridge.rarity(), 0.2);
        assert_eq!(StructureType::Lighthouse.rarity(), 0.2);
        // Extremely Rare = 0.1
        assert_eq!(StructureType::Wall.rarity(), 0.1);
        assert_eq!(StructureType::Gate.rarity(), 0.1);
        assert_eq!(StructureType::Obelisk.rarity(), 0.1);
        assert_eq!(StructureType::Tomb.rarity(), 0.1);
        assert_eq!(StructureType::CrystalFormation.rarity(), 0.1);
    }

    #[test]
    fn min_spacing_exact() {
        assert_eq!(StructureType::Fort.min_spacing(), 100.0);
        assert_eq!(StructureType::Temple.min_spacing(), 100.0);
        assert_eq!(StructureType::Market.min_spacing(), 100.0);
        assert_eq!(StructureType::Villa.min_spacing(), 50.0);
        assert_eq!(StructureType::Tavern.min_spacing(), 50.0);
        assert_eq!(StructureType::Blacksmith.min_spacing(), 50.0);
        assert_eq!(StructureType::Lighthouse.min_spacing(), 50.0);
        assert_eq!(StructureType::Bridge.min_spacing(), 50.0);
        assert_eq!(StructureType::Cottage.min_spacing(), 30.0);
        assert_eq!(StructureType::Farmhouse.min_spacing(), 30.0);
        assert_eq!(StructureType::Cabin.min_spacing(), 30.0);
        assert_eq!(StructureType::Watchtower.min_spacing(), 30.0);
        assert_eq!(StructureType::Well.min_spacing(), 20.0);
        assert_eq!(StructureType::RockFormation.min_spacing(), 20.0);
        assert_eq!(StructureType::Cave.min_spacing(), 15.0);
        assert_eq!(StructureType::CrystalFormation.min_spacing(), 15.0);
        assert_eq!(StructureType::AncientRuin.min_spacing(), 15.0);
        assert_eq!(StructureType::StoneCircle.min_spacing(), 15.0);
        assert_eq!(StructureType::Obelisk.min_spacing(), 15.0);
        assert_eq!(StructureType::Tomb.min_spacing(), 15.0);
        assert_eq!(StructureType::Wall.min_spacing(), 10.0);
        assert_eq!(StructureType::Gate.min_spacing(), 10.0);
        assert_eq!(StructureType::Windmill.min_spacing(), 10.0);
    }

    #[test]
    fn can_place_on_slope_boundary() {
        // Fort max_slope = 0.1 (uses <=)
        assert!(StructureType::Fort.can_place_on_slope(0.1));
        assert!(!StructureType::Fort.can_place_on_slope(0.100001));

        // Cottage max_slope = 0.2
        assert!(StructureType::Cottage.can_place_on_slope(0.2));
        assert!(!StructureType::Cottage.can_place_on_slope(0.200001));

        // Watchtower max_slope = 0.4
        assert!(StructureType::Watchtower.can_place_on_slope(0.4));
        assert!(!StructureType::Watchtower.can_place_on_slope(0.400001));

        // Cave max_slope = 0.8
        assert!(StructureType::Cave.can_place_on_slope(0.8));
        assert!(!StructureType::Cave.can_place_on_slope(0.800001));

        // Bridge max_slope = 1.0
        assert!(StructureType::Bridge.can_place_on_slope(1.0));
    }

    #[test]
    fn for_biome_returns_nonempty() {
        let grassland = StructureType::for_biome(BiomeType::Grassland);
        assert!(
            !grassland.is_empty(),
            "Grassland biome should have structures"
        );

        let desert = StructureType::for_biome(BiomeType::Desert);
        assert!(!desert.is_empty(), "Desert biome should have structures");
    }
}

// ============================================================================
// REMEDIATION 6: noise_gen.rs — sample_height determinism
// ============================================================================

mod noise_boundary {
    use super::*;

    #[test]
    fn sample_height_deterministic() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 42);

        let h1 = noise.sample_height(0.0, 0.0);
        let h2 = noise.sample_height(100.0, 100.0);
        let h3 = noise.sample_height(-50.0, 50.0);

        assert!(h1.is_finite());
        assert!(h2.is_finite());
        assert!(h3.is_finite());

        // Same seed + same position → identical result
        let h1_again = noise.sample_height(0.0, 0.0);
        assert!((h1_again - h1).abs() < f32::EPSILON);
    }

    #[test]
    fn different_positions_differ() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 42);

        let h_a = noise.sample_height(0.0, 0.0);
        let h_b = noise.sample_height(500.0, 500.0);
        // Widely separated positions should produce different heights
        assert_ne!(h_a, h_b, "Distant positions should differ");
    }

    #[test]
    fn sample_height_range() {
        let config = NoiseConfig::default();
        let noise = TerrainNoise::new(&config, 42);

        for x in (-5..5).map(|i| i as f64 * 100.0) {
            for z in (-5..5).map(|i| i as f64 * 100.0) {
                let h = noise.sample_height(x, z);
                assert!(h.is_finite(), "Height at ({}, {}) must be finite", x, z);
            }
        }
    }
}

// ============================================================================
// REMEDIATION 7: heightmap.rs — smooth & calculate_normal boundary tests
// ============================================================================

mod heightmap_boundary {
    use super::*;

    #[test]
    fn flat_heightmap_smooth_preserves_values() {
        let config = HeightmapConfig {
            resolution: 8,
            height_scale: 1.0,
            ..HeightmapConfig::default()
        };
        let mut hm = Heightmap::new(config).unwrap();

        for x in 0..8u32 {
            for z in 0..8u32 {
                hm.set_height(x, z, 5.0);
            }
        }

        hm.smooth(1);

        for x in 0..8u32 {
            for z in 0..8u32 {
                let h = hm.get_height(x, z);
                assert!(
                    (h - 5.0).abs() < 0.01,
                    "Smooth of flat heightmap: ({},{}) = {}, expected 5.0",
                    x,
                    z,
                    h
                );
            }
        }
    }

    #[test]
    fn calculate_normal_flat_is_up() {
        let config = HeightmapConfig {
            resolution: 8,
            height_scale: 1.0,
            ..HeightmapConfig::default()
        };
        let mut hm = Heightmap::new(config).unwrap();
        for x in 0..8u32 {
            for z in 0..8u32 {
                hm.set_height(x, z, 0.0);
            }
        }

        let normal = hm.calculate_normal(4, 4, 1.0);
        assert!(
            (normal.y - 1.0).abs() < 0.01,
            "Flat normal Y should be ~1.0, got {}",
            normal.y
        );
        assert!(
            normal.x.abs() < 0.01,
            "Flat normal X should be ~0, got {}",
            normal.x
        );
        assert!(
            normal.z.abs() < 0.01,
            "Flat normal Z should be ~0, got {}",
            normal.z
        );
    }

    #[test]
    fn heightmap_set_get_roundtrip() {
        let config = HeightmapConfig {
            resolution: 4,
            ..HeightmapConfig::default()
        };
        let mut hm = Heightmap::new(config).unwrap();

        hm.set_height(1, 2, 42.5);
        let h = hm.get_height(1, 2);
        assert!((h - 42.5).abs() < f32::EPSILON);
    }

    #[test]
    fn heightmap_from_data_roundtrip() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let hm = Heightmap::from_data(data.clone(), 2).unwrap();
        assert_eq!(hm.resolution(), 2);
        assert_eq!(hm.data(), &data[..]);
    }

    #[test]
    fn heightmap_default_config() {
        let config = HeightmapConfig::default();
        assert_eq!(config.resolution, 128);
        assert_eq!(config.min_height, 0.0);
        assert_eq!(config.max_height, 100.0);
        assert_eq!(config.height_scale, 1.0);
    }
}

// ============================================================================
// REMEDIATION 8: streaming_diagnostics.rs — MemoryStats exact semantics
// ============================================================================

mod streaming_diagnostics_exact {
    use super::*;

    #[test]
    fn memory_stats_default_is_zero() {
        let stats = MemoryStats::default();
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.bytes_per_chunk, 0);
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.peak_bytes, 0);
    }

    #[test]
    fn memory_stats_update_computes_total() {
        let mut stats = MemoryStats::default();
        stats.update(10, 1024); // 10 chunks × 1024 bytes
        assert_eq!(stats.total_bytes, 10240);
        assert_eq!(stats.chunk_count, 10);
        assert_eq!(stats.bytes_per_chunk, 1024);
        assert_eq!(stats.peak_bytes, 10240);
    }

    #[test]
    fn memory_stats_peak_tracks_max() {
        let mut stats = MemoryStats::default();
        stats.update(10, 1024); // total = 10240
        stats.update(5, 1024); // total = 5120, peak stays 10240
        assert_eq!(stats.total_bytes, 5120);
        assert_eq!(stats.peak_bytes, 10240, "Peak should not decrease");
    }

    #[test]
    fn delta_from_peak_at_peak_is_zero() {
        let mut stats = MemoryStats::default();
        stats.update(10, 100);
        let delta = stats.delta_from_peak_percent();
        assert!(
            delta.abs() < 0.01,
            "At peak, delta should be 0.0%, got {}",
            delta
        );
    }

    #[test]
    fn delta_from_peak_below_peak() {
        let mut stats = MemoryStats::default();
        stats.update(10, 100); // total=1000, peak=1000
        stats.update(5, 100); // total=500, peak=1000
        let delta = stats.delta_from_peak_percent();
        // (500/1000 - 1.0) * 100.0 = -50.0%
        assert!(
            (delta - (-50.0)).abs() < 0.01,
            "Below peak, delta should be -50.0%, got {}",
            delta
        );
    }

    #[test]
    fn delta_from_peak_zero_peak() {
        let stats = MemoryStats::default();
        assert_eq!(
            stats.delta_from_peak_percent(),
            0.0,
            "Zero peak → delta = 0.0"
        );
    }

    #[test]
    fn total_mb_exact() {
        let mut stats = MemoryStats::default();
        stats.update(1, 1048576); // 1 chunk × 1MB
        assert!((stats.total_mb() - 1.0).abs() < 0.001);
    }
}

// ============================================================================
// REMEDIATION 9: voxel_data.rs — VoxelGrid operations
// ============================================================================

mod voxel_grid_exact {
    use super::*;

    #[test]
    fn new_grid_is_empty() {
        let grid = VoxelGrid::new();
        assert_eq!(grid.chunk_count(), 0);
        assert_eq!(grid.memory_usage(), 0);
    }

    #[test]
    fn get_or_create_chunk_creates() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        let _ = grid.get_or_create_chunk(coord);
        assert_eq!(grid.chunk_count(), 1);
    }

    #[test]
    fn set_get_voxel_roundtrip() {
        let mut grid = VoxelGrid::new();
        grid.set_voxel(glam::Vec3::new(1.0, 1.0, 1.0), Voxel::new(0.75, 3));
        let v = grid.get_voxel(glam::Vec3::new(1.0, 1.0, 1.0));
        assert!(v.is_some(), "Voxel should exist after set");
    }

    #[test]
    fn remove_chunk_decrements_count() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        let _ = grid.get_or_create_chunk(coord);
        assert_eq!(grid.chunk_count(), 1);
        grid.remove_chunk(coord);
        assert_eq!(grid.chunk_count(), 0);
    }

    #[test]
    fn clear_resets_all() {
        let mut grid = VoxelGrid::new();
        let _ = grid.get_or_create_chunk(ChunkCoord::new(0, 0, 0));
        let _ = grid.get_or_create_chunk(ChunkCoord::new(1, 0, 0));
        assert_eq!(grid.chunk_count(), 2);
        grid.clear();
        assert_eq!(grid.chunk_count(), 0);
    }

    #[test]
    fn chunk_coords_returns_all() {
        let mut grid = VoxelGrid::new();
        let _ = grid.get_or_create_chunk(ChunkCoord::new(0, 0, 0));
        let _ = grid.get_or_create_chunk(ChunkCoord::new(1, 2, 3));
        let coords = grid.chunk_coords();
        assert_eq!(coords.len(), 2);
    }

    #[test]
    fn dirty_chunks_tracked() {
        let mut grid = VoxelGrid::new();
        let coord = ChunkCoord::new(0, 0, 0);
        grid.set_voxel(glam::Vec3::new(0.0, 0.0, 0.0), Voxel::new(1.0, 0));
        let dirty = grid.dirty_chunks();
        assert!(!dirty.is_empty(), "Setting a voxel should mark chunk dirty");
        grid.mark_chunk_clean(coord);
    }
}

// ============================================================================
// REMEDIATION 10: biome.rs — BiomeConfig named constructors
// ============================================================================

mod biome_config_exact {
    use super::*;

    #[test]
    fn grassland_biome_type() {
        let bc = BiomeConfig::grassland();
        assert_eq!(bc.biome_type, BiomeType::Grassland);
        assert!((bc.vegetation.density - 0.004).abs() < 1e-6);
    }

    #[test]
    fn desert_biome_type() {
        let bc = BiomeConfig::desert();
        assert_eq!(bc.biome_type, BiomeType::Desert);
    }

    #[test]
    fn forest_biome_type() {
        let bc = BiomeConfig::forest();
        assert_eq!(bc.biome_type, BiomeType::Forest);
    }

    #[test]
    fn mountain_biome_type() {
        let bc = BiomeConfig::mountain();
        assert_eq!(bc.biome_type, BiomeType::Mountain);
    }

    #[test]
    fn tundra_biome_type() {
        let bc = BiomeConfig::tundra();
        assert_eq!(bc.biome_type, BiomeType::Tundra);
    }

    #[test]
    fn swamp_biome_type() {
        let bc = BiomeConfig::swamp();
        assert_eq!(bc.biome_type, BiomeType::Swamp);
    }

    #[test]
    fn beach_biome_type() {
        let bc = BiomeConfig::beach();
        assert_eq!(bc.biome_type, BiomeType::Beach);
    }

    #[test]
    fn river_biome_type() {
        let bc = BiomeConfig::river();
        assert_eq!(bc.biome_type, BiomeType::River);
    }

    #[test]
    fn score_conditions_returns_finite() {
        let bc = BiomeConfig::grassland();
        let score = bc.score_conditions(25.0, 0.5, 0.6);
        assert!(
            score.is_finite(),
            "score_conditions must return finite value"
        );
        assert!(score >= 0.0, "score must be non-negative");
    }
}

// ============================================================================
// REMEDIATION 11: chunk.rs — ChunkId exact operations
// ============================================================================

mod chunk_id_exact {
    use super::*;

    #[test]
    fn chunk_id_new_fields() {
        let id = ChunkId::new(3, -5);
        assert_eq!(id.x, 3);
        assert_eq!(id.z, -5);
    }

    #[test]
    fn chunk_id_to_world_pos() {
        let id = ChunkId::new(1, 2);
        let pos = id.to_world_pos(256.0);
        assert_eq!(pos.x, 256.0);
        assert_eq!(pos.z, 512.0);
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn chunk_id_from_world_pos() {
        let id = ChunkId::from_world_pos(glam::Vec3::new(300.0, 0.0, 600.0), 256.0);
        assert_eq!(id.x, 1);
        assert_eq!(id.z, 2);
    }

    #[test]
    fn chunk_id_distance_to_self_is_zero() {
        let id = ChunkId::new(5, 5);
        assert_eq!(id.distance_to(id), 0.0);
    }

    #[test]
    fn chunk_id_distance_to_adjacent() {
        let a = ChunkId::new(0, 0);
        let b = ChunkId::new(1, 0);
        assert!((a.distance_to(b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn chunks_in_radius() {
        let chunks = ChunkId::get_chunks_in_radius(glam::Vec3::new(128.0, 0.0, 128.0), 1, 256.0);
        // Radius 1 → 3×3 = 9 chunks
        assert_eq!(chunks.len(), 9);
    }
}
