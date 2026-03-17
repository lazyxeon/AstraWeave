// Scatter Object Shader
//
// GPU-instanced rendering for procedurally scattered vegetation, rocks, and props.
// Supports per-instance transforms, vertex colors, wind animation, LOD fade,
// and fog integration matching the terrain pipeline.

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_enabled: u32,
    wind_strength: f32,
    wind_frequency: f32,
    cull_distance: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
}

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
    @location(7) tint: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) fog_factor: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Simple hash for per-instance wind phase variation
fn hash_position(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p.xz, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var local_pos = vertex.position;

    // Wind animation: sway vertices based on height above ground.
    // Only apply to top vertices (y > 0.1) so base stays planted.
    if uniforms.wind_strength > 0.0 && local_pos.y > 0.1 {
        let world_origin = model_matrix[3].xyz;
        let phase = hash_position(world_origin);
        let height_factor = saturate(local_pos.y); // More sway at top
        let wind_time = uniforms.time * uniforms.wind_frequency + phase * 6.283;
        let sway = sin(wind_time) * uniforms.wind_strength * height_factor * 0.15;
        let gust = sin(wind_time * 0.37 + 1.7) * uniforms.wind_strength * height_factor * 0.05;
        local_pos.x += sway;
        local_pos.z += gust;
    }

    let world_position = model_matrix * vec4<f32>(local_pos, 1.0);
    let world_normal = normalize((model_matrix * vec4<f32>(vertex.normal, 0.0)).xyz);

    // Distance-based LOD fade
    let dist = length(world_position.xyz - uniforms.camera_pos);
    let fade_start = uniforms.cull_distance * 0.85;
    let fade = 1.0 - saturate((dist - fade_start) / (uniforms.cull_distance - fade_start));

    // Fog calculation
    var fog_factor = 0.0;
    if uniforms.fog_enabled > 0u {
        fog_factor = 1.0 - exp(-uniforms.fog_density * dist * dist);
        fog_factor = saturate(fog_factor);
    }

    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * world_position;
    output.world_position = world_position.xyz;
    output.world_normal = world_normal;
    // Instance tint multiplied by vertex color; alpha carries LOD fade
    output.color = vertex.vertex_color * instance.tint * vec4<f32>(1.0, 1.0, 1.0, fade);
    output.fog_factor = fog_factor;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Alpha discard for LOD fade-out
    if in.color.a < 0.02 {
        discard;
    }

    // Directional lighting (matching entity shader)
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.35;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.65;
    let lighting = ambient + diffuse;
    var lit_color = in.color.rgb * lighting;

    // Fog blending
    lit_color = mix(lit_color, uniforms.fog_color, in.fog_factor);

    return vec4<f32>(lit_color, in.color.a);
}
