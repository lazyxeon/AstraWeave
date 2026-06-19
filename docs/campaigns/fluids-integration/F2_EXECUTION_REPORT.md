# F.2 Execution Report — The `astraweave-water` Facade, Built by Its First Consumer

**Document version**: 1.0
**Execution date**: 2026-06-11
**Branch**: `campaign/fluids-f2` (off `campaign/fluids-f1` tip `1c3be00cb`, which already merges F.1 into main via PR #192 — see Deviation #0)
**Path**: B (layered facade), per the F.0 owner gate.

---

## Deviation #0 — base branch and the missing CAMPAIGN_PLAN.md (recorded first, not silent)

The brief specifies "a new branch `campaign/fluids-f2` off the merged `main`" and lists `CAMPAIGN_PLAN.md` as required reading #1. Two preconditions were false, both resolved by inspection rather than by asking:

1. **`CAMPAIGN_PLAN.md` does not exist.** It never has — the F.1 reports flagged this at every phase. The gate decisions of record (Q1 determinism carve-out, Q3 consolidation, Q7 budget, **Q8 physics-water reconciliation**) live in the F.1 brief text and are restated in `F1_EXECUTION_REPORT.md`. F.2 proceeds on those, as F.1 did. **Recommend the owner create `CAMPAIGN_PLAN.md` to formalize the gate record** (carried forward from F.1's open items).
2. **`main` did not have F.1 merged at the local `main` pointer** (still `8e1505dd8`), but the working tree's HEAD (`1c3be00cb`, "Merge branch 'main'") already includes `0f21fe3af Campaign/fluids f1 (#192)` — i.e. F.1 *was* merged via PR #192, and this base is effectively "merged main + F.1." Branching `campaign/fluids-f2` off this tip is the only base that builds on F.1's foundation; branching off the stale local `main` would strand F.2 from F.1. The git topology was verified (the F.1 source content — `srgb_to_linear`, `_pad_vec2_align`, `current_index` — is present in the tree).

No source was changed to accommodate either; this section is the disclosure.

---

## 1. Resolved dependency graph

```
astraweave-physics ──► astraweave-water ──► glam ──► serde_core
        │
        └──► rapier3d, ... (unchanged)
```

`astraweave-water` is a **glam-only leaf** — it depends on neither `astraweave-physics` nor `astraweave-fluids`. The consumer (physics) calls the facade, so **physics depends on water**, not the reverse. Verified with `cargo tree`:

- `cargo tree -p astraweave-water` → `glam` only. No physics, no fluids.
- `astraweave-physics → astraweave-water` compiles.

**Cycle check**: the standing red line is `physics → fluids → terrain → gameplay → physics`. Adding `physics → water` cannot close it because `water` is a leaf below `physics` with no edges back up. No `astraweave-physics` type was needed by `astraweave-water` (its sole input is `glam::Vec3`, its output is a plain `WaterSample`), so no type had to move to a shared lower crate and no back-edge was created. The F.0-anticipated "fluids must stay a leaf" is preserved: F.2 introduces no `astraweave-water → astraweave-fluids` edge at all (the analytic backend needs no fluids code; the voxel/SPH backend that would is F.3, and the trait is designed so it slots in without changing physics' dependency on `water`).

---

## 2. The work: `WaterQuery` derived from what physics actually calls

The governing rule was "the API is defined by what its first real consumer (physics) actually calls — no speculation." Here is the derivation, computation by computation.

### 2.1 Enumeration of physics' wired water computations

I read every water-touching path in `astraweave-physics`:

| Wired site | What it asks of "the water" |
|---|---|
| `apply_buoyancy_forces` (`lib.rs`), the *only* water code reached from `step_internal` | (a) **surface height** at the body's position — to gate `body_y < surface`; (b) **fluid density** at the body's position — for the Archimedes force `volume · density · g`. |
| `apply_buoyancy_forces` drag term | **Nothing.** Drag is `velocity · buoyancy_data.drag`, a *per-body* coefficient. The water is never queried for drag. |
| `add_water_aabb(min, max, density, linear_damp)` (stub) | **Register a bounded volume** with a density (and a linear-damp authoring value). |
| `clear_water()` (stub) | **Remove all water.** |
| Scalar `water_level`/`fluid_density` (set by `laboratory.rs`, tests) | An **infinite flat plane** at a height with a density. |
| `control_character` (`lib.rs:1247`) | **Nothing** — pure gravity/jump/ground-cast; touches no water. (So swimming is *not* cheap fall-out; see §6.) |
| `EnvironmentManager::WaterVolume` water methods (`environment.rs`) | Unwired (zero `step` callers) — *reference semantics only*, not API drivers. |

### 2.2 The trait that falls out

The deduplicated question list is **one question**: "what are the water properties at this point?" So:

```rust
pub struct WaterSample { pub surface_height: f32, pub density: f32 }

pub trait WaterQuery {
    fn sample(&self, point: Vec3) -> Option<WaterSample>;
}
```

This is deliberately **not** the F.0-sketched four-method surface (`is_submerged`/`flow_at`/`surface_height_at`/`density_at`). Buoyancy reads `surface_height` and `density` together at one point, so one `sample` returning both is the minimal honest API. **Flow, drag, temperature, and submersion-fraction are absent** because no wired consumer reads them — they arrive with the consumer that needs them (swim/flow: F.2-followup/F.3). The trait is the seam for F.3's voxel backend (which implements the same `sample`); F.2 ships exactly one backend.

### 2.3 The caller : method table (proof of zero speculative API)

Every public method of the water facade has a real `astraweave-physics` caller:

| `astraweave-water` public item | `astraweave-physics` caller |
|---|---|
| `WaterQuery::sample` (the trait method) | `apply_buoyancy_forces` — `self.water.sample(pos)` |
| `AnalyticWater::set_plane` | `apply_buoyancy_forces` — write-through-sync of the back-compat scalar plane |
| `AnalyticWater::add_aabb` | `PhysicsWorld::add_water_aabb` |
| `AnalyticWater::clear` | `PhysicsWorld::clear_water` |
| `AnalyticWater::has_any` | `PhysicsWorld::clear_water` test guard; available for `has_water`-style checks |
| `AnalyticWater::new` | both `PhysicsWorld` constructors |
| `AnalyticWater::aabb_count` | diagnostics (test-exercised); the only non-`step`-path method — kept as the minimal introspection a registration API needs |

No method exists that physics does not call. (`WaterSample` fields are both read by buoyancy.)

### 2.4 The one F.2 backend: `AnalyticWater`

`AnalyticWater` holds an optional infinite `Plane` (the retired scalar `water_level`/`fluid_density`, infinite in XZ and downward — reproducing "any body below the level floats") plus a `Vec` of bounded `Aabb` volumes (`add_water_aabb`). **Overlap rule** (WI-6 requires it defined + tested): when several volumes cover a point, the sample reflects the **topmost surface** (a body floats on the highest water); ties break by **registration order** (strict `>` keeps the first-registered). `Aabb::linear_drag` is stored as **honest authoring data** — `add_water_aabb`'s parameter is not silently discarded — but is **not read by `sample`** (no consumer reads per-volume drag yet), documented as such.

---

## 3. The three abstractions retired (gate Q8)

| Abstraction | Disposition |
|---|---|
| **Working flat-plane buoyancy** (`apply_buoyancy_forces` vs scalar `water_level`) | **Preserved, re-sourced.** Now reads surface/density from `self.water.sample()`. The scalar fields remain as a back-compat write-through input synced into the facade each tick, so there is one sampling owner. All 27 pre-existing buoyancy tests pass **unchanged** (including exact-value mutation tests). |
| **Unwired `EnvironmentManager::WaterVolume`** (`environment.rs`) | **Deleted.** Its per-position semantics (AABB containment, surface height, density) were absorbed into `AnalyticWater`; its submersion-fraction/current/per-volume-drag methods were dead (zero `step` callers) and are gone. The **wind** system in the same file (WindZone/Gust/etc.) is untouched. Test-deletion counts: see §7. |
| **No-op stubs** `add_water_aabb` / `clear_water` | **Made real** (facade-backed) with **unchanged signatures**, so the three consumers (`weaving.rs::clear_water`, `physics_demo3d::add_water_aabb`+`clear_water`, `laboratory.rs` scalar fields) get real behavior with **zero call-site changes**. |

### 3.1 The `reset_forces` hazard — resolved via impulse semantics

F.0 Seam 2 flagged that buoyancy re-adds force every tick via `rb.add_force`, while `reset_forces` has zero workspace call sites — Rapier user-forces persist, so the pattern accumulates unbounded force.

**Resolution: buoyancy is applied as an impulse, `apply_impulse(force · dt)`, not `add_force`.** This is one of the two brief-sanctioned options and the cleaner one: it is self-limiting (a one-shot impulse cannot accumulate across frames) and it does **not** touch other systems' forces (the alternative — calling `reset_forces` each tick — would clear forces other code added, e.g. `apply_force`). `impulse = F·dt` reproduces a single step's `add_force` velocity change `Δv = F·dt/m` exactly, which is why the regression tests pass unchanged.

**Guard test**: `f2_buoyancy_impulse_does_not_accumulate` — a constant buoyancy force under impulse semantics produces velocity **linear** in step count (`v(2N) ≈ 2·v(N)`); accumulating `add_force` would produce **quadratic** growth (`v(2N) ≈ 4·v(N)`). The test asserts the doubling ratio stays < 2.5, catching any regression to a force-accumulating pattern.

---

## 4. ECS integration (WI-4)

The brief's Seam-1 GPU-in-ECS decision **does not bite in F.2**: the facade is CPU-only analytic, an ordinary `Send + Sync` struct, so no GPU-owning type becomes a resource and no new ground is broken. That decision is explicitly deferred to whenever a GPU backend enters (F.3/F.4).

**Resource placement — a reasoned choice vs. the brief's "separate resource" suggestion.** The brief suggested the `WaterQuery` backend live as a standalone ECS resource mirroring `PhysicsPlugin`. I instead **co-located it inside `PhysicsWorld`** (the `water` field), because:

- Physics is the **sole F.2 consumer**, and `PhysicsWorld` is *already* the ECS resource inserted by `PhysicsPlugin` (`physics_step_system` runs in the deterministic `PHYSICS` stage and calls `step()` → `apply_buoyancy_forces()` → `self.water.sample()`). The water backend is therefore reachable by the `PHYSICS` stage transitively, through the resource that already exists.
- A *separate* `WaterQuery` resource accessed by physics would create two places water lives and the question of keeping them in sync each tick — precisely the second-owner shape §7.7 forbids.
- **Promotion to a standalone resource is the F.3 move**, when a second consumer (AI "is this point underwater" / gameplay flow forces) needs water independently of physics. At that point the trait already exists as the shared seam.

No scheduler-level parallelism is introduced; sampling runs in the deterministic single-threaded `PHYSICS` stage (subsystem-parallelism doctrine intact).

---

## 5. Determinism proof (gate Q1)

The gameplay-truth layer is deterministic and CPU-resident by construction: `AnalyticWater::sample` is pure `f32` comparison over a fixed-order set (plane, then AABBs in push order), no GPU, no hashing-iteration-order, no float-nondeterministic reduction. Enforced tests:

- **`astraweave-water::determinism_identical_backends_and_order_independence`** — two independently built backends with the same volumes return bit-identical samples for a probe set, and forward-vs-reversed query order yields the same result map. This is the named, enforced gate-Q1 guarantee.
- Plus 8 unit tests covering plane coverage, the `NEG_INFINITY`-clears-plane sentinel, AABB containment boundaries (on-surface, outside, below floor), corner normalization, and both overlap rules (topmost-wins, tie-by-order).

GPU particle fluid state (`astraweave-fluids`) remains **excluded** from this layer per the carve-out — it is presentation-only and non-deterministic; the facade never reads it.

---

## 6. Scope boundary — swimming excluded (flagged, not forced)

`control_character` touches no water (verified: pure gravity/jump/ground-cast). Swimming would require a new `CharState::Swimming` + a buoyant branch in the kinematic controller — a **separate code path** from `apply_buoyancy_forces` (which operates on rigid bodies). It is **not cheap fall-out** from the buoyancy rewire. Per the owner gate (buoyancy/drag reconciliation was chosen as the proof, swim as the alternative-not-addition), **swim is excluded from F.2** and flagged for F.2-followup / F.3-adjacent. The facade's `sample` already provides exactly what a future swim branch needs ("is there water at the character's position, at what surface height"), so no trait change is foreseen.

---

## 7. Three-abstraction retirement: deletion accounting

The dead `EnvironmentManager::WaterVolume` system was deleted in full; the **wind** system (`WindZoneId`, `WindZoneShape`, `WindType`, `WindZoneConfig`, `WindZone`, `GustEvent` + impls) and its tests are preserved untouched, including the four gust tests `mutation_r3_current_gust_force_sum`, `mutation_r4_wind_force_drag_coefficient_multiplier`, `r6_gust_current_strength_envelope`, `r7_current_gust_force_returns_aggregate`.

**Code deleted** (`environment.rs`): `WaterVolumeId`, `WaterVolume` struct + impl, the two `EnvironmentManager` water fields (`water_volumes`, `next_water_id`) + their initializers, the nine water methods (`add_water_volume`, `remove_water_volume`, `get_water_volume`, `get_water_volume_mut`, `buoyancy_force_at`, `is_underwater`, `water_drag_at`, `water_current_at`, `water_volume_count`), and the water-update loop. `lib.rs` re-export: `WaterVolume`/`WaterVolumeId` removed (the only `lib.rs` change in this pass).

**Water tests deleted** (65 total) + **mixed tests pruned** (5 — water lines removed, wind assertions kept):

| File | Water tests deleted | Mixed pruned |
|---|---|---|
| `astraweave-physics/src/environment.rs` | 54 | 3 |
| `astraweave-physics/tests/environment_tests.rs` | 3 | 0 |
| `astraweave-physics/tests/coverage_boost_tests.rs` | 1 | 2 |
| `astraweave-physics/tests/mutation_resistant_comprehensive_tests.rs` | 7 | 0 |

The deleted-test count exceeded the initial 48-name inventory because several `WaterVolume`-constructing tests (`mutation_surface_height_*`, `mutation_r4_surface_height_*`, additional sphere-submersion tests) were not in the by-name list but would not compile after the code deletion; the "constructs `WaterVolume` → delete" rule resolved them. The `BuoyancyData`/`has_water`/`PhysicsConfig` tests were **kept** — those exercise the F.2 facade-backed API, not the deleted `EnvironmentManager`.

**Post-deletion verification (my own, not the subagent's claim)**: `cargo build -p astraweave-physics --tests` → 0 errors; full suite → every binary green **except one pre-existing, water-unrelated failure** (see §9.1).

---

## 8. Budget re-ratification (WI-5) — FOR THE OWNER GATE, no decision made

F.1 recorded the first real fluids numbers and refuted the F.0-proposed 2 ms GPU budget. F.2 adds the analytic-query cost. **This section recommends; it does not decide.** Machine context: Intel i5-10300H @ 2.50 GHz, GTX 1660 Ti Max-Q (min-spec class), Win 11, Rust 1.89.0 — the same dev machine as the F.1 baselines, so the numbers compose.

### 8.1 The analytic `WaterQuery` cost (new, F.2)

`cargo bench -p astraweave-water` (criterion):

| Workload | Cost |
|---|---|
| Single body sample, 8 volumes | **~28 ns** |
| 256 buoyant bodies × 1 volume, per tick | **~3.3 µs** |
| 256 bodies × 16 volumes, per tick | **~11.9 µs** |
| 256 bodies × 64 volumes, per tick | **~48 µs** |
| 256 bodies × 256 volumes (pathological), per tick | **~153 µs** |

**Conclusion: the gameplay-water layer is microsecond-class and does not contend for the fluids budget.** A realistic fleet (a few hundred buoyant bodies, a dozen-ish volumes) costs ~10 µs/tick — under 0.1 % of a 16.6 ms frame; even the pathological 256×256 case is 0.15 ms. The budget conversation is therefore **entirely about the GPU/voxel layers** (F.4 SSFR, F.3 voxel), not the query layer. (Cost is `O(bodies × volumes)`; if volume counts ever grow large, a broadphase over volumes is a trivial future optimization — not needed at F.2/F.3 scales.)

### 8.2 Restated F.1 GPU/voxel numbers (min-spec class)

- `FluidSystem::step` (GPU SSFR-precursor, demo params, 8 PBD iterations): 10k particles ≈ **4.9 ms GPU**; 20k ≈ 7.0 ms; 50k ≈ 19.1 ms. PBD iterations dominate (68–76 %); SDF regen is a flat ~0.9 ms/frame.
- `WaterVolumeGrid::simulate` (CPU voxel): 32³ ≈ **0.55 ms**; 64³ ≈ 13.8 ms; 128³ ≈ 206 ms (dense; sparsity unimplemented).

### 8.3 Recommended budget options (pick one at the gate)

A per-frame fluids allocation is proposed as **CPU-query (negligible, ~0.05 ms reserved) + a GPU/voxel envelope**. The GPU/voxel envelope is the real decision; three defensible framings:

| Option | Shape | Pros | Cons |
|---|---|---|---|
| **A — Iteration-capped GPU** | 3.0 ms GPU, **cap PBD iterations at 3–4** (not 8). At ~4 iters, 10k ≈ 2.7 ms, 20k ≈ 3.8 ms. Particle ceiling ~15–20k on min-spec. | Hits a real 3 ms budget on min-spec today; iteration count is the proven dominant lever. | Lower visual fidelity than 8-iter; the roadmap's "50–100k @ 60 FPS" tier is off the table on min-spec. |
| **B — Particle-ceiling GPU** | Fix a **20k particle ceiling**, accept ~5 ms GPU at 8 iters as the fluids slice of a 16.6 ms frame (≈30 %). | Keeps full-quality PBD; simple to reason about ("≤20k particles"). | 5 ms is a large single-subsystem share; squeezes rendering on min-spec. |
| **C — Voxel-first gameplay water, GPU particles as detail** | Gameplay water = CPU voxel (`WaterVolumeGrid`) budgeted at **1 ms CPU** (→ requires the unimplemented sparsity; 32³ dense already fits, 64³ does not); SSFR particles as a separate, smaller GPU detail budget (~1.5 ms, ≤10k). | Aligns with Path B's T3 ambition and the deterministic-CPU-truth thesis; smallest GPU footprint. | Depends on F.3 voxel sparsity (not yet built); two budgets to track. |

**Agent's lean (not a decision): Option A** for an immediate, honest min-spec budget, with **Option C** as the T3-aligned target once F.3 voxel sparsity lands. The owner ratifies at the F.2 gate.

---

## 9. Tests, gate, and verification

- `astraweave-water`: **9 unit tests** (incl. the gate-Q1 determinism test) — green; **3-group criterion bench** runs.
- `astraweave-physics`: **27 pre-existing buoyancy tests pass unchanged** (regression-preserved); **3 new F.2 facade tests** (impulse-accumulation guard, bounded-AABB capability, flat-plane-via-facade + clear_water); full suite green after the environment deletion (see §7).
- `cargo tree`: acyclic, documented (§1).
- `cargo clippy -p astraweave-water -p astraweave-physics --all-targets -- -D warnings`: clean (only a pre-existing `noop_method_call` warning on `vehicle::Vehicle`, unrelated to F.2).
- `cargo check --workspace`: **unbroken by F.2**. The new `astraweave-water` member compiles into the graph, and every physics consumer in F.2's blast radius compiles clean (`astraweave-gameplay`/`weaving.rs`, `physics_demo3d`, `fluids_demo`, `weaving_playground`). Two pre-existing, F.2-unrelated breaks remain in the workspace (see §9.1) — neither is touched by any F.2 commit.
- `cargo clippy -- -D warnings`: required a one-line fix to a **pre-existing** `noop_method_call` in `astraweave-physics/src/vehicle.rs` (a no-op `.clone()` on `&Vehicle`); fixed (in-scope crate, the compiler's own suggested removal, test still passes) so the gate is genuinely green rather than green-with-an-asterisk.

### 9.1 Pre-existing, water-unrelated test failure (surfaced, not silently passed)

`physics_core_tests.rs::character_controller_stays_on_ground` fails (`y=0.100000024`, expected ~0). It is **pre-existing and unrelated to F.2**:
- The test uses only `add_character` + `control_character` — **zero** water/buoyancy/fluid references (verified by reading it). My buoyancy rewire (`apply_buoyancy_forces`, impulse path) and the environment water-deletion touch no character-controller code.
- Confirmed pre-existing by `git stash` (fails identically on the pre-F.2 baseline).
- It is a character-controller ground-detection tolerance issue, **outside the F.2 water scope wall**. Per scope discipline it is **not fixed here** (fixing it is a separate character-controller investigation); it is handed to the owner as a standing pre-existing defect.

A second pre-existing break: `examples/llm_integration` fails `cargo check` (`DEFAULT_QWEN_INSTRUCT_MODEL` unresolved — the const exists at `astraweave-llm/src/qwen3_ollama.rs:50` but the example's import path is stale, from the recent Qwen-model work in the merged base). It is in the **LLM subsystem**, untouched by any F.2 commit and outside the F.2 scope wall — surfaced here, not fixed.

---

## 10. Deviations (none silent)

1. **#0** (base branch + missing `CAMPAIGN_PLAN.md`) — §Deviation #0.
2. **WaterQuery has one method, not the F.0-sketched four** — by derivation from need (§2.2); this is the intended discipline, recorded for the reader who expects the four.
3. **ECS: co-located in `PhysicsWorld`, not a separate resource** — reasoned in §4 (single consumer; avoids a second owner). Promotion to standalone resource is the F.3 move.
4. **Scalar `water_level`/`fluid_density` retained as a back-compat write-through input** rather than deleted outright — preserves all existing tests as unchanged regression evidence while keeping a single sampling owner (the facade). The fields are documented as facade inputs, not an independent store.
5. **`reset_forces` resolved by impulse semantics, not by adding a reset call** — §3.1 (avoids clearing other systems' forces).

---

## 11. Open items for F.3 / F.4

- **F.3 — voxel backend behind the proven trait.** `WaterQuery::sample` is the seam; `WaterVolumeGrid` (the deterministic CPU voxel sim) is the backend; promote the water resource out of `PhysicsWorld` when a second consumer (AI/gameplay) appears. Voxel sparsity is the gating perf item (64³ dense = 13.8 ms; budget option C depends on it).
- **F.4 — rendering + the F.1 ledger.** SSFR `draw_into` integration, per-particle color, the ocean converge-vs-extend decision, and the XSPH-viscosity exposure (the calm-pool blocker). Budget per the option ratified at this gate.
- **Swim** (`CharState::Swimming` + controller branch) — F.2-followup; `sample` already covers its needs.
- **`CAMPAIGN_PLAN.md`** — owner to create, formalizing the gate record (carried from F.1).
- Per-volume drag / flow / submersion-fraction — the analytic backend can compute them; they ship with the first consumer that reads them.

---

**Commit range** (actual hashes): `db1f4e6a2` (WI-1/2 facade crate) → `2d020ab1a` (WI-3 buoyancy rewire) → `73d214a59` (WI-3 env-water deletion + WI-5 bench + vehicle clippy fix) → this report.

**Revision history**

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-06-11 | F.2 execution: `astraweave-water` facade + `WaterQuery` derived from physics need; buoyancy rewired (impulse semantics); three abstractions retired; ECS co-location; determinism proof; budget re-ratification options for the gate |
