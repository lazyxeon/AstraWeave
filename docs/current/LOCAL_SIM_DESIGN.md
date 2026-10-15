# Local Simulation Design

**Status:** Phase 1, waiting for review. No crate code in this change.
**Base:** `main` `85d09a2c`. Survey: [`LOCAL_SIM_ARCHAEOLOGY.md`](LOCAL_SIM_ARCHAEOLOGY.md).
**Decision:** a new crate, `astraweave-localsim`, owns a sparse integer cellular grid. Active chunks tick. Sleeping chunks persist as run-length bytes. GPU fluids and the Gerstner surface stay presentation.

Phase 2 does not start until this document is approved.

---

## 1. Decision

The hashed sim is a 3D material grid with integer fill, heat, and a small per-cell velocity. Updates are a fixed number of ordered passes over a double buffer. Chunks outside the activation map do not tick. Reactions are a data table. Weaves enqueue modifier records. The grid publishes water columns through `WaterQuery` and hazard samples through a separate query. Structural breakage is a queue the destruction crate consumes.

### Options set aside

**(a) Extend `FluidSystem` and hash the GPU particles.**
`step` already exists, including a temperature field and a Boussinesq term (`shaders/fluid.wgsl` 136–146). The iteration count is adaptive and the neighbor list is built with atomics (`lib.rs` 1160–1167). The water crate's determinism carve-out excludes that state from replay. Making it bit-exact would mean replacing the solver. The W-series already measured this path as the accent layer, not the world sim.

**(b) Restore the deleted voxel water grid.**
`volume_grid.rs` left the tree in W.1. The recovery tag is local-only. That grid was `f32` water, not gas or fire, and it was removed because it could not carry general water on the minimum spec. Putting it back forks the decision the water successor already made.

**(c) Lattice Boltzmann as the world model.**
D3Q19 keeps 19 distributions per cell. The collide step is local, which is attractive, and it is also a float relaxation. An integer lattice exists in the literature and is a poor fit for a reaction table of named materials (oil, poison, steam, ice). Memory and bandwidth lose to a cell that is 8 bytes.

**(d) PIC, FLIP, or MPM as the world model.**
Zhu and Bridson FLIP, and Stomakhin-style MPM, are the right tools for a hero splash or for snow that should keep volume. They store particles, transfer to a grid, and transfer back. Particle order is a hash hazard. The cost sits in the transfers. The frame budget below does not fit an open-world particle fluid beside rendering. The existing GPU PBD path remains the splash visual, fed by the CPU accent upload that `weaving_playground` already performs.

**(e) Semi-Lagrangian stable fluids as the hashed state.**
Stam, *Stable Fluids*, SIGGRAPH 1999: advect, diffuse, project. The semi-Lagrangian trace samples with interpolation. That interpolation is where libm and FMA diverge across the three OS runners. A donor-cell (upwind) subset with a fixed Jacobi count is deterministic, and it is more machine than the gameplay references actually use. The grid below keeps two donor-cell passes for chunks that have a flow flag, and it does not run a Poisson solve.

**(f) Divinity surfaces alone, or a BotW fuel timer alone.**
Both are the right gameplay shape and both are too thin for gas in a dug tunnel. They become the reaction table and the wind bias inside the grid, not a second 2D map.

### Why this shape

| Pressure | What it forces |
|---|---|
| Noita | One material per cell. Liquids fall, gases rise, fire is a material that consumes fuel. The lesson is the shared grid, not their per-pixel random update order. |
| BotW | Fire spreads on fuel. Wind is a direction that biases the spread. Regional temperature is not a PDE. The wind modifier is a vector, quantized once. |
| Divinity | Surfaces and clouds are a reaction matrix: water against heat, oil against heat, a conductive liquid against shock. The matrix is data. |
| Teardown | Fire lives on the voxel material and removal of that material is what drops the structure. Heat crosses into #243 as a command. This crate does not own bonds. |
| Determinism | Cell bytes are integers. Pass counts are constants. Chunk order is coordinate order. Parallel jobs write only their own next buffer. |
| §7.7 | `WaterQuery` stays the only water question. A new backend publishes columns. Physics does not grow a private water type, and this crate does not sample Rapier. |
| Frame budget | The 11 ms figure is the whole frame on a GTX 1660 Ti-class machine. The sim is CPU. Its cap is 24 active chunks. A visit counter enforces the cap. Wall-clock timing is not a hash input. |
| #244 | Swim keeps `WaterPlayerState` and reads depth from `WaterQuery`. |
| #243 | Solidity and structural material ids are read. Heat that should break a voxel is queued. |

