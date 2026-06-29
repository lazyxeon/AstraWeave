# AstraWeave Benchmarking Philosophy

**Version**: 1.0.0  
**Date**: January 2026  
**Status**: CANONICAL REFERENCE  
**Purpose**: Establish benchmarking standards and transparency

---

## Mission Statement

> AstraWeave commits to rigorous, transparent game-engine benchmarking. Every performance claim is backed by reproducible measurements, every edge case is tested adversarially, and every optimization is validated scientifically.

---

## Core Principles

### 1. Transparency Over Marketing

Traditional game engine benchmarks often cherry-pick favorable scenarios. AstraWeave rejects this approach.

**Our commitment:**
- Every benchmark is **reproducible** from source
- Methodology is **documented** alongside results
- Edge cases and **worst-case scenarios** are tested prominently
- Regressions are **publicly tracked** in revision history

### 2. Adversarial by Default

We don't just test happy paths. Our 22 adversarial benchmark suites specifically target:

| Category | What We Test |
|----------|--------------|
| **Pathological Inputs** | Degenerate triangles, NaN/Infinity, empty collections |
| **Stress Conditions** | Memory pressure, high contention, rapid state changes |
| **Edge Cases** | Zero values, maximum limits, boundary conditions |
| **Security Vectors** | Malformed data, DoS patterns, injection attempts |
| **Chaos Engineering** | Random failures, timeouts, corrupted state |

### 3. Scientific Methodology

Every benchmark follows rigorous measurement practices:

```rust
// CORRECT: Prevents compiler from optimizing away the result
b.iter(|| {
    std::hint::black_box(expensive_operation(input))
})

// INCORRECT: Compiler may eliminate the computation
b.iter(|| {
    expensive_operation(input); // Result discarded!
})
```

### 4. Mock vs Real Distinction

We maintain two parallel benchmark tracks:

| Track | Purpose | When to Use |
|-------|---------|-------------|
| **Mock/Adversarial** | Algorithm complexity isolation | Measuring pure computation |
| **Real/Integration** | Production code paths | Measuring end-to-end behavior |

Both are valuable. Mocks eliminate external variance; real tests capture integration overhead.

---

## Benchmark Categories

### Category 1: Microbenchmarks

**Purpose**: Measure isolated operations in nanoseconds/microseconds

**Examples:**
- Entity spawn: 645µs @ 10K entities
- Component query: 808ps single lookup
- Vector normalize: 18-25ns

**Methodology:**
- Use `criterion` with statistical analysis
- Minimum 100 iterations for stability
- Report mean, std deviation, and throughput

### Category 2: System Benchmarks

**Purpose**: Measure subsystem performance under realistic load

**Examples:**
- Full AI pipeline: 471µs @ 500 agents
- Physics step: 47µs @ 100 rigid bodies
- Navigation pathfind: 7.5µs short path

**Methodology:**
- Representative workloads from real games
- Include setup/teardown in total time
- Report 60 FPS budget percentage

### Category 3: Adversarial Benchmarks

**Purpose**: Stress-test worst-case scenarios

**Examples:**
- Sliver triangles: 99ps/triangle (degenerate geometry)
- NaN propagation: 23-34ns (IEEE-754 edge cases)
- Memory pressure: Performance under allocation stress

**Methodology:**
- Intentionally pathological inputs
- Document expected degradation
- Establish failure thresholds

### Category 4: Integration Benchmarks

**Purpose**: Measure cross-system interactions

**Examples:**
- ECS → AI → Physics → Render loop
- Save/Load → Deserialize → Validate cycle
- Network → ECS delta → Compression pipeline

**Methodology:**
- Full system stack active
- Realistic game scenarios
- Profile hotspots for optimization targets

---

## Mock Implementation Guidelines

### When to Use Mocks

✅ **Appropriate for mocks:**
- Testing algorithm complexity (O(n) vs O(n²))
- Isolating pathological code paths
- Avoiding external dependencies (GPU, network)
- Creating reproducible adversarial inputs

