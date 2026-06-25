---
schema_version: 1
trace_id: scene
title: "Scene System (Scene Graph, World Partition, Streaming, GPU Resources)"
description: "Scene — scene graph, world partition, cell streaming, GPU resource manager"
primary_crate: astraweave-scene
domain: physics-world
lifecycle_status: active
integration_status: partial
owns: [astraweave-scene]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Scene System (Scene Graph, World Partition, Streaming, GPU Resources)

## Metadata

| Field | Value |
|---|---|
| **System name** | Scene System (scene graph, world partition, cell streaming, GPU resource manager) |
| **Primary crates** | `astraweave-scene` (with `astraweave-asset` for cell loading, `astraweave-ecs` for the ECS scene-graph layer) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-24 |
| **Status** | Mixed. Some surfaces are wired into runtime consumers; the streaming + GPU-budget + PartitionedScene + ECS scene-graph layers are in-design-but-tested (production callers only in `examples/` and tests). See §6 and §11. |
| **Owner notes** | Derived from forensic data-flow analysis of `astraweave-scene` on 2026-06-24. The crate is a single member but is best understood as five loosely-coupled sub-layers (see §1). |

---

## 1. Executive Summary

**What this system does:**
`astraweave-scene` provides five loosely-coupled capabilities under one crate: (1) a value-type scene graph (`Transform` / `Node` / `Scene`) with hierarchical transform propagation; (2) an optional ECS scene-graph layer (parent/child components + transform/animation/bone systems) behind the `ecs` feature; (3) a grid-based `WorldPartition` of spatial `Cell`s for large open worlds; (4) an async `WorldPartitionManager` streaming layer that loads/unloads cells from `.ron` files based on camera position; and (5) a `GpuResourceBudget` GPU resource lifecycle manager that tracks per-cell wgpu buffers/textures against a memory budget.

**Why it exists:**
To keep memory bounded while allowing seamless exploration of large worlds (partition + streaming), and to provide a transform/hierarchy substrate for entities (scene graph).

**Where it primarily lives:**
- [`astraweave-scene/src/lib.rs`](../../astraweave-scene/src/lib.rs) — value-type scene graph (`Transform`, `Node`, `Scene`) + the `#[cfg(feature = "ecs")] mod ecs` scene-graph component/system layer
- [`astraweave-scene/src/world_partition.rs`](../../astraweave-scene/src/world_partition.rs) — `GridCoord`, `AABB`, `Frustum`, `Cell`, `WorldPartition`, `LRUCache` (in-memory data structures)
- [`astraweave-scene/src/streaming.rs`](../../astraweave-scene/src/streaming.rs) — `WorldPartitionManager` async load/unload (tokio)
- [`astraweave-scene/src/gpu_resource_manager.rs`](../../astraweave-scene/src/gpu_resource_manager.rs) — `CellGpuResources`, `GpuResourceBudget` (wgpu lifecycle)
- [`astraweave-scene/src/partitioned_scene.rs`](../../astraweave-scene/src/partitioned_scene.rs) — `PartitionedScene` glue tying `Scene` + `WorldPartition` + `WorldPartitionManager` + entity-cell maps
- [`astraweave-scene/src/error.rs`](../../astraweave-scene/src/error.rs) — `SceneError` / `SceneResult`

**Status note (read this before working here):**
Wiring is uneven across the five layers and this is the single most important fact for an agent. Verified workspace-wide grep (excluding `*test*`/`examples/`):
- **`Transform`** (value type) is wired — `astraweave-physics/src/ecs.rs:3` imports `astraweave_scene::Transform`.
- **`WorldPartition` / `Cell` / `GridCoord` (in-memory structs)** are wired — `astraweave-gameplay/src/veilweaver_slice.rs` and `veilweaver_slice_runtime/src/*` consume them as zone/cell data structures.
- **`WorldPartitionManager` (streaming), `GpuResourceBudget` / `CellGpuResources`, `PartitionedScene`, and the entire `#[cfg(feature = "ecs")]` scene-graph system layer** (`update_world_transforms`, `sync_scene_to_renderer`, `update_animations`, `compute_poses_stub`, `sync_bone_attachments`) have **zero non-test, non-example production callers**. Their only callers are in `examples/world_partition_demo`, `examples/veilweaver_slice_loader`, and the crate's own test/bench files. They are in-design-but-tested per CLAUDE.md Key Lesson 8.
- **`compute_poses_stub`** (`lib.rs:941-975`) is an explicit stub flagged in CLAUDE.md Documentation Hazards — it marks joint matrices dirty and resizes the buffer but performs no actual pose computation. See §6 and §11.

---

## 2. Authoritative Pipeline

The crate has two principal data flows. **Flow A** (scene graph / transforms) is independent of **Flow B** (partition + streaming + GPU). They share no runtime state. `PartitionedScene` is the only place that attempts to bridge `Scene` (Flow A) and `WorldPartition` (Flow B), and even there the scene graph is left as a default-constructed `Scene::new()` (`partitioned_scene.rs:69`) and not populated from cell data.

### Flow A — Scene graph transform propagation (two variants)

#### A1. Value-type immediate traversal (`Scene::traverse`)

```text
[Scene { root: Node }]   (lib.rs:282-285)
    │
    │ Scene::traverse(&mut f)              lib.rs:325-334
    ▼
[Depth-first walk]
    role: world = parent_matrix * node.transform.matrix()
    key data: (&Node, Mat4 world) handed to callback
    │
    ▼
[Caller-supplied closure consumes (node, world_matrix)]
```

