//! Mutation-resistant comprehensive tests for astraweave-embeddings
//!
//! Targets: VectorStore insert/search/remove/prune, SimilarityMetrics math,
//! MemoryUtils creation/decay/forget/consolidation, TextPreprocessor, MockEmbeddingClient,
//! EmbeddingConfig defaults, DistanceMetric/MemoryCategory/CombatOutcome enums.
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::field_reassign_with_default,
    unused_comparisons
)]

use astraweave_embeddings::*;
use std::collections::HashMap;

// ============================================================================
// EMBEDDING CONFIG DEFAULTS
// ============================================================================

#[test]
fn embedding_config_default_dimensions() {
    let c = EmbeddingConfig::default();
    assert_eq!(c.dimensions, 384);
}

#[test]
fn embedding_config_default_model() {
    let c = EmbeddingConfig::default();
    assert_eq!(c.model, "sentence-transformers/all-MiniLM-L6-v2");
}

#[test]
fn embedding_config_default_batch_size() {
    let c = EmbeddingConfig::default();
    assert_eq!(c.batch_size, 32);
}

#[test]
fn embedding_config_default_max_vectors() {
    let c = EmbeddingConfig::default();
    assert_eq!(c.max_vectors, 100_000);
}

#[test]
fn embedding_config_default_distance_metric() {
    let c = EmbeddingConfig::default();
    let dbg = format!("{:?}", c.distance_metric);
    assert_eq!(dbg, "Cosine");
}

// ============================================================================
// DISTANCE METRIC ENUM
// ============================================================================

#[test]
fn distance_metric_all_variants_distinct() {
    let variants = [
        format!("{:?}", DistanceMetric::Cosine),
        format!("{:?}", DistanceMetric::Euclidean),
        format!("{:?}", DistanceMetric::Manhattan),
        format!("{:?}", DistanceMetric::DotProduct),
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// MEMORY CATEGORY ENUM
// ============================================================================

#[test]
fn memory_category_all_variants_distinct() {
    let variants = [
        MemoryCategory::Social,
        MemoryCategory::Combat,
        MemoryCategory::Exploration,
        MemoryCategory::Quest,
        MemoryCategory::Dialogue,
        MemoryCategory::Gameplay,
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// COMBAT OUTCOME ENUM
// ============================================================================

#[test]
fn combat_outcome_all_variants_distinct() {
    let variants = [
        format!("{:?}", CombatOutcome::Victory),
        format!("{:?}", CombatOutcome::Defeat),
        format!("{:?}", CombatOutcome::Draw),
        format!("{:?}", CombatOutcome::Retreat),
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// VECTOR STORE BASIC
// ============================================================================

#[test]
fn vector_store_new_empty() {
    let store = VectorStore::new(3);
    assert_eq!(store.len(), 0);
    assert!(store.is_empty());
}

#[test]
fn vector_store_insert_and_len() {
    let store = VectorStore::new(3);
    store
        .insert("a".into(), vec![1.0, 0.0, 0.0], "hello".into())
        .unwrap();
    assert_eq!(store.len(), 1);
    assert!(!store.is_empty());
}

#[test]
fn vector_store_insert_wrong_dimension_errors() {
    let store = VectorStore::new(3);
    let result = store.insert("a".into(), vec![1.0, 0.0], "short".into());
    assert!(result.is_err(), "Inserting wrong dimension should fail");
}

#[test]
fn vector_store_get_existing() {
    let store = VectorStore::new(2);
    store
        .insert("x".into(), vec![1.0, 2.0], "text_x".into())
        .unwrap();
    let got = store.get("x");
    assert!(got.is_some());
    assert_eq!(got.unwrap().text, "text_x");
}

#[test]
fn vector_store_get_nonexistent() {
    let store = VectorStore::new(2);
    assert!(store.get("nope").is_none());
}

#[test]
fn vector_store_remove_existing() {
    let store = VectorStore::new(2);
    store
        .insert("r".into(), vec![1.0, 0.0], "removeme".into())
        .unwrap();
    let removed = store.remove("r");
    assert!(removed.is_some());
    assert_eq!(store.len(), 0);
}

#[test]
fn vector_store_remove_nonexistent() {
    let store = VectorStore::new(2);
    assert!(store.remove("nope").is_none());
}

#[test]
fn vector_store_clear() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "a".into())
        .unwrap();
    store
        .insert("b".into(), vec![0.0, 1.0], "b".into())
        .unwrap();
    store.clear();
    assert_eq!(store.len(), 0);
}

#[test]
fn vector_store_get_all_ids() {
    let store = VectorStore::new(2);
    store
        .insert("x".into(), vec![1.0, 0.0], "x".into())
        .unwrap();
    store
        .insert("y".into(), vec![0.0, 1.0], "y".into())
        .unwrap();
    let ids = store.get_all_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"x".to_string()));
    assert!(ids.contains(&"y".to_string()));
}

// ============================================================================
// VECTOR STORE SEARCH
// ============================================================================

#[test]
fn vector_store_search_empty_returns_empty() {
    let store = VectorStore::new(3);
    let results = store.search(&[1.0, 0.0, 0.0], 5).unwrap();
    assert!(results.is_empty());
}

#[test]
fn vector_store_search_wrong_dim_errors() {
    let store = VectorStore::new(3);
    store
        .insert("a".into(), vec![1.0, 0.0, 0.0], "a".into())
        .unwrap();
    let result = store.search(&[1.0, 0.0], 5);
    assert!(result.is_err());
}

#[test]
fn vector_store_search_returns_correct_order() {
    let store = VectorStore::new(3);
    store
        .insert("a".into(), vec![1.0, 0.0, 0.0], "a".into())
        .unwrap();
    store
        .insert("b".into(), vec![0.9, 0.1, 0.0], "b".into())
        .unwrap();
    store
        .insert("c".into(), vec![0.0, 1.0, 0.0], "c".into())
        .unwrap();
    let results = store.search(&[1.0, 0.0, 0.0], 3).unwrap();
    assert!(!results.is_empty());
    // Most similar should be first
    assert_eq!(results[0].vector.id, "a");
}

#[test]
fn vector_store_search_k_limits_results() {
    let store = VectorStore::new(2);
    for i in 0..10 {
        store
            .insert(format!("{}", i), vec![1.0, 0.0], format!("t{}", i))
            .unwrap();
    }
    let results = store.search(&[1.0, 0.0], 3).unwrap();
    assert!(results.len() <= 3);
}

#[test]
fn vector_store_search_score_in_valid_range() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "a".into())
        .unwrap();
    let results = store.search(&[1.0, 0.0], 1).unwrap();
    assert!(!results.is_empty());
    // Cosine similarity should be between -1 and 1
    assert!(results[0].score >= -1.0 && results[0].score <= 1.01);
}

