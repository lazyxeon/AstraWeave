# AstraWeave Master Test Coverage Report

> **Version**: 5.3.0 | **Date**: 2026-06-10 | **Grade**: B+ | **Tool**: `cargo llvm-cov` (LLVM source-based)

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Workspace Crates** | 130 total (29 measured for coverage; `astraweave-alloc` and `astraweave-camera` added since v5.2.0, both unmeasured) |
| **Total Tests** | ~21,900 (measured crates) / ~39,000+ (workspace; live re-count 2026-06-10: 39,973 markers incl. `#[tokio::test]`) |
| **Primary Tool** | `cargo llvm-cov` (LLVM source-based instrumentation) |
| **Overall Line Coverage** | 59.3% weighted (measured crates, last run 2026-02-25) |
| **High-Coverage Crates (≥85%)** | 14 of 28 (50%) |
| **Miri Validated** | 977 tests, 4 crates, zero UB |
| **Kani Verified** | ~71 proof harnesses, 4 crates, all passing |
| **Mutation Testing** | Wave 1: 767 manual + Wave 2: 1,261 automated + Wave 3: 489 targeted + Wave 4: 411 fluids |

### Tier Overview (Line Coverage, Weighted by LOC)

| Tier | Crates | Target | Weighted Actual | Status |
|------|--------|--------|-----------------|--------|
| **P0 — Core Engine** | 12 | 85% | **55.4%** | Below target |
| **P1 — Important Systems** | 5 | 80% | **58.9%** | Below target |
| **P2 — Support** | 6 | 75% | **73.9%** | Approaching target |

> **Note**: Line coverage percentages were last measured via `cargo llvm-cov` on 2026-02-25 — a **~3.5-month measurement gap** as of this revision (2026-06-10). Test counts were re-audited against current source on 2026-02-27 via `#[test]` marker scanning and spot-re-counted 2026-06-10 (39,973 markers workspace-wide). Coverage percentages have not been re-measured; tests added since Feb 2026 (notably the Net-Trio-Remediation 104-test regression net, the Multi-Tool Sub-phase 4 dispatcher tests, and the Unified Camera contract/parity tests) are not reflected in the percentages.

### Context: Why Coverage Numbers Changed Since v4.2

The v4.2.0 report (2026-02-24) claimed 94.57% weighted coverage. The v5.0.0 audit (2026-02-25) revealed **significant discrepancies** caused by:

1. **Aggressive crate growth**: Many crates doubled or tripled in size since last measurement (e.g., astraweave-ai: 2,455 → 7,768 instrumented lines). New code was added without proportional test coverage.
2. **Dependency code inflation**: `cargo llvm-cov --lib --summary-only` instruments all compiled code including inlined generics from workspace dependencies. Crates like `astraweave-audio` (11,662 instrumented lines vs ~4,200 actual source lines) and `astraweave-rag` (9,957 instrumented lines vs ~3,200 actual) include substantial dependency footprint.
3. **GPU/async untestable code**: Rendering (37,035 lines), terrain (18,826 lines), and audio subsystems contain large GPU-only or async code paths that cannot be exercised in headless `cargo test`.
4. **Prior "source-only" filtering**: The v4.2 report used manual file-level analysis for some crates. The v5.0+ methodology uses raw `cargo llvm-cov` output for full reproducibility.

**Bottom line**: The codebase quality is strong (14 crates at 85–99%), but the overall weighted average is pulled down by large rendering/terrain/gameplay modules with substantial untestable or newly-added code.

---

## Measurement Hardware & Tools

Coverage percentages in this report were measured on **2026-02-25**. Test counts and mutation results audited on **2026-02-27**. All data collected on the following system:

| Component | Specification |
|-----------|---------------|
| **System** | HP Pavilion Gaming Laptop 16-a0xxx |
| **CPU** | Intel Core i5-10300H @ 2.50 GHz (4 cores / 8 threads) |
| **RAM** | 32 GB DDR4 @ 3200 MHz |
| **OS** | Microsoft Windows 11 Home (Build 26200, 64-bit) |
| **Rust Toolchain** | rustc 1.89.0 (29483883e 2025-08-04) |
| **Coverage Tool** | `cargo-llvm-cov` (LLVM source-based instrumentation) |
| **Mutation Tool** | `cargo-mutants` v26.2.0 + `cargo-nextest` v0.9.128 |
| **Memory Safety** | Miri (rustc 1.86.0-nightly, 2025-02-01) |
| **Formal Verification** | Kani (CBMC bounded model checking) |
| **Test Framework** | Standard `#[test]` + `cargo-nextest` |

### Measurement Commands

```powershell
# Coverage measurement (line/region/function)
cargo llvm-cov --no-cfg-coverage --lib -p <crate> --summary-only

# Test count verification (source-of-truth: #[test] marker scanning)
Get-ChildItem "<crate>/src" -Recurse -Filter *.rs | ForEach-Object {
    (Select-String -Path $_.FullName -Pattern '#\[test\]' -AllMatches).Matches.Count
} | Measure-Object -Sum
```

Coverage numbers represent LLVM **region coverage** (primary), **line coverage** (secondary), and **function coverage** across all instrumented code. Test counts represent `#[test]` function markers in source, verified by file scanning on 2026-02-27.

---

