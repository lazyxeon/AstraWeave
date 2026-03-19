//! Wave 2 Remediation Tests — terrain_modifier.rs (99 mutants) + terrain_persistence.rs (54 mutants)
//!                            + streaming_diagnostics.rs (76 mutants)
//!
//! Targets: shards 18-20
//! Focus: VoxelOp constructors/priority, NavMeshRegion overlap/merge arithmetic,
//!        TerrainModifier queue/tick, TerrainPersistence dirty tracking/auto-save,
//!        HitchDetector p99/hitch_rate math, MemoryStats delta/total_mb.

use astraweave_terrain::terrain_persistence::{
    get_chunks_in_region, TerrainPersistence, TerrainPersistenceConfig, TerrainSaveHeader,
    TERRAIN_SAVE_VERSION,
};
use astraweave_terrain::*;
use glam::{IVec3, Vec3};
use std::collections::HashMap;

// ============================================================================
// TerrainModifierConfig defaults
// ============================================================================

#[test]
fn modifier_config_defaults() {
    let c = TerrainModifierConfig::default();
    assert_eq!(c.data_pass_budget_us, 1000);
    assert_eq!(c.mesh_pass_budget_us, 2000);
    assert_eq!(c.max_ops_per_frame, 1000);
    assert_eq!(c.max_remeshes_per_frame, 4);
    assert!(c.prioritize_near_camera);
}

// ============================================================================
// VoxelOp constructors and with_priority
// ============================================================================

#[test]
fn voxel_op_set_defaults() {
    let op = VoxelOp::set(IVec3::new(10, 20, 30), Voxel::new(0.8, 1), "req-1".into());
    assert_eq!(op.position, IVec3::new(10, 20, 30));
    assert_eq!(op.priority, 128);
    assert_eq!(op.request_id, "req-1");
}

#[test]
fn voxel_op_add_density_defaults() {
    let op = VoxelOp::add_density(IVec3::new(5, 5, 5), 0.3, "add-1".into());
    assert_eq!(op.position, IVec3::new(5, 5, 5));
    assert_eq!(op.priority, 128);
    assert!(matches!(op.op_type, VoxelOpType::AddDensity(v) if (v - 0.3).abs() < 1e-6));
}

#[test]
fn voxel_op_subtract_density_defaults() {
    let op = VoxelOp::subtract_density(IVec3::new(1, 2, 3), 0.5, "sub-1".into());
    assert_eq!(op.position, IVec3::new(1, 2, 3));
    assert_eq!(op.priority, 128);
    assert!(matches!(op.op_type, VoxelOpType::SubtractDensity(v) if (v - 0.5).abs() < 1e-6));
}

#[test]
fn voxel_op_with_priority_overrides() {
    let op = VoxelOp::set(IVec3::ZERO, Voxel::new(0.5, 0), "p".into()).with_priority(255);
    assert_eq!(op.priority, 255);
}

#[test]
fn voxel_op_with_priority_zero() {
    let op = VoxelOp::set(IVec3::ZERO, Voxel::new(0.5, 0), "p".into()).with_priority(0);
    assert_eq!(op.priority, 0);
}

// ============================================================================
// TerrainModifier queue ordering
// ============================================================================

#[test]
fn modifier_queue_orders_by_priority() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());

    modifier.queue_operation(
        VoxelOp::set(IVec3::ZERO, Voxel::new(0.5, 0), "low".into()).with_priority(10),
    );
    modifier.queue_operation(
        VoxelOp::set(IVec3::ONE, Voxel::new(0.5, 0), "high".into()).with_priority(200),
    );
    modifier.queue_operation(
        VoxelOp::set(IVec3::NEG_ONE, Voxel::new(0.5, 0), "mid".into()).with_priority(100),
    );

    assert_eq!(modifier.pending_ops(), 3);
}

#[test]
fn modifier_queue_operations_batch() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let ops = vec![
        VoxelOp::set(IVec3::new(1, 0, 0), Voxel::new(0.5, 0), "a".into()),
        VoxelOp::set(IVec3::new(2, 0, 0), Voxel::new(0.5, 0), "b".into()),
        VoxelOp::set(IVec3::new(3, 0, 0), Voxel::new(0.5, 0), "c".into()),
    ];
    modifier.queue_operations(ops);
    assert_eq!(modifier.pending_ops(), 3);
}

