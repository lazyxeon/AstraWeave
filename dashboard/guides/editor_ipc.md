# Editor IPC & Hot-Reload (MVP)

Goals: Define minimal schemas and channels for eframe/egui editor panels, hot-reload, and undo/redo.

## Panels & Data Model
- Scene: tree of entities (id, name, enabled, components[])
- Hierarchy: parent/child links
- Inspector: component schema introspection (serde-reflection/typetag)
- Profiler: per-subsystem timings (tracing subscriber)

## Hot-Reload Protocol
- Files watched: `assets/**/*.wgsl`, `assets/materials/**/*.toml`, `assets/scenes/**/*.json`, `assets/**/*.glb`
- Debounce: 150 ms; coalesce bursts within 500 ms window
- Message: `{ kind: "Reload", path: String, hash: u64, ts_ms: u64 }`
- Response: `{ kind: "Ack", ok: bool, error: Option<String> }`

## Undo/Redo Semantics
- Command pattern with diff snapshots (component-level granularity)
- Max history: 128 ops (configurable)
- Serialization: JSON patch for quick diffs

## Transport
- In-process channels by default (`crossbeam_channel` or `tokio::sync::mpsc`)
- Socket IPC optional for external tools (JSON lines over TCP localhost)