#### A2. ECS retained scene-graph layer (`feature = "ecs"`)

```text
[CTransformLocal mutated by gameplay/editor]   lib.rs:377
    │
    │ mark_dirty_transforms(world)              lib.rs:711-734
    │   (change detection via each_changed + ChangeDetectionTick resource)
    ▼
[CDirtyTransform tags inserted]
    │
    │ update_world_transforms(world)            lib.rs:741-830
    │   Phase 1 bulk-read CTransformLocal + CParent
    │   Phase 2 CPU DFS from roots, parent_world * local_mat
    │   Phase 3 batch-write CTransformWorld
    │   Phase 4 batch-clear CDirtyTransform
    ▼
[CTransformWorld(Mat4) per entity]
    │
    │ sync_scene_to_renderer(world) -> Vec<RenderInstance>   lib.rs:842-888
    ▼
[Vec<RenderInstance { entity, world_transform, mesh_handle, material_index }>]
```

There is also an animation sub-pipeline in the same `ecs` module:
`update_animations` (lib.rs:896-936) → `CDirtyAnimation` → `compute_poses_stub` (lib.rs:941-975, **stub**) → `sync_bone_attachments` (lib.rs:979-1025).

### Flow B — World partition cell streaming

```text
[Camera Vec3 position]
    │
    │ WorldPartitionManager::update(camera_position)      streaming.rs:100-144
    ▼
[Desired cells computed]
    file: world_partition.rs:498-514  WorldPartition::cells_in_radius
    role: 2D ring of GridCoords within streaming_radius (y forced to 0)
    key data: Vec<GridCoord> desired
    │
    │ diff against active_cells / loading_cells           streaming.rs:110-124
    ▼
[to_load / to_unload sets, throttled by max_concurrent_loads]
    │
    │ start_load_cell(coord)                              streaming.rs:147-257
    │   - LRU fast path: if cached, mark Loaded synchronously (streaming.rs:149-161)
    │   - else mark Loading, tokio::spawn async file load
    ▼
[tokio task: load_cell_from_ron + load_asset]            streaming.rs:178-252
    file: astraweave-asset/src/cell_loader.rs (load_cell_from_ron, load_asset)
    role: read assets/cells/{x}_{y}_{z}.ron, map AssetKind -> AssetType
    key data: cell.entity_blueprints, cell.metadata, cell.assets; cell.state=Loaded
    │
    ▼
[Cell populated in WorldPartition (behind Arc<RwLock>)]
    │
    │ (optional, demo-only) PartitionedScene.on_cell_loaded(coord, CellData)   partitioned_scene.rs:121-154
    ▼
[CellEntities + entity_cells maps + SceneEvent queue]
```

GPU resources are a **separate, parallel** lifecycle (`GpuResourceBudget`) that is NOT invoked anywhere in `streaming.rs`. The unload path in `streaming.rs:306-336` only flips `CellState` and comments that GPU release is "in real implementation" — it does not call `GpuResourceBudget::unload_cell`.

### Stage-by-stage detail

#### Stage: `WorldPartition::cells_in_radius`
**File:** [`world_partition.rs:498-514`](../../astraweave-scene/src/world_partition.rs)
**Role:** Decide which cells the camera "wants" loaded.
**Inputs:** camera center `Vec3`, `radius` (world units).
**Outputs:** `Vec<GridCoord>` whose cell-center distance ≤ radius.
**Notes:** This is a **2D ring** — it iterates `dx, dz` only and forces `y = 0` (`world_partition.rs:505`). The richer 3D frustum culling in `Frustum::cells_in_frustum` (`world_partition.rs:286-314`) exists but is NOT used by the streaming `update` path; `update` calls `cells_in_radius`, not the frustum method.

#### Stage: `WorldPartitionManager::start_load_cell`
**File:** [`streaming.rs:147-257`](../../astraweave-scene/src/streaming.rs)
**Role:** Begin loading a cell, either from LRU cache (fast) or via a spawned tokio task that reads a `.ron` file.
**Inputs:** `GridCoord`.
**Outputs:** mutates `loading_cells`/`active_cells`, mutates the `Cell` state behind the `Arc<RwLock<WorldPartition>>`, emits `StreamingEvent`s.
**Notes (non-obvious behavior):**
- The async file path is **fire-and-forget**: `start_load_cell` spawns a task and returns `Ok(())` immediately (`streaming.rs:254-256`). The manager's `active_cells`/`loading_cells` are **not** updated when the spawned task completes — only the `Cell.state` inside the `RwLock` is. `finish_load_cell` (streaming.rs:276-289), which would move the coord from `loading_cells` to `active_cells`, is annotated `#[allow(dead_code)]` and is never invoked. The cell-load-fail handler `handle_load_failure` (streaming.rs:293-303) is likewise `#[allow(dead_code)]`.
- Asset bytes from `load_asset_data` are loaded then discarded (`let _ = Self::load_asset_data(...)`, `streaming.rs:193`) with a comment "fire and forget for now / In production, integrate with asset manager." This is a deliberate placeholder, not a wired asset pipeline. Verified: `load_asset_data` (streaming.rs:267) has exactly one call site (streaming.rs:193), where its `Result<Vec<u8>>` is dropped; the returned bytes are never stored on the `Cell` or surfaced to any consumer. The only consumer of streaming output (`examples/world_partition_demo`) reads `cell.assets` (the `AssetRef` metadata it itself pushed, main.rs:40-49), never the loaded bytes. No consumer expects the bytes. (The intended asset-manager hand-off remains an Open Question — see §11.)
- The hard-coded path template is `assets/cells/{x}_{y}_{z}.ron` (`streaming.rs:180-183`).

