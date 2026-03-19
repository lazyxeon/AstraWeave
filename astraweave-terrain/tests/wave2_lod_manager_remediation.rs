//! Wave 2 LOD Manager Remediation Tests
//!
//! Targets mutation-sensitive code in lod_manager.rs:
//! - LodLevel skip_factor, lower, higher
//! - LodConfig::get_threshold hysteresis math
//! - LodManager update_chunk_lod: distance-based LOD selection + hysteresis
//! - LodManager caching: get_cached_mesh, cache_mesh, hit rate, memory, eviction
//! - LodManager update_all_chunks state retention/cleanup
//! - LodManager stats aggregation
//! - ChunkLodCache: get_mesh / set_mesh / has_mesh for all levels

use astraweave_terrain::lod_manager::*;
use astraweave_terrain::meshing::{ChunkMesh, MeshVertex};
use astraweave_terrain::voxel_data::ChunkCoord;
use astraweave_terrain::ChunkId;
use glam::Vec3;
use std::sync::Arc;

// ============================================================================
// Helper
// ============================================================================
fn simple_mesh() -> ChunkMesh {
    ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices: vec![
            MeshVertex {
                position: Vec3::ZERO,
                normal: Vec3::Y,
                material: 1,
            },
            MeshVertex {
                position: Vec3::X,
                normal: Vec3::Y,
                material: 1,
            },
            MeshVertex {
                position: Vec3::Y,
                normal: Vec3::Y,
                material: 1,
            },
        ],
        indices: vec![0, 1, 2],
    }
}

// ============================================================================
// A. LodLevel skip factors are distinct powers of 2
// ============================================================================

#[test]
fn skip_factor_full_is_1() {
    assert_eq!(LodLevel::Full.skip_factor(), 1);
}

#[test]
fn skip_factor_half_is_2() {
    assert_eq!(LodLevel::Half.skip_factor(), 2);
}

#[test]
fn skip_factor_quarter_is_4() {
    assert_eq!(LodLevel::Quarter.skip_factor(), 4);
}

#[test]
fn skip_factor_skybox_is_16() {
    assert_eq!(LodLevel::Skybox.skip_factor(), 16);
}

// ============================================================================
// B. LodLevel lower() chain
// ============================================================================

#[test]
fn lower_full_is_half() {
    assert_eq!(LodLevel::Full.lower(), Some(LodLevel::Half));
}

#[test]
fn lower_half_is_quarter() {
    assert_eq!(LodLevel::Half.lower(), Some(LodLevel::Quarter));
}

#[test]
fn lower_quarter_is_skybox() {
    assert_eq!(LodLevel::Quarter.lower(), Some(LodLevel::Skybox));
}

#[test]
fn lower_skybox_is_none() {
    assert_eq!(LodLevel::Skybox.lower(), None);
}

// ============================================================================
// C. LodLevel higher() chain
// ============================================================================

#[test]
fn higher_skybox_is_quarter() {
    assert_eq!(LodLevel::Skybox.higher(), Some(LodLevel::Quarter));
}

#[test]
fn higher_quarter_is_half() {
    assert_eq!(LodLevel::Quarter.higher(), Some(LodLevel::Half));
}

#[test]
fn higher_half_is_full() {
    assert_eq!(LodLevel::Half.higher(), Some(LodLevel::Full));
}

#[test]
fn higher_full_is_none() {
    assert_eq!(LodLevel::Full.higher(), None);
}

// ============================================================================
// D. LodConfig defaults
// ============================================================================

#[test]
fn lod_config_default_thresholds() {
    let c = LodConfig::default();
    assert_eq!(c.distance_thresholds, [256.0, 512.0, 1024.0]);
}

#[test]
fn lod_config_default_hysteresis() {
    let c = LodConfig::default();
    assert_eq!(c.hysteresis_margin, 0.1);
}

#[test]
fn lod_config_default_blend_zone() {
    let c = LodConfig::default();
    assert_eq!(c.blend_zone_size, 32.0);
}

#[test]
fn lod_config_default_blending_enabled() {
    let c = LodConfig::default();
    assert!(c.enable_blending);
}

// ============================================================================
// E. LodConfig::get_threshold hysteresis math
// ============================================================================

