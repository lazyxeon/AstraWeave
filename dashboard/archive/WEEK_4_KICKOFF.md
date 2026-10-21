# Week 4 Kickoff: Phase B Expansion Sprint 🚀

**Status**: 🟢 **READY TO START**  
**Date**: October 10-16, 2025  
**Duration**: 5-7 days (estimated)  
**Priority**: 🔴 CRITICAL SYSTEMS + 🟡 INFRASTRUCTURE + 🟢 DEMO POLISH

---

## Executive Summary

**Goal: Launch Phase B (Expansion) with async physics, advanced terrain streaming, automated performance telemetry, continued code hardening, production-ready LLM orchestration, and polished Veilweaver demo showcasing all Week 1-4 achievements.**

Week 4 transitions from Phase A (Foundation - Weeks 1-3) to Phase B (Expansion - Weeks 4-8). Building on Week 3's optimization wins (60 FPS terrain, real-time AI, CI pipeline, physics validation), we now scale systems for production workloads, automate performance monitoring, and prepare the first playable demo.

### Week 4 Achievements Target

| Action | System | Baseline → Target | Improvement | Impact |
|--------|--------|------------------|-------------|--------|
| **13** | **Async Physics** | 6.52µs → <4ms (4 threads) | **1.8× speedup** | **5,000+ NPCs @ 60 FPS** |
| **14** | **Terrain Streaming** | 15.06ms → <18ms (LOD) | **Background loading** | **Infinite worlds** |
| **15** | **Benchmark Dashboard** | Manual → Automated | **30-day trends** | **Regression alerts** |
| **16** | **Code Quality** | 579 unwraps → <540 | **40 production** | **Hardened codebase** |
| **17** | **LLM Orchestrator** | Prototype → Production | **95% validity** | **Real AI agents** |
| **18** | **Veilweaver Demo** | N/A → Playable | **10-min soak** | **Showcase ready** |

---

## Strategic Context

### Phase A Completion (Weeks 1-3) ✅

**Achievements**:
- ✅ **Week 1**: GPU skinning, combat physics, unwrap audit (637 catalogued)
- ✅ **Week 2**: 25 benchmarks, S-tier AI (11,500 agents @ 60 FPS), 50 unwraps fixed
- ✅ **Week 3**: 60 FPS terrain (15.06ms), real-time GOAP (1.01µs), CI pipeline (30 benchmarks), physics suite (34 variants)

**Foundation Complete**: ECS, AI core loop, rendering, physics, terrain, audio, navigation all production-ready with performance baselines and CI protection.

### Phase B Goals (Weeks 4-8)

**Focus**: Scale systems for AAA workloads, automate quality gates, deliver playable demos

**Deliverables**:
1. **Async Physics**: Multi-threaded simulation (5,000+ NPCs)
2. **Advanced Streaming**: Infinite worlds with LOD transitions
3. **Performance Telemetry**: Automated regression detection + historical trends
4. **LLM Production**: Scheduler, security, evaluation harness
5. **Veilweaver Demo**: First playable slice (10-minute experience)

**Success Metrics**:
- 5,000+ NPCs @ 60 FPS (physics parallelized)
- <2ms frame-time spikes (streaming hitches eliminated)
- Automated regression alerts (>10% triggers CI issue)
- 95%+ LLM plan validity (evaluation harness)
- 10-minute demo soak @ 60 FPS (no crashes)

---

## Week 4 Actions (6 Actions)

### 🔴 Critical Systems (Actions 13-14)

#### Action 13: Async Physics Pipeline (Rayon Parallelization)
**Priority**: 🔴 CRITICAL  
**Estimated Time**: 8-12 hours  
**Assignee**: _________________  
**Files**:
- `astraweave-physics/src/async_scheduler.rs` (NEW - 250 LOC)
- `astraweave-physics/src/controller.rs` (MODIFY - add async path)
- `astraweave-physics/benches/physics_async.rs` (NEW - 180 LOC)
- `astraweave-physics/tests/determinism.rs` (NEW - 120 LOC)

**Implementation Plan**:

