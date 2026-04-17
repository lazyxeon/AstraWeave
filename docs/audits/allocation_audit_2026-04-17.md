# Allocation Audit — 2026-04-17

**Scope**: Forensic audit of memory allocation strategies across the AstraWeave workspace (55+ crates, ~225k LoC surveyed in the crates most relevant to allocation hotspots).
**Method**: Static analysis via `rg` across all `Cargo.toml` and `.rs` files. No runtime profiling.
**Status**: Discovery + analysis. Phase 3 lists ranked recommendations; no changes made.

---

## Executive Summary

AstraWeave uses the **default Rust/system allocator** throughout. It pulls in **no external arena, bump, slab, slot-map, or GPU sub-allocator crate** anywhere in the workspace. All pool-like structures are hand-rolled in a small number of crates. The unsafe allocation surface is tightly concentrated in `astraweave-ecs/src/blob_vec.rs` (archetype-backing type-erased storage) and is the only place the engine calls `std::alloc::{alloc, dealloc, realloc}` directly. Tracy profiling is wired in (feature-gated, zero-cost when off), but its allocation-tracking macros (`alloc!`, `free!`) are defined and never called from production code. A GPU *accounting* layer (`GpuMemoryBudget`) exists but does not sub-allocate — it only records events when budgets are exceeded. A GPU *staging ring* exists and is the only real frame-scoped allocator in the codebase.

The practical consequence: most allocation goes through `malloc`, and there is no measurement of how much. Several hot paths (GOAP A* search, per-frame instance/light list building, ECS parallel group computation) allocate every time they run, with no scratch reuse.

---

## Phase 1 — Discovery

### 1.1 Global allocator

**Finding**: The default Rust allocator (system `malloc`) is used workspace-wide. No crate installs a replacement in any production binary.

Evidence:
- **Only one `#[global_allocator]` attribute** in the entire workspace, and it is test-only:
  - `astraweave-ecs/tests/zero_alloc_tests.rs:21-23` installs `CountingAlloc` behind `#[cfg(feature = "alloc-counter")]`.
- **No `mimalloc`, `jemallocator`, `snmalloc`, `rpmalloc`, `tcmalloc`, or `dlmalloc` dependency** in any production-path `Cargo.toml`. A single hit exists:
  - `crates/astraweave-blend/Cargo.toml:77,79` declares `tikv-jemallocator = "0.6"` as an *optional* dep gated behind the `jemalloc` feature and is only used from benchmark code. This feature is not activated by any `[features]` section in any other crate I searched. Even if enabled, it only affects the `astraweave-blend` library itself.
- Workspace root `Cargo.toml` (lines 1-250) defines no allocator dep, no allocator feature, and no profile setting that would change one.
- Per-binary: I searched all `[[bin]]` entries and all `fn main()` sites found via `rg`. No binary crate (editor, hello_companion, unified_showcase, aw_build, aw_release, aw_demo_builder, etc.) installs a global allocator.

So: **every allocation in the shipping engine goes through the OS `malloc`** (glibc `malloc` on Linux, `HeapAlloc`-backed `msvcrt malloc` on the Windows target this repo is configured for).

### 1.2 Arena / bump allocators

**Finding**: None. Neither as dependencies nor as hand-rolled equivalents.

Evidence:
- `rg` against all `Cargo.toml` files for `bumpalo`, `typed-arena`, `id-arena`, `generational-arena`, `generational_arena` → **0 matches**.
- `rg` against all `.rs` files for struct names matching `Arena`, `Bump`, `Region`, `ScratchAllocator`, `FrameAllocator`, `Scratch` → all hits are domain terms (boss arena, navmesh region, code comment "Bump resource generation", biome pack, etc.), not memory arenas. Representative false-positives I verified:
  - `veilweaver_slice_runtime/src/walkthrough.rs:90` — "Boss Arena Entry" (game content)
  - `astraweave-render/src/renderer.rs:3909` — comment "Bump resource generation so all cached bind groups rebuild" (refers to `Generation` counter, not allocator)
  - `astraweave-render/src/clustered_megalights.rs:429` — "Scratch buffer for block scan's own block_sums output" (GPU compute buffer, pre-allocated once at construction)
- No per-frame CPU scratch arenas in render, physics, fluids, terrain, or the AI pipeline.

### 1.3 Pool allocators

**Finding**: No external pool crates. Several hand-rolled pools, each implementing its own recycle scheme.

Evidence — external crates absent:
- `rg` against `Cargo.toml` for `slotmap`, `thunderdome`, `\bslab\b` → **0 matches**.

Evidence — hand-rolled pools (ordered by centrality):

1. **`EntityAllocator`** — generational-index allocator backed by `Vec<u32>` free list and `Vec<u32>` generations.
   - Definition: `astraweave-ecs/src/entity_allocator.rs:171-186`.
   - Algorithm: LIFO free list in `free_list: Vec<u32>`, per-slot generation counter in `generations: Vec<u32>`, next-id counter `next_id: u32`. O(1) spawn/despawn.
   - `Entity` is `(u32 id, u32 generation)` = 8 bytes, `entity_allocator.rs:53-57`.

2. **`BlobVec`** — type-erased contiguous component storage (Bevy-style).
   - Definition: `astraweave-ecs/src/blob_vec.rs:26-38`.
   - Allocates via raw `std::alloc::alloc/dealloc/realloc` with `Layout` (`blob_vec.rs:8,111-146,414-429`).
   - This is the only place in the workspace that calls `alloc::alloc` directly.
   - Kani proofs at `astraweave-ecs/src/blob_vec_kani.rs`.

3. **`SparseSet`** — O(1) Entity→dense-index lookup (EnTT/Flecs pattern).
   - Definition: `astraweave-ecs/src/sparse_set.rs:22-30`.
   - Backed by `Vec<Option<usize>>` + `Vec<Entity>`. Grows via `resize`.

4. **`Archetype`** component storage — has two modes:
   - Box mode (legacy): `HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>` at `astraweave-ecs/src/archetype.rs:78`.
   - BlobVec mode: `Option<HashMap<TypeId, BlobVec>>` at `archetype.rs:86-90` (lazy-init to avoid cost when not used).

5. **`SphMemoryPool`** — SoA `Vec<f32>` buffers for SPH particle arrays.
   - Definition: `astraweave-fluids/src/simd_ops.rs:1475-1496`.
   - 12 parallel `Vec<f32>` (positions/velocities/forces XYZ + density/pressure/mass).
   - `ensure_capacity` grows via `next_power_of_two()` (`simd_ops.rs:1549`).

