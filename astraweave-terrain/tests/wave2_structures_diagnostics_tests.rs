//! Wave 2 Mutation Remediation Tests — structures, streaming_diagnostics, lod_blending
//!
//! Pins exact config defaults, enum match arms, size/rarity/spacing tables,
//! diagnostic arithmetic, and morph factor calculations.

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
