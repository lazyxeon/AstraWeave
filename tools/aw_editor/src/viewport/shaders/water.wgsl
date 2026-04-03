// ============================================================================
// AstraWeave Volumetric Water Shader
// ============================================================================
// Gerstner waves, depth-based coloring, shoreline foam, analytic sky Fresnel,
// subsurface scattering, biome-aware properties.

struct WaterUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    fog_color: vec3<f32>,
    fog_density: f32,
    water_level: f32,
    fog_enabled: u32,
    near_plane: f32,
    far_plane: f32,
    sun_dir: vec3<f32>,
    sun_intensity: f32,
    screen_size: vec2<f32>,
    wave_amplitude: f32,
    turbidity: f32,
    water_color_shallow: vec3<f32>,
    shore_foam_intensity: f32,
    water_color_deep: vec3<f32>,
    _pad: f32,
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
@group(0) @binding(1)
var depth_texture: texture_depth_2d;
@group(0) @binding(2)
var depth_sampler: sampler;

// ─── Gerstner Wave System ───────────────────────────────────────────────────

fn gerstner_disp(xz: vec2<f32>, dir: vec2<f32>, amp: f32, freq: f32, speed: f32, steep: f32) -> vec3<f32> {
    let d = normalize(dir);
    let phase = freq * dot(d, xz) - speed * u.time;
    let c = cos(phase);
    let s = sin(phase);
    return vec3<f32>(steep * amp * d.x * c, amp * s, steep * amp * d.y * c);
}

fn gerstner_norm(xz: vec2<f32>, dir: vec2<f32>, amp: f32, freq: f32, speed: f32, steep: f32) -> vec3<f32> {
    let d = normalize(dir);
    let wa = freq * amp;
    let phase = freq * dot(d, xz) - speed * u.time;
    let c = cos(phase);
    let s = sin(phase);
    return vec3<f32>(-d.x * wa * c, steep * wa * s, -d.y * wa * c);
}

fn sum_waves_disp(xz: vec2<f32>) -> vec3<f32> {
    let a = u.wave_amplitude;
    var d = vec3<f32>(0.0);
    d += gerstner_disp(xz, vec2<f32>(1.0, 0.3),  0.45 * a, 0.9,  1.2, 0.5);
    d += gerstner_disp(xz, vec2<f32>(0.6, 1.0),  0.25 * a, 1.6,  1.8, 0.4);
    d += gerstner_disp(xz, vec2<f32>(-0.4, 0.8), 0.12 * a, 2.8,  2.2, 0.3);
    d += gerstner_disp(xz, vec2<f32>(0.8, -0.3), 0.07 * a, 4.2,  3.0, 0.2);
    d += gerstner_disp(xz, vec2<f32>(-0.7, -0.5), 0.04 * a, 6.5, 3.8, 0.15);
    return d;
}

fn sum_waves_norm(xz: vec2<f32>) -> vec3<f32> {
    let a = u.wave_amplitude;
    var nx = 0.0;
    var ny_sub = 0.0;
    var nz = 0.0;
    var n: vec3<f32>;
    n = gerstner_norm(xz, vec2<f32>(1.0, 0.3),  0.45 * a, 0.9,  1.2, 0.5);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(0.6, 1.0),  0.25 * a, 1.6,  1.8, 0.4);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(-0.4, 0.8), 0.12 * a, 2.8,  2.2, 0.3);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(0.8, -0.3), 0.07 * a, 4.2,  3.0, 0.2);
    nx += n.x; ny_sub += n.y; nz += n.z;
    n = gerstner_norm(xz, vec2<f32>(-0.7, -0.5), 0.04 * a, 6.5, 3.8, 0.15);
    nx += n.x; ny_sub += n.y; nz += n.z;
    return normalize(vec3<f32>(nx, 1.0 - ny_sub, nz));
}

// ─── Vertex Shader ──────────────────────────────────────────────────────────

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var pos = vertex.position;
    pos.y = u.water_level;
    let base_xz = pos.xz;
    let disp = sum_waves_disp(base_xz);
    pos += disp;
    let wave_n = sum_waves_norm(base_xz);

    var output: VertexOutput;
    let rel_pos = pos - u.camera_pos;
    output.clip_position = u.view_proj * vec4<f32>(rel_pos, 1.0);
    output.world_position = pos;
    output.world_normal = wave_n;
    return output;
}

