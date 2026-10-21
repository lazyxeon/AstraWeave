# Current Development Documentation

Active planning documents for ongoing AstraWeave development.

> **2026-10-21:** the root [README](../../README.md) is the short summary. The long status page is [ENGINE_STATUS.md](ENGINE_STATUS.md). The banner at the top of [PROJECT_STATUS.md](PROJECT_STATUS.md) is the session log. Phase 8 week plans below are older campaign notes. #216 is on main. The heavy-scene GPU frame on a GTX 1660 Ti Max-Q is 3.688 ms, `budget_11ms=pass`, and the amortized `cloud_shadow` pass is 0.323 ms. #394, #396, and #397 are on main (`49d289eb`). #397 closed #260. GitHub linked that issue to the pull. #389 is the Hi-Z skip. #393 is the wiring inventory. `gpu_frame_budget` rejects an enabled gated flag before the scene. #403 and #400 stay open. #371 and #372 stay open. Local field #330 is on main. Weaving #252, the Aravath greybox #205, traversal #244, terrain columns #215, CI burndown #217, greybox tokio graph #327, greybox combat slice #275, and off-tick AI #331 are on main. #253 and #336 are closed. `gravity.rs` is deleted. `AIArbiter` and `GoapPlanner` stay off the deterministic tick. The required replay set has nineteen cases, including `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`. The greybox golden is `b6a9d5a2c5da28a6` on Linux, Windows, and macOS (#381). #281, #283, and #284 are closed. #388 (`43b1b48f`) closed #295. `queue_weave` rejects origins outside ±2048 cells and does not saturate. The `weaving` case still hashes `NullField`.

> [!TIP]
> For authoritative master reports, see **[../masters/](../masters/)**.

---

## Key Reference Documents
- **[PROJECT_STATUS.md](PROJECT_STATUS.md)** - Current state, active phases, recently completed
- **[ARCHITECTURE_REFERENCE.md](ARCHITECTURE_REFERENCE.md)** - Detailed patterns, API deep dives, performance data
- **[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)** - Master navigation for all project docs

---

## Phase 8: Game Engine Readiness

### Master Plans (Start Here)
- **[PHASE_8_MASTER_INTEGRATION_PLAN.md](PHASE_8_MASTER_INTEGRATION_PLAN.md)** - Coordination of all 4 priorities
- **[GAME_ENGINE_READINESS_ROADMAP.md](GAME_ENGINE_READINESS_ROADMAP.md)** - Overall strategy
- **[PHASE_8_ROADMAP_REVIEW.md](PHASE_8_ROADMAP_REVIEW.md)** - Roadmap validation
- **[IMPLEMENTATION_PLANS_INDEX.md](IMPLEMENTATION_PLANS_INDEX.md)** - Navigation guide

### Priority 1: UI Framework
- **[PHASE_8_PRIORITY_1_UI_PLAN.md](PHASE_8_PRIORITY_1_UI_PLAN.md)** - 5-week UI plan
- **[PHASE_8_1_WEEK_4_PLAN.md](PHASE_8_1_WEEK_4_PLAN.md)** - Current week

### Priority 2: Rendering
- **[PHASE_8_PRIORITY_2_RENDERING_PLAN.md](PHASE_8_PRIORITY_2_RENDERING_PLAN.md)** - 4-5 week rendering
- **[RENDERER_MASTER_IMPLEMENTATION_PLAN.md](RENDERER_MASTER_IMPLEMENTATION_PLAN.md)** - Detailed implementation
- **[RENDERING_ANALYSIS_INDEX.md](RENDERING_ANALYSIS_INDEX.md)** - Analysis navigation

### Priority 3: Save/Load
- **[PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md](PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md)** - 2-3 week save/load
- **[SAVE_LOAD_INTEGRATION_GUIDE.md](SAVE_LOAD_INTEGRATION_GUIDE.md)** - Integration guide

### Priority 4: Audio
- **[PHASE_8_PRIORITY_4_AUDIO_PLAN.md](PHASE_8_PRIORITY_4_AUDIO_PLAN.md)** - 2-3 week audio

---

## Editor Development
- **[AW_EDITOR_QUICK_REFERENCE.md](AW_EDITOR_QUICK_REFERENCE.md)** - Quick reference
- **[AW_EDITOR_KNOWN_ISSUES.md](AW_EDITOR_KNOWN_ISSUES.md)** - Known issues
- **[AW_EDITOR_RECOVERY_ROADMAP.md](AW_EDITOR_RECOVERY_ROADMAP.md)** - Recovery roadmap
- **[EDITOR_STATUS_REPORT.md](EDITOR_STATUS_REPORT.md)** - Current status
- **[EDITOR_TEST_STRATEGY_REPORT.md](EDITOR_TEST_STRATEGY_REPORT.md)** - Test strategy

---

## Strategic Planning
- **[LONG_HORIZON_STRATEGIC_PLAN.md](LONG_HORIZON_STRATEGIC_PLAN.md)** - 12-month roadmap
- **[PERFORMANCE_BUDGET_ANALYSIS.md](PERFORMANCE_BUDGET_ANALYSIS.md)** - Performance budgets

---

## Other Active Documents
- **[ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md](ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md)** - UI framework
- **[BEVY_RENDERER_INTEGRATION_MASTER_PLAN.md](BEVY_RENDERER_INTEGRATION_MASTER_PLAN.md)** - Bevy integration
- **[ATTRIBUTIONS.md](ATTRIBUTIONS.md)** - Third-party attributions

---

*For archived documentation, see [../archive/](../archive/).*
