// Scatter Object Shader
//
// GPU-instanced rendering for procedurally scattered vegetation, rocks, and props.
// Supports per-instance transforms, vertex colors, wind animation, dithered LOD fade,
// and fog integration matching the terrain pipeline.
//
// LOD fade uses screen-door (ordered dithering) transparency instead of alpha blending.
// This prevents depth-write conflicts that cause objects to phase in/out when
// semi-transparent fragments block objects behind them in the depth buffer.

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
    // Lighting uniforms (synced from terrain)
    sun_dir: vec3<f32>,
    sun_intensity: f32,
    sun_color: vec3<f32>,
    ambient_intensity: f32,
    ambient_color: vec3<f32>,
    exposure: f32,
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
    @location(4) lod_fade: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Simple hash for per-instance wind phase variation
fn hash_position(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p.xz, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// 4x4 Bayer ordered dithering threshold for screen-door transparency.
// Produces a regular pattern that, at normal viewing distances, reads as a smooth fade.
fn dither_threshold(screen_pos: vec2<f32>) -> f32 {
    let x = u32(screen_pos.x) % 4u;
    let y = u32(screen_pos.y) % 4u;
    // Bayer 4x4 matrix values (row-major), normalized to [0..1)
    var m = array<f32, 16>(
         0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0,
        12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0,
         3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0,
        15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0
    );
    return m[y * 4u + x];
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

    // Fog calculation — height-aware to match terrain shader
    let dist = length(world_position.xyz - uniforms.camera_pos);
    var fog_factor = 0.0;
    if uniforms.fog_enabled > 0u {
        let height_att = saturate(world_position.y / 200.0);
        let fog_mult = mix(0.7, 0.35, height_att);
        fog_factor = 1.0 - exp(-uniforms.fog_density * dist);
        fog_factor = clamp(fog_factor * fog_mult, 0.0, 0.65);
    }

    var output: VertexOutput;
    // Camera-relative transform: subtract camera_pos to avoid f32 jitter far from origin
    let rel_pos = world_position.xyz - uniforms.camera_pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(rel_pos, 1.0);
    output.world_position = world_position.xyz;
    output.world_normal = world_normal;
    output.color = vertex.vertex_color * instance.tint;

    // Per-vertex color variation to break flat solid-color appearance.
    // Darkens base (ground-contact AO), lightens tips, adds subtle noise.
    let local_y = saturate(vertex.position.y);                 // 0 at base, ~1 at top
    let ao = mix(0.65, 1.0, local_y);                         // 35% darker at base
    let tip_boost = smoothstep(0.6, 1.0, local_y) * 0.12;    // Slight lighten at tips
    // Use fixed instance origin (not wind-displaced position) so noise is stable per-frame
    let noise = (hash_position(model_matrix[3].xyz * 3.7) - 0.5) * 0.08; // ±4% per-instance scatter
    output.color = vec4<f32>(
        clamp(output.color.rgb * ao + tip_boost + noise, vec3<f32>(0.0), vec3<f32>(1.0)),
        output.color.a
    );
    output.fog_factor = fog_factor;
    output.lod_fade = 0.0;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Dynamic directional lighting from terrain sun uniforms
    let light_dir = normalize(uniforms.sun_dir);
    let n = normalize(in.world_normal);
    let ndotl = max(dot(n, light_dir), 0.0);

    // Diffuse: sun color × intensity × NdotL
    let diffuse = uniforms.sun_color * uniforms.sun_intensity * ndotl;
    // Ambient: ambient color × intensity
    let ambient = uniforms.ambient_color * uniforms.ambient_intensity;
    // Combine lighting
    var lit_color = in.color.rgb * (diffuse + ambient);

    // Apply exposure and simple Reinhard tone mapping
    lit_color = lit_color * uniforms.exposure;
    lit_color = lit_color / (lit_color + vec3<f32>(1.0));

    // Distance-based alpha fade (smooth, no discard — shader handles far objects gracefully)
    let dist = length(in.world_position.xyz - uniforms.camera_pos);
    let fade_start = uniforms.cull_distance * 0.85;
    let fade_end = uniforms.cull_distance;
    let alpha = 1.0 - saturate((dist - fade_start) / (fade_end - fade_start));
    if alpha < 0.01 {
        discard;
    }

    // Fog blending
    lit_color = mix(lit_color, uniforms.fog_color, in.fog_factor);

    return vec4<f32>(lit_color, 1.0);
}
