# Phase 5: Codebase-Wide Test Coverage Analysis & Implementation Plan

**Date**: January 13, 2025  
**Scope**: Entire AstraWeave workspace (all testable crates)  
**Previous Achievement**: Phase 4 complete (astraweave-ai: 88% unit-testable coverage)  
**Current Objective**: Identify coverage gaps across ALL workspace crates and develop comprehensive testing plan

---

## Executive Summary

**Key Findings**:
- ✅ **Working Crates Identified**: 16 core crates compile and run tests successfully
- ❌ **Broken Crates**: 3 crates have compilation or test failures (astraweave-render, astraweave-scene, astraweave-memory)
- 📊 **Coverage Range**: 0.61% (profiling) to 88% (ai) with **average 16.8% across tested crates**
- 🎯 **Priority Gap**: 8 crates have <15% coverage and need significant test improvements
- 📈 **Test Inventory**: 621 existing tests across workspace (excluding Phase 4: astraweave-ai)

**Overall Grade**: 🟡 **C+ (Needs Improvement)**  
*Excellent coverage in astraweave-ai (88%) but significant gaps in 50% of other crates*

---

## 1. Coverage Analysis by Crate

### 1.1 High Coverage (>30%) - Maintain & Monitor 🟢

| Crate | Coverage | Tests | Lines Covered | Status | Priority |
|-------|----------|-------|---------------|--------|----------|
| **astraweave-ai** | **88.00%** | **148** | 320/380 | ✅ **EXCELLENT** | P5 (Monitor) |
| **astraweave-terrain** | **34.51%** | 91 | 1241/3596 | ✅ Good | P4 (Maintain) |
| **astraweave-ecs** | **31.46%** | 136 | 514/1634 | ✅ Good | P3 (Target 50%) |

**Analysis**: These crates have solid test foundations. Focus on maintaining coverage as new features are added.

---

### 1.2 Medium Coverage (15-30%) - Needs Improvement 🟡

| Crate | Coverage | Tests | Lines Covered | Gap | Priority |
|-------|----------|-------|---------------|-----|----------|
| **astraweave-gameplay** | **18.04%** | 15 | 448/2483 | -62% | **P1** (Critical) |
| **astraweave-behavior** | **13.51%** | 50 | 215/1591 | -67% | **P2** (High) |
| **astraweave-math** | **13.23%** | 34 | 189/1429 | -67% | P2 (High) |

**Analysis**: These have some tests but significant gaps. Need targeted test additions to reach 80% target.

---

### 1.3 Low Coverage (<15%) - Critical Gaps ❌

| Crate | Coverage | Tests | Lines Covered | Gap | Priority |
|-------|----------|-------|---------------|-----|----------|
| **astraweave-physics** | **10.47%** | 10 | 151/1442 | **-70%** | **P1** (Critical) |
| **astraweave-weaving** | **9.47%** | 21 | 138/1457 | **-71%** | **P1** (Critical) |
| **astraweave-input** | **7.11%** | 4 | 109/1533 | **-73%** | **P1** (Critical) |
| **astraweave-pcg** | **6.68%** | 19 | 93/1393 | **-73%** | **P1** (Critical) |
| **astraweave-nav** | **5.26%** | 26 | 72/1368 | **-75%** | **P1** (Critical) |
| **astraweave-audio** | **4.84%** | 19 | 93/1920 | **-75%** | **P1** (Critical) |
| **astraweave-security** | **3.34%** | 5 | 49/1466 | **-77%** | **P1** (Critical) |
| **astraweave-cinematics** | **1.57%** | 2 | 21/1336 | **-78%** | **P2** (High) |
| **astraweave-profiling** | **0.61%** | 7 | 8/1304 | **-79%** | P3 (Moderate) |

**Analysis**: These crates have minimal test coverage and represent **major risk**. Prioritize P1 crates (gameplay-critical systems).

---

### 1.4 Broken/Untestable Crates ⚠️

| Crate | Issue | Reason | Next Action |
|-------|-------|--------|-------------|
| **astraweave-render** | ❌ Compilation Failed | GPU/wgpu integration complexity | Diagnose build errors, fix dependencies |
| **astraweave-scene** | ❌ Compilation Failed | Async streaming dependencies | Fix async dependencies, add tests |
| **astraweave-memory** | ❌ 4 Test Failures | Logic errors in memory tracking | Fix failing tests (82/86 passing) |
| **astraweave-quests** | ❌ Compilation Failed | Missing dependencies | Add missing deps, validate build |
| **astraweave-dialogue** | ⚠️ No Tests | Empty test suite | Add initial test suite (0 tests) |

**Analysis**: These require **fixing before coverage analysis**. Prioritize render and memory (critical engine systems).

---

## 2. Priority Matrix: What to Test First?

### Priority 1 (P1) - CRITICAL: Gameplay-Essential Systems ⚠️

**Target**: 80% coverage minimum  
**Timeline**: 4-6 weeks (4-6 hours per crate)  
**Why Critical**: These directly affect gameplay quality, physics correctness, and player experience.

