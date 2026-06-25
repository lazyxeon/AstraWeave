# Architecture Trace: Asset System

## Metadata

| Field | Value |
|---|---|
| **System name** | Asset System (loading, cell loader, Nanite preprocess, asset pipeline) |
| **Primary crates** | `astraweave-asset`, `astraweave-asset-pipeline` |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active (mixed): glTF CPU loaders + cell loader + AssetDatabase are wired; Nanite preprocess and the entire `astraweave-asset-pipeline` crate are in-design / dormant scaffolding |
| **Owner notes** | First trace for this subsystem. `astraweave-asset` is one of the most-consumed untraced crates (9+ Cargo consumers). Derived from forensic data-flow analysis on 2026-06-24. |

---

## 1. Executive Summary

**What this system does:**
Provides CPU-side asset ingestion for the engine — parsing glTF/GLB into mesh/material/skeleton/animation data, deserializing World-Partition cells from RON, maintaining a GUID-keyed asset metadata/dependency database, and offering offline mesh/texture transform utilities (meshlet preprocessing, BC7/ASTC compression, validation).

**Why it exists:**
Decouples raw on-disk asset formats from runtime consumers (render, scene streaming, gameplay), giving each a typed in-memory representation plus a deterministic GUID identity for caching and hot-reload.

**Where it primarily lives:**
- `astraweave-asset/src/lib.rs` (3790 LoC) — glTF loaders (`gltf_loader`), `AssetDatabase` + `AssetWatcher` hot-reload, `AssetCache`, deterministic GUIDs, optional `blend_import`/`blend_asset_integration`, `import_pipelines`.
- `astraweave-asset/src/cell_loader.rs` (841 LoC) — World-Partition cell RON (de)serialization + per-asset byte loading.
- `astraweave-asset/src/nanite_preprocess.rs` (941 LoC) — meshlet clustering + QEM LOD hierarchy (CPU preprocess).
- `astraweave-asset-pipeline/src/{lib,texture,mesh,validator}.rs` — separate crate of offline transforms (BC7/ASTC, vertex-cache/overdraw optimization, CI validation).

**Status note:**
The wiring picture is split and must be understood before editing:
- **Wired into runtime:** `cell_loader::load_cell_from_ron` / `load_asset` (consumed by `astraweave-scene` streaming), `gltf_loader` mesh loaders (consumed by examples and the editor terrain integration), and `AssetDatabase` (consumed by `tools/aw_asset_cli` and `tools/aw_editor`).
- **Dormant / in-design-but-tested:** `nanite_preprocess` (no non-test/non-example caller; the render crate has a *separate* GPU-side Nanite path), `AssetWatcher` (zero production callers), `astraweave-render::residency::ResidencyManager` (re-exported, constructed only in render tests), and the **entire `astraweave-asset-pipeline` crate** (zero `astraweave_asset_pipeline::` references anywhere except its own doc-comments). See §5 and §6.

---

## 2. Authoritative Pipeline

The system is three loosely-coupled data paths. They share the `astraweave-asset` crate but do not feed each other at runtime.

### Path A — glTF/GLB → typed mesh/material/skeleton/animation (WIRED, on-demand)

```text
[GLB/.gltf bytes on disk]
    │
    │ fs::read(...) by caller (editor / example)
    ▼
[Stage A1: format detect + container parse]
    file: astraweave-asset/src/lib.rs  (gltf_loader, lib.rs:296+)
    role: GLB magic check ("glTF"), split JSON + BIN chunks
    key data: gltf::Gltf document + buffer blobs
    │
    │ load_first_mesh_from_glb_bytes / load_all_meshes_merged /
    │ load_first_mesh_and_material / load_skinned_mesh_complete /
    │ load_skeleton / load_animations
    ▼
[Stage A2: accessor decode → CPU structs]
    file: astraweave-asset/src/lib.rs  (lib.rs:419-1808)
    role: decode positions/normals/tangents/uv/indices, PBR material factors
          + KHR extension fields, skeleton joints, animation channels
    key data: MeshData / MaterialData / ImageData / SkinnedMeshData /
              Skeleton / AnimationClip
    │
    ▼
[Consumer uploads to GPU / animation rig — outside this crate]
```

### Path B — World-Partition cell RON → entity/asset refs → asset bytes (WIRED, runtime streaming)