## 1. P0: Core Engine (12 crates)

**Target**: 85% line coverage | **Weighted Actual**: 55.4% (137,617 instrumented lines)

| Crate | Region Cov | Line Cov | Func Cov | Instr. Lines | Tests (Lib+Integ) | Status |
|-------|-----------|----------|----------|-------------|-------------------|--------|
| astraweave-ecs | 95.16% | **96.39%** | 97.53% | 4,769 | 728 (331+397) | EXCEEDS |
| astraweave-physics | 94.53% | **94.38%** | 99.65% | 18,324 | 1,884 (1,284+600) | EXCEEDS |
| astraweave-math | 94.01% | **91.12%** | 100.00% | 1,329 | 176 (109+67) | EXCEEDS |
| astraweave-nav | 92.88% | **93.11%** | 99.34% | 3,136 | 496 (216+280) | EXCEEDS |
| astraweave-behavior | 87.51% | **85.81%** | 86.44% | 3,847 | 458 (233+225) | MEETS |
| astraweave-core | 83.29% | **80.98%** | 78.29% | 9,390 | 959 (505+454) | BELOW |
| astraweave-ai | 65.26% | **67.38%** | 54.21% | 7,768 | 921 (514+407) | BELOW |
| astraweave-scene | 55.09% | **55.42%** | 64.55% | 4,890 | 405 (213+192) | BELOW |
| astraweave-gameplay | 48.56% | **50.46%** | 42.46% | 16,641 | 687 (471+216) | BELOW |
| astraweave-render | 46.01% | **48.04%** | 43.96% | 37,035 | 4,272 (851+3,421) | BELOW |
| astraweave-terrain | 46.22% | **44.11%** | 42.16% | 18,826 | 2,107 (543+1,564) | BELOW |
| astraweave-audio | 25.70% | **24.34%** | 21.77% | 11,662 | 531 (240+291) | BELOW |

### Crate Analysis

**astraweave-ecs** (96.39% line, 728 tests): Industry-leading for an ECS implementation. BlobVec, SparseSet, EntityAllocator, Archetype, SystemParam all thoroughly tested. 331 lib tests + 397 integration tests. Miri-validated (379 tests passed, 7 ignored), Kani-verified (27 proofs). 4,769 instrumented lines.

**astraweave-physics** (94.38% line, 1,884 tests): Comprehensive coverage of character controllers, vehicles, ragdolls, cloth simulation, spatial hash. 18,324 instrumented lines with near-complete function coverage (99.65%). Largest well-covered P0 crate. 1,284 lib + 600 integration tests.

**astraweave-math** (91.12% line, 176 tests): SIMD vector/matrix/quaternion operations. 100% function coverage. Miri-validated (109 tests, all passed). Kani proofs for dot product symmetry, cross product anticommutativity, normalization. Small footprint (1,329 instrumented lines). 109 lib + 67 integration tests.

**astraweave-nav** (93.11% line, 496 tests): A* pathfinding, navmesh baking, obstacle avoidance. 99.34% function coverage. 15 winding bugs and 3 topology issues fixed during earlier coverage sprint. 216 lib + 280 integration tests.

**astraweave-behavior** (85.81% line, 458 tests): Behavior trees, GOAP planning, utility evaluation. Meets the 85% target. goap.rs well-covered, ecs.rs strong. 233 lib + 225 integration tests.

**astraweave-core** (80.98% line, 959 tests): 4 pp below the 85% target. 9,390 instrumented lines — grew significantly with ECS bridge, perception, capture/replay, tool sandbox modules. Core modules (ecs_bridge, perception, sim, lib) at near-100%; validation.rs (~82%) and tool_vocabulary remain hotspots. 505 lib + 454 integration tests. Miri-validated (449 passed, 13 ignored). A focused sprint could close the gap.

**astraweave-ai** (67.38% line, 921 tests): Coverage measured when crate had 7,768 instrumented lines. Crate has since grown substantially (~19,500 source lines). 514 lib + 407 integration tests (up from 268 total at time of coverage measurement). Many new orchestrator/plugin modules added. Coverage would likely be lower if re-measured today. Priority improvement target.

**astraweave-scene** (55.42% line, 405 tests): World partitioning, streaming, GPU resource management. gpu_resource_manager and streaming modules have significant untested code paths. 213 lib + 192 integration tests.

**astraweave-gameplay** (50.46% line, 687 tests): Combat physics, damage systems, AI-combat pipeline. 16,641 instrumented lines (inflated by dependency inlining; ~10,800 actual source lines). Many new gameplay systems added without proportional test coverage. 471 lib + 216 integration tests.

**astraweave-render** (48.04% line, 4,272 tests): Largest crate at 37,035 instrumented lines (~38,200 source lines). Contains substantial GPU-only code untestable in headless mode (shaders, framebuffer, pipeline creation). High-coverage files: clustered.rs (97.66%), vertex_compression.rs (96.23%), lod_generator.rs (~95%). Low-coverage: renderer.rs (~1%), ibl.rs (13%), skybox.rs (0%). **Test suite expanded dramatically** since coverage measurement: 851 lib + 3,421 integration tests (including Wave 3 mutation remediation tests). Coverage would likely improve significantly if re-measured. Industry comparison: Unity rendering 25–35%, Bevy 45–50%.

