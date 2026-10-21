# Week 8 Day 2: Tracy Validation - Quick Guide

**Status**: 🟢 READY TO RUN  
**Build**: ✅ Release compiled (`profiling_demo --features profiling --release`)  
**Time Required**: 10-15 minutes  

---

## Prerequisites Checklist

- ✅ Spatial hash implemented (`astraweave-physics/src/spatial_hash.rs`)
- ✅ profiling_demo integrated (collision system using SpatialHash)
- ✅ Release build complete (26.23s compilation)
- ⏳ Tracy server downloaded (if not already installed)

---

## Step-by-Step Instructions

### 1. Download Tracy (if needed)

**Windows**:
1. Go to: https://github.com/wolfpld/tracy/releases
2. Download latest release (e.g., `Tracy-0.11.1.7z`)
3. Extract to: `C:\Tools\Tracy\` (or any preferred location)
4. Run: `C:\Tools\Tracy\Tracy.exe`

**Verify Tracy Server Running**:
- Window title: "Tracy Profiler"
- Status bar: "Listening on port 8086"
- No connection yet (normal until profiling_demo runs)

---

### 2. Run Profiling Demo (Optimized with Spatial Hash)

**Command** (in repository root):
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000
```

**Expected Output**:
```
Profiling enabled (Tracy 0.11.1)
Spawning 1000 entities
Entities spawned: 1000
Running for 1002 frames...
Frame 100/1002
Frame 200/1002
...
Frame 1002/1002
Completed 1002 frames in 3.05s (328 FPS)
Press ESC to exit or wait for auto-shutdown...
```

**Tracy Connection**:
- Tracy window should show: "Connected to: profiling_demo"
- Timeline view populates with profiling spans
- Plots view shows real-time metrics (EntityCount, FPS, etc.)

**Capture Duration**: ~3 seconds (1002 frames @ target FPS)

---

### 3. Capture & Save Tracy Trace

