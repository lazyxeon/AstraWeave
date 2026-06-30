# M1.3 / A2 — Arbiter Architecture Recon + Ratified A2 Criterion

> **Campaign**: R-series · **Phase**: M1.3 / A2 recon · **Branch**: `campaign/roadmap`
> **Mode**: READ-ONLY RECON — established the arbiter's real architecture and the corrected A2 criterion. No test written, no code changed.
> **Input**: R.1 roadmap M1 A2 item; R.0.A §3 A2 criterion ("GOAP↔BT↔LLM mode transitions fire on documented triggers").
> **Status**: **RATIFIED (2026-06-30).** Director decision: the hybrid `AIArbiter` is **post-v1.0**; **A2 = production-path determinism** (`RuleOrchestrator` + `GoapOrchestrator`, the wired deterministic path). The A2 test is the next beat (M1.3-test).
> **Date**: 2026-06-30

---

## Verdict (headline)

The original A2 phrasing is **not fiction** — a real arbiter with real runtime transitions exists (`AIArbiter`), so the "no transitions, just config + async" hypothesis is **refuted**. But the criterion was **mis-targeted**: it describes a **tested-but-dormant, feature-gated, LLM-requiring** subsystem, while the **wired** production AI path is the deterministic `RuleOrchestrator` (+ `GoapOrchestrator` in the flagship demo). Both halves of the criterion (determinism + transitions) are **already substantially tested**. **Ratified outcome: the arbiter is post-v1.0; A2 certifies the wired deterministic path's determinism** (a small consolidation beat, largely already met).

---

## 1. The actual architecture — two mode systems

### 1a. `PlannerMode` (`core_loop.rs:25`) — CONFIGURATION (not transitions)

`enum PlannerMode { Rule, BehaviorTree, GOAP }` (no LLM variant). A `CAiController` is *constructed* with a mode; `dispatch_planner` (`core_loop.rs:243`) routes by it. **No runtime transitions.** And:
- **`PlannerMode::BehaviorTree` is a STUB:** `dispatch_bt` → `anyhow::bail!("BehaviorTree integration not yet implemented")` (`core_loop.rs:278`).
- `PlannerMode::GOAP` → `dispatch_goap` is **real but canned** (`snapshot_to_goap_state` hardcodes facts; `core_loop.rs:283+`) — note this is the *bench-path* GOAP integration, distinct from the real `GoapOrchestrator`/`GoapPlanner` (§1e).
- `PlannerMode::Rule` → `RuleOrchestrator` (real).
- **Wiring:** `dispatch_planner`/`CAiController` appear **only in `astraweave-ai/benches/ai_core_loop.rs`** + tests — not the production loop.

### 1b. `AIControlMode` (`ai_arbiter.rs:93`) — RUNTIME TRANSITIONS (real)

`enum AIControlMode { GOAP, ExecutingLLM { step_index }, BehaviorTree }`. `AIArbiter::update()` (`ai_arbiter.rs:384`) genuinely transitions:
- **GOAP → ExecutingLLM** when an async LLM plan completes (`poll_llm_result` → `transition_to_llm`, `:394-401`); LLM requested in GOAP when the 15 s cooldown elapses (`maybe_request_llm`, `:414`).
- **ExecutingLLM → GOAP** on plan exhaustion / invalid step (`:442-471`).
- **GOAP → BT** when GOAP returns an empty plan (`transition_to_bt`, `:423`).
- **BT → (nothing)** — `update()` just returns BT's first step (`:475-483`). **BT is a terminal emergency sink, no recovery.**

So the real graph is **GOAP ↔ ExecutingLLM (bidirectional) + GOAP → BT (one-way)**, *not* the bidirectional "GOAP↔BT↔LLM" the criterion implied. `transition_to_llm` is `pub` "for testing (manual plan injection)" (`:505`) — transitions are testable without a live LLM.

### 1c. `AIArbiter` is NOT production-wired; `fast_executor` is dead

`rg AIArbiter` outside `astraweave-ai`/examples ⇒ **empty**. Every use is a doc-comment, a `#[cfg(test)]` helper, or internal recursion. **`fast_executor`** (`ai_arbiter.rs:209`) is **stored at construction and never read** (the IDE `dead_code` flag is correct) — the advertised dual-executor is half-decorative.

### 1d. The production AI path

`ecs_ai_plugin::sys_ai_planning` (`ecs_ai_plugin.rs:45`, registered into the `ai_planning` stage) uses **`RuleOrchestrator`** (`:12,99,210`). **The wired production AI is the deterministic `RuleOrchestrator`** — not the arbiter, not the configured GOAP/BT dispatch.

### 1e. Closer-look findings (the arbiter-scope evidence)

Requested before the v1.0-scope decision; all confirm the arbiter is dormant/optional:
- **`llm_orchestrator` is OFF by default** — `astraweave-ai/Cargo.toml` `default = []`; the arbiter's LLM path requires the `llm_orchestrator` feature + `astraweave-llm/ollama` + a tokio runtime + a **live Ollama LLM** (runtime default `phi3:medium`).
- **`hello_companion` (the flagship AI demo) runs `RuleOrchestrator` + `GoapOrchestrator`** (`examples/hello_companion/src/main.rs:50,53` → `orch.propose_plan`, `:663,676`), **not the `AIArbiter`.** The "Arbiter" `DemoMode` is a *dialogue-presentation flavor* (canned content in `dialogue_bank.rs`), not the real `AIArbiter::update` gameplay loop.
- **The arbiter is a "Phase 7 Arbiter" campaign** (archived completion reports: `docs/archive/.../ARBITER_IMPLEMENTATION.md`, `PHASE_7_ARBITER_PHASE_*_COMPLETE.md`) — built + tested + "complete," but never wired into the default loop. The `ai_pipeline.md` trace confirms it **bypasses the entire (dormant ~15K LoC) LLM hardening surface** (no rate limiting / circuit breaking / retry / ToolGuard / 4-tier fallback) and has **zero production constructors**.
- **GOAP is real** — `GoapOrchestrator`/`GoapPlanner` (A*), used by `hello_companion`; the "canned" facts in §1a are specific to `core_loop::dispatch_goap`'s bench path, not the planner.

