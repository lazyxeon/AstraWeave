//! WI-5 budget input: cost of an analytic `WaterQuery::sample` — the per-body,
//! per-tick price physics pays for the gameplay-water layer. The whole point
//! of recording this is that the CPU-analytic facade is *cheap* (microseconds),
//! so it does not contend for the GPU/voxel budget that F.3/F.4 must size; this
//! bench makes that quantitative rather than asserted.
//!
//! `query_one_body` = a single sample (one buoyancy body). `query_fleet`
//! amortizes a realistic fleet of buoyant bodies sampling the water once each
//! per tick, at increasing registered-volume counts (overlap resolution scans
//! all containing volumes).

use astraweave_water::{AnalyticWater, WaterQuery};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec3;

/// A water world with `n` AABB volumes laid out on a grid, plus a global plane.
fn world_with(n: usize) -> AnalyticWater {
    let mut w = AnalyticWater::new();
    w.set_plane(0.0, 1000.0);
    let side = (n as f32).sqrt().ceil() as i32;
    for i in 0..n as i32 {
        let gx = (i % side) as f32 * 12.0;
        let gz = (i / side) as f32 * 12.0;
        w.add_aabb(
            Vec3::new(gx, 0.0, gz),
            Vec3::new(gx + 10.0, 5.0, gz + 10.0),
            1000.0,
            0.5,
        );
    }
    w
}

fn bench_query(c: &mut Criterion) {
    // Single body: the marginal per-body cost.
    {
        let w = world_with(8);
        let p = Vec3::new(2.0, 1.0, 2.0); // inside volume 0
        c.bench_function("query_one_body/8_volumes", |b| {
            b.iter(|| std::hint::black_box(w.sample(std::hint::black_box(p))))
        });
    }

    // Fleet: a plausible count of buoyant bodies each sampling once per tick,
    // across growing registered-volume counts (the overlap scan cost).
    let mut group = c.benchmark_group("query_fleet_per_tick");
    const BODIES: usize = 256;
    for volumes in [1usize, 16, 64, 256] {
        let w = world_with(volumes);
        // Spread bodies over the populated region so containment varies.
        let side = (volumes as f32).sqrt().ceil().max(1.0) * 12.0;
        let bodies: Vec<Vec3> = (0..BODIES)
            .map(|i| {
                let t = i as f32 / BODIES as f32;
                Vec3::new(t * side, 1.0, (1.0 - t) * side)
            })
            .collect();
        group.bench_with_input(
            BenchmarkId::from_parameter(volumes),
            &volumes,
            |b, _| {
                b.iter(|| {
                    let mut acc = 0.0f32;
                    for p in &bodies {
                        if let Some(s) = w.sample(std::hint::black_box(*p)) {
                            acc += s.surface_height;
                        }
                    }
                    std::hint::black_box(acc)
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_query);
criterion_main!(benches);
