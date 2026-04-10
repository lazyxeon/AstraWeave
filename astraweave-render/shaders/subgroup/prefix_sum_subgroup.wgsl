// Subgroup-optimized Blelloch prefix sum (exclusive scan)
//
// Requires: enable subgroups;
// Uses subgroupExclusiveAdd for intra-subgroup scan, then a small
// workgroup-level fixup for inter-subgroup offsets.
//
// Performance improvement over standard Blelloch:
// - Standard: O(log n) up-sweep + O(log n) down-sweep, each with workgroupBarrier()
// - Subgroup: subgroupExclusiveAdd replaces log2(sg_size) steps in both phases
// - For sg_size=32: eliminates 10 of 18 barriers for 512 elements

enable subgroups;

struct PrefixSumParams {
    element_count: u32,
    workgroup_size: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: PrefixSumParams;
@group(0) @binding(3) var<storage, read_write> block_sums: array<u32>;

// Each subgroup produces a partial sum; we store those for inter-subgroup fixup
var<workgroup> subgroup_totals: array<u32, 16>; // max 16 subgroups per workgroup
var<workgroup> subgroup_offsets: array<u32, 16>;

@compute @workgroup_size(256, 1, 1)
fn prefix_sum(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(local_invocation_index) li: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let tid = lid.x;
    let gid_1d = gid.x;

    // Each thread handles 2 elements (same as original)
    let idx0 = gid_1d * 2u;
    let idx1 = gid_1d * 2u + 1u;

    var val0 = 0u;
    var val1 = 0u;

    if (idx0 < params.element_count) {
        val0 = input[idx0];
    }
    if (idx1 < params.element_count) {
        val1 = input[idx1];
    }

    // Step 1: Each thread sums its two values. We'll scan the per-thread sums,
    // then reconstruct both output elements.
    let thread_sum = val0 + val1;

    // Step 2: Intra-subgroup exclusive scan using subgroupExclusiveAdd
    // This replaces log2(sg_size) steps of the up-sweep + down-sweep
    let sg_exclusive = subgroupExclusiveAdd(thread_sum);

    // Step 3: The last thread in each subgroup stores the subgroup total
    let subgroup_index = tid / sg_size;
    let sg_total = sg_exclusive + thread_sum; // inclusive sum for this thread
    if (sg_id == sg_size - 1u) {
        subgroup_totals[subgroup_index] = sg_total;
    }
    workgroupBarrier();

    // Step 4: First subgroup scans the subgroup totals (inter-subgroup fixup)
    let num_subgroups = (256u + sg_size - 1u) / sg_size;
    if (subgroup_index == 0u && sg_id < num_subgroups) {
        let total = subgroup_totals[sg_id];
        let offset = subgroupExclusiveAdd(total);
        subgroup_offsets[sg_id] = offset;
    }
    workgroupBarrier();

    // Step 5: Combine subgroup offset + intra-subgroup scan
    let base_offset = subgroup_offsets[subgroup_index];
    let thread_exclusive = base_offset + sg_exclusive;

    // Step 6: Reconstruct both output elements
    // thread_exclusive is the exclusive prefix sum for thread_sum
    // val0 goes at thread_exclusive, val1 goes at thread_exclusive + val0
    if (idx0 < params.element_count) {
        output[idx0] = thread_exclusive;
    }
    if (idx1 < params.element_count) {
        output[idx1] = thread_exclusive + val0;
    }

    // Step 7: Write block sum for multi-pass scan
    if (tid == 0u) {
        let last_sg = num_subgroups - 1u;
        block_sums[wid.x] = subgroup_offsets[last_sg] + subgroup_totals[last_sg];
    }
}
