// ============================================================================
// AstraWeave Volumetric Water Shader
// ============================================================================
// Gerstner wave simulation, Fresnel reflections, specular sun glint, foam.

struct WaterUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    fog_color: vec3<f32>,
    fog_density: f32,
    water_level: f32,
    fog_enabled: u32,
    _pad: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> u: WaterUniforms;

// Gerstner wave: returns displacement vector
fn gerstner_disp(xz: vec2<f32>, dir: vec2<f32>, amp: f32, freq: f32, speed: f32, steep: f32) -> vec3<f32> {
    let d = normalize(dir);
    let phase = freq * dot(d, xz) - speed * u.time;
    let c = cos(phase);
    let s = sin(phase);
    return vec3<f32>(steep * amp * d.x * c, amp * s, steep * amp * d.y * c);
}

// Gerstner wave: returns normal contribution (nx, ny_subtract, nz)
fn gerstner_norm(xz: vec2<f32>, dir: vec2<f32>, amp: f32, freq: f32, speed: f32, steep: f32) -> vec3<f32> {
    let d = normalize(dir);
    let wa = freq * amp;
    let phase = freq * dot(d, xz) - speed * u.time;
    let c = cos(phase);
    let s = sin(phase);
    return vec3<f32>(-d.x * wa * c, steep * wa * s, -d.y * wa * c);
}

fn sum_waves_disp(xz: vec2<f32>) -> vec3<f32> {
    var d = vec3<f32>(0.0);
    d += gerstner_disp(xz, vec2<f32>(1.0, 0.3),  0.45, 0.9,  1.2, 0.5);
    d += gerstner_disp(xz, vec2<f32>(0.6, 1.0),  0.25, 1.6,  1.8, 0.4);
    d += gerstner_disp(xz, vec2<f32>(-0.4, 0.8), 0.12, 2.8,  2.2, 0.3);
    d += gerstner_disp(xz, vec2<f32>(0.8, -0.3), 0.07, 4.2,  3.0, 0.2);
    d += gerstner_disp(xz, vec2<f32>(-0.7, -0.5), 0.04, 6.5, 3.8, 0.15);
    return d;
}

fn sum_waves_norm(xz: vec2<f32>) -> vec3<f32> {
    var nx = 0.0;
    var ny_sub = 0.0;
    var nz = 0.0;
    var n: vec3<f32>;
    n = gerstner_norm(xz, vec2<f32>(1.0, 0.3),  0.45, 0.9,  1.2, 0.5);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(0.6, 1.0),  0.25, 1.6,  1.8, 0.4);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(-0.4, 0.8), 0.12, 2.8,  2.2, 0.3);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(0.8, -0.3), 0.07, 4.2,  3.0, 0.2);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(-0.7, -0.5), 0.04, 6.5, 3.8, 0.15);
    nx += n.x; ny_sub += n.y; nz += n.z;
    return normalize(vec3<f32>(nx, 1.0 - ny_sub, nz));
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var pos = vertex.position;
    pos.y = u.water_level;
    let base_xz = pos.xz;
    let disp = sum_waves_disp(base_xz);
    pos += disp;
    let wave_n = sum_waves_norm(base_xz);

    var output: VertexOutput;
    output.clip_position = u.view_proj * vec4<f32>(pos, 1.0);
    output.world_position = pos;
    output.world_normal = wave_n;
    return output;
}

// Micro-detail noise for water surface
fn water_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn water_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = water_hash(i);
    let b = water_hash(i + vec2<f32>(1.0, 0.0));
    let c = water_hash(i + vec2<f32>(0.0, 1.0));
    let d = water_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(u.camera_pos - in.world_position);
    var n = in.world_normal;
    let cam_dist = distance(u.camera_pos, in.world_position);

    // Micro-detail normal perturbation — attenuate with distance to prevent aliasing
    let detail_fade = 1.0 - smoothstep(20.0, 80.0, cam_dist);
    if detail_fade > 0.01 {
        let ds = 3.0;
        let eps = 0.15;
        let flow = vec2<f32>(u.time * 0.3, u.time * 0.2);
        let h0 = water_noise(in.world_position.xz * ds + flow);
        let hx = water_noise((in.world_position.xz + vec2<f32>(eps, 0.0)) * ds + flow);
        let hz = water_noise((in.world_position.xz + vec2<f32>(0.0, eps)) * ds + flow);
        n = normalize(n + vec3<f32>(-(hx - h0) / eps, 0.0, -(hz - h0) / eps) * 0.08 * detail_fade);
    }

    // Fresnel (clamped to avoid overbright)
    let n_dot_v = max(dot(n, view_dir), 0.0);
    let fresnel = pow(1.0 - n_dot_v, 3.0);
    let fresnel_f = clamp(mix(0.04, 0.6, fresnel), 0.04, 0.65);

    // Water color (deep blue-green)
    let deep = vec3<f32>(0.005, 0.035, 0.08);
    let shallow = vec3<f32>(0.01, 0.07, 0.10);
    let water_c = mix(shallow, deep, 0.55);

    // Sky reflection color
    let sky_reflect = vec3<f32>(0.30, 0.45, 0.65);

    // Sun specular — attenuated at distance to prevent sparkle
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let half_dir = normalize(light_dir + view_dir);
    let spec_atten = 1.0 - smoothstep(40.0, 120.0, cam_dist);
    let spec = pow(max(dot(n, half_dir), 0.0), 128.0) * 0.8 * spec_atten;
    let sun_c = vec3<f32>(1.3, 1.2, 0.95);

    var color = mix(water_c, sky_reflect, fresnel_f);
    color += sun_c * spec;

    // Foam at wave crests
    let crest = in.world_position.y - u.water_level;
    let foam_n = water_noise(in.world_position.xz * 2.0 + vec2<f32>(u.time * 0.08));
    let foam_f = smoothstep(0.2, 0.55, crest) * foam_n * detail_fade;
    color = mix(color, vec3<f32>(0.75, 0.80, 0.85), foam_f * 0.35);

    // Subsurface scattering approximation
    let sss_f = pow(max(dot(view_dir, -light_dir), 0.0), 4.0) * 0.1;
    color += vec3<f32>(0.015, 0.06, 0.05) * sss_f;

    // Tone map (Reinhard)
    color = color / (color + vec3<f32>(1.0));

    // Fog — capped at 0.65 to match terrain shader
    if u.fog_enabled == 1u {
        let dist = cam_dist;
        let fog_f = 1.0 - exp(-u.fog_density * dist);
        color = mix(color, u.fog_color, clamp(fog_f, 0.0, 0.65));
    }

    // Depth-based alpha (more opaque closer, transparent at edges)
    let alpha = mix(0.55, 0.85, clamp(cam_dist / 80.0, 0.0, 1.0));

    return vec4<f32>(color, alpha);
}
