---
schema_version: 1
trace_id: quests
title: "Quest System"
description: "Quests — components, systems, LLM + terrain quests"
primary_crate: astraweave-quests
domain: gameplay
lifecycle_status: in_design
integration_status: partial
summary: "components/systems + LLM + terrain quests; overlaps the weaving quest layer. quests.md §6"
owns: [astraweave-quests]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Quest System

## Metadata

| Field | Value |
|---|---|
| **System name** | Quest System |
| **Primary crates** | `astraweave-quests` (LLM + terrain + ECS quest layer), `astraweave-weaving` (Veilweaver gameplay quest layer) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Mixed: `astraweave-weaving::quest` is Active (wired via `VeilweaverLevel`); `astraweave-quests` LLM/terrain/ECS layer is In-Design-but-tested (no production callers); the basic `astraweave-quests::{Quest, QuestStep}` struct pair is Active (used by `aw_editor`). |
| **Owner notes** | Two physically separate quest implementations coexist (see §6). The richer, AI-facing one (`astraweave-quests`) is largely dormant; the actually-shipped gameplay one lives in `astraweave-weaving`. |

---

## 1. Executive Summary

**What this system does:**
There are two distinct quest implementations in the workspace. (1) `astraweave-quests` provides an authorable basic quest type (`Quest`/`QuestStep`), plus an LLM-driven generative quest layer (`LlmQuest`, `LlmQuestGenerator`, `QuestLlmSystem`, ECS quest components) and a terrain-feature-driven quest generator (`TerrainQuestGenerator`). (2) `astraweave-weaving::quest` provides a self-contained gameplay quest system (`Quest`, `ObjectiveType`, `QuestManager`) with objective progress tracking, rewards, and prerequisites, used by the Veilweaver demo level.

**Why it exists:**
`astraweave-quests` was built to let the AI/LLM stack and the AI terrain orchestrator emit dynamic, personalized quests; `astraweave-weaving::quest` provides concrete, hand-authored progression quests for the Veilweaver gameplay vertical slice.

**Where it primarily lives:**
- `astraweave-quests/src/lib.rs` — basic `Quest`/`QuestStep` + module wiring
- `astraweave-quests/src/llm_quests.rs` — LLM quest generator and rich quest data model
- `astraweave-quests/src/components.rs` — ECS quest components (`CQuestGenerator`, `CActiveQuest`, `CQuestMetrics`, `CQuestJournal`)
- `astraweave-quests/src/systems.rs` — `QuestLlmSystem` async orchestration + `integration` helpers
- `astraweave-quests/src/terrain_quests.rs` — `TerrainQuestGenerator` (terrain-feature → quest)
- `astraweave-weaving/src/quest.rs` — `Quest`/`QuestManager`/`ObjectiveType` gameplay quest system
- `astraweave-weaving/src/quest_types.rs` — extended objective payload types (escort/defend/boss/etc.)
- `astraweave-weaving/src/starter_quests.rs` — three hand-authored starter quests

**Status note:**
Treat the two layers as separate systems that share a name and some vocabulary but no code. The `astraweave-quests` LLM, terrain, and ECS-component machinery has **no non-test production constructor anywhere in the workspace** (verified §4, §6) — it is in-design-but-tested per CLAUDE.md Key Lesson 8. The `astraweave-weaving::quest` layer **is** wired: `VeilweaverLevel::new` constructs a `QuestManager` from `all_starter_quests()` (`astraweave-weaving/src/level.rs:258-266`) and the `veilweaver_quest_demo` example drives it end-to-end. The only part of `astraweave-quests` consumed by another production crate is the basic `Quest`/`QuestStep` pair, imported by `tools/aw_editor` for quest-graph authoring.

---

## 2. Authoritative Pipeline

Because two independent quest systems exist, two pipelines are documented.

### 2A. `astraweave-weaving::quest` — wired gameplay pipeline

