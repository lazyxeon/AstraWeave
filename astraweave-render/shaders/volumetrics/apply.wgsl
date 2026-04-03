// Volumetric Apply — Composite volumetric lighting onto the scene
//
// Blends the integrated volumetric fog result over the lit scene color.
// Uses transmittance for energy-correct compositing:
//   final = scene * transmittance + accumulated_light

struct ApplyParams {
    resolution:      vec2<f32>,
    inv_resolution:  vec2<f32>,
};

@group(0) @binding(0) var<uniform>  params:       ApplyParams;
@group(0) @binding(1) var           t_scene:      texture_2d<f32>;   // lit scene color
@group(0) @binding(2) var           t_volumetric: texture_2d<f32>;   // RGB=light, A=transmittance
@group(0) @binding(3) var           s_linear:     sampler;
@group(0) @binding(4) var           t_output:     texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn apply_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    let scene_color = textureSampleLevel(t_scene, s_linear, uv, 0.0).rgb;
    let volumetric = textureSampleLevel(t_volumetric, s_linear, uv, 0.0);

    let fog_light = volumetric.rgb;
    let transmittance = volumetric.a;

    // Energy-conserving compositing
    let result = scene_color * transmittance + fog_light;

    textureStore(t_output, pixel, vec4<f32>(result, 1.0));
}
