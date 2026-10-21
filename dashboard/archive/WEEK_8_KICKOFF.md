# Week 8 Kickoff: Performance Optimization Sprint

**Date**: October 12, 2025  
**Duration**: 5 days (October 12-16, 2025)  
**Phase**: Phase B - Month 4 Week 8  
**Prerequisites**: ✅ Week 7 profiling instrumentation complete (28/31 points, 90.3%)  

---

## 🎯 Week 8 Goals

### Primary Objectives
1. **Capture Tracy Performance Baselines** (Day 1, 4-6h)
   - Run profiling_demo at 200, 500, 1000 entities
   - Export `.tracy` files for analysis
   - Identify top 10 hotspots (>5% frame time)

2. **Implement Performance Optimizations** (Days 2-4, 12-16h)
   - Target: Maintain 60 FPS at 500 entities (16.67ms budget)
   - Focus: Top 3 hotspots identified by Tracy
   - Validate: Re-run Tracy, compare to Week 7 baselines

3. **Regression Testing & Documentation** (Day 5, 4-6h)
   - Ensure no breakage in existing systems
   - Update performance baselines
   - Create completion report

### Success Criteria
- ✅ Tracy baselines captured for 3 configurations (200, 500, 1000 entities)
- ✅ Top 10 hotspots identified and prioritized
- ✅ At least 3 optimization targets addressed
- ✅ 60 FPS maintained at 500 entities (p95 latency <16.67ms)
- ✅ No regressions vs Week 7/Week 3 baselines
- ✅ Comprehensive documentation created

---

## 📅 Day-by-Day Breakdown

### **Day 1 (Oct 12): Tracy Baseline Capture** (4-6h)

#### Task 1.1: Setup Tracy Server (30 min)
**Actions**:
```powershell
# Download Tracy 0.11+ from GitHub releases
# https://github.com/wolfpld/tracy/releases/latest

# Extract to C:\Tools\Tracy (or your preferred location)
# Launch Tracy.exe
# Configure: Connection > Connect to localhost
```

**Validation**:
- Tracy window opens successfully
- Connection settings configured
- Ready to capture traces

---

#### Task 1.2: Capture Baseline Traces (2-3h)
**Configuration 1: Low Load (200 entities)**
```powershell
# Build profiling_demo in release mode
cargo build -p profiling_demo --features profiling --release

# Run with Tracy server active
cargo run -p profiling_demo --features profiling --release -- --entities 200

# Let run for 1000+ frames (stable performance)
# Tracy auto-captures during execution
# Save trace: File > Save Trace > baseline_200.tracy
```

**Configuration 2: Medium Load (500 entities)**
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 500

# Capture 1000+ frames
# Save trace: baseline_500.tracy
```

**Configuration 3: High Load (1000 entities)**
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000

# Capture 1000+ frames
# Save trace: baseline_1000.tracy
```

**Expected Outcomes**:
- 3 `.tracy` files saved
- Frame times recorded (avg, p50, p95, p99)
- Visual confirmation of frame consistency

---

#### Task 1.3: Analyze Profiling Data (1.5-2h)
**Tracy Analysis Workflow**:

1. **Statistics View** (`Statistics` menu):
   - Sort by "Self time" descending
   - Identify top 10 functions (>5% frame time)
   - Record CPU time per function

2. **Flame Graph** (`Flame graph` menu):
   - Visualize hierarchical call tree
   - Identify unexpected deep call stacks
   - Spot recursive calls or allocation hotspots

3. **Timeline View** (default):
   - Identify frame spikes (>16.67ms)
   - Check for stalling (long Present wait = GPU bottleneck)
   - Review plot trends (entity_count, draw_calls, cache_hits)

4. **Frame Statistics** (`Find zone` menu):
   - Average frame time per configuration
   - p95/p99 latency (frame time distribution)
   - Frame time variance (stability check)

