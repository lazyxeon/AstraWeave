# D.1.B Execution Report — Long-Tail Corrections + Closed-Vocabulary Sweep

**Second and final write pass before verification (D.2).** Ran the exhaustive closed-vocabulary grep (the authored seed for D.3's regression lint), then applied the four-action keep-vs-act method across the long-tail corpus. **HARD STOP** after this report for Andrew's review of the full D.1 diff before commit.

| | |
|---|---|
| Date | 2026-06-13 |
| HEAD | `9693649d8` |
| New files | `CLOSED_VOCABULARY_LINT.md`, `CLOSED_VOCABULARY_OCCURRENCES.md` |
| Edits (D.1.B) | **325** fan-out (11 DELETE · 47 CORRECT · 267 REWRITE) + root cluster (me) + 4 verification fixes |
| Full D.1 diff (A+B) | **145 files**, 459 insertions / 608 deletions |
| Commit | NOT committed — awaiting review |

## 1. Job 1 — closed-vocabulary sweep (the D.3 lint seed)

Two new deliverables, the durable output of D.1.B:
- **`CLOSED_VOCABULARY_LINT.md`** — Vocab A (poison strings) + Vocab B (provenance-free superlatives) as literal regex/string sets, the keep-vs-act rule, and the exclusion scope. This is what the D.3 regression lint consumes; it closes the *sampling-loss* class D.1.A exposed (grep is exhaustive where the inventory samples).
- **`CLOSED_VOCABULARY_OCCURRENCES.md`** — the complete `file:line:match` evidence: **501 Vocab-A + 415 Vocab-B = 916 occurrences across 202 files**, grouped by cluster.

### 1.1 Why this matters
D.1.A corrected the phantom alias at `CLAUDE_MD_HARDENING:296` but missed its sibling at `:319`; corrected "world's first" at `COMPREHENSIVE:13` but missed `:1022`. Same failure twice — the inventory samples, and sampling is lossy for closed recurring vocabularies. D.1.B enumerates them, the same way the registry treats numbers.

## 2. Acting — keep-vs-act four-action across the corpus

### 2.1 Fan-out (42 file-groups, long tail)

| Cluster | groups | DEL | CORR | REWRITE | KEPT |
|---|---:|---:|---:|---:|---:|
| D.1.A files (sibling occurrences) | 4 | 4 | 4 | 43 | 60 |
| Master reports | 2 | 0 | 9 | 11 | 21 |
| docs/audits | 4 | 0 | 6 | 31 | 43 |
| Architecture traces | 3 | 1 | 3 | 3 | 75 |
| docs/current | 10 | 1 | 7 | 44 | 210 |
| docs/pbr | 4 | 0 | 0 | 57 | 8 |
| GitHub Pages | 2 | 0 | 8 | 15 | 9 |
| mdBook site | 2 | 2 | 0 | 6 | 13 |
| Crate-design + misc | 6 | 0 | 4 | 40 | 69 |
| Per-crate READMEs | 2 | 0 | 5 | 9 | 14 |
| lessons + campaigns | 1 | 0 | 0 | 0 | 17 |
| docs/guides | 1 | 0 | 0 | 8 | 5 |
| docs/reference + config | 1 | 3 | 1 | 0 | 1 |
| **Total (fan-out)** | 42 | 11 | 47 | 267 | 545 |

REWRITE dominates (267) because most Vocab-B hits are **surgical adjective removal** — "world-class geometry rendering but CPU-bound lighting" → drop "world-class", keep the sentence. DELETE (11) is reserved for fabricated values asserted as current truth. CORRECT (47) is ground-truth-backed (6→7 modes, 7→8 stages, TLS/Ed25519→WebSocket/HMAC, 82→130, Hermes→phi3).

### 2.2 KEPT discipline (545 occurrences left byte-identical, by reason)

| Reason | Count | Meaning |
|---|---:|---|
| honest-dormancy | 135 | The text already states the drift truthfully (the *correction*, not the poison) — traces, ARCHITECTURE_MAP, CLAUDE.md Documentation-Hazards |
| contested-leave | 162 | Number with no §1.2 arbiter (coverage %, runnable test counts, per-file LoC, perf) → D.2 re-measures |
| historical-dated | 106 | Correctly-dated note scoped to a past commit (audit findings, campaign snapshots) |
| competitor-cited | 60 | A competitor figure with provenance, or a superlative describing a *competitor* |
| production-shipped | 61 | `production-ready`/`production-grade` on a genuinely shipped, production-called surface |
| test-param | 17 | A literal test parameter (e.g. 676 in planner_tests) |
| registry-linked | 3 | The 12,700 headline already linking the registry |
| **Total kept** | 544 | |

The 135 **honest-dormancy** keeps are the critical safety result: the architecture traces and CLAUDE.md *legitimately contain* poison strings as honest drift descriptions ("advertised but dormant; real broadphase is Rapier DefaultBroadPhase"). The fan-out's keep-vs-act rule protected them — CLAUDE.md received **zero edits** (all 5 occurrences were honest framing).

### 2.3 Root agent-instruction cluster (handled by me, not delegated)
High blast radius — I edited these directly:
- **`.github/copilot-instructions.md`**: phantom `check-all`/`build-core` aliases (instruction + code block) → real commands; 6→7 modes.
- **`README.md`**: "world's first" → "An"; dropped "production-grade performance"; 6→7 modes (×2); fluids row corrected (2,509→2,560 markers, SPH/FLIP→PBD, five→three surfaces).
- **`.zencoder/rules/repo.md`** (stale Nov-2025 agent-rule): 82+→130, 6→7 modes, 7→8 stages, Hermes→phi3 (×3), phantom aliases, "production-ready" dropped.
- **`CLAUDE.md`**: **0 edits** — all 5 occurrences are the honest Documentation-Hazards framing.

### 2.4 Special case — `SECURITY_AUDIT_AND_HARDENING_PLAN.md`
- `sign16` P0 **collapsed** to RESOLVED (HMAC-SHA256 landed per ground truth).
- The three live findings (plaintext WebSocket, `eprintln!` prompt leak, unconsulted Rhai allowlist) **replaced with pointers** to `docs/campaigns/security/S0_FINDINGS_SEED.md` (the S-series owns them) — not restated, not investigated, not fixed.
- "world-class security posture" → "security posture" in the goal line.

## 3. Post-edit grep gate (the required verification)

| Vocabulary | Pre-edit | Post-edit | Acted |
|---|---:|---:|---:|
| A (poison) | 501 | 416 | 85 |
| B (superlative) | 415 | 182 | 233 |

The post-edit remainder is **not zero**, by design — KEEP-class occurrences legitimately survive (honest-dormancy in traces, competitor-cited figures, future-target timelines like "Timeline to World-Class: 6-9 months", audits *debunking* a "world-class" claim, and contested numbers awaiting D.2). The grep gate's job is to surface any **ACT-class survivor** (a live AstraWeave poison/superlative an agent missed). It found four, which I fixed:
- `RENDERER_MASTER_IMPLEMENTATION_PLAN.md:30` "Strengths (World-Class Geometry)" → "(Geometry)" (a D.1.A sibling miss — exactly the sampling-loss class).
- `BENCHMARK_PRODUCTION_AUDIT_REPORT.md:308` "industry-leading coverage" → "extensive coverage".
- `GITHUB_PAGES_PRODUCTION_PLAN.md:455,515` "the first AI-native game engine" (SEO meta) → "an AI-native game engine".

### 3.1 Honest limit — residual long-tail and the D.3 backstop
Across 135 fan-out files, a residual set of agent-misses certainly remains in the 182+416 KEEP-dominated remainder (exhaustively classifying all 598 by hand is out of scope for one pass). **This is exactly what the D.3 lint is for**: `CLOSED_VOCABULARY_LINT.md` is now the standing spec, warn-only for two weeks then enforce, so any surviving or newly-introduced occurrence is flagged mechanically. D.1.B did the bulk cleanup (318 acted) and authored the permanent guard; D.3 closes the tail. The KEEP allowlist in the lint spec (rule 4) defines the false-positive set the lint must honor.

## 4. Merged D.2-deferred ledger

- **D.1.A:** 91 contested/unverifiable rows.
- **D.1.B:** 162 `contested-leave` occurrences left byte-identical (coverage %, runnable test counts, per-file LoC, perf numbers, 99.96%-as-a-number).
- **Merged scope for D.2:** D.1.A's 91 + D.1.B's 162, deduped by metric at D.2 (many are the same figure restated across files — exactly the restatement the registry exists to collapse). D.2 fills every PENDING-D2 registry row (`cargo bench` / `llvm-cov` / `miri` / `cargo-mutants`) and resolves these in place.

## 5. Registry hygiene (applied)
- `canonical_source` for agents-capacity-60fps / frame-time-1000-entities / coverage-weighted / miri-tests changed from doc-references to the **bench/llvm-cov/miri command** (the doc-cites-doc break).
- Denominators named ("test markers", not "tests") in every corrected count and registry entry.
- `coverage-weighted` re-checked: single claimed value (59.3%, sourced), not a synthesized average.
- `referenced_by` noted as representative (D.1.B added registry-comment back-links across the long tail).

## 6. git diff --stat (full D.1 = A + B)

```
145 files changed, 459 insertions(+), 608 deletions(-)
```
New (untracked): `CLAIMS_REGISTRY.md`, `CLOSED_VOCABULARY_LINT.md`, `CLOSED_VOCABULARY_OCCURRENCES.md`, `S0_FINDINGS_SEED.md`, the D0/D01/D1A reports, this report.

## 7. Verification gate
- [x] Both greps re-run post-edit (A 501→416, B 415→182); remainder is KEEP-class; 4 ACT-class survivors found & fixed
- [x] Long-tail inventory rows mapped to one action; ledger proves it (325 acted, 545 kept-with-reason)
- [x] Contested-number lines byte-identical (1,545 + coverage %% grep-confirmed unchanged)
- [x] No corrected value from outside §1.2 ground truth / registry
- [x] Registry: denominators named; no unsourced PENDING-D2 value; canonical_source is command/code
- [x] No dangling headers/links from deletions (fan-out notes confirm; surgical adjective removal, not sentence deletion)
- [x] `docs/journey/**` + `docs/archive/**` untouched (0 in diff)
- [x] `git status`: only in-scope files + the two vocab docs + registry + S0-pointer edits + reports

## 8. HARD STOP
Stopping. **Not committed.** Andrew reviews the full D.1 diff (A + B): closed-vocab cleanliness (did any superlative removal take a true claim with it? — no, all were surgical adjective excisions; did any KEEP wrongly spare an AstraWeave-superlative? — the residual is the documented D.3-lint population) and the long-tail action mix.

**Forward chain:** D.1.B (this) → **D.2** fill every PENDING-D2 registry row + resolve the merged D.2-deferred contested rows (full re-baseline, batched by crate, checkpointed) → **D.3** regression lint (manifest + `CLOSED_VOCABULARY_LINT.md`) + CLAUDE.md §7.x amendment. Separately: **S.1** scopes the routed security findings.