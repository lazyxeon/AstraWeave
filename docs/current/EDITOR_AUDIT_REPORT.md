# AstraWeave Editor Audit Report

**Date**: 2026-04-03
**Scope**: `tools/aw_editor/` — 169,832 lines across 110+ Rust source files, 11 WGSL shaders
**Version**: v0.10.0 | Rust 1.89.0

---

## Executive Summary

The `aw_editor` crate is a **substantial, functional editor** with 3,970 tests across 108 test modules and a sophisticated rendering pipeline with PBR lighting, IBL, shadow mapping, and a post-processing chain. The codebase demonstrates significant maturity — many of the 17 previously-identified rendering bugs have been fixed, the `.unwrap()` discipline is excellent (292 total, **all in test code**), and the shader implementations are correct and feature-rich.

However, the crate has five systemic issues: (1) a **monolithic main.rs** (9,378 lines, `#![allow(dead_code)]`, 0% mutation kill rate) that concentrates untestable logic, (2) **pervasive `#![allow(dead_code)]` annotations** across 15+ files, (3) **CPU-side mipmap generation** in both renderers, (4) the **post-process chain operates on LDR data** instead of HDR, and most critically (5) **the undo system is largely non-functional** — entity creation, transform edits, and property changes all bypass the undo stack despite undo commands existing, and undo of delete corrupts EntityManager state. Additionally, 18 of 50 PanelEvent handlers are status-string-only stubs that perform no actual logic.

**Hard Counts**: 5 Critical, 14 High, 42 Medium, 14 Low findings. 2 TODOs, 2 FIXMEs, 0 stubs/todo!(), 0 unimplemented!(), 6 `filter_map(.ok())` silent error swallowing, 21 `eprintln!` debug diagnostics, 83 `#[allow(dead_code)]`/`#[allow(unused_imports)]` annotations, 1 production `.unwrap()`, 14 stub implementations, 36 non-functional UI buttons, 1 duplicate hotkey handler, 310 lines of dead renderer code.

---

## Critical Findings (Must Fix Before Any Release)

### C-1: Terrain shader `dominant_biome_layer()` skips Grassland (index 0)

**File**: `viewport/shaders/terrain.wgsl:189-201`
**Severity**: CRITICAL
**Category**: logic_bug

The function initializes `best_index = 1` (Desert) and `best_weight = weights_0.y` (Desert weight), then checks indices 1-7 — but **never checks `weights_0.x` (index 0, Grassland)**. Line 194 (`if weights_0.y > best_weight`) is a self-comparison that is always false.

This means: at the far LOD distance, any Grassland-dominant biome silently renders as Desert. The full-resolution `blend_biome_materials()` function at line 360-375 correctly handles all 8 indices, so this only affects distant terrain but causes a visible biome pop at the LOD crossfade boundary.

**Fix**: Add `if weights_0.x > best_weight { best_weight = weights_0.x; best_index = 0; }` as the first check, and remove the redundant line 194.

### C-2: `main.rs` — 9,378 lines with `#![allow(dead_code)]` and 0% mutation kill rate

**File**: `main.rs:1`
**Severity**: CRITICAL
**Category**: dead_code, testability

The binary entrypoint has a crate-wide `#![allow(dead_code)]` that suppresses warnings for the **entire file**. It contains the full `EditorApp` struct (~50+ fields) and its implementation, including:
- GUI layout and panel orchestration
- File I/O (save, load, auto-save, crash recovery)
- Blend import thread management
- Asset database scanning
- Viewport material/environment parameter wiring

None of this is testable from `lib.rs`. The 624 surviving mutants represent **real logic** with zero coverage. The `EditorApp::Default` implementation alone (lines 469-570) contains initialization logic that should be validated.

**Fix**: Extract `EditorApp` into `editor_app.rs` as a `pub struct` in `lib.rs`. Extract the blend import thread logic, file I/O operations, and environment wiring into separate testable modules. Remove `#![allow(dead_code)]` and fix or delete any actual dead code.

### C-3: Duplicate Ctrl+D handler creates ghost entities

**File**: `main.rs:8832-8857` and `main.rs:9117-9139`
**Severity**: CRITICAL
**Category**: copy_paste, logic_bug

Two separate `Ctrl+D` handlers exist in the same `ctx.input()` closure:
1. **First** (line 8832): Properly duplicates via clipboard + undo stack with `DuplicateEntitiesCommand` — creates entities in World
2. **Second** (line 9117): Uses `entity_manager.create()` for a shallow copy — creates entities **only in EntityManager**, not in World

Both fire on every `Ctrl+D` press. The second handler overwrites the selection with a "ghost" entity that exists in the entity list but is invisible in the viewport/scene hierarchy. This is an active, user-facing bug.

**Fix**: Delete the second handler at lines 9117-9139 entirely.

### C-4: Undo of entity delete corrupts editor state (EntityManager desync)

**File**: `main.rs:4195-4218`, `command.rs:1091`
**Severity**: CRITICAL
**Category**: logic_bug (cross-file)

When `DeleteEntity` fires:
1. `entity_manager.remove(entity_id)` — immediately removes from EntityManager
2. `undo_stack.execute(delete_cmd)` — pushes to undo stack

When **undo** is called, `DeleteEntitiesCommand::undo()` restores the entity in `World` via `spawn_with_id()`, but **never restores it in EntityManager**. The entity exists in the ECS World but is invisible to all editor panels (hierarchy, inspector, selection). This applies to all undo/redo operations on entities — `SpawnEntitiesCommand::undo()` and `DuplicateEntitiesCommand::undo()` also only operate on World.

Additionally, entity removal from EntityManager happens BEFORE `undo_stack.execute()`. If execute fails, the entity is already gone with no recovery.

**Fix**: All entity commands must update both World AND EntityManager in both `execute()` and `undo()`. Remove the pre-execution EntityManager mutation.

### C-5: Entity creation and most property edits bypass undo stack entirely

**File**: `main.rs:3939-4558`
**Severity**: CRITICAL
**Category**: logic_bug (cross-file)

The following operations directly mutate state **without pushing to the undo stack**, despite undo commands existing for many of them:

| Operation | Handler Lines | Undo Command Exists? |
|-----------|--------------|---------------------|
| Transform position/rotation/scale | 3939-3995 | Yes (`MoveEntityCommand`, etc.) — NOT USED |
| CreateEntity | 3997-4030 | Yes (`SpawnEntitiesCommand`) — NOT USED |
| SpawnArchetype | 4032-4140 | Yes — NOT USED |
| SpawnModel | 4142-4193 | Yes — NOT USED |
| HealthChanged | 4417-4429 | Yes (`EditHealthCommand`) — NOT USED |
| TeamChanged | 4430-4443 | Yes (`EditTeamCommand`) — NOT USED |
| AmmoChanged | 4444-4459 | Yes (`EditAmmoCommand`) — NOT USED |
| EntityRenamed | 4548-4558 | Yes (`RenameEntityCommand`) — NOT USED |
| MaterialPropertyChanged | 4460-4468 | No |
| ComponentDataChanged | 4469-4478 | No |

This means **Ctrl+Z does nothing** for the most common editor operations: moving entities, creating entities, and editing properties. The undo system infrastructure exists but is not wired to the event handlers.

**Fix**: Replace direct state mutations in event handlers with the corresponding undo command executions.

---

## High Severity Findings

### H-0: Dangerous `let _ =` discarding meaningful Results in main.rs

**File**: `main.rs` — lines 1226, 2200, 7438, 7810-7812, 7918, 8559
**Severity**: HIGH
**Category**: error_swallowing