**Data to Extract**:
```
Configuration: 200 entities
- Average frame time: _____ ms
- p95 frame time: _____ ms
- p99 frame time: _____ ms
- FPS (avg): _____

Top 10 Hotspots (>5% frame time):
1. Function: _____  | Self time: _____ ms (___%)
2. Function: _____  | Self time: _____ ms (___%)
...
10. Function: _____ | Self time: _____ ms (___%)

Repeat for 500 and 1000 entities
```

---

#### Task 1.4: Create Baseline Report (1-1.5h)
**File**: `PROFILING_BASELINE_WEEK_8.md`

**Report Structure**:
```markdown
# Week 8 Tracy Profiling Baselines

## System Specifications
- CPU: [Your CPU model]
- GPU: [Your GPU model]
- RAM: [Total RAM]
- OS: Windows 11 [version]
- Tracy Version: 0.11.x
- Rust Version: 1.89.0
- Build Config: --release --features profiling

## Frame Time Summary
| Configuration | Avg (ms) | p50 (ms) | p95 (ms) | p99 (ms) | Avg FPS | Notes |
|---------------|----------|----------|----------|----------|---------|-------|
| 200 entities  | X.XX     | X.XX     | X.XX     | X.XX     | XX      | Stable |
| 500 entities  | X.XX     | X.XX     | X.XX     | X.XX     | XX      | Target |
| 1000 entities | X.XX     | X.XX     | X.XX     | X.XX     | XX      | Stress |

## Hotspot Analysis (500 entities - target config)
### Top 10 Functions by Self Time
1. **Render::MainPass** - X.XX ms (XX%) - PBR shading, lighting
2. **Render::ShadowMaps** - X.XX ms (XX%) - Cascaded shadow rendering
3. **Physics::Rapier::pipeline** - X.XX ms (XX%) - Collision detection
...

## Subsystem Breakdown (500 entities)
| Subsystem | Total Time | % of Frame | Notes |
|-----------|-----------|------------|-------|
| **Rendering** | X.XX ms | XX% | Expected dominant |
| **Physics** | X.XX ms | XX% | Within budget |
| **AI** | X.XX ms | XX% | Cache hit rate: XX% |
| **ECS** | X.XX ms | XX% | Minimal overhead |

## Optimization Priorities (Week 8)
Based on Tracy data, prioritize optimizations for:
1. **[Top Hotspot]** - XX% frame time → Target: Reduce by 20-30%
2. **[Second Hotspot]** - XX% frame time → Target: Reduce by 15-20%
3. **[Third Hotspot]** - XX% frame time → Target: Reduce by 10-15%

## Regression Analysis vs Week 3 Baselines
| Metric | Week 3 | Week 8 Tracy | Delta | Status |
|--------|--------|--------------|-------|--------|
| GOAP cache hit rate | 97.9% | XX% | +/-X% | ✅/⚠️ |
| Physics tick (async) | 2.96 ms | X.XX ms | +/-X ms | ✅/⚠️ |
| Character move | 114 ns | X ns | +/-X ns | ✅/⚠️ |

## Recommendations
[Based on actual Tracy data, suggest optimization strategies]
```

**Deliverable**: Comprehensive baseline report ready for optimization planning

---

### **Days 2-4 (Oct 13-15): Performance Optimization** (12-16h)

**Note**: Actual optimization targets will be determined by Tracy data from Day 1. Below are **hypothetical scenarios** based on expected hotspots.

---

#### Scenario A: Rendering Dominates (>70% frame time)

**Expected Hotspots**:
1. `Render::MainPass` (50-75% frame time) - PBR shading
2. `Render::ShadowMaps` (6-18% frame time) - Cascaded shadows
3. `Render::Present` (Variable) - VSync wait / GPU sync

**Optimization Strategy 1: Draw Call Batching** (4-6h)
**Goal**: Reduce draw calls from current count to <10 per frame

