// Nanite Material Resolve Shader
// Phase 3: Decode visibility buffer and apply full PBR materials
//
// This fullscreen shader:
// 1. Reads visibility buffer (meshlet ID + triangle ID)
// 2. Fetches triangle vertex data
// 3. Computes barycentric coordinates from pixel position
// 4. Interpolates vertex attributes (position, normal, UV)
// 5. Samples PBR textures from material arrays
// 6. Applies lighting (DDGI/VXGI integration)

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
    material_id: u32, // NEW: material index for texture sampling
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
    uv: vec2<f32>,
}

struct Camera {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

struct MaterialOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) pbr: vec4<f32>, // R=metallic, G=roughness, B=AO, A=unused
    @location(3) emissive: vec4<f32>,
}

// Geometry data
@group(0) @binding(0) var<storage, read> meshlets: array<Meshlet>;
@group(0) @binding(1) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;

// Visibility buffer (input from rasterization pass)
@group(1) @binding(0) var visibility_buffer: texture_2d<u32>;
@group(1) @binding(1) var depth_buffer: texture_2d<f32>;

// Material arrays (PBR textures)
@group(2) @binding(0) var albedo_array: texture_2d_array<f32>;
@group(2) @binding(1) var normal_array: texture_2d_array<f32>;
@group(2) @binding(2) var mra_array: texture_2d_array<f32>; // Metallic-Roughness-AO
@group(2) @binding(3) var emissive_array: texture_2d_array<f32>;
@group(2) @binding(4) var material_sampler: sampler;

// Optional: DDGI/VXGI for global illumination
@group(3) @binding(0) var vxgi_radiance: texture_3d<f32>;
@group(3) @binding(1) var vxgi_sampler: sampler;

// Unpack visibility ID
fn unpack_visibility_id(packed: u32) -> vec2<u32> {
    let meshlet_id = packed >> 16u;
    let triangle_id = packed & 0xFFFFu;
    return vec2<u32>(meshlet_id, triangle_id);
}

// Edge function for barycentric computation
fn edge_function(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    return (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x);
}

// Compute screen-space barycentric coordinates
fn compute_screen_barycentric(
    pixel_pos: vec2<f32>,
    v0_screen: vec2<f32>,
    v1_screen: vec2<f32>,
    v2_screen: vec2<f32>,
) -> vec3<f32> {
    let area = edge_function(v0_screen, v1_screen, v2_screen);
    if (abs(area) < 0.0001) {
        return vec3<f32>(1.0, 0.0, 0.0); // Degenerate, return vertex 0
    }
    
    let w0 = edge_function(v1_screen, v2_screen, pixel_pos) / area;
    let w1 = edge_function(v2_screen, v0_screen, pixel_pos) / area;
    let w2 = edge_function(v0_screen, v1_screen, pixel_pos) / area;
    
    return vec3<f32>(w0, w1, w2);
}

// Interpolate vertex attributes
fn interpolate_vec3(bary: vec3<f32>, v0: vec3<f32>, v1: vec3<f32>, v2: vec3<f32>) -> vec3<f32> {
    return bary.x * v0 + bary.y * v1 + bary.z * v2;
}

fn interpolate_vec2(bary: vec3<f32>, v0: vec2<f32>, v1: vec2<f32>, v2: vec2<f32>) -> vec2<f32> {
    return bary.x * v0 + bary.y * v1 + bary.z * v2;
}

// Compute tangent-space normal from normal map
fn apply_normal_map(
    normal_map: vec3<f32>,
    world_normal: vec3<f32>,
    world_tangent: vec3<f32>,
    tangent_sign: f32,
) -> vec3<f32> {
    let bitangent = cross(world_normal, world_tangent) * tangent_sign;
    let tbn = mat3x3<f32>(world_tangent, bitangent, world_normal);
    
    // Normal map is in [0,1], convert to [-1,1]
    let tangent_normal = normal_map * 2.0 - 1.0;
    return normalize(tbn * tangent_normal);
}

