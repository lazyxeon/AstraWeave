//! Proves a warmed causal tick does not touch the heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use astraweave_causality::{bridge_cut_input, bridge_world, merchant_ford_input};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct CountAlloc;

unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountAlloc = CountAlloc;

#[test]
fn warmed_ticks_do_not_allocate() {
    let mut world = bridge_world();
    world.push_input(bridge_cut_input()).expect("cut");
    world.push_input(merchant_ford_input()).expect("cell");
    while world.tick_index() < 100 {
        world.tick().expect("warmup");
    }
    ALLOCS.store(0, Ordering::SeqCst);
    while world.tick_index() <= 120 {
        world.tick().expect("measured");
    }
    let allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(allocs, 0, "warmed ticks allocated {allocs} times");
    assert_eq!(world.stock(), 0);
    assert_eq!(world.quest_complete(), 1);
}
