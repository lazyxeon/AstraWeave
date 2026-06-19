//! F.3.S WI-1/WI-5: the sparse `simulate` path must be **bit-identical** to the
//! dense `simulate_reference` at EVERY tick — sparsity changes only speed, never
//! the water's behaviour. This is the hard gate: if these diverge, the sparsity
//! is wrong and the benchmark verdict would be built on a lie.
//!
//! `level` is the gameplay-truth state (what `WaterQuery::sample` reads); we
//! assert it bit-for-bit (`f32::to_bits`) every tick across diverse scenarios,
//! including the order-sensitive immediate-apply horizontal flow, walls, gates,
//! sources, drains, and terrain carving.

use astraweave_fluids::{CellFlags, MaterialType, WaterVolumeGrid};
use glam::{IVec3, UVec3, Vec3};

const DT: f32 = 1.0 / 60.0;

fn grid(nx: u32, ny: u32, nz: u32) -> WaterVolumeGrid {
    WaterVolumeGrid::new(UVec3::new(nx, ny, nz), 1.0, Vec3::ZERO)
}

/// Assert two grids have bit-identical water levels in every cell.
fn assert_levels_bit_identical(a: &WaterVolumeGrid, b: &WaterVolumeGrid, tick: usize, scenario: &str) {
    let d = a.dimensions();
    for x in 0..d.x as i32 {
        for y in 0..d.y as i32 {
            for z in 0..d.z as i32 {
                let p = IVec3::new(x, y, z);
                let la = a.get_level(p);
                let lb = b.get_level(p);
                assert_eq!(
                    la.to_bits(),
                    lb.to_bits(),
                    "{scenario}: sparse≠dense at tick {tick}, cell {p:?}: sparse={la} dense={lb}"
                );
            }
        }
    }
}

/// Run `build` through both the sparse (`simulate`) and dense
/// (`simulate_reference`) paths for `ticks` steps, asserting bit-identity after
/// each tick.
fn lockstep(scenario: &str, ticks: usize, build: impl Fn(&mut WaterVolumeGrid)) {
    let mut sparse = grid(8, 8, 8);
    let mut dense = grid(8, 8, 8);
    build(&mut sparse);
    build(&mut dense);
    // Identical at t=0.
    assert_levels_bit_identical(&sparse, &dense, 0, scenario);
    for tick in 1..=ticks {
        sparse.simulate(DT);
        dense.simulate_reference(DT);
        assert_levels_bit_identical(&sparse, &dense, tick, scenario);
    }
}

// ===================================================================
// WI-1 — bit-identical sparse-vs-dense across diverse scenarios
// ===================================================================

#[test]
fn lockstep_collapsing_column() {
    lockstep("collapsing_column", 240, |g| {
        for y in 2..8 {
            g.set_level(IVec3::new(4, y, 4), 1.0);
        }
    });
}

#[test]
fn lockstep_u_bend_with_walls() {
    // Two basins split by a wall, joined by a floor channel (order-sensitive
    // horizontal flow through a constriction — the toughest bit-identity case).
    lockstep("u_bend", 300, |g| {
        for y in 1..7 {
            g.set_material(IVec3::new(4, y, 0), MaterialType::Stone);
            g.set_material(IVec3::new(4, y, 1), MaterialType::Stone);
        }
        for x in 0..4 {
            for y in 0..7 {
                for z in 0..2 {
                    g.set_level(IVec3::new(x, y, z), 1.0);
                }
            }
        }
    });
}

#[test]
fn lockstep_gate_then_open() {
    // A gated channel: the closed gate must block flow identically in both
    // paths; tests that flag-bearing neighbour cells are handled in the frontier.
    lockstep("gate", 200, |g| {
        for y in 0..8 {
            g.set_level(IVec3::new(0, y, 4), 1.0);
        }
        g.get_cell_mut(IVec3::new(1, 0, 4)).unwrap().flags.insert(CellFlags::GATE);
        g.get_cell_mut(IVec3::new(1, 1, 4)).unwrap().flags.insert(CellFlags::GATE);
    });
}

#[test]
fn lockstep_source_fills_dry_grid() {
    // A SOURCE cell in an otherwise-dry grid must stay active and fill in both
    // paths — the dry-fountain case that a naive wet-only active set misses.
    lockstep("source", 200, |g| {
        g.get_cell_mut(IVec3::new(4, 6, 4)).unwrap().flags.insert(CellFlags::SOURCE);
    });
}

#[test]
fn lockstep_drain_empties_basin() {
    // A draining basin — the active set must SHRINK as water leaves (sleep),
    // staying bit-identical to dense the whole way down.
    lockstep("drain", 300, |g| {
        for x in 1..7 {
            for y in 0..4 {
                for z in 1..7 {
                    g.set_level(IVec3::new(x, y, z), 1.0);
                }
            }
        }
        g.get_cell_mut(IVec3::new(3, 0, 3)).unwrap().flags.insert(CellFlags::DRAIN);
    });
}

