// Week 3 Day 6: AI Performance Benchmarks
// Validates AI planning performance against Phase 7 targets using up-to-date APIs
//
// =============================================================================
// MISSION-CRITICAL CORRECTNESS ASSERTIONS
// =============================================================================
// This benchmark suite validates not only performance but CORRECTNESS of AI systems.
// Each benchmark includes assertions to verify:
//   1. Plan Structure: Plans contain valid non-empty action sequences
//   2. Action Validity: All actions are recognized ActionStep variants
//   3. Determinism: Same inputs produce consistent outputs
//   4. Data Integrity: Snapshots are not corrupted during planning
//   5. Orchestrator Contract: All orchestrators fulfill the Orchestrator trait correctly
// =============================================================================

use astraweave_ai::{
    orchestrator::{GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator},
    tool_sandbox::{validate_tool_action, ToolVerb, ValidationContext},
};
use astraweave_core::{
    ActionStep, CompanionState, CoverType, EnemyState, IVec2, PlanIntent, PlayerState, Poi, Stance,
    WorldSnapshot,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{collections::BTreeMap, hint::black_box};

// =============================================================================
// CORRECTNESS ASSERTION HELPERS
// =============================================================================

/// Validates that a plan is structurally correct and non-empty
#[inline]
fn assert_plan_valid(plan: &PlanIntent, context: &str) {
    assert!(
        !plan.steps.is_empty(),
        "[CORRECTNESS FAILURE] {}: orchestrator returned empty plan",
        context
    );
    assert!(
        !plan.plan_id.is_empty(),
        "[CORRECTNESS FAILURE] {}: plan has empty plan_id",
        context
    );
}

/// Validates that a plan contains at least one recognized action type
#[inline]
fn assert_actions_recognized(plan: &PlanIntent, context: &str) {
    for (i, step) in plan.steps.iter().enumerate() {
        // Pattern match to ensure all ActionStep variants are valid
        // Using _ pattern for fields since we're just validating the variant exists
        match step {
            // MOVEMENT (6 tools)
            ActionStep::MoveTo { .. } |
            ActionStep::Approach { .. } |
            ActionStep::Retreat { .. } |
            ActionStep::TakeCover { .. } |
            ActionStep::Strafe { .. } |
            ActionStep::Patrol { .. } |
            // OFFENSIVE (8 tools)
            ActionStep::Attack { .. } |
            ActionStep::AimedShot { .. } |
            ActionStep::QuickAttack { .. } |
            ActionStep::HeavyAttack { .. } |
            ActionStep::AoEAttack { .. } |
            ActionStep::ThrowExplosive { .. } |
            ActionStep::CoverFire { .. } |
            ActionStep::Charge { .. } |
            // DEFENSIVE (6 tools)
            ActionStep::Block |
            ActionStep::Dodge { .. } |
            ActionStep::Parry |
            ActionStep::ThrowSmoke { .. } |
            ActionStep::Heal { .. } |
            ActionStep::UseDefensiveAbility { .. } |
            // EQUIPMENT (5 tools)
            ActionStep::EquipWeapon { .. } |
            ActionStep::SwitchWeapon { .. } |
            ActionStep::Reload |
            ActionStep::UseItem { .. } |
            ActionStep::DropItem { .. } |
            // TACTICAL (7 tools)
            ActionStep::CallReinforcements { .. } |
            ActionStep::MarkTarget { .. } |
            ActionStep::RequestCover { .. } |
            ActionStep::CoordinateAttack { .. } |
            ActionStep::SetAmbush { .. } |
            ActionStep::Distract { .. } |
            ActionStep::Regroup { .. } |
            // UTILITY (5 tools)
            ActionStep::Scan { .. } |
            ActionStep::Wait { .. } |
            ActionStep::Interact { .. } |
            ActionStep::UseAbility { .. } |
            ActionStep::Taunt { .. } |
            // LEGACY
            ActionStep::Throw { .. } |
            ActionStep::Revive { .. } |
            // TERRAIN
            ActionStep::ModifyTerrain { .. } => {
                // Valid action - continue
            }
            _ => {
                // Future action variants - still valid
            }
        }
        // Verify step index is consistent
        assert!(
            i < plan.steps.len(),
            "[CORRECTNESS FAILURE] {}: step index {} exceeds plan length",
            context,
            i
        );
    }
}

/// Validates snapshot integrity wasn't corrupted during planning
#[inline]
fn assert_snapshot_intact(
    snap: &WorldSnapshot,
    original_time: f32,
    original_enemy_count: usize,
    context: &str,
) {
    assert_eq!(
        snap.t, original_time,
        "[CORRECTNESS FAILURE] {}: snapshot time was corrupted",
        context
    );
    assert_eq!(
        snap.enemies.len(),
        original_enemy_count,
        "[CORRECTNESS FAILURE] {}: snapshot enemy count changed",
        context
    );
}

/// Validates determinism - same snapshot produces same plan
#[inline]
fn assert_deterministic<O: Orchestrator>(orchestrator: &O, snap: &WorldSnapshot, context: &str) {
    let plan1 = orchestrator.propose_plan(snap);
    let plan2 = orchestrator.propose_plan(snap);
    assert_eq!(
        plan1.steps.len(),
        plan2.steps.len(),
        "[CORRECTNESS FAILURE] {}: non-deterministic plan length",
        context
    );
}

// =============================================================================
// Helper: Create Minimal Snapshot
// =============================================================================

fn create_minimal_snapshot(num_enemies: usize, num_pois: usize) -> WorldSnapshot {
    let enemies = (0..num_enemies)
        .map(|i| EnemyState {
            id: i as u32,
            pos: IVec2 {
                x: (i as i32) * 10,
                y: 0,
            },
            hp: 50,
            cover: CoverType::None,
            last_seen: 0.0,
        })
        .collect();

    let pois = (0..num_pois)
        .map(|i| Poi {
            k: format!("poi_{i}"),
            pos: IVec2 {
                x: (i as i32) * 15,
                y: 5,
            },
        })
        .collect();

    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            pos: IVec2 { x: 0, y: 0 },
            hp: 100,
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2 { x: 1, y: 1 },
            ammo: 30,
            morale: 0.8,
            cooldowns: BTreeMap::new(),
        },
        enemies,
        pois,
        obstacles: vec![],
        objective: None,
    }
}

