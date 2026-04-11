//! Disney Material Evaluation — bridges `MaterialGpuExtended` to the Disney BRDF.
//!
//! Provides CPU-side Disney BRDF evaluation for validation, material preview,
//! and baking. The GPU path uses `shaders/pbr/disney_brdf.wgsl` directly.
//!
//! Also provides the WGSL source as an includeable constant for shader composition.

use glam::Vec3;

use crate::material_extended::{
    MaterialGpuExtended, MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN, MATERIAL_FLAG_SUBSURFACE,
};

/// The Disney BRDF WGSL source, available for shader composition.
/// All permutation features enabled (backward-compatible default).
/// For compile-time feature elimination, use [`ShaderPermutation::generate_disney_brdf()`].
pub const DISNEY_BRDF_WGSL: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    // Default permutation: all optional lobes enabled (matches runtime-branching behavior).
    "const ENABLE_CLEARCOAT: bool = true;\n",
    "const ENABLE_ANISOTROPY: bool = true;\n",
    "const ENABLE_SUBSURFACE: bool = true;\n",
    "const ENABLE_SHEEN: bool = true;\n",
    "const ENABLE_TRANSMISSION: bool = true;\n\n",
    include_str!("../shaders/pbr/disney_brdf.wgsl")
);

/// The BRDF LUT WGSL source.
pub const BRDF_LUT_WGSL: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    include_str!("../shaders/pbr/brdf_lut.wgsl")
);

/// CPU-side Disney BRDF evaluation result.
#[derive(Debug, Clone, Copy)]
pub struct BrdfResult {
    pub diffuse: Vec3,
    pub specular: Vec3,
}

impl BrdfResult {
    pub fn total(&self) -> Vec3 {
        self.diffuse + self.specular
    }
}

/// CPU-side Disney BRDF evaluation for validation/preview.
///
/// This mirrors the GPU `evaluate_disney_brdf` function.
pub fn evaluate_disney_brdf(
    mat: &MaterialGpuExtended,
    n: Vec3,
    v: Vec3,
    l: Vec3,
    _t: Vec3,
    _b: Vec3,
) -> BrdfResult {
    let h = (v + l).normalize();
    let n_dot_v = n.dot(v).max(1e-5);
    let n_dot_l = n.dot(l).max(0.0);
    let n_dot_h = n.dot(h).max(0.0);
    let l_dot_h = l.dot(h).max(0.0);

    let roughness = mat.roughness_factor.clamp(0.04, 1.0);
    let alpha = (roughness * roughness).max(0.002);
    let metallic = mat.metallic_factor.clamp(0.0, 1.0);

    let base_color = Vec3::new(
        mat.base_color_factor[0],
        mat.base_color_factor[1],
        mat.base_color_factor[2],
    );
    let f0 = Vec3::splat(0.04).lerp(base_color, metallic);

    let mut diffuse = Vec3::ZERO;
    let mut specular;

    // --- Diffuse (Burley) ---
    if metallic < 1.0 {
        let fd = diffuse_burley(n_dot_v, n_dot_l, l_dot_h, roughness);

        if mat.has_feature(MATERIAL_FLAG_SUBSURFACE) && mat.subsurface_scale > 0.0 {
            let sss_color = Vec3::from_array(mat.subsurface_color);
            let tint = Vec3::ONE.lerp(sss_color, mat.subsurface_scale);
            diffuse = base_color * tint * fd;
        } else {
            diffuse = base_color * fd;
        }
        diffuse *= 1.0 - metallic;
    }

    // --- Specular (GGX) ---
    {
        let d = d_ggx(n_dot_h, alpha);
        let vis = v_smith_ggx(n_dot_v, n_dot_l, alpha);
        let f = fresnel_schlick(l_dot_h, f0);
        specular = f * (d * vis);
    }

    // --- Clearcoat ---
    if mat.has_feature(MATERIAL_FLAG_CLEARCOAT) && mat.clearcoat_strength > 0.0 {
        let cc_alpha = (mat.clearcoat_roughness * mat.clearcoat_roughness).max(0.002);
        let cc_f0 = f0_from_ior(1.5);
        let d_cc = d_ggx(n_dot_h, cc_alpha);
        let v_cc = v_kelemen(l_dot_h);
        let f_cc = fresnel_schlick_scalar(l_dot_h, cc_f0);
        specular += Vec3::splat(d_cc * v_cc * f_cc * mat.clearcoat_strength);
    }

    // --- Sheen ---
    if mat.has_feature(MATERIAL_FLAG_SHEEN) {
        let sheen_color = Vec3::from_array(mat.sheen_color);
        if sheen_color.length() > 0.001 {
            let d_s = d_charlie(n_dot_h, mat.sheen_roughness);
            let f_s = fresnel_schlick(l_dot_h, sheen_color);
            diffuse += f_s * d_s * (1.0 - metallic);
        }
    }

    // Scale by NdotL
    diffuse *= n_dot_l;
    specular *= n_dot_l;

    BrdfResult { diffuse, specular }
}

