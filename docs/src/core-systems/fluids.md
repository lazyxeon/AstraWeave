<!--
  Fluids page — replaced 2026-05-15 as part of the post-trace-campaign
  reconciliation.
  Source: ARCHITECTURE_MAP.md §5.1 (Dormant-Code Inventory) and `fluids.md` trace §1, §11.
  Fluids is the single largest dormant-code reservoir in the workspace (~84.5K LoC).
  Only `examples/fluids_demo` consumes it; no production game-loop crate
  (astraweave-render, astraweave-gameplay, astraweave-physics, astraweave-scene,
   astraweave-terrain, astraweave-ecs) depends on astraweave-fluids.
  The pre-trace page framed fluids as "Production Ready (A+ Grade)"; the rewrite
  classifies it accurately as research surface awaiting production wiring.
-->

# Fluid Simulation (Research Surface)

```admonish warning title="Research surface, not a production-wired engine system"
Per `ARCHITECTURE_MAP.md` §5.1, `astraweave-fluids` is **dormant for runtime engine use**:

* **~84.5K LoC** across 35 source files plus 8 WGSL shaders — the single largest
  dormant-code reservoir in the workspace.
* The **only** workspace consumer outside the crate itself is
  `examples/fluids_demo/src/main.rs:18-21`.
* **No production game-loop crate** depends on `astraweave-fluids`. Specifically,
  `astraweave-render`, `astraweave-gameplay`, `astraweave-physics`, `astraweave-scene`,
  `astraweave-terrain`, and `astraweave-ecs` all lack the dependency.
* `astraweave-fluids/src/editor.rs` (5,823 LoC) is forward-design infrastructure —
  the visual editor (`tools/aw_editor`) does not depend on `astraweave-fluids`,
  verified 2026-05-12.

The crate exists, builds, and ships with comprehensive tests (2,404 tests / 600+ inline
plus an integration suite — benchmark-caliber by per-trace §10 grade). It is the
clearest example of the workspace's *in-design-but-tested* dormant-code category.
```

## What's in the crate

<!-- Source: fluids.md §1, §5 -->

`astraweave-fluids` carries **five parallel solver and manager surfaces** that have not
been unified:

| Surface | Role |
|---|---|
| `FluidSystem` | `lib.rs` PBD GPU pipeline |
| `UnifiedSolver` | High-level coordinator |
| `ResearchFluidSystem` | Research-grade umbrella |
| `PCISPHSystem` | Standalone PCISPH solver |
| `WaterEffectsManager` | Visual coordinator |

Plus auxiliary types: `WaterBuildingManager`, `CausticsSystem`, `WaterQualityPreset`
(Low/Medium/High/Ultra/Custom).

The largest single file is `simd_ops.rs` at 39,554 LoC; second-largest is
`editor.rs` at 5,823 LoC.

## Status grade

Per `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` (2026-05-12), the audit
rates current state at **"Grade B (Good for games, insufficient for research)"** — the
intent is research-grade simulation; the existing surface has comprehensive tests but
no production-engine integration path.

## Why this matters

The fluids crate is the canonical example used by the architecture trace campaign to
illustrate the *"wired beats tested"* axiom:

> A subsystem with passing tests and zero production callers is dormant code, not a
> feature. Tests are necessary but not sufficient. The Integration Completeness
> checklist requires a production caller, all registration surfaces touched, every
> UI/API-exposed config field read, and the architecture trace current.

— `ARCHITECTURE_MAP.md` §4.4

Listing fluid simulation as a working AstraWeave engine feature would violate this
axiom. It is documented honestly here as a research surface awaiting a production
wiring decision (Q12 in `ARCHITECTURE_MAP.md` §14).

## Running the demo

The `fluids_demo` example exercises the crate in isolation:

```bash
cargo run -p fluids_demo --release
```

This is the entire production exposure of the subsystem.

## Further reading

* [`fluids.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/fluids.md) — full fluids trace.
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/ARCHITECTURE_MAP.md) §5.1 (dormant in-design inventory),
  §4.4 (wired-beats-tested axiom), §14 (Q12 open question).
* **Interactive workspace map** — the *Dormant Surface Inventory* story preset
  highlights fluids along with the other ~200K LoC of dormant-but-designed surface.
