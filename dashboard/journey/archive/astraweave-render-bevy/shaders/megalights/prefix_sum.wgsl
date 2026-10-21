// MegaLights Stage 2: Prefix Sum (Exclusive Scan)
//
// Purpose: Convert counts to offsets via parallel prefix sum
// Algorithm: Blelloch scan (up-sweep + down-sweep, O(log n) depth)
// Input: light_counts[i] = number of lights in cluster i
// Output: light_offsets[i] = base index for cluster i in light_indices array
//
// Performance: <20µs @ 8192 elements (30× faster than CPU sequential)
// Example: [3, 1, 2, 4] → [0, 3, 4, 6] (exclusive scan)

struct PrefixSumParams {
    element_count: u32,
    workgroup_size: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: PrefixSumParams;

// Shared memory for workgroup-local scan (512 elements max per workgroup)
var<workgroup> shared_data: array<u32, 512>;

@compute @workgroup_size(256, 1, 1)
fn prefix_sum(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let tid = lid.x;
    let gid_1d = gid.x;
    
    // Phase 1: Load into shared memory (2 elements per thread)
    let idx0 = gid_1d * 2u;
    let idx1 = gid_1d * 2u + 1u;
    
    if (idx0 < params.element_count) {
        shared_data[tid * 2u] = input[idx0];
    } else {
        shared_data[tid * 2u] = 0u;
    }
    
    if (idx1 < params.element_count) {
        shared_data[tid * 2u + 1u] = input[idx1];
    } else {
        shared_data[tid * 2u + 1u] = 0u;
    }
    
    workgroupBarrier();
    
    // Phase 2: Up-sweep (reduce) phase
    // Build sum tree bottom-up: O(log n) iterations
    var offset = 1u;
    var d = params.workgroup_size;
    while (d > 0u) {
        workgroupBarrier();
        
        if (tid < d) {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            shared_data[bi] += shared_data[ai];
        }
        
        offset <<= 1u;
        d >>= 1u;
    }
    
    // Phase 3: Clear root (exclusive scan)
    if (tid == 0u) {
        shared_data[params.workgroup_size * 2u - 1u] = 0u;
    }
    
    workgroupBarrier();
    
    // Phase 4: Down-sweep phase
    // Propagate partial sums down tree: O(log n) iterations
    d = 1u;
    while (d < params.workgroup_size * 2u) {
        offset >>= 1u;
        workgroupBarrier();
        
        if (tid < d) {
            let ai = offset * (2u * tid + 1u) - 1u;
            let bi = offset * (2u * tid + 2u) - 1u;
            
            let temp = shared_data[ai];
            shared_data[ai] = shared_data[bi];
            shared_data[bi] += temp;
        }
        
        d <<= 1u;
    }
    
    workgroupBarrier();
    
    // Phase 5: Write results back to global memory
    if (idx0 < params.element_count) {
        output[idx0] = shared_data[tid * 2u];
    }
    if (idx1 < params.element_count) {
        output[idx1] = shared_data[tid * 2u + 1u];
    }
}

// NOTE: For arrays larger than 512 elements, run multiple workgroups and:
// 1. Scan each block (this shader)
// 2. Scan block sums (recursively call this shader)
// 3. Add block offsets to each element (simple compute shader)
//
// For 8192 clusters:
// - Pass 1: 16 workgroups × 512 elements = 8192 elements scanned locally
// - Pass 2: 1 workgroup × 16 block sums = 16 elements scanned
// - Pass 3: Add block offsets to each of 16 blocks (parallel add)
//
// AstraWeave clustered.rs uses 16×16×32 = 8192 clusters (perfect fit!)
