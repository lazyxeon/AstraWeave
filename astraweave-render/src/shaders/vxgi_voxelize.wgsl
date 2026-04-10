// VXGI Voxelization Shader - Conservative Rasterization
// Converts voxel terrain meshes (from Marching Cubes) into a 3D radiance field
//
// This shader implements conservative rasterization in 3D space, voxelizing
// triangle meshes into a sparse voxel octree texture for use with VXGI cone tracing.

// ============================================================================
// STRUCTURES
// ============================================================================

struct VoxelizationConfig {
    voxel_resolution: u32,      // 256 (power of 2)
    world_size: f32,            // World space extent covered by voxel grid
    triangle_count: u32,        // Number of triangles to voxelize
    light_intensity: f32,       // Intensity of direct lighting
}

struct Vertex {
    position: vec3<f32>,        // World-space position
    normal: vec3<f32>,          // Normal vector
}

struct Material {
    albedo: vec3<f32>,          // Base color
    metallic: f32,              // Metallic factor
    roughness: f32,             // Roughness factor
    emissive: vec3<f32>,        // Emissive radiance
}

// ============================================================================
// BINDINGS
// ============================================================================

@group(0) @binding(0) var<uniform> config: VoxelizationConfig;
@group(0) @binding(1) var<storage, read> vertices: array<Vertex>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> materials: array<Material>;
@group(0) @binding(4) var voxel_texture: texture_storage_3d<rgba16float, read_write>;

// ---- Atomic accumulation buffer for race-free radiance injection ----
// Each voxel stores 4 atomic<u32> channels: R, G, B, sample_count
// Radiance is accumulated in fixed-point (×1024) and divided in the resolve pass.
@group(0) @binding(5) var<storage, read_write> voxel_accum: array<atomic<u32>>;

// ============================================================================
// COORDINATE CONVERSION
// ============================================================================

/// Convert world-space position to voxel grid coordinates [0, voxel_resolution)
fn world_to_voxel(world_pos: vec3<f32>) -> vec3<f32> {
    // Normalize to [0, 1] then scale to voxel resolution
    let normalized = (world_pos / config.world_size) + 0.5;
    return normalized * f32(config.voxel_resolution);
}

/// Convert voxel grid coordinates to world-space position
fn voxel_to_world(voxel_pos: vec3<f32>) -> vec3<f32> {
    let normalized = voxel_pos / f32(config.voxel_resolution);
    return (normalized - 0.5) * config.world_size;
}

/// Clamp voxel coordinates to valid texture bounds
fn clamp_voxel_coord(coord: vec3<i32>) -> vec3<i32> {
    let max_coord = i32(config.voxel_resolution) - 1;
    return clamp(coord, vec3<i32>(0), vec3<i32>(max_coord));
}

// ============================================================================
// CONSERVATIVE RASTERIZATION
// ============================================================================

struct AABB {
    min: vec3<i32>,
    max: vec3<i32>,
}

/// Calculate AABB (axis-aligned bounding box) for a triangle in voxel space
fn calculate_triangle_aabb(
    v0: vec3<f32>,
    v1: vec3<f32>,
    v2: vec3<f32>
) -> AABB {
    let min_f = min(min(v0, v1), v2);
    let max_f = max(max(v0, v1), v2);
    
    // Expand by 1 voxel in each direction for conservative rasterization
    let min_voxel = vec3<i32>(floor(min_f)) - vec3<i32>(1);
    let max_voxel = vec3<i32>(ceil(max_f)) + vec3<i32>(1);
    
    // Clamp to texture bounds
    let clamped_min = clamp_voxel_coord(min_voxel);
    let clamped_max = clamp_voxel_coord(max_voxel);
    
    return AABB(clamped_min, clamped_max);
}

