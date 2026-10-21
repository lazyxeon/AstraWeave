// Copy a depth attachment into Hi-Z mip 0.
// The source is texture_depth_2d. The destination is R32Float storage.
// draw_into records this only when sample_hiz is set.
// Occlusion, SSR, and SSGI do not sample that R32Float copy.

@group(0) @binding(0) var src_depth: texture_depth_2d;
@group(0) @binding(1) var dst: texture_storage_2d<r32float, write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn copy_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = id.xy;
    let size = textureDimensions(dst);
    if (coords.x >= size.x || coords.y >= size.y) {
        return;
    }
    let depth = textureLoad(src_depth, coords, 0);
    textureStore(dst, coords, vec4<f32>(depth, 0.0, 0.0, 0.0));
}
