# Veilweaver: Threads of Eternity

> **Reference Implementation for AstraWeave AI-Native Game Engine**

Veilweaver is a complete AI-native Action RPG that serves as AstraWeave's flagship reference implementation. It demonstrates the full capabilities of the engine in a production game context.

## Overview

Veilweaver showcases:

- **AI Companions** with persistent memory and adaptive behavior
- **Adaptive Boss Battles** that learn from player strategies
- **Dynamic World Events** driven by AI directors
- **Emergent Narrative** through AI-orchestrated storytelling
- **Procedural Content** integrated with hand-crafted design

## Current Status

```admonish info
The design notes are in this repository. They are not a second project checkout.
```

The reference notes live in this repository:

- **Design documents**: [docs/archive/projects/veilweaver](https://github.com/lazyxeon/AstraWeave/tree/main/docs/archive/projects/veilweaver)

This page is an engine-facing reference overview, not a canon authority. Canon narrative, geography, and worldbuilding for the current workspace live in `docs/Veilweaver/`.

## Key Features Demonstrated

### AI Companion System

Veilweaver's companions use AstraWeave's full AI stack:

- **Perception Bus**: Companions observe the game world in real-time
- **GOAP Planning**: Goal-oriented action planning for complex behaviors
- **Memory System**: Companions remember interactions and adapt
- **LLM Integration**: Natural language dialogue and reasoning

### Weaving System

The signature gameplay mechanic demonstrates:

- **Tool Sandbox**: AI-validated player abilities
- **Deterministic Simulation**: Consistent weaving effects
- **Physics Integration**: Weave-affected environments

### Reference Slice Design

Three prototype biome slices showcase terrain systems. These are reference-implementation labels for the current vertical slice, not authoritative canon place names:

1. **Loomspire Sanctum** - Tutorial and hub area
2. **Echo Grove** - Forest exploration zone
3. **Fractured Cliffs** - Vertical traversal challenges

## Additional Documentation

Design notes for the reference implementation are in [docs/archive/projects/veilweaver](https://github.com/lazyxeon/AstraWeave/tree/main/docs/archive/projects/veilweaver):

- Game mechanics guide
- AI integration deep-dive
- World design principles
- Asset creation workflow
- Modding support guide

## See Also

- [AI-Native Design](../architecture/ai-native.md)
- [AI Companions Guide](../game-dev/companions.md)
- [Adaptive Bosses Guide](../game-dev/bosses.md)
- [Examples Index](../examples/index.md)
