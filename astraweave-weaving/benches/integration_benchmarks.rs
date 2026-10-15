use astraweave_weaving::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec3;
use std::hint::black_box;

/// Benchmark Player ability cooldown updates (Week 5 Day 1 integration)
fn bench_player_ability_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_abilities");

    for entity_count in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut players: Vec<Player> = (0..count)
                    .map(|i| {
                        let mut p = Player::new(Vec3::new(i as f32, 0.0, 0.0));
                        p.set_echo(100);
                        p
                    })
                    .collect();

                b.iter(|| {
                    for player in &mut players {
                        player.update(black_box(0.016)); // 60 FPS delta
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Player ability activation (dash/shield)
fn bench_player_ability_activation(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_ability_activation");

    // Dash activation
    group.bench_function("dash_activation", |b| {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(1000);
        player.forward = Vec3::new(1.0, 0.0, 0.0);

        b.iter(|| {
            let result = player.use_dash();
            let _ = black_box(result);
            player.add_echo(10); // Refund for continuous testing
            player.update(1.0); // Reset cooldown
        });
    });

    // Shield activation
    group.bench_function("shield_activation", |b| {
        let mut player = Player::new(Vec3::ZERO);
        player.set_echo(1000);

        b.iter(|| {
            let result = player.use_shield();
            let _ = black_box(result);
            player.add_echo(15); // Refund
            player.update(5.0); // Reset cooldown
        });
    });

    group.finish();
}

/// Benchmark Quest objective updates (Week 5 Day 1 integration)
fn bench_quest_objective_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("quest_objectives");

    for quest_count in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(quest_count),
            quest_count,
            |b, &count| {
                // Create quests with different objective types
                let quests: Vec<Quest> = (0..count)
                    .map(|i| {
                        let obj_type = match i % 5 {
                            0 => ObjectiveType::Escort {
                                npc: quest_types::EscortNPC::new(
                                    "NPC",
                                    Vec3::ZERO,
                                    Vec3::new(100.0, 0.0, 0.0),
                                    100.0,
                                ),
                            },
                            1 => ObjectiveType::Defend {
                                objective: quest_types::DefendObjective::new(
                                    "Anchor",
                                    Vec3::ZERO,
                                    10.0,
                                    200.0,
                                    30.0,
                                    3,
                                ),
                                required_waves: 3,
                            },
                            2 => ObjectiveType::Boss {
                                objective: quest_types::BossObjective::new(
                                    "Boss",
                                    500.0,
                                    Vec3::ZERO,
                                    Vec3::ZERO,
                                    50.0,
                                ),
                            },
                            3 => ObjectiveType::TimeTrial {
                                objective: quest_types::TimeTrialObjective::new(15.0, 10.0),
                            },
                            _ => ObjectiveType::Collect {
                                objective: quest_types::CollectObjective::new(
                                    "Item",
                                    vec![Vec3::ZERO; 5],
                                    5.0,
                                ),
                            },
                        };

                        Quest::new(
                            format!("quest_{}", i),
                            format!("Quest {}", i),
                            "Description",
                        )
                        .with_objective(obj_type)
                    })
                    .collect();

                b.iter(|| {
                    for quest in &quests {
                        let _ = black_box(quest.objectives[0].is_complete());
                        let _ = black_box(quest.objectives[0].progress());
                        let _ = black_box(quest.objectives[0].description());
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Enemy spawner archetype determination (Week 5 Day 1 integration)
fn bench_enemy_spawner_archetype(c: &mut Criterion) {
    let mut group = c.benchmark_group("enemy_spawner");

    // Benchmark archetype determination across wave progression
    for wave in [1, 5, 10, 15, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("determine_archetype", wave),
            wave,
            |b, &wave_num| {
                let mut spawner = EnemySpawner::new();
                spawner.add_spawn_point(Vec3::ZERO, 10.0, None);

                // Advance to target wave
                for _ in 0..wave_num {
                    spawner.update(30.0, &[]); // Trigger wave spawn
                }

                b.iter(|| {
                    let requests = spawner.update(black_box(0.016), &[]);
                    black_box(requests);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark full integrated system (Player + Quest + Spawner)
fn bench_integrated_systems(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_systems");

    for scenario_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(scenario_size),
            scenario_size,
            |b, &size| {
                // Setup integrated scenario
                let mut players: Vec<Player> = (0..size)
                    .map(|i| {
                        let mut p = Player::new(Vec3::new(i as f32, 0.0, 0.0));
                        p.set_echo(100);
                        p
                    })
                    .collect();

                let quests: Vec<Quest> = (0..size)
                    .map(|i| {
                        Quest::new(
                            format!("quest_{}", i),
                            format!("Quest {}", i),
                            "Description",
                        )
                        .with_objective(ObjectiveType::Escort {
                            npc: quest_types::EscortNPC::new(
                                "NPC",
                                Vec3::ZERO,
                                Vec3::new(100.0, 0.0, 0.0),
                                100.0,
                            ),
                        })
                    })
                    .collect();

                let mut spawner = EnemySpawner::new();
                spawner.add_spawn_point(Vec3::ZERO, 10.0, None);

                b.iter(|| {
                    // Update all systems
                    for player in &mut players {
                        player.update(black_box(0.016));
                    }

                    for quest in &quests {
                        let _ = black_box(quest.objectives[0].is_complete());
                        let _ = black_box(quest.objectives[0].progress());
                    }

                    let spawn_requests = spawner.update(black_box(0.016), &[]);
                    black_box(spawn_requests);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_player_ability_updates,
    bench_player_ability_activation,
    bench_quest_objective_updates,
    bench_enemy_spawner_archetype,
    bench_integrated_systems,
);
criterion_main!(benches);
