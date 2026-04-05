//! Mutation-resistant tests for astraweave-director public API
//! Targets: BossDirector, PhaseDirector, PlayerBehaviorModel, CDirectorState,
//!          CTacticExecution, CDirectorMetrics, LlmDirectorConfig, TacticOutcome, integration helpers
//! Focus: exact return values, boundary conditions, off-by-one, negation, operator swaps

#![allow(clippy::identity_op, clippy::unnecessary_get_then_check)]

use astraweave_core::{
    CompanionState, DirectorBudget, DirectorOp, EnemyState, IVec2, PlayerState, WorldSnapshot,
};
use astraweave_director::*;
use std::collections::BTreeMap;

// ============================= Test Helpers =============================

fn make_snapshot(player_pos: IVec2, enemies: Vec<(IVec2, i32)>) -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            pos: player_pos,
            hp: 100,
            stance: "melee".into(),
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2 { x: 1, y: 1 },
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 100.0,
        },
        enemies: enemies
            .iter()
            .enumerate()
            .map(|(i, (pos, hp))| EnemyState {
                id: i as u32 + 1,
                pos: *pos,
                hp: *hp,
                cover: "none".into(),
                last_seen: 0.0,
            })
            .collect(),
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

fn budget(spawns: i32, terrain: i32, traps: i32) -> DirectorBudget {
    DirectorBudget {
        spawns,
        terrain_edits: terrain,
        traps,
    }
}

fn make_outcome(tactic: &str, effectiveness: f32) -> TacticOutcome {
    TacticOutcome {
        tactic_used: tactic.to_string(),
        effectiveness,
        player_response: "test".to_string(),
        counter_strategy: "counter".to_string(),
        duration_actual: 30,
        timestamp: 1000,
    }
}

fn make_tactic_plan(ops: Vec<DirectorOp>) -> TacticPlan {
    TacticPlan {
        strategy: "test_strategy".to_string(),
        reasoning: "test_reasoning".to_string(),
        operations: ops,
        difficulty_modifier: 1.0,
        expected_duration: 30,
        counter_strategies: vec![],
        fallback_plan: None,
    }
}

// ============================= BossDirector: distance threshold =============================

#[test]
fn boss_distance_8_uses_collapse_not_fortify() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 8, y: 0 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    // dist=8, NOT > 8, so collapse branch
    assert_eq!(plan.ops.len(), 1);
    assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
}

#[test]
fn boss_distance_9_uses_fortify() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 9, y: 0 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    // dist=9 > 8, so fortify branch
    assert_eq!(plan.ops.len(), 1);
    assert!(matches!(plan.ops[0], DirectorOp::Fortify { .. }));
}

#[test]
fn boss_distance_7_uses_collapse() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 4, y: 3 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    // dist = |4| + |3| = 7 <= 8
    assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
}

// ============================= BossDirector: empty budget =============================

#[test]
fn boss_empty_budget_produces_no_ops() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 0, 0));
    assert!(plan.ops.is_empty());
    assert_eq!(plan.ops.len(), 0);
}

// ============================= BossDirector: spawn details =============================

#[test]
fn boss_spawn_wave_exact_values() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 10, y: 20 }, vec![(IVec2 { x: 12, y: 21 }, 50)]);
    let plan = d.plan(&snap, &budget(1, 0, 0));
    assert_eq!(plan.ops.len(), 1);
    if let DirectorOp::SpawnWave {
        archetype,
        count,
        origin,
    } = &plan.ops[0]
    {
        assert_eq!(archetype, "minion");
        assert_eq!(*count, 3);
        assert_ne!(*count, 2);
        assert_ne!(*count, 4);
        assert_eq!(origin.x, 10 - 2); // player.x - 2
        assert_eq!(origin.y, 20 + 1); // player.y + 1
    } else {
        panic!("Expected SpawnWave");
    }
}

// ============================= BossDirector: fortify rect =============================

#[test]
fn boss_fortify_rect_exact_midpoint() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 20, y: 10 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    // dist = 30 > 8, fortify
    if let DirectorOp::Fortify { rect } = &plan.ops[0] {
        let xm = (0 + 20) / 2; // 10
        let ym = (0 + 10) / 2; // 5
        assert_eq!(rect.x0, xm - 1);
        assert_eq!(rect.y0, ym - 1);
        assert_eq!(rect.x1, xm + 1);
        assert_eq!(rect.y1, ym + 1);
        assert_eq!(rect.x0, 9);
        assert_eq!(rect.y0, 4);
        assert_eq!(rect.x1, 11);
        assert_eq!(rect.y1, 6);
    } else {
        panic!("Expected Fortify");
    }
}

// ============================= BossDirector: both spawn+collapse =============================

#[test]
fn boss_full_budget_close_range_spawn_and_collapse() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 3, y: 3 }, 50)]);
    let plan = d.plan(&snap, &budget(5, 5, 5));
    // dist = 6 <= 8, so spawn + collapse
    assert_eq!(plan.ops.len(), 2);
    assert!(matches!(plan.ops[0], DirectorOp::SpawnWave { .. }));
    assert!(matches!(plan.ops[1], DirectorOp::Collapse { .. }));
}

#[test]
fn boss_collapse_midpoint_exact() {
    let d = BossDirector;
    // dist = |0-4| + |0-2| = 6 <= 8, collapse branch
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 4, y: 2 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    if let DirectorOp::Collapse { a, b } = &plan.ops[0] {
        assert_eq!(*a, IVec2 { x: 0, y: 0 }); // player pos
        assert_eq!(b.x, (0 + 4) / 2); // 2
        assert_eq!(b.y, (0 + 2) / 2); // 1
    } else {
        panic!("Expected Collapse");
    }
}

// ============================= BossDirector: no enemies =============================

