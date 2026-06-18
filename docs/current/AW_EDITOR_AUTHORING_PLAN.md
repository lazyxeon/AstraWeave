# AW Editor – Authoring Surface Upgrade Plan

_Status: Draft (Nov 15, 2025)_

Focus: replace placeholder authoring widgets (behavior graph, asset browser) with workflows that let designers build AI logic, bring assets into a scene, and manage prefabs gracefully.

## 1. Behavior Graph Editor
### Goals
- Interactive node-based editor backed by `astraweave_behavior::BehaviorGraph`.
- Import/export `.behavior.ron` files with validation and round-tripping.
- Support Action/Condition/Sequence/Selector/Decorator/Parallel nodes plus custom metadata (cooldowns, tags).

### Architecture
1. **Graph Data Model** (`tools/aw_editor/src/behavior_graph/model.rs`)
   - Mirror engine enums but include editor-only IDs.
   - Maintain adjacency lists + serialized metadata for layout positions.
2. **Egui Graph UI** (`behavior_graph/ui.rs`)
   - Reuse the existing `graph_panel` utilities from Astract Gizmo sprint (tabs, zoom, pan).
   - Drag nodes from a palette, connect sockets, right-click to delete, double-click to edit parameters.
3. **Persistence Layer**
   - Provide `BehaviorGraphDocument` with `load(path)`/`save(path)` bridging to `astraweave_behavior` via conversion traits.
   - Keep undo/redo history via `UndoStack` or lightweight local stack.
4. **Simulation Hooks**
   - Add “Preview” button that runs the edited graph against a mock `WorldSnapshot` and displays tick status.

### Validation
- Round-trip tests converting between `BehaviorGraphDocument` and runtime graph.
- UI smoke test verifying node creation, connection, and serialization.

## 2. Asset & Texture Ingestion
### Goals
- Allow users to drag a prefab/material/texture from the asset browser into the viewport to instantiate assets.
- Provide “Add to Scene” context actions with grid snapping and undo support.

### Implementation Steps
1. **Asset Catalog Refresh**
   - Scan `assets/` + `assets_src/` using `notify` watcher; store metadata (type, tags, preview) in `AssetDatabase` (already used elsewhere in workspace).
2. **Drag-and-Drop Pipeline**
   - Extend `ViewportWidget` to accept a `PendingDrop` struct from `AssetBrowser`.
   - On drop, call the appropriate handler:
     - Prefab: `PrefabManager::instantiate_prefab` + selection.
     - Mesh/Material: spawn placeholder entity referencing asset metadata.
     - Texture: open Material Inspector with texture pre-selected.
   - Wrap each drop in an `AddAssetCommand` for undo.
3. **Import Dialog**
   - Create `AssetImportWizard` (egui window) that copies files into `assets/` and updates metadata JSON.
   - Support automatic KTX conversion by invoking the existing pipeline scripts (`scripts/import_texture.ps1`).
4. **Scene Assignment**
   - Provide per-entity “Assign Material/Texture” menus in the Entity panel, using the same asset database.

### Validation
- Add CLI test (`cargo test -p aw_editor -- asset_ingest_smoke`) that mocks a prefab drop and asserts entity count increments.
- Document the workflow in `tools/aw_editor/README.md` (drag/drop, import wizard, undo/redo).

## 3. Prefab Override & Sync Workflow
### Goals
- Visualize when an instance has diverged from its source asset.
- Offer “Apply Overrides” and “Revert” actions with confirmation + undo.

### Implementation Steps
1. **Override Tracking**
   - After any transform/component change, call `PrefabInstance::track_override` for the owning entity (already available in `prefab.rs`).
   - Store override metadata in `HierarchyPanel` so entities display a badge (e.g., 🔵 for overridden, 🟢 for pristine).
2. **UI Controls**
   - Entity panel: new section showing source prefab path, override list, buttons for “Apply to Prefab” and “Revert”.
   - Hierarchy context menu: same actions plus “Locate Source”.
3. **Persistence**
   - When applying overrides, call `PrefabInstance::apply_to_prefab`. Reverting calls `revert_to_prefab` and refreshes the world snapshot.
   - Both actions become undoable commands so designers can experiment safely.

### Validation
- Unit tests covering `track_override`, apply, revert flows.
- Manual test plan ensuring badges update instantaneously after editing components.

## 4. Documentation & UX
- Expand `tools/aw_editor/README.md` with dedicated sections for the node editor, asset importer, and prefab workflow.
- Record tutorial videos / GIFs (future stretch) showing drag/drop and behavior graph authoring.

Deliverables from this plan will plug into the overarching recovery roadmap alongside the interaction and simulation work.
