# Architecture Trace: [System Name]

<!--
TEMPLATE USAGE NOTES
====================
This template produces a persistent, agent-readable architecture document for one engine
system. The goal is to give any agent (or future you) the context needed to work on this
system without re-discovering it from scratch every session.

How to use:
1. Copy this file to docs/architecture/<system_name>.md
2. Fill in each section. Delete sections that don't apply.
3. Scale depth to system complexity: small systems may only need sections 1-4.
   Complex systems should fill in everything.
4. Reference this file from CLAUDE.md so agents are directed here before
   working on the system.
5. Update this doc as part of any campaign deliverable that touches this system.
   Stale docs are worse than no docs.

Length guidance:
- Small system (audio, input):           1-3 pages
- Medium system (physics, AI):           5-10 pages
- Large system (render, editor):         10-25 pages

Delete all <!-- guidance --> blocks before committing the filled-in version.
-->

## Metadata

| Field | Value |
|---|---|
| **System name** | [e.g. Terrain Material System] |
| **Primary crates** | [e.g. `astraweave-terrain`, `astraweave-render`] |
| **Document version** | 1.0 |
| **Last verified against commit** | [git short hash] |
| **Last verified date** | [YYYY-MM-DD] |
| **Status** | [Active / Transitional / Deprecated / In-Design] |
| **Owner notes** | [Optional: who designed this, key contributors] |

---

## 1. Executive Summary

<!--
guidance: 3-6 sentences. An agent should be able to read JUST this section and know:
- What this system does at a high level
- Whether they need to read further for their task
- Where in the codebase the system primarily lives
Write it for the busy agent, not the curious historian.
-->

