//! F.3.S WI-2 — the benchmark that answers the budget question: can the voxel
//! `WaterVolumeGrid::simulate` meet the ~1 ms gameplay-water budget on min-spec
//! via dirty-AABB sparsity?
//!
//! Measures PRODUCTION code: the sparse `simulate` vs the dense
//! `simulate_reference` (the pre-F.3.S algorithm, bit-identical) on the SAME
//! machine, so the speedup ratio is exact. Two shape families, because fill
//! ratio AND shape both drive sparsity:
//!
//! - **basin**: flat settled water over the FULL x,z floor at depth = fill.
//!   Realistic flooded-area water. Every column is wet, so `compute_pressure`
//!   cannot sparsify — only flow does. The honest common case.
//! - **pool**: a stone-walled cube of water in one corner (localized). Few wet
//!   columns, so pressure sparsifies too — the clustered best case.
//!
//! All scenarios are SETTLED (flat, contained), so each timed tick is a clean
//! steady-state cost (no transient drift across criterion iterations).
//!
//! Record results in MASTER_BENCHMARK_REPORT.md alongside F.1's dense baselines.
//! Machine context MUST be recorded (min-spec class — the budget target).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::{IVec3, UVec3, Vec3};

use astraweave_fluids::{MaterialType, WaterVolumeGrid};

const DT: f32 = 1.0 / 60.0;

/// Flat settled basin: stone floor, water filling the bottom `fill` fraction of
/// the height across the full x,z extent. Wet-cell fraction ≈ `fill`.
fn make_basin(n: u32, fill: f32) -> WaterVolumeGrid {
    let mut g = WaterVolumeGrid::new(UVec3::new(n, n, n), 1.0, Vec3::ZERO);
    let wet_h = ((n as f32 * fill).round() as i32).clamp(1, n as i32);
    for x in 0..n as i32 {
        for z in 0..n as i32 {
            g.set_material(IVec3::new(x, 0, z), MaterialType::Stone);
            for y in 1..wet_h {
                g.set_level(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    // Settle any residual before timing (one cheap tick); the basin is already
    // flat so this is essentially a no-op but normalises the box state.
    g.simulate(DT);
    g
}

/// Localized stone-walled pool of side `s` in the corner, filled flat. Wet ≈ s³
/// cells in an (s+2)³ corner; the rest of the grid is empty air.
fn make_pool(n: u32, s: i32) -> WaterVolumeGrid {
    let mut g = WaterVolumeGrid::new(UVec3::new(n, n, n), 1.0, Vec3::ZERO);
    // Stone shell: floor + 4 walls of a container occupying [0..=s+1].
    for a in 0..=s + 1 {
        for b in 0..=s + 1 {
            g.set_material(IVec3::new(a, 0, b), MaterialType::Stone);
            g.set_material(IVec3::new(0, a, b), MaterialType::Stone);
            g.set_material(IVec3::new(s + 1, a, b), MaterialType::Stone);
            g.set_material(IVec3::new(a, b, 0), MaterialType::Stone);
            g.set_material(IVec3::new(a, b, s + 1), MaterialType::Stone);
        }
    }
    for x in 1..=s {
        for y in 1..=s {
            for z in 1..=s {
                g.set_level(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    g.simulate(DT);
    g
}

fn sample_size_for(n: u32) -> usize {
    match n {
        0..=32 => 100,
        33..=64 => 50,
        _ => 12, // 128³ dense ≈ 200 ms/tick — keep wall-clock sane
    }
}

fn bench_basin(c: &mut Criterion) {
    let mut group = c.benchmark_group("voxel_sparsity_basin");
    for n in [32u32, 64, 128] {
        group.sample_size(sample_size_for(n));
        for fill_pct in [5u32, 25, 50, 100] {
            let fill = fill_pct as f32 / 100.0;
            group.bench_with_input(
                BenchmarkId::new(format!("sparse/{n}"), fill_pct),
                &(n, fill),
                |b, &(n, fill)| {
                    let mut g = make_basin(n, fill);
                    b.iter(|| g.simulate(std::hint::black_box(DT)));
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("dense/{n}"), fill_pct),
                &(n, fill),
                |b, &(n, fill)| {
                    let mut g = make_basin(n, fill);
                    b.iter(|| g.simulate_reference(std::hint::black_box(DT)));
                },
            );
        }
    }
    group.finish();
}

fn bench_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("voxel_sparsity_pool");
    let n = 64u32;
    group.sample_size(50);
    for s in [12i32, 24, 40] {
        let frac = (s * s * s) as f32 / (n * n * n) as f32;
        let pct = (frac * 100.0).round() as u32;
        group.bench_with_input(
            BenchmarkId::new(format!("sparse/{n}/side{s}"), pct),
            &s,
            |b, &s| {
                let mut g = make_pool(n, s);
                b.iter(|| g.simulate(std::hint::black_box(DT)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new(format!("dense/{n}/side{s}"), pct),
            &s,
            |b, &s| {
                let mut g = make_pool(n, s);
                b.iter(|| g.simulate_reference(std::hint::black_box(DT)));
            },
        );
    }
    group.finish();
}

criterion_group!(sparsity, bench_basin, bench_pool);
criterion_main!(sparsity);
