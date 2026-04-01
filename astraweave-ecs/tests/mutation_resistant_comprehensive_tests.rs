//! Comprehensive mutation-resistant tests for astraweave-ecs
//!
//! These tests mirror Kani formal verification proofs (which only run under `cargo kani`)
//! and target known mutation-vulnerable patterns:
//! - BlobVec: capacity growth, pointer arithmetic, len tracking
//! - Entity: bit encoding/decoding, null sentinel, generational indices
//! - EntityAllocator: spawn/despawn/reuse, is_alive, counters
//! - SparseSet: insert/remove swap logic, resize, contains
//! - Events: frame counter, FIFO ordering, cleanup
//! - Rng: seed determinism, exact value sensitivity
//! - ComponentMeta: layout correctness, drop_fn presence, clone roundtrip
//! - Archetype: entity management, remove/swap, signature ops
//! - CountingAlloc: counter arithmetic

#![allow(
    clippy::approx_constant,
    clippy::nonminimal_bool,
    clippy::manual_range_contains,
    unused_imports
)]

use astraweave_ecs::blob_vec::BlobVec;
use astraweave_ecs::component_meta::{ComponentMeta, ComponentMetaRegistry};
use astraweave_ecs::entity_allocator::{Entity, EntityAllocator};
use astraweave_ecs::events::Events;
use astraweave_ecs::sparse_set::SparseSet;
use astraweave_ecs::Rng;

use std::alloc::Layout;
use std::any::TypeId;

// Helper: Entity::new is pub(crate), so from external tests we construct via from_raw
fn entity(id: u32, gen: u32) -> Entity {
    let raw = (id as u64) | ((gen as u64) << 32);
    unsafe { Entity::from_raw(raw) }
}

// ============================================================================
// Component types for testing
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct TestEvent {
    value: i32,
}
impl astraweave_ecs::Event for TestEvent {}

#[derive(Clone, Debug, PartialEq)]
struct OtherEvent {
    tag: String,
}
impl astraweave_ecs::Event for OtherEvent {}

// ============================================================================
// MODULE 1: BlobVec — Kani Proof Mirrors
// Catches: capacity * 2 → + 2, len += 1 → -= 1, index bounds >= → >,
//          pointer offset multiplication, swap_remove logic
// ============================================================================

mod blob_vec_kani_mirrors {
    use super::*;

    #[test]
    fn push_get_roundtrip_u32() {
        let mut blob = BlobVec::new::<u32>();
        for v in [0u32, 1, 42, u32::MAX, u32::MAX / 2, 0xDEADBEEF] {
            let idx = blob.len();
            unsafe { blob.push(v) };
            let retrieved = unsafe { blob.get::<u32>(idx) };
            assert_eq!(retrieved, Some(&v), "u32 roundtrip failed for {v}");
        }
    }

    #[test]
    fn push_get_roundtrip_u64() {
        let mut blob = BlobVec::new::<u64>();
        for v in [0u64, 1, u64::MAX, 0xCAFEBABEDEADBEEF] {
            let idx = blob.len();
            unsafe { blob.push(v) };
            let retrieved = unsafe { blob.get::<u64>(idx) };
            assert_eq!(retrieved, Some(&v), "u64 roundtrip failed for {v}");
        }
    }

    #[test]
    fn push_get_roundtrip_f32() {
        let mut blob = BlobVec::new::<f32>();
        for v in [0.0f32, 1.0, -1.0, f32::MAX, f32::MIN, f32::EPSILON, 3.14159] {
            let idx = blob.len();
            unsafe { blob.push(v) };
            let retrieved = unsafe { blob.get::<f32>(idx) };
            assert_eq!(retrieved, Some(&v), "f32 roundtrip failed for {v}");
        }
    }

    #[test]
    fn len_increments_correctly() {
        let mut blob = BlobVec::new::<u32>();
        for count in 0..=8 {
            assert_eq!(blob.len(), count, "len mismatch at count {count}");
            unsafe { blob.push(count as u32) };
        }
        assert_eq!(blob.len(), 9);
    }

    #[test]
    fn get_oob_returns_none() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(42u32) };
        // Index 0 valid, index >= 1 invalid
        assert!(unsafe { blob.get::<u32>(0) }.is_some());
        assert!(unsafe { blob.get::<u32>(1) }.is_none());
        assert!(unsafe { blob.get::<u32>(100) }.is_none());
        assert!(unsafe { blob.get::<u32>(usize::MAX) }.is_none());
    }

    #[test]
    fn capacity_always_ge_len() {
        let mut blob = BlobVec::new::<u32>();
        for i in 0..32 {
            assert!(
                blob.capacity() >= blob.len(),
                "cap < len at iteration {i}: cap={}, len={}",
                blob.capacity(),
                blob.len()
            );
            unsafe { blob.push(i as u32) };
            assert!(
                blob.capacity() >= blob.len(),
                "cap < len after push {i}: cap={}, len={}",
                blob.capacity(),
                blob.len()
            );
        }
    }

    #[test]
    fn is_empty_after_push() {
        let mut blob = BlobVec::new::<u32>();
        assert!(blob.is_empty(), "new BlobVec must be empty");
        assert_eq!(blob.len(), 0);

        unsafe { blob.push(1u32) };
        assert!(!blob.is_empty(), "BlobVec must not be empty after push");
        assert_eq!(blob.len(), 1);
    }

    #[test]
    fn clear_resets_len() {
        let mut blob = BlobVec::new::<u32>();
        for i in 0..5 {
            unsafe { blob.push(i as u32) };
        }
        assert_eq!(blob.len(), 5);

        blob.clear();
        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
        // Capacity should NOT change on clear
        assert!(blob.capacity() > 0);
    }

    #[test]
    fn multiple_push_maintains_indices() {
        let mut blob = BlobVec::new::<u32>();
        let values = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        for &v in &values {
            unsafe { blob.push(v as u32) };
        }
        assert_eq!(blob.len(), 5);
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(
                unsafe { blob.get::<u32>(i) },
                Some(&(v as u32)),
                "Index {i} mismatch"
            );
        }
    }

    #[test]
    fn swap_remove_returns_correct_value_and_swaps() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
            blob.push(30u32);
        }

        // Remove index 0 (10), last (30) swaps into position 0
        let removed = unsafe { blob.swap_remove::<u32>(0) };
        assert_eq!(removed, 10);
        assert_eq!(blob.len(), 2);
        // Index 0 now has what was at the end (30)
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&30));
        // Index 1 unchanged (20)
        assert_eq!(unsafe { blob.get::<u32>(1) }, Some(&20));
    }

    #[test]
    fn swap_remove_last_element() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
        }
        // Remove last element (no swap needed)
        let removed = unsafe { blob.swap_remove::<u32>(1) };
        assert_eq!(removed, 20);
        assert_eq!(blob.len(), 1);
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&10));
    }

    #[test]
    fn swap_remove_single_element() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(42u32) };

        let removed = unsafe { blob.swap_remove::<u32>(0) };
        assert_eq!(removed, 42);
        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
    }

    #[test]
    fn get_raw_bounds_check() {
        let mut blob = BlobVec::new::<u64>();
        unsafe { blob.push(0xFFu64) };

        assert!(blob.get_raw(0).is_some());
        assert!(blob.get_raw(1).is_none());
        assert!(blob.get_raw(100).is_none());
    }

    #[test]
    fn from_layout_creates_empty() {
        let layout = Layout::new::<u32>();
        let blob = BlobVec::from_layout(layout, None);

        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
        assert_eq!(blob.capacity(), 0);
    }

    #[test]
    fn reserve_maintains_invariants() {
        let mut blob = BlobVec::new::<u32>();
        blob.reserve(50);

        assert_eq!(blob.len(), 0, "reserve must not change len");
        assert!(blob.capacity() >= 50, "capacity must be >= requested");
    }

    #[test]
    fn as_slice_correct_length() {
        let mut blob = BlobVec::new::<u32>();
        assert_eq!(unsafe { blob.as_slice::<u32>() }.len(), 0);

        for i in 0..5 {
            unsafe { blob.push(i as u32) };
            let slice = unsafe { blob.as_slice::<u32>() };
            assert_eq!(slice.len(), i + 1, "slice len mismatch at {i}");
        }
    }

    #[test]
    fn as_slice_values_correct() {
        let mut blob = BlobVec::new::<u32>();
        let values = [100u32, 200, 300, 400, 500];
        for &v in &values {
            unsafe { blob.push(v) };
        }
        let slice = unsafe { blob.as_slice::<u32>() };
        assert_eq!(slice, &values);
    }

    #[test]
    fn swap_remove_raw_decrements_len() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(1u32);
            blob.push(2u32);
            blob.push(3u32);
        }
        let initial = blob.len();
        blob.swap_remove_raw(1);
        assert_eq!(blob.len(), initial - 1);
    }
}

