# Week 8 Day 3: SIMD Movement - Tracy Validation Guide

**Date**: October 12, 2025  
**Status**: Ready for Tracy validation  

---

## Quick Tracy Run

### 1. Run profiling_demo with SIMD movement
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000
```

### 2. Capture with Tracy
- **Duration**: Let run for 15-20 seconds
- **Frames**: Should capture 1,000+ frames @ 60 FPS
- **Save As**: `baseline_1000_simd_movement.tracy`

### 3. Key Metrics to Check

#### Frame Statistics
- **Mean frame time**: Expected **2.3-2.5 ms** (vs 2.87 ms Day 2 baseline)
- **Median frame time**: Expected **2.2-2.4 ms**
- **FPS**: Expected **400-435 FPS** (vs 348 FPS Day 2 baseline)

#### Statistics View - movement Span
- **Total Time**: Should be ~430-600 ms (1,000 frames)
- **MTPC (Mean Time Per Call)**: Expected **430-600 µs** (vs 861 µs Day 2 baseline)
- **Percentage**: Expected **~18-25%** of frame (vs 30% Day 2 baseline)
- **Target Improvement**: **-30-50%** reduction in movement time

#### Timeline View - Verify SIMD
- Zoom to frames 998-1000 (steady-state)
- **movement span width**: Should be noticeably narrower than Day 2
- **Visual check**: movement should be ~50-60% of previous width

#### Plots View
- **Movement.Updates**: Should still show ~1,001 (all entities updated)

---

## Expected Results

### Conservative (2× SIMD speedup, -30%)
- movement: 861 µs → **602 µs** (-30%)
- Frame time: 2.87 ms → **2.56 ms** (-11%)
- FPS: 348 → **391 FPS** (+12%)

### Target (2× SIMD, low overhead, -40%)
- movement: 861 µs → **516 µs** (-40%)
- Frame time: 2.87 ms → **2.52 ms** (-12%)
- FPS: 348 → **397 FPS** (+14%)

### Optimistic (2× SIMD, minimal overhead, -50%)
- movement: 861 µs → **431 µs** (-50%)
- Frame time: 2.87 ms → **2.44 ms** (-15%)
- FPS: 348 → **410 FPS** (+18%)

---

## What to Look For

### ✅ SUCCESS Indicators
1. **movement MTPC < 600 µs** (vs 861 µs baseline)
2. **Frame time < 2.5 ms** (vs 2.87 ms)
3. **FPS > 400** (vs 348)
4. **movement percentage < 25%** (vs 30%)

### ⚠️ WARNING Signs
1. **movement MTPC > 700 µs** → Array collection overhead too high
2. **Frame time > 2.7 ms** → Regression, SIMD not helping
3. **FPS < 370** → Overall performance degraded

### ❌ FAILURE Indicators
1. **movement MTPC > 861 µs** → SIMD slower than naive (impossible if working correctly)
2. **Frame time > 2.87 ms** → Net regression
3. **Crash or errors** → Implementation bug

---

## Next Steps After Tracy Capture

1. **Save trace**: `baseline_1000_simd_movement.tracy`
2. **Take screenshots**:
   - Trace Information (capture metadata)
   - Frame Statistics (frame time histogram)
   - Statistics View (movement span details)
   - Timeline frames 998-1000 (visual comparison)
   - Plots (Movement.Updates)

3. **Analyze results** and create `WEEK_8_DAY_3_COMPLETE.md`

4. **Compare to Day 2 baseline**:
   - Day 2: 2.87 ms frame, 861 µs movement, 348 FPS
   - Day 3 (expected): 2.3-2.5 ms frame, 430-600 µs movement, 400-435 FPS

---

## Troubleshooting

### If movement time INCREASED
- **Cause**: Array collection overhead > SIMD savings
- **Fix**: Try pre-allocating buffers, or use in-place SIMD (if ECS supports)
- **Acceptable**: Up to 10% overhead (941 µs) still worth it for scalability

### If frame time INCREASED
- **Cause**: Other systems slowed down, or measurement variance
- **Check**: collision_detection, ai_planning times - should be stable
- **Action**: Re-run Tracy capture (startup effects may skew first run)

### If compilation failed
- **Cause**: glam version mismatch (already fixed)
- **Verify**: `astraweave-math` uses `glam.workspace = true`

---

## Success Criteria

**Minimum Acceptable** (Day 3 PASS):
- ✅ movement < 600 µs (-30%)
- ✅ Frame time < 2.5 ms (-13%)
- ✅ FPS > 400 (+15%)

**Target** (Day 3 GOOD):
- ⭐ movement 430-550 µs (-37-50%)
- ⭐ Frame time 2.3-2.5 ms (-13-20%)
- ⭐ FPS 400-435 (+15-25%)

**Stretch** (Day 3 EXCELLENT):
- ⭐⭐⭐ movement < 400 µs (-54%)
- ⭐⭐⭐ Frame time < 2.3 ms (-20%)
- ⭐⭐⭐ FPS > 435 (+25%)

---

**Ready to run Tracy capture!** 🚀
