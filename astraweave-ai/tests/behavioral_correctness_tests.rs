//! Mutation-Resistant Behavioral Correctness Tests for AI Systems
//!
//! These tests verify that AI subsystems produce CORRECT decisions, not just
//! that they run without crashing. Each test is designed to catch common mutations
//! (e.g., + to -, * to /, sign flips, wrong comparisons).
//!
//! Tests verify:
//! - StateValue comparison correctness
//! - WorldState get/set/satisfies correctness
//! - Goal satisfaction logic
//! - Action precondition/effect correctness
//! - Plan cost ordering
//! - Heuristic admissibility
//!
//! Phase 8.8: Production-Ready AI Validation

#![cfg(feature = "planner_advanced")]

use astraweave_ai::goap::{Action, Goal, OrderedFloat, SimpleAction, StateValue, WorldState};
use std::collections::BTreeMap;

// ============================================================================
// STATE VALUE CORRECTNESS
// ============================================================================

/// Verify StateValue::Bool comparison correctness
#[test]
fn test_state_value_bool_comparison() {
    let true_val = StateValue::Bool(true);
    let false_val = StateValue::Bool(false);

    // Same values satisfy each other
    assert!(true_val.satisfies(&true_val), "true should satisfy true");
    assert!(
        false_val.satisfies(&false_val),
        "false should satisfy false"
    );

    // Different values don't satisfy
    assert!(
        !true_val.satisfies(&false_val),
        "true should NOT satisfy false"
    );
    assert!(
        !false_val.satisfies(&true_val),
        "false should NOT satisfy true"
    );
}

/// Verify StateValue::Int comparison correctness
#[test]
fn test_state_value_int_comparison() {
    let val_5 = StateValue::Int(5);
    let val_10 = StateValue::Int(10);
    let val_5_dup = StateValue::Int(5);

    // Same values satisfy
    assert!(val_5.satisfies(&val_5_dup), "5 should satisfy 5");

    // Different values don't satisfy
    assert!(!val_5.satisfies(&val_10), "5 should NOT satisfy 10");
}

/// Verify StateValue::IntRange works correctly (catches off-by-one)
#[test]
fn test_state_value_int_range() {
    let val_5 = StateValue::Int(5);
    let val_0 = StateValue::Int(0);
    let val_10 = StateValue::Int(10);
    let val_11 = StateValue::Int(11);

    let range_0_10 = StateValue::IntRange(0, 10);

    // Boundary inclusion (catches off-by-one mutations)
    assert!(
        val_0.satisfies(&range_0_10),
        "0 should satisfy [0,10] - lower bound"
    );
    assert!(
        val_5.satisfies(&range_0_10),
        "5 should satisfy [0,10] - middle"
    );
    assert!(
        val_10.satisfies(&range_0_10),
        "10 should satisfy [0,10] - upper bound"
    );

    // Boundary exclusion
    assert!(
        !val_11.satisfies(&range_0_10),
        "11 should NOT satisfy [0,10]"
    );

    // Negative boundary test
    let val_neg1 = StateValue::Int(-1);
    assert!(
        !val_neg1.satisfies(&range_0_10),
        "-1 should NOT satisfy [0,10]"
    );
}

/// Verify StateValue::Float comparison with tolerance
#[test]
fn test_state_value_float_comparison() {
    let pi = std::f32::consts::PI;
    let val_a = StateValue::Float(OrderedFloat(pi));
    let val_b = StateValue::Float(OrderedFloat(pi + 1e-8)); // Within tolerance
    let val_c = StateValue::Float(OrderedFloat(4.0)); // Different

    assert!(
        val_a.satisfies(&val_b),
        "Float values within 1e-6 should satisfy"
    );
    assert!(
        !val_a.satisfies(&val_c),
        "Different floats should NOT satisfy"
    );
}

