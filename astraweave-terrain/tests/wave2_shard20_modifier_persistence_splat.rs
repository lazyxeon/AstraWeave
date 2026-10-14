//! Splat rule boundaries. Modifier and persistence tests left with those modules (U4).

use astraweave_terrain::texture_splatting::SplatRule;

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