**astraweave-terrain** (44.11% line, 2,107 tests): 18,826 instrumented lines include large generated/table modules (marching cubes tables, voxel mesh). 543 lib + 1,564 integration tests. Integration tests run via `--tests` and are not captured by `--lib` coverage measurement — effective coverage is likely significantly higher.

**astraweave-audio** (24.34% line, 531 tests): 11,662 instrumented lines include large dependency footprint (actual source ~4,200 lines). Core audio engine requires runtime context (audio devices, mixers) unavailable in headless testing. 240 lib + 291 integration tests.

---

## 2. P1: Important Systems (5 crates)

**Target**: 80% line coverage | **Weighted Actual**: 58.9% (25,136 instrumented lines)

| Crate | Region Cov | Line Cov | Func Cov | Instr. Lines | Tests (Lib+Integ) | Status |
|-------|-----------|----------|----------|-------------|-------------------|--------|
| astraweave-cinematics | 99.35% | **99.54%** | 100.00% | 1,531 | 335 (141+194) | EXCEEDS |
| astraweave-pcg | 98.76% | **99.32%** | 100.00% | 587 | 90 (43+47) | EXCEEDS |
| astraweave-materials | 96.40% | **98.55%** | 100.00% | 1,032 | 241 (41+200) | EXCEEDS |
| astraweave-weaving | 95.04% | **94.34%** | 94.06% | 7,294 | 614 (394+220) | EXCEEDS |
| astraweave-ui | 32.02% | **32.62%** | 31.54% | 14,692 | 751 (320+431) | BELOW |

### Crate Analysis

**astraweave-cinematics** (99.54% line, 335 tests): Near-perfect coverage. Timeline creation, track sequencing, playback — 100% function coverage. Compact design (1,531 instrumented lines). 141 lib + 194 integration tests.

**astraweave-pcg** (99.32% line, 90 tests): Procedural content generation. Room overlap, dungeon layout, encounter generation. 100% function coverage. 43 lib + 47 integration tests. High coverage despite small test count reflects compact, well-factored design.

**astraweave-materials** (98.55% line, 241 tests): PBR material system. 100% function coverage. 41 lib + 200 integration tests. Integration-heavy testing strategy validates the cross-crate material pipeline.

**astraweave-weaving** (94.34% line, 614 tests): Pattern classification, adjudication, damage resolution. Strong coverage across all modules. patterns.rs near-100%, adjudicator.rs 98%+. 394 lib + 220 integration tests.

**astraweave-ui** (32.62% line, 751 tests): 14,692 instrumented lines — the largest P1 crate (~8,900 actual source lines; dependency inflation ~65%). Many UI components require egui Context for rendering, unavailable in headless tests. Menu systems well-tested (menus.rs ~92%); layer.rs and widget code largely untestable without GPU context. 320 lib + 431 integration tests.

---

## 3. P2: Support & Infrastructure (6 crates)

**Target**: 75% line coverage | **Weighted Actual**: 73.9% (38,419 instrumented lines)

| Crate | Region Cov | Line Cov | Func Cov | Instr. Lines | Tests (Lib+Integ) | Status |
|-------|-----------|----------|----------|-------------|-------------------|--------|
| astraweave-embeddings | 98.10% | **97.84%** | 98.10% | 1,663 | 198 (104+94) | EXCEEDS |
| astraweave-input | 96.86% | **95.62%** | 98.98% | 2,215 | 303 (178+125) | EXCEEDS |
| astraweave-memory | 94.29% | **94.47%** | 93.16% | 6,929 | 603 (342+261) | EXCEEDS |
| astraweave-security | 84.36% | **83.21%** | 83.08% | 3,657 | 419 (188+231) | EXCEEDS |
| astraweave-llm | 78.62% | **79.47%** | 74.95% | 16,776 | 0* | MEETS |
| astraweave-scripting | 26.51% | **23.75%** | 19.61% | 7,179 | 128 (45+83) | BELOW |

*LLM crate excluded from standard builds; tests require external API keys. Coverage from prior instrumented runs.

### Crate Analysis

**astraweave-embeddings** (97.84% line, 198 tests): Vector embedding operations, similarity search. Near-complete coverage with 98.10% function coverage. 104 lib + 94 integration tests.

**astraweave-input** (95.62% line, 303 tests): Input bindings, action mapping, manager state. bindings.rs 100%, manager.rs 92%+. 178 lib + 125 integration tests. Exceeds target significantly.

**astraweave-memory** (94.47% line, 603 tests): Short/long-term memory systems, retrieval, importance decay. 6,929 instrumented lines with excellent function coverage (93.16%). 342 lib + 261 integration tests.

**astraweave-security** (83.21% line, 419 tests): RBAC, anti-cheat, injection detection, operation limiting. 188 lib + 231 integration tests provide solid coverage above target.

**astraweave-llm** (79.47% line, 16,776 instrumented lines): LLM integration requires external API dependencies. Coverage measured from prior instrumented run. Substantial async infrastructure.

**astraweave-scripting** (23.75% line, 128 tests): 7,179 instrumented lines include ~5,400 lines of Rhai external FFI bindings (actual source ~1,750 lines). Module-level analysis: api.rs ~76%, lib.rs ~90%, loader.rs ~97%. Effective source-only coverage estimated at ~70–80%. 45 lib + 83 integration tests.

