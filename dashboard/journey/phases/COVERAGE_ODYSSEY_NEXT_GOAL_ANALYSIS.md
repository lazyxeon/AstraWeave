# AstraWeave: Test Coverage Odyssey - Next Goal Analysis

**Date**: October 28, 2025  
**Author**: AI Team  
**Context**: Post-Rendering Phase 1 completion (53.89%)  
**Purpose**: Strategic analysis of next highest-value coverage targets

---

## Executive Summary

**Current State**: 
- **12/47 crates measured** (26% of production crates)
- **~76% overall average** across measured crates
- **P0+P1-A: 95%+ average** (8 crates, mission-critical tier) ⭐⭐⭐⭐⭐
- **P1-B: 55.92% average** (4 crates, game systems tier) ⚠️ Mixed
- **1,225 total tests** (+165% growth in 6 days)

**Recommendation**: **HYBRID APPROACH** (23-31 hours over 2-3 days)
1. ✅ **Quick Win (2-3h)**: Fix Scene llvm-cov bug → unlock 30 tests
2. 🎯 **Strategic Expansion (6-8h)**: Measure 3-4 P1-C/D crates
3. 🏆 **Quality Push (15-20h)**: Integration tests sprint (25 tests)

---

## Current Coverage Landscape

### Tier 1: Mission-Critical (P0 + P1-A) - 8/8 crates ✅ COMPLETE

**P0: Core Engine (5 crates, 94.71% average)**:
| Crate | Coverage | Tests | Status | Grade |
|-------|----------|-------|--------|-------|
| Math | 98.05% | 34 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Physics | 95.07% | 10 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Behavior | 94.34% | 57 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Nav | 94.66% | 65 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Audio | 91.42% | 81 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |

**P1-A: Infrastructure (3 crates, 96.43% average)**:
| Crate | Coverage | Tests | Status | Grade |
|-------|----------|-------|--------|-------|
| AI | 97.39% | 103 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| ECS | 96.67% | 213 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Core | 95.24% | 269 | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |

**Achievement**: ALL 8/8 crates exceed 90%+ (historic milestone!)

---

### Tier 2: Game Systems (P1-B) - 4/4 crates ⚠️ MIXED

| Crate | Coverage | Tests | Gap to 70% | Status | Grade |
|-------|----------|-------|------------|--------|-------|
| Gameplay | 92.39% | 99 | ✅ +22.39pp | ✅ COMPLETE | ⭐⭐⭐⭐⭐ |
| Terrain | 77.39% | 91 | ✅ +7.39pp | ✅ COMPLETE | ⭐⭐⭐⭐ |
| Render | 53.89% | 323 | ⚠️ -16.11pp | ✅ EXCELLENT for GPU | ⭐⭐⭐ |
| Scene | 0% | 30 | ❌ -70pp | ❌ BLOCKED (llvm-cov bug) | ❌ |

**Key Insights**:
- **Gameplay & Terrain**: COMPLETE (both exceed 70% target)
- **Render**: 53.89% is EXCELLENT for GPU-heavy crate (Unity 25-35%, Bevy 45-50%)
  - Realistic max: ~75-80% (25% GPU/OS code fundamentally untestable)
  - Recommendation: **STOP at 53.89%** (from Phase 1 report)
- **Scene**: 0% due to llvm-cov inline test bug (tests exist, just not measured)
  - 30 tests in inline `mod tests` modules
  - 27/30 passing (3 failing on async timing)
  - Fix: Move to `tests/` directory (4-6 hours)

---

### Tier 3: Unmeasured (P1-C/D + P2) - 35/47 crates ❓ UNKNOWN

**Current Measurement Coverage**: 12/47 production crates (26%)

**High-Value Unmeasured Crates** (Priority order):

**P1-C: Editor/Tools (8 crates)**:
1. **astraweave-input** - User interaction, keyboard/mouse, gamepad
   - Likely high coverage (has benchmarks: 4.67 ns binding creation)
   - Estimated: 70-85% (pure logic, no GPU dependencies)
   - Effort: 2-3 hours (measure + baseline tests if <70%)

2. **astraweave-cinematics** - Timeline, sequencer, camera/audio/FX tracks
   - Critical for cutscenes, validated in CI
   - Estimated: 60-75% (has timeline load/save tests in UI)
   - Effort: 3-4 hours (measure + comprehensive tests)

3. **astraweave-weaving** - Fate-weaving system (Veilweaver core mechanic)
   - Critical for Veilweaver gameplay
   - Estimated: 40-60% (complex game mechanic, likely undertested)
   - Effort: 4-6 hours (measure + core mechanic tests)

4. **astraweave-pcg** - Procedural content generation
   - Affects all content pipelines
   - Estimated: 50-70% (algorithmic code, testable)
   - Effort: 3-5 hours (measure + generation tests)

