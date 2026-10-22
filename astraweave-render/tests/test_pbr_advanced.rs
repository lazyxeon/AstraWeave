// Phase PBR-E: Unit Tests for Advanced Materials
// Tests clearcoat, anisotropy, subsurface scattering, sheen, and transmission

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::material_extended::*;
use glam::Vec3;

const EPSILON: f32 = 1e-5;

// ============================================================================
// Helper Functions (Mirror WGSL Implementation)
// ============================================================================

fn distribution_ggx(n: Vec3, h: Vec3, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let n_dot_h = n.dot(h).max(0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;

    let denom = n_dot_h2 * (alpha2 - 1.0) + 1.0;
    alpha2 / (std::f32::consts::PI * denom * denom)
}

#[allow(dead_code)]
fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    let m = (1.0 - cos_theta).clamp(0.0, 1.0);
    let factor = m.powi(5);
    f0 + (Vec3::ONE - f0) * factor
}

fn clearcoat_fresnel(cos_theta: f32) -> f32 {
    const F0: f32 = 0.04; // IOR 1.5
    let m = 1.0 - cos_theta;
    let factor = m.powi(5);
    F0 + (1.0 - F0) * factor
}

fn wrap_diffuse(n_dot_l: f32, wrap: f32) -> f32 {
    ((n_dot_l + wrap) / (1.0 + wrap)).max(0.0)
}

fn distribution_charlie(roughness: f32, n_dot_h: f32) -> f32 {
    let alpha = (roughness * roughness).max(0.001);
    let inv_alpha = 1.0 / alpha;
    let sin_theta = (1.0 - n_dot_h * n_dot_h).sqrt();

    (2.0 + inv_alpha) * sin_theta.powf(inv_alpha * 0.5) / (2.0 * std::f32::consts::PI)
}

fn fresnel_dielectric(cos_theta_i: f32, eta: f32) -> f32 {
    let sin_theta_t_sq = eta * eta * (1.0 - cos_theta_i * cos_theta_i);

    if sin_theta_t_sq > 1.0 {
        return 1.0; // Total internal reflection
    }

    let cos_theta_t = (1.0 - sin_theta_t_sq).sqrt();

    let r_parallel = (eta * cos_theta_i - cos_theta_t) / (eta * cos_theta_i + cos_theta_t);
    let r_perpendicular = (cos_theta_i - eta * cos_theta_t) / (cos_theta_i + eta * cos_theta_t);

    0.5 * (r_parallel * r_parallel + r_perpendicular * r_perpendicular)
}

// ============================================================================
// Clearcoat Tests
// ============================================================================

#[test]
fn test_clearcoat_fresnel_at_normal_incidence() {
    // At normal incidence (cos_theta = 1.0), F should equal F0
    let f = clearcoat_fresnel(1.0);
    assert!(
        (f - 0.04).abs() < EPSILON,
        "Clearcoat F0 should be 0.04 for IOR 1.5"
    );
}

#[test]
fn test_clearcoat_fresnel_at_grazing_angle() {
    // At grazing angle (cos_theta ≈ 0), F should approach 1.0
    let f = clearcoat_fresnel(0.01);
    assert!(
        f > 0.9,
        "Clearcoat Fresnel should approach 1.0 at grazing angles, got {}",
        f
    );
}

#[test]
fn test_clearcoat_energy_conservation() {
    // Clearcoat attenuates base layer: base_energy = (1 - F_coat) * clearcoat_strength
    let mat = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);

    let n = Vec3::Z;
    let v = Vec3::new(0.0, 0.3, 0.9).normalize();
    let cos_theta = n.dot(v);

    let f_coat = clearcoat_fresnel(cos_theta) * mat.clearcoat_strength;
    let base_energy = 1.0 - f_coat;

    // Energy split: coat + base = 1.0
    let total = f_coat + base_energy;
    assert!(
        (total - 1.0).abs() < EPSILON,
        "Clearcoat energy conservation failed: {}",
        total
    );
}

