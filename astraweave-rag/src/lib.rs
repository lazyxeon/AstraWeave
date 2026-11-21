#![forbid(unsafe_code)]
/*!
# AstraWeave RAG Pipeline

Retrieval Augmented Generation (RAG) pipeline for AI-native gaming. This crate provides:

- **Memory Retrieval**: Semantic search through game experiences and conversations
- **Context Injection**: Smart integration of retrieved memories into prompts
- **Memory Consolidation**: Long-term memory formation from short-term experiences
- **Forgetting Mechanisms**: Importance-based memory decay and cleanup
- **Multi-Modal RAG**: Support for different types of game content

## Quick Start

```rust
use astraweave_rag::{RagPipeline, RagConfig, VectorStoreWrapper};
use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
use astraweave_llm::MockLlm;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create components
    let embedding_client = Arc::new(MockEmbeddingClient::new());
    let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
    let llm_client = Arc::new(MockLlm);

    // Create RAG pipeline
    let config = RagConfig::default();
    let mut rag = RagPipeline::new(
        embedding_client,
        vector_store,
        Some(llm_client),
        config
    );

    // Add some memories
    rag.add_memory("Player defeated the dragon boss in epic battle".to_string()).await?;
    rag.add_memory("Found magical sword in hidden cave".to_string()).await?;

    // Retrieve relevant memories
    let memories = rag.retrieve("combat with boss", 3).await?;
    println!("Retrieved {} relevant memories", memories.len());

    // Inject into prompt
    let base_prompt = "You are a game companion. Help the player with their quest.";
    let enhanced_prompt = rag.inject_context(base_prompt, "boss fight strategy").await?;
    println!("Enhanced prompt: {}", enhanced_prompt);

    Ok(())
}
```
*/

pub mod consolidation;
pub mod forgetting;
pub mod injection;
pub mod pipeline;
pub mod retrieval;

pub use consolidation::*;
pub use forgetting::*;
pub use injection::*;
pub use pipeline::*;
pub use retrieval::*;

use astraweave_embeddings::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for RAG pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Maximum number of memories to retrieve
    pub max_retrieval_count: usize,

    /// Minimum similarity score for retrieval
    pub min_similarity_score: f32,

    /// Memory consolidation settings
    pub consolidation: ConsolidationConfig,

    /// Forgetting mechanism settings
    pub forgetting: ForgettingConfig,

    /// Context injection settings
    pub injection: InjectionConfig,

    /// Diversity settings for retrieval
    pub diversity: DiversityConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_retrieval_count: 10,
            min_similarity_score: 0.3,
            consolidation: ConsolidationConfig::default(),
            forgetting: ForgettingConfig::default(),
            injection: InjectionConfig::default(),
            diversity: DiversityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// Context injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Template for injecting memories into prompts
    pub injection_template: String,

    /// Maximum tokens for injected context
    pub max_context_tokens: usize,

    /// Whether to include memory metadata
    pub include_metadata: bool,

    /// Strategy for ordering retrieved memories
    pub ordering_strategy: OrderingStrategy,

    /// Whether to summarize long memory lists
    pub enable_summarization: bool,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            injection_template: "Relevant memories:\n{memories}\n\nNow respond to: {query}"
                .to_string(),
            max_context_tokens: 1024,
            include_metadata: true,
            ordering_strategy: OrderingStrategy::SimilarityDesc,
            enable_summarization: true,
        }
    }
}

/// Strategies for ordering retrieved memories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum OrderingStrategy {
    /// Order by similarity (highest first)
    SimilarityDesc,
    /// Order by similarity (lowest first)
    SimilarityAsc,
    /// Order by recency (newest first)
    RecencyDesc,
    /// Order by recency (oldest first)
    RecencyAsc,
    /// Order by importance (highest first)
    ImportanceDesc,
    /// Order by importance (lowest first)
    ImportanceAsc,
    /// Mixed ordering for diversity
    Mixed,
}

/// Diversity configuration for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityConfig {
    /// Enable diversity in retrieval results
    pub enabled: bool,

    /// Diversity factor (0.0 = no diversity, 1.0 = maximum diversity)
    pub diversity_factor: f32,

    /// Diversity strategy
    pub strategy: DiversityStrategy,

    /// Minimum distance between diverse results
    pub min_diversity_distance: f32,
}

impl Default for DiversityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            diversity_factor: 0.3,
            strategy: DiversityStrategy::Semantic,
            min_diversity_distance: 0.2,
        }
    }
}

/// Strategies for ensuring diversity in retrieval
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum DiversityStrategy {
    /// Semantic diversity (different topics)
    Semantic,
    /// Temporal diversity (different time periods)
    Temporal,
    /// Category diversity (different memory types)
    Category,
    /// Combined diversity approach
    Combined,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable result caching
    pub enable_caching: bool,

    /// Cache size (number of cached queries)
    pub cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl: u64,

    /// Batch size for memory processing
    pub batch_size: usize,

    /// Parallel processing threads
    pub max_threads: usize,

    /// Enable performance metrics
    pub enable_metrics: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_size: 1000,
            cache_ttl: 300, // 5 minutes
            batch_size: 32,
            max_threads: 4,
            enable_metrics: true,
        }
    }
}

/// Retrieved memory with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedMemory {
    /// The memory content
    pub memory: Memory,

    /// Similarity score to query
    pub similarity_score: f32,

    /// Relevance ranking
    pub rank: usize,

    /// Additional retrieval metadata
    pub metadata: RetrievalMetadata,
}

