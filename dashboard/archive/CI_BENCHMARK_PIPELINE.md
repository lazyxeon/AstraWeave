# CI Benchmark Pipeline Documentation

**Week 3 Action 11 Complete**: Automated performance regression detection system for AstraWeave engine.

---

## Overview

The CI Benchmark Pipeline automatically:
1. **Runs all benchmarks** on every PR and push to main/develop
2. **Validates results** against baseline thresholds
3. **Detects regressions** before they reach production
4. **Comments on PRs** with performance impact
5. **Tracks trends** via GitHub Pages dashboard

---

## Components

### 1. GitHub Workflow (`.github/workflows/benchmark.yml`)

**Triggers**:
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`
- Manual dispatch (`workflow_dispatch`)

**Key Steps**:
1. **Setup**: Install Rust, system dependencies, sccache
2. **Run Benchmarks**: Execute `benchmark-runner.sh` for all packages
3. **Validate Results**: Basic JSON validation + threshold checking
4. **Upload Artifacts**: Store results for 30 days
5. **Track Performance**: Update GitHub Pages dashboard (main branch only)
6. **PR Comments**: Post performance comparison on pull requests

**Concurrency**: Cancels in-progress runs for the same ref (saves CI time)

**Timeout**: 45 minutes (prevents stuck jobs)

---

### 2. Benchmark Runner Script (`.github/scripts/benchmark-runner.sh`)

**Packages Benchmarked** (Week 2-3):
- `astraweave-core` - ECS performance (Action 2)
- `astraweave-input` - Input system (Week 1 baseline)
- `astraweave-ai` - AI core loop (Action 4)
- `astraweave-behavior` - GOAP planning + caching (Actions 3, 9)
- `astraweave-stress-test` - Large-scale ECS stress (Action 2)
- `astraweave-terrain` - Terrain generation + streaming (Action 8)

**Features**:
- **Auto-discovery**: Finds additional packages with `benches/` directories
- **Parallel execution**: Runs benchmarks per package
- **JSON output**: Compatible with `github-action-benchmark`
- **Error handling**: Continues on failure, reports summary
- **Timeout protection**: 10-minute timeout per package (600 seconds)

**Output Files**:
- `benchmark_results/benchmarks.json` - Criterion results in JSON format
- `benchmark_results/summary.txt` - Human-readable summary
- `benchmark_results/<pkg>_stdout.log` - Per-package stdout logs
- `benchmark_results/<pkg>_stderr.log` - Per-package stderr logs

---

### 3. Threshold Validation Script (`scripts/check_benchmark_thresholds.ps1`)

**Purpose**: Validate benchmark results against performance baselines.

**Usage**:
```powershell
# Basic validation (warnings only)
.\scripts\check_benchmark_thresholds.ps1 -Verbose

# Strict mode (fail on regressions)
.\scripts\check_benchmark_thresholds.ps1 -Strict

# Update baseline from current results
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline

# Custom paths
.\scripts\check_benchmark_thresholds.ps1 `
    -BenchmarkJsonPath "custom/benchmarks.json" `
    -ThresholdsJsonPath "custom/thresholds.json"
```

**Parameters**:
- `-BenchmarkJsonPath` - Path to benchmark results JSON (default: `benchmark_results/benchmarks.json`)
- `-ThresholdsJsonPath` - Path to thresholds JSON (default: `.github/benchmark_thresholds.json`)
- `-Strict` - Exit with error code 1 on regressions (default: warnings only)
- `-UpdateBaseline` - Update threshold file with current results
- `-Verbose` - Show detailed per-benchmark analysis

**Output**:
```
=== AstraWeave Benchmark Threshold Validation ===
📊 Loading benchmark results from: benchmark_results/benchmarks.json
✅ Loaded 21 benchmark results
📋 Loading thresholds from: .github/benchmark_thresholds.json
✅ Loaded 21 threshold definitions