6. **`StagingRing`** — per-frame GPU ring buffer (the only real frame-scoped sub-allocator in the engine).
   - Definition: `astraweave-render/src/staging_ring.rs:52-67`.
   - Backed by a single `wgpu::Buffer` of `DEFAULT_RING_SIZE = 4 * 1024 * 1024` bytes (`staging_ring.rs:24`).
   - Tracks `MAX_FRAMES_IN_FLIGHT = 3` frame records (`staging_ring.rs:27`).
   - Uniform-aligned at `MIN_UNIFORM_ALIGN = 256` (`staging_ring.rs:21`).
   - Instantiated once in the renderer: `renderer.rs:1115-1118`, advanced per-frame at `renderer.rs:4676` and `renderer.rs:5182`.

7. **`WeatherFx`** particle pool.
   - `pool: Vec<Particle>` with `active_count` swap-remove pattern at `astraweave-render/src/effects.rs:17-22`.
   - Pre-sized via `Vec::with_capacity(max)` at `effects.rs:54`.

8. **`WgpuFluid`** (SPH GPU system) — GPU-side particle buffer with CPU free-list mirror.
   - `free_list: Vec<u32>` at `astraweave-fluids/src/lib.rs:314`.
   - GPU buffer `particle_flags: wgpu::Buffer` at `lib.rs:311` sized to `max_particles` capacity.
   - CPU mirrors `particle_positions: Vec<[f32; 3]>` (line 320) and `particle_active: Vec<bool>` (line 322) for despawn-region checks.

9. **`ResidencyManager`** — asset LRU residency, labelled as a placeholder.
   - Definition: `astraweave-render/src/residency.rs:8-15`.
   - `loaded_assets: HashMap<String, ResidencyInfo>` + `lru_queue: VecDeque<String>`.
   - `ResidencyInfo.gpu_handle: Option<String>` is a literal `format!("gpu_{}", guid)` string (`residency.rs:85`), not a real GPU handle. This module does not hold real GPU resources today.

10. **`TextureStreamingManager`** — the actual texture residency system.
    - Definition: `astraweave-render/src/texture_streaming.rs:76-96`.
    - `assets: HashMap<AssetId, AssetState>` + `lru_queue: VecDeque<AssetId>` + `load_queue: BinaryHeap<LoadRequest>` + `pending_ids: HashSet<AssetId>` + `result_rx: mpsc::Receiver<LoadResult>`.

11. **`CellGpuResources`** — per-world-partition-cell GPU resource map.
    - Definition: `astraweave-scene/src/gpu_resource_manager.rs:15-51`.
    - `HashMap<AssetId, Buffer>` × 2 (vertex, index) + `HashMap<AssetId, Texture>` + `HashMap<AssetId, usize>` for texture sizes.
    - Allocates via `device.create_buffer_init` per upload (`gpu_resource_manager.rs:75`).

12. **`CachedBindGroup`** — generation-based bind group cache.
    - Definition: `astraweave-render/src/bind_group_cache.rs:26-29`.
    - Stores `Option<wgpu::BindGroup>` + `Generation: u64`. Rebuilds lazily when generation advances.

### 1.4 `Vec` / `Box` / `HashMap` patterns in hot paths

I categorised allocations by lifetime (persistent / per-frame / cfg-gated) within each hot system.

**Render submit path (`Renderer::render`, `renderer.rs:4658`)**:

- `enc = self.device.create_command_encoder(...)` — per-frame; cheap per wgpu design (command buffer resource).
- `self.staging_ring.begin_frame()` — bumps ring cursor, no allocation.
- **`bin_lights_cpu(...)`** at `renderer.rs:4719` / definition `clustered.rs:34-108`: allocates **four `Vec<u32>` per call** — `counts` (line 50), `offsets` (line 100), `indices` (line 107), `cursors` (line 108). Called every frame when `self.point_lights` is non-empty.
  - Mitigation in flight: `clustered_megalights.rs` is a GPU replacement noted as "Replaces CPU bin_lights_cpu() (0.5-2ms) with GPU dispatch (<0.1ms)" (`clustered_megalights.rs:4-5`). Feature-gated behind `megalights` with CPU fallback.
- **`glights: Vec<GpuLight> = ...collect()`** at `renderer.rs:4733-4748` — per-frame `collect()` of every point light.
- **`build_visible_instances()`** at `renderer.rs:6032-6069` — frustum-cull path, allocates `Vec::with_capacity(self.instances.len())` at line 6055 each frame, returns by value.
- `update_instances(&mut self, instances: &[Instance])` at `renderer.rs:4408-4454`:
  - Line 4409 `self.instances.clear(); self.instances.extend_from_slice(instances)` — reuses capacity (good).
  - Lines 4416-4439 `let raws: Vec<InstanceRaw> = self.instances.iter().map(...).collect()` — per-call allocation.
  - Line 4444 `self.instance_buf = self.device.create_buffer(...)` — only when size exceeds current buffer.
- 58 total `create_buffer` / `create_texture` calls in `renderer.rs` (`rg` count); most are in the constructor (`renderer.rs:1095-3400`-ish), which runs once.
- `create_bind_group` count across `astraweave-render/src`: **345 occurrences across 61 files** (`rg --count`), but many are in tests and archive dirs. Per-frame creation is mitigated by `CachedBindGroup` (`bind_group_cache.rs`) in passes that opt in — whether every pass opts in was not verified.

**Physics step (`PhysicsWorld::step_internal`, `astraweave-physics/src/lib.rs:1070-1100`)**:

- `self.pipeline.step(...)` — delegates to Rapier3D; Rapier has internal pools/broadphase state. The call itself constructs no new `Vec`/`Box` from our code.
- `self.apply_buoyancy_forces()` at line 1078 — not inspected for this audit; likely iterates bodies.
- `self.query_pipeline.update(&self.colliders)` at line 1099 — Rapier internal.
- Total `Vec::new / Vec::with_capacity / HashMap::new / Box::new` in `astraweave-physics/src`: **104 occurrences across 12 files** (most concentrated in `ragdoll.rs:31`, `destruction.rs:12`, `cloth.rs:12`). These are not all per-frame; ragdoll/destruction/cloth allocate on construction/event, not every step.

**AI planning paths**:

