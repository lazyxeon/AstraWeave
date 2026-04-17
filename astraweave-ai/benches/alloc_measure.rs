// SPDX-License-Identifier: MIT
//! Allocation-count regression bench for `AdvancedGOAP::plan_direct` (a.k.a. the
//! `ai.goap.plan` Tracy span).
//!
//! From the audit (§2.3 #1, open question #2): this is the highest-volume
//! inferred allocation site in the engine. A* expansion clones `WorldState` and
//! `Vec<String>` + allocates a `String` per action name per expansion. This
//! bench gives the first real per-plan allocation count.
//!
//! # How to run
//!
//! ```bash
//! cargo bench -p astraweave-ai --features alloc-counter --bench alloc_measure
//! ```

// Local counting allocator. Inner = System by default, MiMalloc with `fast-alloc`.
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

use astraweave_ai::goap::{AdvancedGOAP, Goal, SimpleAction, StateValue, WorldState};
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

/// Placeholder threshold. From audit inference: up to ~4 allocations per
/// expansion × up to `max_plan_iterations` (default 10000). Set generously.
const MAX_ALLOCS_GOAP_PLAN: usize = 200_000;

/// Build a mini-GOAP problem with N actions so the planner actually does search
/// work. Structured as a linear chain: a0 enables a1 enables a2 ... enables goal.
fn setup_goap(n_actions: usize) -> (AdvancedGOAP, WorldState, Goal) {
    let mut goap = AdvancedGOAP::new();

    // Action a_i has precondition "step_{i} = true" and effect "step_{i+1} = true".
    // step_0 is true in the starting state.
    for i in 0..n_actions {
        let mut preconditions = BTreeMap::new();
        preconditions.insert(format!("step_{}", i), StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert(format!("step_{}", i + 1), StateValue::Bool(true));

        goap.add_action(Box::new(SimpleAction::new(
            // Leak to satisfy 'static bound on action name.
            Box::leak(format!("act_{}", i).into_boxed_str()),
            preconditions,
            effects,
            1.0,
        )));
    }

    let mut start = WorldState::new();
    start.set("step_0", StateValue::Bool(true));

    let mut goal_state = BTreeMap::new();
    goal_state.insert(format!("step_{}", n_actions), StateValue::Bool(true));
    let goal = Goal::new("reach_end", goal_state);

    (goap, start, goal)
}

fn bench_goap_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai.goap.plan");
    for n_actions in [4_usize, 16, 64] {
        group.bench_function(format!("actions_{}", n_actions), |b| {
            let (goap, start, goal) = setup_goap(n_actions);
            b.iter(|| {
                let plan = goap.plan(black_box(&start), black_box(&goal));
                black_box(plan);
            });
        });
    }
    group.finish();

    #[cfg(feature = "alloc-counter")]
    {
        use astraweave_profiling::FrameAllocStats;
        let (goap, start, goal) = setup_goap(16);

        // Warmup: planner may do lazy init the first time.
        let _ = goap.plan(&start, &goal);

        let stats = FrameAllocStats::begin_frame();
        let plan = goap.plan(&start, &goal);
        let delta = stats.end_frame();
        black_box(plan);

        println!(
            "[alloc-measure] ai.goap.plan (actions=16): allocs={} bytes={} reallocs={} (threshold={})",
            delta.allocs, delta.bytes_allocated, delta.reallocs, MAX_ALLOCS_GOAP_PLAN
        );
        assert!(
            delta.allocs <= MAX_ALLOCS_GOAP_PLAN,
            "ai.goap.plan regression: {} allocs > threshold {}",
            delta.allocs,
            MAX_ALLOCS_GOAP_PLAN
        );
    }
}

criterion_group!(benches, bench_goap_plan);
criterion_main!(benches);
