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

// Pass 1: Build histogram using per-workgroup shared histogram to reduce global atomic contention
var<workgroup> local_histogram: array<atomic<u32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn histogram_pass(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
) {
    // Initialize local histogram (256 threads match 256 bins exactly)
    if (li < 256u) {
        atomicStore(&local_histogram[li], 0u);
    }
    workgroupBarrier();

    // Each thread bins its pixel into the per-workgroup shared histogram
    let dims = vec2<u32>(params.resolution);
    if (gid.x < dims.x && gid.y < dims.y) {
        let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
        let color = textureSampleLevel(hdr_tex, samp, uv, 0.0).rgb;
        let lum = luminance(color);
        let bin = luminance_to_bin(lum);
        atomicAdd(&local_histogram[bin], 1u);
    }
    workgroupBarrier();

    // Merge local histogram into global (each thread merges one bin)
    if (li < 256u) {
        let count = atomicLoad(&local_histogram[li]);
        if (count > 0u) {
            atomicAdd(&histogram[li], count);
        }
    }
}

// Pass 2: Compute average luminance from histogram and adapt exposure
// Uses 256 threads (one per bin) with workgroup shared memory reduction
var<workgroup> shared_weighted: array<f32, 256>;
var<workgroup> shared_count: array<u32, 256>;

@compute @workgroup_size(256, 1, 1)
fn average_pass(@builtin(local_invocation_id) lid: vec3<u32>) {
    let bin = lid.x;
    let total_pixels = u32(params.resolution.x * params.resolution.y);
    let low_count = u32(f32(total_pixels) * params.low_percentile);
    let high_count = u32(f32(total_pixels) * params.high_percentile);

    // Each thread loads its histogram bin count
    let count = atomicLoad(&histogram[bin]);
    // Reset bin for next frame
    atomicStore(&histogram[bin], 0u);

    // Compute exclusive prefix sum of counts to determine bin boundaries
    // Use a simple serial scan from thread 0 (fast for 256 elements)
    // We store per-bin running totals in shared memory
    shared_count[bin] = count;
    workgroupBarrier();

    // Thread 0 does a serial prefix scan over 256 elements and computes
    // per-bin included counts, then writes partial results for all threads
    // (This is fast: 256 iterations on one warp, <1µs)
    var weighted_sum = 0.0;
    var valid_count = 0u;

    if (bin == 0u) {
        var running_count = 0u;
        for (var i = 0u; i < 256u; i++) {
            let c = shared_count[i];
            let prev_running = running_count;
            running_count += c;

            if (running_count <= low_count || prev_running >= high_count) {
                shared_weighted[i] = 0.0;
                shared_count[i] = 0u;
                continue;
            }

            var included = c;
            if (prev_running < low_count) {
                included -= (low_count - prev_running);
            }
            if (running_count > high_count) {
                included -= (running_count - high_count);
            }

            let bin_lum = bin_to_luminance(i);
            shared_weighted[i] = bin_lum * f32(included);
            shared_count[i] = included;
        }
    }
    workgroupBarrier();

    // Parallel reduction of weighted_sum and valid_count using shared memory
    // Each thread starts with its own bin's contribution
    var my_weighted = shared_weighted[bin];
    var my_count = shared_count[bin];

    // Tree reduction: 8 steps for 256 elements (log2(256) = 8)
    for (var stride = 128u; stride > 0u; stride >>= 1u) {
        if (bin < stride) {
            my_weighted += shared_weighted[bin + stride];
            my_count += shared_count[bin + stride];
            shared_weighted[bin] = my_weighted;
            shared_count[bin] = my_count;
        }
        workgroupBarrier();
    }

    // Thread 0 writes final exposure
    if (bin == 0u) {
        let final_weighted = shared_weighted[0];
        let final_count = shared_count[0];

        let avg_lum = select(0.18, final_weighted / f32(max(final_count, 1u)), final_count > 0u);

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
}