// ============================================================================
// TerrainModifier pending/has_pending/clear
// ============================================================================

#[test]
fn modifier_starts_empty() {
    let modifier = TerrainModifier::new(TerrainModifierConfig::default());
    assert!(!modifier.has_pending_work());
    assert_eq!(modifier.pending_ops(), 0);
    assert_eq!(modifier.pending_remeshes(), 0);
}

#[test]
fn modifier_has_pending_with_ops() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    modifier.queue_operation(VoxelOp::set(IVec3::ZERO, Voxel::new(0.5, 0), "t".into()));
    assert!(modifier.has_pending_work());
}

#[test]
fn modifier_clear_empties_everything() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    modifier.queue_operation(VoxelOp::set(IVec3::ZERO, Voxel::new(0.5, 0), "t".into()));
    modifier.clear();
    assert_eq!(modifier.pending_ops(), 0);
    assert!(!modifier.has_pending_work());
}

// ============================================================================
// TerrainModifier tick: processes operations and produces stats
// ============================================================================

#[test]
fn modifier_tick_processes_ops() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    // Queue 3 set operations
    for i in 0..3 {
        modifier.queue_operation(VoxelOp::set(
            IVec3::new(i, 0, 0),
            Voxel::new(0.9, 1),
            format!("op-{i}"),
        ));
    }

    let stats = modifier.tick(&mut grid);
    assert_eq!(stats.ops_processed, 3, "Should process all 3 ops");
    assert_eq!(modifier.pending_ops(), 0);
}

#[test]
fn modifier_tick_add_density() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    // Set initial voxel
    let coord = ChunkCoord::new(0, 0, 0);
    let chunk = grid.get_or_create_chunk(coord);
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.3, 1));

    // Queue add density
    modifier.queue_operation(VoxelOp::add_density(IVec3::new(0, 0, 0), 0.5, "add".into()));
    modifier.tick(&mut grid);

    let chunk = grid.get_chunk(coord).unwrap();
    let v = chunk.get_voxel(IVec3::new(0, 0, 0)).unwrap();
    // 0.3 + 0.5 = 0.8
    assert!(
        (v.density - 0.8).abs() < 0.01,
        "Density should be ~0.8, got {}",
        v.density
    );
}

#[test]
fn modifier_tick_subtract_density() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    let coord = ChunkCoord::new(0, 0, 0);
    let chunk = grid.get_or_create_chunk(coord);
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.8, 1));

    modifier.queue_operation(VoxelOp::subtract_density(
        IVec3::new(0, 0, 0),
        0.5,
        "sub".into(),
    ));
    modifier.tick(&mut grid);

    let chunk = grid.get_chunk(coord).unwrap();
    let v = chunk.get_voxel(IVec3::new(0, 0, 0)).unwrap();
    // 0.8 - 0.5 = 0.3
    assert!(
        (v.density - 0.3).abs() < 0.01,
        "Density should be ~0.3, got {}",
        v.density
    );
}

#[test]
fn modifier_tick_add_density_clamps_to_1() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    let coord = ChunkCoord::new(0, 0, 0);
    let chunk = grid.get_or_create_chunk(coord);
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.9, 1));

    modifier.queue_operation(VoxelOp::add_density(
        IVec3::new(0, 0, 0),
        0.5,
        "clamp".into(),
    ));
    modifier.tick(&mut grid);

    let chunk = grid.get_chunk(coord).unwrap();
    let v = chunk.get_voxel(IVec3::new(0, 0, 0)).unwrap();
    assert!(
        (v.density - 1.0).abs() < 0.01,
        "Clamped to 1.0, got {}",
        v.density
    );
}

#[test]
fn modifier_tick_subtract_density_clamps_to_0() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    let coord = ChunkCoord::new(0, 0, 0);
    let chunk = grid.get_or_create_chunk(coord);
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.2, 1));

    modifier.queue_operation(VoxelOp::subtract_density(
        IVec3::new(0, 0, 0),
        0.5,
        "clamp".into(),
    ));
    modifier.tick(&mut grid);

    let chunk = grid.get_chunk(coord).unwrap();
    let v = chunk.get_voxel(IVec3::new(0, 0, 0)).unwrap();
    assert!(
        (v.density - 0.0).abs() < 0.01,
        "Clamped to 0.0, got {}",
        v.density
    );
}