### What Phase 2 will not do

- Hash `FluidSystem` particles, SSFR textures, Gerstner displacements, or accent billboards.
- Restore `volume_grid.rs` or the SPH modules W.1 removed.
- Run LBM, FLIP, PIC, or MPM in the tick.
- Allocate in `tick`.
- Put `f32`, `HashMap`, `Instant`, or `thread_rng` in the sim modules.
- Apply `PhysicsWorld.wind` as a Rapier force.
- Edit `WorldSnapshot` or the `perception_world` bytes.
- Retune swim thresholds or write a character motor.
- Break a destruction bond, fracture a mesh, or write the `i8` overlay.
- Fix the lavapipe SSFR compile. That remains #247.
- Depend on `Fixed64`. PR #245 is the home for that type if a later solve needs it.

---

## 2. Research notes

The references agree on the gameplay model and disagree on the numerical model. The gameplay model is the one that couples to magic.

**Noita (Falling Everything).** The world is a grid of materials. A liquid cell moves into an empty neighbor, a gas cell moves upward, and fire converts a flammable neighbor and leaves smoke. Update order in that game is shuffled so streaks do not favor one screen direction. A shuffle is allowed here only as a `SimRng` stream consumed in cell-index order, and Phase 2 does not shuffle: the scan is index order, which is enough for replay and is easier to test. Inactive chunks are how a large world stays cheap. That is the activation map.

**Breath of the Wild.** Fire on grass is a fuel timer plus neighbor spread, biased by the wind the weather system already has. The cold and heat gauges are regions, not cells. The lesson for Aravath is that a weave which redirects wind must change the spread bias, and a region ambient can be a modifier rather than a solved temperature field.

**Divinity: Original Sin.** The ground carries a surface, and the air above it carries a cloud. A small matrix turns water into steam, oil into a burning surface, and a wet surface into a shock hazard. The step is discrete because the game is turn-based. A fixed tick is the real-time version of that discreteness. Phase 2 ships the matrix as rows, so a new pair is data.

**Teardown.** Voxels have materials. Fire spreads to flammable neighbors and removes matter. The support solver then drops what is no longer held. The split matches #243: this crate spreads heat and names the material, and destruction decides the collapse.

**Stable fluids.** Stam's method is unconditionally stable because semi-Lagrangian advection cannot blow up. The same trace-back is the part that is not bit-stable across libm. Donor-cell advection on integer velocities is the piece worth keeping, and only on chunks marked as flowing. A pressure projection with a data-dependent iteration count is the piece the adaptive GPU solver already got wrong. It stays out.

**Lattice Boltzmann.** Streaming is a copy along a fixed set of directions, and collision is local, so a double buffer maps cleanly onto jobs. The distribution set is the cost. D3Q19 at the active-cap size below is about 7.6 MB of distributions before the rest of the cell, and the BGK relaxation is a float multiply. Integer LBM would be a different project. It does not give oil, poison, and ice as readable materials.

**PIC / FLIP / MPM.** Particles carry what the grid averages away. FLIP adds the grid's velocity change back onto the particle so the liquid does not look like honey. MPM treats the particle as a material sample, which is why it couples to soft solids. Both need a sort or an atomic scatter to build the grid, which is the same class of non-determinism as `fluid.wgsl`'s neighbor list. They remain available as a GPU visual of a region whose integer outcome is already decided. They are not the outcome.

---

## 3. Numeric contract

Sim modules store and update integers only.

```text
cell size     = 500 mm (0.5 m)
chunk         = 16 x 16 x 16 cells (8 m)
Cell          = 8 bytes
  material    : u8
  fill        : u8          // 0 empty, 255 full
  flags       : u8          // FLOW, HOT, SOLID_OVERRIDE, SLEEPING bit lives on the chunk
  heat        : i16         // 0 = ambient, negative = cold
  vel         : [i8; 3]     // cells per tick, saturated
```

