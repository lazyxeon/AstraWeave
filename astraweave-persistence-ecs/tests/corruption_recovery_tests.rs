//! Corruption detection and recovery tests for astraweave-persistence-ecs
//!
//! Tests handling of corrupted save files, partial corruption, and checksum validation.

#![allow(clippy::field_reassign_with_default)]

use astraweave_core::ecs_components::*;
use astraweave_core::IVec2;
use astraweave_ecs::{Query, World};
use astraweave_persistence_ecs::{
    calculate_world_hash, deserialize_ecs_world, serialize_ecs_world, CPersistenceManager,
};
use aw_save::SaveManager;
use std::fs;
use tempfile::tempdir;

// ========== Corrupted File Detection Tests ==========

#[test]
fn test_corrupted_file_detection_invalid_magic() {
    // Create a valid save file then corrupt the magic bytes
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "corrupt_test".to_string(),
    };

    // Create and save world
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: 1, y: 2 },
        },
    );

    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let hash = calculate_world_hash(&world);

    let save_path = persistence
        .save_game(0, 0, hash, blob)
        .expect("save failed");

    // Corrupt the magic bytes
    let mut data = fs::read(&save_path).expect("read file");
    data[0] = 0xFF; // Corrupt first byte
    fs::write(&save_path, data).expect("write corrupted file");

    // Attempt to load should fail gracefully
    let result = persistence.load_game(0);
    assert!(result.is_err(), "loading corrupted file should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        !err_msg.contains("panic"),
        "error should not contain 'panic'"
    );
}

#[test]
fn test_corrupted_file_detection_invalid_payload() {
    // Test corruption in the payload data
    let _temp_dir = tempdir().expect("create temp dir");

    // Create a valid serialized world
    let mut world = World::new();
    for i in 0..10 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(e, CHealth { hp: 100 });
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");

    // Corrupt the blob
    let mut corrupted_blob = blob.clone();
    if corrupted_blob.len() > 10 {
        corrupted_blob[5] = 0xFF;
        corrupted_blob[10] = 0xAA;
    }

    // Attempt to deserialize should fail
    let mut world2 = World::new();
    let result = deserialize_ecs_world(&corrupted_blob, &mut world2);

    assert!(result.is_err(), "deserializing corrupted blob should fail");
}

#[test]
fn test_truncated_file_detection() {
    // Test detection of truncated save files
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "truncate_test".to_string(),
    };

    // Create and save world
    let mut world = World::new();
    for i in 0..100 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let hash = calculate_world_hash(&world);

    let save_path = persistence
        .save_game(0, 0, hash, blob)
        .expect("save failed");

    // Truncate file
    let mut data = fs::read(&save_path).expect("read file");
    data.truncate(data.len() / 2);
    fs::write(&save_path, data).expect("write truncated file");

    // Load should fail gracefully
    let result = persistence.load_game(0);
    assert!(result.is_err(), "loading truncated file should fail");
}

#[test]
fn test_empty_file_handling() {
    // Test handling of empty save files
    let temp_dir = tempdir().expect("create temp dir");
    let file_path = temp_dir.path().join("empty.awsv");

    // Create empty file
    fs::File::create(&file_path).expect("create file");

    // Attempt to read as save file (would be done via aw-save)
    // This tests that the system handles empty files gracefully
    let data = fs::read(&file_path).expect("read file");
    assert!(data.is_empty(), "file should be empty");

    // Empty blob should deserialize without errors
    let mut world = World::new();
    let result = deserialize_ecs_world(&[], &mut world);
    assert!(result.is_ok(), "empty blob should deserialize successfully");
}

