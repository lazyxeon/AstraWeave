# Veilweaver: Threads of Eternity

**Veilweaver** is the flagship reference implementation for the AstraWeave AI-Native Game Engine. It demonstrates the engine's capabilities through an AI-driven action RPG set in a twilight archipelago of floating islands.

> Archive note: This directory preserves prototype, pre-canon, and vertical-slice-era Veilweaver materials. Names such as Loomspire Isle, Loomspire Sanctum, Echo Grove, and Fractured Cliffs are historical implementation labels in this archive, not the current canon authority. Current canon lore and worldbuilding live in `docs/Veilweaver/`.

## 📁 Directory Structure

```
Games-VEILWEAVER/
├── README.md              # This file
├── design-docs/           # Game design documentation
│   ├── Veilweaver.md      # Comprehensive game design overview
│   ├── AI First Concept.md    # AI-native vision and features
│   └── First concept(no AI).md # Original pitch deck
├── documentation/         # Technical documentation
│   ├── overview.md        # Game overview
│   ├── mechanics.md       # Core game mechanics
│   ├── ai-integration.md  # AI companion and endboss systems
│   └── world-design.md    # World and level design
└── assets/                # Game-specific assets
    └── dialogue_intro.toml # Sample dialogue for Veilweaver
```

## 🎮 About Veilweaver

Veilweaver: Threads of Eternity is an **AI-native Action RPG** where players manipulate fate threads to alter the world while adventuring with a **persistent AI companion**. The game features:

- **AI Companions**: Persistent teammates that learn your playstyle across sessions
- **AI Endbosses**: Adaptive adversaries that evolve tactics and reshape battlefields
- **Fate-Weaving**: Dynamic world manipulation system for altering traversal, puzzles, and combat
- **Echo-Infused Combat**: Soulslike combat with environment-linked abilities
- **Procedural Archipelago**: Floating islands with quantum-inspired phenomena

## 🔗 Integration with AstraWeave

Veilweaver serves as a proof-of-concept for AstraWeave's AI-native capabilities:

- **astraweave-ai**: Powers companion learning and endboss adaptation
- **astraweave-gameplay**: Implements weaving mechanics and echo systems
- **astraweave-dialogue**: Handles dynamic NPC interactions
- **astraweave-pcg**: Generates procedural island layouts
- **astraweave-physics**: Manages fate-thread physics and world alterations

## 📖 Documentation

For detailed information about Veilweaver:

- **Game Design**: See `design-docs/` for complete design documentation
- **Technical Docs**: See `documentation/` for implementation details
- **Engine Integration**: Refer to main AstraWeave documentation in `/docs`

## 🚀 Quick Start

To experience Veilweaver concepts:

```bash
# Run the dialogue demo
cargo run --example quest_dialogue_demo

# Run the cutscene/cinematic demo
cargo run --example cutscene_render_demo

# Validate Loomspire greybox streaming
cargo run -p veilweaver_slice_loader
```

## 🎯 Development Status

Veilweaver is currently in the **design and prototyping phase**. Core engine features are production-ready, and we're actively developing game-specific systems:

- ✅ Dialogue system (prototype)
- ✅ Cinematic/cutscene system
- 🚧 Fate-weaving mechanics
- 🚧 AI companion learning
- 🚧 AI endboss adaptation
- 🚧 Full game loop integration

## 📝 License

Veilweaver, as part of the AstraWeave project, is licensed under the MIT License. See the main [LICENSE](../LICENSE) file for details.

---

*"Weave your destiny alongside a living AI companion — and face AI-driven endbosses that evolve with every battle."*
