//! Memory compression and summarization
//!
//! This module handles compressing and summarizing memories to reduce storage
//! while preserving important information.

use crate::memory_types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for memory compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Minimum age in days before compression
    pub min_age_days: f32,
    /// Minimum importance threshold for compression
    pub importance_threshold: f32,
    /// Maximum compression ratio (0.0 to 1.0)
    pub max_compression_ratio: f32,
    /// Whether to preserve emotional context
    pub preserve_emotional_context: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_age_days: 30.0,
            importance_threshold: 0.3,
            max_compression_ratio: 0.5,
            preserve_emotional_context: true,
        }
    }
}

/// Result of compression operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionResult {
    /// Number of memories processed
    pub memories_processed: usize,
    /// Number of memories compressed
    pub memories_compressed: usize,
    /// Total size reduction in bytes (estimated)
    pub size_reduction: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Memory compression engine
#[derive(Debug)]
pub struct CompressionEngine {
    config: CompressionConfig,
}

impl CompressionEngine {
    /// Create a new compression engine
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Compress eligible memories
    pub fn compress_memories(&self, memories: &mut [Memory]) -> Result<CompressionResult> {
        let start_time = std::time::Instant::now();
        let mut result = CompressionResult::default();

        for memory in memories.iter_mut() {
            result.memories_processed += 1;

            if self.should_compress(memory)? {
                let original_size = self.estimate_memory_size(memory);
                self.compress_memory(memory)?;
                let compressed_size = self.estimate_memory_size(memory);

                result.memories_compressed += 1;
                result.size_reduction += original_size.saturating_sub(compressed_size);
            }
        }

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Check if a memory should be compressed
    fn should_compress(&self, memory: &Memory) -> Result<bool> {
        // Don't compress permanent memories
        if memory.metadata.permanent {
            return Ok(false);
        }

        // Check age
        let age_days =
            (time::OffsetDateTime::now_utc() - memory.metadata.created_at).whole_days() as f32;
        if age_days < self.config.min_age_days {
            return Ok(false);
        }

        // Check importance
        if memory.metadata.importance > self.config.importance_threshold {
            return Ok(false);
        }

        // Don't compress if already compressed
        if self.is_compressed(memory) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Compress a single memory
    fn compress_memory(&self, memory: &mut Memory) -> Result<()> {
        // Compress text content
        memory.content.text = self.compress_text(&memory.content.text);

        // Reduce sensory data if not preserving emotional context
        if !self.config.preserve_emotional_context {
            if let Some(ref mut sensory) = memory.content.sensory_data {
                sensory.visual = sensory.visual.as_ref().map(|v| self.compress_text(v));
                sensory.auditory = sensory.auditory.as_ref().map(|a| self.compress_text(a));
                sensory.tactile = sensory.tactile.as_ref().map(|t| self.compress_text(t));
                sensory.environmental = sensory
                    .environmental
                    .as_ref()
                    .map(|e| self.compress_text(e));
            }
        }

        // Reduce context information
        memory.content.context.related_events.truncate(3);
        memory.content.context.participants.truncate(5);

        // Mark as compressed
        memory.metadata.tags.push("compressed".to_string());

        Ok(())
    }

    /// Compress text content using simple summarization
    fn compress_text(&self, text: &str) -> String {
        if text.len() <= 50 {
            return text.to_string();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let target_length = (words.len() as f32 * self.config.max_compression_ratio) as usize;
        let compressed_length = target_length.max(10); // Minimum 10 words

        if compressed_length >= words.len() {
            return text.to_string();
        }

        // Simple compression: take first part and last part
        let first_part = words.len() / 3;
        let last_part = compressed_length - first_part;

        let mut compressed_words = Vec::new();
        compressed_words.extend_from_slice(&words[..first_part]);
        compressed_words.push("[...]");
        if last_part > 0 && words.len() > last_part {
            compressed_words.extend_from_slice(&words[words.len() - last_part..]);
        }

        compressed_words.join(" ")
    }

    /// Check if memory is already compressed
    fn is_compressed(&self, memory: &Memory) -> bool {
        memory.metadata.tags.contains(&"compressed".to_string())
    }

    /// Estimate memory size in bytes
    fn estimate_memory_size(&self, memory: &Memory) -> usize {
        let mut size = 0;

        // Text content
        size += memory.content.text.len();

        // Sensory data
        if let Some(ref sensory) = memory.content.sensory_data {
            size += sensory.visual.as_ref().map_or(0, |v| v.len());
            size += sensory.auditory.as_ref().map_or(0, |a| a.len());
            size += sensory.tactile.as_ref().map_or(0, |t| t.len());
            size += sensory.environmental.as_ref().map_or(0, |e| e.len());
        }

        // Context
        size += memory
            .content
            .context
            .location
            .as_ref()
            .map_or(0, |l| l.len());
        size += memory
            .content
            .context
            .time_period
            .as_ref()
            .map_or(0, |t| t.len());

        // Participants and events
        for participant in &memory.content.context.participants {
            size += participant.len();
        }
        for event in &memory.content.context.related_events {
            size += event.len();
        }

        // Tags
        for tag in &memory.metadata.tags {
            size += tag.len();
        }

        // Associations
        size += memory.associations.len() * 64; // Rough estimate per association

        // Embedding
        if let Some(ref embedding) = memory.embedding {
            size += embedding.len() * 4; // f32 = 4 bytes
        }

        size
    }

    /// Get compression statistics
    pub fn get_compression_stats(&self, memories: &[Memory]) -> CompressionStats {
        let total_memories = memories.len();
        let compressed_memories = memories.iter().filter(|m| self.is_compressed(m)).count();

        let total_size: usize = memories.iter().map(|m| self.estimate_memory_size(m)).sum();

        let avg_size = if total_memories > 0 {
            total_size / total_memories
        } else {
            0
        };

        let compression_ratio = if total_memories > 0 {
            compressed_memories as f32 / total_memories as f32
        } else {
            0.0
        };

        CompressionStats {
            total_memories,
            compressed_memories,
            total_size_bytes: total_size,
            average_size_bytes: avg_size,
            compression_ratio,
        }
    }
}

/// Statistics about memory compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub total_memories: usize,
    pub compressed_memories: usize,
    pub total_size_bytes: usize,
    pub average_size_bytes: usize,
    pub compression_ratio: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // CONFIG TESTS
    // ========================================================================

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.min_age_days, 30.0);
        assert_eq!(config.importance_threshold, 0.3);
        assert_eq!(config.max_compression_ratio, 0.5);
        assert!(config.preserve_emotional_context);
    }

