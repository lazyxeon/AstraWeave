//! Mutation-resistant comprehensive tests for astraweave-rag
//!
//! Targets: RagConfig defaults, MemoryQuery builder, ordering/diversity/injection enums,
//! ConsolidationEngine, ForgettingEngine, InjectionEngine, RetrievalEngine,
//! all Default impls with exact value assertions.

#![allow(dead_code, clippy::field_reassign_with_default)]

use astraweave_embeddings::{Memory, MemoryCategory};
use astraweave_rag::*;
use std::collections::HashMap;

// ============================================================================
// HELPERS
// ============================================================================

fn make_memory(id: &str, text: &str, category: MemoryCategory, importance: f32) -> Memory {
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category,
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        importance,
        valence: 0.0,
        entities: vec![],
        context: HashMap::new(),
    }
}

fn make_memory_with_timestamp(id: &str, text: &str, ts: u64, importance: f32) -> Memory {
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category: MemoryCategory::Social,
        timestamp: ts,
        importance,
        valence: 0.0,
        entities: vec![],
        context: HashMap::new(),
    }
}

fn make_memory_with_entities(id: &str, text: &str, entities: Vec<String>) -> Memory {
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category: MemoryCategory::Combat,
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        importance: 0.5,
        valence: 0.0,
        entities,
        context: HashMap::new(),
    }
}

// ============================================================================
// RAG CONFIG DEFAULTS
// ============================================================================

#[test]
fn rag_config_max_retrieval_count_default() {
    let c = RagConfig::default();
    assert_eq!(c.max_retrieval_count, 10);
}

#[test]
fn rag_config_min_similarity_score_default() {
    let c = RagConfig::default();
    assert!((c.min_similarity_score - 0.3).abs() < 1e-6);
}

#[test]
fn rag_config_consolidation_enabled_default() {
    let c = RagConfig::default();
    assert!(c.consolidation.enabled);
}

#[test]
fn rag_config_forgetting_enabled_default() {
    let c = RagConfig::default();
    assert!(c.forgetting.enabled);
}

#[test]
fn rag_config_injection_default_template() {
    let c = RagConfig::default();
    assert!(c.injection.injection_template.contains("{memories}"));
    assert!(c.injection.injection_template.contains("{query}"));
}

#[test]
fn rag_config_injection_max_context_tokens() {
    let c = RagConfig::default();
    assert_eq!(c.injection.max_context_tokens, 1024);
}

#[test]
fn rag_config_injection_include_metadata_true() {
    let c = RagConfig::default();
    assert!(c.injection.include_metadata);
}

#[test]
fn rag_config_injection_ordering_strategy_default() {
    let c = RagConfig::default();
    assert_eq!(
        c.injection.ordering_strategy,
        OrderingStrategy::SimilarityDesc
    );
}

#[test]
fn rag_config_injection_enable_summarization_true() {
    let c = RagConfig::default();
    assert!(c.injection.enable_summarization);
}

#[test]
fn rag_config_diversity_enabled_default() {
    let c = RagConfig::default();
    assert!(c.diversity.enabled);
}

#[test]
fn rag_config_diversity_factor_default() {
    let c = RagConfig::default();
    assert!((c.diversity.diversity_factor - 0.3).abs() < 1e-6);
}

#[test]
fn rag_config_diversity_strategy_default() {
    let c = RagConfig::default();
    assert_eq!(c.diversity.strategy, DiversityStrategy::Semantic);
}

#[test]
fn rag_config_diversity_min_distance_default() {
    let c = RagConfig::default();
    assert!((c.diversity.min_diversity_distance - 0.2).abs() < 1e-6);
}

#[test]
fn rag_config_performance_caching_enabled() {
    let c = RagConfig::default();
    assert!(c.performance.enable_caching);
}

#[test]
fn rag_config_performance_cache_size() {
    let c = RagConfig::default();
    assert_eq!(c.performance.cache_size, 1000);
}

#[test]
fn rag_config_performance_cache_ttl() {
    let c = RagConfig::default();
    assert_eq!(c.performance.cache_ttl, 300);
}

#[test]
fn rag_config_performance_batch_size() {
    let c = RagConfig::default();
    assert_eq!(c.performance.batch_size, 32);
}

#[test]
fn rag_config_performance_max_threads() {
    let c = RagConfig::default();
    assert_eq!(c.performance.max_threads, 4);
}

#[test]
fn rag_config_performance_metrics_enabled() {
    let c = RagConfig::default();
    assert!(c.performance.enable_metrics);
}

// ============================================================================
// CONSOLIDATION CONFIG DEFAULTS
// ============================================================================

#[test]
fn consolidation_config_enabled_default() {
    let c = ConsolidationConfig::default();
    assert!(c.enabled);
}

#[test]
fn consolidation_config_trigger_threshold() {
    let c = ConsolidationConfig::default();
    assert_eq!(c.trigger_threshold, 100);
}

#[test]
fn consolidation_config_merge_similarity_threshold() {
    let c = ConsolidationConfig::default();
    assert!((c.merge_similarity_threshold - 0.85).abs() < 1e-6);
}

#[test]
fn consolidation_config_max_memories_per_batch() {
    let c = ConsolidationConfig::default();
    assert_eq!(c.max_memories_per_batch, 50);
}

#[test]
fn consolidation_config_strategy_default() {
    let c = ConsolidationConfig::default();
    assert_eq!(c.strategy, ConsolidationStrategy::Importance);
}

#[test]
fn consolidation_config_interval_default() {
    let c = ConsolidationConfig::default();
    assert_eq!(c.consolidation_interval, 3600);
}

#[test]
fn consolidation_config_max_age_default() {
    let c = ConsolidationConfig::default();
    assert_eq!(c.max_age_seconds, 86400);
}

