// ═══════════════════════════════════════════════════════════════════════════════
// Snow Footprint Stamp Compute Shader
// ═══════════════════════════════════════════════════════════════════════════════
//
// Stamps entity footprints into the snow accumulation map by subtracting
// depth where entities contact the ground. Uses the same world-space
// top-down texture approach as vegetation_interaction.
//
// Each frame:
// 1. Recovery pass: slowly fills footprints back (snow settling).
// 2. Stamp pass: subtracts depth at entity positions.

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

struct FootprintParams {
    camera_x:        f32,
    camera_z:        f32,
    stamp_radius:    f32,  // world-space foot radius (e.g. 0.3)
    stamp_depth:     f32,  // how deep to compress (e.g. 0.15)
    recovery_rate:   f32,  // depth/second fill-back rate
    dt:              f32,
    entity_count:    u32,
    tex_size:        f32,  // footprint texture resolution
    world_extent:    f32,  // world-space extent per axis from camera
    _pad0:           f32,
    _pad1:           f32,
    _pad2:           f32,
}

struct EntityFoot {
    x: f32,
    z: f32,
}

@group(0) @binding(0) var<uniform> params: FootprintParams;
@group(0) @binding(1) var<storage, read> entities: array<EntityFoot>;
// Snow accumulation map (R32Float, read-write storage).
@group(0) @binding(2) var snow_depth: texture_storage_2d<r32float, read_write>;

/// Recovery pass: gradually fill footprints back.
@compute @workgroup_size(WG_X, WG_Y)
fn recover(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(snow_depth);
    if (gid.x >= dims.x || gid.y >= dims.y) { return; }

    let coord = vec2<i32>(vec2<u32>(gid.x, gid.y));
    // Read current footprint depression (stored as negative offset from max).
    // We don't modify the actual accumulation map directly — we use a
    // separate footprint depth map that is subtracted during PBR evaluation.
    let current = textureLoad(snow_depth, coord).r;

    // Slowly recover toward 0 (no depression).
    var new_val = current;
    if (current < 0.0) {
        new_val = min(current + params.recovery_rate * params.dt, 0.0);
    }

    textureStore(snow_depth, coord, vec4<f32>(new_val, 0.0, 0.0, 0.0));
}

/// Stamp pass: subtract depth at entity foot positions.
@compute @workgroup_size(WG_X, WG_Y)
fn stamp_footprint(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(snow_depth);
    if (gid.x >= dims.x || gid.y >= dims.y) { return; }

    let coord = vec2<i32>(vec2<u32>(gid.x, gid.y));

    let uv = (vec2<f32>(f32(gid.x), f32(gid.y)) + 0.5) / params.tex_size;
    let world_x = params.camera_x + (uv.x - 0.5) * params.world_extent * 2.0;
    let world_z = params.camera_z + (uv.y - 0.5) * params.world_extent * 2.0;

    var depth = textureLoad(snow_depth, coord).r;

    for (var i = 0u; i < params.entity_count; i = i + 1u) {
        let ex = entities[i].x;
        let ez = entities[i].z;

        let dx = world_x - ex;
        let dz = world_z - ez;
        let dist2 = dx * dx + dz * dz;
        let r2 = params.stamp_radius * params.stamp_radius;

        if (dist2 < r2) {
            let t = 1.0 - sqrt(dist2 / r2);
            // Subtract depth (footprint compresses snow).
            depth = min(depth, -params.stamp_depth * t);
        }
    }

    textureStore(snow_depth, coord, vec4<f32>(depth, 0.0, 0.0, 0.0));
}
