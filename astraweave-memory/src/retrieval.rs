//! Memory retrieval and search functionality
//!
//! This module provides sophisticated memory retrieval using various search strategies
//! including semantic similarity, temporal proximity, and associative connections.

use crate::memory_types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Maximum number of memories to retrieve
    pub max_results: usize,
    /// Minimum relevance score for inclusion
    pub relevance_threshold: f32,
    /// Weight for semantic similarity
    pub semantic_weight: f32,
    /// Weight for temporal proximity
    pub temporal_weight: f32,
    /// Weight for associative strength
    pub associative_weight: f32,
    /// Whether to boost recent memories
    pub recency_boost: bool,
    /// Whether to follow associative chains
    pub follow_associations: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            relevance_threshold: 0.3,
            semantic_weight: 0.6,
            temporal_weight: 0.2,
            associative_weight: 0.2,
            recency_boost: true,
            follow_associations: true,
        }
    }
}

/// Result of a memory retrieval operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Retrieved memory
    pub memory: Memory,
    /// Overall relevance score
    pub relevance_score: f32,
    /// Breakdown of score components
    pub score_breakdown: ScoreBreakdown,
    /// Retrieval path (how this memory was found)
    pub retrieval_path: RetrievalPath,
}

/// Breakdown of relevance score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub semantic_score: f32,
    pub temporal_score: f32,
    pub associative_score: f32,
    pub importance_score: f32,
    pub recency_score: f32,
}

/// How a memory was retrieved
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RetrievalPath {
    /// Direct match to query
    Direct,
    /// Found through association with another memory
    Associative { source_memory_id: String },
    /// Found through temporal proximity
    Temporal {
        #[serde(with = "crate::utc_stamp::serde_rfc3339")]
        reference_time: time::OffsetDateTime,
    },
    /// Found through clustering
    Cluster { cluster_id: String },
}

/// Memory retrieval engine
#[derive(Debug)]
pub struct RetrievalEngine {
    config: RetrievalConfig,
}

impl RetrievalEngine {
    /// Create a new retrieval engine
    pub fn new(config: RetrievalConfig) -> Self {
        Self { config }
    }

    /// Retrieve memories based on context
    pub fn retrieve(
        &self,
        context: &RetrievalContext,
        memories: &[Memory],
    ) -> Result<Vec<RetrievalResult>> {
        let mut results = Vec::new();

        // First pass: direct matching
        for memory in memories {
            if memory.matches_context(context) {
                let relevance_score = self.calculate_relevance(context, memory)?;

                if relevance_score >= self.config.relevance_threshold {
                    let score_breakdown = self.calculate_score_breakdown(context, memory)?;

                    results.push(RetrievalResult {
                        memory: memory.clone(),
                        relevance_score,
                        score_breakdown,
                        retrieval_path: RetrievalPath::Direct,
                    });
                }
            }
        }

        // Second pass: associative retrieval
        if self.config.follow_associations {
            let associated_results =
                self.retrieve_associated_memories(context, memories, &results)?;
            results.extend(associated_results);
        }

        // Sort by relevance score
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        results.truncate(context.limit.min(self.config.max_results));

        Ok(results)
    }

    /// Calculate overall relevance score
    fn calculate_relevance(&self, context: &RetrievalContext, memory: &Memory) -> Result<f32> {
        let breakdown = self.calculate_score_breakdown(context, memory)?;

        let mut total_score = 0.0;
        total_score += breakdown.semantic_score * self.config.semantic_weight;
        total_score += breakdown.temporal_score * self.config.temporal_weight;
        total_score += breakdown.associative_score * self.config.associative_weight;
        total_score += breakdown.importance_score * 0.2; // Fixed weight for importance

        if self.config.recency_boost {
            total_score += breakdown.recency_score * 0.1; // Fixed weight for recency
        }

        Ok(total_score.min(1.0))
    }

    /// Calculate detailed score breakdown
    fn calculate_score_breakdown(
        &self,
        context: &RetrievalContext,
        memory: &Memory,
    ) -> Result<ScoreBreakdown> {
        let semantic_score =
            self.calculate_semantic_similarity(&context.query, &memory.content.text);
        let temporal_score = self.calculate_temporal_score(context, memory);
        let associative_score = self.calculate_associative_score(context, memory);
        let importance_score = memory.metadata.importance;
        let recency_score = self.calculate_recency_score(memory);

        Ok(ScoreBreakdown {
            semantic_score,
            temporal_score,
            associative_score,
            importance_score,
            recency_score,
        })
    }

