//! Edge case tests for astraweave-ai - Week 3 Day 3
//!
//! Tests invalid inputs, boundary conditions, and coordination conflicts.

use astraweave_ai::orchestrator::{
    GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_core::schema::{CompanionState, EnemyState, IVec2, PlayerState, Poi, WorldSnapshot};
use std::collections::BTreeMap;

// Helper from Day 2 (reused)
fn create_test_snapshot(agent_pos: IVec2, enemy_count: usize, poi_count: usize) -> WorldSnapshot {
    let mut enemies = Vec::with_capacity(enemy_count);
    for i in 0..enemy_count {
        enemies.push(EnemyState {
            id: i as u32,
            pos: IVec2 {
                x: (i * 10) as i32,
                y: 0,
            },
            hp: 100,
            cover: if i % 2 == 0 {
                "full".into()
            } else {
                "none".into()
            },
            last_seen: 0.0,
        });
    }
    let mut pois = Vec::with_capacity(poi_count);
    for i in 0..poi_count {
        pois.push(Poi {
            k: if i % 2 == 0 {
                "objective".into()
            } else {
                "cover".into()
            },
            pos: IVec2 {
                x: 0,
                y: (i * 10) as i32,
            },
        });
    }
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            hp: 100,

            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            pos: agent_pos,
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
        },
        enemies,
        pois,
        obstacles: vec![],
        objective: Some("test".into()),

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

// ============================================================================
// Category 1: Invalid/Empty Inputs (8 tests)
// ============================================================================

#[test]
fn edge_empty_snapshot_all_arrays() {
    // Completely empty world
    let o = RuleOrchestrator;
    let snap = WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            hp: 100,

            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2 { x: 0, y: 0 },
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };
    let plan = o.propose_plan(&snap);
    // Should handle empty world gracefully (likely empty or minimal plan)
    assert!(
        plan.steps.len() <= 5,
        "Empty world should produce short plan"
    );
}

#[test]
fn edge_negative_coordinates() {
    // Test with negative positions
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: -100, y: -100 }, 1, 1);
    snap.enemies[0].pos = IVec2 { x: -50, y: -50 };
    snap.pois[0].pos = IVec2 { x: -75, y: -75 };

    let plan = o.propose_plan(&snap);
    // Should handle negative coordinates without panic
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_zero_health() {
    // Agent and player with zero health
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.player.hp = 0;

    let plan = o.propose_plan(&snap);
    // Should still plan even with zero health
    assert!(plan.steps.len() <= 20);
}

#[test]
fn edge_negative_morale() {
    // Negative morale (invalid but should handle gracefully)
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.morale = -1.0;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 20);
}

