//! Mutation-Resistant Tests for astraweave-ecs
//!
//! These tests verify **exact computed values** and **behavioral correctness** to ensure
//! mutations to formulas and logic are detected by `cargo mutants`.

#![cfg(test)]
#![allow(
    clippy::manual_range_contains,
    clippy::len_zero,
    clippy::duplicated_attributes
)]

use crate::{Entity, Event, Events, Rng, SystemStage, World};

// =============================================================================
// Miri-Compatible Iteration Constants
// =============================================================================
// Miri runs ~100× slower than native execution, so we reduce iteration counts
// to keep tests practical while still exercising the same code paths.

/// Small iteration count: 1000 normally, 10 under Miri
#[cfg(miri)]
const ITER_SMALL: usize = 10;
#[cfg(not(miri))]
const ITER_SMALL: usize = 1000;

/// Medium iteration count: 10000 normally, 100 under Miri
#[cfg(miri)]
const ITER_MEDIUM: usize = 100;
#[cfg(not(miri))]
const ITER_MEDIUM: usize = 10000;

/// Large iteration count: 100000 normally, 1000 under Miri
#[cfg(miri)]
const ITER_LARGE: usize = 1000;
#[cfg(not(miri))]
const ITER_LARGE: usize = 100000;

// =============================================================================
// Rng Determinism Tests - Verify exact reproducible values
// =============================================================================

mod rng_tests {
    use super::*;

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut rng1 = Rng::from_seed(12345);
        let mut rng2 = Rng::from_seed(12345);

        for _ in 0..100 {
            assert_eq!(
                rng1.gen_u32(),
                rng2.gen_u32(),
                "Same seed must produce identical u32"
            );
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let mut rng1 = Rng::from_seed(1);
        let mut rng2 = Rng::from_seed(2);

        let val1 = rng1.gen_u32();
        let val2 = rng2.gen_u32();

        assert_ne!(
            val1, val2,
            "Different seeds should produce different values"
        );
    }

    #[test]
    fn seed_getter_returns_initialization_seed() {
        let seed = 42_u64;
        let rng = Rng::from_seed(seed);
        assert_eq!(rng.seed(), seed, "seed() must return initialization seed");
    }

    #[test]
    fn gen_range_stays_within_bounds_u32() {
        let mut rng = Rng::from_seed(1000);
        for _ in 0..ITER_SMALL {
            let val = rng.gen_range(10_u32..20);
            assert!(val >= 10 && val < 20, "gen_range must stay within [10, 20)");
        }
    }

    #[test]
    fn gen_range_stays_within_bounds_f32() {
        let mut rng = Rng::from_seed(2000);
        for _ in 0..ITER_SMALL {
            let val = rng.gen_range(0.0_f32..1.0);
            assert!(
                val >= 0.0 && val < 1.0,
                "gen_range f32 must stay within [0.0, 1.0)"
            );
        }
    }

    #[test]
    fn gen_bool_returns_true_at_high_probability() {
        let mut rng = Rng::from_seed(3000);
        let count = (0..1000).filter(|_| rng.gen_bool(0.99)).count();
        // With p=0.99, expect ~990 trues (allow some variance)
        assert!(count > 900, "gen_bool(0.99) should mostly return true");
    }

    #[test]
    fn gen_bool_returns_false_at_zero_probability() {
        let mut rng = Rng::from_seed(4000);
        let count = (0..100).filter(|_| rng.gen_bool(0.0)).count();
        assert_eq!(count, 0, "gen_bool(0.0) must never return true");
    }

    #[test]
    fn gen_bool_returns_true_at_one_probability() {
        let mut rng = Rng::from_seed(5000);
        let count = (0..100).filter(|_| !rng.gen_bool(1.0)).count();
        assert_eq!(count, 0, "gen_bool(1.0) must always return true");
    }

