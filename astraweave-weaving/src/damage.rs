//! Element schools as data. Nothing here names a spell.
//!
//! A weave supplies school ids, ratings, and resists. Combat applies those
//! rows. The same rows can carry a density step and a whole-unit decay for
//! the destruction log.

use astraweave_combat::{
    CDefense, COffense, CombatError, SchoolRating, SchoolResist, MAX_MODIFIERS,
};

/// One composed school. The id is content data, not a code branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementSchool {
    pub school_id: u32,
    pub rating: i64,
    pub resist: i64,
    pub density_delta: i8,
    pub decay_whole: i64,
    pub origin: [i32; 3],
    pub radius: u16,
}

/// Merge `schools` into the offense ratings and defense resists.
///
/// Duplicate school ids add. The walk is sorted by id so the stored order
/// does not depend on the input order.
pub fn apply_element_schools(
    offense: &mut COffense,
    defense: &mut CDefense,
    schools: &[ElementSchool],
) -> Result<(), CombatError> {
    if schools.len() > MAX_MODIFIERS {
        return Err(CombatError::TooManyModifiers);
    }
    let mut order = [0usize; MAX_MODIFIERS];
    for (index, slot) in order.iter_mut().enumerate().take(schools.len()) {
        *slot = index;
    }
    order[..schools.len()].sort_by_key(|index| schools[*index].school_id);

    let mut cursor = 0;
    while cursor < schools.len() {
        let school_id = schools[order[cursor]].school_id;
        let mut rating = 0i64;
        let mut resist = 0i64;
        while cursor < schools.len() && schools[order[cursor]].school_id == school_id {
            let row = schools[order[cursor]];
            rating = rating.saturating_add(row.rating);
            resist = resist.saturating_add(row.resist);
            cursor += 1;
        }
        if rating != 0 {
            push_rating(offense, school_id, rating)?;
        }
        if resist != 0 {
            push_resist(defense, school_id, resist)?;
        }
    }
    Ok(())
}

fn push_rating(offense: &mut COffense, school: u32, amount: i64) -> Result<(), CombatError> {
    let len = offense.rating_len as usize;
    for row in offense.ratings.iter_mut().take(len) {
        if row.school == school {
            row.amount = row.amount.saturating_add(amount);
            return Ok(());
        }
    }
    if len >= MAX_MODIFIERS {
        return Err(CombatError::TooManyModifiers);
    }
    offense.ratings[len] = SchoolRating { school, amount };
    offense.rating_len += 1;
    Ok(())
}

fn push_resist(defense: &mut CDefense, school: u32, amount: i64) -> Result<(), CombatError> {
    let len = defense.resist_len as usize;
    for row in defense.resists.iter_mut().take(len) {
        if row.school == school {
            row.amount = row.amount.saturating_add(amount);
            return Ok(());
        }
    }
    if len >= MAX_MODIFIERS {
        return Err(CombatError::TooManyModifiers);
    }
    defense.resists[len] = SchoolResist { school, amount };
    defense.resist_len += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schools_merge_by_id_regardless_of_input_order() {
        let mut offense = COffense::default();
        let mut defense = CDefense::default();
        let schools = [
            ElementSchool {
                school_id: 3,
                rating: 4,
                resist: 1,
                density_delta: 0,
                decay_whole: 0,
                origin: [0, 0, 0],
                radius: 1,
            },
            ElementSchool {
                school_id: 2,
                rating: 10,
                resist: 0,
                density_delta: -1,
                decay_whole: 5,
                origin: [1, 2, 3],
                radius: 2,
            },
            ElementSchool {
                school_id: 3,
                rating: 6,
                resist: 2,
                density_delta: 0,
                decay_whole: 0,
                origin: [0, 0, 0],
                radius: 1,
            },
        ];
        apply_element_schools(&mut offense, &mut defense, &schools).unwrap();
        assert_eq!(offense.rating_len, 2);
        assert_eq!(offense.ratings[0].school, 2);
        assert_eq!(offense.ratings[0].amount, 10);
        assert_eq!(offense.ratings[1].school, 3);
        assert_eq!(offense.ratings[1].amount, 10);
        assert_eq!(defense.resist_len, 1);
        assert_eq!(defense.resists[0].school, 3);
        assert_eq!(defense.resists[0].amount, 3);
    }
}