/// Metadata about memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalMetadata {
    /// Query used for retrieval
    pub query: String,

    /// Retrieval method used
    pub method: RetrievalMethod,

    /// Timestamp of retrieval
    pub retrieved_at: u64,

    /// Processing time in milliseconds
    pub processing_time_ms: f32,

    /// Additional context about retrieval
    pub context: HashMap<String, serde_json::Value>,
}

/// Methods used for memory retrieval
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum RetrievalMethod {
    /// Semantic similarity search
    SemanticSearch,
    /// Keyword-based search
    KeywordSearch,
    /// Temporal search (recent memories)
    TemporalSearch,
    /// Category-based search
    CategorySearch,
    /// Hybrid search combining multiple methods
    HybridSearch,
}

/// RAG pipeline performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RagMetrics {
    /// Total queries processed
    pub total_queries: u64,

    /// Successful retrievals
    pub successful_retrievals: u64,

    /// Failed retrievals
    pub failed_retrievals: u64,

    /// Average retrieval time (ms)
    pub avg_retrieval_time_ms: f32,

    /// Average memories per query
    pub avg_memories_per_query: f32,

    /// Cache hit rate
    pub cache_hit_rate: f32,

    /// Memory consolidations performed
    pub consolidations_performed: u64,

    /// Memories forgotten
    pub memories_forgotten: u64,

    /// Total memories stored
    pub total_memories_stored: u64,

    /// Average memory importance
    pub avg_memory_importance: f32,
}

/// Context injection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionResult {
    /// The enhanced prompt with injected context
    pub enhanced_prompt: String,

    /// Memories that were injected
    pub injected_memories: Vec<RetrievedMemory>,

    /// Token count of injected context
    pub context_tokens: usize,

    /// Injection metadata
    pub metadata: InjectionMetadata,
}

/// Metadata about context injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionMetadata {
    /// Original prompt
    pub original_prompt: String,

    /// Query used for retrieval
    pub query: String,

    /// Injection strategy used
    pub strategy: InjectionStrategy,

    /// Processing time
    pub processing_time_ms: f32,

    /// Whether summarization was applied
    pub summarized: bool,
}

/// Strategies for injecting context into prompts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum InjectionStrategy {
    /// Prepend memories before the prompt
    Prepend,
    /// Append memories after the prompt
    Append,
    /// Insert memories at specific markers
    Insert,
    /// Interleave memories throughout the prompt
    Interleave,
    /// Replace prompt sections with memories
    Replace,
}

/// Memory query with advanced filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Text query for semantic search
    pub text: String,

    /// Time range filter
    pub time_range: Option<(u64, u64)>,

    /// Category filter
    pub categories: Vec<MemoryCategory>,

    /// Entity filter (involved characters/objects)
    pub entities: Vec<String>,

    /// Minimum importance threshold
    pub min_importance: Option<f32>,

    /// Maximum age in seconds
    pub max_age: Option<u64>,

    /// Custom metadata filters
    pub metadata_filters: HashMap<String, serde_json::Value>,
}

impl MemoryQuery {
    /// Create a simple text query
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            time_range: None,
            categories: Vec::new(),
            entities: Vec::new(),
            min_importance: None,
            max_age: None,
            metadata_filters: HashMap::new(),
        }
    }

    /// Add category filter
    pub fn with_category(mut self, category: MemoryCategory) -> Self {
        self.categories.push(category);
        self
    }

    /// Add entity filter
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entities.push(entity.into());
        self
    }

    /// Add importance filter
    pub fn with_min_importance(mut self, importance: f32) -> Self {
        self.min_importance = Some(importance);
        self
    }

    /// Add time range filter
    pub fn with_time_range(mut self, start: u64, end: u64) -> Self {
        self.time_range = Some((start, end));
        self
    }
}

/// Get current Unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_config() {
        let config = RagConfig::default();
        assert_eq!(config.max_retrieval_count, 10);
        assert!(config.consolidation.enabled);
        assert!(config.forgetting.enabled);
    }

    #[test]
    fn test_memory_query() {
        let query = MemoryQuery::text("combat")
            .with_category(MemoryCategory::Combat)
            .with_entity("player")
            .with_min_importance(0.5);

        assert_eq!(query.text, "combat");
        assert_eq!(query.categories.len(), 1);
        assert_eq!(query.entities.len(), 1);
        assert_eq!(query.min_importance, Some(0.5));
    }

    #[test]
    fn test_consolidation_strategy() {
        let strategies = [
            ConsolidationStrategy::Importance,
            ConsolidationStrategy::Recency,
            ConsolidationStrategy::Similarity,
            ConsolidationStrategy::Hybrid,
        ];

        assert_eq!(strategies.len(), 4);
    }

    #[test]
    fn test_rag_metrics() {
        let mut metrics = RagMetrics::default();
        assert_eq!(metrics.total_queries, 0);

        metrics.total_queries = 100;
        metrics.successful_retrievals = 95;
        metrics.failed_retrievals = 5;

        let success_rate = metrics.successful_retrievals as f32 / metrics.total_queries as f32;
        assert_eq!(success_rate, 0.95);
    }

    #[test]
    fn test_diversity_config() {
        let config = DiversityConfig::default();
        assert!(config.enabled);
        assert_eq!(config.diversity_factor, 0.3);
        assert!(config.min_diversity_distance > 0.0);
    }

    #[test]
    fn test_injection_strategies() {
        let strategies = [
            InjectionStrategy::Prepend,
            InjectionStrategy::Append,
            InjectionStrategy::Insert,
            InjectionStrategy::Interleave,
            InjectionStrategy::Replace,
        ];

        assert_eq!(strategies.len(), 5);
    }
}

#[cfg(test)]
mod unix_instant;