=== Validating Benchmark Results ===
✅ astraweave-ai::ai_core_loop/ai_core_loop_simple: 180.50 ns (✓ PASS, -1.9% change)
⚠️  astraweave-terrain::terrain_generation/world_chunk_streaming: 16.20 ms (⚠ WARN, +7.6% above baseline, under limit)
❌ astraweave-behavior::goap_planning/goap_planning_20_actions: 40.50 ms (❌ FAIL, 6.3% OVER LIMIT)

=== Validation Summary ===
Total benchmarks: 21
✅ Passed: 19
❌ Failed: 1
🆕 New: 1

=== Performance Regressions Detected ===
| Benchmark | Current | Baseline | Max Allowed | Over Limit |
|-----------|---------|----------|-------------|------------|
| goap_planning_20_actions | 40.50 ms | 25.40 ms | 38.10 ms | 6.3% |
```

**Exit Codes**:
- `0` - All benchmarks passed (or warnings in non-strict mode)
- `1` - Regressions detected (strict mode only)

---

### 4. Baseline Thresholds (`.github/benchmark_thresholds.json`)

**Structure**:
```json
{
  "version": "1.0",
  "benchmarks": {
    "benchmark_name": {
      "baseline": 1000.0,        // Baseline performance (ns)
      "max_allowed": 1500.0,     // Maximum allowed (50% regression)
      "warn_threshold": 1250.0,  // Warning threshold (25% regression)
      "unit": "ns",
      "description": "...",
      "created": "2025-10-08T00:00:00Z",
      "last_updated": "2025-10-09T00:00:00Z",
      "critical": true           // Optional: Critical benchmark flag
    }
  }
}
```

**Policy**:
- **Default max regression**: 50% slower than baseline
- **Default warn threshold**: 25% slower than baseline
- **Critical benchmarks**: Enforce stricter limits (documented separately)

**Current Baselines** (21 benchmarks):
1. **ECS Performance** (Week 2 Action 2):
   - World creation: 25.8 ns
   - Entity spawn: 420 ns
   - Entity tick: 0.95 ns (sub-nanosecond!)
   - 10K entities stress: 5 ms

2. **AI Planning** (Week 2 Action 3 + Week 3 Action 9):
   - Simple GOAP: 6.05 µs
   - Moderate GOAP: 3.65 ms
   - Complex GOAP: 25.4 ms
   - **Cache hit: 1.01 µs** ✅ (97.9% faster than 47.2 µs miss)
   - **Warm cache (90% hit): 737 ns** ✅
   - Simple BT: 57 ns
   - Complex BT: 253 ns

3. **AI Core Loop** (Week 2 Action 4):
   - Simple: 184 ns ✅ (2500x faster than 5ms target!)
   - Moderate: 736 ns
   - Complex: 2.1 µs

4. **Terrain Generation** (Week 3 Action 8):
   - 64×64 chunk: 1.98 ms
   - **World chunk streaming: 15.06 ms** ✅ (< 16.67ms for 60 FPS)

5. **Input System** (Week 1 baseline):
   - Binding creation: 4.67 ns (sub-5ns!)
   - Binding lookup: 12.3 ns
   - Full set processing: 1.03 µs

**Critical Benchmarks** (must not regress):
- `ai_core_loop_simple` - Real-time AI requirement
- `goap_caching_warm_cache_90pct` - Realistic cache scenario
- `goap_cache_comparison/cache_hit` - Cache effectiveness
- `world_chunk_streaming` - 60 FPS streaming target

---

## Workflow Logic

### Pull Request Flow

```
1. PR opened/updated
   ↓
2. Benchmark workflow triggered
   ↓
3. Run all benchmarks (6 packages)
   ↓
4. Validate JSON structure
   ↓
5. Check thresholds (non-strict)
   ↓
6. Post comment on PR with:
   - Performance comparison vs baseline
   - Warnings for regressions
   - Link to detailed results
