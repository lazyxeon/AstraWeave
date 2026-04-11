// Vegetation Billboard Shader — LOD2 (cross-billboard) and LOD3 (impostor card)
//
// Cross-billboard geometry (two quads at 90°) does NOT rotate to face
// the camera — the alpha-test silhouette provides view-dependent coverage.
//
// Impostor cards rotate around Y to face the camera (cylindrical billboard).
// The fragment shader selects the closest atlas angle for the texture lookup.

// ── Uniforms ────────────────────────────────────────────────────────────────

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
}
@group(0) @binding(0) var<uniform> uCamera: CameraUniforms;

struct WindUniforms {
    wind_dir_strength_time: vec4<f32>,
    sway_params: vec4<f32>,
}
@group(1) @binding(0) var<uniform> uWind: WindUniforms;

// Billboard / impostor atlas
@group(2) @binding(0) var atlas_tex: texture_2d<f32>;
@group(2) @binding(1) var atlas_samp: sampler;

struct SceneEnv {
    fog_color_density: vec4<f32>,
    fog_range_pad: vec4<f32>,
    ambient_color_intensity: vec4<f32>,
    tint_color_alpha: vec4<f32>,
    blend_pad: vec4<f32>,
    sun_color_intensity: vec4<f32>,
};
@group(3) @binding(0) var<uniform> uScene: SceneEnv;

struct LodDistances {
    // x = lod0_max, y = lod1_max, z = lod2_max, w = cull_distance
    thresholds: vec4<f32>,
}
@group(3) @binding(1) var<uniform> uLod: LodDistances;

// Per-species atlas info: base_uv = (u_min, v_min, u_max, v_max) of angle 0.
// To access angle `i`, offset u by i * cell_width.
struct AtlasInfo {
    base_uv: vec4<f32>,
}
@group(3) @binding(2) var<storage, read> atlas_species: array<AtlasInfo>;

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
    @location(3) species_idx: f32,
    @location(4) cam_angle: f32,
}

// ── Cross-billboard vertex shader ───────────────────────────────────────────
//
// Cross-billboards do NOT billboard-face the camera; they rely on the
// X-shaped geometry. Wind sway is applied with reduced amplitude.

@vertex
fn vs_cross_billboard(input: VSIn) -> VSOut {
    let inst_pos = input.inst_pos_scale.xyz;
    let inst_scale = input.inst_pos_scale.w;
    let inst_rotation = input.inst_rot_type_normal.x;

    let cos_r = cos(inst_rotation);
    let sin_r = sin(inst_rotation);
    let s = inst_scale;

    let model = mat4x4<f32>(
        vec4<f32>( cos_r * s, 0.0, sin_r * s, 0.0),
        vec4<f32>( 0.0,       s,   0.0,       0.0),
        vec4<f32>(-sin_r * s, 0.0, cos_r * s, 0.0),
        vec4<f32>( inst_pos.x, inst_pos.y, inst_pos.z, 1.0),
    );

    var world_pos = (model * vec4<f32>(input.position, 1.0)).xyz;

    // Simplified trunk sway for billboard (no leaf flutter)
    let wind_dir = uWind.wind_dir_strength_time.xy;
    let wind_strength = uWind.wind_dir_strength_time.z;
    let time = uWind.wind_dir_strength_time.w;
    let trunk_amp = uWind.sway_params.x * 0.5; // halved for billboards
    let trunk_freq = uWind.sway_params.y;
    let height_factor = saturate(input.position.y / max(inst_scale, 0.01));
    let trunk_phase = dot(inst_pos.xz, vec2<f32>(0.7, 0.3));
    let sway = sin(time * trunk_freq + trunk_phase) * wind_strength * trunk_amp * height_factor;
    world_pos.x += sway * wind_dir.x;
    world_pos.z += sway * wind_dir.y;

    // Camera angle for atlas lookup
    let to_cam = uCamera.camera_pos.xyz - inst_pos;
    let cam_angle = atan2(to_cam.z, to_cam.x);

    var out: VSOut;
    out.pos = uCamera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    out.uv = input.uv;
    out.species_idx = input.inst_rot_type_normal.y;
    out.cam_angle = cam_angle;
    return out;
}

// ── Impostor card vertex shader ─────────────────────────────────────────────
//
// The impostor quad is rotated around Y to face the camera (cylindrical
// billboarding).

