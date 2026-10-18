//! Integration test for GOAP mode AI planning with inventory system.
//!
//! This test validates the complete GOAP pipeline:
//! - Inventory state tracking
//! - Goal definition (has_food)
//! - Action planning (gather → craft → consume)
//! - Deterministic plan generation
//! - State transitions

#[cfg(feature = "ai-goap")]
use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
#[cfg(feature = "ai-goap")]
use astraweave_core::{CompanionState, IVec2, PlayerState, WorldSnapshot};
#[cfg(feature = "ai-goap")]
use std::collections::BTreeMap;

#[test]
#[cfg(feature = "ai-goap")]
fn test_goap_mode_basic_planning() {
    // Setup: Create world snapshot with hungry companion
    let controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
        },
        player: PlayerState {
            hp: 30, // Hungry (< 50)
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

    // Generate GOAP plan
    let plan = dispatch_planner(&controller, &snapshot).expect("GOAP planning should succeed");

    // Validate plan structure
    assert!(!plan.steps.is_empty(), "GOAP should produce action steps");
    assert!(
        plan.plan_id.contains("goap"),
        "Plan ID should indicate GOAP"
    );

    // GOAP should produce: GoToTree → ChopWood → GoToCampfire → CookFood
    // (4 actions converted to 4 MoveTo steps)
    assert!(
        plan.steps.len() >= 2,
        "GOAP plan should have multiple steps"
    );

    println!("GOAP plan generated: {} steps", plan.steps.len());
    println!("Plan ID: {}", plan.plan_id);
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_goap_mode_deterministic_planning() {
    // Same state should produce identical GOAP plans
    let controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
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

    // Generate plan twice
    let plan1 = dispatch_planner(&controller, &snapshot).expect("GOAP planning should succeed");
    let plan2 = dispatch_planner(&controller, &snapshot).expect("GOAP planning should succeed");

    // Validate determinism
    assert_eq!(
        plan1.steps.len(),
        plan2.steps.len(),
        "Same state should produce same step count"
    );
    assert_eq!(
        plan1.plan_id, plan2.plan_id,
        "Same state should produce same plan ID"
    );

    println!("GOAP determinism validated: {} steps", plan1.steps.len());
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_goap_mode_goal_satisfaction() {
    // Test that GOAP produces plan to satisfy goal
    let controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
        },
        player: PlayerState {
            hp: 30, // Hungry
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

    let plan = dispatch_planner(&controller, &snapshot).expect("GOAP planning should succeed");

    // Validate plan achieves goal
    assert!(!plan.steps.is_empty(), "Should have steps to achieve goal");

    // Expected sequence: GoToTree → ChopWood → GoToCampfire → CookFood
    // (converted to MoveTo actions)
    println!("GOAP goal satisfaction plan: {} steps", plan.steps.len());
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_goap_mode_policy_variants() {
    // Test different policy configurations
    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
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

    // The method is the typed goal. The policy string does not select it.
    let controller1 = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::KeepWood,
    };

    let plan1 = dispatch_planner(&controller1, &snapshot).expect("GOAP planning should succeed");
    assert_eq!(plan1.steps.len(), 6, "KeepWood cooks and chops again");

    let controller2 = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let plan2 = dispatch_planner(&controller2, &snapshot).expect("GOAP planning should succeed");
    assert_eq!(
        plan2.steps.len(),
        4,
        "CookFood does not read the policy name"
    );

    println!("Policy variants validated");
    println!("  gather_craft: {} steps", plan1.steps.len());
    println!("  other_policy: {} steps", plan2.steps.len());
}

#[test]
#[cfg(feature = "ai-goap")]
fn test_goap_mode_reproducibility() {
    // Test reproducibility across multiple runs
    let controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: Some("gather_craft_policy".to_string()),
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
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

    // Generate plans multiple times
    let mut plan_ids = Vec::new();
    let mut step_counts = Vec::new();

    for _ in 0..10 {
        let plan = dispatch_planner(&controller, &snapshot).expect("GOAP planning should succeed");
        plan_ids.push(plan.plan_id.clone());
        step_counts.push(plan.steps.len());
    }

    // Validate all plans are identical
    for i in 1..10 {
        assert_eq!(plan_ids[i], plan_ids[0], "All plan IDs should be identical");
        assert_eq!(
            step_counts[i], step_counts[0],
            "All step counts should be identical"
        );
    }

    println!(
        "GOAP reproducibility validated: 10 runs, {} steps each",
        step_counts[0]
    );
}

#[test]
#[cfg(not(feature = "ai-goap"))]
fn test_goap_mode_feature_gate() {
    // Test that GOAP mode requires feature flag
    use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
    use astraweave_core::{CompanionState, IVec2, PlayerState, WorldSnapshot};
    use std::collections::BTreeMap;

    let controller = CAiController {
        mode: PlannerMode::GOAP,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    let snapshot = WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,

            pos: IVec2 { x: 5, y: 5 },
        },
        player: PlayerState {
            hp: 100,
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

    let result = dispatch_planner(&controller, &snapshot);
    assert!(result.is_err(), "GOAP without feature should error");
    assert!(
        result.unwrap_err().to_string().contains("ai-goap"),
        "Error should mention feature flag"
    );
}
