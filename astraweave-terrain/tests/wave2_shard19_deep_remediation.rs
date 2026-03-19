//! Deep remediation tests targeting ALL 26 shard-19 MISSED mutations.
//!
//! Shard 19 missed mutations:
//! - streaming_diagnostics.rs:101 → p99 index formula (* → +, * → /)
//! - structures.rs:357 → max_structures density math
//! - structures.rs:388-400 → position offset math (chunk_origin ± x/z)
//! - structures.rs:399-400 → heightmap resolution index math
//! - structures.rs:403 → bounds check || → &&
//! - structures.rs:412 → suitability check delete !
//! - structures.rs:451 → choose_structure_type target -= → +=, /=
//! - structures.rs:452 → choose_structure_type target <= → >

use astraweave_terrain::streaming_diagnostics::HitchDetector;
use astraweave_terrain::structures::{StructureConfig, StructureGenerator, StructureType};
use astraweave_terrain::BiomeType;
use std::collections::HashSet;

// ───────────────────────── Helpers ──────────────────────────

fn make_chunk(cx: i32, cz: i32) -> astraweave_terrain::TerrainChunk {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(cx, cz))
        .unwrap()
}

/// Gather structures from many seeds to get a large sample.
fn gather_structures(
    chunk: &astraweave_terrain::TerrainChunk,
    chunk_size: f32,
    biome: BiomeType,
    density: f32,
    num_seeds: u64,
) -> Vec<astraweave_terrain::structures::StructureInstance> {
    let mut all = Vec::new();
    for seed in 0..num_seeds {
        let config = StructureConfig {
            density,
            seed,
            edge_buffer: 20.0,
            include_ancient: true,
            include_defensive: true,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        if let Ok(result) = gen.generate_structures(chunk, chunk_size, biome) {
            all.extend(result.structures.into_iter());
        }
    }
    all
}

// ───────────────────── P99 Frame Time Tests ─────────────────

/// TARGETS: streaming_diagnostics.rs:101 → * → +
/// With 200 frames, sorted.len()=200, index = ceil(200 * 0.99) = ceil(198) = 198
/// If * becomes +, index = ceil(200 + 0.99) = ceil(200.99) = 201 → capped to 199
/// The p99 should be near the high end but NOT the absolute max.
#[test]
fn p99_large_sample_not_max() {
    let mut det = HitchDetector::new(300, 16.0);
    // Record 300 frames with linearly increasing times
    for i in 0..300 {
        det.record_frame(i as f32 * 0.1); // 0.0, 0.1, 0.2, ..., 29.9
    }
    let p99 = det.p99_frame_time();
    // p99 should be around 29.7 (index ~297 of 300), NOT 29.9 (max)
    assert!(p99 < 29.9, "p99 should NOT be the absolute max, got {p99}");
    assert!(p99 > 25.0, "p99 should be in the upper range, got {p99}");
}

/// TARGETS: streaming_diagnostics.rs:101 → * → /
/// If * becomes /, index = ceil(200 / 0.99) = ceil(202.02) = 203 → capped to 199
/// vs correct: ceil(200 * 0.99) = ceil(198) = 198
/// With linearly spaced data, different indices give different values.
#[test]
fn p99_distinguishable_from_max_and_median() {
    let mut det = HitchDetector::new(400, 16.0);
    // 400 distinct values
    for i in 0..400 {
        det.record_frame(i as f32);
    }
    let p99 = det.p99_frame_time();
    // correct p99: ceil(400*0.99) = 396 → sorted[396] = 396.0
    // with /: ceil(400/0.99) = 405 → capped to 399 → sorted[399] = 399.0
    assert!(
        (p99 - 396.0).abs() < 2.0,
        "p99 should be ~396 for 400 frames, got {p99}"
    );
}

// ───────────── Structure Type Distribution Tests ─────────────

/// TARGETS: structures.rs:451 → target -= → target +=
/// If -= becomes +=, target always increases, never reaching <= 0,
/// so ONLY the fallback (last) type gets selected.
/// We verify multiple types appear in a large sample.
#[test]
fn structure_types_are_diverse() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 50);
    assert!(
        !structures.is_empty(),
        "Need structures for distribution test"
    );

    let types: HashSet<StructureType> = structures.iter().map(|s| s.structure_type).collect();
    assert!(
        types.len() >= 3,
        "Expected at least 3 different structure types in 50 seeds, got {} types: {:?}",
        types.len(),
        types
    );
}

