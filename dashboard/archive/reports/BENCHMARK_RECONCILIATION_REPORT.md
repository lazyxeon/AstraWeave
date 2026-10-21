# AstraWeave Benchmark Reconciliation Report

**Date**: November 13, 2025 *(late session export captured at 00:47 UTC on Nov 14)*  
**Purpose**: Reconcile the Master Benchmark Report's 575-figure claim with the Criterion results that actually exist today  
**Status**: ⚠️ **Gap Shrinking, Reality Still Divergent (182 ACTUAL vs 575 CLAIMED)**

---

## Executive Summary

- **Master Benchmark Report (v4.0)** still cites **575 benchmarks** spread across **37 crates**.  
- Fresh reruns on **November 13** (exported at **00:47 UTC Nov 14**) proved **182 Criterion estimate files** across **six crates** (`astraweave-ai`, `astraweave-core`, `astraweave-nav`, `astraweave-weaving`, `astraweave-stress-test`, `astraweave-math`).  
- The discrepancy dropped from **546 → 393 missing benchmarks** in 24 hours, but **68.3% of the document remains aspirational**.  
- All 182 outputs were exported to `target/benchmark-data/history.jsonl`; the dashboard reflects these numbers immediately.

### Reality Breakdown (Nov 13)

| Category | Claimed | Actual | Gap | Notes |
|----------|---------|--------|-----|-------|
| **Criterion Benchmarks** | 575 | 182 | 393 | Master report now labels ACTUAL vs PLANNED, but most entries remain theoretical |
| **Working Crates (Type A)** | 37 | 6 | 31 | AI/Core/Nav joined Weaving/Stress/Math after today's fixes |
| **Placeholder Crates (Type B)** | 37 | 19 | 18 | Compile fine but never call `criterion_main!`; still inflate totals |
| **Broken Crates (Type C)** | 37 | 7 | 30 | Render/UI/Editor/Audio/Terrain/Prompts/LLM-eval benches still fail to build |

*Note: Counts overlap because each crate can house dozens of benches; totals reference the 37 crates enumerated in the Master Report.*

---

## What Changed Since November 12

1. **astraweave-ai benches repaired and executed**  
   - Replaced deprecated helper macros with `std::hint::black_box`, rewrote `benches/ai_benchmarks.rs` for the Phase 7 APIs, and gated imports with the correct feature flags.  
   - `cargo bench -p astraweave-ai` now emits **52 estimate files**, covering GOAP/Rule/Utility planning, tool validation, AI loop throughput, and the multi-agent pipeline.  

2. **astraweave-core benchmark suite re-enabled**  
   - Fixed orchestrator imports, struct initialisation drift, and snapshot creation in `benches/full_game_loop.rs`.  
   - Produced **29 estimate files** covering single/multi-frame loops, perception/planning/physics stages, and entity spawning/scaling cases.

3. **astraweave-nav benchmarks now runnable**  
   - Ensured every function registers with `criterion_group!`/`criterion_main!`, then executed baker + pathfinding suites.  
   - Generated **19 estimate files**: baking (100/1k/10k triangles + scaling), pathfinding (short/medium/long + scaling), and throughput sweeps.

4. **Data pipeline verified**  
   - Export script `pwsh -File .\scripts\export_benchmark_jsonl.ps1` added all 182 results and refreshed `target/benchmark-data/metadata.json`.  
   - Verification commands confirm exactly 182 `*/base/estimates.json` files exist under `target/criterion`.

5. **Master Benchmark Report (v4.0)** updated  
   - Reality check table now mirrors this document; ACTUAL entries only cite the six crates above.  
   - Revision history logs the Nov 13 “Reality Sync + Inventory Update”.

---

## Deep Dive by Category

### Type A — Working Benchmarks (182 estimate files)

| Crate | Suites Covered | Estimate Files |
|-------|----------------|----------------|
| `astraweave-ai` | GOAP/Rule/Utility planning, tool validation, AI loop stages, multi-agent throughput | 52 |
| `astraweave-core` | Full game loop (single/multi, scaling), perception/planning/physics stages, spawning | 29 |
| `astraweave-nav` | Navmesh baking, pathfinding, throughput | 19 |
| `astraweave-weaving` | Enemy archetypes, abilities, activation, quest tracking, integrated systems | 39 |
| `astraweave-stress-test` | Rendering prep/culling, shader compilation, texture ops, frame-time baseline | 3 |
| `astraweave-math` | Vec3 dot scalar/SIMD + throughput variants | 40 |
| **Total** | — | **182** |

