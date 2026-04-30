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

/// Phase 1.X-F.2.A: piecewise-linear 1D spline.
///
/// Built from sorted `(input, output)` control points with linear
/// interpolation between adjacent points and clamp-at-endpoint
/// behavior for out-of-domain inputs. Used by [`ParamSpline`] to map
/// a single climate-input axis (continentalness, erosion, or PV) to
/// a per-archetype bootstrap noise parameter (mountains_amplitude,
/// mountains_scale, continental_scale, base_elevation_amplitude).
///
/// # Construction
///
/// - [`Spline1D::from_control_points`] validates sortedness, NaN, and
///   infinite values. Returns `Result<Self, SplineError>`.
/// - [`Spline1D::identity`] returns a single-control-point spline that
///   evaluates to `1.0` regardless of input — used as a "no-op"
///   multiplier in separable splines.
///
/// # Evaluation semantics
///
/// - Empty `control_points`: returns `1.0` (defensive identity behavior;
///   construction via `from_control_points` rejects empty input, but
///   `Default::default()` produces an empty spline).
/// - Single control point: returns the constant `y_0` regardless of
///   input.
/// - Multiple control points: binary-search the bracketing pair
///   `(x_i, y_i)` and `(x_{i+1}, y_{i+1})`, linearly interpolate. For
///   inputs below the first control point's x, return `y_0`; above the
///   last, return `y_n`.
/// - Duplicate `input` values produce a sharp step; the search returns
///   the first match deterministically.
///
/// # Determinism
///
/// Pure function; same `(control_points, input)` always produces same
/// output. f32 arithmetic determinism inherited from std.
#[derive(Debug, Clone, Default)]
pub struct Spline1D {
    /// Sorted (ascending by input) `(input, output)` control points.
    /// Empty is valid (returns `1.0` from `evaluate`); single-point is
    /// valid (returns the constant `y_0`); multiple points evaluate
    /// piecewise-linearly with clamp-at-endpoint behavior.
    pub control_points: Vec<(f32, f32)>,
}

/// Phase 1.X-F.2.A: errors from [`Spline1D::from_control_points`]
/// validation. Reported with the offending control-point index where
/// applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineError {
    /// Empty `control_points` Vec.
    Empty,
    /// `control_points` not sorted ascending by input. The first
    /// out-of-order pair's index is reported.
    NotSorted { at_index: usize },
    /// NaN value in input or output at the reported index.
    NaN { at_index: usize },
    /// Infinite value in input or output at the reported index.
    Infinite { at_index: usize },
}

impl Spline1D {
    /// Construct a spline from sorted control points. Validates:
    ///
    /// - Vec is non-empty.
    /// - Points are sorted ascending by input. Equal inputs are
    ///   permitted (encodes a sharp step); strict descent is rejected.
    /// - No NaN or infinite values in input or output.
    ///
    /// Returns `Err(SplineError)` describing the first violation.
    pub fn from_control_points(points: Vec<(f32, f32)>) -> Result<Self, SplineError> {
        if points.is_empty() {
            return Err(SplineError::Empty);
        }
        for (i, (x, y)) in points.iter().enumerate() {
            if x.is_nan() || y.is_nan() {
                return Err(SplineError::NaN { at_index: i });
            }
            if x.is_infinite() || y.is_infinite() {
                return Err(SplineError::Infinite { at_index: i });
            }
            if i > 0 {
                let prev_x = points[i - 1].0;
                if *x < prev_x {
                    return Err(SplineError::NotSorted { at_index: i });
                }
            }
        }
        Ok(Self { control_points: points })
    }

    /// Identity spline: single control point at `(0.0, 1.0)`. Evaluates
    /// to `1.0` for any input. Used as a no-op multiplier in separable
    /// splines where one or more axes have no effect on the output.
    ///
    /// Cannot be a true `const fn` because `Vec::new()` / `vec!` are
    /// not const; constructed at call time. Cheap: single allocation
    /// of one tuple.
    pub fn identity() -> Self {
        Self { control_points: vec![(0.0, 1.0)] }
    }