---

## 4. Additional Measured Crates

These crates are outside the primary tier system but have been measured:

| Crate | Region Cov | Line Cov | Instr. Lines | Tests (Lib+Integ) | Notes |
|-------|-----------|----------|-------------|-------------------|-------|
| astraweave-prompts | 95.46% | **95.66%** | 5,968 | 1,375 (556+819) | Wave 2 mutation: 792 mutants, 100% kill rate |
| astraweave-fluids | 91.57% | **89.27%** | 46,173 | 2,509 (2,410+99) | Largest crate; SPH/FLIP simulation |
| astraweave-context | 30.74% | **25.71%** | 9,813 | 228 (101+127) | Large dependency footprint inflates denominator |
| astraweave-persistence-ecs | 28.96% | **28.29%** | 3,287 | 132 (28+104) | Roundtrip serialization verified; async gaps |
| astraweave-rag | 20.78% | **19.85%** | 9,957 | 235 (76+159) | Large dependency footprint; consolidation.rs high |

### Notable

**astraweave-prompts** (95.66%, 1,375 tests): Template engine, prompt library, optimization. 556 lib + 819 integration tests. Wave 2 mutation testing: 792 mutants with 100% kill rate on all killable mutants. Best-validated crate in the workspace.

**astraweave-fluids** (89.27%, 2,509 tests): SPH fluid simulation, spatial hashing, GPU buffers. Largest crate by instrumented lines (46,173; ~67,800 actual source lines). 2,410 lib + 99 integration tests. Excellent coverage given domain complexity. **Wave 4 mutation testing**: 411 viable mutations, 405 caught, 6 equivalent, **100% adjusted kill rate**. See `docs/current/FLUIDS_MUTATION_TESTING_REPORT.md`.

---

## 5. Unmeasured Crates

The following crates were not measured for coverage in this audit (build issues, external dependencies, or lower priority). Test counts verified via `#[test]` marker scanning on 2026-02-27.

| Crate | Tests (Lib+Integ) | Category | Notes |
|-------|-------------------|----------|-------|
| aw_editor | 8,979 (3,677+5,302) | Editor | Largest test suite in workspace; build depends on astraweave-terrain |
| astraweave-asset | 431 (240+191) | Assets | Asset loading/management |
| veilweaver_slice_runtime | 324 (324+0) | Runtime | Slice runtime system |
| astraweave-dialogue | 291 (168+123) | AI Support | Dialogue tree system |
| astraweave-net | 255 (41+214) | Networking | Network transport layer |
| astraweave-persona | 244 (108+136) | AI Support | AI persona management |
| astraweave-quests | 218 (28+190) | Game Features | Quest system |
| astraweave-director | 180 (90+90) | AI Support | AI director system |
| astraweave-npc | 108 (66+42) | Game Features | NPC behavior |
| astraweave-observability | 105 (64+41) | Observability | Metrics and tracing |
| astraweave-coordination | 94 (4+90) | Infrastructure | Multi-agent coordination |
| astraweave-optimization | 60 (2+58) | AI Support | Optimization algorithms |
| astraweave-asset-pipeline | 61 (14+47) | Assets | Asset pipeline processing |
| astraweave-ipc | 57 (8+49) | Networking | Inter-process communication |
| astraweave-secrets | 54 (19+35) | Security | Secrets management |
| astraweave-llm-eval | 43 (0+43) | AI Support | LLM evaluation framework |
| astraweave-profiling | 41 (7+34) | Observability | Performance profiling |
| astraweave-net-ecs | 35 (8+27) | Networking | ECS-integrated networking |
| astraweave-stress-test | 34 (1+33) | Testing | Stress test infrastructure |
| astraweave-steam | 30 (10+20) | Platform | Steam integration |
| Build/CLI tools | ~9 | Tooling | aw_build, aw_asset_cli, aw_release, aw_demo_builder |
| Other | 0 | Various | astraweave-author (Rhai Sync errors), astraweave-assets, astraweave-blend, net clients/servers |
| **Unmeasured Total** | **~3,433** | | |

**Total workspace tests**: ~39,000+ `#[test]` functions across 128 crates (measured + unmeasured + demo/example crates).

---

## 6. Validation Infrastructure

### 6.1 Miri Memory Safety

**Scope**: All 4 crates containing `unsafe` code
**Status**: Clean — zero undefined behavior detected
**Tool**: `cargo +nightly miri test` (rustc 1.86.0-nightly, Miri 0.1.0-nightly)
**Flags**: `-Zmiri-disable-isolation` (Windows compatibility)

| Crate | Tests | Passed | Ignored | UB |
|-------|-------|--------|---------|-----|
| astraweave-core | 465 | 449 | 13 | None |
| astraweave-ecs | 386 | 379 | 7 | None |
| astraweave-math | 109 | 109 | 0 | None |
| astraweave-sdk | 17 | 17 | 0 | None |
| **Total** | **977** | **954** | **20** | **None** |

3 test failures in core are pre-existing assertion bugs (incorrect test expectations), not memory safety issues. They also fail in regular `cargo test`.

**Validated unsafe code**: BlobVec (type-erased storage), SparseSet (O(1) lookup), EntityAllocator (generational IDs), SystemParam (raw pointer iteration), Archetype (component storage), SIMD operations (SSE2 vector/matrix/quaternion), C ABI FFI (buffer overflow protection, null pointer handling, UTF-8 validity).

