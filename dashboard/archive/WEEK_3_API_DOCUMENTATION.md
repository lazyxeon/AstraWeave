# Week 3: API Documentation & Integration Guides

**Date**: January 2025 (October 20, 2025)  
**Phase**: Week 3 — Testing Sprint  
**Purpose**: Comprehensive API reference and integration patterns for developers  
**Audience**: Engine developers, game developers, contributors

---

## Table of Contents

1. [ActionStep API Reference](#actionstep-api-reference)
2. [Integration Patterns](#integration-patterns)
3. [Performance Best Practices](#performance-best-practices)
4. [Testing Patterns](#testing-patterns)
5. [Common Pitfalls](#common-pitfalls)

---

## ActionStep API Reference

### Overview

`ActionStep` is the **core enum** representing all possible AI agent actions in AstraWeave. It uses Rust's tagged enum pattern with serde serialization for JSON interchange.

**Location**: `astraweave-core/src/schema.rs` (lines 370+)

**Key Characteristics**:
- ✅ **Enum, not struct** — Use pattern matching, not field access
- ✅ **Serde tagged** — JSON serialization with `"act"` discriminator
- ✅ **38 variants** — Movement (6), Offensive (8), Defensive (6), Tactical, Utility, Support, Special
- ✅ **Type-safe** — Compiler-enforced action validation

---

### Core Definition

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "act")]
pub enum ActionStep {
    // Movement Actions
    MoveTo { 
        x: i32, 
        y: i32, 
        speed: Option<MovementSpeed> 
    },
    
    // Combat Actions
    Attack { 
        target_id: Entity 
    },
    
    // Tactical Actions
    TakeCover { 
        position: Option<IVec2> 
    },
    
    // ... 34+ more variants
}
```

**JSON Serialization Example**:
```json
{
  "act": "MoveTo",
  "x": 10,
  "y": 15,
  "speed": "Normal"
}
```

---

### Pattern Matching (Correct Usage)

#### ✅ **Correct**: Pattern Matching

```rust
use astraweave_core::ActionStep;

// Method 1: matches! macro (check only)
if matches!(step, ActionStep::MoveTo { .. }) {
    println!("Agent is moving");
}

// Method 2: match expression (extract fields)
match step {
    ActionStep::MoveTo { x, y, speed } => {
        println!("Moving to ({}, {})", x, y);
        if let Some(spd) = speed {
            println!("Speed: {:?}", spd);
        }
    }
    ActionStep::Attack { target_id } => {
        println!("Attacking target {:?}", target_id);
    }
    ActionStep::TakeCover { position } => {
        if let Some(pos) = position {
            println!("Taking cover at {:?}", pos);
        } else {
            println!("Taking cover (position auto-selected)");
        }
    }
    _ => {
        println!("Other action");
    }
}

// Method 3: if let (extract specific variant)
if let ActionStep::MoveTo { x, y, .. } = step {
    println!("Move destination: ({}, {})", x, y);
}
```

---

#### ❌ **Incorrect**: Field Access

```rust
// WRONG: ActionStep is NOT a struct
if step.tool == "MoveTo" {  // ❌ Compilation error: no field `tool`
    // ...
}

// WRONG: Cannot access fields directly
let x = step.x;  // ❌ Compilation error: no field `x`
let y = step.y;  // ❌ Compilation error: no field `y`
```

**Error Message**:
```
error[E0609]: no field `tool` on type `&ActionStep`
error[E0609]: no field `x` on type `&ActionStep`
```

---

### Wildcard Patterns (Avoiding Unused Bindings)

#### ✅ **Correct**: Wildcard for Unused Fields

```rust
// Only need to check action type, not extract fields
match step {
    ActionStep::MoveTo { .. } => {
        // Physics update (don't need x, y coordinates)
        agent_position.x += 0.5;
        agent_position.z += 0.5;
    }
    _ => {}
}
```

#### ⚠️ **Warning**: Extracting Unused Fields

```rust
// Triggers unused variable warnings
match step {
    ActionStep::MoveTo { x, y, speed } => {
        // ⚠️ Warning: unused variables x, y, speed
        agent_position.x += 0.5;  // Not using x, y
    }
    _ => {}
}
```

**Fix**: Use `..` wildcard:
```rust
ActionStep::MoveTo { .. } => { /* ... */ }
```

---

### Common ActionStep Variants

#### Movement Actions

```rust
ActionStep::MoveTo { x, y, speed }
ActionStep::Patrol { waypoints }
ActionStep::Follow { target_id, distance }
ActionStep::Retreat { direction }
```

**Usage**:
```rust
let move_action = ActionStep::MoveTo {
    x: 10,
    y: 15,
    speed: Some(MovementSpeed::Fast),
};
```

---

#### Combat Actions

```rust
ActionStep::Attack { target_id }
ActionStep::ThrowGrenade { target_pos, fuse_time }
ActionStep::Reload { weapon_slot }
ActionStep::SuppressFire { area }
```

**Usage**:
```rust
let attack_action = ActionStep::Attack {
    target_id: enemy_entity,
};
```

---

#### Tactical Actions

```rust
ActionStep::TakeCover { position }
ActionStep::FlankLeft { distance }
ActionStep::CallReinforcements { priority }
ActionStep::SetTrap { trap_type, position }
```

**Usage**:
```rust
let cover_action = ActionStep::TakeCover {
    position: Some(IVec2 { x: 5, y: 5 }),
};
```

---

### PlanIntent Structure

`PlanIntent` wraps a sequence of `ActionStep` variants with metadata.

```rust
pub struct PlanIntent {
    pub plan_id: String,        // Required (added in Phase 6)
    pub steps: Vec<ActionStep>, // Sequence of actions
}
```

**Usage**:
```rust
use astraweave_core::{PlanIntent, ActionStep};

let plan = PlanIntent {
    plan_id: "plan_12345".to_string(),
    steps: vec![
        ActionStep::MoveTo { x: 10, y: 5, speed: None },
        ActionStep::TakeCover { position: None },
        ActionStep::Attack { target_id: enemy },
    ],
};

// Iterate over steps
for step in &plan.steps {
    match step {
        ActionStep::MoveTo { x, y, .. } => {
            println!("Move to ({}, {})", x, y);
        }
        ActionStep::Attack { target_id } => {
            println!("Attack {:?}", target_id);
        }
        _ => {}
    }
}
```

---

## Integration Patterns

### Pattern 1: ECS → Perception (WorldSnapshot)

**Purpose**: Extract game state from ECS components into AI-friendly `WorldSnapshot`.

```rust
use astraweave_core::{WorldSnapshot, CompanionState, PlayerState, EnemyState, Poi};
use astraweave_ecs::{World, Entity};

fn extract_snapshot(
    world: &World,
    agent: Entity,
    enemies: &[Entity],
) -> WorldSnapshot {
    // Extract agent state
    let agent_pos = world.get::<Position>(agent).unwrap();
    let agent_ammo = world.get::<Ammo>(agent).map(|a| a.0).unwrap_or(0);
    let agent_health = world.get::<Health>(agent).map(|h| h.0).unwrap_or(100.0);
    
    // Extract enemy states
    let enemy_states: Vec<EnemyState> = enemies.iter()
        .filter_map(|&enemy| {
            let pos = world.get::<Position>(enemy)?;
            let hp = world.get::<Health>(enemy)?;
            
            Some(EnemyState {
                id: enemy.id() as u32,
                pos: IVec2 { x: pos.x as i32, y: pos.z as i32 },
                hp: hp.0 as i32,
                cover: "none".to_string(),
                last_seen: 0.0,
            })
        })
        .collect();
    
    // Build snapshot
    WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: agent_ammo,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: agent_pos.x as i32, y: agent_pos.z as i32 },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".to_string(),
            orders: vec![],
        },
        enemies: enemy_states,
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}
```

**Key Points**:
- ✅ Use `world.get::<T>(entity)` for component access
- ✅ Handle `Option<&T>` with `.map()`, `.unwrap_or()`, or `?`
- ✅ Convert world coordinates to grid coordinates (IVec2)
- ✅ Filter invalid entities with `.filter_map()`

---

### Pattern 2: Perception → Planning (dispatch_planner)

**Purpose**: Generate `PlanIntent` from `WorldSnapshot` using AI orchestrator.

```rust
use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::WorldSnapshot;

fn generate_plan(
    controller: &CAiController,
    snapshot: &WorldSnapshot,
) -> anyhow::Result<PlanIntent> {
    // Dispatch to appropriate planner (Rule-based, BehaviorTree, GOAP, LLM, etc.)
    let plan = dispatch_planner(controller, snapshot)?;
    
    // Validate plan has steps
    if plan.steps.is_empty() {
        anyhow::bail!("Empty plan generated");
    }
    
    Ok(plan)
}
```

**Key Points**:
- ✅ Use `dispatch_planner()` — don't call planners directly
- ✅ Always validate plan is non-empty
- ✅ Use `anyhow::Result` for error handling
- ✅ `CAiController` determines planning mode (Rule, BehaviorTree, etc.)

---

### Pattern 3: Planning → Physics (ActionStep Execution)

**Purpose**: Apply `ActionStep` effects to ECS world (physics, state updates).

```rust
use astraweave_core::ActionStep;
use astraweave_ecs::{World, Entity};

fn execute_action_step(
    world: &mut World,
    agent: Entity,
    step: &ActionStep,
) -> anyhow::Result<()> {
    match step {
        ActionStep::MoveTo { x, y, .. } => {
            // Update agent position (simplified physics)
            if let Some(pos) = world.get_mut::<Position>(agent) {
                let target_x = *x as f32;
                let target_z = *y as f32;
                
                // Move toward target (0.5 units per step)
                let dx = target_x - pos.x;
                let dz = target_z - pos.z;
                let dist = (dx * dx + dz * dz).sqrt();
                
                if dist > 0.5 {
                    pos.x += (dx / dist) * 0.5;
                    pos.z += (dz / dist) * 0.5;
                } else {
                    pos.x = target_x;
                    pos.z = target_z;
                }
            }
        }
        
        ActionStep::Attack { target_id } => {
            // Apply damage to target
            if let Some(target_hp) = world.get_mut::<Health>(*target_id) {
                target_hp.0 -= 10.0;  // Damage amount
            }
        }
        
        ActionStep::TakeCover { position } => {
            // Update agent state (crouching, cover position)
            if let Some(state) = world.get_mut::<AgentState>(agent) {
                state.in_cover = true;
                state.cover_position = *position;
            }
        }
        
        _ => {
            // Unhandled action (log or ignore)
            eprintln!("Unhandled action: {:?}", step);
        }
    }
    
    Ok(())
}
```

**Key Points**:
- ✅ Use `world.get_mut::<T>(entity)` for mutable access
- ✅ **No `mut` binding needed** — `get_mut()` returns `&mut T`
- ✅ Handle unimplemented actions gracefully (log, skip, or error)
- ✅ Use pattern matching, not field access

---

### Pattern 4: Physics → ECS Feedback (Multi-Frame Loop)

**Purpose**: Update ECS state, then re-perceive for next frame (feedback loop).

```rust
fn run_multi_frame_loop(
    world: &mut World,
    agent: Entity,
    enemies: &[Entity],
    controller: &CAiController,
    num_frames: usize,
) -> anyhow::Result<()> {
    for frame in 0..num_frames {
        // 1. Perception: Extract current world state
        let snapshot = extract_snapshot(world, agent, enemies);
        
        // 2. Planning: Generate plan from snapshot
        let plan = dispatch_planner(controller, &snapshot)?;
        
        // 3. Physics: Execute plan steps
        for step in &plan.steps {
            execute_action_step(world, agent, step)?;
        }
        
        // 4. ECS Update: State changes propagate to next frame
        // (Components updated in step 3 are visible in step 1 next iteration)
        
        println!("Frame {}: Plan executed with {} steps", frame, plan.steps.len());
    }
    
    Ok(())
}
```

**Key Points**:
- ✅ Loop: Perception → Planning → Physics → ECS Update → Repeat
- ✅ ECS state changes visible in next frame's snapshot
- ✅ Feedback enables adaptive behavior (agent reacts to world changes)
- ✅ Each frame is independent (no state carried between frames except ECS)

---

### Pattern 5: Helper Functions (Test Utilities)

**Purpose**: Reusable setup functions for tests and examples.

```rust
// Helper: Create test world with agents
fn create_test_world(agent_count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    let mut agents = vec![];
    
    for i in 0..agent_count {
        let agent = world.spawn();
        world.insert(agent, Position { x: i as f32, y: 0.0, z: i as f32 });
        world.insert(agent, Velocity { dx: 0.0, dy: 0.0, dz: 0.0 });
        world.insert(agent, Health(100.0));
        world.insert(agent, Ammo(10));
        agents.push(agent);
    }
    
    (world, agents)
}

// Helper: Create NavMesh for pathfinding tests
fn create_test_navmesh() -> NavMesh {
    use astraweave_nav::{NavMesh, Triangle};
    
    let triangles = vec![
        Triangle {
            vertices: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 10.0),
            ],
        },
        Triangle {
            vertices: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(0.0, 0.0, 10.0),
            ],
        },
    ];
    
    NavMesh::bake(&triangles, 0.5, 60.0)
}
```

**Key Points**:
- ✅ Reuse across tests to avoid duplication
- ✅ Parameterize for flexibility (`agent_count`, `navmesh_size`, etc.)
- ✅ Return owned data (`World`, `Vec<Entity>`) for test manipulation
- ✅ Keep helpers simple — focus on common setup patterns

---

## Performance Best Practices

### 60 FPS Frame Budget

**Target**: 16.67 ms per frame @ 60 FPS

**Budget Allocation**:
- **ECS Systems**: ~5 ms (30%)
- **AI Planning**: ~2 ms (12%)
- **Physics**: ~3 ms (18%)
- **Rendering**: ~5 ms (30%)
- **Overhead**: ~1.67 ms (10%)

---

### AI Systems Budget (Per Agent)

**Benchmarked Performance** (Week 3 Day 3):

| Complexity | Time per Agent | Max Agents @ 60 FPS | Budget % |
|------------|----------------|---------------------|----------|
| **Simple** | 135 ns | 123,000 | 0.0008% |
| **Moderate** | 802 ns | 20,800 | 0.0048% |
| **Complex** | 2.065 µs | 8,075 | 0.0124% |

**Guidelines**:
- ✅ Target <1 µs per agent for large-scale games (10,000+ agents)
- ✅ Use simple AI for background NPCs (135 ns)
- ✅ Use moderate AI for squad members (802 ns)
- ✅ Use complex AI for bosses/heroes (2.065 µs)

---

### Optimization Targets

**AI Planning** (Current: 87-202 ns):
- ✅ **No optimization needed** — Sub-microsecond planning achieved
- ✅ 4.95-11.5M plans/sec capacity
- ✅ Focus on correctness, not speed

**Snapshot Creation** (Current: 63 ns - 1.89 µs):
- ✅ **No optimization needed** — Linear scaling validated
- ✅ Simple snapshots: 63 ns (minimal overhead)
- ✅ Complex snapshots: 1.89 µs (acceptable for 10 enemies + 5 POIs + 20 obstacles)

**ECS Multi-System** (Current: 516 µs):
- ⚠️ **Optimization recommended** — 18.77% regression detected
- ⚠️ Target: <435 µs (restore previous performance)
- ⚠️ Use Tracy profiling to identify hotspots

---

### Batching Strategies

**ECS Component Access**:
```rust
// ❌ BAD: Scattered get_mut() calls (O(log n) per call)
for agent in &agents {
    if let Some(pos) = world.get_mut::<Position>(*agent) {
        pos.x += 1.0;
    }
}