Several `let _ =` patterns discard results that indicate real failures:
- **Line 1226**: Lock file write — crash recovery won't work if this fails
- **Line 2200**: Undo triggered by toast action — failure silently ignored
- **Line 7438**: Material auto-sync to viewport — error swallowed (but manual "Apply" button properly handles errors)
- **Lines 7810-7812**: Asset scan + manifest save both silently ignored — user sees "Rescanned" but may have no assets
- **Line 7918**: Hot-reload signal file write silently ignored
- **Line 8559**: Prefab revert during hot-reload — user thinks update succeeded

**Fix**: At minimum, log errors with `tracing::warn!` for each. Critical paths (lock file, asset scan, prefab revert) should propagate errors to the user via toast notifications.

### H-1: CPU-side mipmap generation using box filter

**Files**: `viewport/terrain_renderer.rs:969`, `viewport/entity_renderer.rs:2085`
**Severity**: HIGH
**Category**: performance

Both renderers generate full mipmap chains on the CPU using a simple box filter. For terrain textures (2048x2048 x 22 layers x 3 channels = albedo + normal + MRA), this is approximately **825 MB** of data processed through a nested loop on the CPU at load time. The entity renderer does the same for every glTF mesh texture.

Issues:
1. **Performance**: CPU mipmap generation is orders of magnitude slower than GPU compute mipmaps
2. **Quality**: Box filter in sRGB space is mathematically incorrect — should convert to linear, downsample, then back to sRGB. This affects albedo/emissive textures (created with `Rgba8UnormSrgb` format)
3. **Blocking**: Both run synchronously on the main thread (terrain) or worker thread (entity), causing frame hitches during loading

**Fix**: Use a GPU compute shader or `wgpu::CommandEncoder::copy_texture_to_texture()` with render-to-mip passes. Or use `wgpu::TextureUsages::RENDER_ATTACHMENT` and render a fullscreen quad at each mip level.

### H-2: 15+ files with crate-wide `#![allow(dead_code)]`

**Files**: `main.rs:1`, `lib.rs:2`, `terrain_integration.rs:1`, `viewport/renderer.rs:6`, `viewport/entity_renderer.rs:5`, `viewport/widget.rs:6`, `viewport/water_renderer.rs:1`, `viewport/rain_renderer.rs:1`, `viewport/skybox_renderer.rs:6`, `viewport/grid_renderer.rs:5`, `viewport/gizmo_renderer.rs:6`, `viewport/physics_renderer.rs:11`, `viewport/weather_particle_renderer.rs:1`, `gizmo/rotate.rs:3`, `gizmo/scale.rs:3`
**Severity**: HIGH
**Category**: dead_code

These annotations blanket-suppress dead code warnings across thousands of lines. While `lib.rs` has a reasonable explanation ("benchmark-only exports"), the viewport renderers use it to mask potentially significant amounts of unused code. Without removing these annotations and analyzing the resulting warnings, there's no way to know how much dead code exists.

**Fix**: Remove `#![allow(dead_code)]` from each file. For each resulting warning, either (a) delete truly dead code, (b) add a targeted `#[allow(dead_code)]` with a justifying comment, or (c) add a `pub` export if the code is used externally.

### H-3: `panels/mod.rs` — 30+ `#[allow(unused_imports)]` (reassessed)

**File**: `panels/mod.rs:80-247`
**Severity**: MEDIUM (downgraded from HIGH after investigation)
**Category**: dead_code

Every panel module import in `panels/mod.rs` is wrapped in `#[allow(unused_imports)]`. Investigation reveals these are **deliberate public API re-exports** — e.g., `pub use foliage_panel::FoliageAction` so consumers can write `use panels::FoliageAction`. The pattern is forward-looking for plugin consumers.

**Fix**: Still worth auditing — remove annotations, compile, and verify which re-exports have actual downstream consumers vs. which are speculative.

### H-4: Post-process chain receives `None` for normal G-buffer

**File**: `viewport/renderer.rs:647`
**Severity**: HIGH
**Category**: stub

```rust
None, // TODO: wire normal G-buffer when available
```

The GTAO (Ground Truth Ambient Occlusion) pass in the post-processing chain receives no normal buffer, which means it either (a) falls back to screen-space depth-only AO (lower quality), or (b) is disabled entirely. This degrades ambient occlusion quality across the entire viewport.

### H-5: Post-process chain operates on LDR data instead of HDR

**File**: `viewport/renderer.rs:413-418`
**Severity**: HIGH
**Category**: rendering

All scene passes render to `Bgra8UnormSrgb` (LDR) directly. The `PostProcessChain` creates an `Rgba16Float` HDR target but it is **never used as the scene render target**. This means GTAO, bloom, and god rays operate on already-tonemapped 8-bit data:
- Bloom thresholds see clamped [0,1] values instead of HDR highlights
- Auto-exposure has no dynamic range to work with
- God rays produce banding from 8-bit precision

The comment at line 416 acknowledges this: *"Full HDR pipeline would require refactoring all sub-renderer pipeline formats to Rgba16Float — deferred to a future pass."*

**Fix**: Refactor all sub-renderer pipelines to render to `Rgba16Float`, apply post-processing in HDR space, then tonemap as the final step.

### H-6: Skybox HDRI tonemapped to 8-bit sRGB, losing HDR data

**File**: `viewport/skybox_renderer.rs:243-266`
**Severity**: HIGH
**Category**: rendering

HDR images (Rgb32F) are Reinhard-tonemapped and gamma-corrected to 8-bit RGBA, then uploaded as `Rgba8UnormSrgb`. This destroys the dynamic range that makes HDR skyboxes useful for IBL/reflections. The SH9 irradiance coefficients are computed from the HDR data before conversion (which is correct), but specular reflections from the skybox use the 8-bit version.

**Fix**: Upload HDR data as `Rgba16Float` or `Rgba32Float` and keep full dynamic range for reflection lookups.

### H-7: Entity shader normal transform does not use inverse-transpose

**File**: `viewport/shaders/entity.wgsl:402`
**Severity**: HIGH
**Category**: shader_correctness

```wgsl
let world_normal = (model_matrix * vec4<f32>(vertex.normal, 0.0)).xyz;
```

For non-uniform scale transforms (e.g., scale X=2, Y=1, Z=1), normals should use the inverse-transpose of the model matrix. The current code produces incorrect normals with non-uniform scaling, causing lighting artifacts on stretched entities.

**Fix**: Either pass the inverse-transpose as an additional instance attribute, or compute it in the vertex shader from the model matrix.

### H-8: `ray_from_screen` uses wrong NDC Z for wgpu [0,1] depth

**File**: `viewport/camera.rs:461-462`
**Severity**: HIGH
**Category**: logic_bug

`ray_from_screen` uses `z = -1.0` for the near plane and `z = 1.0` for the far when unprojecting. Since `Mat4::perspective_rh` produces wgpu-compatible [0,1] depth (near=0, far=1), the near-plane Z should be `0.0`, not `-1.0`. The ray **direction** is still roughly correct (both points are on the same screen ray), but the ray **origin** is wrong — behind the actual near plane. This causes:
- Picking inaccuracies for objects close to the camera
- Incorrect t-values in ray-AABB intersection tests
- The grid shader (`grid.wgsl:58`) has the same issue (`z = -1.0` for near plane)

**Fix**: Change near-plane Z from `-1.0` to `0.0` in both `camera.rs:461` and `grid.wgsl:58`.

### H-9: 5 GPU texture chains allocated but never used in post-processing

**File**: `viewport/post_process.rs:197-247`
**Severity**: HIGH
**Category**: performance, dead_code

