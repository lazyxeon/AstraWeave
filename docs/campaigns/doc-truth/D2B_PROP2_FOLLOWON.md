# D.2.B-Prop-2 — Corpus Propagation (non-enumerated files) · RATIFIED FOLLOW-ON

**Status:** RATIFIED, **NOT STARTED.** · **Branch (when run):** off `campaign/doc-truth-resume` · **Authority:** the Session-A measured registry values (`CLAIMS_REGISTRY.md`) + the D.2.B-Prop classify-then-edit discipline.

## Why this exists
D.2.B-Prop corrected the **8 enumerated files**. A comprehensive grep during that beat found the same two Session-A claims in **~10 non-enumerated files** — the contamination is ~3–4× the original enumeration. This follow-on completes the propagation. It is **classify-then-edit**, not find-and-replace: correct present-tense claims; leave dated records, labeled targets, and false positives byte-identical.

## The two claims to propagate (already measured + landed in the registry)
- **miri**: `977` → **`1,059`** (per-crate: ecs 419 / math 109 / core 503 / sdk 28). See `CLAIMS_REGISTRY.md#miri-tests`.
- **frame-time**: `2.70 ms` was the **Week-8 target**, not a measurement → measured **System ~0.97 ms (1,036 FPS) / mimalloc ~0.71 ms (1,410 FPS)**. See `CLAIMS_REGISTRY.md#frame-time-1000-entities`.

## Per-site classification (the starting proposal — D.2.B-Prop-2 verifies + executes)

| File | CORRECT (present-tense) | LEAVE (rationale) |
|---|---|---|
| docs/current/PROJECT_STATUS.md | :260, :275, :179 (miri/frame present-tense status) | :230 `3.09→2.70 −12.6%` (dated optimization record) |
| docs/current/MASTER_COVERAGE_REPORT.md | :16 (present-tense headline) | :249 (the report's own measurement table), :550 (revision-history "v3.2.0 2026-02-03") |
| docs/current/ARCHITECTURE_REFERENCE.md | :557 (miri present-tense) | :546 (`2.70 ms … Week 8` — **labeled target column**) |
| docs/current/KANI_VERIFICATION_PLAN.md | :477 (miri present-tense) | — |
| docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md | :530 (miri present-tense) | — |
| docs/current/PERFORMANCE_BUDGET_ANALYSIS.md | :292 (verify context — "after optimization 2.70 ms" may be analysis/dated; classify at run) | (lean: analysis figure — verify) |
| docs/masters/MASTER_ROADMAP.md | :35, :100, :551 (miri); :42, :248, :263 (frame p95) | :383 (Feb-03 dated row), :446 (timeline Feb-03) |
| docs/masters/MASTER_BENCHMARK_REPORT.md | :15, :174, :180 (frame present-tense) | :870 (`3.09→2.70` dated optimization); Qwen `2,707 ms` streaming = **false positives** |
| gh-pages/setup.md | :122 (miri comment) | — |
| docs/src/resources/roadmap.md | :61 (miri present-tense) | — |
| gh-pages/ai.md, docs/src/core-systems/ai/index.md | — | `12,700+ agents` only (not a 977/2.70 site; out of scope) |
| docs/masters/MASTER_BENCHMARK_REPORT.md**.bak** | — | `.bak` backup file — **not live corpus**, leave |

## Protocol notes (load-bearing for the run)
- **Master Report Maintenance** (CLAUDE.md): correcting `MASTER_BENCHMARK_REPORT.md` and `MASTER_ROADMAP.md` triggers the version-bump + revision-history protocol — handle properly, NOT as a plain propagation.
- **Known leaves to preserve** (do not "correct"): PROJECT_STATUS.md:230 (dated optimization), ARCHITECTURE_REFERENCE.md:546 (labeled Week-8 target), the Qwen `2,707 ms` streaming false-positives, the `.bak` file.
- **Coverage 59.3%** stays untouched until Path B re-baselines it (a separate future propagation corrects coverage references).
- **Other false positives to skip** (2.70 not frame-time): Vec3 lerp `2.70 ns`, deserialize `2.70 ms` (.bak), `num_batch 2700ms`.

## Out of scope here
- Path B (broken test-target fixes — source work).
- The coverage propagation (post-Path-B).
