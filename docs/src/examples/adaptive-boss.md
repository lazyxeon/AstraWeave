# Adaptive Boss Walkthrough

The `adaptive_boss` example demonstrates AstraWeave's **Director System** - an AI-driven game master that orchestrates boss encounters with budget-constrained tactical decisions.

## Running the Example

```bash
cargo run -p adaptive_boss --release
```

The default build uses `BossDirector` from `astraweave-director`. The source also contains a `#[cfg(feature = "veilweaver_slice")]` branch for `OathboundWardenDirector`, but the example's empty `veilweaver_slice = []` feature does **not** enable `astraweave-director/veilweaver_slice`, so `cargo run -p adaptive_boss --features veilweaver_slice` does not compile today. The warden implementation lives in the director crate when that dependency feature is on (see `astraweave-director/src/veilweaver_warden.rs`).

## What It Demonstrates

- **BossDirector**: Heuristic boss planner (default run)
- **Director budget**: `apply_director_plan` spends `spawns` and `terrain_edits`; `traps` stays unchanged
- **DirectorPlan**: Tagged `ops` list applied through `apply_director_plan`
- **OathboundWardenDirector** (director crate, `veilweaver_slice` feature): Three-phase warden with telegraphs — not wired from this example's Cargo features yet

## Expected Output

Default run (BossDirector) prints JSON for `DirectorPlan` and a short apply log:

```
Director plan: {
  "ops": [
    {
      "op": "Fortify",
      "rect": {
        "x0": 7,
        "y0": 1,
        "x1": 9,
        "y1": 3
      }
    }
  ]
}
  [op0] Fortify rect=(7,1..9,3))
Remaining budget: traps=2, terrain_edits=1, spawns=2
```

When `astraweave-director` is built with `veilweaver_slice`, `OathboundWardenDirector` starts in `WardenPhase::Assessment` and may print telegraphs such as `Chains tense—the Warden studies your form.` Phase transitions use `Assessment`, `FulcrumShift`, and `DirectiveOverride` (not the older Stalking/Weaving names).

## Code Walkthrough

### 1. Arena Setup

```rust
let mut w = World::new();

// Create the combat arena: player, companion, and boss
let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
let comp = w.spawn("Comp", IVec2 { x: 3, y: 2 }, Team { id: 1 }, 80, 30);
let boss = w.spawn("Boss", IVec2 { x: 14, y: 2 }, Team { id: 2 }, 400, 0);
```

The world contains three entities:
- **Player** (HP: 100): The human-controlled character
- **Companion** (HP: 80, Ammo: 30): AI ally
- **Boss** (HP: 400): The adversary with high HP

### 2. World Snapshot

```rust
let snap = WorldSnapshot {
    t: w.t,
    player: PlayerState { hp: 100, pos: w.pos_of(player).unwrap(), ... },
    me: CompanionState { ammo: 30, morale: 0.8, pos: w.pos_of(comp).unwrap(), ... },
    enemies: vec![EnemyState { id: boss, hp: 400, cover: "high", ... }],
    pois: vec![],
    obstacles: vec![],
    objective: Some("defeat_boss".into()),
};
```

The Director receives a perception snapshot containing:
- Current game time
- Player state (health, position, stance)
- Companion state (ammo, morale, cooldowns)
- Enemy states (the boss from its perspective)

### 3. Director Budget

```rust
let mut budget = DirectorBudget {
    traps: 2,
    terrain_edits: 2,
    spawns: 2,
};
```

`DirectorOp` is `Fortify`, `SpawnWave`, and `Collapse`. `apply_director_plan` decrements `terrain_edits` by one for each applied `Fortify` or `Collapse`, and decrements `spawns` by one for each applied `SpawnWave`. It does not read `traps`. `spawns: 2` allows two `SpawnWave` ops. `BossDirector` sets `count: 3` on each wave, so one applied wave spawns three minions and still costs one `spawns`.

### 4. Director Planning (Basic)

```rust
#[cfg(not(feature = "veilweaver_slice"))]
{
    let director = BossDirector;
    let plan = director.plan(&snap, &budget);
    apply_director_plan(&mut w, &mut budget, &plan, &mut log);
}
```

The basic `BossDirector` analyzes the world state and generates a plan within budget.

### 5. Oathbound Warden (source branch)

```rust
#[cfg(feature = "veilweaver_slice")]
{
    let mut director = OathboundWardenDirector::new();
    let directive = director.step(&snap, &budget);
    
    println!("Warden phase: {:?}", directive.phase);
    if !directive.telegraphs.is_empty() {
        println!("Telegraphs: {:?}", directive.telegraphs);
    }
    
    apply_director_plan(&mut w, &mut budget, &directive.plan, &mut log);
}
```

`OathboundWardenDirector` uses three phases defined in `veilweaver_warden.rs`. `update_phase` takes one arm per call and does not skip or reverse. `initial` is `enemies[0].hp.max(1)`, captured on the first step that has an enemy.