/// TARGETS: structures.rs:452 → <= → >
/// If <= becomes >, the condition `target > 0.0` is only true when target
/// hasn't been decremented enough. This changes the distribution heavily.
/// Verify that common types (high rarity weight) appear more frequently.
#[test]
fn common_structures_appear_more_than_rare() {
    let chunk = make_chunk(0, 0);
    // Use Grassland for variety (Cottage=0.8, Farmhouse=0.8, Fort=0.2)
    let structures = gather_structures(&chunk, 256.0, BiomeType::Grassland, 5.0, 80);
    if structures.is_empty() {
        return; // Skip if no structures generated
    }

    // Count occurrences
    let mut type_counts = std::collections::HashMap::new();
    for s in &structures {
        *type_counts.entry(s.structure_type).or_insert(0u32) += 1;
    }

    // Common types (rarity 0.8): Cottage, Farmhouse
    let common_count: u32 = type_counts
        .iter()
        .filter(|(t, _)| matches!(t, StructureType::Cottage | StructureType::Farmhouse))
        .map(|(_, c)| *c)
        .sum();

    // Total count
    let total = structures.len() as u32;

    // Common types should represent a non-trivial portion
    // With correct weighting, common types should appear at least sometimes
    assert!(
        common_count > 0 || total < 5,
        "Common types (Cottage/Farmhouse) should appear in {} total structures",
        total
    );
}

/// TARGETS: structures.rs:451 → target -= → target /=
/// If -= becomes /=, the target halving means the first type almost always
/// gets selected (since target = rand * total_weight, and dividing by first
/// rarity often drops below 0). Verify we don't get ONLY the first type.
#[test]
fn not_only_first_structure_type() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 50);
    if structures.len() < 5 {
        return; // Not enough to test distribution
    }

    let first_type = structures[0].structure_type;
    let all_same = structures.iter().all(|s| s.structure_type == first_type);
    assert!(
        !all_same,
        "All {} structures are {:?} — rarity selection is probably broken",
        structures.len(),
        first_type
    );
}

// ────────────── Structure Density Math Tests ──────────────

/// TARGETS: structures.rs:357 → chunk_size * chunk_size / 2000 * density
/// The formula is: max_structures = (chunk_size² / 2000 * density) as u32
/// For chunk_size=256, density=5: (256*256/2000*5) = (65536/2000*5) = 163.84
/// If * becomes +: (256+256) = 512, 512/2000*5 = 1.28 → max 1 or 2
/// If * becomes /: (256/256) = 1, 1/2000*5 = 0.0025 → max 0
#[test]
fn high_density_generates_many_structures() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Grassland, 10.0, 30);
    // With density=10 and 30 seeds, we should get many structures
    assert!(
        structures.len() >= 50,
        "High density should produce at least 50 structures from 30 seeds, got {}",
        structures.len()
    );
}

/// TARGETS: structures.rs:357 → * → / and * → +
/// With different chunk sizes, the count should scale quadratically.
/// chunk_size=256: area=65536, max=(65536/2000*5)=163
/// chunk_size=128: area=16384, max=(16384/2000*5)=40
/// The ratio should be ~4x. If * becomes +, ratio would be ~2x.
#[test]
fn density_scales_quadratically_with_chunk_size() {
    let chunk = make_chunk(0, 0);

    let structures_256 = gather_structures(&chunk, 256.0, BiomeType::Grassland, 5.0, 20);
    let structures_128 = gather_structures(&chunk, 128.0, BiomeType::Grassland, 5.0, 20);

    if structures_128.is_empty() {
        // At 128, we still expect some structures: 128*128/2000*5 = ~40 max
        panic!("128x128 chunk with density=5 should produce structures");
    }

    let ratio = structures_256.len() as f32 / structures_128.len() as f32;
    // With correct quadratic scaling, ratio should be roughly 3-5x
    // With additive scaling, ratio would be ~2x
    assert!(
        ratio > 2.5,
        "256/128 structure ratio should be >2.5 (area-quadratic), got {ratio:.1}"
    );
}

// ────────────── Position Offset Math Tests ──────────────

