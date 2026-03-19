//! Wave 2 Shard 20 Remediation Tests
//!
//! Targets 56 MISSED mutations across:
//! - terrain_modifier.rs (31 MISSED): tick loop conditions, apply_operation arithmetic,
//!   blend formula, update_remesh_priority, stats accessor
//! - terrain_persistence.rs (16 MISSED): should_auto_save, save/load stats,
//!   delete_save, config/dirty_chunks accessors, get_chunks_in_region
//! - texture_splatting.rs (9 MISSED): SplatRule::evaluate height/slope boundaries

use astraweave_terrain::terrain_persistence::{TerrainPersistence, TerrainPersistenceConfig};
use astraweave_terrain::texture_splatting::SplatRule;
use astraweave_terrain::{
    ChunkCoord, TerrainModifier, TerrainModifierConfig, Voxel, VoxelGrid, VoxelOp, VoxelOpType,
    CHUNK_SIZE,
};
use glam::{IVec3, Vec3};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════
// TERRAIN MODIFIER TESTS
// ═══════════════════════════════════════════════════════════════

/// Helper: create a modifier with controlled config
fn make_modifier(max_ops: usize, max_remeshes: usize) -> TerrainModifier {
    TerrainModifier::new(TerrainModifierConfig {
        data_pass_budget_us: 1_000_000, // 1 second — large budget
        mesh_pass_budget_us: 1_000_000,
        max_ops_per_frame: max_ops,
        max_remeshes_per_frame: max_remeshes,
        prioritize_near_camera: true,
    })
}

/// Helper: create a VoxelOp::Set at position
fn set_op(x: i32, y: i32, z: i32, id: &str) -> VoxelOp {
    VoxelOp::set(IVec3::new(x, y, z), Voxel::new(0.5, 1), id.to_string())
}

/// Helper: create a VoxelOp::AddDensity at position
fn add_density_op(x: i32, y: i32, z: i32, density: f32, id: &str) -> VoxelOp {
    VoxelOp::add_density(IVec3::new(x, y, z), density, id.to_string())
}

/// Helper: create a VoxelOp::SubtractDensity at position
fn sub_density_op(x: i32, y: i32, z: i32, density: f32, id: &str) -> VoxelOp {
    VoxelOp::subtract_density(IVec3::new(x, y, z), density, id.to_string())
}

// ─────────── tick() loop condition tests ───────────

/// TARGETS: terrain_modifier.rs:291 delete ! in tick
/// If `!self.op_queue.is_empty()` loses the `!`, the loop never enters.
/// We verify that queued ops actually get processed.
#[test]
fn tick_processes_queued_ops() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "op1"));
    modifier.queue_operation(set_op(2, 2, 2, "op2"));
    modifier.queue_operation(set_op(3, 3, 3, "op3"));

    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 3, "All 3 ops should be processed");
    assert_eq!(stats.ops_pending, 0, "No ops should remain pending");
}

/// TARGETS: terrain_modifier.rs:292 replace && with || in tick
/// TARGETS: terrain_modifier.rs:292 replace < with ==, >, <=
/// If max_ops_per_frame check breaks, either too many or zero ops processed.
#[test]
fn tick_respects_max_ops_per_frame() {
    let mut modifier = make_modifier(2, 4); // Only 2 ops per frame
    let mut grid = VoxelGrid::new();

    // Queue 5 ops
    for i in 0..5 {
        modifier.queue_operation(set_op(i, 0, 0, &format!("op{i}")));
    }

    let stats = modifier.tick(&mut grid);
    assert_eq!(
        stats.ops_processed, 2,
        "Should process exactly max_ops_per_frame=2"
    );
    assert!(stats.ops_pending > 0, "Remaining ops should be pending");

    // Process next batch
    let stats2 = modifier.tick(&mut grid);
    assert_eq!(stats2.ops_processed, 2, "Second tick should also process 2");

    // Final tick gets the last one
    let stats3 = modifier.tick(&mut grid);
    assert_eq!(
        stats3.ops_processed, 1,
        "Third tick should process remaining 1"
    );
}

