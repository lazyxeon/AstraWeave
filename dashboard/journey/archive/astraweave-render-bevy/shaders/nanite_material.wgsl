// Nanite material resolve shader
// This shader reads the visibility buffer and applies materials to visible meshlets

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

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

@group(0) @binding(0) var visibility_texture: texture_2d<u32>;
@group(0) @binding(1) var<storage, read> meshlets: array<Meshlet>;

@group(1) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Generate fullscreen triangle
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
    output.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, 1.0 - y);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Get screen coordinates
    let screen_size = textureDimensions(visibility_texture);
    let pixel_coord = vec2<i32>(input.uv * vec2<f32>(screen_size));
    
    // Read visibility buffer
    let packed_id = textureLoad(visibility_texture, pixel_coord, 0).r;
    
    // Unpack meshlet ID and triangle ID
    let meshlet_id = packed_id >> 16u;
    let triangle_id = packed_id & 0xFFFFu;
    
    // If no geometry was rendered here, discard
    if (packed_id == 0u) {
        discard;
    }
    
    // Get meshlet
    let meshlet = meshlets[meshlet_id];
    
    // Simple material visualization based on LOD level
    var color: vec3<f32>;
    switch (meshlet.lod_level) {
        case 0u: {
            color = vec3<f32>(0.2, 0.8, 0.2); // Green for LOD 0 (highest detail)
        }
        case 1u: {
            color = vec3<f32>(0.8, 0.8, 0.2); // Yellow for LOD 1
        }
        case 2u: {
            color = vec3<f32>(0.8, 0.4, 0.2); // Orange for LOD 2
        }
        default: {
            color = vec3<f32>(0.8, 0.2, 0.2); // Red for LOD 3+
        }
    }
    
    // Add some variation based on meshlet ID for debugging
    let variation = f32(meshlet_id % 10u) * 0.05;
    color = color * (0.8 + variation);
    
    return vec4<f32>(color, 1.0);
}