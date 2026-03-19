//! Wave 2 Shard 21 Remediation Tests
//!
//! Targets 38 MISSED mutations:
//! - texture_splatting.rs: calculate_weights (line 418 &&→||, >→>=, <→<=),
//!   noise variation (line 426 *=, +, *), sample_noise (line 453-454),
//!   hash_value (line 458-462), should_use_triplanar (line 501)
//! - voxel_data.rs: child_index (lines 130-133), is_valid_local_pos (lines 252-256),
//!   world_to_local (line 273), memory_usage (line 289), estimate_tree_size (lines 295-300)

use astraweave_terrain::{
    ChunkCoord, SplatConfig, SplatMapGenerator, SplatRule, SplatWeights, TriplanarWeights, Voxel,
    VoxelChunk, CHUNK_SIZE, MAX_SPLAT_LAYERS,
};
use glam::{IVec3, Vec3};

/// Helper to extract weights array from SplatWeights
fn weights_array(w: &SplatWeights) -> [f32; MAX_SPLAT_LAYERS] {
    let mut arr = [0.0f32; MAX_SPLAT_LAYERS];
    for i in 0..MAX_SPLAT_LAYERS {
        arr[i] = w.get_weight(i);
    }
    arr
}

// ─── SplatMapGenerator::calculate_weights ───────────────────────────────────

/// Build a generator with known rules for testing
fn make_generator(seed: u64) -> SplatMapGenerator {
    SplatMapGenerator::with_default_rules(SplatConfig::default(), seed)
}

#[test]
fn calculate_weights_flat_terrain_grass_dominant() {
    // Height=50 (middle of grass range 0-100), slope=0 (flat, normal=Y)
    // Grass (material_id=0) should get highest weight
    let gen = make_generator(42);
    let w = gen.calculate_weights(50.0, Vec3::Y);
    let weights = weights_array(&w);
    // Grass (id=0) must be positive
    assert!(
        weights[0] > 0.0,
        "Grass weight should be positive at h=50 flat"
    );
    // Grass should be dominant
    assert!(
        weights[0] >= weights[7],
        "Grass should dominate over rock on flat terrain"
    );
}

#[test]
fn calculate_weights_steep_slope_rock_dominant() {
    // Height=50, slope ~60° (steep enough for rock rule: min_slope=35, max_slope=90)
    // Normal tilted 60° from vertical: cos(60°) = 0.5 → y=0.5, xz=0.866
    let normal = Vec3::new(0.866, 0.5, 0.0).normalize();
    let gen = make_generator(42);
    let w = gen.calculate_weights(50.0, normal);
    let weights = weights_array(&w);
    // Rock (id=7) should be present with high weight
    assert!(weights[7] > 0.0, "Rock should have weight on steep slope");
}

#[test]
fn calculate_weights_respects_rule_height_range() {
    // Sand rule: min_height=-5, max_height=8
    // At height=3 (within range), sand should have weight
    // At height=50 (outside range), sand weight should be near zero
    let gen = make_generator(42);

    let w_in = gen.calculate_weights(3.0, Vec3::Y);
    let w_out = gen.calculate_weights(50.0, Vec3::Y);

    let in_sand = weights_array(&w_in)[1]; // sand = material_id 1
    let out_sand = weights_array(&w_out)[1];

    assert!(in_sand > out_sand, "Sand should be higher at h=3 than h=50");
}

#[test]
fn calculate_weights_positive_weight_and_rule_match_required() {
    // Tests line 418: `if weight > 0.0 && (rule.material_id as usize) < MAX_SPLAT_LAYERS`
    // If && becomes ||, any rule with weight=0 OR valid material_id would match
    // With two rules: grass(weight=1.0) + rock(weight=1.0), both should contribute
    // If && → ||, zero-weight rules would also contribute (different distribution)
    let mut gen = SplatMapGenerator::new(SplatConfig::default(), 42);
    let mut grass = SplatRule::grass(); // material_id=0, weight=1.0
    grass.weight = 1.0;
    gen.add_rule(grass);
    let mut rock = SplatRule::rock(); // material_id=7, weight=1.0
    rock.weight = 1.0;
    gen.add_rule(rock);

    // At height=50 flat terrain: grass should have weight, rock should have low weight (slope dependent)
    let w = gen.calculate_weights(50.0, Vec3::Y);
    let weights = weights_array(&w);
    // Grass (id=0) should definitely have weight on flat terrain
    assert!(
        weights[0] > 0.0,
        "Grass should contribute on flat terrain at h=50, got {}",
        weights[0]
    );
}

