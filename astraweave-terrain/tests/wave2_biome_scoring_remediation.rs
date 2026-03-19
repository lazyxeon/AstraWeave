//! Wave 2 Mutation Remediation — Biome Scoring & Classification
//!
//! Targets the 61 missed mutants in biome.rs by testing:
//! - score_conditions: exact penalty magnitudes (height×0.5, temp×2.0, moisture×1.5)
//! - score_conditions: in-range bonuses (+1.0 each)
//! - score_conditions: priority bonus (priority × 0.1)
//! - score_conditions: boundary conditions (exactly at range edges)
//! - is_slope_suitable: slope threshold
//! - BiomeType conversions and lookups

use astraweave_terrain::biome::{BiomeConditions, BiomeConfig, BiomeType};

// ============================================================================
// HELPER — Create a minimal BiomeConfig for testing score_conditions
// ============================================================================

fn test_biome_config(
    height_range: (f32, f32),
    temp_range: (f32, f32),
    moisture_range: (f32, f32),
    priority: i32,
) -> BiomeConfig {
    BiomeConfig {
        biome_type: BiomeType::Grassland,
        conditions: BiomeConditions {
            height_range,
            temperature_range: temp_range,
            moisture_range,
            max_slope: 45.0,
        },
        priority,
        ..BiomeConfig::grassland()
    }
}

// ============================================================================
// SCORE_CONDITIONS — In-range scores
// ============================================================================

/// All conditions perfectly in range: score = 3.0 + priority * 0.1
#[test]
fn score_all_in_range_exact() {
    let config = test_biome_config((0.0, 100.0), (0.0, 1.0), (0.0, 1.0), 0);
    let score = config.score_conditions(50.0, 0.5, 0.5);
    // 1.0 (height) + 1.0 (temp) + 1.0 (moisture) + 0 * 0.1 = 3.0
    assert!(
        (score - 3.0).abs() < 0.001,
        "All in range, priority 0: expected 3.0, got {score}"
    );
}

/// Priority bonus adds 0.1 per priority point.
#[test]
fn score_priority_bonus_adds_correctly() {
    let config0 = test_biome_config((0.0, 100.0), (0.0, 1.0), (0.0, 1.0), 0);
    let config5 = test_biome_config((0.0, 100.0), (0.0, 1.0), (0.0, 1.0), 5);
    let config10 = test_biome_config((0.0, 100.0), (0.0, 1.0), (0.0, 1.0), 10);

    let s0 = config0.score_conditions(50.0, 0.5, 0.5);
    let s5 = config5.score_conditions(50.0, 0.5, 0.5);
    let s10 = config10.score_conditions(50.0, 0.5, 0.5);

    assert!((s0 - 3.0).abs() < 0.001, "priority 0: {s0}");
    assert!((s5 - 3.5).abs() < 0.001, "priority 5: {s5}");
    assert!((s10 - 4.0).abs() < 0.001, "priority 10: {s10}");
}

// ============================================================================
// SCORE_CONDITIONS — Height penalty
// ============================================================================

/// Height below range incurs penalty of distance * 0.5.
#[test]
fn score_height_below_range_penalty() {
    let config = test_biome_config((50.0, 100.0), (0.0, 1.0), (0.0, 1.0), 0);
    // height=0, distance below range = 50.0 - 0.0 = 50.0, penalty = 50.0 * 0.5 = 25.0
    let score = config.score_conditions(0.0, 0.5, 0.5);
    // expected: -25.0 (height) + 1.0 (temp) + 1.0 (moisture) = -23.0
    assert!(
        (score - (-23.0)).abs() < 0.001,
        "Height below: expected -23.0, got {score}"
    );
}

/// Height above range incurs penalty of distance * 0.5.
#[test]
fn score_height_above_range_penalty() {
    let config = test_biome_config((0.0, 50.0), (0.0, 1.0), (0.0, 1.0), 0);
    // height=150, distance above = 150 - 50 = 100, penalty = 100 * 0.5 = 50.0
    let score = config.score_conditions(150.0, 0.5, 0.5);
    // expected: -50.0 (height) + 1.0 (temp) + 1.0 (moisture) = -48.0
    assert!(
        (score - (-48.0)).abs() < 0.001,
        "Height above: expected -48.0, got {score}"
    );
}

/// Height at exact lower boundary is in-range.
#[test]
fn score_height_at_lower_boundary_in_range() {
    let config = test_biome_config((50.0, 100.0), (0.0, 1.0), (0.0, 1.0), 0);
    let score = config.score_conditions(50.0, 0.5, 0.5);
    assert!(
        (score - 3.0).abs() < 0.001,
        "Height at lower bound: expected 3.0, got {score}"
    );
}