#[test]
fn get_threshold_full_half_increasing_detail_subtracts_margin() {
    let c = LodConfig::default();
    // base = 256, increasing_detail → 256 * (1 - 0.1) = 230.4
    let t = c.get_threshold(LodLevel::Full, LodLevel::Half, true);
    assert!((t - 230.4).abs() < 0.1);
}

#[test]
fn get_threshold_full_half_decreasing_detail_adds_margin() {
    let c = LodConfig::default();
    // base = 256, decreasing → 256 * (1 + 0.1) = 281.6
    let t = c.get_threshold(LodLevel::Full, LodLevel::Half, false);
    assert!((t - 281.6).abs() < 0.1);
}

#[test]
fn get_threshold_half_quarter_symmetric() {
    let c = LodConfig::default();
    // base = 512
    let down = c.get_threshold(LodLevel::Half, LodLevel::Quarter, false); // 512 * 1.1 = 563.2
    let up = c.get_threshold(LodLevel::Quarter, LodLevel::Half, true); // 512 * 0.9 = 460.8
    assert!((down - 563.2).abs() < 0.1);
    assert!((up - 460.8).abs() < 0.1);
    assert!(
        down > up,
        "decreasing threshold should be greater than increasing"
    );
}

#[test]
fn get_threshold_quarter_skybox() {
    let c = LodConfig::default();
    // base = 1024
    let down = c.get_threshold(LodLevel::Quarter, LodLevel::Skybox, false); // 1024 * 1.1 = 1126.4
    let up = c.get_threshold(LodLevel::Skybox, LodLevel::Quarter, true); // 1024 * 0.9 = 921.6
    assert!((down - 1126.4).abs() < 0.1);
    assert!((up - 921.6).abs() < 0.1);
}

#[test]
fn get_threshold_invalid_transition_returns_max() {
    let c = LodConfig::default();
    assert_eq!(
        c.get_threshold(LodLevel::Full, LodLevel::Skybox, true),
        f32::MAX
    );
    assert_eq!(
        c.get_threshold(LodLevel::Full, LodLevel::Quarter, false),
        f32::MAX
    );
}

// ============================================================================
// F. LodManager: update_chunk_lod distance-based selection
// ============================================================================

#[test]
fn update_chunk_lod_close_stays_full() {
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: false,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(0, 0);
    let center = chunk.to_center_pos(256.0);
    mgr.update_chunk_lod(chunk, center);
    assert_eq!(mgr.get_chunk_lod(chunk), Some(LodLevel::Full));
}

#[test]
fn update_chunk_lod_far_downgrades_to_half() {
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: false,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(0, 0);
    let center = chunk.to_center_pos(256.0);
    // Initialize at center
    mgr.update_chunk_lod(chunk, center);
    // Move far away (beyond 256 * 1.1 = 281.6)
    let far = center + Vec3::new(300.0, 0.0, 0.0);
    mgr.update_chunk_lod(chunk, far);
    assert_eq!(mgr.get_chunk_lod(chunk), Some(LodLevel::Half));
}

// ============================================================================
// G. LodManager: cache operations
// ============================================================================

#[test]
fn cache_mesh_then_retrieve() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(1, 1);
    let mesh = Arc::new(simple_mesh());
    mgr.cache_mesh(chunk, LodLevel::Full, mesh.clone());
    let retrieved = mgr.get_cached_mesh(chunk, LodLevel::Full);
    assert!(retrieved.is_some());
}

#[test]
fn cache_miss_returns_none() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    assert!(mgr
        .get_cached_mesh(ChunkId::new(99, 99), LodLevel::Full)
        .is_none());
}

#[test]
fn cache_hit_rate_after_operations() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(1, 1);
    let mesh = Arc::new(simple_mesh());

    // 1 miss
    mgr.get_cached_mesh(chunk, LodLevel::Full);
    // cache it
    mgr.cache_mesh(chunk, LodLevel::Full, mesh);
    // 1 hit
    mgr.get_cached_mesh(chunk, LodLevel::Full);

    // hit_rate = 1 / (1 + 1) = 0.5
    assert!((mgr.cache_hit_rate() - 0.5).abs() < 1e-6);
}

