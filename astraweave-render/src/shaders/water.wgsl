// Water shader with Gerstner wave displacement
// Implements animated ocean surface with realistic wave simulation

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) wave_height: f32,
};

struct WaterUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    water_color_deep: vec3<f32>,
    _pad1: f32,
    water_color_shallow: vec3<f32>,
    _pad2: f32,
    foam_color: vec3<f32>,
    foam_threshold: f32,
    // Rain ripple parameters.
    rain_intensity: f32,   // 0.0 = no rain, 1.0 = heavy rain
    ripple_scale: f32,     // UV tile scale for ripple pattern (default 4.0)
    ripple_strength: f32,  // Normal perturbation strength (default 0.15)
    _pad3: f32,
};

@group(0) @binding(0) var<uniform> uniforms: WaterUniforms;

// ── Rain ripple normal perturbation ─────────────────────────────────────────
// Procedural concentric ring pattern from multiple random "drop" origins.
// Each layer uses a different speed and phase offset for variation.

fn ripple_ring(uv: vec2<f32>, center: vec2<f32>, time: f32, freq: f32) -> f32 {
    let dist = length(uv - center);
    let wave = sin(dist * freq - time * 12.0) * exp(-dist * 3.0);
    // Fade out over time (each "drop" lasts ~1 second).
    let age = fract(time * 0.7 + dot(center, vec2<f32>(17.1, 31.7)));
    let fade = 1.0 - smoothstep(0.6, 1.0, age);
    return wave * fade;
}

