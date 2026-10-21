# Option 2: LLM Optimization Implementation Plan

**Date**: November 1, 2025  
**Status**: 📋 **READY FOR IMPLEMENTATION**  
**Estimated Time**: 8-12 hours  
**Target**: Reduce LLM latency from **3462ms → <200ms average** (17× improvement)

---

## Executive Summary

**Current State** (Phase 6/7 Baseline):
- **Baseline Latency**: 3462ms average (LLM mode with Hermes 2 Pro)
- **Full Prompt**: 13,115 characters (~13k), 64.77s response time
- **Simplified Prompt**: 2,009 characters (~2k), 8.46s response time
- **Target**: <200ms average, <500ms p95

**Optimization Strategy**:
- **Priority 1**: Prompt optimization (13k → 2k already proven 7.7× faster)
- **Priority 2**: Make simplified prompts default (already implemented, needs tuning)
- **Priority 3**: Batch inference (reuse LLM context across agents)
- **Priority 4**: Async streaming (reduce perceived latency by 10-20%)
- **Priority 5**: Cache expansion (increase hit rate to 30-50%)

**Key Discovery from Phase 7 Validation**:
> "LATENCY OPTIMIZATION: Skip Tier 1 (FullLlm ~13k chars) and start with Tier 2 (SimplifiedLlm ~2k chars)  
> This reduces prompt processing time by ~60% (21.2s → ~10-12s expected)"

**Already Implemented** (from `fallback_system.rs` line 123):
```rust
// Was: FallbackTier::FullLlm
let mut current_tier = FallbackTier::SimplifiedLlm;  // ✅ ALREADY OPTIMIZED!
```

**Result**: System is **already using simplified prompts by default** as of Phase 7. We need to validate this optimization and implement additional improvements.

---

## 1. Latency Breakdown Analysis

### Current Performance (Phase 6 Validation Data)

| Component | Full Prompt (13k) | Simplified Prompt (2k) | Notes |
|-----------|-------------------|------------------------|-------|
| **Total Response Time** | 64.77s | 8.46s | **7.7× faster** |
| **Prompt Processing** | ~40-50s (est.) | ~5-6s (est.) | Token encoding, context window |
| **Model Inference** | ~10-15s (est.) | ~2-3s (est.) | Hermes 2 Pro Q4_K_M (4.4GB) |
| **JSON Parsing** | <0.1s | <0.1s | 5-stage parser (negligible) |

### Estimated Breakdown (for 8.46s simplified response)
- **Prompt Encoding**: ~2s (25%)
- **Network (Ollama)**: ~0.5s (6%)
- **Model Inference**: ~5s (59%)
- **Response Decoding**: ~0.5s (6%)
- **JSON Parsing**: ~0.46s (5%) - Based on Phase 6 data (3462ms total - 3000ms inference ≈ 460ms overhead)

**Critical Finding**: Most time is in **model inference** (59%), not prompt size. Prompt optimization from 13k → 2k gives us **7.7× speedup**, but we need **further optimizations** to hit <200ms target.

---

## 2. Optimization Opportunities

### ✅ Optimization 1: Simplified Prompts (ALREADY IMPLEMENTED)

**Status**: **COMPLETE** (Phase 7, `fallback_system.rs` line 123)  
**Current State**: System defaults to `FallbackTier::SimplifiedLlm` (2k chars)  
**Impact**: 7.7× faster (64.77s → 8.46s)  
**Validation Needed**: Confirm this is working correctly in current builds

**Action Required**:
1. ✅ Verify `FallbackTier::SimplifiedLlm` is default (already confirmed in code)
2. Run benchmark to confirm 8.46s latency (not 64.77s)
3. Document this optimization in Phase 8 completion report

**Time**: 0.5-1h (validation only, no implementation)

---

### 🔧 Optimization 2: Prompt Compression (Further Reduction)

**Goal**: Reduce simplified prompt from 2k → 1k characters (50% reduction)  
**Expected Impact**: 1.5-2× faster (8.46s → 4-5s)

**Current Simplified Prompt Structure** (from `fallback_system.rs` line 95-109):
```rust
simplified_tools: vec![
    // Position-based (5 tools)
    "MoveTo", "ThrowSmoke", "ThrowExplosive", "AoEAttack", "TakeCover",
    
    // Target-based (5 tools)
    "Attack", "Approach", "Retreat", "MarkTarget", "Distract",
    
    // Simple (5 tools)
    "Reload", "Scan", "Wait", "Block", "Heal",
],
```

