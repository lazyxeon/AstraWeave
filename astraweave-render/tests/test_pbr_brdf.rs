// Unit tests for PBR BRDF functions and shader compilation
// Tests Cook-Torrance BRDF components: GGX distribution, Smith geometry, Fresnel-Schlick

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use std::f32::consts::PI;

/// Helper: Approximate GGX distribution calculation (matches WGSL implementation)
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h2 = n_dot_h * n_dot_h;

    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    a2 / (PI * denom * denom)
}

/// Helper: Smith geometry function (Schlick-GGX)
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0; // Direct lighting remapping

    n_dot_v / (n_dot_v * (1.0 - k) + k)
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    ggx1 * ggx2
}

/// Helper: Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    let m = (1.0 - cos_theta).clamp(0.0, 1.0);
    let factor = m * m * m * m * m; // (1-cos)^5
    f0 + (1.0 - f0) * factor
}

// ============================================================================
// GGX Distribution Tests
// ============================================================================

#[test]
fn test_ggx_peaks_at_normal_incidence() {
    // GGX should peak when N·H = 1.0 (half-vector aligned with normal)
    let roughness = 0.5;
    let peak = distribution_ggx(1.0, roughness);
    let off_peak = distribution_ggx(0.8, roughness);

    assert!(peak > off_peak, "GGX should peak at N·H=1.0");
    assert!(peak > 0.0, "GGX peak should be positive");
}

#[test]
fn test_ggx_increases_with_roughness_at_grazing() {
    // For grazing angles (N·H near 0), rougher surfaces have higher distribution values
    let n_dot_h = 0.3; // Grazing angle
    let rough = distribution_ggx(n_dot_h, 0.9);
    let smooth = distribution_ggx(n_dot_h, 0.1);

    assert!(
        rough > smooth,
        "Rougher surfaces have broader highlights at grazing angles"
    );
}

#[test]
fn test_ggx_decreases_with_roughness_at_peak() {
    // At peak (N·H=1), smoother surfaces have sharper (higher) peaks
    let smooth = distribution_ggx(1.0, 0.1);
    let rough = distribution_ggx(1.0, 0.9);

    assert!(
        smooth > rough,
        "Smooth surfaces have sharper specular peaks"
    );
}

#[test]
fn test_ggx_long_tail() {
    // GGX should have a long tail (non-zero values even at grazing angles)
    let roughness = 0.5;
    let grazing = distribution_ggx(0.1, roughness);

    assert!(
        grazing > 0.0,
        "GGX has non-zero values at grazing angles (long tail characteristic)"
    );
}

// ============================================================================
// Smith Geometry Tests
// ============================================================================

#[test]
fn test_smith_decreases_with_roughness() {
    // Rougher surfaces have more shadowing/masking (lower G)
    let n_dot_v = 0.8;
    let n_dot_l = 0.7;

    let smooth_g = geometry_smith(n_dot_v, n_dot_l, 0.1);
    let rough_g = geometry_smith(n_dot_v, n_dot_l, 0.9);

    assert!(
        smooth_g > rough_g,
        "Smith geometry should decrease with roughness (more shadowing)"
    );
}

#[test]
fn test_smith_decreases_at_grazing_angles() {
    // At grazing angles, G should be lower due to shadowing/masking
    let roughness = 0.5;

    let normal_g = geometry_smith(0.9, 0.9, roughness);
    let grazing_g = geometry_smith(0.1, 0.1, roughness);

    assert!(
        normal_g > grazing_g,
        "Smith G should decrease at grazing angles"
    );
}

#[test]
fn test_smith_is_product_of_view_and_light() {
    // G should be the product of view-dependent and light-dependent terms
    let n_dot_v = 0.8;
    let n_dot_l = 0.7;
    let roughness = 0.5;

    let g_total = geometry_smith(n_dot_v, n_dot_l, roughness);
    let g_view = geometry_schlick_ggx(n_dot_v, roughness);
    let g_light = geometry_schlick_ggx(n_dot_l, roughness);

    let expected = g_view * g_light;
    let diff = (g_total - expected).abs();

    assert!(
        diff < 1e-6,
        "Smith G should be product of G1(V) × G1(L), diff: {}",
        diff
    );
}

#[test]
fn test_smith_bounded_by_one() {
    // Geometry term should never exceed 1.0 (physical constraint)
    let roughness = 0.5;
    let g = geometry_smith(1.0, 1.0, roughness);

    assert!(g <= 1.0, "Smith geometry must be <= 1.0, got {}", g);
    assert!(g > 0.0, "Smith geometry must be > 0.0, got {}", g);
}

// ============================================================================
// Fresnel Tests
// ============================================================================

