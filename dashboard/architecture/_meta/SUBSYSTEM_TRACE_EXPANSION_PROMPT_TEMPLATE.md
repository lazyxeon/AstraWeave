# Architecture Trace Subsystem Expansion Prompt

> **Purpose:** Paste this prompt into Claude Code (or another agentic coding tool with direct repo access) to add a deep subsystem trace to an existing architecture trace document. Used when a parent-system trace has flagged subsystems as needing follow-up. The prompt expands the existing file additively — no new files, no deletions, no restructuring of Sections 1-12.

---

## When to use this prompt

Use this prompt when:

- A parent-system trace exists and has been generated, verified, and (optionally) deep-investigated
- The parent trace covers a system large enough that one or more subsystems were flagged for follow-up work (typically called out in the parent trace's metadata, executive summary, or Section 11 Open Questions)
- You want subsystem-level detail without losing the overview-level coherence of the parent doc

**Do NOT use this prompt to create a fresh trace.** Use `TRACE_PROMPT.md` for that. This prompt is purely additive expansion of an existing doc.

---

## Usage

1. Open Claude Code with the AstraWeave repo as the working directory.
2. Copy the prompt below.
3. Replace `{{TRACE_DOC_PATH}}` with the existing parent trace doc (e.g. `docs/architecture/ai_pipeline.md`).
4. Replace `{{SUBSYSTEM_NAME}}` with the specific subsystem being expanded (e.g. "Memory", "Director", "Advanced GOAP").
5. Optionally fill `{{OPTIONAL_CONTEXT}}` with focus areas or known sensitivities for this subsystem.
6. Send. Review the additions carefully before accepting.

Run once per subsystem. Tightly-coupled subsystem pairs may be done together if it produces clearer cross-references (e.g. Memory + RAG, if they share infrastructure), but the default is one subsystem per run.

---

## The Prompt

```
You are expanding an existing architecture trace document for the AstraWeave engine
to include a deep subsystem trace. The parent doc lives at {{TRACE_DOC_PATH}}.

The subsystem to trace is: {{SUBSYSTEM_NAME}}

Your job:

1. Read the parent trace doc end-to-end so you understand the existing
   architecture and vocabulary.
2. Investigate the {{SUBSYSTEM_NAME}} subsystem in code, deeply.
3. Append a new subsystem trace as a subsection of Section 13 of the parent doc.
4. Make minimal, surgical updates to the existing parent doc only where they
   genuinely improve cross-referencing or close factually-resolved Open Questions
   in Section 11.

This is an ADDITIVE pass. You do not delete content. You do not restructure
Sections 1-12. You do not create new files.

{{OPTIONAL_CONTEXT}}

## Required reading before you start

1. The parent trace doc at {{TRACE_DOC_PATH}} — read it end to end first.
2. The architecture trace template at
   `docs/architecture/_meta/ARCHITECTURE_TRACE_TEMPLATE.md`.
3. The deep investigation prompt at
   `docs/architecture/_meta/DEEP_INVESTIGATION_PROMPT.md` — its closure rules
   apply when you encounter Open Questions during this pass.
4. `CLAUDE.md` at repo root for engine-wide context.
5. The actual code of the {{SUBSYSTEM_NAME}} subsystem. Identify which files
   in the parent doc's Section 5 file map belong to this subsystem, plus any
   subsystem-specific files not yet enumerated there.

## Where the new content goes

### Section 13: Subsystem Traces

If Section 13 does not yet exist in the parent doc, create it immediately after
Section 12 (Maintenance Notes) and before Appendix A. The section heading is:

  ## 13. Subsystem Traces

  This section contains deep traces of subsystems within the parent system.
  Each subsection covers one subsystem and follows a compact mirror of the
  main template structure, scoped to that subsystem's concerns.

If Section 13 already exists, append a new subsection within it. Number it
sequentially (13.1, 13.2, 13.3, ...) based on existing subsections.

### Subsystem trace subsection structure

Each subsystem trace is structured as follows (replace `13.N` with the actual
subsection number):

```

### 13.N Subsystem Trace — {{SUBSYSTEM_NAME}}

**Last verified against commit:** [short hash]
**Last verified date:** [YYYY-MM-DD]
**Status:** [Active / Transitional / In-Design / Legacy]

#### 13.N.1 Role within the parent system

[2-4 sentences: what this subsystem does, why it exists, how it fits into
the parent system's overall flow]

#### 13.N.2 Authoritative pipeline

[Box-and-arrow trace scoped to this subsystem only. May reference the parent
system's pipeline diagram by section number where appropriate, but the
subsystem's own data flow should be explicit here.]

#### 13.N.3 Vocabulary (subsystem-specific)

[Terms specific to this subsystem that didn't earn a slot in the parent doc's
Section 3. If the subsystem inherits all its vocabulary from the parent, write
"All vocabulary inherited from parent Section 3."]

#### 13.N.4 Files involved

[Table of files specific to this subsystem with status. May overlap with the
parent's Section 5 file map; reference parent rows where appropriate rather
than duplicating.]

#### 13.N.5 Cross-subsystem touchpoints

[How this subsystem interacts with sibling subsystems WITHIN the parent system.
For touchpoints outside the parent system, defer to parent Section 4.]

#### 13.N.6 Invariants (subsystem-specific)

[Invariants that apply specifically to this subsystem, beyond what the parent
Section 8 already documents. If none, write "No subsystem-specific invariants
beyond parent Section 8."]

#### 13.N.7 Open questions (subsystem-specific)

[Open questions specific to this subsystem. Use the same factual/decisional
classification from the deep investigation prompt. If a parent-level Open
Question in Section 11 was resolved by this investigation, note that here
AND update Section 11 per the rules below.]

```

## Investigation methodology

### Phase 1: Scope the subsystem within the parent system

Before investigating code, build your mental map:

1. Identify which files in the parent doc's Section 5 belong to this subsystem.
2. Identify any subsystem-specific files NOT in the parent file map (these
   become new entries in your 13.N.4 table).
3. Identify which parent Open Questions in Section 11 are specifically about
   this subsystem (these may become resolvable during your investigation).
4. Identify which parent Decisions in Section 7 are specifically about this
   subsystem (these provide context for your investigation but are NOT
   restated in the subsystem trace).

### Phase 2: Trace the subsystem pipeline

Apply the same rigor as the original trace generation:

1. Identify entry points into this subsystem.
2. Follow data flows through each stage.
3. Identify branches, fallbacks, conditional paths.
4. Note where data leaves this subsystem (into sibling subsystems or out of
   the parent system entirely).
5. Cite files with line ranges throughout.

### Phase 3: Identify subsystem-specific concerns

For each of the structural sections (13.N.3 through 13.N.7), gather what is
specifically true of this subsystem that isn't already captured at the parent
level. Avoid duplicating parent-level content; reference it instead.

### Phase 4: Surface emergent findings

During subsystem investigation you may discover:

- Files that should be added to the parent's Section 5 file map
- Vocabulary that should be promoted to the parent's Section 3
- Cross-system touchpoints that should be added to the parent's Section 4
- New invariants that apply at parent level, not just subsystem level
- Parent-level Open Questions in Section 11 that this investigation resolves
- Conflict map entries (parent Section 6) that should be expanded

For each finding, apply the update rules below.

## Update rules

### Where you may ADD content

1. **A new Section 13 (if it doesn't yet exist).** Created with the standard
   heading and one subsection for this subsystem.
2. **A new subsection within Section 13.** Numbered sequentially.
3. **A new row in the parent's Section 5 file map** if subsystem investigation
   revealed a file not previously enumerated. Add it in the appropriate crate
   group with proper status.
4. **A new entry in the parent's Section 3 vocabulary** if subsystem
   investigation revealed a term whose definition belongs at parent scope.
5. **A new row in the parent's Section 4 cross-system touchpoints** if subsystem
   investigation revealed an interface to an external system not previously
   documented.
6. **A new invariant in the parent's Section 8** if subsystem investigation
   revealed an invariant that holds at parent scope, not just subsystem scope.
7. **A new Open Question in the parent's Section 11** if subsystem
   investigation surfaced a parent-scope question that wasn't previously
   tracked.

### Where you may MODIFY existing content (surgically)

1. **Closing a parent-level Open Question** in Section 11 that the subsystem
   investigation conclusively resolves. Apply the closure rules from
   `DEEP_INVESTIGATION_PROMPT.md` — the question must be purely factual,
   evidence must be conclusive, etc. When closing, move the resolution into the
   appropriate parent section AND remove the question from Section 11.
2. **Enriching a parent-level Open Question** in Section 11 with new factual
   context if the question is decisional and the subsystem investigation
   added useful evidence but did not resolve it.
3. **Adding a cross-reference** in a parent section to the new subsystem trace
   (e.g. inside Section 2 noting "See Section 13.N for detail on the Memory
   subsystem pipeline"). Keep these references minimal and only where they
   genuinely aid navigation.
4. **Updating the metadata block** at the top of the parent doc:
   - Bump the document version (e.g. 1.2 → 1.3)
   - Update "Last verified against commit" to current commit short hash
   - Update "Last verified date" to today's date
   - Add an Owner Notes line indicating subsystem trace was added
     (e.g. "Subsystem trace for {{SUBSYSTEM_NAME}} added 2026-MM-DD.")

### Where you may NOT modify existing content

1. Do NOT restructure Sections 1-12. Their headings, ordering, and prose stay.
2. Do NOT rewrite the parent's Executive Summary unless a factual claim in it
   is now wrong (in which case correct only the wrong claim, not the framing).
3. Do NOT collapse parent sections to "make room" for subsystem detail.
4. Do NOT remove parent file map entries.
5. Do NOT remove parent Decision Log entries.
6. Do NOT delete the existing Appendix A or Appendix B.
7. Do NOT create separate files for subsystem traces. The expansion happens
   in the parent file.

## Anti-overreach rules

The methodology rules from earlier prompts still apply:

- **Rule 1 (Observation over inference)**: Every claim cites evidence.
- **Rule 2 (Mark uncertainty)**: Where evidence is thin, mark it.
- **Rule 3 (Don't fabricate rationale)**: Unrecovered reasoning stays unrecovered.
- **Rule 4 (Respect what's working)**: This is documentation, not refactoring.
- **Rule 5 (Distinguish "wrong" from "not how I'd write it")**: Default to
  "there's a reason I don't see."
- **Rule 6 (Conflict map is forensic, not editorial)**: Surface situations,
  don't propose disposals.
- **Rule 7 (No code changes)**: This pass touches only the trace doc.

Plus subsystem-expansion-specific rules:

- **Rule SE1 (Reference, don't duplicate)**: If parent doc already covers
  something, reference the parent section rather than restating it.
- **Rule SE2 (Subsystem scope discipline)**: If you find yourself describing
  things outside the subsystem's actual boundary, you've drifted. Refocus.
- **Rule SE3 (Promotion threshold)**: Only promote subsystem-level findings
  to parent-level sections when they genuinely apply at parent scope. When in
  doubt, keep findings inside the subsystem trace.
- **Rule SE4 (Subsystem trace is compact)**: A subsystem trace is leaner than
  a full parent trace. The seven subsections (13.N.1-7) are usually enough.
  Don't replicate the full 12-section template inside Section 13.

## Reporting requirements

After completing the subsystem expansion, produce a report with the following:

### Summary
- Subsystem traced
- New subsection added (e.g. "Section 13.1 Subsystem Trace — Memory")
- Number of parent sections updated (with which sections)
- Number of parent Open Questions closed
- Number of parent Open Questions enriched
- Number of new parent-level findings added
- Number of urgent findings flagged

### Subsystem trace contents
- Brief description of the pipeline you traced
- Key files cited
- Subsystem-specific findings worth highlighting

### Parent updates
For each parent-section edit:
- Which section was updated
- What was added/modified
- Why this finding warranted parent-level placement rather than staying in
  the subsystem trace

### Parent Open Questions resolved
For each closed Open Question:
- Original question text
- Evidence found (with citations)
- Where the resolution now lives

### Parent Open Questions enriched (not closed)
For each enriched question:
- What context was added
- Why it remains open

### New parent-level Open Questions
- Questions surfaced that warrant parent-scope tracking

### Urgent findings
- Anything beyond documentation that warrants attention

### Methodology notes
- Anything about the investigation process worth recording

## Self-check before finishing

1. Did I add Section 13 (or a new subsection within it) without modifying
   Sections 1-12 structurally?
2. Did I keep the subsystem trace compact (seven subsections, no full template
   replication)?
3. Did every claim cite a file with line ranges?
4. Did I mark uncertainty where evidence is thin?
5. Did I reference parent sections rather than duplicate their content?
6. For any parent Section 11 question I closed: did I apply all four closure
   conditions from the deep investigation prompt?
7. For any parent-level promotion of findings: is the promotion genuinely
   parent-scope, not just subsystem-scope?
8. Did I avoid making editorial recommendations or proposing refactors?
9. Did I avoid creating any new files?
10. Is the metadata block updated with new version, commit hash, and date?
11. Does my report clearly distinguish subsystem-trace additions from parent-
    section updates?

If any answer is "no," fix before submitting.

Now: read the template, read the deep investigation prompt, read the parent
trace doc, read CLAUDE.md, then begin investigation of {{SUBSYSTEM_NAME}}.
```

---

## Customization notes

### When subsystems are tightly coupled

If two subsystems share infrastructure (e.g. Memory and RAG share embedding storage), you may want to expand them together to capture cross-references cleanly. In that case:

- Use both subsystem names in `{{SUBSYSTEM_NAME}}` (e.g. "Memory + RAG")
- Each gets its own subsection in Section 13 (e.g. 13.N for Memory, 13.N+1 for RAG)
- Their respective 13.N.5 (Cross-subsystem touchpoints) sections cross-reference each other explicitly

For most subsystems, run one at a time.

### When the parent trace itself needs significant updates

If subsystem investigation reveals that the parent trace has substantial factual errors (not just unresolved questions), stop and report rather than rewriting. The fix is a separate verification pass on the parent doc, not an inline correction during subsystem expansion. Verification has its own discipline rules that this prompt is not designed to enforce.

### When to escalate

Some subsystems are large enough to warrant their own top-level trace doc rather than a subsection of a parent. If during investigation you discover the subsystem is really 30K+ LoC across 5+ files with its own architectural complexity, stop and recommend promotion to a standalone trace doc (using `TRACE_PROMPT.md`) rather than forcing it into Section 13. The Subsystem Trace structure is designed for focused detail; if it can't fit in seven compact subsections, it probably wants its own document.

### Iterating on multiple subsystems

For a parent system with many subsystems flagged (e.g. AI Pipeline's 8 follow-ups), realistic cadence is one per session, with the parent doc treated as a living artifact that gets richer each pass. After every subsystem expansion:

- The Section 13 grows by one subsection
- Parent Section 11 (Open Questions) tends to shrink as factual ones resolve
- Parent metadata version increments
- The campaign log accumulates one more report

After all flagged subsystems are expanded, the parent doc should be a complete reference: overview-level Sections 1-12, plus deep-detail Section 13 for each subsystem, plus the standard appendices. That's the final state for a complex parent system.
