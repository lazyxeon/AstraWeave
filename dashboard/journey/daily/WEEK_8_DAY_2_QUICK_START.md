# Week 8 Day 2: Tracy Validation - Quick Start Card

**⏱️ Total Time: 10-15 minutes**

---

## Step 1: Start Tracy Server (1 min)

```powershell
# Download (if needed): https://github.com/wolfpld/tracy/releases
# Run Tracy.exe (should show "Listening on port 8086")
C:\Tools\Tracy\Tracy.exe
```

✅ Window open, status bar shows "Listening on port 8086"

---

## Step 2: Run Profiling Demo (3-5 min)

```powershell
# In repository root:
cd C:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine
cargo run -p profiling_demo --features profiling --release -- --entities 1000
```

**Expected Output**:
```
Profiling enabled (Tracy 0.11.1)
Spawning 1000 entities
Running for 1002 frames...
Completed 1002 frames in ~3s (~350 FPS)
```

✅ Tracy shows "Connected to: profiling_demo"
✅ Timeline populates with spans

---

## Step 3: Save Trace (30 sec)

**In Tracy**:
1. Click "Stop" button (or wait for demo auto-exit)
2. File → Save
3. Navigate to: `C:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine\profiling\`
4. Filename: `baseline_1000_spatial_hash.tracy`
5. Click "Save"

✅ File size: ~50-100 MB

---

## Step 4: Quick Validation (2-3 min)

### Statistics View

1. Click "Statistics" tab
2. Sort by "Total time" descending
3. Find "collision_detection" row
4. Check time: **Should be 250-330 µs** (vs 548.5 µs baseline)

✅ 40-55% reduction

### Plots View

1. Click "Plots" tab
2. Select "Physics.CollisionChecks"
3. Check value: **Should be ~5,000-10,000** (vs ~500,000 baseline)

✅ 99% reduction

---

## Step 5: Compare to Baseline (2-3 min)

**Open both traces**:
1. File → Open: `profiling/trace3.tracy` (Day 1 baseline)
2. File → Open: `profiling/baseline_1000_spatial_hash.tracy` (Day 2)
3. Window → Tile Vertically

**Side-by-Side Check**:
- collision_detection: 548.5 µs → _____ µs (target: 250-330 µs)
- Frame time: 3.09 ms → _____ ms (target: 2.8-2.9 ms)
- CollisionChecks: 500,000 → _____ (target: 5,000-10,000)

---

## Success Criteria Checklist

**Performance** (from Statistics view):
- [ ] collision_detection: 250-330 µs (-40-55% from 548.5 µs)
- [ ] Total frame time: 2.8-2.9 ms (-8-10% from 3.09 ms)
- [ ] FPS: 350+ (+9-11% from 323)

**No Regressions** (other systems ±5%):
- [ ] movement: ~951 µs (unchanged)
- [ ] render_submit: ~845 µs (unchanged)
- [ ] ai_planning: ~518 µs (unchanged)

**Data Quality**:
- [ ] Trace saved: `profiling/baseline_1000_spatial_hash.tracy`
- [ ] Frame count: 1002
- [ ] No gaps or errors

---

## If Validation Succeeds ✅

**Next Steps**:
1. Screenshot Statistics + Plots views
2. Create `WEEK_8_DAY_2_COMPLETE.md` (use data from Tracy)
3. Update `BASELINE_METRICS.md` (add Day 2 optimized baseline)
4. Proceed to Day 3 (SIMD Movement)

**Time to Day 2 Complete**: 1-2 hours (documentation)

---

## If Validation Fails ❌

**Debug Steps**:
1. Check actual collision_detection time (if only -20-30%, not -40-55%)
2. Check Physics.CollisionChecks value (should be < 20,000)
3. Try tuning cell_size:
   ```rust
   // In profiling_demo/src/main.rs, line ~348:
   let mut grid = SpatialHash::new(2.0); // Try 1.0, 4.0, 8.0
   ```
4. Rebuild and re-run Tracy

**Get Help**: See `WEEK_8_DAY_2_TRACY_VALIDATION_GUIDE.md` (troubleshooting section)

---

## Quick Reference

**Tracy Server**: `C:\Tools\Tracy\Tracy.exe`  
**Run Command**: `cargo run -p profiling_demo --features profiling --release -- --entities 1000`  
**Save Location**: `profiling/baseline_1000_spatial_hash.tracy`  
**Target**: 250-330 µs collision_detection, 350+ FPS  

**Documentation**:
- Progress Report: `WEEK_8_DAY_2_SPATIAL_HASH_PROGRESS.md`
- Validation Guide: `WEEK_8_DAY_2_TRACY_VALIDATION_GUIDE.md`
- Summary: `WEEK_8_DAY_2_SUMMARY.md`

---

**Ready to validate? Start Tracy and run the command above! 🚀**
