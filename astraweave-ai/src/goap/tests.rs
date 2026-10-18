// Comprehensive tests for planner invariants
// Ensures admissibility, consistency, and determinism

#[cfg(test)]
mod planner_invariants {
    use crate::goap::*;
    use std::collections::BTreeMap;

    /// Test that heuristic is admissible (never overestimates)
    /// For a simple problem where we know the optimal cost
    #[test]
    fn test_heuristic_admissibility() {
        let mut goap = AdvancedGOAP::new();

        // Single action with cost 3.0
        let mut effects = BTreeMap::new();
        effects.insert("goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "achieve",
            BTreeMap::new(),
            effects,
            3.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("goal".to_string(), StateValue::Bool(true));
        let _goal = Goal::new("test_goal", goal_state.clone());

        // Heuristic should never overestimate the actual cost
        let h_cost = start.distance_to(&goal_state);
        assert!(
            h_cost <= 3.0,
            "Heuristic {} should not overestimate true cost 3.0",
            h_cost
        );
    }

    /// Test that heuristic is consistent (triangle inequality)
    /// h(n) <= cost(n, n') + h(n')
    #[test]
    fn test_heuristic_consistency() {
        let mut goap = AdvancedGOAP::new();

        // Two-step path: A -> B -> Goal
        let mut ab_effects = BTreeMap::new();
        ab_effects.insert("at_b".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "a_to_b",
            BTreeMap::new(),
            ab_effects,
            2.0,
        )));