Full details: [MIRI_VALIDATION_REPORT.md](MIRI_VALIDATION_REPORT.md)

### 6.2 Kani Formal Verification

**Scope**: 4 crates, ~71 proof harnesses
**Status**: Clean — all proofs verified
**CI**: Weekly (Sunday 3:00 AM UTC) via `model-checking/kani-github-action@v1`

| Crate | Proofs | Properties Verified |
|-------|--------|---------------------|
| astraweave-ecs | 27 | Entity ID uniqueness, generational index roundtrip, BlobVec memory safety, push/get invariants, swap_remove correctness, capacity ≥ len |
| astraweave-core | 22 | IVec2 math symmetry/non-negativity, Manhattan/squared distance, WorldSnapshot defaults, addition commutativity, subtraction inverse |
| astraweave-math | 13 | Dot product symmetry, cross product anticommutativity/orthogonality, normalization unit vectors, length non-negativity |
| astraweave-sdk | 9 | C ABI FFI: buffer overflow protection, null pointer handling, struct layout (6 bytes/2-byte aligned), UTF-8/ASCII validity, semantic version format |
| **Total** | **~71** | |

Kani uses CBMC bounded model checking. Math proofs use concrete representative inputs (Kani does not support SIMD intrinsics).

### 6.3 Mutation Testing

#### Wave 1 — Manual (January 2026)

**Scope**: 7 P0 crates | **Types**: Boundary (`<` vs `≤`), comparison (`==` vs `!=`), boolean inversions

| Crate | Mutation Tests | Total Tests (at time) | Status |
|-------|---------------|-----------------------|--------|
| astraweave-core | 80 | 465 | All passing |
| astraweave-ecs | 114 | 300 | All passing |
| astraweave-physics | 80 | 560 | All passing |
| astraweave-ai | 113 | 244 | All passing |
| astraweave-render | 152 | 521 | All passing |
| astraweave-terrain | 140 | 470 | All passing |
| astraweave-prompts | 88 | 480 | All passing |
| **Total** | **767** | **3,040** | **All passing** |

#### Wave 2 — Automated (February 2026)

**Tool**: `cargo-mutants` v26.2.0 + `cargo-nextest` v0.9.128
**Method**: Full automated sweep with `--in-place`. Each mutant is a compiler-valid source change (operator replacement, return value substitution, statement deletion). "Caught" = any test fails; "Missed" = all tests pass.

| Crate | Mutants | Caught | Missed | Equiv | Unviable | Kill Rate |
|-------|---------|--------|--------|-------|----------|-----------|
| astraweave-prompts | 792 | 760 | 0 | 2 | 30 | **100%** |
| astraweave-render | 339 | 238 | 75 | — | 25 | 76.1% (69 GPU-only) |
| aw_editor (sampled) | 130 | 52 | 10 | — | 24 | 83.0% |

**Prompts details** (792 mutants across 4 shards):
- 760 caught, 2 equivalent (unkillable), 30 unviable
- 2 equivalent mutants: `library.rs:367 save_to_directory` (no-op stub), `terrain_prompts.rs:173 required_variables` (default fill)
- 47 MISSED pre-remediation → all remediated via 5 targeted commits → **100% post-remediation kill rate**

**Render details** (339 mutants): 238 caught, 75 missed (69 GPU-only: shaders, framebuffer, pipeline code untestable in headless mode). 6 non-GPU MISSED remediated. Effective non-GPU kill rate: ~97%.

**Editor details** (25,573 total mutants, strategic sampling of 130):
- entity_manager.rs: 100% viable kill rate
- command.rs: 1 missed → remediated
- plugin.rs: 6 missed → all remediated
- 2 permanent GUI-only MISSED: `dock_layout.rs show` and `show_inside` (require egui Context)

**Remediation commits (Wave 2)**: `3a54a591`, `0c834270`, `8c10a46b`, `0581aeec`, `ffc605bc`, `47669d30`, `167666e8`.

#### Wave 3 — Targeted Remediation (February 2026)

**Method**: Targeted `cargo-mutants` runs with `--re` function filters and `--lib` limitation. Focused on specific functions identified as mutation-vulnerable in earlier sweeps.

| Run | Target Functions | Caught | Missed | Unviable | Timeout | Kill Rate |
|-----|-----------------|--------|--------|----------|---------|-----------|
| terrain_targeted_v3 | climate, heightmap, smooth | 241 | 13 | 1 | 1 | **94.9%** |
| terrain_targeted_v4 | sample_bilinear, fbm, smooth | 140 | 4 | 0 | 0 | **97.2%** |
| render_targeted_v2 | Camera::dir, sun, weather, easing | 77 | 5 | 0 | 0 | **93.9%** |
| render_cam_biome | camera.rs, biome_transition.rs | 5 | 0 | 2 | 0 | **100%** |
| **Wave 3 Total** | | **463** | **22** | **3** | **1** | **95.5%** |

**Terrain remaining MISSED** (4 in latest v4 run):
- `heightmap.rs:152–153`: `replace - with +` and `replace - with /` in `Heightmap::sample_bilinear` — bilinear interpolation weight calculations require exact-value regression tests
- All 4 are arithmetic operator mutations in interpolation weight computation