```text
[<x>_<y>_<z>.ron cell file]
    │
    │ cell_loader::load_cell_from_ron(path)   (async, tokio::fs)
    ▼
[Stage B1: RON deserialize]
    file: astraweave-asset/src/cell_loader.rs:161-172
    role: parse CellData { coord, entities, assets, metadata }
    key data: CellData (EntityData[], AssetRef[])
    │
    │ cell_loader::load_asset(asset_ref, assets_root)   (async)
    ▼
[Stage B2: per-asset byte load + header validation]
    file: astraweave-asset/src/cell_loader.rs:223-308
    role: read raw bytes; validate GLB/PNG/JPEG magic by extension
    key data: Vec<u8> raw asset bytes (NOT decoded into MeshData here)
    │
    ▼
[astraweave-scene WorldPartitionManager::update drives this]
    file: astraweave-scene/src/streaming.rs:100-271
```

### Path C — mesh → meshlet LOD hierarchy (DORMANT preprocess; example-only)

```text
[positions/normals/tangents/uvs/indices]
    │
    │ generate_lod_hierarchy(...) / preprocess_mesh_async(...)
    ▼
[Stage C1: LOD 0 meshlet clustering]
    file: astraweave-asset/src/nanite_preprocess.rs:309-357
    role: meshopt::clusterize::build_meshlets (spatial clustering)
    key data: Vec<Meshlet> { vertices, indices(u8), AABB, BoundingCone }
    │
    │ simplify_mesh (QEM edge collapse) per LOD level
    ▼
[Stage C2: simplified LOD levels + error metrics]
    file: astraweave-asset/src/nanite_preprocess.rs:360-672
    key data: MeshletHierarchy { meshlets, lod_ranges, positions... }
    │
    │ save_meshlet_hierarchy / load_meshlet_hierarchy (RON)
    ▼
[examples/nanite_demo only — no engine runtime consumer]
```

### Stage-by-stage detail

#### Stage A1/A2: glTF loaders (`gltf_loader` module)
**File:** `astraweave-asset/src/lib.rs:296-1808` (module `gltf_loader`).
**Role:** Pure-CPU glTF/GLB decode behind the default-on `gltf` feature.
**Inputs:** Raw `&[u8]` (GLB binary or `.gltf` JSON, including `data:` URI buffers).
**Outputs:** `MeshData` (positions/normals/tangents/texcoords/indices, lib.rs:330-336), `MaterialData` (full PBR factor set + KHR_materials_transmission/ior/emissive_strength/clearcoat fields, lib.rs:357-415), `ImageData`, `SkinnedMeshData`/`Skeleton`/`AnimationClip` (lib.rs:857-941).
**Notes:** `load_gltf_bytes` (lib.rs:303) is a Phase-0 header-only validator that does not decode. Real decode entrypoints are `load_first_mesh_from_glb_bytes` (lib.rs:419), `load_all_meshes_merged` (lib.rs:487), `load_first_mesh_and_material` (lib.rs:574), and the skinned variants `load_skinned_mesh_complete` (lib.rs:1276) / `load_first_skinned_mesh_and_idle` (lib.rs:1475). Material defaults follow glTF spec defaults (e.g. `metallic_factor: 1.0`, `ior: 1.5`, lib.rs:392-415).

#### Stage B1: Cell RON deserialize
**File:** `astraweave-asset/src/cell_loader.rs:104-220`.
**Role:** (De)serialize `CellData` to/from RON, async (`tokio::fs`) and sync variants.
**Inputs:** RON cell file path.
**Outputs:** `CellData { coord:[i32;3], entities:Vec<EntityData>, assets:Vec<AssetRef>, metadata:Option<CellMetadata> }` (cell_loader.rs:104-115).
**Notes:** `EntityData` carries a transform (`position`/`rotation` quaternion/`scale`), an optional `mesh` path, an optional `material` layer index, and extensible `Vec<ComponentData>` (cell_loader.rs:49-66). `add_asset` deduplicates by `path` (cell_loader.rs:133-138). `cell_path_from_coord` formats `"{}_{}_{}.ron"` (cell_loader.rs:311).

#### Stage B2: Per-asset byte load + magic validation
**File:** `astraweave-asset/src/cell_loader.rs:223-308`.
**Role:** Read raw asset bytes from `assets_root.join(asset_ref.path)` and run a lightweight magic-number check (GLB `b"glTF"`, PNG `\x89PNG\r\n\x1a\n`, JPEG `0xFF D8 FF`).
**Inputs:** `&AssetRef`, `assets_root: &Path`.
**Outputs:** `Vec<u8>` raw bytes — NOT decoded into `MeshData`. Decode is the consumer's responsibility.
**Notes:** `cell_loader::AssetKind` (cell_loader.rs:12-21) is a *distinct enum* from the crate-root `AssetKind` (lib.rs:3256). See §6 naming collision.

