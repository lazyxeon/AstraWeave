# Complete Codebase Test Coverage Analysis & Roadmap - October 21, 2025

**Objective**: Analyze ALL crates in AstraWeave and develop comprehensive plan to exceed industry standards (70-80%)  
**Date**: October 21, 2025  
**Current Status**: P0 crates COMPLETE (86.85% average), remaining crates need analysis  
**Target**: **85%+ average across ALL production crates**

---

## Executive Summary

### What We Know (As of Oct 21, 2025)

**✅ P0 CRATES COMPLETE** (5/5 - 86.85% average):
- astraweave-audio: **78.57%** (136 tests, 10h work)
- astraweave-nav: **100%** (26 tests, discovery)
- astraweave-physics: **91.08%** (30 tests, 1.5h work)
- astraweave-behavior: **77.62%** (56 tests, discovery)
- astraweave-math: **87.10%** (53 tests, discovery)

**❓ REMAINING CRATES**: Unknown coverage, need systematic analysis

**Total Workspace**:
- **109 members** (82 crates + 27 examples + tools)
- **Core crates**: ~40-50 production libraries
- **Examples**: 27 (test coverage optional)
- **Tools**: 15 (test coverage optional)

---

## Complete Crate Inventory

### Category 1: P0 - Core Engine Systems (5 crates) ✅ COMPLETE

**Mission-Critical for engine functionality**

| Crate | Coverage | Tests | Status | Priority |
|-------|----------|-------|--------|----------|
| astraweave-audio | 78.57% | 136 | ✅ COMPLETE | P0 |
| astraweave-nav | 100% | 26 | ✅ COMPLETE | P0 |
| astraweave-physics | 91.08% | 30 | ✅ COMPLETE | P0 |
| astraweave-behavior | 77.62% | 56 | ✅ COMPLETE | P0 |
| astraweave-math | 87.10% | 53 | ✅ COMPLETE | P0 |
| **AVERAGE** | **86.85%** | **301** | ✅ **100%** | - |

**Status**: ALL P0 crates exceed 70-80% industry standard ✅

---

### Category 2: P1 - Critical Engine Infrastructure (15 crates) ❓ UNKNOWN

**Essential for production games**

| Crate | Purpose | Est. Coverage | Priority |
|-------|---------|---------------|----------|
| **astraweave-ecs** | Entity-Component-System core | ~50%? | **P1-A** |
| **astraweave-ai** | AI orchestration, core loop | ~32%? | **P1-A** |
| **astraweave-core** | Shared schema, validation | ~35%? | **P1-A** |
| **astraweave-render** | wgpu 25 rendering pipeline | Unknown | **P1-B** |
| **astraweave-scene** | World partition, streaming | Unknown | **P1-B** |
| **astraweave-terrain** | Voxel/polygon hybrid | Unknown | **P1-B** |
| **astraweave-gameplay** | Combat physics | Unknown | **P1-B** |
| **astraweave-cinematics** | Timeline sequencer | Unknown | **P1-C** |
| **astraweave-input** | Gilrs controller bindings | Unknown | **P1-C** |
| **astraweave-ui** | egui UI framework | Unknown | **P1-C** |
| **astraweave-materials** | Material system | Unknown | **P1-C** |
| **astraweave-asset** | Asset loading pipeline | Unknown | **P1-C** |
| **astraweave-npc** | NPC behavior | Unknown | **P1-D** |
| **astraweave-dialogue** | Dialogue system | Unknown | **P1-D** |
| **astraweave-quests** | Quest system | Unknown | **P1-D** |

**Estimated Status**: 
- P1-A (ECS/AI/Core): ~35-50% (medium coverage)
- P1-B (Rendering/Scene): 10-30%? (low coverage likely)
- P1-C (UI/Input/Assets): 5-20%? (very low coverage likely)
- P1-D (Gameplay): 0-15%? (minimal coverage likely)

**Priority**: Measure ALL P1 crates next

---

### Category 3: P2 - Advanced Systems (12 crates) ❓ UNKNOWN

**Nice-to-have, not blocking production**