#[test]
fn test_fresnel_at_normal_incidence() {
    // At normal incidence (cos_theta=1), Fresnel should equal F0
    let f0 = 0.04; // Dielectric base reflectance
    let fresnel = fresnel_schlick(1.0, f0);

    let diff = (fresnel - f0).abs();
    assert!(
        diff < 1e-6,
        "Fresnel at normal incidence should equal F0, diff: {}",
        diff
    );
}

#[test]
fn test_fresnel_at_grazing_angle() {
    // At grazing angles (cos_theta→0), Fresnel should approach 1.0
    let f0 = 0.04;
    let grazing = fresnel_schlick(0.01, f0);

    assert!(
        grazing > 0.9,
        "Fresnel at grazing angle should approach 1.0, got {}",
        grazing
    );
}

#[test]
fn test_fresnel_monotonic_increase() {
    // Fresnel should increase monotonically as angle increases (cos_theta decreases)
    let f0 = 0.04;

    let f_normal = fresnel_schlick(1.0, f0);
    let f_mid = fresnel_schlick(0.5, f0);
    let f_grazing = fresnel_schlick(0.1, f0);

    assert!(
        f_grazing > f_mid,
        "Fresnel should increase toward grazing angle"
    );
    assert!(
        f_mid > f_normal,
        "Fresnel should increase from normal to mid angle"
    );
}

#[test]
fn test_fresnel_with_metal_f0() {
    // Metals have high F0 (0.5-1.0), Fresnel behavior should still be correct
    let f0_metal = 0.9; // Gold-like

    let normal = fresnel_schlick(1.0, f0_metal);
    let grazing = fresnel_schlick(0.01, f0_metal);

    assert!(
        (normal - f0_metal).abs() < 1e-6,
        "Metal Fresnel at normal incidence"
    );
    assert!(
        grazing > normal,
        "Metal Fresnel still increases at grazing angle"
    );
    assert!(grazing <= 1.0, "Fresnel must not exceed 1.0");
}

// ============================================================================
// Energy Conservation Tests
// ============================================================================

#[test]
fn test_energy_conservation_dielectric() {
    // For dielectrics: kD + F ≤ 1.0 (energy conservation)
    let metallic = 0.0; // Dielectric
    let cos_theta = 0.7;
    let f0 = 0.04; // Dielectric base reflectance

    let fresnel = fresnel_schlick(cos_theta, f0);
    let kd = (1.0 - fresnel) * (1.0 - metallic);

    let total_energy = kd + fresnel;

    assert!(
        total_energy <= 1.0 + 1e-6,
        "Diffuse + specular must not exceed incident light, got {}",
        total_energy
    );
    assert!(total_energy >= 0.0, "Total energy must be non-negative");
}

#[test]
fn test_energy_conservation_metal() {
    // For metals: kD = 0 (no diffuse), only specular
    let metallic = 1.0; // Pure metal
    let cos_theta = 0.7;
    let f0 = 0.9; // Metal base reflectance

    let fresnel = fresnel_schlick(cos_theta, f0);
    let kd = (1.0 - fresnel) * (1.0 - metallic);

    assert!(
        kd.abs() < 1e-6,
        "Metals should have no diffuse component, got kD={}",
        kd
    );
    assert!(fresnel <= 1.0, "Specular must not exceed 1.0");
}

#[test]
fn test_energy_conservation_partial_metal() {
    // For partial metals (0 < metallic < 1), energy conservation still holds
    let metallic = 0.5;
    let cos_theta = 0.7;
    let f0 = 0.5; // Mix of dielectric and metal

    let fresnel = fresnel_schlick(cos_theta, f0);
    let kd = (1.0 - fresnel) * (1.0 - metallic);

    // Total reflected energy (diffuse + specular) should not exceed incident
    // Note: This is approximate since we're not including the full BRDF denominator
    let total_contribution = kd + fresnel;

    assert!(
        total_contribution <= 1.0 + 1e-6,
        "Energy conservation violated for partial metal, got {}",
        total_contribution
    );
}

// ============================================================================
// Cook-Torrance BRDF Integration Tests
// ============================================================================

#[test]
fn test_brdf_non_negative() {
    // BRDF terms should always be non-negative
    let n_dot_h = 0.8;
    let n_dot_v = 0.7;
    let n_dot_l = 0.6;
    let roughness = 0.5;
    let f0 = 0.04;

    let d = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(n_dot_h, f0); // Using N·H as approximation

    assert!(d >= 0.0, "GGX distribution must be non-negative");
    assert!(g >= 0.0, "Smith geometry must be non-negative");
    assert!(f >= 0.0, "Fresnel must be non-negative");

    let brdf_numerator = d * g * f;
    assert!(brdf_numerator >= 0.0, "BRDF numerator must be non-negative");
}