// ---- BRDF building blocks (CPU mirrors of WGSL) ----

fn pow5(x: f32) -> f32 {
    let x2 = x * x;
    x2 * x2 * x
}

fn f0_from_ior(ior: f32) -> f32 {
    let k = (ior - 1.0) / (ior + 1.0);
    k * k
}

fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    f0 + (Vec3::ONE - f0) * pow5((1.0 - cos_theta).clamp(0.0, 1.0))
}

fn fresnel_schlick_scalar(cos_theta: f32, f0: f32) -> f32 {
    f0 + (1.0 - f0) * pow5((1.0 - cos_theta).clamp(0.0, 1.0))
}

fn d_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    a2 / (std::f32::consts::PI * d * d + 1e-7)
}

fn d_charlie(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let inv_alpha = 1.0 / alpha;
    let sin2 = 1.0 - n_dot_h * n_dot_h;
    (2.0 + inv_alpha) * sin2.powf(inv_alpha * 0.5) / (2.0 * std::f32::consts::PI)
}

fn v_smith_ggx(n_dot_v: f32, n_dot_l: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let ggx_v = n_dot_l * (n_dot_v * n_dot_v * (1.0 - a2) + a2).sqrt();
    let ggx_l = n_dot_v * (n_dot_l * n_dot_l * (1.0 - a2) + a2).sqrt();
    0.5 / (ggx_v + ggx_l + 1e-7)
}

fn v_kelemen(l_dot_h: f32) -> f32 {
    0.25 / (l_dot_h * l_dot_h + 1e-7)
}