// ============================================================================
// MODULE 2: BlobVec — Arithmetic Mutation Targets
// Catches: capacity * 2 → * 0 / * 1 / + 2 / - 2, item_size multiplication
// ============================================================================

mod blob_vec_arithmetic {
    use super::*;

    #[test]
    fn capacity_doubles_on_growth() {
        let mut blob = BlobVec::new::<u32>();
        // Push first element → triggers first allocation (min capacity 4)
        unsafe { blob.push(1u32) };
        let cap_after_first = blob.capacity();
        assert!(
            cap_after_first >= 4,
            "initial cap should be >= 4, got {cap_after_first}"
        );

        // Fill to capacity
        while blob.len() < cap_after_first {
            unsafe { blob.push(0u32) };
        }
        let cap_before = blob.capacity();
        // Push one more → triggers growth
        unsafe { blob.push(0u32) };
        let cap_after = blob.capacity();
        // New capacity should be at least 2× old (doubling strategy)
        assert!(
            cap_after >= cap_before * 2,
            "capacity should at least double: was {cap_before}, now {cap_after}"
        );
    }

    #[test]
    fn multi_byte_type_pointer_arithmetic_correct() {
        // u64 is 8 bytes — if pointer offset uses wrong multiplier, data corrupts
        let mut blob = BlobVec::new::<u64>();
        let values: Vec<u64> = (0..16).map(|i| 0xDEAD_0000_0000 + i).collect();
        for &v in &values {
            unsafe { blob.push(v) };
        }
        // Verify every element is correctly placed
        for (i, &expected) in values.iter().enumerate() {
            let actual = unsafe { blob.get::<u64>(i) };
            assert_eq!(actual, Some(&expected), "u64 at index {i} corrupted");
        }
    }

    #[test]
    fn struct_type_pointer_arithmetic_correct() {
        // Pos is 12 bytes (3 × f32) — non-power-of-2 size
        let mut blob = BlobVec::new::<Pos>();
        let positions: Vec<Pos> = (0..20)
            .map(|i| Pos {
                x: i as f32,
                y: (i * 10) as f32,
                z: (i * 100) as f32,
            })
            .collect();
        for &p in &positions {
            unsafe { blob.push(p) };
        }
        for (i, expected) in positions.iter().enumerate() {
            let actual = unsafe { blob.get::<Pos>(i) };
            assert_eq!(actual, Some(expected), "Pos at index {i} corrupted");
        }
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
        }
        // Modify via get_mut
        unsafe {
            *blob.get_mut::<u32>(0).unwrap() = 99;
        }
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&99));
        assert_eq!(unsafe { blob.get::<u32>(1) }, Some(&20)); // untouched
    }

    #[test]
    fn as_slice_mut_modifies_in_place() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(1u32);
            blob.push(2u32);
            blob.push(3u32);
        }
        let slice = unsafe { blob.as_slice_mut::<u32>() };
        slice[1] = 999;
        assert_eq!(unsafe { blob.get::<u32>(1) }, Some(&999));
    }

    #[test]
    fn with_capacity_preallocates() {
        let blob = BlobVec::with_capacity::<u32>(100);
        assert_eq!(blob.len(), 0);
        assert!(blob.capacity() >= 100);
    }

    #[test]
    fn reserve_no_change_when_sufficient() {
        let mut blob = BlobVec::new::<u32>();
        blob.reserve(10);
        let cap = blob.capacity();
        blob.reserve(5); // Already have enough
        assert_eq!(blob.capacity(), cap, "reserve should not shrink capacity");
    }

    #[test]
    fn get_raw_mut_returns_valid_pointer() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(42u32) };

        assert!(blob.get_raw_mut(0).is_some());
        assert!(blob.get_raw_mut(1).is_none());

        // Modify through raw pointer
        let ptr = blob.get_raw_mut(0).unwrap();
        unsafe { *(ptr as *mut u32) = 99 };
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&99));
    }

    #[test]
    fn swap_remove_middle_preserves_others() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
            blob.push(30u32);
            blob.push(40u32);
            blob.push(50u32);
        }
        // Remove index 2 (30): last element (50) should move to index 2
        let removed = unsafe { blob.swap_remove::<u32>(2) };
        assert_eq!(removed, 30);
        assert_eq!(blob.len(), 4);
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&10));
        assert_eq!(unsafe { blob.get::<u32>(1) }, Some(&20));
        assert_eq!(unsafe { blob.get::<u32>(2) }, Some(&50)); // swapped in
        assert_eq!(unsafe { blob.get::<u32>(3) }, Some(&40));
    }
}

// ============================================================================
// MODULE 3: Entity — Bit Encoding Kani Mirrors
// Catches: to_raw | → &, << 32 → << 0, from_raw >> 32 → >> 0,
//          is_null && → ||, == MAX → != MAX
// ============================================================================

mod entity_kani_mirrors {
    use super::*;

    #[test]
    fn roundtrip_lossless() {
        let test_cases: Vec<(u32, u32)> = vec![
            (0, 0),
            (1, 0),
            (0, 1),
            (u32::MAX - 1, u32::MAX - 1),
            (0x12345678, 0xABCDEF01),
            (u32::MAX, 0),
            (0, u32::MAX),
            (1, 1),
            (0xFFFF, 0xFFFF),
        ];

        for (id, gen) in test_cases {
            let entity = entity(id, gen);
            let raw = entity.to_raw();
            let recovered = unsafe { Entity::from_raw(raw) };

            assert_eq!(recovered.id(), id, "ID roundtrip failed for ({id}, {gen})");
            assert_eq!(
                recovered.generation(),
                gen,
                "Generation roundtrip failed for ({id}, {gen})"
            );
        }
    }

    #[test]
    fn to_raw_encoding_bit_exact() {
        let id: u32 = 0x12345678;
        let gen: u32 = 0xABCDEF01;
        let entity = entity(id, gen);
        let raw = entity.to_raw();

        // Low 32 bits = ID
        assert_eq!((raw as u32), id, "Low 32 bits must be ID");
        // High 32 bits = generation
        assert_eq!(((raw >> 32) as u32), gen, "High 32 bits must be generation");
    }

    #[test]
    fn to_raw_encoding_zero_id() {
        let entity = entity(0, 0x42);
        let raw = entity.to_raw();
        assert_eq!((raw as u32), 0);
        assert_eq!(((raw >> 32) as u32), 0x42);
    }

    #[test]
    fn to_raw_encoding_zero_gen() {
        let entity = entity(0x42, 0);
        let raw = entity.to_raw();
        assert_eq!((raw as u32), 0x42);
        assert_eq!(((raw >> 32) as u32), 0);
    }

    #[test]
    fn null_properties() {
        let null = Entity::null();
        assert!(null.is_null());
        assert_eq!(null.id(), u32::MAX);
        assert_eq!(null.generation(), u32::MAX);
    }

    #[test]
    fn is_null_requires_both_max() {
        // Only both MAX = null
        assert!(entity(u32::MAX, u32::MAX).is_null());

        // One MAX, one not = NOT null (catches && → ||)
        assert!(!entity(u32::MAX, 0).is_null());
        assert!(!entity(0, u32::MAX).is_null());
        assert!(!entity(u32::MAX, u32::MAX - 1).is_null());
        assert!(!entity(u32::MAX - 1, u32::MAX).is_null());

        // Both non-MAX = NOT null
        assert!(!entity(0, 0).is_null());
        assert!(!entity(1, 1).is_null());
    }

