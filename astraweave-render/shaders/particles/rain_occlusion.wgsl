// Rain Occlusion Compute Shader
//
// Post-process pass run after particle simulation. For each active particle,
// projects its world position to screen space, samples the previous frame's
// depth buffer, and kills the particle if it's behind solid geometry.
//
// This prevents rain/snow from rendering inside buildings, under tree canopies,
// or below terrain overhangs.

struct Particle {
    pos_life:  vec4<f32>,  // xyz=position, w=max_lifetime
    vel_age:   vec4<f32>,  // xyz=velocity, w=current_age
    color:     vec4<f32>,
    size_mass: vec4<f32>,
}

struct OcclusionParams {
    view_proj:      mat4x4<f32>,
    screen_size:    vec2<f32>,  // width, height in pixels
    particle_count: u32,
    depth_bias:     f32,       // small bias to prevent self-occlusion
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform>             params:    OcclusionParams;
@group(0) @binding(2) var                      depth_tex: texture_depth_2d;
@group(0) @binding(3) var                      depth_samp: sampler;

@compute @workgroup_size(64)
fn rain_occlusion(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.particle_count) {
        return;
    }

    var p = particles[idx];

    // Skip dead particles
    let age = p.vel_age.w;
    let lifetime = p.pos_life.w;
    if (age >= lifetime || lifetime <= 0.0 || p.color.a <= 0.0) {
        return;
    }

    let world_pos = p.pos_life.xyz;

    // Project to clip space
    let clip = params.view_proj * vec4<f32>(world_pos, 1.0);

    // Behind camera — skip (don't occlude)
    if (clip.w <= 0.0) {
        return;
    }

    let ndc = clip.xyz / clip.w;

    // Outside screen bounds — skip
    if (ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0) {
        return;
    }

    // NDC to UV (Y is flipped: NDC +Y is top, UV +V is bottom)
    let uv = vec2<f32>(
        ndc.x * 0.5 + 0.5,
        -ndc.y * 0.5 + 0.5,
    );

    // Sample scene depth from previous frame
    let scene_depth = textureSampleLevel(depth_tex, depth_samp, uv, 0);

    // Particle depth in [0, 1] range (reverse-Z or standard depending on projection)
    let particle_depth = ndc.z;

    // In standard (non-reverse-Z) depth: closer = smaller depth value.
    // Particle is occluded if its depth > scene depth at that pixel.
    // Add a small bias to prevent self-occlusion from nearby particles.
    if (particle_depth > scene_depth + params.depth_bias) {
        // Kill particle: set alpha to 0 and age to lifetime
        p.color.a = 0.0;
        p.vel_age.w = p.pos_life.w; // mark as dead
        particles[idx] = p;
    }
}
