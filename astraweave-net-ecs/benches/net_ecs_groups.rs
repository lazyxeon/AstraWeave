use astraweave_ecs::World;
use astraweave_net_ecs::{
    CClientPrediction, CNetworkAuthority, CNetworkClient, EntityState, NetworkSnapshot,
};
use criterion::{criterion_group, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::hint::black_box;
use tokio::sync::mpsc;

// ==================== GROUP 1: Serialization Benchmarks ====================

fn benchmark_serialize_entity_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/entity_state");

    let state = EntityState {
        position: glam::Vec3::new(1.0, 2.0, 3.0),
        health: 100,
    };

    group.bench_function("serialize_postcard", |b| {
        b.iter(|| {
            let encoded = postcard::to_allocvec(black_box(&state)).unwrap();
            black_box(encoded)
        })
    });

    group.bench_function("deserialize_postcard", |b| {
        let encoded = postcard::to_allocvec(&state).unwrap();
        b.iter(|| {
            let decoded: EntityState = postcard::from_bytes(black_box(&encoded)).unwrap();
            black_box(decoded)
        })
    });

    group.finish();
}

fn benchmark_serialize_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/snapshot");

    for entity_count in [10, 50, 100, 500] {
        let snapshot = create_test_snapshot(entity_count);

        group.bench_with_input(
            BenchmarkId::new("postcard", entity_count),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    let encoded = postcard::to_allocvec(black_box(snapshot)).unwrap();
                    black_box(encoded)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 2: Compression Benchmarks ====================

fn benchmark_snapshot_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression/snapshot");

    for entity_count in [10, 50, 100, 500] {
        let snapshot = create_test_snapshot(entity_count);
        let serialized = postcard::to_allocvec(&snapshot).unwrap();

        group.bench_with_input(
            BenchmarkId::new("lz4_compress", entity_count),
            &serialized,
            |b, data| {
                b.iter(|| {
                    let compressed = lz4_flex::compress_prepend_size(black_box(data));
                    black_box(compressed)
                })
            },
        );

        let compressed = lz4_flex::compress_prepend_size(&serialized);

        group.bench_with_input(
            BenchmarkId::new("lz4_decompress", entity_count),
            &compressed,
            |b, data| {
                b.iter(|| {
                    let decompressed =
                        lz4_flex::decompress_size_prepended(black_box(data)).unwrap();
                    black_box(decompressed)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 3: Delta Encoding Benchmarks ====================

fn benchmark_delta_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta/encoding");

    for entity_count in [10, 50, 100, 500] {
        let base_snapshot = create_test_snapshot(entity_count);
        let mut modified_snapshot = base_snapshot.clone();

        // Modify 10% of entities
        let modified_count = (entity_count / 10).max(1);
        for i in 0..modified_count {
            if let Some(state) = modified_snapshot.entity_states.get_mut(&(i as u64)) {
                state.position.x += 1.0;
                state.health -= 10;
            }
        }

        group.bench_with_input(
            BenchmarkId::new("compute_delta", entity_count),
            &(&base_snapshot, &modified_snapshot),
            |b, (base, modified)| {
                b.iter(|| {
                    let delta = compute_delta(black_box(base), black_box(modified));
                    black_box(delta)
                })
            },
        );

        let delta = compute_delta(&base_snapshot, &modified_snapshot);

        group.bench_with_input(
            BenchmarkId::new("apply_delta", entity_count),
            &(&base_snapshot, &delta),
            |b, (base, delta)| {
                b.iter(|| {
                    let mut result = (*base).clone();
                    apply_delta(&mut result, delta);
                    black_box(result)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("serialize_delta", entity_count),
            &delta,
            |b, delta| {
                b.iter(|| {
                    let encoded = postcard::to_allocvec(black_box(delta)).unwrap();
                    black_box(encoded)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 4: Client-Server Simulation ====================

fn benchmark_client_input_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_server/input_processing");

    for client_count in [1, 10, 50, 100] {
        let mut world = create_client_world(client_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(client_count),
            &client_count,
            |b, _| {
                b.iter(|| {
                    astraweave_net_ecs::client_input_system(black_box(&mut world));
                })
            },
        );
    }

    group.finish();
}

fn benchmark_client_reconciliation(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_server/reconciliation");

    for client_count in [1, 10, 50, 100] {
        let mut world = create_client_world(client_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(client_count),
            &client_count,
            |b, _| {
                b.iter(|| {
                    astraweave_net_ecs::client_reconciliation_system(black_box(&mut world));
                })
            },
        );
    }

    group.finish();
}

fn benchmark_server_snapshot_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_server/snapshot_generation");

    for client_count in [1, 10, 50, 100] {
        let mut world = create_server_world(client_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(client_count),
            &client_count,
            |b, _| {
                b.iter(|| {
                    astraweave_net_ecs::server_snapshot_system(black_box(&mut world));
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 5: Full Pipeline Benchmarks ====================

fn benchmark_full_sync_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline/sync_cycle");

    for entity_count in [10, 50, 100, 500] {
        let snapshot = create_test_snapshot(entity_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    // Full cycle: serialize → compress → decompress → deserialize
                    let serialized = postcard::to_allocvec(black_box(snapshot)).unwrap();
                    let compressed = lz4_flex::compress_prepend_size(&serialized);
                    let decompressed = lz4_flex::decompress_size_prepended(&compressed).unwrap();
                    let decoded: NetworkSnapshot = postcard::from_bytes(&decompressed).unwrap();
                    black_box(decoded)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_full_delta_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline/delta_cycle");

    for entity_count in [10, 50, 100, 500] {
        let base_snapshot = create_test_snapshot(entity_count);
        let mut modified_snapshot = base_snapshot.clone();

        // Modify 10% of entities
        let modified_count = (entity_count / 10).max(1);
        for i in 0..modified_count {
            if let Some(state) = modified_snapshot.entity_states.get_mut(&(i as u64)) {
                state.position.x += 1.0;
            }
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &(&base_snapshot, &modified_snapshot),
            |b, (base, modified)| {
                b.iter(|| {
                    // Full delta cycle: compute → serialize → compress → decompress → deserialize → apply
                    let delta = compute_delta(black_box(base), black_box(modified));
                    let serialized = postcard::to_allocvec(&delta).unwrap();
                    let compressed = lz4_flex::compress_prepend_size(&serialized);
                    let decompressed = lz4_flex::decompress_size_prepended(&compressed).unwrap();
                    let decoded: NetworkSnapshot = postcard::from_bytes(&decompressed).unwrap();
                    let mut result = (*base).clone();
                    apply_delta(&mut result, &decoded);
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

// ==================== GROUP 6: Bandwidth Analysis ====================

fn benchmark_snapshot_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("bandwidth/snapshot_size");

    for entity_count in [10, 50, 100, 500, 1000] {
        let snapshot = create_test_snapshot(entity_count);

        group.bench_with_input(
            BenchmarkId::new("uncompressed", entity_count),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    let serialized = postcard::to_allocvec(black_box(snapshot)).unwrap();
                    black_box(serialized.len())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compressed", entity_count),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    let serialized = postcard::to_allocvec(black_box(snapshot)).unwrap();
                    let compressed = lz4_flex::compress_prepend_size(&serialized);
                    black_box(compressed.len())
                })
            },
        );
    }

    group.finish();
}

fn benchmark_delta_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("bandwidth/delta_size");

    for entity_count in [10, 50, 100, 500, 1000] {
        let base_snapshot = create_test_snapshot(entity_count);
        let mut modified_snapshot = base_snapshot.clone();

        // Modify 10% of entities
        let modified_count = (entity_count / 10).max(1);
        for i in 0..modified_count {
            if let Some(state) = modified_snapshot.entity_states.get_mut(&(i as u64)) {
                state.position.x += 1.0;
            }
        }

        let delta = compute_delta(&base_snapshot, &modified_snapshot);

        group.bench_with_input(
            BenchmarkId::new("uncompressed", entity_count),
            &delta,
            |b, delta| {
                b.iter(|| {
                    let serialized = postcard::to_allocvec(black_box(delta)).unwrap();
                    black_box(serialized.len())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("compressed", entity_count),
            &delta,
            |b, delta| {
                b.iter(|| {
                    let serialized = postcard::to_allocvec(black_box(delta)).unwrap();
                    let compressed = lz4_flex::compress_prepend_size(&serialized);
                    black_box(compressed.len())
                })
            },
        );
    }

    group.finish();
}

// ==================== Helper Functions ====================

fn create_test_snapshot(entity_count: usize) -> NetworkSnapshot {
    let mut entity_states = HashMap::new();

    for i in 0..entity_count {
        entity_states.insert(
            i as u64,
            EntityState {
                position: glam::Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0),
                health: 100 - (i % 100) as i32,
            },
        );
    }

    NetworkSnapshot {
        server_tick: 12345,
        entity_states,
    }
}

fn create_client_world(client_count: usize) -> World {
    let mut world = World::new();

    for i in 0..client_count {
        let entity = world.spawn();

        world.insert(
            entity,
            CNetworkClient {
                player_id: format!("client_{}", i),
                last_acknowledged_input: 0,
                pending_inputs: Vec::new(),
            },
        );

        world.insert(
            entity,
            CClientPrediction {
                predicted_position: glam::Vec3::ZERO,
                prediction_error: glam::Vec3::ZERO,
            },
        );
    }

    world
}

fn create_server_world(client_count: usize) -> World {
    let mut world = World::new();
    let entity = world.spawn();

    let mut connected_clients = HashMap::new();

    for i in 0..client_count {
        let (tx, _rx) = mpsc::unbounded_channel();
        connected_clients.insert(format!("client_{}", i), tx);
    }

    world.insert(
        entity,
        CNetworkAuthority {
            authoritative_tick: 0,
            connected_clients,
        },
    );

    world
}

fn compute_delta(base: &NetworkSnapshot, modified: &NetworkSnapshot) -> NetworkSnapshot {
    let mut delta_states = HashMap::new();

    // Only include changed entities
    for (entity_id, modified_state) in &modified.entity_states {
        if let Some(base_state) = base.entity_states.get(entity_id) {
            if base_state.position != modified_state.position
                || base_state.health != modified_state.health
            {
                delta_states.insert(*entity_id, modified_state.clone());
            }
        } else {
            // New entity
            delta_states.insert(*entity_id, modified_state.clone());
        }
    }

    NetworkSnapshot {
        server_tick: modified.server_tick,
        entity_states: delta_states,
    }
}

fn apply_delta(base: &mut NetworkSnapshot, delta: &NetworkSnapshot) {
    for (entity_id, delta_state) in &delta.entity_states {
        base.entity_states.insert(*entity_id, delta_state.clone());
    }
    base.server_tick = delta.server_tick;
}

criterion_group!(
    serialization,
    benchmark_serialize_entity_state,
    benchmark_serialize_snapshot
);
criterion_group!(compression, benchmark_snapshot_compression);
criterion_group!(delta, benchmark_delta_encoding);
criterion_group!(
    client_server,
    benchmark_client_input_processing,
    benchmark_client_reconciliation,
    benchmark_server_snapshot_generation
);
criterion_group!(
    full_pipeline,
    benchmark_full_sync_cycle,
    benchmark_full_delta_cycle
);
criterion_group!(bandwidth, benchmark_snapshot_size, benchmark_delta_size);
