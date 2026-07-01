//! Wave 2 Mutation Remediation Tests — biome, lod_manager, terrain_modifier
//!
//! Pins exact BiomeType string conversion, BiomeConfig factory values,
//! LodLevel match arms, LodConfig hysteresis math, TerrainModifierConfig defaults,
//! VoxelOp factory methods, and NavMeshRegion overlap/merge arithmetic.

use astraweave_terrain::biome::{BiomeConditions, BiomeSky, BiomeVegetation};
use astraweave_terrain::*;
use glam::{IVec3, Vec3};

// ============================================================================
// BiomeType: as_str exact strings
// ============================================================================

#[test]
fn biome_type_as_str_grassland() {
    assert_eq!(BiomeType::Grassland.as_str(), "grassland");
}

#[test]
fn biome_type_as_str_desert() {
    assert_eq!(BiomeType::Desert.as_str(), "desert");
}

#[test]
fn biome_type_as_str_forest() {
    assert_eq!(BiomeType::Forest.as_str(), "forest");
}

#[test]
fn biome_type_as_str_mountain() {
    assert_eq!(BiomeType::Mountain.as_str(), "mountain");
}

#[test]
fn biome_type_as_str_tundra() {
    assert_eq!(BiomeType::Tundra.as_str(), "tundra");
}

#[test]
fn biome_type_as_str_swamp() {
    assert_eq!(BiomeType::Swamp.as_str(), "swamp");
}

#[test]
fn biome_type_as_str_beach() {
    assert_eq!(BiomeType::Beach.as_str(), "beach");
}

#[test]
fn biome_type_as_str_river() {
    assert_eq!(BiomeType::River.as_str(), "river");
}

// ============================================================================
// BiomeType: parse round-trip
// ============================================================================

#[test]
fn biome_type_parse_grassland() {
    assert_eq!(BiomeType::parse("grassland"), Some(BiomeType::Grassland));
}

#[test]
fn biome_type_parse_case_insensitive() {
    assert_eq!(BiomeType::parse("DESERT"), Some(BiomeType::Desert));
    assert_eq!(BiomeType::parse("Forest"), Some(BiomeType::Forest));
}

#[test]
fn biome_type_parse_invalid_returns_none() {
    assert_eq!(BiomeType::parse("lava"), None);
}

#[test]
fn biome_type_all_has_8_variants() {
    assert_eq!(BiomeType::all().len(), 8);
}

// ============================================================================
// BiomeType: material_dir paths
// ============================================================================

#[test]
fn biome_type_material_dir_forest() {
    let path = BiomeType::Forest.material_dir();
    assert_eq!(path, std::path::PathBuf::from("assets/materials/forest"));
}

#[test]
fn biome_type_material_dir_desert() {
    let path = BiomeType::Desert.material_dir();
    assert_eq!(path, std::path::PathBuf::from("assets/materials/desert"));
}

#[test]
fn biome_type_terrain_fallback_dir() {
    let path = BiomeType::terrain_fallback_material_dir();
    assert_eq!(path, std::path::PathBuf::from("assets/materials/terrain"));
}

// ============================================================================
// BiomeConfig: grassland factory exact values
// ============================================================================

#[test]
fn biome_config_grassland_type() {
    let c = BiomeConfig::grassland();
    assert_eq!(c.biome_type, BiomeType::Grassland);
}

#[test]
fn biome_config_grassland_name() {
    let c = BiomeConfig::grassland();
    assert_eq!(c.name, "Temperate Grassland");
}

