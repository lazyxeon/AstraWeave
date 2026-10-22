//! `CountingAlloc` is this binary's global allocator.
//!
//! The lib-test module cannot install `#[global_allocator]`, so a reset there
//! does not prove the counters move. Run with:
//!
//! ```bash
//! cargo test -p astraweave-ecs --features alloc-counter --test alloc_trace_quiet
//! ```
//!
//! Without `--features alloc-counter` this file compiles to zero tests and
//! `cargo test` exits 0. The CI step greps for at least one pass.

#![cfg(feature = "alloc-counter")]

use astraweave_ecs::counting_alloc::{allocs, deallocs, reset_allocs};
use std::alloc::{alloc_zeroed, dealloc, realloc, Layout};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[test]
fn alloc_trace_is_quiet() {
    reset_allocs();
    assert_eq!(allocs(), 0, "reset did not clear the allocation counter");
    assert_eq!(deallocs(), 0, "reset did not clear the free counter");
    let held = Box::new(1u64);
    assert!(allocs() >= 1, "CountingAlloc did not count the box");
    drop(held);
    assert!(deallocs() >= 1, "CountingAlloc did not count the free");

    reset_allocs();
    let zero_layout = Layout::from_size_align(32, 8).expect("zeroed layout");
    // SAFETY: `zero_layout` has a non-zero size. The pointer is freed below
    // with the layout of the block `realloc` returns.
    let zeroed = unsafe { alloc_zeroed(zero_layout) };
    assert!(!zeroed.is_null(), "alloc_zeroed returned null");
    assert!(allocs() >= 1, "CountingAlloc did not count alloc_zeroed");
    let bytes = unsafe { std::slice::from_raw_parts(zeroed, 32) };
    assert!(bytes.iter().all(|byte| *byte == 0));

    let allocs_before_realloc = allocs();
    let deallocs_before_realloc = deallocs();
    let grown_layout = Layout::from_size_align(64, 8).expect("grown layout");
    // SAFETY: `zeroed` came from this allocator with `zero_layout`. 64 is the
    // size of `grown_layout`, which is what the free uses.
    let grown = unsafe { realloc(zeroed, zero_layout, grown_layout.size()) };
    assert!(!grown.is_null(), "realloc returned null");
    assert!(
        allocs() > allocs_before_realloc,
        "realloc did not increment allocs"
    );
    assert!(
        deallocs() > deallocs_before_realloc,
        "realloc did not increment deallocs"
    );
    let deallocs_after_realloc = deallocs();
    unsafe { dealloc(grown, grown_layout) };
    assert!(
        deallocs() > deallocs_after_realloc,
        "the grown block was not freed"
    );
}