```

**Example PR Comment**:
> 📊 **Performance Benchmark Results**
> 
> | Benchmark | Current | Baseline | Change |
> |-----------|---------|----------|--------|
> | ai_core_loop_simple | 180 ns | 184 ns | ✅ -2.2% |
> | goap_cache_hit | 1.05 µs | 1.01 µs | ⚠️ +4.0% |
> | world_chunk_streaming | 14.50 ms | 15.06 ms | ✅ -3.7% |
> 
> **Summary**: 3 benchmarks passed, 0 regressions detected.
> 
> [View detailed results](link-to-artifact)

### Main Branch Flow

```
1. Push to main
   ↓
2. Benchmark workflow triggered
   ↓
3. Run all benchmarks (6 packages)
   ↓
4. Validate JSON structure
   ↓
5. Check thresholds (STRICT MODE)
   ↓
6. Update GitHub Pages dashboard
   ↓
7. Fail build if regressions detected
```

**Strict Mode**:
- **Enforces max_allowed thresholds**
- **Blocks merge** if critical benchmarks regress
- **Sends alerts** to configured users (e.g., @lazyxeon)

---

## Usage Guide

### For Developers

**Running Benchmarks Locally**:
```powershell
# Run all benchmarks
cargo bench

# Run specific package
cargo bench -p astraweave-behavior

# Run specific benchmark
cargo bench -p astraweave-behavior --bench goap_planning

# Validate against thresholds
cargo bench -p astraweave-ai
.\scripts\check_benchmark_thresholds.ps1 -Verbose
```

**Before Submitting PR**:
1. Run benchmarks for affected systems
2. Check threshold validation locally
3. If regressions detected:
   - Investigate root cause
   - Optimize if possible
   - Document intentional trade-offs in PR description

**Updating Baselines** (with approval):
```powershell
# Run benchmarks
cargo bench

# Update baseline thresholds
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline

# Commit updated .github/benchmark_thresholds.json
git add .github/benchmark_thresholds.json
git commit -m "chore: Update benchmark baselines after optimization"
```

**When to Update Baselines**:
- ✅ After approved optimization work (e.g., Week 3 Action 8, 9)
- ✅ When new benchmarks added
- ✅ After intentional architectural changes with documented trade-offs
- ❌ To "fix" failing CI without investigation
- ❌ Without team review/approval

---

### For Reviewers

**Interpreting Benchmark Comments**:
1. **All green (✅)**: Performance maintained or improved
2. **Warnings (⚠️)**: Slight regression (< 50% slower), investigate
3. **Failures (❌)**: Significant regression (> 50% slower), **requires fix or explanation**

**Questions to Ask**:
- Is the regression expected for this change?
- Is there a trade-off (e.g., correctness vs speed)?
- Can the regression be mitigated?
- Should baseline be updated (with justification)?

**Review Checklist**:
- [ ] Benchmark results posted in PR
- [ ] No unexpected regressions (or explained)
- [ ] Critical benchmarks remain performant
- [ ] Baseline updates justified (if any)

---

## Maintenance

### Adding New Benchmarks

**1. Create Benchmark File** (e.g., `astraweave-physics/benches/raycast.rs`):
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_raycast_simple(c: &mut Criterion) {
    c.bench_function("raycast_simple", |b| {
        b.iter(|| {
            // Benchmark code
        })
    });
}

criterion_group!(benches, bench_raycast_simple);
criterion_main!(benches);
```

**2. Register in Cargo.toml**:
```toml
[[bench]]
name = "raycast"
harness = false
```

**3. Run Locally**:
```powershell
cargo bench -p astraweave-physics --bench raycast
```

**4. Update Baseline** (first run):
```powershell
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline
```

**5. Commit Changes**:
```powershell
git add astraweave-physics/benches/raycast.rs
git add astraweave-physics/Cargo.toml
git add .github/benchmark_thresholds.json
git commit -m "feat: Add raycast benchmarks (Week 3 Action 12)"
```

**Auto-Discovery**: The `benchmark-runner.sh` script will automatically find and run new benchmarks!

---

### Adjusting Thresholds

