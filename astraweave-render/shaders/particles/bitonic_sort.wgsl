// GPU Bitonic Sort — Sort particles by camera distance for correct alpha blending
//
// Bitonic merge sort: O(n log²n) comparisons, fully parallelizable.
// Each dispatch performs one compare-and-swap pass. The host dispatches
// log2(n) * (log2(n)+1) / 2 passes total.
//
// Sorts by decreasing distance (back-to-front for transparency).

struct SortParams {
    algo_step:      u32,  // current comparison distance (power of 2)
    stage_step:     u32,  // current sub-pass within stage
    num_particles:  u32,
    _pad:           u32,
    camera_pos:     vec3<f32>,
    _pad2:          f32,
};

struct SortEntry {
    index:    u32,
    distance: f32,
};

@group(0) @binding(0) var<uniform>              params:  SortParams;
@group(0) @binding(1) var<storage, read_write>  entries: array<SortEntry>;

@compute @workgroup_size(256)
fn bitonic_sort(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.num_particles) {
        return;
    }

    let half_step = params.stage_step;
    let step = params.algo_step;

    // Determine partner index for this compare-and-swap
    let block = idx / step;
    let offset = idx % half_step;
    let partner = select(
        idx + half_step,      // first half of block: compare with second half
        idx - half_step,      // second half of block: compare with first half
        (idx % step) < half_step
    );

    if (partner >= params.num_particles) {
        return;
    }

    let a = entries[idx];
    let b = entries[partner];

    // Determine sort direction for this block
    // Bitonic sort alternates ascending/descending by block
    let ascending = ((idx / step) % 2u) == 0u;

    // Compare: we want back-to-front (descending distance)
    let should_swap = select(
        a.distance < b.distance,   // ascending block: swap if a < b (wrong order for back-to-front)
        a.distance > b.distance,   // descending block: swap if a > b
        ascending
    );

    // Only the "lower" index in each pair performs the swap
    if ((idx % step) < half_step && should_swap) {
        entries[idx] = b;
        entries[partner] = a;
    }
}
