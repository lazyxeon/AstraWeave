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
    lightning_flash: f32,   // 0..1 storm lightning flash intensity
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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

// Positive-modulo helper: WGSL `%` follows dividend sign, so we need (x%m+m)%m
fn pos_mod(x: f32, m: f32) -> f32 {
    return ((x % m) + m) % m;
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    let kind = u32(u.weather_kind);
    let vol  = u.volume_size;          // half-extent of volume
    let vol2 = vol * 2.0;             // full extent for wrapping

    // Animated falling Y: instance base Y minus accumulated fall distance
    let fall_y = vertex.inst_pos.y - u.time * vertex.inst_speed;

    // ── World-space volumetric wrapping ──
    // Particles tile infinitely around the camera.  As the camera moves,
    // particles seamlessly wrap from the trailing to the leading edge of
    // the volume, so the player genuinely *travels through* precipitation.
    //
    // For each axis:  cam_rel = posmod(offset - camera + half, full) - half
    //   where "offset" is the instance position (or animated position for Y).
    let rx = pos_mod(vertex.inst_pos.x - u.camera_pos.x + vol, vol2) - vol;
    let ry = pos_mod(fall_y           - u.camera_pos.y + vol, vol2) - vol;
    let rz = pos_mod(vertex.inst_pos.z - u.camera_pos.z + vol, vol2) - vol;

    // cam_rel is now the camera-relative offset; avoids f32 jitter.
    var cam_rel = vec3<f32>(rx, ry, rz);

    // Wind drift (stronger at top of volume)
    let height_frac = clamp((cam_rel.y + vol) / vol2, 0.0, 1.0);
    let wind_strength = select(3.0, 1.5, kind == 2u || kind == 5u);
    cam_rel.x += u.wind_x * (1.0 - height_frac) * wind_strength;
    cam_rel.z += u.wind_z * (1.0 - height_frac) * wind_strength;

    var output: VertexOutput;
    output.uv = vertex.local_pos;

    // Weather-specific motion and vertex displacement
    if kind == 1u || kind == 4u {
        // === RAIN / SANDSTORM: Line particles ===
        let streak_dir = normalize(vec3<f32>(u.wind_x * 0.15, -1.0, u.wind_z * 0.15));
        let slen = u.streak_length + vertex.inst_speed * 0.04;
        cam_rel += streak_dir * vertex.local_pos.y * slen;

        let dist = length(cam_rel);
        let dist_fade = 1.0 - smoothstep(vol * 0.3, vol * 0.9, dist);
        // Fade along the streak: bright at head (y=0), dimmer at tail (y=1)
        let streak_fade = 1.0 - vertex.local_pos.y * 0.5;
        output.alpha = streak_fade * dist_fade * u.intensity * u.transition_alpha;

    } else if kind == 2u || kind == 3u || kind == 5u {
        // === SNOW / HAIL / BLIZZARD: Billboard quad particles ===
        let wobble_amp = select(0.0, select(0.8, 2.0, kind == 5u), kind == 2u || kind == 5u);
        let wobble_freq = select(0.0, select(1.5, 3.0, kind == 5u), kind == 2u || kind == 5u);
        let phase = vertex.inst_pos.x * 3.7 + vertex.inst_pos.z * 2.3;
        cam_rel.x += sin(u.time * wobble_freq + phase) * wobble_amp;
        cam_rel.z += cos(u.time * wobble_freq * 0.7 + phase * 1.3) * wobble_amp * 0.6;

        // Billboard: expand quad perpendicular to camera direction
        let cam_to_p = normalize(cam_rel);
        let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), cam_to_p));
        let up    = normalize(cross(cam_to_p, right));
        let scale = u.particle_scale;
        cam_rel += right * (vertex.local_pos.x - 0.5) * scale;
        cam_rel += up    * (vertex.local_pos.y - 0.5) * scale;

        let dist = length(cam_rel);
        let dist_fade = 1.0 - smoothstep(vol * 0.4, vol * 0.85, dist);
        output.alpha = dist_fade * u.intensity * 0.5 * u.transition_alpha;

    } else {
        output.alpha = 0.0;
    }

    output.clip_position = u.view_proj * vec4<f32>(cam_rel, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let kind = u32(u.weather_kind);

    // Lightning flash: brighten particles toward white and boost alpha
    let flash = u.lightning_flash;
    let flash_color = vec3<f32>(1.0, 1.0, 1.0);
    let flash_alpha_boost = 1.0 + flash * 4.0;

    if kind == 2u || kind == 5u {
        // Snow/blizzard: soft circular particle
        let center = input.uv - vec2<f32>(0.5, 0.5);
        let r = length(center) * 2.0;
        let circle_alpha = 1.0 - smoothstep(0.6, 1.0, r);
        let base_color = mix(u.particle_color.rgb, flash_color, flash);
        return vec4<f32>(base_color, input.alpha * circle_alpha * u.particle_color.a * flash_alpha_boost);
    } else if kind == 3u {
        // Hail: harder-edged circle
        let center = input.uv - vec2<f32>(0.5, 0.5);
        let r = length(center) * 2.0;
        let circle_alpha = 1.0 - smoothstep(0.8, 1.0, r);
        let base_color = mix(u.particle_color.rgb, flash_color, flash);
        return vec4<f32>(base_color, input.alpha * circle_alpha * u.particle_color.a * flash_alpha_boost);
    } else {
        // Rain/sandstorm: line color
        let base_color = mix(u.particle_color.rgb, flash_color, flash);
        return vec4<f32>(base_color, input.alpha * u.particle_color.a * flash_alpha_boost);
    }
}

// ============================================================================
// Lightning Flash Overlay (full-screen white flash for storms)
// ============================================================================

@vertex
fn vs_flash(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // Full-screen triangle covering entire viewport (3 verts, no vertex buffers)
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_flash() -> @location(0) vec4<f32> {
    // Bright white flash, intensity from uniform (peaks at ~0.35 alpha for dramatic effect)
    return vec4<f32>(1.0, 1.0, 1.0, u.lightning_flash * 0.35);
}
