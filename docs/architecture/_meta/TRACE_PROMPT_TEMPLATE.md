# Architecture Trace Prompt

> **Purpose:** Paste this prompt into Copilot/Claude Code/etc. with `{{SYSTEM_NAME}}` swapped to the system you want traced. The agent will read the template, the reference example, the relevant code, and produce a filled-in architecture trace doc for that system.

---

## Usage

1. Open chat in your AI coding environment with the AstraWeave repo as context.
2. Copy the prompt below.
3. Replace `{{SYSTEM_NAME}}` with the system you're tracing (e.g. "ECS", "Render Pipeline", "AI Pipeline", "Physics").
4. Optionally fill in `{{OPTIONAL_CONTEXT}}` with anything specific the agent should know (a recent campaign, an existing audit doc, a known concern). If nothing, delete that line.
5. Send. Review and refine the output.

---

## The Prompt

```
You are producing an architecture trace document for the {{SYSTEM_NAME}} system in
the AstraWeave engine. This is part of an iterative campaign to build persistent
architectural context that future agents (and Andrew, the project owner) can rely on
without re-discovering the engine from scratch every session.

{{OPTIONAL_CONTEXT}}

## Required reading before you produce anything

1. The template at `docs/architecture/_meta/ARCHITECTURE_TRACE_TEMPLATE.md`.
   This defines the structure your output must follow.

2. The reference filled-in example at `docs/architecture/terrain_materials.md`
   (if present). This shows what good looks like.

3. The top-level `CLAUDE.md` for engine-wide conventions and AstraWeave context.

4. The actual code of the {{SYSTEM_NAME}} system. Identify the primary crates and
   key files, then read them. Do not skip this step.

5. Any existing audit docs, campaign docs, or design docs that mention
   {{SYSTEM_NAME}}. Search `docs/` for relevant references.

## Methodology rules

These rules exist because they prevent specific failure modes I've seen in past
agent-produced architecture docs. Follow them.

### Rule 1: Observation over inference

When stating how something works, cite the file (and ideally line range) that
backs the claim. If you can't cite evidence for a claim, mark it as inferred:

  GOOD: "The fragment shader samples splat textures and reconstructs a 32-channel
         weight vector (`pbr_terrain.wgsl` lines 142-178)."
  BAD:  "The fragment shader samples splat textures and reconstructs weights."

### Rule 2: Mark uncertainty explicitly

If you're not sure whether a file is active, deprecated, or legacy, say so. Use
explicit markers like `[NEEDS VERIFICATION]` or `[INFERRED]` in the doc. It is
much better to surface uncertainty than to assert a wrong status confidently.

### Rule 3: Do not fabricate decisions or rationale

The Decision Log section (section 7) is one of the most dangerous sections to
fill in. Do NOT invent reasoning for choices you can't verify. If you can find
a commit message, docstring, audit doc, or campaign doc that records the
reasoning, cite it. If you can't, write `[Reasoning not recovered from
available sources]` and move on.

### Rule 4: Respect what's working

Working code is load-bearing until proven otherwise. Do not suggest refactors
in this doc. The doc's job is to describe what exists, not to improve it.
If you genuinely see something concerning, put it in section 11 (Open Questions)
as a question, not as a recommendation.

### Rule 5: Distinguish "this is wrong" from "this isn't how I would write it"

If a pattern in the code looks unusual, default to "there's a reason for this
I don't see" rather than "this should be changed." Architecture docs aren't
opinion pieces.

### Rule 6: Scale depth to system complexity

A small system (e.g. input handling, audio config) may only need sections 1-5
plus appendix A. A large system (render, AI pipeline) should fill in everything.
Don't pad small systems with sections that don't apply. The template's usage
notes describe this.

### Rule 7: Cross-references must be specific

When mentioning another system in section 4 (Cross-System Touchpoints), name
the actual interface (function, trait, event, resource) — not just the system
name. "Uses ECS" is not useful. "Reads `WorldTime` resource via
`world.get_resource::<WorldTime>()` in the main update loop" is useful.

### Rule 8: Conflict map is forensic, not editorial

If you find coexisting abstractions, legacy paths, or naming collisions
(section 6), document them factually. Do not propose deletions or migrations
inline. Andrew makes those calls; your job is to surface the situation.

## Output requirements

- Produce the doc as a single markdown file.
- Filename: `docs/architecture/{{SYSTEM_NAME_SNAKE_CASE}}.md`
  (e.g. "Render Pipeline" → `render_pipeline.md`)
- Follow the template structure exactly. Delete sections that don't apply
  rather than leaving them empty.
- Delete all `<!-- guidance -->` comment blocks from the template before
  finalizing.
- Stamp the metadata section (top of doc) with today's date and the current
  commit short hash.

## Before you finish

Run through this self-check:

1. Does every concrete claim cite a file?
2. Have I marked all uncertainty with `[NEEDS VERIFICATION]` or `[INFERRED]`?
3. Have I avoided inventing rationale for decisions I can't verify?
4. Have I avoided proposing refactors or recommendations?
5. Are cross-system touchpoints specific (named interfaces, not vague mentions)?
6. Have I scaled depth to actual system complexity?
7. Is the conflict map (if present) factual rather than editorial?

If any of these are "no," fix before producing the final output.

## One last thing

If, while reading the code, you discover something that fundamentally doesn't
match the doc structure (e.g. the system isn't really one system but three
loosely-coupled subsystems), stop and tell Andrew before producing the doc.
A wrong doc is worse than no doc. Better to flag the structural problem and
let him decide how to split the trace.

Now: read the template, read the reference example, read the {{SYSTEM_NAME}}
code, and produce the trace doc.
```

---

## Customization notes

### When to add `{{OPTIONAL_CONTEXT}}`

Most systems won't need extra context. Add it when:

- A recent campaign just modified this system (point to the campaign doc)
- An existing audit doc covers part of this system (point to it)
- The system has known quirks the agent should be primed for ("watch for the dual sparse/dense representation pattern")
- You want the agent to focus on a particular aspect ("emphasize the AI integration touchpoints — this is what I care most about")

### When the single prompt is NOT enough

Use a more tailored prompt if:

- The system is currently being actively refactored (the trace would be stale immediately)
- The system has unusual structural properties (e.g. it's split across many crates with no obvious primary location)
- You're tracing something that crosses traditional system boundaries (e.g. "the AI-render integration" rather than "the AI system")

In those cases, customize the methodology rules section or add explicit guidance about how to handle the unusual structure.

### Iterating on the prompt itself

After running this on 2-3 systems, you'll learn which rules are doing the work and which aren't. Refine the prompt based on real output quality. The first version of any prompt is rarely the final version.

### When the agent produces a bad output

The most common failure mode will be Rule 3 violations — invented rationale in the Decision Log. If you see this, give the agent specific feedback ("Decision X in section 7 claims rationale Y, but I never documented that reasoning. Either cite where it came from or mark it as `[Reasoning not recovered]`.") and ask it to revise.

The second most common failure will be Rule 1 violations — claims without file citations. Same fix: point at the specific claim and ask for evidence.

---

## Suggested file location

Save the prompt itself at `docs/architecture/_meta/TRACE_PROMPT.md` next to the template. That way both live together and are versioned with the rest of the architecture docs.