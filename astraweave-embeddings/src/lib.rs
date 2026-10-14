#![forbid(unsafe_code)]
/*!
# AstraWeave Embeddings

High-performance embeddings and vector search for AI-native gaming. This crate provides:

- **Embedding Clients**: Trait-based abstraction for different embedding models
- **Vector Store**: Efficient storage and retrieval of high-dimensional vectors
- **Semantic Search**: Hierarchical navigable small world (HNSW) search. The index lives in this crate (`hnsw.rs`) and is deterministic: integer level draws, id tie-breaks, exact top-k when the search beam covers every node. The optional `hnsw_rs` dependency is not used; its construction RNG is outside the sim contract.
- **Memory Management**: Batching, caching, and memory optimization

## Quick Start

```rust
use astraweave_embeddings::{EmbeddingClient, MockEmbeddingClient, VectorStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create embedding client
    let client = MockEmbeddingClient::new();

    // Create vector store
    let mut store = VectorStore::new(384); // 384-dimensional vectors

    // Embed and store text
    let text = "The player approached the mysterious door";
    let embedding = client.embed(text).await?;
    store.insert("memory_1".to_string(), embedding, text.to_string())?;

    // Search for similar memories
    let query_embedding = client.embed("The player walked to a door").await?;
    let results = store.search(&query_embedding, 5)?;

    println!("Found {} similar memories", results.len());

    Ok(())
}
```
*/

pub mod client;
mod hnsw;
pub mod store;
pub mod utils;

pub use client::*;
pub use store::*;
pub use utils::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for embedding operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Dimensionality of embedding vectors
    pub dimensions: usize,
    /// Model name/path
    pub model: String,
    /// Batch size for efficient processing
    pub batch_size: usize,
    /// Maximum number of vectors to store
    pub max_vectors: usize,
    /// Distance metric for similarity search
    pub distance_metric: DistanceMetric,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimensions: 384, // all-MiniLM-L6-v2 default
            model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            batch_size: 32,
            max_vectors: 100_000,
            distance_metric: DistanceMetric::Cosine,
        }
    }
}

/// Distance metrics for vector similarity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DistanceMetric {
    /// Cosine similarity (default for semantic search)
    Cosine,
    /// Euclidean (L2) distance
    Euclidean,
    /// Manhattan (L1) distance
    Manhattan,
    /// Dot product similarity
    DotProduct,
}

/// A stored vector with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVector {
    /// Unique identifier for this vector
    pub id: String,
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Original text that was embedded
    pub text: String,
    /// Timestamp when this was created
    pub timestamp: u64,
    /// Importance score for memory consolidation
    pub importance: f32,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// A search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The stored vector
    pub vector: StoredVector,
    /// Similarity score (higher = more similar)
    pub score: f32,
    /// Distance value (lower = more similar)
    pub distance: f32,
}

/// Memory for game experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier
    pub id: String,
    /// Text description of the memory
    pub text: String,
    /// When this memory was created
    pub timestamp: u64,
    /// Importance for retention (0.0 to 1.0)
    pub importance: f32,
    /// Emotional valence (-1.0 to 1.0)
    pub valence: f32,
    /// Memory category
    pub category: MemoryCategory,
    /// Associated entity IDs
    pub entities: Vec<String>,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Categories of game memories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum MemoryCategory {
    /// Interactions with other entities
    Social,
    /// Combat encounters
    Combat,
    /// Exploration discoveries
    Exploration,
    /// Quest-related events
    Quest,
    /// Dialogue conversations
    Dialogue,
    /// General gameplay events
    Gameplay,
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetrics {
    /// Total embeddings created
    pub total_embeddings: u64,
    /// Average embedding time (ms)
    pub avg_embedding_time_ms: f32,
    /// Total searches performed
    pub total_searches: u64,
    /// Average search time (ms)
    pub avg_search_time_ms: f32,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.dimensions, 384);
        assert_eq!(config.batch_size, 32);
    }

    #[tokio::test]
    async fn test_memory_creation() {
        let memory = Memory {
            id: "test_memory".to_string(),
            text: "Player defeated the dragon".to_string(),
            timestamp: 1234567890,
            importance: 0.8,
            valence: 0.5,
            category: MemoryCategory::Combat,
            entities: vec!["player_1".to_string(), "dragon_boss".to_string()],
            context: HashMap::new(),
        };

        assert_eq!(memory.importance, 0.8);
        assert_eq!(memory.entities.len(), 2);
    }
}
