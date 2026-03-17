// ============================================================================
// AstraWeave Volumetric Rain Shader
// ============================================================================
// GPU-instanced rain streaks with camera-relative positioning and wind drift.

struct RainUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    rain_intensity: f32,
    wind_x: f32,
    wind_z: f32,
    _pad: f32,
}

struct VertexInput {
    // Per-vertex (line endpoint: 0.0 = top, 1.0 = bottom)
    @location(0) local_y: f32,
    // Per-instance
    @location(1) inst_pos: vec3<f32>,
    @location(2) inst_speed: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
}

@group(0) @binding(0)
var<uniform> u: RainUniforms;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    let cycle_h = 80.0;
    let fall_offset = u.time * vertex.inst_speed;

    // Position relative to camera (rain volume follows camera)
    var world_pos = vertex.inst_pos + u.camera_pos;
    // Cycle vertically: drops wrap around the volume
    world_pos.y = world_pos.y - fract(fall_offset / cycle_h) * cycle_h;

    // Wind drift (stronger at top of rain volume)
    let height_frac = clamp((world_pos.y - u.camera_pos.y + cycle_h * 0.5) / cycle_h, 0.0, 1.0);
    world_pos.x += u.wind_x * (1.0 - height_frac) * 3.0;
    world_pos.z += u.wind_z * (1.0 - height_frac) * 3.0;

    // Streak direction (gravity + wind)
    let streak_dir = normalize(vec3<f32>(u.wind_x * 0.15, -1.0, u.wind_z * 0.15));
    let streak_len = 0.3 + vertex.inst_speed * 0.04;
    world_pos += streak_dir * vertex.local_y * streak_len;

    var output: VertexOutput;
    output.clip_position = u.view_proj * vec4<f32>(world_pos, 1.0);

    // Alpha: fade endpoints, fade with distance
    let dist = distance(u.camera_pos, world_pos);
    let dist_fade = 1.0 - smoothstep(30.0, 60.0, dist);
    let endpoint_fade = 1.0 - abs(vertex.local_y * 2.0 - 1.0);
    output.alpha = endpoint_fade * dist_fade * u.rain_intensity * 0.35;

    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.7, 0.75, 0.85, in.alpha);
}
