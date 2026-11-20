use anyhow::{anyhow, Result};
use dashmap::DashMap;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Advanced prompt caching system for LLM optimization
pub struct PromptCache {
    /// LRU cache for frequently accessed prompts
    cache: Arc<RwLock<LruCache<String, CachedResponse>>>,
    /// Hash-based cache for exact matches
    hash_cache: Arc<DashMap<u64, CachedResponse>>,
    /// Semantic cache for similar prompts
    semantic_cache: Arc<RwLock<HashMap<String, Vec<SemanticCacheEntry>>>>,
    /// Cache configuration
    config: PromptCacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Prompt templates for optimization
    templates: Arc<RwLock<HashMap<String, PromptTemplate>>>,
}

/// Configuration for prompt caching
#[derive(Debug, Clone)]
pub struct PromptCacheConfig {
    /// Maximum number of entries in LRU cache
    pub max_cache_entries: usize,
    /// TTL for cache entries
    pub entry_ttl: Duration,
    /// Enable semantic similarity caching
    pub enable_semantic_cache: bool,
    /// Similarity threshold for semantic cache (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Enable prompt compression
    pub enable_compression: bool,
    /// Maximum prompt length for caching
    pub max_prompt_length: usize,
    /// Enable cache warming
    pub enable_cache_warming: bool,
}

impl Default for PromptCacheConfig {
    fn default() -> Self {
        Self {
            max_cache_entries: 10000,
            // 24 hours
            entry_ttl: Duration::from_secs(24 * 3600),
            enable_semantic_cache: true,
            similarity_threshold: 0.85,
            enable_compression: true,
            max_prompt_length: 8192,
            enable_cache_warming: true,
        }
    }
}

/// Cached response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub id: String,
    pub response: String,
    pub original_prompt: String,
    pub prompt_hash: u64,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub last_accessed: time::OffsetDateTime,
    pub access_count: u32,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub ttl_expires_at: time::OffsetDateTime,
    pub response_quality: f32,
    pub compression_ratio: Option<f32>,
    pub tags: Vec<String>,
}

/// Entry in semantic cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCacheEntry {
    pub cached_response: CachedResponse,
    pub embedding: Vec<f32>,
    pub similarity_scores: HashMap<String, f32>, // Other prompt hashes -> similarity
}

/// Prompt template for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub template: String,
    pub variables: Vec<String>,
    pub usage_count: u32,
    pub average_response_length: f32,
    pub cache_hit_rate: f32,
}

/// Cache performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub semantic_hits: u64,
    pub evictions: u64,
    pub hit_rate: f32,
    pub average_response_time_ms: f32,
    pub memory_usage_mb: f32,
    pub compression_savings_mb: f32,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub last_updated: time::OffsetDateTime,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            semantic_hits: 0,
            evictions: 0,
            hit_rate: 0.0,
            average_response_time_ms: 0.0,
            memory_usage_mb: 0.0,
            compression_savings_mb: 0.0,
            last_updated: time::OffsetDateTime::now_utc(),
        }
    }
}

/// Cache lookup result
#[derive(Debug, Clone)]
#[non_exhaustive]
#[must_use]
pub enum CacheLookupResult {
    /// Exact match found
    ExactHit(CachedResponse),
    /// Semantically similar response found
    SemanticHit(CachedResponse, f32), // response, similarity_score
    /// No matching cache entry
    Miss,
}

/// Cache invalidation strategy
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InvalidationStrategy {
    /// Time-based TTL
    TimeToLive,
    /// Least Recently Used
    LeastRecentlyUsed,
    /// Based on response quality
    QualityBased,
    /// Manual invalidation
    Manual,
}

