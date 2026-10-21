# AstraWeave Documentation Index

> **Master navigation** for all project documentation. Consolidated from inline references that were previously scattered throughout the instructions file.

---

## Quick Access

| Document | Purpose |
|----------|---------|
| `docs/current/PROJECT_STATUS.md` | Current state, active phases, recently completed |
| `docs/current/ARCHITECTURE_REFERENCE.md` | Detailed patterns, API deep dives, performance data |
| `docs/current/MASTER_ROADMAP.md` | Strategic planning context |
| `docs/current/MASTER_BENCHMARK_REPORT.md` | Performance baselines |
| `docs/current/MASTER_COVERAGE_REPORT.md` | Test coverage status |
| `.github/copilot-instructions.md` | Agent behavioral directives |

---

## Master Reports (Authoritative)

These three reports are the authoritative sources and must be kept up to date per the maintenance protocol in the instructions file.

- **`docs/current/MASTER_ROADMAP.md`** — Overall strategic roadmap, prioritized action items, timeline estimates
- **`docs/current/MASTER_BENCHMARK_REPORT.md`** — Per-crate benchmark results, 60 FPS budget analysis, performance highlights
- **`docs/current/MASTER_COVERAGE_REPORT.md`** — Per-tier coverage percentages, overall coverage, priority breakdown

---

## Phase 8: Game Engine Readiness

### Master Plans
- **`docs/current/PHASE_8_MASTER_INTEGRATION_PLAN.md`** — Coordination of all 4 priorities (START HERE)
- **`docs/current/GAME_ENGINE_READINESS_ROADMAP.md`** — Overall gap analysis and strategy
- **`docs/current/PHASE_8_ROADMAP_REVIEW.md`** — Roadmap validation vs actual codebase

### Priority Plans
- **`docs/current/PHASE_8_PRIORITY_1_UI_PLAN.md`** — 5-week UI framework (egui-wgpu)
- **`docs/current/PHASE_8_PRIORITY_2_RENDERING_PLAN.md`** — 4-5 week rendering completion
- **`docs/current/PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md`** — 2-3 week save/load system
- **`docs/current/PHASE_8_PRIORITY_4_AUDIO_PLAN.md`** — 2-3 week production audio

### Active Sprints
- **`docs/current/PHASE_8_8_PHYSICS_ROBUSTNESS.md`** — Physics subsystem upgrade (ACTIVE)
- **`docs/current/PHASE_8_7_LLM_TESTING_SPRINT.md`** — LLM testing sprint (COMPLETE)
- **`docs/current/PHASE_8_6_UI_TESTING_SPRINT.md`** — UI testing sprint (COMPLETE)

---

## Formal Verification & Safety

- **`docs/current/MIRI_VALIDATION_REPORT.md`** — 977 tests, 0 UB, 4 crates validated
- **`docs/current/ECS_MIRI_VALIDATION_REPORT.md`** — ECS-specific Miri deep dive
- **`docs/current/BULLETPROOF_VALIDATION_PLAN.md`** — Miri + Kani + mutation testing plan
- **`.github/workflows/miri.yml`** — Miri CI workflow (weekly)
- **`.github/workflows/kani.yml`** — Kani CI workflow

---

## Quality & Audit Reports

- **`docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`** — Multi-agent audit, A- (92/100)
- **`docs/audits/SECURITY_REMEDIATION_REPORT.md`** — Priority 1 security fixes (A- grade)
- **`docs/audits/DOCUMENTATION_AUDIT_REPORT.md`** — Documentation grade C+ (73/100)
- **`docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`** — Executive summary
- **`docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`** — Market positioning
- **`docs/audits/COMPETITIVE_MATRIX.md`** — Feature comparison matrix
- **`docs/audits/GAP_ANALYSIS_ACTION_PLAN.md`** — Prioritized remediation

---

## Strategic Planning

