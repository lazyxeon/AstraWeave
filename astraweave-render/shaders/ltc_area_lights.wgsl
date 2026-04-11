// LTC (Linearly Transformed Cosines) Area Light Evaluation
//
// Implements Heitz et al. 2016: "Real-Time Polygonal-Light Shading with
// Linearly Transformed Cosines"
//
// Two LUT textures (64×64):
//   1. ltc_matrix: RGBA32Float — inverse M matrix coefficients [a, 0, b; 0, c, 0; d, 0, 1]
//      packed as (a, b, c, d) in RGBA.
//   2. ltc_amplitude: RG16Float — (magnitude, fresnel) for energy normalization.
//
// Area light types: rectangular (2 half-extents), disk (radius), tube (length + radius).

struct AreaLight {
    position: vec3<f32>,    // center of the area light
    light_type: u32,        // 0 = rect, 1 = disk, 2 = tube
    right: vec3<f32>,       // half-extent in X (scaled by width/2)
    width: f32,             // full width
    up: vec3<f32>,          // half-extent in Y (scaled by height/2)
    height: f32,            // full height
    color: vec3<f32>,       // linear RGB
    intensity: f32,
};

struct AreaLightParams {
    num_area_lights: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

// Bind group for area lights (appended to existing lighting groups)
@group(6) @binding(0) var<storage, read> area_lights: array<AreaLight>;
@group(6) @binding(1) var<uniform> area_params: AreaLightParams;
@group(6) @binding(2) var ltc_matrix_tex: texture_2d<f32>;
@group(6) @binding(3) var ltc_amplitude_tex: texture_2d<f32>;
@group(6) @binding(4) var ltc_sampler: sampler;

// Camera UBO (same as main PBR pass)
struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _pad: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// ─── LTC Matrix Reconstruction ───
// The LUT encodes the inverse of the linear transform M that maps
// the clamped cosine distribution to the BRDF lobe.
// LUT coords: (roughness, cos_theta) → [0,1]² → texel (64×64)
fn ltc_coords(roughness: f32, cos_theta: f32) -> vec2<f32> {
    // Roughness on X, cos_theta on Y, both clamped to [0,1]
    let r = clamp(roughness, 0.0, 1.0);
    let ct = clamp(cos_theta, 0.0, 1.0);
    // Bias to texel centers for 64×64 LUT
    let scale = (64.0 - 1.0) / 64.0;
    let bias = 0.5 / 64.0;
    return vec2<f32>(r * scale + bias, ct * scale + bias);
}

fn ltc_sample_matrix(uv: vec2<f32>) -> mat3x3<f32> {
    let t = textureSampleLevel(ltc_matrix_tex, ltc_sampler, uv, 0.0);
    // Reconstruct 3×3 matrix from packed (a, b, c, d)
    // M^-1 = [a, 0, b; 0, c, 0; d, 0, 1]
    return mat3x3<f32>(
        vec3<f32>(t.x, 0.0, t.y),
        vec3<f32>(0.0, t.z, 0.0),
        vec3<f32>(t.w, 0.0, 1.0)
    );
}

fn ltc_sample_amplitude(uv: vec2<f32>) -> vec2<f32> {
    let t = textureSampleLevel(ltc_amplitude_tex, ltc_sampler, uv, 0.0);
    return t.xy; // (magnitude, fresnel term)
}

// ─── Polygon Clipping to Hemisphere ───
// Clip quad polygon to upper hemisphere (z > 0) before integration.
// Returns clipped vertex count (3-5) and writes vertices to out_poly.
fn clip_quad_to_hemisphere(
    v0: vec3<f32>, v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>,
    out_poly: ptr<function, array<vec3<f32>, 5>>,
) -> u32 {
    // Simple clip: for each edge, if it crosses z=0, insert intersection point.
    // For efficiency, we handle the common case (no clipping needed) first.
    var vertices = array<vec3<f32>, 4>(v0, v1, v2, v3);
    var above = array<bool, 4>(v0.z > 0.0, v1.z > 0.0, v2.z > 0.0, v3.z > 0.0);

    var all_above = true;
    var all_below = true;
    for (var i = 0u; i < 4u; i++) {
        if (above[i]) { all_below = false; } else { all_above = false; }
    }

    if (all_below) {
        return 0u;
    }

    if (all_above) {
        (*out_poly)[0] = v0;
        (*out_poly)[1] = v1;
        (*out_poly)[2] = v2;
        (*out_poly)[3] = v3;
        return 4u;
    }

    // General clip: iterate edges, emit vertices
    var count = 0u;
    for (var i = 0u; i < 4u; i++) {
        let j = (i + 1u) % 4u;
        let a = vertices[i];
        let b = vertices[j];

        if (above[i]) {
            (*out_poly)[count] = a;
            count++;
        }

        // If edge crosses z=0, add intersection
        if (above[i] != above[j]) {
            let t = a.z / (a.z - b.z);
            let intersect = a + t * (b - a);
            (*out_poly)[count] = intersect;
            count++;
        }

        if (count >= 5u) {
            break;
        }
    }

    return min(count, 5u);
}

// ─── Integrate Clipped Polygon ───
// Analytic irradiance integration of a polygon on the upper hemisphere.
// Uses the edge-integral formula from Heitz et al. 2016 / Baum et al. 1989.
fn integrate_edge(v0: vec3<f32>, v1: vec3<f32>) -> f32 {
    let cos_theta = dot(v0, v1);
    let cos_theta_clamped = clamp(cos_theta, -0.9999, 0.9999);
    let theta = acos(cos_theta_clamped);
    let cross_z = v0.x * v1.y - v0.y * v1.x; // z-component of cross product

    // Edge integral: theta * cross(v0, v1).z / sin(theta)
    // Use safe division to avoid NaN when theta ≈ 0
    let sin_theta = sin(theta);
    if (abs(sin_theta) < 1e-6) {
        return 0.0;
    }
    return cross_z * theta / sin_theta;
}

fn ltc_integrate_polygon(poly: ptr<function, array<vec3<f32>, 5>>, count: u32) -> f32 {
    if (count < 3u) {
        return 0.0;
    }

    var irradiance = 0.0;
    for (var i = 0u; i < count; i++) {
        let j = (i + 1u) % count;
        let a = normalize((*poly)[i]);
        let b = normalize((*poly)[j]);
        irradiance += integrate_edge(a, b);
    }

    // The result is the solid angle subtended, divided by 2π
    return max(irradiance, 0.0) / TWO_PI;
}

// ─── LTC Area Light Evaluation ───
// Evaluates a single rectangular area light using LTC.
fn evaluate_area_light(
    light: AreaLight,
    world_pos: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    roughness: f32,
    metallic: f32,
    albedo: vec3<f32>,
) -> vec3<f32> {
    let cos_theta = clamp(dot(N, V), 0.0, 1.0);
    let uv = ltc_coords(roughness, cos_theta);

    let M_inv = ltc_sample_matrix(uv);
    let amp = ltc_sample_amplitude(uv);

    // Build local TBN frame
    let T1 = normalize(V - N * dot(V, N));
    let T2 = cross(N, T1);
    let TBN = mat3x3<f32>(T1, T2, N);

    // Transform quad vertices to local space
    let delta = light.position - world_pos;
    let right = light.right;
    let up = light.up;

    // Quad corners in world space (CCW winding)
    let p0 = delta - right - up;
    let p1 = delta + right - up;
    let p2 = delta + right + up;
    let p3 = delta - right + up;

    // Transform to tangent space
    let TBN_t = transpose(TBN);

    // ── SPECULAR contribution ──
    // Apply LTC inverse transform to quad vertices
    let M_spec = M_inv;
    let spec0 = M_spec * (TBN_t * p0);
    let spec1 = M_spec * (TBN_t * p1);
    let spec2 = M_spec * (TBN_t * p2);
    let spec3 = M_spec * (TBN_t * p3);

    // Clip to hemisphere and integrate
    var spec_poly: array<vec3<f32>, 5>;
    let spec_count = clip_quad_to_hemisphere(spec0, spec1, spec2, spec3, &spec_poly);
    let spec_irradiance = ltc_integrate_polygon(&spec_poly, spec_count);

    // ── DIFFUSE contribution ──
    // Diffuse uses identity transform (clamped cosine = no LTC warp)
    let diff0 = TBN_t * p0;
    let diff1 = TBN_t * p1;
    let diff2 = TBN_t * p2;
    let diff3 = TBN_t * p3;

    var diff_poly: array<vec3<f32>, 5>;
    let diff_count = clip_quad_to_hemisphere(diff0, diff1, diff2, diff3, &diff_poly);
    let diff_irradiance = ltc_integrate_polygon(&diff_poly, diff_count);

    // ── Combine with Fresnel ──
    let F0 = mix(vec3<f32>(0.04), albedo, metallic);
    let fresnel = F0 * amp.x + (1.0 - F0) * amp.y;

    let diffuse_color = albedo * (1.0 - metallic);
    let specular = fresnel * spec_irradiance;
    let diffuse = diffuse_color * diff_irradiance;

    return (specular + diffuse) * light.color * light.intensity;
}

// ─── Main Area Lighting Function ───
// Call this from the main fragment shader after point/directional lighting.
fn calculate_area_lighting(
    world_pos: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
) -> vec3<f32> {
    var total = vec3<f32>(0.0);

    for (var i = 0u; i < area_params.num_area_lights; i++) {
        let light = area_lights[i];

        // Distance-based early out
        let delta = light.position - world_pos;
        let dist = length(delta);
        let max_range = max(light.width, light.height) * 5.0;
        if (dist > max_range) {
            continue;
        }

        total += evaluate_area_light(light, world_pos, N, V, roughness, metallic, albedo);
    }

    return total;
}
