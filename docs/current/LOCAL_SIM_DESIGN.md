# Local Simulation Design

**Status:** Phase 1, waiting for review. No crate code in this change.
**Base:** `main` `85d09a2c`. Survey: [`LOCAL_SIM_ARCHAEOLOGY.md`](LOCAL_SIM_ARCHAEOLOGY.md).
**Coordination:** PR #252 (`feat/weaving-framework`, `docs/current/WEAVING_FRAMEWORK_DESIGN.md`). Weaving owns templates, the command log, and sinks. This crate owns the field storage, the activation map, and the neighbor solver. Weaving writes modifiers through `FieldWrite`. It does not store a second grid.

Phase 2 does not start until this document is approved and the schema notes on #252 are confirmed.

---

## 1. Decision

`astraweave-localsim` holds eight regions of 16³ integer cells. The five channels are the ones #252 named: temperature, phase, density delta, surface flags, gravity index. A tick applies that PR's writes, then runs four neighbor passes in a fixed order, with no allocation. GPU fluids and the Gerstner surface stay presentation.

The earlier draft of this file used 0.5 m cells, a packed material/fill/velocity word, and a 24-chunk active cap. Those numbers are retired. #252's channels are the schema, because that is the record weaving can fill without a translation table.

### Options set aside

**(a) Extend `FluidSystem` and hash the GPU particles.**
`step` already exists, including a temperature field and a Boussinesq term (`shaders/fluid.wgsl` 136–146). The iteration count is adaptive and the neighbor list is built with atomics (`astraweave-fluids/src/lib.rs` 1160–1167). The water crate's determinism carve-out excludes that state from replay.

**(b) Restore the deleted voxel water grid.**
`volume_grid.rs` left in W.1. That grid was `f32` water, not this channel set.

**(c) Lattice Boltzmann, PIC/FLIP, or MPM as the hashed state.**
D3Q19 stores 19 distributions per cell. FLIP and MPM scatter particles onto a grid. Both fight a fixed visit order and the 11 ms frame. The GPU PBD path remains the splash visual.

**(d) A second `FieldAgent` inside `astraweave-weave`.**
#252's draft puts the arrays in that crate. Two owners of the same channels will drift. Storage moves here. Weaving keeps the graph that emits writes.

**(e) Semi-Lagrangian stable fluids.**
Stam, *Stable Fluids*, SIGGRAPH 1999, traces characteristics with interpolation. That interpolation is where libm diverges. This tick does not project a pressure field.

### What Phase 2 will not do

