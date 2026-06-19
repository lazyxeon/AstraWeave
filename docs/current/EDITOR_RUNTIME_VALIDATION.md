# EDITOR_RUNTIME_VALIDATION.md

**Date:** 2025-12-08  
**Scope:** `tools/aw_editor` runtime loop (Phase 0 → Phase 2 runway)

---

## Summary

- Replaced the stubbed `EditorRuntime::tick` with the real AstraWeave ECS/physics loop by embedding `astraweave_core::ecs_adapter::build_app` inside the runtime state machine.
- Added a deterministic 60 Hz accumulator, fixed-step batching, and explicit diagnostics via `RuntimeStats::fixed_steps_last_tick` so the UI can display how many authoritative frames were simulated per UI frame.
- Performance panel now surfaces the new stat and remains in sync with the real runtime metrics reported by the ECS world.

---

## Instrumentation & Tracy Hooks

- Every runtime update is wrapped in `span!("editor_runtime.tick")` with per-step zones for `editor_runtime.fixed_step`.
- Frame timing and entity counts are published via `plot!("EditorRuntime::frame_ms", …)` / `plot!("EditorRuntime::entities", …)` plus `frame_mark!()` to delimit editor frames.
- The `profiling` Cargo feature was reintroduced (`profiling = ["astraweave-profiling/profiling"]`) so enabling `--features profiling` turns on Tracy sampling without affecting default builds.

---

## Deterministic Replay Evidence

- `tools/aw_editor/tests/play_mode.rs::test_deterministic_replay` now exercises the upgraded runtime: two independent `EditorRuntime` instances step through identical input deltas and reach matching world hashes (captured through `hash_world`).
- `tools/aw_editor/tests/integration_tests.rs::test_runtime_deterministic_replay` performs the same verification inside the broad integration harness while also asserting that undo/redo history stays intact after play sessions.
- New unit tests (`tick_accumulates_until_full_step`, `step_frame_executes_single_fixed_step`) guarantee the accumulator and stepping paths cannot drift from the fixed 60 Hz cadence, providing direct coverage for the new code paths.

---

## Validation Matrix

| Command | Purpose |
|---------|---------|
| `cargo test -p aw_editor --lib` | Full library/unit coverage (includes the new runtime tests). |
| `cargo test -p aw_editor --test play_mode` | End-to-end play/pause/step snapshot validation using the real runtime loop. |
| `cargo test -p aw_editor --test integration_tests` | Prefab, undo stack, and runtime regression suite with deterministic replay assertions. |

> _Note_: All runs surface the pre-existing `astraweave-persona` warning (`unused_mut`), which predates this work and is unchanged.

---

## Next Steps

1. Wire runtime metrics into the Performance/Console panels for historical graphs once Tracy streaming is enabled in the editor UI.
2. Extend the ECS bootstrap to register physics/audio systems (currently only cooldown + movement) so play mode mirrors the full game loop specified in the Editor plan.
3. Gate “Apply Simulation Changes” behind an explicit command by diffing the `sim_app` world against the restored edit snapshot.
