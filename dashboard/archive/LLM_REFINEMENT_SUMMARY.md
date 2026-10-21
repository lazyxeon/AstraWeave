# AstraWeave LLM Integration - Refinement Summary

**Version**: 1.1  
**Date**: October 2025  
**Status**: Planning Refinement Complete

---

## 1. Overview

This document summarizes the key refinements made to the LLM Integration Master Plan based on strategic feedback. These changes enhance the plan's robustness in five critical areas: **Security, Model Lifecycle, Evaluation, Game Loop Integration, and Fail-Safety**.

The following documents have been updated to reflect these refinements:
- `LLM_INTEGRATION_MASTER_PLAN.md` (v1.1)
- `LLM_INTEGRATION_TRACKER.md` (v1.1)

---

## 2. Summary of Refinements

### Refinement 1: Enhanced Security (`ToolGuard` Layer)

**Recommendation**: Add a `ToolGuard` layer to ensure LLM-generated commands cannot access unsafe resources or panic the engine, using an approved command registry.

**Analysis**: The existing `Tool Sandbox` validates the *parameters* of known actions but does not explicitly prevent the generation of arbitrary or unsafe *commands*. A `ToolGuard` layer provides a critical security boundary.

**Solution**:
1.  **Architecture**: A new `ToolGuard` service will be added to the **Foundation Layer**. It will sit between the `Orchestrator` and the ECS command buffer.
2.  **Mechanism**:
    - An `ActionRegistry` resource will be created, containing a whitelist of all permissible ECS commands that can be initiated by an AI agent.
    - The `ToolGuard` will intercept every `PlanIntent` from the LLM.
    - It will validate that each `ActionStep` in the plan corresponds to a registered action in the `ActionRegistry`.
    - Any plan containing an unregistered or malformed action will be rejected before it can be dispatched to the ECS.
3.  **Master Plan Update**: The "Architecture Overview" and "Risk Assessment" sections have been updated to include the `ToolGuard` layer.
4.  **Tracker Update**: A new task has been added to **Phase 1, Week 4** to "Implement `ToolGuard` and `ActionRegistry`".

---

### Refinement 2: Model Lifecycle & Versioning

**Recommendation**: Implement a model registry to track model versions, checksums, and properties (e.g., embedding dimensions) to manage model changes during RAG or fine-tuning.

**Analysis**: The original plan lacked a formal system for managing multiple model versions, which is essential for reproducibility, preventing data corruption (e.g., mismatched embedding dimensions), and safe A/B testing.

**Solution**:
1.  **New Artifact**: A new crate, `astraweave-models`, will be created. It will contain a `registry.toml` file.
2.  **Registry Schema**:
    ```toml
    # astraweave-models/registry.toml
    [[model]]
    id = "phi3-medium-4k-instruct-q4"
    family = "phi3"
    version = "1.0.0"
    source = "ollama"
    checksum = "sha256-abc123..."
    embedding_dims = null
    
    [[model]]
    id = "all-minilm-l6-v2-f32"
    family = "sentence-transformers"
    version = "1.0.0"
    source = "onnx"
    checksum = "sha256-def456..."
    embedding_dims = 384
    ```
3.  **Integration**: The `LlmClient` and `EmbeddingClient` will now load models by referencing the `registry.toml`. This ensures that the correct model version and its associated parameters (like `embedding_dims`) are always used.
4.  **Master Plan Update**: A new section, "Model Lifecycle Management," has been added.
5.  **Tracker Update**: A new task has been added to **Phase 1, Week 1** to "Create `astraweave-models` crate and `registry.toml`".

---

### Refinement 3: Automated Evaluation Harness

**Recommendation**: Include automated quality scoring for dialogue, quests, and boss adaptations using prompt-based evaluation sets to provide data-driven quality metrics.

**Analysis**: This transforms qualitative goals like "feels intelligent" into measurable, trackable metrics, enabling automated regression testing and A/B test analysis.

**Solution**:
1.  **New Crate**: A new crate, `astraweave-evaluation`, will be developed.
2.  **Harness Components**:
    - **Evaluation Sets**: Curated datasets of prompts and expected outcomes (e.g., `data/eval/dialogue_coherence.json`).
    - **Scorers**: Implementations of standard metrics (BLEU, ROUGE for text) and custom game-specific metrics (e.g., `BossAdaptationWinRate`, `QuestFeasibilityScore`).
    - **Runner**: A CLI tool or test runner that executes evaluation sets against a model and computes scores.
3.  **Master Plan Update**: The "Testing & Validation Strategy" section has been expanded to include the "Evaluation Harness."
4.  **Tracker Update**: A new task has been added to **Phase 4, Week 15** to "Build Automated Evaluation Harness".

---

### Refinement 4: Game Loop Integration (`LLM System Scheduler`)

**Recommendation**: Specify how async LLM systems integrate with the synchronous ECS loop, suggesting an `LLM System Scheduler` ECS resource.

**Analysis**: The plan needed a more concrete bridge between the async world of LLM clients and the sync world of the ECS. A dedicated scheduler is the correct pattern to manage this.

**Solution**:
1.  **Architecture**: A new ECS resource, `LlmScheduler`, will be introduced.
2.  **Mechanism**:
    - **Dispatch**: ECS systems (e.g., `ai_planning_system`) will not block. Instead, they will submit an `LlmRequest` to the `LlmScheduler`. The scheduler returns a `Future<LlmResponse>`.
    - **Polling**: A dedicated ECS stage, `SystemStage::AI_RESPONSE_POLL`, will run a system that polls the `LlmScheduler` for completed futures.
    - **Sync**: When a response is ready, the polling system writes the result back into an ECS component (e.g., `CPlanIntent`) for the next system in the pipeline to consume.
3.  **Master Plan Update**: The "Architecture Overview" and "Data Flow" diagrams have been updated to include the `LlmScheduler` and the new ECS stage.
4.  **Tracker Update**: A new task has been added to **Phase 1, Week 4** to "Implement `LlmScheduler` ECS Resource and Polling System".

---

### Refinement 5: Explicit Fail-Safe Paths

**Recommendation**: Define deterministic, rule-based fallbacks for when the LLM fails mid-session to ensure playability is never halted.

**Analysis**: The plan mentioned "graceful degradation" but lacked a specific implementation strategy. Using pre-defined templates is a robust and practical solution.

**Solution**:
1.  **Mechanism**:
    - The `LlmClient`'s error handling will be enhanced. On a terminal failure (e.g., after retries are exhausted or a circuit breaker trips), it will return a specific `LlmError::ServiceUnavailable`.
    - Systems like `DialogueGenerator` or `QuestGenerator` will catch this error.
    - Upon catching the error, they will use the `astraweave-prompts` `PromptTemplate` engine to render a deterministic, pre-defined fallback template (e.g., `fallbacks/dialogue_generic.toml`).
    - This ensures the game always has a valid, albeit less dynamic, response.
2.  **Master Plan Update**: The "Production Hardening" section has been updated with a detailed "Deterministic Fallback Strategy."
3.  **Tracker Update**: A new task has been added to **Phase 4, Week 16** to "Implement and Test Deterministic Fallback Templates".