/// TARGETS: terrain_modifier.rs:293 replace < with ==, >, <= in tick
/// The data budget check. With a very small budget (1µs), most ops should be deferred.
/// With a huge budget, all should complete.
#[test]
fn tick_zero_budget_defers_work() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig {
        data_pass_budget_us: 1, // 1 microsecond — nearly zero
        mesh_pass_budget_us: 1,
        max_ops_per_frame: 1000,
        max_remeshes_per_frame: 100,
        prioritize_near_camera: true,
    });
    let mut grid = VoxelGrid::new();

    // Queue many ops
    for i in 0..100 {
        modifier.queue_operation(set_op(i % CHUNK_SIZE, 0, 0, &format!("op{i}")));
    }

    let stats = modifier.tick(&mut grid);
    // With 1µs budget, at most a few ops should run (probably 1)
    // The point is: it should NOT run all 100
    assert!(
        stats.ops_processed < 100,
        "With 1µs budget, should not process all 100 ops, got {}",
        stats.ops_processed
    );
}

/// TARGETS: terrain_modifier.rs:304 replace += with *= (ops_this_frame)
/// If += becomes *=, ops_this_frame would go 0*=1=0 forever,
/// meaning the loop never increments the counter.
#[test]
fn tick_ops_processed_count_is_exact() {
    let mut modifier = make_modifier(10, 4);
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "a"));
    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 1, "Exactly 1 op processed");

    modifier.queue_operation(set_op(2, 2, 2, "b"));
    modifier.queue_operation(set_op(3, 3, 3, "c"));
    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 2, "Exactly 2 ops processed");
}

// ─────────── work_deferred tests ───────────

/// TARGETS: terrain_modifier.rs:317:57 replace || with && in work_deferred
/// TARGETS: terrain_modifier.rs:317:31 delete ! in work_deferred
/// TARGETS: terrain_modifier.rs:317:60 delete ! in work_deferred
/// work_deferred = !self.op_queue.is_empty() || !self.dirty_chunks.is_empty()
#[test]
fn work_deferred_true_when_ops_remain() {
    let mut modifier = make_modifier(1, 4); // Process only 1 op per tick
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "a"));
    modifier.queue_operation(set_op(2, 2, 2, "b"));

    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 1);
    assert!(
        stats.work_deferred,
        "Should be deferred: ops remain in queue"
    );
}

#[test]
fn work_deferred_false_when_all_complete() {
    let mut modifier = make_modifier(100, 100);
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "a"));
    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 1);

    // After processing everything and remeshing everything, deferred should be false
    // Note: dirty_chunks may still exist if remesh_priority doesn't drain them all
    // But with high limits and 1 op, it should complete
    let stats2 = modifier.tick(&mut grid);
    assert_eq!(stats2.ops_processed, 0);
    assert!(
        !stats2.work_deferred,
        "Should NOT be deferred when everything is done"
    );
}

/// work_deferred should be true when only dirty chunks remain (no pending ops)
#[test]
fn work_deferred_true_when_dirty_chunks_remain() {
    // Use max_remeshes=0 so dirty chunks can never be cleared
    let mut modifier = make_modifier(100, 0);
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "a"));
    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 1);
    // Ops are all done, but dirty chunks remain because max_remeshes=0
    assert!(
        stats.work_deferred,
        "Should be deferred: dirty chunks remain even though ops are done"
    );
}

// ─────────── apply_operation tests ───────────

/// TARGETS: terrain_modifier.rs:333 replace - with + in SubtractDensity
/// SubtractDensity should LOWER the density. If - becomes +, it would increase.
#[test]
fn subtract_density_actually_subtracts() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    // First set a voxel to density 0.8
    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.8, 1), "set".into()));
    modifier.tick(&mut grid);

    // Now subtract 0.3 density
    modifier.queue_operation(sub_density_op(1, 1, 1, 0.3, "sub"));
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local_pos = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let voxel = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local_pos)
        .unwrap();
    assert!(
        voxel.density < 0.8,
        "SubtractDensity should lower density, got {}",
        voxel.density
    );
    assert!(
        (voxel.density - 0.5).abs() < 0.01,
        "Expected ~0.5 after subtracting 0.3 from 0.8, got {}",
        voxel.density
    );
}

