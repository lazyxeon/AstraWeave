# Claims Archaeology Framework — Documentation Truth Campaign (D-series)

A reusable GCP framework for detecting, classifying, and structurally preventing
unverified claims in prose artifacts (docs, READMEs, status tables, traces, site,
benchmark reports). Companion to the architecture trace discipline: traces audit
what the *code* does; this audits what the *prose* claims.

**Why this class exists (context for any executing agent):** Code defects are
caught by executing consumers — compiler, tests, Miri, demos. Prose has no
executing consumer, so claims authored under earlier, less rigorous epistemic
standards (notably the Oct–Nov 2025 "agent reported success, therefore success"
era) persist indefinitely. Worse, prose feeds back into agent context (CLAUDE.md,
traces, status docs), so a fabricated claim self-reinforces. This framework is
the adversarial consumer prose never had.

---

## Claim Taxonomy

Every factual assertion in scope falls into one of six classes. Sweeps are
organized per-class because each class has a distinct verification method.

| Class | Examples | Verification method |
|---|---|---|
| **C1 Performance numbers** | "12,700 agents @ 60 FPS", ns/entity figures | Reproduction command + hardware context in MASTER_BENCHMARK_REPORT |
| **C2 Status labels** | "✅ Production Ready", "Complete", "Stable" | Trace §-citation showing load-bearing consumer + passing verification gate |
| **C3 Counts** | test counts, crate counts, LoC, coverage % | Single command (`cargo metadata`, `tokei`, llvm-cov) recorded with date |
| **C4 Comparative claims** | "faster than Bevy", "exceeds Unity DOTS coverage 3-6×" | Citable external source for the competitor figure — or deletion |
| **C5 Capability claims** | "SPH/FLIP with caustics and foam", "HNSW indexing" | file:line of the implementing code, verified non-stub |
| **C6 Provenance-free superlatives** | "most validated engine in existence" | None possible — always rewrite or delete |

## Classification States

Every swept claim gets exactly one state, with evidence:

- **VERIFIED** — reproduction command or file:line evidence checked *this sweep*, at current HEAD. Date-stamp it.
- **STALE** — was true, source confirms a different current value. Correct in place.
- **FABRICATED** — no source ever existed (e.g., "estimated" competitor numbers, invented coverage percentages). Delete; never soften to qualitative.
- **UNVERIFIABLE** — might be true but no reproduction path exists. Demote to qualitative language or delete; an unverifiable number is a liability, not an asset.
- **ASPIRATIONAL-AS-PRESENT** — describes designed/dormant/future surface in present tense. Rewrite with explicit dormancy/roadmap framing per the dormancy taxonomy.

**Evidence standard (non-negotiable):** a claim is VERIFIED only by re-running
its reproduction command or re-reading its cited source *now*. A prior document
asserting the claim — including a trace — is not evidence; traces have contained
fabricated types (`ResearchFluidSystem`, fluids.md). Document-cites-document
chains must terminate in code, a command output, or an external citation.

---

## Phase Structure

### D.0 — Sweep & Inventory (read-only)

Per-claim-class grep/read sweeps across: `README.md`, `docs/**`, `CLAUDE.md`,
the GitHub Pages source, all 13 traces, MASTER_BENCHMARK_REPORT.md, and any
pitch/marketing artifacts. Deliverable: `docs/campaigns/doc-truth/D0_CLAIMS_INVENTORY.md`
— every claim, location (file:line), class, provisional state, and the evidence
needed to resolve it. Zero source or doc modifications.

**Seeded poison list** (known-bad from conversation archaeology, 2025-10 → 2026-06;
grep for these verbatim first):

- `610,000` / `103,500` / `103k` entities @ 60 FPS (superseded by 12,700-agents
  figure with hardware context; verify no survivors)
- EnTT "~30-40 ns", Bevy "~40-60 ns", Unity DOTS "~50-100 ns" per-entity figures
  (fabricated "estimates" — FABRICATED, delete)