- `AIArbiter::update` at `astraweave-ai/src/ai_arbiter.rs:431` clones `plan.steps[step_index]` per LLM execution step.
- `AIArbiter::request_llm_replan` at `ai_arbiter.rs:609`: `snap.clone()` into `generate_plan_async`. Only fires when LLM cooldown has elapsed (`ai_arbiter.rs:600-602`), not per AI tick.
- **GOAP A\* search** (`astraweave-ai/src/goap/planner.rs:225-304`, `plan_direct`):
  - `BinaryHeap::new()` + `HashSet::new()` at line 226-227 per plan call.
  - Inside the while loop (line 241-295):
    - `closed_set.insert(current.state.clone())` at line 269 — `WorldState` clone per unique node.
    - `new_state = current.state.clone()` at line 277 — state clone per neighbour.
    - `new_path = current.path.clone()` + `action.name().to_string()` at line 283-284 — `Vec<String>` clone + `String` allocation per neighbour.
    - `PlanNode { state: new_state.clone(), path: new_path, ... }` at line 286-292 — another state clone into the node.
  - 3-4 allocations per expansion × up to `max_plan_iterations = 10000` (`planner.rs:60-61`). This is the single largest-volume non-rendering allocation site I identified.
  - `plan_sequential` (line 161-178) and `plan_multiple_goals` (line 313-341) add `.to_vec()`, `start.clone()`, `combined_plan.extend(sub_plan)` on top.
- Total hot-path allocation count in `astraweave-ai/src/goap/planner.rs`: **48 occurrences** of `Vec::new / Box::new / .clone() / to_string / format!`.

**ECS schedule tick (`ParallelSchedule::run`, `astraweave-ecs/src/parallel.rs:256-277`)**:

- Line 265: `let groups = Self::build_groups(&stage.systems)` — `Vec<Vec<usize>>` rebuilt every tick for every stage. `build_groups` (line 230-252) greedy-colors systems each call. This is a per-tick allocation proportional to system count × average group size.

**Fluids**:

- `SphMemoryPool` handles allocation up-front; per-step reuse is fine once `ensure_capacity` stabilises.
- `astraweave-fluids/src/lib.rs` — `WgpuFluid::spawn_particles` at line 879 pops from `free_list` (reuse), `despawn_particles` at line 993 pushes (reuse). Good.
- `astraweave-fluids` total Vec-new-style count: **116 occurrences across 23 files**; concentrated in `simd_ops.rs:31` and `editor.rs:26`. Not all per-step.

**Terrain**:

- `astraweave-terrain` Vec-new-style count: **119 occurrences across 28 files**. Heaviest in `meshing.rs:8`, `noise_gen.rs:8`, `scatter.rs:12`, `zone_scatter.rs:12`. Chunk meshing is per-chunk-load, not per-frame; scatter is per-chunk at generation time.
- `background_loader.rs` uses priority `BinaryHeap<ChunkLoadRequest>` + `HashMap`/`HashSet` of `ChunkId` + tokio async tasks. Each chunk load allocates into the global heap.

### 1.5 GPU-side allocation

**Finding**: All GPU allocation is raw wgpu (`device.create_buffer` / `device.create_texture` / `device.create_bind_group`). No `gpu-allocator`, no `vk-mem`, no VMA-style sub-allocation. One hand-rolled staging ring.

Evidence:
- `rg` against `Cargo.toml` for `gpu-allocator`, `vk-mem`, `vk_mem` → **0 matches**.
- Raw wgpu buffer creation is spread across 38 files (`astraweave-render/src`, from `files_with_matches` search for `create_buffer_init`).
- Per-frame transient GPU data: `StagingRing` (4 MiB, 3 FIF) — see §1.3 item 6.
- Descriptor / bind-group caching: `CachedBindGroup` (see §1.3 item 12) — generation-bumped, opt-in per pass.
- **No descriptor pool** in the Vulkan sense — wgpu abstracts that away. The closest equivalent is the `bindless` material system at `astraweave-render/src/material_bindless.rs` which uses `TEXTURE_BINDING_ARRAY` + a single `GpuMaterialEntry` storage buffer (`material_bindless.rs:31-46`).
- **Bindless lifetime**: single material storage buffer, resized on material-count change; `std::num::NonZeroU32` import at line 11 indicates descriptor-count typing. No explicit lifetime management — materials persist with the renderer.
- **GPU memory accounting**: `GpuMemoryBudget` at `astraweave-render/src/gpu_memory.rs:96-111` tracks `MemoryCategory::{Geometry, Textures, RenderTargets, Uniforms, Staging, Shadows, Environment, Other}` with soft/hard limits (defaults: 256 MB soft / 512 MB hard per category, `gpu_memory.rs:59-66`). The renderer holds one in `Arc<GpuMemoryBudget>` (`renderer.rs:895,3395`).
- **Important**: `GpuMemoryBudget` does **not** sub-allocate or back any actual buffer. It is a call-in counter — consumers must call `try_allocate`/`deallocate` to record their own sizes. Whether every `create_buffer` call site actually reports into this budget was not verified during this audit.

### 1.6 Streaming and asset loading

**Finding**: Each streamed chunk / asset goes through the global allocator via tokio/std file I/O. No buffer-reuse pool exists for streaming. A per-crate budget in chunks/MB is enforced, not a memory-pool budget.

Evidence:
- **Terrain chunk streaming** — `astraweave-terrain/src/background_loader.rs`:
  - `StreamingConfig { max_loaded_chunks: 256, view_distance: 8, prefetch_distance: 4, max_concurrent_loads: 8, ... }` at lines 104-115.
  - Uses `BinaryHeap<ChunkLoadRequest>` priority queue + tokio async tasks. Each load reads bytes from disk into fresh `Vec<u8>`.
- **World-partition cell streaming** — `astraweave-scene/src/streaming.rs`:
  - Cells loaded via `astraweave_asset::cell_loader::load_cell_from_ron` (`streaming.rs:263`) which calls `std::fs::read_to_string` at `astraweave-asset/src/cell_loader.rs:176`.
  - Assets loaded via `load_asset` → `std::fs::read` into `Vec<u8>` (no pre-sized buffer).
- **Texture streaming** — `astraweave-render/src/texture_streaming.rs`:
  - LRU + priority queue; results delivered over tokio `mpsc::Receiver<LoadResult>` (line 93).
  - `TextureHandle` wraps `Arc<Texture>`; eviction drops the `Arc`.
- **General asset loader** — `astraweave-asset/src/lib.rs:1` has `#![forbid(unsafe_code)]`. Reads via `std::fs::read` at `lib.rs:804` and similar.
- **Blender import** — `astraweave-blend` — not inspected in detail; has `jemalloc` feature (see §1.1) but only for benchmarking.

Net: streaming systems each enforce a *count* budget (number of chunks, MB) and evict LRU, but the underlying byte buffers are always fresh allocations. No buffer-reuse ring for decompressed / decoded asset bytes.

### 1.7 Unsafe allocation surfaces

**Finding**: Unsafe allocation is concentrated in one crate (`astraweave-ecs`), primarily one file (`blob_vec.rs`). All of it is covered by the Miri + Kani validation pipeline documented in `CLAUDE.md`.

