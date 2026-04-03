// Ground Truth Ambient Occlusion (GTAO) with visibility bitmask
//
// Horizon-based AO using a visibility bitmask per direction sector instead of
// traditional min-angle tracking. This captures thin occluders and light leaking
// behind surfaces that classic HBAO misses.
//
// References:
// - "Practical Real-Time Strategies for Accurate Indirect Occlusion" (Jimenez et al.)
// - "Screen-Space Indirect Lighting with Visibility Bitmask" (2023)

struct GtaoParams {
    // Projection parameters for depth linearization
    proj_info: vec4<f32>,       // x: near*far, y: near-far, z: far, w: unused
    // AO parameters
    radius: f32,                // World-space AO radius
    falloff_start: f32,         // Distance where AO starts to fade (fraction of radius)
    falloff_end: f32,           // Distance where AO fully fades (fraction of radius)
    power: f32,                 // AO contrast exponent
    // Screen dimensions
    resolution: vec2<f32>,      // Screen width, height
    inv_resolution: vec2<f32>,  // 1/width, 1/height
    // Sampling
    num_directions: u32,        // Number of angular sectors (typically 8)
    num_steps: u32,             // Steps per direction (typically 4-8)
    frame_index: u32,           // For temporal noise rotation
    _pad: u32,
};

@group(0) @binding(0) var depth_tex: texture_depth_2d;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_sampler: sampler;
@group(0) @binding(3) var<uniform> params: GtaoParams;
@group(0) @binding(4) var ao_output: texture_storage_2d<r32float, write>;

const PI: f32 = 3.14159265;
const TWO_PI: f32 = 6.28318530;
// 32-bit visibility bitmask (32 elevation sectors per direction)
const BITMASK_BITS: u32 = 32u;

// Linearize depth from depth buffer value
fn linearize_depth(d: f32) -> f32 {
    return params.proj_info.x / (params.proj_info.z * d + params.proj_info.y);
}

// Reconstruct view-space position from UV + depth
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let linear_z = linearize_depth(depth);
    // Assumes standard perspective: x = (uv.x * 2 - 1) * z / proj_x, etc.
    let ndc = uv * 2.0 - 1.0;
    return vec3<f32>(
        ndc.x * linear_z * params.proj_info.w, // proj_info.w reused as aspect*tan(fov/2)
        ndc.y * linear_z,
        -linear_z
    );
}

// Spatial noise for temporal variation (Interleaved Gradient Noise)
fn ign_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(0.06711056 * pixel.x + 0.00583715 * pixel.y));
}

// Rotate direction by angle
fn rotate_direction(dir: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
}

@compute @workgroup_size(8, 8, 1)
fn gtao_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let dims = vec2<i32>(params.resolution);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;

    // Sample center depth and normal
    let center_depth = textureLoad(depth_tex, pixel, 0);
    if (center_depth >= 1.0) {
        // Sky pixel — no AO
        textureStore(ao_output, pixel, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    let center_pos = reconstruct_view_pos(uv, center_depth);
    // Normal from G-buffer (world space → view space would be ideal, but world works for screen-aligned AO)
    let normal_raw = textureSampleLevel(normal_tex, depth_sampler, uv, 0.0).rgb;
    let center_normal = normalize(normal_raw * 2.0 - 1.0);

    // Temporal rotation offset
    let noise_angle = ign_noise(vec2<f32>(pixel) + vec2<f32>(f32(params.frame_index) * 5.0, 0.0)) * PI;
    let noise_offset = ign_noise(vec2<f32>(pixel) * 0.7 + vec2<f32>(0.0, f32(params.frame_index) * 3.0));

    let step_radius = params.radius / f32(params.num_steps);
    var total_occlusion = 0.0;

    for (var dir_idx = 0u; dir_idx < params.num_directions; dir_idx++) {
        // Direction angle with noise rotation
        let angle = (f32(dir_idx) + noise_offset) * (TWO_PI / f32(params.num_directions)) + noise_angle;
        let dir = vec2<f32>(cos(angle), sin(angle));
        let screen_dir = dir * params.inv_resolution;

        // Visibility bitmask: 1 = visible, 0 = occluded
        var visibility: u32 = 0xFFFFFFFFu;

        for (var step = 1u; step <= params.num_steps; step++) {
            let t = (f32(step) + noise_offset * 0.5) / f32(params.num_steps);
            let sample_uv = uv + screen_dir * t * params.radius;

            // Bounds check
            if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
                continue;
            }

            let sample_pixel = vec2<i32>(sample_uv * params.resolution);
            let sample_depth = textureLoad(depth_tex, sample_pixel, 0);
            let sample_pos = reconstruct_view_pos(sample_uv, sample_depth);

            let delta = sample_pos - center_pos;
            let dist = length(delta);

            if (dist < 0.001 || dist > params.radius) {
                continue;
            }

            let delta_norm = delta / dist;

            // Elevation angle relative to the surface plane
            let elevation = dot(delta_norm, center_normal);

            // Distance falloff
            let falloff_t = clamp((dist - params.falloff_start * params.radius) /
                                   max(params.falloff_end * params.radius - params.falloff_start * params.radius, 0.001), 0.0, 1.0);
            let falloff = 1.0 - falloff_t;

            // Map elevation to bitmask sector
            // elevation in [-1, 1] → sector in [0, BITMASK_BITS-1]
            let sector = u32(clamp((elevation * 0.5 + 0.5) * f32(BITMASK_BITS), 0.0, f32(BITMASK_BITS - 1u)));

            // If the sample is above the horizon (positive elevation), mark lower sectors as occluded
            if (elevation > 0.0 && falloff > 0.0) {
                // Create a mask: all bits below `sector` are potentially occluded
                let occlude_mask = (1u << sector) - 1u;
                // Apply with falloff (probabilistic: only clear bits if falloff is strong enough)
                if (falloff > 0.5) {
                    visibility = visibility & ~occlude_mask;
                }
            }
        }

        // Count visible bits → AO contribution for this direction
        let visible_bits = countOneBits(visibility);
        total_occlusion += f32(visible_bits) / f32(BITMASK_BITS);
    }

    // Average over all directions
    var ao = total_occlusion / f32(params.num_directions);
    ao = pow(ao, params.power);
    ao = clamp(ao, 0.0, 1.0);

    textureStore(ao_output, pixel, vec4<f32>(ao, 0.0, 0.0, 0.0));
}
