//! Comprehensive save/load tests for astraweave-persistence-ecs
//!
//! Tests roundtrip validation, large world serialization, concurrent access,
//! and partial world saving/loading.

#![allow(clippy::field_reassign_with_default)]
#![allow(unused_mut)]

use astraweave_core::ecs_components::*;
use astraweave_core::IVec2;
use astraweave_ecs::{Query, World};
use astraweave_persistence_ecs::{
    calculate_world_hash, deserialize_ecs_world, serialize_ecs_world, CPersistenceManager,
};
use aw_save::SaveManager;
use std::collections::HashMap;
use tempfile::tempdir;

// ========== Roundtrip Validation Tests ==========

#[test]
fn test_save_load_roundtrip_basic() {
    // Create world with basic entities
    let mut world = World::new();

    let e1 = world.spawn();
    world.insert(
        e1,
        CPos {
            pos: IVec2 { x: 10, y: 20 },
        },
    );
    world.insert(e1, CHealth { hp: 100 });
    world.insert(e1, CTeam { id: 1 });

    let e2 = world.spawn();
    world.insert(
        e2,
        CPos {
            pos: IVec2 { x: 30, y: 40 },
        },
    );
    world.insert(e2, CHealth { hp: 50 });
    world.insert(e2, CAmmo { rounds: 120 });

    // Serialize
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    assert!(!blob.is_empty(), "serialized blob should not be empty");

    // Deserialize into new world
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    // Verify entity counts match
    let mut pos_count = 0;
    let mut health_count = 0;
    let mut team_count = 0;
    let mut ammo_count = 0;

    {
        let q = Query::<CPos>::new(&world2);
        for (_, pos) in q {
            pos_count += 1;
            assert!(
                (pos.pos.x == 10 && pos.pos.y == 20) || (pos.pos.x == 30 && pos.pos.y == 40),
                "position mismatch"
            );
        }
    }
    {
        let q = Query::<CHealth>::new(&world2);
        for (_, health) in q {
            health_count += 1;
            assert!(health.hp == 100 || health.hp == 50, "health value mismatch");
        }
    }
    {
        let q = Query::<CTeam>::new(&world2);
        for (_, team) in q {
            team_count += 1;
            assert_eq!(team.id, 1, "team id mismatch");
        }
    }
    {
        let q = Query::<CAmmo>::new(&world2);
        for (_, ammo) in q {
            ammo_count += 1;
            assert_eq!(ammo.rounds, 120, "ammo count mismatch");
        }
    }

    assert_eq!(pos_count, 2, "position component count mismatch");
    assert_eq!(health_count, 2, "health component count mismatch");
    assert_eq!(team_count, 1, "team component count mismatch");
    assert_eq!(ammo_count, 1, "ammo component count mismatch");
}

#[test]
fn test_save_load_roundtrip_all_components() {
    // Test with all supported component types
    let mut world = World::new();

    let entity = world.spawn();
    world.insert(
        entity,
        CPos {
            pos: IVec2 { x: 5, y: 10 },
        },
    );
    world.insert(entity, CHealth { hp: 75 });
    world.insert(entity, CTeam { id: 2 });
    world.insert(entity, CAmmo { rounds: 30 });
    world.insert(entity, CCooldowns::default());
    world.insert(
        entity,
        CDesiredPos {
            pos: IVec2 { x: 15, y: 25 },
        },
    );
    world.insert(entity, CAiAgent);
    world.insert(
        entity,
        CPersona {
            profile: astraweave_core::ecs_components::CompanionProfile {
                name: "warrior".to_string(),
                personality_traits: vec![],
                background: String::new(),
            },
        },
    );

    // Serialize and deserialize
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    // Verify all components were restored
    let mut entity_count = 0;
    {
        let q = Query::<CPos>::new(&world2);
        for (e, pos) in q {
            entity_count += 1;
            assert_eq!(pos.pos.x, 5);
            assert_eq!(pos.pos.y, 10);

            // Verify other components on same entity
            assert!(world2.get::<CHealth>(e).is_some());
            assert!(world2.get::<CTeam>(e).is_some());
            assert!(world2.get::<CAmmo>(e).is_some());
            assert!(world2.get::<CCooldowns>(e).is_some());
            assert!(world2.get::<CDesiredPos>(e).is_some());
            assert!(world2.get::<CAiAgent>(e).is_some());
            assert!(world2.get::<CPersona>(e).is_some());
        }
    }
    assert_eq!(entity_count, 1, "should have exactly 1 entity");
}

