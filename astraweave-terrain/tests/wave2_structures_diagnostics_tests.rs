//! Wave 2 Mutation Remediation Tests — structures, streaming_diagnostics, lod_blending
//!
//! Pins exact config defaults, enum match arms, size/rarity/spacing tables,
//! diagnostic arithmetic, and morph factor calculations.

use astraweave_terrain::meshing::ChunkMesh;
use astraweave_terrain::voxel_data::ChunkCoord;
use astraweave_terrain::*;

// ============================================================================
// StructureType: biome → allowed structures mapping
// ============================================================================

#[test]
fn structure_type_for_grassland_contains_cottage() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert!(types.contains(&StructureType::Cottage));
}

#[test]
fn structure_type_for_grassland_contains_farmhouse() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert!(types.contains(&StructureType::Farmhouse));
}

#[test]
fn structure_type_for_grassland_contains_windmill() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert!(types.contains(&StructureType::Windmill));
}

#[test]
fn structure_type_for_grassland_excludes_lighthouse() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert!(!types.contains(&StructureType::Lighthouse));
}

#[test]
fn structure_type_for_desert_contains_obelisk() {
    let types = StructureType::for_biome(BiomeType::Desert);
    assert!(types.contains(&StructureType::Obelisk));
}

#[test]
fn structure_type_for_desert_contains_tomb() {
    let types = StructureType::for_biome(BiomeType::Desert);
    assert!(types.contains(&StructureType::Tomb));
}

#[test]
fn structure_type_for_desert_excludes_cabin() {
    let types = StructureType::for_biome(BiomeType::Desert);
    assert!(!types.contains(&StructureType::Cabin));
}

#[test]
fn structure_type_for_forest_contains_cabin() {
    let types = StructureType::for_biome(BiomeType::Forest);
    assert!(types.contains(&StructureType::Cabin));
}

#[test]
fn structure_type_for_forest_contains_stone_circle() {
    let types = StructureType::for_biome(BiomeType::Forest);
    assert!(types.contains(&StructureType::StoneCircle));
}

#[test]
fn structure_type_for_mountain_contains_cave() {
    let types = StructureType::for_biome(BiomeType::Mountain);
    assert!(types.contains(&StructureType::Cave));
}

#[test]
fn structure_type_for_mountain_contains_crystal_formation() {
    let types = StructureType::for_biome(BiomeType::Mountain);
    assert!(types.contains(&StructureType::CrystalFormation));
}

#[test]
fn structure_type_for_tundra_contains_fort() {
    let types = StructureType::for_biome(BiomeType::Tundra);
    assert!(types.contains(&StructureType::Fort));
}

#[test]
fn structure_type_for_swamp_contains_bridge() {
    let types = StructureType::for_biome(BiomeType::Swamp);
    assert!(types.contains(&StructureType::Bridge));
}

#[test]
fn structure_type_for_beach_contains_lighthouse() {
    let types = StructureType::for_biome(BiomeType::Beach);
    assert!(types.contains(&StructureType::Lighthouse));
}

#[test]
fn structure_type_for_river_contains_bridge() {
    let types = StructureType::for_biome(BiomeType::River);
    assert!(types.contains(&StructureType::Bridge));
}

#[test]
fn structure_type_for_river_contains_windmill() {
    let types = StructureType::for_biome(BiomeType::River);
    assert!(types.contains(&StructureType::Windmill));
}

// ============================================================================
// StructureType: typical_size exact values
// ============================================================================

