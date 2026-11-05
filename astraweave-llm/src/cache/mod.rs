// Prompt caching module for LLM plan generation
// Provides exact-match caching with LRU eviction and optional similarity search

use astraweave_core::PlanIntent;
use std::time::Instant;

pub mod key;
pub mod lru;
pub mod similarity; // Phase 7: Semantic similarity matching

pub use key::PromptKey;
pub use lru::LruCache;
pub use similarity::{prompt_similarity, DEFAULT_SIMILARITY_THRESHOLD};

/// A cached plan with metadata
#[derive(Debug, Clone)]
pub struct CachedPlan {
    /// The cached plan
    pub plan: PlanIntent,
    /// When this plan was cached
    pub created_at: Instant,
    /// Estimated tokens saved by cache hit (approximate)
    pub tokens_saved: u32,
}

/// Result of a cache lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheDecision {
    /// Exact match found
    HitExact,
    /// Similar match found (with similarity score 0.0-1.0)
    #[allow(dead_code)]
    HitSimilar(u32), // u32 for simplicity: score * 100 (e.g., 85 = 0.85 similarity)
    /// No match found
    Miss,
}

/// Prompt cache with LRU eviction and metrics
pub struct PromptCache {
    cache: LruCache<PromptKey, CachedPlan>,
    // Metrics (thread-safe via interior mutability)
    pub hits: std::sync::atomic::AtomicU64,
    pub misses: std::sync::atomic::AtomicU64,
    pub evictions: std::sync::atomic::AtomicU64,
    /// Phase 7: Similarity hits counter
    pub similarity_hits: std::sync::atomic::AtomicU64,
    /// Phase 7: Similarity threshold (0.0-1.0)
    similarity_threshold: f32,
}

