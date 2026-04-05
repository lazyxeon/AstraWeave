/*!
# Vector Store

Efficient storage and retrieval of high-dimensional embedding vectors.
Uses HNSW indexing for fast approximate nearest neighbor search.
*/

use crate::{DistanceMetric, EmbeddingConfig, SearchResult, StoredVector};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// High-performance vector store with HNSW indexing (simplified implementation)
pub struct VectorStore {
    /// Configuration
    config: EmbeddingConfig,

    /// Storage for vectors and metadata
    vectors: Arc<DashMap<String, StoredVector>>,

    /// Next available index
    next_index: Arc<parking_lot::Mutex<usize>>,

    /// Metrics tracking
    metrics: Arc<RwLock<VectorStoreMetrics>>,
}

/// Metrics for vector store performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorStoreMetrics {
    pub total_vectors: usize,
    pub total_searches: u64,
    pub avg_search_time_ms: f32,
    pub index_build_time_ms: f32,
    pub memory_usage_bytes: u64,
    pub cache_hit_rate: f32,
}

impl VectorStore {
    /// Create a new vector store with default configuration
    pub fn new(dimensions: usize) -> Self {
        let config = EmbeddingConfig {
            dimensions,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a vector store with custom configuration
    pub fn with_config(config: EmbeddingConfig) -> Self {
        Self {
            config,
            vectors: Arc::new(DashMap::new()),
            next_index: Arc::new(parking_lot::Mutex::new(0)),
            metrics: Arc::new(RwLock::new(VectorStoreMetrics::default())),
        }
    }

    /// Get the distance function for the configured metric
    fn get_distance_func(&self) -> fn(&[f32], &[f32]) -> f32 {
        match self.config.distance_metric {
            DistanceMetric::Cosine => cosine_distance,
            DistanceMetric::Euclidean => euclidean_distance,
            DistanceMetric::Manhattan => manhattan_distance,
            DistanceMetric::DotProduct => dot_product_distance,
        }
    }

    /// Insert a vector into the store
    pub fn insert(&self, id: String, vector: Vec<f32>, text: String) -> Result<()> {
        if vector.len() != self.config.dimensions {
            return Err(anyhow!(
                "Vector dimension {} doesn't match expected {}",
                vector.len(),
                self.config.dimensions
            ));
        }

        // Check if we're at capacity
        if self.vectors.len() >= self.config.max_vectors {
            return Err(anyhow!(
                "Vector store at capacity: {}",
                self.config.max_vectors
            ));
        }

        let stored_vector = StoredVector {
            id: id.clone(),
            vector: vector.clone(),
            text,
            timestamp: current_timestamp(),
            importance: 1.0,
            metadata: HashMap::new(),
        };

        // Store vector
        self.vectors.insert(id, stored_vector);

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.total_vectors = self.vectors.len();
        }

        Ok(())
    }

    /// Insert a vector with metadata
    pub fn insert_with_metadata(
        &self,
        id: String,
        vector: Vec<f32>,
        text: String,
        importance: f32,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        if vector.len() != self.config.dimensions {
            return Err(anyhow!(
                "Vector dimension {} doesn't match expected {}",
                vector.len(),
                self.config.dimensions
            ));
        }

        let stored_vector = StoredVector {
            id: id.clone(),
            vector: vector.clone(),
            text,
            timestamp: current_timestamp(),
            importance,
            metadata,
        };

        self.vectors.insert(id, stored_vector);

        {
            let mut metrics = self.metrics.write();
            metrics.total_vectors = self.vectors.len();
        }

        Ok(())
    }

    /// Search for similar vectors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();

        if query.len() != self.config.dimensions {
            return Err(anyhow!(
                "Query dimension {} doesn't match expected {}",
                query.len(),
                self.config.dimensions
            ));
        }

        let results = self.brute_force_search(query, k)?;

        // Update metrics
        let search_time = start_time.elapsed().as_millis() as f32;
        {
            let mut metrics = self.metrics.write();
            metrics.total_searches += 1;
            metrics.avg_search_time_ms =
                (metrics.avg_search_time_ms * (metrics.total_searches - 1) as f32 + search_time)
                    / metrics.total_searches as f32;
        }

        Ok(results)
    }

    /// Brute force search (used when index is not available)
    fn brute_force_search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        let distance_func = self.get_distance_func();
        let mut results = Vec::new();