**During Run** (Tracy window):
1. **Wait for completion**: Let demo run full 1002 frames
2. **Stop capture**: Click "Stop" button (or demo auto-exits)
3. **Save trace**: File → Save → `baseline_1000_spatial_hash.tracy`
   - Save location: `C:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine\profiling\`
   - Filename: `baseline_1000_spatial_hash.tracy`

**File Size**: Expect ~50-100 MB (1002 frames × profiling data)

---

### 4. Analyze Performance (Quick Validation)

#### 4.1 Statistics View

**Open**: Statistics → Sort by "Total time" descending

**Key Metrics to Check**:

| Span | Baseline (Day 1) | Target (Day 2) | Actual | Status |
|------|------------------|----------------|--------|--------|
| **collision_detection** | 548.5 µs | 250-330 µs | _____ µs | ⏳ |
| **physics** | ~1000 µs | ~700-900 µs | _____ µs | ⏳ |
| **movement** | 951.79 µs | 951.79 µs | _____ µs | ⏳ (unchanged) |
| **render_submit** | 844.76 µs | 844.76 µs | _____ µs | ⏳ (unchanged) |
| **ai_planning** | 518.08 µs | 518.08 µs | _____ µs | ⏳ (unchanged) |

**Success Criteria**:
- ✅ collision_detection: -40-55% reduction (548.5 µs → 250-330 µs)
- ✅ Other systems: ±5% (no regressions)

#### 4.2 Timeline View

**Open**: Timeline → Zoom to first 5 frames

**Visually Inspect**:
- **collision_detection span**: Should be visibly shorter than Day 1 baseline
- **Span order**: Same as before (PRE_SIMULATION → PERCEPTION → SIMULATION → AI_PLANNING → PHYSICS → POST_SIMULATION → PRESENTATION)
- **No gaps**: Continuous execution (no idle time)

**Screenshot**: Save timeline view for documentation

#### 4.3 Plots View

**Open**: Plots → Select "Physics.CollisionChecks"

**Key Metrics**:

| Plot | Baseline (Day 1) | Expected (Day 2) | Actual | Status |
|------|------------------|------------------|--------|--------|
| **Physics.CollisionChecks** | ~500,000 | ~5,000-10,000 | _____ | ⏳ |
| **Physics.Collisions** | ~250 | ~250 | _____ | ⏳ (unchanged) |
| **EntityCount** | 1000 | 1000 | _____ | ⏳ (constant) |
| **FPS** | 323 | 350+ | _____ | ⏳ |

**Success Criteria**:
- ✅ CollisionChecks: 99% reduction (500,000 → 5,000-10,000)
- ✅ Collisions: Same count (spatial hash doesn't drop real collisions)
- ✅ FPS: 10-15% increase (323 → 350+)

---

### 5. Compare to Day 1 Baseline

**Side-by-Side Comparison**:

1. **Open both traces**:
   - `profiling/trace3.tracy` (Day 1 - naive O(n²))
   - `profiling/baseline_1000_spatial_hash.tracy` (Day 2 - spatial hash)

2. **Statistics View**:
   - Sort by "Total time"
   - Compare collision_detection row
   - Calculate % reduction: `(548.5 - actual) / 548.5 × 100`

3. **Plots View**:
   - Open Physics.CollisionChecks in both traces
   - Compare y-axis scale (500,000 vs ~7,500)
   - Screenshot both for visual comparison

**Expected Findings**:
- **Frame Time**: 3.09 ms → 2.8-2.9 ms (-8-10%)
- **collision_detection**: 548.5 µs → 250-330 µs (-40-55%)
- **CollisionChecks**: 500,000 → 5,000-10,000 (-99%)
- **FPS**: 323 → 350-360 (+9-11%)

---

## Validation Checklist

**Performance Targets**:
- [ ] collision_detection: 250-330 µs (from 548.5 µs)
- [ ] Total frame time: 2.8-2.9 ms (from 3.09 ms)
- [ ] Physics.CollisionChecks: 5,000-10,000 (from 500,000)
- [ ] FPS: 350+ (from 323)

**Regression Checks**:
- [ ] movement: 951.79 µs ±5% (no change expected)
- [ ] render_submit: 844.76 µs ±5% (no change expected)
- [ ] ai_planning: 518.08 µs ±5% (no change expected)
- [ ] EntityCount: 1000 (constant)
- [ ] Physics.Collisions: ~250 (same as baseline)

**Data Quality**:
- [ ] Trace file saved: `profiling/baseline_1000_spatial_hash.tracy`
- [ ] Frame count: 1002 frames
- [ ] Capture duration: ~3 seconds
- [ ] No dropped frames or gaps

---

## Troubleshooting

### Tracy Server Won't Connect

**Symptoms**: profiling_demo runs but Tracy shows "No connection"

**Solutions**:
1. Check Tracy server is running (window open, "Listening on port 8086")
2. Disable firewall temporarily (Windows Firewall → Allow Tracy.exe)
3. Restart Tracy server
4. Re-run profiling_demo

### Lower Than Expected Speedup

**Symptoms**: collision_detection only -20-30% (not -40-55%)

**Possible Causes**:
1. **Entity clustering**: More objects near each other than expected
   - Check: Plots → Physics.CollisionChecks average
   - Fix: Adjust cell_size (try 1.0 or 4.0 instead of 2.0)

2. **Grid overhead**: HashMap lookup cost
   - Check: Statistics → time spent in spatial_hash methods
   - Expected: Grid overhead < 50 µs

3. **Compiler optimization issue**:
   - Verify: `--release` flag used (not debug build)
   - Re-compile: `cargo clean; cargo build -p profiling_demo --features profiling --release`

### Crash or Error

**Symptoms**: profiling_demo exits with error

**Debug**:
1. Run without Tracy: `cargo run -p profiling_demo --release -- --entities 1000`
2. Check logs for panic/error message
3. Verify spatial_hash tests pass: `cargo test -p astraweave-physics`

---

## Next Steps (After Validation)

### If Validation Succeeds (✅)

1. **Create WEEK_8_DAY_2_COMPLETE.md**:
   - Document performance improvements
   - Include Tracy screenshots (Statistics, Timeline, Plots)
   - Before/after comparison table
   - Lessons learned

2. **Update BASELINE_METRICS.md**:
   - Add Week 8 Day 2 optimized baseline
   - Update collision_detection threshold: 250-330 µs
   - Document spatial hash parameters

3. **Proceed to Day 3** (SIMD Movement):
   - Goal: movement 951.79 µs → 450-600 µs
   - Create: `astraweave-math/src/simd_movement.rs`
   - Time: 6-8 hours

### If Validation Fails (❌)

1. **Analyze Failure**:
   - Compare actual vs expected metrics
   - Identify bottleneck (grid overhead? entity clustering?)

2. **Iterate**:
   - Tune cell_size (try 1.0, 4.0, 8.0)
   - Profile grid operations (add Tracy spans to spatial_hash methods)
   - Re-run validation

3. **Document**:
   - Record findings in WEEK_8_DAY_2_PROGRESS.md
   - Update todo list with blocking issues

---

## Quick Reference Commands

**Run Profiling Demo** (spatial hash optimized):
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000
```

**Compare Traces** (PowerShell):
```powershell
# Open Tracy
C:\Tools\Tracy\Tracy.exe

# File → Open: profiling/trace3.tracy (Day 1 baseline)
# File → Open: profiling/baseline_1000_spatial_hash.tracy (Day 2 optimized)
# Window → Tile Vertically (side-by-side comparison)
```

**Re-run Tests** (if issues):
```powershell
cargo test -p astraweave-physics --lib spatial_hash
cargo test -p profiling_demo
```

**Clean Rebuild** (if needed):
```powershell
cargo clean
cargo build -p profiling_demo --features profiling --release
```

---

**Estimated Time**: 10-15 minutes (5 min capture, 5-10 min analysis)  
**Success Criteria**: collision_detection 250-330 µs, FPS 350+, no regressions  
**Next**: WEEK_8_DAY_2_COMPLETE.md → Day 3 SIMD Movement