| Crate | Purpose | Est. Coverage | Priority |
|-------|---------|---------------|----------|
| **astraweave-pcg** | Procedural content generation | Unknown | P2-A |
| **astraweave-weaving** | Veilweaver game mechanic | Unknown | P2-A |
| **astraweave-memory** | AI long-term memory | Unknown | P2-B |
| **astraweave-persona** | AI persona system | Unknown | P2-B |
| **astraweave-llm** | LLM integration | Unknown | P2-B |
| **astraweave-embeddings** | Vector embeddings | Unknown | P2-C |
| **astraweave-context** | Context management | Unknown | P2-C |
| **astraweave-prompts** | Prompt engineering | Unknown | P2-C |
| **astraweave-rag** | RAG system | Unknown | P2-C |
| **astraweave-net** | Networking base | Unknown | P2-D |
| **astraweave-ipc** | Inter-process comms | Unknown | P2-D |
| **astraweave-director** | AI director system | Unknown | P2-D |

**Target**: 60-70% coverage (lower than P0/P1)

---

### Category 4: P3 - Infrastructure & Tooling (18 crates) ❓ UNKNOWN

**Developer tools, CI/CD, optional features**

| Subcategory | Crates | Target Coverage |
|-------------|--------|-----------------|
| **Observability** | astraweave-observability, astraweave-profiling, aw_debug | 50-60% |
| **Quality** | astraweave-stress-test, astraweave-security | 70-80% |
| **Networking** | aw-net-proto, aw-net-server, aw-net-client, astraweave-net-ecs | 60-70% |
| **Persistence** | aw-save, astraweave-persistence-ecs | 70-80% |
| **Asset Pipeline** | astraweave-asset-pipeline, astraweave-assets | 40-50% |
| **SDK** | astraweave-sdk | 60-70% |
| **Build Tools** | aw_build, aw_release, aw_demo_builder, aw_texture_gen, aw_headless | 30-40% |
| **CLI Tools** | aw_asset_cli, aw_save_cli, dialogue_audio_cli, ollama_probe, asset_signing | 30-40% |

**Priority**: After P1/P2 complete

---

### Category 5: Examples (27 examples) - OPTIONAL

**Test coverage NOT required for examples** (examples are documentation/demos)

**Categories**:
- LLM demos (7): hello_companion, ollama_probe, llm_toolcall, etc.
- Gameplay demos (8): weaving_playground, combat_physics_demo, etc.
- Rendering demos (5): visual_3d, skinning_demo, biome_showcase, etc.
- System demos (7): core_loop_bt_demo, profiling_demo, ui_menu_demo, etc.

**Strategy**: Ensure examples COMPILE and RUN, but test coverage = 0% is acceptable

---

## Workspace Crate Categorization

### Production Crates Needing Coverage Analysis (47 total)

**Tier 1 - Critical** (20 crates):
```
P0 (5): audio, nav, physics, behavior, math ✅ COMPLETE
P1-A (3): ecs, ai, core ← MEASURE NEXT
P1-B (4): render, scene, terrain, gameplay ← MEASURE NEXT
P1-C (5): cinematics, input, ui, materials, asset ← MEASURE AFTER P1-A/B
P1-D (3): npc, dialogue, quests ← MEASURE AFTER P1-A/B/C
```

**Tier 2 - Important** (12 crates):
```
P2-A (2): pcg, weaving
P2-B (3): memory, persona, llm
P2-C (4): embeddings, context, prompts, rag
P2-D (3): net, ipc, director
```

**Tier 3 - Infrastructure** (15 crates):
```
Observability (3): observability, profiling, aw_debug
Quality (2): stress-test, security
Networking (4): net-proto, net-server, net-client, net-ecs
Persistence (2): aw-save, persistence-ecs
Assets (2): asset-pipeline, assets
SDK (1): sdk
LLM Eval (1): llm-eval
```

---

## Systematic Measurement Plan

### Phase 1: P1-A Crates (ECS/AI/Core) - Priority 1 🎯

**Goal**: Measure 3 foundational crates

**Crates**:
1. astraweave-ecs (archetype ECS implementation)
2. astraweave-ai (AI orchestration, core loop)
3. astraweave-core (shared schema, validation)

**Commands**:
```powershell
# ECS
cargo test -p astraweave-ecs 2>&1 | Select-String -Pattern "test result:"
cargo tarpaulin -p astraweave-ecs --include-files "astraweave-ecs/src/**" --out Html --output-dir coverage/ecs_baseline

# AI  
cargo test -p astraweave-ai 2>&1 | Select-String -Pattern "test result:"
cargo tarpaulin -p astraweave-ai --include-files "astraweave-ai/src/**" --out Html --output-dir coverage/ai_baseline

# Core
cargo test -p astraweave-core 2>&1 | Select-String -Pattern "test result:"
cargo tarpaulin -p astraweave-core --include-files "astraweave-core/src/**" --out Html --output-dir coverage/core_baseline
```