Evidence — unsafe blocks involving allocation primitives (`Layout`, `NonNull`, `MaybeUninit`, `ManuallyDrop`, `std::alloc`):

1. **`astraweave-ecs/src/blob_vec.rs`** — the primary surface. 75 `unsafe` blocks.
   - Uses `alloc`, `dealloc`, `realloc`, `Layout`, `NonNull` directly (`blob_vec.rs:8-9`).
   - `reserve` calls `alloc(new_layout)` / `realloc(..., new_layout.size())` (`blob_vec.rs:111-146`).
   - `Drop` calls `dealloc(self.data.as_ptr(), layout)` (`blob_vec.rs:414-429`).
   - All blocks have `// SAFETY:` comments (e.g. lines 127, 137-141, 159, 181, 213).
   - Two "INVARIANT" `expect` calls for `Layout::from_size_align` at lines 123 and 422.
   - Kani proofs live at `astraweave-ecs/src/blob_vec_kani.rs` (21 unsafe blocks).

2. **`astraweave-ecs/src/component_meta.rs`** — 5 unsafe blocks.
   - `Layout` stored in `ComponentMeta` (`component_meta.rs:38`), `drop_fn`/`clone_fn` are `unsafe fn(*mut u8)` / `unsafe fn(*const u8, *mut u8)` (lines 40-43).
   - `MaybeUninit` used at line 190 (in doc example).

3. **`astraweave-ecs/src/entity_allocator.rs`** — 3 unsafe blocks.
   - `unsafe fn from_raw(raw: u64)` at line 112 — pure bit decode, no allocation.

4. **`astraweave-ecs/src/sparse_set.rs`** — 46 unsafe blocks. Spot-check shows most are in test modules (via the file total of 46 vs small module surface).

5. **`astraweave-ecs/src/archetype.rs`** — 21 unsafe blocks. Wraps BlobVec's unsafe API.

6. **`astraweave-ecs/src/system_param.rs`** — 4 unsafe blocks. For Query iteration.

7. **`astraweave-ecs/src/parallel.rs`** — 4 unsafe blocks. `SendWorldPtr(*mut World)` for Rayon scope (`parallel.rs:51-58`) with SAFETY comment explaining disjoint-access invariant.

8. **`astraweave-ecs/src/counting_alloc.rs`** — 5 unsafe blocks. `unsafe impl GlobalAlloc for CountingAlloc` forwarding to `System` (`counting_alloc.rs:38-60`). Test-only.

Non-allocation unsafe (for completeness):
- `astraweave-math/src/simd_{vec,mat,quat}.rs` — SSE2 intrinsics (SIMD), orthogonal to allocation.
- `tools/aw_editor/src/main.rs:728-810` and `tools/aw_editor/src/viewport/widget.rs:2800-2840` — Win32 `GetProcessMemoryInfo` / `GlobalMemoryStatusEx` for editor memory UI. Uses `MaybeUninit::<ProcessMemoryCounters>::uninit()` pattern. Not allocation; just Windows API struct filling.

**No manual `Layout` construction from runtime-computed values without validation** was found. `BlobVec::reserve` computes `item_layout.size() * new_capacity` and calls `Layout::from_size_align(...).expect("invalid layout")` which panics on overflow — not ideal but deterministic and covered by Kani.

### 1.8 Profiling / instrumentation already present

**Finding**: Tracy integration is wired in as a compile-time feature; its allocation-tracking macros exist in `astraweave-profiling` but are not called from production code. No dhat, no heaptrack. GPU allocation accounting exists but is decoupled from real allocation sites.

Evidence:
- **Tracy**: `tracy-client = "0.18"` in `astraweave-ai`, `astraweave-behavior`, `astraweave-ecs`, `astraweave-physics`, `astraweave-render`, `astraweave-profiling`, `examples/profiling_demo`, `tools/aw_editor` (per `Cargo.toml` grep results). Feature-gated `profiling` / `profiling-sampling` / `profiling-system` / `profiling-full` at `astraweave-profiling/Cargo.toml:25-32`.
- **Macros** at `astraweave-profiling/src/lib.rs`:
  - `span!` (line 87-92), `frame_mark!` (109-117), `plot!` (130-140), `message!` (153-163), `span_color!` (223-228) — **called widely in production code**.
  - `alloc!` (175-183), `free!` (197-205) — **defined only**. Grep for call sites in production code returns zero results; only hits are in `astraweave-profiling/tests/mutation_resistant_comprehensive_tests.rs:143,150` and doc comments.
- **Counting allocator**: `CountingAlloc` at `astraweave-ecs/src/counting_alloc.rs:36-85`. Uses atomics to count `alloc/dealloc/realloc`. Only active in tests via `--features alloc-counter` + explicit `#[global_allocator]` attribute. Usage at `astraweave-ecs/tests/zero_alloc_tests.rs:47-` (debug_find_allocation_source) shows tests for zero-alloc hot paths exist.
- **GPU accounting**: `GpuMemoryBudget` (`astraweave-render/src/gpu_memory.rs:96-111`) with `try_allocate`/`deallocate`/`on_event` API. Renderer owns an `Arc<GpuMemoryBudget>` at `renderer.rs:895`. Whether every `device.create_buffer`/`create_texture` call site reports to it was not verified during this audit (scope: static-only).
- **Editor Win32 memory display**: `tools/aw_editor/src/main.rs:722-810` queries process working-set and system memory. Display-only.
- **No `dhat`, `heaptrack`, `valgrind`, or `allocative` dependency**. Grep hits are only in doc files (`docs/src/resources/troubleshooting.md:299-301`, `docs/archive/WEEK_4_KICKOFF.md:204`, `tools/aw_editor/PRODUCTION_READINESS_AUDIT.md:502-503`) describing them as external tools to run, not integrated.

---

## Phase 2 — Analysis

### 2.1 Strategy inventory

