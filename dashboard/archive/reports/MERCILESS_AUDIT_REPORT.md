# Merciless Code Audit: AstraWeave Engine
**Date:** November 18, 2025
**Auditor:** GitHub Copilot (Gemini 3 Pro)

## Executive Summary
This audit compares the *claimed* capabilities of the AstraWeave engine against the *actual* code implementation. The focus is on identifying "Fake Features" (stubs/TODOs), "Disconnected Systems" (parts that don't talk to each other), and "Critical Stability Risks" (fragile code).

**Verdict:** The engine has a solid "Modern ECS" foundation (`astraweave-ecs`), but the primary AI demo (`hello_companion`) runs on a legacy, disconnected "Core" system. Critical AI infrastructure is fragile and prone to crashing if external services (Ollama) are unavailable.

---

## 1. Fake Features & Technical Debt
*Features that appear to exist but are actually stubs or placeholders.*

| Component | Feature | Status | Location |
| :--- | :--- | :--- | :--- |
| **Editor** | ECS Integration | ❌ **Missing** | `tools/aw_editor/src/main.rs`: "TODO: Implement ECS world snapshot" |
| **Editor** | Hot Reload | ❌ **Missing** | `tools/aw_editor/src/main.rs`: "TODO: Implement hot reload" |
| **Editor** | Gizmos | ❌ **Missing** | `tools/aw_editor/src/widget.rs`: "TODO: Implement gizmo rendering" |
| **Rendering** | GPU Skinning | ⚠️ **Stubbed** | `astraweave-render/tests/skinning_parity_cpu_vs_gpu.rs`: "TODO: Implement GPU skinning" |
| **AI** | Embedding Batching | ⚠️ **Stubbed** | `astraweave-embeddings/src/client.rs`: "TODO: Implement batching" |

---

## 2. Disconnected Systems (The "Glue" Gap)
*Systems that exist in isolation but are not integrated into the main game loop.*

### The "Two Worlds" Problem
The engine currently maintains two completely separate "World" architectures that do not synchronize:

1.  **Legacy World (`astraweave-core`)**:
    -   **Used By:** `hello_companion` (The main AI demo).
    -   **Structure:** Simple `HashMap` storage.
    -   **Logic:** `tick()` only updates time and cooldowns. **No Physics. No Navigation.**
    -   **Consequence:** The AI demo is a "Brain in a Jar". It plans actions, but those actions don't trigger real game mechanics.

2.  **Modern World (`astraweave-ecs`)**:
    -   **Used By:** `astraweave-gameplay` (Combat, Crafting, Quests).
    -   **Structure:** Production-grade Archetype ECS.
    -   **Logic:** Has real systems (`combat_system`, `crafting_system`).
    -   **Consequence:** The actual gameplay code is **not running** in the AI demo.

**Impact:** The AI agents in `hello_companion` are not actually playing the game; they are playing a simplified simulation that ignores collision, physics, and actual gameplay rules.

---

## 3. Critical Stability Risks
*Code patterns that will cause the engine to crash in production.*

| Severity | Component | Issue | Location | Impact |
| :--- | :--- | :--- | :--- | :--- |
| 🚨 **CRITICAL** | **AI / Embeddings** | `unwrap()` on Network Call | `astraweave-embeddings/src/client.rs` | **Engine Crash** if Ollama/LLM service is offline or times out. |
| 🚨 **CRITICAL** | **Rendering** | `unwrap()` on GPU Adapter | `astraweave-render/src/headless.rs` | **Engine Crash** on server/CI environments without a GPU. |
| ⚠️ **HIGH** | **Editor** | `unwrap()` on Undo Stack | `tools/aw_editor/src/main.rs` | **Editor Crash** if user presses Undo on empty stack. |
| ⚠️ **HIGH** | **AI / Demo** | `unwrap()` on Sort | `examples/hello_companion/src/main.rs` | **Panic** if AI scores are NaN (e.g., division by zero). |

---

## 4. Recommendations

1.  **Bridge the Worlds:** Immediately deprecate `astraweave-core::World`. Refactor `hello_companion` to use `astraweave-ecs`. The AI should query the *real* ECS, not a simplified HashMap copy.
2.  **Harden AI Client:** Replace all `.await.unwrap()` calls in `astraweave-embeddings` with proper `Result` handling and retry logic. The game should not crash just because the LLM is thinking.
3.  **Implement Editor Basics:** The editor is currently a shell. Prioritize connecting it to the `astraweave-ecs` world so it can actually inspect entities.
4.  **Verify GPU Features:** Confirm if GPU skinning is actually implemented in `src/` and just missing in tests, or if the feature is entirely missing.

