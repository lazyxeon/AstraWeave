# Architecture Trace Deep Investigation Prompt

> **Purpose:** Paste this prompt into Claude Code (or another agentic coding tool with direct repo access) to perform a thorough investigation pass on an architecture trace document. Unlike the verification prompt, this pass is empowered to *close* Open Questions when investigation produces unambiguous factual answers.

---

## When to use this prompt

This is the **third pass** in the trace workflow, used when:

- A trace doc has been generated and verified (markers checked)
- Open Questions remain that *might* be resolvable through deeper investigation
- You want a comprehensive analysis pass that gathers maximum context before manual review

**Do NOT use this as the first pass on a fresh trace doc.** Use it after generation and at least one verification pass, when the doc is structurally settled and the remaining unknowns are about deeper system properties.

---

## Usage

1. Open Claude Code with the AstraWeave repo as the working directory.
2. Copy the prompt below.
3. Replace `{{TRACE_DOC_PATH}}` with the doc to investigate.
4. Optionally fill `{{OPTIONAL_CONTEXT}}` with focus areas or known sensitivities.
5. Send. Review the agent's findings carefully — especially any closed Open Questions — before accepting changes.

---

## The Prompt

```
You are performing a deep investigation pass on an architecture trace document
for the AstraWeave engine. The doc lives at {{TRACE_DOC_PATH}}.

Your job is broader than verification:

1. Investigate the system thoroughly, gathering as much factual context as
   possible from the codebase.
2. Resolve [NEEDS VERIFICATION] and [INFERRED] markers where investigation is
   conclusive (same standard as the verification pass).
3. Where possible, CLOSE Open Questions in Section 11 when investigation produces
   clear, unambiguous factual answers — moving their resolution into the
   appropriate section of the doc.
4. For Open Questions you cannot close, enrich them with factual context Andrew
   needs to make the decision.
5. Surface new findings, questions, or concerns that emerged during investigation.

{{OPTIONAL_CONTEXT}}

## Required reading before you start

1. The trace doc at {{TRACE_DOC_PATH}} — read it end to end.
2. The trace template at `docs/architecture/_meta/ARCHITECTURE_TRACE_TEMPLATE.md`.
3. The verification prompt at `docs/architecture/_meta/VERIFICATION_PROMPT.md` —
   the methodology rules there apply to this pass too, with additions below.
4. `CLAUDE.md` at repo root for engine-wide context.
5. Any audit docs, campaign docs, or design docs in `docs/` referenced by the
   trace doc.

## Investigation methodology

### Phase 1: Build a complete file map for the system

Before resolving anything, build your own picture of the system:

1. Identify the primary crate(s) the system lives in.
2. List every file in those crates that touches the system.
3. For each file, note:
   - What it exports (types, functions, traits)
   - What imports/uses it from inside the same crate
   - What imports/uses it from other crates
   - Whether it has tests, benchmarks, or examples
   - Recent git activity (if available)

This gives you the factual substrate for everything else.

### Phase 2: Trace data flows end-to-end

For each pipeline described in Section 2 of the trace doc:
1. Start at the entry point and follow the data through every stage.
2. Verify each stage's claimed inputs and outputs against actual code.
3. Note any branches, conditional paths, or feature flags that affect the flow.
4. Identify any data flows the doc *doesn't* mention — these may be parallel
   paths, fallback paths, or undocumented complexity.

### Phase 3: Resolve markers (same standard as verification pass)

For each [NEEDS VERIFICATION], [INFERRED], or similar marker:
- Investigate using the methodology in `VERIFICATION_PROMPT.md`.
- Resolve where evidence is strong; preserve where it isn't.
- Cite evidence with file:line references.

### Phase 4: Classify and address Open Questions

For each item in Section 11 (Open Questions):

#### Step 4a: Classify the question

A question is **factual** if it can be answered by reading code, running tools,
or examining the codebase — regardless of judgment. Examples:
- "Is file X imported anywhere?"
- "Does function Y have test coverage?"
- "What is the exact interface between system A and system B?"
- "Are there asserts enforcing invariant Z?"

A question is **decisional** if answering it requires human judgment about
priorities, direction, or trade-offs. Examples:
- "Should we delete file X?"
- "Should we rename module Y?"
- "Should the historical audit doc be marked as historical?"
- "What's the long-term plan for transitional path Z?"

A question can be **mixed**: part factual (current state), part decisional
(intended direction). Split these and address each part appropriately.

#### Step 4b: Investigate factual questions thoroughly

For each factual question:
1. Identify what evidence would resolve it.
2. Gather that evidence rigorously (not just one or two searches — be thorough).
3. Determine whether evidence is conclusive.

#### Step 4c: Close factual questions when investigation is conclusive

A factual Open Question may be closed ONLY if ALL of these are true:

1. **The question is purely factual** (no judgment component).
2. **Investigation produced clear, unambiguous evidence.** This means:
   - You searched comprehensively (multiple search strategies, not just one)
   - You found no contradicting evidence
   - There is no reasonable alternative interpretation
3. **You can cite the evidence concretely** with file:line references.
4. **Andrew did not explicitly mark the question as requiring his decision**
   (e.g. if a question contains "should we" or "what's our plan", treat as
   decisional regardless of how factual the underlying state is).

If ALL four hold: close the question and move its resolution into the
appropriate section of the doc (most commonly Section 5 file map status,
Section 6 conflict map, or Section 8 invariants).

If ANY do not hold: leave the question open, but enrich it with the factual
context you gathered.

#### Step 4d: Enrich decisional questions

For decisional questions you cannot close:

1. State the current factual state precisely (with citations).
2. List the realistic options.
3. Note the trade-offs implied by current code (e.g. "Option A would require
   removing N call sites in files X, Y, Z").
4. Do NOT recommend an option. Surface the choice, don't make it.

### Phase 5: Surface emergent findings

During deep investigation you may discover things the original doc missed:

- New cross-system touchpoints not previously documented
- Implicit invariants that should be made explicit
- Performance characteristics that warrant a section update
- Tests that contradict claimed behavior
- Code that contradicts claimed design

For each finding:
- Determine which section of the doc it belongs in.
- If factual and uncontroversial: integrate it into that section.
- If it raises a new question or concern: add a new Open Question.
- If it suggests something might be broken or wrong: flag prominently in your
  report (do not silently "fix" anything in code; this pass touches only docs).

## Closure rules (strict)

Closing an Open Question is an irreversible-feeling action. Apply these rules:

### Rule C1: Closure requires comprehensive search, not minimum search

When closing a factual question, you must have searched comprehensively:
- All `use` statements across the workspace
- Direct path references (`mod`, `path =`)
- Feature flag and conditional compilation paths
- Test files and example files
- Documentation references
- Build scripts and config files

"I grepped twice and found nothing" is not sufficient evidence to close a
question about whether something is used anywhere.

### Rule C2: When in doubt, leave it open

Closing a question falsely is worse than leaving it open longer. If after
thorough investigation you're 80% confident but not 95%+, leave the question
open with your findings noted.

### Rule C3: Document the closure path

When you close a question, the resolution in the doc must include:
- A brief statement of what was determined
- Citations to the evidence
- A note that this question was closed via investigation on [date]

Future readers should be able to see not just the answer, but how it was reached.

### Rule C4: Decisional questions never close through investigation

No amount of factual investigation closes a decisional question. The factual
context enriches the question; it does not answer it. Andrew makes those calls.

## Anti-overreach rules

The methodology rules from the trace prompt and verification prompt still apply:

- **Rule 1 (Observation over inference)**: Every claim cites evidence.
- **Rule 2 (Mark uncertainty)**: Where evidence is thin, mark it.
- **Rule 3 (Don't fabricate rationale)**: Reasoning not in code/docs stays unrecovered.
- **Rule 4 (Respect what's working)**: This is a documentation pass, not a refactor pitch.
- **Rule 5 (Distinguish "wrong" from "not how I'd write it")**: Unusual patterns
  default to "there's a reason I don't see."
- **Rule 6 (Conflict map is forensic, not editorial)**: Document situations, don't
  propose disposals.
- **Rule 7 (No restructuring)**: Don't add, remove, or reorganize sections.
- **Rule 8 (No code changes)**: This pass touches only the trace doc. Code-level
  problems become urgent findings in the report.

## Update rules

When updating the trace doc:

1. For resolved markers: same as verification pass — replace marker with verified
   text and citation.
2. For closed Open Questions: move the resolution into the appropriate section
   AND remove the question from Section 11. The question disappears once resolved.
3. For enriched (but not closed) Open Questions: rewrite the question to include
   the factual context gathered. The question stays in Section 11.
4. For new findings integrated into existing sections: add with citation.
5. For new findings that became new Open Questions: add to Section 11.
6. Update metadata block:
   - Bump version (e.g. 1.1 → 1.2)
   - Update last verified commit and date
   - Add a note in "Owner notes" about the deep investigation pass

## Reporting requirements

Your report must include:

### Summary
- Number of markers resolved
- Number of Open Questions closed
- Number of Open Questions enriched (kept open with new context)
- Number of new Open Questions added
- Number of new findings integrated into existing sections
- Number of urgent findings (issues requiring attention beyond doc updates)

### Closed Open Questions
For each closed question:
- Original question text
- What you investigated
- Evidence found (with citations)
- Final resolution
- Where in the doc the resolution now lives

### Enriched Open Questions
For each enriched-but-not-closed question:
- Original question
- Factual context gathered
- Why it remains open (e.g. "decisional — requires Andrew's call on direction X")

### New Open Questions
For each new question added:
- The question
- What investigation surfaced it
- Why it needs human attention

### Urgent findings
- Anything suggesting actual broken behavior, dead code being depended on,
  missing critical tests, or other real problems beyond documentation.

### Methodology notes
- Anything about the investigation process worth noting (e.g. "git history was
  unavailable so historical context is limited", or "feature flags made tracing
  difficult in module X")

## Self-check before finishing

1. For every Open Question I closed: did I apply ALL four closure conditions?
2. For every closure: did I cite concrete evidence with file:line references?
3. For every closure: did I integrate the resolution into the appropriate
   doc section (not just delete the question)?
4. Did I avoid closing any decisional questions?
5. For every Open Question I left open: did I enrich it with what I found?
6. Did I avoid restructuring the doc?
7. Did I avoid making editorial recommendations in the doc body?
8. Did I avoid proposing code changes?
9. Is the metadata block updated?
10. Does my report clearly distinguish factual closures from decisional enrichments?

If any answer is "no," fix before submitting.

Now: read the template, read the verification prompt, read the trace doc, read
CLAUDE.md, and begin deep investigation.
```