On every step where `enemies` is not empty, `step` stores `detect_storm_choice(snap).unwrap_or(self.storm_choice)` before `update_phase`, in every phase. A match replaces the stored choice. A `None` result keeps it. The field starts as `StormChoice::Unknown`. A `storm:stabilize` objective during Assessment stores `Stabilize`, and the first FulcrumShift step uses that stored choice when the objective no longer matches. Detection returns `Option<StormChoice>`. It does not choose a telegraph, and it does not change phase.

`detect_storm_choice` returns on the first match. It checks the objective for `storm:stabilize` or `stabilize storm` (`Stabilize`), then for `storm:redirect` or `redirect storm` (`Redirect`). If the objective misses, the first POI with `k == "storm_route"` returns `Stabilize` when `pos.x >= 0` and `Redirect` when `pos.x < 0`. No match returns `None`.

- **Assessment**: Observes the fight and may fortify a choke when `terrain_edits > 0`. Telegraph: `Chains tense—the Warden studies your form.` Moves to **FulcrumShift** only when current HP is at or below two-thirds of `initial`. HP already at or below one-third still becomes **FulcrumShift** on that call, then waits for a later call to reach **DirectiveOverride**.
- **FulcrumShift**: Always appends `build_anchor_rupture` (a `Collapse`) before the stored storm choice. Stored **Stabilize** appends `Storm calms—armor plates seal the Warden.` and, when `terrain_edits > 0`, a `Fortify`. Stored **Redirect** appends `Motes surge across the arena—vision falters.` and, when `terrain_edits > 0`, a second `Collapse`. Stored **Unknown** appends `Currents buckle unpredictably.` and adds no further op. Moves to **DirectiveOverride** only from **FulcrumShift**, when current HP is at or below one-third of `initial`.
- **DirectiveOverride**: `choose_adaptive_ability` runs on the first step in this phase, while `adaptive` is still `None` (`astraweave-director/src/veilweaver_warden.rs:81-83`). Later steps skip that call because `adaptive.is_none()` is false. Nothing assigns `adaptive` back to `None`. `stance_ranged` is a local `false` inside `choose_adaptive_ability` (`astraweave-director/src/veilweaver_warden.rs:238`) and does not gate that call. The first DirectiveOverride step still reads `player.orders` (`:239-242`). An order whose lowercase text contains `bow` or `ranged` selects `AntiRangedField`. Any other order list selects `CounterShockAura` (`:243-247`). The phase arm (`:131-158`) emits ops on every step that reaches it, including those later steps. An empty `enemies` list returns before the arm (`:63-71`). Each step that reaches the arm extends `build_anchor_rupture` (one `Collapse`; `build_anchor_rupture` ignores `terrain_edits`, `:250-261`) and pushes `SpawnWave` with archetype `stormbound_wisp`, `count: 2`, and `origin` at `snap.player.pos` (`:137-141`). That push ignores `spawns`. `BossDirector`'s `count: 3` (`astraweave-director/src/lib.rs:57-59`) is a different planner. The ability used in the arm is the one stored on that same step, or the ability stored on an earlier step. Stored `AntiRangedField` pushes `Fortify` at `centered_rect(boss.pos, 2)` when `terrain_edits > 0`, and `boss` is `enemies[0]` (`astraweave-director/src/veilweaver_warden.rs:73`, `:144-149`). Stored `CounterShockAura` pushes `Collapse` from `snap.player.pos` to `boss.pos` and the push ignores `terrain_edits` (`:151-155`). The rupture and the wisp wave are emitted again on later steps. `apply_director_plan` borrows the plan (`astraweave-core/src/validation.rs:403`), so those ops stay in the plan. It logs `SKIPPED (budget)` and continues when `terrain_edits <= 0` for `Fortify` (`:409-411`) or `Collapse` (`:421-423`), and when `spawns <= 0` for `SpawnWave` (`:437-439`).

## Director System Architecture

```
┌────────────────────────────────────────────────────────┐
│                    BossDirector                        │
├────────────────────────────────────────────────────────┤
│  Input: WorldSnapshot + DirectorBudget                 │
│                                                        │
│  ┌──────────┐   ┌──────────┐   ┌──────────────────┐   │
│  │ Analyze  │ → │ Plan     │ → │ Apply within     │   │
│  │ Threat   │   │ Response │   │ Budget           │   │
│  └──────────┘   └──────────┘   └──────────────────┘   │
│                                                        │
│  Output: DirectorPlan { ops: [ { op: ... }, ... ] }    │
└────────────────────────────────────────────────────────┘
```

## Telegraph System

When the warden branch is enabled, telegraphs are plain strings on `WardenDirective`. `examples/adaptive_boss` prints one Debug line of the whole `Vec`, `Telegraphs: {:?}` (`examples/adaptive_boss/src/main.rs:54-56`). The loop below is illustrative. It prints each string on its own line, which that binary does not.

