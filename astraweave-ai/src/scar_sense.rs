//! Perception of a fate-scar field.
//!
//! The reading is [`astraweave_veil::ScarPercept`]: fixed-point tension and an
//! integer retreat step. This module does not sample floats, clocks, or
//! unordered maps. The demo turns [`stance_at`] into a steering wish.

use astraweave_veil::{Fixed64, ScarField, ScarPercept};

/// What a simple agent does with a scar percept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScarStance {
    Clear,
    Wary { tension: Fixed64 },
    Avoid { tension: Fixed64, retreat: [i32; 3] },
}

/// The field's percept at whole metres. This is the perception query.
pub fn sense(field: &ScarField, metres: [i32; 3]) -> ScarPercept {
    field.perceive_metres(metres[0], metres[1], metres[2])
}

/// Clear, wary, or avoid. Avoid carries the percept's retreat step.
pub fn stance_at(field: &ScarField, metres: [i32; 3]) -> ScarStance {
    let percept = sense(field, metres);
    if percept.avoid {
        ScarStance::Avoid {
            tension: percept.tension,
            retreat: percept.retreat,
        }
    } else if percept.wary {
        ScarStance::Wary {
            tension: percept.tension,
        }
    } else {
        ScarStance::Clear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_veil::Fixed64;
    use astraweave_veil::{ScarField, AVOID_TENSION, WARY_TENSION};

    fn fold(hash: u64, bytes: &[u8]) -> u64 {
        let mut hash = hash;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn stance_hash(stance: ScarStance) -> u64 {
        match stance {
            ScarStance::Clear => fold(0xcbf29ce484222325, &[0]),
            ScarStance::Wary { tension } => {
                let hash = fold(0xcbf29ce484222325, &[1]);
                fold(hash, &tension.raw().to_le_bytes())
            }
            ScarStance::Avoid { tension, retreat } => {
                let mut hash = fold(0xcbf29ce484222325, &[2]);
                hash = fold(hash, &tension.raw().to_le_bytes());
                hash = fold(hash, &retreat[0].to_le_bytes());
                hash = fold(hash, &retreat[1].to_le_bytes());
                fold(hash, &retreat[2].to_le_bytes())
            }
        }
    }

    #[test]
    fn cross_run_stance_matches_the_golden_hash() {
        let mut field = ScarField::new();
        field.inject_death(0, 4, 0);
        let on_scar = stance_at(&field, [0, 4, 0]);
        let beside = stance_at(&field, [4, 4, 0]);
        let far = stance_at(&field, [80, 4, 80]);
        assert!(matches!(on_scar, ScarStance::Avoid { .. }));
        assert!(matches!(
            beside,
            ScarStance::Wary { .. } | ScarStance::Avoid { .. }
        ));
        assert_eq!(far, ScarStance::Clear);
        let mut hash = stance_hash(on_scar);
        hash = fold(hash, &stance_hash(beside).to_le_bytes());
        hash = fold(hash, &stance_hash(far).to_le_bytes());
        let again = {
            let mut field = ScarField::new();
            field.inject_death(0, 4, 0);
            let mut hash = stance_hash(stance_at(&field, [0, 4, 0]));
            hash = fold(
                hash,
                &stance_hash(stance_at(&field, [4, 4, 0])).to_le_bytes(),
            );
            fold(
                hash,
                &stance_hash(stance_at(&field, [80, 4, 80])).to_le_bytes(),
            )
        };
        assert_eq!(hash, again);
        // Locked from a previous run of this function.
        const GOLDEN: u64 = 0x0eb14da1efc9e7b2;
        assert_eq!(
            hash, GOLDEN,
            "cross-run scar stance hash drifted: {hash:#018x}"
        );
        assert!(AVOID_TENSION > WARY_TENSION);
    }

    #[test]
    fn sense_returns_the_same_tension_the_field_stores() {
        let mut field = ScarField::new();
        field.inject_energy(8, 0, -4, Fixed64::from_int(16));
        let percept = sense(&field, [8, 0, -4]);
        assert!(percept.wary);
        assert!(!percept.avoid);
        assert_eq!(
            stance_at(&field, [8, 0, -4]),
            ScarStance::Wary {
                tension: percept.tension,
            }
        );
    }
}