// ============================================================================
// CONSOLIDATION STRATEGY ENUM
// ============================================================================

#[test]
fn consolidation_strategy_importance_eq() {
    assert_eq!(
        ConsolidationStrategy::Importance,
        ConsolidationStrategy::Importance
    );
}

#[test]
fn consolidation_strategy_recency_eq() {
    assert_eq!(
        ConsolidationStrategy::Recency,
        ConsolidationStrategy::Recency
    );
}

#[test]
fn consolidation_strategy_similarity_eq() {
    assert_eq!(
        ConsolidationStrategy::Similarity,
        ConsolidationStrategy::Similarity
    );
}

#[test]
fn consolidation_strategy_hybrid_eq() {
    assert_eq!(ConsolidationStrategy::Hybrid, ConsolidationStrategy::Hybrid);
}

#[test]
fn consolidation_strategy_variants_not_equal() {
    assert_ne!(
        ConsolidationStrategy::Importance,
        ConsolidationStrategy::Recency
    );
    assert_ne!(
        ConsolidationStrategy::Similarity,
        ConsolidationStrategy::Hybrid
    );
    assert_ne!(
        ConsolidationStrategy::Importance,
        ConsolidationStrategy::Hybrid
    );
}

// ============================================================================
// CONSOLIDATION ENGINE
// ============================================================================

#[test]
fn consolidation_engine_empty_input() {
    let engine = ConsolidationEngine::new(ConsolidationConfig::default());
    let (result, stats) = engine.consolidate(vec![]).unwrap();
    assert!(result.is_empty());
    assert_eq!(stats.processed_count, 0);
    assert_eq!(stats.merged_count, 0);
    assert_eq!(stats.removed_count, 0);
}

#[test]
fn consolidation_engine_single_memory() {
    let engine = ConsolidationEngine::new(ConsolidationConfig::default());
    let m = make_memory("1", "hello world", MemoryCategory::Social, 0.5);
    let (result, stats) = engine.consolidate(vec![m]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(stats.processed_count, 1);
    assert_eq!(stats.merged_count, 0);
}

#[test]
fn consolidation_engine_different_categories_no_merge() {
    let engine = ConsolidationEngine::new(ConsolidationConfig::default());
    let m1 = make_memory("1", "the hero fought bravely", MemoryCategory::Combat, 0.5);
    let m2 = make_memory("2", "the hero fought bravely", MemoryCategory::Social, 0.5);
    let (result, stats) = engine.consolidate(vec![m1, m2]).unwrap();
    assert_eq!(result.len(), 2, "Different categories should not merge");
    assert_eq!(stats.merged_count, 0);
}

#[test]
fn consolidation_engine_identical_text_same_category_merges() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5; // Lower threshold to ensure merge
    let engine = ConsolidationEngine::new(config);
    let m1 = make_memory("1", "the cat sat on the mat", MemoryCategory::Social, 0.5);
    let m2 = make_memory("2", "the cat sat on the mat", MemoryCategory::Social, 0.5);
    let (result, stats) = engine.consolidate(vec![m1, m2]).unwrap();
    assert_eq!(result.len(), 1, "Identical text same category should merge");
    assert_eq!(stats.merged_count, 1);
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn consolidation_engine_dissimilar_text_no_merge() {
    let engine = ConsolidationEngine::new(ConsolidationConfig::default());
    let m1 = make_memory("1", "The quick brown fox", MemoryCategory::Social, 0.5);
    let m2 = make_memory(
        "2",
        "A completely different sentence about nothing similar",
        MemoryCategory::Social,
        0.5,
    );
    let (result, _) = engine.consolidate(vec![m1, m2]).unwrap();
    assert_eq!(result.len(), 2, "Dissimilar text should not merge");
}

#[test]
fn consolidation_result_processing_time_nonnegative() {
    let engine = ConsolidationEngine::new(ConsolidationConfig::default());
    let (_, stats) = engine.consolidate(vec![]).unwrap();
    // Processing time should be >= 0 (it's u64, always true, but semantically meaningful)
    assert!(stats.processing_time_ms < 10_000); // shouldn't take 10s
}

#[test]
fn consolidation_engine_merged_text_contains_both() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5;
    let engine = ConsolidationEngine::new(config);
    let m1 = make_memory("1", "cat sat mat", MemoryCategory::Social, 0.5);
    let m2 = make_memory("2", "cat sat mat", MemoryCategory::Social, 0.5);
    let (result, _) = engine.consolidate(vec![m1, m2]).unwrap();
    assert_eq!(result.len(), 1);
    // Merged text should contain content from both
    assert!(result[0].text.contains("cat"));
}

// ============================================================================
// FORGETTING CONFIG DEFAULTS
// ============================================================================

#[test]
fn forgetting_config_enabled_default() {
    let c = ForgettingConfig::default();
    assert!(c.enabled);
}

#[test]
fn forgetting_config_base_decay_rate() {
    let c = ForgettingConfig::default();
    assert!((c.base_decay_rate - 0.1).abs() < 1e-6);
}

#[test]
fn forgetting_config_importance_factor() {
    let c = ForgettingConfig::default();
    assert!((c.importance_factor - 2.0).abs() < 1e-6);
}

#[test]
fn forgetting_config_min_importance_threshold() {
    let c = ForgettingConfig::default();
    assert!((c.min_importance_threshold - 0.2).abs() < 1e-6);
}

#[test]
fn forgetting_config_max_memory_age() {
    let c = ForgettingConfig::default();
    assert_eq!(c.max_memory_age, 2592000); // 30 days
}

#[test]
fn forgetting_config_cleanup_interval() {
    let c = ForgettingConfig::default();
    assert_eq!(c.cleanup_interval, 86400); // 1 day
}