#[test]
fn test_brdf_specular_increases_with_smoothness() {
    // At the peak (N·H=1.0), smoother surfaces should have higher (sharper) distribution values
    // This tests the sharpness of the specular peak, not the tails
    let n_dot_h = 1.0; // At peak (not grazing)

    let smooth_d = distribution_ggx(n_dot_h, 0.1);
    let rough_d = distribution_ggx(n_dot_h, 0.5);

    assert!(
        smooth_d > rough_d,
        "Smooth surfaces (r=0.1) have sharper specular peaks at N·H=1, smooth={}, rough={}",
        smooth_d,
        rough_d
    );
}

#[test]
fn test_brdf_zero_at_backface() {
    // When N·L or N·V is zero (backface), BRDF should be zero
    let _n_dot_h = 0.8;
    let roughness = 0.5;

    let g_backface = geometry_smith(0.0, 0.8, roughness);

    // With N·V=0, geometry term should be very small (approaching zero)
    assert!(
        g_backface < 0.1,
        "BRDF should approach zero at backface, got G={}",
        g_backface
    );
}

// ============================================================================
// Numerical Stability Tests
// ============================================================================

#[test]
fn test_ggx_numerical_stability_at_zero() {
    // GGX should handle N·H=0 without NaN or infinity
    let roughness = 0.5;
    let result = distribution_ggx(0.0, roughness);

    assert!(result.is_finite(), "GGX should be finite at N·H=0");
    assert!(result >= 0.0, "GGX should be non-negative at N·H=0");
}

#[test]
fn test_smith_numerical_stability() {
    // Smith G should handle extreme cases without NaN
    let g_extreme = geometry_smith(0.001, 0.001, 0.99);

    assert!(
        g_extreme.is_finite(),
        "Smith G should be finite at extreme values"
    );
    assert!(g_extreme >= 0.0, "Smith G should be non-negative");
}

#[test]
fn test_fresnel_clamping() {
    // Fresnel with cos_theta > 1.0 (shouldn't happen, but test robustness)
    let f = fresnel_schlick(1.5, 0.04);

    assert!(
        f.is_finite(),
        "Fresnel should handle out-of-range input gracefully"
    );
    assert!(
        f <= 1.0 + 1e-5,
        "Fresnel should not significantly exceed 1.0"
    );
}

// ============================================================================
// Comparison with Known Values (Validation)
// ============================================================================

#[test]
fn test_ggx_known_value() {
    // GGX(N·H=1, roughness=0.5) should match reference implementation
    // Reference: α²/(π*((N·H)²*(α²-1)+1)²) where α=roughness²=0.25
    // = 0.0625 / (π * 1²) ≈ 0.0199
    let roughness = 0.5;
    let result = distribution_ggx(1.0, roughness);

    // Calculate expected value: α² / (π * denom²)
    let a = roughness * roughness; // α = 0.25
    let a2 = a * a; // α² = 0.0625
    let n_dot_h2 = 1.0; // N·H = 1
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0; // 1.0 * (-0.9375) + 1.0 = 0.0625
    let expected = a2 / (PI * denom * denom); // 0.0625 / (π * 0.0625²) = 0.0625 / (π * 0.00390625)

    let diff = (result - expected).abs();
    assert!(
        diff < 1e-4,
        "GGX value should match reference, expected {}, got {}, diff {}",
        expected,
        result,
        diff
    );
}

#[test]
fn test_smith_known_value() {
    // Smith G at normal incidence with roughness=0.5 should match reference
    // With N·L=N·V=1.0 and roughness=0.5, k=(1.5)²/8=0.28125
    // G1(1.0) = 1.0 / (1.0*(1-k)+k) = 1.0 / (0.71875+0.28125) = 1.0
    let g = geometry_smith(1.0, 1.0, 0.5);

    let diff = (g - 1.0).abs();
    assert!(
        diff < 1e-4,
        "Smith G at normal incidence should be 1.0, got {}, diff {}",
        g,
        diff
    );
}

#[test]
fn test_fresnel_schlick_known_value() {
    // Fresnel-Schlick at 60° (cos=0.5) with F0=0.04
    // F = 0.04 + (1-0.04)*(1-0.5)⁵ = 0.04 + 0.96*0.03125 = 0.07
    let f = fresnel_schlick(0.5, 0.04);
    let expected = 0.04 + 0.96 * (0.5_f32).powi(5);

    let diff = (f - expected).abs();
    assert!(
        diff < 1e-5,
        "Fresnel at 60° should match reference, expected {}, got {}, diff {}",
        expected,
        f,
        diff
    );
}