```rust
// Illustrative. The example prints `Telegraphs: {:?}` of the whole Vec.
if !directive.telegraphs.is_empty() {
    for telegraph in &directive.telegraphs {
        println!("{}", telegraph);
    }
}
```

Strings `step` pushes in `veilweaver_warden.rs`:

- `Chains tense—the Warden studies your form.` (Assessment). The binary's Assessment line is `Telegraphs: ["Chains tense—the Warden studies your form."]`.
- `Storm calms—armor plates seal the Warden.` (FulcrumShift, stored Stabilize)
- `Motes surge across the arena—vision falters.` (FulcrumShift, stored Redirect)
- `Currents buckle unpredictably.` (FulcrumShift, stored Unknown)
- `Warden adapts—activating {:?}.` (DirectiveOverride). The placeholder is the chosen ability's Debug form: `AntiRangedField` or `CounterShockAura`.

## Key Concepts

### Budget-Constrained AI

Unlike traditional boss AI that can spam abilities, the Director operates under resource constraints. This creates:
- **Strategic decisions**: Trade-offs between aggression and defense
- **Fair encounters**: Players can anticipate resource depletion
- **One-shot budget**: Nothing in `astraweave-core` or `astraweave-director` increments `DirectorBudget`. `apply_director_plan` decrements `terrain_edits` once per applied `Fortify` or `Collapse` and `spawns` once per applied `SpawnWave`. It does not read `traps`. `BossDirector` sets `count: 3` on a wave, so `spawns: 2` is two waves of three minions when those ops are applied. The remaining-budget line above is the example budget after one `Fortify`: `terrain_edits` goes from 2 to 1, and `traps` and `spawns` stay 2.

### Phase-Based Behavior (warden)

Phase changes only inside `update_phase`. Assessment becomes FulcrumShift at or below two-thirds of the HP captured on the first step that has an enemy. FulcrumShift becomes DirectiveOverride at or below one-third. A single call does not skip Assessment straight to DirectiveOverride. On every step with a non-empty `enemies` list, `detect_storm_choice` runs before `update_phase` and keeps the stored choice when it returns `None`. `Currents buckle unpredictably.` is the FulcrumShift telegraph when that stored choice is `Unknown`. Detection does not change phase.

## Related Examples

- [Hello Companion](./hello-companion.md) - Basic AI perception and planning
- [Fluids Demo](./fluids-demo.md) - Physics simulation
- [Physics Demo](./physics-demo.md) - Rapier3D integration

## Troubleshooting

### Empty plan output

The default run uses `BossDirector`. `plan` pushes `Fortify` only when Manhattan distance is greater than 8 and `terrain_edits > 0`. Otherwise it pushes `SpawnWave` when `spawns > 0` and `Collapse` when `terrain_edits > 0`. Distance greater than 8 with `terrain_edits == 0` still pushes `SpawnWave` while `spawns > 0`. `test_plan_with_spawn_budget` (`astraweave-director/src/lib.rs:145-157`) is that case: player `(0, 0)`, enemy `(5, 5)`, Manhattan distance 10, budget `spawns: 1`, `terrain_edits: 0`, `traps: 0`. The plan length is 1 and the op is `SpawnWave`. The plan is empty only when the fortify condition is false and both `spawns` and `terrain_edits` are zero. `traps` is not read.

An empty `enemies` list uses a target at `player.pos + (6, 0)`. That Manhattan distance is 6, so the fortify arm does not run, and the other arm still emits ops while `spawns` or `terrain_edits` remains. `OathboundWardenDirector::step` returns empty `ops` when `enemies` is empty.

### Warden branch does not compile

Enable `astraweave-director/veilweaver_slice` on the dependency (see `examples/veilweaver_demo/Cargo.toml` for a forwarding pattern). The example's local `veilweaver_slice` flag alone is not enough.

## Source Location

- **Example**: `examples/adaptive_boss/src/main.rs`
- **Director**: `astraweave-director/src/lib.rs`
- **Warden**: `astraweave-director/src/veilweaver_warden.rs` (`veilweaver_slice` feature on the director crate)

## Revision history

### vNEXT

- Align run instructions, expected output, phases, and `DirectorPlan` JSON with `examples/adaptive_boss` and `astraweave-director` sources; document that the warden path is not enabled from this example's features today.
- Warden phase prose matches `update_phase`: one arm per call, no skip from Assessment to DirectiveOverride. Storm detection runs before the phase arm whenever `enemies` is non-empty and keeps the stored choice. `spawns` counts `SpawnWave` ops. A `BossDirector` plan is empty only when `spawns` and `terrain_edits` are both zero. The per-string telegraph loop is labeled illustrative.
- DirectiveOverride lists the ops that phase's arm emits on every non-empty step in the phase, including later steps that skip `choose_adaptive_ability`: one `Collapse` from `build_anchor_rupture` (the helper ignores `terrain_edits`), a `stormbound_wisp` `SpawnWave` with `count: 2` (the push ignores `spawns`), and the stored ability's op (`Fortify` when `terrain_edits > 0`, or `Collapse`).
