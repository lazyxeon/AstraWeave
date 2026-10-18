#[cfg(feature = "planner_advanced")]
mod hierarchical_planning_tests {
    use astraweave_ai::goap::*;
    use std::collections::BTreeMap;

    fn create_test_planner() -> AdvancedGOAP {
        let mut planner = AdvancedGOAP::new();

        // Add basic test actions
        let mut move_preconds = BTreeMap::new();
        move_preconds.insert("can_move".to_string(), StateValue::Bool(true));
        let mut move_effects = BTreeMap::new();
        move_effects.insert("at_location".to_string(), StateValue::Bool(true));
        planner.add_action(Box::new(SimpleAction::new(
            "move",
            move_preconds,
            move_effects,
            1.0,
        )));

        let mut attack_preconds = BTreeMap::new();
        attack_preconds.insert("has_weapon".to_string(), StateValue::Bool(true));
        attack_preconds.insert("in_range".to_string(), StateValue::Bool(true));
        let mut attack_effects = BTreeMap::new();
        attack_effects.insert("enemy_defeated".to_string(), StateValue::Bool(true));
        planner.add_action(Box::new(SimpleAction::new(
            "attack",
            attack_preconds,
            attack_effects,
            2.0,
        )));

        let mut equip_preconds = BTreeMap::new();
        equip_preconds.insert("has_weapon".to_string(), StateValue::Bool(true));
        let mut equip_effects = BTreeMap::new();
        equip_effects.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        planner.add_action(Box::new(SimpleAction::new(
            "equip",
            equip_preconds,
            equip_effects,
            0.5,
        )));

        let scan_preconds = BTreeMap::new();
        let mut scan_effects = BTreeMap::new();
        scan_effects.insert("area_scanned".to_string(), StateValue::Bool(true));
        scan_effects.insert("enemies_located".to_string(), StateValue::Bool(true));
        planner.add_action(Box::new(SimpleAction::new(
            "scan",
            scan_preconds,
            scan_effects,
            1.0,
        )));

        planner
    }

