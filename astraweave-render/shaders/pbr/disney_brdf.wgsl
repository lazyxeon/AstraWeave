// Disney Principled BRDF — Full-featured PBR material evaluation
//
// Implements the following lobes from the Disney/glTF-extended model:
//   1. Base diffuse (Burley diffuse model)
//   2. Specular GGX (microfacet Cook-Torrance: GGX NDF, Smith-GGX geometry, Schlick Fresnel)
//   3. Clearcoat (GGX with separate roughness, fixed IOR 1.5)
//   4. Anisotropy (anisotropic GGX with tangent direction)
//   5. Sheen (Charlie distribution for fabric/velvet)
//   6. Subsurface approximation (wrap lighting)
//   7. Transmission (refraction with Beer's law attenuation)
//
// This file is designed to be #included from the main lighting shader.

// PI, TWO_PI, HALF_PI, INV_PI provided by constants.wgsl (prepended on Rust side).

// ======================================================================
// Material data (matches MaterialGpuExtended layout)
// ======================================================================

struct DisneyMaterial {
    base_color:           vec3<f32>,
    alpha:                f32,
    metallic:             f32,
    roughness:            f32,
    occlusion:            f32,
    flags:                u32,
    emissive:             vec3<f32>,
    // Clearcoat
    clearcoat:            f32,
    clearcoat_roughness:  f32,
    // Anisotropy
    anisotropy:           f32,
    anisotropy_rotation:  f32,
    // Subsurface
    subsurface_color:     vec3<f32>,
    subsurface_scale:     f32,
    // Sheen
    sheen_color:          vec3<f32>,
    sheen_roughness:      f32,
    // Transmission
    transmission:         f32,
    ior:                  f32,
    attenuation_color:    vec3<f32>,
    attenuation_distance: f32,
};

// Feature flags
const FLAG_CLEARCOAT:    u32 = 0x01u;
const FLAG_ANISOTROPY:   u32 = 0x02u;
const FLAG_SUBSURFACE:   u32 = 0x04u;
const FLAG_SHEEN:        u32 = 0x08u;
const FLAG_TRANSMISSION: u32 = 0x10u;

fn has_flag(flags: u32, flag: u32) -> bool {
    return (flags & flag) != 0u;
}

// ======================================================================
// Common utility functions
// ======================================================================

fn pow5(x: f32) -> f32 {
    let x2 = x * x;
    return x2 * x2 * x;
}

fn f0_from_ior(ior: f32) -> f32 {
    let k = (ior - 1.0) / (ior + 1.0);
    return k * k;
}

// ======================================================================
// 1. Fresnel
// ======================================================================

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow5(saturate(1.0 - cos_theta));
}

fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow5(saturate(1.0 - cos_theta));
}

fn fresnel_schlick_scalar(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow5(saturate(1.0 - cos_theta));
}

// ======================================================================
// 2. Normal Distribution Functions
// ======================================================================

// Isotropic GGX (Trowbridge-Reitz)
fn d_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d + 1e-7);
}

// Anisotropic GGX
fn d_ggx_aniso(n_dot_h: f32, h: vec3<f32>, t: vec3<f32>, b: vec3<f32>,
               alpha_t: f32, alpha_b: f32) -> f32 {
    let h_dot_t = dot(h, t);
    let h_dot_b = dot(h, b);
    let at2 = alpha_t * alpha_t;
    let ab2 = alpha_b * alpha_b;
    let d = (h_dot_t * h_dot_t / at2) + (h_dot_b * h_dot_b / ab2)
          + n_dot_h * n_dot_h;
    return 1.0 / (PI * alpha_t * alpha_b * d * d + 1e-7);
}

// Charlie distribution for sheen (Conty & Kulla 2017)
fn d_charlie(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let inv_alpha = 1.0 / alpha;
    let cos2 = n_dot_h * n_dot_h;
    let sin2 = 1.0 - cos2;
    return (2.0 + inv_alpha) * pow(sin2, inv_alpha * 0.5) / (2.0 * PI);
}

// ======================================================================
// 3. Geometry / Visibility
// ======================================================================

fn v_smith_ggx(n_dot_v: f32, n_dot_l: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    return 0.5 / (ggx_v + ggx_l + 1e-7);
}

fn v_smith_ggx_aniso(n_dot_v: f32, n_dot_l: f32, v: vec3<f32>, l: vec3<f32>,
                     t: vec3<f32>, b: vec3<f32>, alpha_t: f32, alpha_b: f32) -> f32 {
    let tv = dot(t, v); let bv = dot(b, v);
    let tl = dot(t, l); let bl = dot(b, l);
    let ggx_v = n_dot_l * sqrt(alpha_t * alpha_t * tv * tv + alpha_b * alpha_b * bv * bv + n_dot_v * n_dot_v);
    let ggx_l = n_dot_v * sqrt(alpha_t * alpha_t * tl * tl + alpha_b * alpha_b * bl * bl + n_dot_l * n_dot_l);
    return 0.5 / (ggx_v + ggx_l + 1e-7);
}

