<!-- markdownlint-disable MD013 MD033 MD041 -->
<div class="astra-landing">

<section class="astra-hero" style="grid-template-columns:1fr">
<div class="astra-hero__copy">
  <span class="astra-eyebrow">Performance</span>
  <h1>Performance Benchmarks</h1>
  <p class="astra-lead">
    All benchmarks executed via Criterion.rs statistical benchmarking.
    Results sourced from the Master Benchmark Report v5.55 (January 2026).
    Every measurement can be reproduced with documented commands.
  </p>
  <div class="astra-meta" aria-label="Benchmark metadata">
    <span class="astra-meta-badge">~1,500 benchmarks across 76 sections</span>
    <span class="astra-meta-badge">Criterion.rs + Real Ollama validation</span>
    <span class="astra-meta-badge">Production Ready (Grade A+)</span>
  </div>
</div>
</section>

<section class="astra-proof-strip" aria-label="Key benchmark metrics">
  <article class="astra-proof-tile">
    <strong>85%</strong>
    <span>p99 headroom at 1,000 entities @ 60 FPS</span>
  </article>
  <article class="astra-proof-tile">
    <strong>15.0µs</strong>
    <span>ECS entity spawn (100), 70% under target</span>
  </article>
  <article class="astra-proof-tile">
    <strong>286ns</strong>
    <span>GOAP planning (full), 97% under target</span>
  </article>
  <article class="astra-proof-tile">
    <strong>9.8ns</strong>
    <span>GOAP cache hit, 99% under target</span>
  </article>
  <article class="astra-proof-tile">
    <strong>1.27ms</strong>
    <span>p50 frame time at 1,000 entities</span>
  </article>
  <article class="astra-proof-tile">
    <strong>2.42ms</strong>
    <span>p99 frame time at 1,000 entities</span>
  </article>
</section>

<section class="astra-section">
<div class="astra-section-heading">
<span class="astra-kicker">Executive summary</span>
<h2>Key validation results — January 2026</h2>
<p>
  Engine has <strong>85% headroom</strong> at p99 for 1,000 entities at 60 FPS.
  All core systems operate well under their per-frame budgets.
</p>
</div>

| System | Benchmark | Target | Actual | Margin |
|--------|-----------|--------|--------|--------|
| **ECS** | Entity spawn (100) | <50µs | 15.0µs | 70% under |
| **ECS** | Entity spawn (1000) | <500µs | 106.7µs | 79% under |
| **AI** | GOAP planning (full) | <10µs | 286ns | 97% under |
| **AI** | GOAP planning (cache) | <1µs | 9.8ns | 99% under |
| **Frame** | p50 @ 1k entities | <16.67ms | 1.27ms | 92% under |
| **Frame** | p99 @ 1k entities | <16.67ms | 2.42ms | 85% under |

</section>

<section class="astra-section">
<div class="astra-section-heading astra-section-heading--wide">
<span class="astra-kicker">Best performers</span>
<h2>Operations achieving sub-nanosecond and sub-10ns latency.</h2>
<p>
  These benchmarks represent AstraWeave's highest-performing operations,
  many achieving sub-nanosecond latency.
</p>
</div>

<div class="astra-split">
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Sub-nanosecond</span>
<h3>Operations under 1 ns</h3>

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Multi-Agent Per-Agent | 12-20 ps | 50-83 trillion/sec |
| Nav Sliver Triangles | 99-104 ps | 10 billion/sec |
| Multi-Agent Per-Plan | 0.29-0.31 ns | 3.2-3.4 billion/sec |
| Pan Mode Switching | 418 ps | — |
| State Transitions | 0.49-0.51 ns | — |
| Emotion Blending | 0.55 ns | — |
| Multi-Agent Feedback | 0.73-0.76 ns | 1.3 billion/sec |
| MSAA Resize 720p | 582-645 ps | — |
| UI Settings Nav | 696 ps | — |
| Clear Frame | 0.72 ns | — |
| Weather Attenuation | 730-783 ps | 22.8 billion/frame |
| Room Overlap Check | 571-629 ps | — |
| Frustum AABB Inside | 889-915 ps | — |
| GPU Budget Check | 890 ps-1.05 ns | 17 billion/frame |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Sub-10 nanosecond</span>
<h3>Operations under 10 ns</h3>

| Operation | Latency | Notes |
|-----------|---------|-------|
| SparseSet Lookup (1k) | 1.56 ns | 37× faster than BTreeMap |
| SIMD Movement | 1.73 ns | 2.26× faster than naive |
| Quat Multiply | 1.34 ns | glam SIMD-optimized |
| Quat Slerp | 2.10 ns | Rotation interpolation |
| Context Switching | 2.38 ns | 7M switches/frame |
| GOAP (no enemies) | 3.46-3.56 ns | Idle detection FREE |
| Component Deserialize | 3.50 ns | Postcard ECS |
| Physics Stage | 3.63 ns | 7,580× vs perception |
| RAG Engine Creation | 4.61 ns | Zero-cost abstraction |
| Mat4 Multiply | 4.28 ns | glam SIMD matrix |
| GOAP (close) | 4.68-5.11 ns | Tactical decision |
| GOAP (far) | 7.04-7.86 ns | Strategic decision |
| SparseSet Insert | 9.9 ns | 13× faster than BTreeMap |

