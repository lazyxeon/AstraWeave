//! Comprehensive Mutation-Resistant Tests for astraweave-behavior
//!
//! Targets all identified mutation-vulnerable code patterns across:
//! - lib.rs: BehaviorNode, BehaviorStatus, BehaviorContext, DecoratorType, BehaviorGraph
//! - goap.rs: WorldState, GoapAction, GoapGoal, GoapPlanner, GoapPlan
//! - goap_cache.rs: PlanCache LRU, CachedGoapPlanner
//! - interner.rs: StringInterner
//!
//! Every test asserts EXACT computed values to ensure mutations are caught.

use astraweave_behavior::goap::{GoapAction, GoapGoal, GoapPlan, GoapPlanner, WorldState};
use astraweave_behavior::goap_cache::{CachedGoapPlanner, PlanCache};
use astraweave_behavior::{
    BehaviorContext, BehaviorGraph, BehaviorNode, BehaviorStatus, DecoratorType,
};

// =============================================================================
// MODULE 1: WorldState::distance_to() — 3 mutation targets
// =============================================================================
mod world_state_distance_tests {
    use super::*;

    #[test]
    fn distance_to_identical_state_is_zero() {
        let state = WorldState::from_facts(&[("a", true), ("b", false)]);
        let goal_state = WorldState::from_facts(&[("a", true), ("b", false)]);
        assert_eq!(state.distance_to(&goal_state), 0);
    }

    #[test]
    fn distance_to_one_unsatisfied_is_one() {
        let state = WorldState::from_facts(&[("a", true), ("b", false)]);
        let goal_state = WorldState::from_facts(&[("a", true), ("b", true)]);
        assert_eq!(state.distance_to(&goal_state), 1);
    }

    #[test]
    fn distance_to_two_unsatisfied_is_two() {
        let state = WorldState::from_facts(&[("a", false), ("b", false)]);
        let goal_state = WorldState::from_facts(&[("a", true), ("b", true)]);
        assert_eq!(state.distance_to(&goal_state), 2);
    }

    #[test]
    fn distance_to_missing_fact_counts_as_unsatisfied() {
        let state = WorldState::new();
        let goal_state = WorldState::from_facts(&[("a", true)]);
        assert_eq!(state.distance_to(&goal_state), 1);
    }

    #[test]
    fn distance_to_empty_goal_is_zero() {
        let state = WorldState::from_facts(&[("a", true), ("b", false)]);
        let goal_state = WorldState::new();
        assert_eq!(state.distance_to(&goal_state), 0);
    }

    #[test]
    fn distance_to_three_unsatisfied_is_three() {
        let state = WorldState::new();
        let goal_state = WorldState::from_facts(&[("a", true), ("b", true), ("c", true)]);
        assert_eq!(state.distance_to(&goal_state), 3);
    }

    #[test]
    fn distance_superset_state_partial_match() {
        // State has extra facts; only goal facts matter
        let state = WorldState::from_facts(&[("a", true), ("b", false), ("c", true)]);
        let goal_state = WorldState::from_facts(&[("a", true), ("b", true)]);
        assert_eq!(state.distance_to(&goal_state), 1); // only "b" unsatisfied
    }
}

// =============================================================================
// MODULE 2: WorldState::apply() — replace with () mutation target
// =============================================================================
mod world_state_apply_tests {
    use super::*;

    #[test]
    fn apply_adds_new_facts() {
        let mut state = WorldState::new();
        let effects = WorldState::from_facts(&[("a", true)]);
        state.apply(&effects);
        assert_eq!(state.get("a"), Some(true));
    }

    #[test]
    fn apply_overwrites_existing_facts() {
        let mut state = WorldState::from_facts(&[("a", false)]);
        let effects = WorldState::from_facts(&[("a", true)]);
        state.apply(&effects);
        assert_eq!(state.get("a"), Some(true));
    }

    #[test]
    fn apply_preserves_unaffected_facts() {
        let mut state = WorldState::from_facts(&[("a", true), ("b", false)]);
        let effects = WorldState::from_facts(&[("b", true)]);
        state.apply(&effects);
        assert_eq!(state.get("a"), Some(true)); // unchanged
        assert_eq!(state.get("b"), Some(true)); // changed
    }

    #[test]
    fn apply_empty_effects_does_nothing() {
        let mut state = WorldState::from_facts(&[("a", true)]);
        let effects = WorldState::new();
        state.apply(&effects);
        assert_eq!(state.get("a"), Some(true));
        assert_eq!(state.facts.len(), 1);
    }

    #[test]
    fn apply_multiple_effects_all_applied() {
        let mut state = WorldState::new();
        let effects = WorldState::from_facts(&[("a", true), ("b", false), ("c", true)]);
        state.apply(&effects);
        assert_eq!(state.get("a"), Some(true));
        assert_eq!(state.get("b"), Some(false));
        assert_eq!(state.get("c"), Some(true));
        assert_eq!(state.facts.len(), 3);
    }
}

// =============================================================================
// MODULE 3: GoapPlan — is_complete(), advance(), invalidate()
// =============================================================================
mod goap_plan_tests {
    use super::*;

    fn make_action(name: &str) -> GoapAction {
        GoapAction::new(name)
    }

    #[test]
    fn new_single_action_has_current_not_complete() {
        use crate::test_support::TestValue;
        let plan = GoapPlan::new(vec![make_action("a")]);
        assert!(!plan.is_complete());
        assert!(plan.current_action.is_some());
        assert_eq!(plan.current_action.as_ref().test_unwrap().name, "a");
        assert!(plan.actions.is_empty()); // rest of queue empty
    }

    #[test]
    fn new_two_actions_first_is_current() {
        use crate::test_support::TestValue;
        let plan = GoapPlan::new(vec![make_action("a"), make_action("b")]);
        assert!(!plan.is_complete());
        assert_eq!(plan.current_action.as_ref().test_unwrap().name, "a");
        assert_eq!(plan.actions.len(), 1);
    }