    #[test]
    fn cloned_rng_produces_same_sequence() {
        let mut rng1 = Rng::from_seed(6000);
        let rng2 = rng1.clone();
        let mut rng2 = rng2;

        for _ in 0..50 {
            assert_eq!(
                rng1.gen_u64(),
                rng2.gen_u64(),
                "Cloned RNG must produce identical sequence"
            );
        }
    }

    #[test]
    fn serialization_preserves_seed() {
        let original = Rng::from_seed(7890);
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: Rng = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            original.seed(),
            restored.seed(),
            "Serialization must preserve seed"
        );
    }
}

// =============================================================================
// Events System Tests - Verify exact event behavior
// =============================================================================

mod events_tests {
    use super::*;

    // Define test event types
    #[derive(Debug, Clone, PartialEq)]
    struct DamageEvent {
        amount: i32,
    }
    impl Event for DamageEvent {}

    #[derive(Debug, Clone, PartialEq)]
    struct HealEvent {
        amount: i32,
    }
    impl Event for HealEvent {}

    #[test]
    fn new_events_has_zero_frame() {
        let events = Events::new();
        assert_eq!(events.current_frame(), 0, "New events should have frame 0");
    }

    #[test]
    fn update_increments_frame() {
        let mut events = Events::new();
        events.update();
        assert_eq!(events.current_frame(), 1, "update() should increment frame");
        events.update();
        assert_eq!(events.current_frame(), 2, "update() should increment again");
    }

    #[test]
    fn send_increases_len() {
        let mut events = Events::new();
        assert_eq!(events.len::<DamageEvent>(), 0, "Initial len should be 0");

        events.send(DamageEvent { amount: 10 });
        assert_eq!(
            events.len::<DamageEvent>(),
            1,
            "After send, len should be 1"
        );

        events.send(DamageEvent { amount: 20 });
        assert_eq!(
            events.len::<DamageEvent>(),
            2,
            "After second send, len should be 2"
        );
    }

    #[test]
    fn is_empty_reflects_event_count() {
        let mut events = Events::new();
        assert!(
            events.is_empty::<DamageEvent>(),
            "Should be empty initially"
        );

        events.send(DamageEvent { amount: 5 });
        assert!(
            !events.is_empty::<DamageEvent>(),
            "Should not be empty after send"
        );
    }

    #[test]
    fn read_returns_all_sent_events() {
        let mut events = Events::new();
        events.send(DamageEvent { amount: 1 });
        events.send(DamageEvent { amount: 2 });
        events.send(DamageEvent { amount: 3 });

        let amounts: Vec<i32> = events.read::<DamageEvent>().map(|e| e.amount).collect();
        assert_eq!(
            amounts,
            vec![1, 2, 3],
            "read() must return all events in order"
        );
    }

    #[test]
    fn drain_removes_events() {
        let mut events = Events::new();
        events.send(DamageEvent { amount: 10 });
        events.send(DamageEvent { amount: 20 });

        let drained: Vec<_> = events.drain::<DamageEvent>().collect();
        assert_eq!(drained.len(), 2, "drain() should return 2 events");
        assert!(
            events.is_empty::<DamageEvent>(),
            "After drain, events should be empty"
        );
    }

    #[test]
    fn clear_removes_all_events() {
        let mut events = Events::new();
        events.send(DamageEvent { amount: 10 });
        events.send(DamageEvent { amount: 20 });

        events.clear::<DamageEvent>();
        assert!(
            events.is_empty::<DamageEvent>(),
            "After clear, events should be empty"
        );
    }

    #[test]
    fn different_event_types_are_independent() {
        let mut events = Events::new();
        events.send(DamageEvent { amount: 10 });
        events.send(HealEvent { amount: 5 });

        assert_eq!(
            events.len::<DamageEvent>(),
            1,
            "DamageEvent len should be 1"
        );
        assert_eq!(events.len::<HealEvent>(), 1, "HealEvent len should be 1");

        events.clear::<DamageEvent>();
        assert!(
            events.is_empty::<DamageEvent>(),
            "DamageEvent should be empty"
        );
        assert!(
            !events.is_empty::<HealEvent>(),
            "HealEvent should still have 1"
        );
    }

