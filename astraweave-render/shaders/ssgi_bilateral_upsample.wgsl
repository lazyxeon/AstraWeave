// Bilateral Upsample for SSGI
//
// Upsamples half-resolution SSGI output to full resolution using
// depth-aware bilateral weights. Preserves sharp edges at geometry
// boundaries by comparing depth of the full-res pixel against the
// four nearest half-res samples.

struct UpsampleParams {
    full_resolution: vec2<f32>,
    half_resolution: vec2<f32>,
    depth_threshold: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var half_gi: texture_2d<f32>;      // Half-res SSGI result
@group(0) @binding(1) var full_depth: texture_2d<f32>;    // Full-res depth
@group(0) @binding(2) var half_depth: texture_2d<f32>;    // Half-res depth (for bilateral weight)
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> params: UpsampleParams;
@group(0) @binding(5) var full_output: texture_storage_2d<rgba16float, write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn upsample_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let full_dims = vec2<i32>(params.full_resolution);
    if (pixel.x >= full_dims.x || pixel.y >= full_dims.y) {
        return;
    }

    let full_uv = (vec2<f32>(pixel) + 0.5) / params.full_resolution;

    // Reference depth at this full-res pixel
    let ref_depth = textureSampleLevel(full_depth, samp, full_uv, 0.0).r;

    // Map to half-res texel center
    let half_texel = full_uv * params.half_resolution - 0.5;
    let base = vec2<i32>(floor(half_texel));
    let frac = half_texel - vec2<f32>(base);

    let half_dims = vec2<i32>(params.half_resolution);

    // Sample 2×2 neighborhood in half-res with bilateral weights
    var total_color = vec3<f32>(0.0);
    var total_weight = 0.0;

    for (var dy = 0; dy < 2; dy++) {
        for (var dx = 0; dx < 2; dx++) {
            let coord = clamp(base + vec2<i32>(dx, dy), vec2<i32>(0), half_dims - 1);
            let half_uv = (vec2<f32>(coord) + 0.5) / params.half_resolution;

            // Bilinear weight
            let bx = select(1.0 - frac.x, frac.x, dx == 1);
            let by = select(1.0 - frac.y, frac.y, dy == 1);
            let bilinear_w = bx * by;

            // Depth-aware bilateral weight
            let sample_depth = textureSampleLevel(half_depth, samp, half_uv, 0.0).r;
            let depth_diff = abs(ref_depth - sample_depth);
            let bilateral_w = exp(-depth_diff * depth_diff / (params.depth_threshold * params.depth_threshold));

            let w = bilinear_w * bilateral_w;
            let color = textureSampleLevel(half_gi, samp, half_uv, 0.0).rgb;
            total_color += color * w;
            total_weight += w;
        }
    }

    var result = vec3<f32>(0.0);
    if (total_weight > 0.001) {
        result = total_color / total_weight;
    }

    textureStore(full_output, pixel, vec4<f32>(result, 1.0));
}
