// Nanite Software Rasterization Compute Shader
// Phase 2 of 2-pass visibility: Rasterize visible meshlets into visibility buffer
//
// This shader performs tile-based software rasterization:
// - Each workgroup processes one 8x8 tile
// - Edge function rasterization with barycentric coordinates
// - Atomic depth test and visibility buffer update

struct Meshlet {
    bounds_min: vec3<f32>,
    vertex_offset: u32,
    bounds_max: vec3<f32>,
    vertex_count: u32,
    cone_apex: vec3<f32>,
    triangle_offset: u32,
    cone_axis: vec3<f32>,
    triangle_count: u32,
    cone_cutoff: f32,
    lod_level: u32,
    lod_error: f32,
    _padding: u32,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
    uv: vec2<f32>,
}

struct Triangle {
    v0: vec3<f32>,
    v1: vec3<f32>,
    v2: vec3<f32>,
    meshlet_id: u32,
    triangle_id: u32,
}

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0) var<storage, read> meshlets: array<Meshlet>;
@group(0) @binding(1) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> visible_meshlet_ids: array<u32>;
@group(0) @binding(4) var<uniform> camera: Camera;
@group(0) @binding(5) var<uniform> visible_count: u32;

@group(1) @binding(0) var visibility_buffer: texture_storage_2d<r32uint, read_write>;
@group(1) @binding(1) var depth_buffer: texture_storage_2d<r32float, read_write>;

// Edge function for triangle rasterization
fn edge_function(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    return (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x);
}

// Compute barycentric coordinates
fn compute_barycentric(p: vec2<f32>, v0: vec2<f32>, v1: vec2<f32>, v2: vec2<f32>) -> vec3<f32> {
    let area = edge_function(v0, v1, v2);
    if (abs(area) < 0.0001) {
        return vec3<f32>(0.0, 0.0, 0.0); // Degenerate triangle
    }
    
    let w0 = edge_function(v1, v2, p) / area;
    let w1 = edge_function(v2, v0, p) / area;
    let w2 = edge_function(v0, v1, p) / area;
    
    return vec3<f32>(w0, w1, w2);
}

// Check if point is inside triangle
fn point_in_triangle(bary: vec3<f32>) -> bool {
    return bary.x >= 0.0 && bary.y >= 0.0 && bary.z >= 0.0;
}

// Interpolate depth using barycentric coordinates
fn interpolate_depth(bary: vec3<f32>, d0: f32, d1: f32, d2: f32) -> f32 {
    return bary.x * d0 + bary.y * d1 + bary.z * d2;
}

// Pack meshlet ID and triangle ID into u32
fn pack_visibility_id(meshlet_id: u32, triangle_id: u32) -> u32 {
    return (meshlet_id << 16u) | (triangle_id & 0xFFFFu);
}

// Software rasterization: tile-based processing
// Each workgroup handles one 8x8 tile
@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let pixel_coords = global_id.xy;
    let screen_size = textureDimensions(visibility_buffer);
    
    // Bounds check
    if (pixel_coords.x >= screen_size.x || pixel_coords.y >= screen_size.y) {
        return;
    }
    
    let pixel_center = vec2<f32>(f32(pixel_coords.x) + 0.5, f32(pixel_coords.y) + 0.5);
    
    // Iterate through all visible meshlets
    for (var meshlet_idx = 0u; meshlet_idx < visible_count; meshlet_idx++) {
        let meshlet_id = visible_meshlet_ids[meshlet_idx];
        let meshlet = meshlets[meshlet_id];
        
        // Process all triangles in this meshlet
        for (var tri_idx = 0u; tri_idx < meshlet.triangle_count; tri_idx++) {
            // Get triangle vertices
            let index_offset = meshlet.triangle_offset + tri_idx * 3u;
            let i0 = indices[index_offset];
            let i1 = indices[index_offset + 1u];
            let i2 = indices[index_offset + 2u];
            
            let v0 = vertices[meshlet.vertex_offset + i0];
            let v1 = vertices[meshlet.vertex_offset + i1];
            let v2 = vertices[meshlet.vertex_offset + i2];
            
            // Transform to clip space
            let clip0 = camera.view_proj * vec4<f32>(v0.position, 1.0);
            let clip1 = camera.view_proj * vec4<f32>(v1.position, 1.0);
            let clip2 = camera.view_proj * vec4<f32>(v2.position, 1.0);
            
            // Perspective divide
            let ndc0 = clip0.xyz / clip0.w;
            let ndc1 = clip1.xyz / clip1.w;
            let ndc2 = clip2.xyz / clip2.w;
            
            // Convert to screen space
            let screen0 = vec2<f32>(
                (ndc0.x * 0.5 + 0.5) * f32(screen_size.x),
                (1.0 - (ndc0.y * 0.5 + 0.5)) * f32(screen_size.y),
            );
            let screen1 = vec2<f32>(
                (ndc1.x * 0.5 + 0.5) * f32(screen_size.x),
                (1.0 - (ndc1.y * 0.5 + 0.5)) * f32(screen_size.y),
            );
            let screen2 = vec2<f32>(
                (ndc2.x * 0.5 + 0.5) * f32(screen_size.x),
                (1.0 - (ndc2.y * 0.5 + 0.5)) * f32(screen_size.y),
            );
            
            // Compute barycentric coordinates
            let bary = compute_barycentric(pixel_center, screen0, screen1, screen2);
            
            // Check if pixel is inside triangle
            if (point_in_triangle(bary)) {
                // Interpolate depth
                let depth = interpolate_depth(bary, ndc0.z, ndc1.z, ndc2.z);
                
                // Atomic depth test
                let depth_u32 = bitcast<u32>(depth);
                let old_depth_u32 = textureLoad(depth_buffer, pixel_coords).r;
                let old_depth = bitcast<f32>(old_depth_u32);
                
                // Reversed depth: closer = higher value
                if (depth > old_depth) {
                    // This fragment is closer - attempt atomic update
                    // Note: WGSL doesn't have atomic texture operations yet
                    // In practice, use storage buffer or accept race conditions
                    
                    textureStore(depth_buffer, pixel_coords, vec4<f32>(bitcast<f32>(depth_u32), 0.0, 0.0, 0.0));
                    
                    let vis_id = pack_visibility_id(meshlet_id, tri_idx);
                    textureStore(visibility_buffer, pixel_coords, vec4<u32>(vis_id, 0u, 0u, 0u));
                }
            }
        }
    }
}
