//! Kani formal verification proofs for BlobVec
//!
//! These proofs use symbolic execution to exhaustively verify correctness
//! properties that testing alone cannot guarantee.
//!
//! Run with: `cargo kani --package astraweave-ecs`

use crate::blob_vec::BlobVec;
use std::alloc::Layout;

/// Verify that push/get roundtrip preserves values for any u32
#[kani::proof]
#[kani::unwind(2)]
fn blob_vec_push_get_roundtrip_u32() {
    let mut blob = BlobVec::new::<u32>();
    let value: u32 = kani::any();

    unsafe { blob.push(value) };

    let retrieved = unsafe { blob.get::<u32>(0) };
    kani::assert(
        retrieved == Some(&value),
        "Push/get roundtrip must preserve u32 value",
    );
}

/// Verify that push/get roundtrip preserves values for any u64
#[kani::proof]
#[kani::unwind(2)]
fn blob_vec_push_get_roundtrip_u64() {
    let mut blob = BlobVec::new::<u64>();
    let value: u64 = kani::any();

    unsafe { blob.push(value) };

    let retrieved = unsafe { blob.get::<u64>(0) };
    kani::assert(
        retrieved == Some(&value),
        "Push/get roundtrip must preserve u64 value",
    );
}

/// Verify that len() increments correctly after push
#[kani::proof]
#[kani::unwind(5)]
fn blob_vec_len_increments() {
    let mut blob = BlobVec::new::<u32>();
    let count: usize = kani::any();
    kani::assume(count <= 3); // Bound for tractability

    for _ in 0..count {
        let value: u32 = kani::any();
        unsafe { blob.push(value) };
    }

    kani::assert(blob.len() == count, "Length must equal push count");
}

/// Verify that get returns None for out-of-bounds index
#[kani::proof]
#[kani::unwind(2)]
fn blob_vec_get_oob_returns_none() {
    let mut blob = BlobVec::new::<u32>();
    let value: u32 = kani::any();
    unsafe { blob.push(value) };

    let oob_index: usize = kani::any();
    kani::assume(oob_index >= 1); // Any index >= len

    let result = unsafe { blob.get::<u32>(oob_index) };
    kani::assert(result.is_none(), "Out-of-bounds get must return None");
}

/// Verify that capacity is always >= len
#[kani::proof]
#[kani::unwind(10)]
fn blob_vec_capacity_invariant() {
    let mut blob = BlobVec::new::<u32>();
    let count: usize = kani::any();
    kani::assume(count <= 8); // Bound for tractability

    for _ in 0..count {
        let value: u32 = kani::any();
        unsafe { blob.push(value) };
        kani::assert(
            blob.capacity() >= blob.len(),
            "Capacity must always be >= len",
        );
    }
}

/// Verify that is_empty() is correct after push
#[kani::proof]
#[kani::unwind(2)]
fn blob_vec_is_empty_after_push() {
    let mut blob = BlobVec::new::<u32>();
    kani::assert(blob.is_empty(), "New BlobVec must be empty");

    let value: u32 = kani::any();
    unsafe { blob.push(value) };

    kani::assert(!blob.is_empty(), "BlobVec must not be empty after push");
}

/// Verify that clear() resets len to 0
#[kani::proof]
#[kani::unwind(5)]
fn blob_vec_clear_resets_len() {
    let mut blob = BlobVec::new::<u32>();
    let count: usize = kani::any();
    kani::assume(count <= 3);

    for _ in 0..count {
        let value: u32 = kani::any();
        unsafe { blob.push(value) };
    }

    blob.clear();
    kani::assert(blob.len() == 0, "Clear must reset len to 0");
    kani::assert(blob.is_empty(), "Clear must make BlobVec empty");
}

/// Verify multiple pushes maintain correct indices
#[kani::proof]
#[kani::unwind(4)]
fn blob_vec_multiple_push_maintains_indices() {
    let mut blob = BlobVec::new::<u32>();
    let v0: u32 = kani::any();
    let v1: u32 = kani::any();
    let v2: u32 = kani::any();

    unsafe {
        blob.push(v0);
        blob.push(v1);
        blob.push(v2);
    }

    kani::assert(blob.len() == 3, "Should have 3 elements");
    kani::assert(
        unsafe { blob.get::<u32>(0) } == Some(&v0),
        "Index 0 must match first push",
    );
    kani::assert(
        unsafe { blob.get::<u32>(1) } == Some(&v1),
        "Index 1 must match second push",
    );
    kani::assert(
        unsafe { blob.get::<u32>(2) } == Some(&v2),
        "Index 2 must match third push",
    );
}

/// Verify swap_remove returns correct value
#[kani::proof]
#[kani::unwind(4)]
fn blob_vec_swap_remove_returns_correct_value() {
    let mut blob = BlobVec::new::<u32>();
    let v0: u32 = kani::any();
    let v1: u32 = kani::any();

    unsafe {
        blob.push(v0);
        blob.push(v1);
    }

    // Remove first element (swaps with last)
    let removed = unsafe { blob.swap_remove::<u32>(0) };

    kani::assert(removed == v0, "swap_remove must return removed value");
    kani::assert(blob.len() == 1, "Length must decrease by 1");
    kani::assert(
        unsafe { blob.get::<u32>(0) } == Some(&v1),
        "Remaining element must be the swapped value",
    );
}