```text
[Level construction]
    │ VeilweaverLevel::new()
    ▼
[Stage W1: Quest registration]
    file: astraweave-weaving/src/level.rs:258-266
    role: build QuestManager, register all_starter_quests(), activate first quest
    key data: QuestManager { quests: HashMap<String, Quest>, active_quest_id }
    │
    │ all_starter_quests()
    ▼
[Stage W2: Starter quest definitions]
    file: astraweave-weaving/src/starter_quests.rs:71-77
    role: hand-authored Quest builders (repair / kill / fetch+explore)
    key data: 3 Quests with ObjectiveType objectives + QuestReward lists + prerequisites
    │
    │ gameplay events → QuestManager::update_kill/repair/fetch/explore
    ▼
[Stage W3: Objective progress tracking]
    file: astraweave-weaving/src/quest.rs:491-525 (manager), 326-400 (per-quest)
    role: mutate ObjectiveType counters/flags for the active quest
    key data: updated objective state
    │
    │ QuestManager::check_active_quest()
    ▼
[Stage W4: Completion + reward emission]
    file: astraweave-weaving/src/quest.rs:477-489
    role: when all objectives complete, mark Completed, pop active, return Vec<QuestReward>
    key data: Vec<QuestReward> → consumed by VeilweaverLevel::apply_reward (level.rs:388)
    ▼
[Rewards applied to Player / abilities / stats]
```

### 2B. `astraweave-quests` — LLM + terrain generative pipeline (no production driver)

```text
[QuestContext: player_id, level, location, npcs, world_state, recent_activities]
    │ QuestLlmSystem::update(...) / ::generate_quest(...)   [NO PRODUCTION CALLER]
    ▼
[Stage Q1: Generation gating]
    file: astraweave-quests/src/systems.rs:311-333 (should_generate_quest)
    role: cooldown + active-quest-limit + recent-activity checks
    │
    │ LlmQuestGenerator::generate_quest(&context)
    ▼
[Stage Q2: RAG retrieval + prompt render + LLM completion]
    file: astraweave-quests/src/llm_quests.rs:394-471
    role: retrieve player history from RagPipeline, render "quest_generation" prompt,
          call LlmClient::complete, parse JSON into LlmQuest, validate, store summary in RAG
    key data: LlmQuest { id, title, steps[LlmQuestStep], metadata, branching, rewards, ... }
    │
    │ LlmQuestGenerator::validate_quest(...) (second LLM round-trip)
    ▼
[Stage Q3: ECS bookkeeping]
    file: astraweave-quests/src/systems.rs:58-127, components.rs
    role: record metrics (CQuestMetrics), add to journal (CQuestJournal),
          mark generation time on CQuestGenerator; CActiveQuest tracks step progress
    │
    │ player choices → QuestLlmSystem::handle_player_choice(...)
    ▼
[Stage Q4: Branching + dynamic content (further LLM round-trips)]
    file: astraweave-quests/src/llm_quests.rs:474-587, systems.rs:204-258
    role: branch_narrative + generate_dynamic_content via LlmClient
    ▼
[Completed/abandoned quests cleaned up; metrics/journal updated]
```

The terrain variant is a parallel, **synchronous, non-LLM** generator:

```text
[TerrainQuestContext: feature_type, position, radius, intensity, biome, is_ai_generated]
    │ TerrainQuestGenerator::generate_quest(terrain_ctx, player_ctx)   [NO PRODUCTION CALLER]
    ▼
[Stage T1: Gating (enabled flag, max-active limit, spatial spacing)]
    file: astraweave-quests/src/terrain_quests.rs:361-409
    │
    │ create_terrain_quest(...)
    ▼
[Stage T2: Template-string quest synthesis]
    file: astraweave-quests/src/terrain_quests.rs:412-564
    role: pick archetype from TerrainFeatureType::quest_archetypes, compute difficulty
          from feature difficulty_modifier + player level, build a 2-step LlmQuest +
          TerrainObjective list bound to world coordinates
    key data: TerrainQuest { quest: LlmQuest, terrain_context, terrain_objectives }
    ▼
[TerrainQuest pushed onto active_quests vec; quests_generated counter incremented]
```