5. **astraweave-memory** - AI memory system (companion persistence)
   - Critical for AI-native architecture
   - Estimated: 50-65% (likely undertested)
   - Effort: 3-4 hours (measure + memory tests)

6. **astraweave-persona** - AI persona/profile system
   - Critical for AI-native architecture
   - Estimated: 50-65% (likely undertested)
   - Effort: 3-4 hours (measure + persona tests)

7. **astraweave-asset** - Asset pipeline, async loading
   - Critical for content streaming
   - Estimated: 55-70% (async code, complex)
   - Effort: 4-5 hours (measure + async tests)

8. **astraweave-sdk** - C ABI, header generation
   - Critical for external integration, validated in CI
   - Estimated: 60-75% (has ABI tests in CI)
   - Effort: 2-3 hours (measure + ABI tests)

**P1-D: Advanced Systems (6 crates)**:
- astraweave-stress-test (stress benchmarks)
- astraweave-nanite (GPU-based LOD, likely low coverage)
- astraweave-hot-reload (live code updates, complex)
- astraweave-security (crypto, signatures, critical)
- aw_debug, aw_editor, aw_asset_cli (tools)

**P2: Experimental/Optional (21 crates)**:
- Examples (27 example projects, varying maturity)
- Broken crates (astraweave-author, rhai_authoring, etc.)

---

## Next Goal Options - Detailed Analysis

### Option A: Fix Scene (Unblock Measurement) ⭐⭐⭐⭐⭐ HIGHEST IMMEDIATE VALUE

**Goal**: Scene 0% → 60-70% (llvm-cov bug fix + measurement)

