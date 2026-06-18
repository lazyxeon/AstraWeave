// SSFR Depth Pass - Renders particle spheres to a depth texture
// Screen-Space Fluid Rendering with correct sphere-surface depth

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    cam_pos: vec4<f32>,
    light_dir: vec4<f32>,
    time: f32,
    _pad_a: f32,
    _pad_b: f32,
    _pad_c: f32,
    _pad_v0: vec4<f32>,
    _pad_v1: vec4<f32>,
    _pad_v2: vec4<f32>,
    _pad_v3: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec4<f32>, // Instance position
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) view_pos: vec3<f32>,
    @location(2) radius: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let vertex_id = in.vertex_index % 4u;
    var quad_offset = vec2<f32>(0.0, 0.0);
    
    if (vertex_id == 0u) {
        quad_offset = vec2<f32>(-1.0, -1.0);
        out.uv = vec2<f32>(-1.0, -1.0);
    } else if (vertex_id == 1u) {
        quad_offset = vec2<f32>(1.0, -1.0);
        out.uv = vec2<f32>(1.0, -1.0);
    } else if (vertex_id == 2u) {
        quad_offset = vec2<f32>(-1.0, 1.0);
        out.uv = vec2<f32>(-1.0, 1.0);
    } else {
        quad_offset = vec2<f32>(1.0, 1.0);
        out.uv = vec2<f32>(1.0, 1.0);
    }
    
    let particle_radius = 0.5;
    out.radius = particle_radius;
    
    let world_pos = in.position.xyz;
    
    // Extract camera basis vectors from view_inv (camera-to-world, column-major).
    // Column 0 = right, column 1 = up, column 2 = back (-forward).
    let cam_right  = camera.view_inv[0].xyz;
    let cam_up     = camera.view_inv[1].xyz;
    let cam_back   = camera.view_inv[2].xyz;
    let cam_origin = camera.cam_pos.xyz;
    
    // Compute true view-space position of particle center.
    let rel = world_pos - cam_origin;
    out.view_pos = vec3<f32>(dot(rel, cam_right), dot(rel, cam_up), dot(rel, cam_back));
    
    // Project particle center to clip space.
    let clip_center = camera.view_proj * vec4<f32>(world_pos, 1.0);
    
    // Compute perspective-correct projected radii by projecting edge points
    // along BOTH camera axes. NDC x and y scale differently (the projection
    // matrix divides x by the aspect ratio), so using the right-axis radius
    // for both quad axes — the pre-F.1.2 code — stretched every impostor
    // horizontally by the aspect ratio: visibly oblong spheres on any
    // non-square window (the owner's "perfect oblong spheres" report).
    let ndc_center = clip_center.xy / clip_center.w;

    let edge_right_world = world_pos + cam_right * particle_radius;
    let clip_edge_right  = camera.view_proj * vec4<f32>(edge_right_world, 1.0);
    let ndc_radius_x     = length(clip_edge_right.xy / clip_edge_right.w - ndc_center);

    let edge_up_world = world_pos + cam_up * particle_radius;
    let clip_edge_up  = camera.view_proj * vec4<f32>(edge_up_world, 1.0);
    let ndc_radius_y  = length(clip_edge_up.xy / clip_edge_up.w - ndc_center);

    // Expand quad in clip space with 20% margin for anti-aliasing.
    out.clip_position = vec4<f32>(
        clip_center.x + quad_offset.x * ndc_radius_x * clip_center.w * 1.2,
        clip_center.y + quad_offset.y * ndc_radius_y * clip_center.w * 1.2,
        clip_center.z,
        clip_center.w
    );
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @builtin(frag_depth) f32 {
    let r2 = dot(in.uv, in.uv);
    if (r2 > 1.0) {
        discard;
    }
    
    // Compute sphere surface offset in view space.
    // z_norm = how far the surface protrudes toward camera along the view axis.
    let z_norm = sqrt(1.0 - r2);
    
    // Surface point in view space: X,Y offset by uv * radius in the billboard
    // plane (right/up), Z offset by +z_norm * radius (toward camera, i.e.,
    // less negative in back-direction space).
    let surface_view = in.view_pos + vec3<f32>(
        in.uv.x * in.radius,
        in.uv.y * in.radius,
        z_norm * in.radius
    );
    
    // Reconstruct world-space position from view-space coordinates.
    let cam_right = camera.view_inv[0].xyz;
    let cam_up    = camera.view_inv[1].xyz;
    let cam_back  = camera.view_inv[2].xyz;
    let surface_world = camera.cam_pos.xyz
        + cam_right * surface_view.x
        + cam_up    * surface_view.y
        + cam_back  * surface_view.z;
    
    // Re-project to get correct clip-space depth.
    let surface_clip = camera.view_proj * vec4<f32>(surface_world, 1.0);
    return surface_clip.z / surface_clip.w;
}