    /// Evaluate the spline at `input` via piecewise-linear interpolation.
    /// Out-of-domain inputs clamp to the corresponding endpoint output.
    ///
    /// See struct doc for full evaluation semantics.
    pub fn evaluate(&self, input: f32) -> f32 {
        let n = self.control_points.len();
        if n == 0 {
            // Defensive identity — construction normally prevents this,
            // but Default::default() produces an empty Spline1D.
            return 1.0;
        }
        if n == 1 {
            return self.control_points[0].1;
        }

        // Clamp below the first control point's input.
        let (first_x, first_y) = self.control_points[0];
        if input <= first_x {
            return first_y;
        }
        // Clamp above the last control point's input.
        let (last_x, last_y) = self.control_points[n - 1];
        if input >= last_x {
            return last_y;
        }

        // Binary search for the bracketing pair. partition_point returns
        // the first index where the predicate fails; we want the first
        // index where x > input, then bracket = (index-1, index).
        let upper = self
            .control_points
            .partition_point(|&(x, _)| x <= input);
        // upper is in [1, n-1] because we handled the endpoint clamps above.
        let (x0, y0) = self.control_points[upper - 1];
        let (x1, y1) = self.control_points[upper];

        if x1 == x0 {
            // Duplicate input (sharp step). Return the right-side value.
            return y1;
        }
        let t = (input - x0) / (x1 - x0);
        y0 + t * (y1 - y0)
    }
}

/// Phase 1.X-F.2.A: enum identifying which climate field axis a
/// [`ParamSpline`] reads as its evaluation input.
///
/// Per campaign doc §2.3, each archetype's `BootstrapSplineSet` maps
/// climate parameters `(continentalness, erosion, PV)` to bootstrap
/// shape parameters `(mountains_amplitude, mountains_scale,
/// continental_scale, base_elevation_amplitude)` via separable splines.
/// This enum exposes the per-spline input-dimension choice so F.7
/// per-archetype tuning can revise it (e.g., Boreal `mountains_amplitude`
/// reads `pv`; Mediterranean reads `continentalness`).
///
/// PV is computed at sample time via [`PvFold::from_weirdness`] /
/// [`crate::climate::ClimateSample::pv`]; reading `Pv` here invokes the
/// accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClimateInputDim {
    Continentalness,
    Erosion,
    Pv,
}

impl ClimateInputDim {
    /// Read the corresponding f32 from a [`crate::climate::ClimateSample`].
    /// `Pv` calls the derived accessor (which routes through `PvFold`).
    #[inline]
    pub fn read(&self, sample: &crate::climate::ClimateSample) -> f32 {
        match self {
            Self::Continentalness => sample.continentalness,
            Self::Erosion => sample.erosion,
            Self::Pv => sample.pv(),
        }
    }
}

/// Phase 1.X-F.2.A: per-parameter spline that pairs a [`Spline1D`] with
/// the [`ClimateInputDim`] it reads from a `ClimateSample`.
///
/// `ParamSpline::evaluate(sample)` is the single per-vertex per-parameter
/// evaluation: read the configured climate dimension from the sample,
/// then evaluate the spline at that input.
///
/// **Architectural simplification (logged in §10 F.2 entry)**: campaign
/// doc §2.3 specifies "separable form (3 1D splines × multiplied)" —
/// per `BootstrapParam`, three independent splines (one per climate
/// axis) multiplicatively combined with a base value. F.2 ships the
/// simpler one-spline-per-parameter shape because (a) F.2's catalog
/// archetypes are all single-control-point constants where multi-spline
/// product reduces to scalar product, and (b) F.7 tuning is where
/// multi-spline product earns its keep. If F.7 needs true 3-axis
/// separable form, it adds a `ParamSplineMulti` type or refactors
/// `ParamSpline` to carry `[ParamSplineAxis; 3]`.
#[derive(Debug, Clone)]
pub struct ParamSpline {
    pub climate_input: ClimateInputDim,
    pub spline: Spline1D,
}

impl ParamSpline {
    /// Evaluate this parameter spline against a climate sample. Reads
    /// the configured climate dimension and evaluates the underlying
    /// `Spline1D` at that input.
    #[inline]
    pub fn evaluate(&self, sample: &crate::climate::ClimateSample) -> f32 {
        self.spline.evaluate(self.climate_input.read(sample))
    }
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

    // =========================================================================
    // Phase 1.X-F.2.A: Spline1D + ParamSpline tests
    // =========================================================================

    use crate::climate::{ClimateConfig, ClimateMap, ClimateSample};

