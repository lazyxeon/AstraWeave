# Benchmark Dashboard Quick Start

## 🚀 One-Command Launch

### Option 1: PowerShell (Recommended)

```powershell
# Run benchmarks, export data, generate graphs, and open dashboard
.\scripts\run_benchmark_dashboard.ps1

# Or skip running benchmarks (use existing data)
.\scripts\run_benchmark_dashboard.ps1 -SkipBench

# Use custom port
.\scripts\run_benchmark_dashboard.ps1 -Port 8080
```

### Option 2: Double-Click Launch (Windows)

Simply double-click **`Launch-Benchmark-Dashboard.bat`** in the repository root.

This will:
- ✅ Use existing benchmark data (no re-running benchmarks)
- ✅ Export to JSONL format
- ✅ Generate visualization graphs
- ✅ Start HTTP server on port 8000
- ✅ Open your browser to the dashboard

### Option 3: Manual Steps (Advanced)

```powershell
# 1. Run benchmarks
cargo bench --package astraweave-ecs --package astraweave-ai --package astraweave-physics --package astraweave-math --package astraweave-terrain --package astraweave-input

# 2. Export to JSONL
.\scripts\export_benchmark_jsonl.ps1

# 3. Generate graphs
python scripts/generate_benchmark_graphs.py --input target/benchmark-data/history.jsonl --out-dir gh-pages/graphs

# 4. Start server (from tools/benchmark-dashboard directory)
cd tools/benchmark-dashboard
python -m http.server 8000

# 5. Open browser to http://localhost:8000
```

## 📊 What You'll See

The dashboard displays:

- **Summary Cards** - Total benchmarks, snapshot count, date range
- **Time Series Chart** - Performance trends over time (interactive with D3.js)
- **Distribution Histogram** - Latest benchmark value distribution
- **Sparklines Grid** - Small multiples showing individual benchmark trends
- **Benchmark Table** - Sortable list with values and percent changes
- **Static Graphs** - Pre-generated PNGs (top series, distribution, heatmap)

## 🔧 Troubleshooting

**"Python not found"**
```powershell
# Install Python 3.8+ from python.org or via winget
winget install Python.Python.3.13
```

**"No benchmark data found"**
```powershell
# Run benchmarks first
cargo bench --package astraweave-math

# Then re-run the dashboard script
.\scripts\run_benchmark_dashboard.ps1 -SkipBench
```

**"Port 8000 already in use"**
```powershell
# Use a different port
.\scripts\run_benchmark_dashboard.ps1 -Port 8080
```

**CORS errors in browser**
- Always use HTTP server (not direct file:// URLs)
- The script handles this automatically

## 📁 File Locations

- **Benchmark Data**: `target/benchmark-data/history.jsonl`
- **Metadata**: `target/benchmark-data/metadata.json`
- **Graphs**: `gh-pages/graphs/*.png`
- **Dashboard**: `tools/benchmark-dashboard/index.html`

## 🌐 CI/CD Integration

The benchmark dashboard is automatically updated on GitHub Pages via `.github/workflows/benchmark-dashboard.yml`:

- **Live Dashboard**: https://lazyxeon.github.io/AstraWeave/
- **Update Frequency**: Nightly (or on benchmark-related commits)
- **Data Retention**: 30 days of historical data

## 📚 Adding New Benchmarks

1. Add benchmark to a crate's `benches/` directory
2. Use Criterion framework:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};
   
   fn my_benchmark(c: &mut Criterion) {
       c.bench_function("my_operation", |b| {
           b.iter(|| {
               // Code to benchmark
           });
       });
   }
   
   criterion_group!(benches, my_benchmark);
   criterion_main!(benches);
   ```
3. Run `.\scripts\run_benchmark_dashboard.ps1` to include in dashboard

## ⚙️ Configuration

Edit `scripts/export_benchmark_jsonl.ps1` to configure:

- **Data retention**: `-MaxAgeDays 30` (default: 30 days)
- **Output path**: `-OutputFile "custom/path.jsonl"`
- **Benchmark directory**: `-BenchmarkDir "target/criterion"`

## 🎯 Performance Targets

Current benchmarks validate against these thresholds (see `scripts/benchmark_thresholds.json`):

- **ECS World Creation**: <50 ns
- **Entity Spawn**: <500 ns
- **GOAP Planning**: <50 µs
- **Physics Tick**: <10 µs
- **AI Core Loop**: <5 µs

Regression detection runs automatically in CI.
