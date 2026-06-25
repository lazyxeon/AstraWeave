---
schema_version: 1
trace_id: weaving
title: "Weaving / Fate-Weaving System (`astraweave-weaving`)"
description: "Weaving / Fate-Weaving — the Veilweaver mechanic (`astraweave-weaving`)"
primary_crate: astraweave-weaving
domain: gameplay
lifecycle_status: in_design
integration_status: example_only
summary: "Zero library consumers (only examples/advanced_content_demo + veilweaver_quest_demo); systems/* take Vec/slice params, not Query/Res — not registerable as ECS systems as written. weaving.md §1,§8"
owns: [astraweave-weaving]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Weaving / Fate-Weaving System (`astraweave-weaving`)

## Metadata

| Field | Value |
|---|---|
| **System name** | Weaving / Fate-Weaving System (the Veilweaver signature mechanic) |
| **Primary crates** | `astraweave-weaving` |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | In-Design-but-tested (heavily unit-tested; consumed only by two non-runtime examples) |
| **Owner notes** | Built during the Veilweaver vertical-slice "Weeks 1-5" campaign (see `docs/journey/weeks/WEEK_*` and `docs/archive/projects/veilweaver/`). Naming is heavily overloaded across three independent "weaving" surfaces in the workspace — see §3 and §6. |

---

## 1. Executive Summary