1. **Rayon Task Graph** (4-6 hours):
   - Profile current `PhysicsController::step_world` (single-thread baseline)
   - Design 3-stage pipeline: broad-phase → narrow-phase → integration
   - Implement `AsyncPhysicsScheduler` with Rayon parallel iterators
   - Add deterministic barrier ordering (ensure determinism)
   - Gate behind `cfg(feature = "async-physics")` feature flag

2. **Telemetry & Profiling** (2-3 hours):
   - Add `PhysicsStepProfile` struct (timing per stage)
   - Expose `get_last_step_profile()` API
   - Integrate with benchmark dashboard JSON export
   - Add debug overlay for frame-time breakdown

3. **Benchmarks & Testing** (2-3 hours):
   - Add `physics_async_full_tick` benchmark (2,500 NPCs)
   - Add `physics_async_batch` benchmark (varying thread counts)
   - Add `physics_async_determinism` test (100 seeds, verify identical results)
   - Update threshold JSON with async variants

**Acceptance Criteria**:
- [ ] 1.8× speedup vs single-thread on 2,500 NPC benchmark (4 threads)
- [ ] Determinism test passes 100/100 seeds (identical results)
- [ ] `cargo bench -p astraweave-physics --bench physics_async --features async-physics` shows <4.0ms full tick
- [ ] Telemetry JSON exported to `target/benchmark-data/physics_async_profile.json`
- [ ] Feature flag documented in README + Cargo.toml
- [ ] Zero regressions in existing physics benchmarks

**Validation Commands**:
```powershell
# Build with async physics feature
cargo build -p astraweave-physics --features async-physics --release

# Run determinism test (100 seeds)
cargo test -p astraweave-physics determinism --features async-physics -- --nocapture

# Run async benchmarks
cargo bench -p astraweave-physics --bench physics_async --features async-physics

# Validate thresholds
./scripts/check_benchmark_thresholds.ps1 -ShowDetails

# Check for regressions
cargo bench -p astraweave-physics --bench character_controller
cargo bench -p astraweave-physics --bench rigid_body
```

**Expected Metrics**:
| Scenario | Single-Thread | 4-Thread Async | Speedup |
|----------|--------------|----------------|---------|
| **2,500 NPCs** | 6.52µs × 2500 = 16.3ms | <4.0ms | **4.1× faster** |
| **5,000 NPCs** | ~32.6ms (below 60 FPS) | <8.0ms | **4.1× faster** |
| **Broad-Phase** | ~40% of time | ~25% (parallel) | **1.6× faster** |
| **Narrow-Phase** | ~35% of time | ~20% (parallel) | **1.75× faster** |
| **Integration** | ~25% of time | ~15% (parallel) | **1.67× faster** |

**Risk Mitigation**:
- **Determinism Drift**: Add per-stage checksums, compare against single-thread baseline
- **Thread Contention**: Profile with `perf` / `cargo flamegraph`, tune Rayon pool size
- **Feature Flag Stability**: Keep single-thread path as default, async opt-in only

---

#### Action 14: Terrain Streaming Phase 2 (LOD + Background Loading)
**Priority**: 🔴 CRITICAL  
**Estimated Time**: 10-14 hours  
**Assignee**: _________________  
**Files**:
- `astraweave-scene/src/streaming/background_loader.rs` (NEW - 300 LOC)
- `astraweave-scene/src/streaming/lod_manager.rs` (NEW - 250 LOC)
- `astraweave-terrain/src/lod_transition.rs` (NEW - 180 LOC)
- `astraweave-terrain/tests/streaming_integrity.rs` (NEW - 200 LOC)
- `examples/unified_showcase/src/streaming_diagnostics.rs` (NEW - 120 LOC)

**Implementation Plan**:

1. **Background Chunk Loader** (4-6 hours):
   - Design priority-based chunk queue (distance-from-camera sorting)
   - Implement `BackgroundChunkLoader` with async task pool (tokio)
   - Add chunk prefetch strategy (load N chunks ahead in camera direction)
   - Integrate with existing `WorldPartition` system
   - Add memory budget enforcement (evict distant chunks)

