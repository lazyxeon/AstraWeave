//! Shard 19 remediation tests — structures position bounds, density-area math,
//! and p99_frame_time percentile index calculation.
//!
//! Targets MISSED mutations in:
//!   - structures.rs:357  — max_structures area*density arithmetic
//!   - structures.rs:388-399 — try_place_structure position math
//!   - streaming_diagnostics.rs:101 — p99 percentile index (* 0.99)

use astraweave_terrain::biome::BiomeType;
use astraweave_terrain::streaming_diagnostics::HitchDetector;
use astraweave_terrain::structures::{StructureConfig, StructureGenerator};

// ═══════════════════════════════════════════════════════════════════════
// p99_frame_time — percentile index math
// The formula is: index = ((len * 0.99).ceil() as usize).min(len - 1)
// Previous tests used small collections where .min(len-1) always clips.
// Need 200+ frames where the p99 index falls BELOW the max.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn p99_200_frames_unclamped_index() {
    // 200 frames: len*0.99 = 198.0, ceil=198, min(198,199)=198
    // With + mutation: 200+0.99=200.99, ceil=201, min(201,199)=199
    // With / mutation: 200/0.99=202.02, ceil=203, min(203,199)=199
    // sorted: [1.0 x198, 2.0, 999.0]
    // Correct: index 198 → 2.0
    // Mutated: index 199 → 999.0
    let mut d = HitchDetector::new(300, 1000.0);
    for _ in 0..198 {
        d.record_frame(1.0);
    }
    d.record_frame(2.0);
    d.record_frame(999.0);
    let p99 = d.p99_frame_time();
    assert!(
        (p99 - 2.0).abs() < 0.01,
        "p99 with 200 frames should be 2.0 (index 198), got {}",
        p99
    );
}

#[test]
fn p99_500_frames_percentile_accuracy() {
    // 500 frames: len*0.99 = 495.0, ceil=495, min(495,499)=495
    // With + mutation: 500+0.99=500.99, ceil=501, min(501,499)=499
    // sorted: [10.0 x495, 20.0 x4, 100.0]
    // Correct: index 495 → 20.0
    // Mutated: index 499 → 100.0
    let mut d = HitchDetector::new(600, 1000.0);
    for _ in 0..495 {
        d.record_frame(10.0);
    }
    for _ in 0..4 {
        d.record_frame(20.0);
    }
    d.record_frame(100.0);
    let p99 = d.p99_frame_time();
    assert!(
        (p99 - 20.0).abs() < 0.01,
        "p99 with 500 frames should be 20.0 (index 495), got {}",
        p99
    );
}

#[test]
fn p99_1000_frames_extreme() {
    // 1000 frames: len*0.99=990, ceil=990, min(990,999)=990
    // With + mutation: 1000+0.99=1000.99, ceil=1001, min(1001,999)=999
    let mut d = HitchDetector::new(1100, 5000.0);
    for _ in 0..990 {
        d.record_frame(5.0);
    }
    for _ in 0..9 {
        d.record_frame(50.0);
    }
    d.record_frame(500.0); // The 1000th frame, index 999
    let p99 = d.p99_frame_time();
    assert!(
        (p99 - 50.0).abs() < 0.01,
        "p99 with 1000 frames should be 50.0 (index 990), got {}",
        p99
    );
}

#[test]
fn p99_300_frames_boundary() {
    // 300 frames: len*0.99=297.0, ceil=297, min(297,299)=297
    // With mutations the index shifts to 299
    let mut d = HitchDetector::new(400, 5000.0);
    for _ in 0..297 {
        d.record_frame(10.0);
    }
    d.record_frame(15.0); // index 297
    d.record_frame(20.0); // index 298
    d.record_frame(99.0); // index 299
    let p99 = d.p99_frame_time();
    assert!(
        (p99 - 15.0).abs() < 0.01,
        "p99 with 300 frames should be 15.0 (index 297), got {}",
        p99
    );
}

// ═══════════════════════════════════════════════════════════════════════
// StructureGenerator — position bounds & density-area math
//
// Mutations targeted:
// L357: max_structures = (chunk_size * chunk_size / 2000.0 * density)
//   - chunk_size * chunk_size (area) with * → + or /
//   - / 2000.0 with / → % or *
//   - * density with * → + or /
// L388-391: edge_buffer..chunk_size - edge_buffer (the - → +)
// L395-396: chunk_origin + x/z (the + → - or *)
// L399-400: x / chunk_size * (res-1) (the * → + or /)
//           (res-1) where - → + or /
// L403: || → && in bounds check
// L412: delete ! in is_suitable_location check
// ═══════════════════════════════════════════════════════════════════════

fn make_chunk(cx: i32, cz: i32) -> astraweave_terrain::TerrainChunk {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(cx, cz))
        .unwrap()
}