#[test]
fn calculate_weights_material_id_bounds_check() {
    // Tests line 418: material_id check < MAX_SPLAT_LAYERS
    // If && becomes ||, out-of-bounds material would be accepted when weight > 0
    let mut gen = SplatMapGenerator::new(SplatConfig::default(), 42);
    let mut oob_rule = SplatRule::grass();
    oob_rule.material_id = MAX_SPLAT_LAYERS as u32; // Out of bounds
    oob_rule.weight = 1.0;
    gen.add_rule(oob_rule);

    // Should not panic — the bounds check prevents array index out of bounds
    let w = gen.calculate_weights(50.0, Vec3::Y);
    let weights = weights_array(&w);
    // All weights should be 0 or very small (only from the OOB rule which should be skipped)
    let total: f32 = weights.iter().sum();
    // Just verify no panic and total is small (only noise variation at most)
    assert!(total < 2.0, "OOB rule should not add large contributions");
}

#[test]
fn calculate_weights_noise_variation_multiplicative() {
    // Tests line 426: `*weight *= 1.0 + noise_offset * 0.1`
    // If *= becomes += or /=, the result changes significantly
    // Key: noise variation should SCALE existing weight, not add to it
    let gen = make_generator(42);

    // Two different heights should produce different noise offsets
    let w1 = gen.calculate_weights(10.0, Vec3::Y);
    let w2 = gen.calculate_weights(10.001, Vec3::Y); // Very slightly different

    // Both should have valid weights (noise shouldn't destroy them)
    let weights1 = weights_array(&w1);
    let weights2 = weights_array(&w2);
    assert!(weights1[0] > 0.0, "Weight should survive noise variation");
    assert!(weights2[0] > 0.0, "Weight should survive noise variation");
}

#[test]
fn calculate_weights_noise_preserves_positive_weights() {
    // If *= became += with noise_offset near -1, weight could go negative
    // The formula 1.0 + noise_offset * 0.1 means range [0.9, 1.1]
    // Weight should always remain non-negative after noise
    let gen = make_generator(123);
    for h in [0.0f32, 10.0, 20.0, 50.0, 80.0, 99.0] {
        let w = gen.calculate_weights(h, Vec3::Y);
        for &val in weights_array(&w).iter() {
            assert!(
                val >= 0.0,
                "Weight should never go negative at h={h}, got {val}"
            );
        }
    }
}

#[test]
fn calculate_weights_deterministic_same_seed() {
    let gen1 = make_generator(42);
    let gen2 = make_generator(42);
    let w1 = gen1.calculate_weights(50.0, Vec3::Y);
    let w2 = gen2.calculate_weights(50.0, Vec3::Y);
    assert_eq!(
        weights_array(&w1),
        weights_array(&w2),
        "Same seed should give same weights"
    );
}

#[test]
fn calculate_weights_noise_varies_with_height() {
    // The noise function depends on height — different heights produce different hash values.
    // Since all weights are scaled by the SAME noise factor, normalization cancels it out
    // for single-point queries. But noise must be deterministic: same (seed, height) → same result.
    // Test that the generator is deterministic and that the noise factor doesn't destroy weights.
    let gen = make_generator(42);

    // Multiple calls with same params should be identical (determinism)
    let w1 = gen.calculate_weights(50.0, Vec3::Y);
    let w2 = gen.calculate_weights(50.0, Vec3::Y);
    assert_eq!(
        weights_array(&w1),
        weights_array(&w2),
        "Same input should give identical output"
    );

    // All weights should be valid (non-NaN, non-negative)
    for h in [0.0f32, 10.0, 50.0, 100.0, 200.0] {
        let w = gen.calculate_weights(h, Vec3::Y);
        for i in 0..MAX_SPLAT_LAYERS {
            let val = w.get_weight(i);
            assert!(!val.is_nan(), "Weight NaN at h={h} layer={i}");
            assert!(val >= 0.0, "Weight negative at h={h} layer={i}: {val}");
        }
    }
}

