//! Memory consolidation and association formation
//!
//! This module handles the consolidation of related memories, forming associations
//! and strengthening important memory patterns.

use crate::memory_types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for memory consolidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Minimum similarity threshold for association formation
    pub association_threshold: f32,
    /// Time window for temporal associations (in hours)
    pub temporal_window_hours: f32,
    /// Maximum associations per memory
    pub max_associations: usize,
    /// Strength boost for consolidated memories
    pub consolidation_boost: f32,
    /// Enable automatic association formation
    pub auto_associations: bool,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            association_threshold: 0.7,
            temporal_window_hours: 24.0,
            max_associations: 10,
            consolidation_boost: 0.2,
            auto_associations: true,
        }
    }
}

/// Memory consolidation engine
#[derive(Debug)]
pub struct ConsolidationEngine {
    config: ConsolidationConfig,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine
    pub fn new(config: ConsolidationConfig) -> Self {
        Self { config }
    }

    /// Consolidate a collection of memories
    pub fn consolidate(&self, memories: &mut [Memory]) -> Result<ConsolidationResult> {
        let start_time = std::time::Instant::now();
        let mut result = ConsolidationResult {
            temporal_associations: self.form_temporal_associations(memories)?,
            spatial_associations: self.form_spatial_associations(memories)?,
            conceptual_associations: self.form_conceptual_associations(memories)?,
            ..Default::default()
        };

        // Update consolidation states
        for memory in memories.iter_mut() {
            self.update_consolidation_state(memory)?;
            result.memories_processed += 1;
        }

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Form temporal associations between memories
    fn form_temporal_associations(&self, memories: &mut [Memory]) -> Result<usize> {
        let mut associations_formed = 0;
        let window_duration = time::Duration::hours(self.config.temporal_window_hours as i64);

        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                let time_diff = memories[i].metadata.created_at - memories[j].metadata.created_at;

                if time_diff.abs() <= window_duration {
                    // Check if association already exists
                    let already_associated = memories[i]
                        .associations
                        .iter()
                        .any(|assoc| assoc.memory_id == memories[j].id);

                    if !already_associated
                        && memories[i].associations.len() < self.config.max_associations
                    {
                        memories[i].add_association(
                            memories[j].id.clone(),
                            AssociationType::Temporal,
                            0.5 + (1.0
                                - (time_diff.whole_seconds().abs() as f32
                                    / (window_duration.whole_seconds() as f32))),
                        );
                        associations_formed += 1;
                    }
                }
            }
        }

        Ok(associations_formed)
    }

    /// Form spatial associations between memories
    fn form_spatial_associations(&self, memories: &mut [Memory]) -> Result<usize> {
        let mut associations_formed = 0;

        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                // Check if memories share a location
                if let (Some(loc1), Some(loc2)) = (
                    &memories[i].content.context.location,
                    &memories[j].content.context.location,
                ) {
                    if loc1 == loc2 {
                        let already_associated = memories[i]
                            .associations
                            .iter()
                            .any(|assoc| assoc.memory_id == memories[j].id);

                        if !already_associated
                            && memories[i].associations.len() < self.config.max_associations
                        {
                            memories[i].add_association(
                                memories[j].id.clone(),
                                AssociationType::Spatial,
                                0.8, // High strength for spatial associations
                            );
                            associations_formed += 1;
                        }
                    }
                }
            }
        }

        Ok(associations_formed)
    }

    /// Form conceptual associations between memories
    fn form_conceptual_associations(&self, memories: &mut [Memory]) -> Result<usize> {
        let mut associations_formed = 0;

        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                let similarity = self.calculate_conceptual_similarity(&memories[i], &memories[j]);

                if similarity >= self.config.association_threshold {
                    let already_associated = memories[i]
                        .associations
                        .iter()
                        .any(|assoc| assoc.memory_id == memories[j].id);

                    if !already_associated
                        && memories[i].associations.len() < self.config.max_associations
                    {
                        memories[i].add_association(
                            memories[j].id.clone(),
                            AssociationType::Conceptual,
                            similarity,
                        );
                        associations_formed += 1;
                    }
                }
            }
        }

        Ok(associations_formed)
    }

    /// Calculate conceptual similarity between two memories
    fn calculate_conceptual_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        let mut similarity = 0.0;

        // Type similarity
        if memory1.memory_type == memory2.memory_type {
            similarity += 0.3;
        }

        // Content text similarity (simplified word overlap)
        let words1: Vec<&str> = memory1.content.text.split_whitespace().collect();
        let words2: Vec<&str> = memory2.content.text.split_whitespace().collect();

        let common_words = words1.iter().filter(|word| words2.contains(word)).count();

        if !words1.is_empty() && !words2.is_empty() {
            let text_similarity = common_words as f32 / words1.len().min(words2.len()) as f32;
            similarity += text_similarity * 0.5;
        }

        // Participant overlap
        let participants1: std::collections::HashSet<_> =
            memory1.content.context.participants.iter().collect();
        let participants2: std::collections::HashSet<_> =
            memory2.content.context.participants.iter().collect();
        let common_participants = participants1.intersection(&participants2).count();

        if !participants1.is_empty() || !participants2.is_empty() {
            let participant_similarity =
                common_participants as f32 / participants1.union(&participants2).count() as f32;
            similarity += participant_similarity * 0.2;
        }

        similarity.min(1.0)
    }

    /// Update consolidation state for a memory
    fn update_consolidation_state(&self, memory: &mut Memory) -> Result<()> {
        // Increase memory strength based on consolidation
        let boost = self.config.consolidation_boost;
        memory.metadata.strength = (memory.metadata.strength + boost).min(1.0);

        // Update access count (consolidation acts as access)
        memory.metadata.access_count += 1;
        memory.metadata.last_accessed = time::OffsetDateTime::now_utc();

        Ok(())
    }
}

/// Result of consolidation operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsolidationResult {
    /// Number of memories processed
    pub memories_processed: usize,
    /// Number of temporal associations formed
    pub temporal_associations: usize,
    /// Number of spatial associations formed
    pub spatial_associations: usize,
    /// Number of conceptual associations formed
    pub conceptual_associations: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

impl ConsolidationResult {
    /// Get total associations formed
    pub fn total_associations(&self) -> usize {
        self.temporal_associations + self.spatial_associations + self.conceptual_associations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consolidation_engine() {
        let config = ConsolidationConfig::default();
        let engine = ConsolidationEngine::new(config);

        let mut memories = vec![
            Memory::episodic(
                "Met John at the park".to_string(),
                vec!["John".to_string()],
                Some("park".to_string()),
            ),
            Memory::episodic(
                "Played with John in the park".to_string(),
                vec!["John".to_string()],
                Some("park".to_string()),
            ),
        ];

        let result = engine.consolidate(&mut memories).unwrap();
        assert!(result.total_associations() > 0);
        assert!(result.memories_processed == 2);
    }

    #[test]
    fn test_conceptual_similarity() {
        let engine = ConsolidationEngine::new(ConsolidationConfig::default());

        let memory1 = Memory::semantic("Cats are mammals".to_string(), "animals".to_string());
        let memory2 = Memory::semantic("Dogs are mammals".to_string(), "animals".to_string());

        let similarity = engine.calculate_conceptual_similarity(&memory1, &memory2);
        assert!(similarity > 0.5); // Should be similar due to shared words and type
    }
}
