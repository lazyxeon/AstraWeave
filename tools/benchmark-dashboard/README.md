# AstraWeave Benchmark Dashboard

**Live Dashboard**: https://lazyxeon.github.io/AstraWeave/ *(once deployed)*

Interactive performance telemetry dashboard for the AstraWeave AI-native game engine. Visualizes 30-day benchmark trends, detects regressions, and provides historical performance analysis.

---

## Features

### 📊 Interactive Visualizations
- **D3.js Line Charts**: Smooth, interactive trend lines for all benchmarks
- **System Filtering**: Filter by ECS, AI, Physics, Terrain, Input systems
- **Time Range Selection**: 7, 14, 30, or 60-day views
- **Drill-Down Views**: Select specific benchmarks for detailed analysis

### 🎯 Performance Metrics
- **Summary Cards**: Top 8 benchmarks with current values and % change
- **Trend Analysis**: Identify performance improvements and regressions over time
- **Multi-Series Charts**: Compare multiple benchmarks simultaneously
- **Responsive Design**: Works on desktop and mobile devices

### ⚠️ Regression Detection
- **Automated Alerts**: Nightly CI creates GitHub issues for >10% regressions
- **Threshold Validation**: Built-in checks against `benchmark_thresholds.json`
- **Historical Context**: 30-day data retention for trend analysis

### Static Graph Generation

The CI also pre-generates static PNG graphs for quick previews and backup:

- `scripts/generate_benchmark_graphs.py` uses `pandas` + `matplotlib` to output:
       - `top_series_time.png` — time-series plot for the top N benchmarks (by latest value)
       - `distribution_latest.png` — histogram of latest snapshot values across benchmarks
       - `heatmap.png` — heatmap overview (benchmarks vs time)

These files are copied to `gh-pages/graphs` and published along with the dashboard to provide static fallbacks and previews.

### Regeneration

To regenerate graphs locally run:

```bash
# Generate PNG graphs from JSONL history file
python3 scripts/generate_benchmark_graphs.py --input target/benchmark-data/history.jsonl --out-dir gh-pages/graphs
```

Make sure to pip install the dependencies first:
```bash
python3 -m pip install pandas matplotlib seaborn
```

---

## Quick Start

### 1. Run Benchmarks Locally

```powershell
# Run all benchmarks
cargo bench

# Or run specific crates
cargo bench -p astraweave-core --bench ecs_benchmarks
cargo bench -p astraweave-ai --bench ai_core_loop
cargo bench -p astraweave-physics --bench character_controller
```

### 2. Export JSONL History

```powershell
# Export current benchmark results to JSONL
.\scripts\export_benchmark_jsonl.ps1

# With verbose output
.\scripts\export_benchmark_jsonl.ps1 -Verbose

# Custom retention (default 30 days)
.\scripts\export_benchmark_jsonl.ps1 -MaxAgeDays 60
```

### 3. View Dashboard Locally

```powershell
# IMPORTANT: Start HTTP server from repository root (not from tools/benchmark-dashboard/)
# This ensures relative paths work correctly for data loading
cd /path/to/AstraWeave  # Navigate to repo root
python -m http.server 8000

# Open in browser
# http://localhost:8000/tools/benchmark-dashboard/
```

**Note**: The dashboard must be served from the repository root to access data files via relative paths. Serving from `tools/benchmark-dashboard/` will cause a 404 error when loading benchmark data.

### 4. Validate Thresholds

```powershell
# Check for regressions
.\scripts\check_benchmark_thresholds.ps1 -ShowDetails

# Create GitHub issue for regressions (dry run)
.\scripts\check_benchmark_thresholds.ps1 -CreateIssue -DryRun

# Strict mode (exit with error on regressions)
.\scripts\check_benchmark_thresholds.ps1 -Strict
```

---

## Architecture

### Data Flow

```
┌──────────────┐
│ cargo bench  │ Criterion generates benchmark results
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│ target/criterion/<bench>/base/       │
│   estimates.json                     │ Criterion output (mean, stddev)
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ export_benchmark_jsonl.ps1           │ Parses Criterion data
│   - Extracts mean/stddev             │
│   - Adds git metadata (sha, branch)  │
│   - Appends to JSONL                 │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ target/benchmark-data/history.jsonl │ Time-series database (JSONL)
│   {timestamp, benchmark, value, ...} │ One JSON object per line
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ tools/benchmark-dashboard/           │
│   - index.html (UI)                  │
│   - dashboard.js (D3.js charts)      │ Interactive web dashboard
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ GitHub Pages (gh-pages branch)       │ Deployed via GitHub Actions
│   https://lazyxeon.github.io/...     │
└──────────────────────────────────────┘
```