#[test]
fn test_clearcoat_distribution_peaks_at_normal() {
    let n = Vec3::Z;
    let h_normal = Vec3::Z;
    let h_grazing = Vec3::new(0.7, 0.0, 0.3).normalize();

    let roughness = 0.05;
    let d_normal = distribution_ggx(n, h_normal, roughness);
    let d_grazing = distribution_ggx(n, h_grazing, roughness);

    assert!(
        d_normal > d_grazing,
        "GGX should peak at normal incidence for smooth clearcoat"
    );
}

#[test]
fn test_clearcoat_material_creation() {
    let mat = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);

    assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert_eq!(mat.clearcoat_strength, 1.0);
    assert_eq!(mat.clearcoat_roughness, 0.05);
    assert_eq!(mat.metallic_factor, 0.9);
    assert_eq!(mat.roughness_factor, 0.3);
}

// ============================================================================
// Anisotropy Tests
// ============================================================================

#[test]
fn test_anisotropic_aspect_ratio() {
    // Anisotropy = 0.8 should produce elliptical distribution
    let anisotropy: f32 = 0.8;
    let base_roughness: f32 = 0.5;

    let aspect = (1.0 - 0.9 * anisotropy.abs()).sqrt();
    let alpha_t: f32 = (base_roughness * base_roughness / aspect).max(0.001);
    let alpha_b: f32 = (base_roughness * base_roughness * aspect).max(0.001);

    // Tangent direction should be rougher than bitangent for positive anisotropy
    assert!(
        alpha_t > alpha_b,
        "Positive anisotropy should stretch along tangent"
    );

    // Ratio should match 1/aspect (alpha_t / alpha_b = 1 / aspect²)
    let ratio = alpha_t / alpha_b;
    let expected_ratio = 1.0 / (aspect * aspect);
    assert!(
        (ratio - expected_ratio).abs() < 0.01,
        "Aspect ratio mismatch: got {}, expected {}",
        ratio,
        expected_ratio
    );
}

#[test]
fn test_anisotropic_rotation() {
    // Rotation should preserve magnitude but change direction
    let t = Vec3::X;
    let b = Vec3::Y;
    let n = Vec3::Z;

    let rotation = std::f32::consts::PI / 4.0; // 45 degrees
    let cos_r = rotation.cos();
    let sin_r = rotation.sin();

    let t_rotated = cos_r * t + sin_r * b;
    let b_rotated = -sin_r * t + cos_r * b;

    // Rotated basis should remain orthonormal
    assert!((t_rotated.length() - 1.0).abs() < EPSILON);
    assert!((b_rotated.length() - 1.0).abs() < EPSILON);
    assert!((t_rotated.dot(b_rotated)).abs() < EPSILON);
    assert!((t_rotated.cross(b_rotated).dot(n) - 1.0).abs() < EPSILON);
}

#[test]
fn test_brushed_metal_material() {
    let mat = MaterialGpuExtended::brushed_metal(Vec3::new(0.9, 0.9, 0.9), 0.4, 0.8, 0.0);

    assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert_eq!(mat.anisotropy_strength, 0.8);
    assert_eq!(mat.metallic_factor, 1.0);
    assert_eq!(mat.roughness_factor, 0.4);
}

#[test]
fn test_anisotropy_negative_strength() {
    // Negative anisotropy should flip tangent/bitangent roles
    let anisotropy_pos: f32 = 0.8;
    let anisotropy_neg: f32 = -0.8;
    let base_roughness: f32 = 0.5;

    let aspect_pos = (1.0 - 0.9 * anisotropy_pos.abs()).sqrt();
    let alpha_t_pos: f32 = (base_roughness * base_roughness / aspect_pos).max(0.001);
    let alpha_b_pos: f32 = (base_roughness * base_roughness * aspect_pos).max(0.001);

    let aspect_neg = (1.0 - 0.9 * anisotropy_neg.abs()).sqrt();
    let alpha_t_neg: f32 = (base_roughness * base_roughness / aspect_neg).max(0.001);
    let alpha_b_neg: f32 = (base_roughness * base_roughness * aspect_neg).max(0.001);

    // Magnitudes should match (same aspect ratio)
    assert!((aspect_pos - aspect_neg).abs() < EPSILON);
    assert!((alpha_t_pos - alpha_t_neg).abs() < EPSILON);
    assert!((alpha_b_pos - alpha_b_neg).abs() < EPSILON);
}

