// Add screen-space light shafts onto the HDR scene.
// Shafts may be half resolution; the sampler filters them up.

@group(0) @binding(0) var t_shafts: texture_2d<f32>;
@group(0) @binding(1) var s_shafts: sampler;
@group(0) @binding(2) var t_scene: texture_storage_2d<rgba16float, read_write>;

@compute @workgroup_size(8, 8)
fn composite_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(t_scene);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }
    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(dims);
    let scene = textureLoad(t_scene, pixel);
    let shaft = textureSampleLevel(t_shafts, s_shafts, uv, 0.0).rgb;
    textureStore(t_scene, pixel, vec4<f32>(scene.rgb + shaft, scene.a));
}
