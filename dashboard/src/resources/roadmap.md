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
* **AI-first runtime loop** — `WorldSnapshot` → `AIArbiter` → `Orchestrator.plan()`
  → `PlanIntent` → `tool_sandbox` → engine-side `validate_and_execute`. 12,700+
  agents at 60 FPS validated. Canonical GOAP + Behavior Trees + LLM orchestrator
  hybrid (`astraweave-ai`, `astraweave-behavior`, `astraweave-llm`).
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
* **Foundation verification** — Miri (1,059 tests across `ecs`, `math`, `core`,
  `sdk` with zero undefined behavior) + Kani proofs (71+ harnesses across
  safety-critical crates).

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
* **Architecture trace campaign** — 13 of N subsystems traced. Treated as part
  of the production contract by `CLAUDE.md`. Adding more traces as subsystems
  cross the non-trivial-surface threshold.

## What's in-design (research surface, not currently runtime-wired)

<!-- Source: ARCHITECTURE_MAP.md §5.1 (In-design-but-tested) -->

Per `ARCHITECTURE_MAP.md` §5.1, the workspace carries **~200K LoC** of
*in-design-but-tested* surface — code that passes its own tests but has zero
production callers. This is the signature of breadth-first AI-augmented
development, not a flaw. Each item is documented honestly as awaiting a wiring
decision:

| Subsystem | LoC | Status note |
|---|---|---|
| `astraweave-fluids` | ~84.5K | Only `examples/fluids_demo` consumes it. Five parallel solver surfaces, no game-loop deps. Q12 in §14. |
| `astraweave-memory` | ~11K | Zero in-engine production consumers. Q11 in §14. |
| `astraweave-coordination` | ~5.3K | Zero workspace consumers; 3 commented-out `pub mod` declarations. |
| Advanced GOAP (`astraweave-ai/src/goap/`) | ~16.7K | Feature `planner_advanced`; parallel to canonical GOAP. Q2 in §14. |
| LLM Production Hardening | ~15K | Rate limiting / circuit breakers / A/B routing / retry / telemetry / ToolGuard / 4-tier fallback — runtime path bypasses entirely. Q4 in §14. |
| RAG composite (`astraweave-rag` + `embeddings` + `context`) | ~12.3K | Held as field by 5 dormant consumer crates. HNSW advertised; actual is linear scan. |
| Dialogue LLM layer | ~2.9K | 60% of `astraweave-dialogue`. Basic `DialogueGraph` is production-wired; LLM layer is not. |
| `astraweave-net-ecs` ECS Plugin layer | medium | Working tests, no production consumer; declared-but-unused dep in `astraweave-stress-test`. |
| `astraweave-persistence-ecs` Plugin layer | medium | `auto_save_system` body is comment-only TODO; `replay_system` advances tick but never applies events. Q18 in §14. |

Plus dormant scaffolding (TODO-only bodies), orphan source files, declared-but-unused
Cargo deps, dormant feature flags, and the aspirational documentation tree from
commit `28bc94f21`. Full taxonomy in `ARCHITECTURE_MAP.md` §5.

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
* [13 per-subsystem traces](https://github.com/lazyxeon/AstraWeave/tree/main/docs/architecture)
  under `docs/architecture/`.
* [Benchmark dashboard](https://lazyxeon.github.io/AstraWeave/dashboard/benchmark_dashboard/)
  — performance measurements.
