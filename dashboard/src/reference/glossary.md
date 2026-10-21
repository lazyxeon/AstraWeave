# Glossary

A comprehensive glossary of terms used throughout the AstraWeave documentation.

## A

**Action**
: The final stage of the AI loop where an agent executes planned behaviors through validated tool calls.

**Admonish**
: An mdBook preprocessor for creating callout blocks (warnings, notes, tips).

**Agent**
: An AI-controlled entity in the game world with perception, planning, and action capabilities.

**Arbiter**
: The AI system responsible for validating tool calls and ensuring agents can only perform sanctioned actions within game rules.

**Archetype**
: In ECS, a unique combination of component types. Entities with the same components share an archetype for cache-efficient storage.

**Asset Pipeline**
: The content processing workflow that converts raw assets (textures, models, audio) into engine-optimized formats.

## B

**Batch Executor**
: A system for grouping multiple LLM inference requests to improve throughput.

**Behavior Tree**
: A hierarchical AI decision-making structure where nodes represent conditions, actions, and control flow.

**Blackboard**
: A shared data structure for AI agents to read and write state information.

**Biome**
: A terrain zone with distinct environmental characteristics (forest, desert, etc.).

## C

**Character Controller**
: A physics component that handles player/NPC movement with collision detection.

**Clustered Lighting**
: A rendering technique that divides the view frustum into clusters for efficient many-light rendering.

**Component**
: In ECS, a piece of data attached to an entity. Components contain only data, not behavior.

**Core Loop**
: The fundamental AI cycle: Perception -> Reasoning -> Planning -> Action.

## D

**Deterministic Simulation**
: A simulation where given the same inputs, the outputs are always identical. Essential for replay and networking.

**Director**
: An AI system that orchestrates game pacing, difficulty, and narrative events.

## E

**ECS (Entity Component System)**
: A data-oriented architecture pattern where entities are IDs, components are data, and systems are logic operating on components.

**Entity**
: In ECS, a unique identifier (typically an integer with a generation counter) that components are attached to.

**Episode**
: In memory systems, a recorded sequence of events that can be recalled for learning.

## F

**Fixed Timestep**
: A simulation update rate that runs at a constant interval (e.g., 60Hz) regardless of frame rate.

**Frame**
: One complete update cycle of the game, including input, simulation, and rendering.

## G

**GOAP (Goal-Oriented Action Planning)**
: An AI planning algorithm that finds optimal action sequences to achieve goals by searching through possible world states.

**Generational Index**
: An entity ID with a generation counter to detect use-after-free bugs when entities are recycled.

## H

**HDRI (High Dynamic Range Image)**
: An image format storing extended brightness values, commonly used for environment lighting.

## I

**IBL (Image-Based Lighting)**
: A technique using environment maps to provide realistic ambient lighting.

## K

**KTX2**
: A texture container format supporting GPU compression and mipmaps.

## L

**LLM (Large Language Model)**
: AI models (like GPT, Llama, Phi) used for natural language understanding and generation.

**LOD (Level of Detail)**
: Rendering optimization that uses simpler geometry for distant objects.

## M

**Mermaid**
: A JavaScript-based diagramming tool integrated into mdBook for rendering flowcharts and diagrams.

**Mipmap**
: Pre-calculated, progressively smaller versions of a texture for efficient rendering at different distances.

**MRA**
: Metallic-Roughness-AO texture packing format used in PBR materials.

## N

**Nanite**
: Unreal Engine 5's virtualized geometry system. AstraWeave implements similar techniques for GPU-driven LOD.

**Navmesh (Navigation Mesh)**
: A polygon mesh representing walkable areas for AI pathfinding.

## O

**Orchestrator**
: The AI component that coordinates multiple agents and manages their interactions.

## P

**PBR (Physically Based Rendering)**
: A rendering approach that simulates realistic light behavior using physical principles.

**Perception**
: The first stage of the AI loop where agents observe the game world state.

**Planning**
: The AI stage where goals are decomposed into action sequences.

**Preprocessor**
: An mdBook tool that transforms source files before rendering.

## Q

**Query**
: In ECS, a request for entities matching specific component patterns.

## R

**RAG (Retrieval-Augmented Generation)**
: A technique combining information retrieval with LLM generation for context-aware responses.

**Ragdoll**
: A physics-based character simulation for realistic death/knockback animations.

**Rapier**
: The Rust physics engine used by AstraWeave.

**Rhai**
: An embedded scripting language for Rust, used in AstraWeave for game logic.

## S

**Sandbox**
: An isolated execution environment for AI tool calls, preventing unauthorized actions.

**SIMD (Single Instruction Multiple Data)**
: CPU instructions processing multiple data points simultaneously for performance.

**Spatial Hash**
: A data structure for efficient spatial queries (collision detection, neighbor finding).

**System**
: In ECS, logic that operates on entities with specific component combinations.

## T

**Tick**
: One fixed timestep update of the simulation.

**Tool**
: In AI systems, an action an agent can take, validated by the Arbiter.

**Tool Vocabulary**
: The set of actions available to AI agents, each with schema and validation rules.

## U

**Utility AI**
: An AI approach where actions are scored by utility functions and the highest-scoring action is selected.

## V

**Voxel**
: A 3D pixel, used in terrain systems for destructible/modifiable environments.

## W

**WGPU**
: A Rust graphics library providing a cross-platform abstraction over Vulkan, Metal, DX12, and WebGPU.

**WGSL (WebGPU Shading Language)**
: The shader language used with WGPU.

**World**
: The container for all entities, components, and resources in the engine.

## See Also

- [Architecture Overview](../architecture/overview.md)
- [AI-Native Design](../architecture/ai-native.md)
- [ECS Architecture](../architecture/ecs.md)