❌ **Not appropriate for mocks:**
- Validating production performance claims
- Measuring integration overhead
- Testing real-world data patterns
- Final release qualification

### Mock Implementation Standards

```rust
// Example: Mirror local types for adversarial testing
// ============================================================================
// LOCAL TYPES (Mirror astraweave-npc API)
// ============================================================================
// These types replicate the production API structure to enable:
// 1. Isolated algorithm testing without full crate dependencies
// 2. Adversarial input generation that would be rejected by production validation
// 3. Reproducible benchmarks independent of external state

#[derive(Clone, Debug)]
struct MockNpcProfile {
    id: NpcId,
    name: String,
    // ... mirrors production struct
}
```

**Key requirements:**
1. Comment block explaining purpose: `// LOCAL TYPES (Mirror X API)`
2. Document why mocking is appropriate
3. Maintain API compatibility with production types
4. Include real-implementation counterpart where possible

---

## Measurement Standards

### Statistical Requirements

| Metric | Minimum | Preferred |
|--------|---------|-----------|
| Sample size | 100 | 1000+ |
| Warmup iterations | 10 | 50+ |
| Outlier handling | 3σ | Tukey fences |
| Confidence interval | 95% | 99% |

### Reporting Format

```markdown
| Benchmark | Mean | Std Dev | Throughput | Budget % |
|-----------|------|---------|------------|----------|
| entity_spawn/10K | 645µs | ±12µs | 15.5M/s | 3.87% |
```

Required fields:
- **Mean**: Average time/throughput
- **Std Dev**: Measurement variance
- **Throughput**: Elements/second where applicable
- **Budget %**: Percentage of 16.67ms (60 FPS) frame

### Black Box Requirements

All benchmark iterations **MUST** use `std::hint::black_box()`:

```rust
// The result MUST flow through black_box
b.iter(|| std::hint::black_box(compute(input)))

// Inputs MAY also use black_box for extra safety
b.iter(|| std::hint::black_box(compute(std::hint::black_box(&input))))
```

This prevents LLVM from:
- Eliminating "unused" computations
- Hoisting loop-invariant code
- Constant-folding known inputs

---

## Regression Detection

### Threshold Definitions

| Severity | Threshold | Action |
|----------|-----------|--------|
| **Critical** | >50% slower | Block release, immediate fix |
| **Major** | >20% slower | Investigate within 24h |
| **Minor** | >10% slower | Track, fix in next sprint |
| **Noise** | <10% change | Log, no action |

### Historical Tracking

All benchmarks maintain revision history in `MASTER_BENCHMARK_REPORT.md`:

```markdown
### Revision History

| Version | Date | Changes |
|---------|------|---------|
| 5.53 | Jan 2026 | ECS regression FIXED - BlobVec lazy init |
| 5.52 | Jan 2026 | ECS regression DETECTED - 47-333% slower |
| 5.51 | Dec 2025 | Fluids adversarial complete |
```

---

## Transparency Commitments

### What We Publish

1. **Raw benchmark data** in `target/criterion/` (1,650+ result directories)
2. **Methodology documentation** in `docs/masters/MASTER_BENCHMARK_REPORT.md`
3. **Known limitations** documented inline
4. **Regression history** with root cause analysis
5. **Comparison matrices** vs industry competitors

### What We Won't Do

❌ Cherry-pick favorable scenarios for marketing  
❌ Hide regressions or unfavorable comparisons  
❌ Benchmark unrealistic "demo" configurations  
❌ Omit setup/teardown costs from measurements  
❌ Compare against strawman implementations  

---

## File Organization

### Directory Structure

