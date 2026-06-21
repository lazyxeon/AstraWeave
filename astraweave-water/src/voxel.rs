//! Voxel [`WaterQuery`] backend (F.3, feature `voxel`).
//!
//! Implements the gameplay-water-truth trait for `astraweave-fluids`'s
//! deterministic [`WaterVolumeGrid`], so the voxel sim can serve as a backend
//! anywhere a [`WaterQuery`] is consumed — exactly like [`AnalyticWater`], with
//! no signature change to the trait (the F.2-designed seam, now realized).
//!
//! **Cycle-safety**: this is the `astraweave-water → astraweave-fluids` edge.
//! Fluids is a Cargo leaf (no `astraweave-terrain`/`-physics`/`-water` deps),
//! so `water → fluids → (leaf)` is acyclic and cannot close the forbidden
//! `physics → fluids → terrain → gameplay → physics`. The edge is feature-gated
//! so a consumer wanting only analytic water never pulls the fluids weight.
//!
//! **Determinism** (gate Q1): `WaterVolumeGrid` is deterministic by
//! construction (F.0-verified: fixed iteration order, no RNG/threads/hash
//! iteration) and `sample` is a pure column scan, so this backend preserves the
//! gameplay-truth determinism contract — proven by [`tests`].
//!
//! [`AnalyticWater`]: crate::AnalyticWater

use crate::{WaterQuery, WaterSample};
use astraweave_fluids::WaterVolumeGrid;
use glam::{IVec3, Vec3};

/// Density reported for voxel water. The voxel sim models a single fluid
/// (water); cells carry a fill *level*, not a per-cell density, so the backend
/// reports a constant fresh-water density. (A multi-fluid voxel grid would
/// carry density per material; that is not what `WaterVolumeGrid` models.)
pub const VOXEL_WATER_DENSITY: f32 = 1000.0;

/// A column needs at least this fill fraction in its highest wet cell for the
/// surface to count as "water here". Below it, a cell holds only numerical
/// residue with no meaningful surface, so the column reads as dry.
const SURFACE_MIN_LEVEL: f32 = 0.01;

