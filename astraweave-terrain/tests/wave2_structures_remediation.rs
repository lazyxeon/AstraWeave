//! Wave 2 Proactive Remediation: structures.rs (109 mutants)
//!
//! Golden-value tests for every match arm in StructureType methods
//! and StructureConfig/StructureResult logic.

use astraweave_terrain::structures::{
    StructureConfig, StructureGenerator, StructureInstance, StructureResult, StructureType,
};
use astraweave_terrain::BiomeType;
use glam::Vec3;

// ============================================================================
// StructureType::for_biome — exhaustive membership tests per biome
// ============================================================================

#[test]
fn for_biome_grassland_exact_members() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert_eq!(types.len(), 11);
    assert!(types.contains(&StructureType::Cottage));
    assert!(types.contains(&StructureType::Farmhouse));
    assert!(types.contains(&StructureType::Villa));
    assert!(types.contains(&StructureType::Tavern));
    assert!(types.contains(&StructureType::Blacksmith));
    assert!(types.contains(&StructureType::Market));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::Watchtower));
    assert!(types.contains(&StructureType::Well));
    assert!(types.contains(&StructureType::Windmill));
    assert!(types.contains(&StructureType::AncientRuin));
}

#[test]
fn for_biome_grassland_excludes() {
    let types = StructureType::for_biome(BiomeType::Grassland);
    assert!(!types.contains(&StructureType::Fort));
    assert!(!types.contains(&StructureType::Cabin));
    assert!(!types.contains(&StructureType::Cave));
    assert!(!types.contains(&StructureType::Lighthouse));
    assert!(!types.contains(&StructureType::Bridge));
    assert!(!types.contains(&StructureType::Obelisk));
}

#[test]
fn for_biome_desert_exact_members() {
    let types = StructureType::for_biome(BiomeType::Desert);
    assert_eq!(types.len(), 10);
    assert!(types.contains(&StructureType::Villa));
    assert!(types.contains(&StructureType::Market));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::Watchtower));
    assert!(types.contains(&StructureType::Fort));
    assert!(types.contains(&StructureType::AncientRuin));
    assert!(types.contains(&StructureType::Obelisk));
    assert!(types.contains(&StructureType::Tomb));
    assert!(types.contains(&StructureType::RockFormation));
    assert!(types.contains(&StructureType::Well));
}

#[test]
fn for_biome_desert_excludes() {
    let types = StructureType::for_biome(BiomeType::Desert);
    assert!(!types.contains(&StructureType::Cottage));
    assert!(!types.contains(&StructureType::Farmhouse));
    assert!(!types.contains(&StructureType::Cabin));
    assert!(!types.contains(&StructureType::Tavern));
    assert!(!types.contains(&StructureType::Lighthouse));
}

#[test]
fn for_biome_forest_exact_members() {
    let types = StructureType::for_biome(BiomeType::Forest);
    assert_eq!(types.len(), 8);
    assert!(types.contains(&StructureType::Cottage));
    assert!(types.contains(&StructureType::Cabin));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::Watchtower));
    assert!(types.contains(&StructureType::AncientRuin));
    assert!(types.contains(&StructureType::StoneCircle));
    assert!(types.contains(&StructureType::Cave));
    assert!(types.contains(&StructureType::RockFormation));
}

#[test]
fn for_biome_forest_excludes() {
    let types = StructureType::for_biome(BiomeType::Forest);
    assert!(!types.contains(&StructureType::Fort));
    assert!(!types.contains(&StructureType::Villa));
    assert!(!types.contains(&StructureType::Farmhouse));
    assert!(!types.contains(&StructureType::Lighthouse));
    assert!(!types.contains(&StructureType::Obelisk));
}

#[test]
fn for_biome_mountain_exact_members() {
    let types = StructureType::for_biome(BiomeType::Mountain);
    assert_eq!(types.len(), 9);
    assert!(types.contains(&StructureType::Cabin));
    assert!(types.contains(&StructureType::Fort));
    assert!(types.contains(&StructureType::Watchtower));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::Cave));
    assert!(types.contains(&StructureType::AncientRuin));
    assert!(types.contains(&StructureType::CrystalFormation));
    assert!(types.contains(&StructureType::RockFormation));
    assert!(types.contains(&StructureType::Bridge));
}

