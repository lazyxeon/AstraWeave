#![allow(clippy::field_reassign_with_default)]

use astraweave_embeddings::{Memory, MemoryCategory};
use astraweave_rag::consolidation::{
    ConsolidationConfig, ConsolidationEngine, ConsolidationStrategy,
};
use std::collections::HashMap;

fn create_memory(id: &str, text: &str, category: MemoryCategory) -> Memory {
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category,
        timestamp: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        importance: 0.5,
        valence: 0.0,
        entities: vec![],
        context: HashMap::new(),
    }
}

#[test]
fn test_consolidation_merging() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5; // Lower threshold for testing
    let engine = ConsolidationEngine::new(config);

    let memories = vec![
        create_memory("1", "the cat is sleeping", MemoryCategory::Gameplay),
        create_memory("2", "the cat is sleeping", MemoryCategory::Gameplay), // Identical
    ];

    let (consolidated, result) = engine.consolidate(memories).unwrap();

    // Should merge into 1
    assert_eq!(consolidated.len(), 1);
    assert_eq!(result.merged_count, 1);
    assert_eq!(result.removed_count, 1);

    // Check merged content
    assert!(consolidated[0].text.contains("the cat is sleeping"));
}

#[test]
fn test_consolidation_threshold() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.9; // High threshold
    let engine = ConsolidationEngine::new(config);

    let memories = vec![
        create_memory("1", "the cat is sleeping", MemoryCategory::Gameplay),
        create_memory("2", "the dog is barking", MemoryCategory::Gameplay), // Different
    ];

    let (consolidated, result) = engine.consolidate(memories).unwrap();

    // Should NOT merge
    assert_eq!(consolidated.len(), 2);
    assert_eq!(result.merged_count, 0);
}

#[test]
fn test_consolidation_category_mismatch() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.1; // Very low threshold
    let engine = ConsolidationEngine::new(config);

    let memories = vec![
        create_memory("1", "same text", MemoryCategory::Gameplay),
        create_memory("2", "same text", MemoryCategory::Combat), // Different category
    ];

    let (consolidated, result) = engine.consolidate(memories).unwrap();

    // Should NOT merge due to category mismatch
    assert_eq!(consolidated.len(), 2);
    assert_eq!(result.merged_count, 0);
}

#[test]
fn test_consolidation_context_merging() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5;
    let engine = ConsolidationEngine::new(config);

    let mut m1 = create_memory("1", "test memory", MemoryCategory::Gameplay);
    m1.context.insert("key1".to_string(), "value1".to_string());

    let mut m2 = create_memory("2", "test memory", MemoryCategory::Gameplay);
    m2.context.insert("key2".to_string(), "value2".to_string());

    let memories = vec![m1, m2];

    let (consolidated, _) = engine.consolidate(memories).unwrap();

    assert_eq!(consolidated.len(), 1);
    let merged = &consolidated[0];

    assert_eq!(merged.context.get("key1").unwrap(), "value1");
    assert_eq!(merged.context.get("key2").unwrap(), "value2");
}

#[test]
fn test_consolidation_strategies() {
    let strategies = vec![
        ConsolidationStrategy::Importance,
        ConsolidationStrategy::Recency,
        ConsolidationStrategy::Similarity,
        ConsolidationStrategy::Hybrid,
    ];

    for strategy in strategies {
        let mut config = ConsolidationConfig::default();
        config.strategy = strategy;
        config.merge_similarity_threshold = 0.5;

        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_memory("1", "test memory", MemoryCategory::Gameplay),
            create_memory("2", "test memory", MemoryCategory::Gameplay),
        ];

        let (consolidated, _) = engine.consolidate(memories).unwrap();

        // Should merge for all strategies
        assert_eq!(consolidated.len(), 1);
    }
}