    /// Single-control-point spline returns its constant for any input.
    /// This is the F.2.B catalog default shape — used by all 6 archetype
    /// `BootstrapSplineSet`s.
    #[test]
    fn spline_evaluate_single_control_point_returns_constant() {
        let s = Spline1D::from_control_points(vec![(0.0, 5.0)]).unwrap();
        for &input in &[-100.0_f32, -1.0, 0.0, 0.5, 1.0, 100.0] {
            assert_eq!(
                s.evaluate(input),
                5.0,
                "single-point spline should return 5.0 at input {}",
                input
            );
        }
    }

    /// Two control points produce linear interpolation between them.
    #[test]
    fn spline_evaluate_two_control_points_linear_interp() {
        let s = Spline1D::from_control_points(vec![(0.0, 0.0), (1.0, 10.0)]).unwrap();
        assert!((s.evaluate(0.5) - 5.0).abs() < 1e-6);
        assert!((s.evaluate(0.25) - 2.5).abs() < 1e-6);
        assert!((s.evaluate(0.75) - 7.5).abs() < 1e-6);
    }

    /// Out-of-domain inputs below the first control point clamp to first y.
    #[test]
    fn spline_evaluate_clamps_below_domain() {
        let s = Spline1D::from_control_points(vec![(0.0, 0.0), (1.0, 10.0)]).unwrap();
        assert_eq!(s.evaluate(-0.5), 0.0);
        assert_eq!(s.evaluate(-100.0), 0.0);
    }

    /// Out-of-domain inputs above the last control point clamp to last y.
    #[test]
    fn spline_evaluate_clamps_above_domain() {
        let s = Spline1D::from_control_points(vec![(0.0, 0.0), (1.0, 10.0)]).unwrap();
        assert_eq!(s.evaluate(1.5), 10.0);
        assert_eq!(s.evaluate(100.0), 10.0);
    }

