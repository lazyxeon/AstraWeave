// MegaLights Stage 1: Count Lights Per Cluster
// 
// Purpose: Parallel count of lights affecting each cluster
// Algorithm: Each thread processes one cluster, tests all lights against AABB
// Output: light_counts[cluster_idx] = number of intersecting lights
//
// Performance: O(N × M) but fully parallel (GPU crushes this)
// Estimated: <50µs @ 1000 lights × 8192 clusters

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
) {
    // Compute linear cluster index
    let cluster_idx = gid.x + 
                      gid.y * params.cluster_dims.x + 
                      gid.z * (params.cluster_dims.x * params.cluster_dims.y);
    
    // Early exit for threads beyond cluster count
    if (cluster_idx >= params.total_clusters) {
        return;
    }
    
    // Load cluster bounds (cached in registers)
    let cluster = clusters[cluster_idx];
    var count = 0u;
    
    // Test each light against this cluster
    // (Tight loop, GPU auto-vectorizes)
    for (var i = 0u; i < params.light_count; i++) {
        let light = lights[i];
        let light_pos = light.position.xyz;
        let light_radius = light.position.w;
        
        if (sphere_intersects_aabb(light_pos, light_radius, cluster.min_pos, cluster.max_pos)) {
            count++;
        }
    }
    
    // Atomic write (safe, no conflicts across threads)
    atomicStore(&light_counts[cluster_idx], count);
}