@vertex
fn vs_impostor(input: VSIn) -> VSOut {
    let inst_pos = input.inst_pos_scale.xyz;
    let inst_scale = input.inst_pos_scale.w;

    let to_cam = uCamera.camera_pos.xyz - inst_pos;
    let angle = atan2(to_cam.z, to_cam.x) - 1.5707963; // face camera

    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let s = inst_scale;

    let model = mat4x4<f32>(
        vec4<f32>( cos_a * s, 0.0, sin_a * s, 0.0),
        vec4<f32>( 0.0,       s,   0.0,       0.0),
        vec4<f32>(-sin_a * s, 0.0, cos_a * s, 0.0),
        vec4<f32>( inst_pos.x, inst_pos.y, inst_pos.z, 1.0),
    );

    var world_pos = (model * vec4<f32>(input.position, 1.0)).xyz;

    // Very light trunk sway
    let wind_dir = uWind.wind_dir_strength_time.xy;
    let wind_strength = uWind.wind_dir_strength_time.z;
    let time = uWind.wind_dir_strength_time.w;
    let trunk_amp = uWind.sway_params.x * 0.25; // quarter amplitude for impostors
    let trunk_freq = uWind.sway_params.y;
    let height_factor = saturate(input.position.y / max(inst_scale, 0.01));
    let trunk_phase = dot(inst_pos.xz, vec2<f32>(0.7, 0.3));
    let sway = sin(time * trunk_freq + trunk_phase) * wind_strength * trunk_amp * height_factor;
    world_pos.x += sway * wind_dir.x;
    world_pos.z += sway * wind_dir.y;

    let cam_angle = atan2(to_cam.z, to_cam.x);

    var out: VSOut;
    out.pos = uCamera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.normal = normalize(to_cam * vec3<f32>(1.0, 0.0, 1.0)); // horizontal normal facing camera
    out.uv = input.uv;
    out.species_idx = input.inst_rot_type_normal.y;
    out.cam_angle = cam_angle;
    return out;
}

// ── Shared fragment shader ──────────────────────────────────────────────────
//
// Selects the closest atlas angle using cam_angle, then samples the atlas
// texture. Alpha test for tree silhouette.

const ANGLE_COUNT: f32 = 8.0;
const INV_ANGLE_COUNT: f32 = 0.125; // 1.0 / 8.0

@fragment
fn fs_billboard(input: VSOut) -> @location(0) vec4<f32> {
    let species = u32(input.species_idx);

    // Get atlas base UV for this species
    let info = atlas_species[species];
    let u_min = info.base_uv.x;
    let v_min = info.base_uv.y;
    let u_max = info.base_uv.z;
    let v_max = info.base_uv.w;

    // Select angle index from camera angle
    let angle_step = 6.2831853 / ANGLE_COUNT; // TAU / 8
    let a = ((input.cam_angle % 6.2831853) + 6.2831853) % 6.2831853; // normalise to [0, TAU)
    let angle_idx = u32((a / angle_step + 0.5)) % u32(ANGLE_COUNT);

    // Compute per-angle UV within the species row
    let cell_width = (u_max - u_min) * INV_ANGLE_COUNT;
    // Note: u_max - u_min covers the full row (all angles).
    // We want the first cell's width = (u_max - u_min) / angle_count.
    // Wait — the atlas stores angle_count cells from u_min to u_max, so
    // each cell spans cell_width = (u_max - u_min) / ANGLE_COUNT.
    // The cell for angle_idx starts at u_min + angle_idx * cell_width.
    let actual_cell_width = (u_max - u_min) / ANGLE_COUNT;
    let cell_u_min = u_min + f32(angle_idx) * actual_cell_width;
    let cell_u_max = cell_u_min + actual_cell_width;

    // Map input UV [0,1] → atlas cell UV
    let atlas_u = mix(cell_u_min, cell_u_max, input.uv.x);
    let atlas_v = mix(v_min, v_max, input.uv.y);

    let albedo = textureSample(atlas_tex, atlas_samp, vec2<f32>(atlas_u, atlas_v));

    // Alpha test
    if (albedo.a < 0.5) {
        discard;
    }

    // Simple directional lighting (same as vegetation.wgsl)
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
