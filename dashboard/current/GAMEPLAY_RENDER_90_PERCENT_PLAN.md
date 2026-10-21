# Gameplay & Render 90%+ Coverage Implementation Plan

**Version**: 1.0  
**Created**: October 27, 2025  
**Objective**: Push Gameplay from 51.1% → 90%+, Render from 32.2% → 90%+  
**Status**: 🎯 **PLANNING COMPLETE** - Ready for implementation

---

## Executive Summary

**Current State** (Oct 27, 2025):
- **Gameplay**: 51.1% (555/1086 lines covered, 531 missed)
  - Gap to 90%: +38.9pp (~422 lines, 60-80 tests estimated)
- **Render**: 32.2% (3247/10084 lines covered, 6837 missed)
  - Gap to 90%: +57.8pp (~5,829 lines, 150-200 tests estimated)

**Total Effort Estimate**: 8-12 hours (3-4h Gameplay, 5-8h Render)

**Critical Blocker**: **renderer.rs** (3,431 lines @ 1.25%) represents 34% of Render code and 50% of total effort

---

## Part 1: Gameplay Coverage Push (51.1% → 90%+)

### 1.1 Current Coverage Breakdown

**Source-Only Metrics** (15 files, 1086 lines total):

| File | Lines | Covered | Coverage | Status | Priority |
|------|-------|---------|----------|--------|----------|
| **ecs.rs** | 192 | 184 | 95.83% | ⭐⭐⭐⭐⭐ | ✅ Done |
| **combat_physics.rs** | 237 | 225 | 94.94% | ⭐⭐⭐⭐ | ✅ Done |
| **combat.rs** | 73 | 55 | 75.34% | ⭐⭐ | Medium |
| **crafting.rs** | 43 | 25 | 58.14% | ⭐⭐ | Medium |
| **items.rs** | 25 | 14 | 56.00% | ⭐⭐ | Low |
| **stats.rs** | 34 | 15 | 44.12% | ⚠️ | Medium |
| **dialogue.rs** | 126 | 37 | 29.37% | ⚠️ | High |
| **weaving.rs** | 103 | 0 | 0.00% | ❌ | **CRITICAL** |
| **weave_portals.rs** | 95 | 0 | 0.00% | ❌ | **CRITICAL** |
| **cutscenes.rs** | 40 | 0 | 0.00% | ❌ | High |
| **biome_spawn.rs** | 38 | 0 | 0.00% | ❌ | Medium |
| **biome.rs** | 30 | 0 | 0.00% | ❌ | Low |
| **quests.rs** | 24 | 0 | 0.00% | ❌ | Medium |
| **harvesting.rs** | 18 | 0 | 0.00% | ❌ | Low |
| **weave_telemetry.rs** | 8 | 0 | 0.00% | ❌ | Low |

**Summary**:
- ✅ **Strong** (2 files, 429 lines): ecs.rs, combat_physics.rs (already >90%)
- ⭐⭐ **OK** (3 files, 141 lines): combat.rs, crafting.rs, items.rs (need push to 90%)
- ⚠️ **Weak** (2 files, 160 lines): dialogue.rs, stats.rs (need 60-80pp gain)
- ❌ **Zero** (8 files, 356 lines): weaving.rs (103L), weave_portals.rs (95L), others (158L)

### 1.2 Implementation Strategy

**Phase 1: Zero-Coverage Files (8 files, 356 lines, ~50-60 tests)**

**Batch 1A: Simple Utility Functions** (94 lines, 15-20 tests, 30 min):
- `biome.rs` (30 lines): `generate_island_room()` geometry generation
  - Test: Triangle count, vertex positions, ramp/plateau geometry
- `biome_spawn.rs` (38 lines): Spawn point generation
  - Test: Spawn distribution, biome-specific counts, position validation
- `harvesting.rs` (18 lines): Resource harvesting mechanics
  - Test: Resource collection, inventory updates, tool requirements
- `weave_telemetry.rs` (8 lines): Telemetry tracking
  - Test: `add_terrain()`, `add_weather()`, counter increments

**Batch 1B: Complex Systems** (262 lines, 35-40 tests, 1.5 hours):
- `weaving.rs` (103 lines): Fate-weaving core (apply_weave_op, budget consumption)
  - Test: Each WeaveOpKind (ReinforcePath, CollapseBridge, RedirectWind, LowerWater, RaisePlatform)
  - Test: Budget depletion, error handling (no budget), consequence generation
  - Test: Integration with World, PhysicsWorld, NavMesh