impl PromptCache {
    pub fn new(config: PromptCacheConfig) -> Self {
        let cache_size = std::num::NonZeroUsize::new(config.max_cache_entries)
            .unwrap_or_else(|| std::num::NonZeroUsize::new(1000).unwrap());

        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            hash_cache: Arc::new(DashMap::new()),
            semantic_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Look up a cached response for a prompt
    pub async fn lookup(&self, prompt: &str) -> Result<CacheLookupResult> {
        let start_time = std::time::Instant::now();

        // Skip caching for very long prompts
        if prompt.len() > self.config.max_prompt_length {
            self.record_miss().await;
            return Ok(CacheLookupResult::Miss);
        }

        let prompt_hash = self.calculate_hash(prompt);

        // Check exact hash match first
        if let Some(cached) = self.hash_cache.get(&prompt_hash) {
            if !self.is_expired(&cached) {
                let response_id = cached.id.clone();
                let cloned = cached.clone();
                drop(cached); // Release DashMap read lock before record_hit
                self.record_hit(&response_id).await;
                debug!("Cache hit for prompt hash: {}", prompt_hash);
                return Ok(CacheLookupResult::ExactHit(cloned));
            } else {
                drop(cached); // Release read lock before remove
                self.hash_cache.remove(&prompt_hash);
            }
        }

        // Check LRU cache
        {
            let mut cache = self.cache.write().await;
            if let Some(cached) = cache.get(&prompt_hash.to_string()) {
                if !self.is_expired(cached) {
                    self.record_hit(&cached.id).await;
                    debug!("LRU cache hit for prompt");
                    return Ok(CacheLookupResult::ExactHit(cached.clone()));
                } else {
                    cache.pop(&prompt_hash.to_string());
                }
            }
        }

        // Check semantic cache if enabled
        if self.config.enable_semantic_cache {
            if let Some((similar_response, similarity)) =
                self.find_similar_cached_response(prompt).await?
            {
                if similarity >= self.config.similarity_threshold {
                    self.record_semantic_hit(&similar_response.id, similarity)
                        .await;
                    debug!("Semantic cache hit with similarity: {:.3}", similarity);
                    return Ok(CacheLookupResult::SemanticHit(similar_response, similarity));
                }
            }
        }

        let lookup_time = start_time.elapsed().as_millis() as f32;
        self.record_miss_with_time(lookup_time).await;
        Ok(CacheLookupResult::Miss)
    }

    /// Store a response in the cache
    pub async fn store(&self, prompt: &str, response: &str, quality: f32) -> Result<()> {
        if prompt.len() > self.config.max_prompt_length {
            return Ok(()); // Skip caching for very long prompts
        }

        let prompt_hash = self.calculate_hash(prompt);
        let now = time::OffsetDateTime::now_utc();

        let cached_response = CachedResponse {
            id: Uuid::new_v4().to_string(),
            response: response.to_string(),
            original_prompt: prompt.to_string(),
            prompt_hash,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            ttl_expires_at: crate::utc_stamp::instant_after(now, self.config.entry_ttl),
            response_quality: quality,
            compression_ratio: None, // Would be calculated if compression is enabled
            tags: Vec::new(),
        };

        // Store in hash cache
        self.hash_cache.insert(prompt_hash, cached_response.clone());

        // Store in LRU cache
        {
            let mut cache = self.cache.write().await;
            let evicted = cache.put(prompt_hash.to_string(), cached_response.clone());
            if evicted.is_some() {
                self.record_eviction().await;
            }
        }

        // Store in semantic cache if enabled
        if self.config.enable_semantic_cache {
            self.store_semantic_entry(cached_response.clone()).await?;
        }

        debug!("Stored response in cache for prompt hash: {}", prompt_hash);
        Ok(())
    }

    /// Invalidate cached entries based on strategy
    pub async fn invalidate(&self, strategy: InvalidationStrategy) -> Result<usize> {
        match strategy {
            InvalidationStrategy::TimeToLive => self.invalidate_expired().await,
            InvalidationStrategy::LeastRecentlyUsed => self.invalidate_lru().await,
            InvalidationStrategy::QualityBased => self.invalidate_low_quality().await,
            InvalidationStrategy::Manual => Ok(0), // Manual invalidation handled separately
        }
    }

    /// Warm the cache with common prompts
    pub async fn warm_cache(&self, common_prompts: Vec<(String, String)>) -> Result<()> {
        if !self.config.enable_cache_warming {
            return Ok(());
        }

        info!("Warming cache with {} common prompts", common_prompts.len());

        for (prompt, response) in common_prompts {
            self.store(&prompt, &response, 0.8).await?; // High quality assumed for warming
        }

        Ok(())
    }

    /// Get cache performance statistics
    pub async fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.write().await;

        // Update memory usage
        let cache_size = self.cache.read().await.len();
        let hash_cache_size = self.hash_cache.len();
        let semantic_cache_size = self.semantic_cache.read().await.len();

        // Rough memory usage estimate
        stats.memory_usage_mb = ((cache_size + hash_cache_size + semantic_cache_size) * 1024)
            as f32
            / (1024.0 * 1024.0);

        // Update hit rate
        if stats.total_requests > 0 {
            stats.hit_rate =
                (stats.cache_hits + stats.semantic_hits) as f32 / stats.total_requests as f32;
        }

        stats.last_updated = time::OffsetDateTime::now_utc();
        stats.clone()
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        self.hash_cache.clear();

        {
            let mut semantic_cache = self.semantic_cache.write().await;
            semantic_cache.clear();
        }

        // Reset statistics
        {
            let mut stats = self.stats.write().await;
            *stats = CacheStats::default();
        }

        info!("Cleared all cache entries");
        Ok(())
    }

