// Temporal Anti-Aliasing (TAA)
//
// Blends the current jittered frame with reprojected history using motion vectors.
// Uses neighborhood clamping (AABB) to reject stale history and prevent ghosting.
// Catmull-Rom filtering on history for sharpness. Includes optional RCAS sharpening.
//
// Pipeline: jittered render → TAA resolve → optional sharpen → output

struct TaaParams {
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    // x: history blend factor (0.9-0.98), y: clamp_margin, z: sharpen_strength, w: frame_index
    config: vec4<f32>,
};

@group(0) @binding(0) var current_tex: texture_2d<f32>;   // Current jittered frame
@group(0) @binding(1) var history_tex: texture_2d<f32>;    // Previous resolved frame
@group(0) @binding(2) var velocity_tex: texture_2d<f32>;   // Motion vectors (RG16F)
@group(0) @binding(3) var depth_tex: texture_2d<f32>;      // Depth buffer
@group(0) @binding(4) var samp: sampler;
@group(0) @binding(5) var<uniform> params: TaaParams;
@group(0) @binding(6) var output_tex: texture_storage_2d<rgba16float, write>;

// Shared memory tile: 8×8 output + 1-pixel apron = 10×10
const TAA_TILE: u32 = 8u;
const TAA_PAD: u32 = 1u;
const TAA_TILE_PAD: u32 = TAA_TILE + 2u * TAA_PAD;
const TAA_SHM: u32 = TAA_TILE_PAD * TAA_TILE_PAD;

var<workgroup> s_current: array<vec3<f32>, 100>;
var<workgroup> s_taa_depth: array<f32, 100>;
var<workgroup> s_sharp: array<vec3<f32>, 100>;

// ============================================================================
// CATMULL-ROM 5-TAP FILTER (for sharp history sampling)
// ============================================================================

fn sample_catmull_rom(tex: texture_2d<f32>, s: sampler, uv: vec2<f32>, res: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / res;
    let pos = uv * res - 0.5;
    let f = fract(pos);
    let p = floor(pos);

    // Catmull-Rom weights
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);

    let w12 = w1 + w2;
    let tc12 = (p + 1.0 + w2 / w12) * texel;
    let tc0 = (p - 0.5) * texel;
    let tc3 = (p + 2.5) * texel;

    var color = vec3<f32>(0.0);
    // 5-tap approximation using bilinear filtering
    color += textureSampleLevel(tex, s, vec2<f32>(tc12.x, tc0.y), 0.0).rgb * (w12.x * w0.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc0.x, tc12.y), 0.0).rgb * (w0.x * w12.y);
    color += textureSampleLevel(tex, s, tc12, 0.0).rgb * (w12.x * w12.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc3.x, tc12.y), 0.0).rgb * (w3.x * w12.y);
    color += textureSampleLevel(tex, s, vec2<f32>(tc12.x, tc3.y), 0.0).rgb * (w12.x * w3.y);

    return max(color, vec3<f32>(0.0));
}

// ============================================================================
// NEIGHBORHOOD CLAMPING (AABB in YCoCg space)
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

// Sample 3x3 neighborhood from shared memory and compute min/max AABB in YCoCg space
fn compute_neighborhood_aabb_tiled(cx: u32, cy: u32) -> array<vec3<f32>, 2> {
    var aabb_min = vec3<f32>(1e10);
    var aabb_max = vec3<f32>(-1e10);

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let idx = u32(i32(cy) + dy) * TAA_TILE_PAD + u32(i32(cx) + dx);
            let sample_ycocg = rgb_to_ycocg(s_current[idx]);
            aabb_min = min(aabb_min, sample_ycocg);
            aabb_max = max(aabb_max, sample_ycocg);
        }
    }

    // Expand AABB slightly to reduce flickering on edges
    let margin = vec3<f32>(params.config.y);
    return array<vec3<f32>, 2>(aabb_min - margin, aabb_max + margin);
}

// Clamp a color to the AABB
fn clamp_to_aabb(color: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec3<f32> {
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
// CLOSEST DEPTH (for velocity dilation — use motion of nearest surface)
// ============================================================================

// Find pixel offset to closest depth in 3x3 neighborhood (from shared memory)
fn find_closest_depth_offset_tiled(cx: u32, cy: u32) -> vec2<i32> {
    var closest_depth = 1.0;
    var best_offset = vec2<i32>(0, 0);

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let idx = u32(i32(cy) + dy) * TAA_TILE_PAD + u32(i32(cx) + dx);
            let d = s_taa_depth[idx];
            if (d < closest_depth) {
                closest_depth = d;
                best_offset = vec2<i32>(dx, dy);
            }
        }
    }

    return best_offset;
}

// ============================================================================
// TAA RESOLVE
// ============================================================================