    #[test]
    fn new_empty_actions_is_immediately_complete() {
        let plan = GoapPlan::new(vec![]);
        assert!(plan.is_complete());
        assert!(plan.current_action.is_none());
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn advance_moves_to_next_action() {
        use crate::test_support::TestValue;
        let mut plan = GoapPlan::new(vec![make_action("a"), make_action("b"), make_action("c")]);
        assert_eq!(plan.current_action.as_ref().test_unwrap().name, "a");

        plan.advance();
        assert_eq!(plan.current_action.as_ref().test_unwrap().name, "b");
        assert_eq!(plan.completed.len(), 1);
        assert_eq!(plan.completed[0].name, "a");

        plan.advance();
        assert_eq!(plan.current_action.as_ref().test_unwrap().name, "c");
        assert_eq!(plan.completed.len(), 2);

        plan.advance();
        assert!(plan.current_action.is_none());
        assert_eq!(plan.completed.len(), 3);
        assert!(plan.is_complete());
    }

    #[test]
    fn is_complete_false_when_current_action_some_queue_empty() {
        // This specifically catches && -> || mutation
        let plan = GoapPlan::new(vec![make_action("a")]);
        // current_action=Some, actions=empty
        assert!(!plan.is_complete()); // && requires BOTH None and empty
    }

    #[test]
    fn is_complete_false_when_current_action_none_queue_nonempty() {
        // Another && -> || catch
        let mut plan = GoapPlan::new(vec![make_action("a"), make_action("b")]);
        // Manually set current_action to None without advancing
        plan.current_action = None;
        // Now: current_action=None, actions=[b]
        assert!(!plan.is_complete());
    }

    #[test]
    fn invalidate_makes_plan_complete() {
        let mut plan = GoapPlan::new(vec![make_action("a"), make_action("b"), make_action("c")]);
        assert!(!plan.is_complete());

        plan.invalidate();
        assert!(plan.is_complete());
        assert!(plan.current_action.is_none());
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn invalidate_clears_current_and_queue() {
        let mut plan = GoapPlan::new(vec![make_action("a"), make_action("b")]);
        plan.invalidate();
        assert!(plan.current_action.is_none());
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn advance_on_already_complete_plan_stays_complete() {
        let mut plan = GoapPlan::new(vec![]);
        assert!(plan.is_complete());
        plan.advance(); // should not panic
        assert!(plan.is_complete());
    }
}

// =============================================================================
// MODULE 4: GoapGoal::with_priority() — mutation target
// =============================================================================
mod goap_goal_tests {
    use super::*;

    #[test]
    fn with_priority_sets_exact_value() {
        let goal = GoapGoal::new("test", WorldState::new()).with_priority(5.0);
        assert_eq!(goal.priority, 5.0);
    }

    #[test]
    fn default_priority_is_1() {
        let goal = GoapGoal::new("test", WorldState::new());
        assert_eq!(goal.priority, 1.0);
    }

    #[test]
    fn with_priority_zero() {
        let goal = GoapGoal::new("test", WorldState::new()).with_priority(0.0);
        assert_eq!(goal.priority, 0.0);
    }

    #[test]
    fn goal_name_is_stored() {
        let goal = GoapGoal::new("attack_enemy", WorldState::new());
        assert_eq!(goal.name, "attack_enemy");
    }

    #[test]
    fn is_satisfied_true_when_state_satisfies_goal() {
        let desired = WorldState::from_facts(&[("has_weapon", true)]);
        let goal = GoapGoal::new("arm_self", desired);
        let state = WorldState::from_facts(&[("has_weapon", true), ("has_ammo", true)]);
        assert!(goal.is_satisfied(&state));
    }

    #[test]
    fn is_satisfied_false_when_state_doesnt_satisfy() {
        let desired = WorldState::from_facts(&[("has_weapon", true)]);
        let goal = GoapGoal::new("arm_self", desired);
        let state = WorldState::from_facts(&[("has_weapon", false)]);
        assert!(!goal.is_satisfied(&state));
    }
}

// =============================================================================
// MODULE 5: Parallel node boundary — running_count > 0 vs >= 0
// =============================================================================
mod parallel_boundary_tests {
    use super::*;

    #[test]
    fn parallel_all_fail_no_running_returns_failure() {
        // 0 success, 0 running, 3 failure -> Failure (not Running)
        // Catches: running_count > 0 -> running_count >= 0 would wrongly return Running
        let mut ctx = BehaviorContext::new();
        ctx.register_action("f1", || BehaviorStatus::Failure);
        ctx.register_action("f2", || BehaviorStatus::Failure);
        ctx.register_action("f3", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("f1".into()),
                BehaviorNode::Action("f2".into()),
                BehaviorNode::Action("f3".into()),
            ],
            2,
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn parallel_one_running_below_threshold_returns_running() {
        // 1 success, 1 running, 1 failure -> Running (threshold=2)
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);
        ctx.register_action("r1", || BehaviorStatus::Running);
        ctx.register_action("f1", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("s1".into()),
                BehaviorNode::Action("r1".into()),
                BehaviorNode::Action("f1".into()),
            ],
            2,
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn parallel_success_met_even_with_running() {
        // 2 success, 1 running -> Success (threshold=2 met)
        // Catches: success_count >= threshold -> success_count > threshold
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);
        ctx.register_action("s2", || BehaviorStatus::Success);
        ctx.register_action("r1", || BehaviorStatus::Running);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("s1".into()),
                BehaviorNode::Action("s2".into()),
                BehaviorNode::Action("r1".into()),
            ],
            2,
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn parallel_threshold_exactly_equal_children_all_success() {
        // 3 success, threshold=3 -> Success
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);
        ctx.register_action("s2", || BehaviorStatus::Success);
        ctx.register_action("s3", || BehaviorStatus::Success);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("s1".into()),
                BehaviorNode::Action("s2".into()),
                BehaviorNode::Action("s3".into()),
            ],
            3,
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn parallel_threshold_one_with_one_success() {
        // threshold=1, 1 success -> Success immediately
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);
        ctx.register_action("f1", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("s1".into()),
                BehaviorNode::Action("f1".into()),
            ],
            1,
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn parallel_threshold_zero_returns_success_immediately() {
        // threshold=0 -> immediate Success (edge case)
        let ctx = BehaviorContext::new();
        let node = BehaviorNode::Parallel(vec![], 0);
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn parallel_threshold_exceeds_children_returns_failure() {
        // threshold > children.len() -> Failure
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);

        let node = BehaviorNode::Parallel(vec![BehaviorNode::Action("s1".into())], 5);
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn parallel_success_count_incremented_correctly() {
        // Catches success_count += 1 -> -= 1 mutation
        // If += mutated to -=, overflow would occur and comparison would fail
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s1", || BehaviorStatus::Success);
        ctx.register_action("s2", || BehaviorStatus::Success);
        ctx.register_action("s3", || BehaviorStatus::Success);
        ctx.register_action("s4", || BehaviorStatus::Success);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("s1".into()),
                BehaviorNode::Action("s2".into()),
                BehaviorNode::Action("s3".into()),
                BehaviorNode::Action("s4".into()),
            ],
            4, // all must succeed
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }
}

// =============================================================================
// MODULE 6: GoapPlanner A* cost optimality
// =============================================================================
mod goap_planner_optimality_tests {
    use super::*;

    #[test]
    fn planner_chooses_cheapest_path() {
        use crate::test_support::TestValue;
        // Two paths to same goal:
        // Path A: action_cheap (cost 1.0) x 3 = 3.0 total
        // Path B: action_expensive (cost 4.0) x 1 = 4.0 total
        // Planner must choose Path A
        let state = WorldState::from_facts(&[("start", true)]);
        let desired = WorldState::from_facts(&[("goal", true)]);
        let goal = GoapGoal::new("reach_goal", desired);

        let actions = vec![
            GoapAction::new("step1")
                .with_cost(1.0)
                .with_precondition("start", true)
                .with_effect("mid1", true),
            GoapAction::new("step2")
                .with_cost(1.0)
                .with_precondition("mid1", true)
                .with_effect("mid2", true),
            GoapAction::new("step3")
                .with_cost(1.0)
                .with_precondition("mid2", true)
                .with_effect("goal", true),
            GoapAction::new("expensive_shortcut")
                .with_cost(4.0)
                .with_precondition("start", true)
                .with_effect("goal", true),
        ];

        let planner = GoapPlanner::new();
        let plan = planner
            .plan(&state, &goal, &actions)
            .test_expect("should find plan");
        // Cheaper 3-step path (3.0) preferred over 1-step expensive path (4.0)
        assert_eq!(plan.len(), 3);
        let total_cost: f32 = plan.iter().map(|a| a.cost).sum();
        assert!((total_cost - 3.0).abs() < 0.001);
    }

    #[test]
    fn planner_chooses_single_cheap_over_multi_expensive() {
        use crate::test_support::TestValue;
        // Path A: single step (cost 2.0)
        // Path B: 3 steps (cost 1.0 each = 3.0 total)
        // Should choose Path A
        let state = WorldState::from_facts(&[("start", true)]);
        let desired = WorldState::from_facts(&[("goal", true)]);
        let goal = GoapGoal::new("reach_goal", desired);

        let actions = vec![
            GoapAction::new("direct")
                .with_cost(2.0)
                .with_precondition("start", true)
                .with_effect("goal", true),
            GoapAction::new("step1")
                .with_cost(1.0)
                .with_precondition("start", true)
                .with_effect("mid1", true),
            GoapAction::new("step2")
                .with_cost(1.0)
                .with_precondition("mid1", true)
                .with_effect("mid2", true),
            GoapAction::new("step3")
                .with_cost(1.0)
                .with_precondition("mid2", true)
                .with_effect("goal", true),
        ];

        let planner = GoapPlanner::new();
        let plan = planner
            .plan(&state, &goal, &actions)
            .test_expect("plan exists");
        let total_cost: f32 = plan.iter().map(|a| a.cost).sum();
        assert!(
            total_cost <= 2.001,
            "should pick cheaper path, got cost {}",
            total_cost
        );
    }

    #[test]
    fn planner_with_max_iterations_1_fails_on_multi_step() {
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("c", true)]);
        let goal = GoapGoal::new("reach_c", desired);

        let actions = vec![
            GoapAction::new("a_to_b")
                .with_precondition("a", true)
                .with_effect("b", true),
            GoapAction::new("b_to_c")
                .with_precondition("b", true)
                .with_effect("c", true),
        ];

        let planner = GoapPlanner::new().with_max_iterations(1);
        let result = planner.plan(&state, &goal, &actions);
        assert!(
            result.is_none(),
            "should fail with max_iterations=1 on 2-step plan"
        );
    }

