// Bloom downsample pass (13-tap filter to prevent fireflies)
//
// Uses a weighted 13-tap filter that samples a 4x4 texel area, preventing
// single bright pixels from dominating the bloom. This is the standard
// approach from "Next Generation Post Processing in Call of Duty: Advanced Warfare".

struct BloomParams {
    inv_resolution: vec2<f32>,  // 1/src_width, 1/src_height
    threshold: f32,              // Brightness threshold for bloom
    soft_knee: f32,              // Smooth threshold transition width
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> params: BloomParams;
@group(0) @binding(3) var dst_tex: texture_storage_2d<rgba16float, write>;

// Soft threshold: smoothly ramps from 0 to 1 around the threshold
fn soft_threshold(color: vec3<f32>, threshold: f32, knee: f32) -> vec3<f32> {
    let brightness = max(color.r, max(color.g, color.b));
    let soft = brightness - threshold + knee;
    let soft_clamped = clamp(soft, 0.0, 2.0 * knee);
    let contribution = soft_clamped * soft_clamped / (4.0 * knee + 0.0001);
    let mult = max(contribution, brightness - threshold) / max(brightness, 0.0001);
    return color * max(mult, 0.0);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn bloom_downsample(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dst_dims = textureDimensions(dst_tex);
    if (gid.x >= dst_dims.x || gid.y >= dst_dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(dst_dims);
    let d = params.inv_resolution;

    // 13-tap downsample filter (weighted box + cross pattern)
    // Weights: center cross = 0.5 total, corners = 0.5 total
    var color = vec3<f32>(0.0);

    // Center sample (weight: 0.125 * 4 = 0.5 allocated across 4 bilinear taps)
    color += textureSampleLevel(src_tex, samp, uv, 0.0).rgb * 0.125;

    // 4 corner samples (each a bilinear tap between 4 texels)
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(-d.x, -d.y), 0.0).rgb * 0.0625;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>( d.x, -d.y), 0.0).rgb * 0.0625;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(-d.x,  d.y), 0.0).rgb * 0.0625;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>( d.x,  d.y), 0.0).rgb * 0.0625;

    // 4 edge midpoint samples
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(-d.x, 0.0), 0.0).rgb * 0.125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>( d.x, 0.0), 0.0).rgb * 0.125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(0.0, -d.y), 0.0).rgb * 0.125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(0.0,  d.y), 0.0).rgb * 0.125;

    // 4 far corner samples
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(-2.0*d.x, -2.0*d.y), 0.0).rgb * 0.03125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>( 2.0*d.x, -2.0*d.y), 0.0).rgb * 0.03125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>(-2.0*d.x,  2.0*d.y), 0.0).rgb * 0.03125;
    color += textureSampleLevel(src_tex, samp, uv + vec2<f32>( 2.0*d.x,  2.0*d.y), 0.0).rgb * 0.03125;

    // Apply threshold on first mip only (params.threshold > 0 indicates first mip)
    if (params.threshold > 0.0) {
        color = soft_threshold(color, params.threshold, params.soft_knee);
    }

    textureStore(dst_tex, vec2<i32>(gid.xy), vec4<f32>(color, 1.0));
}
