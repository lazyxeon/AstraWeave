# State-of-the-Art Rendering Techniques Reference

**For Rust + wgpu Game Engines (2024–2026)**

| Field | Value |
|-------|-------|
| **Date** | 2026-07-08 |
| **wgpu Version** | 29.0.1 (MSRV 1.87) |
| **Target Hardware** | GTX 1660 Ti class (6 GB VRAM, 192 GB/s bandwidth) |
| **Sources** | 60+ (SIGGRAPH, GPU Gems, wgpu docs, crates.io, GitHub repos, academic papers) |
| **Audience** | Senior graphics programmers building production Rust/wgpu renderers |

---

## Table of Contents

1. [Quick Wins (Top 15 Highest-Impact Practices)](#1-quick-wins)
2. [Common Pitfalls](#2-common-pitfalls)
3. [wgpu API & Platform Status](#3-wgpu-api--platform-status)
4. [Render Graph / Frame Graph Architecture](#4-render-graph--frame-graph-architecture)
5. [GPU-Driven Rendering](#5-gpu-driven-rendering)
6. [Buffer Management & Memory](#6-buffer-management--memory)
7. [Pipeline Caching](#7-pipeline-caching)
8. [Lighting & Shadows](#8-lighting--shadows)
9. [Post-Processing Pipeline](#9-post-processing-pipeline)
10. [Terrain Generation](#10-terrain-generation)
11. [Terrain Rendering](#11-terrain-rendering)
12. [Vegetation & Scatter Systems](#12-vegetation--scatter-systems)
13. [Atmosphere, Weather & Volumetrics](#13-atmosphere-weather--volumetrics)
14. [Particles & VFX](#14-particles--vfx)
15. [WGSL Shader Authoring](#15-wgsl-shader-authoring)
16. [Asset Pipeline](#16-asset-pipeline)
17. [Rust Renderer Patterns](#17-rust-renderer-patterns)
18. [Reference Renderers](#18-reference-renderers)
19. [Vegetation & Scatter Budget Cheat Sheet](#19-vegetation--scatter-budget-cheat-sheet)
20. [Weather System Integration Checklist](#20-weather-system-integration-checklist)
21. [Sources](#21-sources)

---

## 1. Quick Wins

The 15 highest-impact practices for a Rust/wgpu production renderer, ordered by effort-to-reward ratio:

| # | Practice | Impact | Effort |
|---|----------|--------|--------|
| 1 | **Batch command buffers** — never `queue.submit()` per draw call | 10–50× fewer driver calls | Trivial |
| 2 | **Cache pipelines** — create once, reuse every frame; use `PipelineCache` on Vulkan | 20–50% faster startup | Low |
| 3 | **Use `queue.write_buffer()`** instead of staging buffer round-trips | Eliminates alloc per frame | Trivial |
| 4 | **Always benchmark in `--release`** — debug vs release is 10–100× | Prevents false negatives | Trivial |
| 5 | **Render HDR → tonemap last** — `Rgba16Float` targets, ACES/AgX/PBR Neutral | Correct color pipeline | Medium |
| 6 | **Collect-then-upload** — batch ECS data into `Vec`, bulk write | 3–5× over scattered `get_mut()` | Low |
| 7 | **Frustum cull on CPU first** — skip invisible objects before GPU submission | 30–60% fewer draw calls | Medium |
| 8 | **Base PBR on Cook-Torrance GGX** — merge Smith G-term with BRDF denominator | Industry standard + optimized | Medium |
| 9 | **Use `@workgroup_size(64,1,1)` or `(8,8,1)`** — multiples of warp size (32) | Max GPU occupancy | Trivial |
| 10 | **Half-resolution SSAO + bilateral blur** | 4× fewer depth samples | Medium |
| 11 | **Dual Kawase bloom** — 6–8 mip chain, 5-tap down / 9-tap up | Fast, high quality | Medium |
| 12 | **Camera-relative rendering** — DVec3 origin, f32 offsets to shader | Eliminates fp32 jitter beyond 10 km | Medium |
| 13 | **GPU timestamp queries** via `QuerySet` — profile GPU, not just CPU | Find real bottleneck | Low |
| 14 | **`meshopt` for mesh optimization** — vertex cache, overdraw, meshlet generation | 10–30% fewer cache misses | Low |
| 15 | **Compress textures to BC7/ASTC via basis_universal** | 4:1 ratio, near-lossless | Medium |

---

## 2. Common Pitfalls

| Pitfall | Why It Hurts | Fix |
|---------|-------------|-----|
| Creating staging buffers per frame | Allocation + map overhead every frame | Persistent buffers + `queue.write_buffer()` |
| `Buffer::slice().get_mapped_range()` in hot loops | Async readback 100–1000× slower than staying on GPU | Keep data on GPU; use timestamp queries instead of readback for profiling |
| Tonemapping in wrong color space | Washed-out or clipped appearance | Render linear HDR → tonemap → output sRGB |
| Testing perf in debug builds | 10–100× slower than release, wrong conclusions | Always `--release` for perf testing |
| One `queue.submit()` per draw | Massive driver overhead | Batch all draws into one command encoder then submit |
| `f32` world-space beyond 10 km | Vertex jitter, z-fighting | Camera-relative rendering with DVec3 origin |
| Ignoring alignment on texture uploads | `COPY_BYTES_PER_ROW_ALIGNMENT = 256` | Pad row pitch to 256-byte multiple |
| Assuming multi-draw-indirect everywhere | Not in WebGPU spec yet | Fall back to single `draw_indirect()` or direct draws on WebGL2 |
| Deleting buffers mid-frame | GPU still referencing them | Defer deletion until after `queue.submit()` completes |
| Shader hot-reload without cache invalidation | Stale pipeline cache serves old bytecode | Hash shader source → invalidate cache on mismatch |

---

## 3. wgpu API & Platform Status

### Version & Backends

| Property | Value |
|----------|-------|
| **Latest Stable** | 29.0.1 (Feb 2025) |
| **MSRV** | 1.87 |
| **API Model** | Refcounted (Arc-based) |
| **Coordinate System** | D3D (left-handed, Y-up, depth [0, 1]) |
| **Backends** | Vulkan, Metal, DX12, OpenGL ES, WebGPU (all enabled by default) |
| **Shader Languages** | WGSL (default), SPIR-V, GLSL (feature flags) |
| **Transpiler** | Naga (validates → transpiles to backend-native) |

### Feature Availability Matrix

| Feature | Vulkan | DX12 | Metal | WebGPU | WebGL2 |
|---------|--------|------|-------|--------|--------|
| Compute shaders | ✅ | ✅ | ✅ | ✅ | ❌ |
| Indirect draw | ✅ | ✅ | ✅ | ✅ | ❌ (emulated) |
| Multi-draw-indirect | ✅ | ✅ | ✅ | ❌ (Milestone 2) | ❌ |
| Pipeline cache | ✅ | ❌ | ❌ | ❌ | ❌ |
| Ray tracing | 🧪 | 🧪 | ❌ | ❌ | ❌ |
| Mesh shaders | 🧪 | 🧪 | ❌ | ❌ | ❌ |
| Sparse textures | 🧪 | 🧪 | ❌ | ❌ | ❌ |
| Timestamp queries | ✅ | ✅ | ✅ | ✅ | ❌ |

🧪 = experimental feature flag required

### Experimental API Highlights (wgpu 29)

**Ray Tracing** (`RAY_TRACING` feature):
- Acceleration structure build/update
- Ray query in compute/fragment shaders
- Vulkan 1.3+ / DX12 Ultimate only

**Mesh Shaders** (`MESH_SHADER` feature):
- Task + mesh shader pipeline
- Vulkan 1.3+ / DX12 Ultimate only
- Tracked: [wgpu#2948](https://github.com/gfx-rs/wgpu/issues/2948)

**Bindless** (not yet in WebGPU spec):
- Tracked: [gpuweb#5582](https://github.com/gpuweb/gpuweb/issues/5582)
- Workaround: large texture arrays + dynamic indexing

---

## 4. Render Graph / Frame Graph Architecture

### Concept

A directed acyclic graph (DAG) where nodes are render passes and edges are data dependencies (textures, buffers). Enables automatic resource management, parallel execution, and GPU resource aliasing.

### Bevy's RenderGraph (Reference Implementation)

```rust
pub struct RenderGraph { /* nodes, edges */ }

pub trait Node {
    fn update(&mut self, world: &mut World) {}
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError>;
}

pub enum SlotType { TextureView, Sampler, Buffer, BufferBinding }
```

**Key patterns:**
- **Nodes** define render passes (ShadowPassNode, PbrNode, BloomNode)
- **Edges** specify data flow between nodes
- **Subgraphs** allow per-camera composability (CameraDriverNode runs per-camera subgraph)
- **ViewNode** trait for per-view rendering (cameras, shadow cascades)
- Resource lifetimes managed automatically via slot type system

### Blade (kvark) — Minimal Approach

- Low-overhead graph execution over Vulkan/Metal/DX12
- Emphasizes zero-cost abstractions
- Used in Zed editor for GPU-accelerated text rendering
- Source: [github.com/kvark/blade](https://github.com/kvark/blade)

### Known Limitations

- Manual dependency specification (not auto-detected)
- Resource aliasing not automatic (unlike Frostbite's FrameGraph)
- Graph visualization/debugging tools limited vs Unity's FrameDebugger
- 5–10% CPU overhead for graph scheduling vs. hard-coded passes (acceptable for complex scenes)

---

## 5. GPU-Driven Rendering

### Concept

Offload CPU work (culling, LOD selection, draw call generation) to compute shaders. The GPU decides what to render via indirect draw commands.

### wgpu Implementation

```wgsl
// Compute shader: frustum cull + LOD select → build indirect buffer
@compute @workgroup_size(64)
fn cull_and_build(@builtin(global_invocation_id) gid: vec3<u32>) {
    let instance_id = gid.x;
    if (frustum_test(instances[instance_id])) {
        let draw_id = atomicAdd(&indirect_count, 1u);
        indirect_buffer[draw_id] = DrawIndirectArgs {
            vertex_count: mesh_vertex_count,
            instance_count: 1u,
            first_vertex: mesh_offset,
            first_instance: instance_id,
        };
    }
}
```

```rust
// Render pass consumes indirect buffer
render_pass.draw_indirect(&indirect_buffer, offset);
```

### Current Status

- ✅ `RenderPass::draw_indirect()` and `draw_indexed_indirect()` — supported
- ❌ Multi-draw-indirect — NOT in WebGPU spec (tracked: [gpuweb#5582](https://github.com/gpuweb/gpuweb/issues/5582), Milestone 2)
- 🧪 Mesh shaders — experimental, Vulkan/DX12 only
- Bevy: CPU-driven by default; community GPU culling work ongoing

### Known Issues

- Some Qualcomm/Mali drivers have buggy indirect draw (crash or hang)
- WebGL2: no indirect draw support (wgpu emulates via direct draws)
- GPU-generated draw calls invisible to RenderDoc/PIX until execution
- Vulkan 6× slower than DX12 in some cases (driver-specific, [wgpu#9020](https://github.com/gfx-rs/wgpu/discussions/9020))

---

## 6. Buffer Management & Memory

### Write Strategies

```rust
// Simple (creates staging per call — fine for <1MB/frame)
queue.write_buffer(&gpu_buffer, offset, &data);

// More efficient: write directly to staging region
queue.write_buffer_with(&gpu_buffer, offset, size, |staging| {
    staging.copy_from_slice(&data);
});
```

### Ring Buffers (Roll Your Own)

```rust
struct RingBuffer {
    buffer: wgpu::Buffer,
    frame_offsets: [u64; 3], // triple-buffered
    current_frame: usize,
}
// Write to next frame's region; GPU reads from 2 frames ago
// Prevents GPU/CPU synchronization stalls
```

### Persistent Mapped Buffers

```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
    mapped_at_creation: true,
    ..
});
let mut view = buffer.slice(..).get_mapped_range_mut();
view.copy_from_slice(&data);
drop(view);
buffer.unmap();
```

Best for streaming data (vertex uploads, dynamic meshes). Avoid on discrete GPUs without resizable BAR.

### Anti-Patterns

| Pattern | Problem | Solution |
|---------|---------|----------|
| Staging buffer per frame | Allocation overhead | Persistent ring buffer or `write_buffer()` |
| `MAP_READ` in hot loop | Async readback 100–1000× slower | Keep data on GPU |
| Ignoring 256-byte row alignment | Texture upload corruption | Pad to `COPY_BYTES_PER_ROW_ALIGNMENT` |
| Deleting buffers mid-frame | GPU still referencing | Defer until after submit completes |

---

## 7. Pipeline Caching

### wgpu PipelineCache API

```rust
// Load cache from disk
let cache = device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
    label: Some("main_cache"),
    data: Some(&std::fs::read("pipeline.cache").unwrap_or_default()),
    fallback: false,
});

// Use during pipeline creation
let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    cache: Some(&cache),
    ..
});

// Save to disk (atomic write)
if let Some(data) = cache.get_data() {
    let tmp = "pipeline.cache.tmp";
    std::fs::write(tmp, &data)?;
    std::fs::rename(tmp, "pipeline.cache")?;
}
```

### Backend Support

| Backend | Supported | Notes |
|---------|-----------|-------|
| Vulkan | ✅ | Full VkPipelineCache, 20–50% faster startup |
| DX12 | ❌ | Returns empty data (drivers cache internally) |
| Metal | ❌ | Returns empty data (drivers cache internally) |
| WebGPU | ❌ | Not supported |

### Best Practices

- Store per-device (key on adapter name + driver version)
- Use atomic writes (write tmp → rename) to prevent corruption
- Invalidate on shader source change (hash shader → compare)
- No built-in size reduction — cache grows over time (10–50 MB for complex apps)

---

## 8. Lighting & Shadows

### 8.1 Clustered Forward+ Shading

Divides the view frustum into a 3D grid of clusters (typical: 16×8×24 = 3,072). Each cluster tracks which lights affect it, allowing thousands of dynamic lights with minimal per-pixel cost.

**Implementation:**
1. Build 3D cluster grid in compute shader during depth pre-pass
2. Assign lights to clusters via atomic operations or prefix sum
3. Store cluster data in storage buffers (light indices, counts)
4. Fragment shader reads cluster at pixel's depth, iterates only relevant lights
5. Compatible with MSAA (unlike deferred)

**When to use:** Scenes with 100+ dynamic lights; open-world mixed indoor/outdoor; when transparency needs proper lighting.

### 8.2 Cook-Torrance Microfacet BRDF (GGX)

Industry-standard PBR BRDF: `f(l,v) = D(h)F(v,h)G(l,v,h) / [4(n·l)(n·v)]`

```wgsl
// GGX Normal Distribution
fn D_GGX(NdotH: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom);
}

// Schlick-GGX Geometry (merged with denominator for efficiency)
fn V_SmithGGXCorrelated(NdotV: f32, NdotL: f32, roughness: f32) -> f32 {
    let a2 = roughness * roughness;
    let GGXV = NdotL * sqrt(NdotV * NdotV * (1.0 - a2) + a2);
    let GGXL = NdotV * sqrt(NdotL * NdotL * (1.0 - a2) + a2);
    return 0.5 / (GGXV + GGXL);
}

// Fresnel-Schlick
fn F_Schlick(VdotH: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - VdotH, 5.0);
}
```

**Optimization (Brian Karis, Epic):** Merge Smith G-term with BRDF denominator to form the "visibility" function `V_SmithGGXCorrelated`, eliminating the `4(n·l)(n·v)` division.

### 8.3 Linearly Transformed Cosines (LTC) — Area Lights

Real-time polygonal area lights via a 3×3 matrix that transforms a clamped cosine into a GGX-like distribution. Closed-form polygon irradiance.

**Requirements:**
- Pre-computed LTC fit tables (roughness × θ) as 64×64 textures (~16 KB)
- Per-pixel: sample LTC matrix + amplitude, transform light polygon by inverse matrix, compute irradiance

```wgsl
let uv = vec2(NdotV, roughness);
let ltc_mat = textureSample(ltc_matrix_tex, samp, uv);
let ltc_amp = textureSample(ltc_amplitude_tex, samp, uv).r;

// Transform light corners by M^-1, then compute polygon form factor
let spec = polygon_irradiance(transformed_corners) * ltc_amp;
```

**Source:** SIGGRAPH 2016, Heitz et al. — [eheitzresearch.wordpress.com/415-2](https://eheitzresearch.wordpress.com/415-2/)

### 8.4 Shadow Mapping Techniques

#### Cascaded Shadow Maps (CSM)

Standard for directional lights. 3–4 cascades with logarithmic split distances.

#### Percentage-Closer Soft Shadows (PCSS)

Contact-hardening shadows: variable PCF kernel based on blocker distance.

```wgsl
// 1. Blocker search — find average depth of occluders in search radius
fn find_blocker(shadow_map: texture_depth_2d, uv: vec2<f32>,
                receiver_depth: f32, search_radius: f32) -> f32 {
    var blocker_sum = 0.0;
    var blocker_count = 0.0;
    for (var i = 0u; i < POISSON_SAMPLES; i++) {
        let d = textureSample(shadow_map, samp, uv + poisson_disk[i] * search_radius);
        if (d < receiver_depth) {
            blocker_sum += d;
            blocker_count += 1.0;
        }
    }
    return blocker_sum / max(blocker_count, 1.0);
}

// 2. Penumbra width = (receiver - blocker) / blocker * light_size
// 3. PCF with variable kernel radius
```

#### Virtual Shadow Maps (UE5 Nanite Approach)

Clipmap-based shadow system with dynamic page allocation. Requires sparse textures + mesh shaders or compute rasterization. Extremely complex (~1000s LOC), high memory (GBs for shadow cache). RTX 2000+ / RDNA2+ required.

### 8.5 Global Illumination

#### Dynamic Diffuse GI (DDGI)

Probe-based GI using ray tracing to dynamically update irradiance/visibility probes in a grid.

**Algorithm:**
1. For random subset of probes: cast ~144 rays (octahedral + fixed)
2. Accumulate irradiance and hit distance
3. Update probe atlas textures with exponential moving average
4. At shading: sample nearest 8 probes with visibility-aware weighting

**Requirements:** Ray tracing pipeline (`RAY_TRACING` feature), probe atlas textures, ~5–20 MB GPU memory per cascade.

**Limitations:** Dense probe placement required (~1 probe/m³ indoors), light leaking with thin walls, temporal lag.

**Source:** Morgan McGuire, NVIDIA — [morgan3d.github.io/articles/2019-04-01-ddgi](https://morgan3d.github.io/articles/2019-04-01-ddgi/)

#### Radiance Cascades (Experimental, 2024)

Hierarchical resolution/angular resolution tradeoffs. Multiple cascades store radiance at different spatial resolutions with varying ray counts. Primarily demonstrated in 2D; 3D work ongoing. Not production-ready.

**Source:** [jason.today/rc](https://jason.today/rc)

---

## 9. Post-Processing Pipeline

### 9.1 HDR & Tonemapping

Render to `TextureFormat::Rgba16Float` → apply exposure → tonemap → output `Rgba8UnormSrgb`.

**Tonemapping Options:**

| Operator | Origin | Pros | Cons |
|----------|--------|------|------|
| **ACES** | Film industry | De-facto standard, good color | Slight desaturation in brights |
| **AgX** | Blender 3.0+ | Better color preservation, open | Requires LUT or complex math |
| **Khronos PBR Neutral** | glTF/WebGPU 2023 | Open-spec, simpler than ACES | Less battle-tested |

```wgsl
// ACES (fitted approximation)
fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51; let b = 0.03;
    let c = 2.43; let d = 0.59; let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3(0.0), vec3(1.0));
}

// Khronos PBR Neutral
fn tonemap_pbr_neutral(color: vec3<f32>) -> vec3<f32> {
    let startCompression = 0.8 - 0.04;
    let desaturation = 0.15;
    var c = color;
    let x = min(c.r, min(c.g, c.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    c -= offset;
    let peak = max(c.r, max(c.g, c.b));
    if (peak < startCompression) { return c; }
    let d = 1.0 - startCompression;
    let newPeak = 1.0 - d * d / (peak + d - startCompression);
    c *= newPeak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    return mix(c, vec3(newPeak), g);
}
```

### 9.2 Temporal Anti-Aliasing (TAA)

Accumulates jittered frames over time for super-sampled quality. Foundation for temporal upsampling (DLSS, FSR2, TSR).

**Core algorithm:**
1. Generate jitter (Halton sequence, base 2/3, 8-frame cycle)
2. Reproject previous frame via motion vectors
3. Clamp history to current neighborhood (variance clipping)
4. Blend: `result = mix(history, current, 0.1)` with motion-adaptive factor

```wgsl
@fragment
fn taa_resolve(in: VertexOutput) -> @location(0) vec4<f32> {
    let curr = textureSample(current_tex, samp, in.uv);
    let motion = textureSample(motion_tex, samp, in.uv).xy;
    let hist_uv = in.uv - motion;
    var hist = textureSample(history_tex, samp, hist_uv);

    // Variance clipping (better than min/max clamping)
    let neighbors = sample_3x3_neighborhood(current_tex, in.uv);
    let mu = compute_mean(neighbors);
    let sigma = compute_stddev(neighbors);
    hist = clamp(hist, mu - sigma * 1.25, mu + sigma * 1.25);

    let blend = select(0.1, 0.3, length(motion) > 0.01); // responsive on motion
    return mix(hist, curr, blend);
}
```

**Key techniques:** Variance clipping (Uncharted 4), motion-adaptive blend, depth-based disocclusion, Mitchell spatial filter.

### 9.3 FSR2 / Temporal Upsampling

AMD's open-source temporal upscaling. Renders lower → reconstructs higher using TAA + Lanczos + RCAS sharpening.

| Mode | Scale Factor | Perf Gain |
|------|-------------|-----------|
| Ultra Quality | 1.3× | ~40% |
| Quality | 1.5× | ~60% |
| Balanced | 1.7× | ~70% |
| Performance | 2.0× | ~100% |

**Inputs:** Color (HDR pre-tonemap), Depth (reversed-Z preferred), Motion vectors (pixels/frame), optional Reactive mask.

**Mipmap bias:** `log2(render_res / display_res) - 1.0`

### 9.4 SSAO

```wgsl
@fragment
fn ssao(in: VertexOutput) -> @location(0) f32 {
    let depth = textureSample(depth_tex, samp, in.uv).r;
    let view_pos = reconstruct_view_pos(in.uv, depth);
    let normal = textureSample(normal_tex, samp, in.uv).xyz;

    // Random rotation from 4×4 noise texture
    let noise = textureSample(noise_tex, samp, in.uv * screen_size / 4.0).xy;
    let tangent = normalize(noise - normal * dot(noise, normal));
    let bitangent = cross(normal, tangent);
    let TBN = mat3x3(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0u; i < 64u; i++) {
        let sample_pos = view_pos + TBN * kernel[i] * radius;
        let sample_uv = project_to_screen(sample_pos);
        let sample_depth = linearize_depth(textureSample(depth_tex, samp, sample_uv).r);
        let range_check = smoothstep(0.0, 1.0, radius / abs(view_pos.z - sample_depth));
        occlusion += select(0.0, range_check, sample_depth >= sample_pos.z + bias);
    }
    return 1.0 - (occlusion / 64.0);
}
```

**Modern variants:** HBAO+ (NVIDIA, horizon-based), GTAO (Activision, ground-truth), ASSAO (Intel, multi-scale).

**Optimization:** Render at half-res, edge-aware bilateral blur, temporal accumulation.

### 9.5 Bloom (Dual Kawase)

```wgsl
// Downsample: 5-tap
@fragment
fn bloom_down(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset = 1.0 / vec2<f32>(textureDimensions(input_tex)) * 2.0;
    var col = textureSample(input_tex, samp, in.uv);
    col += textureSample(input_tex, samp, in.uv + vec2(-offset.x, -offset.y));
    col += textureSample(input_tex, samp, in.uv + vec2( offset.x, -offset.y));
    col += textureSample(input_tex, samp, in.uv + vec2(-offset.x,  offset.y));
    col += textureSample(input_tex, samp, in.uv + vec2( offset.x,  offset.y));
    return col / 5.0;
}
// Upsample: 9-tap, blend into next higher mip — same pattern
```

6–8 mip chain typical for 1080p. Fast and high quality.

---

## 10. Terrain Generation

### 10.1 Noise-Based Generation

#### FastNoiseLite (Recommended)

```rust
use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};

let mut noise = FastNoiseLite::new();
noise.set_noise_type(NoiseType::OpenSimplex2);
noise.set_fractal_type(FractalType::FBm);
noise.set_fractal_octaves(6);
noise.set_fractal_lacunarity(2.0);
noise.set_fractal_gain(0.5); // H=1.0 → G=0.5 for isotropic terrain

let height = noise.get_noise_2d(x, y);
```

15+ algorithms, domain warp support, HLSL/GLSL ports (translate to WGSL for compute). Source: [github.com/Auburn/FastNoiseLite](https://github.com/Auburn/FastNoiseLite)

#### Fractional Brownian Motion (fBM)

```rust
fn fbm(pos: Vec2, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    for _ in 0..octaves {
        total += amplitude * noise(pos * frequency);
        frequency *= lacunarity; // 2.0
        amplitude *= gain;       // 0.5 for H=1.0
    }
    total
}
```

**Key:** H=1.0 (G=0.5) produces geologically plausible terrain matching real mountain profiles (validated via spectral analysis: -9 dB/octave decay, f^-3). Source: [iquilezles.org/articles/fbm/](https://iquilezles.org/articles/fbm/)

#### Domain Warping

```rust
fn domain_warp(pos: Vec2, warp_strength: f32) -> f32 {
    let offset = Vec2::new(
        noise(pos) * warp_strength,
        noise(pos + Vec2::new(5.2, 1.3)) * warp_strength,
    );
    noise(pos + offset)
}
```

### 10.2 GPU Compute Generation

```wgsl
@group(0) @binding(0) var<storage, read_write> heightmap: array<f32>;
@group(0) @binding(1) var<uniform> params: TerrainParams;

@compute @workgroup_size(8, 8, 1)
fn generate_terrain(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<f32>(f32(id.x), f32(id.y));
    let world_pos = pos * params.scale + params.offset;

    var height = 0.0;
    var freq = 1.0;
    var amp = 1.0;
    for (var i = 0u; i < params.octaves; i++) {
        height += amp * simplex_noise(world_pos * freq);
        freq *= 2.0;
        amp *= 0.5;
    }
    heightmap[id.y * params.width + id.x] = height;
}
```

**Performance:** 1024×1024 heightmap in ~2–5 ms on modern GPUs.

### 10.3 Hydraulic Erosion (GPU-Accelerated)

Particle-based droplet simulation. Algorithm (Ranmantaru, 2011):

1. Spawn droplet: water=1, velocity=0, sediment=0
2. Compute gradient → move downhill
3. Carry capacity: `q = max(slope, 0.05) * velocity * water * Kq`
4. If sediment < capacity → erode; if > → deposit
5. Velocity: `v = sqrt(v² + g * Δh)`
6. Evaporate: `water *= (1 - Kw)`

**Parameters:** Kq=10, Kr=0.9, Kd=0.02, Kw=0.001, minSlope=0.05

**GPU:** Spawn 10k–100k droplets in parallel, atomic heightmap updates. ~100× speedup vs CPU.

Source: [ranmantaru.com/blog/2011/10/08/water-erosion-on-heightmap-terrain/](https://ranmantaru.com/blog/2011/10/08/water-erosion-on-heightmap-terrain/)

### 10.4 Biome Systems (Whittaker Diagram)

```rust
fn get_biome(temp: f32, moisture: f32, altitude: f32) -> Biome {
    let adjusted_temp = temp - altitude * 0.5; // colder at altitude
    match (adjusted_temp, moisture) {
        (t, _) if t < -0.5 => Biome::Tundra,
        (t, m) if t < 0.0 && m < 0.3 => Biome::Taiga,
        (t, m) if t > 0.5 && m < 0.2 => Biome::Desert,
        (_, m) if m > 0.7 => Biome::Rainforest,
        _ => Biome::Grassland,
    }
}
```

Generate in compute shader → output RGBA8 texture (weights for 4-biome blend).

### 10.5 Caves & Overhangs (Dual Contouring)

Dual contouring preferred over marching cubes for sharp features. Requires SDF + gradient at each edge, solves QEF for vertex placement.

Source: [boristhebrave.com/2018/04/15/dual-contouring-tutorial/](https://www.boristhebrave.com/2018/04/15/dual-contouring-tutorial/)

---

## 11. Terrain Rendering

### 11.1 Geometry Clipmaps (Industry Standard)

Nested regular grids centered on camera, incrementally updated. Each level is 2× coarser than previous.

**Structure:**
- L levels (typically 11), each n×n grid (n=255 common)
- Level 0 (finest): complete grid
- Level 1+: hollow rings
- Toroidal addressing for efficient updates

```wgsl
@vertex
fn vs_clipmap(@location(0) grid_pos: vec2<f32>) -> VSOutput {
    let world_pos = grid_pos * scale + offset;
    let uv = grid_pos * uv_scale + uv_offset;

    // Sample vertex texture for elevation
    let z = textureSampleLevel(elevation_tex, samp, uv, 0.0).r;

    // Blend at level boundaries for smooth transitions
    let alpha = compute_blend_alpha(world_pos, viewer_pos);
    // ...
}
```

**Update per frame per level:** Check if viewer moved > grid_spacing/2. If yes, update L-shaped region. Upsample coarser level → add residuals → compute normals.

**Performance (GPU Gems 2):** 130 FPS @ 1024×768, 60M triangles/sec, 20B samples (USA @ 30m) in 355 MB RAM.

Source: [NVIDIA GPU Gems 2 Chapter 2](https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry)

### 11.2 Triplanar Mapping

```wgsl
fn sample_triplanar(tex: texture_2d<f32>, samp: sampler,
                    world_pos: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    let x_proj = textureSample(tex, samp, world_pos.yz);
    let y_proj = textureSample(tex, samp, world_pos.xz);
    let z_proj = textureSample(tex, samp, world_pos.xy);

    var blend = abs(normal);
    blend = pow(blend, vec3(4.0)); // sharpen transitions
    blend /= dot(blend, vec3(1.0));

    return x_proj * blend.x + y_proj * blend.y + z_proj * blend.z;
}
```

3× texture samples — use for detail textures and steep surfaces only.

### 11.3 Splat Mapping (Multi-Layer)

```wgsl
@fragment
fn fs_terrain(in: VSOutput) -> @location(0) vec4<f32> {
    let weights = textureSample(splatmap, samp, in.uv); // RGBA = 4 biome weights
    let grass = sample_triplanar(grass_tex, samp, in.world_pos, in.normal);
    let rock  = sample_triplanar(rock_tex, samp, in.world_pos, in.normal);
    let sand  = textureSample(sand_tex, samp, in.uv * 10.0);
    let snow  = textureSample(snow_tex, samp, in.uv * 5.0);

    return height_blend(
        grass * weights.r + rock * weights.g + sand * weights.b + snow * weights.a,
        weights,
        vec4(grass.a, rock.a, sand.a, snow.a)
    );
}
```

### 11.4 Camera-Relative Rendering (Large Worlds)

Beyond ~10 km from origin, fp32 causes vertex jitter and z-fighting.

```rust
// CPU side (f64 precision)
let camera_pos_f64: DVec3 = get_camera_world_pos();
let camera_origin = camera_pos_f64.floor();

// Per object — compute f32 offset from camera
let relative_pos = (world_pos_f64 - camera_origin).as_f32();

// Shader receives relative coords → full fp32 precision maintained
```

Update ALL world-space data (lights, physics, culling) to camera-relative each frame.

---

## 12. Vegetation & Scatter Systems

### 12.1 Spatial Distribution

#### Mitchell's Best Candidate (Blue Noise)

For N points, generate M×N candidates, pick farthest from existing points. O(N²) naive, O(N) with spatial hash. M=1 sufficient for most cases. Toroidal distance wrapping prevents edge artifacts.

Source: [blog.demofox.org/2017/10/20/generating-blue-noise-sample-points-with-mitchells-best-candidate-algorithm/](https://blog.demofox.org/2017/10/20/generating-blue-noise-sample-points-with-mitchells-best-candidate-algorithm/)

#### Poisson Disc Sampling

Bridson's O(N) algorithm: maintains minimum distance between samples while maximizing coverage.

Source: [jasondavies.com/poisson-disc/](https://www.jasondavies.com/poisson-disc/)

### 12.2 Wind Animation (Gerstner Waves)

```
P' = P + Q × amplitude × direction × sin(dot(direction, P) × frequency + phase)
```

Steepness parameter Q controls lateral vs vertical motion (0.0–1.0). Multiple waves with different frequencies create natural motion. Directly transferable to grass blade animation.

Source: [GPU Gems 1 Chapter 1](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models)

### 12.3 GPU Instancing

```rust
render_pass.draw(vertices, 0..instance_count);
render_pass.draw_indexed(indices, base_vertex, 0..instance_count);
render_pass.multi_draw_indirect(buffer, offset, draw_count); // batch draws
```

Requires `INDIRECT_EXECUTION` downlevel flag. Compute shader updates transforms → storage buffer → indirect draw.

### 12.4 LOD Strategy

- **NanoMesh (SIGGRAPH 2024, Tencent):** Automatic LOD generation + GPU-driven rendering for mobile. Removes manual LOD authoring.
- **Concurrent Binary Trees (Intel, SIGGRAPH 2024):** GPU-friendly adaptive tessellation, <0.2 ms on PS5.
- **Distance-based 2–4 level LOD:** Crossfade or dither transition between levels.

### 12.5 Rust Ecosystem Gap

| Crate | Version | Downloads | Status |
|-------|---------|-----------|--------|
| warbler_grass | 0.6.1 | 37,446 | 2+ years old |
| frosty_grass | 0.1 | 1,813 | Limited adoption |
| bevy_procedural_grass | 0.2.0 | 5,783 | Bevy-specific, outdated |

**Recommendation:** Build custom implementation. No current (2024–2026) production vegetation crates exist for wgpu.

---

## 13. Atmosphere, Weather & Volumetrics

### 13.1 Bruneton Precomputed Atmospheric Scattering

Physically-based sky rendering via precomputed multiple-scattering LUTs.

**LUTs (one-time compute, ~100 ms):**
- Transmittance: 256×64 (optical depth)
- Scattering: 32×128×32×8 (4D: r, μ, μ_s, ν)
- Irradiance: 64×16 (ground irradiance)

**Runtime:** ~3 texture lookups per pixel (cheap).

**Parameters:** Planet radius (6371 km Earth), atmosphere height (100 km), Rayleigh/Mie coefficients, ozone absorption (sunset red).

Source: [ebruneton.github.io/precomputed_atmospheric_scattering/](https://ebruneton.github.io/precomputed_atmospheric_scattering/)

### 13.2 Froxel-Based Volumetric Fog (Frostbite)

Frustum-aligned 3D grid ("froxels"), each storing accumulated lighting/fog density.

**Algorithm:**
1. Build 3D grid (e.g., 160×90×64 froxels)
2. Per-froxel compute: accumulate shadowmapped light, store in-scattering + transmittance
3. Ray-march from camera through froxels in fragment shader

**Cost:** ~64 MB for 160×90×64 RGBA16F. Temporal filtering reduces noise.

Source: SIGGRAPH 2015, "Physically-Based and Unified Volumetric Rendering in Frostbite"

### 13.3 Volumetric Clouds (Guerrilla/Schneider)

Ray-march through 3D noise (Perlin-Worley, 128³) with Beer-Powder multiple-scattering approximation.

**Amortization:** Temporal reprojection — 1/16 pixels fully computed per frame, accumulated over 16 frames. Adds ~3–5 ms for 1080p.

Source: SIGGRAPH 2017, "Nubis: Real-Time Volumetric Cloudscapes" (Guerrilla)

### 13.4 Weather System Techniques

| Technique | Approach | Cost |
|-----------|----------|------|
| Screen-space rain | 2D streak overlay, depth-occluded | 1–2 ms |
| Volumetric precipitation | Ray-march through density field | 2–4 ms |
| Snow deformation | Runtime heightmap manipulation | 1–2 ms |
| Puddle formation | Post-process wetness maps from depth | 0.5–1 ms |

**SIGGRAPH 2024 relevant talks:**
- **Neural Light Grid (Activision):** ML-based irradiance volumes, eliminates light leaking. Shipped in COD Warzone/MW3.
- **GIBS Dynamic GI (EA Frostbite):** Surfel-based GI at 60fps on PS5/XSX. Shipped in College Football 25.
- **Hemispherical Lighting (Activision):** HHD lightmap model, jitter-free 240fps+.

---

## 14. Particles & VFX

### 14.1 Off-Screen Rendering (Overdraw Reduction)

Industry standard technique from GPU Gems 3:

1. Render particles to low-res target (2×2 or 4×4 downscale)
2. Downsample depth buffer (max of 4 samples to reduce halos)
3. Depth-test particles against downsampled depth
4. Compose back with edge-aware upsampling

| Technique | FPS (GeForce 8800) | Pixel Overdraw |
|-----------|---------------------|----------------|
| Full-res | 25 fps | 46.9M pixels |
| Mixed 4×4 | 51 fps | 3.5M pixels |
| Low-res 4×4 | 61 fps | 2.9M pixels |

**GTX 1660 Ti estimate:** 3–4× these numbers.

Source: [GPU Gems 3 Chapter 23](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles)

### 14.2 Soft Particles

```wgsl
fn soft_particle(particle_depth: f32, scene_depth: f32, fade_scale: f32) -> f32 {
    return saturate(fade_scale * (scene_depth - particle_depth));
}
// Apply to alpha: color.a *= soft_particle(my_depth, scene_depth, 2.0);
```

Cost: 1 texture sample + 3 instructions per pixel.

### 14.3 Alpha Blending

```rust
BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
}
```

### 14.4 Compute-Driven Particle Simulation

```rust
// Compute pass — update particles
let mut cpass = encoder.begin_compute_pass(&Default::default());
cpass.set_pipeline(&compute_pipeline);
cpass.set_bind_group(0, &particle_bind_group, &[]);
cpass.dispatch_workgroups(workgroup_count, 1, 1);
drop(cpass);

// Render pass — draw from same buffer (automatic barrier via encoder ordering)
let mut rpass = encoder.begin_render_pass(&render_desc);
rpass.set_vertex_buffer(0, particle_buffer.slice(..));
rpass.draw(0..6, 0..particle_count);
```

Pattern from wgpu [boids example](https://github.com/gfx-rs/wgpu/tree/trunk/examples/src/boids).

### 14.5 Order-Independent Transparency (OIT)

| Approach | Quality | Cost | Notes |
|----------|---------|------|-------|
| Depth peeling | Exact | High (multi-pass) | N passes for N layers |
| A-Buffer | Exact | Memory heavy | Per-pixel fragment lists |
| Weighted Blended OIT | Approximate | Low | McGuire & Bavoil 2013, recommended first attempt |
| Stochastic | Approximate | Low | Noisy but fast |

---

## 15. WGSL Shader Authoring

### 15.1 Performance Patterns

**Workgroup sizing:**
- `@workgroup_size(64, 1, 1)` for 1D dispatch (particles, culling)
- `@workgroup_size(8, 8, 1)` for 2D dispatch (image processing, terrain gen)
- Always multiples of 32 (NVIDIA warp / AMD wavefront)

**Branching:**
- Use `select()` for simple conditionals: `let v = select(a, b, condition);`
- Minimize divergent branches within workgroups
- Group similar work to reduce warp divergence

**Shared memory:**
```wgsl
var<workgroup> shared_data: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(local_invocation_index) lid: u32) {
    shared_data[lid] = input[lid];
    workgroupBarrier();
    let result = shared_data[lid] * 2.0; // fast local access
}
```

### 15.2 Specialization Constants

```rust
let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    constants: &HashMap::from([
        ("TILE_SIZE".to_string(), 16.0),
        ("USE_ALPHA".to_string(), 1.0),
    ]),
    ..
});
```

### 15.3 Hot-Reloading

```rust
// Watch shader files with `notify` crate
let shader_source = std::fs::read_to_string("shader.wgsl")?;
let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Hot Reloaded Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
});
// Recreate pipeline with new module; invalidate pipeline cache
```

### 15.4 Naga Limitations

- WGSL is primary target; SPIR-V and GLSL supported but may have transpilation quirks
- Naga validates → transpiles to backend-native (SPIR-V for Vulkan, MSL for Metal, HLSL for DX12)
- Some advanced GLSL extensions (bindless) don't transpile cleanly
- rust-gpu moved to community maintenance (Embark archive Oct 2025) — use WGSL for stability

---

## 16. Asset Pipeline

### 16.1 Mesh Optimization (meshopt)

```rust
use meshopt::*;

// LOD generation
let lod1 = simplify(&vertices, &indices, target_error);

// Vertex cache optimization
let optimized = optimize_vertex_cache(&indices, vertices.len());

// Meshlet generation for GPU culling
let meshlets = build_meshlets(&optimized, vertices.len(), 64, 126);
```

Modules: `optimize` (cache, overdraw, fetch), `simplify`, `stripify`, `clusterize` (meshlets), `encoding` (quantization).

### 16.2 Texture Compression (basis_universal v2.1)

7 codec modes:

| Format | BPP | Quality | Transcode Speed | Use Case |
|--------|-----|---------|-----------------|----------|
| ETC1S | 0.3–3 | Medium | ~30 MB/s | Download size critical |
| UASTC LDR 4×4 | 8 | High | 500–1000 MB/s | Fast transcode to BC7/ASTC |
| UASTC HDR 4×4 | 8 | High (HDR) | Fast | HDR content |
| XUASTC LDR | 0.3–5.7 | High | ~100 MB/s | Best size/quality balance |

**Recommended pipeline:**
1. Author: PNG/TGA source
2. Compress: `basisu` CLI → `.basis` or `.ktx2`
3. Runtime: Transcode to BC7 (desktop), ASTC (mobile), ETC2 (Android fallback)

Rust crate: [basis-universal](https://crates.io/crates/basis-universal)

### 16.3 Mipmap Generation

```rust
let texture = device.create_texture(&wgpu::TextureDescriptor {
    mip_level_count: ((size.max_element() as f32).log2().floor() as u32) + 1,
    ..
});
// Generate on GPU via compute blit (previous mip → current with linear filter)
```

Helper: [wgpu_mipmap](https://github.com/expenses/wgpu_mipmap)

### 16.4 Hot-Reloading Assets

```rust
use notify::{Watcher, RecursiveMode, Event};

let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
    if let Ok(event) = res {
        if event.kind.is_modify() {
            tx.send(event.paths[0].clone()).ok();
        }
    }
})?;
watcher.watch(Path::new("assets/"), RecursiveMode::Recursive)?;

// In render loop
if let Ok(path) = asset_reload_rx.try_recv() {
    reload_texture(&device, &queue, &path);
}
```

---

## 17. Rust Renderer Patterns

### 17.1 Extract-Prepare-Queue-Render (Bevy Pattern)

```rust
// EXTRACT: Copy ECS → render world (cheap Arc clones)
fn extract_meshes(query: Query<(&Transform, &Mesh)>, mut extracted: ResMut<Vec<ExtractedMesh>>) {
    extracted.clear();
    for (xform, mesh) in query.iter() {
        extracted.push(ExtractedMesh {
            transform: xform.compute_matrix(),
            handle: mesh.handle.clone(),
        });
    }
}

// PREPARE: Upload to GPU (bulk write)
fn prepare_uniforms(extracted: Res<Vec<ExtractedMesh>>, queue: Res<RenderQueue>,
                    mut buf: ResMut<MeshUniformBuffer>) {
    buf.clear();
    for mesh in extracted.iter() {
        buf.push(MeshUniform { model: mesh.transform });
    }
    buf.write_buffer(&queue);
}

// RENDER: Record commands
fn render_meshes(extracted: Res<Vec<ExtractedMesh>>, pipeline: Res<MeshPipeline>,
                 mut pass: RenderPassHandle) {
    pass.set_pipeline(&pipeline);
    for (i, mesh) in extracted.iter().enumerate() {
        pass.set_bind_group(0, &mesh_uniforms, &[i as u32]);
        pass.draw_mesh(&mesh.handle);
    }
}
```

### 17.2 Parallel Data Preparation with Rayon

```rust
use rayon::prelude::*;

// Parallel CPU work (frustum culling, sorting)
let prepared: Vec<PreparedDraw> = entities.par_iter()
    .filter(|e| frustum.contains(e.bounds))
    .map(|e| PreparedDraw {
        mesh: e.mesh.clone(),
        transform: e.transform.compute_matrix(),
    })
    .collect();

// Serial GPU recording (wgpu doesn't support multi-encoder)
for draw in &prepared {
    render_pass.draw_prepared(draw);
}
```

**Rayon guideline (AstraWeave-specific):** Only parallelize workloads > 5 ms. Rayon overhead is ~50–100 µs.

### 17.3 Zero-Copy Renderer Middleware

```rust
pub struct MeshRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,  // persistent, reused every frame
    index_buffer: Buffer,
}

impl MeshRenderer {
    pub fn prepare(&mut self, queue: &Queue, data: &[Vertex]) {
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(data));
    }

    pub fn render<'a>(&'a self, pass: &mut RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
```

### 17.4 GPU Timestamp Profiling

```rust
let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
    label: Some("timestamps"),
    ty: wgpu::QueryType::Timestamp,
    count: 2,
});

render_pass.write_timestamp(&query_set, 0); // start
// ... draw calls
render_pass.write_timestamp(&query_set, 1); // end

encoder.resolve_query_set(&query_set, 0..2, &resolve_buffer, 0);
// Map resolve_buffer → read timestamps → compute duration
```

---

## 18. Reference Renderers

### Active Projects (2024–2026)

| Project | Status | Architecture | Key Features | Learning Value |
|---------|--------|-------------|--------------|----------------|
| **Bevy** | Active (0.18.1) | ECS + RenderGraph | Clustered forward+, PBR, CSM, TAA, Bloom, SSAO, IBL, LTC area lights | ⭐⭐⭐⭐⭐ |
| **Blade** (kvark) | Active (2025) | Minimal GPU abstraction | Low-overhead Vulkan/Metal/DX12, zero-cost abstractions, used in Zed editor | ⭐⭐⭐⭐ |

### Archived (Educational)

| Project | Architecture | Key Techniques | Notes |
|---------|-------------|----------------|-------|
| **Kajiya** (Embark) | Hybrid rasterization + RT | Volumetric temporally-recurrent irradiance cache, ReSTIR, TAA | No longer maintained; excellent learning resource |
| **rend3** (BVE-Reborn) | Forward+ PBR, glTF-focused | Early wgpu render graph patterns | Archived 2023 |
| **Ambient** (AmbientRun) | ECS + WASM | PBR, WebGPU focus, network real-time DB | Development paused indefinitely |

### Emerging Patterns (2025–2026)

- **Bindless rendering:** Not yet in WebGPU spec (Milestone 2). Workaround: large texture arrays + dynamic indexing.
- **Mesh shaders:** Experimental in wgpu 29, Vulkan/DX12 only.
- **GPU-driven occlusion culling:** Two-pass Hi-Z not widely implemented in Rust yet. Compute frustum culling more common.

---

## 19. Vegetation & Scatter Budget Cheat Sheet

Performance budget for GTX 1660 Ti class hardware at 1080p/60fps:

| System | Instance Count | Frame Time | GPU Memory | Notes |
|--------|----------------|------------|------------|-------|
| **Grass (LOD 0–3)** | 500K–2M | 3–6 ms | 100 MB | Compute wind anim, 2–4 LOD levels |
| **Trees (billboard + mesh)** | 5K–20K | 2–4 ms | 200 MB | Billboard > 100m, mesh < 100m |
| **Rocks/debris** | 50K–100K | 1–2 ms | 50 MB | GPU instanced, no animation |
| **Ground cover** | 200K–500K | 1–2 ms | 50 MB | Alpha-test quads, distance fade |
| **Total scatter** | ~2M+ objects | **< 10 ms** | **< 400 MB** | |

**Instance buffer math:** 64 bytes/instance × 2M instances = 128 MB.

**Draw call strategy:** Single `multi_draw_indirect()` per LOD level. Batch 1000+ draw calls → 1 GPU submission.

**Bottleneck:** Memory bandwidth (192 GB/s) not compute. Build adaptive LOD system, profile on target hardware.

**Distribution:** Mitchell's Best Candidate (blue noise) for natural spacing. Poisson Disc for guaranteed minimum distance.

---

## 20. Weather System Integration Checklist

### Rain (Full Chain)

- [ ] **Particle spawn:** Compute shader spawns rain particles in view frustum volume
- [ ] **Motion simulation:** Apply gravity + wind → update positions in compute pass
- [ ] **Depth occlusion:** Test particle depth against scene depth buffer → kill particles behind geometry
- [ ] **Rendering:** Screen-space 2D streak quads (oriented along velocity), off-screen at 1/4 res
- [ ] **Splash VFX:** Spawn splash particles at depth-contact points
- [ ] **Wetness map:** Post-process pass: exposed surfaces → increase wetness (darken albedo, increase roughness)
- [ ] **Puddles:** Accumulate wetness in concavities → reflect environment with planar reflection or SSR
- [ ] **Audio integration:** Spatial rain sound, intensity proportional to density
- [ ] **Performance target:** 1–2 ms total (screen-space streaks + wetness)

### Snow (Full Chain)

- [ ] **Particle spawn:** Lower density, larger particles than rain, horizontal drift
- [ ] **Accumulation:** Runtime heightmap modification (raise surface from snow deposits)
- [ ] **Material blending:** Increase snow weight in splat map based on exposure + accumulated depth
- [ ] **Deformation:** Player/NPC footprints via depth stamp into snow heightmap
- [ ] **Wind interaction:** Gusts displace particles, create drifts at obstacle lee sides
- [ ] **Melt system:** Temperature-driven: reduce accumulation, increase wetness
- [ ] **Performance target:** 1–3 ms total (particles + heightmap update)

### Shared Infrastructure

- [ ] **Wind field:** Global 3D wind texture (16³), sampled by grass, trees, particles, clouds
- [ ] **Time-of-day:** Connect to atmospheric scattering (Bruneton), fog density, cloud coverage
- [ ] **Transition system:** Smooth interpolation between weather states (clear → overcast → rain)
- [ ] **Lightning:** Screen-space flash + directional light impulse + volumetric illumination
- [ ] **Performance budget:** Total weather ≤ 4 ms on GTX 1660 Ti

---

## 21. Sources

### Official Documentation
- [wgpu API Reference (v29.0.1)](https://docs.rs/wgpu/latest/wgpu/)
- [wgpu Examples Repository](https://github.com/gfx-rs/wgpu/tree/trunk/examples)
- [wgpu Wiki — Do's and Don'ts](https://github.com/gfx-rs/wgpu/wiki/Do's-and-Dont's)
- [wgpu Wiki — Debugging Applications](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications)
- [Learn-wgpu Tutorial](https://sotrh.github.io/learn-wgpu/)

### Conference Proceedings
- [SIGGRAPH 2024 Advances in Real-Time Rendering](https://advances.realtimerendering.com/s2024)
- [SIGGRAPH 2021 — Nanite, Virtual Shadow Maps (Epic)](https://advances.realtimerendering.com/s2021/)
- [SIGGRAPH 2017 — Nubis Volumetric Clouds (Guerrilla)](https://advances.realtimerendering.com/s2017/)
- [SIGGRAPH 2016 — LTC Area Lights (Heitz)](https://eheitzresearch.wordpress.com/415-2/)
- [SIGGRAPH 2015 — Frostbite Volumetric Fog](https://advances.realtimerendering.com/s2015/)

### GPU Gems
- [GPU Gems 1 Ch.1 — Water Simulation / Gerstner Waves](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-effective-water-simulation-physical-models)
- [GPU Gems 2 Ch.2 — Geometry Clipmaps](https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry)
- [GPU Gems 3 Ch.23 — High-Speed Screen Particles](https://developer.nvidia.com/gpugems/gpugems3/part-iv-image-effects/chapter-23-high-speed-screen-particles)

### Academic / Industry Papers
- [DDGI — Morgan McGuire, NVIDIA](https://morgan3d.github.io/articles/2019-04-01-ddgi/)
- [Bruneton Atmospheric Scattering](https://ebruneton.github.io/precomputed_atmospheric_scattering/)
- [Radiance Cascades — jason.today/rc](https://jason.today/rc)
- [Inigo Quilez — fBM / Noise](https://iquilezles.org/articles/fbm/)
- [Hydraulic Erosion — Ranmantaru](https://ranmantaru.com/blog/2011/10/08/water-erosion-on-heightmap-terrain/)
- [Dual Contouring Tutorial — Boris the Brave](https://www.boristhebrave.com/2018/04/15/dual-contouring-tutorial/)
- Brian Karis (Epic) — [Specular BRDF Reference](https://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html)
- [Filament PBR Documentation (Google)](https://google.github.io/filament/Filament.html)
- [FSR2 — GPUOpen](https://github.com/GPUOpen-Effects/FidelityFX-FSR2)
- [AgX Tonemapping](https://github.com/sobotka/AgX)

### Algorithms & Sampling
- [Mitchell's Best Candidate (Blue Noise)](https://blog.demofox.org/2017/10/20/generating-blue-noise-sample-points-with-mitchells-best-candidate-algorithm/)
- [Poisson Disc Demo — Jason Davies](https://www.jasondavies.com/poisson-disc/)
- [Sebastian Lague — Hydraulic Erosion](https://github.com/SebLague/Hydraulic-Erosion)

### Rust Ecosystem
- [meshopt crate](https://docs.rs/meshopt/latest/meshopt/)
- [basis-universal crate](https://crates.io/crates/basis-universal)
- [fastnoise-lite crate](https://crates.io/crates/fastnoise-lite)
- [noise crate](https://docs.rs/noise/latest/noise/)
- [rayon crate (v1.11)](https://docs.rs/rayon/latest/rayon/)
- [notify crate (file watching)](https://crates.io/crates/notify)
- [wgpu_mipmap](https://github.com/expenses/wgpu_mipmap)

### Reference Renderers
- [Bevy Engine](https://github.com/bevyengine/bevy)
- [Blade (kvark)](https://github.com/kvark/blade)
- [Kajiya (Embark Studios, archived)](https://github.com/EmbarkStudios/kajiya)

### Community Resources
- [RenderDoc](https://renderdoc.org/)
- [VulkanGuide](https://vkguide.dev/)
- [Rust Gamedev Working Group](https://gamedev.rs/)
- [kvark's Rust Optimization Notes](https://gist.github.com/kvark/f067ba974446f7c5ce5bd544fe370186)
- [kvark's Clustered Shading Notes](https://gist.github.com/kvark/4d400632714011f80ff1)
- [Graphics Pipeline Deep Dive — Fabian Giesen](https://alaingalvan.gitbook.io/a-trip-through-the-graphics-pipeline)
- Graphics Programming Discord
- /r/GraphicsProgramming
- Bevy Discord #rendering channel

### Books
- Real-Time Rendering, 4th Edition (Akenine-Möller et al.)
- Physically Based Rendering, 3rd Edition (Pharr, Jakob, Humphreys)

---

*Compiled: 2026-07-08 | wgpu 29.0.1 | Rust 1.89.0 | 60+ sources consulted*
