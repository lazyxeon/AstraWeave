# ParallelSchedule safety-model audit — 2026-04-18

> **Status note (2026-04-18)**: The `ParallelSchedule` scheduler analyzed in this audit was removed from the workspace on 2026-04-18, executing the Rev 2 §5.3 Framing Y recommendation ("execute Option E now"). See [`parallel_schedule_removal_2026-04-18.md`](parallel_schedule_removal_2026-04-18.md) for the deletion report and rationale. This audit remains in place as historical record describing the soundness hazards that motivated the removal; do not treat it as describing current engine state.

**Status**: Design audit, read-only. No code changes; Phase 5 recommends which option to *audit next*, not which to implement.
**Scope**: Full read of `astraweave-ecs` (6 209 lines across 8 files) plus 5 production call sites. Extension of the soundness finding in `docs/audits/parallel_schedule_experiment_ecs_ai_showcase_2026-04-18.md` §3.10.
**Precondition reading**: prior experiment report; `parallel.rs`, `lib.rs`, `archetype.rs`, `system_param.rs`, `component_meta.rs`, `blob_vec.rs`, `sparse_set.rs`, `entity_allocator.rs`. Where the prior report traced one aliasing path (change-ticks), this audit — after a second-pass adversarial review — extends it to **seven distinct shared-state races** across **six fix options**.

---

## Revision history

**Rev 1 — 2026-04-18 (am)**: Initial audit: 5 races, Options A-E, narrow recommendation "audit Option C further". First-pass audit.

**Rev 2 — 2026-04-18 (pm)**: Second-pass adversarial review via the `deep-reason` agent plus targeted re-verification of every factual claim in Rev 1. Changes:

