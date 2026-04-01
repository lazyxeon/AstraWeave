// Benchmark comparing Advanced GOAP vs Rule-based Orchestrator
// Phase 1: Baseline Performance Metrics

use criterion::{criterion_group, criterion_main};

#[cfg(feature = "planner_advanced")]
use std::hint::black_box;

#[cfg(feature = "planner_advanced")]
use astraweave_core::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance, WorldSnapshot};
#[cfg(feature = "planner_advanced")]
use criterion::{BenchmarkId, Criterion};
#[cfg(feature = "planner_advanced")]
use std::collections::BTreeMap;

#[cfg(feature = "planner_advanced")]
use astraweave_behavior::goap::*;

#[cfg(feature = "planner_advanced")]
fn create_test_snapshot(enemy_count: usize, distance: i32) -> WorldSnapshot {
    let enemies: Vec<EnemyState> = (0..enemy_count)
        .map(|i| EnemyState {
            id: i as u32 + 1,
            pos: IVec2 {
                x: distance + (i as i32 * 2),
                y: distance,
                physics_context: None,
            },
            hp: 50,
            cover: CoverType::None,
            last_seen: 1.0,
        })
        .collect();

    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            physics_context: None,
            pos: IVec2 { x: 0, y: 0 },
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            ammo: 20,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: 5, y: 5 },
        },
        enemies,
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