### JSONL Schema

Each line in `history.jsonl` is a JSON object with the following fields:

```json
{
  "timestamp": "2025-10-10T12:00:00Z",
  "benchmark_name": "astraweave-core::ecs_benchmarks/world_creation",
  "value": 25.8,
  "stddev": 1.2,
  "unit": "ns",
  "git_sha": "abc12345",
  "git_branch": "main",
  "git_dirty": false,
  "crate": "astraweave-core",
  "group": "ecs_benchmarks",
  "name": "world_creation"
}
```

---

## CI/CD Integration

### Nightly Workflow

The `.github/workflows/benchmark-dashboard.yml` workflow runs nightly to:

1. **Run Benchmarks**: Execute core benchmark suite (30 min timeout)
2. **Export JSONL**: Convert Criterion data to time-series format
3. **Check Thresholds**: Validate against `benchmark_thresholds.json`
4. **Deploy Dashboard**: Publish to GitHub Pages (gh-pages branch)
5. **Create Issues**: Auto-open GitHub issues for >10% regressions

### Manual Trigger

```bash
# Trigger workflow manually via GitHub CLI
gh workflow run benchmark-dashboard.yml

# Or via GitHub web UI: Actions > Benchmark Dashboard > Run workflow
```

---

## Dashboard Usage

### Filters

- **System**: Filter by crate category (ECS, AI, Physics, Terrain, Input)
- **Time Range**: Select 7, 14, 30, or 60-day window
- **Benchmark**: Choose specific benchmark or "All Benchmarks"

### Summary Cards

Top 8 benchmarks by current value, showing:
- Current mean time (ns, µs, ms)
- % change over selected time range
- Color-coded by system (ECS = blue, AI = cyan, etc.)

### Trend Chart

- **Hover**: Tooltip shows exact value, date, git SHA, branch
- **Multi-Series**: Compare multiple benchmarks on same chart
- **Legend**: Click to highlight specific series
- **Auto-Scaling**: Y-axis adjusts to data range

---

## Regression Handling

### Automated Detection

When a benchmark exceeds its threshold (defined in `.github/benchmark_thresholds.json`):

1. **Threshold Check**: `check_benchmark_thresholds.ps1` detects regression
2. **GitHub Issue**: Auto-created with title "⚠️ Benchmark Regression Detected"
3. **Investigation Template**: Issue includes checklist and dashboard link

### Manual Investigation

```powershell
# 1. View detailed regression report
.\scripts\check_benchmark_thresholds.ps1 -ShowDetails

# 2. Check dashboard for trends
# Open https://lazyxeon.github.io/AstraWeave/

# 3. Run local benchmarks to reproduce
cargo bench -p <crate> --bench <benchmark_name>

# 4. Profile hot paths (requires cargo-flamegraph)
cargo install flamegraph
cargo flamegraph --bench <benchmark_name>

# 5. Fix issue or update threshold
# If regression is intentional:
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline
```

---

## Maintenance

### Data Retention

JSONL history is automatically rotated to keep only the last 30 days (configurable):

```powershell
# Change retention to 60 days
.\scripts\export_benchmark_jsonl.ps1 -MaxAgeDays 60
```

### Threshold Updates

When baselines change (e.g., after optimization):

```powershell
# Run benchmarks
cargo bench

# Export current results
.\scripts\export_benchmark_jsonl.ps1

# Update thresholds to new baselines
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline
```

### Dashboard Customization

Edit `tools/benchmark-dashboard/dashboard.js` to:
- Change color scheme (`COLOR_SCHEME` object)
- Adjust chart dimensions (margins, width, height)
- Modify stat card selection (top 8 → custom logic)
- Add new filters or views

---

## Technical Details

### Dependencies

- **D3.js v7**: Loaded from CDN (no local install needed)
- **Criterion**: Rust benchmarking framework
- **PowerShell**: Export script (cross-platform with pwsh)
- **GitHub Pages**: Free static site hosting

### Performance

- **JSONL Export**: <500ms runtime overhead
- **Dashboard Load**: <2s for 30-day dataset (~1000 entries)
- **Data Size**: ~10KB per day of benchmarks (~300KB per month)
- **Chart Render**: 60fps smooth interactions

### Browser Compatibility

- Chrome/Edge 90+ ✅
- Firefox 88+ ✅
- Safari 14+ ✅
- Mobile browsers ✅

---

## Troubleshooting

### Issue: "Failed to load ../../target/benchmark-data/history.jsonl"

**Root Cause**: Dashboard is being served from the wrong directory or data file doesn't exist.

