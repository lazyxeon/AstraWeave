// Shadow Depth-Only Shader
//
// Renders geometry from the directional light's perspective into a depth map.
// Used by the shadow map depth pass — no color output, vertex transform only.

struct ShadowUniforms {
    light_vp: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(8) uv: vec2<f32>,
    @location(9) tangent: vec4<f32>,
}

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
    @location(7) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> shadow: ShadowUniforms;

@vertex
fn vs_shadow(vertex: VertexInput, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    return shadow.light_vp * world_position;
}
