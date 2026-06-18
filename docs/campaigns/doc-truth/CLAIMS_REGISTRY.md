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
- value: 2,560 (2,454 src + 106 `tests/`)
- status: VERIFIED-AT-HEAD (2026-06-13, `git grep -c` scoped to `astraweave-fluids`)
- repro: `git grep -E -c '#\[(tokio::)?test\]' -- 'astraweave-fluids' | awk -F: '{s+=$NF} END {print s}'`
- hardware: n/a
- canonical_source: `git grep` scoped to `astraweave-fluids`
- referenced_by: README.md:267; docs/masters/MASTER_ROADMAP.md:86,91,102,474; docs/current/MASTER_COVERAGE_REPORT.md:187; gh-pages/crates.md:43
- note: SUPERSEDES the poison value `4,907`. The mildly-stale `2,509` / `2,404` also resolve here.

### fluids-loc
- metric: `astraweave-fluids` source lines
- value: 80,222 (src) / 83,651 (whole crate incl. `tests/`)
- status: VERIFIED-AT-HEAD (2026-06-13, `git ls-files | xargs wc -l`)
- repro: `git ls-files 'astraweave-fluids/src/**/*.rs' | xargs wc -l | tail -1`
- hardware: n/a
- canonical_source: `wc -l` over `astraweave-fluids/src`
- referenced_by: CLAUDE.md (84K); docs/audits/water_system_architecture_2026-04-20.md:128 (3,810 — STALE)
- note: SUPERSEDES the poison `46,173`; the `84K`/`84.5K` figures are mildly stale.

### rust-loc-total
- metric: Total Rust lines across the workspace
- value: ~1.16M raw / ~892K code-only (tokei, excludes comments/blanks)
- status: VERIFIED-AT-HEAD (2026-06-13, tokei + `git ls-files` cross-check)
- repro: `tokei --type Rust .` (code-only) or `git ls-files '*.rs' | xargs wc -l | tail -1` (raw)
- hardware: n/a
- canonical_source: tokei
- referenced_by: README.md:113 (850 K+); gh-pages/index.md:18 (860,000+)
- note: The `850K+` / `860K+` figures are stale lower bounds (understated).

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
- value: PENDING-D2 (current headline asserts 12,700+; not re-measured this campaign)
- status: PENDING-D2
- repro: `cargo bench -p astraweave-ai` (+ `-p astraweave-stress-test`); divide 16.67 ms frame budget by per-agent cost
- hardware: HP Pavilion Gaming Laptop 16-a0xxx (per MASTER_BENCHMARK_REPORT.md:99) — confirm at measurement
- canonical_source: `cargo bench -p astraweave-ai` / `cargo bench -p astraweave-stress-test` (bench harness output)
- referenced_by: README.md:148,193,259,278; CLAUDE.md; docs/architecture/ARCHITECTURE_MAP.md:27; docs/architecture/ai_pipeline.md; astraweave-ai/README.md:40; CHANGELOG.md:23; .zencoder/rules/repo.md:93; docs/current/RENDERER_DEEP_ANALYSIS_AND_MEGALIGHTS_PLAN.md; docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md
- note: This entry SUPERSEDES the retired `103k` / `610k` entity-capacity figures (see §Retired). Entity-iteration capacity ≠ agent capacity; do not conflate.

### frame-time-1000-entities
- metric: Frame time / FPS at 1,000 entities (reference profiling workload)
- value: PENDING-D2 (asserted 2.70 ms / 370 FPS; conflicting 1.14 ms also seen — resolve in D.2)
- status: CONTESTED-PENDING-D2
- repro: `cargo bench -p astraweave-ecs` (or the producing game-loop bench); cross-check `profiling_demo`
- hardware: reference profiling laptop — confirm at measurement
- canonical_source: `cargo bench -p astraweave-ecs` (bench harness output)
- referenced_by: README.md:213; astraweave-render/README.md:41; gh-pages/rendering.md:510; gh-pages/benchmarks.md; docs/src/README.md:114 (conflicts: architecture/ecs.md:414 says 1.14 ms)

### validation-checks-per-sec
- metric: Tool-sandbox / anti-cheat validation throughput
- value: PENDING-D2 (asserted 6.48M checks/sec)
- status: PENDING-D2
- repro: `cargo bench -p astraweave-ai` (tool-sandbox validation bench)
- hardware: reference laptop — confirm at measurement
- canonical_source: bench harness
- referenced_by: README quality metrics; .zencoder/rules/repo.md:93; gh-pages/index.md:47; gh-pages/ai.md:199

### coverage-weighted
- metric: Weighted line coverage across measured crates
- value: PENDING-D2 (asserted 59.3%; last full measurement 2026-02-25)
- status: PENDING-D2
- repro: `cargo llvm-cov --workspace --summary-only`
- hardware: n/a
- canonical_source: `cargo llvm-cov --workspace --summary-only` (command output)
- referenced_by: README.md:52,57,79,273,376; docs/current/MASTER_COVERAGE_REPORT.md

### miri-tests
- metric: Miri-validated test count (0 undefined behavior)
- value: PENDING-D2 (asserted 977 across ecs/math/core/sdk)
- status: PENDING-D2
- repro: `cargo +nightly miri test -p astraweave-ecs -p astraweave-math -p astraweave-core -p astraweave-sdk --lib -- --test-threads=1` (flags `-Zmiri-symbolic-alignment-check -Zmiri-strict-provenance`)
- hardware: n/a
- canonical_source: `cargo +nightly miri test` (the repro command above; command output)
- referenced_by: README.md:32,59,83,211,276; CLAUDE.md; gh-pages/index.md:22; gh-pages/ecs.md:17; gh-pages/math.md:8

### mutation-kill-rate
- metric: Mutation testing — total mutants and prompt kill rate
- value: PENDING-D2 (asserted ~2,928 tests / 4 waves; 100% kill on 792 prompt mutants)
- status: PENDING-D2
- repro: `cargo mutants -p astraweave-prompts` (+ the wave shards in `docs/current/MUTATION_*`)
- hardware: n/a
- canonical_source: cargo-mutants run
- referenced_by: README.md:62,275

### dormant-loc-inventory
- metric: Dormant-but-designed surface (aggregate LoC across six categories)
- value: PENDING-D2 (asserted ~200K)
- status: PENDING-D2
- repro: `tokei astraweave-fluids astraweave-memory astraweave-coordination astraweave-rag astraweave-llm ...` per the CLAUDE.md dormancy taxonomy
- hardware: n/a
- canonical_source: tokei over the dormancy-taxonomy crate set
- referenced_by: README.md:128,379; CLAUDE.md:391; docs/architecture/ARCHITECTURE_MAP.md §5

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