| Crate | Current | Gap | Tests Needed | Estimated Time |
|-------|---------|-----|--------------|----------------|
| **astraweave-physics** | 10.47% | -70% | ~80 tests | 6 hours |
| **astraweave-gameplay** | 18.04% | -62% | ~60 tests | 5 hours |
| **astraweave-weaving** | 9.47% | -71% | ~75 tests | 6 hours |
| **astraweave-audio** | 4.84% | -75% | ~85 tests | 7 hours |
| **astraweave-input** | 7.11% | -73% | ~80 tests | 6 hours |
| **astraweave-nav** | 5.26% | -75% | ~85 tests | 7 hours |
| **astraweave-security** | 3.34% | -77% | ~90 tests | 8 hours |

**Total**: P1 = ~555 tests, **45 hours** (~6 weeks @ 8h/week)

---

### Priority 2 (P2) - HIGH: Core Engine Systems 🟡

**Target**: 60% coverage minimum  
**Timeline**: 3-4 weeks (3-4 hours per crate)  
**Why High**: Support gameplay systems and provide critical infrastructure.

| Crate | Current | Gap | Tests Needed | Estimated Time |
|-------|---------|-----|--------------|----------------|
| **astraweave-behavior** | 13.51% | -47% | ~45 tests | 4 hours |
| **astraweave-math** | 13.23% | -47% | ~45 tests | 4 hours |
| **astraweave-cinematics** | 1.57% | -58% | ~55 tests | 5 hours |

**Total**: P2 = ~145 tests, **13 hours** (~4 weeks @ 3h/week)

---

### Priority 3 (P3) - MODERATE: Optimization & Expansion 🔵

**Target**: 50% coverage minimum  
**Timeline**: 2-3 weeks (2-3 hours per crate)  
**Why Moderate**: Important for maintaining quality but not gameplay-blocking.

| Crate | Current | Gap | Tests Needed | Estimated Time |
|-------|---------|-----|--------------|----------------|
| **astraweave-ecs** | 31.46% | -19% | ~20 tests | 2 hours |
| **astraweave-pcg** | 6.68% | -43% | ~40 tests | 3 hours |
| **astraweave-profiling** | 0.61% | -49% | ~45 tests | 3 hours |

**Total**: P3 = ~105 tests, **8 hours** (~3 weeks @ 3h/week)

---

### Priority 4 (P4) - MAINTAIN: Already Good ✅

**Target**: Maintain >30% coverage  
**Timeline**: Ongoing (add tests with new features)

| Crate | Current | Action |
|-------|---------|--------|
| **astraweave-terrain** | 34.51% | Monitor, add tests for new features |
| **astraweave-ai** | 88.00% | **EXCELLENT** - Use as reference model |

---

### Priority 5 (P5) - INVESTIGATE: Broken Crates 🔧

**Target**: Fix compilation/test failures first  
**Timeline**: 1-2 weeks (diagnosis + fixes)

| Crate | Issue | Action | Estimated Time |
|-------|-------|--------|----------------|
| **astraweave-render** | Compilation | Fix wgpu dependencies | 4 hours |
| **astraweave-scene** | Compilation | Fix async dependencies | 3 hours |
| **astraweave-memory** | 4 Test Failures | Debug and fix logic | 2 hours |
| **astraweave-quests** | Compilation | Add missing deps | 1 hour |
| **astraweave-dialogue** | No Tests | Add initial test suite | 3 hours |

**Total**: P5 = **13 hours** (~2 weeks @ 6-7h/week)

---

## 3. Detailed Testing Strategy by Crate

### 3.1 astraweave-physics (10.47% → 80% target) [P1]

**Current State**:
- **Coverage**: 10.47% (151/1442 lines)
- **Tests**: 10 total (all passing)
- **Uncovered Modules**: `async_scheduler.rs` (0/8 lines), `spatial_hash.rs` (59/76 partial)

**Critical Gaps** (70% uncovered):
```rust
// astraweave-physics/src/async_scheduler.rs (0% coverage)
- AsyncPhysicsScheduler::new() - Constructor test
- schedule_physics_step() - Core async scheduling
- wait_for_completion() - Sync barrier validation
- error handling for async failures

// astraweave-physics/src/spatial_hash.rs (77.63% coverage, but only lib tests)
- Grid cell insertion/removal edge cases
- Query radius boundary conditions
- Performance regression tests (O(n log n) validation)
```

**Recommended Test Additions** (~80 tests, 6 hours):

1. **AsyncScheduler Tests** (15 tests, 1.5 hours):
   - Constructor with various thread pool sizes (3 tests)
   - Basic scheduling (create → schedule → complete) (3 tests)
   - Error handling (panic recovery, timeout) (4 tests)
   - Concurrent scheduling (multiple steps in flight) (3 tests)
   - Cancellation and cleanup (2 tests)