#[test]
fn boss_no_enemies_uses_default_target() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 5, y: 10 }, vec![]);
    // Default target: (player.x + 6, player.y) = (11, 10)
    // dist = |5-11| + |10-10| = 6 <= 8, so spawn/collapse branch
    let plan = d.plan(&snap, &budget(1, 0, 0));
    assert_eq!(plan.ops.len(), 1);
    assert!(matches!(plan.ops[0], DirectorOp::SpawnWave { .. }));
}

// ============================= BossDirector: determinism =============================

#[test]
fn boss_plan_deterministic_identical_inputs() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 50)]);
    let b = budget(3, 3, 3);
    let p1 = d.plan(&snap, &b);
    let p2 = d.plan(&snap, &b);
    assert_eq!(p1.ops.len(), p2.ops.len());
}

// ============================= BossDirector: negative positions =============================

#[test]
fn boss_negative_positions_work() {
    let d = BossDirector;
    let snap = make_snapshot(
        IVec2 { x: -10, y: -5 },
        vec![(IVec2 { x: -20, y: -15 }, 50)],
    );
    // dist = |-10-(-20)| + |-5-(-15)| = 10+10 = 20 > 8, fortify
    let plan = d.plan(&snap, &budget(0, 1, 0));
    assert!(matches!(plan.ops[0], DirectorOp::Fortify { .. }));
}

// ============================= PhaseDirector: construction =============================

#[test]
fn phase_director_initial_state_exact() {
    let phases = vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ];
    let d = PhaseDirector::new(phases);
    assert_eq!(d.phases.len(), 2);
    assert_eq!(d.state.idx, 0);
    assert_ne!(d.state.idx, 1);
    assert!(d.state.telegraph.is_none());
    assert!((d.state.last_switch_t - 0.0).abs() < f32::EPSILON);
}

// ============================= PhaseDirector: phase switching by HP =============================

#[test]
fn phase_stays_at_idx_0_when_hp_above_threshold() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 80)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(plan.phase_name, "P1");
    assert_eq!(d.state.idx, 0);
}

#[test]
fn phase_switches_to_2_when_hp_50() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 50)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(plan.phase_name, "P2");
    assert_eq!(d.state.idx, 1);
    assert_ne!(d.state.idx, 0);
}

#[test]
fn phase_skips_to_3_when_hp_very_low() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
        PhaseSpec {
            name: "P3".into(),
            hp_threshold: 20,
            terrain_bias: 0.9,
            aggression: 1.0,
        },
    ]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 10)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(plan.phase_name, "P3");
    assert_eq!(d.state.idx, 2);
}

#[test]
fn phase_switch_generates_telegraph_messages() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "Calm".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "Rage".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 40)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert!(
        plan.telegraphs
            .iter()
            .any(|t| t.contains("Boss shifts into phase: Rage")),
        "telegraphs: {:?}",
        plan.telegraphs
    );
}

// ============================= PhaseDirector: terrain bias =============================

#[test]
fn phase_high_terrain_bias_prefers_fortify() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "Fort".into(),
        hp_threshold: 100,
        terrain_bias: 0.8, // > 0.5
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 100)]);
    let plan = d.step(&snap, &budget(0, 1, 0));
    assert!(plan
        .director
        .ops
        .iter()
        .any(|op| matches!(op, DirectorOp::Fortify { .. })));
    assert!(plan.telegraphs.iter().any(|t| t.contains("ramparts")));
}

#[test]
fn phase_low_terrain_bias_prefers_spawn() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "Spawn".into(),
        hp_threshold: 100,
        terrain_bias: 0.3, // <= 0.5
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 100)]);
    let plan = d.step(&snap, &budget(1, 0, 0));
    assert!(plan
        .director
        .ops
        .iter()
        .any(|op| matches!(op, DirectorOp::SpawnWave { .. })));
}

#[test]
fn phase_terrain_bias_exactly_05_uses_spawn_path() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "Balanced".into(),
        hp_threshold: 100,
        terrain_bias: 0.5, // == 0.5, NOT > 0.5
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 100)]);
    let plan = d.step(&snap, &budget(1, 1, 0));
    // terrain_bias NOT > 0.5, so spawn path
    assert!(plan
        .director
        .ops
        .iter()
        .any(|op| matches!(op, DirectorOp::SpawnWave { .. })));
}

// ============================= PhaseDirector: spawn wave details =============================

#[test]
fn phase_spawn_wave_archetype_is_phase_add() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "S".into(),
        hp_threshold: 100,
        terrain_bias: 0.3,
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 100)]);
    let plan = d.step(&snap, &budget(1, 0, 0));
    if let DirectorOp::SpawnWave {
        archetype, count, ..
    } = &plan.director.ops[0]
    {
        assert_eq!(archetype, "phase_add");
        assert_ne!(archetype, "minion"); // Different from BossDirector
        assert_eq!(*count, 4);
        assert_ne!(*count, 3); // BossDirector uses 3
    } else {
        panic!("Expected SpawnWave");
    }
}

// ============================= PhaseDirector: empty budget =============================

#[test]
fn phase_empty_budget_no_ops() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "X".into(),
        hp_threshold: 100,
        terrain_bias: 0.3,
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 100)]);
    let plan = d.step(&snap, &budget(0, 0, 0));
    assert!(plan.director.ops.is_empty());
}

// ============================= PhaseDirector: no enemies default =============================

#[test]
fn phase_no_enemies_doesnt_switch_phase() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(plan.phase_name, "P1");
    assert_eq!(d.state.idx, 0);
}

// ============================= PhaseDirector: state persistence =============================

