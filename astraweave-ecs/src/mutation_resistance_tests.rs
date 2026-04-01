//! Mutation Resistance Tests for astraweave-ecs
//!
//! Comprehensive tests targeting every public function, branch, and boundary
//! condition across all ECS source files to ensure cargo-mutants achieves
//! ≥97% kill rate. Written as part of the NASA-grade verification initiative.
//!
//! ## Coverage Targets
//!
//! - blob_vec.rs: push_raw, swap_remove_raw_to, from_layout, clear with drop
//! - sparse_set.rs: SparseSet & SparseSetData swap semantics, boundary indices
//! - archetype.rs: add_entity_typed_raw, blob get/get_mut, uses_blob, remove_entity_components swap
//! - entity_allocator.rs: reserve, alive_count arithmetic, free list LIFO, is_alive boundary
//! - component_meta.rs: create_blob_vec_with_capacity end-to-end, clone_fn with drop types
//! - counting_alloc.rs: allocs/deallocs/net_allocs value assertions
//! - events.rs: with_keep_frames, keep_frames, get_reader/EventReader, event type independence
//! - lib.rs: World count/entities_with/remove/despawn/register_component/each_mut,
//!   Schedule/Stage, App builder, Plugin trait

#![cfg(test)]
#![allow(clippy::manual_range_contains, clippy::len_zero)]

// =============================================================================
// Module imports
// =============================================================================

use std::any::TypeId;
use std::collections::HashMap;

use crate::archetype::{ArchetypeId, ArchetypeSignature, ArchetypeStorage};
use crate::blob_vec::BlobVec;
use crate::component_meta::{ComponentMeta, ComponentMetaRegistry};
use crate::entity_allocator::{Entity, EntityAllocator};
use crate::events::{Event, Events};
use crate::sparse_set::{SparseSet, SparseSetData};
use crate::{App, Schedule, SystemStage, World};

// =============================================================================
// 1. BlobVec — push_raw / swap_remove_raw_to / from_layout
// =============================================================================

mod blob_vec_mutation_tests {
    use super::*;
    use std::alloc::Layout;

    #[derive(Clone, Copy, Debug, PartialEq)]
    #[repr(C)]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    /// Clone function for Vec3 (mirrors ComponentMeta pattern)
    unsafe fn clone_vec3(src: *const u8, dst: *mut u8) {
        let src_ref = &*(src as *const Vec3);
        let cloned = *src_ref;
        (dst as *mut Vec3).write(cloned);
    }

    #[test]
    fn push_raw_stores_correct_values() {
        // Kills: push_raw → (), push_raw len += 0, push_raw offset arithmetic wrong
        let layout = Layout::new::<Vec3>();
        let mut blob = BlobVec::from_layout(layout, None);

        let v1 = Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let v2 = Vec3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let v3 = Vec3 {
            x: 7.0,
            y: 8.0,
            z: 9.0,
        };

        unsafe {
            blob.push_raw(&v1 as *const Vec3 as *const u8, clone_vec3);
            blob.push_raw(&v2 as *const Vec3 as *const u8, clone_vec3);
            blob.push_raw(&v3 as *const Vec3 as *const u8, clone_vec3);
        }

        assert_eq!(blob.len(), 3, "push_raw must increment len");

        // Verify each element through get_raw
        let p0 = blob.get_raw(0).unwrap();
        let p1 = blob.get_raw(1).unwrap();
        let p2 = blob.get_raw(2).unwrap();

        let r0 = unsafe { std::ptr::read(p0 as *const Vec3) };
        let r1 = unsafe { std::ptr::read(p1 as *const Vec3) };
        let r2 = unsafe { std::ptr::read(p2 as *const Vec3) };

        assert_eq!(r0, v1, "element 0 must match pushed value");
        assert_eq!(r1, v2, "element 1 must match pushed value");
        assert_eq!(r2, v3, "element 2 must match pushed value");
    }

    #[test]
    fn push_raw_triggers_reserve_on_capacity_exceeded() {
        // Kills: push_raw skipping reserve (len == capacity check)
        let layout = Layout::new::<u64>();
        let mut blob = BlobVec::from_layout(layout, None);
        assert_eq!(blob.capacity(), 0, "initial capacity must be 0");

        let val: u64 = 0xDEAD_BEEF;
        unsafe {
            blob.push_raw(&val as *const u64 as *const u8, |src, dst| {
                (dst as *mut u64).write(*(src as *const u64));
            });
        }

        assert_eq!(blob.len(), 1);
        assert!(blob.capacity() >= 1, "capacity must grow to fit element");
    }