#### Stage: `astraweave-asset::cell_loader`
**File:** [`astraweave-asset/src/cell_loader.rs`](../../astraweave-asset/src/cell_loader.rs)
**Role:** Parse a cell `.ron` into `CellData { entities: Vec<EntityData>, assets: Vec<AssetRef>, metadata }`.
**Notes:** `streaming.rs` maps `cell_loader::AssetKind` → `world_partition::AssetType` (`streaming.rs:222-236`) and copies `EntityData` → `CellEntityBlueprint` (`streaming.rs:196-206`). See cross-link in §4.

#### Stage: `GpuResourceBudget` (parallel, not wired to streaming)
**File:** [`gpu_resource_manager.rs`](../../astraweave-scene/src/gpu_resource_manager.rs)
**Role:** Track per-cell wgpu buffer/texture memory against `max_memory_bytes` (default 500 MB, `gpu_resource_manager.rs:229`), and `enforce_budget` by unloading the cell furthest from the camera.
**Notes:** Self-contained. `upload_vertex_buffer`/`upload_index_buffer`/`upload_texture` create real wgpu resources; `enforce_budget` (`gpu_resource_manager.rs:254-266`) loops unloading the furthest cell until under budget. No caller in `streaming.rs` or `partitioned_scene.rs` drives it — it is invoked only by tests and would need an external orchestrator to participate in the streaming loop.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Transform** | Value type: translation (`Vec3`) + rotation (`Quat`) + scale (`Vec3`), with matrix/inverse/lerp helpers | `lib.rs:37-178` |
| **Node** | Value-type scene-graph node: name + `Transform` + `Vec<Node>` children (owned recursive tree, NOT entity-indexed) | `lib.rs:180-265` |
| **Scene** | Container holding a single root `Node` | `lib.rs:282-355` |
| **Cell** | A grid cell in the world partition: coord, `CellState`, entities, assets, bounds (`AABB`), entity blueprints, optional metadata | `world_partition.rs:344-395` |
| **GridCoord** | Signed `i32` 3D cell coordinate; `from_world_pos` floors `pos / cell_size` | `world_partition.rs:47-112` |
| **CellState** | `Unloaded` / `Loading` / `Loaded` / `Unloading` (`#[non_exhaustive]`) | `world_partition.rs:335-342` |
| **WorldPartition** | `HashMap<GridCoord, Cell>` + `GridConfig`; pure in-memory grid (no async) | `world_partition.rs:430-526` |
| **WorldPartitionManager** | Async streaming orchestrator owning `Arc<RwLock<WorldPartition>>`, LRU cache, active/loading sets, metrics, event listeners | `streaming.rs:61-381` |
| **PartitionedScene** | Glue: `Scene` + `Arc<RwLock<WorldPartition>>` + `WorldPartitionManager` + entity↔cell maps + `SceneEvent` queue | `partitioned_scene.rs:54-246` |
| **CellGpuResources** | Per-cell wgpu buffers/textures + tracked memory usage | `gpu_resource_manager.rs:15-206` |
| **GpuResourceBudget** | `HashMap<GridCoord, CellGpuResources>` + max/current byte budget | `gpu_resource_manager.rs:209-300` |
| **CTransformLocal / CTransformWorld** | ECS components: local transform vs computed world `Mat4` | `lib.rs:377,381` |
| **RenderInstance** | Output of `sync_scene_to_renderer`: entity + world matrix + mesh/material indices | `lib.rs:833-839` |

### Terms to NOT confuse

