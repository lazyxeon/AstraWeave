// Ocean shader - ported from Godot Water.gdshader
struct Uniforms {
    view_proj: mat4x4<f32>,
    ocean_pos: vec3<f32>,
    time: f32,
    noise_scale: f32,
    height_scale: f32,
    time_scale: f32,
    wave_direction: vec2<f32>,
    wave_direction2: vec2<f32>,
    beers_law: f32,
    depth_offset: f32,
    edge_scale: f32,
    metallic: f32,
    roughness: f32,
    near: f32,
    far: f32,
    _pad0: f32,
    _pad1: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) vertex_distance: f32,
    @location(3) vertex_distance_clamped: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var wave_texture: texture_2d<f32>;
@group(0) @binding(2) var wave_sampler: sampler;
@group(0) @binding(3) var wave_bump_texture: texture_2d<f32>;
@group(0) @binding(4) var wave_bump_sampler: sampler;
@group(0) @binding(5) var normal_texture: texture_2d<f32>;
@group(0) @binding(6) var normal_sampler: sampler;
@group(0) @binding(7) var normal2_texture: texture_2d<f32>;
@group(0) @binding(8) var normal2_sampler: sampler;

fn wave_height(world_pos: vec3<f32>, offset: vec2<f32>, time: f32, mode: i32) -> f32 {
    let time_offset1 = time * uniforms.wave_direction * uniforms.time_scale;
    let time_offset2 = time * uniforms.wave_direction2 * uniforms.time_scale;
    
    let uv_base = (world_pos.xz + offset) / uniforms.noise_scale;
    
    if (mode == 0) {
        // Wave1 only
        let h1 = textureSampleLevel(wave_texture, wave_sampler, uv_base + time_offset1, 0.0).r;
        return h1;
    } else if (mode == 1) {
        // Wave2 only
        let h2 = textureSampleLevel(wave_texture, wave_sampler, uv_base + time_offset2, 0.0).r;
        return h2;
    } else {
        // Mix both waves
        let h1 = textureSampleLevel(wave_texture, wave_sampler, uv_base + time_offset1, 0.0).r;
        let h2 = textureSampleLevel(wave_texture, wave_sampler, uv_base + time_offset2, 0.0).r;
        return mix(h1, h2, 0.5);
    }
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Calculate world position
    let world_pos = input.position;
    
    // Calculate distance from ocean center
    var vertex_distance = distance(world_pos, uniforms.ocean_pos);
    vertex_distance = clamp(vertex_distance, 0.0, 85.0);
    let vertex_distance_clamped = vertex_distance / 85.0;
    
    // Calculate wave heights
    let prev_height_x = wave_height(world_pos, vec2<f32>(-0.3, 0.0), uniforms.time, 2);
    let next_height_x = wave_height(world_pos, vec2<f32>(0.3, 0.0), uniforms.time, 2);
    let prev_height_y = wave_height(world_pos, vec2<f32>(0.0, -0.3), uniforms.time, 2);
    let next_height_y = wave_height(world_pos, vec2<f32>(0.0, 0.3), uniforms.time, 2);
    let height_mix = wave_height(world_pos, vec2<f32>(0.0, 0.0), uniforms.time, 2);
    
    // Apply wave displacement
    var displaced_pos = world_pos;
    displaced_pos.y += height_mix * uniforms.height_scale * (1.0 - vertex_distance_clamped);
    
    // Edge blending for seamless tiling
    let fraction_x = fract(round(input.uv.x * 1000.0) / 10.0);
    let fraction_y = fract(round(input.uv.y * 1000.0) / 10.0);
    
    if (fraction_x != 0.0 && fract(input.uv.y) == 0.0) {
        displaced_pos.y = ((prev_height_x + next_height_x) * uniforms.height_scale * (1.0 - vertex_distance_clamped)) / 2.0;
    }
    if (fraction_y != 0.0 && fract(input.uv.x) == 0.0) {
        displaced_pos.y = ((prev_height_y + next_height_y) * uniforms.height_scale * (1.0 - vertex_distance_clamped)) / 2.0;
    }
    
    output.clip_position = uniforms.view_proj * vec4<f32>(displaced_pos, 1.0);
    output.world_pos = world_pos;
    output.uv = input.uv;
    output.vertex_distance = vertex_distance;
    output.vertex_distance_clamped = vertex_distance_clamped;
    
    return output;
}

fn fresnel(amount: f32, normal: vec3<f32>, view: vec3<f32>) -> f32 {
    return pow(1.0 - clamp(dot(normalize(normal), normalize(view)), 0.0, 1.0), amount);
}

// F.1.3 (H-2): the albedo constants below were authored in sRGB space (this
// shader is a Godot port; Godot color pickers give sRGB values). The wgpu
// fragment output is treated as LINEAR and hardware-encoded into the *Srgb
// swapchain — writing sRGB-authored constants unconverted double-brightens
// them: the owner's "neon sky blue". Convert authored values to linear once.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Time calculations for wave movement
    let time_offset = uniforms.time * uniforms.wave_direction * uniforms.time_scale;
    let time_offset2 = uniforms.time * uniforms.wave_direction2 * uniforms.time_scale;
    
    // Sample normal maps and blend
    let normal1 = textureSample(normal_texture, normal_sampler, input.world_pos.xz / 10.0 + time_offset / 10.0).rgb;
    let normal2 = textureSample(normal2_texture, normal2_sampler, input.world_pos.xz / 10.0 + time_offset2 / 10.0).rgb;
    let normal_blend = mix(normal1, normal2, 0.5);
    
    // Sample wave bump maps and blend
    let wave_bump1 = textureSample(wave_bump_texture, wave_bump_sampler, input.world_pos.xz / uniforms.noise_scale + time_offset).xyz;
    let wave_bump2 = textureSample(wave_bump_texture, wave_bump_sampler, input.world_pos.xz / uniforms.noise_scale + time_offset2).xyz;
    let wave_normal_blend = mix(wave_bump1, wave_bump2, 0.3);
    
    // Calculate Fresnel effect
    let view = normalize(uniforms.ocean_pos - input.world_pos);
    let normal = normalize(mix(wave_normal_blend, normal_blend, 0.5) * 2.0 - 1.0);
    let fresnel_factor = fresnel(5.0, normal, view);
    
    // Base albedo colors (sRGB-authored; converted to linear — F.1.3 H-2)
    let albedo = srgb_to_linear(vec3<f32>(0.0, 0.32, 0.43));
    let albedo2 = srgb_to_linear(vec3<f32>(0.0, 0.47, 0.76));
    let surface_color = mix(albedo, albedo2, fresnel_factor);

    // Depth colors (sRGB-authored; converted to linear — F.1.3 H-2)
    let color_shallow = srgb_to_linear(vec3<f32>(0.0, 0.47, 0.76));
    let color_deep = srgb_to_linear(vec3<f32>(0.11, 0.29, 0.33));
    
    // Simplified depth blending (without depth texture access)
    let depth_blend = clamp(input.vertex_distance_clamped, 0.0, 1.0);
    let depth_color = mix(color_shallow, color_deep, depth_blend);
    
    // Final color with depth and surface
    let final_color = mix(surface_color, depth_color, 0.3);
    
    return vec4<f32>(final_color, 1.0);
}