2. **Spatial Hash Edge Cases** (20 tests, 2 hours):
   - Boundary conditions (entities at grid edges) (5 tests)
   - Large query radius (spanning multiple cells) (4 tests)
   - Empty cells and sparse grids (3 tests)
   - Dynamic entity insertion/removal during queries (5 tests)
   - Performance regression tests (3 tests)

3. **Character Controller** (25 tests, 1.5 hours):
   - Slope handling (steep vs gentle) (6 tests)
   - Step detection and traversal (5 tests)
   - Ground check validation (standing vs falling) (4 tests)
   - Collision resolution edge cases (5 tests)
   - Velocity clamping and friction (5 tests)

4. **Rigid Body Integration** (20 tests, 1 hour):
   - Force application (impulse vs continuous) (5 tests)
   - Constraint violations (3 tests)
   - Sleep/wake state transitions (4 tests)
   - Multi-body interactions (5 tests)
   - Collision callbacks (3 tests)

**Example Test (async_scheduler.rs)**:
```rust
#[tokio::test]
async fn test_async_scheduler_basic_workflow() {
    let scheduler = AsyncPhysicsScheduler::new(4).unwrap();
    let step_id = scheduler.schedule_physics_step(Duration::from_secs(1)).await.unwrap();
    let result = scheduler.wait_for_completion(step_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_scheduler_concurrent_steps() {
    let scheduler = AsyncPhysicsScheduler::new(4).unwrap();
    let step1 = scheduler.schedule_physics_step(Duration::from_millis(100)).await;
    let step2 = scheduler.schedule_physics_step(Duration::from_millis(100)).await;
    
    // Both should complete without blocking each other
    let (r1, r2) = tokio::join!(
        scheduler.wait_for_completion(step1.unwrap()),
        scheduler.wait_for_completion(step2.unwrap())
    );
    assert!(r1.is_ok() && r2.is_ok());
}
```

**Success Criteria**: 80% coverage, all async tests passing, spatial hash performance validated

---

### 3.2 astraweave-gameplay (18.04% → 80% target) [P1]

**Current State**:
- **Coverage**: 18.04% (448/2483 lines)
- **Tests**: 15 total (all passing)
- **Uncovered Modules**: Many gameplay mechanics missing tests

**Critical Gaps** (62% uncovered):
```rust
// Combat system edge cases
- Damage calculation with armor/resistances
- Critical hit probability and damage multipliers
- Status effect application and duration
- Combo system state transitions

// Biome system validation
- Biome blending at boundaries
- Weather effects and transitions
- Resource spawning probabilities
- Difficulty scaling

// Dialogue system
- Branching conversation flows
- Choice validation and consequences
- NPC relationship tracking
```

**Recommended Test Additions** (~60 tests, 5 hours):

1. **Combat Physics** (20 tests, 2 hours):
   - Attack sweep geometry (cone, sphere, cylinder) (6 tests)
   - Damage calculation formulas (8 tests)
   - Status effects (bleed, stun, slow) (6 tests)

2. **Biome System** (15 tests, 1.5 hours):
   - Biome classification (terrain → biome mapping) (5 tests)
   - Blend zones (desert/forest transitions) (5 tests)
   - Resource generation validation (5 tests)

3. **Dialogue System** (15 tests, 1 hour):
   - Branching paths (3 choices × 2 levels) (6 tests)
   - Choice consequences (reputation, items) (5 tests)
   - State persistence across conversations (4 tests)

4. **Quest System** (10 tests, 0.5 hours):
   - Quest state transitions (inactive → active → complete) (4 tests)
   - Objective tracking (collect, kill, reach location) (6 tests)

**Example Test**:
```rust
#[test]
fn test_combat_physics_attack_sweep_cone() {
    let attacker_pos = Vec3::new(0.0, 0.0, 0.0);
    let attacker_dir = Vec3::new(1.0, 0.0, 0.0);
    let targets = vec![
        (entity_a, Vec3::new(2.0, 0.5, 0.0)), // In cone
        (entity_b, Vec3::new(2.0, 2.0, 0.0)), // Outside cone angle
        (entity_c, Vec3::new(5.0, 0.0, 0.0)), // Beyond range
    ];
    
    let hits = perform_attack_sweep(
        &physics_world, attacker_pos, attacker_dir, &targets,
        attack_range: 3.0, cone_angle: 45.0
    );
    
    assert_eq!(hits.len(), 1); // Only entity_a hit
    assert_eq!(hits[0].entity_id, entity_a);
}
```

**Success Criteria**: 80% coverage, all combat mechanics validated, biome transitions tested

---

### 3.3 astraweave-nav (5.26% → 80% target) [P1]

**Current State**:
- **Coverage**: 5.26% (72/1368 lines)
- **Tests**: 26 total (all passing, BUT only testing `lib.rs` which is 72/72 lines)
- **Uncovered Modules**: **ALL navigation algorithms** (navmesh, A*, portal graphs) have 0% coverage!

