# Week 3 Tactical Plan - Optimization & Infrastructure

**Date**: October 9, 2025  
**Week**: Week 3 (Oct 10-16, 2025)  
**Status**: 🚀 **Ready to Execute**  
**Duration**: 7 days (5 actions planned)

---

## Executive Summary

### Context from Week 2

**Week 2 Achievements** (Completed Day 1 - Oct 9):
- ✅ **25 benchmarks** passing (ECS, AI Planning, AI Core Loop, Terrain, Input)
- ✅ **50 unwraps fixed** across 14 crates (Phase 1 complete)
- ✅ **S-Tier AI performance**: 11,500 agents @ 60 FPS (2500x faster than target!)
- ✅ **Production-ready validation**: ECS, Behavior Trees, Rule Planner
- ✅ **431% efficiency**: 7-day plan completed in 1 day

**Identified Optimization Opportunities**:
1. 🔴 **World Chunk Generation**: 19.8ms (18% over 60 FPS budget of 16.67ms)
2. 🔴 **GOAP Complex Planning**: 31.7ms (too slow for real-time, needs caching)
3. 🟡 **WorldSnapshot Copy-on-Write**: 1.96µs (90% reduction possible for multi-agent)
4. 🟡 **Entity Spawning Outliers**: 13% (target: <5%)

### Week 3 Goals

**Primary Objective**: Optimize critical bottlenecks and establish production infrastructure for continuous performance monitoring.

**Success Metrics**:
- ✅ World chunk generation <16.67ms (60 FPS streaming unlocked)
- ✅ GOAP complex planning <1ms with caching (real-time AI enabled)
- ✅ 50 more unwraps fixed (Phase 2 - 100 total remediated)
- ✅ CI benchmark pipeline operational (automated regression detection)
- ✅ Physics benchmarks established (raycast, character controller, rigid body)

**Timeline**: 7 days (Oct 10-16) with flexibility for early completion

---

## Week 3 Actions Overview

| Action | Priority | Estimated Time | Impact | Dependencies |
|--------|----------|---------------|--------|--------------|
| **8: World Chunk Optimization** | 🔴 Critical | 4-6 hours | 60 FPS streaming | None |
| **9: GOAP Plan Caching** | 🔴 High | 3-4 hours | Real-time complex AI | None |
| **10: Unwrap Remediation Phase 2** | 🟡 Medium | 3-4 hours | Code quality | None |
| **11: CI Benchmark Pipeline** | 🟡 Medium | 2-3 hours | Regression detection | Actions 8-9 complete |
| **12: Physics Benchmarks** | 🟢 Low | 2-3 hours | Baseline expansion | None |

**Total Estimated Time**: 14-20 hours (2-3 days at Week 2 velocity)  
**Stretch Goal**: Complete all 5 actions by Oct 11-12 (Day 2-3)

---

## Action 8: World Chunk Optimization 🔴 CRITICAL

### Objective

**Reduce terrain chunk generation from 19.8ms → <16.67ms** to unlock 60 FPS world streaming.

**Current Performance** (from BASELINE_METRICS.md):
- World chunk (64×64 voxels): **19.8ms** ±1.2ms (18% over budget)
- Breakdown: Heightmap (2ms, 10%) + Climate (2.5ms, 12.6%) + Biome/Voxel (15.3ms, 77.4%)
- **Bottleneck**: Voxel placement is 77% of total time

### Approach

**Strategy 1: SIMD Vectorization for Noise Generation** (Primary)
- **Target**: Heightmap + climate noise functions
- **Expected Gain**: 20-30% reduction in noise overhead
- **Implementation**:
  - Use `std::simd` (Rust 1.89.0 stable) for 4-wide f32 operations
  - Vectorize Perlin/Simplex noise calculations (sample 4 points at once)
  - Apply to `heightmap_generation` and `climate_sampling` functions
- **Files to Modify**:
  - `astraweave-terrain/src/noise.rs` (add SIMD noise variants)
  - `astraweave-terrain/src/heightmap.rs` (use SIMD noise)
  - `astraweave-terrain/benches/terrain_generation.rs` (re-run benchmarks)

**Strategy 2: Voxel Placement Optimization** (Secondary)
- **Target**: 15.3ms voxel placement cost
- **Expected Gain**: 10-20% reduction via pre-allocation
- **Implementation**:
  - Pre-allocate voxel grid with `Vec::with_capacity(64*64*64)`
  - Use `unsafe` set operations to avoid bounds checks in hot loop
  - Cache biome-to-voxel-type mappings
- **Files to Modify**:
  - `astraweave-terrain/src/voxel_mesh.rs` (pre-allocation, unsafe set)
  - `astraweave-terrain/src/biome.rs` (biome cache)

