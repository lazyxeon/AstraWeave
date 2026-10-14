# Combat Damage System — Design

**Status**: design only. No engine code in this change. Implementation waits on orchestrator approval.
**Risk tier**: **r3**. See §1.
**Date**: 2026-10-14
**Branch**: `cursor/combat-damage-design`
**Audited engine HEAD**: `a3fe3ecd` (`main`)
**Related HEADs read, not modified**: `origin/cursor/engine-roadmap` `de5832b5`, `origin/cursor/engine-ws-d2-stab` `05c2bff0`, `origin/cursor/aravath-greybox-b3de` `c86690ab`

This document is the specification for a bespoke damage-dealt and damage-received system. It is written under the GCP Governance Package v0.1 (Evidence Constitution). The human orchestrator remains the authority who may accept the residual risks and allow implementation.

---

## 1. Risk tier

**Declared tier: r3.**

The user set r2 as the floor. This spec is escalated to r3 because the formulas, once implemented, become core simulation state:

- Cross-OS bit-identity is a merge gate for every sim value (`docs/current/ENGINE_PRODUCTION_ROADMAP.md` §0 on `cursor/engine-roadmap`).
- Health, mitigation, and the combat table hash enter saves. A later formula change desyncs replays and old saves.
- AI target choice will consume effective health. A wrong EHP function changes plans, not just numbers.
- The work is hard to reverse after golden hashes and the save meta key ship.

This run does not implement the system. Escalation changes the gate, not the scope: the gate in §18 is **Needs-human**, and the next action is **stop**.

r3 artifacts present: Evidence Index (§4), claim grounding (§5–§7), dual-review twice (§16, §17), acceptance criteria (§14), Gate Decision Card (§18), reproducibility note (§19), provenance (§3).

---

## 2. Hard constraints (locked before the formulas)

These are not tuning knobs. A design that breaks one of them is a failed design.

| # | Constraint | Lock |
|---|---|---|
| H1 | **Bit-identical** results on Windows, Linux, and macOS under Rust 1.89.0, baseline CPU (`-C target-cpu=x86-64` on x86_64, default aarch64 baseline). `target-cpu=native` is forbidden for the hash job. | Roadmap §0 rules 1–2 |
| H2 | **No floating point in the resolver.** Magnitudes are Q32.32 `Fixed64` (`raw = value * 2^32`, `i64`). The type matches `astraweave-veil` on `cursor/aravath-greybox-b3de`: `SCALE = 1 << 32`, mul through `i128`, division truncates toward zero, `den == 0` yields zero. No `f32`/`f64` conversion on the sim path. | Veil `fixed.rs`; roadmap §0 rule 8 |
| H3 | **Randomness** is `astraweave_core::sim::SimRng` seeded by `(world_seed, tick, system_id)`. Present on `cursor/engine-ws-d2-stab` (`astraweave-core/src/sim.rs`). Not on `main` at `a3fe3ecd`. Combat does not call `unit_f32`, `rand::random`, or `thread_rng`. | Roadmap §0 rule 5; d2 `mix_sim_seed(1,2,3) = 0x5991_344E_4904_5002` |
| H4 | **Fixed evaluation order.** Modifiers sort by `modifier_id` ascending. Duplicate ids on one combatant are a hard error. No `HashMap` / `DashMap` iteration. Tables freeze into a `BTreeMap`. | Roadmap §0 rule 4 |
| H5 | **Overflow and rounding are specified** in §7.4. Widening multiplies use `i128` or `u128`. Saturation is a flagged clamp, not a panic and not a silent wrap. | This document |
| H6 | **Data-driven within engine ceilings.** Schools, K, curves, coefficients, caps, and per-ability variance live in data mods can override. Validation rejects a table that crosses an engine ceiling (§8.3). | User constraint |
| H7 | **Hot path.** `resolve_hit` allocates nothing. It reads borrowed slices and writes a fixed `StageTrace`. The ECS batch reuses one `Vec` and sorts it in place. No `rayon` inside the tick. | Roadmap §0 rule 7 |
| H8 | **One resolver.** Hit detection, animation, and physics stay in their crates and submit a `HitEvent`. They do not run a second damage formula. | Gameplay trace: two combat paths already coexist and must not gain a third formula |
| H9 | **Immunity is a gate, not a percent.** I-frames and parry deal 0 by short-circuit. No modifier may encode 100% reduction. | User constraint; §7.3 proof |
| H10 | **60 Hz tick.** Durations that affect sim state are tick counts. Wall clocks stay out of the resolver. | `App::run_fixed`; roadmap §0 rule 3 |
| H11 | **Save/load and AI** observe the same integers the resolver writes. Effective health is a pure function of those integers and the table. It does not roll RNG. | User constraint |
| H12 | **Scar field stays the scar field.** A death emits an event. `ScarField::inject_death` is not inside the damage formula. | Veil trace on `c86690ab` |

---

## 3. Provenance

```yaml
provenance:
  sources:
    - "GCP Governance Package v0.1 (uploaded, 2026-10-12)"
    - "User WoW research input (uploaded; every number treated as an unverified claim)"
    - "https://warcraft.wiki.gg/wiki/Armor"
    - "https://warcraft.wiki.gg/wiki/Damage_reduction"
    - "https://warcraft.wiki.gg/wiki/Attack_power"
    - "https://warcraft.wiki.gg/wiki/Normalization"
    - "https://warcraft.wiki.gg/wiki/Armor_penetration"
    - "https://www.wowhead.com/news/battle-for-azeroth-new-formula-for-calculating-ability-damage-281913"
    - "https://github.com/simulationcraft/simc/wiki/DevelopersDocumentation"
    - "https://www.poewiki.net/wiki/Armour"
    - "https://wiki.guildwars2.com/wiki/Damage_calculation"
    - "https://wiki.guildwars2.com/wiki/Damage"
    - "https://eldenring.wiki.fextralife.com/Calculating_Damage"
    - "https://eldenring.wiki.gg/wiki/Poise"
    - "https://www.akhmorning.com/allagan-studies/stats/defense/"
    - "https://d4guides.gg/en/guides/mechanics/toughness-in-diablo-4-how-your-defense-really-works"
    - "main a3fe3ecd: astraweave-gameplay combat.rs, combat_physics.rs, stats.rs, items.rs, crafting.rs, ecs.rs, types.rs; astraweave-weaving combat.rs, enemy_types.rs; astraweave-ecs rng.rs; astraweave-core schema.rs, ecs_components.rs; docs/architecture/gameplay.md"
    - "origin/cursor/engine-roadmap de5832b5: docs/current/ENGINE_PRODUCTION_ROADMAP.md §0"
    - "origin/cursor/engine-ws-d2-stab 05c2bff0: astraweave-core/src/sim.rs SimRng, astraweave-weaving/src/sim_rng.rs"
    - "origin/cursor/aravath-greybox-b3de c86690ab: aravath_runtime/src/combat.rs, motor.rs; astraweave-veil/src/fixed.rs, field.rs, lib.rs; astraweave-ai/src/scar_sense.rs; docs/architecture/veil.md"
  key_assumptions:
    - "SimRng's (world_seed, tick, system_id) API from the d2 branch is the engine RNG contract even though it is not on main yet. Combat implementation blocks on that API landing, or on a reviewed extract of the same function."
    - "Fixed64's arithmetic (trunc toward zero, i128 mul, saturating cast) is the engine fixed-point contract."
    - "Default K, caps, and the greybox ability row are content defaults for Aravath-scale numbers (sword 25, enemy HP 40, player HP 100). They are not laws."
    - "Retail WoW constants that were not re-read from a current client (crit 2.0/1.5, versatility's 1/2 defensive ratio, the 1.0095 item-level fit, today's ArmorMitigationByLvl rows) are not copied."
  model_context: "Cursor Grok 4.7 cloud agent, 2026-10-14, design-only run. Workspace main a3fe3ecd."
  limitations: "No current WoW client dump. No PoE2 armour re-derivation (PoE1 wiki formula only). Elden Ring curve is a secondary wiki, not a disassembly. Diablo 3's '50 * level' constant was not re-fetched. Physics bit-identity remains the open roadmap WS-D3 problem and is outside this resolver."
```

---

## 4. Evidence index

