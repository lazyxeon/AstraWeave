use astraweave_embeddings::{Memory, MemoryCategory};
use astraweave_rag::{
    ConsolidationConfig, DiversityConfig, ForgettingConfig, InjectionConfig, PerformanceConfig,
    RagConfig, RetrievalConfig, RetrievalEngine, RetrievalQuery,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::hint::black_box;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test memory
fn create_test_memory(id: &str, text: &str, category: MemoryCategory, importance: f32) -> Memory {
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category,
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        importance,
        valence: 0.0,
        entities: vec![],
        context: HashMap::new(),
    }
}

/// Create a batch of test memories
fn create_test_memories(count: usize) -> Vec<Memory> {
    let mut memories = Vec::new();

    for i in 0..count {
        let category = match i % 6 {
            0 => MemoryCategory::Social,
            1 => MemoryCategory::Combat,
            2 => MemoryCategory::Exploration,
            3 => MemoryCategory::Gameplay,
            4 => MemoryCategory::Quest,
            _ => MemoryCategory::Dialogue,
        };

        let text = format!(
            "Test memory {} about {} event with details",
            i,
            match category {
                MemoryCategory::Social => "social",
                MemoryCategory::Combat => "combat",
                MemoryCategory::Exploration => "exploration",
                MemoryCategory::Gameplay => "gameplay",
                MemoryCategory::Quest => "quest",
                MemoryCategory::Dialogue => "dialogue",
                _ => "other",
            }
        );

        memories.push(create_test_memory(
            &format!("mem_{}", i),
            &text,
            category,
            0.5 + (i % 5) as f32 * 0.1,
        ));
    }

    memories
}

/// Create a test retrieval query
fn create_test_query(
    text: &str,
    categories: Vec<MemoryCategory>,
    limit: Option<usize>,
) -> RetrievalQuery {
    RetrievalQuery {
        text: text.to_string(),
        categories,
        filters: HashMap::new(),
        limit,
    }
}

// ============================================================================
// Benchmark 1: Memory Creation
// ============================================================================

fn bench_memory_creation(c: &mut Criterion) {
    c.bench_function("memory_creation", |b| {
        b.iter(|| {
            let memory = create_test_memory(
                "test_id",
                "Test memory with comprehensive content",
                MemoryCategory::Social,
                0.75,
            );
            black_box(memory)
        })
    });
}

fn bench_memory_batch_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_batch_creation");

    for count in [10, 50, 100, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let memories = create_test_memories(count);
                black_box(memories)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark 2: Retrieval Engine Operations
// ============================================================================

fn bench_retrieval_engine_creation(c: &mut Criterion) {
    c.bench_function("retrieval_engine_creation", |b| {
        b.iter(|| {
            let config = RetrievalConfig::default();
            let engine = RetrievalEngine::new(config);
            black_box(engine)
        })
    });
}

fn bench_retrieval_simple_search(c: &mut Criterion) {
    let config = RetrievalConfig::default();
    let engine = RetrievalEngine::new(config);
    let memories = create_test_memories(100);
    let query = create_test_query("combat event", vec![MemoryCategory::Combat], Some(10));

    c.bench_function("retrieval_simple_search", |b| {
        b.iter(|| {
            let results = engine.search(&query, &memories).unwrap();
            black_box(results)
        })
    });
}

fn bench_retrieval_search_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval_search_scaling");
    let config = RetrievalConfig::default();
    let engine = RetrievalEngine::new(config);

    for memory_count in [50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(memory_count),
            &memory_count,
            |b, &memory_count| {
                b.iter_with_setup(
                    || create_test_memories(memory_count),
                    |memories| {
                        let query = create_test_query(
                            "gameplay event details",
                            vec![MemoryCategory::Gameplay],
                            Some(10),
                        );
                        let results = engine.search(&query, &memories).unwrap();
                        black_box(results)
                    },
                )
            },
        );
    }

    group.finish();
}

fn bench_retrieval_category_filtering(c: &mut Criterion) {
    let config = RetrievalConfig::default();
    let engine = RetrievalEngine::new(config);
    let memories = create_test_memories(100);

    c.bench_function("retrieval_category_filtering", |b| {
        b.iter(|| {
            let query = create_test_query(
                "event",
                vec![MemoryCategory::Social, MemoryCategory::Combat],
                None,
            );
            let results = engine.search(&query, &memories).unwrap();
            black_box(results)
        })
    });
}

// ============================================================================
// Benchmark 3: Retrieval Query Operations
// ============================================================================

fn bench_query_creation_simple(c: &mut Criterion) {
    c.bench_function("query_creation_simple", |b| {
        b.iter(|| {
            let query = create_test_query("simple query", vec![], None);
            black_box(query)
        })
    });
}