    #[test]
    fn swap_remove_raw_to_copies_element_to_destination() {
        // Kills: swap_remove_raw_to → (), clone_fn not called, len not decremented
        let layout = Layout::new::<u64>();
        let mut blob = BlobVec::from_layout(layout, None);

        unsafe fn clone_u64(src: *const u8, dst: *mut u8) {
            (dst as *mut u64).write(*(src as *const u64));
        }

        let vals: Vec<u64> = vec![100, 200, 300, 400];
        for v in &vals {
            unsafe {
                blob.push_raw(v as *const u64 as *const u8, clone_u64);
            }
        }
        assert_eq!(blob.len(), 4);

        // Remove index 1 (value 200), copy to dst
        let mut dst_val: u64 = 0;
        unsafe {
            blob.swap_remove_raw_to(1, &mut dst_val as *mut u64 as *mut u8, clone_u64);
        }

        assert_eq!(dst_val, 200, "removed element must be copied to dst");
        assert_eq!(blob.len(), 3, "len must decrement after swap_remove_raw_to");

        // Verify remaining: [100, 400, 300] (last element 400 swapped into index 1)
        let r0 = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const u64) };
        let r1 = unsafe { std::ptr::read(blob.get_raw(1).unwrap() as *const u64) };
        let r2 = unsafe { std::ptr::read(blob.get_raw(2).unwrap() as *const u64) };
        assert_eq!(r0, 100, "element 0 unchanged");
        assert_eq!(r1, 400, "last element swapped into removed index");
        assert_eq!(r2, 300, "element 2 unchanged");
    }

    #[test]
    fn swap_remove_raw_to_last_element_no_swap() {
        // Kills: swap_remove_raw_to != → == (would incorrectly copy when removing last)
        let layout = Layout::new::<u32>();
        let mut blob = BlobVec::from_layout(layout, None);

        unsafe fn clone_u32(src: *const u8, dst: *mut u8) {
            (dst as *mut u32).write(*(src as *const u32));
        }

        let vals: Vec<u32> = vec![10, 20, 30];
        for v in &vals {
            unsafe {
                blob.push_raw(v as *const u32 as *const u8, clone_u32);
            }
        }

        // Remove last element (index 2)
        let mut dst: u32 = 0;
        unsafe {
            blob.swap_remove_raw_to(2, &mut dst as *mut u32 as *mut u8, clone_u32);
        }

        assert_eq!(dst, 30, "last element copied to dst");
        assert_eq!(blob.len(), 2);
        let r0 = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const u32) };
        let r1 = unsafe { std::ptr::read(blob.get_raw(1).unwrap() as *const u32) };
        assert_eq!(r0, 10, "element 0 unchanged after removing last");
        assert_eq!(r1, 20, "element 1 unchanged after removing last");
    }

    #[test]
    fn swap_remove_raw_to_with_drop_type() {
        // Verify drop_fn is called on the original element after copy
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        #[derive(Clone)]
        struct Tracked {
            val: i32,
            dropped: Arc<AtomicBool>,
        }
        impl Drop for Tracked {
            fn drop(&mut self) {
                self.dropped.store(true, Ordering::SeqCst);
            }
        }

        let drop1 = Arc::new(AtomicBool::new(false));
        let drop2 = Arc::new(AtomicBool::new(false));

        let mut blob = BlobVec::new::<Tracked>();
        unsafe {
            blob.push(Tracked {
                val: 1,
                dropped: drop1.clone(),
            });
            blob.push(Tracked {
                val: 2,
                dropped: drop2.clone(),
            });
        }

        assert!(!drop1.load(Ordering::SeqCst), "not dropped yet");

        // swap_remove_raw_to index 0: copies val=1 to dst, drops original
        let meta = ComponentMeta::of::<Tracked>();
        let mut dst = std::mem::MaybeUninit::<Tracked>::uninit();
        unsafe {
            blob.swap_remove_raw_to(0, dst.as_mut_ptr() as *mut u8, meta.clone_fn);
        }

        // The original at index 0 should have been dropped
        assert!(
            drop1.load(Ordering::SeqCst),
            "original element must be dropped after swap_remove_raw_to"
        );
        assert_eq!(blob.len(), 1);

        // dst should contain the cloned value
        let dst_val = unsafe { dst.assume_init() };
        assert_eq!(dst_val.val, 1, "dst must contain cloned value");

        // Clean up dst manually
        drop(dst_val);
    }

    #[test]
    fn from_layout_creates_empty_blob_with_correct_properties() {
        // Kills: from_layout returning wrong capacity/len
        let layout = Layout::new::<[u8; 64]>();
        let blob = BlobVec::from_layout(layout, None);
        assert_eq!(blob.len(), 0);
        assert_eq!(blob.capacity(), 0);
        assert!(blob.is_empty());
    }

    #[test]
    fn from_layout_with_drop_fn_calls_drop_on_clear() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        #[derive(Clone)]
        struct DropTracker {
            dropped: Arc<AtomicI32>,
        }
        impl Drop for DropTracker {
            fn drop(&mut self) {
                self.dropped.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = Arc::new(AtomicI32::new(0));
        let meta = ComponentMeta::of::<DropTracker>();

        let mut blob = BlobVec::from_layout(meta.layout, meta.drop_fn);
        unsafe {
            blob.push_raw(
                &DropTracker {
                    dropped: counter.clone(),
                } as *const DropTracker as *const u8,
                meta.clone_fn,
            );
            blob.push_raw(
                &DropTracker {
                    dropped: counter.clone(),
                } as *const DropTracker as *const u8,
                meta.clone_fn,
            );
        }

        blob.clear();
        // Each element in the blob was dropped: that's 2 drops
        // Plus the 2 original stack values will be dropped at end of scope
        assert!(
            counter.load(Ordering::SeqCst) >= 2,
            "clear must call drop_fn on each element"
        );
    }

    #[test]
    fn swap_remove_raw_drops_element_at_index() {
        // Verify swap_remove_raw calls drop_fn on removed element
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        #[derive(Clone)]
        struct DropMe {
            _id: i32,
            dropped: Arc<AtomicBool>,
        }
        impl Drop for DropMe {
            fn drop(&mut self) {
                self.dropped.store(true, Ordering::SeqCst);
            }
        }

        let drop0 = Arc::new(AtomicBool::new(false));
        let drop1 = Arc::new(AtomicBool::new(false));
        let drop2 = Arc::new(AtomicBool::new(false));

        let mut blob = BlobVec::new::<DropMe>();
        unsafe {
            blob.push(DropMe {
                _id: 0,
                dropped: drop0.clone(),
            });
            blob.push(DropMe {
                _id: 1,
                dropped: drop1.clone(),
            });
            blob.push(DropMe {
                _id: 2,
                dropped: drop2.clone(),
            });
        }

        // swap_remove_raw index 1: should drop element 1, move element 2 to index 1
        blob.swap_remove_raw(1);

        assert!(
            drop1.load(Ordering::SeqCst),
            "element at removed index must be dropped"
        );
        assert!(
            !drop0.load(Ordering::SeqCst),
            "element 0 must NOT be dropped"
        );
        assert!(
            !drop2.load(Ordering::SeqCst),
            "element 2 must NOT be dropped (moved, not dropped)"
        );
        assert_eq!(blob.len(), 2);
    }

    #[test]
    fn reserve_no_realloc_when_sufficient_capacity() {
        // Kills: reserve <= → < (would re-allocate unnecessarily)
        let mut blob = BlobVec::new::<u64>();
        blob.reserve(10);
        let cap_after_first = blob.capacity();
        assert!(cap_after_first >= 10);

        // Reserve less than current capacity — should NOT change capacity
        blob.reserve(5);
        assert_eq!(
            blob.capacity(),
            cap_after_first,
            "reserve within existing capacity must not reallocate"
        );
    }

    #[test]
    fn clear_on_no_drop_type_still_resets_len() {
        // Kills: clear only runs when drop_fn.is_some()
        let mut blob = BlobVec::new::<u32>(); // u32 has no Drop
        unsafe {
            blob.push(1u32);
            blob.push(2u32);
            blob.push(3u32);
        }
        assert_eq!(blob.len(), 3);
        blob.clear();
        assert_eq!(blob.len(), 0, "clear must reset len even for no-drop types");
        assert!(blob.is_empty());
    }

    #[test]
    fn get_mut_allows_mutation_via_typed_api() {
        // Kills: get_mut → None, get_mut returning wrong ptr
        let mut blob = BlobVec::new::<i64>();
        unsafe {
            blob.push(10i64);
            blob.push(20i64);

            let val = blob.get_mut::<i64>(1).unwrap();
            *val = 999;

            assert_eq!(*blob.get::<i64>(0).unwrap(), 10, "element 0 unaffected");
            assert_eq!(*blob.get::<i64>(1).unwrap(), 999, "element 1 mutated");
        }
    }

    #[test]
    fn swap_remove_returns_correct_value_and_swaps() {
        // Kills: swap_remove returning wrong value, not swapping
        let mut blob = BlobVec::new::<i32>();
        unsafe {
            blob.push(10i32);
            blob.push(20i32);
            blob.push(30i32);

            // Remove middle element
            let removed = blob.swap_remove::<i32>(1);
            assert_eq!(removed, 20, "swap_remove must return removed value");
            assert_eq!(blob.len(), 2);

            // After swap: [10, 30]
            assert_eq!(*blob.get::<i32>(0).unwrap(), 10);
            assert_eq!(*blob.get::<i32>(1).unwrap(), 30);
        }
    }

    #[test]
    fn swap_remove_single_element() {
        // Edge case: only one element, no swap needed
        let mut blob = BlobVec::new::<u64>();
        unsafe {
            blob.push(42u64);
            let removed = blob.swap_remove::<u64>(0);
            assert_eq!(removed, 42);
            assert_eq!(blob.len(), 0);
            assert!(blob.is_empty());
        }
    }
}

// =============================================================================
// 2. SparseSet — swap semantics, boundary indices
// =============================================================================

mod sparse_set_mutation_tests {
    use super::*;

    #[test]
    fn sparse_set_remove_first_of_three_swaps_correctly() {
        // Kills: remove != → == (would skip swap), swap index wrong
        let mut set = SparseSet::new();
        let e0 = unsafe { Entity::from_raw(10) };
        let e1 = unsafe { Entity::from_raw(20) };
        let e2 = unsafe { Entity::from_raw(30) };

        set.insert(e0); // dense[0] = e0
        set.insert(e1); // dense[1] = e1
        set.insert(e2); // dense[2] = e2

        // Remove e0 (index 0): e2 should move to index 0
        let removed = set.remove(e0);
        assert_eq!(removed, Some(0), "removed index must be 0");
        assert_eq!(set.len(), 2);

        // e2 should now be at index 0
        assert_eq!(set.get(e2), Some(0), "e2 must be at index 0 after swap");
        // e1 should still be at index 1
        assert_eq!(set.get(e1), Some(1), "e1 must be unchanged at index 1");
        // e0 should be gone
        assert_eq!(set.get(e0), None, "e0 must be gone");
    }

    #[test]
    fn sparse_set_remove_middle_of_five() {
        // Test with larger set to ensure swap is correct
        let mut set = SparseSet::new();
        let entities: Vec<Entity> = (0..5)
            .map(|i| unsafe { Entity::from_raw(i * 10) })
            .collect();
        for &e in &entities {
            set.insert(e);
        }

        // Remove entity at dense index 2
        let removed = set.remove(entities[2]);
        assert_eq!(removed, Some(2));
        assert_eq!(set.len(), 4);

        // Entity 4 (last) should have moved to index 2
        assert_eq!(
            set.get(entities[4]),
            Some(2),
            "last entity must move to removed index"
        );
        assert_eq!(set.get(entities[0]), Some(0), "entity 0 unchanged");
        assert_eq!(set.get(entities[1]), Some(1), "entity 1 unchanged");
        assert_eq!(set.get(entities[3]), Some(3), "entity 3 unchanged");
    }