| # | Strategy | Crate(s) | Scope | Evidence |
|---|---|---|---|---|
| 1 | Default system allocator (malloc) | workspace-wide | long-lived + per-frame | absence of `#[global_allocator]` + absence of allocator deps in `Cargo.toml` |
| 2 | Test-only `CountingAlloc` | astraweave-ecs | test binaries (`alloc-counter` feature) | `astraweave-ecs/src/counting_alloc.rs:36-85`, `tests/zero_alloc_tests.rs:21-23` |
| 3 | Optional `tikv-jemallocator` (unused) | astraweave-blend | `jemalloc` feature, benchmarks only | `crates/astraweave-blend/Cargo.toml:77-81` |
| 4 | Hand-rolled generational-index allocator | astraweave-ecs | entity lifecycle | `entity_allocator.rs:171-186` |
| 5 | Hand-rolled type-erased contiguous storage (raw `alloc`) | astraweave-ecs | ECS component columns | `blob_vec.rs:26-146,414-429` |
| 6 | Hand-rolled sparse set | astraweave-ecs | entity→row index | `sparse_set.rs:22-72` |
| 7 | Fixed-size SoA particle pool (Vec<f32>×12) | astraweave-fluids | SPH simulation | `simd_ops.rs:1475-1562` |
| 8 | GPU staging ring (4MB, 3 FIF) | astraweave-render | per-frame transient GPU | `staging_ring.rs:52-164`, used at `renderer.rs:4676,5182` |
| 9 | Fixed-size particle pool (Vec+swap-remove) | astraweave-render | weather particles | `effects.rs:17-63` |
| 10 | GPU buffer + CPU free-list | astraweave-fluids | WgpuFluid dynamic particles | `lib.rs:310-322,826,879,993` |
| 11 | HashMap+VecDeque LRU (placeholder) | astraweave-render | asset residency (unused placeholder) | `residency.rs:8-15` |
| 12 | LRU + priority BinaryHeap + mpsc | astraweave-render | texture streaming | `texture_streaming.rs:76-96` |
| 13 | LRU + priority BinaryHeap + tokio | astraweave-terrain | chunk streaming | `background_loader.rs:80-116` |
| 14 | Per-cell HashMap<AssetId, Buffer> | astraweave-scene | world-partition cell GPU resources | `gpu_resource_manager.rs:15-63` |
| 15 | Generation-tagged bind group cache | astraweave-render | per-pass GPU descriptor reuse | `bind_group_cache.rs:26-74` |
| 16 | Per-category GPU byte counter (accounting only) | astraweave-render | GPU budget enforcement | `gpu_memory.rs:96-270` |

### 2.2 Coverage map

| Memory domain | Primary strategy | Evidence | Fit? |
|---|---|---|---|
| Per-frame CPU scratch (light bins, visible instance list, ECS group schedule) | Default allocator (fresh `Vec` each call) | `clustered.rs:41-107`, `renderer.rs:6032-6069`, `parallel.rs:265` | **Mismatch**: frame-scoped lifetime going through global heap. |
| Per-frame GPU uniforms | `StagingRing` (4MB, 3 FIF) | `staging_ring.rs`, `renderer.rs:4676,5182` | **Fit**: dedicated ring, correct lifetime. |
| ECS component storage | `BlobVec` archetype + fallback `Box<dyn Any>` | `blob_vec.rs`, `archetype.rs:66-103` | **Fit for BlobVec path**. Legacy Box path is higher overhead but explicitly retained for compat. |
| ECS entity IDs | `EntityAllocator` free-list | `entity_allocator.rs:171-186` | **Fit**. |
| GPU long-lived resources (meshes, textures, pipelines) | Raw `device.create_buffer` / `create_texture` | `renderer.rs` constructor, `mesh_*.rs`, `texture.rs` | **Fit for wgpu usage**. Missing: accounting coverage (GpuMemoryBudget is opt-in per-callsite). |
| GPU per-cell resources (world partition) | `CellGpuResources` HashMap | `gpu_resource_manager.rs:15-63` | **Fit in shape**, but unbounded per-cell HashMap resize cost not examined. |
| Streaming byte buffers (terrain chunks, cells, meshes) | Fresh `Vec<u8>` each load | `astraweave-asset/src/lib.rs:804`, `cell_loader.rs:176` | **Mismatch**: streaming lifetime is bounded, but every load hits the global heap; no reuse. |
| Particle systems | Pre-sized pools (SPH SoA, Weather Vec, GPU fluid) | `simd_ops.rs:1475`, `effects.rs:20`, `fluids/lib.rs:314` | **Fit**. |
| GOAP A* working set (open_set, closed_set, state/path clones) | Default allocator, freshly allocated per plan call | `planner.rs:225-304` | **Mismatch**: per-plan scratch lifetime, no arena. Highest per-AI-tick allocation volume identified. |
| Editor-only memory (Win32 queries, egui buffers) | Default allocator | `aw_editor/src/main.rs:728-810`, egui internals | Acceptable (editor, not runtime). |
| LLM plan payloads | `Arc`-wrapped, cloned on dispatch | `ai_arbiter.rs:609`, `llm_executor.rs:165-182` | `Arc::clone` is cheap; the `snap.clone()` is the real cost but is cooldown-gated. |
| Physics bodies/colliders | Rapier3D internal pooling | `rapier3d` dep, `physics/src/lib.rs:1080-1094` | **Delegated to library** — out of scope for our audit. |

### 2.3 Gaps

Ranked by *likely* allocation volume, with qualifier on how that estimate was derived:

| # | Gap | Likely volume | Basis |
|---|---|---|---|
| 1 | GOAP A* per-expansion `clone()` + `String` allocations | High (per AI tick for GOAP-using agents) | Inferred from `planner.rs:269,277,283-284,287` — 3-4 allocs per iter, up to 10000 iters. Not profiled. |
| 2 | Streaming byte buffers not reused | High (per chunk/cell/texture load) | Inferred from `fs::read`/`fs::read_to_string` call sites. Latency-dominated, but byte buffers can be large (MBs). |
| 3 | `bin_lights_cpu` 4 × `Vec<u32>` per frame when CPU path is used | Medium (per frame) | Code at `clustered.rs:50,100,107,108`. Documented as 0.5-2ms in `clustered_megalights.rs:5`. GPU replacement exists behind feature flag. |
| 4 | `glights: Vec<GpuLight> = ...collect()` per frame | Medium | `renderer.rs:4733-4748` — one allocation per frame, scale-linear with light count. |
| 5 | `build_visible_instances` returns fresh `Vec<InstanceRaw>` per frame | Medium | `renderer.rs:6032-6069` — pre-sized via `with_capacity`, so single alloc per frame. |
| 6 | `ParallelSchedule::build_groups` rebuilt every tick | Medium | `parallel.rs:230-252,265` — `O(systems²)` but small constants; one `Vec<Vec<usize>>` per stage per tick. |
| 7 | `WorldSnapshot::clone` into LLM task | Medium (cooldown-gated, not per-tick) | `ai_arbiter.rs:609`. |
| 8 | Per-cell `HashMap<AssetId, Buffer>` resize allocation | Low-to-Medium | `gpu_resource_manager.rs` — depends on cell-load frequency. Unknown without profiling. |
| 9 | `CachedBindGroup` opt-in coverage not universal | Unknown without profiling | 345 `create_bind_group` sites across 61 files. Whether all per-frame uses route through `CachedBindGroup` was not verified. |
| 10 | ECS parallel `Vec<Box<dyn Any>>` path (legacy mode) | Low unless in use | `archetype.rs:78` — only used when `ComponentMeta` not supplied. Needs runtime check. |