#[test]
fn cache_hit_rate_empty_is_zero() {
    let cfg = LodConfig::default();
    let mgr = LodManager::new(cfg, 256.0);
    assert_eq!(mgr.cache_hit_rate(), 0.0);
}

#[test]
fn cache_memory_empty_is_zero() {
    let cfg = LodConfig::default();
    let mgr = LodManager::new(cfg, 256.0);
    assert_eq!(mgr.cache_memory_usage(), 0);
}

#[test]
fn cache_memory_after_caching() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    let mesh = Arc::new(simple_mesh());
    mgr.cache_mesh(ChunkId::new(0, 0), LodLevel::Full, mesh);
    assert!(
        mgr.cache_memory_usage() > 0,
        "should have non-zero memory after caching"
    );
}

// ============================================================================
// H. LodManager: evict_distant_cache
// ============================================================================

#[test]
fn evict_distant_cache_empty_returns_zero() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    assert_eq!(mgr.evict_distant_cache(Vec3::ZERO, 1000.0), 0);
}

#[test]
fn evict_distant_cache_removes_far_chunks() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    let mesh = Arc::new(simple_mesh());

    // Cache chunk at (0,0) and (100,100)
    mgr.cache_mesh(ChunkId::new(0, 0), LodLevel::Full, mesh.clone());
    mgr.cache_mesh(ChunkId::new(100, 100), LodLevel::Full, mesh);

    // Evict with small radius — far chunk should be evicted
    let evicted = mgr.evict_distant_cache(Vec3::ZERO, 500.0);
    assert!(evicted >= 1, "should evict at least the distant chunk");
}

// ============================================================================
// I. LodManager: update_all_chunks
// ============================================================================

#[test]
fn update_all_chunks_empty_returns_zero() {
    let cfg = LodConfig::default();
    let mut mgr = LodManager::new(cfg, 256.0);
    assert_eq!(mgr.update_all_chunks(&[], Vec3::ZERO), 0);
}

#[test]
fn update_all_chunks_initializes_states() {
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: false,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunks = vec![ChunkId::new(0, 0), ChunkId::new(1, 0)];
    mgr.update_all_chunks(&chunks, Vec3::ZERO);
    let stats = mgr.get_stats();
    assert_eq!(stats.total_chunks, 2);
}

#[test]
fn update_all_chunks_removes_unloaded_states() {
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: false,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunks = vec![ChunkId::new(0, 0), ChunkId::new(1, 0)];
    mgr.update_all_chunks(&chunks, Vec3::ZERO);
    assert_eq!(mgr.get_stats().total_chunks, 2);

    // Update with only one chunk — the other should be removed
    mgr.update_all_chunks(&[ChunkId::new(0, 0)], Vec3::ZERO);
    assert_eq!(mgr.get_stats().total_chunks, 1);
}

// ============================================================================
// J. LodManager: query methods for unknown chunks
// ============================================================================

#[test]
fn get_chunk_state_unknown_is_none() {
    let mgr = LodManager::new(LodConfig::default(), 256.0);
    assert!(mgr.get_chunk_state(ChunkId::new(999, 999)).is_none());
}

#[test]
fn get_chunk_lod_unknown_is_none() {
    let mgr = LodManager::new(LodConfig::default(), 256.0);
    assert!(mgr.get_chunk_lod(ChunkId::new(999, 999)).is_none());
}

#[test]
fn is_transitioning_unknown_is_false() {
    let mgr = LodManager::new(LodConfig::default(), 256.0);
    assert!(!mgr.is_transitioning(ChunkId::new(999, 999)));
}

#[test]
fn get_blend_factor_unknown_is_zero() {
    let mgr = LodManager::new(LodConfig::default(), 256.0);
    assert_eq!(mgr.get_blend_factor(ChunkId::new(999, 999)), 0.0);
}

// ============================================================================
// K. LodStats default
// ============================================================================

#[test]
fn lod_stats_default_all_zero() {
    let s = LodStats::default();
    assert_eq!(s.total_chunks, 0);
    assert_eq!(s.full_count, 0);
    assert_eq!(s.half_count, 0);
    assert_eq!(s.quarter_count, 0);
    assert_eq!(s.skybox_count, 0);
    assert_eq!(s.transitioning_count, 0);
}