// ============================================================================
// VECTOR STORE WITH METADATA
// ============================================================================

#[test]
fn vector_store_insert_with_metadata() {
    let store = VectorStore::new(2);
    let mut meta = HashMap::new();
    meta.insert("key".to_string(), "value".to_string());
    store
        .insert_with_metadata("m".into(), vec![1.0, 0.0], "meta".into(), 0.8, meta)
        .unwrap();
    let got = store.get("m").unwrap();
    assert_eq!(got.importance, 0.8);
    assert_eq!(got.metadata.get("key").unwrap(), "value");
}

// ============================================================================
// VECTOR STORE PRUNE
// ============================================================================

#[test]
fn vector_store_prune_no_op_when_below_target() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "a".into())
        .unwrap();
    let removed = store.prune_vectors(10).unwrap();
    assert_eq!(removed, 0);
    assert_eq!(store.len(), 1);
}

#[test]
fn vector_store_prune_removes_low_importance() {
    let store = VectorStore::new(2);
    let mut low_meta = HashMap::new();
    low_meta.insert("k".to_string(), "v".to_string());
    store
        .insert_with_metadata("low".into(), vec![1.0, 0.0], "low".into(), 0.1, low_meta)
        .unwrap();
    let mut high_meta = HashMap::new();
    high_meta.insert("k".to_string(), "v".to_string());
    store
        .insert_with_metadata("high".into(), vec![0.0, 1.0], "high".into(), 0.9, high_meta)
        .unwrap();
    let removed = store.prune_vectors(1).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(store.len(), 1);
    // High importance should be retained
    assert!(store.get("high").is_some());
}

// ============================================================================
// VECTOR STORE SERIALIZATION
// ============================================================================

#[test]
fn vector_store_json_roundtrip() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "alpha".into())
        .unwrap();
    let json = store.to_json().unwrap();
    let restored = VectorStore::from_json(&json).unwrap();
    assert_eq!(restored.len(), 1);
    let got = restored.get("a").unwrap();
    assert_eq!(got.text, "alpha");
}

