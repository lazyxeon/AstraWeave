# Tracy Profiling Analysis Guide for AstraWeave

**Purpose**: Step-by-step guide for analyzing Tracy `.tracy` files to identify performance hotspots  
**Target**: Week 8 Day 1 baseline analysis (200, 500, 1000 entity configurations)  
**Generated**: October 12, 2025  

---

## Prerequisites

### Required Software
- **Tracy Server 0.11+**: https://github.com/wolfpld/tracy/releases/latest
  - Download: `Tracy-0.11.x-Windows.zip` (or Linux/Mac equivalent)
  - Extract to: `C:\Tools\Tracy\` (or your preferred location)
  - Executable: `Tracy.exe`

### Required Files
- `baseline_200.tracy` - Low load baseline
- `baseline_500.tracy` - Target capacity baseline
- `baseline_1000.tracy` - Stress test baseline

**Location**: AstraWeave root directory (saved during capture phase)

---

## Tracy Interface Overview

### Main Window Sections
```
┌────────────────────────────────────────────────────────────┐
│ Menu Bar: [File] [View] [Statistics] [Find zone] ...      │
├────────────────────────────────────────────────────────────┤
│ Timeline View (default)                                     │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ Frame timeline (horizontal bars = profiling spans)     │ │
│ │ CPU threads (Main, Render, Physics, etc.)              │ │
│ │ Plots (entity_count, draw_calls, cache_hits)           │ │
│ └────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────┤
│ Info Panel (bottom)                                        │
│ - Frame time, FPS, zone statistics                        │
└────────────────────────────────────────────────────────────┘
```

### Key Views for Analysis
1. **Statistics View** - Hotspot identification (primary tool)
2. **Timeline View** - Visual frame inspection
3. **Flame Graph** - Call hierarchy analysis
4. **Plots** - Telemetry trends (draw_calls, cache_hits, etc.)
5. **Find Zone** - Search for specific functions

---

## Step-by-Step Analysis Workflow

### Phase 1: Frame Time Overview (5 min per config)

#### 1.1: Open Trace File
```
File > Open Trace > Select baseline_500.tracy
```

**Verify Connection**:
- Tracy window shows timeline with colored bars (profiling spans)
- Info panel (bottom) displays frame statistics
- No "Waiting for connection" message

---

#### 1.2: Check Frame Time Statistics
**Location**: Info panel (bottom-right)

**Key Metrics to Record**:
```
Frame time:
- Mean: _____ ms
- Median: _____ ms  
- Std dev: _____ ms  (stability indicator)

FPS:
- Mean: _____
- Median: _____

Frame range: _____ to _____ (should be 1000+ frames)
```

**Acceptance Criteria**:
- Mean frame time <16.67 ms (60 FPS) for 200/500 entities
- Std dev <2 ms (stable performance)
- 1000+ frames captured (sufficient sample)

---

#### 1.3: Identify Frame Spikes
**Visual Check**: Timeline view

**Look For**:
- Tall vertical bars (frames >20 ms)
- Irregular spacing (frame time variance)
- Long `Render::Present` zones (GPU bottleneck)

**Action**: Click on spike frame → Inspect which span is longest

**Record**:
```
Frame spikes (>20 ms):
- Count: _____ spikes in 1000 frames
- Cause: [Render::MainPass / Render::Present / Physics / AI]
- p95 frame time: _____ ms  (95th percentile)
- p99 frame time: _____ ms  (99th percentile)
```

---

### Phase 2: Hotspot Identification (15-20 min per config)

#### 2.1: Open Statistics View
**Menu**: `Statistics > Show statistics`

**Configuration**:
1. Sort by: **"Self time"** descending (click column header)
2. Filter: (None - show all functions)
3. Time range: (Entire capture - default)

**What "Self Time" Means**:
- Time spent **within the function itself** (excluding children)
- Example: `Render::Frame` has low self time (mostly calls children)
- Example: `Render::MainPass` has high self time (GPU work)

---

#### 2.2: Extract Top 10 Hotspots
**Criteria**: Functions with >5% total frame time

**Template**:
```markdown
## Hotspot Analysis: 500 Entities