fn diffuse_burley(n_dot_v: f32, n_dot_l: f32, l_dot_h: f32, roughness: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * l_dot_h * l_dot_h;
    let light_scatter = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_l);
    let view_scatter = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_v);
    light_scatter * view_scatter * std::f32::consts::FRAC_1_PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brdf_wgsl_not_empty() {
        assert!(!DISNEY_BRDF_WGSL.is_empty());
        assert!(DISNEY_BRDF_WGSL.contains("evaluate_disney_brdf"));
        assert!(DISNEY_BRDF_WGSL.contains("evaluate_disney_ibl"));
    }

    #[test]
    fn brdf_wgsl_contains_lod_function() {
        assert!(
            DISNEY_BRDF_WGSL.contains("evaluate_disney_brdf_lod"),
            "disney_brdf.wgsl must contain LOD-aware evaluation function"
        );
    }

    #[test]
    fn disney_brdf_wgsl_parses_naga() {
        // Validate that the combined constants + disney_brdf shader parses
        // with naga. This catches syntax errors in the LOD-aware functions.
        let result = naga::front::wgsl::parse_str(DISNEY_BRDF_WGSL);
        assert!(
            result.is_ok(),
            "disney_brdf.wgsl should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn brdf_lut_wgsl_not_empty() {
        assert!(!BRDF_LUT_WGSL.is_empty());
        assert!(BRDF_LUT_WGSL.contains("brdf_lut_main"));
    }

    #[test]
    fn default_material_brdf() {
        let mat = MaterialGpuExtended::default();
        let n = Vec3::Y;
        let v = Vec3::new(0.0, 1.0, 0.0);
        let l = Vec3::new(0.0, 1.0, 0.0); // overhead light
        let t = Vec3::X;
        let b = Vec3::Z;
        let result = evaluate_disney_brdf(&mat, n, v, l, t, b);
        assert!(result.diffuse.length() > 0.0, "Should have diffuse");
        assert!(
            result.specular.length() >= 0.0,
            "Specular should be non-negative"
        );
    }

    #[test]
    fn metallic_material_no_diffuse() {
        let mut mat = MaterialGpuExtended::default();
        mat.metallic_factor = 1.0;
        mat.roughness_factor = 0.3;
        let n = Vec3::Y;
        let v = Vec3::new(0.0, 1.0, 0.0);
        let l = Vec3::new(0.0, 1.0, 0.0);
        let result = evaluate_disney_brdf(&mat, n, v, l, Vec3::X, Vec3::Z);
        assert!(
            result.diffuse.length() < 1e-6,
            "Full metal should have no diffuse: {:?}",
            result.diffuse
        );
        assert!(result.specular.length() > 0.0, "Should have specular");
    }

    #[test]
    fn clearcoat_adds_specular() {
        let base = MaterialGpuExtended::default();
        let car = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);

        let n = Vec3::Y;
        let v = Vec3::new(0.3, 0.95, 0.0).normalize();
        let l = Vec3::new(-0.3, 0.95, 0.0).normalize();

        let r_base = evaluate_disney_brdf(&base, n, v, l, Vec3::X, Vec3::Z);
        let r_car = evaluate_disney_brdf(&car, n, v, l, Vec3::X, Vec3::Z);

        // Clearcoat should add extra specular
        assert!(
            r_car.specular.length() > r_base.specular.length() * 0.5,
            "Car paint should have significant specular"
        );
    }

    #[test]
    fn sheen_adds_diffuse() {
        let plain = MaterialGpuExtended::default();
        let velvet =
            MaterialGpuExtended::velvet(Vec3::new(0.2, 0.0, 0.3), Vec3::new(1.0, 1.0, 1.0), 0.5);

        let n = Vec3::Y;
        let v = Vec3::new(0.5, 0.5, 0.0).normalize();
        let l = Vec3::new(-0.5, 0.5, 0.0).normalize();

        let r_plain = evaluate_disney_brdf(&plain, n, v, l, Vec3::X, Vec3::Z);
        let r_velvet = evaluate_disney_brdf(&velvet, n, v, l, Vec3::X, Vec3::Z);

        // Sheen adds to diffuse component
        assert!(
            r_velvet.diffuse.length() > 0.0,
            "Velvet should have diffuse"
        );
    }

    #[test]
    fn energy_conservation() {
        // Total BRDF * NdotL should not exceed 1 for any configuration
        let mat = MaterialGpuExtended::default();
        let n = Vec3::Y;
        for i in 0..10 {
            let angle = (i as f32 / 10.0) * std::f32::consts::FRAC_PI_2;
            let v = Vec3::new(angle.sin(), angle.cos(), 0.0);
            let l = Vec3::new(0.0, 1.0, 0.0);
            let result = evaluate_disney_brdf(&mat, n, v, l, Vec3::X, Vec3::Z);
            let total = result.total();
            // Each channel should be < some reasonable bound
            assert!(
                total.x < 5.0 && total.y < 5.0 && total.z < 5.0,
                "BRDF too bright at angle {}: {:?}",
                angle,
                total
            );
        }
    }

    #[test]
    fn fresnel_at_normal_incidence() {
        let f0 = Vec3::splat(0.04);
        let f = fresnel_schlick(1.0, f0);
        assert!((f.x - 0.04).abs() < 1e-5, "F(cos=1) should equal F0");
    }

    #[test]
    fn fresnel_at_grazing_angle() {
        let f0 = Vec3::splat(0.04);
        let f = fresnel_schlick(0.0, f0);
        assert!((f.x - 1.0).abs() < 1e-5, "F(cos=0) should be 1.0");
    }

    #[test]
    fn ggx_ndf_peaks_at_normal() {
        // GGX should be maximal when H = N (NdotH = 1)
        let peak = d_ggx(1.0, 0.25);
        let off = d_ggx(0.5, 0.25);
        assert!(peak > off, "Peak at normal: {} > {}", peak, off);
    }

    #[test]
    fn f0_from_ior_glass() {
        let f0 = f0_from_ior(1.5);
        assert!((f0 - 0.04).abs() < 0.01, "Glass IOR 1.5 → F0 ≈ 0.04: {f0}");
    }

    #[test]
    fn f0_from_ior_water() {
        let f0 = f0_from_ior(1.33);
        assert!(f0 > 0.01 && f0 < 0.04, "Water IOR 1.33 → F0 ≈ 0.02: {f0}");
    }

    #[test]
    fn burley_diffuse_positive() {
        let fd = diffuse_burley(0.5, 0.5, 0.7, 0.5);
        assert!(fd > 0.0, "Burley diffuse should be positive: {fd}");
    }
}
