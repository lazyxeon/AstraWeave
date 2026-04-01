// Cascaded Shadow Mapping (CSM) Shaders
//
// This file contains:
// 1. Shadow map rendering (vertex shader for depth-only pass)
// 2. Shadow sampling (fragment shader helper for main pass)
// 3. PCF filtering (Percentage Closer Filtering for soft shadows)
//
// Usage:
// - Shadow Pass: Use shadow_vertex_main() to render depth to cascade atlas
// - Main Pass: Use sample_shadow_csm() in fragment shader for shadow factor

// ============================================================================
// BINDINGS
// ============================================================================

// Shadow cascade data (uploaded per frame)
struct ShadowCascade {
    view_proj: mat4x4<f32>,       // Light-space view-projection matrix
    split_distances: vec4<f32>,   // (near, far, unused, unused)
    atlas_transform: vec4<f32>,   // (offset_x, offset_y, scale_x, scale_y)
}

// Cascade buffer (4 cascades)
@group(1) @binding(0)
var<uniform> cascades: array<ShadowCascade, 4>;

// Shadow atlas texture (4096×4096 depth map)
@group(1) @binding(1)
var shadow_atlas: texture_depth_2d;

// Shadow sampler (comparison sampler for PCF)
@group(1) @binding(2)
var shadow_sampler: sampler_comparison;

// Camera uniforms (for cascade selection in main pass)
struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
    light_dir: vec3<f32>,
    _padding2: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

// ============================================================================
// SHADOW MAP RENDERING (Depth-Only Pass)
// ============================================================================

struct ShadowVertexInput {
    @location(0) position: vec3<f32>,
}

struct ShadowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

// Vertex shader for shadow map rendering
//
// Transforms vertices from world space to light-clip space.
// This is run once per cascade (4 render passes total).
//
// Usage: Set push constant or uniform for cascade_index before each pass.
@vertex
fn shadow_vertex_main(
    in: ShadowVertexInput,
    @builtin(instance_index) cascade_index: u32,
) -> ShadowVertexOutput {
    var out: ShadowVertexOutput;
    
    // Transform to light-clip space
    let world_pos = vec4<f32>(in.position, 1.0);
    let cascade_idx = min(cascade_index, 3u); // Clamp to 0-3
    out.clip_position = cascades[cascade_idx].view_proj * world_pos;
    
    return out;
}

// Fragment shader for shadow map rendering
//
// No-op (depth is written automatically).
// We could add alpha testing here if needed for vegetation.
@fragment
fn shadow_fragment_main(in: ShadowVertexOutput) {
    // Depth is written automatically, nothing to do here
}

// ============================================================================
// SHADOW SAMPLING (Main Render Pass)
// ============================================================================

// Select appropriate cascade based on view-space depth
//
// Returns cascade index (0-3) and blend weight for smooth transitions.
fn select_cascade(view_depth: f32) -> u32 {
    // Linear search (branchless on modern GPUs)
    var cascade_index = 0u;
    
    for (var i = 0u; i < 4u; i = i + 1u) {
        let near = cascades[i].split_distances.x;
        let far = cascades[i].split_distances.y;
        
        if (view_depth >= near && view_depth < far) {
            cascade_index = i;
        }
    }
    
    return cascade_index;
}

// Transform world position to shadow atlas UV coordinates
//
// Returns vec3<f32>: (atlas_u, atlas_v, depth_in_light_space)
fn world_to_shadow_uv(world_pos: vec3<f32>, cascade_index: u32) -> vec3<f32> {
    let cascade = cascades[cascade_index];
    
    // Transform to light-clip space
    let light_clip = cascade.view_proj * vec4<f32>(world_pos, 1.0);
    
    // Perspective divide (orthographic, but keeping for consistency)
    var light_ndc = light_clip.xyz / light_clip.w;
    
    // NDC [-1, 1] to UV [0, 1]
    var shadow_uv = light_ndc.xy * 0.5 + 0.5;
    shadow_uv.y = 1.0 - shadow_uv.y; // Flip Y (wgpu convention)
    
    // Apply atlas transform (map to cascade quadrant)
    let atlas_offset = cascade.atlas_transform.xy;
    let atlas_scale = cascade.atlas_transform.zw;
    shadow_uv = shadow_uv * atlas_scale + atlas_offset;
    
    return vec3<f32>(shadow_uv, light_ndc.z);
}

