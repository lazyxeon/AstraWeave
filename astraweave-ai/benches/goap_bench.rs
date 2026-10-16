// Benchmark for GoapOrchestrator to validate <100 µs target for Phase 3

use astraweave_ai::{GoapOrchestrator, Orchestrator};
use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

fn create_test_snapshot(enemy_dist: i32) -> WorldSnapshot {
    WorldSnapshot {
        t: 1.234,
        player: PlayerState {
            hp: 80,

            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 0.5,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 2,
            pos: IVec2 {
                x: enemy_dist,
                y: 0,
            },
            hp: 50,
            cover: "low".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn bench_goap_propose_plan_close(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let snap = create_test_snapshot(1); // Enemy at distance 1 (in range)

    c.bench_function("goap_propose_plan_close", |b| {
        b.iter(|| {
            let plan = goap.propose_plan(black_box(&snap));
            black_box(plan);
        })
    });
}

fn bench_goap_propose_plan_far(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let snap = create_test_snapshot(10); // Enemy at distance 10 (far)

    c.bench_function("goap_propose_plan_far", |b| {
        b.iter(|| {
            let plan = goap.propose_plan(black_box(&snap));
            black_box(plan);
        })
    });
}

fn bench_goap_next_action_close(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let snap = create_test_snapshot(1); // Enemy at distance 1 (in range)

    c.bench_function("goap_next_action_close", |b| {
        b.iter(|| {
            let action = goap.next_action(black_box(&snap));
            black_box(action);
        })
    });
}

fn bench_goap_next_action_far(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let snap = create_test_snapshot(10); // Enemy at distance 10 (far)

    c.bench_function("goap_next_action_far", |b| {
        b.iter(|| {
            let action = goap.next_action(black_box(&snap));
            black_box(action);
        })
    });
}

fn bench_goap_next_action_no_enemies(c: &mut Criterion) {
    let goap = GoapOrchestrator;
    let mut snap = create_test_snapshot(10);
    snap.enemies.clear(); // No enemies

    c.bench_function("goap_next_action_no_enemies", |b| {
        b.iter(|| {
            let action = goap.next_action(black_box(&snap));
            black_box(action);
        })
    });
}

criterion_group!(
    benches,
    bench_goap_propose_plan_close,
    bench_goap_propose_plan_far,
    bench_goap_next_action_close,
    bench_goap_next_action_far,
    bench_goap_next_action_no_enemies
);
criterion_main!(benches);