### Stage-by-stage detail (selected)

#### Stage W4: Completion + reward emission
**File:** `astraweave-weaving/src/quest.rs:477-489`
**Role:** `QuestManager::check_active_quest` calls `Quest::check_completion` (`quest.rs:292-304`), and on success records the quest in `completed_quests`, clears `active_quest_id`, and returns the cloned `Vec<QuestReward>`.
**Notes:** `QuestManager` enforces a single active quest at a time (`activate_quest` errors if `active_quest_id.is_some()`, `quest.rs:450-452`). Completion only transitions an `Active` quest (`check_completion` returns false unless state is `Active`).

#### Stage Q2: Generation + validation
**File:** `astraweave-quests/src/llm_quests.rs:394-471`
**Role:** Full generative path: RAG retrieve → prompt render → `LlmClient::complete` → `serde_json::from_str::<LlmQuest>` → `validate_quest` (a second LLM round-trip) → best-effort RAG store.
**Notes:** Quest JSON is parsed directly from raw LLM output (`:429-430`); a parse failure returns an error. Validation is itself LLM-driven (`validate_quest`, `:512-546`) and an invalid result aborts generation (`:445-448`). RAG storage is best-effort via `Arc::get_mut` and silently skipped when the pipeline is shared (`:459-464`).

#### Stage T2: Template-string quest synthesis
**File:** `astraweave-quests/src/terrain_quests.rs:412-564`
**Role:** Despite producing an `LlmQuest` value, `create_terrain_quest` performs **no** LLM call — it synthesizes title/description/steps via `format!` from the terrain feature type and player level.
**Notes:** Difficulty = `0.3 + player_level*0.02 + feature.difficulty_modifier() + intensity*0.2`, clamped `[0,1]` (`:433-436`). AI-generated features get a reward multiplier (`config.ai_terrain_reward_bonus`, default 1.25, `:567-585`). The matched trigger is looked up (`:388-391`) but the result is discarded (`let _trigger`) — gating is done by the limit/spacing checks, not by the trigger.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| `Quest` (basic) | Title + `Vec<QuestStep>`; each step is a description + `completed` bool. Minimal authorable struct. | `astraweave-quests/src/lib.rs:5-30` |
| `QuestStep` | A single basic step (`description`, `completed`). | `astraweave-quests/src/lib.rs:5-9` |
| `LlmQuest` | Rich generated quest: id, title, description, `Vec<LlmQuestStep>`, `QuestMetadata`, `QuestBranching`, `QuestRewards`, personalization. | `astraweave-quests/src/llm_quests.rs:19-30` |
| `LlmQuestStep` | Rich step: id, objectives, branching choices, optional dynamic content, validation criteria. | `astraweave-quests/src/llm_quests.rs:33-42` |
| `LlmQuestGenerator` | LLM-backed generator (prompt library + RAG + LLM client). | `astraweave-quests/src/llm_quests.rs:194-203` |
| `QuestLlmSystem` | Async orchestration wrapper around `LlmQuestGenerator` operating on ECS quest components. | `astraweave-quests/src/systems.rs:9-15` |
| `CQuestGenerator` / `CActiveQuest` / `CQuestMetrics` / `CQuestJournal` | ECS components for quest generation context, active quest tracking, analytics, and journal. | `astraweave-quests/src/components.rs` |
| `TerrainQuestGenerator` | Synchronous generator that emits terrain-bound `TerrainQuest`s from `TerrainQuestContext`. | `astraweave-quests/src/terrain_quests.rs:278-636` |
| `TerrainFeatureType` | Enum of terrain features (Mountain, Cave, Forest, …) carrying quest archetypes + difficulty modifiers. | `astraweave-quests/src/terrain_quests.rs:24-107` |
| `Quest` (weaving) | Gameplay quest: id, title, description, `QuestState`, `Vec<ObjectiveType>`, `Vec<QuestReward>`, prerequisites. | `astraweave-weaving/src/quest.rs:227-243` |
| `ObjectiveType` (weaving) | Enum of trackable objectives (Kill/Repair/Fetch/Explore + extended escort/defend/boss/etc.). | `astraweave-weaving/src/quest.rs:22-70` |
| `QuestManager` | Owns weaving quests, enforces single-active + prerequisites, drives progress updates. | `astraweave-weaving/src/quest.rs:405-547` |
| `QuestState` | Lifecycle enum. **Two different definitions** — see §6. | `astraweave-quests/src/components.rs:133-141` AND `astraweave-weaving/src/quest.rs:8-19` |

