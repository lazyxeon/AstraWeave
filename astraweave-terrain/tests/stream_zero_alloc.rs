//! The stream tick must not allocate. `new` reserves the ring and the log.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use astraweave_terrain::column_stream::ColumnStream;
use astraweave_terrain::destruction_seam::{Brush, BrushOp, BrushShape};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn stream_tick_allocates_nothing_after_warmup() {
    let mut stream = ColumnStream::new();
    let _ = stream.around(0, 0, 2);
    stream
        .append_brush(Brush {
            shape: BrushShape::Sphere,
            op: BrushOp::Subtract,
            origin: [4, 2, 4],
            radius: 3,
            half_extents: [1, 1, 1],
            strength: 4,
        })
        .expect("brush");
    let _ = stream.around(6, 0, 2);
    let _ = stream.around(0, 0, 2);
    let _ = stream.lod_for_camera(0, 0);

    ALLOCS.store(0, Ordering::SeqCst);
    let moved = stream.around(1, 0, 2);
    let lod = stream.lod_for_camera(1, 0);
    let back = stream.around(0, 0, 2);
    let allocs = ALLOCS.load(Ordering::SeqCst);
    assert_eq!(allocs, 0, "stream tick allocated {allocs} times");
    assert!(!moved.added().is_empty());
    assert!(!lod.samples().is_empty());
    assert!(!back.removed().is_empty());
}