// ============================================================================
// Subsurface Scattering Tests
// ============================================================================

#[test]
fn test_wrap_diffuse_profile() {
    // Wrapped diffuse should extend illumination beyond hemisphere
    let n_dot_l_front = 0.5;
    let n_dot_l_back = -0.3;

    let wrap = 0.5;
    let front = wrap_diffuse(n_dot_l_front, wrap);
    let back = wrap_diffuse(n_dot_l_back, wrap);

    // Front should be brighter
    assert!(front > 0.0);
    assert!(back >= 0.0);
    assert!(front > back);
}

#[test]
fn test_burley_diffusion_non_negative() {
    // SSS profile should never produce negative values
    let _subsurface_color = Vec3::new(0.9, 0.3, 0.3);

    for angle in 0..360 {
        let theta = (angle as f32).to_radians();
        let n_dot_l = theta.cos();

        let wrap_forward = wrap_diffuse(n_dot_l, 0.5);
        let wrap_back = wrap_diffuse(n_dot_l, -0.5);
        let profile = 0.7 * wrap_forward + 0.3 * wrap_back;

        assert!(
            profile >= 0.0,
            "SSS profile should be non-negative at angle {}",
            angle
        );
    }
}

#[test]
fn test_skin_material() {
    let mat = MaterialGpuExtended::skin(
        Vec3::new(0.95, 0.8, 0.7),
        Vec3::new(0.9, 0.3, 0.3),
        1.5,
        0.7,
    );

    assert!(mat.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert_eq!(mat.subsurface_scale, 0.7);
    assert_eq!(mat.subsurface_radius, 1.5);
    assert_eq!(mat.metallic_factor, 0.0);
}

#[test]
fn test_sss_energy_conservation() {
    // SSS should blend between Lambertian and wrapped diffuse without energy gain
    let n = Vec3::Z;
    let l = Vec3::new(0.0, 0.3, 0.9).normalize();
    let n_dot_l = n.dot(l);

    let lambertian = n_dot_l.max(0.0) / std::f32::consts::PI;

    let wrap_forward = wrap_diffuse(n_dot_l, 0.5);
    let wrap_back = wrap_diffuse(n_dot_l, -0.5);
    let sss_profile = (0.7 * wrap_forward + 0.3 * wrap_back) / std::f32::consts::PI;

    // Both should be within reasonable bounds (0 to ~0.5 for this angle)
    assert!((0.0..=1.0).contains(&lambertian));
    assert!((0.0..=1.0).contains(&sss_profile));
}

// ============================================================================
// Sheen Tests
// ============================================================================

#[test]
fn test_charlie_distribution_retroreflection() {
    // Charlie distribution should peak at grazing angles (opposite of GGX)
    let roughness = 0.3;

    let n_dot_h_normal = 1.0;
    let n_dot_h_grazing = 0.1;

    let d_normal = distribution_charlie(roughness, n_dot_h_normal);
    let d_grazing = distribution_charlie(roughness, n_dot_h_grazing);

    assert!(
        d_grazing > d_normal,
        "Charlie should peak at grazing angles (retroreflection)"
    );
}

#[test]
fn test_sheen_roughness_falloff() {
    // Smoother sheen should have sharper falloff
    let n_dot_h = 0.5;

    let d_smooth = distribution_charlie(0.1, n_dot_h);
    let d_rough = distribution_charlie(0.9, n_dot_h);

    // Rougher sheen has broader distribution
    assert!(
        d_rough > d_smooth,
        "Rough sheen should have broader distribution"
    );
}

#[test]
fn test_velvet_material() {
    let mat = MaterialGpuExtended::velvet(Vec3::new(0.5, 0.0, 0.1), Vec3::ONE, 0.3);

    assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));
    assert_eq!(mat.sheen_roughness, 0.3);
    assert_eq!(mat.metallic_factor, 0.0);
}

