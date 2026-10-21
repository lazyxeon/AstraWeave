# Mutation Testing Hit List - P0 Crates

**Generated**: February 5, 2026  
**Tool**: cargo-mutants v26.2.0  
**Purpose**: Prioritized list of files needing improved test coverage based on mutation testing results

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Total Crates Tested** | 11 / 13 P0 |
| **Total Mutants Generated** | 11,072+ |
| **Total Caught** | 4,523+ |
| **Average Kill Rate** | 58% |
| **High Priority (< 50%)** | 3 crates |

---

## Kill Rate by Crate (Sorted by Priority)

### 🔴 Critical Priority (< 30% Kill Rate)

| Crate | Kill Rate | Caught | Missed | Viable | Action Required |
|-------|-----------|--------|--------|--------|-----------------|
| **astraweave-ai** | 8% | 154 | 1,783 | 1,937 | Major test gaps |
| **astraweave-terrain** | 30% | 1,349 | 3,141 | 4,490 | Large surface area |
| **astraweave-audio** | 30% | 35 | 82 | 117 | Small but weak |

### 🟡 Medium Priority (30-70% Kill Rate)

| Crate | Kill Rate | Caught | Missed | Viable | Action Required |
|-------|-----------|--------|--------|--------|-----------------|
| **astraweave-physics** | 51% | 1,033 | 1,007 | 2,040 | Edge cases |
| **astraweave-gameplay** | 62% | 379 | 236 | 615 | Boundary tests |

### 🟢 Good Coverage (70%+ Kill Rate)

| Crate | Kill Rate | Caught | Missed | Viable | Action Required |
|-------|-----------|--------|--------|--------|-----------------|
| **astraweave-ecs** | 78% | 261 | 75 | 336 | Polish |
| **astraweave-behavior** | 80% | 210 | 51 | 261 | Minor gaps |
| **astraweave-prompts** | 83% | 633 | 128 | 761 | Refinement |
| **astraweave-nav** | 86% | 254 | 41 | 295 | Near complete |
| **astraweave-math** | 94% | - | - | - | Excellent |
| **astraweave-core** | 98% | 215 | 5 | 220 | ⭐ Exemplary |

### ⏸️ Pending / In Progress

| Crate | Status | Notes |
|-------|--------|-------|
| **astraweave-render** | Partial (1.0%) | 38 of 3,682 tested, **54% kill rate** |
| **aw_editor** | Not started | UI-heavy |

#### astraweave-render Partial Results (38/3682 = 1.0%)

| Metric | Value |
|--------|-------|
| **Caught** | 20 |
| **Missed** | 17 |
| **Unviable** | 1 |
| **Kill Rate** | **54%** (20/37 viable) |
| **Test Time** | ~2.5 min/mutant |
| **Est. Full Run** | 147+ hours |

**Mutations Tested** (camera.rs only so far):
- `Camera::view_matrix` - 1 caught, 1 missed
- `Camera::proj_matrix` - caught
- `Camera::vp` - 2 caught
- `Camera::dir` - 4 caught, 1 missed
- `CameraController::is_dragging` - 2 caught
- `CameraController::process_keyboard` - 2 missed (match arm deletions)
- `CameraController::process_mouse_button` - 1 missed
- `CameraController::process_mouse_move` - 11 missed (no assertion tests!)
- `CameraController::process_mouse_delta` - in progress

**Key Finding**: Camera controller mouse handling has **zero test coverage** - all mutations passed tests despite code changes.

**Recommended Actions**:
1. Add assertion tests for `process_mouse_move` to verify yaw/pitch changes
2. Add tests for `process_keyboard` to verify movement state changes
3. Run targeted mutation tests: `cargo mutants -p astraweave-render -f camera.rs --timeout 120`
4. Full run impractical - prioritize file-by-file approach

---

## Detailed Hit Lists by Crate

### 🔴 astraweave-ai (8% Kill Rate - CRITICAL)

**1,783 missed mutations** - Major test infrastructure needed

