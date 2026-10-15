//! CPU time for one simulation tick.
//!
//! Nightly artifact only. This is not the 11 ms GTX 1660 Ti frame budget;
//! that capture is `scripts/ci/gpu_frame_budget.sh` on a self-hosted GPU runner.

use astraweave_core::{step, IVec2, SimConfig, Team, World};
use std::time::Instant;

#[test]
#[ignore = "nightly CPU tick-time artifact; not the 11 ms GPU budget"]
fn cpu_sim_tick_time() {
    let mut world = World::new();
    let entity = world.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
    world
        .cooldowns_mut(entity)
        .unwrap()
        .map
        .insert("attack".into(), 5.0);
    let cfg = SimConfig { dt: 1.0 / 60.0 };
    for _ in 0..100 {
        step(&mut world, &cfg);
    }
    let n = 10_000u32;
    let start = Instant::now();
    for _ in 0..n {
        step(&mut world, &cfg);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(n);
    println!("[tick-time] world.step ms_per_tick={ms:.6} n={n}");
}