#[test]
fn phase_doesnt_regress_after_switch() {
    let mut d = PhaseDirector::new(vec![
        PhaseSpec {
            name: "P1".into(),
            hp_threshold: 100,
            terrain_bias: 0.3,
            aggression: 0.5,
        },
        PhaseSpec {
            name: "P2".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.8,
        },
    ]);
    // Switch to P2
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 40)]);
    d.step(&snap, &budget(5, 5, 5));
    assert_eq!(d.state.idx, 1);
    // HP recovers — should NOT go back
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 80)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(d.state.idx, 1); // stays at 1
    assert_eq!(plan.phase_name, "P2");
}

// ============================= PlayerBehaviorModel: defaults =============================

#[test]
fn player_model_default_all_neutral() {
    let m = PlayerBehaviorModel::default();
    assert!((m.aggression - 0.5).abs() < f32::EPSILON);
    assert!((m.caution - 0.5).abs() < f32::EPSILON);
    assert!((m.skill_level - 0.5).abs() < f32::EPSILON);
    assert!((m.preferred_range - 0.5).abs() < f32::EPSILON);
    assert!((m.adaptability - 0.5).abs() < f32::EPSILON);
    assert!(m.session_performance.is_empty());
    assert!(m.preferred_tactics.is_empty());
    assert!(m.weaknesses.is_empty());
    assert_eq!(m.encounter_count, 0);
}

// ============================= PlayerBehaviorModel: analyze_snapshot =============================

#[test]
fn player_model_analyze_far_enemies_increases_range() {
    let mut m = PlayerBehaviorModel::default();
    let range_before = m.preferred_range;
    // enemies far away (>10)
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 20, y: 0 }, 50)]);
    m.analyze_snapshot(&snap);
    assert!(
        m.preferred_range > range_before,
        "range should increase for far enemies"
    );
}

#[test]
fn player_model_analyze_close_enemies_decreases_range() {
    let mut m = PlayerBehaviorModel::default();
    let range_before = m.preferred_range;
    // enemies very close (<4)
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 1, y: 1 }, 50)]);
    m.analyze_snapshot(&snap);
    assert!(
        m.preferred_range < range_before,
        "range should decrease for close enemies"
    );
}

#[test]
fn player_model_analyze_returns_format_string() {
    let mut m = PlayerBehaviorModel::default();
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 50)]);
    let analysis = m.analyze_snapshot(&snap);
    assert!(
        analysis.contains("Player behavior analysis"),
        "analysis: {}",
        analysis
    );
    assert!(analysis.contains("Aggression"), "analysis: {}", analysis);
    assert!(analysis.contains("Caution"), "analysis: {}", analysis);
    assert!(analysis.contains("Skill"), "analysis: {}", analysis);
}

#[test]
fn player_model_analyze_no_enemies_default_distance() {
    let mut m = PlayerBehaviorModel::default();
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![]);
    let analysis = m.analyze_snapshot(&snap);
    // With no enemies, avg_distance defaults to 8.0 which is medium range
    assert!(
        analysis.contains("average distance 8.0"),
        "analysis: {}",
        analysis
    );
}

// ============================= PlayerBehaviorModel: update_from_outcome =============================

#[test]
fn player_model_update_increments_encounter_count() {
    let mut m = PlayerBehaviorModel::default();
    assert_eq!(m.encounter_count, 0);
    m.update_from_outcome(&make_outcome("tactic_a", 0.5));
    assert_eq!(m.encounter_count, 1);
    m.update_from_outcome(&make_outcome("tactic_b", 0.5));
    assert_eq!(m.encounter_count, 2);
}

#[test]
fn player_model_update_records_performance() {
    let mut m = PlayerBehaviorModel::default();
    m.update_from_outcome(&make_outcome("t", 0.8));
    assert_eq!(m.session_performance.len(), 1);
    assert!((m.session_performance[0] - 0.8).abs() < f32::EPSILON);
}

#[test]
fn player_model_update_high_performance_increases_skill() {
    let mut m = PlayerBehaviorModel::default();
    let initial_skill = m.skill_level;
    // Push many high-performance outcomes
    for _ in 0..5 {
        m.update_from_outcome(&make_outcome("t", 0.9));
    }
    assert!(
        m.skill_level > initial_skill,
        "skill should increase with high performance"
    );
}

#[test]
fn player_model_update_low_performance_decreases_skill() {
    let mut m = PlayerBehaviorModel::default();
    let initial_skill = m.skill_level;
    for _ in 0..5 {
        m.update_from_outcome(&make_outcome("t", 0.1));
    }
    assert!(
        m.skill_level < initial_skill,
        "skill should decrease with low performance"
    );
}

#[test]
fn player_model_update_effective_tactic_added_to_preferred() {
    let mut m = PlayerBehaviorModel::default();
    m.update_from_outcome(&make_outcome("good_tactic", 0.8));
    assert!(m.preferred_tactics.contains(&"good_tactic".to_string()));
}

#[test]
fn player_model_update_ineffective_counter_added_to_weaknesses() {
    let mut m = PlayerBehaviorModel::default();
    let mut outcome = make_outcome("bad_tactic", 0.1);
    outcome.counter_strategy = "player_weakness".to_string();
    m.update_from_outcome(&outcome);
    assert!(m.weaknesses.contains(&"player_weakness".to_string()));
}

#[test]
fn player_model_session_performance_capped_at_20() {
    let mut m = PlayerBehaviorModel::default();
    for i in 0..25 {
        m.update_from_outcome(&make_outcome(&format!("t{}", i), 0.5));
    }
    assert_eq!(m.session_performance.len(), 20);
}

#[test]
fn player_model_preferred_tactics_capped_at_5() {
    let mut m = PlayerBehaviorModel::default();
    for i in 0..8 {
        m.update_from_outcome(&make_outcome(&format!("tactic_{}", i), 0.9));
    }
    assert!(m.preferred_tactics.len() <= 5);
}

