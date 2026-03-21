// Skybox Shader
//
// Procedural gradient skybox with horizon blending.
// Also supports equirectangular HDRI texture sampling.
// Renders at infinite distance (depth = 1.0).

struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    ground_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) view_dir: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// HDRI texture + sampler (only bound in HDRI pipeline)
@group(0) @binding(1)
var hdri_texture: texture_2d<f32>;
@group(0) @binding(2)
var hdri_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle (optimized, no vertex buffer needed)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // Bottom-left
        vec2<f32>(1.0, -1.0),  // Bottom-right
        vec2<f32>(-1.0, 1.0),  // Top-left
        vec2<f32>(-1.0, 1.0),  // Top-left
        vec2<f32>(1.0, -1.0),  // Bottom-right
        vec2<f32>(1.0, 1.0),   // Top-right
    );

    let ndc_pos = positions[vertex_index];

    // Unproject to camera-relative space (at far plane)
    let far_point = uniforms.inv_view_proj * vec4<f32>(ndc_pos, 1.0, 1.0);
    let rel_pos = far_point.xyz / far_point.w;

    // View direction from camera (camera is at origin in relative space)
    let view_dir = normalize(rel_pos);

    var output: VertexOutput;
    output.clip_position = vec4<f32>(ndc_pos, 1.0, 1.0); // Render at far plane
    output.view_dir = view_dir;
    return output;
}

// Procedural gradient fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.view_dir);
    let t = dir.y;

    var color: vec4<f32>;
    if (t > 0.0) {
        let sky_t = smoothstep(0.0, 0.5, t);
        color = mix(uniforms.sky_horizon, uniforms.sky_top, sky_t);
    } else {
        let ground_t = smoothstep(-0.2, 0.0, t);
        color = mix(uniforms.ground_color, uniforms.sky_horizon, ground_t);
    }

    return color;
}

// HDRI equirectangular fragment shader
@fragment
fn fs_hdri(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.view_dir);

    // Equirectangular projection: direction → (u, v)
    let u = atan2(dir.z, dir.x) * (0.5 / 3.14159265) + 0.5;
    let v = 0.5 - asin(clamp(dir.y, -1.0, 1.0)) * (1.0 / 3.14159265);

    let color = textureSample(hdri_texture, hdri_sampler, vec2<f32>(u, v));
    return vec4<f32>(color.rgb, 1.0);
}
