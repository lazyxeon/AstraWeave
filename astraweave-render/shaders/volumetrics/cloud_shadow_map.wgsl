// Cloud Shadow Map — Orthographic Sun-Projected Cloud Transmittance
//
// Compute shader that generates a 2D transmittance map for cloud shadows
// on terrain and world geometry. Centered on the camera, projects cloud
// density from the sun direction through the cloud layer.
//
// Output: Rgba16Float texture (transmittance in R: 1.0 = fully lit, 0.0 = fully shadowed)
//
// The noise functions are a simplified subset of the full raymarching shader
// (fewer octaves, no detail erosion) to keep the shadow map cheap at
// quarter-resolution (e.g. 512×512 for a 2048m× 2048m footprint).

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
};

@group(0) @binding(0) var<uniform>  params:  CloudShadowParams;
@group(0) @binding(1) var           t_out:   texture_storage_2d<rgba16float, write>;

// PI constant (provided by constants.wgsl prepended on Rust side)

// ============================================================================
// Simplified noise (matches raymarching convention but cheaper)
// ============================================================================

fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

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

// 2-octave FBM for shadow map (cheaper than 3-octave full raymarch)
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

// Simplified cloud density (no detail erosion — shadow map doesn't need it)
fn sample_density(pos: vec3<f32>) -> f32 {
    let height_frac = (pos.y - params.cloud_altitude) / params.cloud_thickness;
    if (height_frac < 0.0 || height_frac > 1.0) { return 0.0; }
    let h_grad = height_gradient(height_frac);
    if (h_grad < 0.001) { return 0.0; }
    let coverage = weather_coverage(pos.xz);
    if (coverage < 0.001) { return 0.0; }
    let wind_offset = params.wind_dir * params.time * params.cloud_speed;
    let base_pos = pos * 0.0003 + wind_offset;
    let base_noise = fbm_perlin_worley_low(base_pos);
    let base_cloud = remap(base_noise, 1.0 - coverage, 1.0, 0.0, 1.0);
    return max(base_cloud * h_grad, 0.0) * params.cloud_density;
}

// ============================================================================
// Main: project each shadow map texel through cloud layer
// ============================================================================

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn cloud_shadow_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let res = u32(params.resolution);
    if (gid.x >= res || gid.y >= res) { return; }

    // Map texel to world XZ, centered on camera
    let uv = (vec2<f32>(f32(gid.x), f32(gid.y)) + 0.5) / params.resolution;
    let xz = params.center.xz + (uv - 0.5) * 2.0 * params.extent;

    // March through cloud layer along sun direction.
    // Start at the TOP of the cloud layer (sun enters from above) and march downward.
    let cloud_top = params.cloud_altitude + params.cloud_thickness;
    let sun = params.sun_dir;

    // Compute start position: trace backward from ground (xz) along sun dir
    // to the top of the cloud layer.
    var start_y = cloud_top;
    var march_length = params.cloud_thickness;

    // If sun is not directly overhead, adjust the horizontal offset and march distance
    if (abs(sun.y) > 0.001) {
        march_length = params.cloud_thickness / abs(sun.y);
    }

    // Start from cloud top, project back from the ground point along -sun_dir
    // so the shadow falls at the correct ground position.
    let t_to_top = (cloud_top - params.cloud_altitude) / max(abs(sun.y), 0.001);
    let start_pos = vec3<f32>(xz.x, cloud_top, xz.y) - sun * t_to_top + sun * t_to_top;
    // Simplified: start at the column directly above the ground point
    let column_start = vec3<f32>(xz.x, cloud_top, xz.y);

    let step_count = params.shadow_steps;
    let step_len = march_length / f32(step_count);
    let step_dir = -sun; // march from top toward bottom along negative sun dir

    var optical_depth = 0.0;
    for (var i = 0u; i < step_count; i++) {
        let t = (f32(i) + 0.5) * step_len;
        let sample_pos = column_start + step_dir * t;
        optical_depth += sample_density(sample_pos) * step_len;
    }

    // Beer's law transmittance
    let transmittance = exp(-optical_depth * params.extinction);

    textureStore(t_out, vec2<i32>(gid.xy), vec4<f32>(transmittance, 0.0, 0.0, 0.0));
}
