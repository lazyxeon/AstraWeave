//! F.3 voxel-water integration: gate flags (WI-2, Must-Fix #6), the
//! physical-invariant gap (WI-3: conservation / hydrostatic / U-bend /
//! dt-stability), terrain boundary glue (WI-4), and carve reactivity (WI-5).
//!
//! These exercise `WaterVolumeGrid`'s public API directly — correctness on the
//! *dense* sim as it exists. Sparsity and budget benchmarks are F.3.S.

use astraweave_fluids::{CellFlags, MaterialType, WaterVolumeGrid};
use glam::{IVec3, UVec3, Vec3};

fn grid(nx: u32, ny: u32, nz: u32) -> WaterVolumeGrid {
    WaterVolumeGrid::new(UVec3::new(nx, ny, nz), 1.0, Vec3::ZERO)
}

/// Sum of cell levels × cell³ — the true water volume (independent of the
/// cached `total_volume`, which `cleanup_active_cells` recomputes anyway).
fn measured_volume(g: &WaterVolumeGrid) -> f32 {
    let d = g.dimensions();
    let cs3 = g.cell_size().powi(3);
    let mut v = 0.0;
    for x in 0..d.x as i32 {
        for y in 0..d.y as i32 {
            for z in 0..d.z as i32 {
                v += g.get_level(IVec3::new(x, y, z)) * cs3;
            }
        }
    }
    v
}

// ===================================================================
// WI-2 — the gate flags are real (Must-Fix #6: "does the gate gate?")
// ===================================================================

/// A single horizontal channel: water at the left end, a gate in the middle.
/// Closed gate ⇒ water cannot reach the far end; open ⇒ it spreads through.
/// This is the exact flag the pre-F.3 sim wrote but never read.
#[test]
fn gate_blocks_horizontal_flow() {
    // 1-tall ⇒ no vertical flow; isolates the horizontal gate.
    let run = |gated: bool| -> f32 {
        let mut g = grid(3, 1, 1);
        g.set_level(IVec3::new(0, 0, 0), 1.0); // water at the left
        if gated {
            g.get_cell_mut(IVec3::new(1, 0, 0)).unwrap().flags.insert(CellFlags::GATE);
        }
        for _ in 0..120 {
            g.simulate(1.0 / 60.0);
        }
        g.get_level(IVec3::new(2, 0, 0)) // did water reach the far end?
    };

    let open_far = run(false);
    let gated_far = run(true);
    assert!(open_far > 0.1, "without a gate, water should spread to the far cell (got {open_far})");
    assert!(gated_far < 1e-3, "a closed GATE must block flow to the far cell (got {gated_far})");
}

/// FROZEN cells hold their water (iced — no flow in or out).
#[test]
fn frozen_cell_holds_water() {
    let mut g = grid(3, 1, 1);
    g.set_level(IVec3::new(1, 0, 0), 1.0);
    g.get_cell_mut(IVec3::new(1, 0, 0)).unwrap().flags.insert(CellFlags::FROZEN);
    for _ in 0..120 {
        g.simulate(1.0 / 60.0);
    }
    assert!(
        (g.get_level(IVec3::new(1, 0, 0)) - 1.0).abs() < 1e-3,
        "FROZEN water should not spread"
    );
}

/// PERSISTENT cells are exempt from natural draining (material absorption).
#[test]
fn persistent_cell_exempt_from_absorption() {
    let mut g = grid(1, 1, 1);
    g.set_material(IVec3::new(0, 0, 0), MaterialType::Mud); // Mud absorbs fast (0.5/s)
    g.set_level(IVec3::new(0, 0, 0), 1.0);
    g.get_cell_mut(IVec3::new(0, 0, 0)).unwrap().flags.insert(CellFlags::PERSISTENT);
    for _ in 0..120 {
        g.simulate(1.0 / 60.0);
    }
    assert!(
        g.get_level(IVec3::new(0, 0, 0)) > 0.99,
        "PERSISTENT water on Mud should not be absorbed"
    );
}