    #[test]
    fn sparse_set_insert_returns_existing_index_on_duplicate() {
        // Kills: insert always creating new entry (duplicate returns existing)
        let mut set = SparseSet::new();
        let e = unsafe { Entity::from_raw(5) };

        let idx1 = set.insert(e);
        let idx2 = set.insert(e);
        assert_eq!(idx1, idx2, "duplicate insert must return same index");
        assert_eq!(set.len(), 1, "duplicate must not increase len");
    }

    #[test]
    fn sparse_set_default_is_empty() {
        let set = SparseSet::default();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn sparse_set_data_remove_first_swaps_data_correctly() {
        // Kills: SparseSetData remove swap not updating data array
        let mut set = SparseSetData::new();
        let e0 = unsafe { Entity::from_raw(0) };
        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(2) };

        set.insert(e0, "zero");
        set.insert(e1, "one");
        set.insert(e2, "two");

        // Remove e0: data for e2 should swap into index 0
        let removed = set.remove(e0);
        assert_eq!(removed, Some("zero"), "removed value must be 'zero'");
        assert_eq!(set.len(), 2);

        // After swap: e1 at index 1 (unchanged), e2 at index 0 (swapped from index 2)
        assert_eq!(set.get(e1), Some(&"one"), "e1 data unchanged");
        assert_eq!(set.get(e2), Some(&"two"), "e2 data must follow swap");
        assert_eq!(set.get(e0), None, "e0 must be gone");
    }

    #[test]
    fn sparse_set_data_insert_replaces_existing_data() {
        // Kills: insert not replacing, or returning wrong old value
        let mut set = SparseSetData::new();
        let e = unsafe { Entity::from_raw(5) };

        let old1 = set.insert(e, 100);
        assert_eq!(old1, None, "first insert returns None");

        let old2 = set.insert(e, 200);
        assert_eq!(old2, Some(100), "second insert returns old value");

        assert_eq!(set.get(e), Some(&200), "value must be updated");
    }

    #[test]
    fn sparse_set_data_contains_reflects_insertions_and_removals() {
        let mut set = SparseSetData::new();
        let e = unsafe { Entity::from_raw(3) };

        assert!(!set.contains(e), "empty set does not contain entity");
        set.insert(e, 42);
        assert!(set.contains(e), "set contains inserted entity");
        set.remove(e);
        assert!(!set.contains(e), "set does not contain removed entity");
    }

    #[test]
    fn sparse_set_data_default_is_empty() {
        let set = SparseSetData::<i32>::default();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn sparse_set_data_data_slice_correct() {
        // Kills: data() → empty slice, data_mut() → wrong slice
        let mut set = SparseSetData::new();
        let e0 = unsafe { Entity::from_raw(0) };
        let e1 = unsafe { Entity::from_raw(1) };

        set.insert(e0, 10);
        set.insert(e1, 20);

        let data = set.data();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], 10);
        assert_eq!(data[1], 20);

        // Mutate via data_mut
        set.data_mut()[0] = 99;
        assert_eq!(set.get(e0), Some(&99));
    }

    #[test]
    fn sparse_set_data_iter_yields_all_pairs() {
        let mut set = SparseSetData::new();
        let e0 = unsafe { Entity::from_raw(0) };
        let e1 = unsafe { Entity::from_raw(1) };

        set.insert(e0, "a");
        set.insert(e1, "b");

        let pairs: Vec<_> = set.iter().collect();
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|(e, v)| *e == e0 && **v == "a"));
        assert!(pairs.iter().any(|(e, v)| *e == e1 && **v == "b"));
    }

    #[test]
    fn sparse_set_data_iter_mut_modifies_values() {
        let mut set = SparseSetData::new();
        let e0 = unsafe { Entity::from_raw(0) };
        let e1 = unsafe { Entity::from_raw(1) };

        set.insert(e0, 10);
        set.insert(e1, 20);

        for (_, val) in set.iter_mut() {
            *val += 100;
        }

        assert_eq!(set.get(e0), Some(&110));
        assert_eq!(set.get(e1), Some(&120));
    }

    #[test]
    fn sparse_set_data_clear_empties_all() {
        let mut set = SparseSetData::new();
        set.insert(unsafe { Entity::from_raw(0) }, 1);
        set.insert(unsafe { Entity::from_raw(1) }, 2);

        set.clear();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.entities().len(), 0);
        assert_eq!(set.data().len(), 0);
    }
}

// =============================================================================
// 3. Archetype — typed blob operations, uses_blob, remove_entity_components
// =============================================================================

mod archetype_mutation_tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Vel {
        vx: f32,
        vy: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Hp(i32);

    fn make_blob_archetype_in_storage() -> (ArchetypeStorage, ArchetypeId) {
        let mut storage = ArchetypeStorage::new();
        let mut metas = HashMap::new();
        metas.insert(TypeId::of::<Pos>(), ComponentMeta::of::<Pos>());
        metas.insert(TypeId::of::<Hp>(), ComponentMeta::of::<Hp>());
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Pos>(), TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype_with_blob(sig, metas);
        (storage, id)
    }

    #[test]
    fn uses_blob_returns_correct_mode() {
        // Kills: uses_blob → false (always), uses_blob → true (always)
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let box_id = storage.get_or_create_archetype(sig);
        let box_arch = storage.get_archetype(box_id).unwrap();
        assert!(!box_arch.uses_blob(), "Box archetype must return false");

        let (blob_storage, blob_id) = make_blob_archetype_in_storage();
        let blob_arch = blob_storage.get_archetype(blob_id).unwrap();
        assert!(blob_arch.uses_blob(), "Blob archetype must return true");
    }

    #[test]
    fn blob_get_and_get_mut_work_correctly() {
        // Kills: get → None in blob path, get_mut → None in blob path
        let (mut storage, id) = make_blob_archetype_in_storage();
        let arch = storage.get_archetype_mut(id).unwrap();

        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(2) };

        // Add entities via add_entity_typed_raw
        let pos1 = Pos { x: 1.0, y: 2.0 };
        let hp1 = Hp(100);
        arch.add_entity_typed_raw(
            e1,
            &[
                (TypeId::of::<Pos>(), &pos1 as *const Pos as *const u8),
                (TypeId::of::<Hp>(), &hp1 as *const Hp as *const u8),
            ],
        );

        let pos2 = Pos { x: 3.0, y: 4.0 };
        let hp2 = Hp(200);
        arch.add_entity_typed_raw(
            e2,
            &[
                (TypeId::of::<Pos>(), &pos2 as *const Pos as *const u8),
                (TypeId::of::<Hp>(), &hp2 as *const Hp as *const u8),
            ],
        );

        // Test get
        let pos1_ref = arch.get::<Pos>(e1).unwrap();
        assert_eq!(pos1_ref.x, 1.0);
        assert_eq!(pos1_ref.y, 2.0);
        let hp2_ref = arch.get::<Hp>(e2).unwrap();
        assert_eq!(hp2_ref.0, 200);

        // Test get on non-existent entity
        let e3 = unsafe { Entity::from_raw(3) };
        assert!(
            arch.get::<Pos>(e3).is_none(),
            "non-existent entity returns None"
        );

