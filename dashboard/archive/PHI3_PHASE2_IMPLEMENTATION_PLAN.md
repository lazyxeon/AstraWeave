# Phase 2 — Phi‑3 LLM Integration: Detailed Implementation Plan

Date: 2025-10-14
Status: Planning Approved (ready to execute)
Scope: Bring Phi‑3 LLM integration to production parity with classical AI validation (28 tests, A+ grade), with no placeholders or stubs.

## Goals and Success Criteria

- Functional: End-to-end planning via Phi‑3 with safe fallbacks, zero stubs.
- Quality: 28 tests total (match classical AI tiers), all passing locally.
- Performance: <50 ms p95 cache-hit plan time (Mock + Cache path), ≥100 plans/sec throughput (cache-heavy), <20% fallback rate in demos.
- Stability: No hangs; hard timeouts enforced; retries + circuit breaker.
- Observability: Basic telemetry (latency, hit/miss, retries, fallbacks).
- Documentation: Developer guide + examples updated.

## Deliverables

1. Prompt Cache module (exact-match + pluggable similarity), LRU with metrics.
2. Timeout + Retry + Circuit Breaker around LLM calls and planning pipeline.
3. Telemetry: counters + histograms; simple in-memory aggregator.
4. Benchmarks (criterion) and a lightweight stress harness (example binary).
5. 7 new tests (raise total to 28), including edge and stability tests.
6. Docs: LLM Integration Guide, README updates, example wiring.

## Work Breakdown Structure (WBS)

1) Prompt Caching (8–10h)
- Files/modules:
  - astraweave-llm/src/cache/mod.rs (new)
  - astraweave-llm/src/cache/lru.rs (new, or use crate `lru`)
  - astraweave-llm/src/cache/key.rs (PromptKey hashing + optional SimHash)
  - astraweave-llm/src/telemetry.rs (shared metrics; see item 3)
  - Integrations in astraweave-llm/src/lib.rs (plan_from_llm)
- Functionality:
  - Exact-match cache by default: key = stable hash of normalized prompt (strip timestamps, whitespace normalization) + model id + temperature + tools hash.
  - Optional similarity mode (feature flag `llm_cache_sim`): char 3-gram TF-IDF cosine or SimHash LSH (pure-Rust, no large model dep). Returns nearest neighbor above similarity threshold.
  - LRU eviction policy, capacity configurable (env: LLM_CACHE_CAP, default 4096 entries).
  - Metrics: hit, miss, similarity_hit, evictions, size.
- API:
  - struct PromptCache { get(&PromptCtx) -> Option<CachedPlan>, put(key, plan, meta) }
  - struct CachedPlan { plan: PlanIntent, created_at: Instant, tokens_saved: u32 }
  - enum CacheDecision { HitExact, HitSimilar(f32), Miss }
- Acceptance:
  - Unit tests for key hashing stability, LRU eviction, similarity threshold behavior, and metrics update correctness.
  - Integration test: first request Miss; second identical request HitExact; altered prompt HitSimilar if similarity enabled.
  - p95 cache hit path < 2 µs on dev machine (criterion bench).

2) Timeout and Retry (3–4h)
- Files:
  - astraweave-ai/src/orchestrator.rs (budget plumbing)
  - astraweave-llm/src/lib.rs (plan_from_llm wrapper)
  - astraweave-llm/src/llm_client.rs or existing trait impl sites
- Functionality:
  - Enforce hard timeout using `tokio::time::timeout` on LLM calls and overall planning (respect `_budget_ms`).
  - Retry policy: 3 attempts with exponential backoff + jitter (50 ms, 100 ms, 200 ms base), only on transient errors (timeouts, 5xx, network).
  - Early cancel when budget is exhausted.
- Acceptance:
  - Tests simulate slow client -> timeout triggers; retry engages; stops when budget exceeded.
  - No hangs; planning always resolves with either LLM plan or fallback within budget.

3) Circuit Breaker (4–5h)
- Files:
  - astraweave-llm/src/circuit_breaker.rs (new)
  - Wiring in lib.rs around LLM calls
- Functionality:
  - Rolling window failure counter; open after N consecutive failures (default 5) or failure ratio threshold over time window.
  - Half-open test attempts after cool-down; success closes breaker.
  - When open, immediately route to fallback (and record metric).
- Acceptance:
  - Unit tests for Open/Half-Open/Closed transitions and cooldown.
  - Integration test with injected failures -> breaker opens -> calls short-circuit to fallback -> recovery.

4) Telemetry (2–3h)
- Files:
  - astraweave-llm/src/telemetry.rs (new)
  - Exports simple counters and histograms; thread-safe (Arc<AtomicU64>, HDR-like bins optional simplified)
- Metrics captured:
  - llm.request.count, llm.success.count, llm.error.count
  - llm.retry.count, llm.circuit_open.count, llm.fallback.count
  - cache.hit, cache.miss, cache.sim_hit, cache.evictions
  - latency.ms: llm_call, plan_total, cache_lookup
- Acceptance:
  - Unit tests: increment semantics and snapshot readout.
  - Optional `debug_io` feature to dump metrics on drop in tests.

5) Benchmarks + Stress Harness (6–8h)
- Files:
  - astraweave-llm/benches/cache_benches.rs (criterion)
  - astraweave-llm/benches/parse_benches.rs (JSON extract/parse)
  - examples/llm_stress/src/main.rs (new example)
