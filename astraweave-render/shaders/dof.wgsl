// Depth of Field (circle of confusion based)
//
// Computes per-pixel circle of confusion (CoC) from depth, then applies
// a variable-radius gather blur. Near/far fields are separated to prevent
// bleeding between in-focus and out-of-focus regions.

struct DofParams {
    inv_resolution: vec2<f32>,
    focus_distance: f32,
    focus_range: f32,
    bokeh_size: f32,       // Max blur radius in pixels
    near_start: f32,       // Near field starts blurring
    near_end: f32,         // Near field fully blurred
    _pad: f32,
};

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> params: DofParams;
@group(0) @binding(4) var output_tex: texture_storage_2d<rgba16float, write>;

// Linearize depth (assumes reverse-Z or standard perspective)
fn linearize_depth(d: f32, near: f32, far: f32) -> f32 {
    return near * far / (far - d * (far - near));
}

// Compute circle of confusion
fn compute_coc(linear_depth: f32) -> f32 {
    let focus = params.focus_distance;
    let range = params.focus_range;

    // Distance from focus plane
    let dist_from_focus = abs(linear_depth - focus);

    // CoC normalized to [0, 1]
    let coc = clamp(dist_from_focus / range, 0.0, 1.0);

    return coc * params.bokeh_size;
}

// 16-sample disk pattern for bokeh gather
const BOKEH_SAMPLES: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>( 0.0,    0.0),
    vec2<f32>( 0.54,   0.0),
    vec2<f32>(-0.27,   0.47),
    vec2<f32>(-0.27,  -0.47),
    vec2<f32>( 0.94,   0.34),
    vec2<f32>(-0.94,   0.34),
    vec2<f32>( 0.0,   -1.0),
    vec2<f32>( 0.59,   0.81),
    vec2<f32>(-0.59,   0.81),
    vec2<f32>( 0.59,  -0.81),
    vec2<f32>(-0.59,  -0.81),
    vec2<f32>(-0.95,  -0.31),
    vec2<f32>( 0.95,  -0.31),
    vec2<f32>( 0.0,    1.0),
    vec2<f32>( 0.31,  -0.95),
    vec2<f32>(-0.31,   0.95)
);

@compute @workgroup_size(8, 8, 1)
fn dof_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
    let center_depth = textureSampleLevel(depth_tex, samp, uv, 0.0).r;

    // Approximate linear depth (assuming standard perspective with near=0.1, far=200)
    let linear_z = linearize_depth(center_depth, 0.1, 200.0);
    let coc = compute_coc(linear_z);

    if (coc < 0.5) {
        // In focus — pass through
        let color = textureSampleLevel(color_tex, samp, uv, 0.0);
        textureStore(output_tex, vec2<i32>(gid.xy), color);
        return;
    }

    // Gather blur with variable radius
    var total_color = vec3<f32>(0.0);
    var total_weight = 0.0;

    for (var i = 0u; i < 16u; i++) {
        let offset = BOKEH_SAMPLES[i] * coc * params.inv_resolution;
        let sample_uv = uv + offset;
        let sample_color = textureSampleLevel(color_tex, samp, sample_uv, 0.0).rgb;
        let sample_depth = textureSampleLevel(depth_tex, samp, sample_uv, 0.0).r;
        let sample_z = linearize_depth(sample_depth, 0.1, 200.0);
        let sample_coc = compute_coc(sample_z);

        // Weight: prefer samples with similar or larger CoC (prevents sharp objects bleeding into blur)
        let w = clamp(sample_coc / max(coc, 0.001), 0.0, 1.0);
        total_color += sample_color * w;
        total_weight += w;
    }

    let result = total_color / max(total_weight, 0.001);
    textureStore(output_tex, vec2<i32>(gid.xy), vec4<f32>(result, 1.0));
}