#[test]
fn for_biome_mountain_excludes() {
    let types = StructureType::for_biome(BiomeType::Mountain);
    assert!(!types.contains(&StructureType::Cottage));
    assert!(!types.contains(&StructureType::Farmhouse));
    assert!(!types.contains(&StructureType::Villa));
    assert!(!types.contains(&StructureType::Lighthouse));
    assert!(!types.contains(&StructureType::Well));
}

#[test]
fn for_biome_tundra_exact_members() {
    let types = StructureType::for_biome(BiomeType::Tundra);
    assert_eq!(types.len(), 7);
    assert!(types.contains(&StructureType::Cabin));
    assert!(types.contains(&StructureType::Fort));
    assert!(types.contains(&StructureType::Watchtower));
    assert!(types.contains(&StructureType::Cave));
    assert!(types.contains(&StructureType::AncientRuin));
    assert!(types.contains(&StructureType::RockFormation));
    assert!(types.contains(&StructureType::CrystalFormation));
}

#[test]
fn for_biome_tundra_excludes() {
    let types = StructureType::for_biome(BiomeType::Tundra);
    assert!(!types.contains(&StructureType::Cottage));
    assert!(!types.contains(&StructureType::Villa));
    assert!(!types.contains(&StructureType::Tavern));
    assert!(!types.contains(&StructureType::Temple));
    assert!(!types.contains(&StructureType::Lighthouse));
}

#[test]
fn for_biome_swamp_exact_members() {
    let types = StructureType::for_biome(BiomeType::Swamp);
    assert_eq!(types.len(), 6);
    assert!(types.contains(&StructureType::Cabin));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::AncientRuin));
    assert!(types.contains(&StructureType::StoneCircle));
    assert!(types.contains(&StructureType::Cave));
    assert!(types.contains(&StructureType::Bridge));
}

#[test]
fn for_biome_swamp_excludes() {
    let types = StructureType::for_biome(BiomeType::Swamp);
    assert!(!types.contains(&StructureType::Cottage));
    assert!(!types.contains(&StructureType::Fort));
    assert!(!types.contains(&StructureType::Villa));
    assert!(!types.contains(&StructureType::Lighthouse));
    assert!(!types.contains(&StructureType::Well));
}

#[test]
fn for_biome_beach_exact_members() {
    let types = StructureType::for_biome(BiomeType::Beach);
    assert_eq!(types.len(), 6);
    assert!(types.contains(&StructureType::Cottage));
    assert!(types.contains(&StructureType::Tavern));
    assert!(types.contains(&StructureType::Lighthouse));
    assert!(types.contains(&StructureType::Temple));
    assert!(types.contains(&StructureType::Cave));
    assert!(types.contains(&StructureType::RockFormation));
}

#[test]
fn for_biome_beach_excludes() {
    let types = StructureType::for_biome(BiomeType::Beach);
    assert!(!types.contains(&StructureType::Fort));
    assert!(!types.contains(&StructureType::Cabin));
    assert!(!types.contains(&StructureType::Farmhouse));
    assert!(!types.contains(&StructureType::Bridge));
    assert!(!types.contains(&StructureType::CrystalFormation));
}

#[test]
fn for_biome_river_exact_members() {
    let types = StructureType::for_biome(BiomeType::River);
    assert_eq!(types.len(), 7);
    assert!(types.contains(&StructureType::Cottage));
    assert!(types.contains(&StructureType::Farmhouse));
    assert!(types.contains(&StructureType::Tavern));
    assert!(types.contains(&StructureType::Blacksmith));
    assert!(types.contains(&StructureType::Bridge));
    assert!(types.contains(&StructureType::Well));
    assert!(types.contains(&StructureType::Windmill));
}

#[test]
fn for_biome_river_excludes() {
    let types = StructureType::for_biome(BiomeType::River);
    assert!(!types.contains(&StructureType::Fort));
    assert!(!types.contains(&StructureType::Cave));
    assert!(!types.contains(&StructureType::Temple));
    assert!(!types.contains(&StructureType::Lighthouse));
    assert!(!types.contains(&StructureType::CrystalFormation));
}

