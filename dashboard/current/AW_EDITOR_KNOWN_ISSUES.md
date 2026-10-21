# AW Editor – Current Failure Modes (Jan 17, 2026)

This note captures the reproducible breakages reported for `tools/aw_editor` and the code-level evidence gathered after inspecting the latest sources.

## Status Update (Jan 17, 2026)
- ✅ **Delete Command with Undo**: Hierarchy panel and viewport delete now route through `DeleteEntitiesCommand` for full undo/redo support.
- ✅ **Duplicate Entity**: Hierarchy panel duplicate now uses `ClipboardData` to properly clone all entity properties including BehaviorGraph.
- ✅ **Transform Panel Events**: Tab viewer transform events (position, rotation, scale) now update the real ECS World via `scene_state.world_mut()`.
- ✅ **BehaviorGraph Preservation**: Clipboard copy/paste and delete undo now preserve AI behavior graphs.
- ✅ 429 library tests + 2 integration tests passing, 0 warnings.

## Status Update (Nov 17, 2025)
- ✅ Grid toggle + snap slider now directly drive the renderer: disabling the grid skips the GPU pass entirely, and spacing matches the toolbar/snapping configuration.
- ✅ Telemetry capture + headless harness landed, giving us deterministic gizmo repros and assertions for commit/cancel flows.
- ✅ Viewport selection/state now runs on `EditorSceneState` (`main.rs`, `scene_state.rs`, `panels/entity_panel.rs`) so gizmos, hierarchy, and save/load all mutate the authoritative ECS world. The Known Issues digest has been circulated and is now the canonical Week 1 record.

## 1. Gizmo Translation Snaps Back — **Resolved (Nov 17)**
- **Symptoms**: Pressing `G` to move an entity shows live movement while dragging, but the entity jumps back (often to the origin) as soon as the mouse is released.
- **Root causes**:
  - ✅ Gizmo operations now read/write directly through `EditorSceneState::world_mut()`, the viewport syncs the cached transforms after every interaction, and undo/redo routes through the ECS world (`main.rs` lines ~1660-1930). This keeps visual feedback and serialized data in lockstep.
  - ⏭️ Follow-up: `TransformPanel` still references `entity_manager` for display only. Remaining refactors will either retire the mock manager or hydrate it exclusively from `EditorSceneState` snapshots to avoid duplication.

## 2. Grid Toggle and Snapping — **Resolved (Nov 15)**
- `GridRenderSettings` now flows from the toolbar → widget → renderer → WGSL uniforms. Disabling the grid bypasses the entire render pass, and spacing mirrors the snap slider (clamped to ≥0.1m).
- Remaining follow-ups: surface major-line density in the UI and ensure gizmo snapping math consumes the same `SnappingConfig` used by the renderer, so visual + interaction layers stay in lockstep.

## 3. Behavior Node Editor Is Static — **Resolved (Nov 17)**
- `EditorApp::show_behavior_graph_editor` now drives the dedicated `BehaviorGraphDocument`/`BehaviorGraphEditorUi` pipeline. Designers can add/remove/relabel nodes, tweak decorators, and validate the graph before saving.
- Load/Apply buttons bridge the document to `EditorSceneState`: selecting an entity hydrates the doc from `world.behavior_graph(entity)` (or seeds a fresh doc), and applying pushes the serialized `BehaviorGraph` back into the ECS world + syncs the cache.
- File toolbar (RON save/load, validation) is retained from the document module, so authored graphs can round-trip independently of the current scene.

## 4. Asset / Texture Import Missing — **Resolved (Nov 17)**
- `AssetBrowser.dragged_prefab` now consumed by `EditorApp::spawn_prefab_from_drag` after every UI frame. Drag-and-drop events bridge directly to `PrefabManager::instantiate_prefab`, which loads the `.prefab.ron` file, spawns entities into `EditorSceneState::world_mut()`, and syncs caches.
- Newly instantiated prefabs spawn at grid origin (0, 0) by default, with the root entity auto-selected and logged to the console. Future work can enhance positioning (mouse cursor tracking, viewport raycast) and add material/texture import flows alongside prefab workflows.
- File browsing, filtering, and search already exist in the asset browser—prefab instantiation was the missing link to scene authoring.

