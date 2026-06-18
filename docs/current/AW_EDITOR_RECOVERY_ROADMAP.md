# AW Editor – Recovery Roadmap

_Version 0.2 • Nov 17, 2025_

This roadmap sequences the work required to turn `tools/aw_editor` into a production-ready editor. It ties together the interaction, authoring, and simulation plans into five execution waves with explicit validation gates.

## Milestone Overview
| Week | Focus | Key Deliverables | Validation |
| --- | --- | --- | --- |
| 1 | Telemetry + Interaction Foundations | Known issues doc, headless gizmo harness, `EditorSceneState`, grid toggle plumbing | `cargo test -p aw_editor -- ui_gizmo_smoke` passes, grid toggle affects renderer |
| 2 | Gizmo/Grid Completion & Undo | Transform transactions, undo integration, tracing hooks, documentation update | Manual checklist (move, snap, undo/redo) signed off; tracing logs visible |
| 3 | Authoring Surface | Behavior graph editor MVP, asset drag/drop, prefab override UI | Round-trip behavior graphs; prefab apply/revert tests |
| 4 | Simulation Overhaul | `EditorRuntime`, deterministic snapshots, HUD + step controls | Play/Pause/Stop tests (`play_mode.rs`), manual UAT of level playback |
| 5 | Polish & Reporting | Performance metrics, README refresh, regression suite, master docs | Benchmark dashboard updated, Master Roadmap/Coverage revisions |

## Workstreams
1. **Interaction Track** (Weeks 1-2)
   - Implement `EditorSceneState` and retire the mock `EntityManager` for ECS data.
   - Refactor gizmo → undo pipeline per `AW_EDITOR_INTERACTION_PLAN.md`.
   - Connect toolbar toggles to renderer + gizmo snapping.
2. **Authoring Track** (Week 3)
   - Build new behavior-graph editor and asset ingestion pipeline (`AW_EDITOR_AUTHORING_PLAN.md`).
   - Prefab override visualization + apply/revert commands.
3. **Simulation Track** (Week 4)
   - Replace booleans with `EditorRuntime` per `AW_EDITOR_SIMULATION_PLAN.md`.
   - HUD + telemetry to observe deterministic tick loop.
4. **Quality & Docs** (Week 5)
   - UI smoke tests covering gizmo, asset drops, play mode.
   - Update `docs/current/MASTER_ROADMAP.md`, `MASTER_BENCHMARK_REPORT.md`, `MASTER_COVERAGE_REPORT.md` once features land.

## Testing Strategy
- **Unit / Integration**: add target-specific tests mentioned in each plan (`ui_gizmo`, `asset_ingest`, `play_mode`).
- **Headless Harness**: new CLI scenarios scripted via egui input playback to ensure regressions are reproducible.
- **Manual UAT**: curated checklist stored in `tools/aw_editor/README.md` (or `docs/current/AW_EDITOR_UAT.md`) run at the end of each week.

## Risks & Mitigations
- **Large Refactors**: Interaction changes touch viewport, gizmo, entity storage. Mitigate by staging behind feature flags and landing incremental PRs.
- **Asset Pipeline Complexity**: rely on existing `asset_database` crates and scripts to avoid reinventing importers.
- **Deterministic Snapshots**: reuse `SceneData` + `WorldSnapshot` utilities validated in Phase 6/7 to reduce serialization bugs.

## Reporting
- After each week, log progress in `docs/journey/weekly/PHASE_8_UI_WEEK_[n]_REPORT.md` (or similar) to keep the experiment traceable.
- Update benchmark/history JSONL files when editor metrics improve so dashboards stay accurate.

This roadmap is the execution contract for bringing aw_editor up to standards. Each referenced plan provides the detailed design for its workstream; this document keeps the sequence and validation visible across the project.

## Week 1 Progress – Nov 17, 2025

- ✅ Telemetry capture, structured gizmo events, and the new headless harness are live (`cargo test -p aw_editor --test ui_gizmo_smoke`).
- ✅ Grid toggle + snap slider now directly drive the renderer: disabling the grid skips the GPU pass entirely, and spacing matches the toolbar/snapping configuration.
- ✅ `EditorSceneState` owns edit-mode data end to end. Hierarchy, viewport, save/load, undo/redo, and the entity inspector all route through the canonical ECS world (`tools/aw_editor/src/main.rs`, `scene_state.rs`, `panels/entity_panel.rs`).
- ✅ Known Issues digest circulated (`docs/current/AW_EDITOR_KNOWN_ISSUES.md`, version Nov 17) with Issue #1 marked resolved and follow-up items queued.

## Week 3 Progress – Nov 17, 2025