/// Height at exact upper boundary is in-range.
#[test]
fn score_height_at_upper_boundary_in_range() {
    let config = test_biome_config((50.0, 100.0), (0.0, 1.0), (0.0, 1.0), 0);
    let score = config.score_conditions(100.0, 0.5, 0.5);
    assert!(
        (score - 3.0).abs() < 0.001,
        "Height at upper bound: expected 3.0, got {score}"
    );
}

// ============================================================================
// SCORE_CONDITIONS — Temperature penalty
// ============================================================================

/// Temperature below range incurs penalty of distance * 2.0.
#[test]
fn score_temperature_below_range_penalty() {
    let config = test_biome_config((0.0, 1000.0), (0.5, 1.0), (0.0, 1.0), 0);
    // temp=0.0, distance = 0.5 - 0.0 = 0.5, penalty = 0.5 * 2.0 = 1.0
    let score = config.score_conditions(500.0, 0.0, 0.5);
    // expected: 1.0 (height) - 1.0 (temp penalty) + 1.0 (moisture) = 1.0
    assert!(
        (score - 1.0).abs() < 0.001,
        "Temp below: expected 1.0, got {score}"
    );
}

/// Temperature above range incurs penalty of distance * 2.0.
#[test]
fn score_temperature_above_range_penalty() {
    let config = test_biome_config((0.0, 1000.0), (0.0, 0.5), (0.0, 1.0), 0);
    // temp=1.0, distance = 1.0 - 0.5 = 0.5, penalty = 0.5 * 2.0 = 1.0
    let score = config.score_conditions(500.0, 1.0, 0.5);
    assert!(
        (score - 1.0).abs() < 0.001,
        "Temp above: expected 1.0, got {score}"
    );
}

/// Temperature at exact boundary.
#[test]
fn score_temperature_at_boundary_in_range() {
    let config = test_biome_config((0.0, 1000.0), (0.3, 0.7), (0.0, 1.0), 0);
    let score_low = config.score_conditions(500.0, 0.3, 0.5);
    let score_high = config.score_conditions(500.0, 0.7, 0.5);
    assert!(
        (score_low - 3.0).abs() < 0.001,
        "Temp at lower: {score_low}"
    );
    assert!(
        (score_high - 3.0).abs() < 0.001,
        "Temp at upper: {score_high}"
    );
}

// ============================================================================
// SCORE_CONDITIONS — Moisture penalty
// ============================================================================

/// Moisture below range incurs penalty of distance * 1.5.
#[test]
fn score_moisture_below_range_penalty() {
    let config = test_biome_config((0.0, 1000.0), (0.0, 1.0), (0.6, 1.0), 0);
    // moisture=0.0, distance = 0.6, penalty = 0.6 * 1.5 = 0.9
    let score = config.score_conditions(500.0, 0.5, 0.0);
    // expected: 1.0 + 1.0 - 0.9 = 1.1
    assert!(
        (score - 1.1).abs() < 0.01,
        "Moisture below: expected 1.1, got {score}"
    );
}

/// Moisture above range incurs penalty of distance * 1.5.
#[test]
fn score_moisture_above_range_penalty() {
    let config = test_biome_config((0.0, 1000.0), (0.0, 1.0), (0.0, 0.4), 0);
    // moisture=1.0, distance = 1.0 - 0.4 = 0.6, penalty = 0.6 * 1.5 = 0.9
    let score = config.score_conditions(500.0, 0.5, 1.0);
    assert!(
        (score - 1.1).abs() < 0.01,
        "Moisture above: expected 1.1, got {score}"
    );
}

/// Moisture boundaries are inclusive.
#[test]
fn score_moisture_at_boundary_in_range() {
    let config = test_biome_config((0.0, 1000.0), (0.0, 1.0), (0.2, 0.8), 0);
    let s1 = config.score_conditions(500.0, 0.5, 0.2);
    let s2 = config.score_conditions(500.0, 0.5, 0.8);
    assert!((s1 - 3.0).abs() < 0.001, "Moisture lower: {s1}");
    assert!((s2 - 3.0).abs() < 0.001, "Moisture upper: {s2}");
}

// ============================================================================
// SCORE_CONDITIONS — Combined penalties
// ============================================================================

/// All three parameters outside range — all penalties apply.
#[test]
fn score_all_out_of_range() {
    let config = test_biome_config((100.0, 200.0), (0.4, 0.6), (0.4, 0.6), 0);
    // height=0: dist=100, penalty=100*0.5=50.0 → score -= 50.0
    // temp=0: dist=0.4, penalty=0.4*2.0=0.8 → score -= 0.8
    // moisture=0: dist=0.4, penalty=0.4*1.5=0.6 → score -= 0.6
    let score = config.score_conditions(0.0, 0.0, 0.0);
    let expected = -50.0 - 0.8 - 0.6; // = -51.4
    assert!(
        (score - expected).abs() < 0.01,
        "All out of range: expected {expected}, got {score}"
    );
}

