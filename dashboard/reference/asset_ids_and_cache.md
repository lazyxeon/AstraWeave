# Asset IDs, GUIDs, and Cache Keys (MVP)

## GUID Format
- 128-bit UUID v4 in canonical string format (lowercase, hyphenated)
- Stored in sidecar `.meta` files alongside source assets:
  - `mesh.glb` ↔ `mesh.glb.meta` → `{ guid, import_settings, dependencies[] }`

## Dependency Graph
- Nodes: source assets and generated artifacts (meshes, textures, materials)
- Edges: `depends_on` with version + hash of source + import settings

## Cache Keys
- Key = `hash(source_bytes || import_settings || tool_version)`; use blake3
- Store compiled artifacts under `target/aw_cache/<guid>/<hash>/...`

## Invalidation
- On file change or settings change → recompute hash; if miss, rebuild
- Bubble-up invalidations to dependents (material includes texture; scene includes meshes)