---

## Customization notes

### When to scope the investigation

For docs with many Open Questions, scoping prevents marathon sessions. Use `{{OPTIONAL_CONTEXT}}` to direct focus:

> Focus this investigation on the file status questions only. Cross-system touchpoints can wait for a future pass.

### When the agent closes something you disagree with

If the agent closes an Open Question and you disagree with the closure, the fix is straightforward:

1. Reopen the question (re-add it to Section 11) with a note about why the closure was premature.
2. Run the verification prompt against the doc to ensure no marker or claim was incorrectly resolved as a consequence.
3. Consider whether the closure rules in this prompt need tightening — if the agent keeps making the same mistake, the rules aren't strong enough.

### How this pass relates to the other two

The three-pass model for a trace doc lifecycle:

1. **Generation** (`TRACE_PROMPT.md`) — Create the trace doc. Output is high-coverage but marker-heavy.
2. **Verification** (`VERIFICATION_PROMPT.md`) — Resolve markers where evidence is strong. Output is leaner doc with fewer markers.
3. **Deep Investigation** (this prompt) — Resolve factual Open Questions. Output is a doc where Section 11 contains only true decision questions for you.

In practice these may interleave: a deep investigation may surface new markers, prompting another verification pass. That's normal and healthy.

### When you might NOT need this pass

Some traces will reach a clean state after generation and one verification pass. If Section 11 only contains genuinely decisional questions and there are few unresolved markers, deep investigation may add little. Reserve it for traces where Section 11 has factual questions that are worth resolving.

### Iterating on closure thresholds

The "80% vs 95% confidence" framing in Rule C2 is deliberately strict. If after running this on a few traces you find the agent is being overly conservative (leaving truly resolved questions open), you can soften it. If the agent is closing questions on weak evidence, tighten it. Calibrate to actual output quality.