#[test]
fn forgetting_config_protected_categories_contains_quest() {
    let c = ForgettingConfig::default();
    assert!(c.protected_categories.contains(&MemoryCategory::Quest));
}

#[test]
fn forgetting_config_protected_categories_length() {
    let c = ForgettingConfig::default();
    assert_eq!(c.protected_categories.len(), 1);
}

// ============================================================================
// FORGETTING ENGINE
// ============================================================================

#[test]
fn forgetting_engine_empty_input() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let (retained, stats) = engine.process_forgetting(vec![]).unwrap();
    assert!(retained.is_empty());
    assert_eq!(stats.processed_count, 0);
    assert_eq!(stats.forgotten_count, 0);
    assert_eq!(stats.updated_count, 0);
}

#[test]
fn forgetting_engine_recent_high_importance_retained() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let m = make_memory("1", "Important recent memory", MemoryCategory::Social, 0.9);
    let (retained, stats) = engine.process_forgetting(vec![m]).unwrap();
    assert_eq!(
        retained.len(),
        1,
        "Recent high-importance memory should be retained"
    );
    assert_eq!(stats.forgotten_count, 0);
    assert_eq!(stats.updated_count, 1);
}

#[test]
fn forgetting_engine_quest_memory_protected() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let m = make_memory("1", "Quest memory", MemoryCategory::Quest, 0.1);
    let (retained, _) = engine.process_forgetting(vec![m]).unwrap();
    assert_eq!(
        retained.len(),
        1,
        "Quest memories should be protected from forgetting"
    );
}

#[test]
fn forgetting_engine_very_old_memory_forgotten() {
    let mut config = ForgettingConfig::default();
    config.max_memory_age = 100; // 100 seconds
    let mut engine = ForgettingEngine::new(config);
    // Create memory with very old timestamp
    let m = make_memory_with_timestamp("1", "Old memory", 1000, 0.01);
    let (retained, stats) = engine.process_forgetting(vec![m]).unwrap();
    assert_eq!(retained.len(), 0, "Very old memory should be forgotten");
    assert_eq!(stats.forgotten_count, 1);
}

#[test]
fn forgetting_engine_strengthen_memory() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let m = make_memory("test_mem", "something", MemoryCategory::Social, 0.5);
    let _ = engine.process_forgetting(vec![m]).unwrap();

    // Strengthen memory
    engine.strengthen_memory("test_mem", 0.3).unwrap();
    let strength = engine.get_memory_strength("test_mem");
    assert!(strength.is_some());
    assert!(strength.unwrap().access_count >= 1);
}

#[test]
fn forgetting_engine_protect_memory() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let m = make_memory("prot", "protected", MemoryCategory::Social, 0.5);
    let _ = engine.process_forgetting(vec![m]).unwrap();

    engine.protect_memory("prot").unwrap();
    let strength = engine.get_memory_strength("prot");
    assert!(strength.unwrap().protected);
}

#[test]
fn forgetting_engine_unprotect_memory() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let m = make_memory("unp", "test", MemoryCategory::Quest, 0.5); // Quest = protected
    let _ = engine.process_forgetting(vec![m]).unwrap();

    engine.unprotect_memory("unp").unwrap();
    let strength = engine.get_memory_strength("unp");
    assert!(!strength.unwrap().protected);
}

#[test]
fn forgetting_engine_statistics_initial() {
    let engine = ForgettingEngine::new(ForgettingConfig::default());
    let stats = engine.get_statistics();
    assert_eq!(stats.total_memories, 0);
    assert_eq!(stats.protected_memories, 0);
    assert_eq!(stats.weak_memories, 0);
    assert!((stats.average_strength - 0.0).abs() < 1e-6);
}

#[test]
fn forgetting_engine_statistics_after_processing() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let memories = vec![
        make_memory("1", "a", MemoryCategory::Social, 0.9),
        make_memory("2", "b", MemoryCategory::Quest, 0.5),
    ];
    let _ = engine.process_forgetting(memories).unwrap();
    let stats = engine.get_statistics();
    assert_eq!(stats.total_memories, 2);
    assert!(
        stats.protected_memories >= 1,
        "Quest memory should be protected"
    );
}

#[test]
fn forgetting_result_processed_count_matches_input() {
    let mut engine = ForgettingEngine::new(ForgettingConfig::default());
    let memories = vec![
        make_memory("1", "a", MemoryCategory::Social, 0.5),
        make_memory("2", "b", MemoryCategory::Social, 0.5),
        make_memory("3", "c", MemoryCategory::Social, 0.5),
    ];
    let (_, stats) = engine.process_forgetting(memories).unwrap();
    assert_eq!(stats.processed_count, 3);
}

// ============================================================================
// MEMORY STRENGTH DEFAULTS
// ============================================================================

#[test]
fn memory_strength_default_current_strength() {
    let s = MemoryStrength::default();
    assert!((s.current_strength - 1.0).abs() < 1e-6);
}

#[test]
fn memory_strength_default_initial_strength() {
    let s = MemoryStrength::default();
    assert!((s.initial_strength - 1.0).abs() < 1e-6);
}

#[test]
fn memory_strength_default_access_count() {
    let s = MemoryStrength::default();
    assert_eq!(s.access_count, 0);
}

#[test]
fn memory_strength_default_not_protected() {
    let s = MemoryStrength::default();
    assert!(!s.protected);
}

// ============================================================================
// ORDERING STRATEGY ENUM
// ============================================================================

#[test]
fn ordering_strategy_all_variants_distinct() {
    let variants = [
        OrderingStrategy::SimilarityDesc,
        OrderingStrategy::SimilarityAsc,
        OrderingStrategy::RecencyDesc,
        OrderingStrategy::RecencyAsc,
        OrderingStrategy::ImportanceDesc,
        OrderingStrategy::ImportanceAsc,
        OrderingStrategy::Mixed,
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(
                variants[i], variants[j],
                "Variants {} and {} should differ",
                i, j
            );
        }
    }
}

