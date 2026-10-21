# Phase 1.E Handoff — Terrain Material System Campaign

**Status**: Phase 1 sub-steps 1.A, 1.0, 1.B, 1.C, 1.D complete and committed.
Phase 1.E (forward-lit pipeline registration + draw-into integration) not
yet started. Phase 1.F (cleanup + status update) follows 1.E.

**Why a handoff**: Phase 1.E requires ~500 LOC of careful Rust across three
files with interlocking bind-group orchestration. Executing it in a single
session alongside Phases 1.0–1.D risked a mid-integration rollback. This
document preserves the design sketch so a fresh session (or the same one
after review) can resume with full context.

**Anchor commits**:

- `1233537fe` — 1.A: `terrain-splat-arrays` default on.
- `6bb50bc83` — Deviation log (deferred-pipeline finding → Option A/D).
- `749046a74` — 1.0: campaign plan amended for Option D.
- `d62b6ab28` — 1.B: `terrain_splat: Option<EditorTerrainSplat>` field.
- `a2ef61491` — 1.C: splat builder + upload wired into `upload_terrain_chunks`.
- `(TBD)` — 1.D: `pbr_terrain_forward.wgsl` authored + validated.

---

## 1. What's in place (verified working)

### Cargo features

- `astraweave-render/Cargo.toml:8` — `default = ["postfx", "textures", "terrain-splat-arrays"]`
- `tools/aw_editor/Cargo.toml:17` — `default = [..., "terrain-splat-arrays"]`
- Feature-off fallback verified: `cargo check -p astraweave-render --no-default-features --features "postfx,textures"` passes.

### Editor adapter state (`tools/aw_editor/src/viewport/engine_adapter.rs`)

- Struct field: `terrain_splat: Option<EditorTerrainSplat>` — initialized
  in `new()` via `EditorTerrainSplat::new() + initialize(device, TerrainMaterialConfig::default())`.
- `upload_terrain_chunks`:
  - Calls `splat.clear_chunks()` before the chunk-accept loop.
  - Per-chunk: computes `grid_dim = floor(sqrt(vertex_count))`, uploads the
    square prefix via `splat.upload_chunk_from_vertices(device, queue,
    chunk_index as u64, &vertices[..grid²], grid_dim, grid_dim)`.
  - Debug log: `log::debug!("Phase 1 splat upload: chunk {key} {w}x{h} ...")`.
  - Skirt vertices omitted (duplicate boundary biome weights).

### Splat-array manager state (`astraweave-render/src/terrain_material_manager.rs`)

- Per-chunk splat textures uploaded via `EditorTerrainSplat`'s existing
  path into `chunk_splats: HashMap<ChunkKey, ChunkSplat>`.
- `TerrainMaterialGpu` uniform (existing, 576 B, in `terrain_material.rs`) —
  carries per-layer parameters. `set_material` has not yet been called from
  the editor (it will be during 1.E when biome materials are loaded).
- The existing `ensure_pipeline` / `draw_chunk` methods build the **deferred**
  3-target pipeline. They are **not** used by Phase 1's forward path; they
  remain on disk as reference.

### Forward shader (`astraweave-render/shaders/pbr_terrain_forward.wgsl`)

- Single `@location(0) vec4<f32>` HDR output.
- Bind group layout (see Section 2 below).
- Lighting: Cook-Torrance + Burley (via `evaluate_brdf_lod` from brdf_common.wgsl)
  with sun direct + ambient fallback + distance fog.
- Splat blending: normalizes weights over `active_layer_count`, samples
  per-layer albedo/normal/orm, UDN-style normal blend.
- Validates in `test_pbr_terrain_forward_validates_with_prefix` (shader_validation.rs).

---

## 2. Target bind-group layout for the forward pipeline

```
@group(0) @binding(0)  CameraTerrain UBO  (96 B — mirrors SHADER_SRC Camera)
@group(1) @binding(0)  TerrainParams UBO  (576 B — existing TerrainMaterialGpu)
          @binding(1)  TerrainSceneEnv UBO (new — mirrors SHADER_SRC SceneEnv)
          @binding(2)  terrain_sampler
          @binding(3)  layer_albedo: texture_2d_array<f32>
          @binding(4)  layer_normal: texture_2d_array<f32>
          @binding(5)  layer_orm: texture_2d_array<f32>
@group(2) @binding(0)  splat_map_0 (per-chunk)
          @binding(1)  splat_map_1 (per-chunk)
```

