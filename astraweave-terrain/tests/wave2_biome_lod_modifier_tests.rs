//! Wave 2 Mutation Remediation Tests — biome factories.
//!
//! Pins exact BiomeType string conversion and BiomeConfig factory values.
//! LOD hysteresis, the batched voxel modifier, and the nav-region helpers
//! were removed with those modules.

use astraweave_terrain::biome::{BiomeConditions, BiomeSky, BiomeVegetation};
use astraweave_terrain::*;

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