fn bench_query_creation_complex(c: &mut Criterion) {
    c.bench_function("query_creation_complex", |b| {
        b.iter(|| {
            let query = RetrievalQuery {
                text: "complex query with detailed parameters".to_string(),
                categories: vec![
                    MemoryCategory::Social,
                    MemoryCategory::Combat,
                    MemoryCategory::Exploration,
                ],
                filters: {
                    let mut filters = HashMap::new();
                    filters.insert("importance".to_string(), "high".to_string());
                    filters.insert("valence".to_string(), "positive".to_string());
                    filters
                },
                limit: Some(20),
            };
            black_box(query)
        })
    });
}

// ============================================================================
// Benchmark 4: RAG Configuration
// ============================================================================

fn bench_rag_config_creation(c: &mut Criterion) {
    c.bench_function("rag_config_creation", |b| {
        b.iter(|| {
            let config = RagConfig::default();
            black_box(config)
        })
    });
}

fn bench_rag_config_custom(c: &mut Criterion) {
    c.bench_function("rag_config_custom", |b| {
        b.iter(|| {
            let config = RagConfig {
                max_retrieval_count: 20,
                min_similarity_score: 0.4,
                consolidation: ConsolidationConfig::default(),
                forgetting: ForgettingConfig::default(),
                injection: InjectionConfig::default(),
                diversity: DiversityConfig::default(),
                performance: PerformanceConfig::default(),
            };
            black_box(config)
        })
    });
}

// ============================================================================
// Benchmark 5: Memory Cloning and Serialization
// ============================================================================

fn bench_memory_clone(c: &mut Criterion) {
    let memory = create_test_memory(
        "test_memory",
        "Complex memory with extensive content and metadata",
        MemoryCategory::Social,
        0.8,
    );

    c.bench_function("memory_clone", |b| {
        b.iter(|| {
            let cloned = memory.clone();
            black_box(cloned)
        })
    });
}

fn bench_memory_batch_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_batch_clone");

    for count in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter_with_setup(
                || create_test_memories(count),
                |memories| {
                    let cloned = memories.to_vec();
                    black_box(cloned)
                },
            )
        });
    }

    group.finish();
}

fn bench_memory_serialize_json(c: &mut Criterion) {
    let memory = create_test_memory(
        "test_memory",
        "Memory for JSON serialization benchmarking",
        MemoryCategory::Gameplay,
        0.6,
    );

    c.bench_function("memory_serialize_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&memory).unwrap();
            black_box(json)
        })
    });
}

fn bench_memory_deserialize_json(c: &mut Criterion) {
    let memory = create_test_memory(
        "test_memory",
        "Memory for JSON deserialization benchmarking",
        MemoryCategory::Combat,
        0.7,
    );
    let json = serde_json::to_string(&memory).unwrap();

    c.bench_function("memory_deserialize_json", |b| {
        b.iter(|| {
            let deserialized: Memory = serde_json::from_str(&json).unwrap();
            black_box(deserialized)
        })
    });
}

// ============================================================================
// Benchmark 6: Similarity Calculation
// ============================================================================

fn bench_similarity_calculation(c: &mut Criterion) {
    let config = RetrievalConfig::default();
    let engine = RetrievalEngine::new(config);

    c.bench_function("similarity_calculation", |b| {
        b.iter(|| {
            // Access private method through search (simplified benchmark)
            let query = create_test_query("test query words", vec![], Some(1));
            let memories = vec![create_test_memory(
                "1",
                "test content with query words",
                MemoryCategory::Social,
                0.5,
            )];
            let results = engine.search(&query, &memories).unwrap();
            black_box(results)
        })
    });
}

// ============================================================================
// Benchmark 7: Result Ranking and Sorting
// ============================================================================

fn bench_result_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_ranking");
    let config = RetrievalConfig {
        similarity_threshold: 0.0, // Include all results for ranking
        ..Default::default()
    };
    let engine = RetrievalEngine::new(config);

    for memory_count in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(memory_count),
            &memory_count,
            |b, &memory_count| {
                b.iter_with_setup(
                    || create_test_memories(memory_count),
                    |memories| {
                        let query = create_test_query("event details", vec![], Some(10));
                        let results = engine.search(&query, &memories).unwrap();
                        black_box(results)
                    },
                )
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    benches,
    bench_memory_creation,
    bench_memory_batch_creation,
    bench_retrieval_engine_creation,
    bench_retrieval_simple_search,
    bench_retrieval_search_scaling,
    bench_retrieval_category_filtering,
    bench_query_creation_simple,
    bench_query_creation_complex,
    bench_rag_config_creation,
    bench_rag_config_custom,
    bench_memory_clone,
    bench_memory_batch_clone,
    bench_memory_serialize_json,
    bench_memory_deserialize_json,
    bench_similarity_calculation,
    bench_result_ranking,
);

criterion_main!(benches);