- **`Node` (value-type scene graph) vs ECS `CParent`/`CChildren` (retained scene graph):** These are two *different* hierarchy representations. `Node` is an owned recursive tree in `Scene` (`lib.rs:180-285`). The ECS layer uses entity-indexed `CParent`/`CChildren` components (`lib.rs:385-389`) with separate systems. They do not share data; nothing converts a `Scene`/`Node` tree into ECS components or vice versa in this crate.
- **`WorldPartition` (in-memory data) vs `WorldPartitionManager` (async streaming):** The former is a plain grid struct with no async; the latter is the tokio-driven loader that mutates a partition through an `Arc<RwLock>`. Consumers (`veilweaver_slice_runtime`, gameplay) use the **former** directly and do not use the manager.
- **`GridCoord` (scene) vs `PartitionCoord` (terrain) vs `ChunkCoord` (terrain voxel):** `astraweave-terrain/src/partition_integration.rs:30` defines its *own* `PartitionCoord` with the same fields and `from_world_pos`/`to_world_center` methods, explicitly commented "Re-exported for convenience (normally from astraweave-scene)" — but it is a re-implementation, not a re-export. See §6.
- **`AssetRef` (two definitions):** `world_partition::AssetRef` (`world_partition.rs:318-322`) and `astraweave_asset::cell_loader::AssetRef` are distinct types; `streaming.rs:219-237` converts between them.
- **"Cell" overloaded:** `Cell` (world_partition) is a streaming grid cell; `CellGpuResources` is its GPU-side companion keyed by the same `GridCoord`; `CellEntities`/`CellEntityBlueprint`/`CellData`/`CellMetadata` are all related but distinct types.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-asset` (cell loader) | `cell_loader::load_cell_from_ron(path)`, `cell_loader::load_asset(ref, root)` | `CellData`, asset bytes | Called in the spawned task in `streaming.rs:263,271`. Asset bytes currently discarded (`streaming.rs:193`). Cross-link: see [asset cell-loading] (no dedicated trace yet; lives in `astraweave-asset/src/cell_loader.rs`). |
| `astraweave-ecs` | `World::insert/get/get_mut/each_mut/each_changed/change_tick` | ECS component storage | The `feature = "ecs"` systems read/write components via these `astraweave_ecs::World` methods (`lib.rs:639-1025`). |
| Camera (caller-supplied) | `WorldPartitionManager::update(camera_position: Vec3)` | camera world position | Drives streaming. Caller is the host loop (currently only demos/tests). |
| Gameplay / worldgen authoring | direct construction of `WorldPartition` + `Cell` | zone cell data | `veilweaver_slice_runtime::VeilweaverSliceRuntime::new(config, partition)` (`veilweaver_slice_runtime/src/lib.rs:403`) takes a pre-built `WorldPartition`. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-physics` | `use astraweave_scene::Transform;` (`astraweave-physics/src/ecs.rs:3`) | `Transform` value type | Behind physics' `ecs` feature (`astraweave-physics/Cargo.toml:11,39`). This is the one clearly-wired engine consumer of a scene type. |
| `astraweave-gameplay` (Veilweaver slice) | `world_partition::{Cell, CellComponentView, GridCoord, AABB, ...}` (`veilweaver_slice.rs:2,274`) | cell data + component views | Reads `Cell::components_of_type` (`world_partition.rs:381-394`) to extract weave anchors / trigger zones from cell blueprints. |
| `veilweaver_slice_runtime` | `world_partition::{Cell, GridCoord, WorldPartition}` (`lib.rs:39`) | in-memory partition + cells | Wraps a `WorldPartition` as zone storage; does **not** use streaming/GPU/PartitionedScene. |
| `astraweave-terrain` | (none direct) — re-implements `PartitionCoord` (`partition_integration.rs:30`) | — | Terrain's partition integration does **not** depend on scene's `GridCoord`; it duplicates the coordinate type. See §6. |
| Renderer | `sync_scene_to_renderer -> Vec<RenderInstance>` (`lib.rs:842`) | render instances | **Not wired**: `astraweave-render` does not depend on `astraweave-scene` (verified: no `astraweave_scene` references in `astraweave-render`). `RenderInstance` is consumed only by scene's own tests. |

### Bidirectional / Coupled

- **`PartitionedScene` ↔ `WorldPartitionManager`:** `PartitionedScene` owns both an `Arc<RwLock<WorldPartition>>` and a `WorldPartitionManager` constructed over the *same* `Arc` (`partitioned_scene.rs:70-71`). `update_streaming` (`partitioned_scene.rs:89-118`) registers an event listener that spawns a tokio task per event to translate `StreamingEvent` → `SceneEvent`. Note: a fresh listener is added on **every** `update_streaming` call (`partitioned_scene.rs:94`), so listeners accumulate across calls. Verified: `add_event_listener` (partitioned_scene.rs:94) is inside the per-call `pub async fn update_streaming` body, not the constructor, so each call appends a new listener. Whether this accumulation is intended is parked in §11.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`src/lib.rs`](../../astraweave-scene/src/lib.rs) (value types) | `Transform`, `Node`, `Scene`, traversal | Active | `Transform` wired into `astraweave-physics`. `Scene::traverse` is the only fully-self-contained transform path. |
| [`src/lib.rs`](../../astraweave-scene/src/lib.rs) `mod ecs` (lib.rs:369-1026) | ECS scene-graph + animation components & systems | In-design-but-tested | Behind `feature = "ecs"`. `update_world_transforms`, `sync_scene_to_renderer`, `mark_dirty_transforms`, `update_animations`, `compute_poses_stub`, `sync_bone_attachments`, `SceneGraph::{attach,detach,reparent}` — **zero non-test production callers** (verified workspace grep). Tests in `src/mutation_tests.rs` and `tests/bone_attachment_integration.rs`. |
| `compute_poses_stub` (lib.rs:941-975) | Animation pose computation | Stub | Explicit stub (docstring lib.rs:938-940; flagged in CLAUDE.md Documentation Hazards). Marks `CJointMatrices.dirty=true` and resizes the matrices vector to `joint_count` of identity matrices; performs **no** actual pose math. |
| [`src/world_partition.rs`](../../astraweave-scene/src/world_partition.rs) | `GridCoord`, `AABB`, `Frustum`, `Cell`, `WorldPartition`, `LRUCache` | Active (data structures) | `WorldPartition`/`Cell`/`GridCoord`/`AABB`/`CellComponentView` consumed by gameplay + veilweaver runtime. `Frustum::cells_in_frustum` exists but is unused by the streaming path (which uses `cells_in_radius`). |
| [`src/streaming.rs`](../../astraweave-scene/src/streaming.rs) | `WorldPartitionManager`, `StreamingConfig`, `StreamingEvent`, `StreamingMetrics` | In-design-but-tested | Production callers only in `examples/world_partition_demo`, `examples/veilweaver_slice_loader`, and tests. Contains two `#[allow(dead_code)]` lifecycle helpers (`finish_load_cell`, `handle_load_failure`) never invoked. |
| [`src/gpu_resource_manager.rs`](../../astraweave-scene/src/gpu_resource_manager.rs) | `CellGpuResources`, `GpuResourceBudget`, `GpuMemoryStats` | In-design-but-tested | Self-contained wgpu lifecycle. No caller drives it from streaming. Callers: scene's own `tests/`. Cross-link: the render trace ([render_pipeline_material_system_shader_infrastructure.md](./render_pipeline_material_system_shader_infrastructure.md)) owns the engine's primary GPU resource management; this per-cell budget is a separate, parallel facility. |
| [`src/partitioned_scene.rs`](../../astraweave-scene/src/partitioned_scene.rs) | `PartitionedScene`, `CellEntities`, `SceneEvent`, `ScenePartitionExt` | In-design-but-tested | Production callers only in the two demos + tests. The contained `Scene` is default-constructed and never populated from cell data (`partitioned_scene.rs:69`); entity spawning is comment-only TODO (`partitioned_scene.rs:143-147,165-167`). Entity IDs are synthesized by bit-packing coord+index (`partitioned_scene.rs:133`), not from ECS spawn. |
| [`src/error.rs`](../../astraweave-scene/src/error.rs) | `SceneError`, `SceneResult` | In-design-but-tested (defined) | Re-exported at `lib.rs:32`. Verified: workspace-wide grep for `SceneError`/`SceneResult` finds usage only inside `error.rs`'s own `#[cfg(test)]` tests (error.rs:47-97); the names appear nowhere else in `astraweave-scene` or any other crate. No production path returns `SceneResult` — streaming/partition code uses `anyhow::Result`. Whether `SceneError` is meant to replace `anyhow` on scene paths is parked in §11. |
| [`src/mutation_tests.rs`](../../astraweave-scene/src/mutation_tests.rs) | Mutation-resistant test suite | Active (test-only) | `#[cfg(test)]` module (lib.rs:34-35). |
| [`WORLD_PARTITION.md`](../../astraweave-scene/WORLD_PARTITION.md) | In-crate design doc | Reference | Describes the partition/streaming design; uses a frustum-based mental model that the active `update` path does not fully realize (it uses `cells_in_radius`). Treat as design intent, cross-validate against code. |

