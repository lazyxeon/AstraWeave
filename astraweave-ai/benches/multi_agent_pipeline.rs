//! Multi-Agent AI Pipeline Integration Benchmark
//!
//! Benchmarks the full AI pipeline for multiple agents:
//! **WorldSnapshot Creation → AI Planning → ActionStep Validation → ECS Feedback**
//!
//! **Targets**:
//! - <5ms for 100 agents (20% of 60 FPS frame budget)
//! - <500µs for 10 agents (typical co-op scenario)
//! - Linear scaling with agent count (O(n))
//! - No heap churn (snapshot reuse)
//!
//! **Scenarios**:
//! 1. Small squad (10 agents) - co-op/squad tactics
//! 2. Medium battle (50 agents) - RTS/tactical scenario
//! 3. Large battle (100 agents) - target capacity
//! 4. Massive battle (500 agents) - stress test

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{
    ActionStep, CompanionState, CoverType, EnemyState, IVec2, PlanIntent, PlayerState, Stance,
    WorldSnapshot,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a WorldSnapshot for a single agent
fn create_agent_snapshot(agent_id: usize, agent_pos: IVec2, enemy_count: usize) -> WorldSnapshot {
    let enemies: Vec<EnemyState> = (0..enemy_count)
        .map(|i| EnemyState {
            id: (agent_id * 1000 + i) as u32,
            pos: IVec2 {
                x: 50 + (i as i32) * 5,
                y: 50 + (i as i32) * 5,
            },
            hp: 80,
            cover: CoverType::None,
            last_seen: 0.0,
        })
        .collect();

    WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: agent_pos,
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: Stance::Stand,
            orders: vec!["attack".to_string()],
        },
        enemies,
        pois: vec![],
        obstacles: vec![],
        objective: Some("eliminate_threats".to_string()),
    }
}

/// Simulate perception phase for N agents
fn perception_phase(agent_count: usize, enemies_per_agent: usize) -> Vec<WorldSnapshot> {
    (0..agent_count)
        .map(|i| {
            let agent_pos = IVec2 {
                x: (i as i32) * 10,
                y: (i as i32) * 10,
            };
            create_agent_snapshot(i, agent_pos, enemies_per_agent)
        })
        .collect()
}

/// Simulate planning phase for N agents
fn planning_phase(snapshots: &[WorldSnapshot]) -> Vec<PlanIntent> {
    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    snapshots
        .iter()
        .map(|snap| {
            dispatch_planner(&controller, snap).unwrap_or_else(|_| PlanIntent {
                plan_id: "fallback".to_string(),
                steps: vec![ActionStep::Wait { duration: 0.1 }],
            })
        })
        .collect()
}

/// Simulate validation phase for N plans
fn validation_phase(plans: &[PlanIntent]) -> usize {
    plans.iter().filter(|plan| !plan.steps.is_empty()).count()
}

/// Simulate ECS feedback phase (update world state)
fn feedback_phase(plans: &[PlanIntent], snapshots: &mut [WorldSnapshot]) {
    for (plan, snapshot) in plans.iter().zip(snapshots.iter_mut()) {
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.first() {
            snapshot.me.pos = IVec2 { x: *x, y: *y };
        }
    }
}

/// Full multi-agent pipeline
fn full_multi_agent_pipeline(
    agent_count: usize,
    enemies_per_agent: usize,
) -> (Vec<WorldSnapshot>, Vec<PlanIntent>, usize) {
    // Phase 1: Perception (create snapshots)
    let snapshots = perception_phase(agent_count, enemies_per_agent);

    // Phase 2: Planning (generate plans)
    let plans = planning_phase(&snapshots);

    // Phase 3: Validation (validate plans)
    let valid_count = validation_phase(&plans);

    (snapshots, plans, valid_count)
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_full_multi_agent_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_multi_agent_pipeline");

    let scenarios = [
        (10, 3, "small_10a_3e"),
        (50, 5, "medium_50a_5e"),
        (100, 5, "large_100a_5e"),
        (500, 5, "stress_500a_5e"),
    ];

    for (agent_count, enemies_per_agent, name) in scenarios {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(agent_count, enemies_per_agent),
            |b, &(agents, enemies)| {
                b.iter(|| full_multi_agent_pipeline(black_box(agents), black_box(enemies)))
            },
        );
    }

    group.finish();
}

fn bench_perception_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("perception_phase");

    let scenarios = [
        (10, 3, "10_agents_3_enemies"),
        (50, 5, "50_agents_5_enemies"),
        (100, 5, "100_agents_5_enemies"),
        (500, 5, "500_agents_5_enemies"),
    ];

    for (agent_count, enemies_per_agent, name) in scenarios {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(agent_count, enemies_per_agent),
            |b, &(agents, enemies)| {
                b.iter(|| perception_phase(black_box(agents), black_box(enemies)))
            },
        );
    }

    group.finish();
}

fn bench_planning_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("planning_phase");

    let scenarios = [
        (10, "10_agents"),
        (50, "50_agents"),
        (100, "100_agents"),
        (500, "500_agents"),
    ];

    for (agent_count, name) in scenarios {
        let snapshots = perception_phase(agent_count, 5);

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &snapshots,
            |b, snapshots| b.iter(|| planning_phase(black_box(snapshots))),
        );
    }

    group.finish();
}

fn bench_validation_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_phase");

    let snapshots = perception_phase(100, 5);
    let plans = planning_phase(&snapshots);

    group.bench_function("100_plans", |b| {
        b.iter(|| validation_phase(black_box(&plans)))
    });

    group.finish();
}

fn bench_feedback_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("feedback_phase");

    let mut snapshots = perception_phase(100, 5);
    let plans = planning_phase(&snapshots);

    group.bench_function("100_agents", |b| {
        b.iter(|| feedback_phase(black_box(&plans), black_box(&mut snapshots)))
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_agent_scaling");

    // Test scaling from 1 to 1000 agents
    let agent_counts = [1, 10, 50, 100, 200, 500, 1000];

    for &agent_count in &agent_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            &agent_count,
            |b, &agents| b.iter(|| full_multi_agent_pipeline(black_box(agents), black_box(5))),
        );
    }

    group.finish();
}

fn bench_per_agent_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("per_agent_latency");

    // Measure per-agent cost at different scales
    let scenarios = [
        (10, "10_agents"),
        (100, "100_agents"),
        (1000, "1000_agents"),
    ];

    for (agent_count, name) in scenarios {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &agent_count,
            |b, &agents| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        let _ = full_multi_agent_pipeline(black_box(agents), black_box(5));
                    }
                    let elapsed = start.elapsed();

                    // Return per-agent time
                    elapsed / (iters as u32 * agents as u32)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_full_multi_agent_pipeline,
    bench_perception_phase,
    bench_planning_phase,
    bench_validation_phase,
    bench_feedback_phase,
    bench_scaling,
    bench_per_agent_latency
);

criterion_main!(benches);