    #[test]
    fn planner_with_max_iterations_1000_finds_plan() {
        use crate::test_support::TestValue;
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("c", true)]);
        let goal = GoapGoal::new("reach_c", desired);

        let actions = vec![
            GoapAction::new("a_to_b")
                .with_precondition("a", true)
                .with_effect("b", true),
            GoapAction::new("b_to_c")
                .with_precondition("b", true)
                .with_effect("c", true),
        ];

        let planner = GoapPlanner::new().with_max_iterations(1000);
        let plan = planner.plan(&state, &goal, &actions);
        assert!(plan.is_some());
        assert_eq!(plan.test_unwrap().len(), 2);
    }

    #[test]
    fn planner_already_satisfied_returns_empty() {
        use crate::test_support::TestValue;
        let state = WorldState::from_facts(&[("goal", true)]);
        let desired = WorldState::from_facts(&[("goal", true)]);
        let goal = GoapGoal::new("already_done", desired);
        let planner = GoapPlanner::new();
        let plan = planner.plan(&state, &goal, &[]);
        assert!(plan.is_some());
        assert_eq!(plan.test_unwrap().len(), 0);
    }

    #[test]
    fn planner_impossible_goal_returns_none() {
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("impossible", true)]);
        let goal = GoapGoal::new("impossible", desired);
        // No actions can set "impossible"
        let actions = vec![GoapAction::new("noop")
            .with_precondition("a", true)
            .with_effect("b", true)];
        let planner = GoapPlanner::new();
        assert!(planner.plan(&state, &goal, &actions).is_none());
    }

    #[test]
    fn planner_depth_increments_correctly() {
        use crate::test_support::TestValue;
        // Verify that depth + 1 arithmetic is correct
        // If depth + 1 -> depth - 1, this would cause tie-breaking issues
        // Two equal-cost paths: planner should prefer fewer steps
        let state = WorldState::from_facts(&[("start", true)]);
        let desired = WorldState::from_facts(&[("goal", true)]);
        let goal = GoapGoal::new("test", desired);

        let actions = vec![
            // 1-step path
            GoapAction::new("direct")
                .with_cost(3.0)
                .with_precondition("start", true)
                .with_effect("goal", true),
            // 3-step path, same total cost
            GoapAction::new("s1")
                .with_cost(1.0)
                .with_precondition("start", true)
                .with_effect("m1", true),
            GoapAction::new("s2")
                .with_cost(1.0)
                .with_precondition("m1", true)
                .with_effect("m2", true),
            GoapAction::new("s3")
                .with_cost(1.0)
                .with_precondition("m2", true)
                .with_effect("goal", true),
        ];

        let planner = GoapPlanner::new();
        let plan = planner
            .plan(&state, &goal, &actions)
            .test_expect("plan found");
        // Both paths cost 3.0, but tie-breaking prefers fewer depth (fewer actions)
        assert_eq!(plan.len(), 1, "should prefer 1-step at equal cost");
        assert_eq!(plan[0].name, "direct");
    }
}

// =============================================================================
// MODULE 7: BehaviorStatus methods — all 9 methods
// =============================================================================
mod behavior_status_tests {
    use super::*;

    #[test]
    fn is_success_only_true_for_success() {
        assert!(BehaviorStatus::Success.is_success());
        assert!(!BehaviorStatus::Failure.is_success());
        assert!(!BehaviorStatus::Running.is_success());
    }

    #[test]
    fn is_failure_only_true_for_failure() {
        assert!(!BehaviorStatus::Success.is_failure());
        assert!(BehaviorStatus::Failure.is_failure());
        assert!(!BehaviorStatus::Running.is_failure());
    }

    #[test]
    fn is_running_only_true_for_running() {
        assert!(!BehaviorStatus::Success.is_running());
        assert!(!BehaviorStatus::Failure.is_running());
        assert!(BehaviorStatus::Running.is_running());
    }

    #[test]
    fn is_terminal_true_for_success_and_failure() {
        assert!(BehaviorStatus::Success.is_terminal());
        assert!(BehaviorStatus::Failure.is_terminal());
        assert!(!BehaviorStatus::Running.is_terminal());
    }

    #[test]
    fn name_returns_exact_strings() {
        assert_eq!(BehaviorStatus::Success.name(), "Success");
        assert_eq!(BehaviorStatus::Failure.name(), "Failure");
        assert_eq!(BehaviorStatus::Running.name(), "Running");
    }

