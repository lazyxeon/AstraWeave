// BRDF Integration LUT — Split-Sum Approximation
//
// Generates a 2D look-up table storing (Fresnel scale, Fresnel bias) for the
// split-sum approximation of specular IBL. Indexed by (NdotV, roughness).
// Output format: RG16Float (R=scale, G=bias).
// Reference: Epic Games, "Real Shading in Unreal Engine 4", 2013.

const PI: f32 = 3.14159265359;

struct Params {
    size: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;

// Radical inverse (Van der Corput sequence) for Hammersley QMC
fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(n), radical_inverse_vdc(i));
}

// GGX importance sampling: generates a microfacet half-vector
fn importance_sample_ggx(xi: vec2<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

// Height-correlated Smith-GGX visibility (matches entity.wgsl analytical path)
fn v_smith_ggx_correlated(n_dot_v: f32, n_dot_l: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    return 0.5 / (ggx_v + ggx_l + 0.0001);
}

@compute @workgroup_size(8, 8, 1)
fn cs_brdf_lut(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = params.size;
    if gid.x >= size || gid.y >= size {
        return;
    }

    // Map texel coords → (NdotV, roughness)
    let n_dot_v = max(f32(gid.x) / f32(size - 1u), 0.001);
    let roughness = max(f32(gid.y) / f32(size - 1u), 0.04);

    let n = vec3<f32>(0.0, 0.0, 1.0);
    let v = vec3<f32>(sqrt(1.0 - n_dot_v * n_dot_v), 0.0, n_dot_v);

    var scale = 0.0;
    var bias = 0.0;
    let sample_count = 256u;

    for (var i = 0u; i < sample_count; i++) {
        let xi = hammersley(i, sample_count);
        let h = importance_sample_ggx(xi, roughness);
        let l = normalize(2.0 * dot(v, h) * h - v);

        let n_dot_l = max(l.z, 0.0);
        let n_dot_h = max(h.z, 0.0);
        let v_dot_h = max(dot(v, h), 0.0);

        if n_dot_l > 0.0 {
            let alpha = max(roughness * roughness, 0.002);
            let vis = v_smith_ggx_correlated(n_dot_v, n_dot_l, alpha);
            // V already includes 1/(4*NdotV*NdotL); recover G_vis for integration:
            // G_vis = V * 4 * NdotL * VdotH / NdotH
            let g_vis = vis * 4.0 * n_dot_l * v_dot_h / (n_dot_h + 0.0001);
            let fc = pow(1.0 - v_dot_h, 5.0);
            scale += (1.0 - fc) * g_vis;
            bias += fc * g_vis;
        }
    }

    scale /= f32(sample_count);
    bias /= f32(sample_count);

    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(scale, bias, 0.0, 1.0));
}