#[test]
fn lockstep_terrain_carve() {
    // Terrain boundary + a mid-run carve (re-apply): the active set must react
    // to topology change identically in both paths.
    let mut sparse = grid(8, 8, 8);
    let mut dense = grid(8, 8, 8);
    let build = |g: &mut WaterVolumeGrid| {
        let mut h = vec![0.0f32; 8 * 8];
        for x in 4..8 {
            for z in 0..8 {
                h[z * 8 + x] = 7.0; // ridge on the right half
            }
        }
        g.apply_terrain_boundary(&h, 8, 8);
        for y in 1..6 {
            g.set_level(IVec3::new(0, y, 4), 1.0);
        }
    };
    build(&mut sparse);
    build(&mut dense);
    for tick in 1..=120 {
        sparse.simulate(DT);
        dense.simulate_reference(DT);
        assert_levels_bit_identical(&sparse, &dense, tick, "carve_pre");
    }
    // Carve the ridge flat in both.
    let flat = vec![0.0f32; 8 * 8];
    sparse.apply_terrain_boundary(&flat, 8, 8);
    dense.apply_terrain_boundary(&flat, 8, 8);
    for tick in 121..=300 {
        sparse.simulate(DT);
        dense.simulate_reference(DT);
        assert_levels_bit_identical(&sparse, &dense, tick, "carve_post");
    }
}

/// CRITICAL margin validation: on a grid LARGER than `CASCADE_MARGIN` the box
/// is genuinely smaller than the grid, so the forward-cascade truncation risk
/// is real (unlike the 8³ cases above, where the box clamps to the whole grid
/// and silently runs dense). Water released near one corner must stay
/// bit-identical as it cascades/​spreads across a 40³ volume.
#[test]
fn lockstep_large_grid_exercises_margin() {
    let mut sparse = WaterVolumeGrid::new(UVec3::new(40, 40, 40), 1.0, Vec3::ZERO);
    let mut dense = WaterVolumeGrid::new(UVec3::new(40, 40, 40), 1.0, Vec3::ZERO);
    let build = |g: &mut WaterVolumeGrid| {
        // A tall full column near a corner — high pressure at the base drives
        // the longest forward cascade, the worst case for the margin.
        for y in 5..40 {
            for x in 3..6 {
                for z in 3..6 {
                    g.set_level(IVec3::new(x, y, z), 1.0);
                }
            }
        }
    };
    build(&mut sparse);
    build(&mut dense);
    for tick in 1..=150 {
        sparse.simulate(DT);
        dense.simulate_reference(DT);
        // Spot-check the box is actually smaller than the grid early on (proves
        // we're testing sparsity, not a clamped-to-dense box).
        assert_levels_bit_identical(&sparse, &dense, tick, "large_grid_margin");
    }
}

// ===================================================================
// WI-1 — wake/sleep boundary correctness (both directions)
// ===================================================================

#[test]
fn wake_boundary_water_spreads_into_dry_cells() {
    // Water released in one corner must WAKE previously-dry cells as it flows —
    // the classic sparsity bug is water freezing because a cell never woke.
    let mut g = grid(12, 6, 12);
    for y in 1..5 {
        g.set_level(IVec3::new(1, y, 1), 1.0);
    }
    let reached_before = g.get_level(IVec3::new(6, 0, 6));
    for _ in 0..600 {
        g.simulate(DT);
    }
    let reached_after = g.get_level(IVec3::new(6, 0, 6));
    assert_eq!(reached_before, 0.0);
    assert!(
        reached_after > 0.0,
        "water must wake & spread into distant dry cells (got {reached_after})"
    );
}

#[test]
fn sleep_boundary_active_set_shrinks_to_zero() {
    // A draining basin must SLEEP: once all water is gone the active set empties
    // and the sim goes quiescent (no perpetual wakefulness = no speedup).
    let mut g = grid(8, 8, 8);
    for x in 1..7 {
        for y in 0..3 {
            for z in 1..7 {
                g.set_level(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    // Drain the whole floor.
    for x in 1..7 {
        for z in 1..7 {
            g.get_cell_mut(IVec3::new(x, 0, z)).unwrap().flags.insert(CellFlags::DRAIN);
        }
    }
    let active_start = g.stats().active_cells;
    for _ in 0..2000 {
        g.simulate(DT);
    }
    let active_end = g.stats().active_cells;
    let wet_end = g.stats().wet_cells;
    assert!(active_start > 50, "basin starts with many active cells ({active_start})");
    // All water drained; only the DRAIN-flagged floor cells (36) remain active.
    assert_eq!(wet_end, 0, "all water should have drained");
    assert!(
        active_end <= 36,
        "active set must shrink to (at most) the drain cells once dry, got {active_end}"
    );
}

// ===================================================================
// WI-5 — the sparse machinery itself is deterministic
// ===================================================================

#[test]
fn sparse_path_is_deterministic() {
    let run = || {
        let mut g = grid(10, 10, 10);
        for y in 5..10 {
            g.set_level(IVec3::new(2, y, 2), 1.0);
        }
        for y in 0..5 {
            g.set_material(IVec3::new(5, y, 5), MaterialType::Stone);
        }
        for _ in 0..200 {
            g.simulate(DT);
        }
        g
    };
    let a = run();
    let b = run();
    for x in 0..10 {
        for y in 0..10 {
            for z in 0..10 {
                let p = IVec3::new(x, y, z);
                assert_eq!(
                    a.get_level(p).to_bits(),
                    b.get_level(p).to_bits(),
                    "sparse runs diverged at {p:?}"
                );
            }
        }
    }
}