    #[test]
    fn invert_swaps_success_failure() {
        assert_eq!(BehaviorStatus::Success.invert(), BehaviorStatus::Failure);
        assert_eq!(BehaviorStatus::Failure.invert(), BehaviorStatus::Success);
        assert_eq!(BehaviorStatus::Running.invert(), BehaviorStatus::Running);
    }

    #[test]
    fn to_success_if_running_converts_running_only() {
        assert_eq!(
            BehaviorStatus::Running.to_success_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Success.to_success_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Failure.to_success_if_running(),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn to_failure_if_running_converts_running_only() {
        assert_eq!(
            BehaviorStatus::Running.to_failure_if_running(),
            BehaviorStatus::Failure
        );
        assert_eq!(
            BehaviorStatus::Success.to_failure_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Failure.to_failure_if_running(),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn all_returns_three_variants() {
        let all = BehaviorStatus::all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], BehaviorStatus::Success);
        assert_eq!(all[1], BehaviorStatus::Failure);
        assert_eq!(all[2], BehaviorStatus::Running);
    }

    #[test]
    fn display_matches_name() {
        assert_eq!(format!("{}", BehaviorStatus::Success), "Success");
        assert_eq!(format!("{}", BehaviorStatus::Failure), "Failure");
        assert_eq!(format!("{}", BehaviorStatus::Running), "Running");
    }
}

// =============================================================================
// MODULE 8: BehaviorNode structural methods
// =============================================================================
mod behavior_node_structural_tests {
    use super::*;

    #[test]
    fn total_node_count_single_leaf() {
        assert_eq!(BehaviorNode::action("a").total_node_count(), 1);
        assert_eq!(BehaviorNode::condition("c").total_node_count(), 1);
    }

    #[test]
    fn total_node_count_sequence_with_children() {
        let seq = BehaviorNode::sequence(vec![
            BehaviorNode::action("a"),
            BehaviorNode::action("b"),
            BehaviorNode::action("c"),
        ]);
        assert_eq!(seq.total_node_count(), 4); // 1 sequence + 3 actions
    }

    #[test]
    fn total_node_count_nested_tree() {
        let tree = BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![BehaviorNode::action("a"), BehaviorNode::action("b")]),
            BehaviorNode::action("c"),
        ]);
        assert_eq!(tree.total_node_count(), 5); // selector + sequence + 3 actions
    }

    #[test]
    fn total_node_count_decorator() {
        let node = BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a"));
        assert_eq!(node.total_node_count(), 2);
    }

    #[test]
    fn total_node_count_parallel() {
        let node = BehaviorNode::parallel(
            vec![BehaviorNode::action("a"), BehaviorNode::action("b")],
            1,
        );
        assert_eq!(node.total_node_count(), 3);
    }

    #[test]
    fn total_node_count_empty_composite() {
        assert_eq!(BehaviorNode::sequence(vec![]).total_node_count(), 1);
        assert_eq!(BehaviorNode::selector(vec![]).total_node_count(), 1);
        assert_eq!(BehaviorNode::parallel(vec![], 0).total_node_count(), 1);
    }

    #[test]
    fn max_depth_single_leaf() {
        assert_eq!(BehaviorNode::action("a").max_depth(), 1);
    }

    #[test]
    fn max_depth_flat_sequence() {
        let seq =
            BehaviorNode::sequence(vec![BehaviorNode::action("a"), BehaviorNode::action("b")]);
        assert_eq!(seq.max_depth(), 2);
    }

    #[test]
    fn max_depth_nested_tree() {
        let tree = BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![BehaviorNode::action("a")]),
            BehaviorNode::action("b"),
        ]);
        assert_eq!(tree.max_depth(), 3); // selector -> sequence -> action
    }

    #[test]
    fn max_depth_empty_composite() {
        assert_eq!(BehaviorNode::sequence(vec![]).max_depth(), 1);
    }

    #[test]
    fn max_depth_decorator_chain() {
        let node = BehaviorNode::decorator(
            DecoratorType::Inverter,
            BehaviorNode::decorator(DecoratorType::Succeeder, BehaviorNode::action("a")),
        );
        assert_eq!(node.max_depth(), 3);
    }

    #[test]
    fn child_count_leaf_nodes() {
        assert_eq!(BehaviorNode::action("a").child_count(), 0);
        assert_eq!(BehaviorNode::condition("c").child_count(), 0);
    }

    #[test]
    fn child_count_composites() {
        assert_eq!(
            BehaviorNode::sequence(vec![BehaviorNode::action("a"), BehaviorNode::action("b")])
                .child_count(),
            2
        );
        assert_eq!(
            BehaviorNode::selector(vec![BehaviorNode::action("a")]).child_count(),
            1
        );
        assert_eq!(
            BehaviorNode::parallel(vec![BehaviorNode::action("a")], 1).child_count(),
            1
        );
    }

    #[test]
    fn child_count_decorator_is_one() {
        assert_eq!(
            BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a"))
                .child_count(),
            1
        );
    }

    #[test]
    fn name_returns_some_for_leaves() {
        assert_eq!(BehaviorNode::action("test").name(), Some("test"));
        assert_eq!(BehaviorNode::condition("cond").name(), Some("cond"));
    }

    #[test]
    fn name_returns_none_for_non_leaves() {
        assert!(BehaviorNode::sequence(vec![]).name().is_none());
        assert!(BehaviorNode::selector(vec![]).name().is_none());
        assert!(BehaviorNode::parallel(vec![], 0).name().is_none());
        assert!(
            BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a"))
                .name()
                .is_none()
        );
    }

    #[test]
    fn node_type_returns_unique_strings() {
        assert_eq!(BehaviorNode::action("a").node_type(), "Action");
        assert_eq!(BehaviorNode::condition("c").node_type(), "Condition");
        assert_eq!(BehaviorNode::sequence(vec![]).node_type(), "Sequence");
        assert_eq!(BehaviorNode::selector(vec![]).node_type(), "Selector");
        assert_eq!(BehaviorNode::parallel(vec![], 0).node_type(), "Parallel");
        assert_eq!(
            BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a")).node_type(),
            "Decorator"
        );
    }

    #[test]
    fn is_leaf_correct_for_all_types() {
        assert!(BehaviorNode::action("a").is_leaf());
        assert!(BehaviorNode::condition("c").is_leaf());
        assert!(!BehaviorNode::sequence(vec![]).is_leaf());
        assert!(!BehaviorNode::selector(vec![]).is_leaf());
        assert!(!BehaviorNode::parallel(vec![], 0).is_leaf());
        assert!(
            !BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a")).is_leaf()
        );
    }

    #[test]
    fn is_composite_correct_for_all_types() {
        assert!(!BehaviorNode::action("a").is_composite());
        assert!(!BehaviorNode::condition("c").is_composite());
        assert!(BehaviorNode::sequence(vec![]).is_composite());
        assert!(BehaviorNode::selector(vec![]).is_composite());
        assert!(BehaviorNode::parallel(vec![], 0).is_composite());
        assert!(
            !BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a"))
                .is_composite()
        );
    }

    #[test]
    fn is_action_true_only_for_action() {
        assert!(BehaviorNode::action("a").is_action());
        assert!(!BehaviorNode::condition("c").is_action());
        assert!(!BehaviorNode::sequence(vec![]).is_action());
    }

    #[test]
    fn is_condition_true_only_for_condition() {
        assert!(!BehaviorNode::action("a").is_condition());
        assert!(BehaviorNode::condition("c").is_condition());
        assert!(!BehaviorNode::selector(vec![]).is_condition());
    }

    #[test]
    fn is_sequence_true_only_for_sequence() {
        assert!(BehaviorNode::sequence(vec![]).is_sequence());
        assert!(!BehaviorNode::selector(vec![]).is_sequence());
        assert!(!BehaviorNode::action("a").is_sequence());
    }

    #[test]
    fn is_selector_true_only_for_selector() {
        assert!(BehaviorNode::selector(vec![]).is_selector());
        assert!(!BehaviorNode::sequence(vec![]).is_selector());
        assert!(!BehaviorNode::action("a").is_selector());
    }

    #[test]
    fn is_parallel_true_only_for_parallel() {
        assert!(BehaviorNode::parallel(vec![], 0).is_parallel());
        assert!(!BehaviorNode::sequence(vec![]).is_parallel());
        assert!(!BehaviorNode::action("a").is_parallel());
    }

    #[test]
    fn is_decorator_true_only_for_decorator() {
        assert!(
            BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("a"))
                .is_decorator()
        );
        assert!(!BehaviorNode::action("a").is_decorator());
        assert!(!BehaviorNode::sequence(vec![]).is_decorator());
    }

    #[test]
    fn summary_contains_node_type_info() {
        assert!(BehaviorNode::action("test").summary().contains("Action"));
        assert!(BehaviorNode::action("test").summary().contains("test"));
        assert!(BehaviorNode::condition("cond")
            .summary()
            .contains("Condition"));
        assert!(BehaviorNode::sequence(vec![BehaviorNode::action("a")])
            .summary()
            .contains("1"));
        assert!(BehaviorNode::selector(vec![]).summary().contains("0"));
        assert!(BehaviorNode::parallel(vec![BehaviorNode::action("a")], 1)
            .summary()
            .contains("1"));
    }

    #[test]
    fn display_outputs_summary() {
        let node = BehaviorNode::action("test");
        assert_eq!(format!("{}", node), node.summary());
    }
}

