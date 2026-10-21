# Phase 8.7: LLM & AI System Gap Analysis

**Date**: November 22, 2025
**Status**: COMPLETED
**Auditor**: AstraWeave Copilot

---

## 1. Arbiter System (`astraweave-ai/src/ai_arbiter.rs`)

### Strengths
- **Hybrid Architecture**: Seamlessly switches between GOAP (tactical) and LLM (strategic).
- **Async Handling**: Non-blocking LLM polling (<10 µs overhead).
- **Metrics**: Comprehensive tracking of transitions and success rates.

### Critical Gaps (Mission Critical Violations)
1.  **Blind Execution**: In `ExecutingLLM` mode, the arbiter executes the plan sequentially without re-validating preconditions. If the world state changes (e.g., enemy dies, cover destroyed), the agent will continue executing invalid actions.
    - *Risk*: High (Agent looks stupid/broken).
    - *Fix*: Add `validate_step()` check before execution.
2.  **No Interrupts**: Once in `ExecutingLLM`, the agent is locked in until the plan finishes or fails. High-priority tactical threats (e.g., grenade lands nearby) cannot interrupt the strategic plan.
    - *Risk*: High (Agent dies while "thinking").
    - *Fix*: Allow GOAP to override LLM if a "survival" goal becomes critical.

## 2. LLM System (`astraweave-llm/src/hermes2pro_ollama.rs`)

### Strengths
- **Robust Client**: Connection pooling, health checks, custom system prompts.
- **Streaming Support**: Basic NDJSON streaming implemented.

### Critical Gaps
1.  **Fragile Parsing**: The NDJSON parser manually scans buffers. This is brittle against network fragmentation or API changes.
    - *Risk*: Medium (Potential crashes/hangs).
    - *Fix*: Use a robust framing protocol or a dedicated JSON stream parser.
2.  **Stateless Client**: The client has no concept of context window. It relies on the caller to manage history.
    - *Risk*: High (Context overflow crashes).
    - *Fix*: Implement `ContextManager` with sliding window and summarization.
3.  **Chunk-based Streaming**: Returns `String` chunks, not structured tokens. Hard to parse partial JSON.
    - *Risk*: Low (UX latency).
    - *Fix*: Implement `TokenStream` iterator.

## 3. GOAP System (`astraweave-ai/src/goap/planner.rs`)

### Strengths
- **Hierarchical Planning**: HTN-style decomposition supported.
- **Risk Awareness**: Uses historical success rates.

### Critical Gaps
1.  **Fake Parallelism**: `plan_parallel` explicitly falls back to sequential execution.
    - *Risk*: Low (Suboptimal plans).
    - *Fix*: Implement true interleaved planning or remove the claim.
2.  **Basic Risk Model**: Risk is purely `1.0 - probability`. Does not account for severity (death vs inconvenience).
    - *Risk*: Medium (Agent takes fatal risks).
    - *Fix*: Add `severity` weight to failure recording.
3.  **Unbounded Search**: `max_plan_iterations` is hardcoded (10,000).
    - *Risk*: Medium (Frame spikes).
    - *Fix*: Make configurable per-agent or time-sliced.

---

## 4. Remediation Plan

### Priority 1: Arbiter Hardening
- Implement `validate_step(snap)` in `ExecutingLLM`.
- Implement `check_interrupts(snap)` in `ExecutingLLM`.

### Priority 2: LLM Robustness
- Implement `ContextManager` struct.
- Refactor streaming to use `serde_json::Deserializer::from_reader` (if possible) or robust line framing.

### Priority 3: GOAP Optimization
- Add `severity` to `ActionHistory`.
- Configurable iteration limits.

**Next Step**: Begin Priority 1 (Arbiter Hardening).