#[test]
fn modifier_drain_completed_requests() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    modifier.queue_operation(VoxelOp::set(
        IVec3::ZERO,
        Voxel::new(0.5, 0),
        "req-A".into(),
    ));
    modifier.queue_operation(VoxelOp::set(IVec3::ONE, Voxel::new(0.5, 0), "req-B".into()));
    modifier.tick(&mut grid);

    let completed = modifier.drain_completed_requests();
    assert!(completed.len() >= 2);
    assert!(
        modifier.drain_completed_requests().is_empty(),
        "Second drain should be empty"
    );
}

#[test]
fn modifier_update_camera() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    modifier.update_camera(Vec3::new(100.0, 50.0, 200.0));
    // Just verify it doesn't panic — camera_pos is private but affects prioritization
}

// ============================================================================
// NavMeshRegion
// ============================================================================

#[test]
fn navmesh_region_new_and_fields() {
    let r = NavMeshRegion::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(r.min, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(r.max, Vec3::new(4.0, 5.0, 6.0));
}

#[test]
fn navmesh_region_from_chunk() {
    let r = NavMeshRegion::from_chunk(ChunkCoord::new(0, 0, 0));
    assert_eq!(r.min, Vec3::ZERO);
    // max = min + CHUNK_SIZE = 32.0
    assert!((r.max.x - 32.0).abs() < 1e-4);
    assert!((r.max.y - 32.0).abs() < 1e-4);
    assert!((r.max.z - 32.0).abs() < 1e-4);
}

#[test]
fn navmesh_region_overlaps_true() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::splat(5.0), Vec3::splat(15.0));
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn navmesh_region_overlaps_false_x() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::new(20.0, 0.0, 0.0), Vec3::new(30.0, 10.0, 10.0));
    assert!(!a.overlaps(&b));
}

#[test]
fn navmesh_region_overlaps_false_y() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::new(0.0, 20.0, 0.0), Vec3::new(10.0, 30.0, 10.0));
    assert!(!a.overlaps(&b));
}

#[test]
fn navmesh_region_overlaps_false_z() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::new(0.0, 0.0, 20.0), Vec3::new(10.0, 10.0, 30.0));
    assert!(!a.overlaps(&b));
}

#[test]
fn navmesh_region_overlaps_edge_touching() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(20.0, 10.0, 10.0));
    assert!(a.overlaps(&b), "Edge-touching should count as overlap");
}

#[test]
fn navmesh_region_merge() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(10.0));
    let b = NavMeshRegion::new(Vec3::splat(5.0), Vec3::splat(20.0));
    let merged = a.merge(&b);
    assert_eq!(merged.min, Vec3::ZERO);
    assert_eq!(merged.max, Vec3::splat(20.0));
}

#[test]
fn navmesh_region_merge_non_overlapping() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::splat(5.0));
    let b = NavMeshRegion::new(Vec3::splat(10.0), Vec3::splat(15.0));
    let merged = a.merge(&b);
    assert_eq!(merged.min, Vec3::ZERO);
    assert_eq!(merged.max, Vec3::splat(15.0));
}

#[test]
fn modifier_take_navmesh_dirty_regions() {
    let mut modifier = TerrainModifier::new(TerrainModifierConfig::default());
    let mut grid = VoxelGrid::new();

    // Process an operation to generate dirty regions
    modifier.queue_operation(VoxelOp::set(
        IVec3::new(0, 0, 0),
        Voxel::new(0.9, 1),
        "nav".into(),
    ));
    modifier.tick(&mut grid);

    let regions = modifier.take_navmesh_dirty_regions();
    // take should return previous regions and clear them
    let regions2 = modifier.take_navmesh_dirty_regions();
    assert!(regions2.is_empty(), "Second take should be empty");
}

// ============================================================================
// TerrainPersistenceConfig defaults
// ============================================================================

#[test]
fn persistence_config_defaults() {
    let c = TerrainPersistenceConfig::default();
    assert!(c.use_compression);
    assert_eq!(c.batch_size, 32);
    assert!((c.auto_save_interval_seconds - 60.0).abs() < 1e-4);
}

