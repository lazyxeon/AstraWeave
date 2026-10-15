// Min-depth downsample of the R32Float Hi-Z pyramid.
// textureLoad does not filter, so a uniform depth stays uniform in every mip.
// SSR, SSGI, and occlusion sample this same pyramid.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var dst_mip: texture_storage_2d<r32float, write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn downsample(@builtin(global_invocation_id) id: vec3<u32>) {
    let dst_coords = id.xy;
    let dst_size = textureDimensions(dst_mip);
    if (dst_coords.x >= dst_size.x || dst_coords.y >= dst_size.y) {
        return;
    }

    let src_coords = dst_coords * 2u;
    let src_size = textureDimensions(src);

    var min_depth = 1.0;
    for (var dy = 0u; dy < 2u; dy++) {
        for (var dx = 0u; dx < 2u; dx++) {
            let sc = src_coords + vec2<u32>(dx, dy);
            if (sc.x < src_size.x && sc.y < src_size.y) {
                min_depth = min(min_depth, textureLoad(src, sc, 0).r);
            }
        }
    }

    textureStore(dst_mip, dst_coords, vec4<f32>(min_depth, 0.0, 0.0, 0.0));
}
