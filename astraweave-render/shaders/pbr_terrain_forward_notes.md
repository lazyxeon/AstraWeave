# Phase 1.D notes — SHADER_SRC composition and forward-lit terrain design

**Transient file. Delete before the Phase 1.F commit.**

## SHADER_SRC composition

`astraweave-render/src/renderer.rs:18-328` — built via `concat!(
include_str!("../shaders/constants.wgsl"),
include_str!("../shaders/brdf_common.wgsl"),
inline_wgsl,
)`.

Inline WGSL contains: struct VSIn / VSOut, struct Camera, MaterialUbo,
MainLightUbo, SceneEnv, IblParams, bind-group declarations for groups
0-5, helper fns sample_cloud_shadow + apply_scene_fog + apply_scene_tint
+ compute_ibl, vertex stage `vs`, fragment stage `fs`.

## Engine-side forward pass bind group layout (SHADER_SRC)

| Group | Binding(s) | Purpose |
|-------|-----------|---------|
| 0     | 0         | `uCamera: Camera` — view_proj (mat4x4), light_dir (vec3), _pad, camera_pos (vec3), _pad2 — 96 B |
| 1     | 0         | `uMaterial: MaterialUbo` — base_color (vec4), metallic, roughness, alpha_cutoff, _pad — 32 B |
| 2     | 0, 1, 2   | `uLight: MainLightUbo` + `shadow_tex: texture_depth_2d_array` + `shadow_sampler: sampler_comparison` |
| 3     | 0, 1, 2, 3, 4, 5 | Albedo tex + sampler, MR tex + sampler, Normal tex + sampler (single-material textures) |
| 4     | 0         | `uScene: SceneEnv` — fog_color, fog_density, fog_start, fog_end, ambient_color, ambient_intensity, tint_color, tint_alpha, blend_factor, sun_color, sun_intensity — 80 B |
| 5     | 0, 1, 2, 3, 4, 5, 6, 7, 8 | IBL specular + irradiance + brdf_lut + sampler + uIbl UBO + gi_tex + gi_samp + cloud_shadow_tex + cloud_shadow_samp |

## BRDF helpers available (brdf_common.wgsl)

- `fresnel_schlick(cos_theta, F0)` / `fresnel_schlick_roughness(cos_theta, F0, roughness)`
- `distribution_ggx(NdotH, roughness)`
- `visibility_smith_ggx(NdotV, NdotL, roughness)`
- `diffuse_burley(NdotV, NdotL, VdotH, roughness)`
- `compute_material_lod(world_pos)` — returns u32 LOD tier 0/1/2
- `evaluate_brdf(N, V, L, base_color, metallic, roughness, F0)` — full
- `evaluate_brdf_lod(..., lod)` — LOD-aware variant

## Forward-lit terrain pipeline design (Approach Y — self-contained groups)

