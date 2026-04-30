//! Phase 1.X-F.1.A: spline + climate-fold helper types.
//!
//! This module ships the foundational types for the Regional Archetype
//! Variation campaign's climate-driven shape splines (Approach F per the
//! research audit `docs/audits/regional_archetype_variation_research_2026-04-29.md`
//! §6.6, Minecraft 1.18+ canonical pattern):
//!
//! - [`PvFold`] — implements the canonical Minecraft Peaks-and-Valleys
//!   formula `pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`. Folded
//!   weirdness produces 5 categorical terrain levels (Valleys / Low /
//!   Mid / High / Peaks) and is the third climate-input axis the
//!   per-archetype splines read.
//! - [`Spline1D`] — placeholder type definition for F.2.A. Each spline
//!   is a sorted `(input, output)` series with linear interpolation
//!   between adjacent points; F.2.A populates the implementation.
//! - [`BootstrapParam`] — placeholder enum for F.2.A. Identifies the
//!   four bootstrap noise parameters that become spline outputs:
//!   `MountainsAmplitude`, `MountainsScale`, `ContinentalScale`,
//!   `BaseElevationAmplitude`.
//!
//! Per F.1.A scope: `Spline1D` and `BootstrapParam` ship with `#[allow(dead_code)]`
//! until F.2.A consumes them. `PvFold` is fully usable at F.1.A close;
//! F.1.B's `ClimateSample::pv()` accessor calls into it.
//!
//! References:
//! - `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` §2.2 (climate
//!   field extension), §2.3 (BootstrapSplineSet design).
//! - Minecraft Wiki — Noise router, MultiNoiseUtil.NoiseHypercube
//!   (PV formula source; cited in research audit §5.1).

/// Phase 1.X-F.1.A: Peaks-and-Valleys fold helper.
///
/// Implements the canonical Minecraft 1.18+ formula
/// `pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`. Folded weirdness
/// produces a characteristic 5-band character (Valleys / Low / Mid /
/// High / Peaks) over a `[-1, 1]` weirdness input.
///
/// The PV value is the third climate-input axis the per-archetype
/// `BootstrapSplineSet` reads (alongside `continentalness` and
/// `erosion`); high PV drives mountain peaks, low PV drives valleys.
///
/// # Range
///
/// For `|weirdness| ∈ [0, 1]`, the formula produces `pv ∈ [-1, 1]`.
/// Specifically:
/// - `weirdness = 0` → `pv = -1` (valley extremum)
/// - `weirdness = ±2/3` → `pv = +1` (peak extremum)
/// - `weirdness = ±1/3` or `±1` → `pv = 0` (mid)
///
/// # Determinism
///
/// Pure function; same input always produces same output. No state.
pub struct PvFold;

impl PvFold {
    /// Compute PV from weirdness via the canonical Minecraft 1.18+
    /// formula: `pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`.
    ///
    /// Even-symmetric in weirdness (depends only on `|weirdness|`).
    ///
    /// # Examples
    ///
    /// ```
    /// use astraweave_terrain::spline_types::PvFold;
    ///
    /// // Valley extremum at weirdness=0
    /// let pv = PvFold::from_weirdness(0.0);
    /// assert!((pv - (-1.0)).abs() < 1e-6);
    ///
    /// // Peak extremum at weirdness=2/3
    /// let pv = PvFold::from_weirdness(2.0 / 3.0);
    /// assert!((pv - 1.0).abs() < 1e-6);
    /// ```
    #[inline]
    pub fn from_weirdness(weirdness: f32) -> f32 {
        1.0 - ((3.0 * weirdness.abs()) - 2.0).abs()
    }
}

/// Phase 1.X-F.1.A: placeholder for F.2.A's piecewise-linear 1D spline.
///
/// F.2.A populates with the full implementation: `evaluate(input: f32) -> f32`
/// (binary-search + linear-interpolate between adjacent control points;
/// clamp at endpoints), `identity()` constructor (always returns 1.0),
/// `from_control_points(Vec<(f32, f32)>) -> Result<Self>` (validates
/// sortedness on input). F.1.A ships only the type definition.
///
/// Control points are `(input, output_multiplier)` pairs sorted by input.
/// Output multipliers are dimensionless (typically `[0.0, 4.0]`); the
/// owning `ParamSpline` (F.2.A) multiplies these against a base value.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Spline1D {
    /// Sorted `(input, output_multiplier)` control points. Empty
    /// `control_points` is valid; F.2.A's `evaluate` returns 1.0 in
    /// that case (identity behavior).
    pub control_points: Vec<(f32, f32)>,
}