#[test]
fn get_stats_empty_manager() {
    let mgr = LodManager::new(LodConfig::default(), 256.0);
    let s = mgr.get_stats();
    assert_eq!(s.total_chunks, 0);
    assert_eq!(s.full_count, 0);
}

// ============================================================================
// L. ChunkLodCache
// ============================================================================

#[test]
fn chunk_lod_cache_new_all_none() {
    let c = ChunkLodCache::new();
    assert!(!c.has_mesh(LodLevel::Full));
    assert!(!c.has_mesh(LodLevel::Half));
    assert!(!c.has_mesh(LodLevel::Quarter));
    assert!(!c.has_mesh(LodLevel::Skybox));
}

#[test]
fn chunk_lod_cache_set_and_get_full() {
    let mut c = ChunkLodCache::new();
    let mesh = Arc::new(simple_mesh());
    c.set_mesh(LodLevel::Full, mesh.clone());
    assert!(c.has_mesh(LodLevel::Full));
    assert!(c.get_mesh(LodLevel::Full).is_some());
    assert!(!c.has_mesh(LodLevel::Half));
}

#[test]
fn chunk_lod_cache_set_and_get_half() {
    let mut c = ChunkLodCache::new();
    let mesh = Arc::new(simple_mesh());
    c.set_mesh(LodLevel::Half, mesh);
    assert!(c.has_mesh(LodLevel::Half));
    assert!(c.get_mesh(LodLevel::Half).is_some());
}

#[test]
fn chunk_lod_cache_set_and_get_quarter() {
    let mut c = ChunkLodCache::new();
    let mesh = Arc::new(simple_mesh());
    c.set_mesh(LodLevel::Quarter, mesh);
    assert!(c.has_mesh(LodLevel::Quarter));
}

#[test]
fn chunk_lod_cache_set_and_get_skybox() {
    let mut c = ChunkLodCache::new();
    let mesh = Arc::new(simple_mesh());
    c.set_mesh(LodLevel::Skybox, mesh);
    assert!(c.has_mesh(LodLevel::Skybox));
}

#[test]
fn chunk_lod_cache_memory_zero_when_empty() {
    let c = ChunkLodCache::new();
    assert_eq!(c.memory_usage(), 0);
}

#[test]
fn chunk_lod_cache_memory_nonzero_with_mesh() {
    let mut c = ChunkLodCache::new();
    c.set_mesh(LodLevel::Full, Arc::new(simple_mesh()));
    assert!(c.memory_usage() > 0);
}

// ============================================================================
// M. LodManager with blending: transition starts (target_lod set)
// ============================================================================

#[test]
fn update_chunk_lod_with_blending_begins_transition() {
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: true,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(0, 0);
    let center = chunk.to_center_pos(256.0);

    // Initialize at center → Full
    mgr.update_chunk_lod(chunk, center);
    assert_eq!(mgr.get_chunk_lod(chunk), Some(LodLevel::Full));
    assert!(!mgr.is_transitioning(chunk));

    // Move far away → should start blend transition
    let far = center + Vec3::new(300.0, 0.0, 0.0);
    mgr.update_chunk_lod(chunk, far);
    // With blending enabled, current LOD stays Full but target changes
    // (it starts transitioning)
    assert!(
        mgr.is_transitioning(chunk),
        "should be transitioning with blending enabled"
    );
}

#[test]
fn update_chunk_lod_transition_resets_on_repeated_calls() {
    // Each update_chunk_lod call that passes hysteresis resets blend_factor to 0.0
    // and returns true before the blend advancement code runs
    let cfg = LodConfig {
        distance_thresholds: [256.0, 512.0, 1024.0],
        hysteresis_margin: 0.1,
        blend_zone_size: 32.0,
        enable_blending: true,
    };
    let mut mgr = LodManager::new(cfg, 256.0);
    let chunk = ChunkId::new(0, 0);
    let center = chunk.to_center_pos(256.0);

    mgr.update_chunk_lod(chunk, center);
    let far = center + Vec3::new(300.0, 0.0, 0.0);
    mgr.update_chunk_lod(chunk, far);

    // Blend factor should start at 0.0 (just initiated transition)
    assert_eq!(mgr.get_blend_factor(chunk), 0.0);
    assert!(mgr.is_transitioning(chunk));
}
