# Architecture Trace: PCG — Procedural Content Generation

## Metadata

| Field | Value |
|---|---|
| **System name** | PCG — Procedural Content Generation (WFC, layout, encounters, seeded RNG) |
| **Primary crates** | `astraweave-pcg` |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | In-Design (tested + benchmarked, zero wired production callers) |
| **Owner notes** | Determinism mandate dependency. The seeded-RNG layer (`seed_rng.rs`) is the canonical PCG determinism primitive cited by [`docs/audits/DETERMINISM_AUDIT_JAN_2026.md`](../audits/DETERMINISM_AUDIT_JAN_2026.md) §"PCG Layer RNG". |

---

## 1. Executive Summary

**What this system does:**
`astraweave-pcg` provides deterministic, seed-reproducible procedural generation primitives — a layer-tracked seeded RNG (`SeedRng`), a 2D Wave Function Collapse tile solver (`WfcGrid`/`TileSet`), a room/corridor layout generator (`LayoutGenerator`), and a constraint-driven encounter placer (`EncounterGenerator`).

**Why it exists:**
To give world/level generation a reproducible substrate so the engine's determinism mandate (same seed → same world) holds across procedural content; the crate's own module doc states this purpose ([`lib.rs:1-7`](../../astraweave-pcg/src/lib.rs)).

**Where it primarily lives:**
- `astraweave-pcg/src/seed_rng.rs` — deterministic RNG with `fork()` and layer tracking (the determinism primitive)
- `astraweave-pcg/src/wfc.rs` — Wave Function Collapse solver (largest module, 722 LoC)
- `astraweave-pcg/src/layout.rs` — room placement + connection graph
- `astraweave-pcg/src/encounters.rs` — encounter placement with spacing/difficulty constraints

