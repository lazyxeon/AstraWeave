# Destruction Design

**Status:** Approved 2026-10-15. Phase 2 is in progress on this branch.
**Base:** `main` `85d09a2c`, stacked on the `Fixed64` move (PR #245). Survey: [`DESTRUCTION_ARCHAEOLOGY.md`](DESTRUCTION_ARCHAEOLOGY.md).
**Decision:** hybrid. A new crate owns the graph, fracture, the command log, terrain-edit deltas, and debris. Physics and terrain get thin one-way hooks. Render and combat are callers, not owners.

Approved answers:

1. `Fixed64` lives in `astraweave-math`. Destruction does not depend on `astraweave-combat`.
2. `STRESS_ITERS` is 8. The Aravath gate matches a 64-iteration solve, so no tuning issue was opened. A longer chain stays partial by the cap.
3. Tests use `FlatColumnBase`. The #215 adapter waits until that PR merges (issue #249).
4. `astraweave-core`'s replay test dev-depends on this crate.

Core directives from 2026-10-15 are folded in as section 15. Where section 2 still describes `Fixed64` as living only on PR #212, that sentence is historical: the type is in `astraweave-math` (PR #245), and this crate does not depend on combat.

---

## 1. Decision

### Options

**(a) Extend `astraweave-physics` and `astraweave-terrain`.**
`destruction.rs` would grow a support graph, and terrain would grow CSG. The physics crate's step is Rapier `f32` with `HashMap` body maps and without `enhanced-determinism`. `DestructionManager` stores `f32` health in a `HashMap` and spins debris with `sin`. `Density` is `f32`. `TerrainModifier` spends a frame budget with `Instant`, and PR #215 deletes that file. Putting the hashed sim in those crates ties every destruction test to Rapier and to the terrain PR's churn.

**(b) A new crate and no hooks.**
The graph would be testable, and it would also be another system with zero production callers. That is how `DestructionManager` and `SpatialHash` already shipped.

**(c) Hybrid. Chosen.**
`astraweave-destruction` owns every value that enters the world hash. `astraweave-physics` receives poses and does not write them back. `astraweave-terrain` keeps generating the base column. The editor's float brush and the dormant float manager stay where they are until a later deletion, so this work does not pretend they are the new system and does not delete them out from under their tests.

### Why this shape

| Pressure | What it forces |
|---|---|
| Coupling | The hashed state cannot sit behind Rapier handles. Blast's split (graph here, rigid bodies elsewhere) matches the crate boundary we already have. |
| Determinism | The new crate can forbid `f32` in sim modules the way `astraweave-combat/src/fixed.rs` already checks its own source. The physics crate cannot. |
| Tests | Graph, fracture, brushes, and the replay hash run without wgpu and without Rapier. |
| #215 | Column generation, integer erosion, and `ColumnStream` stay in terrain. Destruction stores deltas and calls those functions. |
| #216 | No new pass, sky, or material. Debris and decals are data the existing renderer can draw. |
| #212 | One `Fixed64`. Destruction does not reimplement hit resolution. |
| #205 | One dig path. The demo's `VoxelEdit` log is replaced by the command log, not paralleled. |

### What Phase 2 will not do

- Grow `add_destructible_box`, `break_destructible`, or `DestructionManager`.
- Revive `TerrainModifier`, `background_loader`, or GPU erosion.
- Add a second mesher, tonemap, or IBL path.
- Implement ledge climbing. Flags only.
- Enable `async-physics` or read Rapier results back into the hash.
- Copy `Fixed64` into this crate.

---

## 2. Fixed64 and SimRng

`SimRng` on `main` is `astraweave_core::SimRng::new(world_seed, tick, system_id)` (`astraweave-core/src/sim.rs`). Draws are ChaCha12. Integer draws are the sim interface. `unit_f32` exists and is not used for fracture or stress.

`Fixed64` is not on `main`. The implementation that matches the contract is `astraweave-combat/src/fixed.rs` on PR #212:

- raw `i64`, `SCALE = 1 << 32`
- `from_int`, `from_ratio`, `saturating_add`, `saturating_sub`, `mul` via `i128`, `mul_ratio`
- division truncates toward zero, `den == 0` yields zero, overflow saturates
- no float conversion

PR #212 says to delete that module and re-export the single type once it lives on `main`. PR #205's runtime already calls another copy (`astraweave_veil::Fixed64`). A third copy is rejected.

**Phase 2 gate.** The first implementation commit depends on one public `Fixed64`. Preferred home is `astraweave-math`, with `astraweave-combat` re-exporting it, because combat's golden hashes (`071dc70e…` stage, `975aae72…` battle) are the drift alarm. That move is a coordination edit with #212, called out in the PR, and it is not a new arithmetic. Until #212's owner accepts the move, `astraweave-destruction` depends on `astraweave-combat` and uses `astraweave_combat::Fixed64`. It does not vendor the file.

**RNG.** New constant, not taken by `main`, #212, or #215:

```text
SIM_SYSTEM_DESTRUCTION = 0xD357_7001
```

It is added to `astraweave-core/src/sim.rs` next to the existing constants. Weaving's duplicate list is not extended.

```rust
pub fn destruction_rng(world_seed: u64, tick: u64, entity_id: u64, seq: u64) -> SimRng {
    let system = SIM_SYSTEM_DESTRUCTION
        ^ entity_id.rotate_left(17)
        ^ seq.rotate_left(41);
    SimRng::new(world_seed, tick, system)
}
```

One `SimRng::new` per command. The fold matches `hit_rng` on #212. No fourth constructor, no `thread_rng`, no `HashMap` walk, no clock.

---

## 3. Data model and command log

Applied inside the fixed 60 Hz tick, before any mirror is written. Order is `(tick, seq, entity_id)` ascending. `seq` is assigned by the producer in the order the tick accepted the inputs. A cascade that breaks a bond appends a `Detach` with the next `seq` in that same tick, so the cause sorts first.

```rust
pub struct DestructionCommand {
    pub tick: u64,
    pub seq: u32,
    pub entity_id: u64,
    pub kind: CommandKind,
}

pub enum CommandKind {
    Impact(Impact),
    Explosion(Explosion),
    Brush(Brush),
    Detach(Detach),
    Anchor(Anchor),
}

pub struct Impact {
    pub point: [Fixed64; 3],
    pub normal: [Fixed64; 3],
    pub impulse: Fixed64,
}

pub struct Explosion {
    pub center: [Fixed64; 3],
    pub radius: Fixed64,
    pub impulse: Fixed64,
}

pub struct Brush {
    pub shape: BrushShape, // Sphere, Box, Capsule
    pub op: BrushOp,       // Subtract, Add, Smooth
    pub origin: [i32; 3],  // quantized to the density grid
    pub radius: u16,       // cells
    pub half_extents: [u16; 3],
    pub strength: i16,     // density steps, or height steps for a crater
}

pub struct Detach {
    pub bond_id: u64,
}

pub struct Anchor {
    pub node_id: u64,
    pub anchored: bool,
}
```

`entity_id` is the structure, the chunk column, or zero for a world brush. Coordinates that enter the log are integers or `Fixed64`. A float from gameplay is converted once at the edge with an explicit quantize (`from_ratio` of milli-units) and the float is dropped.

The log is `Vec<DestructionCommand>`, already sorted. Serialization is `bincode` of that vec (serde, little-endian, no map). The same bytes are the replay, the save blob, and the reliable network payload.

Rejected inputs (unknown entity, `anti_destroy`, zero radius, `seq` going backwards) do not enter the log. Rejection is a returned `Result`, not a swallowed error.

### Per-tick hash

`destruction_state_hash(tick) -> [u8; 32]` is SHA-256 over:

1. magic `b"AWDEST1"` and the tick as `u64` LE
2. each command applied at this tick, in order, in the serde bytes
3. the support graph: nodes sorted by id, then bonds sorted by id
4. debris pool slots in index order, including empty slots
5. dirty terrain chunks in `(x, y, z)` order, then their delta tails

`DefaultHasher` is not used. Rapier transforms, particle positions, and display meshes are not inputs. The hash is what `replay_hash` prints. Net's existing `world_hash: u64` stays untouched in Phase 2; a later net change can store this digest beside it.

---

## 4. Graph tiers

```rust
pub struct NodeId(pub u64);
pub struct BondId(pub u64);

pub struct SupportNode {
    pub id: NodeId,
    pub mass: Fixed64,
    pub island: u64,
    pub flags: PieceFlags,
}

pub struct Bond {
    pub id: BondId,
    pub a: NodeId,
    pub b: NodeId,
    pub strength: Fixed64,
    pub area: Fixed64,
    pub intact: bool,
}

bitflags! {
    pub struct PieceFlags: u32 {
        const ANCHOR     = 1 << 0; // ground or authored static
        const ANTI_DESTROY = 1 << 1;
        const CLIMBABLE  = 1 << 2;
        const ANTI_CLIMB = 1 << 3;
        const DYNAMIC    = 1 << 4; // set by the solver, not by authoring
    }
}
```

Storage is a `Vec<SupportNode>` sorted by `NodeId` and a `Vec<Bond>` sorted by `BondId`. Adjacency is rebuilt from the bond vec when the graph is dirty. No `HashMap`.

`ANTI_CLIMB` wins over `CLIMBABLE`. Nothing on `main` or on the open traversal search reads these bits. `CharacterController.max_climb_angle_deg` remains a slope limit. The flags are published so the traversal work can consume them without a second graph. Phase 2 does not add a climber.

### Tier A — connectivity

Every tick, if the graph is dirty:

1. Walk nodes in id order.
2. Flood-fill from every node with `ANCHOR`, crossing only intact bonds, visiting neighbors in bond-id order.
3. Unreached nodes form islands. An island id is `mix_sim_seed(parent_structure_id, lowest_node_id, SIM_SYSTEM_DESTRUCTION)`.
4. An island with no anchor gets `DYNAMIC` on every member. Those nodes become debris (section 7) unless `ANTI_DESTROY` is set, in which case the island stays kinematic and the command is rejected into the event log as a no-op record.

This is Teardown-style support. It is the fallback if Tier B is over budget.

### Tier B — fixed stress

On the dirty subgraph only, for exactly `STRESS_ITERS` (8) iterations:

1. Each dynamic-capable node starts with load equal to its mass.
2. Nodes are visited in id order. A node pushes load along intact bonds in proportion to `area`, ties broken by bond id. The transfer uses `Fixed64::mul` and `mul_ratio`.
3. After the last iteration, any intact bond whose load is greater than `strength` emits `Detach` with the next `seq`.
4. Tier A runs again on the same tick after those detaches.

The iteration count does not change when the residual is small. There is no "until converged" loop and no sparse-direct solve in Phase 2. A direct solve can replace the iteration later only if it is integer and the iteration order is fixed. That replacement needs its own review because it changes the hash.

Dirty tracking: a command marks the structure it touches. A tick resolves at most `MAX_NODES_PER_TICK` (1024) nodes, lowest structure id first, and leaves the rest dirty. The cap is part of the hash (the same nodes are deferred on every OS).

---

## 5. Fracture pipeline

Two sources, one bond graph.

### Offline prefracture

A bake binary under `astraweave-destruction` (not a new renderer) writes a `PrefractureAsset`:

- nodes, bonds, `PieceFlags`, material id
- chunk AABBs in `Fixed64`
- mesh slices as ordinary glTF the existing loader already accepts

The bake is seeded with `destruction_rng` and a documented tool version string. The asset manifest stores SHA-256 of the bake output. Runtime does not re-roll an authored building. Patterns are Voronoi, cluster (parent chunk stands in for its children until the parent bond breaks), and planar. Cluster break is a `Detach` of the parent bond, Chaos-style, without simulating every child.

### Runtime cuts

Used for a local hit on a wall or a prop that has no bake.

1. `destruction_rng(world_seed, tick, entity_id, seq)` produces cell sites.
2. Sites are sorted by the integer cell id.
3. Each cell is the intersection of `Fixed64` bisector planes. Plane math is `mul` and `saturating_sub` only.
4. Output triangles are ordered by cell id, then by plane id.
5. New nodes and bonds append with ids `mix_sim_seed(parent_node, cell_id, SIM_SYSTEM_DESTRUCTION)`.

A material table, loaded as data, supplies strength, minimum chunk size, debris class, audio tag, and VFX tag. `ANTI_DESTROY` skips the cut and records the command as applied-but-inert so replays stay aligned.

The display mesh of a runtime cut is built from the integer planes with float vertices at the render edge. Those vertices are not hashed and are not the collider. The collider is the cell AABB, or the quantized polygon extruded by one grid step.

---

## 6. Terrain edits and streaming

#215 owns the base. Destruction owns the delta.

```rust
pub trait TerrainBase {
    fn column_top(&self, x: i32, z: i32) -> i32;
    fn column_material(&self, x: i32, y: i32, z: i32) -> u16;
}
```

The Phase 2 adapter calls `surface_height` and `column_material` from #215's `column_gen.rs`. Until that PR is on the branch being built, tests use `FlatColumnBase` (a constant top). The adapter is a separate file so a rebase onto #215 does not rewrite the solver. Destruction does not modify `column_gen.rs`, `erode_chunk_heights`, `ColumnStream`, or scatter.

### Representation

- **Crater / trench.** `Brush` on a column chunk adds an `i32` height delta at integer `(x, z)`. The sim top is `column_top + delta`. No overhang.
- **Tunnel / fill.** An edited chunk gains a sparse `i8` density overlay, 0 empty, 127 solid, only for chunks that have a brush. Unedited columns stay a height. This overlay is destruction state, not a new voxel module inside terrain.
- **Smooth.** A fixed 3-tap on the `i8` field in `(x, z, y)` order. Not a float kernel.

Collision is a set of AABBs (or a height span) built from the integer top and the `i8` sign. `DualContouring` and `collision_mesh_from_chunk` stay display-side. The float mesh is never sampled by the sim.

### Streaming

#215's `ColumnStream::around` decides which chunks exist. Destruction keeps an append-only `BTreeMap<(i32, i32, i32), Vec<CommandRef>>` for every chunk that has ever been edited, including unloaded ones.

On load:

1. Ask `TerrainBase` for the seed column.
2. Replay that chunk's delta tail in command order.
3. Hash the integer result.

On unload, drop the mesh and the mirror body. Keep the delta. The test loads a chunk, digs, unloads, loads, and checks byte-equal occupancy and an equal `destruction_state_hash`.

`morphed_height` is render LOD. Edited chunks use the full integer field, which is the rule #215 already states. Destruction does not author a second LOD morph.

Remesh requests are a sorted list of chunk coords, at most `MAX_REMESHES_PER_TICK` (4). The rest stay dirty. Display meshing runs outside the hash.

---

## 7. Debris lifecycle

Gameplay debris is a slot in a fixed pool. Capacity `MAX_SIM_DEBRIS = 256`. The pool is allocated when the world is created. Steady state does not allocate.

```rust
pub struct DebrisSlot {
    pub occupied: bool,
    pub id: u64,
    pub born_tick: u64,
    pub pos: [Fixed64; 3],
    pub vel: [Fixed64; 3],
    pub ang: [Fixed64; 3],
    pub sleep_held: u16,
    pub lod: DebrisLod,
}
```

Each tick, in slot order:

1. Integrate with gravity `Fixed64::from_ratio(-981, 100)` (the same `-9.81` as `PhysicsConfig`, as a ratio, not a float).
2. Resolve against the integer terrain and against other occupied slots' AABBs, lower id first.
3. If linear and angular speed stay under the sleep threshold for `SLEEP_TICKS` (30), set sleeping and merge the slot into a static body on the graph (an anchor if it rests on the ground).
4. LOD, in order: rigid slot, proxy AABB (no angular integration), cosmetic, despawn. Cosmetic is a render particle. It is not a slot and it is not hashed.
5. When a new piece needs a free slot and none is free, evict the occupied slot with the smallest `born_tick`, then the smallest `id`.

`DestructionManager.max_debris` (500) and the editor slider are not this pool.

The physics mirror, once per tick after the pool step:

```rust
fn mirror_debris_pose(id: BodyId, pos_milli: [i64; 3]);
```

The conversion to `Vec3` happens inside physics, for Rapier only. Destruction does not call `body.translation()` and does not read contacts back. If the character must stand on rubble, it queries the integer AABBs in this crate. #205's Rapier motor remains the demo body until that demo is ready to sample the AABBs; the replay case does not include the motor.

---

## 8. Budgets

Enforced in the tick, so a late OS cannot do extra work and diverge.

| Cap | Value | When it binds |
|---|---|---|
| `MAX_SIM_DEBRIS` | 256 | Pool eviction |
| `MAX_NODES_PER_TICK` | 1024 | Tier B deferral |
| `STRESS_ITERS` | 8 | Always, not "until converged" |
| `MAX_REMESHES_PER_TICK` | 4 | Extra dirty chunks wait |
| `MAX_BRUSH_CELLS` | 4096 | Larger brushes are rejected |
| `SLEEP_TICKS` | 30 | Merge-to-static |
| `MAX_LOG_COMMANDS` | 65536 | Late join closes after this |

Criterion benches, names fixed so the bench CI can pick them up:

| Bench | Measures |
|---|---|
| `graph_solve` | Tier B on N nodes, N in {64, 256, 1024} |
| `fracture_cut` | One runtime planar cut and one 16-cell Voronoi |
| `brush_remesh` | One sphere subtract plus integer occupancy rebuild of one chunk |
| `debris_step` | One tick of a full 256-slot pool |

They follow #223: not a pull-request check. They run on `main`, nightly, `workflow_dispatch`, or a PR labeled `run-benchmarks`. Documented budgets are the caps above, not a wall-time gate, until a labeled run records numbers. Those numbers land in the PR before it leaves draft. `docs/current/MASTER_BENCHMARK_REPORT.md` is updated only if a measured path moves more than 10 percent; a new bench with no prior number gets a revision note when the first number exists.

---

## 9. Replay and network

### Replay case `destruction`

Added to `astraweave-core/tests/replay_hash.rs`'s JSON and to `required` in `.github/workflows/determinism-replay.yml`.

`astraweave-core` cannot depend on the new crate (the new crate depends on core). The workflow step stays one test binary by moving the case list to a tiny runner only if needed. The planned shape, which avoids a dev-dependency cycle:

1. `astraweave-destruction` exposes `fn demolition_bytes(world_seed: u64) -> Vec<u8>`.
2. `astraweave-core/tests/replay_hash.rs` gains a dev-dependency on `astraweave-destruction`. Cargo allows a dev-dependency cycle. The test calls `demolition_bytes` and emits the `destruction` row next to `planner_pure` and `perception_world`.
3. The scripted scene, 300 ticks at 60 Hz:
   - tick 10: impact on a prefractured wall
   - tick 20: explosion that overloads a bond
   - tick 30: Tier B collapse of the island
   - tick 40 and 50: sphere dig and a box fill
   - tick 60: unload the edited chunk and load it again
   - remaining ticks: debris integrates and at least one slot sleeps
4. The hashed bytes are the per-tick `destruction_state_hash` concatenated. The compare job fails if the case is missing or the three OS digests differ.

Required-set merge. This branch's workflow lists `planner_pure`, `perception_world`, and `destruction`. Rebase onto a `main` that already has `combat_resolve` (#212), `worldgen_columns` (#215), or `gameplay_rng` (#223) keeps those names and adds `destruction`. It does not delete a case that landed first.

`ai_depth_tick` and `scripted_system` are not on `main` or on the open heads surveyed. They are not invented here.

### Network

Reliable ordered channel carries `DestructionCommand` bytes. Unreliable state sync is not used for gameplay debris, because the log plus the seed already determines the pool.

Late join sends `(world_seed, log prefix)`. The prefix is capped at `MAX_LOG_COMMANDS`. Past the cap, late join is refused. A full snapshot of debris poses is the fallback this design does not take.

Measured at implementation time and written back into this section:

| Payload | Bytes | Notes |
|---|---|---|
| One command | to be filled | Header is 8+4+8 plus the kind tag and fields. Expect well under 128 bytes. |
| Scripted collapse (300 ticks) | to be filled | The log, not 256 poses per tick. |
| Snapshot of 256 debris poses | to be filled | Contrast only. Not sent. |

`astraweave-net`'s `EntityState` stays the player snapshot. Phase 2 does not rewrite the server. The overlap is a note: a later change can attach the command stream beside `Delta`.

### Save

The log is a named blob. #212 stores combat tables on `SaveBundleV2.meta` and keeps schema version 2. Destruction will not add a parallel meta key until that PR's owner confirms the field is free. The replay test does not need the save crate. The demo save path, when #205 is on the branch, stores the same bytes the net channel would send.

---

## 10. Interfaces with the open PRs

### #212 combat

Combat resolves the hit. When the target is a structure, the caller enqueues `Impact` with the impulse already in `Fixed64`. Destruction does not import armor, schools, or `resolve_hit`. The only shared type is `Fixed64`, via the gate in section 2. `SIM_SYSTEM_DESTRUCTION` does not overlap `SIM_SYSTEM_COMBAT`.

### #215 terrain

Read `surface_height`, `column_material`, `column_voxels`, and `ColumnStream` load sets. Do not change erosion, scatter, LOD hysteresis, or the files #215 deletes. Edited chunks stay on the full mesh by feeding the integer field, not by forking `morphed_height`. If the adapter needs a one-line `pub use`, that edit is listed in the PR and is not a second generator.

### #216 render

No edits to IBL, `TimeOfDay`, `pbr_maps`, or pass list. The showcase uses `add_model` / the composite path #216 adds, and removes a model by the existing name when a chunk is gone. A `replace_mesh` hook is not added unless the demo cannot swap by name. If it is added, it is a few functions at the end of `Renderer` and it is flagged. Decals stay dormant; a damage decal is a `Decal` value produced by destruction and left for a render owner to draw. Cosmetic particles use the existing particle path and are not hashed.

### #205 Aravath

`dig_sphere` and `Vec<VoxelEdit>` stop being the sim. `Sim` enqueues `Brush` and reads occupancy from this crate. Player-facing strings say Aravath. CC0 only: reuse the KayKit, Kenney, and Poly Haven files already on that branch; a destructible building is KayKit or Kenney, with the license line in the asset manifest. Do not vendor a second copy of the forest pack.

The demo's Rapier character is not in the destruction hash. The showcase assertion is the destruction hash plus a visible collapse, not a claim that the motor is bit-identical.

### #223 CI

`determinism-replay.yml` still runs on every PR with no path filter. Benches do not. Docs-only commits still run the replay workflow; they do not need a Rust compile of the workspace once #223's docs skip is on `main`. Until then, this docs PR may compile. A red check that this PR did not cause is fixed or filed as `deferred-failure` with an owner and a plan. It is not ignored, and it is not `#[ignore]`.

### Traversal

No branch and no PR as of 2026-10-15. `PieceFlags::CLIMBABLE` and `ANTI_CLIMB` are the interface. A later traversal change reads them off the node. This crate will not grow a mantle or vault solver.

---

## 11. Test plan

| Test | Asserts |
|---|---|
| Command sort | Same multiset of commands, different insertion order, same applied hash |
| Rejected command | Does not change the hash |
| Tier A | Removing the only anchor bond marks the island `DYNAMIC` and the island id matches the mix formula |
| Tier B | A fixed overload breaks the same bond on two runs; iteration count stays 8 when the load is already past the limit on iteration 1 |
| Fracture | Two calls with the same `(seed, entity, tick, seq)` produce the same planes in the same order |
| Material | `ANTI_DESTROY` records the command and keeps the mesh |
| Brush | Sphere subtract, box add, and smooth are deterministic on the `i8` overlay |
| Crater | Height delta matches across two runs; unedited columns stay `column_top` |
| Stream | Unload, load, occupancy bytes equal, hash equal |
| Debris | Full pool evicts the oldest lowest id; sleep holds for 30 ticks then merges; cosmetic particles are absent from the hash |
| No float | A source scan of sim modules fails if it sees `f32`, `f64`, `HashMap`, `Instant`, or `thread_rng` |
| Replay | Case `destruction` present and identical on Ubuntu, Windows, and macOS |
| Budgets | A 1025-node graph defers the highest id; a 257th debris piece evicts |

Clippy for the new crate is `-D warnings` on the library. `cargo check -p astraweave-destruction` after each Phase 2 edit. Unsafe is forbidden in the crate.

---

## 12. Aravath showcase (Phase 2, after approval)

A headless demolition scene ships with the crate so the replay case and the demo share one script. When `aravath_runtime` is on the branch:

- A CC0 building stands on the integer ground. The player can hit it until Tier B drops an island.
- A brush digs a crater, and a second brush digs a short tunnel in the `i8` overlay.
- Debris settles, sleeps, and merges.
- HUD and prompt text use the name Aravath.
- The asset manifest records the pack name, author, and CC0.

The showcase is not a second copy of `physics_demo3d`.

---

## 13. Phase 2 order

Each step is its own commit, with tests, and stops if a pre-existing failure shows up.

1. Crate skeleton, `Fixed64` dependency gate, `SIM_SYSTEM_DESTRUCTION`, source scan.
2. Command log, sort, hash, serde roundtrip.
3. Tier A and Tier B.
4. Runtime fracture and the material table. Offline bake and its manifest hash.
5. Terrain delta, `FlatColumnBase`, unload/reload test. Adapter file for #215, compiled when that API is present.
6. Debris pool.
7. Replay case and the workflow required set.
8. Benches, not wired into PR checks.
9. Physics mirror function, only if the demo needs a collider it cannot get from the AABB query.
10. Aravath call site, CC0 building, player text.

Known limitations to keep in the PR until they are done: Rapier is not in the hash; decals are not drawn; ledge climb is a flag; late join refuses past the cap; `DestructionManager` remains a dormant float module.

---

## 15. Core directives (2026-10-15)

These constraints sit on top of the approved design. Simulation results stay bit-identical across operating systems.

### Spatial partition

Edited chunks live in a sparse voxel DAG (`VoxelDag` in `octree.rs`). Identical leaves and identical branches share a node. A brush or weave rebuilds that chunk from the integer density slab. Cells with no edit answer from `TerrainBase::column_top`. Today that base is `FlatColumnBase`. When #215 merges, the same trait calls `surface_height` (issue #249). The DAG does not replace the column generator.

### Queries off the sim thread

`QueryPool` starts two workers when the world is created. A tick publishes a DAG image and a fixed list of points. Each worker claims an index, walks the image, and writes that slot. The main thread reads the slots in index order. The walk does not take a lock; the publish barrier is the only wait, and it is quiet while the walk runs. Thread count does not change the merged bytes. Those bytes are in the world hash, and `parallel_queries_match_the_dag_walk` checks them against a direct walk.

GPU compute is not on the simulation path. An integer GPU solve would need its own three-OS replay proof before it could write simulation state. Until then, GPU work stays on meshes, particles, and other derived data.

The 8-iteration load push stays on the sim thread. Inside one structure the push order is the result. Independent structures can later be separate jobs with the same merge rule. The node cap is what keeps a collapse inside the tick. `collapse_tick_fits_in_an_11ms_frame` checks the scripted collapse tick against the 11 ms frame budget. `tests/tick_alloc.rs` counts heap calls and requires zero allocations on warmed ticks. The first time a node loses support it takes one seeded planar cut (`planar_children`). `bake_gate` is the offline prefracture; its SHA-256 is locked in `prefracture_manifest_hash_is_locked` and `assets/aravath_destruction/prefracture_manifest.txt`.

### No heap and no lock contention on the tick

Brush edits, the DAG, notices, and the per-tick command batch sit in buffers reserved when the world is created. Graph scratch for the load push is reserved in `Graph::new`. Workers are not spawned from `tick`. The job body does not take a lock.

### Persistence

`SpawnNode` and `SpawnBond` are commands, so the gate is in the log rather than only in the constructor. `persist_bytes` writes the seed, the tick, and every command. `restore` builds an empty world and replays the log up to that tick. `command_log_restores_the_gate_and_the_crater` checks the hash, the crater, and the fallen lintel.

### Weaving and navigation hooks

`enqueue_weave` records a `Weave` command: an integer density delta and a `Fixed64` decay inside a radius. Decay adds bond damage. The command is replayed with the rest of the log. Weaving calls this hook; destruction does not depend on the weaving crate.

`GeometryNotice` records an integer box and a reason (brush, collapse, weave). `drain_geometry_notices` hands them to navigation in record order. Destruction does not depend on the nav crate, which is `f32`. Navigation reads the notices and rebuilds its own graph outside the hashed tick.

## 14. Review questions

Answered at the top of this document. `Fixed64` is in `astraweave-math`. The load push stays at 8 iterations. `FlatColumnBase` remains until #215 (issue #249). The core replay test dev-depends on this crate.
