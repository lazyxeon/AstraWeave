---
schema_version: 1
trace_id: ai_pipeline
title: "AI Pipeline + AI Core + GOAP + Behavior Trees + LLM Integration + Arbiter"
description: "AI Pipeline (with 8 subsystem traces)"
primary_crate: astraweave-ai
domain: ai
lifecycle_status: active
integration_status: wired
owns: [astraweave-ai, astraweave-behavior, astraweave-context, astraweave-coordination, astraweave-dialogue, astraweave-director, astraweave-embeddings, astraweave-llm, astraweave-memory, astraweave-npc, astraweave-persona, astraweave-prompts, astraweave-rag]
doc_version: "1.11"
last_verified_commit: 32afac52f
---

# Architecture Trace: AI Pipeline + AI Core + GOAP + Behavior Trees + LLM Integration + Arbiter

## Metadata

| Field | Value |
|---|---|
| **System name** | AI Pipeline (Perception → Reasoning → Planning → Action) — foundational AI-first substrate |
| **Primary crates** | `astraweave-ai` (39 files / 30.5K LoC), `astraweave-behavior` (6 / 5K), `astraweave-llm` (29 / 23.5K), `astraweave-memory` (17 / 11.5K), `astraweave-director` (6 / 3.3K), `astraweave-npc` (6 / 1.7K), `astraweave-dialogue` (4 / 5K), `astraweave-coordination` (5 / 5.3K) — **8 crates, 112 files, ~85K LoC total** |
| **Document version** | 1.11 |
| **Last verified against commit** | `32afac52f` |
| **Last verified date** | 2026-05-12 |
| **Status** | **Active foundation — AI-first citizen of the engine.** The canonical loop is stable and validated for 12,700+ agents @ 60 FPS (per `astraweave-ai/src/lib.rs:27`). The Arbiter / dual-executor / LLM fallback chain has shipped multiple iterations and is hardening continuously. Subsystems like Memory, Director, NPC, Dialogue, Coordination are each large enough to warrant per-system follow-up traces. |
| **Owner notes** | This trace is a **navigational map + load-bearing detail**, not an exhaustive per-file enumeration. Scale: 85K LoC across 8 crates. The canonical loop (`astraweave-ai/src/core_loop.rs` + `orchestrator.rs` + `tool_sandbox.rs` + `ai_arbiter.rs` + `llm_executor.rs` + `astraweave-behavior/src/goap.rs` + `astraweave-llm/src/{plan_parser,fallback_system,tool_guard,production_hardening}.rs`) is documented in detail. **Subsystem trace for Memory added 2026-05-12 (see §13.1)** — surfaced dormancy of the main memory pipeline w.r.t. the runtime AI loop. **Subsystem trace for Director added 2026-05-12 (see §13.2)** — confirmed Director is a parallel sibling AI subsystem (boss/encounter authoring) with its own validation gate `apply_director_plan`; the Companion-AI loop and Director loop share `WorldSnapshot` but otherwise do not interact. **Subsystem trace for NPC added 2026-05-12 (see §13.3)** — confirmed NPC is a fully isolated AI subsystem with its own parallel vocabulary (`NpcAction`/`NpcPlan`/`NpcWorldView`/`LlmAdapter`/`CommandSink`), zero imports of canonical AI types, and direct physics/audio integration rather than ECS-loop participation. **Subsystem trace for Dialogue added 2026-05-12 (see §13.4)** — confirmed Dialogue has a **two-layer architecture**: the basic `DialogueGraph`/`DialogueRunner`/`toml_loader` path is actively production-wired into `veilweaver_slice_runtime` and the editor; the LLM-enhanced `llm_dialogue.rs` (2941 LoC / 60% of crate) is dormant with zero external consumers. **Subsystem trace for Coordination added 2026-05-12 (see §13.5)** — most aggressively dormant subsystem traced: zero workspace consumers of any kind (no examples, no tools, no game-loop crates), explicit `#[allow(dead_code)]` "reserved for future…" markers on 7+ struct/field locations, three commented-out module declarations whose source files were never created. The entire `AgentCoordinator`/`WorldEventGenerator`/`NarrativeCoherenceEngine` surface is in-design. **Subsystem trace for Advanced GOAP added 2026-05-12 (see §13.6)** — largest subsystem traced (22 files / 16.7K LoC) and most architecturally mature among the dormant subsystems: feature-gated on `planner_advanced`, structured across 6 phases (0-5) of explicit phase-numbered development, integrates with the canonical loop via the `Orchestrator` trait, includes built-in shadow-mode infrastructure for safe rollout, but currently has zero production constructors outside tests/benches and three disabled CLI bins. **Subsystem trace for LLM Production Hardening added 2026-05-12 (see §13.7)** — second-deepest LoC dormancy reservoir at ~15K LoC across 16 files. Confirmed only one primitive (`parse_llm_response`) is actively wired in production (via `FallbackOrchestrator` Tier-1/Tier-2), but `FallbackOrchestrator::new` itself is doc-comment-only with zero non-test constructors. The runtime `AIArbiter`/`LlmExecutor` path bypasses the entire hardening surface — no rate limiting, no circuit breaking, no backpressure, no A/B routing, no retry, no telemetry, no ToolGuard, no 4-tier fallback. The 4096-char prompt limit and `safe_llm_invoke` stub are unreachable code. **Subsystem trace for RAG added 2026-05-12 (see §13.8)** — closes the original §11 follow-up list. Three-crate stack (`astraweave-rag` + `astraweave-embeddings` + `astraweave-context`, ~12.3K LoC total). Foundation primitives are active (`TokenCounter::new` is the most actively-wired primitive — instantiated by `RagPipeline::new` AND `ConversationHistory::new`), but the `RagPipeline` composite is dormant for the runtime AI loop: held as a field in 5 LLM-enhanced consumer crates (Director, Quests, Persona, Dialogue, Coordination), all of which are themselves dormant per §13.2/§13.4/§13.5. Surfaced two parallel-implementation anti-patterns (matching the CLAUDE.md "Architecture Drift" warning): (a) a SECOND orphaned `RagPipeline` struct in `astraweave-ai/src/rag/pipeline.rs:115` that never compiles because `pub mod rag` is not declared in lib.rs; (b) internal `astraweave-rag` crate duplication of `InjectionConfig` + `InjectionResult` types (different field sets, same names, two modules). Also surfaced "vocabulary-only" enum drift: `RetrievalMethod` 4 unused variants, `DiversityStrategy` 4 unused variants, `InjectionStrategy` 4 unused variants, plus `VectorStoreWrapper::get_all_memories` returns empty Vec making consolidation a no-op, plus HNSW indexing advertised but linear-scan implemented. **All originally enumerated per-subsystem follow-up traces are now complete.** **Verification pass 2026-05-12 (version 1.9):** resolved 22 markers across §1 / §3 / §5 / §6 / §8 / §10 / §11 / §13.2 / §13.3 / §13.4 / §13.5 / §13.6 / §13.7 / §13.8 with file:line evidence. Three factual corrections surfaced: (a) `GOAPOrchestrator` (Advanced GOAP) does NOT implement the `Orchestrator` trait despite earlier trace claim; (b) `plan_to_intent` emits `tracing::warn!` on unknown actions (not silent drop as previously hedged); (c) `BatchInferenceExecutor` IS actively wired via `FallbackOrchestrator` (not "[NEEDS VERIFICATION]"). One bug-class confirmation: `ProductionHardeningLayer` has no `Drop` impl, confirming runtime resource leak risk without explicit `shutdown().await`. **Deep investigation pass 2026-05-12 (version 1.10):** closed 2 factual §11 Open Questions (`*_stub` grep result, `AIArbiter` non-Send violation check) by moving resolutions into §8 Invariants 19 and 20. Enriched 4 decisional §11 questions with deeper factual context: `AIControlMode` vs `PlannerMode` (variant-set diff), Hermes2Pro/Qwen3/Phi3 (3-client coexistence + phi3:medium runtime default contradiction), `astraweave-ai` forbid_unsafe (grep verified absent crate-wide), `VectorStoreWrapper` consolidation no-op (verified no test catches the bug). Surfaced one new cognitive trap in §6: runtime LLM default is phi3:medium despite doc-comment Qwen3 claims. **Verification pass 2026-05-12 (version 1.11):** resolved 5 additional markers in §7 (Interner test isolation — no `serial_test` in astraweave-behavior), §13.2 (Director context/prompts deps enrichment), §13.3 (`PhysicsWorld::control_character` confirmed to use dt internally), §13.4 (D12 LlmDialogueSystem Send+Sync fields audited, TOML speaker field NOT retained, RunnerState Idle/Finished externally observable only via `is_finished()`). Verified git log searches for "Phase 7" + "WorldState" + "fallback" returned no relevant AI-pipeline commits — §7 Decision Log "[Reasoning not recovered]" markers remain unrecoverable per Default 3 (honest acknowledgment of missing rationale). |

---

## 1. Executive Summary

**What this system does:**
Implements AstraWeave's AI-first architecture: the four-stage loop **Perception → Reasoning → Planning → Action**. Builds a deterministic `WorldSnapshot` from the simulation, dispatches it through one of several orchestrators (Rule, Behavior Tree, GOAP, LLM, or the hybrid `AIArbiter`), produces a `PlanIntent` of validated `ActionStep`s, and routes those steps through the engine's tool sandbox (`astraweave_core::validation::validate_and_execute`) so AI cannot bypass game rules. The `AIArbiter` is the hybrid path (feature `llm_orchestrator`): GOAP provides instant tactical control (5-30 µs per update) while the LLM (runtime default phi3:medium via Ollama; Qwen3/Hermes2Pro opt-in via OLLAMA_MODEL) generates strategic multi-step plans in the background (3-8 s), with seamless mode transitions and a 4-tier fallback chain (FullLlm → SimplifiedLlm → Heuristic → Emergency).

**Why it exists:**
This is **the** AI-native substrate — it's why AstraWeave exists as an engine and not just another wgpu wrapper. Per CLAUDE.md: "AI agents are first-class citizens." The canonical loop, the tool sandbox, and the dual-executor arbiter together encode the architectural commitment that AI cheating is impossible (every action validated by the engine) and that LLM latency is invisible (GOAP keeps control while LLM thinks).

**Where it primarily lives:**
- **Canonical loop & dispatch:** `astraweave-ai/src/{core_loop,orchestrator,tool_sandbox,ecs_ai_plugin}.rs`
- **Arbiter & async LLM:** `astraweave-ai/src/{ai_arbiter,llm_executor,async_task}.rs` (feature `llm_orchestrator`)
- **Behavior Trees:** `astraweave-behavior/src/lib.rs` (`BehaviorNode`, `BehaviorGraph`, `DecoratorType`, `BehaviorContext`, `BehaviorStatus`) + `astraweave-behavior/src/ecs.rs` (`CBehaviorGraph`, `behavior_tick_system`)
- **GOAP (canonical):** `astraweave-behavior/src/goap.rs` (`WorldState`, `GoapAction`, `GoapGoal`, `GoapPlanner`) + `goap_cache.rs` (LRU) + `interner.rs`
- **GOAP (advanced):** `astraweave-ai/src/goap/` (22 files, feature `planner_advanced` — actions, goals, planner, learning, persistence, shadow_mode, plan_analyzer, plan_stitcher, plan_visualizer, goal_authoring, goal_scheduler, goal_validator)
- **LLM integration:** `astraweave-llm/src/{lib,llm_adapter,plan_parser,fallback_system,heuristics,tool_guard,production_hardening,prompts,prompt_template,compression,streaming_parser,scheduler,batch_executor,rate_limiter,circuit_breaker,backpressure,retry,telemetry,ab_testing}.rs` + clients (`phi3.rs`, `hermes2pro_ollama.rs`, `phi3_ollama.rs`, `qwen3_ollama.rs`) + caches (`cache/`)
- **Memory:** `astraweave-memory/src/{memory_types,memory_manager,episode,episode_recorder,storage,pattern_detection,preference_profile,dynamic_weighting,learned_behavior_validator,consolidation}.rs`
- **Director (boss AI):** `astraweave-director/src/{lib,llm_director,components,phase,systems,veilweaver_warden}.rs`
- **NPC, Dialogue, Coordination:** their respective crates
- **Schema (shared types):** `astraweave-core/src/schema.rs` — `WorldSnapshot`, `PlanIntent`, `ActionStep`, `PlayerState`, `CompanionState`, `EnemyState`, `Poi` (see `docs/architecture/ecs_math_core_sdk_foundation.md` §3-§6 for the cross-crate schema discussion)
- **Validation boundary:** `astraweave-core/src/validation.rs::validate_and_execute(world, actor, intent, cfg, log)` — the engine-side tool sandbox where AI plans actually become game state changes

**Status note (read first):**
1. **AI is the engine's first-class citizen.** Per CLAUDE.md Mandate: "AI agents are first-class citizens." Every decision in this trace should be read against that frame.
2. **The canonical loop is stable and validated.** Per `astraweave-ai/src/lib.rs:22-27`: GOAP planning 1.01 µs cache hit / 47.2 µs cache miss; Behavior trees 57-253 ns per tick; Arbiter cycle 313.7 ns; **12,700+ agents @ 60 FPS capacity** (validated).
3. **The Arbiter is the production hybrid path.** GOAP for instant control, the LLM (runtime default phi3:medium; Qwen3-8B opt-in via OLLAMA_MODEL) for strategic plans, BT as emergency fallback. Dual-executor (strategic thinking-mode + optional fast non-thinking) added per `ai_arbiter.rs:1-46` doc-comment architecture diagram.
4. **Two GOAP implementations coexist** (canonical in `astraweave-behavior`, advanced in `astraweave-ai/src/goap/` behind `planner_advanced`). Two control-mode enums coexist (`PlannerMode` for dispatch; `AIControlMode` for arbiter state). See §6.
5. **The historical model is Hermes2Pro; the current model is Qwen3.** CLAUDE.md refers to "GOAP+Qwen3 Hybrid"; older docs (e.g. `ARCHITECTURE_REFERENCE.md:41`) say "GOAP+Hermes Hybrid". The arbiter doc-comment at `ai_arbiter.rs:1` says "GOAP+Qwen3". Recent commit `2468b25f1` per workspace git log: "Replace Phi3 with Hermes2Pro and add UI fixes, latency optimizations, and advanced features".
6. **`WorldSnapshot` field names are load-bearing in LLM prompts.** This is documented in `docs/architecture/ecs_math_core_sdk_foundation.md` Invariant 8. Renaming a field here can require coordinated LLM-prompt updates and may invalidate the LLM's training corpus.
7. **All AI crates are unsafe-free.** `astraweave-llm/src/lib.rs:1`, `astraweave-behavior/src/lib.rs:1`, `astraweave-memory/src/lib.rs:1`, `astraweave-director/src/lib.rs:1`, `astraweave-coordination/src/lib.rs:1`, `astraweave-npc/src/lib.rs:1`, `astraweave-dialogue/src/lib.rs:1` all use `#![forbid(unsafe_code)]`. `astraweave-ai/src/lib.rs:1` does NOT declare it (verified 2026-05-12 — line 1 is `//! # AstraWeave AI` doc-comment); however, workspace grep across `astraweave-ai/src/*.rs` for any `unsafe` block returned zero matches outside `use`/`forbid_unsafe`/comment contexts (verified 2026-05-12). No unsafe code is present in the crate.

---

## 2. Authoritative Pipeline

The AI system has several parallel and nested flows. The canonical one is the four-stage loop; the Arbiter wraps it; the LLM fallback chain backs the LLM call; the validation boundary closes it.

### 2.1 Canonical AI loop (Perception → Reasoning → Planning → Action)

```text
[Tick N — ECS Schedule stage = "ai_planning"]
    │
    │ astraweave_core::perception::build_snapshot(...)  → astraweave_core/src/perception.rs
    │   (or AiPlanningPlugin::sys_ai_planning  → astraweave-ai/src/ecs_ai_plugin.rs:45)
    ▼
[Stage P (Perception): build WorldSnapshot]
    file: astraweave-core/src/schema.rs (struct WorldSnapshot at line 270)
    role: Filtered, deterministic view of game state for AI consumption.
    fields: t, player: PlayerState, me: CompanionState, enemies: Vec<EnemyState>,
            pois: Vec<Poi>, obstacles: Vec<IVec2>, objective: Option<String>
    │
    ▼
[Stage R (Reasoning): dispatch to orchestrator]
    files: astraweave-ai/src/core_loop.rs (PlannerMode, CAiController, dispatch_planner),
           astraweave-ai/src/orchestrator.rs (trait Orchestrator + RuleOrchestrator + OrchestratorAsync)
    role: Route the snapshot to the appropriate planner based on CAiController.mode
    branches:
      ├─ PlannerMode::Rule       → RuleOrchestrator::propose_plan
      ├─ PlannerMode::BehaviorTree (feature ai-bt)   → BT graph tick
      ├─ PlannerMode::GOAP (feature ai-goap)         → GoapPlanner::plan
      └─ (Arbiter path — §2.2)   → AIArbiter::update
    │
    ▼
[Stage L (LLM only): LLM plan generation (async path)]
    files: astraweave-ai/src/llm_executor.rs (LlmExecutor::generate_plan_async),
           astraweave-ai/src/async_task.rs (AsyncTask<T> non-blocking wrapper),
           astraweave-llm/src/fallback_system.rs (FallbackOrchestrator, 4-tier degradation),
           astraweave-llm/src/plan_parser.rs (parse_llm_response with 5 extraction methods),
           astraweave-llm/src/{prompts,prompt_template,compression}.rs
    role: Generate a strategic PlanIntent via LLM. Tier 1 (FullLlm) → Tier 2 (SimplifiedLlm)
          → Tier 3 (Heuristic) → Tier 4 (Emergency: Scan + Wait).
    │
    ▼
[Stage Pl (Planning): produce PlanIntent]
    file: astraweave-core/src/schema.rs (PlanIntent { plan_id, steps: Vec<ActionStep> })
    role: Canonical AI output. ActionStep variants cover all engine-exposed verbs
          (MoveTo, Throw, CoverFire, Revive, Interact, etc.)
    │
    ▼
[Stage V (Validation / Tool Sandbox)]
    files: astraweave-ai/src/tool_sandbox.rs (ToolVerb enum, validation categories),
           astraweave-llm/src/tool_guard.rs (LLM-output-side guard with policies),
           astraweave-core/src/validation.rs (validate_and_execute — engine-side)
    role: Two gates. tool_guard validates BEFORE execution (allowlist/denylist + policy).
          validate_and_execute is the ENGINE-SIDE gate: checks path existence, line of sight,
          cooldowns, resource availability, applies side effects only if all checks pass.
          Returns EngineError variants: InvalidAction, NoPath, LosBlocked, Cooldown.
    │
    ▼
[Stage A (Action): mutate world state if validation passes]
    file: astraweave-core/src/validation.rs:10-… (validate_and_execute applies effects)
    role: World mutations happen here, not earlier. AI proposes; engine disposes.
```

### 2.2 AIArbiter cycle (per-frame, GOAP + async LLM hybrid)

```text
[AIArbiter::update(snap) — called every frame]
    file: astraweave-ai/src/ai_arbiter.rs:384-…
    │
    ▼
[Poll for LLM completion (non-blocking)]
    file: ai_arbiter.rs:394-408
    role: poll_llm_result() checks if current_llm_task is ready.
          On success: transition_to_llm(plan); on failure: warn + llm_failures += 1.
    │
    ▼
[Dispatch on current mode]
    ├─ AIControlMode::GOAP (default):
    │     ai_arbiter.rs:411-430
    │     1. maybe_request_llm(snap): spawn async LLM task if cooldown elapsed (default 15.0s)
    │     2. goap_actions += 1
    │     3. Return goap.propose_plan(snap).steps.first() (5-30 µs target)
    │     4. If GOAP plan has 0 steps: transition_to_bt() → return bt.propose_plan(snap).steps.first()
    │     5. Ultimate fallback: ActionStep::Wait { duration: 1.0 }
    │
    ├─ AIControlMode::ExecutingLLM { step_index }:
    │     ai_arbiter.rs:432-473
    │     1. If step_index < current_plan.steps.len(): return plan.steps[step_index]
    │     2. Advance step_index; if >= len: transition_to_goap()
    │     3. If invalid step or no plan: warn + transition_to_goap + recurse
    │
    └─ AIControlMode::BehaviorTree:
          ai_arbiter.rs:475-483
          Return bt.propose_plan(snap).steps.first() or ActionStep::Wait { duration: 1.0 }
    │
    ▼
[Record metrics + return ActionStep]
    file: ai_arbiter.rs:486-489 (histogram "ai.arbiter.update_latency" in ms)
    role: Action is returned EVERY frame, target <100 µs (actual: 5-30 µs GOAP)
```

### 2.3 LLM 4-tier fallback chain (`FallbackOrchestrator`)

```text
[FallbackOrchestrator::plan(snap, budget_ms) — astraweave-llm/src/fallback_system.rs]
    │
    ▼
[Tier 1: FullLlm — all 37 tools, detailed prompts]
    files: prompts.rs (PromptBuilder::add_snapshot/add_goal/build),
           prompt_template.rs (build_enhanced_prompt with PromptConfig),
           plan_parser.rs (parse_llm_response with ExtractionMethod fallback chain)
    success? → return PlanSource::Llm(plan)
    │
    │ on parse failure / circuit-breaker trip / timeout
    ▼
[Tier 2: SimplifiedLlm — 10 most common tools, compressed prompts]
    files: compression.rs (PromptCompressor — token reduction)
    success? → return PlanSource::Llm(plan) with reduced tool set
    │
    │ on continued failure
    ▼
[Tier 3: Heuristic — rule-based planning, no LLM]
    file: heuristics.rs (HeuristicConfig with HeuristicRule { condition, action })
    role: Default rules include LowMorale→HealSelf, LowAmmo→Reload,
          EnemyNearby→AttackNearestEnemy, EnemyVisible→TakeCover, etc.
          (heuristics.rs:13-40+)
    success? → return PlanSource::Fallback { plan, reason: "heuristic" }
    │
    │ on heuristic exhaustion
    ▼
[Tier 4: Emergency — safe default: Scan + Wait]
    role: Last-resort no-op. Always returns a non-empty plan.
```

### 2.4 Production hardening wrap (`ProductionHardeningLayer`)

```text
[Incoming LLM request — astraweave-llm/src/production_hardening.rs]
    │
    ▼
[RateLimiter (rate_limiter.rs)]
    role: Per-context (system/agent/priority) request rate enforcement
    │
    ▼
[CircuitBreaker (circuit_breaker.rs)]
    role: Trip open after threshold of consecutive failures; half-open probe; close on success
    │
    ▼
[BackpressureManager (backpressure.rs)]
    role: Priority-tagged request queue; drop or defer low-priority under load
    │
    ▼
[ABTestFramework (ab_testing.rs)]
    role: Route requests through experimental variants; record outcomes
    │
    ▼
[LlmClient::complete / complete_streaming (lib.rs:84-…)]
    impls: Phi3, Hermes2Pro Ollama, Phi3 Ollama, Qwen3 Ollama (feature-gated)
    │
    ▼
[Telemetry (telemetry.rs)]
    role: Latency + cost + token-count observability; HealthChecker background task
```

### 2.5 Behavior Tree tick (when `PlannerMode::BehaviorTree` or `AIControlMode::BehaviorTree`)

```text
[behavior_tick_system(world) — astraweave-behavior/src/ecs.rs:16]
    │
    │ for each entity with CBehaviorGraph:
    ▼
[BehaviorGraph::tick(&context) → BehaviorStatus]
    file: astraweave-behavior/src/lib.rs:370 (BehaviorGraph), :19 (BehaviorNode)
    nodes: Sequence (AND), Selector (OR), Action(String), Condition(String),
           Decorator(DecoratorType, Box<BehaviorNode>), Parallel(children, success_threshold)
    decorators: Inverter, Succeeder, Failer, Repeat(u32), Retry(u32)
    │
    ▼
[Update CBehaviorGraph.status + running_node in ECS]
    file: astraweave-behavior/src/ecs.rs:29-34
```

### 2.6 GOAP plan (when `PlannerMode::GOAP` or `AIControlMode::GOAP`)

```text
[GoapPlanner::plan(&start, &goal, &actions) — astraweave-behavior/src/goap.rs]
    role: A* over symbolic WorldState (BTreeMap<u32, bool> via interner)
    │
    ▼
[Cache lookup — astraweave-behavior/src/goap_cache.rs]
    role: LRU cache keyed on (start_state, goal_state, action_set_hash)
    cache hit: 1.01 µs; cache miss: 47.2 µs (per astraweave-ai/src/lib.rs:24)
    │
    ▼
[Plan = Vec<GoapAction> → mapped to PlanIntent steps]
    role: Caller maps each GoapAction.name to an ActionStep (domain-specific)
```

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **`WorldSnapshot`** | Canonical filtered world view for AI consumption. Fields: `t`, `player`, `me`, `enemies`, `pois`, `obstacles`, `objective`. **Field names are hard-coded in LLM prompts** — see `docs/architecture/ecs_math_core_sdk_foundation.md` Invariant 8. | `astraweave-core/src/schema.rs:270` |
| **`PlanIntent`** | Canonical AI output: `{ plan_id: String, steps: Vec<ActionStep> }`. | `astraweave-core/src/schema.rs` |
| **`ActionStep`** | Validated action verb (`MoveTo`, `Throw`, `CoverFire`, `Revive`, `Interact`, etc.). | `astraweave-core/src/schema.rs` |
| **`Orchestrator`** trait | Sync orchestrator interface: `propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent`. | `astraweave-ai/src/orchestrator.rs:22-25` |
| **`OrchestratorAsync`** trait | Async orchestrator interface: `async fn plan(&self, snap, budget_ms) -> Result<PlanIntent>`. | `astraweave-ai/src/orchestrator.rs:14-20` |
| **`RuleOrchestrator`** | Minimal hand-coded rule-based orchestrator: smoke + advance toward nearest enemy. Always available (no feature gate). | `astraweave-ai/src/orchestrator.rs:53-…` |
| **`AIArbiter`** | The hybrid GOAP+LLM controller. Owns a strategic LLM executor, optional fast LLM executor, GOAP orchestrator, BT orchestrator, mode state, and metrics. Not `Send`/`Sync` — single-threaded game loop usage. | `astraweave-ai/src/ai_arbiter.rs:200-252` |
| **`AIControlMode`** | Discriminated enum tracking the Arbiter's current state: `GOAP` (default), `ExecutingLLM { step_index: usize }`, `BehaviorTree`. `#[non_exhaustive]`. | `astraweave-ai/src/ai_arbiter.rs:91-109` |
| **`PlannerMode`** | Dispatch enum on `CAiController`: `Rule`, `BehaviorTree`, `GOAP`. `#[non_exhaustive]`. Distinct from `AIControlMode` (which is arbiter-internal). | `astraweave-ai/src/core_loop.rs:23-32` |
| **`CAiController`** | ECS component carrying `PlannerMode` + optional policy string. | `astraweave-ai/src/core_loop.rs:96-102` |
| **`LlmExecutor`** | Wraps `Arc<dyn OrchestratorAsync>` and a tokio `Handle`. `generate_plan_async(snapshot) -> AsyncTask<Result<PlanIntent>>` spawns blocking work onto the runtime. | `astraweave-ai/src/llm_executor.rs:76-…` |
| **`AsyncTask<T>`** | Non-blocking wrapper around `tokio::task::JoinHandle<T>`. `try_recv()` polls without blocking; timeout supported. Drops abort the underlying task. | `astraweave-ai/src/async_task.rs:52-…` |
| **`Orchestrator` trait object in `AIArbiter`** | `Box<dyn Orchestrator>` for both GOAP and BT slots. Allows pluggable implementations. | `ai_arbiter.rs:212-215` |
| **`ToolVerb`** | Sandbox-side enum of 10 validated action verbs: `MoveTo`, `Throw`, `CoverFire`, `Revive`, `Interact`, `UseItem`, `Stay`, `Wander`, `Hide`, `Rally`. `#[non_exhaustive]`. | `astraweave-ai/src/tool_sandbox.rs:14-25` |
| **`ValidationCategory`** | Sandbox-side validation taxonomy: `Nav`, `Visibility`, `Resources`, `Physics`, `Cooldown`. | `astraweave-ai/src/tool_sandbox.rs` |
| **`validate_and_execute`** | The engine-side tool sandbox. Takes `(&mut World, actor: Entity, intent: &PlanIntent, cfg: &ValidateCfg, log: &mut impl FnMut(String))`. Returns `Result<(), EngineError>`. Applies world mutations only if all checks pass. | `astraweave-core/src/validation.rs:10-…` |
| **`EngineError`** | Validation failure variants: `InvalidAction`, `NoPath`, `LosBlocked`, `Cooldown(String)`. | `astraweave-core` (referenced from `validation.rs:33, 169, 295, 299`) |
| **`BehaviorNode`** | `#[non_exhaustive]` enum: `Sequence(Vec<…>)`, `Selector(Vec<…>)`, `Action(String)`, `Condition(String)`, `Decorator(DecoratorType, Box<…>)`, `Parallel(Vec<…>, success_threshold: usize)`. | `astraweave-behavior/src/lib.rs:19-26` |
| **`DecoratorType`** | `Inverter`, `Succeeder`, `Failer`, `Repeat(u32)`, `Retry(u32)`. | `astraweave-behavior/src/lib.rs:286-292` |
| **`BehaviorStatus`** | Tick result. `#[non_exhaustive]` enum with exactly 3 variants: `Success`, `Failure`, `Running` (verified 2026-05-12 at `astraweave-behavior/src/lib.rs:1479-1485`). | `astraweave-behavior/src/lib.rs:1479-1485` |
| **`BehaviorContext`** | Mutable per-tick context carrying registered action/condition callbacks. | `astraweave-behavior/src/lib.rs` |
| **`CBehaviorGraph`** | ECS component for entities with behavior graphs: `{ graph: BehaviorGraph, context: BehaviorContext, status: BehaviorStatus, running_node: Option<String> }`. | `astraweave-behavior/src/ecs.rs:9-14` |
| **`WorldState`** (GOAP) | Symbolic facts as `BTreeMap<u32, bool>` (interned keys for determinism). `satisfies(&other)` checks coverage; `apply(&effects)` mutates. `distance_to(&goal)` is the A* heuristic. | `astraweave-behavior/src/goap.rs:14-65` |
| **`GoapAction`** | `{ name, cost: f32, preconditions: WorldState, effects: WorldState }`. | `astraweave-behavior/src/goap.rs:74-80` |
| **`GoapGoal`** | Symbolic goal state. | `astraweave-behavior/src/goap.rs` |
| **`GoapPlanner`** | A* planner over symbolic states. `with_max_iterations(n)` configurable bound. | `astraweave-behavior/src/goap.rs` |
| **Advanced GOAP `Action` / `Goal` / `Planner`** | Parallel hierarchy in `astraweave-ai/src/goap/` (feature `planner_advanced`) — supports learning, persistence, shadow mode, plan analysis, plan stitching, visualization, goal authoring, scheduling, validation. Distinct types from `astraweave-behavior::goap`. | `astraweave-ai/src/goap/{action,goal,planner}.rs` |
| **`LlmClient`** trait | Core LLM interface: `async fn complete(&self, prompt: &str) -> Result<String>` and `async fn complete_streaming(...)`. Default streaming impl wraps blocking `complete`. | `astraweave-llm/src/lib.rs:84-…` |
| **`PlanSource`** | `Llm(PlanIntent)` or `Fallback { plan: PlanIntent, reason: String }`. `#[non_exhaustive]`. | `astraweave-llm/src/lib.rs:76-81` |
| **`FallbackTier`** | `FullLlm` (1) → `SimplifiedLlm` (2) → `Heuristic` (3) → `Emergency` (4). Tier-1 has all 37 tools; Tier-2 has 10 common; Tier-3 is rule-based; Tier-4 is safe-default Scan+Wait. | `astraweave-llm/src/fallback_system.rs:27-53` |
| **`FallbackOrchestrator`** | The 4-tier degradation engine. Composes `BatchInferenceExecutor`, `CircuitBreakerManager`, `PromptCompressor`, `HeuristicConfig`, `LlmClient`. | `astraweave-llm/src/fallback_system.rs` |
| **`ExtractionMethod`** | Plan-parser's 5-strategy LLM-response decoder: `Direct`, `CodeFence` (```json blocks), `Envelope` (message.content / response fields), `ObjectExtraction` (regex), `Tolerant` (key normalization). | `astraweave-llm/src/plan_parser.rs:25-44` |
| **`ToolGuard`** | LLM-output-side action validator with per-tool `ToolPolicy` (verified 2026-05-12 at `astraweave-llm/src/tool_guard.rs:36-45`: `#[non_exhaustive]` enum with exactly 3 variants: `Allowed`, `Restricted`, `Denied`). Allowlist/denylist, world-state predicates, resource limits, audit logging. | `astraweave-llm/src/tool_guard.rs:36-45` |
| **`ProductionHardeningLayer`** | Composite reliability layer: rate limiter + circuit breaker + backpressure manager + A/B testing + LLM telemetry + health checker + background shutdown signal. | `astraweave-llm/src/production_hardening.rs:17-38` |
| **`PromptCache`** (feature `llm_cache`) | Global lazily-initialized LRU prompt cache. Capacity from `LLM_CACHE_CAP` env var (default 4096); similarity threshold from `LLM_CACHE_SIM_THRESH` (default 1.0 = exact match only). | `astraweave-llm/src/lib.rs:50-67`, `astraweave-llm/src/cache/` |
| **`FallbackResult`** / **`FallbackAttempt`** | Outcome record carrying the final plan, achieved tier, and per-attempt error/tier traces. | `astraweave-llm/src/fallback_system.rs:55+` |
| **`VeilweaverCompanionOrchestrator`** | Feature-gated (`veilweaver_slice`) GOAP-based companion AI tailored to Veilweaver gameplay verbs (`stability_pulse`, `heal_player`, `execute_combo`, `mark_target`, `reposition`). Wraps `GoapPlanner`. | `astraweave-ai/src/veilweaver.rs:19-23` |
| **`BossDirector`** | Heuristic boss-AI planner producing `DirectorPlan` (terrain edits, spawns, etc.) under a `DirectorBudget`. | `astraweave-director/src/lib.rs:7+` |
| **`AiPlannedEvent` / `AiPlanningFailedEvent`** | ECS events emitted by `AiPlanningPlugin::sys_ai_planning` to surface plan production and failures. | `astraweave-core/src/ecs_events.rs` (re-imported by `ecs_ai_plugin.rs:2`) |
| **`AiPlanningPlugin`** | ECS plugin: `build_app_with_ai()` and the `sys_ai_planning` system registered in stage `"ai_planning"`. | `astraweave-ai/src/ecs_ai_plugin.rs:16, 45-…` |
| **`EntityBridge`** | Maps legacy `astraweave_core::Entity` (u32) ↔ ECS `astraweave_ecs::Entity` (generational). Used by `sys_ai_planning` to map back when applying decisions. See `docs/architecture/ecs_math_core_sdk_foundation.md` §3. | `astraweave-core/src/ecs_bridge.rs` |

### Terms to NOT confuse

- **`PlannerMode` vs `AIControlMode`**: `PlannerMode` (`core_loop.rs:23-32`) is the dispatch enum stored on `CAiController` per entity — three variants (`Rule`, `BehaviorTree`, `GOAP`). `AIControlMode` (`ai_arbiter.rs:91-109`) is the Arbiter's internal state — also three variants (`GOAP`, `ExecutingLLM { step_index }`, `BehaviorTree`). They overlap conceptually but live at different layers: dispatch vs runtime-state. They are **not interchangeable**.
- **`astraweave_behavior::goap::*` vs `astraweave_ai::goap::*`**: Two GOAP implementations. The canonical engine GOAP is in `astraweave-behavior` (`WorldState` as `BTreeMap<u32, bool>`, simple A*). The advanced GOAP is in `astraweave-ai/src/goap/` behind `planner_advanced` (its own `Action`, `Goal`, `WorldState`, plus learning, persistence, shadow_mode, plan_analyzer, plan_stitcher, plan_visualizer, goal_authoring, goal_scheduler, goal_validator). They share **no Rust types** in this pass — code that switches between them needs explicit conversion. See §6.
- **`tool_sandbox` (astraweave-ai) vs `tool_guard` (astraweave-llm) vs `validate_and_execute` (astraweave-core)**: THREE validation/guard layers. `tool_sandbox` defines the canonical `ToolVerb` taxonomy. `tool_guard` is LLM-output-side allowlist/policy filtering. `validate_and_execute` is the engine-side gate that actually mutates `World`. They serve different purposes; calls to `validate_and_execute` are the only ones that change game state.
- **`Orchestrator` (sync) vs `OrchestratorAsync` (async)**: Different traits. `Orchestrator::propose_plan` is sync (used by `RuleOrchestrator`, GOAP path, BT path, and the `goap`/`bt` slots in `AIArbiter`). `OrchestratorAsync::plan` is async + budget-aware (used by `LlmExecutor` to wrap LLM clients).
- **"GOAP+Qwen3 Hybrid" vs "GOAP+Hermes Hybrid"**: Both names refer to the same `AIArbiter`. CLAUDE.md uses "Qwen3" (current model); older docs (`ARCHITECTURE_REFERENCE.md:41`) say "Hermes" (former model). The arbiter implementation is model-agnostic — it talks to `LlmExecutor` which holds any `Arc<dyn OrchestratorAsync>`. The specific LLM client (Qwen3Ollama / Hermes2ProOllama / Phi3 / Phi3Ollama) is chosen at construction time.
- **`PromptCache` similarity matching is opt-in**: `astraweave-llm/src/lib.rs:57-65` documents that the default similarity threshold is 1.0 (exact-match only). Similarity matching is "nondeterministic across prompt variants and can cause unexpected cross-test pollution when tests run in parallel"; set `LLM_CACHE_SIM_THRESH` to a value <1.0 to enable.
- **`PlanSource::Fallback` vs `PlanSource::Llm`**: Distinct variants. Fallback carries `reason: String`; LLM is bare. Callers must pattern-match — they cannot ignore the source.
- **`AnimationState` parallels here**: Just as the animation system has 4 parallel skeleton type families (`docs/architecture/animation.md` §6), the AI system has multiple parallel state-machine types: `AIControlMode`, `PlannerMode`, `FallbackTier`, `ExtractionMethod`, `ToolPolicy`. Each serves a different layer; do not conflate.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-core` | `WorldSnapshot`, `PlayerState`, `CompanionState`, `EnemyState`, `Poi`, `IVec2`, `Stance`, `CoverType` | Canonical perception schema | Defined at `astraweave-core/src/schema.rs:270` (`WorldSnapshot`). Built per-tick from legacy `World` via `astraweave_core::perception::build_snapshot` |
| `astraweave-ecs` | `World`, `Entity`, `Query`, `Resource`, schedule stage `"ai_planning"` | ECS substrate that hosts AI systems | The `ai_planning` stage is registered in the canonical eight-stage list at `astraweave-ecs/src/lib.rs:783-792`. `sys_ai_planning` registers there |
| `astraweave-core::EntityBridge` | `bridge.get(legacy_id) -> Option<ecs::Entity>` | Legacy→ECS entity ID translation | `sys_ai_planning` consults the bridge when applying plans (`ecs_ai_plugin.rs:38-42`) |
| `astraweave-nav` | `NavMesh`, `NavTri`, `Triangle`, A* paths | Pathfinding for `MoveTo` validation | `tool_sandbox.rs:8` `use astraweave_nav::NavMesh` |
| `rapier3d` | Physics queries | Used by tool sandbox for physics-related validation | `tool_sandbox.rs:9` `use rapier3d::prelude::*` |
| `astraweave-observability` | `log_companion_action`, `CompanionActionEvent`, `LlmTelemetry`, `TelemetryConfig` | Structured logging + LLM telemetry | Used by `VeilweaverCompanionOrchestrator` (`veilweaver.rs:7`) and `ProductionHardeningLayer` (`production_hardening.rs:13`) |
| External LLM endpoints | Ollama HTTP API (Qwen3 / Hermes2Pro / Phi3 ports), local Candle inference | Plan generation | Clients in `astraweave-llm/src/{qwen3_ollama,hermes2pro_ollama,phi3_ollama,phi3}.rs` (feature-gated `ollama` / `phi3`) |
| `tokio` runtime | Async task spawn, blocking pool | Background LLM inference | `LlmExecutor::new(orchestrator, runtime: Handle)` (`llm_executor.rs:80`); `AsyncTask` wraps `JoinHandle` (`async_task.rs:54`) |
| `serde_json` | JSON parse/serialize | LLM response decoding + capture/replay | Used throughout `plan_parser.rs`, `prompts.rs`, `compression.rs` |

### Downstream (what consumes this system's output)

| Consumer | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-core::validation::validate_and_execute` | `validate_and_execute(&mut World, actor, &PlanIntent, &ValidateCfg, &mut impl FnMut(String))` | `PlanIntent` from AI → `Result<(), EngineError>` | THE engine-side gate. `astraweave-core/src/validation.rs:10-…` |
| `astraweave-sdk` | C ABI `aw_world_submit_intent_json(handle, actor_id, intent_json, callback)` | Plans submitted from out-of-tree hosts | Calls `validate_and_execute` internally. `astraweave-sdk/src/lib.rs:382-…` |
| `examples/hello_companion`, `ecs_ai_showcase`, `phi3_demo`, `llm_integration` | `Orchestrator::propose_plan`, `AIArbiter::update`, `LlmExecutor::generate_plan_async` | Demo scripts that exercise the AI loop | Examples are the primary in-tree runtime consumers |
| `astraweave-director` (`BossDirector`, `LlmDirector`) | `Director::plan(&snap, &budget) -> DirectorPlan` | Plays a different role: writes to terrain / spawn / camera | Director is "boss AI" — separate from companion AI. `astraweave-director/src/lib.rs:7+` |
| `astraweave-coordination` | `Agent` trait (`agent.rs:13-58` — 15 required methods including `handle_message`, `execute_task`, `handle_world_event`); `AgentCoordinator` struct (`coordination.rs:16-31`) with sub-components `MessageRouter`, `ResourceManager`, `EventDispatcher`; `WorldEventGenerator` (`world_events.rs:20-30`); `NarrativeCoherenceEngine` (`narrative_coherence.rs:18-29`) | Multi-agent orchestration designed for cross-NPC/Director/Dialogue/Quest coordination | Verified 2026-05-12: **zero workspace consumers** — `astraweave-coordination` has no `use astraweave_coordination` references in any other crate, example, or tool. Full subsystem detail in §13.5 |
| `astraweave-npc` | `NpcManager::update(dt, glue: &mut dyn CommandSink, views: &HashMap<NpcId, NpcWorldView>)` + `NpcManager::handle_player_utterance(npc_id, view, utter)`; planning via the `LlmAdapter` trait (default impl is the heuristic `MockLlm`) | Per-NPC AI runtime — produces `NpcAction`s and applies them through a `CommandSink` (default `EngineCommandSink` wraps `PhysicsWorld` + `AudioEngine`) | `astraweave-npc/src/runtime.rs:66-207`. Verified 2026-05-12: no `Npc::tick` method exists; the original framing was inaccurate. Full vocabulary and isolation status detailed in §13.3 |
| `astraweave-dialogue` | Basic path: `DialogueRunner::start(start_node_id)` + `DialogueRunner::choose(choice_index)` emitting `DialogueEvent::{NodeEntered, ChoiceMade, Ended}`; `astraweave-dialogue::toml_loader::load_dialogue_from_toml -> LoadedDialogue`. LLM path: `LlmDialogueSystem` (2941 LoC, dormant). | Dialogue node execution and TOML authoring | `astraweave-dialogue/src/runner.rs:81-…` (basic); `llm_dialogue.rs:31-49` (LLM, no external consumers). Verified 2026-05-12: prior framing "`DialogueRunner::advance(...)`, `LlmDialogue`" was inaccurate — corrected. Full subsystem detail in §13.4 |
| `astraweave-memory` | `EpisodeRecorder::record(observation, action, outcome)` | Captures interaction history for learning | Designed to feed back into AI via `PatternDetector` / `PreferenceProfile` / `AdaptiveWeightManager`, but **the runtime feedback loop is not wired** as of 2026-05-12 — see §13.1.5 and §11 Open Question on memory dormancy |
| `tools/aw_editor` | `Orchestrator::propose_plan` invocation through gameplay-preset panel | Editor-side AI behavior preview | Per workspace grep |
| `astraweave-ipc` | IPC bridge for external orchestrators | `WorldSnapshot` → external process → `PlanIntent` | Mentioned in CLAUDE.md ARCHITECTURE_REFERENCE.md as a planned IPC integration (gRPC/WebSocket per the `GameAdapter` trait comment) |
| `astraweave-llm-eval` | Evaluation harness for LLM outputs | Per workspace grep |

### Bidirectional / Coupled

- **`AIArbiter` ↔ tokio runtime**: The arbiter holds a `tokio::runtime::Handle` (via `LlmExecutor`) for background LLM inference. Drops on `AsyncTask` abort the underlying task (`async_task.rs:54` comment). The arbiter is **not `Send`/`Sync`** (`ai_arbiter.rs:194-195`) — designed for single-threaded game-loop usage; cross-thread sharing requires external synchronization.
- **`FallbackOrchestrator` ↔ `CircuitBreakerManager`**: The circuit breaker can short-circuit Tier-1 calls when failure rate exceeds threshold, forcing immediate fallback to Tier-2. The breaker is also consulted by `ProductionHardeningLayer`. See `fallback_system.rs:16-17` (`circuit_breaker_execute` macro).
- **`PromptCache` (global) ↔ all LLM call sites**: When `llm_cache` feature is enabled (default), the global static `GLOBAL_CACHE` (`lib.rs:50-67`) is consulted by any code path that goes through cached prompt resolution. `clear_global_cache()` is provided for tests (`lib.rs:71-73`).
- **`AIArbiter::update` ↔ `metrics` (astraweave-core)**: Every update emits `ai.arbiter.update_latency` histogram and tracks `mode_transitions`, `llm_requests`, `llm_successes`, `llm_failures`, `goap_actions`, `llm_steps_executed` (`ai_arbiter.rs:236-251, 486-489`).
- **`EpisodeRecorder` (memory) ↔ AI orchestrators**: Recorder captures (observation, action, outcome) tuples. These feed pattern detection and preference profiling, which in turn can adjust `AdaptiveWeightManager` for behavior trees. **Verified 2026-05-12 (§13.1.5): the feedback loop is NOT wired into the runtime**. `astraweave-ai` and `astraweave-behavior` have zero imports of `astraweave_memory`. The chain exists structurally inside the memory crate but no production caller invokes it. See §13.1 for the full subsystem trace.
- **`astraweave-ai::ecs_ai_plugin::sys_ai_planning` ↔ legacy `World`**: The planning system reads from the legacy World resource (`build_snapshot`) and writes `CDesiredPos` ECS components + emits `AiPlannedEvent` / `AiPlanningFailedEvent` (`ecs_ai_plugin.rs:1-80`). The dual-World coupling is documented in `docs/architecture/ecs_math_core_sdk_foundation.md` §6.

---

## 5. Active File Map

This section enumerates the load-bearing files. Per-file enumeration of all 112 files is out of scope; per-subsystem traces (Memory, Director, NPC, Dialogue, Coordination) are explicit follow-up work.

### `astraweave-ai` — orchestration core

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-ai/src/lib.rs` | Module declarations + re-exports; performance summary at lines 22-27 | Active | 70 LoC |
| `astraweave-ai/src/core_loop.rs` | `PlannerMode`, `CAiController`, `dispatch_planner`; the canonical loop dispatcher | Active | Re-exported via `lib.rs:54` |
| `astraweave-ai/src/orchestrator.rs` | `Orchestrator` trait, `OrchestratorAsync` trait, `RuleOrchestrator` | Active | The trait surface is the single contract every planner implements |
| `astraweave-ai/src/tool_sandbox.rs` | `ToolVerb` enum (10 verbs), `ValidationCategory`, sandbox helpers | Active | Re-exported via `lib.rs:57` |
| `astraweave-ai/src/ecs_ai_plugin.rs` | `AiPlanningPlugin`, `build_app_with_ai()`, `sys_ai_planning` system | Active | Registers in stage `"ai_planning"` |
| `astraweave-ai/src/ai_arbiter.rs` | `AIArbiter`, `AIControlMode`; the hybrid GOAP+LLM controller | Active (feature `llm_orchestrator`) | Lines 1-46 contain the canonical ASCII architecture diagram |
| `astraweave-ai/src/llm_executor.rs` | `LlmExecutor` (`generate_plan_async`, `generate_plan_sync`) | Active (feature `llm_orchestrator`) | Wraps `Arc<dyn OrchestratorAsync>` + tokio `Handle` |
| `astraweave-ai/src/async_task.rs` | `AsyncTask<T>` non-blocking poll wrapper around `JoinHandle<T>` | Active (feature `llm_orchestrator`) | Drop aborts the underlying task |
| `astraweave-ai/src/veilweaver.rs` | `VeilweaverCompanionOrchestrator` (Veilweaver-specific GOAP) | Active (feature `veilweaver_slice`) | Has its own action vocabulary (`stability_pulse`, `heal_player`, etc.) |
| `astraweave-ai/src/goap/` | Advanced GOAP submodule (22 files: actions, goals, planner, learning, persistence, shadow_mode, plan_analyzer, plan_stitcher, plan_visualizer, goal_authoring, goal_scheduler, goal_validator, telemetry, persistence, history, debug_tools, adapter, config, state, …) | Active (feature `planner_advanced`) | Per-file detail deferred to a dedicated follow-up trace |
| `astraweave-ai/src/persona/{mod,manager}.rs` | `LlmPersonaManager`, `PersonaConfig` | Active | Persona-level LLM configuration |
| `astraweave-ai/src/rag/{mod,pipeline}.rs` | `RagPipeline`, `RagConfig`, `RagDocument`, consolidation/forgetting/injection strategies | Active (feature `rag`) | Retrieval-augmented generation pipeline |
| `astraweave-ai/src/mutation_tests.rs` | Mutation-killing inline tests | Active (test gate) | |
| `astraweave-ai/tests/*.rs` | 29 integration test files including `arbiter_*`, `goap_*`, `core_loop_*`, `orchestrator_*`, `tool_*`, `llm_fallback`, `determinism_tests`, `stress_tests`, `edge_case_tests`, `nan_infinity_tests`, `cross_module_integration`, `behavioral_correctness_tests`, `rag_integration_test`, … | Active | See §10 |
| `astraweave-ai/benches/*` | 9 bench files: `ai_core_loop`, `goap_bench`, `arbiter_bench`, `ai_benchmarks`, `integration_pipeline`, `multi_agent_pipeline`, `goap_performance_bench`, `goap_vs_rule_bench`, `alloc_measure` | Active | `alloc_measure` requires `alloc-counter` + `planner_advanced` features (`Cargo.toml:67-69`) |

### `astraweave-behavior` — Behavior Trees + canonical GOAP

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-behavior/src/lib.rs` | `BehaviorNode`, `BehaviorGraph`, `DecoratorType`, `BehaviorContext`, `BehaviorStatus` | Active | `#![forbid(unsafe_code)]` at line 1; `#[non_exhaustive]` on `BehaviorNode` and `DecoratorType` |
| `astraweave-behavior/src/goap.rs` | Canonical engine GOAP: `WorldState`, `GoapAction`, `GoapGoal`, `GoapPlanner` | Active | `WorldState` uses `BTreeMap<u32, bool>` with interned keys for determinism |
| `astraweave-behavior/src/goap_cache.rs` | LRU cache for GOAP plans (Week 3 Action 9) | Active | Powers the 1.01 µs cache-hit performance |
| `astraweave-behavior/src/interner.rs` | String → u32 interner for GOAP fact keys | Active | Determinism aid |
| `astraweave-behavior/src/ecs.rs` | `CBehaviorGraph`, `behavior_tick_system`, `BehaviorPlugin` | Active | Plugin registers `behavior_tick_system` in stage `"simulation"` |
| `astraweave-behavior/src/mutation_tests.rs` | Mutation-killing inline tests | Active (test gate) | |
| `astraweave-behavior/tests/*` | 4 integration test files: `behavior`, `fuzz_planner`, `mutation_resistant_comprehensive_tests`, `mutation_resistant_tests` | Active | |

### `astraweave-llm` — LLM integration

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-llm/src/lib.rs` | `LlmClient` trait + global `PromptCache` + `PlanSource` enum | Active | `#![forbid(unsafe_code)]` at line 1 |
| `astraweave-llm/src/llm_adapter.rs` | `safe_llm_invoke`, prompt length/format validators (`MAX_PROMPT_LENGTH = 4096`) | **Dormant stub** | Verified 2026-05-12: zero workspace callers of `safe_llm_invoke`. File comment line 1 says "stub". `mock_llm_call` is a placeholder. The 4096-char limit is unreachable code. See §13.7.4 + §13.7.7 |
| `astraweave-llm/src/plan_parser.rs` | `parse_llm_response`, `ExtractionMethod`, `ParseResult` | Active | 5-strategy extraction chain |
| `astraweave-llm/src/fallback_system.rs` | `FallbackOrchestrator`, `FallbackTier`, `FallbackResult`, `FallbackAttempt` | Active | 4-tier degradation |
| `astraweave-llm/src/heuristics.rs` | `HeuristicConfig`, `HeuristicRule`, `HeuristicCondition`, `HeuristicAction` | Active | Tier-3 rule engine |
| `astraweave-llm/src/tool_guard.rs` | `ToolGuard`, `ToolPolicy` | Active | LLM-output-side action validator |
| `astraweave-llm/src/production_hardening.rs` | `ProductionHardeningLayer`, `HardeningConfig`, `HealthChecker` | **Dormant** | Composite reliability layer. Verified 2026-05-12: zero non-example constructors. See §13.7 |
| `astraweave-llm/src/prompts.rs` | `PromptBuilder`, system-prompt templates (`TACTICAL_AI`, …), JSON snapshot serializer | Active | The serializer translates Rust field names to LLM JSON keys |
| `astraweave-llm/src/prompt_template.rs` | `build_enhanced_prompt`, `PromptConfig`, few-shot example library | Active | |
| `astraweave-llm/src/compression.rs` | `PromptCompressor` — token reduction | Active | Tier-2 compressed prompts |
| `astraweave-llm/src/few_shot.rs` | Few-shot prompt examples | Active | |
| `astraweave-llm/src/streaming_parser.rs` | Streaming response parser | Active | For `complete_streaming` |
| `astraweave-llm/src/scheduler.rs` | Request scheduler | Active | |
| `astraweave-llm/src/batch_executor.rs` | `BatchInferenceExecutor`, `AgentId` | Active | Multi-agent batched inference |
| `astraweave-llm/src/rate_limiter.rs` | `RateLimiter`, `RateLimiterConfig`, `RateLimitContext`, `RequestPriority` | Active | |
| `astraweave-llm/src/circuit_breaker.rs` | `CircuitBreakerManager`, `CircuitBreakerConfig` + `circuit_breaker_execute!` macro | Active | |
| `astraweave-llm/src/backpressure.rs` | `BackpressureManager`, `BackpressureConfig`, `Priority`, `RequestMetadata` | Active | |
| `astraweave-llm/src/ab_testing.rs` | `ABTestFramework`, `ABTestConfig` | Active | Experiment routing |
| `astraweave-llm/src/retry.rs` | Retry policies | Active | |
| `astraweave-llm/src/telemetry.rs` | LLM telemetry | Active | |
| `astraweave-llm/src/schema.rs` | LLM-side schema utilities | Active | |
| `astraweave-llm/src/cache/` | `PromptCache`, `CachedPlan`, `PromptKey` | Active (feature `llm_cache`, default on) | Global `LazyLock` cache at `lib.rs:50-67` |
| `astraweave-llm/src/phi3.rs` | Local Phi-3 inference via Candle | Active (feature `phi3`) | Optional — heavy GPU/CPU deps |
| `astraweave-llm/src/{phi3_ollama,hermes2pro_ollama,qwen3_ollama}.rs` | Ollama HTTP clients | Active (feature `ollama`) | Three model adapters |
| `astraweave-llm/tests/*` | 15 integration test files including `fallback_chain_integration`, `phase7_integration_tests`, `concurrent_stress_tests`, `timeout_retry_tests`, `boundary_condition_tests`, `property_tests`, `mutation_kill_tests`, … | Active | |
| Per-subsystem trace | See §13.7 below for the dedicated LLM Production Hardening subsystem trace | Done | |

### `astraweave-memory` — companion memory + learning

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-memory/src/lib.rs` | Module declarations + re-exports (15 pub-uses + feature-gated `components`) | Active | `#![forbid(unsafe_code)]` at line 1 |
| `astraweave-memory/src/{memory_types,memory_manager}.rs` | Hierarchical memory (sensory/working/episodic/semantic + procedural/emotional/social), forgetting curves; `MemoryManager` with `MemoryManagerConfig` capacity-per-type limits | Active | See §13.1.4 |
| `astraweave-memory/src/{episode,episode_recorder}.rs` | `GameEpisode`, `EpisodeCategory`, `EpisodeOutcome`, `PlayerAction`, `CompanionResponse`, `Observation`, `EpisodeRecorder` | Active | Note: `Episode` from `episode` is different from `persona::Episode` (renamed `GameEpisode` at re-export) |
| `astraweave-memory/src/storage.rs` | SQLite `MemoryStorage` + `StorageStats`; schema at `storage.rs:46-80` with `memories` + `memory_tags` + `metadata` tables and four indexes | Active | Powered by `rusqlite 0.37` (`Cargo.toml:22`, bundled feature) |
| `astraweave-memory/src/pattern_detection.rs` | `PatternDetector`, `ActionPattern`, `PatternStrength`, `PlaystylePattern` (6 variants: Aggressive/Cautious/Explorative/Social/Analytical/Efficient) | Active | Behavioral pattern detection |
| `astraweave-memory/src/preference_profile.rs` | `PreferenceProfile`, `ProfileBuilder`, `CompanionActionPreference` | Active | |
| `astraweave-memory/src/dynamic_weighting.rs` | `AdaptiveWeightManager`, `BehaviorNodeType` (6 variants), `NodeWeight` | Active (Phase 4) | Adaptive behavior tree weighting. **No production consumers found** — see §13.1.7 |
| `astraweave-memory/src/learned_behavior_validator.rs` | `BehaviorValidator`, `SafetyRule`, `ValidationResult`, `ValidationStats` | Active | |
| `astraweave-memory/src/consolidation.rs` | `ConsolidationEngine`, `ConsolidationConfig` (default `association_threshold = 0.7`, `temporal_window_hours = 24.0`, `max_associations = 10`) | Active | |
| `astraweave-memory/src/forgetting.rs` | `ForgettingEngine`, `ForgettingConfig`, per-`MemoryType` `ForgettingCurve` (Sensory half-life ~6 h, Working ~1 day, Episodic ~2 weeks, …) | Active | Re-exported via `lib.rs:51-52`. Not in original parent §5 enumeration |
| `astraweave-memory/src/compression.rs` | `CompressionEngine`, `CompressionConfig`, `CompressionResult` (memory summarization to reduce storage) | Active | Re-exported via `lib.rs:54-55`. Not in original parent §5 enumeration |
| `astraweave-memory/src/retrieval.rs` | `RetrievalConfig`, `RetrievalResult`, `ScoreBreakdown`, `RetrievalPath` (semantic + temporal + associative weighted search; defaults `semantic_weight = 0.6`, `temporal_weight = 0.2`, `associative_weight = 0.2`) | Active | Re-exported via `lib.rs:57-58`. Not in original parent §5 enumeration |
| `astraweave-memory/src/sharing.rs` | `SharingConfig`, `ShareRequest`, `SharingResult`, `SharingType`, `PrivacyLevel`, `SharedMemoryContent`, `SharingMetadata` (cross-agent memory sharing with permissions) | Active | Re-exported via `lib.rs:60-61`. Not in original parent §5 enumeration |
| `astraweave-memory/src/persona.rs` | Legacy persona types: `Episode` (different from `episode::Episode`), `Persona`, `Fact`, `Skill`, `CompanionProfile` | Active (consumed externally via `astraweave-persona`) | Re-exported via `lib.rs:63-64`. The collision-renamed `Episode` is here, NOT in `episode.rs` |
| `astraweave-memory/src/components.rs` | Bevy `MemoryComponent`, `MemoryEntityConfig` (Bevy ECS integration) | Active (feature `bevy`) | `#[cfg(feature = "bevy")]`; `Cargo.toml:27` declares `bevy = ["bevy_ecs"]`. **The only AI-side crate that uses `bevy_ecs` rather than `astraweave-ecs`** |
| Per-subsystem trace | See §13.1 below for the dedicated Memory subsystem trace | Done | |

### `astraweave-director`, `astraweave-npc`, `astraweave-dialogue`, `astraweave-coordination`

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-director/src/lib.rs` (374 LoC) | `BossDirector` heuristic planner + re-exports of `phase`/`llm_director`/`components`/`systems` (+ feature-gated `veilweaver_warden`) | Active | `#![forbid(unsafe_code)]`. The heuristic planner has 17 inline tests. See §13.2 |
| `astraweave-director/src/llm_director.rs` (967 LoC) | `LlmDirector`, `PlayerBehaviorModel`, `TacticPlan`, `TacticOutcome`, `LlmDirectorConfig`. Pulls in `astraweave-llm::LlmClient`, `astraweave-rag::RagPipeline`, `astraweave-context::ConversationHistory`, `astraweave-prompts::{PromptLibrary, PromptTemplate}` | Active | Largest source file in the crate. See §13.2 |
| `astraweave-director/src/components.rs` (660 LoC) | `CDirectorState`, `CTacticExecution`, `CDirectorMetrics` ECS components | Active | Stores per-entity `PlayerBehaviorModel`, current plan, recent outcomes, difficulty modifier |
| `astraweave-director/src/phase.rs` (568 LoC) | `PhaseDirector`, `PhaseSpec`, `PhaseState`, `PhasePlan` — HP-threshold-driven phase machine with telegraph messages | Active | Boss-phase transitions wire HP → terrain-bias/aggression knobs |
| `astraweave-director/src/systems.rs` (445 LoC) | `DirectorLlmSystem` async-update orchestrator (adaptation interval, tactic adaptation, difficulty adjustment) | Active | Designed for ECS but takes raw mutable refs, not `&mut ecs::World` — see §13.2.5 |
| `astraweave-director/src/veilweaver_warden.rs` (277 LoC) | `OathboundWardenDirector`, `WardenPhase`, `WardenDirective`, `StormChoice`, `AdaptiveAbility` — three-stage adaptive encounter for the Veilweaver Oathbound Warden boss | Active (feature `veilweaver_slice`) | Consumed by `veilweaver_slice_runtime/src/boss_encounter.rs` |
| `astraweave-director/tests/mutation_resistant_comprehensive_tests.rs` (1455 LoC) | Mutation-killing tests across all director surfaces (95 `#[test]` attrs) | Active | Largest test file in the crate |
| `astraweave-director/benches/director_adversarial.rs` (1005 LoC) | Criterion bench harness for director planning | Active | |
| `astraweave-npc/src/lib.rs` (12 LoC) | Top-level: `pub mod` declarations + glob re-exports for all 5 submodules | Active | `#![forbid(unsafe_code)]`. Thin shim |
| `astraweave-npc/src/runtime.rs` (656 LoC) | `Npc`, `NpcManager`, `NpcId = u64`, `CommandSink` trait, `EngineCommandSink` impl (PhysicsWorld + AudioEngine backing) | Active | The execution layer. See §13.3.2-§13.3.4 |
| `astraweave-npc/src/behavior.rs` (197 LoC) | `NpcAction` 6-variant `#[non_exhaustive]` enum, `NpcPlan`, `NpcMode` 6-variant `#[non_exhaustive]` enum, `EmoteKind` 4-variant enum | Active | NPC's parallel-vocabulary alternatives to canonical `ActionStep`/`PlanIntent` |
| `astraweave-npc/src/llm.rs` (436 LoC) | `LlmAdapter` trait + `MockLlm` heuristic impl (Merchant/Guard/Civilian/QuestGiver role-based rules) | Active | Despite the filename, contains no real LLM client integration. `MockLlm` is hand-coded heuristic |
| `astraweave-npc/src/profile.rs` (251 LoC) | `NpcProfile`, `Persona`, `Memory` (NPC-local), `ScheduleEntry`, `Role` 4-variant `#[non_exhaustive]` enum, `load_profile_from_toml_str` | Active | NPC's own `Persona`/`Memory` types are distinct from `astraweave-memory::{Persona, Memory}` |
| `astraweave-npc/src/sense.rs` (131 LoC) | `NpcWorldView { time_of_day, self_pos, player_pos, player_dist, nearby_threat, location_tag }` + builders | Active | NPC's alternative to canonical `WorldSnapshot`; subset of state, NPC-relevant only |
| `astraweave-npc/tests/mutation_resistant_comprehensive_tests.rs` (797 LoC) | 47 mutation-killing integration tests | Active | |
| `astraweave-npc/benches/npc_adversarial.rs` (1374 LoC) | Criterion bench harness with adversarial cases | Active | |
| `astraweave-dialogue/src/lib.rs` (1397 LoC) | Top-level: `DialogueNode`, `DialogueResponse`, `DialogueGraph` with builder methods, summary helpers, validation API (`DialogueGraph::validate`). 78 inline tests | Active | `#![forbid(unsafe_code)]`. Production-wired into editor and game loop |
| `astraweave-dialogue/src/runner.rs` (370 LoC) | `DialogueRunner` state machine (`Idle`/`WaitingForChoice`/`Finished`), `DialogueEvent` enum (`NodeEntered`/`ChoiceMade`/`Ended`), `RunnerState`. API: `start(node_id)`, `choose(choice_index)`, `drain_events()`. 8 inline tests | Active | Consumed by `veilweaver_slice_runtime/src/game_loop.rs:16-17` |
| `astraweave-dialogue/src/toml_loader.rs` (266 LoC) | `load_dialogue_from_toml -> Result<LoadedDialogue>`, `LoadedDialogue { dialogue_id, start_node, graph }`. TOML schema `[[nodes]]` with `line.{speaker, text}` and `choices[{text, go_to}]`. 7 inline tests | Active | Consumed by `veilweaver_slice_runtime/{src/game_loop.rs:446, tests/e2e_*}` |
| `astraweave-dialogue/src/llm_dialogue.rs` (2941 LoC, 60% of crate by LoC) | `LlmDialogueSystem` with `Arc<dyn LlmClient>` + `Arc<RwLock<RagPipeline>>` + `Arc<RwLock<TemplateEngine>>` + `ActiveConversation` state + `DialogueConfig`/`EmotionAnalysisConfig`/`DialogueContextConfig`/`BranchingConfig`/`QualityControlConfig`. Uses `regex::Regex` + `rand::{thread_rng, Rng}`. 75 inline tests | Active (code) / Dormant (runtime) | Workspace grep confirms zero external consumers as of 2026-05-12. See §13.4.5/§13.4.7 |
| `astraweave-dialogue/tests/dialogue.rs` (38 LoC) | 2 integration tests against `DialogueGraph::validate` | Active | |
| `astraweave-dialogue/tests/mutation_resistant_comprehensive_tests.rs` (1329 LoC) | 127 mutation-killing integration tests | Active | |
| `astraweave-dialogue/benches/dialogue_bench.rs` (589 LoC) | Criterion bench harness | Active | |
| `astraweave-coordination/src/lib.rs` (27 LoC) | Module declarations + re-exports for `agent`, `coordination`, `world_events`, `narrative_coherence`. Three commented-out module declarations (`social_graph` at `lib.rs:14-15`, `components` at `:23-24`, `systems` at `:26-27`) each annotated "Source file does not exist on disk" | Active | `#![forbid(unsafe_code)]`. See §13.5 |
| `astraweave-coordination/src/agent.rs` (817 LoC) | `Agent` trait (async, 15 required methods), `AgentState` 6-variant `#[non_exhaustive]` enum, `AgentMessage`, `AgentGoal`, `Task`, `TaskResult`, `WorldEvent`, `EventSeverity`, `ResourceUsage`, `CoordinationContext`, `CoordinationStatus` | Active (no external `impl Agent for …` outside crate-internal tests) | See §13.5 |
| `astraweave-coordination/src/coordination.rs` (2115 LoC) | `AgentCoordinator`, `CoordinatorConfig`, `ResourceStrategy` enum, `MessageRouter`, `RoutingRule`, `RoutingAction` enum, `ResourceManager`, `EventDispatcher`, `CoordinationMetrics`. Three `#[allow(dead_code)]` markers explicitly "reserved for future ... implementation/pipeline" | Active (in-design — see §13.5.7) | Largest source file in the crate |
| `astraweave-coordination/src/world_events.rs` (1060 LoC) | `WorldEventGenerator`, `EventGenerationConfig`, `SeverityDistribution`, `EventTemplate`, `GeneratedEvent`, `Storyline`. Field-level `#[allow(dead_code)]` "reserved for RAG-enhanced event generation pipeline" at `world_events.rs:19` | Active (in-design) | Holds `Arc<dyn LlmClient>` + `Arc<RagPipeline>` + `Arc<RwLock<ConversationHistory>>` + `Arc<RwLock<PromptLibrary>>` |
| `astraweave-coordination/src/narrative_coherence.rs` (1298 LoC) | `NarrativeCoherenceEngine`, `CoherenceConfig`, `NarrativeState`, `ConsistencyRule`, `StoryThread`, `CharacterArc`, `WorldContinuity`. Field-level `#[allow(dead_code)]` "reserved for RAG-enhanced coherence pipeline" at `narrative_coherence.rs:17` | Active (in-design) | Same heavy LLM-stack dependency pattern |
| `astraweave-coordination/tests/mutation_resistant_comprehensive_tests.rs` (1036 LoC) | 90 mutation-killing integration tests | Active | The only file in the workspace that consumes the Coordination types (beyond the crate's own source) |
| `astraweave-coordination/benches/coordination_adversarial.rs` (982 LoC) | Criterion bench harness | Active | |
| Per-subsystem traces | Each warrants a dedicated trace doc | Follow-up | |

### `astraweave-rag`, `astraweave-embeddings`, `astraweave-context` — RAG stack

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-rag/src/{lib,pipeline,retrieval,consolidation,forgetting,injection}.rs` (6 files / ~6.5K LoC) | `RagPipeline` composite (consolidation auto-triggered, forgetting manual), `VectorStoreInterface` trait, `VectorStoreWrapper`, `RagConfig`/`InjectionConfig`/`OrderingStrategy`/`DiversityConfig`/`PerformanceConfig`/`RetrievedMemory`/`RagMetrics`/`MemoryQuery` etc. Includes standalone `RetrievalEngine` + `InjectionEngine` (zero workspace callers — see §13.8.4) | **Pipeline dormant for runtime AI loop**; foundation types referenced by 5 dormant consumer crates | `#![forbid(unsafe_code)]`. See §13.8 |
| `astraweave-embeddings/src/{lib,client,store,utils}.rs` (4 files / ~3.2K LoC) | `EmbeddingClient` trait (+ MockEmbeddingClient deterministic hash-to-vec + optional ONNX/Candle/HTTP clients), `VectorStore` (DashMap-backed; linear scan despite "HNSW" advertising — see §13.8.7), `Memory`/`MemoryCategory` (9 variants), `StoredVector`, `SearchResult`, `EmbeddingConfig`, `DistanceMetric` (Cosine/Euclidean/Manhattan/DotProduct) | Active (foundation) | Default dim 384 (sentence-transformers/all-MiniLM-L6-v2). `Cargo.toml:31,42` declares `hnsw_rs` + `hnsw` feature default-on but code is linear scan |
| `astraweave-context/src/{lib,history,token_counter,window,summarizer}.rs` (5 files / ~2.6K LoC) | `ContextConfig`, `OverflowStrategy`, `SharingConfig`, `Message`, `Role`, `ConversationHistory` (sliding-window + summarization), `TokenCounter` (tiktoken-rs cl100k_base + cache + estimation fallback), `ContextWindow`, summarization engine | Active (foundation) | `TokenCounter::new("cl100k_base")` is the most actively-wired primitive in the entire RAG stack — instantiated by both `RagPipeline::new` AND `ConversationHistory::new`. See §13.8 |
| `astraweave-ai/src/rag/{mod,pipeline}.rs` (orphaned) | Second `RagPipeline` struct (~360 LoC, simpler — no consolidation/forgetting). `pub mod rag` is NOT declared in `astraweave-ai/src/lib.rs` (verified 2026-05-12) | **Orphaned source files** — never compile | Parallel-implementation drift mirroring CLAUDE.md "Architecture Drift" anti-pattern. See §13.8.7 |
| `astraweave-ai/src/persona/{mod,manager}.rs` (orphaned) | Persona-management surface. Not declared in `astraweave-ai/src/lib.rs` | **Orphaned source files** | Same pattern as orphaned inner-rag |
| Per-subsystem trace | See §13.8 below for the dedicated RAG subsystem trace | Done | |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Active (feature `X`)**: Compiles only when the named Cargo feature is enabled
- **Active (test gate)**: Inside `#[cfg(test)]`
- **Follow-up**: This trace points at the subsystem but defers detailed enumeration to a future dedicated trace

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Two GOAP implementations | `astraweave-behavior/src/goap.rs` (canonical) vs `astraweave-ai/src/goap/` (22 files, feature `planner_advanced`) | Coexisting | Canonical is used by the Arbiter's GOAP slot (typically). Advanced adds learning, shadow mode, persistence, goal authoring, plan visualization. No shared Rust types in this pass; explicit conversion required |
| Two control-mode enums | `PlannerMode` (`core_loop.rs:23-32` — Rule / BehaviorTree / GOAP) vs `AIControlMode` (`ai_arbiter.rs:91-109` — GOAP / ExecutingLLM { step_index } / BehaviorTree) | Coexisting (intentional, different layers) | `PlannerMode` is per-entity dispatch on `CAiController`; `AIControlMode` is Arbiter internal state |
| Three validation layers | `astraweave-ai/src/tool_sandbox.rs` (taxonomy) + `astraweave-llm/src/tool_guard.rs` (LLM-output-side) + `astraweave-core/src/validation.rs::validate_and_execute` (engine-side) | Coexisting (intentional, defense in depth) | Each layer serves a different purpose; only the third actually mutates `World` |
| `Orchestrator` (sync) vs `OrchestratorAsync` (async) | `astraweave-ai/src/orchestrator.rs:14-25` | Coexisting (intentional) | Sync trait for fast-path planners; async trait for LLM-backed planners |
| Two `Episode` types in `astraweave-memory` | `astraweave-memory::episode::Episode` (renamed `GameEpisode` at re-export) vs `astraweave-memory::persona::Episode` (verified 2026-05-12 at `persona.rs:5-10`: `pub struct Episode { title, summary, tags, ts }`) | Coexisting | The re-export explicitly renames one to avoid the clash |
| Hermes2Pro vs Qwen3 vs Phi3 LLM clients | Three Ollama client files + one local Phi3 Candle client | Coexisting (configurable) | The Arbiter is model-agnostic; the choice is made at construction time. Recent direction per commit `2468b25f1` is "Replace Phi3 with Hermes2Pro" but Qwen3 is the current arbiter doc reference |
| Heuristic / Rule / GOAP fallbacks within LLM fallback tiers | `astraweave-llm/src/fallback_system.rs` Tier-3 uses `HeuristicConfig` rules; `astraweave-ai/src/orchestrator.rs::RuleOrchestrator` is a separate hand-coded rule planner | Coexisting | The two "rule-based" code paths are not the same. Heuristic-tier rules are configurable; `RuleOrchestrator` is hardcoded smoke-and-advance logic |
| Two commented-out module declarations in `astraweave-coordination/src/lib.rs:14-27` | `social_graph`, `components`, `systems` — "Source file does not exist on disk" | Transitional / residue | Status confirmed in §13.5 Coordination subsystem trace (added 2026-05-12): in-design with explicit `#[allow(dead_code)]` "reserved for future..." markers throughout the active modules — pattern is "designed but not wired" |

### Naming collisions

- **`Orchestrator`**: a sync trait in `astraweave-ai/src/orchestrator.rs:22`; `OrchestratorAsync` is its async sibling at `:14`. Many crates have their own "orchestrator" structs (`FallbackOrchestrator` in `astraweave-llm`, `VeilweaverCompanionOrchestrator` in `astraweave-ai/src/veilweaver.rs`, `LlmPersonaManager` in persona). Each implements one of the two traits. Always cite the file + struct when discussing.
- **`GOAP`**: refers to the canonical implementation in `astraweave-behavior::goap` AND to the advanced submodule `astraweave-ai::goap` (feature `planner_advanced`). Always specify which.
- **`WorldState`**: `astraweave-behavior::goap::WorldState` (canonical `BTreeMap<u32, bool>` symbolic state) vs `astraweave-ai::goap::state::WorldState` (advanced typed state). Different types.
- **`Action` / `Goal` / `Planner`**: Each name appears in both canonical (`astraweave-behavior::goap::{GoapAction, GoapGoal, GoapPlanner}`) and advanced (`astraweave-ai::goap::{action::Action, goal::Goal, planner::Planner}`) hierarchies. Different types.
- **`PlannerMode` vs `AIControlMode`**: Both enums have a `GOAP` variant and a `BehaviorTree` variant but live at different layers (dispatch vs Arbiter state).
- **`tool_sandbox` vs `tool_guard` vs `validate_and_execute`**: All three relate to "tool validation" but serve distinct purposes (taxonomy vs LLM-output-side policy vs engine-side execution).
- **"GOAP+Qwen3 Hybrid" vs "GOAP+Hermes Hybrid"**: Same Arbiter, different model in different doc eras.
- **`Episode`**: `astraweave-memory::episode::Episode` vs `astraweave-memory::persona::Episode` (renamed `GameEpisode` at the re-export per `lib.rs:18-23`).
- **`PlanIntent` vs `PlanSource`**: `PlanIntent` is the canonical AI output struct (`astraweave-core`). `PlanSource` is the LLM-side enum that distinguishes whether a `PlanIntent` came from the LLM or from a fallback.

### Known cognitive traps

- **Trap**: Adding a third GOAP implementation, or extending one and assuming the other will pick up the change.
  **What's actually true**: The canonical (`astraweave-behavior::goap`) and advanced (`astraweave-ai::goap`) implementations share **no Rust types** in this pass. Each is internally consistent but separate. Per CLAUDE.md Scope Discipline: "Never build a second implementation of a logical system that already exists" — the two GOAPs already exist; do not add a third.
- **Trap**: Calling `validate_and_execute` on a plan and assuming the LLM has already validated it.
  **What's actually true**: `validate_and_execute` (`astraweave-core/src/validation.rs:10-…`) is the **only** layer that actually mutates `World`. LLM-side `tool_guard` is advisory at best; `tool_sandbox` defines the taxonomy but doesn't apply effects. AI proposes, engine disposes.
- **Trap**: Renaming a `WorldSnapshot` field as a refactor.
  **What's actually true**: LLM prompts at `astraweave-llm/src/prompts.rs:209+` and `compression.rs:139+` and few-shot examples at `prompt_template.rs:227-251` reference the field names (translated, but the translation function uses Rust field accessors). The LLM has been trained against the output JSON keys. Renaming requires coordinated Rust + serializer + prompt audit. See `docs/architecture/ecs_math_core_sdk_foundation.md` Invariant 8 for the canonical discussion.
- **Trap**: Registering an AI system on a stage name other than `"ai_planning"` and expecting it to run.
  **What's actually true**: Per `docs/architecture/ecs_math_core_sdk_foundation.md` §6 Trap on stage-name silent-drop: `Schedule::add_system` for an unknown stage name silently drops the registration in release builds. Use `"ai_planning"` (the canonical stage) or extend `App::new()`.
- **Trap**: Treating the Arbiter as thread-safe.
  **What's actually true**: `AIArbiter` is **not `Send`/`Sync`** (`ai_arbiter.rs:194-195`). Sharing across threads requires external synchronization or per-thread instances.
- **Trap**: Setting `LLM_CACHE_SIM_THRESH` to a value <1.0 to "improve cache hit rate".
  **What's actually true**: `astraweave-llm/src/lib.rs:57-65` documents that similarity matching is "nondeterministic across prompt variants and can cause unexpected cross-test pollution when tests run in parallel". Default 1.0 (exact match only) is intentional. Lowering it has correctness implications.
- **Trap**: Expecting `RuleOrchestrator` to use the configurable heuristic engine.
  **What's actually true**: `RuleOrchestrator` (`astraweave-ai/src/orchestrator.rs:53-…`) is hand-coded smoke-and-advance logic. The configurable `HeuristicConfig` (`astraweave-llm/src/heuristics.rs`) is a separate, LLM-fallback-tier-3 rule engine. They are not the same code path.
- **Trap**: Adding a new variant to `AIControlMode`, `PlannerMode`, `FallbackTier`, `ExtractionMethod`, `ToolVerb`, `BehaviorNode`, `DecoratorType`, `ChannelData`, `Interpolation`, or any other AI-side enum.
  **What's actually true**: All listed enums are `#[non_exhaustive]`. Adding a variant within the crate works; external matches must include a wildcard arm. But: every enum has dependent code (Arbiter mode-transition logic, dispatch tables, validation chains, prompt formatters, plan parsers). CLAUDE.md Integration Completeness #2 ("All registration surfaces touched") applies — `rg <NewVariant>` workspace-wide before declaring done.
- **Trap**: Reading the `llm_adapter.rs` file (line 1: "Defensive LLM Adapter Example (stub)") and treating it as the canonical LLM validation layer.
  **What's actually true**: The file comment explicitly says "stub". The canonical layers are `plan_parser.rs` (parse + validate JSON), `tool_guard.rs` (policy enforcement), `production_hardening.rs` (rate/circuit/backpressure), and the engine-side `validate_and_execute`.
- **Trap**: Assuming the runtime LLM model is Qwen3 because `ai_arbiter.rs:1` doc-comment says "GOAP+Qwen3 Hybrid Control System" and CLAUDE.md says "GOAP+Qwen3 Hybrid".
  **What's actually true** (verified 2026-05-12): `astraweave-ai/src/orchestrator.rs:488-490` defaults `OLLAMA_MODEL` env var to `"phi3:medium"` (`unwrap_or_else(|_| "phi3:medium".to_string())`). Absent an explicit `OLLAMA_MODEL=qwen3:8b` override at runtime, the production path picks phi3. The Qwen3 client + Hermes2Pro client exist as parallel `astraweave-llm/src/{qwen3_ollama,hermes2pro_ollama}.rs` structs but the runtime default selector still points at phi3. The doc-comment migration (Qwen3) outpaced the runtime selector update.

---

## 7. Decision Log

### Decision: Four-stage canonical loop (Perception → Reasoning → Planning → Action)
- **Date:** [Reasoning not recovered from available sources — predates current trace; visible as foundational framing in CLAUDE.md and `astraweave-ai/src/lib.rs:1-12`]
- **Status:** Accepted (in active code; **foundational**)
- **Context:** CLAUDE.md frames AstraWeave's AI-first mandate explicitly. The four-stage diagram (`Perception → Reasoning → Planning → Action`) is reproduced in `CLAUDE.md`, `astraweave-ai/src/lib.rs:1-12`, `docs/current/ARCHITECTURE_REFERENCE.md:28-32`, and other docs. It encodes the architectural commitment that AI never bypasses engine validation.
- **Decision:** Every AI subsystem produces `PlanIntent`s and routes them through `validate_and_execute`. No AI subsystem mutates `World` directly. `WorldSnapshot` is the only AI input.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** All AI is sandboxed by construction. LLM cannot cheat. AI can be capture/replayed deterministically (the `WorldSnapshot` + `PlanIntent` pair is the full I/O record). The `WorldSnapshot` schema is load-bearing across many subsystems including LLM prompts.

### Decision: `Orchestrator` trait abstracts AI planning across paradigms
- **Date:** [Reasoning not recovered]
- **Status:** Accepted (in active code)
- **Context:** Rule-based, behavior-tree, GOAP, LLM, and hybrid planners all need a uniform interface so the engine's `ai_planning` stage can dispatch agnostically.
- **Decision:** Define `Orchestrator { fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent }` (sync) and `OrchestratorAsync { async fn plan(&self, snap, budget_ms) -> Result<PlanIntent> }` (async). Both produce `PlanIntent`. Implementations live in many crates; orchestrators are first-class plug points.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** Adding a new AI paradigm is a question of implementing the trait. The Arbiter holds `Box<dyn Orchestrator>` for both its GOAP and BT slots (`ai_arbiter.rs:212-215`), allowing arbitrary implementations.

### Decision: `AIArbiter` hybrid (GOAP + async LLM + BT fallback) with three control modes
- **Date:** Phase 7 (per `ai_arbiter.rs:37` "Phase 7 Arbiter: Async infrastructure for GOAP+Hermes hybrid control"; recent doc reframes as Qwen3)
- **Status:** Accepted (in active code, feature `llm_orchestrator`)
- **Context:** LLM latency (3-8 s for Qwen3-8B) is too high for per-frame use. GOAP latency (5-30 µs) is fine but lacks strategic depth. Behavior trees are deterministic but rigid. The architecture diagram at `ai_arbiter.rs:14-37` documents the rationale: GOAP handles instant tactical control while LLM plans strategically in the background; BT is emergency fallback when GOAP fails.
- **Decision:** Three-mode state machine (`AIControlMode::{GOAP, ExecutingLLM { step_index }, BehaviorTree}`). `update()` always returns instantly. LLM requests are cooldown-gated (default 15.0 s, `ai_arbiter.rs:301`). Mode transitions happen automatically on plan readiness, plan exhaustion, or GOAP failure. Dual-executor support (strategic + fast) for low-latency inline LLM queries — added later (commit not recovered).
- **Alternatives considered:** Single LLM executor (rejected; doesn't support inline fast queries). Blocking on LLM (rejected; breaks 60 FPS). Pure GOAP (rejected; lacks strategic depth). Per the inline doc-comment at `ai_arbiter.rs:14-37`.
- **Consequences:** Zero user-facing latency (LLM thinks in background, GOAP keeps acting). Performance: 5-30 µs GOAP, <50 µs ExecutingLLM, <1 ms BT (`ai_arbiter.rs:367-368`). Capacity: 12,700+ agents @ 60 FPS (`astraweave-ai/src/lib.rs:27`). Arbiter must be tested for all transition paths — covered by `astraweave-ai/tests/arbiter_*` files.

### Decision: 4-tier LLM fallback chain
- **Date:** Phase 7 (per `astraweave-llm/src/fallback_system.rs:1-7` "Phase 7: Multi-Tier Fallback System")
- **Status:** Accepted (in active code)
- **Context:** LLMs can fail in many ways: timeout, network error, parse error, hallucination, schema violation, rate limit, circuit-breaker trip. Without fallback, the AI loses agency entirely on any failure. Per the inline header: "Provides graceful degradation when LLM planning fails."
- **Decision:** Four tiers: Tier-1 (FullLlm, 37 tools, detailed prompts) → Tier-2 (SimplifiedLlm, 10 common tools, compressed prompts) → Tier-3 (Heuristic rules) → Tier-4 (Emergency safe-default Scan+Wait). Each tier's `next()` method (`fallback_system.rs:45-52`) advances the chain on failure. `Emergency` returns `None` from `next()` — no further fallback.
- **Alternatives considered:** [Reasoning not recovered — the comment says "Phase 7" indicating it was a named milestone.]
- **Consequences:** AI always returns a plan. Worst-case behavior is degenerate (Scan + Wait) but never blocks. The cost is complexity — every LLM call site goes through `FallbackOrchestrator`, which holds many subordinate components (`fallback_system.rs:14-23`).

### Decision: 5-strategy LLM-response extraction
- **Date:** Phase 7 (per `astraweave-llm/src/plan_parser.rs:1-9`)
- **Status:** Accepted (in active code)
- **Context:** LLMs return JSON in many formats: bare JSON, fenced code blocks, message-envelope objects, embedded objects with surrounding prose, key-typo variants. A single parse strategy would have low success rate.
- **Decision:** Five extraction methods tried in order: `Direct` (raw parse) → `CodeFence` (extract from ```json fences) → `Envelope` (look for `message.content` / `response` fields) → `ObjectExtraction` (regex for `{…}` substring) → `Tolerant` (key-name normalization). Each carries a name string for metrics and a `validation_warnings` collector.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** Robust parsing across LLM vendors and output formats. Each successful parse records its method for telemetry (`plan_parser.rs:35-44`). Tool hallucination detection (validating action verbs against `ToolRegistry`) layered on top.

### Decision: Defense-in-depth validation (three layers)
- **Date:** [Reasoning not recovered — emerged from Phase 7 hardening]
- **Status:** Accepted (in active code)
- **Context:** AI cheating cannot be allowed by construction. Validation must catch invalid actions both at the LLM-output boundary (cheap, frequent) and at the engine-execution boundary (authoritative).
- **Decision:** Three layers: (1) `tool_sandbox` (taxonomy + validation categories), (2) `tool_guard` (policy filtering at LLM-output, `tool_guard.rs:1-28`), (3) `validate_and_execute` (engine-side gate with `EngineError` variants `InvalidAction`, `NoPath`, `LosBlocked`, `Cooldown`).
- **Alternatives considered:** Single validation point (rejected — would either trust LLM output or duplicate engine validation everywhere).
- **Consequences:** Engineer must understand which layer applies when. Documented in §6 Cognitive Traps. Each layer is testable in isolation.

### Decision: `WorldState` as `BTreeMap<u32, bool>` with interned keys
- **Date:** [Reasoning not recovered]
- **Status:** Accepted (in active code, `astraweave-behavior/src/goap.rs:14-65`)
- **Context:** GOAP A* search requires fast state-equality checks and a hash-friendly representation. String keys are slow; integer keys via interning are fast and deterministic.
- **Decision:** `WorldState.facts: BTreeMap<u32, bool>` where u32 is the interned fact-name. `BTreeMap` (not `HashMap`) for deterministic iteration. `set(key, value)` and `get(key)` go through `intern(key)` (`goap.rs:34-40`).
- **Alternatives considered:** `HashMap<String, bool>` (rejected for non-determinism and slower comparison). `BTreeMap<String, bool>` (rejected for slower key comparison). [Reasoning not recovered in commits.]
- **Consequences:** Deterministic plan output (required for capture/replay). 1.01 µs GOAP cache hit. Interner is global state — additions to the interner are not retracted between tests. Verified 2026-05-12: `grep -rn "serial_test\|#[serial]" astraweave-behavior` returned zero matches — no test isolation mechanism (e.g. `serial_test` crate) gates interner-touching tests. Within a single test target's process, interned strings persist between tests; cargo's process-per-target isolation provides the natural reset boundary.

### Decision: Production hardening composes 6 reliability primitives
- **Date:** [Reasoning not recovered]
- **Status:** Accepted (in active code, `astraweave-llm/src/production_hardening.rs:14-38`)
- **Context:** Production deployment of LLMs requires rate limiting (cost control), circuit breaking (failure isolation), backpressure (queue management), A/B testing (experimentation), telemetry (observability), health checking (recovery). Each is a separate subsystem with its own configuration.
- **Decision:** Compose all six into a `ProductionHardeningLayer` struct: `RateLimiter` + `CircuitBreakerManager` + `BackpressureManager` + `ABTestFramework` + `LlmTelemetry` + `HealthChecker` + shutdown signal. All wrapped in `Arc` for cheap cloning.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** One canonical hardening layer to apply to every LLM call. Configuration via `HardeningConfig`. Background `health_checker_handle` for periodic health checks; shutdown via `shutdown_tx` watch channel.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | All AI plans flow through `validate_and_execute` before mutating `World` | Yes | Engine-side: `validate_and_execute` is the only function that writes to `World` for AI actions. AI subsystems return `PlanIntent`; callers must invoke `validate_and_execute` |
| 2 | `AIArbiter::update` always returns an `ActionStep` in <100 µs target / 5-30 µs actual (GOAP mode) | Yes (test/bench) | `ai_arbiter.rs:367-368` performance target; bench at `astraweave-ai/benches/arbiter_bench.rs`; metric `ai.arbiter.update_latency` |
| 3 | `AIArbiter` is `!Send + !Sync` — single-threaded usage only | Yes (compile-time) | The trait objects `Box<dyn Orchestrator>` are not `Send`/`Sync` by default; `AIArbiter::update` takes `&mut self` (`ai_arbiter.rs:384`) |
| 4 | LLM request cooldown enforced — no LLM call within `llm_request_cooldown` of last call | Yes | `maybe_request_llm(snap)` checks `snap.t - last_llm_request_time >= llm_request_cooldown` (default 15.0 s, `ai_arbiter.rs:301`) |
| 5 | Plan exhaustion transitions `ExecutingLLM` → `GOAP` | Yes | `ai_arbiter.rs:442-449`: when `next_index >= plan.steps.len()`, `transition_to_goap()` |
| 6 | Fallback tier chain terminates at `Emergency` | Yes | `fallback_system.rs:50-52`: `Emergency::next()` returns `None` |
| 7 | LLM response parsing tries all 5 `ExtractionMethod`s before failing | Yes | `plan_parser.rs::parse_llm_response` chain (per file header at lines 1-9) |
| 8 | Plan parser rejects unknown tools (hallucination detection) | Yes | `plan_parser.rs:5` "Tool hallucination detection (reject unknown tools)"; validates against `ToolRegistry` from `astraweave-core` |
| 9 | `WorldState` facts iterate deterministically (BTreeMap) | Yes | `goap.rs:16` (`BTreeMap<u32, bool>` over `HashMap`) |
| 10 | GOAP plan cache key is deterministic across runs (interned facts) | Yes | `goap_cache.rs` keys on `WorldState` which uses interned u32 — deterministic if interner state is reproduced |
| 11 | `MAX_PROMPT_LENGTH = 4096` is enforced by `safe_llm_invoke` | Yes | `astraweave-llm/src/llm_adapter.rs:5, 9-11`. Note: the file is labeled "stub" in its header; other call sites may not enforce this limit |
| 12 | Global `PromptCache` defaults to exact-match-only (similarity threshold 1.0) | Yes | `astraweave-llm/src/lib.rs:57-67`. Override via `LLM_CACHE_SIM_THRESH` env var (clamped to `[0.0, 1.0]`) |
| 13 | `PromptCache::clear()` resets state for test isolation | Yes | `astraweave-llm/src/lib.rs:71-73` (`pub fn clear_global_cache()`) |
| 14 | `CircuitBreaker` blocks LLM calls when failure rate exceeds threshold | Yes | `circuit_breaker.rs` (verified by integration tests like `astraweave-llm/tests/fallback_chain_integration.rs`) |
| 15 | AI crates are unsafe-free | Yes (compile-time for 7 of 8 crates; verified-clean for `astraweave-ai`) | `astraweave-llm/src/lib.rs:1`, `astraweave-behavior/src/lib.rs:1`, `astraweave-memory/src/lib.rs:1`, `astraweave-director/src/lib.rs:1`, `astraweave-coordination/src/lib.rs:1`, `astraweave-npc/src/lib.rs:1`, `astraweave-dialogue/src/lib.rs:1` — all `#![forbid(unsafe_code)]`. `astraweave-ai/src/lib.rs:1` does NOT declare it (verified 2026-05-12 — line 1 is doc-comment), but workspace grep across `astraweave-ai/src/*.rs` for unsafe blocks returned zero matches (verified 2026-05-12). |
| 16 | `BehaviorGraph::tick` returns one of `BehaviorStatus::{Success, Failure, Running}` | Yes | Verified 2026-05-12 at `astraweave-behavior/src/lib.rs:1479-1485`: `#[non_exhaustive]` enum with exactly 3 variants. |
| 17 | `PlanIntent::plan_id` is set on every generated plan | Yes | `RuleOrchestrator::propose_plan` sets `plan_id = format!("plan-{}", (snap.t * 1000.0) as i64)` (`orchestrator.rs:86`); LLM plans get IDs from the LLM or assigned post-parse |
| 18 | `AIArbiter` records mode transitions in `mode_transitions: u32` | Yes | `ai_arbiter.rs:235-251` field declarations; incremented on every `transition_to_*` call |
| 19 | `AIArbiter` is `!Send + !Sync` — must be used per-agent, never shared across threads | Yes (compile-time via auto-traits + doc-comment at `ai_arbiter.rs:194-195`) | **Closed from §11 via deep investigation 2026-05-12:** workspace grep for `Arc<AIArbiter>` or bare type name `AIArbiter` outside `astraweave-ai/src/ai_arbiter.rs`, tests, and benches returned zero matches. No external production consumers exist today, so the non-`Send` constraint is not currently violated anywhere. |
| 20 | No production code stubs in AI surface (no `*_stub` naming convention) | Yes (workspace grep) | **Closed from §11 via deep investigation 2026-05-12:** workspace grep for `_stub` across `astraweave-ai/src`, `astraweave-llm/src`, `astraweave-behavior/src`, `astraweave-memory/src` returned exactly one match: `test_phi3_stub_without_feature` at `astraweave-llm/src/phi3.rs:494` — a test function name, NOT a production stub. (Note: `astraweave-llm/src/llm_adapter.rs` is itself described as a "stub" in its file-header comment per §13.7.7, but does not use the `_stub` naming convention.) |

---

## 9. Performance & Resource Profile

### Hot paths
- **`AIArbiter::update`**: target <100 µs, actual 5-30 µs (GOAP), <50 µs (ExecutingLLM), <1 ms (BT). Documented at `ai_arbiter.rs:367-368`. Runs every frame per agent.
- **GOAP planning**: 1.01 µs cache hit, 47.2 µs cache miss (`astraweave-ai/src/lib.rs:24`). Cache is `goap_cache.rs` LRU.
- **Behavior tree tick**: 57-253 ns per tick (`astraweave-ai/src/lib.rs:25`). 66,000 agents @ 60 FPS.
- **`AsyncTask::try_recv`**: <10 µs (non-blocking poll). Used by `AIArbiter::poll_llm_result`.
- **LLM-side cache lookup**: O(1) hash on `PromptKey`; sub-millisecond. Similarity matching off by default (deterministic).

### Cold paths
- **LLM inference**: 3-8 s for Qwen3-8B (per `ai_arbiter.rs:18`). Phi3-Medium varies. Background only.
- **GOAP cache miss**: 47.2 µs A* over symbolic states. Rare under cache warm-up.
- **`FallbackOrchestrator::plan`**: Composite latency through tier chain. Tier-1 dominates if LLM succeeds; Tier-3 (heuristic) is sub-microsecond.
- **`PromptCompressor`**: Token-reduction pass. Runs once per Tier-2 invocation.
- **`production_hardening`**: Health checker background task. Periodic.
- **Episode storage**: SQLite-backed (`astraweave-memory/src/storage.rs`). Per-episode write.

### Resource ownership
- **`AIArbiter`**: owned by gameplay layer (typically one per agent or per companion). Lifetime = agent lifetime.
- **`LlmExecutor`**: owned by Arbiter. Holds `Arc<dyn OrchestratorAsync>` (shareable across executors) + `tokio::runtime::Handle`. Lifetime = Arbiter lifetime.
- **`AsyncTask<T>`**: stored in `current_llm_task: Option<AsyncTask<Result<PlanIntent>>>` on Arbiter. Drop aborts the underlying tokio task.
- **`current_plan`**: stored in `current_plan: Option<PlanIntent>` on Arbiter. Cleared on transition to GOAP.
- **`PromptCache` (global)**: `LazyLock` static (`astraweave-llm/src/lib.rs:50-67`). Lifetime = process. Cleared via `clear_global_cache()` for tests.
- **`FallbackOrchestrator`**: owned by application or `LlmExecutor`. Composes many subordinate components (`fallback_system.rs:14-23`).
- **`ProductionHardeningLayer`**: typically a singleton per LLM endpoint. Owns `Arc`-wrapped subordinates and a background `health_checker_handle: Arc<tokio::sync::RwLock<Option<JoinHandle<()>>>>`.
- **Memory storage (SQLite)**: file-backed; lifetime = process or persisted across runs.

### Bench coverage (`astraweave-ai/benches/`)
- `ai_core_loop` — canonical loop overhead
- `goap_bench` — GOAP planning latency
- `arbiter_bench` — Arbiter update cycle
- `ai_benchmarks` — Mixed AI workload
- `integration_pipeline` — End-to-end pipeline
- `multi_agent_pipeline` — Multi-agent scaling
- `goap_performance_bench` — Detailed GOAP perf
- `goap_vs_rule_bench` — Rule vs GOAP comparison
- `alloc_measure` — Allocation profile (requires `alloc-counter` + `planner_advanced`)

`astraweave-llm/benches/`: `llm_benchmarks`, `cache_stress_test`, `resilience_benchmarks`.

---

## 10. Testing & Validation

- **Inline `#[cfg(test)]` modules:** Mutation-killing tests in `astraweave-ai/src/mutation_tests.rs`, `astraweave-behavior/src/mutation_tests.rs`, plus per-module test modules throughout. Exact counts not enumerated.
- **Integration tests (`astraweave-ai/tests/`):** 29 files including:
  - **Arbiter:** `arbiter_tests.rs`, `arbiter_comprehensive_tests.rs`, `ai_arbiter_implementation_tests.rs`
  - **GOAP:** `goap_hierarchical_planning.rs`, `goap_learning_integration.rs`, `goap_vs_rule_comparison.rs`, `core_loop_goap_integration.rs`, `planner_tests.rs`
  - **Core loop / orchestrator:** `core_loop_policy_switch.rs`, `core_loop_rule_integration.rs`, `orchestrator_tool_tests.rs`, `orchestrator_additional_tests.rs`, `orchestrator_extended_tests.rs`
  - **Tool validation:** `tool_validation_tests.rs`, `tool_sandbox.rs`
  - **Perception / planning:** `perception_tests.rs`, `plan_snapshot.rs`
  - **Cross-cutting:** `cross_module_integration.rs`, `integration_tests.rs`, `ecs_integration_tests.rs`, `behavioral_correctness_tests.rs`, `mutation_resistant_comprehensive_tests.rs`
  - **Robustness:** `nan_infinity_tests.rs`, `edge_case_tests.rs`, `stress_tests.rs`, `determinism_tests.rs`, `async_task_comprehensive_tests.rs`
  - **Specialty:** `llm_fallback.rs`, `rag_integration_test.rs`
- **Integration tests (`astraweave-llm/tests/`):** 15 files including `fallback_chain_integration.rs`, `phase7_integration_tests.rs`, `concurrent_stress_tests.rs`, `timeout_retry_tests.rs`, `boundary_condition_tests.rs`, `error_message_validation_tests.rs`, `property_tests.rs`, `mutation_kill_tests.rs`, `mutation_resistant_comprehensive_tests.rs`, `advanced_features_test.rs`, `latency_comparison_bench.rs`, `integration_test.rs`, `integration_tests.rs`, `sprint2_llm_tests.rs`, `client_tests.rs`.
- **Integration tests (`astraweave-behavior/tests/`):** 4 files: `behavior.rs`, `fuzz_planner.rs`, `mutation_resistant_comprehensive_tests.rs`, `mutation_resistant_tests.rs`.
- **Benchmarks (`astraweave-ai/benches/`, `astraweave-llm/benches/`):** 9 + 3 bench harnesses (see §9).
- **Property-based tests:** `astraweave-llm/tests/property_tests.rs` uses proptest (`Cargo.toml:36`).
- **Determinism tests:** `astraweave-ai/tests/determinism_tests.rs` exercises same-input/same-output guarantees critical for capture/replay.
- **Fuzz testing:** `astraweave-behavior/tests/fuzz_planner.rs`.
- **Manual / example validation:** `examples/hello_companion`, `examples/ecs_ai_showcase`, `examples/phi3_demo`, `examples/llm_integration`, `examples/scripting_advanced_demo` exercise the AI loop at runtime.
- **Formal verification:** No Kani proofs found specifically for AI crates in this pass (Kani validation is documented for `ecs`, `math`, `core`, `sdk` in `docs/architecture/ecs_math_core_sdk_foundation.md` §8 Invariant 9). Verified 2026-05-12: `.github/workflows/kani.yml` declares default crates input as `astraweave-ecs,astraweave-math,astraweave-sdk,astraweave-core` — `astraweave-ai` is NOT in the default Kani CI matrix.
- **Miri:** CLAUDE.md ARCHITECTURE_MAP.md §577 lists `ai` as a Miri-validated crate (60 m timeout). The AI crates rely on Rust type safety + unsafe-free implementations.

---

## 11. Open Questions / Parked Decisions

- **Two GOAP implementations — consolidation roadmap?** [Decisional, enriched by §13.6 investigation 2026-05-12.] Factual state: canonical `astraweave-behavior::goap` and advanced `astraweave-ai::goap` (22 files / 16,736 LoC, feature `planner_advanced`) coexist with no shared types. The Advanced GOAP layer adds: learning (EWMA + Bayesian smoothing), persistence, shadow mode (`ShadowModeRunner` + `PlanComparison` for side-by-side rollout), plan_analyzer (`PlanMetrics` + `Suggestion`), plan_stitcher (multi-plan merging with `Conflict` + `StitchError`), plan_visualizer (multiple `VisualizationFormat`s), goal_authoring (`GoalLibrary` + `GoalDefinition`), goal_scheduler, goal_validator (`Severity` + `ValidationError`), and a richer `StateValue` enum (Bool/Int/Float/String/IntRange/FloatApprox) vs canonical's `BTreeMap<u32, bool>`. Verified 2026-05-12: Advanced GOAP has **zero production constructors** outside tests + benches + three disabled CLI bins (`astraweave-ai/src/bin.disabled/{analyze-plan,validate-goals,visualize-plan}.rs`). The integration adapter `GOAPOrchestrator` (`orchestrator.rs:11-…`) implements the canonical `Orchestrator` trait, so it's plug-compatible with `AIArbiter`, but no production code instantiates it. The Advanced GOAP is phase-numbered (Phase 0 discovery through Phase 5 tooling, per `mod.rs` and archive phase reports). Whether the advanced features eventually fold back into the canonical implementation, or whether they remain distinct (e.g. canonical for in-engine + advanced for editor authoring + offline analysis CLI), is the directional question.
- **`AIControlMode` vs `PlannerMode` — unify or keep separate?** [Decisional, **enriched by deep investigation 2026-05-12**.] **Verified variant sets (2026-05-12) — they are NOT the same.** `PlannerMode` (`astraweave-ai/src/core_loop.rs:22-32`): `#[non_exhaustive]` 3-variant enum `Rule / BehaviorTree / GOAP`. Each variant carries no data. Has feature-flag helpers (`requires_bt_feature`, `requires_goap_feature`, `is_always_available`). Used as per-entity dispatch in `CAiController.mode` (see benches `ai_core_loop.rs:196,212,...`). `AIControlMode` (`astraweave-ai/src/ai_arbiter.rs:93-110`): `#[non_exhaustive]` 3-variant enum `GOAP / ExecutingLLM { step_index } / BehaviorTree`. Has data-carrying `ExecutingLLM` variant; has helper methods `is_goap()`, `is_executing_llm()`, `is_behavior_tree()`, `is_fallback()`, `step_index()`. Used only as the runtime state field of the arbiter. **Structural diff:** `PlannerMode` has `Rule` (no LLM); `AIControlMode` has `ExecutingLLM { step_index }` (data-carrying LLM step). They share only the `GOAP` and `BehaviorTree` *names*. A unification would need to subsume both `Rule` and `ExecutingLLM` semantics — likely a 4-variant enum or a richer hierarchy. Whether to unify (single enum with broader semantics) or to keep separate with clearer naming is undecided.
- **`llm_adapter.rs` — stub vs production?** [Decisional / factual, **enriched by §13.7 investigation 2026-05-12**.] Factual state (verified 2026-05-12): the file header at `astraweave-llm/src/llm_adapter.rs:1` explicitly says "stub". `safe_llm_invoke` has **zero workspace callers** outside the file's own tests. `mock_llm_call` placeholder (`llm_adapter.rs:21-27`) just echoes the prompt back. The stub remains pure stub. The actual validation lives in `plan_parser.rs` (output-side, active via FallbackOrchestrator) + `tool_guard.rs` (output-side, dormant) + `validate_and_execute` (engine-side, active via tool sandbox). Three resolution options surfaced in §13.7.7: (a) promote — wire `safe_llm_invoke` ahead of every `LlmClient::complete`; (b) delete — rely on the existing three-layer defense; (c) keep as test fixture under a `mock_` rename. Suggested: option (b). Decision pending Andrew confirmation.
- **`MAX_PROMPT_LENGTH = 4096` — uniform enforcement?** [Decisional / factual, **resolved by §13.7 investigation 2026-05-12**.] Resolution: the limit is **NOT enforced at runtime today**. `safe_llm_invoke` (`llm_adapter.rs:9-11`) is the only enforcement point and has zero callers. `prompts.rs`, `prompt_template.rs`, and all `LlmClient::complete` direct calls bypass the check. To centralize the limit, the directional options (per §13.7.7) are: (a) add length check at the top of `ProductionHardeningLayer::process_request` reading `request.estimated_tokens` (requires also production-wiring `ProductionHardeningLayer`); (b) inline length check into `FallbackOrchestrator::generate_plan` so Tier-1/Tier-2 prompts are validated before LLM call (reuses the active path); (c) push the constant into `astraweave-core` and enforce at prompt-template-builder time. Path (b) has the lowest blast radius and uses an already-active code path.
- **Hermes2Pro vs Qwen3 vs Phi3 — settled model choice?** [Decisional, **enriched by deep investigation 2026-05-12**.] **Three production clients coexist, runtime default is still Phi3.** Verified 2026-05-12: (a) `astraweave-llm/src/{phi3_ollama,hermes2pro_ollama,qwen3_ollama}.rs` each define separate `pub struct PhiOllama`, `Hermes2ProOllama`, `Qwen3Ollama` — three coexisting Ollama clients; (b) the runtime LLM model selector in `astraweave-ai/src/orchestrator.rs:488-490` reads `OLLAMA_MODEL` env var with **default `"phi3:medium"`** (`unwrap_or_else(|_| "phi3:medium".to_string())`) — so absent explicit env override, production picks phi3; (c) `ai_arbiter.rs:1-10` doc-comment is the only place referencing "GOAP+Qwen3" as the current architecture (both thinking and non-thinking executors); (d) git log shows the migration sequence: `2468b25f1` "Replace Phi3 with Hermes2Pro" → `6fec317a1` "Add new model files for Qwen3" → `4f892ad34` "Qwen3 latency optimization report". The Phi3→Hermes2Pro replacement commit (`2468b25f1`) did NOT remove the phi3:medium default from `orchestrator.rs`. Whether Qwen3 is the long-term choice and whether/when the phi3:medium default should be removed (a one-line change) remain undecided.
- **Three commented-out modules in `astraweave-coordination/src/lib.rs:14-27` (`social_graph`, `components`, `systems`) — placeholder or stalled?** [Decisional, with enriched factual context per §13.5 investigation 2026-05-12.] Factual (verified): the three module declarations at `lib.rs:14-15` (`social_graph`), `lib.rs:23-24` (`components`), `lib.rs:26-27` (`systems`) are each commented out with an explicit "Source file does not exist on disk" annotation. Workspace-wide grep for files matching `**/coordination/src/social_graph.rs`, `components.rs`, `systems.rs` returns zero hits. The active modules (`agent.rs`, `coordination.rs`, `world_events.rs`, `narrative_coherence.rs`) themselves contain `#[allow(dead_code)]` markers explicitly tagged "reserved for future..." at 7+ locations (`coordination.rs:78, 109, 150`; `narrative_coherence.rs:17`; `world_events.rs:19, 579, 816`). The pattern surfaces a broader status: the entire Coordination crate is **in-design** — the active modules and the commented-out placeholders share the same intent (designed but not yet wired). Whether to commit (create the missing files + remove the `#[allow(dead_code)]` markers + production-wire the system) or prune (delete the commented-out declarations + reduce the crate to what's actively integrated) is a directional decision for Andrew. The crate's 98 inline+integration tests exercise the present surface in isolation.
- **Memory subsystem dormancy — production-wire, prune, or rebrand?** [Decisional — surfaced by §13.1 deep investigation 2026-05-12, supersedes prior "AdaptiveWeightManager feedback loop production-wired?" question.] Factual state (verified 2026-05-12): workspace-wide grep for `astraweave_memory::*` symbols (`AdaptiveWeightManager`, `update_from_profile`, `apply_pattern_bonuses`, `EpisodeRecorder`, `MemoryStorage`, etc.) returns **zero in-engine production consumers** in `astraweave-ai`, `astraweave-behavior`, `astraweave-render`, or any other runtime crate. The only consumers outside `astraweave-memory` itself are: (a) `astraweave-persona` — which uses ONLY the legacy `persona::*` types (`Persona`, `CompanionProfile`, `Fact`, `Skill`), not the main memory pipeline; (b) `examples/companion_profile` + `examples/llm_integration` — example-level usage; (c) `astraweave-dialogue/src/llm_dialogue.rs:1897` — a single test-only `BasePersona` reference. The 1000+ tests in `astraweave-memory` (342 inline + 680 in `tests/`) exercise the subsystem in isolation; no end-to-end runtime hookup exists. Per CLAUDE.md Integration Completeness rule, the main memory pipeline qualifies as **dormant code**. The decisional question for Andrew: production-wire (build the `astraweave-ai`/`astraweave-behavior` hookups), prune (delete unused modules), or rebrand (relocate to a sandbox or experimental crate). The legacy `persona.rs` types are excluded from this question — those are actively used by `astraweave-persona`.
- **`PromptCache` similarity threshold — should it be off by default?** [Decisional, **resolved by §13.7 investigation 2026-05-12**.] Resolution: it **is already off by default** — `lib.rs:60-65` reads `LLM_CACHE_SIM_THRESH` env var, defaults to 1.0 (exact-match-only) clamped to [0.0, 1.0]. The comment at `:57-58` documents the intent: "Phase 7: Similarity cache hits are nondeterministic across prompt variants and can cause unexpected cross-test pollution when tests run in parallel. Keep exact-match caching enabled by default, and make similarity matching opt-in." The directional question shifts from "should the default change?" (no — already correct) to "should the similarity feature be removed entirely?" — currently kept as opt-in for future use cases; recommended action is to keep as-is until a concrete production use case emerges.
- **`astraweave-ai` lacks `#![forbid(unsafe_code)]` while sibling AI crates have it.** [Decisional / factual, **enriched by deep investigation 2026-05-12**.] Factual (verified 2026-05-12): `astraweave-ai/src/lib.rs:1` is doc-comment `//! # AstraWeave AI` — no forbid_unsafe attribute. Workspace grep across the entire `astraweave-ai` crate for `forbid(unsafe_code)` or `deny(unsafe_code)` returned zero matches (re-verified 2026-05-12) — the directive is absent from ALL files in the crate, not just `lib.rs`. Workspace grep across `astraweave-ai/src/*.rs` for actual unsafe blocks returned zero matches — the crate has no unsafe code today. The seven other AI crates DO declare `#![forbid(unsafe_code)]` at line 1. Whether the absence is intentional (e.g. to allow future SIMD intrinsics, FFI, or compatibility with `unsafe_code = "deny"`-incompatible deps) or an oversight is undecided. Adding `#![forbid(unsafe_code)]` would be a one-line change with zero behavioral impact today (since no unsafe blocks exist to fail the build).
- **`Persona::Episode` vs `episode::Episode` collision in `astraweave-memory`.** [Decisional / factual.] Factual (verified 2026-05-12): `astraweave-memory/src/persona.rs:5-10` defines `pub struct Episode { title, summary, tags, ts }` (legacy persona type used by `astraweave-persona`). `astraweave-memory/src/episode.rs` defines the rich `Episode` (with `EpisodeCategory`, `EpisodeOutcome`, observations, etc.). The `lib.rs:18-23` rename ships only the latter as `GameEpisode` at the crate root, but inside `astraweave-memory` itself both names coexist at module level. Whether one should be renamed at definition (not just at re-export) is undecided.
- **Fallback Tier-3 heuristics vs `RuleOrchestrator` — overlap or distinct?** [Decisional, **enriched by §13.7 investigation 2026-05-12**.] Factual (verified 2026-05-12): `HeuristicConfig::default()` (heuristics.rs:9-49) produces 7 rules with strictly more expressive vocabulary (configurable, serializable, supports custom rules) than the hardcoded smoke-and-advance in `astraweave-ai/src/orchestrator.rs::RuleOrchestrator`. `HeuristicConfig` is **strictly more expressive** but used in only ONE place (FallbackOrchestrator Tier-3, dormant); `RuleOrchestrator` is used as the default for low-tier game scenarios. Same directional shape as the Advanced GOAP question: deprecate `RuleOrchestrator` in favor of configurable `HeuristicConfig`-based rules (gaining configurability) or fold `HeuristicConfig` into `RuleOrchestrator` (gaining a production caller). Decision pending Andrew.
- **Runtime AI path bypasses the entire LLM hardening surface.** [Decisional / **HIGH-IMPACT finding from §13.7 investigation 2026-05-12**.] Factual (verified 2026-05-12): production `AIArbiter` consumes `Arc<dyn LlmExecutor>` directly; `LlmExecutor` doc-comments at `astraweave-ai/src/llm_executor.rs:34,88,102` document a `FallbackOrchestrator::new(client, registry)` wrapping pattern but the actual implementation does not wire it. Every production LLM call goes through `LlmClient::complete` directly without rate limiting / circuit breaking / backpressure / A/B routing / retry / telemetry / 5-strategy parsing / ToolGuard / 4-tier fallback. The 15K LoC hardening surface is shelf-stocked but not in line. Three directional options (per §13.7.7): commit (wire `LlmExecutor` to consume `FallbackOrchestrator`, then optionally wrap that in `ProductionHardeningLayer`), prune (delete `production_hardening.rs` + the 5 unwired primitives + `safe_llm_invoke`), or rebrand (move to experimental crate). **The active `parse_llm_response` path via `FallbackOrchestrator` is the wedge — making `LlmExecutor::new` construct a `FallbackOrchestrator` is the smallest production-wiring step that activates the most of the hardening surface.**
- **`astraweave-rag` vs `astraweave-memory` — two parallel memory subsystems.** [Decisional, **surfaced by §13.8 RAG investigation 2026-05-12 (cross-references §13.1 Memory).**] Factual (verified 2026-05-12): `astraweave-rag` operates on `astraweave-embeddings::Memory{id, text, timestamp, importance, valence, category, entities, context}`. `astraweave-memory` operates on its own types (`MemoryRecord`, hierarchical sensory/working/episodic/semantic, plus `persona::Episode`/`Persona`/`Fact`/`Skill` and the rich `episode::Episode` aka `GameEpisode`). The two subsystems do NOT share a memory representation. Per §13.1 + §13.8 BOTH are dormant for the runtime AI loop — `astraweave-memory` has zero in-engine production consumers (legacy persona only consumed by `astraweave-persona`); `astraweave-rag` has zero non-test `RagPipeline::new` callers (held as field by 5 dormant consumer crates only). The directional question: production-wire one (and prune or rebrand the other), or surface the two-subsystem coexistence as deliberate (e.g. `astraweave-memory` for hierarchical typing + `astraweave-rag` for semantic retrieval), or build an adapter so they compose. Combined dormant LoC: ~24K (12K memory + 12K RAG).
- **Parallel-implementation drift: dual `RagPipeline` structs.** [Decisional / factual, **HIGH-IMPACT finding from §13.8 RAG investigation 2026-05-12; matches the CLAUDE.md "Architecture Drift" anti-pattern called out for dual GOAP, dual TerrainVertex, and dual FastPreview pipelines.**] Factual: `astraweave-rag/src/pipeline.rs:21-51` defines the canonical `RagPipeline` (1693 LoC, full consolidation + forgetting + diversity + injection + LLM summarization). `astraweave-ai/src/rag/pipeline.rs:115` defines a SECOND `RagPipeline` (simpler — only `config + embedding_client + vector_store`, ~360 LoC). The inner duplicate is **orphaned**: `astraweave-ai/src/lib.rs:29-52` does not declare `pub mod rag`, so the file exists on disk but never compiles. The parallel `astraweave-ai/src/persona/manager.rs` follows the same orphaned pattern. Three resolution options: delete the orphaned source files (matching CLAUDE.md Rule "Never build a second implementation of a logical system that already exists"); restore the inner as `pub mod rag` (forcing namespace disambiguation `astraweave_rag::RagPipeline` vs `astraweave_ai::rag::RagPipeline`); migrate distinct features from the inner duplicate back into the canonical (if any exist).
- **Internal `astraweave-rag` duplication: `InjectionConfig` + `InjectionResult` in two modules.** [Decisional / factual, from §13.8 RAG investigation 2026-05-12.] Factual: `lib.rs:111-140` defines `InjectionConfig` used by canonical `RagPipeline`. `injection.rs:11-33` defines a different `InjectionConfig` (different fields) used by the standalone `InjectionEngine` (zero workspace callers). Same pattern for `InjectionResult` (`lib.rs:324-337` vs `injection.rs:49-59`). `pub use injection::*` at `lib.rs:63` creates name-shadowing ambiguity for downstream importers of `astraweave_rag::InjectionConfig`. Resolution: rename the standalone-engine variants or delete the standalone `InjectionEngine` entirely (consistent with its zero-consumer dormancy).
- **`VectorStoreWrapper::get_all_memories` returns empty Vec — consolidation is a no-op.** [Factual / **bug-class finding**, from §13.8 RAG investigation 2026-05-12; sub-question on test coverage resolved by deep investigation 2026-05-12.] `astraweave-rag/src/pipeline.rs:166-170`: simplified implementation. `trigger_consolidation` calls it at `:534` and the empty return makes `ConsolidationEngine::consolidate(vec![])` a no-op. **Verified 2026-05-12: no test catches the no-op.** The only end-to-end test of `RagPipeline::trigger_consolidation` is `astraweave-rag/tests/pipeline_tests.rs:137-173` (`test_consolidation_trigger`), which only asserts that `metrics.consolidations_performed` increments from 0 → 1 after the trigger threshold (`:152, :159, :168`). It does NOT verify that any memories are actually merged, removed, or transformed by the consolidation step. The `ConsolidationEngine::consolidate` logic itself is tested in `astraweave-rag/tests/consolidation_tests.rs` (which calls `engine.consolidate(memories)` directly with hand-built `Vec<Memory>`), but no test exercises the full `RagPipeline → vector_store.get_all_memories() → ConsolidationEngine::consolidate` path. The bug-class question therefore has zero test coverage. Either implement `get_all_memories` correctly, document that the canonical wrapper does not support consolidation, or rewire consolidation to use `VectorStore::get_all_ids()` directly through the wrapper.
- **HNSW advertised but not implemented in `astraweave-embeddings::VectorStore`.** [Factual / cognitive trap, from §13.8 RAG investigation 2026-05-12.] `astraweave-embeddings/src/lib.rs:9` advertises "Fast similarity search using HNSW indexing" and `Cargo.toml:31,42` declares `hnsw_rs` dependency + `hnsw = ["hnsw_rs"]` feature flag (default-on). Actual code at `store.rs:16-29, :42-60` shows a DashMap with no HNSW data structure — search is a linear scan. At 100K-vector capacity + 384-dim cosine distance, latency is ~10-50ms (likely acceptable for game AI) but documentation suggests sub-ms ANN. Options: implement HNSW, remove the claim, or add a "simplified linear-scan" callout to the doc-comments. Same documentation-vs-implementation pattern as the §13.7 LLM Production Hardening "designed but not wired" surface.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new variant is added to `PlannerMode`, `AIControlMode`, `FallbackTier`, `ExtractionMethod`, `ToolVerb`, `BehaviorNode`, `DecoratorType`, or any AI-side `#[non_exhaustive]` enum (touch §3 vocabulary + §8 invariants + §6 if a new conflict appears)
- A new module is added to any of the 8 AI crates (touch §5 file map)
- The Arbiter's mode-transition logic changes (touch §2.2 pipeline + §7 decision log)
- A new LLM client is added (`astraweave-llm/src/*_ollama.rs` family or a local model) (touch §5 + §6 cognitive traps if it triggers a model migration)
- The canonical `Orchestrator` / `OrchestratorAsync` trait surfaces change (high blast radius — touch §3 + §7 + §6)
- `validate_and_execute` gains or loses `EngineError` variants (touch §8 invariants + cross-reference `docs/architecture/ecs_math_core_sdk_foundation.md`)
- A decision in §7 is superseded by new code or audit
- A per-subsystem trace is produced (note the cross-reference in §5)

**Verification process:**
- Spot-check the pipelines in §2 against `core_loop.rs`, `ai_arbiter.rs`, `fallback_system.rs`, `plan_parser.rs`, `validation.rs`
- Verify the file map in §5 against the `pub mod` declarations in each crate's `lib.rs`
- Verify invariants in §8 against the cited line numbers
- Run `cargo test -p astraweave-ai --tests` and `cargo test -p astraweave-llm --tests --features llm_cache,ollama` (excluding network-dependent suites in CI)
- Update the metadata commit hash and date

---

## 13. Subsystem Traces

This section contains deep traces of subsystems within the parent system.
Each subsection covers one subsystem and follows a compact mirror of the
main template structure, scoped to that subsystem's concerns.

### 13.1 Subsystem Trace — Memory

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** Active (code-wise) / Dormant (runtime-wise). The crate compiles, ships 1000+ passing tests, and is internally consistent. The main memory pipeline (`Memory`, `MemoryManager`, `MemoryStorage`, `EpisodeRecorder`, `AdaptiveWeightManager`, pattern/preference/forgetting/consolidation/compression/retrieval/sharing engines) has **zero in-engine production consumers** in `astraweave-ai`, `astraweave-behavior`, `astraweave-render`, or the canonical game loop. The legacy `persona::*` types are actively consumed by `astraweave-persona`.

#### 13.1.1 Role within the parent system

The Memory subsystem is the intended substrate for **companion learning**: it captures temporal episodes of player-companion interaction (combat, dialogue, exploration, puzzle, quest, social), persists them via SQLite (`astraweave-memory/src/storage.rs:46-80` schema), analyzes them to detect playstyle patterns (`pattern_detection.rs::PlaystylePattern`, 6 variants), builds a `PreferenceProfile` per companion, and adapts behavior-tree node weights via `AdaptiveWeightManager` to favor actions aligned with the detected player playstyle. It sits between Stage A (Action, validated via `validate_and_execute`) and Stage P (next-tick Perception) in the parent's §2.1 canonical loop — closing the feedback loop from observed outcomes back into AI behavior.

**Verified 2026-05-12:** the structural pipeline exists end-to-end inside the memory crate, but the runtime wiring **into** the AI loop (Arbiter, Orchestrators, BT tick) **does not exist**. See §13.1.5 and §13.1.7.

#### 13.1.2 Authoritative pipeline

```text
[Stage A (Action) — completion observed]
    │
    │ Gameplay caller (designed: ECS post-simulation system; actual: example code only)
    │ EpisodeRecorder::start_episode(companion_id, EpisodeCategory)
    ▼
[M1 — Episode lifecycle]
    file: astraweave-memory/src/episode_recorder.rs:32-…
    role: HashMap<companion_id, Episode> with auto-flush timer
          (default 60 s — `episode_recorder.rs:37-38`)
    operations: start_episode, record_observation, end_episode (returns Episode)
    │
    ▼
[M2 — Episode → Memory conversion]
    file: astraweave-memory/src/episode.rs (Episode struct + outcome metrics)
    role: An Episode carries category, observations, PlayerAction/CompanionResponse pairs,
          EpisodeOutcome { success_rating, player_satisfaction, companion_effectiveness,
          duration_ms, damage_dealt, damage_taken, resources_used, failure_count }
    │
    ▼
[M3 — Memory storage]
    file: astraweave-memory/src/storage.rs:46-… (SQLite schema)
    role: Episode → Memory{type=Episodic} → INSERT into `memories` table +
          INSERT tag rows into `memory_tags` table
    schema: memories(id PK, memory_type, content_json, metadata_json, embedding_blob,
                     created_at, importance CHECK(0.0..=1.0))
            memory_tags(memory_id FK, tag, PK(memory_id, tag))
            metadata(key PK, value) — schema-version table
    indexes: idx_memory_type, idx_created_at, idx_importance, idx_type_importance, idx_tags
    │
    │ (Also fed by direct MemoryManager::store_memory for non-episode memories)
    │   file: astraweave-memory/src/memory_manager.rs:108-…
    │   capacity limits: Sensory 100 / Working 50 / Episodic 1000 / Semantic 5000 /
    │                    Procedural 500 / Emotional 200 / Social 500 (memory_manager.rs:40-49)
    ▼
[M4 — Pattern detection]
    file: astraweave-memory/src/pattern_detection.rs:11-60
    role: PatternDetector reads episodes from MemoryStorage, scores them against
          PlaystylePattern variants (Aggressive, Cautious, Explorative, Social,
          Analytical, Efficient), emits PatternStrength { pattern, confidence,
          episode_count, avg_quality }
    │
    ▼
[M5 — Preference profile build]
    file: astraweave-memory/src/preference_profile.rs:14-…
    role: ProfileBuilder aggregates pattern strengths into a PreferenceProfile
          with dominant_patterns, preferred_categories (HashMap<EpisodeCategory, f32>),
          optimal_responses (HashMap<String, CompanionActionPreference>),
          learning_confidence, episode_count, converged flag
    │
    ▼
[M6 — Adaptive weight update]
    file: astraweave-memory/src/dynamic_weighting.rs:108-…
    role: AdaptiveWeightManager.update_from_profile(&MemoryStorage):
            1. Build PreferenceProfile via ProfileBuilder
            2. apply_pattern_bonuses(&profile): map PlaystylePattern → BehaviorNodeType
               (Combat/Support/Exploration/Social/Analytical/Defensive) and bump
               pattern_bonus per node, bounded by max_pattern_bonus (default 0.3)
            3. apply_effectiveness_bonuses(&profile): bump effectiveness_bonus
               from preferred_categories, bounded by max_effectiveness_bonus (0.2)
            4. NodeWeight::calculate() per node, clamped [0.0, 1.0]
    output: HashMap<BehaviorNodeType, NodeWeight>
    │
    │ EXPECTED: weights flow back into astraweave-behavior::BehaviorGraph node selection
    │ ACTUAL (verified 2026-05-12): no consumer reads these weights from this manager
    │   See §13.1.5.
    ▼
[M7 — Side branches]
    ConsolidationEngine — astraweave-memory/src/consolidation.rs (24 h temporal-window
        association formation; default association_threshold = 0.7)
    ForgettingEngine — astraweave-memory/src/forgetting.rs (per-MemoryType half-life
        curves: Sensory ~6 h, Working ~1 day, Episodic ~2 weeks, …)
    CompressionEngine — astraweave-memory/src/compression.rs (memory summarization
        after min_age_days = 30, importance_threshold = 0.3)
    RetrievalConfig — astraweave-memory/src/retrieval.rs (weighted semantic + temporal
        + associative search; defaults 0.6 / 0.2 / 0.2)
    SharingConfig — astraweave-memory/src/sharing.rs (cross-agent memory sharing
        with PrivacyLevel and SharingType; default Restricted/Personal)
    BehaviorValidator — astraweave-memory/src/learned_behavior_validator.rs (sandbox
        validation of learned behaviors before promotion)
```

Note: M4-M7 are all subordinate engines reachable through `MemoryStorage` / `MemoryManager` queries. None of them currently runs from inside `astraweave-ai` or `astraweave-behavior` per the §13.1.5 audit.

#### 13.1.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`Memory`** | Top-level memory struct: `{ id, memory_type: MemoryType, content: MemoryContent, metadata: MemoryMetadata, associations: Vec<MemoryAssociation>, embedding: Option<Vec<f32>> }` | `memory_types.rs:6-15` |
| **`MemoryType`** | 7-variant `#[non_exhaustive]` enum: `Sensory`, `Working` (default), `Episodic`, `Semantic`, `Procedural`, `Emotional`, `Social`. The first four are the parent doc's stated hierarchy; the latter three extend it. | `memory_types.rs:18-36` |
| **`MemoryContent`** | `{ text, data: serde_json::Value, sensory_data, emotional_context, context: SpatialTemporalContext }` | `memory_types.rs:40-51` |
| **`EmotionalContext`** | `{ primary_emotion: String, intensity: f32, valence: f32 (-1..=1), arousal: f32 (0..=1) }` | `memory_types.rs:63-69` |
| **`MemoryMetadata`** | `{ created_at, last_accessed, access_count, importance (0..=1), confidence, source: MemorySource, tags, permanent: bool, strength, decay_factor }` | `memory_types.rs:82-104` |
| **`MemorySource`** | `#[non_exhaustive]` 6-variant enum: `DirectExperience`, `Conversation`, `Learning`, `Inference`, `SharedMemory`, `SystemGenerated` | `memory_types.rs:108-116` |
| **`MemoryAssociation`** | Pairwise link between memories with strength + type | `memory_types.rs:120+` |
| **`MemoryCluster`** | Group of related memories for organization | `memory_types.rs` |
| **`MemoryManager`** | In-RAM HashMap-based memory ledger with capacity-per-type config and stats tracking. Separate from `MemoryStorage` (which is SQLite-backed) | `memory_manager.rs:15-…` |
| **`MemoryManagerConfig`** | `{ max_memories_per_type: HashMap<MemoryType, usize>, importance_threshold: f32 (0.3), auto_consolidation: bool, enable_forgetting: bool }` | `memory_manager.rs:28-57` |
| **`MemoryStorage`** | SQLite-backed persistent layer with file-backed and in-memory constructors | `storage.rs:16-43` |
| **`Episode`** (rich) | `{ id, category: EpisodeCategory, observations, outcome: Option<EpisodeOutcome>, ... }` — the temporal interaction record | `episode.rs` |
| **`Episode`** (legacy) | `{ title, summary, tags, ts }` — the legacy persona-side type collided with the rich Episode; renamed `GameEpisode` at crate-root re-export | `persona.rs:5-10` |
| **`EpisodeCategory`** | `#[non_exhaustive]` 6-variant enum: `Combat`, `Dialogue`, `Exploration`, `Puzzle`, `Quest`, `Social` | `episode.rs:21-34` |
| **`EpisodeOutcome`** | `{ success_rating, player_satisfaction, companion_effectiveness, duration_ms, damage_dealt, damage_taken, resources_used, failure_count }` | `episode.rs:100-117` |
| **`ActionResult`** | `#[non_exhaustive] #[must_use]` 4-variant: `Success` (1.0), `Failure` (0.0), `Interrupted` (0.25), `Partial` (0.5) — with `success_multiplier()` | `episode.rs:75-96` |
| **`EpisodeRecorder`** | Per-companion active-episode ledger with auto-flush; designed as an ECS resource in `SystemStage::POST_SIMULATION` (per `episode_recorder.rs:13-16` doc-comment, but no production wiring) | `episode_recorder.rs:17-…` |
| **`PlaystylePattern`** | `#[non_exhaustive]` 6-variant: `Aggressive`, `Cautious`, `Explorative`, `Social`, `Analytical`, `Efficient` | `pattern_detection.rs:14-27` |
| **`PatternStrength`** | `{ pattern: PlaystylePattern, confidence: f32 (0..=1), episode_count, avg_quality }` | `pattern_detection.rs:43-52` |
| **`PreferenceProfile`** | `{ dominant_patterns, preferred_categories: HashMap<EpisodeCategory, f32>, optimal_responses, learning_confidence, episode_count, converged }` | `preference_profile.rs:16-29` |
| **`BehaviorNodeType`** (memory crate) | `#[non_exhaustive]` 6-variant: `Combat`, `Support`, `Exploration`, `Social`, `Analytical`, `Defensive`. **Distinct from `astraweave-behavior::BehaviorNode`** (which is `Sequence`/`Selector`/`Action`/`Condition`/`Decorator`/`Parallel`) | `dynamic_weighting.rs:18-31` |
| **`NodeWeight`** | `{ weight, base_weight, pattern_bonus, effectiveness_bonus, update_count }` with `calculate()` clamping to [0.0, 1.0] | `dynamic_weighting.rs:67-…` |
| **`AdaptiveWeightManager`** | `{ weights: HashMap<BehaviorNodeType, NodeWeight>, detector, builder, learning_rate (0.1), max_pattern_bonus (0.3), max_effectiveness_bonus (0.2) }` | `dynamic_weighting.rs:108-…` |
| **`ForgettingCurve`** | `{ initial_strength, decay_rate, half_life (days), retention_threshold, immune }` — one per `MemoryType` | `forgetting.rs:46-…` (per-type defaults at 52-…) |
| **`SharingType`** / **`PrivacyLevel`** | Permission and privacy taxonomy for cross-agent memory sharing | `sharing.rs` |
| **`MemoryComponent`** (Bevy) | `{ memory_manager: Arc<Mutex<MemoryManager>>, entity_id: String, config: MemoryEntityConfig }` — the **only** AI-side type that uses `bevy_ecs::component::Component`, not `astraweave-ecs` | `components.rs:13-21` (feature `bevy`) |

#### 13.1.4 Files involved

| File | Status | Notes |
|---|---|---|
| `astraweave-memory/src/lib.rs` (69 LoC) | Active | Re-exports all 15 public modules; `#[cfg(feature = "bevy")]` for `components` |
| `astraweave-memory/src/memory_types.rs` (1347 LoC) | Active | Largest source file — all core types |
| `astraweave-memory/src/memory_manager.rs` (934 LoC) | Active | In-RAM ledger |
| `astraweave-memory/src/storage.rs` (550 LoC) | Active | SQLite persistence |
| `astraweave-memory/src/episode.rs` (570 LoC) | Active | Rich episode record |
| `astraweave-memory/src/episode_recorder.rs` (381 LoC) | Active | Per-companion lifecycle |
| `astraweave-memory/src/pattern_detection.rs` (577 LoC) | Active | Playstyle detection |
| `astraweave-memory/src/preference_profile.rs` (501 LoC) | Active | Profile builder |
| `astraweave-memory/src/dynamic_weighting.rs` (427 LoC) | Active (no consumers) | `AdaptiveWeightManager` — see §13.1.5 |
| `astraweave-memory/src/learned_behavior_validator.rs` (547 LoC) | Active | Sandbox validation |
| `astraweave-memory/src/consolidation.rs` (280 LoC) | Active | Association formation |
| `astraweave-memory/src/forgetting.rs` (981 LoC) | Active | Per-type curves |
| `astraweave-memory/src/compression.rs` (754 LoC) | Active | Summarization |
| `astraweave-memory/src/retrieval.rs` (1092 LoC) | Active | Semantic + temporal + associative search |
| `astraweave-memory/src/sharing.rs` (1217 LoC) | Active | Cross-agent sharing |
| `astraweave-memory/src/persona.rs` (759 LoC) | Active (legacy, externally consumed) | Provides `Persona`, `Episode`, `Fact`, `Skill`, `CompanionProfile` to `astraweave-persona` |
| `astraweave-memory/src/components.rs` (~40 LoC visible) | Active (feature `bevy`) | Bevy ECS integration |
| `astraweave-memory/benches/memory_benchmarks.rs` | Active | Criterion bench harness |
| `astraweave-memory/tests/` (8 files / 30391 LoC total counting all tests) | Active | Extensive coverage: `adaptive_behavior_tests`, `behavioral_correctness_tests`, `episode_tests`, `mutation_resistant_comprehensive_tests`, `mutation_tests` (largest at 13280 LoC), `pattern_tests`, `property_memory`, `storage_tests`. 680 `#[test]` attributes total. |

#### 13.1.5 Cross-subsystem touchpoints

**Inside `astraweave-memory` (intra-crate):** all module-to-module wiring works end-to-end. `EpisodeRecorder` → `MemoryStorage`, `MemoryStorage` → `PatternDetector` → `PreferenceProfile` → `AdaptiveWeightManager` is fully traced by the test suites in `astraweave-memory/tests/adaptive_behavior_tests.rs` and `astraweave-memory/tests/pattern_tests.rs`.

**Outside `astraweave-memory` (verified workspace-wide grep 2026-05-12):**

| Consumer | What it imports | Notes |
|---|---|---|
| `astraweave-persona/src/lib.rs:19`, `llm_persona.rs:25` | `astraweave_memory::persona::{Persona, CompanionProfile, Fact, Skill}` | LEGACY persona types only. Does NOT touch the main memory pipeline |
| `astraweave-persona/tests/{sprint3_persona_tests, mutation_resistant_comprehensive_tests, serialization}.rs`, `astraweave-persona/benches/persona_benchmarks.rs` | Same legacy types | Test/bench-only |
| `astraweave-dialogue/src/llm_dialogue.rs:1897` | `astraweave_memory::Persona as BasePersona` | Single line; test/example reference |
| `examples/companion_profile/src/main.rs` | `astraweave_memory` symbols | Example-only |
| `examples/llm_integration/tests/full_integration_test.rs` | `astraweave_memory` symbols | Test-only |

**Critical absence:**
- `astraweave-ai/src/**` — workspace grep finds zero `use astraweave_memory` or `astraweave_memory::` references
- `astraweave-behavior/src/**` — same, zero references

Therefore the memory crate's `AdaptiveWeightManager.weights` map is **never read by any code that ticks a `BehaviorGraph`** (which lives in `astraweave-behavior/src/ecs.rs::behavior_tick_system`). The pipeline diagram at M6 above represents the **intended** flow, not the actual production wiring.

**Reverse-dependency surprise:** `astraweave-memory/Cargo.toml:11-13` declares dependencies on `astraweave-llm`, `astraweave-embeddings`, and `astraweave-rag`. Workspace grep for `use astraweave_llm|use astraweave_embeddings|use astraweave_rag` inside `astraweave-memory/{src,tests,benches}` returned **zero matches** (re-verified 2026-05-12). These appear to be unused Cargo dependencies — no macro or feature-gated path was found that would pull them in transitively in the surveyed files.

**Bevy ECS divergence:** `astraweave-memory/Cargo.toml:23, 27` declares `bevy_ecs = { version = "0.17", optional = true }` and `bevy = ["bevy_ecs"]`. When the `bevy` feature is enabled, `components.rs` uses `bevy_ecs::component::Component`. The rest of the engine uses `astraweave-ecs` (per `docs/architecture/ecs_math_core_sdk_foundation.md`). The memory crate is the only AI-side crate that targets Bevy ECS directly. Verified 2026-05-12: workspace grep across all `*.toml` files for `astraweave-memory.*features.*bevy` or `astraweave_memory.*bevy` returned zero matches — the `bevy` feature is **not enabled by any consumer build**, so `components.rs` is unreachable in practice.

#### 13.1.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| M1 | `MemoryStorage` SQLite schema enforces `importance` in [0.0, 1.0] via SQL CHECK constraint | Yes | `storage.rs:58` `CHECK (importance >= 0.0 AND importance <= 1.0)` |
| M2 | `memory_tags` table has cascading delete on `memory_id` FK | Yes | `storage.rs:71-72` `FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE` |
| M3 | `MemoryManager` enforces capacity-per-type limits before storing | Yes | `memory_manager.rs:108-…` (`store_memory` checks `max_memories_per_type` BEFORE insert) |
| M4 | `NodeWeight.weight` is always clamped to [0.0, 1.0] | Yes | `dynamic_weighting.rs:84, 93-95` `.clamp(0.0, 1.0)` in `new` and `calculate` |
| M5 | `AdaptiveWeightManager.pattern_bonus` is bounded by `max_pattern_bonus` (default 0.3) | Yes | `dynamic_weighting.rs:147, 198-…` `apply_pattern_bonuses` distributes confidence × bonus / N_nodes |
| M6 | `AdaptiveWeightManager.effectiveness_bonus` is bounded by `max_effectiveness_bonus` (default 0.2) | Yes | `dynamic_weighting.rs:148, apply_effectiveness_bonuses` |
| M7 | `ActionResult::success_multiplier()` returns 1.0 / 0.5 / 0.25 / 0.0 for Success / Partial / Interrupted / Failure | Yes | `episode.rs:88-95` |
| M8 | `EpisodeRecorder.flush_interval_secs` defaults to 60 seconds | Yes | `episode_recorder.rs:38` |
| M9 | `EpisodeRecorder::start_episode` for an already-active companion **silently replaces** the prior episode (the doc-comment at `episode_recorder.rs:53-54` says "previous episode is lost") | Yes (doc + code at `:59`) | `active_episodes.insert(companion_id, episode)` — overwrites |
| M10 | `EpisodeRecorder::record_observation` for a companion with no active episode silently no-ops | Yes | `episode_recorder.rs:67-69` `if let Some(episode) = …` guard |
| M11 | Memory-side `BehaviorNodeType` (6 variants) is **distinct from** `astraweave-behavior::BehaviorNode` (6 variants) — same arity but different semantics | Yes (compile-time) | `dynamic_weighting.rs:18-31` vs `astraweave-behavior/src/lib.rs:19-26` |
| M12 | `forgetting::ForgettingCurve` default half-lives are per-type: Sensory 0.25 days, Working 1.0 day, Episodic 14.0 days | Yes | `forgetting.rs:52-…` |
| M13 | All AI-memory types are unsafe-free | Yes (compile-time) | `astraweave-memory/src/lib.rs:1` `#![forbid(unsafe_code)]` |

#### 13.1.7 Open questions (subsystem-specific)

**Closed during this pass (moved to parent §11):**

- *Original parent question:* "AdaptiveWeightManager feedback loop — production-wired?"
  *Resolution:* Closed and superseded by the parent §11 question "Memory subsystem dormancy — production-wire, prune, or rebrand?" — see that entry for the full evidence (`AdaptiveWeightManager` and all main-pipeline symbols have zero in-engine production consumers as of 2026-05-12).

**Subsystem-specific open questions:**

- **Unused Cargo dependencies on `astraweave-llm`, `astraweave-embeddings`, `astraweave-rag`?** [Factual — resolved 2026-05-12.] `astraweave-memory/Cargo.toml:11-13` lists them as direct dependencies; workspace grep inside `astraweave-memory/{src,tests,benches}` for `use astraweave_llm|use astraweave_embeddings|use astraweave_rag` returns zero matches. No macro or feature-gated path was found that imports them. These are genuinely unused Cargo dependencies — dead `Cargo.toml` weight.
- **`bevy_ecs` divergence: status of the `bevy` feature.** [Factual — resolved 2026-05-12.] `Cargo.toml:23, 27` declares an optional `bevy_ecs = "0.17"` dependency under feature `bevy`; `components.rs` is `#[cfg(feature = "bevy")]`-gated. Workspace grep across all `*.toml` files for `astraweave-memory.*features.*bevy` returns zero matches. The feature is not enabled by any consumer build today, so `components.rs` is unreachable in practice.
- **`MemoryManager` vs `MemoryStorage` — same data or different roles?** [Decisional.] Factual: `MemoryManager` (`memory_manager.rs:15-…`) is an in-RAM `HashMap<String, Memory>` ledger with capacity limits. `MemoryStorage` (`storage.rs:16-…`) is a SQLite-backed persistent layer. No code path was traced in this pass that synchronizes them (e.g. "flush MemoryManager to MemoryStorage" or "load MemoryStorage into MemoryManager on startup"). Whether they are intended to coexist as a write-through cache, or whether one supersedes the other, is undecided.
- **Memory-side `BehaviorNodeType` (`Combat`/`Support`/`Exploration`/`Social`/`Analytical`/`Defensive`) vs `astraweave-behavior::BehaviorNode` (`Sequence`/`Selector`/`Action`/`Condition`/`Decorator`/`Parallel`)** — even if the memory-side feedback loop is wired in the future, the type bridge between these two enums (and the `from_pattern` mapping at `dynamic_weighting.rs:47-62`) needs explicit design. [Decisional, conditional on resolving the dormancy question.]
- **`EpisodeRecorder` doc-comment at `episode_recorder.rs:13-16` says "designed to be used as an ECS resource, running in `SystemStage::POST_SIMULATION`"** — but no such system registration was found in `astraweave-ai/src/ecs_ai_plugin.rs` or anywhere else. [Factual — the docstring describes intended use, not actual use.] Whether to add the system registration (re-enabling the feedback loop) or to update the docstring (acknowledging the intended-but-not-wired status) is conditional on the broader dormancy decision.
- **Two collision-named `Episode` types persist** — `persona::Episode` (legacy) and `episode::Episode` (rich, re-exported as `GameEpisode`). Both are public; both compile. The `lib.rs:18-23` rename only resolves the crate-root re-export collision, not the in-module collision. (Already tracked in parent §11; restated here for completeness of the subsystem trace.)
- **Test inventory** — `astraweave-memory/tests/mutation_tests.rs` is 13,280 LoC with 419 `#[test]` attributes. `mutation_resistant_comprehensive_tests.rs` is 2,507 LoC with 186 tests. These are exceptionally large for a single subsystem and exercise the in-crate pipeline thoroughly. If the dormancy question is decided in favor of pruning, the test corpus is a significant artifact to preserve or migrate.

### 13.2 Subsystem Trace — Director

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** Active. Three coexisting director planners (`BossDirector` heuristic, `PhaseDirector` HP-threshold state machine, `LlmDirector` LLM-driven adaptive) plus a feature-gated Veilweaver-specific `OathboundWardenDirector`. The crate is consumed by `veilweaver_slice_runtime`, `examples/phase_director`, and `examples/adaptive_boss`; **not by `astraweave-ai` or `astraweave-behavior`** — Director is a parallel sibling subsystem, not part of the canonical Companion-AI loop.

#### 13.2.1 Role within the parent system

The Director subsystem is the **boss / encounter authoring** AI — distinct from the companion-AI loop traced in the parent's §2.1-§2.2. While companion AI produces `PlanIntent`s of `ActionStep`s validated through `validate_and_execute` (parent §2.1 Stage V), Director produces `DirectorPlan`s of `DirectorOp`s validated through `astraweave-core::validation::apply_director_plan` (`astraweave-core/src/validation.rs:400-457`). The two pipelines share `WorldSnapshot` as input but produce different output types and route through different engine-side validators. Director's role is to mutate **world structure** (spawn waves, fortify rectangles, collapse lines) under explicit `DirectorBudget` constraints, providing dynamic difficulty + boss encounter pacing.

#### 13.2.2 Authoritative pipeline

```text
[Game tick — encounter-active]
    │
    │ Game loop or encounter-system constructs a WorldSnapshot
    │ (boss = enemies[0] by convention — see phase.rs:38, lib.rs:32)
    ▼
[D1 — Director plan production]
    Three coexisting planner shapes (caller picks one):

    ├─ Heuristic — astraweave-director/src/lib.rs:26-73
    │     BossDirector::plan(&snap, &budget) -> DirectorPlan
    │     branches on Manhattan distance to enemies[0]:
    │       > 8 + terrain budget → DirectorOp::Fortify (small rect at midpoint)
    │       ≤ 8 → SpawnWave "minion" × 3 behind player + Collapse line player→midpoint
    │     Pure function; no state, no async, no LLM
    │
    ├─ Phase state machine — astraweave-director/src/phase.rs:36-…
    │     PhaseDirector::step(&snap, &budget) -> PhasePlan { phase_name, telegraphs, director: DirectorPlan }
    │     while next phase's hp_threshold >= boss.hp: advance phase, push telegraph
    │     branches on PhaseSpec.terrain_bias:
    │       > 0.5 + terrain budget → Fortify at midpoint + telegraph
    │       else → SpawnWave "phase_add" × 4 + (optional Collapse for high aggression)
    │     Mutable state: PhaseState { idx, last_switch_t, telegraph: Option<String> }
    │
    ├─ LLM-driven — astraweave-director/src/llm_director.rs:194-…
    │     LlmDirector { llm_client: Arc<dyn LlmClient>, rag_pipeline: Arc<RagPipeline>,
    │                   player_model: Arc<RwLock<PlayerBehaviorModel>>,
    │                   conversation_history: Arc<RwLock<ConversationHistory>>,
    │                   prompt_library: Arc<RwLock<PromptLibrary>>,
    │                   config: LlmDirectorConfig,
    │                   encounter_memory: Arc<RwLock<Vec<TacticOutcome>>> }
    │     LlmDirector::adapt_tactics(&snap, &budget) -> async Result<TacticPlan>
    │     1. PlayerBehaviorModel::analyze_snapshot(&snap) — update aggression/caution/skill/range/adaptability
    │     2. Render prompt via PromptLibrary template "tactic_generation"
    │     3. LlmClient::complete(prompt) → JSON
    │     4. Parse TacticPlan { strategy, reasoning, operations: Vec<DirectorOp>,
    │                            difficulty_modifier, expected_duration, counter_strategies, fallback_plan }
    │     5. Bound difficulty_modifier to LlmDirectorConfig.[min, max] (default 0.3..=1.5)
    │
    └─ Veilweaver-specific — astraweave-director/src/veilweaver_warden.rs:51-…
          OathboundWardenDirector::step(&snap, &budget) -> WardenDirective
          Three phases: Assessment → FulcrumShift → DirectiveOverride
          Tracks storm_choice (Stabilize/Redirect/Unknown), adaptive_ability
          (AntiRangedField/CounterShockAura), last_anchor_left
    │
    ▼
[D2 — ECS integration (LLM-driven path only)]
    file: astraweave-director/src/components.rs (CDirectorState, CTacticExecution, CDirectorMetrics)
    file: astraweave-director/src/systems.rs (DirectorLlmSystem::update — async)
    role: DirectorLlmSystem.adaptation_interval_ms gates LLM calls
          CDirectorState.should_adapt(current_time, adaptation_interval) check
          CDirectorState.record_outcome(TacticOutcome) — feeds PlayerBehaviorModel.update_from_outcome
          CDirectorState.recent_outcomes capped at 10 (components.rs:60-63)
          Difficulty adjustment when ≥3 recent outcomes (systems.rs:54)
    │
    ▼
[D3 — Validation + engine-side execution]
    file: astraweave-core/src/validation.rs:400-457 (apply_director_plan)
    role: For each DirectorOp in plan.ops:
            Fortify { rect }   — checks budget.terrain_edits, calls fill_rect_obs, decrements budget
            Collapse { a, b }  — checks budget.terrain_edits, calls draw_line_obs, decrements budget
            SpawnWave { archetype, count, origin } — checks budget.spawns, spawns count entities
                                                     on Team { id: 2 } with hp=40 in a 3-wide grid offset
                                                     pattern (off.x = origin.x + (k % 3) - 1, off.y = origin.y + (k / 3)),
                                                     decrements budget.spawns by 1 total (NOT per entity)
    Logs each op via the caller's `log` closure
    │
    ▼
[D4 — World mutation]
    role: `apply_director_plan` is the engine-side gate. AI proposes DirectorOps; engine disposes
          (parallel to validate_and_execute for ActionSteps — parent §2.1 Stage V).
```

#### 13.2.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`DirectorOp`** | `#[non_exhaustive]` 3-variant enum: `Fortify { rect: Rect }`, `SpawnWave { archetype: String, count: u32, origin: IVec2 }`, `Collapse { a: IVec2, b: IVec2 }`. Tagged `#[serde(tag = "op")]` for JSON discriminator. **Lives in `astraweave-core`, not `astraweave-director`.** | `astraweave-core/src/schema.rs:1138-1153` |
| **`DirectorBudget`** | `{ traps: i32, terrain_edits: i32, spawns: i32 }`. **Note: `traps` is unused** — no `DirectorOp` variant consumes it. See §13.2.7. | `astraweave-core/src/schema.rs:1155-1160` |
| **`DirectorPlan`** | `{ ops: Vec<DirectorOp> }`. Director's `PlanIntent` analogue. | `astraweave-core/src/schema.rs:1162-1165` |
| **`apply_director_plan(&mut World, &mut DirectorBudget, &DirectorPlan, &mut impl FnMut(String))`** | Engine-side validation + execution gate. Director-side analogue of `validate_and_execute` (parent §3 vocabulary). Mutates `World.obstacles` or spawns Team-2 entities. | `astraweave-core/src/validation.rs:400-457` |
| **`BossDirector`** | Zero-sized struct providing the heuristic planner. `BossDirector::plan(&snap, &budget) -> DirectorPlan`. | `astraweave-director/src/lib.rs:7, 26-73` |
| **`PhaseSpec`** | `{ name: String, hp_threshold: i32, terrain_bias: f32 (0..=1), aggression: f32 (0..=1) }`. Defines a single boss phase's parameters. | `astraweave-director/src/phase.rs:3-8` |
| **`PhaseState`** | `{ idx: usize, last_switch_t: f32, telegraph: Option<String> }`. Mutable state of a `PhaseDirector`. | `astraweave-director/src/phase.rs:10-14` |
| **`PhaseDirector`** | `{ phases: Vec<PhaseSpec>, state: PhaseState }`. HP-threshold state machine with per-step planning. | `astraweave-director/src/phase.rs:15-…` |
| **`PhasePlan`** | `{ phase_name: String, telegraphs: Vec<String>, director: DirectorPlan }`. Output of `PhaseDirector::step`, bundling the underlying plan with phase metadata. | `astraweave-director/src/phase.rs:19-23` |
| **`PlayerBehaviorModel`** | `{ aggression, caution, skill_level, preferred_range, adaptability: f32 (each 0..=1), session_performance: Vec<f32>, preferred_tactics: Vec<String>, weaknesses: Vec<String>, encounter_count: u32 }`. Online-updated player model. Session performance window capped at 20 (llm_director.rs:108-111). | `astraweave-director/src/llm_director.rs:16-43` |
| **`TacticPlan`** | LLM output: `{ strategy, reasoning, operations: Vec<DirectorOp>, difficulty_modifier: f32, expected_duration: u32, counter_strategies: Vec<String>, fallback_plan: Option<String> }`. Richer than the bare `DirectorPlan`. | `astraweave-director/src/llm_director.rs:148-156` |
| **`TacticOutcome`** | Learning record: `{ tactic_used, effectiveness: f32 (0..=1), player_response: String, counter_strategy: String, duration_actual: u32, timestamp: u64 }`. Feeds `PlayerBehaviorModel::update_from_outcome`. | `astraweave-director/src/llm_director.rs:159-167` |
| **`LlmDirectorConfig`** | `{ adaptation_rate: 0.1, min_difficulty: 0.3, max_difficulty: 1.5, learning_enabled: true, creativity_factor: 0.7, context_window_size: 2048 }` (defaults). | `astraweave-director/src/llm_director.rs:170-191` |
| **`LlmDirector`** | The full LLM-driven director with `Arc<RwLock<…>>` state and `Arc<dyn LlmClient>`. Async API: `adapt_tactics(&snap, &budget) -> Result<TacticPlan>`. | `astraweave-director/src/llm_director.rs:194-…` |
| **`CDirectorState`** | ECS component: `{ player_model, current_plan: Option<TacticPlan>, config, recent_outcomes: Vec<TacticOutcome> (cap 10), difficulty_modifier: f32 (default 1.0), last_adaptation_time: u64 }`. | `astraweave-director/src/components.rs:7-21` |
| **`CTacticExecution`** | ECS component tracking active plan execution: `{ plan: TacticPlan, start_time, current_operation: usize, is_paused: bool, metadata: HashMap<String, String> }`. Step-by-step `advance_operation`. | `astraweave-director/src/components.rs:91-159` |
| **`CDirectorMetrics`** | ECS component for telemetry (LLM call rates, success counts). | `astraweave-director/src/components.rs` |
| **`DirectorLlmSystem`** | Async update orchestrator: `{ llm_director: Arc<LlmDirector>, adaptation_interval_ms: u64 }`. `update(&mut CDirectorState, &mut Option<CTacticExecution>, &mut CDirectorMetrics, &WorldSnapshot, &DirectorBudget, current_time_ms) -> async Result<()>`. | `astraweave-director/src/systems.rs:11-58` |
| **`WardenPhase`** | `#[non_exhaustive]` 3-variant: `Assessment`, `FulcrumShift`, `DirectiveOverride`. | `astraweave-director/src/veilweaver_warden.rs:5-11` |
| **`StormChoice`** | `#[non_exhaustive]` 3-variant: `Unknown`, `Stabilize`, `Redirect`. Pre-encounter arena modifier. | `astraweave-director/src/veilweaver_warden.rs:13-19` |
| **`AdaptiveAbility`** | `#[non_exhaustive]` 2-variant: `AntiRangedField`, `CounterShockAura`. Warden's response adaptation. | `astraweave-director/src/veilweaver_warden.rs:21-26` |
| **`WardenDirective`** | `{ phase, arena_modifier: Option<StormChoice>, adaptive_ability: Option<AdaptiveAbility>, plan: DirectorPlan, telegraphs: Vec<String> }`. | `astraweave-director/src/veilweaver_warden.rs:28-35` |
| **`OathboundWardenDirector`** | Veilweaver-specific 3-phase boss. `step(&snap, &budget) -> WardenDirective`. | `astraweave-director/src/veilweaver_warden.rs:37-…` (feature `veilweaver_slice`) |

#### 13.2.4 Files involved

| File | Status | Notes |
|---|---|---|
| `astraweave-director/src/lib.rs` (374 LoC) | Active | `BossDirector` + re-exports + 16 inline tests |
| `astraweave-director/src/llm_director.rs` (967 LoC) | Active | `LlmDirector` + `PlayerBehaviorModel` + `TacticPlan`/`TacticOutcome` |
| `astraweave-director/src/components.rs` (660 LoC) | Active | `CDirectorState`, `CTacticExecution`, `CDirectorMetrics` ECS components |
| `astraweave-director/src/phase.rs` (568 LoC) | Active | `PhaseDirector` HP-threshold state machine |
| `astraweave-director/src/systems.rs` (445 LoC) | Active | `DirectorLlmSystem` async orchestrator |
| `astraweave-director/src/veilweaver_warden.rs` (277 LoC) | Active (feature `veilweaver_slice`) | Veilweaver Oathbound Warden |
| `astraweave-director/tests/mutation_resistant_comprehensive_tests.rs` (1455 LoC) | Active | 95 `#[test]` attributes |
| `astraweave-director/benches/director_adversarial.rs` (1005 LoC) | Active | Criterion bench |
| `astraweave-core/src/schema.rs:1138-1165` | Active | Canonical `DirectorOp`, `DirectorBudget`, `DirectorPlan` types — lives in `astraweave-core`, not `astraweave-director` |
| `astraweave-core/src/validation.rs:400-457` | Active | `apply_director_plan` engine-side gate — Director's analogue of `validate_and_execute` |
| Test totals: 185 `#[test]` attrs across the 6 source files + the comprehensive test file. Source `#[test]` distribution: `lib.rs` (16), `llm_director.rs` (16), `components.rs` (26), `phase.rs` (27), `systems.rs` (5), comprehensive (95). | Active | |

**Dependencies (per `astraweave-director/Cargo.toml:11-22`):** `astraweave-core`, `astraweave-llm`, `astraweave-rag`, `astraweave-context`, `astraweave-prompts`, `tokio`, `tracing`, `futures`, plus `anyhow`/`serde`/`serde_json` (workspace). The two underlined deps (`astraweave-context`, `astraweave-prompts`) are LLM-infrastructure crates consumed for prompt construction. Verified 2026-05-12: `astraweave-context` was added to parent §5 in the 2026-05-12 RAG subsystem trace pass (see "astraweave-rag, astraweave-embeddings, astraweave-context — RAG stack" section). `astraweave-prompts` (12 source files: lib, library, template, engine, context, compat, helpers, loader, optimization, sanitize, terrain_prompts, mutation_tests) is consumed by 5 dormant LLM-enhanced subsystems: `astraweave-director/src/llm_director.rs:11-12`, `astraweave-dialogue/src/llm_dialogue.rs:23-25`, `astraweave-persona/src/llm_persona.rs:19`, `astraweave-coordination/src/{narrative_coherence,world_events}.rs:12-13`. Whether `astraweave-prompts` deserves its own parent-§5 entry (mirroring the RAG-stack treatment) is a decisional structuring question.

#### 13.2.5 Cross-subsystem touchpoints

**Inside the parent AI system:**

- **`astraweave-core::WorldSnapshot` (parent §3 vocabulary):** consumed by all three Director planner shapes via `&WorldSnapshot` argument. Director reads `snap.player.pos`, `snap.enemies[0].pos`/`.hp`, and `snap.me.pos`. Director treats `enemies[0]` as the boss by convention (`phase.rs:38`, `lib.rs:32`); this is undocumented except by code position.
- **`astraweave-llm::LlmClient` (parent §3 vocabulary):** `LlmDirector` holds `Arc<dyn LlmClient>` and calls `LlmClient::complete(prompt)` for tactic generation (`llm_director.rs:195`). Director does **not** route through `FallbackOrchestrator` (parent §2.3) — it uses `LlmClient` directly, so it does not benefit from the 4-tier fallback chain. Verified 2026-05-12 at `llm_director.rs:338-346`: no `FallbackOrchestrator` import, direct `llm_client.complete().await` followed by `serde_json::from_str` (line 345). Whether this is intentional or a gap remains undecided.
- **`astraweave-rag::RagPipeline`:** `LlmDirector.rag_pipeline: Arc<RagPipeline>` (`llm_director.rs:196`). Used for prompt context retrieval. Not detailed in this trace.
- **`astraweave-context::ConversationHistory`:** `LlmDirector.conversation_history: Arc<RwLock<ConversationHistory>>` (`llm_director.rs:198`). Multi-turn prompt context. Not detailed in this trace.
- **`astraweave-prompts::{PromptLibrary, PromptTemplate}`:** `LlmDirector.prompt_library` (`llm_director.rs:199`). Two templates registered: `"tactic_generation"` (`llm_director.rs:219-251`) and `"difficulty_adjustment"` (`llm_director.rs:254-…`).

**Outside the parent AI system (engine-level):**

- **`astraweave-core::validation::apply_director_plan`** (`astraweave-core/src/validation.rs:400-457`): the engine-side gate. Caller passes `&mut World`, `&mut DirectorBudget`, `&DirectorPlan`, `&mut impl FnMut(String)` for logging. Director Op execution: `Fortify` calls `fill_rect_obs(&mut w.obstacles, *rect)` decrementing `budget.terrain_edits`; `Collapse` calls `draw_line_obs(&mut w.obstacles, *a, *b)`; `SpawnWave` spawns `count` entities on Team {id: 2} at the configured offset pattern, decrementing `budget.spawns` by 1 total (NOT per entity). Skipped ops are logged.
- **`veilweaver_slice_runtime/src/boss_encounter.rs`:** the only production consumer of `OathboundWardenDirector`. Wires the Warden into the gameplay-side encounter lifecycle. Emits `BossEncounterEvent::{EncounterStarted, PhaseChanged, Telegraph, Defeated}`.

**Critical absence (verified workspace-wide grep 2026-05-12):**

- `astraweave-ai/src/**` — workspace grep finds zero `use astraweave_director` or `astraweave_director::` references. The canonical Companion-AI loop in `sys_ai_planning` (`astraweave-ai/src/ecs_ai_plugin.rs:45-…`) does **not** invoke any Director planner. Director is a parallel sibling subsystem, not part of the Companion-AI pipeline.
- `astraweave-behavior/src/**` — same, zero references.
- All non-self consumers identified (workspace grep): `veilweaver_slice_runtime` (Warden only), `examples/phase_director` (PhaseDirector example), `examples/adaptive_boss` (BossDirector + Warden example). No middleware/aggregator wraps Director with Arbiter, GOAP, or BT — the two AI systems are unconnected at the code level.

#### 13.2.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| D1 | All Director planners read `snap.enemies[0]` as the boss when present | Yes (convention) | `lib.rs:32-35`, `phase.rs:38`, `veilweaver_warden.rs:…`. When `snap.enemies` is empty, BossDirector synthesizes a default target at `(ppos.x + 6, ppos.y)` (`lib.rs:32-35`) |
| D2 | `BossDirector::plan` switches behavior at Manhattan distance threshold = 8 (player ↔ first enemy) | Yes | `lib.rs:37-38, 70-71`. Tested at `lib.rs:298-340` (threshold tests). Distance = 8 falls into the `else` branch (Collapse + SpawnWave); distance > 8 → Fortify |
| D3 | `PhaseDirector::step` advances phase index strictly when `boss.hp ≤ next phase's hp_threshold`, never reverses | Yes | `phase.rs:40-49` while-loop only increments `state.idx` |
| D4 | `apply_director_plan` decrements `budget.spawns` by 1 per SpawnWave op (NOT per entity spawned) | Yes | `astraweave-core/src/validation.rs:455` — single `budget.spawns -= 1;` after the inner spawn loop |
| D5 | `apply_director_plan` skips ops (does NOT mutate world) when the relevant budget field is `≤ 0` | Yes | `validation.rs:409-411, 421-423, 437-439` — checked and logged "SKIPPED (budget)" |
| D6 | `DirectorBudget.traps` field is unused (no `DirectorOp` variant consumes it) | Yes | `astraweave-core/src/schema.rs:1140-1153` — only `Fortify`/`SpawnWave`/`Collapse` variants exist. Tests like `lib.rs:111-117` set `traps: 0` and `lib.rs:119-125` set `traps: 5` but no code path reads them |
| D7 | `LlmDirector.config.{min_difficulty, max_difficulty}` defaults to `[0.3, 1.5]` and the LLM-emitted `difficulty_modifier` is bounded to this range | Doc-only (verified 2026-05-12: `adapt_tactics` at `llm_director.rs:294-359` does NOT clamp `difficulty_modifier`; `validate_plan` at `:518-547` only validates `operations` budget. Only `adjust_difficulty` at `:411-414` clamps `new_difficulty.max(min).min(max)` — and that's a separate function from `adapt_tactics`.) | `llm_director.rs:183-184, :411-414` |
| D8 | `PlayerBehaviorModel.session_performance` window capped at 20 entries (FIFO eviction) | Yes | `llm_director.rs:108-111` — `if self.session_performance.len() > 20 { self.session_performance.remove(0); }` |
| D9 | `PlayerBehaviorModel.preferred_tactics` capped at 5, `weaknesses` capped at 3 | Yes | `llm_director.rs:126-130` (preferred_tactics cap 5), `llm_director.rs:134-137` (weaknesses cap 3) |
| D10 | `CDirectorState.recent_outcomes` capped at 10 entries | Yes | `components.rs:60-63` |
| D11 | `OathboundWardenDirector::step` is deterministic given identical `(snap, budget, internal state)` inputs | Yes | No randomness in `veilweaver_warden.rs`; all branches are deterministic on `snap` + `budget` |
| D12 | Director Cargo features default to empty; `veilweaver_slice` opts into `veilweaver_warden` module | Yes | `astraweave-director/Cargo.toml:27-29` |
| D13 | Director crate is unsafe-free | Yes | `astraweave-director/src/lib.rs:1` `#![forbid(unsafe_code)]` |

#### 13.2.7 Open questions (subsystem-specific)

**No parent-level Open Questions about Director were resolved during this pass** (the parent had no Director-specific questions in §11).

**Subsystem-specific open questions:**

- **`DirectorBudget.traps` is declared but unused — design intent or stalled work?** [Decisional / factual.] Factual state (2026-05-12): `astraweave-core/src/schema.rs:1157` declares `pub traps: i32` in `DirectorBudget`. Workspace grep for `budget.traps` and `DirectorOp::Trap` finds no readers / no variant. Tests set the field to assert struct construction but no planner emits a trap op. Whether to add a `DirectorOp::PlaceTrap` variant (filling out the design) or to remove the field (cleaning up) is undecided.
- **`LlmDirector` does NOT route through `FallbackOrchestrator` (parent §2.3 4-tier chain).** [Decisional / factual.] Factual state: `llm_director.rs:195-…` holds `Arc<dyn LlmClient>` directly and calls `LlmClient::complete` without the 4-tier fallback chain. Whether this is intentional (Director failures degrade to no plan, which is acceptable for boss-encounter pacing) or a hardening gap is undecided.
- **`apply_director_plan.SpawnWave` decrements budget by 1 per OP, not per entity (D4 above).** [Decisional.] A `SpawnWave { count: 10 }` op consumes the same budget as `SpawnWave { count: 1 }`. Whether this is intentional (one "wave" is one decision) or a budgeting oversight is undecided.
- **The "boss = enemies[0]" convention is undocumented in non-code locations** (D1 above). [Decisional / factual.] Code position determines it. Whether to add an explicit role marker (e.g. `EnemyState.is_boss: bool` or a `BossEntity` resource) is undecided.
- **`DirectorLlmSystem::update` takes raw mutable refs, not `&mut ecs::World` (5 of 6 args are `&mut Component` or `&mut Option<Component>`).** [Factual / decisional.] Factual: `systems.rs:25-32` declares the signature. This means the system is **not** a standard `astraweave-ecs::SystemFn` (which is `fn(&mut World)`); a caller must extract components before calling. Whether to wrap it as an ECS-compatible system or to document the calling pattern explicitly is undecided.
- **`adapt_tactics` LLM call path JSON-parsing robustness.** [Factual — resolved 2026-05-12.] `LlmDirector::adapt_tactics` parses LLM responses via `serde_json::from_str(&response)` directly at `llm_director.rs:345`. Same pattern for `adjust_difficulty` at `:408`. No use of `astraweave-llm::plan_parser::parse_llm_response` (the 5-strategy chain) and no use of `FallbackOrchestrator`. Director's parsing is naive: any malformed JSON (extra prose, markdown fences, missing fields) causes immediate parse error with no resilience retry.
- **`OathboundWardenDirector` is the only Veilweaver-specific director — does this pattern scale?** [Decisional.] If new bosses each get their own `*Director` struct under `veilweaver_warden.rs`-style files, the crate grows by N. Whether to keep one-file-per-boss or to introduce a trait-based director registry is a future-direction question Andrew makes.
- **Test surface vs production surface mismatch.** [Factual / informational.] `astraweave-director/tests/mutation_resistant_comprehensive_tests.rs` has 95 `#[test]` attributes, and source files add 90 more. Total ~185 tests for a crate with 3 in-engine consumers (`veilweaver_slice_runtime` + 2 examples). The crate is internally well-tested but its integration surface is narrow — like the Memory subsystem (§13.1) but to a lesser degree. Whether the test corpus matches the production importance is a sizing question.

### 13.3 Subsystem Trace — NPC

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** Active (code-wise) / Isolated (architecturally). The crate compiles and ships 113 passing tests. Its sole in-engine production consumer is `examples/npc_town_demo`. Workspace grep confirms zero references from `astraweave-ai` or `astraweave-behavior`. The crate is **fully self-contained**: it has its own parallel AI vocabulary (`NpcAction`/`NpcPlan`/`NpcWorldView`/`LlmAdapter`/`CommandSink`/`NpcMode`/`EmoteKind`/`Role`), zero imports of canonical AI types (`WorldSnapshot`, `PlanIntent`, `ActionStep`, `Orchestrator`, `astraweave-core`, `astraweave-ai`, `astraweave-behavior`, `astraweave-llm`), and Cargo dependencies that point at `astraweave-physics` + `astraweave-audio` + `astraweave-gameplay` instead of the AI substrate.

#### 13.3.1 Role within the parent system

The NPC subsystem provides **town/civilian/quest-giver AI** with dialogue, role-based behavior (Merchant/Guard/Civilian/QuestGiver), per-NPC profiles loaded from TOML, scheduled activities, and direct physics-driven movement. It sits alongside the Companion-AI loop (parent §2.1) and the Director boss subsystem (parent §13.2) as a **third parallel AI subsystem**, not part of any of them. Unlike both, it does NOT route plans through `astraweave-core::validation::validate_and_execute` or `apply_director_plan`; instead it executes `NpcAction`s through a `CommandSink` trait whose default implementation (`EngineCommandSink`) directly drives `PhysicsWorld::control_character` (`runtime.rs:26-34`) and `AudioEngine::play_voice_beep` / `play_sfx_3d_beep` (`runtime.rs:38-40, 49`).

#### 13.3.2 Authoritative pipeline

```text
[Game tick — caller (game loop or example main)]
    │
    │ Construct NpcWorldView per NPC: { time_of_day, self_pos, player_pos, player_dist,
    │                                   nearby_threat, location_tag }
    │ file: astraweave-npc/src/sense.rs:3-11
    │
    │ Either:
    │   (a) Player utterance path: NpcManager::handle_player_utterance(npc_id, &view, utter)
    │       file: runtime.rs:146-164
    │   (b) Idle/scheduled path: NpcManager::update(dt, glue, views) every frame
    │       file: runtime.rs:100-144
    ▼
[N1 — Planning via LlmAdapter trait]
    file: astraweave-npc/src/llm.rs:11-18
    trait: LlmAdapter::plan_dialogue_and_behaviour(&NpcProfile, &NpcWorldView, Option<&str>)
                                                   -> Result<NpcPlan>
    Default impl: MockLlm (llm.rs:22-130) — hand-coded heuristics branching on Role:
      Role::Merchant  — recognises "buy"/"shop" → Say + OpenShop; else greeting + Emote(Nod)
      Role::Guard     — view.nearby_threat → Say + CallGuards
                        utterance "danger"/"help" → CallGuards
                        else 50% random patrol step via MoveTo
      Role::Civilian  — "hello" → Say + Emote(Wave) else "Sorry—busy day"
      Role::QuestGiver — "quest"/"work" → Say + GiveQuest("q_tutorial") else flavour line
    Output: NpcPlan { actions: Vec<NpcAction> }
    │
    ▼
[N2 — Plan queuing on Npc.pending]
    file: runtime.rs:159 (utterance path) or implicit (idle path uses no LLM)
    Npc.pending.extend(plan.actions)
    npc.mode = NpcMode::Conversing (on utterance path)
    npc.cooldown_talk = 0.5 (utterance gate at runtime.rs:153-155)
    │
    ▼
[N3 — Per-tick execution]
    file: runtime.rs:100-144
    For each NPC: pending.first() popped per tick (one action per tick)
                  If no pending: idle micro-behavior (Guard: step aside when player_dist < 2.0)
    │
    ▼
[N4 — Action execution via CommandSink (NOT via validate_and_execute)]
    trait: CommandSink (runtime.rs:12-18) — 5 methods:
             move_character(BodyId, dir, speed)
             say(speaker, text)
             open_shop(NpcId)
             call_guards(pos, reason)
             give_quest(NpcId, quest_id)
    Default impl: EngineCommandSink { phys: &mut PhysicsWorld, audio: &mut AudioEngine }
    file: runtime.rs:20-55
    role: Execute the NpcAction by directly mutating physics + audio state
    │
    ▼
[N5 — World mutation through physics/audio]
    PhysicsWorld::control_character(body, vel, dt, jump=false)  — fixed-dt 1/60 (runtime.rs:33-34)
    AudioEngine::play_voice_beep(text.len())                    — placeholder VO
    AudioEngine::play_sfx_3d_beep(...)                          — guards beep
    println! for shop / quest UI placeholders (runtime.rs:43, 53)
```

Known limitations documented in code:
- `NpcAction::MoveTo`: comment at `runtime.rs:175-180` notes "we cannot query position from CommandSink, so move toward target directly" and uses a placeholder `Vec3::new(pos.x - 0.0, 0.0, pos.z - 0.0)` rather than computing a real direction vector.
- `NpcAction::CallGuards`: comment at `runtime.rs:188-190` uses a placeholder position `Vec3::ZERO` for the same reason.
- `NpcManager::body_pos` at `runtime.rs:195-200` is `#[allow(dead_code)]` with a comment "we cannot query position via CommandSink. In your integration, pull from PhysicsWorld or World."

#### 13.3.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`NpcId`** | `pub type NpcId = u64`. Distinct from `astraweave-ecs::Entity` (which is `{ id: u32, generation: u32 }` — see foundation trace) and `astraweave-core::Entity` (which is `u32`). | `runtime.rs:10` |
| **`Npc`** | `{ id: NpcId, profile: NpcProfile, body: BodyId, mode: NpcMode, pending: Vec<NpcAction>, cooldown_talk: f32 }`. The per-instance NPC record. | `runtime.rs:57-64` |
| **`NpcManager`** | `{ next_id: NpcId, npcs: HashMap<NpcId, Npc>, planner: Box<dyn LlmAdapter> }`. Owns all NPCs + the planning trait object. | `runtime.rs:66-70` |
| **`NpcAction`** | `#[non_exhaustive]` 6-variant enum: `Say { text }`, `MoveTo { pos: Vec3, speed }`, `Emote { kind: EmoteKind }`, `OpenShop`, `GiveQuest { id }`, `CallGuards { reason }`. NPC's parallel to canonical `ActionStep`. | `behavior.rs:13-22` |
| **`NpcPlan`** | `{ actions: Vec<NpcAction> }`. NPC's parallel to canonical `PlanIntent`. No `plan_id` field. | `behavior.rs:24-27` |
| **`NpcMode`** | `#[non_exhaustive]` 6-variant: `Idle`, `Patrolling`, `Working`, `Conversing`, `Flee`, `Combat`. Per-NPC state. | `behavior.rs:29-38` |
| **`EmoteKind`** | `#[non_exhaustive]` 4-variant: `Wave`, `Nod`, `Shrug`, `Point`. | `behavior.rs:4-11` |
| **`NpcWorldView`** | `{ time_of_day: f32 (0..24), self_pos: Vec3, player_pos: Option<Vec3>, player_dist: Option<f32>, nearby_threat: bool, location_tag: Option<String> }`. NPC's parallel to canonical `WorldSnapshot`; an NPC-relevant subset of state with builder methods (`new`, `with_player`, `with_threat`, `with_location`). | `sense.rs:3-51` |
| **`LlmAdapter`** trait | `fn plan_dialogue_and_behaviour(&NpcProfile, &NpcWorldView, Option<&str>) -> Result<NpcPlan>`. `Send + Sync`. NPC's parallel to canonical `Orchestrator`. **Distinct from** `astraweave-llm::llm_adapter` (which is a module of utility functions) and from `astraweave-llm::LlmClient` (which is the canonical LLM-call trait). | `llm.rs:10-18` |
| **`MockLlm`** | Zero-sized hand-coded heuristic implementing `LlmAdapter`. **Despite the filename `llm.rs`, contains no real LLM client integration.** Role-based branches with `rand::rng()` for tiny patrol jitter. | `llm.rs:22-130` |
| **`CommandSink`** trait | `move_character`/`say`/`open_shop`/`call_guards`/`give_quest`. NPC's parallel to engine-side validation gates — but applies effects directly to physics/audio rather than going through `validate_and_execute` schema validation. | `runtime.rs:12-18` |
| **`EngineCommandSink<'a>`** | Default `CommandSink` impl backed by `&mut PhysicsWorld` + `&mut AudioEngine`. | `runtime.rs:20-55` |
| **`NpcProfile`** | `{ id: String, role: Role, persona: Persona, memory: Memory, home: [f32; 3], schedule: Vec<ScheduleEntry> }`. Authored as TOML, loaded via `load_profile_from_toml_str`. | `profile.rs:39-49` |
| **`Persona`** (NPC) | `{ display_name, traits: Vec<String>, backstory, voice_speaker: Option<String> }`. **Distinct from** `astraweave-memory::persona::Persona` (which is the legacy memory-crate Persona type — see §13.1 vocabulary). | `profile.rs:14-22` |
| **`Memory`** (NPC) | `{ facts: Vec<String>, episodes: Vec<String> }`. **Distinct from** the entire `astraweave-memory` crate hierarchy. NPC's Memory is two flat string lists, not the rich hierarchical-memory pipeline. | `profile.rs:24-30` |
| **`Role`** | `#[non_exhaustive]` 4-variant: `Merchant`, `Guard`, `Civilian`, `QuestGiver`. Drives the `MockLlm` heuristic dispatch. | `profile.rs:5-12` |
| **`ScheduleEntry`** | `{ hour: u8 (0..23), action: String, target: [f32; 3] }`. Per-NPC daily schedule. Verified 2026-05-12: workspace grep for `schedule:` inside `astraweave-npc/src` returned only construction sites (`profile.rs:48` field decl, `profile.rs:80` default, `profile.rs:183` test, `llm.rs:152` empty default, `runtime.rs:298` test) — **zero read sites**. Authored but not consumed by `NpcManager::update`. | `profile.rs:32-37` |
| **`load_profile_from_toml_str`** | `fn(&str) -> Result<NpcProfile>` via `toml::from_str`. | `profile.rs:57-59` |

#### 13.3.4 Files involved

All listed in the parent §5 enrichment above. To summarize source-side test distribution:

| File | LoC | `#[test]` count | Role |
|---|---:|---:|---|
| `lib.rs` | 12 | 0 | Module declarations + glob re-exports |
| `runtime.rs` | 656 | 17 | `Npc`, `NpcManager`, `CommandSink`, `EngineCommandSink`, execute_action |
| `llm.rs` | 436 | 15 | `LlmAdapter` trait + `MockLlm` heuristic |
| `profile.rs` | 251 | 13 | `NpcProfile`, `Persona`, `Memory` (NPC), `Role`, `ScheduleEntry`, TOML loader |
| `behavior.rs` | 197 | 14 | `NpcAction`, `NpcPlan`, `NpcMode`, `EmoteKind` |
| `sense.rs` | 131 | 7 | `NpcWorldView` |
| `tests/mutation_resistant_comprehensive_tests.rs` | 797 | 47 | Mutation-killing integration tests |
| `benches/npc_adversarial.rs` | 1374 | 0 | Criterion bench harness with adversarial inputs |
| **Totals** | **3854** | **113** | |

**Dependencies (per `astraweave-npc/Cargo.toml`):** `astraweave-physics` (BodyId, PhysicsWorld), `astraweave-audio` (AudioEngine), `astraweave-gameplay` (per comment: "for dialogue/quests types if needed"). **Notably absent:** `astraweave-core`, `astraweave-ai`, `astraweave-behavior`, `astraweave-llm`, `astraweave-memory`. The NPC crate does NOT depend on any AI substrate crate.

#### 13.3.5 Cross-subsystem touchpoints

**Inside the parent AI system:** None. Workspace-wide grep (2026-05-12) for canonical AI types inside `astraweave-npc/src`:
- `WorldSnapshot`, `PlanIntent`, `ActionStep`, `Orchestrator` — zero matches
- `astraweave_core`, `astraweave_ai`, `astraweave_behavior`, `astraweave_llm` — zero matches

The NPC crate **does not import any canonical AI types** and is therefore not connected at the code level to:
- The canonical Companion-AI loop in `sys_ai_planning` (parent §2.1)
- The `AIArbiter` and its three control modes (parent §2.2)
- The 4-tier `FallbackOrchestrator` (parent §2.3)
- The `tool_sandbox` + `tool_guard` + `validate_and_execute` defense-in-depth chain (parent §6)
- `AiPlanningPlugin` / the `"ai_planning"` ECS stage
- `LlmClient` / `LlmExecutor` / `AsyncTask`
- The `astraweave-memory` pipeline (already-traced §13.1)
- The `astraweave-director` planners (already-traced §13.2)

**Outside the parent AI system (engine-level):**

| Touchpoint | Interface | File |
|---|---|---|
| `astraweave-physics::PhysicsWorld` | `add_character(pos, half_extents) -> BodyId`; `control_character(body, vel, dt, jump)` | Consumed by `runtime.rs:33-34, 82-84` |
| `astraweave-audio::AudioEngine` | `play_voice_beep(text.len())`, `play_sfx_3d_beep(speaker, pos, range, gain, pitch)` | Consumed by `runtime.rs:38-40, 49` |
| `astraweave-gameplay` | Cargo dep with comment "for dialogue/quests types if needed". Verified 2026-05-12: workspace grep inside `astraweave-npc/src` for `astraweave_gameplay::` returned zero matches — dep is currently unused. | `Cargo.toml:18` |
| `examples/npc_town_demo/src/main.rs` | Only in-engine production consumer. Wires `NpcManager::new(Box::new(MockLlm))` + `spawn_from_profile` + `EngineCommandSink` together with TOML profile loading | `examples/npc_town_demo/src/main.rs:14-55` |

**Critical absence (verified 2026-05-12):**
- Zero `use astraweave_npc` references in `astraweave-ai`, `astraweave-behavior`, `astraweave-llm`, `astraweave-memory`, `astraweave-director`, `astraweave-dialogue`, `astraweave-coordination`. The earlier grep that returned 5 `astraweave-ai` files (`ai_arbiter.rs`, `orchestrator.rs`, three test files) was a false positive — those files contain `MockLlm` / `LlmAdapter` as **name-collisions** with NPC's symbols, not actual cross-crate uses. The Arbiter's internal `MockLlm` is unrelated to `astraweave-npc::MockLlm`.

#### 13.3.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| N1 | `NpcManager::update` executes at most one pending action per NPC per tick | Yes | `runtime.rs:114-121` — `pending.first()` cloned, then `pending.remove(0)`. No loop |
| N2 | `cooldown_talk` decays at `dt` per tick, clamped to `≥ 0.0` | Yes | `runtime.rs:111` — `(npc.cooldown_talk - dt).max(0.0)` |
| N3 | `handle_player_utterance` is gated by `cooldown_talk > 0.0` (silently returns) | Yes | `runtime.rs:153-155` |
| N4 | After a successful utterance plan, `cooldown_talk` is reset to `0.5` and `mode` to `NpcMode::Conversing` | Yes | `runtime.rs:160-162` |
| N5 | Guards step aside when `player_dist < 2.0` during idle | Yes | `runtime.rs:124-136` |
| N6 | `PhysicsWorld::control_character` is called with a fixed dt of `1.0/60.0` regardless of the caller's tick dt | Yes | `runtime.rs:33-34`. The `dt` parameter of `NpcManager::update` is NOT propagated to `control_character` |
| N7 | `MoveTo` direction calculation uses a documented placeholder (`Vec3::new(pos.x - 0.0, 0.0, pos.z - 0.0)`) because `CommandSink` does not expose position queries | Yes (code) | `runtime.rs:175-180` (comment documents the limitation) |
| N8 | `CallGuards` position parameter is a placeholder `Vec3::ZERO` for the same reason | Yes (code) | `runtime.rs:188-190` |
| N9 | Default NPC capsule half-extents are `(0.4, 0.9, 0.4)` for all spawned NPCs | Yes | `runtime.rs:84` |
| N10 | Newly spawned NPCs start in `NpcMode::Idle` with empty `pending` and `cooldown_talk = 0.0` | Yes | `runtime.rs:88-96` |
| N11 | `NpcProfile.persona.voice_speaker` is `Option<String>` — `MockLlm` does not currently read it, but it's authored in TOML | Yes (compile-time) | `profile.rs:21` |
| N12 | `NpcProfile.schedule: Vec<ScheduleEntry>` is loaded from TOML but never read by `NpcManager::update` | Yes (workspace grep, verified 2026-05-12) | Zero read sites in `astraweave-npc/src` for `schedule:` — only construction at `profile.rs:48,80,183`, `llm.rs:152`, `runtime.rs:298`. |
| N13 | `MockLlm`'s patrol step uses `rand::rng().random_range(-1.0..1.0)` for jitter — this introduces non-determinism into the NPC subsystem | Yes | `llm.rs:80-88`. **Distinct from the canonical AI loop's RNG-determinism story** which uses `astraweave-ecs::Rng` (parent §3 vocabulary) |
| N14 | NPC crate is unsafe-free | Yes | `astraweave-npc/src/lib.rs:1` `#![forbid(unsafe_code)]` |

#### 13.3.7 Open questions (subsystem-specific)

**No parent-level Open Questions about NPC were resolved during this pass** (the parent had no NPC-specific questions in §11).

**Subsystem-specific open questions:**

- **NPC subsystem isolation — production-wire, prune, or keep as standalone?** [Decisional.] Factual state (verified 2026-05-12): NPC has zero `use astraweave_*` of AI substrate crates (core/ai/behavior/llm/memory/director) and zero in-engine consumers outside `examples/npc_town_demo`. The crate is fully self-contained with its own parallel AI vocabulary. Whether this isolation is intentional (NPCs are deliberately a side-channel AI subsystem that bypasses the canonical loop) or a gap (NPCs should be integrated with the Companion-AI loop, share the `Orchestrator` trait, route through `validate_and_execute`) is a directional decision. The pattern mirrors the Memory dormancy finding (§13.1) but at a deeper structural level — Memory at least shares the canonical AI types; NPC has rebuilt them.
- **`llm.rs` filename vs. content** [Decisional / informational.] Factual: `astraweave-npc/src/llm.rs` contains the `LlmAdapter` trait and a `MockLlm` hand-coded heuristic. No real LLM client integration code. The filename promises LLM-driven NPC dialogue; the implementation delivers heuristic role-based dispatch. Whether to (a) rename the file to `planner.rs` or `heuristic.rs` to match content, (b) ship a real `LlmAdapter` impl wired to `astraweave-llm::LlmClient`, or (c) leave the filename aspirational, is undecided.
- **`NpcProfile.schedule` is authored but unread.** [Factual — resolved 2026-05-12.] `profile.rs:32-48` defines `ScheduleEntry { hour, action, target }` and `NpcProfile.schedule: Vec<ScheduleEntry>`. Verified 2026-05-12: workspace grep inside `astraweave-npc/src` for `schedule:` returned only construction sites (`profile.rs:48,80,183`, `llm.rs:152`, `runtime.rs:298`), zero read sites. Workspace grep inside `examples/npc_town_demo` for `schedule` or `ScheduleEntry` returned zero matches. The schedule field is loaded from TOML but never consumed by any production or example code path. This is authored-but-unread design surface.
- **Persona / Memory name collisions across crates.** [Decisional, with factual context.] Factual: `astraweave-npc::Persona` (`profile.rs:14-22`) and `astraweave-memory::persona::Persona` (per §13.1 vocabulary) are distinct unrelated types named identically. Same for `Memory`: `astraweave-npc::Memory` (`profile.rs:24-30`, two `Vec<String>` lists) vs `astraweave-memory::Memory` (the rich `Memory { id, memory_type, content, metadata, associations, embedding }` from §13.1.3). The collision is benign at compile time because the crates are not co-imported anywhere; it's a vocabulary-clarity concern.
- **`MockLlm` uses `rand::rng()` introducing non-determinism (N13).** [Decisional / factual.] Factual: `llm.rs:31` calls `rand::rng()` which is the thread-local non-deterministic RNG. The canonical AI loop has `astraweave-ecs::Rng` (parent §3) as the seeded-deterministic alternative. Whether NPC should adopt the seeded path for replay-determinism is undecided.
- **`CommandSink` trait cannot expose position queries (N7, N8).** [Decisional.] Factual: the trait has 5 methods (move/say/open_shop/call_guards/give_quest) but no `position_of(BodyId) -> Vec3` accessor. This forces `MoveTo` and `CallGuards` to use placeholders for the body's current position. Whether to extend the trait (adding `position_of`) or to design around the limitation (NPCs holding their own cached position) is undecided.
- **`PhysicsWorld::control_character` always called with fixed dt = 1.0/60.0 (N6).** [Decisional, **enriched 2026-05-12**.] Factual: `runtime.rs:33-34` hardcodes the dt parameter despite `NpcManager::update` receiving its own `dt: f32`. NPC physics integration is locked to 60 Hz regardless of variable game-loop dt. Verified 2026-05-12 at `astraweave-physics/src/lib.rs:1247-1275`: `pub fn control_character(&mut self, id: BodyId, desired_move: Vec3, dt: f32, _climb: bool)` **does use dt internally** — `ctrl.jump_buffer_timer -= dt` (`:1263`), `ctrl.vertical_velocity -= 9.81 * ctrl.gravity_scale * dt` (`:1267`); the test `control_character_horizontal_movement_scales_with_dt` (`:3023`) confirms scaling behavior. The dt parameter is NOT ignored. Therefore the NPC runtime's hardcoded `1.0/60.0` is a deliberate locked-to-60Hz choice in the NPC integration layer, not a workaround for a parameter-ignored function. Whether this is a known compromise (e.g. determinism preference) or an oversight is undecided.
- **`astraweave-gameplay` Cargo dep is currently unused.** [Factual.] Factual: `Cargo.toml:18` lists `astraweave-gameplay` with comment "for dialogue/quests types if needed". Workspace grep inside `astraweave-npc/src` for `astraweave_gameplay` returns no matches. Similar to the dead-Cargo-deps finding in §13.1.7 (Memory) for `astraweave-llm/embeddings/rag`. Whether to remove the unused dep or to start consuming it is undecided.
- **Test surface vs production surface mismatch.** [Factual / informational.] 113 `#[test]` attributes for a crate with one in-engine consumer (an example). Like Memory (§13.1) and Director (§13.2), NPC is thoroughly tested in isolation but has a narrow integration surface. Whether the test corpus matches the production importance is a sizing question Andrew makes.

### 13.4 Subsystem Trace — Dialogue

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** **Two-layer architecture, mixed status.** The **basic dialogue layer** (`lib.rs` types, `runner.rs` state machine, `toml_loader.rs` authoring) is Active and production-wired into `veilweaver_slice_runtime/src/game_loop.rs` and the editor's `dialogue_editor_panel.rs`. The **LLM-enhanced layer** (`llm_dialogue.rs`, 2941 LoC / 60% of the crate by LoC) is Active (code-wise) / Dormant (runtime-wise) — workspace grep confirms zero external consumers outside the file itself. The two layers share no types: the LLM path defines its own `LlmDialogueSystem`, `DialogueConfig`, `ActiveConversation`, etc., distinct from the basic `DialogueGraph` / `DialogueRunner`.

#### 13.4.1 Role within the parent system

Dialogue provides **branching dialogue graphs** for player↔NPC and player↔companion conversation, with a TOML authoring format consumed by the editor and game runtime. The basic layer is the only AI subsystem in the parent system (alongside `astraweave-director`'s Warden directives) that is **directly wired into a shipped game-runtime path** (the Veilweaver slice's unified game loop). The LLM-enhanced layer was designed as a Phase 2 evolution adding emotion analysis, dynamic branching, RAG-backed context, and persona-aware response generation, but currently has no in-engine consumers. Dialogue's runtime types (`DialogueNode`, `DialogueResponse`, `DialogueGraph`) are distinct from the canonical AI loop's `WorldSnapshot`/`PlanIntent`/`ActionStep` — it's a parallel sibling subsystem that does **not** route through `validate_and_execute`; its mutations are emitted as `DialogueEvent`s and consumed by the game loop.

#### 13.4.2 Authoritative pipeline

```text
[Authoring time — editor or hand-edited TOML file]
    │
    │ TOML file with [[nodes]] entries, each carrying { id, line { speaker, text },
    │                                                    choices [{ text, go_to }], end }
    │ file: astraweave-dialogue/src/toml_loader.rs:1-17 (schema documentation)
    ▼
[D1 — TOML load]
    file: astraweave-dialogue/src/toml_loader.rs (load_dialogue_from_toml)
    role: Parse TOML via serde → translate to DialogueGraph/DialogueNode/DialogueResponse
    output: LoadedDialogue { dialogue_id, start_node, graph }
    │
    │ At game-load or scene-trigger time
    ▼
[D2 — Runner instantiation]
    file: astraweave-dialogue/src/runner.rs:63-74 (DialogueRunner::new)
    state: graph + current_node_id: Option<String> + state: RunnerState (Idle/WaitingForChoice/Finished)
           + pending_events: Vec<DialogueEvent> + history: Vec<String>
    │
    │ Game loop or scene event: runner.start(start_node_id)
    │   file: runner.rs:81-91
    ▼
[D3 — Active dialogue (basic path, production)]
    Per turn:
      1. NodeEntered event emitted on enter_node (runner.rs:88-89)
      2. UI/audio renders text + choices
      3. Player choose(choice_index) → runner.rs:96-…
         - validates state == WaitingForChoice
         - emits ChoiceMade { node_id, choice_index, choice_text, next_node_id }
         - either transitions to next node (NodeEntered) or emits Ended { last_node_id }
      4. Game loop drains events via drain_events() — consumed by veilweaver_slice_runtime
         (`veilweaver_slice_runtime/src/game_loop.rs:16-17` integrates DialogueRunner / DialogueEvent / RunnerState)
    │
    ▼
[D4 — Game loop event consumption]
    file: veilweaver_slice_runtime/src/game_loop.rs:33-39 (GameLoopEvent::DialogueDisplay,
                                                            GameLoopEvent::DialogueEnded)
    role: Translate DialogueEvent → GameLoopEvent for presentation layer (UI, audio, ECS)
```

```text
[Alternative pipeline — LLM-enhanced layer (DORMANT)]
    │
    │ Construct LlmDialogueSystem { llm_client, rag_pipeline, template_engine,
    │                              conversations: HashMap<id, ActiveConversation>,
    │                              config, metrics }
    │ file: astraweave-dialogue/src/llm_dialogue.rs:31-49
    ▼
[L1 — Configuration]
    Multiple config layers compose DialogueConfig:
      EmotionAnalysisConfig (sentiment, emotion detection, empathy, retention)
      DialogueContextConfig (max_history_turns, max_relevant_memories, context_window, summarization)
      BranchingConfig (dynamic branching threshold, max depth, merge strategy)
      QualityControlConfig (validation flag)
    file: llm_dialogue.rs:52-92, 150-…
    │
    ▼
[L2 — Designed flow (DORMANT — verified 2026-05-12)]
    Workspace grep for LlmDialogueSystem, DialogueConfig, EmotionAnalysisConfig,
    ActiveConversation returns ONLY the definition file astraweave-dialogue/src/llm_dialogue.rs.
    Zero external consumers in any other crate. Per CLAUDE.md Integration Completeness rule,
    this layer qualifies as dormant code. The 75 inline tests exercise the system in
    isolation; no end-to-end runtime hookup exists.
```

#### 13.4.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`DialogueNode`** | `{ id: String, text: String, responses: Vec<DialogueResponse> }`. The unit of dialogue. Builder methods: `new`, `with_responses`, `with_response`. Predicates: `is_terminal`, `is_choice`, `is_linear`, `has_responses`, `leads_to(node_id)`, `text_contains(substr)`. | `lib.rs:11-147` |
| **`DialogueResponse`** | `{ text: String, next_id: Option<String> }`. Builder: `new`, `with_next`, `next`. Predicates: `has_next`, `is_terminal`, `has_next_id(id)`. | `lib.rs:171-266` |
| **`DialogueGraph`** | `{ nodes: Vec<DialogueNode> }`. Operations: `add_node`, `get_node`, `validate` (returns `Err` if any `DialogueResponse.next_id` references a non-existent node — see `tests/dialogue.rs:26-38`). | `lib.rs:278-…` |
| **`DialogueRunner`** | State machine: `{ graph, current_node_id: Option<String>, state: RunnerState, pending_events: Vec<DialogueEvent>, history: Vec<String> }`. API: `new(graph)`, `start(start_node_id)`, `choose(choice_index)`, `drain_events()`. | `runner.rs:52-…` |
| **`RunnerState`** | `Idle` / `WaitingForChoice` / `Finished`. | `runner.rs:35-43` |
| **`DialogueEvent`** | `NodeEntered { node_id, text, choices: Vec<String> }` / `ChoiceMade { node_id, choice_index, choice_text, next_node_id: Option<String> }` / `Ended { last_node_id }`. **Distinct from** any canonical AI-loop event types. | `runner.rs:14-30` |
| **`LoadedDialogue`** | `{ dialogue_id: String, start_node: String, graph: DialogueGraph }`. Result of `load_dialogue_from_toml`. | `toml_loader.rs:72-79` |
| **TOML schema** | `[[nodes]]` entries with `id` + `line { speaker, text }` + `choices [{ text, go_to }]` + optional `end: bool`. Translated by `toml_loader.rs` to runtime types via internal `TomlDialogueFile`/`TomlDialogueNode`/`TomlLine`/`TomlChoice`. | `toml_loader.rs:25-65` |
| **`LlmDialogueSystem`** | `{ llm_client: Arc<dyn LlmClient>, rag_pipeline: Arc<RwLock<RagPipeline>>, template_engine: Arc<RwLock<TemplateEngine>>, conversations: Arc<RwLock<HashMap<String, ActiveConversation>>>, config: DialogueConfig, metrics: Arc<RwLock<DialogueMetrics>> }`. The LLM-enhanced dialogue orchestrator. | `llm_dialogue.rs:31-49` |
| **`DialogueConfig`** | LLM path top-level config: `{ max_response_tokens (256), temperature (0.8), top_p (0.9), num_dialogue_options (3), emotion_analysis: EmotionAnalysisConfig, context_config: DialogueContextConfig, branching_config: BranchingConfig, quality_control: QualityControlConfig }`. | `llm_dialogue.rs:52-92` |
| **`EmotionAnalysisConfig`** | `{ enable_sentiment_analysis, enable_emotion_detection, enable_empathy_responses, sentiment_influence (0.3), emotional_memory_retention (0.7) }`. | `llm_dialogue.rs:151-179` |
| **`DialogueContextConfig`** | `{ max_history_turns (10), max_relevant_memories (5), context_window_size (2048), enable_summarization (true) }`. Constructors: `minimal()` (3/2/512/false), `extended()` (20/10/4096/true). | `llm_dialogue.rs:233-281` |
| **`BranchingConfig`** | `{ enable_dynamic_branching, branching_threshold (0.7), max_branch_depth (5), merge_strategy: BranchMergeStrategy }`. | `llm_dialogue.rs:311-…` |
| **`ActiveConversation`** | Per-id LLM conversation state (held in `LlmDialogueSystem.conversations` map). | `llm_dialogue.rs` |
| **`DialogueMetrics`** | Performance metrics for LLM dialogue calls. | `llm_dialogue.rs` |

#### 13.4.4 Files involved

All listed in parent §5 enrichment above. Summary:

| File | LoC | `#[test]` count | Role |
|---|---:|---:|---|
| `lib.rs` | 1397 | 78 | `DialogueNode`/`DialogueResponse`/`DialogueGraph` + impls/builders/predicates/validation |
| `runner.rs` | 370 | 8 | `DialogueRunner` state machine + `DialogueEvent`/`RunnerState` |
| `toml_loader.rs` | 266 | 7 | TOML → `LoadedDialogue` |
| `llm_dialogue.rs` | 2941 | 75 | `LlmDialogueSystem` and all config types (dormant) |
| `tests/dialogue.rs` | 38 | 2 | Validation smoke tests |
| `tests/mutation_resistant_comprehensive_tests.rs` | 1329 | 127 | Mutation-killing tests across all four source files |
| `benches/dialogue_bench.rs` | 589 | 0 | Criterion bench |
| **Totals** | **6930** | **297** | |

**Dependencies (per `astraweave-dialogue/Cargo.toml`):**
- Used by basic layer: `serde`, `serde_json`, `anyhow`, `toml` (stdlib + serde stack only)
- Used by LLM layer only: `astraweave-llm`, `astraweave-embeddings`, `astraweave-context`, `astraweave-prompts`, `astraweave-rag`, `astraweave-persona`, `tokio (full)`, `async-trait`, `uuid`, `chrono`, `rand`, `regex`

The basic layer compiles without any of the LLM-stack deps; the LLM layer pulls in six AstraWeave crates plus heavyweight async + regex + non-deterministic RNG. The dependency split mirrors the active/dormant split.

#### 13.4.5 Cross-subsystem touchpoints

**Inside the parent AI system (basic layer, ACTIVE):**

| Touchpoint | Interface | File |
|---|---|---|
| Editor authoring | `astraweave_dialogue::{DialogueGraph, DialogueNode, DialogueResponse}` constructed from UI panel state | `tools/aw_editor/src/main.rs:94, 463-466, 7964-8015`; `tools/aw_editor/src/panels/dialogue_editor_panel.rs` (dedicated panel) |
| Game loop integration | `astraweave_dialogue::runner::{DialogueEvent, DialogueRunner, RunnerState}` + `astraweave_dialogue::toml_loader::LoadedDialogue` | `veilweaver_slice_runtime/src/game_loop.rs:16-17, 446` |
| Veilweaver e2e tests | `astraweave_dialogue::toml_loader::load_dialogue_from_toml` for storm-choice / pacing / smoke / walkthrough scenarios | `veilweaver_slice_runtime/tests/{e2e_game_loop_smoke, e2e_walkthrough_integration, e2e_pacing_playthrough, e2e_validation_smoke, mutation_tests}.rs` |
| Memory crate (single test reference) | `astraweave_memory::Persona as BasePersona` referenced once in `llm_dialogue.rs` (per parent §13.1.5 audit) | `llm_dialogue.rs:` reference to `astraweave-persona`-mediated `Persona` |

**Inside the parent AI system (LLM layer, DORMANT):**

| Designed touchpoint | Interface | Verified 2026-05-12 |
|---|---|---|
| LLM client | `Arc<dyn astraweave_llm::LlmClient>` (`llm_dialogue.rs:21, 33`) | No external code constructs `LlmDialogueSystem` |
| RAG retrieval | `Arc<RwLock<astraweave_rag::RagPipeline>>` (`llm_dialogue.rs:26, 36`) | Same |
| Prompt templates | `astraweave_prompts::{template::PromptTemplate, engine::TemplateEngine, context::PromptContext}` (`llm_dialogue.rs:23-25`) | Same |
| Conversation history | `astraweave_context::{ConversationHistory, ContextConfig, Role, Message}` (`llm_dialogue.rs:22`) | Same |
| Persona overlay | `astraweave_persona::{LlmPersonaManager, LlmPersona}` (`llm_dialogue.rs:27`) | Same |
| Embedding store | `astraweave_embeddings::{Memory, MemoryCategory}` (`llm_dialogue.rs:28`) | Same |
| Fallback chain (parent §2.3) | Not used. The LLM layer holds `Arc<dyn LlmClient>` directly, bypassing `FallbackOrchestrator`. | Same pattern as Director's `LlmDirector` — see §13.2.7 |

**Outside the parent AI system:**

- No direct touch with `astraweave-physics`, `astraweave-render`, or the canonical Companion-AI loop. Dialogue events are consumed by the game-loop adapter in `veilweaver_slice_runtime`, which then routes presentation to UI/audio.

**Critical absence (verified 2026-05-12):**

- Zero external consumers of `LlmDialogueSystem`, `DialogueConfig`, `EmotionAnalysisConfig`, `DialogueContextConfig`, `BranchingConfig`, `ActiveConversation`, `DialogueMetrics`. The entire 2941 LoC `llm_dialogue.rs` is unreachable from any in-engine production path. This is the same dormancy pattern surfaced for `AdaptiveWeightManager` in §13.1 (Memory) — production-quality LLM-enhanced subsystem with no runtime hookup.

#### 13.4.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| D1 | `DialogueGraph::validate` returns `Err` if any `DialogueResponse.next_id` references a node not present in the graph | Yes | Tested at `astraweave-dialogue/tests/dialogue.rs:26-38` (`test_dialogue_invalid_next_id`) |
| D2 | `DialogueRunner::choose(choice_index)` requires `state == RunnerState::WaitingForChoice` (returns `Err` otherwise) | Yes | `runner.rs:97-101` (`anyhow::ensure!`) |
| D3 | `DialogueRunner::start(start_node_id)` requires the node to exist (returns `Err` with `Context` otherwise) | Yes | `runner.rs:82-86` (`.with_context`) |
| D4 | Each call to `start` clears `history` before entering the start node | Yes | `runner.rs:88` |
| D5 | `NodeEntered` is emitted exactly once per node entry; `ChoiceMade` exactly once per `choose`; `Ended` exactly once when a terminal node is reached | Yes (behavioral) | `runner.rs:14-30, 95-…`. Verified by the 8 inline tests + integration tests |
| D6 | `DialogueResponse.next_id == None` is the terminal marker; `is_terminal()` returns `true` in that case | Yes | `lib.rs:213-215` |
| D7 | `DialogueNode.is_linear()` is true only when `responses.len() == 1`; `is_choice()` requires `> 1` | Yes | `lib.rs:65-73` |
| D8 | TOML `end: true` marker on a node sets `responses: vec![]` after translation (terminal) | Yes (inspection) | `toml_loader.rs:49-50` (`end: bool`) plus translation logic |
| D9 | `DialogueConfig.temperature` clamped to `[0.0, 2.0]` and `top_p` to `[0.0, 1.0]` in builder methods | Yes | `llm_dialogue.rs:104, 113` (`.clamp`) |
| D10 | `DialogueContextConfig::minimal()` and `::extended()` provide named-preset alternatives to defaults | Yes (code) | `llm_dialogue.rs:263-281` |
| D11 | `EmotionAnalysisConfig::disabled()` zeros all enable flags AND the sentiment-influence/retention floats | Yes | `llm_dialogue.rs:184-192` |
| D12 | `LlmDialogueSystem` is `Send + Sync` via auto-traits — all 6 fields are Send + Sync | Yes (verified 2026-05-12) | All 6 fields at `llm_dialogue.rs:31-49`: `Arc<dyn LlmClient>` (LlmClient: Send + Sync per trait at `astraweave-llm/src/lib.rs:85`), `Arc<RwLock<RagPipeline>>`, `Arc<RwLock<TemplateEngine>>`, `Arc<RwLock<HashMap<String, ActiveConversation>>>`, `DialogueConfig` (plain Serialize/Deserialize struct), `Arc<RwLock<DialogueMetrics>>`. All are `Send + Sync` so the struct auto-derives both traits. |
| D13 | `llm_dialogue.rs` uses `rand::{thread_rng, Rng}` (`llm_dialogue.rs:15`) — non-deterministic, parallel to NPC's `MockLlm` finding (§13.3 N13) | Yes | `llm_dialogue.rs:15` |
| D14 | Dialogue crate is unsafe-free | Yes | `astraweave-dialogue/src/lib.rs:1` `#![forbid(unsafe_code)]` |

#### 13.4.7 Open questions (subsystem-specific)

**No parent-level Open Questions about Dialogue were resolved during this pass** (the parent had no Dialogue-specific questions in §11).

**Subsystem-specific open questions:**

- **`llm_dialogue.rs` dormancy — production-wire, prune, or rebrand?** [Decisional, mirrors §13.1 Memory dormancy.] Factual state (verified 2026-05-12): the 2941 LoC LLM-enhanced layer has zero external consumers in the workspace. Workspace grep for `LlmDialogueSystem`, `DialogueConfig`, `EmotionAnalysisConfig`, `DialogueContextConfig`, `BranchingConfig`, `ActiveConversation`, `DialogueMetrics` returns only the file itself. 75 inline tests exercise it in isolation. Six AstraWeave-crate Cargo dependencies (`astraweave-llm`/`embeddings`/`context`/`prompts`/`rag`/`persona`) plus tokio + regex + rand are pulled in for this layer alone. Same decisional template as the Memory subsystem dormancy question: production-wire (hook `LlmDialogueSystem` into `veilweaver_slice_runtime/src/game_loop.rs` alongside the existing `DialogueRunner`), prune (delete `llm_dialogue.rs` and the six Cargo deps it pulls in), or rebrand (relocate to an experimental crate). The basic dialogue layer is excluded from this question — it's actively production-wired.
- **`LlmDialogueSystem` bypasses `FallbackOrchestrator`.** [Decisional / factual.] Same pattern as `LlmDirector` (§13.2.7): the system holds `Arc<dyn LlmClient>` directly (`llm_dialogue.rs:33`) instead of going through the parent §2.3 4-tier fallback chain. Whether this is intentional (dialogue failure modes are different from plan-generation failure modes — a missing LLM response can degrade to scripted dialogue) or a hardening gap is undecided.
- **Non-deterministic RNG in `llm_dialogue.rs:15`.** [Decisional / factual.] `use rand::{thread_rng, Rng}` introduces non-determinism, mirroring NPC's MockLlm (§13.3 N13). The canonical AI loop uses seeded `astraweave-ecs::Rng` for replay determinism. Whether the LLM dialogue path should adopt the seeded path is undecided.
- **TOML schema translation asymmetry.** [Factual — resolved 2026-05-12.] `TomlChoice { text, go_to }` becomes `DialogueResponse { text, next_id: Some(go_to) }`. `TomlDialogueNode.end: true` becomes `DialogueNode.responses: vec![]`. These translations are not 1:1; the runtime types lose the `end` marker (it becomes implicit via empty responses). Verified 2026-05-12 at `astraweave-dialogue/src/lib.rs:130-167` (DialogueNode definition + Display impl) and `:170-177` (DialogueResponse definition): **the `speaker` info from `TomlLine` is NOT retained after translation.** `DialogueNode` has only `id`, `text`, and `responses: Vec<DialogueResponse>`; `DialogueResponse` has only `text` and `next_id`. No speaker field exists in either runtime type.
- **`DialogueGraph::validate` is auto-called by `toml_loader` but NOT by `DialogueRunner::new`.** [Factual — corrected 2026-05-12, supersedes earlier "opt-in" framing.] Verified at `astraweave-dialogue/src/toml_loader.rs:124-127`: `load_dialogue_from_toml` calls `graph.validate().map_err(...)?` automatically. `DialogueRunner::new(graph)` (per `runner.rs:60-75`) does NOT auto-validate — it accepts any `DialogueGraph` regardless of integrity. Whether `veilweaver_slice_runtime/src/game_loop.rs:446` validates before constructing the runner remains unverified in this pass, but the TOML-load path is safe. The decisional question shifts: should `DialogueRunner::new` also validate (defense-in-depth) or trust callers?
- **`runner.rs:35-43` `RunnerState::Idle` vs `Finished` semantics.** [Factual / informational — **enriched 2026-05-12**.] Both are non-active states with no clear behavioral difference from the runner's perspective — `Idle` is the pre-`start` state; `Finished` is the post-terminal state. Verified 2026-05-12 at `runner.rs:81-91`: `start()` does NOT gate on state — it can be called from any state (including `Finished`); it clears history and enters the node unconditionally. The externally-observable distinction is only via `is_finished()` predicate at `:221` (`self.state == RunnerState::Finished`) — there is no `is_idle()`. Workspace grep for external `RunnerState::Idle` or `RunnerState::Finished` consumers (excluding `astraweave-dialogue/src` and `astraweave-dialogue/tests`) returned zero matches. The state machine could collapse to a single `Inactive` variant without behavior change. Whether the distinction is load-bearing for future game-loop adapter UI hints or vestigial is a decisional question.
- **`history: Vec<String>` accumulates without bound.** [Factual — refined 2026-05-12.] `runner.rs:60, 88` — `history` is initialized at `:72`, cleared on `start()` at `:88` AND on `reset()` at `:161`. Read access is provided by `pub fn history(&self) -> &[String]` at `:202` and queried by `pub fn was_visited(node_id: &str)` at `:209`. Within a single dialogue session (between `start()` and `reset()`), `history.push(node_id)` at `:248` is unbounded — long dialogues that revisit nodes grow unboundedly. Whether to cap history (and at what size) is undecided.
- **Test surface vs production surface mismatch.** [Factual / informational.] 297 `#[test]` attributes across the crate. The basic layer has 78+8+7+2 = 95 tests and is genuinely production-wired into the game loop. The LLM layer has 75+127 = ~127 tests (estimated portion of the comprehensive test file pertaining to LLM types) for zero production consumers. Mirrors the Memory (§13.1), Director (§13.2), and NPC (§13.3) pattern of "test corpus exceeds integration surface" for the dormant portion.

### 13.5 Subsystem Trace — Coordination

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** **In-Design (most aggressively dormant subsystem traced).** The crate compiles, ships 98 passing tests across inline + integration suites, and is internally consistent. **Workspace-wide grep (2026-05-12) confirms zero external consumers** — not in `astraweave-ai`, `astraweave-behavior`, `astraweave-llm`, `astraweave-memory`, `astraweave-director`, `astraweave-npc`, `astraweave-dialogue`, `astraweave-render`, examples, tools, or game-loop crates. The source itself contains **7+ explicit `#[allow(dead_code)]` markers** annotated "reserved for future ... implementation/pipeline" (`coordination.rs:78, 109, 150`; `narrative_coherence.rs:17`; `world_events.rs:19, 579, 816`), and **three commented-out module declarations** (`social_graph`, `components`, `systems`) whose source files were never created (`lib.rs:14-15, 23-24, 26-27`). The crate is self-documented as in-design.

#### 13.5.1 Role within the parent system

The Coordination subsystem is the **multi-agent orchestration layer** designed for cross-NPC/Director/Dialogue/Quest coordination, resource arbitration, message routing, and narrative coherence across the entire AI surface. Per `lib.rs:2-6`: "Multi-agent coordination system for LLM-powered game entities ... managing interactions between multiple AI agents, including NPCs, directors, dialogue systems, and quest generators." Architecturally, it would sit **above** the per-agent subsystems (Companion-AI loop, Director, NPC, Dialogue) as a meta-coordinator routing messages and arbitrating shared resources. In practice (verified 2026-05-12), it does not currently consume any other AI subsystem and is not consumed by any. The crate's `Agent` trait (`agent.rs:13-58`) is designed for the coordinated subsystems to implement, but **no production `impl Agent for …` exists outside the crate's own tests**.

#### 13.5.2 Authoritative pipeline

```text
[Designed entry point — game loop or scene-bootstrap code (NOT WIRED)]
    │
    │ Construct AgentCoordinator { agents: HashMap<id, Box<dyn Agent>>,
    │                              message_router: MessageRouter,
    │                              resource_manager: ResourceManager,
    │                              event_dispatcher: EventDispatcher,
    │                              coordination_sessions: HashMap<id, CoordinationContext>,
    │                              metrics: CoordinationMetrics,
    │                              config: CoordinatorConfig }
    │ file: astraweave-coordination/src/coordination.rs:16-31
    │
    │ Designed lifecycle:
    │   coordinator.register_agent(agent_id, Box<dyn Agent>) — fills agents map
    │   per tick: route messages via MessageRouter, allocate resources via ResourceManager,
    │             dispatch events via EventDispatcher
    │
    ▼
[C1 — Agent registration (DESIGNED)]
    role: Each agent implements the Agent trait (agent.rs:13-58):
            agent_id, agent_type, get_state, set_state, handle_message,
            get_goals, set_goals, get_capabilities, can_handle_task,
            execute_task, get_resource_usage, get_event_subscriptions,
            handle_world_event, add_task, is_available
    verified 2026-05-12: zero production `impl Agent for …` outside crate-internal tests
    │
    ▼
[C2 — Message routing (DESIGNED)]
    file: astraweave-coordination/src/coordination.rs:73-…
    structure: MessageRouter { channels: HashMap<id, mpsc::UnboundedSender>,
                                receivers: HashMap<id, mpsc::UnboundedReceiver>,
                                message_history: Vec<AgentMessage>,
                                routing_rules: Vec<RoutingRule> }
    routing rules: RoutingRule { id, from_pattern: Option<regex>, to_pattern: Option<regex>,
                                  message_type: Option<String>, action: RoutingAction, priority: i32 }
    actions: Allow / Block / Redirect(target_id) / Broadcast(Vec<id>) / Transform(content)
    note: `receivers` field is `#[allow(dead_code)]` per coordination.rs:78 "Receivers must be held
          to keep channels open" — design intent recorded inline
    │
    ▼
[C3 — Resource allocation (DESIGNED, RESERVED)]
    file: astraweave-coordination/src/coordination.rs:108-117
    note: `#[allow(dead_code)] // Fields reserved for future allocation strategy implementation`
    strategies: ResourceStrategy { FirstCome, Priority, LoadBalance, Adaptive }
    │
    ▼
[C4 — Event dispatch (DESIGNED, RESERVED)]
    file: astraweave-coordination/src/coordination.rs:150-…
    note: `#[allow(dead_code)] // Fields reserved for future event dispatch pipeline`
    │
    ▼
[C5 — World event generation (DESIGNED, RESERVED — LLM-driven)]
    file: astraweave-coordination/src/world_events.rs:18-30
    note: `#[allow(dead_code)] // Fields reserved for RAG-enhanced event generation pipeline`
    WorldEventGenerator: Arc<dyn LlmClient>, Arc<RagPipeline>, Arc<RwLock<ConversationHistory>>,
                        Arc<RwLock<PromptLibrary>>, config, event_templates, world_state,
                        event_history, active_storylines
    config defaults: generation_interval_ms = 30000, max_concurrent_events = 5,
                     creativity_factor = 0.7, world_coherence_weight = 0.8
    │
    ▼
[C6 — Narrative coherence (DESIGNED, RESERVED — LLM-driven)]
    file: astraweave-coordination/src/narrative_coherence.rs:16-29
    note: `#[allow(dead_code)] // Fields reserved for RAG-enhanced coherence pipeline`
    NarrativeCoherenceEngine: same heavy LLM-stack dependency pattern as WorldEventGenerator
    config defaults: coherence_threshold = 0.8, max_story_threads = 10,
                     character_consistency_weight = 0.4, world_consistency_weight = 0.3,
                     temporal_consistency_weight = 0.3, context_window_size = 4096
    │
    ▼
[C7 — Production wiring (NOT IMPLEMENTED)]
    role: Designed to bridge cross-subsystem coordination back to the canonical AI loop
          via WorldSnapshot updates, plan reconciliation, or shared resource arbitration
    verified 2026-05-12: no in-engine code constructs AgentCoordinator,
                          WorldEventGenerator, or NarrativeCoherenceEngine
```

#### 13.5.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`Agent`** trait | Async trait with 15 required methods: `agent_id`, `agent_type`, `get_state`, `set_state`, `handle_message`, `get_goals`, `set_goals`, `get_capabilities`, `can_handle_task`, `execute_task`, `get_resource_usage`, `get_event_subscriptions`, `handle_world_event`, `add_task`, `is_available`. `Send + Sync`. **Distinct from** `astraweave-ai::orchestrator::Orchestrator` (parent §3) — Coordination's `Agent` is broader, message-oriented, and async-by-default. | `agent.rs:11-58` |
| **`AgentState`** | `#[non_exhaustive]` 6-variant enum: `Idle`, `Processing`, `WaitingForInput`, `Collaborating`, `Disabled`, `Error(String)`. Parallel to NPC's `NpcMode` (§13.3.3) and Director's `CoordinationStatus` but distinct. | `agent.rs:62-70` |
| **`AgentMessage`** | `{ id: String, from: String, to: String, message_type: MessageType, content: serde_json::Value, timestamp: DateTime<Utc>, ... }`. The inter-agent message envelope. | `agent.rs:73-…` |
| **`AgentGoal`**, **`Task`**, **`TaskResult`** | Goal/task lifecycle types used by `Agent::execute_task`. | `agent.rs` |
| **`WorldEvent`**, **`EventSeverity`** | World-event broadcast types used by `Agent::handle_world_event`. `EventSeverity` distribution defaults in `SeverityDistribution`: 40% trivial / 30% minor / 20% moderate / 8% major / 2% critical. | `agent.rs`, `world_events.rs:46-52` |
| **`ResourceUsage`** | Per-agent resource accounting (CPU/memory/LLM-token tracking) returned by `Agent::get_resource_usage`. | `agent.rs` |
| **`CoordinationContext`**, **`CoordinationStatus`** | Per-session state machine for multi-agent coordination sessions. | `agent.rs` |
| **`AgentCoordinator`** | The central orchestrator. Owns `agents`, `message_router`, `resource_manager`, `event_dispatcher`, `coordination_sessions`, `metrics`, `config`. All fields `Arc<RwLock<…>>` for async multi-agent access. | `coordination.rs:16-31` |
| **`CoordinatorConfig`** | `{ max_tasks_per_agent: 5, max_message_queue_size: 100, default_task_timeout: 30s, resource_strategy: ResourceStrategy::Priority, enable_metrics: true, max_coordination_duration: 300s }` (defaults). | `coordination.rs:34-71` |
| **`ResourceStrategy`** | `#[non_exhaustive]` 4-variant: `FirstCome`, `Priority`, `LoadBalance`, `Adaptive`. Allocation policy for shared resources. | `coordination.rs:51-58` |
| **`MessageRouter`** | `{ channels: HashMap<id, mpsc::UnboundedSender>, receivers: HashMap<id, mpsc::UnboundedReceiver>, message_history: Vec<AgentMessage>, routing_rules: Vec<RoutingRule> }`. Tokio-mpsc-based inter-agent message bus. | `coordination.rs:73-84` |
| **`RoutingRule`** | `{ id, from_pattern: Option<String>, to_pattern: Option<String>, message_type: Option<String>, action: RoutingAction, priority: i32 }`. Regex-pattern-based dispatch with priority ordering. | `coordination.rs:87-95` |
| **`RoutingAction`** | `#[non_exhaustive]` 5-variant: `Allow`, `Block`, `Redirect(String)`, `Broadcast(Vec<String>)`, `Transform(String)` (note: `Transform` is documented as "placeholder"). | `coordination.rs:98-106` |
| **`ResourceManager`** | `{ allocations: HashMap<id, ResourceAllocation>, limits: ResourceLimits, strategy: ResourceStrategy }`. Crate-level `#[allow(dead_code)]` marker. | `coordination.rs:108-117` |
| **`EventDispatcher`** | Field-reserved (per `#[allow(dead_code)]` at `coordination.rs:150`). Crate-level event broadcasting for world-event propagation. | `coordination.rs:150-…` |
| **`CoordinationMetrics`** | Per-coordinator performance metrics (message throughput, task completion rates, resource utilization). | `coordination.rs` |
| **`WorldEventGenerator`** | LLM-powered world-event generator. Holds `Arc<dyn LlmClient>` + `Arc<RagPipeline>` + `Arc<RwLock<ConversationHistory>>` + `Arc<RwLock<PromptLibrary>>`. Designed to emit `GeneratedEvent`s on a 30s interval with creativity + coherence weighting. Field-level `#[allow(dead_code)]`. | `world_events.rs:18-30` |
| **`EventGenerationConfig`** | `{ generation_interval_ms: 30000, max_concurrent_events: 5, event_severity_distribution: SeverityDistribution, enable_storyline_continuity: true, context_window_size: 2048, creativity_factor: 0.7, world_coherence_weight: 0.8 }` (defaults). | `world_events.rs:32-72` |
| **`EventTemplate`** | Authored template for procedural event generation (`{ id, name, category, base_severity, ... }`). | `world_events.rs:75-…` |
| **`Storyline`** | Multi-event narrative arc that the generator threads through `GeneratedEvent`s. | `world_events.rs` |
| **`NarrativeCoherenceEngine`** | LLM-powered consistency validator. Same heavy LLM stack as `WorldEventGenerator`. Owns `narrative_state`, `consistency_rules`, `story_threads`, `character_arcs`, `world_continuity`. Field-level `#[allow(dead_code)]`. | `narrative_coherence.rs:16-29` |
| **`CoherenceConfig`** | `{ enable_real_time_validation: true, coherence_threshold: 0.8, max_story_threads: 10, character_consistency_weight: 0.4, world_consistency_weight: 0.3, temporal_consistency_weight: 0.3, context_window_size: 4096 }` (defaults). | `narrative_coherence.rs:32-55` |
| **`NarrativeState`**, **`ConsistencyRule`**, **`StoryThread`**, **`CharacterArc`**, **`WorldContinuity`** | Narrative-coherence sub-types governing story-thread tracking and character-arc validation. | `narrative_coherence.rs` |

#### 13.5.4 Files involved

All listed in parent §5 enrichment above. Summary:

| File | LoC | `#[test]` count | `#[allow(dead_code)]` count | Role |
|---|---:|---:|---:|---|
| `lib.rs` | 27 | 0 | 0 | Module declarations + re-exports; 3 commented-out module decls |
| `agent.rs` | 817 | 6 | 0 | `Agent` trait + AgentState/Message/Goal/Task etc. |
| `coordination.rs` | 2115 | 0 | 3 | `AgentCoordinator` + `MessageRouter` + `ResourceManager` + `EventDispatcher` |
| `world_events.rs` | 1060 | 1 | 3 | `WorldEventGenerator` (LLM-powered) |
| `narrative_coherence.rs` | 1298 | 1 | 1 | `NarrativeCoherenceEngine` (LLM-powered) |
| `tests/mutation_resistant_comprehensive_tests.rs` | 1036 | 90 | 0 | 90 mutation-killing integration tests |
| `benches/coordination_adversarial.rs` | 982 | 0 | 0 | Criterion bench harness |
| **Totals** | **7335** | **98** | **7** | |

**Dependencies (per `astraweave-coordination/Cargo.toml`):** `astraweave-llm`, `astraweave-rag`, `astraweave-context`, `astraweave-prompts`, plus `tokio (workspace)`, `futures 0.3`, `uuid`, `chrono`, `dashmap 6.0`, `petgraph 0.6`, `async-trait`, `rand`. **Notably absent:** `astraweave-core`, `astraweave-ai`, `astraweave-behavior`, `astraweave-memory`, `astraweave-director`, `astraweave-npc`, `astraweave-dialogue`. Despite being designed to coordinate the latter subsystems, the Coordination crate does not depend on any of them.

#### 13.5.5 Cross-subsystem touchpoints

**Inside the parent AI system:** None as production wiring. Designed to be the **meta-coordinator above** the per-subsystem AI crates, but verification 2026-05-12 confirms it has zero workspace consumers and zero imports of the canonical Companion-AI loop or any other sibling AI crate (NPC, Dialogue, Director, Memory).

**Outside the parent AI system (LLM-infrastructure):**

| Touchpoint | Interface | Files |
|---|---|---|
| LLM client | `Arc<dyn astraweave_llm::LlmClient>` | `world_events.rs:21`, `narrative_coherence.rs:19` |
| RAG retrieval | `Arc<astraweave_rag::RagPipeline>` | `world_events.rs:22`, `narrative_coherence.rs:20` |
| Conversation history | `Arc<RwLock<astraweave_context::ConversationHistory>>` + `ContextConfig` | `world_events.rs:23, 10`, `narrative_coherence.rs:21, 10` |
| Prompt templates | `astraweave_prompts::{library::PromptLibrary, template::PromptTemplate}` | `world_events.rs:24, 12-13`, `narrative_coherence.rs:22, 12-13` |
| Petgraph | `petgraph 0.6` for routing graph topology | Verified 2026-05-12: workspace grep inside `astraweave-coordination/src` for `petgraph::` returned zero matches. Cargo dep is declared (`Cargo.toml:25`) but unused. Dead Cargo.toml weight. |

All four LLM-infrastructure crates (`astraweave-llm`/`rag`/`context`/`prompts`) are consumed only by `world_events.rs` and `narrative_coherence.rs`. The `coordination.rs` and `agent.rs` files do not touch the LLM stack — they're tokio-mpsc plumbing + Agent trait definitions. The Coordination crate splits cleanly into a non-LLM core (`coordination.rs` + `agent.rs`, ~2932 LoC) and an LLM-driven extension (`world_events.rs` + `narrative_coherence.rs`, ~2358 LoC).

**Critical absence (verified 2026-05-12):**

- Zero `use astraweave_coordination` references in any other crate, example, tool, or test outside `astraweave-coordination/` itself.
- Zero workspace `impl Agent for …` implementations — only crate-internal mock implementations in `tests/mutation_resistant_comprehensive_tests.rs`.
- Zero workspace constructors of `AgentCoordinator`, `WorldEventGenerator`, or `NarrativeCoherenceEngine` outside the crate's own source.

This is the most aggressively dormant subsystem traced. Even stronger than the Memory subsystem (§13.1, which had `astraweave-persona` consuming legacy `persona::*` types) and the Dialogue LLM layer (§13.4, which at least shares a crate with the production-wired basic layer).

#### 13.5.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| C1 | `Agent` trait requires `Send + Sync` and is async-by-default (`#[async_trait]`) | Yes | `agent.rs:11-13` (`#[async_trait] pub trait Agent: Send + Sync`) |
| C2 | `Agent::handle_message` may emit a single `AgentMessage` response (`Option<AgentMessage>`) | Yes | `agent.rs:27` |
| C3 | `AgentMessage` carries `serde_json::Value` content (not strongly typed) — payload schema enforcement is per-agent | Yes (compile-time + runtime) | `agent.rs:79` |
| C4 | `CoordinatorConfig` default `max_tasks_per_agent = 5`, `max_message_queue_size = 100`, `default_task_timeout = 30s`, `max_coordination_duration = 300s` (5 min) | Yes | `coordination.rs:60-70` |
| C5 | `EventGenerationConfig` default severity distribution sums to 1.0 (0.4 + 0.3 + 0.2 + 0.08 + 0.02 = 1.0) | Yes | `world_events.rs:59-65` |
| C6 | `CoherenceConfig` consistency weights default to {character: 0.4, world: 0.3, temporal: 0.3} summing to 1.0 | Yes | `narrative_coherence.rs:48-50` |
| C7 | `RoutingRule.priority: i32` higher-value rules apply first per the inline doc comment | Doc-only | `coordination.rs:94` ("Higher priority rules apply first") |
| C8 | `RoutingAction::Transform(String)` is documented as "placeholder" — the variant exists but its semantics are unimplemented | Yes (code comment) | `coordination.rs:105` |
| C9 | `MessageRouter.receivers` is `#[allow(dead_code)]` with the comment "Receivers must be held to keep channels open" — they exist for tokio-mpsc semantics, not direct use | Yes | `coordination.rs:78-79` |
| C10 | Three `#[allow(dead_code)]` markers in `coordination.rs` (lines 78, 109, 150) plus 1 in `narrative_coherence.rs:17` plus 3 in `world_events.rs:19, 579, 816` constitute 7 reserved-for-future field-level annotations | Yes | All cited inline |
| C11 | Three commented-out module declarations in `lib.rs:14-15, 23-24, 26-27` (`social_graph`, `components`, `systems`) each annotated "Source file does not exist on disk"; verified workspace-wide no such files exist | Yes | Verified 2026-05-12 |
| C12 | `WorldEventGenerator` and `NarrativeCoherenceEngine` each hold the same set of 4 `Arc<…>` LLM-stack fields: `LlmClient`, `RagPipeline`, `ConversationHistory`, `PromptLibrary` | Yes | `world_events.rs:21-24`, `narrative_coherence.rs:19-22` |
| C13 | Coordination crate is unsafe-free | Yes | `astraweave-coordination/src/lib.rs:1` `#![forbid(unsafe_code)]` |
| C14 | The non-LLM core (`agent.rs` + `coordination.rs`) is independently usable without pulling in `astraweave-llm`/`rag`/`context`/`prompts` at the type level | Partial | The two non-LLM files do not import the LLM-stack crates per inspection. However, the Cargo.toml declares them as direct deps so they're pulled in at link time regardless |

#### 13.5.7 Open questions (subsystem-specific)

**Parent §11 Open Question enriched (not closed):**

- *Parent question:* "Three commented-out modules in `astraweave-coordination/src/lib.rs:14-27` (`social_graph`, `components`, `systems`) — placeholder or stalled?"
  *Enrichment delivered:* The parent question was enriched with §13.5 investigation factual context: the three module decls are commented out with explicit "Source file does not exist on disk" annotations; the active modules themselves contain 7+ `#[allow(dead_code)]` markers explicitly "reserved for future..." attesting to the crate's in-design status. The enriched question is now phrased as committing-or-pruning the entire crate, not just the three commented-out modules. The decisional remainder stays in §11.

**Subsystem-specific open questions:**

- **Coordination subsystem dormancy — commit, prune, or rebrand?** [Decisional, mirrors §13.1 Memory and §13.4 LLM-Dialogue dormancy patterns, but more aggressive.] Factual state (verified 2026-05-12): zero in-workspace consumers of any kind. Production code does not consume `Agent`, `AgentCoordinator`, `WorldEventGenerator`, or `NarrativeCoherenceEngine`. The crate's 98 inline+integration tests exercise it in isolation. Same decisional template as the Memory subsystem dormancy question (§11), with stronger evidence: even examples and tools/ have no `use astraweave_coordination` references. Three options: commit (build the production hookups — register `Agent` impls for NPC/Director/Dialogue, instantiate `AgentCoordinator` in the game loop, wire `WorldEventGenerator` into the Director or scene-trigger pipeline), prune (delete the crate or shrink to the non-LLM core if that's intended as a future starting point), or rebrand (relocate to an experimental crate alongside the same fate decided for §13.1 Memory pipeline and §13.4 LLM-Dialogue).
- **Non-LLM core vs LLM extension — same crate or split?** [Decisional.] Factual: `coordination.rs` + `agent.rs` (~2932 LoC) do not import the LLM stack at the type level; `world_events.rs` + `narrative_coherence.rs` (~2358 LoC) do. Whether to split the crate into `astraweave-coordination-core` (Agent trait + AgentCoordinator + message routing) and `astraweave-coordination-llm` (WorldEventGenerator + NarrativeCoherenceEngine) — or keep the union — is undecided.
- **`Agent` trait vs `astraweave-ai::orchestrator::Orchestrator` (parent §3).** [Decisional, with factual context.] Factual: `Agent` (`agent.rs:13-58`) has 15 required methods covering message handling, goal management, capabilities, task execution, resource usage, event subscription. `Orchestrator` (per parent §3) has a single method `propose_plan(&snap) -> PlanIntent`. They serve different roles (multi-agent coordination vs single-agent plan generation) but the conceptual overlap is substantial. Whether to unify (`Orchestrator` becomes a special case of `Agent` with a single capability) or to keep separate (`Orchestrator` remains the per-agent contract; `Agent` is the multi-agent contract) is undecided.
- **`RoutingAction::Transform(String)` is a documented placeholder.** [Factual.] Factual: `coordination.rs:105` includes the variant with a "placeholder" comment. Whether to implement (define the transform DSL or callback signature), remove the variant, or leave as future-direction marker is undecided.
- **No production `impl Agent for …` exists outside test code.** [Factual.] Factual (verified 2026-05-12): workspace grep for `impl.*Agent for` (where `Agent` is `astraweave-coordination::Agent`) outside the crate's own tests returns no hits. The seven LLM-driven subsystems traced so far (Memory's `AdaptiveWeightManager`, Director's `LlmDirector`/`BossDirector`/`PhaseDirector`/`OathboundWardenDirector`, NPC's `NpcManager`, Dialogue's `DialogueRunner`/`LlmDialogueSystem`) all have their own per-subsystem orchestration patterns; none implements the Coordination `Agent` trait. Whether the canonical pattern is to ship the `Agent` impl alongside the subsystem's own runtime (NPC adds `impl Agent for Npc`, Director adds `impl Agent for LlmDirector`, etc.) or to introduce adapter shims is undecided.
- **Heavy Cargo deps for in-design code.** [Factual / informational.] `petgraph 0.6` + `dashmap 6.0` + tokio-full + `astraweave-llm`/`rag`/`context`/`prompts` are pulled into the dependency graph for a crate with zero consumers. Same Cargo-dead-deps pattern as §13.1 Memory (`astraweave-llm`/`embeddings`/`rag` unused) and §13.3 NPC (`astraweave-gameplay` unused).
- **Test surface vs production surface mismatch (most extreme so far).** [Factual / informational.] 98 `#[test]` attributes (90 integration + 8 inline) for **zero** in-workspace production consumers. Memory (§13.1) had 1000+ tests with the legacy persona-only consumer; NPC (§13.3) had 113 tests with one example consumer; Director (§13.2) had 185 tests with `veilweaver_slice_runtime` + 2 examples; Dialogue (§13.4) had 297 tests with real production wiring on the basic layer. Coordination is the only subsystem where the integration surface is empty.
- **`petgraph` dependency — is it actually consumed?** [Factual — resolved 2026-05-12.] `Cargo.toml:25` declares `petgraph 0.6`. Workspace grep inside `astraweave-coordination/src` for `petgraph::` returned zero matches. Confirmed unused — another dead-Cargo-deps line item.
- **All four `*Config` defaults sum to mathematically clean weight distributions** (C5, C6): `EventGenerationConfig.event_severity_distribution` sums to 1.0; `CoherenceConfig` consistency weights sum to 1.0. The crate's authors deliberately authored balanced weight distributions. [Informational — surfaces care in the in-design defaults.]

### 13.6 Subsystem Trace — Advanced GOAP

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** **In-Design (feature-gated, most architecturally mature among dormant subsystems).** The 22-file submodule under `astraweave-ai/src/goap/` (16,736 LoC) is fully feature-gated on `planner_advanced` (`mod.rs:4-45` — every `pub mod` and `pub use` is `#[cfg(feature = "planner_advanced")]`). It exposes a clean canonical-`Orchestrator`-trait-implementing `GOAPOrchestrator` integration adapter, includes built-in shadow-mode infrastructure for safe rollout, and is structured across 6 phases (Phase 0 discovery → Phase 5 tooling). Verified 2026-05-12: zero production constructors of `AdvancedGOAP` or `GOAPOrchestrator` outside the crate's own tests (4), benches (2), and three disabled CLI binaries in `src/bin.disabled/`.

#### 13.6.1 Role within the parent system

Advanced GOAP is the **richer, learning-aware alternative** to the canonical `astraweave-behavior::goap` (parent §3, §6, §7 Decision Log entry "Archetype-based storage..." — wait, that's ECS; the relevant Decision Log is parent §7 implicit in the §6 conflict-map entry "Two GOAP implementations"). It is designed to slot into the canonical AI loop at the Reasoning stage (parent §2.1) via its `GOAPOrchestrator` integration adapter — same plug-shape as `RuleOrchestrator`, `VeilweaverCompanionOrchestrator`, and the GOAP slot in `AIArbiter`. The richness it adds vs canonical GOAP includes: **typed multi-valued state** (`StateValue` enum with 6 variants vs canonical `BTreeMap<u32, bool>`), **action history with success/failure tracking and dynamic cost** (`ActionHistory` + `Action::calculate_cost(world, history)`), **risk-aware A\* planning** (`f-cost = g + h + risk_weight * risk`, default `risk_weight = 5.0`), **hierarchical goal decomposition** (Phase 4 — `Goal` + `DecompositionStrategy`), **learning algorithms** (EWMA + Bayesian smoothing for success probability estimation), **persistence** (`PersistenceFormat` + `HistoryPersistence`), **shadow-mode rollout** (`ShadowModeRunner` runs canonical and Advanced in parallel emitting `PlanComparison`), and **tooling** (Phase 5 — `PlanAnalyzer`, `PlanVisualizer`, `GoalValidator`, `PlanDebugger`).

#### 13.6.2 Authoritative pipeline

```text
[Caller — designed to instantiate GOAPOrchestrator and pass to AIArbiter]
    │
    │ let orch = GOAPOrchestrator::new();  // registers Phase 2 action library
    │ AIArbiter::new(strategic_executor, fast_executor, Box::new(orch), bt_orch)
    │ file: astraweave-ai/src/goap/orchestrator.rs:16-24
    │
    │ Per-frame: AIArbiter::update(snap) routes GOAP slot through this orchestrator
    ▼
[AG1 — Orchestrator entry]
    file: astraweave-ai/src/goap/orchestrator.rs:11-24
    role: GOAPOrchestrator { planner: AdvancedGOAP } wraps the planner +
          a comprehensive action library registered via register_all_actions
    │
    ▼
[AG2 — Snapshot adaptation]
    file: astraweave-ai/src/goap/adapter.rs:10-… (SnapshotAdapter::to_world_state)
    role: Convert canonical WorldSnapshot into Advanced WorldState by extracting:
          - Player state: hp, pos, stance, plus derived flags (player_critical < 30,
            player_wounded < 60) per `adapter.rs:16-28`
          - Companion state: ammo, pos, morale, plus flags (has_ammo, ammo_low < 10,
            ammo_critical < 5, morale_high > 0.7, morale_low < 0.4) per `:31-43`
          - Cooldowns: per-name `cd_<name>` Float + `<name>_on_cooldown` Bool flag
          - Smoke availability via "throw:smoke" cooldown probe (`:56-58`)
          - Enemy state (aggregate): enemy_count, enemy_present, plus first-enemy
            details (hp, pos, cover, manhattan distance) per `:61-79`
    output: Advanced WorldState (rich BTreeMap<String, StateValue>)
    │
    ▼
[AG3 — Planning (A* search)]
    file: astraweave-ai/src/goap/planner.rs:47-…
    structure: AdvancedGOAP { actions: Vec<Box<dyn Action>>, history: ActionHistory,
                              max_plan_iterations: usize (10000), risk_weight: f32 (5.0) }
    algorithm: A* over WorldState nodes with f-cost = g + h + risk_weight * risk
               (planner.rs:17-19). Uses BinaryHeap with reversed Ord for min-heap
               (planner.rs:38-44). PlanNode { state, path, g_cost, h_cost, risk }.
    │
    │ Per-action cost calculation uses ActionHistory:
    │   file: action.rs:20-35
    │   failure_penalty = stats.failure_rate() * 10.0  (up to +10.0)
    │   success_bonus = stats.success_rate() * -2.0   (up to -2.0)
    │   final cost = max(0.1, base_cost + failure_penalty + success_bonus)
    │
    │ Hierarchical entry: plan_hierarchical(start, goal, depth=0)
    │   file: planner.rs:97-99 — handles both simple and decomposed goals
    │
    ▼
[AG4 — Plan-to-PlanIntent translation]
    file: astraweave-ai/src/goap/orchestrator.rs:32-… (plan_to_intent)
    role: Map Vec<String> action names → Vec<ActionStep> via hard-coded switch:
          - "move_to" | "approach_enemy" → ActionStep::MoveTo
          - "attack" → ActionStep::Attack { target_id }
          - "cover_fire" → ActionStep::CoverFire { target_id, duration: 2.0 }
          - "reload" → ActionStep::Reload
          - "take_cover" → ActionStep::MoveTo (retreat away from enemy)
          - "heal" → ActionStep::Heal { target_id: None }
          - (additional mappings continue in the file)
    output: PlanIntent { plan_id, steps: Vec<ActionStep> }
    │
    ▼
[AG5 — Plan returned to Arbiter / canonical Stage Pl]
    role: From here onward, the plan flows through the canonical pipeline
          (parent §2.1 Stage V validate_and_execute → Stage A apply). The
          Advanced GOAP is opaque to the canonical loop downstream of AG4.
```

```text
[Optional parallel path — Shadow Mode (designed for safe rollout)]
    file: astraweave-ai/src/goap/shadow_mode.rs:9-…
    role: ShadowModeRunner runs RuleOrchestrator AND AdvancedGOAP in parallel,
          emits PlanComparison { rule_plan, goap_plan, differences, metrics }
          for offline analysis without giving Advanced GOAP control of the agent
    PlanDiff: step_count_diff, actions_in_common, unique_to_rule, unique_to_goap,
              order_differs, similarity_score (0.0..=1.0)
    ComparisonMetrics: rule_faster, time_difference_ms, goap_more_steps,
                       both_empty, both_non_empty
```

```text
[Optional offline path — Disabled CLI tooling]
    files: astraweave-ai/src/bin.disabled/{analyze-plan, validate-goals, visualize-plan}.rs
    role: Phase 5 tooling — built but the directory rename .disabled keeps them out
          of cargo's normal binary discovery
    `analyze-plan.rs:331-367`: takes an AdvancedGOAP + plan → ComparisonReport
    `visualize-plan.rs:233`: takes an AdvancedGOAP + format → VisualizationFormat output
    `validate-goals.rs`: takes a GoalLibrary → ValidationResult per GoalValidator
```

#### 13.6.3 Vocabulary (subsystem-specific)

| Term | Definition | File |
|---|---|---|
| **`Action`** trait | 8-method async-free trait: `name`, `preconditions`, `effects`, `base_cost`, `calculate_cost(world, history)` (default impl applies failure/success bonuses), `can_execute(world)`, `success_probability(world, history)`, `state_cost_modifier(world)`, `estimated_duration(history)`. `Send + Sync`. **Distinct from** `astraweave-behavior::GoapAction` struct (canonical, data-only `{ name, cost, preconditions, effects }`). | `action.rs:5-65` |
| **`SimpleAction`** | Static-preconditions/effects helper struct implementing `Action`. | `action.rs:69-…` |
| **`StateValue`** | `#[non_exhaustive]` 6-variant enum: `Bool(bool)`, `Int(i32)`, `Float(OrderedFloat)`, `String(String)`, `IntRange(i32, i32)` (partial match), `FloatApprox(f32, f32)` (value, epsilon — approximate match). Implements `Hash` via discriminant-tag + bit-cast for floats (`state.rs:21-52`). **Distinct from** canonical `astraweave-behavior::goap::WorldState::facts: BTreeMap<u32, bool>`. | `state.rs:8-17` |
| **`OrderedFloat`** | f32 wrapper with deterministic ordering (likely via `to_bits()` per the hash impl pattern at `state.rs:32-34`). Allows `StateValue::Float` to be `Eq`/`Hash`. | `state.rs` (re-exported at `mod.rs:86`) |
| **`WorldState`** (Advanced) | `BTreeMap<String, StateValue>` with `set`/`get`/`satisfies`/`numeric_distance` operations. The numeric_distance method (`state.rs:81-…`) computes heuristic distance per StateValue variant — supports cross-type comparisons (Int vs Float) and range distances (val outside `IntRange(min, max)` → `min-val` or `val-max`). | `state.rs:81-…` |
| **`Goal`** | Hierarchical goal struct with `DecompositionStrategy` enum. Phase 4 expansion. | `goal.rs` |
| **`DecompositionStrategy`** | Goal-decomposition policy (e.g., greedy, exhaustive, priority-ordered). | `goal.rs` |
| **`AdvancedGOAP`** | Main planner: `{ actions: Vec<Box<dyn Action>>, history: ActionHistory, max_plan_iterations: 10000, risk_weight: f32 (5.0) }`. API: `new`, `add_action`, `plan(&start, &goal) -> Option<Vec<String>>` (hierarchical entry), `plan_hierarchical(start, goal, depth)`, `set_max_iterations`, `set_risk_weight`. | `planner.rs:48-99` |
| **`PlanNode`** | A\* search node: `{ state: WorldState, path: Vec<String>, g_cost: f32, h_cost: f32, risk: f32 }`. `f_cost = g + h + risk_weight * risk`. Implements custom `Ord` for `BinaryHeap` min-heap behavior via reverse comparison. | `planner.rs:6-45` |
| **`ActionHistory`** | Per-action stats tracker: `BTreeMap<String, ActionStats>` (likely). | `history.rs` |
| **`ActionStats`** | `{ successes, failures, avg_duration, ... }`. `success_rate()` and `failure_rate()` derived. | `history.rs` |
| **`GOAPOrchestrator`** | The canonical-`Orchestrator`-trait-implementing integration adapter. Holds an `AdvancedGOAP` with pre-registered action library. | `orchestrator.rs:11-…` |
| **`SnapshotAdapter`** | Stateless adapter: `to_world_state(&WorldSnapshot) -> WorldState`, plus `tactical_summary(&WorldSnapshot) -> String` (per usage at `shadow_mode.rs:60`). | `adapter.rs:8-…` |
| **`LearningManager`** | Phase 3 learning orchestrator combining EWMA + Bayesian smoothing. | `learning.rs` |
| **`EWMASmoothing`** | Exponentially Weighted Moving Average smoothing for success probability. `alpha: f32` (0..=1, clamped). | `learning.rs:8-34` |
| **`BayesianSmoothing`** | Beta-distribution Bayesian estimator: `(successes + prior_successes) / (total + prior_total)`. | `learning.rs:37-…` |
| **`HistoryPersistence`** + **`PersistenceFormat`** | Phase 3 persistence layer for ActionHistory across game sessions. | `persistence.rs` |
| **`ShadowModeRunner`** | Side-by-side comparison runner. Executes both `RuleOrchestrator` and `AdvancedGOAP` per tick; emits `PlanComparison` records without giving Advanced GOAP control. | `shadow_mode.rs:50-…` |
| **`PlanComparison`** | `{ timestamp, tactical_summary, rule_plan: PlanSummary, goap_plan: PlanSummary, differences: PlanDiff, metrics: ComparisonMetrics }`. | `shadow_mode.rs:11-18` |
| **`PlanSummary`** | `{ plan_id, step_count, action_types: Vec<String>, planning_time_ms: f64, empty: bool }`. | `shadow_mode.rs:21-28` |
| **`PlanDiff`** | `{ step_count_diff, actions_in_common, unique_to_rule, unique_to_goap, order_differs, similarity_score: f32 (0..=1) }`. | `shadow_mode.rs:31-39` |
| **`ComparisonMetrics`** | `{ rule_faster, time_difference_ms, goap_more_steps, both_empty, both_non_empty }`. | `shadow_mode.rs:42-49` |
| **`PlanAnalyzer`** + **`PlanMetrics`** + **`Suggestion`** + **`SuggestionPriority`** + **`ComparisonReport`** | Phase 5 plan-quality analyzer — emits suggestions for plan improvements. | `plan_analyzer.rs` |
| **`PlanStitcher`** + **`Conflict`** + **`StitchError`** | Phase 4 multi-plan merger for combining plans across decomposed sub-goals. | `plan_stitcher.rs` |
| **`PlanVisualizer`** + **`VisualizationFormat`** | Phase 5 plan-graph renderer. | `plan_visualizer.rs` |
| **`GoalLibrary`** + **`GoalDefinition`** + **`StateValueDef`** | Phase 4 declarative goal authoring (used by `validate-goals` CLI). | `goal_authoring.rs` |
| **`GoalScheduler`** | Phase 4 multi-goal scheduler with priority + dependency support. Takes `&AdvancedGOAP`, `Goal`, `WorldState`. | `goal_scheduler.rs:1, 79` |
| **`GoalValidator`** + **`Severity`** + **`ValidationError`** + **`ValidationResult`** | Phase 5 goal-correctness validator. | `goal_validator.rs` |
| **`PlanDebugger`** + **`Explanation`** + **`ProgressReport`** + **`StateChange`** + **`StateDiff`** | Phase 5 step-by-step plan-execution debugger. | `debug_tools.rs` |
| **`PlanExecutionTracker`** + **`TelemetryCollector`** | Telemetry layer. | `telemetry.rs` |
| **`GOAPConfig`** + **`CostTuningConfig`** + **`LearningConfig`** + **`SmoothingConfig`** + **`SmoothingMethod`** | Top-level config types for the planner + learning + cost tuning subsystems. | `config.rs` |

#### 13.6.4 Files involved

All 22 files behind `#[cfg(feature = "planner_advanced")]` per `mod.rs:4-45`. Plus 3 CLI binaries under `astraweave-ai/src/bin.disabled/` — disabled by directory rename.

| File | LoC | Role / Phase |
|---|---:|---|
| `mod.rs` | 88 | Module declarations + `pub use` re-exports — entirely feature-gated |
| `action.rs` | 279 | `Action` trait + `SimpleAction`. Phase 1 foundation |
| `actions.rs` | 981 | `register_all_actions(planner)` action library (Phase 2 engine integration) |
| `adapter.rs` | 815 | `SnapshotAdapter::to_world_state` Phase 2 integration |
| `config.rs` | 590 | All top-level config types |
| `state.rs` | 633 | `StateValue` 6-variant enum + `WorldState` + `OrderedFloat`. Phase 1 |
| `planner.rs` | 1299 | `AdvancedGOAP` + `PlanNode` + A* algorithm. Phase 1 |
| `orchestrator.rs` | 496 | `GOAPOrchestrator` integration adapter implementing canonical `Orchestrator`. Phase 2 |
| `history.rs` | 532 | `ActionHistory` + `ActionStats`. Phase 1/3 |
| `learning.rs` | 545 | EWMA + Bayesian smoothing. Phase 3 |
| `persistence.rs` | 473 | `HistoryPersistence` + `PersistenceFormat`. Phase 3 |
| `shadow_mode.rs` | 883 | `ShadowModeRunner` + `PlanComparison`. Phase 2 |
| `goal.rs` | 641 | `Goal` + `DecompositionStrategy`. Phase 1 + 4 |
| `goal_authoring.rs` | 558 | `GoalLibrary` + `GoalDefinition`. Phase 4 |
| `goal_scheduler.rs` | 696 | `GoalScheduler`. Phase 4 |
| `goal_validator.rs` | 1807 | `GoalValidator`. Phase 5 (largest single file in subsystem) |
| `plan_stitcher.rs` | 617 | `PlanStitcher` + `Conflict`. Phase 4 |
| `plan_analyzer.rs` | 1546 | `PlanAnalyzer` + `PlanMetrics`. Phase 5 |
| `plan_visualizer.rs` | 1437 | `PlanVisualizer`. Phase 5 |
| `debug_tools.rs` | 858 | `PlanDebugger`. Phase 5 |
| `telemetry.rs` | 642 | `PlanExecutionTracker`. Phase 1+ |
| `tests.rs` | 320 | Crate-internal tests (Phase 5 closure). All test files for Advanced GOAP route through this module |
| **Subtotal** | **16,736** | 22 files |
| `astraweave-ai/src/bin.disabled/analyze-plan.rs` | ~400 | Phase 5 CLI — disabled |
| `astraweave-ai/src/bin.disabled/validate-goals.rs` | ~? | Phase 5 CLI — disabled |
| `astraweave-ai/src/bin.disabled/visualize-plan.rs` | ~250 | Phase 5 CLI — disabled |
| **Tests (in `astraweave-ai/tests/`):** `goap_vs_rule_comparison.rs`, `goap_hierarchical_planning.rs`, `goap_learning_integration.rs`, `behavioral_correctness_tests.rs` (partial — references `planner_advanced` per workspace grep) | n/a | All 4 require feature `planner_advanced` |
| **Benches (in `astraweave-ai/benches/`):** `goap_performance_bench.rs`, `goap_vs_rule_bench.rs`, `alloc_measure.rs` (requires `alloc-counter` + `planner_advanced` per `Cargo.toml:67-69`) | n/a | |

#### 13.6.5 Cross-subsystem touchpoints

**Inside the parent AI system:**

| Touchpoint | Interface | Files |
|---|---|---|
| Canonical `Orchestrator` trait | **Correction (verified 2026-05-12):** `GOAPOrchestrator` (Advanced GOAP, ALL-CAPS) does NOT implement the `astraweave-ai::orchestrator::Orchestrator` trait — `astraweave-ai/src/goap/orchestrator.rs` contains only `impl GOAPOrchestrator` (inherent methods) and `impl Default for GOAPOrchestrator`, no `impl Orchestrator for GOAPOrchestrator`. It has a `pub fn propose_plan(&mut self, snap: &WorldSnapshot) -> PlanIntent` at `:121` whose signature matches the trait, but the trait is not formally implemented. To use Advanced GOAP as `Box<dyn Orchestrator>` (parent §2.2), an explicit `impl Orchestrator for GOAPOrchestrator` block would need to be added. Note: the canonical `GoapOrchestrator` (init-cap, `astraweave-ai/src/orchestrator.rs:353`) DOES implement both `Orchestrator` and `OrchestratorAsync` and is plug-compatible — this is a different struct in a different file. | `astraweave-ai/src/goap/orchestrator.rs:16, :167` |
| Canonical `WorldSnapshot` consumption | `SnapshotAdapter::to_world_state(&snap)` translates canonical state to Advanced GOAP's richer `WorldState`. The adapter reads `snap.player.{hp, pos, stance}`, `snap.me.{ammo, pos, morale, cooldowns}`, `snap.enemies[0].{hp, pos, cover}` and synthesizes 30+ derived state keys. | `astraweave-ai/src/goap/adapter.rs:10-…` |
| Canonical `PlanIntent` emission | `orchestrator.rs::plan_to_intent` hard-codes a switch from Advanced GOAP action-name strings → canonical `ActionStep` variants (`MoveTo`, `Attack`, `CoverFire`, `Reload`, `Heal`, plus more). | `astraweave-ai/src/goap/orchestrator.rs:32-…` |
| Canonical `RuleOrchestrator` (parent §3) | `ShadowModeRunner` (`shadow_mode.rs:9-…`) runs `RuleOrchestrator` alongside `AdvancedGOAP` to emit `PlanComparison` records. This is a side-by-side rollout pattern, not a delegating wrapper. | `astraweave-ai/src/goap/shadow_mode.rs:60` (`SnapshotAdapter::tactical_summary` consumed) |
| Canonical GOAP (`astraweave-behavior::goap`) | **No shared types.** See parent §6 "Two GOAP implementations" — canonical's `WorldState` (`BTreeMap<u32, bool>`) and Advanced's `WorldState` (`BTreeMap<String, StateValue>`) are unrelated. No `From`/`Into` impls; no shared trait surface beyond both implementing the parent `Orchestrator` (canonical does so via embedding in `RuleOrchestrator`-style wrappers; Advanced does so via `GOAPOrchestrator`). | Parent §6 conflict map |

**Outside the parent AI system:** Limited. The adapter only reads `astraweave-core::WorldSnapshot` and emits `astraweave-core::{PlanIntent, ActionStep, MovementSpeed}`. No physics / audio / render / scripting touchpoints — pure AI-substrate operations.

**Critical absence (verified 2026-05-12):**
- Zero `let _ = GOAPOrchestrator::new()` or `Box::new(GOAPOrchestrator::new())` constructors outside `astraweave-ai/tests/`, `astraweave-ai/benches/`, `astraweave-ai/src/bin.disabled/`, and `astraweave-ai/src/goap/orchestrator.rs:18` (the constructor definition).
- Zero `let _ = AdvancedGOAP::new()` outside the same set of locations.
- No examples/, tools/, or game-loop crate constructs either type.
- The `astraweave-ai/src/bin.disabled/` directory's `.disabled` suffix keeps the three CLI tools out of cargo's normal binary discovery — they cannot be invoked via `cargo run --bin`.

Despite the architectural cleanliness of the `Orchestrator`-trait integration, no production caller opts into Advanced GOAP. This contrasts with §13.4 Dialogue's basic layer (production-wired) and aligns with §13.1 Memory, §13.5 Coordination, and the LLM-enhanced Dialogue layer (§13.4) in terms of dormancy.

#### 13.6.6 Invariants (subsystem-specific)

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| AG1 | Every `pub mod` and `pub use` in `mod.rs` is `#[cfg(feature = "planner_advanced")]`-gated. With the feature off, nothing in the Advanced GOAP submodule is reachable from the crate root. | Yes | `mod.rs:4-88` — every line has the cfg attribute |
| AG2 | `AdvancedGOAP::new()` defaults: `max_plan_iterations = 10000`, `risk_weight = 5.0`, empty actions, empty history. | Yes | `planner.rs:55-63` |
| AG3 | `Action::calculate_cost` default impl clamps cost to `≥ 0.1` (never zero or negative). | Yes | `action.rs:34` (`cost.max(0.1)`) |
| AG4 | `Action::calculate_cost` failure_penalty bounded `[0, 10.0]` per `failure_rate * 10.0`; success_bonus bounded `[-2.0, 0]` per `success_rate * -2.0`. | Yes | `action.rs:25-29` |
| AG5 | `Action::success_probability` default returns 0.8 (80%) for unknown actions. | Yes | `action.rs:48` |
| AG6 | `Action::estimated_duration` default returns 1.0 second for unknown actions. | Yes | `action.rs:63` |
| AG7 | `StateValue` implements `Eq` via discriminant + bit-cast for floats (`Float` and `FloatApprox` hash via `to_bits()`). Ensures `StateValue` can be a `HashMap` key (or sortable in `BTreeMap`). | Yes | `state.rs:19, 32-34, 46-48` |
| AG8 | `StateValue::satisfies` supports mixed-type comparisons: `Int` satisfies `Float` within 1e-6 (`state.rs:73`); `Int` satisfies `IntRange` when `min ≤ val ≤ max` (`:67`); `Float` satisfies `FloatApprox` within epsilon (`:68-70`). | Yes | `state.rs:56-78` |
| AG9 | `PlanNode` ordering reverses `f_cost` comparison so `BinaryHeap` behaves as a **min-heap** (lowest-f-cost popped first). | Yes | `planner.rs:38-44` — note the explicit `other.f_cost(5.0).partial_cmp(&self.f_cost(5.0))` reverse direction |
| AG10 | `EWMASmoothing.alpha` is clamped to `[0.0, 1.0]` at construction. | Yes | `learning.rs:14-16` (`alpha.clamp(0.0, 1.0)`) |
| AG11 | `BayesianSmoothing` handles zero-data edge case by returning 0.5 (`learning.rs:57-58`). | Yes | `learning.rs:57-59` |
| AG12 | `ShadowModeRunner` does NOT give Advanced GOAP control of the agent — it only emits `PlanComparison` records for offline analysis. | Yes (architectural) | `shadow_mode.rs:9-49` — only emits comparison data structures, no plan-execution invocation |
| AG13 | `GOAPOrchestrator::plan_to_intent` is a hard-coded action-name → `ActionStep` switch (no general-purpose dispatcher). | Yes (code) | `orchestrator.rs:32-…` |
| AG14 | `bin.disabled/` directory rename keeps the 3 CLI tools out of cargo's default binary set — `cargo run --bin analyze-plan` fails until the directory is renamed back to `bin/`. | Yes | Directory name on disk |
| AG15 | Advanced GOAP integrates via the parent `Orchestrator` trait, NOT via direct ECS system registration. The `Orchestrator` abstraction is the only contract callers see. | Yes | `orchestrator.rs:11-24` — wraps `AdvancedGOAP` in a struct that implements the canonical trait |

#### 13.6.7 Open questions (subsystem-specific)

**Parent §11 Open Question enriched (not closed):**

- *Parent question:* "Two GOAP implementations — consolidation roadmap?"
  *Enrichment delivered:* The parent question was enriched with §13.6 verified factual context: the Advanced GOAP layer is 22 files / 16,736 LoC structured across 6 phases (0-5), has the `GOAPOrchestrator` integration adapter implementing the canonical `Orchestrator` trait, includes `ShadowModeRunner` for safe rollout, but has zero production constructors outside tests/benches/disabled-bins. The decisional remainder stays in §11.

**Subsystem-specific open questions:**

- **Advanced GOAP production-wiring decision.** [Decisional, mirrors §13.1 Memory and §13.5 Coordination dormancy patterns.] Factual state (verified 2026-05-12): zero production constructors of `AdvancedGOAP` or `GOAPOrchestrator` outside tests/benches/disabled-bins. The `Orchestrator`-trait integration is clean and ready, the shadow-mode infrastructure is in place, the action library (`register_all_actions`) is comprehensive, and the disabled CLI tooling indicates Phase 5 was completed-but-shelved. Three directional options (mirroring prior subsystem dormancy questions): commit (instantiate `GOAPOrchestrator` in `AIArbiter::new(...)` constructor call sites or in `ecs_ai_plugin::AiPlanningPlugin::sys_ai_planning`), prune (delete the 22 files and 3 disabled CLI tools, simplifying the crate by ~17K LoC), or rebrand (relocate to an experimental crate alongside the same fate decided for §13.1 Memory pipeline / §13.4 LLM-Dialogue / §13.5 Coordination).
- **`bin.disabled/` directory — disabled how and why?** [Factual — resolved 2026-05-12.] `astraweave-ai/src/bin.disabled/` contains `analyze-plan.rs`, `validate-goals.rs`, `visualize-plan.rs`. The `.disabled` suffix is the sole disabling mechanism — cargo only auto-discovers `src/bin/` for binary targets. Verified 2026-05-12: `astraweave-ai/Cargo.toml` contains 9 `[[bench]]` entries (lines 58-93) and zero `[[bin]]` entries, so no manifest path overrides the directory rename. To re-enable, the directory would be renamed to `bin/` (or each file moved into `bin/`). Whether to remove the bins (matching the prune option above) or to restore them as production tooling is undecided.
- **Hard-coded `plan_to_intent` switch in `orchestrator.rs:32-118`.** [Decisional / factual — resolved 2026-05-12.] The function maps action-name strings (e.g. `"move_to"`, `"approach_enemy"`, `"attack"`, `"cover_fire"`, `"reload"`, `"take_cover"`, `"heal"`, `"throw_smoke"`, `"retreat"`, `"revive"`, `"scan"`) to canonical `ActionStep` variants via a `match` statement. Verified at `orchestrator.rs:111-113`: the default `_ =>` arm emits `tracing::warn!("Unknown GOAP action: {}", action_name)` — **NOT silent**, but the unknown action is still dropped from the resulting `PlanIntent`. So: if `register_all_actions` (which registers ~30+ actions per the action library's 981 LoC) adds an action name not in the orchestrator's switch, the plan step is logged as a warn and dropped. Whether to fail loudly (return Err) or maintain warn-and-drop is undecided.
- **`StateValue::Bool` vs canonical `BTreeMap<u32, bool>`.** [Decisional, with factual context.] Factual: Advanced GOAP's `StateValue::Bool` is one of 6 variants (richness); canonical GOAP's facts map is `BTreeMap<u32, bool>` (intern-key + bool only). The two representations cannot share a planner. Whether the canonical implementation should adopt `StateValue` (forcing all callers to migrate) or whether the two should remain distinct (canonical for fast/deterministic; advanced for richer authoring) is the consolidation question — already in parent §11.
- **Phase 0 discovery → Phase 5 tooling — is Phase 6 (or a "Phase ∞: production wiring") implied?** [Factual — partially resolved 2026-05-12.] `docs/archive/phase_reports/` was checked: the directory contains PHASE2_*.md (15 files), PHASE3_COMPLETE.md, etc., but no file matching `phase6_*.md` was found. Verified 2026-05-12: no Phase 6 or "production wiring" sprint doc was found in the immediate archive listing. The current state remains "Phase 5 complete; not production-wired." Same decisional shape as the parent §11 Memory dormancy question.
- **Why Cargo features `planner_advanced = []` is an empty feature.** [Factual / informational.] `Cargo.toml:37` declares `planner_advanced = []` — no transitive feature flags. Enabling it just turns on the module-level `#[cfg(feature = "planner_advanced")]` gates in `mod.rs`. This is the simplest possible feature shape and matches the pattern used by `astraweave-math/Cargo.toml:23-24`'s `simd = []` no-op stub (per the foundation trace's `ecs_math_core_sdk_foundation.md` §13 note on simd-feature-noop). The Advanced GOAP feature is functionally a code-gate but architecturally clean.
- **`alloc_measure` bench requires both `alloc-counter` AND `planner_advanced`.** [Factual.] `Cargo.toml:67-69` documents this dual-feature requirement. This is the only place in the workspace that enables `planner_advanced` for purposes other than directly testing Advanced GOAP itself — it's measuring allocation profiles of the Advanced GOAP planner specifically. The dual-feature gate confirms Advanced GOAP is exercised by allocation-audit work even though it has no production callers.
- **Test surface vs production surface mismatch (largest absolute LoC mismatch).** [Factual / informational.] 16,736 LoC of implementation behind `planner_advanced` plus 4 dedicated tests + 2 dedicated benches + 3 disabled CLI bins. Zero production constructors. This is the largest absolute LoC of dormancy traced (Coordination was 5.3K LoC dormant; Memory was 11K LoC dormant via the legacy persona only; Dialogue LLM-layer was 2.9K LoC dormant). Advanced GOAP at 16.7K LoC is the deepest reservoir of in-design but un-wired AI code in the workspace.

---

### 13.7 Subsystem Trace — LLM Production Hardening

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** **Mixed activity profile — output-side primitives wired; full hardening composite dormant.** Verified 2026-05-12 across 16 files / ~15K LoC composing the hardening surface. **Actively wired in production paths:** `parse_llm_response` (called twice from `FallbackOrchestrator` — `fallback_system.rs:465` Tier-1 and `:512` Tier-2). **Dormant (zero production constructors):** `ProductionHardeningLayer` (referenced only in its own file + `examples/llm_production_hardening_demo.rs`); `FallbackOrchestrator` itself (doc-comment-only references in `ai_arbiter.rs:51,58` and `llm_executor.rs:34,88,102` — no `::new` instantiation in non-test code); `ToolGuard` (only in its own file + `mutation_resistant_comprehensive_tests.rs`); `LlmScheduler` (only in its own scheduler.rs tests); `RateLimiter`, `CircuitBreakerManager`, `BackpressureManager`, `ABTestFramework` (only as embedded fields of the dormant `ProductionHardeningLayer`); `safe_llm_invoke` from `llm_adapter.rs` (zero workspace callers — stub remains pure stub). The hardening surface is the second-deepest LoC dormancy reservoir in the workspace after Advanced GOAP (parent §13.6).

#### 13.7.1 Role within the parent system

LLM Production Hardening is the **operational reliability + safety wrap** designed to sit between any orchestrator that calls `LlmClient::complete` and the network/LLM boundary (parent §2.4, parent §7 Decision Log "Production hardening composes 6 reliability primitives"). It composes six primitives — `RateLimiter` (per-user/per-model/global RPM+TPM with token-bucket semantics), `CircuitBreakerManager` (per-model failure-rate tripping with three states Closed/Open/HalfOpen), `BackpressureManager` (5-priority queue with admission control and adaptive concurrency), `ABTestFramework` (experiment routing for prompt/model A/B comparisons), `LlmTelemetry` (atomic-counter latency/error metrics), and `HealthChecker` (per-component status with consecutive-success/failure thresholds) — alongside output-side primitives `parse_llm_response` (5-strategy extraction), `ToolGuard` (action policy + audit log), `FallbackOrchestrator` (4-tier degradation chain), `PromptCompressor` (25-30% token reduction), `LlmScheduler` (priority queue for ECS-friendly async planning), and `RetryConfig` (exponential-backoff retry with deterministic jitter). All primitives are designed to be plug-compatible with any `LlmClient` trait implementor (`MockLlm`, `Hermes2ProClient`, `Qwen3Client`, `Phi3Client`).

The architectural intent (per `production_hardening.rs:9-39` doc-comment-equivalent and parent §2.4 pipeline diagram) is that a single `ProductionHardeningLayer` instance fronts ALL LLM traffic for a deployment — `process_request<F, T, Fut>(request, operation)` (`production_hardening.rs:347-…`) runs the supplied closure under all six primitives in order: rate-limit check → circuit-breaker gate → backpressure admission → A/B variant resolution → execute → telemetry record → health-check update. The result is a `HardeningResult<T>` 6-variant `#[non_exhaustive] #[must_use]` enum (Success / RateLimited / CircuitOpen / Queued / Rejected / Error) per `:110-137`. None of this is wired today — the only consumer is the standalone demo at `examples/llm_production_hardening_demo.rs`.

#### 13.7.2 Authoritative pipeline

```text
[Caller — designed pipeline; actual usage limited to plan_parser path]
    │
    │ Designed: ProductionHardeningLayer::new(HardeningConfig::default())
    │           .start().await?
    │           .process_request(HardenedRequest{user_id, model, prompt, ...}, ||
    │             llm_client.complete(prompt))
    │
    ▼
[PH1 — Rate Limiter check]                       (DORMANT in production)
    file: astraweave-llm/src/rate_limiter.rs:40-53 (config defaults)
    role: Per-user RPM (default 100) + per-model RPM/TPM (default 1000 RPM,
          50000 TPM) + global RPM (10000) + optional burst (1.5× multiplier).
          Token-bucket replenishment over `window_duration` (default 60s).
          Adaptive limiting flag (default true) allows latency-driven throttling.
    │
    ▼
[PH2 — Circuit Breaker gate]                     (DORMANT in production)
    file: astraweave-llm/src/circuit_breaker.rs:50-… (CircuitBreakerConfig)
    role: Per-model breaker. Trips Closed→Open when failure_count >= 5 within
          a 60s window AND request_count >= 10 (minimum_requests). Open state
          rejects all requests for recovery_timeout (30s default). Then transitions
          Open→HalfOpen and allows trial requests until success_count >= 3
          → Closed, or any failure → Open. State transitions tracked by
          `state_changed_time` per `circuit_breaker.rs` CircuitBreaker struct.
    │
    ▼
[PH3 — Backpressure admission + 5-priority queue] (DORMANT in production)
    file: astraweave-llm/src/backpressure.rs:50-… (BackpressureConfig + Priority)
    role: Bounded concurrency via tokio Semaphore (default max_concurrent=100,
          max_queue=1000, request_timeout=30s). Priority enum:
          Critical=0 / High=1 / Normal=2 / Low=3 / Background=4. Higher priority
          drains first (Priority::all() helper at `backpressure.rs` provides
          the canonical order). Adaptive concurrency adjusts toward
          target_latency_ms (1000). Load shedding kicks in at 90% queue fill
          (load_shedding_threshold = 0.9). Graceful degradation flag rejects
          Low/Background tier first.
    │
    ▼
[PH4 — A/B variant resolution]                   (DORMANT in production)
    file: astraweave-llm/src/ab_testing.rs:38-89 (Experiment + Variant)
    role: Look up an active `Experiment` (status Running, traffic_percentage,
          control_variant + test_variants[]). Assign user_id → variant via
          assignment_strategy (DeterministicHash by default per
          `ab_testing.rs:5` hasher). Override prompt_template + model_config
          (temperature, max_tokens, top_p, top_k, repetition_penalty) per
          variant. Min sample size 100, significance threshold p=0.05,
          auto_winner_selection off by default.
    │
    ▼
[PH5 — Execute (the supplied operation closure)]
    role: This is where the actual `LlmClient::complete(prompt).await` runs.
          The hardening layer is agnostic about which client (Ollama HTTP /
          Phi-3 local Candle / MockLlm). Wrapped in tokio::time::timeout
          using HardenedRequest.timeout.
    │
    ├── ON SUCCESS:
    │      └─ PH6.a [Plan parser — 5-strategy extraction]
    │             file: astraweave-llm/src/plan_parser.rs:72-…
    │             [Stage 1] try_direct_parse → ExtractionMethod::Direct
    │             [Stage 2] try_code_fence_parse → ExtractionMethod::CodeFence
    │             [Stage 3] try_envelope_parse → ExtractionMethod::Envelope
    │             [Stage 4] try_object_extraction → ExtractionMethod::ObjectExtraction
    │             [Stage 5] try_tolerant_parse → ExtractionMethod::Tolerant
    │             Each stage emits explicit ASCII-art box debug logging via
    │             eprintln (unique among LLM debug paths in the workspace).
    │             ACTIVE: called from FallbackOrchestrator (fallback_system.rs:465
    │             Tier-1 FullLlm path, :512 Tier-2 SimplifiedLlm path)
    │             output: ParseResult{plan: PlanIntent, method: ExtractionMethod}
    │             │
    │             ▼
    │      └─ PH6.b [Output validation — ToolGuard]   (DORMANT in production)
    │             file: astraweave-llm/src/tool_guard.rs:84-…
    │             role: Per-action ToolPolicy lookup (Allowed/Restricted/Denied)
    │                   against the configured policy DashMap. Defaults at
    │                   `tool_guard.rs:100-…`: ExecuteCode/DeleteFile/ModifyWorld
    │                   → Denied; Wait/Look → Allowed; MoveTo/Attack/UseItem/
    │                   Throw/CoverFire → Restricted; default_policy =
    │                   Restricted (whitelist semantics). Audit log (max 1000
    │                   entries) records every Valid/Invalid{reason}/Denied{action}
    │                   outcome with timestamp.
    │
    ├── ON FAILURE (LLM error / timeout / parse error):
    │      └─ PH7 [Fallback chain — 4-tier degradation]   (ACTIVELY READY,
    │              currently consumed only via FallbackOrchestrator doc examples)
    │             file: astraweave-llm/src/fallback_system.rs:50-…
    │             Tier 1 (FullLlm): full prompt + full action registry.
    │                  On parse failure → next.
    │             Tier 2 (SimplifiedLlm): compressed prompt + simplified registry
    │                  (15 tools — MoveTo/ThrowSmoke/ThrowExplosive/AoEAttack/
    │                  TakeCover position-based + Attack/Approach/Retreat/MarkTarget/
    │                  Distract target-based + Reload/Scan/Wait/Block/Heal simple
    │                  per `fallback_system.rs:98-117`). On parse failure → next.
    │             Tier 3 (Heuristic): rule-based via `HeuristicConfig::default()`
    │                  with 7 rules (LowMorale<30 → HealSelf; LowAmmo<=0 → Reload;
    │                  EnemyNearby<=3.0 → AttackNearestEnemy; EnemyVisible →
    │                  TakeCover{distance:2.0}; ObjectiveContains("extract"|"reach")
    │                  → MoveToObjective; Always → Scan{radius:10.0}) per
    │                  `heuristics.rs:9-49`. Always succeeds (Always fallback rule).
    │             Tier 4 (Emergency): hardcoded safe-default plan. Always succeeds.
    │             output: FallbackResult{plan, tier, attempts: Vec<FallbackAttempt>,
    │                     total_duration_ms} — every tier transition tracked for
    │                     metrics (FallbackMetrics.tier_successes/tier_failures).
    │
    ▼
[PH8 — Telemetry record]                          (DORMANT in production)
    file: astraweave-llm/src/telemetry.rs:7-44 (LlmTelemetry struct)
    role: AtomicU64 counters (Ordering::Relaxed throughout). Tracks
          requests_total / requests_success / requests_error / cache_hits /
          cache_misses / retries_attempted / circuit_breaker_open /
          fallbacks_triggered + latency sums for averaging
          (latency_llm_call_ms/_count, latency_plan_total_ms/_count).
          Zero-overhead: pure atomic increments.
    │
    ▼
[PH9 — Health checker update]                     (DORMANT in production)
    file: astraweave-llm/src/production_hardening.rs:166-301 (HealthChecker)
    role: Per-component (rate_limiter / circuit_breaker / backpressure /
          telemetry / ab_testing) status tracking via consecutive successes
          (unhealthy_threshold=3 / healthy_threshold=2 per :72-93
          HealthCheckConfig). Periodic check_interval=30s background task,
          check_timeout=5s per probe. Aggregates to SystemHealth{
          overall_status: HealthStatus::{Healthy/Degraded/Unhealthy},
          components: HashMap<String, ComponentHealth>, uptime_seconds}
          per `production_hardening.rs:274-300`.
    │
    ▼
HardeningResult<T> ::= Success(T) | RateLimited{...} | CircuitOpen{...}
                    | Queued{request_id, ...} | Rejected{reason}
                    | Error(anyhow::Error)
```

**Critical observation:** Steps PH1-PH5, PH8, PH9 are all gated behind `ProductionHardeningLayer::process_request`. Since that method has zero non-test, non-example callers, none of these primitives execute in the runtime AI loop today. The path that DOES run in production is the narrow PH6.a (`parse_llm_response`) wire from `FallbackOrchestrator` Tier-1 / Tier-2 — and `FallbackOrchestrator` itself is dormant per §13.7.5 below. The runtime `AIArbiter` calls `LlmClient::complete` directly via `LlmExecutor` (`astraweave-ai/src/llm_executor.rs`), bypassing both the hardening composite AND the fallback chain.

#### 13.7.3 Subsystem-specific vocabulary (additive to parent §3)

| Term | Definition | Source |
|---|---|---|
| `HardeningConfig` | 6-field composite config: rate_limiter + circuit_breaker + backpressure + telemetry + ab_testing + health_check, plus `graceful_shutdown_timeout: Duration` (default 30s) | `astraweave-llm/src/production_hardening.rs:40-69` |
| `HardenedRequest` | Request envelope: user_id, session_id, model, prompt, estimated_tokens, priority: Priority, timeout: Duration, metadata: HashMap | `production_hardening.rs:96-106` |
| `HardeningResult<T>` | `#[non_exhaustive] #[must_use]` 6-variant outcome | `production_hardening.rs:110-137` |
| `SystemHealth` | Aggregated cross-component status report | `production_hardening.rs:139-155` |
| `ComponentHealth` | Per-primitive health snapshot: status, last_check (RFC3339 string), consecutive_failures/_successes, last_error, response_time_ms | `production_hardening.rs:156-164` |
| `HealthStatus` | `#[non_exhaustive]` 3-variant: Healthy / Degraded / Unhealthy | inferred from `production_hardening.rs:251-289` usage |
| `HealthCheckConfig` | check_interval (30s) + check_timeout (5s) + unhealthy_threshold (3) + healthy_threshold (2) | `production_hardening.rs:72-93` |
| `RateLimiterConfig` | default_rpm (1000) + default_tpm (50000) + user_rpm (100) + global_rpm (10000) + allow_burst (true) + burst_multiplier (1.5) + window_duration (60s) + adaptive_limiting (true) | `rate_limiter.rs:40-53` |
| `CircuitBreakerConfig` | failure_threshold (5) + failure_window (60s) + minimum_requests (10) + recovery_timeout (30s) + success_threshold (3) + enabled (true) | `circuit_breaker.rs:50-…` |
| `CircuitState` | `#[non_exhaustive]` 3-variant: Closed / Open / HalfOpen | `circuit_breaker.rs:…` |
| `BackpressureConfig` | max_concurrent_requests (100) + max_queue_size (1000) + request_timeout (30s) + processing_interval (10ms) + adaptive_concurrency (true) + target_latency_ms (1000) + load_shedding_threshold (0.9) + enable_graceful_degradation (true) | `backpressure.rs:50-…` |
| `Priority` | `#[non_exhaustive]` 5-variant: Critical=0 / High=1 / Normal=2 / Low=3 / Background=4. `Priority::all()` returns canonical order | `backpressure.rs:54-87` (estimated) |
| `RequestPriority` | **Distinct from `Priority` above.** 3-variant Low=0/Normal=1/High=2 used by `LlmScheduler`. Naming collision with `backpressure::Priority` | `scheduler.rs:39-46` |
| `QueuedRequest` (backpressure) | id, priority, payload, timeout, completion: oneshot::Sender<Result<()>>, queued_at, metadata | `backpressure.rs:…` |
| `QueuedRequest` (scheduler) | id: Uuid, prompt: String, priority: RequestPriority, response_tx: oneshot::Sender<Result<String>>, submitted_at | `scheduler.rs:68-75` |
| `LlmScheduler` | Priority queue + concurrent execution + timeout; designed as ECS resource for non-blocking async AI planning | `scheduler.rs:84-93` |
| `RequestStatus` | `#[non_exhaustive]` 5-variant: Queued / Processing / Completed / Failed / TimedOut | `scheduler.rs:49-57` |
| `RequestResult` | request_id: Uuid, response: String, elapsed_ms: u64 | `scheduler.rs:60-65` |
| `ABTestConfig` | default_duration_hours (168 = 1 week) + min_sample_size (100) + significance_threshold (0.05 = 95% confidence) + auto_winner_selection (false) + max_concurrent_experiments (10) | `ab_testing.rs:23-46` |
| `Experiment` | id + name + description + status: ExperimentStatus + lifecycle timestamps + duration_hours + traffic_percentage (0.0-1.0) + control_variant: Variant + test_variants + target_metric + success_criteria + assignment_strategy + metadata | `ab_testing.rs:49-66` |
| `ExperimentStatus` | `#[non_exhaustive]` 5-variant: Draft / Running / Paused / Completed / Stopped | `ab_testing.rs:69-77` |
| `Variant` | id + name + description + prompt_template: Option<String> + model_config: Option<ModelConfig> + parameters: HashMap<String, Value> + traffic_allocation (0.0-1.0) | `ab_testing.rs:80-89` |
| `ModelConfig` | model_name + Option<temperature/max_tokens/top_p/top_k/repetition_penalty> | `ab_testing.rs:92-100` |
| `OptimizationDirection` | `#[non_exhaustive]` 2-variant: Maximize / Minimize | `ab_testing.rs:112-117` |
| `RetryConfig` | max_attempts + initial_backoff_ms + backoff_multiplier + max_backoff_ms + jitter (bool). Constructors: `::production()` (3 attempts, 50ms initial, 2.0×, 500ms cap, jitter on) / `::aggressive()` (5/25/1.5×/300) / `::disabled()` (0) | `retry.rs:7-57` |
| `RetryConfig::backoff_for_attempt(attempt)` | Deterministic jitter via `attempt.wrapping_mul(0x517cc1b727220a95) % (jitter_range*2+1)` — same attempt always produces same jitter (replay-safe) | `retry.rs:60-86` |
| `RetryConfig::backoff_for_attempt_seeded<R: Rng>` | Externally-seeded RNG variant for replay-system control | `retry.rs:88-…` |
| `LlmTelemetry` | 12 atomic counters: requests_total/_success/_error + cache_hits/_misses + retries_attempted + circuit_breaker_open + fallbacks_triggered + latency_llm_call_ms/_count + latency_plan_total_ms/_count. All Ordering::Relaxed | `telemetry.rs:8-46` |
| `parse_llm_response(text, registry) -> Result<ParseResult>` | 5-stage chain entry point. Each stage prints explicit ASCII-art box header + checkmark/X marks via eprintln | `plan_parser.rs:72-…` |
| `ParseResult` | plan: PlanIntent + method: ExtractionMethod | `plan_parser.rs:…` |
| `ExtractionMethod` | `#[non_exhaustive]` 5-variant: Direct / CodeFence / Envelope / ObjectExtraction / Tolerant | `plan_parser.rs:…` |
| `ToolGuard` | DashMap<String, ToolPolicy> + audit_log: DashMap-based ring + default_policy + max_audit_entries (1000) | `tool_guard.rs:84-…` |
| `ToolPolicy` | `#[non_exhaustive]` 3-variant: Allowed / Restricted / Denied | `tool_guard.rs:…` |
| `ValidationResult` | `#[non_exhaustive] #[must_use]` 3-variant: Valid / Invalid{reason} / Denied{action} | `tool_guard.rs:…` |
| `AuditEntry` | timestamp + action_type + result + reason | `tool_guard.rs:…` |
| `FallbackOrchestrator` | metrics + simplified_tools (15) + heuristic_config + circuit_breaker. Implements `Orchestrator` + `OrchestratorAsync` to slot into the canonical loop | `fallback_system.rs:50-…` |
| `FallbackTier` | `#[non_exhaustive]` 4-variant: FullLlm / SimplifiedLlm / Heuristic / Emergency. `next()` chain advances FullLlm → SimplifiedLlm → Heuristic → Emergency → None terminal | `fallback_system.rs:…` |
| `FallbackResult` | plan + tier (which Tier succeeded) + attempts: Vec<FallbackAttempt> + total_duration_ms | `fallback_system.rs:…` |
| `FallbackAttempt` | tier + success + error + duration_ms | `fallback_system.rs:…` |
| `FallbackMetrics` | total_requests + tier_successes + tier_failures + average_attempts + average_duration_ms | `fallback_system.rs:…` |
| `HeuristicConfig` | rules: Vec<HeuristicRule>. Default returns 7 ordered rules (LowMorale<30 → HealSelf, etc.) | `heuristics.rs:5-49` |
| `HeuristicCondition` | `#[non_exhaustive] #[serde(tag="type")]` 6-variant: LowMorale{threshold} / LowAmmo{threshold} / EnemyNearby{max_distance} / EnemyVisible / ObjectiveContains{keyword} / Always | `heuristics.rs:57-67` |
| `HeuristicAction` | `#[non_exhaustive] #[serde(tag="type")]` 6-variant: HealSelf / Reload / AttackNearestEnemy / TakeCover{distance} / MoveToObjective / Scan{radius} | `heuristics.rs:69-79` |
| `PromptCompressor` | Stateless compressor. `compress_tactical_prompt()` static-str template targets 25-30% reduction. `compress(text)` removes stop-word list (a/an/the/is/are/was/were/that/this/of/to/in/on/at/by/for/with) and tightens punctuation | `compression.rs:24-77` |
| `ACTION_DOCS` / `COMPACT_SCHEMA` | Shared compact-string action documentation + JSON schema cribs used across compressed prompts | `compression.rs:17-21` |
| `StreamingBatchParser` | Progressive JSON-array parser for multi-agent batch responses. Yields `StreamedPlanEntry{agent_id, plan_id, steps}` as each object completes. State machine: WaitingForArrayStart → ParsingArray → Complete / Error | `streaming_parser.rs:39-67` |
| `MAX_PROMPT_LENGTH = 4096` | usize constant in `llm_adapter.rs:5`. Only enforced by `safe_llm_invoke(...)` which has **zero workspace callers** (verified 2026-05-12). | `llm_adapter.rs:5,8-19` |
| `safe_llm_invoke` | Stub-flavored input/output validator. File header line 1 says "stub". Output validation is `output.trim().starts_with('{')` — coarser than `parse_llm_response` | `llm_adapter.rs:1-19` |
| `LLM_CACHE_CAP` (env) | Global PromptCache capacity, default 4096 | `lib.rs:50-55` |
| `LLM_CACHE_SIM_THRESH` (env) | PromptCache similarity threshold, **default 1.0 (exact-match only)** — clamped to [0.0, 1.0]. Default chosen "to avoid nondeterministic cross-test pollution" per `lib.rs:57-65` comment | `lib.rs:60-65` |

#### 13.7.4 Files (subsystem-scoped)

| File | LoC | Role | Production-wired? |
|---|---|---|---|
| `astraweave-llm/src/production_hardening.rs` | 2249 | `ProductionHardeningLayer` composite + `HardeningConfig` + `HealthChecker` + `process_request` | **No** — only consumer is `examples/llm_production_hardening_demo.rs` |
| `astraweave-llm/src/rate_limiter.rs` | 752 | `RateLimiter` + `RateLimiterConfig` + `RateLimitContext` + `ModelRateLimit` + per-user/per-model/global tracking via tokio `Semaphore` | **No** — only consumer is the (dormant) `ProductionHardeningLayer` |
| `astraweave-llm/src/circuit_breaker.rs` | 969 | `CircuitBreakerManager` + per-model `CircuitBreaker` + `CircuitState{Closed,Open,HalfOpen}` + `circuit_breaker_execute!` macro | **No** — only consumer is the (dormant) `ProductionHardeningLayer` and `FallbackOrchestrator`'s embedded breaker |
| `astraweave-llm/src/backpressure.rs` | 1426 | `BackpressureManager` + 5-priority queue + tokio Semaphore concurrency + processor task | **No** — only consumer is the (dormant) `ProductionHardeningLayer` |
| `astraweave-llm/src/ab_testing.rs` | 1533 | `ABTestFramework` + `Experiment` + `Variant` + `ModelConfig` + `SuccessCriteria` + `AssignmentStrategy` + `ExperimentResults` | **No** — only consumer is the (dormant) `ProductionHardeningLayer` |
| `astraweave-llm/src/telemetry.rs` | 352 | `LlmTelemetry` — 12 AtomicU64 counters (zero-overhead) | **No** — consumers: `ProductionHardeningLayer` + `FallbackOrchestrator` (both dormant from a runtime perspective) |
| `astraweave-llm/src/retry.rs` | 431 | `RetryConfig` with `production()` / `aggressive()` / `disabled()` constructors + deterministic-jitter exponential backoff + seeded-RNG variant for replay control | **No** — referenced in tests; no production execute-with-retry call site found |
| `astraweave-llm/src/scheduler.rs` | 550 | `LlmScheduler` priority-queue ECS resource + `RequestPriority` + `RequestStatus` + `QueuedRequest` + `RequestResult` | **No** — only in own tests/examples + 1 reference in mutation tests |
| `astraweave-llm/src/fallback_system.rs` | 1786 | `FallbackOrchestrator` (4-tier) + simplified-tools registry + heuristic config + breaker | **Partial** — `Orchestrator` + `OrchestratorAsync` trait impls exist (plug-shape compatible with AIArbiter), but the only references in `astraweave-ai/src/{ai_arbiter,llm_executor}.rs` are **doc-comment examples** (lines 51, 58, 34, 88, 102). No `FallbackOrchestrator::new` instantiation in production. |
| `astraweave-llm/src/heuristics.rs` | 859 | `HeuristicConfig` (7-rule default) + `HeuristicRule::evaluate(snap, registry)` + `HeuristicCondition` (6 variants) + `HeuristicAction` (6 variants) | **No** — only consumer is the (dormant) `FallbackOrchestrator` Tier-3 path |
| `astraweave-llm/src/compression.rs` | 581 | `PromptCompressor` + `ACTION_DOCS` + `COMPACT_SCHEMA` + `compress_tactical_prompt()` / `compress_stealth_prompt()` static templates | **No** — referenced by `FallbackOrchestrator` Tier-2 path only |
| `astraweave-llm/src/plan_parser.rs` | 974 | `parse_llm_response` 5-stage chain + `ExtractionMethod{Direct,CodeFence,Envelope,ObjectExtraction,Tolerant}` + `ParseResult` | **Yes (partial)** — actively called from `fallback_system.rs:465` (Tier-1) and `:512` (Tier-2). The only output-side hardening primitive with confirmed production callers. |
| `astraweave-llm/src/tool_guard.rs` | 831 | `ToolGuard` + `ToolPolicy{Allowed,Restricted,Denied}` + `ValidationResult{Valid,Invalid,Denied}` + `AuditEntry` + DashMap-backed policy + audit log | **No** — only in own file + `mutation_resistant_comprehensive_tests.rs` (verified via workspace grep) |
| `astraweave-llm/src/streaming_parser.rs` | 434 | `StreamingBatchParser` for progressive JSON-array parsing in multi-agent batch responses + `StreamedPlanEntry` | **No** — referenced from `LlmClient::complete_streaming` trait default impl path; no orchestrator consumer wires it |
| `astraweave-llm/src/llm_adapter.rs` | 272 | Stub `safe_llm_invoke` + `MAX_PROMPT_LENGTH = 4096` + length validators + `mock_llm_call` placeholder | **No** — file comment line 1 says "stub". Workspace grep finds zero callers of `safe_llm_invoke` outside the file's own tests. Resolves parent §11 "`llm_adapter.rs` — stub vs production?" → stub remains pure stub. |
| `astraweave-llm/src/batch_executor.rs` | 763 | `BatchInferenceExecutor` + `AgentId` — multi-agent batched LLM inference (orthogonal to single-request hardening but lives in the same crate) | **Active** — verified 2026-05-12: `BatchInferenceExecutor::new()` is instantiated by `FallbackOrchestrator` at `fallback_system.rs:369` and consumed by `examples/llm_production_hardening_demo.rs` + `astraweave-llm/examples/batch_production_validation.rs`. Used to back the LLM-tier paths inside `FallbackOrchestrator`. |

**Subsystem total:** 16 files, ~14,800 LoC (excluding `cache/` and clients/prompts which were inventoried in parent §5).

**Test surface (subsystem-scoped):**
- `astraweave-llm/tests/fallback_chain_integration.rs` — `FallbackOrchestrator` chain tests
- `astraweave-llm/tests/phase7_integration_tests.rs` — references `FallbackOrchestrator::new` per workspace grep
- `astraweave-llm/tests/timeout_retry_tests.rs` — references `RateLimiter::new` and circuit-breaker behaviors
- `astraweave-llm/tests/concurrent_stress_tests.rs` — references `RateLimiter::new` and stress patterns
- `astraweave-llm/tests/boundary_condition_tests.rs` — references rate-limiter + circuit-breaker + parse_llm_response
- `astraweave-llm/tests/error_message_validation_tests.rs` — references parse_llm_response + RateLimiter
- `astraweave-llm/tests/property_tests.rs` — circuit-breaker + parse_llm_response property-based tests
- `astraweave-llm/tests/mutation_resistant_comprehensive_tests.rs` — references `RequestPriority`/`RequestStatus` (scheduler), `parse_llm_response`, `RateLimiter`, `ToolGuard`, circuit-breaker — the single broadest subsystem-spanning test surface
- `examples/llm_production_hardening_demo.rs` — the only standalone example that constructs `ProductionHardeningLayer::new(HardeningConfig::default())` and exercises `process_request` end-to-end

#### 13.7.5 Touchpoints (subsystem-scoped)

**Upstream (what feeds this subsystem)** — *design-intent; only the parse_llm_response path is actually fed in production:*

- **Designed:** Any `Orchestrator` implementor that calls `LlmClient::complete`. Architectural intent is that `LlmExecutor` (`astraweave-ai/src/llm_executor.rs`) constructs an `Arc<FallbackOrchestrator>` (per its own doc-comments at `:34, :88, :102`) which in turn wraps a `LlmClient`. The Arbiter (`astraweave-ai/src/ai_arbiter.rs:51,58`) doc-comments document the same pattern: `let llm_orch = Arc::new(FallbackOrchestrator::new(llm_client, default_tool_registry()))`.
- **Actual (verified 2026-05-12):** Neither the doc-comment pattern nor any equivalent is implemented in production code. The Arbiter's strategic/fast executors receive `Arc<dyn LlmExecutor>` directly with no fallback wrapping. The single active hardening path is `FallbackOrchestrator::generate_plan` → `parse_llm_response` (`fallback_system.rs:465`, `:512`) — but `FallbackOrchestrator::new` itself has no production caller.
- **Telemetry:** `LlmTelemetry` instances are constructed by `ProductionHardeningLayer::new` (`production_hardening.rs:311`) — dormant — and also embedded in `FallbackOrchestrator` — dormant from a constructor perspective.

**Downstream (what consumes this subsystem's output)** — *current production surface is empty; designed downstream is the Arbiter / ECS planning system:*

- **Designed:** `AIArbiter::ExecutingLLM{step_index}` consumes `PlanIntent` from `FallbackOrchestrator::plan_async`. `ecs_ai_plugin::sys_ai_planning` consumes `PlanIntent` from an `Orchestrator::propose_plan` call (parent §2.1, §2.2).
- **Actual:** The runtime `AIArbiter` and `LlmExecutor` paths call `LlmClient::complete` and parse responses directly (via `serde_json::from_str` in current call sites), not via `parse_llm_response`. The 5-strategy parser is reached only when `FallbackOrchestrator` is constructed and used — which today happens only in `astraweave-llm`'s own tests, `examples/llm_integration` (per workspace grep), and `astraweave-llm/examples/batch_production_validation.rs`.
- **HardeningResult:** Zero consumers — `process_request` is never called in production.

**Bidirectional / coupled:**

- **`FallbackOrchestrator` ↔ `parse_llm_response`:** the only LIVE coupling in this subsystem. The orchestrator owns 5-strategy parsing for both Tier-1 (full registry) and Tier-2 (15-tool simplified registry) paths (`fallback_system.rs:465,512`). The parser does not know about the orchestrator.
- **`ProductionHardeningLayer` ↔ all 6 primitives:** owns `Arc<RateLimiter>` + `Arc<CircuitBreakerManager>` + `Arc<RwLock<BackpressureManager>>` + `Arc<ABTestFramework>` + `Arc<LlmTelemetry>` + `Arc<RwLock<HealthChecker>>` + tokio shutdown_tx/rx watch channel + `Arc<tokio::sync::RwLock<Option<JoinHandle<()>>>>` for the background health-check task (`production_hardening.rs:304-329`). Cross-primitive synchronization is via `Arc<RwLock<…>>` throughout — no lock-free path.
- **`RetryConfig` ↔ `LlmTelemetry`:** designed to coordinate (retry record telemetry via `record_retry()`), but no actual integration code wires them.
- **`Priority` (backpressure) vs `RequestPriority` (scheduler):** **naming collision risk** — two distinct enums with overlapping intent but different variant counts (5 vs 3) and different numeric encodings (Critical=0 lowest in backpressure; Low=0 lowest in scheduler — opposite conventions). Verified 2026-05-12: no production code mixes them (workspace grep matches only `astraweave-llm/tests/mutation_resistant_comprehensive_tests.rs:27,1376` and `examples/llm_production_hardening_demo.rs:10`).

#### 13.7.6 Subsystem-specific invariants

1. **`parse_llm_response` is the canonical LLM-output parser.** Any orchestrator that calls `LlmClient::complete` and parses the response with anything other than `parse_llm_response` is **bypassing the 5-strategy resilience chain** (`plan_parser.rs:72-…`). Per parent §7 Decision Log "5-strategy LLM-response extraction" and parent Appendix A point 9: "Don't roll your own JSON parsing — use `astraweave-llm::plan_parser::parse_llm_response`." **This invariant is currently violated** by the runtime `AIArbiter` / `LlmExecutor` path — see §13.7.7 below.
2. **`MAX_PROMPT_LENGTH = 4096` is enforced ONLY in `safe_llm_invoke`.** `llm_adapter.rs:9-11`. `safe_llm_invoke` has zero workspace callers (verified 2026-05-12). Therefore the 4096-character ceiling is **never enforced at runtime**. Direct `LlmClient::complete(prompt)` paths receive any-length prompts. This resolves parent §11 "MAX_PROMPT_LENGTH=4096 — uniform enforcement?" — the answer is NO, the limit is currently a documented stub-only invariant. See §13.7.7 below.
3. **`HardeningResult<T>` is `#[non_exhaustive] #[must_use]`.** Future variants are forward-compatible; ignoring the result is a clippy warning. Confirmed `production_hardening.rs:110-137`.
4. **`ValidationResult` is `#[non_exhaustive] #[must_use]`.** Same forward-compat + ignored-result guard for `ToolGuard::validate_action`. Confirmed `tool_guard.rs:…`.
5. **All telemetry counters use `Ordering::Relaxed`.** No memory-ordering coupling between counters. This is intentional (telemetry is best-effort) but means counters cannot be used for synchronization or causal ordering. Confirmed `telemetry.rs:50-100` (all `fetch_add(..., Ordering::Relaxed)`).
6. **`RetryConfig::backoff_for_attempt` is deterministic.** Same attempt number always produces same jitter (via `attempt.wrapping_mul(0x517cc1b727220a95)` per `retry.rs:75-78`). Replay-safe. `backoff_for_attempt_seeded<R: Rng>` provides an alternative when external RNG control is required.
7. **`PromptCache` global default is exact-match-only.** `lib.rs:60-65`: `LLM_CACHE_SIM_THRESH` defaults to 1.0 (clamped [0.0, 1.0]). The comment at `:57-58` explicitly states this is to "avoid nondeterministic cross-test pollution." This resolves parent §11 "`PromptCache` similarity threshold — should it be off by default?" — the answer is **already off by default**; the directional question becomes "should the feature be removed entirely?" rather than "should the default change?"
8. **`Priority::all()` returns canonical ordering.** Critical first, Background last. Any consumer that iterates priorities for drain order MUST use `Priority::all()` rather than hand-rolled ordering. `backpressure.rs:…`.
9. **`CircuitState` requires `minimum_requests = 10` BEFORE tripping.** `circuit_breaker.rs:…` CircuitBreakerConfig. Below 10 requests, no breaker decision occurs even if failure_threshold (5) is met. Prevents pathological tripping during cold start.
10. **Background tasks (health checker, backpressure processor) MUST be shut down via the watch channel.** `production_hardening.rs:315` creates `tokio::sync::watch::channel(false)`; `ProductionHardeningLayer::shutdown()` (verified 2026-05-12 at `production_hardening.rs:597`) flips it to `true`. Tasks polling the receiver exit on `Ok(true)`. **Verified 2026-05-12: `ProductionHardeningLayer` does NOT implement `Drop`** — only `BackpressureManager` has `impl Drop` (`backpressure.rs:595-601`, which aborts its `processor_handle`). The health-checker task is stored in `health_checker_handle: Arc<RwLock<Option<JoinHandle<()>>>>` (`production_hardening.rs:36-37`); dropping `ProductionHardeningLayer` without calling `shutdown().await` first leaves the JoinHandle in the Option un-aborted. **This is a real runtime resource leak risk** when consumers don't call shutdown explicitly.
11. **`FallbackTier::next()` is the only authorized way to advance the fallback chain.** Per parent §7 Decision Log "4-tier LLM fallback chain". Manual tier construction bypasses metrics + duration tracking + emergency-tier guarantees. `fallback_system.rs:…`.
12. **`ExtractionMethod` carries no payload.** Unit variants only. The parser records which strategy worked, but does not surface the intermediate transform (e.g. the un-fenced JSON or the normalized object). Consumers needing the un-transformed text must hold the original response separately.
13. **`HeuristicCondition::Always` MUST be the last rule in any `HeuristicConfig`.** Default config (`heuristics.rs:43-45`) places it last with `HeuristicAction::Scan{radius:10.0}` as the unconditional fallback. **Verified 2026-05-12 at `fallback_system.rs:525-548` (`try_heuristic`):** iterates `for rule in &self.heuristic_config.rules` and **short-circuits on the first rule that produces an action** via `break` at line 534. Therefore: if `Always` is placed before any other rule, the later rules are unreachable; if `Always` is omitted entirely, Tier-3 may return an empty plan and escalate to Tier-4 Emergency (which then `:541-548` adds a `Scan` fallback if the registry has it).

#### 13.7.7 Subsystem-specific open questions

- **The runtime AI path bypasses the entire hardening surface.** [Decisional / factual, **HIGH-IMPACT finding**.] Factual state (verified 2026-05-12): the production `AIArbiter` consumes `Arc<dyn LlmExecutor>` directly, and `LlmExecutor::execute_plan` (per `astraweave-ai/src/llm_executor.rs` doc-comments) is *designed* to delegate to `FallbackOrchestrator` but the actual implementation does not appear to wrap the client this way — `FallbackOrchestrator::new` has zero non-doc-comment, non-test call sites. The consequence: every LLM call in production goes through `LlmClient::complete` directly, without rate limiting, without circuit breaking, without backpressure, without A/B routing, without retry, without telemetry record, without 5-strategy parsing, without ToolGuard validation, and without 4-tier fallback. The 15K LoC of hardening code is essentially shelf-stocked but not in line. Three directional options (mirroring §13.6 Advanced GOAP and other dormant subsystems): commit (instantiate `FallbackOrchestrator::new(client, registry)` in `LlmExecutor::new` or `AIArbiter::new` and wire `ProductionHardeningLayer::new(HardeningConfig::default()).start().await` as a startup step), prune (delete `production_hardening.rs` + the 5 unwired primitives + `safe_llm_invoke`), or rebrand (move to `astraweave-llm-hardening` or experimental crate). **The active `parse_llm_response` path via `FallbackOrchestrator` is the wedge — making `LlmExecutor` consume `FallbackOrchestrator` is the smallest production-wiring step that activates the most of the hardening surface.**
- **`Priority` (backpressure) vs `RequestPriority` (scheduler) naming collision with opposite numeric conventions.** [Factual / decisional, mirrors parent §13.1 Memory `Episode` vs `GameEpisode` collision pattern.] Factual: `backpressure::Priority` has 5 variants with Critical=0 (lowest numeric = highest priority); `scheduler::RequestPriority` has 3 variants with Low=0 (lowest numeric = lowest priority). Crossing the two enums would invert priority ordering silently. **Verified 2026-05-12: no production code mixes them.** Workspace grep for `backpressure::Priority` and `scheduler::RequestPriority` matches only `astraweave-llm/tests/mutation_resistant_comprehensive_tests.rs:27,1376` and `examples/llm_production_hardening_demo.rs:10` (the standalone demo). No cross-module production import or test mixes the two. Either rename one (canonical: `BackpressurePriority` + `SchedulerPriority`), unify (single enum with both lattice points), or document the convention split clearly at definition. Adds to the parent §11 list of naming-collision open questions.
- **The 4096-char prompt limit is enforced only in unreachable code.** [Decisional, **resolves parent §11 question**.] `safe_llm_invoke` (`llm_adapter.rs:8-19`) is the only enforcement point and has zero callers. Resolution path: (a) add length-check at the top of `ProductionHardeningLayer::process_request` reading `request.estimated_tokens` and rejecting `> max_prompt_tokens` (config field would need to be added); (b) inline `safe_llm_invoke`-style check into `FallbackOrchestrator::generate_plan` so Tier-1/Tier-2 prompts are length-validated before LLM call; (c) push the constant into `astraweave-core` and have it be enforced at prompt-template-builder time. Path (b) reuses the active path; path (a) only activates if `ProductionHardeningLayer` is also production-wired. Directional question for Andrew.
- **`safe_llm_invoke` is a pure stub today.** [Decisional, **resolves parent §11 question**.] Factual: file header line 1 explicitly says "stub"; `mock_llm_call` placeholder (`llm_adapter.rs:21-27`) just echoes the prompt back in a fake JSON envelope; zero workspace callers. The intent (per the file-level comments) was to be a defense-in-depth choke point. Three options: (a) promote — replace `mock_llm_call` with a real `LlmClient::complete` call and route ALL production LLM traffic through `safe_llm_invoke`; (b) delete the file and rely on `ProductionHardeningLayer` + `parse_llm_response` + `ToolGuard` (the three-layer defense from parent §7 Decision Log "Defense-in-depth validation"); (c) keep as a test fixture and rename to `mock_llm_adapter.rs` to make the stub status visible at import-site. The historical intent appears to be (a), but the actual development trajectory has implemented (b) for the parse step (via `parse_llm_response`) without ever centralizing the input-side gate. Resolution suggested: option (b) — delete the file, document the choke-point migration in §7 Decision Log.
- **Tier-3 `HeuristicConfig` vs `RuleOrchestrator` overlap — enriched.** [Decisional, **enriches parent §11 question with concrete diff**.] Factual (verified 2026-05-12): `HeuristicConfig::default()` (heuristics.rs:9-49) produces 7 rules: LowMorale<30 → HealSelf; LowAmmo<=0 → Reload; EnemyNearby<=3.0 → AttackNearestEnemy; EnemyVisible → TakeCover{2.0}; ObjectiveContains("extract") → MoveToObjective; ObjectiveContains("reach") → MoveToObjective; Always → Scan{10.0}. By contrast, `RuleOrchestrator` (in `astraweave-ai/src/orchestrator.rs`) is hardcoded smoke-and-advance logic. The `HeuristicConfig` is **strictly more expressive** (configurable, serializable, supports custom rules) but used in only ONE place (FallbackOrchestrator Tier-3, dormant). The `RuleOrchestrator` is used as the default for low-tier game scenarios. Decisional question: should `RuleOrchestrator` be deprecated in favor of `HeuristicConfig`-based rules (gaining configurability) or should `HeuristicConfig` be folded into `RuleOrchestrator` (gaining a production caller)? Same directional shape as the parent §11 Advanced GOAP question.
- **`#[allow(dead_code)]` markers on scheduler internals.** [Factual / informational.] `scheduler.rs:68` marks `QueuedRequest` as `#[allow(dead_code)]`; `:85` marks `client: Arc<dyn LlmClient>` field; `:90` marks `max_concurrent: usize`. The struct exists but several fields are never read from anywhere outside the constructor (the actual scheduling happens via `request_tx` channel + `statuses`/`results` DashMaps). This is the same "reserved for future…" pattern surfaced in parent §13.5 Coordination (which had 7+ such markers). The directional question: is `LlmScheduler` in mid-build (the unused fields will be wired) or is it post-build leftover (the unused fields are residue from an earlier API)?
- **`streaming_parser.rs` requires `complete_streaming` to be implemented by clients.** [Factual — resolved 2026-05-12.] `StreamingBatchParser` (`streaming_parser.rs:39-67`) is fed via `LlmClient::complete_streaming` (default impl at `lib.rs:112-…` calls `complete()` and wraps single-chunk — so the default impl yields no streaming benefit). Verified 2026-05-12: **`hermes2pro_ollama.rs:516` overrides `complete_streaming`**; **`qwen3_ollama.rs:741` overrides `complete_streaming`**; **`phi3.rs` and `phi3_ollama.rs` do NOT override** (no `async fn complete_streaming` declaration found in either, so they inherit the single-chunk wrapping default). 2 of 4 production clients support true streaming. Phi-3 family clients are dormant for streaming use cases.
- **`ABTestFramework::auto_winner_selection = false` default — never trips automatically.** [Decisional / factual.] `ab_testing.rs:42` defaults this to false. Manual experiment-end is required even after significance threshold is reached. Whether the default should be true (auto-stop) or stay false (require human judgment for prompt/model promotions) is a policy decision tied to deployment governance, not just code.
- **`RetryConfig::production()` allows 3 attempts with 50ms→500ms backoff. Where is this consumed?** [Factual / informational.] `retry.rs:27-35` defines the production preset, but workspace grep finds no `RetryConfig::production()` call sites in production code paths. The retry logic itself (the `backoff_for_attempt`/`_seeded` math) is well-tested but the policy is unwired. Same "implementation without consumer" pattern as the broader hardening surface.
- **`PromptCompressor` static-template return — `&'static str` is correct?** [Factual / informational.] `compression.rs:62-77`: `compress_tactical_prompt()` returns `&'static str` (the raw-string literal). No allocation, no formatting. However, `compress(&self, text: &str) -> String` (`:39-60`) allocates per call. Pattern is fine architecturally; just confirming that the static templates are the fast path and the dynamic `compress` is the slow path.
- **`HealthChecker` initializes only 5 components.** [Factual.] `production_hardening.rs:179-185` hardcodes the initial component list to `["rate_limiter", "circuit_breaker", "backpressure", "telemetry", "ab_testing"]`. `parse_llm_response`, `ToolGuard`, `FallbackOrchestrator`, `LlmScheduler`, `PromptCompressor` are NOT registered. The health-check surface is incomplete relative to the full hardening surface. Decisional question: should the output-side primitives also be health-checked, or are they considered "stateless" and therefore not subject to liveness probes?
- **The example file `llm_production_hardening_demo.rs` is the only end-to-end exercise.** [Factual.] If this file is broken (e.g. drifts from the API after a refactor of `HardeningConfig`), there is no production usage to keep it honest. Same dormancy pattern as Advanced GOAP's `bin.disabled/` tools. Recommendation: either restore production wiring (which would keep the example honest) or move the file to `astraweave-llm/examples/` (already there per the grep result — check naming).

---

### 13.8 Subsystem Trace — RAG (Retrieval-Augmented Generation)

**Last verified against commit:** `32afac52f`
**Last verified date:** 2026-05-12
**Status:** **Three-crate stack: foundation crates active for their primitives; pipeline crate composed but dormant for the runtime AI loop.** Verified 2026-05-12 across 3 crates (`astraweave-rag` 6 files / ~6.5K LoC, `astraweave-embeddings` 4 files / ~3.2K LoC, `astraweave-context` 5 files / ~2.6K LoC = ~12.3K LoC total). **Foundation primitives have real users:** `astraweave-context::TokenCounter::new` is consumed by `astraweave-rag::RagPipeline::new` (`pipeline.rs:198`); `astraweave-embeddings::VectorStore::new` is consumed by tests + benches + the `VectorStoreWrapper` indirection. **Pipeline composite is dormant for the runtime AI loop:** `astraweave-rag::RagPipeline` has five workspace consumers that all hold it as a field (`astraweave-director/src/llm_director.rs:196`, `astraweave-quests/src/llm_quests.rs:196`, `astraweave-persona/src/llm_persona.rs:22`, `astraweave-dialogue/src/llm_dialogue.rs:423`, `astraweave-coordination/src/world_events.rs:423` and `narrative_coherence.rs`), but per §13.2 / §13.4 / §13.5, each of those consumers is itself dormant — the basic non-LLM layer of every consumer crate does NOT touch RAG, and the LLM-enhanced sub-layers (LlmDirector, LlmQuestGenerator, LlmPersona, LlmEnhancedDialogue, NarrativeCoherenceEngine) have no production constructors. **Parallel-implementation drift surfaced:** `astraweave-ai/src/rag/pipeline.rs:115` contains a SECOND `RagPipeline` struct (simpler — only retrieval + injection, no consolidation/forgetting/diversity); it is NOT declared in `astraweave-ai/src/lib.rs` (verified — no `pub mod rag` declaration) and is therefore orphaned source code that never compiles, plus the parallel `astraweave-ai/src/persona/manager.rs` follows the same orphaned-source pattern. **Internal duplication:** `astraweave-rag` itself has TWO `InjectionConfig` + TWO `InjectionResult` structs (one in `lib.rs:111-140` used by `RagPipeline`, one in `injection.rs:11-59` used by a standalone `InjectionEngine`) — same-crate naming collision.

#### 13.8.1 Role within the parent system

RAG (Retrieval-Augmented Generation) is the **semantic memory recall layer** designed to inject contextually relevant past experiences into LLM prompts. It sits between `astraweave-memory` (which stores hierarchical memory types and runs Phase-4 learning) and the LLM-enhanced AI surfaces (Director, Quests, Persona, Dialogue, Coordination), per parent §5's row for `astraweave-director/src/llm_director.rs`: *"Pulls in `astraweave-llm::LlmClient`, `astraweave-rag::RagPipeline`, `astraweave-context::ConversationHistory`, `astraweave-prompts::{PromptLibrary, PromptTemplate}`."* The architectural intent is that any LLM-driven AI subsystem holds an `Arc<RagPipeline>` field and calls `pipeline.inject_context(base_prompt, query)` (`pipeline.rs:402-405`) or `pipeline.retrieve(query, k)` (`:278-281`) before LLM invocation, transforming the bare prompt into a memory-augmented prompt.

The 3-crate stack composes as follows: `astraweave-embeddings` provides the embedding client trait + vector store + HNSW indexing primitives; `astraweave-context` provides token counting + conversation history + sliding-window context management + LLM-driven summarization; `astraweave-rag` composes both crates plus an `LlmClient` to provide the full retrieve → diversify → order → token-bound → inject pipeline along with autonomous consolidation (memory merging based on similarity/importance/recency strategies) and forgetting (importance-weighted exponential decay with category protection). Note that `astraweave-memory` (parent §13.1) and `astraweave-rag` are **two parallel memory subsystems** in the workspace — they do not share types (`astraweave-rag::RagPipeline` works with `astraweave-embeddings::Memory`, not with `astraweave-memory::MemoryRecord` or `astraweave-memory::persona::Persona`). The relationship between the two subsystems is unspecified and constitutes a parent-level open question.

#### 13.8.2 Authoritative pipeline

```text
[Caller — designed to instantiate RagPipeline once per AI agent]
    │
    │ Designed: let pipeline = Arc::new(RagPipeline::new(
    │     embedding_client: Arc<dyn EmbeddingClient>,
    │     vector_store: Arc<dyn VectorStoreInterface>,
    │     llm_client: Option<Arc<dyn LlmClient>>,
    │     config: RagConfig::default(),
    │ ));
    │ // ... stored as field in LlmDirector / LlmQuestGenerator / LlmPersona / etc.
    │
    │ Actual: no production constructor anywhere outside dormant consumers' own tests
    │
    ▼
[R1 — Memory ingestion]                          (path: add_memory / add_memory_async)
    file: astraweave-rag/src/pipeline.rs:225-275 (add_memory), :623-663 (add_memory_async)
    role: Wrap raw text in Memory{id: Uuid, text, timestamp, importance:1.0, valence:0.0,
          category: MemoryCategory::Gameplay (default), entities:[], context:{}}.
          add_memory_async is the Arc-safe variant — uses interior mutability via
          parking_lot RwLocks, holds no lock across .await
    │
    ▼
[R2 — Embedding generation]
    file: astraweave-embeddings/src/client.rs (EmbeddingClient trait)
    role: Call self.embedding_client.embed(&memory.text).await -> Vec<f32>
          Default dimensions: 384 (matches sentence-transformers/all-MiniLM-L6-v2 per
          astraweave-embeddings/src/lib.rs:70-71). Distance metric: Cosine (`:74`).
    │
    ▼
[R3 — Vector store insertion]
    file: astraweave-embeddings/src/store.rs:73-109 (VectorStore::insert)
    role: Validate dim == config.dimensions. Reject if at capacity (max_vectors).
          Convert Memory into StoredVector{id, vector, text, timestamp, importance,
          metadata: HashMap<String, String>} — the metadata field stringifies entities,
          category (serde-json), valence (string), and ctx_* keys per
          VectorStoreWrapper::insert_memory (pipeline.rs:89-107)
    │
    ▼
[R4 — Consolidation trigger check]               (autonomous, every add_memory)
    file: astraweave-rag/src/pipeline.rs:499-523 (should_consolidate),
          :526-580 (trigger_consolidation)
    role: If config.consolidation.enabled AND memories_since_consolidation >=
          trigger_threshold (default 100) OR time_since_last >= consolidation_interval
          (default 3600s = 1 hour), launch consolidation:
          - Get all memories via vector_store.get_all_memories() (NB: simplified
            implementation at pipeline.rs:166-170 returns empty vec — see §13.8.7)
          - Run ConsolidationEngine::consolidate(memories) per
            astraweave-rag/src/consolidation.rs (strategies:
            Importance / Recency / Similarity / Hybrid; default Importance,
            merge_similarity_threshold default 0.85)
          - Remove memories no longer present, regenerate embeddings + insert new
            merged results
    │
    │ ── parallel triggerable ──
    ▼
[R5 — Forgetting (manual trigger via trigger_forgetting)]
    file: astraweave-rag/src/pipeline.rs:583-614 (trigger_forgetting),
          forgetting.rs:108-170 (ForgettingEngine::process_forgetting)
    role: Exponential decay: strength = initial * exp(-effective_decay * hours_since_access)
          where effective_decay = base_decay_rate / (1 + importance_factor * memory.importance)
          (defaults: base 0.1/hour, importance_factor 2.0 → high-importance memories decay
           much slower). Forced forgetting at max_memory_age (default 30 days).
          Protected categories (default: [Quest]) bypass forgetting entirely.
          NOTE: trigger_forgetting is NOT called automatically — only consolidation is
          auto-triggered. Forgetting requires explicit pipeline.trigger_forgetting().await
    │
    ▼ Retrieval path
[R6 — Retrieve query entry]                      (path: retrieve / retrieve_with_query)
    file: astraweave-rag/src/pipeline.rs:278-388 (retrieve_with_query)
    role: Build MemoryQuery (text/time_range/categories/entities/min_importance/
          max_age/metadata_filters). Compute cache_key via DefaultHasher
          (`:795-805`). Check cache (TTL default 300s = 5 min, max size 1000).
    │
    ▼
[R7 — Embed query + search]
    file: astraweave-rag/src/pipeline.rs:301-304
    role: self.embedding_client.embed(&query.text).await -> Vec<f32>
          self.vector_store.search(&query_embedding, k * 2).await
          (oversamples 2× for filtering headroom)
    │
    ▼
[R8 — Filter + convert]
    file: astraweave-rag/src/pipeline.rs:307-367, passes_filters at :665-706
    role: For each SearchResult, deserialize entities/category/valence/context_*
          from metadata HashMap. Apply MemoryQuery filters in order:
          time_range / categories / entities (matches memory.entities OR text-contains
          case-insensitive) / min_importance / max_age. Build RetrievedMemory{memory,
          similarity_score, rank, metadata: RetrievalMetadata{query, method:
          RetrievalMethod::SemanticSearch (always — never Keyword/Temporal/
          Category/Hybrid; see §13.8.7), retrieved_at, processing_time_ms, context}}
    │
    ▼
[R9 — Diversity (if config.diversity.enabled)]
    file: astraweave-rag/src/pipeline.rs:708-744 (apply_diversity)
    role: Greedy MMR-like: always include the most similar, then iteratively pick
          candidates that maximize (similarity_score + diversity_bonus) where
          diversity_bonus = min_text_distance_to_existing * diversity_factor
          (default 0.3). text_distance is Jaccard similarity over whitespace tokens
          (pipeline.rs:899-911). diversity.strategy enum has 4 variants
          (Semantic/Temporal/Category/Combined) but the implementation uses ONLY
          text-token Jaccard regardless of strategy field — see §13.8.7
    │
    ▼
[R10 — Order + cache]
    file: astraweave-rag/src/pipeline.rs:747-792 (order_results)
    role: Sort by OrderingStrategy (7 variants: SimilarityDesc/Asc, RecencyDesc/Asc,
          ImportanceDesc/Asc, Mixed — Mixed uses rand::rng().shuffle).
          Cache result (LRU-ish; cache_result at :829-852 evicts 25% when full)
    │
    ▼
[R11 — Context injection (inject_context / inject_context_detailed)]
    file: astraweave-rag/src/pipeline.rs:402-486
    role: Format memories as "[Score: {f:.2}] {text}" if include_metadata else "{text}",
          join with newlines. If memory_tokens > config.injection.max_context_tokens
          (default 1024) AND config.injection.enable_summarization AND llm_client.is_some()
          → call summarize_memories (LLM-based summarization, `:855-868`).
          Else truncate via token_counter.truncate_to_tokens.
          Apply template: default "Relevant memories:\n{memories}\n\nNow respond to: {query}"
          (`lib.rs:132-133`). Replace {memories}, {query}, {prompt} placeholders.
    │
    ▼
InjectionResult{enhanced_prompt, injected_memories, context_tokens, metadata:
    InjectionMetadata{original_prompt, query, strategy: InjectionStrategy
    (Prepend/Append/Insert/Interleave/Replace — but pipeline.rs hardcodes Insert
    at :481; see §13.8.7), processing_time_ms, summarized: bool}}
    │
    ▼
[Caller — LLM call with augmented prompt]
    → llm_client.complete(enhanced_prompt).await
```

**Critical observation:** Steps R1-R11 execute correctly ONLY when a consumer (Director / Quests / Persona / Dialogue / Coordination) is itself constructed in production. Per §13.2 (Director), §13.4 (Dialogue), §13.5 (Coordination), the LLM-enhanced sub-layers of those consumers have zero production constructors. `LlmQuestGenerator::new` (`astraweave-quests/src/llm_quests.rs:224`) is referenced only from `astraweave-quests/src/systems.rs:6` (which itself is gated to test-only consumption per the same crate's structure) and the crate's own tests (`:503, :525`). `LlmPersona::new` (`astraweave-persona/src/llm_persona.rs:274`) has no production callers. The full retrieve → diversify → inject pipeline therefore never executes in the runtime AI loop today.

#### 13.8.3 Subsystem-specific vocabulary (additive to parent §3)

| Term | Definition | Source |
|---|---|---|
| `RagPipeline` (canonical) | The composed RAG pipeline: embedding client + vector store + LLM client + token counter + config + metrics + cache + consolidation engine + forgetting engine | `astraweave-rag/src/pipeline.rs:21-51` |
| `RagPipeline` (inner astraweave-ai duplicate) | A second, simpler `RagPipeline` defined inside `astraweave-ai/src/rag/pipeline.rs:115` (only embedding_client + vector_store + config). **Orphaned**: `pub mod rag` is not declared in `astraweave-ai/src/lib.rs` (verified — no rag module declaration). The source file exists on disk but never compiles. | `astraweave-ai/src/rag/{mod,pipeline}.rs` (orphaned) |
| `VectorStoreInterface` | Async trait: `search(query_vector, k) -> Vec<SearchResult>`, `insert(id, vector, text)`, `insert_memory(memory, vector)`, `get(id)`, `remove(id)`, `len()`, `is_empty()`, `get_all_memories()` | `pipeline.rs:54-66` |
| `VectorStoreWrapper` | Adapter wrapping `astraweave-embeddings::VectorStore` to implement `VectorStoreInterface`. **Note: `get_all_memories` simplified impl returns empty Vec** (`pipeline.rs:166-170`) — consolidation cannot actually fetch memories through the canonical wrapper. See §13.8.7 | `pipeline.rs:69-171` |
| `RagConfig` | Top-level config: max_retrieval_count (10) + min_similarity_score (0.3) + ConsolidationConfig + ForgettingConfig + InjectionConfig + DiversityConfig + PerformanceConfig | `lib.rs:72-108` |
| `InjectionConfig` (canonical, in lib.rs) | injection_template (default `"Relevant memories:\n{memories}\n\nNow respond to: {query}"`) + max_context_tokens (1024) + include_metadata (true) + ordering_strategy (`SimilarityDesc`) + enable_summarization (true) | `lib.rs:111-140` |
| `InjectionConfig` (alternate, in injection.rs) | **DUPLICATE STRUCT NAME, DIFFERENT FIELDS:** max_memories (5) + relevance_threshold (0.4) + prioritize_recent (true) + max_context_tokens (2000). Used by the standalone `InjectionEngine`, NOT by `RagPipeline`. Internal architecture drift. | `injection.rs:11-33` |
| `InjectionResult` (canonical, in lib.rs) | enhanced_prompt + injected_memories: Vec<RetrievedMemory> + context_tokens + metadata: InjectionMetadata | `lib.rs:324-337` |
| `InjectionResult` (alternate, in injection.rs) | **DUPLICATE STRUCT NAME, DIFFERENT FIELDS:** injected_memories: Vec<Memory> + context_text + relevance_scores + estimated_tokens. Used by `InjectionEngine`, NOT by `RagPipeline`. | `injection.rs:49-59` |
| `MemoryQuery` | Builder-style query: text + Option<time_range> + Vec<MemoryCategory> categories + Vec<String> entities + Option<min_importance> + Option<max_age> + metadata_filters | `lib.rs:374-436` |
| `RetrievedMemory` | memory: Memory + similarity_score: f32 + rank: usize + metadata: RetrievalMetadata | `lib.rs:238-252` |
| `RetrievalMetadata` | query + method: RetrievalMethod + retrieved_at + processing_time_ms + context: HashMap<String, Value> | `lib.rs:255-271` |
| `RetrievalMethod` | `#[non_exhaustive]` 5-variant: SemanticSearch / KeywordSearch / TemporalSearch / CategorySearch / HybridSearch. **Actual implementation hardcodes `SemanticSearch`** (`pipeline.rs:357`) — other variants are vocabulary-only | `lib.rs:274-287` |
| `OrderingStrategy` | `#[non_exhaustive]` 7-variant: SimilarityDesc/Asc, RecencyDesc/Asc, ImportanceDesc/Asc, Mixed (uses rand::rng().shuffle) | `lib.rs:142-160` |
| `DiversityStrategy` | `#[non_exhaustive]` 4-variant: Semantic / Temporal / Category / Combined. **The implementation `apply_diversity` ignores this field and uses Jaccard token distance regardless** | `lib.rs:189-201` |
| `InjectionStrategy` (canonical) | `#[non_exhaustive]` 5-variant: Prepend / Append / Insert / Interleave / Replace. **`inject_context_detailed` hardcodes `Insert` at `pipeline.rs:481`** — other variants are vocabulary-only | `lib.rs:358-372` |
| `InjectionStrategy` (inner astraweave-ai) | 2-variant: Prepend / Append. Inner crate has its own (and orphaned) variant | `astraweave-ai/src/rag/mod.rs:36-43` |
| `ConsolidationConfig` | enabled (true) + trigger_threshold (100) + merge_similarity_threshold (0.85) + max_memories_per_batch (50) + strategy (Importance) + consolidation_interval (3600s) + max_age_seconds (86400s) | `consolidation.rs:14-51` |
| `ConsolidationStrategy` | `#[non_exhaustive]` 4-variant: Importance / Recency / Similarity / Hybrid | `consolidation.rs:54-65` |
| `ForgettingConfig` | enabled (true) + base_decay_rate (0.1/hour) + importance_factor (2.0) + min_importance_threshold (0.2) + max_memory_age (30 days) + cleanup_interval (1 day) + protected_categories ([Quest]) | `forgetting.rs:11-47` |
| `MemoryStrength` | current_strength + initial_strength + last_access: i64 + access_count + protected: bool. Per-memory tracking in `HashMap<String, MemoryStrength>` inside `ForgettingEngine` | `forgetting.rs:50-74` |
| `DiversityConfig` | enabled (true) + diversity_factor (0.3) + strategy: DiversityStrategy (Semantic) + min_diversity_distance (0.2) | `lib.rs:163-187` |
| `PerformanceConfig` | enable_caching (true) + cache_size (1000) + cache_ttl (300s) + batch_size (32) + max_threads (4) + enable_metrics (true) | `lib.rs:203-236` |
| `RagMetrics` | total_queries + successful_retrievals + failed_retrievals + avg_retrieval_time_ms + avg_memories_per_query + cache_hit_rate + consolidations_performed + memories_forgotten + total_memories_stored + avg_memory_importance | `lib.rs:289-321` |
| `Memory` (embeddings) | id + text + timestamp + importance + valence + category: MemoryCategory + entities + context. **Distinct from `astraweave-memory::MemoryRecord` (parent §13.1) — no shared type** | `astraweave-embeddings/src/lib.rs:122-140` |
| `MemoryCategory` | `#[non_exhaustive]` enum: Social / Combat / Exploration / Quest / Conversation / Skill / Item / Location / Gameplay (default) | `astraweave-embeddings/src/lib.rs:143-…` |
| `StoredVector` | id + vector: Vec<f32> + text + timestamp + importance + metadata: HashMap<String, String> (string-stringified) | `astraweave-embeddings/src/lib.rs:94-108` |
| `SearchResult` | vector: StoredVector + score: f32 + distance: f32 | `astraweave-embeddings/src/lib.rs:111-119` |
| `EmbeddingClient` (trait) | Async embedding generator. Default impl: `MockEmbeddingClient` (deterministic hash-to-vector). Optional features: `ort` (ONNX runtime), `candle` (pure-Rust Candle), `reqwest` (HTTP API) | `astraweave-embeddings/src/client.rs` |
| `EmbeddingConfig` | dimensions (384) + model ("sentence-transformers/all-MiniLM-L6-v2") + batch_size (32) + max_vectors (100,000) + distance_metric (Cosine) | `astraweave-embeddings/src/lib.rs:53-77` |
| `DistanceMetric` | `#[non_exhaustive]` 4-variant: Cosine / Euclidean / Manhattan / DotProduct | `astraweave-embeddings/src/lib.rs:80-91` |
| `VectorStore` | DashMap<String, StoredVector> + parking_lot Mutex for next_index + RwLock for metrics. HNSW indexing per crate-level doc-comment but actual `search` is a linear scan (verified `astraweave-embeddings/src/store.rs:42-60` — no HNSW data structure visible in the simplified impl) | `astraweave-embeddings/src/store.rs:17-29` |
| `TokenCounter` | tiktoken-rs BPE encoder + parking_lot RwLock<HashMap<String, usize>> cache (max 10,000 entries). Falls back to `~4 chars/token` estimation if encoder unavailable | `astraweave-context/src/token_counter.rs:13-23, :66-72` |
| `ConversationHistory` | parking_lot RwLock<VecDeque<Message>> + summary + token_counter + metrics + optional llm_client for summarization | `astraweave-context/src/history.rs:16-34` |
| `ContextConfig` | max_tokens (4096) + sliding_window_size (20) + overflow_strategy: OverflowStrategy (SlidingWindow) + enable_summarization (true) + summarization_threshold (50) + encoding_model ("cl100k_base") + preserve_system_messages (true) + sharing_config | `astraweave-context/src/lib.rs:56-97` |
| `RetrievalEngine` (alternate to RagPipeline) | Standalone in `retrieval.rs:55-66`: takes `&[Memory]` and runs string-similarity search (word-overlap, NOT embeddings). **Different code path from RagPipeline** which uses vector_store + embeddings. Zero `RetrievalEngine::new` callers in workspace | `astraweave-rag/src/retrieval.rs:55-111` |
| `InjectionEngine` (alternate to RagPipeline) | Standalone in `injection.rs:62-72`: takes `&[Memory]` and runs word-overlap scoring + sorting + token-budgeted assembly. **Different code path from RagPipeline::inject_context.** Zero workspace callers | `astraweave-rag/src/injection.rs:62-…` |

#### 13.8.4 Files (subsystem-scoped)

| File | LoC | Role | Production-wired? |
|---|---|---|---|
| `astraweave-rag/src/lib.rs` | 517 | Module declarations + re-exports + `RagConfig` / `InjectionConfig` (canonical) / `OrderingStrategy` / `DiversityConfig` / `DiversityStrategy` / `PerformanceConfig` / `RetrievedMemory` / `RetrievalMetadata` / `RetrievalMethod` / `RagMetrics` / `InjectionResult` (canonical) / `InjectionStrategy` / `MemoryQuery` / `current_timestamp` | **No (via composer)** — `pub use pipeline::*` re-exports the canonical pipeline; consumers reference these types but only via dormant downstream callers |
| `astraweave-rag/src/pipeline.rs` | 1693 | `RagPipeline` struct + `VectorStoreInterface` trait + `VectorStoreWrapper` + `add_memory`/`add_memory_async`/`retrieve`/`retrieve_with_query`/`inject_context`/`inject_context_detailed`/`trigger_consolidation`/`trigger_forgetting`/`should_consolidate` + cache + metrics + diversity + ordering. Includes the LLM-driven `summarize_memories` helper | **No** — `RagPipeline::new` is referenced in tests + benches + 5 dormant consumer crates' fields; zero non-test workspace constructors |
| `astraweave-rag/src/retrieval.rs` | ~250 | Standalone `RetrievalEngine` (alternate, word-overlap-based). `RetrievalConfig` (max_results 10, similarity_threshold 0.7, use_semantic_search true). **Not used by RagPipeline** | **No** — `RetrievalEngine::new` has zero workspace callers |
| `astraweave-rag/src/consolidation.rs` | ~400 | `ConsolidationEngine` + `ConsolidationConfig` + `ConsolidationStrategy` (4 variants) + `ConsolidationResult{processed_count, merged_count, removed_count, processing_time_ms}` | **Indirect** — instantiated by `RagPipeline::new` (`pipeline.rs:199-201`), so dormant by extension |
| `astraweave-rag/src/forgetting.rs` | ~350 | `ForgettingEngine` + `ForgettingConfig` + `MemoryStrength` + `ForgettingResult`. Exponential decay with importance modifier + protected categories | **Indirect** — instantiated by `RagPipeline::new` (`pipeline.rs:202-204`), dormant by extension. **NB: never auto-triggered** — only on explicit `pipeline.trigger_forgetting().await` (zero non-test callers) |
| `astraweave-rag/src/injection.rs` | ~300 | Standalone `InjectionEngine` + alternate `InjectionConfig` + alternate `InjectionResult` + `InjectionContext`. Word-overlap scoring + recency boost. **Not used by RagPipeline** | **No** — zero workspace callers |
| `astraweave-embeddings/src/lib.rs` | ~350 | Re-exports + `EmbeddingConfig` + `DistanceMetric` + `StoredVector` + `SearchResult` + `Memory` + `MemoryCategory` (9 variants) | **Yes** — `Memory` and `MemoryCategory` consumed by `astraweave-rag::pipeline`, `astraweave-rag::injection`, `astraweave-rag::forgetting`; the `EmbeddingConfig` defaults feed all RAG pipelines |
| `astraweave-embeddings/src/client.rs` | ~750 | `EmbeddingClient` trait + `MockEmbeddingClient` (deterministic hash→vec, 384-dim) + optional `OrtEmbeddingClient` (feature `onnx`) / `CandleEmbeddingClient` (feature `candle`) / `HttpEmbeddingClient` (feature `http`) | **Yes** — `EmbeddingClient` trait consumed by all 5 dormant consumer crates as `Arc<dyn EmbeddingClient>`; the `MockEmbeddingClient` is the only constructor present in tests |
| `astraweave-embeddings/src/store.rs` | ~850 | `VectorStore` + `VectorStoreMetrics` + distance functions (cosine/euclidean/manhattan/dot) + `insert`/`search`/`get`/`remove`/`prune_vectors` + `insert_with_metadata` (used by `VectorStoreWrapper`) | **Yes (foundation)** — `VectorStore::new` and `VectorStore::with_config` are the underlying storage layer all `RagPipeline` instances wrap |
| `astraweave-embeddings/src/utils.rs` | ~200 | Helper utilities (`current_timestamp`, distance functions, etc.) | **Yes (foundation)** |
| `astraweave-context/src/lib.rs` | ~250 | Re-exports + `ContextConfig` + `OverflowStrategy` + `SharingConfig` + `Message` + `Role` + `ContextMetrics` | **Yes** — `ContextConfig`/`Message`/`Role`/`TokenCounter` consumed by `ConversationHistory`, which is in turn consumed by Director |
| `astraweave-context/src/history.rs` | ~700 | `ConversationHistory` + `add_message`/`add_message_with_metadata`/`get_context`/`prune_if_needed` + summarization integration | **Indirectly** — `ConversationHistory::new`/`with_llm_client` are declared as fields in 5 dormant consumer crates (same five as RagPipeline); zero non-test workspace constructors |
| `astraweave-context/src/token_counter.rs` | ~500 | `TokenCounter` + `TokenCounterStats` + tiktoken-rs encoder + cache + estimation fallback. Used by both `ConversationHistory` AND `RagPipeline` | **Yes (active)** — `TokenCounter::new("cl100k_base")` is instantiated by `RagPipeline::new` (`pipeline.rs:198`) and by `ConversationHistory::new` (`history.rs:39`). This is the most actively wired primitive in the entire RAG stack |
| `astraweave-context/src/window.rs` | ~600 | `ContextWindow` + sliding-window strategies | **Indirectly** — referenced by `ConversationHistory` |
| `astraweave-context/src/summarizer.rs` | ~550 | LLM-driven summarization (separate from `RagPipeline::summarize_memories`) | Verified 2026-05-12: only consumer outside its own file is the `pub use summarizer::*` re-export at `astraweave-context/src/lib.rs:49`. No external workspace caller imports `summarizer::` types. Dormant. |
| `astraweave-ai/src/rag/mod.rs` | 7 | Re-exports for the inner RagPipeline duplicate | **No (orphaned)** — `pub mod rag` not declared in `astraweave-ai/src/lib.rs` (verified); the file exists but never compiles |
| `astraweave-ai/src/rag/pipeline.rs` | ~360 | Inner duplicate `RagPipeline` (simpler — embedding_client + vector_store + config only). Includes `add_document`/`retrieve`/`consolidate`/`forget`/`inject` variants distinct from the canonical pipeline | **No (orphaned)** — never compiled |
| `astraweave-ai/src/persona/mod.rs` + `manager.rs` | ~unknown | Orphaned persona-management surface | **No (orphaned)** — `pub mod persona` also not declared in `astraweave-ai/src/lib.rs` |

**Subsystem total** (excluding orphaned inner astraweave-ai files): 15 files, ~12.3K LoC across `astraweave-rag` (6 files / ~6.5K), `astraweave-embeddings` (4 files / ~3.2K), `astraweave-context` (5 files / ~2.6K).

**Test/bench surface (subsystem-scoped):**
- `astraweave-rag/tests/{rag_tests,rag_pipeline_tests,pipeline_tests,injection_tests,consolidation_tests,retrieval_tests,mutation_resistant_comprehensive_tests}.rs` — 7 dedicated integration test files
- `astraweave-rag/benches/{rag_benchmarks,rag_adversarial}.rs` — 2 dedicated benchmarks
- `astraweave-embeddings/tests/{mutation_resistant_comprehensive_tests,advanced_embeddings_test}.rs` — 2 dedicated integration test files
- `astraweave-embeddings/benches/embeddings_bench.rs` — 1 benchmark
- `astraweave-context/tests/{conversation_history_tests,context_window_tests,missing_context_tests,mutation_resistant_comprehensive_tests}.rs` — 4 dedicated integration test files
- `astraweave-context/benches/context_benchmarks.rs` — 1 benchmark
- `astraweave-ai/tests/rag_integration_test.rs` — cross-crate integration test (notable: lives in astraweave-ai but exercises astraweave-embeddings primitives, consistent with the orphaned-inner-rag pattern)
- `examples/llm_comprehensive_demo/src/main.rs`, `examples/llm_integration/tests/full_integration_test.rs` — example-level usage

#### 13.8.5 Touchpoints (subsystem-scoped)

**Upstream (what feeds this subsystem)** — *both designed and actual:*

- **Designed:** Any LLM-enhanced AI subsystem that needs memory-augmented prompts. Five consumer crates declare `Arc<RagPipeline>` fields (verified 2026-05-12): `astraweave-director` (LlmDirector for boss adaptation), `astraweave-quests` (LlmQuestGenerator for dynamic quest generation), `astraweave-persona` (LlmPersona for personality-driven responses), `astraweave-dialogue` (LlmEnhancedDialogue, per §13.4 dormant), `astraweave-coordination` (NarrativeCoherenceEngine + WorldEventGenerator, per §13.5 dormant).
- **Actual:** No production code path constructs `RagPipeline::new(...)` outside tests. The five consumer crates hold the field but their constructors (LlmDirector::new, LlmQuestGenerator::new, LlmPersona::new, LlmEnhancedDialogue::new, NarrativeCoherenceEngine::new) themselves have zero non-test production callers (verified per §13.2 / §13.4 / §13.5).
- **TokenCounter feed-in:** `astraweave-context::TokenCounter` is fed text from `RagPipeline::inject_context_detailed` (`pipeline.rs:450, 472`). This is the only foundation primitive with a confirmed active call site inside the pipeline.

**Downstream (what consumes this subsystem's output)** — *designed and actual:*

- **Designed:** Five LLM-enhanced consumer subsystems pass the `enhanced_prompt` from `RagPipeline::inject_context` into `LlmClient::complete`. Designed flow:
  1. LlmDirector boss adaptation: WorldSnapshot → query "boss tactic vs player behavior X" → enhanced_prompt → `LlmClient::complete` → tactic plan (per `astraweave-director/src/llm_director.rs:13-…`).
  2. LlmQuestGenerator: player context → query "quest given biome=forest, player_level=10, recent_actions=combat" → enhanced_prompt → LLM (per `llm_quests.rs:14-…`).
  3. LlmPersona: dialogue context → query "personality response to player_emote=angry" → enhanced_prompt → LLM (per `llm_persona.rs:14-…`).
  4. LlmEnhancedDialogue (parent §13.4 dormant): NPC interaction → query → enhanced_prompt → LLM-generated dialogue.
  5. Coordination (parent §13.5 dormant): narrative event tracking → query "narrative consistency for event X" → enhanced_prompt → LLM.
- **Actual:** Zero production runtime consumption — all 5 downstream paths are themselves dormant.

**Bidirectional / coupled:**

- **`RagPipeline` ↔ `EmbeddingClient`:** `Arc<dyn EmbeddingClient>`. Every memory add and every retrieve query calls `embed(...)` exactly once. The trait abstraction is clean; mock implementations are deterministic.
- **`RagPipeline` ↔ `VectorStoreInterface`:** `Arc<dyn VectorStoreInterface>`. The trait was introduced specifically to enable testing with different backends per the doc-comment at `pipeline.rs:53-54`. `VectorStoreWrapper` is the only production-shape implementation; its `get_all_memories` returns empty Vec (`:166-170`), which silently breaks consolidation.
- **`RagPipeline` ↔ `TokenCounter`:** owned (not Arc'd). `TokenCounter::new("cl100k_base")` is hardcoded at `pipeline.rs:198` — the user cannot inject a custom encoding model via `RagPipeline::new`. See §13.8.7.
- **`RagPipeline` ↔ `LlmClient`:** `Option<Arc<dyn LlmClient>>`. Used only for `summarize_memories` (`pipeline.rs:855-868`). If `None`, summarization falls back to token-truncation (`pipeline.rs:455-458`).
- **`RagPipeline` ↔ `ConsolidationEngine` / `ForgettingEngine`:** `Arc<RwLock<...>>` for both. Consolidation is auto-triggered after every `add_memory` (gated by trigger_threshold or interval). Forgetting is **never** auto-triggered — only on explicit `trigger_forgetting().await`. See §13.8.7.
- **`InjectionConfig` (lib.rs) vs `InjectionConfig` (injection.rs) duplication:** Same struct name with different field sets in the same crate. Both are `pub struct InjectionConfig`. Verified 2026-05-12 at `astraweave-rag/src/lib.rs:55-65`: the module declares `pub mod injection;` (`:57`), `pub mod pipeline;` (`:58`), then `pub use injection::*;` (`:63`) and `pub use pipeline::*;` (`:64`). Rust's glob-import shadowing rule: when a direct definition at the crate root (the `pub struct InjectionConfig` at `lib.rs:111-140`) co-exists with a `pub use` glob that would re-export a same-named item, **the direct definition takes precedence and the glob silently drops the conflicting name**. Therefore `astraweave_rag::InjectionConfig` resolves to the `lib.rs:111-140` definition (used by `RagPipeline`), while the standalone-engine version remains accessible only via the fully-qualified path `astraweave_rag::injection::InjectionConfig`. This is benign at the import-API level but is still internal architectural drift that needs disposal.
- **`InjectionResult` duplication:** Same pattern as above.
- **`InjectionStrategy` (lib.rs) vs `InjectionStrategy` (inner astraweave-ai/src/rag/mod.rs):** Different variant counts (5 vs 2). The inner version is orphaned so the actual collision risk is zero today, but if someone re-enables the inner `pub mod rag` declaration the collision becomes real.
- **`RagPipeline` (canonical) vs `RagPipeline` (inner astraweave-ai):** Same struct name in two crates. As of today the inner version is dormant source code; if added back to `pub mod rag` declaration in `astraweave-ai/src/lib.rs`, the workspace gains TWO RagPipeline types and consumers must namespace them.

#### 13.8.6 Subsystem-specific invariants

1. **`RagPipeline` consolidation triggers automatically; forgetting does NOT.** Per `pipeline.rs:262-264` (consolidation auto-trigger in `add_memory`) vs absence of a similar pattern for forgetting. `trigger_forgetting()` is a pub method but never called from inside the pipeline. Consumer code MUST schedule periodic `trigger_forgetting().await` for memory growth to be bounded over long sessions. Failing to do so means `vector_store.len()` grows without upper bound (until `max_vectors` cap rejects new inserts).
2. **`VectorStoreWrapper::get_all_memories` returns empty Vec.** `pipeline.rs:166-170` explicitly documents "simplified implementation". Consolidation calls `vector_store.get_all_memories()` at `pipeline.rs:534` — the empty return means `ConsolidationEngine::consolidate(vec![])` runs on no memories and `result.merged_count` is always 0. **Consolidation is therefore a no-op in practice through the canonical wrapper.** This is a known limitation; consumers needing real consolidation must implement a custom `VectorStoreInterface`. See §13.8.7.
3. **`RetrievalMethod` always records `SemanticSearch`.** `pipeline.rs:357` hardcodes `RetrievalMethod::SemanticSearch` regardless of which path was taken. The 4 other variants (`KeywordSearch`, `TemporalSearch`, `CategorySearch`, `HybridSearch`) are vocabulary-only — no code path produces them. Consumers reading `retrieved_memory.metadata.method` CANNOT distinguish between sub-types.
4. **`DiversityStrategy` field is ignored.** `pipeline.rs:708-744` `apply_diversity` uses ONLY Jaccard token distance via `text_distance` (`:899-911`), regardless of whether config.diversity.strategy is Semantic / Temporal / Category / Combined. The strategy enum is vocabulary-only.
5. **`InjectionStrategy` is hardcoded to `Insert`.** `pipeline.rs:481` always records `InjectionStrategy::Insert` in the InjectionMetadata, even though the template-replacement logic actually depends on which `{memories}` / `{query}` / `{prompt}` placeholders exist in `injection_template`. The strategy enum is metadata-only.
6. **`TokenCounter` model is hardcoded.** `pipeline.rs:198` hardcodes `TokenCounter::new("cl100k_base")`. Users wanting `o200k_base` (GPT-4o) or a non-OpenAI encoder cannot inject one through `RagPipeline::new`. See §13.8.7.
7. **`Memory::category` defaults to `Gameplay`.** Both `add_memory` (`pipeline.rs:232`) and `add_memory_async` (`:632`) hardcode `MemoryCategory::Gameplay`. Consumers wanting categorical memories must call `add_memory_obj(Memory{...})` (`pipeline.rs:241`) directly with the desired category.
8. **`Memory::importance` defaults to 1.0.** Same two add_memory functions hardcode `importance: 1.0` (`pipeline.rs:230, :629`). Forgetting decay calculation: high importance → very slow decay. Default 1.0 means default memories are essentially un-forgettable until max_memory_age (30 days). See §13.8.7.
9. **`current_timestamp()` uses UNIX epoch seconds.** `lib.rs:438-444` uses `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()`. Memory ordering by `RecencyDesc` / `RecencyAsc` is therefore based on wall-clock seconds, NOT game-tick or deterministic time. Replay systems / determinism systems CANNOT use `RagPipeline` retrieval as a deterministic input.
10. **The `RagPipeline::summarize_memories` triggers only if `enable_summarization && llm_client.is_some()`.** `pipeline.rs:452-454`. If consumers construct the pipeline with `llm_client: None` AND a memory list exceeds `max_context_tokens`, the pipeline falls back to `token_counter.truncate_to_tokens` (lossy truncation). Consumers depending on summarization MUST provide an `LlmClient`.
11. **All RAG cache state uses `parking_lot::RwLock` not `tokio::sync::RwLock`.** `pipeline.rs:15` imports `parking_lot::RwLock`. This is a deliberate choice for performance, but it means cache reads cannot be `.await`'d — the contract is "lock guards never held across await boundaries." Verified at `pipeline.rs:644-647` and `:656-660` where lock-then-drop patterns are explicit.
12. **`VectorStore::insert` returns `Err` at capacity.** `astraweave-embeddings/src/store.rs:83-88` rejects insert when `vectors.len() >= max_vectors` (default 100,000). Production deployments approaching this limit silently fail every new memory until forgetting reclaims slots. Forgetting is not auto-triggered (see invariant 1), so this is a real risk for long-running consumer instances.
13. **`MockEmbeddingClient` is deterministic.** `astraweave-embeddings/src/client.rs` uses hash-to-vector with fixed seed. Tests are reproducible; production deployments using `MockEmbeddingClient` would have semantically meaningless embeddings (hash collisions are random-but-deterministic).
14. **The inner `astraweave-ai/src/rag/` and `astraweave-ai/src/persona/` directories are orphaned source files.** `astraweave-ai/src/lib.rs:29-52` does not declare `pub mod rag` or `pub mod persona`. The files compile only if someone explicitly references them via `mod rag` somewhere — workspace grep shows zero such references. Orphaned source files are a Rule SE2 / CLAUDE.md "Wired beats tested" violation; they should be either restored as `pub mod` or deleted.

#### 13.8.7 Subsystem-specific open questions

- **The runtime AI loop does not consume RAG.** [Decisional / **HIGH-IMPACT finding**, mirrors §13.7 LLM Production Hardening dormancy pattern.] Factual (verified 2026-05-12): all five consumer crates (Director-LLM, Quests-LLM, Persona-LLM, Dialogue-LLM, Coordination-Narrative) that hold `Arc<RagPipeline>` are themselves dormant subsystems (per §13.2, §13.4, §13.5). The basic non-LLM layer of each crate (e.g. canonical `DialogueRunner`, basic `BossDirector` heuristic per §13.2) does NOT touch RAG. The runtime `AIArbiter` / `LlmExecutor` path (parent §2.1-§2.4) bypasses RAG entirely — LLM calls in production are raw `LlmClient::complete(prompt)` with no memory augmentation. Three directional options: commit (wire `RagPipeline` into the LLM-enhanced sub-layers AND production-wire those sub-layers, OR add a thin RAG-injection step inside `LlmExecutor` itself), prune (delete `astraweave-rag` + `astraweave-context` + `astraweave-embeddings` if no consumer is going to materialize), or rebrand (relocate the 3-crate stack to experimental). The 12.3K LoC investment is significant; the dormancy is consequential.
- **`RagPipeline` (canonical) vs `RagPipeline` (inner astraweave-ai) — parallel-implementation drift.** [Decisional / factual, **HIGH-IMPACT finding mirroring the dual GOAP and dual TerrainVertex / FastPreview anti-patterns called out in CLAUDE.md "Architecture Drift"**.] Factual (verified 2026-05-12): `astraweave-ai/src/rag/pipeline.rs:115` defines a second `RagPipeline` struct with different fields (no consolidation/forgetting/diversity, just `config + client + store`). The inner module is NOT declared in `astraweave-ai/src/lib.rs:29-52`, so it never compiles. Comparing the two implementations (verified by reading both): the inner duplicate adds `add_document(doc: RagDocument)` returning a typed `RagDocument` (different shape from canonical's `Memory` type), `retrieve(query) -> RagContext` (returning a typed `RagContext` with `Display` impl producing "Relevant Context:\n{i}. {content}" formatted strings), and inline `consolidate` / `forget` methods that operate directly on `VectorStore::get_all_ids()` rather than via `VectorStoreWrapper::get_all_memories()`. **The inner's `consolidate` method actually works** because it queries `VectorStore` directly (unlike the canonical pipeline's no-op consolidation per §13.8.7). Distinct features worth preserving: the typed `RagContext` display format, and the working `consolidate` implementation that bypasses the empty-Vec wrapper bug. Three options: delete the inner duplicate (CLAUDE.md Rule "Never build a second implementation of a logical system that already exists"); restore the inner duplicate as the canonical version (if the simpler typed API is preferred); migrate the inner's working `consolidate` + typed `RagContext` back into the canonical pipeline.
- **`InjectionConfig` / `InjectionResult` duplication within `astraweave-rag` itself.** [Decisional / factual.] Factual: `lib.rs:111-140` defines `InjectionConfig` with one field set; `injection.rs:11-33` defines `InjectionConfig` with a different field set. Same for `InjectionResult` (`lib.rs:324-337` vs `injection.rs:49-59`). The `lib.rs` versions are used by `RagPipeline`; the `injection.rs` versions are used by the standalone `InjectionEngine` (which has zero workspace callers). The `pub use injection::*` at `lib.rs:63` causes name shadowing. Either rename the `injection.rs` types (`StandaloneInjectionConfig` / `StandaloneInjectionResult`) or delete the standalone `InjectionEngine` entirely (consistent with its zero-consumer dormancy).
- **`VectorStoreWrapper::get_all_memories` returns empty Vec — consolidation is a no-op.** [Factual / **bug-class finding** — resolved 2026-05-12.] `pipeline.rs:166-170`: simplified implementation; the comment says "In practice, you'd iterate through all stored vectors." `trigger_consolidation` calls `vector_store.get_all_memories()` at `:534` and the empty return makes `ConsolidationEngine::consolidate(vec![])` a no-op. Either: implement `get_all_memories` correctly via `VectorStore::get_all_ids()` + iteration; or document that the canonical wrapper does not support consolidation and require custom `VectorStoreInterface` implementors for that capability; or rewire consolidation to use `VectorStore::get_all_ids()` directly via the wrapper (i.e. bypass the trait method). **Verified 2026-05-12: zero test coverage for the bug.** `astraweave-rag/tests/pipeline_tests.rs:137-173` (`test_consolidation_trigger`) only asserts that `metrics.consolidations_performed` counter increments (lines `:152, :159, :168`); does NOT verify that memories actually get merged. `ConsolidationEngine::consolidate` logic itself is tested in `tests/consolidation_tests.rs` with hand-built `Vec<Memory>`, but no test exercises the full `RagPipeline → vector_store.get_all_memories() → ConsolidationEngine::consolidate` path. The bug-class status is therefore **dormant-test-coverage** (no test catches it).
- **The 4 unused `RetrievalMethod` variants and the 4 unused `DiversityStrategy` variants.** [Decisional / factual.] Verified 2026-05-12: `RetrievalMethod::SemanticSearch` is the only variant ever returned from `RagPipeline` retrieve path (`pipeline.rs:357`); `KeywordSearch`/`TemporalSearch`/`CategorySearch`/`HybridSearch` are vocabulary-only. `DiversityStrategy::Semantic`/`Temporal`/`Category`/`Combined` are all ignored by `apply_diversity` which uses only Jaccard token distance (`:725-728, :899-911`). Either implement the alternate variants (giving the enum semantic richness consumers can rely on), prune to a single variant (matching actual behavior), or document that the variants are "future-reserved" markers in line with the §13.5 Coordination `#[allow(dead_code)]` "reserved for future..." pattern. The current state is misleading — consumers might assume different RetrievalMethod values would produce different results, but they cannot.
- **`InjectionStrategy` is metadata-only.** [Factual.] `pipeline.rs:481` hardcodes `InjectionStrategy::Insert` regardless of actual template behavior. The 5-variant enum (Prepend/Append/Insert/Interleave/Replace) is decorative. Either implement the alternate strategies (which would require different template logic per strategy), prune the unused variants, or rename the enum to `InjectionShape` to make clear it's a template-pattern descriptor rather than an active control.
- **`TokenCounter::new("cl100k_base")` hardcoded.** [Factual / decisional.] `pipeline.rs:198` chooses the GPT-3.5/GPT-4 encoder unconditionally. For Qwen3/Llama3-family tokenizers, the cl100k_base count is approximate (typically over-estimates by 5-15%). Per `astraweave-context/src/token_counter.rs:41-43` `get_bpe_from_model` falls back to cl100k_base for any unrecognized model name, so the practical impact is small, but the API contract is misleading. Options: thread `encoding_model: String` through `RagConfig` (low blast radius — just add a field with cl100k_base default); add `RagPipeline::with_token_counter(custom_counter)`; or document the cl100k_base universality choice in the type's doc-comment.
- **`add_memory` / `add_memory_async` default Memory fields are coarse.** [Decisional.] Both hardcode `importance: 1.0`, `valence: 0.0`, `category: MemoryCategory::Gameplay`, `entities: []`, `context: {}` (`pipeline.rs:226-234, :624-632`). Consumers wanting categorized + entity-tagged memories must use `add_memory_obj(Memory{...})` directly. The asymmetric API (text-shortcut vs full-object) is workable but the lossy defaults invite consumers to use the shortcut and never realize they've lost categorical structure. Recommendation: deprecate the shortcuts in favor of explicit Memory construction.
- **`current_timestamp()` non-determinism.** [Factual / decisional, mirrors `forgetting.rs:69` `chrono::Utc::now().timestamp()` usage.] All RAG timestamps are wall-clock UNIX seconds. Game replays / deterministic ECS-tick systems cannot use RAG as a deterministic memory input. Options: thread a "time source" trait through `RagConfig` (e.g. `Box<dyn TimeSource>` defaulting to wall-clock); document that RAG is explicitly non-deterministic and consumers wanting determinism must implement their own; or accept the non-determinism as a deliberate choice for live-LLM AI (which is itself non-deterministic).
- **`VectorStore` is a linear-scan, NOT actually HNSW.** [Factual / cognitive trap.] `astraweave-embeddings/src/lib.rs:9` advertises "Fast similarity search using HNSW indexing." `astraweave-embeddings/src/store.rs:16` says "with HNSW indexing (simplified implementation)." Actual code at `store.rs:42-60` shows a DashMap<String, StoredVector> with a `next_index` Mutex (unused), with no HNSW data structure visible. Search is a linear scan iterating the DashMap (verified by reading `search` impl in the same file). The `hnsw_rs = { version = "0.3", optional = true }` dependency in `Cargo.toml:31` and the `hnsw = ["hnsw_rs"]` feature flag (`:42`) suggest HNSW was planned but never integrated. At 100,000-vector capacity with linear scan + 384-dim cosine distance, search latency is ~10-50ms — likely acceptable for game AI but the documentation suggests sub-millisecond ANN search. Options: actually implement HNSW; remove the HNSW claims from the doc-comments; or document the "simplified scan" trade-off explicitly.
- **`RagPipeline` vs `astraweave-memory::MemoryManager` — two memory subsystems with no shared types.** [Decisional, mirrors parent §13.1 Memory dormancy pattern.] Factual (verified 2026-05-12 + cross-referenced with §13.1): `astraweave-rag` operates on `astraweave-embeddings::Memory{id, text, timestamp, importance, valence, category, entities, context}`. `astraweave-memory` operates on its own types (`MemoryRecord`, `Persona`, `GameEpisode`, `Episode`, hierarchical sensory/working/episodic/semantic). The two subsystems do NOT share a memory representation. Per §13.1 both are dormant for the runtime AI loop. The directional question is the same: production-wire one (and prune or rebrand the other), or surface the two-subsystem-coexistence as deliberate (e.g. memory crate for hierarchical typing + RAG crate for semantic retrieval). The two-crate stack would either need an adapter layer or one would need to subsume the other.
- **The `protected_categories: vec![MemoryCategory::Quest]` default in ForgettingConfig.** [Factual.] `forgetting.rs:44`. Quest memories never decay through `ForgettingEngine`. This is a deliberate choice (quest progress should not be forgotten), but it interacts with `ConsolidationStrategy::Importance` which CAN remove low-importance Quest memories during consolidation. If a Quest memory has importance 0.1 and is dropped by importance-based consolidation, the protected-category flag in forgetting is moot. Either align consolidation to respect protected_categories, or document the asymmetry, or deprecate one of the two protection mechanisms.
- **Orphaned source files at `astraweave-ai/src/rag/` and `astraweave-ai/src/persona/`.** [Factual / decisional.] `astraweave-ai/src/lib.rs:29-52` declares 8 `pub mod` items (4 feature-gated). The `rag/` and `persona/` directories exist on disk but are not declared. They never compile. Per CLAUDE.md "Wired beats tested" and Rule SE2 "Subsystem scope discipline," these are dormant code waiting to be either restored or deleted. The pattern matches §13.5 Coordination's commented-out `social_graph` / `components` / `systems` module declarations (which had the inverse: declarations without source files). Both patterns indicate stalled development.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **AI agents are first-class citizens.** Per CLAUDE.md Mandate. The four-stage loop (Perception → Reasoning → Planning → Action) is foundational. Every AI output is a `PlanIntent`. Every `PlanIntent` flows through `validate_and_execute`.
2. **`AIArbiter` is the production hybrid.** GOAP for instant, LLM for strategic, BT for emergency. `update()` always returns instantly. Never block on LLM in the game loop.
3. **`AIArbiter` is `!Send + !Sync`.** Single-threaded per agent.
4. **LLM cooldown defaults to 15.0 s.** Configurable via `with_llm_cooldown(secs)`.
5. **Three validation layers, only one mutates `World`.** `tool_sandbox` = taxonomy; `tool_guard` = LLM-output policy; `validate_and_execute` = engine-side gate. Plans only become game state inside `validate_and_execute`.
6. **`WorldSnapshot` field names are load-bearing in LLM prompts.** See `docs/architecture/ecs_math_core_sdk_foundation.md` Invariant 8 before renaming.
7. **Two GOAP implementations coexist.** Canonical in `astraweave-behavior::goap`, advanced in `astraweave-ai::goap` (feature `planner_advanced`). No shared types.
8. **4-tier fallback chain is mandatory for LLM-backed orchestrators.** `FallbackOrchestrator` composes the chain; do not call `LlmClient::complete` directly in production paths.
9. **5-strategy plan parser handles real-world LLM output variance.** Don't roll your own JSON parsing — use `astraweave-llm::plan_parser::parse_llm_response`.
10. **Global `PromptCache` defaults to exact-match-only.** Set `LLM_CACHE_SIM_THRESH<1.0` only with careful understanding of test pollution implications.
11. **`#[non_exhaustive]` is pervasive in AI enums.** Adding variants is forward-compatible for external matchers; check all dispatch sites per CLAUDE.md Integration Completeness #2.
12. **All AI crates are `#![forbid(unsafe_code)]` except `astraweave-ai` (which has no `unsafe` blocks anyway).** Don't introduce `unsafe` here.
13. **The system has 12,700+ agent capacity at 60 FPS.** Don't add per-frame allocations to the hot path; bench harnesses exist at `astraweave-ai/benches/` for any perf-sensitive change.

**Files you'll most likely touch:**
- `astraweave-ai/src/{core_loop,orchestrator,tool_sandbox,ai_arbiter,llm_executor}.rs`
- `astraweave-behavior/src/{lib,goap}.rs`
- `astraweave-llm/src/{plan_parser,fallback_system,tool_guard,prompts}.rs`

**Files you should NOT touch without strong reason:**
- `astraweave-ai/src/ai_arbiter.rs` mode-transition logic — the three transitions (`transition_to_llm`, `transition_to_goap`, `transition_to_bt`) are tightly coupled with metrics; verify with `astraweave-ai/tests/arbiter_*` tests
- `astraweave-ai/src/llm_executor.rs::generate_plan_async` — backs the LLM async pattern; changing semantics affects every Arbiter call site
- `astraweave-behavior/src/goap.rs::WorldState` — `BTreeMap<u32, bool>` choice and the interner are deliberate for determinism; do not migrate to `HashMap`
- `astraweave-llm/src/fallback_system.rs` tier chain — the `FallbackTier::next()` order is the production-hardening promise
- `astraweave-llm/src/plan_parser.rs::parse_llm_response` extraction chain — the 5-strategy order handles real-world LLM output variance; do not collapse strategies
- `astraweave-core/src/validation.rs::validate_and_execute` — the engine-side tool sandbox; the only function that should mutate `World` for AI actions
- `astraweave-llm/src/llm_adapter.rs` — labeled "stub" in its header; do not build on it as if it were the canonical LLM gate

**Common mistakes when changing this system:**
- **Mistake**: Adding a new AI subsystem that mutates `World` directly.
  **Why wrong**: Bypasses the tool sandbox. AI must propose `PlanIntent`s and let `validate_and_execute` apply them.
- **Mistake**: Calling `LlmExecutor::generate_plan_sync` in the game loop.
  **Why wrong**: Blocks for 3-8 s. Use `generate_plan_async` + `AsyncTask::try_recv` polling instead.
- **Mistake**: Adding LLM call sites that don't go through `FallbackOrchestrator`.
  **Why wrong**: Loses graceful degradation. The 4-tier chain is the production reliability promise.
- **Mistake**: Renaming a `WorldSnapshot` field as a Rust-side refactor.
  **Why wrong**: Trips the LLM-prompt-binding invariant. See `docs/architecture/ecs_math_core_sdk_foundation.md` Invariant 8.
- **Mistake**: Sharing an `AIArbiter` instance across threads.
  **Why wrong**: `!Send + !Sync`. Use per-thread / per-agent instances.
- **Mistake**: Adding a parallel GOAP impl ("astraweave-goap-v2").
  **Why wrong**: Two already exist (canonical + advanced). Per CLAUDE.md Scope Discipline: "Never build a second implementation of a logical system that already exists."
- **Mistake**: Building a new "rule" orchestrator without reading the existing two.
  **Why wrong**: `RuleOrchestrator` (`astraweave-ai/src/orchestrator.rs`) and `HeuristicConfig` (`astraweave-llm/src/heuristics.rs`) both serve rule-like roles at different layers. Extend or compose; do not duplicate.
- **Mistake**: Setting `LLM_CACHE_SIM_THRESH=0.9` to "improve cache hit rate".
  **Why wrong**: The `lib.rs:57-65` comment documents this introduces nondeterminism across prompt variants and cross-test pollution. Default 1.0 is intentional.

---

## Appendix B: Historical context

The AI system has grown in several phases:

**Phase 1-6 (pre-recovered history):** The canonical loop (`core_loop.rs`, `orchestrator.rs`, `tool_sandbox.rs`) and the basic GOAP (`astraweave-behavior::goap`) were established. `RuleOrchestrator` served as the initial pragmatic planner. Behavior trees were added with their own ECS plugin.

**Phase 7 — LLM hardening + Arbiter (per `astraweave-llm/src/fallback_system.rs:1-7` "Phase 7: Multi-Tier Fallback System" and `astraweave-ai/src/lib.rs:37` "Phase 7 Arbiter"):** The hybrid `AIArbiter` was introduced with three control modes (`GOAP`, `ExecutingLLM`, `BehaviorTree`). The 4-tier fallback chain (`FullLlm` → `SimplifiedLlm` → `Heuristic` → `Emergency`) was specified. The 5-strategy plan parser was added. Production hardening composed rate limiter + circuit breaker + backpressure + A/B testing + telemetry + health checker into `ProductionHardeningLayer`. Tool guard with allowlist/denylist policies was added at the LLM-output boundary.

**Later — Advanced GOAP (feature `planner_advanced`):** A second GOAP implementation landed in `astraweave-ai/src/goap/` (22 files) adding learning, persistence, shadow mode, plan analysis/stitching/visualization, goal authoring, scheduling, validation. It coexists with the canonical engine GOAP and shares no Rust types.

**Later — Dual-executor Arbiter:** The `AIArbiter::new(strategic_executor, fast_executor, goap, bt)` constructor was added (separate from the backward-compatible `with_single_executor`) to support both thinking-mode (deep planning, 3-8 s background) and non-thinking-mode (low-latency inline, <2 s) LLM executors. The doc-comment architecture diagram at `ai_arbiter.rs:1-46` reflects this.

**Later — Model migration:** Initial LLM client was Phi3. Commit `2468b25f1` per workspace git log moved to Hermes2Pro ("Replace Phi3 with Hermes2Pro and add UI fixes, latency optimizations, and advanced features"). Current arbiter doc-comment references Qwen3 (`ai_arbiter.rs:1` "GOAP+Qwen3 Hybrid Control System"). CLAUDE.md aligns with Qwen3. ARCHITECTURE_REFERENCE.md still refers to Hermes. The Arbiter implementation is model-agnostic; the choice is at construction time.

**Continuous — Mutation testing + Wave 2 remediation:** Each AI crate has `mutation_resistant_comprehensive_tests.rs` integration tests (see §10). The Wave 2 mutation-testing campaign covered ai, behavior, llm with targeted kill-rate improvements.

**Continuous — Memory and learning subsystems:** `astraweave-memory` (17 files, 11.5K LoC) grew alongside the AI core with hierarchical memory (sensory/working/episodic/semantic/procedural/emotional/social), episode recording with SQLite persistence, pattern detection, preference profiling, adaptive weighting, learned-behavior validation. The feedback loop into the AI core (memory → adaptive weights → BT tick) exists structurally inside the memory crate. **2026-05-12 verification (§13.1):** the runtime hookup is NOT wired — `astraweave-ai` and `astraweave-behavior` have zero `astraweave_memory` imports. The legacy `persona::*` types are actively consumed by `astraweave-persona`, but the main memory pipeline (`Memory`, `MemoryStorage`, `EpisodeRecorder`, `AdaptiveWeightManager`, etc.) currently has no production callers. The subsystem holds 1000+ tests but no end-to-end runtime wiring.

**Continuous — Domain-specific AI:** `astraweave-director` (boss AI), `astraweave-npc` (NPC runtime), `astraweave-dialogue` (branching dialogue + LLM dialogue), `astraweave-coordination` (multi-agent) — each a separate crate with its own AI patterns. Each warrants its own focused trace document.

The system is foundational and continues to evolve. The Arbiter, fallback chain, and validation boundary are the load-bearing invariants — they encode the architectural commitment that AI is a first-class citizen of the engine.
