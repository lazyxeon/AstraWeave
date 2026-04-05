use astraweave_ecs::World;
use astraweave_persistence_ecs::{CPersistenceManager, CReplayState};
use aw_save::{SaveBundleV2, SaveManager, WorldState, SAVE_SCHEMA_VERSION};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hint::black_box;
use tempfile::tempdir;
use time::OffsetDateTime;
use uuid::Uuid;

// ==================== Test Components ====================

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Inventory {
    items: Vec<String>,
    gold: u32,
}

// ==================== GROUP 1: ECS Serialization Benchmarks ====================

fn benchmark_serialize_single_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_serialization/single_component");

    let position = Position {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    group.bench_function("serialize_position", |b| {
        b.iter(|| {
            let encoded = postcard::to_allocvec(black_box(&position)).unwrap();
            black_box(encoded)
        })
    });

    group.bench_function("deserialize_position", |b| {
        let encoded = postcard::to_allocvec(&position).unwrap();
        b.iter(|| {
            let decoded: Position = postcard::from_bytes(black_box(&encoded)).unwrap();
            black_box(decoded)
        })
    });

    let health = Health {
        current: 100,
        max: 150,
    };

    group.bench_function("serialize_health", |b| {
        b.iter(|| {
            let encoded = postcard::to_allocvec(black_box(&health)).unwrap();
            black_box(encoded)
        })
    });

    group.bench_function("deserialize_health", |b| {
        let encoded = postcard::to_allocvec(&health).unwrap();
        b.iter(|| {
            let decoded: Health = postcard::from_bytes(black_box(&encoded)).unwrap();
            black_box(decoded)
        })
    });

    group.finish();
}

