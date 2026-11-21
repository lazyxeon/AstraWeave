//! Integration tests for SQLite storage backend

use astraweave_memory::{
    episode::{
        ActionResult, CompanionResponse, Episode as GameEpisode, EpisodeCategory, EpisodeOutcome,
        Observation, PlayerAction,
    },
    Memory, MemoryContent, MemoryMetadata, MemorySource, MemoryStorage, MemoryType,
    SpatialTemporalContext,
};
use std::time::SystemTime;

fn create_test_memory(
    id: &str,
    memory_type: MemoryType,
    importance: f32,
    tags: Vec<String>,
) -> Memory {
    Memory {
        id: id.to_string(),
        memory_type,
        content: MemoryContent {
            text: format!("Test memory content for {}", id),
            data: serde_json::json!({
                "test_data": true,
                "value": 42
            }),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: vec![],
                related_events: vec![],
            },
        },
        metadata: MemoryMetadata {
            created_at: time::OffsetDateTime::now_utc(),
            last_accessed: time::OffsetDateTime::now_utc(),
            access_count: 0,
            importance,
            confidence: 1.0,
            source: MemorySource::DirectExperience,
            tags,
            permanent: false,
            strength: 1.0,
            decay_factor: 1.0,
        },
        associations: vec![],
        embedding: None,
    }
}

#[test]
fn test_storage_initialization() {
    let storage = MemoryStorage::in_memory().expect("Failed to create in-memory storage");
    assert_eq!(storage.count_memories().unwrap(), 0);

    let version = storage.get_schema_version().unwrap();
    assert_eq!(version, "1");
}

#[test]
fn test_basic_crud_operations() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Create
    let memory = create_test_memory(
        "crud_test_001",
        MemoryType::Episodic,
        0.8,
        vec!["test".to_string(), "crud".to_string()],
    );

    storage
        .store_memory(&memory)
        .expect("Failed to store memory");
    assert_eq!(storage.count_memories().unwrap(), 1);

    // Read
    let retrieved = storage
        .get_memory("crud_test_001")
        .expect("Query failed")
        .expect("Memory not found");

    assert_eq!(retrieved.id, "crud_test_001");
    assert_eq!(retrieved.memory_type, MemoryType::Episodic);
    assert!((retrieved.metadata.importance - 0.8).abs() < 0.01);
    assert_eq!(retrieved.metadata.tags.len(), 2);
    assert!(retrieved.metadata.tags.contains(&"test".to_string()));
    assert!(retrieved.metadata.tags.contains(&"crud".to_string()));

    // Update (same ID, different content)
    let mut updated_memory = memory.clone();
    updated_memory.metadata.importance = 0.9;
    updated_memory.content.text = "Updated content".to_string();

    storage
        .store_memory(&updated_memory)
        .expect("Failed to update");
    assert_eq!(storage.count_memories().unwrap(), 1); // Still 1 memory

    let updated_retrieved = storage.get_memory("crud_test_001").unwrap().unwrap();
    assert!((updated_retrieved.metadata.importance - 0.9).abs() < 0.01);
    assert_eq!(updated_retrieved.content.text, "Updated content");

    // Delete
    let deleted = storage.delete_memory("crud_test_001").unwrap();
    assert!(deleted);
    assert_eq!(storage.count_memories().unwrap(), 0);

    let after_delete = storage.get_memory("crud_test_001").unwrap();
    assert!(after_delete.is_none());
}

#[test]
fn test_query_by_type() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Store different types
    storage
        .store_memory(&create_test_memory(
            "ep1",
            MemoryType::Episodic,
            0.8,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "ep2",
            MemoryType::Episodic,
            0.7,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "sem1",
            MemoryType::Semantic,
            0.9,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "proc1",
            MemoryType::Procedural,
            0.6,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "emo1",
            MemoryType::Emotional,
            0.85,
            vec![],
        ))
        .unwrap();

    // Query episodic
    let episodic = storage.query_by_type(MemoryType::Episodic).unwrap();
    assert_eq!(episodic.len(), 2);
    assert!(episodic.contains(&"ep1".to_string()));
    assert!(episodic.contains(&"ep2".to_string()));

    // Query semantic
    let semantic = storage.query_by_type(MemoryType::Semantic).unwrap();
    assert_eq!(semantic.len(), 1);
    assert_eq!(semantic[0], "sem1");

    // Count by type
    assert_eq!(storage.count_by_type(MemoryType::Episodic).unwrap(), 2);
    assert_eq!(storage.count_by_type(MemoryType::Semantic).unwrap(), 1);
    assert_eq!(storage.count_by_type(MemoryType::Procedural).unwrap(), 1);
    assert_eq!(storage.count_by_type(MemoryType::Emotional).unwrap(), 1);
}