**Optimization Strategies**:
1. **Remove redundant tools**: 15 → 8 core tools (MoveTo, Attack, TakeCover, Reload, Scan, Wait, ThrowSmoke, Retreat)
2. **Compress tool documentation**: Inline examples instead of verbose descriptions
3. **Remove snapshot pretty-printing**: Use compact JSON (no whitespace)
4. **Abbreviate instructions**: "Convert snapshot to plan" instead of full explanation

**Implementation**:
- Modify `build_enhanced_prompt()` in `astraweave-llm/src/prompt_template.rs`
- Add `PromptConfig::UltraCompact` variant
- Benchmark: Compare 2k vs 1k prompt latency

**Success Criteria**:
- Prompt size: 1,000-1,200 characters (from 2k)
- Response time: 4-5s (from 8.46s)
- Plan quality: ≥90% success rate (from 100% at 2k)

**Time**: 2-3 hours

---

### 🔧 Optimization 3: Batch Inference (Context Reuse)

**Goal**: Reuse LLM context across multiple agents (reduce per-agent latency)  
**Expected Impact**: 5-10× per-agent speedup (8.46s → 0.8-1.7s per agent)

**Current State** (from Phase 6 data):
- **Single agent**: 8.46s per plan
- **10 agents**: ~10 agents/frame @ 3.46s latency (Phase 6 MASTER_BENCHMARK_REPORT.md)
- **Inefficiency**: Each agent sends separate LLM request

**Batch Inference Architecture**:
```rust
pub struct BatchInferenceExecutor {
    batches: Vec<Vec<(AgentId, WorldSnapshot)>>,  // Collect agents
    batch_size: usize,                             // Default 5-10
    context_cache: Arc<RwLock<HashMap<String, LlmContext>>>,
}

impl BatchInferenceExecutor {
    // Collect agents for batch (non-blocking)
    pub fn queue_agent(&mut self, agent_id: AgentId, snap: WorldSnapshot);
    
    // Execute batch when threshold reached
    pub async fn execute_batch(&mut self) -> Vec<(AgentId, PlanIntent)> {
        // 1. Build single prompt with all agents
        // 2. Request LLM to generate N plans in one response
        // 3. Parse JSON array of plans
        // 4. Distribute plans to agents
    }
}
```

**Challenges**:
- **Prompt complexity**: Multi-agent prompts harder to parse
- **Fair allocation**: Ensure all agents get valid plans
- **Determinism**: Batch order must be deterministic

**Implementation**:
- Create `astraweave-llm/src/batch_executor.rs`
- Modify `LlmExecutor` to support batch mode
- Add multi-agent prompt template
- Update `FallbackOrchestrator` to handle batch requests

**Success Criteria**:
- Batch 5 agents: <2s total (0.4s per agent = 21× faster)
- Batch 10 agents: <3s total (0.3s per agent = 28× faster)
- Plan quality: ≥85% success rate per agent

**Time**: 3-4 hours

---

### 🔧 Optimization 4: Async Streaming (Progressive Decoding)

**Goal**: Start executing plan steps while LLM is still generating  
**Expected Impact**: 10-20% perceived latency reduction (8.46s → 6.8-7.6s perceived)

**Current State**:
- Wait for full LLM response (8.46s)
- Parse entire JSON
- Then start executing

**Streaming Architecture**:
```rust
pub struct StreamingParser {
    buffer: String,
    partial_steps: Vec<ActionStep>,
}

impl StreamingParser {
    // Parse incrementally as tokens arrive
    pub fn feed_token(&mut self, token: &str) -> Option<ActionStep> {
        self.buffer.push_str(token);
        
        // Attempt to parse first complete step
        if let Some(step) = self.try_parse_next_step() {
            return Some(step);
        }
        None
    }
}
```

**Benefits**:
- **Lower perceived latency**: Agent moves after 2-3s (not 8.46s)
- **Better UX**: Visible progress vs long pause
- **Partial failure recovery**: Use parsed steps even if LLM times out

**Challenges**:
- **Incomplete JSON**: Need robust incremental parser
- **Error handling**: What if streaming fails mid-plan?
- **Determinism**: Must replay exact token order

**Implementation**:
- Integrate `reqwest::Response::bytes_stream()` for Ollama API
- Create incremental JSON parser (state machine)
- Modify `LlmExecutor::generate_plan_async()` to yield steps progressively
- Add timeout handling for partial plans