**Render remaining MISSED** (5 in v2 run):
- `environment.rs:61`: `replace - with +` in `get_sun_position` — azimuth sign inversion
- `environment.rs:67`: `replace < with ==` and `replace < with <=` in `get_sun_position` — boundary condition in sun hemisphere check
- `renderer.rs:3292`: `replace set_weather with ()` — requires GPU context for full validation
- `biome_transition.rs:96`: `replace < with <=` in `EasingFunction::apply` — boundary condition at t=1.0

**Wave 3 remediation commits**: `cd6475df`, `9db0d3be`, `c7d573fb`, `bfb94e46`, `57a14600`.

#### Cumulative Mutation Testing Summary

| Wave | Mutants Tested | Caught | Kill Rate | Period |
|------|---------------|--------|-----------|--------|
| Wave 1 (Manual) | 767 | 767 | 100% | January 2026 |
| Wave 2 (Automated) | 1,261 | 1,050 | 83.3%* | February 2026 |
| Wave 3 (Targeted) | 489 | 463 | 94.7% | February 2026 |
| Wave 4 (Fluids) | 411 | 405 | 98.5%† | March 2026 |
| **Total** | **2,928** | **2,685** | **91.7%** | Jan–Mar 2026 |

*Kill rates in cumulative table = caught ÷ total submitted. Individual run kill rates in detail tables = caught ÷ (caught + missed). Wave 2 includes GPU-untestable mutants; excluding GPU-only code: ~95%+.

†Fluids Wave 4: 411 viable mutations, 405 caught, 6 equivalent (5 boundary.rs + 1 caustics.rs), 0 non-equivalent missed. 45 GPU-dependent mutations excluded (lib.rs). **100% adjusted kill rate**. Full details: `docs/current/FLUIDS_MUTATION_TESTING_REPORT.md`.

---

## 7. Test Quality

### Test Counts by Crate (Audited 2026-02-27 via `#[test]` marker scanning)

| Crate | Lib Tests | Integration Tests | Total |
|-------|----------|-------------------|-------|
| aw_editor | 3,677 | 5,302 | **8,979** |
| astraweave-render | 851 | 3,421 | **4,272** |
| astraweave-fluids | 2,410 | 99 | **2,509** |
| astraweave-terrain | 543 | 1,564 | **2,107** |
| astraweave-physics | 1,284 | 600 | **1,884** |
| astraweave-prompts | 556 | 819 | **1,375** |
| astraweave-core | 505 | 454 | **959** |
| astraweave-ai | 514 | 407 | **921** |
| astraweave-ui | 320 | 431 | **751** |
| astraweave-ecs | 331 | 397 | **728** |
| astraweave-gameplay | 471 | 216 | **687** |
| astraweave-weaving | 394 | 220 | **614** |
| astraweave-memory | 342 | 261 | **603** |
| astraweave-audio | 240 | 291 | **531** |
| astraweave-nav | 216 | 280 | **496** |
| astraweave-behavior | 233 | 225 | **458** |
| astraweave-security | 188 | 231 | **419** |
| astraweave-scene | 213 | 192 | **405** |
| astraweave-cinematics | 141 | 194 | **335** |
| astraweave-input | 178 | 125 | **303** |
| astraweave-materials | 41 | 200 | **241** |
| astraweave-rag | 76 | 159 | **235** |
| astraweave-context | 101 | 127 | **228** |
| astraweave-embeddings | 104 | 94 | **198** |
| astraweave-math | 109 | 67 | **176** |
| astraweave-persistence-ecs | 28 | 104 | **132** |
| astraweave-scripting | 45 | 83 | **128** |
| astraweave-pcg | 43 | 47 | **90** |
| astraweave-sdk | 17 | 53 | **70** |
| **Measured + Editor Total** | **~13,350** | **~16,613** | **~29,855** |

Additional unmeasured crates (persona, dialogue, quests, director, npc, asset, net, coordination, observability, profiling, etc.) contribute ~3,400+ tests. Demo and example crates contribute ~5,700+ more.

**Total workspace**: ~39,000 `#[test]` functions across 128 crates.

### Test Count Methodology Note

Test counts in this revision (v5.1.0) were obtained by scanning source files for `#[test]` markers on 2026-02-27. This differs from the v5.0.0 methodology which used `cargo test --list` output. Key differences:

- `#[test]` scanning reflects current source state and is reproducible independently of build status
- `cargo test --list` may report differently due to conditional compilation, `#[ignore]` attributes, and feature flags
- Some v5.0.0 integration test counts were significantly inflated (e.g., fluids reported 2,503 integration tests; actual `#[test]` markers in `tests/` directory: 99)
- Test counts have grown significantly since Feb 25 due to Wave 3 mutation remediation commits

### Production Code Safety

All `.unwrap()` calls in engine runtime are confined to `#[cfg(test)]` modules. Production paths use `anyhow::Context` and `?` propagation exclusively. Build/CLI tools (`aw_build`, `aw_demo_builder`) have a handful of low-risk `.unwrap()` in non-runtime paths.

### Integration Testing Highlights