    #[test]
    fn clear_all_clears_everything() {
        let mut events = Events::new();
        events.send(DamageEvent { amount: 10 });
        events.send(HealEvent { amount: 5 });

        events.clear_all();
        assert!(
            events.is_empty::<DamageEvent>(),
            "DamageEvent should be empty"
        );
        assert!(events.is_empty::<HealEvent>(), "HealEvent should be empty");
    }
}

// =============================================================================
// World Entity Tests - Verify entity lifecycle
// =============================================================================

mod world_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }

    #[test]
    fn spawn_returns_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity), "Spawned entity should be alive");
    }

    #[test]
    fn spawn_returns_unique_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        assert_ne!(e1.id(), e2.id(), "Entity IDs should be unique");
        assert_ne!(e2.id(), e3.id(), "Entity IDs should be unique");
        assert_ne!(e1.id(), e3.id(), "Entity IDs should be unique");
    }

    #[test]
    fn despawn_makes_entity_not_alive() {
        let mut world = World::new();
        let entity = world.spawn();

        world.despawn(entity);
        assert!(
            !world.is_alive(entity),
            "Despawned entity should not be alive"
        );
    }

    #[test]
    fn insert_adds_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(entity, Position { x: 1.0, y: 2.0 });

        let pos = world.get::<Position>(entity);
        assert!(pos.is_some(), "Component should exist after insert");
        let pos = pos.unwrap();
        assert!((pos.x - 1.0).abs() < 0.0001, "Position x should be 1.0");
        assert!((pos.y - 2.0).abs() < 0.0001, "Position y should be 2.0");
    }

    #[test]
    fn get_returns_none_for_missing_component() {
        let mut world = World::new();
        let entity = world.spawn();

        let vel = world.get::<Velocity>(entity);
        assert!(
            vel.is_none(),
            "get() should return None for missing component"
        );
    }

    #[test]
    fn remove_removes_component() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 0.0, y: 0.0 });

        world.remove::<Position>(entity);

        let pos = world.get::<Position>(entity);
        assert!(pos.is_none(), "Component should be gone after remove");
    }

    #[test]
    fn get_mut_allows_modification() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 0.0, y: 0.0 });

        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x = 5.0;
            pos.y = 10.0;
        }

        let pos = world.get::<Position>(entity).unwrap();
        assert!(
            (pos.x - 5.0).abs() < 0.0001,
            "Position x should be modified to 5.0"
        );
        assert!(
            (pos.y - 10.0).abs() < 0.0001,
            "Position y should be modified to 10.0"
        );
    }

    #[test]
    fn multiple_components_on_same_entity() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { dx: 3.0, dy: 4.0 });

        let pos = world.get::<Position>(entity);
        let vel = world.get::<Velocity>(entity);

        assert!(pos.is_some(), "Position should exist");
        assert!(vel.is_some(), "Velocity should exist");
    }
}

// =============================================================================
// SystemStage Constants Tests - Verify exact stage names
// =============================================================================

mod system_stage_tests {
    use super::*;

    #[test]
    fn pre_simulation_is_exactly_pre_simulation() {
        assert_eq!(SystemStage::PRE_SIMULATION, "pre_simulation");
    }

    #[test]
    fn perception_is_exactly_perception() {
        assert_eq!(SystemStage::PERCEPTION, "perception");
    }

    #[test]
    fn simulation_is_exactly_simulation() {
        assert_eq!(SystemStage::SIMULATION, "simulation");
    }

    #[test]
    fn ai_planning_is_exactly_ai_planning() {
        assert_eq!(SystemStage::AI_PLANNING, "ai_planning");
    }

    #[test]
    fn physics_is_exactly_physics() {
        assert_eq!(SystemStage::PHYSICS, "physics");
    }

    #[test]
    fn post_simulation_is_exactly_post_simulation() {
        assert_eq!(SystemStage::POST_SIMULATION, "post_simulation");
    }

