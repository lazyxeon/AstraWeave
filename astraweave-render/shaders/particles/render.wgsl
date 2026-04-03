// Particle Billboard Rendering — Camera-facing instanced quads
//
// Each particle expands into a screen-aligned billboard quad.
// Supports additive and alpha blending modes.
// Soft particles: fade near depth buffer intersections.

struct CameraUniforms {
    view_proj:   mat4x4<f32>,
    camera_pos:  vec3<f32>,
    _pad0:       f32,
    camera_right: vec3<f32>,
    _pad1:       f32,
    camera_up:   vec3<f32>,
    near_plane:  f32,
    inv_resolution: vec2<f32>,
    soft_depth_range: f32,    // depth range for soft particle fade
    _pad2:       f32,
};

struct Particle {
    pos_life:  vec4<f32>,
    vel_age:   vec4<f32>,
    color:     vec4<f32>,
    size_mass: vec4<f32>,
};

struct SortEntry {
    index:    u32,
    distance: f32,
};

@group(0) @binding(0) var<uniform>         camera:    CameraUniforms;
@group(0) @binding(1) var<storage, read>   particles: array<Particle>;
@group(0) @binding(2) var<storage, read>   sort_indices: array<SortEntry>;
@group(0) @binding(3) var                  t_depth:   texture_2d<f32>;
@group(0) @binding(4) var                  t_texture: texture_2d<f32>;
@group(0) @binding(5) var                  s_linear:  sampler;

struct VertexOutput {
    @builtin(position)   position:  vec4<f32>,
    @location(0)         color:     vec4<f32>,
    @location(1)         uv:        vec2<f32>,
    @location(2)         world_pos: vec3<f32>,
};

// Billboard quad corners: 2 triangles forming a quad
// Vertex ID 0-5 maps to the 6 vertices of a quad
const QUAD_UVS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), // TL
    vec2<f32>(1.0, 0.0), // TR
    vec2<f32>(0.0, 1.0), // BL
    vec2<f32>(1.0, 0.0), // TR
    vec2<f32>(1.0, 1.0), // BR
    vec2<f32>(0.0, 1.0), // BL
);

const QUAD_OFFSETS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-0.5,  0.5), // TL
    vec2<f32>( 0.5,  0.5), // TR
    vec2<f32>(-0.5, -0.5), // BL
    vec2<f32>( 0.5,  0.5), // TR
    vec2<f32>( 0.5, -0.5), // BR
    vec2<f32>(-0.5, -0.5), // BL
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_id: u32,
    @builtin(instance_index) instance_id: u32,
) -> VertexOutput {
    // Look up sorted particle index
    let sorted = sort_indices[instance_id];
    let p = particles[sorted.index];

    let quad_vert = vertex_id % 6u;
    let offset = QUAD_OFFSETS[quad_vert];
    let uv = QUAD_UVS[quad_vert];

    let size = p.size_mass.x;
    let center = p.pos_life.xyz;

    // Billboard: expand quad in camera-aligned plane
    let world_pos = center
        + camera.camera_right * offset.x * size
        + camera.camera_up * offset.y * size;

    var out: VertexOutput;
    out.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = p.color;
    out.uv = uv;
    out.world_pos = world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample particle texture (circular falloff if no texture bound)
    let tex_color = textureSample(t_texture, s_linear, in.uv);

    // Circular soft edge (radial gradient)
    let center_dist = length(in.uv - vec2<f32>(0.5)) * 2.0;
    let circle_alpha = saturate(1.0 - center_dist * center_dist);

    var final_color = in.color * tex_color;
    final_color.a *= circle_alpha;

    // Soft particles: fade near depth buffer intersection
    let screen_uv = in.position.xy * camera.inv_resolution;
    let scene_depth = textureLoad(t_depth, vec2<i32>(in.position.xy), 0).r;
    let particle_depth = in.position.z;
    let depth_diff = abs(scene_depth - particle_depth);
    let soft_fade = saturate(depth_diff / camera.soft_depth_range);
    final_color.a *= soft_fade;

    // Discard fully transparent
    if (final_color.a < 0.001) {
        discard;
    }

    return final_color;
}