**Measurement status**: every row above is "inferred from code structure". No row is "measured".

### 2.4 Redundancy

- **Two residency systems, one unused placeholder**: `ResidencyManager` (`residency.rs`) and `TextureStreamingManager` (`texture_streaming.rs`). The former stores `gpu_handle: Option<String>` as a `format!` string (`residency.rs:85`) — it is not wired to real GPU handles. The latter uses real `Arc<Texture>`. Whether `ResidencyManager` is consumed anywhere beyond its own tests was not verified, but the naming overlap suggests divergence during development. *Architecture-drift-detector territory.*
- **Two LRU + priority-queue streaming implementations**: `astraweave-terrain/src/background_loader.rs` (terrain chunks) and `astraweave-render/src/texture_streaming.rs` (textures). They have nearly identical shapes: `BinaryHeap<Request>` + `HashMap<Id, State>` + budget + async loader. A shared streaming primitive would dedupe ~200 LoC; not urgent.
- **Budgets in two places without a shared abstraction**: `GpuMemoryBudget` (bytes, `gpu_memory.rs`) vs `StreamingConfig.max_loaded_chunks` (count, `background_loader.rs:81`) vs `TextureStreamingManager.max_memory_bytes` (bytes, `texture_streaming.rs:86`). Each uses its own enforcement code. A unified "resource budget" trait would be a small refactor.
- **Multiple pool implementations as noted in §1.3 (items 5, 7, 9, 10)**. `BlobVec`, `SphMemoryPool`, `WeatherFx.pool`, and `WgpuFluid.free_list` all implement "pre-size and reuse" differently. Not obviously redundant — they hold different types — but a workspace `FixedPool<T>` could standardise the pattern.

### 2.5 Unsafe footprint

Isolated: effectively one file (`astraweave-ecs/src/blob_vec.rs`) drives all the raw allocation. Everything else that is `unsafe` in the ECS either wraps BlobVec (`archetype.rs`, `system_param.rs`), is bit manipulation (`entity_allocator.rs::from_raw`), or is a parallelism primitive (`parallel.rs::SendWorldPtr`).

Good patterns observed:
- Every unsafe block has a `// SAFETY:` comment (verified in `blob_vec.rs:127,137,181,213,247,283-288`, `parallel.rs:54-58`).
- Layout construction failures are `.expect(...)` with INVARIANT comments, not silently wrapped.
- Kani proofs exist for the primary file (`blob_vec_kani.rs`) and the entity allocator (`entity_allocator_kani.rs`).
- Miri is wired in CI (per `CLAUDE.md` and `.github/workflows/miri.yml`).

Sketchy patterns — **none found** in the allocation code. No manual `alloc` without a matching `dealloc`; no `Layout` from attacker-controllable runtime values without validation; no uninitialised `MaybeUninit` reads that I saw.

### 2.6 Consistency with engine philosophy

AstraWeave declares itself deterministic, formally verified (Miri + Kani per `CLAUDE.md`), and AI-native.

Tensions:

1. **Global allocator non-determinism** — Default system `malloc` on Windows/Linux can return different pointer patterns and service times across runs. This does not by itself break logical determinism (the engine uses generational indices rather than raw pointers in IDs), but it **can cause frame-time variance that propagates into AI tick budgets** (e.g. a GC-induced pause in any allocator makes a GOAP plan exceed its budget on one run but not another). The project documents determinism heavily (`docs/audits/DETERMINISM_AUDIT_JAN_2026.md` exists alongside this audit) but I found no determinism claim bound to allocation behavior specifically. Worth stating explicitly.

