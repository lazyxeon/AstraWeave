# AI Integration

## Companion Aria

- **Planner Stack**: Utilizes `astraweave-ai` GOAP orchestrator for deterministic decision making, augmented by Hermes 2 Pro LLM responses for narrative banter.
- **Memory Layers**:
  - *Episodic*: Stores recent combat events (stagger assists, player low-health saves) for dialog callbacks.
  - *Semantic*: Tracks thread stability thresholds per region to inform weaving suggestions.
- **Adaptive Unlocks**: Monitors player input ratios to choose between `Threadbind Riposte` (melee support) or `Stability Surge` (defensive weave) mid-demo.
- **Detailed Spec**: See `design-docs/ARIA_COMPANION_BEHAVIOR.md` for the GOAP graph, memory architecture, and telemetry hooks.

## Enemy AI

- **Rift Stalkers**: Configured through behavior trees emphasizing flanking and thread disruption. They target `ThreadAnchor` props to pressure weaving uptime.
- **Echo-bound Sentinels**: Use utility-based AI to choose between barrier deployment and ranged burst depending on player proximity.

## Boss Director

- The Oathbound Warden runs a three-stage adaptive policy:
  1. *Assessment*: Samples player DPS vectors every 5 seconds, chooses counter-formations.
  2. *Fulcrum Shift*: Consumes the `StormRoute` flag set during the Loom Crossroads choice to configure arena hazards.
  3. *Directive Override*: Spawns support wisps or anti-weave fields based on accumulated telemetry.
- Director decisions emit `BossAdaptationEvent` entries consumed by post-run UI and logged for determinism validation.
- **Detailed Spec**: See `design-docs/OATHBOUND_WARDEN_ENCOUNTER.md` for state machine logic, ability catalog, and telemetry requirements.

## Dialogue & Narrative AI

- Dialogue sequences extend `docs/projects/veilweaver/assets/dialogue_intro.toml` with branching nodes that reference runtime state (choice decisions, combat performance).
- LLM outputs are sandboxed through the existing tool-call validator, ensuring actions remain within authored budgets.