**Critical Gap Analysis**:
The 5.26% coverage is **misleading**—tests only cover `lib.rs` (re-exports). **Core navigation systems are untested**:
```rust
// Zero coverage on critical modules:
- navmesh.rs: Navmesh generation, triangulation
- astar.rs: A* pathfinding algorithm
- portal_graph.rs: Portal-based navigation
- path_smooth.rs: Path smoothing/optimization
```

**This is the HIGHEST RISK crate** despite appearing to have "26 tests"!

**Recommended Test Additions** (~85 tests, 7 hours):

1. **Navmesh Generation** (25 tests, 2.5 hours):
   - Triangulation correctness (convex hulls, Delaunay) (8 tests)
   - Edge cases (holes, overlaps, degenerate triangles) (7 tests)
   - Large mesh performance (>10k triangles) (5 tests)
   - Dynamic obstacle insertion/removal (5 tests)

2. **A* Pathfinding** (30 tests, 2.5 hours):
   - Basic shortest path (simple grids) (5 tests)
   - Obstacle avoidance (walls, barriers) (8 tests)
   - Heuristic accuracy (Manhattan vs Euclidean) (4 tests)
   - Path caching and invalidation (6 tests)
   - No-path-found scenarios (7 tests)

3. **Portal Graphs** (20 tests, 1.5 hours):
   - Portal detection (room connections) (6 tests)
   - Multi-room pathfinding (5 tests)
   - Dynamic portal opening/closing (5 tests)
   - Portal traversal cost calculation (4 tests)

4. **Path Smoothing** (10 tests, 0.5 hours):
   - String-pulling algorithm (5 tests)
   - Bezier curve fitting (3 tests)
   - Corner cutting validation (2 tests)

**Example Test**:
```rust
#[test]
fn test_astar_basic_shortest_path() {
    let navmesh = create_simple_grid_navmesh(10, 10);
    let start = Vec2::new(0.0, 0.0);
    let goal = Vec2::new(9.0, 9.0);
    
    let path = astar_find_path(&navmesh, start, goal).unwrap();
    
    // Should find direct diagonal path (Euclidean heuristic)
    assert!(path.len() >= 2);
    assert_eq!(path.first().unwrap(), &start);
    assert_eq!(path.last().unwrap(), &goal);
    
    // Validate path continuity (no jumps)
    for window in path.windows(2) {
        let distance = (window[1] - window[0]).length();
        assert!(distance <= navmesh.max_step_size());
    }
}

#[test]
fn test_navmesh_obstacle_insertion_invalidates_triangles() {
    let mut navmesh = create_simple_navmesh();
    let initial_triangle_count = navmesh.triangles().len();
    
    // Insert obstacle in center
    navmesh.insert_obstacle(Vec2::new(5.0, 5.0), radius: 2.0);
    
    // Should invalidate triangles overlapping obstacle
    assert!(navmesh.triangles().len() < initial_triangle_count);
    
    // Path through obstacle should be blocked
    let path = astar_find_path(&navmesh, Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0));
    assert!(path.is_err() || path.unwrap().iter().all(|p| {
        (p - Vec2::new(5.0, 5.0)).length() >= 2.0 // Must avoid obstacle
    }));
}
```

**Success Criteria**: 80% coverage with **ALL core navigation modules tested** (navmesh, A*, portals, smoothing)

---

### 3.4 astraweave-audio (4.84% → 80% target) [P1]

**Current State**:
- **Coverage**: 4.84% (93/1920 lines)
- **Tests**: 19 total (all passing)
- **Uncovered Modules**: `dialogue_runtime.rs` (6/44), `engine.rs` (79/134 partial), `voice.rs` (0/4)

**Critical Gaps** (75% uncovered):
```rust
// Audio engine core missing tests
- Spatial audio positioning (3D attenuation)
- Bus routing and mixing (master, music, SFX, voice)
- Dynamic crossfading
- Audio occlusion/reverb

// Dialogue runtime missing tests
- Voice line playback sequencing
- Subtitle synchronization
- Interrupt handling (combat vs dialogue)
```

**Recommended Test Additions** (~85 tests, 7 hours):

1. **Audio Engine Core** (30 tests, 3 hours):
   - Spatial positioning (distance attenuation) (8 tests)
   - 4-bus mixer (master, music, SFX, voice) (10 tests)
   - Crossfading (linear, exponential) (6 tests)
   - Occlusion/reverb zones (6 tests)

2. **Dialogue Runtime** (25 tests, 2 hours):
   - Voice line sequencing (queue, priority) (8 tests)
   - Subtitle sync (timing, formatting) (7 tests)
   - Interrupt handling (combat overrides) (5 tests)
   - NPC voice selection (personality matching) (5 tests)

3. **Performance Tests** (15 tests, 1 hour):
   - Many simultaneous sounds (100+ sources) (5 tests)
   - Audio streaming (large files) (5 tests)
   - Memory usage validation (5 tests)

4. **Integration Tests** (15 tests, 1 hour):
   - Audio + physics (footsteps on different materials) (5 tests)
   - Audio + cinematics (timeline sync) (5 tests)
   - Audio + UI (menu sounds) (5 tests)

