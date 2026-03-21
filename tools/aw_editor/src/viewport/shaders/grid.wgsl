// Infinite Grid Shader
//
// Renders an infinite grid overlay on the ground plane using a screen-space technique.
// No vertex buffers needed - renders a fullscreen quad and computes grid in fragment shader.
//
// Features:
// - Infinite grid (no visible edges)
// - Distance-based fading (prevents aliasing at horizon)
// - Major/minor grid lines (1m minor, 10m major)
// - XZ axes highlighted (red X, blue Z)
//
// Performance: ~0.5ms for 1080p

// Uniforms (camera matrices + grid settings)
struct GridUniforms {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,  // For unprojecting screen pos
    camera_pos: vec3<f32>,
    _padding1: f32,
    grid_size: f32,              // Grid spacing (meters)
    major_grid_every: f32,       // Major grid every N lines
    fade_distance: f32,          // Start fading at this distance
    max_distance: f32,           // Completely fade by this distance
    grid_color: vec4<f32>,       // Base grid color (RGBA)
    major_grid_color: vec4<f32>, // Major grid color (RGBA)
    x_axis_color: vec4<f32>,     // X axis color (red)
    z_axis_color: vec4<f32>,     // Z axis color (blue)
};

@group(0) @binding(0)
var<uniform> uniforms: GridUniforms;

// Vertex shader (fullscreen quad)
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) near_point: vec3<f32>,  // Unprojected near plane point
    @location(1) far_point: vec3<f32>,   // Unprojected far plane point
};

// Fullscreen quad vertices (NDC coordinates)
const QUAD_VERTICES = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),  // Bottom-left
    vec2<f32>( 1.0, -1.0),  // Bottom-right
    vec2<f32>(-1.0,  1.0),  // Top-left
    vec2<f32>(-1.0,  1.0),  // Top-left
    vec2<f32>( 1.0, -1.0),  // Bottom-right
    vec2<f32>( 1.0,  1.0),  // Top-right
);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let pos = QUAD_VERTICES[vertex_index];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    
    // Unproject to camera-relative space (near and far plane)
    let near = uniforms.inv_view_proj * vec4<f32>(pos, -1.0, 1.0);
    let far = uniforms.inv_view_proj * vec4<f32>(pos, 1.0, 1.0);
    
    out.near_point = near.xyz / near.w;
    out.far_point = far.xyz / far.w;
    
    return out;
}

// Fragment shader (compute grid)
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ray from camera through fragment (camera-relative, small values)
    let ray_dir = normalize(in.far_point - in.near_point);
    
    // Intersect ray with Y=0 plane (relative to camera)
    // Camera-relative near_point.y = near_point_world.y - camera_pos.y
    // We need t such that near_point.y + camera_pos.y + ray_dir.y * t = 0
    let t = -(in.near_point.y + uniforms.camera_pos.y) / ray_dir.y;
    
    // Discard if ray doesn't hit ground plane or hits behind camera
    if t < 0.0 {
        discard;
    }
    
    // Camera-relative intersection point (high precision)
    let rel_pos = in.near_point + ray_dir * t;
    
    // Distance from camera (camera-relative — just length of rel_pos, no subtraction needed)
    let distance = length(rel_pos);
    
    // Fade grid at distance (prevent aliasing)
    if distance > uniforms.max_distance {
        discard;
    }
    
    // World position (for grid pattern and axis detection)
    let world_pos = rel_pos + uniforms.camera_pos;
    
    // LOD: smoothly blend between grid scales based on camera height
    let cam_height = abs(uniforms.camera_pos.y);
    let lod_size = uniforms.grid_size;
    
    // Two-level grid: minor (1m) fades out at height, major (10m) always visible
    // This avoids blur by keeping crisp lines at all zoom levels
    
    // --- Minor grid (1m spacing) ---
    let minor_coord = world_pos.xz / lod_size;
    let minor_fract = fract(minor_coord);
    let minor_deriv = fwidth(minor_coord);
    // Clamp derivative to prevent overly thick/blurry lines at distance
    let minor_width = clamp(minor_deriv * 1.5, vec2<f32>(0.01), vec2<f32>(0.45));
    let minor_line = smoothstep(minor_width, vec2<f32>(0.0), abs(minor_fract - 0.5) - 0.5 + minor_width);
    let minor_raw = max(minor_line.x, minor_line.y);
    // Fade minor grid when zoomed out (lines would alias)
    let minor_fade = 1.0 - smoothstep(30.0, 80.0, cam_height);
    let minor_alpha = minor_raw * minor_fade;
    
    // --- Major grid (10m spacing) ---
    let major_coord = world_pos.xz / (lod_size * uniforms.major_grid_every);
    let major_fract = fract(major_coord);
    let major_deriv = fwidth(major_coord);
    let major_width = clamp(major_deriv * 1.5, vec2<f32>(0.01), vec2<f32>(0.45));
    let major_line = smoothstep(major_width, vec2<f32>(0.0), abs(major_fract - 0.5) - 0.5 + major_width);
    let major_alpha = max(major_line.x, major_line.y);
    
    // --- Super-major grid (100m spacing, for very far zoom) ---
    let super_coord = world_pos.xz / (lod_size * uniforms.major_grid_every * 10.0);
    let super_fract = fract(super_coord);
    let super_deriv = fwidth(super_coord);
    let super_width = clamp(super_deriv * 1.5, vec2<f32>(0.01), vec2<f32>(0.45));
    let super_line = smoothstep(super_width, vec2<f32>(0.0), abs(super_fract - 0.5) - 0.5 + super_width);
    let super_raw = max(super_line.x, super_line.y);
    // Fade in super grid only when zoomed far out
    let super_fade = smoothstep(60.0, 150.0, cam_height);
    let super_alpha = super_raw * super_fade;
    
    // Axes (X and Z) — scale width with camera height for visibility
    let axis_width = max(lod_size * 0.05, cam_height * 0.002);
    let x_axis_alpha = smoothstep(0.0, axis_width, abs(world_pos.z));
    let z_axis_alpha = smoothstep(0.0, axis_width, abs(world_pos.x));
    
    // Distance fade
    let fade = 1.0 - smoothstep(uniforms.fade_distance, uniforms.max_distance, distance);
    
    // Ground plane base color (opaque dark surface so grid lines stand out)
    let ground_base_color = vec4<f32>(0.12, 0.12, 0.14, 0.85);
    
    // Combine: Ground base → Minor grid → Major grid → Super grid → Axes
    var color = ground_base_color;
    color = mix(color, uniforms.grid_color, minor_alpha);
    color = mix(color, uniforms.major_grid_color, major_alpha);
    color = mix(color, uniforms.major_grid_color, super_alpha);
    color = mix(color, uniforms.x_axis_color, (1.0 - x_axis_alpha) * uniforms.x_axis_color.a);
    color = mix(color, uniforms.z_axis_color, (1.0 - z_axis_alpha) * uniforms.z_axis_color.a);
    
    // Apply distance fade
    color.a *= fade;
    
    // Discard fully transparent pixels (optimization)
    if color.a < 0.01 {
        discard;
    }
    
    return color;
}