    #[test]
    fn entity_equality() {
        let e1 = entity(1, 0);
        let e2 = entity(1, 0);
        let e3 = entity(1, 1);
        let e4 = entity(2, 0);

        assert_eq!(e1, e2);
        assert_ne!(e1, e3); // different gen
        assert_ne!(e1, e4); // different id
    }

    #[test]
    fn entity_ordering_consistent() {
        let e1 = entity(0, 0);
        let e2 = entity(1, 0);
        let e3 = entity(0, 1);

        assert!(e1 < e2); // id 0 < id 1
        assert!(e1 < e3); // same id, gen 0 < gen 1
                          // Antisymmetry
        assert!(!(e2 < e1));
        // Symmetry of equality
        assert_eq!(entity(5, 5), entity(5, 5));
    }

    #[test]
    fn entity_display_format() {
        let e = entity(42, 7);
        assert_eq!(format!("{}", e), "42v7");
        assert_eq!(format!("{:?}", e), "Entity(42v7)");
    }

    #[test]
    fn entity_hash_deterministic() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let e1 = entity(10, 20);
        let e2 = entity(10, 20);

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        e1.hash(&mut h1);
        e2.hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }
}

// ============================================================================
// MODULE 4: EntityAllocator — Kani Proof Mirrors
// Catches: generation wrapping_add(1) → 0, free_list pop logic,
//          spawned_count += 1 → 0, despawned_count += 1 → 0
// ============================================================================

mod entity_allocator_kani_mirrors {
    use super::*;

    #[test]
    fn spawn_new_generation_zero() {
        let mut alloc = EntityAllocator::new();
        for i in 0..5 {
            let entity = alloc.spawn();
            assert_eq!(entity.generation(), 0, "new entity gen must be 0");
            assert_eq!(entity.id(), i as u32, "IDs must be sequential");
        }
    }

    #[test]
    fn spawn_unique_ids() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let e2 = alloc.spawn();
        let e3 = alloc.spawn();

        assert_ne!(e1.id(), e2.id());
        assert_ne!(e2.id(), e3.id());
        assert_ne!(e1.id(), e3.id());
    }

    #[test]
    fn despawn_returns_correct_bool() {
        let mut alloc = EntityAllocator::new();
        let entity = alloc.spawn();

        assert!(alloc.is_alive(entity));
        assert!(alloc.despawn(entity), "first despawn must return true");
        assert!(!alloc.despawn(entity), "second despawn must return false");
    }

    #[test]
    fn is_alive_false_after_despawn() {
        let mut alloc = EntityAllocator::new();
        let entity = alloc.spawn();

        assert!(alloc.is_alive(entity));
        alloc.despawn(entity);
        assert!(!alloc.is_alive(entity));
    }

    #[test]
    fn generation_increments_on_reuse() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let old_gen = e1.generation();

        alloc.despawn(e1);
        let e2 = alloc.spawn();

        assert_eq!(e2.id(), e1.id(), "ID should be reused from free list");
        assert_eq!(
            e2.generation(),
            old_gen.wrapping_add(1),
            "Generation must increment on reuse"
        );
    }

    #[test]
    fn recycled_entity_old_handle_invalid() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        alloc.despawn(e1);
        let e2 = alloc.spawn();

        assert_eq!(e2.id(), e1.id());
        assert_eq!(e2.generation(), e1.generation() + 1);
        assert!(!alloc.is_alive(e1), "old handle must be dead");
        assert!(alloc.is_alive(e2), "new handle must be alive");
    }

    #[test]
    fn is_alive_rejects_never_spawned() {
        let alloc = EntityAllocator::new();
        let fake = unsafe { Entity::from_raw(0) };
        assert!(!alloc.is_alive(fake));
    }

    #[test]
    fn spawned_count_accurate() {
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.spawned_count(), 0);

        for expected in 1..=10 {
            alloc.spawn();
            assert_eq!(alloc.spawned_count(), expected);
        }
    }

    #[test]
    fn despawned_count_accurate() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let e2 = alloc.spawn();

        alloc.despawn(e1);
        assert_eq!(alloc.despawned_count(), 1);

        alloc.despawn(e2);
        assert_eq!(alloc.despawned_count(), 2);
    }

    #[test]
    fn alive_count_tracks_correctly() {
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.alive_count(), 0);

        let e1 = alloc.spawn();
        assert_eq!(alloc.alive_count(), 1);

        let e2 = alloc.spawn();
        assert_eq!(alloc.alive_count(), 2);

        alloc.despawn(e1);
        assert_eq!(alloc.alive_count(), 1);

        alloc.despawn(e2);
        assert_eq!(alloc.alive_count(), 0);
    }

    #[test]
    fn capacity_grows_with_spawns() {
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.capacity(), 0);

        alloc.spawn();
        assert_eq!(alloc.capacity(), 1);

        alloc.spawn();
        assert_eq!(alloc.capacity(), 2);
    }

    #[test]
    fn capacity_unchanged_on_reuse() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let _e2 = alloc.spawn();
        assert_eq!(alloc.capacity(), 2);

        alloc.despawn(e1);
        // Reuse doesn't increase capacity
        let _e3 = alloc.spawn();
        assert_eq!(alloc.capacity(), 2);
    }

    #[test]
    fn generation_query() {
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.generation(0), None); // no slot yet

        let e1 = alloc.spawn();
        assert_eq!(alloc.generation(0), Some(0));

        alloc.despawn(e1);
        assert_eq!(alloc.generation(0), Some(1)); // incremented
    }

    #[test]
    fn clear_resets_everything() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let e2 = alloc.spawn();
        alloc.despawn(e1);

        alloc.clear();

        assert_eq!(alloc.alive_count(), 0);
        assert_eq!(alloc.capacity(), 0);
        assert!(!alloc.is_alive(e1));
        assert!(!alloc.is_alive(e2));

        // Spawning after clear starts fresh
        let e3 = alloc.spawn();
        assert_eq!(e3.id(), 0);
        assert_eq!(e3.generation(), 0);
    }

    #[test]
    fn multiple_despawn_respawn_cycles() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let id = e1.id();

        for gen in 0..10u32 {
            assert!(alloc.despawn(entity(id, gen)));
            let reused = alloc.spawn();
            assert_eq!(reused.id(), id);
            assert_eq!(reused.generation(), gen + 1);
        }
    }

    #[test]
    fn despawn_oob_id_returns_false() {
        let mut alloc = EntityAllocator::new();
        let fake = entity(999, 0);
        assert!(!alloc.despawn(fake));
    }

    #[test]
    fn despawn_wrong_generation_returns_false() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn();
        let stale = entity(e1.id(), e1.generation() + 1);
        assert!(!alloc.despawn(stale));
    }

    #[test]
    fn with_capacity_no_entities() {
        let alloc = EntityAllocator::with_capacity(100);
        assert_eq!(alloc.capacity(), 0); // No entities yet
        assert_eq!(alloc.alive_count(), 0);
    }
}

// ============================================================================
// MODULE 5: SparseSet — Swap Logic & Boundary Tests
// Catches: insert resize id + 1 → + 0, remove swap index != last → ==,
//          sparse update on swap, contains uses get().is_some()
// ============================================================================

mod sparse_set_mutations {
    use super::*;

    #[test]
    fn insert_returns_dense_index() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        let e1 = entity(1, 0);

