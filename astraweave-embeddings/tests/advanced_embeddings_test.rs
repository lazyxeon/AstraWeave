use astraweave_embeddings::{
    DistanceMetric, EmbeddingClient, EmbeddingConfig, MockEmbeddingClient, SimilarityMetrics,
    VectorStore,
};
use std::sync::Arc;
use tokio::task;

// ============================================================================
// VectorStore Tests
// ============================================================================

#[tokio::test]
async fn test_large_scale_search() {
    // Scale down for test speed, but enough to verify logic
    let vector_count = 1000;
    let dimensions = 10;
    let config = EmbeddingConfig {
        dimensions,
        max_vectors: vector_count + 100,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    // Insert vectors
    for i in 0..vector_count {
        let mut vector = vec![0.0; dimensions];
        vector[i % dimensions] = 1.0; // Simple pattern
        store
            .insert(format!("vec_{}", i), vector, format!("text_{}", i))
            .unwrap();
    }

    assert_eq!(store.len(), vector_count);

    // Search
    let query = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; // Matches indices 0, 10, 20...
    let results = store.search(&query, 5).unwrap();

    assert_eq!(results.len(), 5);
    for result in results {
        // Should match vectors with 1.0 at index 0
        assert_eq!(result.vector.vector[0], 1.0);
    }
}

#[test]
fn test_distance_metric_cosine() {
    let config = EmbeddingConfig {
        distance_metric: DistanceMetric::Cosine,
        dimensions: 3,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    store
        .insert("a".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
        .unwrap();

    // Search with identical vector
    let results = store.search(&[1.0, 0.0, 0.0], 1).unwrap();
    assert!(results[0].score > 0.99); // Cosine similarity 1.0

    // Search with orthogonal vector
    let results = store.search(&[0.0, 1.0, 0.0], 1).unwrap();
    assert!(results[0].score < 0.01); // Cosine similarity 0.0 (converted to score)
}

#[test]
fn test_distance_metric_euclidean() {
    let config = EmbeddingConfig {
        distance_metric: DistanceMetric::Euclidean,
        dimensions: 3,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    store
        .insert("a".to_string(), vec![0.0, 0.0, 0.0], "origin".to_string())
        .unwrap();

    // Distance to (3, 4, 0) should be 5
    let results = store.search(&[3.0, 4.0, 0.0], 1).unwrap();
    assert!((results[0].distance - 5.0).abs() < 0.001);
}

#[test]
fn test_distance_metric_manhattan() {
    let config = EmbeddingConfig {
        distance_metric: DistanceMetric::Manhattan,
        dimensions: 3,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    store
        .insert("a".to_string(), vec![0.0, 0.0, 0.0], "origin".to_string())
        .unwrap();

    // Distance to (1, 2, 3) should be 1+2+3 = 6
    let results = store.search(&[1.0, 2.0, 3.0], 1).unwrap();
    assert!((results[0].distance - 6.0).abs() < 0.001);
}

#[test]
fn test_distance_metric_dot() {
    let config = EmbeddingConfig {
        distance_metric: DistanceMetric::DotProduct,
        dimensions: 3,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    store
        .insert("a".to_string(), vec![1.0, 2.0, 3.0], "vec".to_string())
        .unwrap();

    // Dot product with (1, 1, 1) should be 1+2+3 = 6
    let results = store.search(&[1.0, 1.0, 1.0], 1).unwrap();
    // Note: store implementation negates dot product for distance sorting
    // score = -distance = dot_product
    assert!((results[0].score - 6.0).abs() < 0.001);
}

#[test]
fn test_capacity_limits() {
    let config = EmbeddingConfig {
        max_vectors: 2,
        dimensions: 2,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    store
        .insert("1".to_string(), vec![0.0, 0.0], "1".to_string())
        .unwrap();
    store
        .insert("2".to_string(), vec![0.0, 0.0], "2".to_string())
        .unwrap();

    // Should fail
    let result = store.insert("3".to_string(), vec![0.0, 0.0], "3".to_string());
    assert!(result.is_err());
}

#[test]
fn test_overflow_handling() {
    let config = EmbeddingConfig {
        max_vectors: 5,
        dimensions: 2,
        ..Default::default()
    };
    let store = VectorStore::with_config(config);

    // Fill store
    for i in 0..5 {
        store
            .insert(format!("{}", i), vec![0.0, 0.0], format!("{}", i))
            .unwrap();
    }

    // Insert with auto-prune
    store
        .insert_with_auto_prune("new".to_string(), vec![0.0, 0.0], "new".to_string())
        .unwrap();

    // Should have pruned (target is 90% of 5 = 4.5 -> 4)
    // So we expect 4 old + 1 new = 5 vectors total
    assert_eq!(store.len(), 5);
    assert!(store.get("new").is_some());
}

#[tokio::test]
async fn test_concurrent_insertion() {
    let store = Arc::new(VectorStore::new(10));
    let mut handles = vec![];

    for i in 0..10 {
        let store_clone = store.clone();
        handles.push(task::spawn(async move {
            store_clone
                .insert(
                    format!("thread_{}", i),
                    vec![0.0; 10],
                    format!("text_{}", i),
                )
                .unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(store.len(), 10);
}

#[tokio::test]
async fn test_concurrent_search() {
    let store = Arc::new(VectorStore::new(10));
    store
        .insert("target".to_string(), vec![1.0; 10], "target".to_string())
        .unwrap();

    let mut handles = vec![];
    for _ in 0..10 {
        let store_clone = store.clone();
        handles.push(task::spawn(async move {
            let results = store_clone.search(&[1.0_f32; 10], 1).unwrap();
            assert_eq!(results.len(), 1);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[test]
fn test_vectorstore_serialization() {
    let store = VectorStore::new(2);
    store
        .insert("1".to_string(), vec![1.0, 2.0], "text".to_string())
        .unwrap();

    let json = store.to_json().unwrap();
    let loaded_store = VectorStore::from_json(&json).unwrap();

    assert_eq!(loaded_store.len(), 1);
    let vec = loaded_store.get("1").unwrap();
    assert_eq!(vec.vector, vec![1.0, 2.0]);
}

// ============================================================================
// EmbeddingClient Tests
// ============================================================================

#[tokio::test]
async fn test_batch_embedding() {
    let client = MockEmbeddingClient::new();
    let texts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let embeddings = client.embed_batch(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    assert_eq!(embeddings[0].len(), 384);
}

#[tokio::test]
async fn test_cache_hit_behavior() {
    let client = MockEmbeddingClient::new();
    let text = "cached text";

    // First call populates cache
    let _ = client.embed(text).await.unwrap();

    // Second call should hit cache (MockEmbeddingClient implementation details make this hard to verify externally without metrics,
    // but we can verify consistency)
    let emb2 = client.embed(text).await.unwrap();
    assert_eq!(emb2.len(), 384);
}

#[tokio::test]
async fn test_embedding_dimension_consistency() {
    let client = MockEmbeddingClient::with_dimensions(128);
    let emb = client.embed("test").await.unwrap();
    assert_eq!(emb.len(), 128);
    assert_eq!(client.dimensions(), 128);
}

#[tokio::test]
async fn test_client_thread_safety() {
    let client = Arc::new(MockEmbeddingClient::new());
    let mut handles = vec![];

    for i in 0..10 {
        let client_clone = client.clone();
        handles.push(task::spawn(async move {
            let _ = client_clone.embed(&format!("text_{}", i)).await.unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[test]
fn test_model_info_validation() {
    let client = MockEmbeddingClient::new();
    let info = client.model_info();
    assert!(!info.name.is_empty());
    assert!(info.dimensions > 0);
}

// ============================================================================
// Utils Tests
// ============================================================================

#[test]
fn test_vector_normalization() {
    let mut v = vec![3.0, 4.0]; // Magnitude 5
    SimilarityMetrics::normalize_vector(&mut v);

    assert!((v[0] - 0.6).abs() < 0.001); // 3/5
    assert!((v[1] - 0.8).abs() < 0.001); // 4/5

    let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((mag - 1.0).abs() < 0.001);
}

#[test]
fn test_cosine_distance_accuracy() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let sim = SimilarityMetrics::cosine_similarity(&a, &b);
    assert_eq!(sim, 0.0);

    let c = vec![1.0, 0.0];
    let sim2 = SimilarityMetrics::cosine_similarity(&a, &c);
    assert_eq!(sim2, 1.0);
}

#[test]
fn test_euclidean_distance_accuracy() {
    let a = vec![0.0, 0.0];
    let b = vec![3.0, 4.0];
    let dist = SimilarityMetrics::euclidean_distance(&a, &b);
    assert!((dist - 5.0).abs() < 0.001);
}

#[test]
fn test_manhattan_distance_accuracy() {
    let a = vec![0.0, 0.0];
    let b = vec![3.0, 4.0];
    let dist = SimilarityMetrics::manhattan_distance(&a, &b);
    assert!((dist - 7.0).abs() < 0.001);
}

#[test]
fn test_dot_product_accuracy() {
    let a = vec![1.0, 2.0];
    let b = vec![3.0, 4.0];
    let dot = SimilarityMetrics::dot_product(&a, &b);
    assert!((dot - 11.0).abs() < 0.001); // 1*3 + 2*4 = 3 + 8 = 11
}

#[test]
fn test_distance_edge_cases() {
    let a = vec![0.0, 0.0];
    let b = vec![0.0, 0.0];

    assert_eq!(SimilarityMetrics::euclidean_distance(&a, &b), 0.0);
    assert_eq!(SimilarityMetrics::manhattan_distance(&a, &b), 0.0);
    assert_eq!(SimilarityMetrics::dot_product(&a, &b), 0.0);

    // Cosine similarity of zero vectors is 0.0 in our implementation
    assert_eq!(SimilarityMetrics::cosine_similarity(&a, &b), 0.0);
}