Group 1 has 6 bindings (under the 8-per-group default). Group 2 has 2 per-chunk.

**Important byte-layout constraints**:

- `CameraTerrain` must mirror SHADER_SRC's `Camera` struct (`renderer.rs:48-54`)
  byte-for-byte: `view_proj: mat4x4` (64) + `light_dir: vec3 + _pad: f32` (16)
  + `camera_pos: vec3 + _pad2: f32` (16) = **96 bytes total**. The existing
  `CameraUniformsGpu` in `terrain_material_manager.rs:172-178` is 80 bytes —
  NOT compatible. A new buffer is required.
- `TerrainSceneEnv` must mirror SHADER_SRC's `SceneEnv` (`renderer.rs:86-100`)
  byte-for-byte. Note the prompt's ordering of fields in the shader I wrote
  — confirm it matches by checking `renderer.rs:86-100` before coding the
  Rust-side `#[repr(C)]` struct.
- `TerrainParams` in the shader reuses the existing `TerrainMaterialGpu`
  struct directly (576 B). No layout change needed.

---

## 3. Remaining work inventory

### 3.1 — Extend `TerrainMaterialManager` (astraweave-render/src/terrain_material_manager.rs)

Add (keep the existing deferred path intact for reversibility):

- **Fields**:
  - `forward_camera_bgl: wgpu::BindGroupLayout`
  - `forward_terrain_bgl: wgpu::BindGroupLayout`
  - `forward_splat_bgl: wgpu::BindGroupLayout`
  - `forward_camera_buffer: wgpu::Buffer` (96 B)
  - `forward_scene_buffer: wgpu::Buffer` (mirror SHADER_SRC SceneEnv size)
  - `forward_camera_bg: wgpu::BindGroup`
  - `forward_terrain_bg: wgpu::BindGroup`
  - `forward_pipeline: Option<wgpu::RenderPipeline>`
  - `forward_pipeline_formats: Option<(wgpu::TextureFormat, Option<wgpu::TextureFormat>)>`
  - `forward_chunk_splats: HashMap<ChunkKey, ChunkSplatForward>` where
    `ChunkSplatForward { bind_group: wgpu::BindGroup, dims: (u32, u32) }`.
    Can reuse the existing per-chunk splat textures (owned in `chunk_splats`);
    just build a new-layout bind group referencing them. Alternatively,
    separate textures for forward path — less memory efficient but simpler.
  - `forward_shader_src: String` — computed once from `concat!(constants,
    brdf_common, pbr_terrain_forward)` like `TERRAIN_SPLAT_SHADER`.

- **Methods**:
  - `new_forward_shader_src()` — returns the concatenated forward shader source.
  - `ensure_forward_pipeline(device, color_format, depth_format)` — builds
    the forward pipeline with ONE `ColorTargetState` for the Rgba16Float HDR
    target + depth. Mirror the pattern in `ensure_pipeline` (600-681) but
    with 1 color attachment, not 3.
  - `update_forward_camera(queue, view_proj, light_dir, camera_pos)` — writes
    the 96-byte Camera UBO.
  - `update_forward_scene(queue, scene_env: &TerrainSceneEnvGpu)` — writes
    the scene env UBO. The adapter composes this from the renderer's
    existing `scene_env` state.
  - `set_chunk_splat_forward(device, queue, chunk: ChunkKey, splat_0: &[u8],
    splat_1: &[u8], dims: (u32, u32))` — uploads splat textures (if not
    already done via deferred path) and creates the forward-layout bind
    group. If sharing textures with deferred path, just create the bind
    group. If not, create fresh textures.
  - `draw_chunk_forward(rpass, chunk: ChunkKey, vertex_buffer, index_buffer,
    index_count)` — issues the draw. Sets pipeline, binds forward_camera_bg
    at 0, forward_terrain_bg at 1, chunk's splat bind group at 2, binds
    vertex/index buffers, draws indexed.

### 3.2 — Add terrain-forward state to `Renderer` (astraweave-render/src/renderer.rs)

Add (feature-gated on `terrain-splat-arrays`):

- **Field**: `terrain_forward: Option<TerrainForwardRenderer>`
- **Type** (either in-crate or new module):

  ```rust
  pub struct TerrainForwardRenderer {
      pub manager: TerrainMaterialManager,
      pub chunks: HashMap<u64, TerrainChunkGpu>,
  }
  pub struct TerrainChunkGpu {
      pub vertex_buffer: wgpu::Buffer,
      pub index_buffer: wgpu::Buffer,
      pub index_count: u32,
  }
  ```