**Status definitions used here:**
- **Active**: load-bearing, has ≥1 non-test/non-example production consumer.
- **In-design-but-tested**: compiles, has passing tests, but zero non-test/non-example production callers (CLAUDE.md Key Lesson 8 taxonomy).
- **Stub**: body is a placeholder/TODO; documented behavior not implemented.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Value-type scene graph (`Node` tree) | `lib.rs:180-355` | Active | `Scene::traverse` self-contained. |
| ECS retained scene graph (`CParent`/`CChildren` + systems) | `lib.rs:369-1026` | In-design-but-tested | Separate hierarchy model; no bridge to/from the `Node` tree. |
| Grid coordinate type | `world_partition::GridCoord` (`world_partition.rs:47`) vs `terrain::PartitionCoord` (`partition_integration.rs:30`) | Both Active in their crates | Terrain re-implements the coordinate type with identical fields/methods rather than depending on scene. Comment at `partition_integration.rs:28` says "normally from astraweave-scene" but it is a duplicate, not a re-export. |
| Cell GPU lifecycle | `gpu_resource_manager.rs` (per-cell budget) vs `astraweave-render` GPU resource management | Parallel | Scene's budget is not wired into streaming; render's pipeline (see render trace) is the engine's primary GPU path. |
| Streaming cull source | `WorldPartition::cells_in_radius` (used) vs `Frustum::cells_in_frustum` (defined, unused by `update`) | Mixed | The async `update` uses the radius ring; the frustum method is dead relative to streaming. |

### Naming collisions

- **`AssetRef`**: `world_partition::AssetRef` (path + `AssetType`) vs `astraweave_asset::cell_loader::AssetRef` (path + `AssetKind`). Converted explicitly in `streaming.rs:219-237`.
- **"Cell"**: `Cell` (grid cell, world_partition) vs `CellGpuResources` (gpu side) vs `CellEntities`/`CellEntityBlueprint` (partitioned_scene/world_partition) vs `CellData`/`CellMetadata` (asset crate). All keyed/related by `GridCoord` but distinct types.
- **"Transform"**: scene's value-type `Transform` is also imported and used by `astraweave-physics` (`ecs.rs:3`); ensure changes to `Transform`'s public shape consider that consumer.

### Known cognitive traps

- **Trap:** Assuming `WorldPartitionManager::update` populates `active_cells` after an async file load.
  **What's actually true:** The async path is fire-and-forget; only `Cell.state` inside the `RwLock` is updated by the spawned task. `active_cells`/`loading_cells` are advanced synchronously only on the LRU fast path (`streaming.rs:149-161`); the function that would advance them after async completion (`finish_load_cell`) is `#[allow(dead_code)]` and never called. A cell that loads from disk ends up `Loaded` in the partition but is never moved out of `loading_cells` in the manager. Verified: the only public surface for the manager's bookkeeping is `active_cells()` (streaming.rs:368) and the `StreamingMetrics.active_cells`/`loading_cells` counts populated in `update_metrics` (streaming.rs:342-343). No non-test caller depends on these for behavior: the sole streaming consumer (`examples/world_partition_demo`) reads `metrics.active_cells`/`loading_cells` only to print them (main.rs:221,244). Whether this bookkeeping *should* reconcile with disk-loaded cells is parked in §11.