`surface_height` on `WaterSample` is the only float the grid produces. It is `cell_y * 500` millimetres converted at the query edge. Multiples of 0.5 are exact in `f32`. Density is a `u16` in kilograms per cubic metre, widened to `f32` at the same edge. `1000` is exact. Those two floats are not stored in the chunk and are not hash inputs. The hash inputs are the integers those floats are derived from.

Passes per active chunk, every tick, counts fixed:

| Pass | Count | Who runs it |
|---|---|---|
| Liquid settle | 1 | Chunks with any liquid fill |
| Gas rise | 1 | Chunks with any gas fill |
| Heat conduction | 4 | Chunks with `HOT` or a heat modifier |
| Donor-cell flow | 2 | Chunks with `FLOW` |
| Reactions | 1 | Every active chunk, after the passes above |

A chunk with no fill, no heat, and no modifier still occupies an active slot if an observer covers it, and its tick is a flag check. There is no "until the residual is small" loop.

`SimRng` is used when a reaction row says `chance`. Draws happen in cell-index order, one stream per chunk:

```text
SIM_SYSTEM_LOCAL_SIM = 0x51A0_7001
```

Checked against `main` (`0xC2AF_*`, `0xAE11_*`, `0x7E22_7001`) and against #243's `0xD357_7001` and #212's `0xC0BA_7001`. If #244 assigns `SIM_SYSTEM_TRAVERSAL` to this value, Phase 2 picks a new constant before the first commit. The stream is `SimRng::new(world_seed, tick, SIM_SYSTEM_LOCAL_SIM ^ chunk_id.rotate_left(17))`. One constructor per chunk per tick. No `thread_rng`.

Preferred reactions in the demo table use thresholds (heat greater than ignition) so the tick does not draw at all. The stream exists so a row can opt in without a second RNG.

60 Hz. `dt` is not a float in the crate. Callers who live in `f32` time quantize once at the edge and drop the float.

---

## 4. Activation map

```rust
pub struct ActivationMap {
    pub slot_bits: [u64; 2],       // 128 resident slots
    pub active: [u16; MAX_ACTIVE], // sorted slot indices, MAX_ACTIVE = 24
    pub active_len: u16,
}
```

The pool is 128 chunks, allocated when the world is created. `tick` does not allocate. A slot is a candidate when any of these hold:

1. Its AABB intersects an observer sphere. Observers are players and AI entities, passed in already sorted by entity id. Radius is 16 m (two chunks).
2. A modifier overlaps it.
3. It has `HOT`, `FLOW`, or non-zero fill.
4. It is inside `SLEEP_GRACE` (30 ticks) after the last observer and the last reaction.

If more than 24 candidates survive, the keep order is: overlaps a player, then `HOT`, then lowest chunk coordinate. The dropped tail is deterministic. A burning chunk that loses the cap stays resident and sleeping, so the fire pauses instead of vanishing, and the hash records the pause.

Sleep writes a run-length encoding of `(material, fill, heat)` into a preallocated buffer on the slot. Velocity is dropped on sleep (stored as zero). If the encoding would exceed the buffer, the slot keeps the raw 8-byte cells. Both forms are hashed. A query against a sleeping slot reads the encoding. A pool the player walked away from still answers `WaterQuery`.

Wake expands the encoding back to cells and sets the slot active. Unload of a slot that is sleeping writes the same bytes a later load reads. The test is byte equality, matching #243's stream test.

The map is rebuilt every tick from the observer slice and the flags. It is not a GPU structure.

---

## 5. Tick and jobs

```text
tick(observers, modifiers):
  rebuild ActivationMap
  for each active slot, in slot order:
      copy the six neighbor faces into that slot's halo (read-only)
  parallel:
      each active slot reads its cells and its halo
      writes its private next buffer
      appends outgoing mass and heat to its private flux records
  join
  serial, slot order:
      apply incoming flux with saturating integer add
      swap the next buffer in
  serial, modifier id order:
      apply modifiers
  serial:
      reactions
      rebuild flags
      sleep or wake
      publish ColumnWater rows and HazardSample rows
```

Jobs never write a neighbor's cells. The halo is a copy taken before the parallel section. Flux records are per slot, so the parallel section has no shared mutable state and no atomics. The serial flux apply is ordered, which is what keeps a sum independent of the thread count. Integer add is not the hazard. The hazard is applying the same flux twice, and the single serial pass prevents that.