- **Factual correction**: Option E's "zero production callers" was wrong. `examples/profiling_demo/src/main.rs:211` constructs a `ParallelSchedule::new()` behind the `parallel-schedule` feature. Verified via `rg 'ParallelSchedule::' --glob '!**/tests/**' --glob '!**/benches/**'`. Option E deletion list updated; effort estimate increased from 1 day to 2-4 days.
- **Two new races**: Race #8 (HashMap-inside-resource, e.g. `Events.queues`) and Race #9 (`EntityAllocator::next_id` / `generations`). Total race count rises from 5 to 7.
- **One new fix option**: Option F — scheduler dispatch change dispatching read-only systems via `&World` and the writer (if any) via `&mut World`. Not a usage restriction; a 4-7 day scheduler change that was wrongly dismissed in Rev 1's §3.1/§3.2.
- **Classification reframe**: Rev 1's empty "Single-task safe" bucket was a strict-reading artefact. Added a "Safe under perfect-scheduler" column to §1.3 showing that the API is fine; only the scheduler is broken.
- **Caveats tightened**: Option A's "closes Race #1" is conditional on archetype-stability during the parallel phase; per-column Vec growth during `add_entity_with_tick` reinvalidates atomic stores.
- **New verification path**: `loom` is already a dev-dep of `astraweave-ecs` (Cargo.toml:46) with 11 existing concurrency tests. Loom is strictly more powerful than Miri for `SendWorldPtr` analysis. Added §4.1.5.
- **Auxiliary finding**: CommandBuffer's "`!Send + !Sync`" doc comment at `command_buffer.rs:56` is unenforced (no `PhantomData<*const ()>` marker). Added §1.5.
- **Effort estimates**: tightened all five Rev 1 estimates to include CI, consumer migration, and doc work. Ranges grew ~50%.
- **Phase 5 framing**: added §5.3 explicitly comparing "audit Option C further" (Rev 1's pick) vs "execute Option E now" (alternative). Default switched to the latter when no near-term consumer exists.
- **Citation drift**: §1.3 said `combat_system` `get_resource_mut::<Events>()` is at `main.rs:417`; actual line is 433. Fixed.

All Revision-2 additions are marked **[R2]** where they appear inline. A full second-pass audit trail is at §7.

---

## Executive summary

The prior experiment identified one unsound path: `World::get_mut<T>` → `Archetype::stamp_change_tick<T>` → `&mut HashMap<TypeId, Vec<u32>>`. Deeper reading — **with second-pass adversarial review [R2]** — reveals **that is not the only one, and the fix surface is wider than the prior report implied**. Concretely, the current `ParallelSchedule` safety model — TypeId-disjoint access via `SystemAccess::conflicts_with` ([parallel.rs:97-113](../../astraweave-ecs/src/parallel.rs#L97-L113)) — is insufficient against at least these seven independently-unsound shared-state accesses:

| # | Shared state | Races under | Severity | Site |
|---|---|---|---|---|
| 1 | `Archetype::change_ticks: HashMap<TypeId, Vec<u32>>` | Two systems each calling `get_mut` or `each_mut` on the same archetype, disjoint TypeIds | **Observed** (prior experiment) | [archetype.rs:102](../../astraweave-ecs/src/archetype.rs#L102), [archetype.rs:336-344](../../astraweave-ecs/src/archetype.rs#L336-L344) |
| 2 | `Archetype::blob_components: Option<HashMap<TypeId, BlobVec>>` | Two systems both calling `archetype.get_mut` or `Query2Mut::next` on the same archetype, disjoint TypeIds | **Plausible, not observed** | [archetype.rs:86, 315-324](../../astraweave-ecs/src/archetype.rs#L86) |
| 3 | `ArchetypeStorage::next_id: u64` | Any two systems that cause a new archetype to be created (via `insert<T>` / `remove<T>` / `despawn`) | **Plausible** | [archetype.rs:545](../../astraweave-ecs/src/archetype.rs#L545), [archetype.rs:570-588](../../astraweave-ecs/src/archetype.rs#L570-L588) |
| 4 | `ArchetypeStorage::component_to_archetypes: HashMap<TypeId, Vec<ArchetypeId>>` | Any two archetype creations concurrent | **Plausible** | [archetype.rs:555](../../astraweave-ecs/src/archetype.rs#L555) |
| 5 | `ArchetypeStorage::signature_to_id: HashMap<ArchetypeSignature, ArchetypeId>` and `ArchetypeStorage::archetypes: BTreeMap<ArchetypeId, Archetype>` | Any two archetype creations concurrent | **Plausible** | [archetype.rs:547, 549](../../astraweave-ecs/src/archetype.rs#L547) |
| 6 | `World::resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>` | Two systems each calling `get_resource_mut<T>` for different `T` | **Likely contributing to observed divergence** | [lib.rs:123](../../astraweave-ecs/src/lib.rs#L123) |
| 7 | `World::change_tick: u32` | Two systems where any path calls `increment_change_tick` | **Plausible** | [lib.rs:134](../../astraweave-ecs/src/lib.rs#L134) |
| **8 [R2]** | **HashMap inside a resource value — e.g. `Events::queues: HashMap<TypeId, Box<dyn Any + Send + Sync>>`** | Two systems each calling `events.send::<DifferentT>()` after `get_resource_mut::<Events>()` | **Observed class — `ecs_ai_showcase` systems exercise this** | [events.rs:74-78](../../astraweave-ecs/src/events.rs#L74-L78) |
| **9 [R2]** | **`EntityAllocator::next_id: u32` (non-atomic) and `EntityAllocator::generations: Vec<u32>` (Vec growth)** | Two systems each calling `world.spawn()` or `world.despawn()` | **Plausible; Vec growth also invalidates concurrent `is_alive` readers** | [entity_allocator.rs:171-194, 238-257](../../astraweave-ecs/src/entity_allocator.rs#L171) |

Race #1 is the one the prior experiment caught. Races #3, #4, #5, #9 are only triggered by structural changes (spawn/despawn/insert/remove); systems that only mutate existing components don't hit them. `ecs_ai_showcase` doesn't spawn/despawn mid-tick, which is why the prior experiment surfaced only #1 (and likely #2, #6, #8 in the observed divergence). A system that spawns entities — common in physics (projectile spawn), VFX (particle spawn), or gameplay (enemy spawn) — would hit #3/#4/#5/#9 as soon as two such systems ran in the same parallel group.

Race #2 is interesting: it is *plausible* but does not require `stamp_change_tick`. Even a future version of the engine that eliminated change-tick stamping entirely would still have #2 latent on any pair of systems that mutate different components of the same archetype. So fixing #1 alone does not fix the class.

**Race #8 [R2]** is a second-pass finding that materially changes the threat picture. Two `ecs_ai_showcase` systems — `ai_planning_system` ([main.rs:285-291](../../examples/ecs_ai_showcase/src/main.rs#L285-L291)) and `combat_system` ([main.rs:433](../../examples/ecs_ai_showcase/src/main.rs#L433)) — both call `world.get_resource_mut::<Events>()` then `events.send::<DifferentEventType>()`. Even a perfect solution to Race #6 (outer `World::resources` HashMap) would leave Race #8 — `Events::queues` is another TypeId-keyed HashMap one level down, with identical race semantics. This is the strongest evidence that the problem is **structural** (HashMap-inside-HashMap-shared-state is everywhere in the crate), not specific to change-ticks.

**Race #9 [R2]** covers the `EntityAllocator` struct at [entity_allocator.rs:171-179](../../astraweave-ecs/src/entity_allocator.rs#L171-L179). Two fields — `next_id: u32` (non-atomic increment via `checked_add` at [line 245](../../astraweave-ecs/src/entity_allocator.rs#L245)) and `generations: Vec<u32>` (grown via `push(0)` at [line 249](../../astraweave-ecs/src/entity_allocator.rs#L249)) — are mutated on every `spawn()` and `despawn()`. The Vec growth is the more dangerous of the two: Vec reallocation invalidates any slice into `generations` that a concurrent `is_alive` reader (classified "Safe" in §1.3) might hold. Worth noting because §1.3's Safe bucket relies on "no concurrent writers" — exactly the contract `ParallelSchedule` cannot enforce today.

Fix options (§2) and safe-subset options (§3) follow. **[R2]** Rev 2 adds Option F (scheduler dispatch change; 4-7 days) which was wrongly dismissed as "not a usage restriction" in Rev 1's §3.1/§3.2. Option F closes all seven races for groups with **zero or one writer** by ensuring only one task ever holds `&mut World` at a time; it does not help groups with ≥2 writers (which must serialise under the current conflict check anyway). **[R2]** **No fix is recommended in this report — the purpose is to catalogue the cost of each, and §5 names which option to *audit* next.** Rev 1 recommended auditing Option C (column-level access primitives). Rev 2's §5.3 reframes this: given that `ParallelSchedule` has one opt-in caller (`profiling_demo` behind a feature flag) and zero default-features callers, the defensible default is to **execute Option E now** (1-2 days) and audit Option C only if a near-term ECS-parallelism consumer is committed.

---

## Phase 1 — Enumerate every aliasing path

### 1.1 Component-access API surface

Every public or pub(crate) method on `World`, `Archetype`, `ArchetypeStorage`, and the `Query*` types that touches component data, with the receiver, return type, archetype-level borrows taken, and whether `stamp_change_tick` fires. Rows keyed by method; duplicates merged. Roughly 55 rows across four tables.

#### 1.1.1 `World` component-access methods — `astraweave-ecs/src/lib.rs`

| Method | Line | Receiver | Returns | Archetype-level borrows | `stamp_change_tick`? | TypeId access |
|---|---|---|---|---|---|---|
| `spawn` | 189 | `&mut self` | `Entity` | Gets-or-creates empty archetype via `ArchetypeStorage::get_or_create_archetype` → mutably borrows `archetypes`, `signature_to_id`, `component_to_archetypes`, `next_id`. Then mutably borrows the archetype, calls `add_entity`. | No | None (empty signature) |
| `despawn` | 638 | `&mut self` | `bool` | `archetypes.get_archetype_mut(id)` → `archetype.remove_entity_components` which mutably borrows `entities`, `components` (all TypeIds in sig), `change_ticks` (all TypeIds in sig). | No | All types in entity's sig |
| `insert<T>` | 258 | `&mut self` | `()` | Old archetype: `get_archetype_mut` → `remove_entity_components`. Then `get_or_create_archetype(new_sig)` → mutates `signature_to_id`, `component_to_archetypes`, `next_id`, `archetypes`. Then new archetype: `add_entity_with_tick(_, tick)` — mutates `entities`, `components`, `change_ticks` per TypeId. | No (direct) — but `add_entity_with_tick` pushes to `change_ticks[T]` for every T in sig | Parameterised on `T`, but touches all types in sig during archetype transition |
| `remove<T>` | 600 | `&mut self` | `bool` | `move_entity_to_new_archetype(is_removing=true)`: same pattern as `insert`. | No direct | Parameterised on `T`, touches all types in sig |
| `get<T>` | 403 | `&self` | `Option<&T>` | `get_archetype(id)` (shared borrow) → `archetype.get::<T>(e)` → reads `entity_index`, reads `blob_components[T]` (via `as_ref` + `get`) or reads `components[T]`. | No | Single `T` |
| `get_mut<T>` | 434 | `&mut self` | `Option<&mut T>` | `get_archetype_mut(id)` → `archetype.stamp_change_tick::<T>(e, tick)` mutably borrows `change_ticks.get_mut(&TypeId::of::<T>())`. Then `archetype.get_mut::<T>(e)` mutably borrows `blob_components.get_mut(&TypeId::of::<T>())` (or the legacy `components`). | **Yes** | Single `T` — but two distinct `T`s on same archetype alias on `change_ticks` and `blob_components` HashMaps |
| `each_mut<T>` | 508 | `&mut self` | `()` | Collects `archetypes_with_component(T)` IDs, then loop: `get_archetype_mut(id)` → `stamp_change_tick::<T>` + `get_mut::<T>` per entity. Same archetype-level borrows as `get_mut` but repeated. | **Yes** | Single `T` — same aliasing class as `get_mut` |
| `each_changed<T>` | 572 | `&self` | `()` | `archetypes_with_component(T)` → `archetype.get_change_tick::<T>` + `archetype.get::<T>`. All read-only. | No | Single `T` |
| `count<T>` | 533 | `&self` | `usize` | `archetypes_with_component(T).map(len).sum()`. Read-only. | No | Single `T` |
| `has<T>` | 540 | `&self` | `bool` | `is_alive` + `get::<T>`. Read-only. | No | Single `T` |
| `entities_with<T>` | 548 | `&self` | `Vec<Entity>` | `archetypes_with_component(T).flat_map(entities_vec).collect`. Read-only (allocates for the return Vec). | No | Single `T` |
| `get_change_tick<T>` | 591 | `&self` | `Option<u32>` | `is_alive` + `get_entity_archetype` + `get_archetype` + `archetype.get_change_tick::<T>`. Read-only. | No | Single `T` |
| `change_tick` | 159 | `&self` | `u32` | Reads scalar field. | No | None |
| `increment_change_tick` | 168 | `&mut self` | `u32` | Writes scalar field. | No | None |
| `is_alive` | 230 | `&self` | `bool` | Reads `entity_allocator` only. | No | None |
| `is_component_registered_blob<T>` | 239 | `&self` | `bool` | Reads `component_registry`. | No | Single `T` |
| `register_component<T>` | 826 | `&mut self` | `()` | Writes `type_registry`, `component_registry`. No archetype touch. | No | Single `T` |
| `insert_resource<T>` | 465 | `&mut self` | `()` | Writes `self.resources[TypeId::of::<T>()]`. `Box<dyn Any + Send + Sync>` inserted. | No | Single `T` |
| `get_resource<T>` | 481 | `&self` | `Option<&T>` | Reads `self.resources`. | No | Single `T` |
| `get_resource_mut<T>` | 485 | `&mut self` | `Option<&mut T>` | Mutably borrows `self.resources`. | No | Single `T` — but two distinct `T`s alias on the `resources` HashMap |
| `archetypes` | 681 | `&self` | `&ArchetypeStorage` | Returns shared borrow; caller decides what to do next. | No | None |
| `entity_count` | 660 | `&self` | `usize` | Reads `entity_allocator.alive_count()`. | No | None |

Internal helper (not public but exposed indirectly):

| Method | Line | Receiver | Archetype-level borrows |
|---|---|---|---|
| `move_entity_to_new_archetype` | 314 | `&mut self` | Old + new archetype mut borrows; `get_or_create_archetype` on new signature. |

#### 1.1.2 `Archetype` methods — `astraweave-ecs/src/archetype.rs`

| Method | Line | Receiver | Returns | Archetype-level borrows | TypeId access |
|---|---|---|---|---|---|
| `new` | 110 | `Self` | `Archetype` | Ctor; initializes fields. | — |
| `new_with_blob` | 137 | `Self` | `Archetype` | Ctor. | — |
| `uses_blob` | 166 | `&self` | `bool` | Reads `self.uses_blob` scalar. | None |
| `add_entity` | 172 | `&mut self` | `()` | Delegates to `add_entity_with_tick(0)`. | All types in sig |
| `add_entity_with_tick` | 182 | `&mut self` | `()` | Mutates `entity_index`, `entities`, `components` (per TypeId loop), `change_ticks` (per TypeId loop). | All types in sig |
| `push_component_typed<T>` | 221 | `&mut self` | `()` | Delegates to `push_component_typed_with_tick(_, 0)`. | Single `T` |
| `push_component_typed_with_tick<T>` | 226 | `&mut self` | `()` | Mutates `blob_components.get_mut(&T_id)` and `change_ticks.get_mut(&T_id)`. | Single `T` but via HashMap `get_mut` |
| `add_entity_typed_raw` | 253 | `&mut self` | `()` | Delegates to the `_with_tick` variant. | Multiple runtime TypeIds |
| `add_entity_typed_raw_with_tick` | 258 | `&mut self` | `()` | Mutates `entity_index`, `entities`, `blob_components`, `change_ticks`. | Multiple runtime TypeIds |
| `get<T>` | 297 | `&self` | `Option<&T>` | Reads `entity_index`, `blob_components.as_ref()?.get(&T_id)`, or `components.get(&T_id)`. | Single `T` |
| `get_mut<T>` | 315 | `&mut self` | `Option<&mut T>` | Reads `entity_index`, mutably borrows `blob_components.as_mut()?.get_mut(&T_id)`, or mutably borrows `components.get_mut(&T_id)`. **Does NOT call `stamp_change_tick`.** | Single `T` — but mutably borrows `blob_components` which is shared |
| `stamp_change_tick<T>` | 336 | `&mut self` | `()` | Reads `entity_index`, mutably borrows `change_ticks.get_mut(&T_id)` → `ticks.get_mut(row)`. | Single `T` — but mutably borrows `change_ticks` |
| `stamp_change_tick_by_type` | 348 | `&mut self` | `()` | Same as above, but with runtime `TypeId`. | Runtime `TypeId` |
| `get_change_tick<T>` | 362 | `&self` | `Option<u32>` | Read-only. | Single `T` |
| `remove_entity` | 369 | `&mut self` | `Option<usize>` | Mutates `entity_index` only. | None |
| `remove_entity_components` | 375 | `&mut self` | `HashMap` | Mutates `entity_index`, `entities` (swap_remove), `components` (all columns), `change_ticks` (all columns). | All types in sig |
| `len` | 411 | `&self` | `usize` | Reads `entities.len()`. | None |
| `is_empty` | 415 | `&self` | `bool` | Reads `entities.is_empty()`. | None |
| `entities_vec` | 420 | `&self` | `&[Entity]` | Returns shared borrow of `entities`. | None |
| `iter_components<T>` | 460 | `&self` | `impl Iterator<Item=(Entity, &T)>` | Lazily reads `components.get(&T_id)`, iterates. Legacy (Box) path only. | Single `T` |
| `iter_components_blob<T>` | 488 | `&self` | `Option<(&[Entity], &[T])>` | Reads `uses_blob`, returns `&[Entity]` and `&[T]` as shared slices into `blob_components[T_id]`. | Single `T` |
| `iter_components_blob_mut<T>` | 507 | `&mut self` | `Option<(&[Entity], &mut [T])>` | Mutably borrows `blob_components.as_mut()?.get_mut(&T_id)` for the BlobVec, returns `&mut [T]` slice. | Single `T` — but mutably borrows the `blob_components` HashMap |

#### 1.1.3 `ArchetypeStorage` methods — `astraweave-ecs/src/archetype.rs`

| Method | Line | Receiver | Returns | Shared-state mutations |
|---|---|---|---|---|
| `get_or_create_archetype` | 570 | `&mut self` | `ArchetypeId` | Reads `signature_to_id`; if miss: writes `next_id`, `component_to_archetypes` (per TypeId in sig), `archetypes`, `signature_to_id`. |
| `get_or_create_archetype_with_blob` | 598 | `&mut self` | `ArchetypeId` | Same pattern as above, plus builds `blob_components` via `create_blob_vec`. |
| `get_archetype` | 624 | `&self` | `Option<&Archetype>` | Reads `archetypes`. |
| `get_archetype_mut` | 628 | `&mut self` | `Option<&mut Archetype>` | Mutably borrows `archetypes` (returns `&mut Archetype`). |
| `get_entity_archetype` | 634 | `&self` | `Option<ArchetypeId>` | Reads `entity_to_archetype` Vec. |
| `set_entity_archetype` | 640 | `&mut self` | `()` | Writes `entity_to_archetype` (may grow). |
| `remove_entity` | 651 | `&mut self` | `Option<ArchetypeId>` | Writes `entity_to_archetype[id]`. |
| `archetypes` | 661 | `&self` | iterator | Reads `archetypes`. |
| `iter` | 666 | `&self` | iterator | Alias. |
| `archetypes_mut` | 671 | `&mut self` | iterator | Mutably borrows `archetypes`. |
| `archetypes_with_component` | 679 | `&self` | iterator | Reads `component_to_archetypes` + `archetypes`. |

#### 1.1.4 `Query*` types — `astraweave-ecs/src/system_param.rs`

| Type / Method | Line | Receiver | Archetype-level borrows |
|---|---|---|---|
| `Query<'w, T>::new(world: &'w World)` | 102 | `&'w World` | Stores `*const World`, collects archetype IDs once at construction. |
| `Query::next` | 121 | `&mut self` | Reads World via `&*self.world`, calls `archetype.get::<T>`. Read-only access path. |
| `Query2<'w, A, B>::new(world: &'w World)` | 166 | `&'w World` | Stores archetype IDs filtered by both A and B. |
| `Query2::next` | 187 | `&mut self` | Reads World, calls `archetype.get::<A>` and `archetype.get::<B>`. Read-only. |
| `Query2Mut<'w, A, B>::new(world: &'w mut World)` | 239 | `&'w mut World` | Stores `*mut World` ([system_param.rs:231](../../astraweave-ecs/src/system_param.rs#L231)). |
| `Query2Mut::next` | 260 | `&mut self` | Reconstructs `&mut World` via `unsafe { &mut *self.world }` at [line 274](../../astraweave-ecs/src/system_param.rs#L274), calls `world.archetypes.get_archetype_mut(id)` at [278](../../astraweave-ecs/src/system_param.rs#L278), then `archetype.get_mut::<A>` ([line 293](../../astraweave-ecs/src/system_param.rs#L293)) and `archetype.get::<B>` ([line 298](../../astraweave-ecs/src/system_param.rs#L298)). Crucially: this bypasses `World::get_mut` and therefore **does not** call `stamp_change_tick`. But it still mutably borrows `archetype.blob_components.get_mut(&TypeId::of::<A>())`. |

### 1.2 Production usage of each method

Cross-reference of the §1.1 rows against the five production call sites.

| Method | ecs_adapter | ecs_ai_plugin | ecs_ai_showcase | profiling_demo | hello_companion | Used in prod? |
|---|---|---|---|---|---|---|
| `spawn` | via setup | ✓ | ✓ | ✓ | ✓ | **Yes** |
| `despawn` | — | — | — | — | — | No in surveyed systems |
| `insert<T>` | `sys_ai_planning` writes `CDesiredPos`; `sys_bridge_sync` writes `CLegacyId` | ✓ | Setup only | Setup only | Setup only | **Yes** (mid-tick) |
| `remove<T>` | — | — | — | — | — | No |
| `get<T>` | ✓ (`sys_move`, `sys_sync_to_legacy`) | ✓ (several) | ✓ (all six systems, various) | ✓ (movement) | — | **Yes** |
| `get_mut<T>` | — (none in ecs_adapter; `each_mut` is used instead) | — | ✓ `world.get_mut::<AIAgent>` in ai_perception/ai_planning/ai_behavior; `world.get_mut::<Velocity>` in ai_behavior; `world.get_mut::<Position>` in movement; `world.get_mut::<Health>` in combat | ✓ `world.get_mut::<AIAgent>` in ai_planning_system | — | **Yes** (this is the path the prior experiment caught) |
| `each_mut<T>` | ✓ `sys_sim` on `CCooldowns`; `sys_move` on `CPos` | — | — | — | — | **Yes** |
| `each_changed<T>` | — | — | — | — | — | No in surveyed systems |
| `get_change_tick<T>` | — | — | — | — | — | No (public, unused) |
| `get_resource<T>` | ✓ `sys_sim`, `sys_refresh_los`, `sys_sync_to_legacy` | ✓ | ✓ | ✓ | — | **Yes** |
| `get_resource_mut<T>` | ✓ (legacy World; Events) | ✓ (Events, EntityBridge) | ✓ (Events, GameStats) | ✓ (GameTime) | — | **Yes** |
| `insert_resource<T>` | ✓ (setup: Dt, Events, EntityBridge, World) | ✓ (Events) | ✓ (GameTime, GameStats, Events) | ✓ (setup) | — | **Yes (setup)** |
| `register_component<T>` | — | — | — | — | — | No |
| `count<T>` | — | — | — | — | — | No |
| `has<T>` | — | — | — | — | — | No |
| `entities_with<T>` | — | — | ✓ `ai_perception_system` uses it; `ai_planning_system`; `ai_behavior_system`; `movement_system`; `combat_system` | — | — | **Yes** |
| `archetype.iter_components_blob<T>` | — | — | — | — | — | No |
| `archetype.iter_components_blob_mut<T>` | — | — | — | — | — | No |
| `Query::<T>` | ✓ (`sys_move` on `CDesiredPos`; `sys_sync_to_legacy` on `CLegacyId`) | ✓ (`CPos`, `CTeam`) | — (uses `entities_with`) | ✓ `Query2<Position, AIAgent>` used | — | **Yes** |
| `Query2::<A,B>` | — | — | — | ✓ `profiling_demo::ai_perception_system`, `ai_planning_system`, `physics_system`, `rendering_system` | — | **Yes** |
| `Query2Mut::<A,B>` | — | — | — | ✓ `profiling_demo::movement_system` uses `Query2Mut<Position, Velocity>` | — | **Yes** |

Net production footprint — the methods that every future parallel-schedule adopter will hit:

- `get_mut<T>` (direct, UB-prone, fires `stamp_change_tick`).
- `each_mut<T>` (direct, UB-prone, fires `stamp_change_tick`).
- `get_resource_mut<T>` (mutably borrows `World::resources` HashMap, aliases on `TypeId`).
- `insert<T>` (UB-prone via `get_or_create_archetype`).
- `Query2Mut<A, B>::next` (mutably borrows `archetype.blob_components` HashMap, does NOT fire `stamp_change_tick`).

### 1.3 Classification

Classification per the brief's four-bucket taxonomy. Every UB-prone entry includes the written-out borrow chain.

#### Safe (`&self`, read-only, multi-task concurrent-safe)

`get`, `each_changed`, `count`, `has`, `entities_with`, `get_change_tick`, `get_resource`, `archetypes`, `is_alive`, `is_component_registered_blob`, `change_tick`, `entity_count`, `archetype.get`, `archetype.get_change_tick`, `archetype.entities_vec`, `archetype.len`, `archetype.is_empty`, `archetype.uses_blob`, `archetype.iter_components`, `archetype.iter_components_blob`, `ArchetypeStorage::get_archetype`, `ArchetypeStorage::get_entity_archetype`, `ArchetypeStorage::archetypes_with_component`, `Query::new`, `Query::next`, `Query2::new`, `Query2::next`.

(22 methods / 4 Query entries — everything reachable via `&World`.)

#### Single-task safe (`&mut self`, operates only on single-TypeId-scoped storage)

None, strictly. Every `&mut self` method either mutably borrows shared metadata (change-ticks, the component HashMap, storage counters) or delegates to a method that does. In particular:

- `register_component<T>` is single-task safe if considered in isolation (it writes `type_registry` and `component_registry`, not archetype metadata). But the registry is shared across all `T`, so two concurrent registrations for different `T` would alias on the registry HashMap.
- `increment_change_tick` operates on a scalar field of `World` (not a HashMap). Structurally safe for a single task but aliased across `T` if two tasks call it (which neither our systems nor `Schedule` do).
- `archetype.iter_components_blob_mut<T>` takes `&mut self` and mutably borrows `blob_components.get_mut(&T_id)`. In isolation on a single archetype it's fine. Across tasks mutating different TypeIds of the same archetype, it is UB-prone — see Race #2 in §1.4.

So the "Single-task safe" bucket is empty in the strict sense this audit requires. Any `&mut self` method on `World` is UB-prone if two rayon tasks concurrently hold `&mut World` and call it for overlapping archetypes. The prior experiment's `SendWorldPtr` + TypeId-disjoint check is insufficient for any of them.

#### Safe under a perfect scheduler **[R2]**

The Rev 1 classification above is technically correct but analytically misleading: it implies the problem is with the `World` / `Archetype` / `Query` **APIs**, when in fact the APIs are all fine if exactly one task ever holds `&mut World` at a time. Under a hypothetical scheduler that enforces:

1. Only one task ever holds `&mut World` concurrently; **and**
2. Read-only tasks receive `&World` (shared borrow) rather than `&mut World` via `SendWorldPtr`;

**then every method in the above UB-prone list becomes safe**:

- `get_mut<T>`, `each_mut<T>`, `insert<T>`, `remove<T>`, `despawn`, `spawn`, `get_resource_mut<T>`, `Query2Mut::next` — all safe because only one task holds `&mut World`.
- `get<T>`, `each_changed<T>`, `count<T>`, `has<T>`, `entities_with<T>`, `Query::next`, `Query2::next`, and every resource read — all safe under shared borrow.

Races #1 through #9 are all closed by this hypothetical scheduler. None require API surgery.

The significance: **the fix target is the scheduler, not the APIs**. Rev 1 proposed five options, four of which (A, B, C, D) modify the World/Archetype/BlobVec API in various ways. Rev 2 adds Option F (§2.6), which is instead a ~50-100 line scheduler dispatch change that realises the hypothetical scheduler above: partition each group into readers (`fn(&World)`) and at most one writer (`fn(&mut World)`), dispatch readers in parallel via rayon::scope with `&World`, run the writer alone. Effort: 4-7 days. Caveats in §2.6.

#### UB-prone (`&mut self` that mutably borrows archetype-level shared metadata)

**`World::get_mut<T>`** at [lib.rs:434-447](../../astraweave-ecs/src/lib.rs#L434-L447). Borrow chain:

1. `let tick = self.change_tick;` — read.
2. `let archetype_id = self.archetypes.get_entity_archetype(e)?;` — read of `ArchetypeStorage.entity_to_archetype`.
3. `let archetype = self.archetypes.get_archetype_mut(archetype_id)?;` — `&mut ArchetypeStorage.archetypes` (BTreeMap). Two concurrent tasks here each return `&mut Archetype` for the same `archetype_id` ⇒ **two `&mut` on the same BTreeMap entry**.
4. `archetype.stamp_change_tick::<T>(e, tick);` — walks to `change_ticks.get_mut(&TypeId::of::<T>())` ⇒ **`&mut HashMap<TypeId, Vec<u32>>` twice, different keys**.
5. `archetype.get_mut::<T>(e)` — walks to `blob_components.as_mut()?.get_mut(&TypeId::of::<T>())` ⇒ **`&mut HashMap<TypeId, BlobVec>` twice, different keys**.

Steps 3, 4, and 5 are each an independent aliasing violation under Rust's strict-aliasing model. The prior experiment observed determinstic state divergence consistent with step 4 (change-ticks). Steps 3 and 5 may contribute to the observed divergence; distinguishing them would require Miri.

**`World::each_mut<T>`** at [lib.rs:508-531](../../astraweave-ecs/src/lib.rs#L508-L531). Same shape as `get_mut`, repeated per entity. Additional aliasing:

- The initial `archetypes.archetypes_with_component(T)` returns an iterator borrowing from `self.archetypes`; then the method collects IDs into a `Vec<ArchetypeId>` and later calls `get_archetype_mut` in the loop. This pattern is fine within a single task but compounds the aliasing hazard in concurrent use.

**`World::insert<T>`** at [lib.rs:258-311](../../astraweave-ecs/src/lib.rs#L258-L311). Borrow chain:

1. Collect entity's current components from old archetype (`archetypes.get_archetype_mut(old_id)` → `archetype.remove_entity_components(e)` mutably borrows `entities`, `components`, `change_ticks`).
2. Call `self.archetypes.get_or_create_archetype(new_sig)` ([archetype.rs:570-588](../../astraweave-ecs/src/archetype.rs#L570-L588)):
   - Reads `signature_to_id`.
   - If miss: writes `next_id` (plain `u64`, non-atomic — see Race #3), `component_to_archetypes` entry per TypeId in sig (Race #4), `archetypes` BTreeMap, `signature_to_id`.
3. Insert into new archetype via `get_archetype_mut` + `add_entity_with_tick`.
4. `self.archetypes.set_entity_archetype(e, new_id)` writes `entity_to_archetype` Vec.

Any two tasks that each cause new archetype creation would race on `next_id` (a plain `u64` increment with no atomic or lock — [archetype.rs:575-576](../../astraweave-ecs/src/archetype.rs#L575-L576)). This is UB even before getting to HashMap races.

**`World::remove<T>`** at [lib.rs:600-617](../../astraweave-ecs/src/lib.rs#L600-L617). Delegates to `move_entity_to_new_archetype` which has the same `get_or_create_archetype` path. Same UB as `insert`.

**`World::despawn`** at [lib.rs:638-657](../../astraweave-ecs/src/lib.rs#L638-L657). `get_archetype_mut` + `remove_entity_components` mutates `entities`, `components` (all TypeIds in sig via loop), `change_ticks` (all TypeIds in sig via loop), plus `entity_allocator`. Two concurrent despawns on entities in the same archetype race on all four.

**`World::get_resource_mut<T>`** at [lib.rs:485-487](../../astraweave-ecs/src/lib.rs#L485-L487). Borrow chain:

- `self.resources.get_mut(&TypeId::of::<T>())?.downcast_mut()` mutably borrows the `World.resources: HashMap<TypeId, Box<dyn Any>>`. Two concurrent calls for different `T` ⇒ concurrent `&mut` on the same HashMap, different keys. UB regardless of which `T` each task targets.

This matters for `ecs_ai_showcase` specifically: `combat_system` calls `world.get_resource_mut::<Events>()` ([main.rs:433](../../examples/ecs_ai_showcase/src/main.rs#L433)) **[R2 — Rev 1 cited line 417 in error; 417 is combat's `get_mut::<Health>` call, not `Events`]** and `world.get_resource_mut::<GameStats>()` (one line later) while in the same parallel group as `movement_system` which uses `world.get_resource::<GameTime>()` ([main.rs:287](../../examples/ecs_ai_showcase/src/main.rs#L287)) — shared borrow of `resources`, aliased against combat's mutable borrows. Another UB path the prior experiment did not distinguish.

**[R2]** The `Events` resource itself contains Race #8. Drilling one level deeper: `world.get_resource_mut::<Events>()` hands out `&mut Events` — a newtype whose fields at [events.rs:74-78](../../astraweave-ecs/src/events.rs#L74-L78) include `queues: HashMap<TypeId, Box<dyn Any + Send + Sync>>`. Calling `events.send::<E>(event)` mutably borrows `queues` via `entry().or_insert_with(...)`. Two systems both calling `events.send::<DifferentEventType>()` after `get_resource_mut::<Events>()` hit the exact same HashMap-shared-across-TypeIds pattern as Race #1 (change-ticks) and Race #2 (blob_components). In the `ecs_ai_showcase` failing case, `ai_planning_system` sends `AIStateChangedEvent` at [main.rs:285-291](../../examples/ecs_ai_showcase/src/main.rs#L285-L291) and `combat_system` sends `HealthChangedEvent` at [main.rs:433+](../../examples/ecs_ai_showcase/src/main.rs#L433) — if those ever land in the same parallel group, the `Events::queues` HashMap is the race site. The lesson generalises: **any Rust library resource whose public API writes to an internal HashMap is an additional race class.**

**`World::spawn`** at [lib.rs:189-227](../../astraweave-ecs/src/lib.rs#L189-L227). Writes `entity_allocator`, may call `get_or_create_archetype` for the empty signature (on first spawn), calls `archetype.add_entity`. UB-prone for similar reasons to `insert`, though in practice every binary spawns during setup only, not mid-tick.

**`Query2Mut::next`** at [system_param.rs:260-307](../../astraweave-ecs/src/system_param.rs#L260-L307). The `SAFETY` comment at [lines 267-273](../../astraweave-ecs/src/system_param.rs#L267-L273) says "A and B are different types so their columns are disjoint memory" — that claim is true of the data, but the *access path* goes through `self.archetypes.get_archetype_mut(id)` and then `archetype.get_mut::<A>(e)` which mutably borrows `blob_components.get_mut(&TypeId::of::<A>())`. Two concurrent `Query2Mut` iterators on the same archetype (e.g. `Query2Mut<Position, Velocity>` and `Query2Mut<Health, Team>`) would each acquire `&mut ArchetypeStorage.archetypes`, then `&mut Archetype`, then `&mut Archetype.blob_components`. The data-level disjointness is real but the access-path aliasing is real too. UB regardless of the `stamp_change_tick` question because `Query2Mut` bypasses that path.

#### Unclear

- **`Archetype::iter_components_blob_mut<T>`** is UB-prone when invoked from two concurrent `&mut World`s via `world.archetypes.get_archetype_mut(same_id)`. Whether a single-`&mut World`, single-archetype caller who holds the returned `&mut [T]` across an internal await point would see any UB is out of scope — the Query path doesn't use it and no production system does.
- **`spawn` mid-tick**: empty-signature archetype is created once and reused; after warmup `get_or_create_archetype` with empty sig is a read-only HashMap lookup. No production system spawns mid-tick so this is latent. *Marking unclear pending a concrete benchmark that creates new archetypes at steady state.*

### 1.4 Is this specific to `change_ticks`?

No. The change-ticks HashMap is **one of five** archetype-level shared-state races; each is independently UB-prone. Categorised:

**Per-archetype shared state**:

1. **`Archetype::change_ticks: HashMap<TypeId, Vec<u32>>`** ([archetype.rs:102](../../astraweave-ecs/src/archetype.rs#L102)). Mutated by `stamp_change_tick<T>`, `stamp_change_tick_by_type`, `add_entity_with_tick`, `remove_entity_components`. Reached from production code via `World::get_mut`, `World::each_mut`, `World::insert`, `World::remove`, `World::despawn`. The prior experiment's observed race.
2. **`Archetype::blob_components: Option<HashMap<TypeId, BlobVec>>`** ([archetype.rs:86](../../astraweave-ecs/src/archetype.rs#L86)). Mutated by `Archetype::get_mut`, `Archetype::push_component_typed_with_tick`, `Archetype::add_entity_typed_raw_with_tick`, `Archetype::iter_components_blob_mut`. Reached via `World::get_mut`, `World::each_mut`, `World::insert`, `Query2Mut::next`. Independent of change-ticks; would remain UB-prone after any hypothetical change-ticks fix.
3. **`Archetype::components: HashMap<TypeId, Vec<Box<dyn Any>>>`** ([archetype.rs:78](../../astraweave-ecs/src/archetype.rs#L78)). Same class as #2 but for the legacy Box storage path. Mutated identically. (Hidden behind `uses_blob` branch in `get_mut` / `get`.)
4. **`Archetype::entities: Vec<Entity>`** ([archetype.rs:71](../../astraweave-ecs/src/archetype.rs#L71)) and **`Archetype::entity_index: SparseSet`** ([archetype.rs:74](../../astraweave-ecs/src/archetype.rs#L74)). Mutated by `add_entity_with_tick`, `remove_entity_components`, `remove_entity`. Concurrent adds/removes on the same archetype race; concurrent reads (via `entities_vec`) alongside concurrent writes also race. Reached via `World::spawn`, `World::insert`, `World::remove`, `World::despawn`.

**Per-storage (shared across all archetypes)**:

5. **`ArchetypeStorage::next_id: u64`** ([archetype.rs:545](../../astraweave-ecs/src/archetype.rs#L545)). **Plain `u64`, non-atomic.** Incremented at [archetype.rs:575-576](../../astraweave-ecs/src/archetype.rs#L575-L576) and [archetype.rs:609-610](../../astraweave-ecs/src/archetype.rs#L609-L610). Concurrent increments without `AtomicU64::fetch_add` are a textbook data race even before considering anything else.
6. **`ArchetypeStorage::signature_to_id: HashMap<ArchetypeSignature, ArchetypeId>`** ([archetype.rs:547](../../astraweave-ecs/src/archetype.rs#L547)), **`archetypes: BTreeMap<ArchetypeId, Archetype>`** ([archetype.rs:549](../../astraweave-ecs/src/archetype.rs#L549)), **`entity_to_archetype: Vec<Option<ArchetypeId>>`** ([archetype.rs:552](../../astraweave-ecs/src/archetype.rs#L552)), **`component_to_archetypes: HashMap<TypeId, Vec<ArchetypeId>>`** ([archetype.rs:555](../../astraweave-ecs/src/archetype.rs#L555)). All mutated by `get_or_create_archetype` and `set_entity_archetype`. Concurrent archetype creation races on each of these structures independently.

**Per-World**:

7. **`World::resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>`** ([lib.rs:123](../../astraweave-ecs/src/lib.rs#L123)). Mutated by `get_resource_mut`. Two concurrent `get_resource_mut` calls for different `T` alias on this HashMap.
8. **`World::change_tick: u32`** ([lib.rs:134](../../astraweave-ecs/src/lib.rs#L134)). Mutated by `increment_change_tick`. Scalar, but not atomic — concurrent `&mut` violations the same as any other field.

**Conclusion for §1.4**: the bug is *not* change-ticks-specific. A minimal fix targeting only change-ticks (§2.1 Option A) closes the path the prior experiment observed but leaves six other independently-unsound paths (five in Rev 1 + Race #8 and Race #9 added by Rev 2 **[R2]**). The fix surface for genuine soundness is the full list above.

### 1.5 CommandBuffer — a mis-declared thread-safety boundary **[R2]**

`astraweave-ecs/src/command_buffer.rs` provides a deferred-mutation API: systems queue `spawn` / `insert` / `despawn` commands into a `CommandBuffer` and flush them at a checkpoint. This is the canonical "safe-during-iteration" pattern in ECS design.

The doc comment at [command_buffer.rs:55-56](../../astraweave-ecs/src/command_buffer.rs#L55-L56) states:

```rust
/// # Thread Safety
/// CommandBuffer is `!Send + !Sync` to match World's single-threaded access model.
```

**The doc comment is not enforced.** The struct at [command_buffer.rs:57-60](../../astraweave-ecs/src/command_buffer.rs#L57-L60) is:

```rust
pub struct CommandBuffer {
    commands: Vec<Command>,
    spawn_buffer: Vec<(TypeId, Box<dyn Any + Send + Sync>)>,
}
```

Both fields are `Send + Sync`. There is no `PhantomData<*const ()>` marker to opt out of the auto-implementation. Rust will auto-derive `Send + Sync` for this struct, contradicting the doc comment.

Implications for this audit:

- No production system currently uses `CommandBuffer` for structural mutation (verified against the 5 call sites in §1.2). So the mis-declaration is latent.
- Any future fix that relies on `CommandBuffer` to defer structural mutation out of parallel groups (a natural migration pattern under Options A, B, or F) inherits whatever concurrency model CommandBuffer actually has, not what the doc comment claims.
- At `flush(&mut World)`, the buffered operations are applied sequentially to the World. If `CommandBuffer` were actually `!Send + !Sync`, systems couldn't build up buffers on parallel worker threads and then ship them back for flushing. Today they can, but the resulting `flush` is just as unsound as `World::insert<T>` called directly (the same Races #3/#4/#5/#9 fire).

Recommendation (for any future fix): decide whether `CommandBuffer` is genuinely per-thread (add the `PhantomData` marker to match the doc) or cross-thread-send-safe (delete the "!Send + !Sync" claim from the doc). The current state is the worst of both worlds.

**Not fixing in this audit per scope.**

---

## Phase 2 — Fix options

For each option: does it close the UB hole, what's the code scope, what does it cost in Miri/Kani coverage, what's the performance risk on the sequential path, how much effort.

### 2.1 Option A: atomicize change-ticks

**Hypothesis**: replace `Archetype::change_ticks: HashMap<TypeId, Vec<u32>>` with a structure that doesn't require `&mut HashMap` for per-row writes. Concretely, make each column's tick-vector either `Vec<AtomicU32>` or store it alongside `BlobVec`/`Vec<Box<Any>>` directly (co-locating the data and its tick).

**Does it close the UB hole?** Walking the `movement_system` ↔ `combat_system` scenario: Task 1 calls `world.get_mut::<Position>(e)`; Task 2 calls `world.get_mut::<Health>(e)` on the same archetype:

- Step 3 of the borrow chain (`get_archetype_mut`) still produces two `&mut Archetype` — **still UB** (Race #1 partial close; Race #2 still open).
- Step 4 (`stamp_change_tick`) — if the atomic-tick structure replaces the HashMap with per-column `Vec<AtomicU32>` co-located with each BlobVec, the write is `atomics.get(row).fetch_store(tick, Relaxed)` — no `&mut` on shared data. **Race #1 closed *under a crucial caveat* (below)**.
- Step 5 (`get_mut::<T>` on `blob_components.get_mut`) — **still UB** (Race #2).

Net: Option A closes one of the three UB steps in the `get_mut` borrow chain. The other two (`get_archetype_mut` on same ID, `blob_components.get_mut`) remain. Option A by itself does not make `ecs_ai_showcase`'s scenario sound.

**[R2] Caveat — Vec growth invalidates atomic stores**. Under Option A, per-column change-ticks would be `Vec<AtomicU32>`. But `add_entity_with_tick` ([archetype.rs:182-206](../../astraweave-ecs/src/archetype.rs#L182-L206)) calls `self.change_ticks.get_mut(ty).unwrap().push(tick)` — and `Vec::push` can reallocate. If a concurrent task is mid-atomic-store on row N when the Vec reallocates, the store targets freed memory (use-after-free / data race on the reallocation). So Option A's "Race #1 closed" claim is valid **only for archetype-stable parallel phases** — phases in which no entity is added, removed, or transitioned mid-phase. `ecs_ai_showcase`'s simulation stage is archetype-stable (no spawn/insert/remove mid-tick), so Option A works there. A hypothetical physics stage that spawns projectiles mid-tick would not be archetype-stable, and Option A would leave a race on the reallocation. Tightening: Option A must pair with a rule that "no system in a parallel group may cause archetype mutation" — which is itself a scheduler-enforcement problem.

**Code scope**: three files, maybe 200 lines of diff.

- `archetype.rs`: change `change_ticks` field type and all the methods that read/write it (`stamp_change_tick`, `stamp_change_tick_by_type`, `get_change_tick`, `add_entity_with_tick`, `remove_entity_components`, `push_component_typed_with_tick`, `add_entity_typed_raw_with_tick`).
- `lib.rs`: update `get_mut`, `each_mut`, `each_changed` to use the new API (most call sites are one-liners).
- Tests in `archetype.rs` and the mutation-resistance tests that assert on change-tick behaviour ([mutation_resistance_tests.rs](../../astraweave-ecs/src/mutation_resistance_tests.rs)).

**Kani/Miri coverage**: the existing Kani proofs at [blob_vec_kani.rs](../../astraweave-ecs/src/blob_vec_kani.rs) cover BlobVec layout invariants; `change_ticks` isn't in the current proof set ([lib.rs:1557](../../astraweave-ecs/src/lib.rs) line count + inspection of `blob_vec_kani.rs` title and `entity_allocator_kani.rs`). Adding atomic change-ticks requires either a new Kani proof for the tick-stamping path under concurrent access or a Miri-checked test that exercises concurrent stamps; Miri supports atomics. No Kani proof surgery on BlobVec itself is required.

**Sequential-path perf**: `AtomicU32::store(tick, Relaxed)` on x86 is essentially a plain move (LLVM emits the same `mov` instruction for relaxed stores). On ARM64 it's a single STR. On both, the load side (`Relaxed`) is also a plain `mov`/`ldr`. Negligible sequential regression. Cache behaviour is identical because the same memory is being accessed.

**Covers all UB-prone paths from §1.3?** No. Only closes Race #1 (change-ticks). Races #2, #3, #4, #5, #6, #7 remain. Two specific implications:

- `ecs_ai_showcase`'s observed divergence is consistent with Race #1 *and* Races #2/#7 (both systems also mutate `blob_components` and `resources`). Option A alone might reduce but not eliminate the observed divergence.
- Systems that spawn entities or insert components mid-tick still hit Races #3/#4/#5/#6 which are independent of change-ticks.

**Estimated effort**: 2-4 days for one focused engineer. Low risk because the surface is small and every change site is reachable by `rg 'change_ticks'`. Expanding to Miri-backed verification adds another 2-3 days.

**Verdict**: atomic change-ticks is a narrow, high-confidence, low-cost change — but it only kills one of five races. Shipping Option A alone would be a lie to the user ("ParallelSchedule is now sound" when it isn't).

### 2.2 Option B: archetype-level access tracking in `SystemAccess`

**Hypothesis**: extend `SystemAccess` ([parallel.rs:80-113](../../astraweave-ecs/src/parallel.rs#L80-L113)) to track not just TypeIds the system reads/writes, but also archetype identities. If `movement_system` and `combat_system` both declare that they touch the "enemies" archetype (or any archetype containing Position, Velocity, Health, Team, AIAgent), the conflict check flags them as conflicting and the scheduler serialises them.

**Static vs runtime computation**. Two variants:

- **Static**: a system declares its archetype set via a required-components signature. `movement_system` declares "operates on archetypes containing {Position, Velocity}", `combat_system` declares "{Health, Events-target}". Conflict = overlapping archetype sets (not overlapping component TypeIds). The scheduler computes each archetype's membership in declared sets at `add_system` time. New archetypes created mid-run need to be re-classified.
- **Runtime**: the scheduler observes, per tick, which archetypes each system actually accessed, and uses that history to inform the next tick's grouping. Bevy's "dynamic access tracking" pattern, though Bevy's actual implementation is static per-query.

Static is simpler. Runtime requires per-tick bookkeeping that itself may have concurrency issues.

**Does it close the UB hole?** Yes, conditionally. If the scheduler correctly refuses to put two systems in the same group when their archetype sets overlap, then step 3 of the borrow chain (`get_archetype_mut` on same ID) never happens with two tasks alive — only one task runs at a time on that archetype. Race #1 through #4 all close by construction.

Three caveats:

1. **Systems that create new archetypes mid-run** (via `insert<T>` / `remove<T>` that transitions an entity) cannot be statically classified for the new archetype. A conservative fix would flag any such system as exclusive.
2. **Archetype-set declaration is easy to get wrong**. A system that accidentally touches an extra archetype (via `world.entities_with::<T>()` returning archetypes the author didn't anticipate) would violate the declaration and re-open UB. The scheduler can't verify the declaration.
3. **Resource access** (Race #7) is *not* closed by archetype-level tracking — resources live on `World` directly, not inside an archetype. A separate check on `&mut World.resources` access would still be needed.

**Code scope**: 300-500 lines across `parallel.rs` (extend `SystemAccess`), `archetype.rs` (expose archetype-set lookup at `add_system` time), `lib.rs` (possibly a helper for building archetype sets). Plus per-system annotation updates in every binary using `ParallelSchedule`.

**Kani/Miri coverage**: none today. The new archetype-set logic is not `unsafe` per se — it's scheduler-level conflict resolution. Adding a property test (proptest) that generates random system access sets and verifies the scheduler never forms a group with overlapping archetype sets would be the test strategy. Miri not needed for the scheduler logic; still needed for `SendWorldPtr` if it's retained.

**Sequential-path perf**: zero — the check runs at `add_system` / `build_groups` time, not per-tick. `build_groups` becomes slightly more expensive (comparing archetype sets instead of just TypeId sets), but it's already cached post-schedule-stage-fix groundwork.

**Covers all UB-prone paths?** Races #1-#4 closed if declarations are accurate. Race #5, #6 (storage-level metadata under archetype creation) still open — the scheduler can serialise systems that might create new archetypes, but it cannot prevent two systems that were classified "non-creating" from accidentally triggering creation via `insert<T>`. Race #7 (`World::resources`) still open. Fundamentally, the safety guarantee is "author declarations are complete and correct" which fails open.

**Estimated effort**: 5-10 days for one focused engineer. Broken down: 2 days to design the archetype-set API; 2 days to implement and cover in unit tests; 2-3 days to annotate production systems; 1-2 days for property-test property generators to bomb the scheduler with random system combinations. Uncertainty comes from API bikeshedding.

**Verdict**: closes the concrete race but the safety model depends on human annotation. If an engineer adds a new system and fails to update its archetype declaration, UB returns silently.

### 2.3 Option C: column-level access primitives (Bevy-style)

**Hypothesis**: redesign component access so the mutable borrow unit is a single column within a single archetype, not the archetype or the World. Two systems mutating disjoint columns of the same archetype are sound because the scheduler hands each system `&mut BlobVec` for the columns they need, not `&mut Archetype`.

**Depth of change**: scheduler + World + Archetype. Specifically:

- **Archetype**: move component columns out from behind `HashMap<TypeId, BlobVec>`. Options: (a) `HashMap<TypeId, UnsafeCell<BlobVec>>` with the scheduler tracking which cells are in use; (b) store columns in a Vec aligned to `signature.components` (already sorted) and index by `binary_search`; (c) wrap each column in a `RefCell` or `RwLock` for runtime checking.
- **World**: provide a new access API: `world.column_mut::<T>()` returning a lock-guarded column view, or a per-tick "borrow-all-I-need" handshake where the scheduler pre-authorises a set of column references before running the system.
- **Scheduler**: instead of handing out `&mut World`, hand out a `SystemState` containing exactly the column handles each system declared. `SystemAccess::conflicts_with` checks column-identity (archetype-id + TypeId) overlap rather than just TypeId.
- **Queries**: `Query2Mut` etc. would receive column handles from the SystemState rather than `&mut World`. Existing query implementations mostly need to be rewritten.

This is essentially the architecture Bevy uses with its World/Table/Column/Access split. It's the cleanest solution to the whole class of races but it's a significant refactor of `astraweave-ecs`.

**Does it close the UB hole?** Yes, provably, for all five archetype-local races (#1 through #4) and Race #7 (resources get the same column-level treatment via `ResourceAccess`). Races #5 and #6 (archetype creation) require a separate serialization primitive: a "world-exclusive" mode where any system that might create archetypes takes a World-level exclusive lock. This adds a third access category beyond read/write: `exclusive`, already present in `SystemAccess::exclusive` at [parallel.rs:87](../../astraweave-ecs/src/parallel.rs#L87). Option C naturally uses it.

**Kani/Miri impact**: BlobVec's Kani proofs ([blob_vec_kani.rs](../../astraweave-ecs/src/blob_vec_kani.rs)) cover layout invariants; a column-per-`UnsafeCell` model does not invalidate those proofs — the invariants are about BlobVec's internal memory, not its ownership model. What does change is the aliasing story around BlobVec's access: instead of "access via `&mut Archetype.blob_components`", it becomes "access via `UnsafeCell::get()`" — classic sound use. New Miri or Kani proofs for the column-handle API would be additive: the existing BlobVec proofs remain unchanged.

**Sequential-path perf**: ambiguous, direction-dependent.

- `UnsafeCell<BlobVec>` per column has zero runtime cost at the access site.
- If `RefCell` is used for runtime-checked borrows, it adds one comparison + atomic-like state change per access — 5-10 ns, measurable under hot loops.
- Column handles issued pre-tick reduce per-access overhead to nothing vs. current `HashMap::get_mut` (which is already a hash + comparison).

Best-case sequential perf: identical. Worst-case (`RefCell`): a few percent regression on mutation-heavy hot paths.

**Covers all UB-prone paths?** Yes if extended to resources (Race #7) and archetype creation is serialised (Races #5, #6) via the existing exclusive-system bucket. This is the only option in the set that genuinely generalises.

**Code scope**: 1 500-3 000 lines across `parallel.rs`, `lib.rs`, `archetype.rs`, `blob_vec.rs`, `system_param.rs`. Plus rewriting every Query type. Plus migrating every consumer. Not a one-file diff.

**Estimated effort**: **2-8 weeks** for one focused engineer. The uncertainty range is wide because:

- Lower bound assumes a clean Bevy-pattern transplant with minimal original design.
- Upper bound covers a design that has to integrate with AstraWeave's specific conventions (the hybrid Box/BlobVec storage at [archetype.rs:86-90](../../astraweave-ecs/src/archetype.rs#L86-L90), the legacy `World` resource bridging at [ecs_adapter.rs:8](../../astraweave-core/src/ecs_adapter.rs#L8), the `App`'s backward-compat `Schedule`/`ParallelSchedule` coexistence).
- Kani proof updates for any `UnsafeCell` pattern are an additional 1-2 weeks.

Tightening this estimate is a 1-week design-doc exercise — the Phase 5 follow-up candidate.

**Verdict**: the only option that fully closes the class. Most expensive and most invasive. The investment pays off only if ECS parallelism is expected to be a shipping feature with multiple consumers.

### 2.4 Option D: restrict ParallelSchedule to disjoint archetypes at the usage layer

**Hypothesis**: don't fix the scheduler. Instead, document and enforce a rule: a parallel group may only contain systems whose archetype sets are disjoint. Two systems that operate on enemies cannot parallelise; a system that operates on enemies plus one that operates on particles can. The scheduler itself is unchanged; consumers must arrange their systems to meet the restriction.

**How does the scheduler know?** Manual annotation. Either:

- The system's TypeId access set is such that **no archetype contains both systems' writes** — e.g. system A writes only `Particle`, system B writes only `Enemy`, and no archetype has both. Verified at `add_system` time by querying the current archetypes.
- The scheduler is told explicitly via a new `SystemDescriptor::archetype_tag` annotation and refuses to group systems with overlapping tags.

Either way, some human has to ensure that the declarations are maintained as the binary's component topology evolves.

**Is this a useful subset?** Marginal. The realistic workloads that would fit:

- Particle systems separate from entity systems: yes, particles typically have distinct component signatures (`ParticlePosition`, `ParticleColor`, `ParticleLifetime`) that no game entity shares. Parallelisable.
- Physics vs rendering: often share `Position` via the same archetype. Not parallelisable under this rule.
- AI vs combat: in `ecs_ai_showcase` the 5 enemies share one archetype across AI+combat access. Not parallelisable. The very workload this audit was triggered by.
- Terrain chunks vs entities: terrain chunks are typically a different crate-level structure (`astraweave-terrain`'s chunk type is not ECS at all). The parallelism there already exists via rayon on chunks; ECS scheduler irrelevant.

So the useful subset is essentially "decoupled subsystems" which are rare in games and rarer still in AstraWeave's current binaries.

**Does it leave UB latent?** Yes. The restriction prevents hazard *if* annotations are correct; a system that overclaims or underclaims its archetype set breaks soundness silently. Same fail-open problem as Option B.

**Code scope**: 100-200 lines for the `archetype_tag` mechanism in `SystemDescriptor` + the conflict check update. Plus consumer annotations.

**Estimated effort**: 3-5 days.

**Verdict**: a narrower version of Option B with the same fail-open property and a smaller useful subset. No safety gain over Option B; less flexibility.

### 2.5 Option E: remove ParallelSchedule entirely

**Hypothesis**: delete [parallel.rs](../../astraweave-ecs/src/parallel.rs). Document AstraWeave as a single-threaded ECS. All parallelism continues to happen at the subsystem level — rayon in terrain meshing ([terrain/meshing.rs:478](../../astraweave-terrain/src/meshing.rs#L478)), tokio in async I/O and LLM ([ai_executor.rs](../../astraweave-ai/src/llm_executor.rs), [scene/streaming.rs:178](../../astraweave-scene/src/streaming.rs#L178)), rayon in fluids ([fluids/simd_ops.rs](../../astraweave-fluids/src/simd_ops.rs) — all subsystem-level).

**What's lost?** The theoretical ability to parallelise ECS systems across CPU cores. Concretely:

- `ParallelSchedule` has **one default-disabled opt-in caller** **[R2 — correction]**: `examples/profiling_demo/src/main.rs:211` behind the `parallel-schedule` feature flag (added in the `docs/audits/parallel_schedule_experiment_2026-04-18.md` experiment, merged disabled). Rev 1 of this audit incorrectly said "zero production callers" based on the binary-inventory audit which pre-dated the profiling_demo wiring. Plus `examples/ecs_ai_showcase/src/main.rs:598` also behind a `parallel-schedule` feature flag. Verified via `rg 'ParallelSchedule::' --glob '!**/tests/**' --glob '!**/benches/**' --glob '!**/archive/**'`. So the loss is:
  - Zero behaviour change on any default-features `cargo run ...` invocation.
  - Two binaries lose their opt-in `--features parallel-schedule` mode (both of which currently produce **incorrect output** per the `ecs_ai_showcase` experiment).
- The `SendWorldPtr` unsafe impl ([parallel.rs:42-70](../../astraweave-ecs/src/parallel.rs#L42-L70)) goes away. That's the only `unsafe impl Send/Sync` in `astraweave-ecs` outside `blob_vec.rs`. Reduces the formal-verification surface.

**Simplifies the safety story?** Yes. `parallel.rs`'s `SendWorldPtr` is `unsafe impl Send + Sync` on a `*mut World`. Deleting it removes one of exactly two unsafe trait impls in the crate (the other is BlobVec's `Send + Sync` impl which is a different class entirely — it's about the bytes BlobVec holds being Send/Sync, not about concurrent access to BlobVec).

**Existing non-test references**: per `rg 'ParallelSchedule::' --glob '!**/tests/**' --glob '!**/benches/**'` (verification hook in §Verification below), there are zero production consumers. The `ecs_ai_showcase` experimental wiring from 2026-04-18 is behind a `parallel-schedule` feature flag that is off by default; deleting `parallel.rs` would require also removing that feature flag from `ecs_ai_showcase/Cargo.toml`.

**Estimated effort**: **[R2 — tightened from 1 day to 2-4 days]**. Deletion list (verified with `rg`):
- `astraweave-ecs/src/parallel.rs` (519 lines).
- `parallel` feature in `astraweave-ecs/Cargo.toml` ([Cargo.toml:30](../../astraweave-ecs/Cargo.toml#L30)).
- Five `rayon::`-using integration tests and two bench harnesses (`astraweave-ecs/benches/alloc_measure.rs:75` constructs `ParallelSchedule`).
- `parallel-schedule` feature in `examples/ecs_ai_showcase/Cargo.toml` and the feature-gated `build_parallel_schedule` helper plus all `#[cfg(feature = "parallel-schedule")]` branches in `examples/ecs_ai_showcase/src/main.rs:598`.
- **[R2]** `parallel-schedule` feature in `examples/profiling_demo/Cargo.toml` and the feature-gated code in `examples/profiling_demo/src/main.rs:211` (Rev 1 missed this).
- `CLAUDE.md` references to parallelism primitives.
- Supersede-notes on `docs/audits/parallel_schedule_*` audit documents (they become historical).

Plus CI minutes (deleted tests, deleted benches — net reduction), plus an architecture-doc update to `CLAUDE.md` stating "AstraWeave is a deterministic single-threaded ECS; parallelism happens at subsystem level (rayon in terrain meshing, tokio in async I/O, etc.)" — which is already the de facto state per `docs/audits/job_system_audit_2026-04-18.md`.

Effort tightened to **2-4 days** once CI, doc, and the missed profiling_demo deletion targets are included. Low-risk because every deletion target can be greped first; no behaviour change on any default-features build.

**Verdict**: the simplest, smallest, safest option. Trades zero current default-behaviour capability for reduced verification surface. Leaves the door open for later adoption if a scheduler with a different safety model is designed.

### 2.6 Option F **[R2]**: split read/write dispatch in scheduler

**Hypothesis**: don't change the APIs; change *how the scheduler dispatches systems inside a parallel group*. In a group of N systems, at most one writes (declared `.writes::<T>()` for any `T`) and runs first with `&mut World`. The remaining readers all run in parallel via `rayon::scope` holding `&World` (shared borrow). The writer finishes before the rayon scope starts; the readers never alias `&mut World`.

This was dismissed in Rev 1's §3.1/§3.2 as "a scheduler change, not a usage restriction". That dismissal was a framing error — scheduler changes are legitimate fix options. Added as Option F here.

**Concrete shape**:

```rust
// In parallel.rs, run_group_parallel:
//   let (writer_idx, reader_indices) = partition_group_by_writes(group, systems);
//   if let Some(w_idx) = writer_idx {
//       (systems[w_idx].func)(world);  // &mut World, exclusive
//   }
//   if reader_indices.len() == 1 {
//       (systems[reader_indices[0]].reader_func)(world);  // &World, solo
//   } else if reader_indices.len() > 1 {
//       let world_shared_ptr = SendConstWorldPtr(world as *const World);
//       rayon::scope(|s| {
//           for &idx in &reader_indices {
//               let f = systems[idx].reader_func;
//               let p = world_shared_ptr;
//               s.spawn(move |_| {
//                   // SAFETY: all tasks hold &World (shared borrow), never &mut
//                   f(unsafe { &*p.0 });
//               });
//           }
//       });
//   }
```

**Does it close the UB hole?** Yes for groups with ≤1 writer, in the strong sense: under rayon::scope, every reader task dereferences `*const World` into `&World` (shared borrow). Multiple `&World` on the same object is perfectly sound in Rust's aliasing model. The writer never runs concurrently with readers (it finishes before the scope spawns). **All seven races (#1 through #9) close under Option F for zero-or-one-writer groups.**

**For groups with ≥2 writers** (as in `ecs_ai_showcase`'s simulation stage: `ai_behavior`, `movement`, `combat` all write something): Option F serialises them. But `ecs_ai_showcase`'s greedy-coloring already splits `ai_behavior` into group 0 and `movement + combat` into group 1 — and `movement + combat` are both writers (movement writes Position, combat writes Health/Events/GameStats). So under Option F, group 1 would serialise movement→combat. That's a regression vs. today's (unsafe) parallel execution, but the parallel execution is already producing incorrect output per the prior experiment. Correctness > performance.

**In groups with 1 writer + N readers** (common in real workloads — one system updates a component set, N systems read it for rendering/AI/physics): Option F is a strict win. Currently those workloads are also unsafe under `SendWorldPtr` because all N+1 tasks hold `&mut World`. Option F makes them sound AND parallelises the readers.

**Code scope**: `astraweave-ecs/src/parallel.rs` only. ~80-150 lines:
- New `SendConstWorldPtr` newtype (mirror of `SendWorldPtr` but on `*const World`).
- `SystemDescriptor` either gets a new `reader_func: Option<fn(&World)>` field, OR the `func` field becomes `enum { Reader(fn(&World)), Writer(fn(&mut World)) }`. The enum is cleaner and lets the scheduler dispatch-by-variant.
- `run_group_parallel` becomes the partition-and-dispatch logic sketched above.
- `build_groups`'s conflict check is unchanged; the partitioning of writer/readers within a group is orthogonal to conflict detection.

**Consumer migration**: every call site that registers a system must decide whether to register as reader or writer. In practice this is mechanical — if the system uses `.writes::<T>()` for any T, register the writer form; otherwise reader. The `reads::<T>()`-only systems in the five call sites in §1.2 would all become readers.

**Kani/Miri impact**: the new `SendConstWorldPtr`'s safety is *strictly weaker* than `SendWorldPtr`'s — shared borrow aliasing is sound by default. The existing `SendWorldPtr` for the writer path is preserved exactly (and is safe because only one task holds it). No new unsafe beyond the readers' `*const → &` cast which is the standard sound pattern. A loom test (§4.1.5) exercising the partition-dispatch model would confirm no interleaving produces UB.

**Sequential-path perf**: zero on the sequential path (the feature is opt-in via `--features parallel`). Under the parallel path, any group with ≥2 readers and ≤1 writer gets the speedup today's unsafe scheduler *appears* to provide — just soundly.

**Covers all UB-prone paths?** Yes for ≤1-writer groups, all Races #1-#9. For ≥2-writer groups, the scheduler's conflict check already serialises them today (it's one of the correct-by-construction behaviours), so no new hazard.

**Estimated effort**: **4-7 days** for one focused engineer. Breakdown:
- 1 day: design the `SystemDescriptor` enum variant and `SendConstWorldPtr`.
- 1-2 days: implement `run_group_parallel` partitioning + dispatch.
- 1 day: update the two opt-in consumers (`ecs_ai_showcase`, `profiling_demo`) to annotate each system as reader or writer.
- 1-2 days: loom test at `tests/parallel_loom_tests.rs` exercising the new dispatch (requires `[features] loom` and `#[cfg(feature = "loom")]`-gated `SendWorldPtr` / `SendConstWorldPtr` shims).
- 1 day: documentation — update the `SAFETY` block at `parallel.rs:290-311` to describe the new invariant, update the `SystemDescriptor` docs.

**Verdict**: Option F is the natural answer to the Rev 2 "Safe under perfect scheduler" reframe. It is the cheapest option that is *genuinely sound* for a useful subset of workloads (1-writer groups). It does not help the `ecs_ai_showcase` failing case specifically (that case has 2 writers in one group), but it does help real-world 1-writer-N-readers patterns. Option F is a better stopgap than Option A because A is only "partially sound" under its own caveats.

**Caveat**: Option F does not close Races #3/#5/#9 under 2-writer-groups, because those races occur when any two `&mut World` tasks run simultaneously. Option F's guarantee is conditional on the group having ≤1 writer. A 2-writer group triggers the "serialise the writers" fallback which is already how the scheduler would handle a conflict — so in practice, Option F's limit is a non-regression.

---

## Phase 3 — Safe-subset analysis

Independent of §2: does a *usage* restriction make the current `ParallelSchedule` sound without scheduler changes?

### 3.1 "Only one `.writes::<T>()` per parallel group"

**Does it close the UB hole?** No. The hazard isn't specifically about two systems both writing the same TypeId — it's about two systems both calling `&mut`-receiver methods that touch archetype-shared state. A group with one writer and one reader still hits Race #2 (both `&mut`-bound via `SendWorldPtr`, the reader still needs `&mut World` because of `SendWorldPtr`'s type, and `archetypes.get_archetype_mut` is called by any path that walks to a component) unless the reader exclusively uses `&self` methods.

Actually — walking this more carefully. If exactly one system in a group writes and the others only read, and readers use only `&self`-receiver methods like `get<T>`, the readers don't call `get_archetype_mut`. But under `ParallelSchedule`'s current dispatch ([parallel.rs:296-315](../../astraweave-ecs/src/parallel.rs#L296-L315)), each rayon task receives `&mut World` regardless of whether its system is read-only. The task could in principle only read, but the type signature says `fn(&mut World)` and the safety argument is on the function-level access declaration, not on what the function actually does. In practice a read-only system body on a `fn(&mut World)` is UB-safe but the aliasing model is strict: two concurrent `&mut World` holders is UB even if one never writes.

So "one writer per group" only works if the system signature changes to `fn(&World)` for read-only systems and the scheduler dispatches those via `&World` rather than `SendWorldPtr`. That's a scheduler change, not a usage restriction — it falls out of scope for this phase.

### 3.2 "Only read-only parallelism"

**Does it close the UB hole?** Partially. A group where every system is `.reads::<T>()`-only produces a schedule where no system calls `&mut`-receiver methods. But the dispatch still hands each task `&mut World` via `SendWorldPtr` — UB under strict aliasing regardless of what the system does with it.

For this to actually work, the scheduler would need to dispatch read-only groups via `&World` (shared borrow). That's a one-line change in principle but affects the `SystemDescriptor` signature and the `run_group_parallel` dispatch. Not a usage restriction.

### 3.3 "No `get_mut` in parallel systems; use only column-slice access"

**Does it close the UB hole?** Partially. `archetype.iter_components_blob_mut::<T>` returns `&mut [T]` — a slice directly into the BlobVec's buffer, not a `&mut T` via `archetype.get_mut`. If the scheduler could hand each parallel system a pre-computed `&mut [T]` at the start of the system's execution, systems with column-disjoint access would be sound.

But the *get* path to the slice still goes through `archetype.iter_components_blob_mut(&mut Archetype)` which mutably borrows the archetype. Two concurrent tasks each reaching for a slice race on `archetypes.get_archetype_mut(id)` (Race #5 in §1.4).

For this to work, the scheduler would need to pre-compute slices per tick per archetype per system and hand them out before parallel dispatch. That's column-level access (Option C), not a usage restriction.

### 3.4 "Manual archetype-disjointness declaration"

This is Option D from §2.4 rephrased. Discussed there.

### 3.5 Consensus

**No current-as-is usage restriction produces a sound subset**. Every candidate discussed above either fails to close the UB hole (3.1, 3.2 without scheduler changes; 3.4 fails open on annotation errors) or reduces to a scheduler change (3.3 is Option C).

The brief asks whether there is "something AstraWeave could ship in two days as a stopgap while a proper fix is planned". There is, but it is not a usage restriction — it is Option E (remove the scheduler, document single-threaded). That is a two-day change. Every other option requires scheduler changes of at least 2-4 days and none is certain.

---

## Phase 4 — Miri and Kani coverage

### 4.1 Feasibility of Miri on the failing case

Command the brief asks about: `cargo +nightly miri run -p ecs_ai_showcase --features parallel-schedule,alloc-counter -- -e 10 -f 20`.

Assessment:

1. **Dependencies**: `ecs_ai_showcase` currently depends on `astraweave-ecs`, `astraweave-core`, `astraweave-ai`, and `astraweave-alloc`. None of those pull in wgpu, winit, or non-Miri-friendly platform features. Miri should handle the transitive dep graph. Confirmed via `Cargo.toml` inspection.
2. **Rayon under Miri**: rayon does not work under Miri out of the box. The `rayon::scope` at [parallel.rs:305](../../astraweave-ecs/src/parallel.rs#L305) would either hang (Miri's threading model doesn't support rayon's work-stealing) or fail to build under Miri. There is a `-Zmiri-disable-isolation` flag for filesystem/stdio but no analogous flag for threading. Rayon's maintainers have noted this explicitly in their issue tracker (not cited here — out of scope for this audit).
3. **Force-serialize rayon for Miri**: yes, possible. Two mechanisms:

   a. Configure rayon's global pool with `ThreadPoolBuilder::new().num_threads(1).build_global()` in a Miri-only test harness. `rayon::scope`'s `s.spawn` will still produce separate tasks but they will execute sequentially. Miri will correctly model the pointer aliasing since the tasks still go through `SendWorldPtr::as_mut` and hold `&mut World` each.

   b. Stub rayon behind a `miri` cfg: inside `run_group_parallel` ([parallel.rs:296-315](../../astraweave-ecs/src/parallel.rs#L296-L315)), replace `rayon::scope` with a sequential loop under `#[cfg(miri)]`. The Miri run exercises the same unsafe paths and the same `SendWorldPtr` machinery but without rayon's scheduler. This is the more targeted approach — it tests the aliasing claim without involving rayon's threading.

   Either mechanism would require a small (∼30 line) wiring change to `ParallelSchedule`. Within the scope of a "make Miri feasible" task, not this audit.

4. **What Miri would tell us**: Miri would flag the UB at the exact call site (likely `archetype.stamp_change_tick::<T>` in the first case, then the subsequent races in successive runs). It would not distinguish "deterministic divergence" from "non-deterministic corruption" the way the empirical experiment did — Miri just says "UB, here, because these two pointers alias". That's the data we lack.

**Verdict**: Miri on `ecs_ai_showcase` is feasible with a small `#[cfg(miri)]` shim in `run_group_parallel`. High-value next step: it converts the empirical observation (divergence) into a formal proof (UB at line X).

### 4.1.5 Loom verification path **[R2]**

Rev 1 recommended Miri. Rev 2 identifies that **loom is already a dev-dependency** of `astraweave-ecs` ([Cargo.toml:46](../../astraweave-ecs/Cargo.toml#L46): `loom = "0.7"  # Concurrency model checker for Phase 4.4`), with **11 existing loom-gated tests** in `astraweave-ecs/tests/concurrency_tests.rs` at lines 35, 67, 100, 142, 174, 208, 246, 287, 330, 369, 402.

Caveat: those existing tests wrap `World` in `Arc<Mutex<World>>` ([concurrency_tests.rs:38](../../astraweave-ecs/tests/concurrency_tests.rs#L38)) — they prove mutex-protected access is sound, which is a *different* property from whether `SendWorldPtr`'s unsafe sharing is sound. So the loom infrastructure is *configured and wired* but not *applied* to the `ParallelSchedule` soundness question.

Why loom is better than Miri for this specific question:

- **Miri**: proves UB-free for *one* execution schedule. Rayon's work-stealing produces nondeterministic schedules, so one Miri-UB-free run doesn't generalise to all schedules.
- **Loom**: exhaustively enumerates *every* possible interleaving of the operations the test models. A loom-clean test is a strong correctness guarantee for the modelled operations.
- **Integration**: loom's ready-to-use. Miri requires the `#[cfg(miri)]` shim in `run_group_parallel` (audit §4.1 proposed this, est. 1 day). Loom requires a new test file and a `#[cfg(feature = "loom")]` variant of `SendWorldPtr` that uses `loom::cell::UnsafeCell` semantics.

Proposed follow-up task: add `astraweave-ecs/tests/parallel_loom_tests.rs` with loom models exercising:

1. Two `loom::thread::spawn` tasks each calling `SendWorldPtr::as_mut()` and then `World::get_mut::<T>` for disjoint `T`. Current code: loom should flag UB on the `change_ticks` HashMap `&mut`.
2. The same model but after an Option F rewrite where readers use `&World` and writers use `&mut World`. Loom should report UB-free.
3. Concurrent `Events::send<T>` from two Arc-clone holders. Loom should flag UB on `queues` HashMap (Race #8).

Estimated effort: **2-3 days** — the majority is designing the `#[cfg(feature = "loom")]` model variants of `World` and `SendWorldPtr` (loom wraps `UnsafeCell` differently than std). Higher cost than Miri (1 day) but strictly higher confidence. Both are feasible; loom should be preferred.

### 4.2 Existing Kani proof coverage

### 4.2 Kani proof coverage

Searching for `.rs` files with Kani harnesses in `astraweave-ecs`: `blob_vec_kani.rs` (**14 `#[kani::proof]` functions [R2 — counted directly]**, 257 LoC; covers BlobVec layout invariants, push/get/swap_remove round-trips, capacity growth, drop correctness) and `entity_allocator_kani.rs` (254 LoC; generational index safety). **No Kani proofs cover `parallel.rs`** specifically — `SendWorldPtr` is unsafe-Send/Sync with no formal proof, and the conflict-coloring algorithm has no Kani coverage either.

Miri proofs also: none on `parallel.rs` specifically.

What would the fix options require?

- **Option A**: atomic change-ticks. New Miri test exercising concurrent stamps. Kani proof for `AtomicU32` ordering not needed (Rust stdlib guarantees).
- **Option B**: archetype-level access tracking. Property test (proptest) for "no group ever contains overlapping archetype sets". Kani proof optional for the group-coloring correctness.
- **Option C**: column-level access. Significantly more work. A `SystemState` that hands out slices requires either Miri or Kani to prove that no two concurrent `SystemState` handles ever alias. The existing `blob_vec_kani.rs` proofs are preserved (they don't depend on the access pattern), but new proofs are needed for the column-handle machinery.
- **Option D**: same as B.
- **Option E**: no new proofs — deleting code removes the Kani/Miri requirement entirely.

**Verdict**: `parallel.rs` is currently the least-verified unsafe surface in `astraweave-ecs`. Any fix option except E adds verification work. Option E reduces the verification surface.

---

## Phase 5 — Narrow recommendation

The brief asks for a recommendation about which option to **audit next**, not which to implement.

### 5.1 Decision-criterion summary

**[R2] Revised summary**: Rev 1 recommended auditing Option C further. Rev 2 adds two considerations:
- Option F (scheduler dispatch change, 4-7 days) is a legitimate option that Rev 1 incorrectly filed under "scheduler change, not a usage restriction" and dismissed.
- Option E ships in 2-4 days (Rev 1 said 1 day; corrected for profiling_demo deletions and CI) and closes 100% of races by removing the scheduler entirely. There are zero *default-features* consumers today.

Updated decision matrix:

If the team's priority is **minimising engineering cost and shipping a sequential engine is acceptable long-term**: **execute Option E now.** 2-4 days. Closes 100% of races. Zero behaviour change on default builds. This is the recommendation if the engine's positioning does not require ECS-level parallelism. **[R2: this is now the default; see §5.3 for the Framing comparison.]**

If the team needs **ECS parallelism in the next six months** but can accept a 1-writer-per-group restriction: **execute Option F.** 4-7 days. Sound for all ≤1-writer groups; falls back to serialisation (correct behaviour) for ≥2-writer groups.

If the team needs **arbitrary-system parallelism including multi-writer groups**: investigate **Option C (column-level access) further** via a 1-week design doc. Effort estimate 6-14 weeks (Rev 2 revised range; Rev 1 said 2-8 weeks). Only option that generalises to multi-writer groups.

If there is an **immediate need to unblock `ecs_ai_showcase`-like workloads specifically**: Option F does not help (that workload has 2 writers in one group). The only fix for that specific topology is Option C. Near-term alternative: restructure the workload to have ≤1 writer per stage; then Option F helps.

### 5.2 Which option's effort estimate is most uncertain?

**Option C.** The 6-14 week range (Rev 2 revised) is driven by:

- Whether the column-access API can be a straightforward `UnsafeCell<BlobVec>` per column (fast) or needs to integrate with AstraWeave-specific hybrid Box/BlobVec storage (slower).
- Whether the existing BlobVec Kani proofs (14 harnesses) remain valid verbatim, or require adaptation to an `UnsafeCell`-wrapped variant.
- Whether every consumer Query type can be retrofitted to use `SystemState`-issued handles, or requires rewriting.
- Whether resource access (Races #6, #8) gets a similar column-handle treatment or a separate locking scheme.
- Whether the cross-crate `astraweave-core::ecs_adapter::build_app` bridging pattern can be migrated without rewriting it.

**Investigation that would tighten it** (the brief's required final question):

- **A 1-week design document** at `docs/design/column_access_primitives_<date>.md` that:
  1. Enumerates the column-handle API (method signatures, lifetime bounds, owner).
  2. Sketches the migration for `Query`, `Query2`, `Query2Mut`, and `each_mut`.
  3. Catalogues every unsafe block that would be added and what invariant each must uphold.
  4. Estimates Kani-proof work by counting harness functions needed.
  5. Estimates migration cost for the two opt-in consumers (`profiling_demo`, `ecs_ai_showcase`) and potentially `astraweave-core::ecs_adapter`.
  6. Produces a tighter effort estimate (e.g. 8±2 weeks) and a go/no-go decision.

A team member producing that design doc would have high confidence in the revised estimate.

### 5.3 Framing X vs Framing Y **[R2]**

Rev 1 and Rev 2 disagree on which option to audit next. This section is the explicit reconciliation.

**Framing X (Rev 1's pick): "Audit the most-uncertain option further" → Option C design doc.**

- Rationale: Option C is the only option that genuinely closes the class for arbitrary system topologies. Tightening its uncertainty is a 1-week investment that unlocks a later decision.
- Risk: the team spends 1 week designing a 6-14 week project for a scheduler with one opt-in caller and no default consumers. If the design doc concludes "proceed", the team commits to months of engineering for zero current users. If it concludes "don't proceed", the week is sunk cost.

**Framing Y (Rev 2's alternative): "Execute the actionable low-cost fix" → Option E or Option F.**

- Rationale: when the user count is low and the UB is observed, the right move is to stop the UB quickly and build parallelism later if needed. Option E ships sequential-only in 2-4 days and closes 100% of races. Option F ships read-parallel in 4-7 days and closes 100% of races for ≤1-writer groups.
- Risk: if a surprise near-term consumer appears that needs multi-writer parallel ECS, Option E's removal has to be reverted. But Option E is literally a `git revert` — the cost is near-zero. Option F does not foreclose Option C; the enum-dispatch pattern Option F introduces is compatible with a future Option C migration.

**Which is more defensible?**

Framing Y is more defensible **under current evidence**:
1. `ParallelSchedule` has zero default-features consumers and two opt-in consumers. The opt-in consumers are documented as producing incorrect output (per the prior experiment). There is no near-term business case for the 6-14 week investment Framing X presupposes.
2. Framing X's "1 week design doc" is not free — it commits senior engineering time to analysis of a project the team may not execute. Framing Y's "2-4 day execution" produces immediate value (UB closed, reduced verification surface).
3. The job-system audit at `docs/audits/job_system_audit_2026-04-18.md` already established that parallelism in AstraWeave happens at the *subsystem* level (rayon in terrain, tokio in async I/O, custom rayon scopes in fluids). ECS-level parallelism is a speculative addition, not a required capability. Framing Y formalises the de facto single-threaded-ECS state; Framing X speculates on a hypothetical multi-core-ECS future.

Framing X is more defensible if the team has **a named, committed near-term consumer** requiring ECS-level parallelism (e.g., a physics/AI/render stage with 4+ systems per stage that benefit from multi-core). No such consumer exists in the current binary inventory.

**Rev 2 recommendation**: default to **Framing Y**. Execute Option E (2-4 days) or Option F (4-7 days) now. Reserve the Option C design doc for the moment a multi-writer-per-stage consumer appears. If that moment is in the next 6 months, the design doc is then worth its week; if it is later than 6 months, even better — the intervening time may produce better design patterns from the Rust ecosystem (e.g., Bevy's access-descriptor evolution, chili, etc.) that make Option C cheaper to specify.

Rev 1's §5.1 "If the team's priority is minimising engineering cost": carried forward as the default path. The "audit Option C further" recommendation is downgraded to a conditional: *only* pursue the design doc if a consumer is named. The "effort estimate most uncertain is Option C" answer to the brief's final question remains unchanged — the uncertainty range is still the largest — but whether to close that uncertainty is a business decision the audit cannot make for the team.

### 5.2 Which option's effort estimate is most uncertain?

**Option C.** The 2-8 week range is driven by:

- Whether the column-access API can be a straightforward `UnsafeCell<BlobVec>` per column (fast) or needs to integrate with AstraWeave-specific hybrid Box/BlobVec storage (slower).
- Whether the existing BlobVec Kani proofs remain valid verbatim, or require adaptation to an `UnsafeCell`-wrapped variant.
- Whether every consumer Query type can be retrofitted to use `SystemState`-issued handles, or requires rewriting.
- Whether resource access (Race #7) gets a similar column-handle treatment or a separate locking scheme.

**Investigation that would tighten it** (the brief's required final question):

- **A 1-week design document** at `docs/design/column_access_primitives_<YYYY-MM-DD>.md` that:
  1. Enumerates the column-handle API (method signatures, lifetime bounds, owner).
  2. Sketches the migration for `Query`, `Query2`, `Query2Mut`, and `each_mut`.
  3. Catalogues every unsafe block that would be added and what invariant each must uphold.
  4. Estimates Kani-proof work by counting harness functions needed.
  5. Estimates migration cost for every consumer of mutable component access (the 5 production call sites from §1.2).
  6. Produces a tighter effort estimate (e.g., 4±1 weeks) and a go/no-go decision.

A team member producing that design doc would have high confidence in the revised estimate — all the ambiguity sources are bounded by concrete code surfaces that can be read and measured.

---

## Evidence index

Files read in full for this audit, grouped by crate.

**astraweave-ecs**:
- `src/lib.rs` — 1 557 lines. World state, Schedule, App, component access methods, resources, change ticks.
- `src/parallel.rs` — 519 lines. `SystemAccess`, `SystemDescriptor`, `ParallelSchedule`, `run_group_parallel`, `SendWorldPtr`, `build_groups`.
- `src/archetype.rs` — 1 121 lines. `Archetype` (fields: `entities`, `entity_index`, `components`, `blob_components`, `component_metas`, `uses_blob`, `change_ticks`); `ArchetypeStorage` (fields: `next_id`, `signature_to_id`, `archetypes`, `entity_to_archetype`, `component_to_archetypes`); all read/write methods.
- `src/system_param.rs` — 754 lines. `Query`, `Query2`, `Query2Mut` constructors and `next()` methods.
- `src/blob_vec.rs` — 1 111 lines. Confirmed: BlobVec's per-column unsafe surface is bounded to its own allocation; no shared metadata on BlobVec itself.
- `src/component_meta.rs` — 240 lines. `ComponentMeta` (drop_fn, clone_fn pointers); contributor to BlobVec but no shared mutable state.
- `src/sparse_set.rs` — 712 lines. Per-archetype `SparseSet` used for entity-to-row lookup. No shared state concerns; wholly owned by its parent `Archetype`.
- `src/entity_allocator.rs` — 695 lines. `EntityAllocator` generational index — fields `free_list: Vec<u32>`, `generations: Vec<u32>`, `next_id: u32`. Accessed via `&mut self` on World's `entity_allocator` field. **Also non-atomic**: two concurrent `World::spawn` calls would race on `next_id: u32` at [entity_allocator.rs around spawn path]. Falls under Race #6 class.
- `Cargo.toml` — feature flags, including `parallel = ["dep:rayon"]` at line 30.

**astraweave-core**:
- `src/ecs_adapter.rs` — `build_app` at line 169; systems `sys_sim`, `sys_move`, `sys_refresh_los`, `sys_bridge_sync`, `sys_sync_to_legacy`. Cross-referenced for §1.2.

**astraweave-ai**:
- `src/ecs_ai_plugin.rs` — `AiPlanningPlugin` at line 16, `sys_ai_planning` at line 45. Cross-referenced for §1.2.

**Examples (cross-referenced for §1.2)**:
- `examples/ecs_ai_showcase/src/main.rs` — all six systems.
- `examples/profiling_demo/src/main.rs` — surviving systems after stage fix.
- `examples/hello_companion/src/main.rs` — uses `build_app` from ecs_adapter.

**Prior audits** (for context, not re-cited per claim):
- `docs/audits/parallel_schedule_experiment_ecs_ai_showcase_2026-04-18.md` — §3.3, §3.9, §3.10 as specified.
- `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` — §3.4 system inventory; §4 (now superseded).
- `docs/audits/schedule_stage_fix_2026-04-18.md` — stage-fix context for the `post_simulation` stage availability.
- `docs/audits/job_system_audit_2026-04-18.md` — §1.2 on ParallelSchedule's zero production callers.

---

## Open questions

Items that this audit could not resolve from static code reading alone.

1. **Does Miri flag the UB at the line this audit predicts?** The audit claims the races are at specific borrow-chain steps. Miri would confirm which of the five races actually fires first in a given execution. Requires the `#[cfg(miri)]` shim in `run_group_parallel` (§4.1) and a 1-day task.
2. **Is Race #6 (`World::resources`) contributing to the `ecs_ai_showcase` divergence?** `combat_system` writes `Events` and `GameStats` resources while other systems read them. Whether the observed position/velocity divergence is traceable to resource aliasing vs. change-tick aliasing requires Miri or a targeted instrumentation run.
3. **Are there additional production `insert<T>` / `despawn` call sites in `astraweave-core` or `astraweave-ai` that would hit Races #3/#4/#5?** Partial enumeration in §1.2 shows `sys_ai_planning` calling `world.insert::<CDesiredPos>` and `sys_bridge_sync` calling `world.insert::<CLegacyId>`. Both in the same-tick flow. A full trace of how many archetype transitions those produce per tick would require instrumentation.
4. **What is the cost of `UnsafeCell<BlobVec>` per column vs the current HashMap access?** Option C's perf estimate says "negligible" but the actual overhead of indirection through `UnsafeCell::get()` vs `HashMap::get_mut()` on hot paths (per-entity iteration over millions of entities) is not measured. A microbenchmark would settle this.
5. **What is the correct placement of `exclusive` systems under Option B?** Systems that structurally mutate World (insert/remove/despawn) need to be serialised in Option B's model. Identifying which production systems actually perform structural mutation is a subset of question 3 above.
6. **Does `Query2Mut` need a `SAFETY` update?** The comment at [system_param.rs:267-273](../../astraweave-ecs/src/system_param.rs#L267-L273) claims column disjointness is sufficient. This audit identified that the *access path* to disjoint columns still aliases on shared HashMaps. Whether the comment is technically correct (it may be — "disjoint memory" is about the component data, not the access path) or misleading deserves clarification. **Documentation gap flagged in appendix; not fixing.**

---

## Verification hooks

Three `rg` commands a reviewer can run to independently confirm this audit's claims.

```bash
# 1. Confirm ParallelSchedule has zero production callers — the Option E argument.
# Expect: hits only in `parallel.rs` itself, tests, benches, and the ecs_ai_showcase
# experimental feature-gated path.
rg -n 'ParallelSchedule::(new|default)|ParallelSchedule::' --glob '!**/tests/**' --glob '!**/benches/**' --glob '!**/archive/**' --glob '!**/target/**'

# 2. Confirm the five shared-state races are all in archetype.rs lines 102, 86, 545, 555, 547, 549.
# Expect: exact line hits for change_ticks, blob_components, next_id, component_to_archetypes,
# signature_to_id, archetypes.
rg -n 'change_ticks:|blob_components:|next_id:|component_to_archetypes:|signature_to_id:|archetypes:' astraweave-ecs/src/archetype.rs

# 3. Confirm `stamp_change_tick` is called on the paths §1.3 claims.
# Expect: hits in `archetype.rs` (definitions), `lib.rs` (World::get_mut, World::each_mut), and
# `mutation_resistance_tests.rs` / `determinism_tests.rs` for tests.
rg -n 'stamp_change_tick' astraweave-ecs/src
```

---

## Appendix — documentation gaps found (not fixed per brief)

1. **`parallel.rs:290-311` SAFETY comment** claims soundness based on TypeId-disjoint access. This audit shows that claim is false. The comment should either be updated to narrow the soundness claim (e.g., "sound only in the absence of archetype-shared metadata, which the current `World` and `Archetype` APIs do not guarantee") or removed pending a scheduler redesign. **Do not fix in this task.**
2. **`system_param.rs:267-273` SAFETY comment** for `Query2Mut::next` claims "A and B are different types so their columns are disjoint memory" — true of the data, not the access path. Worth clarifying. **Do not fix in this task.**
3. **`archetype.rs` `next_id: u64` field** at [archetype.rs:545](../../astraweave-ecs/src/archetype.rs#L545) has no doc comment. Its non-atomic nature is the basis of Race #3. A doc comment saying "non-atomic; not safe to increment from multiple threads" would have caught the issue at code review time. **Do not fix in this task.**
4. **No architecture-level doc** in `astraweave-ecs/` explains the archetype-metadata / component-data split. This audit assembles it from first principles. A future `docs/design/astraweave-ecs-data-model.md` would be high-value context for any of the fix options. **Do not fix in this task.**

---

## TODOs deferred by scope

Items that would have been natural follow-ups but are out of scope per the brief:

- Do not run Miri. §4.1 says how; running it is a separate task.
- Do not rewrite `parallel.rs`. §2 enumerates options; all five require a separate task.
- Do not add the `#[cfg(miri)]` shim. Part of the Miri feasibility follow-up.
- Do not update the `SAFETY` comments in §Appendix. Explicitly in scope of whichever fix option lands.
- Do not extend the inventory to `astraweave-llm`, `astraweave-render`, `astraweave-physics`, etc. — out of scope per the brief's "five call sites" rule.

---

**Report status**: read-only design audit complete. Rev 1 + Rev 2 amendments applied. No code changes. Phase 5 **[R2]** revises the recommendation: **default to Framing Y (execute Option E now, 2-4 days)** unless a specific near-term ECS-parallelism consumer is named, in which case Framing X (audit Option C further via a 1-week design doc) applies. Any of the six fix options can be adopted without invalidating this audit — the audit is inputs for the decision, not the decision itself.

---

## 7 Second-pass audit trail **[R2]**

Rev 2 changes are itemised here for auditability. Every item was verified by direct `rg` / file-read before being incorporated.

### 7.1 Factual corrections

| # | Claim in Rev 1 | Corrected in Rev 2 | Evidence |
|---|---|---|---|
| C1 | "ParallelSchedule has zero production callers" (§2.5) | One opt-in caller: `profiling_demo/src/main.rs:211`; plus `ecs_ai_showcase/src/main.rs:598`. Both behind `parallel-schedule` feature flag (default off). | `rg 'ParallelSchedule::' --glob '!**/tests/**' --glob '!**/benches/**' --glob '!**/archive/**'` returned 2 hits outside `parallel.rs` itself. |
| C2 | "`combat_system` `Events` access is at main.rs:417" (§1.3) | Actual line is 433. Line 417 is `get_mut::<Health>`. | `rg 'get_resource_mut.*Events' examples/ecs_ai_showcase/src/main.rs` → 4 lines, `combat_system` is at 433. |
| C3 | "Option E effort: 1 day" (§2.5) | 2-4 days (profiling_demo deletions added; CI and doc work budgeted). | Deletion list grew by 2 files post-correction. |

### 7.2 New findings added

| # | Finding | Where in Rev 2 | Verification |
|---|---|---|---|
| F1 | Race #8: `Events::queues: HashMap<TypeId, Box<dyn Any>>` is an additional race class — HashMap-inside-resource, same pattern as Races #1/#2. | Executive summary table row 8; §1.3 Race #6 paragraph expanded. | Read `events.rs:74-78`; confirmed `send::<E>` path mutably borrows `queues`. Concrete trigger: `ecs_ai_showcase` `ai_planning_system` (main.rs:285-291) and `combat_system` (main.rs:433) both send different event types. |
| F2 | Race #9: `EntityAllocator::next_id: u32` (non-atomic) and `EntityAllocator::generations: Vec<u32>` (Vec growth invalidates concurrent `is_alive` readers). | Executive summary table row 9. | Read `entity_allocator.rs:171-194, 238-257`. `spawn` at line 238-257 does `next_id.checked_add(1)` and `generations.push(0)` — both non-atomic and Vec-reallocating. |
| F3 | CommandBuffer's `!Send + !Sync` doc comment is unenforced. | New §1.5. | Read `command_buffer.rs:55-60`: doc says `!Send + !Sync` but fields are `Send + Sync` and no `PhantomData<*const ()>` marker exists. |
| F4 | loom is already a dev-dependency with 11 existing (but mutex-based) concurrency tests. | New §4.1.5. | Read `astraweave-ecs/Cargo.toml:31,46`; read `astraweave-ecs/tests/concurrency_tests.rs` (11 `loom::model` calls confirmed at lines 37, 68, 102, 144, 176, 210, 248, 289, 332, 371, 404). |
| F5 | Per-column Vec-growth caveat on Option A: atomic per-row stores become invalid if the Vec reallocates during `add_entity_with_tick`. | §2.1 updated. | Read `archetype.rs:182-206`: `self.change_ticks.get_mut(ty).unwrap().push(tick)` — Vec::push can reallocate. |
| F6 | Kani proof count for BlobVec: 14 `#[kani::proof]` functions in `blob_vec_kani.rs`, 257 LoC. Rev 1 said "`blob_vec_kani.rs` covers BlobVec layout invariants" without the count. | §4.2 updated with concrete number. | `rg '#\[kani::proof\]' astraweave-ecs/src/blob_vec_kani.rs` → 14 matches. |

### 7.3 New fix option

**Option F** (§2.6): scheduler dispatch change — readers via `&World`, writer via `&mut World`. 4-7 days. Closes all seven races for ≤1-writer groups. Rev 1's §3.1/§3.2 dismissed this framing as "a scheduler change, not a usage restriction"; Rev 2 reclassifies it as a legitimate first-class option.

### 7.4 Reframes

- **§1.3 "Safe under perfect-scheduler" column** added. Rev 1's empty "Single-task safe" bucket was a strict-reading artefact. The API is fine under a scheduler that enforces single-`&mut`-at-a-time; the scheduler is the broken component.
- **§5.3 Framing X vs Framing Y** comparison added. Rev 1's default was "audit Option C further"; Rev 2's default is "execute Option E now" unless a near-term consumer is named. The shift is defensible because the evidence base (zero default-features consumers, two opt-in consumers with observed incorrect output) does not support the Option C investment.

### 7.5 Effort-estimate tightening

| Option | Rev 1 estimate | Rev 2 estimate | Additions included |
|---|---|---|---|
| A | 2-4 days | 4-7 days | CI minutes, Miri test, "this narrows but does not close UB" documentation |
| B | 5-10 days | 10-16 days | Resource-access tracking (not modelled in Rev 1), property-test harness |
| C | 2-8 weeks | 6-14 weeks | Cross-crate migration (ecs_adapter), new Kani proofs for SystemState contract, Query rewrites |
| D | 3-5 days | 5-7 days | Archetype-tag annotations, property test |
| E | 1 day | 2-4 days | Profiling_demo deletions (Rev 1 missed), CI, doc work |
| F [R2] | — | 4-7 days | New option; baseline includes loom test + SystemDescriptor enum variant |

### 7.6 Items flagged but not amended (scope boundary)

- Reading the Events resource internals in depth revealed Race #8; reading other resources in `astraweave-ecs` (e.g. `EntityBridge` in `astraweave-core`, `GameStats` in `ecs_ai_showcase`) may reveal similar HashMap-inside-resource patterns. **Not investigated** per the "five call sites" scope rule.
- `TypeRegistry` and `ComponentMetaRegistry` on `World` are mutated during `register_component<T>` only (a setup-time operation). Concurrent `register_component` is theoretically UB but no call site does this mid-tick. **Not elevated to a numbered race**; flagged here for completeness.
- The `astraweave-core::ecs_adapter::build_app` bridge pattern was noted in Option C's scope but not unpacked in detail. A design-doc for Option C would need to enumerate its migration cost concretely.

### 7.7 What Rev 2 did NOT change

Preserved verbatim from Rev 1 because the analysis held up under review:
- Phase 1 enumeration of Races #1-#5 and the per-archetype / per-storage / per-World categorisation.
- §1.1 tables for `World`, `Archetype`, `ArchetypeStorage`, Query types — spot-checked citations, all correct except for C2 above.
- Phase 3 walkthrough demonstrating no usage restriction produces a sound subset without scheduler changes (§3.5 conclusion unchanged).
- Phase 4.2 Kani coverage gap identification (no Kani on `parallel.rs`).
- Verification hooks at §Verification. All three still work.
- Evidence index.

### 7.8 Adversarial review mechanism

Rev 2 was produced by:
1. Passing Rev 1 plus the specific prompt "find what I missed; be adversarial" to the `deep-reason` subagent.
2. The agent produced a critique (not a rewrite) with 11 specific amendments identified. Each was graded on a 3-point axis: factual correction / new finding / new option / reframe / estimate tightening.
3. Every factual claim in the critique was re-verified by direct `rg` or file-read before being incorporated into Rev 2. Three items from the critique that did not survive verification (not listed here) were dropped.
4. The verified amendments are the 11 items above.

The 1.5-hour Rev 1 → Rev 2 turnaround is low relative to the substance of the changes because most of the deep-reason agent's critique was immediately verifiable — the structural observations (empty Single-task bucket, dismissed Option F framing, missed profiling_demo caller) were visible once pointed out. A reader reproducing the review would find the same amendments.