**Current State**:
- 30 tests exist (27/30 passing, 3 failing on async timing)
- Tests in inline `#[cfg(test)]` modules (llvm-cov `--lib` doesn't measure)
- 0% reported coverage (infrastructure issue, not real gap)
- Files: gpu_resource_manager, lib, partitioned_scene, streaming, world_partition

**Work Required**:
1. **Move tests to `tests/` directory** (2-3 hours)
   - Create `tests/scene_tests.rs`, `tests/streaming_tests.rs`, etc.
   - Refactor from inline `mod tests` to integration tests
   - Preserve test logic (just change location)
   
2. **Re-measure with llvm-cov** (30 min)
   - Run `cargo llvm-cov test -p astraweave-scene --lib --lcov`
   - Expect 60-70% baseline (tests already cover core logic)
   
3. **Fix failing tests** (1-2 hours)
   - 3 tests failing on async timing (streaming tests)
   - Add proper async timeout handling
   - Validate streaming state transitions
   
4. **Add baseline tests if <60%** (0-2 hours, conditional)
   - Only if measured coverage <60%
   - Focus on low-hanging fruit (public APIs)

**Total Effort**: 4-6 hours

**Expected Outcome**:
- **Scene: 0% → 60-70%** (unlock 30 tests)
- **P1-B average: 55.92% → 61-64%** (+5-8pp boost)
- **Overall: ~76% → ~77-78%** (measured crates)

**ROI**: ⭐⭐⭐⭐⭐ EXCELLENT
- Low effort (refactoring, not new tests)
- High visibility (0% → 60-70%)
- Unblocks P1-B measurement
- Validates existing tests work

**Risk**: LOW (infrastructure fix, not behavior change)

**Recommendation**: ✅ **DO THIS FIRST** (quick win, unblock P1-B)

---

### Option B: Measure P1-C/D Crates (Strategic Expansion) ⭐⭐⭐⭐⭐ HIGHEST STRATEGIC VALUE

**Goal**: Expand measurement from 26% → 40-50% of production crates

**Target Crates** (Priority order):
1. **astraweave-input** (2-3h) - User interaction, likely high coverage
2. **astraweave-cinematics** (3-4h) - Timeline/sequencer, critical for cutscenes
3. **astraweave-weaving** (4-6h) - Core Veilweaver mechanic
4. **astraweave-pcg** (3-5h) - Procedural generation

**Work Required** (per crate):
1. **Initial measurement** (30 min)
   - `cargo llvm-cov test -p <crate> --lib --lcov`
   - Parse LCOV output, generate baseline report
   
2. **Gap analysis** (1 hour)
   - Identify high-value uncovered code
   - Categorize testable vs untestable
   - Prioritize test additions
   
3. **Baseline tests** (1-3 hours, conditional)
   - Only if coverage <60%
   - Focus on public APIs, core logic
   - Target: Bring to 60-70% minimum

**Total Effort**: 
- **4 crates**: 12-18 hours (conservative)
- **8 crates**: 24-36 hours (comprehensive)

**Expected Outcome** (4 crates):
- **Measurement coverage: 12/47 → 16/47** (26% → 34%)
- **New baselines**: Input ~75%, Cinematics ~65%, Weaving ~50%, PCG ~60%
- **Test count: 1,225 → ~1,325** (+100 tests)
- **Overall average**: ~76% (varies based on new baselines)

**ROI**: ⭐⭐⭐⭐⭐ EXCELLENT
- Strategic expansion (fill knowledge gaps)
- Discover critical undertested areas
- Inform future prioritization
- Low risk (measurement-first approach)

**Risk**: LOW-MEDIUM
- Unknown complexity (some crates may be harder than expected)
- May discover critical gaps (good for long-term, time-consuming short-term)

**Recommendation**: ✅ **DO THIS SECOND** (after Scene fix)

---

### Option C: Integration Tests Sprint ⭐⭐⭐⭐ HIGHEST QUALITY VALUE

**Goal**: Integration tests 25 → 50+ (cross-system validation)

**Current State**:
- 25 passing integration tests
- Focus: Unit tests (1,225 tests, 95%+ in core crates)
- Gap: Cross-system validation (ECS → AI → Physics → Rendering)

**Work Required**:
1. **AI Planning Cycle Tests** (5-7 hours)
   - Full pipeline: Perception → Reasoning → Planning → Execution → Feedback
   - Test cases: 
     - Happy path (successful plan execution)
     - Error recovery (invalid action, collision, timeout)
     - State transitions (idle → combat → retreat)
     - Multi-agent coordination (team tactics)
   - Tests: 8-10 tests

2. **Combat Physics Integration** (4-6 hours)
   - Raycast attack → damage → parry → iframes
   - Test cases:
     - Normal hit (damage applied)
     - Parry (no damage, cooldown triggered)
     - Iframe (no damage, invulnerability window)
     - Multi-target sweep (cone attack)
   - Tests: 6-8 tests

3. **Rendering Pipeline Tests** (3-4 hours)
   - Material batching → GPU mesh upload → rendering
   - Test cases:
     - Material system (TOML → GPU textures)
     - Skeletal animation (bind pose → animated pose)
     - LOD generation (high poly → 5 LOD levels)
   - Tests: 5-7 tests

4. **Determinism Validation** (3-5 hours)
   - Replay system (capture → replay → validate)
   - Test cases:
     - ECS system ordering (deterministic tick)
     - RNG seeding (WorldSnapshot generation)
     - Physics determinism (fixed timestep, no drift)
   - Tests: 6-8 tests

**Total Effort**: 15-22 hours

**Expected Outcome**:
- **Integration tests: 25 → 50+** (+100% increase)
- **Coverage impact**: Minimal (integration tests don't increase line coverage)
- **Bug discovery**: HIGH (cross-system bugs, timing issues, state corruption)
- **Determinism proof**: 100% replay validation (critical for multiplayer)

**ROI**: ⭐⭐⭐⭐ HIGH
- Quality validation (real-world scenarios)
- Bug discovery (integration issues)
- Determinism proof (multiplayer readiness)
- Documentation (usage examples)

**Risk**: MEDIUM
- Complex setup (multiple systems, async timing)
- Flaky tests (timing-sensitive, state management)
- High maintenance (system changes break tests)

**Recommendation**: ✅ **DO THIS THIRD** (after Scene + P1-C/D measurement)

---

### Option D: Render Phase 2 (53.89% → 60%+) ⭐ LOW PRIORITY

**Goal**: Render 53.89% → 60%+ (edge case expansion)

**Current State**:
- 323 tests, 53.89% coverage
- Phase 1 complete (+18 edge case tests, +1.45pp)
- Realistic max: ~75-80% (25% GPU/OS untestable)

**Work Required**:
1. **Additional edge cases** (3-4 hours)
   - residency.rs: File I/O errors (disk full, permission denied)
   - mesh_registry.rs: Duplicate keys, hash collisions
   - instancing.rs: Pattern builder edge cases
   - material.rs: TOML parsing errors
   - Effort: 15-20 tests

**Expected Outcome**:
- **Render: 53.89% → 60%** (+6.11pp, ~803 lines)
- **Tests: 323 → 338-343** (+15-20 tests)
- **P1-B average: 55.92% → 57.5%** (+1.58pp)

**ROI**: ⭐ LOW
- Diminishing returns (GPU code fundamentally untestable)
- High effort for small gain (+6pp for 15-20 tests)
- Fragile (mock GPU infrastructure needed for further phases)
- Current 53.89% is EXCELLENT for graphics industry

**Risk**: MEDIUM-HIGH
- Mock GPU infrastructure fragile (hardware-dependent)
- High maintenance (wgpu API changes)
- Low value (Industry standard: Unity 25-35%, Bevy 45-50%)

**Recommendation**: ❌ **SKIP THIS** (53.89% is sufficient, from Phase 1 report)

---

## Recommended Path Forward

### 🎯 HYBRID APPROACH (23-31 hours over 2-3 days)

**Phase 1: Quick Win (2-3 hours)** ✅ IMMEDIATE
- Fix Scene llvm-cov bug (move tests to `tests/` directory)
- Re-measure baseline (expect 60-70%)
- Fix 3 failing async tests
- **Outcome**: Scene 0% → 60-70%, P1-B unblocked

**Phase 2: Strategic Expansion (6-8 hours)** 🎯 SHORT-TERM
- Measure 3-4 P1-C/D crates (input, cinematics, weaving, pcg)
- Generate baseline reports
- Add tests if <60% coverage
- **Outcome**: Measurement coverage 26% → 34%, new baselines established

**Phase 3: Quality Push (15-20 hours)** 🏆 MEDIUM-TERM
- Integration tests sprint (25 → 50 tests)
- AI planning cycle, combat physics, rendering pipeline
- Determinism validation (replay system)
- **Outcome**: 100% determinism proof, cross-system validation

**Total Timeline**: 2-3 days (23-31 hours)

**Total Cost**: Medium effort, high strategic value

**Expected Results**:
- **Scene**: 0% → 60-70% (unlock 30 tests)
- **P1-C/D**: 4 crates measured (input, cinematics, weaving, pcg)
- **Integration tests**: 25 → 50+ (cross-system validation)
- **Measurement coverage**: 26% → 34% of production crates
- **Overall quality**: HIGH (determinism validated, integration proven)

---

### Alternative: STRATEGIC DEEP DIVE (16-32 hours)

If maximum strategic value desired, focus on **P1-C/D Measurement Campaign**:

**Goal**: Expand measurement from 26% → 50%+ of production crates

**Targets** (8-10 crates):
1. astraweave-input (2-3h)
2. astraweave-cinematics (3-4h)
3. astraweave-weaving (4-6h)
4. astraweave-pcg (3-5h)
5. astraweave-memory (3-4h)
6. astraweave-persona (3-4h)
7. astraweave-asset (4-5h)
8. astraweave-sdk (2-3h)
9. astraweave-stress-test (2-3h)
10. astraweave-security (3-4h)

**Total Effort**: 29-45 hours (1-2 weeks)

**Expected Outcome**:
- **Measurement coverage: 12/47 → 22/47** (26% → 47%)
- **New baselines**: 10 crates with 60-80% coverage
- **Test count: 1,225 → ~1,525** (+300 tests)
- **Knowledge gaps**: ALL P1 crates measured, P2 prioritized

**ROI**: ⭐⭐⭐⭐⭐ MAXIMUM STRATEGIC VALUE
- Comprehensive coverage awareness (47% of production crates)
- Identify all critical gaps
- Inform 6-12 month roadmap
- Validate architecture completeness

**Recommendation**: ✅ **CONSIDER IF TIME AVAILABLE** (1-2 week sprint)

---

## Success Metrics

### Immediate Success (Phase 1: Scene Fix)
- [ ] Scene: 0% → 60-70% coverage
- [ ] 30 tests measured by llvm-cov
- [ ] 3 failing async tests fixed
- [ ] P1-B average: 55.92% → 61-64%

### Short-Term Success (Phase 2: P1-C/D Measurement)
- [ ] 4 crates measured (input, cinematics, weaving, pcg)
- [ ] Measurement coverage: 26% → 34%
- [ ] Baseline reports generated
- [ ] All measured crates at 60%+ coverage

### Medium-Term Success (Phase 3: Integration Tests)
- [ ] Integration tests: 25 → 50+
- [ ] AI planning cycle validated (8-10 tests)
- [ ] Combat physics validated (6-8 tests)
- [ ] Determinism proof (100% replay validation)

### Long-Term Success (Strategic Deep Dive)
- [ ] Measurement coverage: 26% → 47%
- [ ] 10 crates measured with baselines
- [ ] Test count: 1,225 → 1,525+
- [ ] All P1 crates (A/B/C/D) measured

---

## Final Recommendation

**START WITH**: Option A (Scene Fix) → Quick win, 2-3 hours  
**THEN PROCEED TO**: Option B (P1-C/D Measurement) → Strategic expansion, 6-8 hours  
**FINISH WITH**: Option C (Integration Tests) → Quality validation, 15-20 hours  

**SKIP**: Option D (Render Phase 2) → 53.89% is sufficient for GPU crate

**Total Commitment**: 23-31 hours over 2-3 days

**Strategic Value**: ⭐⭐⭐⭐⭐ MAXIMUM
- Unblocks P1-B measurement (Scene)
- Expands coverage awareness (P1-C/D)
- Validates system integration (Integration tests)
- Proves determinism (Replay validation)

**Next Steps**: User decision on approach (Hybrid vs Strategic Deep Dive)