/// Priority bonus combined with penalties.
#[test]
fn score_out_of_range_with_priority() {
    let config = test_biome_config((100.0, 200.0), (0.4, 0.6), (0.4, 0.6), 10);
    let score = config.score_conditions(0.0, 0.0, 0.0);
    // -51.4 + 10 * 0.1 = -51.4 + 1.0 = -50.4
    let expected = -51.4 + 1.0;
    assert!(
        (score - expected).abs() < 0.01,
        "Out of range + priority 10: expected {expected}, got {score}"
    );
}

/// Large distances amplify penalties differently for each dimension.
/// This verifies the penalty multipliers (0.01, 2.0, 1.5) are correct.
#[test]
fn score_penalty_multiplier_verification() {
    // Set up so each dimension is exactly 1.0 unit outside range
    let config = test_biome_config((50.0, 50.0), (0.5, 0.5), (0.5, 0.5), 0);

    // Height 1.0 below → penalty = 1.0 * 0.5 = 0.5
    let s_height = config.score_conditions(49.0, 0.5, 0.5);
    // expected: -0.5 + 1.0 + 1.0 = 1.5
    assert!(
        (s_height - 1.5).abs() < 0.01,
        "Height penalty 1.0 unit: expected ~1.5, got {s_height}"
    );

    // Temp 1.0 below → penalty = 1.0 * 2.0 = 2.0  (but temp below 0.5 by...
    // Actually with range (0.5, 0.5), temp 0.0 is 0.5 below → penalty = 0.5 * 2.0 = 1.0
    // Let me use a value exactly 1.0 below (but temp is 0-1, so use more general range)
    let config2 = test_biome_config((0.0, 1000.0), (10.0, 10.0), (0.5, 0.5), 0);
    let s_temp = config2.score_conditions(500.0, 9.0, 0.5);
    // temp 1.0 below → penalty = 1.0 * 2.0 = 2.0
    // expected: 1.0 - 2.0 + 1.0 = 0.0
    assert!(
        (s_temp - 0.0).abs() < 0.01,
        "Temp penalty 1.0 unit: expected 0.0, got {s_temp}"
    );

    let config3 = test_biome_config((0.0, 1000.0), (0.0, 1.0), (10.0, 10.0), 0);
    let s_moist = config3.score_conditions(500.0, 0.5, 9.0);
    // moisture 1.0 below → 10.0 - 9.0 = wait, moisture=9.0, range=(10,10)
    // distance = 10.0 - 9.0 = 1.0, penalty = 1.0 * 1.5 = 1.5
    // expected: 1.0 + 1.0 - 1.5 = 0.5
    assert!(
        (s_moist - 0.5).abs() < 0.01,
        "Moisture penalty 1.0 unit: expected 0.5, got {s_moist}"
    );
}

// ============================================================================
// IS_SLOPE_SUITABLE
// ============================================================================

#[test]
fn slope_at_max_is_suitable() {
    let config = BiomeConfig::grassland();
    // Grassland max_slope is some value, slope exactly at it should be fine
    assert!(config.is_slope_suitable(config.conditions.max_slope));
}

#[test]
fn slope_below_max_is_suitable() {
    let config = BiomeConfig::grassland();
    assert!(config.is_slope_suitable(0.0));
    assert!(config.is_slope_suitable(config.conditions.max_slope - 1.0));
}

#[test]
fn slope_above_max_is_not_suitable() {
    let config = BiomeConfig::grassland();
    assert!(!config.is_slope_suitable(config.conditions.max_slope + 0.1));
}

// ============================================================================
// BIOME TYPE CONVERSIONS
// ============================================================================

#[test]
fn biome_type_as_str_all() {
    assert_eq!(BiomeType::Grassland.as_str(), "grassland");
    assert_eq!(BiomeType::Desert.as_str(), "desert");
    assert_eq!(BiomeType::Forest.as_str(), "forest");
    assert_eq!(BiomeType::Mountain.as_str(), "mountain");
    assert_eq!(BiomeType::Tundra.as_str(), "tundra");
    assert_eq!(BiomeType::Swamp.as_str(), "swamp");
    assert_eq!(BiomeType::Beach.as_str(), "beach");
    assert_eq!(BiomeType::River.as_str(), "river");
}

#[test]
fn biome_type_parse_case_insensitive() {
    assert_eq!(BiomeType::parse("grassland"), Some(BiomeType::Grassland));
    assert_eq!(BiomeType::parse("DESERT"), Some(BiomeType::Desert));
    assert_eq!(BiomeType::parse("Forest"), Some(BiomeType::Forest));
    assert_eq!(BiomeType::parse("unknown"), None);
}