- **Accessor methods**:
  - `pub fn terrain_forward(&self) -> Option<&TerrainForwardRenderer>`
  - `pub fn terrain_forward_mut(&mut self) -> Option<&mut TerrainForwardRenderer>`
  - `pub fn init_terrain_forward(&mut self) -> Result<()>` — creates the
    manager with default config, sets up forward pipeline.
  - `pub fn upload_terrain_chunk(&mut self, key: u64, vertices: &[TerrainSplatVertex],
    indices: &[u32], splat_0: &[u8], splat_1: &[u8], splat_dims: (u32, u32))`
  - `pub fn clear_terrain_chunks(&mut self)`
  - `pub fn set_terrain_materials(&mut self, gpu_material: &TerrainMaterialGpu,
    layers: &[LayerTextures<'_>]) -> Result<()>`

### 3.3 — Integrate terrain draws into `Renderer::draw_into`

Location: after the `for model in self.models.values() { ... }` loop
(approximately line 5647) and before the impostor/water/weather draws
(line 5651+).

```rust
// Phase 1 — forward-lit splat terrain draws (Option D).
#[cfg(feature = "terrain-splat-arrays")]
if let Some(ref mut tf) = self.terrain_forward {
    // Ensure the pipeline is built for the current color/depth format.
    tf.manager.ensure_forward_pipeline(
        &self.device,
        wgpu::TextureFormat::Rgba16Float,  // hdr_view format
        Some(self.depth.format),
    );
    // Sync camera + scene UBOs for this frame.
    tf.manager.update_forward_camera(
        &self.queue,
        self.cached_proj * self.cached_view,
        self.light_dir,
        // camera_pos from cached_view inverse, or use stored cam pos
        ...,
    );
    tf.manager.update_forward_scene(&self.queue, &TerrainSceneEnvGpu::from(&self.scene_env));
    // Draw each registered chunk.
    for (key, chunk_gpu) in &tf.chunks {
        let _ = tf.manager.draw_chunk_forward(
            &mut rp,
            *key,
            &chunk_gpu.vertex_buffer,
            &chunk_gpu.index_buffer,
            chunk_gpu.index_count,
        );
    }
}
```

Note: `rp` is the main render pass variable in `draw_into` (the block
beginning at line 5524). The terrain draws happen INSIDE the same pass,
sharing `hdr_view` + depth.

### 3.4 — Editor adapter changes (tools/aw_editor/src/viewport/engine_adapter.rs)

- In `upload_terrain_chunks`, when feature is on:
  - Skip the legacy `convert_terrain_chunk` + cluster-planning + `add_model_with_bounds`
    calls.
  - Build `TerrainSplatVertex` buffers from editor's 80-byte `TerrainVertex`
    (drop biome_weights/material_ids; keep pos/normal/uv). Use the square
    surface prefix — not the skirt.
  - Upload via `renderer.upload_terrain_chunk(key, verts, indices, splat_0,
    splat_1, dims)`.
- On project open, load 8 biome material texture sets and call
  `renderer.set_terrain_materials(&gpu, &layer_textures)`. For Phase 1,
  procedural color-swatch textures (one per biome) are acceptable — the plan
  permits this. File-based loading (parsing `assets/materials/{biome}/materials.toml`)
  is a Phase 3 follow-up if the TOML format is deemed the right project
  shape.