- **5,302** integration tests for aw_editor (largest integration test suite)
- **3,421** integration tests for astraweave-render (including Wave 3 mutation remediation)
- **1,564** integration tests for astraweave-terrain
- **819** integration tests for astraweave-prompts
- Combat physics: AI → Combat → Physics → Damage pipeline validated
- Determinism: 100-frame replay, seed variation, component updates
- Performance SLA: 1,000-entity at 60 FPS, AI latency, memory stability

---

## 8. Industry Comparison

| Tier | Coverage Range | Description | AstraWeave Crates |
|------|---------------|-------------|-------------------|
| Minimal | 0–30% | Untested or prototype | audio (24%), scripting (24%), rag (20%), context (26%) |
| Basic | 30–50% | Some testing | render (48%), gameplay (50%), terrain (44%), ui (33%), persistence-ecs (28%) |
| Good | 50–70% | Reasonable coverage | ai (67%), scene (55%) |
| **Standard** | **70–85%** | **Mature projects** | core (81%), llm (79%), security (83%), behavior (86%) |
| Excellent | 85–95% | High quality | fluids (89%), ecs (96%), physics (94%), math (91%), nav (93%), memory (94%), input (96%), weaving (94%), prompts (96%) |
| Outstanding | 95–100% | Mission-critical | cinematics (99.5%), pcg (99.3%), materials (98.6%), embeddings (97.8%) |

**14 crates at ≥85% line coverage** (50% of 28 measured). **3 crates at ≥98%** (cinematics, pcg, materials). 5 crates below 30% (mostly due to dependency inflation or GPU-only code).

---

## 9. Methodology & Limitations

### Measurement Approach

```powershell
cargo llvm-cov --no-cfg-coverage --lib -p <crate> --summary-only
```

- **`--lib`**: Runs only unit tests defined in `src/`. Integration tests in `tests/` are not included in coverage measurement.
- **`--no-cfg-coverage`**: Prevents `cfg(coverage)` from being set, ensuring normal compilation paths.
- **Region, function, and line coverage** are all reported. Line coverage is the primary metric.
- **Branch coverage** is not instrumented (`-` in all measurements).

### Known Measurement Caveats

1. **Dependency inflation**: LLVM instruments all compiled code, including inlined generics from workspace dependencies. This inflates the instrumented line count for crates that depend on other workspace crates. Most affected: audio (11,662 instrumented vs ~4,200 source), rag (9,957 vs ~3,200), context (9,813 vs ~3,800), scripting (7,179 vs ~1,750), persistence-ecs (3,287 vs ~1,160).
2. **Integration tests excluded**: `--lib` only measures code exercised by `#[test]` functions in `src/`. Crates with extensive `tests/` directories (render: 3,421 integration tests, terrain: 1,564, prompts: 819) have higher effective coverage than the lib-only numbers suggest.
3. **GPU code untestable**: Rendering, terrain GPU meshing, and audio device code requires runtime GPU context unavailable during `cargo test`. This is a fundamental limitation shared by all game engines.
4. **LLM crate**: Excluded from standard builds. Coverage from prior instrumented runs.
5. **Measurement staleness**: Coverage percentages were measured on 2026-02-25. Significant test additions occurred between Feb 25–27 (Wave 3 mutation remediation commits). Re-measurement recommended to capture current state.
6. **astraweave-terrain**: `cargo llvm-cov` initially failed during the v5.0 audit (dependency build error). Coverage of 46.22% is from a successful retry within the same session.

### Report Version Comparison

| Metric | v4.2.0 (Feb 24) | v5.0.0 (Feb 25) | v5.1.0 (Feb 27) |
|--------|-----------------|-----------------|-----------------|
| Weighted coverage | 94.57% | 59.3% | 59.3%* |
| Crates measured | 25 | 28 | 28 + editor |
| Workspace tests | 15,000+ | ~18,200+ | ~39,000+ |
| Mutation mutants | 767 | 2,028+ | 2,517 |
| Methodology | Manual file analysis | `llvm-cov` | `llvm-cov` + `#[test]` scan |

*Coverage not re-measured in v5.1.0; same llvm-cov data from v5.0.0. Test counts re-audited via source scanning.

The v5.0.0+ methodology is more rigorous and reproducible. The trade-off is that including dependency code makes raw numbers lower for some crates compared to source-file-only analysis.

---

## 10. Key Findings & Recommendations

### Strengths

1. **14 crates at 85%+ line coverage** — half of all measured crates exceed industry standards
2. **Miri + Kani dual verification** on all unsafe code — zero UB, all proofs passing
3. **Mutation testing** with 100% kill rate on prompts crate (792 mutants) — gold standard
4. **~39,000+ tests** workspace-wide across 128 crates — extensive test infrastructure
5. **3 crates at 98%+** (cinematics 99.5%, pcg 99.3%, materials 98.6%) — mission-critical quality
6. **Zero `.unwrap()` in production code** — disciplined error handling
7. **Wave 3 mutation testing** achieved 95.5% kill rate on targeted terrain/render functions
8. **aw_editor has 8,979 tests** — largest single-crate test suite in the workspace

### Priority Improvement Targets