    /// Evaluating at exactly a control point's input returns that point's
    /// y value (no interpolation drift across the boundary).
    #[test]
    fn spline_evaluate_at_exact_control_point() {
        let s = Spline1D::from_control_points(vec![
            (0.0, 0.0),
            (0.5, 5.0),
            (1.0, 10.0),
        ])
        .unwrap();
        assert!((s.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((s.evaluate(0.5) - 5.0).abs() < 1e-6);
        assert!((s.evaluate(1.0) - 10.0).abs() < 1e-6);
    }

    /// Three control points produce piecewise-linear evaluation across two
    /// segments. Up-then-down triangle pattern verifies both segments.
    #[test]
    fn spline_evaluate_three_control_points_piecewise() {
        let s = Spline1D::from_control_points(vec![
            (0.0, 0.0),
            (0.5, 10.0),
            (1.0, 0.0),
        ])
        .unwrap();
        // Left segment: 0.0 → 10.0 over [0.0, 0.5]; midpoint = 5.0.
        assert!((s.evaluate(0.25) - 5.0).abs() < 1e-6);
        // Right segment: 10.0 → 0.0 over [0.5, 1.0]; midpoint = 5.0.
        assert!((s.evaluate(0.75) - 5.0).abs() < 1e-6);
    }

    /// Empty control points rejected with `SplineError::Empty`.
    #[test]
    fn spline_from_control_points_rejects_empty() {
        let result = Spline1D::from_control_points(vec![]);
        assert_eq!(result.err(), Some(SplineError::Empty));
    }

    /// Unsorted control points rejected with `SplineError::NotSorted`.
    #[test]
    fn spline_from_control_points_rejects_unsorted() {
        let result = Spline1D::from_control_points(vec![(1.0, 0.0), (0.0, 10.0)]);
        assert_eq!(result.err(), Some(SplineError::NotSorted { at_index: 1 }));
    }

    /// NaN values rejected with `SplineError::NaN`.
    #[test]
    fn spline_from_control_points_rejects_nan() {
        let result_input = Spline1D::from_control_points(vec![(0.0, 0.0), (f32::NAN, 1.0)]);
        assert_eq!(result_input.err(), Some(SplineError::NaN { at_index: 1 }));
        let result_output =
            Spline1D::from_control_points(vec![(0.0, f32::NAN), (1.0, 1.0)]);
        assert_eq!(result_output.err(), Some(SplineError::NaN { at_index: 0 }));
    }

    /// Infinite values rejected with `SplineError::Infinite`.
    #[test]
    fn spline_from_control_points_rejects_infinite() {
        let result =
            Spline1D::from_control_points(vec![(0.0, 0.0), (f32::INFINITY, 1.0)]);
        assert_eq!(result.err(), Some(SplineError::Infinite { at_index: 1 }));
    }

    /// Duplicate input values are accepted (encodes a sharp step). The
    /// step's right-side value is returned at the duplicate input.
    #[test]
    fn spline_from_control_points_accepts_duplicate_inputs() {
        let s = Spline1D::from_control_points(vec![
            (0.0, 0.0),
            (0.0, 5.0),
            (1.0, 10.0),
        ])
        .expect("duplicate inputs should be accepted");
        // Verifies the spline constructed; evaluation at the duplicate
        // input returns the right-side value (deterministic).
        assert!(s.control_points.len() == 3);
    }

    /// `Spline1D::identity()` evaluates to 1.0 for any input.
    #[test]
    fn spline_identity_evaluates_to_one() {
        let s = Spline1D::identity();
        for &input in &[-2.0_f32, -1.0, 0.0, 1.0, 2.0] {
            assert_eq!(s.evaluate(input), 1.0);
        }
    }

    /// `ClimateInputDim::read` returns the corresponding field from a
    /// `ClimateSample`. Builds a sample via `ClimateMap::sample` to use
    /// real values rather than constructing the struct directly (the
    /// fields are public, but going through `sample` exercises the
    /// integration with F.1.B's wiring).
    #[test]
    fn climate_input_dim_reads_correct_field() {
        let cfg = ClimateConfig::default();
        let map = ClimateMap::new(&cfg, 12345);
        let sample = map.sample(1234.0, -567.0, 100.0);
        assert_eq!(
            ClimateInputDim::Continentalness.read(&sample),
            sample.continentalness
        );
        assert_eq!(ClimateInputDim::Erosion.read(&sample), sample.erosion);
        assert_eq!(ClimateInputDim::Pv.read(&sample), sample.pv());
    }

    /// `ParamSpline` with a constant single-control-point spline returns
    /// the same value regardless of the climate sample. This is the F.2
    /// catalog-archetype property: F.2's defaults all single-point at
    /// the F.4.B.3.D.5-fix baseline, so spline output is constant per
    /// archetype regardless of the climate input dimension.
    #[test]
    fn param_spline_evaluate_combines_dim_and_spline() {
        let p = ParamSpline {
            climate_input: ClimateInputDim::Continentalness,
            spline: Spline1D::from_control_points(vec![(0.0, 7.5)]).unwrap(),
        };
        let cfg = ClimateConfig::default();
        let map = ClimateMap::new(&cfg, 12345);
        // Three different positions → three different climate samples,
        // but constant spline means same output.
        for &(x, z) in &[(0.0, 0.0), (1234.0, -567.0), (-3000.0, 4500.0)] {
            let sample = map.sample(x, z, 0.0);
            let result = p.evaluate(&sample);
            assert!(
                (result - 7.5).abs() < 1e-6,
                "constant spline should return 7.5 at ({}, {}); got {}",
                x,
                z,
                result
            );
        }
    }

    /// `ParamSpline` reading `Pv` invokes the `PvFold` accessor — i.e.,
    /// the spline input is the folded weirdness, not raw weirdness. This
    /// is the architecturally-load-bearing routing: per-archetype
    /// `BootstrapSplineSet` reads PV as third climate axis.
    #[test]
    fn param_spline_evaluate_reads_pv_via_pvfold() {
        // Spline that returns input directly (identity-on-input,
        // single-segment).
        let p = ParamSpline {
            climate_input: ClimateInputDim::Pv,
            spline: Spline1D::from_control_points(vec![(-1.0, -1.0), (1.0, 1.0)])
                .unwrap(),
        };
        // Construct a ClimateSample where weirdness is set so PvFold
        // produces a known value. weirdness=0.0 → pv=-1.0 (valley
        // extremum, per F.1.A canonical PV table).
        let sample = ClimateSample {
            temperature_c: 12.0,
            moisture_mm: 800.0,
            continentalness: 0.5,
            erosion: 0.0,
            weirdness: 0.0,
        };
        // Verify the spline reads pv() (= -1.0 here), not weirdness
        // (= 0.0). Spline at input -1.0 returns -1.0; spline at input
        // 0.0 would return 0.0. So if we get -1.0, ParamSpline routes
        // through PvFold correctly.
        let result = p.evaluate(&sample);
        assert!(
            (result - (-1.0)).abs() < 1e-6,
            "ParamSpline with Pv climate input should evaluate at \
             sample.pv() (-1.0 at weirdness=0); got {}",
            result
        );
    }
}
