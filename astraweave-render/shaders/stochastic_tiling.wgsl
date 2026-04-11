// stochastic_tiling.wgsl — Hex-tile stochastic texture sampling
//
// Breaks visible texture repetition by randomly rotating/offsetting samples
// within a hexagonal grid. Based on Heitz & Neyret 2018 "High-Performance
// By-Example Noise using an Histogram-Preserving Blending Operator."
//
// Usage: Replace `textureSample(tex, samp, uv)` with
//        `sample_stochastic(tex, samp, uv, seed)` wherever tiling artifacts
//        are visible (typically large terrain surfaces).

// ============================================================================
// Hex grid helpers
// ============================================================================

// Single hash from 2D coords — produces pseudo-random value in [0,1).
fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

// Hash returning 2D pseudo-random offset.
fn hash22(p: vec2<f32>) -> vec2<f32> {
    let h = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3)),
    );
    return fract(sin(h) * 43758.5453);
}

// Hex grid coordinate decomposition. Returns the three nearest hex centers
// and barycentric-like blend weights.
struct HexSample {
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    uv2: vec2<f32>,
    w0: f32,
    w1: f32,
    w2: f32,
}

fn hex_coords(uv: vec2<f32>) -> HexSample {
    // Transform to hexagonal grid (skewed coordinates)
    let sqrt3 = 1.7320508;
    let inv_sqrt3 = 0.57735026;

    let skewed = vec2<f32>(uv.x + uv.y * inv_sqrt3, uv.y * 2.0 * inv_sqrt3);
    let base = floor(skewed);
    let f = fract(skewed);

    // Determine which simplex triangle we are in
    var v0: vec2<f32>;
    var v1: vec2<f32>;
    var v2: vec2<f32>;

    if (f.x + f.y < 1.0) {
        v0 = base;
        v1 = base + vec2<f32>(1.0, 0.0);
        v2 = base + vec2<f32>(0.0, 1.0);
    } else {
        v0 = base + vec2<f32>(1.0, 1.0);
        v1 = base + vec2<f32>(1.0, 0.0);
        v2 = base + vec2<f32>(0.0, 1.0);
    }

    // Barycentric-like weights from distance to vertices
    let d0 = length(skewed - v0);
    let d1 = length(skewed - v1);
    let d2 = length(skewed - v2);

    // Inverse distance weighting
    let inv0 = 1.0 / max(d0, 0.0001);
    let inv1 = 1.0 / max(d1, 0.0001);
    let inv2 = 1.0 / max(d2, 0.0001);
    let inv_total = inv0 + inv1 + inv2;

    // Random UV offsets per hex cell
    let off0 = hash22(v0);
    let off1 = hash22(v1);
    let off2 = hash22(v2);

    // Random rotation per cell (applied as UV rotation)
    let a0 = hash2(v0 + vec2<f32>(73.0, 19.0)) * 6.2831853;
    let a1 = hash2(v1 + vec2<f32>(73.0, 19.0)) * 6.2831853;
    let a2 = hash2(v2 + vec2<f32>(73.0, 19.0)) * 6.2831853;

    var result: HexSample;
    result.uv0 = rotate_uv(uv, a0) + off0;
    result.uv1 = rotate_uv(uv, a1) + off1;
    result.uv2 = rotate_uv(uv, a2) + off2;
    result.w0 = inv0 / inv_total;
    result.w1 = inv1 / inv_total;
    result.w2 = inv2 / inv_total;

    return result;
}

// Rotate UV coordinates around (0.5, 0.5) center.
fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let centered = uv - vec2<f32>(0.5);
    return vec2<f32>(
        centered.x * c - centered.y * s,
        centered.x * s + centered.y * c,
    ) + vec2<f32>(0.5);
}

// ============================================================================
// Main sampling function
// ============================================================================

// Stochastic hex-tile texture sample. Drop-in replacement for textureSample
// that eliminates visible tiling patterns.
//
// `scale` controls hex cell density — higher values = smaller cells = less
// visible pattern but more texture lookups. Typical range: 0.5 - 2.0.
fn sample_stochastic(
    tex: texture_2d<f32>,
    samp: sampler,
    uv: vec2<f32>,
    scale: f32,
) -> vec4<f32> {
    let hex = hex_coords(uv * scale);

    let s0 = textureSample(tex, samp, hex.uv0);
    let s1 = textureSample(tex, samp, hex.uv1);
    let s2 = textureSample(tex, samp, hex.uv2);

    return s0 * hex.w0 + s1 * hex.w1 + s2 * hex.w2;
}

// Stochastic sample for texture arrays.
fn sample_stochastic_array(
    tex: texture_2d_array<f32>,
    samp: sampler,
    uv: vec2<f32>,
    layer: u32,
    scale: f32,
) -> vec4<f32> {
    let hex = hex_coords(uv * scale);

    let s0 = textureSample(tex, samp, hex.uv0, layer);
    let s1 = textureSample(tex, samp, hex.uv1, layer);
    let s2 = textureSample(tex, samp, hex.uv2, layer);

    return s0 * hex.w0 + s1 * hex.w1 + s2 * hex.w2;
}