    #[test]
    fn presentation_is_exactly_presentation() {
        assert_eq!(SystemStage::PRESENTATION, "presentation");
    }
}

// =============================================================================
// Behavioral Correctness Tests - Mathematical Invariants
// =============================================================================

mod behavioral_correctness_tests {
    use super::*;

    // --- Rng statistical properties ---
    #[test]
    fn rng_covers_full_u32_range() {
        let mut rng = Rng::from_seed(9999);
        let mut min = u32::MAX;
        let mut max = u32::MIN;

        for _ in 0..ITER_MEDIUM {
            let val = rng.gen_u32();
            min = min.min(val);
            max = max.max(val);
        }

        // With 10k samples (100 under Miri), we should hit at least 10% of the range
        assert!(
            max - min > u32::MAX / 10,
            "RNG should cover significant portion of u32 range"
        );
    }

    // --- Entity ID uniqueness ---
    #[test]
    fn spawned_entities_have_unique_ids() {
        let mut world = World::new();
        let mut ids = std::collections::HashSet::new();

        for _ in 0..ITER_SMALL {
            let e = world.spawn();
            assert!(ids.insert(e.id()), "Entity IDs must be unique");
        }
    }

    // --- Event ordering is FIFO ---
    #[test]
    fn events_maintain_fifo_order() {
        #[derive(Debug, Clone)]
        struct OrderedEvent {
            seq: u32,
        }
        impl Event for OrderedEvent {}

        let mut events = Events::new();
        for i in 0..100 {
            events.send(OrderedEvent { seq: i });
        }

        let seqs: Vec<u32> = events.read::<OrderedEvent>().map(|e| e.seq).collect();
        for i in 0..100 {
            assert_eq!(seqs[i as usize], i, "Events must maintain FIFO order");
        }
    }

    // --- len() equals actual count ---
    #[test]
    fn events_len_equals_actual_count() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let mut events = Events::new();
        for i in 0..50 {
            events.send(TestEvent);
            let actual_count = events.read::<TestEvent>().count();
            assert_eq!(
                events.len::<TestEvent>(),
                actual_count,
                "len() must equal actual count"
            );
            assert_eq!(events.len::<TestEvent>(), (i + 1) as usize);
        }
    }

    // --- is_empty() equivalent to len() == 0 ---
    #[test]
    fn is_empty_equals_len_zero() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let mut events = Events::new();
        assert!(events.is_empty::<TestEvent>() == (events.len::<TestEvent>() == 0));

        events.send(TestEvent);
        assert!(events.is_empty::<TestEvent>() == (events.len::<TestEvent>() == 0));
    }

    // --- Frame counter is monotonically increasing ---
    #[test]
    fn frame_counter_monotonically_increases() {
        let mut events = Events::new();
        let mut prev_frame = events.current_frame();

        for _ in 0..100 {
            events.update();
            let curr_frame = events.current_frame();
            assert!(
                curr_frame > prev_frame,
                "Frame must increase on each update"
            );
            prev_frame = curr_frame;
        }
    }

    // --- Alive entity has components, dead entity does not ---
    #[test]
    fn dead_entity_has_no_components() {
        #[derive(Clone)]
        struct TestComp;

        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, TestComp);

        assert!(
            world.get::<TestComp>(entity).is_some(),
            "Alive entity should have component"
        );

        world.despawn(entity);

        // After despawn, entity is not alive
        assert!(
            !world.is_alive(entity),
            "Despawned entity should not be alive"
        );
    }

    // --- gen_range never returns value equal to exclusive upper bound ---
    #[test]
    fn gen_range_exclusive_upper_bound() {
        let mut rng = Rng::from_seed(8888);
        for _ in 0..ITER_MEDIUM {
            let val = rng.gen_range(0_u32..10);
            assert!(
                val < 10,
                "gen_range exclusive upper bound must never be returned"
            );
        }
    }
}