- **Trap:** Reading `gpu_resource_manager.rs` and assuming the streaming system frees GPU memory on unload.
  **What's actually true:** `unload_cell` in `streaming.rs:306-336` only flips `CellState` and comments "in real implementation, release GPU resources." It does not touch `GpuResourceBudget`. The two are wired together by no production code.
- **Trap:** Treating `PartitionedScene` as the runtime scene used by the engine.
  **What's actually true:** Its `Scene` field is default-constructed and never populated; entity spawn/despawn are comment-only TODOs (`partitioned_scene.rs:143,165`). Only the two demos construct it.
- **Trap:** Reading the large comment block inside `CellGpuResources` (`gpu_resource_manager.rs:24-44`) as documentation.
  **What's actually true:** It is a stream-of-consciousness design note left in the struct body reasoning about how to track sizes; the resolved design is `texture_sizes: HashMap` + buffers reporting `.size()`. It is residue, not a spec.

---

## 7. Decision Log

### Decision: Two hierarchy representations (value-type `Node` tree + ECS components)
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (both present in active code; `ecs` layer feature-gated)
- **Context:** [Reasoning not recovered]
- **Decision:** Ship a serializable value-type `Scene`/`Node` tree (lib.rs:180-355) alongside an entity-indexed ECS scene-graph layer (lib.rs:369-1026) behind `feature = "ecs"`.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Two ways to express hierarchy with no conversion path between them. The value-type path is wired (via `Transform` into physics); the ECS path is in-design-but-tested.

### Decision: Bulk read → compute → write pattern in `update_world_transforms`
- **Date:** [Reasoning not recovered — rationale visible in code comments]
- **Status:** Accepted (lib.rs:736-830)
- **Context:** Doc-comment at `lib.rs:736-740` states the three-phase pattern is "3-5× faster than interleaved get/insert per entity" — this matches CLAUDE.md Key Lesson 1 ("Batching > Scattering").
- **Decision:** Phase 1 bulk-read components, Phase 2 pure-CPU DFS, Phase 3 batch-write `CTransformWorld`, Phase 4 batch-clear dirty flags. Roots and children sorted by `entity.id()` for deterministic traversal (`lib.rs:778-783`).
- **Alternatives considered:** Interleaved per-entity get/insert (rejected for cache-locality cost per the comment).
- **Consequences:** Deterministic, cache-friendly transform update consistent with the engine's batching guidance.

### Decision: `compute_poses_stub` left as a stub
- **Date:** [Reasoning not recovered]
- **Status:** Accepted as stub (lib.rs:938-975)
- **Context:** Docstring (lib.rs:938-940): "This is a stub - full implementation requires AnimationClip from render crate." Scene does not depend on render (would be a layering inversion), so the actual pose computation is deferred to renderer integration.
- **Decision:** The stub marks `CJointMatrices` dirty and resizes the matrices buffer; real sampling/joint-matrix math happens elsewhere (renderer integration) when implemented.
- **Alternatives considered:** [Reasoning not recovered — depending on render crate would invert the dependency direction.]
- **Consequences:** Animation pipeline in scene is structurally present but inert; listed in CLAUDE.md Documentation Hazards.

### Decision: Streaming uses `Arc<RwLock<WorldPartition>>` with fire-and-forget tokio tasks
- **Date:** [Reasoning not recovered]
- **Status:** Accepted (streaming.rs)
- **Context:** [Reasoning not recovered]
- **Decision:** `WorldPartitionManager` owns the partition behind a tokio `RwLock` and spawns per-cell async load tasks that mutate cell state through the shared lock.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** The manager's own `active_cells`/`loading_cells` bookkeeping and the partition's `Cell.state` can diverge for disk-loaded cells (the reconciling `finish_load_cell` is dead code). Asset bytes are loaded then discarded pending asset-manager integration.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `GridCoord::from_world_pos` floors `pos / cell_size` per axis (negative positions round toward −∞) | Yes | Tests `test_grid_coord_from_world_pos*` (`world_partition.rs:600-621`) |
| 2 | `update_world_transforms` produces deterministic output (roots and children sorted by `entity.id()`) | Yes | Sorting at `lib.rs:778-783`; mutation tests `mutation_update_world_transforms_*` (`mutation_tests.rs:2406+`) |
| 3 | `LRUCache` evicts the least-recently-touched coord when over capacity, and never exceeds capacity | Yes | `LRUCache::touch` (`world_partition.rs:544-555`); tests `test_lru_cache_eviction` (`world_partition.rs:1092-1105`) |
| 4 | `CellGpuResources` memory accounting: re-uploading the same `asset_id` subtracts the old size before adding the new | Yes | `upload_*` insert-returns-old logic (`gpu_resource_manager.rs:84-88,112-116,173-177`); tests in `tests/memory_accounting_tests.rs` |
| 5 | `GpuResourceBudget::enforce_budget` unloads cells in furthest-from-camera order until `current_usage ≤ max_memory_bytes` or no cells remain | Yes | `enforce_budget` loop (`gpu_resource_manager.rs:254-266`); test `test_enforce_budget_multiple_unloads_until_under` (`gpu_resource_manager.rs:524-538`) |
| 6 | `WorldPartition.assign_entity_to_cell` is idempotent per entity within a cell (no duplicate entity IDs) | Yes | `contains` guard (`world_partition.rs:465`); analogous to `CellEntities::add_entity` dedup (`partitioned_scene.rs:31-35`) |
| 7 | The crate compiles with `#![forbid(unsafe_code)]` (lib.rs:1) — no `unsafe` anywhere in scene | Yes | Compiler (`#![forbid(unsafe_code)]`) |

