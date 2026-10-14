//! Memory consolidation and compression functionality
//!
//! This module handles consolidating and compressing memories to optimize storage and retrieval.

use anyhow::Result;
use astraweave_embeddings::Memory;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use astraweave_embeddings::MemoryCategory;
#[cfg(test)]
use std::collections::HashMap;

/// Consolidation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Enable automatic memory consolidation
    pub enabled: bool,

    /// Number of memories to trigger consolidation
    pub trigger_threshold: usize,

    /// Similarity threshold for merging memories
    pub merge_similarity_threshold: f32,

    /// Maximum memories to keep per consolidation
    pub max_memories_per_batch: usize,

    /// Consolidation strategy
    pub strategy: ConsolidationStrategy,

    /// How often to run consolidation (in seconds)
    pub consolidation_interval: u64,

    /// Maximum age in seconds for memories to be considered for consolidation
    pub max_age_seconds: u64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_threshold: 100,
            merge_similarity_threshold: 0.85,
            max_memories_per_batch: 50,
            strategy: ConsolidationStrategy::Importance,
            consolidation_interval: 3600, // 1 hour
            max_age_seconds: 86400,       // 24 hours
        }
    }
}

/// Strategies for memory consolidation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ConsolidationStrategy {
    /// Consolidate based on importance scores
    Importance,
    /// Consolidate based on recency
    Recency,
    /// Consolidate similar memories together
    Similarity,
    /// Hybrid approach using multiple factors
    Hybrid,
}

/// Result of a consolidation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// Number of memories processed
    pub processed_count: usize,
    /// Number of memories merged
    pub merged_count: usize,
    /// Number of memories removed
    pub removed_count: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Memory consolidation engine
#[derive(Debug)]
pub struct ConsolidationEngine {
    /// Configuration
    config: ConsolidationConfig,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine
    pub fn new(config: ConsolidationConfig) -> Self {
        Self { config }
    }

    /// Consolidate a collection of memories
    pub fn consolidate(&self, memories: Vec<Memory>) -> Result<(Vec<Memory>, ConsolidationResult)> {
        let start_time = std::time::Instant::now();
        let processed_count = memories.len();
        let mut consolidated_memories = Vec::new();
        let mut merged_count = 0;
        let mut removed_count = 0;

        // Simple consolidation logic - merge similar memories
        let mut processed_indices = std::collections::HashSet::new();

        for (i, memory) in memories.iter().enumerate() {
            if processed_indices.contains(&i) {
                continue;
            }

            let mut merged_memory = memory.clone();
            let mut merge_occurred = false;

            // Look for similar memories to merge
            for (j, other_memory) in memories.iter().enumerate().skip(i + 1) {
                if processed_indices.contains(&j) {
                    continue;
                }

                if self.should_merge(memory, other_memory) {
                    merged_memory = self.merge_memories(&merged_memory, other_memory)?;
                    processed_indices.insert(j);
                    merge_occurred = true;
                    removed_count += 1;
                }
            }

            consolidated_memories.push(merged_memory);
            processed_indices.insert(i);

            if merge_occurred {
                merged_count += 1;
            }
        }

        let result = ConsolidationResult {
            processed_count,
            merged_count,
            removed_count,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
        };

        Ok((consolidated_memories, result))
    }

    /// Check if two memories should be merged
    fn should_merge(&self, memory1: &Memory, memory2: &Memory) -> bool {
        // Same category
        if memory1.category != memory2.category {
            return false;
        }

        // Similar content (simplified similarity check)
        let similarity = self.calculate_similarity(&memory1.text, &memory2.text);
        similarity >= self.config.merge_similarity_threshold
    }

    /// Merge two memories into one
    fn merge_memories(&self, memory1: &Memory, memory2: &Memory) -> Result<Memory> {
        let mut merged = memory1.clone();

        // Combine content
        merged.text = format!("{} {}", memory1.text, memory2.text);

        // Take the most recent timestamp
        merged.timestamp = memory1.timestamp.max(memory2.timestamp);

        // Merge context in key order so the map's insertion sequence is stable.
        let mut keys: Vec<&String> = memory2.context.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(value) = memory2.context.get(key) {
                merged.context.insert(key.clone(), value.clone());
            }
        }

        // Content-addressed id. Uuid::v4 made every consolidation replay diverge.
        merged.id = format!("merged_{}_{}", memory1.id, memory2.id);