// ✅ GOOD: Batch collect → process → writeback
let mut positions: Vec<_> = agents.iter()
    .filter_map(|&agent| world.get_mut::<Position>(agent))
    .collect();

for pos in &mut positions {
    pos.x += 1.0;  // SIMD-friendly loop
}

// Writeback happens automatically when mutable references dropped
```

**Performance**:
- ✅ Batching: 3-5× faster than scattered access
- ✅ Cache locality: Contiguous memory access
- ✅ SIMD-friendly: Compiler can auto-vectorize

---

### SIMD Movement Pattern

**Usage** (Week 8 optimization):
```rust
use astraweave_math::simd_movement::update_positions_simd;

// Batch positions and velocities
let mut positions: Vec<Vec3> = /* ... */;
let velocities: Vec<Vec3> = /* ... */;
let dt = 0.016; // 60 FPS delta time

// SIMD batch update (2.08× speedup)
update_positions_simd(&mut positions[..], &velocities[..], dt);
```

**Performance**:
- ✅ 2.08× speedup @ 10,000 entities
- ✅ BATCH_SIZE=4 loop unrolling
- ✅ glam auto-vectorization (80-85% of hand-written AVX2)

---

## Testing Patterns

### Integration Test Structure

**File**: `astraweave-ai/tests/cross_module_integration.rs`

```rust
use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{WorldSnapshot, ActionStep};
use astraweave_ecs::{World, Entity};