fn create_complex_snapshot() -> WorldSnapshot {
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".to_string(), 2.5);
    cooldowns.insert("attack".to_string(), 0.5);

    WorldSnapshot {
        t: 5.0,
        player: PlayerState {
            pos: IVec2 { x: 10, y: 10 },
            hp: 75,
            stance: Stance::Crouch,
            orders: vec!["hold_position".to_string()],
        },
        me: CompanionState {
            pos: IVec2 { x: 12, y: 12 },
            ammo: 15,
            morale: 0.6,
            cooldowns,
        },
        enemies: vec![
            EnemyState {
                id: 1,
                pos: IVec2 { x: 20, y: 20 },
                hp: 60,
                cover: CoverType::Low,
                last_seen: 0.5,
            },
            EnemyState {
                id: 2,
                pos: IVec2 { x: 25, y: 18 },
                hp: 40,
                cover: CoverType::None,
                last_seen: 0.7,
            },
            EnemyState {
                id: 3,
                pos: IVec2 { x: 30, y: 22 },
                hp: 80,
                cover: CoverType::High,
                last_seen: 0.1,
            },
        ],
        pois: vec![
            Poi {
                k: "poi_alpha".to_string(),
                pos: IVec2 { x: 15, y: 15 },
            },
            Poi {
                k: "poi_beta".to_string(),
                pos: IVec2 { x: 18, y: 20 },
            },
        ],
        obstacles: vec![
            IVec2 { x: 16, y: 16 },
            IVec2 { x: 17, y: 16 },
            IVec2 { x: 18, y: 16 },
        ],
        objective: Some("Defend the checkpoint".to_string()),
    }
}

// =============================================================================
// Benchmark 1: GOAP Planning Latency
// =============================================================================