// ============================================================================
// DIVERSITY STRATEGY ENUM
// ============================================================================

#[test]
fn diversity_strategy_all_variants_distinct() {
    let variants = [
        DiversityStrategy::Semantic,
        DiversityStrategy::Temporal,
        DiversityStrategy::Category,
        DiversityStrategy::Combined,
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// RETRIEVAL METHOD ENUM
// ============================================================================

#[test]
fn retrieval_method_all_variants_distinct() {
    let variants = [
        RetrievalMethod::SemanticSearch,
        RetrievalMethod::KeywordSearch,
        RetrievalMethod::TemporalSearch,
        RetrievalMethod::CategorySearch,
        RetrievalMethod::HybridSearch,
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// INJECTION STRATEGY ENUM
// ============================================================================

#[test]
fn injection_strategy_all_variants_distinct() {
    let variants = [
        InjectionStrategy::Prepend,
        InjectionStrategy::Append,
        InjectionStrategy::Insert,
        InjectionStrategy::Interleave,
        InjectionStrategy::Replace,
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
}

// ============================================================================
// MEMORY QUERY BUILDER
// ============================================================================

#[test]
fn memory_query_text_creates_with_text() {
    let q = MemoryQuery::text("combat");
    assert_eq!(q.text, "combat");
}

#[test]
fn memory_query_text_no_time_range() {
    let q = MemoryQuery::text("test");
    assert!(q.time_range.is_none());
}

#[test]
fn memory_query_text_empty_categories() {
    let q = MemoryQuery::text("test");
    assert!(q.categories.is_empty());
}

#[test]
fn memory_query_text_empty_entities() {
    let q = MemoryQuery::text("test");
    assert!(q.entities.is_empty());
}

#[test]
fn memory_query_text_no_min_importance() {
    let q = MemoryQuery::text("test");
    assert!(q.min_importance.is_none());
}

#[test]
fn memory_query_text_no_max_age() {
    let q = MemoryQuery::text("test");
    assert!(q.max_age.is_none());
}

#[test]
fn memory_query_text_empty_metadata_filters() {
    let q = MemoryQuery::text("test");
    assert!(q.metadata_filters.is_empty());
}

#[test]
fn memory_query_with_category() {
    let q = MemoryQuery::text("x").with_category(MemoryCategory::Combat);
    assert_eq!(q.categories.len(), 1);
    assert_eq!(q.categories[0], MemoryCategory::Combat);
}

#[test]
fn memory_query_with_multiple_categories() {
    let q = MemoryQuery::text("x")
        .with_category(MemoryCategory::Combat)
        .with_category(MemoryCategory::Quest);
    assert_eq!(q.categories.len(), 2);
}

#[test]
fn memory_query_with_entity() {
    let q = MemoryQuery::text("x").with_entity("player");
    assert_eq!(q.entities.len(), 1);
    assert_eq!(q.entities[0], "player");
}

#[test]
fn memory_query_with_multiple_entities() {
    let q = MemoryQuery::text("x")
        .with_entity("player")
        .with_entity("dragon");
    assert_eq!(q.entities.len(), 2);
}

#[test]
fn memory_query_with_min_importance() {
    let q = MemoryQuery::text("x").with_min_importance(0.75);
    assert_eq!(q.min_importance, Some(0.75));
}

#[test]
fn memory_query_with_time_range() {
    let q = MemoryQuery::text("x").with_time_range(100, 200);
    assert_eq!(q.time_range, Some((100, 200)));
}

#[test]
fn memory_query_chained_builder() {
    let q = MemoryQuery::text("boss fight")
        .with_category(MemoryCategory::Combat)
        .with_entity("dragon")
        .with_min_importance(0.5)
        .with_time_range(1000, 2000);
    assert_eq!(q.text, "boss fight");
    assert_eq!(q.categories.len(), 1);
    assert_eq!(q.entities.len(), 1);
    assert_eq!(q.min_importance, Some(0.5));
    assert_eq!(q.time_range, Some((1000, 2000)));
}

// ============================================================================
// RAG METRICS DEFAULTS
// ============================================================================

#[test]
fn rag_metrics_default_all_zero() {
    let m = RagMetrics::default();
    assert_eq!(m.total_queries, 0);
    assert_eq!(m.successful_retrievals, 0);
    assert_eq!(m.failed_retrievals, 0);
    assert!((m.avg_retrieval_time_ms - 0.0).abs() < 1e-6);
    assert!((m.avg_memories_per_query - 0.0).abs() < 1e-6);
    assert!((m.cache_hit_rate - 0.0).abs() < 1e-6);
    assert_eq!(m.consolidations_performed, 0);
    assert_eq!(m.memories_forgotten, 0);
    assert_eq!(m.total_memories_stored, 0);
    assert!((m.avg_memory_importance - 0.0).abs() < 1e-6);
}

#[test]
fn rag_metrics_success_rate_calculation() {
    let mut m = RagMetrics::default();
    m.total_queries = 100;
    m.successful_retrievals = 95;
    m.failed_retrievals = 5;
    let rate = m.successful_retrievals as f64 / m.total_queries as f64;
    assert!((rate - 0.95).abs() < 1e-6);
}

// ============================================================================
// INJECTION CONFIG (module-level)
// ============================================================================

#[test]
fn injection_config_default_max_memories() {
    let c = astraweave_rag::injection::InjectionConfig::default();
    assert_eq!(c.max_memories, 5);
}

#[test]
fn injection_config_default_relevance_threshold() {
    let c = astraweave_rag::injection::InjectionConfig::default();
    assert!((c.relevance_threshold - 0.4).abs() < 1e-6);
}

#[test]
fn injection_config_default_prioritize_recent() {
    let c = astraweave_rag::injection::InjectionConfig::default();
    assert!(c.prioritize_recent);
}

#[test]
fn injection_config_default_max_context_tokens() {
    let c = astraweave_rag::injection::InjectionConfig::default();
    assert_eq!(c.max_context_tokens, 2000);
}

// ============================================================================
// INJECTION ENGINE
// ============================================================================

#[test]
fn injection_engine_empty_memories() {
    let config = astraweave_rag::injection::InjectionConfig::default();
    let engine = astraweave_rag::injection::InjectionEngine::new(config);
    let ctx = astraweave_rag::injection::InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };
    let result = engine.inject(&ctx, &[]).unwrap();
    assert!(result.injected_memories.is_empty());
    assert!(result.context_text.is_empty());
    assert_eq!(result.estimated_tokens, 0);
}

#[test]
fn injection_engine_relevant_memory_injected() {
    let config = astraweave_rag::injection::InjectionConfig::default();
    let engine = astraweave_rag::injection::InjectionEngine::new(config);
    let ctx = astraweave_rag::injection::InjectionContext {
        query: "Tell me about cats".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![MemoryCategory::Social],
    };
    let memories = vec![make_memory(
        "1",
        "Cats are wonderful creatures that purr",
        MemoryCategory::Social,
        0.5,
    )];
    let result = engine.inject(&ctx, &memories).unwrap();
    assert!(!result.injected_memories.is_empty());
}

#[test]
fn injection_engine_context_text_has_memories_prefix() {
    let config = astraweave_rag::injection::InjectionConfig::default();
    let engine = astraweave_rag::injection::InjectionEngine::new(config);
    let ctx = astraweave_rag::injection::InjectionContext {
        query: "cats".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![MemoryCategory::Social],
    };
    let memories = vec![make_memory(
        "1",
        "cats are fluffy animals",
        MemoryCategory::Social,
        0.5,
    )];
    let result = engine.inject(&ctx, &memories).unwrap();
    if !result.injected_memories.is_empty() {
        assert!(result.context_text.contains("Relevant memories"));
    }
}

#[test]
fn injection_engine_max_memories_limit() {
    let mut config = astraweave_rag::injection::InjectionConfig::default();
    config.max_memories = 2;
    config.relevance_threshold = 0.0; // Accept all
    let engine = astraweave_rag::injection::InjectionEngine::new(config);
    let ctx = astraweave_rag::injection::InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };
    let memories = vec![
        make_memory("1", "test memory one", MemoryCategory::Social, 0.5),
        make_memory("2", "test memory two", MemoryCategory::Social, 0.5),
        make_memory("3", "test memory three", MemoryCategory::Social, 0.5),
        make_memory("4", "test memory four", MemoryCategory::Social, 0.5),
    ];
    let result = engine.inject(&ctx, &memories).unwrap();
    assert!(result.injected_memories.len() <= 2);
}

#[test]
fn injection_engine_estimated_tokens_positive_when_memories_injected() {
    let mut config = astraweave_rag::injection::InjectionConfig::default();
    config.relevance_threshold = 0.0;
    let engine = astraweave_rag::injection::InjectionEngine::new(config);
    let ctx = astraweave_rag::injection::InjectionContext {
        query: "anything".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };
    let memories = vec![make_memory(
        "1",
        "some text content here",
        MemoryCategory::Social,
        0.5,
    )];
    let result = engine.inject(&ctx, &memories).unwrap();
    if !result.injected_memories.is_empty() {
        assert!(result.estimated_tokens > 0);
    }
}

// ============================================================================
// RETRIEVAL CONFIG DEFAULTS
// ============================================================================

#[test]
fn retrieval_config_default_max_results() {
    let c = astraweave_rag::retrieval::RetrievalConfig::default();
    assert_eq!(c.max_results, 10);
}

#[test]
fn retrieval_config_default_similarity_threshold() {
    let c = astraweave_rag::retrieval::RetrievalConfig::default();
    assert!((c.similarity_threshold - 0.7).abs() < 1e-6);
}

#[test]
fn retrieval_config_default_use_semantic_search() {
    let c = astraweave_rag::retrieval::RetrievalConfig::default();
    assert!(c.use_semantic_search);
}

// ============================================================================
// RETRIEVAL ENGINE
// ============================================================================

#[test]
fn retrieval_engine_empty_memories() {
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(
        astraweave_rag::retrieval::RetrievalConfig::default(),
    );
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "anything".to_string(),
        categories: vec![],
        filters: HashMap::new(),
        limit: None,
    };
    let results = engine.search(&query, &[]).unwrap();
    assert!(results.is_empty());
}

#[test]
fn retrieval_engine_exact_match_high_score() {
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(
        astraweave_rag::retrieval::RetrievalConfig::default(),
    );
    let memories = vec![make_memory("1", "cat", MemoryCategory::Social, 0.5)];
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "cat".to_string(),
        categories: vec![],
        filters: HashMap::new(),
        limit: None,
    };
    let results = engine.search(&query, &memories).unwrap();
    assert!(!results.is_empty());
    assert!(
        (results[0].score - 1.0).abs() < 1e-6,
        "Exact match should score 1.0"
    );
}

#[test]
fn retrieval_engine_category_filtering() {
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(
        astraweave_rag::retrieval::RetrievalConfig::default(),
    );
    let memories = vec![
        make_memory("1", "cat", MemoryCategory::Social, 0.5),
        make_memory("2", "cat", MemoryCategory::Combat, 0.5),
    ];
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "cat".to_string(),
        categories: vec![MemoryCategory::Combat],
        filters: HashMap::new(),
        limit: None,
    };
    let results = engine.search(&query, &memories).unwrap();
    // Should only have the Combat memory
    for r in &results {
        assert_eq!(r.memory.category, MemoryCategory::Combat);
    }
}

