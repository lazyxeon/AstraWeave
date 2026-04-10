// Phase PBR-F: Terrain Layering with Splat Maps and Triplanar Projection
// Multi-layer terrain rendering with height-based blending and seamless slope projection

// ============================================================================
// TERRAIN STRUCTURES (must match Rust TerrainLayerGpu and TerrainMaterialGpu)
// ============================================================================

struct TerrainLayerGpu {
    texture_indices: vec4<u32>,      // [albedo, normal, orm, height]
    uv_scale: vec2<f32>,
    height_range: vec2<f32>,         // [min, max] for height-based blending
    blend_sharpness: f32,
    triplanar_power: f32,
    material_factors: vec2<f32>,     // [metallic, roughness]
    _pad: vec2<f32>,
}

struct TerrainMaterialGpu {
    layers: array<TerrainLayerGpu, 4>,
    splat_map_index: u32,
    splat_uv_scale: f32,
    triplanar_enabled: u32,
    normal_blend_method: u32,        // 0=Linear, 1=RNM, 2=UDN
    triplanar_slope_threshold: f32,  // degrees
    height_blend_enabled: u32,
    _pad: array<u32, 10>,
}

// ============================================================================
// TRIPLANAR PROJECTION
// ============================================================================

/// Calculate triplanar blend weights from world-space normal
/// Returns weights for X, Y, Z planes that sum to 1.0
fn triplanar_weights(world_normal: vec3<f32>, blend_power: f32) -> vec3<f32> {
    // Absolute value of normal components
    var blend = abs(world_normal);
    
    // Apply blend power to sharpen transitions
    blend = pow(blend, vec3(blend_power));
    
    // Normalize so weights sum to 1.0
    let sum = blend.x + blend.y + blend.z;
    return blend / max(sum, 0.0001);
}

/// Sample texture using triplanar projection
fn sample_triplanar(
    texture_array: texture_2d_array<f32>,
    sampler_handle: sampler,
    layer_index: u32,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    uv_scale: f32,
    blend_power: f32
) -> vec4<f32> {
    // Calculate blend weights
    let weights = triplanar_weights(world_normal, blend_power);
    
    // Sample from three orthogonal planes
    let uv_x = world_pos.yz * uv_scale;
    let uv_y = world_pos.xz * uv_scale;
    let uv_z = world_pos.xy * uv_scale;
    
    let sample_x = textureSample(texture_array, sampler_handle, uv_x, layer_index);
    let sample_y = textureSample(texture_array, sampler_handle, uv_y, layer_index);
    let sample_z = textureSample(texture_array, sampler_handle, uv_z, layer_index);
    
    // Blend samples using weights
    return sample_x * weights.x + sample_y * weights.y + sample_z * weights.z;
}

/// Sample normal map using triplanar projection with proper tangent space handling
fn sample_triplanar_normal(
    normal_array: texture_2d_array<f32>,
    sampler_handle: sampler,
    layer_index: u32,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    uv_scale: f32,
    blend_power: f32
) -> vec3<f32> {
    let weights = triplanar_weights(world_normal, blend_power);
    
    // Sample from three planes
    let uv_x = world_pos.yz * uv_scale;
    let uv_y = world_pos.xz * uv_scale;
    let uv_z = world_pos.xy * uv_scale;
    
    var normal_x = textureSample(normal_array, sampler_handle, uv_x, layer_index).xyz * 2.0 - 1.0;
    var normal_y = textureSample(normal_array, sampler_handle, uv_y, layer_index).xyz * 2.0 - 1.0;
    var normal_z = textureSample(normal_array, sampler_handle, uv_z, layer_index).xyz * 2.0 - 1.0;
    
    // Reorient normals to world space (swizzle for plane alignment)
    normal_x = vec3(0.0, normal_x.y, normal_x.x); // YZ plane
    normal_y = vec3(normal_y.x, 0.0, normal_y.y); // XZ plane
    normal_z = vec3(normal_z.x, normal_z.y, 0.0); // XY plane
    
    // Blend and add to world normal
    let blended = normal_x * weights.x + normal_y * weights.y + normal_z * weights.z;
    return normalize(world_normal + blended);
}