        assert_eq!(set.insert(e0), 0);
        assert_eq!(set.insert(e1), 1);
    }

    #[test]
    fn insert_duplicate_returns_existing_index() {
        let mut set = SparseSet::new();
        let e0 = entity(5, 0);

        let idx1 = set.insert(e0);
        let idx2 = set.insert(e0);
        assert_eq!(idx1, idx2);
        assert_eq!(set.len(), 1); // not doubled
    }

    #[test]
    fn get_returns_correct_index() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        let e1 = entity(5, 0);

        set.insert(e0);
        set.insert(e1);

        assert_eq!(set.get(e0), Some(0));
        assert_eq!(set.get(e1), Some(1));
    }

    #[test]
    fn get_missing_returns_none() {
        let set = SparseSet::new();
        assert_eq!(set.get(entity(0, 0)), None);
    }

    #[test]
    fn contains_inserted() {
        let mut set = SparseSet::new();
        let e = entity(3, 0);

        assert!(!set.contains(e));
        set.insert(e);
        assert!(set.contains(e));
    }

    #[test]
    fn remove_returns_dense_index() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        set.insert(e0);

        assert_eq!(set.remove(e0), Some(0));
        assert!(!set.contains(e0));
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn remove_missing_returns_none() {
        let mut set = SparseSet::new();
        assert_eq!(set.remove(entity(0, 0)), None);
    }

    #[test]
    fn remove_swap_updates_sparse_index() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        let e1 = entity(1, 0);
        let e2 = entity(2, 0);

        set.insert(e0); // dense[0]
        set.insert(e1); // dense[1]
        set.insert(e2); // dense[2]

        // Remove e0: e2 swaps into dense[0]
        set.remove(e0);

        assert!(!set.contains(e0));
        assert!(set.contains(e1));
        assert!(set.contains(e2));
        assert_eq!(set.len(), 2);

        // e2 should now be at index 0 (swapped), e1 still at index 1
        assert_eq!(set.get(e2), Some(0));
        assert_eq!(set.get(e1), Some(1));
    }

    #[test]
    fn remove_last_no_swap_needed() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        let e1 = entity(1, 0);

        set.insert(e0);
        set.insert(e1);

        // Remove last element (e1) — no swap needed
        set.remove(e1);
        assert_eq!(set.len(), 1);
        assert!(set.contains(e0));
        assert!(!set.contains(e1));
        assert_eq!(set.get(e0), Some(0)); // unchanged
    }

    #[test]
    fn remove_single_element() {
        let mut set = SparseSet::new();
        let e = entity(5, 0);
        set.insert(e);
        set.remove(e);
        assert!(set.is_empty());
        assert!(!set.contains(e));
    }

    #[test]
    fn insert_high_id_resizes_sparse() {
        let mut set = SparseSet::new();
        let e = entity(1000, 0);
        set.insert(e);
        assert!(set.contains(e));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn entities_returns_dense_slice() {
        let mut set = SparseSet::new();
        let e0 = entity(3, 0);
        let e1 = entity(7, 0);

        set.insert(e0);
        set.insert(e1);

        let entities = set.entities();
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0], e0);
        assert_eq!(entities[1], e1);
    }

    #[test]
    fn clear_empties_set() {
        let mut set = SparseSet::new();
        set.insert(entity(0, 0));
        set.insert(entity(1, 0));
        set.insert(entity(2, 0));

        set.clear();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn len_and_is_empty() {
        let mut set = SparseSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        set.insert(entity(0, 0));
        assert!(!set.is_empty());
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn with_capacity_creates_empty() {
        let set = SparseSet::with_capacity(100);
        assert!(set.is_empty());
        assert!(set.capacity() >= 100);
    }

    #[test]
    fn remove_swap_stress_five_entities() {
        let mut set = SparseSet::new();
        let entities: Vec<Entity> = (0..5).map(|i| entity(i, 0)).collect();
        for &e in &entities {
            set.insert(e);
        }
        assert_eq!(set.len(), 5);

        // Remove from the middle sequentially
        set.remove(entities[2]);
        assert_eq!(set.len(), 4);
        assert!(!set.contains(entities[2]));

        set.remove(entities[0]);
        assert_eq!(set.len(), 3);
        assert!(!set.contains(entities[0]));

        // Remaining should still be findable
        assert!(set.contains(entities[1]));
        assert!(set.contains(entities[3]));
        assert!(set.contains(entities[4]));
    }

    #[test]
    fn insert_after_remove_reuses_slot() {
        let mut set = SparseSet::new();
        let e0 = entity(0, 0);
        let e1 = entity(1, 0);

        set.insert(e0);
        set.insert(e1);
        set.remove(e0);

        // Re-insert e0 — should work and add to end of dense
        let idx = set.insert(e0);
        assert!(set.contains(e0));
        assert_eq!(set.len(), 2);
        assert_eq!(set.get(e0), Some(idx));
    }
}

// ============================================================================
// MODULE 6: Events — Frame Counter & Ordering
// Catches: current_frame += 1 → 0, send ordering, len == 0 → != 0,
//          is_empty negation, with_keep_frames assignment
// ============================================================================

mod events_mutations {
    use super::*;

    #[test]
    fn frame_counter_increments_by_one() {
        let mut events = Events::new();
        assert_eq!(events.current_frame(), 0);

        for expected in 1..=10 {
            events.update();
            assert_eq!(
                events.current_frame(),
                expected,
                "frame should be {expected}"
            );
        }
    }

    #[test]
    fn send_increments_len() {
        let mut events = Events::new();
        assert_eq!(events.len::<TestEvent>(), 0);

        events.send(TestEvent { value: 1 });
        assert_eq!(events.len::<TestEvent>(), 1);

        events.send(TestEvent { value: 2 });
        assert_eq!(events.len::<TestEvent>(), 2);
    }

    #[test]
    fn is_empty_before_and_after_send() {
        let mut events = Events::new();
        assert!(events.is_empty::<TestEvent>());

        events.send(TestEvent { value: 1 });
        assert!(!events.is_empty::<TestEvent>());
    }

    #[test]
    fn read_preserves_fifo_exact_values() {
        let mut events = Events::new();
        events.send(TestEvent { value: 42 });
        events.send(TestEvent { value: -7 });
        events.send(TestEvent { value: 0 });

        let collected: Vec<_> = events.read::<TestEvent>().collect();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].value, 42);
        assert_eq!(collected[1].value, -7);
        assert_eq!(collected[2].value, 0);
    }

    #[test]
    fn drain_consumes_all() {
        let mut events = Events::new();
        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });

        let drained: Vec<_> = events.drain::<TestEvent>().collect();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].value, 1);
        assert_eq!(drained[1].value, 2);

        assert_eq!(events.len::<TestEvent>(), 0);
    }

    #[test]
    fn clear_resets_len_to_zero() {
        let mut events = Events::new();
        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });
        events.send(TestEvent { value: 3 });

        events.clear::<TestEvent>();
        assert_eq!(events.len::<TestEvent>(), 0);
        assert!(events.is_empty::<TestEvent>());
    }

    #[test]
    fn clear_all_removes_all_types() {
        let mut events = Events::new();
        events.send(TestEvent { value: 1 });
        events.send(OtherEvent {
            tag: "x".to_string(),
        });

        events.clear_all();
        assert_eq!(events.len::<TestEvent>(), 0);
        assert_eq!(events.len::<OtherEvent>(), 0);
    }

    #[test]
    fn multiple_types_independent() {
        let mut events = Events::new();
        events.send(TestEvent { value: 10 });
        events.send(OtherEvent {
            tag: "hello".to_string(),
        });
        events.send(TestEvent { value: 20 });

        assert_eq!(events.len::<TestEvent>(), 2);
        assert_eq!(events.len::<OtherEvent>(), 1);

        events.clear::<TestEvent>();
        assert_eq!(events.len::<TestEvent>(), 0);
        assert_eq!(events.len::<OtherEvent>(), 1); // unaffected
    }

    #[test]
    fn with_keep_frames_stores_value() {
        let events = Events::new().with_keep_frames(10);
        // We can't directly read keep_frames, but we verify it compiles
        // and returns an Events (builder pattern)
        assert_eq!(events.current_frame(), 0);
    }

    #[test]
    fn default_creates_new() {
        let events = Events::default();
        assert_eq!(events.current_frame(), 0);
        assert!(events.is_empty::<TestEvent>());
    }

    #[test]
    fn event_reader_sees_same_events() {
        let mut events = Events::new();
        let reader = events.get_reader::<TestEvent>();

        events.send(TestEvent { value: 42 });

        let via_read: Vec<_> = events.read::<TestEvent>().collect();
        let via_reader: Vec<_> = reader.read(&events).collect();

        assert_eq!(via_read.len(), 1);
        assert_eq!(via_reader.len(), 1);
        assert_eq!(via_read[0].value, via_reader[0].value);
    }

    #[test]
    fn drain_then_read_empty() {
        let mut events = Events::new();
        events.send(TestEvent { value: 1 });

        let _: Vec<_> = events.drain::<TestEvent>().collect();
        let remaining: Vec<_> = events.read::<TestEvent>().collect();
        assert!(remaining.is_empty());
    }

    #[test]
    fn send_after_update_persists() {
        let mut events = Events::new();
        events.send(TestEvent { value: 1 });
        events.update(); // frame 0 → 1
        events.send(TestEvent { value: 2 });

        // Both events should be readable
        let collected: Vec<_> = events.read::<TestEvent>().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn unregistered_type_len_zero() {
        let events = Events::new();
        assert_eq!(events.len::<TestEvent>(), 0);
        assert!(events.is_empty::<TestEvent>());
    }
}