**Estimated Time**: 1-2 hours (compilation + measurement)

**Expected Results**:
- ECS: ~50% (from baseline report)
- AI: ~32% (from baseline report)
- Core: ~35% (from baseline report)

**Decision Point**: 
- If <50%: Add to improvement plan
- If 50-70%: Consider improvement
- If >70%: Mark complete ✅

---

### Phase 2: P1-B Crates (Rendering/Scene) - Priority 2

**Goal**: Measure rendering and world systems

**Crates**:
1. astraweave-render (wgpu 25, materials, IBL, GPU skinning)
2. astraweave-scene (world partition, streaming)
3. astraweave-terrain (voxel/polygon hybrid, marching cubes)
4. astraweave-gameplay (combat physics, attack sweep)

**Expected Results**: 10-30% (graphics code often has lower test coverage)

**Strategy**: Target 60-70% for rendering (industry standard for graphics)

---

### Phase 3: P1-C Crates (UI/Input/Assets) - Priority 3

**Goal**: Measure user-facing and asset systems

**Crates**:
1. astraweave-cinematics (timeline sequencer)
2. astraweave-input (gilrs controller)
3. astraweave-ui (egui)
4. astraweave-materials (material system)
5. astraweave-asset (asset loading)

**Expected Results**: 5-20% (UI/asset systems often undertested)

**Strategy**: Target 50-60% (lower priority than core engine)

---

### Phase 4: P1-D Crates (Gameplay) - Priority 4

**Goal**: Measure gameplay-specific systems

**Crates**:
1. astraweave-npc (NPC behavior)
2. astraweave-dialogue (dialogue system)
3. astraweave-quests (quest system)

**Expected Results**: 0-15% (game-specific code often has minimal tests)

**Strategy**: Target 60-70% (important for game quality)

---

### Phase 5: P2 Crates - Priority 5

**Goal**: Measure advanced/optional systems

**12 crates** across 4 subcategories (PCG, AI advanced, LLM, networking)

**Strategy**: Target 50-60% average (lower priority)

---

### Phase 6: P3 Infrastructure - Priority 6

**Goal**: Measure tooling and infrastructure

**15 crates** across observability, quality, networking, persistence, assets, SDK

**Strategy**: Vary by importance:
- Quality crates (stress-test, security): 70-80%
- Persistence: 70-80%
- Others: 30-50%

---

## Coverage Target Strategy

### Industry Standards (Reference)

| Coverage Tier | Percentage | Description |
|---------------|------------|-------------|
| Minimal | 0-40% | Untested/prototype code |
| Basic | 40-60% | Some testing exists |
| Good | 60-70% | Reasonable coverage |
| **Industry Standard** | **70-80%** | **Typical mature project** |
| Excellent | 80-90% | High-quality project |
| Mission-Critical | 90-100% | Safety-critical systems |

### AstraWeave Targets by Category

| Category | Target Range | Rationale |
|----------|--------------|-----------|
| **P0 (Core Engine)** | **85-95%** | ✅ ACHIEVED (86.85% average) |
| **P1-A (ECS/AI/Core)** | **75-85%** | Critical infrastructure |
| **P1-B (Render/Scene)** | **60-70%** | Graphics code, harder to test |
| **P1-C (UI/Input/Assets)** | **50-60%** | User-facing, moderate priority |
| **P1-D (Gameplay)** | **60-70%** | Game quality important |
| **P2 (Advanced)** | **50-60%** | Optional features |
| **P3 (Infrastructure)** | **Varies** | 30-80% depending on criticality |

**Overall Codebase Target**: **70%+ average** across all production crates

---

## Estimated Effort Breakdown

### Based on P0 Campaign Learnings

**Efficiency Benchmarks** (from P0 campaign):
- High efficiency: 53pp/hour (physics - filling known gaps)
- Medium efficiency: 7.7pp/hour (audio - building from scratch)
- Discovery efficiency: Instant (nav/behavior/math - already complete)

**Coverage Improvement Estimates**:
- **Build from 0-20%**: 6-8 hours per crate (10-15 pp/hour)
- **Build from 20-50%**: 3-5 hours per crate (15-20 pp/hour)
- **Build from 50-70%**: 1-3 hours per crate (20-40 pp/hour)
- **Discovery**: 0.25-0.5 hours per crate (measurement only)

### Phase-by-Phase Estimates

