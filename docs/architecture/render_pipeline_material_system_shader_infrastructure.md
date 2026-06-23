# Architecture Trace: Render Pipeline + Material System + Shader Infrastructure

## Metadata

| Field | Value |
|---|---|
| **System name** | Render Pipeline + Material System + Shader Infrastructure |
| **Primary crates** | `astraweave-render` (123 Rust files / ~78K LoC + 71 WGSL files), `astraweave-materials` (single-file material-graph crate), `tools/aw_editor/src/viewport/` (editor-side renderer + engine adapter) |
| **Document version** | 1.3 |
| **Last verified against commit** | `67c9de7e1` (full trace); water rows W.2a + W.2b.2 + W.2c.2 weave-deformation (2026-06-22) |
| **Last verified date** | 2026-05-10 (full trace) |
| **Status** | **ACTIVE WORKZONE** — Editor Multi-Tool Architecture Campaign Sub-phase 3 (Mediator Brush) is in flight as of campaign-doc commit `e3d07f366` (2026-05-08, Round-8-Closure). Fix 27 Unified Pipeline Campaign is structurally complete (per CLAUDE.md) but deeper editor↔runtime unification continues. Treat this trace as a **navigational map**; per-subsystem detailed traces are follow-up work. |
| **Owner notes** | This trace covers an unusually large system (~78K LoC source + 71 WGSL files + editor viewport). Per the template's "One last thing" rule on scale, this doc is intentionally structured as a **subsystem map + load-bearing-aggregator detail**, not an exhaustive per-file trace. Sub-systems like Lumen GI, MegaLights, Nanite, Atmosphere, GPU Particles, Volumetric Fog, IBL, and TAA each warrant their own dedicated trace if and when they enter focused work. The doc covers terrain materials by reference to `docs/architecture/terrain_materials.md`. |

---

## 1. Executive Summary

**What this system does:**
Rasterizes a 3D scene through a wgpu-25-based pipeline: vertex skinning → shadow cascades → clustered-forward main pass → optional deferred path → post-processing chain (bloom/TAA/SSAO/tonemap) → HDR composite → swapchain. Supports PBR materials (Disney BRDF with optional clearcoat/anisotropy/SSS/sheen/transmission), MegaLights GPU light culling, Lumen-style GI, terrain splatting (32-layer canonical material library), GPU particles, atmosphere/sky, water, weather, decals, and impostor/Nanite LOD. The editor viewport in `tools/aw_editor/src/viewport/` shares this pipeline through an adapter so the editor and runtime render paths converge on a single canonical renderer.

