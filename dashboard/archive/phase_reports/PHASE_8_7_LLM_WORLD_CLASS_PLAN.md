# Phase 8.7: World-Class LLM & Arbiter System Verification Plan

**Status**: ACTIVE
**Priority**: CRITICAL (Mission Critical Standards)
**Objective**: Exhaustively analyze, optimize, and validate the LLM, Arbiter, and GOAP systems, culminating in a production-ready interactive demo with Hermes 2 Pro.

---

## 🎯 Mission Objectives

1.  **System Perfection**: Elevate `astraweave-ai` and `astraweave-llm` to world-class standards.
2.  **Exhaustive Verification**: Prove reliability through rigorous testing (not just "it works").
3.  **Interactive Demo**: A real-time, 3D environment with a conversational AI NPC.
    *   **Environment**: House, Grass, HDRI Skybox, Background Music.
    *   **Modes**:
        1.  **Scripted GOAP**: Deterministic, rule-based responses.
        2.  **Local LLM (Hermes)**: Free-form chat via local inference.
    *   **UI**: Real-time chatbox with mode switching.

---

## 📅 Execution Roadmap

### Phase 1: Deep Analysis & Audit (Day 1)
**Goal**: Identify every weakness in the current architecture.

- [ ] **Arbiter System Audit** (`astraweave-ai/src/ai_arbiter.rs`)
    - Analyze state transitions (GOAP ↔ LLM).
    - Verify thread safety and async handling.
    - Check for race conditions in mode switching.
- [ ] **GOAP System Audit** (`astraweave-ai/src/goap/`)
    - Analyze planner performance (A* heuristic).
    - Verify action prerequisite/effect validation.
- [ ] **LLM System Audit** (`astraweave-llm/src/`)
    - Review `hermes2pro_ollama.rs` implementation.
    - Audit `circuit_breaker.rs`, `rate_limiter.rs`, `backpressure.rs`.
    - Verify prompt engineering structure and context window management.
- [ ] **Gap Analysis Report**: Document findings and required optimizations.

### Phase 2: System Hardening & Optimization (Days 2-3)
**Goal**: Refactor for performance, stability, and "Mission Critical" reliability.

- [ ] **Optimize Arbiter**
    - Implement zero-cost abstractions for state checks.
    - Ensure deterministic behavior in mode transitions.
- [ ] **Harden LLM Client**
    - Implement robust streaming response parsing.
    - Optimize context management (sliding window/summarization).
    - Add comprehensive telemetry/metrics for LLM performance.
- [ ] **Enhance GOAP**
    - Optimize plan formulation speed.
    - Add "interruptibility" for seamless LLM takeover.
- [ ] **Exhaustive Testing**
    - Add unit tests for all edge cases (network failure, hallucination, invalid JSON).
    - Add integration tests for full Perception → Reasoning → Action loop.

### Phase 3: Demo Environment Construction (Day 4)
**Goal**: Build the visual and auditory stage for the AI.

- [ ] **Scene Setup**
    - Create `examples/llm_interactive_demo`.
    - Initialize `astraweave-render` with WGPU 0.25.
- [ ] **Environment Assets**
    - **HDRI Skybox**: Load and render high-quality sky.
    - **Grass**: Implement instanced grass rendering.
    - **House**: Load a static mesh (GLTF/OBJ) for context.
- [ ] **Audio Atmosphere**
    - Initialize `astraweave-audio`.
    - Implement looping soft background music.

### Phase 4: Interactive UI & Logic (Day 5)
**Goal**: Connect the user to the AI.

- [ ] **Chatbox UI (egui)**
    - Scrollable history.
    - Input field with focus management.
    - Mode toggle switch (Scripted/LLM).
- [ ] **Interaction Logic**
    - **Mode 1 (Scripted)**: Trigger GOAP actions/dialogue based on keywords.
    - **Mode 2 (LLM)**: Stream user input to Hermes, stream response to UI.
- [ ] **NPC Behavior**
    - Idle animations.
    - "Thinking" state visualization.
    - Face player logic.

### Phase 5: Validation & Polish (Day 6)
**Goal**: Verify "World Class" status.

- [ ] **Performance Profiling**
    - Ensure 60 FPS rendering during LLM inference.
    - Measure latency (Time to First Token).
- [ ] **Mission Critical Validation**
    - Run 1-hour stability test.
    - Verify memory usage (leak detection).
- [ ] **Final Report**
    - Generate `LLM_WORLD_CLASS_VALIDATION_REPORT.md`.

---

## 🛠️ Technical Requirements

- **LLM**: Hermes 2 Pro (via Ollama).
- **Render**: WGPU 0.25 (Forward+ or Deferred).
- **UI**: egui 0.28/0.29 (match workspace version).
- **Standards**: 90%+ Test Coverage, Zero Warnings, Deterministic.

## 🚀 Next Step
Begin **Phase 1: Deep Analysis & Audit**.
