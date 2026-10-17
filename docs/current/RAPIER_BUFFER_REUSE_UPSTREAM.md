# Draft: reuse the modified-collider buffer in Rapier

**Status: draft for the Dimforge maintainers. Do not file this on
GitHub, Discord, or crates.io.** AstraWeave vendors `rapier3d` 0.22.0
until an upstream release contains the change. The local copy and the
re-apply steps are `third_party/rapier3d/VENDORED.md`.

## Summary

`PhysicsPipeline::step` should put the modified-collider `Vec` back
into `ColliderSet` after the CCD loop, with its capacity intact.
`clear_modified_bodies` can do the same immediately, because it runs
once. A settled step then allocates nothing for those buffers.

## What we measured

`ColliderSet::take_modified` is `mem::take(&mut self.modified_colliders)`.
`PhysicsPipeline::step` drains that `Vec` while clearing
`ColliderChanges`, then drops it. The next step's
`RigidBodyColliders::update_positions` pushes into a new `Vec` and
grows it.

On a 64-box stack, after 30 warmup steps, one more `PhysicsWorld::step`
allocated 5 times from that growth (capacity 32 → 512). A caller-side
`QueryPipeline::update` after `step` added 3 more, because `step`
already runs `QueryPipeline::update_incremental`. Removing the extra
update is our call site, not a Rapier change.

The test is `astraweave-physics` `one_physics_step_allocates_nothing`
(`--features alloc-counter -- --ignored`). The assertion is
`delta.allocs == 0`.

## Change

In `PhysicsPipeline`:

`clear_modified_colliders` only drains. It must not assign
`colliders.modified_colliders`.

```rust
fn clear_modified_colliders(
    &mut self,
    colliders: &mut ColliderSet,
    modified_colliders: &mut Vec<ColliderHandle>,
) {
    for handle in modified_colliders.drain(..) {
        if let Some(co) = colliders.get_mut_internal(handle) {
            co.changes = ColliderChanges::empty();
        }
    }
}
```

`clear_modified_bodies` drains, then returns the capacity. It is
called once, before the CCD loop:

```rust
fn clear_modified_bodies(
    &mut self,
    bodies: &mut RigidBodySet,
    modified_bodies: &mut Vec<RigidBodyHandle>,
) {
    for handle in modified_bodies.drain(..) {
        if let Some(rb) = bodies.get_mut_internal(handle) {
            rb.changes = RigidBodyChanges::empty();
        }
    }
    bodies.modified_bodies = std::mem::take(modified_bodies);
}
```

After the CCD `while remaining_substeps > 0` loop, and not inside it:

```rust
colliders.modified_colliders = modified_colliders;
```

`take_modified` left an empty `Vec` in the set. The local buffer now
holds the capacity built while updating collider positions. Putting it
back means the next step does not grow a new `Vec` from zero.

## Why the collider buffer is returned after the loop

The CCD loop calls `clear_modified_colliders` on every substep. If the
first call moves the capacity back into the set, the next call takes
that capacity and the last clear drops it. Drain-only, plus one
assignment after the loop, keeps the allocation.

`clear_modified_bodies` is safe to assign inside the function only
while it stays a single call before the loop.

## What does not change

Handles, contact pairs, island membership, and query hits stay the
same. The `Vec` is still emptied each step. Only its allocation is
kept, so capacity can grow once and then sit there. Positions and
replay bytes do not move.

`CollisionPipeline::step` has the same `take_modified` and drops the
local `Vec` at the end of `step`. It calls clear once, so it can
assign the drained `Vec` back at the end. We did not need that path.
Worth doing in the same change so both pipelines keep the buffer.

## Dropping our vendor

When a release contains this, delete `third_party/rapier3d` and the
`rapier3d` line under `[patch.crates-io]`, bump the dependency, and
re-run the zero-alloc test above. Do not file this draft as the ask;
it is the text we would send.