@compute @workgroup_size(8, 8, 1)
fn taa_resolve(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let dims = vec2<i32>(params.resolution);
    let base_x = i32(wid.x * TAA_TILE) - i32(TAA_PAD);
    let base_y = i32(wid.y * TAA_TILE) - i32(TAA_PAD);

    // Cooperative tile load: 64 threads load 100 entries (1-2 each)
    for (var i = li; i < TAA_SHM; i += 64u) {
        let tx = i % TAA_TILE_PAD;
        let ty = i / TAA_TILE_PAD;
        let px = clamp(base_x + i32(tx), 0, dims.x - 1);
        let py = clamp(base_y + i32(ty), 0, dims.y - 1);
        let coord = vec2<i32>(px, py);
        s_current[i] = textureLoad(current_tex, coord, 0).rgb;
        s_taa_depth[i] = textureLoad(depth_tex, coord, 0).r;
    }
    workgroupBarrier();

    let pixel = vec2<i32>(gid.xy);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;
    let blend_factor = params.config.x;

    // Center in tile coordinates (offset by apron)
    let cx = lid.x + TAA_PAD;
    let cy = lid.y + TAA_PAD;
    let center_idx = cy * TAA_TILE_PAD + cx;

    // Current frame color from shared memory
    let current_rgb = s_current[center_idx];

    // Find velocity at closest depth (from shared memory — dilated velocity for edge stability)
    let closest_offset = find_closest_depth_offset_tiled(cx, cy);
    let closest_uv = uv + vec2<f32>(closest_offset) * params.inv_resolution;
    let velocity = textureSampleLevel(velocity_tex, samp, closest_uv, 0.0).rg;

    // Reprojected history UV
    let history_uv = uv - velocity;

    // Check if history UV is in bounds
    if (history_uv.x < 0.0 || history_uv.x > 1.0 || history_uv.y < 0.0 || history_uv.y > 1.0) {
        // No valid history — use current frame only
        textureStore(output_tex, pixel, vec4<f32>(current_rgb, 1.0));
        return;
    }

    // Sample history with Catmull-Rom for sharpness
    let history_rgb = sample_catmull_rom(history_tex, samp, history_uv, params.resolution);

    // Neighborhood clamping in YCoCg space from shared memory (anti-ghosting)
    let aabb = compute_neighborhood_aabb_tiled(cx, cy);
    let history_ycocg = rgb_to_ycocg(history_rgb);
    let clamped_ycocg = clamp_to_aabb(history_ycocg, aabb[0], aabb[1]);
    let clamped_history = ycocg_to_rgb(clamped_ycocg);

    // Velocity-dependent blend: faster motion = more current frame
    let velocity_magnitude = length(velocity * params.resolution);
    let dynamic_blend = mix(blend_factor, 0.5, clamp(velocity_magnitude * 0.1, 0.0, 0.5));

    // Blend current with clamped history
    let resolved = mix(current_rgb, clamped_history, dynamic_blend);

    textureStore(output_tex, pixel, vec4<f32>(max(resolved, vec3<f32>(0.0)), 1.0));
}

// ============================================================================
// RCAS SHARPENING (Robust Contrast-Adaptive Sharpening)
// ============================================================================

@group(0) @binding(0) var sharp_input: texture_2d<f32>;
// binding 4: sampler (reused)
// binding 5: params (reused)

@compute @workgroup_size(8, 8, 1)
fn rcas_sharpen(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let dims = vec2<i32>(params.resolution);
    let base_x = i32(wid.x * TAA_TILE) - i32(TAA_PAD);
    let base_y = i32(wid.y * TAA_TILE) - i32(TAA_PAD);

    // Cooperative tile load: 64 threads load 100 entries
    for (var i = li; i < TAA_SHM; i += 64u) {
        let tx = i % TAA_TILE_PAD;
        let ty = i / TAA_TILE_PAD;
        let px = clamp(base_x + i32(tx), 0, dims.x - 1);
        let py = clamp(base_y + i32(ty), 0, dims.y - 1);
        let coord = vec2<i32>(px, py);
        s_sharp[i] = textureLoad(sharp_input, coord, 0).rgb;
    }
    workgroupBarrier();

    let pixel = vec2<i32>(gid.xy);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let strength = params.config.z;
    let cx = lid.x + TAA_PAD;
    let cy = lid.y + TAA_PAD;
    let center_idx = cy * TAA_TILE_PAD + cx;

    if (strength <= 0.0) {
        textureStore(output_tex, pixel, vec4<f32>(s_sharp[center_idx], 1.0));
        return;
    }

    // 5-tap cross pattern from shared memory
    let c = s_sharp[center_idx];
    let n = s_sharp[(cy - 1u) * TAA_TILE_PAD + cx];
    let s_val = s_sharp[(cy + 1u) * TAA_TILE_PAD + cx];
    let e = s_sharp[cy * TAA_TILE_PAD + cx + 1u];
    let w = s_sharp[cy * TAA_TILE_PAD + cx - 1u];

    // Luma for edge detection
    let luma_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
    let lc = dot(c, luma_weights);
    let ln = dot(n, luma_weights);
    let ls = dot(s_val, luma_weights);
    let le = dot(e, luma_weights);
    let lw = dot(w, luma_weights);

    let lmin = min(lc, min(min(ln, ls), min(le, lw)));
    let lmax = max(lc, max(max(ln, ls), max(le, lw)));

    // Adaptive sharpening weight (less sharpening on high-contrast edges)
    let contrast = lmax - lmin;
    let w_sharp = clamp(1.0 - contrast * 4.0, 0.0, 1.0) * strength;

    let neighbors = (n + s_val + e + w) * 0.25;
    let sharpened = c + (c - neighbors) * w_sharp;

    textureStore(output_tex, pixel, vec4<f32>(max(sharpened, vec3<f32>(0.0)), 1.0));
}
