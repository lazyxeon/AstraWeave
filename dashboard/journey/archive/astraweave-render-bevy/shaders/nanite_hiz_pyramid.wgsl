// Nanite Hi-Z Pyramid Builder
// Builds hierarchical depth buffer (mipmap pyramid) from previous frame depth
// Used for efficient occlusion culling in cluster cull pass

@group(0) @binding(0) var src_depth: texture_2d<f32>;
@group(0) @binding(1) var dst_depth: texture_storage_2d<r32float, write>;

// Downsample depth: Take maximum of 4 pixels (conservative for occlusion)
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dst_coords = id.xy;
    let dst_size = textureDimensions(dst_depth);
    
    if (dst_coords.x >= dst_size.x || dst_coords.y >= dst_size.y) {
        return;
    }
    
    // Source coordinates (2x resolution)
    let src_coords = dst_coords * 2u;
    let src_size = textureDimensions(src_depth);
    
    // Sample 2x2 block and take maximum depth (conservative)
    var max_depth = 0.0;
    
    for (var dy = 0u; dy < 2u; dy++) {
        for (var dx = 0u; dx < 2u; dx++) {
            let sample_coords = src_coords + vec2<u32>(dx, dy);
            if (sample_coords.x < src_size.x && sample_coords.y < src_size.y) {
                let depth = textureLoad(src_depth, sample_coords, 0).r;
                max_depth = max(max_depth, depth);
            }
        }
    }
    
    textureStore(dst_depth, dst_coords, vec4<f32>(max_depth, 0.0, 0.0, 0.0));
}
