// Hi-Z Pyramid Builder (Min-Depth)
//
// Builds a hierarchical min-depth buffer for screen-space tracing (SSR, SSGI).
// Each mip stores the minimum depth of the 2×2 block from the finer mip,
// enabling ray traversal to skip empty space at coarse mip levels.
//
// Two entry points:
//   copy_depth  — 1:1 copy from source depth to Hi-Z mip 0
//   downsample  — 2:1 min-depth reduction for subsequent mip levels

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var dst: texture_storage_2d<r32float, write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

// 1:1 copy from source depth texture into Hi-Z mip 0.
@compute @workgroup_size(WG_X, WG_Y)
fn copy_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let coords = id.xy;
    let size = textureDimensions(dst);
    if (coords.x >= size.x || coords.y >= size.y) {
        return;
    }
    let depth = textureLoad(src, coords, 0).r;
    textureStore(dst, coords, vec4<f32>(depth, 0.0, 0.0, 0.0));
}

// 2:1 min-depth downsample: minimum of 2×2 block from source mip.
@compute @workgroup_size(WG_X, WG_Y)
fn downsample(@builtin(global_invocation_id) id: vec3<u32>) {
    let dst_coords = id.xy;
    let dst_size = textureDimensions(dst);
    if (dst_coords.x >= dst_size.x || dst_coords.y >= dst_size.y) {
        return;
    }

    let src_coords = dst_coords * 2u;
    let src_size = textureDimensions(src);

    // Take minimum of 2×2 block (closest surface for intersection testing)
    var min_depth = 1.0;
    for (var dy = 0u; dy < 2u; dy++) {
        for (var dx = 0u; dx < 2u; dx++) {
            let sc = src_coords + vec2<u32>(dx, dy);
            if (sc.x < src_size.x && sc.y < src_size.y) {
                min_depth = min(min_depth, textureLoad(src, sc, 0).r);
            }
        }
    }

    textureStore(dst, dst_coords, vec4<f32>(min_depth, 0.0, 0.0, 0.0));
}