// ============================================================================
// StructureType::typical_size — golden value per type
// ============================================================================

#[test]
fn typical_size_residential() {
    assert_eq!(StructureType::Cottage.typical_size(), 8.0);
    assert_eq!(StructureType::Cabin.typical_size(), 8.0);
    assert_eq!(StructureType::Farmhouse.typical_size(), 12.0);
    assert_eq!(StructureType::Villa.typical_size(), 12.0);
}

#[test]
fn typical_size_commercial() {
    assert_eq!(StructureType::Tavern.typical_size(), 10.0);
    assert_eq!(StructureType::Blacksmith.typical_size(), 10.0);
    assert_eq!(StructureType::Market.typical_size(), 10.0);
    assert_eq!(StructureType::Temple.typical_size(), 15.0);
}

#[test]
fn typical_size_defensive() {
    assert_eq!(StructureType::Watchtower.typical_size(), 6.0);
    assert_eq!(StructureType::Fort.typical_size(), 20.0);
    assert_eq!(StructureType::Wall.typical_size(), 5.0);
    assert_eq!(StructureType::Gate.typical_size(), 5.0);
}

#[test]
fn typical_size_ancient() {
    assert_eq!(StructureType::AncientRuin.typical_size(), 15.0);
    assert_eq!(StructureType::StoneCircle.typical_size(), 12.0);
    assert_eq!(StructureType::Obelisk.typical_size(), 3.0);
    assert_eq!(StructureType::Tomb.typical_size(), 8.0);
}

#[test]
fn typical_size_natural() {
    assert_eq!(StructureType::Cave.typical_size(), 6.0);
    assert_eq!(StructureType::RockFormation.typical_size(), 4.0);
    assert_eq!(StructureType::CrystalFormation.typical_size(), 5.0);
}

#[test]
fn typical_size_infrastructure() {
    assert_eq!(StructureType::Bridge.typical_size(), 15.0);
    assert_eq!(StructureType::Well.typical_size(), 2.0);
    assert_eq!(StructureType::Windmill.typical_size(), 8.0);
    assert_eq!(StructureType::Lighthouse.typical_size(), 6.0);
}

// ============================================================================
// StructureType::rarity — golden value per type
// ============================================================================

#[test]
fn rarity_common() {
    assert_eq!(StructureType::Cottage.rarity(), 0.8);
    assert_eq!(StructureType::Farmhouse.rarity(), 0.8);
    assert_eq!(StructureType::RockFormation.rarity(), 0.8);
}

#[test]
fn rarity_uncommon() {
    assert_eq!(StructureType::Cabin.rarity(), 0.6);
    assert_eq!(StructureType::Tavern.rarity(), 0.6);
    assert_eq!(StructureType::Blacksmith.rarity(), 0.6);
    assert_eq!(StructureType::Well.rarity(), 0.6);
    assert_eq!(StructureType::Windmill.rarity(), 0.6);
}

#[test]
fn rarity_rare() {
    assert_eq!(StructureType::Villa.rarity(), 0.4);
    assert_eq!(StructureType::Market.rarity(), 0.4);
    assert_eq!(StructureType::Temple.rarity(), 0.4);
    assert_eq!(StructureType::Watchtower.rarity(), 0.4);
    assert_eq!(StructureType::Cave.rarity(), 0.4);
}

#[test]
fn rarity_very_rare() {
    assert_eq!(StructureType::Fort.rarity(), 0.2);
    assert_eq!(StructureType::AncientRuin.rarity(), 0.2);
    assert_eq!(StructureType::StoneCircle.rarity(), 0.2);
    assert_eq!(StructureType::Bridge.rarity(), 0.2);
    assert_eq!(StructureType::Lighthouse.rarity(), 0.2);
}

#[test]
fn rarity_extremely_rare() {
    assert_eq!(StructureType::Wall.rarity(), 0.1);
    assert_eq!(StructureType::Gate.rarity(), 0.1);
    assert_eq!(StructureType::Obelisk.rarity(), 0.1);
    assert_eq!(StructureType::Tomb.rarity(), 0.1);
    assert_eq!(StructureType::CrystalFormation.rarity(), 0.1);
}