2. **LOD Transitions** (3-4 hours):
   - Design LOD levels: L0 (full detail), L1 (half), L2 (quarter), L3 (skybox)
   - Implement hysteresis curve (prevent LOD popping)
   - Add blend zones (cross-fade meshes during transitions)
   - Expose tuning constants in `streaming_config.toml`

3. **Diagnostics & Testing** (3-4 hours):
   - Add streaming overlay (chunk status, memory usage, queue depth)
   - Create `terrain_streaming_integrity.rs` soak test (1,024 ticks, randomized camera)
   - Add benchmark `terrain_streaming_background` (worst-case scenario)
   - Update threshold JSON

**Acceptance Criteria**:
- [ ] Frame-time spikes <2ms p99 during soak test (1,024 ticks)
- [ ] Memory delta <6% vs baseline (no leaks)
- [ ] Soak test passes with zero panics/errors
- [ ] `cargo bench -p astraweave-terrain --bench terrain_streaming_background` <18ms worst-case
- [ ] LOD transitions visually smooth (no popping, verified manually)
- [ ] Diagnostics overlay shows real-time stats

**Validation Commands**:
```powershell
# Run soak test (1,024 ticks, randomized camera)
cargo test -p astraweave-terrain streaming_integrity --release -- --nocapture

# Run background streaming benchmark
cargo bench -p astraweave-terrain --bench terrain_streaming_background

# Run unified showcase with diagnostics
cargo run -p unified_showcase --release --features streaming-diagnostics

# Validate thresholds
./scripts/check_benchmark_thresholds.ps1 -ShowDetails
```

**Expected Metrics**:
| Metric | Baseline | Target | Achievement |
|--------|----------|--------|-------------|
| **Frame-Time p99** | 15.06ms | <17ms | <2ms spikes |
| **Memory Delta** | 0% | <10% | <6% growth |
| **Chunk Load Time** | 15.06ms | <18ms | Background async |
| **LOD Transitions** | N/A | Smooth | <1 frame blend |
| **Soak Test** | N/A | 1,024 ticks | Zero crashes |

**Risk Mitigation**:
- **LOD Popping**: Implement hysteresis with 10% margin, test with fast camera panning
- **Memory Leaks**: Add `drop` logging, run valgrind/heaptrack on soak test
- **Task Pool Starvation**: Limit concurrent loads to 4, profile task scheduler

---

### 🟡 Infrastructure & Tooling (Actions 15-16)

#### Action 15: Benchmark Dashboard Automation
**Priority**: 🟡 INFRASTRUCTURE  
**Estimated Time**: 6-8 hours  
**Assignee**: _________________  
**Files**:
- `.github/scripts/benchmark-runner.sh` (MODIFY - add JSONL export)
- `.github/workflows/benchmark-nightly.yml` (NEW - 120 LOC)
- `tools/benchmark-dashboard/index.html` (NEW - 400 LOC, d3.js)
- `tools/benchmark-dashboard/process-data.js` (NEW - 250 LOC)
- `scripts/check_benchmark_thresholds.ps1` (MODIFY - add GitHub issue creation)
- `CI_BENCHMARK_PIPELINE.md` (MODIFY - add dashboard section)

**Implementation Plan**:

1. **JSONL History Export** (2-3 hours):
   - Modify `benchmark-runner.sh` to emit JSONL (one entry per benchmark run)
   - Add fields: `timestamp`, `benchmark_name`, `value`, `unit`, `git_sha`, `branch`
   - Store in `target/benchmark-data/history.jsonl`
   - Add rotation script (keep 30 days)

2. **GitHub Pages Dashboard** (3-4 hours):
   - Build static HTML dashboard with d3.js line charts
   - Add filters: system (ECS, AI, physics, terrain), time range (7d, 30d)
   - Add drill-down views for async physics, terrain streaming, LLM harness
   - Deploy to `gh-pages` branch via GitHub Action

3. **CI Alert Integration** (1-2 hours):
   - Extend `check_benchmark_thresholds.ps1` to create GitHub issues
   - Add issue template: `BENCHMARK_REGRESSION.md`
   - Wire into nightly workflow (auto-open issue on >10% regression)
   - Update `CI_BENCHMARK_PIPELINE.md` documentation