    /// Calculate semantic similarity between query and memory content
    fn calculate_semantic_similarity(&self, query: &str, content: &str) -> f32 {
        // Simple word-based similarity (in practice, use embeddings)
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        if query_words.is_empty() || content_words.is_empty() {
            return 0.0;
        }

        let common_words = query_words
            .iter()
            .filter(|word| content_words.contains(word))
            .count();

        common_words as f32 / query_words.len() as f32
    }

    /// Calculate temporal proximity score
    fn calculate_temporal_score(&self, context: &RetrievalContext, memory: &Memory) -> f32 {
        if let Some(time_window) = &context.time_window {
            if memory.metadata.created_at >= time_window.start
                && memory.metadata.created_at <= time_window.end
            {
                return 1.0;
            }

            // Calculate distance from time window
            let distance_start = (memory.metadata.created_at - time_window.start)
                .whole_days()
                .abs();
            let distance_end = (memory.metadata.created_at - time_window.end)
                .whole_days()
                .abs();
            let min_distance = distance_start.min(distance_end) as f32;

            // Exponential decay based on distance
            (-min_distance / 7.0).exp()
        } else {
            0.5 // Neutral score if no temporal context
        }
    }

    /// Calculate associative strength score
    fn calculate_associative_score(&self, context: &RetrievalContext, memory: &Memory) -> f32 {
        let mut max_association_strength: f32 = 0.0;

        for recent_memory_id in &context.recent_memory_ids {
            if let Some(association) = memory
                .associations
                .iter()
                .find(|assoc| &assoc.memory_id == recent_memory_id)
            {
                max_association_strength = max_association_strength.max(association.strength);
            }
        }

        max_association_strength
    }

    /// Calculate recency score
    fn calculate_recency_score(&self, memory: &Memory) -> f32 {
        let now = time::OffsetDateTime::now_utc();
        let age_days = (now - memory.metadata.created_at).whole_days() as f32;
        let last_access_days = (now - memory.metadata.last_accessed).whole_days() as f32;

        // Combine creation recency and access recency
        let creation_recency = (-age_days / 30.0).exp(); // Decay over 30 days
        let access_recency = (-last_access_days / 7.0).exp(); // Decay over 7 days

        (creation_recency + access_recency) / 2.0
    }

    /// Retrieve memories through associative connections
    fn retrieve_associated_memories(
        &self,
        context: &RetrievalContext,
        all_memories: &[Memory],
        direct_results: &[RetrievalResult],
    ) -> Result<Vec<RetrievalResult>> {
        let mut associated_results = Vec::new();
        let mut processed_ids = std::collections::HashSet::new();

        // Collect IDs of direct results to avoid duplicates
        for result in direct_results {
            processed_ids.insert(result.memory.id.clone());
        }

        // Follow associations from direct results
        for result in direct_results {
            for association in &result.memory.associations {
                if processed_ids.contains(&association.memory_id) {
                    continue;
                }

                // Find the associated memory
                if let Some(associated_memory) =
                    all_memories.iter().find(|m| m.id == association.memory_id)
                {
                    // Calculate relevance for associated memory
                    let base_relevance = self.calculate_relevance(context, associated_memory)?;
                    let association_boost = association.strength * 0.3; // Boost from association
                    let final_relevance = (base_relevance + association_boost).min(1.0);

                    if final_relevance >= self.config.relevance_threshold {
                        let score_breakdown =
                            self.calculate_score_breakdown(context, associated_memory)?;

                        associated_results.push(RetrievalResult {
                            memory: associated_memory.clone(),
                            relevance_score: final_relevance,
                            score_breakdown,
                            retrieval_path: RetrievalPath::Associative {
                                source_memory_id: result.memory.id.clone(),
                            },
                        });

                        processed_ids.insert(association.memory_id.clone());
                    }
                }
            }
        }

        Ok(associated_results)
    }