/// Test if a voxel (AABB) intersects with a triangle
/// Uses Separating Axis Theorem (SAT) for triangle-box intersection
fn voxel_triangle_intersection(
    voxel_center: vec3<f32>,
    voxel_half_size: f32,
    v0: vec3<f32>,
    v1: vec3<f32>,
    v2: vec3<f32>
) -> bool {
    // Translate triangle relative to voxel center
    let v0_rel = v0 - voxel_center;
    let v1_rel = v1 - voxel_center;
    let v2_rel = v2 - voxel_center;
    
    // Compute triangle edges
    let e0 = v1_rel - v0_rel;
    let e1 = v2_rel - v1_rel;
    let e2 = v0_rel - v2_rel;
    
    // Triangle normal
    let normal = normalize(cross(e0, e1));
    
    // ========================================
    // Test 1: Triangle normal axis
    // ========================================
    let d = dot(normal, v0_rel);
    let r = voxel_half_size * (abs(normal.x) + abs(normal.y) + abs(normal.z));
    if abs(d) > r {
        return false;
    }
    
    // ========================================
    // Test 2: Box face normals (AABB axes)
    // ========================================
    
    // X-axis
    let p0_x = v0_rel.x;
    let p1_x = v1_rel.x;
    let p2_x = v2_rel.x;
    let min_x = min(min(p0_x, p1_x), p2_x);
    let max_x = max(max(p0_x, p1_x), p2_x);
    if min_x > voxel_half_size || max_x < -voxel_half_size {
        return false;
    }
    
    // Y-axis
    let p0_y = v0_rel.y;
    let p1_y = v1_rel.y;
    let p2_y = v2_rel.y;
    let min_y = min(min(p0_y, p1_y), p2_y);
    let max_y = max(max(p0_y, p1_y), p2_y);
    if min_y > voxel_half_size || max_y < -voxel_half_size {
        return false;
    }
    
    // Z-axis
    let p0_z = v0_rel.z;
    let p1_z = v1_rel.z;
    let p2_z = v2_rel.z;
    let min_z = min(min(p0_z, p1_z), p2_z);
    let max_z = max(max(p0_z, p1_z), p2_z);
    if min_z > voxel_half_size || max_z < -voxel_half_size {
        return false;
    }
    
    // ========================================
    // Test 3: Cross products of edge and AABB axes (9 tests)
    // ========================================
    
    // Edge 0 cross products
    let fex = abs(e0.x);
    let fey = abs(e0.y);
    let fez = abs(e0.z);
    
    // a00 = e0 × (1,0,0)
    let p0 = e0.z * v0_rel.y - e0.y * v0_rel.z;
    let p2 = e0.z * v2_rel.y - e0.y * v2_rel.z;
    let rad = fez * voxel_half_size + fey * voxel_half_size;
    if min(p0, p2) > rad || max(p0, p2) < -rad {
        return false;
    }
    
    // a01 = e0 × (0,1,0)
    let p0_01 = -e0.z * v0_rel.x + e0.x * v0_rel.z;
    let p2_01 = -e0.z * v2_rel.x + e0.x * v2_rel.z;
    let rad_01 = fez * voxel_half_size + fex * voxel_half_size;
    if min(p0_01, p2_01) > rad_01 || max(p0_01, p2_01) < -rad_01 {
        return false;
    }
    
    // a02 = e0 × (0,0,1)
    let p1_02 = e0.y * v1_rel.x - e0.x * v1_rel.y;
    let p2_02 = e0.y * v2_rel.x - e0.x * v2_rel.y;
    let rad_02 = fey * voxel_half_size + fex * voxel_half_size;
    if min(p1_02, p2_02) > rad_02 || max(p1_02, p2_02) < -rad_02 {
        return false;
    }
    
    // Edge 1 cross products (similar pattern)
    let fex1 = abs(e1.x);
    let fey1 = abs(e1.y);
    let fez1 = abs(e1.z);
    
    let p0_10 = e1.z * v0_rel.y - e1.y * v0_rel.z;
    let p2_10 = e1.z * v2_rel.y - e1.y * v2_rel.z;
    let rad_10 = fez1 * voxel_half_size + fey1 * voxel_half_size;
    if min(p0_10, p2_10) > rad_10 || max(p0_10, p2_10) < -rad_10 {
        return false;
    }
    
    let p0_11 = -e1.z * v0_rel.x + e1.x * v0_rel.z;
    let p2_11 = -e1.z * v2_rel.x + e1.x * v2_rel.z;
    let rad_11 = fez1 * voxel_half_size + fex1 * voxel_half_size;
    if min(p0_11, p2_11) > rad_11 || max(p0_11, p2_11) < -rad_11 {
        return false;
    }
    
    let p0_12 = e1.y * v0_rel.x - e1.x * v0_rel.y;
    let p1_12 = e1.y * v1_rel.x - e1.x * v1_rel.y;
    let rad_12 = fey1 * voxel_half_size + fex1 * voxel_half_size;
    if min(p0_12, p1_12) > rad_12 || max(p0_12, p1_12) < -rad_12 {
        return false;
    }
    
    // Edge 2 cross products
    let fex2 = abs(e2.x);
    let fey2 = abs(e2.y);
    let fez2 = abs(e2.z);
    
    let p0_20 = e2.z * v0_rel.y - e2.y * v0_rel.z;
    let p1_20 = e2.z * v1_rel.y - e2.y * v1_rel.z;
    let rad_20 = fez2 * voxel_half_size + fey2 * voxel_half_size;
    if min(p0_20, p1_20) > rad_20 || max(p0_20, p1_20) < -rad_20 {
        return false;
    }
    
    let p0_21 = -e2.z * v0_rel.x + e2.x * v0_rel.z;
    let p1_21 = -e2.z * v1_rel.x + e2.x * v1_rel.z;
    let rad_21 = fez2 * voxel_half_size + fex2 * voxel_half_size;
    if min(p0_21, p1_21) > rad_21 || max(p0_21, p1_21) < -rad_21 {
        return false;
    }
    
    let p1_22 = e2.y * v1_rel.x - e2.x * v1_rel.y;
    let p2_22 = e2.y * v2_rel.x - e2.x * v2_rel.y;
    let rad_22 = fey2 * voxel_half_size + fex2 * voxel_half_size;
    if min(p1_22, p2_22) > rad_22 || max(p1_22, p2_22) < -rad_22 {
        return false;
    }
    
    // All tests passed - triangle intersects voxel
    return true;
}

