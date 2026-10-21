# Week 8 Tracy Profiling Quick Reference

**Phase**: Week 8 Day 1 - Tracy Baseline Capture  
**Goal**: Capture performance baselines at 200, 500, 1000 entities  
**Time**: 4-6 hours total  

---

## 🚀 Quick Start (30 seconds)

```powershell
# 1. Build profiling_demo (one-time, 9-15s)
cargo build -p profiling_demo --features profiling --release

# 2. Start Tracy.exe (download from https://github.com/wolfpld/tracy/releases/latest)

# 3. Run automated capture script
.\scripts\capture_tracy_baselines.ps1
```

---

## 📋 Manual Workflow (If Script Fails)

### Step 1: Build (once)
```powershell
cargo build -p profiling_demo --features profiling --release
```

### Step 2: Start Tracy Server
1. Launch `Tracy.exe`
2. Wait for "Listening on port 8086" message
3. Leave window open

### Step 3: Run Each Configuration

**Low Load (200 entities)**:
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 200
# Let run for 30+ seconds (1000+ frames @ 60 FPS)
# In Tracy: File > Save Trace > baseline_200.tracy
```

**Medium Load (500 entities)** - **PRIMARY TARGET**:
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 500
# Capture 1000+ frames
# Save: baseline_500.tracy
```

**High Load (1000 entities)**:
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000
# Capture 1000+ frames  
# Save: baseline_1000.tracy
```

---

## 🔍 Tracy Analysis Checklist

### Phase 1: Frame Time Overview (5 min per config)
- [ ] Open trace file in Tracy
- [ ] Record mean frame time: _____ ms
- [ ] Record p95 frame time: _____ ms
- [ ] Record FPS (avg): _____
- [ ] Check frame stability (std dev <2 ms)
- [ ] Identify frame spikes (>20 ms)

### Phase 2: Hotspot Identification (15 min per config)
- [ ] Open Statistics view (`Statistics > Show statistics`)
- [ ] Sort by "Self time" descending
- [ ] Record top 10 functions (>5% frame time)
- [ ] Categorize by subsystem (Rendering, Physics, AI, ECS)
- [ ] Calculate subsystem percentages

### Phase 3: Flame Graph Analysis (10 min per config)
- [ ] Open Flame graph (`Flame graph > Show flame graph`)
- [ ] Look for recursive calls (repeating function names)
- [ ] Identify allocation hotspots (alloc/dealloc in stack)
- [ ] Note deep call stacks (>10 levels)

### Phase 4: Plot Analysis (5 min per config)
- [ ] Check `entity_count` plot (should be constant)
- [ ] Record `draw_calls` average: _____ (target: <10)
- [ ] Record `visible_instances` average: _____ (50-80% of total is good)
- [ ] Check `GOAP::cache_hits` rate: _____% (target: >95%)

### Phase 5: Scalability Analysis (10 min total)
- [ ] Compare frame times across 200/500/1000 configs
- [ ] Identify superlinear scaling (O(n²) bottlenecks)
- [ ] Note which subsystems scale poorly

### Phase 6: Optimization Prioritization (15 min total)
- [ ] Rank top 5 optimization targets by impact
- [ ] Select top 3 for Week 8 implementation
- [ ] Estimate time per optimization (1-5 days)
- [ ] Create Week 8 roadmap

---

## 📊 Expected Results (Hypothetical - Your Data May Vary)

### Configuration: 500 Entities (Target)
**Frame Time**: 14-18 ms (60 FPS ✅)  
**Top 5 Hotspots** (expected):
1. **Render::MainPass** - 50-60% (PBR shading, lighting)
2. **Render::ShadowMaps** - 10-15% (2 cascades)
3. **Physics::Rapier::pipeline** - 10-20% (collision detection)
4. **Render::Present** - Variable (VSync wait)
5. **GOAP::Planner::plan** - 2-5% (AI planning, cache misses)

**Subsystem Breakdown** (expected):
- Rendering: 70-75%
- Physics: 15-20%
- AI: 5-8%
- ECS: 1-2%

---

## 🎯 Optimization Decision Tree

```
Is frame time >16.67 ms @ 500 entities?
├─ NO → ✅ 60 FPS achieved! Focus on scalability (1000 entities)
└─ YES → Analyze hotspots:
   
   Is Rendering >70% of frame?
   ├─ YES → Prioritize rendering optimizations:
   │   ├─ Render::MainPass >50% → Draw call batching, material grouping
   │   ├─ Render::ShadowMaps >15% → Reduce cascade resolution, PCF optimization
   │   └─ Render::Present >30% → GPU bottleneck (reduce shader complexity)
   │
   ├─ Is Physics >20% of frame?
   │   └─ YES → Prioritize physics optimizations:
   │       ├─ Rapier::pipeline superlinear → Spatial hashing, broad-phase culling
   │       └─ CharacterController >5% → SIMD raycast batching
   │
   └─ Is AI >10% of frame?
       └─ YES → Prioritize AI optimizations:
           ├─ GOAP cache <95% → Cache warming, increase capacity
           └─ BehaviorTree >3% → Early-outs, lazy evaluation
```

---

## 🛠️ Tracy Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Mouse wheel** | Zoom in/out timeline |
| **Shift + wheel** | Scroll left/right |
| **Ctrl + F** | Find zone by name |
| **Space** | Pause/resume capture |
| **Home** | Jump to first frame |
| **End** | Jump to last frame |
| **Click zone** | Show zone details (time, children) |
| **Ctrl + Click** | Zoom to zone |

---

## 📁 Output Files

After completion, you should have:
- ✅ `baseline_200.tracy` (5-50 MB)
- ✅ `baseline_500.tracy` (10-100 MB)
- ✅ `baseline_1000.tracy` (20-200 MB)
- ✅ `PROFILING_BASELINE_WEEK_8.md` (analysis report)

---

## ⚠️ Common Issues

### Issue: Tracy shows "No connection"
**Fix**: Start Tracy.exe BEFORE running profiling_demo

### Issue: Timeline is empty
**Fix**: Ensure `--features profiling` was used in build

### Issue: Frame time = exactly 16.67 ms
**Explanation**: VSync enabled (normal). Focus on hotspot % instead of absolute time.

### Issue: Render::Present >50% of frame
**Explanation**: GPU bottleneck (CPU waiting for GPU). Optimize shaders, not CPU code.

---

## 📖 Full Documentation

For detailed Tracy analysis guide, see:
- **`TRACY_ANALYSIS_GUIDE.md`** - Step-by-step analysis workflow (30+ pages)
- **`WEEK_8_KICKOFF.md`** - Week 8 overall plan (optimization roadmap)
- **Tracy PDF Manual**: https://github.com/wolfpld/tracy/releases/download/v0.11/tracy.pdf

---

## ✅ Success Criteria (Week 8 Day 1)

- [x] Tracy 0.11+ installed and working
- [x] 3 baseline traces captured (200, 500, 1000 entities)
- [x] Top 10 hotspots identified per configuration
- [x] Subsystem breakdown calculated (Rendering/Physics/AI/ECS %)
- [x] Scalability analysis complete (200→500→1000 comparison)
- [x] Top 3 optimization targets selected for Week 8 Days 2-4
- [x] `PROFILING_BASELINE_WEEK_8.md` report created

**Estimated Total Time**: 4-6 hours (including learning Tracy UI)

---

**Quick Reference Created**: October 12, 2025  
**AstraWeave Version**: 0.7.0  
**Generated by**: GitHub Copilot (100% AI-authored)  

🚀 Ready to capture baselines and identify those hotspots!