#### Stage C1/C2: Nanite meshlet preprocess
**File:** `astraweave-asset/src/nanite_preprocess.rs`.
**Role:** Convert a triangle mesh into a meshlet LOD hierarchy for virtualized-geometry rendering.
**Inputs:** position/normal/tangent/uv/index slices, `lod_count`.
**Outputs:** `MeshletHierarchy` (nanite_preprocess.rs:218-234).
**Notes:** LOD 0 uses `meshopt::clusterize::build_meshlets` with `MAX_MESHLET_VERTICES=64` / `MAX_MESHLET_TRIANGLES=124` / `cone_weight=0.5` (nanite_preprocess.rs:21-24, 334-340). Lower LODs use an in-house QEM edge-collapse simplifier (`QuadricError`, `simplify_mesh`, nanite_preprocess.rs:236-665), targeting ~50% triangle reduction per level. `preprocess_mesh_async` wraps the CPU work in `tokio::task::spawn_blocking` (nanite_preprocess.rs:675-698). **No engine-runtime consumer** — see §6.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **GUID** | 128-bit hex digest (`SHA-256` of lowercased, forward-slash-normalized path, first 16 bytes) identifying an asset. | `guid_for_path` (lib.rs:1820-1825); `AssetDatabase`, `AssetCache` |
| **`AssetKind` (root)** | Crate-root asset-category enum used by `AssetDatabase`: `Mesh, Texture, Audio, Dialogue, Material, Animation, Script, BlenderSource, Other`. `#[non_exhaustive]`. | lib.rs:3256-3267 |
| **`AssetKind` (cell)** | Cell-loader's own enum: `Mesh, Texture, Material, Audio, Animation, Other` (no Dialogue/Script/BlenderSource). `#[non_exhaustive]`. | cell_loader.rs:12-21 |
| **`AssetMetadata`** | DB record: guid, path, kind, content hash, deps, mtime, size. | lib.rs:3244-3252 |
| **`AssetRef`** | Cell-level reference to an asset (path + cell `AssetKind` + optional guid). | cell_loader.rs:23-47 |
| **`CellData`** | Serialized World-Partition cell (entities + asset refs + metadata). | cell_loader.rs:104-147 |
| **Meshlet** | Triangle cluster with local vertex list, `u8` triangle indices, `AABB`, `BoundingCone`, LOD level/error. | nanite_preprocess.rs:142-215 |
| **`MeshletHierarchy`** | All meshlets across LODs + original vertex streams + `lod_ranges`. | nanite_preprocess.rs:218-234 |
| **ACMR** | Average Cache Miss Ratio — vertex-cache efficiency metric (lower is better). | pipeline `mesh.rs:185-210`, `validator.rs:171-200` |
| **Residency** | Memory-budgeted loaded/evicted state of assets (LRU). | `astraweave-render::residency` |

### Terms to NOT confuse

- **Root `AssetKind` vs cell `AssetKind`:** Two independent enums with the same name in the same crate (`lib.rs:3256` and `cell_loader.rs:14`). They are NOT interchangeable and have different variant sets. Code that bridges DB and cell layers must convert explicitly.
- **`MeshData` vs `Mesh`:** `gltf_loader::MeshData` (lib.rs:330) is the decoded glTF result (separate position/normal/tangent/uv vectors). `astraweave-asset-pipeline::mesh::Mesh` (mesh.rs:38) is a flat `positions: Vec<f32>` + `indices` optimization input. Unrelated types in different crates.
- **`astraweave-asset` Nanite preprocess vs `astraweave-render` Nanite:** the CPU `nanite_preprocess::Meshlet`/`MeshletHierarchy` are *not* the render crate's `GpuMeshlet` (in `astraweave-render/src/nanite_visibility.rs`, behind the `nanite` feature). The render path does not consume this crate's meshlets. See §6.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source | Interface | Data | Notes |
|---|---|---|---|
| Filesystem | `std::fs` / `tokio::fs` (in-crate) | raw bytes, RON, JSON manifests | All I/O is direct; no VFS abstraction. |
| `astraweave-blend` (optional `blend` feature) | `blend_import` re-exports `BlendImporter`, `decompose_blend`, etc. (lib.rs:34-40) | `.blend` → glTF via Blender subprocess | Gated by `blend` feature; gracefully disables if Blender absent (lib.rs:88-93). Used by `tools/aw_editor`. |
| `meshopt` (vendored crate) | `clusterize::build_meshlets`, `optimize_vertex_cache`, `analyze_overdraw` | meshlet clustering + cache stats | Used by `nanite_preprocess.rs` and pipeline `mesh.rs`. |

### Downstream (what consumes this system's output)

Cargo consumers of `astraweave-asset` (excluding the crate itself, its `fuzz` member, and the root manifest), verified via `grep "astraweave-asset" */Cargo.toml`:

| Consumer | Interface used | Wired? | Notes |
|---|---|---|---|
| `astraweave-scene` | `cell_loader::{load_cell_from_ron, load_asset, CellData, EntityData, AssetRef, AssetKind, CellMetadata, ComponentData}` | **Yes** | Runtime streaming: `WorldPartitionManager::update` → `start_load_cell` → `load_cell_data`/`load_asset_data` (`streaming.rs:147-271`). |
| `astraweave-render` | `AssetDatabase`, `AssetKind`, `AssetMetadata` via `residency.rs` | **Partial** | `ResidencyManager` (render/src/residency.rs) takes `Arc<Mutex<AssetDatabase>>`; re-exported at `lib.rs:237` but constructed only in render tests/benches (no production caller found). |
| `astraweave-gameplay` | `cell_loader::ComponentData` (`veilweaver_slice.rs:273`) | **Test-only** | The single `use astraweave_asset::cell_loader::ComponentData` is inside `#[cfg(test)] mod tests` (`veilweaver_slice.rs:270-273`); no production-path use of `astraweave_asset` exists in `astraweave-gameplay/src` (verified by `grep -rn astraweave_asset astraweave-gameplay/src` → only line 273). Corrected during 1.1 verification (was "Yes / Single typed import"). |
| `astraweave-scripting` | declared dep, **no `use`** | **No** | Declared-but-unused Cargo dependency (`Cargo.toml:19`); scripting uses its own `loader::ScriptLoader`. Matches CLAUDE.md Key-Lesson-8 "declared-but-unused Cargo deps". |
| `tools/aw_asset_cli` | `AssetDatabase::{new, scan_directory, register_asset, save_manifest}` (`main.rs:166-202`) | **Yes** | CLI builds a manifest. Owns its OWN texture compression + validators (see §6). |
| `tools/aw_editor` | `AssetDatabase`, `blend_import::{BlendImporter, DecomposedAsset, DecompositionResult}` (`main.rs`), `gltf_loader::load_all_meshes_merged` (`terrain_integration.rs:471`) | **Yes** (asset) / **No** (pipeline) | Depends on `astraweave-asset` with `features=["blend"]`. Also declares `astraweave-asset-pipeline` (`Cargo.toml:99`) but never references it — see §6. |
| `examples/hello_companion` | `gltf_loader` (`visual_demo.rs:30`) | example | |
| `examples/veilweaver_demo` | `gltf_loader::load_all_meshes_merged` (`visual_renderer.rs`) | example | |
| `examples/visual_3d` | `gltf_loader as gl` | example | |
| `examples/nanite_demo` | `nanite_preprocess::{generate_lod_hierarchy, MeshletHierarchy}` (`main.rs`) | example | **Only** consumer of `nanite_preprocess`. |
| `examples/skinning_demo` | declares dep; uses `astraweave_render::animation` `Skeleton` | example | Its `Skeleton` comes from the render/animation crate, not this crate's `gltf_loader::Skeleton`. |

`astraweave-asset-pipeline` Cargo consumers: only `tools/aw_editor` (`Cargo.toml:99`) and the root manifest — and the editor never references the crate's API. **No production caller of `astraweave_asset_pipeline::*` exists** (the only references in the workspace are doc-comment examples inside the crate). See §6.

### Bidirectional / Coupled