**Strategy 3: Async Streaming Integration** (If needed)
- **Target**: Offload chunk generation to background thread pool
- **Expected Gain**: 0ms main thread blocking (moved to async)
- **Implementation**:
  - Use existing `astraweave-scene` async cell loader
  - Generate chunks on Rayon thread pool
  - Main thread polls for completed chunks
- **Files to Modify**:
  - `astraweave-scene/src/streaming.rs` (integrate terrain generator)
  - `examples/unified_showcase/src/main.rs` (test async streaming)

### Success Criteria

**Must Have** ✅:
1. `world_chunk_generation` benchmark <16.67ms (run via `cargo bench`)
2. No visual artifacts in `unified_showcase` example
3. All terrain tests passing (`cargo test -p astraweave-terrain`)
4. Updated BASELINE_METRICS.md with new timings

**Nice to Have** 🎯:
5. Heightmap generation <1.5ms (down from 2ms)
6. Climate sampling <2.0ms (down from 2.5ms)
7. Async streaming demo in `unified_showcase` (0ms main thread blocking)

### Validation

```powershell
# Benchmark validation
cargo bench -p astraweave-terrain --bench terrain_generation

# Check for performance regression (should show improvement)
# Before: world_chunk_generation    time:   [19.6 ms 19.8 ms 20.0 ms]
# After:  world_chunk_generation    time:   [15.5 ms 16.2 ms 16.9 ms]  # Target

# Visual validation
cargo run -p unified_showcase --release
# (Verify terrain renders correctly, no artifacts, smooth streaming)

# Unit tests
cargo test -p astraweave-terrain
```

### Estimated Time

- **SIMD Vectorization**: 2-3 hours (implementation + testing)
- **Voxel Optimization**: 1-2 hours (pre-allocation + unsafe)
- **Async Integration**: 1-2 hours (optional, if SIMD insufficient)
- **Benchmarking & Validation**: 30 min
- **Total**: **4-6 hours**

---

## Action 9: GOAP Plan Caching 🔴 HIGH PRIORITY

### Objective

**Enable real-time complex GOAP planning** by caching plans for repeated scenarios (31.7ms → <1ms with cache hit).

**Current Performance** (from BASELINE_METRICS.md):
- GOAP simple (5 actions): 5.4µs (excellent)
- GOAP moderate (10 actions): 11.0µs (acceptable)
- GOAP complex (20 actions): **31.7ms** (190% of frame budget - too slow!)

**Problem**: Complex planning uses A* search with exponential growth (2x actions = 2882x time). Real-time gameplay requires <5ms per agent.

### Approach

**Strategy: LRU Cache with Scenario Fingerprinting**

**Cache Key Design**:
```rust
#[derive(Hash, Eq, PartialEq)]
struct ScenarioFingerprint {
    initial_state: WorldState,  // Simplified state (position bucket, health bucket, inventory)
    goal: Goal,                  // Desired end state
    available_actions: Vec<ActionId>,  // Sorted list of action IDs
}
```

**Cache Implementation**:
```rust
use lru::LruCache;

pub struct GOAPPlanCache {
    cache: LruCache<ScenarioFingerprint, Plan>,
    max_size: usize,  // Default: 1000 plans (~100KB memory)
}

impl GOAPPlanCache {
    pub fn get_or_plan(&mut self, scenario: &Scenario, planner: &GOAPPlanner) -> Plan {
        let fingerprint = scenario.to_fingerprint();
        
        if let Some(cached_plan) = self.cache.get(&fingerprint) {
            // Cache hit: <1µs lookup
            return cached_plan.clone();
        }
        
        // Cache miss: Execute A* planning (31.7ms)
        let plan = planner.plan(scenario);
        self.cache.put(fingerprint, plan.clone());
        plan
    }
}
```