#[test]
fn test_full_ai_pipeline() {
    // 1. Setup: Create world and entities
    let (mut world, agents) = create_test_world(10);
    let enemies = vec![/* ... */];
    
    // 2. Perception: Extract WorldSnapshot
    let snapshot = extract_snapshot(&world, agents[0], &enemies);
    
    // 3. Planning: Generate plan
    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };
    let plan = dispatch_planner(&controller, &snapshot).unwrap();
    
    // 4. Validation: Check plan structure
    assert!(!plan.steps.is_empty(), "Plan should have steps");
    assert!(!plan.plan_id.is_empty(), "Plan should have ID");
    
    // 5. Physics: Execute plan
    for step in &plan.steps {
        execute_action_step(&mut world, agents[0], step).unwrap();
    }
    
    // 6. Verification: Check ECS state changed
    let final_pos = world.get::<Position>(agents[0]).unwrap();
    // assert!(final_pos changed from initial_pos)
}
```

**Key Points**:
- ✅ Test full pipeline, not just individual functions
- ✅ Use helper functions (`create_test_world`, `extract_snapshot`)
- ✅ Validate plan structure before execution
- ✅ Verify ECS state changes after execution

---

### Determinism Testing

**Pattern**: Run identical scenario multiple times, verify bit-identical results.

```rust
#[test]
fn test_determinism() {
    let num_runs = 3;
    let num_frames = 5;
    let agent_count = 5;
    
    let mut all_final_positions = vec![];
    
    for run in 0..num_runs {
        // Identical setup each run
        let (mut world, agents) = create_test_world(agent_count);
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };
        
        // Run N frames
        for _ in 0..num_frames {
            let snapshot = extract_snapshot(&world, agents[0], &[]);
            let plan = dispatch_planner(&controller, &snapshot).unwrap();
            
            for step in &plan.steps {
                execute_action_step(&mut world, agents[0], step).unwrap();
            }
        }
        
        // Record final position
        let final_pos = *world.get::<Position>(agents[0]).unwrap();
        all_final_positions.push(final_pos);
        
        println!("Run {}: Final position ({}, {}, {})", 
                 run, final_pos.x, final_pos.y, final_pos.z);
    }
    
    // Verify all runs produced identical results
    for i in 1..num_runs {
        assert_eq!(
            all_final_positions[i], 
            all_final_positions[0],
            "Run {} position differs from Run 0 (determinism broken)", 
            i
        );
    }
}
```

**Key Points**:
- ✅ Fixed seed for RNG (if used)
- ✅ Identical initial conditions
- ✅ Bit-identical comparison (exact float equality)
- ✅ Validates multiplayer/replay readiness

---

### Benchmark Structure

**File**: `astraweave-ai/benches/ai_core_loop.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_ai_planning(c: &mut Criterion) {
    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };
    let snapshot = create_simple_snapshot();
    
    c.bench_function("ai_loop_rule_planner_simple", |b| {
        b.iter(|| {
            let plan = dispatch_planner(
                black_box(&controller), 
                black_box(&snapshot)
            ).unwrap();
            black_box(plan)
        });
    });
}

