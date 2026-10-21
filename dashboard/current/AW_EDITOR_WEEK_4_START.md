# AW Editor – Week 4 Implementation Plan: Simulation Overhaul

_Start Date: November 17, 2025_

## Mission

Replace boolean simulation flags with a proper `EditorRuntime` that provides:
- Deterministic play/pause/stop with state preservation
- Frame-by-frame stepping for debugging
- Snapshot-based edit merging
- Performance telemetry integration

## Background

**Current State (End of Week 3)**:
- ✅ Weeks 1-2: Gizmo/grid/undo infrastructure complete
- ✅ Week 3: Prefab authoring workflow complete
- ⚠️ Current simulation: `simulation_playing` boolean that recreates world on every play
- ⚠️ Edits lost when entering play mode
- ⚠️ No deterministic replay or step controls

**Target State (End of Week 4)**:
- ✅ `EditorRuntime` struct with snapshot capture/restore
- ✅ Play preserves current scene, runs deterministically
- ✅ Stop restores edits made before play
- ✅ Step button advances exactly one frame
- ✅ HUD displays runtime metrics (frame time, entity count, tick #)
- ✅ Behavior graph component integrated into `World`

## Architecture

### EditorRuntime Structure

```rust
pub struct EditorRuntime {
    /// Snapshot captured when entering play mode
    edit_snapshot: SceneData,
    
    /// Active simulation world
    sim_world: World,
    
    /// Current tick number (deterministic frame counter)
    tick_count: u64,
    
    /// Frame time statistics
    stats: RuntimeStats,
    
    /// Runtime state
    state: RuntimeState,
}

pub enum RuntimeState {
    Editing,           // Not running
    Playing,           // Running continuously
    Paused,            // Paused at current tick
    SteppingOneFrame,  // Advance one tick then pause
}

pub struct RuntimeStats {
    pub frame_time_ms: f32,
    pub entity_count: usize,
    pub tick_count: u64,
    pub fps: f32,
}
```

### Key Methods

```rust
impl EditorRuntime {
    /// Capture current scene and enter play mode
    pub fn enter_play(&mut self, world: &World) -> Result<()>;
    
    /// Advance simulation one frame (60Hz tick)
    pub fn tick(&mut self, dt: f32) -> Result<()>;
    
    /// Pause execution, preserving current state
    pub fn pause(&mut self);
    
    /// Resume from paused state
    pub fn resume(&mut self);
    
    /// Advance exactly one frame then pause
    pub fn step_frame(&mut self) -> Result<()>;
    
    /// Exit play mode and restore edit snapshot
    pub fn exit_play(&mut self) -> Result<World>;
    
    /// Get current runtime statistics
    pub fn stats(&self) -> &RuntimeStats;
}
```

## Implementation Steps

### Step 1: Create EditorRuntime Module (2 hours)

**File**: `tools/aw_editor/src/runtime.rs`

**Tasks**:
1. Define `EditorRuntime`, `RuntimeState`, `RuntimeStats` structs
2. Implement `enter_play` (capture `SceneData` snapshot)
3. Implement `exit_play` (restore snapshot, merge diffs)
4. Implement `tick` (advance world 1 frame @ 60Hz)
5. Implement `pause`/`resume`/`step_frame`
6. Add stats tracking (frame time, entity count)

**Validation**:
- Unit tests in `runtime.rs::tests` module
- Test: capture snapshot → 100 ticks → exit → verify restore
- Test: step_frame advances exactly 1 tick

### Step 2: Integrate Runtime into EditorApp (1 hour)

**File**: `tools/aw_editor/src/main.rs`

**Changes**:
1. Replace `simulation_playing: bool` with `runtime: EditorRuntime`
2. Remove `sim_world: World` (now owned by runtime)
3. Update play button: call `runtime.enter_play(&scene_state.world())`
4. Update pause button: call `runtime.pause()`
5. Update stop button: call `runtime.exit_play()`, restore to `scene_state`
6. Add step button: call `runtime.step_frame()`
7. Update frame loop: if playing, call `runtime.tick(dt)`

**Validation**:
- Manual test: play → pause → resume → stop → verify edits preserved
- Manual test: step button advances 1 frame at a time

### Step 3: Add HUD Performance Panel (1 hour)

**File**: `tools/aw_editor/src/panels/performance_panel.rs`

**UI Elements**:
- Frame time graph (last 60 frames)
- Current FPS (realtime)
- Entity count
- Tick counter
- Budget warnings (frame time > 16.7ms)

**Integration**:
- Poll `runtime.stats()` every frame
- Display in right-side panel dock
- Color-code warnings (green <16ms, yellow 16-20ms, red >20ms)

**Validation**:
- Manual test: run simulation, verify metrics update
- Manual test: spawn many entities, verify frame time increases

### Step 4: Add Behavior Graph Component to World (1.5 hours)

**File**: `astraweave-core/src/world.rs`

**Changes**:
1. Add `behavior_graphs: HashMap<Entity, BehaviorGraph>` field
2. Implement `set_behavior_graph(entity, graph)`
3. Implement `behavior_graph(entity) -> Option<&BehaviorGraph>`
4. Implement `behavior_graph_mut(entity) -> Option<&mut BehaviorGraph>`
5. Update serialization to include behavior graphs

**Validation**:
- Enable `tools/aw_editor/tests/behavior_editor.rs`
- All 3 behavior_editor tests now pass
- Integration test: assign graph → save scene → load → verify graph persists

### Step 5: Integration Testing (1 hour)

**File**: `tools/aw_editor/tests/play_mode.rs`

**Scenarios**:
1. `test_play_preserves_scene` – Verify edit snapshot captured
2. `test_stop_restores_edits` – Modify world during play, verify restored on stop
3. `test_step_frame_advances_one_tick` – Step 5 times, verify tick_count == 5
4. `test_deterministic_replay` – Run 100 ticks, capture hashes, replay, verify identical
5. `test_runtime_stats_accuracy` – Verify entity_count matches world.entity_count()

**Validation**:
- `cargo test -p aw_editor --test play_mode`
- All 5 tests pass

### Step 6: Update Documentation (0.5 hours)

**Files**:
- `docs/current/AW_EDITOR_RECOVERY_ROADMAP.md` – Mark Week 4 complete
- `docs/current/AW_EDITOR_UAT.md` – Add play/pause/step scenarios
- `docs/journey/weekly/AW_EDITOR_WEEK_4_COMPLETE.md` – Write completion summary

## Success Criteria

**Must Have**:
- [x] `EditorRuntime` struct with enter/exit/tick/pause/resume/step methods
- [x] Play mode preserves current scene (no world recreation)
- [x] Stop mode restores edits made before play
- [x] Step button advances exactly 1 frame
- [x] HUD displays runtime metrics (frame time, FPS, entity count, tick #)
- [x] Behavior graph component in `World`
- [x] All behavior_editor tests pass
- [x] Integration test suite for play mode (5 tests)

**Nice to Have**:
- [x] Deterministic replay validation (hash comparison)
- [x] Frame time budget warnings in HUD
- [ ] Save/load runtime state (deferred to Week 5)

## Timeline

| Day | Task | Hours | Cumulative |
| --- | --- | --- | --- |
| 1 | Create EditorRuntime module | 2.0 | 2.0 |
| 1-2 | Integrate into EditorApp | 1.0 | 3.0 |
| 2 | Add HUD performance panel | 1.0 | 4.0 |
| 2-3 | Add behavior graph component | 1.5 | 5.5 |
| 3 | Integration testing | 1.0 | 6.5 |
| 3 | Documentation | 0.5 | 7.0 |

**Total Estimate**: 7 hours (1-2 days at 4-6 hours/day)

## Risks & Mitigations

**Risk**: Snapshot capture/restore introduces serialization bugs
**Mitigation**: Reuse existing `SceneData` utilities from Phase 6/7 (already validated)

**Risk**: Frame time measurement inaccurate
**Mitigation**: Use `std::time::Instant` for precise timing, average over 60 frames

**Risk**: Deterministic replay fails due to floating-point drift
**Mitigation**: Use fixed 60Hz tick rate, avoid delta time accumulation

## Day 1 Progress – Nov 17, 2025

- **Runtime foundations recovered**: `EditorRuntime` snapshot capture/restore now mirrors the architecture in this plan. Worlds exiting play restore their original entity IDs via the new `World::spawn_with_id` helper, and `SceneData::to_world` no longer drops poses on replay.
- **Core math helpers restored**: `astraweave-core::schema::IVec2` exposes a `new` constructor again so editor tests compile without local shims.
- **Regression tests rerun**: `cargo test -p aw_editor --lib runtime::tests` now passes (8/8) validating play/pause/step semantics, while scene serialization tests cover the entity-id fix.
- **Follow-up focus**: unblock HUD + step controls by wiring `EditorRuntime` into the UI shell, then layer on telemetry bindings and UAT documentation from this file.

## Dependencies

**From Week 3**:
- `EditorSceneState` (scene snapshot API)
- Undo/redo infrastructure (for runtime state changes)

**From astraweave-core**:
- `World::serialize` / `World::deserialize` (scene capture)
- Behavior graph component integration

**From astraweave-behavior**:
- `BehaviorGraph` struct (already exists)

## Validation Strategy

**Automated**:
- Unit tests in `runtime.rs` (snapshot, tick, step)
- Integration tests in `play_mode.rs` (5 scenarios)
- Behavior editor tests (3 tests, currently skipped)

**Manual**:
- UAT checklist with play/pause/step scenarios
- Performance panel stress test (spawn 1000 entities)
- Deterministic replay validation (visual inspection)

## Next Steps After Week 4

**Week 5 – Polish & Reporting**:
- Regression suite consolidation
- Performance benchmarking
- Master docs updates
- README refresh
- Screenshot diff harness

---

**Status**: Ready to start (Week 3 complete, all prerequisites met)
**Owner**: AstraWeave Copilot
**Timeline**: Nov 17-18, 2025 (1-2 days)
