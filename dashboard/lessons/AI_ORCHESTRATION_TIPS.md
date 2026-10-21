# AI Orchestration Tips: GitHub Copilot Prompting

**Context**: This document captures lessons learned about effective AI collaboration during 40+ days of building AstraWeave with GitHub Copilot.

---

## Core Principles

### 1. The copilot_instructions.md Pattern ✅

**Pattern**: Maintain comprehensive, evolving instructions file

**Why it works**:
- AI "remembers" context across sessions (persistent memory)
- Consistent code quality (same patterns applied everywhere)
- Self-documenting (instructions evolve with codebase)
- Reduces iteration cycles (no re-explaining)

**What to include**:
```markdown
# copilot_instructions.md

1. Project Overview (what are we building?)
2. Architecture Essentials (key patterns, ECS stages, AI loop)
3. Quick Commands (build, test, benchmark)
4. Working Effectively (what to DO vs DON'T)
5. Common Patterns (code examples with correct/incorrect usage)
6. Critical Warnings (known issues, gotchas)
7. Where to Look (navigation guide for codebase)
8. Next Steps (current priorities, roadmap)
```

**Evidence**:
- **1,000+ line copilot_instructions.md** maintained
- **18-day zero-warning streak** (AI follows established patterns)
- **Zero API regressions** after Phase 6 (consistent usage)

**How to apply**:
```bash
# Update after every major change
1. Add new patterns to copilot_instructions.md
2. Document new APIs with examples
3. Update "Next Steps" section
4. Commit with clear message: "docs: update copilot instructions (Phase X)"
```

---

### 2. Show, Don't Tell (Code Examples > Prose) ✅

**Pattern**: Include code examples in prompts and documentation

**Why it works**:
- Reduces ambiguity (code is precise)
- Faster generation (AI can copy patterns)
- Fewer mistakes (AI sees correct usage)

**Bad prompt** ❌:
> "Fix the AI orchestrator to use the correct API"

**Good prompt** ✅:
> "Fix the AI orchestrator. The correct WorldSnapshot API is:
> ```rust
> snap.me.pos        // NOT snap.my_pos
> snap.me.ammo       // NOT snap.my_stats.ammo
> snap.enemies       // NOT snap.threats
> ```
> Here's the broken code: [paste code]"

**Evidence**:
- **Phase 6**: API fix succeeded after showing correct structs
- **Week 3**: API docs with 23+ examples (clear patterns)

---

### 3. Error-Driven Prompting ✅

**Pattern**: Always include full error messages in prompts

**Why it works**:
- AI sees exact problem (not guessing)
- Faster diagnosis (error message = context)
- Better fixes (addresses root cause)

**How to prompt**:
```markdown
**Context**: Running `cargo check -p astraweave-ai` fails

**Error message**:
```
error[E0609]: no field `my_pos` on type `CompanionState`
  --> astraweave-ai/src/core_loop.rs:42:20
   |
42 |     let pos = snap.me.my_pos;
   |                    ^^^^^^^^ unknown field
```

**Request**: Fix this error. The correct field is `snap.me.pos` (not `my_pos`).
```

**Evidence**:
- **Phase 6**: 54 errors fixed by showing full error messages
- **Phase 7**: Case sensitivity bug found via debug logging

---

### 4. Iterative Validation (Test After Every Change) ✅

**Pattern**: Request `cargo check` after every code generation

**Why it works**:
- Catches errors immediately (not cascading later)
- Builds confidence (always start from working code)
- Prevents technical debt (no "I'll fix it later")

**How to prompt**:
```markdown
Generate the AI orchestrator fix, then:
1. Run `cargo check -p astraweave-ai`
2. If errors, fix them immediately
3. Don't mark task complete until ZERO errors
```

**Evidence**:
- **Zero-error policy**: Enabled 18-day zero-warning streak
- **Phase 6**: Fix-check-fix cycle eliminated 54 errors
- **Week 3**: ZERO warnings maintained (immediate fixes)

---

### 5. Request Comprehensive Fixes (Not Piecemeal) ✅

**Pattern**: Ask AI to fix entire files/modules (not individual lines)

