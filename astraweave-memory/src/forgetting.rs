//! Memory forgetting and decay mechanisms
//!
//! This module implements sophisticated forgetting algorithms based on
//! memory strength, importance, and access patterns.

use crate::memory_types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for forgetting mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingConfig {
    /// Base decay rate for memory strength
    pub base_decay_rate: f32,
    /// Minimum strength for memory retention
    pub retention_threshold: f32,
    /// Importance multiplier for decay rate
    pub importance_factor: f32,
    /// Access frequency impact on forgetting
    pub access_factor: f32,
    /// Enable spaced repetition effects
    pub spaced_repetition: bool,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            base_decay_rate: 0.1,
            retention_threshold: 0.15,
            importance_factor: 0.5,
            access_factor: 0.3,
            spaced_repetition: true,
        }
    }
}

/// Memory forgetting engine
#[derive(Debug)]
pub struct ForgettingEngine {
    config: ForgettingConfig,
    forgetting_curves: HashMap<MemoryType, ForgettingCurve>,
}

impl ForgettingEngine {
    /// Create a new forgetting engine
    pub fn new(config: ForgettingConfig) -> Self {
        let mut forgetting_curves = HashMap::new();

        // Default forgetting curves for different memory types
        forgetting_curves.insert(
            MemoryType::Sensory,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 2.0,
                half_life: 0.25, // 6 hours
                retention_threshold: 0.1,
                immune: false,
            },
        );