### Terms to NOT confuse

- **`Quest` (astraweave-quests) vs `Quest` (astraweave-weaving):** Same name, unrelated structs. The first is `{ title, steps: Vec<QuestStep> }`. The second is `{ id, title, description, state, objectives, rewards, prerequisites }` with builder methods and progress tracking. No conversion path exists between them.
- **`ObjectiveType` (astraweave-quests) vs `ObjectiveType` (astraweave-weaving):** Both enums, different variants and semantics. `astraweave-quests` (`llm_quests.rs:114-126`) is a flat tag enum (Collect/Defeat/Interact/…/Custom). `astraweave-weaving` (`quest.rs:22-70`) carries inline progress payloads (counts, positions, sub-objectives).
- **`QuestState` (two definitions):** `astraweave-quests` variants: Active/Paused/Completed/Failed/Abandoned. `astraweave-weaving` variants: Inactive/Active/Completed/Failed. Different lifecycles.
- **`LlmQuest` produced by `TerrainQuestGenerator`:** The terrain generator outputs an `LlmQuest` value but never calls an LLM — the type name does not imply an LLM was involved on this path.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds these systems)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-llm` | `LlmClient::complete(&prompt)` | quest/branch/validation JSON | Used by `LlmQuestGenerator` (`llm_quests.rs:422-426, 499-503, 536-540, 569-573`). See [ai_pipeline.md](./ai_pipeline.md). |
| `astraweave-rag` | `RagPipeline::retrieve`, `RagPipeline::add_memory` | player quest history, stored quest summaries | `llm_quests.rs:398-405, 459-464`. See [ai_pipeline.md](./ai_pipeline.md) §13.8. |
| `astraweave-prompts` | `PromptLibrary` + `PromptTemplate::render_map` | three templates (`quest_generation`, `quest_branching`, `quest_validation`) registered in `LlmQuestGenerator::new` | `llm_quests.rs:233-381`. |
| `astraweave-context` | `ConversationHistory::new(ContextConfig)` | conversation context (held but `#[allow(dead_code)]`) | `llm_quests.rs:197-198, 227-231`. |
| Terrain feature data | `TerrainQuestContext { feature_type, position, biome, … }` (plain struct, caller-constructed) | terrain feature descriptors | No code in `astraweave-terrain` constructs `TerrainQuestContext` (grep: only tests). The intended producer (AI terrain orchestrator) is not wired. See [terrain.md](./terrain.md). |
| `astraweave-weaving` gameplay events | `QuestManager::update_kill/repair/fetch/explore` | progress increments, player position | Driven by `VeilweaverLevel` and the `veilweaver_quest_demo` example. |

### Downstream (what consumes these systems' output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `tools/aw_editor` | imports `astraweave_quests::Quest` / `astraweave_quests::QuestStep` | basic quest-graph authoring | `tools/aw_editor/src/main.rs:96, 487, 8141`. Only the **basic** struct pair is used — none of the LLM/terrain/ECS layer. See [aw_editor.md](./aw_editor.md). |
| `astraweave-weaving::level` | `QuestManager` field + `check_active_quest` → `Vec<QuestReward>` | reward application | `VeilweaverLevel` owns a `QuestManager` (`level.rs:214, 258`); rewards applied via `apply_reward` (`level.rs:388-389`). |
| `astraweave-weaving::ui::quest_panel` | reads `QuestManager` state | UI display | `astraweave-weaving/src/ui/quest_panel.rs:2`. |
| `examples/veilweaver_quest_demo` | `VeilweaverLevel::new()` + `level.quest_manager.update_*` | full quest playthrough | `examples/veilweaver_quest_demo/src/main.rs` exercises all three starter quests. |