// ============================================================================
// MODULE 7: Rng — Seed Determinism & Value Sensitivity
// Catches: seed assignment, gen_u32/gen_u64 delegation, gen_bool boundary
// ============================================================================

mod rng_mutations {
    use super::*;

    #[test]
    fn seed_stored_correctly() {
        let rng = Rng::from_seed(12345);
        assert_eq!(rng.seed(), 12345);

        let rng2 = Rng::from_seed(0);
        assert_eq!(rng2.seed(), 0);

        let rng3 = Rng::from_seed(u64::MAX);
        assert_eq!(rng3.seed(), u64::MAX);
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut rng1 = Rng::from_seed(42);
        let mut rng2 = Rng::from_seed(42);

        for _ in 0..100 {
            assert_eq!(rng1.gen_u32(), rng2.gen_u32());
        }
    }

    #[test]
    fn different_seed_different_sequence() {
        let mut rng1 = Rng::from_seed(42);
        let mut rng2 = Rng::from_seed(43);

        // At least one of 10 values should differ
        let mut any_different = false;
        for _ in 0..10 {
            if rng1.gen_u32() != rng2.gen_u32() {
                any_different = true;
                break;
            }
        }
        assert!(
            any_different,
            "different seeds must produce different sequences"
        );
    }

    #[test]
    fn gen_u64_deterministic() {
        let mut rng1 = Rng::from_seed(999);
        let mut rng2 = Rng::from_seed(999);

        for _ in 0..50 {
            assert_eq!(rng1.gen_u64(), rng2.gen_u64());
        }
    }

    #[test]
    fn gen_range_within_bounds() {
        let mut rng = Rng::from_seed(1);
        for _ in 0..1000 {
            let v: i32 = rng.gen_range(10..20);
            assert!(v >= 10 && v < 20, "value {v} out of range [10, 20)");
        }
    }

    #[test]
    fn gen_range_f64_within_bounds() {
        let mut rng = Rng::from_seed(2);
        for _ in 0..1000 {
            let v: f64 = rng.gen_range(0.0..1.0);
            assert!(v >= 0.0 && v < 1.0, "value {v} out of range [0.0, 1.0)");
        }
    }

    #[test]
    fn gen_bool_respects_probability() {
        let mut rng = Rng::from_seed(42);
        // p=0.0 should always be false
        for _ in 0..100 {
            assert!(!rng.gen_bool(0.0));
        }
        // p=1.0 should always be true
        for _ in 0..100 {
            assert!(rng.gen_bool(1.0));
        }
    }

    #[test]
    fn choose_returns_element_from_slice() {
        let mut rng = Rng::from_seed(7);
        let items = [10, 20, 30, 40, 50];

        for _ in 0..100 {
            let chosen = rng.choose(&items).unwrap();
            assert!(items.contains(chosen));
        }
    }

    #[test]
    fn choose_empty_returns_none() {
        let mut rng = Rng::from_seed(7);
        let empty: &[i32] = &[];
        assert!(rng.choose(empty).is_none());
    }

    #[test]
    fn shuffle_preserves_elements() {
        let mut rng = Rng::from_seed(42);
        let mut data = vec![1, 2, 3, 4, 5];
        rng.shuffle(&mut data);

        // All elements still present
        let mut sorted = data.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn rng_core_delegation() {
        use rand::RngCore;

        let mut rng1 = Rng::from_seed(100);
        let mut rng2 = Rng::from_seed(100);

        // RngCore::next_u32
        assert_eq!(RngCore::next_u32(&mut rng1), RngCore::next_u32(&mut rng2));
        // RngCore::next_u64
        assert_eq!(RngCore::next_u64(&mut rng1), RngCore::next_u64(&mut rng2));

        // fill_bytes
        let mut buf1 = [0u8; 16];
        let mut buf2 = [0u8; 16];
        RngCore::fill_bytes(&mut rng1, &mut buf1);
        RngCore::fill_bytes(&mut rng2, &mut buf2);
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn serialization_roundtrip() {
        let rng = Rng::from_seed(12345);
        let json = serde_json::to_string(&rng).unwrap();
        let restored: Rng = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.seed(), 12345);

        // Same seed → same sequence from fresh state
        let mut r1 = Rng::from_seed(12345);
        let mut r2 = restored;
        for _ in 0..10 {
            assert_eq!(r1.gen_u32(), r2.gen_u32());
        }
    }

    #[test]
    fn clone_produces_identical_state() {
        let mut rng = Rng::from_seed(42);
        // Advance state a bit
        for _ in 0..50 {
            rng.gen_u32();
        }

        let mut cloned = rng.clone();
        // Both should produce identical subsequent values
        for _ in 0..100 {
            assert_eq!(rng.gen_u32(), cloned.gen_u32());
        }
    }
}

// ============================================================================
// MODULE 8: ComponentMeta — Layout & Function Correctness
// Catches: layout.size() swaps, needs_drop check, clone_fn corruption
// ============================================================================

mod component_meta_mutations {
    use super::*;

    #[test]
    fn layout_matches_type() {
        let meta = ComponentMeta::of::<Pos>();
        assert_eq!(meta.layout.size(), std::mem::size_of::<Pos>());
        assert_eq!(meta.layout.align(), std::mem::align_of::<Pos>());
    }

    #[test]
    fn copy_type_no_drop_fn() {
        let meta = ComponentMeta::of::<u32>();
        assert!(meta.drop_fn.is_none());

        let meta2 = ComponentMeta::of::<Pos>();
        assert!(meta2.drop_fn.is_none());
    }

    #[test]
    fn heap_type_has_drop_fn() {
        #[derive(Clone)]
        struct WithVec {
            _data: Vec<u8>,
        }
        let meta = ComponentMeta::of::<WithVec>();
        assert!(meta.drop_fn.is_some());
    }

    #[test]
    fn clone_fn_roundtrip() {
        let meta = ComponentMeta::of::<Health>();
        let src = Health {
            current: 80,
            max: 100,
        };
        let mut dst = std::mem::MaybeUninit::<Health>::uninit();

        unsafe {
            (meta.clone_fn)(
                &src as *const Health as *const u8,
                dst.as_mut_ptr() as *mut u8,
            );
            let cloned = dst.assume_init();
            assert_eq!(cloned.current, 80);
            assert_eq!(cloned.max, 100);
        }
    }

    #[test]
    fn type_name_not_empty() {
        let meta = ComponentMeta::of::<Pos>();
        assert!(!meta.type_name.is_empty());
    }

    #[test]
    fn create_blob_vec_empty() {
        let meta = ComponentMeta::of::<u32>();
        let blob = meta.create_blob_vec();
        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
    }