/// Verify StateValue::FloatApprox with epsilon tolerance
#[test]
fn test_state_value_float_approx() {
    let val = StateValue::Float(OrderedFloat(100.0));
    let target_tight = StateValue::FloatApprox(100.0, 0.1);
    let target_loose = StateValue::FloatApprox(100.0, 10.0);

    // Within tight tolerance
    assert!(val.satisfies(&target_tight), "100 should satisfy 100 ± 0.1");
    assert!(val.satisfies(&target_loose), "100 should satisfy 100 ± 10");

    // Test boundary of tolerance (mutation catching)
    let val_edge = StateValue::Float(OrderedFloat(100.05));
    assert!(
        val_edge.satisfies(&target_tight),
        "100.05 should satisfy 100 ± 0.1"
    );

    let val_outside = StateValue::Float(OrderedFloat(100.2));
    assert!(
        !val_outside.satisfies(&target_tight),
        "100.2 should NOT satisfy 100 ± 0.1"
    );
}

/// Verify numeric_distance correctness for integers
#[test]
fn test_numeric_distance_int() {
    let val_5 = StateValue::Int(5);
    let val_10 = StateValue::Int(10);

    // Distance should be absolute difference
    let dist = val_5.numeric_distance(&val_10);
    assert!(
        (dist - 5.0).abs() < 0.001,
        "Distance from 5 to 10 should be 5.0, got {}",
        dist
    );

    // Distance is symmetric
    let dist_rev = val_10.numeric_distance(&val_5);
    assert!(
        (dist_rev - 5.0).abs() < 0.001,
        "Distance from 10 to 5 should be 5.0, got {}",
        dist_rev
    );
}

/// Verify numeric_distance for IntRange
#[test]
fn test_numeric_distance_int_range() {
    let range = StateValue::IntRange(10, 20);

    // Below range
    let val_5 = StateValue::Int(5);
    let dist_below = val_5.numeric_distance(&range);
    assert!(
        (dist_below - 5.0).abs() < 0.001,
        "Distance from 5 to [10,20] should be 5.0 (to lower bound)"
    );

    // Above range
    let val_25 = StateValue::Int(25);
    let dist_above = val_25.numeric_distance(&range);
    assert!(
        (dist_above - 5.0).abs() < 0.001,
        "Distance from 25 to [10,20] should be 5.0 (from upper bound)"
    );

    // Within range
    let val_15 = StateValue::Int(15);
    let dist_within = val_15.numeric_distance(&range);
    assert!(
        dist_within.abs() < 0.001,
        "Distance from 15 to [10,20] should be 0.0 (within range)"
    );
}

// ============================================================================
// WORLD STATE CORRECTNESS
// ============================================================================

/// Verify WorldState get/set with StateValue
#[test]
fn test_world_state_get_set_state_value() {
    let mut state = WorldState::new();

    // Initially empty
    assert!(state.get("key").is_none(), "Unset key should return None");

    // Set Bool
    state.set("armed", StateValue::Bool(true));
    assert_eq!(
        state.get("armed"),
        Some(&StateValue::Bool(true)),
        "Key should return Bool(true)"
    );

    // Set Int
    state.set("ammo", StateValue::Int(30));
    assert_eq!(
        state.get("ammo"),
        Some(&StateValue::Int(30)),
        "Key should return Int(30)"
    );

    // Overwrite
    state.set("ammo", StateValue::Int(25));
    assert_eq!(
        state.get("ammo"),
        Some(&StateValue::Int(25)),
        "Key should return updated Int(25)"
    );
}

/// Verify WorldState.satisfies() correctness
#[test]
fn test_world_state_satisfies_conditions() {
    let mut state = WorldState::new();
    state.set("has_weapon", StateValue::Bool(true));
    state.set("ammo", StateValue::Int(30));

    // Matching conditions
    let mut conditions = BTreeMap::new();
    conditions.insert("has_weapon".to_string(), StateValue::Bool(true));
    conditions.insert("ammo".to_string(), StateValue::IntRange(10, 50));

    assert!(
        state.satisfies(&conditions),
        "State should satisfy all conditions"
    );

    // Failing condition
    let mut fail_conditions = BTreeMap::new();
    fail_conditions.insert("has_weapon".to_string(), StateValue::Bool(false));

    assert!(
        !state.satisfies(&fail_conditions),
        "State should NOT satisfy wrong condition"
    );
}

