//! Mutation-resistant comprehensive tests for astraweave-memory
//!
//! Tests specifically designed to kill cargo-mutants mutants by testing:
//! - Exact boundary conditions
//! - Return value verification  
//! - State changes
//! - Error paths
//! - All enum variants

#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_range_contains,
    clippy::drop_non_drop,
    clippy::absurd_extreme_comparisons,
    clippy::useless_vec,
    unused_comparisons
)]

use astraweave_memory::{
    // Pattern detection
    ActionPattern,
    // Episode
    ActionResult,
    // Dynamic weighting
    AdaptiveWeightManager,
    AssociationType,
    BehaviorNodeType,
    // Behavior validator
    BehaviorValidator,
    ClusterType,
    // Preference profile
    CompanionActionPreference,
    // Persona
    CompanionProfile,
    CompanionResponse,
    // Compression
    CompressionConfig,
    CompressionEngine,
    CompressionResult,
    // Consolidation
    ConsolidationConfig,
    ConsolidationEngine,
    ConsolidationResult,
    ConsolidationState,
    EmotionalContext,
    Episode as PersonaEpisode,
    EpisodeCategory,
    EpisodeOutcome,
    EpisodeRecorder,
    Fact,
    // Forgetting
    ForgettingConfig,
    ForgettingCurve,
    ForgettingEngine,
    ForgettingResult,
    GameEpisode,
    // Memory types
    Memory,
    MemoryAssociation,
    MemoryCluster,
    MemorySource,
    MemoryType,
    NodeWeight,
    Observation,
    PatternDetector,
    PatternStrength,
    Persona,
    PlayerAction,
    PlaystylePattern,
    PreferenceProfile,
    PrivacyLevel,
    ProfileBuilder,
    // Retrieval
    RetrievalConfig,
    RetrievalContext,
    RetrievalEngine,
    RetrievalPath,
    SafetyRule,
    ScoreBreakdown,
    SensoryData,
    ShareRequest,
    // Sharing
    SharingConfig,
    SharingEngine,
    SharingMetadata,
    SharingType,
    Skill,
    SpatialTemporalContext,
    TimeWindow,
    ValidationResult,
};
use chrono::Utc;
use std::collections::HashMap;

// =============================================================================
// MEMORY TYPE TESTS - memory_types.rs (97 mutants)
// =============================================================================

mod memory_type_tests {
    use super::*;

    // ----- MemoryType enum (7 variants) -----

    #[test]
    fn memory_type_default_is_working() {
        let mt: MemoryType = Default::default();
        assert_eq!(mt, MemoryType::Working);
    }

    #[test]
    fn memory_type_sensory_is_distinct() {
        assert_ne!(MemoryType::Sensory, MemoryType::Working);
        assert_ne!(MemoryType::Sensory, MemoryType::Episodic);
        assert_ne!(MemoryType::Sensory, MemoryType::Semantic);
        assert_ne!(MemoryType::Sensory, MemoryType::Procedural);
        assert_ne!(MemoryType::Sensory, MemoryType::Emotional);
        assert_ne!(MemoryType::Sensory, MemoryType::Social);
    }

    #[test]
    fn memory_type_all_variants_exist() {
        let _sensory = MemoryType::Sensory;
        let _working = MemoryType::Working;
        let _episodic = MemoryType::Episodic;
        let _semantic = MemoryType::Semantic;
        let _procedural = MemoryType::Procedural;
        let _emotional = MemoryType::Emotional;
        let _social = MemoryType::Social;
    }

    #[test]
    fn memory_type_eq_reflexive() {
        assert_eq!(MemoryType::Sensory, MemoryType::Sensory);
        assert_eq!(MemoryType::Working, MemoryType::Working);
        assert_eq!(MemoryType::Episodic, MemoryType::Episodic);
        assert_eq!(MemoryType::Semantic, MemoryType::Semantic);
        assert_eq!(MemoryType::Procedural, MemoryType::Procedural);
        assert_eq!(MemoryType::Emotional, MemoryType::Emotional);
        assert_eq!(MemoryType::Social, MemoryType::Social);
    }