- `weave_portals.rs` (95 lines): Portal system for weaving
  - Test: Portal creation, linking, traversal, stability
  - Test: Multi-portal networks, one-way portals, cooldowns
- `cutscenes.rs` (40 lines): Cutscene playback
  - Test: Timeline progression, camera control, event triggers
- `quests.rs` (24 lines): Quest tracking
  - Test: Quest activation, objective completion, rewards

**Phase 2: Weak Systems Push** (160 lines, 25-30 tests, 1 hour):
- `dialogue.rs` (126 lines @ 29.37%): Need +76 lines coverage
  - Test: Dialogue tree traversal, branching choices, state persistence
  - Test: NPC responses, player options, conversation end conditions
- `stats.rs` (34 lines @ 44.12%): Need +16 lines coverage
  - Test: Stat calculations, modifiers, min/max clamping
  - Test: Stat persistence, damage/heal application

**Phase 3: Medium Systems Refinement** (141 lines, 15-20 tests, 45 min):
- `combat.rs` (73 lines @ 75.34%): Need +11 lines to 90%
  - Test: Edge cases, error paths, multi-target scenarios
- `crafting.rs` (43 lines @ 58.14%): Need +14 lines to 90%
  - Test: Recipe validation, material consumption, crafting failures
- `items.rs` (25 lines @ 56%): Need +9 lines to 90%
  - Test: Item stacking, durability, rarity tiers

**Total Gameplay Estimate**: 3-4 hours (90-110 tests, ~531 lines coverage gain)

---

## Part 2: Render Coverage Push (32.2% → 90%+)

### 2.1 Current Coverage Breakdown

**Source-Only Metrics** (31 files, 10,084 lines total):

| File | Lines | Covered | Coverage | Status | Priority |
|------|-------|---------|----------|--------|----------|
| **post.rs** | 107 | 107 | 100.00% | ⭐⭐⭐⭐⭐ | ✅ Done |
| **clustered.rs** | 171 | 167 | 97.66% | ⭐⭐⭐⭐⭐ | ✅ Done |
| **vertex_compression.rs** | 159 | 153 | 96.23% | ⭐⭐⭐⭐⭐ | ✅ Done |
| **terrain_material.rs** | 399 | 372 | 93.23% | ⭐⭐⭐⭐ | ✅ Done |
| **material_extended.rs** | 279 | 257 | 92.11% | ⭐⭐⭐⭐ | ✅ Done |
| **lod_generator.rs** | 274 | 247 | 90.15% | ⭐⭐⭐⭐ | ✅ Done |
| **camera.rs** | 308 | 255 | 82.79% | ⭐⭐⭐⭐ | Low |
| **animation.rs** | 343 | 268 | 78.13% | ⭐⭐ | Low |
| **terrain.rs** | 226 | 171 | 75.66% | ⭐⭐ | Low |
| **residency.rs** | 129 | 97 | 75.19% | ⭐⭐ | Low |
| **instancing.rs** | 295 | 200 | 67.80% | ⭐⭐ | Medium |
| **material.rs** | 423 | 261 | 61.70% | ⭐⭐ | Medium |
| **graph.rs** | 235 | 111 | 47.23% | ⚠️ | High |
| **culling.rs** | 377 | 139 | 36.87% | ⚠️ | High |
| **mesh_registry.rs** | 74 | 22 | 29.73% | ⚠️ | Medium |
| **mesh.rs** | 80 | 20 | 25.00% | ⚠️ | Medium |
| **environment.rs** | 615 | 123 | 20.00% | ⚠️ | **CRITICAL** |
| **gi/voxelization_pipeline.rs** | 350 | 54 | 15.43% | ⚠️ | High |
| **types.rs** | 177 | 27 | 15.25% | ⚠️ | Low |
| **ibl.rs** | 784 | 107 | 13.65% | ⚠️ | **CRITICAL** |
| **clustered_forward.rs** | 224 | 29 | 12.95% | ⚠️ | High |
| **gi/vxgi.rs** | 142 | 17 | 11.97% | ⚠️ | High |
| **renderer.rs** | **3431** | **43** | **1.25%** | ❌ | **BLOCKER** |
| **primitives.rs** | 132 | 0 | 0.00% | ❌ | Medium |
| **texture.rs** | 90 | 0 | 0.00% | ❌ | Medium |
| **overlay.rs** | 76 | 0 | 0.00% | ❌ | Low |
| **effects.rs** | 95 | 0 | 0.00% | ❌ | Low |
| **culling_node.rs** | 40 | 0 | 0.00% | ❌ | Low |
| **depth.rs** | 25 | 0 | 0.00% | ❌ | Low |
| **graph_adapter.rs** | 17 | 0 | 0.00% | ❌ | Low |
| **gi/mod.rs** | 7 | 0 | 0.00% | ❌ | Low |

