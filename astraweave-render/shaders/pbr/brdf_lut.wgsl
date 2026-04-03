// BRDF Integration LUT — Split-Sum Approximation for IBL
//
// Precomputes the BRDF integral for specular IBL using the split-sum method
// (Karis 2013). The LUT is parameterized by (NdotV, roughness) and stores
// (scale, bias) for the Fresnel approximation:
//   specular = prefiltered * (F0 * scale + bias)
//
// Only needs to be computed once (resolution-independent).

struct BrdfLutParams {
    lut_size:    u32,
    num_samples: u32,
    _pad0:       u32,
    _pad1:       u32,
};

@group(0) @binding(0) var<uniform>  params:   BrdfLutParams;
@group(0) @binding(1) var           t_output: texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265358979;

// Hammersley quasi-random sequence
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

// Importance sample GGX
fn importance_sample_ggx(xi: vec2<f32>, roughness: f32) -> vec3<f32> {
    let a = roughness * roughness;
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    return vec3<f32>(
        cos(phi) * sin_theta,
        sin(phi) * sin_theta,
        cos_theta,
    );
}

// Smith G1 (GGX)
fn geometry_schlick_ggx(n_dot_x: f32, roughness: f32) -> f32 {
    let k = (roughness * roughness) / 2.0;
    return n_dot_x / (n_dot_x * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

@compute @workgroup_size(8, 8)
fn brdf_lut_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.lut_size || gid.y >= params.lut_size) {
        return;
    }

    let n_dot_v = max((f32(gid.x) + 0.5) / f32(params.lut_size), 0.001);
    let roughness = max((f32(gid.y) + 0.5) / f32(params.lut_size), 0.001);

    // View direction in tangent space (N = (0,0,1))
    let v = vec3<f32>(sqrt(1.0 - n_dot_v * n_dot_v), 0.0, n_dot_v);
    let n = vec3<f32>(0.0, 0.0, 1.0);

    var scale = 0.0;
    var bias = 0.0;

    for (var i = 0u; i < params.num_samples; i++) {
        let xi = hammersley(i, params.num_samples);
        let h = importance_sample_ggx(xi, roughness);
        let l = normalize(2.0 * dot(v, h) * h - v);

        let n_dot_l = max(l.z, 0.0);
        let n_dot_h = max(h.z, 0.0);
        let v_dot_h = max(dot(v, h), 0.0);

        if (n_dot_l > 0.0) {
            let g = geometry_smith(n_dot_v, n_dot_l, roughness);
            let g_vis = (g * v_dot_h) / (n_dot_h * n_dot_v + 1e-7);
            let fc = pow(1.0 - v_dot_h, 5.0);

            scale += (1.0 - fc) * g_vis;
            bias += fc * g_vis;
        }
    }

    scale /= f32(params.num_samples);
    bias /= f32(params.num_samples);

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(scale, bias, 0.0, 0.0));
}