```yaml
evidence_index:
  - claim: "Classic physical DR = Armor / (Armor + K), and damage taken = Raw * K / (Armor + K)."
    evidence:
      - type: source
        ref: "https://warcraft.wiki.gg/wiki/Armor (PaperDollFrame.lua formulas quoted for 5.0.4 and, via the vanilla archive, 2.0.8); https://warcraft.wiki.gg/wiki/Damage_reduction"
        strength: strong
    status: supported
    notes: "K's level breakpoint is contested. See E2."

  - claim: "Armor == K implies DR = 50%, and Armor = 3K implies DR = 75% before any separate cap."
    evidence:
      - type: calculation
        ref: "A/(A+K) = 1/2 when A = K. A/(A+K) = 3/4 when A = 3K. Integer proof in §7.2."
        strength: strong
    status: supported

  - claim: "Below the cap, EHP = HP * (1 + Armor/K) is linear in Armor."
    evidence:
      - type: calculation
        ref: "§7.2. Worked HP=1000, K=400, Armor in {0,400,800,1200} yields EHP {1000,2000,3000,4000}."
        strength: strong
    status: supported
    notes: "Linearity stops at the cap: derivative falls from HP/K to 0. The value is continuous at the cap (EHP = 4*HP when cap = 75%)."

  - claim: "User classic examples K(60)=5500 and K(63)=5755."
    evidence:
      - type: calculation
        ref: "400+85*60 = 5500 and 400+85*63 = 5755, which is the sub-60 formula extended past 59."
      - type: source
        ref: "https://warcraft.wiki.gg/wiki/Armor_penetration pseudocode: if level<60 then K=400+85*L else K=400+85*L+4.5*85*(L-59). That else-branch gives K(60)=5882.5, K(63)=7285, K(80)=15232.5, K(83)=16635."
        strength: strong
    status: contested
    notes: "The 60+ closed form in the user note matches the wiki else-branch. The numeric examples 5500 and 5755 do not. ZugZugDB states both '5500 at 60' and '7285 at 63', which are different branches. AstraWeave does not encode either float expression."

  - claim: "Classic 14 AP = 1 DPS. Normalized speeds 1.7 / 2.4 / 3.3 / 2.8. Spell coefficient cast/3.5."
    evidence:
      - type: source
        ref: "https://warcraft.wiki.gg/wiki/Attack_power ; https://warcraft.wiki.gg/wiki/Normalization"
        strength: strong
      - type: source
        ref: "Spell upper clamp: user says 3.5s; ZugZugDB (secondary) says 1.5..7s so a coefficient can exceed 1."
        strength: moderate
    status: partially-supported
    notes: "14 AP and the four speeds are supported for classic. Patch 6.0.2 changed the AP rate to 3.5 AP = 1 DPS (same Attack power page). The spell upper clamp is contested and is not copied."

  - claim: "Retail: 1 primary = 1 AP, effective AP = AP + weaponDPS * 6, times an ability coefficient."
    evidence:
      - type: source
        ref: "Wowhead, Battle for Azeroth ability-damage article (alpha formula settled on AP + WDPS*6)."
        strength: moderate
      - type: source
        ref: "SimulationCraft developer notes still show weapon_speed * AP / 3.5 plus a separate attack_power coefficient. That is a different decomposition of a later build."
        strength: moderate
    status: partially-supported
    notes: "The coefficient-on-a-rating shape is what AstraWeave keeps. The constant 6 is era-specific and is not an engine constant."

  - claim: "Retail armor K comes from a table, and modern K also moves with content difficulty."
    evidence:
      - type: source
        ref: "SimulationCraft dbc_t::armor_mitigation_constant(level). Community writeups (Kyrastus 10.0.5) describe per-difficulty K. Not re-dumped from a 2026 client."
        strength: moderate
    status: partially-supported

  - claim: "Defender percent reductions multiply, so 40% and 20% leave 48% taken, and a stack of sub-100% reductions never reaches 100%."
    evidence:
      - type: source
        ref: "WoW forum threads on Pain Suppression + Ironbark (0.6*0.8=0.48). Consistent across those posts. Not a Blizzard design citation."
        strength: moderate
      - type: calculation
        ref: "§7.3. Integer basis points: 6000*8000/10000 = 4800."
        strength: strong
    status: supported
    notes: "The algebraic claim is true for factors in (0, 1]. Q32.32 sequential truncation can still reach 0 raw. §7.4 specifies an independent floor so the implemented function does not."

  - claim: "Crit is 2.0x in PvE and 1.5x in PvP. Versatility mitigates at half its damage bonus. Stat budget scales as 1.0095^Δilvl."
    evidence:
      - type: assumption
        ref: "Not re-verified against a primary page in this run."
        strength: weak
    status: assumption
    notes: "Not copied. Crit multiplier, crit cap, and stat curves are data."

  - claim: "Path of Exile armour is DR = Armour / (Armour + 5 * raw_hit), so the same armour stops a larger fraction of a smaller hit."
    evidence:
      - type: source
        ref: "https://www.poewiki.net/wiki/Armour"
        strength: strong
    status: supported
    notes: "50% DR when armour = 5 * raw hit. Rejected as the default school curve because EHP then depends on hit size and AI time-to-kill is no longer a property of the defender alone."

  - claim: "Guild Wars 2 strike damage = weapon_strength * Power * coefficient / Armor, Armor = defense + toughness. Condition damage is a separate rating and ignores armor."
    evidence:
      - type: source
        ref: "https://wiki.guildwars2.com/wiki/Damage_calculation and the Damage page."
        strength: strong
    status: supported
    notes: "GW2 also has skills that set strike damage to zero. AstraWeave refuses that as a percent (H9)."

  - claim: "Elden Ring splits a hit-size defense curve (about 10%..90% of the attack) from multiplicative negation, and poise is a separate stagger meter."
    evidence:
      - type: source
        ref: "https://eldenring.wiki.fextralife.com/Calculating_Damage (piecewise attack-ratio table). https://eldenring.wiki.gg/wiki/Poise"
        strength: moderate
    status: partially-supported
    notes: "Fextralife is a secondary wiki. The poise split is well corroborated. The curve is not the AstraWeave default."

  - claim: "FFXIV defense mitigation is a truncated linear percent, floor(15 * Defense / LevelDiv) / 100, and the rest of the damage function truncates per term."
    evidence:
      - type: source
        ref: "https://www.akhmorning.com/allagan-studies/stats/defense/ (Allagan Studies, Dawntrail notes)."
        strength: moderate
    status: partially-supported
    notes: "The lesson taken is explicit truncation at each stage, not the coefficient 15."

  - claim: "Diablo-family armor is a capped hyperbola, and toughness multiplies several independent reduction buckets."
    evidence:
      - type: source
        ref: "https://d4guides.gg/en/guides/mechanics/toughness-in-diablo-4-how-your-defense-really-works (player-measured; author states Blizzard does not publish the formula). Season 11 note: armor applies to all elements in that game."
        strength: weak
    status: partially-supported
    notes: "Bucket multiplication is the idea used. The 0.9 leading factor and the season-11 'armor hits everything' rule are not copied."

  - claim: "On main, gameplay damage is flat subtraction: max(amount - defense * 0.5, 1) as i32, and DamageType is ignored."
    evidence:
      - type: source
        ref: "astraweave-gameplay/src/stats.rs apply_damage, main a3fe3ecd."
        strength: strong
    status: supported

  - claim: "The repo already has four non-shared combat damage paths."
    evidence:
      - type: source
        ref: "gameplay combat.rs (combo, f32, power + weapon); combat_physics.rs (ray, then Stats::apply_damage); gameplay ecs.rs combat_system (Manhattan, subtracts step.damage from CHealth); weaving CombatSystem and enemy_types.rs (f32, Sentinel armor is a flat 30% and EHP = health/(1-armor)); aravath_runtime combat.rs + motor.rs on c86690ab (flat 25 and 12, iframe 0, block * 0.2)."
        strength: strong
    status: supported

  - claim: "Veil scars are Q32.32, BTreeMap, ticked, and hashed. Float conversion exists only in gpu_f32. AI reads stance, not EHP."
    evidence:
      - type: source
        ref: "astraweave-veil fixed.rs, field.rs, lib.rs; astraweave-ai scar_sense.rs; docs/architecture/veil.md on c86690ab."
        strength: strong
    status: supported

  - claim: "WorldSnapshot.EnemyState has hp: i32 and no armor. Adding a field changes planner bytes."
    evidence:
      - type: source
        ref: "astraweave-core/src/schema.rs EnemyState. Roadmap §0 planner_pure hashes PlanIntent bytes."
        strength: strong
    status: supported
```

---

## 5. Research

### 5.1 What was verified, and what was refused

The useful core of the WoW model is small:

1. A **hit-size-independent hyperbola** for armor, with `K` chosen so that `Armor == K` is 50% mitigation and effective health is linear in armor below a cap.
2. **Ability coefficients** on a rating, so a new attack is data (a coefficient) rather than a new formula.
3. **Multiplicative** attacker buffs and **multiplicative** defender reductions, so stacking has a closed form and cannot sum past 100% by accident.
4. **Schools**. Armor does not have to apply to every school. A periodic effect can bypass armor on purpose.
5. **K is a table**, not a float polynomial in level. Classic's `4.5 * 85` expression is exactly the kind of non-integer constant H2 forbids, and the level-60 examples in the user note disagree with the breakpoint used by the armor-penetration writeup (E2).

Refused, with a reason:

| Idea | Why it is not in the resolver |
|---|---|
| Classic `K = 400 + 85*L` and the `4.5*85` extension | Floats, and the breakpoint is contested. A modder-authored `K` row replaces both. |
| `14 AP = 1 DPS` and normalized 1.7/2.4/3.3/2.8 as engine law | Good MMO weapon-speed policy. AstraWeave abilities declare `weapon_bp` and a rating coefficient instead. A slow sword does not silently gain a larger instant. |
| Retail `weaponDPS * 6` | Era constant. |
| Spell `cast/3.5` clamped to 1.5..3.5 | Upper clamp is contested, and cast time would have to be a float. Haste changes **tick counts** of a cast, not a float coefficient. |
| Crit 2.0 / 1.5 and versatility's half-ratio | Not re-verified. Both are data (`lethality` curve, `guard` curve). |
| `1.0095^Δilvl` stat budget | Unverified fit of a live-service treadmill. Not an engine law. |
| PoE `A/(A+5*hit)` as the default | Breaks defender-only EHP. Optional later school mode, not in v1. See §11. |
| Elden Ring piecewise defense | Hit-size dependent, and the published curve is a secondary source. Poise is taken. The curve is not. |
| GW2 division by armor | Same family as the hyperbola (`taken = raw * K / (A+K)` is division by `(1+A/K)`). GW2's zero-armor singularity is avoided because `K > 0` is validated. GW2's separate condition rating is taken (Blight). |
| A percent that is allowed to be 100% | Breaks H9. GW2 has this; we do not. |
| Diablo "one armor stat for every element" | AstraWeave already has `DamageType` {Physical, Echo, Fire, Frost, Shock, Poison}. Schools stay distinct. |

### 5.2 What each compared system is for