The `PostProcessChain` allocates and resizes on every viewport resize: `hdr_texture`/`hdr_view` (Rgba16Float), `bloom`, `auto_exposure`, `volumetric_fog`, and `hdr_pipeline` fields — **none of which are used in `execute()`**. The `execute` method acknowledges this: "Bloom and tonemap require an HDR intermediate to be effective. Currently the scene renders in LDR." This wastes significant GPU memory (multiple full-resolution float textures) for features that are allocated but never invoked.

**Fix**: Wrap unused resources in `Option<T>`, only allocate when HDR pipeline migration is complete.

### H-10: EntityPanel transform edits snap back every frame

**File**: `tab_viewer.rs:7144-7206`
**Severity**: HIGH
**Category**: logic_bug

The `EntityPanel` branch clones `self.selected_entity_info` into a snapshot, then binds DragValue widgets to the snapshot's position/rotation/scale fields. When the user drags a value, the mutation happens to the local snapshot, a `PanelEvent` is emitted, but `selected_entity_info` is **never updated** with the new value. On the next frame, the old snapshot value overwrites the edit, causing the UI to "snap back" after every drag.

The `Transform` panel (line 5608) correctly uses `self.selected_transform` which is mutable and persists across frames. The `EntityPanel` does not follow this pattern.

**Fix**: Either update `self.selected_entity_info` after editing, or switch the EntityPanel to use `self.selected_transform` like the Transform panel does.

### H-11: 12 BehaviorGraph/material buttons with discarded `.clicked()` results

**File**: `tab_viewer.rs:8735-8738` (4 BehaviorGraph toolbar buttons), plus 8 non-functional buttons already counted in M-4
**Severity**: HIGH
**Category**: dead_code

```rust
ui.button("Sequence").clicked();
ui.button("Selector").clicked();
ui.button("Action").clicked();
ui.button("Condition").clicked();
```

These call `.clicked()` but discard the boolean return value. The buttons render and respond to mouse clicks with visual feedback, but no action occurs. Combined with the gradient/easing preset buttons (M-4), there are **20 non-functional buttons** in tab_viewer.rs alone.

### H-12: 18 of 50 PanelEvent handlers are status-string-only (no actual logic)

**File**: `main.rs:4277-4615`
**Severity**: HIGH
**Category**: stub (cross-file)

18 PanelEvent handlers only update `self.status` with a descriptive string but perform **no actual logic**. This means these editor features are non-functional via the panel event pathway:

- **Viewport controls**: `ViewportGizmoModeChanged`, `ViewportGizmoSpaceChanged`, `ViewportOverlayToggled`, `ViewportCameraChanged`, `ViewportFocusOnSelection`, `ViewportResetCamera`, `ViewportCameraPreset` — toolbar buttons update status text but don't affect the viewport
- **Build**: `BuildRequested` — build button updates status but never starts a build
- **Animation**: `AnimationKeyframeAdded` — keyframe reported added but never inserted into data
- **Console**: `ConsoleCleared` — console NOT actually cleared despite the event name
- **Material**: `MaterialChanged` — material data silently discarded (compare with `MaterialPropertyChanged` which does work)
- Others: `AnimationFrameChanged`, `BehaviorNodeSelected`, `GraphNodeSelected`, `HierarchySearchChanged`, `ConsoleSearchChanged`, `RefreshSceneStats`, `AssetSelected`

### H-13: Spawn/Duplicate undo moves entities offscreen instead of destroying them

**File**: `command.rs:998-1017, 1049-1068`
**Severity**: HIGH
**Category**: logic_bug

`SpawnEntitiesCommand::undo()` and `DuplicateEntitiesCommand::undo()` "undo" by moving entities to position `(-10000, -10000)` with `scale = 0.0`. They don't actually destroy the entity. This means:
- Entity still exists in World (occupies memory, iterated by systems)
- Entity count never decreases
- World accumulates ghost entities over repeated paste/duplicate/undo cycles

### H-14: `charts_panel.rs:223` — `.unwrap()` on `partial_cmp` can panic on NaN

**File**: `panels/charts_panel.rs:223`
**Severity**: HIGH
**Category**: unwrap_in_prod

```rust
sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

This is the **only production-path `.unwrap()`** in the entire crate. If any chart data contains `NaN` (which floating-point operations can produce), this will panic and crash the editor. The fix is trivial: `a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)` or use `total_cmp()`.

---

## Medium Severity Findings

### M-1: 21 `eprintln!` calls used instead of `tracing`

**Files**: `main.rs:4688,4690,6108,6160,9318`, `terrain_integration.rs:1649-2307`, `viewport/renderer.rs:1160,1166,1173`, `viewport/terrain_renderer.rs:190,236,1276`
**Severity**: MEDIUM
**Category**: println_debug

Production code uses `eprintln!` for diagnostics that should use `tracing::debug!` or `tracing::warn!`. These bypass the structured logging pipeline, cannot be filtered by log level, and pollute stderr in production. The main.rs biomepack diagnostics (lines 4688-4690) and blend import diagnostics (lines 6108, 6160) are particularly egregious — they print raw debug data to stderr.

**Fix**: Replace all `eprintln!` with appropriate `tracing` macros. `main.rs:9318` (observability init failure) is the only acceptable use of `eprintln!` since the tracing system itself has failed.

### M-2: 6 instances of `filter_map(.ok())` silently dropping errors

**Files**: `asset_pack.rs:332`, `audio_bridge.rs:23`, `panels/asset_browser.rs:724`, `main.rs:1139,1185,8233`
**Severity**: MEDIUM
**Category**: error_swallowing

These all iterate over `ReadDir` results and silently drop any `io::Error` from directory entries. While individual entry errors are rarely catastrophic, they can mask:
- Permission denied errors on specific files
- Filesystem corruption
- Symlink loops

**Fix**: Replace with `.filter_map(|e| e.map_err(|err| tracing::warn!("dir entry error: {err}")).ok())` or log at debug level.

### M-3: Terrain rendered without back-face culling (double draw cost)

**File**: `viewport/terrain_renderer.rs:795`
**Severity**: MEDIUM
**Category**: performance

```rust
cull_mode: None, // No culling — terrain mesh winding order is mixed/CW
```

The comment acknowledges this is a workaround for inconsistent winding order in generated terrain meshes. Every terrain triangle is rasterized twice (front and back face), which doubles the fragment shader cost for terrain — the most expensive render pass.

**Fix**: Fix the terrain mesh generator's winding order consistency, then set `cull_mode: Some(wgpu::Face::Back)`.

### M-4: `let _ = ui.small_button(...)` — buttons with discarded click results

**File**: `tab_viewer.rs:7993-7996, 8027-8030`
**Severity**: MEDIUM
**Category**: stub

Eight UI buttons ("Sunset", "Ocean", "Forest", "Fire", "Linear", "Ease In", "Ease Out", "Ease In/Out") whose click results are discarded with `let _ =`. These buttons render in the UI but do **nothing** when clicked. This is a known AI code generation pattern — the UI was created but the behavior was never implemented.

### M-5: Legacy `toasts` field with `#[allow(dead_code)]`

**File**: `main.rs:388-390`
**Severity**: MEDIUM
**Category**: dead_code

```rust
// Legacy toasts field (kept for backward compatibility during migration)
#[allow(dead_code)]
toasts: Vec<Toast>,
```

The editor has migrated to `ui::ToastManager` (line 387) but keeps the old `Vec<Toast>` field and its `Toast` type definition (lines 33-55). The "backward compatibility" comment is misleading — there is no external API that depends on this field.

### M-6: `let _ = self.undo_stack.undo(state.world_mut())` — undo result discarded

**File**: `main.rs:2200`
**Severity**: MEDIUM
**Category**: error_swallowing

