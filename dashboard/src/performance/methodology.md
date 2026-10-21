# Benchmark Methodology

This document describes how AstraWeave performance measurements are collected, validated, and reported.

## Measurement Philosophy

> **"Prove it, don't hype it."**

Every performance claim in AstraWeave documentation:
- Has a command that reproduces it
- Captures raw logs for auditing
- Uses statistical analysis for reliability
- Is validated against real workloads, not synthetic benchmarks

---

## Tools & Infrastructure

### Criterion.rs

All microbenchmarks use [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical rigor.

**Why Criterion**:
- Statistical analysis with confidence intervals
- Outlier detection and filtering
- Baseline comparison (catch regressions)
- HTML report generation

**Location**: `target/criterion/**/base/estimates.json`

### Odyssey Runner

For full-suite benchmarking, use the automation script:

```powershell
./scripts/benchmark_odyssey.ps1 -OutDir benchmark_results/$(Get-Date -Format 'yyyy-MM-dd')
```

**Outputs**:
- `environment.txt` - OS/CPU/RAM, rustc/cargo version, git SHA
- `packages_with_benches.txt` - Inventory of benchmarked crates
- `run_order.txt` - Execution order
- `bench_<package>.log` - Raw benchmark output per crate
- `run_results.json` - Success/fail status

---

## Statistical Practices

### Confidence Intervals

Criterion provides 95% confidence intervals for all measurements. We report:

- **Point estimate**: The measured mean
- **Lower bound**: 95% CI lower
- **Upper bound**: 95% CI upper

Example: `1.34 ns [1.33 ns, 1.35 ns]` means the true mean is 95% likely within that range.

### Warm-Up & Iterations

Default Criterion settings:
- **Warm-up**: 3 seconds (eliminates cold-start artifacts)
- **Measurement**: 5 seconds minimum
- **Sample size**: 100 samples minimum

### Outlier Handling

Criterion automatically detects and reports outliers:
- **Mild outliers**: 1.5× IQR
- **Severe outliers**: 3× IQR

Outliers are flagged in reports but included in analysis (not discarded).

---

## Benchmark Categories

### 1. Microbenchmarks

Single-operation measurements (e.g., "how long does `vec3_lerp` take?").

**Location**: `crates/*/benches/*.rs`

**Example**:
```rust
fn bench_vec3_lerp(c: &mut Criterion) {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(1.0, 1.0, 1.0);
    c.bench_function("vec3_lerp", |bencher| {
        bencher.iter(|| a.lerp(b, 0.5))
    });
}
```

### 2. Adversarial Benchmarks

Stress tests for edge cases and worst-case scenarios.

**Categories** (22 sections):
- Gameplay adversarial (massive damage, rapid hits)
- Input adversarial (input storms, frame clear)
- Math adversarial (IEEE-754 edge cases: infinity, NaN, denormals)
- Navigation adversarial (sliver triangles, impossible paths)
- Security adversarial (script sandboxing, anti-cheat)
- And 17 more...

**Purpose**: Ensure production stability under extreme conditions.

### 3. Integration Benchmarks

End-to-end measurements of complete systems.

**Example**: "Full game loop with 5,000 entities"

```rust
fn bench_full_game_loop(c: &mut Criterion) {
    let mut world = setup_world_with_entities(5000);
    c.bench_function("full_game_loop/5000_entities", |bencher| {
        bencher.iter(|| world.tick(1.0 / 60.0))
    });
}
```

### 4. Scalability Benchmarks

Measure performance across varying input sizes.

**Example**: Entity spawn at 10, 100, 1000, 10000 entities.

```rust
fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn");
    for size in [10, 100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| b.iter(|| spawn_entities(size))
        );
    }
    group.finish();
}
```

---

## Environment Standardization

### Hardware Requirements

Benchmark machines should document:
- **CPU**: Model, cores, clock speed
- **RAM**: Size, speed
- **OS**: Windows/Linux/macOS version
- **Rust**: `rustc --version`
- **Profile**: Always `--release`

### Isolation Practices

For reliable measurements:
1. Close unnecessary applications
2. Disable turbo boost (optional, for consistency)
3. Run multiple times to verify reproducibility
4. Use `cargo bench -- --noplot` to skip HTML generation (faster)

---

## Reporting Standards

### Master Benchmark Report

All benchmark results are consolidated in:
- `docs/masters/MASTER_BENCHMARK_REPORT.md`

**Update triggers**:
- Any benchmark changes >10%
- New benchmarks added
- Performance regressions discovered

### Version Tracking

Each report version documents:
- Version number (e.g., v5.55)
- Date of measurement
- Key changes since last version
- Critical fixes applied

---

## Regression Detection

### Baseline Comparison

```bash
# Save current as baseline
cargo bench -p astraweave-ecs -- --save-baseline main

# Compare against baseline
cargo bench -p astraweave-ecs -- --baseline main
```

### CI Integration

GitHub Actions workflow (`benchmark.yml`) runs benchmarks on:
- Pull requests (compare against main)
- Nightly builds (detect gradual regressions)

### Alert Thresholds

| Change | Action |
|--------|--------|
| < 5% | Normal variance, no action |
| 5-10% | Flag for review |
| 10-20% | Investigate root cause |
| > 20% | Block merge, fix required |

---

## Coverage Methodology {#coverage}

Test coverage is measured using `cargo-llvm-cov`:

```bash
# Generate coverage report
cargo llvm-cov --workspace --html

# View report
open target/llvm-cov/html/index.html
```

### Coverage by Tier

| Tier | Crates | Target | Actual |
|------|--------|--------|--------|
| **Tier 1** (Critical) | ecs, core, ai, render | 80% | 75.3% |
| **Tier 2** (Important) | physics, nav, gameplay | 75% | 72.6% |
| **Tier 3** (Supporting) | audio, scene, terrain | 70% | 71.8% |
| **Tier 4** (Specialized) | fluids, llm, prompts | 65% | 71.5% |

### Per-Crate Coverage (verified January 2026)

| Crate | Coverage | Status |
|-------|----------|--------|
| astraweave-ecs | 83.2% | ✅ |
| astraweave-core | 79.1% | ✅ |
| astraweave-ai | 71.3% | ✅ |
| astraweave-render | 67.4% | ✅ |
| astraweave-physics | 76.8% | ✅ |
| astraweave-fluids | 94.2% | ✅ A+ |
| astraweave-nav | 72.1% | ✅ |
| astraweave-gameplay | 68.9% | ✅ |
| astraweave-terrain | 71.5% | ✅ |
| astraweave-audio | 69.2% | ✅ |
| astraweave-scene | 74.6% | ✅ |
| astraweave-llm | 58.3% | ⚠️ Beta |

---

## Reproducing Results

### Quick Verification

```bash
# Verify ECS benchmarks match documentation
cargo bench -p astraweave-ecs -- entity_spawn/empty/10000

# Expected: ~645µs (±10%)
```

### Full Reproduction

1. Clone repository at documented commit
2. Run `./scripts/benchmark_odyssey.ps1`
3. Compare `benchmark_results/*/` against documented values
4. Variance > 20% indicates environment difference

---

## See Also

- [Benchmarks](./benchmarks.md) - Performance data
- [Optimization Guide](./optimization.md) - Improvement techniques
- [Performance Budgets](./budgets.md) - Frame budget allocation
- [Master Report](../../masters/MASTER_BENCHMARK_REPORT.md) - Complete raw data