| File | Mutants | Priority | Issue |
|------|---------|----------|-------|
| `src/orchestrator.rs` | ~400 | P0 | Core AI logic untested |
| `src/planning.rs` | ~350 | P0 | Planning algorithms |
| `src/perception.rs` | ~300 | P1 | Perception system |
| `src/behavior_tree.rs` | ~250 | P1 | BT execution |
| `src/utility.rs` | ~200 | P1 | Utility AI |
| `src/goap.rs` | ~180 | P2 | GOAP planner |
| `src/llm_executor.rs` | ~100 | P2 | LLM integration |

**Recommended Actions**:
1. Add unit tests for orchestrator state transitions
2. Add integration tests for planning pipelines
3. Mock LLM responses for executor tests
4. Test perception thresholds and edge cases

---

### 🔴 astraweave-terrain (30% Kill Rate - CRITICAL)

**3,141 missed mutations** - Largest crate, extensive gaps

| File | Mutants | Priority | Issue |
|------|---------|----------|-------|
| `src/marching_cubes_tables.rs` | 1,636 | P0 | Lookup tables untested |
| `src/advanced_erosion.rs` | 488 | P0 | Erosion algorithms |
| `src/heightmap.rs` | 290 | P0 | Height calculations |
| `src/climate.rs` | 202 | P1 | Climate simulation |
| `src/texture_splatting.rs` | 152 | P1 | Texture blending |
| `src/voxel_data.rs` | 147 | P1 | Voxel operations |
| `src/meshing.rs` | 144 | P1 | Mesh generation |
| `src/biome_blending.rs` | 140 | P1 | Biome transitions |
| `src/background_loader.rs` | 135 | P2 | Async loading |
| `src/chunk.rs` | 131 | P2 | Chunk management |
| `src/scatter.rs` | 116 | P2 | Object scattering |
| `src/lod_manager.rs` | 111 | P2 | LOD transitions |
| `src/structures.rs` | 109 | P2 | Structure placement |
| `src/terrain_modifier.rs` | 99 | P2 | Terrain editing |
| `src/partition_integration.rs` | 99 | P2 | World partitioning |
| `src/noise_gen.rs` | 97 | P2 | Noise functions |
| `src/erosion.rs` | 91 | P2 | Basic erosion |
| `src/biome.rs` | 85 | P3 | Biome definitions |
| `src/lod_blending.rs` | 82 | P3 | LOD blending |
| `src/streaming_diagnostics.rs` | 76 | P3 | Diagnostics |
| `src/noise_simd.rs` | 71 | P3 | SIMD noise |
| `src/terrain_persistence.rs` | 54 | P3 | Save/load |
| `src/solver.rs` | 38 | P3 | Physics solver |
| `src/lib.rs` | 31 | P3 | Module exports |

**Recommended Actions**:
1. Add marching cubes edge case tests (boundary conditions)
2. Add erosion algorithm verification tests
3. Add heightmap interpolation tests
4. Test biome blending at transition boundaries
5. Add chunk loading/unloading stress tests

---

### 🔴 astraweave-audio (30% Kill Rate)

**82 missed mutations** - Small but poorly covered

| File | Mutants | Priority | Issue |
|------|---------|----------|-------|
| `src/spatial.rs` | ~30 | P0 | 3D audio positioning |
| `src/mixer.rs` | ~25 | P0 | Audio mixing |
| `src/source.rs` | ~15 | P1 | Sound sources |
| `src/effects.rs` | ~12 | P2 | Audio effects |

**Recommended Actions**:
1. Add spatial audio distance attenuation tests
2. Add mixer channel blending tests
3. Test audio source lifecycle (play/pause/stop)

---

### 🟡 astraweave-physics (51% Kill Rate)

**1,007 missed mutations** - Complex physics edge cases