The undo operation's `Result` is silently discarded. If undo fails (e.g., corrupted command stack, invalid entity reference), the user sees no feedback and the scene state may be inconsistent.

### M-7: Scatter renderer `cull_mode: None` for vegetation

**File**: `viewport/scatter_renderer.rs:572`
**Severity**: MEDIUM
**Category**: performance

```rust
cull_mode: None, // Two-sided for vegetation
```

While two-sided rendering is intentional for vegetation leaves, it doubles the fragment cost. The scatter shader already handles two-sided lighting via the vertex normal. Consider using `cull_mode: Some(wgpu::Face::Back)` with the shader adjusting the normal for back faces (like entity.wgsl does at line 529-531).

### M-8: Ctrl+S toast goes to dead legacy system

**File**: `main.rs:8756-8764`
**Severity**: MEDIUM
**Category**: dead_code, logic_bug

The `Ctrl+S` hotkey handler pushes to `self.toasts` (the legacy `Vec<Toast>`) instead of `self.toast_manager`. Since the legacy field is `#[allow(dead_code)]` and never rendered, the user **sees no save confirmation** when pressing Ctrl+S. The menu "File > Save" correctly uses the new toast system. This inconsistency means power users (who use keyboard shortcuts) get worse feedback than mouse users.

**Fix**: Replace `self.toasts.push(Toast::new(...))` with `self.toast_manager.success(...)`.

### M-9: GPU resource usage display shows zeros (stub)

**File**: `main.rs:873-877`
**Severity**: MEDIUM
**Category**: stub

`sample_resource_usage()` always sets GPU stats to zero: `gpu_memory_used = 0`, `gpu_memory_total = 0`, `gpu_utilization = 0.0`. The status bar displays these zeros as real data. On non-Windows platforms, CPU memory is also zero.

**Fix**: Either implement platform-specific GPU queries, or display "N/A" in the UI instead of misleading zeros.

### M-10: Voxel undo/redo buttons are stubs

**File**: `main.rs:7700-7709`
**Severity**: MEDIUM
**Category**: stub

The Voxel Editor's Undo and Redo buttons log "(integration pending)" but perform no action. The `can_undo()`/`can_redo()` gates are functional, but the actual operations are not implemented.

### M-11: `LevelDoc.title` used as hierarchy search field

**File**: `main.rs:3419`
**Severity**: MEDIUM
**Category**: logic_bug

The hierarchy panel search bar is bound to `self.level.title`. Typing in the search bar modifies the level title, and the level title serves as the search query. These should be separate fields.

### M-12: `on_open()` hardcoded to single file path

**File**: `main.rs:7883`
**Severity**: MEDIUM
**Category**: placeholder

`on_open()` hardcodes `content/levels/forest_breach.level.toml`. `Ctrl+O` properly opens a file dialog, but `File > Open` from the menu uses the hardcoded path.

### M-13: 22 additional stub buttons in tab_viewer.rs

**File**: `tab_viewer.rs` — lines 4742, 4783, 6615, 6622, 6801, 6810, 6941, 6955, 7715, 7872, 8371, 8374, 8407-8454
**Severity**: MEDIUM
**Category**: stub

22 buttons display interactive UI but perform no action: "Focus in Viewport", entity rename on double-click, material duplicate/delete/save/export, clear/select texture, cancel build, open output folder, add track/keyframe, 6 graph node operation buttons, align nodes, fit view. Several have comments like "Would start rename mode" suggesting the AI generated the UI but not the implementation.

### M-14: "Clear All" button says "requires confirmation" but has none

**File**: `tab_viewer.rs:6396-6404`
**Severity**: MEDIUM
**Category**: logic_bug

The "Clear All" button in the World panel shows "requires confirmation" in its hover text, but immediately deletes all entities without any confirmation dialog. This is a data-loss risk.

### M-15: Budget progress bar shows 100% budget as "50%"

**File**: `tab_viewer.rs:5055-5066, 6452-6464`
**Severity**: MEDIUM
**Category**: logic_bug

`budget_pct = frame_time_ms / 16.67` clamped to 0-2, then the progress bar uses `budget_pct / 2.0` as fill fraction and displays `budget_pct * 50.0` as percentage. A frame exactly at 16.67ms (100% budget) shows as "50%". The same error appears in the Performance panel.

### M-16: `hierarchy_search` shared between hierarchy and Add Panel popup

**File**: `tab_viewer.rs:9212`
**Severity**: MEDIUM
**Category**: logic_bug

`add_popup()` reuses `self.hierarchy_search` as the panel search filter. Typing in the "Add Panel" popup pollutes the hierarchy search field and vice versa.

### M-17: Animation `playback_speed` field is cosmetic only

**File**: `tab_viewer.rs:1090, 3448`
**Severity**: MEDIUM
**Category**: stub

`playback_speed` field exists and is adjustable in the UI, but `update_animation()` never reads it. Speed changes have no effect on playback.

### M-18: `EditorTabViewer` is a God-struct with 90+ fields

**File**: `tab_viewer.rs:1255-1488`
**Severity**: MEDIUM
**Category**: architecture

The struct owns UI state for every panel type, terrain state, build system, animation, graph editor, world environment, viewport, console, 18 panel objects, and play session profiling. This makes it impossible to test panels in isolation or refactor any panel without touching this 9,548-line file.

### M-19: `AssetRegistry` is just a counter wrapper

**File**: `main.rs:260-274`
**Severity**: MEDIUM
**Category**: dead_code

```rust
#[derive(Default)]
struct AssetRegistry {
    count: usize,
}
impl AssetRegistry {
    fn count(&self) -> usize { self.count }
    #[allow(dead_code)]
    fn set_count(&mut self, count: usize) { self.count = count; }
}
```

A struct with one field and `#[allow(dead_code)]` on its only setter. This should either be expanded into a real registry or replaced with a `usize` field directly.

### M-20: `gizmo/picking.rs` — 8 `#[allow(dead_code)]` are false positives

**File**: `gizmo/picking.rs:14,128,171,178,220,243,376,416`
**Severity**: MEDIUM (code hygiene)
**Category**: dead_code

All 8 `#[allow(dead_code)]` items in picking.rs are **false positives** — every item is actively used by `scene_viewport.rs` and re-exported via `mod.rs`. The annotations were added during iterative development before integration and never cleaned up. Same issue in `gizmo/rotate.rs:3` and `gizmo/scale.rs:3` — file-level `#![allow(dead_code)]` where all public APIs are used.

**Genuinely dead**: `gizmo/input.rs` `NumericInput` struct (lines 4-10) — entirely unused, duplicated by `GizmoState::numeric_buffer`. `gizmo/constraints.rs:7` `apply_constraint()` — only used in tests.

### M-21: ScaleGizmo can only scale UP via mouse drag

**File**: `gizmo/scale.rs`
**Severity**: MEDIUM
**Category**: logic_bug

`ScaleGizmo::calculate_scale()` uses `mouse_delta.length()` which is always positive. There is no mechanism to scale down via mouse drag — only numeric input supports shrinking. This is a design limitation documented in tests but affects usability.

### M-22: `main.rs` duplicates all module declarations from `lib.rs`

**File**: `main.rs:71-100+`
**Severity**: MEDIUM
**Category**: architecture

`main.rs` has its own `mod` declarations for every module (`mod animation_bridge`, `mod asset_pack`, etc.) instead of importing from `aw_editor_lib`. This means every module is compiled **twice** — once for the library target, once for the binary. This doubles compile times for the editor crate and risks the library and binary module trees diverging.