**Example Test**:
```rust
#[test]
fn test_audio_engine_spatial_attenuation() {
    let mut engine = AudioEngine::new();
    let listener_pos = Vec3::ZERO;
    let source_id = engine.play_spatial_sound(
        "test.ogg",
        Vec3::new(10.0, 0.0, 0.0),
        max_distance: 20.0
    ).unwrap();
    
    // At listener position: full volume
    engine.set_listener_position(listener_pos);
    assert_approx_eq!(engine.get_source_volume(source_id), 1.0);
    
    // At source position: full volume
    engine.set_listener_position(Vec3::new(10.0, 0.0, 0.0));
    assert_approx_eq!(engine.get_source_volume(source_id), 1.0);
    
    // Halfway: ~0.5 volume (inverse square law)
    engine.set_listener_position(Vec3::new(5.0, 0.0, 0.0));
    assert!(engine.get_source_volume(source_id) >= 0.4 && <= 0.6);
    
    // Beyond max_distance: near-zero volume
    engine.set_listener_position(Vec3::new(25.0, 0.0, 0.0));
    assert!(engine.get_source_volume(source_id) < 0.1);
}

#[test]
fn test_dialogue_runtime_interrupt_priority() {
    let mut runtime = DialogueRuntime::new();
    
    // Start low-priority ambient dialogue
    let ambient_id = runtime.play_voice_line("ambient_chatter.ogg", priority: 1);
    assert!(runtime.is_playing(ambient_id));
    
    // High-priority combat barks should interrupt
    let combat_id = runtime.play_voice_line("under_attack.ogg", priority: 10);
    assert!(runtime.is_playing(combat_id));
    assert!(!runtime.is_playing(ambient_id)); // Ambient interrupted
}
```

**Success Criteria**: 80% coverage, spatial audio validated, 4-bus mixer tested

---

### 3.5 astraweave-security (3.34% → 80% target) [P1]

**Current State**:
- **Coverage**: 3.34% (49/1466 lines)
- **Tests**: 5 total (all passing)
- **Uncovered**: **Critical security systems** have near-zero coverage!

**Critical Gaps** (77% uncovered):
```rust
// Security is EXTREMELY RISKY with 3.34% coverage:
- Asset signature verification (cryptographic validation)
- Anti-cheat validation (game state integrity)
- Sandboxed scripting (isolation and capabilities)
- Tool validation (LLM tool whitelisting)
```

**This is HIGHEST PRIORITY for security-critical game systems!**

**Recommended Test Additions** (~90 tests, 8 hours):

1. **Asset Signature Verification** (30 tests, 3 hours):
   - Valid signatures (ed25519-dalek) (8 tests)
   - Invalid/corrupted signatures (8 tests)
   - Signature generation and signing (6 tests)
   - Performance (1000+ asset validation) (5 tests)
   - Attack scenarios (replay, MITM) (3 tests)

2. **Anti-Cheat Validation** (25 tests, 2.5 hours):
   - Game state integrity checks (8 tests)
   - Deterministic replay validation (7 tests)
   - Impossible action detection (5 tests)
   - Cheat pattern recognition (5 tests)

3. **Sandboxed Scripting** (20 tests, 2 hours):
   - Script isolation (no file system access) (6 tests)
   - Capability enforcement (6 tests)
   - Resource limits (CPU, memory) (5 tests)
   - Escape attempts (3 tests)

4. **Tool Validation** (15 tests, 0.5 hours):
   - Whitelisted tool execution (5 tests)
   - Blocked tool rejection (5 tests)
   - Parameter sanitization (5 tests)

**Example Test**:
```rust
#[test]
fn test_asset_signature_valid() {
    let keypair = generate_test_keypair();
    let asset_bytes = b"test_asset_content";
    let signature = sign_asset(asset_bytes, &keypair);
    
    let is_valid = verify_asset_signature(asset_bytes, &signature, &keypair.public);
    assert!(is_valid);
}

#[test]
fn test_asset_signature_corrupted_fails() {
    let keypair = generate_test_keypair();
    let asset_bytes = b"test_asset_content";
    let signature = sign_asset(asset_bytes, &keypair);
    
    // Corrupt the asset
    let mut corrupted = asset_bytes.to_vec();
    corrupted[0] ^= 0xFF;
    
    let is_valid = verify_asset_signature(&corrupted, &signature, &keypair.public);
    assert!(!is_valid);
}

#[test]
fn test_script_sandbox_file_system_blocked() {
    let sandbox = ScriptSandbox::new();
    let script = r#"
        let file = std::fs::File::open("/etc/passwd").unwrap();
    "#;
    
    let result = sandbox.execute(script);
    assert!(result.is_err()); // Should fail with permission denied
    assert!(result.unwrap_err().to_string().contains("file system access blocked"));
}
```

**Success Criteria**: 80% coverage, **all security attack scenarios tested**, no sandbox escapes possible