**Summary**:
- ✅ **Excellent** (6 files, 1389 lines): Already >90%, no action needed
- ⭐⭐ **OK** (5 files, 1301 lines): Need 10-25pp push to 90%
- ⚠️ **Weak** (12 files, 3258 lines): Need 40-80pp gain (high effort)
- ❌ **Zero** (8 files, 482 lines): Need full coverage (medium effort)
- 🔥 **BLOCKER**: **renderer.rs** (3431 lines @ 1.25%) - 3,388 lines missed!

### 2.2 Implementation Strategy

**⚠️ CRITICAL DECISION POINT**: **renderer.rs Analysis Required**

**renderer.rs** is the single largest blocker:
- 3,431 lines (34% of all Render code)
- Only 43 lines covered (1.25%)
- 3,388 lines need coverage
- Represents ~50% of total Render effort

**Strategy**: Must analyze renderer.rs structure FIRST before proceeding:
1. Read file, identify modular components (init, resize, frame loop, resource management)
2. Determine if file should be **split** (renderer_core.rs, renderer_resources.rs, renderer_passes.rs)
3. Identify testable units vs integration-only code
4. Create targeted test plan (aim for 70-80% initially, not 90%)

**Phase 1: Quick Wins - Zero-Coverage Files** (482 lines, 30-40 tests, 1.5 hours):

**Batch 1A: Simple Systems** (257 lines, 15-20 tests):
- `primitives.rs` (132 lines): Geometry primitives (cube, sphere, plane)
  - Test: Vertex generation, index buffers, normals, UVs
- `texture.rs` (90 lines): Texture loading/management
  - Test: Format validation, mipmap generation, GPU upload
- `depth.rs` (25 lines): Depth buffer operations
  - Test: Buffer creation, clear operations, format selection
- `gi/mod.rs` (7 lines): GI module exports
  - Test: API surface, re-exports validation

**Batch 1B: Rendering Systems** (225 lines, 15-20 tests):
- `overlay.rs` (76 lines): UI overlay rendering
  - Test: Quad generation, transparency, layering
- `effects.rs` (95 lines): Visual effects (particles, post-FX triggers)
  - Test: Effect lifecycle, GPU resource management
- `culling_node.rs` (40 lines): Culling scene graph nodes
  - Test: Frustum culling, occlusion queries
- `graph_adapter.rs` (17 lines): Render graph integration
  - Test: Node registration, dependency resolution

**Phase 2: Weak Systems Push - High Priority** (2658 lines, 80-100 tests, 3-4 hours):

**Batch 2A: Core Rendering** (1376 lines, 40-50 tests):
- `ibl.rs` (784 lines @ 13.65%): IBL (Image-Based Lighting)
  - Need +677 lines coverage (76.35pp to 90%)
  - Test: Cubemap loading, SH coefficients, specular/diffuse lookups
- `environment.rs` (615 lines @ 20%): Environment rendering (skybox, atmosphere)
  - Need +492 lines coverage (70pp to 90%)
  - Test: Sky gradient, atmospheric scattering, day/night transitions

**Batch 2B: GI Systems** (492 lines, 25-30 tests):
- `gi/voxelization_pipeline.rs` (350 lines @ 15.43%): Voxel GI pipeline
  - Need +296 lines coverage (74.57pp to 90%)
  - Test: Voxel grid creation, conservative rasterization, light injection
- `gi/vxgi.rs` (142 lines @ 11.97%): VXGI (Voxel Global Illumination)
  - Need +125 lines coverage (78.03pp to 90%)
  - Test: Cone tracing, indirect lighting, temporal filtering