**Status note (read this before relying on the crate):**
This crate is **in-design-but-tested** scaffolding per the CLAUDE.md "wired vs dormant" taxonomy (Key Lesson 8). It has comprehensive unit tests, a mutation-resistant test suite, and a criterion benchmark suite, but **no wired runtime consumer**. The only Cargo consumer is `astraweave-weaving`, which declares `astraweave-pcg` as a dependency but contains **zero `use` statements** referencing it (declared-but-unused dep — see [§4](#4-cross-system-touchpoints) and [§6](#6-conflict-map--residue)). A separate source tree, `astraweave-ai-gen/`, references `astraweave_pcg::{MeshGenerator, TextureGenerator}` (imported at [`generator.rs:4`](../../astraweave-ai-gen/src/generator.rs)) plus path-qualified `astraweave_pcg::PcgMesh` (line 54) — but **none of those types exist in this crate**, and `astraweave-ai-gen` has no `Cargo.toml` and is not a workspace member (orphan source). See [§6](#6-conflict-map--residue) for the full forensic disposition.

One internal seam to know up front: **WFC does not use `SeedRng`.** `WfcGrid::collapse_all<R: Rng>` ([`wfc.rs:317`](../../astraweave-pcg/src/wfc.rs)) is generic over rand's `Rng` trait; the layout and encounter generators take `&mut SeedRng` directly. These are two different determinism entry points within the same crate ([§6](#6-conflict-map--residue)).

---

## 2. Authoritative Pipeline

There is no single end-to-end pipeline; the crate is three independent generators sharing (or, in WFC's case, *not* sharing) the seeded-RNG primitive. The diagrams below trace each generator as it exists today.

### 2.1 Seeded RNG (the shared determinism primitive)

```text
seed: u64  +  layer: &str
    │
    │ SeedRng::new(seed, layer)            seed_rng.rs:15-20
    ▼
[StdRng::seed_from_u64(seed)]  ← platform-independent deterministic stream
    │
    │ fork(sublayer)                       seed_rng.rs:24-27
    ▼
[child SeedRng]   subseed = parent.inner.random::<u64>()
   layer = "parent::sublayer"
    │
    │ gen_range / gen_bool / choose / shuffle / gen_f32 / gen_f64
    ▼
[deterministic random values consumed by layout/encounter generators]
```

### 2.2 Wave Function Collapse

```text
TileSet (tiles + adjacency rules)         wfc.rs:104-173
    │  add_tile / add_tile_weighted / allow_adjacency
    ▼
WfcGrid::new(w, h, &tileset)              wfc.rs:279-288
   each Cell starts with all tiles possible (Cell::new, wfc.rs:215-221)
    │
    │ collapse_all<R: Rng>(rng)            wfc.rs:317-333   ← takes rand::Rng, NOT SeedRng
    ▼
loop:
  find_min_entropy(rng)                    wfc.rs:337-356   ← Shannon entropy + 1e-6 noise tiebreak
    │ Some((x,y))
    ▼
  observe(x, y, rng)                       wfc.rs:359-398   ← weighted random tile choice → collapse cell
    │
    ▼
  propagate(x, y)                          wfc.rs:401-460   ← arc-consistency worklist (VecDeque)
    │  on zero-possible neighbor → Err(WfcError::Contradiction)
    ▼
  (repeat until find_min_entropy returns None = fully collapsed)
    │
    ▼
to_grid() → Vec<Vec<Option<TileId>>>      wfc.rs:474-484
```

### 2.3 Layout generation

```text
LayoutGenerator::new(grid_size)           layout.rs:53-60
   defaults: room_min 5x5, room_max 15x15, max_placement_attempts 100
    │
    │ generate_rooms(&mut SeedRng, count)  layout.rs:63-76
    ▼
for each requested room:
  try_place_room(rng, &existing)          layout.rs:78-105
    random width/height/x/y; reject on overlap; up to 100 attempts
    │
    ▼
connect_rooms(rng, &mut rooms)            layout.rs:107-129
    chain-connect all rooms (guarantees reachability)
    + ~rooms.len()/3 extra random cyclic connections
    │
    ▼
Vec<Room>  (bounds + connection indices)  layout.rs:8-14
```

### 2.4 Encounter placement

```text
EncounterConstraints (bounds, min_spacing, difficulty_range)   encounters.rs:33-51
    │
    │ EncounterGenerator::generate(&mut SeedRng, count)         encounters.rs:64-97
    ▼
loop (attempts < count*10):
  generate_position(rng)                  encounters.rs:99-103  random IVec2 in bounds
  check_spacing(pos, &placed)             encounters.rs:105-113 reject if < min_spacing
    │ accepted
    ▼
  generate_kind(rng)                      encounters.rs:115-137 Combat / Loot / Ambient (rng 0..3)
  difficulty = rng.gen_range(range)       encounters.rs:83-85
    │
    ▼
Vec<Encounter> (kind + position + difficulty + empty metadata)  encounters.rs:24-30
```

### Stage-by-stage detail

#### `SeedRng::fork` (the determinism workhorse)
**File:** [`seed_rng.rs:24-27`](../../astraweave-pcg/src/seed_rng.rs)
**Role:** Derive a child RNG whose seed is drawn from the parent's stream, so each generation "layer" gets an independent-but-reproducible sub-stream.
**Inputs:** `&mut self`, `sublayer: &str`. **Outputs:** a new `SeedRng` with `layer = "{parent}::{sublayer}"`.
**Notes:** The child seed is `self.inner.random::<u64>()` — so forking advances the parent's stream. This means **the order and count of `fork()` calls is part of the determinism contract**: reorder forks and every downstream layer changes. The `layer` string is debug-only metadata; it does **not** feed into the seed (the seed comes purely from the parent's draw), so two forks with different sublayer names but the same call-order produce the same seed. Tests `test_fork_deterministic` / `test_fork_independent` / `test_fork_layer_nesting` ([`seed_rng.rs:116-138,271-277`](../../astraweave-pcg/src/seed_rng.rs)) pin this behavior.

#### WFC `find_min_entropy`
**File:** [`wfc.rs:337-356`](../../astraweave-pcg/src/wfc.rs)
**Role:** Pick the next cell to collapse — lowest Shannon entropy wins, with `rng.random::<f64>() * 1e-6` additive noise to break ties.
**Notes:** Iterates the grid in fixed row-major order, so ties resolve deterministically *given the same RNG stream*. The entropy formula (`Cell::entropy`, [`wfc.rs:228-245`](../../astraweave-pcg/src/wfc.rs)) is weight-aware: `ln(Σw) − (Σ w·ln w)/Σw`.

#### WFC `observe`
**File:** [`wfc.rs:359-398`](../../astraweave-pcg/src/wfc.rs)
**Role:** Collapse one cell to a single tile via weight-proportional roulette selection (`r = rng.random::<f32>() * total_weight`, then subtract weights until `r <= 0.0`).
**Notes:** Has a `chosen = i; // fallback to last valid` guard ([`wfc.rs:385`](../../astraweave-pcg/src/wfc.rs)) for the floating-point edge where accumulated subtraction never crosses zero. Returns `WfcError::Contradiction` if the cell already has zero possibilities.

#### WFC `propagate`
**File:** [`wfc.rs:401-460`](../../astraweave-pcg/src/wfc.rs)
**Role:** Enforce adjacency constraints outward from the just-collapsed cell using a `VecDeque` worklist (arc consistency). For each neighbor tile `b`, it is removed unless *some* still-possible tile `a` in the current cell allows `b` in that direction.
**Notes:** Re-enqueues a neighbor only if it changed (`if changed`), bounding work. A neighbor reaching zero possibilities raises `WfcError::Contradiction { x, y }`. There is **no backtracking** — a contradiction aborts the whole solve (the doc-comment example at [`wfc.rs:42-45`](../../astraweave-pcg/src/wfc.rs) notes "backtrack or retry" as a *caller* responsibility, not an in-crate feature).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Layer** | A named scope for a deterministic RNG sub-stream (`"parent::child"`). Debug metadata + organizational concept; does **not** alter the seed value. | [`seed_rng.rs`](../../astraweave-pcg/src/seed_rng.rs) |
| **Fork** | Deriving a child `SeedRng` by drawing a fresh `u64` seed from the parent's stream. Advances the parent. | [`seed_rng.rs:24-27`](../../astraweave-pcg/src/seed_rng.rs) |
| **Tile / `TileId`** | A `u16` index into a `TileSet`'s tile list; the unit WFC places. | [`wfc.rs:51`](../../astraweave-pcg/src/wfc.rs) |
| **TileSet** | The full set of tile definitions + a 4-direction adjacency table (`allowed[dir][a][b]: bool`). | [`wfc.rs:104-173`](../../astraweave-pcg/src/wfc.rs) |
| **Cell** | One grid position during WFC solving: a bitset of still-possible tiles + a cached count + optional collapsed `TileId`. Private type. | [`wfc.rs:205-267`](../../astraweave-pcg/src/wfc.rs) |
| **Collapse / Observe** | Reducing a cell's possibility set to exactly one tile (weighted random). | [`wfc.rs:359-398`](../../astraweave-pcg/src/wfc.rs) |
| **Propagate** | Constraint-propagating the consequences of a collapse to neighbors (arc consistency). | [`wfc.rs:401-460`](../../astraweave-pcg/src/wfc.rs) |
| **Entropy** | Weighted Shannon entropy of a cell's remaining possibilities; lowest is collapsed next. | [`wfc.rs:228-245`](../../astraweave-pcg/src/wfc.rs) |
| **Room** | An axis-aligned `IVec2` bounds pair + a list of connected-room indices. | [`layout.rs:8-14`](../../astraweave-pcg/src/layout.rs) |
| **Encounter / `EncounterKind`** | A placed gameplay event (`Combat` / `Loot` / `Ambient`) at an `IVec2` with a difficulty scalar. `EncounterKind` is `#[non_exhaustive]`. | [`encounters.rs:9-30`](../../astraweave-pcg/src/encounters.rs) |
| **Constraint** | Placement bounds: `bounds`, `min_spacing`, `difficulty_range`. | [`encounters.rs:33-51`](../../astraweave-pcg/src/encounters.rs) |

### Terms to NOT confuse

- **`Direction` (PCG) vs cardinal directions elsewhere:** PCG's `Direction` ([`wfc.rs:56-91`](../../astraweave-pcg/src/wfc.rs)) is a 2D `North/East/South/West` enum with `(dx, dy)` offsets where **North = `(0, -1)`** (y-down screen convention). It is local to WFC adjacency; it is unrelated to any 3D world axis.
- **"Layer" (PCG RNG) vs render/material layers:** here "layer" is an RNG-scope label, not a render or terrain-splat layer.
- **WFC `rng: R: Rng` vs `SeedRng`:** WFC consumes the *rand* `Rng` trait, not the crate's `SeedRng` wrapper. They are interoperable only if a caller passes a rand RNG (e.g. `StdRng::seed_from_u64`) directly; `SeedRng` does **not** implement `rand::Rng` and cannot currently be handed to `collapse_all` ([§6](#6-conflict-map--residue)).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source | Interface | Data | Notes |
|---|---|---|---|
| `rand` (external) | `StdRng::seed_from_u64`, `Rng::random*`, `SliceRandom::shuffle` | RNG streams | The determinism backbone. `SeedRng` wraps `StdRng` ([`seed_rng.rs:9`](../../astraweave-pcg/src/seed_rng.rs)). |
| `glam` (external) | `IVec2` | room/encounter coordinates | [`layout.rs`](../../astraweave-pcg/src/layout.rs), [`encounters.rs`](../../astraweave-pcg/src/encounters.rs) |
| `serde` (external) | `Serialize` / `Deserialize` derives | `Room`, `Encounter`, `EncounterKind` | Generated content is serializable for save/load; `EncounterConstraints` and the generators themselves are **not** serde-derived. |

### Downstream (what consumes this system's output)

| Consumer | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-weaving` | **none** | — | Declares `astraweave-pcg = { path = "../astraweave-pcg" }` ([`astraweave-weaving/Cargo.toml:23`](../../astraweave-weaving/Cargo.toml)) but **no `use astraweave_pcg`** anywhere in its sources (grep-verified). Declared-but-unused dep. This is the *only* workspace Cargo consumer. |
| `astraweave-pcg` benches | `EncounterGenerator`, `LayoutGenerator`, `Room`, `SeedRng` | generated content | [`benches/pcg_benchmarks.rs:20`](../../astraweave-pcg/benches/pcg_benchmarks.rs). Benchmark/test harness only — not a runtime caller. |
| `astraweave-ai-gen/src/generator.rs` | `astraweave_pcg::{MeshGenerator, TextureGenerator}` ([line 4](../../astraweave-ai-gen/src/generator.rs)) + `astraweave_pcg::PcgMesh` (line 54) | mesh/texture gen | **Does not resolve against this crate** — those symbols do not exist here. `astraweave-ai-gen` has no `Cargo.toml` and is not a workspace member (orphan source). See [§6](#6-conflict-map--residue). |

### Bidirectional / Coupled

- None. The crate has zero `astraweave-*` workspace dependencies (a leaf crate — confirmed by [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md) §2.2 "Leaf Crates (zero workspace deps)" and the `astraweave-pcg | _(none)_` row in §2.1).

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`src/lib.rs`](../../astraweave-pcg/src/lib.rs) | Crate root, module decls, re-exports. `#![forbid(unsafe_code)]`. | Active | Re-exports the public surface: `SeedRng`, WFC (`Direction`/`TileId`/`TileSet`/`WfcError`/`WfcGrid`), `LayoutGenerator`/`Room`, encounters (`Encounter`/`EncounterConstraints`/`EncounterGenerator`/`EncounterKind`). |
| [`src/seed_rng.rs`](../../astraweave-pcg/src/seed_rng.rs) | Deterministic layer-tracked RNG. | Active | The determinism primitive. Cited by the determinism audit. ~87 LoC + tests. |
| [`src/wfc.rs`](../../astraweave-pcg/src/wfc.rs) | Wave Function Collapse solver + `rooms_and_corridors_tileset()` helper. | Active | Largest module (722 LoC incl. tests). Consumes `rand::Rng`, not `SeedRng`. No backtracking. |
| [`src/layout.rs`](../../astraweave-pcg/src/layout.rs) | Room placement + connection graph. | Active | Consumes `&mut SeedRng`. |
| [`src/encounters.rs`](../../astraweave-pcg/src/encounters.rs) | Constraint-driven encounter placement. | Active | Consumes `&mut SeedRng`. `EncounterKind` is `#[non_exhaustive]`. |
| [`tests/mutation_resistant_comprehensive_tests.rs`](../../astraweave-pcg/tests/mutation_resistant_comprehensive_tests.rs) | Cross-module mutation-resistant test suite (772 LoC). | Active (test) | Backs the mutation audit figures. |
| [`benches/pcg_benchmarks.rs`](../../astraweave-pcg/benches/pcg_benchmarks.rs) | Criterion benchmarks for RNG / layout / encounter / full-dungeon pipeline. | Active (bench) | Declared in `Cargo.toml` `[[bench]]`. Stated targets in file header (e.g. small dungeon <1 ms). |

**Status definitions:** Active = canonical, load-bearing within the crate's own test/bench scope. Note that "Active" here means "current and exercised by tests/benches" — it does **not** imply a wired runtime caller exists (see [§1](#1-executive-summary) status note).

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition (forensic) |
|---|---|---|---|
| `SeedRng` wrapper (layer-tracked) | [`seed_rng.rs`](../../astraweave-pcg/src/seed_rng.rs) | Active | Used by layout + encounters. Does **not** implement `rand::Rng`. |
| Raw `rand::Rng` generic | [`wfc.rs:317`](../../astraweave-pcg/src/wfc.rs) `collapse_all<R: Rng>` | Active | WFC's determinism entry point. Two distinct RNG abstractions coexist in one crate; WFC cannot currently accept a `SeedRng`. Documented factually, no disposition recommended. |
| `astraweave_pcg::{MeshGenerator, TextureGenerator}` (+ path-qualified `astraweave_pcg::PcgMesh`) | imported at [`astraweave-ai-gen/src/generator.rs:4`](../../astraweave-ai-gen/src/generator.rs); `PcgMesh` used path-qualified at line 54 | **Phantom / does not exist in this crate** | Verified: line 4 imports exactly `use astraweave_pcg::{MeshGenerator, TextureGenerator};` and line 54 references `astraweave_pcg::PcgMesh` — none of those three symbols are present in `astraweave-pcg`. (`MeshGenerationParams` comes from `crate::schema`, **not** `astraweave_pcg`, so it is not part of the phantom PCG surface — corrected from an earlier draft that listed it here.) `astraweave-ai-gen/` has no `Cargo.toml` and is not listed in the root workspace members — it is orphan source (per CLAUDE.md "Orphan source" taxonomy). The reference describes a *different or aspirational* PCG surface (mesh/texture generation) that this crate does not implement. **Do not assume these belong here.** |

### Naming collisions

- **`astraweave-pcg` (the name) refers to two different surfaces in the tree.** The live workspace crate implements WFC/layout/encounters/seeded-RNG. The orphan `astraweave-ai-gen` source expects an `astraweave_pcg` that exports mesh/texture generators (`MeshGenerator`, `TextureGenerator`, `PcgMesh`). These are mutually exclusive surfaces under the same crate name. The live crate is authoritative; the ai-gen expectation is unreconciled residue. `MeshGenerator`/`TextureGenerator` symbols *do* exist elsewhere in the tree (e.g. terrain meshing, `examples/unified_showcase/src/texture_generator.rs`) but those are unrelated types, not `astraweave_pcg` exports.
- **`Direction`** is also a common name across the engine; PCG's is a private-to-WFC 2D enum and should not be conflated with any 3D or input direction type ([§3](#3-semantic-vocabulary)).

### Known cognitive traps

- **Trap: assuming WFC is seeded through `SeedRng`.** It is not — `collapse_all` takes any `rand::Rng`. The crate's own doc-comment example ([`wfc.rs:41`](../../astraweave-pcg/src/wfc.rs)) even shows `let mut rng = SeedRng::new(42);` being passed to `collapse_all`, but that call would not compile as written: (1) `SeedRng::new` takes **two** args `(seed, layer)` not one, and (2) `SeedRng` does not implement `rand::Rng`. The example is `///ignore`d and is illustrative, not a working snippet.
- **Trap: treating the crate as wired gameplay.** It has full tests + benches but no runtime caller. Passing tests ≠ shipped feature (CLAUDE.md Key Lesson 8).
- **Trap: `EncounterKind` is `#[non_exhaustive]`** ([`encounters.rs:10`](../../astraweave-pcg/src/encounters.rs)) — external `match` must include a wildcard arm.

### Documentation-hazard note

The WFC module doc-comment example block ([`wfc.rs:26-46`](../../astraweave-pcg/src/wfc.rs)) does not match the current `SeedRng` API (single-arg `SeedRng::new`, and `SeedRng` passed where `rand::Rng` is required). Treat it as aspirational illustration, consistent with CLAUDE.md "Doc-comment migration drift."

---

## 7. Decision Log

### Decision: Layer-tracked deterministic RNG via `fork()`
- **Date:** [Reasoning not recovered from available sources — first introduced no later than the file's earliest committed state; the crate predates commit `802ca086c` "Implement compressed voxel storage…" which is the earliest `git log` entry touching `astraweave-pcg/src/wfc.rs`.]
- **Status:** Accepted.
- **Context:** The engine's determinism mandate requires same-seed-same-world. [`docs/audits/DETERMINISM_AUDIT_JAN_2026.md`](../audits/DETERMINISM_AUDIT_JAN_2026.md) §"PCG Layer RNG" documents `SeedRng` as a determinism guarantee (platform independence, identical sequences from same seed, layer tracking for debugging).
- **Decision:** Wrap `StdRng` and expose a `fork(sublayer)` that derives child seeds from the parent stream, with a debug-only `layer` label.
- **Alternatives considered:** [Not recovered from available sources.]
- **Consequences:** RNG-call order and fork order become part of the reproducibility contract; reordering generation steps changes output for the same seed.

### Decision: `StdRng` (ChaCha-family) as the RNG backend
- **Status:** Accepted.
- **Context:** `seed_rng.rs:9` selects `rand::rngs::StdRng`. The determinism audit explicitly claims "Platform independence (Windows, Linux, macOS, WASM)" for the RNG layer ([`DETERMINISM_AUDIT_JAN_2026.md`](../audits/DETERMINISM_AUDIT_JAN_2026.md) §"Guarantees").
- **Decision:** Use `StdRng::seed_from_u64`. **[NEEDS VERIFICATION]** that the pinned `rand` version's `StdRng` is in fact reproducible across platforms/versions — `StdRng` is documented by `rand` as *not* guaranteed stable across major `rand` releases. The audit asserts platform independence; cross-version stability is a separate property not evidenced in-crate. (Investigated: `rand` is pinned at `"0.9"` in the root [`Cargo.toml:172`](../../Cargo.toml); the determinism audit comments the backend as ChaCha12 at [`DETERMINISM_AUDIT_JAN_2026.md:103`](../audits/DETERMINISM_AUDIT_JAN_2026.md). Within `rand 0.9` reproducibility holds; what remains unverified is stability across a future `rand` major bump — `rand`'s own docs disclaim that guarantee, so this cannot be resolved from in-crate evidence.)
- **Consequences:** Reproducibility holds within a fixed `rand` version; a `rand` major-version bump could change generated content for the same seed.

### Decision: WFC has no backtracking; contradiction aborts the solve
- **Status:** Accepted.
- **Context:** [`wfc.rs:401-460`](../../astraweave-pcg/src/wfc.rs) `propagate` returns `WfcError::Contradiction` and `collapse_all` propagates it ([`wfc.rs:328`](../../astraweave-pcg/src/wfc.rs)). The doc-comment ([`wfc.rs:44`](../../astraweave-pcg/src/wfc.rs)) frames "backtrack or retry" as a caller concern.
- **Decision:** Surface contradictions as errors rather than implementing in-solver backtracking.
- **Alternatives considered:** [Not recovered from available sources — backtracking/retry left to callers.]
- **Consequences:** Callers must retry with a different seed or pre-seeding on `Contradiction`; the crate provides no automatic recovery.

### Decision: `EncounterKind` marked `#[non_exhaustive]`
- **Status:** Accepted.
- **Context:** [`encounters.rs:10`](../../astraweave-pcg/src/encounters.rs).
- **Decision:** Allow future encounter kinds to be added without a breaking change.
- **Consequences:** Downstream matches must carry a wildcard arm. [Rationale beyond the API-stability convention not separately documented.]

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Same `(seed, layer)` → identical RNG sequence. | Yes | `seed_rng.rs::tests::test_same_seed_same_sequence`; `mutation_resistant_comprehensive_tests::seed_rng_deterministic_same_seed`. |
| 2 | Forked children are deterministic and independent of the parent stream. | Yes | `test_fork_deterministic`, `test_fork_independent` ([`seed_rng.rs:116-138`](../../astraweave-pcg/src/seed_rng.rs)). |
| 3 | Fork layer names nest as `"a::b::c"`. | Yes | `test_layer_tracking`, `test_fork_layer_nesting`. |
| 4 | A fully-collapsed WFC grid satisfies all adjacency rules. | Yes | `wfc::tests::wfc_collapse_simple` validates every neighbor pair against `is_allowed`. |
| 5 | Same seed → same WFC result. | Yes | `wfc::tests::wfc_deterministic_with_seed`. |
| 6 | Generated rooms never overlap and are all connected. | Yes | `layout::tests::test_no_overlaps`, `test_all_rooms_connected`. |
| 7 | Encounters respect `min_spacing`, `bounds`, and `difficulty_range`. | Yes | `encounters::tests::test_spacing_constraint` / `test_bounds_constraint` / `test_difficulty_range`. |
| 8 | Same seed → identical room layout and identical encounter set. | Yes | `layout::tests::test_deterministic_generation`, `encounters::tests::test_deterministic_generation`. |
| 9 | Tile weights are clamped to ≥ 0.001 (no zero-weight tiles). | Yes (doc/code) | `add_tile_weighted` clamps via `weight.max(0.001)` ([`wfc.rs:131`](../../astraweave-pcg/src/wfc.rs)). |
| 10 | Crate contains no `unsafe` code. | Yes | `#![forbid(unsafe_code)]` ([`lib.rs:1`](../../astraweave-pcg/src/lib.rs)). |

---

## 9. Performance & Resource Profile

### Hot paths
- **WFC `propagate` / `find_min_entropy`** ([`wfc.rs:337-460`](../../astraweave-pcg/src/wfc.rs)): `find_min_entropy` rescans the entire grid each iteration (O(width·height·tiles) per collapse), and `propagate`'s support check is O(tiles²) per neighbor edge. Cost scales with grid size × tile count. Cold path overall (one-time level generation), but the dominant cost inside a generation.
- Benchmark targets (stated in [`benches/pcg_benchmarks.rs:9-14`](../../astraweave-pcg/benches/pcg_benchmarks.rs)): small dungeon (5–10 rooms) <1 ms; medium (20–30) <10 ms; large (50–100) <50 ms; 100 encounters <5 ms; RNG ops <100 ns. Measured numbers in [`docs/masters/MASTER_BENCHMARK_REPORT.md`](../masters/MASTER_BENCHMARK_REPORT.md) §5.7 are well within target: huge dungeon / 100 rooms = 277.5 µs, encounter generation sub-µs, room overlap check sub-600 ps (verified against report §5.7 at commit `7c29b8182`).

### Cold paths
- Layout and encounter generation run once per level/region build; bounded retry loops (`max_placement_attempts = 100`, encounter `attempts < count*10`) cap worst-case work.

### Resource ownership
- All state is owned by the generator structs / `WfcGrid` and produced as owned `Vec`s; no global or `'static` resource, no GPU resource, no ECS resource. The crate holds nothing across frames.

---

## 10. Testing & Validation

- **Unit tests:** In-module `#[cfg(test)]` in each of `seed_rng.rs`, `wfc.rs`, `layout.rs`, `encounters.rs` — covering determinism, constraint satisfaction, edge cases (empty/tiny grids, serde round-trips, zero counts).
- **Integration / cross-module tests:** [`tests/mutation_resistant_comprehensive_tests.rs`](../../astraweave-pcg/tests/mutation_resistant_comprehensive_tests.rs) (772 LoC).
- **Mutation testing:** Completed. [`docs/current/MUTATION_TESTING_AUDIT.md`](../current/MUTATION_TESTING_AUDIT.md) records `astraweave-pcg` at **65.3% raw / 100% adjusted** kill rate (106 mutants, 12 kill tests added; full-crate run). Audit commit reference: `3b5e79604` "test(astraweave-pcg): complete mutation audit (65.3% raw, 100% adj)".
- **Miri:** Not required — crate is `#![forbid(unsafe_code)]`; not in the Miri-validated set (`ecs`/`math`/`core`/`sdk`).
- **Benchmarks:** [`benches/pcg_benchmarks.rs`](../../astraweave-pcg/benches/pcg_benchmarks.rs) (criterion, `harness = false`).
- **Manual validation:** None — no runtime/visual integration exists (no wired caller).

---

## 11. Open Questions / Parked Decisions

- **Is the crate intended to be wired, or is it design-ahead scaffolding?** It has tests + benches + mutation coverage but no runtime caller; the one Cargo consumer (`astraweave-weaving`) does not import it. Is `astraweave-weaving` the intended integration point (dep declared in anticipation), or is the dep vestigial?
- **What is the relationship to `astraweave-ai-gen`'s expected `astraweave_pcg::{MeshGenerator, TextureGenerator, PcgMesh}` surface?** `astraweave-ai-gen` is orphan (no `Cargo.toml`, not a workspace member) and expects a PCG crate that exports mesh/texture generators this crate does not provide. Was there an earlier/parallel `astraweave-pcg` with a mesh/texture surface, or is `ai-gen` aspirational? This needs reconciliation before either is wired.
- **Should WFC accept `SeedRng` for layer-tracked determinism?** Today WFC's determinism flows through raw `rand::Rng`, separate from the `SeedRng` layer system that layout/encounters use. A world-generation caller wanting one reproducible seed-tree across all three generators would have to bridge these. (Stated as a question, not a recommendation.)
- **Is `StdRng` cross-version reproducible for save-compatibility?** The determinism audit claims platform independence; `rand`'s own docs do not guarantee `StdRng` stability across `rand` major versions. If generated content is persisted (rooms/encounters are serde-derived), a `rand` bump could change same-seed output. [NEEDS VERIFICATION] against the pinned `rand` version's guarantees.
- **WFC contradiction recovery:** with no in-solver backtracking, who owns retry policy when `collapse_all` returns `Contradiction`? No workspace caller currently exercises this path.

---

## 12. Maintenance Notes

**Update this doc when:**
- Any Active file in [§5](#5-active-file-map) changes.
- WFC's RNG abstraction changes (e.g. it starts accepting `SeedRng`) — this would resolve a [§6](#6-conflict-map--residue) coexistence.
- A real runtime caller appears (resolving the "in-design" status in [§1](#1-executive-summary)) — update the wired-status claim and [§4](#4-cross-system-touchpoints) downstream table.
- The `astraweave-ai-gen` orphan is reconciled (wired with a `Cargo.toml`, deleted, or its PCG surface implemented) — update [§6](#6-conflict-map--residue).
- An invariant in [§8](#8-known-invariants) is relaxed.

**Verification process:**
- Re-run the four self-checks from the trace toolkit: every concrete claim cites a file; uncertainty is marked; no invented rationale; no refactor proposals.
- Re-grep wired status: `rg 'use astraweave_pcg' --type rust -g '!*test*' -g '!benches/*'` and confirm whether any non-test/non-bench caller exists.
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **It is not wired into the runtime.** Tests/benches pass; no production caller. Don't assume changes here affect a running game (CLAUDE.md Key Lesson 8).
2. **Two determinism entry points:** layout/encounters use `SeedRng`; WFC uses raw `rand::Rng`. They are not unified.
3. **Fork order is part of the determinism contract.** Reordering `SeedRng::fork()` calls changes all downstream output for the same seed.
4. **`astraweave-ai-gen` is orphan and expects a PCG surface (mesh/texture) that does not exist here.** Don't chase those symbols into this crate.

**Files you'll most likely touch:**
- [`src/wfc.rs`](../../astraweave-pcg/src/wfc.rs) (largest, most logic)
- [`src/seed_rng.rs`](../../astraweave-pcg/src/seed_rng.rs) (the determinism primitive)
- [`src/layout.rs`](../../astraweave-pcg/src/layout.rs), [`src/encounters.rs`](../../astraweave-pcg/src/encounters.rs)

**Files you should NOT touch without strong reason:**
- [`tests/mutation_resistant_comprehensive_tests.rs`](../../astraweave-pcg/tests/mutation_resistant_comprehensive_tests.rs) — backs the 100%-adjusted mutation figure; weakening it regresses the audit.

**Common mistakes when changing this system:**
- **Passing `SeedRng` to `collapse_all`** — it won't compile; `SeedRng` doesn't implement `rand::Rng`. Pass a `rand` RNG (e.g. `StdRng::seed_from_u64`).
- **Trusting the WFC doc-comment example verbatim** — it uses the old single-arg `SeedRng::new` and is `///ignore`d.
- **Adding an `EncounterKind` variant** without a wildcard-aware downstream — it's `#[non_exhaustive]`.
- **Reordering generation/fork calls** "for clarity" — silently changes same-seed output and breaks reproducibility.

---

## Appendix B: Historical context

The crate's module doc frames it as "deterministic procedural generation for AstraWeave" with seed-based RNG, encounter placement, and layout generation ([`lib.rs:1-7`](../../astraweave-pcg/src/lib.rs)). The seeded-RNG layer is one of three deterministic-RNG surfaces catalogued in the January 2026 determinism audit (the others being a "main RNG" and the `astraweave-weaving` test utility `assert_deterministic_behavior`), which marks Weaving/PCG determinism as 100% via multi-run consistency ([`DETERMINISM_AUDIT_JAN_2026.md`](../audits/DETERMINISM_AUDIT_JAN_2026.md) §"Determinism Test Coverage"). The orphan `astraweave-ai-gen` source — which expects a mesh/texture-oriented `astraweave_pcg` — suggests an earlier or parallel conception of "PCG" focused on asset generation rather than tile/layout/encounter generation; that conception is not realized in the current crate and remains unreconciled residue. [Full reasoning for the divergence not recovered from available sources.]