**Implementation** (`astraweave-render/src/renderer.rs`):
```rust
// Current: Multiple draw calls per mesh type
// plane: 1 draw call
// spheres: 1 draw call (instanced)
// external: 1 draw call
// Total: 3+ draw calls

// Optimization: Material-based batching
pub struct MaterialBatch {
    material_id: u32,
    instances: Vec<InstanceRaw>,
    mesh_id: u64,
}

impl Renderer {
    fn batch_by_material(&self) -> Vec<MaterialBatch> {
        // Group instances by (material_id, mesh_id)
        // Result: Single draw call per unique material
        // Expected: 2-5 draw calls (vs 10-20 without batching)
    }
}
```

**Validation**:
- Tracy: Check `Render::draw_calls` plot (should be <10)
- Frame time: MainPass should reduce by 10-15%

---

**Optimization Strategy 2: Shadow Map Resolution Tuning** (2-3h)
**Goal**: Reduce shadow rendering cost by 20-30%

**Implementation** (`astraweave-render/src/renderer.rs`):
```rust
// Current: Both cascades at 1024×1024 (high quality)
const SHADOW_MAP_SIZE_NEAR: u32 = 1024;  // Cascade 0
const SHADOW_MAP_SIZE_FAR: u32 = 1024;   // Cascade 1

// Optimization: Reduce distant cascade resolution
const SHADOW_MAP_SIZE_NEAR: u32 = 1024;  // Keep high quality for near
const SHADOW_MAP_SIZE_FAR: u32 = 512;    // Reduce far cascade (4× fewer pixels)

// Expected savings: 25% reduction in shadow rendering time
// Visual impact: Minimal (distant shadows less noticeable)
```

**Validation**:
- Tracy: `Render::ShadowMaps` should reduce by 20-30%
- Visual check: Ensure acceptable shadow quality

---

**Optimization Strategy 3: Frustum Culling SIMD** (6-8h)
**Goal**: Reduce culling overhead by 50-70%

**Implementation** (`astraweave-render/src/culling.rs` - new file):
```rust
use std::arch::x86_64::*;

// Current: Scalar AABB-frustum test (1 entity at a time)
fn frustum_test_scalar(aabb: &AABB, planes: &[Plane; 6]) -> bool {
    for plane in planes {
        if aabb_plane_distance(aabb, plane) < 0.0 {
            return false; // Outside frustum
        }
    }
    true
}

// Optimization: SIMD AABB-frustum test (4 entities at once)
#[target_feature(enable = "avx2")]
unsafe fn frustum_test_simd_4(
    aabbs: &[AABB; 4],
    planes: &[Plane; 6]
) -> [bool; 4] {
    // Process 4 AABBs simultaneously using AVX2
    // Expected: 4× speedup vs scalar
}
```

**Validation**:
- Tracy: Culling overhead (within `Render::Frame`) should reduce by 50%+
- Benchmark: `cargo bench -p astraweave-render --bench culling_simd`

---

#### Scenario B: Physics Dominates (>20% frame time)

**Expected Hotspots**:
1. `Physics::Rapier::pipeline` (15-25% frame time) - Collision detection
2. `Physics::CharacterController::move` (2-5% frame time) - Many characters

**Optimization Strategy 1: Spatial Hashing for Broad Phase** (8-10h)
**Goal**: Reduce collision detection overhead by 30-50%

**Implementation** (`astraweave-physics/src/spatial_hash.rs` - new file):
```rust
pub struct SpatialHash {
    cell_size: f32,
    grid: HashMap<(i32, i32, i32), Vec<BodyId>>,
}

impl SpatialHash {
    pub fn query_potential_collisions(&self, aabb: &AABB) -> Vec<BodyId> {
        // Only test bodies in nearby grid cells
        // Expected: 10-100× fewer collision pairs than naive O(n²)
    }
}
```

**Validation**:
- Tracy: `Physics::Rapier::pipeline` should reduce by 30%+
- Benchmark: `cargo bench -p astraweave-physics --bench broad_phase`

---

**Optimization Strategy 2: Character Controller Batching** (4-6h)
**Goal**: SIMD raycast batching for multiple characters

