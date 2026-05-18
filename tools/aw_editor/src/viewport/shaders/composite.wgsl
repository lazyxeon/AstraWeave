// Editor-Engine Render Parity P.6 composite pass.
//
// Composes the engine canonical LDR output (ENGINE_LDR_TARGET) with the
// editor overlay output (EDITOR_OVERLAY_TARGET) into the display target.
// The engine target is byte-identical to what the runtime would produce —
// the parity contract — and is read but never mutated by this shader.
//
// Alpha convention: PREMULTIPLIED.
//
// The editor overlay pipelines (grid, physics debug, gizmo) all use
// wgpu::BlendState::ALPHA_BLENDING when drawing into the overlay target,
// which on a transparent (a=0) cleared surface produces premultiplied
// alpha output (src.rgb * src.a, src.a). Subsequent overlay draws preserve
// premultiplication via standard alpha-over math accumulating in the
// overlay target.
//
// Composite formula (standard "premultiplied alpha over background"):
//   composed.rgb = overlay.rgb + engine.rgb * (1.0 - overlay.a)
//   composed.a   = 1.0
//
// Where overlays didn't draw (overlay.a == 0 → overlay.rgb == 0), the
// formula reduces to: composed.rgb = engine.rgb. The engine pixels pass
// through unchanged in non-overlay regions.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle: (-1,-1), (3,-1), (-1,3) covers the entire
    // clip space. UVs span [0,1] over the framebuffer.
    var out: VertexOutput;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // wgpu NDC: Y+ is top of screen. Texture V: V=0 is top of texture.
    // Map: (x=-1, y=1) (top-left clip) → (u=0, v=0) (top-left texture).
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var engine_texture:  texture_2d<f32>;
@group(0) @binding(1) var overlay_texture: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let engine  = textureSample(engine_texture,  samp, in.uv);
    let overlay = textureSample(overlay_texture, samp, in.uv);

    // Premultiplied-over composition.
    let composed_rgb = overlay.rgb + engine.rgb * (1.0 - overlay.a);

    return vec4<f32>(composed_rgb, 1.0);
}