criterion_group!(benches, bench_ai_planning);
criterion_main!(benches);
```

**Key Points**:
- ✅ Use `black_box()` to prevent compiler optimizations
- ✅ Benchmark realistic scenarios (simple, moderate, complex)
- ✅ Run with `cargo bench -p <crate> --bench <name>`
- ✅ Document results in completion reports

---

## Common Pitfalls

### Pitfall 1: ActionStep Field Access ❌

**Problem**: Treating `ActionStep` as struct instead of enum.

```rust
// ❌ WRONG
if step.tool == "MoveTo" {
    let x = step.x;
    let y = step.y;
}

// Error: no field `tool` on type `&ActionStep`
// Error: no field `x` on type `&ActionStep`
```

**Solution**: Use pattern matching.

```rust
// ✅ CORRECT
match step {
    ActionStep::MoveTo { x, y, .. } => {
        println!("Moving to ({}, {})", x, y);
    }
    _ => {}
}
```

---

### Pitfall 2: Unnecessary `mut` Binding ⚠️

**Problem**: Adding `mut` when `get_mut()` already returns mutable reference.

```rust
// ⚠️ WARNING: variable does not need to be mutable
if let Some(mut pos) = world.get_mut::<Position>(agent) {
    pos.x += 1.0;  // pos is already &mut Position
}
```

**Solution**: Remove `mut` from binding.

```rust
// ✅ CORRECT
if let Some(pos) = world.get_mut::<Position>(agent) {
    pos.x += 1.0;  // pos is &mut Position, mutation allowed
}
```

---

### Pitfall 3: Unused Pattern Bindings ⚠️

**Problem**: Extracting fields you don't use.

```rust
// ⚠️ WARNING: unused variables x, y
ActionStep::MoveTo { x, y, speed } => {
    // Only need to know it's a MoveTo, not the coordinates
    agent.x += 0.5;
}
```

**Solution**: Use wildcard pattern.

```rust
// ✅ CORRECT
ActionStep::MoveTo { .. } => {
    agent.x += 0.5;
}
```

---

### Pitfall 4: Empty Plan Validation ⚠️

**Problem**: Not validating plan has steps before execution.

```rust
// ⚠️ RISKY: Plan might be empty
let plan = dispatch_planner(&controller, &snapshot)?;
for step in &plan.steps {  // Loops 0 times if empty
    execute_action_step(&mut world, agent, step)?;
}
```

**Solution**: Validate plan before execution.

```rust
// ✅ CORRECT
let plan = dispatch_planner(&controller, &snapshot)?;