**What this system does:**
`astraweave-weaving` is a single Rust crate that bundles two loosely-coupled feature families under one name: (a) an **emergent-behavior layer** (pattern detection → intent proposal → budget/cooldown adjudication, the crate's original purpose per its `Cargo.toml` description), and (b) a **Veilweaver gameplay slice** — the player-facing "fate-weaving" loop of stabilizing reality **anchors**, spending **Echo** currency, unlocking **abilities** (Echo Dash / Echo Shield), fighting enemies, and completing quests.

**Why it exists:**
It provides the deterministic, pure-logic core for the Veilweaver "signature mechanic" demo (repair anchors to stabilize reality; earn/spend Echoes; unlock abilities) plus a reactive director-style emergent-event proposer, designed as testable pure functions with no global state.

**Where it primarily lives:**
- `astraweave-weaving/src/` — the entire system. There is no second crate.
- Emergent layer: [`patterns.rs`](../../astraweave-weaving/src/patterns.rs), [`intents.rs`](../../astraweave-weaving/src/intents.rs), [`adjudicator.rs`](../../astraweave-weaving/src/adjudicator.rs).
- Anchor/Echo gameplay: [`anchor.rs`](../../astraweave-weaving/src/anchor.rs), [`echo_currency.rs`](../../astraweave-weaving/src/echo_currency.rs), [`abilities.rs`](../../astraweave-weaving/src/abilities.rs), [`systems/`](../../astraweave-weaving/src/systems/), [`level.rs`](../../astraweave-weaving/src/level.rs).
- Quest/enemy/spawn: [`quest.rs`](../../astraweave-weaving/src/quest.rs), [`quest_types.rs`](../../astraweave-weaving/src/quest_types.rs), [`starter_quests.rs`](../../astraweave-weaving/src/starter_quests.rs), [`enemy.rs`](../../astraweave-weaving/src/enemy.rs), [`enemy_types.rs`](../../astraweave-weaving/src/enemy_types.rs), [`spawner.rs`](../../astraweave-weaving/src/spawner.rs), [`combat.rs`](../../astraweave-weaving/src/combat.rs).
- Presentation data structs: [`ui/`](../../astraweave-weaving/src/ui/), [`particles/`](../../astraweave-weaving/src/particles/), [`audio/`](../../astraweave-weaving/src/audio/).

**Status note (read this first):**
This crate is **tested-but-largely-unwired** in the Key-Lesson-8 sense. `cargo metadata` shows **zero library consumers**; the only workspace crates that depend on it are two examples — `examples/advanced_content_demo` and `examples/veilweaver_quest_demo` (verified: `rg 'astraweave-weaving' **/Cargo.toml`). It depends on only one AstraWeave crate (`astraweave-pcg`) and **does not depend on `astraweave-ecs`, `astraweave-core`, `astraweave-render`, `astraweave-audio`, or `astraweave-gameplay`**. The functions in `systems/` are documented as "ECS system functions" but their signatures take plain `Vec`/slices, **not** ECS `Query`/`Res`/`EventReader` params, so they cannot be registered via `App::add_system` as written (see §6). The `ui/` modules' egui `render()` methods are compiled out via `#[cfg(any())]` because egui is not a dependency. Treat this crate as a deterministic logic library and demo backbone, not a runtime-wired subsystem.

---

## 2. Authoritative Pipeline

This crate hosts **two distinct pipelines** that do not call each other. Both are real and tested.

### Pipeline A — Emergent behavior (the crate's `Cargo.toml`-stated purpose)

```text
[WorldMetrics] (caller-supplied aggregate world state)
    │  file: patterns.rs (WorldMetrics struct, lines 52-67)
    │  detector.detect(&metrics)
    ▼
[Stage A1: Pattern detection]
    file: patterns.rs
    role: each PatternDetector returns Vec<(pattern_id: String, strength: f32)>
    key data: e.g. ("low_health_cluster", 0.5), ("resource_scarce_food", 0.8)
    │  patterns collected into BTreeMap<String, f32> by the caller
    │  proposer.propose(&patterns, seed)
    ▼
[Stage A2: Intent proposal]
    file: intents.rs
    role: each IntentProposer maps patterns → Vec<WeaveIntent>
    key data: WeaveIntent { kind, priority, cost, cooldown_key, payload }
    │  adjudicator.begin_tick(); adjudicator.adjudicate(intents)
    ▼
[Stage A3: Adjudication]
    file: adjudicator.rs
    role: WeaveAdjudicator filters by min_priority, sorts (priority desc, cost asc,
          kind asc), enforces per-tick budget + per-key cooldowns
    key data: approved Vec<WeaveIntent>
    ▼
[Approved intents]  (the caller is responsible for executing them — no executor in-crate)
```

### Pipeline B — Veilweaver gameplay loop (anchors / Echo / abilities / quests)

```text
[Player action: approach + repair an Anchor]  (driven by example / VeilweaverLevel)
    │
    │  systems::anchor_repair_system(requests, anchors, &mut echo_currency)
    ▼
[Stage B1: Repair adjudication]
    file: systems/anchor_repair_system.rs
    role: checks Anchor.stability() < 1.0 and EchoCurrency.has(repair_cost);
          spends Echoes (logs Transaction), applies Anchor::repair() (+0.3 stability),
          reports ability unlock if anchor was below 0.7 pre-repair
    key data: Vec<RepairEvent { anchor_id, RepairResult, ability_unlocked }>
    │
    │  Anchor::repair() bumps stability; AnchorVfxState recomputed from stability
    ▼
[Stage B2: Anchor state / VFX]
    file: anchor.rs
    role: stability (0..1) → AnchorVfxState (Perfect/Stable/Unstable/Critical/Broken);
          drives glow_color / hum_frequency / particle_emission_rate accessors
    │
    │  per-frame: systems::anchor_decay_system(anchors, dt, &combat_events)
    ▼
[Stage B3: Decay]
    file: systems/anchor_decay_system.rs
    role: applies passive decay (-0.01/60 per sec) to all anchors and combat stress
          (-0.05 per EnemyKilled event) — NOTE: proximity gate is stubbed (§6)
    │
    │  Echo economy in parallel:
    │  systems::echo_pickup_system(combat_rewards, pickups, &mut currency)
    ▼
[Stage B4: Echo economy + ability gating]
    files: echo_currency.rs, abilities.rs
    role: EchoCurrency tracks count + transaction log; AbilityManager (EchoDash,
          EchoShield) gates use on cooldown + can_afford(echo); abilities return
          (target_pos, damage) / damage-reduction — they do NOT mutate currency
          themselves (the Player wrapper in level.rs deducts the cost)
    │
    │  quest progress fed by repair/kill/explore callbacks
    ▼
[Stage B5: Quest progression]
    files: quest.rs, quest_types.rs, starter_quests.rs
    role: QuestManager tracks ObjectiveType progress, completes quests, emits
          QuestReward (EchoCurrency / AbilityUnlock / StatBoost), activates next
    ▼
[Presentation data (NOT rendered in-crate)]
    files: ui/*, particles/*, audio/*
    role: plain data structs (HUD state, feedback floats, particle pools, audio
          command enums) intended for an external renderer/audio backend to consume
```

### Stage-by-stage detail

#### Stage A1: Pattern detection
**File:** [`patterns.rs`](../../astraweave-weaving/src/patterns.rs)
**Role:** The `PatternDetector` trait (`patterns.rs:43-50`) takes a `&WorldMetrics` (an aggregate, not the live world) and returns `Vec<(String, f32)>`. Four concrete detectors exist: `LowHealthClusterDetector`, `ResourceScarcityDetector`, `FactionConflictDetector`, `CombatIntensityDetector` (`patterns.rs:69-161`). `PatternStrength` (`patterns.rs:14-40`) buckets a strength into Weak (<0.3) / Moderate (<0.7) / Strong.
**Notes:** Detectors are pure functions over the metrics struct; the README emphasizes "use aggregated metrics instead of scanning all entities."

#### Stage A2: Intent proposal
**File:** [`intents.rs`](../../astraweave-weaving/src/intents.rs)
**Role:** `WeaveIntent` (`intents.rs:6-50`) is a builder-style struct (`kind`, `priority`, `cost`, `cooldown_key`, `payload`). The `IntentProposer` trait (`intents.rs:52-59`) maps a `&BTreeMap<String,f32>` of pattern strengths + a `seed` to `Vec<WeaveIntent>`. Four proposers: `AidEventProposer`, `SupplyDropProposer`, `MediatorProposer`, `ScavengerPatrolProposer`. `ScavengerPatrolProposer` uses `seed % 2` for deterministic variation (`intents.rs:159-163`).
**Notes:** The crate-level `WeaveIntent` (this file) is distinct from the gameplay-crate `WeaveOp` (see §6). `WeaveIntent`s here are never converted to `WeaveOp`s anywhere in the workspace.

#### Stage A3: Adjudication
**File:** [`adjudicator.rs`](../../astraweave-weaving/src/adjudicator.rs)
**Role:** `WeaveAdjudicator` (`adjudicator.rs:48-165`) holds a `WeaveConfig` (budget_per_tick=20, min_priority=0.3, per-key cooldowns in ticks). `begin_tick()` resets budget and decrements/expires cooldowns; `adjudicate()` filters by `min_priority`, sorts by `(priority desc, cost asc, kind asc)` for deterministic tie-breaking, then greedily approves within budget while honoring cooldowns. `WeaveConfig` round-trips via TOML (`from_toml`/`to_toml`, `adjudicator.rs:36-46`).
**Notes:** Cooldowns are measured in **ticks** (default 300 = "5 seconds at 60Hz" per the comment). Default unknown-key cooldown is 300 (`adjudicator.rs:138`).

#### Stage B1: Anchor repair
**File:** [`systems/anchor_repair_system.rs`](../../astraweave-weaving/src/systems/anchor_repair_system.rs)
**Role:** `anchor_repair_system(repair_requests, anchors: &mut [(usize, &mut Anchor)], echo_currency)` returns `Vec<RepairEvent>` with `RepairResult` ∈ {Success, InsufficientEchoes, AlreadyMaxStability}. Ability unlock is reported only if the anchor's pre-repair stability was `< 0.7` (`anchor_repair_system.rs:91-99`).
**Notes:** This is one of two repair code paths — `VeilweaverLevel::repair_anchor` (`level.rs:325-354`) is a parallel, simpler path that uses a `0.8` threshold and `i32` echo currency, not this system. See §6.

#### Stage B2: Anchor state / VFX
**File:** [`anchor.rs`](../../astraweave-weaving/src/anchor.rs)
**Role:** `Anchor` (`anchor.rs:39-247`) owns a private `stability: f32`, a `decay_rate`, `repair_cost`, an `Option<AbilityType>` unlock, a proximity radius, and a cached `AnchorVfxState`. Constants: `DEFAULT_DECAY_RATE = -0.01/60`, `COMBAT_STRESS_DECAY = -0.05`, `REPAIR_BONUS = +0.3`, `DEFAULT_PROXIMITY = 3.0`, `REPAIR_ANIMATION_DURATION = 5.0`. `AnchorVfxState::from_stability` (`anchor.rs:277-289`) maps `>=1.0 Perfect / >=0.7 Stable / >=0.4 Unstable / >=0.1 Critical / else Broken`.
**Notes:** `vfx_state` is `#[serde(skip)]` and recomputed on every mutation via `update_vfx_state()`. `Anchor::repair()` adds `+0.3` capped at 1.0 and returns `false` if already at max.

#### Stage B3: Decay
**File:** [`systems/anchor_decay_system.rs`](../../astraweave-weaving/src/systems/anchor_decay_system.rs)
**Role:** Applies passive decay to all anchors and applies combat stress for each `CombatEvent` of type `EnemyKilled`.
**Notes:** `apply_combat_stress_to_nearby_anchors` (`anchor_decay_system.rs:58-71`) **does not actually check distance** — the `_STRESS_RADIUS`/`_event_pos` are unused and the comment states "stubbed distance check for now ... Real implementation would check distance to Position component." Every anchor in the slice receives stress on any `EnemyKilled` event.

#### Stage B4: Echo economy + abilities
**Files:** [`echo_currency.rs`](../../astraweave-weaving/src/echo_currency.rs), [`abilities.rs`](../../astraweave-weaving/src/abilities.rs)
**Role:** `EchoCurrency` (`echo_currency.rs:42-165`) is a `u32` count plus a FIFO-trimmed `Vec<Transaction>` (max 100). `spend()` returns `false` on insufficient balance (no mutation). `TransactionReason` (`echo_currency.rs:228-257`) is a `#[non_exhaustive]` enum keyed to slice content (KillRiftStalker, KillSentinel, FoundShard, RepairAnchor(id), UseEchoDash, DeployBarricade, QuestReward(id), …). `AbilityManager` (`abilities.rs:184-288`) holds `EchoDash` (1s cd, instant, 10 echo, 30 dmg, 10m dash) and `EchoShield` (5s cd, 3s duration, 15 echo, 50% reduction); activation methods return `Result<_, String>` and check `is_ready() && can_afford()`.
**Notes:** Abilities **do not** decrement `EchoCurrency`. The cost is deducted by the caller — in `Player::use_dash`/`use_shield` (`level.rs:111-133`) via a hard-coded `-= 10` / `-= 15`. There is no in-crate enforcement that `AbilityState.echo_cost` equals the number the Player deducts.

#### Stage B5: Quest progression
**Files:** [`quest.rs`](../../astraweave-weaving/src/quest.rs), [`quest_types.rs`](../../astraweave-weaving/src/quest_types.rs), [`starter_quests.rs`](../../astraweave-weaving/src/starter_quests.rs)
**Role:** `QuestState` (Inactive/Active/Completed/Failed), `ObjectiveType` (`quest.rs:24-70`: Kill/Repair/Fetch/Explore plus the richer types in `quest_types.rs` — Escort/Defend/TimeTrial/Boss/Collect). `QuestManager` (in `quest.rs`) registers/activates quests, tracks progress via `update_repair`/`update_kill`/`update_explore`, and yields `QuestReward`s. `starter_quests.rs` defines three onboarding quests (`stabilize_anchors`, `clear_corruption`, `restore_beacon`).
**Notes:** `QuestReward::EchoCurrency(u32)`, `AbilityUnlock(String)`, `StatBoost { stat, amount }`. Reward distribution is done by the consumer (`VeilweaverLevel::apply_reward`).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Weave / Weaving** | In *this* crate's emergent layer: the act of detecting a world pattern and proposing a reactive intent. NOT the player's reality-editing power. | `patterns.rs`, `intents.rs`, `adjudicator.rs`, `lib.rs` (`CWeaveAgent`, `CWeaveSignal`, `WeaveIntentEvent`) |
| **Fate-weaving** | The lore-level player magic (speak/declare reality into being). In code, the *closest* representation is the anchor-repair + ability loop; the term itself is lore (`docs/Veilweaver/lore_bible.md`), not a code symbol in this crate. | lore docs; conceptually the anchor/ability loop |
| **WeaveIntent** | A proposed emergent event (`{kind, priority, cost, cooldown_key, payload}`). | `intents.rs`, `adjudicator.rs` |
| **Anchor** | A "loom node": a stabilizable point with `stability ∈ [0,1]`, a repair cost, and an optional ability unlock. The core fate-weaving interactable. | `anchor.rs`, `systems/anchor_*` |
| **AnchorVfxState** | Discrete presentation tier derived from stability (Perfect/Stable/Unstable/Critical/Broken), driving glow/hum/particle accessors. | `anchor.rs` |
| **Echo / Echoes** | The gameplay currency (`u32`). Earned from kills/shards/quests, spent on repairs/abilities. | `echo_currency.rs`, `abilities.rs`, `systems/echo_*` |
| **Transaction** | A logged Echo gain/spend with a `TransactionReason`. | `echo_currency.rs` |
| **Ability** | A player power gated by cooldown + Echo cost: `EchoDash`, `EchoShield`. | `abilities.rs` |
| **Adjudicator** | The budget/cooldown gatekeeper for emergent intents. | `adjudicator.rs` |
| **Pattern / PatternDetector** | A detected world condition + strength, produced by a detector over `WorldMetrics`. | `patterns.rs` |

### Terms to NOT confuse

- **`WeaveIntent` (this crate) vs `WeaveOp` (`astraweave-gameplay`):** Two unrelated "weave action" types. `WeaveIntent` is an emergent-event proposal adjudicated by budget; `WeaveOp { kind, a, b, budget_cost }` ([`astraweave-gameplay/src/types.rs:42-56`](../../astraweave-gameplay/src/types.rs)) is the **director-plan terrain/water-edit** primitive applied by `apply_weave_op` ([`astraweave-gameplay/src/weaving.rs`](../../astraweave-gameplay/src/weaving.rs)). They never interconvert.
- **`AbilityType` (this crate) — two definitions:** `anchor::AbilityType` (`anchor.rs:340`, variants `EchoDash`, `BarricadeDeploy`; re-exported as the crate's canonical `AbilityType`) and `abilities::AbilityType` (`abilities.rs:9`, variants `EchoDash`, `EchoShield`). They share a name and one variant spelling but are different enums. See §6.
- **`CombatEvent` — two definitions:** `combat::CombatEvent` (rich enum, `combat.rs:31`, re-exported) and `systems::anchor_decay_system::CombatEvent` (struct `{position, event_type}`, re-exported from `systems::mod`). See §6.
- **"Weaving" (this crate) vs the *water* weave-response:** The W-series water campaign's "part/freeze/raise" weave-deformation vocabulary and `WeaveInstance`/`FreezeWater` live in `astraweave-render/src/water.rs` and `astraweave-gameplay`, NOT here. See §4.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-pcg` | Cargo dep (`Cargo.toml:23`) | None imported — declared-but-unused | The only AstraWeave crate dependency, but it is a **declared-but-unused Cargo dep**: zero `use`/`astraweave_pcg`/`pcg` references in `astraweave-weaving/src` (verified `git grep -n 'pcg\|Pcg\|PCG'` → no matches in src). Spawn/enemy randomness uses `rand::rng()` (thread RNG), e.g. `spawner.rs:349,365`, `enemy.rs:259`, which is **non-deterministic and not seed-driven**. (Corrects the prior claim that pcg is used in `enemy.rs`/`spawner.rs` for deterministic generation.) |
| Caller (example/host) | `WorldMetrics` struct | aggregate world state | Pattern detectors consume this; the crate does not build it from any live world. |
| Caller (example/host) | `CombatEvent[]`, `CombatRewardEvent[]`, `PickupEvent[]`, `RepairRequest[]` | event slices | All `systems/*` functions take pre-collected slices, not ECS event readers. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/advanced_content_demo` | `use astraweave_weaving::*;` (`main.rs:1`) | `Quest`, `QuestReward`, `QuestState`, `quest_types::*` | Demonstrates 5 quest scenarios (escort/defend/timetrial/boss/collect). Non-runtime example. |
| `examples/veilweaver_quest_demo` | `use astraweave_weaving::VeilweaverLevel;` (`main.rs:11`) | `VeilweaverLevel`, anchor repair, quest chain | Console walkthrough of the anchor/Echo/quest slice. Non-runtime example. |
| (intended) external renderer | `ui/*`, `particles/*` data structs | HUD state, feedback floats, particle pools | No in-crate renderer; egui `render()` is `#[cfg(any())]`-disabled in `ui/echo_hud.rs`. |
| (intended) external audio backend | `audio::AudioCommand`, `AnchorAudioSystem` | audio command enums | Plain command data; no audio backend dependency. |

### Bidirectional / Coupled

- **None at the crate boundary.** The W2C3_1 water-campaign recon (`docs/campaigns/water-successor/W2C3_1_RECON.md:23-29`) verified that "the three crates are mutually independent (render ⊥ gameplay ⊥ weaving)" and that `astraweave-weaving` has **zero dependency on `astraweave-gameplay`** — "abilities never emit `WeaveOp`s (verified confirmed)." The water-facing weave loop is glued together only inside the `examples/weaving_playground` binary, which does not depend on this crate either.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`lib.rs`](../../astraweave-weaving/src/lib.rs) | Module roots, re-exports, `CWeaveAgent`/`CWeaveSignal`/`WeaveIntentEvent` ECS-style components | Active | `#![forbid(unsafe_code)]`. The three components have no in-crate consumer (no ECS in this crate). |
| [`patterns.rs`](../../astraweave-weaving/src/patterns.rs) | Emergent pattern detection (4 detectors) | Active | Pure functions over `WorldMetrics`. |
| [`intents.rs`](../../astraweave-weaving/src/intents.rs) | `WeaveIntent` + 4 proposers | Active | Builder-style intent. |
| [`adjudicator.rs`](../../astraweave-weaving/src/adjudicator.rs) | `WeaveAdjudicator` + `WeaveConfig` (TOML) | Active | Budget/cooldown/priority gate. |
| [`anchor.rs`](../../astraweave-weaving/src/anchor.rs) | `Anchor`, `AnchorVfxState`, `AbilityType` (canonical) | Active | Core interactable; private fields with accessor API. |
| [`echo_currency.rs`](../../astraweave-weaving/src/echo_currency.rs) | `EchoCurrency`, `Transaction`, `TransactionReason` | Active | u32 balance + transaction log. |
| [`abilities.rs`](../../astraweave-weaving/src/abilities.rs) | `AbilityManager`, `EchoDash`, `EchoShield`, `AbilityState`, `AbilityType` (2nd) | Active | Cooldown/cost logic; does not mutate currency. |
| [`combat.rs`](../../astraweave-weaving/src/combat.rs) | `CombatSystem`, `CombatEvent` (enum), `Killer` | Active | Damage/death/event queue. |
| [`enemy.rs`](../../astraweave-weaving/src/enemy.rs) | `Enemy`, `EnemyBehavior`, `EnemyState`, `AttackTarget` | Active | Enemy AI state. |
| [`enemy_types.rs`](../../astraweave-weaving/src/enemy_types.rs) | Enemy archetype definitions | Active | Exposed via `pub mod enemy_types;` (`lib.rs:19`), not flat-re-exported; public types: `EnemyArchetype`, `Riftstalker`, `Sentinel`, `VoidBoss`, `VoidBossPhase`, `BossSpecialAttack` (`enemy_types.rs:9,26,118,196,205,216`). Consumed via the `enemy_types::*` path by `advanced_content_demo`. |
| [`spawner.rs`](../../astraweave-weaving/src/spawner.rs) | `EnemySpawner`, `SpawnPoint`, `SpawnRequest` | Active | Spawn-point management. |
| [`quest.rs`](../../astraweave-weaving/src/quest.rs) | `Quest`, `QuestManager`, `ObjectiveType`, `QuestReward`, `QuestState` | Active | Objective progression. |
| [`quest_types.rs`](../../astraweave-weaving/src/quest_types.rs) | Escort/Defend/TimeTrial/Boss/Collect objective payloads | Active | Used by `advanced_content_demo`. |
| [`starter_quests.rs`](../../astraweave-weaving/src/starter_quests.rs) | 3 onboarding quests | Active | `stabilize_anchors`, `clear_corruption`, `restore_beacon`. |
| [`level.rs`](../../astraweave-weaving/src/level.rs) | `VeilweaverLevel`, `Player`, `Camera`, `LevelStats` | Active | Integration glue used by `veilweaver_quest_demo`. Contains a 2nd anchor-repair path (§6). |
| [`systems/anchor_decay_system.rs`](../../astraweave-weaving/src/systems/anchor_decay_system.rs) | Passive + combat decay | Active | Distance gate stubbed. |
| [`systems/anchor_proximity_system.rs`](../../astraweave-weaving/src/systems/anchor_proximity_system.rs) | Proximity prompts | Active | Verified — `anchor_proximity_system(anchors: &[AnchorEntity], player_pos: PlayerPosition, previous_in_range: &mut Option<usize>) -> Vec<ProximityEvent>` (`anchor_proximity_system.rs:64-68`); plain slices/value, not ECS params; `add_system` doc-comment is `ignore`d. |
| [`systems/anchor_interaction_system.rs`](../../astraweave-weaving/src/systems/anchor_interaction_system.rs) | Interaction/inspection | Active | Verified — `anchor_interaction_system(in_proximity_anchor: Option<usize>, anchors: &[(usize, &Anchor)], input: &InputState) -> Option<InteractionEvent>` (`anchor_interaction_system.rs:67-71`); plain params, not ECS. |
| [`systems/anchor_repair_system.rs`](../../astraweave-weaving/src/systems/anchor_repair_system.rs) | Echo-spend repair w/ result codes | Active | Canonical repair logic (vs `level.rs`). |
| [`systems/echo_pickup_system.rs`](../../astraweave-weaving/src/systems/echo_pickup_system.rs) | Grant Echoes on kill/shard | Active | `EnemyType`/`PickupType` reward maps. |
| [`systems/echo_transaction_system.rs`](../../astraweave-weaving/src/systems/echo_transaction_system.rs) | Transaction feedback/stats | Active | Verified — `echo_transaction_system(currency: &EchoCurrency, previous_balance: &mut u32) -> Option<TransactionFeedbackEvent>` (`echo_transaction_system.rs:86-89`); plain params, not ECS. |
| [`systems/hud_echo_system.rs`](../../astraweave-weaving/src/systems/hud_echo_system.rs) | HUD state + feedback floats | Active | Verified — `hud_echo_system(currency: &EchoCurrency, hud_state: &mut EchoHudState, new_transaction_amount: Option<i32>, delta_time: f32)` (`hud_echo_system.rs:73-78`); plain params, not ECS. |
| [`ui/echo_hud.rs`](../../astraweave-weaving/src/ui/echo_hud.rs) | HUD data struct | Active (data only) | egui `render()` is `#[cfg(any())]`-disabled (egui not a dep). |
| [`ui/anchor_inspection_modal.rs`](../../astraweave-weaving/src/ui/anchor_inspection_modal.rs) | Inspection modal data | Active (data only) | Verified — egui `render()` is `#[cfg(any())]`-disabled (`anchor_inspection_modal.rs:122`, "Disabled: egui not in dependencies"), same pattern as `echo_hud.rs`/`ability_notification.rs`/`repair_progress_bar.rs`. |
| [`ui/quest_panel.rs`](../../astraweave-weaving/src/ui/quest_panel.rs) | Quest panel data | Active (data only) | Used by `VeilweaverLevel`. |
| [`ui/ability_notification.rs`](../../astraweave-weaving/src/ui/ability_notification.rs) | `AbilityUnlockNotification`/`NotificationState` | Active (data only) | Slide-in animation state. |
| [`ui/repair_progress_bar.rs`](../../astraweave-weaving/src/ui/repair_progress_bar.rs) | `RepairProgressBar` | Active (data only) | World-space UI state. |
| [`particles/anchor_particle.rs`](../../astraweave-weaving/src/particles/anchor_particle.rs) | Particle pool data | Active (data only) | `glam`+`VecDeque`; no GPU. |
| [`audio/anchor_audio.rs`](../../astraweave-weaving/src/audio/anchor_audio.rs) | `AudioCommand`/`AnchorAudioSystem` | Active (data only) | `glam`+`HashMap`; no audio backend. |
| [`integration_tests.rs`](../../astraweave-weaving/src/integration_tests.rs) | `#[cfg(test)]` cross-system tests | Active (test) | Enemy+anchor+combat+spawner. |
| [`mutation_tests.rs`](../../astraweave-weaving/src/mutation_tests.rs) | `#[cfg(test)]` mutation-resistance tests | Active (test) | |
| `tests/*.rs` | External determinism / pattern-edge / thread-manipulation / mutation tests | Active (test) | See §10. |
| `benches/*.rs` | Criterion benchmarks (`weaving_benchmarks`, `integration_benchmarks`) | Active (bench) | |

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Anchor-repair path A (result-coded, `u32` echo, `< 0.7` unlock gate) | `systems/anchor_repair_system.rs` | Active | Canonical "system" path; returns `RepairResult`. |
| Anchor-repair path B (boolean, `i32` echo on `Player`, `0.8` quest threshold) | `level.rs:325-354` (`VeilweaverLevel::repair_anchor`) | Active | Parallel path the demo actually drives. The two paths use different stability thresholds and currency representations and do not share code. |
| Echo currency as `EchoCurrency` (`u32` + transaction log) | `echo_currency.rs` | Active | The "real" economy with audit trail. |
| Echo currency as `Player.echo_currency: i32` | `level.rs:33` | Active | A second, parallel balance with no transaction log; manually mutated (`-= 10`/`-= 15`). The demo uses this one, not `EchoCurrency`. |
| `WeaveIntent` (emergent proposal) | `intents.rs` | Active | Distinct from gameplay-crate `WeaveOp`; never interconvert. |

### Naming collisions

- **`AbilityType`**: In `anchor.rs:340` it is `{EchoDash, BarricadeDeploy}` (the crate's re-exported canonical `AbilityType`, used by repair-unlock logic). In `abilities.rs:9` it is `{EchoDash, EchoShield}` (used by `AbilityState`/`AbilityManager`). Both share the `EchoDash` spelling but are separate types; only the `anchor.rs` one is re-exported from `lib.rs`. Future direction: not recorded.
- **`CombatEvent`**: In `combat.rs:31` it is a rich `enum` (`PlayerDamaged`/`EnemyDamaged`/`EnemyKilled`/`PlayerKilled`) and is the crate's re-exported `CombatEvent`. In `systems/anchor_decay_system.rs:20` it is a `struct {position, event_type: CombatEventType}` re-exported via `systems::mod`. Consumers must qualify which one they mean.
- **`AbilityState`**: Local to `abilities.rs`; the water campaign (`W2C3_1_RECON.md:25`) noted it "carries timing" and lives here, but is never wired to the gameplay-crate weave ops.
- **"Weave"/"Weaving" across crates**: three independent surfaces — `astraweave-weaving` (this), `astraweave-gameplay` `WeaveOp`/`WeaveBudget`/`WeaveConsequence`, and the render water weave-deformation (`WeaveInstance`). See §3.

### Known cognitive traps

- **Trap:** The `systems/*` functions are documented as "ECS system function" with `app.add_system(SystemStage::SIMULATION, anchor_decay_system)` examples in their doc-comments.
  **Why it's confusing:** It implies these are registered ECS systems.
  **What's actually true:** Their signatures take `Vec<&mut Anchor>` / `&[CombatEvent]` / `&mut [(usize, &mut Anchor)]` — plain Rust slices, not ECS `Query`/`Res`/`EventReader`. The crate has **no `astraweave-ecs` dependency**. The `add_system` examples are `///`/`ignore`d aspirational doc-comments, not working call sites.

- **Trap:** Anchor combat-stress decay "applies to nearby anchors."
  **Why it's confusing:** The function name is `apply_combat_stress_to_nearby_anchors` and takes an event position.
  **What's actually true:** The distance check is stubbed (`anchor_decay_system.rs:58-71`); every anchor in the passed slice is stressed on any `EnemyKilled` event regardless of position.

- **Trap:** Abilities have an `echo_cost` field, so they look self-deducting.
  **Why it's confusing:** `AbilityState::echo_cost` and `can_afford()` suggest the ability spends currency.
  **What's actually true:** Abilities only *gate* on affordability; the actual deduction is a hard-coded literal on the `Player` wrapper (`level.rs:119`,`130`) and is not cross-checked against `echo_cost`.

- **Trap:** Searching for the "fate-weaving water freeze/part/raise" mechanic here.
  **What's actually true:** That vocabulary (`FreezeWater`, weave part/freeze/raise, `WeaveInstance`) lives in `astraweave-gameplay/src/types.rs` + `weaving.rs` and `astraweave-render/src/water.rs`, wired only in `examples/weaving_playground`. This crate has none of it.

---

## 7. Decision Log

### Decision: Pure-function, no-global-state, deterministic design
- **Date:** ~Veilweaver Weeks 1-5 (2025; exact dates not recovered)
- **Status:** Accepted
- **Context:** The README "Design Goals" (`README.md:191-198`) state: emergent-not-scripted, budget-controlled, cooldown-protected, deterministic, composable, testable.
- **Decision:** Build detectors/proposers/adjudicator as pure functions over aggregate `WorldMetrics`, with explicit `seed` parameters for determinism (`README.md:124-133`).
- **Alternatives considered:** [Reasoning not recovered from available sources] (README does not record rejected alternatives).
- **Consequences:** No ECS coupling; trivially unit-testable; but the system is not self-wiring into a runtime (the caller must build metrics, collect events, and execute approved intents).

### Decision: Ticks (not seconds) for adjudicator cooldowns
- **Date:** As above
- **Status:** Accepted
- **Context:** `WeaveConfig::default` comments (`adjudicator.rs:22`) read "300 // 5 seconds at 60Hz".
- **Decision:** Cooldown durations are expressed in integer ticks, decremented once per `begin_tick()`.
- **Alternatives considered:** [Reasoning not recovered from available sources].
- **Consequences:** Determinism (no float time accumulation in adjudication) at the cost of binding cooldown semantics to a 60Hz assumption.

### Decision: `#[non_exhaustive]` on public enums
- **Date:** As above
- **Status:** Accepted
- **Context:** `PatternStrength`, `AnchorVfxState`, `AbilityType` (both), `TransactionReason`, `QuestState`, `ObjectiveType`, `CombatEvent`, `Killer`, `RepairResult`, `EnemyType`, `PickupType`, `CombatEventType` are all `#[non_exhaustive]`.
- **Decision:** Allow additive variant growth without breaking downstream match arms.
- **Alternatives considered:** [Reasoning not recovered from available sources].
- **Consequences:** Downstream matches must include a wildcard arm; forward-compatible.

### Decision: Keep `astraweave-weaving` dependency-light (only `astraweave-pcg`)
- **Status:** Accepted (observed, rationale partly recovered)
- **Context:** The W2C3_1 water recon (`W2C3_1_RECON.md:23-29`) explicitly verified "render ⊥ gameplay ⊥ weaving" mutual independence and that abilities "never emit `WeaveOp`s."
- **Decision:** This crate stays decoupled from ECS/render/gameplay; integration is the host binary's job.
- **Alternatives considered:** [Reasoning not recovered].
- **Consequences:** The crate is portable and testable but is dormant scaffolding from a runtime-wiring standpoint (zero library consumers).

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `Anchor::stability()` is always in `[0.0, 1.0]` (constructor clamps; decay floors at 0; repair caps at 1.0). | Yes | `anchor.rs:104,165,191,203`; tests in `anchor.rs` `mod tests`. |
| 2 | `AnchorVfxState` is always consistent with current stability (recomputed on every mutation). | Yes | `update_vfx_state()` called in `apply_decay`/`apply_combat_stress`/`adjust_stability`/`repair`; `test_vfx_state_transitions`. |
| 3 | `EchoCurrency::spend` never produces a negative balance (returns `false` and no-ops on insufficient funds). | Yes | `echo_currency.rs:125-133`; `test_spend_echoes_insufficient`. |
| 4 | `EchoCurrency` transaction log never exceeds `max_log_size` (FIFO trim). | Yes | `echo_currency.rs:160-164`; `test_transaction_log_trimming`. |
| 5 | Adjudication is deterministic for equal inputs (sort key `priority desc, cost asc, kind asc`). | Yes | `adjudicator.rs:107-113`; `test_deterministic_tie_breaking`; `tests/determinism_tests.rs`. |
| 6 | Intent proposal is deterministic given the same patterns + seed. | Yes | `intents.rs` seed usage; `test_scavenger_patrol_deterministic`. |
| 7 | The crate compiles without `unsafe`. | Yes | `#![forbid(unsafe_code)]` in `lib.rs:1`. |
| 8 | An anchor repair only reports an ability unlock when the anchor's pre-repair stability was `< 0.7`. | Yes | `anchor_repair_system.rs:91-99`; `test_ability_not_unlocked_if_stable`. |

---

## 9. Performance & Resource Profile

This is a pure-CPU logic crate; performance is benchmark-tracked but not a runtime hot path (no production caller).

### Hot paths
- **Pattern detection + adjudication**: README (`README.md:200-205`) notes detectors run "per-tick (60Hz)" *by design intent*, are kept lightweight via aggregated `WorldMetrics`, and adjudication is `O(n log n)` in proposed-intent count (typically `< 20`). Benchmarked in `benches/weaving_benchmarks.rs` (timings recorded in `docs/masters/MASTER_BENCHMARK_REPORT.md`).

### Cold paths
- **Quest progression, transaction logging**: event-driven, low frequency; transaction log bounded at 100 entries.

### Resource ownership
- All state is plain owned structs (`Anchor`, `EchoCurrency`, `QuestManager`, `VeilweaverLevel`); no GPU/audio resources, no global state. Determinism relies on `BTreeMap`/`BTreeSet` for stable iteration (`Cargo.toml:25-26`).

---

## 10. Testing & Validation

- **Unit tests:** Extensive in-module `#[cfg(test)]` across `patterns.rs`, `intents.rs`, `adjudicator.rs`, `anchor.rs`, `abilities.rs`, `echo_currency.rs`, and each `systems/*` file. README claims "21 unit tests" for the emergent layer (`README.md:186`); the gameplay-slice modules add many more.
- **Integration tests:** [`src/integration_tests.rs`](../../astraweave-weaving/src/integration_tests.rs) (enemy+anchor+combat+spawner cross-system) and [`src/mutation_tests.rs`](../../astraweave-weaving/src/mutation_tests.rs).
- **External test files (`tests/`):** `determinism_tests.rs`, `pattern_detection_edge_tests.rs`, `thread_manipulation_tests.rs`, `mutation_tests.rs`, `mutation_resistant_comprehensive_tests.rs`, plus `tests/common/mod.rs` helpers.
- **Mutation testing:** Covered in the workspace mutation-testing campaign (see `docs/current/MUTATION_TESTING_AUDIT.md` and `docs/journey/daily/WEAVING_TEST_SPRINT_*`). `mutation_resistant_comprehensive_tests.rs` exists in both `src/` and `tests/`.
- **Benchmarks:** `benches/weaving_benchmarks.rs`, `benches/integration_benchmarks.rs` (criterion). Results in `docs/masters/MASTER_BENCHMARK_REPORT.md`.
- **Miri:** Not applicable (no `unsafe`; `#![forbid(unsafe_code)]`).
- **Manual validation:** Console playthrough via `examples/veilweaver_quest_demo` and `examples/advanced_content_demo`.
- **Important caveat (Key Lesson 8):** High test/bench coverage does NOT imply runtime wiring. This crate has zero library consumers; its tests validate logic, not integration into a running game loop.

---

## 11. Open Questions / Parked Decisions

- **Is the `systems/*` "ECS system" framing intended to be wired?** The doc-comments target `App::add_system`, but the signatures and absence of an `astraweave-ecs` dependency mean they are not registrable today. Is a future ECS-adapter layer planned, or is the framing vestigial from a design that predated the slice-runtime split?
- **Which anchor-repair / echo-balance path is canonical?** `systems/anchor_repair_system.rs` (+`EchoCurrency`) and `VeilweaverLevel::repair_anchor` (+`Player.echo_currency: i32`) implement the same logical operation with different thresholds (0.7 vs 0.8) and currency types. The demo drives the `level.rs` path; the `systems` path is only test-driven. Should these be unified?
- **Two `AbilityType` enums** (`anchor.rs` vs `abilities.rs`) and **two `CombatEvent` types** (`combat.rs` vs `systems`) — is the overlap deliberate domain separation or accidental drift?
- **Relationship to `veilweaver_slice_runtime`:** A separate `veilweaver_slice_runtime` crate exists and references "veilweaver"/anchor/echo concepts but does **not** depend on `astraweave-weaving` (verified: no `astraweave_weaving` import). Is `astraweave-weaving` superseded by the slice runtime, or are they parallel tracks? This determines whether this crate is in-design or effectively legacy.
- **`CWeaveAgent`/`CWeaveSignal`/`WeaveIntentEvent`** (declared in `lib.rs:56-94`) have no in-crate consumer. Are these intended ECS components for a future host integration?
- **`astraweave-pcg` usage scope:** confirm exactly which PCG symbols `enemy.rs`/`spawner.rs` consume and whether spawn determinism is fully seed-driven. *(Verification note 2026-06-24: investigated and found `astraweave-pcg` is a declared-but-unused Cargo dep — zero `use`/`pcg` references in `astraweave-weaving/src`. Spawn/enemy randomness instead uses `rand::rng()` thread-RNG (`spawner.rs:349,365`, `enemy.rs:259`), which is non-deterministic and not seed-driven. The open decision — whether spawning SHOULD be made deterministic/seed-driven (and pcg actually wired or the dep removed) — remains parked.)*

---

## 12. Maintenance Notes

**Update this doc when:**
- A consumer beyond the two examples adopts `astraweave-weaving` (this would flip its wired status — update §1 and §4).
- Either of the duplicate repair/currency paths in §6 is unified, or a duplicate enum (`AbilityType`/`CombatEvent`) is reconciled.
- The `systems/*` functions gain real ECS signatures (would change §2/§6 traps and §11).
- Stability/decay/repair constants in `anchor.rs` or the adjudicator budget/cooldown defaults change (would affect §8 invariants).

**Verification process:**
- `rg 'astraweave-weaving' --type toml` to recheck the consumer set (currently only `examples/advanced_content_demo`, `examples/veilweaver_quest_demo`).
- `rg 'pub enum AbilityType|pub (enum|struct) CombatEvent' astraweave-weaving/src` to recheck the naming collisions.
- Spot-check the two pipelines in §2 against current `patterns.rs`/`intents.rs`/`adjudicator.rs` and `systems/`.
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **It is tested-but-unwired.** Zero library consumers; only two examples use it. Passing tests ≠ runtime integration (Key Lesson 8).
2. **Three different "weaving" surfaces exist** in the workspace. This crate is the anchor/Echo/quest + emergent-intent one. `WeaveOp`/`FreezeWater` (gameplay) and `WeaveInstance`/water-deformation (render) are NOT here.
3. **The `systems/*` functions are not ECS systems** despite their doc-comments; they take plain slices and there is no `astraweave-ecs` dependency.
4. **Duplicate paths coexist** (two repair paths, two echo balances, two `AbilityType`, two `CombatEvent`) — qualify which you mean before editing.

**Files you'll most likely touch:**
- `astraweave-weaving/src/anchor.rs` (core interactable)
- `astraweave-weaving/src/echo_currency.rs` (economy)
- `astraweave-weaving/src/systems/anchor_repair_system.rs` / `level.rs` (the two repair paths)
- `astraweave-weaving/src/{patterns,intents,adjudicator}.rs` (emergent layer)

**Files you should NOT touch without strong reason:**
- The `#[cfg(any())]`-gated egui `render()` in `ui/echo_hud.rs` — it is intentionally compiled out (egui not a dep); enabling it requires adding a UI dependency.

**Common mistakes when changing this system:**
- Assuming `add_system(stage, anchor_decay_system)` works — it does not; rewrite signatures to ECS params first if wiring is the goal.
- Editing one repair/currency path and assuming the demo picks it up — the demo uses `VeilweaverLevel`/`Player.echo_currency`, not `EchoCurrency`/`anchor_repair_system`.
- Conflating `anchor::AbilityType` with `abilities::AbilityType`, or `combat::CombatEvent` with `systems::CombatEvent`.

---

## Appendix B: Historical context

`astraweave-weaving` began (per its `Cargo.toml` description) as an "emergent behavior layer — pattern detection and intent generation." During the Veilweaver vertical-slice campaign (the `docs/journey/weeks/WEEK_1`…`WEEK_5` and `docs/archive/projects/veilweaver/` series, 2025), it absorbed the gameplay slice — anchors, Echo currency, abilities, quests, enemies, spawner, and presentation data structs — making it a grab-bag crate for the Veilweaver demo rather than a single cohesive subsystem. The later W-series water campaign (2026, `docs/campaigns/water-successor/`) re-examined the "weaving" name space and confirmed that the three weaving surfaces (this crate, `astraweave-gameplay` `WeaveOp`, render water `WeaveInstance`) are mutually independent. A separate `veilweaver_slice_runtime` crate now carries a runtime-oriented Veilweaver slice and does not depend on this crate, leaving `astraweave-weaving`'s runtime role unresolved (see §11).