```
crates/
├── astraweave-{crate}/
│   └── benches/
│       ├── {system}_benchmarks.rs     # Core system benchmarks
│       ├── {system}_adversarial.rs    # Adversarial/stress tests
│       └── {system}_integration.rs    # Cross-system integration
docs/
├── masters/
│   └── MASTER_BENCHMARK_REPORT.md     # Canonical results
└── current/
    ├── BENCHMARK_PRODUCTION_AUDIT_REPORT.md
    └── BENCHMARKING_PHILOSOPHY.md     # This document
```

### Naming Conventions

| Pattern | Purpose |
|---------|---------|
| `*_benchmarks.rs` | Standard system benchmarks |
| `*_adversarial.rs` | Pathological/stress benchmarks |
| `*_integration.rs` | Cross-system benchmarks |
| `*_stress.rs` | Long-running stress tests |
| `*_scaling.rs` | N-scaling analysis |

---

## Quality Checklist

Before merging any benchmark file:

- [ ] Uses `std::hint::black_box()` correctly
- [ ] Has file-level documentation (`//!` header)
- [ ] Includes throughput where applicable
- [ ] Tests edge cases (zero, max, boundary)
- [ ] Feature-gated code has fallback behavior
- [ ] No `unwrap()` without documented rationale
- [ ] Follows naming conventions
- [ ] Updates MASTER_BENCHMARK_REPORT.md if needed

---

## Industry Comparison

### How AstraWeave Compares

| Aspect | AstraWeave | Bevy | Godot | Unity |
|--------|-----------|------|-------|-------|
| Benchmark files | 99 | ~20 | N/A | N/A |
| Adversarial suites | 22 | 0 | 0 | 0 |
| Public methodology | ✅ Full | Partial | No | No |
| Regression tracking | ✅ Git history | Manual | No | No |
| Worst-case testing | ✅ Explicit | Implicit | No | No |

### Our Differentiators

1. **Adversarial testing**
2. **Full methodology transparency** - every claim is reproducible
3. **Statistical rigor** - proper sample sizes, confidence intervals
4. **Edge case focus** - pathological inputs tested by default
5. **Historical tracking** - regressions documented with root causes

---

## Contributing Benchmarks

### Requirements for New Benchmarks

1. **Follow this philosophy** - transparency, rigor, adversarial testing
2. **Include documentation** - purpose, methodology, expectations
3. **Add adversarial variant** - for any new system benchmark
4. **Update master report** - add results to MASTER_BENCHMARK_REPORT.md
5. **Review for panics** - justify or eliminate `unwrap()` calls

### Template

```rust
//! Benchmark Suite: [System Name]
//!
//! ## Purpose
//! [What these benchmarks measure and why]
//!
//! ## Methodology  
//! [How measurements are taken, what's included/excluded]
//!
//! ## Expected Performance
//! [Baseline expectations, 60 FPS budgets, scaling characteristics]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;

fn bench_system_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("system_operation");
    
    // Configure measurement parameters
    group.throughput(Throughput::Elements(1000));
    
    // Setup test data
    let input = setup_realistic_input();
    
    group.bench_function("realistic_case", |b| {
        b.iter(|| black_box(process(black_box(&input))))
    });
    
    // Always include edge cases
    group.bench_function("edge_case_empty", |b| {
        b.iter(|| black_box(process(black_box(&empty_input))))
    });
    
    group.finish();
}

criterion_group!(benches, bench_system_operation);
criterion_main!(benches);
```

---

## Conclusion

AstraWeave's benchmarking philosophy prioritizes:

1. **Transparency** - All claims are reproducible
2. **Rigor** - Statistical methodology, proper measurement
3. **Adversarial** - Worst cases tested explicitly
4. **Honesty** - Regressions documented, limitations acknowledged

This approach reflects how we believe game engines should validate and communicate performance. We believe the gaming community deserves better than marketing benchmarks - they deserve truth.

---

**Document Maintainer**: Core Engineering Team  
**Review Cycle**: Quarterly  
**Last Reviewed**: January 2026

*"Measure everything. Hide nothing. Improve constantly."*
