# Aria Companion Behavior Spec

## Purpose

- Define the behavior architecture for Aria within the Veilweaver vertical slice.
- Map goals/actions to existing `astraweave-ai` GOAP framework, including LLM fallback touches.
- Describe memory layers, telemetry hooks, and tuning knobs required for deterministic-yet-adaptive support.

## Behavior Architecture Overview

- **Planner**: `astraweave_ai::goap::Planner` running at 10 Hz (every 6 frames), seeded for determinism.
- **Arbiter**: `AIArbiter` manages hybrid mode switching between GOAP (default) and LLM banter (dialogue only).
- **Action Executor**: Systems in `astraweave-gameplay` perform action side effects, each wrapped in command structs.

## GOAP Goals & Priorities

| Priority | Goal | Satisfaction Condition | Notes |
|----------|------|------------------------|-------|
| 1 | `ProtectPlayer` | Player health > 40%, no lethal threats within 8 m | Hard constraint; triggers heal or taunt behaviors. |
| 2 | `StabilizeThreads` | Active weave anchors ≥ required count per zone | Ensures tutorial beats complete; high weight in Z1/Z4. |
| 3 | `ExploitStagger` | Last enemy stagger event resolved with follow-up within 2 s | Encourages combo synergy when player staggers. |
| 4 | `MaintainPositioning` | Aria within 6 m of player, not blocking path | Quality-of-life behavior; prevents friendly obstruct. |

## Actions & Preconditions

| Action | Preconditions | Effects | Execution Backend |
|--------|---------------|---------|-------------------|
| `CastStabilityPulse` | Weave anchor unstable AND Echo Charge ≥ 1 | Anchor stabilized, Echo Charge -= 1 | `astraweave_weaving::actions::StabilityPulse` |
| `DeployBarrier` | Combat active, Echo Charge ≥ 1, no barrier present | Spawns cover at designated anchor, Echo Charge -=1 | `astraweave_gameplay::cover::spawn_barrier` |
| `MarkSentinel` | Sentinel target exists, not marked | Applies debuff for +15% damage window, 6 s duration | `astraweave_gameplay::status::apply_mark` |
| `HealPlayer` | Player health < 55%, cooldown ready | Restore 20% HP over 3 s | `astraweave_gameplay::support::channel_heal` |
| `ExecuteCombo` | Enemy staggered, player prox ≤ 4 m, combo not used in last 8 s | Deals 250 stagger damage, sets combo cooldown | Custom script hitting `CombatCommand::ComboStrike` |
| `Reposition` | Distance to player > 8 m OR collision risk | Pathfind near player using nav agent | `astraweave_nav::agent::set_target` |

## Memory Layers

- **Episodic (short-term)**
  - Rolling window (10 events) capturing: weave successes/failures, player damage taken, combo usage.
  - Serialized post-run to feed recap metrics (`CompanionAffinity`).
- **Semantic (slice-specific)**
  - Learned player tendency flags: `MeleeFavored`, `WeaveDefensive`, `RangedFocus`.
  - Updated every 20 seconds via telemetry aggregator; drives adaptive unlock selection.
- **LLM Context (dialogue only)**
  - Summarized state: last choice at Loom Crossroads, boss phase, player health band.
  - Passed into Hermes prompt template; sanitized and truncated to ≤512 tokens.

## Adaptive Unlock Logic

1. Calculate weighted ratios:
   - `melee_ratio = melee_damage / total_player_damage`
   - `defense_ratio = weaves_used_defensively / total_weaves`
2. Unlock decision:
   - If `melee_ratio >= 0.7` → grant `Threadbind Riposte` (Action effect: adds counter window to combo).
   - Else if `defense_ratio >= 0.5` → grant `Stability Surge` (Action effect: enhanced barrier).
   - Else default to `Stability Surge` to maintain tutorial success.
3. Persist chosen unlock in run metadata for outro recap and future balancing.

## Telemetry Hooks

- Emit `CompanionEvent::ActionExecuted { action_id, success, latency_ms }` for every GOAP action.
- Emit `CompanionEvent::AdaptiveUnlock { unlock_id }` once per run on unlock resolution.
- Forward to `telemetry_hud` for inclusion in post-run metrics panel.

## Implementation Tasks

1. Extend `astraweave-ai` GOAP config with goals/actions above (feature flag `veilweaver_slice`).
2. Implement companion-specific resource tracker (`EchoCharge` shared with player via ECS resource).
3. Wire telemetry events through `astraweave-observability::events`.
4. Update dialogue system to read unlock outcome for branch-specific banter.

---

*Defines Aria’s behavior blueprint to guide upcoming implementation work.*

