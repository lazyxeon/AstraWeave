# What Didn't Work: Failed Approaches & Pivots

**Context**: This document captures failed experiments, abandoned approaches, and lessons learned the hard way during 40+ days of AI-orchestrated development.

---

## Architecture Decisions

### 1. ❌ Parallel ECS Execution (Week 8 Failure)

**What we tried**: Parallelize ECS system execution with Rayon

**Why it failed**:
- **Amdahl's Law**: Only 0.15-22.4% work is parallelizable
- **Overhead too high**: Rayon adds 50-100 µs (worse than serial)
- **Sequential dependencies**: 59% ECS overhead is inherently sequential
- **Max speedup**: 1.24× best case (not worth complexity)

**Evidence**:
- **Week 8 analysis**: 516 µs → 416 µs best case (-19%), but overhead makes it slower
- **Current**: Single-threaded execution validated (~0.97 ms system / ~0.71 ms mimalloc @ 1,000 entities; 2.70 ms / 370 FPS was the Week-8 target) <!-- Source: CLAIMS_REGISTRY.md#frame-time-1000-entities -->

**Lesson**: Don't parallelize small workloads (<5 ms). Overhead dominates.

**What worked instead**: Spatial hash + SIMD movement (sequential optimizations)

---

### 2. ❌ Phi-3 LLM (Phase 6 → Phase 7 Pivot)

**What we tried**: Use Phi-3-mini for AI orchestration

**Why it failed**:
- **40-50% JSON quality** (frequent parse failures)
- **Verbose output**: Included explanations instead of pure JSON
- **Tool selection issues**: 60% success rate on correct tool choice

**Evidence**:
- **Phase 6**: 54 compilation errors from API mismatches
- **Phase 7**: Pivoted to Hermes 2 Pro → 75-85% success rate

**Lesson**: Model selection matters. Instruction-tuned models > general chat models.

**What worked instead**: Hermes 2 Pro (adrienbrault/nous-hermes2pro:Q4_K_M) as an opt-in model via `OLLAMA_MODEL`; note the runtime default remains `phi3:medium` (Hermes/Qwen3 are not the live default)

---

### 3. ❌ Mock LLM for Production Validation (Phase 7 Bug)

**What we tried**: Use MockLLM for hello_companion validation

**Why it failed**:
- **Hid critical bugs**: Case sensitivity validation bug undetected
- **False confidence**: Tests passed, but real LLM failed
- **No real-world stress**: Mock doesn't exercise full pipeline

**Evidence**:
- **Phase 7**: 0% LLM success rate discovered only when testing with real model
- **Root cause**: MockLLM bypassed enum validation (snake_case vs PascalCase)

**Lesson**: Mocks hide bugs. Test with real systems early.

**What worked instead**: Live Ollama validation in Phase 7 (caught bug immediately)

---

## Performance Optimization

### 4. ❌ Premature Optimization (Week 1-2 Temptation)

**What we tried**: Optimize AI planning (already 87-202 ns)

**Why it failed**:
- **Already fast enough**: 4.95-11.5M plans/sec >> 60 FPS requirement
- **Wasted time**: Could've fixed ECS regression instead
- **No budget pressure**: AI using <1% of frame budget

**Evidence**:
- **Week 2**: AI benchmarks showed sub-microsecond planning
- **Week 8**: AI still not bottleneck (ECS was the issue)

**Lesson**: Measure budget % before optimizing. Don't optimize what's already fast.

**What worked instead**: Tracy profiling → spatial hash (optimized what matters)

---

### 5. ❌ Hand-Written SIMD (Week 5 Consideration)

**What we tried**: Write AVX2 intrinsics manually

**Why it didn't happen**:
- **glam auto-vec is 80-85% performance** (good enough)
- **Maintenance burden**: Platform-specific code (x86, ARM, WASM)
- **Unsafe code**: Requires unsafe blocks (complexity risk)

**Evidence**:
- **Week 8**: 2.08× speedup with auto-vectorization (no hand-written SIMD)
- **Week 5**: Decided to trust compiler (correct call)

**Lesson**: Auto-vectorization is usually good enough. Only hand-optimize if <50% theoretical peak.

**What worked instead**: BATCH_SIZE=4 + loop unrolling + glam auto-vec

---

## Development Process

### 6. ❌ Scattered API Changes (Phase 6 Pain)

**What we tried**: Fix API issues one file at a time

