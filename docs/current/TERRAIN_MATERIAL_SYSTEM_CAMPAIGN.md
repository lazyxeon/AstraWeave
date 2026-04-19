# Terrain Material System — Path C Campaign

**Status**: PLAN — not yet started.
**Scope**: Implementation of AAA-parity terrain material rendering in AstraWeave, comprising splat-map biome blending + per-vertex 4-way material override + user-selectable blend modes, sample budgets, material count tiers, splat resolution, and normal blend modes.
**Author**: Plan drafted from design session 2026-04-19 between Andrew and Claude. Code references accurate as of 2026-04-19; verify before execution.
**Prior work**: Three audits that established the current state — `docs/audits/editor_viewport_render_divergence_2026-04-19.md`, `docs/audits/tonemap_double_application_investigation_2026-04-19.md`, `docs/audits/terrain_material_flow_investigation_2026-04-19.md`.
**Outcome on completion**: Veilweaver and any future AstraWeave-based project can render terrain with per-fragment material blending driven by both biome splat textures and per-vertex authored material IDs, with five project-level settings controlling the system's behavior.

---

## 0. How to use this document

This plan is the authoritative design reference for the terrain material system campaign. It exists alongside three execution prompts — one per phase — which will be written iteratively as each phase becomes ready to execute. The execution prompts reference this document for design decisions; this document does not contain execution steps, only the architecture and rationale.

### Anti-pattern this plan explicitly prevents

