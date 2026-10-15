//! `tick` must not allocate. The pool is built in `LocalSim::new`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use astraweave_localsim::{
    demo_materials, FieldWrite, FloorSolid, LocalSim, Shape, Subject, TickInput, WriteOp,
};

static ARMED: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct TickGuard;

unsafe impl GlobalAlloc for TickGuard {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GUARD: TickGuard = TickGuard;

#[test]
fn tick_does_not_allocate() {
    let mut sim = LocalSim::new();
    let materials = demo_materials();
    let solid = FloorSolid::closed(0);
    let subjects = [Subject {
        id: 1,
        cell: [0, 0, 0],
        player: true,
    }];
    let pour = [FieldWrite {
        seq: 0,
        shape: Shape::Point,
        origin: [0, 2, 0],
        end: [0, 2, 0],
        radius: 0,
        op: WriteOp::AddVolume { delta: 40 },
    }];
    ALLOCS.store(0, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
    sim.tick(
        1,
        TickInput {
            subjects: &subjects,
            writes: &pour,
            materials: &materials,
            zones: &[],
        },
        &solid,
    );
    let _ = sim.field_digest();
    ARMED.store(false, Ordering::Relaxed);
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed),
        0,
        "tick or field_digest allocated"
    );
}
