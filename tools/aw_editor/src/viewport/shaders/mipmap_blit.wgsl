// GPU mipmap blit shader — fullscreen triangle sampling previous mip level
// with bilinear filtering. Used by MipmapGenerator to downsample each mip.

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle: 3 vertices covering the entire clip space.
    // vertex 0 → (-1, -1), vertex 1 → (3, -1), vertex 2 → (-1, 3)
    let x = f32(i32(vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(i32((vertex_index >> 1u) & 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(src_texture, src_sampler, in.uv);
}