#[test]
fn player_model_weaknesses_capped_at_3() {
    let mut m = PlayerBehaviorModel::default();
    for i in 0..6 {
        let mut outcome = make_outcome("t", 0.1);
        outcome.counter_strategy = format!("weakness_{}", i);
        m.update_from_outcome(&outcome);
    }
    assert!(m.weaknesses.len() <= 3);
}

#[test]
fn player_model_adaptability_based_on_tactic_variety() {
    let mut m = PlayerBehaviorModel::default();
    // Add 5 unique tactics (all effective)
    for i in 0..5 {
        m.update_from_outcome(&make_outcome(&format!("unique_{}", i), 0.9));
    }
    // adaptability = unique_count / 5.0 = 5/5 = 1.0
    assert!((m.adaptability - 1.0).abs() < f32::EPSILON);
}

// ============================= CDirectorState =============================

#[test]
fn director_state_default_values() {
    let s = CDirectorState::default();
    assert!(s.current_plan.is_none());
    assert!(s.recent_outcomes.is_empty());
    assert!((s.difficulty_modifier - 1.0).abs() < f32::EPSILON);
    assert_eq!(s.last_adaptation_time, 0);
}

#[test]
fn director_state_should_adapt_initially_true() {
    let s = CDirectorState::default();
    assert!(s.should_adapt(1000, 500));
    assert!(s.should_adapt(500, 500));
    assert!(s.should_adapt(0, 0));
}

#[test]
fn director_state_should_adapt_boundary() {
    let s = CDirectorState {
        last_adaptation_time: 1000,
        ..Default::default()
    };
    // current_time - last = 500 - 1000 would underflow for u64... let's use proper values
    assert!(!s.should_adapt(1200, 500)); // 200 < 500 -> false
    assert!(!s.should_adapt(1499, 500)); // 499 < 500 -> false
    assert!(s.should_adapt(1500, 500)); // 500 >= 500 -> true
    assert!(s.should_adapt(1501, 500)); // 501 >= 500 -> true
    assert!(s.should_adapt(2000, 500)); // 1000 >= 500 -> true
}

#[test]
fn director_state_update_plan_sets_fields() {
    let mut s = CDirectorState::default();
    let plan = make_tactic_plan(vec![]);
    s.update_plan(plan.clone(), 5000);
    assert!(s.current_plan.is_some());
    assert_eq!(s.current_plan.as_ref().unwrap().strategy, "test_strategy");
    assert_eq!(s.last_adaptation_time, 5000);
    assert_ne!(s.last_adaptation_time, 0);
}

#[test]
fn director_state_record_outcome_adds_to_recent() {
    let mut s = CDirectorState::default();
    s.record_outcome(make_outcome("t", 0.5));
    assert_eq!(s.recent_outcomes.len(), 1);
    s.record_outcome(make_outcome("t", 0.7));
    assert_eq!(s.recent_outcomes.len(), 2);
}

#[test]
fn director_state_record_outcome_capped_at_10() {
    let mut s = CDirectorState::default();
    for i in 0..15 {
        s.record_outcome(make_outcome(&format!("t{}", i), 0.5));
    }
    assert_eq!(s.recent_outcomes.len(), 10);
    assert_ne!(s.recent_outcomes.len(), 15);
}

#[test]
fn director_state_get_recent_effectiveness_empty_returns_05() {
    let s = CDirectorState::default();
    assert!((s.get_recent_effectiveness() - 0.5).abs() < f32::EPSILON);
}

#[test]
fn director_state_get_recent_effectiveness_average() {
    let mut s = CDirectorState::default();
    s.record_outcome(make_outcome("t", 0.8));
    s.record_outcome(make_outcome("t", 0.6));
    s.record_outcome(make_outcome("t", 1.0));
    // avg = (0.8 + 0.6 + 1.0) / 3 = 0.8
    assert!((s.get_recent_effectiveness() - 0.8).abs() < 0.01);
}

#[test]
fn director_state_get_recent_effectiveness_single() {
    let mut s = CDirectorState::default();
    s.record_outcome(make_outcome("t", 0.3));
    assert!((s.get_recent_effectiveness() - 0.3).abs() < f32::EPSILON);
}

#[test]
fn director_state_reset_learning_clears_all() {
    let mut s = CDirectorState::default();
    s.update_plan(make_tactic_plan(vec![]), 1000);
    s.record_outcome(make_outcome("t", 0.5));
    s.difficulty_modifier = 2.0;
    s.reset_learning();

    assert!(s.current_plan.is_none());
    assert!(s.recent_outcomes.is_empty());
    assert!((s.difficulty_modifier - 1.0).abs() < f32::EPSILON);
    assert_eq!(s.last_adaptation_time, 0);
    assert_eq!(s.player_model.encounter_count, 0);
}

// ============================= CTacticExecution =============================

#[test]
fn tactic_execution_new_initial_state() {
    let plan = make_tactic_plan(vec![DirectorOp::SpawnWave {
        archetype: "minion".into(),
        count: 3,
        origin: IVec2 { x: 0, y: 0 },
    }]);
    let exec = CTacticExecution::new(plan, 1000);
    assert_eq!(exec.start_time, 1000);
    assert_eq!(exec.current_operation, 0);
    assert!(!exec.is_paused);
    assert!(!exec.is_complete());
    assert!(exec.metadata.is_empty());
}

