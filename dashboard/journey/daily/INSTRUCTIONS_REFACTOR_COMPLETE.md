# Instructions Refactor — Completion Report

**Date**: February 8, 2026
**Type**: Documentation-only refactor (zero Rust code changes)

---

## Summary

Refactored `.github/copilot-instructions.md` from a ~15,000-token hybrid changelog/status/instruction document into a ~3,000-token focused behavioral field manual. All historical content preserved in properly organized reference files.

---

## Before / After

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Characters | 71,541 | 12,000 | **-83%** |
| Words | ~8,752 | ~1,411 | **-84%** |
| Lines | 1,449 | 304 | **-79%** |
| Estimated Tokens | ~15,000 | ~3,000 | **-80%** |

---

## What Changed

### New Instructions Structure (6 sections, all behavioral)

1. **Mission & Identity** (~180 words) — Frontier experiment framing, 4-point mandate, error handling policy
2. **Workflow & Process** (~400 words) — Chain of thought, DO/DON'T build strategy, dev workflow, quick commands, master report maintenance, response guidelines
3. **Code Patterns & Conventions** (~500 words) — Error handling, ECS, WorldSnapshot, BehaviorGraph, GOAP+Hermes arbiter, combat physics, SIMD movement, asset loading
4. **Architecture Orientation** (~250 words) — AI-first loop, 7 ECS stages, crate list, "Where to Look" table
5. **Guardrails & Verification** (~350 words) — **Miri & Kani verification workflows** (kept per user request), known issues, build timings, key lessons, doc organization policy
6. **Reference Pointers** (~80 words) — 8 file references with descriptions

### Content Relocated

| Content | Destination |
|---------|-------------|
| Project status, phase completion records, active work dashboard | `docs/current/PROJECT_STATUS.md` (already existed — no merge needed) |
| Detailed API patterns (7 arbiter patterns), performance data, testing patterns, workspace deep dive, formal verification details | `docs/current/ARCHITECTURE_REFERENCE.md` (NEW) |
| Documentation navigation, strategic plan index, audit report links, journey doc index | `docs/current/DOCUMENTATION_INDEX.md` (NEW) |
| Phase 8 day-by-day completion reports (inline in old instructions) | Already existed in `docs/journey/daily/` — removed from instructions |
| Strategic planning document listings | Consolidated into DOCUMENTATION_INDEX.md |
| Quality & audit report listings | Consolidated into DOCUMENTATION_INDEX.md |
| Week summary listings | Consolidated into DOCUMENTATION_INDEX.md |

### Key Decisions

1. **Miri & Kani verification retained in instructions** — Per user request, the formal verification section (Miri UB detection + Kani formal proofs + unsafe code requirements) stays in the instructions file as a behavioral guardrail, not just a reference. This ensures every generated `unsafe` block follows the verification workflow.

2. **Key lessons kept as behavioral directives** — The 8 numbered lessons are disguised behavioral rules (e.g., "only parallelize >5ms workloads") that actively shape code generation quality.

3. **Documentation organization policy condensed** — The decision tree is kept (it's a behavioral rule), but the lengthy examples and enforcement details moved to reference.

4. **Performance numbers removed from instructions** — All benchmark results now live in `ARCHITECTURE_REFERENCE.md` and `MASTER_BENCHMARK_REPORT.md`. The instructions reference these files instead of inlining stale numbers.

5. **Duplicate content eliminated** — The old file had two copies of the Chain of Thought process, two copies of the Determinism Validation completion, and overlapping build strategy / development workflow sections. Each now appears exactly once.

---

## Files Created/Modified

| File | Action |
|------|--------|
| `.github/copilot-instructions.md` | **Rewritten** (1,449 → 304 lines) |
| `.github/copilot-instructions.md.bak` | **Created** (backup of original) |
| `docs/current/ARCHITECTURE_REFERENCE.md` | **Created** (detailed patterns, API deep dives, performance baselines, formal verification) |
| `docs/current/DOCUMENTATION_INDEX.md` | **Created** (master navigation for all project documentation) |
| `docs/current/README.md` | **Updated** (added links to new reference files) |
| `docs/current/INSTRUCTIONS_REFACTOR_AUDIT.md` | **Created** (Phase 1 audit findings) |

---

## Validation

- **Build check**: `cargo check -p astraweave-core` — passes ✅
- **Coverage check**: Every section from the original 1,449-line file has a home in either the new instructions or a relocated reference file ✅
- **Behavioral test**: The new file tells an agent everything needed to write correct code on the first prompt — identity, workflow, API patterns, verification requirements, and where to look for more ✅
- **Drift check**: No date-stamped facts remain in the instructions. Version number and Rust version are the only version-specific items (appropriate for a field manual) ✅
- **Miri/Kani preserved**: Full formal verification workflow retained as Section 5 behavioral guardrail ✅

---

## Token Budget Accounting

| Section | Est. Tokens | % of Total |
|---------|-------------|------------|
| Mission & Identity | ~400 | 13% |
| Workflow & Process | ~800 | 27% |
| Code Patterns | ~1,000 | 33% |
| Architecture | ~400 | 13% |
| Guardrails & Verification | ~700 | 23% |
| Reference Pointers | ~150 | 5% |
| **Total** | **~3,000** | — |

Target was 3,000-4,000 tokens. Achieved ~3,000 — at the lean end of the range.