**Batch 2C: Rendering Infrastructure** (790 lines, 25-30 tests):
- `clustered_forward.rs` (224 lines @ 12.95%): Clustered forward+ rendering
  - Need +195 lines coverage (77.05pp to 90%)
  - Test: Cluster assignment, light culling, tile-based shading
- `culling.rs` (377 lines @ 36.87%): Frustum and occlusion culling
  - Need +238 lines coverage (53.13pp to 90%)
  - Test: View frustum extraction, AABB tests, hierarchical culling
- `graph.rs` (235 lines @ 47.23%): Render graph framework
  - Need +124 lines coverage (42.77pp to 90%)
  - Test: Pass ordering, resource tracking, barrier insertion
- `mesh.rs` (80 lines @ 25%) + `mesh_registry.rs` (74 lines @ 29.73%): Mesh management
  - Need +112 lines combined (154 lines total)
  - Test: Mesh loading, LOD selection, GPU buffer management

**Phase 3: Medium Systems Refinement** (1301 lines, 25-30 tests, 1.5 hours):
- `instancing.rs` (295 lines @ 67.80%): Need +66 lines to 90%
- `material.rs` (423 lines @ 61.70%): Need +120 lines to 90%
- `residency.rs` (129 lines @ 75.19%): Need +19 lines to 90%
- `terrain.rs` (226 lines @ 75.66%): Need +32 lines to 90%
- `animation.rs` (343 lines @ 78.13%): Need +41 lines to 90%
- `camera.rs` (308 lines @ 82.79%): Need +22 lines to 90%

**Phase 4: THE BIG ONE - renderer.rs** (3431 lines @ 1.25%, 80-100 tests, 3-4 hours):

**⚠️ THIS IS THE CRITICAL PATH - REQUIRES ARCHITECTURAL ANALYSIS**

**Expected Challenges**:
1. **Size**: 3,431 lines is larger than some entire crates
2. **Complexity**: Core rendering engine, many interdependencies
3. **Integration**: May require mock GPU, mock wgpu resources
4. **Coverage Goal**: Aim for **70-80%**, not 90% (acceptable for monolithic renderer)

**Proposed Approach** (MUST VALIDATE WITH CODE ANALYSIS):
1. **Read renderer.rs structure** (identify modules/sections)
2. **Identify testable units**:
   - Renderer initialization (device, queue, surface setup)
   - Window resize handling
   - Frame preparation (command buffer, render passes)
   - Resource management (texture cache, buffer pools)
   - Pipeline state management (shader compilation, PSO cache)
3. **Create modular tests**:
   - Mock wgpu::Device, wgpu::Queue for unit tests
   - Test each major method in isolation
   - Integration tests for full frame rendering (may be slow)
4. **Incremental validation**:
   - Target 50% first (critical paths)
   - Then 70% (error handling)
   - Stretch to 80% if time allows

**Alternative Strategy** (if renderer.rs is unmockable):
- **Refactor renderer.rs** into smaller modules first:
  - `renderer_core.rs` (init, frame loop) - 800 lines
  - `renderer_resources.rs` (textures, buffers, pipelines) - 1200 lines
  - `renderer_passes.rs` (depth, shadow, forward, post) - 1400 lines
  - `renderer_utils.rs` (helpers, conversions) - 31 lines (lib.rs style)
- Then test each module to 90% (easier than monolith)

**Total Render Estimate**: 5-8 hours (150-200 tests, ~5,829 lines coverage gain)

---

## Part 3: Implementation Timeline

**Recommended Order** (optimize for early wins and parallelization):

### Day 1: Gameplay Complete (3-4 hours)
- ✅ Hour 1: Gameplay Batch 1A (simple zero-coverage files)
- ✅ Hour 2-3: Gameplay Batch 1B (complex systems: weaving, portals, cutscenes, quests)
- ✅ Hour 3-4: Gameplay Phase 2-3 (weak systems + medium refinement)
- 🎯 **Milestone**: Gameplay 90%+ achieved

### Day 2: Render Quick Wins (2-3 hours)
- ✅ Hour 1: Render Phase 1 (zero-coverage files: primitives, texture, overlay, effects, etc.)
- ✅ Hour 2: Render Phase 3 (medium systems refinement: instancing, material, camera, etc.)
- 🎯 **Milestone**: Render 50%+ achieved (before tackling critical systems)

