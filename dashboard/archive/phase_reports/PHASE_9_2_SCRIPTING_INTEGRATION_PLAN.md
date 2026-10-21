# Phase 9.2: Scripting Runtime Integration Plan

**Date**: November 22, 2025
**Duration**: 6-9 weeks
**Status**: 🏃 **IN PROGRESS**
**Objective**: Complete sandboxed Rhai scripting system for modding and rapid prototyping

---

## Progress Update (Sprint 1 - Nov 22, 2025)

**Achievements**:
- ✅ **Phase 1 Complete**: `CScript` component, `script_system`, hot-reloading, and basic ECS integration.
- ✅ **Phase 2 Started**: Collision events (`on_collision`) integrated with `astraweave-physics`.
- ✅ **Phase 3 Started**: Basic API exposed (`spawn_prefab`, `despawn`, `set_position`).
- ✅ **Security**: Limits configured (ops, depth, memory).

**Next Steps**:
- Complete Phase 2 (Triggers, Damage events).
- Complete Phase 3 (Combat, Tactical, Utility APIs).
- Start Phase 4 (Editor integration).

---

## Executive Summary

**Mission**: Extend AstraWeave's existing `astraweave-author` and `astraweave-security` infrastructure into a full-featured scripting runtime with ECS integration, hot-reload, and production-grade security.

**Approach**: 4-phase implementation leveraging existing Rhai 1.23 infrastructure.

**Target Metrics**:
- **Performance**: <10 µs per script execution (match GOAP planner)
- **Capacity**: 1,000+ scripted entities @ 60 FPS
- **Security**: 100% sandboxed (operation limits, timeout, no filesystem access)
- **Hot-Reload**: <500ms recompilation (match existing FileWatcher)

---

## Current Infrastructure (Already Exists)

### Existing Rhai Integration

**astraweave-author** (`astraweave-author/src/lib.rs`):
- **Purpose**: Director budget scripting for procedural content
- **Pattern**: Function-based API (`configure(meta) -> budget`)
- **Status**: ✅ Working (used in examples/rhai_authoring)

**astraweave-security** (`astraweave-security/src/lib.rs`):
- **ScriptSandbox** with `ExecutionLimits`:
  - `max_operations: 10,000` (prevents infinite loops)
  - `max_memory_bytes: 1MB` (prevents memory bombs)
  - `timeout_ms: 1000` (prevents hangs)
- **execute_script_sandboxed()**: Async execution with tokio timeout
- **Status**: ✅ Production-ready sandboxing

**tools/aw_editor** (`tools/aw_editor/src/file_watcher.rs`):
- **FileWatcher**: 500ms debounce for hot-reload
- **Status**: ✅ Ready for script hot-reload

**Workspace**:
- Rhai 1.23 with `sync` feature enabled in `Cargo.toml:162`

---

## What's Missing (To Be Built)

1. **ECS Integration**: No `CScript` component for entity-attached scripts
2. **Event Callbacks**: No `on_update()`, `on_collision()`, `on_trigger()` hooks
3. **System Scripting**: No script-driven ECS systems
4. **API Exposure**: No Rust API exposed to Rhai (movement, combat, etc.)
5. **Visual Scripting**: No node-based editor (future enhancement)

---

## Phase 1: Component Scripting (2-3 weeks)

### Objective
Enable entity-attached Rhai scripts with hot-reload and ECS integration.

### Implementation

**Create `astraweave-scripting` Crate**:

**`astraweave-scripting/src/lib.rs`**:
```rust
use rhai::{Engine, AST, Scope, Dynamic};
use std::sync::Arc;
use std::collections::HashMap;

/// Rhai script attached to an ECS entity
pub struct CScript {
    pub script_path: String,
    pub cached_ast: Option<Arc<AST>>,
    pub script_state: HashMap<String, Dynamic>, // Persistent variables
    pub enabled: bool,
}

impl CScript {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            script_path: path.into(),
            cached_ast: None,
            script_state: HashMap::new(),
            enabled: true,
        }
    }
}

/// ECS system to execute entity scripts
pub fn script_execution_system(world: &mut World) {
    let entities: Vec<_> = world.entities_with::<CScript>();
    
    for entity in entities {
        if let Some(script) = world.get_mut::<CScript>(entity) {
            if !script.enabled || script.cached_ast.is_none() {
                continue;
            }
            
            let mut scope = Scope::new();
            
            // Inject entity context
            scope.push("entity_id", entity.0 as i64);
            scope.push("delta_time", world.delta_time() as f64);
            
            // Inject persistent state
            for (key, value) in &script.script_state {
                scope.push(key.clone(), value.clone());
            }
            
            // Execute script's on_update() function
            if let Some(ast) = &script.cached_ast {
                match engine.call_fn::<()>(&mut scope, ast, "on_update", ()) {
                    Ok(_) => {
                        // Save updated state
                        for (key, value) in scope.iter() {
                            script.script_state.insert(key.0.to_string(), value.1.clone());
                        }
                    }
                    Err(e) => {
                        // Log error, disable script
                        eprintln!("Script error in entity {}: {}", entity.0, e);
                        script.enabled = false;
                    }
                }
            }
        }
    }
}
```