    #[test]
    fn test_compression_config_serialization() {
        let config = CompressionConfig {
            min_age_days: 60.0,
            importance_threshold: 0.5,
            max_compression_ratio: 0.3,
            preserve_emotional_context: false,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CompressionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.min_age_days, 60.0);
        assert_eq!(deserialized.importance_threshold, 0.5);
        assert!(!deserialized.preserve_emotional_context);
    }

    // ========================================================================
    // ENGINE CREATION TESTS
    // ========================================================================

    #[test]
    fn test_compression_engine_creation() {
        let config = CompressionConfig::default();
        let engine = CompressionEngine::new(config);
        assert_eq!(engine.config.min_age_days, 30.0);
    }

    #[test]
    fn test_compression_engine_custom_config() {
        let config = CompressionConfig {
            min_age_days: 7.0,
            importance_threshold: 0.5,
            max_compression_ratio: 0.7,
            preserve_emotional_context: false,
        };
        let engine = CompressionEngine::new(config);
        assert_eq!(engine.config.min_age_days, 7.0);
        assert!(!engine.config.preserve_emotional_context);
    }

    // ========================================================================
    // COMPRESSION ELIGIBILITY TESTS
    // ========================================================================

    #[test]
    fn test_should_not_compress_permanent() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::semantic("Important fact".to_string(), "facts".to_string());
        memory.metadata.permanent = true;