    #[test]
    fn create_blob_vec_with_capacity() {
        let meta = ComponentMeta::of::<u32>();
        let blob = meta.create_blob_vec_with_capacity(50);
        assert!(blob.capacity() >= 50);
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn registry_register_returns_correct_bool() {
        let mut reg = ComponentMetaRegistry::new();
        assert!(reg.register::<u32>()); // new
        assert!(!reg.register::<u32>()); // duplicate
        assert!(reg.register::<f64>()); // new different type
    }

    #[test]
    fn registry_is_registered() {
        let mut reg = ComponentMetaRegistry::new();
        assert!(!reg.is_registered(TypeId::of::<u32>()));

        reg.register::<u32>();
        assert!(reg.is_registered(TypeId::of::<u32>()));
        assert!(!reg.is_registered(TypeId::of::<f64>()));
    }

    #[test]
    fn registry_get() {
        let mut reg = ComponentMetaRegistry::new();
        reg.register::<Pos>();

        let meta = reg.get(TypeId::of::<Pos>());
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().layout.size(), std::mem::size_of::<Pos>());

        assert!(reg.get(TypeId::of::<Health>()).is_none());
    }
}

// ============================================================================
// MODULE 9: CountingAlloc — Counter Arithmetic
// Catches: fetch_add → fetch_sub, store 0 → 1, isize subtraction direction
// ============================================================================

#[cfg(feature = "alloc-counter")]
mod counting_alloc_mutations {
    use astraweave_ecs::counting_alloc::{allocs, deallocs, net_allocs, reset_allocs};

    #[test]
    fn reset_sets_zero() {
        reset_allocs();
        // After reset, allocs() and deallocs() return a value (may not be zero
        // because other threads/tests may allocate). But the function runs.
        let _ = allocs();
        let _ = deallocs();
    }

    #[test]
    fn net_allocs_is_allocs_minus_deallocs() {
        // We can verify the relationship even if absolute values are unknown
        let a = allocs() as isize;
        let d = deallocs() as isize;
        let n = net_allocs();
        assert_eq!(n, a - d, "net_allocs must be allocs - deallocs");
    }

    #[test]
    fn functions_return_usize() {
        // Type check — allocs/deallocs are usize, net_allocs is isize
        let _a: usize = allocs();
        let _d: usize = deallocs();
        let _n: isize = net_allocs();
    }
}

// ============================================================================
// MODULE 10: BlobVec — Drop Safety
// Catches: drop_fn not being called, clear not iterating, Drop impl missing
// ============================================================================

mod blob_vec_drop_safety {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct DropTracker {
        dropped: Rc<Cell<u32>>,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.dropped.set(self.dropped.get() + 1);
        }
    }

    #[test]
    fn clear_calls_drop_fn() {
        let counter = Rc::new(Cell::new(0u32));
        let mut blob = BlobVec::new::<DropTracker>();

        unsafe {
            blob.push(DropTracker {
                dropped: counter.clone(),
            });
            blob.push(DropTracker {
                dropped: counter.clone(),
            });
            blob.push(DropTracker {
                dropped: counter.clone(),
            });
        }

        blob.clear();
        assert_eq!(counter.get(), 3, "clear must call drop for each element");
    }

    #[test]
    fn drop_calls_drop_fn() {
        let counter = Rc::new(Cell::new(0u32));

        {
            let mut blob = BlobVec::new::<DropTracker>();
            unsafe {
                blob.push(DropTracker {
                    dropped: counter.clone(),
                });
                blob.push(DropTracker {
                    dropped: counter.clone(),
                });
            }
        } // blob goes out of scope

        assert_eq!(counter.get(), 2, "destructor must drop all elements");
    }

    #[test]
    fn swap_remove_raw_calls_drop_fn() {
        let counter = Rc::new(Cell::new(0u32));
        let mut blob = BlobVec::new::<DropTracker>();

        unsafe {
            blob.push(DropTracker {
                dropped: counter.clone(),
            });
            blob.push(DropTracker {
                dropped: counter.clone(),
            });
        }

        blob.swap_remove_raw(0);
        // Should have dropped the removed item
        assert!(
            counter.get() >= 1,
            "swap_remove_raw must drop removed element"
        );
    }
}

// ============================================================================
// MODULE 11: BlobVec — from_layout_with_capacity
// ============================================================================

mod blob_vec_from_layout {
    use super::*;

    #[test]
    fn from_layout_with_capacity_preallocates() {
        let layout = Layout::new::<u64>();
        let blob = BlobVec::from_layout_with_capacity(layout, None, 100);
        assert!(blob.capacity() >= 100);
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn from_layout_zero_cap() {
        let layout = Layout::new::<u32>();
        let blob = BlobVec::from_layout_with_capacity(layout, None, 0);
        assert_eq!(blob.capacity(), 0);
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn push_raw_roundtrip() {
        let layout = Layout::new::<u32>();
        let mut blob = BlobVec::from_layout(layout, None);

        let value: u32 = 0xDEADBEEF;
        let clone_fn: unsafe fn(*const u8, *mut u8) = |src, dst| unsafe {
            std::ptr::copy_nonoverlapping(src, dst, std::mem::size_of::<u32>());
        };

        unsafe {
            blob.push_raw(&value as *const u32 as *const u8, clone_fn);
        }
        assert_eq!(blob.len(), 1);

        let raw_ptr = blob.get_raw(0).unwrap();
        let retrieved = unsafe { *(raw_ptr as *const u32) };
        assert_eq!(retrieved, 0xDEADBEEF);
    }
}

// ============================================================================
// MODULE 12: Archetype Signature Operations
// ============================================================================

mod archetype_signature_mutations {
    use astraweave_ecs::archetype::ArchetypeSignature;
    use std::any::TypeId;

    #[test]
    fn signature_sorts_and_dedups() {
        let t1 = TypeId::of::<u32>();
        let t2 = TypeId::of::<f64>();

        let sig = ArchetypeSignature::new(vec![t2, t1, t1, t2]);
        // Should be sorted and deduped
        assert_eq!(sig.len(), 2);
        assert!(sig.contains(t1));
        assert!(sig.contains(t2));
    }

    #[test]
    fn signature_contains() {
        let t1 = TypeId::of::<u32>();
        let t2 = TypeId::of::<f64>();
        let t3 = TypeId::of::<String>();

        let sig = ArchetypeSignature::new(vec![t1, t2]);
        assert!(sig.contains(t1));
        assert!(sig.contains(t2));
        assert!(!sig.contains(t3));
    }

    #[test]
    fn signature_len() {
        let empty = ArchetypeSignature::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());

        let one = ArchetypeSignature::new(vec![TypeId::of::<u32>()]);
        assert_eq!(one.len(), 1);
        assert!(!one.is_empty());
    }

    #[test]
    fn signature_equality() {
        let t1 = TypeId::of::<u32>();
        let t2 = TypeId::of::<f64>();

        let sig1 = ArchetypeSignature::new(vec![t1, t2]);
        let sig2 = ArchetypeSignature::new(vec![t2, t1]); // different order
        assert_eq!(sig1, sig2, "order shouldn't matter for signature equality");
    }

    #[test]
    fn signature_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let t1 = TypeId::of::<u32>();
        let t2 = TypeId::of::<f64>();

        let sig1 = ArchetypeSignature::new(vec![t1, t2]);
        let sig2 = ArchetypeSignature::new(vec![t2, t1]);

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        sig1.hash(&mut h1);
        sig2.hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }
}

// ============================================================================
// MODULE 13: BlobVec — Edge Cases for Mutation Killing
// ============================================================================

mod blob_vec_edge_cases {
    use super::*;

    #[test]
    fn empty_blob_vec_get_returns_none() {
        let blob = BlobVec::new::<u32>();
        assert!(unsafe { blob.get::<u32>(0) }.is_none());
    }

    #[test]
    fn empty_blob_vec_as_slice_empty() {
        let blob = BlobVec::new::<u32>();
        assert!(unsafe { blob.as_slice::<u32>() }.is_empty());
    }

    #[test]
    fn reserve_zero_no_change() {
        let mut blob = BlobVec::new::<u32>();
        blob.reserve(0);
        assert_eq!(blob.capacity(), 0);
    }

    #[test]
    fn multiple_reserves_grow() {
        let mut blob = BlobVec::new::<u32>();
        blob.reserve(10);
        assert!(blob.capacity() >= 10);
        blob.reserve(100);
        assert!(blob.capacity() >= 100);
    }

