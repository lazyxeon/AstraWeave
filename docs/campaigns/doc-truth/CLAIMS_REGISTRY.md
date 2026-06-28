# CLAIMS_REGISTRY.md — Single Source of Truth for Load-Bearing Numbers

**Documentation Truth campaign keystone.** Every load-bearing, recurring metric in AstraWeave's prose has exactly one home: a row here. All other documents link to that row instead of restating the value (`<!-- Source: CLAIMS_REGISTRY.md#slug -->` or a markdown link to the anchor). The registry is the only file where such a number may appear without a link.

**Scope (deliberately narrow).** A metric earns a row iff it was flagged `📒 REGISTRY-CANDIDATE` in the D.0 / D.0.1 inventories **and** it either appears in ≥2 documents **or** is on the poison list. One-off numbers in a single dated report stay inline — multi-doc restatement is the drift disease the registry cures.

**Built in two passes (by design).**
- **D.1.A (this pass)** lays the skeleton, fills every row whose value is established at HEAD by D.0 §1.2 ground truth (`status: VERIFIED-AT-HEAD`), and marks every benchmark/coverage/contested row `value: PENDING-D2` with its exact reproduction command.
- **D.2** runs each `repro` and fills the `PENDING-D2` rows in place. Until then, a `PENDING-D2` row is a *promise of measurement*, not a value — documents may link to it, but D.1 does not invent its number.