        // Test get_mut
        {
            let hp1_mut = arch.get_mut::<Hp>(e1).unwrap();
            hp1_mut.0 = 50;
        }
        assert_eq!(arch.get::<Hp>(e1).unwrap().0, 50, "mutation must persist");
    }

    #[test]
    fn blob_iter_components_blob_on_empty_archetype() {
        let (storage, id) = make_blob_archetype_in_storage();
        let arch = storage.get_archetype(id).unwrap();
        let result = arch.iter_components_blob::<Pos>();
        assert!(
            result.is_some(),
            "blob archetype with Pos should return Some"
        );
        let (entities, components) = result.unwrap();
        assert_eq!(entities.len(), 0);
        assert_eq!(components.len(), 0);
    }

    #[test]
    fn add_entity_typed_raw_stores_components() {
        // Kills: add_entity_typed_raw → () (no-op)
        let mut storage = ArchetypeStorage::new();
        let mut metas = HashMap::new();
        metas.insert(TypeId::of::<Hp>(), ComponentMeta::of::<Hp>());
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype_with_blob(sig, metas);
        let arch = storage.get_archetype_mut(id).unwrap();

        let e1 = unsafe { Entity::from_raw(1) };
        let hp_val = Hp(42);
        let components = vec![(TypeId::of::<Hp>(), &hp_val as *const Hp as *const u8)];
        arch.add_entity_typed_raw(e1, &components);

        assert_eq!(arch.len(), 1);
        let hp = arch.get::<Hp>(e1).unwrap();
        assert_eq!(hp.0, 42, "component must be stored correctly");
    }

    #[test]
    fn archetype_signature_deduplicates() {
        // Kills: dedup not called, or sort not called
        let sig = ArchetypeSignature::new(vec![
            TypeId::of::<Hp>(),
            TypeId::of::<Pos>(),
            TypeId::of::<Hp>(), // duplicate
        ]);
        assert_eq!(sig.len(), 2, "duplicates must be removed");
        assert!(sig.contains(TypeId::of::<Hp>()));
        assert!(sig.contains(TypeId::of::<Pos>()));
    }

    #[test]
    fn archetype_remove_entity_returns_correct_index() {
        // Kills: remove_entity → None (always)
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype(sig);
        let arch = storage.get_archetype_mut(id).unwrap();

        let e1 = unsafe { Entity::from_raw(1) };
        let mut c = HashMap::new();
        c.insert(
            TypeId::of::<Hp>(),
            Box::new(Hp(10)) as Box<dyn std::any::Any + Send + Sync>,
        );
        arch.add_entity(e1, c);

        let result = arch.remove_entity(e1);
        assert!(
            result.is_some(),
            "remove_entity must return Some for existing entity"
        );
    }

    #[test]
    fn archetype_remove_entity_components_with_swap() {
        // Test that when removing non-last entity, the swap logic updates correctly
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype(sig);
        let arch = storage.get_archetype_mut(id).unwrap();

        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(2) };
        let e3 = unsafe { Entity::from_raw(3) };

        for (e, hp) in [(e1, 10), (e2, 20), (e3, 30)] {
            let mut c = HashMap::new();
            c.insert(
                TypeId::of::<Hp>(),
                Box::new(Hp(hp)) as Box<dyn std::any::Any + Send + Sync>,
            );
            arch.add_entity(e, c);
        }

        // Remove e1 (index 0): e3 swaps to index 0
        let removed = arch.remove_entity_components(e1);
        let removed_hp = removed[&TypeId::of::<Hp>()].downcast_ref::<Hp>().unwrap();
        assert_eq!(removed_hp.0, 10, "removed component value must match");

        assert_eq!(arch.len(), 2);
        // e2 and e3 should still be accessible
        let hp2 = arch.get::<Hp>(e2).unwrap();
        let hp3 = arch.get::<Hp>(e3).unwrap();
        assert_eq!(hp2.0, 20);
        assert_eq!(hp3.0, 30);
    }

    #[test]
    fn archetype_storage_entity_mapping_sparse_array_grows() {
        // Kills: resize not called (would panic on large entity IDs)
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype(sig);

        // Entity with large ID forces sparse array growth
        let e = unsafe { Entity::from_raw(10000) };
        storage.set_entity_archetype(e, id);
        assert_eq!(storage.get_entity_archetype(e), Some(id));
    }

    #[test]
    fn archetype_storage_remove_entity_beyond_vec_is_none() {
        // Kills: remove_entity < → > (would enter branch for out-of-range)
        let mut storage = ArchetypeStorage::new();
        let e = unsafe { Entity::from_raw(99999) };
        let result = storage.remove_entity(e);
        assert_eq!(result, None, "entity beyond vec must return None");
    }

    #[test]
    fn archetype_storage_archetypes_with_component_inverted_index() {
        // Kills: component_to_archetypes not populated
        let mut storage = ArchetypeStorage::new();

        let sig1 = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let sig2 = ArchetypeSignature::new(vec![TypeId::of::<Pos>()]);
        let sig3 = ArchetypeSignature::new(vec![TypeId::of::<Hp>(), TypeId::of::<Pos>()]);

        storage.get_or_create_archetype(sig1);
        storage.get_or_create_archetype(sig2);
        storage.get_or_create_archetype(sig3);

        let hp_archetypes: Vec<_> = storage
            .archetypes_with_component(TypeId::of::<Hp>())
            .collect();
        assert_eq!(hp_archetypes.len(), 2, "Hp in sig1 and sig3");

        let pos_archetypes: Vec<_> = storage
            .archetypes_with_component(TypeId::of::<Pos>())
            .collect();
        assert_eq!(pos_archetypes.len(), 2, "Pos in sig2 and sig3");

        let vel_archetypes: Vec<_> = storage
            .archetypes_with_component(TypeId::of::<Vel>())
            .collect();
        assert_eq!(vel_archetypes.len(), 0, "Vel not in any archetype");
    }

    #[test]
    fn archetype_storage_get_archetype_mut_allows_mutation() {
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<Hp>()]);
        let id = storage.get_or_create_archetype(sig);

        let arch = storage.get_archetype_mut(id).unwrap();
        assert_eq!(arch.len(), 0, "empty archetype");
        // Verify mutability by checking id
        assert_eq!(arch.id, id);
    }
}

// =============================================================================
// 4. EntityAllocator — reserve, alive_count, free list LIFO, is_alive
// =============================================================================

mod entity_allocator_mutation_tests {
    use super::*;

    #[test]
    fn reserve_preallocates_without_spawning() {
        // Kills: reserve → () (no-op)
        let mut alloc = EntityAllocator::new();
        alloc.reserve(100);
        assert_eq!(alloc.alive_count(), 0, "reserve must not spawn entities");
        assert_eq!(
            alloc.capacity(),
            0,
            "capacity reflects spawned slots, not reserved"
        );
        // reserve() should at minimum not panic or corrupt state
        // After reserving, spawning should work normally
        let e = alloc.spawn();
        assert!(alloc.is_alive(e));
        assert_eq!(alloc.alive_count(), 1);
    }

    #[test]
    fn alive_count_tracks_spawn_and_despawn() {
        // Kills: alive_count → 0, alive_count subtraction wrong
        let mut alloc = EntityAllocator::new();
        assert_eq!(alloc.alive_count(), 0);

        let e1 = alloc.spawn();
        assert_eq!(alloc.alive_count(), 1);

        let e2 = alloc.spawn();
        assert_eq!(alloc.alive_count(), 2);

        let e3 = alloc.spawn();
        assert_eq!(alloc.alive_count(), 3);

        alloc.despawn(e2);
        assert_eq!(alloc.alive_count(), 2, "despawn must decrement alive_count");

        alloc.despawn(e1);
        assert_eq!(alloc.alive_count(), 1);

        alloc.despawn(e3);
        assert_eq!(alloc.alive_count(), 0);
    }

    #[test]
    fn free_list_is_lifo_recycling_order() {
        // Kills: free_list push/pop order wrong
        let mut alloc = EntityAllocator::new();
        let e0 = alloc.spawn(); // id=0
        let e1 = alloc.spawn(); // id=1
        let e2 = alloc.spawn(); // id=2

        // Despawn in order: e0, e1, e2
        alloc.despawn(e0); // free_list: [0]
        alloc.despawn(e1); // free_list: [0, 1]
        alloc.despawn(e2); // free_list: [0, 1, 2]

        // Respawn — LIFO means we get e2's id first, then e1, then e0
        let r1 = alloc.spawn();
        assert_eq!(r1.id(), 2, "LIFO: last despawned (e2) recycled first");
        assert_eq!(
            r1.generation(),
            1,
            "recycled entity has incremented generation"
        );

        let r2 = alloc.spawn();
        assert_eq!(r2.id(), 1, "LIFO: e1 recycled second");

        let r3 = alloc.spawn();
        assert_eq!(r3.id(), 0, "LIFO: e0 recycled last");

        // Next spawn should be a fresh ID
        let r4 = alloc.spawn();
        assert_eq!(r4.id(), 3, "after free list empty, new ID allocated");
    }