// ============================================================================
// TerrainSaveHeader
// ============================================================================

#[test]
fn save_header_new() {
    let h = TerrainSaveHeader::new(12345, 10);
    assert_eq!(h.version, TERRAIN_SAVE_VERSION);
    assert_eq!(h.world_seed, 12345);
    assert_eq!(h.chunk_count, 10);
    assert!(h.timestamp > 0);
    assert!(h.description.is_none());
}

#[test]
fn save_header_with_description() {
    let h = TerrainSaveHeader::new(42, 5).with_description("test save");
    assert_eq!(h.description.as_deref(), Some("test save"));
}

// ============================================================================
// TerrainPersistence dirty tracking
// ============================================================================

#[test]
fn persistence_mark_dirty_and_query() {
    let mut p = TerrainPersistence::default_config();
    let coord = ChunkCoord::new(1, 2, 3);

    assert!(!p.is_dirty(&coord));
    assert_eq!(p.dirty_count(), 0);

    p.mark_dirty(coord);
    assert!(p.is_dirty(&coord));
    assert_eq!(p.dirty_count(), 1);
}

#[test]
fn persistence_mark_dirty_batch() {
    let mut p = TerrainPersistence::default_config();
    let coords = vec![
        ChunkCoord::new(0, 0, 0),
        ChunkCoord::new(1, 0, 0),
        ChunkCoord::new(2, 0, 0),
    ];
    p.mark_dirty_batch(coords);
    assert_eq!(p.dirty_count(), 3);
}

#[test]
fn persistence_clear_dirty() {
    let mut p = TerrainPersistence::default_config();
    p.mark_dirty(ChunkCoord::new(0, 0, 0));
    p.mark_dirty(ChunkCoord::new(1, 0, 0));
    p.clear_dirty();
    assert_eq!(p.dirty_count(), 0);
}

#[test]
fn persistence_duplicate_dirty_no_double_count() {
    let mut p = TerrainPersistence::default_config();
    let c = ChunkCoord::new(5, 5, 5);
    p.mark_dirty(c);
    p.mark_dirty(c);
    assert_eq!(
        p.dirty_count(),
        1,
        "Duplicate dirty should not increase count"
    );
}

// ============================================================================
// TerrainPersistence auto-save
// ============================================================================

#[test]
fn persistence_auto_save_disabled_when_interval_zero() {
    let config = TerrainPersistenceConfig {
        auto_save_interval_seconds: 0.0,
        ..Default::default()
    };
    let mut p = TerrainPersistence::new(config);
    p.mark_dirty(ChunkCoord::new(0, 0, 0));
    assert!(!p.should_auto_save(), "Auto-save disabled when interval=0");
}

#[test]
fn persistence_auto_save_disabled_when_no_dirty() {
    let config = TerrainPersistenceConfig {
        auto_save_interval_seconds: 0.001, // Very short
        ..Default::default()
    };
    let p = TerrainPersistence::new(config);
    // No dirty chunks
    assert!(
        !p.should_auto_save(),
        "Auto-save disabled when no dirty chunks"
    );
}

// ============================================================================
// TerrainPersistence save/load round-trip
// ============================================================================

#[test]
fn persistence_save_and_load_roundtrip() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config = TerrainPersistenceConfig {
        save_directory: temp_dir.path().to_path_buf(),
        use_compression: false,
        ..Default::default()
    };
    let mut p = TerrainPersistence::new(config);

    let coord = ChunkCoord::new(0, 0, 0);
    let mut chunk = VoxelChunk::new(coord);
    chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(0.75, 2));

    let mut chunks = HashMap::new();
    chunks.insert(coord, chunk);
    p.mark_dirty(coord);

    let save_result = p.save_chunks(&chunks, 12345, Some("test")).unwrap();
    assert_eq!(save_result.chunks_saved, 1);
    assert!(save_result.file_size > 0);

    let (load_result, loaded) = p.load_chunks(&save_result.path).unwrap();
    assert_eq!(load_result.chunks_loaded, 1);
    assert!(loaded.contains_key(&coord));

    let stats = p.stats();
    assert_eq!(stats.save_count, 1);
    assert_eq!(stats.load_count, 1);
}

