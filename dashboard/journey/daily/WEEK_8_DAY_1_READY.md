# Week 8 Day 1 Complete: Tracy Baseline Capture Ready

**Date**: October 12, 2025  
**Phase**: Phase B - Month 4 Week 8  
**Status**: ✅ Ready to capture Tracy baselines  
**Time to Start**: <5 minutes  

---

## 🎉 Preparation Complete

All Week 8 Day 1 infrastructure is ready for Tracy baseline capture:

### ✅ Deliverables Created
1. **`WEEK_8_KICKOFF.md`** (50+ pages) - Complete Week 8 plan with optimization scenarios
2. **`TRACY_ANALYSIS_GUIDE.md`** (70+ pages) - Step-by-step Tracy profiling guide
3. **`TRACY_QUICK_REFERENCE.md`** (15 pages) - Quick reference card for Tracy workflow
4. **`scripts/capture_tracy_baselines.ps1`** - Automated baseline capture script
5. **profiling_demo** - Enhanced with command-line argument support

### ✅ Code Updates
**File**: `examples/profiling_demo/src/main.rs`  
**Changes**:
- Added `parse_args()` function for command-line argument parsing
- Support for `--entities` / `-e` flag (200, 500, 1000)
- Support for `--frames` / `-f` flag (default: 1000)
- `--help` / `-h` displays comprehensive usage guide
- Updated output to show configuration and save instructions

**Compilation**: ✅ Verified (0.94s, 1 warning - dead code in RigidBody struct)

---

## 🚀 Quick Start (Next Steps)

### Option 1: Automated Script (Recommended)
```powershell
# Run automated capture workflow
.\scripts\capture_tracy_baselines.ps1

# Or single configuration:
.\scripts\capture_tracy_baselines.ps1 -EntityCount 500
```

**What It Does**:
1. Checks Tracy installation
2. Builds profiling_demo (if needed)
3. Guides you through capturing each configuration (200, 500, 1000 entities)
4. Reminds you to save traces at the right time

---

### Option 2: Manual Workflow

#### Step 1: Download Tracy (One-Time)
**URL**: https://github.com/wolfpld/tracy/releases/latest  
**Download**: `Tracy-0.11.x-Windows.zip` (or your platform)  
**Extract to**: `C:\Tools\Tracy\` (or update script with `-TracyPath`)  

---

#### Step 2: Build profiling_demo (One-Time)
```powershell
cargo build -p profiling_demo --features profiling --release
# First build: 9-15 seconds
# Subsequent: Cached (instant)
```

---

#### Step 3: Start Tracy Server
1. Launch `Tracy.exe`
2. Wait for "Listening on port 8086" message
3. Leave window open during profiling

---

#### Step 4: Capture Baselines (Repeat for Each Config)

**Configuration 1: Low Load (200 entities)**
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 200

# Output shows:
# Configuration: 200 entities, 1000 frames (~16.7s @ 60 FPS)
# Tracy auto-connects and captures profiling data
# Wait for completion message

# In Tracy:
# File > Save Trace > baseline_200.tracy
```

**Configuration 2: Medium Load (500 entities) - PRIMARY TARGET**
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 500
# Save: baseline_500.tracy
```

**Configuration 3: High Load (1000 entities)**
```powershell
cargo run -p profiling_demo --features profiling --release -- --entities 1000
# Save: baseline_1000.tracy
```

---

#### Step 5: Analyze Tracy Data
**Open**: `baseline_500.tracy` (target capacity)  
**Views**:
- **Statistics** → Sort by "Self time" → Record top 10 hotspots
- **Flame Graph** → Identify call hierarchy and allocation patterns
- **Plots** → Check `draw_calls`, `visible_instances`, `cache_hits`
- **Timeline** → Inspect frame spikes and subsystem timing

**Refer to**: `TRACY_ANALYSIS_GUIDE.md` for detailed workflow (6 phases)

---

#### Step 6: Create Baseline Report
**File**: `PROFILING_BASELINE_WEEK_8.md`  
**Template**: See `TRACY_ANALYSIS_GUIDE.md` → "Data Collection Template"  
**Contents**:
- System specifications (CPU, GPU, RAM)
- Frame time statistics (mean, p95, p99, FPS)
- Top 10 hotspots per configuration
- Subsystem breakdown (Rendering/Physics/AI/ECS %)
- Scalability analysis (200 → 500 → 1000)
- Optimization priorities for Week 8 Days 2-4

---

## 📖 Documentation Quick Links

| Document | Purpose | Pages |
|----------|---------|-------|
| **`WEEK_8_KICKOFF.md`** | Overall Week 8 plan, optimization scenarios | 50+ |
| **`TRACY_ANALYSIS_GUIDE.md`** | Step-by-step Tracy profiling workflow | 70+ |
| **`TRACY_QUICK_REFERENCE.md`** | Quick reference card, shortcuts, checklists | 15 |
| **`scripts/capture_tracy_baselines.ps1`** | Automated capture script | - |

---

## 🎯 Expected Timeline (Week 8 Day 1)

| Task | Estimated Time | Status |
|------|---------------|--------|
| Download Tracy | 5 min | ⏳ Next |
| Build profiling_demo | 9-15s (already done) | ✅ Complete |
| Capture 200 entities | 20-30 min | ⏳ Next |
| Capture 500 entities | 20-30 min | ⏳ Next |
| Capture 1000 entities | 20-30 min | ⏳ Next |
| Analyze Tracy data | 1-1.5h | ⏳ Next |
| Create baseline report | 1-1.5h | ⏳ Next |
| **Total** | **4-6 hours** | - |

---

## ✅ Success Criteria (Day 1 Complete)

- [ ] Tracy 0.11+ installed and working
- [ ] 3 baseline traces captured:
  - [ ] `baseline_200.tracy` (Low load)
  - [ ] `baseline_500.tracy` (Medium load - PRIMARY)
  - [ ] `baseline_1000.tracy` (High load)
- [ ] Top 10 hotspots identified per configuration
- [ ] Subsystem breakdown calculated (Rendering/Physics/AI/ECS %)
- [ ] Scalability analysis complete (linear vs superlinear vs sublinear)
- [ ] Top 3 optimization targets selected for Days 2-4
- [ ] `PROFILING_BASELINE_WEEK_8.md` report created

---

## 🔧 profiling_demo Command Reference

### Basic Usage
```powershell
# Default: 1000 entities, 1000 frames
cargo run -p profiling_demo --features profiling --release