// =============================================================================
// BOUNDARY CONDITION TESTS - Test exact boundary values to catch < vs <= mutations
// =============================================================================

mod boundary_condition_tests {
    use super::*;

    // --- Rng gen_range boundaries ---

    #[test]
    fn gen_range_u32_at_lower_bound() {
        let mut rng = Rng::from_seed(1234);
        // With enough iterations, we should hit the lower bound
        let mut hit_lower = false;
        for _ in 0..ITER_LARGE {
            let val = rng.gen_range(0_u32..10);
            if val == 0 {
                hit_lower = true;
                break;
            }
        }
        assert!(hit_lower, "gen_range should eventually hit lower bound 0");
    }

    #[test]
    fn gen_range_u32_at_upper_bound_minus_one() {
        let mut rng = Rng::from_seed(5678);
        // With enough iterations, we should hit upper_bound - 1
        let mut hit_upper = false;
        for _ in 0..ITER_LARGE {
            let val = rng.gen_range(0_u32..10);
            if val == 9 {
                hit_upper = true;
                break;
            }
        }
        assert!(
            hit_upper,
            "gen_range should eventually hit upper bound - 1 (9)"
        );
    }

    #[test]
    fn gen_range_f32_at_lower_bound() {
        let mut rng = Rng::from_seed(2222);
        let mut found_near_zero = false;
        for _ in 0..ITER_MEDIUM {
            let val = rng.gen_range(0.0_f32..1.0);
            if val < 0.01 {
                found_near_zero = true;
            }
        }
        assert!(
            found_near_zero,
            "gen_range f32 should generate values near 0.0"
        );
    }

    #[test]
    fn gen_range_f32_at_upper_bound() {
        let mut rng = Rng::from_seed(3333);
        let mut found_near_one = false;
        for _ in 0..ITER_MEDIUM {
            let val = rng.gen_range(0.0_f32..1.0);
            if val > 0.99 {
                found_near_one = true;
            }
        }
        assert!(
            found_near_one,
            "gen_range f32 should generate values near 1.0"
        );
    }

    #[test]
    fn gen_range_f32_never_reaches_upper_bound() {
        let mut rng = Rng::from_seed(4444);
        for _ in 0..ITER_LARGE {
            let val = rng.gen_range(0.0_f32..1.0);
            assert!(val < 1.0, "gen_range f32 should never equal upper bound");
        }
    }

    // --- gen_bool probability boundaries ---

    #[test]
    fn gen_bool_at_zero_probability_always_false() {
        let mut rng = Rng::from_seed(5555);
        for _ in 0..ITER_SMALL {
            assert!(!rng.gen_bool(0.0), "gen_bool(0.0) must always return false");
        }
    }

    #[test]
    fn gen_bool_at_one_probability_always_true() {
        let mut rng = Rng::from_seed(6666);
        for _ in 0..ITER_SMALL {
            assert!(rng.gen_bool(1.0), "gen_bool(1.0) must always return true");
        }
    }

    #[test]
    fn gen_bool_at_half_probability_has_variance() {
        let mut rng = Rng::from_seed(7777);
        let trues = (0..1000).filter(|_| rng.gen_bool(0.5)).count();
        // With p=0.5, expect ~500 trues, allow variance
        assert!(
            trues > 400 && trues < 600,
            "gen_bool(0.5) should have ~50% true, got {}%",
            trues / 10
        );
    }

    #[test]
    fn gen_bool_at_near_one_mostly_true() {
        let mut rng = Rng::from_seed(8888);
        let trues = (0..1000).filter(|_| rng.gen_bool(0.99)).count();
        assert!(
            trues > 950,
            "gen_bool(0.99) should be mostly true, got {}%",
            trues / 10
        );
    }

    #[test]
    fn gen_bool_at_near_zero_mostly_false() {
        let mut rng = Rng::from_seed(9999);
        let trues = (0..1000).filter(|_| rng.gen_bool(0.01)).count();
        assert!(
            trues < 50,
            "gen_bool(0.01) should be mostly false, got {}%",
            trues / 10
        );
    }