/// Structures at origin chunk must have world positions within [0, chunk_size].
/// Tests catch: L395-396 +→-|*, L388-391 -→+, L412 delete !
#[test]
fn structure_positions_within_chunk_bounds_origin() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    // Use high density + multiple seeds to guarantee structures
    let mut found_any = false;
    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }
        for s in &result.structures {
            assert!(
                s.position.x >= 0.0 && s.position.x <= chunk_size,
                "Structure x={} out of chunk bounds [0, {}]",
                s.position.x,
                chunk_size
            );
            assert!(
                s.position.z >= 0.0 && s.position.z <= chunk_size,
                "Structure z={} out of chunk bounds [0, {}]",
                s.position.z,
                chunk_size
            );
        }
    }
    assert!(
        found_any,
        "At least one seed should produce structures at density 5.0"
    );
}

/// For a non-origin chunk, positions must be shifted by chunk_origin.
/// If `chunk_origin.x + x` is mutated to `chunk_origin.x - x` or `* x`,
/// positions will be outside valid bounds.
#[test]
fn structure_positions_nonorigin_chunk_offset() {
    let chunk = make_chunk(3, 5);
    let chunk_size = 256.0;
    let mut found_any = false;
    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }

        let origin_x = 3.0 * chunk_size;
        let origin_z = 5.0 * chunk_size;

        for s in &result.structures {
            assert!(
                s.position.x >= origin_x && s.position.x <= origin_x + chunk_size,
                "Structure x={} out of non-origin chunk bounds [{}, {}]",
                s.position.x,
                origin_x,
                origin_x + chunk_size,
            );
            assert!(
                s.position.z >= origin_z && s.position.z <= origin_z + chunk_size,
                "Structure z={} out of non-origin chunk bounds [{}, {}]",
                s.position.z,
                origin_z,
                origin_z + chunk_size,
            );
        }
    }
    assert!(
        found_any,
        "At least one seed should produce structures for non-origin chunk"
    );
}

/// Negative chunk indices — positions should be correctly negative.
#[test]
fn structure_positions_negative_chunk() {
    let chunk = make_chunk(-2, -3);
    let chunk_size = 256.0;
    let mut found_any = false;
    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }

        let origin_x = -2.0 * chunk_size;
        let origin_z = -3.0 * chunk_size;

        for s in &result.structures {
            assert!(
                s.position.x >= origin_x && s.position.x <= origin_x + chunk_size,
                "Structure x={} out of negative chunk bounds [{}, {}]",
                s.position.x,
                origin_x,
                origin_x + chunk_size,
            );
            assert!(
                s.position.z >= origin_z && s.position.z <= origin_z + chunk_size,
                "Structure z={} out of negative chunk bounds [{}, {}]",
                s.position.z,
                origin_z,
                origin_z + chunk_size,
            );
        }
    }
    assert!(
        found_any,
        "At least one seed should produce structures for negative chunk"
    );
}

/// Extremely high density should produce many structures (area * density math).
/// If `chunk_size * chunk_size` is mutated to `chunk_size + chunk_size`,
/// max_structures = (256+256)/2000*5 = 1.28 → 1 attempt instead of ~163.
#[test]
fn structure_high_density_produces_many() {
    let chunk_size = 256.0;
    // Try multiple seeds and sum — at least some must produce structures
    let mut total = 0usize;
    for seed in 0..10u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        total += result.total_count();
    }

    // With correct formula: max_structures per chunk ≈ 163 attempts
    // Across 10 seeds, should get many structures total
    // With + mutation: max_structures ≈ 1 each → ~10 total max
    assert!(
        total > 15,
        "High density across 10 seeds should produce many structures, got {}",
        total
    );
}

/// Low density = few structures. If / → * in the formula, it would produce
/// (256*256*2000*0.1) ≈ 13,107,200 attempts → many structures.
#[test]
fn structure_low_density_few() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    let config = StructureConfig {
        density: 0.05,
        seed: 42,
        edge_buffer: 20.0,
        ..Default::default()
    };
    let mut gen = StructureGenerator::new(config);
    let result = gen
        .generate_structures(&chunk, chunk_size, BiomeType::Grassland)
        .unwrap();

    // max_structures = (256*256/2000*0.05) ≈ 1.6 → random 0..=1 attempts
    // Should produce 0 or 1 structure
    assert!(
        result.total_count() <= 3,
        "Very low density should produce few structures, got {}",
        result.total_count()
    );
}