- **`AssetDatabase` ↔ hot-reload:** `AssetDatabase` owns a `tokio::sync::watch` channel (`hot_reload_tx`/`rx`, lib.rs:3275-3276); `AssetWatcher` (lib.rs:3497) wraps the DB in `Arc<Mutex<…>>`, runs a `notify` filesystem watcher with a debounced background thread, and calls `invalidate_asset` which fires the watch channel. `ResidencyManager::with_hot_reload` consumes a `watch::Receiver` to drain invalidations. This whole loop is exercised only by tests today.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-asset/src/lib.rs` | glTF loaders, `AssetDatabase`/`AssetWatcher`, GUID/cache, blend import, import_pipelines | Active (mixed) | 3790 LoC. glTF loaders + `AssetDatabase` are wired; `AssetWatcher` is dormant (no prod caller). |
| `astraweave-asset/src/cell_loader.rs` | World-Partition cell (de)serialize + per-asset byte load | Active | Wired into `astraweave-scene` streaming runtime. |
| `astraweave-asset/src/nanite_preprocess.rs` | meshlet clustering + QEM LOD hierarchy | Transitional / In-design | No non-test/non-example caller. Render Nanite is a separate path. |
| `astraweave-asset/src/mutation_tests.rs` | mutation-killing tests | Active (test-only) | `#[cfg(test)] mod mutation_tests`. |
| `astraweave-asset-pipeline/src/lib.rs` | crate root, re-exports | In-design | Zero production consumers. |
| `astraweave-asset-pipeline/src/texture.rs` | BC7 (`intel_tex`, `bc7` feature) + ASTC (basisu CLI / `basis_universal` transcode, `astc` feature) | In-design | `compress_bc7`/`compress_astc`/transcode funcs. Default feature `astc` requires `basis-universal` (declared `optional`). |
| `astraweave-asset-pipeline/src/mesh.rs` | vertex-cache + overdraw optimization (`meshopt`) | In-design | `optimize_mesh`, `Mesh::new`, ACMR. |
| `astraweave-asset-pipeline/src/validator.rs` | texture/mesh validation reports for CI | In-design | `AssetValidator`, `ValidationReport`, `BatchValidationReport`. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit with care.
- **Transitional**: Active code but planned for change or not yet wired to its intended consumer.
- **In-design**: Implemented + tested but has no production caller (dormant per CLAUDE.md Key Lesson 8).

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition (factual) |
|---|---|---|---|
| CPU Nanite meshlet preprocess | `astraweave-asset/src/nanite_preprocess.rs` (`Meshlet`, `MeshletHierarchy`) | In-design | Example-only consumer (`nanite_demo`). |
| GPU Nanite visibility/culling | `astraweave-render/src/nanite_visibility.rs` (`GpuMeshlet`), `nanite_gpu_culling.rs`, `nanite_render.rs` (behind `nanite` feature) | Active (feature-gated) | Separate type set; does not deserialize this crate's `MeshletHierarchy`. The two Nanite halves are not connected by a shared on-disk format in current code. |
| Texture compression (offline) | `astraweave-asset-pipeline/src/texture.rs` (`compress_bc7`, `compress_astc`) | In-design (no caller) | — |
| Texture compression (CLI) | `tools/aw_asset_cli/src/main.rs:298+` (`process_texture` shells out to `toktx`/`basisu`), `tools/aw_asset_cli/src/texture_baker.rs` | Active | The CLI does NOT call `astraweave-asset-pipeline`; it implements its own compression by invoking external CLI tools. |
| Asset validation | `astraweave-asset-pipeline/src/validator.rs` (`AssetValidator`) | In-design (no caller) | — |
| Asset validation (CLI) | `tools/aw_asset_cli/src/validators.rs` (`validate_texture`, `validate_ktx2_mipmaps`, `validate_material_toml`) + `validators_original.rs` | Active | The CLI has its own validator family, parallel to the pipeline crate's. |
| Mesh format validation | `cell_loader::validate_mesh_format`/`validate_texture_format` (magic-number, cell_loader.rs:272-308); `gltf_loader::load_gltf_bytes` (header check, lib.rs:303) | Both Active | Two lightweight validators in the same crate for overlapping purposes (cell-load path vs generic glTF entry). |

### Naming collisions

- **`AssetKind`**: defined twice in `astraweave-asset` — crate root (`lib.rs:3256`, 9 variants incl. Dialogue/Script/BlenderSource) and `cell_loader` (`cell_loader.rs:14`, 6 variants). Consumers import one or the other; `astraweave-scene` uses the cell variant. No conversion helper exists between them.
- **`Mesh` / `MeshData`**: `gltf_loader::MeshData` (decoded glTF) vs pipeline `mesh::Mesh` (flat optimization input) — unrelated.
- **`Meshlet`**: `astraweave-asset::nanite_preprocess::Meshlet` (CPU) vs `astraweave-render::nanite_visibility::GpuMeshlet` (GPU) — unrelated layouts.
- **`ResidencyManager`**: the real type lives in `astraweave-render/src/residency.rs`; an unrelated bench-local struct of the same name exists in `astraweave-render/benches/clustered_megalights_residency.rs:265` (benchmark scaffolding, not the production type).

### Known cognitive traps

- **Trap:** The `astraweave-asset-pipeline` crate looks like the production asset-processing pipeline.
  **Why it's confusing:** It has polished docs, tests, benches, and is declared as a dependency of `tools/aw_editor`.
  **What's actually true:** No code outside the crate references `astraweave_asset_pipeline::*` (verified by `grep` — only doc-comment occurrences). The editor's dependency line (`tools/aw_editor/Cargo.toml:99`) has no matching `use`. The functioning asset-bake path is `tools/aw_asset_cli`, which reimplements compression/validation independently.

- **Trap:** `nanite_preprocess` appears to feed the render crate's Nanite pipeline.
  **Why it's confusing:** Both are called "Nanite" and both define meshlets.
  **What's actually true:** No data path connects them in current code; the only caller of `nanite_preprocess` is `examples/nanite_demo`.