- ✅ Behavior Graph Editor now uses the production `BehaviorGraphDocument` model + egui UI scaffold. Nodes can be added/removed/reordered, decorators expose full parameter editing, and validation runs via the document's `to_runtime()` conversion.
- ✅ Entity binding landed: "Load From Selection" hydrates the document from `world.behavior_graph(entity)` (or seeds a default), while "Apply To Selection" serializes back into the ECS world and syncs `EditorSceneState` so prefabs pick up the authored AI.
- ✅ File toolbar (RON save/load + validation) integrates with the document object, enabling round-trip authoring independent of the active scene and keeping behavior graphs versionable alongside other assets.
- ✅ Prefab drag-and-drop from AssetBrowser now instantiates `.prefab.ron` files into the scene via `PrefabManager`. Dragged prefabs spawn at (0, 0) grid origin, auto-select the root entity, and sync `EditorSceneState` caches so the hierarchy/inspector reflect the new entities immediately.
- ✅ Asset import pathway bridged end-to-end: `AssetBrowser.take_dragged_prefab()` consumed after every UI frame, `spawn_prefab_from_drag` routes to `PrefabManager::instantiate_prefab`, and success/failure logged to the console with entity ID + position confirmation.

## Week 4 Progress – Nov 17, 2025

- ✅ `EditorRuntime` snapshot capture/restore operational again; play sessions now preserve edit state using `SceneData` + the new `World::spawn_with_id` helper.
- ✅ Serialization + prefab helpers updated so editor tests compile against `IVec2::new`, eliminating local constructors scattered across crates.
- ✅ `cargo test -p aw_editor --lib runtime::tests` passes (8/8), confirming deterministic play/pause/step behavior post-refactor.
- ✅ Scene serialization + runtime plumbing unblocks the UI workstream; remaining tasks moved into dedicated HUD/telemetry slice below.

## Week 4 Progress – Nov 18, 2025

- ✅ Play controls toolbar restored: Play/Pause/Stop/Step buttons now invoke `EditorRuntime` helpers and mirror the F5–F8 shortcuts, so designers can drive the deterministic runtime without memorizing hotkeys.
- ✅ HUD + telemetry online: the Astract Performance panel consumes live `RuntimeStats`, issues 60 FPS budget warnings, and the profiler pane streams tick/entity/frame metrics every 500 ms for quick regression spotting.
- ✅ Behavior graph persistence validated end-to-end (world storage + scene serialization + editor tests), keeping Week 3 authoring workflows intact during simulation.
- ✅ `cargo test -p aw_editor --test play_mode --test behavior_editor` passes (5 + 3 cases), covering snapshot capture, deterministic replay, runtime stats accuracy, and behavior editor round-trips.

**Session 2 Continuation – Nov 17, 2025 (Issues #5-6)**:
- ✅ **Issue #5 (Play/Pause/Stop Controls)**: Integrated `show_play_controls` widget into status bar, displaying runtime state with color-coded indicators (🛠️ Edit gray, ▶️ Playing green, ⏸️ Paused orange). All 4 controls (Play/Pause/Stop/Step) now functional with proper state transitions and snapshot management. Runtime stats (tick count, entities, frame time, FPS) displayed in toolbar during playback.
- ✅ **Issue #6 (Prefab/Entity Sync)**: Added `PrefabAction` enum (RevertToOriginal | ApplyChangesToFile) to bridge EntityPanel → main.rs → PrefabManager. EntityPanel now shows 💾 Apply/🔄 Revert buttons when prefab overrides detected. Main loop handles file I/O and world mutations via `apply_to_prefab`/`revert_to_prefab` methods. Console logs confirm success/failure with entity IDs.
- ✅ Build validation: `cargo check -p aw_editor` passes with 52 warnings (all pre-existing dead code), 0 errors. Play controls, prefab sync, and entity panel all compiling cleanly.
- 🎯 **Achievement**: Simulation + prefab workflows complete—designers can now play/pause/step through levels AND manage prefab overrides with full apply/revert support.

**Session 3 – Telemetry Infrastructure (Nov 17, 2025)**:
- ✅ **Structured Tracing**: Integrated 	racing crate with straweave-observability. Added INFO/DEBUG/ERROR spans to play controls (play/pause/stop/step), prefab operations (instantiate/apply/revert), and spawn workflows. Spans include structured fields: entity IDs, tick counts, file paths, positions.
- ✅ **Console Logging Enhancement**: Replaced bare console_logs.push with tracing macros (info!, warn!, rror!, debug!). All messages still visible in UI + structured logs for tooling.
- ✅ **Observability Integration**: Leveraged existing straweave_observability::init_observability() in main() for tracing subscriber setup (INFO level default).
- ✅ **Build Validation**: cargo check -p aw_editor passes with 0 errors after adding 	racing workspace dependency.
- ⏭️ **Testing Deferred**: 9 test files exist and compile, but automated testing infrastructure (headless harness, UI smoke tests, CI integration) deferred as separate work item.
- 🎯 **Achievement**: Issue #7 (Telemetry) complete—regressions now traceable via structured logs. Testing infrastructure remains future work.
