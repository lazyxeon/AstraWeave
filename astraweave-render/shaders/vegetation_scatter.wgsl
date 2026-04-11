// GPU Vegetation Scatter + Frustum Cull Compute Shader
//
// Two-pass pipeline:
//   Pass 1 (scatter): Generate vegetation instances from heightmap + biome data
//     using GPU Poisson-disk–style placement with hash-based deterministic rejection.
//   Pass 2 (cull):    Frustum-cull generated instances and write survivors into a
//     compacted draw-indirect buffer.
//
// Instance layout (32 bytes, std430):
//   vec4(pos.xyz, scale)
//   vec4(rotation, type_index, normal.xy)  -- normal.z reconstructed
//
// DrawIndexedIndirectCommand (20 bytes):
//   index_count, instance_count, first_index, base_vertex, first_instance

// ── Bindings ────────────────────────────────────────────────────────────────

struct ScatterParams {
    // Chunk world-space origin (x, z) and size
    chunk_origin_x: f32,
    chunk_origin_z: f32,
    chunk_size: f32,
    // Heightmap resolution (e.g. 129)
    heightmap_res: u32,
    // Scatter grid dimensions and spacing
    grid_dim: u32,           // e.g. 64 → 64×64 candidate cells
    min_distance: f32,       // Poisson minimum spacing
    max_slope: f32,          // degrees
    // Seed for deterministic hashing
    seed: u32,
    // Density multiplier (0.0-1.0)
    density: f32,
    // Altitude ceiling (world Y)
    altitude_ceiling: f32,
    // Number of vegetation types
    num_types: u32,
    // Max instances to emit
    max_instances: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

struct VegetationTypeInfo {
    // Scale range (min, max)
    scale_min: f32,
    scale_max: f32,
    // Slope tolerance override (degrees, 0 = use global)
    slope_tolerance: f32,
    // Weight for random type selection
    weight: f32,
}

struct FrustumPlanes {
    planes: array<vec4<f32>, 6>,
}

struct DrawIndexedIndirectCommand {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

struct VegetationInstance {
    // pos.xyz + scale
    pos_scale: vec4<f32>,
    // rotation (radians), type_index (as f32), normal.x, normal.y
    rot_type_normal: vec4<f32>,
}

// ── Pass 1: Scatter ─────────────────────────────────────────────────────────

@group(0) @binding(0) var<uniform> params: ScatterParams;
@group(0) @binding(1) var heightmap_tex: texture_2d<f32>;
@group(0) @binding(2) var heightmap_sampler: sampler;
@group(0) @binding(3) var<storage, read> veg_types: array<VegetationTypeInfo>;
@group(0) @binding(4) var<storage, read_write> instances: array<VegetationInstance>;
@group(0) @binding(5) var<storage, read_write> instance_count: atomic<u32>;

// PCG hash for deterministic pseudo-random numbers
fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash_to_float(h: u32) -> f32 {
    return f32(h) / 4294967295.0;
}

// Sample heightmap at normalized UV [0,1]
fn sample_height(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(heightmap_tex, heightmap_sampler, uv, 0.0).r;
}

// Estimate slope from heightmap central differences
fn estimate_slope(uv: vec2<f32>, texel: f32, chunk_sz: f32) -> vec3<f32> {
    let step_u = vec2<f32>(texel, 0.0);
    let step_v = vec2<f32>(0.0, texel);

    let h_l = sample_height(uv - step_u);
    let h_r = sample_height(uv + step_u);
    let h_d = sample_height(uv - step_v);
    let h_u = sample_height(uv + step_v);

    // World-space derivatives (texel covers chunk_size / resolution)
    let world_step = chunk_sz * texel;
    let dx = (h_r - h_l) / (2.0 * world_step);
    let dz = (h_u - h_d) / (2.0 * world_step);

    // Surface normal from gradient
    return normalize(vec3<f32>(-dx, 1.0, -dz));
}

override WG_SIZE: u32 = 64u;

@compute @workgroup_size(WG_SIZE)
fn scatter_vegetation(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell_index = gid.x;
    let total_cells = params.grid_dim * params.grid_dim;

    if (cell_index >= total_cells) {
        return;
    }

    // 2D cell coordinates
    let cell_x = cell_index % params.grid_dim;
    let cell_z = cell_index / params.grid_dim;

    // Deterministic hash for this cell
    let cell_seed = pcg_hash(cell_index ^ params.seed);

    // Density rejection: skip cells probabilistically
    let density_roll = hash_to_float(pcg_hash(cell_seed + 1u));
    if (density_roll > params.density) {
        return;
    }

    // Jittered position within cell
    let cell_size = params.chunk_size / f32(params.grid_dim);
    let jitter_x = hash_to_float(pcg_hash(cell_seed + 2u));
    let jitter_z = hash_to_float(pcg_hash(cell_seed + 3u));

    let local_x = (f32(cell_x) + jitter_x) * cell_size;
    let local_z = (f32(cell_z) + jitter_z) * cell_size;

    // UV in heightmap space
    let uv = vec2<f32>(local_x / params.chunk_size, local_z / params.chunk_size);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return;
    }

