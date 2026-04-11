// ═══════════════════════════════════════════════════════════════════════════════
// Snow Accumulation Compute Shader
// ═══════════════════════════════════════════════════════════════════════════════
//
// Updates a per-chunk R16Float accumulation heightmap each frame.
// Snow accumulates on upward-facing surfaces; melts otherwise.
// The accumulation map is read by the PBR shader to blend snow material.

struct SnowParams {
    // Accumulation rate per second (world units of depth).
    accumulate_rate: f32,
    // Melt rate per second.
    melt_rate: f32,
    // dt (frame delta time).
    dt: f32,
    // Whether snow is actively falling (1.0 = yes, 0.0 = no).
    snow_active: f32,
    // Chunk dimensions (texels).
    width: u32,
    height: u32,
    // Min slope cosine (dot(normal, UP)) for accumulation.
    // Surfaces steeper than this shed snow.
    min_slope_cos: f32,
    // Maximum accumulation depth (caps the heightmap value).
    max_depth: f32,
};

@group(0) @binding(0) var<uniform> params: SnowParams;
// Normal map (R16G16_SNORM or similar) for slope testing.
@group(0) @binding(1) var normal_map: texture_2d<f32>;
// Accumulation heightmap (read-write storage texture, r32float).
@group(0) @binding(2) var accumulation: texture_storage_2d<r32float, read_write>;

@compute @workgroup_size(8, 8)
fn update_snow(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) {
        return;
    }

    let coord = vec2<i32>(i32(x), i32(y));

    // Sample terrain normal (assuming Y-up world space stored in RG).
    let nrm_raw = textureLoad(normal_map, coord, 0);
    // Reconstruct Y-up normal from RG (X,Z stored, Y derived).
    let nx = nrm_raw.r * 2.0 - 1.0;
    let nz = nrm_raw.g * 2.0 - 1.0;
    let ny = sqrt(max(1.0 - nx * nx - nz * nz, 0.0));

    let slope_cos = ny; // dot(normal, vec3(0,1,0))

    // Current accumulation depth.
    let current = textureLoad(accumulation, coord).r;

    var new_depth = current;
    if (params.snow_active > 0.5 && slope_cos >= params.min_slope_cos) {
        // Accumulate — rate modulated by how upward-facing the surface is.
        new_depth = new_depth + params.accumulate_rate * params.dt * slope_cos;
    } else {
        // Melt (or shed on steep slopes even during snowfall).
        new_depth = new_depth - params.melt_rate * params.dt;
    }

    new_depth = clamp(new_depth, 0.0, params.max_depth);
    textureStore(accumulation, coord, vec4<f32>(new_depth, 0.0, 0.0, 0.0));
}