**Retired, never homed.** Fabricated numbers are deleted, not registered. The `103,500` / `103k` entity-capacity figure (superseded by [agents-capacity-60fps](#agents-capacity-60fps)) and the competitor multipliers (`10.4× Unity`, `2.1-5.2× Unreal`, the invented Unity-9,900 / Unreal-20k-50k baselines) are excised across the corpus and have no registry row. See §Retired below.

Status vocabulary: **VERIFIED-AT-HEAD** (re-checked this campaign at HEAD via the canonical_source) · **PENDING-D2** (value not yet established; repro pending) · **CONTESTED-PENDING-D2** (documents disagree, no ground-truth arbiter yet).

---

## Counts (VERIFIED-AT-HEAD)

### workspace-members
- metric: Workspace member count
- value: 130
- status: VERIFIED-AT-HEAD (2026-06-13, `cargo metadata --no-deps`)
- repro: `cargo metadata --no-deps --format-version 1 | jq '.packages | length'`
- hardware: n/a
- canonical_source: `cargo metadata --no-deps`
- referenced_by: README.md:52,129,274,289; CLAUDE.md:7,130; docs/architecture/ARCHITECTURE_MAP.md:36; docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md:237; docs/README.md:21,411; docs/current/IMPLEMENTATION_PLANS_INDEX.md; gh-pages/index.md:18; gh-pages/crates.md:8

### production-crates
- metric: Production-crate count (judgment bucket, not a mechanical count)
- value: ~51 (53 names start `astraweave-`; some production crates lack the prefix — this is a bucket, not a grep)
- status: VERIFIED-AT-HEAD (2026-06-13, `cargo metadata` + prefix grep)
- repro: `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | grep -c '^astraweave-'`
- hardware: n/a
- canonical_source: `cargo metadata --no-deps` + workspace `Cargo.toml` members
- referenced_by: README.md:52,289,376; CLAUDE.md:7; docs/README.md:21,411; docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md:237

### test-markers-total
- metric: Workspace `#[test]` + `#[tokio::test]` annotation count (source-text markers, NOT a compile-and-run count)
- value: ~39,900 (38,803 `#[test]` + 1,102 `#[tokio::test]`)
- status: VERIFIED-AT-HEAD (2026-06-13, `git grep -c`)
- repro: `git grep -E -c '#\[(tokio::)?test\]' -- '*.rs' | awk -F: '{s+=$NF} END {print s}'`
- hardware: n/a
- canonical_source: `git grep` over tracked `.rs`
- referenced_by: README.md:52,58,274; CLAUDE.md; docs/masters/MASTER_ROADMAP.md:38,244; gh-pages/index.md:20; gh-pages/crates.md:8

### editor-test-markers
- metric: `tools/aw_editor` `#[test]` + `#[tokio::test]` annotation count
- value: 9,427 (README/CLAUDE cite 9,425 — within pattern/pathspec tolerance)
- status: VERIFIED-AT-HEAD (2026-06-13, `git grep -c` scoped to `tools/aw_editor`)
- repro: `git grep -E -c '#\[(tokio::)?test\]' -- 'tools/aw_editor' | awk -F: '{s+=$NF} END {print s}'`
- hardware: n/a
- canonical_source: `git grep` scoped to `tools/aw_editor`
- referenced_by: README.md:58,109,261; CLAUDE.md:309; docs/architecture/aw_editor.md:650,822; docs/architecture/ARCHITECTURE_MAP.md:822
- note: SUPERSEDES the poison values `3,892` and `6,100`. NOT to be confused with the *contested* editor sub-counts (71 / 429 / 1,681 / 3,970 / 4,010 / 4,103 inline vs 5,322 tests/) which remain CONTESTED-PENDING-D2 and are NOT homed here.

### fluids-test-markers
- metric: `astraweave-fluids` `#[test]` + `#[tokio::test]` annotation count
- value: 738
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1 re-resolution post-W.1; `git grep -c` scoped to `astraweave-fluids`)
- repro: `git grep -E -c '#\[(tokio::)?test\]' -- 'astraweave-fluids' | awk -F: '{s+=$NF} END {print s}'`
- hardware: n/a
- canonical_source: `git grep` scoped to `astraweave-fluids`
- referenced_by: README.md:265; docs/masters/MASTER_ROADMAP.md:86,91,102,474; docs/current/MASTER_COVERAGE_REPORT.md:187; gh-pages/crates.md:43
- note: SUPERSEDES the poison value `4,907` AND the former D.2.A value `2,560` (2,454 src + 106 `tests/`, verified 2026-06-13), which W.1 (2026-06-20) invalidated by deleting the SPH/voxel solver and most test-bearing files. The mildly-stale `2,509` / `2,404` also resolve here.

### fluids-loc
- metric: `astraweave-fluids` source lines
- value: 24,251 (src, 19 files) / 27,257 (whole crate incl. `tests/`)
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1 re-resolution post-W.1; `git ls-files | xargs wc -l`)
- repro: `git ls-files -- 'astraweave-fluids/src/' | grep '\.rs$' | xargs wc -l | tail -1`
- hardware: n/a
- canonical_source: `wc -l` over `astraweave-fluids/src`
- referenced_by: CLAUDE.md (84K — STALE); docs/audits/water_system_architecture_2026-04-20.md:128 (3,810 — STALE)
- note: SUPERSEDES the former D.2.A value `80,222 src / 83,651 crate` (verified 2026-06-13), which W.1 (2026-06-20) invalidated by deleting the SPH/voxel solver + `simd_ops.rs` (−58.8K LoC). Also supersedes the poison `46,173`; the `84K`/`84.5K` figures are now badly stale (true ~24.2K). **REPRO FIXED in D.2.A.1:** the prior `astraweave-fluids/src/**/*.rs` glob returned 0 at HEAD (git `**` pathspec does not match files directly under `src/`).