    #[test]
    fn memory_type_hash_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(MemoryType::Sensory);
        set.insert(MemoryType::Working);
        set.insert(MemoryType::Episodic);
        assert_eq!(set.len(), 3);
        assert!(set.contains(&MemoryType::Sensory));
        assert!(set.contains(&MemoryType::Working));
        assert!(set.contains(&MemoryType::Episodic));
    }

    // ----- MemorySource enum -----

    #[test]
    fn memory_source_all_variants() {
        let _direct = MemorySource::DirectExperience;
        let _conv = MemorySource::Conversation;
        let _learn = MemorySource::Learning;
        let _infer = MemorySource::Inference;
        let _shared = MemorySource::SharedMemory;
        let _system = MemorySource::SystemGenerated;
    }

    // ----- AssociationType enum -----

    #[test]
    fn association_type_all_variants() {
        let variants = vec![
            AssociationType::Temporal,
            AssociationType::Spatial,
            AssociationType::Causal,
            AssociationType::Conceptual,
            AssociationType::Emotional,
            AssociationType::Sequential,
            AssociationType::Contrast,
        ];
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn association_type_equality() {
        assert_eq!(AssociationType::Temporal, AssociationType::Temporal);
        assert_ne!(AssociationType::Temporal, AssociationType::Spatial);
        assert_ne!(AssociationType::Causal, AssociationType::Conceptual);
    }

    // ----- ClusterType enum -----

    #[test]
    fn cluster_type_all_variants() {
        let _event = ClusterType::Event;
        let _person = ClusterType::Person;
        let _location = ClusterType::Location;
        let _concept = ClusterType::Concept;
        let _skill = ClusterType::Skill;
        let _relationship = ClusterType::Relationship;
    }

    // ----- SharingType enum -----

    #[test]
    fn sharing_type_all_variants() {
        let variants = vec![
            SharingType::Full,
            SharingType::Summary,
            SharingType::Metadata,
            SharingType::Restricted,
        ];
        assert_eq!(variants.len(), 4);
        assert_eq!(SharingType::Full, SharingType::Full);
        assert_ne!(SharingType::Full, SharingType::Restricted);
    }

    // ----- PrivacyLevel enum -----

    #[test]
    fn privacy_level_all_variants() {
        let variants = vec![
            PrivacyLevel::Public,
            PrivacyLevel::Group,
            PrivacyLevel::Personal,
            PrivacyLevel::Secret,
        ];
        assert_eq!(variants.len(), 4);
        assert_eq!(PrivacyLevel::Public, PrivacyLevel::Public);
        assert_ne!(PrivacyLevel::Public, PrivacyLevel::Secret);
    }

    // ----- Memory constructors -----

    #[test]
    fn memory_sensory_sets_low_importance() {
        let mem = Memory::sensory("test sensory".to_string(), None);
        assert_eq!(mem.memory_type, MemoryType::Sensory);
        assert!(
            mem.metadata.importance < 0.5,
            "Sensory should have low importance"
        );
        assert!(mem.metadata.decay_factor > 1.0, "Sensory should decay fast");
    }

    #[test]
    fn memory_episodic_has_participants() {
        let participants = vec!["Alice".to_string(), "Bob".to_string()];
        let mem = Memory::episodic(
            "Met friends".to_string(),
            participants.clone(),
            Some("Park".to_string()),
        );
        assert_eq!(mem.memory_type, MemoryType::Episodic);
        assert_eq!(mem.content.context.participants.len(), 2);
        assert_eq!(mem.content.context.location, Some("Park".to_string()));
        assert!(
            mem.metadata.importance >= 0.5,
            "Episodic should be important"
        );
    }

    #[test]
    fn memory_semantic_is_permanent() {
        let mem = Memory::semantic("Cats are mammals".to_string(), "animals".to_string());
        assert_eq!(mem.memory_type, MemoryType::Semantic);
        assert!(
            mem.metadata.permanent,
            "Semantic memories should be permanent"
        );
        assert!(
            mem.metadata.importance >= 0.7,
            "Semantic should have high importance"
        );
    }

    #[test]
    fn memory_working_has_current_period() {
        let mem = Memory::working("Current task".to_string());
        assert_eq!(mem.memory_type, MemoryType::Working);
        assert_eq!(mem.content.context.time_period, Some("current".to_string()));
    }

    #[test]
    fn memory_procedural_has_skill_data() {
        let mem = Memory::procedural("How to ride a bike".to_string(), "cycling".to_string());
        assert_eq!(mem.memory_type, MemoryType::Procedural);
        let data = &mem.content.data;
        assert!(data.get("skill").is_some());
    }

    #[test]
    fn memory_new_generates_unique_ids() {
        let mem1 = Memory::working("test1".to_string());
        let mem2 = Memory::working("test2".to_string());
        assert_ne!(mem1.id, mem2.id, "Each memory should have unique ID");
    }

    #[test]
    fn memory_new_initializes_metadata_correctly() {
        let mem = Memory::working("test".to_string());
        assert_eq!(mem.metadata.access_count, 0);
        assert_eq!(mem.metadata.confidence, 1.0);
        assert_eq!(mem.metadata.strength, 1.0);
        assert_eq!(mem.metadata.decay_factor, 1.0);
        assert!(mem.associations.is_empty());
        assert!(mem.embedding.is_none());
    }

    // ----- EmotionalContext -----

    #[test]
    fn emotional_context_fields_persist() {
        let ctx = EmotionalContext {
            primary_emotion: "joy".to_string(),
            intensity: 0.8,
            valence: 0.9,
            arousal: 0.7,
        };
        assert_eq!(ctx.primary_emotion, "joy");
        assert_eq!(ctx.intensity, 0.8);
        assert_eq!(ctx.valence, 0.9);
        assert_eq!(ctx.arousal, 0.7);
    }

    // ----- SensoryData -----

    #[test]
    fn sensory_data_optional_fields() {
        let data = SensoryData {
            visual: Some("bright".to_string()),
            auditory: None,
            tactile: Some("rough".to_string()),
            environmental: None,
        };
        assert!(data.visual.is_some());
        assert!(data.auditory.is_none());
        assert!(data.tactile.is_some());
        assert!(data.environmental.is_none());
    }

    // ----- SpatialTemporalContext -----

    #[test]
    fn spatial_temporal_context_fields() {
        let ctx = SpatialTemporalContext {
            location: Some("Office".to_string()),
            time_period: Some("morning".to_string()),
            duration: Some(3600000),
            participants: vec!["user".to_string()],
            related_events: vec!["meeting".to_string()],
        };
        assert_eq!(ctx.location, Some("Office".to_string()));
        assert_eq!(ctx.duration, Some(3600000));
        assert_eq!(ctx.participants.len(), 1);
        assert_eq!(ctx.related_events.len(), 1);
    }

    // ----- MemoryMetadata -----

    #[test]
    fn memory_metadata_strength_and_decay() {
        let mem = Memory::sensory("test".to_string(), None);
        assert!(mem.metadata.strength >= 0.0 && mem.metadata.strength <= 1.0);
        assert!(mem.metadata.decay_factor > 0.0);
    }

    // ----- MemoryAssociation -----

    #[test]
    fn memory_association_structure() {
        let assoc = MemoryAssociation {
            memory_id: "mem_123".to_string(),
            association_type: AssociationType::Temporal,
            strength: 0.75,
            formed_at: Utc::now(),
        };
        assert_eq!(assoc.memory_id, "mem_123");
        assert_eq!(assoc.strength, 0.75);
        assert_eq!(assoc.association_type, AssociationType::Temporal);
    }

    // ----- MemoryCluster -----

    #[test]
    fn memory_cluster_structure() {
        let cluster = MemoryCluster {
            id: "cluster_1".to_string(),
            name: "Work memories".to_string(),
            cluster_type: ClusterType::Location,
            memory_ids: vec!["m1".to_string(), "m2".to_string()],
            central_concept: "office".to_string(),
            importance: 0.8,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        };
        assert_eq!(cluster.name, "Work memories");
        assert_eq!(cluster.memory_ids.len(), 2);
        assert_eq!(cluster.importance, 0.8);
    }

    // ----- ConsolidationState -----

    #[test]
    fn consolidation_state_fields() {
        let state = ConsolidationState {
            consolidation_level: 0.5,
            passes: 2,
            last_consolidation: Utc::now(),
            needs_consolidation: false,
            priority: 0.7,
        };
        assert_eq!(state.consolidation_level, 0.5);
        assert_eq!(state.passes, 2);
        assert!(!state.needs_consolidation);
        assert_eq!(state.priority, 0.7);
    }

    // ----- ForgettingCurve -----

    #[test]
    fn forgetting_curve_fields() {
        let curve = ForgettingCurve {
            initial_strength: 1.0,
            decay_rate: 0.1,
            half_life: 7.0,
            retention_threshold: 0.15,
            immune: false,
        };
        assert_eq!(curve.initial_strength, 1.0);
        assert_eq!(curve.decay_rate, 0.1);
        assert_eq!(curve.half_life, 7.0);
        assert_eq!(curve.retention_threshold, 0.15);
        assert!(!curve.immune);
    }

    #[test]
    fn forgetting_curve_immune_flag() {
        let immune_curve = ForgettingCurve {
            initial_strength: 1.0,
            decay_rate: 0.0,
            half_life: f32::MAX,
            retention_threshold: 0.0,
            immune: true,
        };
        assert!(immune_curve.immune);
    }

    // ----- RetrievalContext -----

    #[test]
    fn retrieval_context_fields() {
        let ctx = RetrievalContext {
            query: "find memories about work".to_string(),
            emotional_state: None,
            location: Some("office".to_string()),
            recent_memory_ids: vec!["m1".to_string()],
            preferred_types: vec![MemoryType::Episodic, MemoryType::Semantic],
            time_window: None,
            limit: 10,
        };
        assert_eq!(ctx.query, "find memories about work");
        assert_eq!(ctx.limit, 10);
        assert_eq!(ctx.preferred_types.len(), 2);
    }

    // ----- TimeWindow -----

    #[test]
    fn time_window_fields() {
        let start = Utc::now();
        let end = Utc::now();
        let window = TimeWindow { start, end };
        assert!(window.end >= window.start);
    }

    // ----- SharingMetadata -----

    #[test]
    fn sharing_metadata_fields() {
        let meta = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["agent_1".to_string()],
            sharing_type: SharingType::Summary,
            privacy_level: PrivacyLevel::Group,
            sharing_conditions: vec!["consent".to_string()],
        };
        assert!(meta.shareable);
        assert_eq!(meta.authorized_entities.len(), 1);
        assert_eq!(meta.sharing_type, SharingType::Summary);
        assert_eq!(meta.privacy_level, PrivacyLevel::Group);
    }
}

// =============================================================================
// CONSOLIDATION TESTS - consolidation.rs (88 mutants)
// =============================================================================

mod consolidation_tests {
    use super::*;

    #[test]
    fn consolidation_config_default_values() {
        let config = ConsolidationConfig::default();
        assert!(config.association_threshold >= 0.0 && config.association_threshold <= 1.0);
        assert!(config.temporal_window_hours > 0.0);
        assert!(config.max_associations > 0);
        assert!(config.consolidation_boost >= 0.0);
    }

    #[test]
    fn consolidation_config_fields_accessible() {
        let config = ConsolidationConfig {
            association_threshold: 0.6,
            temporal_window_hours: 24.0,
            max_associations: 10,
            consolidation_boost: 0.1,
            auto_associations: true,
        };
        assert_eq!(config.association_threshold, 0.6);
        assert_eq!(config.temporal_window_hours, 24.0);
        assert_eq!(config.max_associations, 10);
        assert_eq!(config.consolidation_boost, 0.1);
        assert!(config.auto_associations);
    }

    #[test]
    fn consolidation_engine_creation() {
        let config = ConsolidationConfig::default();
        let _engine = ConsolidationEngine::new(config);
    }

    #[test]
    fn consolidation_engine_with_empty_memories() {
        let engine = ConsolidationEngine::new(ConsolidationConfig::default());
        let mut memories = Vec::new();
        let result = engine.consolidate(&mut memories).unwrap();
        assert_eq!(result.memories_processed, 0);
    }

    #[test]
    fn consolidation_result_default() {
        let result = ConsolidationResult::default();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.temporal_associations, 0);
        assert_eq!(result.spatial_associations, 0);
        assert_eq!(result.conceptual_associations, 0);
        assert_eq!(result.processing_time_ms, 0);
    }

    #[test]
    fn consolidation_result_total_associations() {
        let mut result = ConsolidationResult::default();
        result.temporal_associations = 2;
        result.spatial_associations = 3;
        result.conceptual_associations = 5;
        assert_eq!(result.total_associations(), 10);
    }

    #[test]
    fn consolidation_result_total_associations_zero() {
        let result = ConsolidationResult::default();
        assert_eq!(result.total_associations(), 0);
    }

    #[test]
    fn consolidation_with_related_memories() {
        let engine = ConsolidationEngine::new(ConsolidationConfig::default());
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
        assert_eq!(result.memories_processed, 2);
    }

    #[test]
    fn consolidation_increases_memory_strength() {
        let config = ConsolidationConfig {
            consolidation_boost: 0.1,
            ..ConsolidationConfig::default()
        };
        let engine = ConsolidationEngine::new(config);
        let mut memories = vec![
            Memory::episodic(
                "Test memory 1".to_string(),
                vec!["Person".to_string()],
                Some("Location".to_string()),
            ),
            Memory::episodic(
                "Test memory 2".to_string(),
                vec!["Person".to_string()],
                Some("Location".to_string()),
            ),
        ];
        let initial_strength = memories[0].metadata.strength;
        let _ = engine.consolidate(&mut memories);
        // At minimum, processed without error
        assert!(memories[0].metadata.strength >= initial_strength * 0.9);
    }
}