Additionally, modules declared only in `main.rs` (not `lib.rs`) — `brdf_preview.rs`, `voxel_tools.rs`, `material_inspector.rs`, `splash.rs`, `dialogs.rs`, `tutorial.rs`, `dock_panels.rs` — cannot be tested via `cargo test -p aw_editor` since the binary has `test = false`.

**Fix**: Remove all `mod` declarations from `main.rs` and `use aw_editor_lib::*` or specific imports instead.

### M-23: `unwrap_used = "allow"` clippy lint at crate level

**File**: `Cargo.toml:10`
**Severity**: MEDIUM
**Category**: code_quality

`[lints.clippy] unwrap_used = "allow"` suppresses the clippy lint for the **entire crate**, not just test code. While the comment says "tests use unwrap() extensively", this override also applies to production code paths, undermining the project's zero-production-unwrap policy.

**Fix**: Remove crate-level override, add `#[allow(clippy::unwrap_used)]` only inside `#[cfg(test)]` modules.

### M-24: Unused dependencies `astraweave-author` and `rhai`

**File**: `Cargo.toml:43,56`
**Severity**: MEDIUM
**Category**: dead_code

`astraweave-author` (line 56) is listed as a non-optional dependency but has **zero imports** in any source file. It's also in the workspace `exclude` list due to known Rhai `Sync` errors. `rhai` (line 43) likewise has zero `use rhai` references. Both increase compile time for no benefit.

### M-25: Build cancel button is non-functional

**File**: `panels/build_manager.rs:452`
**Severity**: MEDIUM
**Category**: logic_bug

The `run_build()` function accepts a `_cancel_flag: Arc<AtomicBool>` parameter but **never reads it**. The leading underscore suppresses the unused-variable warning. The cancel button in the UI sets the flag, but the build thread runs to completion regardless.

**Fix**: Add periodic `if cancel_flag.load(Ordering::Relaxed) { break; }` checks in the build loop.

### M-26: Profiler flame graph divides by zero without guard

**File**: `panels/profiler_panel.rs:866`
**Severity**: MEDIUM
**Category**: logic_bug

```rust
let node_width = (node.time_ms / total_time) * total_width;
```

`total_time` comes from `root.time_ms`. If the root node has `time_ms == 0.0`, this produces NaN. The bar chart path at line 455 correctly guards with `if total <= 0.0 { return; }`, but the flame graph path does not.

### M-27: Animation `total_frames` division by zero

**File**: `tab_viewer.rs:8279,8298`
**Severity**: MEDIUM
**Category**: logic_bug

Timeline rendering divides by `total_frames as f32`. If `total_frames` is set to 0 by the user, this produces NaN/Inf, causing rendering artifacts in the animation timeline.

### M-28: Terrain texture arrays estimated at ~726 MB GPU memory

**File**: `viewport/terrain_renderer.rs:94`
**Severity**: MEDIUM
**Category**: performance

`BIOME_TEX_SIZE: u32 = 2048` with 22 layers, 3 arrays (albedo, normal, MRA), 11 mip levels. Estimated GPU memory: approximately 726 MB (22 x 3 x 2048x2048 x 4 bytes x 1.33 mip factor). This is a significant GPU memory commitment that may cause issues on lower-end hardware.

### M-29: `rain_renderer.rs` is entirely dead code (310 lines)

**File**: `viewport/rain_renderer.rs`
**Severity**: MEDIUM
**Category**: dead_code

The entire 310-line file is unused. `weather_particle_renderer.rs` handles all weather types including rain. The module is declared in `mod.rs` but never imported or used by `renderer.rs`.

### M-30: glTF tangent attributes silently ignored

**File**: `viewport/entity_renderer.rs`
**Severity**: MEDIUM
**Category**: rendering

No tangent attribute is loaded from glTF meshes. If a mesh has explicit tangents (which many do), they are silently ignored. The shader works around this using screen-space derivative cotangent frames, which degrades on flat surfaces.

### M-31: ORM occlusion from separate glTF texture silently ignored

**File**: `viewport/entity_renderer.rs:1903-1906`
**Severity**: MEDIUM
**Category**: rendering

Comment: "TODO: Composite separate occlusion + metallic-roughness into ORM at load time." The code assigns `orm_view = mr_view` directly. glTF assets with a separate occlusion texture (common in high-quality assets) will have occlusion silently ignored, defaulting to 1.0.

### M-32: Weather particle billboard NaN when particle at camera

**File**: `viewport/shaders/weather_particles.wgsl:100-101`
**Severity**: MEDIUM
**Category**: shader_correctness

Billboard calculation uses `normalize(cam_rel)` where `cam_rel` is the camera-to-particle vector. When a particle is exactly at the camera position, this normalizes a zero vector producing NaN for the billboard axes, cascading to NaN vertex positions and rendering artifacts.

### M-33: GTAO silently skipped when normal buffer is None

**File**: `viewport/post_process.rs:228-233`
**Severity**: MEDIUM
**Category**: silent_failure

When `normal_view` is `None`, GTAO does nothing with no log warning. Since renderer.rs always passes `None` (H-4), AO is silently disabled with no diagnostic.

### M-34: `aw_game_runtime.rs` is non-functional (stub render loop)

**File**: `bin/aw_game_runtime.rs:167-225`
**Severity**: MEDIUM
**Category**: stub

The game runtime binary only clears the screen to a dark color. No scene rendering, no ECS integration, no game logic. The entry scene path is logged but never loaded. GPU init uses `panic!` instead of graceful error handling. CLI flags (`--fullscreen`, `--skip-splash`, `--dev`) are parsed but never used.

### M-35: Camera `set_aspect` does not guard against zero height

**File**: `viewport/camera.rs:309-310`
**Severity**: MEDIUM
**Category**: logic_bug

If `height == 0.0`, produces `aspect = inf`. The `sanitize()` method checks `aspect <= 0.0` but not `inf`, so a degenerate projection matrix propagates to all rendering.

### M-36: RemoveComponent not synced to ECS World

**File**: `main.rs:4402-4415`
**Severity**: MEDIUM
**Category**: logic_bug (cross-file)

`RemoveComponent` removes the component from `EntityManager` but NOT from `World`. For core types (Health, Team, Ammo), the component persists in ECS. If saved and reloaded, the "removed" component reappears.

### M-37: EntityRenamed not synced to ECS World

**File**: `main.rs:4548-4558`
**Severity**: MEDIUM
**Category**: logic_bug (cross-file)

`EntityRenamed` updates `entity_manager` name but does NOT call `scene_state.world_mut().set_name()`. World and EntityManager names diverge.

### M-38: ComponentDataChanged not synced to ECS World

**File**: `main.rs:4469-4478`
**Severity**: MEDIUM
**Category**: logic_bug (cross-file)

`ComponentDataChanged` only updates `EntityManager.components` (JSON data). Light, Collider, RigidBody, Audio, Camera, Script, Particle data edited in the inspector never propagates to ECS. In-editor component edits are cosmetic if World is the source of truth.

### M-39: No path traversal protection outside scene_serialization

**Files**: `prefab.rs:410`, `polish.rs:381-384`, `game_project.rs:177`, `behavior_graph/document.rs:492`, `distribution.rs:364-428`, `panels/build_manager.rs:482-608`
**Severity**: MEDIUM
**Category**: security

Only `scene_serialization.rs` has `safe_under()` and `validate_extension()` path validation. All other file I/O accepts user-provided paths without sanitization. `polish.rs` save slot names could contain `../` for path traversal.

### M-40: No atomic writes in any save codepath

**Files**: `scene_serialization.rs:172`, `prefab.rs:410`, `editor_preferences.rs:75`, `polish.rs:401-408`, `behavior_graph/document.rs:492`
**Severity**: MEDIUM
**Category**: data_integrity

