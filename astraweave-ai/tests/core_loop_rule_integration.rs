//! Integration test for Rule mode AI planning with full ECS loop.
//!
//! This test validates the complete pipeline:
//! - ECS world setup with entities
//! - CAiController component with Rule mode
//! - Snapshot generation from world state
//! - Rule orchestrator planning
//! - Action execution and state changes
//! - Deterministic behavior over N ticks

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{
    build_snapshot, CompanionState, IVec2, PerceptionConfig, PlayerState, Team, World,
    WorldSnapshot,
};
use std::collections::BTreeMap;

/// Helper to create a deterministic test world
fn create_test_world() -> World {
    let mut world = World::new();

    // Add player (team 0)
    let _player = world.spawn("player", IVec2 { x: 5, y: 5 }, Team { id: 0 }, 100, 10);

    // Add companion (team 1) - this will be AI-controlled
    let _companion = world.spawn("companion", IVec2 { x: 5, y: 7 }, Team { id: 1 }, 100, 10);

    // Add enemy (team 2) - target for combat
    let _enemy = world.spawn("enemy", IVec2 { x: 10, y: 10 }, Team { id: 2 }, 100, 10);

    // Add some obstacles for LOS checks
    world.obstacles.insert((8, 8));
    world.obstacles.insert((8, 9));

    world
}

/// Helper to build snapshot for AI entity
fn build_ai_snapshot(world: &World, companion_id: u32) -> WorldSnapshot {
    let player = world
        .all_of_team(0)
        .first()
        .copied()
        .expect("Player should exist");
    let enemies = world.enemies_of(1);

    build_snapshot(
        world,
        player,
        companion_id,
        &enemies,
        None,
        &PerceptionConfig { los_max: 15 },
    )
}

#[test]
fn test_rule_mode_deterministic_planning() {
    // Setup: Create world with player, companion, enemy
    let world = create_test_world();
    let companion_id = world
        .all_of_team(1)
        .first()
        .copied()
        .expect("Companion should exist");

    // Create AI controller in Rule mode
    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    // Run planning cycle
    let snapshot = build_ai_snapshot(&world, companion_id);
    let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

    // Validate plan structure
    assert!(
        !plan.steps.is_empty(),
        "Rule orchestrator should produce steps"
    );
    assert!(
        plan.plan_id.starts_with("plan-"),
        "Plan should have ID prefix"
    );

    // Rule orchestrator should produce smoke throw + move + cover fire sequence
    // (since smoke cooldown is 0 initially)
    assert!(plan.steps.len() >= 2, "Should have multiple steps");

    println!("Plan generated: {} steps", plan.steps.len());
    println!("Plan ID: {}", plan.plan_id);
}

#[test]
fn test_rule_mode_multi_tick_determinism() {
    // Setup
    let mut world = create_test_world();
    let companion_id = world
        .all_of_team(1)
        .first()
        .copied()
        .expect("Companion should exist");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    // Track state over multiple ticks
    let mut plan_ids = Vec::new();
    let mut plan_step_counts = Vec::new();

    for tick in 0..5 {
        // Advance world time using tick()
        world.tick(0.1);

        // Generate plan
        let snapshot = build_ai_snapshot(&world, companion_id);
        let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

        plan_ids.push(plan.plan_id.clone());
        plan_step_counts.push(plan.steps.len());

        // Apply cooldowns (simulate smoke throw)
        if tick == 0 {
            if let Some(cds) = world.cooldowns_mut(companion_id) {
                cds.map.insert("throw:smoke".to_string(), 3.0);
            }
        }
    }

    // Validate deterministic behavior
    println!("Plan sequence over 5 ticks:");
    for (i, (id, count)) in plan_ids.iter().zip(plan_step_counts.iter()).enumerate() {
        println!("  Tick {}: {} (steps: {})", i, id, count);
    }

    // First plan should have 3 steps (smoke + move + cover)
    assert_eq!(
        plan_step_counts[0], 3,
        "First plan should have 3 steps (smoke ready)"
    );

    // Subsequent plans should have 2 steps (move + cover, smoke on cooldown)
    for step_count in plan_step_counts.iter().skip(1) {
        assert_eq!(
            *step_count, 2,
            "Plans with smoke on cooldown should have 2 steps"
        );
    }
}

