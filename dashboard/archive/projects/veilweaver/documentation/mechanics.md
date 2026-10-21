# Game Mechanics

## Core Loops in the Vertical Slice

1. **Explore → Weave → Advance**
   - Bridge reconstruction and path redirection tutorials teach the `Thread Anchor` and `Resonant Guard` abilities.
   - Thread stability is capped to prevent softlocks; every weave has a defined reversal cost.
2. **Combat → Synergize → Adapt**
   - Echo-infused combat rewards positional play; weaving barricades modifies enemy pathfinding.
   - Companion Aria reacts to combat telemetry, surfacing combo prompts when stagger thresholds are met.
3. **Decide → Consequence → Boss Adaptation**
   - Loom Crossroads choice toggles arena modifiers and boss move sets, proving determinism of branching outcomes.

## Player Toolkit

- `Thread Anchor`: Targeted weave that restores broken structures; costs 1 Echo Shard, refunds on reversal.
- `Echo Pulse`: Quick ranged attack, builds stagger meter.
- `Resonant Guard`: Reaction weave triggered after perfect parry; emits area knockback.
- `Echo Dash`: Mobility burst unlocked mid-slice; inherits directionality from input vector.

## Companion Behaviors

- Aria operates on a GOAP planner with goals `ProtectPlayer`, `ExploitStagger`, and `StabilizeThreads`.
- Behavior weights shift based on observed player style (melee vs weaving emphasis) sampled every 10 seconds.
- LLM fallback (Hermes 2 Pro) supplies contextual banter and choice consequences, with deterministic gating to maintain reproducibility.

## Boss Director Mechanics

- The Oathbound Warden monitors player damage sources (melee, ranged, weaving) and enabling systems adjust its resistances accordingly.
- Boss phases trigger `ArenaWeaveEvents` that toggle environmental hazards in sync with player choice flags.
- Telemetry events capture adaptation rationale for post-run recap screens.