**Implementation** (`astraweave-physics/src/lib.rs`):
```rust
// Current: 2 raycasts per character (obstacle + ground), processed sequentially
pub fn control_character(&mut self, id: BodyId, desired_move: Vec3, ...) {
    let obstacle_ray = Ray::new(pos, desired_move);
    let ground_ray = Ray::new(pos, Vec3::NEG_Y * 0.1);
    // Process 1 character at a time
}

// Optimization: Batch raycasts for all characters
pub fn control_characters_batch(&mut self, moves: &[(BodyId, Vec3)]) {
    // Collect all rays (2 per character)
    let rays: Vec<Ray> = moves.iter()
        .flat_map(|(id, mv)| [obstacle_ray, ground_ray])
        .collect();
    
    // Single Rapier call processes all rays
    let hits = self.query_pipeline.cast_ray_batch(&rays);
    
    // Apply results to all characters
    // Expected: 2-4× faster than sequential
}
```

**Validation**:
- Tracy: `Physics::CharacterController::move` should reduce by 50%+
- Benchmark: `cargo bench -p astraweave-physics --bench character_batch`

---

#### Scenario C: AI Dominates (>10% frame time)

**Expected Hotspots**:
1. `GOAP::Planner::plan` (8-12% frame time) - A* search (if cache disabled)
2. `GOAP::PlanCache::get` (1-2% frame time) - Cache misses

**Optimization Strategy 1: Cache Warming** (3-4h)
**Goal**: Reduce cache miss rate from 3% to <1%

**Implementation** (`astraweave-behavior/src/goap_cache.rs`):
```rust
impl PlanCache {
    pub fn warm_common_plans(&mut self) {
        // Pre-compute common plans at startup
        let common_goals = vec![
            Goal::AttackNearestEnemy,
            Goal::Retreat,
            Goal::Patrol,
            Goal::Defend,
        ];
        
        for goal in common_goals {
            for start_state in common_start_states() {
                if let Some(plan) = self.planner.plan(start_state, goal) {
                    self.insert(start_state, goal, plan);
                }
            }
        }
        // Expected: Cache hit rate 97.9% → 99%+
    }
}
```

**Validation**:
- Tracy: Check `GOAP::cache_hits` plot (should increase)
- Tracy: `GOAP::Planner::plan` should reduce by 50%+ (fewer cold searches)

---

**Optimization Strategy 2: Behavior Tree Early-Outs** (2-3h)
**Goal**: Reduce tree traversal depth by 30-40%

**Implementation** (`astraweave-behavior/src/behavior_tree.rs`):
```rust
// Current: Always traverse full tree depth
fn tick(&mut self, blackboard: &Blackboard) -> Status {
    match self {
        Node::Sequence(children) => {
            for child in children {
                match child.tick(blackboard) {
                    Status::Success => continue,
                    Status::Failure => return Status::Failure, // Early-out
                    Status::Running => return Status::Running,
                }
            }
            Status::Success
        }
    }
}

// Optimization: Add short-circuit conditions
fn tick_optimized(&mut self, blackboard: &Blackboard) -> Status {
    // Check preconditions before traversing children
    if !self.precondition_met(blackboard) {
        return Status::Failure; // Skip entire subtree
    }
    // ... rest of tick logic
}
```

**Validation**:
- Tracy: Behavior tree traversal time should reduce by 30%+

---

### **Day 5 (Oct 16): Validation & Documentation** (4-6h)

#### Task 5.1: Re-run Tracy Profiling (1.5-2h)
**Goal**: Capture post-optimization baselines

```powershell
# Rebuild with optimizations
cargo build -p profiling_demo --features profiling --release

# Re-run Tracy capture (500 entities - target config)
cargo run -p profiling_demo --features profiling --release -- --entities 500

# Save trace: baseline_500_optimized.tracy
```

**Compare to Week 8 Day 1 Baseline**:
- Load both traces in Tracy (original vs optimized)
- Compare frame times (avg, p95)
- Verify hotspot reductions (expected 20-50% per target)