### Top 10 Functions by Self Time
| Rank | Function Name | Self Time (ms) | % of Frame | Parent |
|------|---------------|----------------|------------|--------|
| 1    | Render::MainPass | X.XX | XX% | Render::Frame |
| 2    | Render::ShadowMaps | X.XX | XX% | Render::Frame |
| 3    | Physics::Rapier::pipeline | X.XX | XX% | Physics::AsyncTick |
| 4    | Render::Present | X.XX | XX% | Render::Frame |
| 5    | GOAP::Planner::plan | X.XX | XX% | AI::Planning |
| 6    | Render::ClusteredLighting | X.XX | XX% | Render::Frame |
| 7    | Render::BufferWrite::Instances | X.XX | XX% | Render::Frame |
| 8    | Physics::CharacterController::move | X.XX | XX% | Physics::AsyncTick |
| 9    | BehaviorTree::tick | X.XX | XX% | AI::Planning |
| 10   | Render::MeshUpload | X.XX | XX% | Render::Frame |
```

**How to Fill**:
1. Click on row in Statistics view
2. "Self time" column = milliseconds spent in function
3. "% of parent" or calculate: (Self time / Mean frame time) × 100
4. "Parent" = Check Timeline view (which span contains this one)

---

#### 2.3: Categorize by Subsystem
**Group hotspots by subsystem**:

```markdown
### Subsystem Breakdown (500 entities)
| Subsystem | Total Time | % of Frame | Top Hotspot | Notes |
|-----------|-----------|------------|-------------|-------|
| **Rendering** | X.XX ms | XX% | Render::MainPass (X.XX ms) | Expected dominant |
| **Physics** | X.XX ms | XX% | Rapier::pipeline (X.XX ms) | Within budget |
| **AI** | X.XX ms | XX% | GOAP::Planner (X.XX ms) | Cache hit rate: XX% |
| **ECS** | X.XX ms | XX% | World::tick (X.XX ms) | Minimal overhead |
| **Other** | X.XX ms | XX% | - | Misc overhead |
| **TOTAL** | X.XX ms | 100% | - | Should ≈ mean frame time |
```

**Calculation Example**:
- `Rendering` = Sum of `Render::MainPass`, `Render::ShadowMaps`, `Render::ClusteredLighting`, etc.
- Compare to mean frame time to verify (should be within 5%)

---

### Phase 3: Flame Graph Analysis (10 min per config)

#### 3.1: Open Flame Graph
**Menu**: `Flame graph > Show flame graph`

**What It Shows**:
- Hierarchical call tree (parent → child relationships)
- Width = time spent in function + children
- Color = different functions/subsystems

**Visual Inspection**:
- **Wide bars at top** = High-level functions (e.g., `Render::Frame`)
- **Wide bars at bottom** = Leaf functions doing actual work (e.g., `wgpu::CommandEncoder::draw_indexed`)
- **Deep stacks** = Expensive call chains (potential optimization target)

---

#### 3.2: Identify Unexpected Patterns
**Look For**:
1. **Recursive calls** (repeating function names in stack)
   - Example: `plan_recursive → plan_recursive → ...`
   - Fix: Memoization, iterative approach

2. **Allocation hotspots** (alloc/dealloc in flame graph)
   - Example: `Vec::push → realloc → ...`
   - Fix: Pre-allocate, use slab allocators

3. **Redundant work** (same function called many times)
   - Example: `calculate_transform` called per entity (not batched)
   - Fix: Batch processing, caching

**Record Findings**:
```markdown
### Flame Graph Insights (500 entities)
1. **Recursive GOAP planning**: `plan_recursive` stack depth 8-12
   - Cause: Complex goal chains (attack → move → wait)
   - Fix: Iterative planner, cache intermediate states

2. **Allocation in Render::MainPass**: `Vec::push` in tight loop
   - Cause: Instance buffer rebuilt per frame
   - Fix: Pre-allocate capacity, reuse buffer

3. **Redundant transform calculations**: `calculate_transform` × 500 calls
   - Cause: Per-entity calculation (not batched)
   - Fix: SIMD batch transform (4 entities at once)
```

---

### Phase 4: Plot Analysis (5 min per config)

#### 4.1: View Telemetry Plots
**Location**: Timeline view → Scroll to `Plots` section (below CPU threads)

**Available Plots** (from Week 7 instrumentation):
1. **`entity_count`** - Total entities in world
2. **`Render::draw_calls`** - GPU draw calls per frame
3. **`Render::visible_instances`** - Frustum culling output
4. **`GOAP::cache_hits`** - Planning cache effectiveness
5. **`Physics::character_move_count`** - Characters moving per frame

---

#### 4.2: Analyze Plot Trends
**For Each Plot**:
1. **Check Stability**: Should be flat/constant (no wild fluctuations)
2. **Record Average**: Hover over plot → Tracy shows value
3. **Correlate to Hotspots**: Does high value = high frame time?

**Template**:
```markdown
### Telemetry Plots (500 entities)
| Plot | Average | Min | Max | Trend | Notes |
|------|---------|-----|-----|-------|-------|
| entity_count | 500 | 500 | 500 | Stable ✅ | Constant (expected) |
| draw_calls | XX | XX | XX | Stable/Variable | Target: <10 |
| visible_instances | XX | XX | XX | Variable | 50-80% of total (good culling) |
| cache_hits | XX% | XX% | XX% | Stable ✅ | Target: >95% |
| character_move_count | XX | XX | XX | Variable | Depends on AI behavior |
```

**Optimization Insights**:
- **draw_calls >10**: Implement draw call batching (Week 8 Day 2)
- **visible_instances >80%**: Improve frustum culling (overly conservative)
- **cache_hits <95%**: Warm GOAP cache at startup (Week 8 Day 3)

---

### Phase 5: Comparison Across Configurations (10 min)

#### 5.1: Load All 3 Traces
**Open in Tracy**:
1. `baseline_200.tracy` → Record metrics
2. `baseline_500.tracy` → Record metrics
3. `baseline_1000.tracy` → Record metrics

**Close and re-open between traces** (Tracy doesn't support multi-trace view)

---

#### 5.2: Scalability Analysis
**Create Comparison Table**:

```markdown
## Scalability Analysis: 200 → 500 → 1000 Entities