impl WaterQuery for WaterVolumeGrid {
    /// Water surface + density at `point`, or `None` if the column is dry or
    /// the point lies outside the grid footprint.
    ///
    /// Mapping:
    /// - `point.xz` is floored to a grid column; **outside the grid's XZ
    ///   footprint → `None`** (a bounded volume, like an `AnalyticWater` AABB).
    /// - Scanning that column top-down, the **highest cell with fill
    ///   `> SURFACE_MIN_LEVEL`** defines the surface: its world-space top is
    ///   `origin.y + (cell_y + level) * cell_size` (the water fills the cell
    ///   from its floor up by `level`). Density is [`VOXEL_WATER_DENSITY`].
    /// - **No vertical gate**: `point.y` does not affect the result (the caller
    ///   compares `point.y` to `surface_height` itself, matching the analytic
    ///   backend's contract). A dry column → `None`.
    fn sample(&self, point: Vec3) -> Option<WaterSample> {
        let dim = self.dimensions();
        let origin = self.origin();
        let cs = self.cell_size();

        // Lateral footprint gate (floor division, matching `world_to_grid`).
        let gx = ((point.x - origin.x) / cs).floor() as i32;
        let gz = ((point.z - origin.z) / cs).floor() as i32;
        if gx < 0 || gx >= dim.x as i32 || gz < 0 || gz >= dim.z as i32 {
            return None;
        }

        // Highest wet cell in the column = the surface.
        for gy in (0..dim.y as i32).rev() {
            let level = self.get_level(IVec3::new(gx, gy, gz));
            if level > SURFACE_MIN_LEVEL {
                let surface_height = origin.y + (gy as f32 + level) * cs;
                return Some(WaterSample {
                    surface_height,
                    density: VOXEL_WATER_DENSITY,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_fluids::MaterialType;
    use glam::UVec3;

    fn basin() -> WaterVolumeGrid {
        // 6×6×6 grid, cell 1.0, origin 0; floor half-filled.
        let mut g = WaterVolumeGrid::new(UVec3::new(6, 6, 6), 1.0, Vec3::ZERO);
        for x in 0..6 {
            for z in 0..6 {
                for y in 0..3 {
                    g.set_level(IVec3::new(x, y, z), 1.0);
                }
            }
        }
        g
    }

    // ---- WI-1: sample mapping ----

    #[test]
    fn sample_returns_column_surface() {
        let g = basin();
        // Column 3,3 is full to y=2 (level 1.0) → surface top = 3.0.
        let s = g.sample(Vec3::new(3.5, 1.0, 3.5)).expect("water here");
        assert!((s.surface_height - 3.0).abs() < 1e-5, "surface {}", s.surface_height);
        assert_eq!(s.density, VOXEL_WATER_DENSITY);
    }

    #[test]
    fn sample_surface_independent_of_query_y() {
        let g = basin();
        // The caller does the depth check; sample returns the same surface for
        // a point above or below it.
        let below = g.sample(Vec3::new(3.5, 0.5, 3.5)).unwrap();
        let above = g.sample(Vec3::new(3.5, 50.0, 3.5)).unwrap();
        assert_eq!(below, above);
    }

    #[test]
    fn sample_outside_footprint_is_none() {
        let g = basin();
        assert_eq!(g.sample(Vec3::new(-1.0, 1.0, 3.0)), None, "x below grid");
        assert_eq!(g.sample(Vec3::new(100.0, 1.0, 3.0)), None, "x beyond grid");
        assert_eq!(g.sample(Vec3::new(3.0, 1.0, -1.0)), None, "z below grid");
    }

    #[test]
    fn sample_dry_column_is_none() {
        let mut g = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::ZERO);
        // No water anywhere.
        assert_eq!(g.sample(Vec3::new(1.5, 1.0, 1.5)), None);
        // A trace below the surface threshold still reads dry.
        g.set_level(IVec3::new(1, 0, 1), 0.005);
        assert_eq!(g.sample(Vec3::new(1.5, 0.0, 1.5)), None);
    }

    #[test]
    fn sample_partial_top_cell_surface() {
        let mut g = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 2.0, Vec3::new(10.0, 0.0, 10.0));
        // Cell (1,1,1): bottom at world y = 0 + 1*2 = 2; half full → top = 3.
        g.set_level(IVec3::new(1, 1, 1), 0.5);
        let s = g.sample(Vec3::new(13.0, 0.0, 13.0)).unwrap(); // world xz inside cell 1,1
        assert!((s.surface_height - 3.0).abs() < 1e-5, "surface {}", s.surface_height);
    }

    #[test]
    fn sample_honors_origin_offset() {
        let mut g = WaterVolumeGrid::new(UVec3::new(4, 4, 4), 1.0, Vec3::new(-100.0, 5.0, -100.0));
        g.set_level(IVec3::new(0, 0, 0), 1.0); // cell at world xz ∈ [-100,-99], bottom y=5
        let s = g.sample(Vec3::new(-99.5, 0.0, -99.5)).unwrap();
        assert!((s.surface_height - 6.0).abs() < 1e-5, "surface {}", s.surface_height);
        assert_eq!(g.sample(Vec3::new(0.0, 0.0, 0.0)), None, "origin-relative footprint");
    }

    // ---- WI-7: determinism (gate Q1) for the voxel backend ----

    /// Two independently-built grids run through the identical source/tick
    /// sequence yield bit-identical cell state AND bit-identical `sample`
    /// results — the gameplay-truth determinism guarantee for the voxel path.
    #[test]
    fn determinism_identical_grids_and_samples() {
        let build_and_run = || {
            let mut g = WaterVolumeGrid::new(UVec3::new(8, 8, 8), 1.0, Vec3::ZERO);
            // A source column + a wall — varied, flow-heavy setup.
            for y in 4..8 {
                g.set_level(IVec3::new(1, y, 1), 1.0);
            }
            for y in 0..4 {
                g.set_material(IVec3::new(4, y, 4), MaterialType::Stone);
            }
            for _ in 0..120 {
                g.simulate(1.0 / 60.0);
            }
            g
        };

        let a = build_and_run();
        let b = build_and_run();

        // Bit-identical cell levels across the whole grid.
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let p = IVec3::new(x, y, z);
                    assert_eq!(
                        a.get_level(p).to_bits(),
                        b.get_level(p).to_bits(),
                        "cell {p:?} diverged: {} vs {}",
                        a.get_level(p),
                        b.get_level(p)
                    );
                }
            }
        }

        // Bit-identical sample results through the trait.
        for &probe in &[
            Vec3::new(1.5, 2.0, 1.5),
            Vec3::new(3.5, 1.0, 3.5),
            Vec3::new(4.5, 0.5, 4.5),
            Vec3::new(7.0, 3.0, 7.0),
        ] {
            match (a.sample(probe), b.sample(probe)) {
                (Some(sa), Some(sb)) => {
                    assert_eq!(sa.surface_height.to_bits(), sb.surface_height.to_bits());
                    assert_eq!(sa.density.to_bits(), sb.density.to_bits());
                }
                (na, nb) => assert_eq!(na, nb, "sample presence diverged at {probe:?}"),
            }
        }
    }
}