fn benchmark_serialize_entity_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_serialization/entity_batch");

    for entity_count in [10, 50, 100, 500, 1000] {
        let entities = create_test_entities(entity_count);

        group.bench_with_input(
            BenchmarkId::new("serialize", entity_count),
            &entities,
            |b, entities| {
                b.iter(|| {
                    let encoded = postcard::to_allocvec(black_box(entities)).unwrap();
                    black_box(encoded)
                })
            },
        );

        let encoded = postcard::to_allocvec(&entities).unwrap();

        group.bench_with_input(
            BenchmarkId::new("deserialize", entity_count),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let decoded: Vec<EntitySnapshot> =
                        postcard::from_bytes(black_box(encoded)).unwrap();
                    black_box(decoded)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 2: World Hash Benchmarks ====================

fn benchmark_world_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_hash/calculation");

    for entity_count in [10, 50, 100, 500, 1000] {
        let entities = create_test_entities(entity_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entities,
            |b, entities| {
                b.iter(|| {
                    let hash = compute_entities_hash(black_box(entities));
                    black_box(hash)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 3: Save/Load Cycle Benchmarks ====================

fn benchmark_save_load_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_load_cycle/full_pipeline");

    let temp_dir = tempdir().unwrap();
    let save_manager = SaveManager::new(temp_dir.path());

    for entity_count in [10, 50, 100, 500] {
        let entities = create_test_entities(entity_count);
        let ecs_blob = postcard::to_allocvec(&entities).unwrap();

        let bundle = create_test_bundle("player1", 0, 1000, &ecs_blob);

        group.bench_with_input(
            BenchmarkId::new("save", entity_count),
            &bundle,
            |b, bundle| {
                b.iter(|| {
                    let _path = save_manager
                        .save(
                            black_box("player1"),
                            black_box(0),
                            black_box(bundle.clone()),
                        )
                        .unwrap();
                })
            },
        );

        // Pre-save for load benchmark
        let _ = save_manager.save("player1", 0, bundle.clone()).unwrap();

        group.bench_function(BenchmarkId::new("load", entity_count), |b| {
            b.iter(|| {
                let (_bundle, _path) = save_manager
                    .load_latest_slot(black_box("player1"), black_box(0))
                    .unwrap();
            })
        });
    }

    group.finish();
}

// ==================== GROUP 4: Replay Benchmarks ====================

fn benchmark_replay_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay/operations");

    for event_count in [10, 50, 100, 500] {
        let replay_state = create_test_replay_state(event_count);

        group.bench_with_input(
            BenchmarkId::new("serialize", event_count),
            &replay_state,
            |b, replay| {
                b.iter(|| {
                    let encoded = postcard::to_allocvec(black_box(replay)).unwrap();
                    black_box(encoded)
                })
            },
        );

        let encoded = postcard::to_allocvec(&replay_state).unwrap();

        group.bench_with_input(
            BenchmarkId::new("deserialize", event_count),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let decoded: CReplayState = postcard::from_bytes(black_box(encoded)).unwrap();
                    black_box(decoded)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_replay_tick_advance(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay/tick_advance");

    let mut world = World::new();
    let entity = world.spawn();
    let replay = create_test_replay_state(1000);
    world.insert(entity, replay);

    group.bench_function("advance_tick", |b| {
        b.iter(|| {
            if let Some(replay) = world.get_mut::<CReplayState>(entity) {
                if replay.current_tick < replay.total_ticks {
                    replay.current_tick += 1;
                }
            }
        })
    });

    group.finish();
}

// ==================== GROUP 5: Persistence Manager Benchmarks ====================

fn benchmark_persistence_manager_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence_manager/operations");

    let temp_dir = tempdir().unwrap();
    let save_manager = SaveManager::new(temp_dir.path());
    let persistence = CPersistenceManager {
        save_manager,
        current_player: "test_player".to_string(),
    };

    // Pre-save some files
    let entities = create_test_entities(100);
    let ecs_blob = postcard::to_allocvec(&entities).unwrap();
    for slot in 0..5 {
        let _ = persistence.save_game(slot, 1000, 12345, ecs_blob.clone());
    }

    group.bench_function("list_saves", |b| {
        b.iter(|| {
            let saves = persistence.list_saves().unwrap();
            black_box(saves)
        })
    });

    group.bench_function("load_game", |b| {
        b.iter(|| {
            let (bundle, _path) = persistence.load_game(black_box(0)).unwrap();
            black_box(bundle)
        })
    });

    group.bench_function("save_game", |b| {
        b.iter(|| {
            let _path = persistence
                .save_game(
                    black_box(1),
                    black_box(2000),
                    black_box(54321),
                    black_box(ecs_blob.clone()),
                )
                .unwrap();
        })
    });

    group.finish();
}

// ==================== GROUP 6: Scaling Benchmarks ====================

fn benchmark_scaling_entity_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling/entity_count");

    let temp_dir = tempdir().unwrap();
    let save_manager = SaveManager::new(temp_dir.path());

    for entity_count in [100, 500, 1000, 5000] {
        let entities = create_test_entities(entity_count);
        let ecs_blob = postcard::to_allocvec(&entities).unwrap();
        let bundle = create_test_bundle("player1", 0, 1000, &ecs_blob);

        group.bench_with_input(
            BenchmarkId::new("full_save", entity_count),
            &(&save_manager, &bundle),
            |b, (mgr, bundle)| {
                b.iter(|| {
                    let _path = mgr
                        .save(
                            black_box("player1"),
                            black_box(0),
                            black_box((*bundle).clone()),
                        )
                        .unwrap();
                })
            },
        );

        // Pre-save for load benchmark
        let _ = save_manager.save("player1", 0, bundle.clone()).unwrap();

        group.bench_with_input(
            BenchmarkId::new("full_load", entity_count),
            &save_manager,
            |b, mgr| {
                b.iter(|| {
                    let (_bundle, _path) = mgr
                        .load_latest_slot(black_box("player1"), black_box(0))
                        .unwrap();
                })
            },
        );
    }

    group.finish();
}

// ==================== Helper Functions ====================

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EntitySnapshot {
    id: u64,
    position: Position,
    health: Health,
    inventory: Inventory,
}

fn create_test_entities(count: usize) -> Vec<EntitySnapshot> {
    (0..count)
        .map(|i| EntitySnapshot {
            id: i as u64,
            position: Position {
                x: i as f32,
                y: i as f32 * 2.0,
                z: i as f32 * 3.0,
            },
            health: Health {
                current: 100 - (i % 100) as i32,
                max: 150,
            },
            inventory: Inventory {
                items: vec![format!("item_{}", i), format!("weapon_{}", i)],
                gold: (i * 100) as u32,
            },
        })
        .collect()
}

fn compute_entities_hash(entities: &[EntitySnapshot]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    for entity in entities {
        entity.id.hash(&mut hasher);
        // Hash position
        (entity.position.x.to_bits()).hash(&mut hasher);
        (entity.position.y.to_bits()).hash(&mut hasher);
        (entity.position.z.to_bits()).hash(&mut hasher);
        // Hash health
        entity.health.current.hash(&mut hasher);
        entity.health.max.hash(&mut hasher);
        // Hash inventory
        entity.inventory.gold.hash(&mut hasher);
    }

    hasher.finish()
}

fn create_test_bundle(player_id: &str, slot: u8, world_tick: u64, ecs_blob: &[u8]) -> SaveBundleV2 {
    let mut meta = HashMap::new();
    meta.insert("test".to_string(), "data".to_string());

    SaveBundleV2 {
        schema: SAVE_SCHEMA_VERSION,
        save_id: Uuid::new_v4(),
        created_at: OffsetDateTime::now_utc(),
        player_id: player_id.to_string(),
        slot,
        world: WorldState {
            tick: world_tick,
            ecs_blob: ecs_blob.to_vec(),
            state_hash: 12345,
        },
        companions: Vec::new(),
        inventory: aw_save::PlayerInventory {
            credits: 1000,
            items: Vec::new(),
        },
        meta,
    }
}

fn create_test_replay_state(event_count: usize) -> CReplayState {
    let events = (0..event_count)
        .map(|i| astraweave_persistence_ecs::ReplayEvent {
            tick: i as u64,
            event_type: format!("event_type_{}", i % 10),
            data: vec![i as u8; 32],
        })
        .collect();

    CReplayState {
        is_replaying: true,
        current_tick: 0,
        total_ticks: event_count as u64,
        events,
    }
}

criterion_group!(
    ecs_serialization,
    benchmark_serialize_single_component,
    benchmark_serialize_entity_batch
);
criterion_group!(world_hash, benchmark_world_hash);
criterion_group!(save_load_cycle, benchmark_save_load_cycle);
criterion_group!(
    replay,
    benchmark_replay_operations,
    benchmark_replay_tick_advance
);
criterion_group!(persistence_manager, benchmark_persistence_manager_ops);
criterion_group!(scaling, benchmark_scaling_entity_count);

criterion_main!(
    ecs_serialization,
    world_hash,
    save_load_cycle,
    replay,
    persistence_manager,
    scaling
);
