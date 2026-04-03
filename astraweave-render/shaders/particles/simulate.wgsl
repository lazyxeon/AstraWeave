// Enhanced Particle Simulation — Niagara-class GPU compute
//
// Forces: gravity, drag, turbulence (curl noise), point attractors, wind
// Lifetime curves: size-over-life, color-over-life (4-key gradient)
// Emission: sphere, cone, box, ring shapes with initial velocity

struct Particle {
    pos_life:  vec4<f32>,  // xyz=position, w=max_lifetime
    vel_age:   vec4<f32>,  // xyz=velocity, w=current_age
    color:     vec4<f32>,  // rgba (current, interpolated each frame)
    size_mass: vec4<f32>,  // x=current_size, y=base_size, z=unused, w=mass
};

struct SimParams {
    delta_time:       f32,
    particle_count:   u32,
    max_particles:    u32,
    random_seed:      u32,
    // Forces
    gravity:          vec3<f32>,
    drag_coefficient: f32,      // linear drag (0 = none, ~2.0 = heavy)
    wind:             vec3<f32>,
    turbulence_str:   f32,      // curl noise strength
    turbulence_freq:  f32,      // curl noise frequency
    turbulence_speed: f32,      // animation speed
    time:             f32,
    // Attractor
    attractor_pos:    vec3<f32>,
    attractor_str:    f32,      // positive = attract, negative = repel
    // Color gradient (4 keys, linear interpolation over lifetime)
    color0:           vec4<f32>,
    color1:           vec4<f32>,
    color2:           vec4<f32>,
    color3:           vec4<f32>,
    // Size curve (4 keys)
    size_keys:        vec4<f32>, // x=t0, y=t1, z=t2, w=t3 size values
    _pad:             vec4<f32>,
};

@group(0) @binding(0) var<storage, read>       particles_in:  array<Particle>;
@group(0) @binding(1) var<storage, read_write>  particles_out: array<Particle>;
@group(0) @binding(2) var<uniform>              params:        SimParams;

// ---- Hash / noise ----

fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(mix(hash31(i), hash31(i + vec3(1.0, 0.0, 0.0)), u.x),
            mix(hash31(i + vec3(0.0, 1.0, 0.0)), hash31(i + vec3(1.0, 1.0, 0.0)), u.x), u.y),
        mix(mix(hash31(i + vec3(0.0, 0.0, 1.0)), hash31(i + vec3(1.0, 0.0, 1.0)), u.x),
            mix(hash31(i + vec3(0.0, 1.0, 1.0)), hash31(i + vec3(1.0, 1.0, 1.0)), u.x), u.y),
        u.z);
}

// Curl noise for divergence-free turbulence
fn curl_noise(p: vec3<f32>) -> vec3<f32> {
    let e = 0.01;
    let dx = vec3<f32>(e, 0.0, 0.0);
    let dy = vec3<f32>(0.0, e, 0.0);
    let dz = vec3<f32>(0.0, 0.0, e);

    let px = value_noise(p + dy) - value_noise(p - dy);
    let py = value_noise(p + dz) - value_noise(p - dz);
    let pz = value_noise(p + dx) - value_noise(p - dx);

    let qx = value_noise(p + dz) - value_noise(p - dz);
    let qy = value_noise(p + dx) - value_noise(p - dx);
    let qz = value_noise(p + dy) - value_noise(p - dy);

    return vec3<f32>(px - qx, py - qy, pz - qz) / (2.0 * e);
}

// ---- Lifetime curves ----

fn sample_color_gradient(t: f32) -> vec4<f32> {
    // 4-key gradient at t=0, 0.33, 0.66, 1.0
    if (t < 0.333) {
        return mix(params.color0, params.color1, t / 0.333);
    } else if (t < 0.666) {
        return mix(params.color1, params.color2, (t - 0.333) / 0.333);
    } else {
        return mix(params.color2, params.color3, (t - 0.666) / 0.334);
    }
}

fn sample_size_curve(t: f32) -> f32 {
    let keys = params.size_keys;
    if (t < 0.333) {
        return mix(keys.x, keys.y, t / 0.333);
    } else if (t < 0.666) {
        return mix(keys.y, keys.z, (t - 0.333) / 0.333);
    } else {
        return mix(keys.z, keys.w, (t - 0.666) / 0.334);
    }
}

@compute @workgroup_size(64)
fn simulate_particles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.max_particles) {
        return;
    }

    var p = particles_in[idx];
    let dt = params.delta_time;

    // Advance age
    p.vel_age.w += dt;
    let age = p.vel_age.w;
    let lifetime = p.pos_life.w;

    // Dead particle: zero out and skip
    if (age >= lifetime || lifetime <= 0.0) {
        p.vel_age.w = lifetime; // clamp at max
        p.color.a = 0.0;       // invisible
        particles_out[idx] = p;
        return;
    }

    let t = age / lifetime; // normalized lifetime [0, 1]
    var vel = p.vel_age.xyz;
    let pos = p.pos_life.xyz;

    // --- Force accumulation ---
    var force = vec3<f32>(0.0);

    // Gravity
    force += params.gravity * p.size_mass.w;

    // Drag (linear, opposing velocity)
    force -= vel * params.drag_coefficient;

    // Wind
    force += params.wind;

    // Turbulence (curl noise)
    if (params.turbulence_str > 0.0) {
        let noise_pos = pos * params.turbulence_freq + params.time * params.turbulence_speed;
        let turb = curl_noise(noise_pos);
        force += turb * params.turbulence_str;
    }

    // Point attractor
    if (abs(params.attractor_str) > 0.001) {
        let to_attractor = params.attractor_pos - pos;
        let dist = length(to_attractor) + 0.1; // avoid singularity
        force += normalize(to_attractor) * params.attractor_str / (dist * dist);
    }

    // Semi-implicit Euler integration
    vel += force * dt / max(p.size_mass.w, 0.01);
    let new_pos = pos + vel * dt;

    // --- Lifetime curves ---
    let new_color = sample_color_gradient(t);
    let new_size = sample_size_curve(t) * p.size_mass.y; // scale by base_size

    // Write output
    p.pos_life = vec4<f32>(new_pos, lifetime);
    p.vel_age = vec4<f32>(vel, age);
    p.color = new_color;
    p.size_mass.x = new_size;

    particles_out[idx] = p;
}