### Day 3: Render Critical Systems (3-4 hours)
- ⚠️ Hour 1: **renderer.rs analysis** (read structure, plan approach)
- ⚠️ Hour 2-3: Render Phase 2A-B (ibl.rs, environment.rs, GI systems)
- ⚠️ Hour 3-4: Render Phase 2C (clustered_forward, culling, graph, mesh systems)
- 🎯 **Milestone**: Render 70%+ achieved (without renderer.rs)

### Day 4: The Big Push - renderer.rs (3-4 hours)
- 🔥 Hour 1: renderer.rs modular tests (init, resize, resource management)
- 🔥 Hour 2: renderer.rs pipeline tests (shader compilation, PSO cache)
- 🔥 Hour 3: renderer.rs frame rendering tests (command buffer, render passes)
- 🔥 Hour 4: renderer.rs integration tests + validation
- 🎯 **Milestone**: Render 90%+ achieved (or 85% if renderer.rs unmockable)

### Day 5: Validation & Documentation (1-2 hours)
- ✅ Re-measure Gameplay and Render coverage with llvm-cov
- ✅ Verify 90%+ thresholds achieved
- ✅ Update MASTER_COVERAGE_REPORT.md v1.13 → v1.14
- ✅ Document achievements, test counts, coverage progression
- 🎯 **Milestone**: P1-B complete, documentation updated

**Total Timeline**: 12-16 hours (3-4 days @ 4h/day)

---

## Part 4: Success Criteria

**Gameplay Success** (90%+ target):
- ✅ Coverage: 51.1% → **90%+** (+38.9pp, ~422 lines)
- ✅ Tests: 15 → **90-110 tests** (+75-95 tests)
- ✅ Zero files: 8 → **0** (all files have tests)
- ✅ Weak files: dialogue.rs, stats.rs → **>90%**

**Render Success** (90%+ target OR 85%+ with justification):
- ✅ Coverage: 32.2% → **90%+** (+57.8pp, ~5,829 lines)
  - **Acceptable**: 85%+ if renderer.rs unmockable (document limitation)
- ✅ Tests: 127 → **277-327 tests** (+150-200 tests)
- ✅ Zero files: 8 → **0** (all files have tests)
- ✅ Critical systems: ibl.rs, environment.rs, GI → **>90%**
- ⚠️ renderer.rs: **70-80%** minimum (monolith exception)

**P1-B Impact** (Overall):
- ✅ P1-B Average: 37.05% → **60%+** (Gameplay + Render + Terrain 77.39% + Scene 0%)
  - With Gameplay 90% + Render 90%: (77.39 + 90 + 90 + 0) / 4 = **64.35%**
  - With Gameplay 90% + Render 85%: (77.39 + 90 + 85 + 0) / 4 = **63.10%**
- ✅ Priority Actions: #10-11 marked COMPLETE (Gameplay + Render 90%)

**Documentation Update** (v1.14):
- ✅ Updated P1-B table with new Gameplay and Render metrics
- ✅ Updated gap analyses with "EXCEEDS TARGET" status
- ✅ Added new Priority Actions for remaining crates (if needed)
- ✅ Documented test implementation approach and learnings

---

## Part 5: Risk Mitigation

**Risk 1: renderer.rs Unmockable** (HIGH)
- **Impact**: Cannot achieve 90% Render coverage (renderer.rs is 34% of code)
- **Mitigation 1**: Refactor renderer.rs into smaller modules first (2-3h overhead)
- **Mitigation 2**: Accept 70-80% renderer.rs coverage, document limitation
- **Mitigation 3**: Create integration tests with real wgpu::Device (slower but valid)
- **Acceptance Criteria**: If renderer.rs reaches 70-80%, Render total can be 85%+

**Risk 2: Weaving System Complexity** (MEDIUM)
- **Impact**: weaving.rs (103 lines) may require complex World/Physics setup
- **Mitigation**: Use existing test utilities from `tests.rs` (setup_world_with_companion)
- **Fallback**: Mock World/PhysicsWorld if integration too complex