#[test]
fn rarity_ordering_common_gt_rare() {
    assert!(StructureType::Cottage.rarity() > StructureType::Villa.rarity());
    assert!(StructureType::Farmhouse.rarity() > StructureType::Fort.rarity());
    assert!(StructureType::RockFormation.rarity() > StructureType::CrystalFormation.rarity());
}

#[test]
fn rarity_ordering_rare_gt_extremely_rare() {
    assert!(StructureType::Villa.rarity() > StructureType::Wall.rarity());
    assert!(StructureType::Cave.rarity() > StructureType::Tomb.rarity());
}

// ============================================================================
// StructureType::can_place_on_slope — boundary tests per group
// ============================================================================

#[test]
fn slope_flat_group_at_boundary() {
    // max_slope = 0.1
    assert!(StructureType::Farmhouse.can_place_on_slope(0.1));
    assert!(!StructureType::Farmhouse.can_place_on_slope(0.101));

    assert!(StructureType::Market.can_place_on_slope(0.1));
    assert!(!StructureType::Market.can_place_on_slope(0.11));

    assert!(StructureType::Temple.can_place_on_slope(0.1));
    assert!(!StructureType::Temple.can_place_on_slope(0.11));

    assert!(StructureType::Fort.can_place_on_slope(0.1));
    assert!(!StructureType::Fort.can_place_on_slope(0.11));
}

#[test]
fn slope_gentle_group_at_boundary() {
    // max_slope = 0.2
    assert!(StructureType::Cottage.can_place_on_slope(0.2));
    assert!(!StructureType::Cottage.can_place_on_slope(0.201));

    assert!(StructureType::Villa.can_place_on_slope(0.2));
    assert!(!StructureType::Villa.can_place_on_slope(0.201));

    assert!(StructureType::Tavern.can_place_on_slope(0.2));
    assert!(StructureType::Blacksmith.can_place_on_slope(0.2));
    assert!(StructureType::Well.can_place_on_slope(0.2));
    assert!(StructureType::Windmill.can_place_on_slope(0.2));
}

#[test]
fn slope_moderate_group_at_boundary() {
    // max_slope = 0.4
    assert!(StructureType::Cabin.can_place_on_slope(0.4));
    assert!(!StructureType::Cabin.can_place_on_slope(0.401));

    assert!(StructureType::Watchtower.can_place_on_slope(0.4));
    assert!(!StructureType::Watchtower.can_place_on_slope(0.41));

    assert!(StructureType::AncientRuin.can_place_on_slope(0.4));
    assert!(StructureType::StoneCircle.can_place_on_slope(0.4));
}

#[test]
fn slope_steep_group_at_boundary() {
    // max_slope = 0.8
    assert!(StructureType::Cave.can_place_on_slope(0.8));
    assert!(!StructureType::Cave.can_place_on_slope(0.801));

    assert!(StructureType::RockFormation.can_place_on_slope(0.8));
    assert!(!StructureType::RockFormation.can_place_on_slope(0.81));

    assert!(StructureType::CrystalFormation.can_place_on_slope(0.8));
    assert!(StructureType::Lighthouse.can_place_on_slope(0.8));
}

#[test]
fn slope_flexible_group_at_boundary() {
    // max_slope = 1.0
    assert!(StructureType::Wall.can_place_on_slope(1.0));
    assert!(!StructureType::Wall.can_place_on_slope(1.001));

    assert!(StructureType::Gate.can_place_on_slope(1.0));
    assert!(StructureType::Obelisk.can_place_on_slope(1.0));
    assert!(StructureType::Tomb.can_place_on_slope(1.0));
    assert!(StructureType::Bridge.can_place_on_slope(1.0));
}

