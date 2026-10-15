---
schema_version: 1
trace_id: combat
title: "Combat damage"
description: "Q32.32 damage dealt and received, data-driven schools, AI knowledge mask"
primary_crate: astraweave-combat
domain: gameplay
lifecycle_status: active
integration_status: partial
summary: "Resolver re-exports core SimRng and astraweave-math Fixed64. whole_raw stays here. combat_resolve is a required determinism-replay case. The demo still uses its own flat hits."
owns: [astraweave-combat]
doc_version: "1.2"
last_verified_commit: be413626
runtime_edges:
  - to: astraweave-ecs
    note: CombatPlugin registers combat_resolve_system on the simulation stage
  - to: astraweave-destruction
    note: Cover hits enqueue Impact commands on DestructionWorld
---

# Architecture Trace: Combat damage

## Metadata

| Field | Value |
|---|---|
| **System name** | Combat damage |
| **Primary crates** | `astraweave-combat` |
| **Document version** | 1.2 |
| **Last verified against commit** | `be413626` |
| **Last verified date** | 2026-10-15 |
| **Status** | Active, partial. The demo does not call it yet. |
| **Owner notes** | Formula spec: `docs/current/COMBAT_DAMAGE_SYSTEM_DESIGN.md`. Gate Pass, 2026-10-14. |

---

## 1. Executive Summary

**What this system does:**
Resolves one hit or heal into a Q32.32 health delta, with school mitigation, sorted modifiers, and a four-draw `SimRng` stream.

**Why it exists:**
The repo had four flat or `f32` damage paths. This crate is the single production resolver. Callers submit `HitEvent`s. They do not run a second formula.

