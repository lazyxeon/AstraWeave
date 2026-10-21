// Nanite GPU Cluster Culling Compute Shader
// Phase 1 of 2-pass visibility: Cull meshlet clusters on GPU
//
// This shader performs three-stage culling:
// 1. Frustum culling (AABB vs frustum planes)
// 2. Occlusion culling (Hi-Z test against hierarchical depth buffer)
// 3. Backface culling (cone test against view direction)

struct Meshlet {
    bounds_min: vec3<f32>,
    vertex_offset: u32,
    bounds_max: vec3<f32>,
    vertex_count: u32,
    cone_apex: vec3<f32>,
    triangle_offset: u32,
    cone_axis: vec3<f32>,
    triangle_count: u32,
    cone_cutoff: f32,
    lod_level: u32,
    lod_error: f32,
    _padding: u32,
}

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding0: f32,
    view_dir: vec3<f32>,
    _padding1: f32,
    frustum_planes: array<vec4<f32>, 6>, // left, right, bottom, top, near, far
    hiz_size: vec2<u32>,
    hiz_mip_count: u32,
    screen_width: u32,
    screen_height: u32,
    enable_occlusion: u32, // 0 = disabled, 1 = enabled
    enable_backface: u32,
    lod_scale: f32,
}

struct CullStats {
    total_clusters: atomic<u32>,
    frustum_culled: atomic<u32>,
    occlusion_culled: atomic<u32>,
    backface_culled: atomic<u32>,
    visible_count: atomic<u32>,
}

@group(0) @binding(0) var<storage, read> meshlets: array<Meshlet>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read_write> visible_meshlets: array<u32>;
@group(0) @binding(3) var<storage, read_write> stats: CullStats;

// Hi-Z buffer for occlusion culling (previous frame depth)
@group(1) @binding(0) var hiz_texture: texture_2d<f32>;
@group(1) @binding(1) var hiz_sampler: sampler;

// Frustum culling: Test AABB against frustum planes
fn frustum_test_aabb(min_pos: vec3<f32>, max_pos: vec3<f32>) -> bool {
    for (var i = 0u; i < 6u; i++) {
        let plane = camera.frustum_planes[i];
        let normal = plane.xyz;
        let d = plane.w;
        
        // Find positive vertex (furthest along plane normal)
        let p = vec3<f32>(
            select(min_pos.x, max_pos.x, normal.x >= 0.0),
            select(min_pos.y, max_pos.y, normal.y >= 0.0),
            select(min_pos.z, max_pos.z, normal.z >= 0.0),
        );
        
        // If positive vertex is outside, AABB is outside
        if (dot(normal, p) + d < 0.0) {
            return false;
        }
    }
    return true;
}