None of the file writes use write-to-temp-then-rename. A crash or power loss during `fs::write()` leaves truncated/corrupted files with no recovery. This affects scene saves, prefab saves, preferences, and behavior graph saves.

### M-41: Entity ID type mismatch — u64 truncated to u32 throughout

**File**: `main.rs` — lines 3942, 3963, 3982, 4199, 4230, 4378, etc.
**Severity**: MEDIUM
**Category**: logic_bug (cross-file)

`PanelEvent` uses `u64` entity IDs, `EntityManager` uses `EntityId = u64`, but `World`/ECS uses `Entity = u32`. Conversions `entity_id as u32` silently truncate. Safe today (IDs derived from u32 World entities), but `EntityManager.next_id` is u64 with no guard preventing IDs >= 2^32.

### M-42: Gizmo arrow geometry is placeholder line segments

**File**: `gizmo/rendering.rs:63-77`
**Severity**: MEDIUM
**Category**: stub

Arrow geometry generates only 4 vertices (2 line segments) instead of proper cone+cylinder mesh. Comment says "simplified as line for now". The gizmo handles are functional but visually primitive.

---

## Low Severity / Informational

### L-1: Empty match arms that may silently swallow events

**Files**: Multiple — 42 instances across the codebase
**Severity**: LOW
**Category**: Various

Most empty match arms (`_ => {}`) are intentional no-ops for irrelevant variants (e.g., `GizmoMode::Inactive`, `ConsoleAction::None`, `EnvironmentPreset::Custom`). A few warrant review:
- `main.rs:4111,4397,4512` — event dispatch arms
- `plugin.rs:1011` — plugin event handling
- `behavior_graph/node_graph_widget.rs:638` — node action handling

### L-2: `viewport/mod.rs` — 4 `#[allow(unused_imports)]`

**File**: `viewport/mod.rs:57,59,64,66`
**Severity**: LOW

### L-3: Duplicate dialogue/quest types in `main.rs`

**File**: `main.rs:2-31`
**Severity**: LOW
**Category**: dead_code

`DialogueDoc`, `DialogueNode`, `DialogueResponse`, `QuestDoc`, `QuestStep` are defined locally in main.rs while the editor also uses `astraweave_dialogue::DialogueNode` and `astraweave_quests::QuestStep` (lines 525-539). The local types appear to be legacy from before the proper crate types were available.

### L-4: `tests_document.rs:13` — bare `matches!()` without `assert!()`

**File**: `behavior_graph/tests_document.rs:13`
**Severity**: LOW
**Category**: logic_bug (test bug)

```rust
matches!(root.kind, BehaviorGraphNodeKind::Action { .. });
```

This is a bare expression, not wrapped in `assert!()`. The match result is discarded — the test passes regardless of the node kind. Should be `assert!(matches!(...))`.

### L-5: Splash video decode path is dead code

**File**: `splash.rs:49-66, 350-458`
**Severity**: LOW
**Category**: dead_code

`video_available` is hardcoded to `false` in `SplashScreen::new()`. The entire H.264 decode infrastructure (~108 lines) is unreachable. The code itself is structurally correct but should either be feature-gated or removed.

### L-6: Audio bridge has 9 unimplemented features and hardcoded stats

**File**: `audio_bridge.rs:156-179, 226-229`
**Severity**: LOW
**Category**: stub

9 audio actions are intentional stubs with comments like "noted for future DSP integration": spatial presets, HRTF, Doppler, distance model, reverb, crossfade, shuffle, loop. The `stats()` method returns hardcoded values (`active_voices: 1`, `active_music_channels: 2`) instead of querying the engine.

### L-7: No panic handlers on any of 7 spawned threads

**Files**: `file_watcher.rs:242`, `main.rs:5998`, `build_manager.rs:408,544,553`, `terrain_panel.rs:2031`, `scatter_renderer.rs:692`
**Severity**: LOW
**Category**: threading

None of the 7 `thread::spawn` calls have panic handlers. If a background thread panics, the panic is silently swallowed (channel disconnects, receiver sees `Disconnected`). Adding `catch_unwind` or named threads with panic hooks would improve debuggability. Only the scatter mesh loader uses `thread::Builder::new().name(...)` (good practice).

### L-8: Win32 memory query duplicated in two files

**Files**: `main.rs:836`, `viewport/widget.rs:2972`
**Severity**: LOW
**Category**: copy_paste

Identical Win32 `GetProcessMemoryInfo`/`GlobalMemoryStatusEx` FFI code in two locations. Should be refactored into a shared helper.

### L-9: `next_entity_id` field initialized but never incremented

**File**: `main.rs:424`
**Severity**: LOW
**Category**: dead_code

`next_entity_id: u64` is initialized to 100 but never incremented or used for entity creation. Entity IDs come from `EntityManager` instead.

### L-10: Redundant `check_transform_changes()` call

**File**: `main.rs:3908,3918`
**Severity**: LOW
**Category**: copy_paste

`self.dock_tab_viewer.check_transform_changes()` is called twice in `show_docking_layout()`, 10 lines apart. The second call is a copy-paste artifact.

### L-11: Legacy central panel has GREEN debug fill

**File**: `main.rs:4983`
**Severity**: LOW
**Category**: placeholder

`fill(egui::Color32::from_rgb(0, 255, 0))` with comment `// GREEN for legacy`. Debug visual that should be removed if the legacy path is still accessible.

### L-12: 2 `// FIXME` comments about Windows emoji encoding

**File**: `panels/navigation_panel.rs:2038,2064`
**Severity**: LOW (test-only)

Tests skipped due to Windows emoji encoding issues. Not a production concern but indicates platform-specific test gaps.

### L-13: `entity_renderer.rs:1804` — `let _ = transmission; // reserved for future use`

**File**: `viewport/entity_renderer.rs:1804`
**Severity**: INFO

glTF transmission property is parsed but explicitly discarded. This is a known limitation rather than a bug — transmission rendering requires order-independent transparency which the current pipeline doesn't support.

### L-14: `entity_renderer.rs:2299` — `let _ = (node, node_to_joint);`

**File**: `viewport/entity_renderer.rs:2299`
**Severity**: INFO

Skin/joint data parsed but unused. Skeletal animation rendering is not yet implemented in the editor viewport.

---

## Safety Assessment (Second Pass)

| Category | Status | Details |
|----------|--------|---------|
| GPU uniform alignment | **CLEAN** | All 12 uniform buffer struct pairs and 9 vertex/instance layouts verified byte-by-byte — zero misalignment bugs |
| Unsafe code | **CLEAN** | 2 `unsafe` blocks (Win32 FFI), both with SAFETY comments, both correct |
| Manual Send/Sync | **CLEAN** | No manual `unsafe impl Send/Sync` found |
| Deadlocks | **CLEAN** | No nested lock acquisition patterns found |
| Undefined behavior | **CLEAN** | No UB risks identified in unsafe blocks |
| Thread panics | **LOW risk** | 7 spawned threads, none with panic handlers; channels handle disconnect gracefully |
| Division by zero | **3 instances** | profiler flame graph, animation total_frames, camera aspect ratio |
| Index out-of-bounds | **CLEAN** | All `len() - 1` patterns are guarded by prior checks or loop bounds |
| Integer overflow | **CLEAN** | No dangerous truncation casts in production code |

---

## AI Code Pathology Summary

