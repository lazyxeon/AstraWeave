// Surface Cache Update — Lumen GI radiance probe grid
//
// Each probe stores second-order spherical harmonics (L2 = 9 coefficients per RGB channel).
// A subset of probes is updated each frame by sampling the scene's direct + indirect lighting
// from 6 axis-aligned directions, then projecting into SH basis.

struct SurfaceCacheParams {
    grid_origin:       vec3<f32>,
    probe_spacing:     f32,
    grid_dims:         vec3<u32>,
    num_probes:        u32,
    update_offset:     u32,     // first probe index to update this frame
    update_count:      u32,     // how many probes to update
    frame_index:       u32,
    hysteresis:        f32,     // temporal blend (0.95 = slow update)
    sky_intensity:     f32,
    _pad0:             f32,
    _pad1:             f32,
    _pad2:             f32,
};

// SH L2 = 9 coefficients per channel, stored as 9 × vec4(r,g,b,weight)
struct ProbeSH {
    c0: vec4<f32>,  // L0 band (DC)
    c1: vec4<f32>,  // L1 Y_{1,-1}
    c2: vec4<f32>,  // L1 Y_{1, 0}
    c3: vec4<f32>,  // L1 Y_{1,+1}
    c4: vec4<f32>,  // L2 Y_{2,-2}
    c5: vec4<f32>,  // L2 Y_{2,-1}
    c6: vec4<f32>,  // L2 Y_{2, 0}
    c7: vec4<f32>,  // L2 Y_{2,+1}
    c8: vec4<f32>,  // L2 Y_{2,+2}
};

struct DirectionalLight {
    direction: vec3<f32>,
    intensity: f32,
    color:     vec3<f32>,
    _pad:      f32,
};

@group(0) @binding(0) var<uniform>              params:      SurfaceCacheParams;
@group(0) @binding(1) var<storage, read_write>  probes:      array<ProbeSH>;
@group(0) @binding(2) var<storage, read>        dir_lights:  array<DirectionalLight>;
@group(0) @binding(3) var t_depth:              texture_2d<f32>;
@group(0) @binding(4) var t_albedo:             texture_2d<f32>;
@group(0) @binding(5) var s_linear:             sampler;

// ----- SH basis functions (real, unnormalized) -----

fn sh_basis(dir: vec3<f32>) -> array<f32, 9> {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    var b: array<f32, 9>;
    // L0
    b[0] = 0.282095;
    // L1
    b[1] = 0.488603 * y;
    b[2] = 0.488603 * z;
    b[3] = 0.488603 * x;
    // L2
    b[4] = 1.092548 * x * y;
    b[5] = 1.092548 * y * z;
    b[6] = 0.315392 * (3.0 * z * z - 1.0);
    b[7] = 1.092548 * x * z;
    b[8] = 0.546274 * (x * x - y * y);
    return b;
}

fn probe_world_pos(idx: u32) -> vec3<f32> {
    let gz = idx / (params.grid_dims.x * params.grid_dims.y);
    let rem = idx % (params.grid_dims.x * params.grid_dims.y);
    let gy = rem / params.grid_dims.x;
    let gx = rem % params.grid_dims.x;
    return params.grid_origin + vec3<f32>(f32(gx), f32(gy), f32(gz)) * params.probe_spacing;
}

// 6-direction cubemap-style sampling directions
const SAMPLE_DIRS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>( 1.0,  0.0,  0.0),
    vec3<f32>(-1.0,  0.0,  0.0),
    vec3<f32>( 0.0,  1.0,  0.0),
    vec3<f32>( 0.0, -1.0,  0.0),
    vec3<f32>( 0.0,  0.0,  1.0),
    vec3<f32>( 0.0,  0.0, -1.0),
);

// Additional sample directions for better SH coverage (12 edges of icosahedron approx)
const EXTRA_DIRS: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
    vec3<f32>( 0.577,  0.577,  0.577),
    vec3<f32>(-0.577,  0.577,  0.577),
    vec3<f32>( 0.577, -0.577,  0.577),
    vec3<f32>( 0.577,  0.577, -0.577),
    vec3<f32>(-0.577, -0.577,  0.577),
    vec3<f32>(-0.577,  0.577, -0.577),
    vec3<f32>( 0.577, -0.577, -0.577),
    vec3<f32>(-0.577, -0.577, -0.577),
);