#[test]
fn slope_zero_all_types_allowed() {
    // All types should allow zero slope
    let all_types = [
        StructureType::Cottage,
        StructureType::Farmhouse,
        StructureType::Villa,
        StructureType::Cabin,
        StructureType::Tavern,
        StructureType::Blacksmith,
        StructureType::Market,
        StructureType::Temple,
        StructureType::Watchtower,
        StructureType::Fort,
        StructureType::Wall,
        StructureType::Gate,
        StructureType::AncientRuin,
        StructureType::StoneCircle,
        StructureType::Obelisk,
        StructureType::Tomb,
        StructureType::Cave,
        StructureType::RockFormation,
        StructureType::CrystalFormation,
        StructureType::Bridge,
        StructureType::Well,
        StructureType::Windmill,
        StructureType::Lighthouse,
    ];
    for t in &all_types {
        assert!(t.can_place_on_slope(0.0), "{:?} should allow slope 0.0", t);
    }
}

#[test]
fn slope_group_boundaries_discriminate() {
    // slope=0.15 should allow gentle+ but not flat
    assert!(!StructureType::Farmhouse.can_place_on_slope(0.15)); // flat: max 0.1
    assert!(StructureType::Cottage.can_place_on_slope(0.15)); // gentle: max 0.2

    // slope=0.3 should allow moderate+ but not gentle
    assert!(!StructureType::Cottage.can_place_on_slope(0.3)); // gentle: max 0.2
    assert!(StructureType::Cabin.can_place_on_slope(0.3)); // moderate: max 0.4

    // slope=0.5 should allow steep+ but not moderate
    assert!(!StructureType::Cabin.can_place_on_slope(0.5)); // moderate: max 0.4
    assert!(StructureType::Cave.can_place_on_slope(0.5)); // steep: max 0.8

    // slope=0.9 should allow flexible+ but not steep
    assert!(!StructureType::Cave.can_place_on_slope(0.9)); // steep: max 0.8
    assert!(StructureType::Wall.can_place_on_slope(0.9)); // flexible: max 1.0
}

// ============================================================================
// StructureType::min_spacing — golden value per type
// ============================================================================

#[test]
fn min_spacing_large() {
    assert_eq!(StructureType::Fort.min_spacing(), 100.0);
    assert_eq!(StructureType::Temple.min_spacing(), 100.0);
    assert_eq!(StructureType::Market.min_spacing(), 100.0);
}

#[test]
fn min_spacing_medium() {
    assert_eq!(StructureType::Villa.min_spacing(), 50.0);
    assert_eq!(StructureType::Tavern.min_spacing(), 50.0);
    assert_eq!(StructureType::Blacksmith.min_spacing(), 50.0);
    assert_eq!(StructureType::Lighthouse.min_spacing(), 50.0);
    assert_eq!(StructureType::Bridge.min_spacing(), 50.0);
}

#[test]
fn min_spacing_small() {
    assert_eq!(StructureType::Cottage.min_spacing(), 30.0);
    assert_eq!(StructureType::Farmhouse.min_spacing(), 30.0);
    assert_eq!(StructureType::Cabin.min_spacing(), 30.0);
    assert_eq!(StructureType::Watchtower.min_spacing(), 30.0);
}

#[test]
fn min_spacing_very_small() {
    assert_eq!(StructureType::Well.min_spacing(), 20.0);
    assert_eq!(StructureType::RockFormation.min_spacing(), 20.0);
}

#[test]
fn min_spacing_minimal() {
    assert_eq!(StructureType::Cave.min_spacing(), 15.0);
    assert_eq!(StructureType::CrystalFormation.min_spacing(), 15.0);
    assert_eq!(StructureType::AncientRuin.min_spacing(), 15.0);
    assert_eq!(StructureType::StoneCircle.min_spacing(), 15.0);
    assert_eq!(StructureType::Obelisk.min_spacing(), 15.0);
    assert_eq!(StructureType::Tomb.min_spacing(), 15.0);
}

#[test]
fn min_spacing_no_restriction() {
    assert_eq!(StructureType::Wall.min_spacing(), 10.0);
    assert_eq!(StructureType::Gate.min_spacing(), 10.0);
    assert_eq!(StructureType::Windmill.min_spacing(), 10.0);
}