---

### 3.6 astraweave-weaving (9.47% → 80% target) [P1]

**Current State**:
- **Coverage**: 9.47% (138/1457 lines)
- **Tests**: 21 total (all passing)
- **Uncovered**: Veilweaver fate-weaving mechanics (core game mechanic!)

**Critical Gaps** (71% uncovered):
```rust
// Fate-weaving system (Veilweaver's unique mechanic)
- Thread manipulation (snip, splice, knot)
- Timeline branching and merging
- Causality validation (no paradoxes)
- Weave pattern matching
```

**Recommended Test Additions** (~75 tests, 6 hours):

1. **Thread Manipulation** (25 tests, 2.5 hours):
   - Snip operation (cut timeline) (8 tests)
   - Splice operation (merge timelines) (8 tests)
   - Knot operation (create checkpoint) (6 tests)
   - Undo/redo validation (3 tests)

2. **Timeline Branching** (20 tests, 2 hours):
   - Branch creation (choice → divergent futures) (7 tests)
   - Branch merging (convergent timelines) (6 tests)
   - Parallel timeline simulation (4 tests)
   - Branch pruning (invalid futures) (3 tests)

3. **Causality Validation** (15 tests, 1 hour):
   - Acyclic timeline verification (5 tests)
   - Grandfather paradox prevention (5 tests)
   - Temporal ordering constraints (5 tests)

4. **Weave Pattern Matching** (15 tests, 0.5 hours):
   - Pattern recognition (knot sequences) (6 tests)
   - Pattern-based rewards (5 tests)
   - Pattern complexity scoring (4 tests)

**Example Test**:
```rust
#[test]
fn test_weaving_snip_operation() {
    let mut timeline = Timeline::new();
    timeline.add_event(Event::new("event_a", t: 0));
    timeline.add_event(Event::new("event_b", t: 1));
    timeline.add_event(Event::new("event_c", t: 2));
    
    // Snip between event_a and event_b
    let result = timeline.snip(t: 0.5);
    assert!(result.is_ok());
    
    // Timeline should have branch point at t=0.5
    let branches = timeline.get_branches();
    assert_eq!(branches.len(), 2); // Original + new branch
    assert_eq!(branches[1].events().len(), 1); // Only event_a in new branch
}

#[test]
fn test_weaving_causality_violation_prevented() {
    let mut timeline = Timeline::new();
    timeline.add_event(Event::new("cause", t: 0));
    timeline.add_event(Event::new("effect", t: 1, depends_on: "cause"));
    
    // Try to snip before "cause" (would break causality)
    let result = timeline.snip(t: -0.5);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("causality violation"));
}
```

**Success Criteria**: 80% coverage, fate-weaving mechanics validated, no paradoxes possible

---

### 3.7 astraweave-input (7.11% → 80% target) [P1]

**Current State**:
- **Coverage**: 7.11% (109/1533 lines)
- **Tests**: 4 total (all passing)
- **Uncovered**: Input binding, device handling, gesture recognition

**Recommended Test Additions** (~80 tests, 6 hours):

1. **Input Binding System** (30 tests, 2.5 hours):
   - Key binding registration (10 tests)
   - Conflict detection (overlapping bindings) (8 tests)
   - Rebinding validation (7 tests)
   - Profile switching (5 tests)

2. **Device Handling** (25 tests, 2 hours):
   - Keyboard input (10 tests)
   - Mouse input (6 tests)
   - Gamepad input (gilrs integration) (9 tests)

3. **Gesture Recognition** (15 tests, 1 hour):
   - Swipe gestures (6 tests)
   - Multi-touch pinch/zoom (5 tests)
   - Tap vs hold detection (4 tests)

4. **Performance Tests** (10 tests, 0.5 hours):
   - High-frequency polling (1000+ inputs/sec) (5 tests)
   - Input buffering validation (5 tests)

**Success Criteria**: 80% coverage, all device types tested, gesture recognition validated

---

## 4. Timeline & Resource Allocation

### 4.1 Overall Timeline (15-17 weeks total)

**Phase 5A: Fix Broken Crates** (Weeks 1-2)
- astraweave-render compilation fix (4 hours)
- astraweave-scene async dependencies (3 hours)
- astraweave-memory test failures (2 hours)
- astraweave-quests missing deps (1 hour)
- astraweave-dialogue initial tests (3 hours)
- **Total**: 13 hours (~2 weeks @ 6-7h/week)

**Phase 5B: P1 Critical Tests** (Weeks 3-8)
- astraweave-physics: 80 tests, 6 hours
- astraweave-gameplay: 60 tests, 5 hours
- astraweave-weaving: 75 tests, 6 hours
- astraweave-audio: 85 tests, 7 hours
- astraweave-input: 80 tests, 6 hours
- astraweave-nav: 85 tests, 7 hours
- astraweave-security: 90 tests, 8 hours
- **Total**: 555 tests, 45 hours (~6 weeks @ 7-8h/week)