### water-facade-loc
- metric: `astraweave-water` source lines (gameplay water-truth facade; W.2–F.4 successor to the deleted fluids sim)
- value: 350 (src, `lib.rs`) / 428 (whole crate incl. `benches/`); 9 test-markers
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1; `git ls-files | xargs wc -l`)
- repro: `git ls-files -- 'astraweave-water/src/' | grep '\.rs$' | xargs wc -l | tail -1`
- hardware: n/a
- canonical_source: `wc -l` over `astraweave-water/src`
- referenced_by: docs/architecture/water.md §5; Cargo.toml:44 (workspace member)
- note: The `WaterQuery` trait + `AnalyticWater` backend — CPU-resident, deterministic gameplay TRUTH layer. Kept strictly separate from the render-side presentation ([water-surface-loc](#water-surface-loc)) per the truth-vs-presentation split (water.md Appendix A). NEW in D.2.A.1.

### water-surface-loc
- metric: Render-side water surface LoC (the visible Gerstner/refraction/foam/weave presentation)
- value: 1,373 (`astraweave-render/src/water.rs` 991 + `astraweave-render/src/shaders/water.wgsl` 382)
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1; `git ls-files | xargs wc -l`)
- repro: `git ls-files 'astraweave-render/src/water.rs' 'astraweave-render/src/shaders/water.wgsl' | xargs wc -l | tail -1`
- hardware: n/a
- canonical_source: `wc -l` over the two render water files
- referenced_by: docs/architecture/water.md §1, §5
- note: `WaterRenderer` — surface, screen-space refraction, depth-foam, weave-response presentation. The PRESENTATION half of the water system; the TRUTH half is [water-facade-loc](#water-facade-loc). NEW in D.2.A.1.

### water-system
- metric: Post-W.1 water system architecture (narrative; what replaced the deleted SPH/voxel fluids sim)
- value: Rendering + truth-facade, **NOT simulation** — 5 components: (1) `WaterQuery` facade + `AnalyticWater` backend (CPU, deterministic truth); (2) chunked-LOD Gerstner render surface (`CHUNK_SIZE=64`, 4 Gerstner waves); (3) split water pass — screen-space refraction + depth-delta shoreline foam; (4) weave-response deformation Part/Raise/Freeze (≤`MAX_WEAVE_INSTANCES=8`, bounded ±`SKIRT_DEPTH`); (5) F.4 GPU-particle accent layer (the only retained `astraweave-fluids` surface). `FreezeWater` is presentation-only (no walkable-ice/buoyancy truth).
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1; per `docs/architecture/water.md` §1/§2/§6 @ `7c29b8182`)
- repro: read `docs/architecture/water.md` §1–§2 (component map) + §6 (weave variants)
- hardware: n/a
- canonical_source: `docs/architecture/water.md` (trace v1.1)
- referenced_by: docs/architecture/water.md; docs/architecture/fluids.md §0.5
- note: Supersedes the pre-W.1 "fluids = SPH/voxel simulation" framing across the corpus. The deleted SPH/voxel solver is preserved only at tag `w0-pre-deprecation @ 3a8296038`. NEW in D.2.A.1.

### rust-loc-total
- metric: Total Rust lines across the workspace
- value: ~1.10M raw (1,104,208) / ~854K code-only (853,992; tokei, excludes comments/blanks)
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1 re-resolution post-W.1; tokei + `git ls-files` cross-check)
- repro: `tokei --type Rust .` (code-only) or `git ls-files '*.rs' | xargs cat | wc -l` (raw)
- hardware: n/a
- canonical_source: tokei
- referenced_by: README.md:113 (850 K+); gh-pages/index.md:18 (860,000+)
- note: SUPERSEDES the former D.2.A value `~1.16M raw / ~892K code` (verified 2026-06-13, pre-W.1); W.1 (2026-06-20) removed ~56K raw lines (fluids SPH/voxel solver). The `850K+` / `860K+` figures remain stale lower bounds. **REPRO FIXED in D.2.A.1:** the prior raw repro `git ls-files '*.rs' | xargs wc -l | tail -1` is machine-dependent — on a small-`ARG_MAX` shell `xargs` batches `wc` and `tail -1` returns only the last batch's subtotal (observed 364,809 of 1,104,208); `xargs cat | wc -l` sums correctly.

### kani-proofs
- metric: Kani proof-harness count
- value: 69 (`kani::proof` attributes across 5 `.rs` files)
- status: VERIFIED-AT-HEAD (2026-06-13, `git grep -h kani::proof`)
- repro: `git grep -h 'kani::proof' -- '*.rs' | wc -l`
- hardware: n/a
- canonical_source: `git grep kani::proof`
- referenced_by: README.md:60,277 (71+ — STALE); CLAUDE.md
- note: SUPERSEDES the stale `71+`.

### toolchain
- metric: Pinned Rust toolchain
- value: 1.89.0
- status: VERIFIED-AT-HEAD (2026-06-13, `rust-toolchain.toml`)
- repro: `grep channel rust-toolchain.toml`
- hardware: n/a
- canonical_source: `rust-toolchain.toml`
- referenced_by: README badge; CLAUDE.md; docs/QUICKSTART.md:21 (1.70+ — STALE); docs/src/overview.md (1.73+ — STALE)
- note: SUPERSEDES `1.70+` / `1.73+` / `1.75.0`.

