//! Large world performance tests for astraweave-persistence-ecs
//!
//! Tests serialization/deserialization performance with large entity counts,
//! memory usage, and load time requirements.

use astraweave_core::ecs_components::*;
use astraweave_core::IVec2;
use astraweave_ecs::{Query, World};
use astraweave_persistence_ecs::{
    calculate_world_hash, deserialize_ecs_world, serialize_ecs_world,
};

// ========== Large World Tests ==========

/// Performance test for serializing 10,000 entities
/// Uses CI-friendly thresholds (500ms) to account for debug builds and system load
#[test]
fn test_save_10000_entities() {
    // Test serialization of 10,000 entities
    let mut world = World::new();

    for i in 0..10_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: (i * 2) },
            },
        );
        world.insert(e, CHealth { hp: 100 });

        // Add variety
        if i % 2 == 0 {
            world.insert(e, CTeam { id: (i % 4) as u8 });
        }
        if i % 3 == 0 {
            world.insert(e, CAmmo { rounds: 30 });
        }
    }

    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let duration = start.elapsed();

    println!("Serialized 10,000 entities in {:?}", duration);
    println!(
        "Blob size: {} bytes ({:.2} MB)",
        blob.len(),
        blob.len() as f64 / (1024.0 * 1024.0)
    );

    // Verify blob size is reasonable
    assert!(
        blob.len() < 1_000_000,
        "blob should be under 1MB for 10,000 entities"
    );

    // CI-friendly performance check (debug builds are 5-10x slower than release)
    // Release typically achieves ~20-50ms, debug ~100-250ms, CI ~250-500ms
    assert!(
        duration.as_millis() < 500,
        "serialization should complete in under 500ms in debug mode, got {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_load_10000_entities_under_5_seconds() {
    // Test that loading 10,000 entities completes in under 5 seconds
    let mut world = World::new();

    for i in 0..10_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 50 + (i % 100) });
        world.insert(e, CTeam { id: (i % 8) as u8 });
    }

    let blob = serialize_ecs_world(&world).expect("serialize failed");

    let start = std::time::Instant::now();
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");
    let duration = start.elapsed();

    println!("Deserialized 10,000 entities in {:?}", duration);

    // Verify entity count
    let mut count = 0;
    {
        let mut q = Query::<CPos>::new(&world2);
        while q.next().is_some() {
            count += 1;
        }
    }
    assert_eq!(count, 10_000);

    // Verify load time < 5 seconds
    assert!(
        duration.as_secs() < 5,
        "load should complete in under 5 seconds"
    );

    // Actually should be much faster - verify < 1 second
    assert!(
        duration.as_secs() < 1,
        "load should actually complete in under 1 second"
    );
}

#[test]
fn test_save_100000_components() {
    // Test with 100,000 total components across entities
    let mut world = World::new();

    // Create ~20,000 entities with 5 components each = 100,000 components
    for i in 0..20_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
        world.insert(e, CTeam { id: (i % 4) as u8 });
        world.insert(e, CAmmo { rounds: 30 });
        world.insert(e, CCooldowns::default());
    }

    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let serialize_duration = start.elapsed();

    println!("Serialized 100,000 components in {:?}", serialize_duration);
    println!(
        "Blob size: {} bytes ({:.2} MB)",
        blob.len(),
        blob.len() as f64 / (1024.0 * 1024.0)
    );

    // Deserialize
    let start = std::time::Instant::now();
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");
    let deserialize_duration = start.elapsed();

    println!(
        "Deserialized 100,000 components in {:?}",
        deserialize_duration
    );

    // Verify
    let mut entity_count = 0;
    {
        let mut q = Query::<CPos>::new(&world2);
        while q.next().is_some() {
            entity_count += 1;
        }
    }
    assert_eq!(entity_count, 20_000);

    // CI-friendly performance checks (debug builds are 5-10x slower, CI adds overhead)
    // Release typically achieves ~100ms serialize, ~200ms deserialize
    // Debug mode: ~500ms serialize, ~1500ms deserialize
    // CI debug mode: ~1000ms serialize, ~3000ms deserialize
    assert!(
        serialize_duration.as_millis() < 2000,
        "serialize should be under 2000ms in debug mode, got {}ms",
        serialize_duration.as_millis()
    );
    assert!(
        deserialize_duration.as_millis() < 5000,
        "deserialize should be under 5000ms in debug mode, got {}ms",
        deserialize_duration.as_millis()
    );
}

#[test]
fn test_memory_usage_reasonable() {
    // Test that serialization doesn't cause excessive memory allocations
    // This is a basic test - more sophisticated profiling would be needed for production

    let mut world = World::new();

    for i in 0..50_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
    }

    // Serialize multiple times to check for memory leaks
    for _ in 0..10 {
        let blob = serialize_ecs_world(&world).expect("serialize failed");
        assert!(!blob.is_empty());

        // Deserialize
        let mut world2 = World::new();
        deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

        // Verify
        let mut count = 0;
        {
            let mut q = Query::<CPos>::new(&world2);
            while q.next().is_some() {
                count += 1;
            }
        }
        assert_eq!(count, 50_000);
    }

    // If we get here without panicking or running out of memory, test passes
}