#[test]
fn tactic_execution_is_complete_false_when_ops_remain() {
    let plan = make_tactic_plan(vec![
        DirectorOp::SpawnWave {
            archetype: "a".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
        DirectorOp::SpawnWave {
            archetype: "b".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
    ]);
    let exec = CTacticExecution::new(plan, 0);
    assert!(!exec.is_complete());
}

#[test]
fn tactic_execution_is_complete_true_when_empty_plan() {
    let plan = make_tactic_plan(vec![]);
    let exec = CTacticExecution::new(plan, 0);
    assert!(exec.is_complete()); // 0 >= 0
}

#[test]
fn tactic_execution_advance_through_all() {
    let plan = make_tactic_plan(vec![
        DirectorOp::SpawnWave {
            archetype: "a".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
        DirectorOp::SpawnWave {
            archetype: "b".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
    ]);
    let mut exec = CTacticExecution::new(plan, 0);
    assert!(!exec.is_complete());
    assert!(exec.advance_operation()); // 0 -> 1
    assert!(!exec.is_complete());
    assert!(exec.advance_operation()); // 1 -> 2
    assert!(exec.is_complete());
    assert!(!exec.advance_operation()); // already complete
}

#[test]
fn tactic_execution_get_current_operation_returns_correct() {
    let plan = make_tactic_plan(vec![
        DirectorOp::SpawnWave {
            archetype: "first".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
        DirectorOp::SpawnWave {
            archetype: "second".into(),
            count: 2,
            origin: IVec2 { x: 1, y: 1 },
        },
    ]);
    let mut exec = CTacticExecution::new(plan, 0);
    if let Some(DirectorOp::SpawnWave { archetype, .. }) = exec.get_current_operation() {
        assert_eq!(archetype, "first");
    } else {
        panic!("Expected first op");
    }
    exec.advance_operation();
    if let Some(DirectorOp::SpawnWave { archetype, .. }) = exec.get_current_operation() {
        assert_eq!(archetype, "second");
    } else {
        panic!("Expected second op");
    }
    exec.advance_operation();
    assert!(exec.get_current_operation().is_none());
}

#[test]
fn tactic_execution_paused_returns_none_for_current_op() {
    let plan = make_tactic_plan(vec![DirectorOp::SpawnWave {
        archetype: "a".into(),
        count: 1,
        origin: IVec2 { x: 0, y: 0 },
    }]);
    let mut exec = CTacticExecution::new(plan, 0);
    assert!(exec.get_current_operation().is_some());
    exec.pause();
    assert!(exec.is_paused);
    assert!(exec.get_current_operation().is_none());
    exec.resume();
    assert!(!exec.is_paused);
    assert!(exec.get_current_operation().is_some());
}

#[test]
fn tactic_execution_get_duration_exact() {
    let exec = CTacticExecution::new(make_tactic_plan(vec![]), 1000);
    assert_eq!(exec.get_duration(1000), 0);
    assert_eq!(exec.get_duration(1500), 500);
    assert_eq!(exec.get_duration(2000), 1000);
    assert_ne!(exec.get_duration(1500), 1500);
}

#[test]
fn tactic_execution_add_metadata() {
    let mut exec = CTacticExecution::new(make_tactic_plan(vec![]), 0);
    exec.add_metadata("key1".to_string(), "value1".to_string());
    exec.add_metadata("key2".to_string(), "value2".to_string());
    assert_eq!(exec.metadata.get("key1").unwrap(), "value1");
    assert_eq!(exec.metadata.get("key2").unwrap(), "value2");
    assert!(!exec.metadata.contains_key("key3"));
}

#[test]
fn tactic_execution_metadata_overwrite() {
    let mut exec = CTacticExecution::new(make_tactic_plan(vec![]), 0);
    exec.add_metadata("k".to_string(), "old".to_string());
    exec.add_metadata("k".to_string(), "new".to_string());
    assert_eq!(exec.metadata.get("k").unwrap(), "new");
    assert_eq!(exec.metadata.len(), 1);
}

// ============================= CDirectorMetrics =============================

#[test]
fn metrics_default_all_zero() {
    let m = CDirectorMetrics::default();
    assert_eq!(m.tactics_executed, 0);
    assert_eq!(m.successful_tactics, 0);
    assert!((m.average_effectiveness - 0.0).abs() < f32::EPSILON);
    assert_eq!(m.total_adaptation_time, 0);
    assert_eq!(m.difficulty_adjustments, 0);
    assert_eq!(m.llm_calls, 0);
    assert_eq!(m.llm_failures, 0);
    assert!((m.average_response_time - 0.0).abs() < f32::EPSILON);
}

#[test]
fn metrics_record_tactic_increments_count() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.8), 100);
    assert_eq!(m.tactics_executed, 1);
    m.record_tactic(&make_outcome("t", 0.5), 100);
    assert_eq!(m.tactics_executed, 2);
}

#[test]
fn metrics_record_tactic_success_threshold_06() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.6), 100); // NOT > 0.6
    assert_eq!(m.successful_tactics, 0); // 0.6 is NOT successful
    m.record_tactic(&make_outcome("t", 0.61), 100); // > 0.6
    assert_eq!(m.successful_tactics, 1);
    m.record_tactic(&make_outcome("t", 0.59), 100); // < 0.6
    assert_eq!(m.successful_tactics, 1);
}

#[test]
fn metrics_average_effectiveness_rolling() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.8), 100);
    assert!((m.average_effectiveness - 0.8).abs() < 0.01);
    m.record_tactic(&make_outcome("t", 0.4), 100);
    assert!((m.average_effectiveness - 0.6).abs() < 0.01); // (0.8 + 0.4) / 2
    m.record_tactic(&make_outcome("t", 0.6), 100);
    assert!((m.average_effectiveness - 0.6).abs() < 0.01); // (0.8 + 0.4 + 0.6) / 3
}

#[test]
fn metrics_record_llm_call_success() {
    let mut m = CDirectorMetrics::default();
    m.record_llm_call(100, true);
    assert_eq!(m.llm_calls, 1);
    assert_eq!(m.llm_failures, 0);
    assert!((m.average_response_time - 100.0).abs() < 0.01);
}

#[test]
fn metrics_record_llm_call_failure() {
    let mut m = CDirectorMetrics::default();
    m.record_llm_call(200, false);
    assert_eq!(m.llm_calls, 1);
    assert_eq!(m.llm_failures, 1);
}

