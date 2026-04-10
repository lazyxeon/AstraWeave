// SSGI spatial+temporal denoise pass
//
// Combines spatial bilateral filtering with temporal reprojection
// to produce stable, noise-free indirect lighting.

struct DenoiseParams {
    inv_resolution: vec2<f32>,
    spatial_sigma: f32,
    temporal_blend: f32,       // 0 = all current, 1 = all history
    depth_threshold: f32,
    normal_threshold: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var gi_current: texture_2d<f32>;
@group(0) @binding(1) var gi_history: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_2d<f32>;
@group(0) @binding(3) var velocity_tex: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;
@group(0) @binding(5) var<uniform> params: DenoiseParams;
@group(0) @binding(6) var gi_output: texture_storage_2d<rgba16float, write>;

// Shared memory tile: 8×8 output + 1-pixel apron = 10×10
const DN_TILE: u32 = 8u;
const DN_PAD: u32 = 1u;
const DN_TILE_PAD: u32 = DN_TILE + 2u * DN_PAD;
const DN_SHM: u32 = DN_TILE_PAD * DN_TILE_PAD;

var<workgroup> s_gi: array<vec3<f32>, 100>;
var<workgroup> s_dn_depth: array<f32, 100>;

@compute @workgroup_size(8, 8, 1)
fn denoise_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let dims = textureDimensions(gi_current);
    let base_x = i32(wid.x * DN_TILE) - i32(DN_PAD);
    let base_y = i32(wid.y * DN_TILE) - i32(DN_PAD);

    // Cooperative tile load: 64 threads load 100 entries (1-2 each)
    for (var i = li; i < DN_SHM; i += 64u) {
        let tx = i % DN_TILE_PAD;
        let ty = i / DN_TILE_PAD;
        let px = clamp(base_x + i32(tx), 0, i32(dims.x) - 1);
        let py = clamp(base_y + i32(ty), 0, i32(dims.y) - 1);
        let coord = vec2<i32>(px, py);
        s_gi[i] = textureLoad(gi_current, coord, 0).rgb;
        s_dn_depth[i] = textureLoad(depth_tex, coord, 0).r;
    }
    workgroupBarrier();

    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;

    // Center in tile coordinates (offset by apron)
    let cx = lid.x + DN_PAD;
    let cy = lid.y + DN_PAD;
    let center_idx = cy * DN_TILE_PAD + cx;

    // Spatial filtering: 3×3 bilateral from shared memory
    let center = s_gi[center_idx];
    let center_depth = s_dn_depth[center_idx];

    var spatial_sum = center;
    var weight_sum = 1.0;

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) { continue; }

            let idx = u32(i32(cy) + dy) * DN_TILE_PAD + u32(i32(cx) + dx);
            let sample_color = s_gi[idx];
            let sample_depth = s_dn_depth[idx];

            let depth_diff = abs(sample_depth - center_depth);
            let depth_weight = step(depth_diff, params.depth_threshold);

            let spatial_dist = length(vec2<f32>(f32(dx), f32(dy)));
            let spatial_weight = exp(-spatial_dist * spatial_dist / (2.0 * params.spatial_sigma * params.spatial_sigma));

            let w = spatial_weight * depth_weight;
            spatial_sum = spatial_sum + sample_color * w;
            weight_sum = weight_sum + w;
        }
    }

    let spatially_filtered = spatial_sum / max(weight_sum, 0.001);

    // Temporal reprojection (incoherent access — remains as texture sample)
    let velocity = textureSampleLevel(velocity_tex, samp, uv, 0.0).rg;
    let history_uv = uv - velocity;

    var result = spatially_filtered;

    if (history_uv.x >= 0.0 && history_uv.x <= 1.0 && history_uv.y >= 0.0 && history_uv.y <= 1.0) {
        let history = textureSampleLevel(gi_history, samp, history_uv, 0.0).rgb;

        // Neighborhood clamping: clamp history to the range of current spatial neighborhood
        let neighborhood_min = min(center, spatially_filtered) - 0.1;
        let neighborhood_max = max(center, spatially_filtered) + 0.1;
        let clamped_history = clamp(history, neighborhood_min, neighborhood_max);

        result = mix(spatially_filtered, clamped_history, params.temporal_blend);
    }

    textureStore(gi_output, pixel, vec4<f32>(result, 1.0));
}