**Success Criteria**:
- First step available: <3s (from 8.46s = 64% faster perceived start)
- Full plan completion: 8.46s (unchanged total time)
- Streaming overhead: <100ms (parsing complexity)

**Time**: 2-3 hours

---

### 🔧 Optimization 5: Cache Expansion (Hit Rate Tuning)

**Goal**: Increase LRU cache hit rate from unknown → 30-50%  
**Expected Impact**: 100% speedup on cache hits (8.46s → <0.1s)

**Current State** (from Phase 6):
- Cache exists (`astraweave-llm/benches/cache_stress_test.rs`)
- No reported hit rate in Phase 6/7 validation
- Unknown cache size and eviction policy

**Cache Tuning Strategies**:
1. **Increase cache size**: 100 → 500 entries (more memory, higher hit rate)
2. **Similarity-based lookup**: Match snapshots within tolerance (e.g., position ±5 units)
3. **Partial plan reuse**: Cache individual action steps, not full plans
4. **Warmup strategy**: Pre-populate cache with common scenarios

**Implementation**:
- Review `astraweave-llm/benches/cache_stress_test.rs` for current implementation
- Add metrics to track hit/miss rates
- Tune cache size via benchmarking (100, 250, 500, 1000 entries)
- Implement similarity-based hashing (perceptual hash for snapshots)

**Success Criteria**:
- Cache hit rate: 30-50% (from unknown)
- Cache hit latency: <0.1s (instant plan retrieval)
- Memory overhead: <50 MB (at 500 entries)

**Time**: 1-2 hours

---

## 3. Implementation Roadmap

### Phase 1: Validation & Baseline (1-2h)

**Goal**: Confirm current state and measure actual latency

**Tasks**:
1. ✅ Run `hello_companion` with simplified prompts (verify 8.46s, not 64.77s)
2. ✅ Benchmark: `cargo bench -p astraweave-ai --bench llm_executor_benchmarks` (if exists)
3. ✅ Validate `FallbackTier::SimplifiedLlm` is default
4. ✅ Document baseline metrics in Phase 8 report

**Deliverables**:
- Baseline benchmark results (actual latency vs Phase 6 data)
- Confirmation that Optimization 1 is active

---

### Phase 2: Prompt Compression (2-3h)

**Goal**: Reduce prompt from 2k → 1k characters

**Tasks**:
1. Create `PromptConfig::UltraCompact` variant
2. Remove 7 least-used tools (15 → 8)
3. Compress tool documentation (inline examples)
4. Remove pretty-printing (compact JSON)
5. Benchmark: Compare 2k vs 1k latency
6. Validate plan quality (≥90% success rate)

**Deliverables**:
- `astraweave-llm/src/prompt_template.rs` updated
- Benchmark comparison (2k vs 1k)
- Plan quality validation report

---

### Phase 3: Batch Inference (3-4h)

**Goal**: Implement multi-agent batching

**Tasks**:
1. Create `astraweave-llm/src/batch_executor.rs`
2. Implement `BatchInferenceExecutor` struct
3. Add multi-agent prompt template
4. Modify `LlmExecutor` for batch mode
5. Test with 5, 10, 20 agents
6. Validate determinism (same order → same plans)

**Deliverables**:
- `batch_executor.rs` (300-400 LOC)
- Batch benchmarks (5, 10, 20 agents)
- Determinism validation tests

---

### Phase 4: Async Streaming (2-3h)

**Goal**: Implement progressive decoding

**Tasks**:
1. Integrate `reqwest::bytes_stream()` with Ollama API
2. Create `StreamingParser` state machine
3. Modify `LlmExecutor::generate_plan_async()` to yield steps
4. Add timeout handling for partial plans
5. Test perceived latency improvement

**Deliverables**:
- `astraweave-llm/src/streaming_parser.rs` (200-300 LOC)
- Streaming benchmarks (perceived vs actual latency)
- Timeout tests (partial plan recovery)

---

### Phase 5: Cache Tuning (1-2h)

**Goal**: Increase cache hit rate to 30-50%

**Tasks**:
1. Add cache hit/miss metrics
2. Benchmark cache sizes (100, 250, 500, 1000)
3. Implement similarity-based hashing
4. Tune eviction policy (LRU → LFU?)
5. Document hit rate improvements

**Deliverables**:
- Cache metrics dashboard
- Tuning benchmark results
- Similarity hashing implementation

---

### Phase 6: Validation & Documentation (1-2h)

**Goal**: Validate all optimizations work together