/// Structure count should scale with density.
/// Multiple seeds to show the trend statistically.
#[test]
fn structure_count_scales_with_density() {
    let chunk_size = 256.0;

    let mut total_low = 0usize;
    let mut total_high = 0usize;

    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config_low = StructureConfig {
            density: 0.5,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen_low = StructureGenerator::new(config_low);
        let result_low = gen_low
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        total_low += result_low.total_count();

        let config_high = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen_high = StructureGenerator::new(config_high);
        let result_high = gen_high
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();
        total_high += result_high.total_count();
    }

    // With correct formula: high density (5.0) produces ~10x more than low (0.5)
    // With + mutation on area: both produce ≈ same (small numbers)
    assert!(
        total_high > total_low,
        "Higher density should produce more structures: total_low={total_low}, total_high={total_high}"
    );
    // High density MUST produce at least some structures
    assert!(
        total_high > 0,
        "High density across 20 seeds must produce at least some structures"
    );
}

/// Structure count should scale with chunk_size (area).
/// If * → + in area calculation, scaling breaks.
#[test]
fn structure_count_scales_with_chunk_size() {
    // Note: chunk_size must be > 2 * edge_buffer(20) = 40 to avoid panic
    let mut total_small = 0usize;
    let mut total_large = 0usize;

    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 3.0,
            seed,
            edge_buffer: 10.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };

        let mut gen_s = StructureGenerator::new(config.clone());
        let result_s = gen_s
            .generate_structures(&chunk, 64.0, BiomeType::Forest)
            .unwrap();
        total_small += result_s.total_count();

        let mut gen_l = StructureGenerator::new(config);
        let result_l = gen_l
            .generate_structures(&chunk, 512.0, BiomeType::Forest)
            .unwrap();
        total_large += result_l.total_count();
    }

    // Large chunk (512^2=262144 area) should spawn far more than small (64^2=4096)
    // With + mutation: 512+512=1024 vs 64+64=128 → 8x instead of 64x
    // With correct: 262144/4096 = 64x more area
    assert!(
        total_large > total_small,
        "Larger chunks should produce more structures: total_small={total_small}, total_large={total_large}"
    );
    // Large chunks MUST produce structures
    assert!(
        total_large > 0,
        "Large chunks with density 3.0 across 20 seeds must produce structures"
    );
}

/// Structures should have valid rotation (0..TAU)
#[test]
fn structure_rotation_in_range() {
    let mut found_any = false;
    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 5.0,
            seed,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, 256.0, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }
        for s in &result.structures {
            assert!(
                s.rotation >= 0.0 && s.rotation <= std::f32::consts::TAU,
                "Structure rotation {} out of [0, TAU]",
                s.rotation
            );
        }
    }
    assert!(found_any, "At least one seed should produce structures");
}

/// Structures should have valid scale (0.8..1.2)
#[test]
fn structure_scale_in_range() {
    let mut found_any = false;
    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 5.0,
            seed,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, 256.0, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }
        for s in &result.structures {
            assert!(
                s.scale >= 0.8 && s.scale <= 1.2,
                "Structure scale {} out of [0.8, 1.2]",
                s.scale
            );
        }
    }
    assert!(found_any, "At least one seed should produce structures");
}

/// Structure Y position should be a terrain height (not NaN or extreme).
#[test]
fn structure_y_is_terrain_height() {
    let mut found_any = false;
    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 5.0,
            seed,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, 256.0, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }
        for s in &result.structures {
            assert!(!s.position.y.is_nan(), "Structure Y should not be NaN");
            // WorldConfig default max_height is 100.0
            assert!(
                s.position.y >= -200.0 && s.position.y <= 200.0,
                "Structure Y={} seems unreasonable",
                s.position.y
            );
        }
    }
    assert!(found_any, "At least one seed should produce structures");
}

/// Every structure should have a non-empty model_path.
#[test]
fn structure_has_model_path() {
    let mut found_any = false;
    for seed in 0..20u64 {
        let chunk = make_chunk(0, 0);
        let config = StructureConfig {
            density: 5.0,
            seed,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, 256.0, BiomeType::Forest)
            .unwrap();
        if !result.structures.is_empty() {
            found_any = true;
        }
        for s in &result.structures {
            assert!(
                !s.model_path.is_empty(),
                "Structure {:?} should have model_path",
                s.structure_type
            );
        }
    }
    assert!(found_any, "At least one seed should produce structures");
}

