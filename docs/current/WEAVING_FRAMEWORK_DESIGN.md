# Weaving framework design

**Status:** approved with the ownership changes below. Phase 2 implements them on `feat/weaving-framework`.

Approved deltas:

- The field solver stays on `feat/local-sim` (`astraweave-localsim`). Weaving writes `SimModifier` records. `TestSimField` is the stand-in until that crate exists.
- One crate: `astraweave-weaving`. No `astraweave-weave`.
- `WorldSnapshot` has a 16-slot hazard tail (PR #251 coordination).
- Freeze emits a static AABB, a `NavChange`, and the ice flag.
- `queue_weave` matches the shape #214 will call. Full sandbox registration waits on that merge.
- Integer milli-units until `Fixed64` lands in `astraweave-math` from the destruction work.
**Reads:** [`WEAVING_ARCHAEOLOGY.md`](WEAVING_ARCHAEOLOGY.md), `main` at `85d09a2c`.
**Does not merge.** Phase 2 starts when the questions in §14 are answered.

A weave is a data record that modifies world state. Heat, cold, force, density, decay, and gravity are rows in that record. They are not new match arms on `WeaveOpKind`.

## 1. Decision

`astraweave-weaving` owns the modifier graph, the template table, and the casts. It emits `FieldWrite` (including `AddVolume` and `SetWind`) through `SimField`. It does not store regions or a cell window. `NullField` returns a zero `field_digest` until `LocalSim` is on this branch; then `apply_writes` feeds `LocalSim::tick` and the replay stores `LocalSim::field_digest`. `TestSimField` is `cfg(test)` and records writes only.

The crate depends on `astraweave-core` for `SimRng` and `Entity`. It does not depend on physics, gameplay, render, fluids, combat, or destruction. Those crates do not depend on it in Phase 2 either. The host (`build_app`, and later the Aravath call site) drains the sink rings. That is the same one-way shape PR #243 uses so a library with no hooks does not go dormant, and so the dependency graph does not gain a cycle.

### Options

| Option | Why it is not the plan |
|---|---|
| Extend `apply_weave_op` | Six hardcoded arms, a `Vec` per call, a discarded `NavMesh::bake`, `FreezeWater` with no truth. Another arm would be another script event. |
| Extend `astraweave-weaving` | No core or ECS dependency, string intents, two repair paths, two ability enums. The anchor slice stays there. The hot path does not. |
| Drive chemistry from GPU fluids | `docs/architecture/fluids.md` §0: particle state is non-deterministic and excluded from replay. Hashing it is a review reject. |
| Hang zones off `GravityManager` | `HashMap` storage, `f32`, not called from `PhysicsWorld::step`. Iterating it is not a stable order. |
| Hang decay off `DestructionManager` | Tested `f32` module with no production caller. PR #243 already refused to extend it. |
| Let a Rhai script build the graph every tick | PR #214's tick is sandboxed and capped, and it allocates. Composition order would follow script call order. |

### What Phase 2 will not do

- It will not delete the anchor slice, `apply_weave_op`, or `WeaveInstance`. Legacy ops become a translator that enqueues a template id (§3). The translator lands after the evaluator, in its own commit.
- It will not add a fourth damage formula. Ignite and freeze on an actor leave as a sink the combat crate already knows how to resolve (PR #212). `Stats::apply_damage` stays the legacy path.
- It will not climb, mantle, or vault. It writes surface flags. PR #244 reads them.
- It will not fracture a support graph. Decay past a strength threshold emits the destruction command PR #243 already defined.
- It will not edit `aravath_runtime/**` or `assets/aravath/**` on this branch. The showcase is a headless scene in the new crate. The demo call site waits until that runtime is the base, or until review says to stack on PR #205.
- It will not vendor assets. The showcase uses CC0 packs already named by the destruction design (KayKit, Kenney, Poly Haven) and records the license line in the manifest.
- It will not introduce a second `Fixed64`, a second `SimRng`, or a second sim-material strength column.

## 2. What the references are for

These are composition rules, not a port of anyone's content.

**BotW / TotK chemistry** (Takuhiro Dohta, GDC 2017, "Breaking Conventions with The Legend of Zelda: Breath of the Wild"). A physics engine moves things. A chemistry engine changes state. Three rules: an element can change a material, an element can change another element, a material cannot change another material. Wind and electricity are elements, and an element may emit a force. The useful lie is a small table that matches everyday expectation (water puts out fire, metal conducts, wet stone slips). TotK keeps that table and adds a fuse slot; the table is still the interaction, the fuse is a loadout. We take the three rules and the "elements may emit forces" clause. We do not take a per-puzzle script.

**Noita.** Each pixel is a material with tags. Reactions in `materials.xml` name two inputs and write outputs, often by tag (`[water]`, `[corrodible]`). The engine steps cells in a fixed order and reactions can cascade. The failure mode is an unbounded cascade and a GPU-unfriendly random access pattern. We take tag rows and a fixed visit order. We cap the cascade at one reaction per cell per tick (§4).

**Divinity: Original Sin surfaces.** Ground effects live on a coarse mask (about 0.5 m in the shipped game), one surface family per cell, with reactions when a spell or a surface meets another (water + cold → ice, oil + fire → burning). Rendering interpolates the mask; the sim does not. We take one phase per cell at 1 m, and we keep the mask on the CPU. The mesh deformation stays in `WeaveInstance`, which is already presentation.

**Magicka.** A spell is an ordered queue of up to five elements plus a cast shape. Some pairs cancel, some combine, the shape decides beam versus area versus self. Freeform queues are easy to mistype and hard to replay if the combination table is implicit. We take the queue and the shape. The legal queues are a closed template table, patched by mods as data, so the tick looks up a precomputed node order (§3).

**Immersive sims** (Looking Glass, System Shock, Deus Ex, Prey). A verb is generic (heat, force, open, decay). An object publishes properties. The player and the AI call the same verb on the same properties, so a hazard the player can ignite is a hazard an agent can flee or exploit. We take that sharing rule: the field written in `simulation` is the field perception copies. There is no player-only spell path.

## 3. Data model

### Identifiers

All ids are `u16` or `u64` assigned at load. Display names live in the data file and are not hashed.

```rust
pub const SIM_SYSTEM_WEAVE: u64 = 0x57EA_7001;
```

Checked against `main` and the open PRs: craft `0xC2AF_7001`, harvest `0xC2AF_7002`, anchor particles `0xAE11_7001`, patrol `0xAE11_7002`, spawner `0xAE11_7003`, terrain solver `0x7E22_7001`, combat `0xC0BA_7001` (#212), destruction `0xD357_7001` (#243), mods `0x4D4F_4453` (#214). No collision.

`weave_rng(world_seed, tick)` is `SimRng::new(world_seed, tick, SIM_SYSTEM_WEAVE)` from `astraweave-core`. The weaving crate's copy of `SimRng` stays on the anchor-particle path until a later cleanup. This crate does not construct `StdRng` itself.

Draws inside a tick are a fixed count per command, taken after the command sort. A tick that emits no random command consumes zero draws. The next tick reseeds, so a skipped draw cannot leak.

### Channels

The field is integer. Phase 2 does not wait on `Fixed64` for temperature or density. Impulses that cross into destruction or combat are `i32` milli-units in the sink. The consumer converts to `Fixed64` when that type is the one shared type (PR #243's gate). Until then the sink stores the milli-units and the weave hash is those integers.

| Channel | Type | Unit |
|---|---|---|
| `temp` | `i16` | deci-degrees, 0 = freezing point of the water row |
| `phase` | `u8` | 0 dry, 1 wet, 2 ice, 3 burn, 4 gas, 5 oil |
| `density_delta` | `i8` | signed steps, 0 = the material's base density |
| `flags` | `u8` | bit 0 wet-surface, 1 ice-surface, 2 burning, 3 slippery, 4 conductive |
| `gravity` | `u16` | 0 = world default, else an index into the zone table |

Gravity presets are an enum stored in the zone table: world-down, zero, ±X, ±Y, ±Z, and a point attractor (center cell, radius in cells, strength in milli-g). No `Vec3` in the sim.

### Graph

A template is a DAG loaded from RON.

```rust
pub struct WeaveTemplate {
    pub id: u16,
    pub nodes: [Node; MAX_NODES_PER_TEMPLATE], // 16
    pub node_count: u8,
    pub order: [u16; MAX_NODES_PER_TEMPLATE],  // topological permutation, computed at load
}

pub struct Node {
    pub kind: NodeKind,
    pub element: u16,     // 0 = none
    pub shape: ShapeId,   // point, sphere, segment, column
    pub radius: u8,       // cells
    pub magnitude: i16,   // channel units per tick
    pub flags: u16,       // Stack, Cancel, EmitForce
}
```

`NodeKind` is `Source`, `Shape`, `Add`, `Max`, `Mask`, `Cancel`, `Reaction`, `Sink`. The permutation is stored beside the template. The tick walks `order[0..node_count]`. It does not topo-sort.

A cast is a command, not a graph allocation:

```rust
pub struct WeaveCommand {
    pub tick: u64,
    pub seq: u32,
    pub caster: u64,
    pub template: u16,
    pub origin: [i32; 3],
}
```

Legal Magicka-style queues are templates. A queue that is not in the table fails validation at load. Mods append templates with new ids (§8). They do not assemble nodes at cast time.

Caps, fixed so a late machine cannot do extra work:

| Cap | Value |
|---|---|
| `MAX_TEMPLATES` | 256 |
| `MAX_NODES_PER_TEMPLATE` | 16 |
| `MAX_COMMANDS_PER_TICK` | 64 |
| `MAX_REGIONS` | 8 |
| `REGION_EDGE` | 16 cells (1 m), so 4096 cells, allocated with the world |
| `REACTION_ITERS` | 4 neighbor passes, always |
| `MAX_REACTIONS_PER_CELL` | 1 |
| `MAX_SINKS_PER_TICK` | 128 |
| `MAX_HAZARDS` | 16 |

### Material rows

One table, keyed by the `u16` material id terrain and destruction already speak. Weaving owns the elemental columns. Destruction owns strength, debris class, and audio tag (PR #243 §5). The same id is the join. Weaving does not copy strength.

| Column | Owner | Examples |
|---|---|---|
| `tags` | weave | Flammable, Conductive, Structural, Freezable, Wettable |
| `ignite` | weave | temp at which Flammable enters `burn` |
| `freeze` | weave | temp at which a wet cell enters `ice` |
| `density0` | weave | base density step the delta is applied to |
| `strength` | destruction | compared when a decay sink is emitted |
| `debris`, `audio` | destruction | unchanged from #243 |

`astraweave-materials` stays the PBR baker. This table is not a `MaterialPackage`.

### Field agent

This is the local simulation agent. It did not exist on `main` (§7 of the archaeology). It is a pool of regions inside `astraweave-weave`, not a new fluids solver.

```rust
pub struct FieldAgent {
    pub regions: [Region; MAX_REGIONS],
    pub occupied: u8,
}

pub struct Region {
    pub origin: [i32; 3],   // cell of the (-8,-8,-8) corner
    pub subject: u64,       // player or AI entity, 0 = empty
    pub temp: [i16; 4096],
    pub phase: [u8; 4096],
    pub density_delta: [i8; 4096],
    pub flags: [u8; 4096],
    pub gravity: [u16; 4096],
}
```

Regions follow the player and up to seven AI entities, quantized to the cell, sorted by `subject` id. An entity that already has a region keeps it (the slot is stable). A new subject takes the empty slot with the lowest index. When all eight are full, the highest `subject` id that is not the player loses the slot and its cells clear on that tick, in index order. The player always occupies slot 0.

Cells outside a region are not simulated. Entering a region starts from the material row's base state (dry, `density_delta` 0, world gravity), then replays the command log's overlapping tails the way destruction replays a chunk delta. The replay is bounded by commands whose origin sits in the region and whose age is under `MAX_COMMANDS` retained (4096). Older commands have already been baked into a region that was occupied; a region that was dropped and reacquired re-simulates from the retained log only. That bound is part of the hash.

### Legacy translator

`WeaveOpKind` maps onto template ids, one row, fixed:

| Legacy op | Template | Sink |
|---|---|---|
| `ReinforcePath`, `RaisePlatform` | density up, small rect | density channel, and a destruction brush only if the host has drained brushes |
| `CollapseBridge` | decay along the segment | destruction `Impact` when the accumulated decay exceeds strength |
| `RedirectWind` | force, direction from the preset nearest the vector | zone table. The global `set_wind` write remains until the translator commit, then the zone is the truth |
| `LowerWater` | phase to dry in the targeted volumes | water facade `clear` stays a sink, not a special case inside the evaluator |
| `FreezeWater` | cold into wet cells | phase `ice`, flag ice-surface. This is the truth `FreezeWater` does not have today |

`WeaveInstance` is filled from the presentation sink (phase and temp), still not hashed.

## 4. Composition rules

Load time rejects a template whose edges cycle, whose id collides, or whose reaction row names an unknown element or tag.

Tick time, after the command sort `(tick, seq, caster, template)`:

1. **Elements write materials. Materials do not write materials.** A `Structural` cell that is burning emits a fire element into its node output. The neighbor changes because that element is written on the next neighbor pass. The material row has no target material id.
2. **Elements write elements.** Water (phase wet, or a water element source) into a burning cell clears `burn` and sets `wet`. Heat into a wet cell raises `temp`. At `freeze`, wet becomes ice. At `ignite` on a `Flammable` tag, dry or oil becomes burn. Oil plus burn stays burn and raises temp by the row's magnitude. These are table rows, first match on `(element_id, tag_bits, priority)` with rows sorted by that key.
3. **Cancel is integer annihilation.** A cell that receives heat and cold in the same tick subtracts the smaller magnitude from the larger. The remainder keeps the winner's sign. No epsilon.
4. **Overlap is `Max` unless the node sets `Stack`.** Stack adds and then clamps to the row's cap. Two fireballs do not silently multiply.
5. **One reaction per cell per tick.** Further candidates stay pending in a fixed queue of 8 per region and are dropped in cell-index order past that. Four neighbor passes run even when a pass writes nothing.
6. **Visit order** is region slot, then `z`, `y`, `x`. Neighbors are ±X, ±Y, ±Z, that axis order.
7. **Forces** (`EmitForce`) append a sink. They do not move a body inside the evaluator.
8. **Conductive** spreads a shock element along conductive tags for one pass, same visit order, and then emits one combat sink per shocked actor id (sorted). It does not recurse.

Random cell noise, if a template asks for it, is one `unit_f32` draw converted to a deci-degree with a documented scale, taken from `weave_rng` in command order. Default templates draw nothing.

## 5. Scheduling

The ECS order on `main` stays. Weaving does not insert a stage.

| Stage | Weave work |
|---|---|
| `pre_simulation` | Age cooldowns by one tick. Clear the scratch arrays. Copy the committed hazard list into the perception buffer. |
| `perception` | `build_snapshot` reads that buffer. It does not read a half-updated field. |
| `simulation` | Producers enqueue commands: player cast, mod `queue_weave`, AI request from the previous plan. Then `weave_eval` sorts, evaluates, writes the field, and fills sink rings. |
| `sync` | Optional compact copy for the legacy `World`, registration-ordered after the existing bridge. Phase 2 ships the copy only if a legacy caller still reads `WeaveConsequence`. |
| `ai_planning` | Planners read the snapshot from this tick's perception, which is last tick's field. They may enqueue commands for the next `weave_eval`. They do not write channels. |
| `physics` | Samples gravity and the ice/wet flags through a function the host passes in. Rapier stays `f32`. The conversion is outside the weave hash. |
| `post_simulation` | Host drains sinks into destruction and combat, in ring order. |
| `presentation` | `WeaveInstance` and fluid accents. Excluded from the hash. |

`weave_eval` is one `fn(&mut World)` registered on `simulation`. Its place in that stage is "after command producers, before any reader of the field." `build_app` is the registration site, with a test that the stage sequence contains `weave_eval` in that position. Systems already in the stage keep their relative order.

The tick is a tick. Channel math does not take a `dt`. A caller that feeds a variable frame delta does not change the field.

One tick of lag for AI and physics is the contract. Perception runs before simulation, so both see the same committed field. Closing the lag by moving perception would desynchronize every other snapshot consumer. We do not do that.

## 6. No allocations, ECS layout

Allocated when the world is created, and never again inside `weave_eval`:

- the region pool and its SoA channels
- the template table and permutations
- the command ring, the sink ring, the hazard array, the per-region pending-reaction queue
- a scratch `[i16; 4096]` for the neighbor pass

`weave_eval` does not call `Vec`, `Box`, `HashMap`, `format`, or `String`. A `CountingAlloc` test, the same shape as `astraweave-combat/tests/no_alloc.rs` on #212, wraps one tick and asserts zero bytes. Load and mod patch apply outside the tick and may allocate; they swap the template table in at a tick boundary by copying into the preallocated slots.

Components, registered with `register_component` so they sit in a `BlobVec`:

```rust
pub struct CWeaveCaster {
    pub cooldown_until: u64,
    pub last_template: u16,
}
```

`CWeaveAgent` in `astraweave-weaving` stays the stringly scan component. It is not this component.

Zone records are a sorted array of 32, not a `HashMap`. Sample order is highest priority, then lowest zone id.

## 7. Determinism and the replay case

Hash, little-endian, SHA-256:

- the command log for the case
- every occupied region's five channel arrays, slot order
- the sink ring
- the hazard array and its count

Excluded: Rapier poses, GPU particles, `WeaveInstance` bytes, display meshes, the legacy `f32` wind vector.

Case name: `weaving`. 300 ticks at a fixed 60 Hz step, world seed `0x57EA_5EED`. The script, as commands:

1. Tick 1: heat template on a flammable cell.
2. Tick 40: the burn has spread to a neighbor (asserted, then hashed).
3. Tick 80: cold template on a wet cell. Phase becomes ice.
4. Tick 120: density-down template on a structural cell. A destruction sink is in the ring.
5. Tick 160: gravity-zero template. The zone table records zero-g.
6. Tick 200: an AI hazard record names the burning cell, and the scripted planner enqueues a step-off command.

The case is added to `astraweave-core/tests/replay_hash.rs` as a dev-dependency on `astraweave-weave`, and to the `required` list in `.github/workflows/determinism-replay.yml`, which already runs on Ubuntu, Windows, and macOS. Cargo allows that dev-dependency edge. The function is `fn weaving_bytes(world_seed: u64) -> Vec<u8>`.

GPU work does not write the field. Fluids may color a particle from a temperature sample copied out after the hash.

## 8. Integration points

### Destruction (PR #243)

Weaving emits `Impact` or `Brush` into a ring the host drains with #243's `DestructionCommand`. Decay maps to `Impact` with the milli-unit impulse. A density carve maps to `Brush` and still has to pass `MAX_BRUSH_CELLS`. Weaving does not add a command variant. `PieceFlags` stay destruction's. `ANTI_DESTROY` still records the command as applied-and-inert on the destruction side.

If #243 has not merged when Phase 2 starts, the ring still fills and the weave hash includes it. A test drains the ring into a fake consumer that checks order. The real enqueue lands in the commit that stacks on `feat/destruction`.

### Combat (PR #212)

A sink `Hit { target, school, milli }` becomes one `resolve_hit` event. School ids are the existing set: Fire, Frost, Shock, and the others already in `DamageType`. One school per event. Shock on a conductive actor is a second event with a later `seq`, not a second formula. Weaving does not import `astraweave-combat`. `Chill.slow` is not revived; a slow is the ice-surface flag plus a combat school, and traversal owns the speed change.

Legacy `Stats::apply_damage` is not called by the new crate.

### Traversal (PR #244)

Flags written per cell: wet, ice, burning, slippery. The traversal plan already multiplies climb drain by wetness. That multiplier reads these flags. Gravity-zero and gravity-axis are samples the motor takes at the capsule origin cell. Weaving does not implement mode C climbing. Ice is a flag, not a new collider type; a walkable ice body is a destruction or physics consumer of the ice flag, and it is a review question (§14) whether Phase 2's freeze also emits a static AABB.

### AI

`WorldSnapshot` gains a fixed tail:

```rust
pub struct WeaveHazard {
    pub cell: [i32; 3],
    pub phase: u8,
    pub temp: i16,
    pub flags: u8,
}

pub hazards: [WeaveHazard; 16],
pub hazard_count: u8,
```

Filled in perception from the committed field, sorted by cell `z, y, x`, nearest occupied cells to the companion first, truncated at 16. The companion's planner treats `burn` and `shock` as costs and ice as a slower cell. The replay's step-off is a deterministic utility pick over that array, not an LLM call. `astraweave-ai` does not depend on `astraweave-weave`; it reads the snapshot fields only.

This is a public snapshot change. It is in the review questions because every planner test constructs a `WorldSnapshot`.

### Mods (PR #214)

Weaves are data. A mod ships `weaves/<id>.ron` templates and reaction-row patches. Load sorts patches by mod id, the same order #214 already uses for manifests. A patch may add a template id or replace a row with the same id. It may not reuse an engine id, exceed the caps, or add a node kind.

The Rhai allowlist gains one command, `queue_weave`, on capability `Weave`, and `MOD_COMMAND_API_VERSION` bumps to 2. The command takes a template id, an origin, and nothing else. Unknown ids fail the script call and enqueue nothing. Rolls stay on `sim_roll`. A new ceiling `max_weaves` defaults to 64 and does not spend `terrain_edits`. Sinks that become brushes still face destruction's cap, so a mod cannot rename a dig to get around it.

Phase 2 stacks this on #214 or lands the data loader with a local ceiling if that PR is still open. The loader does not fork the sandbox.

### Local simulation agent

The `FieldAgent` in §3 is that agent. Region activation is the activation map: a bit per slot plus the subject id. Gas is phase `4`, moved by the same four neighbor passes (a gas cell steps one cell along +Y per pass, index order). It is not a particle. GPU fluids read the phase and temp for accents after the tick.

### Render and water

`WeaveInstance` stays the deformation for part / raise / freeze. The playground's accent producer reads the presentation sink. `AnalyticWater` remains the buoyancy truth; a freeze sink tells the host to mark the overlapping water volume solid for queries. That host write is the commit that replaces the empty `FreezeWater` arm.

## 9. Aravath showcase

Phase 2, after approval. A headless scene in `astraweave-weave` runs the same script as the replay case, so the demo and the hash cannot drift.

When the Aravath runtime is on the branch:

- A CC0 prop with the `Flammable` and `Structural` tags stands in a region. The heat cast ignites it. The cold cast freezes a wet patch. The density cast sinks a platform. The gravity cast lifts the loose piece's expected motion (the piece itself is destruction's debris once #243 is wired; until then the sink is visible in the log and the prop's render offset follows the zone for the camera only).
- HUD and prompt strings say Aravath.
- The asset manifest names the pack, the author, and CC0.
- The demo's Rapier body is not in the weave hash. The showcase assertion is the hash plus a visible state change.

## 10. Tests

| Test | Asserts |
|---|---|
| Command sort | Same multiset, different insert order, same hash |
| Closed table | An unknown template id enqueues nothing and does not change the hash |
| BotW rules | Fire on flammable burns; water clears burn; a material row cannot name another material |
| Cancel | Equal heat and cold leave temp unchanged; unequal leaves the remainder |
| Max versus stack | Two sources without `Stack` take the max; with `Stack` they add and clamp |
| Cascade cap | A loop of reactions advances one cell per tick and stops at one reaction per cell |
| Visit order | Two regions, same data, same channel bytes |
| Region eviction | The ninth subject drops the highest id; the player keeps slot 0 |
| No alloc | `CountingAlloc` around `weave_eval` reports 0 |
| Sinks | Decay emits one `Impact`; conductive shock emits sorted hits |
| AI lag | The snapshot at tick N matches the field at tick N-1 |
| Replay | Case `weaving` matches on Ubuntu, Windows, and macOS |
| Legacy map | Each `WeaveOpKind` produces the template id in §3 |

Clippy on the new crate is `-D warnings`. `cargo check -p astraweave-weave` after each Phase 2 edit. `unsafe` is forbidden.

## 11. Phase 2 order

Each step is its own commit. Stop if a pre-existing failure shows up: fix it, or file it as `deferred-failure` with an owner and a plan.

1. Crate skeleton, `SIM_SYSTEM_WEAVE`, source scan against `f32` in `src/` sim modules, `HashMap`, `Instant`, `thread_rng`.
2. Template load, permutation, validation.
3. `FieldAgent` and the four neighbor passes.
4. Reaction table and the three composition rules.
5. Command log, sinks, hash.
6. No-alloc test.
7. Replay case and the workflow required set.
8. Snapshot hazard tail, behind the review answer on §14.1.
9. Legacy `WeaveOp` translator.
10. Mod data patches and `queue_weave`, stacked on #214 when that is the base.
11. Host drains for destruction, combat, and the water freeze query, each behind the PR it belongs to.
12. Headless showcase scene. Aravath call site only when that runtime is in the tree.

## 12. Known limitations to keep visible

- Rapier and GPU fluids are outside the hash.
- AI acts on a one-tick-old field.
- A dropped region resimulates only the retained command tail.
- `GravityManager` and `DestructionManager` stay dormant. Phase 2 does not wire them.
- The anchor slice, its duplicate `SimRng`, and its non-ECS systems stay until a separate cleanup. This design does not fold quests or currency into the field.
- `process_destructible_hits` stays an empty function until destruction's own PR replaces that hook.
- `EchoMod.special` stays an unread string. Burn and stun come from templates, not from that field.

## 13. Trace

Phase 2's first code commit updates `docs/architecture/weaving.md`: §5 file map (the new crate is a sibling, not a file inside `astraweave-weaving`), §8 invariants (tick order, one reaction per cell, hash inputs), §11 (whichever review questions closed). Front matter then goes through `aw_trace_sync --write`. This docs-only change does not touch the trace, because the code it describes is not in the tree yet.

## 14. Review questions

1. Is the `WorldSnapshot` hazard tail acceptable, given every planner fixture must grow the two fields?
2. Is freeze allowed to emit a static AABB in Phase 2, or is the ice flag enough until traversal lands?
3. Should `queue_weave` wait for #214 to merge, with the RON loader landing first on this branch?
4. Is the dev-dependency from `astraweave-core`'s replay test onto `astraweave-weave` acceptable? Destruction asked the same question for its crate.
5. Are integer deci-degrees and milli-g acceptable until one `Fixed64` exists in `astraweave-math`, with sinks converted at the destruction and combat boundary?

## 15. Stop

No evaluator, no replay-case edit, and no merge until those five answers are in.