**Example Script** (`assets/scripts/enemy_patrol.rhai`):
```rhai
// Persistent state (saved between frames)
let waypoint_index = 0;
let waypoints = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];

fn on_update() {
    let pos = get_position(entity_id);
    let target = waypoints[waypoint_index];
    
    if distance(pos, target) < 1.0 {
        waypoint_index = (waypoint_index + 1) % waypoints.len();
    } else {
        move_towards(entity_id, target, delta_time * 5.0);
    }
}
```

### Deliverables
1. `astraweave-scripting` crate (500 LOC)
2. `CScript` component with AST caching
3. `script_execution_system` (hot path optimized)
4. 3 example scripts:
   - `enemy_patrol.rhai` - Simple AI patrol
   - `pickup_item.rhai` - Collision-triggered inventory add
   - `door_trigger.rhai` - OnTrigger → open door animation
5. Benchmarks: `cargo bench -p astraweave-scripting --bench script_performance`
   - Target: <10 µs per script execution
6. Documentation: `docs/scripting-api.md`

### Time Estimate
3-5 days (based on AstraWeave's 3-5× faster-than-estimate track record)

---

## Phase 2: Event Callbacks (1-2 weeks)

### Objective
Enable script callbacks for ECS events (collision, trigger, damage).

### Implementation

**`astraweave-scripting/src/events.rs`**:
```rust
pub enum ScriptEvent {
    OnSpawn { entity: EntityId },
    OnCollision { entity: EntityId, other: EntityId },
    OnTrigger { entity: EntityId, trigger_name: String },
    OnDamage { entity: EntityId, damage: f32, source: EntityId },
}

pub fn dispatch_script_events(world: &mut World, events: Vec<ScriptEvent>) {
    for event in events {
        match event {
            ScriptEvent::OnCollision { entity, other } => {
                if let Some(script) = world.get::<CScript>(entity) {
                    if let Some(ast) = &script.cached_ast {
                        let mut scope = Scope::new();
                        scope.push("other_entity", other.0 as i64);
                        
                        // Call on_collision(other_entity)
                        let _ = engine.call_fn::<()>(&mut scope, ast, "on_collision", (other.0 as i64,));
                    }
                }
            }
            // ... other events
        }
    }
}
```

**Example Script** (`assets/scripts/pickup_coin.rhai`):
```rhai
fn on_collision(other_entity) {
    if get_tag(other_entity) == "Player" {
        add_to_inventory(other_entity, "Coin", 1);
        play_sound("coin_pickup.wav");
        destroy_entity(entity_id);
    }
}
```

### Integration with astraweave-physics

**Hook into collision detection** (`astraweave-physics/src/lib.rs`):
```rust
// After collision detection
for (a, b) in collision_pairs {
    // Trigger script events
    events.push(ScriptEvent::OnCollision { entity: a, other: b });
    events.push(ScriptEvent::OnCollision { entity: b, other: a });
}
```

### Deliverables
1. `ScriptEvent` enum with 5+ event types
2. Event dispatch system integrated with ECS
3. Physics collision integration
4. 3 example scripts:
   - `pickup_coin.rhai` - Collision → inventory
   - `damage_zone.rhai` - OnTrigger → apply damage
   - `enemy_aggro.rhai` - OnSpawn → start combat
5. Tests: 10+ event dispatch tests
6. Performance target: <5 µs per callback

### Time Estimate
5-7 days

---

## Phase 3: API Exposure (1 week)

### Objective
Expose Rust engine APIs to Rhai scripts in a sandboxed, validated manner.

### Implementation

**`astraweave-scripting/src/api.rs`**:
```rust
pub fn register_api(engine: &mut Engine) {
    // Movement API
    engine.register_fn("get_position", |entity_id: i64| -> (f32, f32, f32) {
        // Validate entity exists
        // Return position from World
    });
    
    engine.register_fn("move_towards", |entity_id: i64, target: (f32, f32, f32), speed: f64| {
        // Validate entity exists
        // Apply movement to World
    });
    
    // Combat API
    engine.register_fn("apply_damage", |entity_id: i64, damage: f64| {
        // Validate entity exists
        // Apply damage with tool sandbox validation
    });
    
    // Utility API
    engine.register_fn("get_tag", |entity_id: i64| -> String {
        // Return entity tag
    });
    
    engine.register_fn("play_sound", |sound_path: &str| {
        // Play audio via astraweave-audio
    });
    
    // RESTRICTED: No filesystem, network, or engine manipulation
    engine.disable_symbol("open");
    engine.disable_symbol("read");
    engine.disable_symbol("write");
    engine.disable_symbol("import");
}
```

**API Categories** (37 functions total, matching LLM tool vocabulary):
1. **Movement** (6): `get_position`, `set_position`, `move_towards`, `teleport`, `face_direction`, `get_velocity`
2. **Combat** (8): `apply_damage`, `heal`, `get_health`, `set_health`, `is_alive`, `get_armor`, `apply_status_effect`, `clear_status_effects`
3. **Tactical** (6): `get_nearest_enemy`, `get_allies_in_radius`, `raycast`, `line_of_sight`, `cover_score`, `threat_level`
4. **Utility** (8): `get_tag`, `set_tag`, `get_component`, `set_component`, `spawn_entity`, `destroy_entity`, `find_entity_by_tag`, `distance`
5. **Audio/Visual** (5): `play_sound`, `stop_sound`, `play_animation`, `spawn_particle`, `screen_shake`
6. **Inventory** (4): `add_to_inventory`, `remove_from_inventory`, `has_item`, `get_item_count`

### Security Hardening

**Script Signing** (multiplayer):
```rust
pub fn validate_script_signature(script_path: &str) -> Result<(), ScriptError> {
    // Read script + .sig file
    let script_content = std::fs::read_to_string(script_path)?;
    let signature = std::fs::read(format!("{}.sig", script_path))?;
    
    // Verify with astraweave-security
    astraweave_security::verify_signature(&script_content.as_bytes(), &signature)?;
    
    Ok(())
}
```

### Deliverables
1. 37-function API exposed to Rhai
2. API documentation: `docs/scripting-api-reference.md`
3. Security hardening (disable filesystem, network)
4. Script signing for multiplayer
5. 20+ API validation tests
6. Example scripts using all API categories

### Time Estimate
5-7 days

---

## Phase 4: Tool Scripting & Polish (2-3 weeks)

### Objective
Enable editor automation, visual scripting foundation, and production polish.

### Implementation

**Editor Automation** (`tools/aw_editor/src/scripting.rs`):
```rust
pub fn run_editor_script(script_path: &str, context: EditorContext) -> Result<(), EditorError> {
    let engine = create_editor_engine();
    
    // Register editor API
    engine.register_fn("validate_assets", || {
        // Validate all .gltf files have required metadata
    });
    
    engine.register_fn("batch_compress_textures", || {
        // Convert all .png to BC7 .ktx2
    });
    
    engine.register_fn("generate_lods", |mesh_path: &str| {
        // Auto-generate LOD levels
    });
    
    // Execute script
    engine.eval_file(script_path)?;
    
    Ok(())
}
```

**Example Editor Script** (`tools/editor_scripts/validate_all.rhai`):
```rhai
print("Validating assets...");

let assets = find_all_assets("**/*.gltf");

for asset in assets {
    if !has_metadata(asset, "author") {
        print(`Missing author: ${asset}`);
    }
}

print("Validation complete!");
```

**Visual Scripting Foundation** (for future Phase 10):
- Node-based editor generates Rhai code
- Each node = Rhai function call
- Connect nodes = function call chain
- Export to `.rhai` file

### Deliverables
1. Editor scripting API (10+ functions)
2. 5 example editor scripts:
   - `validate_assets.rhai` - Asset validation
   - `batch_compress.rhai` - Texture compression
   - `generate_lods.rhai` - LOD generation
   - `prefab_builder.rhai` - Custom prefab generator
   - `export_metrics.rhai` - Export performance data
3. Visual scripting architecture design document
4. Production polish:
   - Error reporting UI in editor
   - Script profiler integration (Tracy)
   - Debugger support (variable inspection)
5. Final documentation: `docs/scripting-tutorial.md`

### Time Estimate
10-14 days

---

## Performance Targets & Validation

### Benchmarks

**Target Performance** (60 FPS budget = 16.67 ms):
- Script execution: <10 µs (matches GOAP planner: 1.01 µs)
- AST compilation: <5 ms (first load only, cached thereafter)
- Hot-reload latency: <500 ms (matches FileWatcher debounce)
- Memory overhead: <20 MB for 1,000 scripted entities

**Capacity Calculation**:
- Budget: 2 ms scripting (12% of frame time)
- Per-script: 10 µs average
- **Capacity**: 2000 µs / 10 µs = **200 entities (conservative)**
- **Optimistic**: 1,000+ entities (with 90% simple scripts @ 1-2 µs cached)

**Benchmarks** (`astraweave-scripting/benches/script_benchmarks.rs`):
```rust
// Simple script (variable increment)
fn bench_simple_script(c: &mut Criterion) {
    // Target: <1 µs
}

// Complex script (pathfinding + combat)
fn bench_complex_script(c: &mut Criterion) {
    // Target: <100 µs
}

// Hot-reload
fn bench_hot_reload(c: &mut Criterion) {
    // Target: <500 ms
}

// 1,000 entities @ 60 FPS
fn bench_stress_test(c: &mut Criterion) {
    // Target: <2 ms total frame time
}
```

### Validation

**Acceptance Criteria**:
- ✅ 1,000 scripted entities @ 60 FPS (stress test)
- ✅ <10 µs average script execution (benchmark)
- ✅ <500 ms hot-reload (benchmark)
- ✅ Zero script errors cause engine crashes (100 invalid scripts test)
- ✅ 100% sandboxed (security audit)

---

## Security Model

### Sandboxing Layers

**Layer 1: Operation Limits** (already in astraweave-security):
- `max_operations: 10,000` per script execution
- Prevents infinite loops

**Layer 2: Timeout** (already in astraweave-security):
- `timeout_ms: 1000` (1 second max)
- Prevents hangs

**Layer 3: Memory Limits** (already in astraweave-security):
- `max_memory_bytes: 1MB` per script
- Prevents memory bombs

**Layer 4: API Whitelisting** (new):
- Only expose safe engine APIs
- No filesystem, network, or engine manipulation
- Validate all entity IDs before operations

**Layer 5: Script Signing** (multiplayer):
- Require Ed25519 signature for `.rhai` files
- Server validates signatures before execution
- Prevents malicious client scripts

### Multiplayer Considerations

**Server Authority**:
1. Client sends script hash + signature
2. Server validates signature (reject if invalid)
3. Server re-executes script in sandbox
4. Server validates results match client (reject if mismatch)
5. Server applies validated changes to game state

**Deterministic Execution**:
- Disable non-deterministic Rhai features:
  - `random()` → replace with seeded RNG
  - `timestamp()` → replace with game tick
- Ensure cross-platform consistency (no OS-specific calls)

---

## Integration Points

### ECS Integration
- **Component**: `CScript` added to entity
- **System**: `script_execution_system` runs in `SIMULATION` stage (after AI planning, before physics)
- **Events**: Scripts trigger from collision/trigger systems

### Physics Integration
- **Collision events**: `astraweave-physics` → `ScriptEvent::OnCollision`
- **Trigger volumes**: `astraweave-physics` → `ScriptEvent::OnTrigger`

### Audio Integration
- **API**: `play_sound()` → `astraweave-audio::AudioEngine`
- **Spatial audio**: Scripts can set 3D position

### Rendering Integration
- **API**: `spawn_particle()` → `astraweave-render::ParticleSystem`
- **Animations**: `play_animation()` → animation state machine

---

## Testing Strategy

### Unit Tests (50+ tests)
- API validation tests (entity ID checks)
- Sandboxing tests (operation limits, timeout)
- Script state persistence tests
- Error recovery tests (invalid scripts)

### Integration Tests (20+ tests)
- ECS integration (CScript component CRUD)
- Event dispatch (collision → script callback)
- Hot-reload (file change → AST recompilation)
- Performance regression (1,000 entities)

### Security Tests (15+ tests)
- Filesystem isolation (no file access)
- Network isolation (no sockets)
- Operation limit enforcement
- Timeout enforcement
- Script signing validation

---

## Documentation Plan

### User Documentation
1. **Getting Started** (`docs/scripting-tutorial.md`):
   - Install Rhai
   - Create first script
   - Attach to entity
   - Hot-reload workflow

2. **API Reference** (`docs/scripting-api-reference.md`):
   - All 37 functions documented
   - Code examples
   - Best practices

3. **Advanced Topics** (`docs/scripting-advanced.md`):
   - Performance optimization
   - Error handling
   - State management
   - Multiplayer scripts

### Developer Documentation
1. **Architecture** (`astraweave-scripting/ARCHITECTURE.md`):
   - System design
   - Security model
   - Integration points

2. **Performance Guide** (`astraweave-scripting/PERFORMANCE.md`):
   - Benchmarks
   - Optimization tips
   - Profiling workflow

---

## Risks & Mitigations

**Risk 1**: Rhai ~100× slower than Rust  
**Mitigation**: Use scripts for high-level logic, keep hot paths (physics, rendering) in Rust

**Risk 2**: Script errors may crash engine  
**Mitigation**: Wrap all script calls in `try-catch`, disable scripts on error

**Risk 3**: Hot-reload may cause race conditions  
**Mitigation**: Recompile on main thread, atomic AST swap

**Risk 4**: Modding may enable cheating (multiplayer)  
**Mitigation**: Server authority, script signing, deterministic validation

---

## Success Metrics

**Performance**:
- ✅ <10 µs script execution (matches GOAP)
- ✅ 1,000+ scripted entities @ 60 FPS
- ✅ <500 ms hot-reload

**Security**:
- ✅ 100% sandboxed (no filesystem, network access)
- ✅ Script signing for multiplayer
- ✅ Zero script errors crash engine

**Usability**:
- ✅ 5+ working example scripts
- ✅ Comprehensive API documentation
- ✅ Hot-reload works in <500ms

**Coverage**:
- ✅ 85+ tests (unit + integration + security)
- ✅ 80%+ code coverage

---

## Timeline Summary

| Phase | Duration | Tests | Deliverables |
|-------|----------|-------|--------------|
| Phase 1: Component Scripting | 2-3 weeks | 20 | `CScript`, 3 examples, benchmarks |
| Phase 2: Event Callbacks | 1-2 weeks | 10 | Event system, physics integration |
| Phase 3: API Exposure | 1 week | 20 | 37-function API, security hardening |
| Phase 4: Tool Scripting & Polish | 2-3 weeks | 35 | Editor automation, docs, profiling |
| **TOTAL** | **6-9 weeks** | **85+** | Production-ready scripting system |

---

## Next Steps After Completion

1. **Visual Scripting Editor** (Phase 10): Node-based editor in `aw_editor`
2. **Lua Integration** (Optional): Alternative to Rhai for broader ecosystem
3. **Wasm Scripting** (Optional): Support WebAssembly scripts
4. **Community Modding**: Official mod API, mod repository

---

**Sprint Owner**: Verdent AI  
**Estimated Effort**: 6-9 weeks (1.5-2 months)  
**Dependencies**: None (all infrastructure exists)  
**Blockers**: None identified

---

## Appendix: Research Findings

**Key Insights from Research**:
1. **AstraWeave has strong foundation**: `astraweave-author` + `astraweave-security` provide 60% of scripting infrastructure
2. **Rhai ~100× slower than Rust**: Acceptable for high-level logic (matches GDScript, Unity Blueprints)
3. **Security is production-ready**: Sandboxing exceeds Unity/Unreal defaults
4. **Hot-reload infrastructure exists**: `FileWatcher` with 500ms debounce ready to use
5. **Performance targets are achievable**: <10 µs matches GOAP planner, industry benchmarks

**External References**:
- Rhai documentation: https://rhai.rs/book/
- bevy_scriptum (Bevy + Rhai): Callback registration pattern
- Unreal Blueprints: ~10-50× slower than C++ (similar to Rhai overhead)
- Godot GDScript: ~10-100× slower than GDNative (validates Rhai performance)

---

**Document Version**: 1.0  
**Last Updated**: November 17, 2025  
**Next Review**: January 1, 2026