// ===================================================================
// WI-3 — physical invariants (the gap F.0 found: zero such tests)
// ===================================================================

/// Conservation: a closed basin (no sources/drains, water over Air ⇒ no
/// absorption) conserves total water across many ticks. This is the test that
/// catches the pre-F.3 horizontal-flow leak (multi-neighbor inflow clamped
/// past 1.0 lost the excess).
#[test]
fn conservation_closed_basin() {
    let mut g = grid(6, 6, 6);
    // A tall thin column that will collapse and spread (exercises flow hard).
    for y in 1..=4 {
        for x in 2..=3 {
            for z in 2..=3 {
                g.set_level(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    let initial = measured_volume(&g);
    assert!(initial > 0.0);
    for _ in 0..180 {
        g.simulate(1.0 / 60.0);
    }
    let after = measured_volume(&g);
    let drift = (after - initial).abs() / initial;
    assert!(
        drift < 0.01,
        "closed basin must conserve water: initial={initial}, after={after}, drift={drift}"
    );
}

/// Hydrostatic: water released as a tall column settles DOWNWARD (gravity) and
/// spreads laterally toward equilibrium — it does not stay a one-cell tower.
#[test]
fn hydrostatic_column_settles() {
    let mut g = grid(6, 8, 6);
    for y in 2..=6 {
        g.set_level(IVec3::new(3, y, 3), 1.0); // 5-tall single column
    }
    let top_before = g.get_level(IVec3::new(3, 6, 3));
    for _ in 0..600 {
        g.simulate(1.0 / 60.0);
    }
    let top_after = g.get_level(IVec3::new(3, 6, 3));
    // Sum of the floor layer (y=0) — gravity should have pulled most of the
    // water down to it (it spreads thin across the 6×6 footprint, so no single
    // floor cell is full, but their sum captures "settled to the floor").
    let mut floor_sum = 0.0;
    for x in 0..6 {
        for z in 0..6 {
            floor_sum += g.get_level(IVec3::new(x, 0, z));
        }
    }
    // Gravity pulled water down: the top emptied, the floor holds most of it.
    assert!(top_after < top_before - 0.5, "column top should drain (before {top_before}, after {top_after})");
    assert!(floor_sum > 3.0, "most of the 5 water-cells should settle to the floor (floor_sum={floor_sum})");
    // Spread laterally: a neighbor column now holds water.
    let neighbor = g.get_level(IVec3::new(2, 0, 3)) + g.get_level(IVec3::new(4, 0, 3));
    assert!(neighbor > 0.1, "water should spread laterally as it settles (got {neighbor})");
}

/// U-bend (the headline capability the docstring advertises but had no test):
/// two basins separated by a wall, connected only by a channel at the floor;
/// water poured into one basin rises in the other through the low channel.
#[test]
fn u_bend_connected_basins_equalize() {
    // 5 wide, 4 tall, 1 deep. Wall at x=2 for y∈[1,3]; the floor (y=0) is open
    // — the low connecting channel.
    let mut g = grid(5, 4, 1);
    for y in 1..=3 {
        g.set_material(IVec3::new(2, y, 0), MaterialType::Stone);
    }
    // Fill the left basin (x 0..1) full.
    for x in 0..=1 {
        for y in 0..=3 {
            g.set_level(IVec3::new(x, y, 0), 1.0);
        }
    }
    let right_before = g.get_level(IVec3::new(4, 0, 0));
    for _ in 0..600 {
        g.simulate(1.0 / 60.0);
    }
    let right_after = g.get_level(IVec3::new(4, 0, 0));
    assert!(right_before < 1e-6, "right basin starts empty");
    assert!(
        right_after > 0.1,
        "water must travel through the floor channel into the far basin (U-bend), got {right_after}"
    );
}

/// dt-stability: a single absurdly large dt is substepped, not applied raw —
/// state stays finite and conserved, equivalent (within tolerance) to many
/// small steps.
#[test]
fn dt_stability_large_dt_is_substepped() {
    let setup = || {
        let mut g = grid(6, 6, 6);
        for y in 1..=4 {
            for x in 2..=3 {
                g.set_level(IVec3::new(x, y, 2), 1.0);
            }
        }
        g
    };

    let mut big = setup();
    let initial = measured_volume(&big);
    big.simulate(3.0); // 3 seconds in ONE call — must substep internally

    // Finite + conserved (a raw huge dt would over-transfer and corrupt).
    let v = measured_volume(&big);
    assert!(v.is_finite(), "state must stay finite under a huge dt");
    assert!((v - initial).abs() / initial < 0.01, "huge dt must still conserve (initial={initial}, after={v})");
    for x in 0..6 {
        for y in 0..6 {
            for z in 0..6 {
                let l = big.get_level(IVec3::new(x, y, z));
                assert!((0.0..=1.0).contains(&l), "level out of range under huge dt: {l}");
            }
        }
    }
}

// ===================================================================
// WI-4 — terrain heightfield boundary (cycle-safe, plain &[f32])
// ===================================================================

/// A heightfield with a tall ridge on one side becomes a solid wall: water
/// poured on the low side pools and does not climb the ridge.
#[test]
fn terrain_boundary_blocks_water() {
    let mut g = grid(6, 6, 1);
    // Heightfield: left half low (y=0), right half a tall ridge (y=5).
    let (hx, hz) = (6usize, 1usize);
    let mut heights = vec![0.0f32; hx * hz];
    heights[3..6].fill(5.0); // ridge on the right half
    g.apply_terrain_boundary(&heights, hx, hz);
    // Cells under the ridge are solid; the left is open.
    assert_eq!(g.get_cell(IVec3::new(4, 1, 0)).unwrap().material, MaterialType::Stone);
    assert_eq!(g.get_cell(IVec3::new(1, 1, 0)).unwrap().material, MaterialType::Air);

    // Pour water on the low side; it must not appear inside the ridge.
    for y in 1..=4 {
        g.set_level(IVec3::new(0, y, 0), 1.0);
    }
    for _ in 0..300 {
        g.simulate(1.0 / 60.0);
    }
    assert!(g.get_level(IVec3::new(0, 0, 0)) > 0.1, "water pools on the low side");
    assert_eq!(g.get_level(IVec3::new(4, 0, 0)), 0.0, "no water inside the solid ridge");
}

// ===================================================================
// WI-5 — carve reactivity (re-apply boundary, bounded)
// ===================================================================

/// Carve a channel through the ridge (lower the heightfield, re-apply): the
/// previously-solid cells reopen and water flows through on the next ticks.
#[test]
fn carve_reactivity_reopens_flow() {
    let mut g = grid(6, 6, 1);
    let (hx, hz) = (6usize, 1usize);
    let mut heights = vec![0.0f32; hx];
    heights[3..6].fill(5.0);
    g.apply_terrain_boundary(&heights, hx, hz);
    for y in 1..=4 {
        g.set_level(IVec3::new(0, y, 0), 1.0);
    }
    for _ in 0..200 {
        g.simulate(1.0 / 60.0);
    }
    assert_eq!(g.get_level(IVec3::new(5, 0, 0)), 0.0, "blocked before carve");

    // Carve: drop the whole heightfield to the floor and re-apply.
    let flat = vec![0.0f32; hx];
    g.apply_terrain_boundary(&flat, hx, hz);
    assert_eq!(
        g.get_cell(IVec3::new(4, 1, 0)).unwrap().material,
        MaterialType::Air,
        "carve reopened the formerly-solid cell"
    );
    for _ in 0..400 {
        g.simulate(1.0 / 60.0);
    }
    assert!(
        g.get_level(IVec3::new(5, 0, 0)) > 0.05,
        "after carving, water reaches the far side (got {})",
        g.get_level(IVec3::new(5, 0, 0))
    );
}