Decision: do NOT attempt to share bind group layouts with SHADER_SRC.
Reason: the engine's BGLs are not publicly accessible in a way that
`TerrainMaterialManager` can safely import without coupling crates, and
the shader-group index coordination (placeholder BGLs for unused groups
to match SHADER_SRC's 6-group pipeline layout) is fragile.

Instead, the forward-lit terrain pipeline owns ALL its own bind groups.
The engine passes lighting state in each frame via a new method on the
manager, which writes it into manager-owned UBOs.

### Pipeline bind group layout (self-contained)

| Group | Binding | Purpose | Owner |
|-------|---------|---------|-------|
| 0     | 0       | `CameraTerrain` UBO — mirrors SHADER_SRC Camera struct byte-for-byte | TerrainMaterialManager |
| 1     | 0       | `TerrainMaterialGpu` UBO (existing) | TerrainMaterialManager |
| 1     | 1       | `TerrainSceneGpu` UBO (new, mirrors SceneEnv) | TerrainMaterialManager |
| 1     | 2       | sampler | TerrainMaterialManager |
| 1     | 3       | splat_map_0 (per-chunk) | TerrainMaterialManager |
| 1     | 4       | splat_map_1 (per-chunk) | TerrainMaterialManager |
| 1     | 5       | layer_albedo (array) | TerrainMaterialManager |
| 1     | 6       | layer_normal (array) | TerrainMaterialManager |
| 1     | 7       | layer_orm (array) | TerrainMaterialManager |

Wait — 8 bindings in group 1 collides with wgpu's default max of 8 per
group. Splitting splat_0/splat_1 into a per-chunk group 2 is how the
existing deferred pipeline solves this. Doing the same:

### Revised: three bind groups

| Group | Binding | Purpose | Set how often |
|-------|---------|---------|---------------|
| 0     | 0       | `CameraTerrain` UBO | once per frame |
| 1     | 0       | `TerrainMaterialGpu` UBO | once per material change |
| 1     | 1       | `TerrainSceneGpu` UBO | once per frame |
| 1     | 2       | sampler | once |
| 1     | 3       | layer_albedo (array) | once |
| 1     | 4       | layer_normal (array) | once |
| 1     | 5       | layer_orm (array) | once |
| 2     | 0       | splat_map_0 (per-chunk) | per chunk draw |
| 2     | 1       | splat_map_1 (per-chunk) | per chunk draw |

Group 1 now has 6 bindings. Group 2 has 2. Total 3 groups.

### Fragment output

Single `@location(0) vec4<f32>` — linear HDR.

### Lighting model (Phase 1)

- Sun direct: `evaluate_brdf_lod(N, V, L, base_color, metallic, roughness, F0, mat_lod) * sun_color * sun_intensity`
  - L = `normalize(-camera.light_dir)`
  - No shadow cascades — add in a follow-up Phase 1 iteration if needed for
    visual parity with legacy.
- Ambient fallback: `base_color * (1 - metallic) * ambient_color * ambient_intensity * 0.35` — mirrors
  SHADER_SRC's ambient fill at line 314.
- Fog: `apply_scene_fog(color, frag_dist)` — reuse SHADER_SRC's formula.
- No IBL, cloud shadows, SSGI for Phase 1. Phase 3 revisits visual quality.

### Splat blending (Phase 1)

- Sample splat_map_0 + splat_map_1 at fragment UV.
- Normalize weights across 8 biome slots.
- For each biome with weight > threshold, sample its albedo/normal/orm
  from the layer arrays.
- Accumulate:
  - albedo: weighted sum.
  - normal: RNM for the 2 dominant contributions (Phase 1 — cheaper than
    full 4-way RNM chain; sufficient for plumbing gate).
    Actually: take weight-scaled TBN perturbations and renormalize the
    result. This is basically UDN/Whiteout; RNM can be added later.
  - metallic/roughness: weighted sum of per-layer material_factors × per-layer ORM samples.
- Result fed into `evaluate_brdf_lod`.

### Vertex layout

Reuse `TerrainSplatVertex` (pos[3], normal[3], uv[2] — 32 B). Same as
deferred pipeline, so upload path stays identical.

## Integration

`Renderer::draw_into` (renderer.rs:5184+) runs a single forward pass to
`hdr_view`. New integration point: after the existing terrain-legacy
draws (or replacing them, behind feature flag), call
`manager.draw_chunk_forward(rpass, chunk_key, vb, ib, indices)` per
chunk with uploaded splats.

The manager's forward pipeline needs access to:
- vertex buffer (`TerrainSplatVertex`) — built per chunk by Phase 1.E.
- index buffer — built per chunk.
- Camera + Scene uniforms — updated each frame by the adapter.
- Material texture arrays — uploaded at project load.
- Per-chunk splat bind group — uploaded by Phase 1.C.

Since `draw_chunk_forward` is called from inside `Renderer::draw_into`'s
pass, it shares `hdr_view` and the depth buffer automatically.