- **Trap:** `gltf_loader::load_gltf_bytes` looks like the loader.
  **Why it's confusing:** Name + position.
  **What's actually true:** It only validates the GLB header / detects JSON (lib.rs:301-327, comment "Phase 0 scope"). The decoding entrypoints are the `load_first_mesh_*` / `load_*_skinned_*` functions.

- **Trap:** `astraweave-scripting` depends on `astraweave-asset`.
  **What's actually true:** It never `use`s it; the dependency is unused (Cargo.toml:19).

### Discarded-Result residue

- `astraweave-scene/src/streaming.rs:193` uses `let _ = Self::load_asset_data(asset_ref, assets_root).await;` — the per-asset byte-load Result is discarded in the streaming loop. This is a consumer-side pattern, surfaced here because it touches the cell-loader output. (CLAUDE.md flags `let _ =` on fallible asset I/O as a hazard; recorded as an open question in §11, not a recommendation.)
- `astraweave-asset/src/lib.rs:3375` (`invalidate_asset`) and `lib.rs:3579` (watcher channel send) use `.ok()` on `watch`/`mpsc` sends — intentional fire-and-forget on channels with possibly-dropped receivers.

---

## 7. Decision Log

### Decision: Deterministic path-based GUIDs (SHA-256, 128-bit)
- **Date:** [Reasoning not recovered from available sources] (code comment "Phase 2 foundations", lib.rs:1817)
- **Status:** Accepted.
- **Context:** Assets need a stable identity across runs for caching/dependency tracking.
- **Decision:** `guid_for_path` hashes the lowercased, `\`→`/`-normalized path with SHA-256 and keeps the first 16 bytes as hex (lib.rs:1820-1825).
- **Consequences:** GUIDs are case-insensitive and separator-insensitive (verified by test `guid_is_deterministic_and_case_insensitive`, lib.rs:1853). Renaming/moving a file changes its GUID. Content is not part of the GUID (content hash is tracked separately in `AssetMetadata.hash`).

### Decision: Cells serialized as RON, loaded async via `tokio::fs`
- **Date:** [Reasoning not recovered from available sources] (module docstring, cell_loader.rs:1-4)
- **Status:** Accepted.
- **Context:** World-Partition streaming needs human-readable, diffable cell files loadable off the main thread.
- **Decision:** `CellData` is `serde`-derived and (de)serialized with `ron`; async (`load_cell_from_ron`) and sync variants both exist (cell_loader.rs:161-220).
- **Consequences:** Matches the documented `load_cell_from_ron` async pattern in CLAUDE.md. Per-asset byte loading is decoupled from cell parsing (`load_asset` returns raw `Vec<u8>`, leaving decode to the consumer).

### Decision: Meshlet clustering delegated to `meshopt`; LOD simplification in-house QEM
- **Date:** [Reasoning not recovered from available sources] (code comments, nanite_preprocess.rs:333, 480)
- **Status:** Accepted (preprocess), but unwired.
- **Context:** Virtualized geometry needs meshlets + LOD chain.
- **Decision:** LOD-0 clustering uses `meshopt::clusterize::build_meshlets`; lower LODs use a hand-written quadric-error-metric edge-collapse simplifier (`QuadricError`, `simplify_mesh`).
- **Alternatives considered:** [Reasoning not recovered from available sources].
- **Consequences:** The simplifier uses a midpoint heuristic for the collapsed-vertex position rather than a full QEF solve (explicit comment, nanite_preprocess.rs:544-548).

### Decision: Asset transforms split into a separate `astraweave-asset-pipeline` crate
- **Date:** [Reasoning not recovered from available sources] (crate description: "Asset processing pipeline … texture compression, mesh optimization")
- **Status:** Accepted as a crate; not adopted by any consumer.
- **Context:** Offline asset processing (compression, optimization, CI validation).
- **Decision:** A standalone crate with feature-gated BC7 (`intel_tex`) / ASTC (`basis-universal`) compression, `meshopt` mesh optimization, and a CI validator.
- **Consequences:** Coexists with the independently-implemented compression/validation inside `tools/aw_asset_cli`. The two were not unified.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `guid_for_path` is deterministic and case/separator-insensitive. | Yes | Test `guid_is_deterministic_and_case_insensitive` (lib.rs:1853). |
| 2 | Meshlet generation requires `indices.len() % 3 == 0`; empty input yields empty output. | Yes | `generate_meshlets` guard (nanite_preprocess.rs:316-322) + tests. |
| 3 | `Mesh::new` rejects positions not multiple of 3, indices not multiple of 3, and out-of-bounds indices. | Yes | pipeline `mesh.rs:47-73` + `test_mesh_validation`. |
| 4 | BC7 compression requires width and height divisible by 4. | Yes | `compress_bc7` guard (texture.rs:70-76) + `test_bc7_requires_multiple_of_4`. |
| 5 | `ValidationReport.add_error` sets `passed = false`; warnings do not. | Yes | `validator.rs:46-54` + `test_validation_report`. |
| 6 | `CellData::add_asset` does not insert a duplicate `path`. | Yes | `cell_loader.rs:133-138` + `test_cell_add_asset_no_duplicates`. |
| 7 | Cell `AssetKind` and root `AssetKind` are distinct types and must be converted explicitly when bridged. | No | doc-only (this trace). |
| 8 | `astraweave-asset` forbids `unsafe`; `astraweave-asset-pipeline` forbids `unsafe`. | Yes | `#![forbid(unsafe_code)]` (lib.rs:1; pipeline lib.rs:1). |