#[test]
fn min_spacing_all_positive() {
    let all_types = [
        StructureType::Cottage,
        StructureType::Farmhouse,
        StructureType::Villa,
        StructureType::Cabin,
        StructureType::Tavern,
        StructureType::Blacksmith,
        StructureType::Market,
        StructureType::Temple,
        StructureType::Watchtower,
        StructureType::Fort,
        StructureType::Wall,
        StructureType::Gate,
        StructureType::AncientRuin,
        StructureType::StoneCircle,
        StructureType::Obelisk,
        StructureType::Tomb,
        StructureType::Cave,
        StructureType::RockFormation,
        StructureType::CrystalFormation,
        StructureType::Bridge,
        StructureType::Well,
        StructureType::Windmill,
        StructureType::Lighthouse,
    ];
    for t in &all_types {
        assert!(
            t.min_spacing() > 0.0,
            "{:?} must have positive min_spacing",
            t
        );
    }
}

// ============================================================================
// StructureConfig defaults
// ============================================================================

#[test]
fn structure_config_default_values() {
    let cfg = StructureConfig::default();
    assert_eq!(cfg.density, 0.3);
    assert_eq!(cfg.edge_buffer, 20.0);
    assert_eq!(cfg.height_tolerance, 2.0);
    assert!(cfg.include_ancient);
    assert!(cfg.include_defensive);
    assert_eq!(cfg.seed, 0);
}

#[test]
fn structure_config_edge_buffer_is_20() {
    // edge_buffer=20.0 means chunk_size must be >40.0 or placement range is negative
    let cfg = StructureConfig::default();
    assert_eq!(cfg.edge_buffer, 20.0);
    // Valid range = (edge_buffer, chunk_size - edge_buffer)
    // For chunk_size=64: range = (20, 44)
    assert!(
        64.0 - 2.0 * cfg.edge_buffer > 0.0,
        "64 chunk must have positive range"
    );
}

// ============================================================================
// StructureResult
// ============================================================================

#[test]
fn structure_result_new_is_empty() {
    let result = StructureResult::new();
    assert_eq!(result.total_count(), 0);
    assert!(result.structures.is_empty());
    assert!(result.counts_by_type.is_empty());
}

#[test]
fn structure_result_add_increments_count() {
    let mut result = StructureResult::new();
    result.add_structure(make_instance(StructureType::Cottage, Vec3::ZERO));
    assert_eq!(result.total_count(), 1);
    assert_eq!(
        *result.counts_by_type.get(&StructureType::Cottage).unwrap(),
        1
    );
}

#[test]
fn structure_result_add_multiple_same_type() {
    let mut result = StructureResult::new();
    result.add_structure(make_instance(StructureType::Cave, Vec3::new(0.0, 0.0, 0.0)));
    result.add_structure(make_instance(
        StructureType::Cave,
        Vec3::new(50.0, 0.0, 50.0),
    ));
    assert_eq!(result.total_count(), 2);
    assert_eq!(*result.counts_by_type.get(&StructureType::Cave).unwrap(), 2);
}

#[test]
fn structure_result_add_multiple_different_types() {
    let mut result = StructureResult::new();
    result.add_structure(make_instance(
        StructureType::Cottage,
        Vec3::new(0.0, 0.0, 0.0),
    ));
    result.add_structure(make_instance(
        StructureType::Fort,
        Vec3::new(200.0, 0.0, 200.0),
    ));
    result.add_structure(make_instance(
        StructureType::Well,
        Vec3::new(50.0, 0.0, 50.0),
    ));
    assert_eq!(result.total_count(), 3);
    assert_eq!(result.counts_by_type.len(), 3);
    assert_eq!(
        *result.counts_by_type.get(&StructureType::Cottage).unwrap(),
        1
    );
    assert_eq!(*result.counts_by_type.get(&StructureType::Fort).unwrap(), 1);
    assert_eq!(*result.counts_by_type.get(&StructureType::Well).unwrap(), 1);
}

#[test]
fn structure_result_counts_by_type_mixed() {
    let mut result = StructureResult::new();
    for i in 0..5 {
        result.add_structure(make_instance(
            StructureType::Wall,
            Vec3::new(i as f32 * 20.0, 0.0, 0.0),
        ));
    }
    for i in 0..3 {
        result.add_structure(make_instance(
            StructureType::Gate,
            Vec3::new(0.0, 0.0, i as f32 * 20.0),
        ));
    }
    assert_eq!(result.total_count(), 8);
    assert_eq!(*result.counts_by_type.get(&StructureType::Wall).unwrap(), 5);
    assert_eq!(*result.counts_by_type.get(&StructureType::Gate).unwrap(), 3);
}

