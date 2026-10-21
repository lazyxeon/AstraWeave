<!-- Extracted by AD.6 Phase 0.4 (2026-07-16) from commit 0fb845c28, original path 'AI Engine: AstraWeave.md' (added Sept 2025 via GitHub web, deleted 84f68d0a5). The original path is Windows-invalid (colon) and is purged from the rewritten history; this extraction preserves the repo's founding concept document. -->
2) AI-Native Engine (codename: AstraWeave)
2.1 Core architecture

Deterministic Simulation Kernel: fixed 60 Hz sim tick (render decoupled), fixed-point math where feasible, deterministic RNG; server-authoritative mode for co-op.

ECS (data-oriented): archetype or sparse-set ECS for cache-friendly reads; systems for physics, nav, perception, planner, behavior, animation, audio.

Perception Bus: builds compact “world snapshots” (diffs) for agents: entities in LOS, threat summaries, player orders, terrain affordances, event cues.

Planning Stack:

Utility/HTN/GOAP planner for concrete goals (cover, breach, kite, heal).

Micro-policy nets (tiny models) for aim/peek/timing.

LLM orchestrator (optional) for natural-language orders, multi-step plans, roleplay—always outputs structured intents (never raw powers).

Behavior Controller: merges planner intents + player orders, enforces cooldowns, navmesh, stamina, line-of-sight; resolves to atomic actions.

Tool Contract (sandbox): the only way agents act. Tools are schema-checked and validated by the engine (no cheats, ever).

Intent schema (example):

{"plan_id":"p-7c1",
 "steps":[
  {"act":"move_to","x":14.2,"y":0.0,"z":7.1},
  {"act":"throw","item":"smoke","x":16.0,"y":0.0,"z":8.0},
  {"act":"cover_fire","targetId":7,"duration":2.8}
 ]}

2.2 Companion Memory (portable across games)

Persona: temperament, voice, risk, humor.

Semantic facts: allies, safehouses, tactics that worked.

Episodic: stitched summaries of “what happened” (for banter, callbacks).

Skills: learned timings (e.g., “grenade after reload”), weapon affinities.

Format: encrypted, signed Companion Profile (e.g., *.cprof) with versioned JSON + vector index; cloud-sync optional.

2.3 AI Boss “Director”

Boss memory: per-player counters and past defeats.

Environment authoring tools: fortify, flood, collapse, fog, spawn lanes, lure nodes.

Escalation policy: reads player/companion patterns → selects fortify/ambush/siege; enforces budget so it’s fair and readable.

2.4 Inference runtime

Local (reflex): 7B–12B quantized LLM + small policy nets; 60–150 ms target.

Cloud/Edge (deep think; optional): 70B+ for long plans/lore; 500–1500 ms budget; event-driven only.

Safety: profanity/harassment filters; jailbreak guards; regional privacy controls.

2.5 Editor & QA

Live Plan View: utility scores/goal tree in real time.

Memory Inspector: view/edit persona, facts, episodes (distill/forget).

Deterministic Replays: frame-accurate sim record/replay; A/B different models.

Encounter Fuzzer: auto-varies layouts/enemies to verify robustness.

2.6 Tech stack (pragmatic)

Lang: Rust for engine core; tiny scripting (Lua/Rhai) for quests and triggers.

Graphics: Vulkan via wgpu; Physics: Rapier (deterministic settings).

Nav: tiled navmesh (Recast/Detour-style) + flow fields for crowds.

Inference: llama.cpp/ggml + ONNX Runtime; backends for CUDA/ROCm/Metal.

3) Companion & Boss APIs (for all games on the platform)
3.1 Companion API (game → AI)

WorldSnapshot: minimal, diff-friendly state (player hp/pos, enemies simplified, POIs).

PlayerOrders: high-level intents (“hold east”, “non-lethal only”).

ToolRegistry: the verbs the AI is allowed to use in this game.

3.2 Companion API (AI → game)

PlanIntent: ordered steps of validated tools (see JSON above).

Chat/Banter: text/voice events with priority tags (urgent > quip).

PostActionFeedback: success/fail, rationale (for learnability).

3.3 Boss Director API

DirectorBudget: caps for spawns/traps/environmental edits per phase.

EnvironmentOps: fortify(area), breach(route), raise_water(level), collapse(bridge_id)…

CounterLibrary: codified responses to popular player tactics; tunable.
