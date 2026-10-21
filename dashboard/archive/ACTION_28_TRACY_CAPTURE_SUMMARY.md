# Action 28: Tracy Hotspot Analysis - Capture Summary

**Date**: October 13, 2025  
**Status**: 🔄 **IN PROGRESS** (Captures Complete, Analysis Pending)  
**Phase**: B (Week 9)

---

## Tracy Captures Complete

### Profile Configurations

| Profile | Entities | Frames | Frame Time | FPS | Tracy File |
|---------|----------|--------|------------|-----|------------|
| **Baseline** | 1,000 | 1,000 | 3.774 ms | 264.91 | `baseline_1000.tracy` |
| **Scaling** | 2,000 | 1,000 | 14.841 ms | 67.38 | `baseline_2000.tracy` |
| **Stress** | 5,000 | 1,000 | 77.433 ms | 12.91 | `baseline_5000.tracy` |

### Scaling Analysis

**1k → 2k entities**:
- Frame time: 3.774 ms → 14.841 ms (**+293% or 3.93×**)
- Expected: 2× (linear scaling)
- **Observation**: Sub-quadratic scaling (3.93× < 4×), but worse than linear

**2k → 5k entities**:
- Frame time: 14.841 ms → 77.433 ms (**+422% or 5.22×**)
- Expected: 2.5× (linear scaling from 2k to 5k)
- **Observation**: Super-linear scaling (5.22× > 2.5×), approaching quadratic

**Analysis**: Performance degrades non-linearly. Suspect O(n log n) or O(n²) bottleneck kicking in at higher entity counts.

---

## Comparison to Week 8 Baseline

### Week 8 Final Results (October 12, 2025)
- **1,000 entities**: 2.70 ms @ 370 FPS
- **Configuration**: Spatial hash with HashMap (likely default SipHash)

### Week 9 FxHashMap Results (October 13, 2025)
- **1,000 entities**: 3.774 ms @ 265 FPS (with Tracy overhead)
- **Configuration**: Spatial hash with FxHashMap

**Regression**: **+1.074 ms (+39.8%)**

### Possible Causes

1. **Tracy Overhead** (~10-15%):
   - Week 8 Tracy captures: 2.70 ms baseline
   - Week 9 with Tracy: 3.774 ms
   - Expected Tracy overhead: 0.4-0.5 ms
   - **Remaining regression**: ~0.6-0.7 ms

2. **FxHashMap Performance**:
   - Hypothesis: FxHashMap may NOT be faster for collision detection workload
   - Reason: Collision detection has excellent key locality (nearby cells)
   - SipHash might have better cache behavior for clustered keys

3. **Other System Changes**:
   - ECS iteration overhead
   - Memory allocation patterns
   - SIMD movement changes

4. **Measurement Variance**:
   - Different system state (background processes, CPU throttling)
   - Need multiple runs to establish confidence intervals

---

## Next Steps (Analysis Phase)

### 1. Examine Tracy Captures

**Baseline (1,000 entities)**:
- Open `baseline_1000.tracy` in Tracy profiler
- Identify top 10 functions by total time
- Look for:
  - `collision_detection_system` (expected hotspot)
  - `SpatialHash::insert` / `SpatialHash::query`
  - `movement_system` (SIMD batch processing)
  - `ai_planning_system` (GOAP/BT execution)

**Scaling (2,000 entities)**:
- Compare to baseline
- Identify functions that scale super-linearly (>2× time increase)
- Check for:
  - HashMap resize events (allocation spikes)
  - Collision pair explosion (O(n²) behavior)

**Stress (5,000 entities)**:
- Identify catastrophic bottlenecks (>10× time increase)
- Check for:
  - Memory allocator thrashing (malloc/free)
  - Cache misses (L1/L2/L3 miss rates)
  - System-level contention (lock contention, page faults)

### 2. Generate Hotspot Report

**Format**:
```
=== Top 10 Hotspots @ 1,000 Entities ===
1. collision_detection_system: 1,200 µs (31.8% of frame)
2. movement_system: 675 µs (17.9%)
3. SpatialHash::query: 450 µs (11.9%)
4. ai_planning_system: 380 µs (10.1%)
5. ...
```

### 3. Compare FxHashMap vs SipHash

**Hypothesis Testing**:
- If `SpatialHash::insert` + `SpatialHash::query` > 500 µs: FxHashMap is slower
- If `SpatialHash::insert` + `SpatialHash::query` < 300 µs: FxHashMap is faster, regression is elsewhere

**Action**:
- If FxHashMap is slower: Revert to SipHash or try BTreeMap
- If FxHashMap is faster: Investigate ECS/collision detection overhead

### 4. Identify Allocation Hotspots

**Check Tracy Memory View**:
- Malloc/free flamegraph
- Look for:
  - Frequent small allocations (<64 bytes)
  - Large allocation spikes (Vec resize, HashMap rehash)
  - Memory leaks (allocations without frees)

**Common Culprits**:
- `Vec::push` in hot loops
- `HashMap::entry().or_insert()` overhead
- Temporary `Vec` allocations in queries

### 5. Flamegraph Analysis