2. **AI tick budget** — The GOAP A* per-iteration clone storm (§2.3 #1) is the most direct threat to bounded AI planning latency. The engine's 60 Hz tick (16.67 ms budget) can be derailed by several thousand small allocations in a single GOAP call; without an arena or a hard iteration cap that also bounds allocation count, latency is a property of the allocator's state, not the algorithm.

3. **Replay vs allocation order** — Not directly addressed anywhere. If replay ever needs to reproduce allocation counts (e.g. for mutation-test determinism), the default allocator's behavior becomes an input.

4. **Unsafe outside the formally-verified set** — The Miri/Kani set per `CLAUDE.md` is `ecs`, `math`, `core`, `sdk`. `astraweave-ecs` is validated; no other crate in the workspace uses `std::alloc::*` directly (per §1.7), so this tension is minimal.

5. **Measurement gap** — The engine *could* measure allocations in production-gated builds (`CountingAlloc` exists, `alloc!`/`free!` macros exist), but it does not. Every finding in §2.3 is unverified allocation volume. This is the biggest philosophy-mismatch: spacecraft-grade standards + mission-critical AI paths + zero allocation measurement in the live engine.

---

## Phase 3 — Recommendations

Ranked by value-per-risk. I capped the list at 10; additional ideas are in the appendix.

### 1. Wire `alloc!/free!` or `CountingAlloc` into production hot-path tests and benchmarks

- **Current state**: `alloc!`/`free!` macros defined at `astraweave-profiling/src/lib.rs:175-205` but unused outside tests. `CountingAlloc` is gated to tests.
- **Target state**: Expose allocation counts/bytes per frame in `profiling_demo` and in benchmarks for render, physics step, AI tick, GOAP plan. Make the count visible in Tracy as a plotted value via `plot!`. Optionally enable `CountingAlloc` behind a debug-build feature so `cargo bench` fails when hot paths regress to `> N` allocs per iteration.
- **Scope**: Workspace-wide, but purely additive.
- **Risk**: Low (feature-gated; zero cost when off).
- **Value**: High (diagnostic). Unlocks all other recommendations because they become measurable. Without this, the rest of this list is guesswork.

### 2. Scratch arena for GOAP A* expansion state

- **Current state**: `astraweave-ai/src/goap/planner.rs:269,277,283-287` clones `WorldState` + `Vec<String>` path + allocates `String` per action name per A* expansion. Up to 10000 iterations × 3-4 allocs.
- **Target state**: Add a `PlannerScratch` struct owned by `AdvancedGOAP` (or passed into `plan`). Replace `Vec<String>` path with `Vec<u32>` action-index path into a per-frame action-name interner. Replace `WorldState::clone` with a reversible "apply/unapply" on a single mutable working state, or with an arena-backed `StateId` that stores one `WorldState` per unique state (dedup via hash). Clear the scratch at the start of each `plan` call, reuse capacity.
- **Scope**: One crate (`astraweave-ai`), one module (`goap/planner.rs`), and callers.
- **Risk**: Medium — changes planner data flow and must preserve behavior. Full test coverage exists (`goap/tests.rs`, mutation tests), so regressions are detectable.
- **Value**: High — this is the largest allocation volume per AI tick I could identify statically.

### 3. Reuse `build_visible_instances` and `bin_lights_cpu` output buffers

- **Current state**: `renderer.rs:6055` allocates `Vec<InstanceRaw>` every frame. `clustered.rs:50,100,107,108` allocates four `Vec<u32>` every frame.
- **Target state**: Store `vis_raws: Vec<InstanceRaw>`, `light_counts: Vec<u32>`, `light_indices: Vec<u32>`, `light_offsets: Vec<u32>`, `light_cursors: Vec<u32>` as fields of `Renderer` (or in the staging struct) and `clear()` + reuse each frame. API change: `build_visible_instances(&mut self, out: &mut Vec<InstanceRaw>)`.
- **Scope**: Single crate (`astraweave-render`), two functions.
- **Risk**: Low (additive; existing callers are internal).
- **Value**: Medium. Depends on instance/light count. The GPU `clustered_megalights` path subsumes the CPU light-bin cost when enabled, so this recommendation is strongest for the `build_visible_instances` side.

### 4. Cache `ParallelSchedule::build_groups` output

- **Current state**: `astraweave-ecs/src/parallel.rs:265` rebuilds greedy conflict-coloured groups every `run()` call. Stage systems rarely change across ticks.
- **Target state**: Compute groups once when a system is added or mutated; store in the stage. Invalidate on `add_system`. Run path reads cached groups.
- **Scope**: Single file (`parallel.rs`).
- **Risk**: Low.
- **Value**: Medium. The tick-rate allocation is eliminated; CPU cycles are tiny but consistent.

### 5. Add an optional `mimalloc` / `snmalloc` global-allocator feature to the root workspace

- **Current state**: System allocator. No benchmarks comparing allocators exist in the workspace.
- **Target state**: Introduce a `fast-alloc` feature on the workspace root that conditionally installs `mimalloc` or `snmalloc` as the global allocator from each binary. Default off. Document a bench harness (using #1) that measures per-frame allocation cost with and without.
- **Scope**: Workspace-wide `Cargo.toml` + one line in each binary's `main.rs`.
- **Risk**: Low (opt-in, default off). License check (`mimalloc` is MIT, `snmalloc` is MIT — compatible with project MIT).
- **Value**: Medium. Measured at other Rust game/engine projects to shave 10-30% off small-allocation-heavy workloads. Cannot be asserted for AstraWeave without #1 measuring first.

### 6. Consolidate `ResidencyManager` and `TextureStreamingManager`

- **Current state**: `astraweave-render/src/residency.rs` is a placeholder (`gpu_handle: Option<String>` at `residency.rs:85` is a literal `format!` string). `astraweave-render/src/texture_streaming.rs` is the real one. Both implement LRU + capacity budget.
- **Target state**: Delete `ResidencyManager` entirely, or re-home it as a trait with a single real implementation (`TextureStreamingManager`). If asset-kind-generic residency is needed, generalise `TextureStreamingManager` over a resource trait.
- **Scope**: One crate (`astraweave-render`), touches any module that references `ResidencyManager` — needs a quick consumer search first.
- **Risk**: Medium (deletes a public type; confirm no downstream users).
- **Value**: Medium (clarity, not perf).

### 7. Reuse streaming byte buffers for terrain chunks and cell loads

- **Current state**: Each `std::fs::read` / `tokio::fs::read` call allocates a fresh `Vec<u8>` (`astraweave-asset/src/lib.rs:804`, `cell_loader.rs:176`). `StreamingConfig.max_concurrent_loads = 8` caps concurrency but not allocations.
- **Target state**: A small bounded pool of reusable `Vec<u8>` scratch buffers sized for typical chunk/cell size, handed out to load tasks and returned on completion. Channel-based hand-off, similar to `TextureStreamingManager`'s `mpsc`.
- **Scope**: `astraweave-asset` + one call-site change per consumer.
- **Risk**: Medium (adds cross-cutting state; must not deadlock on pool exhaustion).
- **Value**: Medium. Dominated by disk I/O, so the allocation saving is secondary to latency. Real value is reducing OS `brk`/`VirtualAlloc` churn during heavy streaming.

### 8. Verify and enforce `GpuMemoryBudget` coverage

- **Current state**: `GpuMemoryBudget` at `astraweave-render/src/gpu_memory.rs:96-270` records allocations when callers opt in. No audit of which `device.create_buffer`/`create_texture` sites report. 345 bind-group sites, 58+ buffer sites in `renderer.rs` alone.
- **Target state**: Wrap `create_buffer` / `create_texture` in helper functions that both record to `GpuMemoryBudget` *and* emit a Tracy `plot!`. Replace raw wgpu calls in the render crate. Add a CI lint (deny a hand-rolled `device.create_buffer` in `astraweave-render/src/**` via `clippy::disallowed_methods`).
- **Scope**: Single crate (`astraweave-render`), large diff but mechanical.
- **Risk**: Medium (touches every buffer site in the render crate; must preserve behavior).
- **Value**: Medium-high (observability). Complements #1.

### 9. Per-frame allocation-count CI gate

- **Current state**: `astraweave-ecs/tests/zero_alloc_tests.rs` tests ECS hot paths but nothing similar covers render, physics, AI.
- **Target state**: Add `#[cfg(feature = "alloc-counter")]` tests that spin a minimal render frame / physics step / AI tick and assert `allocs() - deallocs() <= N` for a small `N`. Wire into CI as a non-blocking job initially; tighten `N` as recommendations 2-4 land.
- **Scope**: Workspace (new test files in render/physics/ai).
- **Risk**: Low (new tests only).
- **Value**: High — prevents regressions from landing silently, which is the most likely way any of the above recs drift back over time.

### 10. Deprecate the legacy `Box<dyn Any>` archetype path

- **Current state**: `astraweave-ecs/src/archetype.rs:66-103` supports two storage modes: BlobVec (fast, type-erased raw) and `Vec<Box<dyn Any + Send + Sync>>` (legacy). The legacy path is kept for when `ComponentMeta` isn't supplied.
- **Target state**: Audit consumers to confirm all register `ComponentMeta`. Remove the legacy path once verified. Eliminates `HashMap` of `Vec<Box<dyn Any>>` entirely, along with its per-insert `Box::new`.
- **Scope**: `astraweave-ecs` + every component registration site.
- **Risk**: High (ECS core, behavior change if any consumer silently uses the legacy path).
- **Value**: Medium (performance + code simplification).

### Appendix — future considerations (not in the top 10)

- `StagingRing` default size (4 MiB) and FIF count (3) are constants; a diagnostic that warns when `peak_bytes >= capacity * 0.9` (already tracked at `staging_ring.rs:65-66`) would preempt silent stalls. Cheap.
- `SphMemoryPool` grows via `next_power_of_two`. Under repeated shrinks it never releases memory. Add a `shrink_to_fit` hook tied to quality settings.
- `Archetype.blob_components` is `Option<HashMap<...>>` for lazy init — a small-vector or `TypeIdMap` could replace `HashMap` and remove the per-archetype `HashMap::new` cost when archetype count grows.
- `astraweave-asset/src/lib.rs:1` uses `#![forbid(unsafe_code)]` — good. Worth propagating to `astraweave-terrain`, `astraweave-nav`, `astraweave-ai`, `astraweave-behavior` where no unsafe is used today, to pin that invariant.
- Rapier3D's allocation behavior is not visible to this audit. A benchmark under `CountingAlloc` would reveal whether its internal pools suffice.

---

## Evidence index

Files read or cited, grouped by crate.

**astraweave-ecs**
- `src/lib.rs` (header, module map)
- `src/entity_allocator.rs` (generational allocator)
- `src/blob_vec.rs` (type-erased raw allocation)
- `src/archetype.rs` (component storage, dual-mode)
- `src/sparse_set.rs` (O(1) entity index)
- `src/component_meta.rs` (Layout + drop/clone fn)
- `src/parallel.rs` (ParallelSchedule, SendWorldPtr)
- `src/counting_alloc.rs` (test-only `GlobalAlloc`)
- `Cargo.toml`
- `tests/zero_alloc_tests.rs`

**astraweave-ai**
- `src/core_loop.rs`
- `src/ai_arbiter.rs`
- `src/llm_executor.rs`
- `src/goap/planner.rs`
- `Cargo.toml`

**astraweave-render**
- `src/renderer.rs` (render, update_instances, build_visible_instances, GpuMemoryBudget ownership)
- `src/clustered.rs` (`bin_lights_cpu`)
- `src/clustered_megalights.rs` (GPU replacement, header)
- `src/staging_ring.rs` (per-frame GPU ring)
- `src/bind_group_cache.rs` (generation-tagged bind group cache)
- `src/gpu_memory.rs` (GpuMemoryBudget)
- `src/residency.rs` (placeholder asset residency)
- `src/texture_streaming.rs` (real texture residency)
- `src/material_bindless.rs` (bindless materials)
- `src/effects.rs` (WeatherFx particle pool)
- `src/virtual_texture.rs` (page encoding)
- `Cargo.toml`

**astraweave-physics**
- `src/lib.rs` (step, step_internal, field layout around handles)
- `Cargo.toml`

**astraweave-fluids**
- `src/lib.rs` (WgpuFluid, free_list, particle_flags)
- `src/simd_ops.rs` (SphMemoryPool)

**astraweave-terrain**
- `src/background_loader.rs` (StreamingConfig, priority queue)

**astraweave-scene**
- `src/gpu_resource_manager.rs` (CellGpuResources)
- `src/streaming.rs` (cell streaming path)

**astraweave-asset**
- `src/lib.rs` (header; `fs::read` call sites)
- `src/cell_loader.rs` (`fs::read_to_string` call site)

**astraweave-profiling**
- `src/lib.rs` (span/frame_mark/plot/message/alloc/free macros)
- `Cargo.toml`

**astraweave-blend**
- `Cargo.toml` (optional tikv-jemallocator)

**Workspace root & others**
- `Cargo.toml` (workspace members, profiles, deps)
- `CLAUDE.md` (Miri/Kani/philosophy claims referenced)
- `tools/aw_editor/src/main.rs` (Win32 memory query)
- `tools/aw_editor/src/viewport/widget.rs` (Win32 memory query)

---

## Open questions

These cannot be answered from static analysis alone. Each is resolvable with a specific profiling run.

1. **What fraction of frame time in `unified_showcase` is spent in the default allocator?** Answer: run `unified_showcase --release` under Tracy with `profiling-system` feature; inspect the system allocator zone. Or run with `CountingAlloc` for a bounded duration and report allocs/frame.
2. **How many allocations does a single GOAP plan cost in practice?** Answer: enable `alloc-counter` in `astraweave-ai`, gate a test that runs a representative `plan_direct` on a 20-action action set, call `reset_allocs()` then measure.
3. **Which of the 345 `create_bind_group` sites in `astraweave-render` run per-frame vs at init?** Answer: Tracy-capture a render frame with `profiling-full`; zones will name the caller.
4. **Is `ResidencyManager` referenced anywhere outside its own file?** Answer: `rg "ResidencyManager" astraweave-render/src examples tools` in the main branch with tests excluded. (I did not run this as part of this audit to keep scope contained.)
5. **What is the peak `StagingRing` utilisation in a steady-state frame?** Answer: the renderer already tracks `peak_bytes` at `staging_ring.rs:66`; log it per N frames to a Tracy plot.
6. **Does every `device.create_buffer` call site report to `GpuMemoryBudget`?** Answer: add `#[cfg(debug_assertions)]` counter in a wrapper; assert that `(create_buffer calls) == (try_allocate calls)` over a fixed capture.
7. **Does Rapier3D's per-step allocation count grow with simulation time?** Answer: `CountingAlloc` gated test that runs 10000 physics steps with a fixed body set and asserts linearity.
8. **What is the steady-state per-tick allocation count of `ParallelSchedule::run`?** Answer: `alloc-counter` gated benchmark on a representative stage/system topology.

## Verification hooks

Three one-line commands that reproduce the core findings of this audit:

```bash
# 1. Confirm no global allocator override outside tests and no mainstream allocator crate.
rg -n '#\[global_allocator\]|mimalloc|jemallocator|tikv-jemallocator|snmalloc|rpmalloc|tcmalloc' --glob '!target'

# 2. Confirm no arena/bump/slab/slotmap/thunderdome dependencies.
rg -n '(bumpalo|typed-arena|id-arena|generational-arena|slotmap|thunderdome|^slab )|slab ' --glob '*Cargo.toml'

# 3. Locate every raw `std::alloc` / `Layout` / `NonNull` allocation site outside tests.
rg -n 'std::alloc::(alloc|dealloc|realloc|Layout)|NonNull::new\(alloc|Layout::from_size_align' --glob '!**/tests/**' --glob '!**/benches/**' --glob '!target'
```

---

**Report status**: Discovery and analysis complete. No code changes made. All recommendations require explicit follow-up.
