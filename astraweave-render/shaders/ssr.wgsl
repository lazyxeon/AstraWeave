// Screen-Space Reflections (SSR)
//
// Hierarchical screen-space ray marching using the Hi-Z min-depth pyramid.
// Uses mip-chain traversal for O(log N) convergence vs O(N) linear march.
// Produces reflection color + confidence mask.
//
// Algorithm: Start at a coarse mip level. If the ray depth is less than the
// min-depth stored in that cell, the entire cell is empty — skip it and try
// an even coarser level. If the ray depth exceeds the min-depth, refine by
// stepping down to a finer mip. At mip 0, perform a thickness check for the
// final hit determination.

struct SsrParams {
    inv_proj: mat4x4<f32>,
    proj: mat4x4<f32>,
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    max_distance: f32,
    stride: f32,              // Base stride scale (pixels)
    max_steps: u32,
    thickness: f32,
    fade_start: f32,          // Screen edge fade start (0..1 from edge)
    fade_end: f32,            // Screen edge fade end
    roughness_cutoff: f32,    // Don't trace for roughness above this
    temporal_blend: f32,      // Base history blend amount
    frame_index: u32,
    hiz_mip_count: u32,       // Number of mip levels in the Hi-Z pyramid
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var hiz_tex: texture_2d<f32>;      // Hi-Z min-depth pyramid (mip 0 = full res)
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var color_tex: texture_2d<f32>;
@group(0) @binding(3) var mr_tex: texture_2d<f32>;
@group(0) @binding(4) var velocity_tex: texture_2d<f32>;
@group(0) @binding(5) var history_tex: texture_2d<f32>;
@group(0) @binding(6) var samp: sampler;
@group(0) @binding(7) var<uniform> params: SsrParams;
@group(0) @binding(8) var ssr_output: texture_storage_2d<rgba16float, write>;

// Reconstruct view-space position from texture UV and depth
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let view_pos = params.inv_proj * ndc;
    return view_pos.xyz / view_pos.w;
}

// Project view-space position to NDC-based UV [0,1] + depth
fn project_to_screen(view_pos: vec3<f32>) -> vec3<f32> {
    let clip = params.proj * vec4<f32>(view_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.xy * 0.5 + 0.5, ndc.z);
}

// Convert NDC-based UV to texture UV (flip Y for texture sampling)
fn ndc_to_tex_uv(ndc_uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(ndc_uv.x, 1.0 - ndc_uv.y);
}

// Sample the Hi-Z min-depth pyramid at a given NDC-UV and mip level
fn sample_hiz(ndc_uv: vec2<f32>, level: i32) -> f32 {
    let tex_uv = ndc_to_tex_uv(ndc_uv);
    let mip_res = vec2<f32>(textureDimensions(hiz_tex, level));
    let texel = clamp(vec2<i32>(tex_uv * mip_res), vec2<i32>(0), vec2<i32>(mip_res) - 1);
    return textureLoad(hiz_tex, texel, level).r;
}

// Screen edge fade
fn screen_edge_fade(uv: vec2<f32>) -> f32 {
    let edge_dist = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return smoothstep(params.fade_end, params.fade_start, edge_dist);
}

// Interleaved Gradient Noise for temporal jitter
fn ign(pixel: vec2<f32>, frame: f32) -> f32 {
    return fract(52.9829189 * fract(0.06711056 * pixel.x + 0.00583715 * pixel.y + frame * 0.17));
}

