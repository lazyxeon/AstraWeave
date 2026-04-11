// Vegetation interaction stamp compute shader.
//
// Stamps circular footprints from entity positions into an R8 interaction
// render texture.  The texture represents a world-space top-down region
// centered on the camera (extent = 2 × interaction_radius).
//
// Two entry points:
// 1. `decay`: fade all texels toward zero each frame (temporal trail fade).
// 2. `stamp`: write entity footprints into the texture.

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

struct StampParams {
    camera_x:          f32,
    camera_z:          f32,
    interaction_radius: f32,
    tex_size:          f32,  // interaction texture resolution (e.g. 128)
    decay_rate:        f32,  // per-frame decay multiplier (e.g. 0.95)
    entity_count:      u32,
    stamp_radius:      f32,  // world-space footprint radius (e.g. 0.5)
    stamp_intensity:   f32,  // footprint intensity (0..1, e.g. 0.8)
}

// x,z world position of each interacting entity.
struct EntityPos {
    x: f32,
    z: f32,
}

@group(0) @binding(0) var<uniform> params: StampParams;
@group(0) @binding(1) var<storage, read> entities: array<EntityPos>;
@group(0) @binding(2) var interaction_tex: texture_storage_2d<r8unorm, read_write>;

/// Temporal decay pass: multiply every texel by decay_rate.
@compute @workgroup_size(WG_X, WG_Y, 1)
fn decay(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(interaction_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) { return; }

    let coord = vec2<i32>(vec2<u32>(gid.x, gid.y));
    let prev = textureLoad(interaction_tex, coord).r;
    let decayed = prev * params.decay_rate;
    textureStore(interaction_tex, coord, vec4<f32>(decayed, 0.0, 0.0, 1.0));
}

/// Stamp pass: write entity footprints into the interaction texture.
@compute @workgroup_size(WG_X, WG_Y, 1)
fn stamp(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(interaction_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) { return; }

    let coord = vec2<i32>(vec2<u32>(gid.x, gid.y));

    // UV of this texel (0..1).
    let uv = (vec2<f32>(f32(gid.x), f32(gid.y)) + 0.5) / params.tex_size;

    // World XZ position of this texel.
    let world_x = params.camera_x + (uv.x - 0.5) * params.interaction_radius * 2.0;
    let world_z = params.camera_z + (uv.y - 0.5) * params.interaction_radius * 2.0;

    // Accumulate stamps from all entities.
    var intensity = textureLoad(interaction_tex, coord).r;

    for (var i = 0u; i < params.entity_count; i = i + 1u) {
        let ex = entities[i].x;
        let ez = entities[i].z;

        let dx = world_x - ex;
        let dz = world_z - ez;
        let dist2 = dx * dx + dz * dz;
        let r2 = params.stamp_radius * params.stamp_radius;

        if (dist2 < r2) {
            // Smooth falloff within the stamp radius.
            let t = 1.0 - sqrt(dist2 / r2);
            intensity = max(intensity, t * params.stamp_intensity);
        }
    }

    textureStore(interaction_tex, coord, vec4<f32>(intensity, 0.0, 0.0, 1.0));
}
