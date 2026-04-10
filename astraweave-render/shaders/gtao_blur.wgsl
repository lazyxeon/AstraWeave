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

// Shared memory tile: 8×8 output + 3-pixel apron per side = 14×14
const TILE: u32 = 8u;
const BLUR_APRON: u32 = 3u;
const TILE_PAD: u32 = TILE + 2u * BLUR_APRON;
const SHM_SIZE: u32 = TILE_PAD * TILE_PAD;

var<workgroup> s_ao: array<f32, 196>;
var<workgroup> s_depth: array<f32, 196>;

@compute @workgroup_size(8, 8, 1)
fn blur_main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let dims = textureDimensions(ao_input);
    let base_x = i32(wid.x * TILE) - i32(BLUR_APRON);
    let base_y = i32(wid.y * TILE) - i32(BLUR_APRON);

    // Cooperative tile load: 64 threads load 196 entries (3-4 each)
    for (var i = li; i < SHM_SIZE; i += 64u) {
        let tx = i % TILE_PAD;
        let ty = i / TILE_PAD;
        let px = clamp(base_x + i32(tx), 0, i32(dims.x) - 1);
        let py = clamp(base_y + i32(ty), 0, i32(dims.y) - 1);
        let coord = vec2<i32>(px, py);
        s_ao[i] = textureLoad(ao_input, coord, 0).r;
        s_depth[i] = textureLoad(depth_tex, coord, 0);
    }
    workgroupBarrier();

    let pixel = vec2<i32>(gid.xy);
    if (pixel.x >= i32(dims.x) || pixel.y >= i32(dims.y)) {
        return;
    }

    // Center in tile coordinates (offset by apron)
    let cx = lid.x + BLUR_APRON;
    let cy = lid.y + BLUR_APRON;
    let center_idx = cy * TILE_PAD + cx;
    let center_ao_val = s_ao[center_idx];
    let center_depth_val = s_depth[center_idx];

    var total_ao = center_ao_val * WEIGHTS[0];
    var total_weight = WEIGHTS[0];

    // Step direction in tile space: (1,0) for horizontal, (0,1) for vertical
    let step_x = i32(params.direction.x);
    let step_y = i32(params.direction.y);

    // Bilateral filter: 7-tap (3 each side) from shared memory
    for (var i = 1; i < 4; i++) {
        // Positive direction
        let pi_idx = u32(i32(cy) + step_y * i) * TILE_PAD + u32(i32(cx) + step_x * i);
        let ao_p = s_ao[pi_idx];
        let depth_p = s_depth[pi_idx];
        let weight_p = WEIGHTS[i] * step(abs(depth_p - center_depth_val), params.depth_threshold);
        total_ao += ao_p * weight_p;
        total_weight += weight_p;

        // Negative direction
        let ni_idx = u32(i32(cy) - step_y * i) * TILE_PAD + u32(i32(cx) - step_x * i);
        let ao_n = s_ao[ni_idx];
        let depth_n = s_depth[ni_idx];
        let weight_n = WEIGHTS[i] * step(abs(depth_n - center_depth_val), params.depth_threshold);
        total_ao += ao_n * weight_n;
        total_weight += weight_n;
    }

    let final_ao = total_ao / max(total_weight, 0.001);
    textureStore(ao_output, pixel, vec4<f32>(final_ao, 0.0, 0.0, 0.0));
}
