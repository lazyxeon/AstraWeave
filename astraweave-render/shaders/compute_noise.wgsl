// GPU Compute Noise Generation — Perlin/fBM/Ridged/Billow
//
// Generates heightmap textures entirely on the GPU, eliminating the
// CPU→GPU upload bottleneck for terrain noise evaluation.
//
// Dispatch at terrain chunk resolution (e.g. 512×512):
//   dispatch(ceil(width/8), ceil(height/8), 1)
//
// Output: R32Float texture with normalized height values [0, 1].

struct NoiseParams {
    // Grid configuration
    resolution: vec2<f32>,       // Output texture dimensions
    inv_resolution: vec2<f32>,   // 1.0 / resolution

    // Noise parameters
    frequency: f32,              // Base frequency (e.g. 0.01)
    amplitude: f32,              // Base amplitude (e.g. 1.0)
    lacunarity: f32,             // Frequency multiplier per octave (e.g. 2.0)
    persistence: f32,            // Amplitude multiplier per octave (e.g. 0.5)

    octaves: u32,                // Number of fBM octaves (1-16)
    noise_type: u32,             // 0=fBM, 1=Ridged, 2=Billow, 3=DomainWarped
    seed: u32,                   // Random seed for noise
    _pad: u32,

    // World-space offset for tiling chunks
    world_offset: vec2<f32>,
    world_scale: f32,            // Maps pixel coords to world coords
    warp_strength: f32,          // Domain warp strength (type=3 only)
};

@group(0) @binding(0) var<uniform> params: NoiseParams;
@group(0) @binding(1) var output_tex: texture_storage_2d<r32float, write>;

// ─────────────────── Hash functions ───────────────────
// pcg3d: high-quality 3D hash via permuted congruential generator
fn pcg3d(v: vec3<u32>) -> vec3<u32> {
    var p = v * vec3<u32>(1664525u, 1013904223u, 1664525u) + vec3<u32>(1013904223u);
    p.x += p.y * p.z;
    p.y += p.z * p.x;
    p.z += p.x * p.y;
    p = p ^ (p >> vec3<u32>(16u));
    p.x += p.y * p.z;
    p.y += p.z * p.x;
    p.z += p.x * p.y;
    return p;
}

fn hash_to_gradient(hash: vec3<u32>) -> vec3<f32> {
    // Map hash to [-1, 1] gradient vector
    let f = vec3<f32>(hash & vec3<u32>(0xFFFFu)) / 32767.5 - 1.0;
    return normalize(f);
}

// ─────────────────── Perlin noise (3D, seeded) ───────────────────
fn fade(t: vec3<f32>) -> vec3<f32> {
    // Improved Perlin fade: 6t^5 - 15t^4 + 10t^3
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn perlin_3d(p: vec3<f32>, seed: u32) -> f32 {
    let pi = vec3<i32>(floor(p));
    let pf = p - floor(p);
    let f = fade(pf);

    let seed_v = vec3<u32>(seed);

    // 8 corner gradients
    let g000 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x,     pi.y,     pi.z    )) + seed_v));
    let g100 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x + 1, pi.y,     pi.z    )) + seed_v));
    let g010 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x,     pi.y + 1, pi.z    )) + seed_v));
    let g110 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x + 1, pi.y + 1, pi.z    )) + seed_v));
    let g001 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x,     pi.y,     pi.z + 1)) + seed_v));
    let g101 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x + 1, pi.y,     pi.z + 1)) + seed_v));
    let g011 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x,     pi.y + 1, pi.z + 1)) + seed_v));
    let g111 = hash_to_gradient(pcg3d(vec3<u32>(vec3<i32>(pi.x + 1, pi.y + 1, pi.z + 1)) + seed_v));

    // Offset vectors from corners to point
    let d000 = pf - vec3<f32>(0.0, 0.0, 0.0);
    let d100 = pf - vec3<f32>(1.0, 0.0, 0.0);
    let d010 = pf - vec3<f32>(0.0, 1.0, 0.0);
    let d110 = pf - vec3<f32>(1.0, 1.0, 0.0);
    let d001 = pf - vec3<f32>(0.0, 0.0, 1.0);
    let d101 = pf - vec3<f32>(1.0, 0.0, 1.0);
    let d011 = pf - vec3<f32>(0.0, 1.0, 1.0);
    let d111 = pf - vec3<f32>(1.0, 1.0, 1.0);

    // Dot products
    let n000 = dot(g000, d000);
    let n100 = dot(g100, d100);
    let n010 = dot(g010, d010);
    let n110 = dot(g110, d110);
    let n001 = dot(g001, d001);
    let n101 = dot(g101, d101);
    let n011 = dot(g011, d011);
    let n111 = dot(g111, d111);

    // Trilinear interpolation with fade curve
    let x00 = mix(n000, n100, f.x);
    let x10 = mix(n010, n110, f.x);
    let x01 = mix(n001, n101, f.x);
    let x11 = mix(n011, n111, f.x);

    let y0 = mix(x00, x10, f.y);
    let y1 = mix(x01, x11, f.y);

    return mix(y0, y1, f.z);
}