**Implication:** the wired, default, v1.0-relevant AI path is `RuleOrchestrator` + `GoapOrchestrator` (both deterministic). Wiring the arbiter for v1.0 would mean enabling `llm_orchestrator` by default + a hard live-LLM dependency + connecting the dormant hardening — a large capability build, not a test. **Hence the ratified post-v1.0 scope.**

---

## 2. The real A2-relevant contracts

| Contract | Reality | Already tested? |
|---|---|---|
| **Determinism (production path)** | `RuleOrchestrator::propose_plan` pure over `WorldSnapshot`; GOAP sorts/hashes deterministically (`goap/persistence.rs:50,62`) | **Yes** — `goap/tests.rs:92 test_complete_determinism`, `orchestrator.rs:992/997`, `mutation_tests.rs:365` |
| **Arbiter transition-correctness** | Real, but on a dormant/feature-gated subsystem (post-v1.0) | **Yes** — `test_poll_llm_result_success_transitions_to_executing_llm`, `test_transition_to_goap_clears_plan`, `..._increments_mode_transitions` (71 "transition" refs) |
| **Mode/planner correctness** | Rule real; GOAP real (`GoapOrchestrator`); BT-`PlannerMode` is a stub | Rule/GOAP yes; BT-`PlannerMode` untestable (unimplemented) |
| **LLM integration** | Async, non-deterministic *generation*; falls back to GOAP; post-v1.0 | Partly (failure→GOAP covered by transition tests) |

---

## 3. The ratified A2 criterion (post-v1.0 arbiter)

The original "GOAP↔BT↔LLM mode transitions fire on documented triggers" was wrong in three ways: (i) it implied a **bidirectional** graph, but BT is a one-way terminal sink and there is no "BT↔LLM" edge; (ii) it described the **dormant, feature-gated `AIArbiter`**, not the wired path; (iii) it omitted that the production path is the deterministic `RuleOrchestrator`/`GoapOrchestrator` and that determinism + transitions are **already tested**.

**Ratified A2 criterion (replaces R.0.A §3 / R.1 M1 A2 wording):**

> **A2 is met when the WIRED, deterministic production AI path is provably deterministic:** for identical `WorldSnapshot` input, `RuleOrchestrator::propose_plan` (the `ecs_ai_plugin` production path) — and `GoapOrchestrator` (the deterministic GOAP path used by the flagship demo) — yield **identical `PlanIntent`** across repeated runs. No latency bound (the AI path is synchronous and off any LLM dependency for v1.0).
>
> **The hybrid `AIArbiter` (runtime transitions + async LLM) is post-v1.0** — it is tested ("Phase 7 complete") but feature-gated (`llm_orchestrator`, off by default), LLM-requiring, and not production-wired. Its transition-correctness is **already covered** by existing tests; wiring it (enable the feature by default, accept the live-LLM dependency, connect the dormant LLM hardening, fix the BT-stub / dead-`fast_executor` / one-way-BT) is a **post-v1.0 capability build**, not a v1.0 A2 requirement.

---

## 4. Proposed A2 test (against the ratified criterion — NOT written)

For the **next beat (M1.3-test)**:
- **Determinism test [the A2 deliverable]:** run the production snapshot→plan path on a fixed `WorldSnapshot` ≥N times, assert byte-identical `PlanIntent` (`plan_id` + `steps`): (1) `RuleOrchestrator::propose_plan` (the wired `ecs_ai_plugin` path), (2) `GoapOrchestrator` (the deterministic GOAP path). Mostly consolidates `goap/tests.rs:92` + `orchestrator.rs:992` under an explicit, named "A2 determinism" contract test so the wired path's determinism is pinned against future drift.
- **No arbiter-transition test in v1.0** — those tests already exist and the arbiter is post-v1.0; they stay as the in-design arbiter's regression net.

---

## 5. Follow-ons (flagged, not done in this beat)

- **Sync the roadmap A2 wording:** R.0.A §3 and R.1 M1's A2 item still carry the superseded "GOAP↔BT↔LLM transitions" phrasing; update them to point to this ratified criterion (a small doc-sync, like the v1.1-split-fold-into-R.0.A item).
- **Record the arbiter as in-design/post-v1.0** in the `ai_pipeline.md` trace front-matter / CLAIM-MISMATCH backlog (it is tested-but-not-wired; the "stable AI pipeline" v1.0 claim rests on `RuleOrchestrator`/`GoapOrchestrator`, not the arbiter).
- **Honesty gaps (own decisions):** the `PlannerMode::BehaviorTree` stub (`dispatch_bt`), the dead `fast_executor`, and the one-way "↔BT" (no recovery from the emergency sink) — wire, relabel, or remove; not v1.0 A2 blockers.

---

## What this is NOT

- **NOT writing the A2 test** — the next beat, against the ratified criterion.
- **NOT changing arbiter code** — `ai`/`behavior` stay VERIFIED-PRODUCTION; the stub/dead/one-way-BT findings are recorded, their fix is a separate decision.

*Read-only diagnostic; ratified. Tree unchanged. The A2 test (production-path determinism) is the next beat.*
