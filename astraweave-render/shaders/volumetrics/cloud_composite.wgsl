// Volumetric Cloud Composite — Upscale + Depth-Aware Compositing
//
// Reads the half-resolution cloud raymarching result and composites it
// onto the full-resolution scene. Clouds are rendered behind scene geometry
// by checking scene depth against the cloud layer altitude.
//
// Output: composited scene with clouds blended in.

struct CloudCompositeParams {
    resolution:          vec2<f32>,
    inv_resolution:      vec2<f32>,
    cloud_resolution:    vec2<f32>,
    inv_cloud_resolution: vec2<f32>,
    near_plane:          f32,
    far_plane:           f32,
    cloud_altitude:      f32,
    cloud_thickness:     f32,
};

@group(0) @binding(0) var<uniform>  params:      CloudCompositeParams;
@group(0) @binding(1) var           t_scene:     texture_2d<f32>;     // full-res scene color (HDR)
@group(0) @binding(2) var           t_cloud:     texture_2d<f32>;     // half-res cloud result
@group(0) @binding(3) var           t_depth:     texture_2d<f32>;     // full-res depth buffer
@group(0) @binding(4) var           s_linear:    sampler;
@group(0) @binding(5) var           t_output:    texture_storage_2d<rgba16float, write>;

// Linearize depth from NDC
fn linearize_depth(ndc_z: f32) -> f32 {
    let n = params.near_plane;
    let f = params.far_plane;
    return (n * f) / (f - ndc_z * (f - n));
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn cloud_composite_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(u32(params.resolution.x), u32(params.resolution.y));
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    // Read scene color at full resolution
    let scene_color = textureSampleLevel(t_scene, s_linear, uv, 0.0).rgb;

    // Read depth
    let raw_depth = textureSampleLevel(t_depth, s_linear, uv, 0.0).r;
    let linear_depth = linearize_depth(raw_depth);

    // Read cloud result (bilinear upscale from half-res)
    let cloud = textureSampleLevel(t_cloud, s_linear, uv, 0.0);
    let cloud_color = cloud.rgb;
    let cloud_transmittance = cloud.a;

    // Only composite clouds where scene geometry is behind the cloud layer.
    // Approximate: cloud starts at cloud_altitude, which maps to some view distance.
    // If scene depth is very close (geometry in front of clouds), skip compositing.
    // A distance of far_plane * 0.95 or greater is considered "sky" (no geometry).
    let is_sky = linear_depth > params.far_plane * 0.95;

    // Smooth blend: if geometry is beyond half the far plane, start fading in clouds
    let cloud_fade = smoothstep(params.far_plane * 0.3, params.far_plane * 0.5, linear_depth);

    // Composite: cloud_color + scene * transmittance (energy-conserving blend)
    var composited: vec3<f32>;
    if (is_sky || cloud_fade > 0.0) {
        let effective_transmittance = mix(1.0, cloud_transmittance, cloud_fade);
        composited = scene_color * effective_transmittance + cloud_color * cloud_fade;
    } else {
        composited = scene_color;
    }

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(composited, 1.0));
}
