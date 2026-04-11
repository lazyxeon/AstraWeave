// ═══════════════════════════════════════════════════════════════════════════════
// Puddle Accumulation Compute Shader
// ═══════════════════════════════════════════════════════════════════════════════
//
// Updates a per-chunk R32Float puddle depth heightmap each frame.
// Water accumulates in terrain concavities (Laplacian > threshold) during rain.
// Drains/evaporates when rain stops.
// The puddle map modifies PBR material: roughness → 0, metallic → 1 where wet.

struct PuddleParams {
    // Accumulation rate per second (world units of depth).
    fill_rate: f32,
    // Drain/evaporation rate per second.
    drain_rate: f32,
    // dt (frame delta time).
    dt: f32,
    // Rain intensity (0.0 = no rain, 1.0 = heavy rain).
    rain_intensity: f32,
    // Chunk dimensions (texels).
    width: u32,
    height: u32,
    // Concavity threshold: Laplacian must exceed this for puddle formation.
    concavity_threshold: f32,
    // Maximum puddle depth (caps the heightmap value).
    max_depth: f32,
}

@group(0) @binding(0) var<uniform> params: PuddleParams;
// Terrain heightmap (for Laplacian concavity detection).
@group(0) @binding(1) var heightmap: texture_2d<f32>;
// Normal map for slope gating (water flows off steep surfaces).
@group(0) @binding(2) var normal_map: texture_2d<f32>;
// Puddle depth map (read-write storage texture, r32float).
@group(0) @binding(3) var puddle_map: texture_storage_2d<r32float, read_write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

/// Compute discrete Laplacian of the heightmap at the given coordinate.
/// Positive Laplacian = concavity (bowl), negative = convexity (hill).
fn laplacian(coord: vec2<i32>, dims: vec2<i32>) -> f32 {
    let c  = textureLoad(heightmap, coord, 0).r;
    let l  = textureLoad(heightmap, clamp(coord + vec2<i32>(-1, 0), vec2<i32>(0), dims - 1), 0).r;
    let r  = textureLoad(heightmap, clamp(coord + vec2<i32>( 1, 0), vec2<i32>(0), dims - 1), 0).r;
    let u  = textureLoad(heightmap, clamp(coord + vec2<i32>( 0,-1), vec2<i32>(0), dims - 1), 0).r;
    let d  = textureLoad(heightmap, clamp(coord + vec2<i32>( 0, 1), vec2<i32>(0), dims - 1), 0).r;

    // Standard 5-point Laplacian.
    return (l + r + u + d - 4.0 * c);
}

@compute @workgroup_size(WG_X, WG_Y)
fn update_puddles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) {
        return;
    }

    let coord = vec2<i32>(i32(x), i32(y));
    let dims = vec2<i32>(i32(params.width), i32(params.height));

    // Slope check: water flows off steep surfaces.
    let nrm_raw = textureLoad(normal_map, coord, 0);
    let nx = nrm_raw.r * 2.0 - 1.0;
    let nz = nrm_raw.g * 2.0 - 1.0;
    let ny = sqrt(max(1.0 - nx * nx - nz * nz, 0.0));
    let slope_cos = ny; // dot(normal, UP)

    // Must be fairly flat for puddle to form (cos > 0.85 ≈ < 32° slope).
    let flat_enough = slope_cos > 0.85;

    // Concavity check: Laplacian detects terrain depressions.
    let lap = laplacian(coord, dims);
    let is_concave = lap > params.concavity_threshold;

    let current = textureLoad(puddle_map, coord).r;
    var new_depth = current;

    if (params.rain_intensity > 0.0 && flat_enough && is_concave) {
        // Fill: rain fills concavities. Rate scales with rain intensity and concavity.
        let concavity_factor = clamp(lap / (params.concavity_threshold * 4.0), 0.0, 1.0);
        new_depth += params.fill_rate * params.dt * params.rain_intensity * concavity_factor;
    }

    // Always drain (evaporation + seepage), even during rain.
    // Drain faster on non-flat or non-concave surfaces.
    var drain_multiplier = 1.0;
    if (!flat_enough) { drain_multiplier = 3.0; }
    if (!is_concave) { drain_multiplier = max(drain_multiplier, 2.0); }
    new_depth -= params.drain_rate * params.dt * drain_multiplier;

    new_depth = clamp(new_depth, 0.0, params.max_depth);
    textureStore(puddle_map, coord, vec4<f32>(new_depth, 0.0, 0.0, 0.0));
}
