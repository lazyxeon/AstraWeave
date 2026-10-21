# D.3.0 — Regression Lint: Spec Update + Mechanism Plan (recon-and-propose)

**Campaign:** D-series · **Phase:** D.3.0 (final beat's recon) · **Mode:** READ-ONLY RECON + PROPOSAL
**Branch:** `campaign/doc-truth-resume` · **HEAD at recon:** `135e9915b` (clean, pushed, == upstream)
**Status:** PROPOSED — awaiting director ratification. No lint built, no CI wired, no spec mutated, no KANI correction made.

> **Authority consulted (all read this pass):** `CLOSED_VOCABULARY_LINT.md` (the spec, rev 0.1), `CLOSED_VOCABULARY_ALLOWLIST.md` (266 keys / 587 KEEP), `CLOSED_VOCABULARY_OCCURRENCES.md` (evidence list), `CLAIMS_REGISTRY.md` (rev 0.5 — fully settled), `CLAIMS_ARCHAEOLOGY_FRAMEWORK.md` (the D.2/D.3 plan), `D2A_EXECUTION_REPORT.md` §8 (PENDING-D2.B manifest), `D2B_PROP2_FOLLOWON.md`, `.github/workflows/trace-sync.yml` (the existing CI gate precedent).

---

## 0. Framing notes (two facts that shape everything below)

1. **The spec predates the measurements.** `CLOSED_VOCABULARY_LINT.md` was seeded at D.1.B (2026-06-13). At that moment coverage / miri / frame-time were `PENDING-D2` in the registry — *unmeasured*, so they could not be poison-listed (a value you haven't measured can't have a "superseded" string). Vocab A therefore contains **none** of `59.3` / `94.57` / `977` / `2.70 ms` / `370 FPS`. D.2.B (`agents-capacity`, `frame-time`, `validation`, `miri`) + Path-B.2 (`coverage`) measured them; the *act of measuring* is what minted the new poison generation. This is the gap D.3.0 closes.

2. **"Phase renumbering is local."** The framework (`CLAIMS_ARCHAEOLOGY_FRAMEWORK.md` §D.2) calls this the "D.2 Regression Guard"; the spec + campaign call it **D.3**. Same beat. The framework also floats a *broader* lint — bare-number pattern matching (`\d+[km]?\s*(entities|agents|tests|FPS|…)`) + status-emoji-without-trace-citation. The spec deliberately **narrowed to the closed sets only** ("`git grep` is exhaustive where sampling is lossy"). This recon **honors that narrowing** — the open-ended bare-number lint is a high-false-positive beast and is NOT proposed for D.3 (noted as a possible future enhancement only).

---

## Deliverable 1 — Spec-update proposal (the new poison generation)

All evidence below is the **linted surface only** (excludes `docs/journey/**`, `docs/archive/**`, `docs/campaigns/doc-truth/**`, `.github/copilot-instructions-old-backup.md`). Verified by `git grep` at HEAD `135e9915b` (post-D.2.B-Prop-Final).

### 1a. Coverage — `59.3` / `59.3%` → **ADD to Vocab A**

**Replacement truth:** `57.35%` whole-workspace line (`CLAIMS_REGISTRY.md#coverage-weighted`, VERIFIED-AT-HEAD 2026-06-29).

Post-propagation, **zero bare present-tense survivors.** Every linted-surface `59.3` is now either **supersession-context** (the correction itself cites the superseded value) or a **dated version-comparison / revision row**:

| File:line | Class | Disposition |
|---|---|---|
| `README.md:57,79,271` | "supersedes the prior 29-crate-subset 59.3%" | LEAVE → **allowlist** (supersession-context) |
| `docs/masters/MASTER_ROADMAP.md:40,242` | "supersedes the 59.3% … subset" | LEAVE → **allowlist** (supersession-context) |
| `docs/masters/MASTER_ROADMAP.md:379` | revision row "(was 59.3% subset)" | LEAVE → **allowlist** (dated revision) |
| `docs/masters/MASTER_ROADMAP.md:474` | dated v5.0 revision ("59.3% weighted, 14 crates …"; also `94.71%`/`78%`) | LEAVE → **allowlist** (dated revision) |
| `docs/current/MASTER_COVERAGE_REPORT.md:14,28` | reframed headline + note (supersession) | LEAVE → **allowlist** (supersession-context) |
| `docs/current/MASTER_COVERAGE_REPORT.md:463` | version-comparison table (`94.57% \| 59.3% \| 59.3%*`) | LEAVE → **allowlist** (version-comparison) |

**Net:** add `59.3`/`59.3%` to Vocab A; add ~7 allowlist keys (supersession/dated). **No present-tense correction needed** — the final propagation already cleared them. Clean closed-set addition (nobody's coverage *target* is 59.3%, so the string never doubles as a legitimate target — unlike frame-time).

### 1b. Coverage — `94.57` / `94.57%` → **ADD to Vocab A** + **1 present-tense correction**

**Replacement truth:** same — `57.35%` (the 94.57% was the v4.2.0 figure, debunked by the v5.0.0 audit; `MASTER_COVERAGE_REPORT.md:32`).

| File:line | Class | Disposition |
|---|---|---|
| **`docs/current/KANI_VERIFICATION_PLAN.md:479`** | **"✅ Tests: 3,040+ passing tests (94.57% coverage)"** — **present-tense** | **CORRECT in D.3.1** (the director-flagged deferred item) |
| `docs/current/MASTER_COVERAGE_REPORT.md:32` | "The v4.2.0 report … claimed 94.57% … the v5.0.0 audit revealed discrepancies" | LEAVE → **allowlist** (debunking-context) |
| `docs/current/MASTER_COVERAGE_REPORT.md:463` | version-comparison table | LEAVE → **allowlist** (version-comparison) |
| *all `docs/journey/**` hits (BULLETPROOF_VALIDATION_COMPLETE, PHASE_10_*)* | excluded path | not linted |

**Net:** add `94.57`/`94.57%` to Vocab A; **correct KANI:479** to registry-framed truth in D.3.1 (this is where the deferred KANI correction lands — note line drifted from `:476`→`:479` after the final-pass miri correction at `:477`); add 2 allowlist keys. **Establishing the clean baseline requires this one correction.**

### 1c. Miri — `977` → **ADD to Vocab A** (with the most allowlisting)

**Replacement truth:** `1,059` miri tests (`CLAIMS_REGISTRY.md#miri-tests`, VERIFIED-AT-HEAD 2026-06-27; ecs 419 / core 503 / sdk 28 / math 109).

**Director's question — "can 977 be poison-listed without false-flagging the dated/quoting surfaces?" Honest answer: YES, but NOT for free.** The exclusions (journey/archive/doc-truth) do **not** protect the dated 977 surfaces, because they live in `docs/current/` and `docs/masters/` (linted paths). Post-propagation there are **zero present-tense survivors**, but `977` still appears legitimately in **~13 dated / report-description sites** that each need an allowlist key:

| File:line | Class | Disposition |
|---|---|---|
| `docs/current/MIRI_VALIDATION_REPORT.md:20,255` | **the dated source-of-record** (TOTAL 977) | LEAVE → **allowlist** (dated-record; this is the canonical home of 977) |
| `docs/masters/MASTER_ROADMAP.md:59,384,447,552` | dated Feb-3 entry / v1.45 revision row / timeline / `<details>v1.45</details>` block | LEAVE → **allowlist** (dated) |
| `docs/current/MASTER_COVERAGE_REPORT.md:249,551` | dated Miri table + v3.2.0 revision row | LEAVE → **allowlist** (dated) |
| `docs/current/PROJECT_STATUS.md:179` | dated Feb-3 block (the over-correction reverted in the final pass) | LEAVE → **allowlist** (dated) |
| `docs/current/DOCUMENTATION_INDEX.md:52` · `.github/copilot-instructions.md:303` · `CLAUDE.md:455` · `docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md:591` | rows **describing** the dated MIRI report ("Miri validation details (977 tests, 0 UB)") | LEAVE → **allowlist** (report-description) |

> **`CLAUDE.md:455` note:** this is a report-description row, NOT inside the trace-sync-gated TRACE-TABLE block (lines 71–100) — so allowlisting it is correct and editing-it-is-never-needed; the gated block is untouched either way.

**Net:** add `977` to Vocab A; add ~13 allowlist keys (dated-record / report-description). This is the heaviest allowlisting of the three clean additions — flagged so the director knows 977 is poison-listable but carries a one-time allowlist cost concentrated in the master reports + the dated MIRI report.

### 1d. Frame-time — `2.70 ms` / `370 FPS` / `84% headroom` → **DIRECTOR DECISION** (recommend: do NOT string-lint)

**Replacement truth:** System **0.965 ms** (~1,036 FPS) / mimalloc **0.709 ms** (~1,410 FPS); **"2.70 ms" was the Week-8 *target*** (`CLAIMS_REGISTRY.md#frame-time-1000-entities`).

This is the value the director correctly flagged as "tricky for a string lint." The grep proves why — the **same string `2.70 ms` is simultaneously**:

1. **the sanctioned reframing** — "2.70 ms was the Week-8 target" (≈10 sites: README/render README, gh-pages ×3, docs/src/README, MASTER_ROADMAP ×3, MASTER_BENCHMARK ×3) — *must keep the string*;
2. **a labeled target** — `gh-pages/benchmarks.md:220` "2.70 ms (target — …)";
3. **a dated achievement record** — Week-8 sprint rows in `docs/lessons/**` + `docs/audits/**` (EXTERNAL_RESEARCH, COMPREHENSIVE_AUDIT, COMPETITIVE_MATRIX) — these are dated and many *read* present-tense ("AstraWeave 2.70ms @ 1k entities");
4. **a false positive** elsewhere (Vec3-lerp `2.70 ns`, deserialize `2.70 ms` in `.bak`, `num_batch 2700ms`);
5. **genuine present-tense poison** — `.zencoder/rules/repo.md:84` "Frame time (2.70 ms @ 1000 entities)" — **a real survivor the 17-file final propagation missed** (`.zencoder/rules/` was never enumerated). Plus borderline `docs/lessons/WHAT_DIDNT.md:21` ("Current: … 2.70 ms, 370 FPS") and `PERFORMANCE_PATTERNS.md:73`.

A pure string match cannot separate (1)/(2)/(3) from (5). The companion strings don't help: `370 FPS` and `84% headroom` *also* appear in dated records and in the sanctioned "370 FPS was the Week-8 target" reframing. **What a string lint can and cannot catch here, honestly:** it can catch a *bare reappearance* of the number, but it cannot tell achievement from target — so it would either (a) need ~30+ allowlist entries (most in dated docs → low signal, high maintenance) or (b) chronically false-positive, which erodes the warn→enforce credibility (the "cried wolf" failure that kills rollouts).

**Three options (director picks):**

- **Option A (RECOMMENDED).** Do **not** add frame-time to the string lint. Instead (i) in D.3.1, one-time-correct the present-tense survivors (`.zencoder/rules/repo.md:84`; classify-at-run `WHAT_DIDNT.md:21`, `PERFORMANCE_PATTERNS.md:73`) to registry-framed truth, and (ii) document in the spec a `## Frame-time carve-out` section stating *why* `2.70 ms` is human-review-only (it fails the closed-set test: the string is a legitimate target). Keeps the lint's signal-to-noise high; the corpus is still cleaned.
- **Option B (standing guard, more maintenance).** Same-line-context rule: flag `2.70 ms`/`2.7 ms`/`2.70ms` **only on lines lacking** `target` / `Week 8` / `Week-8`. Catches `repo.md:84` and future regressions; passes the reframed master-report rows; still needs ~12 allowlist keys for the dated audits/lessons that assert it present-tense without those words.
- **Option C (not recommended).** Full bare-string `2.70 ms` in Vocab A → 30+ allowlist keys. Signal/noise too poor for enforce-mode.

**Recommendation: A** (clean up + document the carve-out), with **B** available if the director wants a standing frame-time regression guard despite the audit-doc allowlisting cost.

### 1e. Cross-check of the existing Vocab A for newly-stale members

- `2,560` fluids tests (Vocab A line 18) — **still correctly poison** (now → `738` per W.1; the spec's inline note already records this). No change.
- **Sibling stale-coverage generations** found but NOT recommended for poison-listing: `94.71%` (`MASTER_ROADMAP:474`, single dated revision row), `97.39%` (`COMPETITIVE_ANALYSIS_SUMMARY:43`), `96.43%` (`COMPREHENSIVE_AUDIT:403`) — all single-occurrence, in dated audit/revision contexts, too generic as strings. **LEAVE** (dated); flagged for awareness only. Adding them buys ~3 allowlist keys and ~0 regression protection.
- **Optional add:** fluids LoC `84K` / `84.5K` (registry: now ~24.2K, "badly stale") — cited in `CLAUDE.md` (honest-dormancy-ish) + one audit. Low priority; out of the director's stated coverage+miri+frame-time scope. Noted, not proposed.

### 1f. New allowlist `reason` categories this generation introduces

Consistent with the existing taxonomy (`historical-dated`, `competitor-cited`, `combinatorial-dormant`, …): `supersession-context` (the correction cites the value it supersedes), `version-comparison` (a vN-vs-vM table), `debunking-context` (prose explaining the value was wrong), `dated-record` (the canonical dated source, e.g. MIRI_VALIDATION_REPORT), `report-description` (an index/reference row describing a dated report).

---

## Deliverable 2 — Lint mechanism proposal

### 2a. What runs it — **recommend a small Rust tool** (`tools/aw_doc_lint`), with a shell-script bootstrap option

| Option | Pros | Cons |
|---|---|---|
| **Rust tool** (mirrors `aw_trace_sync`) | robust markdown parsing of the vocab fenced-blocks + the 266-row allowlist table; **unit-testable** (satisfies the workspace's exhaustive-testing mandate — the guard itself must be tested); joins the existing tool+CI family; one binary, cross-platform | ~a day to build; compiles in CI (cache like trace-sync) |
| **Shell script** (`git grep`, the spec's stated approach) | fastest to land; matches spec line 3 framing; zero toolchain in CI | allowlist matching (file+match+reason, line-drift-tolerant) is fiddly in awk/sed; harder to test; Windows/macOS/Linux portability friction |

**Recommendation:** **Rust tool, spec-as-source.** The allowlist is 266 keyed entries and the vocab blocks carry inline `#`-comments and `|`-alternatives — robust parsing wants Rust, and the project's own lesson ("wired beats tested") argues the regression guard must itself be tested. If the director wants D.3.1 to land faster, a `git grep` shell script is an acceptable **warn-mode bootstrap** that the Rust tool later supersedes.

### 2b. How it consumes the spec — **spec-as-source (strongly preferred)**

The lint **reads** its vocabulary from `CLOSED_VOCABULARY_LINT.md` (Vocab A + Vocab B fenced blocks) and its false-positive set from `CLOSED_VOCABULARY_ALLOWLIST.md` (the `| File | Match | Reason | Lifetime |` table). **No duplication** — same single-source-of-truth pattern as `aw_trace_sync` reading trace front-matter. To make parsing deterministic, D.3.1 adds machine-readable markers around the blocks (mirroring the `TRACE-TABLE` markers), e.g. `<!-- VOCAB-A-START -->` … `<!-- VOCAB-A-END -->` and `<!-- ALLOWLIST-START -->` … `<!-- ALLOWLIST-END -->`. The parser: strip `#`-comments → split on `|` → trim → each token is a poison/superlative literal; allowlist rows key on `(file, match)` (reason is documentation).

### 2c. How it applies scope + exclusions + keep-vs-act

- **Scope set:** `git ls-files` → keep `*.md` + `gh-pages/**` (`.md`/`.html`/`_config.yml`) + `.zencoder/**/*.md` + `.github/*.md` + per-crate `README.md`.
- **Exclusions:** drop `docs/journey/**`, `docs/archive/**`, `docs/campaigns/doc-truth/**`, `.github/copilot-instructions-old-backup.md`. *(Spec-coherence flag — see §5: the spec includes `.zencoder/**/*.md` in scope, but `D2A_EXECUTION_REPORT.md:103` says `.zencoder/chats/` is excluded; the allowlist resolves it by blessing the chat entries individually. The tool should follow the **spec** (chats in scope, blessed via allowlist) and the director should confirm.)*
- **Keep-vs-act (rule 4):** a match is suppressed iff `(file, match)` is in the allowlist. Rules 1–3 (the ACT dispositions) are human work done once in D.3.1 to reach the clean baseline; the standing lint only enforces "no *new* un-allowlisted occurrence."

### 2d. Output format

```
<file>:<line>: [VOCAB-A|VOCAB-B] "<matched string>"  (not in allowlist)
```
- **warn-mode:** print + emit GitHub `::warning file=…,line=…::`, **always exit 0**.
- **enforce-mode:** same, but emit `::error::` and **exit 1** if any un-allowlisted hit.
- Summary footer: `N files scanned · M matches · K un-allowlisted`. A clean baseline = `K=0` (the objective terminal condition, exactly as the framework wants).

---

## Deliverable 3 — CI integration proposal

### 3a. New workflow `doc-truth-lint.yml` (separate from `trace-sync.yml`)

**Separate workflow, not a job in trace-sync.** They guard **disjoint surfaces** with disjoint triggers:

| | `trace-sync.yml` (exists) | `doc-truth-lint.yml` (proposed) |
|---|---|---|
| Surface | CLAUDE.md TRACE-TABLE block + `workspace_map.html` (cargo-derived) | prose poison/superlative strings across all in-scope `.md`/`.html` |
| Source of truth | trace front-matter | `CLOSED_VOCABULARY_LINT.md` + `…_ALLOWLIST.md` |
| Trigger paths | `docs/architecture/**`, `**/Cargo.toml`, `CLAUDE.md`, tool, workflow | the in-scope prose globs + spec + allowlist + lint tool + workflow |

**No overlap.** Subtlety: both *touch* `CLAUDE.md` — trace-sync owns its TRACE-TABLE block; doc-lint scans its prose for poison (already allowlisted: `check-all`, `QUIC`, `qwen3:8b`, `99.96%`, `production-grade`). Different concerns, same file, no conflict — and doc-lint must not treat the TRACE-TABLE block specially (its rows are descriptive, none are poison/superlative).

### 3b. warn → enforce rollout

- **Implemented by a `--mode` flag** on the tool: `warn` (always exit 0 + `::warning::`) vs `enforce` (exit 1 on un-allowlisted + `::error::`). The workflow passes `--mode=warn` at D.3.2 landing.
- **Flip trigger (gated, not purely calendar):** the spec says "warn-only for two weeks, then enforce." Concretely — after a **≥2-week soak** on `main` (≈2026-07-13 if D.3.2 lands ~2026-06-29) **with zero un-allowlisted hits**, flip `--mode=warn`→`enforce` in a one-line PR (D.3.3). If the soak surfaces a real hit, fix-or-allowlist first, *then* flip. The gate is "soak observed clean," with the two weeks as the floor.
- Toolchain: if Rust, pin `1.89.0` + `Swatinem/rust-cache` like trace-sync; if shell, just checkout + run (no toolchain step).

### 3c. Complementarity confirmed

Prose-string surface (doc-lint) ∩ trace-table/map surface (trace-sync) = ∅. The two guards are orthogonal; D.3 adds the prose guard the campaign was built to install.

---

## Deliverable 4 — Registry self-verification lint: **DEFER (build cheap-rows-only, separately)**

A *second, distinct* idea: run each `CLAIMS_REGISTRY.md` row's `repro` and compare to the recorded `value`, catching the broken-repro class (two such meta-defects occurred this campaign: the `**`-glob returning 0 in `fluids-loc`; the machine-dependent `xargs … wc -l | tail -1` batching in `rust-loc-total`).

**The cost asymmetry is the whole decision.** The vocab lint is near-free `git grep` (seconds). Self-verification *re-executes measurements* — and the registry's ~25 rows split sharply:

| Tier | Rows | Repro cost |
|---|---|---|
| **Cheap** (counts / LoC / version / markers — machine-independent, CI-friendly) | workspace-members, production-crates, test-markers-total, editor-test-markers, fluids-test-markers, fluids-loc, water-facade-loc, water-surface-loc, rust-loc-total, kani-proofs, toolchain, dependency-versions, ai-modes, dormant-loc-inventory (~14 rows) | `cargo metadata` / `git grep` / `tokei` / `wc` — **seconds–<1 min total** |
| **Expensive** (perf / coverage / miri / mutants — hardware-stamped, machine-specific) | agents-capacity-60fps, frame-time-1000-entities, validation-checks-per-sec, coverage-weighted, miri-tests, mutation-kill-rate, water-budget (~7 rows) | bench / `llvm-cov --workspace` (the multi-hour + Windows-cmdline-limit beast) / miri (~9 min) / mutants (**6–15 h**, already director-deferred) / GPU-timestamp |

**Recommendation:**
1. **Do NOT bundle self-verification into D.3's vocab-lint beat.** Different mechanism (repro-runner vs grep), and bundling delays the campaign's stated D.3 deliverable.
2. If built at all, **scope it to the ~14 cheap rows only** — a "registry counts re-verifier" runnable in CI in <1 min, machine-independent. **This catches exactly the meta-defect class that actually occurred** (both broken repros were cheap-row repros), which is the strongest argument for it. The expensive rows stay **manually** re-verified (their repros are documented; re-run when a subsystem campaign touches them — the framework's "trace-currency hook"). Running expensive rows as a lint is prohibitive (hours) and would be CI-flaky (hardware-dependent values).
3. Sequence it as a **separate small follow-on** (D.3.4 or post-D.3), director's call — **not** a blocker for the vocab lint.

---

## Deliverable 5 — D.3 sub-phasing proposal

| Sub-phase | Work | Mutation class | Gate |
|---|---|---|---|
| **D.3.0** (this) | recon + 5-deliverable proposal | none (read-only) | director ratifies the decisions below |
| **D.3.1** | (a) spec update — add `59.3`/`94.57`/`977` to Vocab A + the frame-time decision (carve-out §, or Option B rule) + machine-readable `VOCAB-*`/`ALLOWLIST` markers; (b) allowlist additions (~7 coverage + 2 + ~13 miri + frame-time-per-option); (c) **one-time present-tense cleanup to clean baseline** — `KANI:479` (94.57), `.zencoder/rules/repo.md:84` (frame-time), classify-at-run `WHAT_DIDNT:21`/`PERFORMANCE_PATTERNS:73` | spec + corpus (docs-only); master-report protocol if any master report is corrected | HARD-STOP commit |
| **D.3.2** | build the lint (Rust tool or script, spec-as-source) + run locally → confirm `K=0` un-allowlisted (baseline clean = terminal condition) + wire `doc-truth-lint.yml` in **warn-mode** | new tool/script + CI file (source + workflow) | HARD-STOP commit |
| **D.3.3** | after ≥2-week clean soak: flip warn→enforce (one line) + add CLAUDE.md §7.x key lesson ("Prose is not evidence — every number/status links the registry or carries a repro") | one-line CI flag + CLAUDE.md prose (outside the gated TRACE-TABLE block) | HARD-STOP commit |
| **D.3.4** *(optional, director's call)* | registry **cheap-row** self-verifier (Deliverable 4) | new tool + CI | separate beat |

**Could it be one beat?** No — D.3.1 must land + be reviewed *before* D.3.2, because the lint's "`K=0`" check is only meaningful over a clean baseline; and warn→enforce is inherently time-separated (the soak). Genuinely ≥3 beats.

---

## Director decisions to ratify (the gate)

1. **Frame-time poison handling** *(headline)* — **Option A** (one-time cleanup + document the human-review carve-out; recommended), **B** (same-line-context standing guard, ~12 audit allowlist keys), or **C** (full string-lint, not recommended)?
2. **Coverage + miri additions** — ratify adding `59.3`/`59.3%`, `94.57`/`94.57%`, `977` to Vocab A, with the ~22 allowlist keys (supersession / version-comparison / debunking / dated-record / report-description) and the **two present-tense corrections** (`KANI:479` 94.57; `repo.md:84` frame-time) folded into D.3.1's clean-baseline step. Confirm: **977 is poison-listable but carries ~13 dated/report-description allowlist keys** (the exclusions do not cover the dated MIRI report or the master-report dated rows).
3. **Lint mechanism** — Rust tool `aw_doc_lint` (recommended, spec-as-source, unit-tested) vs `git grep` shell-script bootstrap?
4. **CI shape** — new `doc-truth-lint.yml` (recommended) confirmed complementary to `trace-sync.yml`; warn→enforce via `--mode` flag, flip gated on a clean ≥2-week soak.
5. **Registry self-verification lint** — **defer**, and if built, **cheap-rows-only** as a separate D.3.4 (recommended)? Or in-scope now?
6. **Sub-phasing** — D.3.1 (spec+baseline) / D.3.2 (build+warn) / D.3.3 (enforce+CLAUDE.md §7.x) / D.3.4 (optional self-verifier) as laid out?
7. **Spec-coherence nit** — `.zencoder/chats/**`: spec says in-scope (blessed via allowlist) vs `D2A:103` says excluded. Follow the spec (in-scope) and keep the chat allowlist keys?

---

## Hard-constraint compliance

- ✅ **READ-ONLY** — no lint built, no CI file written, no spec mutated, no KANI correction made. This recon doc lives in the lint-**excluded** `docs/campaigns/doc-truth/` (the campaign's own audit trail) — not a spec/CI/source surface.
- ✅ Every claim carries a `file:line` reference (verified by `git grep` at HEAD `135e9915b`).
- ✅ Honors the existing spec's decisions (scope, exclusions, posture, closed-set narrowing); the one spec-coherence nit is **flagged, not overridden** (§Director decision 7).
- ✅ HEAD `135e9915b`, tree clean, == upstream.

**Gate:** stop here. Director ratifies decisions 1–7 before D.3.1 builds/mutates anything.