// Hierarchical ray march through the Hi-Z min-depth pyramid.
// Returns vec4(ndc_uv.xy, depth, 1.0) on hit, or vec4(0) on miss.
fn hiz_trace(origin_vs: vec3<f32>, dir_vs: vec3<f32>, jitter: f32) -> vec4<f32> {
    // Project ray start and far point to screen space
    let p0 = project_to_screen(origin_vs + dir_vs * 0.01); // offset to avoid self-hit
    let p1 = project_to_screen(origin_vs + dir_vs * params.max_distance);

    // Screen-space ray delta
    let delta = p1 - p0;

    // Measure extent in pixels to normalize step size
    let pixel_len = length(delta.xy * params.resolution);
    if (pixel_len < 0.5) {
        return vec4<f32>(0.0); // Ray nearly perpendicular to screen
    }

    // Normalize so each unit step ≈ 1 pixel at mip 0
    let ray = delta / pixel_len;

    var pos = p0 + ray * jitter;
    var level = 2; // Start at mip 2 (4×4 pixel blocks)
    let max_level = min(i32(params.hiz_mip_count) - 1, 6);

    for (var i = 0u; i < params.max_steps; i++) {
        // Bounds check (NDC UV space)
        if (pos.x < 0.0 || pos.x > 1.0 || pos.y < 0.0 || pos.y > 1.0) {
            break;
        }
        if (pos.z < 0.0 || pos.z > 1.0) {
            break;
        }

        let z_min = sample_hiz(pos.xy, level);

        if (pos.z > z_min) {
            // Ray depth > min surface depth → behind closest surface
            if (level <= 0) {
                // Finest level: check thickness for final hit
                let thickness = pos.z - z_min;
                if (thickness < params.thickness) {
                    return vec4<f32>(pos.xy, z_min, 1.0);
                }
                // Passed through (too thick or backface) → advance 1 pixel
                pos += ray;
            } else {
                // Refine at finer mip level
                level -= 1;
            }
        } else {
            // Ray in front of all geometry in this cell → skip it
            // Advance by cell size at current mip level (2^level pixels)
            let step = exp2(f32(level));
            pos += ray * step;

            // Try a coarser level for faster traversal
            level = min(level + 1, max_level);
        }
    }

    return vec4<f32>(0.0); // Miss
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
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

    // Read depth from Hi-Z mip 0 (= full-resolution depth)
    let depth = sample_hiz(uv, 0);
    if (depth >= 1.0) {
        textureStore(ssr_output, pixel, vec4<f32>(0.0));
        return;
    }

    let view_pos = reconstruct_view_pos(uv, depth);
    let normal_raw = textureSampleLevel(normal_tex, samp, uv, 0.0).rgb;
    let N = normalize(normal_raw * 2.0 - 1.0);
    let V = normalize(-view_pos);

    // Reflection direction in view space
    let R = reflect(-V, N);

    let jitter = ign(vec2<f32>(pixel), f32(params.frame_index));

    // Hierarchical ray march
    let hit = hiz_trace(view_pos, R, jitter);
    let velocity = textureSampleLevel(velocity_tex, samp, uv, 0.0).rg;
    let history_uv = uv - velocity;

    var current_reflection = vec4<f32>(0.0);

    if (hit.w > 0.0) {
        // Convert hit NDC-UV to texture UV for color sampling
        let hit_uv = ndc_to_tex_uv(hit.xy);
        let hit_color = textureSampleLevel(color_tex, samp, hit_uv, 0.0).rgb;

        let edge_fade = screen_edge_fade(hit_uv);
        let rough_fade = 1.0 - roughness / params.roughness_cutoff;
        let confidence = edge_fade * rough_fade;

        current_reflection = vec4<f32>(hit_color * confidence, confidence);
    }

    var result = current_reflection;
    if (history_uv.x >= 0.0 && history_uv.x <= 1.0 && history_uv.y >= 0.0 && history_uv.y <= 1.0) {
        let history = textureSampleLevel(history_tex, samp, history_uv, 0.0);

        // More motion means less history to reduce ghosting.
        let motion_px = length(velocity * params.resolution);
        let history_weight = params.temporal_blend * (1.0 - clamp(motion_px * 0.05, 0.0, 1.0));
        result = mix(current_reflection, history, history_weight);
    }

    textureStore(ssr_output, pixel, vec4<f32>(max(result.rgb, vec3<f32>(0.0)), clamp(result.a, 0.0, 1.0)));
}