// ============================================================================
// StructureGenerator — public interface
// ============================================================================

#[test]
fn generator_new_accepts_config() {
    let config = StructureConfig {
        density: 0.5,
        seed: 42,
        ..Default::default()
    };
    let _gen = StructureGenerator::new(config);
    // Should not panic
}

#[test]
fn generator_zero_density_returns_empty() {
    let config = StructureConfig {
        density: 0.0,
        seed: 42,
        ..Default::default()
    };
    let mut gen = StructureGenerator::new(config);

    // We need a TerrainChunk. Use WorldGenerator to create one.
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    let result = gen
        .generate_structures(&chunk, 256.0, BiomeType::Grassland)
        .unwrap();
    assert_eq!(result.total_count(), 0);
}

#[test]
fn generator_negative_density_returns_empty() {
    let config = StructureConfig {
        density: -1.0,
        seed: 42,
        ..Default::default()
    };
    let mut gen = StructureGenerator::new(config);

    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    let result = gen
        .generate_structures(&chunk, 256.0, BiomeType::Grassland)
        .unwrap();
    assert_eq!(result.total_count(), 0);
}

#[test]
fn generator_deterministic_with_same_seed() {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    let config1 = StructureConfig {
        density: 0.5,
        seed: 12345,
        ..Default::default()
    };
    let mut gen1 = StructureGenerator::new(config1);
    let result1 = gen1
        .generate_structures(&chunk, 256.0, BiomeType::Grassland)
        .unwrap();

    let config2 = StructureConfig {
        density: 0.5,
        seed: 12345,
        ..Default::default()
    };
    let mut gen2 = StructureGenerator::new(config2);
    let result2 = gen2
        .generate_structures(&chunk, 256.0, BiomeType::Grassland)
        .unwrap();

    assert_eq!(result1.total_count(), result2.total_count());
}

#[test]
fn generator_different_seeds_may_differ() {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    // Run many seeds and verify at least some variation
    let mut counts = Vec::new();
    for seed in 0..20 {
        let config = StructureConfig {
            density: 0.8,
            seed,
            ..Default::default()
        };
        let mut gen = StructureGenerator::new(config);
        let result = gen
            .generate_structures(&chunk, 256.0, BiomeType::Grassland)
            .unwrap();
        counts.push(result.total_count());
    }
    let min = counts.iter().min().unwrap();
    let max = counts.iter().max().unwrap();
    // With 20 seeds and density 0.8, there should be SOME variation
    assert!(
        max > min,
        "Expected variation across seeds, got all {}",
        min
    );
}

#[test]
fn generator_exclude_ancient_filters_ruins() {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    let config = StructureConfig {
        density: 1.0,
        include_ancient: false,
        seed: 42,
        ..Default::default()
    };
    let mut gen = StructureGenerator::new(config);
    let result = gen
        .generate_structures(&chunk, 256.0, BiomeType::Desert)
        .unwrap();

    // No ancient structures should appear
    for s in &result.structures {
        assert!(
            !matches!(
                s.structure_type,
                StructureType::AncientRuin
                    | StructureType::StoneCircle
                    | StructureType::Obelisk
                    | StructureType::Tomb
            ),
            "Ancient structure {:?} should have been filtered",
            s.structure_type
        );
    }
}

#[test]
fn generator_exclude_defensive_filters_forts() {
    let world_config = astraweave_terrain::WorldConfig::default();
    let world_gen = astraweave_terrain::WorldGenerator::new(world_config);
    let chunk = world_gen
        .generate_chunk(astraweave_terrain::ChunkId::new(0, 0))
        .unwrap();

    let config = StructureConfig {
        density: 1.0,
        include_defensive: false,
        seed: 42,
        ..Default::default()
    };
    let mut gen = StructureGenerator::new(config);
    let result = gen
        .generate_structures(&chunk, 256.0, BiomeType::Mountain)
        .unwrap();

    // No defensive structures should appear
    for s in &result.structures {
        assert!(
            !matches!(
                s.structure_type,
                StructureType::Fort
                    | StructureType::Watchtower
                    | StructureType::Wall
                    | StructureType::Gate
            ),
            "Defensive structure {:?} should have been filtered",
            s.structure_type
        );
    }
}

