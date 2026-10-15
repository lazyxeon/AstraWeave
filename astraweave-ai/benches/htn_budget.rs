//! Budget check for [`HTN_EXPANSIONS_PER_TICK`] against the 11 ms frame.

use astraweave_ai::htn::{plan_route, AgentPlan, SpatialHooks, HTN_EXPANSIONS_PER_TICK};
use astraweave_nav::{bridge_and_ford, NavScratch};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn bench_htn_expansions(c: &mut Criterion) {
    let tiles = bridge_and_ford();
    let hooks = SpatialHooks::none();
    c.bench_function("htn_expansions_per_tick", |b| {
        b.iter(|| {
            let mut scratch = NavScratch::new();
            let mut budget = HTN_EXPANSIONS_PER_TICK as u32;
            let mut plan = AgentPlan::empty();
            while budget > 0 {
                let before = budget;
                plan_route(
                    &tiles,
                    &mut scratch,
                    [1_000, 0, 1_000],
                    [1_000, 0, 46_000],
                    &hooks,
                    1,
                    &mut plan,
                    &mut budget,
                );
                if budget == before {
                    break;
                }
            }
            black_box(plan.len)
        });
    });
}

criterion_group!(htn_budget, bench_htn_expansions);
criterion_main!(htn_budget);