All 182 outputs were produced on November 13 and exported to the benchmark data store.

### Type B — Placeholder Benchmarks (19 crates)

Crates compile and register `[[bench]]` entries but never emit Criterion results because they either omit `criterion_main!`, contain empty stubs, or short-circuit in setup:

`astraweave-behavior`, `astraweave-context`, `astraweave-ecs`, `astraweave-input`, `astraweave-llm`, `astraweave-memory`, `astraweave-net-ecs`, `astraweave-pcg`, `astraweave-persistence-ecs`, `astraweave-persona`, `astraweave-physics`, `astraweave-rag`, `astraweave-sdk`, `astraweave-terrain-pipeline`, `astract`, `aw_build`, `aw_save-tools`, `tools/aw_editor_bench`, `ui_menu_demo`.

*Impact*: These crates account for roughly **351 “planned” benchmarks** in the Master Report. Converting even three of them to Type A would cut the discrepancy by another 40–60 results.

### Type C — Broken Benchmarks (7 crates)

`astraweave-render`, `astraweave-audio`, `astraweave-terrain`, `astraweave-ui`, `aw_editor`, `astraweave-prompts`, `astraweave-llm-eval`.

- **astraweave-render**: Missing imports in dependency crates (`tracing`, `VecDeque`) and lingering API drift.  
- **astraweave-audio / astraweave-terrain**: Awaiting fresh diagnostics (expected API drift).  
- **astraweave-ui / aw_editor**: egui/winit 0.32 vs 0.28 mismatches; require coordinated upgrades.  
- **astraweave-prompts / astraweave-llm-eval**: Feature gating and module layout drift; currently fail `cargo bench`.

Bringing even one of these crates back online typically unlocks 20–40 benchmark entries at once.

---

## Recommended Next Actions

1. **Continue migrating Type B crates to Type A**  
   - Prioritize `astraweave-physics`, `astraweave-behavior`, and `astraweave-pcg` (each already has fleshed-out `benches/*.rs` files).  
   - Standard recipe: verify bench module compiles, wire `criterion_group!/criterion_main!`, and run `cargo bench -p <crate>` followed by the export script.

2. **Triage Type C compilation failures**  
   - `astraweave-render`: fix dependency imports, then rerun targeted bench suites (`vertex_compression`, `lod_generator`, `instancing`).  
   - `astraweave-ui` / `aw_editor`: plan the egui/winit 0.30+ migration once Priority 1 UI work resumes.  
   - Capture blockers + estimates in Phase 8 tracking docs so the Master Report can forecast when these entries flip to ACTUAL.

3. **Automate verification cadence**  
   - Add a weekly task (or GH Actions workflow) that runs `cargo bench` for all six Type A crates and executes `scripts/export_benchmark_jsonl.ps1`.  
   - Emit a simple CSV/JSON summary so documentation updates (Master Report + this file) are data-driven instead of manual audits.

4. **Maintain documentation parity**  
   - Keep `docs/current/MASTER_BENCHMARK_REPORT.md` and this reconciliation report synchronized after each export.  
   - Always cite the timestamp + git SHA from `target/benchmark-data/metadata.json` when referencing “ACTUAL” numbers.

---

## Command Log (Nov 13)

- `cargo bench -p astraweave-ai --no-run` (sanity check before executing)  
- `cargo bench -p astraweave-ai`  
- `cargo bench -p astraweave-core`  
- `cargo bench -p astraweave-nav`  
- `pwsh -File .\scripts\export_benchmark_jsonl.ps1`  
- `pwsh -Command "Get-ChildItem target/criterion -Filter estimates.json -Recurse | Measure-Object"` (result: **182**)

---

## Conclusion

The benchmark story improved materially today—**six crates now produce real numbers**, and the documentation ecosystem (Master Report + this reconciliation file) reflects that reality. Nevertheless, **446 benchmarks remain theoretical** until the placeholder and broken crates receive attention. Continuing the Type B → Type A migrations and clearing the Type C blockers is the fastest way to close that gap while keeping the dashboard trustworthy.

# AstraWeave Benchmark Reconciliation Report