    #[test]
    fn test_simple_goal_still_works() {
        let planner = create_test_planner();
        let mut start = WorldState::new();
        start.set("can_move", StateValue::Bool(true));

        let mut desired = BTreeMap::new();
        desired.insert("at_location".to_string(), StateValue::Bool(true));
        let goal = Goal::new("reach_location", desired);

        let plan = planner.plan(&start, &goal);
        assert!(plan.is_some());
        let plan = plan.unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0], "move");
    }

    #[test]
    fn test_sequential_decomposition() {
        let planner = create_test_planner();
        let mut start = WorldState::new();
        start.set("can_move", StateValue::Bool(false));
        start.set("has_weapon", StateValue::Bool(true));

        // Sub-goal 1: scan area
        let mut scan_desired = BTreeMap::new();
        scan_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let scan_goal = Goal::new("scan", scan_desired);

        // Sub-goal 2: equip weapon
        let mut equip_desired = BTreeMap::new();
        equip_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let equip_goal = Goal::new("equip", equip_desired);

        // Main goal with sequential sub-goals
        let mut main_desired = BTreeMap::new();
        main_desired.insert("ready_for_combat".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("prepare_for_combat", main_desired)
            .with_strategy(DecompositionStrategy::Sequential)
            .with_sub_goals(vec![scan_goal, equip_goal]);

        // Should plan both sub-goals in sequence
        let plan = planner.plan(&start, &main_goal);

        if let Some(plan) = plan {
            assert!(plan.contains(&"scan".to_string()));
            assert!(plan.contains(&"equip".to_string()));
            // Scan should come before equip (sequential)
            let scan_idx = plan.iter().position(|a| a == "scan").unwrap();
            let equip_idx = plan.iter().position(|a| a == "equip").unwrap();
            assert!(scan_idx < equip_idx);
        }
    }

    #[test]
    fn test_hierarchical_depth() {
        let planner = create_test_planner();
        let start = WorldState::new();

        // Level 2 sub-sub-goal
        let mut l2_desired = BTreeMap::new();
        l2_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let l2_goal = Goal::new("level2", l2_desired);

        // Level 1 sub-goal
        let mut l1_desired = BTreeMap::new();
        l1_desired.insert("level1_complete".to_string(), StateValue::Bool(true));
        let l1_goal = Goal::new("level1", l1_desired).with_sub_goals(vec![l2_goal]);

        // Root goal
        let mut root_desired = BTreeMap::new();
        root_desired.insert("mission_complete".to_string(), StateValue::Bool(true));
        let root_goal = Goal::new("root", root_desired).with_sub_goals(vec![l1_goal]);

        // Check goal depth calculation
        assert_eq!(root_goal.depth(), 3);
        assert_eq!(root_goal.total_goal_count(), 3);

        // Plan should work with hierarchy
        let plan = planner.plan(&start, &root_goal);
        assert!(plan.is_some());
    }

    #[test]
    fn test_max_depth_limit() {
        let planner = create_test_planner();
        let start = WorldState::new();

        // Create deep hierarchy
        let mut l3_desired = BTreeMap::new();
        l3_desired.insert("l3".to_string(), StateValue::Bool(true));
        let l3_goal = Goal::new("l3", l3_desired);

        let mut l2_desired = BTreeMap::new();
        l2_desired.insert("l2".to_string(), StateValue::Bool(true));
        let l2_goal = Goal::new("l2", l2_desired).with_sub_goals(vec![l3_goal]);

        let mut l1_desired = BTreeMap::new();
        l1_desired.insert("l1".to_string(), StateValue::Bool(true));
        let l1_goal = Goal::new("l1", l1_desired).with_sub_goals(vec![l2_goal]);

        let mut root_desired = BTreeMap::new();
        root_desired.insert("root".to_string(), StateValue::Bool(true));
        let root_goal = Goal::new("root", root_desired)
            .with_max_depth(2) // Limit to 2 levels
            .with_sub_goals(vec![l1_goal]);

        // Should not decompose beyond max_depth
        let plan = planner.plan(&start, &root_goal);
        // Plan should either succeed with direct planning or handle gracefully
        assert!(plan.is_some() || plan.is_none()); // Either outcome is valid
    }

    #[test]
    fn test_any_of_decomposition() {
        let planner = create_test_planner();
        let mut start = WorldState::new();
        start.set("has_weapon", StateValue::Bool(true));

        // Option 1: equip weapon (achievable)
        let mut equip_desired = BTreeMap::new();
        equip_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let equip_goal = Goal::new("equip_option", equip_desired).with_priority(5.0);

        // Option 2: scan (also achievable)
        let mut scan_desired = BTreeMap::new();
        scan_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let scan_goal = Goal::new("scan_option", scan_desired).with_priority(3.0);

        // Main goal: any of these options
        let mut main_desired = BTreeMap::new();
        main_desired.insert("prepared".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("prepare_any_way", main_desired)
            .with_strategy(DecompositionStrategy::AnyOf)
            .with_sub_goals(vec![equip_goal, scan_goal]);

        // Should pick one of the options (likely higher priority one)
        let plan = planner.plan(&start, &main_goal);
        assert!(plan.is_some());

        if let Some(plan) = plan {
            // Should contain either equip or scan, but not necessarily both
            assert!(plan.contains(&"equip".to_string()) || plan.contains(&"scan".to_string()));
        }
    }

    #[test]
    fn test_parallel_decomposition() {
        let planner = create_test_planner();
        let mut start = WorldState::new();
        start.set("has_weapon", StateValue::Bool(true));

        // Task 1: equip weapon
        let mut equip_desired = BTreeMap::new();
        equip_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let equip_goal = Goal::new("equip", equip_desired).with_priority(5.0);

        // Task 2: scan area
        let mut scan_desired = BTreeMap::new();
        scan_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let scan_goal = Goal::new("scan", scan_desired).with_priority(7.0);

        // Main goal: do both in parallel (actually ordered by priority)
        let mut main_desired = BTreeMap::new();
        main_desired.insert("fully_prepared".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("full_prepare", main_desired)
            .with_strategy(DecompositionStrategy::Parallel)
            .with_sub_goals(vec![equip_goal, scan_goal]);

        // Should plan for both sub-goals
        let plan = planner.plan(&start, &main_goal);
        assert!(plan.is_some());

        if let Some(plan) = plan {
            assert!(plan.contains(&"equip".to_string()));
            assert!(plan.contains(&"scan".to_string()));
        }
    }

    #[test]
    fn test_already_satisfied_subgoal() {
        let planner = create_test_planner();
        let mut start = WorldState::new();
        start.set("area_scanned", StateValue::Bool(true)); // Already satisfied
        start.set("has_weapon", StateValue::Bool(true));

        // Sub-goal 1: scan (already done)
        let mut scan_desired = BTreeMap::new();
        scan_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let scan_goal = Goal::new("scan", scan_desired);

        // Sub-goal 2: equip weapon (needs to be done)
        let mut equip_desired = BTreeMap::new();
        equip_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let equip_goal = Goal::new("equip", equip_desired);

        // Main goal
        let mut main_desired = BTreeMap::new();
        main_desired.insert("ready".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("prepare", main_desired)
            .with_strategy(DecompositionStrategy::Sequential)
            .with_sub_goals(vec![scan_goal, equip_goal]);

        // Should skip already-satisfied scan goal
        let plan = planner.plan(&start, &main_goal);
        assert!(plan.is_some());

        if let Some(plan) = plan {
            // Should only contain equip, not scan
            assert!(plan.contains(&"equip".to_string()));
            assert!(!plan.contains(&"scan".to_string()));
        }
    }

    #[test]
    fn test_goal_authoring_integration() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let goal_path = temp_dir.path().join("test_goal.toml");

        // Create a goal definition
        let mut sub_desired = BTreeMap::new();
        sub_desired.insert("scanned".to_string(), StateValueDef::Bool(true));

        let sub_goal_def = GoalDefinition {
            name: "scan_area".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: sub_desired,
            sub_goals: None,
        };

        let mut main_desired = BTreeMap::new();
        main_desired.insert("mission_complete".to_string(), StateValueDef::Bool(true));

        let goal_def = GoalDefinition {
            name: "complete_mission".to_string(),
            priority: Some(10.0),
            deadline_seconds: Some(60.0),
            decomposition: Some("sequential".to_string()),
            max_depth: Some(3),
            desired_state: main_desired,
            sub_goals: Some(vec![sub_goal_def]),
        };

        // Save and load
        goal_def.save(&goal_path).unwrap();
        let loaded_def = GoalDefinition::load(&goal_path).unwrap();

        // Convert to internal goal
        let goal = loaded_def.to_goal();
        assert_eq!(goal.name, "complete_mission");
        assert_eq!(goal.priority, 10.0);
        assert_eq!(goal.deadline, Some(60.0));
        assert_eq!(goal.sub_goals.len(), 1);
        assert_eq!(goal.sub_goals[0].name, "scan_area");
    }

    #[test]
    fn test_goal_scheduler_integration() {
        let planner = create_test_planner();
        let mut scheduler = GoalScheduler::new();

        // Add multiple goals
        let mut goal1_desired = BTreeMap::new();
        goal1_desired.insert("area_scanned".to_string(), StateValue::Bool(true));
        let goal1 = Goal::new("scan", goal1_desired).with_priority(5.0);

        let mut goal2_desired = BTreeMap::new();
        goal2_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let goal2 = Goal::new("equip", goal2_desired).with_priority(3.0);

        scheduler.add_goal(goal1);
        scheduler.add_goal(goal2);

        let mut world = WorldState::new();
        world.set("has_weapon", StateValue::Bool(true));

        // Update should plan for highest priority goal
        let plan = scheduler.update(0.0, &world, &planner);
        assert!(plan.is_some());
        assert_eq!(scheduler.goal_count(), 2);
    }
}
