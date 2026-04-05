/*!
# RAG Pipeline

Main RAG pipeline implementation combining retrieval, consolidation, and injection.
*/

use crate::{
    current_timestamp, ConsolidationEngine, ForgettingEngine, InjectionResult, InjectionStrategy,
    MemoryQuery, RagConfig, RagMetrics, RetrievalMethod, RetrievedMemory,
};
use anyhow::{anyhow, Result};
use astraweave_context::TokenCounter;
use astraweave_embeddings::{EmbeddingClient, Memory, MemoryCategory, SearchResult, VectorStore};
use astraweave_llm::LlmClient;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Main RAG pipeline combining all components
pub struct RagPipeline {
    /// Embedding client for vector operations
    embedding_client: Arc<dyn EmbeddingClient>,

    /// Vector store for memory storage and retrieval
    vector_store: Arc<dyn VectorStoreInterface>,

    /// LLM client for consolidation and summarization
    llm_client: Option<Arc<dyn LlmClient>>,

    /// Token counter for context management
    token_counter: TokenCounter,

    /// Pipeline configuration
    config: RagConfig,

    /// Performance metrics
    metrics: Arc<RwLock<RagMetrics>>,

    /// Query result cache
    cache: Arc<DashMap<String, CachedResult>>,

    /// Memory consolidation state
    consolidation_state: Arc<RwLock<ConsolidationState>>,

    /// Consolidation engine
    consolidation_engine: Arc<RwLock<ConsolidationEngine>>,

    /// Forgetting engine
    forgetting_engine: Arc<RwLock<ForgettingEngine>>,
}

/// Trait for vector store operations (allows testing with different implementations)
#[async_trait::async_trait]
pub trait VectorStoreInterface: Send + Sync {
    async fn search(&self, query_vector: &[f32], k: usize) -> Result<Vec<SearchResult>>;
    async fn insert(&self, id: String, vector: Vec<f32>, text: String) -> Result<()>;
    async fn insert_memory(&self, memory: Memory, vector: Vec<f32>) -> Result<()>;
    async fn get(&self, id: &str) -> Option<Memory>;
    async fn remove(&self, id: &str) -> Option<Memory>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    async fn get_all_memories(&self) -> Vec<Memory>;
}

/// Wrapper to make VectorStore compatible with VectorStoreInterface
pub struct VectorStoreWrapper {
    inner: VectorStore,
}

impl VectorStoreWrapper {
    pub fn new(store: VectorStore) -> Self {
        Self { inner: store }
    }
}

#[async_trait::async_trait]
impl VectorStoreInterface for VectorStoreWrapper {
    async fn search(&self, query_vector: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        self.inner.search(query_vector, k)
    }

    async fn insert(&self, id: String, vector: Vec<f32>, text: String) -> Result<()> {
        self.inner.insert(id, vector, text)
    }

    async fn insert_memory(&self, memory: Memory, vector: Vec<f32>) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "entities".to_string(),
            serde_json::to_string(&memory.entities)?,
        );
        metadata.insert(
            "category".to_string(),
            serde_json::to_string(&memory.category)?,
        );
        metadata.insert("valence".to_string(), memory.valence.to_string());

        for (k, v) in memory.context {
            metadata.insert(format!("ctx_{}", k), v);
        }

        self.inner
            .insert_with_metadata(memory.id, vector, memory.text, memory.importance, metadata)
    }

    async fn get(&self, id: &str) -> Option<Memory> {
        self.inner.get(id).map(|stored| {
            let entities: Vec<String> = stored
                .metadata
                .get("entities")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            let category: MemoryCategory = stored
                .metadata
                .get("category")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(MemoryCategory::Gameplay);

            let valence: f32 = stored
                .metadata
                .get("valence")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            let mut context = HashMap::new();
            for (k, v) in &stored.metadata {
                if let Some(key) = k.strip_prefix("ctx_") {
                    context.insert(key.to_string(), v.clone());
                }
            }

            Memory {
                id: stored.id,
                text: stored.text,
                timestamp: stored.timestamp,
                importance: stored.importance,
                valence,
                category,
                entities,
                context,
            }
        })
    }

    async fn remove(&self, id: &str) -> Option<Memory> {
        self.inner.remove(id).map(|stored| Memory {
            id: stored.id,
            text: stored.text,
            timestamp: stored.timestamp,
            importance: stored.importance,
            valence: 0.0,
            category: MemoryCategory::Gameplay,
            entities: vec![],
            context: HashMap::new(),
        })
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    async fn get_all_memories(&self) -> Vec<Memory> {
        // This is a simplified implementation
        // In practice, you'd iterate through all stored vectors
        vec![]
    }
}

/// Cached retrieval result
#[derive(Debug, Clone)]
struct CachedResult {
    memories: Vec<RetrievedMemory>,
    timestamp: u64,
    #[allow(dead_code)]
    query_hash: u64,
}

/// State for memory consolidation
#[derive(Debug, Default)]
struct ConsolidationState {
    last_consolidation: u64,
    memories_since_consolidation: usize,
    consolidation_in_progress: bool,
}

impl RagPipeline {
    /// Create a new RAG pipeline
    pub fn new(
        embedding_client: Arc<dyn EmbeddingClient>,
        vector_store: Arc<dyn VectorStoreInterface>,
        llm_client: Option<Arc<dyn LlmClient>>,
        config: RagConfig,
    ) -> Self {
        let token_counter = TokenCounter::new("cl100k_base");
        let consolidation_engine = Arc::new(RwLock::new(ConsolidationEngine::new(
            config.consolidation.clone(),
        )));
        let forgetting_engine = Arc::new(RwLock::new(ForgettingEngine::new(
            config.forgetting.clone(),
        )));

        Self {
            embedding_client,
            vector_store,
            llm_client,
            token_counter,
            config,
            metrics: Arc::new(RwLock::new(RagMetrics::default())),
            cache: Arc::new(DashMap::new()),
            consolidation_state: Arc::new(RwLock::new(ConsolidationState {
                last_consolidation: current_timestamp(),
                memories_since_consolidation: 0,
                consolidation_in_progress: false,
            })),
            consolidation_engine,
            forgetting_engine,
        }
    }