### dependency-versions
- metric: Pinned headline dependency versions
- value: wgpu 25.0.2 · egui 0.32 · glam 0.30 · winit 0.30 · rapier3d 0.22 · rodio 0.17
- status: VERIFIED-AT-HEAD (2026-06-13, root `Cargo.toml`)
- repro: `git grep -nE '^(wgpu|egui|glam|winit|rapier3d|rodio) = ' -- Cargo.toml`
- hardware: n/a
- canonical_source: root `Cargo.toml` `[workspace.dependencies]`
- referenced_by: .zencoder/rules/repo.md:53; gh-pages/rendering.md; gh-pages/ui.md; gh-pages/physics.md

### ai-modes
- metric: hello_companion AI planning modes
- value: 7 (feature-gated; several require non-default features)
- status: VERIFIED-AT-HEAD (2026-06-13, example header + mode enum)
- repro: read `examples/hello_companion/src/main.rs` mode dispatch
- hardware: n/a
- canonical_source: `examples/hello_companion/src/main.rs`
- referenced_by: README.md:100,189,361; CLAUDE.md; .zencoder/rules/repo.md:17,98
- note: SUPERSEDES the stale `6 modes`. CODE-FINDING for docs (cite 7, note feature-gating).

---

## Benchmarks / coverage (PENDING-D2 — value established by measurement in D.2)

### agents-capacity-60fps
- metric: AI-native agent capacity headline @ 60 FPS
- value: **12,700+ CONFIRMED** as a defensible mid-complexity figure. Measured per-agent full end-to-end AI loop: 103 ns (simple) / 708 ns (moderate) / 1.617 µs (complex) → capacity = 16.67 ms ÷ cost = ~162K / ~23.5K / **~10.3K** agents @ 60 FPS. The asserted 12,700 (implies ~1.31 µs/agent) sits between the moderate and complex full-loop points — defensible, neither contradicted nor a single-bench match. (Orchestrator-plan-only throughput is lighter: ~128 ns/agent across 10/50/100/500 in "Multi-Agent Throughput" → ~130K at full budget — a different, lighter workload; do not conflate.)
- status: VERIFIED-AT-HEAD (2026-06-27, D.2.B.A; cargo bench -p astraweave-ai)
- repro: `cargo bench -p astraweave-ai --bench ai_core_loop -- full_end_to_end` (per-agent full loop) + `--bench ai_benchmarks -- "Multi-Agent Throughput"`; capacity = 16.67 ms ÷ per-agent cost
- hardware: i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0 / 2026-06-27 / **platform-default (System) allocator** (the ai benches install no `#[global_allocator]`, per D2B0_RECON)
- canonical_source: `cargo bench -p astraweave-ai` (ai_core_loop + ai_benchmarks harness output)
- referenced_by: README.md:148,193,259,278; CLAUDE.md; docs/architecture/ARCHITECTURE_MAP.md:27; docs/architecture/ai_pipeline.md; astraweave-ai/README.md:40; CHANGELOG.md:23; .zencoder/rules/repo.md:93; docs/current/RENDERER_DEEP_ANALYSIS_AND_MEGALIGHTS_PLAN.md; docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md
- note: SUPERSEDES the retired `103k` / `610k` entity-capacity figures (see §Retired). Entity-iteration capacity ≠ agent capacity; do not conflate. Capacity is **workload-dependent** — 12,700 (full-loop, complex-ish), ~130K (orchestrator-plan-only), and MASTER_BENCHMARK_REPORT's ~186K @ 10% budget (GOAP next_action only) are three different denominators of the same pipeline.

