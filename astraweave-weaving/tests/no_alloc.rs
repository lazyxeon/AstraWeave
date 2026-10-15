//! `weave_eval` must not allocate. The field is [`NullField`], which stores no cells.

use astraweave_weaving::{
    queue_weave, weave_tick, NullField, WeaveFrame, WeaveScratch, TEMPLATE_HEAT,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting(System);

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        self.0.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting(System);

#[test]
fn weave_tick_does_not_allocate() {
    let mut scratch = WeaveScratch::new();
    let mut frame = WeaveFrame::new();
    let mut field = NullField;
    queue_weave(&mut scratch, 0, 8, 0, 1, TEMPLATE_HEAT, [0, 0, 0]).unwrap();
    ALLOCS.store(0, Ordering::Relaxed);
    for tick in 0..8 {
        weave_tick(&scratch, &mut frame, &mut field, tick);
    }
    assert_eq!(ALLOCS.load(Ordering::Relaxed), 0, "weave_tick allocated");
}