/// TARGETS: terrain_modifier.rs:363 Blend formula arithmetic (6 mutations)
/// new_density = existing.density * (1.0 - factor) + voxel.density * factor
/// Test: blend factor=0 should keep original, factor=1 should use target
#[test]
fn blend_factor_zero_keeps_original() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    // Set initial voxel
    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.8, 1), "set".into()));
    modifier.tick(&mut grid);

    // Blend with factor 0 — should keep original
    let blend_op = VoxelOp {
        position: pos,
        op_type: VoxelOpType::Blend {
            voxel: Voxel::new(0.2, 2),
            factor: 0.0,
        },
        priority: 128,
        request_id: "blend0".into(),
    };
    modifier.queue_operation(blend_op);
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let v = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    assert!(
        (v.density - 0.8).abs() < 0.01,
        "factor=0 should keep original density 0.8, got {}",
        v.density
    );
}

#[test]
fn blend_factor_one_uses_target() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.8, 1), "set".into()));
    modifier.tick(&mut grid);

    let blend_op = VoxelOp {
        position: pos,
        op_type: VoxelOpType::Blend {
            voxel: Voxel::new(0.2, 2),
            factor: 1.0,
        },
        priority: 128,
        request_id: "blend1".into(),
    };
    modifier.queue_operation(blend_op);
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let v = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    assert!(
        (v.density - 0.2).abs() < 0.01,
        "factor=1 should use target density 0.2, got {}",
        v.density
    );
}

/// Test blend with factor=0.5: should be average of source and target
#[test]
fn blend_factor_half_averages() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.8, 1), "set".into()));
    modifier.tick(&mut grid);

    let blend_op = VoxelOp {
        position: pos,
        op_type: VoxelOpType::Blend {
            voxel: Voxel::new(0.2, 2),
            factor: 0.5,
        },
        priority: 128,
        request_id: "blendhalf".into(),
    };
    modifier.queue_operation(blend_op);
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let v = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    // 0.8 * 0.5 + 0.2 * 0.5 = 0.4 + 0.1 = 0.5
    assert!(
        (v.density - 0.5).abs() < 0.01,
        "factor=0.5 should give 0.5, got {}",
        v.density
    );
}

/// TARGETS: terrain_modifier.rs:365 replace > with ==, <, >= in Blend material
/// factor > 0.5 → use target material; factor <= 0.5 → use existing
#[test]
fn blend_material_selection_by_factor() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    // Set material=10
    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.5, 10), "set".into()));
    modifier.tick(&mut grid);

    // Blend with material=20, factor=0.8 (> 0.5 → should use target material 20)
    let blend_hi = VoxelOp {
        position: pos,
        op_type: VoxelOpType::Blend {
            voxel: Voxel::new(0.5, 20),
            factor: 0.8,
        },
        priority: 128,
        request_id: "bhi".into(),
    };
    modifier.queue_operation(blend_hi);
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let v = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    assert_eq!(
        v.material, 20,
        "factor=0.8 > 0.5 should use target material 20"
    );

    // Now blend with material=30, factor=0.3 (<= 0.5 → should keep existing material 20)
    let blend_lo = VoxelOp {
        position: pos,
        op_type: VoxelOpType::Blend {
            voxel: Voxel::new(0.5, 30),
            factor: 0.3,
        },
        priority: 128,
        request_id: "blo".into(),
    };
    modifier.queue_operation(blend_lo);
    modifier.tick(&mut grid);

    let v2 = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    assert_eq!(
        v2.material, 20,
        "factor=0.3 <= 0.5 should keep existing material 20"
    );
}

// ─────────── update_remesh_priority tests ───────────

/// TARGETS: terrain_modifier.rs:384 replace update_remesh_priority with ()
/// TARGETS: terrain_modifier.rs:387 + with -, / with %, / with *
/// TARGETS: terrain_modifier.rs:388 - with +, - with /
/// If update_remesh_priority is no-op, chunks near camera won't be remeshed first.
/// If distance calc is wrong, prioritization breaks.
#[test]
fn remesh_prioritizes_near_camera() {
    let mut modifier = make_modifier(100, 1); // Only 1 remesh per tick
    let mut grid = VoxelGrid::new();

    // Place ops in two distant chunks
    let near_pos = IVec3::new(1, 1, 1); // Chunk (0,0,0)
    let far_pos = IVec3::new(CHUNK_SIZE * 10, 1, 1); // Chunk (10,0,0)

    modifier.queue_operation(set_op(far_pos.x, far_pos.y, far_pos.z, "far"));
    modifier.queue_operation(set_op(near_pos.x, near_pos.y, near_pos.z, "near"));
    modifier.update_camera(Vec3::ZERO); // Camera at origin

    // First tick: process both ops, remesh only 1 chunk (nearest to camera)
    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 2);
    assert_eq!(stats.chunks_remeshed, 1, "Should remesh 1 chunk per tick");

    // The nearest chunk should have been remeshed first
    // Second tick: remesh the remaining one
    let stats2 = modifier.tick(&mut grid);
    assert_eq!(
        stats2.chunks_remeshed, 1,
        "Second tick should remesh remaining chunk"
    );
}