`rayon` is the job runner. It is already a workspace dependency. The sim result does not depend on the pool size. A test runs the same script on one thread and on the default pool and compares the hash.

No `Mutex` on a cell. No lock on the flux records.

---

## 6. Materials and reactions

```rust
pub struct SimMaterial {
    pub id: u16,             // same id space as #243's material id
    pub kind: MaterialKind,  // Empty, Liquid, Gas, Solid, Fire
    pub density: u16,        // kg/m^3, published through WaterQuery for liquids
    pub ignition: i16,
    pub fuel: u16,
    pub conductivity: u8,
    pub flammable: bool,
}

pub struct Reaction {
    pub a: u8,
    pub b: u8,
    pub min_heat: i16,
    pub out_a: u8,
    pub out_b: u8,
    pub heat_delta: i16,
}
```

The first table is static data in the crate, not a script:

| Pair | Result |
|---|---|
| Water + heat at the boil line | Steam, heat drops |
| Water + cold at the freeze line | Ice, `SOLID_OVERRIDE` |
| Oil + heat at ignition | Fire |
| Fuel solid + fire neighbor | Fire, fuel decrements |
| Fire + fuel 0 | Smoke |
| Smoke, steam | Gas, rises |
| Poison gas | Gas, rises, hazard bit |
| Ice + heat above the melt line | Water, override clears |

`SimMaterial.id` joins #243's material id. This crate does not add fields to #243's struct. Stone is non-flammable and high conductivity. Wood is flammable. Air is empty. Until #243 is on the branch, tests use this static table and a `FlatSolid` floor.

---

## 7. Coupling

### 7.1 Destruction voxels and materials (#243)

```rust
pub trait SolidField {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool;
    fn material_id(&self, x: i32, y: i32, z: i32) -> u16;
}
```

The Phase 2 adapter reads #243's overlay (`i8`, 0 empty, 127 solid) and `column_material`. It does not write them. A liquid cell treats `solid == true` as occupied. When a later brush clears the solid, the next liquid pass can fall into that cell. When a later brush fills a cell the sim occupies, the liquid pass displaces upward in index order.

```rust
pub struct HeatBreach {
    pub tick: u64,
    pub seq: u32,
    pub material_id: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub heat: i16,
}
```

A fixed array of 32 breaches per tick. Overflow keeps the 32 lowest coordinates and counts the drop in the hash. Destruction's owner turns a breach into their command log. This crate does not call `break_destructible` and does not allocate debris.

`FlatSolid` is the test double until the adapter compiles against #243. Same pattern as that PR's `FlatColumnBase`.

### 7.2 Weave modifiers

```rust
pub struct SimModifier {
    pub id: u32,
    pub kind: ModifierKind,
    pub origin: [i32; 3],
    pub radius: u16,
    pub a: i16,
    pub b: i16,
    pub c: i16,
}

pub enum ModifierKind {
    HeatSource,      // a = heat added per tick
    Wind,            // a, b = dir on xz in milli-units, c = strength
    GravityDelta,    // a = milli-g added to the liquid/gas bias
    MaterialStamp,   // a = material id
    FillStamp,       // a = material id, b = fill
    DensityScale,    // a = density in kg/m^3 for liquids inside the radius
}
```

The sim applies these records. It does not match on `WeaveOpKind`. The gameplay edge compiles the three weather ops:

| Existing op | Modifier |
|---|---|
| `FreezeWater` at `a` | `MaterialStamp` ice, then `HeatSource` held at the freeze line, radius from a quantized `op` extent |
| `RedirectWind` | `Wind` with the direction quantized to milli-units once. `PhysicsWorld.wind` is left as the stored `f32` and is not read. |
| `LowerWater` | `FillStamp` water fill 0 inside the radius. `clear_water` remains the far-field plane clear, and it stops being the only effect. |

`ReinforcePath`, `CollapseBridge`, and `RaisePlatform` stay on the director path. A new elemental pair is a `Reaction` row plus, when a weave should spawn it, a modifier. It is not a new match arm inside `astraweave-localsim`.

The presentation `WeaveInstance` in `astraweave-render` stays a visual of the same event. The playground can keep uploading accent particles. Those particles are not hashed.

### 7.3 Swim (#244) and buoyancy