### Frame Time Scaling
| Metric | 200 Entities | 500 Entities | 1000 Entities | Scaling |
|--------|-------------|--------------|---------------|---------|
| Mean frame time | X.XX ms | X.XX ms | X.XX ms | 2.5× / 5× |
| p95 frame time | X.XX ms | X.XX ms | X.XX ms | - |
| FPS (avg) | XX | XX | XX | - |
| **Status** | ✅ 60+ FPS | ✅/⚠️ 60 FPS | ⚠️/❌ <60 FPS | - |

### Subsystem Scaling (% of Frame)
| Subsystem | 200 | 500 | 1000 | Trend |
|-----------|-----|-----|------|-------|
| Rendering | XX% | XX% | XX% | Constant/Linear/Superlinear |
| Physics | XX% | XX% | XX% | Linear (expected) |
| AI | XX% | XX% | XX% | Sublinear (cache helps) |
| ECS | XX% | XX% | XX% | Sublinear (archetype efficiency) |
```

**Scaling Interpretation**:
- **Constant** (XX% same across configs): Not entity-dependent (good)
- **Linear** (XX% increases proportionally): O(n) complexity (acceptable)
- **Superlinear** (XX% increases >2× when entities 2×): O(n²) or worse (optimize!)

**Example Analysis**:
```
Physics: 10% @ 200 → 20% @ 500 → 45% @ 1000
Scaling: Superlinear (4.5× increase for 5× entities)
Diagnosis: O(n²) broad-phase collision detection
Fix: Spatial hashing (reduce to O(n log n))
```

---

### Phase 6: Optimization Prioritization (15 min)

#### 6.1: Rank Optimization Targets
**Criteria** (weighted):
1. **Impact** (50%): % of frame time consumed
2. **Feasibility** (30%): Complexity of optimization (1-5 days)
3. **Scalability** (20%): Does it improve with more entities?

**Scoring Example**:
```markdown
### Optimization Candidates (500 entities)

| Target | Frame % | Feasibility | Scalability | Score | Priority |
|--------|---------|-------------|-------------|-------|----------|
| Render::MainPass | 50% | Medium (3d) | Constant | 42 | 🔴 High |
| Render::ShadowMaps | 15% | Easy (1d) | Constant | 13 | 🟡 Med |
| Physics::Rapier | 20% | Hard (5d) | Superlinear | 24 | 🔴 High |
| GOAP::Planner | 8% | Easy (1d) | Sublinear | 6 | 🟢 Low |
| Draw Call Batching | 5% | Medium (2d) | Constant | 5 | 🟡 Med |

Score = (Frame% × 0.5) + (5 - Feasibility) × 0.3 × 10 + (Scalability bonus)
```

**Prioritize**:
1. **High Score (>20)**: Must-do optimizations (Week 8 Days 2-3)
2. **Medium Score (10-20)**: Should-do (Week 8 Day 4)
3. **Low Score (<10)**: Nice-to-have (defer to Week 9)

---

#### 6.2: Define Week 8 Roadmap
**Select Top 3 Targets**:

```markdown
## Week 8 Optimization Roadmap

### Target 1: Render::MainPass Optimization (50% → 35-40%)
**Approach**: Draw call batching via material grouping
**Expected Impact**: 10-15% frame time reduction
**Time**: 4-6 hours (Day 2-3)
**Files**: `astraweave-render/src/renderer.rs`, `src/material.rs`

### Target 2: Physics::Rapier Spatial Hashing (20% → 12-15%)
**Approach**: Grid-based broad-phase collision culling
**Expected Impact**: 5-8% frame time reduction
**Time**: 8-10 hours (Day 3-4)
**Files**: `astraweave-physics/src/spatial_hash.rs` (new)

### Target 3: Shadow Map Resolution Tuning (15% → 10-12%)
**Approach**: Reduce distant cascade to 512×512 (from 1024×1024)
**Expected Impact**: 3-5% frame time reduction
**Time**: 2-3 hours (Day 4)
**Files**: `astraweave-render/src/renderer.rs`