**Where it primarily lives:**
- `astraweave-combat/src/resolve.rs` — `resolve_hit`, `resolve_heal`, `expected_hit`, `effective_health`
- `astraweave-combat/src/tables.rs` — RON load, ceilings, mod overrides, semantic hash
- `astraweave-combat/src/ecs.rs` — components, `combat_resolve_system`, and cover impulses
- `astraweave-weaving/src/damage.rs` — data-driven element schools
- `astraweave-weaving/src/aravath.rs` — sword, weave bolt, and cover destruction
- `astraweave-combat/src/save.rs` — `combat_tables` and `combat_vitals` meta keys
- `astraweave-combat/src/sim_rng.rs` — `hit_rng` on `astraweave_core::SimRng`
- `astraweave-combat/src/fixed.rs` — `whole_raw` on `astraweave_math::Fixed64` (PR #245)

**Status note:**
Schools are data. The shipped table is physical, weave, and the mundane hazards fire, frost, and poison. Shock is not shipped. The labels Echo and Veil are rejected. A sword and shield with only Might, Armor, and Bulwark is a complete build. AI queries take a `StatKnowledge` mask. `CThreadStrain` is saved and is not read by the formula.

---

## 2. Authoritative Pipeline

```text
HitEvent queued by the caller
    │  HitQueue::drain_sorted  (attacker, defender, heal, attack id, seq)
    ▼
hit_rng(world_seed, tick, ordinal, attack_id)     sim_rng.rs
    │  four u32 draws inside resolve_hit
    ▼
gates: dead, iframe, parry, evasion
    │
    ▼
base = rolled weapon + rating * coeff + flat
    │  attacker modifiers, id order
    │  crit, glance
    │  school hyperbola  taken = raw * K / (armor + K), then cap
    │  defender modifiers, id order, then min-taken floor
    │  flat block / absorb, pinned to the floor
    ▼
CVitals.hp / poise            ecs.rs apply_hp_delta
    │
    ├─ CombatLog.deaths when hp crosses to <= 0
    └─ save meta combat_vitals + combat_tables hash
```

`expected_hit` and `effective_health` use the same integers and a knowledge mask. They do not draw.

---

## 3. Semantic Vocabulary

| Term | Meaning |
|---|---|
| School | Data id. Default set: physical, weave, fire, frost, poison. |
| Hazard | Presentation of fire, frost, and poison. Not a spell school. |
| Weave | Non-physical causal school. Not Veil, not Echo. |
| Bleed | Attack flag. Periodic and physical only. Skips the hyperbola. |
| Bulwark | Shield percent and flat, applied only while the block gate is open. |
| Thread strain | capacity, reserve, coherence, strain. Saved. Unused by damage. |
| Knowledge mask | Which stats an AI observer may read. Hidden armor cannot change the estimate. |

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-combat/src/fixed.rs` | `whole_raw` | Active | Re-exports `astraweave_math::Fixed64`. The range check returns `CombatError` |
| `astraweave-combat/src/sim_rng.rs` | `hit_rng` | Active | Re-exports `astraweave_core::SimRng`. One extra fold, then `SimRng::new` |
| `astraweave-combat/src/tables.rs` | Load, validate, hash | Active | `DEFAULT_TABLE_RON` |
| `astraweave-combat/src/resolve.rs` | Hit and heal math | Active | No `Vec` on the hit path |
| `astraweave-combat/src/ecs.rs` | Tick system | Active | Simulation stage. Cover hits enqueue `Impact` commands |
| `astraweave-weaving/src/aravath.rs` | Aravath slice | Active | Sword 25, weave rating 15, cover bond breaks |
| `astraweave-combat/src/save.rs` | Meta codec | Active | No dependency on `aw-save` |

---

## 6. Conflict Map

- Do not wrap `astraweave-gameplay::Stats::apply_damage`. It remains the legacy flat formula until a later port.
- Do not put a second damage product in `astraweave-weaving` or `aravath_runtime`.
- Do not add a second `Fixed64`. The type is `astraweave_math::Fixed64` on PR #245. `SimRng` is `astraweave_core::sim`.
- Do not add shock, Echo, or Veil to the default table.
- Do not let `expected_hit` or `effective_health` ignore `StatKnowledge`.

---

## 7. Decision Log

- **RNG seam closed.** `SimRng` and `mix_sim_seed` come from core. `hit_rng` still folds ordinal and attack id into `SIM_SYSTEM_COMBAT` and calls `SimRng::new` once.
- **One Fixed64.** Combat re-exports `astraweave_math::Fixed64`. `whole_raw` stays in this crate. Arithmetic was not copied, so the golden hashes stay the previous bytes.
- **Generic resists.** Ratings and resistances are id-keyed arrays, not Heat/Rime/Volt/Toxin fields.
- **AI mask.** The world resolver sees true components. The estimate functions do not.
- **K is the attacker's tier.** The table row is selected with `AttackerView.tier`.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Crate is unsafe-free | Yes | `#![forbid(unsafe_code)]` |
| 2 | Resolver modules do not mention `f32`, `f64`, `HashMap`, `SystemTime`, `Instant` | Yes | `sim_modules_have_no_float_or_hashmap` |
| 3 | Canonical seven stages hash to `071dc70e116987782a8a8b137a6cce648d390a82967726775308946f1e69da71` | Yes | `canonical_hit_matches_the_golden_hash` |
| 3b | 48-tick 8-fighter battle hashes to `975aae72e3c83a2335be05142eea2bf2ae4413d621fc5009ed6019f89e2a6e52` | Yes | `tests/replay_hash.rs`. `determinism-replay.yml` requires `combat_resolve` on three OS |
| 4 | Armor 400, K 400, 1000 HP → EHP 2000 | Yes | `armor_equal_k_is_exactly_half` |
| 5 | Default table has no veil, echo, shock, or spell | Yes | `default_table_is_lore_safe_and_physical_viable` |
| 6 | Sword 25 through an 8000 bp bulwark is 5 and still positive | Yes | same test |
| 7 | Hidden armor does not change `effective_health` | Yes | `hidden_armor_cannot_change_the_estimate` |
| 8 | `resolve_hit` allocates 0 times after warmup | Yes | `tests/no_alloc.rs` |

---

## 9. Performance & Resource Profile

`resolve_hit` is a stack-only function. The ECS queue `Vec` is recycled. The timer walk appends into `CombatScratch.ids` through `ArchetypeStorage::append_entities_with_component`. After the first tick that sized that buffer, the walk does not allocate.

---

## 10. Testing & Validation

- `cargo test -p astraweave-combat --lib --tests`
- `cargo clippy -p astraweave-combat --all-targets -- -D warnings`
- Bench: `cargo bench -p astraweave-combat --bench resolve_hit` (the untimed loop asserts a zero allocation delta)

---

## 11. Open Questions / Parked Decisions

- Demo sword, shield, roll, and scar injection still live in `aravath_runtime`. Wiring them is a later step.
- `WorldSnapshot` does not grow an EHP field until planner hashes are versioned.
- Raising the 8-modifier ceiling needs a new overflow proof.

---

## 12. Maintenance Notes

`fixed.rs` keeps `whole_raw` only. If that helper moves, the golden stage hash and the battle digest are the check that the move did not change a bit. When the default school list changes, re-run the lore-name test. Bump `doc_version` if the stage order or the golden hash changes.

`combat_resolve` is a required case of `.github/workflows/determinism-replay.yml`. The combat test writes `replay-hash-combat.json` so it does not replace core's `replay-hash.json`. The workflow merges the two files before upload.