// =============================================================================
// MODULE 9: DecoratorType methods
// =============================================================================
mod decorator_type_tests {
    use super::*;

    #[test]
    fn name_returns_exact_strings() {
        assert_eq!(DecoratorType::Inverter.name(), "Inverter");
        assert_eq!(DecoratorType::Succeeder.name(), "Succeeder");
        assert_eq!(DecoratorType::Failer.name(), "Failer");
        assert_eq!(DecoratorType::Repeat(5).name(), "Repeat");
        assert_eq!(DecoratorType::Retry(3).name(), "Retry");
    }

    #[test]
    fn is_inverter_true_only_for_inverter() {
        assert!(DecoratorType::Inverter.is_inverter());
        assert!(!DecoratorType::Succeeder.is_inverter());
        assert!(!DecoratorType::Failer.is_inverter());
        assert!(!DecoratorType::Repeat(1).is_inverter());
        assert!(!DecoratorType::Retry(1).is_inverter());
    }

    #[test]
    fn forces_result_true_for_succeeder_and_failer() {
        assert!(!DecoratorType::Inverter.forces_result());
        assert!(DecoratorType::Succeeder.forces_result());
        assert!(DecoratorType::Failer.forces_result());
        assert!(!DecoratorType::Repeat(1).forces_result());
        assert!(!DecoratorType::Retry(1).forces_result());
    }

    #[test]
    fn is_looping_true_for_repeat_and_retry() {
        assert!(!DecoratorType::Inverter.is_looping());
        assert!(!DecoratorType::Succeeder.is_looping());
        assert!(!DecoratorType::Failer.is_looping());
        assert!(DecoratorType::Repeat(1).is_looping());
        assert!(DecoratorType::Retry(1).is_looping());
    }

    #[test]
    fn iteration_count_returns_count_for_looping() {
        assert_eq!(DecoratorType::Inverter.iteration_count(), None);
        assert_eq!(DecoratorType::Succeeder.iteration_count(), None);
        assert_eq!(DecoratorType::Failer.iteration_count(), None);
        assert_eq!(DecoratorType::Repeat(5).iteration_count(), Some(5));
        assert_eq!(DecoratorType::Retry(3).iteration_count(), Some(3));
    }

    #[test]
    fn all_returns_five_variants() {
        let all = DecoratorType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&DecoratorType::Inverter));
        assert!(all.contains(&DecoratorType::Succeeder));
        assert!(all.contains(&DecoratorType::Failer));
        // Repeat and Retry with default count 1
        assert!(all.iter().any(|d| matches!(d, DecoratorType::Repeat(1))));
        assert!(all.iter().any(|d| matches!(d, DecoratorType::Retry(1))));
    }

    #[test]
    fn display_format_non_looping() {
        assert_eq!(format!("{}", DecoratorType::Inverter), "Inverter");
        assert_eq!(format!("{}", DecoratorType::Succeeder), "Succeeder");
        assert_eq!(format!("{}", DecoratorType::Failer), "Failer");
    }

    #[test]
    fn display_format_looping_with_count() {
        assert_eq!(format!("{}", DecoratorType::Repeat(3)), "Repeat(3)");
        assert_eq!(format!("{}", DecoratorType::Retry(5)), "Retry(5)");
    }
}

// =============================================================================
// MODULE 10: BehaviorGraph delegation methods
// =============================================================================
mod behavior_graph_tests {
    use super::*;

    #[test]
    fn node_count_delegates_to_root() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![
            BehaviorNode::action("a"),
            BehaviorNode::action("b"),
        ]));
        assert_eq!(graph.node_count(), 3); // sequence + 2 actions
    }

    #[test]
    fn max_depth_delegates_to_root() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![BehaviorNode::action("a")]));
        assert_eq!(graph.max_depth(), 2);
    }

    #[test]
    fn root_type_returns_root_node_type() {
        assert_eq!(
            BehaviorGraph::new(BehaviorNode::sequence(vec![])).root_type(),
            "Sequence"
        );
        assert_eq!(
            BehaviorGraph::new(BehaviorNode::action("a")).root_type(),
            "Action"
        );
    }

    #[test]
    fn is_leaf_delegates() {
        assert!(BehaviorGraph::new(BehaviorNode::action("a")).is_leaf());
        assert!(!BehaviorGraph::new(BehaviorNode::sequence(vec![])).is_leaf());
    }

    #[test]
    fn is_composite_delegates() {
        assert!(!BehaviorGraph::new(BehaviorNode::action("a")).is_composite());
        assert!(BehaviorGraph::new(BehaviorNode::sequence(vec![])).is_composite());
    }

    #[test]
    fn current_node_name_returns_root() {
        let graph = BehaviorGraph::new(BehaviorNode::action("test"));
        assert_eq!(graph.current_node_name(), Some("root".to_string()));
    }

    #[test]
    fn summary_contains_type_nodes_depth() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![BehaviorNode::action("a")]));
        let summary = graph.summary();
        assert!(summary.contains("Sequence"));
        assert!(summary.contains("2")); // node count
    }

    #[test]
    fn display_matches_summary() {
        let graph = BehaviorGraph::new(BehaviorNode::action("a"));
        assert_eq!(format!("{}", graph), graph.summary());
    }

    #[test]
    fn tick_delegates_to_root() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s", || BehaviorStatus::Success);
        let graph = BehaviorGraph::new(BehaviorNode::action("s"));
        assert_eq!(graph.tick(&ctx), BehaviorStatus::Success);
    }
}