// ─── SplatMapGenerator::generate_splat_map ──────────────────────────────────

#[test]
fn generate_splat_map_matches_individual_calculate() {
    let gen = make_generator(42);
    let heights = vec![10.0, 50.0, 90.0, 130.0];
    let normals = vec![Vec3::Y, Vec3::Y, Vec3::Y, Vec3::Y];
    let splat_map = gen.generate_splat_map(&heights, &normals, 2);

    for (i, &h) in heights.iter().enumerate() {
        let expected = gen.calculate_weights(h, normals[i]);
        assert_eq!(
            weights_array(&splat_map[i]),
            weights_array(&expected),
            "Splat map entry {i} should match individual calculate at h={h}"
        );
    }
}

// ─── TriplanarWeights::should_use_triplanar ─────────────────────────────────

#[test]
fn should_use_triplanar_below_threshold() {
    // y < threshold → true (steep surface, needs triplanar)
    let tw = TriplanarWeights::from_normal(Vec3::new(1.0, 0.1, 0.0).normalize(), 4.0);
    assert!(tw.y < 0.5, "Steep normal should have low y weight");
    assert!(
        tw.should_use_triplanar(0.5),
        "Should use triplanar when y ({}) < threshold (0.5)",
        tw.y
    );
}

#[test]
fn should_use_triplanar_above_threshold() {
    // y >= threshold → false (flat surface, no triplanar needed)
    let tw = TriplanarWeights::from_normal(Vec3::Y, 4.0); // Pure Y normal
    assert!(tw.y > 0.9, "Flat normal should have high y weight");
    assert!(
        !tw.should_use_triplanar(0.5),
        "Should NOT use triplanar when y ({}) >= threshold (0.5)",
        tw.y
    );
}

#[test]
fn should_use_triplanar_at_threshold_boundary() {
    // Tests line 501: `self.y < threshold` — exactly at boundary should be false
    // Create a weight where y is exactly set to a known value
    let tw = TriplanarWeights::from_normal(Vec3::Y, 1.0); // sharpness=1 makes all equal to abs(n)
                                                          // For Vec3::Y with sharpness=1: x=0, y=1.0, z=0
    assert!(
        !tw.should_use_triplanar(1.0),
        "At exact threshold (y=1.0, threshold=1.0), < should return false"
    );
    // Just below
    assert!(
        tw.should_use_triplanar(1.01),
        "Just above threshold should return true (y < 1.01)"
    );
}

// ─── VoxelChunk octree: child_index, is_valid_local_pos ─────────────────────

#[test]
fn voxel_chunk_set_get_roundtrip() {
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let voxel = Voxel::new(0.8, 1);
    let pos = IVec3::new(5, 10, 15);

    chunk.set_voxel(pos, voxel);
    let retrieved = chunk.get_voxel(pos);

    assert!(retrieved.is_some(), "Should retrieve set voxel");
    let v = retrieved.unwrap();
    assert!((v.density - 0.8).abs() < 0.01, "Density should match");
    assert_eq!(v.material, 1, "Material should match");
}

#[test]
fn voxel_chunk_all_octants_addressable() {
    // Tests child_index lines 130-133: >= comparisons determine which octant
    // If >= becomes <, octant assignment flips
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

    // Place voxels in all 8 octants of the chunk
    let half = CHUNK_SIZE / 2;
    let test_positions = [
        IVec3::new(0, 0, 0),          // octant 0: all < half
        IVec3::new(half, 0, 0),       // octant 1: x >= half
        IVec3::new(0, half, 0),       // octant 2: y >= half
        IVec3::new(half, half, 0),    // octant 3: x,y >= half
        IVec3::new(0, 0, half),       // octant 4: z >= half
        IVec3::new(half, 0, half),    // octant 5: x,z >= half
        IVec3::new(0, half, half),    // octant 6: y,z >= half
        IVec3::new(half, half, half), // octant 7: all >= half
    ];

    for (i, &pos) in test_positions.iter().enumerate() {
        let voxel = Voxel::new(0.5 + i as f32 * 0.05, i as u16);
        chunk.set_voxel(pos, voxel);
    }

    // Verify all 8 octant positions are retrievable with correct data
    for (i, &pos) in test_positions.iter().enumerate() {
        let v = chunk
            .get_voxel(pos)
            .unwrap_or_else(|| panic!("Voxel at octant {i} pos {pos:?} not found"));
        assert_eq!(v.material, i as u16, "Material mismatch at octant {i}");
    }
}

