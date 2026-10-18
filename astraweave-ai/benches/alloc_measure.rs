//! Allocation count for the deterministic gather method.
//!
//! The symbolic search this bench used to measure is gone. The method builds
//! one plan id and a fixed step list.

#[cfg(feature = "alloc-counter")]
mod bench_alloc {
    use std::alloc::{GlobalAlloc, Layout};

    #[cfg(not(feature = "fast-alloc"))]
    static INNER: std::alloc::System = std::alloc::System;
    #[cfg(feature = "fast-alloc")]
    static INNER: astraweave_alloc::MiMalloc = astraweave_alloc::MiMalloc;

    pub struct BenchAlloc;

    unsafe impl GlobalAlloc for BenchAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            INNER.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            astraweave_profiling::counters::record_dealloc(layout.size());
            INNER.dealloc(ptr, layout)
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            INNER.alloc_zeroed(layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            astraweave_profiling::counters::record_realloc(layout.size(), new_size);
            INNER.realloc(ptr, layout, new_size)
        }
    }
}

#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: bench_alloc::BenchAlloc = bench_alloc::BenchAlloc;

use astraweave_ai::htn::gather_method;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// One plan id plus the step vector. Well under the old search ceiling.
const MAX_ALLOCS_GATHER: usize = 32;

fn bench_gather(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai.htn.gather");
    group.bench_function("keep_wood", |b| {
        b.iter(|| {
            let plan = gather_method(black_box(Some("gather")));
            black_box(plan);
        });
    });
    group.finish();

    #[cfg(feature = "alloc-counter")]
    {
        use astraweave_profiling::FrameAllocStats;
        let _ = gather_method(Some("gather"));
        let stats = FrameAllocStats::begin_frame();
        let plan = gather_method(Some("gather"));
        let delta = stats.end_frame();
        black_box(plan);
        println!(
            "[alloc-measure] ai.htn.gather: allocs={} bytes={} reallocs={} (threshold={})",
            delta.allocs, delta.bytes_allocated, delta.reallocs, MAX_ALLOCS_GATHER
        );
        assert!(
            delta.allocs <= MAX_ALLOCS_GATHER,
            "gather method regression: {} allocs > threshold {}",
            delta.allocs,
            MAX_ALLOCS_GATHER
        );
    }
}

criterion_group!(benches, bench_gather);
criterion_main!(benches);