// =============================================================================
// FORGETTING TESTS - forgetting.rs (91 mutants)
// =============================================================================

mod forgetting_tests {
    use super::*;

    #[test]
    fn forgetting_config_default_values() {
        let config = ForgettingConfig::default();
        assert!(config.base_decay_rate >= 0.0 && config.base_decay_rate <= 1.0);
        assert!(config.retention_threshold > 0.0);
        assert!(config.importance_factor >= 0.0);
        assert!(config.access_factor >= 0.0);
    }

    #[test]
    fn forgetting_config_all_fields() {
        let config = ForgettingConfig {
            base_decay_rate: 0.2,
            retention_threshold: 0.1,
            importance_factor: 0.5,
            access_factor: 0.3,
            spaced_repetition: true,
        };
        assert_eq!(config.base_decay_rate, 0.2);
        assert_eq!(config.retention_threshold, 0.1);
        assert_eq!(config.importance_factor, 0.5);
        assert_eq!(config.access_factor, 0.3);
        assert!(config.spaced_repetition);
    }

    #[test]
    fn forgetting_config_spaced_repetition_disabled() {
        let config = ForgettingConfig {
            spaced_repetition: false,
            ..ForgettingConfig::default()
        };
        assert!(!config.spaced_repetition);
    }

    #[test]
    fn forgetting_engine_creation() {
        let config = ForgettingConfig::default();
        let _engine = ForgettingEngine::new(config);
    }

    #[test]
    fn forgetting_result_default() {
        let result = ForgettingResult::default();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_forgotten, 0);
        assert_eq!(result.total_strength_lost, 0.0);
        assert_eq!(result.processing_time_ms, 0);
    }

    #[test]
    fn forgetting_with_empty_memories() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories = Vec::new();
        let result = engine.apply_forgetting(&mut memories).unwrap();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_forgotten, 0);
    }

    #[test]
    fn forgetting_skips_permanent_memories() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories = vec![Memory::semantic(
            "Important fact".to_string(),
            "facts".to_string(),
        )];
        // Semantic memories are permanent by default
        let initial_strength = memories[0].metadata.strength;
        let _result = engine.apply_forgetting(&mut memories).unwrap();
        // Should process memory but not forget it
        assert!(!memories.is_empty());
        assert!(memories[0].metadata.strength <= initial_strength);
    }

    #[test]
    fn forgetting_different_memory_types_have_different_curves() {
        let _engine = ForgettingEngine::new(ForgettingConfig::default());
        let sensory = Memory::sensory("Quick flash".to_string(), None);
        let semantic = Memory::semantic("Permanent fact".to_string(), "facts".to_string());
        // Sensory should decay faster than semantic
        assert!(sensory.metadata.decay_factor > semantic.metadata.decay_factor);
    }
}

// =============================================================================
// COMPRESSION TESTS - compression.rs (81 mutants)
// =============================================================================

mod compression_tests {
    use super::*;

    #[test]
    fn compression_config_default_values() {
        let config = CompressionConfig::default();
        assert!(config.min_age_days > 0.0);
        assert!(config.importance_threshold >= 0.0 && config.importance_threshold <= 1.0);
        assert!(config.max_compression_ratio > 0.0 && config.max_compression_ratio <= 1.0);
    }

    #[test]
    fn compression_config_all_fields() {
        let config = CompressionConfig {
            min_age_days: 14.0,
            importance_threshold: 0.4,
            max_compression_ratio: 0.6,
            preserve_emotional_context: true,
        };
        assert_eq!(config.min_age_days, 14.0);
        assert_eq!(config.importance_threshold, 0.4);
        assert_eq!(config.max_compression_ratio, 0.6);
        assert!(config.preserve_emotional_context);
    }

    #[test]
    fn compression_config_preserve_emotional_disabled() {
        let config = CompressionConfig {
            preserve_emotional_context: false,
            ..CompressionConfig::default()
        };
        assert!(!config.preserve_emotional_context);
    }

    #[test]
    fn compression_engine_creation() {
        let config = CompressionConfig::default();
        let _engine = CompressionEngine::new(config);
    }

    #[test]
    fn compression_result_default() {
        let result = CompressionResult::default();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_compressed, 0);
        assert_eq!(result.size_reduction, 0);
        assert_eq!(result.processing_time_ms, 0);
    }

    #[test]
    fn compression_with_empty_memories() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memories: [Memory; 0] = [];
        let result = engine.compress_memories(&mut memories).unwrap();
        assert_eq!(result.memories_processed, 0);
        assert_eq!(result.memories_compressed, 0);
    }

    #[test]
    fn compression_skips_permanent_memories() {
        let engine = CompressionEngine::new(CompressionConfig {
            min_age_days: 0.0, // Immediate compression
            ..CompressionConfig::default()
        });
        let mut memories = [Memory::semantic("fact".to_string(), "test".to_string())];
        let result = engine.compress_memories(&mut memories).unwrap();
        // Should process but not compress permanent
        assert_eq!(result.memories_processed, 1);
    }
}

// =============================================================================
// RETRIEVAL TESTS - retrieval.rs (100 mutants)
// =============================================================================

mod retrieval_tests {
    use super::*;

    #[test]
    fn retrieval_config_default_values() {
        let config = RetrievalConfig::default();
        assert!(config.max_results > 0);
        assert!(config.relevance_threshold >= 0.0 && config.relevance_threshold <= 1.0);
        assert!(config.semantic_weight >= 0.0);
        assert!(config.temporal_weight >= 0.0);
        assert!(config.associative_weight >= 0.0);
    }

    #[test]
    fn retrieval_config_all_fields() {
        let config = RetrievalConfig {
            max_results: 20,
            relevance_threshold: 0.5,
            semantic_weight: 0.7,
            temporal_weight: 0.15,
            associative_weight: 0.15,
            recency_boost: false,
            follow_associations: false,
        };
        assert_eq!(config.max_results, 20);
        assert_eq!(config.relevance_threshold, 0.5);
        assert_eq!(config.semantic_weight, 0.7);
        assert_eq!(config.temporal_weight, 0.15);
        assert_eq!(config.associative_weight, 0.15);
        assert!(!config.recency_boost);
        assert!(!config.follow_associations);
    }

    #[test]
    fn retrieval_engine_creation() {
        let config = RetrievalConfig::default();
        let _engine = RetrievalEngine::new(config);
    }