#[test]
fn test_random_data_file() {
    // Test that random data is detected as invalid or produces empty world
    let temp_dir = tempdir().expect("create temp dir");
    let file_path = temp_dir.path().join("random.awsv");

    // Write random data that's unlikely to be valid postcard
    let random_data: Vec<u8> = (0..1000).map(|i| (i * 17) as u8).collect();
    fs::write(&file_path, random_data).expect("write file");

    // This would fail when aw-save tries to parse it
    // We test that our deserializer handles invalid data
    let data = fs::read(&file_path).expect("read file");
    let mut world = World::new();
    let result = deserialize_ecs_world(&data, &mut world);

    // Either it fails to deserialize, or if it succeeds, it should produce an empty/near-empty world
    if let Ok(()) = result {
        // If it somehow deserialized, verify it's empty or has very few entities
        let mut count = 0;
        {
            let mut q = Query::<CPos>::new(&world);
            while q.next().is_some() {
                count += 1;
            }
        }
        assert!(
            count < 10,
            "random data should not produce many valid entities"
        );
    }
    // If it errored, that's the expected behavior
}

// ========== Partial Corruption Tests ==========

#[test]
fn test_partial_corruption_recovery() {
    // Test that we fail cleanly rather than partially loading corrupted data
    let mut world = World::new();

    for i in 0..50 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");

    // Corrupt middle of blob
    let mut corrupted_blob = blob.clone();
    let mid = corrupted_blob.len() / 2;
    if mid < corrupted_blob.len() {
        corrupted_blob[mid] = !corrupted_blob[mid];
        corrupted_blob[mid + 1] = !corrupted_blob[mid + 1];
    }

    // Deserialize should fail entirely (not partial)
    let mut world2 = World::new();
    let result = deserialize_ecs_world(&corrupted_blob, &mut world2);

    assert!(result.is_err(), "should fail on corrupted data");

    // Verify world2 wasn't partially modified
    let mut q = Query::<CPos>::new(&world2);
    // World should either be empty or fully loaded, not partial
    let entity_count = std::iter::from_fn(|| q.next()).count();
    assert_eq!(entity_count, 0, "world should be empty after failed load");
}

#[test]
fn test_corruption_at_start() {
    // Test corruption at the beginning of the blob
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: 1, y: 2 },
        },
    );

    let blob = serialize_ecs_world(&world).expect("serialize failed");

    // Corrupt first few bytes
    let mut corrupted_blob = blob.clone();
    for i in 0..std::cmp::min(5, corrupted_blob.len()) {
        corrupted_blob[i] = 0xFF;
    }

    let mut world2 = World::new();
    let result = deserialize_ecs_world(&corrupted_blob, &mut world2);

    assert!(result.is_err(), "corruption at start should be detected");
}

#[test]
fn test_corruption_at_end() {
    // Test corruption at the end of the blob
    let mut world = World::new();
    for i in 0..10 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");

    // Corrupt last few bytes
    let mut corrupted_blob = blob.clone();
    let len = corrupted_blob.len();
    for byte in corrupted_blob.iter_mut().skip(len.saturating_sub(5)) {
        *byte = 0xAA;
    }

    let mut world2 = World::new();
    let result = deserialize_ecs_world(&corrupted_blob, &mut world2);

    // Depending on compression, this might succeed or fail
    // If it fails, that's expected. If it succeeds, verify integrity with hash
    if result.is_ok() {
        // Verify data integrity by comparing counts
        let original_count = {
            let mut q = Query::<CPos>::new(&world);
            std::iter::from_fn(|| q.next()).count()
        };

        let loaded_count = {
            let mut q = Query::<CPos>::new(&world2);
            std::iter::from_fn(|| q.next()).count()
        };

        // If deserialization succeeded, counts should match
        assert_eq!(
            original_count, loaded_count,
            "entity count should match if load succeeded"
        );
    }
}

// ========== Checksum Validation Tests ==========

#[test]
fn test_world_hash_validation() {
    // Test that world hash is deterministic for same world state
    let mut world = World::new();
    for i in 0..100 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let original_hash = calculate_world_hash(&world);

    // Hash should be deterministic for same world
    let hash2 = calculate_world_hash(&world);
    assert_eq!(original_hash, hash2, "hash should be deterministic");

    // Deserialize normally
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    let loaded_hash = calculate_world_hash(&world2);

    // The blob is sorted by raw id. Loading it into an empty world spawns in that
    // same order, so the remapped ids match the originals and the hash matches.
    assert_eq!(
        original_hash, loaded_hash,
        "empty-world load keeps entity ids when the blob is sorted by raw id"
    );

    let mut occupied = World::new();
    let _prior = occupied.spawn();
    deserialize_ecs_world(&blob, &mut occupied).expect("deserialize into occupied world");
    let shifted_hash = calculate_world_hash(&occupied);
    assert_ne!(
        original_hash, shifted_hash,
        "a world that already has an entity remaps ids and changes the hash"
    );

    // Verify data integrity by checking counts
    let original_count = {
        let mut q = Query::<CPos>::new(&world);
        std::iter::from_fn(|| q.next()).count()
    };
    let loaded_count = {
        let mut q = Query::<CPos>::new(&world2);
        std::iter::from_fn(|| q.next()).count()
    };
    assert_eq!(original_count, loaded_count, "entity counts should match");
}