---

## 9. Performance & Resource Profile

### Hot paths
- **Cell streaming load** (`cell_loader::load_cell_from_ron` + `load_asset`): runs from `WorldPartitionManager::update` as the camera moves. RON parse + raw byte read per cell; budget governed by `StreamingConfig` in `astraweave-scene`. Async via `tokio::fs` to stay off the simulation thread.

### Cold paths
- **glTF decode** (`gltf_loader::load_*`): on-demand at load/import time, not per-frame. Allocates full vertex/material/animation buffers.
- **Meshlet preprocess** (`nanite_preprocess`): offline/preprocess; `preprocess_mesh_async` offloads to `tokio::task::spawn_blocking` (nanite_preprocess.rs:691). QEM simplifier builds per-vertex quadrics + an edge `BinaryHeap` — O(E log E)-ish, intended for bake time, not runtime.
- **Pipeline transforms** (BC7/ASTC/optimize): offline bake only (would be, if wired).

### Resource ownership
- `AssetDatabase`: owns all `AssetMetadata`, path→GUID map, and forward/reverse dependency graphs (lib.rs:3270-3277). Typically shared as `Arc<Mutex<AssetDatabase>>` by `AssetWatcher` and `ResidencyManager`.
- `MeshletHierarchy` / `MeshData`: owned by the caller after load; this crate retains nothing.

---

## 10. Testing & Validation

- **Unit tests:** Extensive in-module `#[cfg(test)]` blocks: `lib.rs` (glTF/GUID/DB/watcher), `cell_loader.rs:315-841` (RON round-trip, magic validation, async load), `nanite_preprocess.rs:717-941` (AABB, meshlet gen, LOD hierarchy, QEM), pipeline `mesh.rs`/`texture.rs`/`validator.rs`.
- **Mutation testing:** `astraweave-asset/src/mutation_tests.rs` (~33 KB) — explicitly boundary/operator/constant mutation-killers. `astraweave-asset` is in the `sanitizers.yml` P1 array (per ARCHITECTURE_MAP.md:815).
- **Fuzzing:** `astraweave-asset/fuzz/` member present (declares the crate as a dep).
- **Benchmarks:** `astraweave-asset-pipeline` declares `benches/pipeline_adversarial` (Cargo.toml). (Render-side Nanite/residency benches live in `astraweave-render/benches/`, using their own types.)
- **Manual validation:** glTF loaders are exercised by `examples/visual_3d`, `hello_companion`, `veilweaver_demo`; meshlet preprocess by `examples/nanite_demo`.

---

## 11. Open Questions / Parked Decisions