### frame-time-1000-entities
- metric: Frame time / FPS at 1,000 entities (reference profiling workload)
- value: **System allocator: 0.965 ms avg (1,036 FPS); mimalloc (fast-alloc): 0.709 ms avg (1,410 FPS)** — 1,000 entities × 1,000 frames. **CONTEST RESOLVED:** the asserted "2.70 ms" is the demo's **Week-8 TARGET** (`profiling_demo` prints `Target (Week 8): 2.700 ms`), NOT a measured frame time — it was mis-recorded as a measurement. The "1.14 ms" is consistent with a System-allocator measured frame time (per-frame spread 0.86–1.22 ms). Allocator effect: mimalloc ~1.36× faster / ~36% more FPS — real but secondary.
- status: VERIFIED-AT-HEAD (2026-06-27, D.2.B.A; contest resolved — 2.70 ms = target, not a measurement)
- repro: **CORRECTED** (old `cargo bench -p astraweave-ecs` measured the wrong thing — ecs has no 1000-entity frame-time group): System = `cargo run -p profiling_demo --release --no-default-features --features alloc-counter -- -e 1000 -f 1000`; mimalloc = `cargo run -p profiling_demo --release --features alloc-counter,fast-alloc -- -e 1000 -f 1000`
- hardware: i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0 / 2026-06-27 / allocator stamped per value (System vs mimalloc)
- canonical_source: `profiling_demo` binary (`-e 1000 -f 1000`) — "Average frame time" / "Average FPS" line
- referenced_by: README.md:213; astraweave-render/README.md:41; gh-pages/rendering.md:510; gh-pages/benchmarks.md; docs/src/README.md:114; architecture/ecs.md:414 ("1.14 ms" ≈ System measured — was never a true conflict)
- note: **META-DEFECT FIXED** — the prior repro `cargo bench -p astraweave-ecs` does not produce this figure. The 2.70-vs-1.14 "conflict" was a **target-vs-measurement confusion**, not two competing measurements; the recon's "2.37× ratio = allocator artifact" hypothesis is REFUTED (2.70 was never a measurement). Allocator config is a real but secondary axis (System 0.965 / mimalloc 0.709). Both measured configs are well under the 2.70 ms target.

### validation-checks-per-sec
- metric: Tool-sandbox / anti-cheat validation throughput
- value: **~6.3M checks/sec CONFIRMS the asserted 6.48M.** Measured `validate MoveTo` = 158 ns (range 145–171 ns → 5.8M–6.9M/sec); 6.48M ≡ 154 ns, inside the measured range. `validate CoverFire` is heavier (220 ns → ~4.5M/sec) — the 6.48M headline tracks the MoveTo path.
- status: VERIFIED-AT-HEAD (2026-06-27, D.2.B.A; cargo bench -p astraweave-ai)
- repro: `cargo bench -p astraweave-ai --bench ai_benchmarks -- "Tool Validation"`
- hardware: i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0 / 2026-06-27 / platform-default (System) allocator
- canonical_source: `cargo bench -p astraweave-ai` (ai_benchmarks "Tool Validation" group)
- referenced_by: README quality metrics; .zencoder/rules/repo.md:93; gh-pages/index.md:47; gh-pages/ai.md:199

