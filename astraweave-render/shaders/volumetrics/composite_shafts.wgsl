// Add screen-space light shafts onto a copy of the HDR scene.
//
// The scene is sampled. Shafts are written to a separate rgba16float
// storage texture (write-only). Read-write storage of rgba16float is not
// in the WebGPU guaranteed feature set, so this pass never uses it.
// The caller copies the storage texture onto the HDR target.

@group(0) @binding(0) var t_shafts: texture_2d<f32>;
@group(0) @binding(1) var s_shafts: sampler;
@group(0) @binding(2) var t_scene: texture_2d<f32>;
@group(0) @binding(3) var t_output: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn composite_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(t_output);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }
    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(dims);
    let scene = textureLoad(t_scene, pixel, 0);
    let shaft = textureSampleLevel(t_shafts, s_shafts, uv, 0.0).rgb;
    textureStore(t_output, pixel, vec4<f32>(scene.rgb + shaft, scene.a));
}
