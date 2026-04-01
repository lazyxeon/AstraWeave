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

@compute @workgroup_size(8, 8, 1)
fn denoise_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let dims = textureDimensions(gi_current);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;

    // Spatial filtering: 3x3 bilateral
    let center = textureSampleLevel(gi_current, samp, uv, 0.0).rgb;
    let center_depth = textureSampleLevel(depth_tex, samp, uv, 0.0).r;

    var spatial_sum = center;
    var weight_sum = 1.0;

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) { continue; }

            let offset = vec2<f32>(f32(dx), f32(dy)) * params.inv_resolution;
            let sample_uv = uv + offset;
            let sample_color = textureSampleLevel(gi_current, samp, sample_uv, 0.0).rgb;
            let sample_depth = textureSampleLevel(depth_tex, samp, sample_uv, 0.0).r;

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

    // Temporal reprojection
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
