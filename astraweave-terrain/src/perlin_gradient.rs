//! Phase 1.6-F.2-T-4: analytical-derivative Perlin noise for derivative-weighted fBm.
//!
//! Returns `(value, dvalue/dx, dvalue/dz)` in one call. Based on Inigo Quilez's
//! gradient-noise derivation at <https://iquilezles.org/articles/gradientnoise>.
//!
//! This is a 2D specialization — AstraWeave's terrain heightmap is always 2D.
//! If 3D is ever needed, extend to return 4 components.
//!
//! The derivative-weighted fBm wrapper here implements Quilez's "fake erosion"
//! pattern from <https://iquilezles.org/articles/morenoise> (2008): each
//! higher-frequency octave is attenuated by the magnitude of the accumulated
//! gradient, so steep regions (where spikes would otherwise accumulate) get
//! smoothed while flat regions retain full fBm character.
//!
//! This module is the F.2-T-4 implementation per
//! `docs/audits/terrain_noise_research_2026-04-22.md` Rank 1 recommendation.

/// 2D gradient hash. Maps integer lattice points to one of 8 unit-magnitude
/// gradient vectors. Using 8 gradients rather than 4 reduces axis-aligned
/// artifacts; the diagonal gradients are normalized by (1/√2) so all 8 have
/// magnitude ~1.
#[inline]
fn grad2(seed: u32, ix: i32, iz: i32) -> (f32, f32) {
    const DIAG: f32 = std::f32::consts::FRAC_1_SQRT_2; // 1/√2 ≈ 0.7071
    let h = hash_2d(seed, ix, iz) & 7;
    match h {
        0 => (1.0, 0.0),
        1 => (-1.0, 0.0),
        2 => (0.0, 1.0),
        3 => (0.0, -1.0),
        4 => (DIAG, DIAG),
        5 => (-DIAG, DIAG),
        6 => (DIAG, -DIAG),
        7 => (-DIAG, -DIAG),
        _ => unreachable!(),
    }
}

/// Wang-style integer hash. Deterministic per `(seed, ix, iz)`.
#[inline]
fn hash_2d(seed: u32, ix: i32, iz: i32) -> u32 {
    let mut h = seed.wrapping_add(ix as u32).wrapping_mul(0x9E3779B9);
    h ^= h >> 16;
    h = h.wrapping_add(iz as u32).wrapping_mul(0x85EBCA6B);
    h ^= h >> 13;
    h.wrapping_mul(0xC2B2AE35)
}

/// Ken Perlin's 5th-degree smoothstep fade function and its derivative.
/// `f(t) = 6t⁵ − 15t⁴ + 10t³`  →  `f'(t) = 30t⁴ − 60t³ + 30t² = 30t²(t−1)²`
#[inline]
fn fade_and_deriv(t: f32) -> (f32, f32) {
    let t2 = t * t;
    let t3 = t2 * t;
    let fade = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    let dfade = 30.0 * t2 * (t - 1.0) * (t - 1.0);
    (fade, dfade)
}