// Kelemen visibility for clearcoat (cheap approximation)
fn v_kelemen(l_dot_h: f32) -> f32 {
    return 0.25 / (l_dot_h * l_dot_h + 1e-7);
}

// ======================================================================
// 4. Diffuse models
// ======================================================================

// Burley (Disney) diffuse
fn diffuse_burley(n_dot_v: f32, n_dot_l: f32, l_dot_h: f32, roughness: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * l_dot_h * l_dot_h;
    let light_scatter = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_l);
    let view_scatter = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_v);
    return light_scatter * view_scatter * INV_PI;
}

// Subsurface wrap lighting approximation
fn diffuse_subsurface(n_dot_l: f32, n_dot_v: f32, l_dot_h: f32,
                      roughness: f32, subsurface_scale: f32) -> f32 {
    let wrap = subsurface_scale * 0.5;
    let n_dot_l_wrap = (n_dot_l + wrap) / (1.0 + wrap);
    let f90 = roughness * l_dot_h * l_dot_h;
    let fl = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_l_wrap);
    let fv = 1.0 + (f90 - 1.0) * pow5(1.0 - n_dot_v);
    return fl * fv * INV_PI * max(n_dot_l_wrap, 0.0);
}

// ======================================================================
// 5. Full Disney BRDF evaluation
// ======================================================================

struct BRDFResult {
    diffuse:  vec3<f32>,
    specular: vec3<f32>,
};

fn evaluate_disney_brdf(
    mat:   DisneyMaterial,
    n:     vec3<f32>,    // surface normal
    v:     vec3<f32>,    // view direction
    l:     vec3<f32>,    // light direction
    t:     vec3<f32>,    // tangent (for anisotropy)
    b:     vec3<f32>,    // bitangent
) -> BRDFResult {
    let h = normalize(v + l);
    let n_dot_v = max(dot(n, v), 1e-5);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let l_dot_h = max(dot(l, h), 0.0);

    let alpha = max(mat.roughness * mat.roughness, 0.002);
    let f0 = mix(vec3<f32>(0.04), mat.base_color, mat.metallic);

    var result: BRDFResult;
    result.diffuse = vec3<f32>(0.0);
    result.specular = vec3<f32>(0.0);

    // --- Diffuse ---
    if (mat.metallic < 1.0) {
        var fd: f32;
        if (ENABLE_SUBSURFACE && has_flag(mat.flags, FLAG_SUBSURFACE) && mat.subsurface_scale > 0.0) {
            fd = diffuse_subsurface(n_dot_l, n_dot_v, l_dot_h, mat.roughness, mat.subsurface_scale);
            // Tint subsurface
            result.diffuse = mat.base_color * mix(vec3<f32>(1.0), mat.subsurface_color, mat.subsurface_scale) * fd;
        } else {
            fd = diffuse_burley(n_dot_v, n_dot_l, l_dot_h, mat.roughness);
            result.diffuse = mat.base_color * fd;
        }
        result.diffuse *= (1.0 - mat.metallic);
    }

    // --- Specular (base lobe) ---
    {
        var d: f32;
        var vis: f32;

        if (ENABLE_ANISOTROPY && has_flag(mat.flags, FLAG_ANISOTROPY) && abs(mat.anisotropy) > 0.01) {
            // Anisotropic specular
            let aspect = sqrt(1.0 - mat.anisotropy * 0.9);
            let alpha_t = alpha / aspect;
            let alpha_b = alpha * aspect;

            // Rotate tangent by anisotropy_rotation
            let cos_r = cos(mat.anisotropy_rotation * PI * 2.0);
            let sin_r = sin(mat.anisotropy_rotation * PI * 2.0);
            let rt = t * cos_r + b * sin_r;
            let rb = -t * sin_r + b * cos_r;

            d = d_ggx_aniso(n_dot_h, h, rt, rb, alpha_t, alpha_b);
            vis = v_smith_ggx_aniso(n_dot_v, n_dot_l, v, l, rt, rb, alpha_t, alpha_b);
        } else {
            d = d_ggx(n_dot_h, alpha);
            vis = v_smith_ggx(n_dot_v, n_dot_l, alpha);
        }

        let f = fresnel_schlick(l_dot_h, f0);
        result.specular = d * vis * f;
    }

    // --- Clearcoat ---
    if (ENABLE_CLEARCOAT && has_flag(mat.flags, FLAG_CLEARCOAT) && mat.clearcoat > 0.0) {
        let cc_alpha = max(mat.clearcoat_roughness * mat.clearcoat_roughness, 0.002);
        let cc_f0 = f0_from_ior(1.5); // fixed IOR for clearcoat
        let d_cc = d_ggx(n_dot_h, cc_alpha);
        let v_cc = v_kelemen(l_dot_h);
        let f_cc = fresnel_schlick_scalar(l_dot_h, cc_f0);
        result.specular += vec3<f32>(d_cc * v_cc * f_cc * mat.clearcoat);
    }

    // --- Sheen ---
    if (ENABLE_SHEEN && has_flag(mat.flags, FLAG_SHEEN) && length(mat.sheen_color) > 0.001) {
        let d_sheen = d_charlie(n_dot_h, mat.sheen_roughness);
        let f_sheen = fresnel_schlick(l_dot_h, mat.sheen_color);
        result.diffuse += d_sheen * f_sheen * (1.0 - mat.metallic);
    }

    // --- Transmission ---
    if (ENABLE_TRANSMISSION && has_flag(mat.flags, FLAG_TRANSMISSION) && mat.transmission > 0.0) {
        // Simplified: treat as modulated specular transmission
        let f_t = 1.0 - fresnel_schlick_scalar(n_dot_v, f0_from_ior(mat.ior));
        let attenuation = exp(-mat.base_color * (1.0 / max(mat.attenuation_distance, 0.001)));
        result.diffuse = mix(result.diffuse, mat.attenuation_color * attenuation, mat.transmission * f_t);
    }

    // Scale by n_dot_l (caller applies radiance and shadow)
    result.diffuse *= n_dot_l;
    result.specular *= n_dot_l;

    return result;
}