</article>
</div>
</section>

<section class="astra-section">
<div class="astra-section-heading astra-section-heading--wide">
<span class="astra-kicker">Core systems</span>
<h2>Engine subsystem benchmarks</h2>
<p>
  Each subsystem is benchmarked independently with Criterion.rs.
  Results include budget analysis against the 16.67ms frame budget.
</p>
</div>

<div class="astra-grid astra-grid--three">
<article class="astra-card astra-card--panel">
<span class="astra-kicker">ECS</span>
<h3>Entity-Component-System</h3>

| Benchmark | Result | Budget |
|-----------|--------|--------|
| Spawn empty (10k) | 645µs | Excellent |
| Spawn + Position (10k) | 5.6ms | Production |
| Despawn empty (10k) | 287µs | Fixed |
| Despawn + comp (10k) | 2.5ms | 68% faster |
| Iteration (10k) | 273µs | Excellent |
| Archetype trans (10k) | 5.6ms | Within budget |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">AI</span>
<h3>Planning and orchestration</h3>

| Benchmark | Result | Notes |
|-----------|--------|-------|
| GOAP (cache hit) | 9.8 ns | 99% under |
| GOAP (cache miss) | 286 ns | 97% under |
| GOAP next (idle) | 3.5 ns | 4.7B ops/frame |
| GOAP next (close) | 5.1 ns | 3.5B ops/frame |
| GOAP next (far) | 7.9 ns | 2.4B ops/frame |
| Multi-agent (10) | 1.34-1.39µs | 66-68% faster |
| Arbiter GOAP | 101.7 ns | 982× faster |
| Arbiter LLM | 575 ns | 86× faster |
| Mode transition | 221.9 ns | 45× faster |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Physics</span>
<h3>Simulation and collision</h3>

| Benchmark | Result | Notes |
|-----------|--------|-------|
| Character move | 43.8-52.0 ns | 12-26% faster |
| Rigid body lookup | 14.8-15.4 ns | 10× vs character |
| Raycast (empty) | 26.3-31.5 ns | 8-23% faster |
| Rigid body batch | 47µs | Excellent |
| Spatial hash | 99.96% fewer | Grid optimization |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Fluids</span>
<h3>SPH simulation (A+ grade)</h3>

| Benchmark | Result | Notes |
|-----------|--------|-------|
| Particles (1K-10K) | 5.3-110µs | 100-322 Melem/s |
| Spatial hashing | 163µs-5.6ms | 38-62% improved |
| SPH kernels (100K) | 171-223µs | poly6/spiky |
| Density/pressure | 3.5-10.5ms | — |
| Sim step (1K) | 1.8-3.0ms | — |
| Multi-step | 450-500µs | 45-57% faster |
| GPU data prep | 0.9-2.6 ns | Sub-nanosecond |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Rendering</span>
<h3>wgpu pipeline benchmarks</h3>

| Category | Benchmark | Result |
|----------|-----------|--------|
| Culling | AABB inside | 889-915 ps |
| Culling | Contains point | 951 ps-1.01 ns |
| MSAA | Mode check | 795-842 ps |
| MSAA | Resize 720p | 582-645 ps |
| Camera | View matrix | 4.42-5.36 ns |
| Camera | Toggle mode | 1.72-2.29 ns |
| Instancing | Savings calc | 1.43-1.52 ns |
| Weather | Particle | 1.95-2.04 ns |
| Weather | Attenuation | 730-783 ps |

</article>
<article class="astra-card astra-card--panel">
<span class="astra-kicker">Animation and navigation</span>
<h3>Interpolation and pathfinding</h3>

| Benchmark | Result | Notes |
|-----------|--------|-------|
| vec3_lerp | 1.69-1.83 ns | 57% faster |
| quat_to_rotation | 1.63-1.73 ns | 36% faster |
| Tween update | 22.1 ns | — |
| Spring update | 14.2 ns | 1.6× vs tween |
| Sliver triangles | 99-104 ps | Sub-nanosecond |
| Impossible paths | 3.7-24.9µs | Fast-fail |
| Maze stress | 1.6-108µs | — |
| Pathfind short | 7.5µs | Excellent |

</article>
</div>
</section>