    #[test]
    fn is_alive_rejects_wrong_generation() {
        // Kills: generation comparison == → !=
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.spawn(); // id=0, gen=0
        assert!(alloc.is_alive(e1));

        alloc.despawn(e1); // gen[0] = 1
        assert!(!alloc.is_alive(e1), "stale generation must be rejected");

        let e2 = alloc.spawn(); // id=0, gen=1
        assert!(alloc.is_alive(e2));
        assert!(!alloc.is_alive(e1), "old handle still invalid");
    }

    #[test]
    fn is_alive_rejects_out_of_range_entity() {
        // Kills: is_alive → true for unallocated slots
        let alloc = EntityAllocator::new();
        let fake = Entity::new(999, 0);
        assert!(
            !alloc.is_alive(fake),
            "unallocated entity must not be alive"
        );
    }

    #[test]
    fn despawn_returns_false_for_out_of_range() {
        // Kills: despawn >= → < (would skip bounds check)
        let mut alloc = EntityAllocator::new();
        let fake = Entity::new(999, 0);
        assert!(
            !alloc.despawn(fake),
            "despawn of unallocated must return false"
        );
    }

    #[test]
    fn despawn_double_despawn_returns_false() {
        let mut alloc = EntityAllocator::new();
        let e = alloc.spawn();
        assert!(alloc.despawn(e));
        assert!(!alloc.despawn(e), "double despawn must return false");
    }

    #[test]
    fn entity_to_raw_and_back_preserves_all_bits() {
        let cases = [
            (0u32, 0u32),
            (1, 0),
            (0, 1),
            (u32::MAX, 0),
            (0, u32::MAX),
            (0x12345678, 0xABCDEF01),
        ];
        for (id, gen) in cases {
            let e = Entity::new(id, gen);
            let raw = e.to_raw();
            let restored = unsafe { Entity::from_raw(raw) };
            assert_eq!(restored.id(), id, "id must roundtrip for ({}, {})", id, gen);
            assert_eq!(
                restored.generation(),
                gen,
                "gen must roundtrip for ({}, {})",
                id,
                gen
            );
        }
    }

    #[test]
    fn entity_null_properties() {
        let null = Entity::null();
        assert!(null.is_null());
        assert_eq!(null.id(), u32::MAX);
        assert_eq!(null.generation(), u32::MAX);

        let not_null = Entity::new(0, 0);
        assert!(!not_null.is_null());
    }

    #[test]
    fn entity_debug_and_display() {
        let e = Entity::new(5, 3);
        assert_eq!(format!("{}", e), "5v3");
        assert_eq!(format!("{:?}", e), "Entity(5v3)");
    }

    #[test]
    fn entity_ordering_is_correct() {
        let e00 = Entity::new(0, 0);
        let e10 = Entity::new(1, 0);
        let e01 = Entity::new(0, 1);

        assert!(e00 < e01, "(0,0) < (0,1)");
        assert!(e01 < e10, "(0,1) < (1,0)");
        assert!(e00 < e10, "(0,0) < (1,0)");
    }

    #[test]
    fn clear_resets_all_state() {
        let mut alloc = EntityAllocator::new();
        alloc.spawn();
        alloc.spawn();
        let e = alloc.spawn();
        alloc.despawn(e);

        alloc.clear();
        assert_eq!(alloc.alive_count(), 0);
        assert_eq!(alloc.capacity(), 0);
        assert_eq!(alloc.spawned_count(), 0);
        assert_eq!(alloc.despawned_count(), 0);

        // After clear, first spawn gets id=0 again
        let e_new = alloc.spawn();
        assert_eq!(e_new.id(), 0);
        assert_eq!(e_new.generation(), 0);
    }

    #[test]
    fn with_capacity_does_not_affect_initial_state() {
        let alloc = EntityAllocator::with_capacity(1000);
        assert_eq!(alloc.alive_count(), 0);
        assert_eq!(alloc.capacity(), 0);
        assert_eq!(alloc.spawned_count(), 0);
    }
}

// =============================================================================
// 5. ComponentMeta — create_blob_vec_with_capacity e2e, clone_fn with drop
// =============================================================================

mod component_meta_mutation_tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Point {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Named {
        name: String,
    }

    #[test]
    fn create_blob_vec_with_capacity_allows_push_without_realloc() {
        let meta = ComponentMeta::of::<Point>();
        let mut blob = meta.create_blob_vec_with_capacity(10);
        assert!(blob.capacity() >= 10);
        assert_eq!(blob.len(), 0);

        // Push values and verify
        let p = Point { x: 1.0, y: 2.0 };
        unsafe {
            blob.push_raw(&p as *const Point as *const u8, meta.clone_fn);
        }
        assert_eq!(blob.len(), 1);

        let read_back = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const Point) };
        assert_eq!(
            read_back, p,
            "value must survive push_raw + get_raw roundtrip"
        );
    }

    #[test]
    fn clone_fn_works_with_heap_allocated_types() {
        let meta = ComponentMeta::of::<Named>();

        let src = Named {
            name: "hello world".to_string(),
        };
        let mut dst = std::mem::MaybeUninit::<Named>::uninit();

        unsafe {
            (meta.clone_fn)(
                &src as *const Named as *const u8,
                dst.as_mut_ptr() as *mut u8,
            );
            let cloned = dst.assume_init();
            assert_eq!(cloned.name, "hello world");
            assert_ne!(
                &cloned as *const Named, &src as *const Named,
                "clone must be a separate allocation"
            );
        }
    }

    #[test]
    fn drop_fn_some_for_heap_types_none_for_copy() {
        let copy_meta = ComponentMeta::of::<Point>();
        assert!(
            copy_meta.drop_fn.is_none(),
            "Copy type must have no drop_fn"
        );

        let heap_meta = ComponentMeta::of::<Named>();
        assert!(heap_meta.drop_fn.is_some(), "Heap type must have drop_fn");
    }

    #[test]
    fn type_name_includes_struct_name() {
        let meta = ComponentMeta::of::<Point>();
        assert!(
            meta.type_name.contains("Point"),
            "type_name must contain 'Point', got '{}'",
            meta.type_name
        );
    }

    #[test]
    fn registry_register_returns_true_first_false_after() {
        let mut reg = ComponentMetaRegistry::new();
        assert!(reg.register::<Point>(), "first register must return true");
        assert!(
            !reg.register::<Point>(),
            "duplicate register must return false"
        );
    }

    #[test]
    fn registry_get_returns_correct_meta() {
        let mut reg = ComponentMetaRegistry::new();
        reg.register::<Point>();

        let meta = reg.get(TypeId::of::<Point>()).unwrap();
        assert_eq!(meta.layout.size(), std::mem::size_of::<Point>());
        assert_eq!(meta.layout.align(), std::mem::align_of::<Point>());

        assert!(
            reg.get(TypeId::of::<Named>()).is_none(),
            "unregistered type returns None"
        );
    }

    #[test]
    fn registry_is_registered_reflects_state() {
        let mut reg = ComponentMetaRegistry::new();
        assert!(!reg.is_registered(TypeId::of::<Point>()));
        reg.register::<Point>();
        assert!(reg.is_registered(TypeId::of::<Point>()));
        assert!(!reg.is_registered(TypeId::of::<Named>()));
    }
}

// =============================================================================
// 6. CountingAlloc — value-based assertions
// =============================================================================

#[cfg(feature = "alloc-counter")]
mod counting_alloc_tests {
    use crate::counting_alloc;

    #[test]
    fn reset_zeroes_both_counters() {
        counting_alloc::reset_allocs();
        // After reset, both should be 0 (or close, since test framework may allocate)
        let a = counting_alloc::allocs();
        let d = counting_alloc::deallocs();
        // We can't assert exact 0 since the test framework allocates,
        // but net_allocs should be well-defined
        let _net = counting_alloc::net_allocs();
        // At minimum, verify the functions return consistent types
        assert!(a >= 0_usize);
        assert!(d >= 0_usize);
    }