---

#### Task 5.2: Regression Testing (1-1.5h)
**Goal**: Ensure no breakage

```powershell
# Run full test suite
cargo test -p astraweave-ecs -p astraweave-ai -p astraweave-physics -p astraweave-render

# Run benchmarks (validate no regressions)
cargo bench -p astraweave-core --bench ecs_benchmarks
cargo bench -p astraweave-behavior --bench goap_planning
cargo bench -p astraweave-physics --bench character_controller

# Run working examples
cargo run -p hello_companion --release
cargo run -p profiling_demo --release -- --entities 500
```

**Expected**: ✅ All tests pass, no benchmark regressions

---

#### Task 5.3: Update Performance Baselines (1h)
**File**: `BASELINE_METRICS.md`

**Update Sections**:
```markdown
## Week 8 Performance Optimizations (October 2025)

### Frame Time Improvements (500 entities)
| Metric | Week 7 (Pre-Opt) | Week 8 (Post-Opt) | Improvement | Status |
|--------|------------------|-------------------|-------------|--------|
| Average frame time | X.XX ms | X.XX ms | -X.X% | ✅ |
| p95 frame time | X.XX ms | X.XX ms | -X.X% | ✅ |
| FPS (avg) | XX | XX | +X% | ✅ |

### Hotspot Reductions
| Function | Week 7 Time | Week 8 Time | Improvement | Target Met |
|----------|-------------|-------------|-------------|------------|
| Render::MainPass | X.XX ms | X.XX ms | -X.X% | ✅/⚠️ |
| Render::ShadowMaps | X.XX ms | X.XX ms | -X.X% | ✅/⚠️ |
| Physics::Rapier::pipeline | X.XX ms | X.XX ms | -X.X% | ✅/⚠️ |

### Telemetry Metrics
| Plot | Week 7 | Week 8 | Delta | Status |
|------|--------|--------|-------|--------|
| draw_calls (avg) | XX | XX | -XX | ✅ (target <10) |
| cache_hit_rate | XX% | XX% | +X% | ✅ (target >95%) |
| visible_instances | XX | XX | ±X | ✅ |
```

---

#### Task 5.4: Create Completion Report (1.5-2h)
**File**: `WEEK_8_OPTIMIZATION_COMPLETE.md`

**Report Structure**:
```markdown
# Week 8 Complete: Performance Optimization Sprint

## Executive Summary
Successfully completed Week 8 performance optimization sprint, achieving [X]% frame time reduction at 500 entities. Targeted [N] hotspots identified by Tracy profiling, implementing optimizations for [rendering/physics/AI]. 60 FPS maintained at target entity count.

## Optimizations Implemented
1. **[Optimization 1 Name]** (X hours)
   - Target: [Hotspot name]
   - Approach: [Brief description]
   - Results: X.X% reduction in function time
   - Files: [Modified files]

2. **[Optimization 2 Name]** (X hours)
   ...

## Performance Results
### Frame Time Comparison (500 entities)
- **Week 7 (Pre-Opt)**: X.XX ms avg, X.XX ms p95
- **Week 8 (Post-Opt)**: X.XX ms avg, X.XX ms p95
- **Improvement**: -X.X% frame time, +X FPS

### Hotspot Reductions
[Charts/tables showing before/after Tracy data]

## Lessons Learned
- What worked well
- What was challenging
- Architecture insights

## Next Steps
- Week 9 priorities
- Remaining optimization opportunities
```

---

## 📊 Expected Week 8 Outcomes

### Performance Targets
| Metric | Week 7 Baseline | Week 8 Target | Stretch Goal |
|--------|----------------|---------------|--------------|
| **Frame time @ 500 entities** | X.XX ms | <16.67 ms (60 FPS) | <13.33 ms (75 FPS) |
| **p95 latency** | X.XX ms | <16.67 ms | <14 ms |
| **Draw calls** | XX | <10 | <5 |
| **Cache hit rate** | 97.9% | >95% | >99% |

