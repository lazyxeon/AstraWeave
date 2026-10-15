//! Range check on the canonical Q32.32 type.
//!
//! [`Fixed64`] is [`astraweave_math::Fixed64`] from PR #245. This module does
//! not copy the arithmetic. [`whole_raw`] stays here because it returns
//! [`CombatError`](super::CombatError).

pub use astraweave_math::Fixed64;

/// Whole units as raw Q32.32 in `i128`, so a later multiply can widen first.
#[inline]
pub fn whole_raw(v: i64) -> Result<i128, super::CombatError> {
    if v < 0 {
        return Err(super::CombatError::StatOutOfRange);
    }
    if v > Fixed64::MAX_WHOLE {
        return Err(super::CombatError::StatOutOfRange);
    }
    Ok(i128::from(v) * i128::from(Fixed64::SCALE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexport_keeps_one_times_one() {
        let one = Fixed64::from_int(1);
        assert_eq!(one.mul(one), one);
        assert_eq!(one.raw(), Fixed64::SCALE);
    }

    #[test]
    fn reexport_keeps_the_ratio_identity() {
        let quarter = Fixed64::from_ratio(1, 4);
        assert_eq!(quarter.raw(), 1 << 30);
        let forty_eight = Fixed64::from_int(48);
        assert_eq!(forty_eight.mul_ratio(1, 48), Fixed64::from_int(1));
    }

    #[test]
    fn this_module_has_no_float_conversion() {
        let src = include_str!("fixed.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(!code.contains("f32"), "{code}");
        assert!(!code.contains("f64"));
        assert!(!code.contains("HashMap"));
        assert!(!code.contains("SystemTime"));
        assert!(!code.contains("Instant"));
    }
}