        forgetting_curves.insert(
            MemoryType::Working,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 1.0,
                half_life: 1.0, // 1 day
                retention_threshold: 0.2,
                immune: false,
            },
        );

        forgetting_curves.insert(
            MemoryType::Episodic,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 0.2,
                half_life: 14.0, // 2 weeks
                retention_threshold: 0.15,
                immune: false,
            },
        );

        forgetting_curves.insert(
            MemoryType::Semantic,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 0.05,
                half_life: 180.0, // 6 months
                retention_threshold: 0.1,
                immune: true,
            },
        );

        forgetting_curves.insert(
            MemoryType::Procedural,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 0.1,
                half_life: 30.0, // 1 month
                retention_threshold: 0.12,
                immune: false,
            },
        );

        forgetting_curves.insert(
            MemoryType::Emotional,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 0.15,
                half_life: 7.0, // 1 week
                retention_threshold: 0.18,
                immune: false,
            },
        );

        forgetting_curves.insert(
            MemoryType::Social,
            ForgettingCurve {
                initial_strength: 1.0,
                decay_rate: 0.12,
                half_life: 21.0, // 3 weeks
                retention_threshold: 0.15,
                immune: false,
            },
        );

        Self {
            config,
            forgetting_curves,
        }
    }

    /// Apply forgetting to a collection of memories
    pub fn apply_forgetting(&self, memories: &mut Vec<Memory>) -> Result<ForgettingResult> {
        let start_time = std::time::Instant::now();
        let mut result = ForgettingResult::default();
        let mut memories_to_remove = Vec::new();

        for (index, memory) in memories.iter_mut().enumerate() {
            // Skip permanent memories
            if memory.metadata.permanent {
                continue;
            }

            // Update memory strength based on decay
            let old_strength = memory.metadata.strength;
            self.update_memory_strength(memory)?;

            result.memories_processed += 1;
            result.total_strength_lost += old_strength - memory.metadata.strength;

            // Check if memory should be forgotten
            if self.should_forget(memory)? {
                memories_to_remove.push(index);
                result.memories_forgotten += 1;
            }
        }

        // Remove memories in reverse order to maintain indices
        for &index in memories_to_remove.iter().rev() {
            memories.remove(index);
        }

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Update memory strength based on decay and usage patterns
    fn update_memory_strength(&self, memory: &mut Memory) -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
        let age_days = (now - memory.metadata.created_at).whole_days() as f32;
        let _last_access_days = (now - memory.metadata.last_accessed).whole_days() as f32;

        // Get forgetting curve for this memory type
        let default_curve = ForgettingCurve {
            initial_strength: 1.0,
            decay_rate: self.config.base_decay_rate,
            half_life: 7.0,
            retention_threshold: self.config.retention_threshold,
            immune: false,
        };
        let curve = self
            .forgetting_curves
            .get(&memory.memory_type)
            .unwrap_or(&default_curve);

        // Calculate base decay
        let decay_factor = if curve.half_life > 0.0 {
            (-0.693 * age_days / curve.half_life).exp()
        } else {
            (-curve.decay_rate * age_days).exp()
        };

        // Apply importance modifier
        let importance_modifier =
            1.0 + (memory.metadata.importance - 0.5) * self.config.importance_factor;

        // Apply access frequency modifier
        let access_modifier = if memory.metadata.access_count > 0 {
            let access_frequency = memory.metadata.access_count as f32 / age_days.max(1.0);
            1.0 + (access_frequency * self.config.access_factor)
        } else {
            1.0
        };

        // Apply spaced repetition effects
        let spaced_repetition_modifier =
            if self.config.spaced_repetition && memory.metadata.access_count > 1 {
                // Memories that are accessed multiple times decay more slowly
                let repetition_factor = (memory.metadata.access_count as f32).ln() * 0.1;
                1.0 + repetition_factor
            } else {
                1.0
            };

        // Calculate new strength
        let new_strength = curve.initial_strength
            * decay_factor
            * importance_modifier
            * access_modifier
            * spaced_repetition_modifier;

        memory.metadata.strength = new_strength.clamp(0.0, 1.0);

        Ok(())
    }

    /// Determine if a memory should be forgotten
    fn should_forget(&self, memory: &Memory) -> Result<bool> {
        // Never forget permanent memories
        if memory.metadata.permanent {
            return Ok(false);
        }

        // Check against type-specific curve
        if let Some(curve) = self.forgetting_curves.get(&memory.memory_type) {
            if curve.immune {
                return Ok(false);
            }
            return Ok(memory.metadata.strength < curve.retention_threshold);
        }

        // Default threshold check
        Ok(memory.metadata.strength < self.config.retention_threshold)
    }

    /// Calculate memory half-life based on access patterns
    pub fn calculate_adaptive_half_life(&self, memory: &Memory) -> f32 {
        let base_half_life = self
            .forgetting_curves
            .get(&memory.memory_type)
            .map(|curve| curve.half_life)
            .unwrap_or(7.0);

        // Adjust based on access frequency
        let access_modifier = if memory.metadata.access_count > 1 {
            let access_factor = (memory.metadata.access_count as f32).ln();
            1.0 + (access_factor * 0.5)
        } else {
            1.0
        };

        // Adjust based on importance
        let importance_modifier = 0.5 + memory.metadata.importance;

        base_half_life * access_modifier * importance_modifier
    }

    /// Get forgetting statistics for a memory type
    pub fn get_type_statistics(
        &self,
        memory_type: &MemoryType,
        memories: &[Memory],
    ) -> TypeForgettingStats {
        let type_memories: Vec<_> = memories
            .iter()
            .filter(|m| &m.memory_type == memory_type)
            .collect();

        if type_memories.is_empty() {
            return TypeForgettingStats::default();
        }

        let total_strength: f32 = type_memories.iter().map(|m| m.metadata.strength).sum();
        let avg_strength = total_strength / type_memories.len() as f32;

        let weak_memories = type_memories
            .iter()
            .filter(|m| {
                let threshold = self
                    .forgetting_curves
                    .get(memory_type)
                    .map(|curve| curve.retention_threshold)
                    .unwrap_or(self.config.retention_threshold);
                m.metadata.strength < threshold
            })
            .count();

        let avg_age_days = type_memories
            .iter()
            .map(|m| (time::OffsetDateTime::now_utc() - m.metadata.created_at).whole_days() as f32)
            .sum::<f32>()
            / type_memories.len() as f32;

        TypeForgettingStats {
            memory_type: memory_type.clone(),
            total_memories: type_memories.len(),
            average_strength: avg_strength,
            weak_memories,
            average_age_days: avg_age_days,
        }
    }
}

