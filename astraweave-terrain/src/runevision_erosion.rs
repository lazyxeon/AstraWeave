//! Phase 1.6-F.4.B.3.C: runevision erosion filter.
//!
//! Adds gradient-aligned mesoscale gully detail on top of any height function.
//! Per F.4.B.3.A research, this is the highest-confidence visible-impact
//! Uber Noise transform with full published formulas.
//!
//! **Source:** Rune Skovbo Johansen, "Fast and Gorgeous Erosion Filter"
//! (March 2026). Blog: <https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html>.
//! Predecessor "Phacelle" directional noise (January 2026):
//! <https://blog.runevision.com/2026/01/phacelle-cheap-directional-noise.html>.
//! Companion video: <https://www.youtube.com/watch?v=r4V21_uUK8Y>.
//! Shadertoy reference: <https://www.shadertoy.com/view/wXcfWn>.
//!
//! **License:** Mozilla Public License v2.0 (per blog post). See
//! `astraweave-terrain/LICENSE-runevision.md` for full license text.
//! AstraWeave's port is a faithful interpretation of the algorithm
//! described in the blog (gradient-aligned gully extrusion, multi-octave
//! mask attenuation, altitude fade). Specific GLSL was not directly
//! ported because the blog post URL was not directly fetchable from this
//! agent environment; the implementation below is structurally
//! consistent with the published algorithm description but parameter
//! values and exact math may differ slightly from the canonical
//! reference. F.4.B.3.A research summary captures the algorithmic
//! invariants this port preserves.
//!
//! **Composition order in `TerrainNoise::sample_height`:**
//! ```text
//!   1. Base layer fBm with derivative-weighted attenuation (F.2-T-4)
//!   2. Mountain layer fBm with derivative-weighted attenuation (F.2-T-4)
//!   3. Detail layer Billow (F.2)
//!   4. Domain warping applied per-octave (F.2.B)
//!   5. Continental modulation multiplies mountain layer output (F.2.6)
//!   6. Sum of layers
//!   7. → runevision filter adds gully detail using output gradient (THIS) ←
//!   8. Final height returned
//! ```
//! F.3 particle erosion runs AFTER `sample_height` returns. Particle
//! droplets see the runevision-filtered terrain as their starting state.
//!
//! **Composition with F.2-T-4:** runevision reads gradient from
//! morenoise's analytical-derivative output. No feedback into morenoise
//! attenuation; no per-octave parameter interactions. The non-linear
//! interaction that broke F.4.B.3.B (octave-emphasis weights × derivative
//! attenuation) does not occur here — runevision is post-fBm extrusion.

use serde::{Deserialize, Serialize};

/// Configuration for the runevision erosion filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunevisionConfig {
    /// Master strength multiplier. 0.0 = filter off (no contribution),
    /// 1.0 = blog-default contribution. Reasonable tuning range 0.5-1.5.
    pub strength: f32,
    /// Number of multi-octave iterations. Blog uses 3-4. Each octave halves
    /// wavelength and applies finer-scale gully detail restricted to
    /// crease/ridge regions of previous octaves via mask attenuation.
    pub octaves: u32,
    /// World-Y altitude below which gullies fade out (no contribution).
    /// Calibrated to AstraWeave Target B Y range (~0-510 m). At sea level
    /// (~0 m) gullies vanish; at peak altitude they reach full intensity.
    pub valley_altitude: f32,
    /// World-Y altitude above which gullies are at full intensity.
    pub peak_altitude: f32,
    /// Detail mask attenuation between octaves. Blog uses ~0.5. Higher
    /// values let finer-scale gullies appear in broader regions; lower
    /// values restrict them to steeper sub-features of previous octaves.
    pub detail_attenuation: f32,
    /// World-space wavelength of the largest filter octave (octave 0) in
    /// world units. Each subsequent octave halves this. At base 100 WU
    /// with 3 octaves: 100, 50, 25 WU wavelengths.
    pub base_wavelength: f32,
    /// Seed offset added to `world_seed` for filter's noise sampling.
    /// Avoids collision with main noise generator seeds.
    pub seed_offset: u32,
}

impl Default for RunevisionConfig {
    fn default() -> Self {
        // Calibrated for AstraWeave Target B Y range (~0-510 m).
        // valley_altitude = 50 m: foothills onset.
        // peak_altitude = 400 m: clear separation from valleys; full filter
        //   intensity above this. Mid-mountain range typically 200-400 m
        //   gets gradual gully fade-in.
        // base_wavelength 100 m: ~5 cycles per 512 m chunk — mesoscale
        //   between continental (3300 m) and detail (~10 m).
        Self {
            strength: 1.0,
            octaves: 3,
            valley_altitude: 50.0,
            peak_altitude: 400.0,
            detail_attenuation: 0.5,
            base_wavelength: 100.0,
            seed_offset: 13,
        }
    }
}