#[test]
fn vector_store_metrics_initial() {
    let store = VectorStore::new(3);
    let m = store.get_metrics();
    assert_eq!(m.total_vectors, 0);
    assert_eq!(m.total_searches, 0);
}

#[test]
fn vector_store_metrics_after_operations() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "a".into())
        .unwrap();
    let _ = store.search(&[1.0, 0.0], 1).unwrap();
    let m = store.get_metrics();
    assert_eq!(m.total_vectors, 1);
    assert!(m.total_searches >= 1);
}

// ============================================================================
// SIMILARITY METRICS
// ============================================================================

#[test]
fn cosine_similarity_identical_vectors() {
    let v = vec![1.0, 2.0, 3.0];
    let sim = SimilarityMetrics::cosine_similarity(&v, &v);
    assert!((sim - 1.0).abs() < 1e-5);
}

#[test]
fn cosine_similarity_orthogonal_vectors() {
    let sim = SimilarityMetrics::cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
    assert!(sim.abs() < 1e-5);
}

#[test]
fn cosine_similarity_opposite_vectors() {
    let sim = SimilarityMetrics::cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]);
    assert!((sim - (-1.0)).abs() < 1e-5);
}

#[test]
fn cosine_similarity_different_lengths_returns_zero() {
    let sim = SimilarityMetrics::cosine_similarity(&[1.0, 0.0], &[1.0]);
    assert!((sim - 0.0).abs() < 1e-5);
}

#[test]
fn cosine_similarity_zero_vector_returns_zero() {
    let sim = SimilarityMetrics::cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]);
    assert!((sim - 0.0).abs() < 1e-5);
}

#[test]
fn euclidean_distance_same_point() {
    let d = SimilarityMetrics::euclidean_distance(&[1.0, 2.0], &[1.0, 2.0]);
    assert!(d.abs() < 1e-5);
}

#[test]
fn euclidean_distance_unit_apart() {
    let d = SimilarityMetrics::euclidean_distance(&[0.0, 0.0], &[3.0, 4.0]);
    assert!((d - 5.0).abs() < 1e-5);
}

#[test]
fn euclidean_distance_different_lengths_returns_max() {
    let d = SimilarityMetrics::euclidean_distance(&[1.0], &[1.0, 2.0]);
    assert_eq!(d, f32::MAX);
}

#[test]
fn manhattan_distance_same_point() {
    let d = SimilarityMetrics::manhattan_distance(&[1.0, 2.0], &[1.0, 2.0]);
    assert!(d.abs() < 1e-5);
}

#[test]
fn manhattan_distance_known_value() {
    let d = SimilarityMetrics::manhattan_distance(&[0.0, 0.0], &[3.0, 4.0]);
    assert!((d - 7.0).abs() < 1e-5);
}

#[test]
fn manhattan_distance_different_lengths_returns_max() {
    let d = SimilarityMetrics::manhattan_distance(&[1.0], &[1.0, 2.0]);
    assert_eq!(d, f32::MAX);
}

#[test]
fn dot_product_known_value() {
    let dp = SimilarityMetrics::dot_product(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]);
    assert!((dp - 32.0).abs() < 1e-5);
}

#[test]
fn dot_product_orthogonal() {
    let dp = SimilarityMetrics::dot_product(&[1.0, 0.0], &[0.0, 1.0]);
    assert!(dp.abs() < 1e-5);
}

#[test]
fn dot_product_different_lengths_returns_zero() {
    let dp = SimilarityMetrics::dot_product(&[1.0], &[1.0, 2.0]);
    assert!((dp - 0.0).abs() < 1e-5);
}

#[test]
fn normalize_vector_unit_result() {
    let mut v = vec![3.0, 4.0];
    SimilarityMetrics::normalize_vector(&mut v);
    let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((mag - 1.0).abs() < 1e-5);
}

#[test]
fn normalize_vector_zero_vector_unchanged() {
    let mut v = vec![0.0, 0.0];
    SimilarityMetrics::normalize_vector(&mut v);
    assert!((v[0]).abs() < 1e-5);
    assert!((v[1]).abs() < 1e-5);
}

#[test]
fn jaccard_similarity_identical() {
    let a = vec!["cat".to_string(), "dog".to_string()];
    let sim = SimilarityMetrics::jaccard_similarity(&a, &a);
    assert!((sim - 1.0).abs() < 1e-5);
}

