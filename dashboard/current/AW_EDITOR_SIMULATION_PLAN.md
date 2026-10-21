# AW Editor – Simulation & Play Mode Overhaul

_Status: Draft (Nov 15, 2025)_

The existing Play/Pause/Stop buttons toggle booleans without preserving designer changes or feeding the deterministic ECS loop. This plan re-architects simulation so the editor can enter/exit play mode safely.

## 1. Guiding Principles
- **Deterministic Snapshots**: Entering Play captures the exact ECS world (entities, prefabs, RNG seeds). Leaving Play restores or merges changes intentionally.
- **Isolated Simulation**: Runtime-only systems (AI, physics) update in a sandbox separate from edit-mode data structures.
- **Realtime Feedback**: Play state exposes HUD counters (frame time, entity count) and logs tool outputs.

## 2. State Management
1. Replace `sim_world: Option<World>` + `world_snapshot: Option<SceneData>` with:
   ```rust
   struct EditorRuntime {
       edit_world: World,
       play_world: Option<World>,
       snapshot: Option<WorldSnapshotBlob>,
       last_tick: Instant,
       schedule: SimulationSchedule,
   }
   ```
2. `edit_world` stays alive 24/7; viewport renders directly from it in Edit mode. No more deleting it when play stops.
3. When Play starts:
   - Serialize `edit_world` via `SceneData::from_world` (or direct binary snapshot) into `snapshot`.
   - Clone into `play_world` and seed deterministic timers.
4. When Stop triggers:
   - Drop `play_world`, restore `edit_world` from `snapshot`, and clear runtime-only systems.
   - Optionally expose “Apply Simulation Changes” later by diffing `play_world` to `edit_world` (out of scope for MVP).

## 3. Simulation Loop
1. Introduce `SimulationSchedule` that wraps the existing `world.tick(dt)` plus AI/planning orchestrators. It runs on the UI thread for now (future: background worker with channel).
2. Tick cadence:
   - Use fixed-step accumulator (e.g., 16.666 ms). Each frame, accumulate delta, run as many fixed steps as needed, clamp to prevent spiral of death.
   - Track `tick_count`, `sim_time`, and present in HUD.
3. Pause toggles simply stop consuming accumulated time; resume continues.
4. Provide instrumentation via `tracing::info_span!("aw_editor.play_tick", tick = tick_id)`. Emit metrics for AI plan count, physics time, etc.

## 4. UI & Controls
- Replace the current boolean toggles with a `PlayControls` widget that shows:
  - Mode indicator (Edit / Play / Paused).
  - Buttons: Play, Pause, Stop, Step (advance one tick while paused).
  - Stats: FPS, sim tick, entity count.
- Keyboard shortcuts still use F5/F6/F7 but now call into `EditorRuntime`.

## 5. Content Linking
- Before Play, flush pending edits: ensure gizmo/transform transactions, prefab apply actions, and asset instantiations have been written into `edit_world`.
- Inject `LevelDoc` data into `edit_world` via a resync pass so authored level structures (quests, fate threads) match what simulation expects.
- Provide hooks for AI pipelines: `EditorRuntime` exposes `llm_executor` so designers can preview Hermes/GOAP behavior while playing.

## 6. Telemetry & Testing
- Add `aw_editor/tests/play_mode.rs` that instantiates `EditorRuntime`, toggles Play/Pause/Stop, and asserts snapshot restoration.
- Log key transitions with durations (time to snapshot, time to restore) to help track regressions.
- Build a manual checklist verifying: entering play preserves edits, pausing halts AI, stepping advances single tick, stop restores original placements.

## 7. Dependencies
- Requires interaction plan (gizmo/world unification) to be implemented first; otherwise edit_world lacks accurate transforms.
- Optional: integrate with `astraweave-observability` span exporter for timeline debugging.

With this plan in place, aw_editor will behave like a real PIE (Play-In-Editor) system instead of a cosmetic toolbar.
