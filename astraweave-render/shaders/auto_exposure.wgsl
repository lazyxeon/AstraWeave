// Auto-exposure via luminance histogram
//
// Two-pass system:
// 1. Histogram pass: compute a 256-bin luminance histogram of the HDR image
// 2. Average pass: compute weighted average luminance, apply temporal adaptation

struct ExposureParams {
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    min_log_lum: f32,       // Minimum log2 luminance (e.g., -10)
    max_log_lum: f32,       // Maximum log2 luminance (e.g., 2)
    time_delta: f32,         // Frame delta time for adaptation
    adaptation_speed: f32,   // How fast exposure adapts (1-10)
    low_percentile: f32,     // Ignore darkest N% (e.g., 0.1)
    high_percentile: f32,    // Ignore brightest N% (e.g., 0.95)
    target_exposure: f32,    // Manual override (0 = auto)
    _pad: f32,
};

@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> params: ExposureParams;
@group(0) @binding(3) var<storage, read_write> histogram: array<atomic<u32>, 256>;
@group(0) @binding(4) var<storage, read_write> exposure_data: array<f32, 2>; // [0]=current, [1]=target

// Compute luminance from linear RGB
fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Map luminance to histogram bin
fn luminance_to_bin(lum: f32) -> u32 {
    if (lum < 0.001) {
        return 0u;
    }
    let log_lum = log2(lum);
    let t = (log_lum - params.min_log_lum) / (params.max_log_lum - params.min_log_lum);
    return clamp(u32(t * 255.0), 0u, 255u);
}

fn bin_to_luminance(bin: u32) -> f32 {
    let t = (f32(bin) + 0.5) / 256.0;
    let log_lum = params.min_log_lum + t * (params.max_log_lum - params.min_log_lum);
    return exp2(log_lum);
}

// Pass 1: Build histogram
@compute @workgroup_size(16, 16, 1)
fn histogram_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
    let color = textureSampleLevel(hdr_tex, samp, uv, 0.0).rgb;
    let lum = luminance(color);
    let bin = luminance_to_bin(lum);

    atomicAdd(&histogram[bin], 1u);
}

// Pass 2: Compute average luminance from histogram and adapt exposure
@compute @workgroup_size(1, 1, 1)
fn average_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total_pixels = u32(params.resolution.x * params.resolution.y);
    let low_count = u32(f32(total_pixels) * params.low_percentile);
    let high_count = u32(f32(total_pixels) * params.high_percentile);

    // Accumulate histogram, ignoring low and high percentiles
    var running_count = 0u;
    var weighted_sum = 0.0;
    var valid_count = 0u;

    for (var bin = 0u; bin < 256u; bin++) {
        let count = atomicLoad(&histogram[bin]);
        let prev_running = running_count;
        running_count += count;

        // Skip bins below low percentile or above high percentile
        if (running_count <= low_count || prev_running >= high_count) {
            // Reset bin for next frame
            atomicStore(&histogram[bin], 0u);
            continue;
        }

        // Partial inclusion for boundary bins
        var included = count;
        if (prev_running < low_count) {
            included -= (low_count - prev_running);
        }
        if (running_count > high_count) {
            included -= (running_count - high_count);
        }

        let bin_lum = bin_to_luminance(bin);
        weighted_sum += bin_lum * f32(included);
        valid_count += included;

        // Reset bin for next frame
        atomicStore(&histogram[bin], 0u);
    }

    let avg_lum = select(0.18, weighted_sum / f32(max(valid_count, 1u)), valid_count > 0u);

    // Target exposure: EV = log2(avg_lum / 0.18)
    let target_ev = -log2(avg_lum / 0.18 + 0.001);

    // Temporal adaptation (smooth transition)
    let current_ev = exposure_data[0];
    let adapted_ev = mix(current_ev, target_ev, 1.0 - exp(-params.time_delta * params.adaptation_speed));

    // Manual override
    let final_ev = select(adapted_ev, params.target_exposure, params.target_exposure != 0.0);

    exposure_data[0] = final_ev;
    exposure_data[1] = target_ev;
}