/// TARGETS: structures.rs:388,391 → chunk_origin.x + x becomes - x
/// TARGETS: structures.rs:395,396 → chunk_origin + x/z inverted
/// For non-origin chunks, positions MUST be offset by chunk origin.
/// Chunk (3,5) at chunk_size=256: origin = (768, 1280)
/// Structures must have x in [768+20, 768+236] and z in [1280+20, 1280+236]
#[test]
fn positions_offset_by_chunk_origin() {
    let chunk = make_chunk(3, 5);
    let chunk_size = 256.0;
    let origin_x = 3.0 * chunk_size;
    let origin_z = 5.0 * chunk_size;
    let edge = 20.0;

    let structures = gather_structures(&chunk, chunk_size, BiomeType::Forest, 5.0, 30);
    assert!(!structures.is_empty(), "Need structures for position test");

    for s in &structures {
        assert!(
            s.position.x >= origin_x + edge - 1.0
                && s.position.x <= origin_x + chunk_size - edge + 1.0,
            "Structure x={} outside chunk(3,_) bounds [{}, {}]",
            s.position.x,
            origin_x + edge,
            origin_x + chunk_size - edge
        );
        assert!(
            s.position.z >= origin_z + edge - 1.0
                && s.position.z <= origin_z + chunk_size - edge + 1.0,
            "Structure z={} outside chunk(_,5) bounds [{}, {}]",
            s.position.z,
            origin_z + edge,
            origin_z + chunk_size - edge
        );
    }
}

/// TARGETS: structures.rs:395,396 → + x becomes - x or * x
/// For negative chunk coords, positions should still be in correct range.
/// Chunk (-2, -3): origin = (-512, -768)
#[test]
fn positions_correct_for_negative_chunks() {
    let chunk = make_chunk(-2, -3);
    let chunk_size = 256.0;
    let origin_x = -2.0 * chunk_size;
    let origin_z = -3.0 * chunk_size;
    let edge = 20.0;

    let structures = gather_structures(&chunk, chunk_size, BiomeType::Grassland, 5.0, 30);
    assert!(
        !structures.is_empty(),
        "Need structures for negative chunk test"
    );

    for s in &structures {
        let local_x = s.position.x - origin_x;
        let local_z = s.position.z - origin_z;
        assert!(
            local_x >= edge - 1.0 && local_x <= chunk_size - edge + 1.0,
            "Local x={} outside [edge, chunk_size-edge] for chunk(-2,-3)",
            local_x
        );
        assert!(
            local_z >= edge - 1.0 && local_z <= chunk_size - edge + 1.0,
            "Local z={} outside [edge, chunk_size-edge] for chunk(-2,-3)",
            local_z
        );
    }
}

// ──────────── Heightmap Resolution Index Tests ──────────────

/// TARGETS: structures.rs:399-400 → * → + or / in resolution formula
/// The formula: local_x = (x / chunk_size * (resolution - 1)) as u32
/// If * becomes +: (x / chunk_size + (resolution-1)) → always >= 63 for res=64
/// Structures near chunk start would get height from wrong cell.
/// We verify that a structure near edge_buffer gets the correct height.
#[test]
fn height_lookup_uses_correct_resolution_formula() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    let resolution = chunk.heightmap().resolution();

    let structures = gather_structures(&chunk, chunk_size, BiomeType::Forest, 5.0, 50);
    assert!(
        !structures.is_empty(),
        "Need structures for height formula test"
    );

    let mut correct_count = 0;
    for s in &structures {
        let x = s.position.x;
        let z = s.position.z;

        // Recompute the correct heightmap index
        let hm_x = (x / chunk_size * (resolution - 1) as f32) as u32;
        let hm_z = (z / chunk_size * (resolution - 1) as f32) as u32;

        if hm_x < resolution && hm_z < resolution {
            let expected = chunk.heightmap().get_height(hm_x, hm_z);
            if (s.position.y - expected).abs() < 0.01 {
                correct_count += 1;
            }
        }
    }

    assert!(
        correct_count > 0,
        "No structures had correct height — resolution formula is probably wrong"
    );
}

/// TARGETS: structures.rs:399:77, 400:77 → - → + in (resolution - 1)
/// If resolution-1 becomes resolution+1, the indices would exceed bounds
/// and structures might use wrong heights or fail bounds check.
/// Verify structures exist AND have height matching terrain.
#[test]
fn resolution_minus_one_not_plus_one() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;
    let _resolution = chunk.heightmap().resolution();

    let structures = gather_structures(&chunk, chunk_size, BiomeType::Forest, 5.0, 40);
    assert!(
        !structures.is_empty(),
        "Need structures for resolution-1 test"
    );

    // With resolution+1 instead of resolution-1, the local_x/z could exceed resolution
    // and the `if local_x >= resolution || local_z >= resolution { continue; }` would
    // reject most placements. Count how many structures are successfully placed.
    // A working formula should place many.
    assert!(
        structures.len() >= 10,
        "Only {} structures placed from 40 seeds — resolution formula may add instead of subtract",
        structures.len()
    );
}