**Why it works**:
- Avoids cascading errors (one fix breaks another)
- Maintains context (AI sees full picture)
- Faster overall (one comprehensive pass > many small fixes)

**Bad prompt** ❌:
> "Fix line 42 in core_loop.rs"

**Good prompt** ✅:
> "Rewrite core_loop.rs to use the correct WorldSnapshot API throughout. Here's the correct API: [examples]. Current file has 12 errors—fix comprehensively."

**Evidence**:
- **Phase 6**: Comprehensive core_loop.rs rewrite → clean build
- **Week 3**: Full API documentation pass (not incremental)

---

## Prompting Techniques

### 6. Three-Tier Requests (Summary → Detail → Examples) ✅

**Pattern**: Structure prompts with executive summary, detailed context, code examples

**Why it works**:
- AI scans summary first (understands goal)
- Falls back to details (full context)
- References examples (correct patterns)

**Template**:
```markdown
## Executive Summary (1-2 sentences)
Fix the AI orchestrator to use the correct WorldSnapshot API.

## Detailed Context (2-3 paragraphs)
The Phase 6 migration changed the WorldSnapshot API:
- `snap.my_pos` → `snap.me.pos`
- `snap.my_stats.ammo` → `snap.me.ammo`
- `snap.threats` → `snap.enemies`

Current code has 12 compilation errors from old API usage.

## Code Examples
**Correct**:
```rust
let pos = snap.me.pos;
let ammo = snap.me.ammo;
```

**Incorrect** (what we have now):
```rust
let pos = snap.my_pos;
let ammo = snap.my_stats.ammo;
```

## Request
Rewrite core_loop.rs with correct API, then run `cargo check -p astraweave-ai`.
```

**Evidence**:
- **Phase 6 docs**: 15,000-word report + quick reference + summary
- **Week 3 docs**: API reference with 3 tiers (details, examples, cheat sheets)

---

### 7. Positive Reinforcement (Celebrate Successes) ✅

**Pattern**: Acknowledge AI achievements in follow-up prompts

**Why it works**:
- Maintains engagement (gamification)
- Reinforces correct patterns (positive feedback)
- Builds momentum (small wins → big wins)

**How to prompt**:
```markdown
**Great work on Phase 6!** 54 errors → 0 errors is a huge achievement.

**Next step**: Let's validate with `hello_companion --demo-all`. Run the example and check that all 6 AI modes work correctly.
```

**Evidence**:
- **18-day zero-warning streak**: Maintained by celebrating milestones
- **40+ day timeline**: Momentum sustained by acknowledging progress

---

### 8. Specify Output Format (Tables, Lists, Code) ✅

**Pattern**: Tell AI what format you want (not just what content)

**Why it works**:
- Reduces ambiguity (AI knows structure expected)
- Easier to parse (consistent formatting)
- Professional output (tables, not prose)

**How to prompt**:
```markdown
Create a completion report with this structure:

## Executive Summary (2-3 sentences)

## Achievements (bullet list with metrics)
- ✅ Feature X: [metric]
- ✅ Feature Y: [metric]

## Metrics Table
| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Frame time | 3.09 ms | 2.70 ms | -12.6% |

## Next Steps (numbered list)
1. Task 1
2. Task 2
```

**Evidence**:
- **300+ completion reports**: Consistent format across all docs
- **Week summaries**: Tables, lists, clear structure
- **Phase reports**: Executive summary + detailed metrics

---

### 9. Multi-Step Instructions (Checklists) ✅

**Pattern**: Break complex tasks into numbered steps

**Why it works**:
- Prevents missed steps (AI follows checklist)
- Clear progress tracking (cross off completed items)
- Easy to resume (know where you left off)

**How to prompt**:
```markdown
**Task**: Documentation reorganization (4 priorities)

**Priority 1: Week Summaries** (6 files)
1. List files: `ls docs/root-archive/WEEK*SUMMARY*.md`
2. Add to git: `git add [files]`
3. Move: `git mv [file] docs/journey/weeks/`
4. Verify: `ls docs/journey/weeks/`

**Priority 2: Daily Logs** (70+ files)
[similar checklist]

**Priority 3: Current Roadmaps** (13 files)
[similar checklist]

**Priority 4: Create Lessons** (5 files)
[similar checklist]
```

