> **Canonical team roadmap (Vote 6):** [docs/ROADMAP.md](https://github.com/lazyxeon/AstraWeave/blob/main/docs/ROADMAP.md) — Astraea owns the content; the [sitrep page](https://astraweave-sitrep-and-roadmap-7wkx.pages.grok.bot/) mirrors it. The sections below are a dated project-status snapshot from the architecture trace campaign, not the live lane table.

<!--
  Roadmap page — replaced 2026-05-15 as part of the post-trace-campaign
  reconciliation.
  Source: ARCHITECTURE_MAP.md v0.7.0 (overall synthesis), §5 (dormancy inventory),
  §14 (23 cross-cutting open questions), CLAUDE.md (project mandate and methodology).
  The pre-trace roadmap framed "Fluids System (A+)" as a completed milestone and
  marked Audio as "Stable / High API stability" — both contradicted by trace evidence.
  Replaced with an honest project-status page per the prompt's Addition 3.
-->

# Project Status

```admonish info title="What this page is"
This page reflects the engineering reality surfaced by the architecture trace
campaign as of `ARCHITECTURE_MAP.md` v0.7.0 (2026-05-13). It supersedes earlier
roadmap documents that framed designed-but-not-wired surface as shipping features.
For navigation, the [interactive workspace map](https://lazyxeon.github.io/AstraWeave/architecture/)
visualises the same information.
```

## What this project is

AstraWeave is an **active, solo-built, research-grade AI-native game engine** in
development as the runtime for the **Veilweaver** game project. It is not a
shipping engine product. Its distinguishing engineering practice is the
*architecture trace campaign* — a 13-subsystem forensic documentation effort that
produces evidence-grounded, version-controlled traces of how the engine actually
works, separate from how older documentation aspirationally describes it.

The engine is built through AI-augmented development under the
**Genesis Code Protocol (GCP)**. The trace campaign is GCP applied at the
documentation/audit meta-level — a counterweight to the AI-generated drift that
otherwise accumulates in a 850K+ LoC workspace.

## What's working today

<!-- Source: ARCHITECTURE_MAP.md §3 (Public API Surface), §8 (Data Flow Paths) -->

* **Deterministic ECS substrate** — `astraweave-ecs` archetype storage,
  generational `Entity { id, generation }`, deterministic single-threaded
  scheduler (8 stages, executed in fixed order per tick). `ParallelSchedule`
  removed 2026-04-18; parallelism lives at the subsystem level (rayon, tokio,
  GPU compute) per `docs/audits/parallel_schedule_removal_2026-04-18.md`.
* **AI loop on the default tick** — `RuleOrchestrator`. A local model proposes off that tick. The simulation validates and executes. `AIArbiter` compiles only with `llm_orchestrator` and is not on `sys_ai_planning`. Agent-capacity numbers are dated on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page.
* **Rendering** — `astraweave-render` 78K LoC + 71 WGSL shaders. Disney BRDF +
  multi-scatter PBR, 4-cascade CSM shadows, IBL cubemaps, clustered forward+,
  Lumen GI and VXGI (both implemented), TAA, SSAO/GTAO, SSGI, SSR, volumetric
  fog, god rays, atmosphere, particle system, impostor LOD3. Editor is fully
  unified onto this pipeline post-Fix-27 (April 2026).
* **Terrain** — `astraweave-terrain` with climate field, Whittaker biome lookup,
  per-biome parameter blending, regional archetype variation, 32-layer material
  pipeline driven by `MaterialLibrary` in `astraweave-render`.
* **Visual editor** — `tools/aw_editor` 224K LoC, 41 panel types, 49 panel files,
  ~9,397 test annotations. Editor depends on `astraweave-render` non-optionally
  post-Fix-27.
* **Physics** — `astraweave-physics` wrapping Rapier3D 0.22.
  `PhysicsWorld` + `CharacterController`. `Send + Sync`.
* **Foundation verification** — Miri and Kani figures are dated on the engine status page. This tree contains 69 `#[kani::proof]` attributes. That is a source count, not a fresh proof run. The older "71+" figure is not that count.

## What's in active development

<!-- Source: ARCHITECTURE_MAP.md §12 (Active Work as of 2026-05-13) -->

* **Veilweaver** — the game project this engine is built for. Vertical slice
  shipped February 2026; full game development is the next 12-18 months of
  primary effort.
* **Editor Multi-Tool Architecture Campaign (Sub-phase 3)** — Mediator Brush
  diagnostic, Round 8 closure. §7.7 wrapped-component resource identity trap
  surfaced at four layers; Real-Fix.A/B/C landed, Real-Fix.D pending. See
  [`aw_editor.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/aw_editor.md) §1.
* **Editor Behavioral Correctness Audit remediation** — 37 fixes across 47
  commits shipped; per-audit open items still pending Andrew-gate. See
  [`docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md).
* **Architecture trace campaign** — 31 traces are listed in the `CLAUDE.md` table. `CLAUDE.md` treats a behavior change as updating its trace in the same pull request.

## What's in-design (research surface, not currently runtime-wired)

<!-- Source: ARCHITECTURE_MAP.md §5.1 (In-design-but-tested) -->

Line-count inventories from the May 2026 map were not re-counted. What this pass rechecked:

* `astraweave-fluids` is not a game-loop solver. `PcisphSystem` is gone. `examples/fluids_demo` still exists.
* Advanced GOAP (`astraweave-ai/src/goap`) compiles with `planner_advanced`.
* `VectorStore::search` is a linear scan (`astraweave-embeddings/src/store.rs` calls `brute_force_search`). Crate docs that say HNSW are ahead of the code.
* `astraweave-persistence-ecs` applies a health payload when `CHealth` is already on the replay entity. The older sentence that replay never applies events is not the engine status description.

Memory, RAG, and coordination are present and are not on the default sim tick. The engine status page is the list to trust when this table and that page disagree.

## What's not shipping yet

The engine itself. There is **no v1.0 timeline**. The engine ships when Veilweaver
ships, which is a 12–18 month horizon as of this update (2026-05-15). API
stability guarantees do not exist; consumers should pin to a commit.

## Open questions (cross-cutting)

`ARCHITECTURE_MAP.md` §14 documents **23 cross-cutting open questions** surfaced by
the trace campaign — decisional items that affect more than one subsystem and
require explicit Andrew-gate decisions. Highlights:

* **Q1** — Long-term plan for legacy `astraweave-core::World` (dual-World coexistence).
* **Q2** — Two GOAP implementations: consolidation roadmap?
* **Q3** — Runtime LLM model default (currently `phi3:medium` despite Qwen3 doc-comments).
* **Q4** — Production-hardening surface bypassed by runtime `AIArbiter` path.
* **Q5** — §7.7 wrapped-component resource identity trap, preventive instrumentation.
* **Q11, Q12** — Memory pipeline and Fluids dormancy: production-wire, prune, or rebrand?
* **Q17** — Standalone server HMAC vs. XOR `sign16` mismatch.
* **Q20** — Editor god-struct refactor (`EditorApp` 123 fields).

The list is exhaustive; the architecture map is the canonical reference.

## Further reading

* [Interactive workspace map](https://lazyxeon.github.io/AstraWeave/architecture/)
  — explore the engine.
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/ARCHITECTURE_MAP.md)
  — consolidated synthesis (2,500+ lines, version-controlled).
* [Subsystem traces](https://github.com/lazyxeon/AstraWeave/tree/main/docs/architecture)
  under `docs/architecture/`. The `CLAUDE.md` table lists 31. The "13 traces" figure is the original campaign.
* [Benchmark dashboard](https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/)
  — performance measurements.