The Fix 27 Unified Pipeline Campaign (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`) shipped Phases 0–1 and stopped, while `ARCHITECTURE_MAP.md` was updated as if the whole campaign had shipped. Documentation drifted ahead of code. A subsequent audit (`docs/audits/editor_viewport_render_divergence_2026-04-19.md`) was required to reconcile them. This campaign must not repeat that failure.

**Discipline imposed:**
1. Every phase's completion commit must update section 7 of this document (the phase status block) to mark the phase complete, with the commit hash of completion.
2. No phase is "complete" until both the plumbing tests pass AND the status update commit has landed.
3. The "PLAN — not yet started" header at the top of this document must be updated as phases land: "Phase 1 complete, Phases 2–3 not yet started" → "Phases 1–2 complete, Phase 3 in progress" → "Campaign complete (date)."
4. Any discovered need to deviate from this plan during execution must be recorded in section 9 (Deviations log) with rationale, before the deviation is executed.

---

## 1. Design summary

### 1.1 The problem being solved

Terrain renders as monochromatic-per-cluster because the editor's rich per-vertex material authoring (80-byte `TerrainVertex` with 8 biome weights + 4 material IDs + 4 material weights) is aggregated into per-cluster histograms, argmax-reduced to a single material choice, and uploaded to GPU as plain `MeshVertex` data. Each cluster renders with one texture set and one tint colour. The splat-map infrastructure (`TerrainMaterialManager`, `pbr_terrain.wgsl`, `EditorTerrainSplat`) exists but is triply dormant: feature-gated off, compiled out, and with no production call sites.

### 1.2 The target

AAA-parity terrain rendering comparable to Enshrouded and Crimson Desert, achieved by reaching the shader with the data the editor already authors, and blending it per-fragment according to user-controlled settings.

### 1.3 The five settings

All settings live in the project file (not editor preferences), travel with projects opened by other developers, and are surfaced in a first-open wizard for new projects.

**Setting 1: Material blend mode.** Runtime uniform. Controls how per-vertex authored material overrides combine with splat-based biome materials at a single fragment.
- **Direct:** Per-vertex material fully overrides biome where authored.
- **Hybrid (default):** Per-vertex material mostly wins, with subtle biome influence so painted areas feel integrated with their surroundings.
- **Contextual:** Per-vertex material blends with biome, same brush stroke looks different in different biomes.
- **Advanced slider:** continuous 0.0–1.0 control exposing the underlying blend factor.

**Setting 2: Material sample budget.** Runtime uniform. Caps how many simultaneous material contributions blend at a single fragment.
- **Standard (default):** 4 biome slots + 4 material slots = 8 total texture-set samples per fragment.
- **Extended:** Up to 12 total samples per fragment. Measurably more GPU cost; recommended only when visibly needed.
- Default: Standard.

**Setting 3: Material count tier.** Compile-time variant. Caps total distinct material types the project supports.
- **Compact:** 8 materials.
- **Standard (default):** 16 materials.
- **Extended:** 22 materials.
- Default: Standard. Changing requires shader recompile; the editor must handle this gracefully (likely a project reload prompt).

**Setting 4: Splat resolution.** Runtime. Controls how fine splat-map details can be authored.
- **Coarse (preset):** 0.5 m/pixel.
- **Standard (preset, default):** 0.2 m/pixel.
- **High (preset):** 0.1 m/pixel.
- **Advanced slider:** continuous 0.05–1.0 m/pixel.
- VRAM warning shown above 0.1 m/pixel (resolution finer than 0.1 m/pixel).
- Splat streaming is always on (implementation detail, not a setting).

**Setting 5: Normal blend mode.** Runtime uniform. Controls how normal maps from blended materials combine.
- **Reoriented (default):** Reoriented Normal Mapping. AAA-standard, preserves detail at blend zones.
- **Whiteout:** Slightly cheaper, nearly identical.
- **Naive (stylized):** Direct weighted average. Produces washed-out seams. Warning text required in UI. Only for intentionally soft art directions.

### 1.4 The three-phase breakdown

**Phase 1: Activate the splat-map pipeline.** Turn on `terrain-splat-arrays` by default in editor builds. Wire `EditorTerrainSplat` construction and per-chunk splat upload into `engine_adapter::upload_terrain_chunks`. Drive the terrain draw calls through `TerrainMaterialManager`'s pipeline instead of the generic PBR pipeline. Ship 8-biome blended terrain rendering.

**Phase 2: Extend the pipeline to carry per-vertex material data.** Add a new vertex layout (`TerrainMaterialVertex`, extending `TerrainSplatVertex`) with `material_ids[4]` and `material_weights[4]` attributes. Update the shader to consume them and produce per-vertex 4-way material blending. Not user-facing until Phase 3 combines it with Phase 1.

**Phase 3: Combine the systems and ship settings.** Implement all five settings, the first-open wizard, and the import conflict dialog. Combine the splat-based biome layer and the per-vertex material layer per the blend mode. Ship AAA-parity terrain.

Each phase's plumbing must be correct and tested in isolation. Visual aesthetic validation is a Phase 3 gate only; earlier phases may not look visually finished and that is expected.

---

## 2. Technical architecture

### 2.1 Data flow at the end state

```
Editor authoring (CPU)
  │  - Per-vertex: position, normal, tangent, UV, biome_weights[8], material_ids[4], material_weights[4]
  │  - Per-chunk: authored in astraweave-terrain + brush-painted in editor
  │
  ▼
Splat builder (CPU, runs per chunk or per brush edit)
  │  - Input: per-vertex biome_weights
  │  - Output: RGBA8 splat textures (splat_0, splat_1) at project splat-resolution
  │
  ▼
Chunk upload to GPU (engine_adapter::upload_terrain_chunks)
  │  - Vertex buffer: TerrainMaterialVertex layout (pos, normal, tangent, UV, material_ids[4], material_weights[4])
  │  - Splat textures: uploaded via TerrainMaterialManager
  │  - Material texture arrays: loaded on project open, shared across chunks
  │
  ▼
Terrain draw (GPU, pbr_terrain.wgsl or successor)
  │  - Vertex shader: standard transform, pass material_ids / material_weights to fragment
  │  - Fragment shader:
  │      1. Sample splat textures at fragment's UV → biome blend weights
  │      2. Read per-vertex material_ids / material_weights (interpolated)
  │      3. Per blend mode: compute final per-material weights (capped at sample budget)
  │      4. Sample albedo/normal/MR for each contributing material
  │      5. Blend normals per normal blend mode
  │      6. Blend colors per standard weighted average
  │      7. Apply PBR lighting (existing)
  │      8. Return linear HDR
  │
  ▼
Engine HDR target (Rgba16Float) → editor tonemap → display
```

### 2.2 Vertex layout specification

The current `TerrainSplatVertex` at `astraweave-render/src/terrain_material_manager.rs:132-151` has:
- `@location(0) position: vec3<f32>` (12 B)
- `@location(1) normal: vec3<f32>` (12 B)
- `@location(2) uv: vec2<f32>` (8 B)
- Total: 32 B

The new `TerrainMaterialVertex` extends this to include tangent and per-vertex material data:
- `@location(0) position: vec3<f32>` (12 B)
- `@location(1) normal: vec3<f32>` (12 B)
- `@location(2) tangent: vec4<f32>` (16 B) — w component is bitangent sign
- `@location(3) uv: vec2<f32>` (8 B)
- `@location(4) material_ids: vec4<f32>` (16 B) — floats not uints, interpolated across triangle
- `@location(5) material_weights: vec4<f32>` (16 B) — weights for material_ids
- Total: 80 B

Rationale for `material_ids` as `vec4<f32>` rather than `vec4<u32>`: the shader needs to interpolate these across a triangle. Integer attributes don't interpolate in hardware; floats do. Each fragment receives an interpolated value and rounds to the nearest integer to index into the material array. This matches how per-vertex material IDs are handled in published AAA terrain systems (Unreal Landscape, Frostbite).

### 2.3 Shader architecture

One WGSL shader source file: `astraweave-render/shaders/pbr_terrain.wgsl` (existing, will be significantly extended).

Three compile-time variants driven by `#ifdef`-style preprocessor defines (or WGSL's equivalent, which in our codebase is done by string substitution before shader compilation):
- `TERRAIN_MATERIAL_COUNT_COMPACT` → 8 material layers declared
- `TERRAIN_MATERIAL_COUNT_STANDARD` → 16 material layers declared
- `TERRAIN_MATERIAL_COUNT_EXTENDED` → 22 material layers declared

Runtime uniforms driven by project settings:
- `u.blend_mode: u32` — 0=Direct, 1=Hybrid, 2=Contextual, 3=Advanced (use u.blend_factor)
- `u.blend_factor: f32` — 0.0–1.0, used only when blend_mode == 3
- `u.sample_budget: u32` — 0=Standard (8 total), 1=Extended (12 total)
- `u.normal_blend_mode: u32` — 0=Reoriented, 1=Whiteout, 2=Naive

### 2.4 Blend formulas (for reference)

All formulas operate per-fragment on normalized weights. Given biome weights `b[0..8]` from splat textures and material weights `m[0..4]` from per-vertex interpolation:

**Direct mode:** If sum of m[] >= 0.01, use m[] only. Otherwise use b[] only.
- `final = mix(biome_result, material_result, step(0.01, sum(m)))`
- In words: if any per-vertex material is authored, it fully replaces the biome; otherwise biome shows through.

**Hybrid mode (default):** Per-vertex material wins where present, biome influence softens it.
- Let `material_strength = saturate(sum(m))`.
- `biome_contribution = (1.0 - material_strength) + material_strength * 0.15`
- `material_contribution = material_strength * 0.85`
- Numeric example: fragment with m = [0.9, 0, 0, 0] and b showing 100% grass:
  - material_strength = 0.9
  - biome_contribution = 0.1 + 0.9 * 0.15 = 0.235
  - material_contribution = 0.9 * 0.85 = 0.765
  - Final: ~77% of first material + ~24% of grass biome.

**Contextual mode:** Per-vertex material and biome blend as equals, weighted by their own strength.
- `biome_contribution = 1.0 - 0.5 * saturate(sum(m))`
- `material_contribution = 0.5 * saturate(sum(m))`
- Numeric example: same as above:
  - biome_contribution = 1.0 - 0.5 * 0.9 = 0.55
  - material_contribution = 0.5 * 0.9 = 0.45
  - Final: ~45% first material + ~55% grass biome. Material blends more into the biome.

**Advanced mode:** Use `u.blend_factor` as the biome/material split directly.
- `biome_contribution = 1.0 - u.blend_factor * saturate(sum(m))`
- `material_contribution = u.blend_factor * saturate(sum(m))`
- `u.blend_factor = 0.85` reproduces Hybrid; `u.blend_factor = 0.5` reproduces Contextual; `u.blend_factor = 1.0` reproduces Direct.

### 2.5 Sample budget enforcement

The shader reads all declared material layers but only samples textures for the top N contributors (by weight) within the current sample budget. This is done with a runtime-constant loop count.

**Standard budget (8 total):** Top 4 biomes + top 4 materials.
**Extended budget (12 total):** Top 6 biomes + top 6 materials (or up to 8 biomes + 4 materials if biome weights dominate; implementation detail).

Ties are broken by slot order (lower index wins) for determinism.

### 2.6 Normal blend formulas

For two normal vectors `n1` and `n2` blended by weights `w1` and `w2` (with `w1 + w2 = 1`):

**Naive:** `normalize(n1 * w1 + n2 * w2)`. Produces washed-out seams.

**Whiteout:** Add tangent-space perturbations without averaging the base direction.
- Approximate formula: `normalize(vec3(n1.xy * w1 + n2.xy * w2, n1.z * n2.z))`
- In words: blend the XY perturbations, multiply the Z components (which are both ~1.0).

**Reoriented Normal Mapping (RNM):** The AAA standard. Pick the higher-weighted normal as base, use the other as a tangent-space delta.
- Full formula is non-trivial; the standard reference is Barre-Brisebois & Hill's 2012 GDC talk. Agent implementing should look up the canonical implementation.
- Key property: preserves surface detail of both inputs without averaging them out.

For N > 2 contributing normals, the blending is iterative: accumulate pairwise using the selected formula.

### 2.7 Splat resolution → VRAM implications

Splat textures are per-chunk, streaming-gated. Memory cost per chunk for a 256m × 256m chunk:

| Resolution (m/pixel) | Texture size | Bytes per chunk (2× RGBA8) |
|---|---|---:|
| 0.5 | 512 × 512 | ~1 MB |
| 0.2 | 1280 × 1280 | ~6.5 MB |
| 0.1 | 2560 × 2560 | ~26 MB |
| 0.05 | 5120 × 5120 | ~105 MB |

With splat streaming (typically ~15 chunks loaded at once), the realistic budget at Standard resolution is ~100 MB for splat maps across the visible area. At High (0.1), ~400 MB. The warning threshold at 0.1 m/pixel reflects where non-streamed resident cost would exceed 2 GB for a 100-chunk terrain.

### 2.8 Settings storage

All settings stored in the project file (format TBD — match existing AstraWeave project file format). Setting schema:

```
TerrainMaterialSettings {
  blend_mode: Direct | Hybrid | Contextual | Advanced,
  blend_factor: f32 (0.0–1.0, only meaningful when blend_mode == Advanced),
  sample_budget: Standard | Extended,
  material_count: Compact | Standard | Extended,
  splat_resolution_m_per_pixel: f32 (0.05–1.0),
  normal_blend_mode: Reoriented | Whiteout | Naive,
}
```

Changing `material_count` requires a shader rebuild and a project reload. The editor must prompt the user for this explicitly; it is not a live-reload setting.

---

## 3. Phase 1 — Splat pipeline activation

### 3.1 Goal

Enable the existing splat-map infrastructure end-to-end so that terrain renders with 8-biome blending driven by splat textures, visible in the editor at default features. No per-vertex material support yet. No settings infrastructure yet — hardcode sensible defaults in code, make them settings in Phase 3.

### 3.2 Scope

**In scope:**
- Enable `terrain-splat-arrays` feature by default in `tools/aw_editor/Cargo.toml` and `astraweave-render/Cargo.toml`. Verify `TerrainMaterialManager` module compiles.
- Construct `EditorTerrainSplat` inside `EngineRenderAdapter` (field on the adapter struct).
- In `upload_terrain_chunks`:
  - Call the existing `terrain_splat_builder::build_chunk_splat_maps` per chunk.
  - Upload the resulting splat textures via `EditorTerrainSplat`.
  - Register the chunk's terrain mesh with `TerrainMaterialManager` rather than via the generic `add_model_with_bounds` path.
- Drive terrain rendering through `TerrainMaterialManager::draw_chunk` (or its equivalent pipeline entry point) instead of the generic PBR shader.
- Load the 8 biome material texture sets (albedo / normal / MR for each of grassland, desert, forest, mountain, tundra, swamp, beach, river) into the material texture arrays at project open. Hardcode the texture paths to `assets/materials/{biome_name}/` (match existing convention).
- Hardcode splat resolution to Standard (0.2 m/pixel) for Phase 1. Settings come in Phase 3.

**Out of scope for Phase 1:**
- Per-vertex material IDs / weights reaching the shader (Phase 2).
- Any of the five settings (Phase 3).
- First-open wizard (Phase 3).
- Import conflict dialog (Phase 3).
- Normal blend mode choice — use Reoriented as a hardcoded default (Phase 3 exposes the setting).
- Material count tier — compile with Standard (16 layers) hardcoded. Only 8 are used in Phase 1 (biomes); the other 8 slots sit unused until Phase 2.
- Brush authoring updates — the brush continues to author what it already authors; the splat builder reads the biome weights that are already being produced.

### 3.3 Existing code to reference

- `tools/aw_editor/src/viewport/terrain_splat.rs` — the existing wrapper with its `#[cfg(feature = "terrain-splat-arrays")]` / `#[cfg(not(...))]` split. Call sites need adding; the wrapper itself largely exists.
- `tools/aw_editor/src/viewport/terrain_splat_builder.rs` — builds splat textures from `biome_weights_0/1`. Currently called only by tests; needs production integration.
- `astraweave-render/src/terrain_material_manager.rs` — the GPU-side manager. Entire module is `#![cfg(feature = "terrain-splat-arrays")]`.
- `astraweave-render/shaders/pbr_terrain.wgsl` and `pbr_terrain_vs.wgsl` — the existing shaders. Phase 1 uses them roughly as-is; Phase 2 extends the vertex shader.
- `tools/aw_editor/src/viewport/engine_adapter.rs:1329` — `upload_terrain_chunks` entry point. Around lines 1371–1375 is the chunk-accept loop where splat upload needs to integrate.
- `tools/aw_editor/src/viewport/engine_adapter.rs:1665-1729` — `convert_terrain_chunk`. This function must continue to produce a valid `MeshVertex` buffer for any code path that still uses it, but Phase 1 adds a parallel path that also builds splat data and routes rendering through the manager.

**All line numbers above are accurate as of 2026-04-19. Verify before use.**

### 3.4 Success criteria

- `cargo check -p astraweave-render --all-features` passes.
- `cargo check -p aw_editor` passes with `terrain-splat-arrays` enabled by default.
- Editor opens, loads a test project, renders terrain. The terrain rendering uses the `pbr_terrain.wgsl` pipeline (confirmed by shader pipeline introspection or debug logging).
- Splat textures are generated per chunk and uploaded to the GPU. Visible by either:
  - A debug view in the editor that displays the splat texture as a colored overlay, OR
  - Confirmed via GPU inspection tools (RenderDoc) that the splat texture binding is non-empty.
- Terrain renders with 8-biome blending visible (regions authored as different biomes show different material appearance). This may not look visually "finished" — that is expected. Plumbing is the gate, not aesthetic.
- Existing non-terrain rendering is not regressed: entity PBR, scatter, sky, shadows, etc. all continue to work.
- No panics, no validation layer errors from wgpu.

### 3.5 Reversibility

- Revert feature flag default changes to restore prior build behavior.
- The code paths that Phase 1 adds should be gated such that if `terrain-splat-arrays` is disabled, the old `convert_terrain_chunk` → `create_mesh_from_full_arrays` path is taken. This preserves a working fallback.
- Git revert of the Phase 1 commit should cleanly restore pre-Phase-1 behavior.

### 3.6 Testing expectations

- Automated: cargo build + cargo test for both crates with feature on and off. No new integration tests required; the existing tests should pass.
- Visual smoke: Andrew opens the editor on a test project and confirms terrain renders without crashes or visible corruption. Does not have to look "good" — has to render and be stable.

---

## 4. Phase 2 — Per-vertex material data extension

### 4.1 Goal

Extend the terrain vertex layout and shader to carry and consume per-vertex `material_ids[4]` and `material_weights[4]`. Produce per-vertex 4-way material blending visible in the shader's fragment output. Still hardcoded defaults — no settings infrastructure yet.

### 4.2 Scope

**In scope:**
- Introduce `TerrainMaterialVertex` type in `astraweave-render/src/terrain_material_manager.rs` (or similar location) with the 80-byte layout specified in §2.2.
- Update `TerrainMaterialManager::ensure_pipeline` to build the pipeline against the new vertex layout.
- Update `pbr_terrain_vs.wgsl` to accept the new attributes and pass them to the fragment shader as interpolated values.
- Update `pbr_terrain.wgsl` fragment shader to:
  - Read interpolated `material_ids` (round to nearest integer per-slot).
  - Read interpolated `material_weights`.
  - Sample the appropriate material layers from the material texture arrays.
  - Blend them using a hardcoded Hybrid formula (Phase 3 makes this a setting).
  - Combine the per-vertex material blend with the existing splat-biome blend using the hardcoded Hybrid formula.
  - Blend normals using hardcoded RNM.
- Extend `upload_terrain_chunks` to upload the new vertex layout instead of the old `TerrainSplatVertex` layout. Route the editor's authored `viewport::TerrainVertex` fields directly into the new GPU vertex data.
- Load material texture arrays to support up to 16 materials (Standard tier, hardcoded for Phase 2).

**Out of scope for Phase 2:**
- All settings (Phase 3).
- UI for selecting blend mode, normal blend mode, etc.
- Variable material count tiers (compile with Standard = 16 hardcoded).
- Variable splat resolution (use Phase 1's hardcoded 0.2 m/pixel).
- Brush UX changes — brush continues to author per-vertex material IDs/weights as it already does.
- First-open wizard.
- Conflict dialog.

### 4.3 Existing code to reference

- `tools/aw_editor/src/viewport/types.rs:17-29` — the editor's `TerrainVertex` that Phase 2's pipeline consumes. All 80 bytes of it.
- `tools/aw_editor/src/viewport/engine_adapter.rs:113-188` — `TerrainSurfaceSummary`. Phase 2 may still compute this for non-rendering purposes (metadata, stats), but it is no longer the primary path to the GPU. Do not remove it without confirming no non-rendering consumers exist.
- `astraweave-render/shaders/pbr_terrain.wgsl` — the fragment shader to extend.
- `astraweave-render/shaders/pbr_terrain_vs.wgsl` — the vertex shader to extend.
- `astraweave-render/src/terrain_material_manager.rs:132-151` — `TerrainSplatVertex::LAYOUT`. Add `TerrainMaterialVertex::LAYOUT` alongside it (or replace).

### 4.4 Success criteria

- `cargo check` passes on both crates with the default feature set.
- Shader compilation succeeds for `pbr_terrain.wgsl` and `pbr_terrain_vs.wgsl`.
- Editor renders terrain with visible per-vertex material variation. A brush-painted dirt path on a grass hillside should be visible as dirt in the painted area, not as uniform grass.
- Normal blending at material transitions looks detailed, not washed-out (visual confirmation of RNM vs. Naive).
- No regressions in entity or scatter rendering.

### 4.5 Reversibility

- Git revert of the Phase 2 commit should restore Phase 1's splat-only rendering. The Phase 2 vertex layout change is the biggest risk here; ensure the old layout path still exists as a fallback or that the revert is clean.
- If revert is needed and the vertex layout must be rolled back, the splat builder does not need to change — it consumes the editor's `TerrainVertex` which is stable. Only the GPU-side plumbing reverts.

### 4.6 Testing expectations

- Automated: cargo build + cargo test. Include at least one new unit test that validates the `TerrainMaterialVertex` byte layout matches the expected 80 B.
- Visual smoke: Andrew confirms per-vertex material variation is visible. A smoke test scene with a known brush-painted pattern should render that pattern recognizably.

---

## 5. Phase 3 — Settings, wizard, conflict dialog, and final polish

### 5.1 Goal

Ship all five settings, the first-open wizard, the import conflict dialog, and the combined Phase 1 + Phase 2 system as a coherent user-facing feature. Final aesthetic validation happens here.

### 5.2 Scope

**In scope:**
- **Setting 1 (Material blend mode):** Add `blend_mode` and `blend_factor` uniforms. Update fragment shader's combined-blend logic to switch on `blend_mode`. Implement the four formulas (Direct, Hybrid, Contextual, Advanced). UI: three preset buttons + expandable advanced slider.
- **Setting 2 (Material sample budget):** Add `sample_budget` uniform. Update shader's sample loop to respect the budget. UI: two-option selector.
- **Setting 3 (Material count tier):** Introduce compile-time variants (Compact / Standard / Extended). Shader string substitution to produce three shader variants. UI: three-option selector with "requires project reload" note.
- **Setting 4 (Splat resolution):** Plumb the m/pixel value through to the splat builder and texture allocation. Rebuild splat textures for all chunks on change. UI: three presets + advanced slider with VRAM warning above 0.1 m/pixel.
- **Setting 5 (Normal blend mode):** Add `normal_blend_mode` uniform. Update shader's normal blend function to switch. UI: three-option selector with warning text on Naive.
- **Project file schema:** Add the `TerrainMaterialSettings` struct to the project file format. Load on project open, write on change.
- **First-open wizard:** On new project creation, or first open of a project that has no `TerrainMaterialSettings`, show a wizard that surfaces the five settings with explanatory text for each. Default values populate. User confirms or adjusts.
- **Import conflict dialog:** When importing a terrain whose material count exceeds the project's tier, show the Auto-drop / Change tier / Cancel dialog described in design session. Manual Remap is logged as future-phase feature (see §8).
- **Documentation update:** Update `docs/current/ARCHITECTURE_MAP.md` in the same commit as Phase 3 completion, to describe the new terrain material system accurately.

**Out of scope for Phase 3:**
- Manual Remap import workflow (future-phase, §8).
- Runtime editing of blend mode via keyboard shortcut (may add later).
- Preset "packs" for common art directions (stylized / photoreal / etc.).

### 5.3 Success criteria

- All five settings present in the project file and editable in the editor.
- First-open wizard appears for new projects and surfaces all five settings with explanatory text.
- Changing any runtime setting (1, 2, 4, 5) updates terrain rendering live without project reload.
- Changing material count tier (Setting 3) prompts for project reload and, on reload, produces correctly compiled shaders for the new tier.
- Splat resolution changes rebuild splat textures and update rendering correctly. VRAM warning appears at the correct threshold.
- Import conflict dialog appears for too-many-materials scenarios and its three options all work.
- Final terrain rendering at defaults (Hybrid / Standard / Standard / 0.2 m/pixel / Reoriented) looks visually comparable to Enshrouded / Crimson Desert in terms of material variation, blend quality, and detail preservation. Andrew's aesthetic judgment is the final gate.
- `ARCHITECTURE_MAP.md` is updated to describe the new system, including removal or correction of §4.5's outdated description of `to_engine_vertex()` as an active code path.
- Section 7 of this document (phase status) is updated to "Campaign complete" with the completion commit hash.

### 5.4 Reversibility

- Each setting can be individually disabled / hardcoded to its default if a bug is found; the other settings should continue to work.
- Full Phase 3 revert rolls back to Phase 2's hardcoded-defaults state, which is a working terrain system, just without user controls.

### 5.5 Testing expectations

- Automated: unit tests for setting serialization / deserialization in the project file. Unit tests for conflict dialog logic (material count comparison). Shader compilation tests for all three material count tier variants.
- Visual: final AAA-parity aesthetic validation. Andrew loads a representative Veilweaver scene and confirms the terrain meets the project's quality bar. This is a judgment call; if it doesn't look right, remediation work before marking Phase 3 complete.

---

## 6. Out of scope for this entire campaign

These items are intentionally not part of Path C and are logged here to prevent scope creep during execution.

- **Manual Remap import workflow** (guided material-by-material reassignment when a terrain exceeds the project's material tier). Future-phase, see §8.
- **Runtime blend mode switching via keyboard shortcut.** Convenience feature, not required for shipping.
- **Art-direction preset packs.** "Stylized," "Photoreal," etc. bundled settings. May come in a future quality-of-life pass.
- **Per-material parameters beyond texture sets.** Things like per-material roughness scaling, color tinting, triplanar mapping. Each is legitimate but none are required for AAA parity at the base level.
- **Terrain decals.** Surface-projected graphics on top of the terrain (footprints, blood splatters, etc.). Separate system.
- **Vertex-painted color on terrain.** Distinct from material painting; would be vertex color modulating the final lit output. Not in this campaign.
- **Shader permutations beyond material count tier.** No quality tiers for rendering fidelity (low / medium / high / epic). The three settings handle quality implicitly.
- **Changes to existing non-terrain rendering.** Entities, scatter, sky, shadows, post-processing — none of these are touched by this campaign. If a change is needed in any of them during execution, it is out of scope and must be logged as a deviation (§9) or deferred to a follow-on task.

---

## 7. Phase status

This section must be updated in the same commit that completes each phase.

**Phase 1 — Splat pipeline activation:** NOT STARTED
**Phase 2 — Per-vertex material data extension:** NOT STARTED
**Phase 3 — Settings, wizard, conflict dialog, final polish:** NOT STARTED

Format for completion update: `Phase N — <title>: COMPLETE <YYYY-MM-DD>, commit <hash>`

---

## 8. Logged future-phase features

- **Manual Remap import workflow.** Guided UX for reassigning materials when importing a terrain with more materials than the project supports. Opens a per-material walkthrough with preview, undo, and commit. Estimated: ~1 week of editor UX work. Build after Path C ships.

---

## 9. Deviations log

This section records any design decisions made during execution that deviate from this plan. Every deviation must be recorded here before or in the same commit as the deviation itself.

Format for entries:
```
### <YYYY-MM-DD>, Phase <N>, commit <hash>
**Deviation:** <short description>
**Rationale:** <why>
**Impact:** <what parts of later phases or other systems are affected>
```

### 2026-04-19, Phase 1, commit TBD — PROPOSED (execution paused pending decision)

**Deviation:** Phase 1 as specified (§3.2) is not executable without violating its own scope boundaries. The plan claims the splat infrastructure is "dormant but ready to activate"; investigation during Phase 1.B reveals a fundamental rendering-architecture mismatch that neither Phase 1's feature-flag flip nor its call-site-wiring mandate can resolve on its own.

**Rationale (finding):** `pbr_terrain.wgsl` + `pbr_terrain_vs.wgsl` at `astraweave-render/shaders/` produce a **deferred-style G-buffer**: three unlit color attachments (albedo, world-space-packed normal, ORM) with no lighting, shadows, IBL, sun, or fog (confirmed at `pbr_terrain.wgsl:165-169, 313-317`). The shader's `FragmentOutput` struct has three `@location(N)` outputs and writes no lit color.

The editor's rendering is **forward**: `astraweave_render::Renderer::draw_into` (at `astraweave-render/src/renderer.rs:5184+`) writes a single lit HDR color to `self.hdr_view` (a `Rgba16Float`-format target) inside its main pass. The lit color is computed by the main PBR shader `SHADER_SRC` (renderer.rs:18-445), which does sun lighting, shadow cascades, IBL, fog, cloud shadows, and tonemap prep in one fragment stage.

`TerrainMaterialManager::ensure_pipeline` (terrain_material_manager.rs:598-681) explicitly builds a render pipeline with three `ColorTargetState` entries — one per fragment output — confirmed in the only existing test (`astraweave-render/tests/terrain_splat_pipeline.rs:249-276`), which has to set up a render pass with three color attachments + depth just to issue a single `draw_chunk`. There is no forward-lit variant of the splat pipeline in the codebase.

The forward renderer has no intercept point that would let the editor inject draw calls sharing its camera, lighting, shadow, and IBL bindings mid-pass. `Renderer::render_with` (renderer.rs:5928+) takes a post-scene callback, not a pre-scene or mid-scene one, and even if it were restructured, the splat shader would still be writing to three attachments that the main pass's single-target `hdr_view` cannot satisfy.

Consequently, Phase 1.D ("Route terrain rendering through `TerrainMaterialManager::draw_chunk`") **cannot** be executed while respecting Phase 1's constraints:
- "No shader changes" (§3.2 Out of scope) rules out rewriting `pbr_terrain.wgsl` to do forward lighting.
- "Do not change non-terrain rendering" (execution prompt constraint 2) rules out restructuring `Renderer::draw_into` into a deferred pipeline with a lighting/composite pass, which would be the textbook consumer of the splat G-buffer output.
- "No speculative refactoring" rules out adding a new forward-lit terrain pipeline inside `TerrainMaterialManager` alongside the existing deferred one.

The three prior audits (`docs/audits/terrain_material_flow_investigation_2026-04-19.md` especially §4 and §6.3) documented that `EditorTerrainSplat` had zero production call sites and that the splat path was triply dormant, but did not surface the deeper issue: the dormant pipeline was designed for a different rendering architecture than the one actually shipped. Plan §1.1/1.3 assumed the splat code was "infrastructure waiting to be turned on." In reality it was a G-buffer-style terrain pass written in anticipation of a deferred renderer that never landed — making it unconditionally unusable from the current forward renderer without substantial new work.

**Recommended alternatives** (for Andrew to decide; stop and escalate per §0 discipline rule and execution-prompt explicit guidance):

1. **Option A — Re-scope Phase 1 to "extend the forward renderer with a terrain-splat forward variant".** Write a new forward-lit fragment shader that samples the 8-layer splat texture arrays and combines them with the engine's sun / shadow / IBL / fog bindings, producing a single lit HDR output that plugs into the existing `hdr_view` target. This treats the existing `pbr_terrain.wgsl` as reference material for the splat-sampling logic but does not reuse it verbatim. Additions to `astraweave-render/src/renderer.rs` to register a terrain-splat draw pass inside `draw_into`. Phase 1 becomes a ~1–2 week piece of rendering work instead of a plumbing flip. Fallback (feature-off) path preserved: the old `convert_terrain_chunk` / `add_model_with_bounds` path remains.

2. **Option B — Defer the deferred-renderer decision to a larger campaign.** Declare Phase 1 blocked, land only §1.A (feature-flag flip, already committed) as a no-op compilation enabler, and add a new campaign "Deferred renderer with terrain-splat G-buffer consumer" that becomes a prerequisite for Phase 1.D–1.E. This preserves `ARCHITECTURE_MAP.md` alignment with the current state while unblocking the campaign's Phase 2/3 dependency graph (per-vertex material authoring is useful in a forward-lit custom shader too, so Phase 2 could become "build the forward-lit splat + per-vertex shader" independently).

3. **Option C — Narrow Phase 1 to splat-upload plumbing only, ship no rendering change.** Wire `EditorTerrainSplat` construction, `build_chunk_splat_maps` invocation, and `set_chunk_splat` upload into `engine_adapter::upload_terrain_chunks` behind the default-on feature flag. Leave rendering entirely on the legacy path. The splat textures and material arrays sit uploaded and untouched on the GPU. This is the **only** subset of Phase 1.B–1.D that can run under the current constraints. It delivers zero user-visible change; Phase 2 or the subsequent fix campaign does the rendering wiring. Value: keeps §7 phase-status honest about what shipped, lets Phase 2's planning assume splat uploads are already integrated. Risk: commits GPU-memory overhead for uploads that nothing reads, and violates plan §3.4's explicit success criterion "Editor opens, loads a test project, renders terrain. The terrain rendering uses the `pbr_terrain.wgsl` pipeline."

**Impact on later phases:**
- Phase 2 (§4) inherits the same shader-output mismatch. It cannot extend `pbr_terrain_vs.wgsl` with per-vertex material attributes and expect them to reach a lit fragment output through the current pipeline. Option A above would need to be executed before or as part of Phase 2.
- Phase 3 (§5) is unaffected except that it must settle on whichever of the above options the earlier phases implemented before its settings can be meaningful.
- `ARCHITECTURE_MAP.md` correction in §5.2 (Phase 3 completion) gains a new item: describe the forward-vs-deferred decision and the final shape of the terrain shader.

**Execution state at time of this entry:** Phase 1.A committed (commit `1233537fe` — feature flag flipped to default on, verified with `cargo check -p astraweave-render --all-features`, `cargo check -p aw_editor`, and `cargo check -p astraweave-render --no-default-features --features "postfx,textures"`). Phase 1.B not started (no `EditorTerrainSplat` field added to `EngineRenderAdapter`). All subsequent Phase 1 work **paused** pending Andrew's selection of Option A, B, or C (or a substitute).

**Why this was caught here and not at plan-write time:** the three prior audits established *that* the splat pipeline was dormant and catalogued *which* call sites were missing, but did not inspect the shader's fragment-output signature or the pipeline's `ColorTargetState` count. A full read of `pbr_terrain.wgsl:165-169` plus `terrain_material_manager.rs:640-660` during Phase 1.B's adapter-field planning step surfaced the three-output architecture and its incompatibility with the forward renderer. Suggest updating the next campaign's plan-drafting protocol to require a shader-signature cross-check against the target render-pass's attachment layout before declaring infrastructure "dormant but ready."

---

## 10. References

- `docs/audits/editor_viewport_render_divergence_2026-04-19.md` — established the current state of the editor viewport code relative to `astraweave-render`.
- `docs/audits/tonemap_double_application_investigation_2026-04-19.md` — ruled out double-tonemapping as a cause of terrain visual issues.
- `docs/audits/terrain_material_flow_investigation_2026-04-19.md` — established the precise data-loss point and the dormant splat infrastructure that this campaign activates.
- `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` — prior campaign document whose half-shipped state motivated the discipline requirements in §0.
- `docs/current/ARCHITECTURE_MAP.md` §4.5 and §6 — current terrain-related architectural description; must be updated as part of Phase 3 completion.

---

*End of plan.*