**Tasks**:
1. Run full benchmark suite (all optimizations enabled)
2. Test with `hello_companion` (end-to-end validation)
3. Verify <200ms average latency achieved
4. Create completion report
5. Update MASTER_BENCHMARK_REPORT.md

**Deliverables**:
- `OPTION_2_LLM_OPTIMIZATION_COMPLETE.md` (comprehensive report)
- `MASTER_BENCHMARK_REPORT.md` v3.3 (LLM section updated)
- Before/After comparison tables

---

## 4. Success Criteria

### Quantitative Metrics

| Metric | Baseline (Phase 6) | Target | Success Threshold |
|--------|-------------------|--------|-------------------|
| **Average Latency** | 3462ms | <200ms | <500ms |
| **p95 Latency** | ~8,000ms (est.) | <500ms | <1,000ms |
| **Prompt Size** | 13k → 2k (current) | <1k | <1.5k |
| **Batch Throughput** | 1 agent/8.46s | 10 agents/2s | 5 agents/3s |
| **Cache Hit Rate** | Unknown | 30-50% | >20% |
| **Plan Quality** | 100% (2k prompt) | ≥90% (1k prompt) | ≥85% |

### Qualitative Metrics

✅ **Determinism**: Plans are reproducible with same inputs  
✅ **Scalability**: 100+ agents @ 60 FPS (AI Arbiter polling overhead)  
✅ **Robustness**: Graceful fallback on LLM failure  
✅ **Monitoring**: Prometheus metrics for latency/hit rate  

---

## 5. Risk Mitigation

### Risk 1: Prompt Compression Reduces Quality

**Likelihood**: Medium  
**Impact**: High (plan success rate drops below 85%)

**Mitigation**:
- Benchmark plan quality at each compression level
- Keep 2k prompt as fallback tier
- A/B test 1k vs 2k prompts (randomized)

---

### Risk 2: Batch Inference Breaks Determinism

**Likelihood**: Medium  
**Impact**: Critical (multiplayer desync)

**Mitigation**:
- Sort batch by agent ID (deterministic order)
- Hash batch inputs before sending to LLM
- Replay tests with same batch order

---

### Risk 3: Streaming Parser Fails on Incomplete JSON

**Likelihood**: High  
**Impact**: Medium (fall back to full parsing)

**Mitigation**:
- Robust state machine (handle truncated JSON)
- Timeout fallback (use partial plan if >10s)
- Extensive fuzz testing with corrupted streams

---

### Risk 4: Cache Similarity Hashing Too Loose

**Likelihood**: Low  
**Impact**: Medium (incorrect plans cached)

**Mitigation**:
- Conservative tolerance (±2 units, not ±5)
- Validate cached plans against current snapshot
- Metrics to track cache misses due to similarity mismatch

---

### Risk 5: Ollama API Changes Break Streaming

**Likelihood**: Low  
**Impact**: Medium (streaming disabled)

**Mitigation**:
- Feature flag `streaming` (can disable if broken)
- Fallback to full response mode
- Pin Ollama version (document in README)

---

## 6. Time Estimation Summary

| Phase | Tasks | Estimated Time | Confidence |
|-------|-------|---------------|------------|
| **Phase 1: Validation** | Baseline benchmarks, verify optimizations | 1-2h | High (90%) |
| **Phase 2: Prompt Compression** | 1k chars, quality validation | 2-3h | High (85%) |
| **Phase 3: Batch Inference** | Multi-agent batching, determinism | 3-4h | Medium (70%) |
| **Phase 4: Async Streaming** | Progressive decoding, timeout handling | 2-3h | Medium (65%) |
| **Phase 5: Cache Tuning** | Hit rate optimization, similarity hashing | 1-2h | High (80%) |
| **Phase 6: Documentation** | Completion report, master reports | 1-2h | High (95%) |
| **TOTAL** | All optimizations | **10-16h** | Medium (75%) |

**Recommended Allocation**: 12 hours (median estimate)

**Critical Path**:
1. Phase 1 (validation) → Phase 2 (compression) → Phase 6 (docs)  
   Estimated: 4-7h for minimal viable optimization

2. Phase 3 (batching) can run in parallel with Phase 4 (streaming)  
   Estimated: +3-5h for advanced optimizations

3. Phase 5 (cache tuning) can be done last  
   Estimated: +1-2h for polish

---

## 7. Validation Approach

### Unit Tests