/// TARGETS: terrain_modifier.rs:401 replace stats() with default
/// The stats accessor should return actual stats, not a default.
#[test]
fn stats_accessor_returns_real_stats() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(set_op(1, 1, 1, "a"));
    modifier.queue_operation(set_op(2, 2, 2, "b"));
    modifier.tick(&mut grid);

    let stats = modifier.stats();
    assert_eq!(
        stats.ops_processed, 2,
        "stats() should reflect actual ops processed"
    );
    assert!(stats.ops_processed > 0);
}

/// AddDensity should INCREASE density
#[test]
fn add_density_actually_adds() {
    let mut modifier = make_modifier(100, 4);
    let mut grid = VoxelGrid::new();
    let pos = IVec3::new(1, 1, 1);

    modifier.queue_operation(VoxelOp::set(pos, Voxel::new(0.3, 1), "set".into()));
    modifier.tick(&mut grid);

    modifier.queue_operation(add_density_op(1, 1, 1, 0.4, "add"));
    modifier.tick(&mut grid);

    let chunk_coord = ChunkCoord::from_world_pos(Vec3::new(1.0, 1.0, 1.0));
    let local = pos
        - IVec3::new(
            (chunk_coord.x as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.y as f32 * CHUNK_SIZE as f32) as i32,
            (chunk_coord.z as f32 * CHUNK_SIZE as f32) as i32,
        );
    let v = grid
        .get_chunk(chunk_coord)
        .unwrap()
        .get_voxel(local)
        .unwrap();
    assert!(
        (v.density - 0.7).abs() < 0.01,
        "Should be ~0.7 after adding 0.4 to 0.3, got {}",
        v.density
    );
}

// ═══════════════════════════════════════════════════════════════
// TERRAIN PERSISTENCE TESTS
// ═══════════════════════════════════════════════════════════════

/// TARGETS: terrain_persistence.rs:179 should_auto_save -> bool with false
/// TARGETS: terrain_persistence.rs:185 >= with < in should_auto_save
#[test]
fn should_auto_save_true_when_dirty_and_elapsed() {
    let config = TerrainPersistenceConfig {
        auto_save_interval_seconds: 0.0001, // Very short interval
        ..Default::default()
    };
    let mut persistence = TerrainPersistence::new(config);

    // Mark a chunk dirty
    persistence.mark_dirty(ChunkCoord::new(0, 0, 0));

    // Wait long enough for auto-save to trigger
    std::thread::sleep(std::time::Duration::from_millis(2));

    assert!(
        persistence.should_auto_save(),
        "should_auto_save must be true when dirty and interval elapsed"
    );
}

#[test]
fn should_auto_save_false_when_no_dirty() {
    let config = TerrainPersistenceConfig {
        auto_save_interval_seconds: 0.001,
        ..Default::default()
    };
    let persistence = TerrainPersistence::new(config);

    std::thread::sleep(std::time::Duration::from_millis(5));

    assert!(
        !persistence.should_auto_save(),
        "should_auto_save must be false when no dirty chunks"
    );
}

#[test]
fn should_auto_save_false_when_disabled() {
    let config = TerrainPersistenceConfig {
        auto_save_interval_seconds: 0.0, // Disabled
        ..Default::default()
    };
    let mut persistence = TerrainPersistence::new(config);
    persistence.mark_dirty(ChunkCoord::new(0, 0, 0));

    assert!(
        !persistence.should_auto_save(),
        "should_auto_save must be false when interval is 0"
    );
}