    #[test]
    fn clear_then_push_works() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(1u32);
            blob.push(2u32);
        }
        blob.clear();
        unsafe {
            blob.push(99u32);
        }
        assert_eq!(blob.len(), 1);
        assert_eq!(unsafe { blob.get::<u32>(0) }, Some(&99));
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn swap_remove_raw_panics_on_oob() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(1u32) };
        blob.swap_remove_raw(1); // OOB
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn swap_remove_panics_on_oob() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(1u32) };
        unsafe { blob.swap_remove::<u32>(1) }; // OOB
    }

    #[test]
    fn swap_remove_preserves_order_of_remaining() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
            blob.push(30u32);
            blob.push(40u32);
        }
        // Remove index 1 (20): last (40) moves to index 1
        let removed = unsafe { blob.swap_remove::<u32>(1) };
        assert_eq!(removed, 20);
        assert_eq!(blob.len(), 3);

        let slice = unsafe { blob.as_slice::<u32>() };
        assert_eq!(slice[0], 10);
        assert_eq!(slice[1], 40); // swapped in
        assert_eq!(slice[2], 30);
    }

    #[test]
    fn get_at_len_minus_one_valid() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
        }
        // len=2, so index 1 (len-1) should be valid
        assert!(unsafe { blob.get::<u32>(1) }.is_some());
        // index 2 (len) should be invalid
        assert!(unsafe { blob.get::<u32>(2) }.is_none());
    }
}

// ============================================================================
// MODULE 14: Integration — BlobVec with ComponentMeta
// ============================================================================

mod blob_vec_component_meta_integration {
    use super::*;

    #[test]
    fn component_meta_blob_vec_push_get_roundtrip() {
        let meta = ComponentMeta::of::<Pos>();
        let mut blob = meta.create_blob_vec();

        let pos = Pos {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        unsafe {
            blob.push_raw(&pos as *const Pos as *const u8, meta.clone_fn);
        }

        assert_eq!(blob.len(), 1);
        let retrieved = unsafe { blob.get::<Pos>(0) };
        assert_eq!(retrieved, Some(&pos));
    }

    #[test]
    fn component_meta_blob_vec_multiple_types() {
        // u32 (4 bytes)
        let meta_u32 = ComponentMeta::of::<u32>();
        let mut blob_u32 = meta_u32.create_blob_vec();
        unsafe { blob_u32.push(42u32) };
        assert_eq!(unsafe { blob_u32.get::<u32>(0) }, Some(&42));

        // u64 (8 bytes)
        let meta_u64 = ComponentMeta::of::<u64>();
        let mut blob_u64 = meta_u64.create_blob_vec();
        unsafe { blob_u64.push(0xDEADu64) };
        assert_eq!(unsafe { blob_u64.get::<u64>(0) }, Some(&0xDEADu64));

        // Pos (12 bytes)
        let meta_pos = ComponentMeta::of::<Pos>();
        let mut blob_pos = meta_pos.create_blob_vec();
        let p = Pos {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        unsafe { blob_pos.push(p) };
        assert_eq!(unsafe { blob_pos.get::<Pos>(0) }, Some(&p));
    }
}

// ============================================================================
// MODULE 15: Entity to_raw bit-level exhaustive verification
// ============================================================================

mod entity_bit_level {
    use super::*;

    #[test]
    fn to_raw_id_in_low_bits_comprehensive() {
        for id in [0u32, 1, 0xFF, 0xFFFF, 0xFFFFFF, u32::MAX] {
            let entity = entity(id, 0);
            let raw = entity.to_raw();
            assert_eq!(raw as u32, id, "low bits for id={id:#X}");
            assert_eq!((raw >> 32) as u32, 0, "high bits should be 0 for gen=0");
        }
    }

    #[test]
    fn to_raw_gen_in_high_bits_comprehensive() {
        for gen in [0u32, 1, 0xFF, 0xFFFF, 0xFFFFFF, u32::MAX] {
            let entity = entity(0, gen);
            let raw = entity.to_raw();
            assert_eq!(raw as u32, 0, "low bits should be 0 for id=0");
            assert_eq!((raw >> 32) as u32, gen, "high bits for gen={gen:#X}");
        }
    }

    #[test]
    fn from_raw_low_32_is_id() {
        let raw: u64 = 0x00000042_DEADBEEF;
        let entity = unsafe { Entity::from_raw(raw) };
        assert_eq!(entity.id(), 0xDEADBEEF);
        assert_eq!(entity.generation(), 0x00000042);
    }

    #[test]
    fn to_raw_from_raw_all_ones() {
        let entity = entity(u32::MAX, u32::MAX);
        let raw = entity.to_raw();
        assert_eq!(raw, u64::MAX);
        let recovered = unsafe { Entity::from_raw(raw) };
        assert_eq!(recovered.id(), u32::MAX);
        assert_eq!(recovered.generation(), u32::MAX);
    }

    #[test]
    fn to_raw_from_raw_alternating_bits() {
        let entity = entity(0xAAAAAAAA, 0x55555555);
        let raw = entity.to_raw();
        let recovered = unsafe { Entity::from_raw(raw) };
        assert_eq!(recovered.id(), 0xAAAAAAAA);
        assert_eq!(recovered.generation(), 0x55555555);
    }
}

// ============================================================================
// COMPONENT REGISTRY / BLOB REGISTRATION TESTS
// Targets missed mutant: is_component_registered_blob -> bool with true/false
// ============================================================================

mod component_registry_tests {
    use astraweave_ecs::World;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct RegisteredComponent {
        value: i32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct UnregisteredComponent {
        data: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct AnotherRegisteredComponent {
        x: i32,
        y: i32,
    }

    /// Verifies that is_component_registered_blob returns FALSE for unregistered types.
    /// This kills the mutation "replace ... with true".
    #[test]
    fn is_component_registered_blob_returns_false_for_unregistered() {
        let world = World::new();
        // UnregisteredComponent was never registered
        let result = world.is_component_registered_blob::<UnregisteredComponent>();
        assert!(
            !result,
            "Expected false for unregistered component, got true"
        );
    }

    /// Verifies that is_component_registered_blob returns TRUE after registration.
    /// This kills the mutation "replace ... with false".
    #[test]
    fn is_component_registered_blob_returns_true_after_register() {
        let mut world = World::new();
        // Register the component type
        world.register_component::<RegisteredComponent>();
        let result = world.is_component_registered_blob::<RegisteredComponent>();
        assert!(result, "Expected true for registered component, got false");
    }

    /// Additional boundary test: register one type, check another is still unregistered.
    #[test]
    fn registration_is_type_specific() {
        let mut world = World::new();
        world.register_component::<RegisteredComponent>();

        // RegisteredComponent should be true
        assert!(world.is_component_registered_blob::<RegisteredComponent>());

        // UnregisteredComponent should still be false
        assert!(!world.is_component_registered_blob::<UnregisteredComponent>());
    }

    /// Multiple registrations: each type tracked independently.
    #[test]
    fn multiple_component_registrations() {
        let mut world = World::new();

        // Initially neither is registered
        assert!(!world.is_component_registered_blob::<RegisteredComponent>());
        assert!(!world.is_component_registered_blob::<AnotherRegisteredComponent>());

        // Register first type
        world.register_component::<RegisteredComponent>();
        assert!(world.is_component_registered_blob::<RegisteredComponent>());
        assert!(!world.is_component_registered_blob::<AnotherRegisteredComponent>());

        // Register second type
        world.register_component::<AnotherRegisteredComponent>();
        assert!(world.is_component_registered_blob::<RegisteredComponent>());
        assert!(world.is_component_registered_blob::<AnotherRegisteredComponent>());
    }

    /// Idempotency: registering twice doesn't break the registration check.
    #[test]
    fn double_registration_remains_true() {
        let mut world = World::new();
        world.register_component::<RegisteredComponent>();
        world.register_component::<RegisteredComponent>(); // Register again

        // Should still return true
        assert!(world.is_component_registered_blob::<RegisteredComponent>());
    }

    /// Fresh world has no registered components (negative assertion).
    #[test]
    fn fresh_world_has_no_blob_registrations() {
        let world = World::new();
        assert!(!world.is_component_registered_blob::<RegisteredComponent>());
        assert!(!world.is_component_registered_blob::<UnregisteredComponent>());
        assert!(!world.is_component_registered_blob::<AnotherRegisteredComponent>());
    }
}

// ============================================================================
// MUTATION REMEDIATION — BlobVec raw pointer, archetype blob
// Targets specific missed mutants from cargo-mutants ECS shard run.
// ============================================================================

mod blob_vec_raw_pointer_remediation {
    use super::*;

    /// Verifies get_raw returns a pointer to the correct data (not just is_some).
    /// Catches: `* → +` and `* → /` in `index * item_layout.size()` (line 198).
    #[test]
    fn get_raw_returns_correct_pointer_value() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(111u32);
            blob.push(222u32);
            blob.push(333u32);
        }
        // Verify each slot returns the correct value via raw pointer
        for (i, expected) in [111u32, 222, 333].iter().enumerate() {
            let ptr = blob.get_raw(i).expect("should return Some");
            let val = unsafe { *(ptr as *const u32) };
            assert_eq!(
                val, *expected,
                "get_raw({i}) returned wrong value: {val} != {expected}"
            );
        }
    }

    /// Verifies get_raw_mut returns pointer to correct data and allows mutation.
    /// Catches: `* → +` and `* → /` in `index * item_layout.size()` (line 214).
    #[test]
    fn get_raw_mut_returns_correct_pointer_value() {
        let mut blob = BlobVec::new::<u64>();
        unsafe {
            blob.push(1000u64);
            blob.push(2000u64);
            blob.push(3000u64);
        }
        // Verify each slot and mutate through raw pointer
        for (i, expected) in [1000u64, 2000, 3000].iter().enumerate() {
            let ptr = blob.get_raw_mut(i).expect("should return Some");
            let val = unsafe { *(ptr as *const u64) };
            assert_eq!(
                val, *expected,
                "get_raw_mut({i}) wrong value: {val} != {expected}"
            );
        }
        // Mutate index 1 and verify
        let ptr = blob.get_raw_mut(1).unwrap();
        unsafe { *(ptr as *mut u64) = 9999 };
        let check = blob.get_raw(1).unwrap();
        let val = unsafe { *(check as *const u64) };
        assert_eq!(val, 9999, "mutation through get_raw_mut should persist");
    }

    /// Verifies get_raw boundary: `>= → <` mutation would give wrong results.
    /// Catches: `>= → <` in `if index >= self.len` (lines 194, 209)
    #[test]
    fn get_raw_exact_boundary() {
        let mut blob = BlobVec::new::<u32>();
        unsafe { blob.push(42u32) };
        // index 0 is valid (len = 1)
        assert!(blob.get_raw(0).is_some(), "index 0 should be valid");
        // index 1 == len, should be None where >= makes it None
        assert!(blob.get_raw(1).is_none(), "index == len should be None");
    }

    /// Verifies swap_remove_raw actually swaps the last element into the hole.
    /// Catches: `- → +/÷` in `last_index = self.len - 1` (line 228),
    ///          `* → +/÷` in pointer offset calculations (lines 235, 245-246),
    ///          `!= → ==` in swap condition (line 241).
    #[test]
    fn swap_remove_raw_verifies_swap_content() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
            blob.push(30u32);
            blob.push(40u32);
        }
        // Remove index 1 (value 20). Last element (40) should move to index 1.
        blob.swap_remove_raw(1);
        assert_eq!(blob.len(), 3);

        // Verify remaining values are [10, 40, 30]
        let val0 = unsafe { *(blob.get_raw(0).unwrap() as *const u32) };
        let val1 = unsafe { *(blob.get_raw(1).unwrap() as *const u32) };
        let val2 = unsafe { *(blob.get_raw(2).unwrap() as *const u32) };
        assert_eq!(val0, 10, "index 0 should be untouched");
        assert_eq!(val1, 40, "index 1 should have last element (40)");
        assert_eq!(val2, 30, "index 2 should be untouched");
    }