| File | Mutants | Priority | Issue |
|------|---------|----------|-------|
| `src/collision.rs` | ~200 | P0 | Collision detection |
| `src/rigid_body.rs` | ~180 | P0 | Physics simulation |
| `src/character_controller.rs` | ~150 | P1 | Character physics |
| `src/constraints.rs` | ~120 | P1 | Joint constraints |
| `src/forces.rs` | ~100 | P2 | Force application |
| `src/spatial_hash.rs` | ~80 | P2 | Spatial queries |

**Recommended Actions**:
1. Add collision edge case tests (grazing, penetration)
2. Add constraint violation tests
3. Test character controller ground detection

---

### 🟡 astraweave-gameplay (62% Kill Rate)

**236 missed mutations** - Gameplay logic gaps

| File | Mutants | Priority | Issue |
|------|---------|----------|-------|
| `src/combat.rs` | ~80 | P0 | Combat calculations |
| `src/inventory.rs` | ~50 | P1 | Item management |
| `src/quest.rs` | ~40 | P1 | Quest system |
| `src/dialogue.rs` | ~35 | P2 | Dialogue trees |
| `src/progression.rs` | ~31 | P2 | XP/leveling |

**Recommended Actions**:
1. Add combat damage calculation edge cases
2. Add inventory overflow/stack tests
3. Test quest state transitions

---

## Test Improvement Checklist

### Phase 1: Critical Gaps (Week 1-2)

- [ ] **astraweave-ai**: Add 50+ orchestrator tests
- [ ] **astraweave-terrain**: Add 100+ erosion/heightmap tests
- [ ] **astraweave-audio**: Add 20+ spatial audio tests

### Phase 2: Medium Priority (Week 3-4)

- [ ] **astraweave-physics**: Add 40+ collision edge case tests
- [ ] **astraweave-gameplay**: Add 30+ combat tests

### Phase 3: Polish (Week 5+)

- [ ] **astraweave-ecs**: Add 10+ edge case tests
- [ ] **astraweave-behavior**: Add 10+ BT tests
- [ ] **astraweave-prompts**: Add 15+ validation tests

---

## Mutation Categories to Target

### High-Value Mutation Types (Often Missed)

1. **Boundary Conditions**: `>=` vs `>`, `<=` vs `<`
2. **Return Value Changes**: `return x` → `return x + 1`
3. **Boolean Negations**: `!condition` → `condition`
4. **Arithmetic Operators**: `+` → `-`, `*` → `/`
5. **Default Values**: `Default::default()` mutations
6. **Option Handling**: `Some(x)` → `None`
7. **Result Handling**: `Ok(x)` → `Err(...)`

### Test Patterns That Catch Mutations

```rust
// Pattern 1: Exact value assertions
assert_eq!(calculate(5), 10);  // Catches +1/-1 mutations

// Pattern 2: Boundary testing
assert!(is_valid(0));          // Lower bound
assert!(is_valid(100));        // Upper bound
assert!(!is_valid(-1));        // Just below
assert!(!is_valid(101));       // Just above

// Pattern 3: Error path testing
assert!(process(-1).is_err()); // Error paths

// Pattern 4: State transition verification
let mut state = State::new();
state.transition(Event::A);
assert_eq!(state.current(), Expected::B);
```

---

## Running Mutation Tests

### Full Crate Test
```bash
cargo mutants -p <crate-name> --timeout 120 --jobs 4
```

### Single File Test
```bash
cargo mutants -p <crate-name> --file src/target.rs --timeout 60
```

### List Mutants Only (No Execution)
```bash
cargo mutants -p <crate-name> --list > mutants_list.txt
```

### Check Results
```bash
# Kill rate calculation
$caught = (Get-Content mutants.out/caught.txt | Measure-Object -Line).Lines
$missed = (Get-Content mutants.out/missed.txt | Measure-Object -Line).Lines
$rate = [math]::Round(100 * $caught / ($caught + $missed), 1)
Write-Host "Kill rate: $rate%"
```

---

## Version History

| Date | Version | Changes |
|------|---------|---------|
| 2026-02-05 | 1.0 | Initial hit list - 11/13 crates |

---

**Next Steps**: Complete astraweave-render and aw_editor mutation testing, then begin Phase 1 remediation.