---

## 9. Performance & Resource Profile

### Hot paths
- **`update_world_transforms`** (lib.rs:741-830): runs per frame in the intended design (PRESENTATION/POST_SIMULATION). Early-outs when nothing is dirty (lib.rs:742-754). Uses bulk read/compute/write to stay cache-friendly. Currently no production caller, so the per-frame cost is hypothetical.
- **`WorldPartitionManager::update`** (streaming.rs:100-144): per-frame (or per-camera-move). Acquires the partition read lock, computes `cells_in_radius`, diffs sets, may acquire the write lock and spawn tokio tasks. Cost scales with `(streaming_radius / cell_size)²` candidate cells.

### Cold paths
- **Cell disk load** (`streaming.rs:178-252`): runs on a spawned tokio task at cell activation, off the main loop. Reads a `.ron` and (currently discarded) asset bytes.
- **GPU upload/unload** (`gpu_resource_manager.rs`): at cell activation/budget-enforcement, not per frame.

### Resource ownership
- **`WorldPartition`**: owned either directly by a consumer (`veilweaver_slice_runtime`) or behind `Arc<RwLock>` when a `WorldPartitionManager`/`PartitionedScene` is used. Lifetime = world/zone lifetime.
- **`CellGpuResources` (wgpu `Buffer`/`Texture`)**: owned per `GridCoord` in `GpuResourceBudget.cells`. Dropping clears GPU memory (`unload_all`, `gpu_resource_manager.rs:183-190`). Default budget 500 MB.
- **`Arc<RwLock<WorldPartition>>`**: shared between a `PartitionedScene` and its `WorldPartitionManager` (same Arc).

---

## 10. Testing & Validation

- **Unit tests (inline `#[cfg(test)]`):** extensive — `lib.rs` (~70 transform/node/scene tests), `world_partition.rs` (GridCoord/AABB/Frustum/WorldPartition/LRUCache), `streaming.rs` (config/metrics/events/manager fast-path), `gpu_resource_manager.rs` (budget/eviction/memory accounting), `partitioned_scene.rs` (entity tracking/events), `error.rs`.
- **Mutation-resistant suite:** [`src/mutation_tests.rs`](../../astraweave-scene/src/mutation_tests.rs) (`#[cfg(test)] mod mutation_tests`, lib.rs:34-35) — covers `SceneGraph::attach/detach/reparent`, `update_world_transforms`, `sync_scene_to_renderer`, `update_animations`, `compute_poses_stub`, `sync_bone_attachments`. Referenced by `docs/current/MUTATION_TESTING_REMEDIATION_REPORT.md` and Phase 10A scene completion docs.
- **Integration tests (`astraweave-scene/tests/`):** `streaming_integration.rs`, `unit_tests.rs`, `bone_attachment_integration.rs`, `memory_accounting_tests.rs`, `mutation_resistant_comprehensive_tests.rs`.
- **Benchmarks:** [`benches/scene_partition_streaming.rs`](../../astraweave-scene/benches/scene_partition_streaming.rs) (criterion, declared in `Cargo.toml:27-29`).
- **Miri:** N/A — crate is `#![forbid(unsafe_code)]` (lib.rs:1).
- **Caveat:** Heavy test coverage on the streaming/GPU/PartitionedScene/ECS-systems layers does NOT imply they are wired (CLAUDE.md Key Lesson 8: "Wired beats tested"). Their only non-test callers are the two `examples/`.

---

## 11. Open Questions / Parked Decisions