#[test]
fn biome_config_grassland_height_range() {
    let c = BiomeConfig::grassland();
    assert!((c.conditions.height_range.0 - 0.0).abs() < 1e-6);
    assert!((c.conditions.height_range.1 - 50.0).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_temperature_range() {
    let c = BiomeConfig::grassland();
    assert!((c.conditions.temperature_range.0 - 0.3).abs() < 1e-6);
    assert!((c.conditions.temperature_range.1 - 0.8).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_moisture_range() {
    let c = BiomeConfig::grassland();
    assert!((c.conditions.moisture_range.0 - 0.4).abs() < 1e-6);
    assert!((c.conditions.moisture_range.1 - 0.8).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_max_slope() {
    let c = BiomeConfig::grassland();
    assert!((c.conditions.max_slope - 30.0).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_fog_density() {
    let c = BiomeConfig::grassland();
    assert!((c.sky.fog_density - 0.05).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_cloud_coverage() {
    let c = BiomeConfig::grassland();
    assert!((c.sky.cloud_coverage - 0.4).abs() < 1e-6);
}

#[test]
fn biome_config_grassland_vegetation_density() {
    let c = BiomeConfig::grassland();
    assert!((c.vegetation.density - 0.004).abs() < 1e-6);
}

// ============================================================================
// BiomeConditions: default values
// ============================================================================

#[test]
fn biome_conditions_default_height_range() {
    let c = BiomeConditions::default();
    assert!((c.height_range.0 - 0.0).abs() < 1e-6);
    assert!((c.height_range.1 - 1000.0).abs() < 1e-6);
}

#[test]
fn biome_conditions_default_temperature_range() {
    let c = BiomeConditions::default();
    assert!((c.temperature_range.0 - 0.0).abs() < 1e-6);
    assert!((c.temperature_range.1 - 1.0).abs() < 1e-6);
}

#[test]
fn biome_conditions_default_moisture_range() {
    let c = BiomeConditions::default();
    assert!((c.moisture_range.0 - 0.0).abs() < 1e-6);
    assert!((c.moisture_range.1 - 1.0).abs() < 1e-6);
}

#[test]
fn biome_conditions_default_max_slope() {
    let c = BiomeConditions::default();
    assert!((c.max_slope - 90.0).abs() < 1e-6);
}

// ============================================================================
// BiomeSky: exact defaults
// ============================================================================

#[test]
fn biome_sky_default_fog_density() {
    let s = BiomeSky::default();
    assert!((s.fog_density - 0.0).abs() < 1e-6);
}

#[test]
fn biome_sky_default_cloud_coverage() {
    let s = BiomeSky::default();
    assert!((s.cloud_coverage - 0.3).abs() < 1e-6);
}

#[test]
fn biome_sky_default_horizon_color() {
    let s = BiomeSky::default();
    assert!((s.horizon_color.x - 0.5).abs() < 1e-6);
    assert!((s.horizon_color.y - 0.7).abs() < 1e-6);
    assert!((s.horizon_color.z - 0.9).abs() < 1e-6);
}

#[test]
fn biome_sky_default_zenith_color() {
    let s = BiomeSky::default();
    assert!((s.zenith_color.x - 0.2).abs() < 1e-6);
    assert!((s.zenith_color.y - 0.4).abs() < 1e-6);
    assert!((s.zenith_color.z - 0.8).abs() < 1e-6);
}

// ============================================================================
// BiomeVegetation: exact defaults
// ============================================================================

#[test]
fn biome_vegetation_default_density() {
    let v = BiomeVegetation::default();
    assert!((v.density - 0.005).abs() < 1e-6);
}

#[test]
fn biome_vegetation_default_size_variation() {
    let v = BiomeVegetation::default();
    assert!((v.size_variation.0 - 0.8).abs() < 1e-6);
    assert!((v.size_variation.1 - 1.5).abs() < 1e-6);
}

#[test]
fn biome_vegetation_default_has_one_type() {
    let v = BiomeVegetation::default();
    assert_eq!(v.vegetation_types.len(), 1);
    assert_eq!(v.vegetation_types[0].name, "grass");
}

#[test]
fn biome_vegetation_default_random_rotation() {
    let v = BiomeVegetation::default();
    assert!(v.random_rotation);
}

// ============================================================================
// LodLevel: skip_factor exact values
// ============================================================================

#[test]
fn lod_level_skip_factor_full() {
    assert_eq!(LodLevel::Full.skip_factor(), 1);
}

#[test]
fn lod_level_skip_factor_half() {
    assert_eq!(LodLevel::Half.skip_factor(), 2);
}

#[test]
fn lod_level_skip_factor_quarter() {
    assert_eq!(LodLevel::Quarter.skip_factor(), 4);
}

#[test]
fn lod_level_skip_factor_skybox() {
    assert_eq!(LodLevel::Skybox.skip_factor(), 16);
}

// ============================================================================
// LodLevel: lower/higher chains
// ============================================================================

#[test]
fn lod_level_lower_full_to_half() {
    assert_eq!(LodLevel::Full.lower(), Some(LodLevel::Half));
}

#[test]
fn lod_level_lower_half_to_quarter() {
    assert_eq!(LodLevel::Half.lower(), Some(LodLevel::Quarter));
}

#[test]
fn lod_level_lower_quarter_to_skybox() {
    assert_eq!(LodLevel::Quarter.lower(), Some(LodLevel::Skybox));
}

#[test]
fn lod_level_lower_skybox_is_none() {
    assert_eq!(LodLevel::Skybox.lower(), None);
}

#[test]
fn lod_level_higher_skybox_to_quarter() {
    assert_eq!(LodLevel::Skybox.higher(), Some(LodLevel::Quarter));
}

#[test]
fn lod_level_higher_quarter_to_half() {
    assert_eq!(LodLevel::Quarter.higher(), Some(LodLevel::Half));
}

#[test]
fn lod_level_higher_half_to_full() {
    assert_eq!(LodLevel::Half.higher(), Some(LodLevel::Full));
}

#[test]
fn lod_level_higher_full_is_none() {
    assert_eq!(LodLevel::Full.higher(), None);
}

// ============================================================================
// LodConfig: exact default values
// ============================================================================

#[test]
fn lod_hysteresis_config_default_thresholds() {
    let c = LodHysteresisConfig::default();
    assert!((c.distance_thresholds[0] - 256.0).abs() < 1e-6);
    assert!((c.distance_thresholds[1] - 512.0).abs() < 1e-6);
    assert!((c.distance_thresholds[2] - 1024.0).abs() < 1e-6);
}

#[test]
fn lod_hysteresis_config_default_hysteresis_margin() {
    let c = LodHysteresisConfig::default();
    assert!((c.hysteresis_margin - 0.1).abs() < 1e-6);
}

#[test]
fn lod_hysteresis_config_default_blend_zone_size() {
    let c = LodHysteresisConfig::default();
    assert!((c.blend_zone_size - 32.0).abs() < 1e-6);
}

#[test]
fn lod_hysteresis_config_default_blending_enabled() {
    let c = LodHysteresisConfig::default();
    assert!(c.enable_blending);
}

// ============================================================================
// LodConfig: hysteresis threshold equations
// ============================================================================

#[test]
fn lod_config_threshold_increasing_detail() {
    let c = LodHysteresisConfig::default();
    // Full→Half base is 256.0, increasing detail: 256 * (1 - 0.1) = 230.4
    let t = c.get_threshold(LodLevel::Full, LodLevel::Half, true);
    assert!((t - 230.4).abs() < 1e-3);
}

#[test]
fn lod_config_threshold_decreasing_detail() {
    let c = LodHysteresisConfig::default();
    // Full→Half base is 256.0, decreasing detail: 256 * (1 + 0.1) = 281.6
    let t = c.get_threshold(LodLevel::Full, LodLevel::Half, false);
    assert!((t - 281.6).abs() < 1e-3);
}

#[test]
fn lod_config_threshold_half_quarter_base() {
    let c = LodHysteresisConfig::default();
    // Half→Quarter base is 512.0, decreasing: 512 * 1.1 = 563.2
    let t = c.get_threshold(LodLevel::Half, LodLevel::Quarter, false);
    assert!((t - 563.2).abs() < 1e-3);
}

#[test]
fn lod_config_threshold_quarter_skybox_increasing() {
    let c = LodHysteresisConfig::default();
    // Quarter→Skybox base is 1024.0, increasing: 1024 * 0.9 = 921.6
    let t = c.get_threshold(LodLevel::Quarter, LodLevel::Skybox, true);
    assert!((t - 921.6).abs() < 1e-3);
}

#[test]
fn lod_config_threshold_invalid_transition_returns_max() {
    let c = LodHysteresisConfig::default();
    let t = c.get_threshold(LodLevel::Full, LodLevel::Skybox, false);
    assert_eq!(t, f32::MAX);
}

// ============================================================================
// TerrainModifierConfig: exact defaults
// ============================================================================

#[test]
fn terrain_modifier_config_default_data_budget() {
    let c = TerrainModifierConfig::default();
    assert_eq!(c.data_pass_budget_us, 1000);
}

#[test]
fn terrain_modifier_config_default_mesh_budget() {
    let c = TerrainModifierConfig::default();
    assert_eq!(c.mesh_pass_budget_us, 2000);
}

#[test]
fn terrain_modifier_config_default_max_ops() {
    let c = TerrainModifierConfig::default();
    assert_eq!(c.max_ops_per_frame, 1000);
}

#[test]
fn terrain_modifier_config_default_max_remeshes() {
    let c = TerrainModifierConfig::default();
    assert_eq!(c.max_remeshes_per_frame, 4);
}

#[test]
fn terrain_modifier_config_default_prioritize_camera() {
    let c = TerrainModifierConfig::default();
    assert!(c.prioritize_near_camera);
}

// ============================================================================
// VoxelOp: factory methods and defaults
// ============================================================================

#[test]
fn voxel_op_set_has_default_priority_128() {
    let v = Voxel::default();
    let op = VoxelOp::set(IVec3::ZERO, v, "req-1".to_string());
    assert_eq!(op.priority, 128);
}

#[test]
fn voxel_op_add_density_has_default_priority_128() {
    let op = VoxelOp::add_density(IVec3::ZERO, 0.5, "req-2".to_string());
    assert_eq!(op.priority, 128);
}

#[test]
fn voxel_op_subtract_density_has_default_priority_128() {
    let op = VoxelOp::subtract_density(IVec3::ZERO, 0.5, "req-3".to_string());
    assert_eq!(op.priority, 128);
}

#[test]
fn voxel_op_with_priority_overrides() {
    let v = Voxel::default();
    let op = VoxelOp::set(IVec3::ZERO, v, "req-4".to_string()).with_priority(255);
    assert_eq!(op.priority, 255);
}

#[test]
fn voxel_op_position_preserved() {
    let pos = IVec3::new(10, 20, 30);
    let op = VoxelOp::add_density(pos, 1.0, "req-5".to_string());
    assert_eq!(op.position, pos);
}

#[test]
fn voxel_op_request_id_preserved() {
    let op = VoxelOp::add_density(IVec3::ZERO, 1.0, "my-req-id".to_string());
    assert_eq!(op.request_id, "my-req-id");
}

// ============================================================================
// NavMeshRegion: overlap and merge math
// ============================================================================

#[test]
fn navmesh_region_overlaps_identical() {
    let r = NavMeshRegion::new(Vec3::ZERO, Vec3::ONE);
    assert!(r.overlaps(&r));
}

#[test]
fn navmesh_region_overlaps_partial() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
    let b = NavMeshRegion::new(Vec3::ONE, Vec3::new(3.0, 3.0, 3.0));
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn navmesh_region_no_overlap_separated() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::ONE);
    let b = NavMeshRegion::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));
    assert!(!a.overlaps(&b));
    assert!(!b.overlaps(&a));
}

#[test]
fn navmesh_region_merge_bounding_box() {
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::ONE);
    let b = NavMeshRegion::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0));
    let merged = a.merge(&b);
    assert!((merged.min.x - 0.0).abs() < 1e-6);
    assert!((merged.min.y - 0.0).abs() < 1e-6);
    assert!((merged.min.z - 0.0).abs() < 1e-6);
    assert!((merged.max.x - 3.0).abs() < 1e-6);
    assert!((merged.max.y - 3.0).abs() < 1e-6);
    assert!((merged.max.z - 3.0).abs() < 1e-6);
}

#[test]
fn navmesh_region_overlap_edge_touching() {
    // Touching at boundary should overlap (min <= max)
    let a = NavMeshRegion::new(Vec3::ZERO, Vec3::ONE);
    let b = NavMeshRegion::new(Vec3::ONE, Vec3::new(2.0, 2.0, 2.0));
    assert!(a.overlaps(&b));
}

#[test]
fn navmesh_region_no_overlap_one_axis() {
    // Only X axis separated → no overlap
    let a = NavMeshRegion::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
    let b = NavMeshRegion::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));
    assert!(!a.overlaps(&b));
}

// ============================================================================
// ModifierStats: default zeroed
// ============================================================================

#[test]
fn modifier_stats_default_all_zero() {
    let s = ModifierStats::default();
    assert_eq!(s.ops_processed, 0);
    assert_eq!(s.ops_pending, 0);
    assert_eq!(s.chunks_remeshed, 0);
    assert_eq!(s.chunks_pending_remesh, 0);
    assert_eq!(s.data_pass_time_us, 0);
    assert_eq!(s.mesh_pass_time_us, 0);
    assert!(!s.work_deferred);
}
