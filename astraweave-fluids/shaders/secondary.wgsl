// Secondary (accent) billboard shader.
//
// F.4.2 — weave-impact water accents. The CPU accent producer (binary glue,
// `weave_accent_producer.rs`) packs the weave KIND into `info.y` (0=Part,
// 1=Raise, 2=Freeze); this shader reads it to drive a per-kind tint + HDR glow
// (C3) and a per-kind procedural shape (A1: streak / round / teardrop). No
// texture binding — the stylized character is entirely UV math + the additive
// blend into the HDR target, so glow is free (tint > 1.0 blooms additively).
//
// The per-kind tint/shape constants below ARE the art-directable surface for
// colour and silhouette; the producer owns motion/spawn/lifetime. Edit these
// named constants to retune the look — no pipeline change.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_inv: mat4x4<f32>,
    cam_pos: vec4<f32>,
    light_dir: vec4<f32>,
    time: f32,
    _pad_a: f32,
    _pad_b: f32,
    _pad_c: f32,
    _pad_v0: vec4<f32>,
    _pad_v1: vec4<f32>,
    _pad_v2: vec4<f32>,
    _pad_v3: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view_params: CameraUniform;

struct SecondaryParticle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    info: vec4<f32>, // x: lifetime/age-alpha, y: kind (0=Part,1=Raise,2=Freeze), z: alpha, w: scale
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) kind: f32, // weave kind index, drives the fragment shape
};

// --- Per-kind tint LUT (C3). Values > 1.0 bloom for free via additive blend. ---
const TINT_PART:   vec3<f32> = vec3<f32>(0.85, 0.72, 0.55); // silt / earthy (bed exposed)
const TINT_RAISE:  vec3<f32> = vec3<f32>(0.85, 0.97, 1.25); // clean white-blue lift (glow)
const TINT_FREEZE: vec3<f32> = vec3<f32>(0.70, 1.15, 1.40); // frost-cyan shimmer (glow)

// Base opacity multiplier (tunable) applied on top of the producer's envelope alpha.
const ACCENT_OPACITY: f32 = 0.7;

fn tint_for_kind(kind: i32) -> vec3<f32> {
    switch kind {
        case 1: { return TINT_RAISE; }
        case 2: { return TINT_FREEZE; }
        default: { return TINT_PART; }
    }
}

@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    @location(0) pos: vec4<f32>,
    @location(1) vel: vec4<f32>,
    @location(2) info: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    // Billboard quad (triangle-strip).
    let quad_pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    let uv = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0)
    );

    let scale = info.w;
    let alpha = info.z;
    let kind = i32(round(info.y));

    // Camera-aligned billboard.
    let right = view_params.view_inv[0].xyz;
    let up = view_params.view_inv[1].xyz;
    let billboard_pos = pos.xyz + (quad_pos[v_idx].x * right + quad_pos[v_idx].y * up) * scale;

    out.clip_position = view_params.view_proj * vec4<f32>(billboard_pos, 1.0);
    out.uv = uv[v_idx];
    out.kind = info.y;

    // Per-kind tint + HDR glow (C3); alpha from the producer's lifetime envelope.
    let tint = tint_for_kind(kind);
    out.color = vec4<f32>(tint, alpha * ACCENT_OPACITY);

    return out;
}

// Per-kind procedural shape mask (A1). p is the centred quad coord in [-1,1].
fn shape_mask(kind: i32, p: vec2<f32>) -> f32 {
    switch kind {
        // Part → streak: tall, thin (water shoved outward / down).
        case 0: {
            let r = length(vec2<f32>(p.x * 2.4, p.y * 0.8));
            return 1.0 - smoothstep(0.5, 1.0, r);
        }
        // Freeze → teardrop: rounded base, tapered top (frost shimmer).
        case 2: {
            let taper = 1.0 + 0.55 * p.y; // widen below, narrow above
            let r = length(vec2<f32>(p.x * taper, p.y * 0.9));
            return 1.0 - smoothstep(0.5, 1.0, r);
        }
        // Raise (1) + default → round soft puff (lifting burst).
        default: {
            let r = length(p);
            return 1.0 - smoothstep(0.5, 1.0, r);
        }
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let kind = i32(round(in.kind));
    let p = (in.uv - 0.5) * 2.0;
    let mask = shape_mask(kind, p);
    if (mask <= 0.0) { discard; }
    return vec4<f32>(in.color.rgb, in.color.a * mask);
}