fn rain_ripple_normal(world_xz: vec2<f32>, time: f32, scale: f32, strength: f32) -> vec3<f32> {
    let uv = world_xz * scale;
    var h = 0.0;

    // 3 layers of ripple drops at different pseudo-random positions.
    let c1 = vec2<f32>(fract(sin(dot(vec2<f32>(1.0, 2.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(1.0, 2.0), vec2<f32>(269.5, 183.3))) * 43758.5453));
    let c2 = vec2<f32>(fract(sin(dot(vec2<f32>(3.0, 4.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(3.0, 4.0), vec2<f32>(269.5, 183.3))) * 43758.5453));
    let c3 = vec2<f32>(fract(sin(dot(vec2<f32>(5.0, 6.0), vec2<f32>(127.1, 311.7))) * 43758.5453),
                       fract(sin(dot(vec2<f32>(5.0, 6.0), vec2<f32>(269.5, 183.3))) * 43758.5453));

    // Tile centers to repeat across the surface.
    let tile = floor(uv);
    h += ripple_ring(fract(uv), c1, time, 25.0);
    h += ripple_ring(fract(uv + 0.37), c2, time + 0.33, 30.0);
    h += ripple_ring(fract(uv + 0.71), c3, time + 0.67, 22.0);

    h *= strength;

    // Compute normal from finite differences of the height.
    let eps = 0.01;
    let uv_dx = (uv + vec2<f32>(eps, 0.0));
    let uv_dz = (uv + vec2<f32>(0.0, eps));
    var h_dx = 0.0;
    var h_dz = 0.0;
    h_dx += ripple_ring(fract(uv_dx), c1, time, 25.0);
    h_dx += ripple_ring(fract(uv_dx + 0.37), c2, time + 0.33, 30.0);
    h_dx += ripple_ring(fract(uv_dx + 0.71), c3, time + 0.67, 22.0);
    h_dz += ripple_ring(fract(uv_dz), c1, time, 25.0);
    h_dz += ripple_ring(fract(uv_dz + 0.37), c2, time + 0.33, 30.0);
    h_dz += ripple_ring(fract(uv_dz + 0.71), c3, time + 0.67, 22.0);
    h_dx *= strength;
    h_dz *= strength;

    let dx = (h_dx - h) / eps;
    let dz = (h_dz - h) / eps;
    return normalize(vec3<f32>(-dx, 1.0, -dz));
}

// Gerstner wave parameters
// Each wave: (direction.x, direction.y, amplitude, frequency)
const WAVE_COUNT: u32 = 4u;

fn gerstner_wave(
    pos: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    direction: vec2<f32>,
    steepness: f32,
) -> vec3<f32> {
    let d = normalize(direction);
    let phase = frequency * (dot(d, pos) - speed * time);
    let Q = steepness / (frequency * amplitude * f32(WAVE_COUNT));
    
    return vec3<f32>(
        Q * amplitude * d.x * cos(phase),
        amplitude * sin(phase),
        Q * amplitude * d.y * cos(phase)
    );
}

fn gerstner_normal(
    pos: vec2<f32>,
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    direction: vec2<f32>,
    steepness: f32,
) -> vec3<f32> {
    let d = normalize(direction);
    let phase = frequency * (dot(d, pos) - speed * time);
    let Q = steepness / (frequency * amplitude * f32(WAVE_COUNT));
    let WA = frequency * amplitude;
    
    let s = sin(phase);
    let c = cos(phase);
    
    return vec3<f32>(
        -d.x * WA * c,
        1.0 - Q * WA * s,
        -d.y * WA * c
    );
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    let time = uniforms.time;
    var pos = input.position;
    
    // Apply 4 Gerstner waves with different parameters
    var displacement = vec3<f32>(0.0);
    var normal_accum = vec3<f32>(0.0, 1.0, 0.0);
    
    // Wave 1: Primary swell (large, slow)
    displacement += gerstner_wave(pos.xz, time, 0.8, 0.15, 2.0, vec2<f32>(1.0, 0.3), 0.5);
    normal_accum += gerstner_normal(pos.xz, time, 0.8, 0.15, 2.0, vec2<f32>(1.0, 0.3), 0.5);
    
    // Wave 2: Secondary swell (medium)
    displacement += gerstner_wave(pos.xz, time, 0.5, 0.25, 2.5, vec2<f32>(-0.5, 1.0), 0.4);
    normal_accum += gerstner_normal(pos.xz, time, 0.5, 0.25, 2.5, vec2<f32>(-0.5, 1.0), 0.4);
    
    // Wave 3: Chop (small, fast)
    displacement += gerstner_wave(pos.xz, time, 0.25, 0.5, 3.5, vec2<f32>(0.7, -0.7), 0.3);
    normal_accum += gerstner_normal(pos.xz, time, 0.25, 0.5, 3.5, vec2<f32>(0.7, -0.7), 0.3);
    
    // Wave 4: Ripples (tiny, very fast)
    displacement += gerstner_wave(pos.xz, time, 0.1, 1.0, 4.0, vec2<f32>(-0.3, 0.9), 0.2);
    normal_accum += gerstner_normal(pos.xz, time, 0.1, 1.0, 4.0, vec2<f32>(-0.3, 0.9), 0.2);
    
    pos.x += displacement.x;
    pos.y += displacement.y;
    pos.z += displacement.z;
    
    output.world_pos = pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    output.uv = input.uv;
    output.normal = normalize(normal_accum);
    output.wave_height = displacement.y;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var N = normalize(input.normal);
    let V = normalize(uniforms.camera_pos - input.world_pos);

    // Rain ripple normal perturbation.
    if (uniforms.rain_intensity > 0.0) {
        let ripple_N = rain_ripple_normal(
            input.world_pos.xz,
            uniforms.time,
            uniforms.ripple_scale,
            uniforms.ripple_strength * uniforms.rain_intensity,
        );
        // Blend ripple normal with wave normal.
        N = normalize(mix(N, ripple_N, uniforms.rain_intensity * 0.6));
    }
    
    // Fresnel effect for reflection blend
    let fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.0);
    
    // Depth-based color blend (shallow vs deep)
    let depth_factor = clamp(input.wave_height * 2.0 + 0.5, 0.0, 1.0);
    let water_color = mix(uniforms.water_color_deep, uniforms.water_color_shallow, depth_factor);
    
    // Fake sky reflection (blue-ish)
    let sky_color = vec3<f32>(0.6, 0.75, 0.95);
    let reflected = mix(water_color, sky_color, fresnel * 0.6);
    
    // Sun specular highlight
    let sun_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let H = normalize(V + sun_dir);
    let spec = pow(max(dot(N, H), 0.0), 128.0);
    let sun_color = vec3<f32>(1.0, 0.95, 0.8);
    
    // Foam on wave peaks
    let foam_intensity = smoothstep(uniforms.foam_threshold, uniforms.foam_threshold + 0.2, input.wave_height);
    let with_foam = mix(reflected, uniforms.foam_color, foam_intensity * 0.7);
    
    // Final color with specular
    let final_color = with_foam + sun_color * spec * 0.8;
    
    // Slight transparency for water
    let alpha = mix(0.85, 0.95, fresnel);
    
    // Final color with specular and alpha
    return vec4<f32>(final_color, alpha);
}