### Bidirectional / Coupled

- **`astraweave-quests` ↔ AI pipeline:** Designed to both consume (RAG retrieve / LLM complete) and feed (RAG `add_memory` of quest summaries) the memory stack. Per [ai_pipeline.md](./ai_pipeline.md) §13.8.2, this round-trip never executes in the runtime because no production consumer constructs `LlmQuestGenerator`.

**Production-caller check (CLAUDE.md Integration Completeness):**
- `LlmQuestGenerator::new` — referenced only by `systems.rs:525` (test), `llm_quests.rs:739/762` (tests). Zero non-test callers. Corroborated independently by [ai_pipeline.md](./ai_pipeline.md):2379.
- `QuestLlmSystem::new` — only `systems.rs:529` (test). Zero non-test callers.
- `TerrainQuestGenerator::new` / `::default_config` — only `terrain_quests.rs` tests. Zero non-test callers.
- `CQuestGenerator::new` / `integration::initialize_player_quest_system` — only `components.rs`/`systems.rs` tests. Zero non-test callers.
- `astraweave_quests::Quest` / `QuestStep` — used by `aw_editor` (production). **Wired.**
- `astraweave-weaving` `QuestManager::new` / `all_starter_quests` — used by `VeilweaverLevel::new` (`level.rs:258-259`) and the `veilweaver_quest_demo` example. **Wired.**

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-quests/src/lib.rs` | Basic `Quest`/`QuestStep` + module re-exports | Active | `Quest`/`QuestStep` consumed by `aw_editor`. |
| `astraweave-quests/src/llm_quests.rs` | LLM quest data model + `LlmQuestGenerator` | In-Design (tested) | No production constructor; full LLM round-trip path. |
| `astraweave-quests/src/components.rs` | ECS quest components | In-Design (tested) | No ECS registration site found; not added via `app.add_system`/component spawn in production. |
| `astraweave-quests/src/systems.rs` | `QuestLlmSystem` + `integration` helpers | In-Design (tested) | Async, not an ECS `System`; orchestrates components manually. |
| `astraweave-quests/src/terrain_quests.rs` | Terrain-feature → quest generator | In-Design (tested) | Synchronous, no LLM; no production caller; no `TerrainQuestContext` producer. |
| `astraweave-weaving/src/quest.rs` | Gameplay `Quest`/`QuestManager`/`ObjectiveType` | Active | Wired via `VeilweaverLevel`. |
| `astraweave-weaving/src/quest_types.rs` | Extended objective payload types | Active | Referenced by `ObjectiveType` variants (escort/defend/boss/collect/timetrial). |
| `astraweave-weaving/src/starter_quests.rs` | 3 hand-authored starter quests | Active | `all_starter_quests` called by `VeilweaverLevel::new`. |
| `astraweave-weaving/src/level.rs` | `VeilweaverLevel` owning `QuestManager` | Active | Construction + reward application seam. |
| `astraweave-weaving/src/ui/quest_panel.rs` | Quest status UI | Active | Reads `QuestManager`. |

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Basic `Quest`/`QuestStep` (struct pair) | `astraweave-quests/src/lib.rs:5-30` | Active | Used by editor; also a conversion target via `integration::to_basic_quest` (`systems.rs:449`) and `LlmQuestGenerator::to_basic_quest` (`llm_quests.rs:624`). |
| `LlmQuest` rich model | `astraweave-quests/src/llm_quests.rs` | In-Design | Generative AI quest model. No production driver. |
| Weaving `Quest`/`QuestManager` | `astraweave-weaving/src/quest.rs` | Active | The actually-shipped gameplay quest system. |

**Two independent quest systems with no shared code.** `astraweave-quests` and `astraweave-weaving::quest` are separate crates with separately-defined `Quest`, `ObjectiveType`, and `QuestState` types. `astraweave-weaving` does not depend on `astraweave-quests` (its `quest.rs` imports only `glam` + `std`). There is no bridge, conversion, or shared trait between the two. Forensically: the richer `astraweave-quests` was built for the LLM/terrain integration campaigns (commit history references `LLM_integration` and `Editor stability phase1`/`Phase 10`), while `astraweave-weaving::quest` was built for the Veilweaver gameplay slice. Documenting only; no migration is proposed.

### Naming collisions

- **`Quest`**: In `astraweave-quests`, means `{ title, steps }`. In `astraweave-weaving`, means a full objective/reward/prerequisite quest. Unrelated.
- **`ObjectiveType`**: In `astraweave-quests` (`llm_quests.rs:114-126`), a flat tag enum. In `astraweave-weaving` (`quest.rs:22-70`), a payload-bearing progress enum. Unrelated.
- **`QuestState`**: Two definitions (`components.rs:133-141` 5-variant; `quest.rs:8-19` 4-variant). Unrelated lifecycles.
- **`to_basic_quest`**: Two functions converting `LlmQuest`→basic `Quest` — a free fn in `systems.rs::integration:449` and a method on `LlmQuestGenerator` (`llm_quests.rs:624`). Both produce the same `crate::Quest`.

### Known cognitive traps

- **Trap:** "Quest System" is one system.
  **Why it's confusing:** The CLAUDE.md crate-domain list groups `quests` and `weaving` together under Gameplay, and both expose a `Quest` type.
  **What's actually true:** They are two non-interacting implementations. Edits to one have no effect on the other.

- **Trap:** `TerrainQuestGenerator` produces `LlmQuest`, so it must use the LLM.
  **Why it's confusing:** The output type is named `LlmQuest`.
  **What's actually true:** `create_terrain_quest` is pure string templating; no `LlmClient` is involved (`terrain_quests.rs:412-564`).

- **Trap:** The ECS quest components imply a wired ECS quest system.
  **Why it's confusing:** Components are prefixed `C…` matching the engine's component convention.
  **What's actually true:** `QuestLlmSystem::update` takes `&mut` component references directly (an async method, not an ECS `System`); no `App::add_system` or component-spawn registration exists in production. See §4 caller check.

- **Trap:** `should_trigger` randomness.
  **Why it's confusing:** `TerrainQuestTrigger::should_trigger` accepts an `rng` and documents a probability roll.
  **What's actually true:** The roll is stubbed — it always returns `true` after the deterministic checks (`terrain_quests.rs:194-197`, comment: "using simple deterministic check for now").

---

## 7. Decision Log

### Decision: Build LLM-driven generative quests as a separate layer from the basic `Quest` struct
- **Date:** [Reasoning not recovered from available sources] (module introduced under the LLM-integration campaign; see `docs/archive/LLM_INTEGRATION_MASTER_PLAN.md` which lists `astraweave-quests` "extensions" for dynamic quest generation)
- **Status:** Accepted (layer exists; not wired to runtime)
- **Context:** The LLM integration plan called for dynamic quest generation as one of several LLM-extended subsystems.
- **Decision:** Add `llm_quests.rs` + ECS components + `QuestLlmSystem` on top of the existing basic `Quest`/`QuestStep`, with a `to_basic_quest` bridge for compatibility.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** A rich, test-covered but currently dormant generative layer. Per [ai_pipeline.md](./ai_pipeline.md):2379 the RAG/LLM pipeline behind it never executes in the runtime because no production consumer constructs `LlmQuestGenerator`.

### Decision: Keep the Veilweaver gameplay quest system in `astraweave-weaving` rather than reuse `astraweave-quests`
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted
- **Context:** The Veilweaver vertical slice needed concrete objective tracking (kill/repair/fetch/explore) wired into a level.
- **Decision:** Implement `Quest`/`QuestManager`/`ObjectiveType` natively in `astraweave-weaving` (deps: only `glam`/`std`), independent of `astraweave-quests`.
- **Alternatives considered:** [Reasoning not recovered from available sources] — no evidence the basic `astraweave-quests::Quest` was evaluated for this role.
- **Consequences:** Two quest systems coexist (§6). The weaving system is the one actually exercised by an example.

### Decision: `TerrainQuestGenerator` synthesizes quests without an LLM
- **Date:** Module added in `92d55e131` ("Editor stability phase1", first-add of `terrain_quests.rs`), labeled "Phase 10: Terrain-driven quest generation" (`lib.rs:42`)
- **Status:** Accepted
- **Context:** Terrain quests are tied to deterministic terrain features and world coordinates.
- **Decision:** Generate quests via template strings + difficulty math rather than LLM calls, while still emitting the shared `LlmQuest` shape.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** Deterministic, fast, testable; reuses `LlmQuest` as a transport type without LLM involvement.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `astraweave-weaving::QuestManager` has at most one `Active` quest at a time. | Yes | `activate_quest` errors if `active_quest_id.is_some()` (`quest.rs:450-452`); test `test_quest_manager_one_active_at_time` (`quest.rs:743`). |
| 2 | A weaving quest activates only from `Inactive`, and prerequisites in `completed_quests` must all be satisfied. | Yes | `activate_quest` (`quest.rs:430-460`); test `test_quest_manager_prerequisites`. |
| 3 | `Quest::check_completion` only transitions a quest that is currently `Active`. | Yes | `quest.rs:292-304`. |
| 4 | `astraweave-quests` basic `Quest::validate` rejects empty title or empty steps. | Yes | `lib.rs:18-25` + tests `test_quest_validate_*`. |
| 5 | Terrain quests respect `min_quest_spacing` and `max_active_quests`. | Yes | `terrain_quests.rs:371-385`; test `test_quest_spacing`. |
| 6 | `astraweave-quests` and `astraweave-weaving::quest` share no code and `astraweave-weaving` does not depend on `astraweave-quests`. | Yes | `astraweave-weaving/Cargo.toml` has no `astraweave-quests` dependency; `quest.rs` imports only `glam`/`std`. |
| 7 | `LlmQuestGenerator`/`QuestLlmSystem`/`TerrainQuestGenerator`/`CQuestGenerator` have zero non-test production constructors. | Yes (grep) | `rg '<Name>::new' -g '!*test*' -g '!*example*'` returns only test/bench/doc hits (§4). |

---

## 9. Testing & Validation

- **Unit tests:** Extensive inline `#[cfg(test)]` modules in all five `astraweave-quests` files and in `astraweave-weaving/src/quest.rs` (18 inline `#[test]` functions, verified by grep 2026-06-25; the previous "28" was incorrect) + `starter_quests.rs`.
- **Integration tests:** `astraweave-quests/tests/quest.rs` (basic), `astraweave-quests/tests/mutation_resistant_comprehensive_tests.rs`; `astraweave-weaving/tests/mutation_tests.rs` and `mutation_resistant_comprehensive_tests.rs` cover `QuestManager`/`ObjectiveType`/`all_starter_quests`.
- **Benchmarks:** `astraweave-quests/benches/quest_bench.rs` (criterion harness, declared in `Cargo.toml:28-30`).
- **Mutation testing:** `astraweave-quests` recorded as complete in `docs/current/MUTATION_TESTING_AUDIT.md` (66.5% raw / 100% adjusted, 341 mutants).
- **Manual validation:** `examples/veilweaver_quest_demo` is a full ASCII playthrough of all three starter quests through the weaving layer.