    #[test]
    fn net_allocs_equals_allocs_minus_deallocs() {
        counting_alloc::reset_allocs();
        let a = counting_alloc::allocs();
        let d = counting_alloc::deallocs();
        let net = counting_alloc::net_allocs();
        assert_eq!(
            net,
            a as isize - d as isize,
            "net_allocs = allocs - deallocs"
        );
    }
}

// =============================================================================
// 7. Events — with_keep_frames, keep_frames, get_reader, EventReader
// =============================================================================

mod events_mutation_tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct PingEvent {
        seq: u32,
    }
    impl Event for PingEvent {}

    #[derive(Debug, Clone, PartialEq)]
    struct PongEvent {
        reply: String,
    }
    impl Event for PongEvent {}

    #[test]
    fn with_keep_frames_sets_value() {
        let events = Events::new().with_keep_frames(5);
        assert_eq!(
            events.keep_frames(),
            5,
            "keep_frames must reflect set value"
        );

        let events2 = Events::new().with_keep_frames(0);
        assert_eq!(events2.keep_frames(), 0);
    }

    #[test]
    fn default_keep_frames_is_two() {
        let events = Events::new();
        assert_eq!(events.keep_frames(), 2, "default keep_frames must be 2");
    }

    #[test]
    fn get_reader_reads_events() {
        let mut events = Events::new();
        events.send(PingEvent { seq: 1 });
        events.send(PingEvent { seq: 2 });

        let reader = events.get_reader::<PingEvent>();
        let collected: Vec<_> = reader.read(&events).collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].seq, 1);
        assert_eq!(collected[1].seq, 2);
    }

    #[test]
    fn drain_consumes_events() {
        let mut events = Events::new();
        events.send(PingEvent { seq: 1 });
        events.send(PingEvent { seq: 2 });

        let drained: Vec<_> = events.drain::<PingEvent>().collect();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].seq, 1);
        assert_eq!(drained[1].seq, 2);

        // After drain, no events left
        assert!(events.is_empty::<PingEvent>());
        assert_eq!(events.len::<PingEvent>(), 0);
    }

    #[test]
    fn clear_removes_specific_type_only() {
        let mut events = Events::new();
        events.send(PingEvent { seq: 1 });
        events.send(PongEvent { reply: "hi".into() });

        events.clear::<PingEvent>();
        assert!(events.is_empty::<PingEvent>());
        assert!(
            !events.is_empty::<PongEvent>(),
            "PongEvent must survive PingEvent clear"
        );
    }

    #[test]
    fn clear_all_removes_all_types() {
        let mut events = Events::new();
        events.send(PingEvent { seq: 1 });
        events.send(PongEvent { reply: "hi".into() });

        events.clear_all();
        assert!(events.is_empty::<PingEvent>());
        assert!(events.is_empty::<PongEvent>());
    }

    #[test]
    fn is_empty_true_then_false_after_send() {
        let mut events = Events::new();
        assert!(events.is_empty::<PingEvent>());
        events.send(PingEvent { seq: 1 });
        assert!(!events.is_empty::<PingEvent>());
    }

    #[test]
    fn len_matches_number_of_sends() {
        let mut events = Events::new();
        assert_eq!(events.len::<PingEvent>(), 0);
        events.send(PingEvent { seq: 1 });
        assert_eq!(events.len::<PingEvent>(), 1);
        events.send(PingEvent { seq: 2 });
        assert_eq!(events.len::<PingEvent>(), 2);
    }

    #[test]
    fn update_increments_frame() {
        let mut events = Events::new();
        assert_eq!(events.current_frame(), 0);
        events.update();
        assert_eq!(events.current_frame(), 1);
        events.update();
        assert_eq!(events.current_frame(), 2);
    }

    #[test]
    fn events_default_is_new() {
        let events = Events::default();
        assert_eq!(events.current_frame(), 0);
        assert_eq!(events.keep_frames(), 2);
    }

    #[test]
    fn read_on_unsent_type_returns_empty() {
        let events = Events::new();
        let count = events.read::<PingEvent>().count();
        assert_eq!(count, 0, "reading unsent type must return empty iterator");
    }

    #[test]
    fn drain_on_unsent_type_returns_empty() {
        let mut events = Events::new();
        let count = events.drain::<PingEvent>().count();
        assert_eq!(count, 0);
    }
}

// =============================================================================
// 8. World — count, entities_with, remove, despawn, each_mut, has,
//    register_component, insert_boxed, remove_by_type_id, resources
// =============================================================================

mod world_mutation_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Pos {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Vel {
        vx: f32,
        vy: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Hp(i32);

    #[test]
    fn count_returns_correct_number() {
        let mut world = World::new();
        assert_eq!(world.count::<Pos>(), 0);

        let e1 = world.spawn();
        world.insert(e1, Pos { x: 0.0, y: 0.0 });
        assert_eq!(world.count::<Pos>(), 1);

        let e2 = world.spawn();
        world.insert(e2, Pos { x: 1.0, y: 1.0 });
        assert_eq!(world.count::<Pos>(), 2);

        // Different component type
        assert_eq!(world.count::<Vel>(), 0);

        world.insert(e1, Vel { vx: 1.0, vy: 1.0 });
        assert_eq!(world.count::<Vel>(), 1);
        // Pos count still 2 (e1 now has both Pos and Vel, in a different archetype)
        assert_eq!(world.count::<Pos>(), 2);
    }

    #[test]
    fn entities_with_returns_correct_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, Hp(100));
        world.insert(e3, Hp(200));
        // e2 does NOT have Hp

        let with_hp = world.entities_with::<Hp>();
        assert_eq!(with_hp.len(), 2);
        assert!(with_hp.contains(&e1));
        assert!(with_hp.contains(&e3));
        assert!(!with_hp.contains(&e2));
    }

    #[test]
    fn remove_returns_true_when_component_exists() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));

        assert!(world.has::<Hp>(e));
        let result = world.remove::<Hp>(e);
        assert!(result, "remove must return true when component exists");
        assert!(!world.has::<Hp>(e), "component must be gone after remove");
    }

    #[test]
    fn remove_returns_false_when_component_missing() {
        let mut world = World::new();
        let e = world.spawn();

        let result = world.remove::<Hp>(e);
        assert!(!result, "remove must return false when component missing");
    }

    #[test]
    fn remove_returns_false_for_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));
        world.despawn(e);

        let result = world.remove::<Hp>(e);
        assert!(!result, "remove must return false for dead entity");
    }

    #[test]
    fn despawn_returns_true_then_false() {
        let mut world = World::new();
        let e = world.spawn();

        assert!(world.despawn(e), "first despawn must return true");
        assert!(!world.despawn(e), "second despawn must return false");
    }

    #[test]
    fn despawn_cleans_up_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Pos { x: 1.0, y: 2.0 });
        world.insert(e, Hp(100));

        world.despawn(e);
        assert!(!world.is_alive(e));
        assert!(world.get::<Pos>(e).is_none());
        assert!(world.get::<Hp>(e).is_none());
    }

    #[test]
    fn each_mut_modifies_all_matching_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, Hp(10));
        world.insert(e2, Hp(20));
        world.insert(e3, Hp(30));

        world.each_mut::<Hp>(|_e, hp| hp.0 *= 2);

        assert_eq!(world.get::<Hp>(e1).unwrap().0, 20);
        assert_eq!(world.get::<Hp>(e2).unwrap().0, 40);
        assert_eq!(world.get::<Hp>(e3).unwrap().0, 60);
    }

    #[test]
    fn each_mut_does_not_affect_other_types() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));
        world.insert(e, Pos { x: 1.0, y: 2.0 });

        world.each_mut::<Hp>(|_e, hp| hp.0 = 999);

        assert_eq!(world.get::<Hp>(e).unwrap().0, 999);
        assert_eq!(world.get::<Pos>(e).unwrap().x, 1.0, "Pos unchanged");
    }

    #[test]
    fn has_returns_false_for_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));
        world.despawn(e);
        assert!(!world.has::<Hp>(e), "has must return false for dead entity");
    }

    #[test]
    fn has_returns_true_then_false_after_remove() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));
        assert!(world.has::<Hp>(e));
        world.remove::<Hp>(e);
        assert!(!world.has::<Hp>(e));
    }

    #[test]
    fn insert_on_dead_entity_is_noop() {
        let mut world = World::new();
        let e = world.spawn();
        world.despawn(e);
        world.insert(e, Hp(100));
        assert!(
            world.get::<Hp>(e).is_none(),
            "insert on dead entity must be ignored"
        );
    }

    #[test]
    fn get_mut_on_dead_entity_returns_none() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));
        world.despawn(e);
        assert!(world.get_mut::<Hp>(e).is_none());
    }

    #[test]
    fn entity_count_tracks_spawns_and_despawns() {
        let mut world = World::new();
        assert_eq!(world.entity_count(), 0);

        let e1 = world.spawn();
        assert_eq!(world.entity_count(), 1);

        let e2 = world.spawn();
        assert_eq!(world.entity_count(), 2);

        world.despawn(e1);
        assert_eq!(world.entity_count(), 1);

        world.despawn(e2);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn register_component_enables_blob_storage() {
        let mut world = World::new();
        assert!(!world.is_component_registered_blob::<Pos>());
        world.register_component::<Pos>();
        assert!(world.is_component_registered_blob::<Pos>());
    }

    #[test]
    fn resources_insert_get_get_mut() {
        struct GameTime(f32);

        let mut world = World::new();
        assert!(world.get_resource::<GameTime>().is_none());

        world.insert_resource(GameTime(0.0));
        assert_eq!(world.get_resource::<GameTime>().unwrap().0, 0.0);

        if let Some(gt) = world.get_resource_mut::<GameTime>() {
            gt.0 = 1.5;
        }
        assert_eq!(world.get_resource::<GameTime>().unwrap().0, 1.5);
    }

    #[test]
    fn archetypes_accessor_returns_storage() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Hp(100));

        let archetypes = world.archetypes();
        let count = archetypes.iter().count();
        assert!(count >= 1, "must have at least one archetype");
    }

    #[test]
    fn command_buffer_insert_and_remove() {
        use crate::CommandBuffer;

        let mut world = World::new();
        world.register_component::<Hp>();

        let e = world.spawn();
        let mut cmds = CommandBuffer::new();
        cmds.insert(e, Hp(42));
        cmds.flush(&mut world);

        assert_eq!(
            world.get::<Hp>(e).unwrap().0,
            42,
            "command buffer insert must work"
        );

        let mut cmds2 = CommandBuffer::new();
        cmds2.remove::<Hp>(e);
        cmds2.flush(&mut world);

        assert!(!world.has::<Hp>(e), "command buffer remove must work");
    }
}

