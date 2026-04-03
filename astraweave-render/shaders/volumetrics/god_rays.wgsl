// God Rays — Screen-space radial light shafts
//
// Projects the sun position to screen space and performs radial blur
// from the sun to each pixel, sampling the depth buffer to determine
// occlusion. Produces bright shafts where light passes between occluders.
//
// Based on the classic Crepuscular Rays technique with multi-sample
// radial marching and exponential decay.

struct GodRayParams {
    resolution:        vec2<f32>,
    inv_resolution:    vec2<f32>,
    sun_screen_pos:    vec2<f32>,  // sun position in screen UV [0,1]
    sun_visible:       f32,        // 1.0 if sun is on screen, 0.0 otherwise
    num_samples:       u32,        // ray march samples (32-64)
    density:           f32,        // overall ray density
    weight:            f32,        // per-sample weight
    decay:             f32,        // exponential decay per sample
    exposure:          f32,        // final brightness multiplier
    sun_color:         vec3<f32>,
    _pad:              f32,
};

@group(0) @binding(0) var<uniform>  params:    GodRayParams;
@group(0) @binding(1) var           t_depth:   texture_depth_2d;  // scene depth (Depth32Float)
@group(0) @binding(2) var           t_scene:   texture_2d<f32>;   // lit scene (for luminance)
@group(0) @binding(3) var           s_linear:  sampler;
@group(0) @binding(4) var           t_output:  texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn god_rays_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    // Skip if sun is behind camera
    if (params.sun_visible < 0.5) {
        textureStore(t_output, pixel, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Direction from pixel toward sun in screen space
    let delta_uv = (params.sun_screen_pos - uv) * params.density / f32(params.num_samples);

    var sample_uv = uv;
    var accumulated = vec3<f32>(0.0);
    var illumination_decay = 1.0;

    for (var i = 0u; i < params.num_samples; i++) {
        sample_uv += delta_uv;

        // Clamp to screen bounds
        let clamped_uv = clamp(sample_uv, vec2<f32>(0.001), vec2<f32>(0.999));

        // Sample depth — sky pixels (depth ≈ 1.0) contribute light, occluders don't
        let depth_dims = vec2<f32>(textureDimensions(t_depth));
        let depth_pixel = vec2<i32>(clamped_uv * depth_dims);
        let depth = textureLoad(t_depth, depth_pixel, 0);

        // Occlusion test: sky = bright, geometry = dark
        let is_sky = step(0.999, depth);

        // Also sample scene luminance for bright emissive surfaces
        let scene_lum = textureSampleLevel(t_scene, s_linear, clamped_uv, 0.0).rgb;
        let luminance = dot(scene_lum, vec3<f32>(0.2126, 0.7152, 0.0722));
        let bright = step(2.0, luminance); // only very bright pixels contribute

        let contribution = max(is_sky, bright) * params.weight * illumination_decay;
        accumulated += params.sun_color * contribution;

        illumination_decay *= params.decay;
    }

    let result = accumulated * params.exposure;
    textureStore(t_output, pixel, vec4<f32>(result, 0.0));
}