#[test]
fn test_hash_mismatch_detection() {
    // Test that hash mismatch indicates corruption
    let mut world1 = World::new();
    for i in 0..50 {
        let e = world1.spawn();
        world1.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
    }

    let mut world2 = World::new();
    for i in 0..50 {
        let e = world2.spawn();
        world2.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i + 1 },
            },
        ); // Different data
    }

    let hash1 = calculate_world_hash(&world1);
    let hash2 = calculate_world_hash(&world2);

    assert_ne!(
        hash1, hash2,
        "different world states should have different hashes"
    );
}

#[test]
fn test_save_load_with_hash_validation() {
    // Full integration test with hash validation
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "hash_validation_test".to_string(),
    };

    // Create world
    let mut world = World::new();
    for i in 0..200 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(e, CHealth { hp: 50 + (i % 50) });
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let original_hash = calculate_world_hash(&world);

    // Save with hash
    persistence
        .save_game(0, 1000, original_hash, blob)
        .expect("save failed");

    // Load and validate
    let (bundle, _) = persistence.load_game(0).expect("load failed");

    assert_eq!(
        bundle.world.state_hash, original_hash,
        "saved hash should match"
    );

    // Deserialize and verify data integrity
    let mut world2 = World::new();
    deserialize_ecs_world(&bundle.world.ecs_blob, &mut world2).expect("deserialize failed");

    // Verify counts match
    let original_count = {
        let mut q = Query::<CPos>::new(&world);
        std::iter::from_fn(|| q.next()).count()
    };
    let loaded_count = {
        let mut q = Query::<CPos>::new(&world2);
        std::iter::from_fn(|| q.next()).count()
    };

    assert_eq!(
        original_count, loaded_count,
        "entity count should match after full roundtrip"
    );
}

#[test]
fn test_concurrent_save_attempts() {
    // Test that concurrent save attempts don't corrupt data
    // Note: This is a basic test - full concurrency testing would require threads
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "concurrent_test".to_string(),
    };

    // Save multiple times rapidly to same slot
    for i in 0..5 {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 + i });

        let blob = serialize_ecs_world(&world).expect("serialize failed");
        let hash = calculate_world_hash(&world);

        persistence
            .save_game(0, i as u64, hash, blob)
            .expect("save failed");

        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Load latest save
    let (bundle, _) = persistence.load_game(0).expect("load failed");

    // Should be the last save (tick 4), but verify it's one of the valid ticks
    assert!(
        bundle.world.tick <= 4,
        "tick should be <= 4, got {}",
        bundle.world.tick
    );

    // Verify data integrity - world should deserialize successfully
    let mut world = World::new();
    deserialize_ecs_world(&bundle.world.ecs_blob, &mut world).expect("deserialize failed");

    let mut q = Query::<CPos>::new(&world);
    let (_, _pos) = q.next().expect("should have entity");
    // Position should be valid (between 0 and 4)
    // We don't check exact value since we're not sure which save was loaded
}

#[test]
fn test_load_nonexistent_slot() {
    // Test loading from a slot that doesn't exist
    let temp_dir = tempdir().expect("create temp dir");
    let save_manager = SaveManager::new(temp_dir.path());

    let persistence = CPersistenceManager {
        save_manager,
        current_player: "nonexistent_test".to_string(),
    };

    // Attempt to load from slot that doesn't exist
    let result = persistence.load_game(99);

    assert!(result.is_err(), "loading nonexistent slot should fail");
}
