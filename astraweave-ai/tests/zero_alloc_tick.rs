//! Zero-allocation proof for the spatial tick and tile commit.
//!
//! Run with one thread. The counter is process-global.
//!
//! ```text
//! cargo test -p astraweave-ai --features alloc-counter --test zero_alloc_tick -- --test-threads=1
//! ```

#![cfg(feature = "alloc-counter")]

use astraweave_ai::htn::{tick_agents, SpatialTick};
use astraweave_ecs::counting_alloc::{allocs, reset_allocs};
use astraweave_nav::{
    nav_change_from_geometry_notice, GeometryNotice, NavScratch, NOTICE_COLLAPSE,
};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[test]
fn htn_tick_and_tile_commit_allocate_nothing_after_warmup() {
    let mut tick = SpatialTick::demo_agent();
    let mut scratch = NavScratch::new();
    for step in 0..4u64 {
        let _ = tick.nav.commit_tick();
        tick_agents(
            &mut tick,
            &mut scratch,
            step,
            #[cfg(feature = "decision-trace")]
            &mut astraweave_ai::decision_trace::NullSink,
        );
    }
    let collapse = GeometryNotice {
        tick: 8,
        x0: -1_000,
        y0: -1_000,
        z0: 17_000,
        x1: 5_000,
        y1: 1_000,
        z1: 31_999,
        reason: NOTICE_COLLAPSE,
    };
    let warmup = nav_change_from_geometry_notice(&collapse, 0).expect("notice");
    tick.nav.mark(warmup);
    let _ = tick.nav.commit_tick();
    tick_agents(
        &mut tick,
        &mut scratch,
        8,
        #[cfg(feature = "decision-trace")]
        &mut astraweave_ai::decision_trace::NullSink,
    );

    reset_allocs();
    let again = nav_change_from_geometry_notice(&collapse, 1).expect("notice");
    tick.nav.mark(again);
    let committed = tick.nav.commit_tick();
    let commit_allocs = allocs();

    reset_allocs();
    tick_agents(
        &mut tick,
        &mut scratch,
        9,
        #[cfg(feature = "decision-trace")]
        &mut astraweave_ai::decision_trace::NullSink,
    );
    let tick_allocs = allocs();

    assert!(committed >= 1, "warmup left a dirty tile to commit");
    assert_eq!(commit_allocs, 0, "tile commit allocated after warmup");
    assert_eq!(tick_allocs, 0, "HTN tick allocated after warmup");
}
