// Per-pixel motion blur using velocity buffer
//
// Samples along the velocity direction for each pixel, producing
// directional blur proportional to motion speed. Uses tile-based
// max velocity for early-out optimization.

struct MotionBlurParams {
    inv_resolution: vec2<f32>,
    max_samples: u32,
    strength: f32,
    max_blur_pixels: f32,      // Clamp max blur distance
    _pad: vec3<f32>,
};

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var velocity_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> params: MotionBlurParams;
@group(0) @binding(5) var output_tex: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn motion_blur_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
    let velocity = textureSampleLevel(velocity_tex, samp, uv, 0.0).rg;

    // Convert velocity from UV space to pixel space
    let velocity_px = velocity * vec2<f32>(f32(dims.x), f32(dims.y)) * params.strength;
    let speed = length(velocity_px);

    // Early out for static pixels
    if (speed < 0.5) {
        let color = textureSampleLevel(color_tex, samp, uv, 0.0);
        textureStore(output_tex, vec2<i32>(gid.xy), color);
        return;
    }

    // Clamp blur distance
    let clamped_speed = min(speed, params.max_blur_pixels);
    let blur_dir = (velocity_px / speed) * clamped_speed * params.inv_resolution;

    // Number of samples proportional to blur length
    let num_samples = clamp(u32(clamped_speed * 0.5), 2u, params.max_samples);
    let step = blur_dir / f32(num_samples);

    // Sample along velocity direction (centered on pixel)
    var total_color = vec3<f32>(0.0);
    var total_weight = 0.0;
    let center_depth = textureSampleLevel(depth_tex, samp, uv, 0.0).r;

    for (var i = 0u; i < num_samples; i++) {
        let t = (f32(i) / f32(num_samples - 1u)) - 0.5; // -0.5 to 0.5
        let sample_uv = uv + step * f32(num_samples) * t;

        // Bounds check
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            continue;
        }

        let sample_color = textureSampleLevel(color_tex, samp, sample_uv, 0.0).rgb;
        let sample_depth = textureSampleLevel(depth_tex, samp, sample_uv, 0.0).r;

        // Depth-aware weighting: don't blur background into foreground
        let depth_diff = abs(sample_depth - center_depth);
        let depth_weight = 1.0 - clamp(depth_diff * 100.0, 0.0, 0.8);

        total_color += sample_color * depth_weight;
        total_weight += depth_weight;
    }

    let result = total_color / max(total_weight, 0.001);
    textureStore(output_tex, vec2<i32>(gid.xy), vec4<f32>(result, 1.0));
}