fn evaluate_radiance(probe_pos: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    // Accumulate direct lighting from all directional lights
    var radiance = vec3<f32>(0.0);

    // Sky contribution: use simple hemisphere model
    let sky_factor = max(dir.y * 0.5 + 0.5, 0.0);
    radiance += vec3<f32>(0.4, 0.5, 0.7) * sky_factor * params.sky_intensity;

    // Ground bounce
    let ground_factor = max(-dir.y * 0.5 + 0.5, 0.0) * 0.15;
    radiance += vec3<f32>(0.3, 0.25, 0.2) * ground_factor * params.sky_intensity;

    // Direct lights contribution (N·L for diffuse)
    for (var i = 0u; i < arrayLength(&dir_lights); i++) {
        let light = dir_lights[i];
        let n_dot_l = max(dot(dir, -light.direction), 0.0);
        radiance += light.color * light.intensity * n_dot_l * 0.318310; // 1/pi
    }

    return radiance;
}

@compute @workgroup_size(64)
fn surface_cache_update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let local_idx = gid.x;
    if (local_idx >= params.update_count) {
        return;
    }

    let probe_idx = params.update_offset + local_idx;
    if (probe_idx >= params.num_probes) {
        return;
    }

    let pos = probe_world_pos(probe_idx);

    // Build new SH from sampled radiance
    var new_sh: ProbeSH;
    new_sh.c0 = vec4<f32>(0.0); new_sh.c1 = vec4<f32>(0.0);
    new_sh.c2 = vec4<f32>(0.0); new_sh.c3 = vec4<f32>(0.0);
    new_sh.c4 = vec4<f32>(0.0); new_sh.c5 = vec4<f32>(0.0);
    new_sh.c6 = vec4<f32>(0.0); new_sh.c7 = vec4<f32>(0.0);
    new_sh.c8 = vec4<f32>(0.0);

    // Sample 14 directions (6 axis + 8 diagonal)
    let total_samples = 14.0;
    let weight = 4.0 * 3.14159265 / total_samples; // uniform sphere PDF

    for (var i = 0; i < 6; i++) {
        let dir = SAMPLE_DIRS[i];
        let radiance = evaluate_radiance(pos, dir);
        let basis = sh_basis(dir);

        new_sh.c0 += vec4<f32>(radiance * basis[0] * weight, 0.0);
        new_sh.c1 += vec4<f32>(radiance * basis[1] * weight, 0.0);
        new_sh.c2 += vec4<f32>(radiance * basis[2] * weight, 0.0);
        new_sh.c3 += vec4<f32>(radiance * basis[3] * weight, 0.0);
        new_sh.c4 += vec4<f32>(radiance * basis[4] * weight, 0.0);
        new_sh.c5 += vec4<f32>(radiance * basis[5] * weight, 0.0);
        new_sh.c6 += vec4<f32>(radiance * basis[6] * weight, 0.0);
        new_sh.c7 += vec4<f32>(radiance * basis[7] * weight, 0.0);
        new_sh.c8 += vec4<f32>(radiance * basis[8] * weight, 0.0);
    }

    for (var i = 0; i < 8; i++) {
        let dir = EXTRA_DIRS[i];
        let radiance = evaluate_radiance(pos, dir);
        let basis = sh_basis(dir);

        new_sh.c0 += vec4<f32>(radiance * basis[0] * weight, 0.0);
        new_sh.c1 += vec4<f32>(radiance * basis[1] * weight, 0.0);
        new_sh.c2 += vec4<f32>(radiance * basis[2] * weight, 0.0);
        new_sh.c3 += vec4<f32>(radiance * basis[3] * weight, 0.0);
        new_sh.c4 += vec4<f32>(radiance * basis[4] * weight, 0.0);
        new_sh.c5 += vec4<f32>(radiance * basis[5] * weight, 0.0);
        new_sh.c6 += vec4<f32>(radiance * basis[6] * weight, 0.0);
        new_sh.c7 += vec4<f32>(radiance * basis[7] * weight, 0.0);
        new_sh.c8 += vec4<f32>(radiance * basis[8] * weight, 0.0);
    }

    // Temporal blend with previous SH (hysteresis)
    let h = params.hysteresis;
    let old = probes[probe_idx];

    probes[probe_idx].c0 = mix(new_sh.c0, old.c0, h);
    probes[probe_idx].c1 = mix(new_sh.c1, old.c1, h);
    probes[probe_idx].c2 = mix(new_sh.c2, old.c2, h);
    probes[probe_idx].c3 = mix(new_sh.c3, old.c3, h);
    probes[probe_idx].c4 = mix(new_sh.c4, old.c4, h);
    probes[probe_idx].c5 = mix(new_sh.c5, old.c5, h);
    probes[probe_idx].c6 = mix(new_sh.c6, old.c6, h);
    probes[probe_idx].c7 = mix(new_sh.c7, old.c7, h);
    probes[probe_idx].c8 = mix(new_sh.c8, old.c8, h);
}