#[test]
fn test_similarity_calculation_edge_cases() {
    let config = ConsolidationConfig::default();
    let engine = ConsolidationEngine::new(config);

    // Test empty strings
    let m1 = create_memory("1", "", MemoryCategory::Gameplay);
    let m2 = create_memory("2", "text", MemoryCategory::Gameplay);

    let memories = vec![m1, m2];
    let (consolidated, _) = engine.consolidate(memories).unwrap();

    // Should not merge empty with non-empty
    assert_eq!(consolidated.len(), 2);
}

#[test]
fn test_consolidation_preserves_importance() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5;
    let engine = ConsolidationEngine::new(config);

    let mut m1 = create_memory("1", "test memory", MemoryCategory::Gameplay);
    m1.importance = 0.8;

    let mut m2 = create_memory("2", "test memory", MemoryCategory::Gameplay);
    m2.importance = 0.6;

    let memories = vec![m1, m2];
    let (consolidated, _) = engine.consolidate(memories).unwrap();

    assert_eq!(consolidated.len(), 1);
    // Merged memory should preserve importance from first memory
    assert_eq!(consolidated[0].importance, 0.8);
}

#[test]
fn test_consolidation_timestamp_preservation() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.5;
    let engine = ConsolidationEngine::new(config);

    let mut m1 = create_memory("1", "test memory", MemoryCategory::Gameplay);
    m1.timestamp = 1000;

    let mut m2 = create_memory("2", "test memory", MemoryCategory::Gameplay);
    m2.timestamp = 2000; // More recent

    let memories = vec![m1, m2];
    let (consolidated, _) = engine.consolidate(memories).unwrap();

    assert_eq!(consolidated.len(), 1);
    // Should take the most recent timestamp
    assert_eq!(consolidated[0].timestamp, 2000);
}

#[test]
fn test_consolidation_no_duplicates() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.1; // Very low threshold
    let engine = ConsolidationEngine::new(config);

    let memories = vec![
        create_memory("1", "unique text one", MemoryCategory::Gameplay),
        create_memory("2", "unique text two", MemoryCategory::Gameplay),
        create_memory("3", "unique text three", MemoryCategory::Gameplay),
    ];

    let (consolidated, result) = engine.consolidate(memories).unwrap();

    // All memories have unique IDs in output
    let mut ids = std::collections::HashSet::new();
    for mem in &consolidated {
        assert!(ids.insert(mem.id.clone()), "Duplicate ID found: {}", mem.id);
    }

    assert_eq!(result.processed_count, 3);
}

#[test]
fn test_consolidation_empty_input() {
    let config = ConsolidationConfig::default();
    let engine = ConsolidationEngine::new(config);

    let memories: Vec<Memory> = vec![];
    let (consolidated, result) = engine.consolidate(memories).unwrap();

    assert_eq!(consolidated.len(), 0);
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.merged_count, 0);
}

#[test]
fn test_consolidation_single_memory() {
    let config = ConsolidationConfig::default();
    let engine = ConsolidationEngine::new(config);

    let memories = vec![create_memory(
        "1",
        "single memory",
        MemoryCategory::Gameplay,
    )];
    let (consolidated, result) = engine.consolidate(memories).unwrap();

    assert_eq!(consolidated.len(), 1);
    assert_eq!(result.processed_count, 1);
    assert_eq!(result.merged_count, 0);
}

#[test]
fn test_consolidation_different_categories_no_merge() {
    let mut config = ConsolidationConfig::default();
    config.merge_similarity_threshold = 0.1; // Very low threshold
    let engine = ConsolidationEngine::new(config);

    let memories = vec![
        create_memory("1", "identical text", MemoryCategory::Combat),
        create_memory("2", "identical text", MemoryCategory::Social),
        create_memory("3", "identical text", MemoryCategory::Quest),
    ];

    let (consolidated, result) = engine.consolidate(memories).unwrap();

    // Should not merge due to different categories
    assert_eq!(consolidated.len(), 3);
    assert_eq!(result.merged_count, 0);
}
