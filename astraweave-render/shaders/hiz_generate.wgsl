// Hierarchical Z-Buffer (Hi-Z) mip generation
//
// Generates a mip chain of the depth buffer where each texel contains
// the maximum (farthest) depth of the 2x2 parent texels. Used for
// efficient screen-space ray marching in SSR and SSGI.

@group(0) @binding(0) var src_mip: texture_2d<f32>;
@group(0) @binding(1) var dst_mip: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8, 8, 1)
fn hiz_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dst_dims = textureDimensions(dst_mip);
    if (gid.x >= dst_dims.x || gid.y >= dst_dims.y) {
        return;
    }

    // Sample 2x2 block from source mip, take max (conservative depth)
    let src_coord = vec2<i32>(gid.xy) * 2;

    let d00 = textureLoad(src_mip, src_coord + vec2<i32>(0, 0), 0).r;
    let d10 = textureLoad(src_mip, src_coord + vec2<i32>(1, 0), 0).r;
    let d01 = textureLoad(src_mip, src_coord + vec2<i32>(0, 1), 0).r;
    let d11 = textureLoad(src_mip, src_coord + vec2<i32>(1, 1), 0).r;

    let max_depth = max(max(d00, d10), max(d01, d11));
    textureStore(dst_mip, vec2<i32>(gid.xy), vec4<f32>(max_depth, 0.0, 0.0, 0.0));
}
