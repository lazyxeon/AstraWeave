# Veilweaver Slice Runtime Architecture

## Goal

Provide a reusable runtime harness that can bootstrap the Loomspire vertical slice with deterministic ECS state, streaming metadata, and feature-gated systems (companion GOAP + boss director).

## Components

- `veilweaver_slice_runtime` crate (new)
  - Wraps `astraweave-ecs` app creation via `ecs_adapter::build_app`
  - Captures `WorldPartition` metadata (anchors, triggers, prompts)
  - Exposes helper APIs to inspect anchors per cell and refresh metadata when streaming loads new cells
- Feature flag `veilweaver_slice` (examples + core crates)
  - Propagates to `astraweave-ai` and `astraweave-director` for companion/boss logic
  - Adds optional dependency to `examples/veilweaver_demo`

## Boot Flow

1. Load or construct `WorldPartition` (greybox cells).
2. Instantiate `VeilweaverRuntime::new(config, partition)`.
3. Optionally register post-setup systems via `add_post_setup_system`.
4. Drive simulation with `run_tick()`, or integrate into existing scheduler.
5. Call `refresh_metadata()` when streaming state changes to keep gameplay logic in sync with anchors/trigger zones.

## Usage

```rust
let mut runtime = VeilweaverRuntime::new(
    VeilweaverSliceConfig {
        dt: 0.016,
        initial_cell: Some([100, 0, 0]),
        camera_start: Some([0.0, 5.0, 0.0]),
    },
    partition,
)?;
runtime.add_post_setup_system(|world| {
    // register slice-specific systems
});
for _ in 0..60 {
    runtime.run_tick();
}
```

## Next Steps

- Integrate weaving tutorial systems with runtime metadata (anchors/triggers).
- Persist slice state snapshots for determinism validation.
- Add smoke tests under `examples/veilweaver_demo` to ensure feature builds stay green.