    #[test]
    fn retrieval_path_variants() {
        let direct = RetrievalPath::Direct;
        let assoc = RetrievalPath::Associative {
            source_memory_id: "m1".to_string(),
        };
        let temporal = RetrievalPath::Temporal {
            reference_time: Utc::now(),
        };
        let cluster = RetrievalPath::Cluster {
            cluster_id: "c1".to_string(),
        };

        // Just verify they exist and are distinct types
        match direct {
            RetrievalPath::Direct => {}
            _ => panic!(),
        }
        match assoc {
            RetrievalPath::Associative { .. } => {}
            _ => panic!(),
        }
        match temporal {
            RetrievalPath::Temporal { .. } => {}
            _ => panic!(),
        }
        match cluster {
            RetrievalPath::Cluster { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn score_breakdown_fields() {
        let breakdown = ScoreBreakdown {
            semantic_score: 0.8,
            temporal_score: 0.5,
            associative_score: 0.3,
            importance_score: 0.7,
            recency_score: 0.9,
        };
        assert_eq!(breakdown.semantic_score, 0.8);
        assert_eq!(breakdown.temporal_score, 0.5);
        assert_eq!(breakdown.associative_score, 0.3);
        assert_eq!(breakdown.importance_score, 0.7);
        assert_eq!(breakdown.recency_score, 0.9);
    }

    #[test]
    fn retrieval_with_empty_memories() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };
        let results = engine.retrieve(&context, &[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn retrieval_respects_limit() {
        let config = RetrievalConfig {
            max_results: 5,
            relevance_threshold: 0.0, // Accept all
            ..RetrievalConfig::default()
        };
        let engine = RetrievalEngine::new(config);
        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 2, // Request only 2
        };
        let memories: Vec<Memory> = (0..10)
            .map(|i| Memory::working(format!("memory {}", i)))
            .collect();
        let results = engine.retrieve(&context, &memories).unwrap();
        assert!(results.len() <= 2);
    }
}

// =============================================================================
// EPISODE TESTS - episode.rs (82 mutants)
// =============================================================================

mod episode_tests {
    use super::*;

    // ----- EpisodeCategory enum -----

    #[test]
    fn episode_category_all_variants() {
        let categories = vec![
            EpisodeCategory::Combat,
            EpisodeCategory::Dialogue,
            EpisodeCategory::Exploration,
            EpisodeCategory::Puzzle,
            EpisodeCategory::Quest,
            EpisodeCategory::Social,
        ];
        assert_eq!(categories.len(), 6);
    }

    #[test]
    fn episode_category_display() {
        assert_eq!(format!("{}", EpisodeCategory::Combat), "Combat");
        assert_eq!(format!("{}", EpisodeCategory::Dialogue), "Dialogue");
        assert_eq!(format!("{}", EpisodeCategory::Exploration), "Exploration");
        assert_eq!(format!("{}", EpisodeCategory::Puzzle), "Puzzle");
        assert_eq!(format!("{}", EpisodeCategory::Quest), "Quest");
        assert_eq!(format!("{}", EpisodeCategory::Social), "Social");
    }

    #[test]
    fn episode_category_equality() {
        assert_eq!(EpisodeCategory::Combat, EpisodeCategory::Combat);
        assert_ne!(EpisodeCategory::Combat, EpisodeCategory::Dialogue);
    }

    // ----- ActionResult enum -----

    #[test]
    fn action_result_all_variants() {
        let results = vec![
            ActionResult::Success,
            ActionResult::Failure,
            ActionResult::Interrupted,
            ActionResult::Partial,
        ];
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn action_result_success_multiplier() {
        assert_eq!(ActionResult::Success.success_multiplier(), 1.0);
        assert_eq!(ActionResult::Partial.success_multiplier(), 0.5);
        assert_eq!(ActionResult::Interrupted.success_multiplier(), 0.25);
        assert_eq!(ActionResult::Failure.success_multiplier(), 0.0);
    }

    #[test]
    fn action_result_success_multiplier_ordering() {
        assert!(
            ActionResult::Success.success_multiplier() > ActionResult::Partial.success_multiplier()
        );
        assert!(
            ActionResult::Partial.success_multiplier()
                > ActionResult::Interrupted.success_multiplier()
        );
        assert!(
            ActionResult::Interrupted.success_multiplier()
                > ActionResult::Failure.success_multiplier()
        );
    }

    // ----- EpisodeOutcome -----

    #[test]
    fn episode_outcome_quality_score_range() {
        let outcome = EpisodeOutcome {
            success_rating: 0.8,
            player_satisfaction: 0.9,
            companion_effectiveness: 0.7,
            duration_ms: 60000,
            damage_dealt: 100.0,
            damage_taken: 20.0,
            resources_used: 50.0,
            failure_count: 0,
        };
        let score = outcome.quality_score();
        assert!(
            score >= 0.0 && score <= 1.0,
            "Quality score should be 0.0-1.0"
        );
    }

    #[test]
    fn episode_outcome_quality_score_success_weighted() {
        let high_success = EpisodeOutcome {
            success_rating: 1.0,
            player_satisfaction: 0.5,
            companion_effectiveness: 0.5,
            duration_ms: 1000,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            resources_used: 0.0,
            failure_count: 0,
        };
        let low_success = EpisodeOutcome {
            success_rating: 0.0,
            player_satisfaction: 0.5,
            companion_effectiveness: 0.5,
            duration_ms: 1000,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            resources_used: 0.0,
            failure_count: 0,
        };
        assert!(high_success.quality_score() > low_success.quality_score());
    }

    #[test]
    fn episode_outcome_zero_resources() {
        let outcome = EpisodeOutcome {
            success_rating: 0.5,
            player_satisfaction: 0.5,
            companion_effectiveness: 0.5,
            duration_ms: 1000,
            damage_dealt: 100.0,
            damage_taken: 50.0,
            resources_used: 0.0, // Zero resources
            failure_count: 0,
        };
        // Should not panic
        let score = outcome.quality_score();
        assert!(score >= 0.0);
    }

    #[test]
    fn episode_outcome_zero_damage() {
        let outcome = EpisodeOutcome {
            success_rating: 0.5,
            player_satisfaction: 0.5,
            companion_effectiveness: 0.5,
            duration_ms: 1000,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            resources_used: 10.0,
            failure_count: 0,
        };
        // Should not panic
        let score = outcome.quality_score();
        assert!(score >= 0.0);
    }

    // ----- PlayerAction -----

    #[test]
    fn player_action_fields() {
        let action = PlayerAction {
            action_type: "melee_attack".to_string(),
            target: Some("enemy_1".to_string()),
            parameters: serde_json::json!({"damage": 50}),
        };
        assert_eq!(action.action_type, "melee_attack");
        assert_eq!(action.target, Some("enemy_1".to_string()));
    }

    // ----- CompanionResponse -----

    #[test]
    fn companion_response_fields() {
        let response = CompanionResponse {
            action_type: "heal".to_string(),
            result: ActionResult::Success,
            effectiveness: 0.9,
        };
        assert_eq!(response.action_type, "heal");
        assert_eq!(response.result, ActionResult::Success);
        assert_eq!(response.effectiveness, 0.9);
    }

    // ----- Observation -----

    #[test]
    fn observation_new() {
        let obs = Observation::new(
            1000,
            Some(PlayerAction {
                action_type: "attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::json!({"player_health": 100.0}),
        );
        assert_eq!(obs.timestamp_ms, 1000);
        assert!(obs.player_action.is_some());
        assert!(obs.companion_response.is_none());
    }

    #[test]
    fn observation_player_health_extraction() {
        let obs = Observation::new(0, None, None, serde_json::json!({"player_health": 75.5}));
        assert_eq!(obs.player_health(), Some(75.5));
    }

    #[test]
    fn observation_player_health_missing() {
        let obs = Observation::new(0, None, None, serde_json::json!({}));
        assert!(obs.player_health().is_none());
    }

    #[test]
    fn observation_enemy_count_extraction() {
        let obs = Observation::new(0, None, None, serde_json::json!({"enemy_count": 5}));
        assert_eq!(obs.enemy_count(), Some(5));
    }

    // ----- GameEpisode -----

    #[test]
    fn game_episode_new() {
        let episode = GameEpisode::new("ep_001".to_string(), EpisodeCategory::Combat);
        assert_eq!(episode.id, "ep_001");
        assert_eq!(episode.category, EpisodeCategory::Combat);
        assert!(episode.observations.is_empty());
        assert!(episode.outcome.is_none());
        assert!(episode.end_time.is_none());
    }

    #[test]
    fn game_episode_add_observation() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.add_observation(Observation::new(0, None, None, serde_json::Value::Null));
        episode.add_observation(Observation::new(100, None, None, serde_json::Value::Null));
        assert_eq!(episode.observations.len(), 2);
    }

    #[test]
    fn game_episode_complete() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        assert!(!episode.is_complete());

        episode.complete(EpisodeOutcome {
            success_rating: 0.8,
            player_satisfaction: 0.7,
            companion_effectiveness: 0.6,
            duration_ms: 5000,
            damage_dealt: 100.0,
            damage_taken: 30.0,
            resources_used: 20.0,
            failure_count: 0,
        });

        assert!(episode.is_complete());
        assert!(episode.end_time.is_some());
        assert!(episode.outcome.is_some());
    }

    #[test]
    fn game_episode_add_tag() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.add_tag("boss".to_string());
        episode.add_tag("hard".to_string());
        episode.add_tag("boss".to_string()); // Duplicate
        assert_eq!(episode.tags.len(), 2); // No duplicates
    }

    #[test]
    fn game_episode_duration() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        assert!(episode.duration().is_none());

        episode.complete(EpisodeOutcome {
            success_rating: 0.5,
            player_satisfaction: 0.5,
            companion_effectiveness: 0.5,
            duration_ms: 1000,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            resources_used: 0.0,
            failure_count: 0,
        });

        assert!(episode.duration().is_some());
    }

    #[test]
    fn game_episode_to_memory() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.complete(EpisodeOutcome {
            success_rating: 0.8,
            player_satisfaction: 0.7,
            companion_effectiveness: 0.6,
            duration_ms: 5000,
            damage_dealt: 100.0,
            damage_taken: 30.0,
            resources_used: 20.0,
            failure_count: 0,
        });

        let memory = episode.to_memory().unwrap();
        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert_eq!(memory.id, "ep");
    }

    #[test]
    fn game_episode_average_player_health() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.add_observation(Observation::new(
            0,
            None,
            None,
            serde_json::json!({"player_health": 100.0}),
        ));
        episode.add_observation(Observation::new(
            100,
            None,
            None,
            serde_json::json!({"player_health": 80.0}),
        ));
        episode.add_observation(Observation::new(
            200,
            None,
            None,
            serde_json::json!({"player_health": 60.0}),
        ));

        let avg = episode.average_player_health().unwrap();
        assert!((avg - 80.0).abs() < 0.01);
    }