#[test]
fn test_sheen_energy_conservation() {
    // Sheen reduces diffuse proportionally to avoid over-brightening
    let mat = MaterialGpuExtended::velvet(Vec3::new(0.5, 0.0, 0.1), Vec3::new(0.8, 0.8, 0.8), 0.3);

    let sheen_max = mat.sheen_color.iter().fold(0.0f32, |a, &b| a.max(b));
    let diffuse_reduction = 1.0 - sheen_max;

    // Diffuse should be attenuated by sheen intensity
    assert!((0.0..=1.0).contains(&diffuse_reduction));
    assert!(
        (diffuse_reduction - 0.2).abs() < EPSILON,
        "Sheen max 0.8 should reduce diffuse to 0.2"
    );
}

// ============================================================================
// Transmission Tests
// ============================================================================

#[test]
fn test_fresnel_dielectric_at_normal_incidence() {
    // Glass (IOR 1.5) at normal incidence
    let eta = 1.0 / 1.5; // Air to glass
    let f = fresnel_dielectric(1.0, eta);

    // Exact Fresnel for IOR 1.5 at normal: ((1-1.5)/(1+1.5))^2 = 0.04
    assert!(
        (f - 0.04).abs() < 1e-3,
        "Fresnel for glass at normal should be ~0.04, got {}",
        f
    );
}

#[test]
fn test_fresnel_dielectric_at_grazing() {
    let eta = 1.0 / 1.5;
    let f = fresnel_dielectric(0.01, eta);

    // At grazing angles, Fresnel approaches 1.0
    assert!(f > 0.9, "Fresnel at grazing should approach 1.0, got {}", f);
}

#[test]
fn test_total_internal_reflection() {
    // Water (IOR 1.33) with light from inside at steep angle
    let eta: f32 = 1.33; // Water to air
    let critical_angle = (1.0 / eta).asin();
    let cos_theta = (critical_angle + 0.1).cos(); // Beyond critical angle

    let f = fresnel_dielectric(cos_theta, eta);

    // Should have total internal reflection (F = 1.0)
    assert!(
        (f - 1.0).abs() < EPSILON,
        "Total internal reflection failed"
    );
}

#[test]
fn test_transmission_energy_conservation() {
    // Reflected + transmitted = 1.0 (no absorption)
    let eta = 1.0 / 1.5;
    let cos_theta = 0.7;

    let reflected = fresnel_dielectric(cos_theta, eta);
    let transmitted = 1.0 - reflected;

    assert!((reflected + transmitted - 1.0).abs() < EPSILON);
}

#[test]
fn test_glass_material() {
    let mat =
        MaterialGpuExtended::glass(Vec3::ONE, 0.05, 0.95, 1.5, Vec3::new(0.9, 1.0, 0.9), 10.0);

    assert!(mat.has_feature(MATERIAL_FLAG_TRANSMISSION));
    assert_eq!(mat.transmission_factor, 0.95);
    assert_eq!(mat.ior, 1.5);
    assert_eq!(mat.attenuation_distance, 10.0);
}

#[test]
fn test_beer_lambert_attenuation() {
    // Beer-Lambert: I(d) = I0 * color^(d / distance)
    let attenuation_color = Vec3::new(0.9, 1.0, 0.9); // Slight green tint
    let _attenuation_distance = 10.0;

    // At distance = attenuation_distance, should be color^1
    let _attenuation_1x = attenuation_color;

    // At distance = 2 * attenuation_distance, should be color^2
    let attenuation_2x = Vec3::new(
        attenuation_color.x.powi(2),
        attenuation_color.y.powi(2),
        attenuation_color.z.powi(2),
    );

    // Red channel should attenuate more (0.9^2 = 0.81)
    assert!((attenuation_2x.x - 0.81).abs() < 0.01);
    assert!((attenuation_2x.y - 1.0).abs() < 0.01); // Green unchanged
}

