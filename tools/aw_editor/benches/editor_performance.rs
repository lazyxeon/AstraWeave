//! Editor performance benchmarks for regression detection.
//!
//! These benchmarks measure CPU-side performance of critical editor subsystems.
//! They run headless (no GPU) and can execute in CI.
//!
//! Run with: `cargo bench -p aw_editor --bench editor_performance`

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

use aw_editor_lib::command::{MoveEntityCommand, UndoStack};
use aw_editor_lib::entity_manager::{EditorEntity, EntityManager};

// ================================
// 1. ENTITY MANAGER OPERATIONS
// ================================

fn bench_entity_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_manager");

    for &count in &[100, 500, 1000, 5000] {
        group.bench_with_input(
            BenchmarkId::new("add_entities", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut em = EntityManager::new();
                    for i in 0..count {
                        let entity = EditorEntity::new(i as u64, format!("Entity_{i}"));
                        em.add(entity);
                    }
                    black_box(&em);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("lookup_entity", count),
            &count,
            |b, &count| {
                let mut em = EntityManager::new();
                for i in 0..count {
                    em.add(EditorEntity::new(i as u64, format!("Entity_{i}")));
                }
                b.iter(|| {
                    for i in 0..count {
                        black_box(em.get(i as u64));
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("iterate_entities", count),
            &count,
            |b, &count| {
                let mut em = EntityManager::new();
                for i in 0..count {
                    em.add(EditorEntity::new(i as u64, format!("Entity_{i}")));
                }
                b.iter(|| {
                    let sum: f32 = em
                        .entities()
                        .values()
                        .map(|e| e.position.x + e.position.y + e.position.z)
                        .sum();
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

// ================================
// 2. UNDO/REDO PERFORMANCE
// ================================

fn bench_undo_redo(c: &mut Criterion) {
    let mut group = c.benchmark_group("undo_redo");

    group.bench_function("execute_100_commands", |b| {
        b.iter_batched(
            || {
                // Setup: create world with entity (not measured)
                let mut world = astraweave_core::World::new();
                let entity = world.spawn(
                    "Test",
                    astraweave_core::IVec2 { x: 0, y: 0 },
                    astraweave_core::Team { id: 0 },
                    0,
                    0,
                );
                (UndoStack::new(200), world, entity)
            },
            |(mut stack, mut world, entity)| {
                // Measured: execute 100 commands
                for i in 0..100 {
                    let cmd = MoveEntityCommand::new(
                        entity,
                        astraweave_core::IVec2 { x: i, y: i },
                        astraweave_core::IVec2 { x: i + 1, y: i + 1 },
                    );
                    let _ = stack.execute(cmd, &mut world, None);
                }
                black_box(&stack);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("execute_and_undo_50", |b| {
        b.iter_batched(
            || {
                // Setup: create world + execute 50 commands (not measured)
                let mut world = astraweave_core::World::new();
                let entity = world.spawn(
                    "Test",
                    astraweave_core::IVec2 { x: 0, y: 0 },
                    astraweave_core::Team { id: 0 },
                    0,
                    0,
                );
                let mut stack = UndoStack::new(200);
                for i in 0..50 {
                    let cmd = MoveEntityCommand::new(
                        entity,
                        astraweave_core::IVec2 { x: i, y: i },
                        astraweave_core::IVec2 { x: i + 1, y: i + 1 },
                    );
                    let _ = stack.execute(cmd, &mut world, None);
                }
                (stack, world)
            },
            |(mut stack, mut world)| {
                // Measured: undo all 50 commands
                for _ in 0..50 {
                    let _ = stack.undo(&mut world, None);
                }
                black_box(&stack);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ================================
// 3. TERRAIN VERTEX CONVERSION
// ================================

fn bench_terrain_vertex_conversion(c: &mut Criterion) {
    use aw_editor_lib::viewport::types::TerrainVertex;

    let mut group = c.benchmark_group("terrain_vertex");

    for &count in &[1000, 10000, 50000] {
        // Create test vertices
        let vertices: Vec<TerrainVertex> = (0..count)
            .map(|i| {
                let f = i as f32;
                TerrainVertex {
                    position: [f * 0.1, (f * 0.3).sin() * 5.0, f * 0.1],
                    normal: [0.0, 1.0, 0.0],
                    uv: [(f * 0.01) % 1.0, (f * 0.01) % 1.0],
                    biome_weights_0: [0.6, 0.3, 0.1, 0.0],
                    biome_weights_1: [0.0, 0.0, 0.0, 0.0],
                    material_ids: [0.0, 1.0, 2.0, 0.0],
                    material_weights: [0.5, 0.3, 0.2, 0.0],
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("to_engine_vertex", count),
            &vertices,
            |b, verts| {
                b.iter(|| {
                    let result: Vec<_> = verts.iter().map(|v| v.to_engine_vertex()).collect();
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

// ================================
// 4. LEVEL DOCUMENT OPERATIONS
// ================================

fn bench_level_doc(c: &mut Criterion) {
    use aw_editor_lib::level_doc::*;

    let mut group = c.benchmark_group("level_doc");

    group.bench_function("validate_complex_level", |b| {
        let mut doc = LevelDoc::default();
        doc.title = "Test Level".to_string();
        doc.biome = "forest".to_string();
        for i in 0..100 {
            doc.obstacles.push(Obstacle {
                id: format!("obs_{i}"),
                pos: [i as f32, 0.0, i as f32],
                yaw: 0.0,
                tags: vec!["solid".to_string()],
            });
        }
        for i in 0..20 {
            doc.npcs.push(NpcSpawn {
                archetype: format!("npc_{}", i % 5),
                count: 3,
                spawn: Spawn {
                    pos: [0.0, 0.0, 0.0],
                    radius: 10.0,
                },
                behavior: "patrol".to_string(),
            });
        }
        for i in 0..10 {
            doc.fate_threads.push(FateThread {
                name: format!("thread_{i}"),
                triggers: vec![Trigger::EnterArea {
                    center: [0.0, 0.0, 0.0],
                    radius: 20.0,
                }],
                ops: vec![DirectorOp::SpawnWave {
                    archetype: "enemy".to_string(),
                    count: 5,
                    scatter: 3.0,
                }],
            });
        }

        b.iter(|| {
            let issues = doc.validate();
            black_box(issues);
        });
    });

    group.bench_function("serialize_roundtrip", |b| {
        let doc = LevelDoc {
            title: "Benchmark Level".to_string(),
            biome: "forest".to_string(),
            seed: 42,
            sky: Sky {
                time_of_day: "noon".to_string(),
                weather: "clear".to_string(),
            },
            ..Default::default()
        };
        b.iter(|| {
            let serialized = toml::to_string(&doc).unwrap();
            let deserialized: LevelDoc = toml::from_str(&serialized).unwrap();
            black_box(deserialized);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_manager,
    bench_undo_redo,
    bench_terrain_vertex_conversion,
    bench_level_doc,
);
criterion_main!(benches);