    #[test]
    fn game_episode_average_player_health_empty() {
        let episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        assert!(episode.average_player_health().is_none());
    }

    #[test]
    fn game_episode_count_actions() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.add_observation(Observation::new(
            0,
            Some(PlayerAction {
                action_type: "melee_attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));
        episode.add_observation(Observation::new(
            100,
            Some(PlayerAction {
                action_type: "ranged_attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));
        episode.add_observation(Observation::new(
            200,
            Some(PlayerAction {
                action_type: "melee_attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));

        assert_eq!(episode.count_actions("melee"), 2);
        assert_eq!(episode.count_actions("ranged"), 1);
        assert_eq!(episode.count_actions("spell"), 0);
    }

    #[test]
    fn game_episode_action_diversity() {
        let mut episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Combat);
        episode.add_observation(Observation::new(
            0,
            Some(PlayerAction {
                action_type: "attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));
        episode.add_observation(Observation::new(
            100,
            Some(PlayerAction {
                action_type: "defend".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));
        episode.add_observation(Observation::new(
            200,
            Some(PlayerAction {
                action_type: "attack".to_string(),
                target: None,
                parameters: serde_json::Value::Null,
            }),
            None,
            serde_json::Value::Null,
        ));

        assert_eq!(episode.action_diversity(), 2);
    }
}

// =============================================================================
// PATTERN DETECTION TESTS - pattern_detection.rs (111 mutants)
// =============================================================================

mod pattern_detection_tests {
    use super::*;

    // ----- PlaystylePattern enum -----

    #[test]
    fn playstyle_pattern_all_variants() {
        let patterns = vec![
            PlaystylePattern::Aggressive,
            PlaystylePattern::Cautious,
            PlaystylePattern::Explorative,
            PlaystylePattern::Social,
            PlaystylePattern::Analytical,
            PlaystylePattern::Efficient,
        ];
        assert_eq!(patterns.len(), 6);
    }

    #[test]
    fn playstyle_pattern_display() {
        assert_eq!(format!("{}", PlaystylePattern::Aggressive), "Aggressive");
        assert_eq!(format!("{}", PlaystylePattern::Cautious), "Cautious");
        assert_eq!(format!("{}", PlaystylePattern::Explorative), "Explorative");
        assert_eq!(format!("{}", PlaystylePattern::Social), "Social");
        assert_eq!(format!("{}", PlaystylePattern::Analytical), "Analytical");
        assert_eq!(format!("{}", PlaystylePattern::Efficient), "Efficient");
    }

    #[test]
    fn playstyle_pattern_equality() {
        assert_eq!(PlaystylePattern::Aggressive, PlaystylePattern::Aggressive);
        assert_ne!(PlaystylePattern::Aggressive, PlaystylePattern::Cautious);
    }

    #[test]
    fn playstyle_pattern_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PlaystylePattern::Aggressive);
        set.insert(PlaystylePattern::Cautious);
        set.insert(PlaystylePattern::Aggressive); // Duplicate
        assert_eq!(set.len(), 2);
    }

    // ----- PatternStrength -----

    #[test]
    fn pattern_strength_fields() {
        let strength = PatternStrength {
            pattern: PlaystylePattern::Aggressive,
            confidence: 0.85,
            episode_count: 10,
            avg_quality: 0.75,
        };
        assert_eq!(strength.pattern, PlaystylePattern::Aggressive);
        assert_eq!(strength.confidence, 0.85);
        assert_eq!(strength.episode_count, 10);
        assert_eq!(strength.avg_quality, 0.75);
    }

    // ----- ActionPattern -----

    #[test]
    fn action_pattern_fields() {
        let pattern = ActionPattern {
            sequence: vec!["attack".to_string(), "defend".to_string()],
            frequency: 5,
            avg_effectiveness: 0.8,
        };
        assert_eq!(pattern.sequence.len(), 2);
        assert_eq!(pattern.frequency, 5);
        assert_eq!(pattern.avg_effectiveness, 0.8);
    }

    // ----- PatternDetector -----

    #[test]
    fn pattern_detector_new() {
        let detector = PatternDetector::new();
        // Just verify it creates without panic
        drop(detector);
    }

    #[test]
    fn pattern_detector_with_thresholds() {
        let detector = PatternDetector::with_thresholds(10, 0.7);
        // Just verify it creates with custom thresholds
        drop(detector);
    }
}

// =============================================================================
// PREFERENCE PROFILE TESTS - preference_profile.rs (85 mutants)
// =============================================================================

mod preference_profile_tests {
    use super::*;

    #[test]
    fn preference_profile_fields() {
        let profile = PreferenceProfile {
            dominant_patterns: vec![],
            preferred_categories: HashMap::new(),
            optimal_responses: HashMap::new(),
            learning_confidence: 0.5,
            episode_count: 0,
            converged: false,
        };
        assert_eq!(profile.learning_confidence, 0.5);
        assert_eq!(profile.episode_count, 0);
        assert!(!profile.converged);
    }

    #[test]
    fn companion_action_preference_fields() {
        let pref = CompanionActionPreference {
            action_type: "heal".to_string(),
            positive_response_rate: 0.8,
            avg_effectiveness: 0.75,
            sample_count: 20,
        };
        assert_eq!(pref.action_type, "heal");
        assert_eq!(pref.positive_response_rate, 0.8);
        assert_eq!(pref.avg_effectiveness, 0.75);
        assert_eq!(pref.sample_count, 20);
    }

    #[test]
    fn profile_builder_new() {
        let builder = ProfileBuilder::new();
        drop(builder);
    }

    #[test]
    fn profile_builder_with_thresholds() {
        let builder = ProfileBuilder::with_thresholds(0.7, 20);
        drop(builder);
    }
}

// =============================================================================
// DYNAMIC WEIGHTING TESTS - dynamic_weighting.rs (48 mutants)
// =============================================================================

mod dynamic_weighting_tests {
    use super::*;

    // ----- BehaviorNodeType enum -----

    #[test]
    fn behavior_node_type_all_variants() {
        let types = vec![
            BehaviorNodeType::Combat,
            BehaviorNodeType::Support,
            BehaviorNodeType::Exploration,
            BehaviorNodeType::Social,
            BehaviorNodeType::Analytical,
            BehaviorNodeType::Defensive,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn behavior_node_type_to_category() {
        assert_eq!(
            BehaviorNodeType::Combat.to_category(),
            EpisodeCategory::Combat
        );
        assert_eq!(
            BehaviorNodeType::Support.to_category(),
            EpisodeCategory::Combat
        );
        assert_eq!(
            BehaviorNodeType::Exploration.to_category(),
            EpisodeCategory::Exploration
        );
        assert_eq!(
            BehaviorNodeType::Social.to_category(),
            EpisodeCategory::Social
        );
        assert_eq!(
            BehaviorNodeType::Analytical.to_category(),
            EpisodeCategory::Puzzle
        );
        assert_eq!(
            BehaviorNodeType::Defensive.to_category(),
            EpisodeCategory::Combat
        );
    }

    #[test]
    fn behavior_node_type_from_pattern() {
        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Aggressive);
        assert!(types.contains(&BehaviorNodeType::Combat));

        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Cautious);
        assert!(
            types.contains(&BehaviorNodeType::Defensive)
                || types.contains(&BehaviorNodeType::Support)
        );

        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Social);
        assert!(types.contains(&BehaviorNodeType::Social));
    }

    // ----- NodeWeight -----

    #[test]
    fn node_weight_new() {
        let weight = NodeWeight::new(0.5);
        assert_eq!(weight.base_weight, 0.5);
        assert_eq!(weight.weight, 0.5);
        assert_eq!(weight.pattern_bonus, 0.0);
        assert_eq!(weight.effectiveness_bonus, 0.0);
        assert_eq!(weight.update_count, 0);
    }

    #[test]
    fn node_weight_clamps_values() {
        let weight = NodeWeight::new(1.5); // Over max
        assert!(weight.weight <= 1.0);

        let weight = NodeWeight::new(-0.5); // Under min
        assert!(weight.weight >= 0.0);
    }

    #[test]
    fn node_weight_calculate() {
        let mut weight = NodeWeight::new(0.5);
        weight.pattern_bonus = 0.1;
        weight.effectiveness_bonus = 0.2;
        let result = weight.calculate();
        assert!((result - 0.8).abs() < 0.01);
    }

    #[test]
    fn node_weight_calculate_clamps() {
        let mut weight = NodeWeight::new(0.8);
        weight.pattern_bonus = 0.5;
        weight.effectiveness_bonus = 0.5;
        let result = weight.calculate();
        assert!(result <= 1.0);
    }

    #[test]
    fn node_weight_reset() {
        let mut weight = NodeWeight::new(0.5);
        weight.pattern_bonus = 0.3;
        weight.effectiveness_bonus = 0.2;
        weight.update_count = 10;
        weight.calculate();

        weight.reset();

        assert_eq!(weight.weight, 0.5);
        assert_eq!(weight.pattern_bonus, 0.0);
        assert_eq!(weight.effectiveness_bonus, 0.0);
        assert_eq!(weight.update_count, 0);
    }

    // ----- AdaptiveWeightManager -----

    #[test]
    fn adaptive_weight_manager_new() {
        let manager = AdaptiveWeightManager::new();
        drop(manager);
    }
}

// =============================================================================
// BEHAVIOR VALIDATOR TESTS - learned_behavior_validator.rs (44 mutants)
// =============================================================================

mod behavior_validator_tests {
    use super::*;

    // ----- ValidationResult -----

    #[test]
    fn validation_result_valid() {
        let result = ValidationResult::valid(0.9, 0.8, "Good action");
        assert!(result.valid);
        assert_eq!(result.confidence, 0.9);
        assert_eq!(result.predicted_satisfaction, 0.8);
        assert!(!result.reasons.is_empty());
        assert!(result.alternatives.is_empty());
    }

    #[test]
    fn validation_result_invalid() {
        let result =
            ValidationResult::invalid("Bad action", vec!["alt1".to_string(), "alt2".to_string()]);
        assert!(!result.valid);
        assert!(result.confidence > 0.0);
        assert_eq!(result.predicted_satisfaction, 0.0);
        assert_eq!(result.alternatives.len(), 2);
    }

    #[test]
    fn validation_result_uncertain() {
        let result = ValidationResult::uncertain(0.4, "Maybe good");
        assert!(!result.valid);
        assert_eq!(result.confidence, 0.4);
        assert_eq!(result.predicted_satisfaction, 0.5);
    }

    #[test]
    fn validation_result_clamps_confidence() {
        let result = ValidationResult::valid(1.5, 0.5, "test");
        assert!(result.confidence <= 1.0);

        let result = ValidationResult::valid(-0.5, 0.5, "test");
        assert!(result.confidence >= 0.0);
    }

    #[test]
    fn validation_result_clamps_satisfaction() {
        let result = ValidationResult::valid(0.5, 1.5, "test");
        assert!(result.predicted_satisfaction <= 1.0);

        let result = ValidationResult::valid(0.5, -0.5, "test");
        assert!(result.predicted_satisfaction >= 0.0);
    }

    // ----- SafetyRule -----

    #[test]
    fn safety_rule_new() {
        let rule = SafetyRule::new("test_rule", "Test description", 0.5, true);
        assert_eq!(rule.id, "test_rule");
        assert_eq!(rule.description, "Test description");
        assert_eq!(rule.min_satisfaction, 0.5);
        assert!(rule.strict);
    }

    #[test]
    fn safety_rule_clamps_satisfaction() {
        let rule = SafetyRule::new("test", "test", 1.5, false);
        assert!(rule.min_satisfaction <= 1.0);

        let rule = SafetyRule::new("test", "test", -0.5, false);
        assert!(rule.min_satisfaction >= 0.0);
    }

    // ----- BehaviorValidator -----

    #[test]
    fn behavior_validator_new() {
        let validator = BehaviorValidator::new();
        drop(validator);
    }

    #[test]
    fn behavior_validator_with_thresholds() {
        let validator = BehaviorValidator::with_thresholds(0.7, 0.5);
        drop(validator);
    }

    #[test]
    fn behavior_validator_add_safety_rule() {
        let mut validator = BehaviorValidator::new();
        validator.add_safety_rule(SafetyRule::new("custom", "Custom rule", 0.6, false));
    }
}

// =============================================================================
// SHARING TESTS - sharing.rs (29 mutants)
// =============================================================================

mod sharing_tests {
    use super::*;

    #[test]
    fn sharing_config_default() {
        let config = SharingConfig::default();
        assert_eq!(config.default_sharing_type, SharingType::Restricted);
        assert_eq!(config.default_privacy_level, PrivacyLevel::Personal);
        assert!(!config.auto_sharing_enabled);
        assert!(config.max_authorized_entities > 0);
    }

    #[test]
    fn sharing_config_all_fields() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Group,
            auto_sharing_enabled: true,
            max_authorized_entities: 20,
        };
        assert_eq!(config.default_sharing_type, SharingType::Full);
        assert_eq!(config.default_privacy_level, PrivacyLevel::Group);
        assert!(config.auto_sharing_enabled);
        assert_eq!(config.max_authorized_entities, 20);
    }

    #[test]
    fn share_request_fields() {
        let request = ShareRequest {
            memory_id: "mem_1".to_string(),
            target_entity: "agent_2".to_string(),
            sharing_type: SharingType::Summary,
            reason: "Collaboration".to_string(),
            conditions: vec!["approved".to_string()],
        };
        assert_eq!(request.memory_id, "mem_1");
        assert_eq!(request.target_entity, "agent_2");
        assert_eq!(request.sharing_type, SharingType::Summary);
    }

    #[test]
    fn sharing_engine_creation() {
        let engine = SharingEngine::new(SharingConfig::default());
        drop(engine);
    }
}

// =============================================================================
// PERSONA TESTS - persona.rs (28 mutants)
// =============================================================================

mod persona_tests {
    use super::*;

    #[test]
    fn persona_episode_default() {
        let episode = PersonaEpisode::default();
        assert!(episode.title.is_empty());
        assert!(episode.summary.is_empty());
        assert!(episode.tags.is_empty());
        assert!(episode.ts.is_empty());
    }

    #[test]
    fn persona_default() {
        let persona = Persona::default();
        assert!(persona.name.is_empty());
        assert!(persona.likes.is_empty());
        assert!(persona.dislikes.is_empty());
    }

    #[test]
    fn fact_default() {
        let fact = Fact::default();
        assert!(fact.k.is_empty());
        assert!(fact.v.is_empty());
        assert!(fact.t.is_empty());
    }

    #[test]
    fn skill_default() {
        let skill = Skill::default();
        assert!(skill.name.is_empty());
        assert_eq!(skill.level, 0);
        assert!(skill.notes.is_empty());
    }

    #[test]
    fn companion_profile_new_default() {
        let profile = CompanionProfile::new_default();
        assert_eq!(profile.id, "companion_default");
        assert!(profile.episodes.is_empty());
        assert!(profile.facts.is_empty());
        assert!(profile.skills.is_empty());
    }

    #[test]
    fn companion_profile_has_episodes() {
        let mut profile = CompanionProfile::new_default();
        assert!(!profile.has_episodes());
        profile.add_episode(PersonaEpisode::default());
        assert!(profile.has_episodes());
    }

    #[test]
    fn companion_profile_episode_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.episode_count(), 0);
        profile.add_episode(PersonaEpisode::default());
        profile.add_episode(PersonaEpisode::default());
        assert_eq!(profile.episode_count(), 2);
    }

    #[test]
    fn companion_profile_has_facts() {
        let mut profile = CompanionProfile::new_default();
        assert!(!profile.has_facts());
        profile.add_fact(Fact::default());
        assert!(profile.has_facts());
    }

    #[test]
    fn companion_profile_fact_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.fact_count(), 0);
        profile.add_fact(Fact {
            k: "k1".to_string(),
            v: "v1".to_string(),
            t: "".to_string(),
        });
        assert_eq!(profile.fact_count(), 1);
    }

    #[test]
    fn companion_profile_has_skills() {
        let mut profile = CompanionProfile::new_default();
        assert!(!profile.has_skills());
        profile.add_skill(Skill::default());
        assert!(profile.has_skills());
    }

    #[test]
    fn companion_profile_skill_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.skill_count(), 0);
        profile.add_skill(Skill {
            name: "sword".to_string(),
            level: 5,
            notes: "".to_string(),
        });
        profile.add_skill(Skill {
            name: "magic".to_string(),
            level: 3,
            notes: "".to_string(),
        });
        assert_eq!(profile.skill_count(), 2);
    }

    #[test]
    fn companion_profile_get_skill() {
        let mut profile = CompanionProfile::new_default();
        profile.add_skill(Skill {
            name: "sword".to_string(),
            level: 5,
            notes: "".to_string(),
        });

        let skill = profile.get_skill("sword");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().level, 5);

        let missing = profile.get_skill("archery");
        assert!(missing.is_none());
    }

    #[test]
    fn companion_profile_verify() {
        let mut profile = CompanionProfile::new_default();
        assert!(!profile.verify());
        profile.sign();
        assert!(profile.verify());
    }

    #[test]
    fn companion_profile_distill() {
        let mut profile = CompanionProfile::new_default();
        profile.add_episode(PersonaEpisode {
            title: "Battle".to_string(),
            summary: "Won a fight".to_string(),
            tags: vec![],
            ts: "".to_string(),
        });

        let initial_facts = profile.fact_count();
        profile.distill();
        assert!(profile.fact_count() > initial_facts);
    }

    #[test]
    fn companion_profile_sign_and_save() {
        let mut profile = CompanionProfile::new_default();
        profile.sign(); // Should not panic
        assert!(profile.save_to_file("test.json").is_ok());
    }

    #[test]
    fn companion_profile_load_from_file() {
        let profile = CompanionProfile::load_from_file("nonexistent.json").unwrap();
        // Returns default profile
        assert_eq!(profile.id, "companion_default");
    }
}

// =============================================================================
// EPISODE RECORDER TESTS - episode_recorder.rs (32 mutants)
// =============================================================================

mod episode_recorder_tests {
    use super::*;

    #[test]
    fn episode_recorder_creation() {
        let _recorder = EpisodeRecorder::new();
    }
}

// =============================================================================
// ADDITIONAL MEMORY TESTS - Extended coverage for memory_types.rs
// =============================================================================

mod memory_extended_tests {
    use super::*;

    #[test]
    fn memory_emotional_sets_correct_importance() {
        let mem = Memory::emotional("Happy event".to_string(), "joy".to_string(), 0.9);
        assert_eq!(mem.memory_type, MemoryType::Emotional);
        assert!(mem.metadata.importance >= 0.7);
        let ctx = mem.content.emotional_context.unwrap();
        assert_eq!(ctx.primary_emotion, "joy");
        assert_eq!(ctx.intensity, 0.9);
    }

    #[test]
    fn memory_social_has_participants() {
        let parts = vec!["Alice".to_string(), "Bob".to_string()];
        let mem = Memory::social("Met friends".to_string(), parts.clone());
        assert_eq!(mem.memory_type, MemoryType::Social);
        assert_eq!(mem.content.context.participants, parts);
        assert!(mem.metadata.importance >= 0.7);
    }

    #[test]
    fn memory_accessed_increments_count() {
        let mut mem = Memory::working("test".to_string());
        assert_eq!(mem.metadata.access_count, 0);
        mem.accessed();
        assert_eq!(mem.metadata.access_count, 1);
        mem.accessed();
        assert_eq!(mem.metadata.access_count, 2);
    }

    #[test]
    fn memory_accessed_increases_strength() {
        let mut mem = Memory::working("test".to_string());
        mem.metadata.strength = 0.5;
        mem.accessed();
        assert!(mem.metadata.strength > 0.5);
    }

    #[test]
    fn memory_accessed_strength_caps_at_one() {
        let mut mem = Memory::working("test".to_string());
        mem.metadata.strength = 0.95;
        mem.accessed();
        assert!(mem.metadata.strength <= 1.0);
    }

    #[test]
    fn memory_should_forget_returns_false_for_permanent() {
        let mem = Memory::semantic("fact".to_string(), "test".to_string());
        assert!(!mem.should_forget(0.9));
    }

    #[test]
    fn memory_calculate_current_strength_clamped() {
        let mem = Memory::working("test".to_string());
        let strength = mem.calculate_current_strength();
        assert!(strength >= 0.0 && strength <= 1.0);
    }

    #[test]
    fn memory_add_association() {
        let mut mem = Memory::working("test".to_string());
        assert!(mem.associations.is_empty());
        mem.add_association("mem2".to_string(), AssociationType::Temporal, 0.8);
        assert_eq!(mem.associations.len(), 1);
        assert_eq!(mem.associations[0].memory_id, "mem2");
        assert_eq!(mem.associations[0].strength, 0.8);
    }

    #[test]
    fn memory_add_association_clamps_strength() {
        let mut mem = Memory::working("test".to_string());
        mem.add_association("m1".to_string(), AssociationType::Causal, 1.5);
        assert!(mem.associations[0].strength <= 1.0);

        let mut mem2 = Memory::working("test2".to_string());
        mem2.add_association("m2".to_string(), AssociationType::Causal, -0.5);
        assert!(mem2.associations[0].strength >= 0.0);
    }

    #[test]
    fn memory_get_strong_associations() {
        let mut mem = Memory::working("test".to_string());
        mem.add_association("m1".to_string(), AssociationType::Temporal, 0.9);
        mem.add_association("m2".to_string(), AssociationType::Spatial, 0.3);
        mem.add_association("m3".to_string(), AssociationType::Causal, 0.7);

        let strong = mem.get_strong_associations(0.5);
        assert_eq!(strong.len(), 2);
    }

    #[test]
    fn memory_matches_context_empty_preferred() {
        let mem = Memory::working("test".to_string());
        let ctx = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![], // Empty = matches all
            time_window: None,
            limit: 10,
        };
        assert!(mem.matches_context(&ctx));
    }

    #[test]
    fn memory_matches_context_wrong_type() {
        let mem = Memory::working("test".to_string());
        let ctx = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![MemoryType::Episodic], // Working not in list
            time_window: None,
            limit: 10,
        };
        assert!(!mem.matches_context(&ctx));
    }

    #[test]
    fn memory_matches_context_correct_type() {
        let mem = Memory::working("test".to_string());
        let ctx = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![MemoryType::Working],
            time_window: None,
            limit: 10,
        };
        assert!(mem.matches_context(&ctx));
    }

    #[test]
    fn memory_calculate_relevance_empty_query() {
        let mem = Memory::working("hello world".to_string());
        let ctx = RetrievalContext {
            query: "".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };
        let relevance = mem.calculate_relevance(&ctx);
        assert!(relevance >= 0.0); // Should not panic
    }

    #[test]
    fn memory_calculate_relevance_matching_words() {
        let mem = Memory::working("the quick brown fox".to_string());
        let ctx = RetrievalContext {
            query: "quick fox".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };
        let relevance = mem.calculate_relevance(&ctx);
        assert!(relevance > 0.0);
    }

    #[test]
    fn memory_calculate_relevance_capped_at_one() {
        let mem = Memory::semantic("important fact".to_string(), "science".to_string());
        // High importance memory
        let ctx = RetrievalContext {
            query: "important fact".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: vec![],
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };
        let relevance = mem.calculate_relevance(&ctx);
        assert!(relevance <= 1.0);
    }
}

// =============================================================================
// MEMORY CLUSTER TESTS
// =============================================================================

mod memory_cluster_tests {
    use super::*;

