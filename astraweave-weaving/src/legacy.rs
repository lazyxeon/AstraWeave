//! Compiles the six legacy ops into templates and director rectangles.

use crate::field::{meters_to_cell, SimModifier};
use crate::templates::{
    builtin, modifiers_for, MAX_NODES, TEMPLATE_DENSITY, TEMPLATE_FREEZE, TEMPLATE_LOWER_WATER,
    TEMPLATE_WIND,
};
use astraweave_nav::NavChange;

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
    pub modifiers: [SimModifier; MAX_NODES],
    pub modifier_count: u8,
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
    let origin = [meters_to_cell(ax), meters_to_cell(ay), meters_to_cell(az)];
    let mut compiled = CompiledOp {
        modifiers: modifiers_for(&builtin(TEMPLATE_DENSITY).unwrap(), origin, 1).0,
        modifier_count: 0,
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
            let (mods, count) = modifiers_for(&builtin(TEMPLATE_DENSITY).unwrap(), origin, 1);
            compiled.modifiers = mods;
            compiled.modifier_count = count;
        }
        1 => {
            if let (Some(bx), Some(bz)) = (bx, bz) {
                compiled.collapse = Some(DirectorCollapse {
                    ax: ax as i32,
                    ay: az as i32,
                    bx: bx as i32,
                    by: bz as i32,
                });
            }
            let _ = by;
        }
        2 => {
            let (mods, count) = modifiers_for(&builtin(TEMPLATE_WIND).unwrap(), origin, 1);
            compiled.modifiers = mods;
            compiled.modifier_count = count;
            if let (Some(bx), Some(bz)) = (bx, bz) {
                let dx = ((bx - ax) * 1000.0) as i16;
                let dz = ((bz - az) * 1000.0) as i16;
                if compiled.modifier_count > 0 {
                    compiled.modifiers[0].a = dx;
                    compiled.modifiers[0].b = dz;
                }
            }
        }
        3 => {
            let (mods, count) = modifiers_for(&builtin(TEMPLATE_LOWER_WATER).unwrap(), origin, 1);
            compiled.modifiers = mods;
            compiled.modifier_count = count;
        }
        5 => {
            let template = builtin(TEMPLATE_FREEZE).unwrap();
            let (mods, count) = modifiers_for(&template, origin, 1);
            compiled.modifiers = mods;
            compiled.modifier_count = count;
            let radius = template.ice_radius as i32;
            let min_milli = [
                (origin[0] - radius) * crate::field::CELL_MM,
                origin[1] * crate::field::CELL_MM,
                (origin[2] - radius) * crate::field::CELL_MM,
            ];
            let max_milli = [
                (origin[0] + radius + 1) * crate::field::CELL_MM,
                (origin[1] + 1) * crate::field::CELL_MM,
                (origin[2] + radius + 1) * crate::field::CELL_MM,
            ];
            compiled.ice_min_milli = Some(min_milli);
            compiled.ice_max_milli = Some(max_milli);
            compiled.nav = Some(NavChange {
                tick,
                seq,
                min_milli,
                max_milli,
            });
        }
        _ => {}
    }
    compiled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{ModifierKind, MAT_ICE};

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
        assert!(compiled.modifier_count >= 1);
        assert_eq!(compiled.modifiers[0].kind, ModifierKind::MaterialStamp);
        assert_eq!(compiled.modifiers[0].a, MAT_ICE as i16);
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
}