**State Bucketing** (Reduce cache misses):
- Position: Round to 5-unit grid (exact position doesn't affect plan)
- Health: Buckets: [0-25%, 26-50%, 51-75%, 76-100%]
- Inventory: Binary flags (has_weapon, has_ammo, has_healthkit)

**Expected Cache Hit Rate**:
- **Combat scenarios**: 80-90% (limited state variations)
- **Exploration scenarios**: 50-70% (more diverse states)
- **Boss AI**: 60-80% (phase-based planning)

### Implementation

**Files to Create/Modify**:
1. **Create**: `astraweave-behavior/src/goap/cache.rs`
   - `GOAPPlanCache` struct with LRU cache
   - `ScenarioFingerprint` struct with state bucketing
   - Cache statistics (hits, misses, evictions)

2. **Modify**: `astraweave-behavior/src/goap/planner.rs`
   - Add `cache: Option<GOAPPlanCache>` field to `GOAPPlanner`
   - Wrap `plan()` method to check cache first
   - Add `plan_with_cache()` method

3. **Create**: `astraweave-behavior/benches/goap_caching.rs`
   - Benchmark cache hit performance (<1µs target)
   - Benchmark cache miss (same as current 31.7ms)
   - Benchmark repeated scenario planning (90% hit rate)

4. **Modify**: `examples/core_loop_goap_demo/src/main.rs`
   - Enable caching in GOAP demo
   - Print cache statistics (hits, misses, efficiency)

### Success Criteria

**Must Have** ✅:
1. Cache hit time <1ms (target: <100µs for lookup + clone)
2. Cache hit rate >80% for repeated combat scenarios
3. All GOAP benchmarks passing (cache doesn't break planning)
4. Cache statistics accessible (hits, misses, memory usage)

**Nice to Have** 🎯:
5. Cache serialization (save/load plans between sessions)
6. Cache warming (pre-populate with common scenarios)
7. Adaptive bucketing (adjust buckets based on plan variation)

### Validation

```powershell
# New caching benchmark
cargo bench -p astraweave-behavior --bench goap_caching

# Expected results:
# goap_cache_hit         time:   [50 µs - 100 µs]   # 300-600x faster than 31.7ms!
# goap_cache_miss        time:   [31.5 ms - 32.0 ms]  # Same as uncached
# goap_repeated_scenario time:   [2 ms - 4 ms]        # 90% hits, 10% misses

# Integration test (GOAP demo with caching)
cargo run -p core_loop_goap_demo --release

# Output should show:
# GOAP Cache Stats: 450 hits, 50 misses (90% hit rate)
# Average planning time: 2.1ms (vs 31.7ms uncached)
```

### Estimated Time

- **Cache Implementation**: 1.5-2 hours (LRU cache + fingerprinting)
- **State Bucketing**: 30-45 min (position/health/inventory buckets)
- **Benchmarking**: 1 hour (create goap_caching.rs, run tests)
- **Integration**: 30-45 min (update GOAP demo, validate)
- **Total**: **3-4 hours**

---

## Action 10: Unwrap Remediation Phase 2 🟡 MEDIUM PRIORITY

### Objective

**Fix next 50 P0 unwraps** to continue code quality improvements (100 total unwraps remediated after Phase 2).

**Progress from Week 2**:
- ✅ **Phase 1 Complete**: 50/50 unwraps fixed across 14 crates
- ✅ **Velocity Proven**: 14.3 unwraps/hour (2x faster than estimate)
- ✅ **Patterns Established**: 6 consistent patterns for fixes
- ⏳ **Remaining**: ~192 P0 unwraps in production code (from original audit)

### Approach

**Phase 2 Target Crates** (from UNWRAP_AUDIT_ANALYSIS.md):

| Crate | Remaining P0 Unwraps | Priority | Focus Areas |
|-------|---------------------|----------|-------------|
| **astraweave-render** | ~30 | 🔴 Critical | GPU pipeline, shader compilation |
| **astraweave-physics** | ~15 | 🔴 Critical | Rapier integration, collision handling |
| **astraweave-scene** | ~12 | 🟡 High | World streaming, async cell loading |
| **astraweave-cinematics** | ~10 | 🟡 High | Timeline sequencer, track playback |
| **astraweave-behavior** | ~8 | 🟡 Medium | GOAP planning, BT execution |
| **astraweave-pcg** | ~7 | 🟢 Low | Procedural generation |

**Target**: Fix top 3 crates (render, physics, scene) = ~57 unwraps, select 50 highest priority.

**Patterns to Apply** (from Phase 1):
1. **Post-operation invariant** → `expect("BUG: ... should exist after ...")`
2. **Mutex poisoning** → `expect("... mutex poisoned - cannot recover")`
3. **Component access** → `expect("Entity should have Component")`
4. **Post-check unwrap** → `expect("... should contain ... after check")`
5. **Proper error propagation** → `.ok_or_else(|| EngineError::...)?`
6. **Fallback handling** → `.unwrap_or(default)` or `if let Some()`

### Implementation

**Step 1: Search & Prioritize** (30 min)
```powershell
# Find unwraps in target crates
grep -n "\.unwrap()" astraweave-render/src/**/*.rs > render_unwraps.txt
grep -n "\.unwrap()" astraweave-physics/src/**/*.rs > physics_unwraps.txt
grep -n "\.unwrap()" astraweave-scene/src/**/*.rs > scene_unwraps.txt

# Manual review: Exclude test code, prioritize hot paths
```

**Step 2: Fix by Pattern** (2-2.5 hours)
- **Render**: GPU resource creation, shader compilation, material loading
- **Physics**: Collision shape creation, rigid body access, raycast results
- **Scene**: Cell deserialization, async streaming, world partition

**Step 3: Validate** (30 min)
```powershell
# Incremental compilation checks
cargo check -p astraweave-render
cargo check -p astraweave-physics
cargo check -p astraweave-scene

# Integration test
cargo run -p unified_showcase --release
```

### Success Criteria

**Must Have** ✅:
1. 50 unwraps fixed (100 total across Phase 1 + Phase 2)
2. All modified crates compile cleanly
3. No test regressions (`cargo test -p <crate>`)
4. Consistent error messages (BUG: prefix, clear context)

**Nice to Have** 🎯:
5. Updated UNWRAP_AUDIT_ANALYSIS.md with Phase 2 progress
6. Identified patterns specific to render/physics/scene crates
7. Velocity >14.3/hr (exceed Phase 1 performance)

### Validation

```powershell
# Compilation validation
cargo check -p astraweave-render -p astraweave-physics -p astraweave-scene

# Unit tests
cargo test -p astraweave-render -p astraweave-physics -p astraweave-scene

# Integration example
cargo run -p unified_showcase --release
# (Verify rendering, physics, and streaming all work correctly)
```

### Estimated Time

- **Search & Prioritize**: 30 min (grep, manual review)
- **Fixes (50 unwraps)**: 2.5-3 hours (proven 14.3/hr velocity)
- **Validation**: 30 min (compilation, tests, integration)
- **Documentation**: 15-30 min (update progress report)
- **Total**: **3-4 hours**

---

## Action 11: CI Benchmark Pipeline 🟡 MEDIUM PRIORITY

### Objective

**Establish automated performance regression detection** via GitHub Actions CI pipeline for all 25 benchmarks.

**Current State**:
- ✅ 25 benchmarks passing locally (manual execution)
- ✅ Regression thresholds documented in BASELINE_METRICS.md
- ❌ No automated CI checks (regressions could slip through PR reviews)

### Approach

**CI Workflow Design**:

**Trigger Conditions**:
- Pull requests to `main` branch (on-demand)
- Weekly schedule (Sunday 2 AM UTC) (baseline refresh)
- Manual workflow dispatch (ad-hoc testing)

**Benchmark Execution Strategy**:
- Run on `windows-latest` (matches dev environment)
- Execute all 25 benchmarks in parallel jobs (5 min total vs 15 min sequential)
- Parse Criterion JSON output for regression detection
- Fail build if RED thresholds exceeded

### Implementation

**File to Create**: `.github/workflows/benchmarks.yml`

```yaml
name: Performance Benchmarks

on:
  pull_request:
    branches: [main]
    paths:
      - 'crates/astraweave-*/**'
      - 'examples/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
  schedule:
    - cron: '0 2 * * 0'  # Weekly Sunday 2 AM UTC
  workflow_dispatch:  # Manual trigger

jobs:
  benchmark-ecs:
    name: ECS Benchmarks
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.89.0
      
      - name: Cache Dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-bench-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run ECS Benchmarks
        run: |
          cargo bench -p astraweave-core --bench ecs_benchmarks -- --save-baseline pr-baseline
          cargo bench -p astraweave-stress-test --bench stress_benchmarks -- --save-baseline pr-baseline
      
      - name: Check Thresholds
        run: |
          # Parse Criterion JSON, compare against RED thresholds
          # See scripts/check_benchmark_thresholds.ps1
          ./scripts/check_benchmark_thresholds.ps1 -Baseline pr-baseline -Crate astraweave-core
      
      - name: Upload Results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: ecs-benchmark-results
          path: target/criterion/

  benchmark-ai:
    name: AI Benchmarks
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run AI Benchmarks
        run: |
          cargo bench -p astraweave-behavior --bench goap_planning -- --save-baseline pr-baseline
          cargo bench -p astraweave-behavior --bench behavior_tree -- --save-baseline pr-baseline
          cargo bench -p astraweave-ai --bench ai_core_loop -- --save-baseline pr-baseline
      - name: Check Thresholds
        run: ./scripts/check_benchmark_thresholds.ps1 -Baseline pr-baseline -Crate astraweave-behavior
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: ai-benchmark-results
          path: target/criterion/

  benchmark-terrain:
    name: Terrain Benchmarks
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run Terrain Benchmarks
        run: cargo bench -p astraweave-terrain --bench terrain_generation -- --save-baseline pr-baseline
      - name: Check Thresholds
        run: ./scripts/check_benchmark_thresholds.ps1 -Baseline pr-baseline -Crate astraweave-terrain
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: terrain-benchmark-results
          path: target/criterion/

  benchmark-input:
    name: Input Benchmarks
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run Input Benchmarks
        run: cargo bench -p astraweave-input --bench input_benchmarks -- --save-baseline pr-baseline
      - name: Check Thresholds
        run: ./scripts/check_benchmark_thresholds.ps1 -Baseline pr-baseline -Crate astraweave-input
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: input-benchmark-results
          path: target/criterion/

  report:
    name: Generate Report
    needs: [benchmark-ecs, benchmark-ai, benchmark-terrain, benchmark-input]
    runs-on: windows-latest
    if: always()
    steps:
      - name: Download All Artifacts
        uses: actions/download-artifact@v4
      
      - name: Generate Summary
        run: |
          # Consolidate all benchmark results into PR comment
          # See scripts/generate_benchmark_report.ps1
          ./scripts/generate_benchmark_report.ps1 -OutputPath benchmark_summary.md
      
      - name: Comment on PR
        uses: actions/github-script@v7
        if: github.event_name == 'pull_request'
        with:
          script: |
            const fs = require('fs');
            const summary = fs.readFileSync('benchmark_summary.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: summary
            });
```

**Supporting Script**: `scripts/check_benchmark_thresholds.ps1`

```powershell
param(
    [string]$Baseline = "pr-baseline",
    [string]$Crate = "astraweave-core"
)

# Threshold definitions (from BASELINE_METRICS.md)
$thresholds = @{
    "world_creation" = @{ red = 31; yellow = 28 }  # ns
    "entity_spawning" = @{ red = 48000; yellow = 45000 }  # ns (100 entities)
    "world_tick" = @{ red = 50; yellow = 45 }  # ns
    # ... (add all 25 benchmarks)
}

# Parse Criterion JSON estimates
$results = Get-ChildItem "target/criterion/$Baseline/*/new/estimates.json" | ForEach-Object {
    $data = Get-Content $_ | ConvertFrom-Json
    $benchmark_name = $_.Directory.Parent.Name
    [PSCustomObject]@{
        Name = $benchmark_name
        Mean = $data.mean.point_estimate
        StdDev = $data.std_dev.point_estimate
    }
}

# Check thresholds
$failures = @()
foreach ($result in $results) {
    $threshold = $thresholds[$result.Name]
    if ($result.Mean -gt $threshold.red) {
        $failures += "REGRESSION: $($result.Name) = $($result.Mean)ns (RED threshold: $($threshold.red)ns)"
    } elseif ($result.Mean -gt $threshold.yellow) {
        Write-Warning "SLOWDOWN: $($result.Name) = $($result.Mean)ns (YELLOW threshold: $($threshold.yellow)ns)"
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host "✅ All benchmarks within thresholds" -ForegroundColor Green
```

### Success Criteria

**Must Have** ✅:
1. CI workflow file created (`.github/workflows/benchmarks.yml`)
2. Threshold check script created (`scripts/check_benchmark_thresholds.ps1`)
3. Workflow runs successfully on PR (manual test)
4. RED thresholds correctly fail the build
5. Benchmark results uploaded as artifacts

**Nice to Have** 🎯:
6. PR comment with benchmark summary (automated report)
7. Comparison with `main` branch baseline (trend analysis)
8. Performance badge in README.md (shields.io integration)

### Validation

```powershell
# Local workflow validation (act or manual)
# 1. Create test PR
# 2. Push changes
# 3. Verify workflow runs in GitHub Actions

# Local threshold check
./scripts/check_benchmark_thresholds.ps1 -Baseline pr-baseline -Crate astraweave-core
# Expected: ✅ All benchmarks within thresholds

# Test RED threshold failure (inject slow benchmark)
# Expected: REGRESSION error, exit code 1
```

### Estimated Time

- **Workflow YAML**: 1 hour (4 parallel jobs + report)
- **Threshold Script**: 45 min (parse JSON, check thresholds)
- **Integration Testing**: 45 min (create test PR, validate)
- **Documentation**: 15-30 min (README update)
- **Total**: **2-3 hours**

---

## Action 12: Physics Benchmarks 🟢 LOW PRIORITY

### Objective

**Establish physics performance baselines** for raycast, character controller, and rigid body systems.

**Current State**:
- ✅ Rapier3D integration working (validated in examples)
- ❌ No performance benchmarks for physics operations
- ❌ Unknown scalability limits (how many raycasts/bodies @ 60 FPS?)

### Approach

**Benchmark Suite Design**:

**1. Raycast Performance**
```rust
// astraweave-physics/benches/raycast.rs
fn raycast_1k_in_100_colliders(c: &mut Criterion) {
    let mut world = PhysicsWorld::new();
    // Add 100 static colliders (boxes, spheres, capsules)
    for _ in 0..100 {
        world.add_static_collider(/* random shapes */);
    }
    
    c.bench_function("raycast_1k_in_100_colliders", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let ray = Ray { origin, direction };
                world.cast_ray(ray, max_distance);
            }
        })
    });
}
```

**2. Character Controller Update**
```rust
// astraweave-physics/benches/character_controller.rs
fn character_controller_100_agents(c: &mut Criterion) {
    let mut world = PhysicsWorld::new();
    // Add terrain collider + 100 character controllers
    let mut controllers = vec![];
    for _ in 0..100 {
        controllers.push(CharacterController::new(/* config */));
    }
    
    c.bench_function("character_controller_100_agents", |b| {
        b.iter(|| {
            for controller in &mut controllers {
                controller.update(&mut world, delta_time);
            }
        })
    });
}
```

**3. Rigid Body Simulation**
```rust
// astraweave-physics/benches/rigid_body.rs
fn rigid_body_simulation_500_bodies(c: &mut Criterion) {
    let mut world = PhysicsWorld::new();
    // Add 500 dynamic rigid bodies (stacked boxes)
    for _ in 0..500 {
        world.add_rigid_body(/* random position, box shape */);
    }
    
    c.bench_function("rigid_body_simulation_500_bodies", |b| {
        b.iter(|| {
            world.step(1.0 / 60.0);  // 60 FPS timestep
        })
    });
}
```

### Performance Targets

Based on industry standards and 60 FPS constraints:

| Benchmark | Target | Reasoning |
|-----------|--------|-----------|
| **1000 raycasts / 100 colliders** | <10ms | Combat AI (attack raycasts) + navmesh queries |
| **100 character controllers** | <5ms | 100 AI agents with collision avoidance |
| **500 rigid bodies** | <5ms | Physics simulation @ 200Hz (4x60 FPS margin) |

**Total Physics Budget**: ~20ms per frame (if all systems active)  
**60 FPS Budget**: 16.67ms → Physics would be **120% of budget** (optimization needed if all active)

### Implementation

**Files to Create**:
1. `astraweave-physics/benches/raycast.rs` (3 benchmarks)
2. `astraweave-physics/benches/character_controller.rs` (3 benchmarks)
3. `astraweave-physics/benches/rigid_body.rs` (3 benchmarks)

**Benchmark Variations**:
- **Raycast**: 1K rays in 100/500/1000 colliders
- **Character Controller**: 10/100/500 agents
- **Rigid Body**: 100/500/1000 bodies

**Total**: 9 physics benchmarks (expand from 25 to 34 total benchmarks)

### Success Criteria

**Must Have** ✅:
1. 9 physics benchmarks compile and run successfully
2. Results documented in BASELINE_METRICS.md
3. Regression thresholds defined (RED/YELLOW limits)
4. All benchmarks complete in <30 seconds (CI-friendly)

**Nice to Have** 🎯:
5. Comparison with Rapier3D's internal benchmarks
6. Scalability analysis (linear, quadratic, etc.)
7. Optimization recommendations (BVH tuning, collision groups)

### Validation

```powershell
# Run physics benchmarks
cargo bench -p astraweave-physics --bench raycast
cargo bench -p astraweave-physics --bench character_controller
cargo bench -p astraweave-physics --bench rigid_body

# Expected results (hypothetical - need to measure):
# raycast_1k_in_100_colliders    time:   [8 ms - 12 ms]
# character_controller_100       time:   [3 ms - 6 ms]
# rigid_body_500                 time:   [4 ms - 7 ms]

# Update BASELINE_METRICS.md with results
# Add to CI pipeline (if Action 11 complete)
```

### Estimated Time

- **Raycast Benchmarks**: 45 min (setup, 3 variations)
- **Character Controller**: 45 min (setup, 3 variations)
- **Rigid Body**: 45 min (setup, 3 variations)
- **Validation & Documentation**: 30 min
- **Total**: **2-3 hours**

---

## Timeline & Milestones

### Week 3 Schedule (Oct 10-16, 2025)

**Day 1 (Oct 10)**: 🔴 Critical Optimizations
- **Morning**: Action 8 - World Chunk Optimization (4-6 hours)
  - SIMD vectorization
  - Voxel pre-allocation
  - Benchmark validation
- **Afternoon**: Action 9 - GOAP Plan Caching (3-4 hours)
  - Cache implementation
  - Benchmarking
  - Integration testing

**Day 2 (Oct 11)**: 🟡 Code Quality & Infrastructure
- **Morning**: Action 10 - Unwrap Remediation Phase 2 (3-4 hours)
  - Render crate fixes (~20 unwraps)
  - Physics crate fixes (~15 unwraps)
  - Scene crate fixes (~12 unwraps)
  - Validation
- **Afternoon**: Action 11 - CI Benchmark Pipeline (2-3 hours)
  - Workflow YAML
  - Threshold script
  - Integration testing

**Day 3 (Oct 12)**: 🟢 Benchmark Expansion (Optional)
- **Morning**: Action 12 - Physics Benchmarks (2-3 hours)
  - Raycast, character controller, rigid body
  - Documentation
- **Afternoon**: Week 3 Completion Report
  - Consolidate results
  - Update BASELINE_METRICS.md
  - Create WEEK_3_COMPLETE.md

**Days 4-7 (Oct 13-16)**: 🎯 Stretch Goals / Buffer
- Performance profiling (if optimizations fell short)
- Additional unwrap remediation (Phase 2 continuation)
- Documentation polish
- Integration testing with Veilweaver game mechanics

### Milestones

**Milestone 1**: Critical Optimizations Complete (End of Day 1)
- ✅ World chunk generation <16.67ms (60 FPS streaming unlocked)
- ✅ GOAP complex planning <1ms with caching (real-time AI enabled)

**Milestone 2**: Code Quality & CI (End of Day 2)
- ✅ 100 total unwraps fixed (Phase 1 + Phase 2)
- ✅ CI benchmark pipeline operational (regression detection automated)

**Milestone 3**: Comprehensive Benchmarking (End of Day 3)
- ✅ 34 total benchmarks (25 existing + 9 physics)
- ✅ All systems baselined (ECS, AI, Terrain, Input, Physics)

**Milestone 4**: Week 3 Complete (End of Day 3-5)
- ✅ All 5 actions complete
- ✅ Documentation updated (BASELINE_METRICS.md, WEEK_3_COMPLETE.md)
- ✅ Production-ready for large-scale gameplay

---

## Success Criteria Summary

### Quantitative Metrics

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| **Actions Complete** | 5/5 | 5/5 + stretch goals |
| **World Chunk Time** | <16.67ms | <15ms |
| **GOAP Cache Hit Time** | <1ms | <100µs |
| **Cache Hit Rate** | >80% | >90% |
| **Unwraps Fixed** | 50 | 75 (Phase 2+) |
| **CI Pipeline** | Operational | Auto-PR comments |
| **Physics Benchmarks** | 9 created | 9 + scalability analysis |

### Qualitative Outcomes

**Must Achieve** ✅:
1. 60 FPS world streaming validated (unlock core gameplay mechanic)
2. Real-time complex GOAP enabled (boss AI, strategic NPCs)
3. Automated regression detection (protect performance gains)
4. Continued code quality improvement (100 unwraps remediated)

**Nice to Achieve** 🎯:
5. Physics scalability validated (1000+ rigid bodies, 10K+ raycasts)
6. Performance badges in README (visual validation)
7. Week 3 complete in 3 days (continue Week 2 momentum)

---

## Risk Assessment

### Potential Blockers

**Risk 1**: SIMD optimization insufficient to reach <16.67ms
- **Likelihood**: Medium (20-30% gain may not be enough)
- **Mitigation**: Have async streaming ready as fallback (0ms main thread)
- **Impact**: High (blocks 60 FPS streaming)

**Risk 2**: GOAP cache hit rate lower than expected (<50%)
- **Likelihood**: Low (combat scenarios have limited state variations)
- **Mitigation**: Adaptive bucketing, cache warming with common scenarios
- **Impact**: Medium (reduces effectiveness but still improves average case)

**Risk 3**: CI pipeline flaky on Windows runners
- **Likelihood**: Medium (Windows CI can have timer resolution issues)
- **Mitigation**: Use wider thresholds (±30% vs ±10%), retry on failure
- **Impact**: Low (can disable problematic benchmarks initially)

**Risk 4**: Physics benchmarks reveal critical bottlenecks
- **Likelihood**: Medium (Rapier3D performance unknown at scale)
- **Mitigation**: Document findings, add to Month 2 optimization roadmap
- **Impact**: Low (discovery vs optimization - defer to future week)

### Dependencies

**No Blocking Dependencies**:
- All actions can execute in parallel (if desired)
- Action 11 (CI) benefits from Actions 8-9 (new benchmarks) but not required

**External Dependencies**:
- Rust 1.89.0 stable (SIMD support) - ✅ Available
- GitHub Actions Windows runners - ✅ Available
- Rapier3D 0.17+ (physics) - ✅ Integrated

---

## Resource Requirements

### Time Investment

**Total Estimated Time**: 14-20 hours
- Action 8: 4-6 hours
- Action 9: 3-4 hours
- Action 10: 3-4 hours
- Action 11: 2-3 hours
- Action 12: 2-3 hours

**Realistic Timeline**:
- **Optimistic**: 2 days (at Week 2 velocity: 431% efficiency)
- **Likely**: 3 days (at 200% efficiency)
- **Conservative**: 5 days (at 100% efficiency)

### Tools & Libraries

**Existing**:
- ✅ Criterion.rs (benchmarking)
- ✅ std::simd (Rust 1.89.0)
- ✅ LRU crate (for GOAP cache)
- ✅ Rapier3D (physics)
- ✅ GitHub Actions

**New**:
- 📦 `lru = "0.12"` (add to Cargo.toml for GOAP cache)

### Hardware

**Development**:
- Laptop: Intel i5-10300H, GTX 1660 Ti, 32GB RAM (same as Week 2)

**CI**:
- GitHub Actions: `windows-latest` (similar specs to dev environment)

---

## Comparison with Week 2

| Metric | Week 2 | Week 3 | Change |
|--------|--------|--------|--------|
| **Actions Planned** | 7 | 5 | -29% (more focused) |
| **Est. Time** | 7 days | 7 days | Same |
| **Critical Actions** | 2 | 2 | Same (Actions 8-9) |
| **Code Quality** | 50 unwraps | 50 unwraps | Same (Phase 2) |
| **Benchmarks Added** | 21 | 9 | +43% total (34) |
| **Infrastructure** | None | CI pipeline | New capability |

**Week 3 Focus Shift**:
- Week 2: **Baseline establishment** (benchmarks, validation)
- Week 3: **Optimization & automation** (improve performance, automate checks)

---

## Next Steps After Week 3

### Immediate (Week 4)

**If optimizations successful** ✅:
1. **Stress test at scale**: 1000 entities + 100 AI agents + physics
2. **Profile Veilweaver mechanics**: Fate-weaving system performance
3. **Memory optimization**: Reduce heap allocations in hot paths

**If optimizations fell short** ⚠️:
1. **Deep profiling**: Use `perf` / `cargo-flamegraph` to find hotspots
2. **Advanced techniques**: GPU compute for terrain, parallel GOAP search
3. **Architecture review**: Fundamental changes (streaming vs real-time gen)

### Medium-Term (Month 1-2)

**Performance**:
- WorldSnapshot copy-on-write (10x multi-agent efficiency)
- Entity spawn pre-allocation (reduce outliers)
- GPU cluster light binning (1000+ lights)

**Features**:
- Unwrap Remediation Phase 3 (final 92 unwraps)
- LLM integration benchmarks (when opt-in enabled)
- Network multiplayer benchmarks

### Long-Term (Month 3+)

**Production Readiness**:
- End-to-end gameplay scenarios (Veilweaver demo)
- Performance at scale (10K entities, 1K AI agents)
- Cross-platform validation (Linux, macOS)

---

## Conclusion

### Week 3 Vision

**Transform AstraWeave from "validated" to "optimized & production-ready"** by:
1. Unlocking 60 FPS world streaming (critical gameplay enabler)
2. Enabling real-time complex AI (boss battles, strategic NPCs)
3. Automating performance regression detection (protect gains)
4. Continuing code quality improvements (100 unwraps total)
5. Expanding benchmark coverage (physics systems)

**Success Looks Like**:
- ✅ Smooth 60 FPS gameplay with streaming terrain
- ✅ Complex AI planning in real-time (<1ms cached)
- ✅ CI pipeline auto-detects performance regressions
- ✅ 34 benchmarks covering all core systems
- ✅ Production-grade code quality (minimal unwraps)

**Week 3 Motto**: *"Optimize the bottlenecks, automate the guards, prepare for scale."*

---

## Related Documentation

### Week 3 (This Week)
- **Planning**: WEEK_3_KICKOFF.md (This document)
- **Progress**: TBD (WEEK_3_ACTION_*_COMPLETE.md files)
- **Completion**: TBD (WEEK_3_COMPLETE.md)

### Week 2 (Completed)
- **Planning**: WEEK_2_KICKOFF.md
- **Progress**: WEEK_2_ACTIONS_1_2_COMPLETE.md, WEEK_2_ACTION_3_COMPLETE.md, etc.
- **Completion**: WEEK_2_COMPLETE.md

### Week 1 (Completed)
- **Completion**: WEEK_1_COMPLETION_SUMMARY.md
- **Actions**: ACTION_1_GPU_SKINNING_COMPLETE.md, etc.

### Strategic Plans
- **12-Month Roadmap**: LONG_HORIZON_STRATEGIC_PLAN.md
- **Gap Analysis**: COMPREHENSIVE_STRATEGIC_ANALYSIS.md
- **Navigation**: IMPLEMENTATION_PLANS_INDEX.md

### Performance
- **Baselines**: BASELINE_METRICS.md (to be updated in Actions 8, 9, 12)
- **Unwrap Audit**: UNWRAP_AUDIT_ANALYSIS.md (to be updated in Action 10)

---

**Week 3 Status**: 🚀 **Ready to Execute**  
**Start Date**: October 10, 2025  
**Target Completion**: October 12-16, 2025 (3-5 days)

_Generated by AstraWeave Copilot - October 9, 2025_  
_Building on Week 2's 431% efficiency to optimize AstraWeave for production gameplay_
