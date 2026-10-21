# Adaptive Boss Walkthrough

The `adaptive_boss` example demonstrates AstraWeave's **Director System** - an AI-driven game master that orchestrates boss encounters with dynamic phase transitions, telegraphed attacks, and budget-constrained tactical decisions.

## Running the Example

```bash
cargo run -p adaptive_boss --release
```

For the full Veilweaver boss experience (requires `veilweaver_slice` feature):

```bash
cargo run -p adaptive_boss --release --features veilweaver_slice
```

## What It Demonstrates

- **BossDirector**: AI that plans boss encounter actions
- **Phase-based combat**: Boss behavior changes based on situation
- **Director budget**: Limited resources for spawns, traps, and terrain edits
- **Telegraph system**: Visual/audio warnings before powerful attacks
- **OathboundWardenDirector**: Full Veilweaver boss with Fate-weaving mechanics

## Expected Output

```
Warden phase: Stalking
Telegraphs: ["Thread gathering at position (5, 3)"]
Director plan: {
  "spawn": [{"enemy": "shade", "pos": [4, 1]}],
  "traps": [{"type": "thread_snare", "pos": [6, 2]}],
  "terrain": []
}
Remaining budget: traps=1, terrain_edits=2, spawns=1
```

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

### 5. Veilweaver Director (Advanced)

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

The Oathbound Warden has distinct phases:
- **Stalking**: Observing, placing traps, gathering threads
- **Weaving**: Manipulating fate threads, buffing/debuffing
- **Severing**: Aggressive attacks, breaking player connections
- **Unraveling**: Desperate phase when low HP

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
│  Output: DirectorPlan { spawns, traps, terrain }       │
└────────────────────────────────────────────────────────┘
```

## Telegraph System

Telegraphs provide fair warning to players before powerful attacks:

```rust
if !directive.telegraphs.is_empty() {
    for telegraph in &directive.telegraphs {
        // Display visual/audio warning
        // e.g., "Thread gathering at position (5, 3)"
    }
}
```

Telegraph types:
- **Thread gathering**: Fate-weaving attack incoming
- **Ground tremor**: Area attack warning
- **Shadow forming**: Minion spawn location
- **Pattern shift**: Phase transition imminent

## Key Concepts

### Budget-Constrained AI

Unlike traditional boss AI that can spam abilities, the Director operates under resource constraints. This creates:
- **Strategic decisions**: Trade-offs between aggression and defense
- **Fair encounters**: Players can anticipate resource depletion
- **Dynamic difficulty**: Budget recharges over time

### Phase-Based Behavior

The boss doesn't just cycle through attacks. Phases emerge from:
- **HP thresholds**: Different behavior at 75%, 50%, 25% HP
- **Player behavior**: Adapts to aggressive vs defensive playstyles
- **Companion effectiveness**: Responds to ally threat level

## Related Examples

- [Hello Companion](./hello-companion.md) - Basic AI perception and planning
- [Fluids Demo](./fluids-demo.md) - Physics simulation
- [Physics Demo](./physics-demo.md) - Rapier3D integration

## Troubleshooting

### Missing veilweaver_slice feature
The advanced Warden Director requires the Veilweaver game data:
```bash
cargo run -p adaptive_boss --release --features veilweaver_slice
```

### Empty plan output
If the Director returns an empty plan, the budget may already be exhausted or the boss doesn't see any targets.

## Source Location

- **Example**: `examples/adaptive_boss/src/main.rs`
- **Director**: `astraweave-director/src/lib.rs`
- **Warden**: `astraweave-director/src/oathbound_warden.rs` (feature-gated)