impl PromptCache {
    /// Create a new prompt cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity),
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
            evictions: std::sync::atomic::AtomicU64::new(0),
            similarity_hits: std::sync::atomic::AtomicU64::new(0),
            similarity_threshold: DEFAULT_SIMILARITY_THRESHOLD,
        }
    }

    /// Create cache with custom similarity threshold
    pub fn with_similarity_threshold(capacity: usize, threshold: f32) -> Self {
        let mut cache = Self::new(capacity);
        cache.similarity_threshold = threshold;
        cache
    }

    /// Get a cached plan by key (with optional similarity matching)
    ///
    /// Phase 7: Falls back to similarity search if exact match fails
    pub fn get(&self, key: &PromptKey) -> Option<(CachedPlan, CacheDecision)> {
        // Try exact match first
        if let Some(cached) = self.cache.get(key) {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Some((cached.clone(), CacheDecision::HitExact));
        }

        // Phase 7: Try similarity match
        if let Some((cached, score)) = self.find_similar(key) {
            self.similarity_hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Some((cached, CacheDecision::HitSimilar(score)));
        }

        // No match found
        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    /// Phase 7: Find similar cached plan using semantic similarity
    fn find_similar(&self, query_key: &PromptKey) -> Option<(CachedPlan, u32)> {
        let mut best_match: Option<(CachedPlan, f32)> = None;

        // Iterate through cache to find best similarity match
        // Note: This is O(n) but acceptable for small-medium cache sizes (<1000 entries)
        // For large caches, could use approximate nearest neighbor (ANN) index
        let all_keys = self.cache.keys();

        for cached_key in &all_keys {
            // Only compare prompts with same client, model, and similar temperature
            if cached_key.client_id != query_key.client_id {
                continue;
            }
            if cached_key.model != query_key.model {
                continue;
            }
            let temp_diff =
                (cached_key.temperature_q as i32 - query_key.temperature_q as i32).abs();
            if temp_diff > 10 {
                // Allow ±0.1 temperature difference
                continue;
            }

            // Compute semantic similarity
            let similarity =
                prompt_similarity(&query_key.normalized_prompt, &cached_key.normalized_prompt);

            // Update best match if this is better
            if similarity >= self.similarity_threshold {
                if let Some((_, best_score)) = best_match {
                    if similarity > best_score {
                        if let Some(cached_plan) = self.cache.get(cached_key) {
                            best_match = Some((cached_plan.clone(), similarity));
                        }
                    }
                } else if let Some(cached_plan) = self.cache.get(cached_key) {
                    best_match = Some((cached_plan.clone(), similarity));
                }
            }
        }

        // Convert to u32 score (0-100)
        best_match.map(|(plan, score)| (plan, (score * 100.0) as u32))
    }

    /// Insert a plan into the cache
    pub fn put(&self, key: PromptKey, plan: CachedPlan) {
        let evicted = self.cache.put(key, plan);
        if evicted {
            self.evictions
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all items from the cache
    pub fn clear(&self) {
        self.cache.clear();
        self.hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.misses.store(0, std::sync::atomic::Ordering::Relaxed);
        self.similarity_hits
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.evictions
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let similarity_hits = self
            .similarity_hits
            .load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };

        CacheStats {
            hits,
            misses,
            evictions: self.evictions.load(std::sync::atomic::Ordering::Relaxed),
            size: self.len() as u64,
            hit_rate,
            similarity_hits, // Phase 7
        }
    }
}

/// Cache statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: u64,
    pub hit_rate: u32, // percentage 0-100
    /// Phase 7: Number of similarity-based cache hits
    pub similarity_hits: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::ActionStep;

    fn make_test_plan(id: &str) -> PlanIntent {
        PlanIntent {
            plan_id: id.to_string(),
            steps: vec![ActionStep::MoveTo {
                x: 1,
                y: 2,
                speed: None,
            }],
        }
    }

    #[test]
    fn test_cache_hit_miss() {
        let cache = PromptCache::new(10);
        let key1 = PromptKey::new("prompt1", "client", "model1", 0.7, &[]);

        // First access - miss
        assert!(cache.get(&key1).is_none());
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);

        // Insert
        let plan = CachedPlan {
            plan: make_test_plan("test1"),
            created_at: Instant::now(),
            tokens_saved: 100,
        };
        cache.put(key1.clone(), plan.clone());
        assert_eq!(cache.len(), 1);

        // Second access - hit
        let result = cache.get(&key1);
        assert!(result.is_some());
        let (cached_plan, decision) = result.unwrap();
        assert_eq!(decision, CacheDecision::HitExact);
        assert_eq!(cached_plan.plan.plan_id, "test1");
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = PromptCache::new(2);

        let key1 = PromptKey::new("prompt1", "client", "model1", 0.7, &[]);
        let key2 = PromptKey::new("prompt2", "client", "model1", 0.7, &[]);
        let key3 = PromptKey::new("prompt3", "client", "model1", 0.7, &[]);

        let plan1 = CachedPlan {
            plan: make_test_plan("plan1"),
            created_at: Instant::now(),
            tokens_saved: 100,
        };
        let plan2 = CachedPlan {
            plan: make_test_plan("plan2"),
            created_at: Instant::now(),
            tokens_saved: 100,
        };
        let plan3 = CachedPlan {
            plan: make_test_plan("plan3"),
            created_at: Instant::now(),
            tokens_saved: 100,
        };

        // Fill cache to capacity
        cache.put(key1.clone(), plan1);
        cache.put(key2.clone(), plan2);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 0);

        // Insert third item - should evict key1 (LRU)
        cache.put(key3.clone(), plan3);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 1);

        // key1 should be evicted
        assert!(cache.get(&key1).is_none());
        // key2 and key3 should still be present
        assert!(cache.get(&key2).is_some());
        assert!(cache.get(&key3).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("test", "client", "model", 0.7, &[]);

        // Initial stats
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 0);

        // Miss
        cache.get(&key);
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0);

        // Put and hit
        let plan = CachedPlan {
            plan: make_test_plan("test"),
            created_at: Instant::now(),
            tokens_saved: 100,
        };
        cache.put(key.clone(), plan);
        cache.get(&key);
        cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 66); // 2/3 = 66%
    }
}