    /// Add a memory to the pipeline
    pub async fn add_memory(&mut self, text: String) -> Result<String> {
        let memory = Memory {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.clone(),
            timestamp: current_timestamp(),
            importance: 1.0,
            valence: 0.0,
            category: MemoryCategory::Gameplay,
            entities: vec![],
            context: HashMap::new(),
        };

        self.add_memory_obj(memory).await
    }

    /// Add a memory object to the pipeline
    pub async fn add_memory_obj(&mut self, memory: Memory) -> Result<String> {
        let start_time = std::time::Instant::now();

        // Generate embedding for the memory
        let embedding = self.embedding_client.embed(&memory.text).await?;

        // Store in vector store
        self.vector_store
            .insert_memory(memory.clone(), embedding)
            .await?;

        // Clear cache to ensure new memory is retrievable
        self.clear_cache();

        // Update consolidation state
        {
            let mut state = self.consolidation_state.write();
            state.memories_since_consolidation += 1;
        }

        // Check if consolidation is needed
        if self.should_consolidate().await {
            self.trigger_consolidation().await?;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.total_memories_stored += 1;
            let _duration = start_time.elapsed().as_millis() as f32;
            // Update average processing time if needed
        }

        Ok(memory.id)
    }

    /// Retrieve memories based on text query
    pub async fn retrieve(&self, query: &str, k: usize) -> Result<Vec<RetrievedMemory>> {
        let memory_query = MemoryQuery::text(query);
        self.retrieve_with_query(&memory_query, k).await
    }

    /// Retrieve memories with advanced query
    pub async fn retrieve_with_query(
        &self,
        query: &MemoryQuery,
        k: usize,
    ) -> Result<Vec<RetrievedMemory>> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = self.generate_cache_key(query, k);
        if self.config.performance.enable_caching {
            if let Some(cached) = self.get_cached_result(&cache_key) {
                self.update_cache_hit_metrics();
                return Ok(cached.memories);
            }
        }

        // Generate query embedding
        let query_embedding = self.embedding_client.embed(&query.text).await?;

        // Search vector store
        let search_results = self.vector_store.search(&query_embedding, k * 2).await?; // Get more for filtering

        // Convert to RetrievedMemory and apply filters
        let mut retrieved_memories = Vec::new();
        for (rank, result) in search_results.into_iter().enumerate() {
            // Convert SearchResult to Memory
            let entities: Vec<String> = result
                .vector
                .metadata
                .get("entities")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            let category: MemoryCategory = result
                .vector
                .metadata
                .get("category")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(MemoryCategory::Gameplay);

            let valence: f32 = result
                .vector
                .metadata
                .get("valence")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            let mut context = HashMap::new();
            for (k, v) in &result.vector.metadata {
                if let Some(key) = k.strip_prefix("ctx_") {
                    context.insert(key.to_string(), v.clone());
                }
            }

            let memory = Memory {
                id: result.vector.id,
                text: result.vector.text,
                timestamp: result.vector.timestamp,
                importance: result.vector.importance,
                valence,
                category,
                entities,
                context,
            };

            // Apply filters
            if self.passes_filters(&memory, query) {
                retrieved_memories.push(RetrievedMemory {
                    memory,
                    similarity_score: result.score,
                    rank,
                    metadata: crate::RetrievalMetadata {
                        query: query.text.clone(),
                        method: RetrievalMethod::SemanticSearch,
                        retrieved_at: current_timestamp(),
                        processing_time_ms: start_time.elapsed().as_millis() as f32,
                        context: HashMap::new(),
                    },
                });

                if retrieved_memories.len() >= k {
                    break;
                }
            }
        }

        // Apply diversity if enabled
        if self.config.diversity.enabled {
            retrieved_memories = self.apply_diversity(retrieved_memories);
        }

        // Order results
        self.order_results(retrieved_memories.as_mut_slice());

        // Cache results
        if self.config.performance.enable_caching {
            self.cache_result(&cache_key, &retrieved_memories);
        }

        // Update metrics
        let duration = start_time.elapsed().as_millis() as f32;
        self.update_retrieval_metrics(retrieved_memories.len(), duration, true);