// Simple diffuse + specular lighting (placeholder for full DDGI)
fn compute_lighting(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
) -> vec3<f32> {
    // Simple directional light (sun)
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let light_color = vec3<f32>(1.0, 0.95, 0.9);
    
    // Diffuse
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse = albedo * light_color * n_dot_l;
    
    // Ambient (from VXGI - simplified)
    let ambient = albedo * 0.2 * ao;
    
    // TODO: Sample VXGI radiance texture for full GI
    // let vxgi_uv = world_pos_to_voxel_uv(world_pos);
    // let indirect = textureSample(vxgi_radiance, vxgi_sampler, vxgi_uv).rgb;
    
    return diffuse + ambient;
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> MaterialOutput {
    var output: MaterialOutput;
    
    let pixel_coords = vec2<i32>(input.position.xy);
    
    // Read visibility buffer
    let vis_id = textureLoad(visibility_buffer, pixel_coords, 0).r;
    
    // Check for background (0 = no geometry)
    if (vis_id == 0u) {
        // Background - output default values
        output.albedo = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        output.normal = vec4<f32>(0.0, 0.0, 1.0, 0.0);
        output.pbr = vec4<f32>(0.0, 1.0, 1.0, 0.0);
        output.emissive = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return output;
    }
    
    // Unpack meshlet and triangle IDs
    let ids = unpack_visibility_id(vis_id);
    let meshlet_id = ids.x;
    let triangle_id = ids.y;
    
    // Fetch meshlet
    let meshlet = meshlets[meshlet_id];
    
    // Fetch triangle vertices
    let index_offset = meshlet.triangle_offset + triangle_id * 3u;
    let i0 = indices[index_offset];
    let i1 = indices[index_offset + 1u];
    let i2 = indices[index_offset + 2u];
    
    let v0 = vertices[meshlet.vertex_offset + i0];
    let v1 = vertices[meshlet.vertex_offset + i1];
    let v2 = vertices[meshlet.vertex_offset + i2];
    
    // Transform vertices to clip space
    let clip0 = camera.view_proj * vec4<f32>(v0.position, 1.0);
    let clip1 = camera.view_proj * vec4<f32>(v1.position, 1.0);
    let clip2 = camera.view_proj * vec4<f32>(v2.position, 1.0);
    
    // Convert to screen space
    let screen_size = vec2<f32>(textureDimensions(visibility_buffer));
    let ndc0 = clip0.xyz / clip0.w;
    let ndc1 = clip1.xyz / clip1.w;
    let ndc2 = clip2.xyz / clip2.w;
    
    let screen0 = vec2<f32>(
        (ndc0.x * 0.5 + 0.5) * screen_size.x,
        (1.0 - (ndc0.y * 0.5 + 0.5)) * screen_size.y,
    );
    let screen1 = vec2<f32>(
        (ndc1.x * 0.5 + 0.5) * screen_size.x,
        (1.0 - (ndc1.y * 0.5 + 0.5)) * screen_size.y,
    );
    let screen2 = vec2<f32>(
        (ndc2.x * 0.5 + 0.5) * screen_size.x,
        (1.0 - (ndc2.y * 0.5 + 0.5)) * screen_size.y,
    );
    
    // Compute barycentric coordinates
    let pixel_pos = vec2<f32>(input.position.xy);
    let bary = compute_screen_barycentric(pixel_pos, screen0, screen1, screen2);
    
    // Interpolate vertex attributes
    let world_pos = interpolate_vec3(bary, v0.position, v1.position, v2.position);
    let world_normal = normalize(interpolate_vec3(bary, v0.normal, v1.normal, v2.normal));
    let world_tangent = normalize(interpolate_vec3(bary, v0.tangent.xyz, v1.tangent.xyz, v2.tangent.xyz));
    let tangent_sign = v0.tangent.w; // Assuming all vertices have same sign
    let uv = interpolate_vec2(bary, v0.uv, v1.uv, v2.uv);
    
    // Sample material textures
    let material_layer = f32(meshlet.material_id);
    let albedo_sample = textureSample(albedo_array, material_sampler, uv, i32(meshlet.material_id));
    let normal_sample = textureSample(normal_array, material_sampler, uv, i32(meshlet.material_id));
    let mra_sample = textureSample(mra_array, material_sampler, uv, i32(meshlet.material_id));
    let emissive_sample = textureSample(emissive_array, material_sampler, uv, i32(meshlet.material_id));
    
    // Apply normal map
    let final_normal = apply_normal_map(
        normal_sample.xyz,
        world_normal,
        world_tangent,
        tangent_sign,
    );
    
    // Extract PBR parameters
    let metallic = mra_sample.r;
    let roughness = mra_sample.g;
    let ao = mra_sample.b;
    
    // Compute lighting
    let lit_color = compute_lighting(
        world_pos,
        final_normal,
        albedo_sample.rgb,
        metallic,
        roughness,
        ao,
    );
    
    // Output G-buffer / final color
    output.albedo = vec4<f32>(lit_color + emissive_sample.rgb, albedo_sample.a);
    output.normal = vec4<f32>(final_normal * 0.5 + 0.5, 1.0);
    output.pbr = vec4<f32>(metallic, roughness, ao, 1.0);
    output.emissive = emissive_sample;
    
    return output;
}