// ─────────────────── fBM (Fractal Brownian Motion) ───────────────────
fn fbm(p: vec3<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    var sum = 0.0;
    var freq = 1.0;
    var amp = 1.0;
    var max_amp = 0.0;
    var pos = p;

    for (var i = 0u; i < octaves; i++) {
        sum += perlin_3d(pos * freq, seed + i) * amp;
        max_amp += amp;
        freq *= lacunarity;
        amp *= persistence;
    }

    return sum / max_amp; // Normalize to [-1, 1]
}

// ─────────────────── Ridged Multi-fractal ───────────────────
fn ridged(p: vec3<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    var sum = 0.0;
    var freq = 1.0;
    var amp = 1.0;
    var max_amp = 0.0;

    for (var i = 0u; i < octaves; i++) {
        let n = perlin_3d(p * freq, seed + i);
        let ridge = 1.0 - abs(n); // fold to create ridges
        sum += ridge * ridge * amp;
        max_amp += amp;
        freq *= lacunarity;
        amp *= persistence;
    }

    return sum / max_amp;
}

// ─────────────────── Billow noise ───────────────────
fn billow(p: vec3<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    var sum = 0.0;
    var freq = 1.0;
    var amp = 1.0;
    var max_amp = 0.0;

    for (var i = 0u; i < octaves; i++) {
        let n = perlin_3d(p * freq, seed + i);
        sum += abs(n) * amp; // absolute value creates billowy clouds
        max_amp += amp;
        freq *= lacunarity;
        amp *= persistence;
    }

    return sum / max_amp;
}

// ─────────────────── Domain-warped fBM ───────────────────
fn domain_warped(p: vec3<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32, warp_strength: f32) -> f32 {
    // Displace coordinates using noise from different seeds
    let warp_x = fbm(p, seed + 100u, 3u, lacunarity, persistence);
    let warp_z = fbm(p, seed + 200u, 3u, lacunarity, persistence);

    let warped_p = p + vec3<f32>(warp_x, 0.0, warp_z) * warp_strength;

    return fbm(warped_p, seed, octaves, lacunarity, persistence);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

// ─────────────────── Main compute entry ───────────────────
@compute @workgroup_size(WG_X, WG_Y, 1)
fn generate_noise(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    // Map pixel to world coordinates
    let world_pos = vec3<f32>(
        (f32(gid.x) * params.inv_resolution.x) * params.world_scale + params.world_offset.x,
        0.0,
        (f32(gid.y) * params.inv_resolution.y) * params.world_scale + params.world_offset.y,
    );

    let p = world_pos * params.frequency;
    var height: f32;

    switch (params.noise_type) {
        case 0u: { // fBM
            height = fbm(p, params.seed, params.octaves, params.lacunarity, params.persistence);
        }
        case 1u: { // Ridged
            height = ridged(p, params.seed, params.octaves, params.lacunarity, params.persistence);
        }
        case 2u: { // Billow
            height = billow(p, params.seed, params.octaves, params.lacunarity, params.persistence);
        }
        case 3u: { // Domain Warped
            height = domain_warped(p, params.seed, params.octaves, params.lacunarity, params.persistence, params.warp_strength);
        }
        default: {
            height = fbm(p, params.seed, params.octaves, params.lacunarity, params.persistence);
        }
    }

    // Remap from [-1, 1] to [0, 1] for storage
    let normalized = clamp(height * params.amplitude * 0.5 + 0.5, 0.0, 1.0);

    textureStore(output_tex, vec2<i32>(gid.xy), vec4<f32>(normalized, 0.0, 0.0, 1.0));
}