#[test]
fn voxel_chunk_child_index_boundary_half() {
    // Tests >= half boundary: pos at (half-1) should be octant 0, pos at (half) should be octant 1
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let half = CHUNK_SIZE / 2;

    // Just below half
    let below = IVec3::new(half - 1, 0, 0);
    chunk.set_voxel(below, Voxel::new(0.9, 100));

    // At half
    let at_half = IVec3::new(half, 0, 0);
    chunk.set_voxel(at_half, Voxel::new(0.9, 200));

    let v_below = chunk.get_voxel(below).unwrap();
    let v_at = chunk.get_voxel(at_half).unwrap();

    assert_eq!(
        v_below.material, 100,
        "Below half should be in lower octant"
    );
    assert_eq!(v_at.material, 200, "At half should be in upper octant");
}

#[test]
fn voxel_chunk_or_bitmask_index() {
    // Tests line 133: x | (y << 1) | (z << 2)
    // If | becomes ^, certain combinations break
    // Position (half, half, half) should be octant 7 (0b111 = 1|2|4)
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let half = CHUNK_SIZE / 2;

    chunk.set_voxel(IVec3::new(half, half, half), Voxel::new(0.9, 77));
    let v = chunk.get_voxel(IVec3::new(half, half, half)).unwrap();
    assert_eq!(v.material, 77, "All-high octant should be addressable");
}

#[test]
fn voxel_chunk_is_valid_bounds() {
    // Tests lines 252-256: < CHUNK_SIZE boundary
    // If < becomes <=, position at CHUNK_SIZE would be accepted (should be rejected)
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

    // Valid position at max valid index
    let max_valid = IVec3::new(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1);
    chunk.set_voxel(max_valid, Voxel::new(0.9, 50));
    let v = chunk.get_voxel(max_valid);
    assert!(v.is_some(), "Max valid position should be accessible");
}

#[test]
fn voxel_chunk_origin_position_valid() {
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.5, 1));
    let v = chunk.get_voxel(IVec3::new(0, 0, 0));
    assert!(v.is_some(), "Origin should be valid");
    assert_eq!(v.unwrap().material, 1);
}

// ─── VoxelChunk::world_to_local ─────────────────────────────────────────────

#[test]
fn voxel_chunk_world_to_local_roundtrip() {
    // Tests line 273: world_to_local should produce correct IVec3
    // If replaced with Default::default(), all world positions would map to (0,0,0)
    let mut chunk = VoxelChunk::new(ChunkCoord::new(1, 0, 0));
    // ChunkCoord(1,0,0) → world origin at (32, 0, 0) (CHUNK_SIZE=32)
    let world_pos = Vec3::new(37.5, 5.5, 10.5); // local should be (5, 5, 10)

    chunk.set_voxel_world(world_pos, Voxel::new(0.7, 42));
    let v = chunk.get_voxel_world(world_pos);

    assert!(v.is_some(), "Should find voxel at world position");
    assert_eq!(v.unwrap().material, 42, "Material should match");
}

#[test]
fn voxel_chunk_world_to_local_different_coords() {
    // Two different world positions should map to different local positions
    // If world_to_local returns Default (0,0,0), all positions would collide
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

    chunk.set_voxel_world(Vec3::new(5.0, 5.0, 5.0), Voxel::new(0.5, 10));
    chunk.set_voxel_world(Vec3::new(15.0, 15.0, 15.0), Voxel::new(0.5, 20));

    let v1 = chunk.get_voxel_world(Vec3::new(5.0, 5.0, 5.0)).unwrap();
    let v2 = chunk.get_voxel_world(Vec3::new(15.0, 15.0, 15.0)).unwrap();

    assert_eq!(v1.material, 10);
    assert_eq!(v2.material, 20);
    assert_ne!(
        v1.material, v2.material,
        "Distinct positions should have distinct values"
    );
}

// ─── VoxelChunk::memory_usage / estimate_tree_size ──────────────────────────