### Total Expected Improvement
- Frame time: X.XX ms → X.XX ms (-18-28%)
- FPS: XX → XX (+X FPS)
- p95 latency: <16.67 ms (60 FPS sustained)
```

---

## Data Collection Template

**Use this template for baseline report**:

```markdown
# Week 8 Tracy Profiling Baselines

## System Specifications
- CPU: [e.g., AMD Ryzen 9 5950X, 16 cores @ 3.4 GHz]
- GPU: [e.g., NVIDIA RTX 3080, 10 GB VRAM]
- RAM: [e.g., 32 GB DDR4-3200]
- OS: Windows 11 Pro 23H2
- Tracy Version: 0.11.1
- Rust Version: 1.89.0
- Build Config: --release --features profiling

## Configuration: 200 Entities (Low Load)
### Frame Time Statistics
- Mean: X.XX ms
- Median: X.XX ms
- Std dev: X.XX ms
- p95: X.XX ms
- p99: X.XX ms
- FPS (avg): XX
- **Status**: ✅ Well above 60 FPS

### Top 5 Hotspots
1. Render::MainPass - X.XX ms (XX%)
2. Render::ShadowMaps - X.XX ms (XX%)
3. Physics::Rapier::pipeline - X.XX ms (XX%)
4. Render::ClusteredLighting - X.XX ms (XX%)
5. Render::Present - X.XX ms (XX%)

### Subsystem Breakdown
- Rendering: X.XX ms (XX%)
- Physics: X.XX ms (XX%)
- AI: X.XX ms (XX%)
- ECS: X.XX ms (XX%)

---

## Configuration: 500 Entities (Target Capacity)
[Repeat template above]

---

## Configuration: 1000 Entities (Stress Test)
[Repeat template above]

---

## Scalability Analysis
[Use table from Phase 5.2]

---

## Optimization Priorities
[Use roadmap from Phase 6.2]

---

## Regression Check vs Week 3 Baselines
| Metric | Week 3 Benchmark | Week 8 Tracy | Delta | Status |
|--------|-----------------|--------------|-------|--------|
| GOAP cache hit rate | 97.9% | XX% | +/-X% | ✅/⚠️ |
| Physics async tick | 2.96 ms | X.XX ms | +/-X ms | ✅/⚠️ |
| Character move | 114 ns | X ns | +/-X ns | ✅/⚠️ |
| Terrain chunk gen | 15.06 ms | X.XX ms | +/-X ms | ✅/⚠️ |

**Notes**:
- ✅ Within 5% of baseline (acceptable variance)
- ⚠️ >5% regression (investigate before optimizations)
- Tracy measures real-world performance (vs Criterion micro-benchmarks)
```

---

## Common Tracy Issues & Fixes

### Issue 1: "No connection to profiled program"
**Cause**: profiling_demo not built with `--features profiling`

**Fix**:
```powershell
cargo build -p profiling_demo --features profiling --release
```

---

### Issue 2: Tracy shows empty timeline
**Cause**: Tracy server started AFTER profiling_demo (missed connection window)

**Fix**:
1. Start Tracy.exe FIRST
2. Wait for "Listening on port 8086" message
3. THEN run profiling_demo

---

### Issue 3: Frame time = 16.67 ms (exactly)
**Cause**: VSync enabled (caps FPS at 60)

**Fix**: Profiling correctly, but VSync masks true GPU performance
- Disable VSync in profiling_demo (if needed for headroom analysis)
- Focus on hotspot % distribution instead of absolute time

---

### Issue 4: Render::Present dominates (>50% frame time)
**Cause**: GPU bottleneck (CPU waiting for GPU to finish)

**Interpretation**:
- NOT a CPU optimization target
- Indicates GPU shader work is slow (PBR complexity, shadow quality)
- Solution: Reduce shader complexity, lower shadow resolution, optimize materials

---

### Issue 5: Statistics view shows 1000s of tiny functions
**Cause**: Default filter shows all captured zones

**Fix**:
1. Click "Filter" in Statistics view
2. Set minimum self time: 0.1 ms (filter noise)
3. Focus on functions >5% of frame

---

## Next Steps After Analysis

1. **Create `PROFILING_BASELINE_WEEK_8.md`** using data collection template
2. **Define Week 8 optimization roadmap** (top 3 targets)
3. **Start Day 2 optimizations** (highest priority hotspot)
4. **Re-run Tracy after optimizations** (Day 5 validation)

---

**Tracy Analysis Guide Complete**  
**Generated**: October 12, 2025  
**AstraWeave Version**: 0.7.0  
**Phase**: Phase B - Month 4 Week 8 Day 1  
**100% AI-Authored by GitHub Copilot**  

Happy profiling! 🔍🚀