---

## 11. Open Questions / Parked Decisions

- **Will the `astraweave-quests` LLM/terrain/ECS layer be wired to a runtime consumer?** It is fully implemented and tested but has no production constructor (§4, §7-7). [ai_pipeline.md](./ai_pipeline.md):2379 already flags this. Parked: whether the engine should add an ECS-integrated `QuestLlmSystem` driver or whether the layer is reference scaffolding.
- **Who produces `TerrainQuestContext` in production?** The terrain generator's docstring (`terrain_quests.rs:1-12`) describes the AI orchestrator emitting terrain features; no such producer exists in `astraweave-terrain` today. Parked pending the AI-terrain orchestration integration.
- **Is the two-quest-system coexistence intentional long-term, or is one slated to absorb the other?** No bridge exists. Recorded factually in §6; disposition is Andrew's call.
- **`TerrainQuestTrigger::should_trigger` probability is stubbed to always-true** (`terrain_quests.rs:194-197`). Is the probabilistic roll intended to be activated before this path ships?

---

## 12. Maintenance Notes

**Update this doc when:**
- A production consumer is added that constructs `LlmQuestGenerator`, `QuestLlmSystem`, `TerrainQuestGenerator`, or the ECS quest components (this would flip their status from In-Design to Active in §5/§7-7).
- The two coexisting quest systems are bridged, merged, or one is removed (§6).
- `astraweave-weaving`'s `QuestManager`/`ObjectiveType`/`starter_quests` surface changes (§2A, §8).
- A `TerrainQuestContext` producer is wired (§11).

