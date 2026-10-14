//! What an agent can know about the scar field at an integer world position.
//!
//! The retreat step is chosen from a fixed horizontal neighbor order. Equal
//! tensions keep the earlier step. No float, no clock, no map iteration.

use crate::field::ScarField;
use crate::fixed::Fixed64;

/// Horizontal steps, tie-broken by this order: -X, -Z, +X, +Z.
const HORIZONTAL: [[i32; 3]; 4] = [[-1, 0, 0], [0, 0, -1], [1, 0, 0], [0, 0, 1]];

/// Fixed-point reading at one world cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScarPercept {
    pub cell: [i32; 3],
    pub tension: Fixed64,
    /// `tension >= wary` threshold.
    pub wary: bool,
    /// `tension >= avoid` threshold.
    pub avoid: bool,
    /// A horizontal step toward lower tension, or zero when every neighbor
    /// is at least as tense as this cell.
    pub retreat: [i32; 3],
}

impl ScarField {
    pub fn perceive_metres(&self, x: i32, y: i32, z: i32) -> ScarPercept {
        let cell = self.cell_of_metres([x, y, z]);
        let tension = self.effective_at_cell(cell);
        let mut retreat = [0, 0, 0];
        let mut best = tension;
        for step in HORIZONTAL {
            let neighbor = [cell[0] + step[0], cell[1] + step[1], cell[2] + step[2]];
            let sample = self.effective_at_cell(neighbor);
            if sample < best {
                best = sample;
                retreat = step;
            }
        }
        ScarPercept {
            cell,
            tension,
            wary: tension >= self.wary_at(),
            avoid: tension >= self.avoid_at(),
            retreat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed64;

    #[test]
    fn a_centered_scar_retreats_along_the_tie_break() {
        let mut field = ScarField::new();
        field.inject_death(0, 0, 0);
        let here = field.perceive_metres(0, 0, 0);
        assert!(here.avoid);
        assert_eq!(here.retreat, [-1, 0, 0]);
        let east = field.perceive_metres(8, 0, 0);
        assert!(!east.avoid);
        assert_ne!(east.retreat, [-1, 0, 0]);
    }

    #[test]
    fn far_from_a_scar_the_percept_is_clear() {
        let mut field = ScarField::new();
        field.inject_death(0, 4, 0);
        let far = field.perceive_metres(400, 4, 400);
        assert_eq!(far.tension, Fixed64::ZERO);
        assert!(!far.wary);
        assert!(!far.avoid);
        assert_eq!(far.retreat, [0, 0, 0]);
    }

    #[test]
    fn this_module_has_no_float_or_hashmap() {
        let src = include_str!("percept.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(!code.contains("f32"));
        assert!(!code.contains("f64"));
        assert!(!code.contains("HashMap"));
        assert!(!code.contains("SystemTime"));
        assert!(!code.contains("Instant"));
    }
}