    /// Find memories similar to a given memory
    pub fn find_similar(
        &self,
        target_memory: &Memory,
        all_memories: &[Memory],
    ) -> Result<Vec<RetrievalResult>> {
        let mut results = Vec::new();

        for memory in all_memories {
            if memory.id == target_memory.id {
                continue; // Skip the target memory itself
            }

            let similarity = self.calculate_memory_similarity(target_memory, memory)?;

            if similarity >= self.config.relevance_threshold {
                let score_breakdown = ScoreBreakdown {
                    semantic_score: similarity,
                    temporal_score: 0.0,
                    associative_score: 0.0,
                    importance_score: memory.metadata.importance,
                    recency_score: self.calculate_recency_score(memory),
                };

                results.push(RetrievalResult {
                    memory: memory.clone(),
                    relevance_score: similarity,
                    score_breakdown,
                    retrieval_path: RetrievalPath::Direct,
                });
            }
        }

        // Sort by similarity
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(self.config.max_results);
        Ok(results)
    }

    /// Calculate similarity between two memories
    fn calculate_memory_similarity(&self, memory1: &Memory, memory2: &Memory) -> Result<f32> {
        let mut similarity = 0.0;

        // Text similarity
        let text_sim =
            self.calculate_semantic_similarity(&memory1.content.text, &memory2.content.text);
        similarity += text_sim * 0.5;

        // Type similarity
        if memory1.memory_type == memory2.memory_type {
            similarity += 0.2;
        }

        // Location similarity
        if memory1.content.context.location == memory2.content.context.location
            && memory1.content.context.location.is_some()
        {
            similarity += 0.1;
        }

        // Participant overlap
        let common_participants = memory1
            .content
            .context
            .participants
            .iter()
            .filter(|p| memory2.content.context.participants.contains(p))
            .count();

        if !memory1.content.context.participants.is_empty() {
            let participant_sim =
                common_participants as f32 / memory1.content.context.participants.len() as f32;
            similarity += participant_sim * 0.2;
        }

        Ok(similarity.min(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieval_engine_creation() {
        let config = RetrievalConfig::default();
        let engine = RetrievalEngine::new(config);
        assert_eq!(engine.config.max_results, 10);
    }

    #[test]
    fn test_semantic_similarity() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("hello world", "world hello");
        assert!(similarity > 0.0);
    }

    #[test]
    fn test_memory_retrieval() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memories = vec![
            Memory::episodic(
                "Went to the park with Alice".to_string(),
                vec!["Alice".to_string()],
                Some("park".to_string()),
            ),
            Memory::semantic("The sky is blue".to_string(), "color".to_string()),
        ];

        let context = RetrievalContext {
            query: "park".to_string(),
            emotional_state: None,
            location: Some("park".to_string()),
            recent_memory_ids: Vec::new(),
            preferred_types: vec![MemoryType::Episodic],
            time_window: None,
            limit: 5,
        };

        let results = engine.retrieve(&context, &memories).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_find_similar_memories() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let target_memory = Memory::episodic(
            "Had lunch with Bob".to_string(),
            vec!["Bob".to_string()],
            Some("restaurant".to_string()),
        );

        let all_memories = vec![
            Memory::episodic(
                "Had dinner with Bob".to_string(),
                vec!["Bob".to_string()],
                Some("restaurant".to_string()),
            ),
            Memory::semantic("Cats are independent".to_string(), "animals".to_string()),
        ];

        let results = engine.find_similar(&target_memory, &all_memories).unwrap();
        assert!(!results.is_empty());
    }

    // ==================== NEW COMPREHENSIVE TESTS ====================

    #[test]
    fn test_retrieval_config_default_values() {
        let config = RetrievalConfig::default();

        assert_eq!(config.max_results, 10);
        assert_eq!(config.relevance_threshold, 0.3);
        assert_eq!(config.semantic_weight, 0.6);
        assert_eq!(config.temporal_weight, 0.2);
        assert_eq!(config.associative_weight, 0.2);
        assert!(config.recency_boost);
        assert!(config.follow_associations);
    }

    #[test]
    fn test_retrieval_config_custom() {
        let config = RetrievalConfig {
            max_results: 20,
            relevance_threshold: 0.5,
            semantic_weight: 0.8,
            temporal_weight: 0.1,
            associative_weight: 0.1,
            recency_boost: false,
            follow_associations: false,
        };

        let engine = RetrievalEngine::new(config);
        assert_eq!(engine.config.max_results, 20);
        assert_eq!(engine.config.relevance_threshold, 0.5);
        assert!(!engine.config.recency_boost);
    }

    #[test]
    fn test_retrieval_config_serialization() {
        let config = RetrievalConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RetrievalConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.max_results, deserialized.max_results);
        assert_eq!(config.semantic_weight, deserialized.semantic_weight);
    }