    // --- Events len/is_empty boundaries ---

    #[test]
    fn events_len_at_zero() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let events = Events::new();
        assert_eq!(events.len::<TestEvent>(), 0, "New events should have len 0");
    }

    #[test]
    fn events_len_at_one() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let mut events = Events::new();
        events.send(TestEvent);
        assert_eq!(
            events.len::<TestEvent>(),
            1,
            "After one send, len should be 1"
        );
    }

    #[test]
    fn events_is_empty_at_boundary() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let mut events = Events::new();
        assert!(
            events.is_empty::<TestEvent>(),
            "Empty events is_empty should be true"
        );

        events.send(TestEvent);
        assert!(
            !events.is_empty::<TestEvent>(),
            "Non-empty events is_empty should be false"
        );

        events.clear::<TestEvent>();
        assert!(
            events.is_empty::<TestEvent>(),
            "After clear, is_empty should be true"
        );
    }

    // --- Frame counter boundary ---

    #[test]
    fn frame_counter_starts_at_zero() {
        let events = Events::new();
        assert_eq!(events.current_frame(), 0, "New events should have frame 0");
    }

    #[test]
    fn frame_counter_at_one_after_update() {
        let mut events = Events::new();
        events.update();
        assert_eq!(
            events.current_frame(),
            1,
            "After one update, frame should be 1"
        );
    }

    // --- Entity ID boundaries ---

    #[test]
    fn first_entity_has_expected_id() {
        let mut world = World::new();
        let e = world.spawn();
        // First entity should have a low ID (typically 0 or 1)
        assert!(
            e.id() < 10,
            "First entity should have low ID, got {}",
            e.id()
        );
    }

    #[test]
    fn entity_ids_increment() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        assert!(
            e2.id() > e1.id(),
            "Second entity ID should be greater than first"
        );
    }
}

// =============================================================================
// COMPARISON OPERATOR TESTS - Test to catch == vs != and < vs > swaps
// =============================================================================

mod comparison_operator_tests {
    use super::*;

    // --- Entity equality ---

    #[test]
    fn entity_equal_when_same() {
        let mut world = World::new();
        let e = world.spawn();
        assert_eq!(e, e, "Entity should equal itself");
    }

    #[test]
    fn entity_not_equal_when_different() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        assert_ne!(e1, e2, "Different entities should not be equal");
    }

    // --- Rng seed comparison ---

    #[test]
    fn same_seed_produces_same_first_value() {
        let mut rng1 = Rng::from_seed(42);
        let mut rng2 = Rng::from_seed(42);
        assert_eq!(
            rng1.gen_u32(),
            rng2.gen_u32(),
            "Same seed should produce same first value"
        );
    }

    #[test]
    fn different_seeds_produce_different_first_value() {
        let mut rng1 = Rng::from_seed(1);
        let mut rng2 = Rng::from_seed(2);
        assert_ne!(
            rng1.gen_u32(),
            rng2.gen_u32(),
            "Different seeds should produce different first value"
        );
    }

    // --- Frame comparison ---

    #[test]
    fn frame_equal_to_self() {
        let events = Events::new();
        let frame = events.current_frame();
        assert_eq!(frame, 0, "Frame should equal 0 initially");
        assert!(frame == 0, "Frame comparison should work");
    }

    #[test]
    fn frame_not_equal_after_update() {
        let mut events = Events::new();
        let frame_before = events.current_frame();
        events.update();
        let frame_after = events.current_frame();
        assert_ne!(
            frame_before, frame_after,
            "Frame should change after update"
        );
    }

    // --- Component presence comparison ---

    #[test]
    fn component_present_vs_absent() {
        #[derive(Clone)]
        struct TestComp;

        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();

        world.insert(e1, TestComp);

        assert!(
            world.get::<TestComp>(e1).is_some(),
            "e1 should have TestComp"
        );
        assert!(
            world.get::<TestComp>(e2).is_none(),
            "e2 should NOT have TestComp"
        );
    }

    // --- Alive vs dead entity comparison ---

    #[test]
    fn alive_entity_is_alive() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e), "Spawned entity should be alive");
    }

    #[test]
    fn dead_entity_is_not_alive() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e);
        assert!(!world.is_alive(e), "Despawned entity should not be alive");
    }
}