**Risk 3: GI Systems Require GPU** (MEDIUM)
- **Impact**: gi/voxelization_pipeline.rs, gi/vxgi.rs may need real GPU
- **Mitigation**: Use `#[cfg(all(test, feature = "gpu-tests"))]` like skinning tests
- **Fallback**: Test only CPU-side logic (voxel grid creation, not rendering)

**Risk 4: Time Overrun** (MEDIUM)
- **Impact**: 12-16h estimate may be optimistic for 240+ new tests
- **Mitigation**: Prioritize critical systems (weaving, renderer.rs, ibl.rs, environment.rs)
- **Fallback**: Accept 85-90% instead of 90%+, document remaining gaps

**Risk 5: Test Brittleness** (LOW)
- **Impact**: Graphics tests may be fragile (floating point, GPU variance)
- **Mitigation**: Use `approx::assert_relative_eq!` for float comparisons
- **Best Practice**: Test invariants (vertex count, buffer sizes) over exact values

---

## Part 6: Post-Implementation Validation

**Validation Checklist**:

1. ✅ **Re-measure Gameplay**:
   ```powershell
   cargo llvm-cov clean -p astraweave-gameplay
   cargo llvm-cov test -p astraweave-gameplay --lib --no-fail-fast
   cargo llvm-cov report | Select-String "astraweave-gameplay\\src\\" | Where-Object { $_ -notmatch "test" }
   ```
   - Calculate source-only coverage (should be 90%+)
   - Verify all files have >80% coverage (none at 0%)

2. ✅ **Re-measure Render**:
   ```powershell
   cargo llvm-cov clean -p astraweave-render
   cargo llvm-cov test -p astraweave-render --lib --no-fail-fast
   cargo llvm-cov report | Select-String "astraweave-render\\src\\" | Where-Object { $_ -notmatch "test" }
   ```
   - Calculate source-only coverage (should be 85-90%+)
   - Verify renderer.rs >70%, all other files >80%

3. ✅ **Verify Test Counts**:
   ```powershell
   cargo test -p astraweave-gameplay --lib -- --list | Measure-Object -Line
   cargo test -p astraweave-render --lib -- --list | Measure-Object -Line
   ```
   - Gameplay: 15 → 90-110 tests (+75-95)
   - Render: 127 → 277-327 tests (+150-200)

4. ✅ **Run Full Test Suite**:
   ```powershell
   cargo test -p astraweave-gameplay --lib
   cargo test -p astraweave-render --lib
   ```
   - All tests pass (0 failures)
   - No warnings (clean build)

5. ✅ **Update MASTER_COVERAGE_REPORT.md**:
   - Version 1.13 → 1.14
   - Update P1-B table (Gameplay + Render metrics)
   - Update gap analyses (mark as EXCEEDS TARGET)
   - Update Priority Actions (mark #10-11 COMPLETE)
   - Add revision history entry

---

## Part 7: Next Steps After 90%+

**If Gameplay + Render Reach 90%**:
- ✅ P1-B Average: 64.35% (excellent progress!)
- 🎯 **Next Target**: Scene test refactoring (fix llvm-cov bug, move tests to `tests/` dir)
- 🎯 **Optional**: Terrain 80% push (+2.61pp from 77.39%, ~168 lines)

**If Only Gameplay Reaches 90%** (Render blocked):
- ⚠️ Document renderer.rs blocker in MASTER_COVERAGE_REPORT.md
- 🎯 Consider architectural refactor (split renderer.rs into modules)
- 🎯 Proceed with Scene test refactoring while planning Render strategy

**If Both Stall Below 90%**:
- ⚠️ Re-assess effort estimates (may need 16-20h instead of 12-16h)
- 🎯 Prioritize Gameplay first (smaller, faster wins)
- 🎯 Document blockers and request guidance

---

## Appendix A: Test Template Examples

### Gameplay Test Template (Simple Function)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_island_room_triangle_count() {
        let triangles = generate_island_room();
        assert_eq!(triangles.len(), 6, "Island room should have 6 triangles (2 floor + 2 ramp + 2 plateau)");
    }

    #[test]
    fn test_generate_island_room_floor_vertices() {
        let triangles = generate_island_room();
        let floor_tri = &triangles[0];
        assert_eq!(floor_tri.a, vec3(-4.0, 0.0, -4.0));
        assert_eq!(floor_tri.b, vec3(4.0, 0.0, -4.0));
        assert_eq!(floor_tri.c, vec3(4.0, 0.0, 4.0));
    }
}
```

### Gameplay Test Template (Complex System)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::World;
    use astraweave_physics::PhysicsWorld;

    fn setup_test_world() -> (World, PhysicsWorld) {
        let mut world = World::new();
        let mut physics = PhysicsWorld::new();
        // ... setup code ...
        (world, physics)
    }

    #[test]
    fn test_apply_weave_op_reinforce_path() {
        let (mut world, mut physics) = setup_test_world();
        let nav_src = vec![];
        let mut budget = WeaveBudget {
            terrain_edits: 5,
            weather_ops: 0,
        };
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(0.0, 0.0, 0.0),
            b: None,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(&mut world, &mut physics, &nav_src, &mut budget, &op, &mut logger);

        assert!(result.is_ok(), "ReinforcePath should succeed");
        assert_eq!(budget.terrain_edits, 4, "Should consume 1 terrain edit");
    }

    #[test]
    fn test_apply_weave_op_no_budget() {
        let (mut world, mut physics) = setup_test_world();
        let nav_src = vec![];
        let mut budget = WeaveBudget {
            terrain_edits: 0, // No budget!
            weather_ops: 0,
        };
        let op = WeaveOp {
            kind: WeaveOpKind::ReinforcePath,
            a: vec3(0.0, 0.0, 0.0),
            b: None,
        };
        let mut log_output = Vec::new();
        let mut logger = |msg: String| log_output.push(msg);

        let result = apply_weave_op(&mut world, &mut physics, &nav_src, &mut budget, &op, &mut logger);

        assert!(result.is_err(), "Should fail with no budget");
        assert!(result.unwrap_err().to_string().contains("No terrain budget"));
    }
}
```

