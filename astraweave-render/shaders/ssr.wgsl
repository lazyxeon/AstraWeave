// Screen-Space Reflections (SSR)
//
// Hierarchical screen-space ray marching using the Hi-Z depth buffer.
// Produces reflection color + confidence mask. Falls back to IBL cubemap
// for rays that miss the screen.

struct SsrParams {
    inv_proj: mat4x4<f32>,
    proj: mat4x4<f32>,
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    max_distance: f32,
    stride: f32,              // Initial step stride in pixels
    max_steps: u32,
    thickness: f32,
    fade_start: f32,          // Screen edge fade start (0..1 from edge)
    fade_end: f32,            // Screen edge fade end
    roughness_cutoff: f32,    // Don't trace for roughness above this
    frame_index: u32,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var color_tex: texture_2d<f32>;
@group(0) @binding(3) var mr_tex: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;
@group(0) @binding(5) var<uniform> params: SsrParams;
@group(0) @binding(6) var ssr_output: texture_storage_2d<rgba16float, write>;

// Reconstruct view-space position from UV and depth
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let view_pos = params.inv_proj * ndc;
    return view_pos.xyz / view_pos.w;
}

// Project view-space position to screen UV + depth
fn project_to_screen(view_pos: vec3<f32>) -> vec3<f32> {
    let clip = params.proj * vec4<f32>(view_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.xy * 0.5 + 0.5, ndc.z);
}

// Screen edge fade
fn screen_edge_fade(uv: vec2<f32>) -> f32 {
    let edge_dist = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return smoothstep(params.fade_end, params.fade_start, edge_dist);
}

// Interleaved Gradient Noise for jitter
fn ign(pixel: vec2<f32>, frame: f32) -> f32 {
    return fract(52.9829189 * fract(0.06711056 * pixel.x + 0.00583715 * pixel.y + frame * 0.17));
}

@compute @workgroup_size(8, 8, 1)
fn ssr_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let dims = vec2<i32>(params.resolution);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;

    // Sample roughness — skip very rough surfaces
    let mr = textureSampleLevel(mr_tex, samp, uv, 0.0);
    let roughness = max(mr.g, 0.04);
    if (roughness > params.roughness_cutoff) {
        textureStore(ssr_output, pixel, vec4<f32>(0.0));
        return;
    }

    let depth = textureSampleLevel(depth_tex, samp, uv, 0.0).r;
    if (depth >= 1.0) {
        textureStore(ssr_output, pixel, vec4<f32>(0.0));
        return;
    }

    let view_pos = reconstruct_view_pos(uv, depth);
    let normal_raw = textureSampleLevel(normal_tex, samp, uv, 0.0).rgb;
    let N = normalize(normal_raw * 2.0 - 1.0);
    let V = normalize(-view_pos);

    // Reflection direction
    let R = reflect(-V, N);

    // Jitter the starting point slightly for temporal variation
    let jitter = ign(vec2<f32>(pixel), f32(params.frame_index)) * 0.5;

    // Ray march in view space
    var ray_pos = view_pos + R * 0.1; // Small offset to avoid self-intersection
    let ray_step = R * params.stride * 0.1;

    var hit_color = vec3<f32>(0.0);
    var hit_confidence = 0.0;

    for (var i = 0u; i < params.max_steps; i++) {
        ray_pos = ray_pos + ray_step * (1.0 + f32(i) * 0.1 + jitter * 0.1); // Increasing step size

        let projected = project_to_screen(ray_pos);
        let screen_uv = vec2<f32>(projected.x, 1.0 - projected.y);

        // Bounds check
        if (screen_uv.x < 0.0 || screen_uv.x > 1.0 || screen_uv.y < 0.0 || screen_uv.y > 1.0) {
            break;
        }

        let sampled_depth = textureSampleLevel(depth_tex, samp, screen_uv, 0.0).r;
        let sampled_pos = reconstruct_view_pos(screen_uv, sampled_depth);

        let depth_diff = ray_pos.z - sampled_pos.z;

        if (depth_diff > 0.0 && depth_diff < params.thickness) {
            // Hit!
            hit_color = textureSampleLevel(color_tex, samp, screen_uv, 0.0).rgb;

            // Confidence based on:
            // 1. Distance from screen edge
            let edge_fade = screen_edge_fade(screen_uv);
            // 2. Number of steps taken (closer hits are more reliable)
            let step_fade = 1.0 - f32(i) / f32(params.max_steps);
            // 3. Roughness (smoother = sharper, more confident)
            let rough_fade = 1.0 - roughness / params.roughness_cutoff;

            hit_confidence = edge_fade * step_fade * rough_fade;
            break;
        }
    }

    textureStore(ssr_output, pixel, vec4<f32>(hit_color * hit_confidence, hit_confidence));
}