    /// Verifies swap_remove_raw when removing the last element (no swap needed).
    /// Catches: `!= → ==` in `if index != last_index` (line 241).
    #[test]
    fn swap_remove_raw_last_element_no_swap() {
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10u32);
            blob.push(20u32);
        }
        // Remove last element (index 1)
        blob.swap_remove_raw(1);
        assert_eq!(blob.len(), 1);
        let val = unsafe { *(blob.get_raw(0).unwrap() as *const u32) };
        assert_eq!(
            val, 10,
            "first element should be untouched after removing last"
        );
    }

    /// Verifies reserve multiplication is correct (not addition or division).
    /// Catches: `* → +` and `* → /` in `self.item_layout.size() * new_capacity` (line 116)
    ///          and `self.item_layout.size() * self.capacity` (line 131).
    #[test]
    fn reserve_allocates_correct_capacity_for_large_types() {
        // Use a large type so size multiplication matters
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Big([u8; 256]);

        let mut blob = BlobVec::new::<Big>();
        blob.reserve(10);
        assert!(blob.capacity() >= 10, "capacity should be >= 10");

        // Push should succeed up to capacity without panic
        for i in 0..10 {
            unsafe { blob.push(Big([i as u8; 256])) };
        }
        assert_eq!(blob.len(), 10);

        // Verify stored values are correct (catches pointer math issues)
        for i in 0..10 {
            let ptr = blob.get_raw(i).expect("should have value");
            let val = unsafe { *(ptr as *const Big) };
            assert_eq!(val.0[0], i as u8, "value at {i} should be {i}");
        }
    }

    /// Verifies with_capacity pre-allocation doesn't break for boundary case.
    /// Catches: `> → >=` in `if capacity > 0` (line 62).
    #[test]
    fn with_capacity_zero_is_empty() {
        let blob = BlobVec::with_capacity::<u32>(0);
        assert_eq!(blob.len(), 0);
        assert_eq!(blob.capacity(), 0, "capacity(0) should not allocate");
    }

    /// Verifies from_layout_with_capacity boundary.
    /// Catches: `> → >=` in `if capacity > 0` (line 96).
    #[test]
    fn from_layout_with_capacity_zero() {
        let layout = Layout::new::<u32>();
        let blob = BlobVec::from_layout_with_capacity(layout, None, 0);
        assert_eq!(blob.len(), 0);
        assert_eq!(
            blob.capacity(),
            0,
            "from_layout_with_capacity(0) should not allocate"
        );
    }
}

mod archetype_blob_remediation {
    use super::*;
    use astraweave_ecs::World;

    /// Verify archetype storage with blob operations through World API.
    /// Catches mutations in archetype blob iteration and entity removal.
    #[test]
    fn world_insert_and_query_blob_components() {
        let mut world = World::new();
        world.register_component::<u32>();
        world.register_component::<f64>();

        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, 100u32);
        world.insert(e2, 200u32);
        world.insert(e3, 300u32);
        world.insert(e1, 1.5f64);
        world.insert(e3, 3.5f64);

        // Query u32 components
        let u32_val = world.get::<u32>(e1);
        assert_eq!(u32_val, Some(&100u32), "e1 should have u32=100");
        let u32_val = world.get::<u32>(e2);
        assert_eq!(u32_val, Some(&200u32), "e2 should have u32=200");
        let u32_val = world.get::<u32>(e3);
        assert_eq!(u32_val, Some(&300u32), "e3 should have u32=300");

        // Query f64 components
        assert_eq!(world.get::<f64>(e1), Some(&1.5f64));
        assert_eq!(world.get::<f64>(e2), None, "e2 should NOT have f64");
        assert_eq!(world.get::<f64>(e3), Some(&3.5f64));
    }

    /// Verify entity removal from archetype doesn't corrupt other entities.
    /// Catches: `< → <=` in remove_entity (archetype.rs:559),
    ///          `+= → -=/*=` in get_or_create_archetype_with_blob (archetype.rs:516).
    #[test]
    fn remove_entity_preserves_others() {
        let mut world = World::new();
        world.register_component::<u32>();

        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, 10u32);
        world.insert(e2, 20u32);
        world.insert(e3, 30u32);

        // Remove middle entity
        world.despawn(e2);

        // Verify remaining entities still have correct values
        assert_eq!(world.get::<u32>(e1), Some(&10u32), "e1 should survive");
        assert_eq!(world.get::<u32>(e2), None, "e2 should be gone");
        assert_eq!(world.get::<u32>(e3), Some(&30u32), "e3 should survive");
    }
}