**Acceptance Criteria**:
- [ ] Nightly GitHub Action publishes dashboard (accessible at `https://<user>.github.io/AstraWeave-AI-Native-Gaming-Engine/benchmarks`)
- [ ] Dashboard shows 30-day trend lines for all 30+ benchmarks
- [ ] Regression >10% auto-opens GitHub issue with template
- [ ] JSONL history includes all required fields (timestamp, sha, value, etc.)
- [ ] Documentation updated with dashboard usage guide

**Validation Commands**:
```powershell
# Run benchmark with JSONL export
./.github/scripts/benchmark-runner.sh --export-jsonl

# Check JSONL output
cat target/benchmark-data/history.jsonl | jq .

# Test threshold script with issue creation (dry-run)
./scripts/check_benchmark_thresholds.ps1 -ShowDetails -CreateIssue -DryRun

# Build dashboard locally
cd tools/benchmark-dashboard
python -m http.server 8000
# Open http://localhost:8000 in browser
```

**Expected Metrics**:
- JSONL export adds <500ms to benchmark runtime
- Dashboard loads in <2s (30-day dataset)
- Issue creation triggers within 5 minutes of regression detection
- 30-day retention keeps dataset <10MB

**Risk Mitigation**:
- **Data Volume**: Enforce 30-day rotation, compress old entries
- **Dashboard Complexity**: Use CDN-hosted d3.js, keep HTML <500KB
- **False Positives**: Add manual issue close workflow, tune thresholds

---

#### Action 16: Unwrap Remediation Phase 3
**Priority**: 🟡 CODE QUALITY  
**Estimated Time**: 4-6 hours  
**Assignee**: _________________  
**Files**:
- `astraweave-render/src/**/*.rs` (~15 unwraps)
- `astraweave-scene/src/**/*.rs` (~12 unwraps)
- `astraweave-nav/src/**/*.rs` (~13 unwraps)
- `unwrap_audit_report.csv` (UPDATE)
- `UNWRAP_REMEDIATION_LOG.md` (NEW - 300 LOC)

**Implementation Plan**:

1. **Prioritize Top 40 Unwraps** (1 hour):
   - Run `./scripts/audit_unwrap.ps1 -ExportCsv` to refresh audit
   - Sort by P0 priority (production code, public APIs)
   - Target crates: render (15), scene (12), nav (13)

2. **Apply Safe Patterns** (2-3 hours):
   - Pattern 1: Default fallback (`unwrap_or(default)`)
   - Pattern 2: Early return (`let Some(x) = opt else { return; }`)
   - Pattern 3: Graceful skip (`if let Some(x) = opt { ... }`)
   - Pattern 4: Error propagation (`?` operator)
   - Pattern 5: Logging + default (`warn!("..."); return default;`)
   - Pattern 6: Documented panic (`expect("...")`)

3. **Testing & Documentation** (1-2 hours):
   - Add regression tests where feasible (test unwrap edge cases)
   - Update `unwrap_audit_report.csv` (mark fixed unwraps)
   - Create `UNWRAP_REMEDIATION_LOG.md` (summary of changes)
   - Run full test suite to ensure no regressions

**Acceptance Criteria**:
- [ ] ≥40 production unwraps removed (render, scene, nav)
- [ ] Remaining P0 count <500 (down from 579)
- [ ] New tests passing (regression coverage)
- [ ] Audit CSV updated with fixed entries
- [ ] `UNWRAP_REMEDIATION_LOG.md` documents patterns used and velocity

**Validation Commands**:
```powershell
# Refresh unwrap audit
./scripts/audit_unwrap.ps1 -ExportCsv

# Test affected crates
cargo test -p astraweave-render -p astraweave-scene -p astraweave-nav

# Check for benchmark regressions
cargo bench -p astraweave-render --bench material_loading
cargo bench -p astraweave-scene --bench cell_streaming

# Verify P0 count
(Import-Csv unwrap_audit_report.csv | Where-Object Priority -eq "P0").Count
# Expected: <500
```

**Expected Metrics**:
- 40 unwraps fixed in 4-6 hours (6.7-10 unwraps/hour velocity)
- Remaining P0: 579 → <500 (13.7% reduction)
- Total fixed (Weeks 2-4): 50 + 8 + 40 = 98 (15.4% of 637)
- Zero test regressions

