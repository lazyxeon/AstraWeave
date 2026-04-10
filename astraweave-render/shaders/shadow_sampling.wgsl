// Production shadow sampling utilities
//
// Features:
// - PCSS (Percentage-Closer Soft Shadows) for contact-hardening penumbras
// - Poisson disk PCF with per-pixel rotation for noise-free edges
// - Cascade blending (smooth transition between CSM cascades)
// - Normal-offset bias (eliminates acne without peter-panning)
// - Cascade stabilization support (texel-snapping done on CPU)
//
// Designed to be included/called from the main PBR fragment shader.

// ============================================================================
// POISSON DISK SAMPLE POINTS (16 samples, well-distributed)
// ============================================================================

const POISSON_16: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216),
    vec2<f32>( 0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870),
    vec2<f32>( 0.34495938,  0.29387760),
    vec2<f32>(-0.91588581,  0.45771432),
    vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543,  0.27676845),
    vec2<f32>( 0.97484398,  0.75648379),
    vec2<f32>( 0.44323325, -0.97511554),
    vec2<f32>( 0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023),
    vec2<f32>( 0.79197514,  0.19090188),
    vec2<f32>(-0.24188840,  0.99706507),
    vec2<f32>(-0.81409955,  0.91437590),
    vec2<f32>( 0.19984126,  0.78641367),
    vec2<f32>( 0.14383161, -0.14100790)
);

// ============================================================================
// SHADOW PARAMETERS
// ============================================================================

struct ShadowParams {
    // Per-cascade view-projection matrices
    cascade_vp0: mat4x4<f32>,
    cascade_vp1: mat4x4<f32>,
    cascade_vp2: mat4x4<f32>,
    cascade_vp3: mat4x4<f32>,
    // Cascade split distances (view-space depth)
    splits: vec4<f32>,
    // x: PCF radius in texels, y: depth bias, z: normal offset scale, w: PCSS light size
    shadow_config: vec4<f32>,
    // x: cascade blend range, y: PCSS blocker search radius, z: unused, w: unused
    shadow_config2: vec4<f32>,
};

// ============================================================================
// ROTATION MATRIX FOR POISSON DISK
// ============================================================================

// Per-pixel rotation angle from Interleaved Gradient Noise
fn shadow_rotation_angle(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(0.06711056 * pixel.x + 0.00583715 * pixel.y)) * 6.28318530;
}

fn rotate_sample(sample: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(sample.x * c - sample.y * s, sample.x * s + sample.y * c);
}

// ============================================================================
// NORMAL-OFFSET BIAS
// ============================================================================

// Apply normal-offset to the world position before shadow projection.
// This pushes the position along the surface normal, eliminating shadow acne
// without the peter-panning artifacts of depth-only bias.
fn apply_normal_offset(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    light_dir: vec3<f32>,
    normal_offset_scale: f32,
    texel_size: f32,
) -> vec3<f32> {
    let cos_alpha = dot(normal, light_dir);
    // More offset needed at grazing angles
    let offset_scale = clamp(1.0 - cos_alpha, 0.0, 1.0) * normal_offset_scale * texel_size;
    return world_pos + normal * offset_scale;
}

// ============================================================================
// CASCADE SELECTION WITH BLEND WEIGHT
// ============================================================================

struct CascadeResult {
    index: u32,
    blend_weight: f32,     // 0 = fully this cascade, 1 = fully next cascade
    next_index: u32,
};

fn select_cascade_blend(
    view_depth: f32,
    splits: vec4<f32>,
    blend_range: f32,
) -> CascadeResult {
    var result: CascadeResult;
    result.index = 3u;
    result.next_index = 3u;
    result.blend_weight = 0.0;

    if (view_depth < splits.x) {
        result.index = 0u;
        result.next_index = 1u;
        // Blend near the boundary
        let t = (view_depth - (splits.x - blend_range)) / blend_range;
        result.blend_weight = clamp(t, 0.0, 1.0);
    } else if (view_depth < splits.y) {
        result.index = 1u;
        result.next_index = 2u;
        let t = (view_depth - (splits.y - blend_range)) / blend_range;
        result.blend_weight = clamp(t, 0.0, 1.0);
    } else if (view_depth < splits.z) {
        result.index = 2u;
        result.next_index = 3u;
        let t = (view_depth - (splits.z - blend_range)) / blend_range;
        result.blend_weight = clamp(t, 0.0, 1.0);
    } else {
        result.index = 3u;
        result.next_index = 3u;
        result.blend_weight = 0.0;
    }

    return result;
}

// Get cascade VP matrix by index
fn get_cascade_vp(params: ShadowParams, index: u32) -> mat4x4<f32> {
    if (index == 0u) { return params.cascade_vp0; }
    if (index == 1u) { return params.cascade_vp1; }
    if (index == 2u) { return params.cascade_vp2; }
    return params.cascade_vp3;
}

// ============================================================================
// PCSS: BLOCKER SEARCH + PENUMBRA ESTIMATION
// ============================================================================