// ─── Utility Functions ──────────────────────────────────────────────────────

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

// Multi-octave FBM noise for richer foam/detail
fn water_fbm(p: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp_v = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        val += water_noise(pos) * amp_v;
        pos *= 2.1;
        amp_v *= 0.5;
    }
    return val;
}

/// Linearize depth from clip-space [0,1] to view-space distance.
fn linearize_depth(d: f32) -> f32 {
    return u.near_plane * u.far_plane / (u.far_plane - d * (u.far_plane - u.near_plane));
}

/// Analytic sky gradient — approximates environment for Fresnel reflections
/// without needing a cubemap. Uses fog_color at horizon, blue at zenith.
fn analytic_sky(reflect_dir: vec3<f32>) -> vec3<f32> {
    let up = max(reflect_dir.y, 0.0);
    let zenith = vec3<f32>(0.22, 0.38, 0.72);
    let horizon = u.fog_color;
    // Gradient from horizon color to zenith blue
    let sky = mix(horizon, zenith, pow(up, 0.45));
    // Add subtle sun glow near sun direction
    let sun_dot = max(dot(reflect_dir, normalize(u.sun_dir)), 0.0);
    let sun_glow = pow(sun_dot, 64.0) * 0.4 * u.sun_intensity;
    let sun_color = vec3<f32>(1.2, 1.1, 0.9);
    return sky + sun_color * sun_glow;
}

