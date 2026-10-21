# AstraWeave — AI‑Native Game Engine

**AstraWeave** is a ground‑up engine where **agents are first‑class**. The core loop (Perception → Reasoning → Planning → Action) is built into the sim, not bolted on. It powers Veilweaver’s companion and boss AI, and exposes a stable **SDK** so other teams can build AI‑native titles.

---

## 1) Design Pillars

- **Deterministic sim**: authoritative, fixed‑tick; rendering decoupled.  
- **Data‑oriented**: ECS for cache‑friendly traversal and clean agent IO.  
- **Agents as citizens**: built‑in perception bus, planners, behavior controller.  
- **Tool‑sandbox**: AI can only act through validated tools — no cheats.  
- **Local‑first AI**: 7B–12B quantized LLM for low‑latency intent; cloud “deep think” (optional).  
- **Portable memory**: companions persist via signed, versioned profiles.

---

## 2) Architecture at a Glance

+------------------------+ +---------------------+
| Fixed-Tick Simulation |-----> | Perception Bus |
| (60 Hz, deterministic)| | (world snapshots) |
+------------------------+ +----------+----------+
|
+-------v-------+
| Cognition |
| Planner/LLM |
+-------+-------+
|
+-------v-------+
| Behavior Ctlr |
| + Tool Calls |
+-------+-------+
|
+-------v-------+
| Engine APIs |
| (Validate) |
+---------------+


---

## 3) Systems

### 3.1 Kernel & Scheduling
- **Fixed sim tick** (60 Hz target), variable render.  
- **Determinism**: fixed‑point ops (where sensible), deterministic RNG.  
- **Job graph**: physics → nav → perception → cognition → behavior → animation/audio.

### 3.2 ECS
- **Archetype or sparse‑set**; hot components (Pose, Health, Faction, Threat, Inventory, Abilities, Cooldowns).  
- **Blackboard** per agent: last orders, threat map, goal stack.

### 3.3 Perception Bus
- Emits **WorldSnapshots** (diff‑friendly) to agents on events or cadence:
  - Player state (hp, pos, stance), nearby enemies (redacted, LOS‑filtered), POIs, objective hints.  
  - Companion/boss receives structured summaries over raw geometry.

### 3.4 Cognition Stack
- **Planner** (Utility/GOAP/HTN hybrid): decomposes goals (cover, breach, kite, revive); scores by safety, progress, player orders.  
- **Micro‑policy nets**: tiny models for timing/peeking/aim assist (engine‑validated).  
- **LLM orchestrator**: converts natural orders → plans; **always** outputs structured intents.

### 3.5 Behavior Controller
- Merges planner intent + orders + cooldowns → **atomic actions** (engine‑validated).

### 3.6 Memory Fabric
- **Persona** (tone, risk, humor).  
- **Semantic facts** (safehouses, counters that worked).  
- **Episodic** (play highlights for banter & learning).  
- **Skills** (weapon affinities, combo timings).  
- Stored in **Companion Profile** (`.cprof`): versioned JSON + small vector index; signed & encrypted.

---

## 4) Core Interfaces (Schemas)

### 4.1 World Snapshot → AI
```json
{
  "t": 123.4,
  "player": {"hp": 62, "pos": [12,0,5], "stance": "crouch", "orders": ["hold_east"]},
  "self": {"ammo": 23, "cooldowns": {"smoke": 8}, "morale": 0.7},
  "enemies": [{"id":7,"pos":[18,0,9],"hp":40,"cover":"low","last_seen":121.1}],
  "pois": [{"k":"breach_door","pos":[15,0,8]}],
  "objective": "extract"
}
```

### 4.2 Plan Intent (AI → Engine)
```
{
  "plan_id": "p_4821",
  "steps": [
    {"act":"move_to","x":14.2,"y":0.0,"z":7.1},
    {"act":"throw","item":"smoke","x":16.0,"y":0.0,"z":8.0},
    {"act":"cover_fire","targetId":7,"duration":2.8}
  ]
}
```

### 4.3 Tool Registry (engine‑exposed verbs)
```
{
  "tools": [
    {"name":"move_to","args":{"x":"f32","y":"f32","z":"f32"}},
    {"name":"throw","args":{"item":"enum[smoke,grenade]","x":"f32","y":"f32","z":"f32"}},
    {"name":"cover_fire","args":{"targetId":"u32","duration":"f32"}},
    {"name":"revive","args":{"allyId":"u32"}},
    {"name":"converse","args":{"npcId":"u32","intent":"string"}}
  ],
  "constraints": {"cooldowns": true, "los": true, "stamina": true}
}
```

### 4.4 Companion Profile (.cprof)
```
{
  "version": "1.0.0",
  "persona": {"tone":"dry","risk":"medium","humor":"light","voice":"v_01"},
  "player_prefs": {"stealth_bias": 0.7, "loot_greed": 0.2},
  "facts": [{"k":"safehouse_west","v":"has_medkits","t":"2025-09-01"}],
  "episodes": [{"title":"rescue_echo","summary":"flanked north; clutch revive","tags":["flank","revive"]}],
  "skills": [{"name":"grenade_timing","level":3,"notes":"throw after enemy reload"}],
  "signing": {"pub":"...", "sig":"..."}
}
```

### 4.5 Boss Director (Environment Ops)
```
{
  "director_budget": {"traps": 3, "terrain_edits": 2, "spawn_points": 2},
  "ops": [
    {"op":"fortify","areaId":"A7"},
    {"op":"spawn_wave","archetype":"stagger_unit","count":4},
    {"op":"collapse","node":"bridge_02"}
  ]
}
```

# Validation: The engine verifies every step/op (navmesh, LOS, cooldowns, budgets). Invalid actions are auto‑repaired or rejected with reasons.

## Performance Targets

Local LLM (7B–12B, quantized): 60–150 ms intent calls, event‑driven (not every tick).

Planner: 1–2 Hz cadence, or on triggers (aggro, order, phase shift).

Perception: 10–20 Hz snapshots; deltas preferred.

Render: independent; engine exposes debug overlays for plan trees and utility scores.

## Networking & Anti‑Cheat

Authoritative server in co‑op: plans replicate as intents, not privileged powers.

No special physics for AI — same cooldowns and constraints as players.

Desync guard: if a deep‑think call lags, local micro‑policy handles dodge/retreat until plan arrives.

## Editor & Tooling

Live Plan View: utility heatmaps, selected goals, candidate actions.

Memory Inspector: view persona/facts/episodes; distill/forget controls.

Deterministic Replays: record/replay at frame granularity for A/B testing.

Encounter Fuzzer: churns layouts/enemy mixes to validate robustness.

## SDK (for other games)

Bindings: C/C++/Rust + Godot adapter (Unity/Unreal adapters optional).

Schemas: protobuf/JSON for snapshots, intents, profiles.

Samples: “Hello Companion” (smoke→push), “Adaptive Boss” (fortify vs flank).

Test harness: headless arena, KPIs (assist rate, revive saves, fight length, friendly‑fire).

## Security, Privacy, Safety

On‑device first; cloud opt‑in for “deep think.”

Companion profiles are signed & encrypted; per‑game sandboxes.

Content filters for LLM outputs; regional toggles; full player control.

## Implementation Notes

Lang: Rust for core; small sandboxed script (Rhai/Lua) for quests/triggers.

Graphics: Vulkan via wgpu; Physics: Rapier (deterministic settings).

Nav: Recast/Detour‑style tiles + flow fields.

Inference: llama.cpp/ggml + ONNX Runtime; CUDA/ROCm/Metal backends.
