// Tonemap Blit Shader
//
// Renders a fullscreen triangle that samples the HDR scene target (Rgba16Float)
// and applies ACES tonemapping + sRGB gamma encoding to produce LDR output.
//
// Uses the "fullscreen triangle" technique: 3 vertices, no vertex buffer.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a fullscreen triangle from vertex index alone.
    // Vertices: (-1,-1), (3,-1), (-1,3) — covers the entire clip space.
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV: (0,1) at bottom-left, (2,1) at bottom-right, (0,-1) at top
    // wgpu clip space: Y up, texture: Y down → flip V
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var hdr_texture: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;

// ACES Filmic Tonemapping (Narkowicz 2015 fit)
// Input: linear HDR color, Output: tonemapped color in [0,1]
fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Linear to sRGB gamma encoding
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= cutoff);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv).rgb;

    // Apply exposure (EV 0.0 = no change)
    let exposed = hdr_color;

    // ACES tonemapping
    let tonemapped = aces_tonemap(exposed);

    // sRGB gamma (the output surface is Bgra8UnormSrgb so the hardware
    // does sRGB encoding, but ACES output is in linear space)
    // Since target is *Srgb format, GPU applies gamma automatically.
    // We output linear tonemapped values.
    return vec4<f32>(tonemapped, 1.0);
}