**Phase 1: P1-A (3 crates)** - ECS/AI/Core
- Measurement: 2 hours
- Improvement (assume 35-50% → 75-85%): 
  - ECS: 3-5 hours (50% → 80%)
  - AI: 5-8 hours (32% → 80%)
  - Core: 4-6 hours (35% → 80%)
- **Total: 14-21 hours**

**Phase 2: P1-B (4 crates)** - Render/Scene/Terrain/Gameplay
- Measurement: 2 hours
- Improvement (assume 10-30% → 60-70%):
  - Render: 6-8 hours (20% → 65%)
  - Scene: 5-7 hours (15% → 65%)
  - Terrain: 5-7 hours (15% → 65%)
  - Gameplay: 4-6 hours (25% → 65%)
- **Total: 22-30 hours**

**Phase 3: P1-C (5 crates)** - UI/Input/Assets/Cinematics/Materials
- Measurement: 2 hours
- Improvement (assume 5-20% → 50-60%):
  - Each crate: 4-6 hours average
- **Total: 22-32 hours**

**Phase 4: P1-D (3 crates)** - NPC/Dialogue/Quests
- Measurement: 1 hour
- Improvement (assume 0-15% → 60-70%):
  - Each crate: 5-8 hours average
- **Total: 16-25 hours**

**Phase 5: P2 (12 crates)** - Advanced systems
- Measurement: 3 hours
- Improvement (assume 0-30% → 50-60%):
  - Average 4-6 hours per crate
- **Total: 51-75 hours**

**Phase 6: P3 (15 crates)** - Infrastructure
- Measurement: 3 hours
- Improvement (varies widely):
  - High priority (6 crates): 4-6 hours each = 24-36h
  - Low priority (9 crates): 2-3 hours each = 18-27h
- **Total: 45-66 hours**

### Grand Total Estimate

**Measurement Only**: 13 hours (all phases)

**Improvement Work**:
- Phase 1 (P1-A): 14-21 hours
- Phase 2 (P1-B): 22-30 hours
- Phase 3 (P1-C): 22-32 hours
- Phase 4 (P1-D): 16-25 hours
- Phase 5 (P2): 51-75 hours
- Phase 6 (P3): 45-66 hours
- **Subtotal**: 170-249 hours

**Total Campaign**: **183-262 hours** (23-33 working days @ 8h/day)

**With Discoveries** (like nav/behavior/math pattern):
- Assume 30% of crates already >70%: Save ~50-75 hours
- **Realistic Total**: **133-212 hours** (17-27 working days)

---

## Risk Assessment

### Known Risks from P0 Campaign

**✅ Resolved**:
- Baseline measurement errors (fixed with scoped tarpaulin)
- API discovery issues (grep for methods before testing)
- Diminishing returns (recognize <2pp/hour threshold)

**⚠️ Potential New Risks**:

1. **Compilation Issues** ⚠️
   - Some crates may have dependency issues
   - Rendering crates may need GPU features
   - **Mitigation**: Test compilation before measurement

2. **Integration Test Complexity** ⚠️
   - Graphics tests need window/GPU context
   - Networking tests need mock servers
   - **Mitigation**: Use feature gates, mocks, headless rendering

3. **Async Code Testing** ⚠️
   - Tokio runtime setup
   - Async test complexity
   - **Mitigation**: Use tokio::test macro, existing patterns

4. **GPU Code Testing** ⚠️
   - wgpu shaders hard to unit test
   - Need integration tests with rendering
   - **Mitigation**: Focus on CPU-side logic, use shader validation tools

5. **Time Estimates** ⚠️
   - Rendering/GPU code may take 2-3× longer than estimated
   - LLM integration may have complex mocking needs
   - **Mitigation**: Buffer 50% extra time for complex crates

---

## Success Criteria

### Tier 1: Minimum Viable (MUST HAVE)

✅ P0 crates: 85%+ average ← **ACHIEVED (86.85%)**  
🎯 P1-A crates: 75%+ average  
🎯 P1-B crates: 60%+ average  
🎯 Overall P0+P1: 70%+ average  

**Timeline**: 4-6 weeks (Phase 1-4 complete)

### Tier 2: Industry Standard (SHOULD HAVE)

🎯 All P1 crates: 70%+ average  
🎯 P2 crates: 50%+ average  
🎯 Overall P0+P1+P2: 70%+ average  

**Timeline**: 8-12 weeks (Phase 1-5 complete)

### Tier 3: Excellent (NICE TO HAVE)

🎯 All production crates: 70%+ minimum  
🎯 P3 critical crates: 70%+ (security, persistence, quality)  
🎯 Overall codebase: 75%+ average  