**Generate Flamegraphs**:
- CPU time flamegraph (identify call stack bottlenecks)
- Memory flamegraph (identify allocation sources)
- Compare 1k vs 2k vs 5k (identify scaling bottlenecks)

---

## Expected Findings (Predictions)

### Prediction 1: Collision Detection Dominates @ 5k Entities

**Reason**: Collision detection scales as O(n * avg_neighbors)
- @ 1k entities: ~1,000 × 5 = 5,000 checks
- @ 5k entities: ~5,000 × 15 = 75,000 checks (15× more work)

**Expected Time**: 60-70% of frame time @ 5k entities

### Prediction 2: FxHashMap is NOT Faster for Spatial Hash

**Reason**: Collision detection has excellent key locality
- Entities cluster in nearby cells (spatial coherence)
- SipHash cache behavior might be better for clustered keys
- FxHashMap optimization may not help for this workload

**Expected**: `SpatialHash::query` takes 10-15% of frame time (not improved)

### Prediction 3: SIMD Movement Scales Linearly

**Reason**: SIMD batch processing is O(n) with great vectorization
- @ 1k entities: ~675 µs (from Week 8)
- @ 5k entities: ~3,375 µs (5× linear scaling)

**Expected**: Movement system is NOT a bottleneck (remains ~5-10% of frame)

### Prediction 4: Memory Allocator Thrashing @ 5k Entities

**Reason**: Spatial hash rebuilds every frame (clear + insert)
- @ 5k entities: ~5,000 inserts/frame = 5,000 `Vec::push` calls
- HashMap rehashes during growth (allocation spikes)

**Expected**: Malloc/free accounts for 10-20% of frame time @ 5k

---

## Tracy Analysis Checklist

### Statistics View
- [ ] Identify top 10 functions by total time
- [ ] Identify top 10 functions by self time (exclude children)
- [ ] Check mean time per function call (identify hot per-call overhead)
- [ ] Check call counts (identify functions called too often)

### Timeline View
- [ ] Identify frame time variance (min/max/p95)
- [ ] Check for allocation spikes (memory timeline)
- [ ] Check for GC/allocator pauses (flat zones in CPU timeline)
- [ ] Identify system-level interrupts (context switches, page faults)

### Flamegraph View
- [ ] Generate CPU time flamegraph
- [ ] Generate memory allocation flamegraph
- [ ] Compare flamegraphs across 1k/2k/5k entities
- [ ] Identify widest bars (most time spent)

### Memory View
- [ ] Check total memory usage (1k vs 5k entities)
- [ ] Identify large allocations (>1 KB)
- [ ] Identify frequent allocations (<64 bytes, >1000/frame)
- [ ] Check for memory leaks (allocations without frees)

### Zones View
- [ ] Expand `collision_detection_system` zone
- [ ] Measure `SpatialHash::insert` time
- [ ] Measure `SpatialHash::query` time
- [ ] Measure `distance check` time (narrow-phase collision)

---

## Deliverable

### Hotspot Report (ACTION_28_HOTSPOT_REPORT.md)

**Contents**:
1. **Top 10 Hotspots @ 1k/2k/5k Entities**
   - Function name, time, % of frame
   - Scaling factor (how time increases with entities)

2. **FxHashMap Performance Analysis**
   - `SpatialHash::insert` time
   - `SpatialHash::query` time
   - Comparison to Week 8 baseline (if available)
   - Recommendation: Keep FxHashMap or revert to SipHash

3. **Allocation Hotspots**
   - Top 10 allocation sites by count
   - Top 10 allocation sites by size
   - Recommendations for memory pooling

4. **Scaling Bottlenecks**
   - Functions with super-linear scaling (>2× per 2× entities)
   - O(n²) suspects (collision detection, spatial queries)
   - Recommendations for parallel processing

5. **Next Optimizations (Week 10-12 Preview)**
   - Top 3 optimization targets (by potential impact)
   - Estimated frame time reduction per optimization
   - Implementation complexity (low/medium/high)

---

## Tracy Server Instructions

### Capture Files
- `baseline_1000.tracy` (1,000 entities, 1,000 frames)
- `baseline_2000.tracy` (2,000 entities, 1,000 frames)
- `baseline_5000.tracy` (5,000 entities, 1,000 frames)

### Save Locations
Recommend saving to: `docs/profiling/week9/`

### Analysis Workflow
1. Open Tracy profiler GUI
2. File > Open Trace > `baseline_1000.tracy`
3. Navigate to Statistics view → sort by total time
4. Screenshot top 10 functions
5. Navigate to Flamegraph view → generate CPU flamegraph
6. Screenshot flamegraph
7. Repeat for 2k and 5k traces

---

## Summary

**Status**: ✅ **3 Tracy captures complete**  
**Next**: 🔍 **Analyze captures in Tracy GUI**  
**Deliverable**: **Hotspot report with top 10 bottlenecks**

Tracy server should now have 3 traces ready for analysis. Once you've examined the captures, we can create the detailed hotspot report and proceed with targeted optimizations based on the findings.

---

**Version**: 1.0  
**Status**: Captures Complete (Analysis Pending)  
**Owner**: AstraWeave Copilot (AI-generated)  
**Last Updated**: October 13, 2025