| Priority | Crate | Current Line Cov | Target | Effort | Impact |
|----------|-------|-----------------|--------|--------|--------|
| P0 | astraweave-ai | 67.38%† | 85% | Medium | High — core AI loop |
| P0 | astraweave-core | 80.98% | 85% | Low | High — 4 pp from target |
| P0 | astraweave-gameplay | 50.46% | 70% | High | Medium — large crate |
| P0 | astraweave-audio | 24.34% | 50% | High | Medium — dependency inflation |
| P1 | astraweave-ui | 32.62% | 60% | High | Low — requires egui mocking |
| P2 | astraweave-scripting | 23.75% | 50% | Medium | Low — Rhai FFI dominates |

†AI coverage likely lower if re-measured due to substantial crate growth since Feb 25.

### Methodology Recommendations

1. **Re-measure coverage**: Coverage was last measured 2026-02-25. Given significant test additions (Wave 3 mutation remediation), a full re-measurement via `cargo llvm-cov` is recommended to capture the improvement in render, terrain, and editor crates.
2. **Run integration tests under llvm-cov**: Use `cargo llvm-cov --tests -p <crate>` to capture coverage from `tests/` directories. This would significantly improve reported numbers for render (3,421 integration tests), terrain (1,564), and prompts (819).
3. **Source-only filtering**: Use `cargo llvm-cov --html` and inspect per-file coverage to separate true source coverage from dependency inflation.
4. **Exclude GPU code from targets**: Set separate coverage targets for headless-testable vs GPU-only code paths.
5. **Continue Wave 3 mutation testing**: 4 remaining terrain MISSED (sample_bilinear arithmetic) and 5 render MISSED (sun position, easing boundary) are concrete remediation targets.
6. **Track coverage delta per-commit**: Integrate `cargo llvm-cov` into CI to prevent coverage regression on high-coverage crates.

---

## 11. Execution Commands

```powershell
# Single crate measurement (recommended)
cargo llvm-cov --no-cfg-coverage --lib -p astraweave-ecs --summary-only

# HTML report with file-level breakdown
cargo llvm-cov --no-cfg-coverage --lib -p astraweave-ecs --html --output-dir coverage/ecs

# Integration test coverage (captures tests/ directory)
cargo llvm-cov --no-cfg-coverage --tests -p astraweave-prompts --summary-only

# Multiple crates
cargo llvm-cov --no-cfg-coverage --lib -p astraweave-ecs -p astraweave-core --summary-only

# Miri validation (requires nightly)
cargo +nightly miri test -p astraweave-ecs --lib -- --test-threads=1

# Kani verification
cargo kani --package astraweave-ecs

# Mutation testing
cargo mutants --package astraweave-prompts --test-tool nextest --in-place --timeout-multiplier 3 --json
```

---

## 12. Revision History

| Version | Date | Type | Summary | Impact |
|:-------:|:----:|:----:|:--------|:------:|
| **5.3.0** | 2026-06-10 | **Update** | Engine-health-audit reconciliation: workspace crate count 128 → 130 (`astraweave-alloc` 2026-04-17, `astraweave-camera` 2026-05-18 — both unmeasured); recorded post-v5.2.0 test growth (Net-Trio-Remediation 104 trio tests: proto 59 / server 41 / client 4; Multi-Tool SP4 dispatcher tests 15 → 21; Unified Camera 25 crate tests + 8 contract tests + parity fixtures); live marker re-count 39,973. Coverage percentages NOT re-measured (still 2026-02-25) — gap caveat added. | Incremental |
| 5.2.0 | 2026-03-01 | Update | Added Wave 4 fluids mutation testing (411 mutants, 100% adjusted kill rate). Updated cumulative mutation totals (2,928 mutants, 91.7% raw). Created `FLUIDS_MUTATION_TESTING_REPORT.md`. | Significant |
| 5.1.0 | 2026-02-27 | Audit | Full audit: corrected test counts via `#[test]` marker scanning (39,000+ workspace total). Added Wave 3 mutation testing results (463 mutants, 95.5% kill rate). Fixed "4 crates at ≥98%" → 3. Corrected fluids integration test count (4,907→2,509). Added detailed unmeasured crate inventory with verified test counts. Updated methodology comparison table. | Significant |
| 5.0.0 | 2026-02-25 | Major audit | Full re-measurement of 28 crates via `cargo llvm-cov`. Corrected stale coverage data. Reformatted to match benchmark report structure. Added hardware, methodology, and limitations sections. Grade recalibrated to B+. All numbers reproducible. | Critical |
| 4.2.0 | 2026-02-24 | Update | Wave 2 mutation testing: prompts 100% kill (792 mutants), render 97% non-GPU | Significant |
| 4.1.0 | 2026-02-24 | Update | Added cargo-mutants automated sweep data | Incremental |
| 4.0.0 | 2026-02-10 | Major | Full audit: removed duplicates, consolidated tiers, eliminated contradictions | Significant |
| 3.2.0 | 2026-02-03 | Update | Miri validation: 977 tests, 4 crates, zero UB | Significant |
| 3.1.0 | 2026-01-31 | Update | Manual mutation testing: 767 tests across 7 P0 crates | Significant |
| 3.0.0 | 2026-01-20 | Major | Bulletproof validation (Phases 1–9) | Significant |
| 2.0.0 | 2025-12-06 | Major | Full workspace audit: 47 crates measured | Significant |
| 1.0 | 2025-10-21 | Major | Initial report consolidating 40+ documents | Significant |

---

**Next Review**: March 2026 (monthly cadence)