        for entry in self.vectors.iter() {
            let stored_vector = entry.value();
            let distance = distance_func(query, &stored_vector.vector);
            let score = match self.config.distance_metric {
                DistanceMetric::Cosine => 1.0 - distance,
                DistanceMetric::DotProduct => -distance,
                _ => 1.0 / (1.0 + distance),
            };

            results.push(SearchResult {
                vector: stored_vector.clone(),
                score,
                distance,
            });
        }

        // Sort by distance and take top k
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(k);

        Ok(results)
    }

    /// Remove a vector from the store
    pub fn remove(&self, id: &str) -> Option<StoredVector> {
        if let Some((_, stored_vector)) = self.vectors.remove(id) {
            Some(stored_vector)
        } else {
            None
        }
    }

    /// Get a vector by ID
    pub fn get(&self, id: &str) -> Option<StoredVector> {
        self.vectors.get(id).map(|entry| entry.value().clone())
    }

    /// Get all vector IDs
    pub fn get_all_ids(&self) -> Vec<String> {
        self.vectors
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get number of stored vectors
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Clear all vectors
    pub fn clear(&self) {
        self.vectors.clear();

        {
            let mut next_idx = self.next_index.lock();
            *next_idx = 0;
        }
    }

    /// Rebuild index (simplified stub - no HNSW in this version)
    pub fn rebuild_index(&self) -> Result<()> {
        // Simplified implementation - no actual indexing
        Ok(())
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> VectorStoreMetrics {
        self.metrics.read().clone()
    }

    /// Prune old or low-importance vectors to stay under capacity
    pub fn prune_vectors(&self, target_count: usize) -> Result<usize> {
        if self.vectors.len() <= target_count {
            return Ok(0);
        }

        // Collect vectors with scores (recency + importance)
        let mut scored_vectors: Vec<(String, f32)> = Vec::new();
        let current_time = current_timestamp();

        for entry in self.vectors.iter() {
            let vector = entry.value();
            let age_hours = ((current_time - vector.timestamp) as f32) / 3600.0;
            let recency_score = (-age_hours / 24.0).exp(); // Exponential decay over days
            let total_score = vector.importance * 0.7 + recency_score * 0.3;
            scored_vectors.push((vector.id.clone(), total_score));
        }

        // Sort by score (ascending, so lowest scores are first)
        scored_vectors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Remove lowest-scoring vectors
        let to_remove = self.vectors.len() - target_count;
        let mut removed_count = 0;

        for (id, _) in scored_vectors.iter().take(to_remove) {
            if self.remove(id).is_some() {
                removed_count += 1;
            }
        }

        // No index rebuild needed in simplified version

        Ok(removed_count)
    }
    /// Insert a vector into the store, automatically pruning if at capacity
    pub fn insert_with_auto_prune(&self, id: String, vector: Vec<f32>, text: String) -> Result<()> {
        if self.vectors.len() >= self.config.max_vectors {
            // Prune 10% of capacity to make room
            let prune_target = (self.config.max_vectors as f32 * 0.9) as usize;
            self.prune_vectors(prune_target)?;
        }
        self.insert(id, vector, text)
    }

    /// Insert a vector with metadata, automatically pruning if at capacity
    pub fn insert_with_metadata_and_auto_prune(
        &self,
        id: String,
        vector: Vec<f32>,
        text: String,
        importance: f32,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        if self.vectors.len() >= self.config.max_vectors {
            let prune_target = (self.config.max_vectors as f32 * 0.9) as usize;
            self.prune_vectors(prune_target)?;
        }
        self.insert_with_metadata(id, vector, text, importance, metadata)
    }

    /// Serialize the store to a JSON string
    pub fn to_json(&self) -> Result<String> {
        let vectors_map: HashMap<String, StoredVector> = self
            .vectors
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let snapshot = VectorStoreSnapshot {
            config: self.config.clone(),
            vectors: vectors_map,
            metrics: self.metrics.read().clone(),
        };

        Ok(serde_json::to_string(&snapshot)?)
    }

    /// Deserialize the store from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let snapshot: VectorStoreSnapshot = serde_json::from_str(json)?;

        let vectors = DashMap::new();
        for (k, v) in snapshot.vectors {
            vectors.insert(k, v);
        }

        Ok(Self {
            config: snapshot.config,
            vectors: Arc::new(vectors),
            next_index: Arc::new(parking_lot::Mutex::new(0)), // Reset index
            metrics: Arc::new(RwLock::new(snapshot.metrics)),
        })
    }
}