#[test]
fn test_save_load_entity_ids_preserved_semantically() {
    // Entity IDs are remapped, but relationships should be preserved
    let mut world = World::new();

    let e1 = world.spawn();
    world.insert(
        e1,
        CPos {
            pos: IVec2 { x: 1, y: 1 },
        },
    );
    world.insert(e1, CHealth { hp: 100 });

    let e2 = world.spawn();
    world.insert(
        e2,
        CPos {
            pos: IVec2 { x: 2, y: 2 },
        },
    );
    world.insert(e2, CHealth { hp: 90 });

    let e3 = world.spawn();
    world.insert(
        e3,
        CPos {
            pos: IVec2 { x: 3, y: 3 },
        },
    );
    world.insert(e3, CHealth { hp: 80 });

    // Serialize and deserialize
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    // Verify the same number of entities exist with correct data
    let mut entities_by_pos = HashMap::new();
    {
        let q = Query::<CPos>::new(&world2);
        for (e, pos) in q {
            entities_by_pos.insert((pos.pos.x, pos.pos.y), e);
        }
    }

    assert_eq!(entities_by_pos.len(), 3, "should have 3 entities");

    // Verify health values match positions
    let e1_new = entities_by_pos[&(1, 1)];
    let e2_new = entities_by_pos[&(2, 2)];
    let e3_new = entities_by_pos[&(3, 3)];

    assert_eq!(world2.get::<CHealth>(e1_new).unwrap().hp, 100);
    assert_eq!(world2.get::<CHealth>(e2_new).unwrap().hp, 90);
    assert_eq!(world2.get::<CHealth>(e3_new).unwrap().hp, 80);
}

#[test]
fn test_save_load_empty_world() {
    // Empty world should serialize and deserialize without errors
    let world = World::new();

    let blob = serialize_ecs_world(&world).expect("serialize failed");
    assert!(!blob.is_empty(), "empty world still produces blob");

    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    // Verify world is still empty
    let mut q = Query::<CPos>::new(&world2);
    assert!(q.next().is_none(), "world should be empty");
}

// ========== Large World Serialization Tests ==========

/// Performance test for serializing 1,000 entities
/// Uses CI-friendly thresholds (50ms) to account for debug builds and system load
#[test]
fn test_large_world_1000_entities() {
    // Test performance and correctness with 1,000 entities
    let mut world = World::new();

    for i in 0..1000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(e, CHealth { hp: 50 + (i % 100) });
        world.insert(e, CTeam { id: (i % 4) as u8 });
    }

    // Serialize
    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let serialize_duration = start.elapsed();

    println!("Serialized 1,000 entities in {:?}", serialize_duration);
    println!(
        "Blob size: {} bytes ({:.2} KB)",
        blob.len(),
        blob.len() as f64 / 1024.0
    );

    // Verify blob size is reasonable (target: <100KB for 1,000 entities)
    assert!(blob.len() < 100_000, "blob size should be under 100KB");

    // Deserialize
    let start = std::time::Instant::now();
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");
    let deserialize_duration = start.elapsed();

    println!("Deserialized 1,000 entities in {:?}", deserialize_duration);

    // Verify entity count
    let mut count = 0;
    {
        let mut q = Query::<CPos>::new(&world2);
        while q.next().is_some() {
            count += 1;
        }
    }
    assert_eq!(count, 1000, "should have 1,000 entities after deserialize");

    // CI-friendly performance targets (debug builds are 5-10x slower than release)
    // Release typically achieves ~2-5ms, debug ~10-25ms, CI ~25-50ms
    assert!(
        serialize_duration.as_millis() < 50,
        "serialize should be under 50ms in debug mode, got {}ms",
        serialize_duration.as_millis()
    );
    assert!(
        deserialize_duration.as_millis() < 50,
        "deserialize should be under 50ms in debug mode, got {}ms",
        deserialize_duration.as_millis()
    );
}

#[test]
fn test_large_world_10000_entities() {
    // Test with 10,000 entities
    let mut world = World::new();

    for i in 0..10_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(e, CHealth { hp: 100 });

        // Add some variety
        if i % 2 == 0 {
            world.insert(e, CAmmo { rounds: 30 });
        }
        if i % 3 == 0 {
            world.insert(e, CTeam { id: 1 });
        }
    }

    // Serialize
    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let serialize_duration = start.elapsed();

    println!("Serialized 10,000 entities in {:?}", serialize_duration);
    println!(
        "Blob size: {} bytes ({:.2} KB)",
        blob.len(),
        blob.len() as f64 / 1024.0
    );

    // Verify blob size is reasonable (target: <1MB for 10,000 entities)
    assert!(blob.len() < 1_000_000, "blob size should be under 1MB");

    // Deserialize
    let start = std::time::Instant::now();
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");
    let deserialize_duration = start.elapsed();

    println!("Deserialized 10,000 entities in {:?}", deserialize_duration);

    // Verify entity count
    let mut count = 0;
    {
        let mut q = Query::<CPos>::new(&world2);
        while q.next().is_some() {
            count += 1;
        }
    }
    assert_eq!(
        count, 10_000,
        "should have 10,000 entities after deserialize"
    );

    // Verify load time < 5 seconds
    assert!(
        deserialize_duration.as_secs() < 5,
        "load should be under 5 seconds"
    );
}

