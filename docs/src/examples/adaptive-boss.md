# Adaptive Boss Walkthrough

The `adaptive_boss` example demonstrates AstraWeave's **Director System** - an AI-driven game master that orchestrates boss encounters with budget-constrained tactical decisions.

## Running the Example

```bash
cargo run -p adaptive_boss --release
```

The default build uses `BossDirector` from `astraweave-director`. The source also contains a `#[cfg(feature = "veilweaver_slice")]` branch for `OathboundWardenDirector`, but the example's empty `veilweaver_slice = []` feature does **not** enable `astraweave-director/veilweaver_slice`, so `cargo run -p adaptive_boss --features veilweaver_slice` does not compile today. The warden implementation lives in the director crate when that dependency feature is on (see `astraweave-director/src/veilweaver_warden.rs`).

## What It Demonstrates

- **BossDirector**: Heuristic boss planner (default run)
- **Director budget**: Limited resources for spawns, traps, and terrain edits
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
    traps: 2,      // Can place 2 traps
    terrain_edits: 2, // Can modify terrain twice
    spawns: 2,     // Can summon 2 minions
};
```

The budget prevents the Director from overwhelming the player. Each action consumes budget:
- **Spawns**: Summoning minions costs 1 spawn point
- **Traps**: Placing hazards costs 1 trap point
- **Terrain edits**: Blocking paths or creating cover costs 1 edit point

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

`OathboundWardenDirector` uses three phases defined in `veilweaver_warden.rs`:

- **Assessment**: Observes the fight, may fortify a choke, telegraph `Chains tense—the Warden studies your form.`
- **FulcrumShift**: Entered at or below two-thirds of starting boss HP; storm detection (`objective` strings or `storm_route` POIs) picks **Stabilize** vs **Redirect** for this phase's ops
- **DirectiveOverride**: Entered at or below one-third of starting boss HP; picks an adaptive ability (`AntiRangedField` or `CounterShockAura`) from player orders

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

When the warden branch is enabled, telegraphs are plain strings on `WardenDirective`:

```rust
if !directive.telegraphs.is_empty() {
    for telegraph in &directive.telegraphs {
        println!("{}", telegraph);
    }
}
```

Examples from `veilweaver_warden.rs`:

- `Chains tense—the Warden studies your form.` (Assessment)
- `Storm calms—armor plates seal the Warden.` (FulcrumShift, Stabilize)
- `Motes surge across the arena—vision falters.` (FulcrumShift, Redirect)

## Key Concepts

### Budget-Constrained AI

Unlike traditional boss AI that can spam abilities, the Director operates under resource constraints. This creates:
- **Strategic decisions**: Trade-offs between aggression and defense
- **Fair encounters**: Players can anticipate resource depletion
- **Dynamic difficulty**: Budget recharges over time

### Phase-Based Behavior (warden)

Phase transitions depend only on boss HP (`update_phase` in `veilweaver_warden.rs`): the director moves to **FulcrumShift** when current HP is at or below two-thirds of the HP captured on the first step, and to **DirectiveOverride** at or below one-third. Storm detection runs each step but only chooses **Stabilize** or **Redirect** for behavior inside **FulcrumShift**; it does not change phase.

## Related Examples

- [Hello Companion](./hello-companion.md) - Basic AI perception and planning
- [Fluids Demo](./fluids-demo.md) - Physics simulation
- [Physics Demo](./physics-demo.md) - Rapier3D integration

## Troubleshooting

### Empty plan output

If the Director returns an empty plan, the budget may already be exhausted or the boss doesn't see any targets.

### Warden branch does not compile

Enable `astraweave-director/veilweaver_slice` on the dependency (see `examples/veilweaver_demo/Cargo.toml` for a forwarding pattern). The example's local `veilweaver_slice` flag alone is not enough.

## Source Location

- **Example**: `examples/adaptive_boss/src/main.rs`
- **Director**: `astraweave-director/src/lib.rs`
- **Warden**: `astraweave-director/src/veilweaver_warden.rs` (`veilweaver_slice` feature on the director crate)

## Revision history

### vNEXT

- Align run instructions, expected output, phases, and `DirectorPlan` JSON with `examples/adaptive_boss` and `astraweave-director` sources; document that the warden path is not enabled from this example's features today.
- Warden phase prose: HP thresholds (≤⅔, ≤⅓) drive phases; storm choice affects FulcrumShift only.