**Timeline**: 12-16 weeks (Phase 1-6 complete)

---

## Immediate Next Steps (This Week)

### Step 1: Measure P1-A Crates (2-3 hours) 🎯

**Goal**: Establish baseline for ECS, AI, Core

**Commands**:
```powershell
# Test execution
cargo test -p astraweave-ecs --lib 2>&1 | Select-String -Pattern "test result:"
cargo test -p astraweave-ai --lib 2>&1 | Select-String -Pattern "test result:"  
cargo test -p astraweave-core --lib 2>&1 | Select-String -Pattern "test result:"

# Coverage measurement (if tests pass)
cargo tarpaulin -p astraweave-ecs --include-files "astraweave-ecs/src/**" --out Html --output-dir coverage/ecs_baseline -- --test-threads=1
cargo tarpaulin -p astraweave-ai --include-files "astraweave-ai/src/**" --out Html --output-dir coverage/ai_baseline -- --test-threads=1
cargo tarpaulin -p astraweave-core --include-files "astraweave-core/src/**" --out Html --output-dir coverage/core_baseline -- --test-threads=1
```

**Expected Output**:
```
astraweave-ecs: XX% coverage (YYY/ZZZ lines)
astraweave-ai: XX% coverage (YYY/ZZZ lines)
astraweave-core: XX% coverage (YYY/ZZZ lines)
```

**Decision Tree**:
- If any crate has compilation errors: Fix compilation first
- If coverage <50%: Add to Phase 1 improvement plan
- If coverage 50-70%: Add to Phase 1 optional improvement
- If coverage >70%: Mark complete ✅, move to Phase 2

---

### Step 2: Create Phase 1 Improvement Plan (1 hour)

Based on P1-A measurements, create detailed test plans for crates <70%

**Template** (reuse from P0 campaign):
1. Gap analysis (uncovered lines by file)
2. Test categories needed
3. Estimated tests required
4. Time estimate
5. Prioritization

---

### Step 3: Execute Phase 1 Improvements (14-21 hours)

Follow proven patterns from P0 campaign:
- Start with highest-gap files
- Create comprehensive test files
- Measure after each iteration
- Accept results when hitting diminishing returns

---

## Long-Term Roadmap

### Month 1: P1 Crates Complete (Weeks 1-4)
- Week 1: P1-A measurement + improvement ✅
- Week 2: P1-B measurement + improvement
- Week 3: P1-C measurement + improvement
- Week 4: P1-D measurement + improvement
- **Goal**: All 15 P1 crates at target coverage

### Month 2: P2 Crates Complete (Weeks 5-8)
- Week 5-6: P2-A/B measurement + improvement (5 crates)
- Week 7-8: P2-C/D measurement + improvement (7 crates)
- **Goal**: All 12 P2 crates at 50-60% coverage

### Month 3: P3 Infrastructure (Weeks 9-12)
- Week 9-10: Critical P3 crates (security, persistence, quality)
- Week 11-12: Tooling crates (build, asset pipeline, CLI)
- **Goal**: Critical P3 at 70%+, others at 30-50%

### Month 4: Polish & Documentation (Weeks 13-16)
- Integration tests (cross-crate)
- Performance regression tests
- Fuzz testing for critical paths
- Comprehensive documentation
- **Goal**: Production-ready test suite

---

## Conclusion

**Current Status**: P0 crates COMPLETE (86.85% avg, 5/5 crates) ✅

**Remaining Work**: 
- 42 production crates need measurement & improvement
- Estimated 133-212 hours (17-27 working days)
- Phased approach: P1 → P2 → P3

**Strategy**:
1. ✅ Measure systematically (avoid baseline errors)
2. ✅ Prioritize critical crates first (P1-A → P1-B → P1-C → P1-D)
3. ✅ Reuse proven patterns from P0 campaign
4. ✅ Accept great results (recognize diminishing returns)
5. ✅ Focus on production crates (examples optional)

**Success Definition**:
- Minimum Viable: P0+P1 at 70%+ average (4-6 weeks)
- Industry Standard: P0+P1+P2 at 70%+ average (8-12 weeks)
- Excellent: All production crates 70%+ minimum (12-16 weeks)

**Immediate Action**: Measure P1-A crates (ECS, AI, Core) - 2-3 hours

---

**Next Document**: `P1A_CRATES_MEASUREMENT_PLAN_OCT_21_2025.md` (after Step 1 complete)

**End of Analysis** | **Status**: Ready for Phase 1 execution 🚀