**Phase 5C: P2 High-Priority Tests** (Weeks 9-12)
- astraweave-behavior: 45 tests, 4 hours
- astraweave-math: 45 tests, 4 hours
- astraweave-cinematics: 55 tests, 5 hours
- **Total**: 145 tests, 13 hours (~4 weeks @ 3-4h/week)

**Phase 5D: P3 Moderate Tests** (Weeks 13-15)
- astraweave-ecs: 20 tests, 2 hours
- astraweave-pcg: 40 tests, 3 hours
- astraweave-profiling: 45 tests, 3 hours
- **Total**: 105 tests, 8 hours (~3 weeks @ 3h/week)

**Phase 5E: Documentation & Validation** (Weeks 16-17)
- Update TESTING_INITIATIVE_FINAL_SUMMARY.md (2 hours)
- Create per-crate testing reports (8 hours)
- Run full workspace coverage validation (2 hours)
- **Total**: 12 hours (~2 weeks @ 6h/week)

---

### 4.2 Resource Requirements

**Recommended Pace**: 6-8 hours/week for 15-17 weeks

**Equipment Needed**:
- Development machine with Rust 1.89.0+
- Tarpaulin for coverage measurement
- Tokio for async test runtime
- Tracy profiling tools (optional for perf tests)

**Skills Required**:
- Rust unit testing (criterion, tokio::test)
- Domain knowledge (physics, navigation, audio)
- Security testing (cryptographic validation)
- Game engine architecture understanding

---

## 5. Success Metrics

### 5.1 Coverage Targets by End of Phase 5

| Priority | Crate Count | Target Coverage | Current Avg | Gap |
|----------|-------------|-----------------|-------------|-----|
| P1 (Critical) | 7 | 80% | 8.3% | **+72%** |
| P2 (High) | 3 | 60% | 9.4% | **+51%** |
| P3 (Moderate) | 3 | 50% | 17.5% | **+33%** |
| P4 (Maintain) | 2 | 30% | 61.5% | ✅ Already Met |
| P5 (Broken) | 5 | Fix → 50% | N/A | N/A |

**Overall Workspace Target**: **65% average coverage** (up from current 16.8%)

---

### 5.2 Test Count Targets

| Phase | Tests Added | Cumulative Total | Hours |
|-------|-------------|------------------|-------|
| **Current Baseline** | 621 | 621 | N/A |
| **Phase 5A (Broken)** | ~50 | 671 | 13 |
| **Phase 5B (P1)** | 555 | 1,226 | 45 |
| **Phase 5C (P2)** | 145 | 1,371 | 13 |
| **Phase 5D (P3)** | 105 | 1,476 | 8 |
| **Phase 5E (Docs)** | 0 | 1,476 | 12 |
| **TOTAL** | **+855 tests** | **1,476 tests** | **91 hours** |

---

### 5.3 Validation Criteria

**Phase 5A Complete** when:
- ✅ All 5 broken crates compile successfully
- ✅ astraweave-memory 86/86 tests passing (fix 4 failures)
- ✅ astraweave-dialogue has initial test suite (>10 tests)

**Phase 5B Complete** when:
- ✅ All P1 crates have ≥80% coverage
- ✅ 555 new tests passing (0 failures)
- ✅ Security attack scenarios validated (no exploits)

**Phase 5C Complete** when:
- ✅ All P2 crates have ≥60% coverage
- ✅ 145 new tests passing
- ✅ Behavior trees and math utilities validated

**Phase 5D Complete** when:
- ✅ All P3 crates have ≥50% coverage
- ✅ 105 new tests passing
- ✅ ECS archetype handling validated

**Phase 5E Complete** when:
- ✅ Workspace coverage validation passes (65% avg)
- ✅ Documentation updated with final metrics
- ✅ Per-crate testing reports published

---

## 6. Risk Assessment

### 6.1 High-Risk Areas 🔴

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **Security gaps (3.34%)** | **CRITICAL** | High | Prioritize P1, add attack scenario tests |
| **Navigation untested (5.26%)** | **SEVERE** | High | Focus on A* and navmesh tests first |
| **Audio spatial positioning uncovered** | **HIGH** | Medium | Add 3D attenuation tests early |
| **Weaving mechanics untested** | **HIGH** | Medium | Veilweaver-specific tests in P1 |

---

### 6.2 Medium-Risk Areas 🟡

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **Physics async scheduler (0%)** | Medium | High | Add async tests in Phase 5B |
| **Behavior tree edge cases** | Medium | Medium | Enumerate all node types |
| **Math SIMD validation** | Medium | Low | Performance regression tests |

---

### 6.3 Low-Risk Areas 🟢

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **Profiling coverage (0.61%)** | Low | Low | Add basic integration tests |
| **ECS archetype expansion** | Low | Low | Focus on edge cases |
| **Terrain generation (34.51%)** | Low | Low | Maintain current coverage |

---

## 7. Conclusion & Next Steps

### 7.1 Summary