**Risk Mitigation**:
- **Breaking Changes**: Focus on internal code, avoid public API changes
- **Test Coverage Gaps**: Add tests for new error paths
- **Velocity Slowdown**: Target easy wins first (default fallbacks)

---

### 🟢 AI & Gameplay Expansion (Actions 17-18)

#### Action 17: LLM Orchestrator Hardening
**Priority**: 🟢 FEATURE  
**Estimated Time**: 10-14 hours  
**Assignee**: _________________  
**Files**:
- `astraweave-llm/src/scheduler.rs` (NEW - 300 LOC)
- `astraweave-llm/src/tool_guard.rs` (NEW - 200 LOC)
- `astraweave-llm-eval/src/lib.rs` (NEW - 400 LOC)
- `astraweave-llm-eval/tests/evaluation_suite.rs` (NEW - 600 LOC, 20 scenarios)
- `.github/workflows/llm-evaluation.yml` (NEW - 80 LOC)
- `docs/llm_architecture_v1.1.md` (NEW - 500+ LOC)

**Implementation Plan**:

1. **LlmScheduler Resource** (3-4 hours):
   - Implement ECS resource for async LLM request polling
   - Add request queue with priority (high/normal/low)
   - Integrate with embeddings/context/prompts crates
   - Add timeout handling (5s default, configurable)
   - Expose `poll_llm_requests()` system for ECS integration

2. **ToolGuard & Security** (2-3 hours):
   - Implement `ToolGuard` validation layer
   - Add `ActionRegistry` with allowlist/denylist
   - Validate tool calls against world state (e.g., "attack" requires enemy in range)
   - Add security tests (reject invalid tools, out-of-range actions)

3. **Evaluation Harness** (4-6 hours):
   - Create 20 test scenarios (combat, exploration, stealth, support, puzzle)
   - Implement scoring: validity (40%), goal (30%), safety (15%), coherence (15%)
   - Add `MockLlmClient` for deterministic testing
   - Wire into CI job `llm_evaluation`

4. **Documentation** (1-2 hours):
   - Write `docs/llm_architecture_v1.1.md` (architecture guide)
   - Document scheduler usage, security model, evaluation workflow
   - Add examples for extending scenario suite

**Acceptance Criteria**:
- [ ] Harness runtime <30s (20 scenarios)
- [ ] Mock client achieves ≥95% plan validity
- [ ] Security tests catch invalid tool usage (100% rejection rate)
- [ ] CI job `llm_evaluation` passes
- [ ] Architecture doc ≥500 lines with diagrams

**Validation Commands**:
```powershell
# Run evaluation harness (20 scenarios)
cargo test -p astraweave-llm-eval --release -- --nocapture

# Run security tests
cargo test -p astraweave-llm tool_guard --nocapture

# Run benchmarks
cargo bench -p astraweave-llm-eval --bench evaluate_mock_llm

# Validate CI job locally
act -j llm-evaluation  # GitHub Actions local runner
```

**Expected Metrics**:
| Scenario Type | Mock LLM Validity | Goal Achievement | Safety Score |
|--------------|------------------|------------------|--------------|
| **Combat** | 98% | 90% | 95% |
| **Exploration** | 96% | 85% | 100% |
| **Stealth** | 94% | 80% | 98% |
| **Support** | 97% | 88% | 100% |
| **Puzzle** | 92% | 75% | 100% |
| **Overall** | **95.4%** | **83.6%** | **98.6%** |

**Risk Mitigation**:
- **Flaky Tests**: Use deterministic mock client, seed RNG
- **Performance Regression**: Gate real API behind env flag
- **Scenario Complexity**: Start with simple scenarios, iterate

---

#### Action 18: Veilweaver Demo Polish
**Priority**: 🟢 DEMO  
**Estimated Time**: 6-8 hours  
**Assignee**: _________________  
**Files**:
- `examples/veilweaver_demo/src/main.rs` (NEW - 400 LOC)
- `examples/veilweaver_demo/src/telemetry_hud.rs` (NEW - 200 LOC)
- `examples/veilweaver_demo/src/scripted_encounter.rs` (NEW - 300 LOC)
- `examples/veilweaver_demo/assets/encounters/tutorial.ron` (NEW)
- `VEILWEAVER_DEMO_BRIEF.md` (NEW - 200 LOC)

