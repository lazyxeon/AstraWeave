// Parallax Occlusion Mapping (POM) — Reusable Utility Module
//
// Implements steep parallax mapping with binary refinement for high-quality
// displacement effects without actual tessellation. The step count adapts
// based on view angle: grazing angles use more steps for accuracy.
//
// Usage: Import these functions from your PBR shader. Call pom_offset_uv()
// before sampling albedo, normal, metallic-roughness, etc.
//
// Reference: "Parallax Occlusion Mapping in HLSL" (Tatarchuk, GDC 2006)

// ============================================================================
// CONFIGURATION CONSTANTS
// ============================================================================

const POM_MIN_STEPS: f32 = 8.0;
const POM_MAX_STEPS: f32 = 32.0;
const POM_BINARY_REFINEMENT_STEPS: u32 = 5u;

// ============================================================================
// POM FOR SINGLE TEXTURE (standard PBR materials)
// ============================================================================

/// Compute parallax-displaced UV coordinates using steep parallax + binary refinement.
///
/// Parameters:
///   height_tex  - single-channel (R) heightmap texture (1.0 = raised, 0.0 = depressed)
///   height_samp - sampler for the heightmap
///   uv          - initial texture coordinates
///   view_ts     - view direction in tangent space (T, B, N basis)
///   height_scale - maximum displacement depth in UV space (typical: 0.02–0.08)
///
/// Returns: displaced UV that should be used for all subsequent texture lookups.
fn pom_offset_uv(
    height_tex: texture_2d<f32>,
    height_samp: sampler,
    uv: vec2<f32>,
    view_ts: vec3<f32>,
    height_scale: f32
) -> vec2<f32> {
    if (height_scale <= 0.0) {
        return uv;
    }

    // Adaptive step count: more steps at grazing angles
    let n_dot_v = max(abs(view_ts.z), 0.001);
    let num_steps = mix(POM_MAX_STEPS, POM_MIN_STEPS, n_dot_v);
    let step_size = 1.0 / num_steps;

    // UV offset direction (scale by height and flatten by view angle)
    let max_offset = view_ts.xy / view_ts.z * height_scale;
    let delta_uv = max_offset * step_size;

    // Linear search: march through layers until we intersect the heightfield
    var current_uv = uv;
    var current_layer_depth: f32 = 0.0;
    var current_height = textureSampleLevel(height_tex, height_samp, current_uv, 0.0).r;

    var i: u32 = 0u;
    let max_i = u32(num_steps);
    while (i < max_i && current_layer_depth < current_height) {
        current_uv -= delta_uv;
        current_layer_depth += step_size;
        current_height = textureSampleLevel(height_tex, height_samp, current_uv, 0.0).r;
        i++;
    }

    // Binary refinement: converge on exact intersection point
    var prev_uv = current_uv + delta_uv;
    var prev_depth = current_layer_depth - step_size;
    var prev_height = textureSampleLevel(height_tex, height_samp, prev_uv, 0.0).r;

    for (var j: u32 = 0u; j < POM_BINARY_REFINEMENT_STEPS; j++) {
        let mid_uv = (current_uv + prev_uv) * 0.5;
        let mid_depth = (current_layer_depth + prev_depth) * 0.5;
        let mid_height = textureSampleLevel(height_tex, height_samp, mid_uv, 0.0).r;

        if (mid_depth < mid_height) {
            // Haven't reached surface yet — search deeper
            prev_uv = mid_uv;
            prev_depth = mid_depth;
            prev_height = mid_height;
        } else {
            // Past the surface — search shallower
            current_uv = mid_uv;
            current_layer_depth = mid_depth;
            current_height = mid_height;
        }
    }

    return current_uv;
}

/// Compute self-shadowing from a heightmap (optional — called after POM offset).
///
/// Returns a shadow factor (0.0 = fully shadowed, 1.0 = fully lit).
/// Uses a reduced number of steps since this is an approximation.
fn pom_self_shadow(
    height_tex: texture_2d<f32>,
    height_samp: sampler,
    uv: vec2<f32>,
    light_ts: vec3<f32>,
    height_scale: f32,
    surface_height: f32
) -> f32 {
    if (height_scale <= 0.0 || light_ts.z <= 0.0) {
        return 1.0;
    }

    let num_steps = 8.0;
    let step_size = 1.0 / num_steps;
    let delta_uv = light_ts.xy / light_ts.z * height_scale * step_size;

    var current_uv = uv;
    var current_depth = surface_height;

    for (var i: u32 = 0u; i < u32(num_steps); i++) {
        current_uv += delta_uv;
        current_depth += step_size;
        let h = textureSampleLevel(height_tex, height_samp, current_uv, 0.0).r;
        if (h > current_depth) {
            // Something is above us in the light direction → self-shadow
            return 0.0;
        }
    }

    return 1.0;
}

// ============================================================================
// POM FOR TEXTURE ARRAYS (terrain materials)
// ============================================================================

/// Compute parallax-displaced UV for a specific layer in a texture array.
///
/// Same algorithm as pom_offset_uv but samples from a texture_2d_array.
fn pom_offset_uv_array(
    height_array: texture_2d_array<f32>,
    height_samp: sampler,
    layer_index: u32,
    uv: vec2<f32>,
    view_ts: vec3<f32>,
    height_scale: f32
) -> vec2<f32> {
    if (height_scale <= 0.0) {
        return uv;
    }

    let n_dot_v = max(abs(view_ts.z), 0.001);
    let num_steps = mix(POM_MAX_STEPS, POM_MIN_STEPS, n_dot_v);
    let step_size = 1.0 / num_steps;
    let max_offset = view_ts.xy / view_ts.z * height_scale;
    let delta_uv = max_offset * step_size;

    var current_uv = uv;
    var current_layer_depth: f32 = 0.0;
    var current_height = textureSampleLevel(
        height_array, height_samp, current_uv, layer_index, 0.0
    ).r;

    var i: u32 = 0u;
    let max_i = u32(num_steps);
    while (i < max_i && current_layer_depth < current_height) {
        current_uv -= delta_uv;
        current_layer_depth += step_size;
        current_height = textureSampleLevel(
            height_array, height_samp, current_uv, layer_index, 0.0
        ).r;
        i++;
    }

    // Binary refinement
    var prev_uv = current_uv + delta_uv;
    var prev_depth = current_layer_depth - step_size;

    for (var j: u32 = 0u; j < POM_BINARY_REFINEMENT_STEPS; j++) {
        let mid_uv = (current_uv + prev_uv) * 0.5;
        let mid_depth = (current_layer_depth + prev_depth) * 0.5;
        let mid_height = textureSampleLevel(
            height_array, height_samp, mid_uv, layer_index, 0.0
        ).r;

        if (mid_depth < mid_height) {
            prev_uv = mid_uv;
            prev_depth = mid_depth;
        } else {
            current_uv = mid_uv;
            current_layer_depth = mid_depth;
        }
    }

    return current_uv;
}