- Unity DOTS "15-25% coverage", Unreal "20-30% coverage", "exceeds by 3-6×",
  "most validated game engine in existence", "no other engine can make this
  claim" (FABRICATED comparatives/superlatives — delete)
- Fluids "✅ Production Ready", "4,907 tests", "SPH/FLIP with caustics and foam"
  (corrected in May 2026 README pass — verify the correction landed everywhere,
  including site and any wiki/Copilot-authored pages)
- "27,000+ tests", "Editor 3,892", "6,100+ tests", "49 production crates",
  "128 workspace members" (STALE per May 2026 reconciliation — verify no survivors)
- HNSW indexing (linear scan), production LLM hardening (bypassed at runtime),
  SpatialHash broadphase (dormant; Rapier DefaultBroadPhase is real),
  `auto_save_system` / replay event application (stubs) — each is C5/C2; verify
  current docs state dormancy honestly
- "50× prompt caching", "676 agents" (early test-plan numbers; verify provenance
  or demote)
- Spatial-hash bottleneck guidance (">25k entities O(n log n)") describing a
  system not in the hot path

### D.1 — Correction Batch + Registry Migration (execution)

1. Apply corrections per D.0 states. FABRICATED → delete; STALE → correct with
   source comment; UNVERIFIABLE → qualitative; ASPIRATIONAL → dormancy framing.
2. **Create `docs/claims/CLAIMS_REGISTRY.md`** — the single owner of every
   load-bearing number. One row per claim: value, class, reproduction command,
   hardware/context, date verified, owning trace/report. All other documents
   replace inline restatements with links to the registry entry. The registry is
   the only file where a number may appear without a link.
3. Every correction carries a hidden source comment
   (`<!-- Source: CLAIMS_REGISTRY.md #perf-agents-60fps -->`) per the existing
   site-reconciliation convention.

### D.2 — Regression Guard (standing defense)

1. **Claims lint**: a CI script (or pre-commit hook) that greps tracked prose for
   (a) the poison list, (b) bare numbers matching claim patterns
   (`\d+[km]?\s*(entities|agents|tests|FPS|ns|crates|LoC)`) outside the registry
   without a registry link, (c) status emoji + "Ready/Complete" without a trace
   citation. Warn-only for two weeks, then enforce.
2. **CLAUDE.md amendment** (one Key Lesson, candidate §7.14): *"Prose is not
   evidence. Any number or status label an agent writes must link a
   CLAUDS_REGISTRY entry or carry a reproduction command; document-cites-document
   chains must terminate in code or command output. Agent success reports are
   claims to verify, not results to record."*
3. **Trace-currency hook**: registry entries owned by a trace get re-verified
   whenever that trace's pre-flight runs (piggybacks on existing
   resumption-archaeology discipline; no new ceremony).

---

## Dynamic Workflows Mapping

This campaign is a near-ideal bounded candidate:

- **Parallel fan-out**: D.0's six claim-class sweeps are independent read-only
  passes over the same corpus — run as parallel workflow branches, each emitting
  its inventory section.
- **Hard gate**: D.0 → D.1 transition is owner-gated on the consolidated
  inventory (same shape as F.0 → F.1). No correction lands from a sweep branch
  directly.
- **Bounded verification**: D.1's registry migration has a mechanical
  completion check (lint passes = migration done), which gives the workflow an
  objective terminal condition rather than a judgment call.
- **Scope wall**: prose and CI scripts only. Zero source-code edits. If a sweep
  discovers the *code* is wrong rather than the doc (claim true in doc, false in
  code), that is a finding handed to the owning subsystem campaign, not fixed here.

## Deliverables Summary

- `D0_CLAIMS_INVENTORY.md` — full classified inventory with evidence
- `D1_CORRECTION_REPORT.md` — every edit, before/after, state, source
- `docs/claims/CLAIMS_REGISTRY.md` — the standing single source of truth
- CI claims-lint script + CLAUDE.md §7.14 amendment
- Open items: any claim-true/code-false findings routed to subsystem campaigns

**Failure mode this framework exists to eliminate:** a confident document
nobody has adversarially read since the day an agent wrote it.