// Search for average blocker depth in a region around the sample point.
// Uses textureLoad to read actual shadow map depths (comparison-free).
fn pcss_blocker_search(
    shadow_tex_arr: texture_depth_2d_array,
    shadow_samp: sampler_comparison,
    uv: vec2<f32>,
    receiver_depth: f32,
    search_radius: f32,
    texel_size: f32,
    layer: i32,
    rotation: f32,
) -> vec2<f32> {
    // Returns: (average_blocker_depth, num_blockers)
    let dims = vec2<f32>(textureDimensions(shadow_tex_arr).xy);
    var blocker_sum = 0.0;
    var num_blockers = 0.0;

    for (var i = 0u; i < 16u; i++) {
        let offset = rotate_sample(POISSON_16[i], rotation) * search_radius * texel_size;
        let sample_uv = uv + offset;

        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            continue;
        }

        // Read actual depth from shadow map (no comparison sampler needed)
        let texel_coord = vec2<i32>(sample_uv * dims);
        let shadow_depth = textureLoad(shadow_tex_arr, texel_coord, layer, 0);

        // A texel is a blocker if its stored depth is closer to the light
        // than the receiver (shadow_depth < receiver_depth in standard depth)
        if (shadow_depth < receiver_depth) {
            blocker_sum += shadow_depth;
            num_blockers += 1.0;
        }
    }

    return vec2<f32>(blocker_sum, num_blockers);
}

// ============================================================================
// MAIN SHADOW SAMPLING: POISSON PCF WITH OPTIONAL PCSS
// ============================================================================

// Sample shadow with rotated Poisson disk PCF
fn sample_shadow_poisson(
    shadow_tex_arr: texture_depth_2d_array,
    shadow_samp: sampler_comparison,
    uv: vec2<f32>,
    depth: f32,
    bias: f32,
    pcf_radius: f32,
    texel_size: f32,
    layer: i32,
    rotation: f32,
) -> f32 {
    let biased_depth = depth - bias;
    var sum = 0.0;

    // 16-sample rotated Poisson disk
    for (var i = 0u; i < 16u; i++) {
        let offset = rotate_sample(POISSON_16[i], rotation) * pcf_radius * texel_size;
        let sample_uv = uv + offset;
        sum += textureSampleCompare(shadow_tex_arr, shadow_samp, sample_uv, layer, biased_depth);
    }

    return sum / 16.0;
}

// Full shadow sampling with PCSS + cascade blending
fn sample_shadow_production(
    shadow_tex_arr: texture_depth_2d_array,
    shadow_samp: sampler_comparison,
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    light_dir: vec3<f32>,
    view_depth: f32,
    pixel_pos: vec2<f32>,
    params: ShadowParams,
) -> f32 {
    let pcf_radius = params.shadow_config.x;
    let depth_bias = params.shadow_config.y;
    let normal_offset = params.shadow_config.z;
    let pcss_light_size = params.shadow_config.w;
    let blend_range = params.shadow_config2.x;
    let blocker_search_radius = params.shadow_config2.y;

    let dims = vec2<f32>(textureDimensions(shadow_tex_arr).xy);
    let texel_size = 1.0 / dims.x;

    // Per-pixel rotation for Poisson disk
    let rotation = shadow_rotation_angle(pixel_pos);

    // Cascade selection with blend weight
    let cascade = select_cascade_blend(view_depth, params.splits, blend_range);

    // Normal-offset bias
    let offset_pos = apply_normal_offset(world_pos, normal, light_dir, normal_offset, texel_size * dims.x);

    // Project to shadow UV for primary cascade
    let vp = get_cascade_vp(params, cascade.index);
    let lp = vp * vec4<f32>(offset_pos, 1.0);
    let ndc = lp.xyz / lp.w;
    let uv = ndc.xy * 0.5 + vec2<f32>(0.5, 0.5);
    let depth = ndc.z;

    // Bounds check
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 1.0;
    }

    // Slope-based bias
    let cos_theta = max(dot(normal, light_dir), 0.001);
    let slope_bias = depth_bias * (1.0 + (1.0 - cos_theta) * 2.0);
    let final_bias = clamp(slope_bias, depth_bias * 0.5, depth_bias * 5.0);

    // Determine PCF radius (PCSS or fixed)
    var effective_radius = pcf_radius;
    if (pcss_light_size > 0.0) {
        // PCSS: estimate penumbra width from blocker distance
        let blocker_result = pcss_blocker_search(
            shadow_tex_arr, shadow_samp,
            uv, depth, blocker_search_radius, texel_size,
            i32(cascade.index), rotation
        );

        if (blocker_result.y > 0.0) {
            let avg_blocker = blocker_result.x / blocker_result.y;
            let penumbra = pcss_light_size * (depth - avg_blocker) / max(avg_blocker, 0.001);
            effective_radius = clamp(penumbra, 1.0, pcf_radius * 3.0);
        }
    }

    // Primary cascade shadow
    var shadow = sample_shadow_poisson(
        shadow_tex_arr, shadow_samp,
        uv, depth, final_bias, effective_radius, texel_size,
        i32(cascade.index), rotation
    );

    // Cascade blending
    if (cascade.blend_weight > 0.01 && cascade.next_index != cascade.index) {
        let next_vp = get_cascade_vp(params, cascade.next_index);
        let next_lp = next_vp * vec4<f32>(offset_pos, 1.0);
        let next_ndc = next_lp.xyz / next_lp.w;
        let next_uv = next_ndc.xy * 0.5 + vec2<f32>(0.5, 0.5);
        let next_depth = next_ndc.z;

        if (next_uv.x >= 0.0 && next_uv.x <= 1.0 && next_uv.y >= 0.0 && next_uv.y <= 1.0) {
            let next_shadow = sample_shadow_poisson(
                shadow_tex_arr, shadow_samp,
                next_uv, next_depth, final_bias, effective_radius, texel_size,
                i32(cascade.next_index), rotation
            );
            shadow = mix(shadow, next_shadow, cascade.blend_weight);
        }
    }

    return shadow;
}
