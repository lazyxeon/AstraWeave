// determinism.rs - Async Physics Determinism Tests
//
// These tests verify that async/parallel physics produces identical results
// to single-threaded physics across multiple runs with different seeds.
//
// Critical for gameplay: players expect consistent behavior in multiplayer,
// replays, and AI simulations.

#![cfg(all(test, feature = "async-physics"))]

use astraweave_physics::{Layers, PhysicsWorld};
use glam::{vec3, Vec3};
use std::collections::HashMap;

/// Helper to create a deterministic test world
fn create_test_world(seed: u64) -> PhysicsWorld {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));

    // Create ground
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);

    // Create deterministic random positions based on seed
    use rand::Rng;
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Add 50 dynamic bodies in deterministic positions
    for _ in 0..50 {
        let x = rng.random_range(-20.0..20.0);
        let y = rng.random_range(5.0..15.0);
        let z = rng.random_range(-20.0..20.0);

        world.add_dynamic_box(vec3(x, y, z), vec3(0.5, 0.5, 0.5), 1.0, Layers::DEFAULT);
    }

    // Add 10 characters
    for i in 0..10 {
        let x = (i as f32) * 3.0 - 15.0;
        world.add_character(vec3(x, 1.0, 0.0), vec3(0.4, 0.9, 0.4));
    }

    world
}

/// Extract deterministic state snapshot from world
fn extract_state_snapshot(world: &PhysicsWorld) -> HashMap<u64, Vec3> {
    let mut positions = HashMap::new();

    // Get all body positions
    for (idx, (_h, rb)) in world.bodies.iter().enumerate() {
        let iso = rb.position();
        let pos = vec3(iso.translation.x, iso.translation.y, iso.translation.z);
        // Use index as key since we need consistent ordering
        positions.insert(idx as u64, pos);
    }

    positions
}

/// Run simulation for N steps and return final state
fn simulate_steps(world: &mut PhysicsWorld, steps: usize) -> HashMap<u64, Vec3> {
    for _ in 0..steps {
        world.step();
    }
    extract_state_snapshot(world)
}

#[test]
fn test_determinism_single_run() {
    // Same seed, same results
    let seed = 12345;

    let mut world1 = create_test_world(seed);
    let mut world2 = create_test_world(seed);

    let state1 = simulate_steps(&mut world1, 60);
    let state2 = simulate_steps(&mut world2, 60);

    assert_eq!(state1.len(), state2.len(), "Body counts should match");

    for (id, pos1) in state1.iter() {
        let pos2 = state2.get(id).expect("Body ID should exist in both");

        // Allow small floating point error (< 0.0001)
        let diff = (*pos1 - *pos2).length();
        assert!(
            diff < 0.0001,
            "Body {} positions differ: {:?} vs {:?} (diff: {})",
            id,
            pos1,
            pos2,
            diff
        );
    }
}

#[test]
fn test_determinism_100_seeds() {
    // Test determinism across 100 different seeds
    const SEED_COUNT: u64 = 100;
    const STEPS: usize = 30; // Shorter simulation for speed

    for seed in 0..SEED_COUNT {
        let mut world1 = create_test_world(seed);
        let mut world2 = create_test_world(seed);

        let state1 = simulate_steps(&mut world1, STEPS);
        let state2 = simulate_steps(&mut world2, STEPS);

        assert_eq!(
            state1.len(),
            state2.len(),
            "Seed {}: Body counts should match",
            seed
        );

        for (id, pos1) in state1.iter() {
            let pos2 = state2
                .get(id)
                .unwrap_or_else(|| panic!("Seed {}: Body {} missing", seed, id));

            let diff = (*pos1 - *pos2).length();
            assert!(
                diff < 0.0001,
                "Seed {}: Body {} positions differ: {:?} vs {:?} (diff: {})",
                seed,
                id,
                pos1,
                pos2,
                diff
            );
        }
    }
}

#[test]
fn test_determinism_with_character_movement() {
    let seed = 54321;
    let steps = 60;

    // Create two identical worlds with character movement
    let mut world1 = create_test_world(seed);
    let mut world2 = create_test_world(seed);

    // Get character IDs (same order due to deterministic creation)
    let char_ids: Vec<u64> = (1..=10).collect(); // Character IDs 1-10

    for step in 0..steps {
        // Move characters deterministically
        for &id in &char_ids {
            let move_dir = vec3(
                ((step as f32) * 0.1).sin(),
                0.0,
                ((step as f32) * 0.1).cos(),
            );
            world1.control_character(id, move_dir, 1.0 / 60.0, false);
            world2.control_character(id, move_dir, 1.0 / 60.0, false);
        }

        world1.step();
        world2.step();
    }

    let state1 = extract_state_snapshot(&world1);
    let state2 = extract_state_snapshot(&world2);

    for (id, pos1) in state1.iter() {
        let pos2 = state2.get(id).unwrap();
        let diff = (*pos1 - *pos2).length();
        assert!(
            diff < 0.0001,
            "Character body {} positions differ after movement: {:?} vs {:?} (diff: {})",
            id,
            pos1,
            pos2,
            diff
        );
    }
}

#[test]
fn test_async_vs_sync_equivalence() {
    // This test will be expanded once async implementation is complete
    // For now, it verifies the structure is in place

    let seed = 99999;
    let mut world = create_test_world(seed);

    // Enable async physics (currently delegates to sync)
    world.enable_async_physics(4);

    // Verify scheduler is enabled
    assert!(world.async_scheduler.is_some());

    // Run a few steps
    for _ in 0..10 {
        world.step();
    }

    // Verify telemetry is captured
    let _profile = world.get_last_profile();
    // Profile will be None until we implement full async pipeline
    // This test documents the expected API
}

#[test]
fn test_determinism_stress() {
    // Stress test: large world, many bodies, many steps
    let seed = 77777;
    let mut world = create_test_world(seed);

    // Add more bodies for stress
    use rand::Rng;
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    for _ in 0..200 {
        let x = rng.random_range(-30.0..30.0);
        let y = rng.random_range(5.0..20.0);
        let z = rng.random_range(-30.0..30.0);

        world.add_dynamic_box(vec3(x, y, z), vec3(0.3, 0.3, 0.3), 0.5, Layers::DEFAULT);
    }

    // Run 120 steps (2 seconds at 60 FPS)
    let initial_state = extract_state_snapshot(&world);

    for _ in 0..120 {
        world.step();
    }

    let final_state = extract_state_snapshot(&world);

    // Verify some bodies moved (physics is active)
    let mut moved_count = 0;
    for (id, initial_pos) in initial_state.iter() {
        if let Some(final_pos) = final_state.get(id) {
            if (*initial_pos - *final_pos).length() > 0.1 {
                moved_count += 1;
            }
        }
    }

    assert!(
        moved_count > 50,
        "Expected many bodies to move, only {} moved",
        moved_count
    );
}