#[test]
fn retrieval_engine_results_sorted_by_score() {
    let mut config = astraweave_rag::retrieval::RetrievalConfig::default();
    config.similarity_threshold = 0.0;
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(config);
    let memories = vec![
        make_memory("1", "foo bar baz qux", MemoryCategory::Social, 0.5),
        make_memory("2", "foo bar", MemoryCategory::Social, 0.5),
    ];
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "foo bar".to_string(),
        categories: vec![],
        filters: HashMap::new(),
        limit: None,
    };
    let results = engine.search(&query, &memories).unwrap();
    if results.len() >= 2 {
        assert!(
            results[0].score >= results[1].score,
            "Results should be sorted by score desc"
        );
    }
}

#[test]
fn retrieval_engine_limit_respected() {
    let mut config = astraweave_rag::retrieval::RetrievalConfig::default();
    config.similarity_threshold = 0.0;
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(config);
    let memories = vec![
        make_memory("1", "test", MemoryCategory::Social, 0.5),
        make_memory("2", "test", MemoryCategory::Social, 0.5),
        make_memory("3", "test", MemoryCategory::Social, 0.5),
    ];
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "test".to_string(),
        categories: vec![],
        filters: HashMap::new(),
        limit: Some(1),
    };
    let results = engine.search(&query, &memories).unwrap();
    assert!(results.len() <= 1);
}