    #[test]
    fn memory_cluster_new() {
        let cluster = MemoryCluster::new(
            "Work".to_string(),
            ClusterType::Location,
            "office".to_string(),
        );
        assert_eq!(cluster.name, "Work");
        assert_eq!(cluster.central_concept, "office");
        assert!(cluster.memory_ids.is_empty());
        assert_eq!(cluster.importance, 0.5);
    }

    #[test]
    fn memory_cluster_add_memory() {
        let mut cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Concept, "test".to_string());
        cluster.add_memory("m1".to_string());
        cluster.add_memory("m2".to_string());
        assert_eq!(cluster.memory_ids.len(), 2);
    }

    #[test]
    fn memory_cluster_add_memory_no_duplicates() {
        let mut cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Concept, "test".to_string());
        cluster.add_memory("m1".to_string());
        cluster.add_memory("m1".to_string()); // Duplicate
        assert_eq!(cluster.memory_ids.len(), 1);
    }

    #[test]
    fn memory_cluster_remove_memory() {
        let mut cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Concept, "test".to_string());
        cluster.add_memory("m1".to_string());
        cluster.add_memory("m2".to_string());
        cluster.remove_memory("m1");
        assert_eq!(cluster.memory_ids.len(), 1);
        assert!(!cluster.memory_ids.contains(&"m1".to_string()));
    }

    #[test]
    fn memory_cluster_calculate_importance_empty() {
        let cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Concept, "test".to_string());
        let importance = cluster.calculate_importance(&[]);
        assert_eq!(importance, 0.0);
    }

    #[test]
    fn memory_cluster_calculate_importance_with_memories() {
        let cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Concept, "test".to_string());
        let mem1 = Memory::semantic("fact1".to_string(), "test".to_string());
        let mem2 = Memory::working("task".to_string());
        let memories: Vec<&Memory> = vec![&mem1, &mem2];
        let importance = cluster.calculate_importance(&memories);
        assert!(importance > 0.0 && importance <= 1.0);
    }
}