#[test]
fn test_query_by_tag() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    storage
        .store_memory(&create_test_memory(
            "combat1",
            MemoryType::Episodic,
            0.8,
            vec!["combat".to_string(), "boss".to_string()],
        ))
        .unwrap();

    storage
        .store_memory(&create_test_memory(
            "combat2",
            MemoryType::Episodic,
            0.7,
            vec!["combat".to_string(), "arena".to_string()],
        ))
        .unwrap();

    storage
        .store_memory(&create_test_memory(
            "dialogue1",
            MemoryType::Episodic,
            0.6,
            vec!["dialogue".to_string(), "npc".to_string()],
        ))
        .unwrap();

    // Query by combat tag
    let combat_memories = storage.query_by_tag("combat").unwrap();
    assert_eq!(combat_memories.len(), 2);
    assert!(combat_memories.contains(&"combat1".to_string()));
    assert!(combat_memories.contains(&"combat2".to_string()));

    // Query by boss tag
    let boss_memories = storage.query_by_tag("boss").unwrap();
    assert_eq!(boss_memories.len(), 1);
    assert_eq!(boss_memories[0], "combat1");

    // Get all tags
    let all_tags = storage.get_all_tags().unwrap();
    assert_eq!(all_tags.len(), 5);
    assert!(all_tags.contains(&"combat".to_string()));
    assert!(all_tags.contains(&"boss".to_string()));
    assert!(all_tags.contains(&"arena".to_string()));
    assert!(all_tags.contains(&"dialogue".to_string()));
    assert!(all_tags.contains(&"npc".to_string()));
}

#[test]
fn test_query_recent() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Add memories with slight delays (in real usage they'd have different timestamps)
    storage
        .store_memory(&create_test_memory(
            "old1",
            MemoryType::Episodic,
            0.5,
            vec![],
        ))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    storage
        .store_memory(&create_test_memory(
            "mid1",
            MemoryType::Episodic,
            0.6,
            vec![],
        ))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    storage
        .store_memory(&create_test_memory(
            "new1",
            MemoryType::Episodic,
            0.7,
            vec![],
        ))
        .unwrap();

    // Query 2 most recent
    let recent = storage.query_recent(2).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0], "new1"); // Most recent first
    assert_eq!(recent[1], "mid1");
}

#[test]
fn test_query_important() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    storage
        .store_memory(&create_test_memory(
            "low",
            MemoryType::Episodic,
            0.2,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "medium",
            MemoryType::Episodic,
            0.5,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "high",
            MemoryType::Episodic,
            0.9,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "critical",
            MemoryType::Episodic,
            1.0,
            vec![],
        ))
        .unwrap();

    // Query importance >= 0.7
    let important = storage.query_important(0.7, 10).unwrap();
    assert_eq!(important.len(), 2);
    assert!(important.contains(&"high".to_string()));
    assert!(important.contains(&"critical".to_string()));

    // Query top 1
    let top_one = storage.query_important(0.0, 1).unwrap();
    assert_eq!(top_one.len(), 1);
    assert_eq!(top_one[0], "critical");
}

#[test]
fn test_query_by_type_and_importance() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    storage
        .store_memory(&create_test_memory(
            "ep_low",
            MemoryType::Episodic,
            0.3,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "ep_high",
            MemoryType::Episodic,
            0.9,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "sem_high",
            MemoryType::Semantic,
            0.95,
            vec![],
        ))
        .unwrap();

    // Query episodic with importance >= 0.5
    let results = storage
        .query_by_type_and_importance(MemoryType::Episodic, 0.5, 10)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], "ep_high");

    // Query semantic with importance >= 0.9
    let sem_results = storage
        .query_by_type_and_importance(MemoryType::Semantic, 0.9, 10)
        .unwrap();
    assert_eq!(sem_results.len(), 1);
    assert_eq!(sem_results[0], "sem_high");
}