<div class="astra-band">
<div class="astra-section-heading astra-section-heading--compact">
<span class="astra-eyebrow">Frame budget analysis</span>
<h2>Target: 60 FPS = 16.67ms per frame</h2>
<p>
  Budget breakdown at 1,000 entities shows the engine using only 14.5% of the
  available frame time at p99, leaving 85% headroom for gameplay logic and rendering.
</p>
</div>
<div class="astra-split">
<div class="astra-node" style="flex:1">
<strong>Budget breakdown (1,000 entities)</strong>

| System | Time | Budget % | Status |
|--------|------|----------|--------|
| ECS Core | 85 µs | 0.51% | ✅ |
| AI (500 agents) | 471 µs | 2.83% | ✅ |
| Physics (100 bodies) | 47 µs | 0.28% | ✅ |
| Core loop (5k) | 529 µs | 3.17% | ✅ |
| **p50 Total** | **1.27 ms** | **7.6%** | ✅ |
| **p99 Total** | **2.42 ms** | **14.5%** | ✅ |
| **Headroom** | **14.25 ms** | **85%** | ✅ |

</div>
<div class="astra-node" style="flex:1">
<strong>Scalability projections</strong>

| Entity Count | p99 Estimate | Feasibility |
|--------------|--------------|-------------|
| 1,000 | 2.42 ms | ✅ 85% headroom |
| 5,000 | ~8-10 ms | ✅ 40-50% headroom |
| 10,000 | ~15-18 ms | ⚠️ Near budget |
| 20,000+ | >30 ms | ❌ Requires 30 FPS |

</div>
</div>
</div>

<section class="astra-section">
<div class="astra-section-heading">
<span class="astra-kicker">60 FPS capacity</span>
<h2>ECS time at scale</h2>
</div>

| Entity Count | ECS Time | Budget Used |
|--------------|----------|-------------|
| 1,000 | ~85µs | 0.51% |
| 5,000 | ~529µs | 3.17% |
| 10,000 | ~1ms | ~6% |

</section>

<section class="astra-section">
<div class="astra-section-heading">
<span class="astra-kicker">Reproduce locally</span>
<h2>Running benchmarks</h2>
<p>
  Every benchmark can be reproduced with a single command.
  Criterion.rs provides confidence intervals and statistical rigor.
</p>
</div>

<div class="astra-split">
<article class="astra-card astra-card--panel">
<h3>Full suite</h3>

```bash
# Run all Criterion benchmarks
cargo bench --workspace

# Run with odyssey automation (captures logs)
./scripts/benchmark_odyssey.ps1 \
  -OutDir benchmark_results/$(Get-Date -Format 'yyyy-MM-dd')
```

</article>
<article class="astra-card astra-card--panel">
<h3>Per-crate benchmarks</h3>

```bash
# ECS benchmarks
cargo bench -p astraweave-ecs

# AI benchmarks
cargo bench -p astraweave-ai

# Physics benchmarks
cargo bench -p astraweave-physics

# Render benchmarks
cargo bench -p astraweave-render
```

</article>
</div>

<div class="astra-card astra-card--panel" style="margin-top:1rem">
<h3>Generating HTML reports</h3>

```bash
# Open Criterion HTML report
cargo bench -p astraweave-ecs -- --save-baseline main
# Reports at: target/criterion/*/report/index.html
```

</div>
</section>

<div class="astra-band">
<div class="astra-section-heading astra-section-heading--compact">
<span class="astra-eyebrow">Philosophy</span>
<h2>Benchmarks as verification artifacts, not marketing numbers.</h2>
</div>
<div class="astra-flow">
<div class="astra-node">
  <strong>Reproducibility</strong>
  <span>Every claimed measurement has a command that reproduces it.</span>
</div>
<div class="astra-node">
  <strong>Raw logs</strong>
  <span>All runs capture raw output for auditing.</span>
</div>
<div class="astra-node">
  <strong>Statistical rigor</strong>
  <span>Criterion.rs provides confidence intervals.</span>
</div>
<div class="astra-node">
  <strong>Adversarial testing</strong>
  <span>22 adversarial benchmark sections stress edge cases.</span>
</div>
<div class="astra-node">
  <strong>Real hardware</strong>
  <span>No synthetic workloads — real game scenarios.</span>
</div>
</div>
</div>

<section class="astra-section">
<div class="astra-section-heading">
<span class="astra-kicker">Further reading</span>
<h2>See also</h2>
</div>
<article class="astra-card astra-card--panel">
<ul class="astra-link-list">
  <li><span>Methodology</span><a href="./methodology.md">How benchmarks are measured</a></li>
  <li><span>Optimization guide</span><a href="./optimization.md">Performance improvement techniques</a></li>
  <li><span>Performance budgets</span><a href="./budgets.md">Frame budget allocation</a></li>
  <li><span>Master benchmark report</span><a href="../../masters/MASTER_BENCHMARK_REPORT.md">Complete raw data</a></li>
</ul>
</article>
</section>

</div>