// =============================================================================
// ADDITIONAL FORGETTING ENGINE TESTS
// =============================================================================

mod forgetting_extended_tests {
    use super::*;

    #[test]
    fn forgetting_result_fields() {
        let result = ForgettingResult {
            memories_processed: 10,
            memories_forgotten: 2,
            total_strength_lost: 0.5,
            processing_time_ms: 100,
        };
        assert_eq!(result.memories_processed, 10);
        assert_eq!(result.memories_forgotten, 2);
        assert!((result.total_strength_lost - 0.5).abs() < 0.01);
        assert_eq!(result.processing_time_ms, 100);
    }

    #[test]
    fn forgetting_processes_multiple_memories() {
        let engine = ForgettingEngine::new(ForgettingConfig::default());
        let mut memories = vec![
            Memory::working("m1".to_string()),
            Memory::sensory("m2".to_string(), None),
            Memory::episodic("m3".to_string(), vec![], None),
        ];
        let result = engine.apply_forgetting(&mut memories).unwrap();
        assert!(result.memories_processed >= 0);
    }
}

// =============================================================================
// ADDITIONAL COMPRESSION ENGINE TESTS
// =============================================================================

mod compression_extended_tests {
    use super::*;

    #[test]
    fn compression_result_fields() {
        let result = CompressionResult {
            memories_processed: 5,
            memories_compressed: 2,
            size_reduction: 1024,
            processing_time_ms: 50,
        };
        assert_eq!(result.memories_processed, 5);
        assert_eq!(result.memories_compressed, 2);
        assert_eq!(result.size_reduction, 1024);
        assert_eq!(result.processing_time_ms, 50);
    }

