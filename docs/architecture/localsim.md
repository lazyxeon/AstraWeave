---
schema_version: 1
trace_id: localsim
title: "Local Simulation"
description: "Deterministic local fluid, gas, and heat field. Activation pool, FieldWrite, FieldEvent, field_digest."
primary_crate: astraweave-localsim
domain: physics-world
lifecycle_status: active
integration_status: partial
summary: "256-region pool, 32 active per tick. Integer volume, gas, and heat. Publishes ColumnWater. GPU fluids stay a picture. localsim.md §2,§8"
owns: [astraweave-localsim]
doc_version: "1.0"
last_verified_commit: 2d560807
runtime_edges:
  - to: astraweave-water
    note: "publish writes ColumnWater; buoyancy samples it before AnalyticWater"
---

# Architecture Trace: Local Simulation

## Metadata

| Field | Value |
|---|---|
| **System name** | Local simulation — fluid volume, gas, and heat on a region pool |
| **Primary crates** | `astraweave-localsim` (solver), `astraweave-water` (column query), `astraweave-physics` (buoyancy consumer) |
| **Document version** | 1.0 |
| **Last verified against commit** | `2d560807` (parent of this crate) |
| **Last verified date** | 2026-10-15 |
| **Status** | Active solver, partial host wiring. Weaving (PR #252) is the write producer and is not merged. Destruction voxels (PR #243) are read through `SolidField`; this crate does not own bonds. Swim (PR #244) consumes `submersion_between`. The Aravath demo showcase waits on that demo's runtime. |
| **Owner notes** | Design: `docs/current/LOCAL_SIM_DESIGN.md` revision 2. Archaeology: `docs/current/LOCAL_SIM_ARCHAEOLOGY.md`. Shared contract with #252: this crate owns the field, `FieldWrite`, `FieldEvent`, and `field_digest`. |

---

## 1. Executive Summary

**What this system does:**
Simulates local fluid volume, gas concentration, and heat on integer cells around players and AI subjects, then publishes water columns and a hazard list.

**Why it exists:**
GPU particle fluids cannot be hashed. An open world needs a region around every player and AI entity, most of them asleep, with flow that follows new holes in the solid field.

**Where it primarily lives:**
- `astraweave-localsim/src/lib.rs` — `LocalSim::tick`, activation, flow, gas, heat, sleep, digest.
- `astraweave-localsim/src/types.rs` — the #252 channel contract plus volume, gas, and wind.
- `astraweave-water/src/columns.rs` — the query the rest of the engine already asks.
- `astraweave-core/tests/replay_hash.rs` — the `local_sim` replay case.

**Status note:**
The tick does not allocate and does not take a lock. Regions run in slot order. A later job split can run active slots in parallel only because each slot writes its own cells. Do not put rayon inside `tick`.

---

## 2. Authoritative Pipeline

```text
[Host]
    subjects, FieldWrite (seq order), MaterialView, GravityView, SolidField
    │
    ▼
[LocalSim::tick]
    file: astraweave-localsim/src/lib.rs
    activate → writes → select 32 → wake → sample solid
    → 4 flow → 4 gas → 4 heat → reconcile → sleep → publish
    │
    ├─ ColumnWater rows (x, z, surface)
    ├─ HazardSample[16]
    ├─ FieldEvent (cap 128)
    └─ field_digest (SHA-256, magic AWLOCL1)
```

### Stage-by-stage detail

#### Activate
Players, then lively occupied slots, then remaining occupied slots, always by ascending slot index, up to 32. A missing subject stays occupied with `present = false` and sleeps. A full pool evicts the highest non-player subject id. Origin is the subject cell minus `(8, 8, 8)`. Overlap: the lowest slot index owns the world cell.

#### Flow
Each pass copies volume to scratch, clears the destination, and moves one step along gravity when the destination is in-region and not solid. Ice does not move. Volume that would leave the region stays. A blocked step equalizes horizontally. `SolidField::solid` is the destruction hole: `FloorSolid` is the test double until #243's overlay exists. This crate does not write that overlay.

#### Gas
Diffusion gives `gas/8` to each of six neighbors and keeps the remainder, including shares that would leave the region. Advection then steps one cell along the dominant wind axis. Ties prefer X, then Y, then Z.

#### Heat
Undirected edges +X, +Y, +Z, once per pass, from scratch temperatures. Flux is `(temp_a - temp_b) * min(conductivity) / (256 * 6)`. Conductivity 255 cannot invert a cell in one pass. A missing material id conducts 0.

#### Reconcile
Wet freezes only when `temp < freeze`. Ambient 0 stays liquid. Volume above 0 becomes wet unless the cell is ice, burn, or oil. Flammable dry or oil at `temp >= ignite` burns. A conductive cell beside burn emits `shocked`.

#### Sleep
Run-length of dirty cells, cap 1024. Above the cap the slot stays dense. Sparse sleep zeros the raw channels. Wake decodes the runs back onto those channels.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| Region | One 16³ block in the pool of 256 | `SlotMeta` |
| Active | One of at most 32 regions that run transport this tick | `select_active` |
| Quantum | One unit of `volume` or `gas`, 0–255 | solver channels |
| `FieldWrite` | A stamp from weaving. Ops include the #252 set plus `AddVolume` and `SetWind` | `types.rs` |
| `field_digest` | SHA-256 of the pool, events, hazards, and columns | `LocalSim::field_digest` |
| Column | One `(x, z)` water surface published for `WaterQuery` | `ColumnWater` |

### Terms to NOT confuse

- **This field vs `FluidSystem`:** `FluidSystem::step` is a GPU picture. It is not hashed. Gameplay transport is this crate.
- **`ColumnWater` vs `AnalyticWater`:** Columns are the near grid. The analytic plane is the far field. Physics asks columns first.
- **Phase ice vs `FreezeWater`:** Phase ice is solver truth. `FreezeWater` in gameplay weaving is still presentation-only.

---

## 4. Cross-System Touchpoints

### Upstream

| Source | Interface | Data | Notes |
|---|---|---|---|
| Weaving #252 | `FieldWrite` | temp, phase, density, flags, gravity, volume, wind | Weaving does not store the grid. Decay stays on that side |
| Destruction #243 | `SolidField` | `solid(x,y,z)`, `material_id` | Read only. Do not call `break_destructible` |
| Host | `MaterialView`, `GravityView` | conductivity, ignite, freeze, tags, gravity axis | Passed in. Not owned here |

### Downstream

| Consumer | Interface | Data | Notes |
|---|---|---|---|
| Physics buoyancy | `write_columns` → `set_water_columns` | `ColumnWater` | Empty table leaves the analytic plane |
| Traversal #244 | `submersion_between` | fraction in `[0, 1]` | Helper lives in `astraweave-gameplay`. The motor is not on this branch |
| AI perception | `hazards()` | up to 16 samples nearest the player | Copying them onto `WorldSnapshot` waits. This crate does not edit the snapshot |
| Aravath greybox | `Greybox::step` calls `LocalSim::tick` | player plus living enemies, voxel `SolidField`, columns into `PhysicsWorld` | `aravath_runtime/src/sim.rs`. The replay hash includes `field_digest` |
| GPU fluids | `write_visual` | volume, gas, temp, phase | Caller buffer. Upload is a host concern. SSFR stays deferred (#247) |

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-localsim/src/lib.rs` | Tick, solver, sleep, digest, replay script | Active | No heap allocation inside `tick` |
| `astraweave-localsim/src/types.rs` | Contract types and `FloorSolid` | Active | `FloorSolid` is a test double, not the destruction overlay |
| `astraweave-localsim/benches/tick.rs` | `localsim_tick_32_regions` | Active | Not a PR check (#223) |
| `astraweave-localsim/tests/no_alloc.rs` | Global allocator armed around one pour tick and the digest | Active | |
| `astraweave-water/src/columns.rs` | `ColumnWater` | Active | See `water.md` |
| `astraweave-core/tests/replay_hash.rs` | Writes `local_sim` into the replay JSON | Active | Dev-dependency only |
| `.github/workflows/determinism-replay.yml` | Required set includes `local_sim` | Active | Three OS compare job |

---

## 6. Conflict Map / Residue

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| GPU PBD `FluidSystem::step` | `astraweave-fluids` | Picture | Keep. Do not hash. Do not delete; examples call it |
| SSFR smooth | `ssfr_smooth_v2.wgsl` | Broken on lavapipe | Deferred #247. Do not fix in this crate |
| Analytic water plane | `AnalyticWater` | Far field | Keep. Columns win on the same cell |
| `PhysicsWorld.wind` | physics `set_wind` | Write-only Rapier vector | Not this wind field. Gas uses `SetWind` on the grid |
| Eight-region `FieldAgent` | weaving design draft | Superseded | Pool is 256 here. Weaving must not allocate regions |

- **Trap:** Treating ambient temperature 0 as frozen because the water freeze point is 0.
- **What's actually true:** Wet freezes only when `temp < freeze`. Equal temperatures stay liquid.

---

## 7. Decision Log

### Decision: Integer grid, fixed iterations, serial slots
- **Date:** 2026-10-15
- **Status:** Accepted
- **Context:** Replay must match on three operating systems. The frame budget is under 11 ms on a GTX 1660 Ti-class machine, and this CPU slice has to stay inside that.
- **Decision:** 256 preallocated regions, 32 active, 4 iterations of each transport, no `dt`, no RNG in the neighbor passes, no allocation in `tick`.
- **Alternatives considered:** Lattice Boltzmann, PIC/FLIP, hashing GPU particles. Recorded in `LOCAL_SIM_DESIGN.md`.
- **Consequences:** Mass that would leave a region stays on the edge cell. Regions do not exchange volume with each other.

### Decision: Heat flux uses min conductivity over 256 × 6
- **Date:** 2026-10-15
- **Status:** Accepted
- **Context:** `(diff * cond) / 256` with metal conductivity 200 has gain above 1 and saturates `i16`.
- **Decision:** Edge flux `(temp_a - temp_b) * min(cond) / (256 * 6)` from scratch, so one pass cannot invert a cell at conductivity 255.
- **Consequences:** Metal still outruns wood. The pass is conservative.

### Decision: Existing fluids
- **Date:** 2026-10-15
- **Status:** Accepted
- **Decision:** Defer SSFR (#247). Keep GPU PBD as a picture. Wire columns ahead of the analytic plane. Wire `submersion_between` and leave the swim motor to #244.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `tick` and `field_digest` allocate nothing | Yes | `tests/no_alloc.rs` (pour tick, so column sorting runs) |
| 2 | Two runs of the same inputs produce the same digest, including 300-tick `local_sim_case_bytes` | Yes | `digest_matches_across_two_runs` |
| 3 | At most 32 regions run transport. A player stays active inside that budget | Yes | `active_budget_keeps_the_player_and_is_stable` |
| 4 | Sleep then wake preserves volume quanta | Yes | `sleep_encoding_wakes_to_the_same_bytes` |
| 5 | Closed floor holds volume. Opening the floor lets it fall | Yes | `water_falls_onto_the_floor_and_then_into_a_breach` |
| 6 | +X wind puts more gas on +X than on −X | Yes | `gas_diffuses_and_advects_with_wind` |
| 7 | A metal neighbor ends hotter than a wood neighbor | Yes | `heat_follows_material_conductivity` |
| 8 | Neighbor passes do not draw `SimRng` | Yes (review) | No `sim` import in this crate |
| 9 | Column sort is in-place. `slice::sort_by` is not used on the tick path | Yes | `sort_columns` heapsort |

---

## 9. Performance & Resource Profile

- **Hot path:** `LocalSim::tick` plus `field_digest`, capped at 32 regions. Release bench on this cloud VM (not a GTX 1660 Ti): quiet budget `localsim_tick_32_regions` 1.33 ms; busy budget `localsim_tick_32_busy` (volume and a heat stamp in every region) 5.24 ms. Both are inside the 11 ms whole-frame figure. The bench is not a PR check.
- **Cold path:** `LocalSim::new` allocates the pool (on the order of 18 MB of channel storage). That is outside the tick.
- **Ownership:** The pool, scratch, events, hazards, and column buffer belong to `LocalSim`. The host owns subjects, writes, materials, zones, and the `SolidField`.

---

## 10. Testing & Validation

- `cargo test -p astraweave-localsim --lib --tests`
- `cargo test -p astraweave-water --lib columns::`
- `cargo test -p astraweave-gameplay --lib submersion_between`
- `cargo test -p astraweave-physics --lib column_water_wins_over_the_analytic_plane`
- `cargo test -p aravath_runtime --lib field_follows_the_player`
- `cargo test -p aravath_runtime --lib replay_matches_golden_hash`
- `cargo clippy -p astraweave-localsim --all-features -- -D warnings`

---

## 11. Open Questions / Parked Decisions

- **Host loop.** `aravath_runtime::Greybox::step` calls `LocalSim::tick` after movement and dig. The three-OS `local_sim` case stays on `feat/local-sim` (this branch does not carry `determinism-replay.yml`). The Aravath golden covers the same digest inside `state_hash`.
- **Destruction overlay.** The Aravath host reads the greybox `VoxelGrid`, so a dug cell is open on the next field tick. #243's bond overlay is a later `SolidField`. Do not fork a second solidity grid.
- **`WorldSnapshot` hazards.** The array is filled here. Copying it onto the snapshot waits on the weaving review question and must not land as a silent schema change inside this crate.
- **SSFR driven from `write_visual`.** Blocked on #247. The label `deferred-failure` could not be applied from this agent (`gh` is read-only). The issue already states the plan.
- **Cross-region flow.** Volume stops at the region edge. A breach on the boundary does not spill into the neighbor region.

---

## 12. Maintenance Notes

**Update this doc when:**
- The pool cap, active budget, iteration counts, or heat divisor change.
- A new `WriteOp` or `FieldEvent` kind is added (weaving must accept it).
- `tick` starts allocating, locking, or drawing RNG.
- A production caller other than the replay case starts owning a `LocalSim`.

**Verification process:**
- Re-run the §10 commands.
- Confirm `determinism-replay.yml` still requires `local_sim`.
- Stamp the commit hash in the front matter after the change lands.
