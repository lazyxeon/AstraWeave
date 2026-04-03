// Lumen Final Gather — Multi-bounce diffuse indirect lighting
//
// Composites three GI sources into a unified indirect diffuse result:
//   1. Screen-space GI (SSGI) — near-field, high-detail indirect
//   2. Surface cache probes (SH) — far-field, multi-bounce irradiance
//   3. Distance-field AO (DFAO) — long-range occlusion modulation
//
// The final gather also applies spatial denoising and temporal reprojection
// for flicker-free output.

struct FinalGatherParams {
    inv_view_proj:      mat4x4<f32>,
    prev_view_proj:     mat4x4<f32>,
    resolution:         vec2<f32>,
    inv_resolution:     vec2<f32>,
    // Surface cache grid
    grid_origin:        vec3<f32>,
    probe_spacing:      f32,
    grid_dims:          vec3<u32>,
    ssgi_weight:        f32,       // weight for screen-space GI (0-1)
    probe_weight:       f32,       // weight for probe irradiance (0-1)
    dfao_weight:        f32,       // weight for DFAO modulation (0-1)
    temporal_blend:     f32,       // history blend factor
    frame_index:        u32,
    near_plane:         f32,
    far_plane:          f32,
    _pad0:              f32,
    _pad1:              f32,
};

// Probe SH L2 (must match surface_cache layout)
struct ProbeSH {
    c0: vec4<f32>,
    c1: vec4<f32>,
    c2: vec4<f32>,
    c3: vec4<f32>,
    c4: vec4<f32>,
    c5: vec4<f32>,
    c6: vec4<f32>,
    c7: vec4<f32>,
    c8: vec4<f32>,
};

@group(0) @binding(0)  var<uniform>             params:     FinalGatherParams;
@group(0) @binding(1)  var                      t_depth:    texture_2d<f32>;
@group(0) @binding(2)  var                      t_normal:   texture_2d<f32>;
@group(0) @binding(3)  var                      t_albedo:   texture_2d<f32>;
@group(0) @binding(4)  var                      t_ssgi:     texture_2d<f32>;
@group(0) @binding(5)  var                      t_dfao:     texture_2d<f32>;
@group(0) @binding(6)  var                      t_velocity: texture_2d<f32>;
@group(0) @binding(7)  var                      t_history:  texture_2d<f32>;
@group(0) @binding(8)  var<storage, read>       probes:     array<ProbeSH>;
@group(0) @binding(9)  var                      s_linear:   sampler;
@group(0) @binding(10) var                      t_output:   texture_storage_2d<rgba16float, write>;

// ---- SH evaluation ----

fn sh_basis(dir: vec3<f32>) -> array<f32, 9> {
    let x = dir.x; let y = dir.y; let z = dir.z;
    var b: array<f32, 9>;
    b[0] = 0.282095;
    b[1] = 0.488603 * y;
    b[2] = 0.488603 * z;
    b[3] = 0.488603 * x;
    b[4] = 1.092548 * x * y;
    b[5] = 1.092548 * y * z;
    b[6] = 0.315392 * (3.0 * z * z - 1.0);
    b[7] = 1.092548 * x * z;
    b[8] = 0.546274 * (x * x - y * y);
    return b;
}

fn evaluate_probe(probe: ProbeSH, normal: vec3<f32>) -> vec3<f32> {
    let b = sh_basis(normal);
    var result = vec3<f32>(0.0);
    result += probe.c0.xyz * b[0];
    result += probe.c1.xyz * b[1];
    result += probe.c2.xyz * b[2];
    result += probe.c3.xyz * b[3];
    result += probe.c4.xyz * b[4];
    result += probe.c5.xyz * b[5];
    result += probe.c6.xyz * b[6];
    result += probe.c7.xyz * b[7];
    result += probe.c8.xyz * b[8];
    return max(result, vec3<f32>(0.0));
}

// ---- Probe grid interpolation ----

fn probe_index(ix: u32, iy: u32, iz: u32) -> u32 {
    return iz * params.grid_dims.x * params.grid_dims.y + iy * params.grid_dims.x + ix;
}