if plan.steps.is_empty() {
    eprintln!("Warning: Empty plan generated for agent {:?}", agent);
    return Ok(());  // Or use fallback behavior
}

for step in &plan.steps {
    execute_action_step(&mut world, agent, step)?;
}
```

---

### Pitfall 5: Scattered ECS Access 🐢

**Problem**: Repeated `get_mut()` calls cause archetype lookups.

```rust
// 🐢 SLOW: O(log n) archetype lookup per agent
for agent in &agents {
    if let Some(pos) = world.get_mut::<Position>(*agent) {
        pos.x += 1.0;
    }
}
```

**Solution**: Batch collect → process → writeback.

```rust
// ⚡ FAST: Single collect, SIMD-friendly loop, auto writeback
let mut positions: Vec<_> = agents.iter()
    .filter_map(|&agent| world.get_mut::<Position>(agent))
    .collect();

for pos in &mut positions {
    pos.x += 1.0;
}
```

---

## Quick Reference

### ActionStep Cheat Sheet

| Pattern | Purpose | Example |
|---------|---------|---------|
| `matches!(step, ActionStep::X { .. })` | Check variant type | `if matches!(step, ActionStep::MoveTo { .. })` |
| `match step { ActionStep::X { fields } => ... }` | Extract fields | `match step { ActionStep::MoveTo { x, y, .. } => ... }` |
| `if let ActionStep::X { fields } = step` | Extract specific variant | `if let ActionStep::Attack { target_id } = step` |
| `ActionStep::X { .. }` | Wildcard (ignore fields) | `ActionStep::MoveTo { .. } => { /* no bindings */ }` |

---

### ECS Access Cheat Sheet

| Operation | Method | Returns | Mutability |
|-----------|--------|---------|------------|
| Read component | `world.get::<T>(entity)` | `Option<&T>` | Immutable |
| Write component | `world.get_mut::<T>(entity)` | `Option<&mut T>` | Mutable |
| Spawn entity | `world.spawn()` | `Entity` | N/A |
| Insert component | `world.insert(entity, comp)` | `()` | Mutable world |

**Note**: `get_mut()` returns `&mut T`, no need for `mut` binding!

---

### Performance Targets

| System | Target | Current | Status |
|--------|--------|---------|--------|
| **AI Planning** | <1 µs | 87-202 ns | ✅ Excellent |
| **Snapshot Creation** | <2 µs | 63 ns - 1.89 µs | ✅ Excellent |
| **Full AI Loop** | <5 µs | 135 ns - 2.065 µs | ✅ Excellent |
| **ECS Multi-System** | <500 µs | 516 µs | ⚠️ Optimize |

---

## Conclusion

This documentation covers the essential APIs and patterns discovered during Week 3 testing sprint:

- ✅ **ActionStep**: Enum pattern matching (not field access)
- ✅ **Integration**: ECS → Perception → Planning → Physics → ECS feedback
- ✅ **Performance**: 60 FPS budgets, batching, SIMD
- ✅ **Testing**: Integration tests, determinism, benchmarks
- ✅ **Pitfalls**: Common mistakes and solutions

**For More Details**:
- Week 3 Day 1: [Warning Cleanup Report](WEEK_3_DAY_1_COMPLETION_REPORT.md)
- Week 3 Day 2: [Integration Tests Report](WEEK_3_DAY_2_COMPLETION_REPORT.md)
- Week 3 Day 3: [Performance Benchmarks Report](WEEK_3_DAY_3_COMPLETION_REPORT.md)

---

*Generated by AstraWeave AI-Native Engine Development*  
*AI-Generated Documentation — 100% AI-Driven Development Experiment*
