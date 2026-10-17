# Vendored rapier3d

This directory is crates.io `rapier3d` **0.22.0** (Dimforge, Apache-2.0,
<https://github.com/dimforge/rapier>). The workspace `[patch.crates-io]`
entry points `rapier3d` here. Do not rustfmt the tree.

## Why it is vendored

`PhysicsPipeline::step` does `ColliderSet::take_modified`, which is
`mem::take` of `modified_colliders`. Upstream then drains that `Vec` to
clear change flags and drops it. The next step starts from a
zero-capacity `Vec`. `RigidBodyColliders::update_positions` grows it
again (measured: 5 allocations, 32 → 512, on a settled 64-box stack).

`astraweave-physics` also used to call `QueryPipeline::update` again
after `pipeline.step`. `step` already calls `update_incremental`, so
that second full rebuild allocated 3 more times. That call is gone.
The assertion in `one_physics_step_allocates_nothing` stays at 0.

## The patch

Only `src/pipeline/physics_pipeline.rs`:

- `clear_modified_colliders` drains the local `Vec` and does not write
  it back. The CCD loop calls this function again. Writing the buffer
  back inside the function lets the next call steal it.
- `clear_modified_bodies` drains, then
  `bodies.modified_bodies = std::mem::take(modified_bodies)`. That
  function runs once, before the CCD loop.
- After the CCD `while` loop:
  `colliders.modified_colliders = modified_colliders`.

`CollisionPipeline::step` still drops its local `Vec`. AstraWeave does
not call it. The step results are unchanged; only the allocation is kept.

## Re-apply on upgrade

1. Replace this directory with the new `rapier3d` sources. Keep the
   `[patch.crates-io]` path. Do not rustfmt the vendor.
2. Re-apply the three edits above in `src/pipeline/physics_pipeline.rs`.
   Do not assign `colliders.modified_colliders` inside
   `clear_modified_colliders`.
3. Leave `astraweave-physics` `step_internal` without a second
   `query_pipeline.update` after `pipeline.step`.
4. Run:

```bash
cargo test -p astraweave-physics --features alloc-counter --test zero_alloc_step -- --ignored --test-threads=1 --nocapture
```

`delta.allocs` must be 0. Replay hashes must stay the same. The buffer
reuse does not change positions.

Delete this directory and the `rapier3d` patch line once upstream
ships the buffer return. A draft for the maintainers is in
`docs/current/RAPIER_BUFFER_REUSE_UPSTREAM.md`. Do not file that draft.