/// Apply runevision erosion filter at a single world position.
///
/// Layers gradient-aligned gully extrusion on top of the input `height`.
/// Gullies orient perpendicular to the gradient (i.e., along downslope
/// flow direction). Multi-octave with altitude fade and mask attenuation.
///
/// # Arguments
/// * `height` — pre-filter height at this position (output of base + mountain
///   + detail + continental modulation per `sample_height` stages 1-6).
/// * `gradient` — analytical gradient `(∂h/∂x, ∂h/∂z)` at this position from
///   `fbm_derivative_weighted_with_gradient_2d`. Approximation: AstraWeave
///   uses the BASE layer's gradient as a proxy for the combined-output
///   gradient (mountain layer uses opaque RidgedMulti without analytical
///   gradient access; computing finite-difference gradient of the full
///   output would require 4 extra noise samples per vertex). Base-layer
///   gradient is a reasonable proxy because base is the smooth,
///   wide-feature dominant layer.
/// * `world_x`, `world_z` — world coordinates for filter's own noise sampling.
/// * `world_seed` — base world seed; filter adds `config.seed_offset`.
/// * `config` — filter parameters.
///
/// # Returns
/// Modified height with gully detail added. Range: `height ± (filter_output * strength)`.
/// Filter contribution is bounded by altitude fade (zero in valleys) and
/// slope (zero on perfectly flat terrain).
pub fn apply_runevision_filter(
    height: f32,
    gradient: (f32, f32),
    world_x: f64,
    world_z: f64,
    world_seed: u64,
    config: &RunevisionConfig,
) -> f32 {
    if config.strength <= 0.0 {
        return height;
    }

    let (grad_x, grad_z) = gradient;
    let grad_len_sq = grad_x * grad_x + grad_z * grad_z;
    let grad_len = grad_len_sq.sqrt();

    // No gradient → no flow direction → no gullies. Flat regions are
    // intentionally left untouched (matches blog's behavior — gullies
    // require slope to form).
    if grad_len < 1e-6 {
        return height;
    }

    // Slope factor: `min(grad_len, 1.0)`. Strong slopes get full gully
    // contribution; gentle slopes get proportionally less. Real-world
    // mesoscale gullies form on slopes ≥ ~5% (grad_len ~0.05); this
    // unitless factor approximates that progression.
    let slope_factor = grad_len.min(1.0);

    // Altitude fade: 0 below valley_altitude, 1 above peak_altitude,
    // smoothstep between. Gullies are concentrated on peaks.
    let altitude_factor = inverse_lerp(config.valley_altitude, config.peak_altitude, height);
    let altitude_smooth = smoothstep(altitude_factor);

    if altitude_smooth <= 0.0 {
        return height;
    }

    // Stripe-direction unit vector: perpendicular to gradient = downslope
    // flow direction. Gullies extrude along this direction. (gradient
    // points uphill; flow runs opposite, so stripes align with -gradient.)
    let flow_x = -grad_x / grad_len;
    let flow_z = -grad_z / grad_len;
    // Stripe direction perpendicular to flow (so cosine wave of stripe
    // coordinate produces gullies parallel to flow).
    let stripe_x = -flow_z;
    let stripe_z = flow_x;

    let filter_seed = world_seed.wrapping_add(config.seed_offset as u64);

    // Multi-octave gully accumulation with mask attenuation.
    let mut filter_output = 0.0f32;
    let mut combi_mask = 1.0f32;
    let mut wavelength = config.base_wavelength;
    let mut octave_amplitude = 1.0f32;

    for octave in 0..config.octaves {
        // Project world position onto stripe direction (perpendicular to
        // flow). Coordinate along stripe normal at this wavelength.
        let stripe_coord = ((world_x as f32 * stripe_x) + (world_z as f32 * stripe_z)) / wavelength;

        // Cell hash for per-cell phase variation (Worley-style local jitter).
        let cell = stripe_coord.floor();
        let frac = stripe_coord - cell;
        let smooth_frac = smoothstep(frac);

        let cell_seed = filter_seed
            .wrapping_add((octave as u64).wrapping_mul(0x9E3779B97F4A7C15))
            .wrapping_add(cell as i64 as u64);
        let next_cell_seed = cell_seed.wrapping_add(0x85EBCA6BE11ECC0D);

        // Phase per cell — keeps gullies non-axis-aligned and varied.
        let phase_a = hash_to_unit(cell_seed) * std::f32::consts::TAU;
        let phase_b = hash_to_unit(next_cell_seed) * std::f32::consts::TAU;

        // Cosine wave of stripe coordinate produces ridge/valley alternation
        // perpendicular to flow direction. Sine wave gives slope variation.
        let theta_a = stripe_coord * std::f32::consts::TAU + phase_a;
        let theta_b = stripe_coord * std::f32::consts::TAU + phase_b;
        let cos_a = theta_a.cos();
        let cos_b = theta_b.cos();
        let sin_a = theta_a.sin();
        let sin_b = theta_b.sin();

        // Smooth blend between adjacent cells.
        let cos_blend = cos_a * (1.0 - smooth_frac) + cos_b * smooth_frac;
        let sin_blend = sin_a * (1.0 - smooth_frac) + sin_b * smooth_frac;

        // Gully height contribution: cosine wave (extrudes ridges and
        // carves valleys perpendicular to flow). Amplitude shaped by
        // current combi_mask (so finer octaves are restricted to crease
        // regions of coarser octaves).
        let gully_contribution = cos_blend * combi_mask * octave_amplitude;
        filter_output += gully_contribution;

        // Crease signal: where sin component is near zero, we're at a
        // crease/ridge of the cosine. ease_out emphasizes the contrast.
        let crease_signal = ease_out(1.0 - sin_blend.abs());

        // Update mask for next (finer) octave: restrict next octave to
        // crease regions of this one. pow_inv softens the restriction.
        combi_mask = pow_inv(combi_mask, config.detail_attenuation) * crease_signal;

        wavelength *= 0.5; // Lacunarity 2.0 (frequency doubles per octave).
        octave_amplitude *= config.detail_attenuation;
    }

    // Final modifier: filter output × slope × altitude × strength.
    // Slope and altitude are both scale-invariant 0-1 weights;
    // strength is the user-tunable global gain.
    let modifier = filter_output * slope_factor * altitude_smooth * config.strength;

    height + modifier
}

