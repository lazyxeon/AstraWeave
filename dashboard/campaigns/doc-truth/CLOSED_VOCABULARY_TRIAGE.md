# Closed-Vocabulary Triage — D.1.C certification

Every occurrence the closed-vocabulary grep returned at the current working tree (post-A+B), with exactly one verdict. **594 pre-fix** = **586 KEEP-from-start** + **8 ACT** (fixed below). Post-fix grep = **588** (the 6 surgical superlative removals leave the tree; the 2 vocab-A ACTs were corrected in place and remain as KEEP). The KEEP set is the standing allowlist ([`CLOSED_VOCABULARY_ALLOWLIST.md`](CLOSED_VOCABULARY_ALLOWLIST.md)).

## ACT (8) — fixed this pass

| File:line | V | Action | Before → After | Note |
|---|:-:|:-:|---|---|
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`:151 | B | REWRITE | `Industry-standard PBR (matches UE5/Unity quality)` → `Industry-standard PBR (Cook-Torrance BRDF)` | uncited UE5/Unity parity clause removed |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`:185 | B | DELETE | `...(3% of 60 FPS budget). Competitive with UE5/Unity.` → `...(3% of 60 FPS budget).` | uncited "Competitive with UE5/Unity" sentence removed |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:532 | B | REWRITE | `**No competitor has this** (Unity ML-Agents is training-onl…` → `**No direct competitor equivalent identified** (Unity ML-Ag…` | no-other-engine absolute softened to a findable claim |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:636 | B | REWRITE | `Coverage exceeds industry (71.37% vs 60-70%)` → `Coverage 71.37% (vs typical 60-70%)` | "exceeds industry" superlative removed; numbers kept (contested) |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md`:270 | B | REWRITE | `**AstraWeave Exceeds Industry in:**` → `**AstraWeave's benchmark strengths:**` | "Exceeds Industry" bald comparative reframed |
| `docs/current/EDITOR_STATUS_REPORT.md`:207 | B | REWRITE | `Override tracking rivals Unity/Unreal` → `Override tracking modeled on Unity/Unreal prefab systems` | "rivals" superlative -> "modeled on" (design influence, not quality claim) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:936 | A | CORRECT | ``cargo check-all` + `cargo test-all`` → ``make check-all` + `make test-all` (Makefile targets; no ca…` | phantom cargo aliases -> the real Makefile targets |
| `docs/reference/Interfaces.md`:94 | A | REWRITE | `Notes - These are interface sketches; exact fields may diff…` → `Notes - **non-implemented design sketch, not the wire forma…` | protobuf EntityView/WorldSnapshot block caveated to honest-dormancy |

Discipline note: two `6 AI modes` occurrences (`PROJECT_STATUS.md:216`, `AI_ORCHESTRATION_TIPS.md:212`) were **reversed from ACT to KEEP** on review — both are Phase-6 historical records ("6 modes" was accurate then), and correcting them would falsify the record. This is the over-correction the phase guards against.

## KEEP (588) — by reason

### `honest-dormancy` (164) — permanent

*Text states the drift truthfully (the correction layer) — traces, ARCHITECTURE_MAP, CLAUDE.md, F0/F1 audits, "Not QUIC", "linear scan", corrected "phi3 default; Hermes opt-in"*