#[test]
fn metrics_average_response_time_rolling() {
    let mut m = CDirectorMetrics::default();
    m.record_llm_call(100, true);
    assert!((m.average_response_time - 100.0).abs() < 0.01);
    m.record_llm_call(200, true);
    assert!((m.average_response_time - 150.0).abs() < 0.01);
    m.record_llm_call(300, true);
    assert!((m.average_response_time - 200.0).abs() < 0.01);
}

#[test]
fn metrics_get_success_rate_empty_returns_0() {
    assert!((CDirectorMetrics::default().get_success_rate() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn metrics_get_success_rate_exact() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.9), 100); // successful
    m.record_tactic(&make_outcome("t", 0.9), 100); // successful
    m.record_tactic(&make_outcome("t", 0.3), 100); // not successful
                                                   // 2/3 = 0.666...
    assert!((m.get_success_rate() - 2.0 / 3.0).abs() < 0.01);
}

#[test]
fn metrics_get_llm_failure_rate_empty_returns_0() {
    assert!((CDirectorMetrics::default().get_llm_failure_rate() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn metrics_get_llm_failure_rate_exact() {
    let mut m = CDirectorMetrics::default();
    m.record_llm_call(100, true);
    m.record_llm_call(100, false);
    m.record_llm_call(100, true);
    // 1/3
    assert!((m.get_llm_failure_rate() - 1.0 / 3.0).abs() < 0.01);
}

#[test]
fn metrics_record_difficulty_adjustment() {
    let mut m = CDirectorMetrics::default();
    m.record_difficulty_adjustment(50);
    m.record_difficulty_adjustment(30);
    assert_eq!(m.difficulty_adjustments, 2);
    assert_eq!(m.total_adaptation_time, 80);
}

#[test]
fn metrics_get_average_adaptation_time_exact() {
    let mut m = CDirectorMetrics::default();
    assert!((m.get_average_adaptation_time() - 0.0).abs() < f32::EPSILON);
    m.record_difficulty_adjustment(100);
    m.record_difficulty_adjustment(200);
    // 300 / 2 = 150.0
    assert!((m.get_average_adaptation_time() - 150.0).abs() < 0.01);
}

#[test]
fn metrics_record_skill_progression() {
    let mut m = CDirectorMetrics::default();
    m.record_skill_progression(1000, 0.5);
    m.record_skill_progression(2000, 0.6);
    assert_eq!(m.skill_progression.len(), 2);
    assert_eq!(m.skill_progression[0], (1000, 0.5));
    assert_eq!(m.skill_progression[1], (2000, 0.6));
}

#[test]
fn metrics_skill_progression_capped_at_100() {
    let mut m = CDirectorMetrics::default();
    for i in 0..110 {
        m.record_skill_progression(i as u64, i as f32 * 0.01);
    }
    assert_eq!(m.skill_progression.len(), 100);
}

#[test]
fn metrics_reset_clears_all() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.9), 100);
    m.record_llm_call(200, false);
    m.record_difficulty_adjustment(50);
    m.record_skill_progression(1000, 0.5);
    m.reset();
    assert_eq!(m.tactics_executed, 0);
    assert_eq!(m.successful_tactics, 0);
    assert_eq!(m.llm_calls, 0);
    assert_eq!(m.llm_failures, 0);
    assert_eq!(m.difficulty_adjustments, 0);
    assert_eq!(m.total_adaptation_time, 0);
    assert!(m.skill_progression.is_empty());
}

// ============================= LlmDirectorConfig =============================

#[test]
fn llm_config_default_exact_values() {
    let c = LlmDirectorConfig::default();
    assert!((c.adaptation_rate - 0.1).abs() < f32::EPSILON);
    assert!((c.min_difficulty - 0.3).abs() < f32::EPSILON);
    assert!((c.max_difficulty - 1.5).abs() < f32::EPSILON);
    assert!(c.learning_enabled);
    assert!((c.creativity_factor - 0.7).abs() < f32::EPSILON);
    assert_eq!(c.context_window_size, 2048);
}

// ============================= TacticPlan/TacticOutcome Serde =============================

#[test]
fn tactic_plan_serde_roundtrip() {
    let plan = TacticPlan {
        strategy: "ambush".to_string(),
        reasoning: "player prefers ranged".to_string(),
        operations: vec![DirectorOp::SpawnWave {
            archetype: "flanker".into(),
            count: 2,
            origin: IVec2 { x: 5, y: 5 },
        }],
        difficulty_modifier: 0.9,
        expected_duration: 45,
        counter_strategies: vec!["close_combat".into()],
        fallback_plan: Some("retreat".into()),
    };
    let json = serde_json::to_string(&plan).unwrap();
    let restored: TacticPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.strategy, "ambush");
    assert_eq!(restored.reasoning, "player prefers ranged");
    assert_eq!(restored.operations.len(), 1);
    assert!((restored.difficulty_modifier - 0.9).abs() < f32::EPSILON);
    assert_eq!(restored.expected_duration, 45);
    assert_eq!(restored.counter_strategies, vec!["close_combat"]);
    assert_eq!(restored.fallback_plan, Some("retreat".into()));
}

#[test]
fn tactic_outcome_serde_roundtrip() {
    let outcome = TacticOutcome {
        tactic_used: "pincer".to_string(),
        effectiveness: 0.75,
        player_response: "retreated".to_string(),
        counter_strategy: "flank".to_string(),
        duration_actual: 20,
        timestamp: 12345,
    };
    let json = serde_json::to_string(&outcome).unwrap();
    let restored: TacticOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.tactic_used, "pincer");
    assert!((restored.effectiveness - 0.75).abs() < f32::EPSILON);
    assert_eq!(restored.player_response, "retreated");
    assert_eq!(restored.counter_strategy, "flank");
    assert_eq!(restored.duration_actual, 20);
    assert_eq!(restored.timestamp, 12345);
}

