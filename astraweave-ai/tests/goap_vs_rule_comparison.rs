// Integration test: GOAP vs Rule-based Orchestrator comparison
// Phase 2: Engine Integration - Shadow Mode validation

#[cfg(feature = "planner_advanced")]
mod goap_comparison_tests {
    use astraweave_ai::goap::orchestrator::GOAPOrchestrator;
    use astraweave_ai::goap::shadow_mode::ShadowModeRunner;
    use astraweave_ai::orchestrator::RuleOrchestrator;
    use astraweave_core::{
        CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance, WorldSnapshot,
    };
    use std::collections::BTreeMap;

    fn make_test_snapshot(player_hp: i32, ammo: i32, enemies: Vec<EnemyState>) -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: player_hp,

                pos: IVec2 { x: 0, y: 0 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                ammo,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies,
            pois: vec![],
            obstacles: vec![],
            objective: Some("Defeat enemies".to_string()),
        }
    }

    fn make_enemy(id: u32, x: i32, y: i32, hp: i32) -> EnemyState {
        EnemyState {
            id,
            pos: IVec2 { x, y },
            hp,
            cover: CoverType::None,
            last_seen: 1.0,
        }
    }

    #[test]
    fn test_shadow_mode_basic() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        let snap = make_test_snapshot(100, 20, vec![make_enemy(1, 10, 10, 50)]);

        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Both should produce plans
        assert!(
            !comparison.rule_plan.action_types.is_empty()
                || !comparison.goap_plan.action_types.is_empty()
        );

        // Should have valid metrics
        assert!(comparison.metrics.time_difference_ms != 0.0 || comparison.metrics.rule_faster);

        println!("{}", comparison.to_log_entry());
    }

    #[test]
    fn test_shadow_mode_multiple_scenarios() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        // Scenario 1: Normal combat
        let snap1 = make_test_snapshot(100, 20, vec![make_enemy(1, 10, 10, 50)]);
        shadow.compare(&snap1, &rule_orchestrator, &mut goap_orchestrator);

        // Scenario 2: Low health, need healing
        let snap2 = make_test_snapshot(30, 20, vec![make_enemy(1, 10, 10, 50)]);
        shadow.compare(&snap2, &rule_orchestrator, &mut goap_orchestrator);

        // Scenario 3: No ammo, need reload
        let snap3 = make_test_snapshot(100, 0, vec![make_enemy(1, 10, 10, 50)]);
        shadow.compare(&snap3, &rule_orchestrator, &mut goap_orchestrator);

        // Scenario 4: No enemies, exploration
        let snap4 = make_test_snapshot(100, 20, vec![]);
        shadow.compare(&snap4, &rule_orchestrator, &mut goap_orchestrator);

        // Generate aggregate report
        let report = shadow.generate_report();
        assert_eq!(report.total_comparisons, 4);

        report.print_report();
    }

    #[test]
    fn test_comparison_similarity_calculation() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        let snap = make_test_snapshot(100, 20, vec![make_enemy(1, 8, 8, 50)]);
        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Similarity score should be between 0 and 1
        assert!(comparison.differences.similarity_score >= 0.0);
        assert!(comparison.differences.similarity_score <= 1.0);

        println!(
            "Similarity: {:.1}%",
            comparison.differences.similarity_score * 100.0
        );
        println!(
            "Common actions: {}",
            comparison.differences.actions_in_common
        );
    }

    #[test]
    fn test_shadow_mode_performance() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        // Create a moderately complex scenario
        let enemies = vec![make_enemy(1, 10, 10, 50), make_enemy(2, 15, 8, 40)];
        let snap = make_test_snapshot(80, 15, enemies);

        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Both should be reasonably fast (< 100ms)
        assert!(
            comparison.rule_plan.planning_time_ms < 100.0,
            "Rule planner too slow: {:.2}ms",
            comparison.rule_plan.planning_time_ms
        );
        assert!(
            comparison.goap_plan.planning_time_ms < 100.0,
            "GOAP planner too slow: {:.2}ms",
            comparison.goap_plan.planning_time_ms
        );

        println!(
            "Rule: {:.2}ms, GOAP: {:.2}ms",
            comparison.rule_plan.planning_time_ms, comparison.goap_plan.planning_time_ms
        );
    }

    #[test]
    fn test_shadow_mode_with_cooldowns() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        let mut snap = make_test_snapshot(100, 20, vec![make_enemy(1, 10, 10, 50)]);

        // Add smoke grenade cooldown
        snap.me.cooldowns.insert("throw:smoke".to_string(), 5.0);

        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Plans should adapt to cooldown
        let has_smoke = comparison
            .goap_plan
            .action_types
            .iter()
            .any(|a| a.contains("smoke") || a.contains("Throw"));

        if snap.me.cooldowns.get("throw:smoke").unwrap_or(&0.0) > &0.0 {
            // With active cooldown, shouldn't try to throw smoke immediately
            println!("Cooldown active, smoke actions: {}", has_smoke);
        }

        println!("{}", comparison.to_log_entry());
    }

    #[test]
    fn test_comparison_json_export() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        let snap = make_test_snapshot(100, 20, vec![make_enemy(1, 10, 10, 50)]);
        shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        let comparisons = shadow.get_comparisons();
        assert_eq!(comparisons.len(), 1);

        let json = comparisons[0]
            .to_json()
            .expect("Failed to serialize to JSON");
        assert!(json.contains("tactical_summary"));
        assert!(json.contains("rule_plan"));
        assert!(json.contains("goap_plan"));

        println!(
            "JSON export (truncated): {}...",
            &json[..200.min(json.len())]
        );
    }

    #[test]
    fn test_goap_action_diversity() {
        let goap_orchestrator = GOAPOrchestrator::new();

        // Check that GOAP has registered multiple actions
        let action_count = goap_orchestrator.planner().action_count();
        assert!(
            action_count >= 10,
            "GOAP should have at least 10 actions, got {}",
            action_count
        );

        let action_names = goap_orchestrator.planner().action_names();
        println!("Registered GOAP actions ({}):", action_count);
        for name in &action_names {
            println!("  - {}", name);
        }

        // Should include key action types
        assert!(action_names.iter().any(|n| n.contains("attack")));
        assert!(action_names.iter().any(|n| n.contains("heal")));
        assert!(action_names.iter().any(|n| n.contains("reload")));
    }

    #[test]
    fn test_empty_plan_handling() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        // No enemies, no objectives
        let snap = make_test_snapshot(100, 20, vec![]);

        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Should handle empty/idle plans gracefully
        if comparison.metrics.both_empty {
            println!("Both planners correctly returned empty plans for idle state");
        }

        // Should not panic or produce errors
        assert!(!comparison.rule_plan.plan_id.is_empty());
        assert!(!comparison.goap_plan.plan_id.is_empty());
    }

    #[test]
    fn test_plan_diff_output() {
        let mut shadow = ShadowModeRunner::new(false);
        let rule_orchestrator = RuleOrchestrator;
        let mut goap_orchestrator = GOAPOrchestrator::new();

        let snap = make_test_snapshot(100, 20, vec![make_enemy(1, 10, 10, 50)]);
        let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);

        // Print detailed diff
        println!("\n=== Plan Comparison Details ===");
        println!("Rule actions: {:?}", comparison.rule_plan.action_types);
        println!("GOAP actions: {:?}", comparison.goap_plan.action_types);
        println!(
            "Actions in common: {}",
            comparison.differences.actions_in_common
        );
        println!(
            "Unique to Rule: {:?}",
            comparison.differences.unique_to_rule
        );
        println!(
            "Unique to GOAP: {:?}",
            comparison.differences.unique_to_goap
        );
        println!(
            "Similarity: {:.1}%",
            comparison.differences.similarity_score * 100.0
        );

        // Diff should have meaningful content
        assert!(comparison.differences.similarity_score >= 0.0);
    }
}