**Why it failed**:
- **Cascading errors**: One fix breaks another file
- **Context loss**: Forgot what was already fixed
- **Inefficient**: 54 errors took multiple attempts

**Evidence**:
- **Phase 6 early attempts**: Piecemeal fixes → new errors
- **Phase 6 success**: Created one corrected core_loop.rs → clean build

**Lesson**: For API migrations, fix comprehensively (not incrementally).

**What worked instead**: Complete file rewrite with correct API (Phase 6 success)

---

### 7. ❌ Test-Driven Everything (Week 2 Lesson)

**What we tried**: Write tests before implementation

**Why it partially failed**:
- **Some tests needed implementation first**: Integration tests require working systems
- **Over-testing simple code**: 100% coverage not always valuable
- **Time investment**: Week 2 spent 50% time on tests (diminishing returns)

**Evidence**:
- **Week 2**: 111 tests written, but 95.5% would've been sufficient
- **Week 3**: Focused on integration tests (higher value)

**Lesson**: Test high-risk code first (AI planning, determinism, API boundaries). Don't test trivial getters.

**What worked instead**: Risk-based testing (integration > unit for complex systems)

---

### 8. ❌ Deferring Warnings (Week 1-2 Approach)

**What we tried**: Ignore warnings, fix later

**Why it failed**:
- **Technical debt accumulation**: 14 warnings by Week 3
- **Harder to fix later**: Context lost, more errors
- **CI friction**: Warnings → failed builds eventually

**Evidence**:
- **Week 3 Day 1**: Spent 0.2h fixing 14 accumulated warnings
- **Week 3+**: ZERO warnings policy → 18-day streak

**Lesson**: Fix warnings immediately (like compilation errors).

**What worked instead**: Zero-warning policy from Week 3 onward

---

## Tool Selection

### 9. ❌ egui 0.28 + winit 0.29 (Phase 8.1 Week 1 Issue)

**What we tried**: Keep old egui/winit versions

**Why it failed**:
- **API breaking changes**: winit 0.30 required major refactor
- **Feature gaps**: Newer versions have better UI widgets
- **Technical debt**: Delaying migration makes it harder

**Evidence**:
- **Week 1 Day 2**: Migration took 1.5h (moderate pain)
- **Week 1 Day 3+**: Zero warnings after migration

**Lesson**: Migrate dependencies proactively (not reactively).

**What worked instead**: winit 0.30 + egui 0.32 migration (painful but necessary)

---

### 10. ❌ Rhai Scripting (astraweave-author)

**What we tried**: Rhai for modding/scripting

**Why it failed**:
- **Sync trait issues**: Rhai 1.19 → 1.20 broke compatibility
- **Low priority**: No immediate need for scripting
- **Maintenance burden**: External dependency with breaking changes

**Evidence**:
- **Current status**: astraweave-author compiles clean (`cargo check --workspace` passes 130/130, ci-excludes list empty)
- **Examples**: rhai_authoring compiles clean

**Lesson**: Don't add dependencies unless immediate need. Scripting can wait.

**What worked instead**: Focus on core engine (defer scripting to Phase 9+)

---

## Testing Strategy

### 11. ❌ Testing Everything with --nocapture (Week 2)

**What we tried**: Always run tests with `--nocapture` flag

**Why it partially failed**:
- **Noisy output**: Hard to find actual failures
- **Slower**: Printf debugging slows test execution
- **Not always needed**: Most tests don't need stdout

**Evidence**:
- **Week 2**: Used --nocapture for all tests (verbose)
- **Week 3+**: Selective --nocapture only for debugging

**Lesson**: Use --nocapture only when debugging specific failures.

**What worked instead**: Clean test runs by default, --nocapture on-demand

---

### 12. ❌ Clippy with -D warnings from Day 1 (Week 1 Temptation)

**What we tried**: Enforce clippy from start

**Why it didn't happen**:
- **Too strict early**: Blocks rapid prototyping
- **Bikeshedding risk**: Spend time on style vs functionality
- **Incremental better**: Add clippy after features stable

**Evidence**:
- **Week 1-3**: No clippy enforcement (rapid development)
- **Week 3+**: Added clippy incrementally (reasonable pace)

**Lesson**: Enforce linting after features stable (not during rapid prototyping).

**What worked instead**: Clippy added in Week 3 after core features working

---

## Documentation

### 13. ❌ Flat docs/ Directory (Root-Level Clutter)

**What we tried**: Put all docs in docs/ or root without structure

