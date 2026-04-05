// Tonemap Blit Shader
//
// Renders a fullscreen triangle that samples the HDR scene target (Rgba16Float)
// and applies tonemapping + sRGB gamma encoding to produce LDR output.
//
// Supports multiple tonemappers selectable via uniform:
// 0 = ACES Filmic (Narkowicz 2015)
// 1 = Khronos PBR Neutral (2024)
// 2 = Reinhard

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct TonemapParams {
    mode: u32,       // 0=ACES, 1=PBR Neutral, 2=Reinhard
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

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
@group(0) @binding(2) var<uniform> params: TonemapParams;

// ─── ACES Filmic Tonemapping (Narkowicz 2015 fit) ───────────────────────────
fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// ─── Khronos PBR Neutral Tonemapper (May 2024) ─────────────────────────────
// Reference: https://github.com/KhronosGroup/ToneMapping
// Designed for faithful reproduction of PBR material colors under neutral lighting.
// Linear pass-through below ~0.8, smooth highlight compression above.
fn pbr_neutral_tonemap(color_in: vec3<f32>) -> vec3<f32> {
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;

    var color = color_in;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    color -= vec3<f32>(offset);

    let peak = max(color.r, max(color.g, color.b));
    if peak < start_compression {
        return color;
    }

    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;

    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), g);
}

// ─── Reinhard Tonemapping ───────────────────────────────────────────────────
fn reinhard_tonemap(color: vec3<f32>) -> vec3<f32> {
    return color / (color + vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv).rgb;

    // Select tonemapper based on uniform parameter
    var tonemapped: vec3<f32>;
    switch params.mode {
        case 1u: {
            tonemapped = pbr_neutral_tonemap(hdr_color);
        }
        case 2u: {
            tonemapped = reinhard_tonemap(hdr_color);
        }
        default: {
            tonemapped = aces_tonemap(hdr_color);
        }
    }

    // Since target is Bgra8UnormSrgb format, GPU applies sRGB gamma automatically.
    // We output linear tonemapped values.
    return vec4<f32>(tonemapped, 1.0);
}
