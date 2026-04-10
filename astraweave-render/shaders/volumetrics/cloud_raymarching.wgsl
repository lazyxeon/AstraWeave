// Volumetric Clouds — Perlin-Worley Raymarching
//
// Compute shader that raymarches through a cloud shell layer to produce
// physically-based volumetric clouds. Based on Schneider & Vos (SIGGRAPH 2015)
// "The Real-Time Volumetric Cloudscapes of Horizon Zero Dawn."
//
// Pipeline:
//   1. Reconstruct view ray from screen UV
//   2. Intersect with cloud shell [altitude, altitude + thickness]
//   3. March through shell, sampling density from procedural noise
//   4. Compute lighting via Beer-Powder law + dual-lobe HG phase
//   5. Blend with temporal history for stability
//
// Output: RGBA16Float at half resolution
//   RGB = in-scattered cloud radiance
//   A   = transmittance (1.0 = fully transparent, 0.0 = fully opaque)

struct CloudParams {
    inv_view_proj:       mat4x4<f32>,
    view_pos:            vec3<f32>,
    near_plane:          f32,
    sun_dir:             vec3<f32>,
    sun_intensity:       f32,
    sun_color:           vec3<f32>,
    cloud_altitude:      f32,     // bottom of cloud layer (world Y)
    cloud_thickness:     f32,     // vertical extent
    cloud_coverage:      f32,     // 0..1 coverage fraction
    cloud_density:       f32,     // density multiplier
    cloud_speed:         f32,     // wind animation speed
    wind_dir:            vec3<f32>,
    time:                f32,
    resolution:          vec2<f32>,
    inv_resolution:      vec2<f32>,
    max_steps:           u32,
    light_steps:         u32,
    anisotropy_fwd:      f32,
    anisotropy_bck:      f32,
    extinction_coeff:    f32,
    scatter_coeff:       f32,
    ambient_intensity:   f32,
    temporal_blend:      f32,
    frame_index:         u32,
    _pad0:               f32,
    _pad1:               f32,
    _pad2:               f32,
};

@group(0) @binding(0) var<uniform>  params:    CloudParams;
@group(0) @binding(1) var           t_history: texture_2d<f32>;   // previous frame cloud result
@group(0) @binding(2) var           s_linear:  sampler;
@group(0) @binding(3) var           t_output:  texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265358979;

// ============================================================================
// Noise functions
// ============================================================================

// Hash for pseudo-random values in [0, 1]
fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Hash returning 3D vector
fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