/// Verify get_raw returns correct pointer offset
#[kani::proof]
#[kani::unwind(3)]
fn blob_vec_get_raw_bounds_check() {
    let mut blob = BlobVec::new::<u64>();
    let v: u64 = kani::any();

    unsafe { blob.push(v) };

    kani::assert(blob.get_raw(0).is_some(), "Index 0 should be valid");
    kani::assert(blob.get_raw(1).is_none(), "Index 1 should be invalid");

    let index: usize = kani::any();
    kani::assume(index >= 1);
    kani::assert(
        blob.get_raw(index).is_none(),
        "Any index >= len must return None",
    );
}

/// Verify from_layout creates valid BlobVec
#[kani::proof]
fn blob_vec_from_layout_valid() {
    // Test with u32 layout
    let layout = Layout::new::<u32>();
    let blob = BlobVec::from_layout(layout, None);

    kani::assert(blob.len() == 0, "from_layout creates empty BlobVec");
    kani::assert(blob.is_empty(), "from_layout creates empty BlobVec");
    kani::assert(blob.capacity() == 0, "from_layout starts with 0 capacity");
}

/// Verify reserve maintains invariants
#[kani::proof]
#[kani::unwind(2)]
fn blob_vec_reserve_maintains_invariants() {
    let mut blob = BlobVec::new::<u32>();
    let additional: usize = kani::any();
    kani::assume(additional <= 100); // Bound for tractability

    blob.reserve(additional);

    kani::assert(blob.len() == 0, "Reserve must not change len");
    kani::assert(
        blob.capacity() >= additional,
        "Capacity must be >= requested",
    );
}

/// Verify as_slice returns correct length
#[kani::proof]
#[kani::unwind(4)]
fn blob_vec_as_slice_correct_length() {
    let mut blob = BlobVec::new::<u32>();
    let count: usize = kani::any();
    kani::assume(count <= 3);

    for _ in 0..count {
        unsafe { blob.push(kani::any::<u32>()) };
    }

    let slice = unsafe { blob.as_slice::<u32>() };
    kani::assert(slice.len() == count, "Slice length must match BlobVec len");
}

/// Verify swap_remove_raw maintains len correctly
#[kani::proof]
#[kani::unwind(4)]
fn blob_vec_swap_remove_raw_decrements_len() {
    let mut blob = BlobVec::new::<u32>();
    unsafe {
        blob.push(1u32);
        blob.push(2u32);
        blob.push(3u32);
    }

    let initial_len = blob.len();
    blob.swap_remove_raw(1);

    kani::assert(blob.len() == initial_len - 1, "Length must decrease by 1");
}

/// Element dropped by [`BlobVec::clear`] / [`BlobVec::drop_last`].
///
/// `drops` is a harness-owned counter. `Drop` records the slot and rejects a
/// second drop.
struct DropProbe {
    id: u8,
    drops: *mut u8,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        unsafe {
            let slot = &mut *self.drops.add(self.id as usize);
            *slot = slot.saturating_add(1);
            kani::assert(*slot == 1, "BlobVec drops each element exactly once");
        }
    }
}

/// Each live element is dropped once, including when a destructor panics.
///
/// `clear` is `drop_last(len)`. `drop_last` publishes the shorter `len` before
/// `drop_fn`. A panic leaves `len` counting only elements that have not been
/// dropped, and [`Drop`] for `BlobVec` calls `clear` on that tail. Kani does
/// not implement `catch_unwind` (model-checking/kani#267), so the harness
/// applies that split itself: `drop_last` runs through the panicking slot,
/// then `clear` finishes the rest. `clear_panic_does_not_double_free` runs the
/// real panic under Miri.
#[kani::proof]
#[kani::unwind(8)]
fn blob_vec_drops_each_element_once_including_panicking_drop() {
    const MAX: usize = 2;
    let count: usize = kani::any();
    kani::assume(count > 0 && count <= MAX);
    // How many elements `clear` drops from the end before the panicking one,
    // not counting the panicking slot itself.
    let dropped_before_panic: usize = kani::any();
    kani::assume(dropped_before_panic < count);

    let mut drops = [0u8; MAX];
    let ptr = drops.as_mut_ptr();
    let mut blob = BlobVec::new::<DropProbe>();
    let mut i = 0;
    while i < count {
        unsafe {
            blob.push(DropProbe {
                id: i as u8,
                drops: ptr,
            });
        }
        i += 1;
    }

    // The panicking destructor runs (the slot is committed) and then aborts
    // the rest of that `clear`. `+ 1` is that slot.
    blob.drop_last(dropped_before_panic + 1);
    blob.clear();

    i = 0;
    while i < count {
        unsafe {
            kani::assert(
                *ptr.add(i) == 1,
                "each element is dropped exactly once, including after a panicking drop",
            );
        }
        i += 1;
    }
    kani::assert(blob.len() == 0, "the tail clear empties the column");
}