/// Calculate triplanar blend factor based on slope angle
/// Returns 0.0 for flat surfaces, 1.0 for steep slopes
fn calculate_triplanar_blend(world_normal: vec3<f32>, slope_threshold: f32) -> f32 {
    // Angle from vertical (dot with up vector)
    let up_dot = abs(dot(world_normal, vec3(0.0, 1.0, 0.0)));
    
    // Convert threshold from degrees to cosine
    let threshold_cos = cos(slope_threshold * 0.01745329); // radians
    
    // Smooth transition around threshold
    return smoothstep(threshold_cos + 0.1, threshold_cos - 0.1, up_dot);
}

// ============================================================================
// NORMAL MAP BLENDING
// ============================================================================

/// Linear normal blending (simple but can lose detail)
fn blend_normals_linear(n1: vec3<f32>, n2: vec3<f32>, weight: f32) -> vec3<f32> {
    return normalize(mix(n1, n2, weight));
}

/// Reoriented Normal Mapping (RNM) - High quality, preserves detail
/// Based on "Blending in Detail" by Colin Barré-Brisebois
fn blend_normals_rnm(base: vec3<f32>, detail: vec3<f32>) -> vec3<f32> {
    let t = base.xyz + vec3(0.0, 0.0, 1.0);
    let u = detail.xyz * vec3(-1.0, -1.0, 1.0);
    return normalize(t * dot(t, u) - u * t.z);
}

/// UDN (Unity Derivative Normals) - Alternative high-quality method
fn blend_normals_udn(n1: vec3<f32>, n2: vec3<f32>) -> vec3<f32> {
    return normalize(vec3(n1.xy + n2.xy, n1.z * n2.z));
}

/// Blend multiple normals with weights (must sum to 1.0)
fn blend_normals_weighted(
    normals: array<vec3<f32>, 4>,
    weights: vec4<f32>,
    method: u32
) -> vec3<f32> {
    if (method == 0u) {
        // Linear blending
        var result = normals[0] * weights.x;
        result += normals[1] * weights.y;
        result += normals[2] * weights.z;
        result += normals[3] * weights.w;
        return normalize(result);
    } else if (method == 1u) {
        // RNM blending (accumulate)
        var result = normals[0];
        if (weights.y > 0.01) {
            result = blend_normals_rnm(result, normals[1]);
        }
        if (weights.z > 0.01) {
            result = blend_normals_rnm(result, normals[2]);
        }
        if (weights.w > 0.01) {
            result = blend_normals_rnm(result, normals[3]);
        }
        return result;
    } else {
        // UDN blending
        var result = normals[0];
        if (weights.y > 0.01) {
            result = blend_normals_udn(result, normals[1]);
        }
        if (weights.z > 0.01) {
            result = blend_normals_udn(result, normals[2]);
        }
        if (weights.w > 0.01) {
            result = blend_normals_udn(result, normals[3]);
        }
        return result;
    }
}

// ============================================================================
// HEIGHT-BASED BLENDING
// ============================================================================

/// Calculate height-based blend weights
/// Uses height maps to create more natural transitions at layer boundaries
fn calculate_height_weights(
    base_weights: vec4<f32>,
    heights: vec4<f32>,
    blend_sharpness: f32
) -> vec4<f32> {
    // Height-adjusted weights (higher areas blend more prominently)
    let adjusted = base_weights * (1.0 + heights * blend_sharpness);
    
    // Normalize
    let sum = adjusted.x + adjusted.y + adjusted.z + adjusted.w;
    return adjusted / max(sum, 0.0001);
}

// ============================================================================
// PARALLAX OCCLUSION MAPPING (TERRAIN — texture array variant)
// ============================================================================

const POM_MIN_STEPS: f32 = 8.0;
const POM_MAX_STEPS: f32 = 32.0;
const POM_BINARY_STEPS: u32 = 5u;

