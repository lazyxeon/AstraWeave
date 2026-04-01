// Point light shadow sampling (omnidirectional shadow maps)
//
// Uses a cube shadow map array where each point light gets a cubemap.
// Sampling direction is computed from the fragment-to-light vector.

struct PointShadowParams {
    // Per-light: position.xyz, radius in .w
    light_pos_radius: vec4<f32>,
    // Shadow map parameters: x=bias, y=pcf_radius, z=near, w=far
    shadow_config: vec4<f32>,
};

// Sample point light shadow map
//
// Arguments:
// - shadow_cube: Cube depth texture array (one cube per light)
// - shadow_samp: Comparison sampler
// - world_pos: Fragment world position
// - light_pos: Light position
// - light_radius: Light attenuation radius
// - bias: Depth comparison bias
// - layer: Cubemap array layer index (which light)
//
// Returns: Shadow factor [0.0 = shadowed, 1.0 = lit]
fn sample_point_shadow(
    shadow_cube: texture_depth_cube_array,
    shadow_samp: sampler_comparison,
    world_pos: vec3<f32>,
    light_pos: vec3<f32>,
    light_radius: f32,
    bias: f32,
    layer: i32,
) -> f32 {
    let frag_to_light = world_pos - light_pos;
    let dist = length(frag_to_light);
    let direction = normalize(frag_to_light);

    // Normalize depth to [0, 1] range using light radius
    let normalized_depth = dist / light_radius;

    // PCF with 4 offset samples for soft edges
    var shadow = 0.0;
    let offsets = array<vec3<f32>, 4>(
        vec3<f32>( 0.02,  0.01, -0.01),
        vec3<f32>(-0.01,  0.02,  0.01),
        vec3<f32>( 0.01, -0.01,  0.02),
        vec3<f32>(-0.02, -0.02, -0.02)
    );

    for (var i = 0; i < 4; i++) {
        let sample_dir = direction + offsets[i];
        shadow += textureSampleCompare(
            shadow_cube,
            shadow_samp,
            sample_dir,
            layer,
            normalized_depth - bias
        );
    }

    return shadow / 4.0;
}

// ============================================================================
// SPOT LIGHT SHADOW SAMPLING
// ============================================================================

struct SpotShadowParams {
    // Spot light VP matrix (perspective projection)
    view_proj: mat4x4<f32>,
    // x=bias, y=pcf_radius, z/w=unused
    config: vec4<f32>,
};

// Sample spot light shadow map (2D perspective projection)
fn sample_spot_shadow(
    shadow_tex: texture_depth_2d_array,
    shadow_samp: sampler_comparison,
    world_pos: vec3<f32>,
    spot_vp: mat4x4<f32>,
    bias: f32,
    pcf_radius: f32,
    layer: i32,
) -> f32 {
    let light_clip = spot_vp * vec4<f32>(world_pos, 1.0);
    let ndc = light_clip.xyz / light_clip.w;

    // NDC to UV
    let uv = ndc.xy * 0.5 + vec2<f32>(0.5, 0.5);
    let depth = ndc.z;

    // Bounds check
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0) {
        return 1.0;
    }

    let dims = vec2<f32>(textureDimensions(shadow_tex).xy);
    let texel = 1.0 / dims;
    let biased_depth = depth - bias;

    // 4-sample PCF
    var shadow = 0.0;
    let offsets = array<vec2<f32>, 4>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5,  0.5)
    );

    for (var i = 0; i < 4; i++) {
        let sample_uv = uv + offsets[i] * texel * pcf_radius;
        shadow += textureSampleCompare(shadow_tex, shadow_samp, sample_uv, layer, biased_depth);
    }

    return shadow / 4.0;
}
