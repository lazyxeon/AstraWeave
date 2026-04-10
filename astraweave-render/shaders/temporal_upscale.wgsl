// Temporal Upsampling (TAA-U) — Custom Temporal Upscaler
//
// Extends TAA with upscaling: renders at reduced internal resolution,
// accumulates sub-pixel detail at native display resolution over time.
// Each frame's jittered sub-pixel offset samples different positions,
// building up native-resolution detail in the history buffer.
//
// Pipeline: reduced-res render → upscale resolve → RCAS sharpen → output
//
// Key differences from standard TAA:
//   - Current frame is at lower resolution than output
//   - Neighborhood AABB is sampled at input resolution texel spacing
//   - History is at native resolution (accumulated from previous frames)
//   - Catmull-Rom history sampling at native texel size

struct UpscaleParams {
    output_resolution:     vec2<f32>,
    output_inv_resolution: vec2<f32>,
    input_resolution:      vec2<f32>,
    input_inv_resolution:  vec2<f32>,
    // x: blend_factor, y: clamp_margin, z: sharpen_strength, w: frame_index (for anti-banding)
    config:                vec4<f32>,
};

@group(0) @binding(0) var t_current:   texture_2d<f32>;     // reduced-res current frame
@group(0) @binding(1) var t_history:   texture_2d<f32>;     // native-res history
@group(0) @binding(2) var t_velocity:  texture_2d<f32>;     // motion vectors (at input res)
@group(0) @binding(3) var t_depth:     texture_2d<f32>;     // depth buffer (at input res)
@group(0) @binding(4) var samp:        sampler;
@group(0) @binding(5) var<uniform>     params: UpscaleParams;
@group(0) @binding(6) var t_output:    texture_storage_2d<rgba16float, write>;

// ============================================================================
// COLOR SPACE CONVERSION (YCoCg for tighter clamping)
// ============================================================================

fn rgb_to_ycocg(rgb: vec3<f32>) -> vec3<f32> {
    let y  = dot(rgb, vec3<f32>(0.25, 0.5, 0.25));
    let co = dot(rgb, vec3<f32>(0.5, 0.0, -0.5));
    let cg = dot(rgb, vec3<f32>(-0.25, 0.5, -0.25));
    return vec3<f32>(y, co, cg);
}

fn ycocg_to_rgb(ycocg: vec3<f32>) -> vec3<f32> {
    let y = ycocg.x;
    let co = ycocg.y;
    let cg = ycocg.z;
    return vec3<f32>(y + co - cg, y + cg, y - co - cg);
}

// ============================================================================
// CATMULL-ROM 5-TAP (sharp history sampling at native resolution)
// ============================================================================

fn sample_catmull_rom(tex: texture_2d<f32>, s: sampler, uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / res;
    let pos = uv * res - 0.5;
    let f = fract(pos);
    let p = floor(pos);

    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);

    let w12 = w1 + w2;
    let tc12 = (p + 1.0 + w2 / w12) * texel;
    let tc0 = (p - 0.5) * texel;
    let tc3 = (p + 2.5) * texel;

    var color = vec3<f32>(0.0);
    color += textureSampleLevel(tex, s, vec2<f32>(tc12.x, tc0.y), 0.0).rgb * (w12.x * w0.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc0.x, tc12.y), 0.0).rgb * (w0.x * w12.y);
    color += textureSampleLevel(tex, s, tc12, 0.0).rgb * (w12.x * w12.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc3.x, tc12.y), 0.0).rgb * (w3.x * w12.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc12.x, tc3.y), 0.0).rgb * (w12.x * w3.y);

    return max(color, vec3<f32>(0.0));
}

// ============================================================================
// NEIGHBORHOOD CLAMPING (sampled at input resolution texel spacing)
// ============================================================================

// Compute 3×3 AABB from current frame samples.
// Uses input_inv_resolution for texel spacing since the current frame is at input res.
fn compute_neighborhood_aabb(uv: vec2<f32>) -> array<vec3<f32>, 2> {
    let texel = params.input_inv_resolution;
    var aabb_min = vec3<f32>(1e10);
    var aabb_max = vec3<f32>(-1e10);

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let sample_uv = uv + vec2<f32>(f32(dx), f32(dy)) * texel;
            let rgb = textureSampleLevel(t_current, samp, sample_uv, 0.0).rgb;
            let ycocg = rgb_to_ycocg(rgb);
            aabb_min = min(aabb_min, ycocg);
            aabb_max = max(aabb_max, ycocg);
        }
    }

    let margin = vec3<f32>(params.config.y);
    return array<vec3<f32>, 2>(aabb_min - margin, aabb_max + margin);
}