fn sample_probe_grid(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let rel = (world_pos - params.grid_origin) / params.probe_spacing;
    let base = vec3<i32>(floor(rel));
    let frac = fract(rel);

    var result = vec3<f32>(0.0);
    let dims = vec3<i32>(params.grid_dims);

    for (var dz = 0; dz < 2; dz++) {
        for (var dy = 0; dy < 2; dy++) {
            for (var dx = 0; dx < 2; dx++) {
                let ix = clamp(base.x + dx, 0, dims.x - 1);
                let iy = clamp(base.y + dy, 0, dims.y - 1);
                let iz = clamp(base.z + dz, 0, dims.z - 1);
                let idx = probe_index(u32(ix), u32(iy), u32(iz));

                let wx = select(1.0 - frac.x, frac.x, dx == 1);
                let wy = select(1.0 - frac.y, frac.y, dy == 1);
                let wz = select(1.0 - frac.z, frac.z, dz == 1);
                let w = wx * wy * wz;

                // Visibility weighting: prefer probes on the same side of the surface
                let probe_pos = params.grid_origin + vec3<f32>(f32(ix), f32(iy), f32(iz)) * params.probe_spacing;
                let to_probe = normalize(probe_pos - world_pos);
                let vis_weight = max(dot(to_probe, normal) * 0.5 + 0.5, 0.05);

                let irradiance = evaluate_probe(probes[idx], normal);
                result += irradiance * w * vis_weight;
            }
        }
    }

    return result;
}

// ---- World position reconstruction ----

fn reconstruct_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world_h = params.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
}

// ---- Temporal reprojection ----

fn reproject_uv(world_pos: vec3<f32>) -> vec2<f32> {
    let prev_clip = params.prev_view_proj * vec4<f32>(world_pos, 1.0);
    let prev_ndc = prev_clip.xy / prev_clip.w;
    return prev_ndc * 0.5 + 0.5;
}

@compute @workgroup_size(8, 8)
fn final_gather_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(textureDimensions(t_depth));
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
    let depth_raw = textureLoad(t_depth, pixel, 0).r;

    // Sky pixels: pass through zero indirect
    if (depth_raw >= 1.0) {
        textureStore(t_output, pixel, vec4<f32>(0.0));
        return;
    }

    let world_pos = reconstruct_world_pos(uv, depth_raw);
    let normal = normalize(textureLoad(t_normal, pixel, 0).xyz * 2.0 - 1.0);
    let albedo = textureLoad(t_albedo, pixel, 0).rgb;

    // --- Source 1: Screen-space GI ---
    let ssgi = textureLoad(t_ssgi, pixel, 0).rgb;

    // --- Source 2: Probe irradiance ---
    let probe_irr = sample_probe_grid(world_pos, normal);

    // --- Source 3: DFAO ---
    let dfao = textureLoad(t_dfao, pixel, 0).r;

    // Blend SSGI (near-field) and probes (far-field) with DFAO modulation
    let gi_combined = ssgi * params.ssgi_weight + probe_irr * params.probe_weight;
    let ao_modulated = gi_combined * mix(1.0, dfao, params.dfao_weight);

    // Apply albedo (Lambertian diffuse: E * albedo / pi)
    let indirect = ao_modulated * albedo * 0.318310; // 1/pi

    // --- Temporal reprojection ---
    let prev_uv = reproject_uv(world_pos);
    var result = indirect;

    if (all(prev_uv >= vec2<f32>(0.0)) && all(prev_uv <= vec2<f32>(1.0))) {
        let history = textureSampleLevel(t_history, s_linear, prev_uv, 0.0).rgb;

        // Simple neighborhood clamping (cross-shaped, 5 taps)
        let c = indirect;
        let n = textureLoad(t_ssgi, pixel + vec2<i32>(0, -1), 0).rgb * params.ssgi_weight;
        let s = textureLoad(t_ssgi, pixel + vec2<i32>(0,  1), 0).rgb * params.ssgi_weight;
        let e = textureLoad(t_ssgi, pixel + vec2<i32>( 1, 0), 0).rgb * params.ssgi_weight;
        let w = textureLoad(t_ssgi, pixel + vec2<i32>(-1, 0), 0).rgb * params.ssgi_weight;

        let min_color = min(min(min(n, s), min(e, w)), c);
        let max_color = max(max(max(n, s), max(e, w)), c);

        let clamped_history = clamp(history, min_color, max_color);

        // Velocity-based blend: fast motion = less history
        let velocity = textureLoad(t_velocity, pixel, 0).rg;
        let speed = length(velocity * params.resolution);
        let dynamic_blend = mix(params.temporal_blend, 0.3, saturate(speed * 0.05));

        result = mix(indirect, clamped_history, dynamic_blend);
    }

    textureStore(t_output, pixel, vec4<f32>(max(result, vec3<f32>(0.0)), 1.0));
}