#[test]
fn test_prune_operations() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    storage
        .store_memory(&create_test_memory(
            "keep1",
            MemoryType::Episodic,
            0.8,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "prune1",
            MemoryType::Episodic,
            0.2,
            vec![],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "prune2",
            MemoryType::Episodic,
            0.3,
            vec![],
        ))
        .unwrap();

    assert_eq!(storage.count_memories().unwrap(), 3);

    // Prune unimportant (< 0.5)
    let pruned = storage.prune_unimportant(0.5).unwrap();
    assert_eq!(pruned, 2); // Pruned 2 memories
    assert_eq!(storage.count_memories().unwrap(), 1);

    // Verify kept memory is the important one
    let kept = storage.get_memory("keep1").unwrap();
    assert!(kept.is_some());
}

#[test]
fn test_episode_to_memory_storage() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Create episode with proper constructor
    let mut episode = GameEpisode::new("combat_episode_001".to_string(), EpisodeCategory::Combat);

    // Add observations with proper structure
    episode.add_observation(Observation::new(
        0,
        Some(PlayerAction {
            action_type: "melee_attack".to_string(),
            target: Some("enemy_goblin".to_string()),
            parameters: serde_json::json!({
                "damage": 50.0,
                "critical": true
            }),
        }),
        Some(CompanionResponse {
            action_type: "support_spell".to_string(),
            result: ActionResult::Success,
            effectiveness: 0.9,
        }),
        serde_json::json!({
            "player_health": 100.0,
            "companion_health": 95.0,
            "enemy_count": 3
        }),
    ));

    // End episode
    episode.end_time = Some(SystemTime::now());
    episode.tags = vec!["combat".to_string(), "goblin".to_string()];
    episode.outcome = Some(EpisodeOutcome {
        success_rating: 0.9,
        player_satisfaction: 0.85,
        companion_effectiveness: 0.88,
        duration_ms: 15000,
        damage_dealt: 450.0,
        damage_taken: 75.0,
        resources_used: 120.0,
        failure_count: 0,
    });

    // Convert to Memory and store
    let memory = episode
        .to_memory()
        .expect("Failed to convert episode to memory");
    storage
        .store_memory(&memory)
        .expect("Failed to store episode memory");

    // Retrieve and validate
    let retrieved = storage
        .get_memory(&episode.id)
        .expect("Query failed")
        .expect("Memory not found");

    assert_eq!(retrieved.memory_type, MemoryType::Episodic);
    assert_eq!(retrieved.metadata.tags.len(), 2);
    assert!(retrieved.metadata.tags.contains(&"combat".to_string()));

    // Check importance (should be high due to good outcome)
    assert!(retrieved.metadata.importance > 0.7);
}

#[test]
fn test_multiple_episode_storage() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Create combat episode
    let mut combat_ep = GameEpisode::new("ep_combat_001".to_string(), EpisodeCategory::Combat);
    combat_ep.tags = vec!["combat".to_string()];
    combat_ep.outcome = Some(EpisodeOutcome {
        success_rating: 0.9,
        player_satisfaction: 0.85,
        companion_effectiveness: 0.82,
        duration_ms: 10000,
        damage_dealt: 450.0,
        damage_taken: 75.0,
        resources_used: 120.0,
        failure_count: 0,
    });

    // Create dialogue episode
    let mut dialogue_ep =
        GameEpisode::new("ep_dialogue_001".to_string(), EpisodeCategory::Dialogue);
    dialogue_ep.tags = vec!["dialogue".to_string(), "npc".to_string()];
    dialogue_ep.outcome = Some(EpisodeOutcome {
        success_rating: 0.8,
        player_satisfaction: 0.9,
        companion_effectiveness: 0.75,
        duration_ms: 5000,
        damage_dealt: 0.0,
        damage_taken: 0.0,
        resources_used: 5.0,
        failure_count: 0,
    });

    // Create exploration episode
    let mut exploration_ep = GameEpisode::new(
        "ep_exploration_001".to_string(),
        EpisodeCategory::Exploration,
    );
    exploration_ep.tags = vec!["exploration".to_string(), "discovery".to_string()];
    exploration_ep.outcome = Some(EpisodeOutcome {
        success_rating: 0.95,
        player_satisfaction: 0.92,
        companion_effectiveness: 0.88,
        duration_ms: 20000,
        damage_dealt: 0.0,
        damage_taken: 0.0,
        resources_used: 10.0,
        failure_count: 0,
    });

    // Store all episodes
    storage
        .store_memory(&combat_ep.to_memory().unwrap())
        .unwrap();
    storage
        .store_memory(&dialogue_ep.to_memory().unwrap())
        .unwrap();
    storage
        .store_memory(&exploration_ep.to_memory().unwrap())
        .unwrap();

    // Verify count
    assert_eq!(storage.count_memories().unwrap(), 3);
    assert_eq!(storage.count_by_type(MemoryType::Episodic).unwrap(), 3);

    // Query by tags
    let combat_memories = storage.query_by_tag("combat").unwrap();
    assert_eq!(combat_memories.len(), 1);

    let dialogue_memories = storage.query_by_tag("dialogue").unwrap();
    assert_eq!(dialogue_memories.len(), 1);

    // Query important episodes
    let important = storage.query_important(0.8, 10).unwrap();
    assert!(!important.is_empty()); // At least exploration episode
}

