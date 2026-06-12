// SSFR Shading Pass - Final Fluid Appearance
// Computes normals, refraction, Fresnel, and absorption

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    cam_pos: vec4<f32>,
    light_dir: vec4<f32>,
    time: f32, // Offset 224
    _pad_a: f32, // 228
    _pad_b: f32, // 232
    _pad_c: f32, // 236 -> 240
    _pad_v0: vec4<f32>, // 256
    _pad_v1: vec4<f32>, // 272
    _pad_v2: vec4<f32>, // 288
    _pad_v3: vec4<f32>, // 304
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var smoothed_depth: texture_2d<f32>;
@group(0) @binding(2) var scene_texture: texture_2d<f32>;
@group(0) @binding(3) var skybox_texture: texture_2d<f32>;
@group(0) @binding(4) var default_sampler: sampler;
@group(0) @binding(5) var scene_depth: texture_depth_2d;
@group(0) @binding(6) var nearest_sampler: sampler;

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(443.897, 441.423, 437.195));
    p3 += dot(p3, p3.yzx + 19.19);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn voronoi(p: vec2<f32>) -> f32 {
    let n = floor(p);
    let f = fract(p);
    var res = 8.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = hash22(n + g);
            let r = g + o - f;
            let d = dot(r, r);
            res = min(res, d);
        }
    }
    return sqrt(res);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let u = f32((vertex_index << 1u) & 2u);
    let v = f32(vertex_index & 2u);
    out.uv = vec2<f32>(u, v);
    out.clip_position = vec4<f32>(u * 2.0 - 1.0, 1.0 - v * 2.0, 0.0, 1.0);
    return out;
}

fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let clip_pos = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    let world_pos = camera.inv_view_proj * clip_pos;
    return world_pos.xyz / world_pos.w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(smoothed_depth, nearest_sampler, in.uv).r;
    if (depth >= 1.0) {
        discard;
    }
    
    // Reconstruct world position
    let pos = world_pos_from_depth(in.uv, depth);
    
    // Compute normal from depth gradient
    let size = vec2<f32>(textureDimensions(smoothed_depth));
    let texel_size = 1.0 / size;
    
    let depth_x = textureSample(smoothed_depth, nearest_sampler, in.uv + vec2<f32>(texel_size.x, 0.0)).r;
    let depth_y = textureSample(smoothed_depth, nearest_sampler, in.uv + vec2<f32>(0.0, texel_size.y)).r;
    
    let pos_x = world_pos_from_depth(in.uv + vec2<f32>(texel_size.x, 0.0), depth_x);
    let pos_y = world_pos_from_depth(in.uv + vec2<f32>(0.0, texel_size.y), depth_y);
    
    let normal = normalize(cross(pos_x - pos, pos_y - pos));
    let view_dir = normalize(camera.cam_pos.xyz - pos);
    
    // Fresnel
    let f0 = 0.02; // Water IOR ≈ 1.33
    let fresnel = f0 + (1.0 - f0) * pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);
    
    // Refraction
    let refraction_strength = 0.05;
    let RefractOffset = normal.xy * refraction_strength;
    let scene_color = textureSample(scene_texture, default_sampler, in.uv + RefractOffset).rgb;
    
    // Caustics & Volumetrics
    // nearest_sampler, NOT default_sampler: depth textures may not be
    // paired with a filtering sampler (wgpu rejects the pair at pipeline
    // creation — F.1.1 finding).
    let ground_depth = textureSample(scene_depth, nearest_sampler, in.uv + RefractOffset);
    let ground_pos = world_pos_from_depth(in.uv + RefractOffset, ground_depth);
    
    // Procedural Caustics - Triple layer for richness
    let caustic_scale = 1.5;
    let caustic_speed = 0.2;
    let caustic_uv = ground_pos.xz * caustic_scale + normal.xz * 0.1;
    let caustic1 = voronoi(caustic_uv + camera.light_dir.xz * camera.time * caustic_speed);
    let caustic2 = voronoi(caustic_uv * 1.5 - camera.light_dir.zx * camera.time * caustic_speed * 1.3);
    let caustic3 = voronoi(caustic_uv * 2.2 + vec2<f32>(camera.time * 0.1, -camera.time * 0.15)); // Third layer
    let caustic_pattern = pow(1.0 - min(min(caustic1, caustic2), caustic3), 14.0) * 4.0;
    
    // Absorption depth (distance from fluid surface to ground)
    let thickness = max(pos.y - ground_pos.y, 0.0);
    
    // Depth-based color gradient (shallow=cyan, mid=blue, deep=dark blue)
    let shallow_color = vec3<f32>(0.4, 0.8, 0.9);   // Bright cyan
    let mid_color = vec3<f32>(0.1, 0.4, 0.8);       // Ocean blue
    let deep_color = vec3<f32>(0.02, 0.08, 0.2);    // Dark abyss
    
    let depth_factor = clamp(thickness * 0.3, 0.0, 1.0);
    let water_tint = mix(shallow_color, mix(mid_color, deep_color, depth_factor), depth_factor * 0.7);
    
    // Beer-Lambert law for absorption with enhanced coefficients
    let absorb_coeffs = vec3<f32>(1.8, 0.6, 0.08); 
    let absorption = exp(-absorb_coeffs * thickness);
    
    // Deep water scattering (enhanced Tyndall effect)
    let scatter_color = vec3<f32>(0.02, 0.12, 0.25);
    let scatter_factor = 1.0 - exp(-thickness * 0.25);
    
    // Foam hints near surface (white edge glow)
    let foam_threshold = 0.3;
    let foam_intensity = smoothstep(foam_threshold, 0.0, thickness) * 0.4;
    let foam_color = vec3<f32>(0.9, 0.95, 1.0);
    
    // Light attenuation & Caustics apply to refraction
    let caustic_fade = exp(-thickness * 1.2);
    let final_caustic = caustic_pattern * caustic_fade * max(camera.light_dir.y, 0.0) * 1.5;
    
    let refracted_with_tint = scene_color * absorption + water_tint * (1.0 - absorption) * 0.5;
    let refracted_with_caustic = refracted_with_tint + final_caustic * vec3<f32>(0.9, 0.95, 1.0) * absorption;
    let total_refracted = mix(refracted_with_caustic, scatter_color, scatter_factor) + foam_color * foam_intensity;
    
    // Reflection (Skybox)
    let reflect_dir = reflect(-view_dir, normal);
    // Simple equirectangular skybox sampling
    let PI = 3.14159265359;
    let sky_u = atan2(reflect_dir.z, reflect_dir.x) / (2.0 * PI) + 0.5;
    let sky_v = asin(reflect_dir.y) / PI + 0.5;
    let reflected_color = textureSample(skybox_texture, nearest_sampler, vec2<f32>(sky_u, sky_v)).rgb;
    
    // Enhanced Fresnel with rim lighting
    let rim_factor = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0) * 0.15;
    let final_color = mix(total_refracted, reflected_color, fresnel) + rim_factor * vec3<f32>(0.6, 0.8, 1.0);
    
    return vec4<f32>(final_color, 1.0);
}