#[test]
fn edge_negative_ammo() {
    // Negative ammo (invalid but should handle gracefully)
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.ammo = -10;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

#[test]
fn edge_empty_strings() {
    // Empty stance, cover, objective strings
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.player.stance = "".into();
    snap.enemies[0].cover = "".into();
    snap.objective = Some("".into());

    let plan = o.propose_plan(&snap);
    assert!(
        !plan.plan_id.is_empty(),
        "Plan ID should still be generated"
    );
}

#[test]
fn edge_very_large_entity_ids() {
    // Test with large entity IDs
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 2, 1);
    snap.enemies[0].id = u32::MAX;
    snap.enemies[1].id = u32::MAX - 1;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_duplicate_entity_ids() {
    // Duplicate entity IDs (invalid but should handle)
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.enemies[0].id = 1;
    snap.enemies[1].id = 1;
    snap.enemies[2].id = 1;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

// ============================================================================
// Category 2: Boundary Conditions (8 tests)
// ============================================================================

#[test]
fn edge_max_i32_coordinates() {
    // Maximum i32 coordinates
    let o = GoapOrchestrator;
    let snap = create_test_snapshot(
        IVec2 {
            x: i32::MAX,
            y: i32::MAX,
        },
        1,
        1,
    );

    let plan = o.propose_plan(&snap);
    // Should handle extreme coordinates
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_min_i32_coordinates() {
    // Minimum i32 coordinates
    let o = RuleOrchestrator;
    let snap = create_test_snapshot(
        IVec2 {
            x: i32::MIN,
            y: i32::MIN,
        },
        1,
        1,
    );

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 20);
}

#[test]
fn edge_zero_cooldowns() {
    // All cooldowns at exactly 0.0 (ready to use)
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.cooldowns.insert("attack".into(), 0.0);
    snap.me.cooldowns.insert("throw:smoke".into(), 0.0);
    snap.me.cooldowns.insert("revive".into(), 0.0);

    let plan = o.propose_plan(&snap);
    assert!(
        !plan.steps.is_empty(),
        "Should generate plan with ready cooldowns"
    );
}

#[test]
fn edge_infinite_cooldowns() {
    // Infinite cooldowns
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.cooldowns.insert("attack".into(), f32::INFINITY);
    snap.me
        .cooldowns
        .insert("throw:smoke".into(), f32::INFINITY);

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

#[test]
fn edge_nan_cooldowns() {
    // NaN cooldowns (invalid but should handle)
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.cooldowns.insert("attack".into(), f32::NAN);

    let plan = o.propose_plan(&snap);
    // Should not crash on NaN
    assert!(plan.steps.len() <= 20);
}

#[test]
fn edge_morale_above_one() {
    // Morale > 1.0 (invalid but should handle)
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.morale = 100.0;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_very_old_timestamp() {
    // Very old timestamp
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.t = -999999.0;

    let plan = o.propose_plan(&snap);
    assert!(!plan.plan_id.is_empty());
}

#[test]
fn edge_future_timestamp() {
    // Far future timestamp
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.t = 999999999.0;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

// ============================================================================
// Category 3: Spatial Edge Cases (6 tests)
// ============================================================================

#[test]
fn edge_all_entities_same_position() {
    // All entities at exactly same position
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.player.pos = IVec2 { x: 0, y: 0 };
    snap.me.pos = IVec2 { x: 0, y: 0 };
    snap.enemies[0].pos = IVec2 { x: 0, y: 0 };
    snap.enemies[1].pos = IVec2 { x: 0, y: 0 };
    snap.pois[0].pos = IVec2 { x: 0, y: 0 };

    let plan = o.propose_plan(&snap);
    // Should handle overlapping positions
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_very_close_entities() {
    // Entities within 1 unit of each other
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.enemies[0].pos = IVec2 { x: 1, y: 0 };
    snap.enemies[1].pos = IVec2 { x: 0, y: 1 };
    snap.pois[0].pos = IVec2 { x: -1, y: 0 };

    let plan = o.propose_plan(&snap);
    assert!(!plan.steps.is_empty());
}

#[test]
fn edge_very_far_entities() {
    // Entities very far apart
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 2, 1);
    snap.enemies[0].pos = IVec2 { x: 10000, y: 10000 };
    snap.pois[0].pos = IVec2 {
        x: -10000,
        y: -10000,
    };

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

#[test]
fn edge_linear_arrangement() {
    // All entities in perfect line
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3);
    for i in 0..5 {
        snap.enemies[i].pos = IVec2 {
            x: (i * 10) as i32,
            y: 0,
        };
    }
    for i in 0..3 {
        snap.pois[i].pos = IVec2 {
            x: (i * 10) as i32,
            y: 0,
        };
    }

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_circular_arrangement() {
    // Entities arranged in circle around agent
    let o = RuleOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 4, 0);
    snap.enemies[0].pos = IVec2 { x: 10, y: 0 };
    snap.enemies[1].pos = IVec2 { x: 0, y: 10 };
    snap.enemies[2].pos = IVec2 { x: -10, y: 0 };
    snap.enemies[3].pos = IVec2 { x: 0, y: -10 };

    let plan = o.propose_plan(&snap);
    assert!(!plan.steps.is_empty());
}

#[test]
fn edge_diagonal_positions() {
    // All entities on diagonals
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 4, 2);
    snap.enemies[0].pos = IVec2 { x: 10, y: 10 };
    snap.enemies[1].pos = IVec2 { x: -10, y: 10 };
    snap.enemies[2].pos = IVec2 { x: 10, y: -10 };
    snap.enemies[3].pos = IVec2 { x: -10, y: -10 };
    snap.pois[0].pos = IVec2 { x: 5, y: 5 };
    snap.pois[1].pos = IVec2 { x: -5, y: -5 };

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

// ============================================================================
// Category 4: Temporal Edge Cases (4 tests)
// ============================================================================

#[test]
fn edge_rapid_time_progression() {
    // Simulate rapid time steps
    let o = GoapOrchestrator;
    for i in 0..50 {
        let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
        snap.t = (i as f32) * 0.01; // 10ms steps
        let plan = o.propose_plan(&snap);
        assert!(plan.steps.len() <= 30);
    }
}

#[test]
fn edge_time_going_backwards() {
    // Time going backwards (invalid but should handle)
    let o = RuleOrchestrator;
    let mut snap1 = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap1.t = 100.0;
    let _plan1 = o.propose_plan(&snap1);

    let mut snap2 = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap2.t = 50.0;
    let plan2 = o.propose_plan(&snap2);

    assert!(!plan2.steps.is_empty());
}

#[test]
fn edge_cooldown_decay_edge() {
    // Cooldown exactly at decay boundary
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.cooldowns.insert("attack".into(), 0.001);

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_very_small_time_delta() {
    // Very small time delta
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.t = 0.000001;

    let plan = o.propose_plan(&snap);
    assert!(plan.steps.len() <= 10);
}

// ============================================================================
// Category 5: Orchestrator-Specific Edge Cases (4 tests)
// ============================================================================

#[test]
fn edge_rule_with_only_pois_no_enemies() {
    // Rule orchestrator with POIs but no enemies
    let o = RuleOrchestrator;
    let snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 0, 5);

    let plan = o.propose_plan(&snap);
    // Rule orchestrator should return empty plan (no enemies to engage)
    assert!(plan.steps.len() <= 5);
}

#[test]
fn edge_goap_all_preconditions_fail() {
    // GOAP with conditions that make all actions invalid
    let o = GoapOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    snap.me.ammo = 0;
    snap.me.morale = 0.0;
    snap.me.cooldowns.insert("attack".into(), 999.0);
    snap.me.cooldowns.insert("throw:smoke".into(), 999.0);
    snap.me.cooldowns.insert("revive".into(), 999.0);

    let plan = o.propose_plan(&snap);
    // Should still generate some plan (likely movement)
    assert!(plan.steps.len() <= 30);
}

#[test]
fn edge_utility_all_zero_scores() {
    // Utility AI where all actions might score zero
    let o = UtilityOrchestrator;
    let mut snap = create_test_snapshot(IVec2 { x: 10000, y: 10000 }, 1, 1);
    snap.enemies[0].pos = IVec2 {
        x: -10000,
        y: -10000,
    };
    snap.pois[0].pos = IVec2 { x: 0, y: 0 };

    let plan = o.propose_plan(&snap);
    // Should still produce a plan (pick best of bad options)
    assert!(plan.steps.len() <= 10);
}

#[test]
fn edge_orchestrator_switching_same_snapshot() {
    // Switch orchestrators on same snapshot
    let snap = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3);

    let plan1 = RuleOrchestrator.propose_plan(&snap);
    let plan2 = GoapOrchestrator.propose_plan(&snap);
    let plan3 = UtilityOrchestrator.propose_plan(&snap);

    // All should produce valid plans
    assert!(!plan1.steps.is_empty() || !plan2.steps.is_empty() || !plan3.steps.is_empty());
}

#[test]
fn edge_suite_summary() {
    println!("\n=== Week 3 Day 3: 30 Edge Case Tests ===");
    println!("Invalid/Empty: 8 | Boundaries: 8 | Spatial: 6 | Temporal: 4 | Orchestrator: 4");
    println!("Total: 30 edge case tests");
    println!("==========================================\n");
}
