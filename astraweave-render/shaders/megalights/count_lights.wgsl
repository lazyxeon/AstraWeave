// MegaLights Stage 1: Count Lights Per Cluster
// 
// Purpose: Parallel count of lights affecting each cluster
// Algorithm: Cooperative light batching — workgroup loads 64 lights into
//            shared memory, then each thread tests its cluster AABB locally.
//            Eliminates redundant global memory reads across threads.
// Output: light_counts[cluster_idx] = number of intersecting lights

const TILE_SIZE: u32 = 64u;

struct GpuLight {
    position: vec4<f32>,  // xyz = position, w = radius
    color: vec4<f32>,      // rgb = color, a = intensity
}

struct ClusterBounds {
    min_pos: vec3<f32>,
    _pad1: f32,
    max_pos: vec3<f32>,
    _pad2: f32,
}

struct CountParams {
    cluster_dims: vec3<u32>,
    _pad1: u32,
    total_clusters: u32,
    light_count: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<storage, read> lights: array<GpuLight>;
@group(0) @binding(1) var<storage, read> clusters: array<ClusterBounds>;
@group(0) @binding(2) var<storage, read_write> light_counts: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: CountParams;

// Shared memory: cooperatively loaded batch of lights
var<workgroup> shared_lights: array<GpuLight, 64>;

/// Sphere-AABB intersection test (conservative, fast)
fn sphere_intersects_aabb(
    center: vec3<f32>,
    radius: f32,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>
) -> bool {
    // Find closest point on AABB to sphere center
    let closest = clamp(center, aabb_min, aabb_max);
    
    // Check if distance to closest point <= radius
    let dist_sq = dot(center - closest, center - closest);
    return dist_sq <= radius * radius;
}

@compute @workgroup_size(64, 1, 1)
fn count_lights_per_cluster(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_index) lid: u32,
) {
    // Compute linear cluster index
    let cluster_idx = gid.x + 
                      gid.y * params.cluster_dims.x + 
                      gid.z * (params.cluster_dims.x * params.cluster_dims.y);
    
    // Load cluster bounds into registers (each thread has its own cluster)
    var cluster_min = vec3<f32>(0.0);
    var cluster_max = vec3<f32>(0.0);
    let valid = cluster_idx < params.total_clusters;
    if (valid) {
        let cluster = clusters[cluster_idx];
        cluster_min = cluster.min_pos;
        cluster_max = cluster.max_pos;
    }

    var count = 0u;
    let num_batches = (params.light_count + TILE_SIZE - 1u) / TILE_SIZE;

    for (var batch = 0u; batch < num_batches; batch++) {
        // Cooperative load: each thread loads one light into shared memory
        let light_idx = batch * TILE_SIZE + lid;
        if (light_idx < params.light_count) {
            shared_lights[lid] = lights[light_idx];
        }
        workgroupBarrier();

        // Test all lights in the batch against this thread's cluster
        let batch_end = min(TILE_SIZE, params.light_count - batch * TILE_SIZE);
        if (valid) {
            for (var i = 0u; i < batch_end; i++) {
                let light = shared_lights[i];
                if (sphere_intersects_aabb(light.position.xyz, light.position.w, cluster_min, cluster_max)) {
                    count++;
                }
            }
        }
        workgroupBarrier();
    }

    // Write final count
    if (valid) {
        atomicStore(&light_counts[cluster_idx], count);
    }
}
