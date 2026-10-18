//! The deformable movement tick must not touch the heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use astraweave_destruction::FractureFlags;
use astraweave_physics::deformed::{deformed_step, ClimbTick, ColumnStamp};
use astraweave_physics::traversal::{TraversalBody, TraversalInput};
use glam::Vec3;

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
fn deformed_movement_tick_does_not_allocate() {
    let stamp = ColumnStamp {
        x: 0,
        z: 0,
        heights: [[4, 4, 5], [4, 4, 5], [4, 4, 4]],
    };
    let mut body = TraversalBody::default();
    body.pos = Vec3::new(0.2, 4.0, 0.2);
    let mut anchor = [0, 4, 0];
    let mut attached = false;
    let input = TraversalInput {
        move_y: 1.0,
        ..TraversalInput::default()
    };
    for tick in 0..8 {
        deformed_step(
            &mut body,
            input,
            &stamp,
            &[],
            &mut anchor,
            &mut attached,
            ClimbTick {
                flags: FractureFlags::CLIMBABLE,
                tick,
            },
        );
    }
    ALLOCS.store(0, Ordering::SeqCst);
    for tick in 8..40 {
        deformed_step(
            &mut body,
            input,
            &stamp,
            &[],
            &mut anchor,
            &mut attached,
            ClimbTick {
                flags: FractureFlags::CLIMBABLE,
                tick,
            },
        );
    }
    let allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(allocs, 0, "movement tick allocated {allocs} times");
}