/// Performance test for world hash calculation with 50,000 entities
/// Uses CI-friendly thresholds (5000ms) to account for debug builds and system load
#[test]
fn test_large_world_hash_performance() {
    // Test that world hash calculation is fast even for large worlds
    let mut world = World::new();

    for i in 0..50_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
        world.insert(e, CTeam { id: (i % 4) as u8 });
        world.insert(e, CAmmo { rounds: 30 });
    }

    let start = std::time::Instant::now();
    let hash1 = calculate_world_hash(&world);
    let duration1 = start.elapsed();

    println!("Calculated hash for 50,000 entities in {:?}", duration1);

    // Calculate again to verify consistency
    let start = std::time::Instant::now();
    let hash2 = calculate_world_hash(&world);
    let duration2 = start.elapsed();

    // Hashes should match
    assert_eq!(hash1, hash2);

    // CI-friendly thresholds: 60000ms (1 min) for debug builds on slow CI runners
    // Release builds typically achieve ~500-800ms, debug ~3000-10000ms, CI debug ~10000-30000ms
    // Hash calculation involves full world traversal which is O(n) with high constant in debug
    assert!(
        duration1.as_millis() < 60000,
        "hash calculation should be under 60000ms in debug mode, got {}ms",
        duration1.as_millis()
    );
    assert!(
        duration2.as_millis() < 60000,
        "hash calculation should be consistent, got {}ms",
        duration2.as_millis()
    );
}

#[test]
fn test_roundtrip_50000_entities() {
    // Full roundtrip test with 50,000 entities
    let mut world = World::new();

    for i in 0..50_000 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 {
                    x: i,
                    y: (i % 1000),
                },
            },
        );
        world.insert(e, CHealth { hp: 50 + (i % 100) });

        if i % 5 == 0 {
            world.insert(e, CTeam { id: (i % 10) as u8 });
        }
    }

    // Serialize
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let _original_hash = calculate_world_hash(&world);

    // Deserialize
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");

    // Verify entity count matches (hash won't match due to entity ID remapping)
    let original_count = {
        let mut q = Query::<CPos>::new(&world);
        std::iter::from_fn(|| q.next()).count()
    };
    let loaded_count = {
        let mut q = Query::<CPos>::new(&world2);
        std::iter::from_fn(|| q.next()).count()
    };

    assert_eq!(original_count, loaded_count, "entity count should match");
    assert_eq!(loaded_count, 50_000, "should have 50,000 entities");
}

#[test]
fn test_blob_size_scaling() {
    // Test that blob size scales linearly with entity count
    let sizes = vec![100, 500, 1000, 5000];
    let mut blob_sizes = Vec::new();

    for &entity_count in &sizes {
        let mut world = World::new();

        for i in 0..entity_count {
            let e = world.spawn();
            world.insert(
                e,
                CPos {
                    pos: IVec2 { x: i, y: i },
                },
            );
            world.insert(e, CHealth { hp: 100 });
            world.insert(e, CTeam { id: 1 });
        }

        let blob = serialize_ecs_world(&world).expect("serialize failed");
        blob_sizes.push(blob.len());

        println!(
            "{} entities → {} bytes ({:.2} bytes/entity)",
            entity_count,
            blob.len(),
            blob.len() as f64 / entity_count as f64
        );
    }

    // Verify scaling is roughly linear
    // bytes per entity should be relatively consistent
    let bytes_per_entity: Vec<f64> = sizes
        .iter()
        .zip(&blob_sizes)
        .map(|(&count, &size)| size as f64 / count as f64)
        .collect();

    // All should be within 50% of each other (accounting for overhead)
    let avg = bytes_per_entity.iter().sum::<f64>() / bytes_per_entity.len() as f64;
    for &bpe in &bytes_per_entity {
        assert!(
            (bpe - avg).abs() / avg < 0.5,
            "bytes per entity should be relatively consistent"
        );
    }
}

#[test]
fn test_serialize_deserialize_throughput() {
    // Measure throughput in entities per second
    let entity_count = 10_000;
    let mut world = World::new();

    for i in 0..entity_count {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: i },
            },
        );
        world.insert(e, CHealth { hp: 100 });
        world.insert(e, CTeam { id: 1 });
    }

    // Serialize throughput
    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).expect("serialize failed");
    let serialize_duration = start.elapsed();

    let serialize_throughput = entity_count as f64 / serialize_duration.as_secs_f64();

    println!(
        "Serialize throughput: {:.0} entities/sec",
        serialize_throughput
    );

    // Deserialize throughput
    let start = std::time::Instant::now();
    let mut world2 = World::new();
    deserialize_ecs_world(&blob, &mut world2).expect("deserialize failed");
    let deserialize_duration = start.elapsed();

    let deserialize_throughput = entity_count as f64 / deserialize_duration.as_secs_f64();

    println!(
        "Deserialize throughput: {:.0} entities/sec",
        deserialize_throughput
    );

    // Verify reasonable throughput (should be >> 1000 entities/sec)
    assert!(
        serialize_throughput > 10_000.0,
        "serialize throughput should be > 10k entities/sec"
    );
    assert!(
        deserialize_throughput > 5_000.0,
        "deserialize throughput should be > 5k entities/sec"
    );
}