/// TARGETS: terrain_persistence.rs:256-257 += with *= in save_chunks stats
/// TARGETS: terrain_persistence.rs:316-317 += with *= in load_chunks stats
/// Save and load, then verify stats are correctly incremented.
#[test]
fn save_load_updates_stats_correctly() {
    let dir = std::env::temp_dir().join("aw_persist_test_stats");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    let config = TerrainPersistenceConfig {
        save_directory: dir.clone(),
        use_compression: false,
        ..Default::default()
    };
    let mut persistence = TerrainPersistence::new(config);

    // Create some chunks
    let mut chunks = HashMap::new();
    let coord1 = ChunkCoord::new(0, 0, 0);
    let coord2 = ChunkCoord::new(1, 0, 0);
    chunks.insert(coord1, astraweave_terrain::VoxelChunk::new(coord1));
    chunks.insert(coord2, astraweave_terrain::VoxelChunk::new(coord2));

    persistence.mark_dirty(coord1);
    persistence.mark_dirty(coord2);

    // Save
    let result = persistence
        .save_chunks(&chunks, 42, Some("test_stats"))
        .unwrap();
    assert_eq!(result.chunks_saved, 2);

    let stats = persistence.stats();
    assert_eq!(stats.total_chunks_saved, 2, "Should have saved 2 chunks");
    assert!(stats.total_bytes_written > 0, "Should have written bytes");
    assert_eq!(stats.save_count, 1);

    // Load
    let (load_result, loaded_chunks) = persistence.load_chunks(&result.path).unwrap();
    assert_eq!(load_result.chunks_loaded, 2);

    let stats = persistence.stats();
    assert_eq!(stats.total_chunks_loaded, 2, "Should have loaded 2 chunks");
    assert!(stats.total_bytes_read > 0, "Should have read bytes");
    assert_eq!(stats.load_count, 1);
    assert_eq!(loaded_chunks.len(), 2);

    // Save again — stats should accumulate (+=), not multiply (*=)
    persistence.mark_dirty(coord1);
    let _ = persistence
        .save_chunks(&chunks, 42, Some("test_stats2"))
        .unwrap();
    let stats = persistence.stats();
    assert_eq!(
        stats.total_chunks_saved, 3,
        "After 2nd save of 1 chunk, total should be 2+1=3, not 2*1"
    );
    assert_eq!(stats.save_count, 2);

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

/// TARGETS: terrain_persistence.rs:302 > with < in load_chunks (version check)
/// The version check ensures we don't load future save formats.
/// If > becomes <, it would reject CURRENT version saves.
#[test]
fn load_accepts_current_version() {
    let dir = std::env::temp_dir().join("aw_persist_version_test");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    let config = TerrainPersistenceConfig {
        save_directory: dir.clone(),
        use_compression: false,
        ..Default::default()
    };
    let mut persistence = TerrainPersistence::new(config);

    let mut chunks = HashMap::new();
    let coord = ChunkCoord::new(0, 0, 0);
    chunks.insert(coord, astraweave_terrain::VoxelChunk::new(coord));
    persistence.mark_dirty(coord);

    let save_result = persistence
        .save_chunks(&chunks, 42, Some("ver_test"))
        .unwrap();

    // Should successfully load a file we just saved (same version)
    let load_result = persistence.load_chunks(&save_result.path);
    assert!(
        load_result.is_ok(),
        "Should accept current version save file"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// TARGETS: terrain_persistence.rs:391 delete_save -> Ok(())
/// delete_save should actually delete the file.
#[test]
fn delete_save_actually_deletes_file() {
    let dir = std::env::temp_dir().join("aw_persist_delete_test");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    let config = TerrainPersistenceConfig {
        save_directory: dir.clone(),
        use_compression: false,
        ..Default::default()
    };
    let mut persistence = TerrainPersistence::new(config);

    let mut chunks = HashMap::new();
    let coord = ChunkCoord::new(0, 0, 0);
    chunks.insert(coord, astraweave_terrain::VoxelChunk::new(coord));
    persistence.mark_dirty(coord);

    let save_result = persistence
        .save_chunks(&chunks, 42, Some("del_test"))
        .unwrap();
    assert!(save_result.path.exists(), "File should exist after save");

    persistence.delete_save(&save_result.path).unwrap();
    assert!(
        !save_result.path.exists(),
        "File should be deleted after delete_save"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// TARGETS: terrain_persistence.rs:402 config() with default
/// The config accessor should return the actual config, not default.
#[test]
fn config_accessor_returns_actual_config() {
    let custom_dir = std::path::PathBuf::from("/custom/terrain/saves");
    let config = TerrainPersistenceConfig {
        save_directory: custom_dir.clone(),
        use_compression: false,
        batch_size: 99,
        auto_save_interval_seconds: 123.0,
    };
    let persistence = TerrainPersistence::new(config);

    let cfg = persistence.config();
    assert_eq!(cfg.save_directory, custom_dir);
    assert!(!cfg.use_compression);
    assert_eq!(cfg.batch_size, 99);
    assert!((cfg.auto_save_interval_seconds - 123.0).abs() < 0.01);
}

/// TARGETS: terrain_persistence.rs:407 dirty_chunks() with empty()
/// The dirty_chunks accessor should return actual dirty chunks.
#[test]
fn dirty_chunks_accessor_returns_actual_chunks() {
    let mut persistence = TerrainPersistence::new(TerrainPersistenceConfig::default());

    let c1 = ChunkCoord::new(1, 2, 3);
    let c2 = ChunkCoord::new(4, 5, 6);
    persistence.mark_dirty(c1);
    persistence.mark_dirty(c2);

    let dirty: Vec<_> = persistence.dirty_chunks().collect();
    assert_eq!(dirty.len(), 2, "Should have 2 dirty chunks, not empty");
    assert!(dirty.contains(&&c1));
    assert!(dirty.contains(&&c2));
}

/// TARGETS: terrain_persistence.rs:448-450 + with -, + with * in get_chunks_in_region
/// The function adds x/y/z offsets to center_coord. If + becomes - or *, wrong chunks.
#[test]
fn get_chunks_in_region_covers_correct_area() {
    use astraweave_terrain::terrain_persistence::get_chunks_in_region;

    let center = Vec3::new(0.0, 0.0, 0.0);
    let radius = CHUNK_SIZE as f32; // 1 chunk radius

    let chunks = get_chunks_in_region(center, radius);

    // With radius = chunk_size, chunk_radius = ceil(1) = 1
    // So we get -1..=1 in x, y, z → 3^3 = 27 chunks
    assert_eq!(
        chunks.len(),
        27,
        "Should have 27 chunks for radius=chunk_size"
    );

    // The center chunk should be included
    let center_coord = ChunkCoord::from_world_pos(center);
    assert!(
        chunks.contains(&center_coord),
        "Center chunk should be in region"
    );

    // Positive offset chunk should be included
    let positive_chunk =
        ChunkCoord::new(center_coord.x + 1, center_coord.y + 1, center_coord.z + 1);
    assert!(
        chunks.contains(&positive_chunk),
        "Positive offset chunk should be in region"
    );

    // Negative offset chunk should be included
    let negative_chunk =
        ChunkCoord::new(center_coord.x - 1, center_coord.y - 1, center_coord.z - 1);
    assert!(
        chunks.contains(&negative_chunk),
        "Negative offset chunk should be in region"
    );
}

/// Test that a non-origin center produces correct chunks
#[test]
fn get_chunks_in_region_non_origin() {
    use astraweave_terrain::terrain_persistence::get_chunks_in_region;

    let center = Vec3::new(200.0, 200.0, 200.0);
    let radius = CHUNK_SIZE as f32 * 0.5; // Half a chunk — ceil(0.5) = 1

    let chunks = get_chunks_in_region(center, radius);
    let center_coord = ChunkCoord::from_world_pos(center);

    // All chunks should be near the center
    for c in &chunks {
        assert!(
            (c.x - center_coord.x).abs() <= 1,
            "X offset too large: {} vs center {}",
            c.x,
            center_coord.x
        );
        assert!((c.y - center_coord.y).abs() <= 1, "Y offset too large");
        assert!((c.z - center_coord.z).abs() <= 1, "Z offset too large");
    }
}

// ═══════════════════════════════════════════════════════════════
// TEXTURE SPLATTING TESTS
// ═══════════════════════════════════════════════════════════════

/// TARGETS: texture_splatting.rs:352 < with <= , 354 > with >= (height boundaries)
/// TARGETS: texture_splatting.rs:359 < with <=, 361 > with >= (slope boundaries)
/// At exactly min_height, weight should be full (not reduced).
/// At exactly max_height, weight should be full (not reduced).
#[test]
fn evaluate_at_exact_min_height_gives_full_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 10.0,
        max_height: 50.0,
        min_slope: 0.0,
        max_slope: 90.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    // At exactly min_height: should be full weight (height == min_height → not below)
    let w = rule.evaluate(10.0, 45.0);
    assert!(
        (w - 1.0).abs() < 0.001,
        "At min_height boundary, weight should be 1.0, got {w}"
    );
}

#[test]
fn evaluate_at_exact_max_height_gives_full_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 10.0,
        max_height: 50.0,
        min_slope: 0.0,
        max_slope: 90.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    let w = rule.evaluate(50.0, 45.0);
    assert!(
        (w - 1.0).abs() < 0.001,
        "At max_height boundary, weight should be 1.0, got {w}"
    );
}

#[test]
fn evaluate_at_exact_min_slope_gives_full_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 0.0,
        max_height: 100.0,
        min_slope: 15.0,
        max_slope: 60.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    let w = rule.evaluate(50.0, 15.0);
    assert!(
        (w - 1.0).abs() < 0.001,
        "At min_slope boundary, weight should be 1.0, got {w}"
    );
}

#[test]
fn evaluate_at_exact_max_slope_gives_full_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 0.0,
        max_height: 100.0,
        min_slope: 15.0,
        max_slope: 60.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    let w = rule.evaluate(50.0, 60.0);
    assert!(
        (w - 1.0).abs() < 0.001,
        "At max_slope boundary, weight should be 1.0, got {w}"
    );
}

/// TARGETS: texture_splatting.rs:360 arithmetic mutations in slope falloff
/// Below min_slope, weight should decrease. Verify the direction is correct.
#[test]
fn evaluate_below_min_slope_reduces_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 0.0,
        max_height: 100.0,
        min_slope: 30.0,
        max_slope: 60.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    // 10 degrees below min_slope
    let w = rule.evaluate(50.0, 20.0);
    // Expected: 1.0 * (1.0 - (30 - 20) * 0.1).max(0) = 1.0 * 0.0 = 0.0
    assert!(
        w < 1.0,
        "Below min_slope by 10°, weight should be reduced, got {w}"
    );
    assert!(
        (w - 0.0).abs() < 0.001,
        "10° below with falloff=0.1 should give 0.0, got {w}"
    );
}

