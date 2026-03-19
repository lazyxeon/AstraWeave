//! Full Game Loop Integration Benchmark
//!
//! Benchmarks the complete AI-native game loop:
//! **World Tick → Perception → AI Planning → Physics Update**
//!
//! Measures FULL INTEGRATION using simplified simulation (based on performance_integration.rs pattern).
//!
//! **Targets** (60 FPS @ 1000 entities):
//! - Frame time <16.67ms (60 FPS budget)
//! - Multi-agent coordination (100+ agents)
//! - Deterministic execution
//!
//! **Scenarios**:
//! 1. Small scale (100 entities) - overhead measurement
//! 2. Medium scale (500 entities) - typical game scenario
//! 3. Large scale (1000 entities) - 60 FPS target
//! 4. Stress test (5000 entities) - capacity measurement

use astraweave_ai::{Orchestrator, RuleOrchestrator};
use astraweave_core::{
    ActionStep, CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, Pose, Team, World,
    WorldSnapshot,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

// ============================================================================
// Helper Functions
// ============================================================================

/// Simulate a typical game frame with entity updates (based on performance_integration.rs)
fn simulate_game_frame(world: &mut World, dt: f32) {
    // Tick world (cooldowns, time advancement)
    world.tick(dt);

    // Simulate entity movement (update all poses)
    let entities: Vec<_> = world.entities();
    for entity in entities {
        if let Some(pose) = world.pose_mut(entity) {
            // Simple movement pattern (circular motion)
            pose.pos.x = (pose.pos.x + 1) % 100;
            pose.pos.y = (pose.pos.y + 1) % 100;
        }
    }

    // Simulate damage over time (update all health)
    for entity in world.entities() {
        if let Some(health) = world.health_mut(entity) {
            health.hp = (health.hp - 1).max(0);
        }
    }

    // Simulate AI queries (find enemies - typical perception cost)
    for team_id in 0..3 {
        let _allies = world.all_of_team(team_id);
        let _enemies = world.enemies_of(team_id);
    }
}

/// Create a world with N entities
fn create_world_with_entities(count: usize) -> World {
    let mut world = World::new();

    // Distribute entities across 3 teams
    let team_distribution = [
        (0, count / 3),               // Player team (33%)
        (1, count / 3),               // Companion team (33%)
        (2, count - 2 * (count / 3)), // Enemy team (34%, remainder)
    ];

    for (team_id, team_count) in &team_distribution {
        for i in 0..*team_count {
            let pos = IVec2 {
                x: ((i % 32) * 2) as i32,
                y: ((i / 32) * 2) as i32,
            };
            let team = Team { id: *team_id };
            let name = format!("entity_t{}_{}", team_id, i);
            world.spawn(&name, pos, team, 100, 30);
        }
    }

    world
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_single_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_game_loop_single_frame");

    let scenarios = [
        (100, "100_entities"),
        (500, "500_entities"),
        (1000, "1000_entities"),
        (5000, "5000_entities"),
    ];

    for (entity_count, name) in scenarios {
        let mut world = create_world_with_entities(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(name), &entity_count, |b, _| {
            b.iter(|| {
                simulate_game_frame(black_box(&mut world), black_box(0.016));
            })
        });
    }

    group.finish();
}

fn bench_multi_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_game_loop_multi_frame");

    let scenarios = [
        (100, 60, "100e_60f"),
        (500, 60, "500e_60f"),
        (1000, 60, "1000e_60f"),
    ];

    for (entity_count, frame_count, name) in scenarios {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(entity_count, frame_count),
            |b, &(entities, frames)| {
                b.iter(|| {
                    let mut world = create_world_with_entities(entities);
                    for _ in 0..frames {
                        simulate_game_frame(black_box(&mut world), black_box(0.016));
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_frame_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_game_loop_scaling");

    let entity_counts = [50, 100, 250, 500, 1000, 2000, 5000];

    for &entity_count in &entity_counts {
        let mut world = create_world_with_entities(entity_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, _| {
                b.iter(|| {
                    simulate_game_frame(black_box(&mut world), black_box(0.016));
                })
            },
        );
    }

    group.finish();
}

// Merged with second criterion_group below (avoiding duplicate benches name)

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a world with specified entity/agent counts
fn create_benchmark_world(entity_count: usize, ai_agent_count: usize) -> (World, Vec<u32>) {
    let mut world = World::new();
    let mut ai_agent_ids = vec![];

    // Create AI agents (companions with perception)
    for i in 0..ai_agent_count {
        let pos = IVec2 {
            x: (i as i32) * 5,
            y: (i as i32) * 5,
        };
        let team = Team { id: 1 }; // companion
        let agent_id = world.spawn("companion", pos, team, 100, 30);
        ai_agent_ids.push(agent_id);
    }

    // Create passive entities (enemies, obstacles, POIs)
    let remaining = entity_count - ai_agent_count;
    let enemies = remaining / 2;
    let objects = remaining - enemies;

    // Spawn enemies
    for i in 0..enemies {
        let pos = IVec2 {
            x: 50 + (i as i32) * 3,
            y: 50 + (i as i32) * 3,
        };
        let team = Team { id: 2 }; // enemy
        world.spawn("enemy", pos, team, 80, 0);
    }

    // Spawn objects
    for i in 0..objects {
        let pos = IVec2 {
            x: (i as i32) * 2,
            y: (i as i32) * 2,
        };
        let team = Team { id: 0 }; // neutral
        world.spawn("object", pos, team, 10, 0);
    }

    (world, ai_agent_ids)
}

/// Simulate perception stage: Build WorldSnapshots for all agents
fn perception_stage(world: &World, ai_agents: &[u32]) -> Vec<WorldSnapshot> {
    let player_pos = IVec2 { x: 0, y: 0 };

    ai_agents
        .iter()
        .map(|&agent_id| {
            let agent_pose = world.pose(agent_id).unwrap_or(Pose {
                pos: IVec2 { x: 0, y: 0 },
                height: 0.0,
                rotation: 0.0,
                rotation_x: 0.0,
                rotation_z: 0.0,
                scale: 1.0,
            });
            let agent_ammo = world.ammo(agent_id).map(|a| a.rounds).unwrap_or(0);
            let agent_health = world.health(agent_id).map(|h| h.hp).unwrap_or(100);

            // Find nearby enemies (simplified perception)
            let enemies: Vec<EnemyState> = world
                .all_of_team(2) // enemy team
                .iter()
                .filter_map(|&enemy_id| {
                    let enemy_pose = world.pose(enemy_id)?;
                    let enemy_hp = world.health(enemy_id).map(|h| h.hp).unwrap_or(0);

                    // Distance check (perception radius = 20)
                    let dx = (enemy_pose.pos.x - agent_pose.pos.x).abs();
                    let dy = (enemy_pose.pos.y - agent_pose.pos.y).abs();
                    if dx <= 20 && dy <= 20 {
                        Some(EnemyState {
                            id: enemy_id,
                            pos: enemy_pose.pos,
                            hp: enemy_hp,
                            cover: "none".to_string(),
                            last_seen: 0.0,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            WorldSnapshot {
                t: world.t,
                me: CompanionState {
                    ammo: agent_ammo,
                    cooldowns: BTreeMap::new(),
                    morale: (agent_health as f32) / 100.0,
                    pos: agent_pose.pos,
                },
                player: PlayerState {
                    hp: 100,
                    pos: player_pos,
                    stance: "stand".into(),
                    orders: vec![],
                },
                enemies,
                pois: vec![],
                obstacles: vec![],
                objective: Some("patrol".to_string()),
            }
        })
        .collect()
}

/// Simulate AI planning stage: Generate PlanIntents
fn planning_stage(snapshots: &[WorldSnapshot], orchestrator: &RuleOrchestrator) -> Vec<PlanIntent> {
    snapshots
        .iter()
        .map(|snap| orchestrator.propose_plan(snap))
        .collect()
}

/// Simulate physics stage: Update world state from plans
fn physics_stage(world: &mut World, ai_agents: &[u32], plans: &[PlanIntent]) {
    for (&agent_id, plan) in ai_agents.iter().zip(plans.iter()) {
        // Execute first action step (simplified)
        if let Some(step) = plan.steps.first() {
            match step {
                ActionStep::MoveTo { x, y, speed: _ } => {
                    if let Some(pose) = world.pose_mut(agent_id) {
                        pose.pos = IVec2 { x: *x, y: *y };
                    }
                }
                ActionStep::Attack { target_id } => {
                    if let Some(health) = world.health_mut(*target_id) {
                        health.hp = (health.hp - 10).max(0);
                    }
                }
                ActionStep::TakeCover { position } => {
                    if let Some(pose) = world.pose_mut(agent_id) {
                        if let Some(pos) = position {
                            pose.pos = *pos;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Simulate rendering prep: Cull entities, update transforms
fn rendering_prep_stage(world: &World) -> usize {
    // Simulate frustum culling (count visible entities)
    let camera_pos = IVec2 { x: 0, y: 0 };
    let view_radius = 50;

    world
        .entities()
        .iter()
        .filter(|&&entity_id| {
            if let Some(pose) = world.pose(entity_id) {
                let dx = (pose.pos.x - camera_pos.x).abs();
                let dy = (pose.pos.y - camera_pos.y).abs();
                dx <= view_radius && dy <= view_radius
            } else {
                false
            }
        })
        .count()
}

/// Full game loop: All stages integrated
fn full_game_loop(world: &mut World, ai_agents: &[u32], orchestrator: &RuleOrchestrator) -> usize {
    // Tick world (cooldowns, time advancement)
    world.tick(0.016); // 60 FPS timestep

    // Stage 1: Perception
    let snapshots = perception_stage(world, ai_agents);

    // Stage 2: AI Planning
    let plans = planning_stage(&snapshots, orchestrator);

    // Stage 3: Physics
    physics_stage(world, ai_agents, &plans);

    // Stage 4: Rendering Prep
    rendering_prep_stage(world)
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_full_game_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_game_loop");

    let scenarios = [
        (100, 10, "small_100e_10a"),
        (500, 50, "medium_500e_50a"),
        (1000, 100, "large_1000e_100a"),
        (5000, 500, "stress_5000e_500a"),
    ];

    for (entity_count, ai_count, name) in scenarios {
        group.bench_function(name, |b| {
            b.iter_with_setup(
                || {
                    let (world, ai_agents) = create_benchmark_world(entity_count, ai_count);
                    (world, ai_agents, RuleOrchestrator)
                },
                |(mut world, ai_agents, orchestrator)| {
                    black_box(full_game_loop(&mut world, &ai_agents, &orchestrator))
                },
            )
        });
    }

    group.finish();
}

fn bench_perception_stage(c: &mut Criterion) {
    let mut group = c.benchmark_group("perception_stage");

    let scenarios = [
        (100, 10, "10_agents"),
        (500, 50, "50_agents"),
        (1000, 100, "100_agents"),
    ];

    for (entity_count, ai_count, name) in scenarios {
        let (world, ai_agents) = create_benchmark_world(entity_count, ai_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(&world, &ai_agents),
            |b, (world, ai_agents)| {
                b.iter(|| perception_stage(black_box(*world), black_box(*ai_agents)))
            },
        );
    }

    group.finish();
}

fn bench_planning_stage(c: &mut Criterion) {
    let mut group = c.benchmark_group("planning_stage");

    let (world, ai_agents) = create_benchmark_world(1000, 100);
    let snapshots = perception_stage(&world, &ai_agents);
    let orchestrator = RuleOrchestrator;

    group.bench_function("100_agents", |b| {
        b.iter(|| planning_stage(black_box(&snapshots), black_box(&orchestrator)))
    });

    group.finish();
}

fn bench_physics_stage(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_stage");

    let (mut world, ai_agents) = create_benchmark_world(1000, 100);
    let snapshots = perception_stage(&world, &ai_agents);
    let orchestrator = RuleOrchestrator;
    let plans = planning_stage(&snapshots, &orchestrator);

    group.bench_function("100_agents", |b| {
        b.iter(|| {
            physics_stage(
                black_box(&mut world),
                black_box(&ai_agents),
                black_box(&plans),
            )
        })
    });

    group.finish();
}

fn bench_rendering_prep(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering_prep");

    let scenarios = [
        (100, "100_entities"),
        (500, "500_entities"),
        (1000, "1000_entities"),
    ];

    for (entity_count, name) in scenarios {
        let (world, _) = create_benchmark_world(entity_count, entity_count / 10);

        group.bench_with_input(BenchmarkId::from_parameter(name), &world, |b, world| {
            b.iter(|| rendering_prep_stage(black_box(world)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_frame,
    bench_multi_frame,
    bench_frame_scaling,
    bench_full_game_loop,
    bench_perception_stage,
    bench_planning_stage,
    bench_physics_stage,
    bench_rendering_prep
);

criterion_main!(benches);