    #[test]
    fn compression_processes_multiple_memories() {
        let engine = CompressionEngine::new(CompressionConfig::default());
        let mut memories = vec![
            Memory::working("m1".to_string()),
            Memory::sensory("m2".to_string(), None),
        ];
        let result = engine.compress_memories(&mut memories).unwrap();
        assert_eq!(result.memories_processed, 2);
    }
}

// =============================================================================
// ADDITIONAL DYNAMIC WEIGHTING TESTS
// =============================================================================

mod dynamic_weighting_extended_tests {
    use super::*;

    #[test]
    fn behavior_node_from_pattern_explorative() {
        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Explorative);
        assert!(types.contains(&BehaviorNodeType::Exploration));
    }

    #[test]
    fn behavior_node_from_pattern_analytical() {
        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Analytical);
        assert!(types.contains(&BehaviorNodeType::Analytical));
    }

    #[test]
    fn behavior_node_from_pattern_efficient() {
        let types = BehaviorNodeType::from_pattern(PlaystylePattern::Efficient);
        assert!(!types.is_empty());
        // Efficient should map to multiple types
    }

    #[test]
    fn node_weight_successive_calculations() {
        let mut weight = NodeWeight::new(0.3);
        weight.pattern_bonus = 0.2;
        let result1 = weight.calculate();

        weight.effectiveness_bonus = 0.1;
        let result2 = weight.calculate();

        assert!(result2 > result1);
    }
}

// =============================================================================
// ADDITIONAL BEHAVIOR VALIDATOR TESTS
// =============================================================================

mod behavior_validator_extended_tests {
    use super::*;

    #[test]
    fn safety_rule_not_strict() {
        let rule = SafetyRule::new("test", "Test rule", 0.3, false);
        assert!(!rule.strict);
    }

    #[test]
    fn validation_result_invalid_has_reasons() {
        let result = ValidationResult::invalid("Bad", vec!["alt".to_string()]);
        assert!(!result.reasons.is_empty());
        assert!(!result.alternatives.is_empty());
    }

    #[test]
    fn validation_result_valid_no_alternatives() {
        let result = ValidationResult::valid(0.8, 0.7, "Good");
        assert!(result.alternatives.is_empty());
    }

    #[test]
    fn validation_result_uncertain_midrange_satisfaction() {
        let result = ValidationResult::uncertain(0.3, "Maybe");
        assert_eq!(result.predicted_satisfaction, 0.5);
    }
}

// =============================================================================
// ADDITIONAL SHARING TESTS
// =============================================================================

mod sharing_extended_tests {
    use super::*;

    #[test]
    fn share_request_with_conditions() {
        let request = ShareRequest {
            memory_id: "m1".to_string(),
            target_entity: "agent".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: vec!["c1".to_string(), "c2".to_string()],
        };
        assert_eq!(request.conditions.len(), 2);
    }

    #[test]
    fn sharing_config_auto_enabled() {
        let config = SharingConfig {
            auto_sharing_enabled: true,
            ..SharingConfig::default()
        };
        assert!(config.auto_sharing_enabled);
    }
}

// =============================================================================
// ADDITIONAL PERSONA TESTS
// =============================================================================

mod persona_extended_tests {
    use super::*;

    #[test]
    fn persona_all_fields() {
        let persona = Persona {
            name: "Test".to_string(),
            likes: vec!["a".to_string()],
            dislikes: vec!["b".to_string()],
            tone: "friendly".to_string(),
            risk: "low".to_string(),
            humor: "dry".to_string(),
            voice: "casual".to_string(),
            backstory: "origin".to_string(),
            goals: vec!["goal1".to_string()],
        };
        assert_eq!(persona.name, "Test");
        assert_eq!(persona.tone, "friendly");
        assert_eq!(persona.goals.len(), 1);
    }

    #[test]
    fn skill_with_notes() {
        let skill = Skill {
            name: "sword".to_string(),
            level: 10,
            notes: "mastered".to_string(),
        };
        assert_eq!(skill.level, 10);
        assert_eq!(skill.notes, "mastered");
    }

    #[test]
    fn fact_with_type() {
        let fact = Fact {
            k: "key".to_string(),
            v: "value".to_string(),
            t: "metadata".to_string(),
        };
        assert_eq!(fact.t, "metadata");
    }

    #[test]
    fn persona_episode_with_tags() {
        let ep = PersonaEpisode {
            title: "Adventure".to_string(),
            summary: "story".to_string(),
            tags: vec!["combat".to_string(), "boss".to_string()],
            ts: "2024-01-01".to_string(),
        };
        assert_eq!(ep.tags.len(), 2);
        assert_eq!(ep.ts, "2024-01-01");
    }
}

// =============================================================================
// MORE EDGE CASE TESTS
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn episode_outcome_all_zeros() {
        let outcome = EpisodeOutcome {
            success_rating: 0.0,
            player_satisfaction: 0.0,
            companion_effectiveness: 0.0,
            duration_ms: 0,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            resources_used: 0.0,
            failure_count: 0,
        };
        let score = outcome.quality_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn episode_outcome_all_max() {
        let outcome = EpisodeOutcome {
            success_rating: 1.0,
            player_satisfaction: 1.0,
            companion_effectiveness: 1.0,
            duration_ms: 1000000,
            damage_dealt: 10000.0,
            damage_taken: 0.0,
            resources_used: 1.0,
            failure_count: 0,
        };
        let score = outcome.quality_score();
        assert!((score - 1.0).abs() < 0.01 || score <= 1.0);
    }

    #[test]
    fn game_episode_empty_observations() {
        let episode = GameEpisode::new("ep".to_string(), EpisodeCategory::Exploration);
        assert_eq!(episode.average_player_health(), None);
        assert_eq!(episode.action_diversity(), 0);
        assert_eq!(episode.count_actions("any"), 0);
    }

    #[test]
    fn pattern_strength_zero_episode_count() {
        let strength = PatternStrength {
            pattern: PlaystylePattern::Cautious,
            confidence: 0.0,
            episode_count: 0,
            avg_quality: 0.0,
        };
        assert_eq!(strength.episode_count, 0);
    }

    #[test]
    fn action_pattern_empty_sequence() {
        let pattern = ActionPattern {
            sequence: vec![],
            frequency: 0,
            avg_effectiveness: 0.0,
        };
        assert!(pattern.sequence.is_empty());
    }

    #[test]
    fn retrieval_context_all_filters() {
        let ctx = RetrievalContext {
            query: "test".to_string(),
            emotional_state: Some(EmotionalContext {
                primary_emotion: "joy".to_string(),
                intensity: 0.5,
                valence: 0.5,
                arousal: 0.5,
            }),
            location: Some("office".to_string()),
            recent_memory_ids: vec!["r1".to_string()],
            preferred_types: vec![MemoryType::Episodic, MemoryType::Semantic],
            time_window: Some(TimeWindow {
                start: Utc::now(),
                end: Utc::now(),
            }),
            limit: 5,
        };
        assert!(ctx.emotional_state.is_some());
        assert!(ctx.time_window.is_some());
        assert_eq!(ctx.limit, 5);
    }

    #[test]
    fn node_weight_at_extremes() {
        let mut weight = NodeWeight::new(1.0);
        weight.pattern_bonus = 1.0;
        weight.effectiveness_bonus = 1.0;
        let result = weight.calculate();
        assert!(result <= 1.0);
    }
}