**Implementation Plan**:

1. **Integrate Physics/AI Metrics** (2-3 hours):
   - Add telemetry HUD (FPS, frame-time, physics step time, AI planning time)
   - Integrate async physics toggle (feature flag)
   - Add terrain streaming diagnostics overlay
   - Export telemetry JSON for dashboard ingestion

2. **Scripted GOAP Encounter** (2-3 hours):
   - Design tutorial encounter (player + 3 companions vs 5 enemies)
   - Use cached GOAP plans for predictable behavior
   - Add victory/defeat conditions
   - Integrate with weaving system (fate-weaving mechanic)

3. **Demo Polish & Soak Test** (2-3 hours):
   - Add camera smoothing, UI polish, audio integration
   - Run 10-minute soak test (capture telemetry)
   - Fix visual glitches, performance hitches
   - Prepare `VEILWEAVER_DEMO_BRIEF.md` (3-slide summary)

**Acceptance Criteria**:
- [ ] Demo maintains 60 FPS over 10-minute soak
- [ ] Telemetry exported to `target/telemetry/veilweaver_demo.json`
- [ ] Visual polish sign-off (team review)
- [ ] Demo brief ready (≥200 lines, includes screenshots)
- [ ] Zero crashes during soak test

**Validation Commands**:
```powershell
# Run demo with telemetry
cargo run -p veilweaver_demo --release --features telemetry

# Run 10-minute soak (automated)
cargo test -p veilweaver_demo soak_test --release -- --nocapture

# Check telemetry output
cat target/telemetry/veilweaver_demo.json | jq .

# Validate FPS consistency
# (Telemetry JSON should show p50/p95/p99 all >60 FPS)
```

**Expected Metrics**:
- FPS: p50=60, p95=60, p99=58 (consistent 60 FPS)
- Frame-time: p95 <16.67ms (60 FPS budget)
- Physics: <4ms (async enabled)
- AI Planning: <2ms (cached GOAP)
- Memory: <2GB RSS, <1% growth over 10 min

**Risk Mitigation**:
- **Performance Regression**: Profile with `cargo flamegraph`, optimize hotspots
- **Visual Glitches**: Add debug overlay for LOD transitions, physics bodies
- **Soak Test Failures**: Add watchdog timer, auto-restart on hang

---

## Week 4 Timeline

### Day 1-2 (Oct 10-11): Critical Systems (Actions 13-14)
- **Morning (Action 13)**: Async physics profiling, Rayon task graph design
- **Afternoon (Action 13)**: Implementation, determinism tests
- **Evening**: Review, benchmark runs

- **Morning (Action 14)**: Background chunk loader design, priority queues
- **Afternoon (Action 14)**: LOD transitions, hysteresis curve
- **Evening**: Soak test, diagnostics overlay

**Daily Coordination**:
- Shared profiling review (physics + streaming metrics)
- Benchmark captures for dashboard dry-run

---

### Day 3-4 (Oct 12-13): Infrastructure & Quality (Actions 15-16)
- **Morning (Action 15)**: JSONL export, benchmark-runner modifications
- **Afternoon (Action 15)**: Dashboard build (d3.js), GitHub Pages deploy
- **Evening**: CI alert integration, documentation

- **Morning (Action 16)**: Unwrap audit refresh, prioritization
- **Afternoon (Action 16)**: Safe pattern application (render, scene, nav)
- **Evening**: Testing, audit CSV update

