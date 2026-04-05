//! # Embeddings Benchmark Suite
//!
//! Comprehensive benchmarks for the astraweave-embeddings crate covering:
//! - Embedding generation (mock client for deterministic benchmarking)
//! - Vector store operations (insert, search, remove)
//! - Distance calculations (cosine, euclidean, manhattan, dot product)
//! - Text preprocessing and chunking
//! - Batch operations and scaling
//!
//! Run with: `cargo bench -p astraweave-embeddings`

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box;

use astraweave_embeddings::EmbeddingClient;

// ============================================================================
// CORRECTNESS ASSERTION HELPERS
// ============================================================================

/// Assert that a vector has the expected dimensions
fn assert_vector_valid(vector: &[f32], expected_dims: usize) {
    assert_eq!(
        vector.len(),
        expected_dims,
        "Vector should have {} dimensions",
        expected_dims
    );
    assert!(
        vector.iter().all(|v| v.is_finite()),
        "All vector values should be finite"
    );
}

/// Assert that search results are valid
fn assert_search_results_valid(
    results: &[astraweave_embeddings::SearchResult],
    max_results: usize,
) {
    assert!(
        results.len() <= max_results,
        "Should return at most {} results",
        max_results
    );
    // Results should be sorted by distance (ascending)
    for i in 1..results.len() {
        assert!(
            results[i].distance >= results[i - 1].distance,
            "Results should be sorted by distance"
        );
    }
}

/// Assert that a stored vector is valid
fn assert_stored_vector_valid(stored: &astraweave_embeddings::StoredVector, expected_dims: usize) {
    assert!(!stored.id.is_empty(), "ID should not be empty");
    assert_eq!(
        stored.vector.len(),
        expected_dims,
        "Vector should have {} dimensions",
        expected_dims
    );
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

/// Generate a random vector of given dimensions
fn generate_vector(dims: usize, seed: u64) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut result = Vec::with_capacity(dims);
    for i in 0..dims {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        (i as u64).hash(&mut hasher);
        let hash = hasher.finish();
        // Convert hash to float in range [-1, 1]
        let value = ((hash % 20001) as f32 / 10000.0) - 1.0;
        result.push(value);
    }
    // Normalize
    let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut result {
            *v /= magnitude;
        }
    }
    result
}

/// Generate sample text for embedding
fn generate_sample_text(index: usize) -> String {
    let templates = [
        "The player encountered a mysterious stranger in the dark forest",
        "A powerful explosion rocked the ancient castle walls",
        "The merchant offered rare items at surprisingly low prices",
        "Dark clouds gathered as the battle reached its climax",
        "The hero discovered a hidden passage behind the waterfall",
        "Ancient runes glowed with an eerie blue light",
        "The village elder shared tales of forgotten kingdoms",
        "A fierce dragon guarded the treasure deep within the cave",
    ];
    format!(
        "{} - instance {}",
        templates[index % templates.len()],
        index
    )
}

/// Create a populated vector store for benchmarking
fn create_populated_store(dims: usize, count: usize) -> astraweave_embeddings::VectorStore {
    let config = astraweave_embeddings::EmbeddingConfig {
        dimensions: dims,
        max_vectors: count + 1000,
        ..Default::default()
    };
    let store = astraweave_embeddings::VectorStore::with_config(config);

    for i in 0..count {
        let vector = generate_vector(dims, i as u64);
        let text = generate_sample_text(i);
        store
            .insert(format!("vec_{}", i), vector, text)
            .expect("Insert should succeed");
    }

    store
}

// ============================================================================
// EMBEDDING CLIENT BENCHMARKS
// ============================================================================

fn bench_embedding_client(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = astraweave_embeddings::MockEmbeddingClient::new();

    let mut group = c.benchmark_group("embedding_client");

    // Single embedding generation
    group.bench_function("single_embed", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = client
                    .embed(black_box("The player approached the door"))
                    .await;
                let embedding = result.expect("Embed should succeed");
                assert_vector_valid(&embedding, 384);
                embedding
            })
        })
    });

    // Batch embedding (varying sizes)
    for batch_size in [1, 5, 10, 20, 50] {
        let texts: Vec<String> = (0..batch_size).map(generate_sample_text).collect();

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_embed", batch_size),
            &texts,
            |b, texts| {
                b.iter(|| {
                    rt.block_on(async {
                        let result = client.embed_batch(black_box(texts)).await;
                        let embeddings = result.expect("Batch embed should succeed");
                        assert_eq!(embeddings.len(), texts.len());
                        for emb in &embeddings {
                            assert_vector_valid(emb, 384);
                        }
                        embeddings
                    })
                })
            },
        );
    }

    // Model info retrieval (should be instant)
    group.bench_function("model_info", |b| {
        b.iter(|| {
            let info = client.model_info();
            assert_eq!(info.dimensions, 384);
            black_box(info)
        })
    });

    group.finish();
}