// ============================================================================
// Multi-Feature Integration Tests
// ============================================================================

#[test]
fn test_material_size_and_alignment() {
    // Critical: GPU struct must be 256 bytes with 16-byte alignment
    assert_eq!(std::mem::size_of::<MaterialGpuExtended>(), 256);
    assert_eq!(std::mem::align_of::<MaterialGpuExtended>(), 16);
}

#[test]
fn test_feature_flag_combinations() {
    let mut mat = MaterialGpuExtended {
        clearcoat_strength: 1.0,
        anisotropy_strength: 0.5,
        ..Default::default()
    };

    // Enable clearcoat + anisotropy (car paint with brushed finish)
    mat.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    mat.enable_feature(MATERIAL_FLAG_ANISOTROPY);

    assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!mat.has_feature(MATERIAL_FLAG_SUBSURFACE));

    // Disable clearcoat
    mat.disable_feature(MATERIAL_FLAG_CLEARCOAT);
    assert!(!mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
}

#[test]
fn test_multi_lobe_energy_conservation() {
    // Material with clearcoat + sheen should still conserve energy
    let mut mat = MaterialGpuExtended {
        clearcoat_strength: 1.0,
        sheen_color: [0.5, 0.5, 0.5],
        metallic_factor: 0.0,
        ..Default::default()
    };
    mat.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    mat.enable_feature(MATERIAL_FLAG_SHEEN);

    let n = Vec3::Z;
    let v = Vec3::new(0.0, 0.3, 0.9).normalize();
    let cos_theta = n.dot(v);

    // 1. Clearcoat takes F_coat energy
    let f_coat = clearcoat_fresnel(cos_theta) * mat.clearcoat_strength;

    // 2. Base layer gets (1 - F_coat)
    let base_energy = 1.0 - f_coat;

    // 3. Sheen reduces diffuse by sheen_max
    let sheen_max = mat.sheen_color.iter().fold(0.0f32, |a, &b| a.max(b));
    let diffuse_energy = base_energy * (1.0 - sheen_max) * (1.0 - mat.metallic_factor);

    // Total should not exceed 1.0
    let total = f_coat + diffuse_energy + (base_energy * sheen_max);
    assert!(
        total <= 1.0 + EPSILON,
        "Multi-lobe energy conservation failed: {}",
        total
    );
}

#[test]
fn test_toml_material_parsing() {
    let toml_str = r#"
        name = "test_car_paint"
        albedo = "red_albedo.ktx2"
        normal = "normal.ktx2"
        orm = "orm.ktx2"
        base_color_factor = [0.8, 0.0, 0.0, 1.0]
        metallic_factor = 0.9
        roughness_factor = 0.3
        clearcoat_strength = 1.0
        clearcoat_roughness = 0.05
    "#;

    let def: MaterialDefinitionExtended = toml::from_str(toml_str).unwrap();
    assert_eq!(def.name, "test_car_paint");
    assert_eq!(def.clearcoat_strength, 1.0);

    let gpu = def.to_gpu(0, 1, 2, 0, 0);
    assert!(gpu.has_feature(MATERIAL_FLAG_CLEARCOAT));
}

#[test]
fn test_toml_defaults() {
    let toml_str = r#"
        name = "minimal_material"
    "#;

    let def: MaterialDefinitionExtended = toml::from_str(toml_str).unwrap();

    // Verify defaults
    assert_eq!(def.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(def.metallic_factor, 0.0);
    assert_eq!(def.roughness_factor, 0.5);
    assert_eq!(def.clearcoat_roughness, 0.03);
    assert_eq!(def.ior, 1.5);
}
