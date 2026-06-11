struct SdfConfig {
    resolution: u32,
    world_size: f32,
    triangle_count: u32,
    // Number of valid entries in dynamic_objects. The buffer is fixed-size
    // (128 entries), so iterating arrayLength() would also visit zeroed
    // entries — whose zero inverse-transform makes sd_box(0,0) = 0, seeding
    // EVERY voxel as "inside an object" (F.1 finding).
    object_count: u32,
};

struct JfaParams {
    step_size: u32,
    // Three scalar pads (NOT vec3<u32>: vec3 has 16-byte alignment in the
    // uniform address space, which would inflate this struct to 32 bytes and
    // mismatch the 16-byte host-side `JfaParams` buffer — F.1 found exactly
    // that mismatch making every FluidSystem::step dispatch fail validation).
    pad0: u32,
    pad1: u32,
    pad2: u32,
};

@group(0) @binding(0) var<uniform> config: SdfConfig;
@group(0) @binding(1) var<storage, read> dynamic_objects: array<DynamicObject>;

struct DynamicObject {
    transform: mat4x4<f32>,
    inv_transform: mat4x4<f32>,
    half_extents: vec4<f32>, // w = type (0=box, 1=sphere)
};

@group(1) @binding(0) var sdf_tex_read: texture_storage_3d<rgba32float, read>;
@group(1) @binding(1) var sdf_tex_write: texture_storage_3d<rgba32float, write>;

// --- Helper Functions ---
fn sd_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sd_sphere(p: vec3<f32>, s: f32) -> f32 {
    return length(p) - s;
}

// --- Passes ---

// Pass 0: Initialization
// Seeds the texture with dynamic object locations. 
// Uses RGBA32Float: RGB = nearest point coordinate, A = signed distance (initial)
@compute @workgroup_size(8, 8, 4)
fn init(@builtin(global_invocation_id) id: vec3<u32>) {
    if (any(id >= vec3<u32>(config.resolution))) { return; }

    let world_pos = (vec3<f32>(id) / f32(config.resolution) - 0.5) * config.world_size;
    
    var min_dist = 1e6;
    var nearest_point = vec3<f32>(1e6);

    // Voxelize dynamic objects into the seed (only the valid prefix of the
    // fixed-size buffer — see SdfConfig.object_count).
    for (var i = 0u; i < config.object_count; i++) {
        let obj = dynamic_objects[i];
        let local_p = (obj.inv_transform * vec4<f32>(world_pos, 1.0)).xyz;
        
        var d = 0.0;
        if (obj.half_extents.w < 0.5) { // Box
            d = sd_box(local_p, obj.half_extents.xyz);
        } else { // Sphere
            d = sd_sphere(local_p, obj.half_extents.x);
        }

        if (d < min_dist) {
            min_dist = d;
            // For JFA, seed is the coordinate of the surface
            // Simple approximation: nearest point on surface
            nearest_point = world_pos; // This pass is just an initial seed
        }
    }

    // A = 0.0 means "is an object", A = 1.0 means "empty"
    var seed_val = vec4<f32>(1e6, 1e6, 1e6, 1e6);
    if (min_dist <= 0.0) {
        seed_val = vec4<f32>(world_pos, 0.0);
    }

    textureStore(sdf_tex_write, id, seed_val);
}

@group(2) @binding(0) var<uniform> jfa_params: JfaParams;

// Pass 1: Jump Flood Step
@compute @workgroup_size(8, 8, 4)
fn jfa_step(@builtin(global_invocation_id) id: vec3<u32>) {
    if (any(id >= vec3<u32>(config.resolution))) { return; }

    let world_pos = (vec3<f32>(id) / f32(config.resolution) - 0.5) * config.world_size;
    var best_seed = textureLoad(sdf_tex_read, id);
    var min_dist_sq = 1e12;
    if (best_seed.w < 1e5) {
        let diff = best_seed.xyz - world_pos;
        min_dist_sq = dot(diff, diff);
    }

    let step = i32(jfa_params.step_size);
    let res = i32(config.resolution);

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec3<i32>(x, y, z) * step;
                let sample_coord = vec3<i32>(id) + offset;

                if (all(sample_coord >= vec3<i32>(0)) && all(sample_coord < vec3<i32>(res))) {
                    let sample_seed = textureLoad(sdf_tex_read, vec3<u32>(sample_coord));
                    if (sample_seed.w < 1e5) {
                        let diff = sample_seed.xyz - world_pos;
                        let d2 = dot(diff, diff);
                        if (d2 < min_dist_sq) {
                            min_dist_sq = d2;
                            best_seed = sample_seed;
                        }
                    }
                }
            }
        }
    }

    textureStore(sdf_tex_write, id, best_seed);
}

// Pass 2: Finalize
@compute @workgroup_size(8, 8, 4)
fn finalize(@builtin(global_invocation_id) id: vec3<u32>) {
    if (any(id >= vec3<u32>(config.resolution))) { return; }

    let world_pos = (vec3<f32>(id) / f32(config.resolution) - 0.5) * config.world_size;
    let best_seed = textureLoad(sdf_tex_read, id);
    
    var dist = config.world_size;
    if (best_seed.w < 1e5) {
        dist = length(best_seed.xyz - world_pos);
    }

    // Check if originally inside for Signed distance
    // We reusePass 0's logic but more cheaply if possible
    // For now, let's just make it Unsigned for stability or use a sign bit from Init
    textureStore(sdf_tex_write, id, vec4<f32>(dist, 0.0, 0.0, 1.0));
}