**Scenario 1: Benchmark Too Strict** (false positives)
```json
{
  "baseline": 1000,
  "max_allowed": 1200,  // Reduce from 1500 (20% regression vs 50%)
  "warn_threshold": 1100  // Warn at 10%
}
```

**Scenario 2: Critical Benchmark** (must not regress)
```json
{
  "baseline": 15060000,
  "max_allowed": 16670000,  // Tight limit (< 16.67ms for 60 FPS)
  "warn_threshold": 16000000,
  "critical": true
}
```

**Scenario 3: New Optimization** (update baseline)
```powershell
# After Action 9 (GOAP caching), update baseline:
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline
```

---

### Troubleshooting

**Problem**: Benchmarks fail in CI but pass locally
- **Cause**: CI environment variability (different CPU, load, etc.)
- **Solution**: Increase `max_allowed` threshold by 10-20% for noisy benchmarks

**Problem**: New benchmarks not discovered
- **Cause**: Missing `[[bench]]` in `Cargo.toml` or no `benches/` directory
- **Solution**: Check package structure, ensure `harness = false` in Cargo.toml

**Problem**: Threshold validation fails with "file not found"
- **Cause**: PowerShell not installed on Linux CI runner
- **Solution**: Workflow includes PowerShell installation step (already handled)

**Problem**: JSON validation fails
- **Cause**: Criterion output format change or empty results
- **Solution**: Check `benchmark_results/<pkg>_stderr.log` for errors

**Problem**: Baseline update not working
- **Cause**: Permission issues or invalid JSON structure
- **Solution**: Run `.\scripts\check_benchmark_thresholds.ps1 -Verbose` to see detailed error

---

## Performance Tracking Dashboard

### Week 4 Action 15: Enhanced Dashboard

**Production Dashboard**: `https://lazyxeon.github.io/AstraWeave-AI-Native-Gaming-Engine/dashboard/benchmark_dashboard/`

**Week 3 Legacy Dashboard**: `https://lazyxeon.github.io/AstraWeave-AI-Native-Gaming-Engine/dev/bench/` (auto-generated by github-action-benchmark)

#### New Features (Action 15)

**Interactive d3.js Dashboard** (`docs/benchmark_dashboard/index.html`):
- ✨ **30-day performance trends** with customizable time ranges
- 📊 **Multi-benchmark comparison** with interactive legend
- 🎯 **Threshold visualization** (red dashed lines show max limits)
- 📈 **Moving average smoothing** (3/5/7-point options)
- 🖱️ **Hover tooltips** with commit details, dates, values
- 🎨 **Color-coded lines** per benchmark with toggle on/off
- 📱 **Responsive design** works on mobile/desktop

**Data Export System**:
- **Script**: `scripts/export_benchmark_history.sh`
- **Format**: JSONL (JSON Lines) for efficient parsing
- **Metadata**: `docs/benchmark_data/metadata.json` with stats
- **Auto-generation**: Runs on every main branch push

**Automated Regression Alerts**:
- **Workflow**: `.github/workflows/benchmark_alert.yml`
- **Triggers**: Push to `main`, manual dispatch
- **Issue Creation**: Auto-creates GitHub issues on >10% regression
- **Smart Deduplication**: Prevents spam (max 1 issue per 24 hours)
- **Labels**: `performance-regression`, `automated`, `high-priority`
- **Assignees**: Auto-assigns `@lazyxeon`
- **Details**: Includes benchmark values, thresholds, % over limit

**Alert Configuration**:
- **Detection Threshold**: Uses `benchmark_thresholds.json` (50% default)
- **Issue Threshold**: >10% regression creates GitHub issue
- **Notifications**: Comments on commit + mentions `@lazyxeon`
- **Fail on alert**: `true` in strict mode (main branch only)
- **Historical tracking**: gh-pages branch stores all results

#### Dashboard Usage

**Accessing the Dashboard**:
```bash
# Local development (requires exported data)
bash scripts/export_benchmark_history.sh
open docs/benchmark_dashboard/index.html

# Or view production dashboard
open https://lazyxeon.github.io/AstraWeave-AI-Native-Gaming-Engine/dashboard/benchmark_dashboard/
```