- Store the five channels in `astraweave-weave`.
- Hash `FluidSystem` particles, SSFR textures, or Gerstner displacements.
- Allocate inside `tick`.
- Put `f32`, `HashMap`, `Instant`, or `thread_rng` in the sim modules.
- Draw from `SimRng` inside the neighbor passes. A template that needs a roll draws in weaving, in command order, before the write is emitted.
- Apply `PhysicsWorld.wind` as a Rapier force.
- Edit `WorldSnapshot` in this crate. The hazard array is filled here. Copying it onto the snapshot waits on #252 review question 1.
- Retune swim thresholds or break a destruction bond.
- Depend on `Fixed64`. Deci-degrees and milli-g stay integers. #252 question 5 matches this.
- Fix the lavapipe SSFR compile (#247).

---

## 2. Schema

Cell size is 1 m. A region is 16 cells on a side, 4096 cells, origin at the subject's cell minus `(8, 8, 8)`. Index, with `x` fastest:

```text
i = (z * 16 + y) * 16 + x    // x, y, z in 0..16
```

Eight regions, allocated with the world. Slot 0 is the player. A subject that already has a slot keeps it. A new subject takes the lowest empty slot. When all eight are full, the highest `subject` id other than the player is cleared, in index order. That is #252's rule, and it is the whole activation map: a slot is active or empty. There is no ninth region.

### Channels

SoA, one array per channel per region. These are the bytes weaving's hash should treat as the field.

| Channel | Type | Unit | Who writes it |
|---|---|---|---|
| `temp` | `i16` | deci-degrees, 0 = freeze point of the water row | weaving via `AddTemp`, then the solver's conduction |
| `phase` | `u8` | 0 dry, 1 wet, 2 ice, 3 burn, 4 gas, 5 oil | weaving via `SetPhase`, then reactions |
| `density_delta` | `i8` | kilograms per cubic metre added to the row's `density0` | weaving via `AddDensity` |
| `flags` | `u8` | see bits | weaving and the solver |
| `gravity` | `u16` | 0 = world default, else a zone-table index | weaving via `SetGravity` |

Flag bits, matching #252:

| Bit | Name |
|---|---|
| 0 | wet-surface |
| 1 | ice-surface |
| 2 | burning |
| 3 | slippery |
| 4 | conductive |
| 5–7 | zero until #252 assigns them |

Phase values above 5 are not written. Poison stays out of the field until weaving names a phase or a flag bit.

`density0` is weaving's material column, in kg/m³ (water 1000, oil 800). `density_delta` adds to it and saturates at `i8` (±127). The sum is what `WaterQuery` widens to `f32`. Multiples of 1.0 are exact. The float is not stored and is not a hash input.

Tag bits on the material row, so the solver can read the table weaving already owns:

| Bit | Tag |
|---|---|
| 0 | Flammable |
| 1 | Conductive |
| 2 | Structural |
| 3 | Freezable |
| 4 | Wettable |

`ignite` and `freeze` are `i16` deci-degrees on that row. Strength, debris, and audio stay on #243's row, joined by the same `u16` material id. This crate does not copy strength.

### The write interface

Weaving expands a template into stamps. It does not rasterize cells and it does not hold the arrays. The host passes the stamp list into `LocalSim::tick`. `astraweave-weave` does not depend on this crate. This crate does not depend on `astraweave-weave`. The structs below are the contract. Phase 2 copies them into both crates and locks the layout with a byte-size test.

```rust
pub const MAX_FIELD_WRITES: usize = 64; // #252 MAX_COMMANDS_PER_TICK
pub const MAX_REGIONS: usize = 8;
pub const REGION_EDGE: usize = 16;

pub struct FieldWrite {
    pub seq: u32,
    pub shape: Shape,
    pub origin: [i32; 3],
    pub end: [i32; 3],
    pub radius: u8,
    pub op: WriteOp,
}

pub enum Shape {
    Point,
    Sphere,
    Segment,
    Column,
}

pub enum WriteOp {
    AddTemp { delta: i16, stack: bool },
    SetPhase { phase: u8 },
    AddDensity { delta: i8, stack: bool },
    SetFlags { set: u8, clear: u8 },
    SetGravity { index: u16 },
}
```

`Sphere` covers cells whose squared Euclidean distance from `origin` is `<= radius²` (`i32`, no square root). `Segment` is the Bresenham cells from `origin` to `end`, thickened by the Chebyshev radius. `Column` is every cell in an active region with the same `x` and `z` as `origin`. `Point` is the one cell.

A stamp whose cells all miss the eight regions is dropped. The drop count is hashed. The stamp is not stored.

Fold, per world cell, after rasterizing every stamp in `(seq)` order and before the neighbor passes:

1. **Temperature.** Positive deltas and negative deltas are folded separately. `stack` sums that sign. Without `stack`, the sign keeps the delta with the largest magnitude, ties taking the later `seq`. The two sides then annihilate: subtract the smaller magnitude from the larger and keep the winner's sign. Equal magnitudes leave 0. This is #252's Max, Stack, and Cancel.
2. **Density.** Same Max versus Stack. No cancel.
3. **Phase and gravity.** The highest `seq` wins.
4. **Flags.** Walk the stamps in `seq` order. `set` ors bits. `clear` clears bits.

Saturating arithmetic. No epsilon.

### Overlap

Two subjects can cover the same world cell. The slot with the lower index owns the cell. Higher slots read that cell as a halo and do not write it. Visit order still walks every slot, and a non-owner cell is skipped. One world cell has one phase.

### Neighbor passes

Four passes, always, even when a pass changes nothing. One reaction per cell per tick. Visit order is region slot, then `z`, then `y`, then `x`. Neighbors are `±X`, `±Y`, `±Z`, in that axis order.

Each pass reads the current buffer and writes the next buffer. The scratch is one `[NextCell; 4096]`, reused across regions, allocated with the world. Jobs may run one region each. A job writes only its slot's next buffer. The owner rule means two jobs never write the same world cell. No atomics and no locks. After the four passes the buffers swap in slot order.

Inside a pass, for each owned cell:

1. **Gas** (phase 4) steps one cell along gravity. Index 0 steps `+Y`. Any other index steps along the axis in the zone record the host passed in (see below). The destination must be dry and empty of solid. The source becomes dry.
2. **Liquid** (phase 1 or 5) steps one cell along gravity, default `−Y`, into dry or gas. Gas at the destination swaps upward.
3. **Heat.** `next = temp + (neighbor_sum - 6 * temp) / 4`, division toward zero, `i32` intermediate, saturate to `i16`. Same rate for every cell. Conductivity is not a column on the weave material row, so the solver does not invent one.
4. **One reaction**, first match, and then the cell is done for the tick:
   - wet, and `temp <= freeze` → ice, set ice-surface
   - dry or oil, tag Flammable, and `temp >= ignite` → burn, set burning
   - burn with a wet neighbor → wet, clear burning
   - conductive tag, and a neighbor already marked shocked this tick → mark shocked, one `FieldEvent`

Solid cells come from `SolidField` (#243's overlay, or `FlatSolid` in tests). A solid cell is not a move destination. The solver does not write the overlay.

Gravity presets are weaving's zone table. Each tick the host passes a sorted copy, cap 32, highest priority then lowest zone id, which is #252's sample order:

```rust
pub struct GravityView {
    pub index: u16,
    pub axis: u8, // 0 down, 1 zero, 2 +X, 3 -X, 4 +Y, 5 -Y, 6 +Z, 7 -Z, 8 point
    pub strength_milli_g: i16,
    pub center: [i32; 3],
    pub radius: u8,
}
```

`axis == 1` does not move liquid or gas. `axis == 8` steps toward `center` when the cell is inside `radius`, ties broken by the neighbor axis order. No `Vec3`.

### Events the host hands back to weaving

The solver notices edges. Weaving turns them into destruction and combat sinks. This crate does not build a `DestructionCommand` and does not call `resolve_hit`.

```rust
pub struct FieldEvent {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub kind: u8, // 0 ignited, 1 froze, 2 cleared_burn, 3 shocked
    pub material_id: u16,
    pub temp: i16,
}
```

Cap 128, the same as #252's sink cap. Overflow keeps the lowest `(z, y, x)` and hashes the drop count. Decay against destruction's strength stays in weaving: it can read `density_delta` after the tick and emit `Impact` itself. This crate does not read strength.

### Material view passed in

```rust
pub struct MaterialView {
    pub id: u16,
    pub tags: u16,
    pub ignite: i16,
    pub freeze: i16,
    pub density0: u16,
}
```

Sorted by `id`. The tick binary-searches. The table lives in weaving. The host passes a slice. A missing id is dry, `density0` 0, no tags.

### Sleep

A cleared slot writes its five channels into a sleep record in a preallocated buffer of eight records (the maximum that can be evicted over the life of a full pool is bounded by reusing these eight). The record is the raw arrays, 4096 cells, because eight of them already fit in the pool and an RLE pass would be a second encoding to hash. Reacquiring a subject whose origin still matches restores those bytes. A miss starts from the material row (dry, delta 0, gravity 0) and weaving replays its retained command tail as new `FieldWrite`s on the following ticks. The solver does not replay the weave command log.

The `local_sim` hash covers the sleep records. A region that was dropped and restored matches either the sleep bytes or the base state, and both are in the digest.

### RNG

`SIM_SYSTEM_LOCAL_SIM = 0x51A0_7001`. It does not collide with #252's `SIM_SYSTEM_WEAVE = 0x57EA_7001`, nor with craft, harvest, anchor, patrol, spawner, terrain, combat (`0xC0BA_7001`), or destruction (`0xD357_7001`). Neighbor passes do not construct an RNG. The constant is reserved so a later tie-break has a home, and Phase 2's source scan may treat an unused constant as dead. If the scan requires a use, the hash mixes the constant into the magic and still draws nothing.

---

## 3. Tick

```text
tick(subjects, writes, materials, zones):
  assign slots                         // §2 activation, no alloc
  rasterize writes into a per-region stamp list
      the list is a count plus indices into the caller's write array
  fold each owned cell                 // slot, z, y, x
  for pass in 0..4:
      for slot in 0..8:
          write next buffer from current + halo
  swap in slot order
  emit FieldEvent rows
  publish ColumnWater and HazardSample from the committed buffers
```

`subjects` is sorted by entity id by the caller. `writes` is sorted by `seq` by weaving. The tick does not sort with a heap allocation. An unsorted `seq` is a programming error and the debug assert fires. Release still applies in the array order it was given, so the host's sort is part of the contract.

Channel math does not take a `dt`. One tick is one tick. AI and physics read the field committed on the previous tick, which is #252's lag. Perception runs before this tick's `tick`.

Parallelism is the per-slot next-buffer write inside a pass. Eight jobs, lock-free, result independent of the pool size. A test runs the script on one thread and on eight and compares the digest.

---

## 4. What the references changed

Noita, BotW, Divinity, and Teardown are why the field is a material grid and not a pressure solve. #252 already binds the composition rules (elements write materials, elements write elements, one reaction per cell). This crate executes those rules. It does not keep a second reaction table. The four passes are the execution.

Gas rising one cell per pass is their rule, and it replaces the donor-cell velocity channel from the first draft of this design. Wind is a gravity-zone axis, which is how `RedirectWind` reaches the solver after their translator writes `SetGravity` or a zone record.

---

## 5. Coupling

### Water and swim

`ColumnWater` in `astraweave-water` stays the `WaterQuery` backend from the first draft, fed by this field.

A column is published for each owned cell whose phase is wet, oil, or ice. `surface_milli = (origin_y + y + 1) * 1000`. Wet and oil use `density0 + density_delta`. Ice uses density 0, so buoyancy's Archimedes term is zero and a body at or above the surface takes no impulse. On a miss, `AnalyticWater` answers, which remains the sea plane.

`PhysicsWorld` samples `ColumnWater` first when the host has installed one. Empty columns leave today's buoyancy tests unchanged. This crate does not depend on physics. Physics does not depend on this crate.

#244 keeps `WaterPlayerState::update(submersion, dt)` and computes submersion from two `WaterQuery` samples. Ice is the ice-surface flag plus density 0. This crate does not emit a static AABB. That is #252 review question 2, and the flag is enough for the field.

### Destruction

`SolidField::solid` and `material_id` are read. `HeatBreach` from the first draft is replaced by `FieldEvent` kind `ignited`. Weaving maps that event onto #243's `Impact` or `Brush`. This crate does not write the `i8` overlay.

### AI

```rust
pub struct HazardSample {
    pub cell: [i32; 3],
    pub phase: u8,
    pub temp: i16,
    pub flags: u8,
}
```

Sixteen slots, the same shape as #252's `WeaveHazard`. Selection: occupied cells (phase not dry, or any flag bit) nearest the companion, Manhattan distance, ties broken by `z`, then `y`, then `x`. The "sorted by z, y, x" and "nearest first" sentences in #252 are this one order. The array is the committed field from the previous tick. The host copies it into `WorldSnapshot` only if that review accepts the new `perception_world` hash. Until then the array is still hashed in `local_sim`.

### Presentation

After the hash, a caller may copy `phase` and `temp` into the existing accent upload. The copy is not a hash input. `WeaveInstance` stays the mesh deformation.

---

## 6. Budgets

| Cap | Value |
|---|---|
| Regions | 8 |
| Cells per region | 4096 |
| Writes per tick | 64 |
| Neighbor passes | 4 |
| Reactions per cell | 1 |
| Events | 128 |
| Hazard samples | 16 |
| Gravity views | 32 |
| Sleep records | 8 |
| Column rows | 4096 |

The active cell ceiling is `8 * 4096` checks, four times. That is the budget inside the 11 ms frame. The solver is CPU. The GTX 1660 Ti figure is the whole frame. A criterion bench lands with the crate and stays off pull-request checks (#223). Wall-clock time is not hashed. If a release tick of the full eight regions exceeds 1.5 ms on a 6-thread CPU of that era, the pass body gets simpler. The region count does not grow.

---

## 7. Hash and replay

`local_sim_hash(tick)` is SHA-256 over, little-endian:

1. Magic `b"AWLOCL1"`, the tick, and `SIM_SYSTEM_LOCAL_SIM`.
2. Slot table: subject id and origin, slot order, including empty slots.
3. The five channel arrays of every slot, slot order, index order.
4. Sleep records in slot order.
5. `FieldEvent` rows and the drop count.
6. `ColumnWater` rows.
7. The 16 hazard slots, empty ones included.
8. The write-drop count.

`DefaultHasher` is not used. Rapier, GPU particles, and `WeaveInstance` are not inputs.

#252's `weaving` case should hash commands, sinks, and the 32-byte `field_digest` this function returns, not a private copy of the arrays. Two digests of two grids will not stay equal.

Case `local_sim` is 300 ticks, headless, no wgpu. The script is the same beats as their `weaving` case (heat, spread, freeze, density, gravity, hazard), applied through `FieldWrite` so the channels move without their crate linked. Their case, when it lands, feeds the same writes. This docs change does not edit `determinism-replay.yml`.

---

## 8. Phase 2 order

Each step is its own commit. Stop on a pre-existing failure: fix it, or file `deferred-failure` with an owner and a plan.

1. Crate, the eight regions, the five channels, source scan, `SIM_SYSTEM_LOCAL_SIM`.
2. Activation, overlap ownership, sleep records, hash.
3. `FieldWrite` raster and the fold (Max, Stack, Cancel).
4. Four neighbor passes, gas, liquid, heat, one reaction.
5. `FieldEvent` ring.
6. `ColumnWater` and the buoyancy sample order.
7. `HazardSample`.
8. `SolidField` plus `FlatSolid`.
9. Replay case `local_sim`.
10. Showcase when the Aravath runtime from #205 is on the branch. CC0 only. Player text says Aravath.

`cargo check -p astraweave-localsim` after each edit. Clippy `-D warnings`. Unsafe is forbidden. Step 6 updates `docs/architecture/water.md`.

---

## 9. Review questions

1. Confirm the five channels, the phase ids, the flag bits, the 1 m cell, and the eight regions, as in §2.
2. Confirm `FieldWrite` is the only way weaving changes a channel, and that `FieldAgent` leaves `astraweave-weave`.
3. Confirm overlap: the lower slot owns the world cell.
4. Confirm hazard order: nearest to the companion, ties `z`, `y`, `x`.
5. Confirm tag bits 0–4 as in §2, and that flag bits 5–7 stay zero.
6. Confirm `density_delta` is kg/m³ added to `density0`, saturating at ±127.
7. Confirm #252's `weaving` hash stores `field_digest` rather than a second copy of the arrays.
8. #247 stays with the fluids owner.

---

## 10. Failures this plan refuses to bury

| Item | Owner | Plan |
|---|---|---|
| #247 SSFR `textureLoad` on a depth texture | Fluids, issue already open | `R32Float` plus `texture_2d<f32>`. Not this phase. The `deferred-failure` label is still missing. |
| `PhysicsWorld.wind` write-only | Weaving's translator plus this field | `RedirectWind` becomes a zone axis. The Rapier `f32` is not given a force. |
| `fluids.md` §1 and the `EnvironmentManager` crate comment | The commit that edits those files | Correct the prose. |
| Two grids, if #252 keeps `FieldAgent` | This schema | Phase 2 does not start the solver until the comment thread on #252 accepts §9.2. |