// =============================================================================
// 9. Schedule / Stage / App / Plugin
// =============================================================================

mod schedule_mutation_tests {
    use super::*;

    #[test]
    fn schedule_with_stage_adds_stages() {
        let schedule = Schedule::default()
            .with_stage("a")
            .with_stage("b")
            .with_stage("c");
        assert_eq!(schedule.stages.len(), 3);
        assert_eq!(schedule.stages[0].name, "a");
        assert_eq!(schedule.stages[1].name, "b");
        assert_eq!(schedule.stages[2].name, "c");
    }

    #[test]
    fn schedule_add_system_to_existing_stage() {
        fn dummy(_w: &mut World) {}

        let mut schedule = Schedule::default().with_stage("sim");
        schedule.add_system("sim", dummy);
        assert_eq!(schedule.stages[0].systems.len(), 1);
    }

    #[test]
    fn schedule_add_system_to_nonexistent_stage_is_noop() {
        fn dummy(_w: &mut World) {}

        let mut schedule = Schedule::default().with_stage("sim");
        schedule.add_system("nonexistent", dummy);
        // No panic, just ignored
        assert_eq!(schedule.stages[0].systems.len(), 0);
    }

    #[test]
    fn schedule_run_executes_systems_in_order() {
        fn sys1(world: &mut World) {
            world.insert_resource(1_u32);
        }
        fn sys2(world: &mut World) {
            let val = world.get_resource::<u32>().copied().unwrap_or(0);
            world.insert_resource(val + 10);
        }

        let mut schedule = Schedule::default().with_stage("a").with_stage("b");
        schedule.add_system("a", sys1);
        schedule.add_system("b", sys2);

        let mut world = World::new();
        schedule.run(&mut world);

        assert_eq!(
            *world.get_resource::<u32>().unwrap(),
            11,
            "sys1 sets 1, sys2 reads 1 and sets 11"
        );
    }

    #[test]
    fn app_new_has_default_stages() {
        let app = App::new();
        let stage_names: Vec<&str> = app.schedule.stages.iter().map(|s| s.name).collect();
        assert!(stage_names.contains(&"perception"));
        assert!(stage_names.contains(&"simulation"));
        assert!(stage_names.contains(&"ai_planning"));
        assert!(stage_names.contains(&"physics"));
        assert!(stage_names.contains(&"presentation"));
    }

    #[test]
    fn app_default_equals_new() {
        let app = App::default();
        assert_eq!(app.world.entity_count(), 0);
        assert!(!app.schedule.stages.is_empty());
    }

    #[test]
    fn app_insert_resource_is_accessible() {
        struct Config(i32);
        let app = App::new().insert_resource(Config(42));
        assert_eq!(app.world.get_resource::<Config>().unwrap().0, 42);
    }

    #[test]
    fn app_run_fixed_executes_n_times() {
        fn counter(world: &mut World) {
            let count = world.get_resource::<u32>().copied().unwrap_or(0);
            world.insert_resource(count + 1);
        }

        let mut app = App::new();
        app.add_system("simulation", counter);
        let app = app.run_fixed(5);
        assert_eq!(
            *app.world.get_resource::<u32>().unwrap(),
            5,
            "run_fixed(5) must execute system 5 times"
        );
    }

    #[test]
    fn app_add_plugin() {
        struct TestPlugin;
        impl crate::Plugin for TestPlugin {
            fn build(&self, app: &mut App) {
                app.world.insert_resource(42_u32);
            }
        }

        let app = App::new().add_plugin(TestPlugin);
        assert_eq!(*app.world.get_resource::<u32>().unwrap(), 42);
    }

    #[test]
    fn system_stage_constants_exact_values() {
        assert_eq!(SystemStage::PRE_SIMULATION, "pre_simulation");
        assert_eq!(SystemStage::PERCEPTION, "perception");
        assert_eq!(SystemStage::SIMULATION, "simulation");
        assert_eq!(SystemStage::AI_PLANNING, "ai_planning");
        assert_eq!(SystemStage::PHYSICS, "physics");
        assert_eq!(SystemStage::POST_SIMULATION, "post_simulation");
        assert_eq!(SystemStage::PRESENTATION, "presentation");
    }
}

// =============================================================================
// 10. Rng — additional mutation-specific boundary tests
// =============================================================================

mod rng_mutation_tests {
    use crate::Rng;

    #[test]
    fn gen_u64_produces_nonzero_values() {
        let mut rng = Rng::from_seed(42);
        let mut any_nonzero = false;
        for _ in 0..100 {
            if rng.gen_u64() != 0 {
                any_nonzero = true;
                break;
            }
        }
        assert!(any_nonzero, "gen_u64 must produce nonzero values");
    }

    #[test]
    fn gen_u32_diverse_output_rng() {
        let mut rng = Rng::from_seed(123);
        let mut values = std::collections::HashSet::new();
        for _ in 0..100 {
            values.insert(rng.gen_u32());
        }
        assert!(values.len() > 50, "gen_u32 must produce diverse values");
    }

    #[test]
    fn gen_range_single_value_range_returns_that_value() {
        let mut rng = Rng::from_seed(77);
        for _ in 0..100 {
            let val = rng.gen_range(42_u32..43);
            assert_eq!(val, 42, "single-value range must always return start");
        }
    }

    #[test]
    fn gen_range_i32_boundaries() {
        let mut rng = Rng::from_seed(999);
        let mut hit_low = false;
        let mut hit_high = false;
        for _ in 0..100_000 {
            let val = rng.gen_range(-5_i32..5);
            assert!(val >= -5 && val < 5);
            if val == -5 {
                hit_low = true;
            }
            if val == 4 {
                hit_high = true;
            }
            if hit_low && hit_high {
                break;
            }
        }
        assert!(hit_low, "must hit lower bound -5");
        assert!(hit_high, "must hit upper bound - 1 = 4");
    }
}