/// Compute parallax-displaced UV for a specific terrain layer using its height texture.
///
/// Parameters:
///   height_array - the terrain height texture array
///   samp         - linear sampler
///   layer_index  - index into the texture array for this layer's heightmap
///   uv           - initial texture coordinates
///   view_ts      - view direction in tangent space
///   height_scale - maximum displacement (0.0 = disabled, typical 0.02–0.06)
///
/// Returns: displaced UV for subsequent texture lookups on this layer.
fn pom_offset_uv_terrain(
    height_array: texture_2d_array<f32>,
    samp: sampler,
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
        height_array, samp, current_uv, layer_index, 0.0
    ).r;

    var i: u32 = 0u;
    let max_i = u32(num_steps);
    while (i < max_i && current_layer_depth < current_height) {
        current_uv -= delta_uv;
        current_layer_depth += step_size;
        current_height = textureSampleLevel(
            height_array, samp, current_uv, layer_index, 0.0
        ).r;
        i++;
    }

    // Binary refinement
    var prev_uv = current_uv + delta_uv;
    var prev_depth = current_layer_depth - step_size;

    for (var j: u32 = 0u; j < POM_BINARY_STEPS; j++) {
        let mid_uv = (current_uv + prev_uv) * 0.5;
        let mid_depth = (current_layer_depth + prev_depth) * 0.5;
        let mid_height = textureSampleLevel(
            height_array, samp, mid_uv, layer_index, 0.0
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

// ============================================================================
// SPLAT MAP SAMPLING
// ============================================================================

/// Sample splat map and normalize weights
fn sample_splat_weights(
    splat_texture: texture_2d<f32>,
    splat_sampler: sampler,
    uv: vec2<f32>
) -> vec4<f32> {
    let splat = textureSample(splat_texture, splat_sampler, uv);
    
    // Normalize weights to sum to 1.0
    let sum = splat.r + splat.g + splat.b + splat.a;
    if (sum > 0.0001) {
        return splat / sum;
    }
    
    // Fallback: all weight to first layer
    return vec4<f32>(1.0, 0.0, 0.0, 0.0);
}

// ============================================================================
// TERRAIN MATERIAL EVALUATION
// ============================================================================

struct TerrainSample {
    albedo: vec3<f32>,
    normal: vec3<f32>,
    metallic: f32,
    roughness: f32,
    occlusion: f32,
}

/// Evaluate terrain material with multi-layer blending
fn evaluate_terrain_material(
    material: TerrainMaterialGpu,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    uv: vec2<f32>,
    albedo_array: texture_2d_array<f32>,
    normal_array: texture_2d_array<f32>,
    orm_array: texture_2d_array<f32>,
    height_array: texture_2d_array<f32>,
    splat_texture: texture_2d<f32>,
    albedo_sampler: sampler,
    linear_sampler: sampler
) -> TerrainSample {
    var result: TerrainSample;
    
    // Sample splat map for layer weights
    let splat_uv = uv * material.splat_uv_scale;
    var layer_weights = sample_splat_weights(splat_texture, linear_sampler, splat_uv);
    
    // Calculate triplanar blend factor if enabled
    let triplanar_blend = select(
        0.0,
        calculate_triplanar_blend(world_normal, material.triplanar_slope_threshold),
        material.triplanar_enabled != 0u
    );
    
    // Sample all 4 layers
    var albedos: array<vec3<f32>, 4>;
    var normals: array<vec3<f32>, 4>;
    var metallics: array<f32, 4>;
    var roughnesses: array<f32, 4>;
    var occlusions: array<f32, 4>;
    var heights: array<f32, 4>;
    
    for (var i = 0u; i < 4u; i = i + 1u) {
        let layer = material.layers[i];
        
        // Choose between standard UV and triplanar based on blend factor
        if (triplanar_blend > 0.01) {
            // Triplanar projection (world-space)
            let tp_uv_scale = layer.uv_scale.x / 10.0; // Scale for world space
            
            albedos[i] = sample_triplanar(
                albedo_array, albedo_sampler,
                layer.texture_indices[0], world_pos, world_normal,
                tp_uv_scale, layer.triplanar_power
            ).rgb;
            
            normals[i] = sample_triplanar_normal(
                normal_array, linear_sampler,
                layer.texture_indices[1], world_pos, world_normal,
                tp_uv_scale, layer.triplanar_power
            );
            
            let orm = sample_triplanar(
                orm_array, linear_sampler,
                layer.texture_indices[2], world_pos, world_normal,
                tp_uv_scale, layer.triplanar_power
            );
            
            occlusions[i] = orm.r;
            roughnesses[i] = orm.g;
            metallics[i] = orm.b;
            
            heights[i] = sample_triplanar(
                height_array, linear_sampler,
                layer.texture_indices[3], world_pos, world_normal,
                tp_uv_scale, layer.triplanar_power
            ).r;
        } else {
            // Standard UV sampling
            let scaled_uv = uv * layer.uv_scale;
            
            albedos[i] = textureSample(
                albedo_array, albedo_sampler,
                scaled_uv, layer.texture_indices[0]
            ).rgb;
            
            let normal_sample = textureSample(
                normal_array, linear_sampler,
                scaled_uv, layer.texture_indices[1]
            ).xyz;
            normals[i] = normalize(normal_sample * 2.0 - 1.0);
            
            let orm = textureSample(
                orm_array, linear_sampler,
                scaled_uv, layer.texture_indices[2]
            );
            
            occlusions[i] = orm.r;
            roughnesses[i] = orm.g;
            metallics[i] = orm.b;
            
            heights[i] = textureSample(
                height_array, linear_sampler,
                scaled_uv, layer.texture_indices[3]
            ).r;
        }
        
        // Apply material factors
        metallics[i] *= layer.material_factors.x;
        roughnesses[i] *= layer.material_factors.y;
    }
    
    // Height-based weight adjustment if enabled
    if (material.height_blend_enabled != 0u) {
        let height_vec = vec4(heights[0], heights[1], heights[2], heights[3]);
        let avg_sharpness = (material.layers[0].blend_sharpness + 
                            material.layers[1].blend_sharpness +
                            material.layers[2].blend_sharpness +
                            material.layers[3].blend_sharpness) * 0.25;
        layer_weights = calculate_height_weights(layer_weights, height_vec, avg_sharpness);
    }
    
    // Blend albedo (linear)
    result.albedo = albedos[0] * layer_weights.x +
                   albedos[1] * layer_weights.y +
                   albedos[2] * layer_weights.z +
                   albedos[3] * layer_weights.w;
    
    // Blend normals (using selected method)
    result.normal = blend_normals_weighted(normals, layer_weights, material.normal_blend_method);
    
    // Blend material properties (linear)
    result.metallic = metallics[0] * layer_weights.x +
                     metallics[1] * layer_weights.y +
                     metallics[2] * layer_weights.z +
                     metallics[3] * layer_weights.w;
    
    result.roughness = roughnesses[0] * layer_weights.x +
                      roughnesses[1] * layer_weights.y +
                      roughnesses[2] * layer_weights.z +
                      roughnesses[3] * layer_weights.w;
    
    result.occlusion = occlusions[0] * layer_weights.x +
                      occlusions[1] * layer_weights.y +
                      occlusions[2] * layer_weights.z +
                      occlusions[3] * layer_weights.w;
    
    return result;
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Get slope angle in degrees from world normal
fn get_slope_angle(world_normal: vec3<f32>) -> f32 {
    let up_dot = abs(dot(world_normal, vec3(0.0, 1.0, 0.0)));
    return acos(up_dot) * 57.29578; // Convert radians to degrees
}

/// Check if a position should use triplanar projection
fn should_use_triplanar(world_normal: vec3<f32>, threshold_degrees: f32) -> bool {
    return get_slope_angle(world_normal) > threshold_degrees;
}

/// Calculate per-layer UV based on world position for seamless tiling
fn world_to_layer_uv(world_pos: vec3<f32>, uv_scale: f32) -> vec2<f32> {
    return world_pos.xz * uv_scale;
}