// ======================================================================
// 6. Environment (IBL) BRDF evaluation
// ======================================================================

fn evaluate_disney_ibl(
    mat:       DisneyMaterial,
    n:         vec3<f32>,
    v:         vec3<f32>,
    irradiance: vec3<f32>,    // diffuse irradiance from probe/SH
    prefiltered: vec3<f32>,   // specular prefiltered from cubemap
    brdf_lut:  vec2<f32>,     // (scale, bias) from BRDF integration LUT
) -> vec3<f32> {
    let n_dot_v = max(dot(n, v), 1e-5);
    let f0 = mix(vec3<f32>(0.04), mat.base_color, mat.metallic);

    // Diffuse IBL
    let kd = (1.0 - mat.metallic) * (1.0 - mat.transmission);
    let diffuse = irradiance * mat.base_color * kd;

    // Specular IBL (split-sum approximation)
    let f = fresnel_schlick_roughness(n_dot_v, f0, mat.roughness);
    let specular = prefiltered * (f * brdf_lut.x + brdf_lut.y);

    // Clearcoat IBL
    var cc = vec3<f32>(0.0);
    if (ENABLE_CLEARCOAT && has_flag(mat.flags, FLAG_CLEARCOAT) && mat.clearcoat > 0.0) {
        let cc_f0 = f0_from_ior(1.5);
        let cc_f = fresnel_schlick_scalar(n_dot_v, cc_f0);
        cc = prefiltered * cc_f * mat.clearcoat;
    }

    return (diffuse + specular + cc) * mat.occlusion;
}

// ======================================================================
// 7. LOD-aware Disney BRDF evaluation
// ======================================================================

// LOD-aware wrapper for evaluate_disney_brdf / evaluate_disney_ibl.
// Strips optional lobes at higher LOD tiers to save ALU on distant fragments.
//   LOD 0: full 7-lobe evaluation (clearcoat, anisotropy, sheen, subsurface, transmission)
//   LOD 1: base diffuse + specular only (clears optional lobe flags)
//   LOD 2: minimal Lambertian + Schlick specular approximation
fn evaluate_disney_brdf_lod(
    mat:   DisneyMaterial,
    n:     vec3<f32>,
    v:     vec3<f32>,
    l:     vec3<f32>,
    t:     vec3<f32>,
    b:     vec3<f32>,
    lod:   u32,
) -> BRDFResult {
    // LOD 2: minimal path — Lambertian diffuse + Schlick specular
    if (lod >= 2u) {
        let h = normalize(v + l);
        let n_dot_l = max(dot(n, l), 0.0);
        let l_dot_h = max(dot(l, h), 0.0);
        let f0 = mix(vec3<f32>(0.04), mat.base_color, mat.metallic);
        var result: BRDFResult;
        result.diffuse = mat.base_color * (1.0 - mat.metallic) * INV_PI * n_dot_l;
        result.specular = fresnel_schlick(l_dot_h, f0) * 0.25 * n_dot_l;
        return result;
    }

    // LOD 1: strip optional lobes (clearcoat, anisotropy, sheen, subsurface, transmission)
    if (lod >= 1u) {
        var stripped = mat;
        stripped.flags = 0u;
        return evaluate_disney_brdf(stripped, n, v, l, t, b);
    }

    // LOD 0: full evaluation
    return evaluate_disney_brdf(mat, n, v, l, t, b);
}