### Optimization Impact (Hypothetical)
| Subsystem | Week 7 % | Week 8 Target % | Reduction |
|-----------|----------|-----------------|-----------|
| **Rendering** | 70% | 50-60% | -10-20% |
| **Physics** | 20% | 15-20% | -0-5% |
| **AI** | 8% | 5-8% | -0-3% |
| **ECS** | 2% | 1-2% | -0-1% |

---

## 🔧 Tools & Resources

### Tracy Profiling
- **Download**: https://github.com/wolfpld/tracy/releases/latest
- **Documentation**: https://github.com/wolfpld/tracy/releases/download/v0.11/tracy.pdf
- **Shortcuts**:
  - `Statistics` → Sort by self time (find hotspots)
  - `Find zone` → Search for specific function
  - `Flame graph` → Hierarchical call tree
  - `Plots` → Telemetry trends (draw_calls, cache_hits)

### Benchmarking Commands
```powershell
# Re-run baselines after optimizations
cargo bench -p astraweave-core --bench ecs_benchmarks
cargo bench -p astraweave-behavior --bench goap_planning
cargo bench -p astraweave-physics --bench character_controller
cargo bench -p astraweave-render --bench mesh_optimization

# Compare to Week 3 baselines (see BASELINE_METRICS.md)
```

### Validation Commands
```powershell
# Test suite
cargo test -p astraweave-ecs -p astraweave-ai -p astraweave-physics -p astraweave-render

# Profiling demo (validate optimizations)
cargo run -p profiling_demo --features profiling --release -- --entities 500

# Example (sanity check)
cargo run -p hello_companion --release
```

---

## 📝 Documentation Deliverables

### Week 8 Reports
1. ✅ `WEEK_8_KICKOFF.md` - This document (planning)
2. ⏳ `PROFILING_BASELINE_WEEK_8.md` - Tracy baseline analysis (Day 1)
3. ⏳ `WEEK_8_OPTIMIZATION_COMPLETE.md` - Final results (Day 5)
4. ⏳ Updated `BASELINE_METRICS.md` - Performance metrics

### Supporting Documentation
- Week 7 profiling reports (reference for instrumentation)
- Week 3 benchmarks (regression validation)
- Phase B roadmap (track progress vs plan)

---

## 🎯 Success Criteria Summary

### Must-Have (Critical)
- ✅ Tracy baselines captured (3 configurations)
- ✅ Top 10 hotspots identified
- ✅ At least 2 optimizations implemented
- ✅ 60 FPS maintained at 500 entities (p95 <16.67ms)
- ✅ Zero regressions (tests pass, benchmarks stable)

### Should-Have (Important)
- ✅ 3+ optimizations implemented
- ✅ 10-20% frame time reduction
- ✅ Draw calls <10 per frame
- ✅ Cache hit rate >95%

### Nice-to-Have (Stretch)
- ✅ 20-30% frame time reduction
- ✅ 75 FPS at 500 entities (p95 <13.33ms)
- ✅ Draw calls <5 per frame
- ✅ Cache hit rate >99%

---

## 🚀 Let's Begin!

**Week 8 Day 1 Status**: Ready to start Tracy baseline capture  
**Next Immediate Action**: Download Tracy server, run profiling_demo  
**Estimated Time to First Baseline**: 30-60 minutes  

**Command to Start**:
```powershell
# Build profiling_demo
cargo build -p profiling_demo --features profiling --release

# Launch Tracy.exe (download first if needed)
# Then run:
cargo run -p profiling_demo --features profiling --release -- --entities 200
```

---

**Week 8 Kickoff Created**: October 12, 2025  
**Generated By**: GitHub Copilot (100% AI-authored)  
**AstraWeave Version**: 0.7.0  
**Phase**: Phase B - Month 4 Week 8 (Performance Optimization Sprint)  

Let's capture those baselines and make AstraWeave even faster! 🚀