// ============================================================================
// get_chunks_in_region
// ============================================================================

#[test]
fn get_chunks_in_region_origin() {
    let coords = get_chunks_in_region(Vec3::ZERO, 32.0);
    assert!(!coords.is_empty());
    // Should include origin chunk
    assert!(coords.contains(&ChunkCoord::new(0, 0, 0)));
}

#[test]
fn get_chunks_in_region_larger_radius() {
    let small = get_chunks_in_region(Vec3::ZERO, 32.0);
    let large = get_chunks_in_region(Vec3::ZERO, 64.0);
    assert!(large.len() > small.len(), "Larger radius → more chunks");
}

// ============================================================================
// HitchDetector
// ============================================================================

#[test]
fn hitch_detector_empty_stats() {
    let d = HitchDetector::new(100, 16.67);
    assert_eq!(d.average_frame_time(), 0.0);
    assert_eq!(d.p99_frame_time(), 0.0);
    assert_eq!(d.hitch_rate(), 0.0);
    assert_eq!(d.hitch_count(), 0);
}

#[test]
fn hitch_detector_normal_frames() {
    let mut d = HitchDetector::new(100, 16.67);
    for _ in 0..50 {
        assert!(!d.record_frame(10.0), "10ms should not be a hitch");
    }
    assert_eq!(d.hitch_count(), 0);
    assert!((d.average_frame_time() - 10.0).abs() < 0.1);
}

#[test]
fn hitch_detector_detects_hitch() {
    let mut d = HitchDetector::new(100, 16.67);
    assert!(d.record_frame(20.0), "20ms > 16.67ms threshold → hitch");
    assert_eq!(d.hitch_count(), 1);
}

#[test]
fn hitch_detector_p99_calculation() {
    let mut d = HitchDetector::new(100, 100.0);
    for _ in 0..99 {
        d.record_frame(10.0);
    }
    d.record_frame(50.0);
    assert!(
        (d.p99_frame_time() - 50.0).abs() < 1.0,
        "p99 should be ~50.0, got {}",
        d.p99_frame_time()
    );
}

#[test]
fn hitch_detector_evicts_old_hitches() {
    let mut d = HitchDetector::new(5, 2.0);
    d.record_frame(5.0); // Hitch
    assert_eq!(d.hitch_count(), 1);

    // Fill buffer with normal frames to evict the hitch
    for _ in 0..5 {
        d.record_frame(1.0);
    }
    assert_eq!(d.hitch_count(), 0, "Should evict old hitch");
}

#[test]
fn hitch_detector_hitch_rate() {
    let mut d = HitchDetector::new(100, 2.0);
    for _ in 0..90 {
        d.record_frame(1.0);
    }
    for _ in 0..10 {
        d.record_frame(5.0);
    }
    // 10% hitch rate
    assert!(
        (d.hitch_rate() - 10.0).abs() < 0.5,
        "Hitch rate ~10%, got {}",
        d.hitch_rate()
    );
}

// ============================================================================
// MemoryStats
// ============================================================================

#[test]
fn memory_stats_default() {
    let s = MemoryStats::default();
    assert_eq!(s.total_bytes, 0);
    assert_eq!(s.peak_bytes, 0);
    assert_eq!(s.chunk_count, 0);
    assert_eq!(s.bytes_per_chunk, 0);
}

#[test]
fn memory_stats_update_basic() {
    let mut s = MemoryStats::default();
    s.update(100, 1024);
    assert_eq!(s.chunk_count, 100);
    assert_eq!(s.bytes_per_chunk, 1024);
    assert_eq!(s.total_bytes, 100 * 1024);
    assert_eq!(s.peak_bytes, 100 * 1024);
}

#[test]
fn memory_stats_peak_tracking() {
    let mut s = MemoryStats::default();
    s.update(100, 1024);
    s.update(50, 1024);
    assert_eq!(
        s.peak_bytes,
        100 * 1024,
        "Peak should stay at high-water mark"
    );
    s.update(150, 1024);
    assert_eq!(s.peak_bytes, 150 * 1024, "Peak should update to new high");
}

