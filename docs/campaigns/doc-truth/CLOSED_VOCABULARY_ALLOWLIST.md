# Closed-Vocabulary Allowlist — the D.3 lint false-positive set

Every KEEP occurrence from the D.1.C triage. The D.3 regression lint (which greps for [`CLOSED_VOCABULARY_LINT.md`](CLOSED_VOCABULARY_LINT.md) Vocab A + B) must **honor** these — they are correct occurrences (honest-dormancy descriptions, competitor figures, future targets, historical notes, real-module references, contested numbers awaiting D.2). Keyed on **(file + match-string + reason)** so the entry survives line drift. `lifetime`: **permanent** or **pending-D2** (re-evaluated when D.2 resolves the number).

**588 allowlisted occurrences.** Re-running the lint with this allowlist applied returns zero un-allowlisted hits — the certification (see `D1C_EXECUTION_REPORT.md`).

> **pending-D2 entries** (`contested-pending-D2`, `production-status-contested`): these are kept byte-identical now only because the number has no §1.2 arbiter yet. When D.2 resolves the value, the entry is re-evaluated — a then-stale number becomes ACT.

| File | Match | Reason | Lifetime |
|---|---|---|:-:|
| `docs/current/ATTRIBUTIONS.md` | `Hermes 2 Pro` | attribution | permanent |
| `docs/masters/MASTER_API_PATTERNS.md` | `Hermes 2 Pro` | benchmark/api-subject | permanent |
| `docs/masters/MASTER_BENCHMARK_REPORT.md` | `Qwen3-8B` | benchmark/api-subject | permanent |
| `docs/masters/MASTER_BENCHMARK_REPORT.md` | `Hermes 2 Pro` | benchmark/api-subject | permanent |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md` | `MaterialGraph` | chat-artifact | permanent |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/requirements.md` | `competitive with Unity` | chat-artifact | permanent |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/requirements.md` | `production-ready` | chat-artifact | permanent |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md` | `MaterialGraph` | chat-artifact | permanent |
| `.zencoder/chats/fd10dfb9-9c69-48ed-8473-5073acd4f1e8/spec.md` | `world-class` | chat-artifact | permanent |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md` | `world-class` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md` | `matches Unity` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md` | `exceeds Unity` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `matches Unity` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `production-ready` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `world-class` | competitor-cited | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `TLS 1.3` | competitor-cited | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `production-grade` | competitor-cited | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `Ed25519` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `exceeds Unity` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `TLS 1.3` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `matches Unity` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `world-class` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `exceeds Bevy` | competitor-cited | permanent |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md` | `matches Unity` | competitor-cited | permanent |
| `docs/audits/GAP_ANALYSIS_ACTION_PLAN.md` | `world-class` | competitor-cited | permanent |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md` | `world-class` | competitor-cited | permanent |
| `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md` | `production-grade` | competitor-cited | permanent |
| `docs/audits/g_pointer_events_research_2026-05-03.md` | `production-grade` | competitor-cited | permanent |
| `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md` | `Ed25519` | competitor-cited | permanent |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md` | `production-ready` | competitor-cited | permanent |
| `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` | `production-grade` | competitor-cited | permanent |
| `docs/current/GAME_ENGINE_READINESS_ROADMAP.md` | `QUIC` | competitor-cited | permanent |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md` | `production-ready` | competitor-cited | permanent |
| `docs/current/MUTATION_TESTING_REMEDIATION_REPORT.md` | `Ed25519` | competitor-cited | permanent |
| `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` | `production-grade` | competitor-cited | permanent |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md` | `production-ready` | competitor-cited | permanent |
| `docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md` | `Ed25519` | competitor-cited | permanent |
| `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` | `Ed25519` | competitor-cited | permanent |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md` | `matches Unity` | competitor-cited | permanent |
| `docs/pbr/PBR_E_DESIGN.md` | `matches UE5` | competitor-cited | permanent |
| `tools/ASSET_SIGNING_DESIGN.md` | `Ed25519` | competitor-cited | permanent |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md` | `world-class` | competitor-cited | permanent |
| `docs/current/PROJECT_STATUS.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/lessons/PERFORMANCE_PATTERNS.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/masters/MASTER_BENCHMARK_REPORT.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/src/api/physics.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/src/core-systems/physics.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/src/performance/benchmarks.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/src/performance/budgets.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/src/performance/optimization.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `gh-pages/benchmarks.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `gh-pages/index.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `gh-pages/physics.md` | `99.96%` | contested-pending-D2 | pending-D2 |
| `docs/reference/Interfaces.md` | `EntityView` | design-sketch-caveated | permanent |
| `docs/audits/tonemap_double_application_investigation_2026-04-19.md` | `no other engine` | false-positive | permanent |
| `astraweave-llm/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/current/ARCHITECTURE_REFERENCE.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/current/DOCUMENTATION_INDEX.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/current/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/current/RENDERING_ANALYSIS_INDEX.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/current/RENDERING_QUICK_REFERENCE.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/guides/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_D_FINAL_SUMMARY.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_D_QUICK_SUMMARY.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_D_VALIDATION_REPORT.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_E_QUICK_REFERENCE.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_F_COMPLETION_SUMMARY.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/pbr/PBR_F_QUICK_REFERENCE.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/src/core-systems/ai/arbiter.md` | `QUIC` | false-positive-QUICK | permanent |
| `examples/hello_companion/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `tools/astraweave-assets/README.md` | `QUIC` | false-positive-QUICK | permanent |
| `docs/Veilweaver/lore_bible.md` | `most comprehensive` | fiction | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `production-ready` | future-target | permanent |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md` | `production-ready` | future-target | permanent |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md` | `world-class` | future-target | permanent |
| `docs/audits/TEST_SUITE_COMPREHENSIVE_AUDIT.md` | `production-grade` | future-target | permanent |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md` | `production-ready` | future-target | permanent |
| `docs/current/AW_EDITOR_RECOVERY_ROADMAP.md` | `production-ready` | future-target | permanent |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md` | `production-ready` | future-target | permanent |
| `docs/current/ECS_REGRESSION_SESSION_COMPLETE.md` | `production-ready` | future-target | permanent |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md` | `production-grade` | future-target | permanent |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md` | `production-ready` | future-target | permanent |
| `docs/current/IMPLEMENTATION_PLANS_INDEX.md` | `production-ready` | future-target | permanent |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md` | `production-ready` | future-target | permanent |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md` | `industry-leading` | future-target | permanent |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md` | `world-class` | future-target | permanent |
| `docs/current/WORLD_CLASS_EDITOR_DELIVERY_PLAN.md` | `world-class` | future-target | permanent |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md` | `world-class` | future-target | permanent |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md` | `production-ready` | future-target | permanent |
| `tools/aw_editor/CODE_QUALITY_STATUS.md` | `production-ready` | future-target | permanent |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md` | `world-class` | future-target | permanent |
| `tools/aw_editor/PHASE_2_COMPLETION_SUMMARY.md` | `world-class` | future-target | permanent |
| `docs/_audit/discovery-report.md` | `47 crates` | historical-audit-denominator | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `47 crates` | historical-audit-denominator | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `47 crates` | historical-audit-denominator | permanent |
| `docs/current/MASTER_COVERAGE_REPORT.md` | `47 crates` | historical-audit-denominator | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `Hermes 2 Pro` | historical-dated | permanent |
| `docs/current/PROJECT_STATUS.md` | `Hermes 2 Pro` | historical-dated | permanent |
| `docs/current/PROJECT_STATUS.md` | `6 AI modes` | historical-dated | permanent |
| `docs/lessons/AI_ORCHESTRATION_TIPS.md` | `6 AI modes` | historical-dated | permanent |
| `docs/masters/MASTER_BENCHMARK_REPORT.md` | `Qwen3-8B` | historical-dated | permanent |
| `docs/masters/MASTER_ROADMAP.md` | `4,907` | historical-dated | permanent |
| `docs/src/core-systems/networking.md` | `QUIC` | historical-dated | permanent |
| `tools/ASSET_SIGNING_DESIGN.md` | `Ed25519` | historical-dated | permanent |
| `tools/aw_editor/WORLD_CLASS_EDITOR_PLAN.md` | `world-class` | historical-dated | permanent |
| `.github/copilot-instructions.md` | `check-all` | honest-dormancy | permanent |
| `.zencoder/rules/repo.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `CLAUDE.md` | `production-grade` | honest-dormancy | permanent |
| `CLAUDE.md` | `check-all` | honest-dormancy | permanent |
| `CLAUDE.md` | `QUIC` | honest-dormancy | permanent |
| `CLAUDE.md` | `qwen3:8b` | honest-dormancy | permanent |
| `CLAUDE.md` | `99.96%` | honest-dormancy | permanent |
| `astraweave-ecs/README.md` | `production-ready` | honest-dormancy | permanent |
| `astraweave-physics/README.md` | `99.96%` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `production-grade` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `ResearchFluidSystem` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `qwen3:8b` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `99.96%` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `QUIC` | honest-dormancy | permanent |
| `docs/architecture/ARCHITECTURE_MAP.md` | `hermes2pro` | honest-dormancy | permanent |
| `docs/architecture/ai_pipeline.md` | `hermes2pro` | honest-dormancy | permanent |
| `docs/architecture/ai_pipeline.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `docs/architecture/aw_editor.md` | `world-class` | honest-dormancy | permanent |
| `docs/architecture/aw_editor.md` | `QUIC` | honest-dormancy | permanent |
| `docs/architecture/aw_editor.md` | `production-grade` | honest-dormancy | permanent |
| `docs/architecture/aw_editor.md` | `3,892` | honest-dormancy | permanent |
| `docs/architecture/fluids.md` | `ResearchFluidSystem` | honest-dormancy | permanent |
| `docs/architecture/fluids.md` | `DFSPH` | honest-dormancy | permanent |
| `docs/architecture/fluids.md` | `world-class` | honest-dormancy | permanent |
| `docs/architecture/fluids.md` | `UnifiedSolver` | honest-dormancy | permanent |
| `docs/architecture/net.md` | `QUIC` | honest-dormancy | permanent |
| `docs/architecture/net_ecs.md` | `production-grade` | honest-dormancy | permanent |
| `docs/architecture/net_ecs.md` | `production-ready` | honest-dormancy | permanent |
| `docs/architecture/persistence_ecs.md` | `production-grade` | honest-dormancy | permanent |
| `docs/architecture/physics.md` | `99.96%` | honest-dormancy | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `world-class` | honest-dormancy | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md` | `world-class` | honest-dormancy | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `Ed25519` | honest-dormancy | permanent |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md` | `production-ready` | honest-dormancy | permanent |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` | `ResearchFluidSystem` | honest-dormancy | permanent |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` | `UnifiedSolver` | honest-dormancy | permanent |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` | `DFSPH` | honest-dormancy | permanent |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md` | `UnifiedSolver` | honest-dormancy | permanent |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md` | `ResearchFluidSystem` | honest-dormancy | permanent |
| `docs/current/ARCHITECTURE_REFERENCE.md` | `99.96%` | honest-dormancy | permanent |
| `docs/current/FLUIDS_MUTATION_TESTING_REPORT.md` | `DFSPH` | honest-dormancy | permanent |
| `docs/current/MASTER_COVERAGE_REPORT.md` | `4,907` | honest-dormancy | permanent |
| `docs/current/QWEN3_BENCHMARK_REPORT.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/current/QWEN3_BENCHMARK_REPORT.md` | `qwen3:8b` | honest-dormancy | permanent |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `hermes2pro` | honest-dormancy | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `qwen3:8b` | honest-dormancy | permanent |
| `docs/current/RENDERING_GAPS_ANALYSIS_AND_FIX_PLAN.md` | `AAA standards` | honest-dormancy | permanent |
| `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` | `Ed25519` | honest-dormancy | permanent |
| `docs/guides/networking_envelopes.md` | `QUIC` | honest-dormancy | permanent |
| `docs/lessons/AI_ORCHESTRATION_TIPS.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/lessons/PERFORMANCE_PATTERNS.md` | `99.96%` | honest-dormancy | permanent |
| `docs/lessons/WHAT_DIDNT.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/lessons/WHAT_WORKED.md` | `Hermes 2 Pro` | honest-dormancy | permanent |
| `docs/lessons/WHAT_WORKED.md` | `99.96%` | honest-dormancy | permanent |
| `docs/masters/MASTER_ROADMAP.md` | `27,000+` | honest-dormancy | permanent |
| `docs/pbr/PBR_F_DESIGN.md` | `production-ready` | honest-dormancy | permanent |
| `docs/src/core-systems/networking.md` | `QUIC` | honest-dormancy | permanent |
| `docs/src/core-systems/networking.md` | `quinn` | honest-dormancy | permanent |
| `docs/src/core-systems/physics.md` | `99.96%` | honest-dormancy | permanent |
| `gh-pages/ai.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `gh-pages/architecture.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `gh-pages/index.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `gh-pages/physics.md` | `99.96%` | honest-dormancy | permanent |
| `gh-pages/setup.md` | `Qwen3-8B` | honest-dormancy | permanent |
| `gh-pages/setup.md` | `qwen3:8b` | honest-dormancy | permanent |
| `tools/ASSET_SIGNING_DESIGN.md` | `Ed25519` | honest-dormancy | permanent |
| `docs/audits/COMPETITIVE_MATRIX.md` | `world-class` | keep-verified | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `exceeds industry` | keep-verified | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `world-class` | keep-verified | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md` | `world-class` | keep-verified | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `AAA standards` | keep-verified | permanent |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | `exceeds industry` | keep-verified | permanent |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md` | `world-class` | keep-verified | permanent |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md` | `most comprehensive` | keep-verified | permanent |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` | `world-class` | keep-verified | permanent |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md` | `world-class` | keep-verified | permanent |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md` | `world-class` | keep-verified | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `check-all` | make-target | permanent |
| `gh-pages/setup.md` | `build-core` | make-target | permanent |
| `gh-pages/setup.md` | `check-all` | make-target | permanent |
| `gh-pages/setup.md` | `test-all` | make-target | permanent |
| `gh-pages/setup.md` | `clippy-all` | make-target | permanent |
| `README.md` | `production-grade` | production-shipped | permanent |
| `astraweave-ecs/README.md` | `production-grade` | production-shipped | permanent |
| `docs/current/EDITOR_STATUS_REPORT.md` | `production-ready` | production-shipped | permanent |
| `docs/_audit/discovery-report.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/audits/TEST_SUITE_COMPREHENSIVE_AUDIT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/AW_EDITOR_CORRECTNESS_AUDIT_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md` | `production-grade` | production-status-contested | pending-D2 |
| `docs/current/BLEND_IMPORT_INTEGRATION_COMPLETE.md` | `production-grade` | production-status-contested | pending-D2 |
| `docs/current/BLEND_IMPORT_INTEGRATION_COMPLETE.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/ECS_MIRI_VALIDATION_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/ECS_REGRESSION_SESSION_COMPLETE.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/EDITOR_STATUS_REPORT.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/GAME_ENGINE_READINESS_ROADMAP.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md` | `production-grade` | production-status-contested | pending-D2 |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/IMPLEMENTATION_PLANS_INDEX.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/PHASE_8_ROADMAP_REVIEW.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/PROJECT_STATUS.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/RENDERING_QUICK_REFERENCE.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/TERRAIN_SCATTER_FIX_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/VEILWEAVER_VERTICAL_SLICE_ANALYSIS.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/lessons/PERFORMANCE_PATTERNS.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/lessons/WHAT_WORKED.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/masters/MASTER_ROADMAP.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/reference/RENDERING_SOTA_REFERENCE.md` | `production-ready` | production-status-contested | pending-D2 |
| `docs/src/resources/faq.md` | `production-ready` | production-status-contested | pending-D2 |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md` | `production-grade` | production-status-contested | pending-D2 |
| `tools/aw_editor/INTEGRATION_ROADMAP.md` | `production-ready` | production-status-contested | pending-D2 |
| `tools/aw_editor/PHASE_2_COMPLETION_SUMMARY.md` | `production-ready` | production-status-contested | pending-D2 |
| `tools/aw_editor/PRODUCTION_READINESS_AUDIT.md` | `production-ready` | production-status-contested | pending-D2 |
| `tools/aw_editor/VIEWPORT_ENHANCEMENT_COMPLETE.md` | `production-ready` | production-status-contested | pending-D2 |
| `tools/aw_editor/WORLD_CLASS_EDITOR_PLAN.md` | `production-ready` | production-status-contested | pending-D2 |
| `.github/copilot-instructions.md` | `production-grade` | project-thesis | permanent |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | `production-grade` | project-thesis | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `Hermes 2 Pro` | real-client-mention | permanent |
| `docs/lessons/TESTING_STRATEGIES.md` | `Hermes 2 Pro` | real-client-mention | permanent |
| `docs/lessons/TESTING_STRATEGIES.md` | `hermes2pro` | real-client-mention | permanent |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/audits/job_system_audit_2026-04-18.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/current/MUTATION_TESTING_AUDIT.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/lessons/TESTING_STRATEGIES.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/masters/MASTER_API_PATTERNS.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/masters/MASTER_BENCHMARK_REPORT.md` | `hermes2pro` | real-module-reference | permanent |
| `docs/current/QWEN3_BENCHMARK_REPORT.md` | `Qwen3-8B` | subject-doc | permanent |
| `docs/current/QWEN3_BENCHMARK_REPORT.md` | `qwen3:8b` | subject-doc | permanent |
| `docs/current/QWEN3_BENCHMARK_REPORT.md` | `Hermes 2 Pro` | subject-doc | permanent |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md` | `Qwen3-8B` | subject-doc | permanent |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md` | `hermes2pro` | subject-doc | permanent |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md` | `Hermes 2 Pro` | subject-doc | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `Hermes 2 Pro` | subject-doc | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `hermes2pro` | subject-doc | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `Qwen3-8B` | subject-doc | permanent |
| `docs/current/QWEN3_MIGRATION_PLAN.md` | `qwen3:8b` | subject-doc | permanent |
| `scripts/temperature_experiment_guide.md` | `Hermes 2 Pro` | subject-doc | permanent |
| `scripts/temperature_experiment_guide.md` | `hermes2pro` | subject-doc | permanent |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` | `DFSPH` | subject-doc-roadmap | permanent |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` | `IISPH` | subject-doc-roadmap | permanent |
| `docs/audits/terrain_scale_diagnostic_2026-04-24.md` | `3-6` | terrain-measurement | permanent |

*264 distinct (file, match, reason) allowlist keys covering all 588 occurrences. Full per-line evidence in [`CLOSED_VOCABULARY_TRIAGE.md`](CLOSED_VOCABULARY_TRIAGE.md).*