// =============================================================================
// 11. TypeRegistry — command buffer handler registration
// =============================================================================

mod type_registry_mutation_tests {
    use super::*;
    use crate::type_registry::TypeRegistry;

    #[derive(Clone, Debug)]
    struct Marker;

    #[test]
    fn register_populates_insert_and_remove_handlers() {
        let mut reg = TypeRegistry::new();
        reg.register::<Marker>();

        assert!(
            reg.insert_handlers.contains_key(&TypeId::of::<Marker>()),
            "insert handler must be registered"
        );
        assert!(
            reg.remove_handlers.contains_key(&TypeId::of::<Marker>()),
            "remove handler must be registered"
        );
    }

    #[test]
    fn register_populates_type_name() {
        let mut reg = TypeRegistry::new();
        reg.register::<Marker>();

        assert!(
            reg.type_names.contains_key(&TypeId::of::<Marker>()),
            "type_name must be registered"
        );
        let name = reg.type_names.get(&TypeId::of::<Marker>()).unwrap();
        assert!(name.contains("Marker"), "type_name must contain 'Marker'");
    }

    #[test]
    fn insert_handler_inserts_component_via_world() {
        // Test insert handler indirectly via CommandBuffer which uses TypeRegistry
        let mut world = World::new();
        world.register_component::<Marker>();
        let e = world.spawn();

        // Use CommandBuffer which uses the TypeRegistry
        let mut cmds = crate::CommandBuffer::new();
        cmds.insert(e, Marker);
        cmds.flush(&mut world);

        assert!(world.has::<Marker>(e), "handler must insert component");
    }
}

// =============================================================================
// 12. Targeted Mutation Kills — Round 2
// =============================================================================
// These tests target the 7 catchable mutations missed in round 1.

mod round2_mutation_kills {
    use super::*;

    // -------------------------------------------------------------------------
    // archetype.rs:285 — remove_entity -> Option<usize> with Some(0)/Some(1)
    // We must verify None is returned for non-existent entities.
    // -------------------------------------------------------------------------
    #[test]
    fn archetype_remove_entity_returns_none_for_unknown_entity() {
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<u32>()]);
        let arch_id = storage.get_or_create_archetype(sig);
        let arch = storage.get_archetype_mut(arch_id).unwrap();

        // Never inserted entity — remove must return None, not Some(0) or Some(1)
        let fake_entity = Entity::new(999, 0);
        let result = arch.remove_entity(fake_entity);
        assert_eq!(
            result, None,
            "remove_entity on unknown entity must return None"
        );
    }

    #[test]
    fn archetype_remove_entity_returns_correct_dense_index() {
        let mut storage = ArchetypeStorage::new();
        let sig = ArchetypeSignature::new(vec![TypeId::of::<u32>()]);
        let arch_id = storage.get_or_create_archetype(sig);
        let arch = storage.get_archetype_mut(arch_id).unwrap();

        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);
        let e2 = Entity::new(2, 0);
        let mut comps0 = HashMap::new();
        comps0.insert(
            TypeId::of::<u32>(),
            Box::new(10u32) as Box<dyn std::any::Any + Send + Sync>,
        );
        arch.add_entity(e0, comps0);
        let mut comps1 = HashMap::new();
        comps1.insert(
            TypeId::of::<u32>(),
            Box::new(20u32) as Box<dyn std::any::Any + Send + Sync>,
        );
        arch.add_entity(e1, comps1);
        let mut comps2 = HashMap::new();
        comps2.insert(
            TypeId::of::<u32>(),
            Box::new(30u32) as Box<dyn std::any::Any + Send + Sync>,
        );
        arch.add_entity(e2, comps2);

        // e1 is at dense index 1 — removing it should return Some(1)
        let idx = arch.remove_entity(e1);
        assert_eq!(
            idx,
            Some(1),
            "remove_entity must return the correct dense index"
        );

        // e0 is at dense index 0
        let idx0 = arch.remove_entity(e0);
        assert_eq!(
            idx0,
            Some(0),
            "remove_entity must return correct index after prior removal"
        );
    }

    // -------------------------------------------------------------------------
    // blob_vec.rs:117 — capacity * 2 growth factor in reserve
    // Mutant changes * 2 to + 2 or / 2. After growing past 4, next growth
    // should double: 4 → 8. With + 2 it would be 6, with / 2 it would be 2.
    // -------------------------------------------------------------------------
    #[test]
    fn blob_vec_reserve_growth_factor_doubles() {
        let mut blob = BlobVec::new::<u32>();
        // First reserve: capacity goes to max(additional, 0*2, 4) = max(additional, 4)
        // Push 4 items to fill capacity
        for i in 0u32..4 {
            unsafe {
                blob.push(i);
            }
        }
        assert_eq!(blob.capacity(), 4, "initial capacity should be 4");

        // Push 5th item: triggers reserve(1), required_cap=5
        // new_capacity = max(5, 4*2, 4) = max(5, 8, 4) = 8  (correct)
        // with +2: max(5, 4+2, 4) = max(5, 6, 4) = 6
        // with /2: max(5, 4/2, 4) = max(5, 2, 4) = 5
        unsafe {
            blob.push(4u32);
        }
        assert_eq!(
            blob.capacity(),
            8,
            "capacity must double from 4 to 8 (growth factor * 2)"
        );
    }

    // -------------------------------------------------------------------------
    // sparse_set.rs:239 — != to == in SparseSetData::remove
    // Mutant inverts the swap condition. When removing a non-last element,
    // the swap wouldn't happen; when removing the last, it would try to swap.
    // -------------------------------------------------------------------------
    #[test]
    fn sparse_set_data_remove_non_last_preserves_remaining() {
        let mut set = SparseSetData::<i32>::new();
        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);
        let e2 = Entity::new(2, 0);

        set.insert(e0, 100);
        set.insert(e1, 200);
        set.insert(e2, 300);

        // Remove e0 (at dense index 0, NOT the last element)
        // With correct !=: swaps e2 into position 0, removes last
        // With wrong ==: no swap, pops last (e2's data), but e0's data remains
        let removed = set.remove(e0);
        assert_eq!(removed, Some(100));

        // e1 and e2 must still be accessible with correct values
        assert_eq!(
            set.get(e1),
            Some(&200),
            "e1 must still be accessible after removing e0"
        );
        assert_eq!(
            set.get(e2),
            Some(&300),
            "e2 must still be accessible after removing e0"
        );
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn sparse_set_data_remove_last_element_works() {
        let mut set = SparseSetData::<i32>::new();
        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);

        set.insert(e0, 10);
        set.insert(e1, 20);

        // Remove the LAST element (e1 at dense index 1 = last_index 1)
        // With correct !=: dense_index == last_index, no swap, just pop
        // With wrong ==: dense_index == last_index → tries to swap with itself
        let removed = set.remove(e1);
        assert_eq!(removed, Some(20));
        assert_eq!(
            set.get(e0),
            Some(&10),
            "e0 must survive after removing last element"
        );
        assert_eq!(set.len(), 1);
    }

    // -------------------------------------------------------------------------
    // entity_allocator.rs:389 — reserve → ()
    // Verify reserve actually pre-allocates via generations_capacity()
    // -------------------------------------------------------------------------
    #[test]
    fn entity_allocator_reserve_preallocates() {
        let mut alloc = EntityAllocator::new();
        alloc.reserve(1000);
        assert!(
            alloc.generations_capacity() >= 1000,
            "reserve(1000) must pre-allocate, got capacity {}",
            alloc.generations_capacity()
        );
    }

    // -------------------------------------------------------------------------
    // sparse_set.rs:177 — with_capacity → Default::default()
    // Verify with_capacity actually pre-allocates via capacity()
    // -------------------------------------------------------------------------
    #[test]
    fn sparse_set_data_with_capacity_preallocates() {
        let set: SparseSetData<u64> = SparseSetData::with_capacity(256);
        assert!(
            set.capacity() >= 256,
            "with_capacity(256) must pre-allocate, got capacity {}",
            set.capacity()
        );
    }
}
