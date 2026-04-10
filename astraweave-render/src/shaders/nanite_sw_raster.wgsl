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
// Atomic depth buffer: flat array of atomic<u32>, indexed pixel_y * width + pixel_x
// Depth stored as bitcast u32 with reversed-Z convention (higher = closer).
// Uses atomicMax for race-free depth test (closer fragments naturally have higher u32 values).
@group(1) @binding(2) var<storage, read_write> atomic_depth: array<atomic<u32>>;

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

// Software rasterization: tile-based processing with shared memory
// Each workgroup handles one 8×8 tile.  Thread 0 loads and transforms
// each triangle; all 64 threads test their pixel against the shared
// pre-transformed triangle — eliminating 63/64 redundant global reads
// and vertex transforms.

// Shared pre-transformed triangle data (written by thread 0, read by all)
var<workgroup> s_screen0: vec2<f32>;
var<workgroup> s_screen1: vec2<f32>;
var<workgroup> s_screen2: vec2<f32>;
var<workgroup> s_ndc_z0: f32;
var<workgroup> s_ndc_z1: f32;
var<workgroup> s_ndc_z2: f32;
var<workgroup> s_vis_id: u32;
// Meshlet iteration state (written by thread 0)
var<workgroup> s_tri_count: u32;
var<workgroup> s_tri_offset: u32;
var<workgroup> s_vtx_offset: u32;
var<workgroup> s_meshlet_id: u32;

@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) lid: u32,
) {
    let pixel_coords = global_id.xy;
    let screen_size = textureDimensions(visibility_buffer);
    let is_leader = lid == 0u;
    
    // All threads need to participate in barriers even if out of bounds,
    // so we track validity separately
    let valid_pixel = pixel_coords.x < screen_size.x && pixel_coords.y < screen_size.y;
    let pixel_center = vec2<f32>(f32(pixel_coords.x) + 0.5, f32(pixel_coords.y) + 0.5);
    
    // Iterate through all visible meshlets
    for (var meshlet_idx = 0u; meshlet_idx < visible_count; meshlet_idx++) {
        // Thread 0 loads meshlet metadata into shared memory
        if (is_leader) {
            let mid = visible_meshlet_ids[meshlet_idx];
            let m = meshlets[mid];
            s_meshlet_id = mid;
            s_tri_count = m.triangle_count;
            s_tri_offset = m.triangle_offset;
            s_vtx_offset = m.vertex_offset;
        }
        workgroupBarrier();

        // All threads read meshlet metadata from shared memory
        let tri_count = s_tri_count;
        let tri_offset = s_tri_offset;
        let vtx_offset = s_vtx_offset;
        let meshlet_id = s_meshlet_id;
        
        // Process all triangles in this meshlet
        for (var tri_idx = 0u; tri_idx < tri_count; tri_idx++) {
            // Thread 0 loads indices, vertices, transforms to screen space
            if (is_leader) {
                let index_offset = tri_offset + tri_idx * 3u;
                let i0 = indices[index_offset];
                let i1 = indices[index_offset + 1u];
                let i2 = indices[index_offset + 2u];
                
                let v0 = vertices[vtx_offset + i0];
                let v1 = vertices[vtx_offset + i1];
                let v2 = vertices[vtx_offset + i2];
                
                // Transform to clip space
                let clip0 = camera.view_proj * vec4<f32>(v0.position, 1.0);
                let clip1 = camera.view_proj * vec4<f32>(v1.position, 1.0);
                let clip2 = camera.view_proj * vec4<f32>(v2.position, 1.0);
                
                // Perspective divide
                let ndc0 = clip0.xyz / clip0.w;
                let ndc1 = clip1.xyz / clip1.w;
                let ndc2 = clip2.xyz / clip2.w;
                
                // Convert to screen space
                let sx = f32(screen_size.x);
                let sy = f32(screen_size.y);
                s_screen0 = vec2<f32>((ndc0.x * 0.5 + 0.5) * sx, (1.0 - (ndc0.y * 0.5 + 0.5)) * sy);
                s_screen1 = vec2<f32>((ndc1.x * 0.5 + 0.5) * sx, (1.0 - (ndc1.y * 0.5 + 0.5)) * sy);
                s_screen2 = vec2<f32>((ndc2.x * 0.5 + 0.5) * sx, (1.0 - (ndc2.y * 0.5 + 0.5)) * sy);
                s_ndc_z0 = ndc0.z;
                s_ndc_z1 = ndc1.z;
                s_ndc_z2 = ndc2.z;
                s_vis_id = pack_visibility_id(meshlet_id, tri_idx);
            }
            workgroupBarrier();

            // All threads: test their pixel against the shared triangle
            if (valid_pixel) {
                let bary = compute_barycentric(pixel_center, s_screen0, s_screen1, s_screen2);
                
                if (point_in_triangle(bary)) {
                    let depth = interpolate_depth(bary, s_ndc_z0, s_ndc_z1, s_ndc_z2);
                    
                    // Atomic depth test (reversed-Z: higher = closer)
                    let depth_u32 = bitcast<u32>(depth);
                    let pixel_idx = pixel_coords.y * screen_size.x + pixel_coords.x;
                    let old_depth_u32 = atomicMax(&atomic_depth[pixel_idx], depth_u32);

                    if (depth_u32 > old_depth_u32) {
                        textureStore(visibility_buffer, pixel_coords, vec4<u32>(s_vis_id, 0u, 0u, 0u));
                        textureStore(depth_buffer, pixel_coords, vec4<f32>(depth, 0.0, 0.0, 0.0));
                    }
                }
            }
            workgroupBarrier();
        }
    }
}