### coverage-weighted
- metric: Weighted line coverage across measured crates
- value: PENDING-D2 — **could not measure cleanly at HEAD (D.2.B.A, 2026-06-27).** `cargo llvm-cov --workspace` is BLOCKED by ≥3 pre-existing broken TEST targets: `astraweave-blend` property_tests (E0063 — `ProcessOptions` missing `parallel_workers`; `ConversionOptions` missing `decomposition`), `astraweave-render` memory_safety_tests (E0432 — `post::BloomConfig` unresolved; bloom split to `bloom.rs`) and wave2_culling_anim_misc_tests (E0063 — `CpuMesh` missing `albedo_image`/`texture_source_hint`). `cargo check --workspace` (130/130) MASKS these — it does not build test targets. Surgical `--exclude` is whack-a-mole and gutting the denominator (render is huge), so no comparable number was banked. The asserted 59.3% was a **29-curated-crate** subset, NOT full-workspace.
- status: PENDING-D2 (blocked — couldn't measure cleanly; see blocker)
- repro: `cargo llvm-cov --workspace --summary-only` — BLOCKED (broken test targets above; enumerate the full set with `cargo build --workspace --tests --keep-going`, fix, then re-attempt). NOTE: asserted 59.3% denominator = 29 curated crates, not `--workspace`.
- hardware: i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0 / 2026-06-27 (tool confirmed working on astraweave-core; headless GPU init works — hazard-2 REFUTED; the blocker is compile errors, not GPU)
- canonical_source: `cargo llvm-cov --workspace --summary-only` (command output) — once test targets compile
- referenced_by: README.md:52,57,79,273,376; docs/current/MASTER_COVERAGE_REPORT.md
- note: **CODE-FINDING** — the workspace has ≥3 broken TEST targets (astraweave-blend ×1, astraweave-render ×2) uncaught by `cargo check --workspace`; they block full-workspace `cargo test`/`llvm-cov`. Routed to a follow-on test-target-fix beat (source changes, out of D.2.B.A docs-only scope). **METHODOLOGY-FINDING** — the asserted 59.3% (29 measured crates, 2026-02-25) and `--workspace` (130 crates) are different denominators. **RATIFIED RESOLUTION (D.2.B.A): Path B** — the test-target-fix beat (the D-series's first source work) fixes the broken targets, then re-baselines true `--workspace` coverage. Hazard-2 (headless GPU crash) REFUTED — failure was compile errors, not GPU init.

### miri-tests
- metric: Miri-validated test count (0 undefined behavior)
- value: **1,059 miri tests pass, 0 failed, 0 UB** across the four crates (ecs 419 · core 503 [+17 ignored under miri] · sdk 28 · math 109). The asserted **977 is SUPERSEDED (stale-low)** — the suites grew. Hazard-3 (SIMD abort) refuted live: math 109/109 incl. SSE2 intrinsics, no abort.
- status: VERIFIED-AT-HEAD (2026-06-27, D.2.B.A; cargo +nightly miri test per-crate)
- repro: `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test -p astraweave-ecs -p astraweave-math -p astraweave-core -p astraweave-sdk --lib` (run per-crate; on Windows `-Zmiri-disable-isolation` is the proven-working flag — the CI `-Zmiri-symbolic-alignment-check -Zmiri-strict-provenance` flags are also compatible)
- hardware: rustc 1.89.0 host / nightly miri 0.1.0 (2300c2aef7, 2025-10-12) / 2026-06-27 / n/a (CPU interpreter, machine-independent)
- canonical_source: `cargo +nightly miri test` per-crate (the "test result: ok" lines)
- referenced_by: README.md:32,59,83,211,276; CLAUDE.md; gh-pages/index.md:22; gh-pages/ecs.md:17; gh-pages/math.md:8
- note: SUPERSEDES the stale `977`. Per-crate passed: ecs 419 (270s), core 503 (237s; 17 ignored under miri), sdk 28 (9s), math 109 (24s) = **1,059**. math runs real SSE2 intrinsics under miri without abort (D2B0_RECON hazard-3 refuted; `simd_vec` falls to scalar, `simd_mat`/`simd_quat` run SSE2 which miri supports).

### mutation-kill-rate
- metric: Mutation testing — total mutants and prompt kill rate
- value: **PROVENANCE-ACCEPTED** — 100% kill on **792** prompt mutants (count re-confirmed at HEAD via `cargo mutants --list -p astraweave-prompts`, D2B0_RECON). The 100%-kill RESULT is from the prior mutation campaign (`docs/current/MUTATION_TESTING_AUDIT.md`, 2026-03-13), NOT re-run this campaign. Prior workspace-wide figures: ~2,928 tests / 4 waves, 100% kill on the audited library crates.
- status: PROVENANCE-ACCEPTED (measured-previously; re-measurement cost-deferred — NOT VERIFIED-AT-HEAD, NOT bare-asserted)
- repro: `cargo mutants -p astraweave-prompts` — **deferred**. A full 792-mutant re-run is ~6–15 h on this machine: the baseline alone exceeded 10 min and NTFS lacks reflink, so cargo-mutants full-copies the ~1.1M-LoC workspace per run (D2B0_RECON §timing). A fresh re-stamp would need a **sharded, resumable** run (`--shard k/n`; Session B, specced-and-deferred).
- hardware: prior-campaign machine (MUTATION_TESTING_AUDIT.md, 2026-03-13); a re-stamp would record i5-10300H / GTX 1660 Ti Max-Q / 2026-06
- canonical_source: prior cargo-mutants campaign (`docs/current/MUTATION_TESTING_AUDIT.md`); mutant count re-confirmed by `cargo mutants --list` at HEAD
- referenced_by: README.md:62,275
- note: **Director decision (D.2.B.A): do NOT re-run this campaign** — the figure is recorded with provenance; the 6–15 h grind is cost-deferred. **Drift direction:** a 100% kill rate cannot drift UP, only down — a fresh sharded run (Session B) would only ever DETECT regression (un-killed mutants from code added since 2026-03-13), never improve the number. Until re-run, treat 100% as the prior-wave result, not a HEAD guarantee.

### water-budget
- metric: Water system per-frame GPU budget (min-spec)
- value: PENDING-D2 (re-measured 2026-06-27, D.2.B.A: combined water surface + F.4 accents worst-case **0.2745 ms** [near cam; horizon 0.159 ms; surface 0.261 ms + accents 0.014 ms], ~7.3× under the **provisional** 2.0 ms ceiling — CONFIRMS the prior ~0.26 ms. Single-machine, not workspace-portable.)
- status: PENDING-D2 (stays PENDING by classification — single-machine + provisional ceiling; NOT flipped to VERIFIED)
- repro: `cargo run -p weaving_playground --example accent_budget_probe --release` (headless wgpu TIMESTAMP_QUERY, medians over 240 frames after 60-frame warmup); isolated water-pass via `cargo run -p astraweave-render --example water_budget_probe --release`
- hardware: NVIDIA GTX 1660 Ti Max-Q · Vulkan · DiscreteGpu · driver 592.27 · 1920×1080 · 2026-06-27 (TIMESTAMP_QUERY + INSIDE_ENCODERS both present — exact match to water.md §9)
- canonical_source: `accent_budget_probe` COMBINED median line (real wgpu TIMESTAMP_QUERY)
- referenced_by: docs/architecture/water.md §9
- note: A real GPU-timestamp measurement, single-machine, the 2.0 ms ceiling self-described provisional — stays PENDING-D2 (NOT VERIFIED-AT-HEAD) per the D.2.A.1 director ruling. **Real-scene full-frame budget gap STANDS:** the windowed demo cannot run headless (W2A §2 / D2B0_RECON hazard-5 CONFIRMED); **no display-mocking/Xvfb attempted** (known-dead path). The isolated probe is the ceiling, not the full-frame headroom. NEW in D.2.A.1, re-measured D.2.B.A.

### dormant-loc-inventory
- metric: Dormant-but-designed research-surface LoC (per the CLAUDE.md dormancy taxonomy)
- value: **~53K** across the six core research crates (`astraweave-fluids` 24,251 · `astraweave-memory` 11,538 · `astraweave-coordination` 5,317 · `astraweave-context` 4,625 · `astraweave-rag` 3,867 · `astraweave-embeddings` 3,184 = **52,782**), plus the LLM-hardening surface (`production_hardening`+`fallback_system`+`tool_guard`+`plan_parser` ≈ 5,840 of ~15K across 16 files) and advanced-GOAP (~16.7K). The former "~200K" headline counted the pre-W.1 fluids reservoir; post-W.1 the six-crate sum is ~53K.
- status: VERIFIED-AT-HEAD (2026-06-25, D.2.A.1 re-resolution post-W.1; `git ls-files | xargs wc -l` per crate — machine-independent)
- repro: `for c in astraweave-fluids astraweave-memory astraweave-coordination astraweave-rag astraweave-embeddings astraweave-context; do git ls-files -- "$c/src/" | grep '\.rs$' | xargs wc -l | tail -1; done`
- hardware: n/a
- canonical_source: `git ls-files | xargs wc -l` over the dormancy-taxonomy crate set
- referenced_by: README.md:128,379; CLAUDE.md:391; docs/architecture/ARCHITECTURE_MAP.md §5
- note: SUPERSEDES the former D.2.A value `108,753` (~109K, verified 2026-06-13). W.1 (2026-06-20) cut the `astraweave-fluids` term from 80,222 → 24,251 (−55,971); the five other crates re-measured byte-identical at HEAD (`memory` 11,538 · `coordination` 5,317 · `context` 4,625 · `rag` 3,867 · `embeddings` 3,184). The **52,782** is the precise machine-independent sum of the six named research crates' `src/`. D.2.B does not touch this (no perf component).

---

## Retired (deleted across the corpus — no registry row, recorded here so D.1.B/D.2 never re-home them)

| Retired figure | Why | Superseded by |
|---|---|---|
| `103,500` / `103k` entities @ 60 FPS | Entity-iteration extrapolation mislabelled as capacity; superseded headline | [agents-capacity-60fps](#agents-capacity-60fps) |
| `610,000` / `610k` entities | Same lineage, older | [agents-capacity-60fps](#agents-capacity-60fps) |
| `10.4× Unity` / `2.1-5.2× Unreal` | FABRICATED competitor multipliers; no citable source | (deleted, not replaced) |
| Unity-9,900 / Unreal-20k-50k baselines | Invented competitor baselines | (deleted, not replaced) |
| `4,907` fluids tests | Stale, ~2× reality | [fluids-test-markers](#fluids-test-markers) |
| `128` workspace / `49` production / `82+` crates | Stale | [workspace-members](#workspace-members) / [production-crates](#production-crates) |
| `3,892` / `6,100` editor tests | Stale | [editor-test-markers](#editor-test-markers) |
| `71+` Kani proofs | Stale | [kani-proofs](#kani-proofs) |

---

## Revision history

| Version | Date | Change |
|---|---|---|
| 0.1 (D.1.A skeleton) | 2026-06-13 | Created. 11 VERIFIED-AT-HEAD count/version rows; 7 PENDING-D2 benchmark/coverage rows; Retired table seeded. PENDING-D2 rows to be filled by D.2 measurement. |
| 0.2 (D.1.B hygiene) | 2026-06-13 | Hygiene pass: `canonical_source` for agents-capacity-60fps / frame-time-1000-entities / coverage-weighted / miri-tests changed from doc-references to the bench/llvm-cov/miri **command** (doc-cites-doc break). Denominators confirmed named ("test markers", not "tests"). `referenced_by` lists are representative load-bearing sites, not exhaustive — D.1.B added registry-comment back-links across the long-tail corpus. |
| 0.3 (D.2.A.1) | 2026-06-25 | **W.1-contamination re-resolution.** Re-resolved the 4 W.1-invalidated rows (fluids-loc 80,222→24,251 src; fluids-test-markers 2,560→738; rust-loc-total ~1.16M→~1.10M raw / ~892K→~854K code; dormant-loc-inventory 108,753→52,782). Fixed 2 broken repro commands (fluids-loc `**`-glob returned 0; rust-loc-total `xargs … wc -l \| tail -1` batching). Added 4 NEW rows for the W.2–F.4 water successor: water-facade-loc, water-surface-loc, water-system (narrative), water-budget (PENDING-D2, single-machine/provisional). Dated D.2.A snapshots in the execution reports left byte-identical (per-row resolution principle). Authority: `D_RESUME_0_RECON.md`. |
| 0.4 (D.2.B.A) | 2026-06-27 | **Fast-tier measurement session** (hardware-stamped i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0). Flipped 4 rows to VERIFIED-AT-HEAD: agents-capacity-60fps (12,700 confirmed, workload-dependent — ~10.3K complex / ~23.5K moderate full-loop / ~130K plan-only); frame-time-1000-entities (**CONTEST RESOLVED** — "2.70 ms" was the demo's Week-8 *target* mis-recorded as a measurement; real System 0.965 ms / mimalloc 0.709 ms; repro corrected to `profiling_demo`); validation-checks-per-sec (6.48M confirmed, 158 ns MoveTo); miri-tests (**1,059** vs stale-low 977; repro corrected to `-Zmiri-disable-isolation`). water-budget re-confirmed 0.2745 ms (stays PENDING-by-classification). coverage-weighted stays PENDING — **BLOCKED** by ≥3 pre-existing broken test targets (CODE-FINDING; Path-B fix beat ratified). mutation-kill-rate → **PROVENANCE-ACCEPTED** (100%/792, prior wave 2026-03-13; re-run cost-deferred). Docs/ledger only; no source changed. Authority: `D2B0_RECON.md`. |
