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

`astraweave-fluids` is in the workspace. The [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page calls it the deprecated accent remnant, not a game-loop solver. `examples/fluids_demo` is a consumer. Line-count and test-count figures from the May 2026 trace were not re-counted here, and they predate the W.1 removal recorded in the crate's `Cargo.toml`.

## What's in the crate

<!-- Source: fluids.md §1, §5 -->

`FluidSystem` is a public type in `astraweave-fluids/src/lib.rs` (`FluidSystem::new`).
`WaterEffectsManager` is in `water_effects.rs`. `examples/fluids_demo` registers
`LaboratoryScenario` and `OceanScenario` only.

`PCISPHSystem` / `PcisphSystem` is not in the crate. The `Cargo.toml` comment records
that the `experimental` modules, including that solver, were removed in W.1
(2026-06-20), and that `simd_ops` was removed with them. There is no `experimental`
feature key left. `parallel` still exists and the same comment says it gates nothing.

Line counts and test totals on older copies of this page were not re-counted.
The [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md)
page calls `astraweave-fluids` the deprecated accent remnant, not a game-loop solver.
The greybox local field is `astraweave-localsim`. The view-side water crate is
`astraweave-water`.

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

* [`fluids.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/fluids.md) — full fluids trace.
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/ARCHITECTURE_MAP.md) §5.1 (dormant in-design inventory),
  §4.4 (wired-beats-tested axiom), §14 (Q12 open question).
* **Interactive workspace map** — the *Dormant Surface Inventory* story preset
  highlights fluids along with other surface that is present and not on the sim tick.