#[cfg(feature = "planner_advanced")]
fn bench_goap_planning(c: &mut Criterion) {
    let mut group = c.benchmark_group("goap_planning");

    for enemy_count in [1, 3, 5].iter() {
        group.bench_with_input(
            BenchmarkId::new("plan_generation", enemy_count),
            enemy_count,
            |b, &enemy_count| {
                let mut orch = GOAPOrchestrator::new();
                let snap = create_test_snapshot(enemy_count, 10);

                b.iter(|| {
                    let intent = orch.propose_plan(&snap);
                    black_box(intent);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "planner_advanced")]
fn bench_goap_with_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("goap_with_learning");

    // Benchmark planning with various amounts of historical data
    for history_size in [0, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("with_history", history_size),
            history_size,
            |b, &history_size| {
                let mut orch = GOAPOrchestrator::new();
                let snap = create_test_snapshot(1, 10);

                // Populate history
                let history = orch.planner_mut().get_history_mut();
                for i in 0..history_size {
                    if i % 2 == 0 {
                        history.record_success("attack_enemy", 1.0);
                    } else {
                        history.record_failure("attack_enemy");
                    }
                }

                b.iter(|| {
                    let intent = orch.propose_plan(&snap);
                    black_box(intent);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "planner_advanced")]
fn bench_multi_goal_planning(c: &mut Criterion) {
    let mut goap = AdvancedGOAP::new();

    // Register simple actions
    let mut move_effects = BTreeMap::new();
    move_effects.insert("moved".to_string(), StateValue::Bool(true));
    goap.add_action(Box::new(SimpleAction::new(
        "move",
        BTreeMap::new(),
        move_effects,
        1.0,
    )));

    let mut attack_effects = BTreeMap::new();
    attack_effects.insert("attacked".to_string(), StateValue::Bool(true));
    goap.add_action(Box::new(SimpleAction::new(
        "attack",
        BTreeMap::new(),
        attack_effects,
        2.0,
    )));

    let mut heal_effects = BTreeMap::new();
    heal_effects.insert("healed".to_string(), StateValue::Bool(true));
    goap.add_action(Box::new(SimpleAction::new(
        "heal",
        BTreeMap::new(),
        heal_effects,
        3.0,
    )));

    c.bench_function("multi_goal_planning_3_goals", |b| {
        let start = WorldState::new();

        let mut goal1_state = BTreeMap::new();
        goal1_state.insert("moved".to_string(), StateValue::Bool(true));
        let goal1 = Goal::new("move_goal", goal1_state).with_priority(5.0);

        let mut goal2_state = BTreeMap::new();
        goal2_state.insert("attacked".to_string(), StateValue::Bool(true));
        let goal2 = Goal::new("attack_goal", goal2_state).with_priority(3.0);

        let mut goal3_state = BTreeMap::new();
        goal3_state.insert("healed".to_string(), StateValue::Bool(true));
        let goal3 = Goal::new("heal_goal", goal3_state).with_priority(10.0);

        let goals = vec![goal1, goal2, goal3];

        b.iter(|| {
            let plans = goap.plan_multiple_goals(&start, &goals, 0.0);
            black_box(plans);
        });
    });
}

#[cfg(feature = "planner_advanced")]
fn bench_state_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_operations");

    group.bench_function("state_hashing", |b| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut state = WorldState::new();
        for i in 0..20 {
            state.set(format!("key_{}", i), StateValue::Int(i));
        }

        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            state.hash(&mut hasher);
            black_box(hasher.finish());
        });
    });

    group.bench_function("state_satisfies", |b| {
        let mut state = WorldState::new();
        state.set("health", StateValue::Int(75));
        state.set("ammo", StateValue::Int(20));
        state.set("in_combat", StateValue::Bool(true));

        let mut conditions = BTreeMap::new();
        conditions.insert("health".to_string(), StateValue::IntRange(50, 100));
        conditions.insert("ammo".to_string(), StateValue::IntRange(10, 30));
        conditions.insert("in_combat".to_string(), StateValue::Bool(true));

        b.iter(|| {
            let result = state.satisfies(&conditions);
            black_box(result);
        });
    });

    group.bench_function("state_distance", |b| {
        let mut current = WorldState::new();
        current.set("health", StateValue::Int(30));
        current.set("ammo", StateValue::Int(5));
        current.set("in_combat", StateValue::Bool(true));

        let mut goal = BTreeMap::new();
        goal.insert("health".to_string(), StateValue::Int(100));
        goal.insert("ammo".to_string(), StateValue::Int(30));
        goal.insert("in_combat".to_string(), StateValue::Bool(false));

        b.iter(|| {
            let distance = current.distance_to(&goal);
            black_box(distance);
        });
    });

    group.finish();
}

#[cfg(feature = "planner_advanced")]
fn bench_action_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_history");

    group.bench_function("record_success", |b| {
        let mut history = ActionHistory::new();

        b.iter(|| {
            history.record_success("test_action", 1.5);
        });
    });

    group.bench_function("get_stats_with_100_actions", |b| {
        let mut history = ActionHistory::new();
        for i in 0..100 {
            history.record_success(&format!("action_{}", i), 1.0);
        }

        b.iter(|| {
            let stats = history.get_action_stats("action_50");
            black_box(stats);
        });
    });

    group.bench_function("merge_histories", |b| {
        let mut history1 = ActionHistory::new();
        for i in 0..50 {
            history1.record_success(&format!("action_{}", i), 1.0);
        }

        let mut history2 = ActionHistory::new();
        for i in 25..75 {
            history2.record_success(&format!("action_{}", i), 1.0);
        }

        b.iter(|| {
            let mut h = history1.clone();
            h.merge(&history2);
            black_box(h);
        });
    });

    group.finish();
}

#[cfg(feature = "planner_advanced")]
criterion_group!(
    benches,
    bench_goap_planning,
    bench_goap_with_history,
    bench_multi_goal_planning,
    bench_state_operations,
    bench_action_history
);

#[cfg(not(feature = "planner_advanced"))]
fn bench_feature_disabled(c: &mut criterion::Criterion) {
    c.bench_function("goap_feature_disabled", |b| {
        b.iter(|| {
            // Placeholder when planner_advanced feature is not enabled
            std::hint::black_box(0)
        })
    });
}

#[cfg(not(feature = "planner_advanced"))]
criterion_group!(benches, bench_feature_disabled);

criterion_main!(benches);
