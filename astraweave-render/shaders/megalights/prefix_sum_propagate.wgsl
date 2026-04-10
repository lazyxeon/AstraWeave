// MegaLights: Propagate block offsets after multi-pass prefix sum
//
// Pass 3 of the 3-pass parallel prefix sum:
// 1. Local scan per block (prefix_sum.wgsl) → locally scanned blocks + block sums
// 2. Scan block sums (prefix_sum.wgsl with block_sums as input)
// 3. Add block offsets to each element (this shader)

struct PrefixSumParams {
    element_count: u32,
    workgroup_size: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@group(0) @binding(1) var<storage, read> block_offsets: array<u32>;
@group(0) @binding(2) var<uniform> params: PrefixSumParams;

@compute @workgroup_size(256, 1, 1)
fn propagate_offsets(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let offset = block_offsets[wid.x];

    // Each thread handles 2 elements (matching scan shader)
    let idx0 = gid.x * 2u;
    let idx1 = gid.x * 2u + 1u;

    if (idx0 < params.element_count) {
        data[idx0] += offset;
    }
    if (idx1 < params.element_count) {
        data[idx1] += offset;
    }
}
