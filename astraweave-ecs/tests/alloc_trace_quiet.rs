//! `CountingAlloc` is this binary's global allocator.
//!
//! The lib-test module cannot install `#[global_allocator]`, so a reset there
//! does not prove the counters move. Run with:
//!
//! ```bash
//! cargo test -p astraweave-ecs --features alloc-counter --test alloc_trace_quiet
//! ```

#![cfg(feature = "alloc-counter")]

use astraweave_ecs::counting_alloc::{allocs, deallocs, reset_allocs};

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
}
