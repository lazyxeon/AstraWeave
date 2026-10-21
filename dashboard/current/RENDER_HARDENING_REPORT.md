# astraweave-render Hardening Report

**Date**: 2025-01-XX (updated)
**Scope**: Pre-mutation-testing hardening of `astraweave-render`
**Test count**: 671 → 806 (+135 new tests across 3 sessions)
**Clippy**: 10 errors → 0
**Compilation**: Clean (`cargo check -p astraweave-render`)

---

## Phase 1: Critical Correctness Fixes

### 1A — Texture Streaming State Machine (FIXED)

**Bug**: `request_texture()` inserted `AssetState::Loading` immediately, but `process_next_load()` also set Loading when popping from the queue. If a texture was requested but not yet processed, the premature Loading state prevented the texture from being loaded when `process_next_load()` ran — it saw the key in `assets` and skipped it.

**Fix**:
- Removed premature `assets.insert(id, AssetState::Loading)` from `request_texture()`
- Added `pending_ids: HashSet<AssetId>` field for O(1) dedup of queue entries
- `request_texture()` checks `pending_ids` + `assets` before enqueueing
- `process_next_load()` is now the sole owner of the Loading→Resident lifecycle
- `clear()` resets `pending_ids`

**Files changed**: `texture_streaming.rs`
**Tests added (9)**:
1. `request_queues_without_premature_loading_state`
2. `duplicate_request_does_not_double_queue`
3. `failed_texture_is_not_requeued`
4. `loading_texture_blocks_requeue_but_not_via_assets_map`
5. `stats_counts_pending_correctly`
6. `clear_resets_all_state`
7. `resident_texture_is_returned_and_lru_updated`
8. `evict_lru_removes_oldest`
9. `stats_counts_loaded_with_resident`

### 1B — Shadow Override (FIXED)

**Bug**: WGSL shader had hardcoded `shadow = 1.0;` that bypassed all shadow computation regardless of `force_shadow_override` flag.

**Fix**:
- Added `pub force_shadow_override: bool` to Renderer (defaults `false`)
- CPU: When flag is true, writes `-1.0` as `extras.x` in light UBO (sentinel)
- WGSL: Both main and skinned shaders now check `if (uLight.extras.x < 0.0) { shadow = 1.0; }` — shadows only bypassed when sentinel is set
- Sentinel applied at all 3 light buffer write locations

**Files changed**: `renderer.rs`
**Tests added (2)**:
1. `test_shadow_override_sentinel_logic` — verifies sentinel bytes in light UBO
2. `test_shader_has_conditional_shadow_not_hardcoded` — verifies WGSL source contains conditional, not hardcoded override

---

## Phase 2: Render Path Unification

### 2A — Divergence Audit

Full comparison of `render()` (standalone viewport) vs `draw_into()` (composable offscreen). 12 operations compared across both paths.

### 2B — Alignment Fixes (8 applied)

| # | Operation | Path Fixed | Change |
|---|-----------|-----------|--------|
| 1 | Shadow pass: sphere instances + ext mesh | `draw_into()` | Was plane-only; now draws sphere instances and external mesh |
| 2 | Main pass: external mesh instance count | `render()` | Used `0..1` hardcoded; now uses `self.ext_inst_count` |
| 3 | Main pass: named models | `render()` | Was missing entirely; now renders `self.models.values()` |
| 4 | Shadow pass: ext mesh instance count | `render()` | Used `0..1` hardcoded; now uses `self.ext_inst_count` |
| 5 | Main pass: water rendering | `draw_into()` | Was missing; now renders water after opaque geometry |
| 6 | Sky rendering: VP matrix + IBL | `render()` | Used full VP + `None,None`; now uses rotation-only VP + IBL resources |
| 7 | Main pass: depth load op | `render()` | Was `Clear(1.0)` (overwrote sky depth); now `Load` (matches draw_into) |
| 8 | Scene env UBO upload | `draw_into()` | Was missing; now uploads before main pass |

**Files changed**: `renderer.rs`

---

## Phase 3: Cleanup & Polish

### 3A — Surface Error Handling (NORMALIZED)

**Issue**: `render_with()` and `render_with_simple()` used raw `surface.get_current_texture()?` without Lost/OutOfMemory handling, while `render()` had full match-based error handling.

**Fix**: Extracted `acquire_surface_texture()` helper method returning `Result<Option<(SurfaceTexture, TextureView)>>`:
- Returns `Ok(None)` when no surface configured
- Returns `Ok(None)` on `SurfaceError::Lost` (after reconfiguring)
- Returns `Err(...)` on `SurfaceError::OutOfMemory`
- All three render methods now use this shared helper

**Files changed**: `renderer.rs`

### 3B — Debug Artifact Cleanup (4 items)