#[test]
fn test_rule_mode_no_enemies() {
    // Setup: World with no enemies (companion should hold position)
    let mut world = World::new();
    let _player = world.spawn("player", IVec2 { x: 5, y: 5 }, Team { id: 0 }, 100, 10);
    let _companion = world.spawn("companion", IVec2 { x: 5, y: 7 }, Team { id: 1 }, 100, 10);

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    // Build snapshot with no enemies
    let snapshot = WorldSnapshot {
        t: 0.0,
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
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    // Plan with no enemies
    let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

    // Rule orchestrator returns empty plan when no enemies
    assert!(
        plan.steps.is_empty(),
        "No enemies should produce empty plan"
    );
}

#[test]
fn test_rule_mode_golden_trace() {
    // Golden trace test: validate exact sequence over known scenario
    let mut world = create_test_world();
    let companion_id = world
        .all_of_team(1)
        .first()
        .copied()
        .expect("Companion should exist");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    // Tick 0: Initial plan with smoke available
    // (World starts at t=0.0 by default)
    let snapshot = build_ai_snapshot(&world, companion_id);
    let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

    // Golden expectations for tick 0
    assert_eq!(plan.steps.len(), 3, "T=0: smoke + move + cover");
    let plan_id_t0 = plan.plan_id.clone();

    // Simulate smoke throw (set cooldown)
    if let Some(cds) = world.cooldowns_mut(companion_id) {
        cds.map.insert("throw:smoke".to_string(), 3.0);
    }

    // Tick 1: Plan with smoke on cooldown
    world.tick(0.1);
    let snapshot = build_ai_snapshot(&world, companion_id);
    let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

    assert_eq!(plan.steps.len(), 2, "T=0.1: move + cover (no smoke)");
    let plan_id_t1 = plan.plan_id.clone();

    // Validate plan IDs changed (different timestamps)
    assert_ne!(
        plan_id_t0, plan_id_t1,
        "Different timestamps should produce different plan IDs"
    );

    // Tick 2: Still on cooldown (tick advances time and reduces cooldowns)
    world.tick(0.1);
    let snapshot = build_ai_snapshot(&world, companion_id);
    let plan = dispatch_planner(&controller, &snapshot).expect("Planning should succeed");

    assert_eq!(plan.steps.len(), 2, "T=0.2: still on cooldown");

    println!("Golden trace validation passed");
    println!("  T=0.0: {} steps, plan_id={}", 3, plan_id_t0);
    println!("  T=0.1: {} steps, plan_id={}", 2, plan_id_t1);
}

#[test]
fn test_rule_mode_reproducibility() {
    // Same world state should produce identical plans
    let world1 = create_test_world();
    let world2 = create_test_world();

    let companion_id = world1
        .all_of_team(1)
        .first()
        .copied()
        .expect("Companion should exist");

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
        gather: astraweave_ai::htn::GatherGoal::CookFood,
    };

    // Generate plans from identical worlds
    let snapshot1 = build_ai_snapshot(&world1, companion_id);
    let plan1 = dispatch_planner(&controller, &snapshot1).expect("Planning should succeed");

    let snapshot2 = build_ai_snapshot(&world2, companion_id);
    let plan2 = dispatch_planner(&controller, &snapshot2).expect("Planning should succeed");

    // Validate identical output
    assert_eq!(
        plan1.steps.len(),
        plan2.steps.len(),
        "Same state should produce same step count"
    );

    // Plan IDs should be identical (same timestamp)
    assert_eq!(
        plan1.plan_id, plan2.plan_id,
        "Same state should produce same plan ID"
    );

    println!(
        "Reproducibility validated: {} steps, plan_id={}",
        plan1.steps.len(),
        plan1.plan_id
    );
}
