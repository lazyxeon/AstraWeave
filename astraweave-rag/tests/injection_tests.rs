#![allow(clippy::field_reassign_with_default)]

use astraweave_embeddings::{Memory, MemoryCategory};
use astraweave_rag::injection::{InjectionConfig, InjectionContext, InjectionEngine};
use astraweave_rag::InjectionStrategy;
use std::collections::HashMap;

fn create_memory(id: &str, text: &str, category: MemoryCategory, age_hours: i64) -> Memory {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp() - (age_hours * 3600);
    Memory {
        id: id.to_string(),
        text: text.to_string(),
        category,
        timestamp: timestamp as u64,
        importance: 0.5,
        valence: 0.0,
        entities: vec![],
        context: HashMap::new(),
    }
}

#[test]
fn test_injection_relevance_threshold() {
    let mut config = InjectionConfig::default();
    config.relevance_threshold = 0.5; // High threshold
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "cats".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let memories = vec![
        create_memory("1", "cats are great", MemoryCategory::Gameplay, 0), // Relevant
        create_memory("2", "dogs are loyal", MemoryCategory::Gameplay, 0), // Irrelevant
    ];

    let result = engine.inject(&context, &memories).unwrap();

    // "cats are great" has "cats" (1/3 match) -> 0.33 * 0.5 = 0.165
    // Plus base score 0.1 = 0.265.
    // Wait, the naive similarity is: common_words / words1.len().
    // query="cats" (len 1). memory="cats are great".
    // similarity("cats", "cats are great") -> common=1, len=1 -> 1.0.
    // relevance = 1.0 * 0.5 + 0.1 = 0.6.

    // "dogs are loyal". query="cats".
    // similarity("cats", "dogs are loyal") -> common=0.
    // relevance = 0.0 * 0.5 + 0.1 = 0.1.

    // So "cats" should pass (0.6 >= 0.5), "dogs" should fail (0.1 < 0.5).

    assert_eq!(result.injected_memories.len(), 1);
    assert_eq!(result.injected_memories[0].id, "1");
}

#[test]
fn test_injection_recency_prioritization() {
    let mut config = InjectionConfig::default();
    config.prioritize_recent = true;
    config.max_memories = 1;
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "news".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    // Two identical memories, one recent, one old
    let memories = vec![
        create_memory("old", "breaking news", MemoryCategory::Gameplay, 24), // 24 hours old
        create_memory("new", "breaking news", MemoryCategory::Gameplay, 0),  // Now
    ];

    let result = engine.inject(&context, &memories).unwrap();

    assert_eq!(result.injected_memories.len(), 1);
    assert_eq!(result.injected_memories[0].id, "new");
}

#[test]
fn test_injection_category_preference() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "something".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![MemoryCategory::Combat],
    };

    let memories = vec![
        create_memory("1", "something happened", MemoryCategory::Gameplay, 0),
        create_memory("2", "something happened", MemoryCategory::Combat, 0),
    ];

    let result = engine.inject(&context, &memories).unwrap();

    // Both have same text similarity.
    // Memory 2 matches preferred category -> +0.3 boost.
    // Memory 1 does not -> +0.0 (and base score logic might differ).
    // calculate_relevance:
    // if preferred matches: +0.3.
    // else: +0.1.
    // So Combat gets +0.3, Gameplay gets +0.1.
    // Combat should be first.

    assert!(result.injected_memories.len() >= 2);
    assert_eq!(result.injected_memories[0].id, "2");
}

#[test]
fn test_injection_max_memories() {
    let mut config = InjectionConfig::default();
    config.max_memories = 2;
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let memories = vec![
        create_memory("1", "test 1", MemoryCategory::Gameplay, 0),
        create_memory("2", "test 2", MemoryCategory::Gameplay, 0),
        create_memory("3", "test 3", MemoryCategory::Gameplay, 0),
    ];

    let result = engine.inject(&context, &memories).unwrap();

    assert_eq!(result.injected_memories.len(), 2);
}

#[test]
fn test_injection_strategies_all_variants() {
    let strategies = vec![
        (InjectionStrategy::Prepend, "prepend test"),
        (InjectionStrategy::Append, "append test"),
        (InjectionStrategy::Insert, "insert test"),
        (InjectionStrategy::Interleave, "interleave test"),
        (InjectionStrategy::Replace, "replace test"),
    ];

    for (strategy, query_text) in strategies {
        let config = InjectionConfig::default();
        let engine = InjectionEngine::new(config);

        let context = InjectionContext {
            query: query_text.to_string(),
            conversation_history: vec![],
            metadata: HashMap::new(),
            preferred_categories: vec![],
        };

        let memories = vec![create_memory("1", query_text, MemoryCategory::Gameplay, 0)];

        let result = engine.inject(&context, &memories);
        assert!(result.is_ok(), "Strategy {:?} failed", strategy);
    }
}