    /// Add a prompt template for optimization
    pub async fn add_template(&self, template: PromptTemplate) -> Result<()> {
        let mut templates = self.templates.write().await;
        templates.insert(template.id.clone(), template);
        Ok(())
    }

    /// Get cached responses matching a template
    pub async fn get_template_responses(&self, template_id: &str) -> Result<Vec<CachedResponse>> {
        let templates = self.templates.read().await;

        if let Some(template) = templates.get(template_id) {
            // Find cached responses that match this template pattern
            let mut matching_responses = Vec::new();

            for cached_entry in self.hash_cache.iter() {
                let cached = cached_entry.value();
                if self.matches_template(&cached.original_prompt, template) {
                    matching_responses.push(cached.clone());
                }
            }

            Ok(matching_responses)
        } else {
            Err(anyhow!("Template {} not found", template_id))
        }
    }

    /// Calculate hash for a prompt
    fn calculate_hash(&self, prompt: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if a cached response is expired
    fn is_expired(&self, cached: &CachedResponse) -> bool {
        self.expired_at(cached, time::OffsetDateTime::now_utc())
    }

    fn expired_at(&self, cached: &CachedResponse, now: time::OffsetDateTime) -> bool {
        now > cached.ttl_expires_at
    }

    /// Record a cache hit
    async fn record_hit(&self, response_id: &str) {
        // Update access information in hash cache
        for mut cached_entry in self.hash_cache.iter_mut() {
            let cached = cached_entry.value_mut();
            if cached.id == response_id {
                cached.last_accessed = time::OffsetDateTime::now_utc();
                cached.access_count += 1;
                break;
            }
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.cache_hits += 1;
    }

    /// Record a semantic cache hit
    async fn record_semantic_hit(&self, response_id: &str, _similarity: f32) {
        self.record_hit(response_id).await;

        let mut stats = self.stats.write().await;
        stats.semantic_hits += 1;
        stats.cache_hits = stats.cache_hits.saturating_sub(1); // Don't double count
    }

    /// Record a cache miss
    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.cache_misses += 1;
    }

    /// Record a cache miss with lookup time
    async fn record_miss_with_time(&self, lookup_time_ms: f32) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.cache_misses += 1;

        // Update average response time
        let total_hits = stats.cache_hits + stats.semantic_hits;
        if total_hits > 0 {
            stats.average_response_time_ms = (stats.average_response_time_ms * total_hits as f32
                + lookup_time_ms)
                / (total_hits + 1) as f32;
        }
    }

    /// Record a cache eviction
    async fn record_eviction(&self) {
        let mut stats = self.stats.write().await;
        stats.evictions += 1;
    }

