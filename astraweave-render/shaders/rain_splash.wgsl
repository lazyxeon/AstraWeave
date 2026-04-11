// ═══════════════════════════════════════════════════════════════════════════════
// Rain Impact Splash Compute Shader
// ═══════════════════════════════════════════════════════════════════════════════
//
// Runs AFTER rain_occlusion. Scans rain particles for "dead" ones (age ≥ lifetime)
// that were recently alive (age was < lifetime last frame). At each impact site,
// spawns a small radial burst of splash particles into a separate splash buffer.

override WG_SIZE: u32 = 64u;

struct SplashParams {
    rain_particle_count: u32,
    max_splash_particles: u32,
    splash_per_impact:    u32,  // e.g. 4-8 per raindrop impact
    splash_lifetime:      f32,
    splash_speed:         f32,
    splash_scale:         f32,
    dt:                   f32,
    random_seed:          u32,
}

// Must match GpuParticle layout (64 bytes).
struct Particle {
    position: vec4<f32>,   // xyz + lifetime in w
    velocity: vec4<f32>,   // xyz + age in w
    color:    vec4<f32>,
    scale:    vec4<f32>,   // xyz + mass in w
}

@group(0) @binding(0) var<uniform> params: SplashParams;
// Rain particles (read-only, post-occlusion).
@group(0) @binding(1) var<storage, read> rain_particles: array<Particle>;
// Splash particle output buffer (append via atomic counter).
@group(0) @binding(2) var<storage, read_write> splash_particles: array<Particle>;
// Atomic counter for splash particle allocation.
@group(0) @binding(3) var<storage, read_write> splash_count: atomic<u32>;

fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash_to_float(h: u32) -> f32 {
    return f32(h) / 4294967295.0;
}

@compute @workgroup_size(WG_SIZE)
fn spawn_splashes(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.rain_particle_count) { return; }

    let rain = rain_particles[idx];
    let lifetime = rain.position.w;
    let age = rain.velocity.w;

    // Detect "just died" particles: age >= lifetime means killed by occlusion
    // or natural expiry. Only spawn splash if it was recently alive.
    if (age < lifetime || lifetime <= 0.0) { return; }

    // Only spawn if within one frame of death (avoid re-spawning every frame).
    let frames_dead = (age - lifetime) / max(params.dt, 0.001);
    if (frames_dead > 1.5) { return; }

    // Allocate splash particles atomically.
    let alloc_start = atomicAdd(&splash_count, params.splash_per_impact);
    if (alloc_start + params.splash_per_impact > params.max_splash_particles) {
        return; // Buffer full.
    }

    let impact_pos = rain.position.xyz;

    // Spawn radial splash burst around the impact point.
    for (var i = 0u; i < params.splash_per_impact; i = i + 1u) {
        let seed = pcg_hash(idx * 16u + i + params.random_seed);
        let angle = hash_to_float(pcg_hash(seed)) * 6.283185;
        let speed_var = 0.5 + hash_to_float(pcg_hash(seed + 1u)) * 0.5;
        let up_bias = 0.3 + hash_to_float(pcg_hash(seed + 2u)) * 0.7;

        let vx = cos(angle) * params.splash_speed * speed_var;
        let vz = sin(angle) * params.splash_speed * speed_var;
        let vy = params.splash_speed * up_bias; // Upward component.

        let slot = alloc_start + i;
        splash_particles[slot] = Particle(
            vec4<f32>(impact_pos, params.splash_lifetime),
            vec4<f32>(vx, vy, vz, 0.0), // age = 0 (just born)
            vec4<f32>(0.7, 0.75, 0.8, 0.6), // translucent water color
            vec4<f32>(params.splash_scale, params.splash_scale, params.splash_scale, 0.0),
        );
    }
}