**Solutions**:

1. **Ensure HTTP server runs from repository root**:
   ```bash
   # CORRECT - serve from repo root
   cd /path/to/AstraWeave
   python -m http.server 8000
   # Then open: http://localhost:8000/tools/benchmark-dashboard/
   
   # INCORRECT - serving from dashboard directory will fail
   cd tools/benchmark-dashboard  # ❌ Don't do this
   python -m http.server 8000     # ❌ Data won't be accessible
   ```

2. **Generate benchmark data**:
   ```powershell
   cargo bench
   .\scripts\export_benchmark_jsonl.ps1
   ```

3. **Verify data file exists**:
   ```bash
   # Check if data file was created
   ls -la target/benchmark-data/history.jsonl
   ```

### Issue: "No data available"

**Solution**: Run benchmarks and export JSONL first:
```powershell
cargo bench
.\scripts\export_benchmark_jsonl.ps1
```

### Issue: "D3.js library failed to load"

**Root Cause**: CDN blocked by ad blocker, firewall, or network issues.

**Solutions**:
1. Disable ad blocker for localhost
2. Check browser console for specific error
3. Download d3.v7.min.js locally (optional):
   ```bash
   cd tools/benchmark-dashboard
   curl https://d3js.org/d3.v7.min.js -o d3.v7.min.js
   # Update index.html to use local file instead of CDN
   ```

**Note**: Stats cards will still display correctly even if D3.js fails to load - only the chart will be unavailable.

### Issue: Dashboard shows data from wrong source

**Explanation**: Dashboard tries multiple data sources in order:
1. `target/benchmark-data/history.jsonl` (local development)
2. `docs/benchmark_data/benchmark_history.jsonl` (production/GitHub Pages)

Check browser console to see which source was used.

### Issue: GitHub Pages 404

**Solution**: Enable GitHub Pages in repository settings:
1. Go to Settings > Pages
2. Set Source to "gh-pages" branch
3. Wait 2-5 minutes for deployment

### Issue: Threshold validation fails

**Solution**: Update thresholds after major changes:
```powershell
.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline
git add .github/benchmark_thresholds.json
git commit -m "chore: update benchmark thresholds"
```

---

## Examples

### Weekly Performance Review

```powershell
# 1. Export latest benchmarks
.\scripts\export_benchmark_jsonl.ps1 -Verbose

# 2. Check for any regressions
.\scripts\check_benchmark_thresholds.ps1 -ShowDetails

# 3. Open dashboard in browser (serve from repo root)
python -m http.server 8000
# Navigate to: http://localhost:8000/tools/benchmark-dashboard/

# 4. Review 7-day trends (select "Last 7 Days" in UI)
```

### Pre-Merge PR Validation

```powershell
# 1. Run full benchmark suite
cargo bench

# 2. Validate against thresholds (strict mode)
.\scripts\check_benchmark_thresholds.ps1 -Strict

# Exit code:
#   0 = All benchmarks within thresholds ✅
#   1 = Regressions detected ❌
```

### Investigate Specific Regression

```powershell
# 1. View detailed report
.\scripts\check_benchmark_thresholds.ps1 -ShowDetails

# 2. Check dashboard for context
# Filter: System = "AI", Time Range = "30 days"
# Look for sudden spikes in GOAP planning benchmarks

# 3. Profile hot path
cargo flamegraph --bench goap_planning

# 4. Fix and re-validate
cargo bench -p astraweave-behavior --bench goap_planning
.\scripts\check_benchmark_thresholds.ps1 -ShowDetails
```

---

## Resources

- **Live Dashboard**: https://lazyxeon.github.io/AstraWeave/
- **Threshold Config**: `.github/benchmark_thresholds.json`
- **CI Workflow**: `.github/workflows/benchmark-dashboard.yml`
- **Export Script**: `scripts/export_benchmark_jsonl.ps1`
- **Validation Script**: `scripts/check_benchmark_thresholds.ps1`

---

## Contributing

To add a new benchmark to the dashboard:

1. **Create Benchmark**: Add to appropriate crate (e.g., `benches/my_benchmark.rs`)
2. **Run Benchmark**: `cargo bench -p <crate> --bench my_benchmark`
3. **Export JSONL**: `.\scripts\export_benchmark_jsonl.ps1`
4. **Add Threshold**: `.\scripts\check_benchmark_thresholds.ps1 -UpdateBaseline`
5. **Commit Changes**: Threshold JSON will be updated automatically

---

**Version**: 1.0.0  
**Last Updated**: 2025-10-10  
**Maintainer**: AstraWeave Team