- Functionality:
  - Measure: cache lookup p50/p95, JSON extraction speed, mock LLM full pipeline throughput (100–10k requests synthetic mix with configurable cache hit rate).
  - Stress harness parameters: --agents, --duration, --hit-rate, --budget-ms.
- Targets:
  - Cache hit p95 < 2 µs; pipeline (mock+cache) p95 < 50 ms; ≥100 plans/sec at 70% hit rate.
- Acceptance:
  - Benchmarks run successfully; results logged to console; thresholds documented.

6) Tests to reach 28 total (6–8h)
- Add 7 tests (names illustrative):
  - test_prompt_cache_exact_hit_miss
  - test_prompt_cache_lru_eviction_order
  - test_prompt_cache_similarity_threshold (feature `llm_cache_sim`)
  - test_timeout_respected_within_budget
  - test_retry_exponential_backoff_sequence
  - test_circuit_breaker_open_and_recover
  - test_telemetry_counters_and_fallback_rate
- Acceptance:
  - All tests pass locally via existing Phase1 tasks; no flaky timing (use deterministic backoff with seeded jitter in tests).

7) Documentation & Examples (2–3h)
- Files:
  - LLM_INTEGRATION_GUIDE.md (new)
  - README.md (LLM section update: env vars, features, cache knobs)
  - examples/phi3_demo: show cache/timeout flags; new `llm_stress` example
- Acceptance:
  - Follows Quick Start with Ollama; shows env vars: OLLAMA_URL, OLLAMA_MODEL, LLM_CACHE_CAP, LLM_TIMEOUT_MS, LLM_RETRY_MAX, LLM_BREAKER_THRESHOLD.

## Dependencies and Feature Flags

- New crates (minimal, well-known):
  - lru = "0.12" (or implement simple LRU ourselves if avoiding deps)
  - ahash = "0.8" (fast hashing for PromptKey)
  - criterion = { version = "0.5", optional = true }
- Feature flags in astraweave-llm/Cargo.toml:
  - llm_cache (default on)
  - llm_cache_sim (optional) — enables similarity search
  - llm_bench (optional) — enables criterion benches
  - debug_io (existing) — prints telemetry in tests/demos

## API Changes (backwards-compatible)

- astraweave_llm::plan_from_llm now consults PromptCache before calling client; on cache hit, returns immediately and records metrics.
- Orchestrator budget respected strictly via `tokio::time::timeout`; signature unchanged.
- New public types behind feature flags:
  - PromptCache, CachedPlan, CacheDecision
  - CircuitBreaker
  - TelemetrySnapshot

## Day-by-Day Schedule (3–4 days)

Day 1 (8–10h)
- Implement PromptCache (exact-match + LRU) + metrics.
- Wire into plan_from_llm; unit tests for cache basics.
- Add timeout enforcement in orchestrator and client wrapper; tests for timeout.

Day 2 (7–8h)
- Implement Retry (backoff+jitter) + Circuit Breaker with tests.
- Telemetry counters and histograms; tests.
- Begin criterion benches (cache, parse).

Day 3 (7–8h)
- Complete benches; add stress example `llm_stress`.
- Add remaining tests to reach 28; stabilize timings (seeded jitter).
- Documentation (LLM_INTEGRATION_GUIDE.md, README updates).

Optional Day 4 (4–6h buffer)
- Polish, address clippy warnings, refine thresholds, run extended stress.

## Acceptance Criteria (Pass/Fail)

- Build: PASS cargo check on workspace (Phase1-check task).
- Lint: PASS clippy on touched crates (warnings allowed if noisy, but document).
- Tests: PASS 28/28.
- Benchmarks: Achieve targets or document shortfall + follow-up.
- Demos: `phi3_demo` and `llm_stress` run without hangs; fallback rate < 20% on defaults.

## Risks and Mitigations

- Risk: Similarity cache adds complexity/timing flakiness.
  - Mitigation: Keep behind feature flag; default to exact-match; deterministic tests.
- Risk: Criterion benches vary across machines.
  - Mitigation: Report relative metrics and rough targets; do not gate CI on benches.
- Risk: Circuit breaker false positives under transient spikes.
  - Mitigation: Use both consecutive-failure threshold and time-window ratio; expose env knobs.
- Risk: Dependency bloat.
  - Mitigation: Prefer tiny, popular crates (lru, ahash) or implement minimal LRU in-house.

## Implementation Notes

- Prompt normalization: trim, collapse whitespace, remove volatile sections (timestamps, RNG seeds) before hashing.
- Tool registry hash: stable serialization (sorted keys) -> blake3 or ahash digest for PromptKey.
- Budget propagation: Track remaining time across retries; abort when remaining <= 0.
- Telemetry: Use AtomicU64 counters and a simple ring buffer for latency buckets (e.g., 0–1, 1–2, 2–4, 4–8, … ms).

## Integration Plan

- Implement behind feature flags; default `llm_cache` ON.
- Validate locally with existing `Phase1-check` and `Phase1-tests` tasks.
- Land in small PR-sized commits per WBS item to ease review.

## Next Step

- Proceed to Phase 3 execution: start with PromptCache + Timeout wiring (Day 1 tasks).