// ─── Fragment Shader ────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(u.camera_pos - in.world_position);
    var n = in.world_normal;
    let cam_dist = distance(u.camera_pos, in.world_position);

    // ── Micro-detail normal perturbation ────────────────────────────────
    let detail_fade = 1.0 - smoothstep(30.0, 120.0, cam_dist);
    if detail_fade > 0.01 {
        let ds = 3.0;
        let eps = 0.15;
        let flow = vec2<f32>(u.time * 0.3, u.time * 0.2);
        let h0 = water_noise(in.world_position.xz * ds + flow);
        let hx = water_noise((in.world_position.xz + vec2<f32>(eps, 0.0)) * ds + flow);
        let hz = water_noise((in.world_position.xz + vec2<f32>(0.0, eps)) * ds + flow);
        // Second layer at different scale/flow for richer detail
        let flow2 = vec2<f32>(-u.time * 0.15, u.time * 0.25);
        let h0b = water_noise(in.world_position.xz * 1.7 + flow2);
        let hxb = water_noise((in.world_position.xz + vec2<f32>(eps, 0.0)) * 1.7 + flow2);
        let hzb = water_noise((in.world_position.xz + vec2<f32>(0.0, eps)) * 1.7 + flow2);
        let dx = (-(hx - h0) / eps) + (-(hxb - h0b) / eps) * 0.5;
        let dz = (-(hz - h0) / eps) + (-(hzb - h0b) / eps) * 0.5;
        n = normalize(n + vec3<f32>(dx, 0.0, dz) * 0.12 * detail_fade);
    }

    // ── Depth-based water thickness ─────────────────────────────────────
    // Use textureLoad for exact texel fetch — avoids sampler compatibility issues
    let depth_coords = vec2<i32>(in.clip_position.xy);
    let raw_depth = textureLoad(depth_texture, depth_coords, 0);
    let terrain_depth = linearize_depth(raw_depth);
    // @builtin(position).z is already in [0,1] NDC (perspective divide done by rasterizer)
    let water_depth = linearize_depth(in.clip_position.z);
    let thickness = max(terrain_depth - water_depth, 0.0);

    // Depth-based blending factor (exponential absorption)
    let absorption = 1.0 - exp(-thickness * u.turbidity * 0.3);

    // ── Water color ─────────────────────────────────────────────────────
    let water_c = mix(u.water_color_shallow, u.water_color_deep, absorption);

    // ── Fresnel with analytic sky reflection ─────────────────────────────
    let n_dot_v = max(dot(n, view_dir), 0.0);
    let fresnel = pow(1.0 - n_dot_v, 4.0);
    let fresnel_f = clamp(mix(0.02, 0.8, fresnel), 0.02, 0.85);

    // Compute reflection direction and sample analytic sky
    let reflect_dir = reflect(-view_dir, n);
    let sky_reflect = analytic_sky(reflect_dir);

    var color = mix(water_c, sky_reflect, fresnel_f);

    // ── Sun specular (Blinn-Phong) ──────────────────────────────────────
    let light_dir = normalize(u.sun_dir);
    let half_dir = normalize(light_dir + view_dir);
    let spec_atten = 1.0 - smoothstep(60.0, 200.0, cam_dist);
    let spec = pow(max(dot(n, half_dir), 0.0), 256.0) * 1.2 * spec_atten * u.sun_intensity;
    let sun_c = vec3<f32>(1.3, 1.2, 0.95);
    color += sun_c * spec;

    // ── Shoreline foam ──────────────────────────────────────────────────
    let shore_threshold = 1.5;
    let shore_factor = 1.0 - smoothstep(0.0, shore_threshold, thickness);
    if shore_factor > 0.01 && u.shore_foam_intensity > 0.01 {
        // Animated foam pattern at shore
        let foam_uv1 = in.world_position.xz * 1.5 + vec2<f32>(u.time * 0.12, u.time * 0.08);
        let foam_uv2 = in.world_position.xz * 3.2 + vec2<f32>(-u.time * 0.06, u.time * 0.14);
        let foam_n1 = water_fbm(foam_uv1);
        let foam_n2 = water_noise(foam_uv2);
        let foam_pattern = foam_n1 * 0.7 + foam_n2 * 0.3;

        // Sharpen foam at very shallow water
        let foam_mask = smoothstep(0.3, 0.6, foam_pattern) * shore_factor;
        let foam_color = vec3<f32>(0.85, 0.88, 0.92);
        color = mix(color, foam_color, foam_mask * u.shore_foam_intensity * 0.7);
    }

    // ── Wave crest foam ─────────────────────────────────────────────────
    let crest = in.world_position.y - u.water_level;
    let crest_noise = water_noise(in.world_position.xz * 2.0 + vec2<f32>(u.time * 0.08));
    let crest_foam = smoothstep(0.2 * u.wave_amplitude, 0.55 * u.wave_amplitude, crest)
                     * crest_noise * detail_fade;
    color = mix(color, vec3<f32>(0.80, 0.84, 0.88), crest_foam * 0.35);

    // ── Subsurface scattering ───────────────────────────────────────────
    let sss_f = pow(max(dot(view_dir, -light_dir), 0.0), 4.0) * 0.12;
    let sss_color = vec3<f32>(0.02, 0.08, 0.06);
    color += sss_color * sss_f * (1.0 - absorption * 0.5);

    // ── Tone map (ACES approximation) ───────────────────────────────────
    // Use ACES for more pleasing highlight rolloff than Reinhard
    let a_val = 2.51;
    let b_val = 0.03;
    let c_val = 2.43;
    let d_val = 0.59;
    let e_val = 0.14;
    color = clamp((color * (a_val * color + b_val)) / (color * (c_val * color + d_val) + e_val), vec3<f32>(0.0), vec3<f32>(1.0));

    // ── Fog ─────────────────────────────────────────────────────────────
    if u.fog_enabled == 1u {
        let fog_f = 1.0 - exp(-u.fog_density * cam_dist);
        color = mix(color, u.fog_color, clamp(fog_f, 0.0, 0.65));
    }

    // ── Alpha: depth-based transparency ─────────────────────────────────
    // Shallow water is transparent; deep water is nearly opaque
    let depth_alpha = clamp(absorption * 1.5 + 0.15, 0.1, 0.92);
    // Minimum alpha at distance to prevent water from disappearing
    let dist_alpha = mix(depth_alpha, 0.85, smoothstep(100.0, 500.0, cam_dist));
    let alpha = clamp(dist_alpha, 0.1, 0.92);

    return vec4<f32>(color, alpha);
}
