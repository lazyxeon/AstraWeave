//! Behavior-graph and interner tests.
//!
//! The symbolic planner tests left with the planner.

use astraweave_behavior::{
    BehaviorContext, BehaviorGraph, BehaviorNode, BehaviorStatus, DecoratorType,
};

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