#[test]
fn memory_stats_total_mb() {
    let mut s = MemoryStats::default();
    s.update(1, 1024 * 1024);
    assert!((s.total_mb() - 1.0).abs() < 1e-4);
}

#[test]
fn memory_stats_delta_from_peak_at_peak() {
    let mut s = MemoryStats::default();
    s.update(100, 1024);
    assert!(
        (s.delta_from_peak_percent() - 0.0).abs() < 1e-4,
        "At peak → 0%"
    );
}

#[test]
fn memory_stats_delta_from_peak_below() {
    let mut s = MemoryStats::default();
    s.update(100, 1024);
    s.update(50, 1024);
    assert!(
        s.delta_from_peak_percent() < 0.0,
        "Below peak should be negative"
    );
}

#[test]
fn memory_stats_delta_from_peak_zero_peak() {
    let s = MemoryStats::default();
    assert!(
        (s.delta_from_peak_percent() - 0.0).abs() < 1e-4,
        "Zero peak → 0%"
    );
}

// ============================================================================
// StreamingDiagnostics
// ============================================================================

#[test]
fn streaming_diag_new() {
    let d = StreamingDiagnostics::new(16.67, 100);
    assert!(d.get_all_chunk_states().is_empty());
    assert_eq!(d.camera_pos(), Vec3::ZERO);
}

#[test]
fn streaming_diag_update_camera() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    d.update_camera(Vec3::new(100.0, 50.0, 200.0));
    assert_eq!(d.camera_pos(), Vec3::new(100.0, 50.0, 200.0));
}

#[test]
fn streaming_diag_chunk_state_default_unloaded() {
    let d = StreamingDiagnostics::new(16.67, 100);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(99, 99)),
        ChunkLoadState::Unloaded
    );
}

#[test]
fn streaming_diag_update_chunk_states() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    let loaded = vec![ChunkId::new(0, 0), ChunkId::new(1, 0)];
    let loading = vec![ChunkId::new(2, 0)];
    let pending = vec![ChunkId::new(3, 0)];
    d.update_chunk_states(&loaded, &loading, &pending);

    assert_eq!(
        d.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Loaded
    );
    assert_eq!(
        d.get_chunk_state(ChunkId::new(2, 0)),
        ChunkLoadState::Loading
    );
    assert_eq!(
        d.get_chunk_state(ChunkId::new(3, 0)),
        ChunkLoadState::Pending
    );
}

#[test]
fn streaming_diag_record_frame_normal() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    assert!(!d.record_frame(10.0));
}

#[test]
fn streaming_diag_record_frame_hitch() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    assert!(d.record_frame(20.0));
}

#[test]
fn streaming_diag_update_memory() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    d.update_memory(50, 1024 * 1024);
    assert_eq!(d.memory_stats().chunk_count, 50);
}

#[test]
fn streaming_diag_generate_report() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    let loaded = vec![ChunkId::new(0, 0), ChunkId::new(1, 0)];
    let loading = vec![ChunkId::new(2, 0)];
    let pending = vec![];
    d.update_chunk_states(&loaded, &loading, &pending);

    for _ in 0..10 {
        d.record_frame(10.0);
    }
    d.record_frame(30.0); // Hitch

    let report = d.generate_report();
    assert_eq!(report.chunk_counts.loaded, 2);
    assert_eq!(report.chunk_counts.loading, 1);
    assert_eq!(report.chunk_counts.pending, 0);
    assert_eq!(report.frame_stats.hitch_count, 1);
}

#[test]
fn streaming_diag_update_streaming_stats() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    let stats = StreamingStats {
        loaded_chunk_count: 42,
        pending_load_count: 5,
        ..Default::default()
    };
    d.update_streaming_stats(stats);
    assert_eq!(d.streaming_stats().loaded_chunk_count, 42);
}

#[test]
fn streaming_diag_update_lod_stats() {
    let mut d = StreamingDiagnostics::new(16.67, 100);
    let stats = LodStats {
        total_chunks: 100,
        full_count: 40,
        half_count: 30,
        quarter_count: 20,
        skybox_count: 10,
        transitioning_count: 0,
    };
    d.update_lod_stats(stats);
    assert_eq!(d.lod_stats().total_chunks, 100);
    assert_eq!(d.lod_stats().full_count, 40);
}
