// Weighted Blended Order-Independent Transparency (WBOIT)
//
// McGuire & Bavoil (2013): "Weighted Blended Order-Independent Transparency"
//
// Pass 1: Accumulate weighted color + alpha revealage into two render targets.
// Pass 2: Composite accumulated transparency over the opaque scene.

// ─── Pass 1: Accumulation ───

struct AccumInput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color:   vec4<f32>,  // premultiplied RGBA from vertex/material
    @location(1) frag_z:  f32,        // linear depth (view-space Z)
}

struct AccumOutput {
    // Render target 0: weighted premultiplied color accumulation (Rgba16Float)
    @location(0) accum:     vec4<f32>,
    // Render target 1: alpha revealage (R8Unorm or R16Float)
    @location(1) revealage: vec4<f32>,
}

// Weight function: emphasizes fragments closer to the camera.
// From McGuire & Bavoil eq. 10, modified for better depth distribution.
fn oit_weight(z: f32, alpha: f32) -> f32 {
    // Clamp z to avoid extreme weights near the near plane
    let clamped_z = clamp(abs(z), 0.01, 3000.0);
    // Weight: higher for closer fragments and higher alpha
    let w = alpha * max(0.01, min(3000.0, 10.0 / (0.00001 + pow(clamped_z / 5.0, 2.0) + pow(clamped_z / 200.0, 6.0))));
    return clamp(w, 0.001, 300.0);
}

@fragment
fn fs_accumulate(input: AccumInput) -> AccumOutput {
    var out: AccumOutput;

    let alpha = input.color.a;
    if (alpha < 0.001) {
        discard;
    }

    let w = oit_weight(input.frag_z, alpha);

    // Accumulate: premultiplied color * weight, alpha * weight
    out.accum = vec4<f32>(input.color.rgb * alpha * w, alpha * w);
    // Revealage: (1 - alpha) product — stored in red channel
    out.revealage = vec4<f32>(alpha, 0.0, 0.0, 0.0);

    return out;
}

// ─── Pass 2: Composite ───

struct CompositeInput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var accum_tex:     texture_2d<f32>;
@group(0) @binding(1) var revealage_tex: texture_2d<f32>;
@group(0) @binding(2) var comp_sampler:  sampler;

@vertex
fn vs_composite(@builtin(vertex_index) vid: u32) -> CompositeInput {
    // Full-screen triangle
    var out: CompositeInput;
    let x = f32(i32(vid & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vid >> 1u)) * 4.0 - 1.0;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

@fragment
fn fs_composite(input: CompositeInput) -> @location(0) vec4<f32> {
    let accum     = textureSample(accum_tex, comp_sampler, input.uv);
    let revealage = textureSample(revealage_tex, comp_sampler, input.uv).r;

    // No transparent fragments here
    if (accum.a < 0.00001) {
        discard;
    }

    // Average color: divide accumulated color by accumulated alpha
    let avg_color = accum.rgb / max(accum.a, 0.00001);

    // Revealage = prod(1 - alpha_i), i.e., how much background shows through.
    // Coverage = 1 - revealage = how much is covered by transparent surfaces.
    let coverage = 1.0 - clamp(revealage, 0.0, 1.0);

    return vec4<f32>(avg_color, coverage);
}