# Custom entity count
cargo run -p profiling_demo --features profiling --release -- --entities 500

# Custom frame count (longer capture)
cargo run -p profiling_demo --features profiling --release -- --frames 2000

# Combined
cargo run -p profiling_demo --features profiling --release -- -e 500 -f 2000
```

### Help
```powershell
cargo run -p profiling_demo --features profiling -- --help
```

### Output Example
```
=== AstraWeave Profiling Demo ===
Tracy profiling enabled: true
Configuration:
  Entities: 500
  Frames: 1000 (~16.7s @ 60 FPS)

Start Tracy server before running for best results.
Tracy will auto-connect and capture profiling data.

Frame 0/1000
Frame 100/1000
...
Frame 900/1000

=== Profiling Complete ===
Configuration: 500 entities, 1000 frames
Total time: 14.52s
Average FPS: 68.87
Average frame time: 14.52ms

Check Tracy for detailed profiling data!
Save trace: File > Save Trace > baseline_500.tracy
```

---

## ⚠️ Common Issues & Fixes

### Issue: Tracy shows "No connection to profiled program"
**Cause**: Tracy started AFTER profiling_demo (missed connection window)  
**Fix**: Start Tracy.exe FIRST, wait for "Listening" message, THEN run profiling_demo

---

### Issue: Timeline is empty in Tracy
**Cause**: profiling_demo built without `--features profiling`  
**Fix**: Ensure build command includes `--features profiling --release`

---

### Issue: Frame time = exactly 16.67 ms (every frame)
**Explanation**: VSync enabled (normal). Profiling is working correctly.  
**Action**: Focus on hotspot % distribution instead of absolute time. Disable VSync only if testing GPU headroom.

---

### Issue: Render::Present >50% of frame time
**Explanation**: GPU bottleneck (CPU waiting for GPU to finish)  
**Action**: NOT a CPU optimization target. Optimize GPU shaders (reduce PBR complexity, lower shadow resolution).

---

## 🎯 Next Immediate Action

**Your Next Step**: Download Tracy (5 minutes)
```
1. Visit: https://github.com/wolfpld/tracy/releases/latest
2. Download: Tracy-0.11.x-Windows.zip
3. Extract to: C:\Tools\Tracy\
4. Launch: Tracy.exe
5. Verify: "Listening on port 8086" message appears
```

**After Tracy Installed**:
```powershell
# Run automated capture script
.\scripts\capture_tracy_baselines.ps1

# OR manual workflow:
cargo run -p profiling_demo --features profiling --release -- --entities 200
```

---

## 📊 Expected Results (Hypothetical)

### Frame Time (500 entities - target)
- **Mean**: 14-18 ms (60 FPS ✅)
- **p95**: 16-20 ms (target: <16.67 ms)
- **p99**: 18-25 ms

### Top 5 Hotspots (expected)
1. **Render::MainPass** - 50-60% (PBR shading, lighting)
2. **Render::ShadowMaps** - 10-15% (cascaded shadows)
3. **Physics::Rapier::pipeline** - 10-20% (collision detection)
4. **Render::Present** - Variable (VSync wait)
5. **GOAP::Planner::plan** - 2-5% (AI planning, cache misses)

### Subsystem Breakdown (expected)
- **Rendering**: 70-75%
- **Physics**: 15-20%
- **AI**: 5-8%
- **ECS**: 1-2%

**Note**: Your actual results may vary based on hardware. These are expectations to validate against.

---

## 🚀 Week 8 Roadmap After Day 1

**Day 1 (Today)**: Tracy baseline capture + analysis (4-6h)  
**Days 2-4**: Implement top 3 optimizations (12-16h)  
**Day 5**: Re-run Tracy, validate improvements, update baselines (4-6h)  

**Week 8 Goal**: Maintain 60 FPS at 500 entities (p95 <16.67ms), 10-20% frame time reduction

---

**Week 8 Day 1 Preparation Complete!** 🎉  
**Status**: Ready to capture Tracy baselines  
**Documentation**: 150+ pages of guides created  
**Infrastructure**: profiling_demo enhanced with CLI args  
**Time to First Baseline**: <30 minutes (after Tracy download)  

**Generated**: October 12, 2025  
**AstraWeave Version**: 0.7.0  
**100% AI-Authored by GitHub Copilot**  

Let's capture those baselines and find the hotspots! 🔍🚀
