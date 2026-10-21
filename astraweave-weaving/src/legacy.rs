//! Compiles the six legacy ops into templates and director rectangles.

use crate::field::{meters_to_cell, milli_corner, FieldWrite, WriteOp};
use crate::mod_queue::ISLAND_HALF_CELLS;
use crate::templates::{
    builtin, writes_for, MAX_NODES, TEMPLATE_DENSITY, TEMPLATE_FREEZE, TEMPLATE_LOWER_WATER,
    TEMPLATE_WIND,
};
use astraweave_nav::{NavChange, NavChangeKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectorRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectorCollapse {
    pub ax: i32,
    pub ay: i32,
    pub bx: i32,
    pub by: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct CompiledOp {
    pub writes: [FieldWrite; MAX_NODES],
    pub write_count: u8,
    pub fortify: Option<DirectorRect>,
    pub collapse: Option<DirectorCollapse>,
    pub ice_min_milli: Option<[i32; 3]>,
    pub ice_max_milli: Option<[i32; 3]>,
    pub nav: Option<NavChange>,
}

/// Points for one legacy op. `kind` is 0 reinforce, 1 collapse, 2 wind, 3 lower, 4 raise, 5 freeze.
#[derive(Clone, Copy, Debug)]
pub struct LegacyPoints {
    pub a: [f32; 3],
    pub b: Option<[f32; 3]>,
    pub tick: u64,
    pub seq: u32,
}

pub fn compile_legacy(kind: u8, points: LegacyPoints) -> CompiledOp {
    let [ax, ay, az] = points.a;
    let (bx, by, bz) = match points.b {
        Some([x, y, z]) => (Some(x), Some(y), Some(z)),
        None => (None, None, None),
    };
    let tick = points.tick;
    let seq = points.seq;
    let Some(origin) = cell_origin(ax, ay, az) else {
        return empty_compiled();
    };
    let mut compiled = CompiledOp {
        writes: writes_for(&builtin(TEMPLATE_DENSITY).unwrap(), origin, 1).0,
        write_count: 0,
        fortify: None,
        collapse: None,
        ice_min_milli: None,
        ice_max_milli: None,
        nav: None,
    };
    match kind {
        0 | 4 => {
            let span = if kind == 0 { 1 } else { 0 };
            let x = ax as i32;
            let z = az as i32;
            compiled.fortify = Some(DirectorRect {
                x0: x - span,
                y0: z - span,
                x1: x + span,
                y1: z + span,
            });
            let (mods, count) = writes_for(&builtin(TEMPLATE_DENSITY).unwrap(), origin, 1);
            compiled.writes = mods;
            compiled.write_count = count;
        }
        1 => {
            if let (Some(bx), Some(by), Some(bz)) = (bx, by, bz) {
                if cell_origin(bx, by, bz).is_none() {
                    return empty_compiled();
                }
                compiled.collapse = Some(DirectorCollapse {
                    ax: ax as i32,
                    ay: az as i32,
                    bx: bx as i32,
                    by: bz as i32,
                });
            }
        }
        2 => {
            if let (Some(bx), Some(by), Some(bz)) = (bx, by, bz) {
                if !wind_delta_fits(ax, ay, az, bx, by, bz) {
                    return empty_compiled();
                }
            }
            let (mods, count) = writes_for(&builtin(TEMPLATE_WIND).unwrap(), origin, 1);
            compiled.writes = mods;
            compiled.write_count = count;
            if let (Some(bx), Some(bz)) = (bx, bz) {
                let dx = ((bx - ax) as i8).clamp(-1, 1);
                let dz = ((bz - az) as i8).clamp(-1, 1);
                if compiled.write_count > 0 {
                    compiled.writes[0].op = WriteOp::SetWind { x: dx, y: 0, z: dz };
                }
            }
        }
        3 => {
            let (mods, count) = writes_for(&builtin(TEMPLATE_LOWER_WATER).unwrap(), origin, 1);
            compiled.writes = mods;
            compiled.write_count = count;
        }
        5 => {
            let template = builtin(TEMPLATE_FREEZE).unwrap();
            let (mods, count) = writes_for(&template, origin, 1);
            compiled.writes = mods;
            compiled.write_count = count;
            let radius = template.ice_radius as i32;
            if let (Some(min_milli), Some(max_milli)) = (
                milli_corner(
                    origin[0].checked_sub(radius),
                    Some(origin[1]),
                    origin[2].checked_sub(radius),
                ),
                milli_corner(
                    origin[0]
                        .checked_add(radius)
                        .and_then(|cell| cell.checked_add(1)),
                    origin[1].checked_add(1),
                    origin[2]
                        .checked_add(radius)
                        .and_then(|cell| cell.checked_add(1)),
                ),
            ) {
                compiled.ice_min_milli = Some(min_milli);
                compiled.ice_max_milli = Some(max_milli);
                compiled.nav = Some(NavChange::new(
                    tick,
                    seq,
                    min_milli,
                    max_milli,
                    NavChangeKind::Weave,
                ));
            }
        }
        _ => {}
    }
    compiled
}

fn wind_delta_fits(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    let dz = bz - az;
    if !(dx.is_finite() && dy.is_finite() && dz.is_finite()) {
        return false;
    }
    (dx * dx + dy * dy + dz * dz).is_finite()
}

fn cell_origin(x: f32, y: f32, z: f32) -> Option<[i32; 3]> {
    let origin = [meters_to_cell(x)?, meters_to_cell(y)?, meters_to_cell(z)?];
    if origin
        .iter()
        .all(|cell| (-ISLAND_HALF_CELLS..=ISLAND_HALF_CELLS).contains(cell))
    {
        Some(origin)
    } else {
        None
    }
}

fn empty_compiled() -> CompiledOp {
    let (writes, _) = writes_for(&builtin(TEMPLATE_DENSITY).unwrap(), [0, 0, 0], 1);
    CompiledOp {
        writes,
        write_count: 0,
        fortify: None,
        collapse: None,
        ice_min_milli: None,
        ice_max_milli: None,
        nav: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{WriteOp, PHASE_ICE};

    #[test]
    fn freeze_compiles_ice_sheet_and_nav() {
        let compiled = compile_legacy(
            5,
            LegacyPoints {
                a: [1.0, 0.0, 1.0],
                b: None,
                tick: 4,
                seq: 2,
            },
        );
        assert!(compiled.write_count >= 1);
        assert!(matches!(
            compiled.writes[0].op,
            WriteOp::SetPhase { phase: PHASE_ICE }
        ));
        let nav = compiled.nav.unwrap();
        assert_eq!(nav.tick, 4);
        assert_eq!(nav.seq, 2);
        assert_eq!(compiled.ice_min_milli.unwrap(), nav.min_milli);
    }

    #[test]
    fn reinforce_still_has_a_rect() {
        let compiled = compile_legacy(
            0,
            LegacyPoints {
                a: [5.0, 0.0, 5.0],
                b: None,
                tick: 0,
                seq: 0,
            },
        );
        let rect = compiled.fortify.unwrap();
        assert_eq!((rect.x0, rect.y0, rect.x1, rect.y1), (4, 4, 6, 6));
    }

    #[test]
    fn far_origin_does_not_saturate_into_a_milli_aabb() {
        let compiled = compile_legacy(
            5,
            LegacyPoints {
                a: [1.0e20, 0.0, 0.0],
                b: None,
                tick: 4,
                seq: 2,
            },
        );
        assert_eq!(compiled.write_count, 0);
        assert!(compiled.ice_min_milli.is_none());
        assert!(compiled.nav.is_none());
        let edge = compile_legacy(
            5,
            LegacyPoints {
                a: [ISLAND_HALF_CELLS as f32, 0.0, 0.0],
                b: None,
                tick: 1,
                seq: 1,
            },
        );
        let min = edge.ice_min_milli.unwrap();
        assert_eq!(min[0], (ISLAND_HALF_CELLS - 2) * crate::field::CELL_MM);
        assert_eq!(min[1], 0);
    }

    #[test]
    fn non_finite_and_i32_edge_origins_drop_the_op() {
        for origin in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            (i32::MAX as f32) / 1000.0,
            (i32::MIN as f32) / 1000.0,
        ] {
            let compiled = compile_legacy(
                5,
                LegacyPoints {
                    a: [origin, 0.0, 0.0],
                    b: None,
                    tick: 1,
                    seq: 1,
                },
            );
            assert_eq!(compiled.write_count, 0, "{origin}");
            assert!(compiled.ice_min_milli.is_none(), "{origin}");
        }
    }

    #[test]
    fn far_collapse_endpoint_is_rejected() {
        let compiled = compile_legacy(
            1,
            LegacyPoints {
                a: [0.0, 0.0, 0.0],
                b: Some([1.0e20, 0.0, 0.0]),
                tick: 0,
                seq: 0,
            },
        );
        assert!(compiled.collapse.is_none());
        assert_eq!(compiled.write_count, 0);
        let kept = compile_legacy(
            1,
            LegacyPoints {
                a: [0.0, 0.0, 0.0],
                b: Some([10.0, 0.0, 10.0]),
                tick: 0,
                seq: 0,
            },
        );
        let line = kept.collapse.unwrap();
        assert_eq!((line.ax, line.ay, line.bx, line.by), (0, 0, 10, 10));
    }

    #[test]
    fn non_finite_wind_endpoint_drops_the_op() {
        for endpoint in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let compiled = compile_legacy(
                2,
                LegacyPoints {
                    a: [0.0, 0.0, 0.0],
                    b: Some([endpoint, 0.0, 1.0]),
                    tick: 0,
                    seq: 0,
                },
            );
            assert_eq!(compiled.write_count, 0, "{endpoint}");
        }
        let kept = compile_legacy(
            2,
            LegacyPoints {
                a: [0.0, 0.0, 0.0],
                b: Some([1.0, 0.0, 0.0]),
                tick: 0,
                seq: 0,
            },
        );
        assert!(kept.write_count > 0);
        let huge = compile_legacy(
            2,
            LegacyPoints {
                a: [0.0, 0.0, 0.0],
                b: Some([1.9e19, 0.0, 0.0]),
                tick: 0,
                seq: 0,
            },
        );
        assert_eq!(huge.write_count, 0);
    }
}
