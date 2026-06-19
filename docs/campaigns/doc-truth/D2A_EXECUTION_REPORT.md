# D.2.A Execution Report — Cheap Machine-Independent Verification

**The cheap half of D.2.** Resolved every deferred row verifiable by counts, per-file LoC, caller-existence, and code-existence — answers identical on any machine, seconds-to-minutes. Filled the cheap registry row, flipped the resolved allowlist entries, routed everything expensive to D.2.B. **HARD STOP** for review + commit.

| | |
|---|---|
| Date | 2026-06-13 |
| HEAD | `9693649d8` |
| Doc edits | 1 REWRITE (`EDITOR_STATUS_REPORT.md:282`) + registry `dormant-loc-inventory` filled + allowlist re-emitted |
| Cost boundary | counts / LoC / callers / code-existence only — **no** perf, coverage, miri, mutants |
| Commit | NOT committed — awaiting review |

## 1. Resolution principle applied

Most contested numbers live in **dated audit/design/plan docs** (RENDERING_INFRASTRUCTURE_AUDIT Jan-2025, PBR_F design, COMPREHENSIVE/EXTERNAL audits, VEILWEAVER analysis). Per the per-row rule, a number *scoped to its commit/date* is **LEFT byte-identical** (correcting it falsifies the record); the registry holds the current value. So D.2.A's job here was mostly **measure → record in ledger → LEAVE the dated doc**, plus the few genuine present-tense corrections and the caller-checks. Several of these docs were already reframed-as-historical in D.1.A (RENDERING_INFRASTRUCTURE_AUDIT "Jan-2025 snapshot", PBR_F "historical design document, superseded 4-layer schema"), which is why their LoC/struct rows are LEAVE.

## 2. Measurement ledger (machine-independent, at HEAD)

| Surface | Doc claim | Measured (`wc -l` / grep) | Action |
|---|---|---|---|
| `astraweave-render/src/clustered_megalights.rs` | 534 LOC | **679** | LEAVE (RENDERING_INFRASTRUCTURE_AUDIT Jan-2025 dated snapshot; recorded) |
| `…/shadow_csm.rs` | 722 LOC | **1011** | LEAVE (dated) |
| `…/ssao.rs` | 634 LOC, "Production Ready" | **ABSENT** (git log: existed historically, removed since) | LEAVE (dated) + **CODE-FINDING** |
| `…/post.rs` | 964 LOC | **110** (bloom split to `bloom.rs` **486**) | LEAVE (dated) |
| `…/advanced_post.rs` | 604 LOC | **725** | LEAVE (dated) |
| `…/clustered_forward.rs` | 462 LOC | **634** | LEAVE (RENDERER_DEEP Nov-2025 plan; recorded) |
| render module count | "45 Rendering Modules" | **118** top-level `.rs` | LEAVE (COMPREHENSIVE dated) |
| `clustered_megalights` module-wired? | (implied) | **yes** — `lib.rs:36 pub mod clustered_megalights` | confirms MegaLights reachable |
| VXGI present? | "VXGI" capability | **yes** — `lib.rs:42 pub mod gi; // VXGI` | capability claim valid (module exists) |
| `TerrainMaterialGpu` size / layers | "320 bytes, 4 layers" (PBR_F) | **`assert_eq!(size_of, 2112)`; `MAX_TERRAIN_LAYERS = 32`** | LEAVE (PBR_F reframed-historical in D.1.A) + **CODE-FINDING** (code is 32/2112) |
| `astraweave-gameplay/src/veilweaver.rs` | 736 LOC (VEILWEAVER slice) | **ABSENT** at that path | LEAVE (VEILWEAVER analysis dated) + **CODE-FINDING** |
| SpatialHash external callers | "broadphase / 99.96%" | **1** (only `profiling_demo`, no production) | confirms **dormant** (live broadphase = Rapier `DefaultBroadPhase`) |
| `auto_save_system` body | "autosaves" | **still `{ // TODO }`** (`persistence-ecs/lib.rs:72`) | confirms **dormant stub** |
| fluids `src/` LoC | (registry 80,222) | **80,222** (`wc -l`, 34 files) — reconciled | registry **confirmed correct** |
| dormant research crates LoC | "~200K" | **108,753** (6 named crates; see registry) | registry `dormant-loc-inventory` → **VERIFIED-AT-HEAD** |