/// Snapshot for serialization
#[derive(Serialize, Deserialize)]
struct VectorStoreSnapshot {
    config: EmbeddingConfig,
    vectors: HashMap<String, StoredVector>,
    metrics: VectorStoreMetrics,
}

/// Distance functions for different metrics
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        1.0 // Maximum distance for zero vectors
    } else {
        1.0 - (dot_product / (norm_a * norm_b))
    }
}

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    -a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>() // Negative for distance
}

/// Get current Unix timestamp in seconds
/// Returns 0 if system time is before UNIX_EPOCH (should never happen in practice)
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HELPER FUNCTIONS
    // ========================================================================

    fn create_test_store(dimensions: usize) -> VectorStore {
        VectorStore::new(dimensions)
    }

    fn create_store_with_max_capacity(dimensions: usize, max_vectors: usize) -> VectorStore {
        let config = EmbeddingConfig {
            dimensions,
            max_vectors,
            ..Default::default()
        };
        VectorStore::with_config(config)
    }

    fn create_store_with_metric(dimensions: usize, metric: DistanceMetric) -> VectorStore {
        let config = EmbeddingConfig {
            dimensions,
            distance_metric: metric,
            ..Default::default()
        };
        VectorStore::with_config(config)
    }

    // ========================================================================
    // BASIC CREATION TESTS
    // ========================================================================

    #[test]
    fn test_vector_store_creation() {
        let store = VectorStore::new(384);
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_with_config() {
        let config = EmbeddingConfig {
            dimensions: 256,
            max_vectors: 500,
            distance_metric: DistanceMetric::Euclidean,
            ..Default::default()
        };
        let store = VectorStore::with_config(config);
        assert!(store.is_empty());
    }

    #[test]
    fn test_is_empty_true() {
        let store = create_test_store(3);
        assert!(store.is_empty());
    }

    #[test]
    fn test_is_empty_false() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 2.0, 3.0], "test".to_string())
            .unwrap();
        assert!(!store.is_empty());
    }

    // ========================================================================
    // INSERT TESTS
    // ========================================================================

    #[test]
    fn test_insert_and_get() {
        let store = VectorStore::new(3);
        let vector = vec![1.0, 2.0, 3.0];

        store
            .insert(
                "test_1".to_string(),
                vector.clone(),
                "test vector".to_string(),
            )
            .unwrap();

        let retrieved = store.get("test_1").unwrap();
        assert_eq!(retrieved.vector, vector);
        assert_eq!(retrieved.text, "test vector");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_insert_dimension_mismatch() {
        let store = create_test_store(3);
        let result = store.insert("v1".to_string(), vec![1.0, 2.0], "wrong dim".to_string());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("dimension"));
    }

    #[test]
    fn test_insert_at_capacity_error() {
        let store = create_store_with_max_capacity(3, 2);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "first".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "second".to_string())
            .unwrap();

        let result = store.insert("v3".to_string(), vec![0.0, 0.0, 1.0], "third".to_string());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("capacity"));
    }

    #[test]
    fn test_insert_updates_metrics() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "first".to_string())
            .unwrap();

        let metrics = store.get_metrics();
        assert_eq!(metrics.total_vectors, 1);
    }

    #[test]
    fn test_insert_with_metadata() {
        let store = create_test_store(3);
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), "test".to_string());
        metadata.insert("priority".to_string(), "high".to_string());

        store
            .insert_with_metadata(
                "v1".to_string(),
                vec![1.0, 2.0, 3.0],
                "metadata test".to_string(),
                0.8,
                metadata.clone(),
            )
            .unwrap();

        let retrieved = store.get("v1").unwrap();
        assert_eq!(retrieved.importance, 0.8);
        assert_eq!(
            retrieved.metadata.get("category"),
            Some(&"test".to_string())
        );
        assert_eq!(
            retrieved.metadata.get("priority"),
            Some(&"high".to_string())
        );
    }

    #[test]
    fn test_insert_with_metadata_dimension_mismatch() {
        let store = create_test_store(3);
        let result = store.insert_with_metadata(
            "v1".to_string(),
            vec![1.0, 2.0, 3.0, 4.0], // Wrong dimension
            "test".to_string(),
            0.5,
            HashMap::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_overwrites_existing() {
        let store = create_test_store(3);
        store
            .insert(
                "v1".to_string(),
                vec![1.0, 0.0, 0.0],
                "original".to_string(),
            )
            .unwrap();
        store
            .insert("v1".to_string(), vec![0.0, 1.0, 0.0], "updated".to_string())
            .unwrap();

        let retrieved = store.get("v1").unwrap();
        assert_eq!(retrieved.text, "updated");
        assert_eq!(retrieved.vector, vec![0.0, 1.0, 0.0]);
        assert_eq!(store.len(), 1);
    }

    // ========================================================================
    // AUTO-PRUNE INSERT TESTS
    // ========================================================================

    #[test]
    fn test_insert_with_auto_prune() {
        let store = create_store_with_max_capacity(3, 3);

        // Fill to capacity
        store
            .insert_with_metadata(
                "v1".to_string(),
                vec![1.0, 0.0, 0.0],
                "low".to_string(),
                0.1,
                HashMap::new(),
            )
            .unwrap();
        store
            .insert_with_metadata(
                "v2".to_string(),
                vec![0.0, 1.0, 0.0],
                "medium".to_string(),
                0.5,
                HashMap::new(),
            )
            .unwrap();
        store
            .insert_with_metadata(
                "v3".to_string(),
                vec![0.0, 0.0, 1.0],
                "high".to_string(),
                0.9,
                HashMap::new(),
            )
            .unwrap();

        assert_eq!(store.len(), 3);

        // Insert with auto-prune should prune first and then insert
        store
            .insert_with_auto_prune("v4".to_string(), vec![1.0, 1.0, 0.0], "new".to_string())
            .unwrap();

        // Should still have room (pruned 10% = 0, but at least allows insert after prune attempt)
        assert!(store.len() <= 3);
    }

    #[test]
    fn test_insert_with_metadata_and_auto_prune() {
        let store = create_store_with_max_capacity(3, 3);

        // Fill to capacity with low importance
        store
            .insert_with_metadata(
                "v1".to_string(),
                vec![1.0, 0.0, 0.0],
                "a".to_string(),
                0.1,
                HashMap::new(),
            )
            .unwrap();
        store
            .insert_with_metadata(
                "v2".to_string(),
                vec![0.0, 1.0, 0.0],
                "b".to_string(),
                0.1,
                HashMap::new(),
            )
            .unwrap();
        store
            .insert_with_metadata(
                "v3".to_string(),
                vec![0.0, 0.0, 1.0],
                "c".to_string(),
                0.1,
                HashMap::new(),
            )
            .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        store
            .insert_with_metadata_and_auto_prune(
                "v4".to_string(),
                vec![1.0, 1.0, 1.0],
                "new with metadata".to_string(),
                0.9,
                metadata,
            )
            .unwrap();

        // Should still have room after auto-prune
        assert!(store.len() <= 3);
    }

    // ========================================================================
    // SEARCH TESTS
    // ========================================================================

    #[test]
    fn test_search() {
        let store = VectorStore::new(3);

        // Insert test vectors
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "first".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "second".to_string())
            .unwrap();
        store
            .insert("v3".to_string(), vec![0.0, 0.0, 1.0], "third".to_string())
            .unwrap();

        // Search for vector similar to v1
        let query = vec![0.9, 0.1, 0.0];
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].vector.id, "v1"); // Should be most similar
    }

    #[test]
    fn test_search_dimension_mismatch() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "test".to_string())
            .unwrap();

        let result = store.search(&[1.0, 0.0], 1);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("dimension"));
    }

    #[test]
    fn test_search_updates_metrics() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "test".to_string())
            .unwrap();

        let _ = store.search(&[1.0, 0.0, 0.0], 1).unwrap();
        let _ = store.search(&[0.0, 1.0, 0.0], 1).unwrap();

        let metrics = store.get_metrics();
        assert_eq!(metrics.total_searches, 2);
        assert!(metrics.avg_search_time_ms >= 0.0);
    }

    #[test]
    fn test_search_empty_store() {
        let store = create_test_store(3);
        let results = store.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_k_larger_than_store() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_with_euclidean_metric() {
        let store = create_store_with_metric(3, DistanceMetric::Euclidean);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();
        store
            .insert("v3".to_string(), vec![0.5, 0.5, 0.0], "c".to_string())
            .unwrap();

        let results = store.search(&[0.5, 0.5, 0.0], 1).unwrap();
        assert_eq!(results[0].vector.id, "v3");
    }

    #[test]
    fn test_search_with_manhattan_metric() {
        let store = create_store_with_metric(3, DistanceMetric::Manhattan);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        let results = store.search(&[0.9, 0.1, 0.0], 1).unwrap();
        assert_eq!(results[0].vector.id, "v1");
    }

    #[test]
    fn test_search_with_dot_product_metric() {
        let store = create_store_with_metric(3, DistanceMetric::DotProduct);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 2).unwrap();
        // Dot product should rank v1 higher (dot product = 1.0 vs 0.0)
        assert_eq!(results[0].vector.id, "v1");
    }

    // ========================================================================
    // REMOVE / GET / UTILITY TESTS
    // ========================================================================

    #[test]
    fn test_remove() {
        let store = VectorStore::new(3);
        store
            .insert("test".to_string(), vec![1.0, 2.0, 3.0], "test".to_string())
            .unwrap();

        assert_eq!(store.len(), 1);

        let removed = store.remove("test").unwrap();
        assert_eq!(removed.id, "test");
        assert_eq!(store.len(), 0);
        assert!(store.get("test").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let store = create_test_store(3);
        let result = store.remove("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_nonexistent() {
        let store = create_test_store(3);
        let result = store.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_ids_empty() {
        let store = create_test_store(3);
        let ids = store.get_all_ids();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_get_all_ids() {
        let store = create_test_store(3);
        store
            .insert("a".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("b".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();
        store
            .insert("c".to_string(), vec![0.0, 0.0, 1.0], "c".to_string())
            .unwrap();

        let mut ids = store.get_all_ids();
        ids.sort();

        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_clear() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        assert_eq!(store.len(), 2);

        store.clear();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert!(store.get("v1").is_none());
        assert!(store.get("v2").is_none());
    }

    #[test]
    fn test_rebuild_index() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();

        // Should succeed (no-op in simplified version)
        let result = store.rebuild_index();
        assert!(result.is_ok());
    }

    // ========================================================================
    // METRICS TESTS
    // ========================================================================

    #[test]
    fn test_get_metrics_initial() {
        let store = create_test_store(3);
        let metrics = store.get_metrics();

        assert_eq!(metrics.total_vectors, 0);
        assert_eq!(metrics.total_searches, 0);
        assert_eq!(metrics.avg_search_time_ms, 0.0);
    }

    #[test]
    fn test_get_metrics_after_operations() {
        let store = create_test_store(3);

        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        let _ = store.search(&[0.5, 0.5, 0.0], 1).unwrap();

        let metrics = store.get_metrics();
        assert_eq!(metrics.total_vectors, 2);
        assert_eq!(metrics.total_searches, 1);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = VectorStoreMetrics::default();
        assert_eq!(metrics.total_vectors, 0);
        assert_eq!(metrics.total_searches, 0);
        assert_eq!(metrics.avg_search_time_ms, 0.0);
        assert_eq!(metrics.index_build_time_ms, 0.0);
        assert_eq!(metrics.memory_usage_bytes, 0);
        assert_eq!(metrics.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_metrics_clone() {
        let metrics = VectorStoreMetrics {
            total_vectors: 100,
            total_searches: 50,
            ..Default::default()
        };

        let cloned = metrics.clone();
        assert_eq!(cloned.total_vectors, 100);
        assert_eq!(cloned.total_searches, 50);
    }

    // ========================================================================
    // DISTANCE FUNCTION TESTS
    // ========================================================================

    #[test]
    fn test_distance_functions() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0]; // Same as a

        // Cosine distance
        let cos_ab = cosine_distance(&a, &b);
        let cos_ac = cosine_distance(&a, &c);
        assert!(cos_ab > cos_ac); // a and b are orthogonal, a and c are identical
        assert_eq!(cos_ac, 0.0); // Identical vectors have 0 cosine distance

        // Euclidean distance
        let euc_ab = euclidean_distance(&a, &b);
        let euc_ac = euclidean_distance(&a, &c);
        assert!(euc_ab > euc_ac);
        assert_eq!(euc_ac, 0.0);
    }

    #[test]
    fn test_cosine_distance_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];

        let dist = cosine_distance(&a, &b);
        assert_eq!(dist, 1.0); // Maximum distance for zero vector
    }

    #[test]
    fn test_cosine_distance_identical() {
        let a = vec![0.5, 0.5, 0.5];
        let dist = cosine_distance(&a, &a);
        assert!(dist.abs() < 0.0001); // Should be ~0
    }

    #[test]
    fn test_cosine_distance_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];

        let dist = cosine_distance(&a, &b);
        assert!((dist - 2.0).abs() < 0.0001); // Opposite directions = max distance
    }

    #[test]
    fn test_euclidean_distance_basic() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];

        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.0001); // 3-4-5 triangle
    }

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = euclidean_distance(&a, &a);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn test_manhattan_distance_basic() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];

        let dist = manhattan_distance(&a, &b);
        assert_eq!(dist, 6.0); // 1 + 2 + 3
    }

    #[test]
    fn test_manhattan_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = manhattan_distance(&a, &a);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn test_manhattan_distance_negative() {
        let a = vec![5.0, 5.0];
        let b = vec![2.0, 1.0];

        let dist = manhattan_distance(&a, &b);
        assert_eq!(dist, 7.0); // |5-2| + |5-1| = 3 + 4
    }

    #[test]
    fn test_dot_product_distance_basic() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];

        let dist = dot_product_distance(&a, &b);
        assert_eq!(dist, -1.0); // Negative of dot product
    }

    #[test]
    fn test_dot_product_distance_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let dist = dot_product_distance(&a, &b);
        assert_eq!(dist, 0.0); // Orthogonal = 0 dot product
    }

    #[test]
    fn test_dot_product_distance_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];

        let dist = dot_product_distance(&a, &b);
        assert_eq!(dist, 1.0); // -(-1) = 1
    }

    // ========================================================================
    // PRUNE TESTS
    // ========================================================================

    #[test]
    fn test_prune_vectors() {
        let store = VectorStore::new(3);

        // Insert vectors with different importance
        store
            .insert_with_metadata(
                "low_importance".to_string(),
                vec![1.0, 0.0, 0.0],
                "low".to_string(),
                0.1,
                HashMap::new(),
            )
            .unwrap();

        store
            .insert_with_metadata(
                "high_importance".to_string(),
                vec![0.0, 1.0, 0.0],
                "high".to_string(),
                0.9,
                HashMap::new(),
            )
            .unwrap();

        store
            .insert_with_metadata(
                "medium_importance".to_string(),
                vec![0.0, 0.0, 1.0],
                "medium".to_string(),
                0.5,
                HashMap::new(),
            )
            .unwrap();

        assert_eq!(store.len(), 3);

        // Prune to 2 vectors
        let removed = store.prune_vectors(2).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.len(), 2);

        // Low importance vector should be removed
        assert!(store.get("low_importance").is_none());
        assert!(store.get("high_importance").is_some());
        assert!(store.get("medium_importance").is_some());
    }

    #[test]
    fn test_prune_vectors_already_under_target() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();

        let removed = store.prune_vectors(10).unwrap();
        assert_eq!(removed, 0);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_prune_vectors_to_zero() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "a".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "b".to_string())
            .unwrap();

        let removed = store.prune_vectors(0).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.len(), 0);
    }

    // ========================================================================
    // SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_to_json() {
        let store = create_test_store(3);
        store
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], "first".to_string())
            .unwrap();
        store
            .insert("v2".to_string(), vec![0.0, 1.0, 0.0], "second".to_string())
            .unwrap();

        let json = store.to_json().unwrap();

        assert!(json.contains("\"v1\""));
        assert!(json.contains("\"v2\""));
        assert!(json.contains("first"));
        assert!(json.contains("second"));
    }

    #[test]
    fn test_from_json() {
        let store = create_test_store(3);
        store
            .insert_with_metadata(
                "v1".to_string(),
                vec![1.0, 2.0, 3.0],
                "test".to_string(),
                0.7,
                HashMap::new(),
            )
            .unwrap();

        let _ = store.search(&[1.0, 2.0, 3.0], 1).unwrap();

        let json = store.to_json().unwrap();
        let restored = VectorStore::from_json(&json).unwrap();

        assert_eq!(restored.len(), 1);
        let retrieved = restored.get("v1").unwrap();
        assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);
        assert_eq!(retrieved.text, "test");
        assert_eq!(retrieved.importance, 0.7);

        let metrics = restored.get_metrics();
        assert_eq!(metrics.total_searches, 1);
    }

    #[test]
    fn test_from_json_invalid() {
        let result = VectorStore::from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let store = create_store_with_metric(4, DistanceMetric::Euclidean);

        let mut metadata = HashMap::new();
        metadata.insert("tag".to_string(), "test".to_string());

        store
            .insert_with_metadata(
                "vec1".to_string(),
                vec![0.1, 0.2, 0.3, 0.4],
                "first vector".to_string(),
                0.95,
                metadata,
            )
            .unwrap();

        store
            .insert(
                "vec2".to_string(),
                vec![0.5, 0.6, 0.7, 0.8],
                "second".to_string(),
            )
            .unwrap();

        // Perform some searches to update metrics
        let _ = store.search(&[0.1, 0.2, 0.3, 0.4], 1).unwrap();

        // Serialize and deserialize
        let json = store.to_json().unwrap();
        let restored = VectorStore::from_json(&json).unwrap();

        // Verify all data
        assert_eq!(restored.len(), 2);

        let v1 = restored.get("vec1").unwrap();
        assert_eq!(v1.text, "first vector");
        assert_eq!(v1.importance, 0.95);
        assert_eq!(v1.metadata.get("tag"), Some(&"test".to_string()));

        let v2 = restored.get("vec2").unwrap();
        assert_eq!(v2.text, "second");
    }

    #[test]
    fn test_to_json_empty_store() {
        let store = create_test_store(3);
        let json = store.to_json().unwrap();

        let restored = VectorStore::from_json(&json).unwrap();
        assert!(restored.is_empty());
    }

    // ========================================================================
    // CONCURRENT ACCESS TESTS
    // ========================================================================

    #[test]
    fn test_concurrent_insert() {
        use std::thread;

        let store = Arc::new(create_test_store(3));
        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                store_clone
                    .insert(
                        format!("v{}", i),
                        vec![i as f32, 0.0, 0.0],
                        format!("vector {}", i),
                    )
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.len(), 10);
    }

    #[test]
    fn test_concurrent_search() {
        use std::thread;

        let store = Arc::new(create_test_store(3));

        // Pre-populate
        for i in 0..10 {
            store
                .insert(
                    format!("v{}", i),
                    vec![i as f32, 0.0, 0.0],
                    format!("vector {}", i),
                )
                .unwrap();
        }

        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let query = vec![i as f32, 0.0, 0.0];
                let results = store_clone.search(&query, 3).unwrap();
                assert!(!results.is_empty());
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // ========================================================================
    // HELPER FUNCTION TESTS
    // ========================================================================

    #[test]
    fn test_current_timestamp() {
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = current_timestamp();

        // Should be non-zero and increasing (or equal for very fast execution)
        assert!(ts1 > 0);
        assert!(ts2 >= ts1);
    }

    #[test]
    fn test_get_distance_func_cosine() {
        let store = create_store_with_metric(3, DistanceMetric::Cosine);
        let func = store.get_distance_func();

        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];

        assert_eq!(func(&a, &b), cosine_distance(&a, &b));
    }

    #[test]
    fn test_get_distance_func_euclidean() {
        let store = create_store_with_metric(3, DistanceMetric::Euclidean);
        let func = store.get_distance_func();

        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];

        assert_eq!(func(&a, &b), euclidean_distance(&a, &b));
    }

    #[test]
    fn test_get_distance_func_manhattan() {
        let store = create_store_with_metric(3, DistanceMetric::Manhattan);
        let func = store.get_distance_func();

        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];

        assert_eq!(func(&a, &b), manhattan_distance(&a, &b));
    }

    #[test]
    fn test_get_distance_func_dot_product() {
        let store = create_store_with_metric(3, DistanceMetric::DotProduct);
        let func = store.get_distance_func();

        let a = [1.0, 0.0, 0.0];
        let b = [0.5, 0.5, 0.0];

        assert_eq!(func(&a, &b), dot_product_distance(&a, &b));
    }
}