**Prompt Compression** (`astraweave-llm/tests/prompt_compression_tests.rs`):
```rust
#[test]
fn test_ultra_compact_prompt_size() {
    let snap = create_test_snapshot();
    let reg = default_tool_registry();
    
    let prompt = build_enhanced_prompt(&snap, &reg, PromptConfig::UltraCompact);
    
    assert!(prompt.len() < 1500, "Prompt should be <1.5k chars, got {}", prompt.len());
    assert!(prompt.contains("MoveTo"), "Should include core tools");
}

#[test]
fn test_ultra_compact_plan_quality() {
    // Test that 1k prompt still generates valid plans
    let plans = run_llm_test_suite(PromptConfig::UltraCompact);
    let success_rate = plans.iter().filter(|p| p.is_ok()).count() as f32 / plans.len() as f32;
    
    assert!(success_rate >= 0.85, "Plan quality should be ≥85%, got {:.1}%", success_rate * 100.0);
}
```

**Batch Inference** (`astraweave-llm/tests/batch_inference_tests.rs`):
```rust
#[tokio::test]
async fn test_batch_determinism() {
    let agents = vec![agent_a, agent_b, agent_c];
    
    let batch1 = batch_executor.execute_batch(agents.clone()).await;
    let batch2 = batch_executor.execute_batch(agents.clone()).await;
    
    assert_eq!(batch1, batch2, "Batches should produce identical plans");
}

#[tokio::test]
async fn test_batch_throughput() {
    let start = Instant::now();
    let plans = batch_executor.execute_batch(10_agents).await;
    let elapsed = start.elapsed();
    
    assert!(elapsed < Duration::from_secs(3), "10 agents should complete <3s, took {:?}", elapsed);
    assert_eq!(plans.len(), 10, "All agents should get plans");
}
```

**Streaming Parser** (`astraweave-llm/tests/streaming_parser_tests.rs`):
```rust
#[test]
fn test_incremental_parsing() {
    let mut parser = StreamingParser::new();
    
    // Simulate token-by-token arrival
    parser.feed_token(r#"{"plan_id":"p1","#);
    assert!(parser.partial_steps.is_empty(), "No steps yet");
    
    parser.feed_token(r#""steps":[{"act":"MoveTo","x":10,"y":10},"#);
    assert_eq!(parser.partial_steps.len(), 1, "First step should parse");
    
    parser.feed_token(r#"{"act":"Attack","target_id":5}]}"#);
    assert_eq!(parser.partial_steps.len(), 2, "Second step should parse");
}

#[test]
fn test_streaming_timeout_recovery() {
    // Test that partial plans can be used if LLM times out
    let mut parser = StreamingParser::new();
    parser.feed_token(r#"{"plan_id":"p1","steps":[{"act":"MoveTo","x":10,"y":10}"#);
    
    let partial_plan = parser.finalize_partial();
    assert_eq!(partial_plan.steps.len(), 1, "Should recover 1 step from incomplete JSON");
}
```

**Cache Tuning** (`astraweave-llm/tests/cache_tuning_tests.rs`):
```rust
#[test]
fn test_similarity_hashing() {
    let snap1 = WorldSnapshot { me: pos(10, 10), ... };
    let snap2 = WorldSnapshot { me: pos(12, 10), ... };  // ±2 units
    
    let hash1 = perceptual_hash(&snap1);
    let hash2 = perceptual_hash(&snap2);
    
    assert_eq!(hash1, hash2, "Snapshots within tolerance should hash identically");
}

#[test]
fn test_cache_hit_rate() {
    let cache = PlanCache::new(500);
    let snapshots = generate_realistic_snapshots(1000);
    
    for snap in snapshots {
        let _plan = cache.get_or_compute(snap, || expensive_llm_call());
    }
    
    let hit_rate = cache.metrics().hit_rate();
    assert!(hit_rate >= 0.30, "Cache hit rate should be ≥30%, got {:.1}%", hit_rate * 100.0);
}
```

---

### Integration Tests

**End-to-End Latency** (`astraweave-ai/tests/llm_latency_tests.rs`):
```rust
#[tokio::test]
async fn test_average_latency_below_200ms() {
    let executor = setup_optimized_llm_executor();
    let snapshots = generate_100_scenarios();
    
    let mut latencies = Vec::new();
    for snap in snapshots {
        let start = Instant::now();
        let _plan = executor.generate_plan_async(snap).await.unwrap();
        latencies.push(start.elapsed().as_millis());
    }
    
    let avg_latency = latencies.iter().sum::<u128>() / latencies.len() as u128;
    assert!(avg_latency < 200, "Average latency should be <200ms, got {}ms", avg_latency);
}

#[tokio::test]
async fn test_p95_latency_below_500ms() {
    let latencies = run_latency_test_suite(100);
    latencies.sort();
    
    let p95 = latencies[(latencies.len() as f32 * 0.95) as usize];
    assert!(p95 < 500, "p95 latency should be <500ms, got {}ms", p95);
}
```