| Category | Count | Details |
|----------|-------|---------|
| `todo!()` / `unimplemented!()` | **0** | None found — excellent |
| `// TODO` / `// FIXME` | **4** | 2 TODOs (renderer.rs:647, entity_renderer.rs:1905), 2 FIXMEs (navigation_panel.rs:2038,2064) |
| `#![allow(dead_code)]` (crate-wide) | **15** | main.rs, lib.rs, terrain_integration.rs, 12 viewport files |
| `#[allow(dead_code)]` (per-item) | **18** | command.rs (8), main.rs (4), entity_manager.rs (2), foliage_panel.rs (1), ui_editor_panel.rs (1), picking.rs (8) |
| `#[allow(unused_imports)]` | **34** | panels/mod.rs (30), behavior_graph/mod.rs (2), viewport/mod.rs (4) — partially unused panel imports |
| `.unwrap()` in production code | **1** | charts_panel.rs:223 (NaN panic risk) |
| `.unwrap()` in test code | **291** | Correctly confined to `#[cfg(test)]` — verified |
| `filter_map(.ok())` | **6** | Silent error swallowing in directory iteration |
| `eprintln!` in prod code | **8** | Should use `tracing` (13 more in test code) |
| `let _ =` discarding Results | **~50** | Most are low-risk (channel sends, dir creation). ~5 warrant attention |
| Buttons with no behavior | **36** | 8 gradient/easing presets (tab_viewer.rs:7993-8030), 4 BehaviorGraph toolbar (8735-8738), 22 stub buttons in tab_viewer.rs, foliage category filter (foliage_panel.rs:966), asset store tags (ready_asset_store_panel.rs:913) |
| Dead/legacy types | **6** | main.rs DialogueDoc/QuestDoc/Toast types superseded by crate types |
| Stub implementations | **14** | GPU stats (main.rs:873), non-Windows memory (main.rs:862), voxel undo (main.rs:7700), gizmo arrow geometry (gizmo/rendering.rs:63), 9 audio bridge actions (audio_bridge.rs:156-179), audio stats hardcoded (audio_bridge.rs:226) |
| Duplicate hotkey handlers | **1** | Ctrl+D at main.rs:8832 AND main.rs:9117 — creates ghost entities |
| Placeholder implementations | **1** | on_open() hardcoded path (main.rs:7883) |
| Commented-out code blocks | **0** | None found |
| Copy-paste artifacts | **2** | Duplicate Ctrl+D handler, redundant check_transform_changes() call |

---

## Known Bug Status Verification

| # | Bug Description | Status | Evidence |
|---|----------------|--------|----------|
| 1 | Normal maps sampled as sRGB | **FIXED** | terrain_renderer.rs:1045 uses `Rgba8Unorm`, entity_renderer.rs:1895 passes `srgb: false` |
| 2 | PBR view direction `normalize(-world_pos)` | **FIXED** | entity.wgsl:284 uses `normalize(uniforms.camera_pos - world_pos)` |
| 3 | Splat material ID mismatch | **FIXED** | terrain.wgsl:425-429 uses `@interpolate(flat)` material_ids with rounding |
| 4 | Missing tangent fallback [1,0,0,1] | **MITIGATED** | Terrain uses UDN blending (terrain.wgsl:152) avoiding tangents entirely. Entity renderer uses cotangent frame (entity.wgsl:362-386) |
| 5 | Missing normal fallback [0,1,0] | **FIXED** | terrain_renderer.rs:537-542 has flat normal fallback `[128,128,255,255]` |
| 6 | VXGI undefined function | **FIXED/REMOVED** | No VXGI references found anywhere in editor |
| 7 | Hot-reload dead code path | **FIXED** | file_watcher.rs integrated with material_inspector.rs, `process_hot_reload()` called at line 883 |
| 8 | Vegetation placeholder cubes | **FIXED** | scatter_renderer.rs loads GLB meshes via `load_mesh_on_worker()` |
| 9 | Terrain no back-face culling | **ACKNOWLEDGED** | terrain_renderer.rs:795 — intentional workaround for mixed winding order |
| 10 | Stats overlay 0K triangles | **FIXED** | renderer.rs:1214-1231 has `terrain_triangles()` and `scatter_triangles()` |
| 11 | CPU-side mipmap generation | **STILL PRESENT** | terrain_renderer.rs:969, entity_renderer.rs:2085 |
| 12 | Scatter rebuilds on camera move | **FIXED** | scatter_renderer.rs:941-946 has delta threshold + yaw/pitch checks |
| 13 | HDRI `filter_map(.ok())` | **STILL PRESENT** | Multiple instances across 6 files |
| 14 | BC7 encoding truncation | **Not in editor scope** | N/A |
| 15 | Fluid depth pass clip-space | **FIXED** | water.wgsl:146-148 correct linearization, uses `textureLoad` (line 195) |
| 16 | Fluid depth linear filtering | **FIXED** | water.wgsl:195 uses `textureLoad` (exact texel fetch, no filtering) |
| 17 | Caustics Voronoi performance | **Not in editor scope** | UI config only at terrain_panel.rs:304 |

**Score**: 11 of 13 applicable bugs fixed. 2 remaining (CPU mipmaps, silent error swallowing). 1 new bug found (C-1).

---

## Pipeline Divergence Map

| Feature | Editor Implementation | Core Render Crate | Status | Recommendation |
|---------|----------------------|-------------------|--------|----------------|
| Vertex format | 48-byte (pos+norm+color+uv) | 96-byte (full PBR attributes) | **Diverged** | Intentional for editor — lighter weight meshes acceptable |
| PBR BRDF | Disney BRDF with clearcoat, sheen, IBL (entity.wgsl) | Cook-Torrance (render crate) | **Superset** | Editor implementation is more advanced |
| Normal mapping | Cotangent frame (entity.wgsl:362-386) + UDN triplanar (terrain.wgsl:152) | TBN from vertex tangents | **Different approach** | Both correct, terrain UDN avoids tangent requirement |
| Shadow mapping | 5-tap PCF, single cascade (entity.wgsl:82-117) | Cascaded shadow maps | **Simpler** | Adequate for editor preview |
| IBL | SH L2 irradiance + BRDF LUT (entity.wgsl:119-156) | Not implemented | **Editor-only** | Advanced feature unique to editor |
| Tone mapping | Dual: inline ACES fallback + HDR post-chain (entity.wgsl:586-596) | ACES only | **More flexible** | Good architecture |
| Mipmap generation | CPU box filter | GPU mipmaps | **Diverged (worse)** | Unify to GPU mipmaps |
| Texture format handling | sRGB/linear correctly separated | Uses TextureUsage metadata | **Different mechanism** | Both work, editor approach is explicit |
| Color space pipeline | HDR + post-process chain with exposure | LDR pipeline | **Editor more advanced** | Editor is ahead |

**Assessment**: The editor rendering pipeline has **matured significantly** beyond a naive reimplementation. In several areas (IBL, Disney BRDF, HDR pipeline), it exceeds what the core render crate provides. The divergence is now architectural rather than accidental — the editor provides a preview-quality viewport, not a duplicate of the runtime renderer.

---

## `tab_viewer.rs` Decomposition Proposal

**Current size**: 9,548 lines (5.6% of crate, 10.4% of mutations)
**God-struct**: `EditorTabViewer` has 90+ fields spanning all panel types

### Current File Structure

| Section | Lines (approx) | Description |
|---------|----------------|-------------|
| `EditorDrawContext` + `SimpleTabViewer` | 1-462 | Draw context wrapper, basic docking |
| `PanelEvent` enum + impls | 464-872 | Event system with 50+ variants |
| Data models | 874-1248 | 13 data-only types |
| `EditorTabViewer` struct + state | 1251-1488 | God-struct with 90+ fields |
| `EditorTabViewer` methods | 1490-3598 | Setters, getters, computation |
| Component inspectors | 2440-3410 | 7 inspector functions (~970 lines) |
| `TabViewer` impl (panel UI) | 3600-9388 | Giant `match tab` with ~5,800 lines |
| Tests | 9410-9547 | 8 tests |

