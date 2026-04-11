// Procedural Grass Blade Rendering
//
// Each grass blade is a 3-vertex triangle (bottom-left, bottom-right, tip).
// Per-instance data provides position, facing, dimensions, and color.
// Wind animation uses world-position-phase sinusoidal displacement.
//
// Dispatch: draw 3 vertices per instance, instanced count = blade count.

struct CameraUniforms {
    view_proj:    mat4x4<f32>,
    light_dir:    vec4<f32>,
    camera_pos:   vec4<f32>,
}

struct GrassParams {
    time:             f32,
    wind_strength:    f32,
    wind_dir_x:       f32,
    wind_dir_z:       f32,
    interaction_radius: f32,
    _pad0:            f32,
    _pad1:            f32,
    _pad2:            f32,
}

// Per-instance blade data (32 bytes, packed).
struct GrassInstance {
    pos_height: vec4<f32>,   // xyz = world position, w = blade height
    dir_width:  vec4<f32>,   // xy = facing direction (normalised XZ), z = blade width, w = tint
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> params: GrassParams;
@group(0) @binding(2) var<storage, read> blades: array<GrassInstance>;

// Optional interaction stamp texture (R8, world-space top-down around camera).
@group(1) @binding(0) var interaction_tex: texture_2d<f32>;
@group(1) @binding(1) var interaction_samp: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos:  vec3<f32>,
    @location(1) normal:     vec3<f32>,
    @location(2) color:      vec3<f32>,
    @location(3) ao:         f32,
}

// Hash for per-blade variation (position-based, deterministic).
fn blade_hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let blade = blades[iid];
    let base_pos = blade.pos_height.xyz;
    let height = blade.pos_height.w;
    let facing = normalize(blade.dir_width.xy);
    let width = blade.dir_width.z;
    let tint_packed = blade.dir_width.w;

    // Perpendicular to facing direction (XZ plane).
    let perp = vec2<f32>(-facing.y, facing.x);

    // Per-blade variation from position hash.
    let h = blade_hash(base_pos.xz);
    let height_var = height * (0.8 + 0.4 * h);   // ±20% height variation
    let width_var = width * (0.85 + 0.3 * h);    // ±15% width variation
    let bend_phase = h * 6.283185;                // unique phase per blade

    // Wind displacement (applied to tip only).
    let wind_dir = vec2<f32>(params.wind_dir_x, params.wind_dir_z);
    let wind_phase = dot(base_pos.xz, wind_dir) * 0.1 + params.time * 2.5 + bend_phase;
    let wind_bend = sin(wind_phase) * params.wind_strength * 0.15;
    let wind_gust = sin(wind_phase * 0.37 + 1.7) * params.wind_strength * 0.05;

    // Interaction stamp sampling (bends grass away from entities).
    let interact_uv = (base_pos.xz - camera.camera_pos.xz) / (params.interaction_radius * 2.0) + 0.5;
    var interact_bend = vec2<f32>(0.0);
    if (interact_uv.x >= 0.0 && interact_uv.x <= 1.0 && interact_uv.y >= 0.0 && interact_uv.y <= 1.0) {
        let stamp = textureSampleLevel(interaction_tex, interaction_samp, interact_uv, 0.0).r;
        // Bend away from camera/entity position.
        let away_dir = normalize(base_pos.xz - camera.camera_pos.xz + vec2<f32>(0.001));
        interact_bend = away_dir * stamp * 0.3;
    }

    // Vertex positions: 0 = bottom-left, 1 = bottom-right, 2 = tip.
    var local_offset: vec3<f32>;
    var vertex_ao: f32;

    switch (vid) {
        case 0u: {
            // Bottom-left: grounded.
            local_offset = vec3<f32>(
                perp.x * width_var * -0.5,
                0.0,
                perp.y * width_var * -0.5,
            );
            vertex_ao = 0.4; // darker at base (ambient occlusion)
        }
        case 1u: {
            // Bottom-right: grounded.
            local_offset = vec3<f32>(
                perp.x * width_var * 0.5,
                0.0,
                perp.y * width_var * 0.5,
            );
            vertex_ao = 0.4;
        }
        default: {
            // Tip: elevated, displaced by wind + interaction.
            let tip_wind = vec3<f32>(
                wind_dir.x * wind_bend + interact_bend.x,
                0.0,
                wind_dir.y * wind_bend + wind_gust + interact_bend.y,
            );
            local_offset = vec3<f32>(0.0, height_var, 0.0) + tip_wind;
            vertex_ao = 1.0; // bright at tip
        }
    }

    let world_pos = base_pos + local_offset;
    out.world_pos = world_pos;
    out.clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);

    // Approximate normal: cross product of blade edges.
    let edge_up = vec3<f32>(0.0, height_var, 0.0) - vec3<f32>(perp.x * width_var, 0.0, perp.y * width_var);
    let edge_side = vec3<f32>(perp.x * width_var, 0.0, perp.y * width_var);
    out.normal = normalize(cross(edge_side, edge_up));

    // Color: green base with tint variation.
    let base_green = vec3<f32>(0.15, 0.45, 0.08);
    let dry_yellow = vec3<f32>(0.55, 0.50, 0.15);
    let tint_factor = fract(tint_packed);
    out.color = mix(base_green, dry_yellow, tint_factor * 0.4);
    out.ao = vertex_ao;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional + ambient lighting.
    let light_dir = normalize(camera.light_dir.xyz);
    let ndl = max(dot(input.normal, light_dir), 0.0);

    // Two-sided lighting: grass blades are thin, light passes through.
    let back_ndl = max(dot(-input.normal, light_dir), 0.0) * 0.3;

    let ambient = 0.2;
    let diffuse = (ndl + back_ndl) * 0.8;
    let lit = input.color * (ambient + diffuse) * input.ao;

    return vec4<f32>(lit, 1.0);
}