    #[test]
    fn test_retrieval_result_serialization() {
        let memory = Memory::sensory("Test".to_string(), None);
        let result = RetrievalResult {
            memory,
            relevance_score: 0.8,
            score_breakdown: ScoreBreakdown {
                semantic_score: 0.9,
                temporal_score: 0.5,
                associative_score: 0.3,
                importance_score: 0.7,
                recency_score: 0.6,
            },
            retrieval_path: RetrievalPath::Direct,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: RetrievalResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.relevance_score, deserialized.relevance_score);
        assert_eq!(
            result.score_breakdown.semantic_score,
            deserialized.score_breakdown.semantic_score
        );
    }

    #[test]
    fn test_score_breakdown_serialization() {
        let breakdown = ScoreBreakdown {
            semantic_score: 0.9,
            temporal_score: 0.5,
            associative_score: 0.3,
            importance_score: 0.7,
            recency_score: 0.6,
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        let deserialized: ScoreBreakdown = serde_json::from_str(&json).unwrap();

        assert_eq!(breakdown.semantic_score, deserialized.semantic_score);
        assert_eq!(breakdown.temporal_score, deserialized.temporal_score);
        assert_eq!(breakdown.associative_score, deserialized.associative_score);
        assert_eq!(breakdown.importance_score, deserialized.importance_score);
        assert_eq!(breakdown.recency_score, deserialized.recency_score);
    }

    #[test]
    fn test_retrieval_path_direct() {
        let path = RetrievalPath::Direct;
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: RetrievalPath = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, RetrievalPath::Direct));
    }

    #[test]
    fn test_retrieval_path_associative() {
        let path = RetrievalPath::Associative {
            source_memory_id: "memory_123".to_string(),
        };
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: RetrievalPath = serde_json::from_str(&json).unwrap();

        if let RetrievalPath::Associative { source_memory_id } = deserialized {
            assert_eq!(source_memory_id, "memory_123");
        } else {
            panic!("Expected Associative path");
        }
    }

    #[test]
    fn test_retrieval_path_temporal() {
        let time = time::OffsetDateTime::now_utc();
        let path = RetrievalPath::Temporal {
            reference_time: time,
        };
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: RetrievalPath = serde_json::from_str(&json).unwrap();

        if let RetrievalPath::Temporal { reference_time } = deserialized {
            assert_eq!(reference_time, time);
        } else {
            panic!("Expected Temporal path");
        }
    }

    #[test]
    fn test_retrieval_path_cluster() {
        let path = RetrievalPath::Cluster {
            cluster_id: "cluster_456".to_string(),
        };
        let json = serde_json::to_string(&path).unwrap();
        let deserialized: RetrievalPath = serde_json::from_str(&json).unwrap();

        if let RetrievalPath::Cluster { cluster_id } = deserialized {
            assert_eq!(cluster_id, "cluster_456");
        } else {
            panic!("Expected Cluster path");
        }
    }

    #[test]
    fn test_semantic_similarity_exact_match() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_semantic_similarity_no_match() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("apple banana", "cat dog");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_semantic_similarity_partial_match() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("hello world", "hello there");
        assert!(similarity > 0.0 && similarity < 1.0);
    }

    #[test]
    fn test_semantic_similarity_empty_query() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("", "hello world");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_semantic_similarity_empty_content() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("hello world", "");
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_semantic_similarity_case_insensitive() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let similarity = engine.calculate_semantic_similarity("HELLO WORLD", "hello world");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_recency_score_calculation() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        // Recent memory should have higher score
        let recent_memory = Memory::sensory("Recent".to_string(), None);
        let recency_score = engine.calculate_recency_score(&recent_memory);

        // Score should be close to 1.0 for very recent memories
        assert!(recency_score > 0.5);
    }

    #[test]
    fn test_temporal_score_no_time_window() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memory = Memory::sensory("Test".to_string(), None);

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let score = engine.calculate_temporal_score(&context, &memory);
        assert_eq!(score, 0.5); // Neutral score when no time window
    }

    #[test]
    fn test_temporal_score_within_window() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memory = Memory::sensory("Test".to_string(), None);

        let now = time::OffsetDateTime::now_utc();
        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: Some(TimeWindow {
                start: now - time::Duration::days(1),
                end: now + time::Duration::days(1),
            }),
            limit: 10,
        };

        let score = engine.calculate_temporal_score(&context, &memory);
        assert_eq!(score, 1.0); // Perfect score when within window
    }

    #[test]
    fn test_associative_score_no_associations() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memory = Memory::sensory("Test".to_string(), None);

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec!["other_memory".to_string()],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let score = engine.calculate_associative_score(&context, &memory);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_associative_score_with_associations() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        let mut memory = Memory::sensory("Test".to_string(), None);
        memory.add_association(
            "recent_memory".to_string(),
            AssociationType::Conceptual,
            0.8,
        );

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec!["recent_memory".to_string()],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let score = engine.calculate_associative_score(&context, &memory);
        assert_eq!(score, 0.8);
    }

    #[test]
    fn test_calculate_relevance() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memory = Memory::episodic(
            "Went to the park".to_string(),
            vec![],
            Some("park".to_string()),
        );

        let context = RetrievalContext {
            query: "park visit".to_string(),
            emotional_state: None,
            location: Some("park".to_string()),
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let relevance = engine.calculate_relevance(&context, &memory).unwrap();
        assert!((0.0..=1.0).contains(&relevance));
    }

    #[test]
    fn test_calculate_score_breakdown() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memory = Memory::sensory("Hello world".to_string(), None);

        let context = RetrievalContext {
            query: "hello world".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let breakdown = engine.calculate_score_breakdown(&context, &memory).unwrap();

        assert_eq!(breakdown.semantic_score, 1.0); // Exact match
        assert!(breakdown.recency_score >= 0.0 && breakdown.recency_score <= 1.0);
        assert!(breakdown.importance_score >= 0.0 && breakdown.importance_score <= 1.0);
    }

    #[test]
    fn test_retrieval_respects_limit() {
        let config = RetrievalConfig {
            max_results: 2,
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let memories: Vec<Memory> = (0..10)
            .map(|i| Memory::episodic(format!("Event {} happened", i), vec![], None))
            .collect();

        let context = RetrievalContext {
            query: "event happened".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 5, // Higher than config max_results
        };

        let results = engine.retrieve(&context, &memories).unwrap();
        assert!(results.len() <= 2); // Should respect config.max_results
    }

    #[test]
    fn test_retrieval_empty_memories() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let memories: Vec<Memory> = vec![];

        let context = RetrievalContext {
            query: "anything".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let results = engine.retrieve(&context, &memories).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_retrieval_below_threshold() {
        let config = RetrievalConfig {
            relevance_threshold: 0.99, // Very high threshold
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let memories = vec![Memory::semantic(
            "Cats are animals".to_string(),
            "biology".to_string(),
        )];

        let context = RetrievalContext {
            query: "completely unrelated query xyz".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let results = engine.retrieve(&context, &memories).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_retrieval_sorted_by_relevance() {
        let config = RetrievalConfig {
            relevance_threshold: 0.0, // Accept everything
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let memories = vec![
            Memory::episodic("event".to_string(), vec![], None), // Less relevant (1 word)
            Memory::episodic("event event event".to_string(), vec![], None), // More word matches
        ];

        let context = RetrievalContext {
            query: "event".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![MemoryType::Episodic],
            time_window: None,
            limit: 10,
        };

        let results = engine.retrieve(&context, &memories).unwrap();

        // Results should be sorted by relevance (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].relevance_score >= results[i].relevance_score);
        }
    }

    #[test]
    fn test_find_similar_excludes_target() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        let target = Memory::episodic("Target memory".to_string(), vec![], None);
        let target_clone = target.clone();

        let all_memories = vec![
            target.clone(),
            Memory::episodic("Similar memory".to_string(), vec![], None),
        ];

        let results = engine.find_similar(&target_clone, &all_memories).unwrap();

        // Target memory should not be in results
        assert!(!results.iter().any(|r| r.memory.id == target_clone.id));
    }

    #[test]
    fn test_calculate_memory_similarity_same_type() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        let memory1 = Memory::episodic("Test content".to_string(), vec![], None);
        let memory2 = Memory::episodic("Test content".to_string(), vec![], None);

        let similarity = engine
            .calculate_memory_similarity(&memory1, &memory2)
            .unwrap();

        // Should have bonus for same type
        assert!(similarity >= 0.5); // At least text + type bonus
    }

    #[test]
    fn test_calculate_memory_similarity_same_location() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        let memory1 = Memory::episodic("Event 1".to_string(), vec![], Some("park".to_string()));
        let memory2 = Memory::episodic("Event 2".to_string(), vec![], Some("park".to_string()));

        let similarity = engine
            .calculate_memory_similarity(&memory1, &memory2)
            .unwrap();

        // Should have bonus for same location
        assert!(similarity > 0.0);
    }

    #[test]
    fn test_calculate_memory_similarity_shared_participants() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        let memory1 = Memory::episodic(
            "Event with Alice".to_string(),
            vec!["Alice".to_string(), "Bob".to_string()],
            None,
        );
        let memory2 = Memory::episodic(
            "Event with Alice".to_string(),
            vec!["Alice".to_string(), "Charlie".to_string()],
            None,
        );

        let similarity = engine
            .calculate_memory_similarity(&memory1, &memory2)
            .unwrap();

        // Should have bonus for shared participants
        assert!(similarity > 0.0);
    }

    #[test]
    fn test_calculate_memory_similarity_caps_at_one() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());

        // Create memories that would score very high
        let memory1 = Memory::episodic(
            "same content".to_string(),
            vec!["Alice".to_string()],
            Some("park".to_string()),
        );
        let memory2 = Memory::episodic(
            "same content".to_string(),
            vec!["Alice".to_string()],
            Some("park".to_string()),
        );

        let similarity = engine
            .calculate_memory_similarity(&memory1, &memory2)
            .unwrap();

        assert!(similarity <= 1.0);
    }

    #[test]
    fn test_retrieval_with_associations_disabled() {
        let config = RetrievalConfig {
            follow_associations: false,
            relevance_threshold: 0.0,
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let mut memory1 = Memory::episodic("First event".to_string(), vec![], None);
        let mut memory2 = Memory::episodic("Second event".to_string(), vec![], None);

        // Create bidirectional association
        memory1.add_association(memory2.id.clone(), AssociationType::Conceptual, 0.9);
        memory2.add_association(memory1.id.clone(), AssociationType::Conceptual, 0.9);

        let memories = vec![memory1, memory2];

        let context = RetrievalContext {
            query: "first".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![MemoryType::Episodic],
            time_window: None,
            limit: 10,
        };

        let results = engine.retrieve(&context, &memories).unwrap();

        // With associations disabled, should only get direct matches
        // All results should have Direct retrieval path
        for result in &results {
            assert!(matches!(result.retrieval_path, RetrievalPath::Direct));
        }
    }

    #[test]
    fn test_retrieval_with_recency_boost_disabled() {
        let config = RetrievalConfig {
            recency_boost: false,
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let memory = Memory::sensory("Test".to_string(), None);
        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        // Should still calculate relevance, just without recency boost
        let relevance = engine.calculate_relevance(&context, &memory).unwrap();
        assert!(relevance >= 0.0);
    }

    #[test]
    fn test_find_similar_respects_max_results() {
        let config = RetrievalConfig {
            max_results: 3,
            relevance_threshold: 0.0, // Accept everything
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let target = Memory::episodic("Target".to_string(), vec![], None);
        let all_memories: Vec<Memory> = (0..10)
            .map(|i| Memory::episodic(format!("Memory {}", i), vec![], None))
            .collect();

        let results = engine.find_similar(&target, &all_memories).unwrap();
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_find_similar_sorted_by_similarity() {
        let config = RetrievalConfig {
            relevance_threshold: 0.0,
            ..Default::default()
        };
        let engine = RetrievalEngine::new(config);

        let target = Memory::episodic(
            "Meeting at the park".to_string(),
            vec!["Alice".to_string()],
            Some("park".to_string()),
        );

        let all_memories = vec![
            Memory::episodic("Random event".to_string(), vec![], None),
            Memory::episodic(
                "Meeting at the park".to_string(),
                vec!["Alice".to_string()],
                Some("park".to_string()),
            ),
            Memory::semantic("Facts".to_string(), "category".to_string()),
        ];

        let results = engine.find_similar(&target, &all_memories).unwrap();

        // Results should be sorted by relevance (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].relevance_score >= results[i].relevance_score);
        }
    }
}