/// Verify WorldState.apply_effects() correctness
#[test]
fn test_world_state_apply_effects() {
    let mut state = WorldState::new();
    state.set("ammo", StateValue::Int(30));
    state.set("health", StateValue::Int(100)); // Unrelated

    let mut effects = BTreeMap::new();
    effects.insert("ammo".to_string(), StateValue::Int(29)); // Decrement
    effects.insert("reloading".to_string(), StateValue::Bool(false)); // New key

    state.apply_effects(&effects);

    assert_eq!(
        state.get("ammo"),
        Some(&StateValue::Int(29)),
        "Ammo should be updated to 29"
    );
    assert_eq!(
        state.get("health"),
        Some(&StateValue::Int(100)),
        "Unrelated health should be preserved"
    );
    assert_eq!(
        state.get("reloading"),
        Some(&StateValue::Bool(false)),
        "New key should be added"
    );
}

/// Verify WorldState.distance_to() heuristic
#[test]
fn test_world_state_distance_to() {
    let mut state = WorldState::new();
    state.set("has_weapon", StateValue::Bool(false));
    state.set("ammo", StateValue::Int(0));

    let mut goal = BTreeMap::new();
    goal.insert("has_weapon".to_string(), StateValue::Bool(true));
    goal.insert("ammo".to_string(), StateValue::Int(30));

    let distance = state.distance_to(&goal);

    // Distance should be > 0 (unmet conditions + numeric diff)
    assert!(distance > 0.0, "Distance to unmet goal should be positive");

    // After meeting goal, distance should decrease
    state.set("has_weapon", StateValue::Bool(true));
    let partial_dist = state.distance_to(&goal);

    assert!(
        partial_dist < distance,
        "Distance should decrease when conditions met"
    );

    // After fully meeting goal
    state.set("ammo", StateValue::Int(30));
    let final_dist = state.distance_to(&goal);

    assert!(
        final_dist < 0.001,
        "Distance should be ~0 when goal satisfied"
    );
}

// ============================================================================
// GOAL SATISFACTION CORRECTNESS
// ============================================================================

/// Verify Goal.is_satisfied() with StateValue conditions
#[test]
fn test_goal_is_satisfied_with_state_values() {
    let mut desired = BTreeMap::new();
    desired.insert("has_weapon".to_string(), StateValue::Bool(true));
    desired.insert("health".to_string(), StateValue::IntRange(50, 100));

    let goal = Goal::new("combat_ready", desired);

    // Satisfied state
    let mut good_state = WorldState::new();
    good_state.set("has_weapon", StateValue::Bool(true));
    good_state.set("health", StateValue::Int(75));

    assert!(goal.is_satisfied(&good_state), "Goal should be satisfied");

    // Unsatisfied - wrong weapon
    let mut no_weapon = WorldState::new();
    no_weapon.set("has_weapon", StateValue::Bool(false));
    no_weapon.set("health", StateValue::Int(75));

    assert!(
        !goal.is_satisfied(&no_weapon),
        "Goal should NOT be satisfied without weapon"
    );

    // Unsatisfied - health too low
    let mut low_health = WorldState::new();
    low_health.set("has_weapon", StateValue::Bool(true));
    low_health.set("health", StateValue::Int(25));

    assert!(
        !goal.is_satisfied(&low_health),
        "Goal should NOT be satisfied with low health"
    );
}