**Verification process:**
- Re-run the caller checks: `rg 'LlmQuestGenerator::new|QuestLlmSystem::new|TerrainQuestGenerator::new|TerrainQuestGenerator::default_config|CQuestGenerator::new' --type rust -g '!*test*' -g '!*example*' -g '!benches/*'` — expect zero hits while the layer remains dormant.
- Confirm `VeilweaverLevel::new` still constructs `QuestManager` from `all_starter_quests()` (`astraweave-weaving/src/level.rs:258-266`).
- Confirm `aw_editor` still imports only the basic `Quest`/`QuestStep` (`tools/aw_editor/src/main.rs:96`).
- Stamp the new commit hash and date in the Metadata section after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. There are **two** quest systems. `astraweave-weaving::quest` is the wired gameplay one; `astraweave-quests` LLM/terrain/ECS machinery is dormant (in-design-but-tested). Only `astraweave-quests::{Quest, QuestStep}` (basic structs) is consumed in production, by the editor.
2. `TerrainQuestGenerator` does **not** call the LLM despite emitting `LlmQuest`.
3. `Quest`, `ObjectiveType`, and `QuestState` are each defined twice with different shapes — always check which crate you're in.

**Files you'll most likely touch:**
- Gameplay quests: `astraweave-weaving/src/quest.rs`, `starter_quests.rs`, `level.rs`
- Generative/AI quests: `astraweave-quests/src/llm_quests.rs`, `systems.rs`, `terrain_quests.rs`, `components.rs`

