# AW Editor – Manual Checklist

_Version 0.1 • Nov 16, 2025_

## Overview
This living checklist captures the manual validation steps that accompany the Week 2 gizmo/grid work. Each run logs the operator, the scenario that was exercised, the telemetry signal that confirms success, and any follow-up notes.

## Latest Run – Nov 16, 2025
| Step | Scenario | Expected Outcome | Evidence |
| --- | --- | --- | --- |
| 1 | Translate entity with snapping **ON** | Entity moves to snapped grid coordinate, one undo entry is produced | `aw_editor.gizmo.*` spans show start/commit; `ui_gizmo_smoke::translate_drag_records_commit_event` plus manual drag matched telemetry payloads |
| 2 | Toggle grid snapping **OFF** then translate | Entity follows raw delta, grid overlay disappears, no grid render pass | Manual viewport capture (grid hidden) + new `grid_render` harness tests and `grid_pass_enabled` gating confirm GPU pass skipped |
| 3 | Translate then **Cancel** | Entity reverts to original position, undo depth unchanged | `ui_gizmo_smoke::cancel_reverts_world_and_emits_event` plus manual cancel gesture; telemetry includes `GizmoCancelled` event |
| 4 | Undo/Redo after snapped move | Undo returns to start, redo reapplies snapped pose | `tests/undo_transactions.rs` + manual keyboard shortcuts; telemetry shows `UndoApplied` / `RedoApplied` console messages |
| 5 | Drag `.prefab.ron` from asset browser into viewport (edit mode) | Prefab spawns at snapped cursor, becomes selected, undo stack increments | Console log shows ✅ spawn message, viewport outline highlights selection, `prefab_spawn_helper_records_undo_entry` mirrors the flow |
| 6 | Drag prefab while simulation is playing | Drop is rejected with clear log, drag cancels without mutating runtime world | Console message "Prefab drops are disabled while the simulation is running" plus paused undo depth |
| 7 | Move spawned prefab with gizmo then open inspector | Prefab Apply/Revert buttons enable automatically and list override deltas | Inspector displays override badge immediately; `prefab_manager_tracks_override_from_snapshot` backs the behavior |

## Notes
- Manual runs executed on Windows (DX12) build using `cargo run -p aw_editor` with default scene. All telemetry captured via `RUST_LOG=aw_editor=info` and persisted to `logs/week2_snap_validation.jsonl`.
- Automated coverage complements the checklist: `cargo test -p aw_editor` now includes `tests/grid_render.rs`, ensuring snapping config parity for both GPU and headless flows.
- Future manual runs should append new entries with date/operator to preserve the historical record required by the recovery plan.