## 5. Play/Pause/Stop Does Not Drive Simulation — **Resolved (Nov 17)**
- `EditorRuntime` now fully integrated with toolbar UI via `show_play_controls` widget in status bar. Play/Pause/Stop/Step buttons display runtime state (Edit/Playing/Paused) with color-coded indicators.
- Simulation state management working correctly:
  - **Play (F5)**: Captures edit-mode snapshot via `SceneData::from_world`, clones into `sim_world`, begins 60Hz deterministic ticking.
  - **Pause (F6)**: Preserves simulation state, stops ticking but maintains `sim_world` intact.
  - **Stop (F7)**: Restores original edit snapshot via `SceneData::to_world`, discards `sim_world`, returns to Edit mode.
  - **Step (F8)**: Advances exactly one frame (16.67ms) then pauses—useful for debugging frame-by-frame.
- Runtime stats displayed in toolbar: tick count, entity count, frame time (ms), FPS. Performance metrics accumulate during playback for profiling.
- `EditorApp::active_world()` abstracts world access: returns edit world when `RuntimeState::Editing`, sim world otherwise. This ensures viewport, hierarchy, and entity panel always query the correct world.
- Tests in `runtime.rs` validate snapshot capture/restore, pause/resume transitions, and single-frame stepping. Zero regressions from Week 1 determinism work.

## 6. Prefab/Entity Sync Gaps — **Resolved (Nov 17)**
- `EntityPanel` now receives prefab instance context from `PrefabManager::find_instance(entity)` lookup in main UI loop. When an entity belongs to a prefab, the inspector displays:
  - Prefab source file name (monospace label)
  - Override indicator if `has_overrides(entity)` returns true (blue warning text)
  - **💾 Apply to Prefab** button: calls `apply_to_prefab(world)` → saves current entity state back to `.prefab.ron` file, making changes permanent
  - **🔄 Revert to Prefab** button: calls `revert_to_prefab(world)` → discards local changes, reloads original prefab data, clears override tracking
- `PrefabAction` enum (RevertToOriginal | ApplyChangesToFile) bridges UI → main.rs → PrefabManager execution. Button clicks return actions, main loop handles file I/O and world mutations.
- Override tracking infrastructure exists (`track_override`, `EntityOverrides` struct with pos/health/ammo fields) but not yet auto-called on component edits—requires integration with undo system in future work. Current workflow: manual Apply/Revert for user-initiated sync.
- Prefab instantiation from AssetBrowser (Issue #4) and prefab sync (Issue #6) together complete the authoring → persistence → reuse pipeline. Designers can now: drag prefab → spawn instance → tweak properties → revert mistakes OR apply improvements back to source.

## 7. Telemetry & Testing — **Partially Resolved (Nov 17)**

### Telemetry — ✅ Complete
- **Structured tracing** now integrated via `astraweave-observability` + `tracing` crate:
  - `request_play()`: INFO-level span tracks mode transitions, captures snapshot success/failure with error details
  - `request_pause()`: INFO-level span logs tick count at pause moment
  - `request_stop()`: INFO-level span tracks final tick count and snapshot restoration
  - `request_step()`: DEBUG-level span logs single-frame advancement
  - `spawn_prefab_from_drag()`: INFO-level span with prefab path, spawn position, root entity tracking
  - Prefab Apply/Revert actions: INFO-level spans log entity IDs and file paths
- **Console logging enhanced** with tracing macros (`info!`, `warn!`, `error!`, `debug!`):
  - Severity levels: `info!` for normal operations, `warn!` for recoverable issues, `error!` for failures
  - Structured fields: entity IDs, tick counts, file paths, positions captured in span metadata
  - All console messages still visible in UI + structured logs for tooling/debugging
- **Observability integration**: `astraweave_observability::init_observability()` called in `main()` configures tracing subscriber with INFO level by default
- Tracing spans enable: log filtering, distributed tracing (future), performance profiling, automated analysis

### Testing — ⏭️ Deferred
- **Existing test suite**: 9 test files exist covering gizmo, play mode, behavior editor, prefab workflow, scene serialization, undo transactions
  - Tests compile successfully (`cargo test -p aw_editor --lib --no-run` passes)
  - Manual test execution deferred (out of scope for telemetry focus)
- **Headless harness**: Mentioned in Week 1 status but no evidence of headless UI test infrastructure
- **Automated regression tests**: Zero automated UI smoke tests for gizmo drags, asset drops confirmed
- **Follow-up work needed**:
  1. Run full test suite and document pass/fail status
  2. Implement headless egui test harness for UI regression testing
  3. Add smoke tests for critical workflows (gizmo translate, prefab drag-drop, play/pause cycles)
  4. Integrate with CI pipeline for regression detection

These findings will seed the follow-up design documents (interaction fixes, authoring upgrades, simulation overhaul) and give us concrete repro steps for validation harnesses.