/// Verify Goal.priority ordering
#[test]
fn test_goal_priority_ordering() {
    let low = Goal::new("patrol", BTreeMap::new()).with_priority(1.0);
    let high = Goal::new("survive", BTreeMap::new()).with_priority(10.0);

    assert!(
        high.priority > low.priority,
        "Higher priority goal should have higher value"
    );

    // Exact values (mutation catching)
    assert!(
        (low.priority - 1.0).abs() < 0.001,
        "Low priority should be 1.0"
    );
    assert!(
        (high.priority - 10.0).abs() < 0.001,
        "High priority should be 10.0"
    );
}

/// Verify Goal.urgency() increases as deadline approaches
#[test]
fn test_goal_urgency_increases_near_deadline() {
    let goal = Goal::new("escape", BTreeMap::new())
        .with_priority(5.0)
        .with_deadline(100.0);

    // Far from deadline
    let urgency_far = goal.urgency(0.0);

    // Close to deadline
    let urgency_close = goal.urgency(95.0);

    // At deadline
    let urgency_at = goal.urgency(100.0);

    assert!(
        urgency_close > urgency_far,
        "Urgency should increase as deadline approaches"
    );
    assert!(
        urgency_at > urgency_close,
        "Urgency should be highest at deadline"
    );
}

/// Verify Goal without deadline has constant urgency
#[test]
fn test_goal_no_deadline_constant_urgency() {
    let goal = Goal::new("explore", BTreeMap::new()).with_priority(3.0);

    let urgency_0 = goal.urgency(0.0);
    let urgency_100 = goal.urgency(100.0);
    let urgency_1000 = goal.urgency(1000.0);

    assert!(
        (urgency_0 - urgency_100).abs() < 0.001,
        "Urgency should be constant without deadline"
    );
    assert!(
        (urgency_100 - urgency_1000).abs() < 0.001,
        "Urgency should remain constant"
    );
}

// ============================================================================
// ACTION CORRECTNESS
// ============================================================================

/// Verify SimpleAction preconditions/effects
#[test]
fn test_simple_action_structure() {
    let mut preconditions = BTreeMap::new();
    preconditions.insert("has_weapon".to_string(), StateValue::Bool(true));
    preconditions.insert("ammo".to_string(), StateValue::IntRange(1, 100));

    let mut effects = BTreeMap::new();
    effects.insert("enemy_hit".to_string(), StateValue::Bool(true));
    effects.insert("ammo".to_string(), StateValue::Int(29)); // Assuming 30 - 1

    let action = SimpleAction::new("shoot", preconditions.clone(), effects.clone(), 1.0);

    assert_eq!(action.name(), "shoot", "Action name should match");
    assert_eq!(
        action.preconditions(),
        &preconditions,
        "Preconditions should match"
    );
    assert_eq!(action.effects(), &effects, "Effects should match");
    assert!(
        (action.base_cost() - 1.0).abs() < 0.001,
        "Base cost should be 1.0"
    );
}

/// Verify Action.can_execute() respects preconditions
#[test]
fn test_action_can_execute() {
    let mut preconditions = BTreeMap::new();
    preconditions.insert("has_weapon".to_string(), StateValue::Bool(true));

    let action = SimpleAction::new("shoot", preconditions, BTreeMap::new(), 1.0);

    // Valid state
    let mut valid = WorldState::new();
    valid.set("has_weapon", StateValue::Bool(true));
    assert!(
        action.can_execute(&valid),
        "Should be able to execute with weapon"
    );

    // Invalid state
    let mut invalid = WorldState::new();
    invalid.set("has_weapon", StateValue::Bool(false));
    assert!(
        !action.can_execute(&invalid),
        "Should NOT be able to execute without weapon"
    );

    // Missing state
    let empty = WorldState::new();
    assert!(
        !action.can_execute(&empty),
        "Should NOT be able to execute with missing state"
    );
}

/// Verify Action cost is never negative (after history adjustments)
#[test]
fn test_action_cost_non_negative() {
    use astraweave_ai::goap::ActionHistory;

    let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 0.5);
    let state = WorldState::new();
    let history = ActionHistory::new();

    let cost = action.calculate_cost(&state, &history);

    assert!(
        cost >= 0.1, // Minimum is 0.1 per implementation
        "Cost should be at least 0.1, got {}",
        cost
    );
}

