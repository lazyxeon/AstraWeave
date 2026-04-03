// Volumetric Integration — Front-to-back accumulation along each pixel's view ray
//
// For each screen pixel, march through the froxel grid (front-to-back),
// accumulating in-scattered light and transmittance using Beer's law.
// Output: RGBA16Float screen-space texture where RGB = accumulated light, A = transmittance.

struct IntegrateParams {
    inv_view_proj:   mat4x4<f32>,
    resolution:      vec2<f32>,
    inv_resolution:  vec2<f32>,
    froxel_dims:     vec3<u32>,
    near_plane:      f32,
    far_plane:       f32,
    scatter_strength: f32,  // overall scattering strength multiplier
    _pad0:           f32,
    _pad1:           f32,
};

@group(0) @binding(0) var<uniform>         params:    IntegrateParams;
@group(0) @binding(1) var                  t_scatter: texture_3d<f32>;   // in-scattered light + density
@group(0) @binding(2) var                  t_depth:   texture_2d<f32>;   // scene depth
@group(0) @binding(3) var                  s_linear:  sampler;
@group(0) @binding(4) var                  t_output:  texture_storage_2d<rgba16float, write>;

// Linearize depth to view-space distance
fn linearize_depth(d: f32) -> f32 {
    let near = params.near_plane;
    let far = params.far_plane;
    return near * far / (far - d * (far - near));
}

// Convert a depth slice index to linear depth (matching exponential distribution)
fn slice_to_depth(slice: f32) -> f32 {
    let z_near = params.near_plane;
    let z_far = params.far_plane;
    let t = slice / f32(params.froxel_dims.z);
    return z_near * pow(z_far / z_near, t);
}

@compute @workgroup_size(8, 8)
fn integrate_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let screen_dims = vec2<u32>(params.resolution);
    if (gid.x >= screen_dims.x || gid.y >= screen_dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    // Scene depth determines how far to integrate
    let scene_depth_raw = textureLoad(t_depth, pixel, 0).r;
    let scene_depth_linear = linearize_depth(scene_depth_raw);

    // Front-to-back accumulation along view ray through froxel slices
    var accumulated_light = vec3<f32>(0.0);
    var transmittance = 1.0;

    let num_slices = params.froxel_dims.z;
    var prev_depth = params.near_plane;

    for (var z = 0u; z < num_slices; z++) {
        let slice_depth = slice_to_depth(f32(z) + 0.5);

        // Stop if we've gone past the scene geometry
        if (slice_depth > scene_depth_linear) {
            break;
        }

        // Froxel UVW for sampling the scatter volume
        let uvw = vec3<f32>(uv, (f32(z) + 0.5) / f32(num_slices));
        let scatter_sample = textureSampleLevel(t_scatter, s_linear, uvw, 0.0);

        let in_scatter = scatter_sample.rgb * params.scatter_strength;
        let density = scatter_sample.a;

        // Step length in world units
        let step_length = slice_depth - prev_depth;
        prev_depth = slice_depth;

        // Beer's law extinction for this step
        let extinction = density * step_length;
        let step_transmittance = exp(-extinction);

        // Energy-conserving accumulation
        // Light visible through this slab = in_scatter * (1 - step_transmittance) / density
        // Simplified: in_scatter * step_length * transmittance for small steps
        let visible_scatter = in_scatter * step_length * transmittance;
        accumulated_light += visible_scatter;
        transmittance *= step_transmittance;

        // Early termination if nearly fully opaque
        if (transmittance < 0.01) {
            break;
        }
    }

    textureStore(t_output, pixel, vec4<f32>(accumulated_light, transmittance));
}