#[test]
fn structure_typical_size_cottage() {
    assert!((StructureType::Cottage.typical_size() - 8.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_cabin() {
    assert!((StructureType::Cabin.typical_size() - 8.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_farmhouse() {
    assert!((StructureType::Farmhouse.typical_size() - 12.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_villa() {
    assert!((StructureType::Villa.typical_size() - 12.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_tavern() {
    assert!((StructureType::Tavern.typical_size() - 10.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_temple() {
    assert!((StructureType::Temple.typical_size() - 15.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_fort() {
    assert!((StructureType::Fort.typical_size() - 20.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_obelisk() {
    assert!((StructureType::Obelisk.typical_size() - 3.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_well() {
    assert!((StructureType::Well.typical_size() - 2.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_watchtower() {
    assert!((StructureType::Watchtower.typical_size() - 6.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_bridge() {
    assert!((StructureType::Bridge.typical_size() - 15.0).abs() < 1e-6);
}

#[test]
fn structure_typical_size_lighthouse() {
    assert!((StructureType::Lighthouse.typical_size() - 6.0).abs() < 1e-6);
}

// ============================================================================
// StructureType: rarity exact values
// ============================================================================

#[test]
fn structure_rarity_cottage() {
    assert!((StructureType::Cottage.rarity() - 0.8).abs() < 1e-6);
}

#[test]
fn structure_rarity_farmhouse() {
    assert!((StructureType::Farmhouse.rarity() - 0.8).abs() < 1e-6);
}

#[test]
fn structure_rarity_rock_formation() {
    assert!((StructureType::RockFormation.rarity() - 0.8).abs() < 1e-6);
}

#[test]
fn structure_rarity_cabin() {
    assert!((StructureType::Cabin.rarity() - 0.6).abs() < 1e-6);
}

#[test]
fn structure_rarity_tavern() {
    assert!((StructureType::Tavern.rarity() - 0.6).abs() < 1e-6);
}

#[test]
fn structure_rarity_villa() {
    assert!((StructureType::Villa.rarity() - 0.4).abs() < 1e-6);
}

#[test]
fn structure_rarity_fort() {
    assert!((StructureType::Fort.rarity() - 0.2).abs() < 1e-6);
}

#[test]
fn structure_rarity_ancient_ruin() {
    assert!((StructureType::AncientRuin.rarity() - 0.2).abs() < 1e-6);
}

#[test]
fn structure_rarity_crystal_formation() {
    assert!((StructureType::CrystalFormation.rarity() - 0.1).abs() < 1e-6);
}

#[test]
fn structure_rarity_obelisk() {
    assert!((StructureType::Obelisk.rarity() - 0.1).abs() < 1e-6);
}

// ============================================================================
// StructureType: can_place_on_slope exact thresholds
// ============================================================================

#[test]
fn structure_slope_farmhouse_flat_only() {
    assert!(StructureType::Farmhouse.can_place_on_slope(0.1));
    assert!(!StructureType::Farmhouse.can_place_on_slope(0.11));
}

#[test]
fn structure_slope_cottage_gentle() {
    assert!(StructureType::Cottage.can_place_on_slope(0.2));
    assert!(!StructureType::Cottage.can_place_on_slope(0.21));
}

#[test]
fn structure_slope_cabin_moderate() {
    assert!(StructureType::Cabin.can_place_on_slope(0.4));
    assert!(!StructureType::Cabin.can_place_on_slope(0.41));
}

#[test]
fn structure_slope_cave_steep() {
    assert!(StructureType::Cave.can_place_on_slope(0.8));
    assert!(!StructureType::Cave.can_place_on_slope(0.81));
}

#[test]
fn structure_slope_wall_any() {
    assert!(StructureType::Wall.can_place_on_slope(1.0));
}

// ============================================================================
// StructureType: min_spacing exact values
// ============================================================================

#[test]
fn structure_spacing_fort() {
    assert!((StructureType::Fort.min_spacing() - 100.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_temple() {
    assert!((StructureType::Temple.min_spacing() - 100.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_tavern() {
    assert!((StructureType::Tavern.min_spacing() - 50.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_cottage() {
    assert!((StructureType::Cottage.min_spacing() - 30.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_well() {
    assert!((StructureType::Well.min_spacing() - 20.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_cave() {
    assert!((StructureType::Cave.min_spacing() - 15.0).abs() < 1e-6);
}

#[test]
fn structure_spacing_windmill() {
    assert!((StructureType::Windmill.min_spacing() - 10.0).abs() < 1e-6);
}

// ============================================================================
// StructureConfig: exact default values
// ============================================================================

#[test]
fn structure_config_default_density() {
    let c = StructureConfig::default();
    assert!((c.density - 0.3).abs() < 1e-6);
}

#[test]
fn structure_config_default_edge_buffer() {
    let c = StructureConfig::default();
    assert!((c.edge_buffer - 20.0).abs() < 1e-6);
}

#[test]
fn structure_config_default_height_tolerance() {
    let c = StructureConfig::default();
    assert!((c.height_tolerance - 2.0).abs() < 1e-6);
}

#[test]
fn structure_config_default_include_ancient() {
    let c = StructureConfig::default();
    assert!(c.include_ancient);
}

#[test]
fn structure_config_default_include_defensive() {
    let c = StructureConfig::default();
    assert!(c.include_defensive);
}

#[test]
fn structure_config_default_seed() {
    let c = StructureConfig::default();
    assert_eq!(c.seed, 0);
}

// ============================================================================
// HitchDetector: arithmetic tests
// ============================================================================

#[test]
fn hitch_detector_initial_no_hitches() {
    let h = HitchDetector::new(100, 16.6);
    assert_eq!(h.hitch_count(), 0);
}

#[test]
fn hitch_detector_records_hitch_above_threshold() {
    let mut h = HitchDetector::new(100, 16.6);
    let is_hitch = h.record_frame(20.0);
    assert!(is_hitch);
    assert_eq!(h.hitch_count(), 1);
}

#[test]
fn hitch_detector_no_hitch_below_threshold() {
    let mut h = HitchDetector::new(100, 16.6);
    let is_hitch = h.record_frame(10.0);
    assert!(!is_hitch);
    assert_eq!(h.hitch_count(), 0);
}

#[test]
fn hitch_detector_average_frame_time_single() {
    let mut h = HitchDetector::new(100, 16.6);
    h.record_frame(10.0);
    assert!((h.average_frame_time() - 10.0).abs() < 1e-6);
}

#[test]
fn hitch_detector_average_frame_time_multiple() {
    let mut h = HitchDetector::new(100, 16.6);
    h.record_frame(10.0);
    h.record_frame(20.0);
    assert!((h.average_frame_time() - 15.0).abs() < 1e-6);
}

#[test]
fn hitch_detector_empty_average_is_zero() {
    let h = HitchDetector::new(100, 16.6);
    assert!((h.average_frame_time() - 0.0).abs() < 1e-6);
}

#[test]
fn hitch_detector_empty_p99_is_zero() {
    let h = HitchDetector::new(100, 16.6);
    assert!((h.p99_frame_time() - 0.0).abs() < 1e-6);
}

#[test]
fn hitch_detector_hitch_rate_50_percent() {
    let mut h = HitchDetector::new(100, 16.6);
    h.record_frame(10.0); // not hitch
    h.record_frame(20.0); // hitch
    assert!((h.hitch_rate() - 50.0).abs() < 1e-6);
}

#[test]
fn hitch_detector_evicts_oldest_after_max() {
    let mut h = HitchDetector::new(3, 16.6);
    h.record_frame(20.0); // hitch, count=1
    h.record_frame(10.0); // not hitch, count=1
    h.record_frame(10.0); // not hitch, count=1
    h.record_frame(10.0); // evicts 20.0 (hitch), count=0
    assert_eq!(h.hitch_count(), 0);
}

#[test]
fn hitch_detector_empty_hitch_rate_zero() {
    let h = HitchDetector::new(100, 16.6);
    assert!((h.hitch_rate() - 0.0).abs() < 1e-6);
}

// ============================================================================
// MemoryStats: arithmetic tests
// ============================================================================

#[test]
fn memory_stats_default_all_zero() {
    let m = MemoryStats::default();
    assert_eq!(m.total_bytes, 0);
    assert_eq!(m.bytes_per_chunk, 0);
    assert_eq!(m.chunk_count, 0);
    assert_eq!(m.peak_bytes, 0);
}

#[test]
fn memory_stats_update_basic() {
    let mut m = MemoryStats::default();
    m.update(10, 1024);
    assert_eq!(m.chunk_count, 10);
    assert_eq!(m.bytes_per_chunk, 1024);
    assert_eq!(m.total_bytes, 10240);
    assert_eq!(m.peak_bytes, 10240);
}

#[test]
fn memory_stats_peak_tracks_high_water() {
    let mut m = MemoryStats::default();
    m.update(10, 1024);
    m.update(5, 1024);
    assert_eq!(m.total_bytes, 5120);
    assert_eq!(m.peak_bytes, 10240);
}

#[test]
fn memory_stats_total_mb() {
    let mut m = MemoryStats::default();
    m.update(1024, 1024);
    assert!((m.total_mb() - 1.0).abs() < 1e-6);
}

#[test]
fn memory_stats_delta_from_peak_at_peak() {
    let mut m = MemoryStats::default();
    m.update(10, 1024);
    assert!((m.delta_from_peak_percent() - 0.0).abs() < 1e-6);
}

#[test]
fn memory_stats_delta_from_peak_below() {
    let mut m = MemoryStats::default();
    m.update(10, 1024); // peak = 10240
    m.update(5, 1024); // current = 5120
                       // (5120/10240 - 1.0) * 100 = -50.0
    assert!((m.delta_from_peak_percent() - (-50.0)).abs() < 1e-4);
}

#[test]
fn memory_stats_delta_from_peak_zero_divisor() {
    let m = MemoryStats::default();
    assert!((m.delta_from_peak_percent() - 0.0).abs() < 1e-6);
}

// ============================================================================
// MorphConfig: exact default values
// ============================================================================

#[test]
fn morph_config_default_morph_start() {
    let c = MorphConfig::default();
    assert!((c.morph_start - 0.0).abs() < 1e-6);
}

#[test]
fn morph_config_default_morph_end() {
    let c = MorphConfig::default();
    assert!((c.morph_end - 50.0).abs() < 1e-6);
}

#[test]
fn morph_config_default_search_radius() {
    let c = MorphConfig::default();
    assert!((c.search_radius - 2.0).abs() < 1e-6);
}

// ============================================================================
// MorphConfig::for_lod_transition exact math
// ============================================================================

#[test]
fn morph_config_for_lod_transition_end_matches() {
    let c = MorphConfig::for_lod_transition(100.0, 200.0);
    assert!((c.morph_end - 200.0).abs() < 1e-6);
}

#[test]
fn morph_config_for_lod_transition_start_is_80_percent() {
    // transition_zone = (200 - 100) * 0.2 = 20
    // morph_start = 200 - 20 = 180
    let c = MorphConfig::for_lod_transition(100.0, 200.0);
    assert!((c.morph_start - 180.0).abs() < 1e-6);
}

#[test]
fn morph_config_for_lod_transition_search_radius() {
    let c = MorphConfig::for_lod_transition(100.0, 200.0);
    assert!((c.search_radius - 2.0).abs() < 1e-6);
}

// ============================================================================
// LodBlender: compute_morph_factor boundary tests
// ============================================================================

#[test]
fn lod_blender_morph_factor_before_start() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((blender.compute_morph_factor(30.0) - 0.0).abs() < 1e-6);
}

#[test]
fn lod_blender_morph_factor_at_start() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((blender.compute_morph_factor(50.0) - 0.0).abs() < 1e-6);
}

#[test]
fn lod_blender_morph_factor_at_end() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((blender.compute_morph_factor(100.0) - 1.0).abs() < 1e-6);
}

#[test]
fn lod_blender_morph_factor_after_end() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((blender.compute_morph_factor(150.0) - 1.0).abs() < 1e-6);
}

#[test]
fn lod_blender_morph_factor_midpoint() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 50.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    // (75 - 50) / (100 - 50) = 25/50 = 0.5
    assert!((blender.compute_morph_factor(75.0) - 0.5).abs() < 1e-6);
}

#[test]
fn lod_blender_morph_factor_quarter() {
    let blender = LodBlender::new(MorphConfig {
        morph_start: 0.0,
        morph_end: 100.0,
        search_radius: 2.0,
    });
    assert!((blender.compute_morph_factor(25.0) - 0.25).abs() < 1e-6);
}

// ============================================================================
// StreamingDiagnostics: chunk state tracking
// ============================================================================

#[test]
fn diagnostics_new_no_chunk_states() {
    let d = StreamingDiagnostics::new(16.6, 100);
    assert!(d.get_all_chunk_states().is_empty());
}

#[test]
fn diagnostics_unknown_chunk_is_unloaded() {
    let d = StreamingDiagnostics::new(16.6, 100);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(99, 99)),
        ChunkLoadState::Unloaded
    );
}

#[test]
fn diagnostics_update_loaded_chunks() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let loaded = vec![ChunkId::new(0, 0), ChunkId::new(1, 0)];
    d.update_chunk_states(&loaded, &[], &[]);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Loaded
    );
    assert_eq!(
        d.get_chunk_state(ChunkId::new(1, 0)),
        ChunkLoadState::Loaded
    );
}

#[test]
fn diagnostics_update_loading_chunks() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let loading = vec![ChunkId::new(2, 2)];
    d.update_chunk_states(&[], &loading, &[]);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(2, 2)),
        ChunkLoadState::Loading
    );
}

#[test]
fn diagnostics_update_pending_chunks() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let pending = vec![ChunkId::new(5, 5)];
    d.update_chunk_states(&[], &[], &pending);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(5, 5)),
        ChunkLoadState::Pending
    );
}

#[test]
fn diagnostics_update_clears_old_states() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let loaded = vec![ChunkId::new(0, 0)];
    d.update_chunk_states(&loaded, &[], &[]);
    // Now update with empty → old state gone
    d.update_chunk_states(&[], &[], &[]);
    assert_eq!(
        d.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Unloaded
    );
}

#[test]
fn diagnostics_record_frame_detects_hitch() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let is_hitch = d.record_frame(20.0);
    assert!(is_hitch);
}

#[test]
fn diagnostics_record_frame_no_hitch() {
    let mut d = StreamingDiagnostics::new(16.6, 100);
    let is_hitch = d.record_frame(10.0);
    assert!(!is_hitch);
}

// ============================================================================
// MorphedMesh: basic construction and accessors
// ============================================================================

#[test]
fn morphed_mesh_initial_morph_factor_zero() {
    let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let morphed = MorphedMesh::new(mesh);
    assert!((morphed.morph_factor - 0.0).abs() < 1e-6);
}

#[test]
fn morphed_mesh_vertex_count_empty() {
    let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let morphed = MorphedMesh::new(mesh);
    assert_eq!(morphed.vertex_count(), 0);
}

#[test]
fn morphed_mesh_triangle_count_empty() {
    let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let morphed = MorphedMesh::new(mesh);
    assert_eq!(morphed.triangle_count(), 0);
}

#[test]
fn morphed_mesh_triangle_count_one_triangle() {
    let mut mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    mesh.indices = vec![0, 1, 2];
    let morphed = MorphedMesh::new(mesh);
    assert_eq!(morphed.triangle_count(), 1);
}

// ============================================================================
// ChunkLoadState enum variant equality
// ============================================================================

#[test]
fn chunk_load_state_loaded_eq() {
    assert_eq!(ChunkLoadState::Loaded, ChunkLoadState::Loaded);
}

#[test]
fn chunk_load_state_loading_ne_pending() {
    assert_ne!(ChunkLoadState::Loading, ChunkLoadState::Pending);
}

#[test]
fn chunk_load_state_unloaded_ne_loaded() {
    assert_ne!(ChunkLoadState::Unloaded, ChunkLoadState::Loaded);
}

// ============================================================================
// ValidationStatus enum tests (solver.rs)
// ============================================================================

#[test]
fn validation_status_valid_eq() {
    assert_eq!(ValidationStatus::Valid, ValidationStatus::Valid);
}

#[test]
fn validation_status_out_of_bounds_eq() {
    assert_eq!(ValidationStatus::OutOfBounds, ValidationStatus::OutOfBounds);
}

#[test]
fn validation_status_valid_ne_out_of_bounds() {
    assert_ne!(ValidationStatus::Valid, ValidationStatus::OutOfBounds);
}

#[test]
fn validation_status_biome_incompatible_eq() {
    assert_eq!(
        ValidationStatus::BiomeIncompatible(BiomeType::Desert),
        ValidationStatus::BiomeIncompatible(BiomeType::Desert)
    );
}

#[test]
fn validation_status_biome_incompatible_ne_different_biome() {
    assert_ne!(
        ValidationStatus::BiomeIncompatible(BiomeType::Desert),
        ValidationStatus::BiomeIncompatible(BiomeType::Forest)
    );
}