**What this system does:**
[One sentence describing the system's purpose at the engine level.]

**Why it exists:**
[One sentence on what problem it solves or what capability it provides.]

**Where it primarily lives:**
[Crates and key directories. Bullet list acceptable here.]

**Status note:**
[Anything an agent must know up front — e.g. "Mid-refactor as of <date>; the splat
path is authoritative, the biome path is being migrated."]

---

## 2. Authoritative Pipeline

<!--
guidance: This is the box-and-arrow trace of how data actually flows through the
system today. Not historical, not aspirational — current. Use ASCII art in a code
block. Annotate each stage with file/function references.

This section is the heart of the document. Spend time on it.
-->

```text
[Upstream input]
    │
    │ <function or method name>
    ▼
[Stage 1: <what happens>]
    file: path/to/file.rs
    role: <one-line description>
    key data: <what data this stage produces>
    │
    │ <transformation>
    ▼
[Stage 2: <what happens>]
    file: path/to/file.rs
    role: <one-line description>
    key data: <what data this stage produces>
    │
    ▼
[Final output]
```

### Stage-by-stage detail

<!--
guidance: For each stage in the diagram above, give:
- What it does in 2-4 sentences
- Inputs and outputs
- Files involved
- Any non-obvious behavior

Scale depth to complexity. A simple stage might only need 2 sentences.
A complex stage might need a full subsection.
-->

#### Stage 1: [Name]
**File(s):** `path/to/file.rs`
**Role:** [What this stage does in the pipeline]
**Inputs:** [What it consumes]
**Outputs:** [What it produces]
**Notes:** [Any non-obvious behavior, design rationale, or gotchas]

#### Stage 2: [Name]
[...]

---

## 3. Semantic Vocabulary

<!--
guidance: This section pins down the terms used in and around this system.
The goal is to prevent ambiguity — e.g. "biome" vs "material" vs "layer" should each
have one definition that everyone (including agents) uses consistently.

If your system has 3+ overlapping terms, this section is high-leverage.
If your system uses only obvious terms, this section can be brief.
-->

| Term | Definition | Used in |
|---|---|---|
| [Term A] | [Precise definition] | [Files/contexts where this term applies] |
| [Term B] | [Precise definition] | [Files/contexts where this term applies] |

### Terms to NOT confuse

<!--
guidance: If certain terms are commonly conflated, call out the distinction explicitly.
e.g. "Biome semantics describes ecological regions. Material semantics describes
render surface layers. These are related but not the same thing."
-->

- **[Term X] vs [Term Y]:** [Why these are different and where confusion happens]

---

## 4. Cross-System Touchpoints

<!--
guidance: What other systems feed into this one, and what other systems consume
this one? This is the "map" agents need to understand how a change here ripples
elsewhere.

Be specific: don't just list system names, list the actual interfaces.
-->

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| [System name] | [Function/trait/event] | [What flows through] | [Constraints, frequency, ownership] |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| [System name] | [Function/trait/event] | [What flows through] | [Constraints, frequency, ownership] |

### Bidirectional / Coupled

<!--
guidance: Systems that have two-way interaction with this one. List the nature
of the coupling explicitly.
-->

- [Coupled system]: [Nature of coupling]

---

## 5. Active File Map

<!--
guidance: Authoritative list of which files implement which roles in this system.
Marks the canonical "if you're working on X, edit Y" mapping.

Use the table below. Mark files clearly as Active, Transitional, Deprecated, or Legacy.
-->

| File | Role | Status | Notes |
|---|---|---|---|
| `path/to/active.rs` | [What it does] | Active | [Anything load-bearing] |
| `path/to/transitional.rs` | [What it does] | Transitional | [Migration target/timeline] |
| `path/to/legacy.rs` | [What it does] | Deprecated | [Why kept, when removable] |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care
- **Transitional**: Active but planned for change or replacement
- **Deprecated**: Kept for compatibility, no new uses, removable when dependents migrate
- **Legacy**: Dead code or near-dead code, candidate for deletion

---

## 6. Conflict Map / Residue

<!--
guidance: This is where you document the "old worldview that hasn't fully died yet"
problem. Be honest about it. Agents reading this should know exactly what's
load-bearing vs what's noise.

Skip this section if your system has no historical residue. Most non-trivial
systems have some.
-->

### Coexisting abstractions

<!--
guidance: If multiple ways of representing the same thing coexist in the codebase,
list them here with disposition.
-->

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| [Old representation] | [files] | Deprecated | [Plan: migrate by date X / delete after dependent Y] |
| [Current representation] | [files] | Active | Canonical |

### Naming collisions

<!--
guidance: Words that mean different things in different parts of the codebase.
Common offenders: "material", "layer", "chunk", "node".
-->

- **[Term]**: In `crate_a`, means [definition A]. In `crate_b`, means [definition B]. Future direction: [unify / keep separate / rename].

### Known cognitive traps

<!--
guidance: Patterns or files that have historically caused confusion. Forewarn agents.
-->

- **Trap**: [Description]
- **Why it's confusing**: [Reason]
- **What's actually true**: [Clarification]

---

## 7. Decision Log

<!--
guidance: Key architectural choices with their reasoning. This is what prevents agents
from "improving" intentional decisions.

Format each decision as a mini-ADR (Architecture Decision Record).
-->

### Decision: [Short name]
- **Date:** [YYYY-MM-DD]
- **Status:** Accepted / Superseded by [link] / Under review
- **Context:** [What forces or constraints led to this decision]
- **Decision:** [What was chosen]
- **Alternatives considered:** [What else was on the table and why rejected]
- **Consequences:** [What this commits us to, what trade-offs we accepted]

### Decision: [Short name]
[...]

---

## 8. Known Invariants

<!--
guidance: Claims about the system that should always be true. The most valuable
ones are machine-checkable.

These serve two purposes:
1. Tell agents what they must not break
2. Optionally back them with automated tests/lints

If you write down an invariant, you're committing to it. Don't write invariants
you're not willing to enforce.
-->

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | [Claim that must hold] | Yes/No | [test/lint/doc-only] |
| 2 | [Claim that must hold] | Yes/No | [test/lint/doc-only] |

---

## 9. Performance & Resource Profile

<!--
guidance: Optional section. Include for systems where performance characteristics
matter (render, physics, AI). Skip for systems where they don't (input, audio config).

What an agent needs to know to avoid making this system slow.
-->

### Hot paths
- [Path X]: runs at [frequency]. Budget: [time/memory]. Sensitivity: [what makes it slow].

### Cold paths
- [Path Y]: runs at [frequency]. Looser budget. Can afford [expensive operations].

### Resource ownership
- [Resource type]: owned by [system]. Lifetime: [scope]. Access pattern: [read/write].

---

## 10. Testing & Validation

<!--
guidance: Where do tests for this system live? What kind of testing exists?
What's the bar for changes?
-->

- **Unit tests:** [Location, coverage notes]
- **Integration tests:** [Location, coverage notes]
- **Mutation testing:** [Status — covered in Wave X campaign or not yet]
- **Miri validation:** [If applicable for unsafe code]
- **Benchmarks:** [Location, key metrics tracked]
- **Manual validation:** [Visual/playthrough checks if applicable]

---

## 11. Open Questions / Parked Decisions

<!--
guidance: Things you know aren't resolved yet. Better to write them down than to
have them rediscovered painfully later.
-->

- **[Question]:** [Context, why it's parked, when it might need resolution]
- **[Question]:** [Context, why it's parked, when it might need resolution]

---

## 12. Maintenance Notes

<!--
guidance: How to keep this doc current. Brief.
-->

**Update this doc when:**
- A campaign touches any of the Active files in Section 5
- A decision in Section 7 is superseded
- An invariant in Section 8 is broken or relaxed
- A coexisting abstraction in Section 6 is migrated or removed

**Verification process:**
- [How to confirm doc matches code — e.g. "run trace script", "spot-check pipeline against current code"]
- Stamp the new commit hash and date in Section 0 metadata after verification.

---

## Appendix A: Quick reference for agents

<!--
guidance: Optional but highly recommended for complex systems.
A scannable summary an agent can refer to mid-task without re-reading the whole doc.
-->

**If you're working on this system, remember:**
1. [Most important thing — usually a key invariant or trap]
2. [Second most important]
3. [Third most important]

**Files you'll most likely touch:**
- `path/to/most/edited.rs`
- `path/to/next/most/edited.rs`

**Files you should NOT touch without strong reason:**
- `path/to/load/bearing.rs` — [why]
- `path/to/legacy.rs` — [why; e.g. "deprecated, will be deleted; don't add features here"]

**Common mistakes when changing this system:**
- [Mistake X]: [Why it's wrong, what to do instead]
- [Mistake Y]: [Why it's wrong, what to do instead]

---

## Appendix B: Historical context

<!--
guidance: Optional. For systems with significant history that informs current design
but isn't load-bearing for current work.

Keep this brief. The goal is "useful background", not "complete history".
-->

[1-3 paragraphs on how this system got to where it is, if relevant.]

---

## Reference: Filled-in example

The terrain material system trace at `docs/architecture/terrain_materials.md` is the
canonical example of this template filled in for a non-trivial system. Reference it
when filling in sections you're unsure about.