**Why it failed**:
- **Impossible to navigate**: 300+ files in flat directories
- **No context**: Hard to know what's current vs historical
- **Overwhelming newcomers**: Can't find anything

**Evidence**:
- **October 20, 2025**: Reorganization required 1.7h (moved 90+ files)
- **Before**: 300+ files in docs/ and docs/root-archive/
- **After**: 5-category structure (current, journey, lessons, supplemental)

**Lesson**: Structure documentation early (don't accumulate technical debt).

**What worked instead**: 5-category structure with navigation guide (docs/journey/README.md)

---

### 14. ❌ Prose-Only Documentation (Week 1-2)

**What we tried**: Write explanations without code examples

**Why it failed**:
- **Hard to understand**: Abstract patterns without concrete code
- **More questions**: Users ask "how do I...?" 
- **Slower onboarding**: No copy-paste examples

**Evidence**:
- **Week 3**: Added 23+ code examples to API docs (much clearer)
- **Phase 6**: Added correct/incorrect usage examples (prevented mistakes)

**Lesson**: Always include code examples (correct + incorrect usage).

**What worked instead**: Three-tier docs (prose + examples + cheat sheets)

---

## Collaboration

### 15. ❌ Assuming AI Knows Context (Week 1 Mistake)

**What we tried**: Brief prompts without full context

**Why it failed**:
- **Incorrect code**: AI guesses wrong API
- **More iterations**: Have to fix and re-prompt
- **Wasted time**: Clarification loops

**Evidence**:
- **Week 1**: Some API mismatches (brief prompts)
- **Week 3+**: copilot_instructions.md → consistent quality

**Lesson**: Provide full context (copilot_instructions.md, code samples, error messages).

**What worked instead**: Comprehensive copilot_instructions.md (1,000+ lines)

---

### 16. ❌ Deleting Historical Documentation (Temptation)

**What we tried**: Consider deleting old completion reports

**Why we didn't**:
- **Evidence of journey**: 40-day timeline proves AI capability
- **Learning resource**: Others can study our process
- **Validation**: Complete history validates GCP methodology

**Evidence**:
- **Reorganization**: Moved 90+ files (didn't delete any)
- **docs/journey/**: Preserved ALL historical documentation

**Lesson**: Never delete documentation in open-source experiments. Archive instead.

**What worked instead**: docs/journey/ structure (preserve + organize)

---

## Build System

### 17. ❌ Full Workspace Builds Without Exclusions (Phase 1 Pain)

**What we tried**: `cargo build --workspace` (no exclusions)

**Why it failed**:
- **Broken crates fail**: (historical) some crates formerly failed to compile; astraweave-author and rhai_authoring now compile clean (`cargo check --workspace` passes 130/130)
- **Wasted time**: Re-compile everything repeatedly
- **CI failures**: Broken crates block CI

**Evidence**:
- **Phase 1**: Multiple failed build attempts
- **Current**: editor aliases (`editor`, `editor-release`, `editor-dev`) plus explicit per-crate commands (`cargo check -p <crate>`) and `cargo check --workspace`

**Lesson**: Exclude broken crates early (don't let them block development).

**What worked instead**: editor cargo aliases (`editor`, `editor-release`, `editor-dev`) plus explicit `cargo check -p <crate>` / `cargo check --workspace`

---

### 18. ❌ Canceling Long-Running Builds (Week 1 Temptation)

**What we tried**: Cancel wgpu dependency compilation

**Why it failed**:
- **Repeated work**: Have to start over
- **Broken cache**: sccache corrupted by cancels
- **Slower overall**: 15 min wait once > 5 min cancels × 3

**Evidence**:
- **Week 1**: Initial build took 15-45 min (normal for Rust graphics)
- **Current**: Incremental builds 8-15 seconds

**Lesson**: Let initial builds complete (dependency compilation takes time).

**What worked instead**: Patience + sccache (incremental builds fast after first)

---

## Measurement & Verification

### 19. ❌ Believing a Metric That Was Never Challenged (L.3.B, 2026-07-29)

**What we tried**: Diagnose a far-field shadow artefact by tracking a metric — first an
aggregate over a screen band, then per-feature luma of world-anchored points — and report
the result to the director as soon as it moved (or didn't).

**Why it failed**:
- **Insensitive aggregate**: the band metric averaged mostly near/mid-field pixels, which
  cascades 0/1 own and which were never suspect. The far-field signal was swamped. On that
  basis texel snapping was reported REFUTED.
- **Contaminated per-feature metric**: at 40 m/frame a tracked world point's patch moves
  several luma from terrain mip level, hex-tile phase and LOD alone (measured: 6.35 luma at
  a point whose shadow factor was provably constant). On that basis texel snapping was then
  reported as THE FIX.
- Both reports were retracted. The proof they were artefact: three builds — baseline,
  snapped, and cull-disabled — produced the **identical** 13.24 / 22.57 figures, and two of
  the three were byte-identical frames.

**Evidence**:
- L.3.B: two public retractions in one beat (`docs/audits/L3B_OUTCOME.md` §3).
- The director independently retracted a "discrete stepping between frames" claim from
  frame analysis of a screen recording — 1 Hz sampling cannot support a per-frame claim.
  **Both sides of the review made the same class of error in the same beat.**

**Lesson**: **Verify that a metric RESPONDS to a known change before believing what it says
about an unknown one.** Feed it a change whose sign you already know — disable the feature,
break it deliberately, or compare against a byte-identical rebuild. A metric that cannot
move is not evidence, and a metric that moves for reasons other than the effect is worse
than none. State the sampling rate of any observation before drawing a conclusion from it.

**What worked instead**: **Twin deterministic flights, differenced.** The identical camera
path is flown twice, shadows on and shadows off, and the frames are subtracted
(`off[i] − on[i]`). Everything view-dependent that is not shadow cancels exactly, leaving
the shadow field alone. That instrument survived realistic flight speed and is the metric
of record for the whole arc (`scripts/l3b/l3b_shadow_diff.py`).

---

### 20. ❌ Pricing a System From a Model of Itself (L.3.C, 2026-08-01)

**What we tried**: Price cascade quality (minimum castable relief) from a calibrated MODEL
of the renderer's shadow-cascade fits — sphere radius, ortho pad, depth range, texel size —
rather than from the fits the renderer emits.

**Why it failed**: The model was fed the wrong ortho pad for one cascade. `FIRST_CACHED_CASCADE
= 2` means BOTH far cascades carry the 400 m drift pad, but the layout was priced with the
2 m every-frame pad on the inner one. Consequences, all published before being caught:
- c2's texel understated 1.908 → 1.520 m, its depth range 5112 → 4714 m, and therefore its
  minimum castable relief **3.39 → 2.82 m**;
- the headline seam recovery overstated (500 m seam loss 26.7 pp reported as 20.5 pp);
- a shader constant (`C2_BIAS_SCALE`) derived from the wrong depth range, leaving that
  cascade's receiver bias 8.5% looser than the value it was meant to cap at.

**Evidence**: `docs/audits/L3C_OUTCOME.md` §2 (corrected in place, with the original figures
retained and labelled — they turned out to describe a configuration worth shipping, which is
why the slip was recoverable rather than merely wrong).

**Lesson**: **A model of the fit is not the fit.** When a downstream number is derived from
values a system computes at runtime, make the system PUBLISH them and check the model against
that. Calibrating a model against a few measured samples (as this one was) proves it
interpolates; it does not prove you fed it the right inputs.

**What worked instead**: `Renderer::shadow_cascade_fits()` — every cascade's near/far,
centre, radius, pad, depth range and texel size exactly as computed, printed by the perf
harness alongside the policy in force, so a published quality figure can be traced to
ground truth.

---

## Conclusion

**Key Insight**: Failed approaches are valuable—they guide future decisions

The mistakes we made weren't wasted time—they **validated what works**:

1. ❌ Parallel ECS → ✅ Sequential + spatial hash
2. ❌ Phi-3 LLM → ✅ Hermes 2 Pro
3. ❌ Mock validation → ✅ Real system testing
4. ❌ Premature optimization → ✅ Tracy-guided optimization
5. ❌ Hand-written SIMD → ✅ Auto-vectorization trust
6. ❌ Scattered fixes → ✅ Comprehensive rewrites
7. ❌ Deferring warnings → ✅ Zero-warning policy
8. ❌ Old dependencies → ✅ Proactive migration
9. ❌ Flat docs → ✅ Structured organization
10. ❌ Brief prompts → ✅ Comprehensive instructions

**Evidence**: 40+ days, multiple pivots, zero major regressions

**Next**: See `WHAT_WORKED.md` for successful patterns and `AI_ORCHESTRATION_TIPS.md` for GCP methodology

---

*Last Updated*: January 2026 (October 20, 2025)  
*Extracted from*: 300+ completion reports, post-mortems, and pivot analyses