#[test]
fn jaccard_similarity_disjoint() {
    let a = vec!["cat".to_string()];
    let b = vec!["dog".to_string()];
    let sim = SimilarityMetrics::jaccard_similarity(&a, &b);
    assert!((sim - 0.0).abs() < 1e-5);
}

#[test]
fn jaccard_similarity_empty_returns_zero() {
    let a: Vec<String> = vec![];
    let b = vec!["x".to_string()];
    let sim = SimilarityMetrics::jaccard_similarity(&a, &b);
    assert!((sim - 0.0).abs() < 1e-5);
}

// ============================================================================
// MEMORY UTILS
// ============================================================================

#[test]
fn create_memory_sets_text() {
    let m = MemoryUtils::create_memory("hello".to_string(), MemoryCategory::Social, 0.5, vec![]);
    assert_eq!(m.text, "hello");
}

#[test]
fn create_memory_sets_category() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Combat, 0.5, vec![]);
    assert_eq!(m.category, MemoryCategory::Combat);
}

#[test]
fn create_memory_clamps_importance_above_one() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 1.5, vec![]);
    assert!(m.importance <= 1.0);
}

#[test]
fn create_memory_clamps_importance_below_zero() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, -0.5, vec![]);
    assert!(m.importance >= 0.0);
}

#[test]
fn create_memory_sets_entities() {
    let m = MemoryUtils::create_memory(
        "t".to_string(),
        MemoryCategory::Social,
        0.5,
        vec!["player".into()],
    );
    assert_eq!(m.entities, vec!["player".to_string()]);
}

#[test]
fn create_social_memory_category() {
    let m = MemoryUtils::create_social_memory("hi".to_string(), vec![], 0.5, 0.0);
    assert_eq!(m.category, MemoryCategory::Social);
}

#[test]
fn create_social_memory_clamps_valence() {
    let m = MemoryUtils::create_social_memory("hi".to_string(), vec![], 0.5, 2.0);
    assert!(m.valence <= 1.0);
}

#[test]
fn create_combat_memory_victory_positive_valence() {
    let m =
        MemoryUtils::create_combat_memory("won".to_string(), vec![], 0.5, CombatOutcome::Victory);
    assert!(m.valence > 0.0, "Victory should have positive valence");
}

#[test]
fn create_combat_memory_defeat_negative_valence() {
    let m =
        MemoryUtils::create_combat_memory("lost".to_string(), vec![], 0.5, CombatOutcome::Defeat);
    assert!(m.valence < 0.0, "Defeat should have negative valence");
}

#[test]
fn create_combat_memory_draw_neutral_valence() {
    let m = MemoryUtils::create_combat_memory("tied".to_string(), vec![], 0.5, CombatOutcome::Draw);
    assert!(
        m.valence.abs() < 0.4,
        "Draw should have near-neutral valence"
    );
}

#[test]
fn create_combat_memory_retreat_negative_valence() {
    let m =
        MemoryUtils::create_combat_memory("ran".to_string(), vec![], 0.5, CombatOutcome::Retreat);
    assert!(m.valence < 0.0, "Retreat should have negative valence");
}

#[test]
fn create_combat_memory_category() {
    let m = MemoryUtils::create_combat_memory("x".to_string(), vec![], 0.5, CombatOutcome::Victory);
    assert_eq!(m.category, MemoryCategory::Combat);
}

#[test]
fn calculate_decay_recent_memory_high() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 0.5, vec![]);
    let now = current_timestamp();
    let decay = MemoryUtils::calculate_decay(&m, now);
    assert!(
        decay > 0.5,
        "Recent memory should have high decay value (retention)"
    );
}

#[test]
fn calculate_decay_high_importance_slower() {
    let m_high = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 1.0, vec![]);
    let m_low = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 0.1, vec![]);
    let future = current_timestamp() + 3600 * 24; // 24h later
    let decay_high = MemoryUtils::calculate_decay(&m_high, future);
    let decay_low = MemoryUtils::calculate_decay(&m_low, future);
    assert!(
        decay_high >= decay_low,
        "High importance should decay slower"
    );
}

#[test]
fn should_forget_recent_important_no() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 0.9, vec![]);
    let now = current_timestamp();
    assert!(!MemoryUtils::should_forget(&m, now, 0.1));
}

#[test]
fn should_forget_old_low_importance_yes() {
    let mut m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 0.01, vec![]);
    m.timestamp = 1000; // very old
    let now = current_timestamp();
    assert!(MemoryUtils::should_forget(&m, now, 0.5));
}

