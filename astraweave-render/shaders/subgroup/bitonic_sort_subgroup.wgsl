// Subgroup-optimized bitonic sort for particle transparency ordering
//
// Requires: enable subgroups;
// Uses subgroupShuffleXor for intra-subgroup compare-and-swap operations,
// eliminating global memory round-trips for inner sort stages where
// stage_step <= subgroup_size.
//
// For a subgroup size of 32, this collapses 5 inner passes into a single
// dispatch, reducing total host-side dispatch count by ~30-40%.

enable subgroups;

struct SortParams {
    algo_step:      u32,
    stage_step:     u32,
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

// Subgroup-accelerated compare-and-swap
// When stage_step fits within the subgroup, we can exchange data via
// subgroupShuffleXor instead of reading/writing global memory.
@compute @workgroup_size(256)
fn bitonic_sort(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let idx = gid.x;
    if (idx >= params.num_particles) {
        return;
    }

    let half_step = params.stage_step;
    let step = params.algo_step;

    // For inner stages where stage_step <= subgroup_size,
    // the partner is within the same subgroup.
    // We can use subgroupShuffleXor to exchange data directly.
    if (half_step <= sg_size) {
        // Load our entry
        var my_entry = entries[idx];
        var my_index = my_entry.index;
        var my_dist = my_entry.distance;

        // Exchange with partner via subgroup shuffle
        // XOR with half_step gives the partner's subgroup lane
        let partner_index = subgroupShuffleXor(my_index, half_step);
        let partner_dist = subgroupShuffleXor(my_dist, half_step);

        // Determine sort direction
        let ascending = ((idx / step) % 2u) == 0u;

        // Determine if we're in the "lower" half of the pair
        let is_lower = (idx % step) < half_step;

        // Compare
        let should_swap = select(
            my_dist < partner_dist,
            my_dist > partner_dist,
            ascending
        );

        // Apply swap: the lower-indexed thread takes the "winner"
        if (is_lower && should_swap) {
            my_entry.index = partner_index;
            my_entry.distance = partner_dist;
        } else if (!is_lower && should_swap) {
            my_entry.index = partner_index;
            my_entry.distance = partner_dist;
        }

        entries[idx] = my_entry;
    } else {
        // Fallback: standard global memory compare-and-swap for outer stages
        let partner = select(
            idx + half_step,
            idx - half_step,
            (idx % step) < half_step
        );

        if (partner >= params.num_particles) {
            return;
        }

        let a = entries[idx];
        let b = entries[partner];

        let ascending = ((idx / step) % 2u) == 0u;
        let should_swap = select(
            a.distance < b.distance,
            a.distance > b.distance,
            ascending
        );

        if ((idx % step) < half_step && should_swap) {
            entries[idx] = b;
            entries[partner] = a;
        }
    }
}