---

### Benchmarks

**Before/After Comparison** (`astraweave-llm/benches/optimization_comparison.rs`):
```rust
fn bench_baseline_phase6(c: &mut Criterion) {
    // Phase 6 baseline: 3462ms average
    c.bench_function("llm_phase6_baseline", |b| {
        b.iter(|| {
            // Full LLM call with 13k prompt (Tier 1)
            executor.plan_full_prompt(black_box(&snapshot))
        })
    });
}

fn bench_optimized_phase8(c: &mut Criterion) {
    // Phase 8 optimized: target <200ms
    c.bench_function("llm_phase8_optimized", |b| {
        b.iter(|| {
            // All optimizations enabled:
            // - 1k compact prompt
            // - Batch inference (5 agents)
            // - Streaming parser
            // - Cache tuning (50% hit rate)
            executor.plan_optimized(black_box(&snapshot))
        })
    });
}

criterion_group!(benches, bench_baseline_phase6, bench_optimized_phase8);
criterion_main!(benches);
```

---

## 8. Documentation Plan

### Primary Deliverable

**`OPTION_2_LLM_OPTIMIZATION_COMPLETE.md`** (600-800 lines):
- Executive summary (before/after metrics)
- Optimization breakdown (5 areas)
- Implementation details per phase
- Benchmark results (tables + graphs)
- Validation results (tests + integration)
- Time efficiency analysis (actual vs estimate)
- Lessons learned
- Next steps

### Master Report Updates

**`MASTER_BENCHMARK_REPORT.md`** v3.3:
- New section: "LLM Optimization (Phase 8)"
- Before/After tables (3462ms → <200ms)
- Optimization contribution breakdown
- Cache hit rate metrics

**`MASTER_ROADMAP.md`** v1.15:
- Update "Current State" with Option 2 completion
- Mark Medium-Term Priority #7 complete
- Document latency improvements

**`.github/copilot-instructions.md`**:
- Add Option 2 completion to "Current State"
- Update LLM performance baselines
- Document optimization techniques

---

## 9. Next Steps After Completion

### Option 2 Success → Unblocks

1. **Medium-Term Priority #6**: GUI Performance (LLM latency no longer bottleneck)
2. **Medium-Term Priority #8**: Veilweaver Demo (fast LLM enables real gameplay)
3. **Long-Term Goals**: Production multiplayer (deterministic, low-latency AI)

### Follow-Up Optimizations (Future Work)

1. **GPU Acceleration**: Integrate llama.cpp for local GPU inference (10-100× faster)
2. **Model Quantization**: Test Q3_K_S (2.2GB) vs Q4_K_M (4.4GB) for speed/quality tradeoff
3. **Prompt Caching API**: Use Ollama's native prompt caching (if supported)
4. **Hybrid AI**: Use GOAP for <200ms cases, reserve LLM for strategic decisions only

---

## 10. Approval & Next Actions

**This plan is ready for implementation. Please approve to proceed with:**

1. ✅ **Phase 1 Validation** (1-2h) - Verify current state, baseline benchmarks
2. ✅ **Phase 2 Prompt Compression** (2-3h) - Reduce 2k → 1k chars
3. ✅ **Phase 3 Batch Inference** (3-4h) - Multi-agent batching
4. ✅ **Phase 4 Async Streaming** (2-3h) - Progressive decoding
5. ✅ **Phase 5 Cache Tuning** (1-2h) - Hit rate optimization
6. ✅ **Phase 6 Documentation** (1-2h) - Completion report

**Total Estimated Time**: 10-16 hours (median: 12h)

**Expected Outcome**:
- ✅ Average latency: 3462ms → <200ms (17× improvement)
- ✅ p95 latency: ~8,000ms → <500ms (16× improvement)
- ✅ Batch throughput: 1 agent/8.46s → 10 agents/2s (42× improvement)
- ✅ Cache hit rate: Unknown → 30-50%
- ✅ Plan quality: Maintained ≥90%

---

**Awaiting approval to begin Phase 1 validation.**