### Render Test Template (Graphics System)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_vertex_size() {
        let compressed_size = std::mem::size_of::<CompressedVertex>();
        let original_size = std::mem::size_of::<Vertex>();
        
        assert_eq!(compressed_size, 16, "CompressedVertex should be 16 bytes (4x u32)");
        assert!(compressed_size < original_size, "Compression should save memory");
        
        let savings_pct = ((original_size - compressed_size) as f32 / original_size as f32) * 100.0;
        assert!(savings_pct > 30.0, "Should save >30% memory (actual: {:.1}%)", savings_pct);
    }

    #[test]
    fn test_octahedral_normal_encoding_roundtrip() {
        let normals = vec![
            Vec3::Y,              // Up
            Vec3::NEG_Y,          // Down
            Vec3::X,              // Right
            Vec3::new(0.5, 0.5, 0.5).normalize(), // Diagonal
        ];

        for normal in normals {
            let encoded = encode_octahedral_normal(normal);
            let decoded = decode_octahedral_normal(encoded);
            
            let error = (normal - decoded).length();
            assert!(error < 0.01, "Roundtrip error too high: {} (normal: {:?})", error, normal);
        }
    }
}
```

---

## Appendix B: llvm-cov Measurement Commands

```powershell
# Gameplay Coverage (Source-Only)
cargo llvm-cov clean -p astraweave-gameplay
cargo llvm-cov test -p astraweave-gameplay --lib --no-fail-fast
cargo llvm-cov report | Select-String "astraweave-gameplay\\src\\" | Where-Object { $_ -notmatch "test" }

# Render Coverage (Source-Only)
cargo llvm-cov clean -p astraweave-render
cargo llvm-cov test -p astraweave-render --lib --no-fail-fast
cargo llvm-cov report | Select-String "astraweave-render\\src\\" | Where-Object { $_ -notmatch "test" }

# Calculate Coverage (PowerShell)
$files = @(
    @{Name='file.rs'; Lines=100; Covered=80}
    # ... add all files ...
)
$total = ($files | Measure-Object -Property Lines -Sum).Sum
$covered = ($files | Measure-Object -Property Covered -Sum).Sum
$pct = [math]::Round(($covered / $total) * 100, 2)
Write-Host "Coverage: $pct%"
```

---

**END OF PLAN**

**Status**: ✅ Ready for implementation  
**Next Action**: Begin with Gameplay Batch 1A (simple zero-coverage files)  
**Expected Completion**: 3-5 days @ 3-4 hours/day  
**Success Metric**: Gameplay 90%+, Render 85-90%+, P1-B Average 60%+
