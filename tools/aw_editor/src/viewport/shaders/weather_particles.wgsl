// ============================================================================
// AstraWeave Multi-Weather GPU Particle Shader
// ============================================================================
// Supports rain (lines), snow (quads), hail (quads), sandstorm (lines),
// blizzard (quads). Weather kind selects motion model and appearance.

struct WeatherUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    intensity: f32,
    wind_x: f32,
    wind_z: f32,
    weather_kind: f32,      // 0=none, 1=rain, 2=snow, 3=hail, 4=sandstorm, 5=blizzard
    particle_color: vec4<f32>,
    volume_size: f32,       // half-extent of particle volume
    streak_length: f32,     // for line particles
    particle_scale: f32,    // for quad particles
    transition_alpha: f32,  // 0..1 crossfade between weather states
}

struct VertexInput {
    @location(0) local_pos: vec2<f32>,   // vertex within particle (line: y=0/1, quad: corner xy)
    @location(1) inst_pos: vec3<f32>,    // instance origin offset
    @location(2) inst_speed: f32,        // fall/drift speed
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) alpha: f32,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> u: WeatherUniforms;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    let kind = u32(u.weather_kind);
    let vol = u.volume_size;
    let cycle_h = vol * 2.0;
    let fall_offset = u.time * vertex.inst_speed;

    // Base world position: camera-relative volume
    var world_pos = vertex.inst_pos + u.camera_pos;

    // Vertical cycling (wrap particles through volume)
    world_pos.y = world_pos.y - fract(fall_offset / cycle_h) * cycle_h;

    // Wind drift (stronger at top of volume)
    let height_frac = clamp((world_pos.y - u.camera_pos.y + vol) / cycle_h, 0.0, 1.0);
    let wind_strength = select(3.0, 1.5, kind == 2u || kind == 5u); // Snow/blizzard: less wind displacement
    world_pos.x += u.wind_x * (1.0 - height_frac) * wind_strength;
    world_pos.z += u.wind_z * (1.0 - height_frac) * wind_strength;

    var output: VertexOutput;
    output.uv = vertex.local_pos;

    // Weather-specific motion and vertex displacement
    if kind == 1u || kind == 4u {
        // === RAIN / SANDSTORM: Line particles ===
        let streak_dir = normalize(vec3<f32>(u.wind_x * 0.15, -1.0, u.wind_z * 0.15));
        let slen = u.streak_length + vertex.inst_speed * 0.04;
        world_pos += streak_dir * vertex.local_pos.y * slen;

        let dist = distance(u.camera_pos, world_pos);
        let dist_fade = 1.0 - smoothstep(u.volume_size * 0.4, u.volume_size * 0.85, dist);
        let endpoint_fade = 1.0 - abs(vertex.local_pos.y * 2.0 - 1.0);
        output.alpha = endpoint_fade * dist_fade * u.intensity * 0.35 * u.transition_alpha;

    } else if kind == 2u || kind == 3u || kind == 5u {
        // === SNOW / HAIL / BLIZZARD: Billboard quad particles ===
        // Tumble: snow has gentle wobble, hail has none, blizzard has strong
        let wobble_amp = select(0.0, select(0.8, 2.0, kind == 5u), kind == 2u || kind == 5u);
        let wobble_freq = select(0.0, select(1.5, 3.0, kind == 5u), kind == 2u || kind == 5u);
        let phase = vertex.inst_pos.x * 3.7 + vertex.inst_pos.z * 2.3;
        world_pos.x += sin(u.time * wobble_freq + phase) * wobble_amp;
        world_pos.z += cos(u.time * wobble_freq * 0.7 + phase * 1.3) * wobble_amp * 0.6;

        // Billboard: expand quad in camera-facing plane
        let cam_to_p = normalize(world_pos - u.camera_pos);
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), cam_to_p));
        let up = normalize(cross(cam_to_p, right));
        let scale = u.particle_scale;
        world_pos += right * (vertex.local_pos.x - 0.5) * scale;
        world_pos += up * (vertex.local_pos.y - 0.5) * scale;

        let dist = distance(u.camera_pos, world_pos);
        let dist_fade = 1.0 - smoothstep(u.volume_size * 0.4, u.volume_size * 0.85, dist);
        output.alpha = dist_fade * u.intensity * 0.5 * u.transition_alpha;

    } else {
        // Fallback: no particles
        output.alpha = 0.0;
    }

    output.clip_position = u.view_proj * vec4<f32>(world_pos, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let kind = u32(u.weather_kind);

    if kind == 2u || kind == 5u {
        // Snow/blizzard: soft circular particle
        let center = input.uv - vec2<f32>(0.5, 0.5);
        let r = length(center) * 2.0;
        let circle_alpha = 1.0 - smoothstep(0.6, 1.0, r);
        return vec4<f32>(u.particle_color.rgb, input.alpha * circle_alpha * u.particle_color.a);
    } else if kind == 3u {
        // Hail: harder-edged circle
        let center = input.uv - vec2<f32>(0.5, 0.5);
        let r = length(center) * 2.0;
        let circle_alpha = 1.0 - smoothstep(0.8, 1.0, r);
        return vec4<f32>(u.particle_color.rgb, input.alpha * circle_alpha * u.particle_color.a);
    } else {
        // Rain/sandstorm: line color
        return vec4<f32>(u.particle_color.rgb, input.alpha * u.particle_color.a);
    }
}