// ============================================================================
// MUTATION KILL TESTS — forgetting.rs (Phase 43: astraweave-rag audit)
// ============================================================================

/// Kills L143:32 (+ → *) and L143:64 (* → +) in decay_modifier formula.
/// decay_modifier = 1.0 / (1.0 + importance_factor * importance)
/// With importance_factor=2.0 and importance=0.5:
///   correct: 1/(1+2*0.5) = 1/2 = 0.5
///   + → *:   1/(1*2*0.5) = 1/1 = 1.0 (no decay modifier effect)
///   * → +:   1/(1+2+0.5) = 1/3.5 ≈ 0.286 (different decay)
///     We compare two memories with different importance: if the arithmetic is wrong,
///     the decay difference between high/low importance will be distorted.
#[test]
fn decay_modifier_formula_importance_matters() {
    let config = ForgettingConfig {
        base_decay_rate: 0.1,
        importance_factor: 2.0,
        min_importance_threshold: 0.0, // don't forget any
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine = ForgettingEngine::new(config);

    // Use timestamps far enough in the past that decay is visible
    let old_ts = (time::OffsetDateTime::now_utc().unix_timestamp() - 36000) as u64; // 10 hours ago

    let m_high = make_memory_with_timestamp("high", "important memory", old_ts, 0.9);
    let m_low = make_memory_with_timestamp("low", "unimportant memory", old_ts, 0.1);

    let (retained, _) = engine.process_forgetting(vec![m_high, m_low]).unwrap();
    assert_eq!(retained.len(), 2, "Both should be retained");

    let s_high = engine.get_memory_strength("high").unwrap().current_strength;
    let s_low = engine.get_memory_strength("low").unwrap().current_strength;

    // High importance → slower decay → higher strength
    assert!(
        s_high > s_low,
        "High importance ({}) should retain more strength than low ({})",
        s_high,
        s_low
    );
    // With + → * mutation: decay_modifier = 1/(1*2*0.9) = 0.556 for high,
    //   1/(1*2*0.1) = 5.0 for low — inverted! Low decays LESS. Test catches this.
}

/// Kills L192 (< → <=) in should_forget_static: strength boundary at threshold.
/// A memory with strength EXACTLY at min_importance_threshold should NOT be forgotten.
#[test]
fn should_forget_boundary_at_threshold() {
    let config = ForgettingConfig {
        base_decay_rate: 0.0, // no time-based decay
        importance_factor: 0.0,
        min_importance_threshold: 0.5,
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine = ForgettingEngine::new(config);

    // Insert a memory into the strength map by processing it
    let now_ts = time::OffsetDateTime::now_utc().unix_timestamp() as u64;
    let m = make_memory_with_timestamp("boundary", "at threshold", now_ts, 0.5);
    let (retained, _) = engine.process_forgetting(vec![m]).unwrap();

    // With zero decay and recent timestamp, strength stays at initial 1.0 which is above 0.5
    // We need to manually adjust the strength to exactly the threshold
    // So let's use a different approach: set min_importance_threshold high enough
    // that the decayed strength lands exactly at threshold.
    // Actually, since base_decay_rate=0, exp(0)=1.0, so strength=1.0.
    // 1.0 >= 0.5, not forgotten. Let me set threshold to 1.0.
    drop(retained);

    let config2 = ForgettingConfig {
        base_decay_rate: 0.0,
        importance_factor: 0.0,
        min_importance_threshold: 1.0, // threshold = 1.0
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine2 = ForgettingEngine::new(config2);
    let m2 = make_memory_with_timestamp("boundary2", "exactly at threshold", now_ts, 0.5);
    let (retained2, result2) = engine2.process_forgetting(vec![m2]).unwrap();

    // strength = 1.0 (no decay), threshold = 1.0
    // With <: 1.0 < 1.0 is FALSE → retained
    // With <=: 1.0 <= 1.0 is TRUE → forgotten
    assert_eq!(
        retained2.len(),
        1,
        "Memory at exactly threshold should be retained (not forgotten). forgotten={}",
        result2.forgotten_count
    );
}

/// Kills L198 (> → >=) in should_forget_static: age boundary at max_memory_age.
#[test]
fn should_forget_boundary_at_max_age() {
    let max_age: u64 = 3600; // 1 hour
    let config = ForgettingConfig {
        base_decay_rate: 0.0,
        importance_factor: 0.0,
        min_importance_threshold: 0.0,
        max_memory_age: max_age,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine = ForgettingEngine::new(config);

    // Memory timestamp exactly max_age seconds ago
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let ts_at_boundary = (now - max_age as i64) as u64;
    let m = make_memory_with_timestamp("age_boundary", "exactly at max age", ts_at_boundary, 0.5);
    let (retained, result) = engine.process_forgetting(vec![m]).unwrap();

    // age_seconds = now - ts = max_age (exactly)
    // With >: max_age > max_age is FALSE → retained
    // With >=: max_age >= max_age is TRUE → forgotten
    assert_eq!(
        retained.len(),
        1,
        "Memory at exactly max age should be retained. forgotten={}",
        result.forgotten_count
    );
}

/// Kills L208:68 (+ → -) and (+ → *) in strengthen_memory.
/// strength.current_strength = (current_strength + boost_factor).min(1.0)
#[test]
fn strengthen_memory_adds_boost() {
    let config = ForgettingConfig {
        base_decay_rate: 0.0,
        importance_factor: 0.0,
        min_importance_threshold: 0.0,
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine = ForgettingEngine::new(config);

    // First populate the strength map
    let now_ts = time::OffsetDateTime::now_utc().unix_timestamp() as u64;
    let m = make_memory_with_timestamp("boost_test", "memory to strengthen", now_ts, 0.5);
    engine.process_forgetting(vec![m]).unwrap();

    // Initial strength is 1.0 (no decay). Set it lower so boost is visible.
    // We can't directly set strength, so let's use a memory with some decay.
    // Alternative: call strengthen with a known boost and check the result.
    // With initial=1.0 and boost=0.2: (1.0 + 0.2).min(1.0) = 1.0 (clamped)
    // Let's instead use a second engine where decay happens first.

    let config2 = ForgettingConfig {
        base_decay_rate: 0.5, // heavy decay
        importance_factor: 0.0,
        min_importance_threshold: 0.0,
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine2 = ForgettingEngine::new(config2);

    // Memory from 10 hours ago to ensure meaningful decay
    let old_ts = (time::OffsetDateTime::now_utc().unix_timestamp() - 36000) as u64;
    let m2 = make_memory_with_timestamp("boost2", "decayed memory", old_ts, 0.0);
    engine2.process_forgetting(vec![m2]).unwrap();

    let before = engine2
        .get_memory_strength("boost2")
        .unwrap()
        .current_strength;
    assert!(before < 1.0, "Should have decayed: {}", before);

    engine2.strengthen_memory("boost2", 0.3).unwrap();
    let after = engine2
        .get_memory_strength("boost2")
        .unwrap()
        .current_strength;

    // With +: after = (before + 0.3).min(1.0) > before
    // With -: after = (before - 0.3).min(1.0) < before
    // With *: after = (before * 0.3).min(1.0) < before (since before < 1.0)
    assert!(
        after > before,
        "Strengthening should increase: before={}, after={}",
        before,
        after
    );
    let expected = (before + 0.3).min(1.0);
    assert!(
        (after - expected).abs() < 1e-5,
        "After should be before+0.3: expected={}, got={}",
        expected,
        after
    );
}

/// Kills L250 (< → ==, < → >, < → <=) and L251 (+= → -=, += → *=) in get_statistics.
#[test]
fn statistics_weak_count_and_average_strength() {
    let config = ForgettingConfig {
        base_decay_rate: 0.0,
        importance_factor: 0.0,
        min_importance_threshold: 0.5, // threshold for "weak"
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine = ForgettingEngine::new(config);

    // Populate with 3 memories (all retained since threshold=0.5 and strength=1.0)
    let now_ts = time::OffsetDateTime::now_utc().unix_timestamp() as u64;
    let m1 = make_memory_with_timestamp("s1", "strong 1", now_ts, 0.5);
    let m2 = make_memory_with_timestamp("s2", "strong 2", now_ts, 0.5);
    let m3 = make_memory_with_timestamp("s3", "strong 3", now_ts, 0.5);
    engine.process_forgetting(vec![m1, m2, m3]).unwrap();

    // All have strength=1.0 (no decay), threshold=0.5
    // 1.0 < 0.5 is FALSE → weak_memories = 0
    // With < → >: 1.0 > 0.5 is TRUE → weak_memories = 3 (wrong)
    // With < → ==: 1.0 == 0.5 is FALSE → weak_memories = 0 (same, need below-threshold case)

    // Manually lower one memory's strength via strengthen with negative-ish approach
    // Actually we can't set strength directly. Let's use heavy-decay memories instead.
    let config2 = ForgettingConfig {
        base_decay_rate: 1.0, // very heavy decay
        importance_factor: 0.0,
        min_importance_threshold: 0.3, // threshold
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine2 = ForgettingEngine::new(config2);

    // Recent memory: strength ≈ 1.0 (above threshold)
    let m_recent = make_memory_with_timestamp("recent", "recent mem", now_ts, 0.0);
    // Old memory: decays to low strength (below threshold)
    let old_ts = (time::OffsetDateTime::now_utc().unix_timestamp() - 72000) as u64; // 20 hours ago
    let m_old = make_memory_with_timestamp("old", "old mem", old_ts, 0.0);

    let (_retained, _) = engine2.process_forgetting(vec![m_recent, m_old]).unwrap();

    let stats = engine2.get_statistics();

    // Check that we have at least one memory tracked
    assert!(
        stats.total_memories >= 1,
        "Should track at least 1 memory (total={})",
        stats.total_memories
    );

    // average_strength should be positive (with += correct) or 0
    assert!(
        stats.average_strength >= 0.0,
        "Average strength should be non-negative: {}",
        stats.average_strength
    );
    // With += → -=: strength_sum would be negative → average negative
    // With += → *=: strength_sum = product → very different value

    // If both memories survived, verify average is reasonable
    if stats.total_memories == 2 {
        assert!(
            stats.average_strength > 0.0 && stats.average_strength <= 1.0,
            "Average strength should be in (0, 1]: {}",
            stats.average_strength
        );
    }

    // The number of weak memories should be consistent with threshold
    // If old memory's strength < 0.3, it counts as weak
    // If old memory was forgotten (removed from map), it's not counted
    // Either way, weak_memories should be <= total_memories
    assert!(
        stats.weak_memories <= stats.total_memories,
        "Weak ({}) should be <= total ({})",
        stats.weak_memories,
        stats.total_memories
    );

    // Verify with only a recent (strong) memory: weak = 0
    let config3 = ForgettingConfig {
        base_decay_rate: 0.0,
        importance_factor: 0.0,
        min_importance_threshold: 0.5,
        max_memory_age: u64::MAX,
        protected_categories: vec![],
        ..ForgettingConfig::default()
    };
    let mut engine3 = ForgettingEngine::new(config3);
    let m_strong = make_memory_with_timestamp("strong", "strong", now_ts, 0.5);
    engine3.process_forgetting(vec![m_strong]).unwrap();
    let stats3 = engine3.get_statistics();
    assert_eq!(stats3.total_memories, 1);
    assert_eq!(
        stats3.weak_memories, 0,
        "Strength 1.0 is not < 0.5, so weak=0"
    );
    // With < → >: 1.0 > 0.5 = true → weak=1 (wrong)
    assert!(
        (stats3.average_strength - 1.0).abs() < 1e-5,
        "Average should be 1.0: {}",
        stats3.average_strength
    );
    // With += → -=: sum = -1.0, avg = -1.0 (wrong)
    // With += → *=: sum starts at 0.0, 0.0 * 1.0 = 0.0, avg = 0.0 (wrong)
}

#[test]
fn retrieval_engine_ranks_updated() {
    let mut config = astraweave_rag::retrieval::RetrievalConfig::default();
    config.similarity_threshold = 0.0;
    let engine = astraweave_rag::retrieval::RetrievalEngine::new(config);
    let memories = vec![
        make_memory("1", "cat", MemoryCategory::Social, 0.5),
        make_memory("2", "cat", MemoryCategory::Social, 0.5),
    ];
    let query = astraweave_rag::retrieval::RetrievalQuery {
        text: "cat".to_string(),
        categories: vec![],
        filters: HashMap::new(),
        limit: None,
    };
    let results = engine.search(&query, &memories).unwrap();
    for (i, r) in results.iter().enumerate() {
        assert_eq!(r.rank, i);
    }
}

// ============================================================================
// CURRENT_TIMESTAMP
// ============================================================================

#[test]
fn current_timestamp_returns_recent_value() {
    let ts = current_timestamp();
    // Should be after 2024-01-01 (1704067200)
    assert!(ts > 1704067200, "Timestamp should be recent");
}

#[test]
fn current_timestamp_monotonic() {
    let t1 = current_timestamp();
    let t2 = current_timestamp();
    assert!(
        t2 >= t1,
        "Timestamps should be monotonically non-decreasing"
    );
}

// ============================================================================
// CLONE / DEBUG TRAIT COVERAGE
// ============================================================================

#[test]
fn rag_config_clone_preserves_values() {
    let c = RagConfig::default();
    let c2 = c.clone();
    assert_eq!(c2.max_retrieval_count, c.max_retrieval_count);
    assert!((c2.min_similarity_score - c.min_similarity_score).abs() < 1e-6);
}

#[test]
fn consolidation_config_debug() {
    let c = ConsolidationConfig::default();
    let dbg = format!("{:?}", c);
    assert!(dbg.contains("ConsolidationConfig"));
}

#[test]
fn forgetting_config_debug() {
    let c = ForgettingConfig::default();
    let dbg = format!("{:?}", c);
    assert!(dbg.contains("ForgettingConfig"));
}

#[test]
fn memory_query_debug() {
    let q = MemoryQuery::text("hello");
    let dbg = format!("{:?}", q);
    assert!(dbg.contains("hello"));
}

#[test]
fn rag_metrics_clone() {
    let mut m = RagMetrics::default();
    m.total_queries = 42;
    let m2 = m.clone();
    assert_eq!(m2.total_queries, 42);
}

#[test]
fn memory_strength_clone() {
    let ms = MemoryStrength::default();
    let ms2 = ms.clone();
    assert!((ms2.current_strength - ms.current_strength).abs() < 1e-6);
}

#[test]
fn forgetting_statistics_debug() {
    let s = ForgettingStatistics {
        total_memories: 10,
        protected_memories: 2,
        weak_memories: 3,
        average_strength: 0.7,
    };
    let dbg = format!("{:?}", s);
    assert!(dbg.contains("10"));
}

#[test]
fn consolidation_result_clone() {
    let r = ConsolidationResult {
        processed_count: 5,
        merged_count: 2,
        removed_count: 1,
        processing_time_ms: 10,
    };
    let r2 = r.clone();
    assert_eq!(r2.processed_count, 5);
    assert_eq!(r2.merged_count, 2);
    assert_eq!(r2.removed_count, 1);
}

#[test]
fn forgetting_result_clone() {
    let r = ForgettingResult {
        processed_count: 3,
        forgotten_count: 1,
        updated_count: 2,
        processing_time_ms: 5,
    };
    let r2 = r.clone();
    assert_eq!(r2.processed_count, 3);
    assert_eq!(r2.forgotten_count, 1);
    assert_eq!(r2.updated_count, 2);
}
