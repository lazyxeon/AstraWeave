// Minimal shaders for shadow demo
// Proves CSM integration works without full PBR complexity

struct Uniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Shadow map texture array (4 cascades)
@group(1) @binding(0)
var shadow_texture: texture_depth_2d_array;

@group(1) @binding(1)
var shadow_sampler: sampler_comparison;

// Cascade data for shadow sampling
struct CascadeData {
    view_proj: mat4x4<f32>,
    split_distances: vec4<f32>,
}

@group(1) @binding(2)
var<uniform> cascades: array<CascadeData, 4>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let world_pos = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    out.world_normal = (uniforms.model * vec4<f32>(in.normal, 0.0)).xyz;
    
    return out;
}

// Sample shadow map with PCF (Percentage Closer Filtering)
fn sample_shadow(world_pos: vec3<f32>, view_z: f32, surface_normal: vec3<f32>) -> f32 {
    // Determine which cascade to use based on view-space depth
    var cascade_index = 0u;
    for (var i = 0u; i < 3u; i = i + 1u) {
        if view_z > cascades[i].split_distances.y {
            cascade_index = i + 1u;
        }
    }
    
    // Transform world position to light space for selected cascade
    let light_space_pos = cascades[cascade_index].view_proj * vec4<f32>(world_pos, 1.0);
    
    // Perspective divide to get NDC coordinates
    let ndc = light_space_pos.xyz / light_space_pos.w;
    
    // Convert NDC (-1 to 1) to UV (0 to 1)
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, ndc.y * 0.5 + 0.5);
    
    // Check if position is within shadow map bounds
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return 1.0; // Outside shadow map = fully lit
    }
    
    // Depth to compare against shadow map (convert from NDC -1..1 to 0..1)
    let current_depth = clamp(ndc.z * 0.5 + 0.5, 0.0, 1.0);
    
    // Add slope-scaled depth bias to combat acne while preserving contact shadows
    let normal = normalize(surface_normal);
    let light_dir = normalize(uniforms.light_dir.xyz);
    let ndotl = max(dot(normal, -light_dir), 0.0);
    let bias = max(0.0015 * (1.0 - ndotl), 0.0008);
    let biased_depth = clamp(current_depth - bias, 0.0, 1.0);
    
    // PCF: Sample 9 times (3×3 grid) for soft shadows
    var shadow_sum = 0.0;
    let texel_size = 1.0 / 2048.0; // CASCADE_RESOLUTION
    
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            let sample_uv = uv + offset;
            
            // textureSampleCompare returns 1.0 if current_depth <= shadow_depth, 0.0 otherwise
            shadow_sum += textureSampleCompare(
                shadow_texture,
                shadow_sampler,
                sample_uv,
                cascade_index,
                biased_depth
            );
        }
    }
    
    return shadow_sum / 9.0; // Average of 9 samples
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lambert lighting
    let light_dir = normalize(uniforms.light_dir.xyz);
    let normal = normalize(in.world_normal);
    let ndotl = max(dot(normal, -light_dir), 0.0);
    
    // Color based on Y position
    var base_color: vec3<f32>;
    if in.world_position.y < 0.5 {
        base_color = vec3<f32>(0.0, 1.0, 0.0); // GREEN ground
    } else {
        base_color = vec3<f32>(0.8, 0.2, 0.2); // Red cube
    }
    
    // Sample shadow map (use cascade 0 for simplicity)
    let view_pos = uniforms.view * vec4<f32>(in.world_position, 1.0);
    let view_depth = max(-view_pos.z, 0.0);
    let shadow_factor = sample_shadow(in.world_position, view_depth, normal);
    var cascade_index = 0u;
    if view_depth > cascades[2].split_distances.y {
        cascade_index = 3u;
    } else if view_depth > cascades[1].split_distances.y {
        cascade_index = 2u;
    } else if view_depth > cascades[0].split_distances.y {
        cascade_index = 1u;
    }
    
    // Apply simple lambert lighting with shadowed diffuse term
    let ambient = 0.2;
    let diffuse = ndotl * 0.8 * shadow_factor;
    var cascade_tint = vec3<f32>(1.0, 1.0, 1.0);
    switch cascade_index {
        case 0u: {
            cascade_tint = vec3<f32>(1.0, 1.0, 1.0);
        }
        case 1u: {
            cascade_tint = vec3<f32>(0.8, 1.0, 0.8);
        }
        case 2u: {
            cascade_tint = vec3<f32>(0.8, 0.8, 1.0);
        }
        default: {
            cascade_tint = vec3<f32>(1.0, 0.8, 0.8);
        }
    }

    let final_color = base_color * (ambient + diffuse) * cascade_tint;

    return vec4<f32>(final_color, 1.0);
}

// ============================================================================
// SHADOW DEPTH SHADER
// Simple depth-only shader for rendering into shadow maps
// ============================================================================

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> shadow_uniforms: ShadowUniforms;

struct ShadowVertexInput {
    @location(0) position: vec3<f32>,
}

struct ShadowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn shadow_vs_main(in: ShadowVertexInput) -> ShadowVertexOutput {
    var out: ShadowVertexOutput;
    
    let world_pos = shadow_uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip_position = shadow_uniforms.light_view_proj * world_pos;
    
    return out;
}

// No fragment shader needed - depth writes automatically handled
