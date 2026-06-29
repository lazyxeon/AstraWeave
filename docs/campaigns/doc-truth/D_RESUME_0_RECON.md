# D-Resume.0 — Doc-Truth Campaign Resume: State Audit + Scoped Continuation

**Phase:** D-Resume.0 (resume reconnaissance) · **Mode:** read-only · **HEAD at recon:** `63d0581a7`
**Method:** 5-agent parallel audit workflow + 2 free-text re-runs (the schema-failed surfaces) + director-side cross-checks.
**Status:** Ratified. The director ratified D.2.A.1 as a distinct cheap beat ahead of D.2.B (this doc is its authority).

---

## Deliverable 1 — Campaign state-of-record

**D.2.A LANDED in main.** Its report self-describes `HEAD 9693649d8 / Commit: NOT committed — awaiting review`
(`D2A_EXECUTION_REPORT.md:8-11`) — that line is **stale**. The diff was reviewed and cherry-picked in as
**`20467eed2`** (2026-06-19, "docs: Update CLAIMS_REGISTRY and CLOSED_VOCABULARY_ALLOWLIST; add D.2.A Execution
Report", from `f547f5f0da`), which **is** an ancestor of HEAD (15 commits back). Confirmed:
`git merge-base --is-ancestor 20467eed2 HEAD` → ancestor; `9693649d8` → *not* ancestor, and it is actually the
`Fluids-Integration.F.1.4` commit (the workspace tip when D.2.A *ran*, not a doc-truth commit). All three D.2.A
edits verifiably landed: the `EDITOR_STATUS_REPORT.md:282` rewrite, the registry `dormant-loc-inventory` fill
(PENDING-D2 → VERIFIED-AT-HEAD, 108,753), and the allowlist 58-entry resolution.

**Current registry state** (`CLAIMS_REGISTRY.md`) — 18 rows at recon:

| Bucket | Count | Rows |
|---|---|---|
| VERIFIED-AT-HEAD (resolved) | 12 | workspace-members, production-crates, test-markers-total, editor-test-markers, **fluids-test-markers**, **fluids-loc**, **rust-loc-total**, kani-proofs, toolchain, dependency-versions, ai-modes, **dormant-loc-inventory** |
| PENDING-D2 (→ D.2.B) | 5 | agents-capacity-60fps, validation-checks-per-sec, coverage-weighted, miri-tests, mutation-kill-rate |
| CONTESTED-PENDING-D2 (→ D.2.B) | 1 | frame-time-1000-entities (2.70 ms/370 FPS vs 1.14 ms) |

The four bolded "VERIFIED-AT-HEAD" rows were turned **false** by W.1 — D.2.A resolved them on 2026-06-13, W.1 deleted
the fluids sim on 2026-06-20. This is the contamination D.2.A.1 repairs.

**Allowlist** (`CLOSED_VOCABULARY_ALLOWLIST.md`): 266 keys, all `lifetime=permanent`, **0 pending-D2.B**. Consequence:
D.2.A's forward note that "D.2.B flips the pending allowlist entries" is a near-no-op — D.2.B's only ledger writes are
the registry PENDING-D2 → VERIFIED-AT-HEAD flips.

**D.2.B / D.3 artifacts: none exist.** They are the genuine next phases.

---

## Deliverable 2 — W.1-deprecation contamination audit

**Baseline.** W.1 shipped in squash-merge `71a1dde73` (#194); fluids deletion `1a57fdd41` (2026-06-20, ~58.8K
deletions). `astraweave-fluids`: 80,222 → **24,251 src LoC** (19 files; whole-crate `.rs` = **27,257**, 23 files),
test-markers 2,560 → **738**. `simd_ops.rs` (cited 39,554) and `PcisphSystem`/`UnifiedSolver`/`ResearchFluidSystem`
are deleted (verified absent in tree; only a `Cargo.toml` comment still names `PcisphSystem`). The fluids and water
architecture traces are already current (`last_verified 2026-06-25 @ 7c29b8182`); the **registry is the lagging surface**.

### 2a. Contamination inside the campaign's own ledger

Per the per-row resolution principle — current values get re-resolved; dated snapshots stay byte-identical.

**(A) CURRENT-VALUE-STALE — needs re-resolution:**
- `CLAIMS_REGISTRY.md:58` `fluids-test-markers` 2,560 → **738**
- `CLAIMS_REGISTRY.md:66-74` `fluids-loc` 80,222/83,651 → **24,251/27,257** — *and* its repro glob is **broken** (returns 0)
- `CLAIMS_REGISTRY.md:184-192` `dormant-loc-inventory` 108,753 → **52,782** (fluids term 80,222 → 24,251; 5 other crates byte-identical)
- `CLAIMS_REGISTRY.md:73-74` the "84K mildly stale" note (now doubly stale)
- `CLAIMS_REGISTRY.md:76-84` `rust-loc-total` ~1.16M/892K (verified 2026-06-13, predates W.1) → re-measure
- `CLOSED_VOCABULARY_LINT.md:18` corrected-value annotation `…-> 2,560` is now stale (poison string `4,907` membership stays correct)
- `D0_CLAIMS_INVENTORY.md:51,53` — the §1.2 "ground truth at HEAD" anchor cells (2,560 / 80,222), the live anchor every STALE verdict derives from
- `D01_GAP_INVENTORY.md:651,678,921,950` — D01-0049 (~84K), D01-0076 (SpatialHash→`fluids/simd_ops`, now dangling), D01-0298, D01-0327 present-tense corrected-current values
- `CLOSED_VOCABULARY_ALLOWLIST.md:164,176,178` — whitelist `ResearchFluidSystem`/`UnifiedSolver` (deleted/never-existed). *Low severity, nuanced:* gone at F.1 (pre-W.1), so W.1 didn't newly break them — the question is whether the campaign still wants the keys.

**(B) DATED-FROZEN-LEAVE — must NOT be touched (≈300+ rows):** all fluids figures inside the execution reports
(incl. **D2A:34 and D2A:68**, dated 2026-06-13 records), the entire `CLOSED_VOCABULARY_OCCURRENCES.md` +
`CLOSED_VOCABULARY_TRIAGE.md` evidence snapshots (~208 rows; their internal `A/B` column means *vocabulary class*),
the ~38 D0 + ~40 D01 dated audit-row verdicts (incl. `D0:623` which VERIFIED `simd_ops.rs = 39,554` — now
false-at-HEAD but a dated sweep measurement, stays).

### 2b. Broader corpus — what W.2 caught vs. the gap

**ALREADY-BANNERED by W.2 Phase 2 (do NOT re-deprecate):** `PROJECT_STATUS.md:183-191`,
`MASTER_COVERAGE_REPORT.md:187,196`, both MASTER_BENCHMARK_REPORTs, `KANI_VERIFICATION_PLAN`, the three focused
fluids/water plan docs, and `fluids.md` (banner-gated at §0.5). All carry self-dated "(added W-series W.2 Phase 2,
2026-06-21)" banners inside `71a1dde73`.

