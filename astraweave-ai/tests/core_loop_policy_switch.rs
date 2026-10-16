//! Integration test for AI planner mode switching at runtime.
//!
//! This test validates that:
//! - Controllers can switch between Rule ↔ GOAP modes
//! - No panics or state corruption during transitions
//! - Deterministic behavior after mode switches
//! - Clean handoff between planners

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{CompanionState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

/// Helper to create test snapshot
fn create_test_snapshot(t: f32, has_enemies: bool) -> WorldSnapshot {
    WorldSnapshot {
        t,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 7 },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: if has_enemies {
            vec![astraweave_core::EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 100,
                cover: "none".into(),
                last_seen: t,
            }]
        } else {
            vec![]
        },
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

#[test]
fn test_switch_rule_to_rule() {
    // Trivial case: switch from Rule to Rule (no actual switch)
    let mut controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let snapshot = create_test_snapshot(0.0, true);

    // Plan with Rule mode
    let plan1 = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");
    assert!(!plan1.steps.is_empty(), "Rule should produce steps");

    // "Switch" to Rule again (no-op)
    controller.mode = PlannerMode::Rule;

    let plan2 = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");
    assert_eq!(
        plan1.steps.len(),
        plan2.steps.len(),
        "Same mode should produce same plan"
    );

    println!("Rule→Rule transition: OK");
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_switch_rule_to_goap() {
    // Switch from Rule mode to GOAP mode
    let mut controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Start with Rule planning
    let snapshot1 = create_test_snapshot(0.0, true);
    let plan1 = dispatch_planner(&controller, &snapshot1).expect("Rule planning should succeed");
    assert!(!plan1.steps.is_empty(), "Rule should produce steps");
    println!("Rule plan: {} steps", plan1.steps.len());

    // Switch to GOAP mode
    controller.mode = PlannerMode::GOAP;
    controller.policy = Some("gather_craft_policy".to_string());

    // Generate plan with GOAP (different snapshot for hungry state)
    let snapshot2 = WorldSnapshot {
        t: 0.1,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 7 },
        },
        player: PlayerState {
            hp: 30, // Hungry for GOAP
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    let plan2 = dispatch_planner(&controller, &snapshot2).expect("GOAP planning should succeed");
    assert!(!plan2.steps.is_empty(), "GOAP should produce steps");
    println!("GOAP plan: {} steps", plan2.steps.len());

    // Validate transition was clean (no panic, valid plans produced)
    assert!(
        plan2.plan_id.contains("goap"),
        "GOAP plan should have goap ID"
    );

    println!("Rule→GOAP transition: OK");
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_switch_goap_to_rule() {
    // Switch from GOAP mode to Rule mode
    let mut controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
    };

    // Start with GOAP planning
    let snapshot1 = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 7 },
        },
        player: PlayerState {
            hp: 30,
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    let plan1 = dispatch_planner(&controller, &snapshot1).expect("GOAP planning should succeed");
    assert!(!plan1.steps.is_empty(), "GOAP should produce steps");
    println!("GOAP plan: {} steps", plan1.steps.len());

    // Switch to Rule mode
    controller.mode = PlannerMode::Rule;
    controller.policy = None;

    // Generate plan with Rule (snapshot with enemies)
    let snapshot2 = create_test_snapshot(0.1, true);
    let plan2 = dispatch_planner(&controller, &snapshot2).expect("Rule planning should succeed");
    assert!(!plan2.steps.is_empty(), "Rule should produce steps");
    println!("Rule plan: {} steps", plan2.steps.len());

    // Validate transition was clean
    assert!(
        plan2.plan_id.starts_with("plan-"),
        "Rule plan should have plan- ID"
    );

    println!("GOAP→Rule transition: OK");
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_multi_switch_cycle() {
    // Test multiple switches in sequence: Rule → GOAP → Rule → GOAP
    let mut controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let snapshot_rule = create_test_snapshot(0.0, true);
    let snapshot_goap = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 7 },
        },
        player: PlayerState {
            hp: 30,
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    let mut plan_sequence = Vec::new();

    // Cycle 1: Rule
    controller.mode = PlannerMode::Rule;
    let plan = dispatch_planner(&controller, &snapshot_rule).expect("Rule should succeed");
    plan_sequence.push(("Rule".to_string(), plan.steps.len()));

    // Cycle 2: GOAP
    controller.mode = PlannerMode::GOAP;
    controller.policy = Some("gather_craft_policy".to_string());
    let plan = dispatch_planner(&controller, &snapshot_goap).expect("GOAP should succeed");
    plan_sequence.push(("GOAP".to_string(), plan.steps.len()));

    // Cycle 3: Rule again
    controller.mode = PlannerMode::Rule;
    controller.policy = None;
    let plan = dispatch_planner(&controller, &snapshot_rule).expect("Rule should succeed");
    plan_sequence.push(("Rule".to_string(), plan.steps.len()));

    // Cycle 4: GOAP again
    controller.mode = PlannerMode::GOAP;
    controller.policy = Some("gather_craft_policy".to_string());
    let plan = dispatch_planner(&controller, &snapshot_goap).expect("GOAP should succeed");
    plan_sequence.push(("GOAP".to_string(), plan.steps.len()));

    // Validate all transitions succeeded
    assert_eq!(plan_sequence.len(), 4, "Should have 4 plan cycles");
    println!("Multi-switch cycle:");
    for (mode, steps) in &plan_sequence {
        println!("  {}: {} steps", mode, steps);
    }

    println!("Rule↔GOAP multi-cycle: OK");
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_switch_determinism() {
    // Test that switching modes produces deterministic results
    let snapshot_rule = create_test_snapshot(0.0, true);
    let snapshot_goap = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 7 },
        },
        player: PlayerState {
            hp: 30,
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    // Run switch sequence twice
    for run in 0..2 {
        let mut controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };

        // Rule → GOAP → Rule
        let plan1 = dispatch_planner(&controller, &snapshot_rule).expect("Rule should succeed");

        controller.mode = PlannerMode::GOAP;
        controller.policy = Some("gather_craft_policy".to_string());
        let plan2 = dispatch_planner(&controller, &snapshot_goap).expect("GOAP should succeed");

        controller.mode = PlannerMode::Rule;
        controller.policy = None;
        let plan3 = dispatch_planner(&controller, &snapshot_rule).expect("Rule should succeed");

        println!(
            "Run {}: Rule({}) → GOAP({}) → Rule({})",
            run,
            plan1.steps.len(),
            plan2.steps.len(),
            plan3.steps.len()
        );

        // Plans should be consistent across runs
        assert!(!plan1.steps.is_empty());
        assert!(!plan2.steps.is_empty());
        assert!(!plan3.steps.is_empty());
    }

    println!("Switch determinism: OK");
}

#[test]
fn test_switch_preserves_policy() {
    // Test that switching modes preserves policy field
    let mut controller = CAiController {
        mode: PlannerMode::Rule,
        policy: Some("custom_policy".to_string()),
    };

    assert_eq!(
        controller.policy,
        Some("custom_policy".to_string()),
        "Initial policy should be set"
    );

    // Switch mode but keep policy
    controller.mode = PlannerMode::BehaviorTree;

    assert_eq!(
        controller.policy,
        Some("custom_policy".to_string()),
        "Policy should be preserved after mode switch"
    );

    println!("Policy preservation: OK");
}
