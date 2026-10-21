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
Veilweaver is developed in a separate repository and is currently in active development.
```

The Veilweaver documentation is being migrated to this documentation site. For the latest information:

- **GitHub Repository**: [Games-VEILWEAVER](https://github.com/lazyxeon/Games-VEILWEAVER)
- **Design Documents**: Located in `docs/archive/projects/veilweaver/`

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

## Getting Started with Veilweaver

```bash
# Clone the Veilweaver repository
git clone https://github.com/lazyxeon/Games-VEILWEAVER.git

# Build and run
cd Games-VEILWEAVER
cargo run --release
```

## Additional Documentation

For detailed Veilweaver documentation, see the [Games-VEILWEAVER repository](https://github.com/lazyxeon/Games-VEILWEAVER):

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