## 3. Doc corrections applied (present-tense only)

- **`EDITOR_STATUS_REPORT.md:282`** — REWRITE. "Editor is production-ready pending Phase 1 completion" (self-contradictory over-claim on a prototype) → "Editor is a feature-prototype; not yet production-ready (pending Phase 1 completion — see PRODUCTION_READINESS_AUDIT)". The editor is the one present-tense `production-ready` claim on a genuinely-non-shipped surface (its own sibling audit scores it NOT READY).

All other measured rows are **LEAVE** (dated-historical; the measurement is recorded above for the record but the doc is not edited — correcting a dated snapshot falsifies it).

## 4. The 58 `production-status-contested` allowlist entries — resolved by caller-check

| Verdict | n | Basis |
|---|---:|---|
| historical-dated (permanent keep) | 48 | The `production-ready` is a dated audit/`*_COMPLETE`/roadmap assessment scoped to its date |
| production-shipped (permanent keep) | 7 | Names a genuinely-shipped surface with real callers: glam math, `perform_attack_sweep`, GPU skinning, Core/ECS, render |
| future-target (permanent keep) | 2 | "X% production-ready" / "to production-ready" roadmap framing |
| tier-definition / technical-ref / faq-question (permanent keep) | 3 | A tier *definition*, a SOTA technique reference, a neutral FAQ *question* |
| **REWRITE** | 1 | `EDITOR_STATUS_REPORT.md:282` (§3) |

**None left as pending-D2.** The allowlist's 58 entries are flipped to their permanent disposition (or removed where rewritten). The `99.96%` SpatialHash family (14 entries) is resolved as **combinatorial-dormant** — `499,500 → 180` is a deterministic pair-count ratio (`n(n-1)/2`), machine-independent, describing the *dormant* module — permanent honest-dormancy, **not** a D.2.B perf row.

## 5. CODE-FINDINGs (routed, not enshrined)