#[test]
fn test_persistence_manager_save_load_integration() {
    // Test full integration with SaveManager
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "test_player".to_string(),
    };

    // Create test world
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: 42, y: 84 },
        },
    );
    world.insert(e, CHealth { hp: 150 });

    // Serialize world
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let hash = calculate_world_hash(&world);

    // Save game
    let save_path = persistence
        .save_game(0, 100, hash, blob.clone())
        .expect("save failed");

    assert!(save_path.exists(), "save file should exist");

    // Load game
    let (bundle, loaded_path) = persistence.load_game(0).expect("load failed");

    assert_eq!(bundle.slot, 0);
    assert_eq!(bundle.player_id, "test_player");
    assert_eq!(bundle.world.tick, 100);
    assert_eq!(bundle.world.state_hash, hash);
    assert_eq!(loaded_path, save_path);

    // Deserialize and verify
    let mut world2 = World::new();
    deserialize_ecs_world(&bundle.world.ecs_blob, &mut world2).expect("deserialize failed");

    let mut q = Query::<CPos>::new(&world2);
    let (_, pos) = q.next().expect("should have entity");
    assert_eq!(pos.pos.x, 42);
    assert_eq!(pos.pos.y, 84);
}

#[test]
fn test_partial_world_saving() {
    // Create world with mixed entities
    let mut world = World::new();

    // Player entities (with CTeam { id: 0 })
    for i in 0..10 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
        world.insert(e, CTeam { id: 0 });
    }

    // Enemy entities (with CTeam { id: 1 })
    for i in 10..20 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 50 });
        world.insert(e, CTeam { id: 1 });
    }

    // For partial saving, we'd need to create a filtered world
    // This demonstrates the concept
    let mut filtered_world = World::new();

    // Copy only player entities
    {
        let q = Query::<CTeam>::new(&world);
        for (entity, team) in q {
            if team.id == 0 {
                // Player team
                let new_entity = filtered_world.spawn();
                if let Some(pos) = world.get::<CPos>(entity) {
                    filtered_world.insert(new_entity, *pos);
                }
                if let Some(health) = world.get::<CHealth>(entity) {
                    filtered_world.insert(new_entity, *health);
                }
                filtered_world.insert(new_entity, *team);
            }
        }
    }

    // Serialize filtered world
    let blob = serialize_ecs_world(&filtered_world).expect("serialize failed");

    // Deserialize and verify only player entities present
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    let mut count = 0;
    {
        let q = Query::<CTeam>::new(&world2);
        for (_, team) in q {
            count += 1;
            assert_eq!(team.id, 0, "should only have player team entities");
        }
    }

    assert_eq!(count, 10, "should have exactly 10 player entities");
}

#[test]
fn test_multiple_save_slots() {
    // Test saving to multiple slots
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "multi_slot_player".to_string(),
    };

    // Create different worlds for each slot
    for slot in 0..3 {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 {
                    x: slot as i32 * 10,
                    y: slot as i32 * 10,
                },
            },
        );
        world.insert(
            e,
            CHealth {
                hp: 100 + slot as i32 * 10,
            },
        );

        let blob = serialize_ecs_world(&world).expect("serialize failed");
        let hash = calculate_world_hash(&world);

        persistence
            .save_game(slot, slot as u64 * 100, hash, blob)
            .expect("save failed");
    }

    // Load and verify each slot
    for slot in 0..3 {
        let (bundle, _) = persistence.load_game(slot).expect("load failed");

        assert_eq!(bundle.slot, slot);
        assert_eq!(bundle.world.tick, slot as u64 * 100);

        let mut world = World::new();
        deserialize_ecs_world(&bundle.world.ecs_blob, &mut world).expect("deserialize failed");

        let mut q = Query::<CPos>::new(&world);
        let (_, pos) = q.next().expect("should have entity");
        assert_eq!(pos.pos.x, slot as i32 * 10);
        assert_eq!(pos.pos.y, slot as i32 * 10);
    }
}

#[test]
fn test_world_hash_changes_on_modification() {
    // Verify hash changes when world state changes
    let mut world = World::new();

    let e1 = world.spawn();
    world.insert(
        e1,
        CPos {
            pos: IVec2 { x: 10, y: 20 },
        },
    );
    world.insert(e1, CHealth { hp: 100 });

    let hash1 = calculate_world_hash(&world);

    // Modify world
    let e2 = world.spawn();
    world.insert(
        e2,
        CPos {
            pos: IVec2 { x: 30, y: 40 },
        },
    );

    let hash2 = calculate_world_hash(&world);

    assert_ne!(hash1, hash2, "hash should change when world changes");

    // Modify component
    world.insert(e1, CHealth { hp: 50 });

    let hash3 = calculate_world_hash(&world);

    assert_ne!(hash2, hash3, "hash should change when component changes");
}

#[test]
fn test_world_hash_deterministic() {
    // Verify hash is deterministic for same world state
    let mut world = World::new();

    for i in 0..100 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(e, CHealth { hp: 50 + i });
    }

    let hash1 = calculate_world_hash(&world);
    let hash2 = calculate_world_hash(&world);
    let hash3 = calculate_world_hash(&world);

    assert_eq!(hash1, hash2, "hash should be deterministic");
    assert_eq!(hash2, hash3, "hash should be deterministic");
}
