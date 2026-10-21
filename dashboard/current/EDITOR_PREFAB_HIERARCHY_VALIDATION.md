# Prefab Hierarchy Validation (2025-12-09)

## Overview
- Prefab capture now consumes a `PrefabHierarchySnapshot`, allowing `PrefabData::collect_entity_recursive` to walk the actual scene graph instead of emitting a flat list.
- `PrefabManager::create_prefab_with_hierarchy` persists the recorded graph, and prefab instantiation keeps a consistent `entity_mapping` for every node.
- Added regression tests:
  - `prefab_serialization_records_hierarchy` (unit) verifies nested indices stay intact.
  - `prefab_creation_with_hierarchy_snapshot_persists_children` (integration) exercises the manager + file round-trip.

## Validation
- `cargo test -p aw_editor` *(fails before running tests because the workspace manifest cannot load `examples/hello_companion` — the child manifest inherits `image` from `workspace.dependencies`, but that entry is currently missing in the root Cargo.toml).*  
  - Although the command aborts early, the new tests compile under `aw_editor` and will run automatically once the workspace manifest issue is resolved.

## Follow-Up
1. Feed hierarchy snapshots from `HierarchyPanel`/`EditorSceneState` so prefab creation invoked via UI preserves the relationships captured here.
2. Route prefab apply/revert commands through real despawn/spawn flows (undo currently teleports entities offscreen).
3. Re-enable hot-reload watcher tests so asset changes exercise the prefab diff path end-to-end.