**THE GAP:**
1. **W-series deprecation is entirely absent from the doc-truth ledger** (the only "W.1/W.2" strings there refer to
   the unrelated Net-Trio signature W-series). The two campaigns are silently disjoint.
2. **Two highest-authority docs W.2 never touched, still describing the deleted simulator as live:**
   - `README.md:265` asserts `PcisphSystem` as a live solver (also :178/:207 SPH-as-parallelism, :327 PBD framing).
   - `ARCHITECTURE_MAP.md:23,240,241,243,372,927` — the canonical cross-crate map asserts 84.5K LoC / `simd_ops.rs`
     39,554 / `PcisphSystem` / "five parallel solver surfaces". Crate is now 24.2K/19 files.
   - (minor) `ecs_math_core_sdk_foundation.md:403` lists "astraweave-fluids SPH" rayon parallelism as live.
3. **Inherited open item (out of D-series core):** `fluids.md` body (§5/§6/§7/§11) still present-tense behind the
   §0.5 banner; `fluids.md:924` defers a full re-verification pass — a trace-maintenance task.

### 2c. New truth to enter (all verified at HEAD `63d0581a7`)

| Registry target | Old | New (measured) |
|---|---|---|
| `fluids-loc` | 80,222 src / 83,651 | **24,251 src / 27,257 whole** + fixed repro |
| `fluids-test-markers` | 2,560 | **738** |
| `dormant-loc-inventory` | 108,753 (~109K) | **52,782 (~53K)**; fluids term 24,251 |
| `rust-loc-total` | ~1.16M raw / ~892K code | **~1.10M raw (1,104,208) / ~854K code (853,992)** |
| **NEW** `water-facade-loc` | — | `astraweave-water` 350 src / 428 whole / 9 markers (WaterQuery + AnalyticWater, CPU-deterministic) |
| **NEW** `water-surface-loc` | — | render `water.rs` 991 + `water.wgsl` 382 = **1,373** (WaterRenderer) |
| **NEW** water-system narrative | — | post-W.1 = rendering + truth-facade, NOT simulation: 5 components (WaterQuery facade · chunked-LOD Gerstner surface · screen-space refraction + depth-foam split pass · part/freeze/raise weave deformation bounded ±SKIRT_DEPTH · F.4 GPU-particle accent layer); `FreezeWater` presentation-only |
| **NEW** water-perf (PENDING-D2) | — | combined surface+accents ≈ 0.26 ms worst-case (1660 Ti Max-Q), ~8× under the **provisional** 2.0 ms ceiling (`water.md §9`) |