- Remove the `terrain_splat: Option<EditorTerrainSplat>` field OR make it
  a thin delegate to `renderer.terrain_forward()`. The cleaner path is to
  remove the field entirely; the wrapper type in `terrain_splat.rs` can stay
  on disk for tests. Document whichever choice is taken as an amendment to
  §9 (it's a divergence from Phase 1.B as originally committed).

### 3.5 — Loading 8 biome material textures

Options:

- **A (recommended for Phase 1 scope)**: generate 8 procedural color-swatch
  textures in Rust (e.g., `astraweave-render/src/terrain_biome_placeholder.rs`
  or inline). Each is a 256×256 RGBA8 buffer filled with a distinct hue.
  Their normal maps are a flat-blue default; ORM is a neutral mid-grey.
  This satisfies the plan's Phase 1 success criterion of "visible biome
  blending — may not look visually finished".
- **B**: Parse `assets/materials/{biome}/materials.toml` and load real
  textures. More code (TOML parsing + image decoding + fallback on missing
  files). Higher risk of hitting existing asset-loading bugs.

Option A is recommended. Store a constant array of 8 biome colors, keyed by
the `BiomeId` ordering in `TerrainVertex.biome_weights_0/1`
(Grassland=0, Desert=1, Forest=2, Mountain=3, Tundra=4, Swamp=5, Beach=6,
River=7).

---

## 4. Suggested commit splits for Phase 1.E

1. **1.E.1** — `TerrainMaterialManager` forward pipeline support (struct
   fields, BGLs, buffers, `new_forward_shader_src`, `ensure_forward_pipeline`).
   Standalone — verifiable via the existing shader_validation test plus
   a new unit test that constructs a manager + ensures the forward pipeline
   builds on a headless wgpu device.

2. **1.E.2** — Manager draw + upload methods (`update_forward_camera`,
   `update_forward_scene`, `set_chunk_splat_forward`, `draw_chunk_forward`).
   Verifiable via a new integration test pattern modeled on
   `astraweave-render/tests/terrain_splat_pipeline.rs` (which proves the
   deferred path works).

3. **1.E.3** — Renderer's `terrain_forward` field + accessors + draw_into
   integration. Verifiable manually by launching the editor and observing
   a non-crashing render. Biomes will be unblended (single color) at this
   point because no material textures have been uploaded yet.

4. **1.E.4** — Editor adapter: route chunks through `renderer.upload_terrain_chunk`
   instead of legacy `add_model_with_bounds`; generate and upload 8 biome
   placeholder textures at project open. Verifiable visually: biomes
   visible as distinct colors in the editor, blending at boundaries.

5. **1.E.5** — Remove or delegate the `terrain_splat` field from the
   adapter (cleanup from 1.B). Optional — can be merged into 1.F.

Each commit ends with the required three `cargo check` invocations passing.

---

## 5. Open risks / validation gates

- **Camera UBO byte layout**: the Camera struct in SHADER_SRC differs from
  the manager's existing 80-byte `CameraUniformsGpu`. Get this wrong and
  the shader reads `camera_pos` from `light_dir`'s memory slot. Write a
  unit test that asserts the new `CameraForwardGpu` type is 96 bytes and
  aligns to 16.

- **Scene env byte layout**: Phase 1.D's shader declares
  `TerrainSceneEnv { fog_color: vec3<f32>, fog_density: f32, fog_start: f32,
  fog_end: f32, _pad0: vec2<f32>, ambient_color: vec3<f32>, ambient_intensity: f32,
  sun_color: vec3<f32>, sun_intensity: f32 }` — cross-check against SHADER_SRC's
  `SceneEnv` at `renderer.rs:86-100`. Note SHADER_SRC's also has `tint_color`,
  `tint_alpha`, `blend_factor`, `_pad1x/y/z`. The forward shader could read
  those, but Phase 1 doesn't need tint/blend — decide whether to declare
  them (easier to stay in sync with SHADER_SRC) or omit (smaller UBO, more
  drift risk).

- **Render pass lifetime / borrow juggling**: the manager's
  `draw_chunk_forward` borrows `&self` and the render pass borrows the
  encoder. Inside `draw_into`, `self.terrain_forward` is borrowed mutably
  (to update camera/scene UBOs). The draw loop needs careful structuring:
  finish all `&mut self.terrain_forward` state updates BEFORE opening the
  pass, then use `&self.terrain_forward` for the draw. Alternatively,
  split the manager's method into an `&mut self` update pass and an
  `&self` draw pass — already the case in the existing code.

- **Non-terrain regression check**: after 1.E.3 lands, verify entity
  rendering, scatter, sky, shadows, post-processing all look identical.
  Open the editor with a terrain-free scene, then with a terrain scene;
  both should render without artifacts.

- **Feature-off compile**: every sub-commit validates `cargo check
  -p astraweave-render --no-default-features --features "postfx,textures"`
  and the aw_editor variant. Any `#[cfg(feature = "terrain-splat-arrays")]`
  gate that slips would break this — catch immediately.

---

## 6. Plan-status update for §7

When Phase 1.E.1 lands, update §7:

```
Sub-steps complete:
- ...existing entries...
- 1.E.1 (commit `<hash>`) — forward pipeline registered in TerrainMaterialManager.
```

Continue appending sub-step entries as they land. When 1.E.4 lands and the
editor renders biome-blended terrain successfully, and 1.E.5 cleans up
1.B's now-redundant field, Phase 1.F closes the phase with "COMPLETE
<date>, commit <final hash>".

---

*End of handoff.*
