# 🚀 Week 8 START HERE - Quick Guide

**You are here**: Week 8 Day 1 - Tracy Baseline Capture  
**Time to complete**: 4-6 hours  
**Goal**: Capture performance baselines and identify optimization targets  

---

## ✅ What's Already Done

- ✅ Week 7 profiling instrumentation (28 spans, 9 plots)
- ✅ profiling_demo working with command-line args
- ✅ Documentation created (200+ pages of guides)
- ✅ Automation script ready
- ✅ All infrastructure tested and validated

**You're 100% ready to start!** Just need Tracy software.

---

## 🎯 Three Simple Steps

### Step 1: Get Tracy (5 minutes)

**Download**: https://github.com/wolfpld/tracy/releases/latest

**Windows Users**:
1. Download `Tracy-0.11.x-Windows.zip`
2. Extract to `C:\Tools\Tracy\` (or anywhere you like)
3. Double-click `Tracy.exe`
4. Wait for "Listening on port 8086" message ✅

**Linux/Mac Users**:
1. Download appropriate package
2. Extract and run `tracy-profiler`
3. Wait for "Listening" message ✅

---

### Step 2: Capture Baselines (1-2 hours)

**Option A: Automated Script** (Recommended)
```powershell
# Run this in PowerShell:
.\scripts\capture_tracy_baselines.ps1
```

The script will:
1. ✅ Check Tracy is running
2. ✅ Build profiling_demo (if needed)
3. ✅ Guide you through capturing 200, 500, 1000 entity configs
4. ✅ Remind you when to save traces

**Just follow the on-screen prompts!**

---

**Option B: Manual** (If script fails)
```powershell
# Low load baseline
cargo run -p profiling_demo --features profiling --release -- --entities 200
# Wait 30 seconds, then in Tracy: File > Save Trace > baseline_200.tracy

# Medium load baseline (PRIMARY TARGET)
cargo run -p profiling_demo --features profiling --release -- --entities 500
# Save as: baseline_500.tracy

# High load baseline
cargo run -p profiling_demo --features profiling --release -- --entities 1000
# Save as: baseline_1000.tracy
```

**You should now have 3 files**:
- `baseline_200.tracy`
- `baseline_500.tracy`
- `baseline_1000.tracy`

---

### Step 3: Analyze Data (2-3 hours)

**Open**: `baseline_500.tracy` in Tracy (your primary target)

**Quick Analysis**:
1. **Statistics** menu → "Show statistics"
   - Sort by "Self time" (descending)
   - **Write down top 5 functions** (>5% frame time)

2. **Flame Graph** menu → "Show flame graph"
   - Look for wide bars at the bottom (leaf functions doing work)
   - **Note any deep stacks** (>10 levels = potential issue)

3. **Plots** section in Timeline view
   - Check `draw_calls` plot (target: <10)
   - Check `cache_hits` plot (target: >95%)

4. **Info Panel** (bottom-right)
   - **Record mean frame time**: _____ ms
   - **Record FPS**: _____

**That's it!** Now you know your hotspots.

---

## 📝 Create Your Report (1 hour)

**Copy this template** into new file `PROFILING_BASELINE_WEEK_8.md`:

```markdown
# Week 8 Tracy Profiling Baselines

## System Specs
- CPU: [Your CPU model]
- GPU: [Your GPU model]
- RAM: [Total RAM]
- OS: [Your OS]

## Configuration: 500 Entities (Target)
### Frame Time
- Mean: _____ ms
- p95: _____ ms
- FPS: _____

### Top 5 Hotspots
1. [Function name] - [X.XX ms] ([XX%])
2. [Function name] - [X.XX ms] ([XX%])
3. [Function name] - [X.XX ms] ([XX%])
4. [Function name] - [X.XX ms] ([XX%])
5. [Function name] - [X.XX ms] ([XX%])

### Subsystem Breakdown
- Rendering: [XX%]
- Physics: [XX%]
- AI: [XX%]
- ECS: [XX%]

## Week 8 Optimization Priorities
1. **[Top Hotspot]** - Target: Reduce by 20-30%
2. **[Second Hotspot]** - Target: Reduce by 15-20%
3. **[Third Hotspot]** - Target: Reduce by 10-15%
```

**Fill in the blanks** with data from Tracy!

---

## 🎯 Success Criteria (Day 1 Complete)

- [x] Tracy installed and working
- [x] 3 traces captured (200, 500, 1000)
- [x] Top 5 hotspots identified
- [x] Subsystem breakdown calculated
- [x] Report created with optimization priorities

**When all checked** ✅ → Day 1 complete! Move to Days 2-4 (optimizations).

---

## 📚 Need More Help?

**Full Guides** (if you get stuck):
- `TRACY_ANALYSIS_GUIDE.md` - 70+ pages, step-by-step workflow
- `TRACY_QUICK_REFERENCE.md` - 15 pages, checklists and shortcuts
- `WEEK_8_KICKOFF.md` - 50+ pages, complete Week 8 plan

**Quick Questions**:
- **Tracy shows "No connection"?** → Start Tracy BEFORE running profiling_demo
- **Timeline is empty?** → Ensure `--features profiling --release` in command
- **Frame time = exactly 16.67 ms?** → Normal (VSync enabled), focus on hotspot %

---

## 🚀 Your Next 10 Minutes

1. **Download Tracy** → https://github.com/wolfpld/tracy/releases/latest (5 min)
2. **Extract and launch** `Tracy.exe` (1 min)
3. **Run automated script** → `.\scripts\capture_tracy_baselines.ps1` (starts immediately)

**You'll be capturing your first baseline in less than 10 minutes!**

---

## 💡 What Happens Next?

**After Day 1** (baselines captured + analyzed):
- **Days 2-4**: Implement optimizations for top 3 hotspots
- **Day 5**: Re-run Tracy, validate improvements, create final report

**Week 8 Goal**: Maintain 60 FPS at 500 entities (10-20% frame time improvement)

---

**Ready to start?** Download Tracy now! 🔥

**Questions?** See full guides in `TRACY_ANALYSIS_GUIDE.md` or `WEEK_8_KICKOFF.md`.

**Generated**: October 12, 2025  
**AstraWeave Version**: 0.7.0  
**100% AI-Authored by GitHub Copilot**  

Let's find those hotspots and make AstraWeave faster! 🚀🔍