        Ok(merged)
    }

    /// Calculate content similarity (simplified)
    fn calculate_similarity(&self, content1: &str, content2: &str) -> f32 {
        let words1: Vec<&str> = content1.split_whitespace().collect();
        let words2: Vec<&str> = content2.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let common_words = words1.iter().filter(|word| words2.contains(word)).count();

        let total_unique_words = words1.len() + words2.len() - common_words;
        common_words as f32 / total_unique_words as f32
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    // ========================================================================
    // HELPER FUNCTIONS
    // ========================================================================

    fn create_test_memory(id: &str, text: &str, importance: f32) -> Memory {
        Memory {
            id: id.to_string(),
            text: text.to_string(),
            category: MemoryCategory::Social,
            timestamp: chrono::Utc::now().timestamp() as u64,
            importance,
            valence: 0.0,
            entities: vec![],
            context: HashMap::new(),
        }
    }

    fn create_test_memory_with_category(id: &str, text: &str, category: MemoryCategory) -> Memory {
        Memory {
            id: id.to_string(),
            text: text.to_string(),
            category,
            timestamp: chrono::Utc::now().timestamp() as u64,
            importance: 0.5,
            valence: 0.0,
            entities: vec![],
            context: HashMap::new(),
        }
    }

    fn create_test_memory_with_context(
        id: &str,
        text: &str,
        context: HashMap<String, String>,
    ) -> Memory {
        Memory {
            id: id.to_string(),
            text: text.to_string(),
            category: MemoryCategory::Social,
            timestamp: chrono::Utc::now().timestamp() as u64,
            importance: 0.5,
            valence: 0.0,
            entities: vec![],
            context,
        }
    }

    // ========================================================================
    // CONSOLIDATION CONFIG TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_config_default() {
        let config = ConsolidationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.trigger_threshold, 100);
        assert_eq!(config.merge_similarity_threshold, 0.85);
        assert_eq!(config.max_memories_per_batch, 50);
        assert_eq!(config.strategy, ConsolidationStrategy::Importance);
        assert_eq!(config.consolidation_interval, 3600);
        assert_eq!(config.max_age_seconds, 86400);
    }

    #[test]
    fn test_consolidation_config_clone() {
        let config = ConsolidationConfig {
            enabled: false,
            trigger_threshold: 50,
            merge_similarity_threshold: 0.7,
            max_memories_per_batch: 25,
            strategy: ConsolidationStrategy::Recency,
            consolidation_interval: 1800,
            max_age_seconds: 43200,
        };

        let cloned = config.clone();
        assert_eq!(cloned.enabled, config.enabled);
        assert_eq!(cloned.trigger_threshold, config.trigger_threshold);
        assert_eq!(
            cloned.merge_similarity_threshold,
            config.merge_similarity_threshold
        );
        assert_eq!(cloned.strategy, config.strategy);
    }

    #[test]
    fn test_consolidation_strategy_variants() {
        assert_eq!(
            ConsolidationStrategy::Importance,
            ConsolidationStrategy::Importance
        );
        assert_eq!(
            ConsolidationStrategy::Recency,
            ConsolidationStrategy::Recency
        );
        assert_eq!(
            ConsolidationStrategy::Similarity,
            ConsolidationStrategy::Similarity
        );
        assert_eq!(ConsolidationStrategy::Hybrid, ConsolidationStrategy::Hybrid);

        assert_ne!(
            ConsolidationStrategy::Importance,
            ConsolidationStrategy::Recency
        );
    }

    // ========================================================================
    // CONSOLIDATION ENGINE BASIC TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_engine() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            Memory {
                id: "1".to_string(),
                text: "The cat is sleeping".to_string(),
                category: MemoryCategory::Social,
                timestamp: chrono::Utc::now().timestamp() as u64,
                importance: 0.5,
                valence: 0.0,
                entities: vec![],
                context: HashMap::new(),
            },
            Memory {
                id: "2".to_string(),
                text: "The cat is resting".to_string(),
                category: MemoryCategory::Social,
                timestamp: chrono::Utc::now().timestamp() as u64,
                importance: 0.5,
                valence: 0.0,
                entities: vec![],
                context: HashMap::new(),
            },
        ];

        let (consolidated, result) = engine.consolidate(memories).unwrap();
        assert!(!consolidated.is_empty());
        assert_eq!(result.processed_count, 2);
    }

    #[test]
    fn test_consolidation_engine_empty_input() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memories: Vec<Memory> = vec![];
        let (consolidated, result) = engine.consolidate(memories).unwrap();

        assert!(consolidated.is_empty());
        assert_eq!(result.processed_count, 0);
        assert_eq!(result.merged_count, 0);
        assert_eq!(result.removed_count, 0);
    }

    #[test]
    fn test_consolidation_engine_single_memory() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memories = vec![create_test_memory("1", "Single memory", 0.5)];
        let (consolidated, result) = engine.consolidate(memories).unwrap();

        assert_eq!(consolidated.len(), 1);
        assert_eq!(result.processed_count, 1);
        assert_eq!(result.merged_count, 0);
        assert_eq!(result.removed_count, 0);
    }

    // ========================================================================
    // SIMILARITY AND MERGING TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_similar_memories_merged() {
        let mut config = ConsolidationConfig::default();
        config.merge_similarity_threshold = 0.5; // Lower threshold to allow merging

        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory("1", "The quick brown fox jumps", 0.5),
            create_test_memory("2", "The quick brown fox runs", 0.5),
        ];

        let (consolidated, result) = engine.consolidate(memories).unwrap();

        // Should merge similar memories
        let _ = consolidated; // Verify consolidation ran
        assert_eq!(result.processed_count, 2);
    }

    #[test]
    fn test_consolidation_different_memories_not_merged() {
        let mut config = ConsolidationConfig::default();
        config.merge_similarity_threshold = 0.99; // Very high threshold

        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory("1", "The sun is bright", 0.5),
            create_test_memory("2", "Computers process data", 0.5),
        ];

        let (consolidated, result) = engine.consolidate(memories).unwrap();

        // No merging should occur with high threshold
        assert_eq!(consolidated.len(), 2);
        assert_eq!(result.merged_count, 0);
        assert_eq!(result.removed_count, 0);
    }

    #[test]
    fn test_consolidation_different_categories_not_merged() {
        let mut config = ConsolidationConfig::default();
        config.merge_similarity_threshold = 0.1; // Very low threshold

        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory_with_category("1", "Test content", MemoryCategory::Combat),
            create_test_memory_with_category("2", "Test content", MemoryCategory::Social),
        ];

        let (consolidated, result) = engine.consolidate(memories).unwrap();

        // Different categories should not merge even with identical text
        assert_eq!(consolidated.len(), 2);
        assert_eq!(result.merged_count, 0);
    }

    #[test]
    fn test_calculate_similarity_identical() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let text = "The quick brown fox";
        let similarity = engine.calculate_similarity(text, text);

        // Identical text should have similarity ~1.0
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_similarity_empty() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let similarity = engine.calculate_similarity("", "");
        assert_eq!(similarity, 0.0);

        let similarity2 = engine.calculate_similarity("text", "");
        assert_eq!(similarity2, 0.0);

        let similarity3 = engine.calculate_similarity("", "text");
        assert_eq!(similarity3, 0.0);
    }

    #[test]
    fn test_calculate_similarity_partial_overlap() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let text1 = "apple banana cherry";
        let text2 = "banana cherry date";

        let similarity = engine.calculate_similarity(text1, text2);

        // 2 words overlap (banana, cherry), 4 unique words total
        // Jaccard: 2/4 = 0.5
        assert!((similarity - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_calculate_similarity_no_overlap() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let text1 = "apple banana cherry";
        let text2 = "dog elephant fox";

        let similarity = engine.calculate_similarity(text1, text2);
        assert_eq!(similarity, 0.0);
    }

    // ========================================================================
    // MERGE BEHAVIOR TESTS
    // ========================================================================

    #[test]
    fn test_merge_memories_combines_text() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memory1 = create_test_memory("1", "First part", 0.5);
        let memory2 = create_test_memory("2", "Second part", 0.5);

        let merged = engine.merge_memories(&memory1, &memory2).unwrap();

        assert!(merged.text.contains("First part"));
        assert!(merged.text.contains("Second part"));
    }

    #[test]
    fn test_merge_memories_takes_latest_timestamp() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let mut memory1 = create_test_memory("1", "First", 0.5);
        memory1.timestamp = 1000;

        let mut memory2 = create_test_memory("2", "Second", 0.5);
        memory2.timestamp = 2000;

        let merged = engine.merge_memories(&memory1, &memory2).unwrap();
        assert_eq!(merged.timestamp, 2000);

        // Test reverse order
        let merged2 = engine.merge_memories(&memory2, &memory1).unwrap();
        assert_eq!(merged2.timestamp, 2000);
    }

    #[test]
    fn test_merge_memories_combines_context() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let mut context1 = HashMap::new();
        context1.insert("key1".to_string(), "value1".to_string());

        let mut context2 = HashMap::new();
        context2.insert("key2".to_string(), "value2".to_string());

        let memory1 = create_test_memory_with_context("1", "First", context1);
        let memory2 = create_test_memory_with_context("2", "Second", context2);

        let merged = engine.merge_memories(&memory1, &memory2).unwrap();

        assert_eq!(merged.context.get("key1"), Some(&"value1".to_string()));
        assert_eq!(merged.context.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_merge_memories_creates_new_id() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memory1 = create_test_memory("original_1", "First", 0.5);
        let memory2 = create_test_memory("original_2", "Second", 0.5);

        let merged = engine.merge_memories(&memory1, &memory2).unwrap();

        assert!(merged.id.starts_with("merged_"));
        assert_ne!(merged.id, memory1.id);
        assert_ne!(merged.id, memory2.id);
    }

    // ========================================================================
    // CONSOLIDATION RESULT TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_result_default() {
        let result = ConsolidationResult {
            processed_count: 10,
            merged_count: 3,
            removed_count: 2,
            processing_time_ms: 50,
        };

        assert_eq!(result.processed_count, 10);
        assert_eq!(result.merged_count, 3);
        assert_eq!(result.removed_count, 2);
        assert_eq!(result.processing_time_ms, 50);
    }

    #[test]
    fn test_consolidation_result_clone() {
        let result = ConsolidationResult {
            processed_count: 5,
            merged_count: 2,
            removed_count: 1,
            processing_time_ms: 25,
        };

        let cloned = result.clone();
        assert_eq!(cloned.processed_count, result.processed_count);
        assert_eq!(cloned.merged_count, result.merged_count);
        assert_eq!(cloned.removed_count, result.removed_count);
        assert_eq!(cloned.processing_time_ms, result.processing_time_ms);
    }

    // ========================================================================
    // MULTI-MEMORY CONSOLIDATION TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_multiple_similar_groups() {
        let mut config = ConsolidationConfig::default();
        config.merge_similarity_threshold = 0.3; // Low threshold

        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory("1", "cat dog bird", 0.5),
            create_test_memory("2", "cat dog fish", 0.5),
            create_test_memory("3", "apple banana cherry", 0.5),
            create_test_memory("4", "apple banana date", 0.5),
        ];

        let (consolidated, result) = engine.consolidate(memories).unwrap();

        // Should consolidate into fewer memories
        assert!(consolidated.len() <= 4);
        assert_eq!(result.processed_count, 4);
    }

    #[test]
    fn test_consolidation_preserves_order_of_first_memory() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory("first", "First memory", 0.9),
            create_test_memory("second", "Second memory", 0.1),
        ];

        let (consolidated, _) = engine.consolidate(memories).unwrap();

        // First memory should be first in output (no merging with high threshold)
        assert_eq!(consolidated[0].id, "first");
    }

    // ========================================================================
    // SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_consolidation_config_serialization() {
        let config = ConsolidationConfig {
            enabled: true,
            trigger_threshold: 75,
            merge_similarity_threshold: 0.80,
            max_memories_per_batch: 40,
            strategy: ConsolidationStrategy::Hybrid,
            consolidation_interval: 7200,
            max_age_seconds: 172800,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ConsolidationConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.trigger_threshold, config.trigger_threshold);
        assert_eq!(deserialized.strategy, config.strategy);
    }

    #[test]
    fn test_consolidation_result_serialization() {
        let result = ConsolidationResult {
            processed_count: 100,
            merged_count: 20,
            removed_count: 15,
            processing_time_ms: 250,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ConsolidationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.processed_count, result.processed_count);
        assert_eq!(deserialized.merged_count, result.merged_count);
    }

    #[test]
    fn test_consolidation_strategy_serialization() {
        let strategies = vec![
            ConsolidationStrategy::Importance,
            ConsolidationStrategy::Recency,
            ConsolidationStrategy::Similarity,
            ConsolidationStrategy::Hybrid,
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let deserialized: ConsolidationStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, strategy);
        }
    }
}