// ============================= Integration helpers =============================

#[test]
fn integration_process_director_operations_returns_current() {
    let plan = make_tactic_plan(vec![
        DirectorOp::SpawnWave {
            archetype: "a".into(),
            count: 1,
            origin: IVec2 { x: 0, y: 0 },
        },
        DirectorOp::SpawnWave {
            archetype: "b".into(),
            count: 2,
            origin: IVec2 { x: 1, y: 1 },
        },
    ]);
    let exec = CTacticExecution::new(plan, 0);
    let ops = astraweave_director::integration::process_director_operations(&exec, 100);
    assert_eq!(ops.len(), 1); // Only current op
}

#[test]
fn integration_process_director_operations_empty_when_complete() {
    let plan = make_tactic_plan(vec![]);
    let exec = CTacticExecution::new(plan, 0);
    let ops = astraweave_director::integration::process_director_operations(&exec, 100);
    assert!(ops.is_empty());
}

#[test]
fn integration_should_provide_feedback_timing() {
    let plan = make_tactic_plan(vec![DirectorOp::SpawnWave {
        archetype: "a".into(),
        count: 1,
        origin: IVec2 { x: 0, y: 0 },
    }]);
    let exec = CTacticExecution::new(plan, 0);

    // Not enough time elapsed
    assert!(!astraweave_director::integration::should_provide_feedback(
        &exec, 0, 400, 500
    ));
    // Exact boundary
    assert!(astraweave_director::integration::should_provide_feedback(
        &exec, 0, 500, 500
    ));
    // Past boundary
    assert!(astraweave_director::integration::should_provide_feedback(
        &exec, 0, 600, 500
    ));
}

#[test]
fn integration_should_provide_feedback_false_when_complete() {
    let plan = make_tactic_plan(vec![]);
    let exec = CTacticExecution::new(plan, 0);
    // Execution is complete (no ops)
    assert!(!astraweave_director::integration::should_provide_feedback(
        &exec, 0, 1000, 500
    ));
}

// ============================================================================
// MUTATION KILL TESTS — targeting analyze_snapshot arithmetic,
// threshold boundaries, and aggression/caution updates
// ============================================================================

/// Kill: analyze_snapshot aggression/caution formulas (lines 78-82).
/// 3 enemies within dist 6 and avg_distance < 5 → aggression += 0.05,
/// caution -= 0.03.
/// Kills: +→-, +→*, -→+, -→/ on aggression/caution increments (4),
///        >→== and >→< on enemies_nearby comparison (2),
///        <→== and <→> on avg_distance comparison (2).
#[test]
fn analyze_snapshot_aggression_increases_with_close_pack() {
    let mut m = PlayerBehaviorModel::default();
    // 3 enemies all within dist 6 of player at (5,3), avg_distance < 5
    let snap = make_snapshot(
        IVec2 { x: 5, y: 3 },
        vec![
            (IVec2 { x: 7, y: 3 }, 50), // dist 2
            (IVec2 { x: 4, y: 4 }, 50), // dist 2
            (IVec2 { x: 6, y: 2 }, 50), // dist 2
        ],
    );
    m.analyze_snapshot(&snap);

    // aggression should increase from 0.5 to 0.55
    assert!(
        (m.aggression - 0.55).abs() < 0.01,
        "aggression should be ~0.55, got {}",
        m.aggression
    );
    // caution should decrease from 0.5 to 0.47
    assert!(
        (m.caution - 0.47).abs() < 0.01,
        "caution should be ~0.47, got {}",
        m.caution
    );
    // preferred_range should decrease (avg_dist 2.0 < 4.0)
    assert!(
        (m.preferred_range - 0.4).abs() < 0.01,
        "preferred_range should be ~0.4, got {}",
        m.preferred_range
    );
}

/// Kill: boundary mutation > → >= on avg_distance > 10.0 threshold (line 62).
/// avg_distance = exactly 10.0 should NOT trigger range increase.
#[test]
fn analyze_snapshot_boundary_avg_distance_10_no_range_change() {
    let mut m = PlayerBehaviorModel::default();
    // 1 enemy at Manhattan distance exactly 10 from player
    let snap = make_snapshot(
        IVec2 { x: 0, y: 0 },
        vec![(IVec2 { x: 7, y: 3 }, 50)], // dist = 7 + 3 = 10
    );
    m.analyze_snapshot(&snap);

    // avg_distance = 10.0, > 10.0 is false, preferred_range should stay 0.5
    assert!(
        (m.preferred_range - 0.5).abs() < 0.01,
        "preferred_range should stay 0.5 at boundary, got {}",
        m.preferred_range
    );
}

/// Kill: /→* mutation on avg_distance division (line 56).
/// With 2 enemies, division by len produces different result from multiplication.
/// Also catches -→+ on distance x-component (line 54:40) since player is at
/// non-origin position.
#[test]
fn analyze_snapshot_multiple_enemies_non_origin_avg_distance() {
    let mut m = PlayerBehaviorModel::default();
    // Player at (5,0), 2 enemies at (12,3) and (15,2)
    // Distances: |5-12|+|0-3|=10, |5-15|+|0-2|=12. Sum=22, avg=11.0
    // avg > 10.0 → preferred_range increases
    let snap = make_snapshot(
        IVec2 { x: 5, y: 0 },
        vec![(IVec2 { x: 12, y: 3 }, 50), (IVec2 { x: 15, y: 2 }, 50)],
    );
    m.analyze_snapshot(&snap);

    assert!(
        (m.preferred_range - 0.6).abs() < 0.01,
        "preferred_range should be ~0.6 (increased from 0.5), got {}",
        m.preferred_range
    );
}