// Hi-Z occlusion test: Check if AABB is visible using hierarchical depth
fn hiz_occlusion_test(min_pos: vec3<f32>, max_pos: vec3<f32>) -> bool {
    // Transform AABB corners to clip space
    let corners = array<vec3<f32>, 8>(
        min_pos,
        vec3<f32>(max_pos.x, min_pos.y, min_pos.z),
        vec3<f32>(min_pos.x, max_pos.y, min_pos.z),
        vec3<f32>(max_pos.x, max_pos.y, min_pos.z),
        vec3<f32>(min_pos.x, min_pos.y, max_pos.z),
        vec3<f32>(max_pos.x, min_pos.y, max_pos.z),
        vec3<f32>(min_pos.x, max_pos.y, max_pos.z),
        max_pos,
    );
    
    var screen_min = vec2<f32>(1e6, 1e6);
    var screen_max = vec2<f32>(-1e6, -1e6);
    var min_depth = 1e6;
    var behind_camera = 0u;
    
    // Project all corners and find screen-space bounds
    for (var i = 0u; i < 8u; i++) {
        let clip_pos = camera.view_proj * vec4<f32>(corners[i], 1.0);
        
        // Check if behind camera
        if (clip_pos.w <= 0.0) {
            behind_camera += 1u;
            continue;
        }
        
        let ndc = clip_pos.xyz / clip_pos.w;
        
        // Convert NDC to screen coordinates
        let screen_pos = vec2<f32>(
            (ndc.x * 0.5 + 0.5) * f32(camera.screen_width),
            (1.0 - (ndc.y * 0.5 + 0.5)) * f32(camera.screen_height),
        );
        
        screen_min = min(screen_min, screen_pos);
        screen_max = max(screen_max, screen_pos);
        min_depth = min(min_depth, ndc.z);
    }
    
    // If all corners behind camera, not visible
    if (behind_camera == 8u) {
        return false;
    }
    
    // Clamp to screen bounds
    screen_min = clamp(screen_min, vec2<f32>(0.0), vec2<f32>(f32(camera.screen_width), f32(camera.screen_height)));
    screen_max = clamp(screen_max, vec2<f32>(0.0), vec2<f32>(f32(camera.screen_width), f32(camera.screen_height)));
    
    let screen_size = screen_max - screen_min;
    if (screen_size.x <= 0.0 || screen_size.y <= 0.0) {
        return false; // Degenerate screen rect
    }
    
    // Select appropriate Hi-Z mip level based on screen size
    let max_dimension = max(screen_size.x, screen_size.y);
    let mip_level = u32(ceil(log2(max_dimension))) + 1u;
    let clamped_mip = min(mip_level, camera.hiz_mip_count - 1u);
    
    // Sample Hi-Z buffer at selected mip
    let hiz_size_at_mip = vec2<f32>(
        f32(camera.hiz_size.x >> clamped_mip),
        f32(camera.hiz_size.y >> clamped_mip),
    );
    
    // Convert screen rect to Hi-Z coordinates
    let hiz_min = screen_min / vec2<f32>(f32(camera.screen_width), f32(camera.screen_height));
    let hiz_max = screen_max / vec2<f32>(f32(camera.screen_width), f32(camera.screen_height));
    
    // Sample 4 corners of the rect and find maximum depth
    var max_hiz_depth = 0.0;
    
    let sample_positions = array<vec2<f32>, 4>(
        hiz_min,
        vec2<f32>(hiz_max.x, hiz_min.y),
        vec2<f32>(hiz_min.x, hiz_max.y),
        hiz_max,
    );
    
    for (var i = 0u; i < 4u; i++) {
        let depth = textureSampleLevel(hiz_texture, hiz_sampler, sample_positions[i], f32(clamped_mip)).r;
        max_hiz_depth = max(max_hiz_depth, depth);
    }
    
    // Conservative test: if any part of AABB is in front of Hi-Z depth, it's visible
    // We use >= because depth buffer is typically reversed (1.0 = near, 0.0 = far)
    return min_depth >= max_hiz_depth;
}

// Backface culling: Test if meshlet cone is backfacing
fn backface_test(cone_apex: vec3<f32>, cone_axis: vec3<f32>, cone_cutoff: f32) -> bool {
    let view_dir = normalize(cone_apex - camera.position);
    let facing = dot(cone_axis, view_dir);
    return facing >= cone_cutoff; // Front-facing if dot product >= cutoff
}

// Main culling compute shader
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_id = id.x;
    
    // Bounds check
    if (cluster_id >= arrayLength(&meshlets)) {
        return;
    }
    
    atomicAdd(&stats.total_clusters, 1u);
    
    let meshlet = meshlets[cluster_id];
    let min_pos = meshlet.bounds_min;
    let max_pos = meshlet.bounds_max;
    
    // Stage 1: Frustum culling
    if (!frustum_test_aabb(min_pos, max_pos)) {
        atomicAdd(&stats.frustum_culled, 1u);
        return;
    }
    
    // Stage 2: Occlusion culling (if enabled)
    if (camera.enable_occlusion != 0u) {
        if (!hiz_occlusion_test(min_pos, max_pos)) {
            atomicAdd(&stats.occlusion_culled, 1u);
            return;
        }
    }
    
    // Stage 3: Backface culling (if enabled)
    if (camera.enable_backface != 0u) {
        if (!backface_test(meshlet.cone_apex, meshlet.cone_axis, meshlet.cone_cutoff)) {
            atomicAdd(&stats.backface_culled, 1u);
            return;
        }
    }
    
    // Meshlet is visible - add to output list
    let visible_index = atomicAdd(&stats.visible_count, 1u);
    visible_meshlets[visible_index] = cluster_id;
}
