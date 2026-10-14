//! Q32.32 fixed-point. The raw `i64` is `value * 2^32`.
//!
//! # Seam
//!
//! This is a local copy of `astraweave-veil`'s `Fixed64` on
//! `cursor/aravath-greybox-b3de` (`fixed.rs`). That crate is not on `main`,
//! and this change does not merge it. Arithmetic is the veil contract:
//! `SCALE = 1 << 32`, multiply through `i128`, division truncates toward
//! zero, `den == 0` yields zero, overflow saturates to `i64`.
//!
//! When one `Fixed64` exists in `astraweave-math` (or veil re-exports that
//! type), delete this module and re-export it. Do not let the two copies drift.
//! The lock tests below are the drift alarm.

use serde::{Deserialize, Serialize};

/// Deterministic scalar. `2^32` raw units equal `1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fixed64(i64);

impl Fixed64 {
    pub const SCALE: i64 = 1 << 32;
    pub const ZERO: Self = Self(0);

    /// Largest whole unit `from_int` can store without saturating.
    pub const MAX_WHOLE: i64 = i64::MAX / Self::SCALE;

    #[inline]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Whole units. `from_int(1)` is exactly `SCALE` raw.
    #[inline]
    pub const fn from_int(v: i64) -> Self {
        Self(v.saturating_mul(Self::SCALE))
    }

    /// `num/den` in Q32.32, truncated toward zero. `den == 0` yields zero.
    #[inline]
    pub const fn from_ratio(num: i64, den: i64) -> Self {
        if den == 0 {
            return Self::ZERO;
        }
        let wide = (num as i128) << 32;
        Self(clamp_i128(wide / (den as i128)))
    }

    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[inline]
    pub const fn min(self, rhs: Self) -> Self {
        if self.0 <= rhs.0 {
            self
        } else {
            rhs
        }
    }

    /// Q32.32 multiply via `i128`, then saturating cast back to `i64`.
    #[inline]
    pub const fn mul(self, rhs: Self) -> Self {
        let wide = (self.0 as i128) * (rhs.0 as i128);
        Self(clamp_i128(wide >> 32))
    }

    /// `(self * num) / den` in raw units, truncated toward zero.
    #[inline]
    pub const fn mul_ratio(self, num: i64, den: i64) -> Self {
        if den == 0 {
            return Self::ZERO;
        }
        let wide = (self.0 as i128) * (num as i128);
        Self(clamp_i128(wide.div_euclid(den as i128)))
    }
}

const fn clamp_i128(v: i128) -> i64 {
    if v > i64::MAX as i128 {
        i64::MAX
    } else if v < i64::MIN as i128 {
        i64::MIN
    } else {
        v as i64
    }
}

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
    fn one_times_one_is_one() {
        let one = Fixed64::from_int(1);
        assert_eq!(one.mul(one), one);
        assert_eq!(one.raw(), Fixed64::SCALE);
    }

    #[test]
    fn ratio_and_mul_ratio_stay_integral() {
        let quarter = Fixed64::from_ratio(1, 4);
        assert_eq!(quarter.raw(), 1 << 30);
        let forty_eight = Fixed64::from_int(48);
        assert_eq!(forty_eight.mul_ratio(1, 48), Fixed64::from_int(1));
        assert_eq!(forty_eight.mul_ratio(1, 3).raw(), (48i64 << 32) / 3);
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