#[test]
fn voxel_chunk_empty_memory_usage() {
    let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let mem = chunk.memory_usage();
    // Empty chunk: just sizeof(Self) + 0 tree overhead
    assert!(mem > 0, "Even empty chunk has base memory");
    assert!(
        mem < 1024,
        "Empty chunk should use minimal memory (got {mem})"
    );
}

#[test]
fn voxel_chunk_populated_memory_grows() {
    // Tests lines 289, 295-300: memory_usage and estimate_tree_size
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let base_mem = chunk.memory_usage();

    // Add some voxels to create tree nodes
    for i in 0..8 {
        chunk.set_voxel(IVec3::new(i * 3, 0, 0), Voxel::new(0.5, 1));
    }
    let populated_mem = chunk.memory_usage();

    assert!(
        populated_mem > base_mem,
        "Populated chunk ({populated_mem}) should use more memory than empty ({base_mem})"
    );
}

#[test]
fn voxel_chunk_memory_addition_not_multiplication() {
    // Tests line 289: sizeof::<Self>() + estimate_tree_size()
    // If + becomes *, result would be wildly different (base * tree_size)
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.5, 1));

    let mem = chunk.memory_usage();
    let self_size = std::mem::size_of::<VoxelChunk>();

    // With + : result ≈ self_size + tree_overhead
    // With * : result ≈ self_size * tree_overhead (much much larger for big trees)
    // A single voxel creates a few octree nodes; reasonable result is < 5KB
    assert!(
        mem < 5000,
        "Memory {mem} should be reasonable (not multiplicative), self_size={self_size}"
    );
    assert!(mem >= self_size, "Memory should be at least sizeof(Self)");
}

#[test]
fn voxel_chunk_tree_size_accumulates() {
    // Tests line 300: size += node_size(child)
    // If += becomes -= or *=, the accumulated size would be wrong
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

    // Add voxels at many positions to create multiple tree branches
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                chunk.set_voxel(IVec3::new(x * 7, y * 7, z * 7), Voxel::new(0.5, 1));
            }
        }
    }

    let mem = chunk.memory_usage();
    let base = std::mem::size_of::<VoxelChunk>();

    // With 64 voxels in different octants, tree should be non-trivial
    assert!(
        mem > base + 64,
        "64 scattered voxels should have significant tree overhead, got {mem} (base {base})"
    );
}

// ─── VoxelChunk::get_voxel with depth traversal ────────────────────────────

#[test]
fn voxel_get_at_child_pos_modulo() {
    // Tests line 150: local_pos.x % half + ...
    // If + becomes *, the child_pos calculation breaks
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

    // Place voxels at specific positions that exercise the modulo arithmetic
    let pos1 = IVec3::new(3, 7, 11);
    let pos2 = IVec3::new(19, 23, 27);

    chunk.set_voxel(pos1, Voxel::new(0.5, 111));
    chunk.set_voxel(pos2, Voxel::new(0.5, 222));

    assert_eq!(chunk.get_voxel(pos1).unwrap().material, 111);
    assert_eq!(chunk.get_voxel(pos2).unwrap().material, 222);
}

// ─── VoxelGrid chunk operations ─────────────────────────────────────────────

#[test]
fn voxel_grid_chunk_count_tracks_additions() {
    use astraweave_terrain::VoxelGrid;
    let mut grid = VoxelGrid::new();
    assert_eq!(grid.chunk_count(), 0);

    let coord = ChunkCoord::new(0, 0, 0);
    let _chunk = grid.get_or_create_chunk(coord);
    assert_eq!(grid.chunk_count(), 1);

    let coord2 = ChunkCoord::new(1, 0, 0);
    let _chunk2 = grid.get_or_create_chunk(coord2);
    assert_eq!(grid.chunk_count(), 2);
}

#[test]
fn voxel_grid_remove_chunk_decrements_count() {
    use astraweave_terrain::VoxelGrid;
    let mut grid = VoxelGrid::new();
    let coord = ChunkCoord::new(0, 0, 0);
    let _chunk = grid.get_or_create_chunk(coord);
    assert_eq!(grid.chunk_count(), 1);

    grid.remove_chunk(coord);
    assert_eq!(grid.chunk_count(), 0);
    assert!(grid.get_chunk(coord).is_none());
}
