//! The traced spatial tick does not allocate after the rings exist.
//!
//! ```text
//! cargo test -p astraweave-ai --features decision-trace,alloc-counter --test decision_trace_alloc -- --test-threads=1
//! ```

#![cfg(all(feature = "decision-trace", feature = "alloc-counter"))]

use astraweave_ai::decision_trace::DecisionTrace;
use astraweave_ai::htn::{plan_route, AgentPlan, SpatialHooks};
use astraweave_ecs::counting_alloc::{allocs, reset_allocs};
use astraweave_nav::{bridge_and_ford, NavScratch};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[test]
fn traced_plan_route_allocates_nothing_after_warmup() {
    let tiles = bridge_and_ford();
    let mut scratch = NavScratch::new();
    let hooks = SpatialHooks::none();
    let mut trace = DecisionTrace::new();
    assert!(trace.bind(1));
    for tick in 0..4u64 {
        let mut budget = 64u32;
        let mut plan = AgentPlan::empty();
        plan_route(
            &tiles,
            &mut scratch,
            [1_000, 0, 1_000],
            [1_000, 0, 46_000],
            &hooks,
            tick,
            &mut plan,
            &mut budget,
            &mut trace,
            1,
        );
    }

    reset_allocs();
    let mut budget = 64u32;
    let mut plan = AgentPlan::empty();
    plan_route(
        &tiles,
        &mut scratch,
        [1_000, 0, 1_000],
        [1_000, 0, 46_000],
        &hooks,
        4,
        &mut plan,
        &mut budget,
        &mut trace,
        1,
    );
    assert_eq!(allocs(), 0, "traced plan_route allocated");
    assert!(trace.count_of(1) > 4);
}