- **Is the streaming layer (`WorldPartitionManager`), the GPU budget (`GpuResourceBudget`), `PartitionedScene`, and the ECS scene-graph/animation systems intended to be wired into a runtime, or are they reference/demo scaffolding?** Verified 2026-06-24: zero non-test/non-example production callers for any of these. Resolving this determines whether they should be marked Active (after wiring), kept as demo references, or pruned.
- **Should `WorldPartitionManager`'s active/loading bookkeeping reconcile with disk-loaded cells?** `finish_load_cell` and `handle_load_failure` are `#[allow(dead_code)]` (`streaming.rs:275,292`); the async path never advances `active_cells`. Is this intentional (consumers poll `Cell.state` via `partition.get_cell`) or a gap?
- **`PartitionedScene::update_streaming` adds a new event listener on every call** (`partitioned_scene.rs:94`), so listeners accumulate. Is this intended, or should the listener be registered once at construction?
- **Asset bytes loaded then discarded** (`streaming.rs:193`, "fire and forget for now / integrate with asset manager"). What is the intended hand-off to an asset manager, and which one?
- **`SceneError`/`SceneResult` vs `anyhow::Result`:** the typed error exists and is tested, but streaming/partition code returns `anyhow::Result`. Is `SceneError` meant to replace `anyhow` on the fallible scene paths, or is it for a future API surface?
- **Terrain's `PartitionCoord` duplicates scene's `GridCoord`** (`partition_integration.rs:30`). Should terrain depend on `astraweave-scene::world_partition::GridCoord` instead of re-implementing it, or is the duplication deliberate to avoid a terrain→scene dependency?
- **`compute_poses_stub`:** when (if) is the renderer-integrated pose computation expected to land, and where will it live given scene must not depend on render?
- **Cargo features `ecs` and `world-partition` have empty feature lists** (`Cargo.toml:24-25`) while `astraweave-ecs`/`astraweave-asset`/`wgpu` are unconditional deps. The `#[cfg(feature = "ecs")]` gates the ECS *module*, but the dep is always compiled. Is the intent to make those deps optional behind the features? [NEEDS VERIFICATION of intended feature semantics.] (Mechanics verified 2026-06-24: `ecs` feature gates `pub mod ecs` — `#[cfg(feature = "ecs")] pub mod ecs` at lib.rs:369 with a `#[cfg(not(feature = "ecs"))]` empty-stub module at lib.rs:1028-1031. `world-partition` gates nothing — grep finds no `#[cfg(feature = "world-partition")]` sites in the crate. `astraweave-ecs`/`astraweave-asset`/`wgpu`/`tokio` are unconditional in Cargo.toml:12-15 and not declared `optional`, so they compile regardless of feature selection. The remaining open item is purely the *intended* semantics, which is Andrew's call.)

---

## 12. Maintenance Notes

**Update this doc when:**
- Any Active file in §5 changes public shape (especially `Transform`, since `astraweave-physics` consumes it).
- A streaming/GPU/PartitionedScene/ECS-system surface gains its first real production caller (move it from In-design-but-tested to Active in §5/§6, and update §1 status note).
- `compute_poses_stub` is implemented (update §6/§7/§11).
- A §11 open question is resolved (move the resolution into the relevant section and delete the question).
- Terrain stops duplicating `GridCoord` (update §6 conflict map).

**Verification process:**
- Re-run the wired-vs-dormant grep: `rg 'WorldPartitionManager::new|GpuResourceBudget::|CellGpuResources::|PartitionedScene::new|sync_scene_to_renderer|update_world_transforms' --type rust -g '!*test*' -g '!examples/*'`. Currently expected: zero matches (all callers are examples/tests).
- Confirm `astraweave-physics/src/ecs.rs` still imports `astraweave_scene::Transform`.
- Spot-check the Flow B diagram in §2 against `streaming.rs::update` and `start_load_cell`.
- Stamp the new commit hash and date in §0 after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **The crate is five loosely-coupled layers, not one system.** Know which layer your change touches. The scene-graph value types and the partition data structs are wired; the streaming/GPU/PartitionedScene/ECS-system layers are in-design-but-tested.
2. **`Transform` is consumed by `astraweave-physics`.** Changing its public shape is a cross-crate change — check `astraweave-physics/src/ecs.rs`.
3. **Streaming does not free GPU memory and does not reconcile `active_cells` for disk-loaded cells.** Don't assume the two halves (`streaming.rs` and `gpu_resource_manager.rs`) are connected — they aren't, in production code.
4. **`compute_poses_stub` is a stub.** It resizes/dirties joint matrices but computes no poses. Do not treat it as a working animation system.
5. **`Frustum::cells_in_frustum` is unused by streaming** — `update` uses the 2D `cells_in_radius` ring. Don't "fix" the frustum method expecting it to affect streaming.

**Files you'll most likely touch:**
- `astraweave-scene/src/world_partition.rs` (partition data structures — the wired part)
- `astraweave-scene/src/lib.rs` (`Transform`/`Node`/`Scene` value types — the wired part)
- `astraweave-scene/src/streaming.rs` (if wiring the streaming loop)

**Files you should NOT touch without strong reason:**
- The large comment block in `gpu_resource_manager.rs:24-44` — it's design residue, not a contract; deleting it is harmless but don't treat it as a spec.
- `compute_poses_stub` — coordinate with whoever owns renderer-side animation before implementing.

**Common mistakes when changing this system:**
- **Mistake:** Adding features to `PartitionedScene` assuming it's the runtime scene. **Why wrong:** its `Scene` is empty and entity spawn is a TODO; only demos use it.
- **Mistake:** Wiring `GpuResourceBudget` into streaming inside `astraweave-scene` and assuming the renderer will pick it up. **Why wrong:** `astraweave-render` does not depend on `astraweave-scene`; there is no consumer of `RenderInstance`/`CellGpuResources` in the render crate today.
- **Mistake:** Editing terrain's `PartitionCoord` to "match" scene's `GridCoord` without surfacing the duplication decision (§6/§11).

---

## Appendix B: Historical context

The partition/streaming system originated in a "World Partition" milestone (see archived `docs/archive/WORLD_PARTITION_COMPLETE.md`, `WORLD_PARTITION_COMPLETION.md`, `WORLD_PARTITION_STATUS.md`, and the in-crate [`WORLD_PARTITION.md`](../../astraweave-scene/WORLD_PARTITION.md)). The ECS scene-graph + skeletal-animation component layer (`CSkeleton`, `CAnimator`, `CJointMatrices`, bone attachment) carries "Phase 2 Task 5" markers in the source (lib.rs:415-417) and corresponds to the archived `docs/archive/PHASE2_TASK5_*` reports. The mutation-resistant test suites and edge-case coverage were added during the Phase 10A coverage/mutation campaign (`docs/current/PHASE_10A_DAY_2_ASTRAWEAVE_SCENE_COMPLETE.md`, `docs/current/MUTATION_TESTING_REMEDIATION_REPORT.md`), which explains the high test density relative to the system's production wiring. Per CLAUDE.md Key Lesson 8, that density is a coverage artifact, not evidence of runtime integration.