// Gradient noise (Perlin-style) using hash-based gradients
fn gradient_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic Hermite interpolation (C2 continuous)
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    // Compute gradient dot products at 8 corners
    let n000 = dot(hash33(i + vec3<f32>(0.0, 0.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 0.0, 0.0));
    let n100 = dot(hash33(i + vec3<f32>(1.0, 0.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = dot(hash33(i + vec3<f32>(0.0, 1.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = dot(hash33(i + vec3<f32>(1.0, 1.0, 0.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = dot(hash33(i + vec3<f32>(0.0, 0.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = dot(hash33(i + vec3<f32>(1.0, 0.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = dot(hash33(i + vec3<f32>(0.0, 1.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = dot(hash33(i + vec3<f32>(1.0, 1.0, 1.0)) * 2.0 - 1.0, f - vec3<f32>(1.0, 1.0, 1.0));

    // Trilinear interpolation
    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
        u.z
    ) * 0.5 + 0.5; // Remap from [-1,1] to [0,1]
}

// Worley (cellular) noise — returns distance to nearest cell point
fn worley_noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let frac = fract(p);

    var min_dist = 1.0;

    // Check 3×3×3 neighborhood
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec3<f32>(f32(x), f32(y), f32(z));
                let neighbor = cell + offset;
                let point = offset + hash33(neighbor) - frac;
                let d = dot(point, point); // squared distance
                min_dist = min(min_dist, d);
            }
        }
    }

    return sqrt(min_dist);
}

// Perlin-Worley blend: combines smooth Perlin gradient with sharp Worley cells.
// Produces the characteristic cloud "billowy" shapes.
fn perlin_worley(p: vec3<f32>) -> f32 {
    let pn = gradient_noise(p);
    let wn = 1.0 - worley_noise(p); // Invert: high at cell centers
    // Remap Perlin using Worley as erosion mask
    return remap(pn, wn * 0.4, 1.0, 0.0, 1.0);
}

// Fractal Brownian Motion using Perlin-Worley, 3 octaves
fn fbm_perlin_worley(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.625;
    var freq = 1.0;
    for (var i = 0; i < 3; i++) {
        value += amplitude * perlin_worley(p * freq);
        freq *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Simple Worley FBM for detail erosion, 2 octaves
fn fbm_worley(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.625;
    var freq = 1.0;
    for (var i = 0; i < 2; i++) {
        value += amplitude * worley_noise(p * freq);
        freq *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Utility: remap value from [lo, hi] to [new_lo, new_hi], clamped
fn remap(value: f32, lo: f32, hi: f32, new_lo: f32, new_hi: f32) -> f32 {
    return new_lo + (clamp(value, lo, hi) - lo) / max(hi - lo, 0.0001) * (new_hi - new_lo);
}

// ============================================================================
// Phase functions
// ============================================================================

// Henyey-Greenstein phase function
fn phase_hg(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * PI * pow(denom, 1.5));
}

// Dual-lobe HG: forward silver lining + backward dark edge
fn dual_phase_hg(cos_theta: f32) -> f32 {
    let fwd = phase_hg(cos_theta, params.anisotropy_fwd);
    let bck = phase_hg(cos_theta, params.anisotropy_bck);
    return mix(fwd, bck, 0.3);
}

// ============================================================================
// Cloud density sampling
// ============================================================================

// Height gradient shapes the cloud vertically (cumulus profile)
fn height_gradient(height_frac: f32) -> f32 {
    // Cumulus: dense bottom, rounded top, thin edges
    let bottom = smoothstep(0.0, 0.12, height_frac);
    let top = smoothstep(1.0, 0.7, height_frac);
    return bottom * top;
}

// Weather function: analytical 2D coverage pattern
fn weather_coverage(xz: vec2<f32>) -> f32 {
    // Slowly varying large-scale coverage pattern
    let p = xz * 0.00004;
    let n1 = gradient_noise(vec3<f32>(p.x, 0.0, p.y));
    let n2 = gradient_noise(vec3<f32>(p.x * 2.3, 1.7, p.y * 2.3)) * 0.5;
    let pattern = n1 + n2;
    // Modulate by global coverage
    return smoothstep(1.0 - params.cloud_coverage, 1.0, pattern);
}

// Sample cloud density at a world position
fn sample_cloud_density(pos: vec3<f32>) -> f32 {
    // Height fraction [0, 1] within cloud layer
    let height_frac = (pos.y - params.cloud_altitude) / params.cloud_thickness;
    if (height_frac < 0.0 || height_frac > 1.0) {
        return 0.0;
    }

    // Height gradient (cumulus shape)
    let h_grad = height_gradient(height_frac);
    if (h_grad < 0.001) {
        return 0.0;
    }

    // Weather coverage
    let coverage = weather_coverage(pos.xz);
    if (coverage < 0.001) {
        return 0.0;
    }

    // Wind animation
    let wind_offset = params.wind_dir * params.time * params.cloud_speed;

    // Base shape noise (large-scale Perlin-Worley FBM)
    let base_pos = pos * 0.0003 + wind_offset;
    let base_noise = fbm_perlin_worley(base_pos);

    // Remap noise with coverage: higher coverage → more clouds
    let base_cloud = remap(base_noise, 1.0 - coverage, 1.0, 0.0, 1.0);
    var density = base_cloud * h_grad;

    // Early-out before expensive detail noise
    if (density < 0.01) {
        return 0.0;
    }

    // Detail erosion (high-frequency Worley, 2 octaves)
    let detail_pos = pos * 0.002 + wind_offset * 1.5;
    let detail = fbm_worley(detail_pos);
    // Erode edges with height-dependent intensity (more erosion at top)
    let erode_amount = mix(0.15, 0.45, height_frac);
    density = max(density - detail * erode_amount, 0.0);

    return density * params.cloud_density;
}

// ============================================================================
// Light marching (sun shadow)
// ============================================================================

fn light_march(pos: vec3<f32>, density_at_pos: f32) -> f32 {
    let step_count = params.light_steps;
    // March toward sun through cloud layer
    let light_dir = params.sun_dir;

    // Determine how far to march (to top of cloud layer)
    let top = params.cloud_altitude + params.cloud_thickness;
    var march_dist: f32;
    if (light_dir.y > 0.001) {
        march_dist = (top - pos.y) / light_dir.y;
    } else {
        march_dist = params.cloud_thickness * 2.0;
    }
    march_dist = min(march_dist, params.cloud_thickness * 3.0);

    let step_length = march_dist / f32(step_count);
    var total_density = 0.0;

    for (var i = 1u; i <= step_count; i++) {
        let sample_pos = pos + light_dir * (f32(i) * step_length);
        total_density += sample_cloud_density(sample_pos) * step_length;
    }

    // Beer-Powder approximation (Schneider 2015):
    // Beer's law for primary extinction + "powder" effect for dark self-shadowed edges
    let beer = exp(-total_density * params.extinction_coeff);
    let powder = 1.0 - exp(-total_density * params.extinction_coeff * 2.0);

    // Combine: Beer handles standard attenuation, powder adds multi-scatter brightening
    return beer * mix(1.0, powder, 0.5);
}

// ============================================================================
// Ray-sphere/plane intersection
// ============================================================================

// Intersect view ray with the horizontal cloud shell [y_bottom, y_top].
// Returns (t_enter, t_exit). If no intersection, t_enter >= t_exit.
fn intersect_cloud_shell(origin: vec3<f32>, dir: vec3<f32>) -> vec2<f32> {
    let y_bottom = params.cloud_altitude;
    let y_top = params.cloud_altitude + params.cloud_thickness;

    // Degenerate: ray is exactly horizontal
    if (abs(dir.y) < 0.0001) {
        if (origin.y >= y_bottom && origin.y <= y_top) {
            // Inside the layer, march a fixed distance
            return vec2<f32>(0.0, params.cloud_thickness * 5.0);
        }
        return vec2<f32>(1.0, 0.0); // No intersection
    }

    let t_bottom = (y_bottom - origin.y) / dir.y;
    let t_top = (y_top - origin.y) / dir.y;

    var t_enter = min(t_bottom, t_top);
    var t_exit = max(t_bottom, t_top);

    // Clamp to forward direction
    t_enter = max(t_enter, 0.0);

    // If camera is inside the cloud layer, start at origin
    if (origin.y >= y_bottom && origin.y <= y_top) {
        t_enter = 0.0;
    }

    return vec2<f32>(t_enter, t_exit);
}

// ============================================================================
// Temporal jitter (reduces banding from discrete steps)
// ============================================================================

fn temporal_jitter(pixel: vec2<u32>) -> f32 {
    // Interleaved gradient noise for per-pixel jitter
    let p = vec2<f32>(pixel) + 0.5;
    let frame = f32(params.frame_index % 16u);
    return fract(52.9829189 * fract(0.06711056 * p.x + 0.00583715 * p.y + frame * 0.618034));
}

// ============================================================================
// Main raymarching kernel
// ============================================================================

@compute @workgroup_size(8, 8, 1)
fn cloud_raymarch_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(u32(params.resolution.x), u32(params.resolution.y));
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    // Reconstruct view ray from screen UV
    let ndc = uv * 2.0 - 1.0;
    let clip_near = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    let clip_far = vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
    let world_near_h = params.inv_view_proj * clip_near;
    let world_far_h = params.inv_view_proj * clip_far;
    let world_near = world_near_h.xyz / world_near_h.w;
    let world_far = world_far_h.xyz / world_far_h.w;
    let ray_dir = normalize(world_far - world_near);

    // Intersect with cloud shell
    let t_range = intersect_cloud_shell(params.view_pos, ray_dir);
    let t_enter = t_range.x;
    let t_exit = t_range.y;

    if (t_enter >= t_exit || t_exit <= 0.0) {
        // No intersection — fully transparent
        let history = textureSampleLevel(t_history, s_linear, uv, 0.0);
        let result = mix(vec4<f32>(0.0, 0.0, 0.0, 1.0), history, params.temporal_blend);
        textureStore(t_output, vec2<i32>(gid.xy), result);
        return;
    }

    // Compute step size
    let march_distance = t_exit - t_enter;
    let step_count = min(params.max_steps, 64u);
    let step_length = march_distance / f32(step_count);

    // Temporal jitter: offset start position to reduce banding
    let jitter = temporal_jitter(gid.xy);
    let t_start = t_enter + jitter * step_length;

    // Phase function (constant for entire ray)
    let cos_theta = dot(ray_dir, params.sun_dir);
    let phase = dual_phase_hg(cos_theta);

    // March through cloud layer
    var accumulated_light = vec3<f32>(0.0);
    var transmittance = 1.0;

    for (var i = 0u; i < step_count; i++) {
        let t = t_start + f32(i) * step_length;
        if (t > t_exit) { break; }

        let pos = params.view_pos + ray_dir * t;
        let density = sample_cloud_density(pos);

        if (density <= 0.0) {
            continue;
        }

        // Extinction for this step
        let extinction = density * params.extinction_coeff;
        let step_extinction = extinction * step_length;
        let step_transmittance = exp(-step_extinction);

        // Light march toward sun
        let light_energy = light_march(pos, density);

        // In-scattered radiance (sun + ambient)
        let sun_scatter = params.sun_color * params.sun_intensity * light_energy * phase;
        let ambient_scatter = vec3<f32>(params.ambient_intensity);

        // Energy-conserving integration (Frostbite / Hillaire 2016)
        let scattering = (sun_scatter + ambient_scatter) * params.scatter_coeff * density;
        let scatter_integral = scattering * (1.0 - step_transmittance) / max(extinction, 0.0001);

        accumulated_light += transmittance * scatter_integral;
        transmittance *= step_transmittance;

        // Early termination when fully opaque
        if (transmittance < 0.005) {
            transmittance = 0.0;
            break;
        }
    }

    let current = vec4<f32>(accumulated_light, transmittance);

    // Temporal reprojection: blend with history for stability
    let history = textureSampleLevel(t_history, s_linear, uv, 0.0);
    let result = mix(current, history, params.temporal_blend);

    textureStore(t_output, vec2<i32>(gid.xy), result);
}