**Meta-defect:** the `fluids-loc` repro `astraweave-fluids/src/**/*.rs` **returns 0** at HEAD (git `**` pathspec
doesn't match files directly under `src/`). Fix must land with the value change. (D.2.A.1 also found a second repro
defect: `rust-loc-total`'s raw repro `git ls-files '*.rs' | xargs wc -l | tail -1` is machine-dependent — xargs
batching makes `tail -1` return only the last batch.)

---

## Deliverable 3 — Trace-sync coverage delineation

**CI gate** (`.github/workflows/trace-sync.yml:48,51`): `--validate-only --list-untraced` (informational) then
`--check` (drift gate; non-zero exit if `--write` would change CLAUDE.md or `workspace_map.html`).

| MACHINE-ENFORCED (exclude from D.2.B/D.3) | STILL-MANUAL (in scope) |
|---|---|
| CLAUDE.md trace table + descriptions (`TRACE-TABLE` block) | **CLAIMS_REGISTRY values** + prose-claim corpus |
| trace↔crate ownership linkage (`owns`/`primary_crate`) | trace BODY prose (§6/§7/§8/§11) |
| `workspace_map.html` per-crate `trace` link; primary `status`/`statusCategory`/`statusEvidence`; per-edge `runtime` | map secondary node statuses + curated overlay + node/edge topology (deferred `map_overlay.json`) |
| the two CLAUDE.md prose-enumeration pointers (point at the table) | **README claims, ARCHITECTURE_MAP body prose, MASTER_*_REPORT figures outside the table**, `docs/src/` wiki |
| mutual consistency of the 3 trace surfaces | **front-matter-verdict-vs-code truth** (`--check` proves agreement, not factual truth) |

**Untraced crates: 26 confirmed** (tool run + `cargo metadata`: 132 total / 59 examples / 73 non-example / 26
untraced). ~10 real library subsystems; ~16 CLI/dev tools.

**Recommendation: keep the untraced worklist SEPARATE from the D-series** — a different deliverable type (net-new
trace authoring vs. claim-truth audit), already CI-tracked by `--list-untraced`. *Director decision.*

---

## Deliverable 4 — Scoped D.2.B + D.3 proposal

- **W.1-contamination → its own cheap beat `D.2.A.1`** (RATIFIED). Reasoning: cheap/machine-independent (same cost
  class as D.2.A, opposite of D.2.B); bundling violates the cost boundary; it is live contamination that should not
  wait out the long perf campaign; large enough to warrant its own hard-stop-for-commit beat.
- **D.2.B — expensive re-baseline** (perf/coverage/miri/mutants), the 6 registry rows + (director's call) the inline
  perf-doc corpus by crate, hardware-stamped, checkpointed. Trace-sync exclusion is near-no-op here (perf rows aren't
  in the trace table). The allowlist-flip sub-step is a no-op (0 pending entries).
- **D.3 — regression lint** materially narrowed by trace-sync: closed-vocabulary lint over the still-manual prose
  corpus, honoring the 266-key allowlist, **excluding** the CLAUDE.md `TRACE-TABLE` block and `workspace_map.html`
  data fields (CI `--check` gates them). Optional enhancement: a registry self-verification lint (each value
  reproducible by its repro command) — the class of defect D.2.A.1 found.

---

## Deliverable 5 — Sub-phasing + sequencing

```
D.2.A    ✅ landed (20467eed2)
D.2.A.1  ← W.1-CONTAMINATION RE-RESOLUTION   [CHEAP, machine-independent]   ◀ this beat
D.2.B    ← EXPENSIVE RE-BASELINE             [perf/coverage/miri/mutants, hardware-stamped]
D.3      ← REGRESSION LINT                   [closed-vocabulary, scoped to still-manual surfaces]

Separate tracks (NOT D-series): S.1 (security findings) · trace-completion (26 untraced) · trace-maintenance (fluids.md body re-verify)
```

**Director decisions surfaced at the gate:** (1) W.1-contamination own beat — RATIFIED; (2) untraced worklist stays
separate — recommended; (3) D.2.B scope (6 rows vs. full perf corpus); (4) water-perf classification — D.2.A.1 enters
it PENDING-D2; (5) two water rows not one — adopted; (6) annotate (not rewrite) the stale D2A front-matter — adopted.