**Daily Coordination**:
- Dashboard dry-run (validate JSONL format, chart rendering)
- Benchmark regressions check (ensure unwrap fixes don't slow code)

---

### Day 5-6 (Oct 14-15): LLM & Demo (Actions 17-18)
- **Morning (Action 17)**: LlmScheduler resource, request queue
- **Afternoon (Action 17)**: ToolGuard, ActionRegistry, security tests
- **Evening**: Evaluation harness (20 scenarios)

- **Morning (Action 18)**: Telemetry HUD, physics/AI integration
- **Afternoon (Action 18)**: Scripted GOAP encounter, weaving integration
- **Evening**: Demo polish, 10-minute soak test

**Daily Coordination**:
- Nightly CI + hello_companion validation
- LLM harness + demo soak data for dashboard

---

### Day 7 (Oct 16): Integration & Completion
- **Morning**: Integrated demo review (Actions 13-18 working together)
- **Afternoon**: Metrics snapshot, Week 4 completion report
- **Evening**: Celebration, Week 5 planning kickoff

---

## Validation & Reporting

### Benchmarks
- Extend `benchmark_thresholds.json` with:
  - `physics_async_full_tick` (4 threads, <4ms, 20% fail threshold)
  - `terrain_streaming_background` (<18ms, 25% fail threshold)
  - `llm_eval_mock_client` (<5ms per scenario, 15% fail threshold)
- Run threshold validation before all merges:
  ```powershell
  ./scripts/check_benchmark_thresholds.ps1 -Strict
  ```

### Testing
```powershell
# Core physics tests (determinism critical)
cargo test -p astraweave-physics --features async-physics

# Terrain streaming integrity
cargo test -p astraweave-terrain streaming_integrity --release

# LLM evaluation suite
cargo test -p astraweave-llm-eval --release

# Demo soak test
cargo test -p veilweaver_demo soak_test --release
```

### Documentation
- Update `BASELINE_METRICS.md` with Week 4 metrics
- Create `WEEK_4_COMPLETE.md` (completion summary)
- Refresh `docs/llm_architecture_v1.1.md` (LLM guide)
- Publish dashboard usage notes (`CI_BENCHMARK_PIPELINE.md`)

---

## Risk Management

### Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Async physics determinism drift** | 🔴 HIGH | 🟡 MEDIUM | Per-stage checksums, 100-seed validation |
| **LOD popping artifacts** | 🟡 MEDIUM | 🟡 MEDIUM | Hysteresis curve, manual visual QA |
| **Dashboard data volume** | 🟡 MEDIUM | 🟢 LOW | 30-day retention, compression |
| **LLM evaluation flakiness** | 🟡 MEDIUM | 🟡 MEDIUM | Mock client default, seed RNG |
| **Demo performance regression** | 🔴 HIGH | 🟢 LOW | Profiling, flamegraph analysis |

### Schedule Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Action 13 overrun** | 🔴 HIGH | 🟡 MEDIUM | Simplify task graph (2-stage vs 3-stage) |
| **Action 14 complexity** | 🔴 HIGH | 🟡 MEDIUM | Skip LOD blending (hard cut acceptable) |
| **Action 17 scope creep** | 🟡 MEDIUM | 🔴 HIGH | Start with 10 scenarios, expand later |
| **Integration delays** | 🟡 MEDIUM | 🟡 MEDIUM | Daily sync, blocked task escalation |

### Dependency Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| **Rayon 1.x** | API changes | Lock to 1.10.0, test upgrade in isolated branch |
| **d3.js CDN** | Availability | Host local copy as fallback |
| **GitHub Pages** | Deploy failures | Manual deploy script as backup |
| **Rapier3D** | Determinism bugs | Pin to 0.17.2, upstream issue tracking |

---

## Success Metrics

### Performance Targets

| System | Baseline | Week 4 Target | Stretch Goal |
|--------|----------|---------------|--------------|
| **Async Physics** | 6.52µs/NPC | <4ms (2,500 NPCs) | <2ms (5,000 NPCs) |
| **Terrain Streaming** | 15.06ms | <18ms (LOD) | <15ms (optimized) |
| **LLM Harness** | N/A | <30s (20 scenarios) | <20s (optimized) |
| **Demo FPS** | N/A | p95 >58 FPS | p99 =60 FPS |

### Quality Targets

| Metric | Baseline | Week 4 Target | Stretch Goal |
|--------|----------|---------------|--------------|
| **Unwraps (P0)** | 579 | <500 | <450 |
| **LLM Validity** | Unknown | ≥95% | ≥98% |
| **Benchmark Coverage** | 30 | 36 | 40 |
| **CI Automation** | Manual | Automated alerts | Full dashboard |

### Deliverable Targets

| Deliverable | Status | Target | Stretch |
|-------------|--------|--------|---------|
| **Async Physics** | Not Started | Feature-complete | Production default |
| **Terrain Streaming** | Not Started | Working | Visually polished |
| **Dashboard** | Not Started | Published | Interactive drilldowns |
| **LLM Harness** | Not Started | 20 scenarios | 30 scenarios |
| **Veilweaver Demo** | Not Started | 10-min soak | Public-ready |

---

## Celebration Points 🎉

### Performance Milestones
- 🎯 **5,000+ NPCs @ 60 FPS**: Async physics enables AAA-scale crowds
- 🚀 **Infinite Worlds**: Background streaming eliminates world size limits
- 💪 **Real-Time AI**: LLM orchestrator handles production workloads
- ⚡ **Zero Hitches**: <2ms frame-time spikes during streaming
- 🔬 **Automated Quality**: Dashboard + alerts protect all gains

### Infrastructure Wins
- 🤖 **30-Day Trends**: Historical performance visualization
- 📊 **Regression Alerts**: >10% triggers auto-issue creation
- 🛡️ **Code Hardening**: 98 total unwraps eliminated (15.4%)
- 📈 **36+ Benchmarks**: Comprehensive system coverage
- 🎨 **Developer UX**: Live dashboard, instant feedback

### Efficiency Achievements
- ⚡ **Phase B Launch**: Foundation → Expansion in 4 weeks
- 🏃 **Proven Velocity**: Weeks 1-3 efficiency maintained
- 🎯 **100% Week 4**: All 6 actions complete
- 🚀 **Production-Ready**: LLM, physics, streaming hardened

---

## Next Steps

### Immediate (Start Now)
1. **Assign Actions**: Designate owners for Actions 13-18
2. **Setup Branches**: Create feature branches for each action
3. **Kick Off Action 13**: Begin async physics profiling

### Short-Term (Week 4)
1. **Daily Standups**: Sync progress, blockers, metrics
2. **Benchmark Snapshots**: Capture metrics after each action
3. **Integration Testing**: Validate Actions 13-18 working together

### Medium-Term (Week 5-8)
1. **Phase B Execution**: GPU particles, networking, advanced rendering
2. **Veilweaver Expansion**: Extend demo to 30-minute experience
3. **Community Prep**: Documentation, tutorials, videos

---

## Document Structure

### Week 4 Deliverables
- `WEEK_4_KICKOFF.md` (this file) - Planning and overview
- `WEEK_4_ACTION_13_COMPLETE.md` - Async physics completion report
- `WEEK_4_ACTION_14_COMPLETE.md` - Terrain streaming completion report
- `WEEK_4_ACTION_15_COMPLETE.md` - Dashboard automation completion report
- `WEEK_4_ACTION_16_COMPLETE.md` - Unwrap remediation completion report
- `WEEK_4_ACTION_17_COMPLETE.md` - LLM orchestrator completion report
- `WEEK_4_ACTION_18_COMPLETE.md` - Veilweaver demo completion report
- `WEEK_4_COMPLETE.md` - Overall Week 4 summary

### Related Documents
- `WEEK_3_COMPLETE.md` - Previous week summary
- `BASELINE_METRICS.md` - Performance metrics dashboard
- `IMPLEMENTATION_PLANS_INDEX.md` - Strategic plan navigator
- `LONG_HORIZON_STRATEGIC_PLAN.md` - 12-month roadmap
- `CI_BENCHMARK_PIPELINE.md` - Automation documentation

---

**Week 4 Status**: 🟢 **READY TO START**  
**Phase B Status**: 🟡 **LAUNCHING**  
**Next**: Action 13 - Async Physics Pipeline

**Let's Build**: 🎊 **Week 4 begins - Phase B expansion, production-scale systems, automated quality, playable demo!** 🎊

---

**Report Generated**: October 10, 2025  
**Engineer**: GitHub Copilot (AI-Native Development Experiment)  
**Session**: Week 4 Kickoff - Phase B Expansion Sprint