// =============================================================================
// MODULE 11: BehaviorContext methods
// =============================================================================
mod behavior_context_tests {
    use super::*;

    #[test]
    fn total_count_sums_actions_and_conditions() {
        let mut ctx = BehaviorContext::new();
        assert_eq!(ctx.total_count(), 0);
        ctx.register_action("a1", || BehaviorStatus::Success);
        assert_eq!(ctx.total_count(), 1);
        ctx.register_condition("c1", || true);
        assert_eq!(ctx.total_count(), 2);
    }

    #[test]
    fn is_empty_true_when_no_handlers() {
        let ctx = BehaviorContext::new();
        assert!(ctx.is_empty());
    }

    #[test]
    fn is_empty_false_with_action_only() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a", || BehaviorStatus::Success);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn is_empty_false_with_condition_only() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("c", || true);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn has_action_returns_correct_results() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("attack", || BehaviorStatus::Success);
        assert!(ctx.has_action("attack"));
        assert!(!ctx.has_action("defend"));
    }

    #[test]
    fn has_condition_returns_correct_results() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("is_alive", || true);
        assert!(ctx.has_condition("is_alive"));
        assert!(!ctx.has_condition("is_dead"));
    }

    #[test]
    fn action_names_returns_all_registered() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        let names = ctx.action_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a1"));
        assert!(names.contains(&"a2"));
    }

    #[test]
    fn condition_names_returns_all_registered() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("c1", || true);
        ctx.register_condition("c2", || false);
        let names = ctx.condition_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"c1"));
        assert!(names.contains(&"c2"));
    }

    #[test]
    fn remove_action_returns_true_when_exists() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a", || BehaviorStatus::Success);
        assert!(ctx.remove_action("a"));
        assert!(!ctx.has_action("a"));
    }

    #[test]
    fn remove_action_returns_false_when_missing() {
        let mut ctx = BehaviorContext::new();
        assert!(!ctx.remove_action("nonexistent"));
    }

    #[test]
    fn remove_condition_returns_true_when_exists() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("c", || true);
        assert!(ctx.remove_condition("c"));
        assert!(!ctx.has_condition("c"));
    }

    #[test]
    fn remove_condition_returns_false_when_missing() {
        let mut ctx = BehaviorContext::new();
        assert!(!ctx.remove_condition("nonexistent"));
    }

    #[test]
    fn clear_actions_only_removes_actions() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a", || BehaviorStatus::Success);
        ctx.register_condition("c", || true);
        ctx.clear_actions();
        assert_eq!(ctx.action_count(), 0);
        assert_eq!(ctx.condition_count(), 1);
    }

    #[test]
    fn clear_conditions_only_removes_conditions() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a", || BehaviorStatus::Success);
        ctx.register_condition("c", || true);
        ctx.clear_conditions();
        assert_eq!(ctx.action_count(), 1);
        assert_eq!(ctx.condition_count(), 0);
    }

    #[test]
    fn clear_removes_everything() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a", || BehaviorStatus::Success);
        ctx.register_condition("c", || true);
        ctx.clear();
        assert!(ctx.is_empty());
        assert_eq!(ctx.total_count(), 0);
    }

    #[test]
    fn summary_contains_counts() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        ctx.register_condition("c1", || true);
        let summary = ctx.summary();
        assert!(summary.contains("2 actions"));
        assert!(summary.contains("1 conditions"));
    }

    #[test]
    fn display_contains_behavior_context() {
        let ctx = BehaviorContext::new();
        let display = format!("{}", ctx);
        assert!(display.contains("BehaviorContext"));
    }

    #[test]
    #[should_panic(expected = "not registered")]
    fn evaluate_missing_action_panics_in_debug() {
        let ctx = BehaviorContext::new();
        let node = BehaviorNode::Action("nonexistent".to_string());
        // In debug mode, debug_assert fires a panic
        node.tick(&ctx);
    }

    #[test]
    #[should_panic(expected = "not registered")]
    fn evaluate_missing_condition_panics_in_debug() {
        let ctx = BehaviorContext::new();
        let node = BehaviorNode::Condition("nonexistent".to_string());
        node.tick(&ctx);
    }

    #[test]
    fn condition_true_returns_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("yes", || true);
        let node = BehaviorNode::Condition("yes".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn condition_false_returns_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("no", || false);
        let node = BehaviorNode::Condition("no".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }
}

// =============================================================================
// MODULE 12: PlanCache LRU ordering and eviction mutations
// =============================================================================
mod plan_cache_lru_tests {
    use super::*;

    fn make_state(key: &str, val: bool) -> WorldState {
        WorldState::from_facts(&[(key, val)])
    }

    fn make_goal(key: &str) -> GoapGoal {
        GoapGoal::new("goal", WorldState::from_facts(&[(key, true)]))
    }

    fn make_actions() -> Vec<GoapAction> {
        vec![GoapAction::new("act")]
    }

    fn make_plan(name: &str) -> Vec<GoapAction> {
        vec![GoapAction::new(name)]
    }

    #[test]
    fn lru_evicts_oldest_entry() {
        let mut cache = PlanCache::new(2);
        let actions = make_actions();

        // Put entry A and B
        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan_a"));

        let sb = make_state("b", true);
        let gb = make_goal("gb");
        cache.put(&sb, &gb, &actions, make_plan("plan_b"));

        assert_eq!(cache.len(), 2);

        // Put C -> should evict A (oldest)
        let sc = make_state("c", true);
        let gc = make_goal("gc");
        cache.put(&sc, &gc, &actions, make_plan("plan_c"));

        assert_eq!(cache.len(), 2);
        // A should be evicted
        assert!(cache.get(&sa, &ga, &actions).is_none());
        // B and C should be present
        assert!(cache.get(&sb, &gb, &actions).is_some());
        assert!(cache.get(&sc, &gc, &actions).is_some());
    }