**Evidence**:
- **Documentation reorganization**: 4-priority checklist (90+ files moved successfully)
- **Week 3 testing**: 5-day checklist (all tasks completed)

---

### 10. Request Validation Steps ✅

**Pattern**: Ask AI to include validation in generated code/scripts

**Why it works**:
- Catches errors early (validation before commit)
- Builds confidence (know it works)
- Reduces debugging (problems found immediately)

**How to prompt**:
```markdown
Generate the spatial hash optimization, then:

**Validation steps**:
1. Run benchmark: `cargo bench -p astraweave-physics --bench collision`
2. Check regression: Should show -99% collision checks
3. Run tests: `cargo test -p astraweave-physics`
4. Profile: `cargo run -p profiling_demo --release -- --entities 1000`
5. Verify frame time improvement: Should be <3 ms

Don't mark complete until all 5 validations pass.
```

**Evidence**:
- **Week 8**: Tracy validation after every optimization
- **Week 3**: 242 tests passing (100% validation)

---

## Advanced Patterns

### 11. Contextual Anchors (Link to Prior Work) ✅

**Pattern**: Reference previous completions to build on context

**Why it works**:
- AI recalls past work (no starting from scratch)
- Maintains consistency (builds on established patterns)
- Faster iteration (don't re-explain)

**How to prompt**:
```markdown
**Context**: Continue from Week 3 Day 4 completion (API documentation)

**Reference**: See `WEEK_3_DAY_4_COMPLETION_REPORT.md` for what was done

**Task**: Now create Week 3 Day 5 validation report with:
- Test all 23 API examples from Day 4
- Verify integration patterns work
- Document any issues found
```

**Evidence**:
- **40+ day timeline**: Each day builds on previous work
- **Phase 6 → Phase 7**: Smooth continuation (context preserved)

---

### 12. Request Decision Trees (For Complex Choices) ✅

**Pattern**: Ask AI to document decision-making process

**Why it works**:
- Transparency (know why choices made)
- Reusable (apply pattern to future decisions)
- Educational (learn AI reasoning)

**How to prompt**:
```markdown
**Task**: Choose between parallel ECS vs sequential optimization

**Request**: Create decision tree with:
1. Option A: Parallel ECS (Rayon)
   - Pros: [list]
   - Cons: [list]
   - Estimated gain: [metric]
2. Option B: Sequential optimization (spatial hash)
   - Pros: [list]
   - Cons: [list]
   - Estimated gain: [metric]
3. Recommendation: [A or B] because [reasoning]
```

**Evidence**:
- **Week 8**: Parallel ECS decision tree (chose sequential)
- **Phase 6-7**: LLM selection decision tree (Phi-3 → Hermes 2 Pro)

---

### 13. Meta-Documentation Requests ✅

**Pattern**: Ask AI to document its own process

**Why it works**:
- Captures lessons (what worked/didn't)
- Improves future prompts (learn from history)
- Validates GCP methodology (AI can explain itself)

**How to prompt**:
```markdown
**Task**: Create AI_ORCHESTRATION_TIPS.md

**Content**:
- Extract prompting patterns from our 40-day journey
- Document what made prompts effective
- Include examples (good vs bad prompts)
- Explain GCP principles we followed

**Meta-goal**: This document will teach others how to orchestrate AI effectively.
```

**Evidence**:
- **This document**: AI-generated guide to AI orchestration
- **300+ docs**: All created via effective prompting

---

### 14. Progressive Disclosure (Start Simple, Add Complexity) ✅

**Pattern**: Begin with simple prompt, refine with follow-ups

**Why it works**:
- Prevents overwhelming AI (too much context = confusion)
- Allows course correction (adjust based on output)
- Iterative refinement (each pass improves quality)

**How to prompt**:
```markdown
**Step 1** (simple):
"Create basic AI orchestrator"

**Step 2** (refine after seeing output):
"Add tool sandbox validation to orchestrator"

**Step 3** (refine again):
"Integrate orchestrator with ECS system stages"

**Step 4** (finalize):
"Add metrics tracking and export functionality"
```

**Evidence**:
- **Phase 6**: API migration in 3 passes (analyze → fix → validate)
- **Week 3**: Testing sprint in 5 daily iterations (each built on previous)

---

## Common Pitfalls (What NOT to Do)

### 15. ❌ Vague Prompts Without Context

**Bad prompt**:
> "Fix the AI stuff"

**Why it fails**: AI doesn't know what "AI stuff" means, what's broken, or how to fix it

**Good prompt**:
> "Fix `astraweave-ai/src/core_loop.rs` to use the correct WorldSnapshot API. Error: `no field 'my_pos'`. Correct field: `snap.me.pos`."

---

### 16. ❌ Assuming AI Knows Latest State

**Bad prompt**:
> "Continue where we left off"

**Why it fails**: AI doesn't recall session state (needs explicit context)

**Good prompt**:
> "Continue from Week 3 Day 4 (API documentation complete). Next task: Week 3 Day 5 validation. See `WEEK_3_DAY_4_COMPLETION_REPORT.md` for context."

---

### 17. ❌ Requesting Without Validation Steps

**Bad prompt**:
> "Generate the spatial hash optimization"

**Why it fails**: No way to know if it works (could be broken)

**Good prompt**:
> "Generate spatial hash optimization. Validate with: `cargo bench --bench collision`, `cargo test -p astraweave-physics`, `cargo run -p profiling_demo`. Should show -99% collision checks and <3 ms frame time."

---

### 18. ❌ Ignoring Error Messages in Prompts

**Bad prompt**:
> "Fix the build errors in astraweave-ai"

**Why it fails**: AI doesn't see errors (guesses what might be wrong)

**Good prompt**:
> "Fix these 3 errors in astraweave-ai:
> ```
> error[E0609]: no field `my_pos` on type `CompanionState`
> error[E0609]: no field `my_stats` on type `CompanionState`
> error[E0609]: no field `threats` on type `WorldSnapshot`
> ```
> Correct fields: `snap.me.pos`, `snap.me.ammo`, `snap.enemies`."

---

## GCP (GitHub Copilot Prompting) Methodology

### Core GCP Principles

**GCP = Prompt Engineering for Software Development**

1. **Context is King**: copilot_instructions.md + error messages + code examples
2. **Iterate Quickly**: Fix-check-fix cycle (not big-bang changes)
3. **Validate Everything**: cargo check/test after every change
4. **Document Continuously**: Completion reports capture lessons
5. **Celebrate Progress**: Positive reinforcement maintains momentum

**Evidence**: 40+ days, 300+ docs, 370 FPS, 12,700+ agents, 75-85% LLM success, 100% determinism

---

### The GCP Workflow

```
1. Clear Prompt (context + examples + validation)
   ↓
2. AI Generates (code + docs + tests)
   ↓
3. Immediate Validation (cargo check/test/bench)
   ↓
4. Iterate if Needed (fix-check-fix cycle)
   ↓
5. Document Success (completion report)
   ↓
6. Update copilot_instructions.md (capture patterns)
   ↓
7. Repeat (next task builds on this)
```

**Key Insight**: Each cycle improves AI quality (context accumulates)

---

## Conclusion

**Key Insight**: Effective AI collaboration is a skill (learned through practice)

The prompts that work best are:
1. ✅ **Comprehensive context** (copilot_instructions.md)
2. ✅ **Code examples** (correct + incorrect usage)
3. ✅ **Error messages** (full context for diagnosis)
4. ✅ **Validation steps** (cargo check/test/bench)
5. ✅ **Iterative refinement** (fix-check-fix cycles)
6. ✅ **Positive reinforcement** (celebrate milestones)
7. ✅ **Documentation requests** (capture lessons)

**Evidence**: 40+ days, 1,000+ line copilot_instructions.md, 18-day zero-warning streak, 300+ docs

**Next**: See `WHAT_WORKED.md` for successful patterns and `PERFORMANCE_PATTERNS.md` for optimization lessons

---

*Last Updated*: January 2026 (October 20, 2025)  
*Extracted from*: 40+ days of AI orchestration experience building AstraWeave