    let height = sample_height(uv);
    let world_x = params.chunk_origin_x + local_x;
    let world_z = params.chunk_origin_z + local_z;

    // Altitude ceiling check
    if (height > params.altitude_ceiling) {
        return;
    }

    // Slope check
    let texel = 1.0 / f32(params.heightmap_res);
    let normal = estimate_slope(uv, texel, params.chunk_size);
    let slope_cos = normal.y; // cos(angle) = dot(normal, up)
    let max_slope_cos = cos(radians(params.max_slope));
    if (slope_cos < max_slope_cos) {
        return;
    }

    // Select vegetation type by weighted random
    let type_roll = hash_to_float(pcg_hash(cell_seed + 4u));
    var accum_weight = 0.0;
    var selected_type = 0u;
    let n_types = min(params.num_types, 16u); // cap for safety
    for (var t = 0u; t < n_types; t = t + 1u) {
        accum_weight += veg_types[t].weight;
        if (type_roll < accum_weight) {
            selected_type = t;
            break;
        }
        selected_type = t; // fallback to last type
    }

    // Per-type slope tolerance override
    let type_slope_tol = veg_types[selected_type].slope_tolerance;
    if (type_slope_tol > 0.0) {
        let type_max_cos = cos(radians(type_slope_tol));
        if (slope_cos < type_max_cos) {
            return;
        }
    }

    // Random scale and rotation
    let scale_min = veg_types[selected_type].scale_min;
    let scale_max = veg_types[selected_type].scale_max;
    let scale_t = hash_to_float(pcg_hash(cell_seed + 5u));
    let instance_scale = mix(scale_min, scale_max, scale_t);

    let rotation = hash_to_float(pcg_hash(cell_seed + 6u)) * TWO_PI; // 0..2π

    // Atomically allocate an instance slot
    let slot = atomicAdd(&instance_count, 1u);
    if (slot >= params.max_instances) {
        // Overflowed — decrement and bail
        atomicSub(&instance_count, 1u);
        return;
    }

    // Write instance
    instances[slot].pos_scale = vec4<f32>(world_x, height, world_z, instance_scale);
    instances[slot].rot_type_normal = vec4<f32>(rotation, f32(selected_type), normal.x, normal.y);
}

// ── Pass 2: Frustum Cull ────────────────────────────────────────────────────

@group(0) @binding(0) var<uniform> cull_frustum: FrustumPlanes;
@group(0) @binding(1) var<storage, read> cull_instances: array<VegetationInstance>;
@group(0) @binding(2) var<uniform> cull_instance_count: u32;
@group(0) @binding(3) var<storage, read_write> visible_instances: array<VegetationInstance>;
@group(0) @binding(4) var<storage, read_write> draw_cmd: DrawIndexedIndirectCommand;

fn test_sphere_frustum(center: vec3<f32>, radius: f32) -> bool {
    // Test sphere against all 6 frustum planes
    let p0 = cull_frustum.planes[0];
    if (dot(center, p0.xyz) + p0.w < -radius) { return false; }
    let p1 = cull_frustum.planes[1];
    if (dot(center, p1.xyz) + p1.w < -radius) { return false; }
    let p2 = cull_frustum.planes[2];
    if (dot(center, p2.xyz) + p2.w < -radius) { return false; }
    let p3 = cull_frustum.planes[3];
    if (dot(center, p3.xyz) + p3.w < -radius) { return false; }
    let p4 = cull_frustum.planes[4];
    if (dot(center, p4.xyz) + p4.w < -radius) { return false; }
    let p5 = cull_frustum.planes[5];
    if (dot(center, p5.xyz) + p5.w < -radius) { return false; }
    return true;
}

@compute @workgroup_size(WG_SIZE)
fn cull_vegetation(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= cull_instance_count) {
        return;
    }

    let inst = cull_instances[idx];
    let pos = inst.pos_scale.xyz;
    let scale = inst.pos_scale.w;

    // Conservative bounding sphere: scale × sqrt(3) ≈ 1.732 for unit cube
    let radius = scale * 1.732;

    if (!test_sphere_frustum(pos, radius)) {
        return;
    }

    // Atomically append to visible list
    let visible_idx = atomicAdd(&draw_cmd.instance_count, 1u);
    visible_instances[visible_idx] = inst;
}
