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
        let window_duration = temporal_window(self.config.temporal_window_hours);

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
                            temporal_strength(time_diff, window_duration),
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

/// A configured hour count becomes a duration the clock can represent.
///
/// An unusable window matches no pair. A finite count that does not fit
/// the duration range uses the largest duration. A count in `[0, 1)` hour
/// is zero seconds.
fn temporal_window(hours: f32) -> time::Duration {
    if !hours.is_finite() || hours < 0.0 {
        return time::Duration::new(-1, 0);
    }
    match (hours as i64).checked_mul(3_600) {
        Some(seconds) => time::Duration::seconds(seconds),
        None => time::Duration::MAX,
    }
}

/// Strength of a temporal link. A zero-second window does not divide.
///
/// An equal-time pair uses ratio 0, so the value before the association
/// clamp is 1.5. Any other pair uses the fraction of the window.
fn temporal_strength(time_diff: time::Duration, window: time::Duration) -> f32 {
    let window_seconds = window.whole_seconds();
    let ratio = if window_seconds == 0 {
        0.0
    } else {
        time_diff.whole_seconds().abs() as f32 / window_seconds as f32
    };
    0.5 + (1.0 - ratio)
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
        assert_eq!(memories[0].metadata.access_count, 1);
        assert_eq!(memories[1].metadata.access_count, 1);
    }

    #[test]
    fn test_conceptual_similarity() {
        let engine = ConsolidationEngine::new(ConsolidationConfig::default());

        let memory1 = Memory::semantic("Cats are mammals".to_string(), "animals".to_string());
        let memory2 = Memory::semantic("Dogs are mammals".to_string(), "animals".to_string());

        let similarity = engine.calculate_conceptual_similarity(&memory1, &memory2);
        assert!(similarity > 0.5); // Should be similar due to shared words and type
    }

    fn temporal_count(hours: f32) -> usize {
        let engine = ConsolidationEngine::new(ConsolidationConfig {
            temporal_window_hours: hours,
            ..ConsolidationConfig::default()
        });
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
        let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(30);
        memories[0].metadata.created_at = now;
        memories[1].metadata.created_at = now - time::Duration::days(10);
        engine
            .consolidate(&mut memories)
            .expect("consolidate")
            .temporal_associations
    }

    #[test]
    fn a_wide_temporal_window_still_links_memories() {
        assert_eq!(temporal_count(3e15_f32), 1);
    }

    #[test]
    fn an_unusable_temporal_window_links_nothing() {
        assert_eq!(temporal_count(f32::NAN), 0);
        assert_eq!(temporal_count(f32::INFINITY), 0);
        assert_eq!(temporal_count(f32::NEG_INFINITY), 0);
        assert_eq!(temporal_count(-0.5), 0);
    }

    #[test]
    fn a_zero_hour_window_links_memories_from_the_same_instant() {
        assert_eq!(same_instant_count(0.0), 1);
        assert_eq!(same_instant_count(-0.5), 0);
    }

    #[test]
    fn a_zero_second_window_stores_a_finite_strength() {
        for hours in [0.0_f32, 0.5] {
            let engine = ConsolidationEngine::new(ConsolidationConfig {
                temporal_window_hours: hours,
                ..ConsolidationConfig::default()
            });
            let mut memories = vec![
                Memory::episodic(
                    "north ridge".to_string(),
                    vec!["scout".to_string()],
                    Some("ridge".to_string()),
                ),
                Memory::episodic(
                    "south ford".to_string(),
                    vec!["courier".to_string()],
                    Some("ford".to_string()),
                ),
            ];
            let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(30);
            memories[0].metadata.created_at = now;
            memories[1].metadata.created_at = now;
            engine.consolidate(&mut memories).expect("consolidate");
            let strength = memories[0]
                .associations
                .iter()
                .find(|assoc| assoc.association_type == AssociationType::Temporal)
                .expect("temporal association")
                .strength;
            assert!(strength.is_finite());
            assert!((strength - 1.0).abs() < 1e-5);
            assert!(memories[0]
                .get_strong_associations(0.5)
                .iter()
                .any(|assoc| assoc.association_type == AssociationType::Temporal));
        }
    }

    fn same_instant_count(hours: f32) -> usize {
        let engine = ConsolidationEngine::new(ConsolidationConfig {
            temporal_window_hours: hours,
            ..ConsolidationConfig::default()
        });
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
        let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(30);
        memories[0].metadata.created_at = now;
        memories[1].metadata.created_at = now;
        engine
            .consolidate(&mut memories)
            .expect("consolidate")
            .temporal_associations
    }

    #[test]
    fn temporal_strength_uses_the_fraction_of_the_window() {
        let engine = ConsolidationEngine::new(ConsolidationConfig {
            temporal_window_hours: 24.0,
            ..ConsolidationConfig::default()
        });
        let mut memories = vec![
            Memory::episodic(
                "north ridge".to_string(),
                vec!["scout".to_string()],
                Some("ridge".to_string()),
            ),
            Memory::episodic(
                "south ford".to_string(),
                vec!["courier".to_string()],
                Some("ford".to_string()),
            ),
        ];
        let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(30);
        memories[0].metadata.created_at = now;
        memories[1].metadata.created_at = now - time::Duration::hours(18);
        engine.consolidate(&mut memories).expect("consolidate");
        let strength = memories[0]
            .associations
            .iter()
            .find(|assoc| assoc.association_type == AssociationType::Temporal)
            .expect("temporal association")
            .strength;
        assert!((strength - 0.75).abs() < 1e-5);
    }
}