    /// Find semantically similar cached response
    async fn find_similar_cached_response(
        &self,
        prompt: &str,
    ) -> Result<Option<(CachedResponse, f32)>> {
        // This is a simplified implementation
        // In practice, would use embeddings and vector similarity

        let semantic_cache = self.semantic_cache.read().await;
        let mut best_match = None;
        let mut best_similarity = 0.0;

        for entries in semantic_cache.values() {
            for entry in entries {
                // Calculate similarity (simplified word overlap)
                let similarity =
                    self.calculate_text_similarity(prompt, &entry.cached_response.original_prompt);

                if similarity > best_similarity && similarity >= self.config.similarity_threshold {
                    best_similarity = similarity;
                    best_match = Some(entry.cached_response.clone());
                }
            }
        }

        Ok(best_match.map(|response| (response, best_similarity)))
    }

    /// Store an entry in semantic cache
    async fn store_semantic_entry(&self, cached_response: CachedResponse) -> Result<()> {
        // Simplified semantic storage - would use proper embeddings in practice
        let category = self.categorize_prompt(&cached_response.original_prompt);

        let mut semantic_cache = self.semantic_cache.write().await;
        let entries = semantic_cache.entry(category).or_insert_with(Vec::new);

        let semantic_entry = SemanticCacheEntry {
            cached_response,
            embedding: Vec::new(), // Would be actual embeddings
            similarity_scores: HashMap::new(),
        };

        entries.push(semantic_entry);

        // Limit semantic cache size
        if entries.len() > 100 {
            entries.remove(0);
        }

        Ok(())
    }

    /// Calculate text similarity (simplified)
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: Vec<&str> = text1.split_whitespace().collect();
        let words2: Vec<&str> = text2.split_whitespace().collect();