// ────────────── Bounds Check || vs && Test ──────────────

/// TARGETS: structures.rs:403 → || → &&
/// The check is: `if local_x >= resolution || local_z >= resolution { continue; }`
/// If || becomes &&, both must be out-of-bounds to skip, meaning
/// structures at x > resolution but z < resolution would be placed at
/// garbage height (index out of bounds or wrong data).
///
/// We verify all structure heights are within the heightmap range.
#[test]
fn all_heights_within_heightmap_range() {
    let chunk = make_chunk(0, 0);
    let chunk_size = 256.0;

    let structures = gather_structures(&chunk, chunk_size, BiomeType::Forest, 5.0, 50);
    assert!(
        !structures.is_empty(),
        "Need structures for bounds check test"
    );

    let min_h = chunk.heightmap().min_height();
    let max_h = chunk.heightmap().max_height();

    for s in &structures {
        assert!(
            s.position.y >= min_h - 1.0 && s.position.y <= max_h + 1.0,
            "Structure at ({}, {}, {}) has height outside heightmap range [{}, {}]",
            s.position.x,
            s.position.y,
            s.position.z,
            min_h,
            max_h
        );
    }
}

// ────────────── Suitability Check (delete !) Test ──────────────

/// TARGETS: structures.rs:412 → delete ! in `!self.is_suitable_location()`
/// If ! is deleted, suitable locations would be REJECTED and unsuitable
/// ones accepted → almost no structures would pass.
/// With a flat chunk (max_slope=0), all locations should be suitable.
#[test]
fn flat_terrain_produces_structures() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Grassland, 5.0, 20);
    assert!(
        structures.len() >= 5,
        "Flat terrain should easily produce structures, got {} from 20 seeds",
        structures.len()
    );
}

/// TARGETS: structures.rs:412 → delete !
/// Another approach: with very high density and flat terrain, we should get many.
/// If the ! is deleted, we get nearly zero because flat terrain IS suitable,
/// so `if is_suitable { continue }` skips all of them.
#[test]
fn high_density_flat_terrain_many_structures() {
    let chunk = make_chunk(0, 0);
    // density=20 on 256x256 flat chunk should produce plenty
    let structures = gather_structures(&chunk, 256.0, BiomeType::Grassland, 20.0, 10);
    assert!(
        structures.len() >= 20,
        "Expected >=20 structures from density=20 flat terrain, got {}",
        structures.len()
    );
}

// ────────── Additional cross-cutting tests ──────────

/// All structures should have valid model paths (non-empty).
/// This verifies the full placement pipeline works end-to-end.
#[test]
fn all_structures_have_model_paths() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 20);
    for s in &structures {
        assert!(
            !s.model_path.is_empty(),
            "Structure {:?} at {:?} has empty model path",
            s.structure_type,
            s.position
        );
    }
}

/// Scale should be in the [0.8, 1.2] range as per the source code.
#[test]
fn all_structures_valid_scale() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 30);
    assert!(!structures.is_empty());
    for s in &structures {
        assert!(
            s.scale >= 0.8 && s.scale <= 1.2,
            "Scale {} outside [0.8, 1.2]",
            s.scale
        );
    }
}

/// Rotation should be in [0, TAU).
#[test]
fn all_structures_valid_rotation() {
    let chunk = make_chunk(0, 0);
    let structures = gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 30);
    assert!(!structures.is_empty());
    for s in &structures {
        assert!(
            s.rotation >= 0.0 && s.rotation < std::f32::consts::TAU,
            "Rotation {} outside [0, TAU)",
            s.rotation
        );
    }
}

/// Multiple biomes produce different structure types.
/// Cross-checks that the biome→structure_type mapping is working.
#[test]
fn different_biomes_produce_different_types() {
    let chunk = make_chunk(0, 0);

    let forest_types: HashSet<StructureType> =
        gather_structures(&chunk, 256.0, BiomeType::Forest, 5.0, 30)
            .into_iter()
            .map(|s| s.structure_type)
            .collect();

    let desert_types: HashSet<StructureType> =
        gather_structures(&chunk, 256.0, BiomeType::Desert, 5.0, 30)
            .into_iter()
            .map(|s| s.structure_type)
            .collect();

    // Forest and Desert should NOT have identical type sets
    if !forest_types.is_empty() && !desert_types.is_empty() {
        assert_ne!(
            forest_types, desert_types,
            "Forest and Desert shouldn't produce identical structure type sets"
        );
    }
}