// Clip color to AABB (ray-box intersection toward center for smoother results)
fn clip_to_aabb(color: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec3<f32> {
    let center = (aabb_min + aabb_max) * 0.5;
    let half_extent = (aabb_max - aabb_min) * 0.5 + 0.001;
    let offset = color - center;
    let unit = offset / half_extent;
    let max_unit = max(abs(unit.x), max(abs(unit.y), abs(unit.z)));
    if (max_unit > 1.0) {
        return center + offset / max_unit;
    }
    return color;
}

// ============================================================================
// CLOSEST DEPTH DILATION
// ============================================================================

// Find the velocity of the nearest surface in a 3×3 neighborhood.
// This prevents thin objects from losing their motion vectors.
fn find_closest_velocity(uv: vec2<f32>) -> vec2<f32> {
    let texel = params.input_inv_resolution;
    var closest_depth = 1.0;
    var best_uv = uv;

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let sample_uv = uv + vec2<f32>(f32(dx), f32(dy)) * texel;
            let d = textureSampleLevel(t_depth, samp, sample_uv, 0.0).r;
            if (d < closest_depth) {
                closest_depth = d;
                best_uv = sample_uv;
            }
        }
    }

    return textureSampleLevel(t_velocity, samp, best_uv, 0.0).rg;
}

// ============================================================================
// TEMPORAL UPSCALE RESOLVE
// ============================================================================

@compute @workgroup_size(8, 8, 1)
fn temporal_upscale_resolve(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(u32(params.output_resolution.x), u32(params.output_resolution.y));
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    // Output UV (native resolution)
    let output_uv = (vec2<f32>(gid.xy) + 0.5) * params.output_inv_resolution;

    // Sample current frame at output UV — hardware bilinear upscale from input res
    let current_rgb = textureSampleLevel(t_current, samp, output_uv, 0.0).rgb;

    // Find velocity of nearest surface (sampled at input res spacing)
    let velocity = find_closest_velocity(output_uv);

    // Reproject to history UV
    let history_uv = output_uv - velocity;

    // Out-of-bounds history → use current frame only
    if (history_uv.x < 0.0 || history_uv.x > 1.0 || history_uv.y < 0.0 || history_uv.y > 1.0) {
        textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(current_rgb, 1.0));
        return;
    }

    // Sample history at native resolution with Catmull-Rom for sharpness
    let history_rgb = sample_catmull_rom(t_history, samp, history_uv, params.output_resolution);

    // Neighborhood clamping in YCoCg space (sampled at input resolution spacing)
    let aabb = compute_neighborhood_aabb(output_uv);
    let history_ycocg = rgb_to_ycocg(history_rgb);
    let clamped_ycocg = clip_to_aabb(history_ycocg, aabb[0], aabb[1]);
    let clamped_history = ycocg_to_rgb(clamped_ycocg);

    // Velocity-dependent blend: faster motion → more current frame, less ghosting
    let velocity_pixels = length(velocity * params.output_resolution);
    let blend_factor = params.config.x;
    let dynamic_blend = mix(blend_factor, 0.4, clamp(velocity_pixels * 0.08, 0.0, 0.6));

    // Upscale confidence: when upscaling significantly, slightly prefer history
    // (accumulated sub-pixel detail) over the bilinearly-upscaled current frame.
    let scale_ratio = params.output_resolution.x / max(params.input_resolution.x, 1.0);
    let upscale_boost = clamp((scale_ratio - 1.0) * 0.02, 0.0, 0.03);
    let final_blend = min(dynamic_blend + upscale_boost, 0.98);

    let resolved = mix(current_rgb, clamped_history, final_blend);
    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(max(resolved, vec3<f32>(0.0)), 1.0));
}

// ============================================================================
// RCAS SHARPENING (post-upscale, at native resolution)
// ============================================================================

@compute @workgroup_size(8, 8, 1)
fn upscale_rcas_sharpen(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<i32>(params.output_resolution);
    let pixel = vec2<i32>(gid.xy);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let strength = params.config.z;
    if (strength <= 0.0) {
        let c = textureLoad(t_current, pixel, 0).rgb;
        textureStore(t_output, pixel, vec4<f32>(c, 1.0));
        return;
    }

    // 5-tap cross pattern
    let c = textureLoad(t_current, pixel, 0).rgb;
    let n = textureLoad(t_current, pixel + vec2<i32>(0, -1), 0).rgb;
    let s = textureLoad(t_current, pixel + vec2<i32>(0, 1), 0).rgb;
    let e = textureLoad(t_current, pixel + vec2<i32>(1, 0), 0).rgb;
    let w = textureLoad(t_current, pixel + vec2<i32>(-1, 0), 0).rgb;

    // Luma for edge detection
    let luma_w = vec3<f32>(0.2126, 0.7152, 0.0722);
    let lc = dot(c, luma_w);
    let ln = dot(n, luma_w);
    let ls = dot(s, luma_w);
    let le = dot(e, luma_w);
    let lw = dot(w, luma_w);

    let lmin = min(lc, min(min(ln, ls), min(le, lw)));
    let lmax = max(lc, max(max(ln, ls), max(le, lw)));

    // Adaptive sharpening: less on high-contrast edges (avoids ringing)
    let contrast = lmax - lmin;
    let w_sharp = clamp(1.0 - contrast * 4.0, 0.0, 1.0) * strength;

    let neighbors = (n + s + e + w) * 0.25;
    let sharpened = c + (c - neighbors) * w_sharp;

    textureStore(t_output, pixel, vec4<f32>(max(sharpened, vec3<f32>(0.0)), 1.0));
}