        let common_words = words1.iter().filter(|word| words2.contains(word)).count();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let total_words = (words1.len() + words2.len()) as f32;
        (common_words * 2) as f32 / total_words
    }

    /// Categorize a prompt for semantic caching
    fn categorize_prompt(&self, prompt: &str) -> String {
        // Simplified categorization - would use more sophisticated NLP
        if prompt.contains("dialogue") || prompt.contains("conversation") {
            "dialogue".to_string()
        } else if prompt.contains("quest") || prompt.contains("mission") {
            "quest".to_string()
        } else if prompt.contains("description") || prompt.contains("describe") {
            "description".to_string()
        } else {
            "general".to_string()
        }
    }

    /// Check if a prompt matches a template
    fn matches_template(&self, prompt: &str, template: &PromptTemplate) -> bool {
        // Simplified template matching - would use proper template parsing
        template.variables.iter().all(|var| prompt.contains(var))
    }

    /// Invalidate expired entries
    async fn invalidate_expired(&self) -> Result<usize> {
        let now = time::OffsetDateTime::now_utc();
        let mut invalidated = 0;

        // Clean hash cache
        self.hash_cache.retain(|_, cached| {
            let expired = cached.ttl_expires_at <= now;
            if expired {
                invalidated += 1;
            }
            !expired
        });

        // Clean LRU cache
        {
            let mut cache = self.cache.write().await;
            let mut to_remove = Vec::new();

            // Note: LruCache doesn't have retain method, so we collect keys to remove
            for (key, cached) in cache.iter() {
                if cached.ttl_expires_at <= now {
                    to_remove.push(key.clone());
                }
            }

            for key in to_remove {
                cache.pop(&key);
                invalidated += 1;
            }
        }

        // Clean semantic cache
        {
            let mut semantic_cache = self.semantic_cache.write().await;
            for entries in semantic_cache.values_mut() {
                entries.retain(|entry| {
                    let expired = entry.cached_response.ttl_expires_at <= now;
                    if expired {
                        invalidated += 1;
                    }
                    !expired
                });
            }

            // Remove empty categories
            semantic_cache.retain(|_, entries| !entries.is_empty());
        }

        if Self::any_expired(invalidated) {
            debug!("Invalidated {} expired cache entries", invalidated);
        }

        Ok(invalidated)
    }

    fn any_expired(count: usize) -> bool {
        count > 0
    }

    /// Invalidate least recently used entries
    async fn invalidate_lru(&self) -> Result<usize> {
        // LRU cache handles this automatically, but we can force eviction
        let current_size = self.cache.read().await.len();
        let target_size = current_size * 3 / 4; // Remove 25% of entries

        if current_size <= target_size {
            return Ok(0);
        }

        let mut invalidated = 0;
        {
            let mut cache = self.cache.write().await;
            while cache.len() > target_size {
                if cache.pop_lru().is_some() {
                    invalidated += 1;
                } else {
                    break;
                }
            }
        }

        debug!("Invalidated {} LRU cache entries", invalidated);
        Ok(invalidated)
    }

    /// Invalidate low quality entries
    async fn invalidate_low_quality(&self) -> Result<usize> {
        let quality_threshold = 0.3;
        let mut invalidated = 0;

        // Clean hash cache
        self.hash_cache.retain(|_, cached| {
            let low_quality = cached.response_quality < quality_threshold;
            if low_quality {
                invalidated += 1;
            }
            !low_quality
        });

        debug!("Invalidated {} low quality cache entries", invalidated);
        Ok(invalidated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_cache_creation() {
        let config = PromptCacheConfig::default();
        let cache = PromptCache::new(config);

        let stats = cache.get_stats().await;
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_cache_store_and_lookup() {
        let cache = PromptCache::new(PromptCacheConfig::default());

        // Store a response
        cache
            .store("Test prompt", "Test response", 0.8)
            .await
            .unwrap();

        // Look it up
        let result = cache.lookup("Test prompt").await.unwrap();
        match result {
            CacheLookupResult::ExactHit(cached) => {
                assert_eq!(cached.response, "Test response");
                assert_eq!(cached.response_quality, 0.8);
            }
            _ => panic!("Expected exact hit"),
        }
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = PromptCache::new(PromptCacheConfig::default());

        let result = cache.lookup("Non-existent prompt").await.unwrap();
        matches!(result, CacheLookupResult::Miss);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = PromptCacheConfig {
            entry_ttl: Duration::from_millis(100),
            ..PromptCacheConfig::default()
        };
        let cache = PromptCache::new(config);

        // Store a response
        cache
            .store("Test prompt", "Test response", 0.8)
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        let result = cache.lookup("Test prompt").await.unwrap();
        matches!(result, CacheLookupResult::Miss);
    }

    #[test]
    fn test_text_similarity() {
        let cache = PromptCache::new(PromptCacheConfig::default());

        let similarity = cache.calculate_text_similarity("hello world", "hello universe");
        assert!(similarity > 0.0);
        assert!(similarity < 1.0);

        let exact_similarity = cache.calculate_text_similarity("hello world", "hello world");
        assert_eq!(exact_similarity, 1.0);
    }

    fn cached(id: &str, ttl: time::OffsetDateTime) -> CachedResponse {
        CachedResponse {
            id: id.into(),
            response: "r".into(),
            original_prompt: id.into(),
            prompt_hash: 1,
            created_at: ttl,
            last_accessed: ttl,
            access_count: 0,
            ttl_expires_at: ttl,
            response_quality: 1.0,
            compression_ratio: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn expiry_is_strictly_after_the_ttl() {
        let cache = PromptCache::new(PromptCacheConfig::default());
        let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(5_000);
        let entry = cached("a", now);
        assert!(!cache.expired_at(&entry, now));
        assert!(cache.expired_at(&entry, now + time::Duration::nanoseconds(1)));
        assert!(!cache.expired_at(&entry, now - time::Duration::seconds(1)));

        let mut past = cached(
            "past",
            time::OffsetDateTime::now_utc() - time::Duration::hours(1),
        );
        past.ttl_expires_at = time::OffsetDateTime::now_utc() - time::Duration::hours(1);
        assert!(cache.is_expired(&past));
        let mut future = cached("future", time::OffsetDateTime::now_utc());
        future.ttl_expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
        assert!(!cache.is_expired(&future));
    }

    #[tokio::test]
    async fn record_hit_updates_only_the_matching_id_and_stats() {
        let cache = PromptCache::new(PromptCacheConfig::default());
        let now = time::OffsetDateTime::now_utc();
        cache.hash_cache.insert(1, cached("alpha", now));
        cache.hash_cache.insert(2, cached("beta", now));
        cache.record_hit("alpha").await;
        assert_eq!(cache.hash_cache.get(&1).unwrap().access_count, 1);
        assert_eq!(cache.hash_cache.get(&2).unwrap().access_count, 0);
        {
            let stats = cache.stats.read().await;
            assert_eq!(stats.cache_hits, 1);
            assert_eq!(stats.total_requests, 1);
        }
        cache.semantic_cache.write().await.insert(
            "kept".into(),
            vec![SemanticCacheEntry {
                cached_response: cached("sem", time::OffsetDateTime::now_utc()),
                embedding: Vec::new(),
                similarity_scores: HashMap::new(),
            }],
        );
        {
            let mut stats = cache.stats.write().await;
            stats.semantic_hits = 3;
            stats.total_requests = 8;
        }
        let stats = cache.get_stats().await;
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.semantic_hits, 3);
        assert_eq!(stats.total_requests, 8);
        assert!((stats.hit_rate - 0.5).abs() < f32::EPSILON);
        assert!((stats.memory_usage_mb - (3.0 / 1024.0)).abs() < 1e-6);

        let empty = PromptCache::new(PromptCacheConfig::default());
        let empty_stats = empty.get_stats().await;
        assert_eq!(empty_stats.hit_rate, 0.0);
        assert_eq!(empty_stats.memory_usage_mb, 0.0);
        assert_eq!(empty_stats.total_requests, 0);
        assert!(PromptCache::any_expired(1));
        assert!(PromptCache::any_expired(2));
        assert!(!PromptCache::any_expired(0));
    }

    #[tokio::test]
    async fn invalidate_expired_removes_every_expired_hash_entry() {
        let cache = PromptCache::new(PromptCacheConfig::default());
        let now = time::OffsetDateTime::now_utc();
        cache
            .hash_cache
            .insert(1, cached("old-a", now - time::Duration::hours(2)));
        cache
            .hash_cache
            .insert(2, cached("old-b", now - time::Duration::hours(3)));
        cache
            .hash_cache
            .insert(3, cached("fresh", now + time::Duration::hours(2)));
        cache.cache.write().await.put(
            "old-lru".into(),
            cached("old-lru", now - time::Duration::hours(1)),
        );
        cache.cache.write().await.put(
            "fresh-lru".into(),
            cached("fresh-lru", now + time::Duration::hours(1)),
        );
        cache.semantic_cache.write().await.insert(
            "old".into(),
            vec![SemanticCacheEntry {
                cached_response: cached("old-sem", now - time::Duration::hours(1)),
                embedding: Vec::new(),
                similarity_scores: HashMap::new(),
            }],
        );
        cache.semantic_cache.write().await.insert(
            "keep".into(),
            vec![SemanticCacheEntry {
                cached_response: cached("fresh-sem", now + time::Duration::hours(1)),
                embedding: Vec::new(),
                similarity_scores: HashMap::new(),
            }],
        );
        let removed = cache.invalidate_expired().await.unwrap();
        assert_eq!(removed, 4);
        assert_eq!(cache.hash_cache.len(), 1);
        assert!(cache.hash_cache.contains_key(&3));
        let lru = cache.cache.read().await;
        assert!(lru.contains(&String::from("fresh-lru")));
        assert!(!lru.contains(&String::from("old-lru")));
        drop(lru);
        let semantic = cache.semantic_cache.read().await;
        assert!(semantic.contains_key("keep"));
        assert!(!semantic.contains_key("old"));
    }

    #[tokio::test]
    async fn a_ten_thousand_year_ttl_does_not_abort() {
        let config = PromptCacheConfig {
            entry_ttl: Duration::from_secs(10_000 * 365 * 24 * 3600),
            enable_semantic_cache: false,
            ..PromptCacheConfig::default()
        };
        let cache = PromptCache::new(config);
        cache.store("prompt", "response", 1.0).await.unwrap();
        let stored = cache.hash_cache.iter().next().expect("stored entry");
        assert_eq!(
            stored.ttl_expires_at,
            time::OffsetDateTime::new_utc(time::Date::MAX, time::Time::MAX)
        );
    }
}