**Current State**:
- ✅ **astraweave-ai**: 88% coverage (Phase 4 complete) - **EXCELLENT**
- 🟡 **13 Other Crates**: 16.8% average coverage - **NEEDS SIGNIFICANT IMPROVEMENT**
- ⚠️ **5 Broken Crates**: Compilation/test failures - **MUST FIX FIRST**

**What This Plan Achieves**:
- **+855 tests** added across 16 crates
- **65% average workspace coverage** (up from 16.8%)
- **Zero broken crates** (all compilation issues resolved)
- **Security validation** (attack scenarios tested)
- **Production-ready navigation** (navmesh, A*, portals tested)

**Grade After Phase 5**: 🟢 **A- (Excellent)** - Up from C+ (Needs Improvement)

---

### 7.2 Immediate Next Steps (Week 1)

**Action 1**: Fix astraweave-memory test failures (4 failing tests)
```bash
cargo test -p astraweave-memory --lib -- --nocapture
# Debug failures, fix logic errors
```

**Action 2**: Fix astraweave-render compilation
```bash
cargo build -p astraweave-render 2>&1 | tee render_errors.txt
# Diagnose wgpu dependency issues
```

**Action 3**: Start P1 critical tests (astraweave-security first)
```bash
# Create test file: astraweave-security/src/signature_tests.rs
# Add 30 asset signature tests (Week 1 target: 15 tests)
```

**Action 4**: Update documentation
```bash
# Update TESTING_INITIATIVE_FINAL_SUMMARY.md with Phase 5 status
# Create PHASE_5_WEEK_1_PROGRESS.md
```

---

### 7.3 Long-Term Vision (Weeks 2-17)

**Weeks 2-8**: Execute P1 critical tests (security, navigation, physics, audio, gameplay)  
**Weeks 9-12**: Execute P2 high-priority tests (behavior, math, cinematics)  
**Weeks 13-15**: Execute P3 moderate tests (ECS, PCG, profiling)  
**Weeks 16-17**: Documentation, validation, and final metrics collection

**End Goal**: Production-ready game engine with **65% average test coverage**, **zero broken crates**, and **all critical gameplay systems validated**.

---

## 8. Appendix: Full Crate Status Table

| Crate | Tests | Coverage | Lines Covered | Status | Priority | Est. Hours |
|-------|-------|----------|---------------|--------|----------|------------|
| astraweave-ai | 148 | 88.00% | 320/380 | ✅ EXCELLENT | P5 (Monitor) | 0 |
| astraweave-terrain | 91 | 34.51% | 1241/3596 | ✅ Good | P4 (Maintain) | 2 |
| astraweave-ecs | 136 | 31.46% | 514/1634 | ✅ Good | P3 (Expand) | 2 |
| astraweave-gameplay | 15 | 18.04% | 448/2483 | 🟡 Needs Work | **P1 (Critical)** | 5 |
| astraweave-behavior | 50 | 13.51% | 215/1591 | 🟡 Needs Work | P2 (High) | 4 |
| astraweave-math | 34 | 13.23% | 189/1429 | 🟡 Needs Work | P2 (High) | 4 |
| astraweave-physics | 10 | 10.47% | 151/1442 | ❌ Critical Gap | **P1 (Critical)** | 6 |
| astraweave-weaving | 21 | 9.47% | 138/1457 | ❌ Critical Gap | **P1 (Critical)** | 6 |
| astraweave-input | 4 | 7.11% | 109/1533 | ❌ Critical Gap | **P1 (Critical)** | 6 |
| astraweave-pcg | 19 | 6.68% | 93/1393 | ❌ Critical Gap | P3 (Moderate) | 3 |
| astraweave-nav | 26 | 5.26% | 72/1368 | ❌ Critical Gap | **P1 (Critical)** | 7 |
| astraweave-audio | 19 | 4.84% | 93/1920 | ❌ Critical Gap | **P1 (Critical)** | 7 |
| astraweave-security | 5 | 3.34% | 49/1466 | ❌ Critical Gap | **P1 (Critical)** | 8 |
| astraweave-cinematics | 2 | 1.57% | 21/1336 | ❌ Critical Gap | P2 (High) | 5 |
| astraweave-profiling | 7 | 0.61% | 8/1304 | ❌ Critical Gap | P3 (Moderate) | 3 |
| astraweave-render | N/A | N/A | N/A | ⚠️ Broken | P5 (Fix) | 4 |
| astraweave-scene | N/A | N/A | N/A | ⚠️ Broken | P5 (Fix) | 3 |
| astraweave-memory | 82/86 | N/A | N/A | ⚠️ 4 Failures | P5 (Fix) | 2 |
| astraweave-quests | N/A | N/A | N/A | ⚠️ Broken | P5 (Fix) | 1 |
| astraweave-dialogue | 0 | N/A | N/A | ⚠️ No Tests | P5 (Fix) | 3 |

**Total Estimated Effort**: 91 hours (15-17 weeks @ 6-8h/week)

---

**END OF PHASE 5 ANALYSIS**