**Files you should NOT touch without strong reason:**
- The basic `Quest`/`QuestStep` in `astraweave-quests/src/lib.rs:5-30` — load-bearing for `aw_editor`; changing the shape breaks the editor's quest-graph authoring.

**Common mistakes when changing this system:**
- Editing `astraweave-quests::Quest` expecting it to affect Veilweaver gameplay (it does not — that uses `astraweave-weaving::Quest`).
- Assuming the ECS quest components are scheduled by the engine — they are not; `QuestLlmSystem` is an async helper, not a registered ECS `System`.
- Treating the terrain quest output as LLM-validated content — it is template-synthesized.

---

## Appendix B: Historical context

`astraweave-quests` carries the residue of the LLM-integration campaign (see `docs/archive/LLM_INTEGRATION_MASTER_PLAN.md`, which lists `astraweave-quests` "extensions" for dynamic quest generation) and a later "Phase 10" terrain-quest addition (`lib.rs:42`, `terrain_quests.rs` first added in commit `92d55e131`). The basic `Quest`/`QuestStep` predate both and are the only part the editor ever adopted. The `astraweave-weaving::quest` layer grew alongside the Veilweaver gameplay slice as a self-contained, dependency-light objective tracker, which is why the two never converged. This document is the first architecture trace for the quest subsystem and the first to record the two-system split factually.
