// Vegetation Rendering Shader — Instanced PBR with Wind Animation
//
// Receives per-instance data from the GPU scatter pipeline:
//   location 9:  pos_scale  (vec4: world_x, world_y, world_z, scale)
//   location 10: rot_type_normal (vec4: rotation_rad, type_index, normal_x, normal_y)
//
// Wind model (two-layer):
//   - Trunk sway:  low-frequency sine, large amplitude, driven by instance hash
//   - Leaf flutter: high-frequency sine, amplitude modulated by vertex height
//
// Both layers are coherent across the landscape via spatial phase offsets.

// ── Uniforms ────────────────────────────────────────────────────────────────

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}
@group(0) @binding(0) var<uniform> uCamera: CameraUniforms;

struct WindUniforms {
    // xy = wind direction (normalised XZ), z = wind strength, w = time (seconds)
    wind_dir_strength_time: vec4<f32>,
    // x = trunk_sway_amplitude, y = trunk_sway_frequency
    // z = leaf_flutter_amplitude, w = leaf_flutter_frequency
    sway_params: vec4<f32>,
}
@group(1) @binding(0) var<uniform> uWind: WindUniforms;

// PBR material textures
@group(2) @binding(0) var albedo_tex: texture_2d<f32>;
@group(2) @binding(1) var albedo_samp: sampler;

// Scene environment
struct SceneEnv {
    fog_color_density: vec4<f32>,
    fog_range_pad: vec4<f32>,
    ambient_color_intensity: vec4<f32>,
    tint_color_alpha: vec4<f32>,
    blend_pad: vec4<f32>,
    sun_color_intensity: vec4<f32>,
};
@group(3) @binding(0) var<uniform> uScene: SceneEnv;

// ── Vertex I/O ──────────────────────────────────────────────────────────────

struct VSIn {
    // Per-vertex
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    // Per-instance (from VegetationInstanceGpu)
    @location(9) inst_pos_scale: vec4<f32>,
    @location(10) inst_rot_type_normal: vec4<f32>,
}

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// ── Wind displacement ───────────────────────────────────────────────────────

fn apply_wind(
    local_pos: vec3<f32>,
    instance_world_pos: vec3<f32>,
    wind_dir: vec2<f32>,
    wind_strength: f32,
    time: f32,
    trunk_amp: f32,
    trunk_freq: f32,
    leaf_amp: f32,
    leaf_freq: f32,
) -> vec3<f32> {
    // Height factor: wind effect increases with vertical distance from root
    let height_factor = saturate(local_pos.y);  // assume Y-up, model root at y≈0

    // ── Trunk sway (low frequency, coherent across instance) ──────────────
    // Phase offset from world position for spatial variation
    let trunk_phase = dot(instance_world_pos.xz, vec2<f32>(0.7, 0.3));
    let trunk_sway = sin(time * trunk_freq + trunk_phase)
                   * wind_strength * trunk_amp * height_factor;

    // ── Leaf flutter (high frequency, per-vertex variation) ───────────────
    // Each vertex gets a unique phase based on its local position
    let leaf_phase = local_pos.y * 3.14159 + local_pos.x * 2.71828 + instance_world_pos.x;
    let leaf_flutter = sin(time * leaf_freq + leaf_phase)
                     * wind_strength * leaf_amp * height_factor * height_factor;

    // Combine displacements along wind direction (XZ plane)
    let total = trunk_sway + leaf_flutter;
    return vec3<f32>(
        total * wind_dir.x,
        0.0,  // no vertical displacement
        total * wind_dir.y,
    );
}

// ── Vertex Main ─────────────────────────────────────────────────────────────

@vertex
fn vs_main(input: VSIn) -> VSOut {
    // Unpack instance data
    let inst_pos = input.inst_pos_scale.xyz;
    let inst_scale = input.inst_pos_scale.w;
    let inst_rotation = input.inst_rot_type_normal.x;  // radians around Y

    // Build instance transform: scale → rotate Y → translate
    let cos_r = cos(inst_rotation);
    let sin_r = sin(inst_rotation);
    let s = inst_scale;

    // Rotation around Y axis (column-major)
    let model = mat4x4<f32>(
        vec4<f32>( cos_r * s, 0.0, sin_r * s, 0.0),
        vec4<f32>( 0.0,       s,   0.0,       0.0),
        vec4<f32>(-sin_r * s, 0.0, cos_r * s, 0.0),
        vec4<f32>( inst_pos.x, inst_pos.y, inst_pos.z, 1.0),
    );

    // Transform to world space
    var world_pos = (model * vec4<f32>(input.position, 1.0)).xyz;

    // Apply wind displacement
    let wind_dir = uWind.wind_dir_strength_time.xy;
    let wind_strength = uWind.wind_dir_strength_time.z;
    let time = uWind.wind_dir_strength_time.w;
    let trunk_amp = uWind.sway_params.x;
    let trunk_freq = uWind.sway_params.y;
    let leaf_amp = uWind.sway_params.z;
    let leaf_freq = uWind.sway_params.w;

    world_pos += apply_wind(
        input.position,
        inst_pos,
        wind_dir,
        wind_strength,
        time,
        trunk_amp, trunk_freq,
        leaf_amp, leaf_freq,
    );

    // Transform normal to world space (rotation only, no translation)
    let world_normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);

    var out: VSOut;
    out.pos = uCamera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.normal = world_normal;
    out.uv = input.uv;
    return out;
}

// ── Fragment Main ───────────────────────────────────────────────────────────

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    // Sample albedo
    let albedo = textureSample(albedo_tex, albedo_samp, input.uv);

    // Alpha test for foliage transparency
    if (albedo.a < 0.5) {
        discard;
    }

    // Simple directional lighting
    let sun_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let sun_color = uScene.sun_color_intensity.xyz * uScene.sun_color_intensity.w;
    let ambient = uScene.ambient_color_intensity.xyz * uScene.ambient_color_intensity.w;

    let N = normalize(input.normal);
    let NdotL = max(dot(N, sun_dir), 0.0);

    let diffuse = albedo.rgb * (sun_color * NdotL + ambient);

    // Distance fog
    let dist = length(input.world_pos - uCamera.camera_pos.xyz);
    let fog_start = uScene.fog_range_pad.x;
    let fog_end = uScene.fog_range_pad.y;
    let fog_factor = saturate((dist - fog_start) / (fog_end - fog_start + 0.001));
    let fog_color = uScene.fog_color_density.xyz;

    let final_color = mix(diffuse, fog_color, fog_factor * uScene.fog_color_density.w);

    return vec4<f32>(final_color, 1.0);
}