`astraweave-water` gains a backend, `ColumnWater`: a sorted list of `(x_milli, z_milli, surface_milli, density_u16, ice: bool)`, cap 4096. `sample` binary-searches the column. On a hit, `surface_height` and `density` follow the edge rule in §3. Ice publishes density 0, so the Archimedes term is zero, and `pos.y >= surface_height` keeps a body standing on the sheet from taking a buoyancy impulse. On a miss, the caller uses `AnalyticWater`, which remains the sea plane and the authored AABBs.

The sim fills `ColumnWater` from active cells and from sleeping encodings. Over the cap, rows nearest the first observer survive, ties broken by coordinate.

`PhysicsWorld::apply_buoyancy_forces` samples `ColumnWater` first when one is installed, then the existing facade. Default installation is empty, so current buoyancy tests keep their results. `astraweave-physics` does not depend on `astraweave-localsim`. The column buffer type lives in `astraweave-water`.

#244's phase 4 calls `WaterQuery` for the feet and the head and passes the submersion fraction into `WaterPlayerState::update`. This design does not change that function. A walkable ice sheet is the density-0 column plus the traversal motor treating the surface as ground. The motor stays in #244.

`Aabb.linear_drag` stays authoring data. The grid does not read it.

### 7.4 AI hazards

```rust
pub struct HazardSample {
    pub kind: HazardKind, // Fire, Steam, PoisonGas, DeepWater, Ice
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub heat: i16,
    pub radius_milli: u16,
}
```

`hazard_samples(origin, radius) -> [HazardSample; 16]` returns the nearest samples, sorted by kind then coordinate. The perception system may copy those into its own working set. Phase 2 does not push them into `WorldSnapshot.pois`, because `perception_world` is a required replay case. That push is a review question (§12).

Deep water is a column whose depth exceeds the swim threshold #244 already uses, so the AI and the player share one depth.

---

## 8. Budgets

| Cap | Value | On overflow |
|---|---|---|
| Resident chunks | 128 | The farthest sleeping slot is compacted out. Its bytes are already in the save blob the caller holds. |
| Active chunks | 24 | §4 keep order. |
| Heat passes | 4 | Fixed. |
| Flow passes | 2 | Fixed. |
| Modifiers | 32 | Extra modifiers are rejected at the edge with `Result`. They do not enter the hash. |
| Breaches | 32 | Lowest coordinates kept. Drop count hashed. |
| Column rows | 4096 | Nearest to the first observer. |
| Hazard samples | 16 | Nearest, kind then coordinate. |
| Observer inputs | 64 | Caller truncates by entity id before `tick`. |

The cell visit ceiling is `24 * 4096` checks, and most of those return on the empty-chunk flag. The budget that protects the 11 ms frame is the active cap, not a timer. A criterion bench lands with the crate and stays off pull-request checks, consistent with #223. The bench records the tick on a debug build and on a release build. It is evidence for the review after Phase 2, and it is not a CI gate in the first implementation PR.

Target: the release tick of a full 24-chunk worst case stays inside 1.5 ms on a 6-thread CPU of the same era as a GTX 1660 Ti, leaving the rest of the 11 ms for the renderer, Rapier, and AI. If the bench misses that, the cap drops. The cap does not grow in order to pass a frame-time wish.

---

## 9. Hash, replay, save

`local_sim_hash(tick) -> [u8; 32]` is SHA-256 over:

1. Magic `b"AWLOCL1"` and the tick as `u64` LE.
2. `ActivationMap` bytes.
3. Each resident slot in index order: flags, then either raw cells or the sleep encoding, plus a tag byte that says which.
4. Modifiers in id order.
5. Breaches in the array order.
6. `ColumnWater` rows in the published order.
7. The 16 hazard slots, empty slots included.

`DefaultHasher` is not used. Rapier poses, GPU particles, and the Gerstner surface are not inputs.

Replay case `local_sim`, 300 ticks, headless, no wgpu:

1. Flat floor, a wood cell, a water spill, one ignition, one wind modifier.
2. Tick 100: freeze modifier on the pool.
3. Tick 150: observers leave. The region sleeps.
4. Tick 200: an observer returns. The region wakes.
5. Tick 300: hash.

The case is added to `astraweave-core/tests/replay_hash.rs` and to the `required` set in `determinism-replay.yml`, in the same commit as the sim, so a missing hash fails the three-OS compare. This docs PR does not add it. Rebase keeps `planner_pure` and `perception_world`, and keeps `destruction`, `traversal`, and `combat_resolve` if those have landed.