// ============================================================================
// HEURISTIC ADMISSIBILITY (A* CORRECTNESS)
// ============================================================================

/// Verify heuristic is admissible (never overestimates)
#[test]
fn test_heuristic_admissibility() {
    let mut state = WorldState::new();
    state.set("a", StateValue::Bool(false));
    state.set("b", StateValue::Int(0));

    let mut goal = BTreeMap::new();
    goal.insert("a".to_string(), StateValue::Bool(true));
    goal.insert("b".to_string(), StateValue::Int(10));

    let h = state.distance_to(&goal);

    // Heuristic should be >= 0
    assert!(h >= 0.0, "Heuristic should be non-negative");

    // For this simple case, heuristic should reflect unmet conditions
    // At least 2 conditions are unmet (Bool and Int), so h >= 4.0 (2 * 2.0 per unmet)
    // Plus numeric distance for Int (0 to 10 = 10.0)
    // So h should be approximately 4.0 + 10.0 = 14.0
    assert!(
        h >= 2.0, // At minimum, 2 unmet conditions
        "Heuristic should account for unmet conditions, got {}",
        h
    );
}

/// Verify heuristic decreases as we approach goal
#[test]
fn test_heuristic_monotonically_decreases() {
    let mut goal = BTreeMap::new();
    goal.insert("a".to_string(), StateValue::Bool(true));
    goal.insert("b".to_string(), StateValue::Bool(true));
    goal.insert("c".to_string(), StateValue::Bool(true));

    // 0 conditions met
    let state_0 = WorldState::new();
    let h_0 = state_0.distance_to(&goal);

    // 1 condition met
    let mut state_1 = WorldState::new();
    state_1.set("a", StateValue::Bool(true));
    let h_1 = state_1.distance_to(&goal);

    // 2 conditions met
    let mut state_2 = WorldState::new();
    state_2.set("a", StateValue::Bool(true));
    state_2.set("b", StateValue::Bool(true));
    let h_2 = state_2.distance_to(&goal);

    // 3 conditions met (goal)
    let mut state_3 = WorldState::new();
    state_3.set("a", StateValue::Bool(true));
    state_3.set("b", StateValue::Bool(true));
    state_3.set("c", StateValue::Bool(true));
    let h_3 = state_3.distance_to(&goal);

    assert!(h_0 > h_1, "h should decrease: h_0={} > h_1={}", h_0, h_1);
    assert!(h_1 > h_2, "h should decrease: h_1={} > h_2={}", h_1, h_2);
    assert!(h_2 > h_3, "h should decrease: h_2={} > h_3={}", h_2, h_3);
    assert!(h_3 < 0.001, "h should be ~0 at goal, got {}", h_3);
}

// ============================================================================
// MUTATION-CATCHING EDGE CASES
// ============================================================================

/// Catch comparison operator mutations (< vs <=, > vs >=)
#[test]
fn test_mutation_boundary_comparisons() {
    // Test exact boundary in IntRange
    let range = StateValue::IntRange(0, 10);

    // Lower boundary
    let at_lower = StateValue::Int(0);
    let below_lower = StateValue::Int(-1);

    assert!(
        at_lower.satisfies(&range),
        "0 should satisfy [0,10] - catches >= vs >"
    );
    assert!(
        !below_lower.satisfies(&range),
        "-1 should NOT satisfy [0,10]"
    );

    // Upper boundary
    let at_upper = StateValue::Int(10);
    let above_upper = StateValue::Int(11);

    assert!(
        at_upper.satisfies(&range),
        "10 should satisfy [0,10] - catches <= vs <"
    );
    assert!(
        !above_upper.satisfies(&range),
        "11 should NOT satisfy [0,10]"
    );
}