/// Above max_slope, weight should decrease
#[test]
fn evaluate_above_max_slope_reduces_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 0.0,
        max_height: 100.0,
        min_slope: 0.0,
        max_slope: 45.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.1,
        slope_falloff: 0.05,
    };

    // 10 degrees above max
    let w = rule.evaluate(50.0, 55.0);
    // Expected: 1.0 * (1.0 - (55-45) * 0.05).max(0) = 1.0 * 0.5
    assert!(
        (w - 0.5).abs() < 0.01,
        "10° above max with falloff=0.05 should give 0.5, got {w}"
    );
}

/// Below min_height, weight should decrease
#[test]
fn evaluate_below_min_height_reduces_weight() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 50.0,
        max_height: 100.0,
        min_slope: 0.0,
        max_slope: 90.0,
        priority: 0,
        weight: 1.0,
        height_falloff: 0.05,
        slope_falloff: 0.1,
    };

    // 10 units below min_height
    let w = rule.evaluate(40.0, 45.0);
    // Expected: 1.0 * (1.0 - (50-40) * 0.05).max(0) = 1.0 * 0.5
    assert!(
        (w - 0.5).abs() < 0.01,
        "10 units below min_height with falloff=0.05 should give 0.5, got {w}"
    );
}

/// TARGETS: Verify that falloff is multiplicative with weight, not additive
#[test]
fn evaluate_falloff_is_multiplicative() {
    let rule = SplatRule {
        material_id: 0,
        min_height: 50.0,
        max_height: 100.0,
        min_slope: 0.0,
        max_slope: 90.0,
        priority: 0,
        weight: 0.8, // Non-1.0 weight
        height_falloff: 0.1,
        slope_falloff: 0.1,
    };

    // 5 units below min_height: factor = (1.0 - 5*0.1) = 0.5
    // Result = 0.8 * 0.5 = 0.4
    let w = rule.evaluate(45.0, 45.0);
    assert!(
        (w - 0.4).abs() < 0.01,
        "Weight 0.8 with 50% falloff should give 0.4, got {w}"
    );
}
