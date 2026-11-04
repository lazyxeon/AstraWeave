# Fluids API Reference

> **Crate**: `astraweave-fluids`
> **Status**: standalone subsystem — engine integration in progress (Fluids-Integration campaign, Path B)
>
> **⚠ This page was rewritten in Fluids-Integration F.1 (2026-06-11).** The
> previous revision (from the 2025-09-08 aspirational wiki sweep, commit
> `28bc94f21`) documented ≥13 types that do not exist in the crate —
> `FluidWorld`, `FluidConfig`, `PcisphConfig`, `CausticRenderer`,
> `GodRayRenderer`, `WaterTerrainIntegration`, `FloodConfig`,
> `GpuVolumeGrid`, `WaterEditor`, `WaterLod`, `save_water_state`, a
> `unified_solver::ParticleType`, and an `editor::validation` module — plus
> method signatures that never matched the source. Per the engine's
> documentation-hazard policy, treat any older copy of this page as
> historical fiction. The authoritative API references are the crate's
> `lib.rs` re-exports and `docs/architecture/fluids.md` (v1.3+).

## What actually exists (F.1 state)

### Features

| Feature | Default | Gates |
|---|---|---|
| `parallel` | off | Declared. The `Cargo.toml` comment says the W.1 removal left it gating nothing. `simd_ops` is gone. |

There is no `experimental` feature. `PcisphSystem` and the other modules named in that old feature (`multi_phase`, `warm_start`, `particle_shifting`, `turbulence`, `viscosity_gpu`) are not in `src/`.

serde support is unconditional (the former decorative `serde` feature was removed in F.1).

### The production-exercised surface (what `examples/fluids_demo` uses)

```rust,ignore
use astraweave_fluids::{FluidSystem, FluidRenderer, Particle, DynamicObject};

// Device requirement: TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
// (the internal SDF pipeline uses Rgba32Float storage textures).
let mut system = FluidSystem::new(&device, 20_000);
system.smoothing_radius = 0.5;
system.target_density = 1.0;
// `viscosity` scales vorticity confinement (the XSPH blend is hardcoded
// in fluid.wgsl); a dead `pressure_multiplier` uniform was removed in F.1.
system.viscosity = 0.01;

// Per frame — the encoder MUST be submitted before the next step() call
// (documented contract; the density-error readback relies on it).
system.step(&device, &mut encoder, &queue, dt);

// Optional per-pass GPU timings (requires Features::TIMESTAMP_QUERY):
system.enable_gpu_timing(&device, &queue);
// ... after submit:
let timings = system.read_gpu_timings(&device); // Option<Vec<(&str, f32)>>
```

Spawn/despawn: `spawn_particles`, `reset_particles`, `despawn_region` (real
since F.1: despawned particles are flag-skipped by every kernel and parked
below the world; region membership uses the CPU position cache, exact only
at spawn/reset time).

### Other names checked in this pass

- `FluidRenderer` is re-exported from `renderer`.
- `Particle` and `DynamicObject` are public structs in `lib.rs`.
- `WaterEffectsManager` is in `water_effects.rs`. `FluidEditorConfig` is in `editor.rs`.
- `WaterVolumeGrid`, `WaterBuildingManager`, and `PcisphSystem` are not in `src/`. Do not copy them from older pages.
- The game-loop water field is `astraweave-localsim`. The view-side crate is `astraweave-water`. See the engine status page.

### Determinism

GPU particle state is non-deterministic by construction and excluded from
`world_hash`/replay/replication (campaign gate Q1 carve-out). Use the
deterministic CPU layers for gameplay-relevant water state. See
`docs/architecture/fluids.md` §0.