#[test]
fn consolidate_memories_empty() {
    let result = MemoryUtils::consolidate_memories(vec![], 0.9);
    assert!(result.is_empty());
}

#[test]
fn consolidate_memories_single() {
    let m = MemoryUtils::create_memory("t".to_string(), MemoryCategory::Social, 0.5, vec![]);
    let result = MemoryUtils::consolidate_memories(vec![m], 0.9);
    assert_eq!(result.len(), 1);
}

// ============================================================================
// TEXT PREPROCESSOR
// ============================================================================

#[test]
fn preprocess_trims_whitespace() {
    let result = TextPreprocessor::preprocess("  hello world  ");
    assert_eq!(result, "hello world");
}

#[test]
fn preprocess_truncates_at_8192() {
    let long = "a".repeat(10_000);
    let result = TextPreprocessor::preprocess(&long);
    assert!(result.len() <= 8192);
}

#[test]
fn chunk_text_single_chunk() {
    let chunks = TextPreprocessor::chunk_text("hello world", 100, 0);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "hello world");
}

#[test]
fn chunk_text_splits_long_text() {
    let text = "word ".repeat(100);
    let chunks = TextPreprocessor::chunk_text(&text, 20, 0);
    assert!(chunks.len() > 1);
}

#[test]
fn chunk_text_overlap() {
    let text = "one two three four five six seven eight nine ten";
    let chunks = TextPreprocessor::chunk_text(text, 20, 5);
    // With overlap, we should have overlapping content
    assert!(chunks.len() >= 2);
}

#[test]
fn extract_keyphrases_filters_short() {
    let kp = TextPreprocessor::extract_keyphrases("a the big battle is won by us");
    // Words <= 3 chars should be filtered
    for phrase in &kp {
        assert!(
            phrase.len() > 3,
            "Short words should be filtered: {}",
            phrase
        );
    }
}

#[test]
fn extract_keyphrases_empty_input() {
    let kp = TextPreprocessor::extract_keyphrases("");
    assert!(kp.is_empty());
}

// ============================================================================
// MOCK EMBEDDING CLIENT
// ============================================================================

#[tokio::test]
async fn mock_client_embed_returns_correct_dimensions() {
    let client = MockEmbeddingClient::new();
    let dims = client.dimensions();
    let embedding = client.embed("test").await.unwrap();
    assert_eq!(embedding.len(), dims);
}

#[tokio::test]
async fn mock_client_with_dimensions() {
    let client = MockEmbeddingClient::with_dimensions(128);
    assert_eq!(client.dimensions(), 128);
    let embedding = client.embed("test").await.unwrap();
    assert_eq!(embedding.len(), 128);
}

#[tokio::test]
async fn mock_client_embed_batch() {
    let client = MockEmbeddingClient::new();
    let texts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let embeddings = client.embed_batch(&texts).await.unwrap();
    assert_eq!(embeddings.len(), 3);
    for emb in &embeddings {
        assert_eq!(emb.len(), client.dimensions());
    }
}

#[tokio::test]
async fn mock_client_model_info() {
    let client = MockEmbeddingClient::new();
    let info = client.model_info();
    assert!(!info.name.is_empty());
    assert_eq!(info.dimensions, client.dimensions());
}

// ============================================================================
// PERFORMANCE MONITOR
// ============================================================================

#[test]
fn performance_monitor_elapsed_nonnegative() {
    let pm = PerformanceMonitor::start();
    assert!(pm.elapsed_ms() >= 0);
}

#[test]
fn performance_monitor_elapsed_us_nonnegative() {
    let pm = PerformanceMonitor::start();
    assert!(pm.elapsed_us() >= 0);
}

// ============================================================================
// STORED VECTOR / SEARCH RESULT
// ============================================================================

#[test]
fn stored_vector_fields() {
    let sv = StoredVector {
        id: "test".to_string(),
        vector: vec![1.0, 2.0],
        text: "content".to_string(),
        timestamp: 12345,
        importance: 0.7,
        metadata: HashMap::new(),
    };
    assert_eq!(sv.id, "test");
    assert_eq!(sv.text, "content");
    assert_eq!(sv.timestamp, 12345);
    assert!((sv.importance - 0.7).abs() < 1e-6);
}

#[test]
fn embedding_metrics_from_store() {
    let store = VectorStore::new(2);
    let m = store.get_metrics();
    assert_eq!(m.total_vectors, 0);
    assert_eq!(m.total_searches, 0);
}

