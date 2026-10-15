//! Q32.32 fixed-point. The raw `i64` is `value * 2^32`.
//!
//! This is the single `Fixed64` implementation. Arithmetic matches the type
//! that lived on PR #212: multiply through `i128`, `from_ratio` truncates
//! toward zero, `mul_ratio` uses `div_euclid`, `den == 0` yields zero, and
//! overflow saturates to `i64`.
//!
//! A range-checked whole-unit helper that returns a combat error stays with
//! combat. This module exposes [`Fixed64::MAX_WHOLE`] and saturating
//! [`Fixed64::from_int`] instead.

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

    /// `(self * num) / den` in raw units, truncated toward negative infinity.
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

/// Canonical little-endian raw values for the cross-OS replay case.
///
/// The sequence is integer arithmetic only. Two machines that disagree on
/// one saturating multiply or one division rounding produce a different byte
/// string.
pub fn fixed64_contract_bytes() -> Vec<u8> {
    let mut out = Vec::with_capacity(32 * 8);
    let mut push = |value: Fixed64| {
        out.extend_from_slice(&value.raw().to_le_bytes());
    };

    push(Fixed64::ZERO);
    push(Fixed64::from_int(1));
    push(Fixed64::from_int(-1));
    push(Fixed64::from_int(48));
    push(Fixed64::from_int(Fixed64::MAX_WHOLE));
    push(Fixed64::from_int(Fixed64::MAX_WHOLE.saturating_add(1)));
    push(Fixed64::from_ratio(1, 4));
    push(Fixed64::from_ratio(-1, 4));
    push(Fixed64::from_ratio(1, -4));
    push(Fixed64::from_ratio(1, 0));
    push(Fixed64::from_int(1).mul(Fixed64::from_int(1)));
    push(Fixed64::from_int(48).mul_ratio(1, 48));
    push(Fixed64::from_int(48).mul_ratio(1, 3));
    push(Fixed64::from_int(-48).mul_ratio(1, 3));
    push(Fixed64::from_raw(i64::MAX).saturating_add(Fixed64::from_int(1)));
    push(Fixed64::from_raw(i64::MIN).saturating_sub(Fixed64::from_int(1)));
    push(Fixed64::from_int(2).min(Fixed64::from_int(-5)));

    let mut acc = Fixed64::from_int(1);
    let three_halves = Fixed64::from_ratio(3, 2);
    for _ in 0..32 {
        acc = acc.mul(three_halves);
        push(acc);
    }
    out
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
    fn negative_ratio_truncates_toward_zero() {
        assert_eq!(Fixed64::from_ratio(-1, 4).raw(), -((1i64) << 30));
        assert_eq!(Fixed64::from_ratio(1, -4).raw(), -((1i64) << 30));
        assert_eq!(Fixed64::from_ratio(1, 0), Fixed64::ZERO);
    }

    #[test]
    fn mul_ratio_on_a_negative_uses_div_euclid() {
        let got = Fixed64::from_int(-48).mul_ratio(1, 3).raw();
        let expect = ((-48i128) << 32).div_euclid(3) as i64;
        assert_eq!(got, expect);
        assert_eq!(Fixed64::from_int(-48).mul_ratio(1, 0), Fixed64::ZERO);
    }

    #[test]
    fn saturating_edges_stay_inside_i64() {
        assert_eq!(
            Fixed64::from_raw(i64::MAX)
                .saturating_add(Fixed64::from_int(1))
                .raw(),
            i64::MAX
        );
        assert_eq!(
            Fixed64::from_raw(i64::MIN)
                .saturating_sub(Fixed64::from_int(1))
                .raw(),
            i64::MIN
        );
        let huge = Fixed64::from_raw(i64::MAX).mul(Fixed64::from_int(2));
        assert_eq!(huge.raw(), i64::MAX);
    }

    #[test]
    fn contract_bytes_are_stable_in_process() {
        assert_eq!(fixed64_contract_bytes(), fixed64_contract_bytes());
        assert_eq!(fixed64_contract_bytes().len() % 8, 0);
    }

    #[test]
    fn this_module_has_no_float_conversion() {
        let src = include_str!("fixed64.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(!code.contains("f32"), "{code}");
        assert!(!code.contains("f64"));
        assert!(!code.contains("HashMap"));
        assert!(!code.contains("SystemTime"));
        assert!(!code.contains("Instant"));
    }
}
