# Instructions Refactor Audit

**Date**: February 8, 2026  
**File**: `.github/copilot-instructions.md`  
**Total Lines**: 1,449  
**Estimated Tokens**: ~15,000-17,000

---

## Section-by-Section Classification

### 1. Frontier Experiment: Mission Critical (Lines 1-17)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~17
- **Notes**: Core identity framing + 4-point mandate. High-signal, front-loaded. Keep and tighten.

### 2. Current State (Lines 19-148 + 153-399)
- **Bucket**: RELOCATE-STATUS
- **Lines**: ~375
- **Notes**: Massive changelog/dashboard. Contains:
  - Miri validation status (Feb 2026)
  - Phase 8.8 Physics Robustness status
  - Fluids System completion
  - WGPU 0.25 migration
  - Session summaries (Nov 18, 2025)
  - Phase 8.7 LLM Testing completion
  - Phase 8.6 UI Testing completion
  - Determinism Validation (DUPLICATE — appears twice, lines 79-83 and 88-101)
  - Phase B Month 4 Integration (DUPLICATE — appears twice, lines 84-86 and 103-114)
  - Phase 7 LLM Validation
  - Phase 6 Real LLM Integration
  - Week 8 Performance Sprint
  - AI-Native Validation
  - Week 3 Testing Sprint (with day-by-day sub-entries)
  - Phase 8 Game Engine Readiness (with day-by-day sub-entries for 4 weeks)
  - Phase 8.6, 8.7, 8.8 status blocks
  - Phase 9.2 future planning
  - Astract Gizmo Sprint (Days 9-13)
  - Doc file listing for every single completion report (~40 file references)
  - **ALL of this is project status/history, not behavioral instructions**

### 3. Master Report Maintenance Protocol (Lines ~400-455)
- **Bucket**: KEEP-INSTRUCTIONS (condense)
- **Lines**: ~55
- **Notes**: Behavioral rule — when/how to update master reports. Keep trigger conditions and enforcement rules. Cut the step-by-step "Update process" details (agent can read the files themselves). Cut the PowerShell verification command.

### 4. Your Role (Lines ~456-475)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~20
- **Notes**: Core principles (AI-Generated Only, Iterative Excellence, Security & Performance, Documentation). Tight, keep as-is.

### 5. Chain of Thought Process — SHORT version (Lines ~476-483)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~8
- **Notes**: 6-step CoT. Clean and behavioral. DUPLICATE — a longer version follows.

### 6. Error Handling Policy (Lines ~484-488)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~5
- **Notes**: Zero tolerance policy. Keep.

### 7. Chain of Thought Process — LONG version (Lines ~490-520)
- **Bucket**: DUPLICATE of #5
- **Lines**: ~30
- **Notes**: More detailed CoT. Keep ONE version — merge the best of both.

### 8. Response Guidelines (Lines ~521-530)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~10
- **Notes**: Output format, edge cases, experiment mindset, error handling. Keep.

### 9. Quick Commands (Lines ~534-555)
- **Bucket**: KEEP-REFERENCE (trim)
- **Lines**: ~22
- **Notes**: Setup, test, bench commands. High-value, compact. Keep commands, relocate Performance Summary.

### 10. Performance Summary (Lines ~556-572)
- **Bucket**: RELOCATE-STATUS → ARCHITECTURE_REFERENCE.md
- **Lines**: ~17
- **Notes**: Benchmark numbers that go stale. Relocate entirely.

### 11. Key Cargo Aliases (Lines ~574-580)
- **Bucket**: KEEP-REFERENCE (trim)
- **Lines**: ~7
- **Notes**: Compact and useful. Keep.

### 12. Architecture Essentials (Lines ~584-655)
- **Bucket**: KEEP-REFERENCE (partial)
- **Lines**: ~70
- **Notes**: Contains:
  - AI-First Loop diagram: KEEP (core conceptual anchor)
  - Key Concepts (4 bullets): KEEP
  - ECS System Stages (8-stage list): KEEP
  - Rendering & Materials (~25 lines): RELOCATE detailed parts, keep 2-line summary
  - Performance Optimization — Week 8 (~20 lines): RELOCATE to reference

### 13. Workspace Structure (Lines ~656-690)
- **Bucket**: KEEP-REFERENCE
- **Lines**: ~35
- **Notes**: Crate list with descriptions, examples status. Keep but trim sub-bullets.

### 14. Strategic Planning Documents (Lines ~694-780)
- **Bucket**: RELOCATE-INDEX
- **Lines**: ~85
- **Notes**: 15 numbered doc references with multi-line summaries. This is a documentation index, not instructions. Relocate entirely.

### 15. Week Summaries list (Lines ~782-795)
- **Bucket**: RELOCATE-INDEX
- **Lines**: ~14
- **Notes**: Historical week summary file listings.

### 16. Key Metrics Documents (Lines ~797-805)
- **Bucket**: RELOCATE-INDEX
- **Lines**: ~9

### 17. Automation Scripts (Lines ~807-812)
- **Bucket**: RELOCATE-INDEX
- **Lines**: ~6

### 18. Quality & Audit Reports (Lines ~816-895)
- **Bucket**: RELOCATE-INDEX
- **Lines**: ~80
- **Notes**: 10 numbered audit report references with summaries. Pure index content.

### 19. Working Effectively — Build Strategy (Lines ~900-930)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~30
- **Notes**: DO/DON'T lists. Excellent behavioral content.

### 20. Development Workflow (Lines ~932-945)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~14
- **Notes**: 7-step numbered workflow. Keep as-is.