/// Phase 1.X-F.1.A: placeholder enum identifying the bootstrap noise
/// parameters that become spline outputs in F.2.A's `BootstrapSplineSet`.
///
/// Per campaign doc §2.3:
/// - [`Self::MountainsAmplitude`] — `NoiseConfig.mountains.amplitude`
///   (default 480; controls mountain layer height).
/// - [`Self::MountainsScale`] — `NoiseConfig.mountains.scale` (default
///   0.002; controls mountain layer feature size).
/// - [`Self::ContinentalScale`] — `NoiseConfig.continental_scale`
///   (default 0.0003; controls continental modulation wavelength).
/// - [`Self::BaseElevationAmplitude`] — `NoiseConfig.base_elevation.amplitude`
///   (default 150; controls base elevation layer height).
///
/// F.2.A consumes this enum; F.1.A ships type only.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BootstrapParam {
    MountainsAmplitude,
    MountainsScale,
    ContinentalScale,
    BaseElevationAmplitude,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 1.X-F.1.A: hand-verified canonical Minecraft 1.18+ PV
    /// values per F.1 prompt §1.2. Each tuple is
    /// `(weirdness, expected_pv, character_label)`.
    ///
    /// Hand-derivation (also in F.1 prompt §1.2 table):
    /// - w=-1.0 → |w|=1.0 → ×3=3.0 → -2=1.0 → abs=1.0 → 1-1=0.0
    /// - w=-2/3 → |w|=2/3 → ×3=2.0 → -2=0.0 → abs=0.0 → 1-0=1.0
    /// - w=-1/3 → |w|=1/3 → ×3=1.0 → -2=-1.0 → abs=1.0 → 1-1=0.0
    /// - w= 0.0 → |w|=0.0 → ×3=0.0 → -2=-2.0 → abs=2.0 → 1-2=-1.0
    /// - w=+1/3 → same as -1/3 by symmetry → 0.0
    /// - w=+2/3 → same as -2/3 → 1.0
    /// - w=+1.0 → same as -1.0 → 0.0
    #[test]
    fn pv_fold_seven_canonical_points() {
        let cases: &[(f32, f32, &str)] = &[
            (-1.0, 0.0, "Mid (low ridge, neg extreme)"),
            (-2.0 / 3.0, 1.0, "Peak (neg)"),
            (-1.0 / 3.0, 0.0, "Mid (neg)"),
            (0.0, -1.0, "Valley (zero)"),
            (1.0 / 3.0, 0.0, "Mid (pos)"),
            (2.0 / 3.0, 1.0, "Peak (pos)"),
            (1.0, 0.0, "Mid (low ridge, pos extreme)"),
        ];

        for &(weirdness, expected_pv, label) in cases {
            let actual = PvFold::from_weirdness(weirdness);
            assert!(
                (actual - expected_pv).abs() < 1e-6,
                "{}: PvFold::from_weirdness({}) = {} (expected {} ± 1e-6)",
                label,
                weirdness,
                actual,
                expected_pv
            );
        }
    }

    /// PvFold is even-symmetric in weirdness (depends only on |weirdness|).
    /// Verifies symmetry property at 5 sample magnitudes across `[0, 1]`.
    #[test]
    fn pv_fold_symmetry() {
        for &mag in &[0.1f32, 0.25, 0.4, 0.55, 0.85] {
            let pv_pos = PvFold::from_weirdness(mag);
            let pv_neg = PvFold::from_weirdness(-mag);
            assert!(
                (pv_pos - pv_neg).abs() < 1e-7,
                "symmetry violated at mag={}: pv(+)={}, pv(-)={}",
                mag,
                pv_pos,
                pv_neg
            );
        }
    }

    /// PV output stays within `[-1, 1]` for every weirdness in `[-1, 1]`.
    /// Sweeps 100 values to catch any out-of-range artifact.
    #[test]
    fn pv_fold_range_bounded() {
        for i in 0..=100 {
            let weirdness = -1.0 + (i as f32) * 0.02; // [-1, 1] in 0.02 steps
            let pv = PvFold::from_weirdness(weirdness);
            assert!(
                (-1.0..=1.0).contains(&pv),
                "PV out of range at weirdness={}: pv={}",
                weirdness,
                pv
            );
        }
    }

    #[test]
    fn pv_fold_at_zero_is_minus_one() {
        let pv = PvFold::from_weirdness(0.0);
        assert!((pv - (-1.0)).abs() < 1e-7, "pv(0) = {} (expected -1)", pv);
    }

    #[test]
    fn pv_fold_at_two_thirds_is_one() {
        let pv = PvFold::from_weirdness(2.0 / 3.0);
        assert!((pv - 1.0).abs() < 1e-6, "pv(2/3) = {} (expected 1)", pv);
    }

    #[test]
    fn pv_fold_at_extremes_is_zero() {
        let pv_neg = PvFold::from_weirdness(-1.0);
        let pv_pos = PvFold::from_weirdness(1.0);
        assert!((pv_neg - 0.0).abs() < 1e-7, "pv(-1) = {} (expected 0)", pv_neg);
        assert!((pv_pos - 0.0).abs() < 1e-7, "pv(+1) = {} (expected 0)", pv_pos);
    }

    /// Smoke test that `Spline1D::default()` compiles and produces an
    /// empty `control_points` vector. F.2.A populates the type with
    /// `evaluate`, `identity`, `from_control_points`.
    #[test]
    fn spline_1d_struct_exists() {
        let s = Spline1D::default();
        assert!(s.control_points.is_empty());
    }

    /// Match-arm-exhaustive check that `BootstrapParam` has all four
    /// variants per campaign doc §2.3. If a variant is added or
    /// removed, this match fails compilation, surfacing the change.
    #[test]
    fn bootstrap_param_has_four_variants() {
        fn variant_id(p: BootstrapParam) -> u8 {
            match p {
                BootstrapParam::MountainsAmplitude => 0,
                BootstrapParam::MountainsScale => 1,
                BootstrapParam::ContinentalScale => 2,
                BootstrapParam::BaseElevationAmplitude => 3,
            }
        }
        assert_eq!(variant_id(BootstrapParam::MountainsAmplitude), 0);
        assert_eq!(variant_id(BootstrapParam::MountainsScale), 1);
        assert_eq!(variant_id(BootstrapParam::ContinentalScale), 2);
        assert_eq!(variant_id(BootstrapParam::BaseElevationAmplitude), 3);
    }
}