### Proposed Module Split

| Module | Lines | Content |
|--------|-------|---------|
| `tab_viewer/mod.rs` | ~1,500 | `EditorTabViewer` struct, setters/getters, `TabViewer` dispatch skeleton |
| `tab_viewer/panel_event.rs` | ~400 | `PanelEvent` enum, Display, category helpers |
| `tab_viewer/editor_models.rs` | ~375 | 13 data-only types (EntityInfo, AnimationState, etc.) |
| `tab_viewer/component_inspectors.rs` | ~970 | 7 inspector functions (light, collider, rigidbody, audio, camera, script, particle) |
| `tab_viewer/sky_colors.rs` | ~170 | `compute_sky_colors`, `lighting_params`, `fog_weather_params` (pure computation) |
| `tab_viewer/panel_ui/viewport.rs` | ~400 | Viewport panel UI |
| `tab_viewer/panel_ui/hierarchy.rs` | ~400 | Hierarchy panel UI |
| `tab_viewer/panel_ui/inspector.rs` | ~700 | Inspector + EntityPanel UI (deduplicate transform editing) |
| `tab_viewer/panel_ui/profiler.rs` | ~600 | Profiler + Performance panels (deduplicate grade computation) |
| `tab_viewer/panel_ui/world.rs` | ~500 | World panel (skybox, weather, lighting, physics) |
| `tab_viewer/panel_ui/build_manager.rs` | ~260 | BuildManager panel UI |
| `tab_viewer/panel_ui/animation.rs` | ~250 | Animation panel UI |
| `tab_viewer/panel_ui/graph.rs` | ~600 | Graph + BehaviorGraph + Charts panels |
| `tab_viewer/panel_ui/material_editor.rs` | ~380 | MaterialEditor panel UI |
| `tab_viewer/panel_ui/theme_manager.rs` | ~170 | ThemeManager panel UI |

**Key deduplication opportunities**:
- Transform editing UI: ~500 lines duplicated between Inspector and Transform panel branches
- Performance grade calculation: computed 3 times with slight variations
- `TerrainAction` polling: duplicated between `take_events()` and Terrain UI branch

---

## Test Infrastructure Recommendations

### 1. Extract `EditorApp` from `main.rs` (Estimated: ~200 mutation kills)

The 624 zero-coverage mutants in main.rs cannot be tested while `EditorApp` lives in a binary crate. Moving it to `lib.rs` (as `editor_app.rs`) immediately enables unit testing of:
- Default initialization logic
- File I/O operations (save/load/autosave)
- Environment parameter calculation
- Panel event dispatch
- Blend import state machine

### 2. Remove `#![allow(dead_code)]` and fix warnings (Estimated: ~50 mutation kills)

Dead code can't have mutations killed. Removing the blanket annotations will reveal functions that can be deleted (reducing mutation count) or that need test coverage.

### 3. Add integration tests for viewport rendering pipeline (Estimated: ~100 mutation kills)

The `headless.rs` module provides test harness infrastructure. Add tests for:
- Terrain texture format validation (sRGB vs linear)
- Shader uniform buffer packing (verify GPU struct alignment matches Rust structs)
- Camera frustum culling correctness
- Material parameter wiring (from panel UI values to shader uniforms)

### 4. Test the non-functional buttons (Estimated: ~8 mutation kills, but validates UX)

The 8 `let _ = ui.small_button(...)` calls in tab_viewer.rs either need behavior implemented or should be removed. Each is a user-facing UI element that does nothing.

### 5. Property-based tests for terrain shader logic

The terrain shader's biome blending, material slot blending, and LOD crossfade functions have complex branching that is ripe for property-based testing. Consider extracting the weight-selection logic into Rust functions that mirror the WGSL and testing with `proptest`.

---

## Recommended Fix Ordering

Sequenced by dependency and impact:

### Tier 0: Undo system and state integrity (most impactful)

- **C-5**: Wire event handlers to undo commands — transform edits, entity creation, property changes all bypass undo
- **C-4**: Fix delete undo to restore EntityManager — undo currently corrupts editor state
- **H-13**: Fix Spawn/Duplicate undo to actually destroy entities instead of moving offscreen
- **H-12**: Implement the 18 status-only PanelEvent handlers (viewport controls, build, console clear)
- **M-36/37/38**: Sync RemoveComponent, EntityRenamed, ComponentDataChanged to ECS World

### Tier 1: Immediate user-facing bugs (1-line to small fixes)

- **C-3**: Delete duplicate Ctrl+D handler at main.rs:9117-9139 — ghost entity creation
- **C-1**: Fix `dominant_biome_layer()` in terrain.wgsl — Grassland renders as Desert at distance
- **H-14**: Fix `charts_panel.rs:223` `.unwrap()` — NaN crash risk
- **H-8**: Fix `ray_from_screen` NDC Z from -1.0 to 0.0 — picking inaccuracy
- **M-8**: Fix Ctrl+S toast to use `toast_manager` — users see no save confirmation
- **H-7**: Fix entity normal transform to use inverse-transpose — wrong lighting on scaled entities

### Tier 2: Code hygiene and observability

- **M-1**: Replace all `eprintln!` with `tracing` macros
- **H-0**: Add error logging to dangerous `let _ =` patterns in main.rs
- **M-2**: Add error logging to `filter_map(.ok())` calls

### Tier 3: Architectural extraction (enables testing)

- **C-2**: Extract `EditorApp` from main.rs into testable `editor_app.rs`
- **H-2**: Remove `#![allow(dead_code)]` annotations — reveals dead code
- **M-23**: Delete dead `rain_renderer.rs` (310 lines, superseded by weather_particle_renderer)
- **M-5 + L-3 + L-5**: Delete legacy Toast/DialogueDoc/QuestDoc types, dead splash video code

### Tier 4: UI completeness

- **M-4 + M-13 + H-9**: Wire or remove 36 non-functional buttons
- **M-11**: Separate `LevelDoc.title` from hierarchy search field
- **M-12**: Wire `on_open()` to file dialog instead of hardcoded path
- **M-14**: Add confirmation to "Clear All" button
- **M-15**: Fix budget progress bar math (100% shows as 50%)

### Tier 5: Rendering pipeline improvements

- **H-5**: Refactor scene rendering to HDR (`Rgba16Float`) — enables proper bloom/exposure
- **H-6**: Upload HDRI as float texture, not 8-bit sRGB
- **H-1**: Implement GPU mipmap generation — performance + correctness
- **H-4**: Wire normal G-buffer for GTAO
- **M-3**: Fix terrain winding order → enable back-face culling — 2x terrain fragment perf
- **M-24**: Load glTF tangent attributes when available
- **M-25**: Composite separate occlusion texture into ORM

---

## Appendix: File Size Distribution

| File | Lines | % of Crate |
|------|-------|------------|
| tab_viewer.rs | 9,548 | 5.6% |
| main.rs | 9,378 | 5.5% |
| dialogue_editor_panel.rs | 3,823 | 2.3% |
| animation_panel.rs | 3,662 | 2.2% |
| project_settings_panel.rs | 3,444 | 2.0% |
| particle_system_panel.rs | 3,269 | 1.9% |
| entity_renderer.rs | 3,264 | 1.9% |
| lighting_panel.rs | 3,161 | 1.9% |
| widget.rs | 2,999 | 1.8% |
| asset_browser.rs | 2,957 | 1.7% |
| **Total** | **169,832** | **100%** |

**Test count**: 3,970 `#[test]` functions across 108 files (including 286 in mutation_tests.rs)