/// For multiple chunks, structures should NOT overlap chunk boundaries.
#[test]
fn structures_dont_cross_chunk_boundaries() {
    let chunk_size = 256.0;
    let mut total_structures = 0usize;

    for cx in 0..3 {
        for cz in 0..3 {
            let chunk = make_chunk(cx, cz);
            let config = StructureConfig {
                density: 5.0,
                seed: 42 + (cx * 10 + cz) as u64,
                edge_buffer: 20.0,
                include_ancient: true,
                include_defensive: true,
                ..Default::default()
            };
            let mut gen = StructureGenerator::new(config);
            let result = gen
                .generate_structures(&chunk, chunk_size, BiomeType::Forest)
                .unwrap();
            total_structures += result.structures.len();

            let origin_x = cx as f32 * chunk_size;
            let origin_z = cz as f32 * chunk_size;

            for s in &result.structures {
                assert!(
                    s.position.x >= origin_x && s.position.x <= origin_x + chunk_size,
                    "Chunk({},{}) structure x={} outside [{}, {}]",
                    cx,
                    cz,
                    s.position.x,
                    origin_x,
                    origin_x + chunk_size
                );
                assert!(
                    s.position.z >= origin_z && s.position.z <= origin_z + chunk_size,
                    "Chunk({},{}) structure z={} outside [{}, {}]",
                    cx,
                    cz,
                    s.position.z,
                    origin_z,
                    origin_z + chunk_size
                );
            }
        }
    }
    assert!(
        total_structures > 0,
        "9 chunks at density 5.0 should produce at least some structures"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Height verification — catches L399-400 resolution scaling mutations
//
// If `x / chunk_size * (resolution - 1)` has * → + or / mutations,
// the heightmap index is wrong → structure Y ≠ terrain height at its X/Z.
// We re-compute the CORRECT index in the test and compare.
// ═══════════════════════════════════════════════════════════════════════

/// Structure Y must match the interpolated terrain height at its world X/Z.
/// If the resolution index formula is mutated, the lookup will be at a
/// wrong heightmap cell → wrong Y.
#[test]
fn structure_height_matches_terrain_at_position() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    let resolution = chunk.heightmap().resolution();

    let mut found_any = false;
    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();

        for s in &result.structures {
            found_any = true;
            // For chunk (0,0), world_x == local x offset
            let x = s.position.x;
            let z = s.position.z;

            // Recompute the CORRECT heightmap index
            let local_x = (x / chunk_size * (resolution - 1) as f32) as u32;
            let local_z = (z / chunk_size * (resolution - 1) as f32) as u32;

            if local_x < resolution && local_z < resolution {
                let expected_height = chunk.heightmap().get_height(local_x, local_z);
                assert!(
                    (s.position.y - expected_height).abs() < 0.01,
                    "Structure at ({}, {}) has height {} but terrain has {} at ({}, {})",
                    s.position.x,
                    s.position.z,
                    s.position.y,
                    expected_height,
                    local_x,
                    local_z
                );
            }
        }
    }
    assert!(found_any, "Need structures to validate height correctness");
}

/// With many structures, heights should NOT all be identical.
/// Catches * → / in resolution formula (always index 0 → same height).
#[test]
fn structure_heights_are_varied() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    let mut heights = Vec::new();

    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();

        for s in &result.structures {
            heights.push(s.position.y);
        }
    }

    assert!(
        heights.len() > 3,
        "Need multiple structures to check variation, got {}",
        heights.len()
    );

    // Count unique heights (round to nearest 0.1 to avoid float noise)
    let mut unique: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for h in &heights {
        unique.insert((*h * 10.0) as i32);
    }
    assert!(
        unique.len() > 1,
        "All {} structures have the same height — resolution index formula probably wrong",
        heights.len()
    );
}

/// Non-origin chunk: height still matches terrain at correct world position.
/// Catches + → - or * mutations in chunk_origin offset that would cause
/// the height lookup to be at a completely wrong position.
#[test]
fn structure_height_matches_nonorigin_chunk() {
    let chunk = make_chunk(2, 3);
    let chunk_size = 256.0;
    let resolution = chunk.heightmap().resolution();
    let origin_x = 2.0 * chunk_size;
    let origin_z = 3.0 * chunk_size;

    let mut verified = 0;
    for seed in 0..20u64 {
        let config = StructureConfig {
            density: 5.0,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, chunk_size, BiomeType::Forest)
            .unwrap();

        for s in &result.structures {
            // Local offset within this chunk
            let local_offset_x = s.position.x - origin_x;
            let local_offset_z = s.position.z - origin_z;

            // Recompute correct heightmap index
            let hm_x = (local_offset_x / chunk_size * (resolution - 1) as f32) as u32;
            let hm_z = (local_offset_z / chunk_size * (resolution - 1) as f32) as u32;

            if hm_x < resolution && hm_z < resolution {
                let expected_height = chunk.heightmap().get_height(hm_x, hm_z);
                assert!(
                    (s.position.y - expected_height).abs() < 0.01,
                    "Non-origin structure at ({}, {}) has y={} but expected {} at hm({}, {})",
                    s.position.x,
                    s.position.z,
                    s.position.y,
                    expected_height,
                    hm_x,
                    hm_z
                );
                verified += 1;
            }
        }
    }
    assert!(
        verified > 0,
        "Need at least one verified structure for non-origin chunk"
    );
}