| Finding | Route to |
|---|---|
| `ssao.rs` cited "Production Ready, 634 LOC" but the file does **not exist** at HEAD (removed since the Jan-2025 audit) | astraweave-render / render trace |
| `astraweave-gameplay/src/veilweaver.rs` (cited 736 LOC) does **not exist** at that path | astraweave-gameplay / VEILWEAVER campaign |
| PBR_F design doc says terrain "4 layers / 320 bytes"; the **code asserts** `size_of::<TerrainMaterialGpu>() == 2112` and `MAX_TERRAIN_LAYERS == 32` (bumped 2026-05-08, Real-Fix.D) | astraweave-render / terrain trace (doc already reframed-historical in D.1.A) |
| `post.rs` is 110 LOC (the audit's "964" — bloom split to `bloom.rs`) | render trace (informational) |

These are not enshrined as corrected doc values (the docs are dated snapshots); they are findings for the owning surface.

## 6. Registry update

- **`dormant-loc-inventory`** PENDING-D2 → **VERIFIED-AT-HEAD**: 108,753 LoC across the six named research crates (fluids 80,222 · memory 11,538 · coordination 5,317 · context 4,625 · rag 3,867 · embeddings 3,184), machine-independent (`git ls-files | xargs wc -l`). The "~200K" headline retained as the defensible full-taxonomy upper bound.
- All other VERIFIED-AT-HEAD count rows (members, markers, editor, fluids, kani, toolchain, deps, production-crates, ai-modes, LoC) were already filled in D.1.A and re-confirmed; no change.

## 7. Runnable-vs-marker note

The registry count rows are **test markers** (`#[test]`+`#[tokio::test]` grep — machine-independent, no compile), and they **name that denominator**. The **runnable** counts (`cargo test -p <crate> -- --list`) are a *different, slightly lower* number that requires per-crate test-binary compilation (borderline the cost boundary, and the editor binary is multi-minute). They are deferred to **D.2.B**, which compiles each crate anyway for the test/coverage run — capturing runnable counts there is free. The contested editor sub-counts (71 / 429 / 1,681 / 3,970 / 4,010) and physics (103-vs-209) live in the **honest-dormancy trace docs** (`aw_editor.md`, `physics.md`) that *document* the breakdown — already KEEP, not corrections.

## 8. PENDING-D2.B manifest (D.2.B's input — expensive AND/OR machine-specific)

Each needs a bench/coverage/miri/mutants run, **hardware-stamped** (i5-10300H / GTX 1660 Ti Max-Q / rustc 1.89.0 / date). Registry PENDING-D2 rows:

| Registry row | Repro | Crate(s) |
|---|---|---|
| `agents-capacity-60fps` (12,700) | `cargo bench -p astraweave-ai` / `-p astraweave-stress-test` | ai, stress-test |
| `frame-time-1000-entities` (2.70 ms / 370 FPS; vs 1.14 ms) | `cargo bench -p astraweave-ecs` | ecs |
| `validation-checks-per-sec` (6.48M) | `cargo bench -p astraweave-ai` (tool-sandbox bench) | ai |
| `coverage-weighted` (59.3%) | `cargo llvm-cov --workspace --summary-only` | workspace |
| `miri-tests` (977) | `cargo +nightly miri test -p astraweave-ecs -p astraweave-math -p astraweave-core -p astraweave-sdk --lib` | ecs/math/core/sdk |
| `mutation-kill-rate` (792 mutants) | `cargo mutants -p astraweave-prompts` | prompts |

Plus the perf doc rows deferred from D.1 (all ns/µs/ms/FPS/throughput): render perf (LOD 68-2110 µs, vertex 16-29 ns, GPU mesh 10-100×, MegaLights dispatch 37-44 µs), AI/GOAP/BT/arbiter ns figures, physics perf (114 ns char move, 6.52 µs tick), SIMD speedups (2.08×, 1.7-2.5×), nav (142k QPS), LLM latency (Qwen3/Hermes ms), and the sub-ns figures (triage as optimizer-elision CODE-FINDINGs, do not enshrine). D.2.B batches these **by crate, checkpointed, resume-safe**.

## 9. Verification gate
- [x] Every cheap-verifiable deferred row resolved (measured → CORRECT/REWRITE/LEAVE/CODE-FINDING/registry-fill) or routed to D.2.B with repro
- [x] Every count names its denominator (markers, not "tests"); runnable-vs-marker explained (§7)
- [x] No machine-specific value (coverage/perf/miri/mutants) corrected here — all in §8
- [x] Dated-historical rows left byte-identical; only the present-tense `EDITOR_STATUS:282` corrected
- [x] Degenerate/structural findings flagged as CODE-FINDINGs (§5), not enshrined
- [x] All 58 production-status entries resolved by caller-check (§4); **0 left pending-D2**
- [x] `docs/journey/**` + `docs/archive/**` untouched by the campaign (the env URL-linter caveat from D.1.C still applies)
- [x] `git status`: exactly 4 files — `EDITOR_STATUS_REPORT.md`, `CLAIMS_REGISTRY.md`, `CLOSED_VOCABULARY_ALLOWLIST.md`, this report. (D.1 = A+B+C was committed after its review; D.2.A's diff is cleanly isolated.)

## 10. HARD STOP
Stopping. **Not committed.** Andrew reviews the D.2.A diff — the LoC/count measurements (right denominator? any dated-historical wrongly corrected? — no: only `EDITOR_STATUS:282` was edited, all dated LoC LEFT), the 58 caller-check verdicts, and the registry `dormant-loc-inventory` fill — then commits.

**Forward chain:** D.2.A → **D.2.B** (the expensive re-baseline: coverage / perf / miri / mutants per §8, batched by crate, checkpointed, hardware-stamped; sub-ns triaged as elision CODE-FINDINGs; flips the `pending-D2.B` allowlist entries as resolved) → **D.3** (regression lint: manifest + `CLOSED_VOCABULARY_LINT.md` honoring `CLOSED_VOCABULARY_ALLOWLIST.md`; `.zencoder/chats/` excluded; alias entries scoped to `cargo `) + CLAUDE.md §7.x. Separately: **S.1** scopes the routed security findings.