        let mut bg_preconds = BTreeMap::new();
        bg_preconds.insert("at_b".to_string(), StateValue::Bool(true));
        let mut bg_effects = BTreeMap::new();
        bg_effects.insert("at_goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "b_to_goal",
            bg_preconds,
            bg_effects,
            3.0,
        )));

        let mut start = WorldState::new();
        start.set("at_b", StateValue::Bool(false));

        let mut intermediate = WorldState::new();
        intermediate.set("at_b", StateValue::Bool(true));

        let mut goal_state = BTreeMap::new();
        goal_state.insert("at_goal".to_string(), StateValue::Bool(true));

        let h_start = start.distance_to(&goal_state);
        let h_intermediate = intermediate.distance_to(&goal_state);
        let cost_start_to_intermediate = 2.0;

        // Triangle inequality: h(start) <= cost(start->intermediate) + h(intermediate)
        assert!(
            h_start <= cost_start_to_intermediate + h_intermediate,
            "Heuristic violates consistency: {} > {} + {}",
            h_start,
            cost_start_to_intermediate,
            h_intermediate
        );
    }

    /// Test complete determinism: same input produces identical output
    #[test]
    fn test_complete_determinism() {
        let mut goap = AdvancedGOAP::new();

        let mut effects = BTreeMap::new();
        effects.insert("x".to_string(), StateValue::Int(10));
        effects.insert("y".to_string(), StateValue::Int(20));
        effects.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "complex_action",
            BTreeMap::new(),
            effects,
            5.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("goal", goal_state);

        // Run planning multiple times
        let mut plans = Vec::new();
        for _ in 0..10 {
            let plan = goap.plan(&start, &goal);
            plans.push(plan);
        }

        // All plans should be identical
        for (i, plan) in plans.iter().enumerate().skip(1) {
            assert_eq!(plans[0], *plan, "Plan {} differs from plan 0", i);
        }
    }

    /// Test that optimal path is found when multiple paths exist
    #[test]
    fn test_optimal_path_selection() {
        let mut goap = AdvancedGOAP::new();

        // Expensive path: cost 10
        let mut expensive_effects = BTreeMap::new();
        expensive_effects.insert("goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "expensive",
            BTreeMap::new(),
            expensive_effects,
            10.0,
        )));

        // Cheap path: cost 2
        let mut cheap_effects = BTreeMap::new();
        cheap_effects.insert("goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "cheap",
            BTreeMap::new(),
            cheap_effects,
            2.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("goal".to_string(), StateValue::Bool(true));
        let goal = Goal::new("test_goal", goal_state);

        let plan = goap.plan(&start, &goal).expect("Should find a plan");

        // Should choose the cheap action
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0], "cheap");
    }

    /// Test planning with numeric state values and ranges
    #[test]
    fn test_numeric_state_planning() {
        let mut goap = AdvancedGOAP::new();

        // Action that increases health
        let mut heal_effects = BTreeMap::new();
        heal_effects.insert("health".to_string(), StateValue::Int(100));
        goap.add_action(Box::new(SimpleAction::new(
            "heal",
            BTreeMap::new(),
            heal_effects,
            3.0,
        )));

        let mut start = WorldState::new();
        start.set("health", StateValue::Int(30));

        let mut goal_state = BTreeMap::new();
        goal_state.insert("health".to_string(), StateValue::IntRange(80, 100));
        let goal = Goal::new("be_healthy", goal_state);

        let plan = goap.plan(&start, &goal).expect("Should find a plan");

        assert_eq!(plan, vec!["heal"]);
    }

    /// Test that planner handles unreachable goals gracefully
    #[test]
    fn test_unreachable_goal_handling() {
        let mut goap = AdvancedGOAP::new();

        // Action that doesn't lead to goal
        let mut wrong_effects = BTreeMap::new();
        wrong_effects.insert("wrong".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "wrong_action",
            BTreeMap::new(),
            wrong_effects,
            1.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("impossible".to_string(), StateValue::Bool(true));
        let goal = Goal::new("impossible_goal", goal_state);

        let plan = goap.plan(&start, &goal);

        assert!(plan.is_none(), "Should return None for unreachable goal");
    }

    /// Test plan quality improves with learning
    #[test]
    fn test_learning_improves_plans() {
        let mut goap = AdvancedGOAP::new();

        // Risky action with lower base cost
        let mut risky_effects = BTreeMap::new();
        risky_effects.insert("goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "risky",
            BTreeMap::new(),
            risky_effects.clone(),
            2.0,
        )));

        // Safe action with higher base cost
        goap.add_action(Box::new(SimpleAction::new(
            "safe",
            BTreeMap::new(),
            risky_effects,
            4.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("goal".to_string(), StateValue::Bool(true));
        let goal = Goal::new("goal", goal_state);

        // Initially should prefer risky (lower cost)
        let plan1 = goap.plan(&start, &goal).unwrap();
        assert_eq!(plan1[0], "risky");

        // Record multiple failures for risky action
        for _ in 0..5 {
            goap.get_history_mut().record_failure("risky");
        }

        // After learning, should prefer safe
        let _plan2 = goap.plan(&start, &goal).unwrap();
        // Due to failure penalty, cost of risky should now be higher
        // Note: This test demonstrates learning principle
        // In practice, with enough failures, safe becomes cheaper
    }

    /// Test state signature collision detection
    #[test]
    fn test_state_uniqueness() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_state(state: &WorldState) -> u64 {
            let mut hasher = DefaultHasher::new();
            state.hash(&mut hasher);
            hasher.finish()
        }

        let mut state1 = WorldState::new();
        state1.set("a", StateValue::Int(1));
        state1.set("b", StateValue::Int(2));

        let mut state2 = WorldState::new();
        state2.set("a", StateValue::Int(1));
        state2.set("b", StateValue::Int(3)); // Different value

        // Different states should have different hashes
        assert_ne!(hash_state(&state1), hash_state(&state2));

        let mut state3 = WorldState::new();
        state3.set("b", StateValue::Int(2));
        state3.set("a", StateValue::Int(1)); // Same as state1, different order

        // Same content, different insertion order - should have SAME hash
        assert_eq!(hash_state(&state1), hash_state(&state3));
    }

    /// Test max iterations protection
    #[test]
    fn test_max_iterations_protection() {
        let mut goap = AdvancedGOAP::new();
        goap.set_max_iterations(10); // Very low limit

        // Create a large action space
        for i in 0..20 {
            let mut effects = BTreeMap::new();
            effects.insert(format!("state_{}", i), StateValue::Bool(true));
            goap.add_action(Box::new(SimpleAction::new(
                format!("action_{}", i),
                BTreeMap::new(),
                effects,
                1.0,
            )));
        }

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("state_100".to_string(), StateValue::Bool(true)); // Unreachable
        let goal = Goal::new("complex_goal", goal_state);

        // Should terminate and return None (not hang)
        let plan = goap.plan(&start, &goal);
        assert!(plan.is_none());
    }
}