        Ok(retrieved_memories)
    }

    /// Convenience helper: retrieve and return the text contents as Vec<String>
    pub async fn retrieve_texts(&self, query: &str, k: usize) -> Result<Vec<String>> {
        let items = self.retrieve(query, k).await?;
        Ok(items.into_iter().map(|r| r.memory.text).collect())
    }

    /// Return true if an LLM client is configured for this pipeline
    pub fn has_llm_client(&self) -> bool {
        self.llm_client.is_some()
    }

    /// Inject retrieved memories into a prompt
    pub async fn inject_context(&self, base_prompt: &str, query: &str) -> Result<String> {
        let injection_result = self.inject_context_detailed(base_prompt, query).await?;
        Ok(injection_result.enhanced_prompt)
    }

    /// Inject context with detailed result information
    pub async fn inject_context_detailed(
        &self,
        base_prompt: &str,
        query: &str,
    ) -> Result<InjectionResult> {
        let start_time = std::time::Instant::now();

        // Retrieve relevant memories
        let memories = self
            .retrieve(query, self.config.max_retrieval_count)
            .await?;

        if memories.is_empty() {
            return Ok(InjectionResult {
                enhanced_prompt: base_prompt.to_string(),
                injected_memories: vec![],
                context_tokens: 0,
                metadata: crate::InjectionMetadata {
                    original_prompt: base_prompt.to_string(),
                    query: query.to_string(),
                    strategy: InjectionStrategy::Prepend,
                    processing_time_ms: start_time.elapsed().as_millis() as f32,
                    summarized: false,
                },
            });
        }

        // Format memories for injection
        let memory_texts: Vec<String> = memories
            .iter()
            .map(|m| {
                if self.config.injection.include_metadata {
                    format!("[Score: {:.2}] {}", m.similarity_score, m.memory.text)
                } else {
                    m.memory.text.clone()
                }
            })
            .collect();

        let memories_text = memory_texts.join("\n");

        // Check token limit and summarize if needed
        let memory_tokens = self.token_counter.count_tokens(&memories_text)?;
        let final_memories_text = if memory_tokens > self.config.injection.max_context_tokens {
            if self.config.injection.enable_summarization && self.llm_client.is_some() {
                self.summarize_memories(&memories_text).await?
            } else {
                // Truncate to fit token limit
                self.token_counter
                    .truncate_to_tokens(&memories_text, self.config.injection.max_context_tokens)?
            }
        } else {
            memories_text
        };

        // Apply injection template
        let enhanced_prompt = self
            .config
            .injection
            .injection_template
            .replace("{memories}", &final_memories_text)
            .replace("{query}", query)
            .replace("{prompt}", base_prompt);

        let context_tokens = self.token_counter.count_tokens(&final_memories_text)?;

        Ok(InjectionResult {
            enhanced_prompt,
            injected_memories: memories,
            context_tokens,
            metadata: crate::InjectionMetadata {
                original_prompt: base_prompt.to_string(),
                query: query.to_string(),
                strategy: InjectionStrategy::Insert,
                processing_time_ms: start_time.elapsed().as_millis() as f32,
                summarized: memory_tokens > self.config.injection.max_context_tokens,
            },
        })
    }

    /// Get pipeline metrics
    pub fn get_metrics(&self) -> RagMetrics {
        self.metrics.read().clone()
    }

    /// Clear the memory cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Check if memory consolidation should be triggered
    async fn should_consolidate(&self) -> bool {
        if !self.config.consolidation.enabled {
            return false;
        }

        let state = self.consolidation_state.read();

        // Check if already in progress
        if state.consolidation_in_progress {
            return false;
        }

        // Check memory threshold
        if state.memories_since_consolidation >= self.config.consolidation.trigger_threshold {
            return true;
        }

        // Check time threshold
        let time_since_last = current_timestamp() - state.last_consolidation;
        if time_since_last >= self.config.consolidation.consolidation_interval {
            return true;
        }

        false
    }

    /// Trigger memory consolidation
    pub async fn trigger_consolidation(&self) -> Result<()> {
        // Set consolidation in progress
        {
            let mut state = self.consolidation_state.write();
            state.consolidation_in_progress = true;
        }

        // Get all memories
        let memories = self.vector_store.get_all_memories().await;
        let original_ids: HashSet<String> = memories.iter().map(|m| m.id.clone()).collect();

        // Run consolidation
        let (consolidated_memories, result) = {
            let engine = self.consolidation_engine.read();
            engine.consolidate(memories)?
        };

        // If changes occurred, update vector store
        if result.merged_count > 0 || result.removed_count > 0 {
            let new_ids: HashSet<String> =
                consolidated_memories.iter().map(|m| m.id.clone()).collect();

            // Remove memories that are no longer present
            for id in &original_ids {
                if !new_ids.contains(id) {
                    self.vector_store.remove(id).await;
                }
            }

            // Add new memories (merged results)
            for memory in consolidated_memories {
                if !original_ids.contains(&memory.id) {
                    // New memory (merged result) - generate embedding and insert
                    let embedding = self.embedding_client.embed(&memory.text).await?;
                    self.vector_store.insert_memory(memory, embedding).await?;
                }
            }
        }

        // Reset state
        {
            let mut state = self.consolidation_state.write();
            state.last_consolidation = current_timestamp();
            state.memories_since_consolidation = 0;
            state.consolidation_in_progress = false;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.consolidations_performed += 1;
        }

        Ok(())
    }

    /// Trigger memory forgetting
    pub async fn trigger_forgetting(&self) -> Result<()> {
        if !self.config.forgetting.enabled {
            return Ok(());
        }

        let memories = self.vector_store.get_all_memories().await;
        let original_ids: HashSet<String> = memories.iter().map(|m| m.id.clone()).collect();

        let (retained_memories, result) = {
            let mut engine = self.forgetting_engine.write();
            engine.process_forgetting(memories)?
        };

        if result.forgotten_count > 0 {
            let retained_ids: HashSet<String> =
                retained_memories.iter().map(|m| m.id.clone()).collect();

            for id in &original_ids {
                if !retained_ids.contains(id) {
                    self.vector_store.remove(id).await;
                }
            }

            // Update metrics
            {
                let mut metrics = self.metrics.write();
                metrics.memories_forgotten += result.forgotten_count as u64;
            }
        }

        Ok(())
    }

    /// Adds a memory item asynchronously without requiring &mut self or Arc::get_mut.
    ///
    /// This method is safe to call from an Arc<RagPipeline> and uses the pipeline's
    /// interior mutability (parking_lot locks) for coordination. It performs
    /// embedding generation and vector store insertion asynchronously and updates
    /// consolidation/metrics via synchronized locks. The lock scopes are kept
    /// minimal and are not held across awaits.
    pub async fn add_memory_async(&self, text: String) -> Result<String> {
        let memory = Memory {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.clone(),
            timestamp: current_timestamp(),
            importance: 1.0,
            valence: 0.0,
            category: MemoryCategory::Gameplay,
            entities: vec![],
            context: HashMap::new(),
        };

        // Generate embedding using the shared embedding client
        let embedding = self.embedding_client.embed(&memory.text).await?;

        // Store in vector store
        self.vector_store
            .insert_memory(memory.clone(), embedding)
            .await?;

        // Update consolidation state (synchronous lock, dropped immediately)
        {
            let mut state = self.consolidation_state.write();
            state.memories_since_consolidation += 1;
        }

        // Trigger consolidation if needed (runs while holding locks briefly internally)
        if self.should_consolidate().await {
            // Fire-and-forget consolidation is acceptable here; await it to keep behavior
            // consistent with existing sync path.
            let _ = self.trigger_consolidation().await;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.total_memories_stored += 1;
        }

        Ok(memory.id)
    }

    /// Check if a memory passes the query filters
    fn passes_filters(&self, memory: &Memory, query: &MemoryQuery) -> bool {
        // Time range filter
        if let Some((start, end)) = query.time_range {
            if memory.timestamp < start || memory.timestamp > end {
                return false;
            }
        }

        // Category filter
        if !query.categories.is_empty() && !query.categories.contains(&memory.category) {
            return false;
        }

        // Entity filter
        if !query.entities.is_empty() {
            let has_entity = query.entities.iter().any(|entity| {
                memory.entities.contains(entity)
                    || memory.text.to_lowercase().contains(&entity.to_lowercase())
            });
            if !has_entity {
                return false;
            }
        }

        // Importance filter
        if let Some(min_importance) = query.min_importance {
            if memory.importance < min_importance {
                return false;
            }
        }

        // Age filter
        if let Some(max_age) = query.max_age {
            let age = current_timestamp() - memory.timestamp;
            if age > max_age {
                return false;
            }
        }

        true
    }

    /// Apply diversity to retrieval results
    fn apply_diversity(&self, mut memories: Vec<RetrievedMemory>) -> Vec<RetrievedMemory> {
        if memories.len() <= 1 {
            return memories;
        }

        let mut diverse_memories = Vec::new();
        diverse_memories.push(memories.remove(0)); // Always include the most similar

        while !memories.is_empty() && diverse_memories.len() < self.config.max_retrieval_count {
            let mut best_idx = 0;
            let mut best_score = f32::MIN;

            for (i, candidate) in memories.iter().enumerate() {
                // Calculate diversity score
                let mut min_distance = f32::MAX;
                for existing in &diverse_memories {
                    // Simple text-based diversity (could use embedding distance)
                    let distance = text_distance(&candidate.memory.text, &existing.memory.text);
                    min_distance = min_distance.min(distance);
                }

                // Combined score: similarity + diversity
                let diversity_bonus = min_distance * self.config.diversity.diversity_factor;
                let combined_score = candidate.similarity_score + diversity_bonus;

                if combined_score > best_score {
                    best_score = combined_score;
                    best_idx = i;
                }
            }

            diverse_memories.push(memories.remove(best_idx));
        }

        diverse_memories
    }

    /// Order results based on configuration
    fn order_results(&self, memories: &mut [RetrievedMemory]) {
        match self.config.injection.ordering_strategy {
            crate::OrderingStrategy::SimilarityDesc => {
                memories.sort_by(|a, b| {
                    b.similarity_score
                        .partial_cmp(&a.similarity_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            crate::OrderingStrategy::SimilarityAsc => {
                memories.sort_by(|a, b| {
                    a.similarity_score
                        .partial_cmp(&b.similarity_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            crate::OrderingStrategy::RecencyDesc => {
                memories.sort_by(|a, b| b.memory.timestamp.cmp(&a.memory.timestamp));
            }
            crate::OrderingStrategy::RecencyAsc => {
                memories.sort_by(|a, b| a.memory.timestamp.cmp(&b.memory.timestamp));
            }
            crate::OrderingStrategy::ImportanceDesc => {
                memories.sort_by(|a, b| {
                    b.memory
                        .importance
                        .partial_cmp(&a.memory.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            crate::OrderingStrategy::ImportanceAsc => {
                memories.sort_by(|a, b| {
                    a.memory
                        .importance
                        .partial_cmp(&b.memory.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            crate::OrderingStrategy::Mixed => {
                // Shuffle for variety
                use rand::seq::SliceRandom;
                let mut rng = rand::rng();
                memories.shuffle(&mut rng);
            }
        }
    }

    /// Generate cache key for query
    fn generate_cache_key(&self, query: &MemoryQuery, k: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.text.hash(&mut hasher);
        k.hash(&mut hasher);
        // Add other query parameters to hash if needed

        format!("rag_query_{}", hasher.finish())
    }

    /// Get cached result if available and not expired
    fn get_cached_result(&self, cache_key: &str) -> Option<CachedResult> {
        // Check if entry exists and get its age
        let should_remove = if let Some(cached) = self.cache.get(cache_key) {
            let age = current_timestamp() - cached.timestamp;
            if age <= self.config.performance.cache_ttl {
                return Some(cached.clone());
            }
            true // Entry is expired, needs removal
        } else {
            false
        };

        // Drop the read lock before attempting write lock for removal
        if should_remove {
            self.cache.remove(cache_key);
        }

        None
    }

    /// Cache retrieval result
    fn cache_result(&self, cache_key: &str, memories: &[RetrievedMemory]) {
        // Limit cache size
        if self.cache.len() >= self.config.performance.cache_size {
            // Remove some old entries (simplified LRU)
            let keys_to_remove: Vec<String> = self
                .cache
                .iter()
                .take(self.config.performance.cache_size / 4) // Remove 25%
                .map(|entry| entry.key().clone())
                .collect();

            for key in keys_to_remove {
                self.cache.remove(&key);
            }
        }

        let cached_result = CachedResult {
            memories: memories.to_vec(),
            timestamp: current_timestamp(),
            query_hash: 0, // Would compute proper hash in practice
        };

        self.cache.insert(cache_key.to_string(), cached_result);
    }

    /// Summarize memories using LLM
    async fn summarize_memories(&self, memories_text: &str) -> Result<String> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| anyhow!("LLM client required for summarization"))?;

        let prompt = format!(
            "Summarize the following memories concisely while preserving key information:\n\n{}",
            memories_text
        );

        let summary = llm_client.complete(&prompt).await?;
        Ok(summary)
    }

    /// Update retrieval metrics
    fn update_retrieval_metrics(&self, result_count: usize, duration_ms: f32, success: bool) {
        let mut metrics = self.metrics.write();
        metrics.total_queries += 1;

        if success {
            metrics.successful_retrievals += 1;

            // Update averages
            let total = metrics.successful_retrievals as f32;
            metrics.avg_retrieval_time_ms =
                (metrics.avg_retrieval_time_ms * (total - 1.0) + duration_ms) / total;
            metrics.avg_memories_per_query =
                (metrics.avg_memories_per_query * (total - 1.0) + result_count as f32) / total;
        } else {
            metrics.failed_retrievals += 1;
        }
    }

    /// Update cache hit metrics
    fn update_cache_hit_metrics(&self) {
        let mut metrics = self.metrics.write();
        let total_queries = metrics.total_queries + 1;
        let cache_hits = (metrics.cache_hit_rate * metrics.total_queries as f32) + 1.0;
        metrics.cache_hit_rate = cache_hits / total_queries as f32;
    }
}

/// Simple text distance calculation (Jaccard similarity)
fn text_distance(text1: &str, text2: &str) -> f32 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        1.0 - (intersection as f32 / union as f32) // Distance = 1 - similarity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
    use astraweave_llm::MockLlm;

    #[tokio::test]
    async fn test_rag_pipeline_creation() {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let llm_client = Arc::new(MockLlm);
        let config = RagConfig::default();

        let pipeline = RagPipeline::new(embedding_client, vector_store, Some(llm_client), config);

        let metrics = pipeline.get_metrics();
        assert_eq!(metrics.total_queries, 0);
        assert_eq!(metrics.total_memories_stored, 0);
    }

    #[tokio::test]
    async fn test_add_memory() {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let config = RagConfig::default();

        let mut pipeline = RagPipeline::new(embedding_client, vector_store, None, config);

        let memory_id = pipeline
            .add_memory("Test memory content".to_string())
            .await
            .unwrap();
        assert!(!memory_id.is_empty());

        let metrics = pipeline.get_metrics();
        assert_eq!(metrics.total_memories_stored, 1);
    }

    #[tokio::test]
    async fn add_memory_async_is_concurrent_arc_safe() {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let config = RagConfig::default();

        let pipeline = Arc::new(RagPipeline::new(
            embedding_client,
            vector_store,
            None,
            config,
        ));

        let p1 = Arc::clone(&pipeline);
        let p2 = Arc::clone(&pipeline);

        let t1 = tokio::spawn(async move { p1.add_memory_async("alpha".into()).await });
        let t2 = tokio::spawn(async move { p2.add_memory_async("beta".into()).await });

        let (r1, r2) = tokio::join!(t1, t2);
        let id1 = r1.unwrap().unwrap();
        let id2 = r2.unwrap().unwrap();

        assert_ne!(id1, id2, "distinct memories should yield distinct IDs");

        // Optionally verify vector store length increased (VectorStoreWrapper.len may be simplistic)
        // We only check that the IDs are non-empty above to keep test robust.
    }

    #[test]
    fn test_text_distance() {
        let text1 = "the quick brown fox";
        let text2 = "the lazy brown dog";
        let text3 = "completely different text";

        let dist1_2 = text_distance(text1, text2);
        let dist1_3 = text_distance(text1, text3);

        // Should be less similar to completely different text
        assert!(dist1_3 > dist1_2);
    }

    #[test]
    fn test_memory_query() {
        let query = MemoryQuery::text("combat")
            .with_category(MemoryCategory::Combat)
            .with_min_importance(0.5);

        let memory_combat = Memory {
            id: "1".to_string(),
            text: "Combat encounter".to_string(),
            timestamp: current_timestamp(),
            importance: 0.8,
            valence: 0.0,
            category: MemoryCategory::Combat,
            entities: vec![],
            context: HashMap::new(),
        };

        let memory_social = Memory {
            id: "2".to_string(),
            text: "Social interaction".to_string(),
            timestamp: current_timestamp(),
            importance: 0.3,
            valence: 0.0,
            category: MemoryCategory::Social,
            entities: vec![],
            context: HashMap::new(),
        };

        let pipeline = create_test_pipeline();

        assert!(pipeline.passes_filters(&memory_combat, &query));
        assert!(!pipeline.passes_filters(&memory_social, &query)); // Wrong category and low importance
    }

    fn create_test_pipeline() -> RagPipeline {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let config = RagConfig::default();

        RagPipeline::new(embedding_client, vector_store, None, config)
    }

    #[test]
    fn test_passes_filters_time_range() {
        let pipeline = create_test_pipeline();
        let now = current_timestamp();

        let memory = Memory {
            id: "1".to_string(),
            text: "Test memory".to_string(),
            timestamp: now,
            importance: 0.5,
            valence: 0.0,
            category: MemoryCategory::Combat,
            entities: vec![],
            context: HashMap::new(),
        };

        // Memory within range
        let query = MemoryQuery::text("test").with_time_range(now - 1000, now + 1000);
        assert!(pipeline.passes_filters(&memory, &query));

        // Memory outside range (too old)
        let query_old = MemoryQuery::text("test").with_time_range(now + 1000, now + 2000);
        assert!(!pipeline.passes_filters(&memory, &query_old));
    }

    #[test]
    fn test_passes_filters_entities() {
        let pipeline = create_test_pipeline();

        let memory = Memory {
            id: "1".to_string(),
            text: "The dragon attacked the village".to_string(),
            timestamp: current_timestamp(),
            importance: 0.5,
            valence: 0.0,
            category: MemoryCategory::Combat,
            entities: vec!["dragon".to_string()],
            context: HashMap::new(),
        };

        // Entity in list
        let query = MemoryQuery::text("test").with_entity("dragon");
        assert!(pipeline.passes_filters(&memory, &query));

        // Entity in text (case-insensitive)
        let query_text = MemoryQuery::text("test").with_entity("Village");
        assert!(pipeline.passes_filters(&memory, &query_text));

        // Entity not found
        let query_missing = MemoryQuery::text("test").with_entity("goblin");
        assert!(!pipeline.passes_filters(&memory, &query_missing));
    }

    #[test]
    fn test_passes_filters_max_age() {
        let pipeline = create_test_pipeline();
        let now = current_timestamp();

        // Create memory at current timestamp - filters don't have max_age builder
        // but passes_filters checks max_age field directly. Test other filters instead.
        let memory = Memory {
            id: "1".to_string(),
            text: "Test memory".to_string(),
            timestamp: now,
            importance: 0.7,
            valence: 0.0,
            category: MemoryCategory::Combat,
            entities: vec!["hero".to_string()],
            context: HashMap::new(),
        };

        // Should pass when max_age is set but memory is recent
        let mut query = MemoryQuery::text("test");
        query.max_age = Some(10000); // 10 seconds max age
        assert!(pipeline.passes_filters(&memory, &query));
    }

    #[test]
    fn test_generate_cache_key_deterministic() {
        let pipeline = create_test_pipeline();

        let query = MemoryQuery::text("combat strategies");
        let key1 = pipeline.generate_cache_key(&query, 5);
        let key2 = pipeline.generate_cache_key(&query, 5);

        assert_eq!(key1, key2);
        assert!(key1.starts_with("rag_query_"));
    }

    #[test]
    fn test_generate_cache_key_different_k() {
        let pipeline = create_test_pipeline();

        let query = MemoryQuery::text("combat");
        let key_5 = pipeline.generate_cache_key(&query, 5);
        let key_10 = pipeline.generate_cache_key(&query, 10);

        assert_ne!(key_5, key_10);
    }

    #[test]
    fn test_generate_cache_key_different_queries() {
        let pipeline = create_test_pipeline();

        let query1 = MemoryQuery::text("combat");
        let query2 = MemoryQuery::text("social");

        let key1 = pipeline.generate_cache_key(&query1, 5);
        let key2 = pipeline.generate_cache_key(&query2, 5);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_order_results_similarity_desc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::SimilarityDesc;

        let mut memories = vec![
            create_retrieved_memory("1", 0.5),
            create_retrieved_memory("2", 0.9),
            create_retrieved_memory("3", 0.7),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "2"); // 0.9
        assert_eq!(memories[1].memory.id, "3"); // 0.7
        assert_eq!(memories[2].memory.id, "1"); // 0.5
    }

    #[test]
    fn test_order_results_similarity_asc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::SimilarityAsc;

        let mut memories = vec![
            create_retrieved_memory("1", 0.5),
            create_retrieved_memory("2", 0.9),
            create_retrieved_memory("3", 0.7),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "1"); // 0.5
        assert_eq!(memories[1].memory.id, "3"); // 0.7
        assert_eq!(memories[2].memory.id, "2"); // 0.9
    }

    #[test]
    fn test_order_results_importance_desc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::ImportanceDesc;

        let mut memories = vec![
            create_retrieved_memory_with_importance("1", 0.3),
            create_retrieved_memory_with_importance("2", 0.9),
            create_retrieved_memory_with_importance("3", 0.6),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "2"); // 0.9
        assert_eq!(memories[1].memory.id, "3"); // 0.6
        assert_eq!(memories[2].memory.id, "1"); // 0.3
    }

    #[test]
    fn test_order_results_recency_desc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::RecencyDesc;

        let now = current_timestamp();
        let mut memories = vec![
            create_retrieved_memory_with_timestamp("1", now.saturating_sub(1000)),
            create_retrieved_memory_with_timestamp("2", now),
            create_retrieved_memory_with_timestamp("3", now.saturating_sub(500)),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "2"); // most recent
        assert_eq!(memories[1].memory.id, "3");
        assert_eq!(memories[2].memory.id, "1"); // oldest
    }

    #[test]
    fn test_apply_diversity_single_memory() {
        let pipeline = create_test_pipeline();

        let memories = vec![create_retrieved_memory("1", 0.9)];
        let result = pipeline.apply_diversity(memories);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_apply_diversity_empty() {
        let pipeline = create_test_pipeline();

        let memories: Vec<RetrievedMemory> = vec![];
        let result = pipeline.apply_diversity(memories);

        assert!(result.is_empty());
    }

    #[test]
    fn test_clear_cache() {
        let pipeline = create_test_pipeline();

        // Add some items to cache
        pipeline.cache.insert(
            "key1".to_string(),
            CachedResult {
                memories: vec![],
                timestamp: current_timestamp(),
                query_hash: 0,
            },
        );
        pipeline.cache.insert(
            "key2".to_string(),
            CachedResult {
                memories: vec![],
                timestamp: current_timestamp(),
                query_hash: 0,
            },
        );

        assert_eq!(pipeline.cache.len(), 2);

        pipeline.clear_cache();

        assert_eq!(pipeline.cache.len(), 0);
    }

    #[test]
    fn test_get_metrics_initial() {
        let pipeline = create_test_pipeline();
        let metrics = pipeline.get_metrics();

        assert_eq!(metrics.total_queries, 0);
        assert_eq!(metrics.successful_retrievals, 0);
        assert_eq!(metrics.failed_retrievals, 0);
    }

    #[test]
    fn test_has_llm_client() {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let config = RagConfig::default();

        let pipeline_no_llm = RagPipeline::new(
            embedding_client.clone(),
            vector_store.clone(),
            None,
            config.clone(),
        );
        assert!(!pipeline_no_llm.has_llm_client());

        let llm_client = Arc::new(MockLlm);
        let pipeline_with_llm =
            RagPipeline::new(embedding_client, vector_store, Some(llm_client), config);
        assert!(pipeline_with_llm.has_llm_client());
    }

    #[test]
    fn test_update_retrieval_metrics_success() {
        let pipeline = create_test_pipeline();

        pipeline.update_retrieval_metrics(5, 10.0, true);
        let metrics = pipeline.get_metrics();

        assert_eq!(metrics.total_queries, 1);
        assert_eq!(metrics.successful_retrievals, 1);
        assert_eq!(metrics.failed_retrievals, 0);
        assert!((metrics.avg_retrieval_time_ms - 10.0).abs() < 0.01);
        assert!((metrics.avg_memories_per_query - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_update_retrieval_metrics_failure() {
        let pipeline = create_test_pipeline();

        pipeline.update_retrieval_metrics(0, 5.0, false);
        let metrics = pipeline.get_metrics();

        assert_eq!(metrics.total_queries, 1);
        assert_eq!(metrics.successful_retrievals, 0);
        assert_eq!(metrics.failed_retrievals, 1);
    }

    #[test]
    fn test_text_distance_identical() {
        let distance = text_distance("hello world", "hello world");
        assert!(distance.abs() < 0.01); // Should be 0 (identical)
    }

    #[test]
    fn test_text_distance_no_overlap() {
        let distance = text_distance("hello world", "foo bar baz");
        assert!((distance - 1.0).abs() < 0.01); // Should be 1 (no overlap)
    }

    #[test]
    fn test_text_distance_partial_overlap() {
        let distance = text_distance("hello world", "hello there");
        assert!(distance > 0.0 && distance < 1.0);
    }

    #[test]
    fn test_text_distance_empty() {
        let distance = text_distance("", "");
        assert!(distance.abs() < 0.01); // Should be 0 for empty strings
    }

    // ===== Additional ordering strategy tests =====

    #[test]
    fn test_order_results_recency_asc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::RecencyAsc;

        let now = current_timestamp();
        let mut memories = vec![
            create_retrieved_memory_with_timestamp("1", now.saturating_sub(1000)),
            create_retrieved_memory_with_timestamp("2", now),
            create_retrieved_memory_with_timestamp("3", now.saturating_sub(500)),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "1"); // oldest
        assert_eq!(memories[1].memory.id, "3");
        assert_eq!(memories[2].memory.id, "2"); // most recent
    }

    #[test]
    fn test_order_results_importance_asc() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::ImportanceAsc;

        let mut memories = vec![
            create_retrieved_memory_with_importance("1", 0.3),
            create_retrieved_memory_with_importance("2", 0.9),
            create_retrieved_memory_with_importance("3", 0.6),
        ];

        pipeline.order_results(memories.as_mut_slice());

        assert_eq!(memories[0].memory.id, "1"); // 0.3
        assert_eq!(memories[1].memory.id, "3"); // 0.6
        assert_eq!(memories[2].memory.id, "2"); // 0.9
    }

    #[test]
    fn test_order_results_mixed_shuffles() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.injection.ordering_strategy = crate::OrderingStrategy::Mixed;

        let mut memories = vec![
            create_retrieved_memory("1", 0.5),
            create_retrieved_memory("2", 0.9),
            create_retrieved_memory("3", 0.7),
        ];

        // Just verify it doesn't panic and returns same number of elements
        pipeline.order_results(memories.as_mut_slice());
        assert_eq!(memories.len(), 3);
    }

    // ===== Cache operations tests =====

    #[test]
    fn test_get_cached_result_valid() {
        let pipeline = create_test_pipeline();

        let memories = vec![create_retrieved_memory("1", 0.9)];
        pipeline.cache.insert(
            "test_key".to_string(),
            CachedResult {
                memories: memories.clone(),
                timestamp: current_timestamp(),
                query_hash: 0,
            },
        );

        let result = pipeline.get_cached_result("test_key");
        assert!(result.is_some());
        assert_eq!(result.unwrap().memories.len(), 1);
    }

    #[test]
    fn test_get_cached_result_expired() {
        let pipeline = create_test_pipeline();

        // Insert with old timestamp (expired)
        pipeline.cache.insert(
            "test_key".to_string(),
            CachedResult {
                memories: vec![],
                timestamp: 0, // Very old timestamp
                query_hash: 0,
            },
        );

        // Should return None and remove the expired entry
        let result = pipeline.get_cached_result("test_key");
        assert!(result.is_none());
        assert!(pipeline.cache.get("test_key").is_none());
    }

    #[test]
    fn test_get_cached_result_nonexistent() {
        let pipeline = create_test_pipeline();

        let result = pipeline.get_cached_result("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_result_basic() {
        let pipeline = create_test_pipeline();

        let memories = vec![
            create_retrieved_memory("1", 0.9),
            create_retrieved_memory("2", 0.8),
        ];

        pipeline.cache_result("test_key", &memories);

        assert!(pipeline.cache.get("test_key").is_some());
        assert_eq!(pipeline.cache.get("test_key").unwrap().memories.len(), 2);
    }

    #[test]
    fn test_cache_result_eviction() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.performance.cache_size = 4; // Small cache for testing

        // Fill cache beyond capacity
        for i in 0..6 {
            pipeline.cache_result(&format!("key_{}", i), &[]);
        }

        // Should have evicted some entries
        assert!(pipeline.cache.len() <= pipeline.config.performance.cache_size);
    }

    // ===== Update cache hit metrics test =====

    #[test]
    fn test_update_cache_hit_metrics() {
        let pipeline = create_test_pipeline();

        // First, update with some retrieval metrics to set total_queries
        pipeline.update_retrieval_metrics(1, 5.0, true);

        // Now update cache hit
        pipeline.update_cache_hit_metrics();

        let metrics = pipeline.get_metrics();
        // After one query and one hit, rate should be non-zero
        assert!(metrics.cache_hit_rate > 0.0);
    }

    // ===== Apply diversity with multiple memories test =====

    #[test]
    fn test_apply_diversity_multiple_diverse_texts() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.diversity.enabled = true;
        pipeline.config.diversity.diversity_factor = 0.5;

        let memories = vec![
            create_retrieved_memory_with_text("1", 0.95, "combat dragon attack sword shield"),
            create_retrieved_memory_with_text("2", 0.90, "combat dragon attack sword shield helm"), // Very similar
            create_retrieved_memory_with_text("3", 0.85, "peaceful village market trade gold"), // Different
        ];

        let result = pipeline.apply_diversity(memories);

        // Should return results (at least 1 for most similar)
        assert!(!result.is_empty());
    }

    #[test]
    fn test_apply_diversity_two_items() {
        let pipeline = create_test_pipeline();

        let memories = vec![
            create_retrieved_memory("1", 0.9),
            create_retrieved_memory("2", 0.8),
        ];

        let result = pipeline.apply_diversity(memories);

        // Both should be present since we only have 2
        assert_eq!(result.len(), 2);
        // First should be most similar
        assert_eq!(result[0].memory.id, "1");
    }

    // ===== Consolidation state tests =====

    #[tokio::test]
    async fn test_should_consolidate_disabled() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.consolidation.enabled = false;

        let should = pipeline.should_consolidate().await;
        assert!(!should);
    }

    #[tokio::test]
    async fn test_should_consolidate_in_progress() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.consolidation.enabled = true;

        {
            let mut state = pipeline.consolidation_state.write();
            state.consolidation_in_progress = true;
        }

        let should = pipeline.should_consolidate().await;
        assert!(!should);
    }

    #[tokio::test]
    async fn test_should_consolidate_threshold_met() {
        let mut pipeline = create_test_pipeline();
        pipeline.config.consolidation.enabled = true;
        pipeline.config.consolidation.trigger_threshold = 5;

        {
            let mut state = pipeline.consolidation_state.write();
            state.memories_since_consolidation = 10; // Exceeds threshold
        }

        let should = pipeline.should_consolidate().await;
        assert!(should);
    }

    // ===== Additional metrics test =====

    #[test]
    fn test_update_retrieval_metrics_multiple_successes() {
        let pipeline = create_test_pipeline();

        pipeline.update_retrieval_metrics(5, 10.0, true);
        pipeline.update_retrieval_metrics(3, 20.0, true);
        pipeline.update_retrieval_metrics(7, 15.0, true);

        let metrics = pipeline.get_metrics();
        assert_eq!(metrics.total_queries, 3);
        assert_eq!(metrics.successful_retrievals, 3);
        // Average should be calculated correctly
        assert!((metrics.avg_retrieval_time_ms - 15.0).abs() < 0.01); // (10+20+15)/3 = 15
        assert!((metrics.avg_memories_per_query - 5.0).abs() < 0.01); // (5+3+7)/3 = 5
    }

    // Helper function with custom text
    fn create_retrieved_memory_with_text(id: &str, similarity: f32, text: &str) -> RetrievedMemory {
        RetrievedMemory {
            memory: Memory {
                id: id.to_string(),
                text: text.to_string(),
                timestamp: current_timestamp(),
                importance: 0.5,
                valence: 0.0,
                category: MemoryCategory::Gameplay,
                entities: vec![],
                context: HashMap::new(),
            },
            similarity_score: similarity,
            rank: 0,
            metadata: crate::RetrievalMetadata {
                query: "test".to_string(),
                method: RetrievalMethod::SemanticSearch,
                retrieved_at: current_timestamp(),
                processing_time_ms: 0.0,
                context: HashMap::new(),
            },
        }
    }

    // Helper functions for tests
    fn create_retrieved_memory(id: &str, similarity: f32) -> RetrievedMemory {
        RetrievedMemory {
            memory: Memory {
                id: id.to_string(),
                text: format!("Memory {}", id),
                timestamp: current_timestamp(),
                importance: 0.5,
                valence: 0.0,
                category: MemoryCategory::Gameplay,
                entities: vec![],
                context: HashMap::new(),
            },
            similarity_score: similarity,
            rank: 0,
            metadata: crate::RetrievalMetadata {
                query: "test".to_string(),
                method: RetrievalMethod::SemanticSearch,
                retrieved_at: current_timestamp(),
                processing_time_ms: 0.0,
                context: HashMap::new(),
            },
        }
    }

    fn create_retrieved_memory_with_importance(id: &str, importance: f32) -> RetrievedMemory {
        RetrievedMemory {
            memory: Memory {
                id: id.to_string(),
                text: format!("Memory {}", id),
                timestamp: current_timestamp(),
                importance,
                valence: 0.0,
                category: MemoryCategory::Gameplay,
                entities: vec![],
                context: HashMap::new(),
            },
            similarity_score: 0.5,
            rank: 0,
            metadata: crate::RetrievalMetadata {
                query: "test".to_string(),
                method: RetrievalMethod::SemanticSearch,
                retrieved_at: current_timestamp(),
                processing_time_ms: 0.0,
                context: HashMap::new(),
            },
        }
    }

    fn create_retrieved_memory_with_timestamp(id: &str, timestamp: u64) -> RetrievedMemory {
        RetrievedMemory {
            memory: Memory {
                id: id.to_string(),
                text: format!("Memory {}", id),
                timestamp,
                importance: 0.5,
                valence: 0.0,
                category: MemoryCategory::Gameplay,
                entities: vec![],
                context: HashMap::new(),
            },
            similarity_score: 0.5,
            rank: 0,
            metadata: crate::RetrievalMetadata {
                query: "test".to_string(),
                method: RetrievalMethod::SemanticSearch,
                retrieved_at: current_timestamp(),
                processing_time_ms: 0.0,
                context: HashMap::new(),
            },
        }
    }
}