/// Kill: >→>= on update_from_outcome skill threshold (line 117).
/// With avg_performance = exactly 0.7, > 0.7 is false → no skill increase.
#[test]
fn update_outcome_boundary_07_no_skill_increase() {
    let mut m = PlayerBehaviorModel::default();
    // effectiveness = 0.7 → avg = 0.7 → NOT > 0.7
    m.update_from_outcome(&make_outcome("t", 0.7));

    assert!(
        (m.skill_level - 0.5).abs() < 0.01,
        "skill should stay 0.5 at boundary 0.7, got {}",
        m.skill_level
    );
}

/// Kill: BossDirector::plan distance formula with player at non-origin.
/// At (2,0) with no enemies: fallback target = (8,0), dist = 6 ≤ 8 → spawn.
/// Mutation +→* on ppos.x+6: target = (12,0), dist = 10 > 8 → fortify.
#[test]
fn boss_plan_non_origin_distance_spawn_not_fortify() {
    let d = BossDirector;
    let snap = make_snapshot(IVec2 { x: 2, y: 0 }, vec![]);
    let plan = d.plan(&snap, &budget(5, 5, 5));
    // dist = 6, should spawn not fortify
    assert!(
        plan.ops
            .iter()
            .any(|op| matches!(op, DirectorOp::SpawnWave { .. })),
        "should spawn at dist=6, got {:?}",
        plan.ops
    );
}

// ============================= Clone/Serde verifications =============================

#[test]
fn director_state_clone_independent() {
    let mut s = CDirectorState::default();
    s.record_outcome(make_outcome("t", 0.5));
    let cloned = s.clone();
    s.record_outcome(make_outcome("t2", 0.9));
    assert_eq!(cloned.recent_outcomes.len(), 1);
    assert_eq!(s.recent_outcomes.len(), 2);
}

#[test]
fn tactic_execution_clone_independent() {
    let plan = make_tactic_plan(vec![DirectorOp::SpawnWave {
        archetype: "a".into(),
        count: 1,
        origin: IVec2 { x: 0, y: 0 },
    }]);
    let mut exec = CTacticExecution::new(plan, 0);
    let cloned = exec.clone();
    exec.advance_operation();
    assert!(exec.is_complete());
    assert!(!cloned.is_complete());
}

#[test]
fn director_state_serde_roundtrip() {
    let mut s = CDirectorState::default();
    s.record_outcome(make_outcome("flanking", 0.85));
    s.difficulty_modifier = 1.3;
    let json = serde_json::to_string(&s).unwrap();
    let restored: CDirectorState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.recent_outcomes.len(), 1);
    assert!((restored.difficulty_modifier - 1.3).abs() < f32::EPSILON);
}

#[test]
fn metrics_serde_roundtrip() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.8), 100);
    m.record_llm_call(150, true);
    let json = serde_json::to_string(&m).unwrap();
    let restored: CDirectorMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.tactics_executed, 1);
    assert_eq!(restored.llm_calls, 1);
}

// ============================= PhaseSpec/PhaseState field verification =============================

#[test]
fn phase_spec_clone_preserves_all() {
    let spec = PhaseSpec {
        name: "TestPhase".into(),
        hp_threshold: 42,
        terrain_bias: 0.77,
        aggression: 0.33,
    };
    let c = spec.clone();
    assert_eq!(c.name, "TestPhase");
    assert_eq!(c.hp_threshold, 42);
    assert!((c.terrain_bias - 0.77).abs() < f32::EPSILON);
    assert!((c.aggression - 0.33).abs() < f32::EPSILON);
}

#[test]
fn phase_state_clone_includes_telegraph() {
    let state = PhaseState {
        idx: 3,
        last_switch_t: 99.9,
        telegraph: Some("Watch out!".into()),
    };
    let c = state.clone();
    assert_eq!(c.idx, 3);
    assert!((c.last_switch_t - 99.9).abs() < 0.01);
    assert_eq!(c.telegraph, Some("Watch out!".to_string()));
}

// ============================= Edge cases =============================

#[test]
fn boss_manhattan_distance_both_axes() {
    let d = BossDirector;
    // dist = |0-4| + |0-5| = 9 > 8 → fortify
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 4, y: 5 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    assert!(matches!(plan.ops[0], DirectorOp::Fortify { .. }));

    // dist = |0-4| + |0-4| = 8 ≤ 8 → collapse
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 4, y: 4 }, 50)]);
    let plan = d.plan(&snap, &budget(0, 1, 0));
    assert!(matches!(plan.ops[0], DirectorOp::Collapse { .. }));
}

#[test]
fn phase_director_single_phase_never_switches() {
    let mut d = PhaseDirector::new(vec![PhaseSpec {
        name: "Only".into(),
        hp_threshold: 100,
        terrain_bias: 0.5,
        aggression: 0.5,
    }]);
    let snap = make_snapshot(IVec2 { x: 0, y: 0 }, vec![(IVec2 { x: 5, y: 5 }, 1)]);
    let plan = d.step(&snap, &budget(5, 5, 5));
    assert_eq!(plan.phase_name, "Only");
    assert_eq!(d.state.idx, 0);
}

#[test]
fn metrics_success_rate_all_failures() {
    let mut m = CDirectorMetrics::default();
    m.record_tactic(&make_outcome("t", 0.1), 100);
    m.record_tactic(&make_outcome("t", 0.2), 100);
    m.record_tactic(&make_outcome("t", 0.3), 100);
    assert!((m.get_success_rate() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn metrics_llm_all_failures() {
    let mut m = CDirectorMetrics::default();
    m.record_llm_call(100, false);
    m.record_llm_call(200, false);
    assert!((m.get_llm_failure_rate() - 1.0).abs() < f32::EPSILON);
}