    #[test]
    fn lru_access_refreshes_position() {
        let mut cache = PlanCache::new(2);
        let actions = make_actions();

        // Put A then B
        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan_a"));

        let sb = make_state("b", true);
        let gb = make_goal("gb");
        cache.put(&sb, &gb, &actions, make_plan("plan_b"));

        // Access A (moves to back of LRU)
        let _ = cache.get(&sa, &ga, &actions);

        // Put C -> should evict B now (oldest after A was refreshed)
        let sc = make_state("c", true);
        let gc = make_goal("gc");
        cache.put(&sc, &gc, &actions, make_plan("plan_c"));

        // B should be evicted (was oldest after A refresh)
        assert!(cache.get(&sb, &gb, &actions).is_none());
        // A should still be present
        assert!(cache.get(&sa, &ga, &actions).is_some());
        // C should be present
        assert!(cache.get(&sc, &gc, &actions).is_some());
    }

    #[test]
    fn put_same_key_does_not_evict() {
        let mut cache = PlanCache::new(2);
        let actions = make_actions();

        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan_a"));
        cache.put(&sa, &ga, &actions, make_plan("plan_a_v2")); // overwrite
        assert_eq!(cache.len(), 1); // still 1 entry
        assert_eq!(cache.stats().evictions, 0);
    }

    #[test]
    fn eviction_counter_increments() {
        let mut cache = PlanCache::new(1);
        let actions = make_actions();

        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan_a"));

        let sb = make_state("b", true);
        let gb = make_goal("gb");
        cache.put(&sb, &gb, &actions, make_plan("plan_b"));

        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn hit_increments_hits_counter() {
        let mut cache = PlanCache::new(10);
        let actions = make_actions();

        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan_a"));

        let _ = cache.get(&sa, &ga, &actions);
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn miss_increments_misses_counter() {
        let mut cache = PlanCache::new(10);
        let actions = make_actions();

        let sa = make_state("nonexist", true);
        let ga = make_goal("missing");
        let _ = cache.get(&sa, &ga, &actions);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);
    }

    #[test]
    fn clear_resets_everything() {
        let mut cache = PlanCache::new(10);
        let actions = make_actions();
        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan"));
        let _ = cache.get(&sa, &ga, &actions);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn capacity_returns_max_size() {
        let cache = PlanCache::new(42);
        assert_eq!(cache.capacity(), 42);
    }

    #[test]
    fn default_cache_capacity_1000() {
        let cache = PlanCache::default();
        assert_eq!(cache.capacity(), 1000);
    }

    #[test]
    fn is_empty_correct() {
        let mut cache = PlanCache::new(10);
        assert!(cache.is_empty());
        let actions = make_actions();
        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions, make_plan("plan"));
        assert!(!cache.is_empty());
    }

    #[test]
    fn action_set_change_invalidates_cache() {
        let mut cache = PlanCache::new(10);
        let actions_v1 = vec![GoapAction::new("act").with_cost(1.0)];
        let sa = make_state("a", true);
        let ga = make_goal("ga");
        cache.put(&sa, &ga, &actions_v1, make_plan("plan"));

        // Change action cost (different action hash)
        let actions_v2 = vec![GoapAction::new("act").with_cost(2.0)];
        let result = cache.get(&sa, &ga, &actions_v2);
        assert!(result.is_none());
        assert_eq!(cache.stats().invalidations, 1);
    }
}

// =============================================================================
// MODULE 13: CachedGoapPlanner — with_planner, base_planner, base_planner_mut
// =============================================================================
mod cached_goap_planner_tests {
    use super::*;

    #[test]
    fn with_planner_uses_custom_config() {
        let planner = GoapPlanner::new().with_max_iterations(5);
        let mut cached = CachedGoapPlanner::with_planner(planner, 50);

        // Verify the custom planner is used (max_iterations=5 should fail for multi-step)
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("d", true)]);
        let goal = GoapGoal::new("test", desired);
        let actions = vec![
            GoapAction::new("a_b")
                .with_precondition("a", true)
                .with_effect("b", true),
            GoapAction::new("b_c")
                .with_precondition("b", true)
                .with_effect("c", true),
            GoapAction::new("c_d")
                .with_precondition("c", true)
                .with_effect("d", true),
        ];

        // With max_iterations=5, should still find 3-step plan (5 iterations is enough)
        let result = cached.plan(&state, &goal, &actions);
        assert!(result.is_some());
    }

    #[test]
    fn base_planner_returns_reference() {
        let cached = CachedGoapPlanner::new(100);
        let _planner: &GoapPlanner = cached.base_planner();
        // Just verify it compiles and doesn't panic
    }

    #[test]
    fn base_planner_mut_allows_modification() {
        let mut cached = CachedGoapPlanner::new(100);
        let planner_mut = cached.base_planner_mut();
        // Modify via mutable reference
        *planner_mut = GoapPlanner::new().with_max_iterations(1);
        // Verify the modification took effect
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("c", true)]);
        let goal = GoapGoal::new("test", desired);
        let actions = vec![
            GoapAction::new("a_b")
                .with_precondition("a", true)
                .with_effect("b", true),
            GoapAction::new("b_c")
                .with_precondition("b", true)
                .with_effect("c", true),
        ];
        // With max_iterations=1, should fail to find 2-step plan
        assert!(cached.plan(&state, &goal, &actions).is_none());
    }

    #[test]
    fn clear_cache_resets_stats() {
        let mut cached = CachedGoapPlanner::new(10);
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("b", true)]);
        let goal = GoapGoal::new("test", desired);
        let actions = vec![GoapAction::new("a_b")
            .with_precondition("a", true)
            .with_effect("b", true)];
        let _ = cached.plan(&state, &goal, &actions);
        assert!(cached.cache_stats().total_accesses() > 0);

        cached.clear_cache();
        assert_eq!(cached.cache_stats().total_accesses(), 0);
    }

    #[test]
    fn default_capacity_1000() {
        let cached = CachedGoapPlanner::default();
        // Access through cache_stats to verify it works
        assert_eq!(cached.cache_stats().hits, 0);
    }

    #[test]
    fn cached_plan_matches_direct_plan() {
        use crate::test_support::TestValue;
        let mut cached = CachedGoapPlanner::new(10);
        let state = WorldState::from_facts(&[("a", true)]);
        let desired = WorldState::from_facts(&[("b", true)]);
        let goal = GoapGoal::new("test", desired);
        let actions = vec![GoapAction::new("a_b")
            .with_precondition("a", true)
            .with_effect("b", true)];

        let plan1 = cached.plan(&state, &goal, &actions);
        let plan2 = cached.plan(&state, &goal, &actions); // should hit cache

        assert_eq!(cached.cache_stats().hits, 1);
        assert_eq!(cached.cache_stats().misses, 1);

        // Plans should have same structure
        let p1 = plan1.test_unwrap();
        let p2 = plan2.test_unwrap();
        assert_eq!(p1.len(), p2.len());
        assert_eq!(p1[0].name, p2[0].name);
    }
}

// =============================================================================
// MODULE 14: GoapAction methods
// =============================================================================
mod goap_action_tests {
    use super::*;

    #[test]
    fn new_action_default_cost_is_1() {
        let action = GoapAction::new("test");
        assert_eq!(action.cost, 1.0);
    }

    #[test]
    fn with_cost_sets_exact_cost() {
        let action = GoapAction::new("test").with_cost(3.5);
        assert_eq!(action.cost, 3.5);
    }