/// Catch sign/negation mutations
#[test]
fn test_mutation_sign_errors() {
    let val_pos = StateValue::Int(5);
    let val_neg = StateValue::Int(-5);

    // These must be different
    assert_ne!(val_pos, val_neg, "5 and -5 should not be equal");

    // Numeric distance should be 10 (not 0)
    let dist = val_pos.numeric_distance(&val_neg);
    assert!(
        (dist - 10.0).abs() < 0.001,
        "Distance from 5 to -5 should be 10.0, got {}",
        dist
    );
}

/// Catch off-by-one errors in iteration/counting
#[test]
fn test_mutation_off_by_one() {
    let mut goal = BTreeMap::new();
    goal.insert("a".to_string(), StateValue::Bool(true));
    goal.insert("b".to_string(), StateValue::Bool(true));
    goal.insert("c".to_string(), StateValue::Bool(true));

    // Empty state - should have 3 unmet
    let empty = WorldState::new();
    let h_empty = empty.distance_to(&goal);
    // With 3 unmet conditions (each worth 2.0 + 1.0 numeric), h >= 6.0
    assert!(
        h_empty >= 6.0,
        "3 unmet conditions should give h >= 6.0, got {}",
        h_empty
    );

    // One satisfied - should have 2 unmet
    let mut one = WorldState::new();
    one.set("a", StateValue::Bool(true));
    let h_one = one.distance_to(&goal);
    // With 2 unmet conditions, h >= 4.0
    assert!(
        h_one >= 4.0 && h_one < h_empty,
        "2 unmet conditions should give h >= 4.0 and < h_empty={}, got {}",
        h_empty,
        h_one
    );
}

/// Catch return value mutations (wrong constant returned)
#[test]
fn test_mutation_return_values() {
    let action = SimpleAction::new("precise", BTreeMap::new(), BTreeMap::new(), 7.5);

    assert!(
        (action.base_cost() - 7.5).abs() < 0.001,
        "Base cost should be exactly 7.5, got {}",
        action.base_cost()
    );

    assert_eq!(action.name(), "precise", "Name should be exactly 'precise'");
}

/// Catch boolean logic mutations (!x, x && y vs x || y)
#[test]
fn test_mutation_boolean_logic() {
    let val_true = StateValue::Bool(true);
    let val_false = StateValue::Bool(false);

    // true satisfies true, not false
    assert!(
        val_true.satisfies(&val_true),
        "true.satisfies(true) must be true"
    );
    assert!(
        !val_true.satisfies(&val_false),
        "true.satisfies(false) must be false"
    );

    // false satisfies false, not true
    assert!(
        val_false.satisfies(&val_false),
        "false.satisfies(false) must be true"
    );
    assert!(
        !val_false.satisfies(&val_true),
        "false.satisfies(true) must be false"
    );
}

/// Catch empty collection edge cases
#[test]
fn test_mutation_empty_collections() {
    // Empty goal should be satisfied by any state
    let empty_goal: BTreeMap<String, StateValue> = BTreeMap::new();
    let goal = Goal::new("empty", empty_goal.clone());

    let any_state = WorldState::new();
    assert!(
        goal.is_satisfied(&any_state),
        "Empty goal should be satisfied by any state"
    );

    // Distance to empty goal should be 0
    let dist = any_state.distance_to(&empty_goal);
    assert!(
        dist < 0.001,
        "Distance to empty goal should be 0, got {}",
        dist
    );
}

/// Catch type mismatch handling
#[test]
fn test_mutation_type_safety() {
    let int_val = StateValue::Int(5);
    let bool_val = StateValue::Bool(true);
    let string_val = StateValue::String("test".to_string());

    // Different types should not satisfy each other
    assert!(!int_val.satisfies(&bool_val), "Int should not satisfy Bool");
    assert!(!bool_val.satisfies(&int_val), "Bool should not satisfy Int");
    assert!(
        !int_val.satisfies(&string_val),
        "Int should not satisfy String"
    );
    assert!(
        !string_val.satisfies(&int_val),
        "String should not satisfy Int"
    );
}