/// Sample Perlin noise with analytical gradient.
/// Returns `(value, dvalue/dx, dvalue/dz)`.
///
/// Derivation (see Quilez, gradientnoise article):
/// Perlin noise at a sample point `(x, z)` inside a unit lattice cell is
/// the bilinear interpolation of four dot products, with fade-function
/// weights. The dot products and fade weights are each differentiable with
/// respect to `(x, z)`, so the final gradient is closed-form.
///
/// This function does NOT match `noise::Perlin::get`'s output exactly — it
/// uses a different hash function and a slightly different gradient set. It
/// matches Perlin's statistical properties (range, smoothness, isotropy) and
/// its analytical derivatives match finite-difference derivatives to high
/// precision. That is what derivative-weighted fBm requires.
pub fn perlin_noised_2d(seed: u32, x: f32, z: f32) -> (f32, f32, f32) {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fz = z - iz as f32;

    // Four lattice gradients.
    let g00 = grad2(seed, ix, iz);
    let g10 = grad2(seed, ix + 1, iz);
    let g01 = grad2(seed, ix, iz + 1);
    let g11 = grad2(seed, ix + 1, iz + 1);

    // Dot products: grad_ij · offset_ij (offset from corner to sample point).
    let d00 = g00.0 * fx + g00.1 * fz;
    let d10 = g10.0 * (fx - 1.0) + g10.1 * fz;
    let d01 = g01.0 * fx + g01.1 * (fz - 1.0);
    let d11 = g11.0 * (fx - 1.0) + g11.1 * (fz - 1.0);

    // Fade weights and their derivatives wrt fx / fz.
    let (u, du) = fade_and_deriv(fx);
    let (v, dv) = fade_and_deriv(fz);

    // Bilinear interpolation coefficients.
    let k0 = d00;
    let k1 = d10 - d00;
    let k2 = d01 - d00;
    let k3 = d00 + d11 - d10 - d01;

    let value = k0 + k1 * u + k2 * v + k3 * u * v;

    // Analytical derivatives (Quilez's closed form).
    // dv/dfx splits into two terms:
    //   1) `du * (k1 + k3*v)` — differentiation of the fade-weighted interpolation.
    //   2) gradient contribution from the dot products at each corner, weighted by the
    //      corner's fade-product `(1-u)(1-v)`, etc.
    let one_minus_u = 1.0 - u;
    let one_minus_v = 1.0 - v;

    let dvalue_dx = du * (k1 + k3 * v)
        + g00.0 * one_minus_u * one_minus_v
        + g10.0 * u * one_minus_v
        + g01.0 * one_minus_u * v
        + g11.0 * u * v;

    let dvalue_dz = dv * (k2 + k3 * u)
        + g00.1 * one_minus_u * one_minus_v
        + g10.1 * u * one_minus_v
        + g01.1 * one_minus_u * v
        + g11.1 * u * v;

    (value, dvalue_dx, dvalue_dz)
}

/// Derivative-weighted fBm using analytical-gradient Perlin.
///
/// Per Quilez's morenoise article (iquilezles.org/articles/morenoise, 2008):
/// accumulated gradient magnitude attenuates each new octave's contribution,
/// so high-frequency octaves are suppressed on steep terrain where they would
/// otherwise produce spike artifacts. Flat regions retain full fBm character.
///
/// Returns only the final value. The gradient is accumulated internally and
/// consumed by the attenuation term; exposing it would be a richer API but
/// terrain callers don't currently need it.
///
/// Phase 1.6-F.4.B.3.B: optional `octave_weights` parameter implements
/// Murray's "octave-emphasis tuning" (GDC 2017 ~39:18-40:15). When `Some`,
/// per-octave amplitude is taken from the slice instead of the standard
/// `persistence^i` exponential decay. Lets callers damp octave 0 and boost
/// mid-octaves to break the "first octave dominates" pattern that produces
/// uniform peak shapes. When `None`, behavior is byte-identical to
/// pre-F.4.B.3.B (Quilez H=1, G=persistence).
///
/// Critical caveat: octave-emphasis weights are bespoke tuning — no
/// published source provides specific numerical values. Standard H=1,
/// G=0.5 is physically validated for terrain realism per Quilez. Departing
/// from it is an aesthetic choice, not a correctness improvement.
pub fn fbm_derivative_weighted_2d(
    seed: u32,
    x: f32,
    z: f32,
    octaves: u32,
    persistence: f32,
    lacunarity: f32,
    octave_weights: Option<&[f32]>,
) -> f32 {
    fbm_derivative_weighted_with_gradient_2d(
        seed,
        x,
        z,
        octaves,
        persistence,
        lacunarity,
        octave_weights,
    )
    .0
}