// =============================================================================
// BOOLEAN RETURN PATH TESTS - Test all paths through boolean-returning functions
// =============================================================================

mod boolean_return_path_tests {
    use super::*;

    // --- is_alive() all paths ---

    #[test]
    fn is_alive_returns_true_for_spawned() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
    }

    #[test]
    fn is_alive_returns_false_for_despawned() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e);
        assert!(!world.is_alive(e));
    }

    #[test]
    fn is_alive_returns_false_for_invalid_entity() {
        let world = World::new();
        // Create a null entity (not spawned in this world)
        let invalid = Entity::null();
        assert!(!world.is_alive(invalid));
    }

    // --- gen_bool() paths ---

    #[test]
    fn gen_bool_can_return_true() {
        let mut rng = Rng::from_seed(12345);
        let any_true = (0..1000).any(|_| rng.gen_bool(0.5));
        assert!(any_true, "gen_bool(0.5) should return true at least once");
    }

    #[test]
    fn gen_bool_can_return_false() {
        let mut rng = Rng::from_seed(12345);
        let any_false = (0..1000).any(|_| !rng.gen_bool(0.5));
        assert!(any_false, "gen_bool(0.5) should return false at least once");
    }

    // --- is_empty() paths ---

    #[test]
    fn events_is_empty_returns_true_for_empty() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let events = Events::new();
        assert!(events.is_empty::<TestEvent>());
    }

    #[test]
    fn events_is_empty_returns_false_for_non_empty() {
        #[derive(Debug, Clone)]
        struct TestEvent;
        impl Event for TestEvent {}

        let mut events = Events::new();
        events.send(TestEvent);
        assert!(!events.is_empty::<TestEvent>());
    }

    // --- Option return paths for get() ---

    #[test]
    fn get_returns_some_when_component_exists() {
        #[derive(Clone)]
        struct TestComp {
            value: i32,
        }

        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, TestComp { value: 42 });

        let result = world.get::<TestComp>(e);
        assert!(
            result.is_some(),
            "get should return Some when component exists"
        );
        assert_eq!(result.unwrap().value, 42);
    }

    #[test]
    fn get_returns_none_when_component_missing() {
        #[derive(Clone)]
        struct TestComp;
        #[derive(Clone)]
        struct OtherComp;

        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, OtherComp);

        let result = world.get::<TestComp>(e);
        assert!(
            result.is_none(),
            "get should return None when component missing"
        );
    }

    #[test]
    fn get_returns_none_when_entity_dead() {
        #[derive(Clone)]
        struct TestComp;

        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, TestComp);
        world.despawn(e);

        let result = world.get::<TestComp>(e);
        assert!(
            result.is_none(),
            "get should return None when entity is dead"
        );
    }

    // --- has_component() paths (if method exists) ---

    #[test]
    fn component_present_after_insert() {
        #[derive(Clone)]
        struct Marker;

        let mut world = World::new();
        let e = world.spawn();

        assert!(
            world.get::<Marker>(e).is_none(),
            "Marker absent before insert"
        );
        world.insert(e, Marker);
        assert!(
            world.get::<Marker>(e).is_some(),
            "Marker present after insert"
        );
    }

    #[test]
    fn component_absent_after_remove() {
        #[derive(Clone)]
        struct Marker;

        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Marker);

        assert!(
            world.get::<Marker>(e).is_some(),
            "Marker present before remove"
        );
        world.remove::<Marker>(e);
        assert!(
            world.get::<Marker>(e).is_none(),
            "Marker absent after remove"
        );
    }
}