**Dashboard Controls**:
- **Benchmark Selector**: View all benchmarks or filter by name
- **Time Range**: 1-365 days (default: 30 days)
- **Smoothing**: None, 3-point, 5-point, 7-point moving average
- **Legend**: Click benchmark names to show/hide lines

**Data Flow**:
```
1. Benchmarks run on main branch
   ↓
2. Results posted to gh-pages (github-action-benchmark)
   ↓
3. Export script converts to JSONL (benchmark_alert.yml workflow)
   ↓
4. Dashboard deployed to GitHub Pages
   ↓
5. Users view interactive trends at dashboard URL
```

**Legacy Dashboard (Week 3)**:
- **Tool**: github-action-benchmark (auto-generated)
- **Threshold**: 200% regression (very conservative)
- **Format**: Static HTML with basic charts
- **Location**: `dev/bench/` subdirectory

---

## Week 3 Action 11 Achievements

### What Was Created

1. ✅ **Threshold Validation Script** (`scripts/check_benchmark_thresholds.ps1`)
   - 350 lines of PowerShell
   - Colorized output, verbose mode
   - Baseline update functionality
   - Strict mode for CI enforcement

2. ✅ **Baseline Thresholds** (`.github/benchmark_thresholds.json`)
   - 21 benchmarks from Week 2-3
   - Conservative 50% regression limits
   - Critical benchmark flagging
   - Detailed metadata and descriptions

3. ✅ **Enhanced Workflow** (`.github/workflows/benchmark.yml`)
   - Integrated threshold validation
   - Non-strict PR validation (warnings)
   - Strict main branch validation (blocking)
   - PowerShell installation on Linux

4. ✅ **Updated Runner Script** (`.github/scripts/benchmark-runner.sh`)
   - Added 4 new benchmark packages (ai, behavior, stress-test, terrain)
   - Auto-discovery for future packages
   - Comprehensive error handling

5. ✅ **Documentation** (this file)
   - Complete usage guide
   - Maintenance procedures
   - Troubleshooting section

### Performance Targets Protected

**Critical Metrics**:
- ✅ **AI Core Loop**: < 5ms target (achieved 184ns, 2500x faster!)
- ✅ **GOAP Cache Hit**: < 1ms target (achieved 1.01µs, 1000x faster!)
- ✅ **Terrain Streaming**: < 16.67ms for 60 FPS (achieved 15.06ms)
- ✅ **Behavior Tree**: < 1µs for 60K agents (achieved 57ns, 17x faster!)

**Regression Detection**:
- Prevents accidental performance loss
- Catches regressions before merge
- Tracks trends over time

### Impact

**Before Action 11**:
- Manual benchmark comparison
- No automated regression detection
- Performance improvements could regress silently

**After Action 11**:
- ✅ **Automated validation** on every PR
- ✅ **Strict enforcement** on main branch
- ✅ **Historical tracking** via GitHub Pages
- ✅ **Alert system** for critical regressions
- ✅ **Developer-friendly** with local validation tools

---

## Future Enhancements

### Short-Term (Month 1)
1. **Per-crate thresholds**: Different regression limits for different systems
2. **Regression tracking**: Auto-create issues for persistent regressions
3. **Benchmark comparison**: Compare PR vs main branch directly

### Medium-Term (Month 3)
1. **Performance budgets**: System-level budgets (e.g., total AI < 10ms)
2. **Flamegraph integration**: Automated profiling on regressions
3. **Multi-platform benchmarks**: Test on Windows, macOS, Linux

### Long-Term (Month 6)
1. **Machine learning**: Detect anomalies, predict performance trends
2. **A/B testing**: Compare optimization strategies automatically
3. **Distributed benchmarking**: Run benchmarks on dedicated hardware

---

**Last Updated**: October 9, 2025  
**Status**: ✅ PRODUCTION READY  
**Maintainer**: GitHub Copilot (AI-Native Development Experiment)