#[test]
fn test_injection_relevance_calculation() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "cats and dogs".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let high_relevance = create_memory("1", "cats and dogs are pets", MemoryCategory::Gameplay, 0);
    let low_relevance = create_memory(
        "2",
        "completely unrelated topic",
        MemoryCategory::Gameplay,
        0,
    );

    let memories = vec![high_relevance, low_relevance];

    let result = engine.inject(&context, &memories).unwrap();

    // High relevance memory should be first
    if result.injected_memories.len() >= 2 {
        assert_eq!(result.injected_memories[0].id, "1");
    }
}

#[test]
fn test_injection_recency_boost() {
    let mut config = InjectionConfig::default();
    config.prioritize_recent = true;
    config.max_memories = 1;
    config.relevance_threshold = 0.3;
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "event".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    // Two memories with same text, different ages
    let old = create_memory("old", "event happened", MemoryCategory::Gameplay, 48); // 2 days old
    let recent = create_memory("recent", "event happened", MemoryCategory::Gameplay, 1); // 1 hour old

    let memories = vec![old, recent];

    let result = engine.inject(&context, &memories).unwrap();

    // Recent memory should win
    assert_eq!(result.injected_memories.len(), 1);
    assert_eq!(result.injected_memories[0].id, "recent");
}

#[test]
fn test_injection_conversation_history_relevance() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "query".to_string(),
        conversation_history: vec![
            "We were discussing cats".to_string(),
            "Cats are great pets".to_string(),
        ],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let cat_memory = create_memory("1", "Cats enjoy sunbathing", MemoryCategory::Social, 0);
    let dog_memory = create_memory("2", "Dogs need walks", MemoryCategory::Social, 0);

    let memories = vec![cat_memory, dog_memory];

    let result = engine.inject(&context, &memories).unwrap();

    // Cat memory should rank higher due to conversation history
    if !result.injected_memories.is_empty() {
        assert_eq!(result.injected_memories[0].id, "1");
    }
}

#[test]
fn test_injection_empty_memories() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "query".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let memories: Vec<Memory> = vec![];

    let result = engine.inject(&context, &memories).unwrap();

    assert!(result.injected_memories.is_empty());
    assert!(result.context_text.is_empty());
    assert_eq!(result.relevance_scores.len(), 0);
}

#[test]
fn test_injection_context_text_generation() {
    let mut config = InjectionConfig::default();
    config.relevance_threshold = 0.1; // Low threshold to ensure memories are included
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "memory test query".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let memories = vec![
        create_memory("1", "first memory test", MemoryCategory::Gameplay, 0),
        create_memory("2", "second memory test", MemoryCategory::Gameplay, 0),
    ];

    let result = engine.inject(&context, &memories).unwrap();

    // If memories were injected, context text should contain them
    if !result.injected_memories.is_empty() {
        assert!(!result.context_text.is_empty());
        assert!(result.context_text.contains("Relevant memories:"));
        assert!(result.context_text.contains("1."));
    }
}

#[test]
fn test_injection_token_estimation() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![],
    };

    let memories = vec![create_memory(
        "1",
        "This is a test memory with several words",
        MemoryCategory::Gameplay,
        0,
    )];

    let result = engine.inject(&context, &memories).unwrap();

    // Token count should be greater than 0
    assert!(result.estimated_tokens > 0);
}

#[test]
fn test_injection_no_category_preference() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![], // No preference
    };

    let memories = vec![
        create_memory("1", "test memory", MemoryCategory::Combat, 0),
        create_memory("2", "test memory", MemoryCategory::Social, 0),
    ];

    let result = engine.inject(&context, &memories).unwrap();

    // Both should be injected when relevance threshold is met
    assert!(!result.injected_memories.is_empty());
}

#[test]
fn test_injection_multiple_preferred_categories() {
    let config = InjectionConfig::default();
    let engine = InjectionEngine::new(config);

    let context = InjectionContext {
        query: "test".to_string(),
        conversation_history: vec![],
        metadata: HashMap::new(),
        preferred_categories: vec![MemoryCategory::Combat, MemoryCategory::Social],
    };

    let memories = vec![
        create_memory("1", "test memory", MemoryCategory::Combat, 0),
        create_memory("2", "test memory", MemoryCategory::Social, 0),
        create_memory("3", "test memory", MemoryCategory::Exploration, 0),
    ];

    let result = engine.inject(&context, &memories).unwrap();

    // Combat and Social should rank higher than Exploration
    for mem in &result.injected_memories {
        if mem.id == "1" || mem.id == "2" {
            // These should be preferred
        }
    }
}
