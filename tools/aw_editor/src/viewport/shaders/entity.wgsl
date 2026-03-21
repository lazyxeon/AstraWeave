// Entity Shader
//
// Renders entities with instance rendering and per-vertex colors.
// Uses basic directional lighting for 3D perception.
// Supports shading modes: 0=Lit, 1=Unlit, 2=Wireframe
// Textured variant samples albedo from group(1) texture.

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    shading_mode: u32,
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if uniforms.shading_mode == 1u {
        // Unlit: flat color, no lighting
        return in.color;
    }
    
    if uniforms.shading_mode == 2u {
        // Wireframe: edge detection via screen-space derivatives of the world normal
        // At cube edges, normals change abruptly, producing large derivative magnitudes
        let dn = fwidth(in.world_normal);
        let edge = length(dn);
        // Threshold: values > ~0.3 indicate we're near a geometric edge
        let edge_factor = smoothstep(0.1, 0.5, edge);
        // Dark fill with bright white edges  
        let fill_color = vec4<f32>(0.15, 0.15, 0.18, 0.6);
        let edge_color = vec4<f32>(0.9, 0.95, 1.0, 1.0);
        return mix(fill_color, edge_color, edge_factor);
    }
    
    // Lit: directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;
    let lit_color = in.color.rgb * lighting;

    return vec4<f32>(lit_color, in.color.a);
}

// Textured fragment shader — samples albedo texture and applies lighting
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

    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.7;
    let lighting = ambient + diffuse;
    let base = tex_color * in.color;
    let lit_color = base.rgb * lighting;

    return vec4<f32>(lit_color, base.a);
}
