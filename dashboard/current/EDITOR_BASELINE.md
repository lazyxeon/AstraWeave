# AstraWeave Editor – Phase 0 Baseline (2025-12-07 18:15 -05:00)

This document captures the factual state of `tools/aw_editor` before Phase 0 stabilization work begins.
All observations below were gathered by running `cargo llvm-cov --package aw_editor --all-features --lcov --output-path coverage/aw-editor.lcov`
with the shared configuration in `llvm-cov.toml` and by inspecting the crates and tests referenced.

## CI and Instrumentation
- `.github/workflows/editor-ci.yml` now enforces `cargo fmt`, `cargo clippy`, `cargo check`, headless smoke tests, and publishes the `cargo llvm-cov` artifact on every PR touching the editor (see `.github/workflows/editor-ci.yml:1`).  
  The job uploads both the LCOV file and the HTML report plus a summary extracted from `coverage/summary.json` so coverage deltas are always reviewable (`.github/workflows/editor-ci.yml:63-90`).
- The root `llvm-cov.toml` forces `--all-features`, disables early aborts, and filters generated/test-only paths so measurements stay comparable to the north-star targets (`llvm-cov.toml:1-16`).

## Coverage Snapshot (`cargo llvm-cov` on Windows, Rust 1.89.0)

| Metric    | Covered / Total | Percent |
|-----------|-----------------|---------|
| Lines     | 5 398 / 16 032  | 33.67 % |
| Functions | 592 / 1 383     | 42.81 % |
| Regions   | 8 323 / 24 082  | 34.56 % |

Additional observations:
- The crate ships ten dedicated integration files (`tools/aw_editor/tests/*.rs`) covering behavior editor, gizmos, runtime, prefab workflows, and UI smoke flows.
- Scene serialization + workflow benchmarks now run under CI via the `content_scene_path` helper that fabricates temp files inside `content/tests/…`, satisfying the security wrapper without mocking (`tools/aw_editor/tests/integration_tests.rs:56-113`).
- Hot-reload watcher tests are still skipped, so we have no automated coverage for notifying pipelines when assets change on disk.
- The gizmo undo smoke tests now assert both undo depth and position reversion after the `GizmoHarness` learned how to expose real stack depth (`tools/aw_editor/src/headless.rs:162-196`) and after `GizmoState` began tracking the last active mode for deferred commits (`tools/aw_editor/src/gizmo/state.rs:118-215`). 

## Undo / Redo Reality Check
- `UndoStack` centralizes history as a bounded vector with cursor semantics and opt-in command merging (`tools/aw_editor/src/command.rs:114-210`).  
  The editor instantiates it with a hard-coded depth of 100 entries (`tools/aw_editor/src/main.rs:352`).
- Unit tests cover the foundational mechanics: execution, branching, max-size pruning, and description formatting (`tools/aw_editor/src/command.rs:930-1145`).  
  Integration tests extend this to entity-level flows (`tools/aw_editor/tests/integration_tests.rs:323-408` and `tools/aw_editor/tests/integration_tests.rs:757-783`).
- Remaining gaps relative to roadmap claims:
  - Clipboard/prefab actions skip the shared `UndoStack` entirely, so deleting or spawning via `PrefabManager` bypasses redo (see `tools/aw_editor/tests/prefab_workflow.rs:40-69`).
  - There is no telemetry or menu wiring that surfaces the per-command descriptions captured by `UndoStack::undo_description` (`tools/aw_editor/src/command.rs:245-279`).

## Runtime Baseline
- `EditorRuntime` still runs a stub loop: `tick` simply increments `World::t`, updates a moving average frame time, and copies entity counts (`tools/aw_editor/src/runtime.rs:51-228`).  
  No ECS/physics/audio schedulers are invoked yet, so the “real runtime” goal from the north-star document is not met.
- Tests prove deterministic snapshots and the play/pause/step lifecycle (`tools/aw_editor/src/runtime.rs:267-371` and `tools/aw_editor/tests/play_mode.rs:41-111`). Integration tests also cover deterministic replay but only within the headless harness (`tools/aw_editor/tests/integration_tests.rs:447-520`).
- Gaps:
  - Performance panel data is synthetic—the runtime never exports Tracy or WGPU timestamps yet (`tools/aw_editor/src/panels/performance_panel.rs:11-95` remains UI-only).
  - Stepping uses a fixed 60 Hz constant and does not observe the engine scheduler budget, so “real-time” validations are currently aspirational.
  - Play-mode instrumentation is isolated from the undo stack—entering/exiting play does not emit undoable commands, so state transitions cannot be reviewed.

## Prefab / Scene Facts
- `PrefabEntityData` only tracks names, poses, team IDs, and health maxima; there is no serialization of arbitrary components or nested prefabs (`tools/aw_editor/src/prefab.rs:17-53`).
- `collect_entity_recursive` stores a placeholder `children_indices` vector but never populates it, so parent/child hierarchies are dropped on serialization (`tools/aw_editor/src/prefab.rs:77-119`). Likewise, `prefab_reference` is persisted but never read.
- Instantiation is flat as well: `PrefabData::instantiate` simply spawns every stored entity without rebuilding relationships or registering undo commands, then returns a `PrefabInstance` whose override table only mirrors pose/health snapshots (`tools/aw_editor/src/prefab.rs:130-210`).  
  There is no ability to apply/revert overrides or to detect drift between source files and in-scene instances.
- Automated coverage is limited to two basic tests: a RON round-trip and a spawn smoke test (`tools/aw_editor/tests/prefab_workflow.rs:40-69`). The richer prefab suite mentioned in the roadmap is still commented out/ignored (`tools/aw_editor/tests/integration_tests.rs:585-647`).
- Practical implication: claims about hierarchical prefabs, override tracking, and hot-reload do not match the current code. Any Phase 1+ work must start by teaching `collect_entity_recursive` to truly walk the scene graph and by adding commands for apply/revert semantics.

## Immediate Risks to Track Into Phase 1
1. **Undo coverage gaps** – gizmo transactions and prefab/clipboard flows need real assertions before we can trust the ZERO-regression goal for `UndoStack`.
2. **Runtime realism** – until `EditorRuntime` executes the actual AstraWeave schedule, performance panels and deterministic replay claims remain unproven.
3. **Prefab hierarchy** – serialization ignores child graphs, so prefabs cannot yet satisfy “hierarchical overrides” requirements and will corrupt multi-entity selections.
4. **Hot-reload coverage** - watcher/file-change tests remain disabled, so the editor's asset pipeline can regress without detection.

These findings should remain pinned at the top of Phase 0 checklists; once the above four bullets are addressed, we can promote the roadmap back to source-of-truth status.