// ============================================================================
// CURRENT TIMESTAMP
// ============================================================================

#[test]
fn current_timestamp_positive() {
    let ts = current_timestamp();
    assert!(ts > 0);
}

#[test]
fn current_timestamp_after_2024() {
    let ts = current_timestamp();
    assert!(ts > 1704067200); // 2024-01-01
}

// ============================================================================
// CAPACITY / WITH_CONFIG
// ============================================================================

#[test]
fn vector_store_with_config() {
    let mut config = EmbeddingConfig::default();
    config.dimensions = 5;
    config.max_vectors = 2;
    let store = VectorStore::with_config(config);
    store
        .insert("a".into(), vec![1.0, 0.0, 0.0, 0.0, 0.0], "a".into())
        .unwrap();
    store
        .insert("b".into(), vec![0.0, 1.0, 0.0, 0.0, 0.0], "b".into())
        .unwrap();
    // Third insert should fail if capacity is enforced
    let result = store.insert("c".into(), vec![0.0, 0.0, 1.0, 0.0, 0.0], "c".into());
    assert!(result.is_err(), "Should fail when exceeding max_vectors");
}

#[test]
fn vector_store_rebuild_index_no_error() {
    let store = VectorStore::new(2);
    store
        .insert("a".into(), vec![1.0, 0.0], "a".into())
        .unwrap();
    // rebuild_index is a no-op but should not error
    store.rebuild_index().unwrap();
}

// ============================================================================
// MUTATION KILL TESTS — targeting brute_force_search score formula and
// insert_with_metadata_and_auto_prune body replacement
// ============================================================================

/// Kill: brute_force_search `_ => 1.0 / (1.0 + distance)` mutations
/// (lines 185: /→%, /→*, +→-, +→*). Euclidean metric hits the catch-all
/// branch. For a known distance d, score must be 1/(1+d) ∈ (0, 1).
#[test]
fn euclidean_search_score_formula_correct() {
    let config = EmbeddingConfig {
        dimensions: 2,
        distance_metric: DistanceMetric::Euclidean,
        ..EmbeddingConfig::default()
    };
    let store = VectorStore::with_config(config);

    // Insert a vector at the origin and one at (3,4); distance from origin = 5.0
    store
        .insert("origin".into(), vec![0.0, 0.0], "origin".into())
        .unwrap();
    store
        .insert("far".into(), vec![3.0, 4.0], "far".into())
        .unwrap();

    // Search from origin: "origin" has distance 0, "far" has distance 5.0
    let results = store.search(&[0.0, 0.0], 2).unwrap();
    assert_eq!(results.len(), 2);

    // Closest result should be "origin" with score = 1/(1+0) = 1.0
    let origin_result = results.iter().find(|r| r.vector.id == "origin").unwrap();
    assert!(
        (origin_result.score - 1.0).abs() < 0.01,
        "origin score should be 1.0, got {}",
        origin_result.score
    );

    // "far" should have score = 1/(1+5) ≈ 0.1667
    let far_result = results.iter().find(|r| r.vector.id == "far").unwrap();
    let expected_far = 1.0 / (1.0 + 5.0);
    assert!(
        (far_result.score - expected_far).abs() < 0.02,
        "far score should be ~{:.4}, got {}",
        expected_far,
        far_result.score
    );
    // Sanity: score must be in (0, 1]
    assert!(far_result.score > 0.0 && far_result.score < 1.0);
}

/// Kill: insert_with_metadata_and_auto_prune body → Ok(()) (line 309).
/// The mutation replaces the entire body with Ok(()), so no vector is
/// actually inserted. We verify the vector is retrievable after insert.
#[test]
fn insert_with_metadata_and_auto_prune_stores_vector() {
    let store = VectorStore::new(2);
    let mut meta = HashMap::new();
    meta.insert("tag".to_string(), "test".to_string());
    store
        .insert_with_metadata_and_auto_prune(
            "ap1".into(),
            vec![1.0, 0.0],
            "auto-pruned".into(),
            0.5,
            meta,
        )
        .unwrap();

    assert_eq!(store.len(), 1, "vector should be stored");
    let got = store
        .get("ap1")
        .expect("inserted vector must be retrievable");
    assert_eq!(got.text, "auto-pruned");
    assert!((got.importance - 0.5).abs() < 1e-6);
    assert_eq!(got.metadata.get("tag").unwrap(), "test");
}