| System | Lesson kept |
|---|---|
| WoW classic / retail armor | Hyperbola, `Armor == K` ⇒ 50%, linear EHP, cap creates a flat tail, K is content-scaled. |
| WoW pipeline | Attacker product, then a miss/crit/glance roll, then school mitigation, then defender product, then flat absorb. Order is now **ours** and locked; public WoW order is forum lore. |
| PoE | Hit-size-dependent armor makes big hits punch through. Good for an ARPG spike. Bad for an AI that wants one EHP number. Also a warning: players will stack the layer that the formula underprices. |
| GW2 | One coefficient per skill. Weapon variance is an explicit range. Periodic damage scales off a different rating and does not have to see armor. A base armor above zero keeps division defined. |
| Diablo | Several buckets (armor, resist, generic DR, flat) multiply. Each bucket needs its own ceiling or the product becomes immunity. |
| Elden Ring / Souls | Poise is not HP. I-frames are a timed gate. Negation (a percent) and defense (a curve) are different layers. Split damage applies defense per component, which taxes multi-school hits; v1 applies **one school per hit event**. A multi-school swing is N events, so the sort key stays total. |
| FFXIV | Truncate at every stage and publish the truncation. Potency is data. Crit chance and crit multiplier are different stats. |
| AstraWeave veil | Q32.32, `BTreeMap`, tick counts, golden hash, source scan that forbids `f32` and `HashMap`, save via an explicit meta key, float only at a presentation boundary. |
| AstraWeave roadmap §0 | SHA-256 of a canonical little-endian encoding. `SimRng` triple. No libm. No HashMap iteration. Physics `to_bits` is a separate unsolved gate. |

### 5.3 What AstraWeave has today

**Legacy formula** (`astraweave-gameplay/src/stats.rs`):

```text
mitigated = max(amount as f32 - defense as f32 * 0.5, 1.0) as i32
```

`DamageType` is an argument and is ignored. Defense is flat subtraction, so enough defense pins every hit at 1. That 1 is a floor and an immunity bug at the same time: a hit always lands, and a large hit is reduced by a constant rather than a fraction.

**Four callers, four policies.**

| Path | Where | Policy |
|---|---|---|
| Combo ticker | `gameplay/src/combat.rs` | `step.damage + power`, times optional `echo.power_mult`, `f32` cast to `i32`, then `apply_damage` |
| Physics sweep | `gameplay/src/combat_physics.rs` | Ray, ~60° cone, parry consumes, iframe blocks, then `apply_damage` |
| ECS combo | `gameplay/src/ecs.rs` `combat_system` | Manhattan reach, `CHealth.hp -= step.damage`, no defense |
| Weaving | `astraweave-weaving/src/combat.rs`, `enemy_types.rs` | Flat `f32`. Crit is a TODO (`was_critical = false`). Sentinel `armor: 0.3` is a percent. `effective_health = health / (1 - armor)` |
| Aravath greybox | `aravath_runtime` on `c86690ab` | Sword 25, enemy hit 12. Roll i-frames from tick-local `dt` between 0.08s and 0.32s of a 0.5s roll (`motor.rs`). Block multiplies by 0.2 with no facing test. Death calls `ScarField::inject_death` after `world_metres` quantizes `f32` physics to `i32` |

