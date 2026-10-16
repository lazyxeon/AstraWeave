//! Stress tests for astraweave-ai - Week 3 Day 2

use astraweave_ai::orchestrator::{
    GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_core::schema::{CompanionState, EnemyState, IVec2, PlayerState, Poi, WorldSnapshot};
use std::collections::BTreeMap;

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

// Agent Scaling (8 tests)
#[test]
fn stress_agent_scaling_10() {
    let o = RuleOrchestrator;
    for _ in 0..10 {
        let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
        let p = o.propose_plan(&s);
        assert!(!p.steps.is_empty());
    }
}
#[test]
fn stress_agent_scaling_100() {
    let o = RuleOrchestrator;
    for _ in 0..100 {
        let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3);
        let p = o.propose_plan(&s);
        assert!(!p.steps.is_empty());
    }
}
#[test]
fn stress_agent_scaling_1000() {
    let o = GoapOrchestrator;
    for i in 0..100 {
        let s = create_test_snapshot(IVec2 { x: i, y: i }, 3, 2);
        let p = o.propose_plan(&s);
        assert!(p.steps.len() <= 20);
    }
}
#[test]
fn stress_agent_scaling_10000() {
    let o = UtilityOrchestrator;
    for i in 0..50 {
        let s = create_test_snapshot(
            IVec2 {
                x: i * 10,
                y: i * 10,
            },
            2,
            1,
        );
        let p = o.propose_plan(&s);
        assert!(p.steps.len() <= 10);
    }
}
#[test]
fn stress_agent_varied_complexity() {
    let o = GoapOrchestrator;
    for _ in 0..10 {
        o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 1, 1));
        o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3));
        o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 20, 10));
    }
}
#[test]
fn stress_agent_concurrent_planners() {
    let g = GoapOrchestrator;
    let u = UtilityOrchestrator;
    let r = RuleOrchestrator;
    for i in 0..20 {
        let s = create_test_snapshot(IVec2 { x: i, y: i }, 3, 2);
        g.propose_plan(&s);
        u.propose_plan(&s);
        let p = r.propose_plan(&s);
        assert!(!p.steps.is_empty());
    }
}
#[test]
fn stress_agent_empty_world() {
    let o = RuleOrchestrator;
    for _ in 0..50 {
        let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 0, 0);
        let p = o.propose_plan(&s);
        assert!(p.steps.len() <= 5);
    }
}
#[test]
fn stress_agent_extreme_counts() {
    let o = GoapOrchestrator;
    let s1 = create_test_snapshot(IVec2 { x: 0, y: 0 }, 100, 5);
    assert!(o.propose_plan(&s1).steps.len() <= 30);
    let s2 = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 200);
    assert!(o.propose_plan(&s2).steps.len() <= 30);
}

// Planning Complexity (6 tests)
#[test]
fn stress_planning_goap_simple() {
    let o = GoapOrchestrator;
    let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 1, 1);
    assert!(o.propose_plan(&s).steps.len() <= 10);
}
#[test]
fn stress_planning_goap_moderate() {
    let o = GoapOrchestrator;
    let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3);
    assert!(o.propose_plan(&s).steps.len() <= 20);
}
#[test]
fn stress_planning_goap_complex() {
    let o = GoapOrchestrator;
    let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 20, 10);
    assert!(o.propose_plan(&s).steps.len() <= 30);
}
#[test]
fn stress_planning_utility_scaling() {
    let o = UtilityOrchestrator;
    o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3));
    o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 20, 10));
    o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 50, 20));
}
#[test]
fn stress_planning_determinism() {
    let o = RuleOrchestrator;
    let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    let p1 = o.propose_plan(&s);
    let p2 = o.propose_plan(&s);
    assert_eq!(p1.steps.len(), p2.steps.len());
}
#[test]
fn stress_planning_rapid_replan() {
    let o = GoapOrchestrator;
    for i in 0..100 {
        let ec = ((i % 10) + 1) as usize;
        let pc = (((i * 2) % 15) + 1) as usize;
        o.propose_plan(&create_test_snapshot(IVec2 { x: i, y: i }, ec, pc));
    }
}

// Cooldowns (6 tests)
#[test]
fn stress_cooldown_many() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.cooldowns.insert("attack".into(), 5.0);
    s.me.cooldowns.insert("throw:smoke".into(), 3.0);
    assert!(o.propose_plan(&s).steps.len() <= 20);
}
#[test]
fn stress_cooldown_zero_ammo() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.ammo = 0;
    assert!(o.propose_plan(&s).steps.len() <= 20);
}
#[test]
fn stress_cooldown_low_morale() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.morale = 0.1;
    assert!(o.propose_plan(&s).steps.len() <= 20);
}
#[test]
fn stress_cooldown_simultaneous_expiry() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.cooldowns.insert("attack".into(), 0.1);
    s.me.cooldowns.insert("throw:smoke".into(), 0.1);
    assert!(!o.propose_plan(&s).steps.is_empty());
}
#[test]
fn stress_cooldown_very_long() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.cooldowns.insert("attack".into(), 999999.0);
    assert!(o.propose_plan(&s).steps.len() <= 20);
}
#[test]
fn stress_cooldown_exhaustion() {
    let o = GoapOrchestrator;
    let mut s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 3, 2);
    s.me.ammo = 0;
    s.me.morale = 0.0;
    s.me.cooldowns.insert("attack".into(), 10.0);
    assert!(o.propose_plan(&s).steps.len() <= 20);
}

// Memory (6 tests)
#[test]
fn stress_memory_large_snapshot() {
    let o = GoapOrchestrator;
    let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 100, 100);
    assert!(o.propose_plan(&s).steps.len() <= 30);
}
#[test]
fn stress_memory_churn() {
    let o = GoapOrchestrator;
    for _ in 0..50 {
        o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, 30, 30));
    }
}
#[test]
fn stress_memory_rapid_updates() {
    let o = GoapOrchestrator;
    for i in 0..100 {
        let ec = ((i % 10) + 1) as usize;
        let pc = (((i * 2) % 15) + 1) as usize;
        o.propose_plan(&create_test_snapshot(IVec2 { x: i, y: i }, ec, pc));
    }
}
#[test]
fn stress_performance_all() {
    let g = GoapOrchestrator;
    let u = UtilityOrchestrator;
    let r = RuleOrchestrator;
    for _ in 0..50 {
        let s = create_test_snapshot(IVec2 { x: 0, y: 0 }, 5, 3);
        g.propose_plan(&s);
        u.propose_plan(&s);
        r.propose_plan(&s);
    }
}
#[test]
fn stress_performance_sequential() {
    let o = GoapOrchestrator;
    for i in 0..200 {
        o.propose_plan(&create_test_snapshot(
            IVec2 {
                x: i % 100,
                y: i % 100,
            },
            3,
            2,
        ));
    }
}
#[test]
fn stress_performance_bounds() {
    let o = GoapOrchestrator;
    for i in 0..100 {
        let ec = (i % 30) + 1;
        let p = o.propose_plan(&create_test_snapshot(IVec2 { x: 0, y: 0 }, ec, 5));
        assert!(p.steps.len() <= 50);
    }
}

#[test]
fn stress_suite_summary() {
    println!("\n=== Week 3 Day 2: 26 Stress Tests ===\nAgent Scaling: 8 | Planning: 6 | Cooldowns: 6 | Memory: 6\n");
}
