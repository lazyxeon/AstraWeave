// Cloud Shadow Map — Orthographic Sun-Projected Cloud Transmittance
//
// One texel is one vertical column through the cloud layer, marched along
// the sun. The stored R channel is Beer's-law transmittance
// (1 = lit, 0 = shadowed). GBA are unused.
//
// Cost, measured on lavapipe (debug, validation on, 512²):
// - The 32-step Perlin-Worley march is the pass. Worley is about three
//   quarters of that time. A gradient-only stand-in is much faster and
//   does not match the shadowed ground, so the Worley taps stay.
// - `weather_coverage` depends only on column XZ. Sampling it once per
//   texel matches the old per-step value bit-for-bit and skips the FBM
//   on clear columns.
// - Workgroups 8×8, 16×8, and 16×16 landed within 1% of each other.
//   8×8 stays.
// - R16Float is filterable but wgpu does not allow it as storage.
//   R32Float is storage but not linearly filterable without
//   FLOAT32_FILTERABLE. The PBR sampler is linear, so the map stays
//   rgba16float. A 512² store is 2 MiB; that is not this pass.
// - 16 march steps vs 32 changes the tonemapped ground by at most one
//   8-bit level, on 0.18% of texels. The column only spans ~0.2 noise
//   cells, so the extra 16 taps were resampling a smooth signal.

struct CloudShadowParams {
    // World-space center of the shadow map (camera XZ, cloud_altitude Y)
    center:          vec3<f32>,
    extent:          f32,          // half-extent in world units (e.g. 1024.0 for 2048m)
    sun_dir:         vec3<f32>,
    cloud_altitude:  f32,
    cloud_thickness: f32,
    cloud_coverage:  f32,
    cloud_density:   f32,
    cloud_speed:     f32,
    wind_dir:        vec3<f32>,
    time:            f32,
    extinction:      f32,
    shadow_steps:    u32,          // march steps through cloud layer
    resolution:      f32,          // shadow map resolution (pixels, square)
    _pad0:           f32,
    // This dispatch writes rows [y_begin, y_begin + row_count). The frame
    // index picks the slice. gid.y is the row inside the slice, not the
    // texture row, because the dispatch is only `row_count` tall.
    y_begin:         u32,
    row_count:       u32,
    _pad1:           u32,
    _pad2:           u32,
};

@group(0) @binding(0) var<uniform>  params:  CloudShadowParams;
@group(0) @binding(1) var           t_out:   texture_storage_2d<rgba16float, write>;

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

fn gradient_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let n000 = dot(hash33(i + vec3<f32>(0.0, 0.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 0.0, 0.0));
    let n100 = dot(hash33(i + vec3<f32>(1.0, 0.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = dot(hash33(i + vec3<f32>(0.0, 1.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = dot(hash33(i + vec3<f32>(1.0, 1.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = dot(hash33(i + vec3<f32>(0.0, 0.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = dot(hash33(i + vec3<f32>(1.0, 0.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = dot(hash33(i + vec3<f32>(0.0, 1.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = dot(hash33(i + vec3<f32>(1.0, 1.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 1.0, 1.0));
    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
        u.z
    ) * 0.5 + 0.5;
}

fn worley_noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let frac = fract(p);
    var min_dist = 1.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec3<f32>(f32(x), f32(y), f32(z));
                let neighbor = cell + offset;
                let point = offset + hash33(neighbor) - frac;
                let d = dot(point, point);
                min_dist = min(min_dist, d);
            }
        }
    }
    return sqrt(min_dist);
}

fn remap(value: f32, lo: f32, hi: f32, new_lo: f32, new_hi: f32) -> f32 {
    return new_lo + (clamp(value, lo, hi) - lo) / max(hi - lo, 0.0001) * (new_hi - new_lo);
}

fn perlin_worley(p: vec3<f32>) -> f32 {
    let pn = gradient_noise(p);
    let wn = 1.0 - worley_noise(p);
    return remap(pn, wn * 0.4, 1.0, 0.0, 1.0);
}

// 2-octave FBM for the shadow map (the full raymarch uses 3).
fn fbm_perlin_worley_low(p: vec3<f32>) -> f32 {
    return perlin_worley(p) * 0.625 + perlin_worley(p * 2.0) * 0.3125;
}

fn height_gradient(height_frac: f32) -> f32 {
    let bottom = smoothstep(0.0, 0.12, height_frac);
    let top = smoothstep(1.0, 0.7, height_frac);
    return bottom * top;
}

fn weather_coverage(xz: vec2<f32>) -> f32 {
    let p = xz * 0.00004;
    let n1 = gradient_noise(vec3<f32>(p.x, 0.0, p.y));
    let n2 = gradient_noise(vec3<f32>(p.x * 2.3, 1.7, p.y * 2.3)) * 0.5;
    let pattern = n1 + n2;
    return smoothstep(1.0 - params.cloud_coverage, 1.0, pattern);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn cloud_shadow_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let res = u32(params.resolution);
    let row = params.y_begin + gid.y;
    if (gid.x >= res || gid.y >= params.row_count || row >= res) { return; }

    let uv = (vec2<f32>(f32(gid.x), f32(row)) + 0.5) / params.resolution;
    let xz = params.center.xz + (uv - 0.5) * 2.0 * params.extent;

    // Clear columns used to pay for coverage on every step, then skip the FBM.
    let coverage = weather_coverage(xz);
    if (coverage < 0.001) {
        textureStore(t_out, vec2<i32>(i32(gid.x), i32(row)), vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    let cloud_top = params.cloud_altitude + params.cloud_thickness;
    let sun = params.sun_dir;
    var march_length = params.cloud_thickness;
    if (abs(sun.y) > 0.001) {
        march_length = params.cloud_thickness / abs(sun.y);
    }
    let column_start = vec3<f32>(xz.x, cloud_top, xz.y);
    let step_count = params.shadow_steps;
    let step_len = march_length / f32(max(step_count, 1u));
    let step_dir = -sun;
    let wind_offset = params.wind_dir * params.time * params.cloud_speed;

    var optical_depth = 0.0;
    for (var i = 0u; i < step_count; i++) {
        let t = (f32(i) + 0.5) * step_len;
        let sample_pos = column_start + step_dir * t;
        let height_frac = (sample_pos.y - params.cloud_altitude) / params.cloud_thickness;
        if (height_frac < 0.0 || height_frac > 1.0) { continue; }
        let h_grad = height_gradient(height_frac);
        if (h_grad < 0.001) { continue; }
        let base_pos = sample_pos * 0.0003 + wind_offset;
        let base_noise = fbm_perlin_worley_low(base_pos);
        let base_cloud = remap(base_noise, 1.0 - coverage, 1.0, 0.0, 1.0);
        optical_depth += max(base_cloud * h_grad, 0.0) * params.cloud_density * step_len;
    }

    let transmittance = exp(-optical_depth * params.extinction);
    textureStore(t_out, vec2<i32>(i32(gid.x), i32(row)), vec4<f32>(transmittance, 0.0, 0.0, 0.0));
}