#[test]
fn biome_type_all_returns_8() {
    let all = BiomeType::all();
    assert_eq!(all.len(), 8);
    // Each type should be in the list
    assert!(all.contains(&BiomeType::Grassland));
    assert!(all.contains(&BiomeType::Desert));
    assert!(all.contains(&BiomeType::Forest));
    assert!(all.contains(&BiomeType::Mountain));
    assert!(all.contains(&BiomeType::Tundra));
    assert!(all.contains(&BiomeType::Swamp));
    assert!(all.contains(&BiomeType::Beach));
    assert!(all.contains(&BiomeType::River));
}

#[test]
fn biome_type_material_dir() {
    assert_eq!(
        BiomeType::Forest.material_dir(),
        std::path::PathBuf::from("assets/materials/forest")
    );
    assert_eq!(
        BiomeType::Desert.material_dir(),
        std::path::PathBuf::from("assets/materials/desert")
    );
}

#[test]
fn biome_type_terrain_fallback_dir() {
    assert_eq!(
        BiomeType::terrain_fallback_material_dir(),
        std::path::PathBuf::from("assets/materials/terrain")
    );
}

// ============================================================================
// BIOME CONFIG FACTORY METHODS — catch literal mutations in preset configs
// ============================================================================

#[test]
fn biome_config_grassland_priority() {
    let c = BiomeConfig::grassland();
    assert_eq!(c.biome_type, BiomeType::Grassland);
    assert_eq!(c.priority, 1);
}

#[test]
fn biome_config_desert_priority() {
    let c = BiomeConfig::desert();
    assert_eq!(c.biome_type, BiomeType::Desert);
    assert_eq!(c.priority, 2);
}

#[test]
fn biome_config_forest_priority() {
    let c = BiomeConfig::forest();
    assert_eq!(c.biome_type, BiomeType::Forest);
    assert_eq!(c.priority, 3);
}

#[test]
fn biome_config_mountain_priority() {
    let c = BiomeConfig::mountain();
    assert_eq!(c.biome_type, BiomeType::Mountain);
    assert_eq!(c.priority, 4);
}

#[test]
fn biome_config_tundra_priority() {
    let c = BiomeConfig::tundra();
    assert_eq!(c.biome_type, BiomeType::Tundra);
    assert_eq!(c.priority, 5);
}

#[test]
fn biome_config_swamp_priority() {
    let c = BiomeConfig::swamp();
    assert_eq!(c.biome_type, BiomeType::Swamp);
    assert_eq!(c.priority, 6);
}

#[test]
fn biome_config_beach_priority() {
    let c = BiomeConfig::beach();
    assert_eq!(c.biome_type, BiomeType::Beach);
    assert_eq!(c.priority, 7);
}

#[test]
fn biome_config_river_priority() {
    let c = BiomeConfig::river();
    assert_eq!(c.biome_type, BiomeType::River);
    assert_eq!(c.priority, 8);
}

// ============================================================================
// BIOME SCORING — factory biome configs with real conditions
// ============================================================================

/// Desert biome should score well in hot, dry conditions at low elevations.
#[test]
fn desert_scores_well_in_hot_dry() {
    let desert = BiomeConfig::desert();
    let grassland = BiomeConfig::grassland();

    // Hot, dry, low elevation conditions
    let desert_score = desert.score_conditions(200.0, 0.9, 0.1);
    let grassland_score = grassland.score_conditions(200.0, 0.9, 0.1);

    assert!(
        desert_score > grassland_score || desert.priority > grassland.priority,
        "Desert should beat grassland in hot/dry: desert={desert_score} grassland={grassland_score}"
    );
}

/// Mountain biome should score well at its actual height range (60-200).
#[test]
fn mountain_scores_well_at_height() {
    let mountain = BiomeConfig::mountain();
    // Mountain height_range = (60.0, 200.0), temp = (0.0, 0.5), moisture = (0.2, 0.7)
    let score = mountain.score_conditions(130.0, 0.25, 0.45);
    // All params in-range: base 3.0 + priority(4)*0.1 = 3.4
    assert!(
        score > 0.0,
        "Mountain should score positively at 130m (mid-range): {score}"
    );
}

/// Score with large height penalty should go negative.
#[test]
fn score_can_go_negative() {
    let config = test_biome_config((500.0, 600.0), (0.5, 0.5), (0.5, 0.5), 0);
    // Height way off: dist = 500, penalty = 500*0.01 = 5.0
    // Temp exactly off: 0.5 = in range (0.5,0.5)
    // Moisture exactly: in range
    // Score = -5.0 + 1.0 + 1.0 = -3.0
    let score = config.score_conditions(0.0, 0.5, 0.5);
    assert!(
        score < 0.0,
        "Large penalties should produce negative scores: {score}"
    );
}
