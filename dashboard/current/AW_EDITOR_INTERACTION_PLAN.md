# AW Editor – Interaction Fix Plan

_Status: Draft (Nov 15, 2025)_

This document translates the known gizmo/grid failures into concrete engineering work. Scope covers spatial manipulation, snap/grid controls, and the data flow between the editor-side `EntityManager` and the authoritative ECS world.

## 1. Objectives
- Gizmo drags must persist entity transforms in the ECS world and push a single undo command per gesture.
- Grid visibility and snapping toggles must feed both rendering and gizmo math in real time.
- Scene data must stop diverging between the temporary `EntityManager` cache and `sim_world`.

## 2. Data Model Alignment
1. **Introduce `EditorSceneState`** (`tools/aw_editor/src/scene_state.rs`)
   - Holds `{ world: World, entity_cache: HashMap<EntityId, WorldHandle> }`.
   - Owns the authoritative `World` in Edit mode; `EditorApp::sim_world` becomes `Option<EditorSceneState>` to make room for simulation state later.
   - Provides helper APIs: `fn entity_pose(&self, entity: Entity) -> Pose`, `fn apply_transform(&mut self, entity, TransformDelta)`.
2. **Retire standalone `EntityManager`** for ECS-backed entities (keep it only for pure viewport debug meshes). The transform panel and gizmo will query `EditorSceneState` instead of the mock cache.

## 3. Gizmo Pipeline
1. **Transform Transaction**
   - Add `gizmo::transactions::TransformTransaction` capturing `{ entity, start_pose, accumulated_delta }`.
   - Start transaction on mouse-down, update during drag, commit on mouse release:
     - Apply delta through `EditorSceneState::apply_transform`.
     - Emit `MoveEntityCommand` populated with start/end pose and push via `UndoStack`.
   - Transaction handles cancel (Esc) to revert to `start_pose` without touching undo.
2. **Viewport Integration**
   - `ViewportWidget::ui` currently mutates `EntityManager`; replace with callbacks into `EditorSceneState`.
   - Add trait `TransformableScene` to decouple viewport crate from concrete storage – default impl for `EditorSceneState`.
3. **Snapping**
   - Feed toolbar toggles into the transaction: `TransformTransaction::next_delta` receives `SnappingConfig` (grid + angle). `SnappingConfig::snap_position` already exists; ensure `translate_gizmo.rs` uses it when `snap_enabled` is true.

## 4. Grid Rendering & Controls
1. **ViewportRenderState**
   - Create struct containing `show_grid`, `grid_major`, `grid_minor`, `snap_size`.
   - Toolbar writes into `ViewportRenderState`; viewport renderer reads it when submitting grid mesh.
2. **Renderer Changes**
   - `viewport/rendering.rs` gains `fn draw_grid(&self, state: &ViewportRenderState)`. When `show_grid == false`, skip the draw call entirely.
   - Implement major/minor grid lines based on `snap_size` (major every N steps, minor tinted). Tie F1 overlay text to current spacing.
3. **Gizmo Visualization**
   - Align axis/plane indicators with the snapped grid orientation (e.g., fade planar handles when grid hidden).

## 5. Input & UX Polish
- Assign distinct hotkeys (`Shift+G` for grid toggle) to avoid fighting the gizmo `G` shortcut; document in the toolbar tooltip.
- Surface live transform coordinates in a floating HUD near the gizmo; use color cues when snapping engages.
- Log each completed drag (`tracing::info!("gizmo_move", entity, delta)`), aiding regression triage.

## 6. Validation
- **Headless regressions**: add `tests/ui_gizmo.rs` that runs the gizmo transaction state machine without rendering – simulate drag events, assert final poses.
- **Manual checklist**: record reproduction steps in README (move entity with/without snap, toggle grid, undo/redo) and gate feature completion on successful walkthroughs.

## 7. Deliverables
1. New scene state module + wiring throughout `EditorApp`.
2. Refactored gizmo transaction system with undo integration.
3. Functional grid toggle/snap controls visible in both UI and renderer.
4. Tracing + documentation updates.

Once these pieces are merged we can progress to the authoring upgrades (node editor, asset ingestion) with confidence that spatial editing is trustworthy.