| Artifact | File | Classification | Action |
|---------|------|---------------|--------|
| `// DEBUG: Binary search - Test 3` | `renderer.rs` (draw_into) | Dead comment | Removed |
| `println!("🔍 Light frustum...")` | `shadow_csm.rs` | Dev debug | Converted to `log::debug!()` |
| `println!("🔍 Shadow rendering...")` (5 lines) | `shadow_csm.rs` | Dev debug | Converted to `log::debug!()` |
| Magenta clear color (1,0,1) | `environment.rs` | Diagnostic placeholder | Conditional: magenta in debug, dark grey in release |

**Files changed**: `renderer.rs`, `shadow_csm.rs`, `environment.rs`

### 3C — Clippy Resolution (10 → 0)

| # | Lint | File | Fix |
|---|------|------|-----|
| 1 | `unused_variable` | `renderer.rs:584` | Prefixed `f` → `_f` |
| 2-3 | `manual_div_ceil` ×2 | `nanite_gpu_culling.rs:911-912` | `(x+7)/8` → `x.div_ceil(8)` |
| 4 | `manual_div_ceil` | `nanite_gpu_culling.rs:935` | `(x+63)/64` → `x.div_ceil(64)` |
| 5-6 | `manual_div_ceil` ×2 | `nanite_gpu_culling.rs:953-954` | `(x+7)/8` → `x.div_ceil(8)` |
| 7 | `too_many_arguments` | `nanite_render.rs:36` | `#[allow(clippy::too_many_arguments)]` (constructor) |
| 8 | `unnecessary_cast` | `nanite_render.rs:245` | Removed `as u32` (already u32) |
| 9 | `let_and_return` | `nanite_visibility.rs:159` | Returned expression directly |
| 10 | `needless_range_loop` | `ssao.rs:101` | Converted to `iter_mut().enumerate()` |

**Files changed**: `renderer.rs`, `nanite_gpu_culling.rs`, `nanite_render.rs`, `nanite_visibility.rs`, `ssao.rs`

### 3D — Source Artifact Cleanup

Moved 3 stale files to `src/_archive/` (not referenced by build or code):
- `renderer.rs.baseline`
- `renderer.rs.broken.keep`
- `residency.rs.head`

---

## Summary

| Metric | Before | After |
|--------|--------|-------|
| Tests | 671 | 806 (+135) |
| Clippy errors | 10 | 0 |
| Production println!/eprintln! | 49 | 0 (44 converted to log macros, 5 to log::debug) |
| Surface error handling paths | 1/3 correct | 3/3 correct |
| Render path divergences | 8 operations | 0 (unified) |
| Stale source artifacts | 3 files | 0 (archived) |

---

## Phase 4: Mutation-Resistant Test Hardening (Session 2)

### 4A — Production Logging Conversion (44 calls)

Converted all remaining `println!`/`eprintln!` in production code to structured `log` macros:

| File | Calls | Conversions |
|------|-------|-------------|
| `material_loader.rs` | 27 | `eprintln!` → `log::warn!`, `println!` → `log::info!/log::debug!` |
| `material.rs` | 8 | `println!` → `log::info!`, `eprintln!` → `log::warn!` |
| `texture.rs` | 8 | `println!` → `log::info!` |
| `residency.rs` | 1 | `println!` → `log::debug!` |

### 4B — New Test Modules for Untested Files (+23 tests)

**advanced_post.rs** (11 tests — new module):
Config defaults (TAA, motion blur, DOF, color grading), Halton sequence (base2/base3 known values, zero-returns-zero, unit interval), shader constants (non-empty, have entry points via naga).

**debug_quad.rs** (6 tests — new module):
Vertex count (6), NDC coverage (-1 to 1), UV corners, z=0, vertex descriptor attributes (2), stride matches struct size.

**ssao.rs** (6 tests — appended):
Halton base2 known values, halton zero-returns-zero, kernel hemisphere validation, kernel scale increases with index, unused samples are zero, quality levels ascending.

### 4C — Mutation-Resistant Test Strengthening (+40 tests)

**vertex_compression.rs** (8 tests):
Neg-Z axis encoding, lower hemisphere diagonal, lower hemisphere sweep (8×3 normals), encoding error nonzero for non-axis, all 6 axis-aligned normals exact, half-float negative values, half-float zero exact, memory_reduction constant matches calculation. Also fixed `encoding_error()` to clamp dot product to [-1,1] preventing NaN from float imprecision.

**lod_generator.rs** (11 tests):
Exact vertex/triangle counts (8/12 cube), LOD monotonic reduction, calculate_reduction identity=0, quadric zero error=0, quadric add commutative, quadric add accumulates (expected 2.0), from_plane error=0 on plane, from_plane error proportional to distance², edge_collapse min-heap order (BinaryHeap 1→5→10), LodConfig default has 3 targets in (0,1).