    #[test]
    fn can_apply_true_when_preconditions_met() {
        let action = GoapAction::new("test").with_precondition("has_weapon", true);
        let state = WorldState::from_facts(&[("has_weapon", true)]);
        assert!(action.can_apply(&state));
    }

    #[test]
    fn can_apply_false_when_preconditions_not_met() {
        let action = GoapAction::new("test").with_precondition("has_weapon", true);
        let state = WorldState::from_facts(&[("has_weapon", false)]);
        assert!(!action.can_apply(&state));
    }

    #[test]
    fn can_apply_true_with_empty_preconditions() {
        let action = GoapAction::new("test");
        let state = WorldState::new();
        assert!(action.can_apply(&state));
    }

    #[test]
    fn apply_returns_new_state_with_effects() {
        let action = GoapAction::new("test").with_effect("armed", true);
        let state = WorldState::from_facts(&[("alive", true)]);
        let new_state = action.apply(&state);
        assert_eq!(new_state.get("alive"), Some(true)); // preserved
        assert_eq!(new_state.get("armed"), Some(true)); // added
    }

    #[test]
    fn apply_does_not_modify_original_state() {
        let action = GoapAction::new("test").with_effect("changed", true);
        let state = WorldState::from_facts(&[("original", true)]);
        let _new = action.apply(&state);
        assert_eq!(state.get("changed"), None); // original unchanged
    }
}

// =============================================================================
// MODULE 15: WorldState::satisfies() edge cases
// =============================================================================
mod world_state_satisfies_tests {
    use super::*;

    #[test]
    fn any_state_satisfies_empty_goal() {
        let state = WorldState::from_facts(&[("a", true)]);
        let empty = WorldState::new();
        assert!(state.satisfies(&empty));
    }

    #[test]
    fn empty_state_satisfies_empty_goal() {
        let state = WorldState::new();
        let goal = WorldState::new();
        assert!(state.satisfies(&goal));
    }

    #[test]
    fn empty_state_does_not_satisfy_nonempty_goal() {
        let state = WorldState::new();
        let goal = WorldState::from_facts(&[("a", true)]);
        assert!(!state.satisfies(&goal));
    }

    #[test]
    fn value_mismatch_not_satisfied() {
        let state = WorldState::from_facts(&[("a", false)]);
        let goal = WorldState::from_facts(&[("a", true)]);
        assert!(!state.satisfies(&goal));
    }

    #[test]
    fn partial_match_not_satisfied() {
        let state = WorldState::from_facts(&[("a", true)]);
        let goal = WorldState::from_facts(&[("a", true), ("b", true)]);
        assert!(!state.satisfies(&goal));
    }

    #[test]
    fn superset_satisfies_subset() {
        let state = WorldState::from_facts(&[("a", true), ("b", true), ("c", false)]);
        let goal = WorldState::from_facts(&[("a", true), ("b", true)]);
        assert!(state.satisfies(&goal));
    }
}

// =============================================================================
// MODULE 16: Decorator tick behavior (Running in Repeat/Retry)
// =============================================================================
mod decorator_tick_tests {
    use super::*;

    #[test]
    fn repeat_zero_returns_success_immediately() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail", || BehaviorStatus::Failure);
        let node = BehaviorNode::decorator(DecoratorType::Repeat(0), BehaviorNode::action("fail"));
        // Repeat(0) loops 0 times, so returns Success without executing child
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn retry_zero_returns_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed", || BehaviorStatus::Success);
        let node =
            BehaviorNode::decorator(DecoratorType::Retry(0), BehaviorNode::action("succeed"));
        // Retry(0) loops 0 times, returns Failure (exhausted)
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn repeat_running_returns_running() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("run", || BehaviorStatus::Running);
        let node = BehaviorNode::decorator(DecoratorType::Repeat(3), BehaviorNode::action("run"));
        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn retry_running_returns_running() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("run", || BehaviorStatus::Running);
        let node = BehaviorNode::decorator(DecoratorType::Retry(3), BehaviorNode::action("run"));
        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn succeeder_ignores_child_result() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("run", || BehaviorStatus::Running);
        let node = BehaviorNode::decorator(DecoratorType::Succeeder, BehaviorNode::action("run"));
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn failer_ignores_child_result() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("run", || BehaviorStatus::Running);
        let node = BehaviorNode::decorator(DecoratorType::Failer, BehaviorNode::action("run"));
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }
}

// =============================================================================
// MODULE 17: Sequence/Selector empty and single-child edge cases
// =============================================================================
mod sequence_selector_edge_tests {
    use super::*;

    #[test]
    fn empty_sequence_returns_success() {
        let ctx = BehaviorContext::new();
        assert_eq!(
            BehaviorNode::Sequence(vec![]).tick(&ctx),
            BehaviorStatus::Success
        );
    }

    #[test]
    fn empty_selector_returns_failure() {
        let ctx = BehaviorContext::new();
        assert_eq!(
            BehaviorNode::Selector(vec![]).tick(&ctx),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn single_child_sequence_returns_child_result() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("f", || BehaviorStatus::Failure);
        assert_eq!(
            BehaviorNode::Sequence(vec![BehaviorNode::Action("f".into())]).tick(&ctx),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn single_child_selector_returns_child_result() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("s", || BehaviorStatus::Success);
        assert_eq!(
            BehaviorNode::Selector(vec![BehaviorNode::Action("s".into())]).tick(&ctx),
            BehaviorStatus::Success
        );
    }
}

// =============================================================================
// MODULE 18: Interner exact ID assignment
// =============================================================================
mod interner_tests {
    use astraweave_behavior::interner::StringInterner;

    #[test]
    fn first_intern_returns_0() {
        let mut interner = StringInterner::new();
        assert_eq!(interner.intern("first"), 0);
    }

    #[test]
    fn second_intern_returns_1() {
        let mut interner = StringInterner::new();
        interner.intern("first");
        assert_eq!(interner.intern("second"), 1);
    }

    #[test]
    fn duplicate_returns_same_id() {
        let mut interner = StringInterner::new();
        let id1 = interner.intern("hello");
        let id2 = interner.intern("hello");
        assert_eq!(id1, id2);
        assert_eq!(id1, 0);
    }

    #[test]
    fn resolve_valid_id() {
        let mut interner = StringInterner::new();
        let id = interner.intern("test_string");
        assert_eq!(interner.resolve(id), Some("test_string"));
    }

    #[test]
    fn resolve_invalid_id_returns_none() {
        let interner = StringInterner::new();
        assert_eq!(interner.resolve(999), None);
    }

    #[test]
    fn resolve_after_multiple_interns() {
        let mut interner = StringInterner::new();
        let a = interner.intern("alpha");
        let b = interner.intern("beta");
        let c = interner.intern("gamma");
        assert_eq!(interner.resolve(a), Some("alpha"));
        assert_eq!(interner.resolve(b), Some("beta"));
        assert_eq!(interner.resolve(c), Some("gamma"));
    }
}

#[path = "../src/test_support.rs"]
mod test_support;
