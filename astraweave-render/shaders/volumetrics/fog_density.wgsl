// Volumetric Fog — Froxel Density Generation
//
// Writes per-froxel fog density into a 3D RGBA16Float texture.
// R = density, G = unused, B = unused, A = unused.
//
// Density sources:
//   - Uniform base density (constant fog everywhere)
//   - Height-based exponential falloff (ground fog / atmospheric)
//   - 3D noise (animated, turbulent detail)

struct FogDensityParams {
    // Camera / projection
    inv_view_proj:       mat4x4<f32>,
    view_pos:            vec3<f32>,
    near_plane:          f32,
    far_plane:           f32,
    // Froxel grid
    froxel_dims:         vec3<u32>,
    // Density controls
    base_density:        f32,
    height_fog_density:  f32,
    height_fog_falloff:  f32,  // exponential rate
    height_fog_offset:   f32,  // sea-level offset
    noise_scale:         f32,
    noise_intensity:     f32,
    noise_speed:         f32,
    time:                f32,
    // Wind
    wind_dir:            vec3<f32>,
    _pad:                f32,
};

@group(0) @binding(0) var<uniform>          params:     FogDensityParams;
@group(0) @binding(1) var                   t_output:   texture_storage_3d<rgba16float, write>;

// ---- Noise functions ----

// Hash function for pseudo-random values
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// Value noise with smooth interpolation
fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Smooth Hermite interpolation
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash31(i + vec3<f32>(0.0, 0.0, 0.0)),
                hash31(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 0.0)),
                hash31(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y),
        mix(
            mix(hash31(i + vec3<f32>(0.0, 0.0, 1.0)),
                hash31(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 1.0)),
                hash31(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y),
        u.z);
}

// Fractal Brownian Motion (3 octaves)
fn fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i++) {
        value += amplitude * value_noise(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// ---- Froxel world position ----

fn froxel_to_world(froxel: vec3<u32>) -> vec3<f32> {
    let dims = vec3<f32>(params.froxel_dims);
    // Normalized [0,1] position in the froxel grid
    let uvw = (vec3<f32>(froxel) + 0.5) / dims;

    // Screen UV from X/Y
    let screen_uv = uvw.xy;

    // Exponential depth distribution for better near-plane resolution
    let linear_t = uvw.z;
    let z_near = params.near_plane;
    let z_far = params.far_plane;
    // Exponential: more slices near camera, fewer far away
    let depth = z_near * pow(z_far / z_near, linear_t);

    // Unproject screen UV + depth to world space
    let ndc_xy = screen_uv * 2.0 - 1.0;
    // Convert linear depth to NDC depth (reverse-Z or standard)
    let ndc_z = (z_far * (depth - z_near)) / (depth * (z_far - z_near));

    let clip = vec4<f32>(ndc_xy, ndc_z, 1.0);
    let world_h = params.inv_view_proj * clip;
    return world_h.xyz / world_h.w;
}

override WG_X: u32 = 4u;
override WG_Y: u32 = 4u;
override WG_Z: u32 = 4u;

@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn fog_density_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = params.froxel_dims;
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }

    let world_pos = froxel_to_world(gid);

    // --- Base uniform density ---
    var density = params.base_density;

    // --- Height fog (exponential falloff) ---
    let height = world_pos.y - params.height_fog_offset;
    let height_fog = params.height_fog_density * exp(-max(height, 0.0) * params.height_fog_falloff);
    density += height_fog;

    // --- Animated noise ---
    let wind_offset = params.wind_dir * params.time * params.noise_speed;
    let noise_pos = world_pos * params.noise_scale + wind_offset;
    let noise = fbm(noise_pos);
    density += (noise - 0.5) * 2.0 * params.noise_intensity;

    // Clamp to non-negative
    density = max(density, 0.0);

    textureStore(t_output, vec3<i32>(gid), vec4<f32>(density, 0.0, 0.0, 0.0));
}