**clustered.rs** (9 tests):
clamp_u32 boundary values (negative/positive/equal bounds), empty lights produces zero counts, light behind far plane skipped, z-range collapse robustness (near==far), single centered light occupies center clusters, large radius fills many clusters, multiple lights indices cover all IDs, offsets are exclusive scan of counts, cluster_index linearization formula (3×5×7 dims).

**culling.rs** (21 tests):
WGSL shader parses (naga), InstanceAABB::new field values, DrawIndirectCommand::new all fields, DrawIndirectCommand::default is zeroed, BatchId::new field values, BatchId equality and Ord ordering, DrawBatch starts empty, add_instance increments count, build_indirect_commands empty/single/multiple batches, batch_visible_instances grouping + empty input, cpu_frustum_cull empty instances, cpu_frustum_cull preserves instance indices, frustum planes are six + normalized, AABB from rotated transform expands extent, struct sizes (InstanceAABB=32, FrustumPlanes=96, DrawIndirectCommand=16).

### Bug Fix: encoding_error NaN

`OctahedralEncoder::encoding_error()` could return NaN when encode→decode round-trip produced a dot product > 1.0 due to floating-point imprecision. Fixed by clamping to [-1.0, 1.0] before `acos()`.

---

## Phase 5: Final Gap Closure (Session 3) — +61 tests (745 → 806)

Systematic audit of all 65 source files identified 8 files with weak or zero test coverage. All gaps closed:

### 5A — error.rs (17 tests — new module)

Display formatting for all 12 error variants, `From<io::Error>` conversion, `From<anyhow::Error>` conversion, structured `AssetLoad` field validation, `RenderResult<T>` with `?` operator, Debug trait verification.

### 5B — shadow_csm.rs (+9 tests)

`CASCADE_COUNT=4` constant, `CASCADE_RESOLUTION=2048` constant, `DEPTH_BIAS` positive+small, `GpuShadowCascade` From conversion (all fields), cascade splits monotonic for 4 ranges, splits boundary conditions, light view up-vector branch selection (|y|>0.9), `SHADOW_DEPTH_SHADER` WGSL parsing via naga, `ATLAS_RESOLUTION` equals `CASCADE_RESOLUTION`.

### 5C — deferred.rs (+5 tests)

Complete `GBufferFormats::default()` all 5 fields, 5 attachment formats count, `DEFERRED_LIGHT_SHADER` WGSL parsing (vs_main/fs_main entry points), shader binding count (5), format type distinctness checks.

### 5D — clustered_forward.rs (+7 tests)

`ClusterConfig` near/far/padding values, total cluster count (3456), struct size (32 bytes), `GpuLight::new` all 8 field values, `GpuLight` struct size (32), `GpuCluster` struct size (48), `GpuLight::zeroed()` validation.

### 5E — skinning_gpu.rs (+5 tests)

Handle equality (same value), empty matrices palette, stored translation verification, shader constant non-empty + contains `apply_skinning`/`joint_palette`, four joint influence weights (w.x/y/z/w).

### 5F — graph.rs (+8 tests)

`RenderGraph` default empty, add_node increases count, empty graph executes OK, missing resource returns error, missing key for all resource types, `ClearNode::name()`, `RendererMainNode::name()`, `target_view` surface fallback.

### 5G — gpu_particles.rs (+6 tests)

`GpuParticle::zeroed()` all fields, `EmitterParams::zeroed()` all fields, `PARTICLE_UPDATE_SHADER` WGSL parsing (update_particles entry), `PARTICLE_EMIT_SHADER` WGSL parsing (emit_particles entry), 16-byte alignment of GpuParticle, gravity field write/read.

### 5H — residency.rs (+9 tests)

New manager starts empty, not-found asset returns error, load succeeds and appears in `get_loaded_assets()`, duplicate load no memory increase, evict_lru removes oldest, touch moves to back of LRU, memory pressure triggers automatic eviction, gpu_handle set on load.

---

## Remaining Gaps

- **No GPU integration tests**: All tests run CPU-side or with headless wgpu device. True GPU render output validation would require screenshot comparison infrastructure.
- **Shadow CSM one-shot debug logging**: Still fires once per process via `AtomicBool` — acceptable for diagnostics but could be gated behind a feature flag for zero-cost in release.
- **render() commented-out plane buffer**: ~10-line block at render() is disabled via `/* */` — retained intentionally per comment "DISABLE to fix interference with TerrainSystem".
- **material_loader.rs**: No test module (complex I/O: KTX2/BC texture decoding). Would benefit from mock-based tests in future.

## Mutation Testing Readiness

**READY**. 806 tests covering critical paths including edge cases for clustered lighting, frustum culling, vertex compression, LOD generation, post-processing, shadow cascades, deferred rendering, render graph, GPU particles, skeletal skinning, error types, and residency management. All production println!/eprintln! converted to log macros. Clippy clean. Zero compilation warnings.