// Sample shadow map with PCF (Percentage Closer Filtering)
//
// Uses 5×5 PCF kernel for soft shadow edges.
// Returns shadow factor [0.0 = fully shadowed, 1.0 = fully lit]
fn sample_shadow_pcf(shadow_uv: vec3<f32>, bias: f32) -> f32 {
    let atlas_size = 4096.0; // ATLAS_RESOLUTION
    let texel_size = 1.0 / atlas_size;
    
    var shadow_factor = 0.0;
    let kernel_size = 2; // 5×5 kernel (2 texels in each direction)
    let total_samples = f32((kernel_size * 2 + 1) * (kernel_size * 2 + 1));
    
    // Apply depth bias to prevent shadow acne
    let biased_depth = shadow_uv.z - bias;
    
    // PCF loop (unrolled on most GPUs)
    for (var x = -kernel_size; x <= kernel_size; x = x + 1) {
        for (var y = -kernel_size; y <= kernel_size; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            let sample_uv = shadow_uv.xy + offset;
            
            // textureSampleCompare returns 1.0 if biased_depth <= shadow_map_depth
            let visibility = textureSampleCompare(
                shadow_atlas,
                shadow_sampler,
                sample_uv,
                biased_depth
            );
            
            shadow_factor += visibility;
        }
    }
    
    return shadow_factor / total_samples;
}

// Main shadow sampling function (call this from fragment shader)
//
// Arguments:
// - world_pos: Fragment world position
// - view_depth: Fragment depth in view space (for cascade selection)
// - normal: Fragment normal (for normal-offset bias)
//
// Returns: Shadow factor [0.0 = shadow, 1.0 = lit]
fn sample_shadow_csm(
    world_pos: vec3<f32>,
    view_depth: f32,
    normal: vec3<f32>,
) -> f32 {
    // Select cascade
    let cascade_index = select_cascade(view_depth);
    
    // Calculate shadow UV
    let shadow_uv = world_to_shadow_uv(world_pos, cascade_index);
    
    // Check if fragment is outside shadow map bounds
    if (shadow_uv.x < 0.0 || shadow_uv.x > 1.0 ||
        shadow_uv.y < 0.0 || shadow_uv.y > 1.0 ||
        shadow_uv.z < 0.0 || shadow_uv.z > 1.0) {
        return 1.0; // Outside shadow map = fully lit
    }
    
    // Calculate adaptive bias (slope-dependent)
    let light_dir = normalize(camera.light_dir);
    let cos_theta = clamp(dot(normal, -light_dir), 0.0, 1.0);
    let base_bias = 0.005; // DEPTH_BIAS constant
    let slope_bias = base_bias * tan(acos(cos_theta));
    let bias = clamp(slope_bias, 0.0, 0.01);
    
    // Sample shadow map with PCF
    return sample_shadow_pcf(shadow_uv, bias);
}

// ============================================================================
// DEBUG VISUALIZATION
// ============================================================================

// Visualize cascade splits (color-coded overlay)
//
// Returns color: RED (cascade 0), GREEN (1), BLUE (2), YELLOW (3)
fn debug_cascade_color(view_depth: f32) -> vec3<f32> {
    let cascade_index = select_cascade(view_depth);
    
    if (cascade_index == 0u) {
        return vec3<f32>(1.0, 0.0, 0.0); // Red (nearest)
    } else if (cascade_index == 1u) {
        return vec3<f32>(0.0, 1.0, 0.0); // Green
    } else if (cascade_index == 2u) {
        return vec3<f32>(0.0, 0.0, 1.0); // Blue
    } else {
        return vec3<f32>(1.0, 1.0, 0.0); // Yellow (farthest)
    }
}

// ============================================================================
// EXAMPLE USAGE IN MAIN FRAGMENT SHADER
// ============================================================================

// Example integration into your main fragment shader:
//
// @fragment
// fn main_fragment(in: FragmentInput) -> @location(0) vec4<f32> {
//     // ... your existing lighting calculations ...
//     
//     // Calculate view depth
//     let view_pos = camera.view * vec4<f32>(in.world_position, 1.0);
//     let view_depth = -view_pos.z;
//     
//     // Sample shadow
//     let shadow_factor = sample_shadow_csm(
//         in.world_position,
//         view_depth,
//         in.normal
//     );
//     
//     // Apply to lighting
//     let final_color = base_color * (ambient + shadow_factor * diffuse);
//     
//     // Optional: Debug cascade visualization
//     // let debug_color = debug_cascade_color(view_depth);
//     // final_color = mix(final_color, debug_color, 0.3);
//     
//     return vec4<f32>(final_color, 1.0);
// }