**Why it exists:**
Provides a unified, AAA-parity rendering pipeline shared by runtime examples, the editor viewport, and out-of-tree embedders. Single source of truth for PBR, shadows, IBL, and post-processing — eliminating the prior dual-pipeline drift (FastPreview vs. EnginePBR) that motivated the Fix 27 Unified Pipeline Campaign (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`).

**Where it primarily lives:**
- `astraweave-render/src/renderer.rs` (7,809 LoC) — canonical `Renderer` aggregator; owns surface, device, queue, pipelines, bind groups, and `render()` / `draw_into(...)` entry points
- `astraweave-render/src/frame_graph.rs` (868 LoC) — DAG-based pass orchestration with topological ordering and resource lifetime/aliasing analysis
- `astraweave-render/src/graph.rs` + `graph_adapter.rs` — minimal render-graph scaffolding the frame_graph builds on
- `astraweave-render/src/material*.rs` (5 files, ~3,000 LoC) — material system: `MaterialManager`, `MaterialLibrary`, `MaterialGpuExtended`, bindless system, material loader
- `astraweave-render/src/shader_manager.rs` (391 LoC) + `shader_permutation.rs` + `pipeline_cache.rs` — shader infrastructure
- `astraweave-render/shaders/` (71 WGSL files, ~14K LoC) — shader programs
- `astraweave-materials/src/lib.rs` (single file) — declarative material-graph node enum (foundational, Phase 2)
- `tools/aw_editor/src/viewport/renderer.rs` + `engine_adapter.rs` — editor-side viewport that delegates scene rendering to `astraweave_render::Renderer` and adds editor overlays (grid, gizmos, physics debug, blueprint overlay)

**Status note (read first):**
1. **Active campaign work**: The Editor Multi-Tool Architecture Campaign Sub-phase 3 (Mediator Brush) is the highest-velocity work area. Eight diagnostic rounds (2026-05-04 → 2026-05-08) confirmed and elevated the §7.7 wrapped-component resource identity trap to a structural axiom characterizing the editor↔renderer boundary. Real-Fix.A, .B, .C have landed; .D pending Andrew-gate.
2. **Two render paths coexisted historically** — the FastPreview editor-only path and the EnginePBR path through `astraweave_render::Renderer`. Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` and CLAUDE.md, the structural unification phase is complete; the `RenderMode::FastPreview` enum variant remains at `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` as residue.
3. **The terrain material system has its own canonical trace** at `docs/architecture/terrain_materials.md`. Cross-referenced rather than duplicated here.
4. **Scale warning**: 123 source files + 71 WGSL files. This trace gives the navigational map and load-bearing detail; per-subsystem traces (Lumen GI, MegaLights, Nanite, Atmosphere, GPU Particles, Volumetric Fog, IBL, TAA, Vegetation, Water, Decals, Impostor/LOD) are follow-up work.

---

## 2. Authoritative Pipeline

The render system has multiple entry points and parallel sub-pipelines. The three load-bearing flows are:

### 2.1 Runtime per-frame render (`Renderer::render`)

```text
[Caller: example main loop, integration test, or windowed app]
    │
    │ renderer.update_camera(...) + renderer.update_instances(...) (or set_terrain_materials/etc.)
    │ renderer.render()
    ▼
[Stage R0: Surface acquire]
    file: astraweave-render/src/renderer.rs:4708-4720
    role: Pull SurfaceTexture from swapchain; handle Lost/OOM
    │
    ▼
[Stage R1: GPU profiler + staging ring begin_frame]
    file: astraweave-render/src/renderer.rs:4726-4732
    role: Allocate per-pass timestamp slots; advance per-frame transient allocator
    │
    ▼
[Stage R2: Clustered light bin]
    file: astraweave-render/src/clustered.rs (CPU bin: bin_lights_cpu),
          astraweave-render/src/clustered_forward.rs (GPU bin pipeline)
    role: Cluster lights into froxel grid; produce offset/index buffers
    │
    ▼
[Stage R3: Shadow cascades (CSM)]
    files: astraweave-render/src/shadow_csm.rs,
           astraweave-render/src/shadow_quality.rs,
           astraweave-render/shaders/shadow_sampling.wgsl
    role: Render directional-light depth into a texture_2d_array (one cascade per layer);
          per-cascade uniform buffers prevent the queue.write_buffer race (renderer.rs:752-757)
    │
    ▼
[Stage R4: Main scene pass (forward+ clustered)]
    file: astraweave-render/src/renderer.rs (main pipeline, inline WGSL at SHADER_SRC line 18+)
    bind groups:
        0: camera UBO
        1: material UBO (legacy single-material; bindless system in material_bindless.rs is the modern path)
        2: shadow uniforms + texture_2d_array + comparison sampler
        3: PBR material textures (albedo / mr / normal)
    role: Rasterize geometry; sample shadow + cluster light list; evaluate Disney BRDF
    │
    ▼
[Stage R5: Post-FX chain]
    files: astraweave-render/src/hdr_pipeline.rs (orchestrator),
           astraweave-render/src/bloom.rs, gtao.rs, taa.rs, auto_exposure.rs,
           astraweave-render/src/advanced_post.rs (TAA / MotionBlur / DoF / ColorGrading)
    role: HDR → tonemap → LDR; bloom / SSAO / SSGI / SSR / DOF / motion blur where enabled
    │
    ▼
[Stage R6: Composite + present]
    file: astraweave-render/src/renderer.rs (post_pipeline + hdr_blit_pipeline at renderer.rs:725-731)
    role: Blit HDR to swapchain (or LDR for editor); submit + present
```

### 2.2 Editor draw into texture (`Renderer::draw_into`)

```text
[Caller: tools/aw_editor/src/viewport/engine_adapter.rs:render_to_texture]
    │
    │ renderer.draw_into(scene_target_view, depth_view, encoder)
    │   depth_view: Option<&wgpu::TextureView>  ← added 2026-05-07 Real-Fix.A (commit 0f569d212)
    ▼
[Same R1-R5 stages as Renderer::render, but writes to caller-provided target]
    file: astraweave-render/src/renderer.rs:5234-…
    role: Editor-side rendering; `depth_view` parameter lets the editor share its own depth target
          (resolves Sub-phase 3 Mediator Brush Mechanism 1 — wrong-texture / different-render-target
           §7.7 instance — per docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md Round-5-Closure)
```

### 2.3 Frame graph build (declarative DAG, parallel scaffolding)

```text
[FrameGraphConfig { width, height, shadow_resolution, cascade_count, feature toggles }]
    │
    ▼
[FrameGraphBuilder declares typed pass nodes]
    file: astraweave-render/src/frame_graph.rs
    role: Declare PassDecl with explicit resource inputs/outputs;
          cluster_bin / shadow / sky / main_scene / tonemap topology (frame_graph.rs:9-15)
    │
    │ RenderGraph::compile() → topological order + resource lifetimes + alias analysis
    ▼
[GraphContext records pass execution]
    file: astraweave-render/src/graph.rs, graph_adapter.rs
    role: Validates resource availability; current pass nodes delegate actual GPU work
          back to Renderer methods (frame_graph.rs:18-25, "Migration Status" comment block)
```

### 2.4 Material upload flows

```text
[A] TOML / RON authoring → material_loader (texture array build) → MaterialManager (GPU arrays + UBO)
    files: astraweave-render/src/material_loader.rs (1,113 LoC; feature-gated on "textures"),
           astraweave-render/src/material.rs:949 (MaterialManager)
    output: MaterialGpu storage buffer, texture arrays bound at group 3

[B] Canonical 32-layer terrain material library (Real-Fix.D 2026-05-08)
    files: astraweave-render/src/material_library.rs (MAX_TERRAIN_LAYERS = 32, NUM_SPLAT_MAPS = 8),
           astraweave-render/src/terrain_material.rs (TerrainLayerGpu, TerrainMaterialGpu),
           astraweave-render/src/terrain_material_manager.rs (forward path: draw_chunk_forward)
    output: Per-chunk splat textures + shared layer arrays. See docs/architecture/terrain_materials.md

[C] Bindless material system (alternative modern path)
    file: astraweave-render/src/material_bindless.rs
    output: BindlessMaterialSystem; GpuMaterialEntry indexed by material_id

[D] Extended material flags (clearcoat / anisotropy / SSS / sheen / transmission)
    file: astraweave-render/src/material_extended.rs
    flags: MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_ANISOTROPY, MATERIAL_FLAG_SHEEN,
           MATERIAL_FLAG_SUBSURFACE, MATERIAL_FLAG_TRANSMISSION (re-exported at lib.rs:222-226)
```

### 2.5 Shader pipeline / permutation infrastructure

```text
[ShaderManager registers shader files by ShaderKey (string)]
    file: astraweave-render/src/shader_manager.rs:74-…
    role: Path → content_hash → dirty flag; check_for_changes() rescans on a timer
    │
    │ Renderer recreates wgpu::ShaderModule + RenderPipeline when dirty
    ▼
[ShaderPermutation compiles BRDF lobe permutations]
    file: astraweave-render/src/shader_permutation.rs
    role: Compile-time enum of permutations for Disney BRDF lobes
    │
    ▼
[PipelineCache persists wgpu::PipelineCache to disk]
    file: astraweave-render/src/pipeline_cache.rs
    role: Vulkan/DX12 pipeline cache; eliminates cold-start shader-compile stalls;
          held by Renderer as _pipeline_cache_mgr to keep alive for Drop persistence (renderer.rs:701-703)
```

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Renderer** | The canonical aggregator: owns wgpu Device / Queue / Surface + all pipelines + all bind groups. Single Rust struct, ~7,800 LoC. | `astraweave-render/src/renderer.rs:691-…` |
| **`render()`** | Per-frame top-level entry: acquires swapchain texture and runs the full pipeline. | `renderer.rs:4708` |
| **`draw_into(view, depth_view, enc)`** | Editor entry: render into a caller-supplied texture view using a caller-supplied depth view (added 2026-05-07 per Round-5-Closure). | `renderer.rs:5234` |
| **Frame graph** | DAG-based declarative pass orchestration. Each `PassDecl` has explicit resource I/O; `RenderGraph::compile()` produces topological order + resource lifetime + alias analysis. Currently scaffolded — pass nodes delegate GPU recording back to `Renderer`. | `astraweave-render/src/frame_graph.rs`, `graph.rs`, `graph_adapter.rs` |
| **Clustered forward** | Lights binned into a 3D froxel grid; fragment shader samples the cluster offsets/indices buffer. | `clustered.rs`, `clustered_forward.rs`, `clustered_megalights.rs` |
| **MegaLights** | GPU-accelerated light culling via compute-shader prefix-sum and write-indices passes. | `clustered_megalights.rs`, `shaders/megalights/*.wgsl` |
| **CSM (Cascaded Shadow Maps)** | Directional-light shadow rendering with cascaded depth slices (currently 2 cascades in `App::new()` defaults). | `shadow_csm.rs`, `shadow_quality.rs`, `shaders/shadow_sampling.wgsl` |
| **Lumen GI** | Lumen-style global illumination orchestrator (distance-field AO + surface cache + final gather). | `lumen.rs`, `distance_field.rs`, `surface_cache.rs`, `final_gather.rs`, `shaders/lumen/*.wgsl` |
| **HDR pipeline** | The HDR → tonemap → LDR chain, with bloom / GTAO / SSGI / SSR / DOF / TAA / motion blur / color grading composed in sequence. | `hdr_pipeline.rs` (orchestrator), `bloom.rs`, `gtao.rs`, `taa.rs`, `auto_exposure.rs`, `advanced_post.rs` |
| **Material library** | The canonical 32-layer terrain material registry. UI-side `MATERIAL_NAMES: [&str; 22]` was earlier capacity; Real-Fix.D bumped to 32. | `material_library.rs` (`MAX_TERRAIN_LAYERS = 32`, `NUM_SPLAT_MAPS = 8`) |
| **MaterialGpu / MaterialGpuExtended** | The GPU representation of a material entry (texture indices, factors, flags). Extended adds clearcoat / anisotropy / SSS / sheen / transmission. | `material.rs:6-43`, `material_extended.rs` |
| **MaterialManager** | Owns the GPU material arrays + UBO uploaded from TOML/RON. | `material.rs:949-…` (`pub struct MaterialManager`) |
| **BindlessMaterialSystem** | Alternative material binding path using bindless texture arrays + a `GpuMaterialEntry` storage buffer. | `material_bindless.rs` |
| **ShaderManager** | Hash-tracked shader source registry with dirty-flag for hot-reload. Does NOT own GPU resources. | `shader_manager.rs:74-…` |
| **PipelineCache** | Disk-backed wgpu pipeline cache (Vulkan/DX12), bound to Renderer's lifetime via the `_pipeline_cache_mgr` field. | `pipeline_cache.rs`, held at `renderer.rs:701-703` |
| **Frame graph node** | A `RenderNode` implementation that owns resource declarations and a record-callback. | `graph.rs`, `frame_graph.rs:78-…` |
| **Resource generation** | Monotonic counter bumped on resize/resource invalidation; drives `CachedBindGroup` rebuild decisions. | `renderer.rs:708-710`, `bind_group_cache.rs` |
| **Staging ring** | Per-frame ring buffer for transient GPU uniforms / storage allocations. | `staging_ring.rs`, used at `renderer.rs:707, 4732, 5246` |
| **GpuProfiler** | Per-pass GPU timestamp profiler. None when `TIMESTAMP_QUERY` is unsupported. | `gpu_profiler.rs`, used at `renderer.rs:704-705` |
| **RenderMode** | Editor-side enum for switching between `EnginePBR` (default, full PBR through `Renderer`) and `FastPreview` (legacy cube placeholders). Residue from Fix 27 dual-pipeline era. | `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` |
| **EngineRenderAdapter** | Editor-side adapter that wraps `Renderer` and bridges editor scene data into the render pipeline. | `tools/aw_editor/src/viewport/engine_adapter.rs` |
| **ViewportRenderer** | Editor-side multi-pass coordinator. Delegates scene rendering to `EngineRenderAdapter` (which delegates to `Renderer`); owns editor overlays (grid, entity, gizmo, physics debug). | `tools/aw_editor/src/viewport/renderer.rs:60-…` |
| **§7.7 wrapped-component resource identity trap** | The canonical architectural anti-pattern surfaced by the Editor Multi-Tool Architecture Campaign Sub-phase 3. When component A wraps component B and both manage resources of the same logical type, reads from A's resource don't reflect writes to B's resource. Per the campaign-doc Status header at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`, this is now a **structural axiom** with four-layer evidence: depth-target (Round 5), mesh-data (Round 6), texture-data attribute-set (Round 7), UI-vs-renderer capacity boundary (Round 8). | Editor↔renderer boundary across `engine_adapter.rs`, `renderer.rs`, `terrain_material_manager.rs`, `terrain_splat_builder.rs` |

### Terms to NOT confuse

- **`Renderer::render` vs `Renderer::draw_into`**: `render()` (`renderer.rs:4708`) acquires the surface texture and presents to the swapchain; `draw_into(view, depth_view, enc)` (`renderer.rs:5234`) writes to a caller-supplied texture view without presenting and accepts an optional depth view. The editor uses `draw_into`; runtime examples use `render`.
- **`MaterialManager` vs `MaterialLibrary` vs `BindlessMaterialSystem`**: Three different material binding paths coexist. `MaterialManager` (`material.rs:949`) is the TOML-driven legacy path. `MaterialLibrary` (`material_library.rs`) is the canonical 32-layer **terrain** material registry. `BindlessMaterialSystem` (`material_bindless.rs`) is the bindless modern path. Section 6 documents the coexistence.
- **`types::Material` vs `material_library::Material`**: Re-exported at `lib.rs:147` (`types::Material`) but the inner `material_library::Material` is **intentionally NOT re-exported** at the crate root to avoid the collision (see `lib.rs:251-253` for the explicit no-export comment).
- **Frame graph vs render pipeline**: The frame graph is declarative DAG scaffolding (`frame_graph.rs`); the actual GPU work still flows through `Renderer` methods. Per `frame_graph.rs:18-25`, "full delegation is designed for incremental adoption" — the frame graph today validates topology but does not yet drive command recording.
- **Editor render path vs runtime render path**: Both call into `astraweave_render::Renderer` (post Fix 27 structural completion). The remaining divergence lives in the editor's `EngineRenderAdapter` (`tools/aw_editor/src/viewport/engine_adapter.rs`) which manages editor-specific scene data and the FastPreview residue. See §6.
- **Shadow uniforms (single buffer) vs per-cascade buffers**: The renderer uses **per-cascade uniform buffers** (`renderer.rs:752-757`) explicitly to avoid `queue.write_buffer` race conditions where all writes resolve before the command buffer executes, causing both shadow passes to read the same (last-written) cascade matrix. This is a deliberate non-obvious choice — do not "simplify" to a single shadow uniform buffer.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `wgpu` 25.x | Device / Queue / Surface APIs | GPU command submission | `Renderer::new_from_device` is the canonical constructor; CLAUDE.md ARCHITECTURE_REFERENCE.md lists key methods |
| `astraweave-math` | SIMD vec/mat/quat ops | Transform math (CPU side) | Auto-used by `glam` re-exports |
| `astraweave-asset` + `astraweave-asset-pipeline` | Asset index, texture/mesh loading | Material TOMLs, glTF, OBJ | `material_loader.rs` consumes asset paths; gated on `textures` feature |
| `astraweave-cinematics` | Cinematic camera frames | Camera transforms | `renderer.rs:15` `use astraweave_cinematics as awc;` |
| `astraweave-materials` | `Node` material-graph enum | Authored material graph data | `astraweave-materials/src/lib.rs:8-19` — currently used as foundation, not yet wired into runtime material instances. `renderer.rs:16` `use astraweave_materials::MaterialPackage;` |
| `astraweave-terrain` | `WorldGenerator`, `TerrainChunk`, `Heightmap` | Heightmap geometry + biome assignment | Consumed by `astraweave-render/src/terrain.rs` (legacy single-`biome_id` path) and the canonical `terrain_material_manager` (32-layer path, see `docs/architecture/terrain_materials.md`) |
| `astraweave-scene` | Scene partition data | Scene graph + streaming | Used by streaming-aware rendering paths |
| ~~`astraweave-fluids`~~ | — | — | **Row corrected F.1 (2026-06-11): the previous entry ("SPH water particle data → water.rs") was aspirational.** `astraweave-render` has no Cargo dependency on and zero imports from `astraweave-fluids`; `water.rs` is a self-contained Gerstner-wave renderer. SPH render integration is future Fluids-Integration campaign work (seam mapped in `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` §Seam 4). |
| `astraweave-ecs` | `Entity`, `World`, components | Scene data source | Editor and example mains pull entity data and feed instance buffers |
| `tools/aw_asset_cli` | Cooked asset blobs | Texture / mesh runtime assets | Build-time pipeline (cross-references in CLAUDE.md note this is a `tools/` dep) |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/hello_companion`, `unified_showcase`, `ecs_ai_showcase`, `weaving_playground`, `biome_weather_demo`, `phi3_demo`, ~20 example crates | `Renderer::render()` + `Renderer::update_camera/update_instances/etc.` | Final frame on swapchain | Examples are the primary in-tree runtime consumers |
| `tools/aw_editor` | `Renderer::draw_into(view, depth_view, enc)` via `EngineRenderAdapter::render_to_texture` | HDR scene + LDR composite for egui display | The editor is the most demanding in-tree consumer and the active campaign target |
| `astraweave-render/src/bin/aw_impostor_bake.rs` | `impostor_bake` module | Pre-baked impostor atlases (offline) | Feature-gated on `impostor-bake` |
| `astraweave-stress-test`, benchmarks | `Renderer` + headless surface | Performance measurement | Stress-test crates rely on the public Renderer API |
| Out-of-tree embedders via `astraweave-sdk` | None directly — SDK wraps the legacy World, not the renderer | n/a | The SDK does not currently expose render APIs; renderer is in-process only |

### Bidirectional / Coupled

- **Editor ViewportRenderer ↔ Renderer**: The editor passes its own depth view into `Renderer::draw_into` (added 2026-05-07 Real-Fix.A `0f569d212`) so editor overlay passes (grid, gizmos, physics debug, blueprint overlay) and engine terrain depth-write into the same depth target. This is the canonical fix shape for §7.7 wrapped-component resource identity trap at the depth-target layer. Source: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-5-Closure.
- **Terrain material upload coupling**: The editor's brush emits `TerrainAction` events that flow into `engine_adapter::upload_terrain_chunks` (initial path) **or** `engine_adapter::update_terrain_chunk` (incremental path). Real-Fix.B (`eaaa53433`, 2026-05-07) extracted `upload_or_update_terrain_chunk_forward` so both paths route to the same `Renderer::terrain_forward.chunks` HashMap (mesh-data layer §7.7 instance per Round-6-Closure).
- **Shader hot-reload ↔ Pipeline rebuild**: `ShaderManager::check_for_changes()` flips dirty flags; `Renderer` is responsible for recreating `wgpu::ShaderModule` + `RenderPipeline` (`shader_manager.rs:69-73` documents this split).
- **`bind_group_cache` ↔ `Renderer.resource_generation`**: The monotonic generation counter at `renderer.rs:708-710` is bumped on resize / resource invalidation; `CachedBindGroup` rebuilds against the new generation (`bind_group_cache.rs`).

---

## 5. Active File Map

This map enumerates the load-bearing files. Per-file traces of every subsystem are out of scope here — see metadata note on follow-up work. Subsystems are grouped functionally.

### Core aggregator / orchestration

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/renderer.rs` | Canonical `Renderer` aggregator: device/queue/surface ownership + all pipelines + `render()` / `draw_into()` entry points | Active | 7,809 LoC. Single-largest source file in the crate. Inline WGSL `SHADER_SRC` at lines 18-… for the main forward+ pipeline |
| `astraweave-render/src/lib.rs` | Module declarations + `pub use` re-exports | Active | 279 LoC; ~30 sub-modules declared, large public-API re-export surface (lines 139-279) |
| `astraweave-render/src/frame_graph.rs` | Frame render graph: DAG builder + pass nodes for cluster_bin / shadow / sky / main_scene / tonemap | Active (scaffolded; not yet command-driving per `frame_graph.rs:18-25`) | 868 LoC. Topology validation works; pass nodes delegate GPU recording back to `Renderer` |
| `astraweave-render/src/graph.rs`, `graph_adapter.rs` | Minimal render-graph scaffolding (`RenderGraph`, `RenderNode`, `PassDecl`, `GraphContext`) | Active | Foundation for `frame_graph.rs` |
| `astraweave-render/src/error.rs` | `RenderError`, `RenderResult` typed errors | Active | Re-exported at `lib.rs:143` |
| `astraweave-render/src/pipeline_cache.rs` | Disk-backed pipeline cache | Active | `_pipeline_cache_mgr` held by Renderer for Drop persistence |
| `astraweave-render/src/bind_group_cache.rs` | Generation-tracked bind-group cache | Active | Driven by `Renderer.resource_generation` |
| `astraweave-render/src/staging_ring.rs` | Per-frame transient GPU allocator | Active | `Renderer.staging_ring` field |
| `astraweave-render/src/gpu_memory.rs` | Memory budget tracker | Active | `GpuMemoryBudget`, `MemoryCategory` |
| `astraweave-render/src/gpu_profiler.rs` | Per-pass GPU timestamp profiler | Active (optional — None when `TIMESTAMP_QUERY` unsupported) | `Renderer.gpu_profiler` field |

### Material system

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/material.rs` | `MaterialGpu`, `MaterialLayerDesc`, `MaterialPackDesc`, `MaterialManager`, `ArrayLayout` | Active | 949 LoC; flags at lines 25-31 |
| `astraweave-render/src/material_library.rs` | Canonical 32-layer terrain material registry (`MAX_TERRAIN_LAYERS = 32`, `NUM_SPLAT_MAPS = 8`) | Active | Real-Fix.D 2026-05-08 — see `terrain_materials.md` |
| `astraweave-render/src/material_loader.rs` | TOML-driven texture array build pipeline (feature `textures`) | Active | 1,113 LoC; gated on `textures` feature flag |
| `astraweave-render/src/material_bindless.rs` | `BindlessMaterialSystem`, `GpuMaterialEntry` — bindless texture-array material path | Active | 585 LoC; alternative modern path to `MaterialManager` |
| `astraweave-render/src/material_extended.rs` | Extended PBR materials (clearcoat, anisotropy, SSS, sheen, transmission); flags at `material_extended.rs:lib.rs:222-226` re-export | Active | Phase PBR-E |
| `astraweave-render/src/disney_material.rs` | Disney principled BRDF evaluation + WGSL source export | Active | `evaluate_disney_brdf`, `BRDF_LUT_WGSL`, `DISNEY_BRDF_WGSL` exports |
| `astraweave-render/src/brdf_lut.rs` | Split-sum BRDF integration LUT for IBL (Phase 9) | Active | `BrdfLutPass`, `BrdfLutConfig` |
| `astraweave-render/src/parallax.rs` | Parallax Occlusion Mapping (steep ray-march + binary refinement) | Active | `PomConfig` |
| `astraweave-render/src/terrain_material.rs` | `TerrainLayerGpu`, `TerrainMaterialGpu`, `TerrainLayerDesc`, `TerrainMaterialDesc` — GPU schema | Active | 32-layer; see `docs/architecture/terrain_materials.md` |
| `astraweave-render/src/terrain_material_manager.rs` | 32-layer splat-array terrain pipeline (forward path) | Active (`#[cfg(feature = "terrain-splat-arrays")]` per `lib.rs:108-109, 260-264`) | Per `docs/architecture/terrain_materials.md` |
| `astraweave-render/src/biome_material.rs` | `BiomeMaterialSystem`, `BiomeMaterialConfig` | Active | Biome-aware material selection |
| `astraweave-render/src/biome_detector.rs` | `BiomeDetector` runtime biome classification | Active | Companion to `biome_material.rs` |
| `astraweave-render/src/biome_transition.rs` | `BiomeVisuals`, `TransitionConfig`, easing functions | Active | Cross-biome blend logic |
| `astraweave-render/src/biome_audio.rs` | Audio cues tied to biome transitions | Active | Cross-system into audio crate |
| `astraweave-materials/src/lib.rs` | Single-file declarative material-graph node enum (`Node`) | Active | Phase 2 foundation; `renderer.rs:16` uses `MaterialPackage` from this crate |

### Lighting

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/clustered.rs` | CPU cluster light binning (`bin_lights_cpu`, `ClusterDims`, `CpuLight`); WGSL placeholder `WGSL_CLUSTER_BIN` | Active | Loaded by `renderer.rs:11` |
| `astraweave-render/src/clustered_forward.rs` | Complete clustered forward rendering | Active | |
| `astraweave-render/src/clustered_megalights.rs` | MegaLights GPU light culling (Phase 1) | Active | Shaders at `shaders/megalights/*.wgsl` |
| `astraweave-render/src/shadow_csm.rs` | Cascaded Shadow Mapping (Phase 2) | Active | |
| `astraweave-render/src/shadow_point.rs` | Point/spot light shadow maps with priority selection | Active | |
| `astraweave-render/src/shadow_quality.rs` | PCSS, Poisson PCF, cascade stabilization, normal-offset bias | Active | |
| `astraweave-render/src/ltc_area_lights.rs` | LTC area lights (rectangular, disk, tube) — Heitz et al. 2016 | Active | `AreaLightManager`, `AreaLightType`, `GpuAreaLight` |
| `astraweave-render/src/ibl.rs` | Image-based lighting manager | Active | `IblManager`, `IblQuality`, `IblResources`, `SkyMode` |
| `astraweave-render/src/lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` | Lumen GI orchestrator (Phase 5) | Active | Distance-field AO + surface cache + multi-bounce final gather |
| `astraweave-render/src/gi/mod.rs`, `vxgi.rs`, `voxelization_pipeline.rs` | VXGI global illumination | Active | Alternative GI path to Lumen |
| `astraweave-render/src/ssgi.rs` | Screen-Space Global Illumination (temporal denoise) | Active | |
| `astraweave-render/src/ssr.rs` | Screen-Space Reflections (Hi-Z ray marching) | Active | |
| `astraweave-render/src/hiz_pyramid.rs` | Shared Hi-Z min-depth pyramid (SSR + SSGI acceleration) | Active | |
| `astraweave-render/src/gtao.rs` | Ground Truth Ambient Occlusion (visibility bitmask) | Active | |
| `astraweave-render/src/atmosphere.rs` | Bruneton physically-based atmosphere (Phase 8) | Active | Shaders at `shaders/atmosphere/*.wgsl` |
| `astraweave-render/src/god_rays.rs` | Screen-space god rays / crepuscular shafts (Phase 6) | Active | `sun_to_screen`, `GodRayPass` |
| `astraweave-render/src/volumetric_fog.rs` | Froxel-based volumetric fog + light scattering (Phase 6) | Active | 1,070 LoC |
| `astraweave-render/src/volumetric_clouds.rs` | Perlin-Worley volumetric cloud raymarching (Phase 3) | Active | |
| `astraweave-render/src/scene_environment.rs` | Scene-env UBO (fog/ambient/tint/sun); WGSL shared snippets `WGSL_FOG_FUNCTIONS`, `WGSL_SCENE_ENVIRONMENT` | Active | |

### Post-processing

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/hdr_pipeline.rs` | HDR pipeline orchestration: tonemap + color grading + post-FX chain | Active | 776 LoC |
| `astraweave-render/src/bloom.rs` | Physically-based bloom (13-tap downsample + tent upsample) | Active | |
| `astraweave-render/src/auto_exposure.rs` | Luminance-histogram auto-exposure with temporal adaptation | Active | Has subgroup-optimized variant in `subgroup_ops.rs` |
| `astraweave-render/src/taa.rs` | Temporal Anti-Aliasing (neighborhood clamping + RCAS sharpening) | Active | |
| `astraweave-render/src/temporal_upscale.rs` | TAA-U: reduced internal-res render with native-res resolve | Active | |
| `astraweave-render/src/velocity.rs` | Motion vector / velocity buffer | Active | Feeds TAA, motion blur, TSR |
| `astraweave-render/src/advanced_post.rs` | TAA + motion blur + DOF + color grading | Active (feature `advanced-post`) | `AdvancedPostFx`, `ColorGradingConfig`, `DofConfig`, `MotionBlurConfig` |
| `astraweave-render/src/post.rs` | Compile-only WGSL placeholders + tests; `WGSL_SSAO`, `WGSL_SSGI`, `WGSL_SSR` referenced from `renderer.rs:1-2` under `feature = "postfx"` | Active | |
| `astraweave-render/src/oit.rs` | Weighted Blended Order-Independent Transparency | Active | `OitBuffers`, `WboitRenderer` |
| `astraweave-render/src/transparency.rs` | Transparency depth sorting and render pass | Active | |
| `astraweave-render/src/msaa.rs` | MSAA anti-aliasing resources | Active | `MsaaMode`, `MsaaRenderTarget` |
| `astraweave-render/src/effects.rs` | `WeatherFx`, `WeatherKind` weather-effect overlay | Active | |
| `astraweave-render/src/overlay.rs` | Cutscene-fade / letterbox overlay | Active | |

### Geometry, culling, LOD

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/mesh.rs` | `CpuMesh`, `MeshVertex`, `MeshVertexLayout` | Active | |
| `astraweave-render/src/mesh_registry.rs` | `MeshRegistry`, `MeshHandle`, `MeshKey` | Active | Mesh lookup by stable key |
| `astraweave-render/src/mesh_gltf.rs`, `mesh_obj.rs` | glTF and OBJ loaders | Active (feature-gated on `assets` / `gltf-assets` / `obj-assets`) | |
| `astraweave-render/src/primitives.rs` | Primitive mesh generation | Active | |
| `astraweave-render/src/instancing.rs` | GPU instancing for draw call reduction | Active | |
| `astraweave-render/src/vertex_compression.rs` | Vertex compression (octahedral normals, half-float UVs); 37.5% memory reduction per `lib.rs:14` | Active | Week 5 Action 19 |
| `astraweave-render/src/lod_generator.rs` | LOD chain generation (quadric error metrics) | Active | Week 5 Action 19 |
| `astraweave-render/src/culling.rs` + `culling_node.rs` | GPU-driven frustum culling (Phase 2 Task 3) | Active | `CullingPipeline`, `IndirectDrawPipeline`, `FrustumPlanes`, `InstanceAABB` |
| `astraweave-render/src/nanite_gpu_culling.rs`, `nanite_render.rs`, `nanite_visibility.rs` | Nanite virtualized geometry | Active (feature `nanite`) | Per `lib.rs:131-137` |
| `astraweave-render/src/impostor_bake.rs`, `impostor_lod3.rs`, `impostor_pass.rs` | Phase 5.3 impostor atlas + LOD3 live-draw + reusable draw helper | Active (feature `impostor-bake`) | Per `lib.rs:159-164` |
| `astraweave-render/src/vegetation_gpu.rs` | GPU-instanced vegetation scatter + frustum cull | Active | |
| `astraweave-render/src/vegetation_interaction.rs` | Entity-proximity grass-bending stamp | Active | |
| `astraweave-render/src/vegetation_lod.rs` | Tree LOD chain with billboard/impostor support | Active | |
| `astraweave-render/src/grass_blade.rs` | Per-blade procedural grass geometry | Active | |
| `astraweave-render/src/stochastic_tiling.rs` | Hex-tile stochastic sampling (anti-tiling for terrain textures) | Active | |

### Animation

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/animation.rs` | `Skeleton`, `AnimationClip`, `AnimationState`, `Joint`, `JointPalette`, `JointMatrixGPU`, `MAX_JOINTS`, `compute_joint_matrices`, `skin_vertex_cpu` | Active | Phase 2 Task 5 |
| `astraweave-render/src/skinning_gpu.rs` | GPU skinning pipeline (`JointPaletteManager`, `JointPaletteHandle`, `SKINNING_GPU_SHADER`) | Active (feature `skinning-gpu`) | Phase 2 Task 5 Phase D |

### Particles + weather

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/gpu_particles.rs` | GPU compute-based particle system | Active (feature `gpu-particles`) | `GpuParticleSystem`, `GpuParticle`, `EmitterParams` |
| `astraweave-render/src/particle_forces.rs` | Enhanced particle simulation (forces, curves, emission shapes) | Active | Phase 7 |
| `astraweave-render/src/particle_render.rs` | Billboard particle render pipeline with blending | Active | Phase 7 |
| `astraweave-render/src/particle_sort.rs` | GPU bitonic sort for depth-ordered transparency | Active | Phase 7 |
| `astraweave-render/src/weather_system.rs` | Weather system orchestrator | Active | 1,083 LoC |
| `astraweave-render/src/weather_gpu.rs` | GPU weather particle emitter configs | Active | |
| `astraweave-render/src/rain_occlusion.rs` | GPU rain/weather particle occlusion via depth buffer | Active | |
| `astraweave-render/src/rain_splash.rs` | Rain-impact splash spawner | Active | |
| `astraweave-render/src/puddle_accumulation.rs` | Rain-driven puddle formation in terrain concavities | Active | |
| `astraweave-render/src/snow_accumulation.rs`, `snow_footprint.rs` | Per-chunk snow accumulation + entity footprint depression | Active | |
| `astraweave-render/src/environment.rs` | `SkyConfig`, `SkyRenderer`, `TimeOfDay`, `WeatherParticles`, `WeatherSystem`, `WeatherType` | Active | Re-exported via `lib.rs:140-142` |
| `astraweave-render/src/hdri_catalog.rs` | `HdriCatalog`, `HdriEntry`, `DayPeriod` | Active | |

### Decals, water, special effects

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/decals.rs` | Screen-space decal system | Active (feature `decals`) | `DecalAtlas`, `DecalSystem`, `DecalBlendMode`, `GpuDecal`, `DECAL_SHADER` |
| `astraweave-render/src/water.rs` | Chunked-LOD ocean surface (Gerstner waves) + refraction + weave deformation | Active | ~1020 LoC. **W.2a**: camera-distance discrete chunk grid, per-LOD instanced tiles + crack-hiding skirts; world Y from a `water_level` uniform (no baked mesh Y); `set_water_level` live (incl. editor knob). **W.2b.2**: water is now a SEPARATE post-opaque raster pass (`Renderer::run_water_pass`), no longer inside the shared main pass — it samples a scene-color snapshot for screen-space refraction and read-only scene depth for depth-delta shoreline foam; bind group is 4 entries (uniform + scene_color + scene_depth + sampler); `prepare_scene` wires the snapshot/depth each frame. **W.2c.2**: `WaterRenderer` now owns a **weave-deformation instance list** (`WeaveInstance`/`WeaveKind`, ceiling `MAX_WEAVE_INSTANCES = 8`) via `set_weave_instances`/`clear_weave_instances`, riding in `WaterUniforms` (now 512 B; the weave array is 16-aligned at offset 256). The instances are normalized, **position-agnostic analytical profiles** (location lives only in the instance: `world → local = rotate(world_xz − position, −orientation)/radius`), applied view-side in `water.wgsl` `vs_main` **after** the Gerstner sum (Q-cap untouched): `part`/`raise` as additive ±height, `freeze` as a **unified dual-channel** mask (suppresses waves + flattens normal + fragment material-state shift toward ice). Bounded to `±skirt_depth` (CPU intensity-clamp + shader net-clamp) and world-XZ sampled so it never re-exposes a LOD seam; **identity at zero instances** (runtime consumers set none). Editor-side test scaffolding (one part/raise/freeze) lives in `viewport/engine_adapter.rs::set_water_enabled` — the W.2c.3 plug-in seam for gameplay-emitted instances. Measured free on the 1660 Ti (~0.0005 ms/instance; full surface ~0.19–0.21 ms vs the 2.0 ms ceiling). See `W2C1_RECON.md`, `W2_DECISIONS.md` §B.1/§F, `W2A_EXECUTION_REPORT.md`, `W2B2_EXECUTION_REPORT.md`. |
| `astraweave-render/src/deferred.rs` | Deferred rendering pipeline (`DeferredRenderer`, `GBuffer`, `GBufferFormats`) | Active (feature `deferred`) | |
| `astraweave-render/src/debug_quad.rs`, `depth.rs` | Debug visualization + depth resources | Active | |
| `astraweave-render/src/clipmap_terrain.rs` | Clipmap-based terrain rendering | Active | |
| `astraweave-render/src/terrain.rs` | Legacy single-`biome_id` terrain rendering path | Transitional | One in-tree caller (`examples/weaving_playground`) — see `docs/architecture/terrain_materials.md` |
| `astraweave-render/src/terrain_gpu_bridge.rs` | Render-side `TerrainGpuAccelerator` impl (GPU noise + erosion bridge) | Active | |
| `astraweave-render/src/gpu_erosion.rs` | GPU compute SWE erosion | Active | |
| `astraweave-render/src/compute_noise.rs` | GPU compute noise generation (Perlin/fBM/Ridged/Billow/DomainWarped) | Active | |

### Shader infrastructure, streaming, low-level

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/shader_manager.rs` | Hot-reload shader source registry (`ShaderKey`, `ShaderManager`) | Active | 391 LoC; does NOT own GPU resources |
| `astraweave-render/src/shader_permutation.rs` | Compile-time permutation system for Disney BRDF lobes | Active | |
| `astraweave-render/src/subgroup_ops.rs` | Subgroup-optimized shader variants (auto-exposure, prefix sum, bitonic sort) | Active | `SubgroupCapabilities` |
| `astraweave-render/src/asset_index.rs` | `AssetIndex`, `MaterialSetEntry`, `TextureEntry`, `HdriRef as AssetHdriRef` | Active | |
| `astraweave-render/src/residency.rs` | GPU residency manager (`ResidencyManager`) | Active | Streaming-aware texture/mesh lifetime |
| `astraweave-render/src/texture_streaming.rs` | LRU-cache + priority-based texture streaming (`TextureStreamingManager`, `TextureStreamingStats`) | Active | |
| `astraweave-render/src/virtual_texture.rs` | Sparse virtual texturing (page cache, feedback, LRU) | Active | |
| `astraweave-render/src/texture.rs`, `types.rs` | Core texture / vertex / instance types | Active | `Instance`, `Material`, `Mesh`, `SkinnedVertex`, `Vertex` (re-exported at `lib.rs:147`) |
| `astraweave-render/src/camera.rs` | `Camera`, `CameraController` | Active | |

### Editor-side viewport + adapter

| File | Role | Status | Notes |
|---|---|---|---|
| `tools/aw_editor/src/viewport/renderer.rs` | `ViewportRenderer` multi-pass coordinator: grid → entities → gizmos → selection outline. HDR target Rgba16Float, LDR Rgba8UnormSrgb (lines 31-34) | Active | Post-Fix 27 simplified architecture per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` |
| `tools/aw_editor/src/viewport/engine_adapter.rs` | `EngineRenderAdapter` wrapping `astraweave_render::Renderer`; carries `RenderMode` enum (`EnginePBR`, `FastPreview`); `EditorQualityPreset` for shadow/post-FX quality | Active | `RenderMode::FastPreview` is Fix 27 residue (lines 25-28). `EditorQualityPreset` at lines 40-… |
| `tools/aw_editor/src/viewport/widget.rs` | `ViewportWidget`: egui input + viewport lifecycle | Active | |
| `tools/aw_editor/src/viewport/terrain_splat.rs` | `EditorTerrainSplat` (superseded by `Renderer::terrain_forward` per `engine_adapter.rs:6-10`) | Transitional (reference material) | See `docs/architecture/terrain_materials.md` |
| `tools/aw_editor/src/viewport/impostor_wiring.rs` | Editor-side impostor pipeline wiring | Active | |
| `tools/aw_editor/src/gizmo/rendering.rs` | Transform-handle gizmo rendering | Active | |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Transitional**: In active code paths but tagged for migration (e.g. `terrain.rs` legacy path; `terrain_splat.rs` editor reference material; `FastPreview` residue)
- **Feature-gated Active**: Compiles only when a Cargo feature is enabled; canonical when enabled

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Three material binding paths | `MaterialManager` (`material.rs`), `MaterialLibrary` (`material_library.rs`), `BindlessMaterialSystem` (`material_bindless.rs`) | Coexisting | Each serves a different purpose: TOML-driven legacy / canonical terrain / bindless modern. No single "the" material system |
| FastPreview vs EnginePBR | `RenderMode::FastPreview` enum variant at `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` | Transitional (Fix 27 residue) | Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`: the structural unification phase is complete and deeper unification continues; `FastPreview` mode persists for fallback / weak-GPU scenarios |
| Frame graph vs Renderer | `frame_graph.rs` declarative DAG vs `renderer.rs` imperative command recording | Coexisting (intentional, mid-migration) | `frame_graph.rs:18-25` documents "full delegation is designed for incremental adoption" |
| Lumen GI vs VXGI | `lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` vs `gi/vxgi.rs` + `gi/voxelization_pipeline.rs` | Coexisting | Two complete GI implementations; the choice is configurable per scene |
| Terrain rendering paths | Canonical 32-layer `terrain_material_manager.rs` vs legacy single-`biome_id` `terrain.rs` | Coexisting | See `docs/architecture/terrain_materials.md` for full forensic analysis |
| Particle render paths | `particle_render.rs` (Phase 7 modern) vs older inline particle handling in `effects.rs` and `weather_system.rs` | [NEEDS VERIFICATION on whether the older paths are still load-bearing or pure transitional] | |
| `types::Material` vs `material_library::Material` | Both exist as types named `Material` | Coexisting with explicit no-export guard | `lib.rs:251-253` documents the intentional non-re-export of `material_library::Material` to avoid the collision |
| Shadow-uniform single-buffer vs per-cascade buffers | Per-cascade buffers (`renderer.rs:752-757`) are the active path | Single-buffer historical | Per-cascade chosen explicitly to avoid `queue.write_buffer` race; don't "simplify" |

### Naming collisions

- **`Material`**: `astraweave_render::types::Material` (re-exported at `lib.rs:147`) vs `astraweave_render::material_library::Material` (intentionally NOT re-exported at root, per the comment block at `lib.rs:251-253`). Always namespace fully.
- **`TerrainVertex`**: `astraweave_render::TerrainVertex` (legacy single-`biome_id` format at `terrain.rs:18-23`) vs `tools/aw_editor/src/viewport/types.rs::TerrainVertex` (96-byte editor format with `material_ids[4]` / `material_weights[4]`). The `to_engine_vertex()` adapter collapses the rich format to the simple format. See `docs/architecture/terrain_materials.md`.
- **`Renderer`**: `astraweave_render::Renderer` (the canonical aggregator) vs `tools/aw_editor/src/viewport/renderer.rs::ViewportRenderer` (editor coordinator that wraps the canonical). Different responsibilities, sometimes confused in conversation.
- **"PBR shader"**: At least three distinct WGSL programs claim this name in shader files: `pbr.wgsl`, `pbr/disney_brdf.wgsl`, `pbr_terrain.wgsl` / `pbr_terrain_forward.wgsl` / `pbr_terrain_vs.wgsl`. Always cite the exact file when referring to "the PBR shader."
- **"Forward"**: `clustered_forward.rs` (cluster-bin forward), `pbr_terrain_forward.wgsl` (the 32-layer terrain forward path), `Renderer::terrain_forward` field (the live terrain-chunk map at `renderer.rs:5755`). Different scopes.

### Known cognitive traps

- **Trap (§7.7 wrapped-component resource identity)**: When the editor's `EngineRenderAdapter` wraps `Renderer`, the adapter's own resources (e.g. local depth target, terrain-cluster map, splat-build buffers, UI material library) are NOT the same as the underlying `Renderer`'s resources of the same logical role. Reads from the wrapper layer don't reflect writes to the engine layer (or vice versa). **Four confirmed instances in the Editor Multi-Tool Architecture Campaign Sub-phase 3**:
  - Round 5 (2026-05-07, Mechanism 1): depth-target layer — editor's `read_depth_at_pixel` sampled its own local depth texture that engine adapter's `render_to_texture` never wrote to. Fixed by Real-Fix.A `0f569d212` (`Option<&wgpu::TextureView>` depth_view parameter on `Renderer::draw_into`).
  - Round 6 (2026-05-07, Mechanism C): mesh-data layer — initial chunk upload routed to live `Renderer::terrain_forward.chunks`; incremental brush update routed to legacy `terrain_clusters` Vec that no rendering path reads. Fixed by Real-Fix.B `eaaa53433` (shared `upload_or_update_terrain_chunk_forward` helper).
  - Round 7 (2026-05-08, Mechanism H1): texture-data attribute-set layer — paint mutated `vertex.material_ids` + `vertex.material_weights`; `build_chunk_splat_maps` read only `vertex.biome_weights_0/1`. Fixed by Real-Fix.C `ded9a0457` (Option C unified `TerrainVertex` into single canonical `material_*` attribute set).
  - Round 8 (2026-05-08, Mechanism H8.1): UI/renderer capacity boundary — UI offers 22 materials (`MATERIAL_NAMES: [&str; 22]`); splat-build caps at 8; renderer texture array allocates 8 slots. Fix design pending Andrew-gate (Option D-2: canonical material library shared between UI + renderer).
  - Source of truth: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header (commit `e3d07f366`, 2026-05-08).
- **Trap**: Treating the frame graph as the command-driving path.
  **What's actually true**: `frame_graph.rs:18-25` explicitly states the frame graph "validates resource flow, and exercises the automatic topological ordering, resource lifetime, and aliasing analysis" but pass nodes "delegate actual GPU command recording to `Renderer` methods. Full delegation is designed for incremental adoption." `Renderer::render` is the actual command-recording path today.
- **Trap**: Adding new render features to `renderer.rs` as inline WGSL.
  **What's actually true**: `renderer.rs:18-…` contains a giant inline `SHADER_SRC` constant via `concat!` + `include_str!`. New shaders should go in `shaders/*.wgsl` and be loaded via `ShaderManager` + included into pipelines explicitly. The inline `SHADER_SRC` is historical.
- **Trap**: Adding a new "FastPreview" path or any editor-only render duplicate.
  **What's actually true**: Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, the campaign exists explicitly because dual pipelines diverged across 12 dimensions. CLAUDE.md Scope Discipline says "Never build a second implementation of a logical system that already exists (rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer)."
- **Trap**: Adding a new render module that duplicates an existing subsystem.
  **What's actually true**: The 123-file render crate has many similar-named modules (e.g. `bloom.rs` vs older bloom in `advanced_post.rs`; `gtao.rs` vs older SSAO in `post.rs`'s `WGSL_SSAO`). Before adding a new module, `rg 'struct <Name>\|trait <Name>'` workspace-wide per CLAUDE.md Scope Discipline.

---

## 7. Decision Log

### Decision: Single canonical `Renderer` struct as aggregator
- **Date:** [Reasoning not recovered from available sources — predates current trace]
- **Status:** Accepted (in active code)
- **Context:** `astraweave-render/src/renderer.rs:691-…` defines a single `Renderer` struct that owns wgpu device/queue/surface plus all pipelines. The choice was reaffirmed by the Fix 27 Unified Pipeline Campaign (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`) which eliminated the dual editor pipeline.
- **Decision:** All scene rendering goes through one `Renderer`. The editor wraps it via `EngineRenderAdapter` (`tools/aw_editor/src/viewport/engine_adapter.rs`); examples instantiate it directly.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** 7,800-line file with high blast radius for changes. Mitigated by extensive integration tests at `astraweave-render/tests/`. CLAUDE.md Scope Discipline explicitly forbids a second rendering path.

### Decision: Frame graph as declarative scaffolding with incremental adoption
- **Date:** Not recovered from commit messages; `frame_graph.rs:18-25` "Migration Status" comment documents the current state.
- **Status:** Accepted (transitional; full delegation deferred)
- **Context:** A pure imperative `Renderer::render` is brittle to refactor; a declarative frame graph (DAG of `PassDecl` with explicit resource I/O) enables topology validation, resource-lifetime analysis, and aliasing decisions.
- **Decision:** Build the frame graph in `frame_graph.rs` + `graph.rs` + `graph_adapter.rs`, but keep pass nodes delegating actual GPU command recording to `Renderer` methods. Migration to fully delegated nodes is incremental.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Two parallel orchestration models coexist (frame graph for topology + `Renderer::render` for commands). Adding a new pass currently requires touching both.

### Decision: Unify editor and runtime rendering through a single Renderer (Fix 27)
- **Date:** Plan dated 2026-04-05 (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`). Status per CLAUDE.md: "structural phase complete (astraweave-render non-optional, entity rendering through engine); deeper unification (shadow/IBL/post-processing alignment) ongoing."
- **Status:** Accepted (in active execution)
- **Context:** Per the campaign plan executive summary, the editor maintained two parallel rendering pipelines (FastPreview + EnginePBR) that "diverged across 12 dimensions (vertex format, materials, shadows, IBL, post-processing, etc.)". Each had its own PBR shader, CSM, BRDF LUT, IBL, glTF loader, tonemap, and texture pipeline.
- **Decision:** Eliminate the FastPreview path. Route ALL scene rendering through `astraweave_render::Renderer`. Preserve editor-specific overlays (grid, gizmo, physics debug, blueprint overlay) via a formal overlay injection protocol.
- **Alternatives considered:** Documented in the campaign plan (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`): FastPreview-as-permanent fallback was considered and rejected because the dual path created semantic drift across 12 dimensions.
- **Consequences:** ~4,000 lines of duplicated rendering code targeted for deletion. The `RenderMode::FastPreview` enum variant remains (`engine_adapter.rs:25-28`) as transitional residue. Visual parity between editor and runtime is now guaranteed by construction for the unified parts of the pipeline.

### Decision: Per-cascade shadow uniform buffers (not a single buffer)
- **Date:** Not directly recovered; comment block at `renderer.rs:752-757`.
- **Status:** Accepted (in active code)
- **Context:** Using a single shadow uniform buffer caused a `queue.write_buffer` race where all writes resolved before the command buffer executed, causing both shadow passes to read the same (last-written) cascade matrix.
- **Decision:** Use separate `wgpu::Buffer` + `wgpu::BindGroup` per cascade (`shadow_cascade_bufs: [wgpu::Buffer; 2]`, `shadow_cascade_bgs: [wgpu::BindGroup; 2]`).
- **Alternatives considered:** [Reasoning not recovered — the race-condition discovery is documented in the inline comment; alternative mitigations like explicit submission ordering weren't recorded.]
- **Consequences:** Two cascade-buffers worth of GPU memory vs one. Trivial cost; correct behavior.

### Decision: `Renderer::draw_into` accepts `Option<&wgpu::TextureView>` for depth (Real-Fix.A)
- **Date:** 2026-05-07, commit `0f569d212`. Andrew-gate Option (a) per `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-5-Closure.
- **Status:** Accepted (in active code)
- **Context:** Editor's `read_depth_at_pixel` sampled its own local depth texture; engine adapter's `render_to_texture` wrote to its own internal depth target (no depth attachment passed). Reads returned cleared/uninitialized 0.0 → near-plane unprojection → cursor_center ≈ cam_pos invariantly. The §7.7 wrapped-component resource identity trap at the depth-target layer.
- **Decision:** Cross-crate API addition — pass `Option<&wgpu::TextureView>` depth_view as a `Renderer::draw_into` parameter. Editor supplies its own depth view; engine writes terrain depth into it.
- **Alternatives considered:** Option (b) — expose engine adapter's internal depth target via accessor (works around the API shape; leaves overlay-vs-terrain depth-test bug latent). Option (c) — aw_editor-local depth pre-pass re-rendering terrain depth (redundant work; precision-mismatch risk). Documented in Round-5-Closure §12.
- **Consequences:** +33/-9 lines across 5 files. Resolves Mechanism 1 (depth-target §7.7). Aligns with CLAUDE.md v0.10.1 Edit 2 (no-second-implementation).

### Decision: Shared `upload_or_update_terrain_chunk_forward` helper (Real-Fix.B)
- **Date:** 2026-05-07, commit `eaaa53433`. Andrew-gate Option 1.
- **Status:** Accepted (in active code)
- **Context:** Initial terrain upload (engine adapter `upload_terrain_chunks`) routed correctly to `Renderer::terrain_forward.chunks`. Incremental brush update (engine adapter `update_terrain_chunk` → `rebuild_terrain_clusters_for_chunk` → legacy `terrain_clusters` Vec) routed to a path no rendering ever read. §7.7 at the mesh-data layer.
- **Decision:** Extract a shared helper called by both paths.
- **Alternatives considered:** Option 2 — inline copy of splat-build + filter logic into `update_terrain_chunk` (creates the exact second-implementation anti-pattern Edit 2 forbids; documented as rejected in the Round-6-Closure entry).
- **Consequences:** Mesh-data §7.7 instance fixed. Dead-code observation in Round-6-Closure: legacy cluster path is dead, cleanup deferred.

### Decision: Unified `TerrainVertex` material attribute set (Real-Fix.C)
- **Date:** 2026-05-08, commit `ded9a0457`. Andrew-gate Option C.
- **Status:** Accepted (in active code)
- **Context:** Paint brush mutated `vertex.material_ids` + `vertex.material_weights`; `build_chunk_splat_maps` read only `vertex.biome_weights_0/1`. §7.7 at the texture-data attribute-set layer (intra-component sub-variant).
- **Decision:** Option C — unify into a single canonical `material_*` attribute set. New `biome_weights_8_to_material_slots` helper preserves visual blending semantics.
- **Alternatives considered:** Option A (mirror writes — smallest, doesn't unify identity) and Option B (switch reader — eliminates trap but biome generation may need updating). Documented in Round-7-Closure §12.
- **Consequences:** +297/-140 lines across 9 files. Texture-data §7.7 instance fixed. Andrew-gate verification 2026-05-08 found 8/22 materials paint correctly, 14/22 produce "pixelated green splotches" — surfaced Round 8 Mechanism H8.1 (UI/renderer capacity boundary).

### Decision: 32-layer canonical material library (Real-Fix.D 2026-05-08)
- **Date:** 2026-05-08. See `docs/architecture/terrain_materials.md` for the canonical decision record.
- **Status:** Accepted (in active code)
- **Notes:** Detailed in the terrain materials trace. Mentioned here because it directly resolves the Round-8 Mechanism H8.1 (UI/renderer capacity boundary) at the material library layer.

### Decision: Two GI implementations coexist (Lumen + VXGI)
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (in active code)
- **Notes:** `lumen.rs` orchestrates Lumen-style GI; `gi/vxgi.rs` implements Voxel GI. Both are first-class modules. Whether one is preferred for new scenes, or whether they serve different use cases, is not recorded in available sources.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | All scene rendering goes through `astraweave_render::Renderer`; the editor wraps it via `EngineRenderAdapter` | Partial | Compile-time for entity rendering; FastPreview residue at `engine_adapter.rs:25-28` is a documented exception |
| 2 | `Renderer::draw_into(view, depth_view, enc)` writes depth to the caller-supplied `depth_view` when provided | Yes | Code path at `renderer.rs:5234-…`; behavioural verification via editor brush ring rendering (Andrew-gate 2026-05-07 partial PASS) |
| 3 | Per-cascade shadow uniforms use separate `wgpu::Buffer` instances to avoid `queue.write_buffer` race | Yes | `renderer.rs:752-757` (per-cascade arrays + the comment explaining the race) |
| 4 | The frame graph in `frame_graph.rs` validates topology but delegates GPU command recording to `Renderer` methods | Yes | `frame_graph.rs:18-25` "Migration Status" block plus the absence of command-encoder calls in `frame_graph.rs` pass-node implementations |
| 5 | `MAX_TERRAIN_LAYERS = 32` and `NUM_SPLAT_MAPS = 8` are the canonical material library capacity | Yes | `material_library.rs` constants; mirrored in `pbr_terrain.wgsl` (`array<TerrainLayer, 32>`), `terrain_material.rs` (`TerrainMaterialGpu`), and `terrain_splat_builder.rs`. Detailed in `docs/architecture/terrain_materials.md` |
| 6 | `MaterialGpu` flags (`FLAG_HAS_ALBEDO`, `FLAG_HAS_NORMAL`, `FLAG_HAS_ORM`, `FLAG_TRIPLANAR`) and extended-material flags (`MATERIAL_FLAG_CLEARCOAT`, etc.) are stable bit positions | Yes | `material.rs:25-31` and `material_extended.rs` (`MATERIAL_FLAG_*` constants re-exported at `lib.rs:223-226`) |
| 7 | `types::Material` is re-exported at the crate root; `material_library::Material` is intentionally NOT, to avoid the collision | Yes | `lib.rs:147` (re-export) + `lib.rs:251-253` (explicit no-export comment) |
| 8 | `Renderer::resource_generation` is monotonically incremented on resize / resource invalidation; `CachedBindGroup` rebuilds against the new generation | Yes | `renderer.rs:708-710` (field doc) + `bind_group_cache.rs` API |
| 9 | All shader files under `astraweave-render/shaders/` are registered via `ShaderManager` (for hot-reload) OR inlined via `include_str!` in pipeline construction | Partial | [NEEDS VERIFICATION — workspace audit of shader registration coverage] |
| 10 | `#![deny(unsafe_code)]` is enforced on the crate root (`astraweave-render/src/lib.rs:1`) — the crate is unsafe-free | Yes | Compile-time deny lint at `lib.rs:1` |
| 11 | `astraweave-materials::Node` enum is `#[non_exhaustive]` so external adders cannot break exhaustive matches | Yes | `astraweave-materials/src/lib.rs:7` |
| 12 | Editor `RenderMode::EnginePBR` is the default; `FastPreview` exists for fallback only | Yes | `engine_adapter.rs:31-34` `impl Default for RenderMode` returns `EnginePBR` |
| 13 | All Sub-phase 3 Mediator Brush §7.7 fixes preserve visual parity with the canonical pipeline — i.e. Real-Fix.A/B/C/D each verified via Andrew-gate | Yes | Campaign-doc Status header at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` records each gate verdict |
| 14 | The water surface renders as a chunked-LOD discrete tile grid (per-LOD instanced tiles + skirts) with world Y from `WaterUniforms.water_level` — no baked mesh Y, no single fixed plane | Yes | `water.rs` `WaterRenderer::update`/`render` (camera-distance LOD assignment + per-LOD instanced draw) + `WaterUniforms.water_level`; `water.wgsl` `vs_main` applies the uniform; W.2a (`docs/campaigns/water-successor/W2A_EXECUTION_REPORT.md`) |
| 15 | The water pass runs in its OWN raster pass AFTER the opaque main pass closes AND after the `hdr_tex → water_scene_color` snapshot copy; it writes `hdr_view` while sampling `water_scene_color` (a copy, never the live attachment) and binds the scene depth READ-ONLY (`depth_ops: None`) for both depth-test and sampling. The depth attachment view and the sampled `scene_depth` binding must be the same texture; `draw_into()` threads the caller-supplied depth so the editor tests/samples the depth its opaque geometry wrote. | Yes | `renderer.rs` `run_water_pass` + its call sites in `render()` (depth `None`) and `draw_into()` (caller depth); read-only-depth+sample capability proven by `examples/depth_sample_capability_probe.rs`; W.2b.2 (`docs/campaigns/water-successor/W2B2_EXECUTION_REPORT.md`) |
| 16 | Weave-response (part/freeze/raise) is **render-side presentation owned by `WaterRenderer`** (NOT the `astraweave-water` gameplay-truth facade — see `W2_DECISIONS.md` §B.1); `astraweave-render` has no dependency on `astraweave-water`. Profiles are **normalized, position-agnostic** (location lives only in the runtime `WeaveInstance`); deformation applies in `water.wgsl` `vs_main` AFTER the Gerstner sum (per-wave Q-cap untouched), is bounded to `±skirt_depth` and world-XZ sampled (no new LOD-seam mechanism), and is **identity at zero instances** (the loop is `weave_count`-gated, so the surface is bit-identical to pre-W.2c with no weaves). Ceiling `MAX_WEAVE_INSTANCES = 8`. | Yes | `water.rs` `WeaveInstance`/`set_weave_instances` + `WaterUniforms` 512 B layout (`test_uniforms_size`); `water.wgsl` `weave_accumulate`/`weave_profile`; zero-instance identity + per-instance cost measured by `examples/water_budget_probe.rs` (PROBE D); W.2c.2 (`docs/campaigns/water-successor/W2C1_RECON.md`) |

---

## 9. Performance & Resource Profile

### Hot paths
- **Main scene pass** (`Renderer::render` and `Renderer::draw_into`): runs once per frame at ~60 Hz target. Cost dominated by fragment-shader light + shadow + material sampling.
- **Per-fragment cluster lookup + light list iteration**: bounded by clustered-light bin density and per-cluster light count.
- **Shadow cascade rasterization**: 2 cascades by default per `FrameGraphConfig::default()` (`frame_graph.rs:67`).
- **Bloom downsample + tent upsample**: 13-tap downsample, fixed chain length. Per `bloom.rs` and `lib.rs:65` header.
- **GTAO bilateral upsample, SSGI denoise, SSR Hi-Z ray march**: per-frame, fixed-cost; shaders at `shaders/{gtao.wgsl, ssgi*.wgsl, ssr.wgsl}`.

### Cold paths
- **Shader / pipeline creation**: cached via `PipelineCache` (`pipeline_cache.rs`) and `bind_group_cache.rs` (generation-tracked). First-launch cold-start cost mitigated by disk-persisted Vulkan/DX12 pipeline cache.
- **glTF / OBJ loading**: feature-gated; loaded at scene/asset boundaries, not per-frame.
- **Material library upload**: per material-set change; texture array build is O(layers × resolution²).
- **Terrain chunk upload**: per chunk-update; CPU-side splat bake + GPU upload through `Renderer::upload_terrain_chunks` (initial) or `update_terrain_chunk` (incremental, via the shared `upload_or_update_terrain_chunk_forward` helper post Real-Fix.B).

### Resource ownership
- **`wgpu::Device` / `wgpu::Queue` / `wgpu::Surface`**: owned by `Renderer` (`renderer.rs:692-694`). Lifetime = renderer lifetime.
- **All render pipelines, bind groups, depth/shadow/HDR textures**: owned by `Renderer` (extensive field list at `renderer.rs:691-…`).
- **Per-frame transients**: allocated through `staging_ring` (`renderer.rs:707`).
- **Cached bind groups**: live in `bind_group_cache.rs`, indexed by generation.
- **Pipeline cache disk file**: persisted via `_pipeline_cache_mgr`'s `Drop` (`renderer.rs:701-703`).
- **GPU timestamps**: allocated per-pass by `gpu_profiler` (`renderer.rs:704-705`); may be `None` if `TIMESTAMP_QUERY` is unsupported.
- **Terrain chunk forward path**: `Renderer::terrain_forward.chunks: HashMap<ChunkKey, …>` (referenced at `renderer.rs:5755` per Round-6 evidence). Lifetime = renderer lifetime; entries written by upload/update paths.

---

## 10. Testing & Validation

- **Inline `#[cfg(test)]` modules:**
  - `astraweave-render/src/renderer.rs` — see `renderer_tests.rs` sibling file (referenced at `lib.rs:???`); inline tests count not fully enumerated here
  - `astraweave-render/src/mutation_tests.rs` (declared `#[cfg(test)]` at `lib.rs:129`)
  - `astraweave-render/src/animation_extra_tests.rs` (`lib.rs:126`)
  - `astraweave-render/src/renderer_tests.rs` (sibling file)
  - `astraweave-render/src/nanite_gpu_culling_tests.rs` (sibling file)
- **Integration tests (`astraweave-render/tests/`):**
  - Per workspace audit, this crate has multiple `tests/*.rs` files including `coverage_booster_render.rs`, `terrain_splat_pipeline.rs`, `test_terrain_material.rs`, `wave2_*` remediation suites covering camera, decals, HDRI, mesh, scene env, SSAO, terrain quad registry, transparency, weather. Exact count not enumerated here.
- **WGSL shader review:** The CLAUDE.md `shader-wgsl-reviewer` agent serves as the GPU code verifier — since no formal WGSL verification tooling exists at the Miri/Kani level, this agent fills that gap through rigorous pattern-based review. Used when `.wgsl` files are created or modified.
- **Visual validation gate:**
  - The Editor Multi-Tool Architecture Campaign Sub-phase 3 is governed by Andrew-gate visual verification per `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md §0`. Real-Fix.A/B/C each received partial-PASS verdicts narrowing the defect class. Real-Fix.D pending.
  - The Editor Behavioral Correctness Audit (`docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`, completed 2026-04-05) covered 37 fixes including visual math, data pipeline, undo system, silent failures, integration seams.
- **Mutation testing:** Wave 2 mutation testing campaigns covered renderer with `tests/wave2_*` suites. Specific kill rates not enumerated here.
- **Manual / example validation:** Every render example crate (`hello_companion`, `unified_showcase`, `weaving_playground`, `biome_weather_demo`, `phi3_demo`, `ecs_ai_showcase`, etc.) exercises portions of the pipeline at runtime. The `unified_showcase` example is the canonical end-to-end visual exercise per CLAUDE.md.

---

## 11. Open Questions / Parked Decisions

- **Frame graph migration: when does it move from scaffold to command-driver?** [Decisional.] `frame_graph.rs:18-25` documents "full delegation is designed for incremental adoption." The current state — pass nodes delegating back to `Renderer` methods — is intentional. The question is whether/when to migrate command recording into the frame graph itself, and what triggers that work.
- **Lumen GI vs VXGI — coexisting or eventual single choice?** [Decisional.] Both are fully implemented (`lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` vs `gi/vxgi.rs` + `gi/voxelization_pipeline.rs`). Whether to keep both as configurable alternatives or to deprecate one has not been recorded in available sources.
- **`RenderMode::FastPreview` long-term disposition.** [Decisional.] Per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, FastPreview was targeted for elimination but the enum variant remains at `engine_adapter.rs:25-28` as a transitional fallback. Whether to fully remove it (and what the new fallback path is for weak GPUs / very large scenes) is open.
- **Three material binding paths — eventual consolidation?** [Decisional.] `MaterialManager` (TOML-driven), `MaterialLibrary` (canonical terrain), and `BindlessMaterialSystem` (bindless modern) coexist. Each currently serves a distinct purpose; whether to consolidate to one canonical path is not recorded.
- **Inline `SHADER_SRC` in `renderer.rs:18-…` vs registered shaders.** [Decisional.] The main forward+ pipeline's WGSL is inlined in `renderer.rs` via `concat!` + `include_str!`. Most newer shaders live in separate files registered via `ShaderManager`. Whether to migrate the inline shader to a registered file (enabling hot-reload for the main pass) is an open question with non-trivial blast radius.
- **§7.7 wrapped-component resource identity trap — preventive instrumentation?** [Decisional, surfaced by Editor Multi-Tool Architecture Campaign Sub-phase 3.] Four distinct §7.7 instances have been confirmed during Sub-phase 3. The campaign Status header (commit `e3d07f366`) suggests a CLAUDE.md amendment cycle is pending — potentially elevating the resource-identity rule from "candidate corollary appended to Edit 2" to a first-class top-level Edit. The decisional question is whether to encode the rule architecturally (lint, build-script, or trait-pattern enforcement) rather than relying on documentation alone.
- **`shader-wgsl-reviewer` agent vs formal verification.** [Decisional.] CLAUDE.md describes the agent as filling the Miri/Kani gap for WGSL code. Whether to invest in formal WGSL verification tooling (Naga validation, third-party shader analyzers) for safety-critical shaders is open.
- **Round 8 Andrew-gate decision (h) for Real-Fix.D — Option D-1 vs D-2 vs D-3 — was made via the canonical material library landing 2026-05-08 (commit `7067cc03d` per repo `git log` recent commit list).** Factual portion closed; the decisional follow-up (does the same approach extend to non-terrain material catalogs?) is not currently a campaign agenda item but worth flagging.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new top-level module is added to `astraweave-render/` (touch §5)
- The `Renderer` struct gains or loses major fields, especially `render()` / `draw_into()` signatures (§2, §3, §6)
- A new §7.7 wrapped-component resource identity instance is confirmed (§6 trap list, §7 decision log if a Real-Fix lands)
- A new shader file is added to `astraweave-render/shaders/` (§5, possibly §3 if it introduces vocabulary)
- The frame graph transitions from scaffold to command-driver (§7 decision log, §11 question)
- A material binding path is consolidated or removed (§5, §6, §11)
- The Fix 27 Unified Pipeline Campaign or Editor Multi-Tool Architecture Campaign advances a sub-phase or closes
- The `RenderMode::FastPreview` variant is removed (§5, §6, §11)
- A decision in §7 is superseded by new code or audit

**Verification process:**
- Spot-check the pipeline diagrams in §2 against `renderer.rs::render` (around line 4708), `renderer.rs::draw_into` (around line 5234), and `frame_graph.rs::build` (in `frame_graph.rs:80+`).
- Verify the file map in §5 against `astraweave-render/src/lib.rs` `pub mod` declarations (lines 34-178) and `pub use` re-exports (lines 139-279).
- Verify invariants in §8 against actual code locations cited.
- Check `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header for any new sub-phase or §7.7 instance landing.
- Update metadata commit hash and date after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **All scene rendering goes through `astraweave_render::Renderer`.** Per CLAUDE.md Scope Discipline ("Never build a second implementation of a logical system that already exists — rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer") and per the Fix 27 Unified Pipeline Campaign. The editor wraps the Renderer via `EngineRenderAdapter`; do not introduce a third path.
2. **§7.7 wrapped-component resource identity trap is the single most-cited failure mode in this layer.** Four confirmed instances during Sub-phase 3 Mediator Brush diagnostics across depth-target, mesh-data, texture-attribute-set, and UI-vs-renderer-capacity boundaries. Before adding a new wrapper layer around `Renderer`, ask: "do I create a parallel resource of the same logical role?" If yes, design the unification at the boundary, not in code that reads its own resource.
3. **Frame graph is scaffolded, not command-driving.** Pass nodes delegate GPU command recording back to `Renderer` methods. Don't try to record commands directly inside `frame_graph.rs` pass nodes today.
4. **Three material binding paths coexist intentionally.** `MaterialManager` (TOML), `MaterialLibrary` (canonical 32-layer terrain), `BindlessMaterialSystem` (bindless modern). Each serves a different purpose.
5. **Per-cascade shadow uniform buffers are deliberate.** Do not "simplify" to a single shadow uniform buffer — the comment at `renderer.rs:752-757` documents the `queue.write_buffer` race.
6. **`Renderer::draw_into` accepts an optional depth view.** Editor callers pass their own depth target so overlays and engine terrain share depth. Pass `None` only if the caller does not need depth coherence.
7. **`MAX_TERRAIN_LAYERS = 32`.** Anything terrain-material-related crosses the canonical material library — see `docs/architecture/terrain_materials.md`.
8. **`#![deny(unsafe_code)]` on the crate root.** Don't ship `unsafe` to `astraweave-render`. Use `wgpu`'s safe API or push the `unsafe` boundary into a different crate.

**Files you'll most likely touch:**
- `astraweave-render/src/renderer.rs` — the canonical aggregator (high blast radius)
- `astraweave-render/src/lib.rs` — `pub mod` + `pub use` (always check before adding a new public type)
- `astraweave-render/src/material*.rs` — material system (choose the right path)
- `astraweave-render/src/frame_graph.rs` + `graph.rs` — DAG topology
- `astraweave-render/shaders/*.wgsl` — shader programs (invoke the `shader-wgsl-reviewer` agent on changes per CLAUDE.md)
- `tools/aw_editor/src/viewport/{renderer.rs, engine_adapter.rs}` — editor entry into the pipeline

**Files you should NOT touch without strong reason:**
- `astraweave-render/src/renderer.rs:18-…` `SHADER_SRC` inline WGSL — historical inline shader for the main pass. Adding to it bloats the already-huge file; if changing the main pass shader, consider migrating to a registered file (see §11).
- `astraweave-render/src/renderer.rs:752-757` per-cascade shadow uniform buffers — don't "simplify"; the race is real.
- `astraweave-render/src/material_library.rs` — canonical 32-layer terrain material registry; changes affect the editor, the splat builder, the shader, and the renderer simultaneously.
- `astraweave-render/src/terrain_material_manager.rs` — feature-gated forward terrain pipeline, see `docs/architecture/terrain_materials.md`.
- `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` `RenderMode::FastPreview` — Fix 27 residue. Removing it has consequences (see §11).
- `tools/aw_editor/src/viewport/terrain_splat.rs` — superseded reference material; do not extend.

**Common mistakes when changing this system:**
- **Mistake**: Adding a new editor-side render path "just for previewing" something quickly.
  **Why wrong**: Fix 27 exists explicitly because dual pipelines diverged across 12 dimensions. CLAUDE.md Scope Discipline forbids this. Use `Renderer::draw_into` + an editor overlay hook instead.
- **Mistake**: Creating a wrapper around `Renderer` that holds its own depth target, terrain chunks, splat textures, or material library.
  **Why wrong**: §7.7. The wrapper's resources won't reflect the underlying `Renderer`'s state (or vice versa). Either share resources via cross-crate API changes (Real-Fix.A pattern) or unify at the boundary (Real-Fix.C pattern).
- **Mistake**: Routing initial-upload through one code path and incremental-update through another.
  **Why wrong**: §7.7 at the data-routing layer (Real-Fix.B pattern). Extract a shared helper called by both.
- **Mistake**: Writing to one attribute (`vertex.material_ids`) and reading from another (`vertex.biome_weights_0/1`) of the same logical role.
  **Why wrong**: §7.7 intra-component sub-variant (Real-Fix.C pattern). Unify into one canonical attribute set.
- **Mistake**: Sizing UI capacity and renderer capacity to different values for the same logical role.
  **Why wrong**: §7.7 UI-vs-renderer capacity boundary (Round-8 / Real-Fix.D pattern). Share the capacity from a single canonical source.
- **Mistake**: Adding a new shader as inline WGSL inside `renderer.rs`.
  **Why wrong**: That file is already 7,800 LoC and the `SHADER_SRC` inline block is historical. New shaders go in `shaders/*.wgsl` and are registered via `ShaderManager` or `include_str!`'d at pipeline creation.
- **Mistake**: Skipping the `shader-wgsl-reviewer` agent on a new `.wgsl` file.
  **Why wrong**: CLAUDE.md mandates this for `.wgsl` changes. There is no Miri/Kani equivalent for shaders; the agent is the safety net.

---

## Appendix B: Historical context

The render system grew in three eras.

The earliest era produced the core `Renderer` aggregator and the inline `SHADER_SRC` main-pass shader in `renderer.rs`. This established the single-aggregator pattern and the wgpu device/queue/surface ownership.

The middle era added subsystems in phase-numbered waves (visible in module comments: Phase PBR-E/-F for materials, Phase 2 for clustered forward and CSM, Phase 3 for volumetric clouds, Phase 5 for Lumen GI + impostor bake, Phase 6 for god rays + volumetric fog, Phase 7 for particles, Phase 8 for atmosphere, Phase 9 for IBL/BRDF LUT). Each phase brought new shader files and new `pub use` re-exports at the crate root. The `frame_graph.rs` declarative DAG scaffolding was added as part of this phase but kept in delegated mode pending incremental migration.

The recent era is dominated by **two campaigns**:

1. **Fix 27 Unified Pipeline Campaign** (planned 2026-04-05, `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`). The editor maintained a `FastPreview` path independent of the engine renderer — its own PBR shader, its own CSM, its own BRDF LUT, its own IBL, its own glTF loader, its own tonemap. Twelve dimensions of divergence. The campaign re-routed all scene rendering through `astraweave_render::Renderer` and formalized editor overlay injection. Per CLAUDE.md current status: "structural phase complete (astraweave-render non-optional, entity rendering through engine); deeper unification (shadow/IBL/post-processing alignment) ongoing." `RenderMode::FastPreview` survives as transitional residue.

2. **Editor Multi-Tool Architecture Campaign Sub-phase 3 Mediator Brush** (2026-05-04 onwards, `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`). Eight diagnostic rounds across 4 days converged on a single architectural anti-pattern that recurs at every editor↔renderer boundary: when component A wraps component B and both manage resources of the same logical role, A's reads don't reflect B's writes. Round 5 confirmed it at the depth-target layer (Real-Fix.A `0f569d212`). Round 6 confirmed it at the mesh-data layer (Real-Fix.B `eaaa53433`). Round 7 confirmed it at the texture-data attribute-set layer (Real-Fix.C `ded9a0457`). Round 8 confirmed it at the UI-vs-renderer-capacity boundary (Real-Fix.D 2026-05-08, commit `7067cc03d`). Four distinct layers, four distinct fixes, one canonical anti-pattern — now codified as **§7.7 structural axiom** of the editor's resource-management at every architectural boundary. The campaign's working hypothesis is that a CLAUDE.md amendment cycle is pending to elevate the resource-identity rule from a corollary to a first-class top-level Edit.

The Sub-phase 3 work pattern — multi-round instrument-and-narrow with single-concern sessions — is itself canonical for this layer going forward.