// ============================================================================
// VECTOR STORE BENCHMARKS
// ============================================================================

fn bench_vector_store_operations(c: &mut Criterion) {
    let dims = 384;
    let mut group = c.benchmark_group("vector_store_operations");

    // Insert operations
    group.bench_function("insert_single", |b| {
        let store = astraweave_embeddings::VectorStore::new(dims);
        let mut idx = 0u64;
        b.iter(|| {
            let vector = generate_vector(dims, idx);
            let id = format!("bench_{}", idx);
            let result = store.insert(black_box(id), black_box(vector), "test text".to_string());
            assert!(result.is_ok(), "Insert should succeed");
            idx += 1;
        })
    });

    // Insert with metadata
    group.bench_function("insert_with_metadata", |b| {
        let store = astraweave_embeddings::VectorStore::new(dims);
        let mut idx = 0u64;
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), "benchmark".to_string());
        metadata.insert("priority".to_string(), "high".to_string());

        b.iter(|| {
            let vector = generate_vector(dims, idx);
            let id = format!("bench_{}", idx);
            let result = store.insert_with_metadata(
                black_box(id),
                black_box(vector),
                "test text".to_string(),
                0.8,
                metadata.clone(),
            );
            assert!(result.is_ok(), "Insert with metadata should succeed");
            idx += 1;
        })
    });

    // Get by ID
    for store_size in [100, 1000, 5000] {
        let store = create_populated_store(dims, store_size);
        group.bench_with_input(
            BenchmarkId::new("get_by_id", store_size),
            &store,
            |b, store| {
                let mut idx = 0;
                b.iter(|| {
                    let id = format!("vec_{}", idx % store_size);
                    let result = store.get(black_box(&id));
                    assert!(result.is_some(), "Vector should exist");
                    let stored = result.unwrap();
                    assert_stored_vector_valid(&stored, dims);
                    idx += 1;
                    stored
                })
            },
        );
    }

    // Remove operations
    group.bench_function("remove_single", |b| {
        b.iter_batched(
            || {
                let store = astraweave_embeddings::VectorStore::new(dims);
                for i in 0..100 {
                    let vector = generate_vector(dims, i as u64);
                    store
                        .insert(format!("vec_{}", i), vector, "test".to_string())
                        .unwrap();
                }
                store
            },
            |store| {
                let result = store.remove(black_box("vec_50"));
                assert!(result.is_some(), "Remove should find vector");
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// SEARCH BENCHMARKS
// ============================================================================

fn bench_vector_search(c: &mut Criterion) {
    let dims = 384;
    let mut group = c.benchmark_group("vector_search");

    // Search with different store sizes
    for store_size in [100, 500, 1000, 2000] {
        let store = create_populated_store(dims, store_size);
        let query = generate_vector(dims, 999999);

        group.throughput(Throughput::Elements(store_size as u64));
        group.bench_with_input(
            BenchmarkId::new("brute_force_k10", store_size),
            &(store, query.clone()),
            |b, (store, query)| {
                b.iter(|| {
                    let results = store.search(black_box(query), black_box(10)).unwrap();
                    assert_search_results_valid(&results, 10);
                    results
                })
            },
        );
    }

    // Search with different k values
    let store = create_populated_store(dims, 1000);
    for k in [1, 5, 10, 20, 50] {
        let query = generate_vector(dims, 888888);

        group.bench_with_input(BenchmarkId::new("search_k_variation", k), &k, |b, &k| {
            b.iter(|| {
                let results = store.search(black_box(&query), black_box(k)).unwrap();
                assert_search_results_valid(&results, k);
                results
            })
        });
    }

    group.finish();
}

// ============================================================================
// DISTANCE CALCULATION BENCHMARKS
// ============================================================================

fn bench_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_calculations");

    // Test different dimensions
    for dims in [128, 384, 768, 1024] {
        let vec_a = generate_vector(dims, 12345);
        let vec_b = generate_vector(dims, 67890);

        // Cosine distance (inline implementation for benchmarking)
        group.bench_with_input(
            BenchmarkId::new("cosine_distance", dims),
            &(vec_a.clone(), vec_b.clone()),
            |b, (a, b_vec)| {
                b.iter(|| {
                    let dot_product: f32 = a.iter().zip(b_vec.iter()).map(|(x, y)| x * y).sum();
                    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let norm_b: f32 = b_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let distance = if norm_a == 0.0 || norm_b == 0.0 {
                        1.0
                    } else {
                        1.0 - (dot_product / (norm_a * norm_b))
                    };
                    assert!(
                        (0.0_f32..=2.0_f32).contains(&distance),
                        "Cosine distance should be in [0, 2]"
                    );
                    black_box(distance)
                })
            },
        );

        // Euclidean distance
        group.bench_with_input(
            BenchmarkId::new("euclidean_distance", dims),
            &(vec_a.clone(), vec_b.clone()),
            |b, (a, b_vec)| {
                b.iter(|| {
                    let distance: f32 = a
                        .iter()
                        .zip(b_vec.iter())
                        .map(|(x, y)| (x - y) * (x - y))
                        .sum::<f32>()
                        .sqrt();
                    assert!(distance >= 0.0, "Euclidean distance should be non-negative");
                    black_box(distance)
                })
            },
        );

        // Manhattan distance
        group.bench_with_input(
            BenchmarkId::new("manhattan_distance", dims),
            &(vec_a.clone(), vec_b.clone()),
            |b, (a, b_vec)| {
                b.iter(|| {
                    let distance: f32 =
                        a.iter().zip(b_vec.iter()).map(|(x, y)| (x - y).abs()).sum();
                    assert!(distance >= 0.0, "Manhattan distance should be non-negative");
                    black_box(distance)
                })
            },
        );

        // Dot product
        group.bench_with_input(
            BenchmarkId::new("dot_product", dims),
            &(vec_a.clone(), vec_b.clone()),
            |b, (a, b_vec)| {
                b.iter(|| {
                    let result: f32 = a.iter().zip(b_vec.iter()).map(|(x, y)| x * y).sum();
                    // Dot product of unit vectors should be in [-1, 1]
                    assert!(
                        (-1.1_f32..=1.1_f32).contains(&result),
                        "Dot product of unit vectors should be in [-1, 1]"
                    );
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// TEXT PREPROCESSING BENCHMARKS
// ============================================================================

fn bench_text_preprocessing(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_preprocessing");

    // Preprocess text of varying lengths
    for text_len in [100, 500, 1000, 5000] {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(text_len / 45);

        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("preprocess", text_len),
            &text,
            |b, text| {
                b.iter(|| {
                    let result =
                        astraweave_embeddings::TextPreprocessor::preprocess(black_box(text));
                    assert!(!result.is_empty(), "Preprocessed text should not be empty");
                    result
                })
            },
        );
    }

    // Chunking text
    for (text_len, chunk_size) in [(1000, 200), (5000, 500), (10000, 1000)] {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(text_len / 45);

        group.bench_with_input(
            BenchmarkId::new("chunk_text", format!("{}_{}", text_len, chunk_size)),
            &(text.clone(), chunk_size),
            |b, (text, chunk_size)| {
                b.iter(|| {
                    let chunks = astraweave_embeddings::TextPreprocessor::chunk_text(
                        black_box(text),
                        black_box(*chunk_size),
                        50, // overlap
                    );
                    assert!(!chunks.is_empty(), "Should produce at least one chunk");
                    chunks
                })
            },
        );
    }

    // Extract keyphrases
    let sample_text = "The ancient artifact was discovered in the mysterious dungeon beneath the castle. The brave adventurer claimed the powerful magical item.";
    group.bench_function("extract_keyphrases", |b| {
        b.iter(|| {
            let phrases =
                astraweave_embeddings::TextPreprocessor::extract_keyphrases(black_box(sample_text));
            assert!(!phrases.is_empty(), "Should extract some keyphrases");
            phrases
        })
    });

    group.finish();
}

// ============================================================================
// SERIALIZATION BENCHMARKS
// ============================================================================

fn bench_serialization(c: &mut Criterion) {
    let dims = 384;
    let mut group = c.benchmark_group("serialization");

    // Serialize stores of different sizes
    for store_size in [10, 50, 100, 500] {
        let store = create_populated_store(dims, store_size);

        group.throughput(Throughput::Elements(store_size as u64));
        group.bench_with_input(
            BenchmarkId::new("to_json", store_size),
            &store,
            |b, store| {
                b.iter(|| {
                    let json = store.to_json().expect("Serialization should succeed");
                    assert!(!json.is_empty(), "JSON should not be empty");
                    json
                })
            },
        );
    }

    // Deserialize
    for store_size in [10, 50, 100] {
        let store = create_populated_store(dims, store_size);
        let json = store.to_json().unwrap();

        group.bench_with_input(
            BenchmarkId::new("from_json", store_size),
            &json,
            |b, json| {
                b.iter(|| {
                    let restored = astraweave_embeddings::VectorStore::from_json(black_box(json))
                        .expect("Deserialization should succeed");
                    assert_eq!(restored.len(), store_size, "Should restore all vectors");
                    restored
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// BATCH AND SCALING BENCHMARKS
// ============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let dims = 384;
    let mut group = c.benchmark_group("batch_operations");

    // Batch inserts
    for batch_size in [10, 50, 100, 500] {
        let vectors: Vec<(String, Vec<f32>, String)> = (0..batch_size)
            .map(|i| {
                (
                    format!("batch_{}", i),
                    generate_vector(dims, i as u64 + 10000),
                    generate_sample_text(i),
                )
            })
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_insert", batch_size),
            &vectors,
            |b, vectors| {
                b.iter_batched(
                    || astraweave_embeddings::VectorStore::new(dims),
                    |store| {
                        for (id, vec, text) in vectors.iter() {
                            store
                                .insert(id.clone(), vec.clone(), text.clone())
                                .expect("Insert should succeed");
                        }
                        assert_eq!(store.len(), vectors.len());
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    // Prune operations
    for initial_size in [100, 500, 1000] {
        let target_size = initial_size / 2;

        group.bench_with_input(
            BenchmarkId::new(
                "prune_vectors",
                format!("{}_to_{}", initial_size, target_size),
            ),
            &(initial_size, target_size),
            |b, &(initial, target)| {
                b.iter_batched(
                    || create_populated_store(dims, initial),
                    |store| {
                        let pruned = store.prune_vectors(target).expect("Prune should succeed");
                        assert!(pruned > 0, "Should prune some vectors");
                        assert!(
                            store.len() <= target,
                            "Store size should be at or below target"
                        );
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    // Get all IDs scaling
    for store_size in [100, 500, 1000, 2000] {
        let store = create_populated_store(dims, store_size);

        group.bench_with_input(
            BenchmarkId::new("get_all_ids", store_size),
            &store,
            |b, store| {
                b.iter(|| {
                    let ids = store.get_all_ids();
                    assert_eq!(ids.len(), store_size, "Should return all IDs");
                    ids
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// MEMORY OPERATIONS BENCHMARKS
// ============================================================================

fn bench_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");

    // Create memory
    group.bench_function("create_memory", |b| {
        b.iter(|| {
            let memory = astraweave_embeddings::MemoryUtils::create_memory(
                std::hint::black_box("The player defeated the dragon".to_string()),
                std::hint::black_box(astraweave_embeddings::MemoryCategory::Combat),
                std::hint::black_box(0.9),
                std::hint::black_box(vec!["player_1".to_string(), "dragon_boss".to_string()]),
            );
            assert!(!memory.id.is_empty(), "Memory should have valid ID");
            assert_eq!(
                memory.category,
                astraweave_embeddings::MemoryCategory::Combat
            );
            memory
        })
    });

    // Create specialized memories
    group.bench_function("create_social_memory", |b| {
        b.iter(|| {
            let memory = astraweave_embeddings::MemoryUtils::create_social_memory(
                std::hint::black_box("The merchant greeted the player warmly".to_string()),
                std::hint::black_box(vec!["player_1".to_string(), "merchant_npc".to_string()]),
                std::hint::black_box(0.7), // importance
                std::hint::black_box(0.5), // valence
            );
            assert_eq!(
                memory.category,
                astraweave_embeddings::MemoryCategory::Social
            );
            memory
        })
    });

    group.bench_function("create_combat_memory", |b| {
        b.iter(|| {
            let memory = astraweave_embeddings::MemoryUtils::create_combat_memory(
                std::hint::black_box("A fierce battle ensued with the goblin horde".to_string()),
                std::hint::black_box(vec!["player_1".to_string()]),
                std::hint::black_box(0.8),
                std::hint::black_box(astraweave_embeddings::CombatOutcome::Victory),
            );
            assert_eq!(
                memory.category,
                astraweave_embeddings::MemoryCategory::Combat
            );
            memory
        })
    });

    // Use generic create_memory for exploration
    group.bench_function("create_exploration_memory", |b| {
        b.iter(|| {
            let memory = astraweave_embeddings::MemoryUtils::create_memory(
                std::hint::black_box("The player discovered a hidden cave system".to_string()),
                std::hint::black_box(astraweave_embeddings::MemoryCategory::Exploration),
                std::hint::black_box(0.6),
                std::hint::black_box(vec!["hidden_cave".to_string()]),
            );
            assert_eq!(
                memory.category,
                astraweave_embeddings::MemoryCategory::Exploration
            );
            memory
        })
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUP REGISTRATION
// ============================================================================

criterion_group!(
    benches,
    bench_embedding_client,
    bench_vector_store_operations,
    bench_vector_search,
    bench_distance_calculations,
    bench_text_preprocessing,
    bench_serialization,
    bench_batch_operations,
    bench_memory_operations,
);

criterion_main!(benches);
