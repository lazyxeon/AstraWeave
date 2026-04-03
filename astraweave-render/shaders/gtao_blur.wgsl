// Bilateral blur for GTAO — depth-aware spatial filtering
//
// Preserves edges by weighting samples based on depth similarity.
// Run as a separable filter: horizontal pass then vertical pass.

struct BlurParams {
    direction: vec2<f32>,       // (1,0) for horizontal, (0,1) for vertical
    inv_resolution: vec2<f32>,  // 1/width, 1/height
    depth_threshold: f32,       // Depth difference threshold for edge detection
    _pad: vec3<f32>,
};

@group(0) @binding(0) var ao_input: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> params: BlurParams;
@group(0) @binding(4) var ao_output: texture_storage_2d<r32float, write>;

// Gaussian weights for 7-tap filter
const WEIGHTS: array<f32, 4> = array<f32, 4>(0.3829, 0.2417, 0.0606, 0.0060);

@compute @workgroup_size(8, 8, 1)
fn blur_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let dims = textureDimensions(ao_input);
    if (pixel.x >= i32(dims.x) || pixel.y >= i32(dims.y)) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;
    let center_depth = textureLoad(depth_tex, pixel, 0);
    let center_ao = textureSampleLevel(ao_input, samp, uv, 0.0).r;

    var total_ao = center_ao * WEIGHTS[0];
    var total_weight = WEIGHTS[0];

    // Bilateral filter: 7-tap (3 each side)
    for (var i = 1; i < 4; i++) {
        let offset = params.direction * params.inv_resolution * f32(i);

        // Positive direction
        let uv_p = uv + offset;
        let pixel_p = vec2<i32>(uv_p / params.inv_resolution);
        let depth_p = textureLoad(depth_tex, pixel_p, 0);
        let ao_p = textureSampleLevel(ao_input, samp, uv_p, 0.0).r;
        let depth_diff_p = abs(depth_p - center_depth);
        let weight_p = WEIGHTS[i] * step(depth_diff_p, params.depth_threshold);
        total_ao += ao_p * weight_p;
        total_weight += weight_p;

        // Negative direction
        let uv_n = uv - offset;
        let pixel_n = vec2<i32>(uv_n / params.inv_resolution);
        let depth_n = textureLoad(depth_tex, pixel_n, 0);
        let ao_n = textureSampleLevel(ao_input, samp, uv_n, 0.0).r;
        let depth_diff_n = abs(depth_n - center_depth);
        let weight_n = WEIGHTS[i] * step(depth_diff_n, params.depth_threshold);
        total_ao += ao_n * weight_n;
        total_weight += weight_n;
    }

    let final_ao = total_ao / max(total_weight, 0.001);
    textureStore(ao_output, pixel, vec4<f32>(final_ao, 0.0, 0.0, 0.0));
}