`docs/architecture/gameplay.md` (trace 1.1) already records that the physics sweep and the ECS combo do not share code, and that both are example-driven. The new crate does not wrap `Stats`. Wrapping it would desync the component that examples still mutate (the project's wrapped-component failure mode).

**RNG.** On main, `astraweave-ecs::Rng` is a world-singleton `StdRng` whose serde impl stores **only the original seed**, not the ChaCha state, so a mid-stream save rewinds the stream. Crafting still has `rand::random` beside `craft_seeded`. The d2 branch replaces sim draws with `SimRng::new(world_seed, tick, system_id)` and reseeds every tick so previous consumption does not leak. Combat follows the d2 contract, not the singleton.

**AI.** `EnemyState` carries `hp: i32` and no mitigation. Planners that hash `PlanIntent` bytes (`planner_pure` in the roadmap) must not gain a snapshot field in the same change as the first resolver. Effective health is a function the AI crate can call. Snapshot growth is a later, hash-versioned change (§11).

**Items.** `ItemKind::Weapon { base_damage: i32, dtype }` and `Armor { defense: i32 }`, plus `EchoMod { power_mult: f32, dtype_override }`. The bridge into combat is `Fixed64::from_int` on the integers. `power_mult` becomes a modifier id with a basis-point value validated at load. The `f32` field stays on the legacy item until the item schema migrates; the resolver never reads it.

**Determinism gate the new case must join.** Roadmap §0 (not yet on main) defines `determinism-replay.yml`: three OS runners, SHA-256 of a canonical encoding, baseline CPU. Combat adds the case `combat_resolve` when the resolver exists. This design locks the first vector in §7.5 so the implementation has a number to hit.

---

## 6. The system, in one pass

A hit that has already passed spatial tests is a `HitEvent`. The resolver is a pure function of `(tables, attacker, defender, attack, SimRng)`. It writes a `HitOutcome` of fixed-point stage values and flags.

```text
gates (dead, iframe, parry)          → 0, and the four RNG words are still drawn
base = weapon_roll + rating * coeff + flat
attacker product, modifiers sorted by id
crit, then glance
school hyperbola: taken = value * K / (mitigation + K), then cap
defender percent product, sorted by id, then independent floor
flat block and absorbs, clamped so they cannot cross the floor
saturating apply to HP and to poise
```

Schools in the shipped table: `Physical`, `Echo`, `Fire`, `Frost`, `Shock`, `Poison`. These are the existing `DamageType` variants. A seventh flag, `Bleed`, is not a school. It is an attack bit that skips the school hyperbola and still takes Guard, Bulwark, and the floor.

Expected damage for AI uses the same function with the variance roll forced to the table's displayed midpoint and crit applied as its expectation `1 + p*(m-1)`, in integers. It does not draw.

---

## 7. Formulas, proofs, worked integers

### 7.1 Representation

- `Fixed64` raw `i64`. One unit of health is `SCALE = 2^32` raw.
- A basis point is an integer percent of a percent. `10_000 bp = 100%`. `0 bp = 0%`.
- Chances and percent modifiers are `u32` basis points, validated into a per-field range.
- Display HP is `floor(raw / SCALE)`. Death is `raw <= 0` after an applied hit. Sub-unit health is real state and is saved.

Truncation is toward zero at every `/`. For the non-negative quantities this pipeline allows, that is floor. There is no banker's round and no `libm`.

### 7.2 Armor hyperbola

Let `A >= 0`, `K > 0`, both whole units, damage `D >= 0`, all stored as `Fixed64`.

```text
taken_raw = (D_raw * K_raw) / (A_raw + K_raw)     # i128 product, trunc toward 0
```

`A_raw + K_raw` cannot overflow `i64` for legal tables: validation requires `A_max + K <= i64::MAX / 4` in raw units (§8.3).

**Armor == K ⇒ exactly half, for every even raw damage.**

If `A_raw = K_raw = K` and `K > 0`,

```text
taken_raw = (D_raw * K) / (2K) = D_raw / 2     # integer division
```

`D_raw / 2` equals the real half when `D_raw` is even. Whole-unit damage has `D_raw = n * 2^32`, which is even. So every whole-unit hit with `Armor == K` is exactly 50%.

Checked: `D = 100 * SCALE`, `K = A = 400 * SCALE` ⇒ `taken = 50 * SCALE = 214748364800`.

When `D_raw` is odd, `D_raw / 2` is one raw unit below the non-representable exact half. That one-ulp gap is the specified rounding. It does not depend on OS.

**Cap.** Default `armor_cap_bp = 7500` (75%). Engine ceiling `9000` (90%). If `A * (10_000 - cap) > K * cap` in the cross-multiply (the integer form of `A/(A+K) > cap`), replace the hyperbola with

```text
taken_raw = D_raw * (10_000 - cap_bp) / 10_000
```

At cap 75%, `A = 3K` gives hyperbola `K/(3K+K) = 1/4`, and the cap formula also gives `1/4`. The switch is continuous. Above `3K`, taken stays `D/4` and extra armor does nothing.

Checked: `D = 100`, `K = 400`, `A = 1200 = 3K` ⇒ taken `25`. `A = 2000` with the cap also taken `25`.

**EHP, below the cap.**

```text
EHP = HP * (A + K) / K          # whole units, trunc toward 0
```

Untruncated, `EHP = HP * (1 + A/K)`, so `d(EHP)/dA = HP/K`, a constant. Integer step: moving armor from `A` to `A+Δ` changes EHP by `HP * Δ / K`, exact when `K` divides `HP * Δ`.

Checked, `HP = 1000`, `K = 400`:

| Armor | EHP |
|---:|---:|
| 0 | 1000 |
| 400 | 2000 |
| 800 | 3000 |
| 1200 | 4000 |

Slope per 400 armor is 1000. At the 75% cap (`A > 1200` here, since `3*400 = 1200`), EHP stays `4000`. A non-dividing step truncates: `HP=1000`, `K=400`, `A=1` ⇒ `EHP = 1000*401/400 = 1002` (the real value is 1002.5). Property tests allow a slack of one unit on that division and require exact equality when `K` divides `HP*Δ`.

**Penetration.** `A_eff = A * (10_000 - pen_bp) / 10_000`, with `pen_bp` clamped to `pen_cap_bp` (default 7500, ceiling 9000). Then the hyperbola runs on `A_eff`. Pen never makes `K` zero. Negative results clamp to 0 before the division.

**Other schools** use the same function with that school's resistance stat and that school's `K` row. Echo reads Ward, Fire reads Heat, Frost reads Rime, Shock reads Volt, Poison reads Toxin. A `Bleed` bit skips this stage.

### 7.3 Attacker product, defender product, and the 48% case

Attacker modifiers are `(1 + b_i)` in basis points. Defender modifiers are `(10_000 - c_i)`. Both lists are sorted by `modifier_id`. Each step is

```text
raw = raw * factor_bp / 10_000     # i128, trunc toward 0
```

**40% and 20%.** Factors `6000` and `8000` bp. Fused: `6000 * 8000 / 10_000 = 4800` bp, which is 48% taken. Sequential on a whole-unit hit of 1000:

```text
1000 * SCALE * 6000 / 10000 = 600 * SCALE
600 * SCALE * 8000 / 10000 = 480 * SCALE
```

Exact. The user note's `0.6 * 0.8 = 0.48` holds in this integer path for this pair.

**Percent product never reaches 0, as a real number**, when every `c_i <= c_max < 10_000` and the count is finite:

```text
P >= ((10_000 - c_max) / 10_000) ^ n > 0
```

That algebraic statement is not an implementation. Sequential Q32.32 truncation of many small factors underflows. A direct check: twelve factors of "keep 10%" applied to `1000 * SCALE` finish at raw `4` (about `10^-9` HP). Eight such factors finish near `10^-5` HP. A basis-point accumulator that only keeps four digits hits 0 even sooner. Five stacked 90% reductions in a `10_000`-denominated integer reach 0.

**Implemented rule, which is what the proof attaches to:**

1. Engine ceiling: at most **8** defender percent modifiers, each `c_i <= 9000` bp (keep at least 10%). Mods cannot raise these.
2. Let `A` be the post-school value (after armor). Compute the sequential product `P` as above.
3. Independently, `floor = A * min_taken_bp / 10_000`. Default `min_taken_bp = 500` (5% of post-school). Engine floor `100` (1%). Mods may raise the default; they cannot lower it past 100.
4. Result of the percent stage is `max(P, floor)`.
5. If `A_raw > 0` and that result is `0` raw (only possible for a tiny `A`), add **1 raw unit**. A positive input cannot become a zero output inside the percent stage.

Together with the armor cap, default ceilings leave at least `25% * 5% = 1.25%` of pre-armor damage, and the absolute engine ceilings leave at least `10% * 1% = 0.1%`, unless a gate says the hit does not connect. Flat block is not allowed to cross this floor (§7.4).

**Gates** are outside the proof on purpose. I-frames, parry, and "already dead" return 0. They are booleans, not percents, and they match Aravath's roll and the existing parry consume.

### 7.4 Full stage order, overflow, rounding

```text
0. Draw exactly four u32s: variance, crit, glance, evasion. Always, even if a gate then zeroes the hit.
1. If target HP <= 0 at the start of this hit (after earlier hits in the sorted batch): flags |= TargetDead, delta = 0. Stop applying.
2. If iframe ticks_left > 0: flags |= Immune, delta = 0.
3. If parry window open: flags |= Parried, consume the window, delta = 0.
4. If evasion roll says miss (chance clamped to evasion_cap_bp, default 0, ceiling 4000): flags |= Evaded, delta = 0.
5. Base. See §7.6.
6. Attacker product. Each bonus bp <= attacker_bonus_cap_bp (default 10_000, meaning +100%, ceiling +200% = 20_000 bp). Count <= 8.
7. Crit. Crit if (crit_roll as u64) * 10_000 < (chance_bp as u64) << 32. Chance comes from the Precision curve and is clamped to crit_cap_bp (default 7500, ceiling 9500). An ability flag always_crit bypasses the roll and the cap. Multiplier is base_crit_bp (default 15_000 = 1.50x) plus Lethality, clamped to crit_mult_cap_bp (default 20_000 = 2.00x, ceiling 30_000).
8. Glance. Default chance 0. When the tier row sets a chance, a matching compare multiplies by glance_bp (default 7500). The roll was already drawn.
9. School hyperbola and cap (§7.2), unless the Bleed bit is set.
10. Defender percent product and floor (§7.3). Bulwark's percent, when the block gate is open and in_arc is true, is one of these modifiers.
11. Flat. Subtract Block flat, then absorb, each capped by the table's max_flat. After each subtraction, if the value would fall below the floor from step 10, pin it to that floor. A flat cannot mint immunity.
12. Apply. HP = saturating_sub(HP, delta). Poise = saturating_sub(poise, attack.poise). Hitting 0 poise sets `Staggered` and refills poise to max. The stagger lasts `stagger_ticks` from the table, default **12** (0.2 s at 60 Hz), and never 0. If the i128 intermediate of any stage exceeds i64::MAX, clamp the stage to i64::MAX raw and set flags |= Saturated. Widen to `i128` **before** multiplying. `saturating_mul` on `i64` is the wrong helper: it clamps before the division and breaks the half-damage identity for large K. The `Saturated` clamp lives in `resolve_hit`, including tables built in tests that skipped the loader.
```

`Saturated` is telemetry. Legal tables are rejected at load if the maximum chain (max base × max attacker product × max crit) does not fit in `i128` before the divisions, and if the max applied delta exceeds `2^31` whole units. That load check is the overflow proof for content; the clamp is the backstop for a bug.

Healing is `resolve_heal`, a separate function: `Mending` coefficient, attacker heal modifiers, no armor, no pen, no crit unless the heal ability sets `always_crit` or a heal-crit flag. Heals add to HP up to max. They are not negative damage routed through armor, because that would make armor reduce healing.

**DoT residue.** A periodic tick is a `HitEvent` with the Bleed or Periodic bit, sourced from a component the tick system emits in entity-id order. The same resolver runs. Because HP is Q32.32, a tick of `1/60` of a point is `SCALE/60` raw and is representable. The 1-raw backstop in §7.3 covers the case where mitigation would truncate a legal positive tick to nothing.

### 7.5 Canonical worked hit

Inputs, all whole units unless noted:

- Weapon roll locked at 20 (ability variance band `[10000, 10000]`, so the variance word is drawn and ignored)
- Might 100, Might coefficient `1/10`, flat 0 → Might contributes 10. **Base = 30**
- Attacker modifiers id 1 = +15% (`11500` keep-form is the bonus path: factor `11500`), id 2 = +20% (factor `12000`)
- No crit (chance 0), no glance, no evasion
- Armor 400, K 400, pen 0, cap 75%
- Defender modifiers id 10 = 40% (`6000` kept), id 11 = 20% (`8000` kept), `min_taken_bp = 500`
- Flat block = 2, and the floor does not bind

| Stage | raw `i64` | Reading |
|---|---:|---|
| S0 base | 128849018880 | 30 |
| S1 ×11500/10000 | 148176371712 | 34.5 exactly |
| S2 ×12000/10000 | 177811646054 | trunc(41.4 × 2^32) |
| S3 ×1/2 | 88905823027 | exact half of S2 |
| S4 ×6000/10000 | 53343493816 | |
| S5 ×8000/10000 | 42674795052 | |
| S6 − 2×2^32 | 34084860460 | |

The real-number story is `30 * 1.15 * 1.20 = 41.4`, times `0.5 = 20.7`, times `0.48 = 9.936`, minus `2 = 7.936`. The integer result is **one raw unit** below `7936 * 2^32 / 1000` (`34084860461 - 34084860460 = 1`). That 1 ulp is the truncation budget of this vector. It is part of the contract, not an error.

Canonical bytes: the seven `i64` values, little-endian, concatenated. SHA-256:

```text
071dc70e116987782a8a8b137a6cce648d390a82967726775308946f1e69da71
```

The first `combat_resolve` golden vector is this digest. A second vector in the same test is the Armor=K case `100 → 50` with digest of that single `i64` (`50 * 2^32 = 214748364800`). A third is `1000` through 40% then 20% ⇒ `480 * SCALE`, exact.

### 7.6 Base damage and the variance roll

```text
rating = Might     if school == Physical and the Bleed bit is off
       = Blight    if the Bleed bit is on, or school == Poison and the ability says periodic
       = Focus     otherwise

base_raw = weapon_rolled_raw
         + rating_raw * coeff_num / coeff_den
         + flat_raw
```

`coeff_num` and `coeff_den` are `u32`, `coeff_den > 0`, validated.

Weapon roll, inclusive, from the four-draw variance word `r`:

```text
span = max_bp - min_bp          # validated max >= min
pick = min_bp + ((r as u128) * (span as u128 + 1)) >> 32
weapon_rolled = weapon_min + (weapon_max - weapon_min) * pick / 10_000
```

`r = 0` yields `min_bp`. `r = 2^32 - 1` yields `max_bp`. Checked for `[9500, 10500]`. The low-end bias of this integer map is deterministic and is locked by those two endpoints, not argued away.

Default band for a new ability is `[9500, 10500]` (the retail-style ±5% the user note describes; treated as a content default, not a verified client constant). The Aravath sword and the Aravath enemy swipe ship as `[10000, 10000]`, so they stay flat 25 and flat 12 until a designer opens the band. Opening the band changes the table hash.

### 7.7 Expected value (AI, no RNG)

```text
mid_bp = (min_bp + max_bp) / 2          # trunc
expected = resolve with variance forced to mid_bp,
           crit multiplier replaced by
           (10_000 + chance_bp * (crit_mult_bp - 10_000) / 10_000) / 10_000
           in basis points, glance and evasion at their chances the same way
```

Time-to-kill is `ehp_raw / expected_raw` in integers. `expected_raw == 0` yields `u64::MAX` and a flag, not a div-by-zero. Effective health for a school, below the cap, for the default defender product (no temporary cooldowns) is the §7.2 EHP, computed in `i128` and clamped to `i64::MAX` with `Saturated` on the query if the clamp binds. With temporary cooldowns, `effective = hp * 10_000 / taken_bp_of_those_modifiers`, using the same floor rule, still with no RNG.

The floor is 5% of **this hit's** post-school value. Splitting one hit into two does not invent an extra minimum: the sum of the fragment floors equals 5% of the sum when the divisions agree. The 1-raw backstop is at most one ulp per hit.

---

## 8. Stats

Each stat is a non-negative integer in the combatant component. Curves (how gear grants the integer, how Precision becomes a chance) are table rows, not code. The resolver sees the already-curved integers.

### 8.1 Offense (damage dealt)

| Stat | Role |
|---|---|
| **Might** | Rating for Physical direct hits. |
| **Focus** | Rating for Echo, Fire, Frost, Shock direct hits. |
| **Blight** | Rating for Bleed and other periodic ticks. A direct hit does not read it. |
| **Precision** | Input to the crit-chance curve. The curve's output is clamped to `crit_cap_bp`. |
| **Lethality** | Added crit multiplier, clamped to `crit_mult_cap_bp`. |
| **Haste** | Subtracts ticks from cooldowns and casts, with a minimum of 1 tick. Not a factor inside `resolve_hit`. |
| **Penetration** | Basis points of the relevant mitigation stat ignored, clamped to `pen_cap_bp`. |

### 8.2 Defense (damage received)

| Stat | Role |
|---|---|
| **Vitality** | Maximum HP, linear. Current HP is separate state. |
| **Armor** | Physical hyperbola. |
| **Ward** | Echo hyperbola. |
| **Heat, Rime, Volt, Toxin** | Fire, Frost, Shock, Poison hyperbola. |
| **Guard** | Generic percent reduction. One defender modifier sourced from the Guard curve. Shares the 8-slot cap and the floor. |
| **Bulwark** | Only while `BlockGate { active, in_arc }` is open. Contributes a percent (inside the product) and a flat (after it, floor-clamped). This is the shield, not a random block roll. |
| **Poise** | Stagger meter. Hits subtract `attack.poise`. At 0 the outcome flag is `Staggered` and poise refills to max. Not HP. Regen delay is a tick count in data. |
| **Evasion** | Optional miss chance, ceiling 40%, default 0. Player dodge is the iframe gate. Evasion is for creatures that do not have a roll. |

### 8.3 Healing

| Stat | Role |
|---|---|
| **Mending** | Heal rating. `resolve_heal` only. |

A heal's value in a fight is its HP times the target's EHP factor. The function that reports that is `effective_heal = heal_raw * (A + K) / K` for the school the designer marks as the creature's main incoming school. The HP added is still the raw heal. Armor does not reduce the heal.

### 8.4 Engine ceilings (not moddable upward)

| Ceiling | Value | Why |
|---|---|---|
| Defender percent modifiers | 8 | Bounds the product and the sort |
| Attacker bonus modifiers | 8 | Same |
| Single defender reduction | 9000 bp | Keeps each factor ≥ 10% |
| `min_taken_bp` | ≥ 100 | Floor cannot be removed |
| Armor / resist cap | ≤ 9000 bp | Hyperbola cannot be set to 100% |
| Pen cap | ≤ 9000 bp | A target keeps some of its curve |
| Crit chance cap | ≤ 9500 bp | A non-flagged hit can fail to crit |
| Crit multiplier cap | ≤ 30000 bp | Bounds the i128 chain |
| Evasion cap | ≤ 4000 bp | Cannot replace i-frames |
| Max applied delta | 2^31 whole units | Saturation backstop |
| K | ≥ 1 and ≤ 10^12 whole units | Denominator defined, table bounded |

Validation failure refuses the table. The previous good table stays loaded. A tick with no table deals no damage and raises `CombatEvent::TablesMissing`. It does not fall back to `defense * 0.5`.

---

## 9. Data schema

RON, loaded once, semantically hashed. The hash is over the validated struct (little-endian integers, ids sorted), not over source whitespace. Mods are a list of patches applied in `mod_id` order. A patch replaces a row with the same id or appends. Unknown school ids, `K < 1`, a ceiling breach, or a duplicate id inside one patch fail the load.

```ron
CombatTables(
    schema: 1,
    ceilings_locked: true,
    draw_count: 4,
    defaults: (
        armor_cap_bp: 7500,
        pen_cap_bp: 7500,
        crit_cap_bp: 7500,
        crit_base_bp: 15000,
        crit_mult_cap_bp: 20000,
        min_taken_bp: 500,
        glance_bp: 7500,
        variance_bp: (9500, 10500),
        evasion_cap_bp: 0,
    ),
    schools: [
        (id: 1, name: "physical", k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
        (id: 2, name: "echo",     k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
        (id: 3, name: "fire",     k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
        (id: 4, name: "frost",    k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
        (id: 5, name: "shock",    k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
        (id: 6, name: "poison",   k_by_tier: [(tier: 0, k: 100)], cap_bp: 7500),
    ],
    curves: [
        (id: "precision_to_crit", points: [(0, 0), (100, 1500), (400, 4500)]),
        (id: "guard_to_bp",       points: [(0, 0), (100, 1000)]),
    ],
    abilities: [
        (
            id: 1, name: "sword_light", school: 1,
            weapon_bp: 10000, might_num: 0, might_den: 1,
            focus_num: 0, focus_den: 1, blight_num: 0, blight_den: 1,
            flat: 25, poise: 10,
            variance_bp: (10000, 10000),
            flags: (),
        ),
        (
            id: 2, name: "enemy_swipe", school: 1,
            weapon_bp: 10000, might_num: 0, might_den: 1,
            focus_num: 0, focus_den: 1, blight_num: 0, blight_den: 1,
            flat: 12, poise: 8,
            variance_bp: (10000, 10000),
            flags: (),
        ),
    ],
)
```

Tier 0 `K = 100` means 100 armor is 50% at Aravath scale. It is a content default so the greybox can put a number on an enemy without a 5500-point classic budget. A second tier is a new row, not a code change.

Curves are piecewise linear between integer keypoints, evaluated with the same truncating division. At most **64** keypoints. Evaluation is a binary search on the sorted slice. Extrapolation clamps to the end key. A descending chance curve is legal only if the author sets `monotonic: false`; the default validator requires non-decreasing chance curves.

The semantic hash walks the validated struct in field order. Ids and keypoints are sorted first. Parser key order and whitespace are not inputs. `schema` is part of the hash. An unknown schema refuses the load. A future v2 reader that still accepts v1 hashes the v1 interpretation.

School patches may not rename an existing id. Abilities store the school id only. `Bleed` implies `Periodic`, and `Bleed` is legal only on `Physical`. `Poison` + `Bleed` fails validation, so a periodic poison always hits Toxin and cannot also take the armor bypass.

`BlockGate.in_arc` defaults to **true whenever `active` is true** (`block_omnidirectional: true` on the greybox profile). A later profile can require a front arc. The resolver receives the boolean. It does not compute a float dot. Quantizing that boolean is the caller's physics-boundary problem, same as `world_metres` for scars.

Greybox Bulwark row for the held shield: `percent_bp: 8000` (keep 20%), `flat: 0`. That is Aravath's `0.2` multiplier, expressed as a modifier, and it sits above the 5% floor so the floor does not change it.

I-frame window for the roll, in ticks at 60 Hz, as data on the motor ability rather than `0.08 * dt`: `start_tick: 5`, `end_tick: 19`, `roll_ticks: 30`. Those are the integer readings of 0.08s, 0.32s, and 0.5s at 60 Hz (`ceil(0.08*60)=5`, `floor(0.32*60)=19`, `0.5*60=30`). This quantization is a behavior change from a variable-`dt` window and is called out in §15.

---

## 10. Crate and API

**Crate:** `astraweave-combat`.

**Dependencies:** `astraweave-core` (for `SimRng`, `Entity`), `astraweave-math` (for `Fixed64` after M0), `astraweave-ecs` (systems only). No dependency on `astraweave-physics`, `astraweave-gameplay`, `astraweave-veil`, or `astraweave-ai`. AI may depend on `astraweave-combat` for `effective_health`. The demo depends on `astraweave-combat` and still owns raycasts.

```rust
pub const SIM_SYSTEM_COMBAT: u64 = 0xC0BA_7001;

pub struct CombatTables { /* private, hashed */ }

pub struct AttackDef { /* id, school, coeffs, flags, variance, poise */ }

pub struct Modifier {
    pub id: u32,
    pub kind: ModifierKind, // AttackerBonusBp, DefenderReduceBp, FlatAbsorb
    pub bp_or_flat: u32,
}

pub struct BlockGate { pub active: bool, pub in_arc: bool }
pub struct ImmuneGate { pub iframe_ticks: u16, pub parry_open: bool }

pub struct StageTrace {
    pub raw: [i64; 8],
    pub flags: HitFlags,
}

pub fn resolve_hit(
    tables: &CombatTables,
    atk: &AttackerView<'_>,
    def: &DefenderView<'_>,
    attack: &AttackDef,
    rng: &mut SimRng,
) -> HitOutcome;

pub fn resolve_heal(...) -> HealOutcome;
pub fn effective_health(tables: &CombatTables, def: &DefenderView<'_>, school: SchoolId) -> Fixed64;
pub fn expected_hit(...) -> Fixed64;

pub fn hit_rng(world_seed: u64, tick: u64, ordinal: u64, attack_id: u64) -> SimRng {
    let system = SIM_SYSTEM_COMBAT
        .wrapping_add(ordinal.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add((attack_id as u64).rotate_left(7));
    SimRng::new(world_seed, tick, system)
}
```

`resolve_hit` draws exactly `tables.draw_count` (4) `u32`s and no more. Views borrow modifier slices the caller already sorted; the function checks sortedness in debug and sorts into a stack array of 8 in release only if a `must_sort` flag is set. The ECS system sorts before the call, so the hot path does not sort.

`StageTrace` is eight `i64`s. No `Vec`.

**System id** `0xC0BA_7001` does not collide with the d2 constants `0xC2AF_7001`, `0xC2AF_7002`, `0xAE11_7001`, `0xAE11_7002`, `0xAE11_7003`, `0x7E22_7001`. It is registered beside them when M0/M2 touches `sim.rs`.

---

## 11. ECS, save, AI, demo

**Stage.** `combat_resolve_system` runs in `simulation`, after the systems that enqueue `HitEvent`s and before death cleanup. Perception on this tick still sees last tick's HP. That one-tick delay is the existing stage order (`PERCEPTION` then `SIMULATION` then `AI_PLANNING`). The design does not move stages.

**Sort key** of the batch: `(attacker_entity, defender_entity, attack_id, seq)`. `seq` is assigned by the producer in schedule order. The batch applies in that order, so the lower attacker id lands the killing blow when two hits would both kill. Later hits in the same tick see the reduced HP and take the `TargetDead` gate. Scar injection runs once, on the transition to `<= 0`, after the batch, in defender-id order.

**Ordinal** is the index in that sorted batch. Producers enqueue a hit that i-frames will cancel. Filtering before enqueue would renumber later ordinals and change their rolls. This is a correctness rule, not an optimization.

**Components** (new, not stuffed into `Stats`):

- `CVitals { hp: Fixed64, hp_max: Fixed64, poise: Fixed64, poise_max: Fixed64 }`
- `COffense { might, focus, blight, precision, lethality, haste, pen }` as `i64` whole units or bp, as the schema marks each field
- `CDefense { armor, ward, heat, rime, volt, toxin, guard, bulwark_bp, bulwark_flat, evasion_bp }`
- `CModifiers { entries: [Modifier; 8], len: u8 }` kept sorted
- `CBlock { active: bool, in_arc: bool }`
- `CImmune { iframe_ticks: u16, parry_ticks: u16 }`

`CHealth` in `astraweave-core` stays the legacy `i32` until a migration reads `CVitals` and writes both, or until callers move. Writing both from two formulas is forbidden. During M4 only the demo reads `CVitals`.

**Save.** Components serialize raw `i64`. The table hash is stored in save meta under `combat_tables`. A load whose hash differs from the running table refuses to enter the combat system and reports the mismatch. It does not recompute old hits with new K. Scar meta `veil_scars` is untouched. Saves happen on tick boundaries. `SimRng` is not stored per hit; `(world_seed, tick, ordinal, attack_id)` reconstructs it. Mid-tick saves are out of contract, matching the d2 reseed rule.

**AI.** `effective_health` and `expected_hit` are pure. A threat table, if the AI crate adds one, is a `BTreeMap<(Entity, Entity), i64>` updated from `DamageDealt` events in entity order. `WorldSnapshot` does not grow in M1–M3. Adding `ehp` to `EnemyState` is a separate change that re-locks `planner_pure` and is listed as residual risk R4.

**Demo.** Aravath keeps raycasts, lock-on, and the roll motor. The motor's iframe output becomes `iframe_ticks` from the tick window in §9. The sword and the swipe submit `HitEvent`s. `apply_damage_multiplier` is deleted from the demo when M4 lands, not wrapped. Death still calls `inject_death` from the death event.

**Legacy.** `Stats::apply_damage`, the ECS combo's raw subtraction, and weaving's `f32` combat stay until M5 ports or explicitly marks them legacy. They are not called by `astraweave-combat`. New content does not add a fifth formula.

---

## 12. Test plan

Resolver tests do not boot physics or a GPU.

| Test | Assert |
|---|---|
| Golden half | `100` at `A = K` ⇒ raw `214748364800` |
| Golden pipeline | Seven stage raws and SHA-256 `071dc70e…da71` |
| Golden 48% | `1000` through 40% then 20% ⇒ `480 * SCALE` |
| EHP grid | `HP=1000, K=400, A=0,400,800,1200` exact; `A=1` ⇒ `1002`; `A=2000` with 75% cap stays at the `A=3K` value |
| Floor | Eight modifiers at 9000 bp on a 1000-point hit equals `max(truncated product, 5% of post-school)`, and the result is `> 0` |
| Cap continuity | `A = 3K` matches the cap formula exactly at 75% |
| Pen clamp | `pen` above the cap equals pen at the cap; `A_eff` never negative |
| Crit endpoints | chance 0 never crits; chance 7500 crits only when the compare says so; `always_crit` ignores the cap |
| Variance endpoints | roll 0 → min bp; roll `u32::MAX` → max bp |
| Draw count | A gate-zeroed hit still consumes 4 `u32`s from its `hit_rng` |
| Ordinal stability | Enqueue order swapped, same sort key, same outcomes |
| Dead gate | Two killing hits: only the first in sort order changes HP; one death event |
| Block | `in_arc: false` ignores Bulwark; `active` with omnidirectional greybox applies 8000 bp |
| I-frame | delta 0, flag `Immune`, parry not consumed |
| Parry | delta 0, window consumed |
| Duplicate modifier id | `Err`, no silent drop |
| Table ceilings | `K = 0`, cap 10000, 9th modifier, negative Might all fail load |
| Overflow backstop | A hostile in-memory table that skips validation sets `Saturated` and does not panic |
| Widen-then-mul | The 50% identity still holds when `K_raw` is above `2^31` (product does not take the `i64` saturating path) |
| Hash stability | Two RON documents with the same rows in different key order produce one table hash |
| Bleed tag | `Bleed` without `Periodic`, or `Bleed` on Poison, fails load |
| Double writer | A second system writing `CVitals.hp` in the same tick trips the apply counter |
| Source scan | Sim modules contain no `f32`, `f64`, `HashMap`, `SystemTime`, `Instant`, `unit_f32` |
| Cross-OS | `combat_resolve` case in `determinism-replay.yml` once that workflow exists; until then the golden SHA-256 is a single-machine lock and the gap is residual risk R1 |
| Property | For all `A` in `0..3K` step `K/4`, EHP step equals `HP/4` when `K` divides `HP*K/4` |
| AI | `expected_hit` with a 0% crit and a `[10000,10000]` band equals `resolve_hit` |
| Save | Table hash mismatch refuses resolve; matching hash round-trips `CVitals` raw |

No new float epsilon. Comparisons are `==` on `i64`.

---

## 13. Implementation milestones

Each milestone is a later change with its own review. This document does not authorize starting M0.

| Milestone | What lands | Done when |
|---|---|---|
| **M0** | Move `Fixed64` to `astraweave-math`. Veil, when it is on main, re-exports it. Add `SIM_SYSTEM_COMBAT` next to the d2 constants. If `SimRng` is not on main yet, M0 stops after `Fixed64` and records the block. | Veil's existing golden hashes still match. `Fixed64` has one definition. |
| **M1** | `astraweave-combat` library: load, validate, `resolve_hit`, `resolve_heal`, `effective_health`, `expected_hit`, golden tests, source scan. No ECS. | `cargo test -p astraweave-combat` green. Clippy `-D warnings`. The three golden digests match §7.5. |
| **M2** | ECS components and `combat_resolve_system` in `simulation`. Reused event buffer. Death event. | A headless 100-hit batch is allocation-stable after warm-up (`alloc_plot` or a counting allocator already in `astraweave-ecs`). Sort-order test from §12. |
| **M3** | AI calls `effective_health` / `expected_hit` from a non-snapshot helper. | `planner_pure` bytes unchanged. A unit test shows time-to-kill prefers the lower-EHP target. |
| **M4** | Aravath sword, swipe, block, and roll ticks call the resolver. Scar inject stays on the death event. | Demo health after one unblocked sword hit is 25 below max in raw whole units. Blocked swipe keeps 20%. I-frame swipe keeps 100%. |
| **M5** | Port or quarantine `Stats::apply_damage`, `combat_system`'s raw subtraction, and weaving flat damage. Update `docs/architecture/gameplay.md` and add `docs/architecture/combat.md` via the trace template. `aw_trace_sync --write`. | One production formula. Trace table regenerated by the tool. |
| **M6** | `combat_resolve` required in the roadmap's replay-hash workflow. | Three-OS digest match, or the workflow is still absent and the milestone stays open. |

M5's trace is the architecture-trace obligation. It is not done in this design commit because the crate does not exist yet, and a trace of vaporware would fail the "last verified commit" rule.

---

## 14. Acceptance criteria

The design is acceptable for implementation planning when all of the following are true:

1. The orchestrator has accepted the r3 gate in §18, including residual risks R1–R6.
2. Every formula in §7 is an integer operation with an explicit truncation, and the three golden numbers in §7.5 are the ones implementation must reproduce.
3. `Armor == K` is exactly half for whole-unit damage; EHP is linear below the cap; the 40%+20% case is exactly 48% taken; the percent stage cannot return 0 raw for a positive input.
4. I-frames, parry, and death are gates. No data file can express a 100% reduction.
5. The resolver's inputs are reproducible from `(world_seed, tick, ordinal, attack_id)` plus components, with modifiers sorted by id.
6. Mods can replace K, curves, and coefficients, and cannot raise the ceilings in §8.4.
7. The crate boundary does not add a second formula inside `Stats`, weaving, or the physics sweep.
8. `WorldSnapshot` layout is unchanged by M1–M3.
9. Scar injection stays a subscriber to death, not a term in the damage product.

---

## 15. Error taxonomy and residual risks

| ID | Failure | Likelihood / impact | Mitigation / status |
|---|---|---|---|
| E1 | Sequential percent product truncates to 0 | Certain if the floor is omitted. High impact (immunity). | Independent floor + 1-raw backstop. §7.3. Covered by the eight-modifier test. |
| E2 | Implementer encodes classic `4.5 * 85` and breaks H2 | Medium / high | K is a table of integers. The contested 5500-vs-5882.5 breakpoint is documented so nobody "fixes" it with a float. |
| E3 | I-frame filtered before enqueue, later crits change | Medium / high | §11 ordinal rule. Test: a cancelled hit still occupies its ordinal. |
| E4 | Flat block larger than the hit deletes damage | Certain if unclamped | Step 11 pins to the floor. |
| E5 | Two hits, one target, kill credit flips with `HashMap` order | Certain if unsorted | Total sort key. Lower entity id wins. Documented, tested. |
| E6 | `rand` crate bump changes ChaCha12 | Low / high | Pin `rand`. Golden-test `SimRng::new(1, 2, SIM_SYSTEM_COMBAT).next_u32()` once M0's API is on main. The mix vector `mix_sim_seed(1,2,3)` is already locked on d2. |
| E7 | Save scum of a crit | Certain, by H3 | Accepted. Tick-seeded RNG is replayable. That is the point of the gate. |
| E8 | Mod sets cap to 100% | Certain if validation is advisory | Load refusal. Previous table kept. |
| E9 | Legacy `apply_damage` and the new resolver both write HP | Medium / high | Different components until M5. Demo M4 does not call `apply_damage_multiplier`. |
| E10 | Physics `f32` makes `in_arc` differ across OS | Medium / medium for the full-frame hash; **none** for `combat_resolve` | Resolver sees a bool. Spatial determinism stays WS-D3. R1. |
| E11 | Roll iframe quantized to ticks feels different from 0.08–0.32s at variable dt | Medium / low (feel) | Numbers written in §9. M4 can retune the three integers without a formula change. |
| E12 | Heals routed through armor | Low / medium | Separate function. |
| E13 | Snapshot `ehp` added "while we're here" and `planner_pure` moves | Medium / high | Forbidden until a versioned snapshot change. R4. |
| E14 | `Fixed64` forked a second time | Medium / high | M0 puts it in `astraweave-math`. Combat does not vendor a divergent copy. |
| E15 | DoT of a fraction of a point vanishes | Low / medium | Q32.32 HP plus the 1-raw backstop. |
| E16 | Nine modifiers needed by a boss fight | Medium / low | The 9th is a load error. Design response is a single pre-baked modifier, not a higher ceiling, unless a later review raises the ceiling and re-proves the `i128` bound. |

```yaml
residual_risks:
  - id: R1
    text: "combat_resolve golden digest is specified but cannot be compared across Windows/macOS until determinism-replay.yml exists. Single-OS equality is necessary and not sufficient for H1."
  - id: R2
    text: "SimRng is not on main (a3fe3ecd). It is on cursor/engine-ws-d2-stab. M1 blocks if that API changes or does not merge."
  - id: R3
    text: "Veil's Fixed64 is on the Aravath branch, not main. M0 is a cross-branch extract. A divergent copy would split the fixed-point contract."
  - id: R4
    text: "AI target choice does not see EHP inside WorldSnapshot until a later hash-versioned change. Planners keep using raw hp until that lands."
  - id: R5
    text: "Default K=100, 75% cap, 5% floor, and ±5% variance are content defaults chosen for Aravath-scale integers. They are not a balance pass."
  - id: R6
    text: "Rapier hit detection can still disagree across OS. This design keeps that disagreement outside the resolver. A full-frame combat hash will move until WS-D3 is closed, even if resolve_hit is stable."
```

---

## 16. Dual review, first pass

### 16.1 Advocate

The proposal is one pure Q32.32 function, `resolve_hit`, in a new crate `astraweave-combat`. It keeps WoW's hyperbola (`Armor == K` is exactly 50% for whole-unit hits; EHP is linear in armor below a continuous cap), WoW's multiplicative buff and cooldown stacks, GW2's per-ability coefficient and separate periodic rating, Souls' poise meter and iframe gate, and FFXIV's stage-wise truncation. It refuses hit-size-dependent armor as the default, refuses float K polynomials, and refuses a percent that can be 100%.

Existing AstraWeave combat is four flat or `f32` policies, one of which ignores `DamageType` and floors every hit at 1. The new function does not wrap those. Callers submit `HitEvent`s. Random draws are four `u32`s from `SimRng` keyed by world seed, tick, sort ordinal, and attack id. The canonical 7-stage hit is an exact `i64` vector whose SHA-256 is `071dc70e116987782a8a8b137a6cce648d390a82967726775308946f1e69da71`. Mods own K and coefficients inside published ceilings. The scar field stays a death subscriber.

Evidence for the hyperbola and the classic AP/normalization facts is the Warcraft wiki. Evidence that several celebrated constants (level-60 K, spell upper clamp, retail crit split, 1.0095 ilvl) are unsafe to copy is recorded as contested or unverified in §4. The integer proofs were executed, not sketched: the 50% case, the EHP table, the 48% case, the 1-ulp pipeline gap, and the underflow of an unclamped percent product.

### 16.2 Critic

1. **The "never 100%" proof is about real numbers.** The first draft of a percent product in basis points or in Q32.32 reaches 0. Shipping the algebra without the floor would create the immunity the constraint forbids. This is the strongest defect in the WoW-shaped design.
2. **EHP linearity is false past the cap and false by a unit when K does not divide.** A reader who implements `HP * (1 + A/K)` in `f32` will not match the integer function and will fail H1.
3. **Four existing formulas** means the "one resolver" claim is false until M5. Until then the engine has five, and an example can still call `apply_damage`.
4. **Aravath block has no arc and uses `f32` 0.2.** A front-arc default would change the greybox. A claim of bit-matching 2.4 HP is false because `12 * 0.2` in `f32` is not the Q32.32 trunc.
5. **`SimRng` and `Fixed64` are not on main.** The design cites other branches as if they were the engine. An implementer on `main` cannot `use` them.
6. **WoW order is not evidence.** Forums disagree about armor vs versatility vs absorb. Locking an order is correct; citing WoW as the reason is not.
7. **Default K = 100** will be mistaken for a verified WoW constant. It is a scale choice.
8. **Adding EHP to the snapshot** is the obvious "integration" and it breaks a hash the roadmap just defined.
9. **PoE's curve might be the better gameplay** for an action game where a heavy swing should hurt through a shield. Rejecting it for AI elegance could be the wrong product call. The design should say the product call is open, not pretend the math forced it.
10. **Eight modifiers** will be too few the first time a boss has named cooldowns. A hard ceiling that content cannot meet will spawn a second, uncapped multiplier in the boss script. That is the dual-pipeline failure again.

### 16.3 Resolution

| Finding | Resolution |
|---|---|
| 1 | §7.3 floor is computed from the pre-product value, not from the truncated product. The 1-raw backstop covers sub-floor underflow. Test listed. |
| 2 | §7.2 states the exact division, the 1-unit slack, and the cap kink. Implementation is forbidden to use `f32`. |
| 3 | Accepted until M5. M4 is limited to the demo. Residual: examples still show the old formula. Not called "done" at M1. |
| 4 | Greybox block is omnidirectional and 8000 bp, flat 0. Migration target is whole-unit sword 25 and a 20% keep on the swipe, not an `f32` bit match. |
| 5 | M0/M1 block on the API. R2 and R3. No vendored divergent `Fixed64`. |
| 6 | §5.1 labels WoW stage order as forum lore. §7.4 is the contract. |
| 7 | §9 and R5 call K=100 a content default. |
| 8 | §11 forbids the snapshot edit. R4. |
| 9 | Recorded as an open product choice in §11 of the parked questions below, not as a math necessity. v1 still ships the hyperbola because H11 asks for one EHP number. A hit-size mode is a new school flag in a later schema, default off. |
| 10 | The 9th modifier is a load error (E16). Content pre-bakes a stack into one id. Raising the ceiling requires a new proof. A boss script that multiplies again is a bug, not an extension point. |

**Parked product question (not a silent default):** a later schema may add `curve: hyperbola | hit_scaled` per school for a PoE-style physical variant. v1 has only `hyperbola`. AI EHP for a `hit_scaled` school would have to take the expected hit as an argument. That change is out of scope until someone accepts the AI cost.

---

## 17. Dual review, second pass

This pass tries to break determinism, overflow, precision, exploits, and moddability. It assumes §16's resolutions are in the text and looks for what those resolutions do not cover.

### 17.1 Attacks

**Determinism**

- D1. `hit_rng` mixes ordinal and attack id, then `SimRng::new` mixes again with world seed and tick. A future "cleanup" that removes one mix changes every roll. The double mix is easy to "simplify" incorrectly.
- D2. `StdRng` (ChaCha12 via `rand` 0.9) is deterministic only while the crate version and the feature set stay pinned. Workspace bumps have happened (wgpu, rand).
- D3. Entity ids come from a generational allocator. If spawn order differs, ids differ, sort order differs, ordinals differ, crits differ. The resolver cannot repair an unstable spawn order upstream.
- D4. The four draws happen even on a gate. A fifth secret draw inside `always_crit` would desync a replay that branches on the flag.
- D5. `block_omnidirectional` defaults true, but a caller that sets `in_arc` from a glam dot imports libm into the only bit the resolver cannot see. The combat golden test stays green while the game diverges.
- D6. Applying the batch mutates HP, so `resolve_hit` is pure and the **system** is not. A second system that also applies `CVitals` in `presentation` doubles damage on one machine if the schedule differs.

**Overflow and precision**

- O1. `u32` roll times `(span+1)` overflows `u64` when done naively (`2^32 * 2^32`). The spec says `u128`. An implementer using `as u64` wraps the endpoint test and fails the golden variance check only for large spans.
- O2. `(D_raw as i128) * (K_raw as i128)` is safe for legal `i64` inputs because `i128` holds the product. A `saturating_mul` on `i64` before the widening cast would clamp early and break the 50% identity for large K.
- O3. The load-time max-chain check uses the ceilings. A test-only `CombatTables` built in Rust without `load()` skips it. The `Saturated` clamp must live in `resolve_hit`, not only in the loader.
- O4. `HP * (A+K) / K` for EHP can overflow `i64` when all three are near the top of the legal range. EHP must be `i128` then clamped, and `effective_health` must document the clamp. Time-to-kill then uses the clamped value.
- O5. Truncation bias always favors the defender on the damage path (toward zero). Over a long fight this is a systematic chip. It is deterministic, so it is not a desync, but it is a balance skew the ±5% variance does not cancel.

**Exploits and balance**

- X1. Save/load on a tick boundary replays the same crit. Combined with an editor that can rewind one tick, every attack crits. Accepted as R-adjacent to E7, but the editor rewind is a separate surface (`tools/aw_editor` play mode).
- X2. `always_crit` plus max lethality plus eight +200% attacker mods is a one-shot. Ceilings bound it; they do not make it small. A greybox ability with `always_crit` set by a mod is legal.
- X3. Bleed bypasses armor. A mod that tags every physical ability `Bleed` deletes armor from the game without tripping the armor-cap ceiling.
- X4. Poison uses Toxin, but a periodic Poison that is also `Bleed` skips the hyperbola. Double-tagging is an exploit.
- X5. The 5% floor makes many small hits better than one big hit once the attacker is past the floor (each hit pays the floor, so splitting does not reduce damage below 5% each). That inverts the usual "many small hits lose to armor rounding" pattern and buffs fast weapons. With the floor at 5% of a **large** hit this matters; with the floor at 5% of a small hit, fast weapons are taxed less by truncation and more by the floor. Needs a numeric note, not a shrug.
- X6. Killing-blow priority to the lower entity id is farmable if the player can control spawn ids. Unlikely in the demo, real if mods spawn allies in a loop.
- X7. Poise refill on stagger plus a 1-tick stagger is a stun lock if poise damage is 1 and max poise is 1. Content, but the resolver will happily do it. A minimum stagger of 1 tick is already implied; a minimum of several ticks is not specified.

**Moddability**

- M1. Semantic hash ignores whitespace, good. It must also ignore RON key order. If the hash walks struct-field declaration order, it is stable; if it walks parser-encounter order, two identical mods hash differently.
- M2. A mod that replaces school id 1's name but not its id silently retargets every ability. Validation should require the name to stay bound to the id across patches, or abilities should store the id only and names are comments.
- M3. Curve keypoints as a long list make `resolve` branchy. Caps of 8 modifiers do not cap curve length. A 100_000-key curve is a load-time stall and a tick stall if evaluated naively per hit.
- M4. The engine ceiling is in code, so a "mod" that is actually a fork of `astraweave-combat` removes it. The ceiling is a trust boundary against data, not against a rebuilt engine. Say that.
- M5. `schema: 1` is in the file and not in the hash story. A schema 2 loader that accepts schema 1 must hash the interpreted v1 struct, or old saves look like mismatches.

### 17.2 Resolution

| Attack | Resolution |
|---|---|
| D1 | `hit_rng` in §10 is the only mix recipe. `SimRng::new` is the inner mix. Tests lock `hit_rng(1, 2, 3, 4).next_u32()` once the API exists. A second mix in the caller is a bug. |
| D2 | E6 stands. Workspace `rand` pin is an M1 acceptance check, not a hope. |
| D3 | Accepted limitation. Combat sorts the ids it is given. Unstable spawn is a spawning-system defect (roadmap already requires stable entity order for perception). Recorded under R6's family: upstream order is out of scope, and the contract is "same ids in ⇒ same damage out". |
| D4 | `always_crit` does not draw a fifth word. Draw count is `tables.draw_count`, checked by the gate test. |
| D5 | Stated again: golden `combat_resolve` excludes the bool's producer. Full-frame equality waits on WS-D3. R1, R6. |
| D6 | Only `combat_resolve_system` writes `CVitals.hp`. A debug assertion counts applies per tick per entity and trips on a second writer. M2 test. |
| O1 | Variance formula in §7.6 uses `u128`. The endpoint test is the regression. |
| O2 | Spec text: widen **before** multiply. `saturating_mul` on `i64` is the wrong helper. Code review checklist item for M1. |
| O3 | `Saturated` is inside `resolve_hit` (§7.4 step 12), including tables built in tests. |
| O4 | `effective_health` computes in `i128` and clamps to `i64::MAX`, flagging `Saturated` on the query result. Added to the M1 API contract. |
| O5 | Accepted skew. Truncation toward zero is the FFXIV-shaped choice and it is stable. The 1 ulp on the canonical hit is the published size of the bias for that input. Not "fixed" by rounding half away from zero, because a tie-break that inspects a sign bit is easier to get wrong across the heal path, and the bias is far below the variance band when variance is on. Greybox variance is off, so the bias is visible only as 1 ulp, which does not change `floor(raw/SCALE)` for the canonical hit (7). |
| X1 | Editor rewind must restore `tick` with the world. If it restores HP and not tick, that is an editor bug. M4 notes it. Not solved in the resolver. |
| X2 | Legal and loud: `always_crit` is an ability flag, so it shows up in the table hash. A mod that sets it changes the save hash and fails closed on old saves. |
| X3 | `Bleed` on a non-periodic ability is a validation error. Bleed implies Periodic. A mod cannot mark `sword_light` as Bleed without setting Periodic, and the greybox sword does not set either. |
| X4 | Periodic Poison uses Toxin **unless** Bleed is set, and Bleed is only legal for Physical. Poison+Bleed fails validation. One bypass path, not two. |
| X5 | Documented numeric: the floor is 5% of **this hit's** post-school value, so splitting one hit into two does not create extra minimum damage beyond two truncations. Each fragment's floor is 5% of that fragment, and the sum of floors equals 5% of the sum (when truncation agrees). The floor does not grant a per-hit minimum of 5% of some other reference hit. Fast weapons are not paid a bonus by the floor. They are paid a bonus by truncation only at sub-unit fragments, which the 1-raw rule bounds by 1 ulp per hit. |
| X6 | Accepted. Kill credit follows the sort key. Mods that need shared credit read the full `DamageDealt` log, not only the death event. |
| X7 | Minimum `stagger_ticks` on the outcome is a data field, default **12** (0.2 s at 60 Hz), not 1. Poise damage still comes from the ability. A 1-poise enemy still staggers for 12 ticks. |
| M1 | Hash walks the struct in field order after validation, ids sorted, keypoints sorted by x. Parser order is not observed. M1 test: two RON texts with swapped keys produce one hash. |
| M2 | Abilities store school **id** only. A patch that changes the name of an id fails validation. Names are checked equal to the base row. |
| M3 | Curve keypoints capped at **64** per curve. Evaluation is a binary search on a sorted slice, no allocation. |
| M4 | Ceilings are a data trust boundary. A rebuilt engine is a different program and is out of scope for the mod format. Stated here so nobody treats the RON file as a sandbox against native code. |
| M5 | `schema` is hashed. A v2 reader that accepts v1 hashes the v1 interpretation and stores that. Unknown schema refuses the load. |

No attack in this pass remains without a resolution or a residual-risk id. R1–R6 are unchanged and still require a human.

---

## 18. Gate decision card

```yaml
gate:
  name: "Combat damage system design — approve before implementation"
  tier: r3
  status: Pass
  rationale: >
    The user (human authority) and the orchestrator both reviewed this
    document and approved it on 2026-10-14. The design locks a single Q32.32
    resolver with tested integer identities (Armor == K is half, EHP linear
    below a continuous 75% cap, 40%+20% is 48% taken, cooldown percents cannot
    reach zero because of an independent floor). Residual risks R1–R6 stay
    open and are accepted for implementation: SimRng and Fixed64 are local
    seams until their branches land, and the three-OS replay job is still absent.
    A later lore steer (same day) keeps schools data-driven, changes only the
    shipped default labels, masks AI queries, and adds an unused thread-strain hook.
  residual_risks:
    - "R1 cross-OS replay workflow not on main"
    - "R2 SimRng only on cursor/engine-ws-d2-stab; local seam until PR 211"
    - "R3 Fixed64 only on cursor/aravath-greybox-b3de; local seam"
    - "R4 WorldSnapshot EHP deferred so planner hashes stay put"
    - "R5 default K and caps are content, not a balance pass"
    - "R6 physics hit detection remains outside the resolver"
  next_action: proceed
  reviewer: both
  date: 2026-10-14
```

---

## 19. Reproducibility note

Another agent can re-check the integer claims without the game:

1. Check out `main` at `a3fe3ecd`, `origin/cursor/engine-roadmap` at `de5832b5`, `origin/cursor/engine-ws-d2-stab` at `05c2bff0`, and `origin/cursor/aravath-greybox-b3de` at `c86690ab`.
2. Re-read the files named in §3. `SimRng` and `Fixed64` are not on `main`.
3. Recompute §7 with integer arithmetic (`SCALE = 2**32`, division truncating toward zero). The seven stage raws and the digest `071dc70e116987782a8a8b137a6cce648d390a82967726775308946f1e69da71` are SHA-256 over the little-endian `i64`s in order S0..S6.
4. The 12×10% underflow (1000 HP → raw 4) is the witness that the algebraic product proof is not an implementation.

No GPU, no WoW client, and no three-OS runner were used. That is why H1 is not marked satisfied.

---

## 20. Revision

| Version | Date | Note |
|---|---|---|
| 0.1 | 2026-10-14 | Design only. r3. Gate Needs-human. No engine code. |
| 0.2 | 2026-10-14 | Human and orchestrator sign-off. Gate Pass. Lore steer recorded in §21. Implementation started. |

---

## 21. Lore steer (implementation, same day)

The school pipeline stays generic: a mod may add any school id. The shipped default table does not present elemental magic.

- Physical attacks and the Bleed flag stay.
- Fire, frost, and poison ship as mundane `hazard` schools, not spell schools.
- Shock is absent from the default table. A mod can add a school; the engine does not name one.
- The non-physical causal school is named `weave`. The labels `Veil` and `Echo` are not in the default table.
- A sword-and-shield combatant with only Might, Armor, and Bulwark can deal and receive damage. No other school rating is required.
- `resolve_hit` is the world. It uses the true components. `expected_hit` and `effective_health` are the AI queries. They take a `StatKnowledge` mask and do not read a stat the mask hides. Hidden armor cannot change the estimate.
- `ThreadStrain` { capacity, reserve, coherence, strain } is stored and saved. The damage formula does not read it.

These are deliberate deviations from the hardcoded Focus / Heat / Rime / Volt / Toxin fields in §8. Resistances and non-physical ratings are small id-keyed arrays so a new school does not need a new component field.