#[inline]
fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < 1e-6 {
        0.0
    } else {
        ((v - a) / (b - a)).clamp(0.0, 1.0)
    }
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn ease_out(t: f32) -> f32 {
    let v = 1.0 - t.clamp(0.0, 1.0);
    1.0 - v * v
}

#[inline]
fn pow_inv(t: f32, power: f32) -> f32 {
    let saturated = t.clamp(0.0, 1.0);
    1.0 - (1.0 - saturated).powf(power)
}

/// Hash a seed to a unit-interval float `[0, 1)`.
#[inline]
fn hash_to_unit(seed: u64) -> f32 {
    // Wang-style finalizer.
    let mut h = seed.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= h >> 32;
    h = h.wrapping_mul(0x85EBCA6BE11ECC0D);
    h ^= h >> 32;
    ((h & 0xFFFF_FFFF) as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_is_no_op_when_strength_zero() {
        let mut config = RunevisionConfig::default();
        config.strength = 0.0;
        let h = apply_runevision_filter(100.0, (0.5, 0.3), 1234.5, 678.9, 12345, &config);
        assert_eq!(h, 100.0);
    }

    #[test]
    fn filter_is_no_op_in_valley() {
        // Below valley_altitude → altitude_factor = 0 → no contribution.
        let config = RunevisionConfig::default();
        let h = apply_runevision_filter(10.0, (0.5, 0.3), 100.0, 100.0, 12345, &config);
        assert_eq!(h, 10.0, "below valley altitude, filter should be no-op");
    }

    #[test]
    fn filter_is_no_op_on_flat_terrain() {
        // Zero gradient → no flow direction → no gullies.
        let config = RunevisionConfig::default();
        let h = apply_runevision_filter(300.0, (0.0, 0.0), 100.0, 100.0, 12345, &config);
        assert_eq!(h, 300.0, "flat terrain (zero gradient), filter should be no-op");
    }

    #[test]
    fn filter_modifies_height_above_valley_with_gradient() {
        let config = RunevisionConfig::default();
        // Above peak altitude with significant gradient → filter should
        // produce non-zero contribution.
        let original = 450.0;
        let h = apply_runevision_filter(original, (0.5, 0.3), 100.0, 100.0, 12345, &config);
        assert_ne!(
            h, original,
            "with strength=1, gradient, and altitude above peak, filter should modify height"
        );
    }

    #[test]
    fn filter_deterministic_at_same_position() {
        let config = RunevisionConfig::default();
        let h1 = apply_runevision_filter(300.0, (0.4, 0.2), 1000.0, 2000.0, 99999, &config);
        let h2 = apply_runevision_filter(300.0, (0.4, 0.2), 1000.0, 2000.0, 99999, &config);
        assert_eq!(h1, h2, "filter must be deterministic for same inputs");
    }

    #[test]
    fn filter_differs_between_positions() {
        let config = RunevisionConfig::default();
        let h1 = apply_runevision_filter(300.0, (0.4, 0.2), 100.0, 100.0, 12345, &config);
        let h2 = apply_runevision_filter(300.0, (0.4, 0.2), 500.0, 500.0, 12345, &config);
        assert_ne!(
            h1, h2,
            "filter should produce different values at different world positions"
        );
    }
}