### 21. Key Files to Check (Lines ~947-960)
- **Bucket**: KEEP-REFERENCE (trim)
- **Lines**: ~14
- **Notes**: Useful but several references are to Phase 6/7 docs — those are stale pointers.

### 22. Common Patterns & Conventions (Lines ~964-1000)
- **Bucket**: KEEP-REFERENCE
- **Lines**: ~36
- **Notes**: Error handling, ECS, system registration, combat physics, asset loading, SIMD. Compact and essential.

### 23. WorldSnapshot API (Lines ~1000-1030)
- **Bucket**: KEEP-REFERENCE (critical)
- **Lines**: ~30
- **Notes**: Prevents API misuse. Must stay.

### 24. BehaviorGraph API (Lines ~1032-1055)
- **Bucket**: KEEP-REFERENCE (critical)
- **Lines**: ~23
- **Notes**: Prevents API misuse. Must stay.

### 25. GOAP+Hermes Hybrid Arbiter (Lines ~1057-1170)
- **Bucket**: KEEP-REFERENCE (condense) + RELOCATE deep patterns
- **Lines**: ~113
- **Notes**: 7 patterns, performance characteristics, testing patterns. Keep Pattern 1 (basic usage) + Pattern 2 (shared executor). Relocate Patterns 3-7, all perf numbers, and testing patterns to ARCHITECTURE_REFERENCE.md.

### 26. Critical Warnings (Lines ~1175-1210)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~35
- **Notes**: Known issues, error handling policy (DUPLICATE of #6), build timings, performance baselines, validation. Keep warnings, cut duplicates, relocate validation achievements.

### 27. Where to Look (Lines ~1215-1255)
- **Bucket**: KEEP-REFERENCE
- **Lines**: ~40
- **Notes**: File path quick reference. Very high signal density. Keep as-is but cut the "Strategic Plans" sub-list (relocate to index).

### 28. Next Steps — Phase 8 (Lines ~1260-1345)
- **Bucket**: RELOCATE-STATUS
- **Lines**: ~85
- **Notes**: Phase 8 roadmap summary. Entirely status/planning content. Relocate.

### 29. Key Lessons Learned (Lines ~1347-1380)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~33
- **Notes**: 13 numbered lessons. Behavioral directives disguised as history. Keep all, trim parenthetical details.

### 30. Documentation Organization Policy (Lines ~1382-1445)
- **Bucket**: KEEP-INSTRUCTIONS (condense)
- **Lines**: ~63
- **Notes**: Decision tree is behavioral. Keep the tree and enforcement rules. Cut the verbose examples and "Why This Matters" section. Cut the example workflow (agent knows how to create files).

### 31. Footer (Lines ~1447-1449)
- **Bucket**: KEEP-INSTRUCTIONS
- **Lines**: ~3

---

## Summary by Bucket

| Bucket | Estimated Lines | % of Original |
|--------|----------------|---------------|
| KEEP-INSTRUCTIONS | ~290 | 20% |
| KEEP-REFERENCE | ~235 | 16% |
| RELOCATE-STATUS | ~460 | 32% |
| RELOCATE-INDEX | ~194 | 13% |
| RELOCATE-HISTORY | (embedded in STATUS) | — |
| DUPLICATE | ~45 | 3% |
| Overhead (headers/breaks) | ~225 | 16% |

**Key finding**: Only ~36% of the file (KEEP-INSTRUCTIONS + KEEP-REFERENCE) is behavioral or reference content. The remaining ~64% is status dashboards, historical records, and documentation indexes that dilute every prompt.

## Duplicates Found

1. **Determinism Validation COMPLETE** — appears at lines ~79-83 AND lines ~88-101 (two different detail levels)
2. **Phase B Month 4 Integration COMPLETE** — appears at lines ~84-86 AND lines ~103-114
3. **Chain of Thought Process** — appears at lines ~476-483 (short) AND lines ~490-520 (long)
4. **Error Handling Policy** — appears at lines ~484-488 AND lines ~1195-1200
5. **Phase 8 roadmap content** — appears in "Current State" AND in "Next Steps" section

## Relocation Plan

| Content | Destination |
|---------|-------------|
| Current State dashboard (all ~375 lines) | `docs/current/PROJECT_STATUS.md` |
| Performance Summary numbers | `docs/current/ARCHITECTURE_REFERENCE.md` |
| Strategic Planning Documents index | `docs/current/DOCUMENTATION_INDEX.md` |
| Week Summaries list | `docs/current/DOCUMENTATION_INDEX.md` |
| Key Metrics Documents list | `docs/current/DOCUMENTATION_INDEX.md` |
| Automation Scripts references | `docs/current/DOCUMENTATION_INDEX.md` |
| Quality & Audit Reports index | `docs/current/DOCUMENTATION_INDEX.md` |
| Next Steps — Phase 8 roadmap | `docs/current/PROJECT_STATUS.md` |
| Rendering & Materials deep dive | `docs/current/ARCHITECTURE_REFERENCE.md` |
| Performance Optimization (Week 8) | `docs/current/ARCHITECTURE_REFERENCE.md` |
| GOAP+Hermes Patterns 3-7 + perf + tests | `docs/current/ARCHITECTURE_REFERENCE.md` |
| Validation achievements list | `docs/current/PROJECT_STATUS.md` |

## Target Sizes

- **New copilot-instructions.md**: ~500-600 lines (~2,500-3,500 words, ~3,000-4,000 tokens)
- **PROJECT_STATUS.md**: ~400-500 lines
- **ARCHITECTURE_REFERENCE.md**: ~400-500 lines
- **DOCUMENTATION_INDEX.md**: ~300-400 lines
