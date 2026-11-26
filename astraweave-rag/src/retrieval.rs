//! Memory retrieval and search functionality
//!
//! This module handles retrieving and searching through memories in the RAG system.

use anyhow::Result;
use astraweave_embeddings::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Retrieval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Maximum number of memories to retrieve
    pub max_results: usize,
    /// Minimum similarity threshold
    pub similarity_threshold: f32,
    /// Whether to use semantic search
    pub use_semantic_search: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            similarity_threshold: 0.7,
            use_semantic_search: true,
        }
    }
}

/// Query for memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQuery {
    /// Query text
    pub text: String,
    /// Categories to search in
    pub categories: Vec<MemoryCategory>,
    /// Additional filters
    pub filters: HashMap<String, String>,
    /// Maximum results to return
    pub limit: Option<usize>,
}

/// Result of a memory retrieval operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Retrieved memory
    pub memory: Memory,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Ranking position
    pub rank: usize,
}

/// Memory retrieval engine
#[derive(Debug)]
pub struct RetrievalEngine {
    /// Configuration
    config: RetrievalConfig,
}

impl RetrievalEngine {
    /// Create a new retrieval engine
    pub fn new(config: RetrievalConfig) -> Self {
        Self { config }
    }

    /// Search for memories matching a query
    pub fn search(
        &self,
        query: &RetrievalQuery,
        memories: &[Memory],
    ) -> Result<Vec<RetrievalResult>> {
        let mut results = Vec::new();

        for (index, memory) in memories.iter().enumerate() {
            // Category filtering
            if !query.categories.is_empty() && !query.categories.contains(&memory.category) {
                continue;
            }

            // Simple text similarity (in real implementation, use embeddings)
            let score = self.calculate_similarity(&query.text, &memory.text);

            if score >= self.config.similarity_threshold {
                results.push(RetrievalResult {
                    memory: memory.clone(),
                    score,
                    rank: index,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        let limit = query.limit.unwrap_or(self.config.max_results);
        results.truncate(limit);

        // Update ranks
        for (index, result) in results.iter_mut().enumerate() {
            result.rank = index;
        }

        Ok(results)
    }

    /// Calculate text similarity (simplified)
    fn calculate_similarity(&self, query: &str, content: &str) -> f32 {
        let query_words: Vec<&str> = query.split_whitespace().collect();
        let content_words: Vec<&str> = content.split_whitespace().collect();

        if query_words.is_empty() || content_words.is_empty() {
            return 0.0;
        }

        let common_words = query_words
            .iter()
            .filter(|word| content_words.contains(word))
            .count();

        common_words as f32 / query_words.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieval_engine() {
        let config = RetrievalConfig::default();
        let engine = RetrievalEngine::new(config);

        let memories = vec![
            Memory {
                id: "1".to_string(),
                text: "The cat sat on the mat".to_string(),
                category: MemoryCategory::Social,
                timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
                importance: 0.5,
                valence: 0.0,
                entities: vec![],
                context: HashMap::new(),
            },
            Memory {
                id: "2".to_string(),
                text: "Dogs are loyal animals".to_string(),
                category: MemoryCategory::Social,
                timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
                importance: 0.5,
                valence: 0.0,
                entities: vec![],
                context: HashMap::new(),
            },
        ];

        let query = RetrievalQuery {
            text: "cat".to_string(),
            categories: vec![MemoryCategory::Social],
            filters: HashMap::new(),
            limit: None,
        };

        let results = engine.search(&query, &memories).unwrap();
        assert!(!results.is_empty());
    }
}
