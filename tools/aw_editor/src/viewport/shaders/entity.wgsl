// Entity Shader
//
// Renders entities with instance rendering and per-vertex colors.
// Supports scene lights (directional sun + up to 4 point lights).
// Supports shading modes: 0=Lit, 1=Unlit, 2=Wireframe
// Textured variant samples albedo from group(1) texture.

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    shading_mode: u32,
    // Scene lighting (vec4 packed)
    sun_dir_and_count: vec4<f32>,       // xyz=sun direction, w=point light count
    sun_color_and_intensity: vec4<f32>, // xyz=sun color, w=sun intensity
    ambient_color_and_intensity: vec4<f32>, // xyz=ambient color, w=ambient intensity
    // Point lights (position+range, color+intensity)
    light0_pos_range: vec4<f32>,
    light0_color_intensity: vec4<f32>,
    light1_pos_range: vec4<f32>,
    light1_color_intensity: vec4<f32>,
    light2_pos_range: vec4<f32>,
    light2_color_intensity: vec4<f32>,
    light3_pos_range: vec4<f32>,
    light3_color_intensity: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(8) uv: vec2<f32>,
}

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
    @location(7) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    let world_normal = (model_matrix * vec4<f32>(vertex.normal, 0.0)).xyz;

    var output: VertexOutput;
    // Camera-relative transform: subtract camera_pos to avoid f32 jitter far from origin
    let rel_pos = world_position.xyz - uniforms.camera_pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(rel_pos, 1.0);
    output.world_position = world_position.xyz;
    output.world_normal = normalize(world_normal);
    // Multiply vertex color by instance tint (white tint = pass-through vertex colors)
    output.color = vertex.vertex_color * instance.color;
    output.uv = vertex.uv;
    return output;
}

// Calculate point light contribution with distance attenuation
fn calc_point_light(light_pos_range: vec4<f32>, light_color_intensity: vec4<f32>, world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let light_pos = light_pos_range.xyz;
    let light_range = light_pos_range.w;
    let light_color = light_color_intensity.xyz;
    let light_intensity = light_color_intensity.w;

    let light_vec = light_pos - world_pos;
    let dist = length(light_vec);
    if dist > light_range || dist < 0.001 {
        return vec3<f32>(0.0);
    }
    let light_dir = light_vec / dist;
    let ndotl = max(dot(normal, light_dir), 0.0);
    // Smooth distance attenuation
    let atten = saturate(1.0 - (dist / light_range)) * saturate(1.0 - (dist / light_range));
    return light_color * light_intensity * ndotl * atten;
}

// Full scene lighting calculation
fn calc_lighting(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(uniforms.sun_dir_and_count.xyz);
    let point_count = u32(uniforms.sun_dir_and_count.w);
    let sun_color = uniforms.sun_color_and_intensity.xyz;
    let sun_intensity = uniforms.sun_color_and_intensity.w;
    let ambient_color = uniforms.ambient_color_and_intensity.xyz;
    let ambient_intensity = uniforms.ambient_color_and_intensity.w;

    // Ambient
    var lighting = ambient_color * ambient_intensity;

    // Directional sun light
    let sun_ndotl = max(dot(normal, sun_dir), 0.0);
    lighting += sun_color * sun_intensity * sun_ndotl;

    // Point lights
    if point_count >= 1u {
        lighting += calc_point_light(uniforms.light0_pos_range, uniforms.light0_color_intensity, world_pos, normal);
    }
    if point_count >= 2u {
        lighting += calc_point_light(uniforms.light1_pos_range, uniforms.light1_color_intensity, world_pos, normal);
    }
    if point_count >= 3u {
        lighting += calc_point_light(uniforms.light2_pos_range, uniforms.light2_color_intensity, world_pos, normal);
    }
    if point_count >= 4u {
        lighting += calc_point_light(uniforms.light3_pos_range, uniforms.light3_color_intensity, world_pos, normal);
    }

    return lighting;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if uniforms.shading_mode == 1u {
        // Unlit: flat color, no lighting
        return in.color;
    }
    
    if uniforms.shading_mode == 2u {
        // Wireframe: edge detection via screen-space derivatives of the world normal
        let dn = fwidth(in.world_normal);
        let edge = length(dn);
        let edge_factor = smoothstep(0.1, 0.5, edge);
        let fill_color = vec4<f32>(0.15, 0.15, 0.18, 0.6);
        let edge_color = vec4<f32>(0.9, 0.95, 1.0, 1.0);
        return mix(fill_color, edge_color, edge_factor);
    }
    
    // Lit: scene lighting
    let lighting = calc_lighting(in.world_position, in.world_normal);
    let lit_color = in.color.rgb * lighting;

    return vec4<f32>(lit_color, in.color.a);
}

// Textured fragment shader — samples albedo texture and applies scene lighting
@group(1) @binding(0)
var albedo_texture: texture_2d<f32>;
@group(1) @binding(1)
var albedo_sampler: sampler;

@fragment
fn fs_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(albedo_texture, albedo_sampler, in.uv);

    if uniforms.shading_mode == 1u {
        return tex_color * in.color;
    }

    if uniforms.shading_mode == 2u {
        let dn = fwidth(in.world_normal);
        let edge = length(dn);
        let edge_factor = smoothstep(0.1, 0.5, edge);
        let fill_color = vec4<f32>(0.15, 0.15, 0.18, 0.6);
        let edge_color = vec4<f32>(0.9, 0.95, 1.0, 1.0);
        return mix(fill_color, edge_color, edge_factor);
    }

    let lighting = calc_lighting(in.world_position, in.world_normal);
    let base = tex_color * in.color;
    let lit_color = base.rgb * lighting;

    return vec4<f32>(lit_color, base.a);
}
