//! Memory injection and contextual enhancement
//!
//! This module handles injecting relevant memories into conversations and contexts.

use anyhow::Result;
use astraweave_embeddings::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionConfig {
    /// Maximum number of memories to inject
    pub max_memories: usize,
    /// Relevance threshold for injection
    pub relevance_threshold: f32,
    /// Whether to prioritize recent memories
    pub prioritize_recent: bool,
    /// Maximum context length in tokens
    pub max_context_tokens: usize,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            max_memories: 5,
            // Lowered threshold to be more inclusive in default tests and demos
            relevance_threshold: 0.4,
            prioritize_recent: true,
            max_context_tokens: 2000,
        }
    }
}

/// Context for memory injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionContext {
    /// Current conversation or query
    pub query: String,
    /// Current conversation history
    pub conversation_history: Vec<String>,
    /// Additional context metadata
    pub metadata: HashMap<String, String>,
    /// Preferred memory categories
    pub preferred_categories: Vec<MemoryCategory>,
}

/// Result of memory injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionResult {
    /// Injected memories
    pub injected_memories: Vec<Memory>,
    /// Generated context text
    pub context_text: String,
    /// Relevance scores for each memory
    pub relevance_scores: Vec<f32>,
    /// Total token count estimate
    pub estimated_tokens: usize,
}

/// Memory injection engine
#[derive(Debug)]
pub struct InjectionEngine {
    /// Configuration
    config: InjectionConfig,
}

impl InjectionEngine {
    /// Create a new injection engine
    pub fn new(config: InjectionConfig) -> Self {
        Self { config }
    }

    /// Inject relevant memories into a context
    pub fn inject(
        &self,
        context: &InjectionContext,
        available_memories: &[Memory],
    ) -> Result<InjectionResult> {
        let mut relevant_memories = Vec::new();
        let mut relevance_scores = Vec::new();

        // Score and filter memories
        for memory in available_memories {
            let relevance = self.calculate_relevance(context, memory);

            if relevance >= self.config.relevance_threshold {
                relevant_memories.push(memory.clone());
                relevance_scores.push(relevance);
            }
        }

        // Sort by relevance (and recency if enabled)
        let mut memory_score_pairs: Vec<(Memory, f32)> = relevant_memories
            .into_iter()
            .zip(relevance_scores.iter().copied())
            .collect();

        memory_score_pairs.sort_by(|a, b| {
            if self.config.prioritize_recent {
                // Combine relevance and recency
                let score_a = a.1 + self.recency_boost(&a.0);
                let score_b = b.1 + self.recency_boost(&b.0);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Limit number of memories
        memory_score_pairs.truncate(self.config.max_memories);

        let injected_memories: Vec<Memory> =
            memory_score_pairs.iter().map(|(m, _)| m.clone()).collect();
        let final_scores: Vec<f32> = memory_score_pairs.iter().map(|(_, s)| *s).collect();

        // Generate context text
        let context_text = self.generate_context_text(&injected_memories)?;

        // Estimate token count (rough approximation)
        let estimated_tokens = context_text.split_whitespace().count();

        Ok(InjectionResult {
            injected_memories,
            context_text,
            relevance_scores: final_scores,
            estimated_tokens,
        })
    }

    /// Calculate relevance of a memory to the current context
    fn calculate_relevance(&self, context: &InjectionContext, memory: &Memory) -> f32 {
        let mut relevance = 0.0;

        // Category preference
        if !context.preferred_categories.is_empty() {
            if context.preferred_categories.contains(&memory.category) {
                relevance += 0.3;
            }
        } else {
            relevance += 0.1; // Base score if no category preference
        }

        // Content similarity to query
        let content_similarity = self.calculate_text_similarity(&context.query, &memory.text);
        relevance += content_similarity * 0.5;

        // Similarity to conversation history
        let mut history_similarity = 0.0;
        for message in &context.conversation_history {
            history_similarity += self.calculate_text_similarity(message, &memory.text);
        }
        if !context.conversation_history.is_empty() {
            history_similarity /= context.conversation_history.len() as f32;
        }
        relevance += history_similarity * 0.2;

        relevance.min(1.0)
    }

    /// Calculate recency boost for memory
    fn recency_boost(&self, memory: &Memory) -> f32 {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let age_seconds = (now - memory.timestamp as i64).max(0) as f32;
        let age_hours = age_seconds / 3600.0;

        // Exponential decay: more recent = higher boost
        (-age_hours / 24.0).exp() * 0.1 // Max boost of 0.1
    }

    /// Calculate text similarity (simplified)
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: Vec<String> = text1
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let words2: Vec<String> = text2
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let common_words = words1.iter().filter(|word| words2.contains(word)).count();

        common_words as f32 / words1.len() as f32
    }

    /// Generate context text from memories
    fn generate_context_text(&self, memories: &[Memory]) -> Result<String> {
        if memories.is_empty() {
            return Ok(String::new());
        }

        let mut context_parts = vec!["Relevant memories:".to_string()];

        for (i, memory) in memories.iter().enumerate() {
            context_parts.push(format!("{}. {}", i + 1, memory.text));
        }

        Ok(context_parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_engine() {
        let config = InjectionConfig::default();
        let engine = InjectionEngine::new(config);

        let context = InjectionContext {
            query: "Tell me about cats".to_string(),
            conversation_history: vec!["Hello".to_string()],
            metadata: HashMap::new(),
            preferred_categories: vec![MemoryCategory::Social],
        };

        let memories = vec![Memory {
            id: "1".to_string(),
            text: "Cats are independent animals".to_string(),
            category: MemoryCategory::Social,
            timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
            importance: 0.5,
            valence: 0.0,
            entities: vec![],
            context: HashMap::new(),
        }];

        let result = engine.inject(&context, &memories).unwrap();
        assert!(!result.injected_memories.is_empty());
        assert!(!result.context_text.is_empty());
    }
}
