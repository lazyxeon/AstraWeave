use astraweave_memory::{
    Memory, MemoryContent, MemoryManager, MemoryMetadata, MemorySource, MemoryType,
    SpatialTemporalContext,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

/// Helper to create a basic memory
fn create_test_memory(id: &str) -> Memory {
    Memory {
        id: id.to_string(),
        memory_type: MemoryType::Working,
        content: MemoryContent {
            text: format!("Test memory content {}", id),
            data: serde_json::json!({}),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: vec![],
                related_events: vec![],
            },
        },
        metadata: MemoryMetadata {
            created_at: time::OffsetDateTime::now_utc(),
            last_accessed: time::OffsetDateTime::now_utc(),
            access_count: 0,
            importance: 0.5,
            confidence: 0.8,
            source: MemorySource::DirectExperience,
            tags: vec![],
            permanent: false,
            strength: 1.0,
            decay_factor: 0.1,
        },
        associations: vec![],
        embedding: None,
    }
}

/// Benchmark memory creation (heap allocation)
fn bench_memory_creation(c: &mut Criterion) {
    c.bench_function("memory_creation", |b| {
        b.iter(|| {
            let memory = create_test_memory(black_box("test_id"));
            black_box(memory)
        })
    });
}

/// Benchmark memory storage (HashMap insertion)
fn bench_memory_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_storage");

    // Limit to 10, 25, 50 to stay within Working memory capacity (default: 50)
    for count in [10, 25, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_with_setup(
                || {
                    let manager = MemoryManager::new();
                    let memories: Vec<Memory> = (0..count)
                        .map(|i| create_test_memory(&format!("memory_{}", i)))
                        .collect();
                    (manager, memories)
                },
                |(mut manager, memories)| {
                    for memory in memories {
                        black_box(manager.store_memory(black_box(memory)).unwrap());
                    }
                    black_box(manager)
                },
            )
        });
    }
    group.finish();
}

/// Benchmark memory retrieval by ID (HashMap lookup)
fn bench_memory_retrieval(c: &mut Criterion) {
    c.bench_function("memory_retrieval_by_id", |b| {
        b.iter_with_setup(
            || {
                let mut manager = MemoryManager::new();
                // Pre-populate with 50 memories (Working memory capacity limit)
                for i in 0..50 {
                    let memory = create_test_memory(&format!("memory_{}", i));
                    manager.store_memory(memory).unwrap();
                }
                manager
            },
            |mut manager| {
                let exists = manager.get_memory(black_box("memory_25")).is_some();
                black_box(exists)
            },
        )
    });
}

/// Benchmark memory access tracking (updates metadata)
fn bench_memory_access_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_access_tracking");

    // Limit to 10, 25, 50 to stay within Working memory capacity (default: 50)
    for count in [10, 25, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_with_setup(
                || {
                    let mut manager = MemoryManager::new();
                    for i in 0..count {
                        let memory = create_test_memory(&format!("memory_{}", i));
                        manager.store_memory(memory).unwrap();
                    }
                    manager
                },
                |mut manager| {
                    for i in 0..count {
                        manager.get_memory(&format!("memory_{}", i));
                    }
                    black_box(manager)
                },
            )
        });
    }
    group.finish();
}

/// Benchmark memory updates (importance recalculation)
fn bench_memory_updates(c: &mut Criterion) {
    c.bench_function("memory_importance_update", |b| {
        b.iter_with_setup(
            || {
                let mut manager = MemoryManager::new();
                // Pre-populate with 50 memories (Working memory capacity limit)
                for i in 0..50 {
                    let mut memory = create_test_memory(&format!("memory_{}", i));
                    memory.metadata.importance = 0.5;
                    manager.store_memory(memory).unwrap();
                }
                manager
            },
            |mut manager| {
                if let Some(memory) = manager.get_memory(black_box("memory_25")) {
                    memory.metadata.importance = black_box(0.8);
                    memory.metadata.access_count += 1;
                    black_box(&memory);
                }
                black_box(manager)
            },
        )
    });
}

criterion_group!(
    benches,
    bench_memory_creation,
    bench_memory_storage,
    bench_memory_retrieval,
    bench_memory_access_tracking,
    bench_memory_updates,
);
criterion_main!(benches);
