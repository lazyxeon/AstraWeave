// Nanite visibility buffer shader
// This shader renders meshlets to a visibility buffer storing meshlet/triangle IDs

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

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0) var<storage, read> meshlets: array<Meshlet>;
@group(0) @binding(1) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;

@group(1) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) meshlet_id: u32,
    @location(1) triangle_id: u32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) meshlet_id: u32,
) -> VertexOutput {
    var output: VertexOutput;
    
    // Get meshlet
    let meshlet = meshlets[meshlet_id];
    
    // Get triangle ID within meshlet
    let triangle_id = vertex_index / 3u;
    let vertex_in_tri = vertex_index % 3u;
    
    // Get vertex index from meshlet indices
    let index_offset = meshlet.triangle_offset + triangle_id * 3u + vertex_in_tri;
    let local_vertex_index = indices[index_offset];
    let global_vertex_index = meshlet.vertex_offset + local_vertex_index;
    
    // Get vertex data
    let vertex = vertices[global_vertex_index];
    
    // Transform to clip space
    output.position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    output.meshlet_id = meshlet_id;
    output.triangle_id = triangle_id;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) u32 {
    // Pack meshlet ID and triangle ID into a single 32-bit value
    // Upper 16 bits: meshlet ID, Lower 16 bits: triangle ID
    let packed_id = (input.meshlet_id << 16u) | (input.triangle_id & 0xFFFFu);
    return packed_id;
}