// ============================================================================
// Cross-method consistency
// ============================================================================

#[test]
fn all_biome_types_return_nonempty() {
    let biomes = [
        BiomeType::Grassland,
        BiomeType::Desert,
        BiomeType::Forest,
        BiomeType::Mountain,
        BiomeType::Tundra,
        BiomeType::Swamp,
        BiomeType::Beach,
        BiomeType::River,
    ];
    for b in &biomes {
        let types = StructureType::for_biome(*b);
        assert!(
            !types.is_empty(),
            "{:?} biome returned no structure types",
            b
        );
    }
}

#[test]
fn all_types_have_positive_size_and_rarity() {
    let all_types = [
        StructureType::Cottage,
        StructureType::Farmhouse,
        StructureType::Villa,
        StructureType::Cabin,
        StructureType::Tavern,
        StructureType::Blacksmith,
        StructureType::Market,
        StructureType::Temple,
        StructureType::Watchtower,
        StructureType::Fort,
        StructureType::Wall,
        StructureType::Gate,
        StructureType::AncientRuin,
        StructureType::StoneCircle,
        StructureType::Obelisk,
        StructureType::Tomb,
        StructureType::Cave,
        StructureType::RockFormation,
        StructureType::CrystalFormation,
        StructureType::Bridge,
        StructureType::Well,
        StructureType::Windmill,
        StructureType::Lighthouse,
    ];
    for t in &all_types {
        assert!(t.typical_size() > 0.0, "{:?} has non-positive size", t);
        assert!(t.rarity() > 0.0, "{:?} has non-positive rarity", t);
    }
}

#[test]
fn fort_is_largest_structure() {
    let all_types = [
        StructureType::Cottage,
        StructureType::Farmhouse,
        StructureType::Villa,
        StructureType::Cabin,
        StructureType::Tavern,
        StructureType::Blacksmith,
        StructureType::Market,
        StructureType::Temple,
        StructureType::Watchtower,
        StructureType::Fort,
        StructureType::Wall,
        StructureType::Gate,
        StructureType::AncientRuin,
        StructureType::StoneCircle,
        StructureType::Obelisk,
        StructureType::Tomb,
        StructureType::Cave,
        StructureType::RockFormation,
        StructureType::CrystalFormation,
        StructureType::Bridge,
        StructureType::Well,
        StructureType::Windmill,
        StructureType::Lighthouse,
    ];
    let fort_size = StructureType::Fort.typical_size();
    for t in &all_types {
        assert!(
            t.typical_size() <= fort_size,
            "{:?} size {} exceeds Fort size {}",
            t,
            t.typical_size(),
            fort_size
        );
    }
}

#[test]
fn well_is_smallest_structure() {
    let all_types = [
        StructureType::Cottage,
        StructureType::Farmhouse,
        StructureType::Villa,
        StructureType::Cabin,
        StructureType::Tavern,
        StructureType::Blacksmith,
        StructureType::Market,
        StructureType::Temple,
        StructureType::Watchtower,
        StructureType::Fort,
        StructureType::Wall,
        StructureType::Gate,
        StructureType::AncientRuin,
        StructureType::StoneCircle,
        StructureType::Obelisk,
        StructureType::Tomb,
        StructureType::Cave,
        StructureType::RockFormation,
        StructureType::CrystalFormation,
        StructureType::Bridge,
        StructureType::Well,
        StructureType::Windmill,
        StructureType::Lighthouse,
    ];
    let well_size = StructureType::Well.typical_size();
    for t in &all_types {
        assert!(
            t.typical_size() >= well_size,
            "{:?} size {} is smaller than Well size {}",
            t,
            t.typical_size(),
            well_size
        );
    }
}

// ============================================================================
// Helper
// ============================================================================

fn make_instance(structure_type: StructureType, position: Vec3) -> StructureInstance {
    StructureInstance {
        structure_type,
        position,
        rotation: 0.0,
        scale: 1.0,
        model_path: format!("test_{:?}.glb", structure_type),
        texture_variant: None,
    }
}