// ============================================================================
// RADIANCE INJECTION
// ============================================================================

/// Calculate direct lighting radiance for a voxel
fn calculate_radiance(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    material: Material
) -> vec3<f32> {
    // Simple directional light (sun) from above
    let light_dir = normalize(vec3<f32>(0.3, 1.0, 0.2));
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    
    // Lambertian diffuse BRDF
    let diffuse = material.albedo * n_dot_l * config.light_intensity;
    
    // Add emissive contribution
    let radiance = diffuse + material.emissive;
    
    return radiance;
}

/// Linearize a voxel coordinate to a flat buffer index (4 u32 per voxel: R, G, B, count)
fn voxel_accum_index(coord: vec3<u32>) -> u32 {
    let res = config.voxel_resolution;
    return (coord.x + coord.y * res + coord.z * res * res) * 4u;
}

/// Atomically accumulate radiance into the accumulation buffer.
/// Uses fixed-point arithmetic (×1024) with atomicAdd for race-free injection.
fn inject_radiance(voxel_coord: vec3<i32>, radiance: vec3<f32>, opacity: f32) {
    let u_coord = vec3<u32>(voxel_coord);
    let base_idx = voxel_accum_index(u_coord);

    // Fixed-point encoding: multiply by 1024 and clamp to avoid overflow
    // Max radiance per sample: ~4 million / 1024 = ~4096 HDR units (sufficient)
    let scale = 1024.0;
    let r = u32(clamp(radiance.x * opacity * scale, 0.0, 4194304.0));
    let g = u32(clamp(radiance.y * opacity * scale, 0.0, 4194304.0));
    let b = u32(clamp(radiance.z * opacity * scale, 0.0, 4194304.0));

    atomicAdd(&voxel_accum[base_idx + 0u], r);
    atomicAdd(&voxel_accum[base_idx + 1u], g);
    atomicAdd(&voxel_accum[base_idx + 2u], b);
    atomicAdd(&voxel_accum[base_idx + 3u], 1u);  // sample count
}

// ============================================================================
// COMPUTE SHADER ENTRY POINT
// ============================================================================