/// Phase 1.6-F.4.B.3.C: same as `fbm_derivative_weighted_2d` but also returns
/// the accumulated analytical gradient `(grad_x, grad_z)`. Exposed for the
/// runevision erosion filter (`runevision_erosion.rs`), which needs gradient
/// direction to align its gully extrusion with downslope flow.
///
/// Internal computation is identical to `fbm_derivative_weighted_2d`; the
/// gradient was already accumulated for Quilez's attenuation term but was
/// previously discarded at function return. Now exposed.
pub fn fbm_derivative_weighted_with_gradient_2d(
    seed: u32,
    x: f32,
    z: f32,
    octaves: u32,
    persistence: f32,
    lacunarity: f32,
    octave_weights: Option<&[f32]>,
) -> (f32, (f32, f32)) {
    let mut value = 0.0f32;
    let mut grad_x = 0.0f32;
    let mut grad_z = 0.0f32;
    // Used only when octave_weights is None — preserves byte-identical
    // pre-F.4.B.3.B behavior on default code paths.
    let mut amplitude_default = 1.0f32;
    let mut frequency = 1.0f32;

    for i in 0..octaves {
        let (n, dn_dx, dn_dz) =
            perlin_noised_2d(seed.wrapping_add(i), x * frequency, z * frequency);

        // F.4.B.3.B: select per-octave amplitude. Slice indexing is bounds-
        // checked; out-of-range octave indices fall back to 0.0 (effectively
        // skipping that octave) which is the documented contract per the
        // NoiseConfig field comment.
        let amplitude = match octave_weights {
            Some(w) => w.get(i as usize).copied().unwrap_or(0.0),
            None => amplitude_default,
        };

        // Quilez's attenuation: 1 / (1 + |grad|²)
        let attenuation = 1.0 / (1.0 + grad_x * grad_x + grad_z * grad_z);
        value += amplitude * n * attenuation;

        // Accumulate gradient. Chain rule: d(amplitude * n(x*freq)) / dx
        // = amplitude * dn_dx * frequency.
        grad_x += amplitude * dn_dx * frequency;
        grad_z += amplitude * dn_dz * frequency;

        amplitude_default *= persistence;
        frequency *= lacunarity;
    }

    (value, (grad_x, grad_z))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F.2-T-4.A validation: analytical derivatives must match finite-difference
    /// derivatives at a handful of representative positions. Tolerance 0.02 —
    /// analytical derivatives are exact; numerical has `O(eps²)` truncation at
    /// `eps=1e-3`.
    #[test]
    fn perlin_noised_matches_numerical_derivative() {
        let eps = 1e-3f32;
        let seed = 42u32;
        let positions = [(1.5f32, 2.7f32), (-3.1, 0.8), (10.0, -5.0), (0.5, 0.5)];
        for &(x, z) in &positions {
            let (_v, dvdx_a, dvdz_a) = perlin_noised_2d(seed, x, z);

            let (vp, _, _) = perlin_noised_2d(seed, x + eps, z);
            let (vm, _, _) = perlin_noised_2d(seed, x - eps, z);
            let dvdx_n = (vp - vm) / (2.0 * eps);
            assert!(
                (dvdx_a - dvdx_n).abs() < 0.02,
                "analytical dv/dx ({dvdx_a}) diverges from numerical ({dvdx_n}) at ({x}, {z}); diff {}",
                (dvdx_a - dvdx_n).abs()
            );

            let (vp, _, _) = perlin_noised_2d(seed, x, z + eps);
            let (vm, _, _) = perlin_noised_2d(seed, x, z - eps);
            let dvdz_n = (vp - vm) / (2.0 * eps);
            assert!(
                (dvdz_a - dvdz_n).abs() < 0.02,
                "analytical dv/dz ({dvdz_a}) diverges from numerical ({dvdz_n}) at ({x}, {z}); diff {}",
                (dvdz_a - dvdz_n).abs()
            );
        }
    }

    /// F.2-T-4.A sanity: Perlin-style noise output must be in a bounded range
    /// (approximately `[-1, 1]` for 8-gradient Perlin; often narrower in practice
    /// because the interior of each cell doesn't reach the corner extremes).
    /// Also must actually vary (not constant — detects hash or interpolation bug).
    #[test]
    fn perlin_noised_value_in_expected_range() {
        let seed = 7u32;
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for i in 0..300 {
            let x = (i as f32) * 0.11;
            for j in 0..300 {
                let z = (j as f32) * 0.11;
                let (v, _, _) = perlin_noised_2d(seed, x, z);
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
            }
        }
        assert!(
            (-1.0..1.0).contains(&min) && (-1.0..1.0).contains(&max),
            "value out of [-1, 1] range: [{min}, {max}]"
        );
        assert!(
            min < -0.2 && max > 0.2,
            "value range too narrow (hash or interpolation bug?): [{min}, {max}]"
        );
    }

    /// F.2-T-4.B validation: on flat-ish regions (no cumulative gradient), the
    /// derivative-weighted fBm output must match plain fBm. With `octaves=1`
    /// the accumulated gradient starts at zero, so the first-octave attenuation
    /// is 1/(1+0) = 1, and the output is identically the first-octave Perlin
    /// value. Any deviation from plain fBm under these conditions would indicate
    /// a bug in either amplitude/frequency scaling or the attenuation term.
    #[test]
    fn derivative_weighted_matches_plain_fbm_on_flat_regions() {
        let seed = 42u32;
        let octaves = 1u32;
        let persistence = 0.5f32;
        let lacunarity = 2.0f32;

        let plain_fbm = |x: f32, z: f32| -> f32 {
            let mut v = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            for i in 0..octaves {
                let (n, _, _) = perlin_noised_2d(seed.wrapping_add(i), x * freq, z * freq);
                v += amp * n;
                amp *= persistence;
                freq *= lacunarity;
            }
            v
        };

        let mut total_error = 0.0f32;
        let mut total_magnitude = 0.0f32;
        for i in 0..50 {
            for j in 0..50 {
                // Small spatial step keeps gradients small. Offset + irrational-ish
                // scale avoids landing on lattice corners where Perlin is 0.
                let x = i as f32 * 0.017 + 0.333;
                let z = j as f32 * 0.017 + 0.777;
                let plain = plain_fbm(x, z);
                let weighted =
                    fbm_derivative_weighted_2d(seed, x, z, octaves, persistence, lacunarity, None);
                total_error += (plain - weighted).abs();
                total_magnitude += plain.abs();
            }
        }
        let rel_err = total_error / total_magnitude.max(1e-6);
        assert!(
            rel_err < 0.01,
            "Derivative-weighted fBm at octaves=1 should match plain fBm identically (d=0 at first octave → attenuation=1); measured {:.4}% relative error",
            rel_err * 100.0
        );
    }

    /// F.2-T-4.B validation: on high-octave sampling, derivative-weighted fBm
    /// must be measurably smoother (lower local curvature) than plain fBm. This
    /// is the behavioral test confirming the attenuation actually fires.
    #[test]
    fn derivative_weighted_smoother_than_plain_fbm_on_rough_regions() {
        let seed = 42u32;
        let octaves = 5u32;
        let persistence = 0.5f32;
        let lacunarity = 2.0f32;

        let plain_fbm = |x: f32, z: f32| -> f32 {
            let mut v = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            for i in 0..octaves {
                let (n, _, _) = perlin_noised_2d(seed.wrapping_add(i), x * freq, z * freq);
                v += amp * n;
                amp *= persistence;
                freq *= lacunarity;
            }
            v
        };

        const GRID: usize = 100;
        let mut plain_h = vec![0f32; GRID * GRID];
        let mut weighted_h = vec![0f32; GRID * GRID];
        for i in 0..GRID {
            for j in 0..GRID {
                // Spatial step 0.37 is non-integer to avoid landing on lattice
                // corners at any octave (Perlin returns 0 at integer coordinates).
                // Offset by a small irrational-ish constant per axis for the same
                // reason.
                let x = i as f32 * 0.37 + 0.111;
                let z = j as f32 * 0.37 + 0.777;
                plain_h[i * GRID + j] = plain_fbm(x, z);
                weighted_h[i * GRID + j] =
                    fbm_derivative_weighted_2d(seed, x, z, octaves, persistence, lacunarity, None);
            }
        }

        let curvature = |h: &[f32]| -> f32 {
            let mut sum = 0.0f32;
            let mut n = 0u32;
            for i in 1..GRID - 1 {
                for j in 1..GRID - 1 {
                    let c = h[i * GRID + j];
                    let avg = (h[(i - 1) * GRID + j]
                        + h[(i + 1) * GRID + j]
                        + h[i * GRID + j - 1]
                        + h[i * GRID + j + 1])
                        * 0.25;
                    sum += (c - avg).abs();
                    n += 1;
                }
            }
            sum / n as f32
        };

        let plain_c = curvature(&plain_h);
        let weighted_c = curvature(&weighted_h);
        println!(
            "F.2-T-4 smoother test: plain_curv={plain_c:.4}, weighted_curv={weighted_c:.4}, ratio={:.4}",
            weighted_c / plain_c
        );
        assert!(
            weighted_c < plain_c,
            "Derivative-weighted fBm not smoother than plain (weighted={weighted_c}, plain={plain_c})"
        );
        assert!(
            weighted_c < plain_c * 0.85,
            "Derivative-weighted fBm not meaningfully smoother: weighted={weighted_c} vs plain={plain_c}; expected ≥15% reduction"
        );
    }
}