Save bytes are the sleep encodings plus the live raw slots, bincode, little-endian, no map. The same bytes are the reliable payload a later net change can attach beside `Delta`. Phase 2 does not rewrite `astraweave-net`.

---

## 10. Aravath showcase

After approval, and when `aravath_runtime` from #205 is on the branch:

- A pool the player can wade. Depth comes from `ColumnWater`. The swim motor is #244's, called when that API is present. Until then the showcase still publishes the column and a debug query.
- A CC0 wooden prop (KayKit or Kenney, the pack #205 already vendors, license line in the manifest) that ignites. Heat emits `HeatBreach`. If #243 is present, the prop loses the burned voxel. If it is not, the breach list is still visible in the hash and the prop swaps to a charred mesh through the existing add/remove model API.
- A freeze weave that stamps ice. The column reports density 0. The Gerstner freeze tint may play as a visual and is not hashed.
- A wind weave that leans the fire. The lean is the integer velocity bias, visible as which cells ignite next.
- HUD strings say Aravath.

No second forest pack. No new art beyond CC0 already accepted on #205.

---

## 11. Phase 2 order

Each step is its own commit, with tests, and stops if a pre-existing failure shows up.

1. Crate skeleton, pool, source scan, `SIM_SYSTEM_LOCAL_SIM`.
2. Activation map, sleep encoding, hash, unload/load byte test.
3. Liquid, gas, heat, donor-cell, reactions. Fixed pass counts.
4. Modifiers. Gameplay weather arms enqueue them. `clear_water` stays the plane clear.
5. `ColumnWater` and the buoyancy sample order. Existing buoyancy tests stay green when the column buffer is empty.
6. `HazardSample` query. Snapshot untouched.
7. `SolidField`, `FlatSolid`, `HeatBreach`. Adapter file for #243, compiled when that API is present.
8. Replay case `local_sim` on the three OS workflow.
9. Showcase scene, gated on #205's runtime.

`cargo check -p astraweave-localsim` after each edit. Clippy `-D warnings` on the library. Unsafe is forbidden. The water and physics edits in steps 4 and 5 run `cargo check` and the existing tests of those crates.

Traces: step 5 updates `docs/architecture/water.md` (§5, §8, revision history) because `ColumnWater` is a new backend. Step 4 updates `docs/architecture/gameplay.md` for the weather arms. `fluids.md` §1 is corrected in any commit that touches `astraweave-fluids`. The current plan does not need that commit. The physics trace gets a sentence on the column sample order in step 5.

---

## 12. Review questions

1. Confirm integer cells, with `f32` only at the `WaterQuery` edge. #244's motor can stay `f32` plus `libm`. This crate will not take `Fixed64` from #245.
2. Confirm `ColumnWater` in `astraweave-water` so physics does not depend on the sim crate.
3. Confirm `HeatBreach` is the only write toward #243, and that destruction applies it on their side.
4. Confirm hazard samples stay off `WorldSnapshot` until a review accepts a new `perception_world` hash.
5. Confirm the active cap: 24 chunks, 16³ cells, 0.5 m, 16 m observer radius, 30-tick sleep grace.
6. Confirm #247 stays with the fluids owner and is not a Phase 2 task of this design. The issue needs the `deferred-failure` label; this PR cannot add it.

---

## 13. Failures this plan refuses to bury

| Item | Owner | Plan |
|---|---|---|
| #247 SSFR `textureLoad` on a depth texture | Fluids, already written on the issue | `R32Float` plus `texture_2d<f32>`. Not this phase. Label `deferred-failure` is missing and should be added by someone who can edit issue labels. |
| `PhysicsWorld.wind` write-only | This design, Phase 2 step 4 | Wind becomes a `SimModifier`. The Rapier field is not given a force, because that force would be an `f32` outside the hash. |
| `fluids.md` §1 and the `EnvironmentManager` crate comment | The next commit that edits those files | Correct the prose so it matches the tree. Not a silent behavior change. |
| `add_buoyancy` has no gameplay caller | #244 and this design together | Swim uses the character motor. Buoyancy remains for rigid bodies the demo registers. An empty `buoyancy_bodies` map stays valid. |