        let should_compress = engine.should_compress(&memory).unwrap();
        assert!(!should_compress);
    }

    #[test]
    fn test_should_not_compress_young_memory() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let memory = Memory::sensory("Recent memory".to_string(), None);
        // Memory is too young (default is 30 days)

        let should_compress = engine.should_compress(&memory).unwrap();
        assert!(!should_compress);
    }

    #[test]
    fn test_should_not_compress_important_memory() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::episodic("Important event".to_string(), vec![], None);
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(35);
        memory.metadata.importance = 0.8; // High importance

        let should_compress = engine.should_compress(&memory).unwrap();
        assert!(!should_compress);
    }

    #[test]
    fn test_should_not_compress_already_compressed() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::sensory("Already compressed".to_string(), None);
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(35);
        memory.metadata.importance = 0.1;
        memory.metadata.tags.push("compressed".to_string());

        let should_compress = engine.should_compress(&memory).unwrap();
        assert!(!should_compress);
    }

    #[test]
    fn test_should_compress_eligible_memory() {
        let config = CompressionConfig {
            min_age_days: 7.0,
            importance_threshold: 0.5,
            max_compression_ratio: 0.5,
            preserve_emotional_context: true,
        };
        let engine = CompressionEngine::new(config);

        let mut memory = Memory::sensory("Old unimportant memory".to_string(), None);
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(10);
        memory.metadata.importance = 0.2;

        let should_compress = engine.should_compress(&memory).unwrap();
        assert!(should_compress);
    }

    // ========================================================================
    // SINGLE MEMORY COMPRESSION TESTS
    // ========================================================================

    #[test]
    fn test_memory_compression() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::sensory(
            "This is a very long memory text that should be compressed when it meets the criteria for compression".to_string(),
            None,
        );

        // Make memory old enough for compression
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(35);
        memory.metadata.importance = 0.2; // Low importance

        let original_length = memory.content.text.len();
        engine.compress_memory(&mut memory).unwrap();

        assert!(memory.content.text.len() < original_length);
        assert!(memory.metadata.tags.contains(&"compressed".to_string()));
    }

    #[test]
    fn test_compress_short_text() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let short_text = "Brief note";

        let compressed = engine.compress_text(short_text);
        assert_eq!(compressed, short_text);
    }

    #[test]
    fn test_compress_long_text() {
        let config = CompressionConfig {
            min_age_days: 0.0,
            importance_threshold: 1.0,
            max_compression_ratio: 0.3,
            preserve_emotional_context: true,
        };
        let engine = CompressionEngine::new(config);

        let long_text = "This is a very long memory text that needs to be compressed into a shorter form while preserving essential information about the event";
        let compressed = engine.compress_text(long_text);

        assert!(compressed.contains("[...]"));
        assert!(compressed.len() < long_text.len());
    }

    #[test]
    fn test_compress_text_at_threshold() {
        let config = CompressionConfig {
            min_age_days: 0.0,
            importance_threshold: 1.0,
            max_compression_ratio: 0.9,
            preserve_emotional_context: true,
        };
        let engine = CompressionEngine::new(config);

        // Short text that's close to threshold
        let text = "One two three four five six seven eight nine ten eleven twelve";
        let compressed = engine.compress_text(text);

        // Should still be processed
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_compress_with_sensory_data() {
        let config = CompressionConfig {
            min_age_days: 0.0,
            importance_threshold: 1.0,
            max_compression_ratio: 0.5,
            preserve_emotional_context: false, // Don't preserve
        };
        let engine = CompressionEngine::new(config);

        let sensory = SensoryData {
            visual: Some(
                "A very long visual description that needs compression into shorter form"
                    .to_string(),
            ),
            auditory: Some(
                "Long audio description with lots of detail that needs shortening".to_string(),
            ),
            tactile: Some("Detailed tactile feedback information to compress".to_string()),
            environmental: Some("Long environmental description with many details".to_string()),
        };

        let mut memory = Memory::sensory("Main content".to_string(), Some(sensory));
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(35);
        memory.metadata.importance = 0.1;

        engine.compress_memory(&mut memory).unwrap();

        // Sensory data should be compressed when preserve_emotional_context is false
        assert!(memory.metadata.tags.contains(&"compressed".to_string()));
    }

    #[test]
    fn test_compress_truncates_context() {
        let engine = CompressionEngine::new(CompressionConfig::default());

        let mut memory = Memory::episodic(
            "Event".to_string(),
            vec![
                "P1".into(),
                "P2".into(),
                "P3".into(),
                "P4".into(),
                "P5".into(),
                "P6".into(),
                "P7".into(),
            ],
            None,
        );
        memory.content.context.related_events = vec![
            "E1".into(),
            "E2".into(),
            "E3".into(),
            "E4".into(),
            "E5".into(),
        ];
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(35);
        memory.metadata.importance = 0.1;

        engine.compress_memory(&mut memory).unwrap();

        assert!(memory.content.context.participants.len() <= 5);
        assert!(memory.content.context.related_events.len() <= 3);
    }

    // ========================================================================
    // BATCH COMPRESSION TESTS
    // ========================================================================

    #[test]
    fn test_compress_memories_batch() {
        let config = CompressionConfig {
            min_age_days: 1.0,
            importance_threshold: 0.5,
            max_compression_ratio: 0.5,
            preserve_emotional_context: true,
        };
        let engine = CompressionEngine::new(config);

        let mut memories = vec![
            {
                let mut m = Memory::sensory(
                    "Long text that should be compressed into shorter form".to_string(),
                    None,
                );
                m.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(5);
                m.metadata.importance = 0.2;
                m
            },
            {
                let mut m = Memory::working("Short".to_string());
                m.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(5);
                m.metadata.importance = 0.2;
                m
            },
            Memory::semantic("Important".to_string(), "fact".to_string()), // Permanent, won't compress
        ];

        let result = engine.compress_memories(&mut memories).unwrap();

        assert_eq!(result.memories_processed, 3);
        assert!(result.memories_compressed > 0);
        let _ = result.processing_time_ms; // Verify it exists
    }

    #[test]
    fn test_compress_empty_batch() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memories: Vec<Memory> = vec![];

        let result = engine.compress_memories(&mut memories).unwrap();

        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_compressed, 0);
        assert_eq!(result.size_reduction, 0);
    }

    // ========================================================================
    // SIZE ESTIMATION TESTS
    // ========================================================================

    #[test]
    fn test_estimate_memory_size_basic() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let memory = Memory::sensory("Test content".to_string(), None);

        let size = engine.estimate_memory_size(&memory);

        assert!(size > 0);
        assert!(size >= "Test content".len());
    }

    #[test]
    fn test_estimate_memory_size_with_all_fields() {
        let engine = CompressionEngine::new(CompressionConfig::default());

        let sensory = SensoryData {
            visual: Some("Visual".to_string()),
            auditory: Some("Auditory".to_string()),
            tactile: Some("Tactile".to_string()),
            environmental: Some("Environmental".to_string()),
        };

        let mut memory = Memory::sensory("Content".to_string(), Some(sensory));
        memory.content.context.location = Some("Location".to_string());
        memory.content.context.time_period = Some("Morning".to_string());
        memory.content.context.participants = vec!["Alice".to_string(), "Bob".to_string()];
        memory.content.context.related_events = vec!["Event1".to_string()];
        memory.metadata.tags.push("tag1".to_string());
        memory.add_association("other".to_string(), AssociationType::Temporal, 0.5);
        memory.embedding = Some(vec![0.1, 0.2, 0.3, 0.4]);

        let size = engine.estimate_memory_size(&memory);

        // Should include all components
        assert!(size > "Content".len());
        assert!(size > 100); // Should be substantial with all fields
    }

    #[test]
    fn test_estimate_memory_size_with_embedding() {
        let engine = CompressionEngine::new(CompressionConfig::default());

        let mut memory = Memory::working("Text".to_string());
        memory.embedding = Some(vec![0.0; 384]); // Standard embedding size

        let size = engine.estimate_memory_size(&memory);

        // 384 floats * 4 bytes = 1536 bytes just for embedding
        assert!(size >= 1536);
    }

    // ========================================================================
    // STATISTICS TESTS
    // ========================================================================

    #[test]
    fn test_compression_stats() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let memories = vec![
            Memory::sensory("Test memory 1".to_string(), None),
            Memory::sensory("Test memory 2".to_string(), None),
        ];

        let stats = engine.get_compression_stats(&memories);
        assert_eq!(stats.total_memories, 2);
        assert_eq!(stats.compressed_memories, 0);
    }

    #[test]
    fn test_compression_stats_empty() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let memories: Vec<Memory> = vec![];

        let stats = engine.get_compression_stats(&memories);

        assert_eq!(stats.total_memories, 0);
        assert_eq!(stats.compressed_memories, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.average_size_bytes, 0);
        assert_eq!(stats.compression_ratio, 0.0);
    }

    #[test]
    fn test_compression_stats_with_compressed() {
        let engine = CompressionEngine::new(CompressionConfig::default());

        let mut compressed_memory = Memory::sensory("Compressed".to_string(), None);
        compressed_memory
            .metadata
            .tags
            .push("compressed".to_string());

        let memories = vec![
            Memory::sensory("Not compressed".to_string(), None),
            compressed_memory,
        ];

        let stats = engine.get_compression_stats(&memories);

        assert_eq!(stats.total_memories, 2);
        assert_eq!(stats.compressed_memories, 1);
        assert_eq!(stats.compression_ratio, 0.5);
    }

    // ========================================================================
    // COMPRESSION RESULT TESTS
    // ========================================================================

    #[test]
    fn test_compression_result_default() {
        let result = CompressionResult::default();

        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_compressed, 0);
        assert_eq!(result.size_reduction, 0);
        assert_eq!(result.processing_time_ms, 0);
    }

    #[test]
    fn test_compression_result_serialization() {
        let result = CompressionResult {
            memories_processed: 100,
            memories_compressed: 25,
            size_reduction: 50000,
            processing_time_ms: 150,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CompressionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.memories_processed, 100);
        assert_eq!(deserialized.memories_compressed, 25);
        assert_eq!(deserialized.size_reduction, 50000);
    }

    #[test]
    fn test_compression_stats_serialization() {
        let stats = CompressionStats {
            total_memories: 1000,
            compressed_memories: 300,
            total_size_bytes: 5000000,
            average_size_bytes: 5000,
            compression_ratio: 0.3,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: CompressionStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.total_memories, 1000);
        assert_eq!(deserialized.compression_ratio, 0.3);
    }

    // ========================================================================
    // IS_COMPRESSED TESTS
    // ========================================================================

    #[test]
    fn test_is_compressed_true() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::working("Test".to_string());
        memory.metadata.tags.push("compressed".to_string());

        assert!(engine.is_compressed(&memory));
    }

    #[test]
    fn test_is_compressed_false() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let memory = Memory::working("Test".to_string());

        assert!(!engine.is_compressed(&memory));
    }

    #[test]
    fn test_is_compressed_with_other_tags() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memory = Memory::working("Test".to_string());
        memory.metadata.tags.push("important".to_string());
        memory.metadata.tags.push("reviewed".to_string());

        assert!(!engine.is_compressed(&memory));
    }
}