| File:line | V | Match |
|---|:-:|---|
| `.github/copilot-instructions.md`:43 | A | - Use the editor cargo aliases (`editor`, `editor-release`, `editor-dev`); for works… |
| `.zencoder/rules/repo.md`:118 | A | - **Phase 6** ✅ (Oct 14): Hermes 2 Pro integration, 54 errors → 0 |
| `CLAUDE.md`:7 | B | AstraWeave is a **scientific proof of concept**: a production-grade AI-native game e… |
| `CLAUDE.md`:107 | A | - Use the editor cargo aliases (`editor`, `editor-release`, `editor-dev`); for works… |
| `CLAUDE.md`:368 | A | - `docs/src/core-systems/networking.md` — claims QUIC via Quinn; actual implementati… |
| `CLAUDE.md`:374 | A | - **AI runtime model**: doc-comments and CLAUDE.md describe Qwen3-based hybrid; `ast… |
| `CLAUDE.md`:377 | A | - **SpatialHash broadphase**: `astraweave-physics/src/lib.rs:25-26` doc-comment adve… |
| `astraweave-ecs/README.md`:5 | B | AstraWeave ECS is a **production-ready, AI-native** Entity Component System designed… |
| `astraweave-physics/README.md`:15 | A | \| `SpatialHash` \| Dormant in-crate grid (advertised "99.96% pair reduction"); the … |
| `docs/architecture/ARCHITECTURE_MAP.md`:19 | B | \| Persistence (aw-save + persistence-ecs) \| [`persistence_ecs.md`](persistence_ecs… |
| `docs/architecture/ARCHITECTURE_MAP.md`:23 | A | \| Fluids \| [`fluids.md`](fluids.md) \| **Dormant for runtime engine.** 84.5K LoC; … |
| `docs/architecture/ARCHITECTURE_MAP.md`:204 | A | - **Reconciliation note (v0.7.0)**: Per `ai_pipeline.md` §6, runtime LLM model defau… |
| `docs/architecture/ARCHITECTURE_MAP.md`:235 | A | - **Reconciliation note (v0.7.0 — SpatialHash dormancy)**: The crate-level doc-comme… |
| `docs/architecture/ARCHITECTURE_MAP.md`:241 | A | - Three real parallel solver/manager surfaces (**not unified**): `FluidSystem` (lib.… |
| `docs/architecture/ARCHITECTURE_MAP.md`:508 | A | \| `docs/src/core-systems/networking.md` \| Claims **QUIC (via Quinn)** transport wi… |
| `docs/architecture/ARCHITECTURE_MAP.md`:514 | A | \| AI runtime model is Qwen3 \| `astraweave-ai/src/ai_arbiter.rs:1` doc-comment ("GO… |
| `docs/architecture/ARCHITECTURE_MAP.md`:517 | A | \| `SpatialHash` is the physics broadphase \| `astraweave-physics/src/lib.rs:25-26` … |
| `docs/architecture/ARCHITECTURE_MAP.md`:650 | B | The `aw-save` layer (file format) is production-grade. The `astraweave-persistence-e… |
| `docs/architecture/ARCHITECTURE_MAP.md`:977 | A | - Added §7 Documentation Hazards consolidating the cross-trace doc-vs-code drift inv… |
| `docs/architecture/ai_pipeline.md`:31 | A | - **LLM integration:** `astraweave-llm/src/{lib,llm_adapter,plan_parser,fallback_sys… |
| `docs/architecture/ai_pipeline.md`:41 | A | 3. **The Arbiter is the production hybrid path.** GOAP for instant control, the LLM … |
| `docs/architecture/ai_pipeline.md`:318 | A | \| External LLM endpoints \| Ollama HTTP API (Qwen3 / Hermes2Pro / Phi3 ports), loca… |
| `docs/architecture/ai_pipeline.md`:412 | A | \| `astraweave-llm/src/{phi3_ollama,hermes2pro_ollama,qwen3_ollama}.rs` \| Ollama HT… |
| `docs/architecture/ai_pipeline.md`:540 | A | **What's actually true** (verified 2026-05-12): `astraweave-ai/src/orchestrator.rs:4… |
| `docs/architecture/ai_pipeline.md`:565 | A | - **Context:** LLM latency (3-8 s for Qwen3-8B) is too high for per-frame use. GOAP … |
| `docs/architecture/ai_pipeline.md`:649 | A | - **LLM inference**: 3-8 s for Qwen3-8B (per `ai_arbiter.rs:18`). Phi3-Medium varies… |
| `docs/architecture/ai_pipeline.md`:711 | A | - **Hermes2Pro vs Qwen3 vs Phi3 — settled model choice?** [Decisional, **enriched by… |
| `docs/architecture/ai_pipeline.md`:2225 | A | - **`streaming_parser.rs` requires `complete_streaming` to be implemented by clients… |
| `docs/architecture/aw_editor.md`:333 | B | - `tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md` — World-class editor target fea… |
| `docs/architecture/aw_editor.md`:341 | A | - `docs/current/AW_EDITOR_*.md` (10 files verified 2026-05-12: `AUTHORING_PLAN`, `CO… |
| `docs/architecture/aw_editor.md`:356 | B | \| `src/panels/` (49 files) \| One file per dockable panel; each implements `Panel` … |
| `docs/architecture/aw_editor.md`:650 | A | - **Test count**: ~9,427 `#[test]` annotations total <!-- Source: CLAIMS_REGISTRY.md… |
| `docs/architecture/fluids.md`:12 | A | \| **Status** \| **Dormant for the runtime engine; large parallel-solver inventory; … |
| `docs/architecture/fluids.md`:13 | A | \| **Owner notes** \| Scale: 35 Rust source files, 8 WGSL compute shaders (7 in `sha… |
| `docs/architecture/fluids.md`:22 | A | - **`ResearchFluidSystem` never existed.** v1.2 inventoried it as an active research… |
| `docs/architecture/fluids.md`:30 | A | - **`SolverType::DFSPH`/`IISPH` variants deleted** (no solver loop existed); quality… |
| `docs/architecture/fluids.md`:42 | A | Provides GPU-accelerated fluid simulation through multiple coexisting solvers (PBD, … |
| `docs/architecture/fluids.md`:45 | B | Per `astraweave-fluids/README.md:1`: "A production-grade GPU-accelerated fluid simul… |
| `docs/architecture/fluids.md`:49 | A | - **Research solvers:** `astraweave-fluids/src/{research,pcisph_system,unified_solve… |
| `docs/architecture/fluids.md`:62 | A | 2. **Five parallel solver/manager surfaces coexist** with overlapping responsibiliti… |
| `docs/architecture/fluids.md`:274 | A | \| **`UnifiedSolver`** \| High-level interface combining research-grade SPH solvers … |
| `docs/architecture/fluids.md`:275 | A | \| **`UnifiedSolverConfig`** \| Config selecting `SolverType`, `ViscositySolverType`… |
| `docs/architecture/fluids.md`:277 | A | \| **`SolverType` (research.rs)** \| `#[non_exhaustive]` enum: `PBD` (default), `PCI… |
| `docs/architecture/fluids.md`:282 | A | \| **`ResearchQualityTier`** \| `#[non_exhaustive]` 5-variant enum: `Low`, `Medium` … |
| `docs/architecture/fluids.md`:283 | A | \| **`ResearchFluidSystem`** \| Research-grade SPH GPU pipeline supporting PCISPH/DF… |
| `docs/architecture/fluids.md`:336 | A | - **`SolverType` in `unified_solver.rs` vs `SolverType` in `research.rs`**: Two enum… |
| `docs/architecture/fluids.md`:339 | A | - **`FluidSystem` (`lib.rs:250-415+`) vs `UnifiedSolver` (`unified_solver.rs`) vs `R… |
| `docs/architecture/fluids.md`:404 | A | \| `astraweave-fluids/src/research.rs` \| 1,190 \| `ResearchFluidSystem` + `Research… |
| `docs/architecture/fluids.md`:412 | A | \| `astraweave-fluids/src/unified_solver.rs` \| 982 \| `UnifiedSolver` + `UnifiedSol… |
| `docs/architecture/fluids.md`:417 | A | \| `astraweave-fluids/src/warm_start.rs` \| 740 \| Warm-starting (reuse previous pre… |
| `docs/architecture/fluids.md`:446 | A | \| `UnifiedSolver` (high-level coordinator) \| `unified_solver.rs` (982 LoC) \| Acti… |
| `docs/architecture/fluids.md`:447 | A | \| `ResearchFluidSystem` (research-grade SPH GPU) \| `research.rs` (1,190 LoC) \| Ac… |
| `docs/architecture/fluids.md`:454 | A | - **`SolverType`**: In `unified_solver.rs:50-60`, has variants `Pbd / Pcisph / Dfsph… |
| `docs/architecture/fluids.md`:466 | B | - **Trap**: Reading `astraweave-fluids/README.md` (which advertises "production-grad… |
| `docs/architecture/fluids.md`:469 | A | **What's actually true**: That's the `unified_solver::SolverType` re-export at `lib.… |
| `docs/architecture/fluids.md`:470 | A | - **Trap**: Treating `FluidSystem`, `UnifiedSolver`, `ResearchFluidSystem`, and `PCI… |
| `docs/architecture/fluids.md`:491 | A | - **Decision:** Adopt multi-solver inventory (PBD/PCISPH/DFSPH/IISPH) with quality-t… |
| `docs/architecture/fluids.md`:493 | A | - **Consequences:** Three parallel solver implementations coexist (`FluidSystem` PBD… |
| `docs/architecture/fluids.md`:579 | A | \| 15 \| `SolverType::PBD::typical_iterations() == 4`, `PCISPH == 5`, `DFSPH == 3`, … |
| `docs/architecture/fluids.md`:580 | A | \| 16 \| `SolverType::supports_warm_start()` returns true only for PCISPH / DFSPH / … |
| `docs/architecture/fluids.md`:608 | A | - High (DFSPH): 200-500k particles @ 30-60 fps (hero fluids, AAA) |
| `docs/architecture/fluids.md`:609 | A | - Research (DFSPH+Implicit): 100-300k particles @ 15-30 fps (offline) |
| `docs/architecture/fluids.md`:658 | A | > **F.1 closures (2026-06-11):** "Runtime production wiring — when and via which sol… |
| `docs/architecture/fluids.md`:660 | A | - **Runtime production wiring of fluids — when and via which solver?** [Decisional /… |
| `docs/architecture/fluids.md`:661 | A | - **Five parallel solver/manager surfaces — consolidation roadmap?** [Decisional.] F… |
| `docs/architecture/fluids.md`:679 | A | \| 1.3 \| 2026-06-11 \| **F.1 revision** (§0): F.0 audit corrections (phantom `Resea… |
| `docs/architecture/fluids.md`:684 | A | - A new solver is added to the parallel inventory (`FluidSystem`/`UnifiedSolver`/`Re… |
| `docs/architecture/fluids.md`:689 | A | - The `examples/fluids_demo` consumer pattern changes (e.g. switches from `FluidSyst… |
| `docs/architecture/fluids.md`:710 | A | 2. **Five parallel solver/manager surfaces coexist.** `FluidSystem` (lib.rs PBD), `U… |
| `docs/architecture/fluids.md`:711 | A | 3. **`SolverType` naming collision:** `unified_solver::SolverType` has `Pbd/Pcisph/D… |
| `docs/architecture/fluids.md`:742 | A | - **Treating `FluidSystem` and `UnifiedSolver` as interchangeable.** They have diffe… |
| `docs/architecture/net.md`:17 | A | \| **Revision history** \| 1.2 (2026-05-12): Deep investigation pass. **Major new fi… |
| `docs/architecture/net.md`:327 | A | \| Architectural mismatch in same aspirational doc: claims **QUIC (via Quinn)** tran… |
| `docs/architecture/net_ecs.md`:16 | B | \| **Status** \| Active (mixed: production-grade standalone binary; dormant ECS Plug… |
| `docs/architecture/net_ecs.md`:32 | B | Per `net/README.md:1-12`, the goal was a "production-ready multiplayer capabilities"… |
| `docs/architecture/net_ecs.md`:405 | B | \| `connect_to_server` / `start_network_server` (ECS layer) vs. `aw-net-server`'s `h… |
| `docs/architecture/net_ecs.md`:451 | B | - **Date:** Standalone trio: 2025-09-09 commit `cc9a7e3e3` ("Implement production-re… |
| `docs/architecture/net_ecs.md`:453 | B | - **Context:** Per `net/README.md:90-92`, "This enhanced networking layer runs along… |
| `docs/architecture/net_ecs.md`:694 | B | The standalone trio (`net/aw-net-{proto,client,server}`) was introduced as a single … |
| `docs/architecture/persistence_ecs.md`:16 | B | \| **Status** \| Active (mixed maturity: file format + serialization + hashing are p… |
| `docs/architecture/persistence_ecs.md`:41 | B | The disk-format layer (`aw-save`) is fully production-grade and CI-covered. The ECS … |
| `docs/architecture/persistence_ecs.md`:409 | B | \| `astraweave-persistence-ecs/src/lib.rs` \| ECS Plugin + components + 3 pipeline f… |
| `docs/architecture/persistence_ecs.md`:423 | B | - **Active (mixed)**: Some parts production-grade, other parts stubbed. |
| `docs/architecture/persistence_ecs.md`:727 | B | 2. **`aw-save` is production-grade**: atomic writes, CRC32, LZ4, schema migration, W… |
| `docs/architecture/physics.md`:211 | A | - **`SpatialHash` (in-crate) vs `DefaultBroadPhase` (Rapier)**: The in-crate `Spatia… |
| `docs/architecture/physics.md`:259 | A | \| `astraweave-physics/src/spatial_hash.rs` (1,038 LoC) \| Standalone `SpatialHash<T… |
| `docs/architecture/physics.md`:313 | A | - **Trap**: Reading the crate doc-comment at `lib.rs:25-26` ("`SpatialHash` — Grid-b… |
| `docs/architecture/physics.md`:493 | A | - **`SpatialHash` module — wire in or remove?** [Decisional / factual, **enriched 20… |
| `docs/audits/COMPETITIVE_MATRIX.md`:43 | A | \| **LLM Integration** \| ❌ \| ❌ \| ❌ \| ❌ \| ✅ Ollama (phi3:medium default; Hermes … |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:70 | A | - **AI/LLM**: Ollama (runtime default `phi3:medium`); Hermes 2 Pro / Qwen3 supported… |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:107 | A | - **astraweave-llm**: Ollama (runtime default `phi3:medium`; Hermes 2 Pro opt-in via… |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:326 | B | - **Grade:** B+ (Good but not "WORLD-CLASS zero defects") |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:163 | A | - LLM integration: Ollama working (phi3:medium runtime default; Hermes 2 Pro/Qwen3 o… |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:168 | B | - "WORLD-CLASS" - **Good but not AAA-studio level** ⚠️ |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:326 | A | \| **Signing** \| Ed25519/RSA \| ✅ HMAC-SHA256 (sign16 deleted) \| ⭐⭐⭐⭐ \| |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:129 | B | - **Phase 1**: Editor + Scripting + Crash Reporting + CI/CD + Docs → 85% production-… |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:130 | B | - **Phase 2**: Mobile + Multiplayer + Visual Scripting + Asset Pipeline → 95% produc… |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:131 | B | - **Phase 3**: VR + Asset Store + Consoles + Cloud → 100% production-ready |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:3 | A | > **Superseded by F.1 (2026-06-12):** UnifiedSolver was deleted (`c3f19e31e`), DFSPH… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:63 | A | 3. `UnifiedSolver::step` — the crate's flagship-named, root-re-exported solver — **i… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:100 | A | #### C. `ResearchFluidSystem` — does not exist |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:102 | A | `research.rs` contains **zero wgpu code and no system struct**. It is a types/config… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:104 | A | #### D. `UnifiedSolver` (`unified_solver.rs`) — config shell with a no-op step |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:106 | A | No wgpu anywhere in the file. **`pub fn step(&mut self, _particles: &mut [ResearchPa… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:135 | A | \| C1 \| `UnifiedSolver::step` is a no-op (frame counter only); entire `UnifiedSolve… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:150 | A | **MED — dormant modules** (zero non-test production callers, all verified by caller … |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:178 | A | **Headline answer: NO test verifies end-to-end simulation correctness against a phys… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:196 | A | \| D1 \| `ResearchFluidSystem` inventoried as active research-grade GPU pipeline (fl… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:298 | A | **Exists**: working PBF (with Must-Fix #1-#3); real PCISPH pipeline with warm-start … |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:299 | A | **Missing**: everything that makes "research-grade" a checkable claim — a validation… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:450 | A | \| 4 \| **`UnifiedSolver::step` no-op** sold as the flagship re-export: delete, gate… |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`:466 | A | 3. **Solver consolidation authority**: may the campaign delete or `experimental`-gat… |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`:12 | A | \| `c3f19e31e` \| WI-4: solver consolidation (UnifiedSolver deleted, DFSPH/IISPH var… |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`:89 | A | **Deleted (vapor):** `unified_solver.rs` wholesale (982 LoC — `UnifiedSolver::step` … |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`:121 | A | 2. **`docs/architecture/fluids.md` v1.3** — new §0 "F.1 Revision Notice" (trace-erro… |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`:145 | A | \| `cargo check --workspace` \| `Finished ... in 11.55s` ✅ (only pre-existing deferr… |
| `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`:149 | A | Test-count delta accounting: pre-F.1 lib 2,480 → default 2,259 / experimental 2,448.… |
| `docs/current/ARCHITECTURE_REFERENCE.md`:189 | A | O(n log n) grid-based spatial partitioning in `astraweave-physics/src/spatial_hash.r… |
| `docs/current/FLUIDS_MUTATION_TESTING_REPORT.md`:7 | A | > `unified_solver.rs`, removed the `SolverType::DFSPH/IISPH` variants, gated |
| `docs/current/MASTER_COVERAGE_REPORT.md`:545 | A | \| 5.1.0 \| 2026-02-27 \| Audit \| Full audit: corrected test counts via `#[test]` m… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:13 | A | \| **Migration Context** \| Hermes 2 Pro → Qwen3-8B (local LLM via Ollama) \| |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:945 | A | Qwen3-8B is a validated opt-in local LLM backend, selectable via `OLLAMA_MODEL=qwen3… |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:188 | A | Hermes 2 Pro wins aggregated TTFC (184ms vs 274ms avg), but **Qwen3 achieves 160ms a… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1 | A | # AstraWeave LLM Migration Plan: Hermes 2 Pro → Qwen3-8B |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:60 | A | - **Module name**: `hermes2pro_ollama` → `qwen3_ollama` |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:62 | A | - **Model string**: `adrienbrault/nous-hermes2pro:Q4_K_M` → `qwen3:8b` |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:132 | A | \| `hermes2pro_ollama.rs` (→ `qwen3_ollama.rs`) \| **HIGH** — Rename + modify \| Med… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:168 | A | └─ Rename hermes2pro_ollama.rs → qwen3_ollama.rs |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:212 | A | Rename `astraweave-llm/src/hermes2pro_ollama.rs` → `astraweave-llm/src/qwen3_ollama.… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:867 | A | - `docs/archive/HERMES2PRO_*.md` (~10 files) — These document the Phi-3 → Hermes mig… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:914 | A | If Qwen3-8B non-thinking mode doesn't meet the ≥87% overall target, the strategic ex… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1055 | A | \| `astraweave-llm/src/hermes2pro_ollama.rs` → `qwen3_ollama.rs` \| **Rename + major… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1073 | A | \| `examples/llm_modelfiles/Modelfile.hermes2pro-game` → `Modelfile.qwen3-game` \| R… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1074 | A | \| `examples/llm_modelfiles/Modelfile.hermes2pro-fast` → `Modelfile.qwen3-fast` \| R… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1091 | A | Module:   hermes2pro_ollama            → qwen3_ollama |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1093 | A | Model:    adrienbrault/nous-hermes2pro:Q4_K_M → qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1094 | A | Pull:     ollama pull adrienbrault/... → ollama pull qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1101 | A | Label:    "Hermes 2 Pro"               → "Qwen3-8B" |
| `docs/current/RENDERING_GAPS_ANALYSIS_AND_FIX_PLAN.md`:98 | B | - Visual quality below AAA standards |
| `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md`:532 | A | - **§1.2.3 aw_asset_cli pipeline**: 3 subcommands (cook, bake-texture, validate). Co… |
| `docs/guides/networking_envelopes.md`:14 | A | - WebSockets (tokio-tungstenite over TCP); QUIC/UDP not implemented |
| `docs/lessons/AI_ORCHESTRATION_TIPS.md`:377 | A | - **Phase 6-7**: LLM selection decision tree (Phi-3 → Hermes 2 Pro) |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:20 | A | - **Spatial hash**: 99.96% collision reduction (499,500 → 180 checks) |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:315 | A | - 99.96% fewer checks (499,500 → 180 at 1,000 entities) |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:321 | A | - **Collision reduction**: 499,500 → 180 checks (99.96%) |
| `docs/lessons/WHAT_DIDNT.md`:40 | A | - **Phase 7**: Pivoted to Hermes 2 Pro → 75-85% success rate |
| `docs/lessons/WHAT_DIDNT.md`:44 | A | **What worked instead**: Hermes 2 Pro (adrienbrault/nous-hermes2pro:Q4_K_M) as an op… |
| `docs/lessons/WHAT_DIDNT.md`:374 | A | 2. ❌ Phi-3 LLM → ✅ Hermes 2 Pro |
| `docs/lessons/WHAT_WORKED.md`:165 | A | - **75-85% LLM success rate** (runtime default is phi3:medium; Hermes 2 Pro is opt-i… |
| `docs/lessons/WHAT_WORKED.md`:302 | A | - **Spatial hash**: present but DORMANT — the live broadphase is Rapier `DefaultBroa… |
| `docs/masters/MASTER_ROADMAP.md`:471 | A | - **Corrected total tests**: 7,600+ → ~27,000+ measured / ~35,000 `#[test]` markers |
| `docs/pbr/PBR_F_DESIGN.md`:772 | B | **Status**: Historical design document. Describes a superseded 4-layer terrain schem… |
| `docs/src/core-systems/networking.md`:6 | A | Copilot bot as part of commit 28bc94f21) described a QUIC-based transport with |
| `docs/src/core-systems/networking.md`:31 | A | * **Wire format:** JSON over WebSocket text frames via `tokio-tungstenite`. **Not QU… |
| `docs/src/core-systems/networking.md`:32 | A | There is no `quinn` dependency in the workspace. |
| `docs/src/core-systems/physics.md`:9 | A | "99.96% pair reduction"; the actual broadphase is Rapier's DefaultBroadPhase. |
| `gh-pages/ai.md`:37 | A | \| 4 \| LLM \| Ollama (default `phi3:medium`; Qwen3-8B opt-in) \| 2–8 s \| Creative,… |
| `gh-pages/architecture.md`:73 | A | \| LLM \| Ollama (default `phi3:medium`; Qwen3-8B opt-in) \| 2–8 s \| Creative, emer… |
| `gh-pages/index.md`:37 | A | AstraWeave's AI-native architecture treats every NPC as an intelligent agent running… |
| `gh-pages/index.md`:106 | A | - **Optional**: Ollama for LLM features (runtime default `phi3:medium`; Qwen3-8B opt… |
| `gh-pages/physics.md`:39 | A | - **99.96% collision pair reduction** (499,500 → 180 checks for 1,000 entities) |
| `gh-pages/setup.md`:84 | A | 2. Pull a model. The runtime default is `phi3:medium`; Qwen3-8B is opt-in via `OLLAM… |
| `gh-pages/setup.md`:87 | A | ollama pull qwen3:8b         # opt-in: set OLLAMA_MODEL=qwen3:8b |
| `tools/ASSET_SIGNING_DESIGN.md`:1499 | A | **A**: No. GPG uses different signature formats (OpenPGP) and key types (RSA, DSA, E… |

### `subject-doc` (86) — permanent

*The doc is about that subject (QWEN3_* about Qwen3; temperature guide about Hermes tuning)*

| File:line | V | Match |
|---|:-:|---|
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:1 | A | # AstraWeave Qwen3-8B Migration — Comprehensive Benchmark Report |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:3 | A | > **Version**: 1.0 \| **Date**: 2026-02-27 \| **Model**: Qwen3-8B (Q4_K_M) via Ollam… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:20 | A | The Qwen3-8B migration introduces **zero performance regressions** to the engine. Al… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:35 | A | \| **Ollama** \| v0.17+ with `qwen3:8b` (5.2 GB, Q4_K_M quantization) \| |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:121 | A | These benchmarks use **mock LLM clients** for deterministic, reproducible performanc… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:166 | A | ## 3. Live Qwen3-8B Inference Tests |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:168 | A | Four integration tests hit the live Ollama API with the `qwen3:8b` model. All four p… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:206 | A | The 3 `hermes2pro_ollama` tests expectedly failed (model not installed). This confir… |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:937 | A | The Qwen3-8B migration is **performance-neutral** across the entire engine: |
| `docs/current/QWEN3_BENCHMARK_REPORT.md`:942 | A | 4. **Live inference confirmed** — Qwen3-8B produces valid JSON tactical plans via bo… |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:1 | A | # Qwen3-8B Latency Optimization Report |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:6 | A | **Ollama**: v0.9+, models: `qwen3:8b` (5.2GB Q4_K_M), `adrienbrault/nous-hermes2pro:… |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:12 | A | Four rounds of bespoke optimizations transformed Qwen3-8B from **1.84× slower** than… |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:20 | A | \| Metric \| Qwen3-8B \| Hermes 2 Pro \| Winner \| Speedup \| |
| `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md`:30 | A | **Verdict: Qwen3-8B is FASTER than Hermes 2 Pro** |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:6 | A | **Scope**: Full engine migration from Nous Hermes 2 Pro (Mistral 7B) to Qwen3-8B |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:33 | A | Qwen3-8B is a significant upgrade over Hermes 2 Pro (Mistral 7B) for AstraWeave's AI… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:35 | A | \| Dimension \| Hermes 2 Pro (Mistral 7B) \| Qwen3-8B \| Improvement \| |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:45 | A | \| **Ollama Support** \| `adrienbrault/nous-hermes2pro:Q4_K_M` \| `qwen3:8b` (offici… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:73 | A | ### Qwen3-8B Technical Specifications |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:76 | A | Model:          Qwen3-8B (post-trained: pretraining + SFT + RLHF) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:113 | A | **However**, for AstraWeave's plan-based architecture (where we want a full JSON pla… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:117 | A | Qwen3-8B uniquely supports dual-mode operation: |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:119 | A | - **Non-thinking mode** (`/no_think` or `enable_thinking=False`): Behaves like Herme… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:193 | A | └─ Update all P0 docs referencing Hermes 2 Pro |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:198 | A | └─ Run test suite with Qwen3-8B (640 unit + 4 integration + 275 AI = ALL PASS) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:220 | A | pub model: String,        // "adrienbrault/nous-hermes2pro:Q4_K_M" |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:229 | A | pub model: String,        // "qwen3:8b" |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:244 | A | Self::new("http://localhost:11434", "adrienbrault/nous-hermes2pro:Q4_K_M") |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:247 | A | Self::new("http://localhost:11434", "adrienbrault/nous-hermes2pro:Q4_K_M") |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:256 | A | Self::new("http://localhost:11434", "qwen3:8b") |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:259 | A | Self::new("http://localhost:11434", "qwen3:8b") |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:267 | A | Self::new("http://localhost:11434", "qwen3:8b") |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:287 | A | "num_ctx": 8192,  // Hermes 2 Pro: 8K context |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:299 | A | "num_ctx": self.context_length,  // Qwen3-8B: 32K default |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:382 | A | pub mod hermes2pro_ollama; |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:581 | A | "model": "qwen3:8b", |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:589 | A | "model": "qwen3:8b", |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:598 | A | "model": "qwen3:8b", |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:627 | A | // OLD (Hermes 2 Pro oriented) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:631 | A | // NEW (Qwen3-8B oriented) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:677 | A | FROM qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:679 | A | # Gaming-optimized Qwen3-8B configuration |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:708 | A | FROM qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:710 | A | # Low-latency Qwen3-8B configuration |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:729 | A | FROM qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:731 | A | # Strategic planning Qwen3-8B configuration |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:770 | A | ollama pull adrienbrault/nous-hermes2pro:Q4_K_M    # 4.4GB |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:773 | A | ollama pull qwen3:8b                                # ~5GB (Q4_K_M default) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:775 | A | ollama pull qwen3:8b-q8_0                          # ~8.5GB (Q8_0) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:782 | A | ASTRAWEAVE_OLLAMA_MODEL=adrienbrault/nous-hermes2pro:Q4_K_M |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:785 | A | ASTRAWEAVE_OLLAMA_MODEL=qwen3:8b |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:790 | A | \| Quantization \| VRAM (Qwen3-8B) \| VRAM (Hermes 2 Pro) \| Notes \| |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:796 | A | **Impact**: Users with 6 GB VRAM GPUs can run Q4_K_M comfortably. Users with 8 GB+ (… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:806 | A | use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama; |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:830 | A | println!("  ║        AstraWeave  ·  Hermes 2 Pro  ·  60 FPS      ║"); |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:831 | A | println!("🧠 LLM AI (Hermes 2 Pro via Ollama)"); |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:834 | A | println!("  ║        AstraWeave  ·  Qwen3-8B  ·  60 FPS          ║"); |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:835 | A | println!("🧠 LLM AI (Qwen3-8B via Ollama)"); |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:885 | A | # Step 1: Run baseline with Hermes 2 Pro (BEFORE migration) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:886 | A | ASTRAWEAVE_OLLAMA_MODEL=adrienbrault/nous-hermes2pro:Q4_K_M \ |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:888 | A | --scenarios all --runs 3 --output results/hermes2pro_baseline.json |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:892 | A | # Step 3: Run comparison with Qwen3-8B (AFTER migration) |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:893 | A | ASTRAWEAVE_OLLAMA_MODEL=qwen3:8b \ |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:898 | A | ASTRAWEAVE_OLLAMA_MODEL=qwen3:8b \ |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:906 | A | \| Metric \| Hermes 2 Pro Baseline \| Qwen3-8B Target (non-think) \| Qwen3-8B Target… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:934 | A | 1. **Baseline**: Run `astraweave-llm-eval` against Hermes 2 Pro — record all four su… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:939 | A | 6. **Eval Comparison**: Run `astraweave-llm-eval` against Qwen3-8B (non-thinking + t… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:943 | A | 10. **Regression**: Compare all metrics against Hermes 2 Pro baseline |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:966 | A | Hermes 2 Pro: |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:973 | A | Qwen3-8B: |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:999 | A | \| Aspect \| Hermes 2 Pro \| Qwen3-8B \| |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1005 | A | **Impact**: The larger vocabulary means Qwen3 is more token-efficient for the same t… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1036 | A | \| Temperature sensitivity differs from Hermes 2 Pro \| Run temperature experiment b… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1044 | A | \| Qwen3 regression on JSON structured output \| Extensive testing in Phase 6; fallb… |
| `docs/current/QWEN3_MIGRATION_PLAN.md`:1076 | A | \| `scripts/test_hermes2pro_validation.ps1` \| Rename + update \| |
| `scripts/temperature_experiment_guide.md`:1 | A | # Temperature Experiment Guide - Hermes 2 Pro Optimization |
| `scripts/temperature_experiment_guide.md`:11 | A | Determine optimal temperature configuration for Hermes 2 Pro in AstraWeave productio… |
| `scripts/temperature_experiment_guide.md`:76 | A | .\test_hermes2pro_validation.ps1 -Iterations 10 -OutputFile "hermes2pro_temp_0.3.csv" |
| `scripts/temperature_experiment_guide.md`:83 | A | # CSV saved to: scripts\hermes2pro_temp_0.3.csv |
| `scripts/temperature_experiment_guide.md`:90 | A | .\test_hermes2pro_validation.ps1 -Iterations 10 -OutputFile "hermes2pro_temp_0.7.csv" |
| `scripts/temperature_experiment_guide.md`:120 | A | - `scripts/hermes2pro_temp_0.3.csv` (10 runs) |
| `scripts/temperature_experiment_guide.md`:121 | A | - `scripts/hermes2pro_extended_validation.csv` (20 runs @ temp 0.5) |
| `scripts/temperature_experiment_guide.md`:122 | A | - `scripts/hermes2pro_temp_0.7.csv` (10 runs) |
| `scripts/temperature_experiment_guide.md`:155 | A | - Expected: All temperatures score 4-5 (Hermes 2 Pro is trained for reasoning) |
| `scripts/temperature_experiment_guide.md`:283 | A | 5. **Production Deployment** - Ship Hermes 2 Pro for turn-based games |

### `competitor-cited` (63) — permanent

*A competitor figure/comparison with a named competitor (comparison docs, industry-standard columns)*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:15 | B | - ⚠️ **FALLS SHORT** in editor/ecosystem (broken vs Unity/Unreal world-class) |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:88 | B | **Comparison**: Matches Unity HDRP, exceeds Godot 4, slightly behind Unreal 5 |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:97 | B | **Comparison**: Matches Bevy architecture, exceeds Unity GameObject model |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:106 | B | **Comparison**: Matches Unity/Godot (Rapier3D standard), below Unreal PhysX |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:114 | B | - **Gap**: Unreal/Unity have world-class editors (100/100 vs 0/100) |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`:219 | B | - ✅ **Rendering quality** (matches Unity HDRP) |
| `docs/audits/COMPETITIVE_MATRIX.md`:68 | B | **Verdict**: AstraWeave **matches Unity HDRP** in feature parity, **and Godot/Bevy**. |
| `docs/audits/COMPETITIVE_MATRIX.md`:84 | B | **Verdict**: AstraWeave builds on Rapier3D (a production-ready physics backend). |
| `docs/audits/COMPETITIVE_MATRIX.md`:92 | B | \| **Editor** \| ✅ World-class \| ✅ World-class \| ✅ Excellent \| ⚠️ Third-party \| … |
| `docs/audits/COMPETITIVE_MATRIX.md`:184 | A | \| **Network Encryption** \| ✅ TLS 1.3 \| ✅ TLS 1.3 \| ✅ TLS 1.3 \| ✅ \| ❌ WebSocket… |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:13 | B | This comprehensive audit evaluated the AstraWeave AI-Native Gaming Engine across sev… |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:334 | A | - ✅ Ed25519 cryptographic signing (asset_signing tool) |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:372 | A | \| **Cryptography** \| 9/10 \| A \| Ed25519, SHA-256, ChaCha20 strong \| |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:179 | B | **Verdict**: AstraWeave **matches Bevy** in entity capacity (100k+ range), **exceeds… |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:318 | A | 3. **Encryption**: TLS 1.3 for network, AES-256 for saves |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:325 | A | \| **Network Encryption** \| TLS 1.3 \| ⚠️ WebSocket (tokio-tungstenite), no TLS ter… |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:482 | B | 5. **Rendering Quality**: broad PBR feature set (matches Unity HDRP, exceeds Godot 4) |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:486 | B | 1. **Physics**: Rapier3D integration (matches Unity/Godot, below Unreal) |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:492 | B | 1. **Editor**: Non-functional (vs Unreal/Unity/Godot world-class editors) |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:634 | B | \| **Architecture** \| 98/100 \| A+ \| ECS design exceeds Bevy, determinism unique \| |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:666 | B | 1. Editor (broken vs Unreal/Unity world-class) |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:698 | B | 2. ✅ **Rendering**: broad PBR feature set (matches Unity HDRP, exceeds Godot 4) |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:92 | B | - **Rendering**: AstraWeave **4.2k-5k draw calls** (matches Unity HDRP) |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:177 | B | 5. **Rendering**: PBR features (matches Unity HDRP, exceeds Godot 4) |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:226 | B | - ✅ **Rendering quality** (matches Unity HDRP) |
| `docs/audits/GAP_ANALYSIS_ACTION_PLAN.md`:33 | B | **Gap**: Unreal/Unity have world-class editors (100/100 vs 0/100) |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:896 | B | The **good news**: All missing features can be built on the existing Rapier3D founda… |
| `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`:517 | B | **Inference for AstraWeave's purposes**: rerun confirms egui's base pattern scales t… |
| `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`:532 | B | - Among **mature** Rust 3D editors (only Fyrox at production-grade as of 2026): trai… |
| `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`:853 | B | - **Approach II (trait-object collection)** scores highest on Rust+egui ecosystem-fi… |
| `docs/audits/g_pointer_events_research_2026-05-03.md`:320 | B | rerun's [`re_viewer`](https://github.com/rerun-io/rerun) is a substantial production… |
| `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md`:136 | A | - Signs manifest with Ed25519 (asset_signing::KeyStore). |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md`:1893 | B | 1. **Builds on Solid Foundation**: egui 0.32, Camera, ECS, Input all production-ready |
| `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`:135 | B | Per research audit §7.7 Approach I+II hybrid synthesis: registry/manager owns trait-… |
| `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`:263 | B | **Rationale**: Fyrox's surface is the production-grade Rust + egui canonical referen… |
| `docs/current/GAME_ENGINE_READINESS_ROADMAP.md`:216 | A | 1. Networking library integration (e.g., `bevy_renet`, `laminar`, `quinn` for QUIC) |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:1420 | B | \| **Phase C: Polish** \| 7-12 \| RAG integration, editor stability, community launc… |
| `docs/current/MUTATION_TESTING_REMEDIATION_REPORT.md`:473 | A | - Ed25519 signature verification |
| `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`:1782 | B | 2. **Approach II — Trait-object collection with iteration-based dispatch**: trait + … |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md`:4 | B | **Mission**: Build production-ready, UE5-tier rendering system with ZERO deferrals |
| `docs/current/SECURITY_AUDIT_AND_HARDENING_PLAN.md`:97 | A | - Introduce signed session tokens (JWT or Ed25519) issued by the matchmaking service… |
| `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md`:197 | A | **Decision**: cook pipeline's Ed25519 manifest signing preserved unchanged. Sub-phas… |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:1711 | B | **Competitive Position**: AstraWeave matches Unity/Unreal in core editor functionali… |
| `docs/pbr/PBR_E_DESIGN.md`:493 | B | - ✅ Visual quality matches UE5/Unity HDRP reference images |
| `tools/ASSET_SIGNING_DESIGN.md`:5 | A | This document provides a design for upgrading AstraWeave's asset signing system from… |
| `tools/ASSET_SIGNING_DESIGN.md`:7 | A | **Current State**: Ephemeral Ed25519 keys generated per-run (tools/aw_asset_cli/src/… |
| `tools/ASSET_SIGNING_DESIGN.md`:159 | A | let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng) |
| `tools/ASSET_SIGNING_DESIGN.md`:161 | A | let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()) |
| `tools/ASSET_SIGNING_DESIGN.md`:201 | A | 1. **Ed25519 Choice**: Modern, fast, secure signature algorithm |
| `tools/ASSET_SIGNING_DESIGN.md`:215 | A | │  root.key (Ed25519, offline, encrypted, 10-year validity)   │ |
| `tools/ASSET_SIGNING_DESIGN.md`:664 | A | **Performance**: Ed25519 verification is ~50,000 ops/sec on modern CPU; negligible o… |
| `tools/ASSET_SIGNING_DESIGN.md`:1074 | A | - Ed25519 signature detects any modification |
| `tools/ASSET_SIGNING_DESIGN.md`:1111 | A | - ❌ Post-quantum attacks (Ed25519 vulnerable to quantum computers; mitigated by TUF'… |
| `tools/ASSET_SIGNING_DESIGN.md`:1442 | A | - Why Ed25519 over RSA/ECDSA? |
| `tools/ASSET_SIGNING_DESIGN.md`:1475 | A | \| **Custom Ed25519** \| Low \| N/A \| Excellent \| Simple, greenfield \| |
| `tools/ASSET_SIGNING_DESIGN.md`:1477 | A | **Recommendation**: **Custom Ed25519 with TUF-inspired hierarchy** |
| `tools/ASSET_SIGNING_DESIGN.md`:1484 | A | ### Q: Why Ed25519 instead of RSA? |
| `tools/ASSET_SIGNING_DESIGN.md`:1486 | A | **A**: Ed25519 is modern (2011), faster (50k verifications/sec vs 10k for RSA-2048),… |
| `tools/ASSET_SIGNING_DESIGN.md`:1510 | A | # Ed25519 verification is not directly supported by openssl CLI |
| `tools/ASSET_SIGNING_DESIGN.md`:1521 | A | **A**: Negligible. Ed25519 verification: |
| `tools/ASSET_SIGNING_DESIGN.md`:1569 | A | 6. **Ed25519 Paper** (Bernstein et al.): https://ed25519.cr.yp.to |
| `tools/ASSET_SIGNING_DESIGN.md`:1604 | A | pub signature: String,        // base64-encoded Ed25519 signature |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md`:13 | B | The AstraWeave visual editor (`tools/aw_editor/`) has been subjected to a multi-agen… |

### `production-status-contested` (58) — pending-D2 (re-evaluate after D.2 resolves the number)

*production-ready/grade on a contested surface — kept now; re-evaluated at D.2*

| File:line | V | Match |
|---|:-:|---|
| `docs/_audit/discovery-report.md`:30 | B | ### 1.1 Core Engine Crates (Production-Ready) |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:901 | B | **Result**: Production-ready testing (coverage A+) |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:309 | B | - ✅ Zero warnings, production-ready |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:108 | B | + Production-Ready (Phases 1-8 COMPLETE: 36 tasks, AAA features, 3 minor TODOs) |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:668 | B | ### ✅ Complete (Production-Ready) |
| `docs/audits/TEST_SUITE_COMPREHENSIVE_AUDIT.md`:460 | B | With an estimated **125-160 hours** of focused effort over **8 weeks**, the test sui… |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md`:103 | B | **Status**: ✅ Production-ready bidirectional mapping |
| `docs/current/AW_EDITOR_CORRECTNESS_AUDIT_REPORT.md`:74 | B | **Assessment**: Production-ready with comprehensive design. |
| `docs/current/AW_EDITOR_CORRECTNESS_AUDIT_REPORT.md`:403 | B | *This audit was conducted by GitHub Copilot as part of the AstraWeave AI-Native Game… |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md`:120 | B | **Pattern observed**: Most panics are `expect()` calls on setup operations. While ac… |
| `docs/current/BLEND_IMPORT_INTEGRATION_COMPLETE.md`:27 | B | A production-grade crate with: |
| `docs/current/BLEND_IMPORT_INTEGRATION_COMPLETE.md`:333 | B | - ✅ **Production-ready** with caching, progress, cancellation |
| `docs/current/ECS_MIRI_VALIDATION_REPORT.md`:254 | B | - Production-ready status confirmed |
| `docs/current/ECS_REGRESSION_SESSION_COMPLETE.md`:44 | B | BlobVec **already exists** and is production-ready (626 LOC, fully tested). The fix … |
| `docs/current/ECS_REGRESSION_SESSION_COMPLETE.md`:66 | B | - `blob_vec.rs` - Found production-ready alternative (unused!) |
| `docs/current/EDITOR_STATUS_REPORT.md`:5 | B | **Overall Completion:** 85% (Production-Ready for Core Features) |
| `docs/current/EDITOR_STATUS_REPORT.md`:282 | B | **Status:** Editor is production-ready pending Phase 1 completion |
| `docs/current/GAME_ENGINE_READINESS_ROADMAP.md`:6 | B | **Objective**: Transform AstraWeave from "production-ready infrastructure" to "ship … |
| `docs/current/GAME_ENGINE_READINESS_ROADMAP.md`:18 | B | **✅ Production-Ready**: |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:15 | B | - **Missing Features**: 12/12 production-grade features not implemented |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:448 | B | **Priority**: Production-grade enhancements |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:690 | B | **After Phase 0-1**: B+ (Production-ready) |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:724 | B | This plan transforms AstraWeave's GitHub Pages from a basic mdBook deployment with c… |
| `docs/current/IMPLEMENTATION_PLANS_INDEX.md`:10 | B | This directory contains three planning documents that form a roadmap for transformin… |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:5 | B | **Goal**: Transform from "compiles cleanly" to "production-ready AI-native game engi… |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:700 | B | **Deliverable**: Comprehensive production-ready documentation |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:1375 | B | **Deliverable**: Production-ready documentation and launch materials |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:1384 | B | 1. **Blog Post**: "AstraWeave v1.0: Production-Ready AI-Native Game Engine" |
| `docs/current/LONG_HORIZON_STRATEGIC_PLAN.md`:1462 | B | - Blog post: "AstraWeave v1.0: Production-Ready AI-Native Game Engine" |
| `docs/current/PHASE_8_ROADMAP_REVIEW.md`:60 | B | - ⚠️ BUT: Features gated by `#[cfg(feature = "postfx")]`, not production-ready |
| `docs/current/PHASE_8_ROADMAP_REVIEW.md`:206 | B | - AudioEngine architecture is production-ready |
| `docs/current/PHASE_8_ROADMAP_REVIEW.md`:274 | B | **Deliverable**: Production-ready Phase 8 codebase |
| `docs/current/PHASE_8_ROADMAP_REVIEW.md`:386 | B | - Higher confidence in production-ready output |
| `docs/current/PROJECT_STATUS.md`:59 | B | - **Mission**: Transform from "production-ready infrastructure" to "ship a game on i… |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md`:33 | B | - ✅ GPU-driven culling: Task 3 complete, production-ready |
| `docs/current/RENDERING_QUICK_REFERENCE.md`:224 | B | - ✅ Production-ready quality |
| `docs/current/TERRAIN_SCATTER_FIX_PLAN.md`:91 | B | The rendering engine has production-ready systems that are simply turned off in `Edi… |
| `docs/current/VEILWEAVER_VERTICAL_SLICE_ANALYSIS.md`:32 | B | \| **Core Engine Systems** \| ✅ Production-ready \| 100% \| |
| `docs/current/VEILWEAVER_VERTICAL_SLICE_ANALYSIS.md`:408 | B | ### Production-Ready Slice (Phases 1–5) — ✅ COMPLETE |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:1489 | B | ### Tier 3: Professional (Production-ready) |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:256 | B | - **Week 5**: Math infrastructure using glam (production-ready) |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:375 | B | - **astraweave-gameplay**: perform_attack_sweep (production-ready) |
| `docs/lessons/WHAT_WORKED.md`:196 | B | - **Week 1**: GPU skinning production-ready |
| `docs/lessons/WHAT_WORKED.md`:319 | B | - **Week 5**: Math infrastructure using glam (production-ready) |
| `docs/masters/MASTER_ROADMAP.md`:71 | B | - ❌ A fully production-ready game engine |
| `docs/masters/MASTER_ROADMAP.md`:202 | B | **Objective**: Transform from "production-ready infrastructure" to "ship a game on i… |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`:181 | B | ### Q: Is this production-ready? |
| `docs/reference/RENDERING_SOTA_REFERENCE.md`:436 | B | Hierarchical resolution/angular resolution tradeoffs. Multiple cascades store radian… |
| `docs/src/resources/faq.md`:21 | B | ### Is AstraWeave production-ready? |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md`:427 | B | **Goal**: Production-grade stability and testing |
| `tools/aw_editor/INTEGRATION_ROADMAP.md`:198 | B | **Goal**: Make editor production-ready |
| `tools/aw_editor/PHASE_2_COMPLETION_SUMMARY.md`:10 | B | Phase 2 added **complete undo/redo**, **scene persistence**, **extensible component … |
| `tools/aw_editor/PHASE_2_COMPLETION_SUMMARY.md`:349 | B | **Quality**: Phase 2 features complete with 90% test coverage (editor not yet produc… |
| `tools/aw_editor/PRODUCTION_READINESS_AUDIT.md`:337 | B | - **Scene serialization** is production-ready (RON format with versioning) |
| `tools/aw_editor/VIEWPORT_ENHANCEMENT_COMPLETE.md`:6 | B | **Grade**: ⭐⭐⭐⭐⭐ A+ (viewport milestone; editor not yet production-ready overall) |
| `tools/aw_editor/VIEWPORT_ENHANCEMENT_COMPLETE.md`:379 | B | **Status**: ✅ **VIEWPORT MILESTONE COMPLETE** (editor not yet production-ready overa… |
| `tools/aw_editor/WORLD_CLASS_EDITOR_PLAN.md`:7 | B | **Objective**: Transform aw_editor into a fully production-ready game editor |
| `tools/aw_editor/WORLD_CLASS_EDITOR_PLAN.md`:474 | B | *This plan represents the critical path to a production-ready game editor capable of… |

### `subject-doc-roadmap` (42) — permanent

*Fluids research-plan describing DFSPH/PCISPH/etc. as roadmap solver options*

| File:line | V | Match |
|---|:-:|---|
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:25 | A | - **SPlisHSPlasH** (RWTH Aachen): Research SPH with DFSPH, IISPH, PCISPH, implicit v… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:36 | A | \| **Solver Accuracy** \| B (PBD) \| A+ (DFSPH/PCISPH) \| Medium \| Add PCISPH optio… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:54 | A | \| **High** \| 200-500k \| DFSPH \| 30-60 fps \| Hero fluids, AAA \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:55 | A | \| **Research** \| 100-300k \| DFSPH+Implicit \| 15-30 fps \| Validation, offline \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:58 | A | > ⚠️ **Note**: Previous 1M+ target with full DFSPH was overly optimistic. Realistic … |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:140 | A | **Research Target**: DFSPH or IISPH for <0.1% density error |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:183 | A | **Goal**: Upgrade from PBD to DFSPH/PCISPH with stability enhancements |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:185 | A | #### 1.1 DFSPH Implementation (Divergence-Free SPH) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:223 | A | **Why Add This**: Often faster convergence than DFSPH in real-time scenarios, simple… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:229 | A | DFSPH,    // Divergence-Free (accurate) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:230 | A | IISPH,    // Implicit (most stable, slowest) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:248 | A | **Problem Solved**: Standard SPH suffers from **tensile instability** (particle clum… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:902 | A | \| **P0** \| DFSPH/PCISPH Solver \| 4-5 weeks \| ⭐⭐⭐⭐⭐ \| None \| Start with PCISPH … |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:903 | A | \| **P0** \| δ-SPH Particle Shifting \| 1 week \| ⭐⭐⭐⭐⭐ \| DFSPH \| **Critical for s… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:904 | A | \| **P0** \| Warm-Starting \| 0.5 week \| ⭐⭐⭐⭐ \| DFSPH \| 70-90% fewer iterations \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:905 | A | \| **P0** \| Matrix-Free Implicit Viscosity \| 2-3 weeks \| ⭐⭐⭐⭐⭐ \| DFSPH \| Replac… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:907 | A | \| **P1** \| Validation Suite \| 2 weeks \| ⭐⭐⭐⭐⭐ \| DFSPH \| Research credibility \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:914 | A | \| **P2** \| Vorticity Confinement \| 1 week \| ⭐⭐⭐⭐ \| DFSPH \| **Critical for visu… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:940 | A | └─ Week 6:     DFSPH Upgrade (optional, if PCISPH insufficient) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:989 | A | // DFSPH (new) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:993 | A | pub density_derivative: f32,      // Dρ/Dt for DFSPH |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1074 | A | pub solver: SolverType,  // PBD, DFSPH, IISPH |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1116 | A | PCISPH,  // NEW: Balanced, simpler than DFSPH |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1117 | A | DFSPH,   // Accurate (AAA games, pre-viz) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1118 | A | IISPH,   // Most stable (research, VFX) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1156 | A | \| DFSPH \| 2-3 + 1-2 \| ~4.5ms \| 144 bytes \| Excellent \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1157 | A | \| DFSPH + δ-SPH \| 2-3 + 1-2 + 1 \| ~5ms \| 176 bytes \| Best \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1158 | A | \| IISPH \| 10-50 Jacobi \| ~8ms \| 144 bytes \| Excellent \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1160 | A | > ⚠️ **Note**: With warm-starting, DFSPH often converges in 1-2 iterations instead o… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1167 | A | \| High (PC) \| 200-350k \| 60 \| DFSPH \| Full δ-SPH, vorticity \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1168 | A | \| Ultra (PC) \| 350-500k \| 30 \| DFSPH \| Multi-phase, micropolar \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1169 | A | \| Research \| 500k-1M \| Offline \| DFSPH/IISPH \| All features + VTK export \| |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1189 | A | // DFSPH specific: smaller groups for divergence solve |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1259 | A | 1. **DFSPH**: Bender & Koschier, "Divergence-Free Smoothed Particle Hydrodynamics" (… |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1261 | A | 3. **IISPH**: Ihmsen et al., "Implicit Incompressible SPH" (2014) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1276 | A | - Implements DFSPH, IISPH, all viscosity models, particle shifting |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1285 | A | - DFSPH solver, boundary handling |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1291 | A | 3. Bender & Koschier 2017 (DFSPH details) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1297 | A | 9. Ihmsen 2014 (IISPH for comparison) |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1320 | A | 1. **PCISPH Solver** - Simpler alternative to DFSPH |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1345 | A | 1. **Incremental adoption**: Keep PBD as fallback, add PCISPH/DFSPH as options |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:1346 | A | 2. **Performance scaling**: LOD between PBD/DFSPH based on importance, warm-starting… |

### `false-positive-QUICK` (40) — permanent

*"QUIC" regex matched "QUICK" (Quick Reference / QUICKSTART)*

| File:line | V | Match |
|---|:-:|---|
| `astraweave-llm/README.md`:237 | A | - [Build Quick Reference](../docs/BUILD_QUICK_REFERENCE.md) |
| `docs/README.md`:28 | A | **[→ QUICKSTART.md](QUICKSTART.md)** - Complete setup and first steps guide |
| `docs/README.md`:125 | A | \| Reference Docs \| 8 \| `RENDERING_QUICK_REFERENCE.md`, `AW_EDITOR_QUICK_REFERENCE… |
| `docs/README.md`:146 | A | \| Build & Setup \| 4 \| `BUILD_QUICK_REFERENCE.md`, `RUST_TOOLCHAIN_GUIDE.md` \| |
| `docs/README.md`:147 | A | \| Assets \| 6 \| `assets_pipeline.md`, `POLYHAVEN_QUICK_START.md` \| |
| `docs/README.md`:303 | A | - **[QUICKSTART.md](QUICKSTART.md)** - 5-minute setup guide |
| `docs/README.md`:307 | A | - **[guides/BUILD_QUICK_REFERENCE.md](guides/BUILD_QUICK_REFERENCE.md)** - Build com… |
| `docs/README.md`:357 | A | - **[QUICKSTART.md](QUICKSTART.md)** - Complete setup guide |
| `docs/README.md`:358 | A | - **[guides/BUILD_QUICK_REFERENCE.md](guides/BUILD_QUICK_REFERENCE.md)** - Build com… |
| `docs/README.md`:363 | A | - **[guides/POLYHAVEN_QUICK_START.md](guides/POLYHAVEN_QUICK_START.md)** - PBR assets |
| `docs/README.md`:373 | A | - **[guides/SECURITY_QUICK_REFERENCE.md](guides/SECURITY_QUICK_REFERENCE.md)** - Qui… |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:380 | A | - `/docs/QUICKSTART.md` (Step-by-step first run) |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:509 | A | 7. Write comprehensive QUICKSTART.md |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:18 | A | **Fix:** Create `/docs/QUICKSTART.md` with step-by-step setup |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:76 | A | - [Quickstart Guide](QUICKSTART.md) |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:227 | A | - [ ] Create `/docs/QUICKSTART.md` |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:311 | A | 1. `/docs/QUICKSTART.md` - CREATE |
| `docs/current/ARCHITECTURE_REFERENCE.md`:144 | A | 📚 **Docs**: `docs/archive/completion_reports/ARBITER_IMPLEMENTATION.md`, `ARBITER_QU… |
| `docs/current/DOCUMENTATION_INDEX.md`:90 | A | - **`docs/current/AW_EDITOR_QUICK_REFERENCE.md`** — Quick reference |
| `docs/current/README.md`:44 | A | - **[AW_EDITOR_QUICK_REFERENCE.md](AW_EDITOR_QUICK_REFERENCE.md)** - Quick reference |
| `docs/current/RENDERING_ANALYSIS_INDEX.md`:14 | A | **File**: [`RENDERING_DEBUG_QUICK_REFERENCE.md`](./RENDERING_DEBUG_QUICK_REFERENCE.m… |
| `docs/current/RENDERING_ANALYSIS_INDEX.md`:234 | A | **Next Command**: Open `RENDERING_DEBUG_QUICK_REFERENCE.md` and copy Test 1 code |
| `docs/current/RENDERING_QUICK_REFERENCE.md`:232 | A | 3. **RENDERING_QUICK_REFERENCE.md** - This document |
| `docs/guides/README.md`:11 | A | \| [BUILD_QUICK_REFERENCE.md](BUILD_QUICK_REFERENCE.md) \| Build commands and troubl… |
| `docs/guides/README.md`:22 | A | \| [POLYHAVEN_QUICK_START.md](POLYHAVEN_QUICK_START.md) \| PolyHaven PBR assets \| |
| `docs/guides/README.md`:23 | A | \| [QUICK_START_GLB_ASSETS.md](QUICK_START_GLB_ASSETS.md) \| GLB model integration \| |
| `docs/guides/README.md`:54 | A | \| [SECURITY_QUICK_REFERENCE.md](SECURITY_QUICK_REFERENCE.md) \| Security quick refe… |
| `docs/guides/README.md`:56 | A | \| [HMAC_QUICK_REFERENCE.md](HMAC_QUICK_REFERENCE.md) \| HMAC quick reference \| |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`:71 | A | \| `PBR_D_QUICK_SUMMARY.md` \| 100+ \| Fast reference guide \| |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`:221 | A | **For Quick Reference**: See `PBR_D_QUICK_SUMMARY.md` (100+ lines) |
| `docs/pbr/PBR_D_FINAL_SUMMARY.md`:77 | A | - **PBR_D_QUICK_SUMMARY.md** (100+ lines): Fast reference guide with key metrics |
| `docs/pbr/PBR_D_QUICK_SUMMARY.md`:91 | A | \| **New**: `PBR_D_QUICK_SUMMARY.md` \| +100 lines \| ✅ This file \| |
| `docs/pbr/PBR_D_VALIDATION_REPORT.md`:278 | A | \| PBR_D_QUICK_SUMMARY.md \| 100+ \| Fast reference guide \| ✅ Complete \| |
| `docs/pbr/PBR_E_QUICK_REFERENCE.md`:115 | A | - **Quick Reference**: `PBR_E_QUICK_REFERENCE.md` (this document) |
| `docs/pbr/PBR_F_COMPLETION_SUMMARY.md`:203 | A | ### 2. PBR_F_QUICK_REFERENCE.md (400+ lines) |
| `docs/pbr/PBR_F_COMPLETION_SUMMARY.md`:367 | A | PBR_F_QUICK_REFERENCE.md                            400+ lines (quick reference) |
| `docs/pbr/PBR_F_QUICK_REFERENCE.md`:352 | A | - **This File**: `PBR_F_QUICK_REFERENCE.md` |
| `docs/src/core-systems/ai/arbiter.md`:337 | A | - [Quick Reference](../../archive/completion_reports/ARBITER_QUICK_REFERENCE.md) - 5… |
| `examples/hello_companion/README.md`:106 | A | - [Quick Reference (5 min read)](../../docs/ARBITER_QUICK_REFERENCE.md) - API docs, … |
| `tools/astraweave-assets/README.md`:467 | A | - **Quick Ref**: [../../docs/root-archive/POLYHAVEN_QUICK_REF.md](../../docs/root-ar… |

### `future-target` (28) — permanent

*world-class/production-ready framed as a future goal ("Timeline to World-Class: 6-9 months")*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:675 | B | **Current Status**: 70% production-ready (README estimate) |
| `docs/audits/EXTERNAL_RESEARCH_INDEX.md`:154 | B | 1. **README.md**: Current status (70% production-ready), feature list, benchmarks |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:36 | B | ### Timeline to World-Class: **6-9 months** |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:662 | B | - **Required for "World-Class"**: 650+ tests |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:905 | B | 6. **Everything else** (10 weeks) - Polish to world-class |
| `docs/audits/TEST_SUITE_COMPREHENSIVE_AUDIT.md`:453 | B | The AstraWeave test suite requires significant investment to reach production-grade … |
| `docs/current/ASTRACT_GIZMO_IMPLEMENTATION_PLAN.md`:50 | B | **Quality Target**: 70%+ coverage, benchmarked, production-ready |
| `docs/current/AW_EDITOR_RECOVERY_ROADMAP.md`:5 | B | This roadmap sequences the work required to turn `tools/aw_editor` into a production… |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md`:231 | B | - Define "production-ready" criteria |
| `docs/current/ECS_REGRESSION_SESSION_COMPLETE.md`:167 | B | The fix is well-defined, low-risk (BlobVec already production-ready), and the infras… |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:1 | B | # AstraWeave GitHub Pages Production-Grade Transformation Plan |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:11 | B | AstraWeave has a **documentation foundation** with 1,078+ files and 2M+ words of con… |
| `docs/current/GITHUB_PAGES_PRODUCTION_PLAN.md`:19 | B | **Target Grade**: A (Production-ready, best-in-class) |
| `docs/current/IMPLEMENTATION_PLANS_INDEX.md`:82 | B | **Purpose**: Transform to production-ready engine |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md`:12 | B | Complete the AstraWeave renderer's lighting path (GPU light culling) and post-proces… |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:4 | B | **Purpose**: Establish benchmark criteria for AstraWeave Editor based on industry-le… |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:11 | B | This research establishes comprehensive benchmarks for world-class game engine edito… |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:27 | B | - **Must-Have**: Critical for world-class status |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:1453 | B | ## MUST-HAVE FEATURES FOR WORLD-CLASS STATUS |
| `docs/current/WORLD_CLASS_EDITOR_DELIVERY_PLAN.md`:1 | B | # AstraWeave Visual Editor – World-Class Delivery Plan |
| `docs/current/WORLD_CLASS_EDITOR_DELIVERY_PLAN.md`:5 | B | **Objective:** Close the gap between the current feature-prototype state and a verif… |
| `docs/current/WORLD_CLASS_EDITOR_DELIVERY_PLAN.md`:11 | B | \| Dimension \| World-Class Target \| Verification Method \| |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md`:662 | B | **Overall Maturity:** 70% of world-class target |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md`:681 | B | **Estimated Effort:** 160-200 person-hours to reach production-ready state. |
| `tools/aw_editor/CODE_QUALITY_STATUS.md`:126 | B | **Total Time to Production-Ready Phase 2**: ✅ COMPLETE (Phase 2.1, 2.2, 2.3 done) |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md`:281 | B | **Must Close (World-Class Blocking)**: |
| `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md`:475 | B | ### World-Class Criteria |
| `tools/aw_editor/PHASE_2_COMPLETION_SUMMARY.md`:330 | B | **Total to World-Class**: 14-20 weeks remaining (~3.5-5 months) |

### `chat-artifact` (22) — permanent

*.zencoder/chats/ AI chat-session log (process artifact, like journey)*

| File:line | V | Match |
|---|:-:|---|
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:632 | A | pub struct MaterialGraph { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:663 | A | **Contracts**: `MaterialGraph`, `MaterialNode`, `MaterialNodeType` structs |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:687 | A | graph: MaterialGraph, |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:727 | A | **Objective**: Compile MaterialGraph to WGSL fragment shader code. |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:733 | A | fn validate(graph: &MaterialGraph) -> Result<(), CompileError> { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:741 | A | fn topological_sort(graph: &MaterialGraph) -> Result<Vec<NodeId>, CompileError> { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:793 | A | pub fn set_material_graph(&mut self, graph: &MaterialGraph) -> Result<()> { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/plan.md`:833 | A | impl MaterialGraph { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/requirements.md`:12 | B | The AstraWeave visual editor currently implements a foundational editor with 16+ pan… |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/requirements.md`:21 | B | **Target State**: Production-ready visual editor matching Unity/Godot feature parity |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/requirements.md`:593 | B | **Context**: Rhai integration exists but per audit is not production-ready. Graph-on… |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:162 | A | │   ├── graph.rs                     [NEW] - MaterialGraph data structure |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:254 | A | pub struct MaterialGraph { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:439 | A | /// Compile MaterialGraph to WGSL fragment shader code |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:440 | A | pub fn compile(graph: &MaterialGraph) -> Result<String, CompileError> { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:458 | A | fn validate(graph: &MaterialGraph) -> Result<(), CompileError>; |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:459 | A | fn topological_sort(graph: &MaterialGraph) -> Result<Vec<NodeId>, CompileError>; |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:656 | A | - Enhance `brdf_preview.rs` to accept MaterialGraph |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:661 | A | - Save MaterialGraph to `.mat.ron` |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:850 | A | let graph = MaterialGraph { |
| `.zencoder/chats/8d391d61-ea96-4b7e-8595-9406b2e34960/spec.md`:887 | A | let graph = MaterialGraph { /* ... */ }; |
| `.zencoder/chats/fd10dfb9-9c69-48ed-8473-5073acd4f1e8/spec.md`:1 | B | # Technical Specification: AstraWeave Editor World-Class Transformation |

### `contested-pending-D2` (14) — pending-D2 (re-evaluate after D.2 resolves the number)

*99.96% and similar numbers in the D.2-deferred ledger; allowlist entry expires post-D.2*

| File:line | V | Match |
|---|:-:|---|
| `docs/current/PROJECT_STATUS.md`:221 | A | - Tracy profiling integrated, spatial hash 99.96% fewer checks |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:160 | A | ├─ Direct: 99.96% fewer collision checks |
| `docs/lessons/PERFORMANCE_PATTERNS.md`:605 | A | 7. ✅ **Spatial hash** (99.96% fewer collision checks) |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:205 | A | \| Spatial hash \| 99.96% fewer collision checks \| O(n log n), cascade 9-17% to all… |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:842 | A | - Spatial hash: 99.96% fewer collision checks, 9-17% cascade improvement |
| `docs/src/api/physics.md`:121 | A | **Performance**: 99.96% collision check reduction vs brute force |
| `docs/src/core-systems/physics.md`:16 | A | `SpatialHash` broadphase claiming "99.96% pair reduction vs brute-force." That |
| `docs/src/performance/benchmarks.md`:178 | A | \| Spatial hash \| 99.96% fewer \| Grid optimization \| |
| `docs/src/performance/budgets.md`:87 | A | With spatial hashing (99.96% check reduction): |
| `docs/src/performance/budgets.md`:91 | A | \| 100 \| 4,950 checks \| 2 checks \| 99.96% \| |
| `docs/src/performance/optimization.md`:104 | A | AstraWeave's spatial hash reduces collision checks by 99.96%: |
| `gh-pages/benchmarks.md`:109 | A | \| Spatial hash \| 99.96% fewer collision checks \| |
| `gh-pages/index.md`:94 | A | \| Physics \| Collision pair reduction \| 99.96% \| |
| `gh-pages/physics.md`:146 | A | \| Collision pair reduction \| 99.96% \| |

### `keep-verified` (12) — permanent

*Hand-verified KEEP this triage (legend definitions, audits debunking the claim, research-doc titles, corrected text)*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/COMPETITIVE_MATRIX.md`:230 | B | - ✅ You need **mature editor** (world-class tools) |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:1222 | B | **Legend**: ⭐ Exceeds industry, ✅ Meets industry, ⚠️ Partial support, ❌ Not available |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:91 | B | > - **Status**: "WORLD-CLASS (Phases 1-8 COMPLETE: 36 tasks, zero defects, AAA featu… |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:387 | B | - Change "WORLD-CLASS zero defects" to "Production-ready with minor TODOs" |
| `docs/audits/DOCUMENTATION_AUDIT_SUMMARY.md`:107 | B | - WORLD-CLASS (Phases 1-8 COMPLETE: 36 tasks, zero defects, AAA features) |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:141 | B | ## 2. Performance Benchmarks vs AAA Standards |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`:694 | B | **AstraWeave is an AI-native game engine** with **mature core systems** (ECS, AI, Re… |
| `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md`:822 | B | **Total Estimate**: 22-30 weeks (5.5-7.5 months) for full world-class physics |
| `docs/current/BENCHMARK_PRODUCTION_AUDIT_REPORT.md`:174 | B | **Top 10 most comprehensive benchmarks:** |
| `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`:30 | B | **Reality Check**: Pure SPH is rarely "world-class" in games without hybrids or appr… |
| `docs/current/WORLD_CLASS_EDITOR_BENCHMARK_RESEARCH.md`:1 | B | # World-Class Video Game Engine Editor Benchmark Research |
| `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md`:649 | B | ## 11. Comparison to World-Class Standards |

### `historical-audit-denominator` (11) — permanent

*"47 crates" as a dated audit fraction denominator (39/47 missing READMEs)*

| File:line | V | Match |
|---|:-:|---|
| `docs/_audit/discovery-report.md`:183 | A | \| **Reference** \| 5 \| ⚠️ Needs verification \| ⚠️ Incomplete crates.md \| Add all… |
| `docs/_audit/discovery-report.md`:320 | A | \| **API Reference** \| Individual pages for 47 crates \| P0 \| |
| `docs/_audit/discovery-report.md`:334 | A | \| Crates Reference \| Only ~15 of 47 crates documented \| Add remaining 32 \| |
| `docs/_audit/discovery-report.md`:384 | A | 3. **Add crate documentation** - Document all 47 crates |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:513 | A | - **Found**: 5/47 crates (10.6%) have README.md |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:514 | A | - **Missing**: 42/47 crates (89%) lack README.md |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:783 | A | 11. **Per-Crate READMEs Missing** (42/47 crates) |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:123 | A | > - Overall Coverage: ~71.37% (26/47 crates measured) |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:248 | A | \| **Crate READMEs** \| 39/47 crates missing READMEs \| 🔴 Critical \| |
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:478 | A | 4. **Missing Crate READMEs** - 39/47 crates undocumented |
| `docs/current/MASTER_COVERAGE_REPORT.md`:553 | A | \| 2.0.0 \| 2025-12-06 \| Major \| Full workspace audit: 47 crates measured \| Signi… |

### `real-module-reference` (11) — permanent

*References the real astraweave-llm/src/hermes2pro_ollama.rs module / model string*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:157 | A | use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama; // ✅ Module exists |
| `docs/audits/job_system_audit_2026-04-18.md`:212 | A | - `tokio::sync::Mutex`: `astraweave-llm/src/phi3.rs:33`, `astraweave-llm/src/hermes2… |
| `docs/audits/job_system_audit_2026-04-18.md`:222 | A | - `astraweave-llm/src/phi3_ollama.rs:245`, `astraweave-llm/src/hermes2pro_ollama.rs:… |
| `docs/audits/job_system_audit_2026-04-18.md`:567 | A | - `src/qwen3_ollama.rs`, `src/phi3_ollama.rs`, `src/hermes2pro_ollama.rs` — `OnceLoc… |
| `docs/current/MUTATION_TESTING_AUDIT.md`:1961 | A | - `hermes2pro_ollama.rs` (32): Hermes2Pro Ollama client, same dependency |
| `docs/lessons/TESTING_STRATEGIES.md`:510 | A | let model = "adrienbrault/nous-hermes2pro:Q4_K_M"; |
| `docs/masters/MASTER_API_PATTERNS.md`:555 | A | use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama; |
| `docs/masters/MASTER_API_PATTERNS.md`:559 | A | "adrienbrault/nous-hermes2pro:Q4_K_M" |
| `docs/masters/MASTER_API_PATTERNS.md`:568 | A | use astraweave_llm::hermes2pro_ollama::{Hermes2ProOllama, ChatSession}; |
| `docs/masters/MASTER_API_PATTERNS.md`:1220 | A | hermes2pro_ollama::Hermes2ProOllama, |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:387 | A | **Models**: `qwen3:8b` (5.2GB Q4_K_M), `adrienbrault/nous-hermes2pro:Q4_K_M` (4.4GB). |

### `historical-dated` (10) — permanent

*A note correctly scoped to a past commit/phase/date*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:152 | A | //! 4. LLM (Hermes 2 Pro via Ollama with Phase 7 enhancements) |
| `docs/current/PROJECT_STATUS.md`:212 | A | - Hermes 2 Pro integration via Ollama |
| `docs/current/PROJECT_STATUS.md`:216 | A | - 54 compilation errors resolved, all 6 AI modes functional |
| `docs/current/PROJECT_STATUS.md`:217 | A | - Hermes 2 Pro connected, MockLLM eliminated |
| `docs/lessons/AI_ORCHESTRATION_TIPS.md`:212 | A | **Next step**: Let's validate with `hello_companion --demo-all`. Run the example and… |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:853 | A | \| 5.56 \| 2026-02-28 \| Major \| Qwen3-8B LLM latency: 4 rounds, 3.60× faster than … |
| `docs/masters/MASTER_ROADMAP.md`:474 | A | - **Added missing crates**: astraweave-fluids (4,907 tests), astraweave-prompts (1,9… |
| `docs/src/core-systems/networking.md`:17 | A | 2025-09-08, by an automated documentation pass) described a QUIC-based transport and |
| `tools/ASSET_SIGNING_DESIGN.md`:1124 | A | - Key strength: Ed25519 ~128-bit security (meets 2030+ requirements) |
| `tools/aw_editor/WORLD_CLASS_EDITOR_PLAN.md`:414 | B | ### World-Class (Week 8) |

### `benchmark/api-subject` (7) — permanent

*Qwen3-vs-Hermes benchmark comparison / API-pattern perf note*

| File:line | V | Match |
|---|:-:|---|
| `docs/masters/MASTER_API_PATTERNS.md`:29 | A | \| LLM round-trip (Hermes 2 Pro) \| 3-8 sec \| N/A \| async, off main thread \| |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:67 | A | ### Qwen3-8B LLM Latency Breakthrough (v5.56) |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:69 | A | - Qwen3-8B: **3.60× faster** than Hermes 2 Pro (blocking), **76.9% faster** than uno… |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:337 | A | #### Qwen3-8B vs Hermes 2 Pro — Head-to-Head Latency (Final, Post Round 4) |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:339 | A | \| Metric \| Qwen3-8B \| Hermes 2 Pro \| Winner \| Ratio \| |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:349 | A | **Score**: Qwen3 4/5 \| Hermes 1/5. **Verdict**: Qwen3-8B is the faster model. |
| `docs/masters/MASTER_BENCHMARK_REPORT.md`:828 | A | - **LLM latency**: Blocking 2,745 ms avg (Qwen3-8B, optimized); further reduction re… |

### `make-target` (5) — permanent

*make check-all/build-core/test-all/clippy-all — real Makefile targets (NOT the nonexistent cargo aliases)*

| File:line | V | Match |
|---|:-:|---|
| `docs/current/QWEN3_MIGRATION_PLAN.md`:936 | A | 3. **Smoke Test**: `make check-all` + `make test-all` (Makefile targets; there are n… |
| `gh-pages/setup.md`:99 | A | make build-core       # Build core crates only |
| `gh-pages/setup.md`:100 | A | make check-all        # Workspace check |
| `gh-pages/setup.md`:101 | A | make test-all         # Test all working crates |
| `gh-pages/setup.md`:102 | A | make clippy-all       # Full linting |

### `real-client-mention` (3) — permanent

*Hermes 2 Pro named as a real opt-in client*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/DOCUMENTATION_AUDIT_REPORT.md`:143 | A | > - Hermes 2 Pro LLM integrated |
| `docs/lessons/TESTING_STRATEGIES.md`:502 | A | - **hello_companion --demo-all**: End-to-end validation with Hermes 2 Pro |
| `docs/lessons/TESTING_STRATEGIES.md`:508 | A | async fn test_llm_integration_with_hermes2pro() { |

### `production-shipped` (3) — permanent

*production-ready on a genuinely shipped surface (ECS/Camera/Input/core)*

| File:line | V | Match |
|---|:-:|---|
| `README.md`:52 | B | **🏆 Production-Grade Quality**: AstraWeave has **~39,000+ test annotations** across … |
| `astraweave-ecs/README.md`:1 | B | # AstraWeave ECS - Production-Grade, AI-Native Entity Component System |
| `docs/current/EDITOR_STATUS_REPORT.md`:267 | B | The AstraWeave Editor is **production-ready for core scene editing workflows**. The … |

### `design-sketch-caveated` (2) — permanent

*Interfaces.md protobuf sketch, now caveated as non-implemented (honest-dormancy)*

| File:line | V | Match |
|---|:-:|---|
| `docs/reference/Interfaces.md`:84 | A | message EntityView { uint64 id = 1; string kind = 2; Vec3 pos = 3; Vec3 vel = 4; uin… |
| `docs/reference/Interfaces.md`:86 | A | message WorldSnapshot { uint32 version = 1; uint64 tick = 2; uint64 time_ms = 3; uin… |

### `project-thesis` (2) — permanent

*"production-grade game engine" — the project mandate/thesis in its own instruction file*

| File:line | V | Match |
|---|:-:|---|
| `.github/copilot-instructions.md`:11 | B | AstraWeave is a **scientific proof of concept**: a production-grade game engine buil… |
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`:1022 | B | AstraWeave is a 100% AI-generated production-grade game engine. The codebase demonst… |

### `fiction` (2) — permanent

*docs/Veilweaver/lore_bible.md game-narrative fiction*

| File:line | V | Match |
|---|:-:|---|
| `docs/Veilweaver/lore_bible.md`:5693 | B | leaving no trace that the thread can record. Their survival strategy was perfect. Th… |
| `docs/Veilweaver/lore_bible.md`:7726 | B | The words are healing. The language is recovering the meanings that the Nachash spen… |

### `terrain-measurement` (1) — permanent

*"3-6×" terrain-amplitude increase (not a competitor multiplier)*

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/terrain_scale_diagnostic_2026-04-24.md`:112 | A | \| Y span \| 92 WU (m?) \| **300-600m** (3-6× increase) \| |

### `attribution` (1) — permanent

*External-model attribution / license note*

| File:line | V | Match |
|---|:-:|---|
| `docs/current/ATTRIBUTIONS.md`:91 | A | - Ollama / Hermes 2 Pro (Nous Research) — External model; consult model card/license |

### `false-positive` (1) — permanent

**

| File:line | V | Match |
|---|:-:|---|
| `docs/audits/tonemap_double_application_investigation_2026-04-19.md`:100 | B | [astraweave-render/examples/tonemap_probe.rs](../../astraweave-render/examples/tonem… |