#[test]
fn test_storage_stats() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    storage
        .store_memory(&create_test_memory(
            "ep1",
            MemoryType::Episodic,
            0.8,
            vec!["combat".to_string()],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "ep2",
            MemoryType::Episodic,
            0.7,
            vec!["dialogue".to_string()],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "sem1",
            MemoryType::Semantic,
            0.9,
            vec!["knowledge".to_string()],
        ))
        .unwrap();
    storage
        .store_memory(&create_test_memory(
            "proc1",
            MemoryType::Procedural,
            0.6,
            vec!["skill".to_string()],
        ))
        .unwrap();

    let stats = storage.get_stats().expect("Failed to get stats");

    assert_eq!(stats.total_memories, 4);
    assert_eq!(stats.episodic_count, 2);
    assert_eq!(stats.semantic_count, 1);
    assert_eq!(stats.procedural_count, 1);
    assert_eq!(stats.total_tags, 4);
    assert!(stats.db_size_bytes > 0);
}

#[test]
fn test_persistence_across_instances() {
    // Use temp file for this test
    let temp_path = std::env::temp_dir().join("astraweave_storage_test.db");

    // Cleanup if exists
    let _ = std::fs::remove_file(&temp_path);

    // Create first instance and store data
    {
        let mut storage = MemoryStorage::new(&temp_path).expect("Failed to create storage");
        storage
            .store_memory(&create_test_memory(
                "persist1",
                MemoryType::Episodic,
                0.8,
                vec![],
            ))
            .unwrap();
        storage
            .store_memory(&create_test_memory(
                "persist2",
                MemoryType::Semantic,
                0.9,
                vec![],
            ))
            .unwrap();
        assert_eq!(storage.count_memories().unwrap(), 2);
        // storage dropped here
    }

    // Create second instance and verify data persists
    {
        let storage = MemoryStorage::new(&temp_path).expect("Failed to reopen storage");
        assert_eq!(storage.count_memories().unwrap(), 2);

        let mem1 = storage.get_memory("persist1").unwrap();
        assert!(mem1.is_some());

        let mem2 = storage.get_memory("persist2").unwrap();
        assert!(mem2.is_some());
    }

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn test_embedding_storage() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    let mut memory = create_test_memory("with_embedding", MemoryType::Semantic, 0.8, vec![]);
    memory.embedding = Some(vec![0.1, 0.2, 0.3, 0.4, 0.5]);

    storage.store_memory(&memory).unwrap();

    let retrieved = storage.get_memory("with_embedding").unwrap().unwrap();
    assert!(retrieved.embedding.is_some());

    let embedding = retrieved.embedding.unwrap();
    assert_eq!(embedding.len(), 5);
    assert!((embedding[0] - 0.1).abs() < 0.001);
    assert!((embedding[4] - 0.5).abs() < 0.001);
}

#[test]
fn test_optimize_operation() {
    let mut storage = MemoryStorage::in_memory().unwrap();

    // Add some data
    for i in 0..10 {
        storage
            .store_memory(&create_test_memory(
                &format!("opt_{}", i),
                MemoryType::Episodic,
                0.5,
                vec![],
            ))
            .unwrap();
    }

    // Optimize should not fail
    storage.optimize().expect("Optimize failed");

    // Data should still be accessible
    assert_eq!(storage.count_memories().unwrap(), 10);
}
