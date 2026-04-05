use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{CompanionState, CoverType, IVec2, PlayerState, Stance, WorldSnapshot};
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

/// Helper to create a minimal WorldSnapshot for benchmarking
fn create_simple_snapshot() -> WorldSnapshot {
    WorldSnapshot {
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
            stance: Stance::Stand,
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

/// Helper to create a moderate WorldSnapshot with some entities
fn create_moderate_snapshot() -> WorldSnapshot {
    use astraweave_core::{EnemyState, Poi};

    WorldSnapshot {
        t: 10.5,
        me: CompanionState {
            ammo: 5,
            cooldowns: {
                let mut map = BTreeMap::new();
                map.insert("grenade".to_string(), 2.5);
                map.insert("heal".to_string(), 0.5);
                map
            },
            morale: 0.7,
            pos: IVec2 { x: 10, y: 12 },
        },
        player: PlayerState {
            hp: 75,
            pos: IVec2 { x: 8, y: 10 },
            stance: Stance::Crouch,
            orders: vec!["hold_position".to_string()],
        },
        enemies: vec![
            EnemyState {
                id: 100,
                pos: IVec2 { x: 20, y: 15 },
                hp: 50,
                cover: CoverType::Low,
                last_seen: 1.0,
            },
            EnemyState {
                id: 101,
                pos: IVec2 { x: 22, y: 18 },
                hp: 80,
                cover: CoverType::High,
                last_seen: 0.5,
            },
        ],
        pois: vec![
            Poi {
                k: "medkit".to_string(),
                pos: IVec2 { x: 5, y: 5 },
            },
            Poi {
                k: "ammo".to_string(),
                pos: IVec2 { x: 7, y: 8 },
            },
        ],
        obstacles: vec![
            IVec2 { x: 15, y: 15 },
            IVec2 { x: 16, y: 15 },
            IVec2 { x: 17, y: 15 },
        ],
        objective: Some("eliminate_all_enemies".to_string()),
    }
}

/// Helper to create a complex WorldSnapshot with many entities
fn create_complex_snapshot() -> WorldSnapshot {
    use astraweave_core::{EnemyState, Poi};

    let mut enemies = vec![];
    for i in 0..10 {
        enemies.push(EnemyState {
            id: 200 + i as u32,
            pos: IVec2 {
                x: 20 + i * 2,
                y: 15 + (i % 3),
            },
            hp: 50 + i * 5,
            cover: if i % 2 == 0 {
                CoverType::Low
            } else {
                CoverType::High
            },
            last_seen: (i as f32) * 0.5,
        });
    }

    let mut pois = vec![];
    for i in 0..5 {
        pois.push(Poi {
            k: if i % 2 == 0 { "medkit" } else { "ammo" }.to_string(),
            pos: IVec2 {
                x: 5 + i * 3,
                y: 5 + i * 2,
            },
        });
    }

    let mut obstacles = vec![];
    for i in 0..20 {
        obstacles.push(IVec2 {
            x: 10 + (i % 10),
            y: 10 + (i / 10),
        });
    }

    WorldSnapshot {
        t: 45.7,
        me: CompanionState {
            ammo: 2,
            cooldowns: {
                let mut map = BTreeMap::new();
                map.insert("grenade".to_string(), 5.0);
                map.insert("heal".to_string(), 3.0);
                map.insert("sprint".to_string(), 1.5);
                map.insert("special_ability".to_string(), 10.0);
                map
            },
            morale: 0.4,
            pos: IVec2 { x: 15, y: 20 },
        },
        player: PlayerState {
            hp: 30,
            pos: IVec2 { x: 12, y: 18 },
            stance: Stance::Prone,
            orders: vec![
                "retreat".to_string(),
                "call_for_backup".to_string(),
                "suppress_fire".to_string(),
            ],
        },
        enemies,
        pois,
        obstacles,
        objective: Some("survive_and_extract".to_string()),
    }
}

/// Benchmark simple WorldSnapshot creation
fn bench_snapshot_creation_simple(c: &mut Criterion) {
    c.bench_function("ai_loop_snapshot_creation_simple", |b| {
        b.iter(|| {
            let snapshot = create_simple_snapshot();
            black_box(snapshot)
        });
    });
}

/// Benchmark moderate WorldSnapshot creation
fn bench_snapshot_creation_moderate(c: &mut Criterion) {
    c.bench_function("ai_loop_snapshot_creation_moderate", |b| {
        b.iter(|| {
            let snapshot = create_moderate_snapshot();
            black_box(snapshot)
        });
    });
}

/// Benchmark complex WorldSnapshot creation
fn bench_snapshot_creation_complex(c: &mut Criterion) {
    c.bench_function("ai_loop_snapshot_creation_complex", |b| {
        b.iter(|| {
            let snapshot = create_complex_snapshot();
            black_box(snapshot)
        });
    });
}

/// Benchmark rule-based planner dispatch (simple snapshot)
fn bench_rule_planner_simple(c: &mut Criterion) {
    c.bench_function("ai_loop_rule_planner_simple", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };
        let snapshot = create_simple_snapshot();

        b.iter(|| {
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark rule-based planner dispatch (moderate snapshot)
fn bench_rule_planner_moderate(c: &mut Criterion) {
    c.bench_function("ai_loop_rule_planner_moderate", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };
        let snapshot = create_moderate_snapshot();

        b.iter(|| {
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark rule-based planner dispatch (complex snapshot)
fn bench_rule_planner_complex(c: &mut Criterion) {
    c.bench_function("ai_loop_rule_planner_complex", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };
        let snapshot = create_complex_snapshot();

        b.iter(|| {
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark full AI loop end-to-end (snapshot creation + planning)
fn bench_full_ai_loop_simple(c: &mut Criterion) {
    c.bench_function("ai_loop_full_end_to_end_simple", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };

        b.iter(|| {
            let snapshot = create_simple_snapshot();
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark full AI loop end-to-end (moderate)
fn bench_full_ai_loop_moderate(c: &mut Criterion) {
    c.bench_function("ai_loop_full_end_to_end_moderate", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };

        b.iter(|| {
            let snapshot = create_moderate_snapshot();
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark full AI loop end-to-end (complex)
fn bench_full_ai_loop_complex(c: &mut Criterion) {
    c.bench_function("ai_loop_full_end_to_end_complex", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };

        b.iter(|| {
            let snapshot = create_complex_snapshot();
            let plan = dispatch_planner(black_box(&controller), black_box(&snapshot)).unwrap();
            black_box(plan)
        });
    });
}

/// Benchmark PlanIntent validation (checking plan steps)
fn bench_plan_validation(c: &mut Criterion) {
    c.bench_function("ai_loop_plan_validation", |b| {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };
        let snapshot = create_moderate_snapshot();

        b.iter(|| {
            let plan = dispatch_planner(&controller, &snapshot).unwrap();
            // Validate plan has steps and plan_id
            black_box(!plan.steps.is_empty() && !plan.plan_id.is_empty())
        });
    });
}

criterion_group!(
    benches,
    bench_snapshot_creation_simple,
    bench_snapshot_creation_moderate,
    bench_snapshot_creation_complex,
    bench_rule_planner_simple,
    bench_rule_planner_moderate,
    bench_rule_planner_complex,
    bench_full_ai_loop_simple,
    bench_full_ai_loop_moderate,
    bench_full_ai_loop_complex,
    bench_plan_validation,
);

criterion_main!(benches);