/// Result of forgetting operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgettingResult {
    /// Number of memories processed
    pub memories_processed: usize,
    /// Number of memories forgotten
    pub memories_forgotten: usize,
    /// Total strength lost across all memories
    pub total_strength_lost: f32,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Statistics for memory forgetting by type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypeForgettingStats {
    pub memory_type: MemoryType,
    pub total_memories: usize,
    pub average_strength: f32,
    pub weak_memories: usize,
    pub average_age_days: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a Working memory (no public constructor exists)
    fn create_working_memory(text: &str) -> Memory {
        let content = MemoryContent {
            text: text.to_string(),
            data: serde_json::json!({}),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: vec![],
                related_events: vec![],
            },
        };
        Memory::new(MemoryType::Working, content)
    }

    #[test]
    fn test_forgetting_engine_creation() {
        let config = ForgettingConfig::default();
        let engine = ForgettingEngine::new(config);
        assert!(engine.forgetting_curves.contains_key(&MemoryType::Sensory));
    }

    #[test]
    fn test_memory_strength_decay() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = Memory::sensory("Test memory".to_string(), None);

        // Artificially age the memory
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(30);

        let initial_strength = memory.metadata.strength;
        engine.update_memory_strength(&mut memory).unwrap();

        assert!(memory.metadata.strength < initial_strength);
    }

    #[test]
    fn test_permanent_memory_protection() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = Memory::semantic("Important fact".to_string(), "facts".to_string());
        memory.metadata.permanent = true;
        memory.metadata.strength = 0.05; // Very weak

        let should_forget = engine.should_forget(&memory).unwrap();
        assert!(!should_forget);
    }

    #[test]
    fn test_spaced_repetition_effect() {
        let config = ForgettingConfig {
            spaced_repetition: true,
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config);

        let mut memory1 = Memory::episodic("Test".to_string(), vec![], None);
        let mut memory2 = Memory::episodic("Test".to_string(), vec![], None);

        // Age both memories equally
        let old_date = time::OffsetDateTime::now_utc() - time::Duration::days(10);
        memory1.metadata.created_at = old_date;
        memory2.metadata.created_at = old_date;

        // Memory2 has been accessed multiple times
        memory2.metadata.access_count = 5;

        engine.update_memory_strength(&mut memory1).unwrap();
        engine.update_memory_strength(&mut memory2).unwrap();

        // Memory2 should be stronger due to spaced repetition
        assert!(memory2.metadata.strength > memory1.metadata.strength);
    }

    // ======================= NEW TESTS =======================

    // --- ForgettingConfig Tests ---
    #[test]
    fn test_forgetting_config_default_values() {
        let config = ForgettingConfig::default();
        assert!((config.base_decay_rate - 0.1).abs() < 0.001);
        assert!((config.retention_threshold - 0.15).abs() < 0.001);
        assert!((config.importance_factor - 0.5).abs() < 0.001);
        assert!((config.access_factor - 0.3).abs() < 0.001);
        assert!(config.spaced_repetition);
    }

    #[test]
    fn test_forgetting_config_custom_values() {
        let config = ForgettingConfig {
            base_decay_rate: 0.2,
            retention_threshold: 0.3,
            importance_factor: 0.8,
            access_factor: 0.5,
            spaced_repetition: false,
        };
        assert!((config.base_decay_rate - 0.2).abs() < 0.001);
        assert!(!config.spaced_repetition);
    }

    // --- ForgettingEngine curve setup ---
    #[test]
    fn test_forgetting_engine_all_memory_types_have_curves() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        assert!(engine.forgetting_curves.contains_key(&MemoryType::Sensory));
        assert!(engine.forgetting_curves.contains_key(&MemoryType::Working));
        assert!(engine.forgetting_curves.contains_key(&MemoryType::Episodic));
        assert!(engine.forgetting_curves.contains_key(&MemoryType::Semantic));
        assert!(engine
            .forgetting_curves
            .contains_key(&MemoryType::Procedural));
        assert!(engine
            .forgetting_curves
            .contains_key(&MemoryType::Emotional));
        assert!(engine.forgetting_curves.contains_key(&MemoryType::Social));
    }

    #[test]
    fn test_semantic_memory_curve_is_immune() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let curve = engine.forgetting_curves.get(&MemoryType::Semantic).unwrap();
        assert!(curve.immune);
    }

    #[test]
    fn test_sensory_memory_curve_fast_decay() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let curve = engine.forgetting_curves.get(&MemoryType::Sensory).unwrap();
        assert!((curve.half_life - 0.25).abs() < 0.01); // 6 hours
        assert!(curve.decay_rate > 1.0); // Fast decay
    }

    // --- apply_forgetting Tests (main public method) ---
    #[test]
    fn test_apply_forgetting_empty_vec() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories: Vec<Memory> = vec![];

        let result = engine.apply_forgetting(&mut memories).unwrap();

        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_forgotten, 0);
        assert!((result.total_strength_lost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_apply_forgetting_fresh_memories_stay() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories = vec![
            Memory::sensory("Fresh memory".to_string(), None),
            create_working_memory("Working memory"),
            Memory::episodic("Episodic".to_string(), vec![], None),
        ];

        let result = engine.apply_forgetting(&mut memories).unwrap();

        // Fresh memories should not be forgotten
        assert_eq!(result.memories_processed, 3);
        assert_eq!(result.memories_forgotten, 0);
        assert_eq!(memories.len(), 3);
    }

    #[test]
    fn test_apply_forgetting_old_weak_memories_removed() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = Memory::sensory("Very old memory".to_string(), None);

        // Age and weaken the memory significantly
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(365);
        memory.metadata.strength = 0.01; // Below any threshold

        let mut memories = vec![memory];
        let result = engine.apply_forgetting(&mut memories).unwrap();

        assert_eq!(result.memories_processed, 1);
        assert_eq!(result.memories_forgotten, 1);
        assert!(memories.is_empty());
    }

    #[test]
    fn test_apply_forgetting_permanent_memories_skipped() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = Memory::semantic("Important".to_string(), "facts".to_string());
        memory.metadata.permanent = true;
        memory.metadata.strength = 0.001; // Very weak but permanent
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(1000);

        let mut memories = vec![memory];
        let result = engine.apply_forgetting(&mut memories).unwrap();

        // Permanent memory should not be processed
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_forgotten, 0);
        assert_eq!(memories.len(), 1);
    }

    #[test]
    fn test_apply_forgetting_multiple_memories_mixed_results() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        // Fresh memory
        let fresh = create_working_memory("Fresh");

        // Old weak memory (will be forgotten)
        let mut old = Memory::sensory("Old".to_string(), None);
        old.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(365);
        old.metadata.strength = 0.01;

        // Permanent memory (won't be processed)
        let mut permanent = Memory::semantic("Permanent".to_string(), "facts".to_string());
        permanent.metadata.permanent = true;
        permanent.metadata.strength = 0.5;

        let mut memories = vec![fresh, old, permanent];
        let result = engine.apply_forgetting(&mut memories).unwrap();

        assert_eq!(result.memories_processed, 2); // fresh + old (not permanent)
        assert_eq!(result.memories_forgotten, 1); // only old
        assert_eq!(memories.len(), 2); // fresh + permanent remain
    }

    #[test]
    fn test_apply_forgetting_processing_time_recorded() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories = vec![create_working_memory("Test")];

        let result = engine.apply_forgetting(&mut memories).unwrap();

        // Should have recorded some processing time (may be 0 for fast operations)
        let _ = result.processing_time_ms; // Just verify it exists
    }

    // --- update_memory_strength Tests ---
    #[test]
    fn test_update_strength_working_memory_decay() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");

        // Age the memory
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(7);
        let initial = memory.metadata.strength;

        engine.update_memory_strength(&mut memory).unwrap();

        assert!(memory.metadata.strength < initial);
        assert!(memory.metadata.strength > 0.0);
    }

    #[test]
    fn test_update_strength_importance_modifier_high() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut low_importance = create_working_memory("Low");
        low_importance.metadata.importance = 0.1;
        low_importance.metadata.created_at =
            time::OffsetDateTime::now_utc() - time::Duration::days(5);

        let mut high_importance = create_working_memory("High");
        high_importance.metadata.importance = 0.9;
        high_importance.metadata.created_at =
            time::OffsetDateTime::now_utc() - time::Duration::days(5);

        engine.update_memory_strength(&mut low_importance).unwrap();
        engine.update_memory_strength(&mut high_importance).unwrap();

        // High importance should retain more strength
        assert!(high_importance.metadata.strength > low_importance.metadata.strength);
    }

    #[test]
    fn test_update_strength_access_frequency_modifier() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut never_accessed = Memory::episodic("Never".to_string(), vec![], None);
        never_accessed.metadata.access_count = 0;
        never_accessed.metadata.created_at =
            time::OffsetDateTime::now_utc() - time::Duration::days(10);

        let mut frequently_accessed = Memory::episodic("Frequent".to_string(), vec![], None);
        frequently_accessed.metadata.access_count = 100;
        frequently_accessed.metadata.created_at =
            time::OffsetDateTime::now_utc() - time::Duration::days(10);

        engine.update_memory_strength(&mut never_accessed).unwrap();
        engine
            .update_memory_strength(&mut frequently_accessed)
            .unwrap();

        // Frequently accessed should be stronger
        assert!(frequently_accessed.metadata.strength > never_accessed.metadata.strength);
    }

    #[test]
    fn test_update_strength_spaced_repetition_disabled() {
        let config = ForgettingConfig {
            spaced_repetition: false,
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config);

        let mut memory = Memory::episodic("Test".to_string(), vec![], None);
        memory.metadata.access_count = 10;
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(5);

        let config_enabled = ForgettingConfig {
            spaced_repetition: true,
            ..Default::default()
        };
        let engine_enabled = ForgettingEngine::new(config_enabled);

        let mut memory_with_sr = memory.clone();

        engine.update_memory_strength(&mut memory).unwrap();
        engine_enabled
            .update_memory_strength(&mut memory_with_sr)
            .unwrap();

        // With spaced repetition enabled, memory should be stronger
        assert!(memory_with_sr.metadata.strength >= memory.metadata.strength);
    }

    #[test]
    fn test_update_strength_clamps_to_valid_range() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");

        // Set extreme values
        memory.metadata.importance = 10.0; // Unrealistically high
        memory.metadata.access_count = 10000;
        memory.metadata.created_at = time::OffsetDateTime::now_utc(); // Very fresh

        engine.update_memory_strength(&mut memory).unwrap();

        // Should be clamped to [0, 1]
        assert!(memory.metadata.strength >= 0.0);
        assert!(memory.metadata.strength <= 1.0);
    }

    #[test]
    fn test_update_strength_uses_default_curve() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");
        memory.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(30);

        // This should work with the default Working curve
        engine.update_memory_strength(&mut memory).unwrap();
        assert!(memory.metadata.strength >= 0.0);
    }

    // --- should_forget Tests ---
    #[test]
    fn test_should_forget_permanent_always_false() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");
        memory.metadata.permanent = true;
        memory.metadata.strength = 0.0; // Zero strength

        assert!(!engine.should_forget(&memory).unwrap());
    }

    #[test]
    fn test_should_forget_immune_type_always_false() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = Memory::semantic("Knowledge".to_string(), "facts".to_string());
        memory.metadata.strength = 0.0; // Zero strength

        // Semantic is immune
        assert!(!engine.should_forget(&memory).unwrap());
    }

    #[test]
    fn test_should_forget_below_threshold() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");
        memory.metadata.strength = 0.01; // Below default 0.2 threshold for working

        assert!(engine.should_forget(&memory).unwrap());
    }

    #[test]
    fn test_should_forget_above_threshold() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memory = create_working_memory("Test");
        memory.metadata.strength = 0.9; // Well above threshold

        assert!(!engine.should_forget(&memory).unwrap());
    }

    // --- calculate_adaptive_half_life Tests ---
    #[test]
    fn test_adaptive_half_life_base_value() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let memory = Memory::episodic("Test".to_string(), vec![], None);

        let half_life = engine.calculate_adaptive_half_life(&memory);

        // Episodic base is 14 days, result should be positive
        assert!(half_life > 0.0);
    }

    #[test]
    fn test_adaptive_half_life_increases_with_access() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut low_access = Memory::episodic("Low".to_string(), vec![], None);
        low_access.metadata.access_count = 1;

        let mut high_access = Memory::episodic("High".to_string(), vec![], None);
        high_access.metadata.access_count = 100;

        let low_half_life = engine.calculate_adaptive_half_life(&low_access);
        let high_half_life = engine.calculate_adaptive_half_life(&high_access);

        assert!(high_half_life > low_half_life);
    }

    #[test]
    fn test_adaptive_half_life_increases_with_importance() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut low_importance = create_working_memory("Low");
        low_importance.metadata.importance = 0.1;

        let mut high_importance = create_working_memory("High");
        high_importance.metadata.importance = 0.9;

        let low_half_life = engine.calculate_adaptive_half_life(&low_importance);
        let high_half_life = engine.calculate_adaptive_half_life(&high_importance);

        assert!(high_half_life > low_half_life);
    }

    #[test]
    fn test_adaptive_half_life_different_memory_types() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let sensory = Memory::sensory("Sense".to_string(), None);
        let semantic = Memory::semantic("Knowledge".to_string(), "facts".to_string());

        let sensory_half_life = engine.calculate_adaptive_half_life(&sensory);
        let semantic_half_life = engine.calculate_adaptive_half_life(&semantic);

        // Semantic should have much longer half-life (180 days vs 0.25 days)
        assert!(semantic_half_life > sensory_half_life * 10.0);
    }

    // --- get_type_statistics Tests ---
    #[test]
    fn test_type_statistics_empty_memories() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let memories: Vec<Memory> = vec![];

        let stats = engine.get_type_statistics(&MemoryType::Working, &memories);

        assert_eq!(stats.total_memories, 0);
        assert!((stats.average_strength - 0.0).abs() < 0.001);
        assert_eq!(stats.weak_memories, 0);
    }

    #[test]
    fn test_type_statistics_no_matching_type() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let memories = vec![
            create_working_memory("Work1"),
            create_working_memory("Work2"),
        ];

        let stats = engine.get_type_statistics(&MemoryType::Sensory, &memories);

        assert_eq!(stats.total_memories, 0);
    }

    #[test]
    fn test_type_statistics_correct_counts() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let memories = vec![
            create_working_memory("Work1"),
            create_working_memory("Work2"),
            create_working_memory("Work3"),
            Memory::sensory("Sense1".to_string(), None),
        ];

        let stats = engine.get_type_statistics(&MemoryType::Working, &memories);

        assert_eq!(stats.total_memories, 3);
        assert_eq!(stats.memory_type, MemoryType::Working);
    }

    #[test]
    fn test_type_statistics_average_strength() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut m1 = create_working_memory("W1");
        m1.metadata.strength = 0.8;
        let mut m2 = create_working_memory("W2");
        m2.metadata.strength = 0.6;

        let memories = vec![m1, m2];
        let stats = engine.get_type_statistics(&MemoryType::Working, &memories);

        // Average should be (0.8 + 0.6) / 2 = 0.7
        assert!((stats.average_strength - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_type_statistics_weak_memories_count() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut strong = create_working_memory("Strong");
        strong.metadata.strength = 0.9;

        let mut weak = create_working_memory("Weak");
        weak.metadata.strength = 0.05; // Below working threshold (0.2)

        let mut medium = create_working_memory("Medium");
        medium.metadata.strength = 0.3;

        let memories = vec![strong, weak, medium];
        let stats = engine.get_type_statistics(&MemoryType::Working, &memories);

        assert_eq!(stats.weak_memories, 1); // Only 'weak' is below threshold
    }

    #[test]
    fn test_type_statistics_average_age() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut m1 = create_working_memory("W1");
        m1.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(10);

        let mut m2 = create_working_memory("W2");
        m2.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(20);

        let memories = vec![m1, m2];
        let stats = engine.get_type_statistics(&MemoryType::Working, &memories);

        // Average age should be around 15 days
        assert!(stats.average_age_days > 14.0);
        assert!(stats.average_age_days < 16.0);
    }

    // --- ForgettingResult Tests ---
    #[test]
    fn test_forgetting_result_default() {
        let result = ForgettingResult::default();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_forgotten, 0);
        assert!((result.total_strength_lost - 0.0).abs() < 0.001);
        assert_eq!(result.processing_time_ms, 0);
    }

    // --- TypeForgettingStats Tests ---
    #[test]
    fn test_type_forgetting_stats_default() {
        let stats = TypeForgettingStats::default();
        assert_eq!(stats.total_memories, 0);
        assert!((stats.average_strength - 0.0).abs() < 0.001);
        assert_eq!(stats.weak_memories, 0);
        assert!((stats.average_age_days - 0.0).abs() < 0.001);
    }

    // --- ForgettingCurve properties ---
    #[test]
    fn test_forgetting_curve_sensory_fast_decay() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let sensory = engine.forgetting_curves.get(&MemoryType::Sensory).unwrap();
        let episodic = engine.forgetting_curves.get(&MemoryType::Episodic).unwrap();

        // Sensory should decay much faster than episodic
        assert!(sensory.half_life < episodic.half_life);
        assert!(sensory.decay_rate > episodic.decay_rate);
    }

    #[test]
    fn test_forgetting_curve_procedural_month_half_life() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let procedural = engine
            .forgetting_curves
            .get(&MemoryType::Procedural)
            .unwrap();

        assert!((procedural.half_life - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_forgetting_curve_social_three_weeks() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let social = engine.forgetting_curves.get(&MemoryType::Social).unwrap();

        assert!((social.half_life - 21.0).abs() < 0.1);
    }

    // --- Edge cases ---
    #[test]
    fn test_apply_forgetting_strength_loss_accumulated() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        let mut m1 = create_working_memory("W1");
        m1.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(30);

        let mut m2 = create_working_memory("W2");
        m2.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(30);

        let initial_strength = m1.metadata.strength + m2.metadata.strength;
        let mut memories = vec![m1, m2];

        let result = engine.apply_forgetting(&mut memories).unwrap();

        let final_strength: f32 = memories.iter().map(|m| m.metadata.strength).sum();
        let actual_loss = initial_strength - final_strength;

        // Loss should be positive (memories decayed)
        assert!(result.total_strength_lost > 0.0);
        assert!((result.total_strength_lost - actual_loss).abs() < 0.01);
    }

    #[test]
    fn test_apply_forgetting_removal_order_correct() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());

        // Create 3 memories, mark first and third for removal
        let mut m1 = Memory::sensory("First".to_string(), None);
        m1.metadata.strength = 0.001;
        m1.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(100);

        let m2 = Memory::sensory("Second".to_string(), None); // Fresh, will stay

        let mut m3 = Memory::sensory("Third".to_string(), None);
        m3.metadata.strength = 0.001;
        m3.metadata.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(100);

        let mut memories = vec![m1, m2, m3];
        engine.apply_forgetting(&mut memories).unwrap();

        // Only the fresh memory should remain
        assert_eq!(memories.len(), 1);
        assert!(memories[0].content.text.contains("Second"));
    }
}