- **Is `astraweave-asset-pipeline` intended to replace `tools/aw_asset_cli`'s in-house compression/validation, or are both intentional?** Currently the pipeline crate has zero production callers and the CLI duplicates its function. (Context: §6 coexisting abstractions.)
- **Why does `tools/aw_editor` depend on `astraweave-asset-pipeline` (`Cargo.toml:99`) with no `use`?** Either a planned-but-unwired integration or a stale dependency line.
- **Why does `astraweave-scripting` depend on `astraweave-asset` with no `use` (`Cargo.toml:19`)?** Declared-but-unused; candidate for the CLAUDE.md Key-Lesson-8 inventory.
- **Will the CPU `nanite_preprocess` output ever feed the render crate's GPU Nanite path?** No shared on-disk format connects `MeshletHierarchy` (RON) to `GpuMeshlet` today; the bridge, if intended, is not implemented.
- **Should `astraweave-scene/src/streaming.rs:193` discard the asset-load Result (`let _ = …`)?** CLAUDE.md flags this pattern on fallible asset I/O. Surfaced as a question for the scene/streaming owner.
- **Should the two `AssetKind` enums be unified or given an explicit conversion?** They diverge in variant sets and have no bridge function.
- **Pipeline default feature mismatch:** `astraweave-asset-pipeline` sets `default = ["astc"]` and `astc = ["basis-universal"]`, but `basis-universal` is declared `optional = true`. [NEEDS VERIFICATION] whether `default` builds pull it in correctly in all configurations.
  - *1.1 verification note (not a resolution — this is a parked decision):* The relevant lines are `Cargo.toml:45` (`default = ["astc"]`), `:47` (`astc = ["basis-universal"]`), `:18` (`basis-universal = { version = "0.3", optional = true }`). Because the `astc` feature lists the bare optional-dependency name `basis-universal` (the implicit-feature form, not `dep:basis-universal`), enabling `astc` activates the optional dependency, so a plain `cargo build -p astraweave-asset-pipeline` (default features) does pull in `basis-universal`. The marker is retained because no build matrix / CI run was executed to confirm behavior across all feature-flag combinations (e.g. `--no-default-features --features bc7`), per the read-only constraint of this pass.

---

## 12. Maintenance Notes

**Update this doc when:**
- A consumer wires (or unwires) `nanite_preprocess`, `AssetWatcher`, `ResidencyManager`, or `astraweave-asset-pipeline` — the wired/dormant status in §1, §4, §5 changes.
- The glTF loader public surface (`gltf_loader`) gains/loses entrypoints or material fields.
- `cell_loader` `CellData`/`AssetRef`/`AssetKind` shape changes (ripples into `astraweave-scene`).
- The two `AssetKind` enums are unified or bridged (§6, §8 #7).

**Verification process:**
- Re-run consumer grep: `grep -rl "astraweave-asset" */Cargo.toml` and `grep -rn "astraweave_asset::\|astraweave_asset_pipeline::" --include=*.rs .` to refresh §4 wired/dormant claims.
- Confirm `nanite_preprocess` / pipeline production callers with `rg '<fn>\(' --type rust -g '!*test*' -g '!*example*'`.
- Stamp the new commit hash and date in the Metadata table.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. `astraweave-asset-pipeline` has NO production caller — do not assume editing it changes engine behavior. The live bake path is `tools/aw_asset_cli` (which reimplements compression + validation).
2. `nanite_preprocess` is example-only and disconnected from the render crate's GPU Nanite (`GpuMeshlet`). Two different meshlet worlds.
3. There are TWO `AssetKind` enums in `astraweave-asset` (root vs `cell_loader`). Pick the right one; there's no conversion helper.
4. The wired, load-bearing surfaces are: `gltf_loader::load_*`, `cell_loader::{load_cell_from_ron, load_asset}`, and `AssetDatabase`.

**Files you'll most likely touch:**
- `astraweave-asset/src/lib.rs` (glTF loaders, `AssetDatabase`)
- `astraweave-asset/src/cell_loader.rs` (cell streaming — ripples into `astraweave-scene`)

**Files you should NOT touch without strong reason:**
- `astraweave-asset/src/nanite_preprocess.rs` — in-design, example-only; changes won't affect runtime rendering.
- `astraweave-asset-pipeline/src/*` — dormant crate; confirm a consumer before investing.

**Common mistakes when changing this system:**
- Editing `astraweave-asset-pipeline` expecting the editor/CLI to pick it up — they don't.
- Decoding a cell asset inside `cell_loader::load_asset` — by design it returns raw bytes; decode belongs to the consumer.
- Confusing `gltf_loader::load_gltf_bytes` (header validator) with a real loader.

---

## Appendix B: Historical context

The crate carries explicit "Phase 0 / Phase 2 foundations" markers (lib.rs:302, 316, 1812, 1817), indicating an incremental build-out where header-only validators and GUID/cache scaffolding landed before full decoders. The glTF decoders, cell loader, and `AssetDatabase` matured into wired runtime use; `nanite_preprocess` and `astraweave-asset-pipeline` were built to completeness and tested but never adopted by a runtime consumer, while `tools/aw_asset_cli` grew its own independent compression/validation. The `render → aw_asset_cli` Cargo dependency (`astraweave-render/Cargo.toml:61`; the `astraweave-asset` dep is the adjacent line 60 — verified at commit `7c29b8182`) is an unusual production→tool direction flagged in ARCHITECTURE_MAP.md (anomaly #2; ARCHITECTURE_MAP.md still cites the dep as Cargo.toml:60, which is the `astraweave-asset` line) and is orthogonal to this crate, but it is the reason the CLI's compression path — rather than `astraweave-asset-pipeline` — is reachable from the render-side asset story.
