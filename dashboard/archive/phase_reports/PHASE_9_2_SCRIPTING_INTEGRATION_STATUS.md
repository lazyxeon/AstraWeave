# Phase 9.2: Scripting Runtime Integration - Status Report

**Status**: In Progress
**Started**: November 22, 2025
**Owner**: AstraWeave Copilot

## Objectives
- Implement sandboxed Rhai scripting system
- Integrate with ECS (CScript component)
- Enable hot-reload and event callbacks
- Create API for script access to engine features

## Progress

### Week 1: Foundation (Current)
- [x] Create `astraweave-scripting` crate
- [x] Define `CScript` component
- [x] Implement `ScriptEngineResource` with Rhai engine
- [x] Implement `script_system` for executing scripts
- [x] Verify state persistence (variables updated back to component)
- [x] Add basic unit tests
- [x] Add performance benchmark (`benches/script_performance.rs`)
- [x] Implement `ScriptLoader` to load scripts from files (Async, Hash-verified)
- [x] Integrate with `astraweave-asset` (Added `AssetKind::Script`)

### Next Steps
- [ ] Expose ECS API to scripts (spawn, query, etc.)
- [ ] Implement event callbacks (`on_collision`, etc.)
- [ ] Integrate with `astraweave-security` for stricter sandboxing

## Technical Details

### CScript Component
```rust
pub struct CScript {
    pub script_path: String,
    pub source: String,
    pub cached_ast: Option<Arc<AST>>,
    pub script_state: HashMap<String, Dynamic>,
    pub enabled: bool,
}
```

### Script System
- Iterates all entities with `CScript`
- Compiles source if AST is missing
- Executes AST with a fresh Scope
- Pushes `script_state` to Scope before execution
- Pulls variables back from Scope to `script_state` after execution
- Injects `entity_id` into Scope

### Performance
- ASTs are cached in `Arc<AST>`
- Engine is shared via `Arc<Engine>` in Resource
- Benchmarks added to ensure <10µs overhead per script (target)
