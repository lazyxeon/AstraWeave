//! Proves a warmed destruction tick does not touch the heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use astraweave_destruction::DestructionWorld;

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
    let mut world = DestructionWorld::aravath_gate(0xA2A0_0001);
    for _ in 0..80 {
        world.tick();
    }
    ALLOCS.store(0, Ordering::SeqCst);
    for _ in 0..20 {
        world.tick();
    }
    let allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(allocs, 0, "warmed ticks allocated {allocs} times");
}