@compute @workgroup_size(64, 1, 1)
fn voxelize(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let triangle_index = global_id.x;
    
    // Bounds check
    if triangle_index >= config.triangle_count {
        return;
    }
    
    // Load triangle vertices
    let i0 = indices[triangle_index * 3u + 0u];
    let i1 = indices[triangle_index * 3u + 1u];
    let i2 = indices[triangle_index * 3u + 2u];
    
    let v0_world = vertices[i0].position;
    let v1_world = vertices[i1].position;
    let v2_world = vertices[i2].position;
    
    let n0 = vertices[i0].normal;
    let n1 = vertices[i1].normal;
    let n2 = vertices[i2].normal;
    
    // Convert vertices to voxel space
    let v0_voxel = world_to_voxel(v0_world);
    let v1_voxel = world_to_voxel(v1_world);
    let v2_voxel = world_to_voxel(v2_world);
    
    // Calculate triangle AABB in voxel space
    let aabb = calculate_triangle_aabb(v0_voxel, v1_voxel, v2_voxel);
    let min_voxel = aabb.min;
    let max_voxel = aabb.max;
    
    // Load material (assume one material per mesh for simplicity)
    let material = materials[0];
    
    // Calculate average normal
    let avg_normal = normalize(n0 + n1 + n2);
    
    // Voxel half-size in voxel space (always 0.5)
    let voxel_half_size = 0.5;
    
    // Conservative rasterization: iterate over AABB and test intersection
    for (var x = min_voxel.x; x <= max_voxel.x; x = x + 1) {
        for (var y = min_voxel.y; y <= max_voxel.y; y = y + 1) {
            for (var z = min_voxel.z; z <= max_voxel.z; z = z + 1) {
                let voxel_coord = vec3<i32>(x, y, z);
                let voxel_center = vec3<f32>(voxel_coord) + vec3<f32>(0.5);
                
                // Test triangle-voxel intersection
                if voxel_triangle_intersection(
                    voxel_center,
                    voxel_half_size,
                    v0_voxel,
                    v1_voxel,
                    v2_voxel
                ) {
                    // Calculate radiance at voxel center in world space
                    let world_pos = voxel_to_world(voxel_center);
                    let radiance = calculate_radiance(world_pos, avg_normal, material);
                    
                    // Inject radiance into voxel texture
                    inject_radiance(voxel_coord, radiance, 1.0);
                }
            }
        }
    }
}

// ============================================================================
// CLEAR SHADER (separate pass to reset voxel texture)
// ============================================================================

@compute @workgroup_size(8, 8, 8)
fn clear_voxels(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= config.voxel_resolution ||
       global_id.y >= config.voxel_resolution ||
       global_id.z >= config.voxel_resolution {
        return;
    }
    
    // Clear texture to transparent black
    textureStore(voxel_texture, global_id, vec4<f32>(0.0, 0.0, 0.0, 0.0));

    // Clear accumulation buffer (4 u32 per voxel: R, G, B, count)
    let base_idx = voxel_accum_index(global_id);
    atomicStore(&voxel_accum[base_idx + 0u], 0u);
    atomicStore(&voxel_accum[base_idx + 1u], 0u);
    atomicStore(&voxel_accum[base_idx + 2u], 0u);
    atomicStore(&voxel_accum[base_idx + 3u], 0u);
}

// ============================================================================
// RESOLVE SHADER — converts atomic accumulation to final radiance texture
// ============================================================================

@compute @workgroup_size(8, 8, 8)
fn resolve_voxels(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= config.voxel_resolution ||
       global_id.y >= config.voxel_resolution ||
       global_id.z >= config.voxel_resolution {
        return;
    }

    let base_idx = voxel_accum_index(global_id);
    let count = atomicLoad(&voxel_accum[base_idx + 3u]);

    if count == 0u {
        textureStore(voxel_texture, global_id, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Decode fixed-point: divide by (scale × count) to get average radiance
    let inv = 1.0 / (1024.0 * f32(count));
    let r = f32(atomicLoad(&voxel_accum[base_idx + 0u])) * inv;
    let g = f32(atomicLoad(&voxel_accum[base_idx + 1u])) * inv;
    let b = f32(atomicLoad(&voxel_accum[base_idx + 2u])) * inv;
    let alpha = min(f32(count), 1.0);  // opaque if any triangle contributed

    textureStore(voxel_texture, global_id, vec4<f32>(r, g, b, alpha));
}