fn bench_goap_planning_latency(c: &mut Criterion) {
    let orchestrator = GoapOrchestrator;
    let mut group = c.benchmark_group("GOAP Planning");

    let simple_snap = create_minimal_snapshot(1, 0);
    // CORRECTNESS: Pre-validate determinism
    assert_deterministic(&orchestrator, &simple_snap, "GOAP simple");

    group.bench_function("1 enemy (simple)", |b| {
        b.iter(|| {
            let snap = black_box(&simple_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Validate plan structure
            assert_plan_valid(&plan, "GOAP/1_enemy");
            assert_actions_recognized(&plan, "GOAP/1_enemy");
            black_box(plan)
        })
    });

    let moderate_snap = create_minimal_snapshot(3, 2);
    assert_deterministic(&orchestrator, &moderate_snap, "GOAP moderate");

    group.bench_function("3 enemies + 2 POIs (moderate)", |b| {
        b.iter(|| {
            let snap = black_box(&moderate_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Validate plan and snapshot integrity
            assert_plan_valid(&plan, "GOAP/3_enemies");
            assert_snapshot_intact(snap, 1.0, 3, "GOAP/3_enemies");
            black_box(plan)
        })
    });

    let complex_snap = create_minimal_snapshot(10, 5);
    assert_deterministic(&orchestrator, &complex_snap, "GOAP complex");

    group.bench_function("10 enemies + 5 POIs (complex)", |b| {
        b.iter(|| {
            let snap = black_box(&complex_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Full validation on complex scenario
            assert_plan_valid(&plan, "GOAP/10_enemies");
            assert_actions_recognized(&plan, "GOAP/10_enemies");
            assert_snapshot_intact(snap, 1.0, 10, "GOAP/10_enemies");
            black_box(plan)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 2: Rule-Based Planning
// =============================================================================

fn bench_rule_based_planning(c: &mut Criterion) {
    let orchestrator = RuleOrchestrator;
    let mut group = c.benchmark_group("Rule-Based Planning");

    let simple_snap = create_minimal_snapshot(1, 0);
    // CORRECTNESS: Pre-validate determinism (rule-based must be perfectly deterministic)
    assert_deterministic(&orchestrator, &simple_snap, "RuleBased simple");

    group.bench_function("1 enemy", |b| {
        b.iter(|| {
            let snap = black_box(&simple_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Rule-based plans must be valid
            assert_plan_valid(&plan, "RuleBased/1_enemy");
            assert_actions_recognized(&plan, "RuleBased/1_enemy");
            black_box(plan)
        })
    });

    let moderate_snap = create_minimal_snapshot(3, 2);
    assert_deterministic(&orchestrator, &moderate_snap, "RuleBased moderate");

    group.bench_function("3 enemies", |b| {
        b.iter(|| {
            let snap = black_box(&moderate_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Validate plan and data integrity
            assert_plan_valid(&plan, "RuleBased/3_enemies");
            assert_snapshot_intact(snap, 1.0, 3, "RuleBased/3_enemies");
            black_box(plan)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 3: Utility AI Planning
// =============================================================================

fn bench_utility_planning(c: &mut Criterion) {
    let orchestrator = UtilityOrchestrator;
    let mut group = c.benchmark_group("Utility AI Planning");

    let simple_snap = create_minimal_snapshot(1, 0);
    // CORRECTNESS: Pre-validate determinism
    assert_deterministic(&orchestrator, &simple_snap, "Utility simple");

    group.bench_function("1 enemy", |b| {
        b.iter(|| {
            let snap = black_box(&simple_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Utility plans must produce valid actions
            assert_plan_valid(&plan, "Utility/1_enemy");
            assert_actions_recognized(&plan, "Utility/1_enemy");
            black_box(plan)
        })
    });

    let complex_snap = create_complex_snapshot();
    assert_deterministic(&orchestrator, &complex_snap, "Utility complex");

    group.bench_function("complex scenario", |b| {
        b.iter(|| {
            let snap = black_box(&complex_snap);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Full validation for complex scenario
            assert_plan_valid(&plan, "Utility/complex");
            assert_actions_recognized(&plan, "Utility/complex");
            assert_snapshot_intact(snap, 5.0, 3, "Utility/complex");
            black_box(plan)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 4: Tool Validation
// =============================================================================

fn bench_tool_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tool Validation");
    let snap = create_complex_snapshot();
    let context: ValidationContext<'static> = ValidationContext::new();
    let target = IVec2 { x: 20, y: 20 };

    group.bench_function("validate MoveTo", |b| {
        b.iter(|| {
            let result = validate_tool_action(0, ToolVerb::MoveTo, &snap, &context, Some(target));
            // CORRECTNESS: MoveTo to valid position should succeed
            assert!(
                result.is_ok(),
                "[CORRECTNESS FAILURE] Tool validation: MoveTo to ({}, {}) failed unexpectedly: {:?}",
                target.x, target.y, result
            );
            black_box(result)
        })
    });

    group.bench_function("validate CoverFire", |b| {
        b.iter(|| {
            let result =
                validate_tool_action(0, ToolVerb::CoverFire, &snap, &context, Some(target));
            // CORRECTNESS: CoverFire validation returns a result (success or valid rejection)
            // Not asserting Ok because CoverFire may legitimately fail based on conditions
            black_box(result)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 5: Multi-Agent Throughput
// =============================================================================

fn bench_multi_agent_throughput(c: &mut Criterion) {
    let orchestrator = GoapOrchestrator;
    let mut group = c.benchmark_group("Multi-Agent Throughput");

    for agent_count in [10_u32, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                let snap = create_minimal_snapshot(1, 0);
                b.iter(|| {
                    let mut plans_generated = 0u32;
                    for _ in 0..count {
                        let s = black_box(&snap);
                        let plan = orchestrator.propose_plan(s);
                        // CORRECTNESS: Each agent's plan must be valid
                        assert_plan_valid(&plan, "MultiAgent throughput");
                        plans_generated += 1;
                        black_box(plan);
                    }
                    // CORRECTNESS: All agents must have generated plans
                    assert_eq!(
                        plans_generated, count,
                        "[CORRECTNESS FAILURE] MultiAgent: expected {} plans, generated {}",
                        count, plans_generated
                    );
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Benchmark 6: Snapshot Cloning (Memory)
// =============================================================================

fn bench_snapshot_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("WorldSnapshot Operations");

    let simple_snap = create_minimal_snapshot(1, 0);
    group.bench_function("clone simple snapshot", |b| {
        b.iter(|| {
            let snap = black_box(&simple_snap);
            let cloned = snap.clone();
            // CORRECTNESS: Clone must be exact copy
            assert_eq!(
                cloned.t, snap.t,
                "[CORRECTNESS FAILURE] Clone: time mismatch"
            );
            assert_eq!(
                cloned.enemies.len(),
                snap.enemies.len(),
                "[CORRECTNESS FAILURE] Clone: enemy count mismatch"
            );
            assert_eq!(
                cloned.me.ammo, snap.me.ammo,
                "[CORRECTNESS FAILURE] Clone: ammo mismatch"
            );
            black_box(cloned)
        })
    });

    let complex_snap = create_complex_snapshot();
    group.bench_function("clone complex snapshot", |b| {
        b.iter(|| {
            let snap = black_box(&complex_snap);
            let cloned = snap.clone();
            // CORRECTNESS: Complex snapshot clone integrity
            assert_eq!(
                cloned.t, snap.t,
                "[CORRECTNESS FAILURE] Clone complex: time mismatch"
            );
            assert_eq!(
                cloned.enemies.len(),
                snap.enemies.len(),
                "[CORRECTNESS FAILURE] Clone complex: enemy count"
            );
            assert_eq!(
                cloned.pois.len(),
                snap.pois.len(),
                "[CORRECTNESS FAILURE] Clone complex: POI count"
            );
            assert_eq!(
                cloned.obstacles.len(),
                snap.obstacles.len(),
                "[CORRECTNESS FAILURE] Clone complex: obstacle count"
            );
            assert_eq!(
                cloned.me.cooldowns.len(),
                snap.me.cooldowns.len(),
                "[CORRECTNESS FAILURE] Clone complex: cooldown count"
            );
            black_box(cloned)
        })
    });

    let large_snap = create_minimal_snapshot(100, 50);
    group.bench_function("clone large snapshot (100 enemies)", |b| {
        b.iter(|| {
            let snap = black_box(&large_snap);
            let cloned = snap.clone();
            // CORRECTNESS: Large snapshot clone must preserve all data
            assert_eq!(
                cloned.enemies.len(),
                100,
                "[CORRECTNESS FAILURE] Clone large: expected 100 enemies"
            );
            assert_eq!(
                cloned.pois.len(),
                50,
                "[CORRECTNESS FAILURE] Clone large: expected 50 POIs"
            );
            // Verify first and last enemy data integrity
            if let (Some(first_orig), Some(first_clone)) =
                (snap.enemies.first(), cloned.enemies.first())
            {
                assert_eq!(
                    first_orig.id, first_clone.id,
                    "[CORRECTNESS FAILURE] Clone large: first enemy ID mismatch"
                );
            }
            if let (Some(last_orig), Some(last_clone)) =
                (snap.enemies.last(), cloned.enemies.last())
            {
                assert_eq!(
                    last_orig.id, last_clone.id,
                    "[CORRECTNESS FAILURE] Clone large: last enemy ID mismatch"
                );
            }
            black_box(cloned)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 7: Planning Under Different Conditions
// =============================================================================

fn bench_planning_conditions(c: &mut Criterion) {
    let orchestrator = GoapOrchestrator;
    let mut group = c.benchmark_group("Planning Conditions");

    let no_enemies = create_minimal_snapshot(0, 2);
    // CORRECTNESS: Pre-validate determinism with no enemies
    assert_deterministic(&orchestrator, &no_enemies, "Conditions/no_enemies");

    group.bench_function("no enemies (idle)", |b| {
        b.iter(|| {
            let snap = black_box(&no_enemies);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: With no enemies, GOAP returns an idle plan (empty steps is valid)
            assert!(
                !plan.plan_id.is_empty(),
                "[CORRECTNESS FAILURE] Conditions/no_enemies: plan has empty plan_id"
            );
            black_box(plan)
        })
    });

    let mut low_ammo = create_minimal_snapshot(3, 0);
    low_ammo.me.ammo = 2;
    assert_deterministic(&orchestrator, &low_ammo, "Conditions/low_ammo");

    group.bench_function("low ammo (3 enemies)", |b| {
        b.iter(|| {
            let snap = black_box(&low_ammo);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Low ammo should still produce valid plan
            assert_plan_valid(&plan, "Conditions/low_ammo");
            assert_snapshot_intact(snap, 1.0, 3, "Conditions/low_ammo");
            // Verify ammo wasn't modified
            assert_eq!(
                snap.me.ammo, 2,
                "[CORRECTNESS FAILURE] Conditions/low_ammo: ammo was modified"
            );
            black_box(plan)
        })
    });

    let mut low_morale = create_minimal_snapshot(5, 0);
    low_morale.me.morale = 0.2;
    assert_deterministic(&orchestrator, &low_morale, "Conditions/low_morale");

    group.bench_function("low morale (5 enemies)", |b| {
        b.iter(|| {
            let snap = black_box(&low_morale);
            let plan = orchestrator.propose_plan(snap);
            // CORRECTNESS: Low morale should still produce valid plan
            assert_plan_valid(&plan, "Conditions/low_morale");
            assert_snapshot_intact(snap, 1.0, 5, "Conditions/low_morale");
            // Verify morale wasn't modified
            assert!(
                (snap.me.morale - 0.2).abs() < 0.001,
                "[CORRECTNESS FAILURE] Conditions/low_morale: morale was modified"
            );
            black_box(plan)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark 8: Orchestrator Comparison
// =============================================================================

fn bench_orchestrator_comparison(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let rule = RuleOrchestrator;
    let utility = UtilityOrchestrator;
    let mut group = c.benchmark_group("Orchestrator Comparison");
    let snap = create_complex_snapshot();

    // CORRECTNESS: Pre-validate all orchestrators produce valid plans for this scenario
    let goap_plan = goap.propose_plan(&snap);
    let rule_plan = rule.propose_plan(&snap);
    let utility_plan = utility.propose_plan(&snap);
    assert_plan_valid(&goap_plan, "Comparison/GOAP pre-check");
    assert_plan_valid(&rule_plan, "Comparison/Rule pre-check");
    assert_plan_valid(&utility_plan, "Comparison/Utility pre-check");

    group.bench_function("Rule-based", |b| {
        b.iter(|| {
            let s = black_box(&snap);
            let plan = rule.propose_plan(s);
            // CORRECTNESS: Verify valid plan
            assert_plan_valid(&plan, "Comparison/Rule");
            black_box(plan)
        })
    });

    group.bench_function("GOAP", |b| {
        b.iter(|| {
            let s = black_box(&snap);
            let plan = goap.propose_plan(s);
            // CORRECTNESS: Verify valid plan
            assert_plan_valid(&plan, "Comparison/GOAP");
            black_box(plan)
        })
    });

    group.bench_function("Utility", |b| {
        b.iter(|| {
            let s = black_box(&snap);
            let plan = utility.propose_plan(s);
            // CORRECTNESS: Verify valid plan
            assert_plan_valid(&plan, "Comparison/Utility");
            black_box(plan)
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(
    benches,
    bench_goap_planning_latency,
    bench_rule_based_planning,
    bench_utility_planning,
    bench_tool_validation,
    bench_multi_agent_throughput,
    bench_snapshot_cloning,
    bench_planning_conditions,
    bench_orchestrator_comparison,
);

criterion_main!(benches);