- **`docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`** — 12-month roadmap (Phases A, B, C)
- **`docs/current/IMPLEMENTATION_PLANS_INDEX.md`** — Navigation guide for all planning docs
- **`docs/current/PERFORMANCE_BUDGET_ANALYSIS.md`** — Performance budgets
- **`docs/current/PHASE_9_2_SCRIPTING_INTEGRATION_PLAN.md`** — Rhai scripting (future)

---

## Veilweaver Game Project

- **`docs/current/VEILWEAVER_FOUNDATION_AUDIT_REPORT.md`** — Foundation audit (A+)
- **`docs/projects/veilweaver/FOUNDATION_AUDIT_SUMMARY.md`** — Audit summary

---

## Editor Development

- **`docs/current/AW_EDITOR_QUICK_REFERENCE.md`** — Quick reference
- **`docs/current/AW_EDITOR_KNOWN_ISSUES.md`** — Known issues
- **`docs/current/AW_EDITOR_RECOVERY_ROADMAP.md`** — Recovery roadmap
- **`docs/current/EDITOR_STATUS_REPORT.md`** — Current status
- **`docs/current/EDITOR_TEST_STRATEGY_REPORT.md`** — Test strategy

---

## Code Quality & Metrics

- **`docs/root-archive/UNWRAP_AUDIT_ANALYSIS.md`** — 637 `.unwrap()` calls cataloged
- **`docs/root-archive/BASELINE_METRICS.md`** — Performance baselines (all subsystems)
- **`docs/root-archive/AI_NATIVE_VALIDATION_REPORT.md`** — 28 tests, A+ grade, 12,700+ capacity
- **`scripts/audit_unwrap.ps1`** — PowerShell unwrap auditing script

---

## Astract Gizmo (UI Framework)

- **`docs/astract/GETTING_STARTED.md`** — Installation → first widget
- **`docs/astract/CHARTS_TUTORIAL.md`** — LineChart, BarChart, ScatterPlot
- **`docs/astract/ADVANCED_WIDGETS_TUTORIAL.md`** — ColorPicker, TreeView, RangeSlider
- **`docs/astract/NODEGRAPH_TUTORIAL.md`** — Behavior trees, shaders, dialogue
- **`docs/astract/ANIMATION_TUTORIAL.md`** — Tweens, springs, easing
- **`docs/astract/API_REFERENCE.md`** — Complete method docs
- **`docs/astract/BENCHMARKS.md`** — Performance analysis

---

## Journey Documentation (Historical)

> These documents preserve the development history as evidence of the AI-orchestration experiment. **Never delete.**

### Phase Completion Reports
Located in `docs/journey/phases/` and `docs/root-archive/`:
- Phase 6 Completion Summary, Phase 7 Validation Report
- Hermes2Pro Migration reports (Phase 1 Audit, Phase 3 Code, Phase 4 Docs)

### Weekly Summaries
Located in `docs/journey/weeks/` and `docs/root-archive/`:
- Weeks 1-8 completion summaries and detailed day-by-day reports

### Daily Logs
Located in `docs/journey/daily/`:
- Phase 8.1 Week 1-4 daily completion reports
- Astract Gizmo Days 9-13

### Week Summaries Index
| Week | Key Achievement |
|------|----------------|
| Week 1 | GPU skinning, combat physics, unwrap audit |
| Week 2 | Testing sprint (233 tests passing) |
| Week 3 | Integration tests, benchmarks, API docs |
| Week 4 | Async physics, terrain, LLM, Veilweaver demo |
| Week 5 | GPU mesh optimization, SIMD math |
| Week 8 | Tracy profiling, spatial hash, SIMD movement |

---

## Automation Scripts

- **`scripts/audit_unwrap.ps1`** — Find/categorize `.unwrap()` calls
- **`scripts/bootstrap.sh`** — Project setup
- **`scripts/check_benchmark_thresholds.ps1`** — Benchmark validation

---

*Last updated: February 8, 2026*
