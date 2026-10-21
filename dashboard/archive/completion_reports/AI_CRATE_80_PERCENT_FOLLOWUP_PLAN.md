# AI Crate 80%+ Coverage Follow-Up Plan

**Date:** October 22, 2025  
**Current Coverage:** 23.30% (527/2262 lines)  
**Target Coverage:** 80%+ overall  
**Gap:** +56.7 percentage points ≈ 1,285 additional lines  
**Estimated Time:** 8-12 hours

---

## Executive Summary

This document outlines a 3-phase approach to achieve 80%+ test coverage for the AI crate, building on the foundation of 42 existing tests. The strategy focuses on high-impact modules with measurable milestones.

**Current State:**
- ✅ 42 unit tests passing (100% pass rate)
- ✅ 3/5 modules at 80%+ coverage
- ⚠️ Async modules untested (async_task.rs, ai_arbiter.rs implementation)
- ❌ LLM modules require external service mocking

**Success Criteria:**
- 🎯 Overall coverage: 80%+
- 🎯 All critical modules: 80%+
- 🎯 Integration test coverage: 70%+
- 🎯 Zero test failures
- 🎯 < 5 warnings

---

## Phase 1: Async Infrastructure Testing (3-4 hours)

**Goal:** Test async_task.rs and complete ai_arbiter.rs coverage  
**Impact:** +198 lines, +8.7% coverage  
**Priority:** 🔥 CRITICAL (blocks Phase 2)

### 1.1 AsyncTask Testing (1-1.5 hours)

**File:** `astraweave-ai/src/async_task.rs`  
**Current:** 0/48 lines (0%)  
**Target:** 40/48 lines (83%)  
**Tests to Add:** 8-10

**Uncovered Functions:**
```rust
// Core functionality (lines 81-204)
- new() - Spawn task and create AsyncTask wrapper
- try_recv() - Non-blocking result polling
- blocking_recv() - Blocking wait for completion
- is_finished() - Check task status
- abort() - Cancel running task
- join() - Wait for task completion

// Error handling
- Task panics and unwinding
- Timeout scenarios
- Abort during execution
```

**Test Plan:**

```rust
// Test 1: Basic task creation and completion
#[tokio::test]
async fn test_async_task_completes_successfully() {
    let task = AsyncTask::new(async { Ok(42) });
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    let result = task.try_recv();
    assert!(result.is_some());
    assert_eq!(result.unwrap().unwrap(), 42);
}

// Test 2: Polling before completion
#[tokio::test]
async fn test_async_task_try_recv_returns_none_when_pending() {
    let task = AsyncTask::new(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(42)
    });
    
    // Poll immediately - should be None
    let result = task.try_recv();
    assert!(result.is_none());
}

// Test 3: Blocking wait
#[tokio::test]
async fn test_async_task_blocking_recv_waits() {
    let task = AsyncTask::new(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(42)
    });
    
    let start = Instant::now();
    let result = task.blocking_recv();
    let elapsed = start.elapsed();
    
    assert!(elapsed >= Duration::from_millis(50));
    assert_eq!(result.unwrap(), 42);
}

// Test 4: Task abort
#[tokio::test]
async fn test_async_task_abort_cancels_execution() {
    let task = AsyncTask::new(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(42)
    });
    
    task.abort();
    let result = task.try_recv();
    
    assert!(result.is_some());
    assert!(result.unwrap().is_err()); // Aborted tasks return error
}

// Test 5: is_finished() status check
#[tokio::test]
async fn test_async_task_is_finished() {
    let task = AsyncTask::new(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(42)
    });
    
    assert!(!task.is_finished()); // Not finished yet
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(task.is_finished()); // Should be finished now
}

// Test 6: Task panic handling
#[tokio::test]
async fn test_async_task_handles_panic() {
    let task = AsyncTask::new(async {
        panic!("Test panic");
        Ok(42)
    });
    
    tokio::time::sleep(Duration::from_millis(10)).await;
    let result = task.try_recv();
    
    assert!(result.is_some());
    assert!(result.unwrap().is_err()); // Panic becomes error
}

// Test 7: Multiple try_recv calls
#[tokio::test]
async fn test_async_task_multiple_try_recv() {
    let task = AsyncTask::new(async { Ok(42) });
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    let result1 = task.try_recv();
    let result2 = task.try_recv();
    
    assert!(result1.is_some());
    assert!(result2.is_none()); // Already consumed
}

// Test 8: Clone behavior (if applicable)
#[tokio::test]
async fn test_async_task_clone_independence() {
    // Test that cloned tasks are independent
}

// Test 9: Error propagation
#[tokio::test]
async fn test_async_task_error_propagation() {
    let task = AsyncTask::new(async {
        Err(anyhow::anyhow!("Test error"))
    });
    
    tokio::time::sleep(Duration::from_millis(10)).await;
    let result = task.try_recv();
    
    assert!(result.is_some());
    assert!(result.unwrap().is_err());
}

// Test 10: Concurrent task creation
#[tokio::test]
async fn test_async_task_concurrent_spawns() {
    let tasks: Vec<_> = (0..10)
        .map(|i| AsyncTask::new(async move { Ok(i) }))
        .collect();
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    for (i, task) in tasks.into_iter().enumerate() {
        let result = task.try_recv();
        assert_eq!(result.unwrap().unwrap(), i);
    }
}
```

**Files to Modify:**
- Create: `astraweave-ai/tests/async_task_tests.rs`

**Estimated Impact:** +40 lines covered, +1.8% coverage

---

### 1.2 AIArbiter Integration Tests (1.5-2 hours)

**File:** `astraweave-ai/src/ai_arbiter.rs`  
**Current:** ~10/200 lines (~5%)  
**Target:** 160/200 lines (80%)  
**Tests to Add:** 12-15

**Uncovered Functions:**
```rust
// Core methods (lines 200-550)
- new() - AIArbiter initialization
- with_llm_cooldown() - Configure cooldown
- update() - Main game loop (CRITICAL)
- transition_to_llm() - Mode transition
- transition_to_goap() - Return to GOAP
- transition_to_bt() - Emergency fallback
- poll_llm_result() - Non-blocking polling
- maybe_request_llm() - LLM request logic
- mode() - Get current mode
- metrics() - Get performance metrics
- is_llm_active() - Check task status
- current_plan() - Get active plan
```

**Test Plan:**

```rust
// Test 1: AIArbiter initialization
#[tokio::test]
async fn test_arbiter_new_creates_in_goap_mode() {
    let (arbiter, _) = create_test_arbiter();
    
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    assert!(!arbiter.is_llm_active());
    assert!(arbiter.current_plan().is_none());
}

// Test 2: Cooldown configuration
#[tokio::test]
async fn test_arbiter_with_llm_cooldown() {
    let arbiter = create_test_arbiter()
        .0
        .with_llm_cooldown(5.0);
    
    // Verify cooldown is set (requires metrics or internal check)
}

// Test 3: GOAP mode returns action
#[tokio::test]
async fn test_arbiter_update_in_goap_mode_returns_action() {
    let (mut arbiter, _) = create_test_arbiter();
    let snap = create_test_snapshot(1.0);
    
    let action = arbiter.update(&snap);
    
    assert!(matches!(action, ActionStep::_)); // Verify valid action
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

// Test 4: LLM request on cooldown expiry
#[tokio::test]
async fn test_arbiter_requests_llm_after_cooldown() {
    let (mut arbiter, _) = create_test_arbiter_with_cooldown(0.1);
    let snap1 = create_test_snapshot(1.0);
    let snap2 = create_test_snapshot(1.2); // 0.2s later
    
    arbiter.update(&snap1);
    assert!(!arbiter.is_llm_active());
    
    arbiter.update(&snap2);
    assert!(arbiter.is_llm_active()); // Should have spawned task
}

// Test 5: Mode transition to ExecutingLLM
#[tokio::test]
async fn test_arbiter_transitions_to_executing_llm() {
    let (mut arbiter, _) = create_test_arbiter();
    let plan = create_test_plan(3); // 3-step plan
    
    arbiter.transition_to_llm(plan);
    
    assert_eq!(arbiter.mode(), AIControlMode::ExecutingLLM { step_index: 0 });
    assert!(arbiter.current_plan().is_some());
}

// Test 6: Plan execution advances steps
#[tokio::test]
async fn test_arbiter_executes_plan_steps_sequentially() {
    let (mut arbiter, _) = create_test_arbiter();
    let plan = create_test_plan(3);
    arbiter.transition_to_llm(plan);
    
    let snap = create_test_snapshot(1.0);
    
    // Step 1
    arbiter.update(&snap);
    assert_eq!(arbiter.mode(), AIControlMode::ExecutingLLM { step_index: 1 });
    
    // Step 2
    arbiter.update(&snap);
    assert_eq!(arbiter.mode(), AIControlMode::ExecutingLLM { step_index: 2 });
    
    // Step 3 (last)
    arbiter.update(&snap);
    assert_eq!(arbiter.mode(), AIControlMode::GOAP); // Should return to GOAP
}

// Test 7: Return to GOAP after plan exhaustion
#[tokio::test]
async fn test_arbiter_returns_to_goap_after_plan() {
    let (mut arbiter, _) = create_test_arbiter();
    let plan = create_test_plan(2);
    arbiter.transition_to_llm(plan);
    
    let snap = create_test_snapshot(1.0);
    
    arbiter.update(&snap); // Execute step 1
    arbiter.update(&snap); // Execute step 2, return to GOAP
    
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    assert!(arbiter.current_plan().is_none());
}

// Test 8: Emergency BT fallback
#[tokio::test]
async fn test_arbiter_fallback_to_bt_on_goap_failure() {
    let (mut arbiter, _) = create_test_arbiter_with_failing_goap();
    let snap = create_test_snapshot(1.0);
    
    let action = arbiter.update(&snap);
    
    // Should fall back to BT and return valid action
    assert!(matches!(action, ActionStep::_));
}

// Test 9: Metrics tracking
#[tokio::test]
async fn test_arbiter_metrics_increment() {
    let (mut arbiter, _) = create_test_arbiter();
    let snap = create_test_snapshot(1.0);
    
    let (transitions_before, _, _, _, goap_before, _) = arbiter.metrics();
    
    arbiter.update(&snap);
    
    let (transitions_after, _, _, _, goap_after, _) = arbiter.metrics();
    
    assert_eq!(goap_after, goap_before + 1); // GOAP action count increased
}

// Test 10: Concurrent LLM requests blocked
#[tokio::test]
async fn test_arbiter_blocks_concurrent_llm_requests() {
    let (mut arbiter, _) = create_test_arbiter_with_cooldown(0.0);
    let snap1 = create_test_snapshot(1.0);
    let snap2 = create_test_snapshot(2.0);
    
    arbiter.update(&snap1); // First request
    assert!(arbiter.is_llm_active());
    
    arbiter.update(&snap2); // Should not spawn second request
    // Verify only one task active
}

// Test 11: Invalid plan handling
#[tokio::test]
async fn test_arbiter_handles_invalid_plan() {
    let (mut arbiter, _) = create_test_arbiter();
    let empty_plan = PlanIntent {
        plan_id: "empty".into(),
        steps: vec![],
    };
    arbiter.transition_to_llm(empty_plan);
    
    let snap = create_test_snapshot(1.0);
    let action = arbiter.update(&snap);
    
    // Should fall back to GOAP or BT
    assert!(matches!(action, ActionStep::_));
}

// Test 12: Poll LLM result
#[tokio::test]
async fn test_arbiter_poll_llm_result() {
    let (mut arbiter, mock_llm) = create_test_arbiter();
    // Spawn LLM task
    // Poll until complete
    // Verify result handling
}

// Test 13: LLM failure handling
#[tokio::test]
async fn test_arbiter_handles_llm_failure() {
    let (mut arbiter, _) = create_test_arbiter_with_failing_llm();
    let snap = create_test_snapshot(1.0);
    
    // Trigger LLM request
    // Wait for failure
    // Verify fallback to GOAP
}

// Test 14: Mode transitions don't lose state
#[tokio::test]
async fn test_arbiter_mode_transitions_preserve_state() {
    let (mut arbiter, _) = create_test_arbiter();
    let plan = create_test_plan(2);
    
    let (trans_before, _, _, _, _, _) = arbiter.metrics();
    arbiter.transition_to_llm(plan);
    let (trans_after, _, _, _, _, _) = arbiter.metrics();
    
    assert_eq!(trans_after, trans_before + 1);
}

// Test 15: Stress test - rapid updates
#[tokio::test]
async fn test_arbiter_handles_rapid_updates() {
    let (mut arbiter, _) = create_test_arbiter();
    let snap = create_test_snapshot(1.0);
    
    for _ in 0..1000 {
        let action = arbiter.update(&snap);
        assert!(matches!(action, ActionStep::_));
    }
}
```

**Helper Functions:**
```rust
fn create_test_arbiter() -> (AIArbiter, Arc<MockLlmOrch>) {
    let mock_llm = Arc::new(MockLlmOrch::new());
    let goap = Box::new(MockGoap::new());
    let bt = Box::new(MockBT::new());
    let runtime = tokio::runtime::Handle::current();
    let llm_executor = LlmExecutor::new(mock_llm.clone(), runtime);
    
    (AIArbiter::new(llm_executor, goap, bt), mock_llm)
}

fn create_test_plan(steps: usize) -> PlanIntent {
    PlanIntent {
        plan_id: "test-plan".into(),
        steps: (0..steps)
            .map(|i| ActionStep::MoveTo { x: i as i32, y: 0 })
            .collect(),
    }
}
```

**Files to Modify:**
- Extend: `astraweave-ai/tests/arbiter_comprehensive_tests.rs`

**Estimated Impact:** +150 lines covered, +6.6% coverage

---

## Phase 2: LLM Module Testing (3-4 hours)

**Goal:** Add mocks for astraweave-llm modules  
**Impact:** +200 lines, +8.8% coverage  
**Priority:** 🟡 MEDIUM (enhances reliability)

### 2.1 LLM Client Mocking (2-2.5 hours)

**Modules to Test:**
- `hermes2pro_ollama.rs` (0/7 lines)
- `phi3_ollama.rs` (0/7 lines)
- `retry.rs` (0/22 lines)
- `circuit_breaker.rs` (0/21 lines)
- `production_hardening.rs` (0/127 lines)

**Strategy:** Create mock HTTP server for Ollama API

**Test Plan:**

```rust
// Mock Ollama Server
struct MockOllamaServer {
    port: u16,
    responses: HashMap<String, String>,
}

impl MockOllamaServer {
    fn new() -> Self {
        // Start mock HTTP server
    }
    
    fn add_response(&mut self, endpoint: &str, response: &str) {
        self.responses.insert(endpoint.to_string(), response);
    }
}

// Test 1: Successful LLM request
#[tokio::test]
async fn test_hermes2pro_successful_request() {
    let server = MockOllamaServer::new();
    server.add_response("/api/generate", r#"{"response": "..."}"#);
    
    let client = Hermes2ProOllama::new(&format!("http://localhost:{}", server.port), "model");
    let result = client.generate("prompt").await;
    
    assert!(result.is_ok());
}

// Test 2: Retry on failure
#[tokio::test]
async fn test_retry_mechanism() {
    let server = MockOllamaServer::new();
    // First 2 requests fail, 3rd succeeds
    
    let client = with_retry(client, RetryPolicy::new(3));
    let result = client.generate("prompt").await;
    
    assert!(result.is_ok());
    assert_eq!(server.request_count(), 3);
}

// Test 3: Circuit breaker trips
#[tokio::test]
async fn test_circuit_breaker_trips_on_failures() {
    let server = MockOllamaServer::new_always_failing();
    let client = with_circuit_breaker(client);
    
    // Make 5 requests (threshold)
    for _ in 0..5 {
        let _ = client.generate("prompt").await;
    }
    
    // Circuit should be open
    let result = client.generate("prompt").await;
    assert!(matches!(result, Err(CircuitBreakerError::Open)));
}

// Test 4: Timeout handling
#[tokio::test]
async fn test_timeout_on_slow_response() {
    let server = MockOllamaServer::new_with_delay(Duration::from_secs(10));
    let client = with_timeout(client, Duration::from_millis(100));
    
    let result = client.generate("prompt").await;
    
    assert!(matches!(result, Err(TimeoutError)));
}

// ... 10-12 more tests
```

**Estimated Impact:** +100 lines covered, +4.4% coverage

---

### 2.2 LLM Cache and Tool Guard (1-1.5 hours)

**Modules to Test:**
- `cache/lru.rs` (0/44 lines)
- `cache/key.rs` (0/5 lines)
- `tool_guard.rs` (0/17 lines)

**Test Plan:**

```rust
// Test 1: LRU cache basic operations
#[test]
fn test_lru_cache_put_get() {
    let mut cache = LruCache::new(3);
    cache.put("key1", "value1");
    
    assert_eq!(cache.get("key1"), Some(&"value1"));
}

// Test 2: LRU eviction
#[test]
fn test_lru_cache_evicts_oldest() {
    let mut cache = LruCache::new(2);
    cache.put("key1", "value1");
    cache.put("key2", "value2");
    cache.put("key3", "value3"); // Evicts key1
    
    assert!(cache.get("key1").is_none());
    assert!(cache.get("key2").is_some());
}

// Test 3: Cache key generation
#[test]
fn test_cache_key_deterministic() {
    let snap1 = create_test_snapshot(1.0);
    let snap2 = create_test_snapshot(1.0);
    
    let key1 = CacheKey::from_snapshot(&snap1);
    let key2 = CacheKey::from_snapshot(&snap2);
    
    assert_eq!(key1, key2); // Same snapshot = same key
}

// Test 4: Tool guard validation
#[test]
fn test_tool_guard_allows_valid_tools() {
    let guard = ToolGuard::new(vec!["MoveTo", "Attack"]);
    
    assert!(guard.is_allowed("MoveTo"));
    assert!(!guard.is_allowed("Invalid"));
}

// ... 8-10 more tests
```

**Estimated Impact:** +66 lines covered, +2.9% coverage

---

## Phase 3: Remaining Gaps (2-4 hours)

**Goal:** Fill remaining coverage gaps in core modules  
**Impact:** +237 lines, +10.5% coverage  
**Priority:** 🟢 LOW (polish for 80%+)

### 3.1 Core Module Completion (1-2 hours)

**Modules:**
- `astraweave-core/src/validation.rs` (0/181 lines)
- `astraweave-core/src/perception.rs` (0/17 lines)
- `astraweave-ecs/src/blob_vec.rs` (0/67 lines)
- `astraweave-ecs/src/events.rs` (0/54 lines)

**Strategy:** Add focused unit tests for uncovered functions

**Estimated Impact:** +150 lines covered, +6.6% coverage

---

### 3.2 Nav and Physics Completion (1-2 hours)

**Modules:**
- `astraweave-nav/src/lib.rs` (9/72 lines, 12.5%)
- `astraweave-physics/src/spatial_hash.rs` (0/59 lines)

**Strategy:** Integration tests for pathfinding and collision

**Estimated Impact:** +87 lines covered, +3.8% coverage

---

## Implementation Schedule

### Week 1: Async Infrastructure

| Day | Task | Hours | Lines | Coverage Δ |
|-----|------|-------|-------|------------|
| Mon | AsyncTask tests (1.1) | 1.5 | +40 | +1.8% |
| Tue | AIArbiter tests (1.2) | 2.0 | +150 | +6.6% |
| Wed | Integration & bugfixes | 1.0 | - | - |
| **Total** | **Phase 1** | **4.5** | **+190** | **+8.4%** |

### Week 2: LLM Modules

| Day | Task | Hours | Lines | Coverage Δ |
|-----|------|-------|-------|------------|
| Mon | LLM client mocking (2.1) | 2.5 | +100 | +4.4% |
| Tue | Cache & guard tests (2.2) | 1.5 | +66 | +2.9% |
| Wed | Integration & bugfixes | 1.0 | - | - |
| **Total** | **Phase 2** | **5.0** | **+166** | **+7.3%** |

### Week 3: Completion

| Day | Task | Hours | Lines | Coverage Δ |
|-----|------|-------|-------|------------|
| Mon | Core module tests (3.1) | 2.0 | +150 | +6.6% |
| Tue | Nav/physics tests (3.2) | 2.0 | +87 | +3.8% |
| Wed | Final validation | 1.0 | - | - |
| **Total** | **Phase 3** | **5.0** | **+237** | **+10.5%** |

---

## Success Metrics

### Coverage Targets

| Metric | Current | Phase 1 | Phase 2 | Phase 3 | Target |
|--------|---------|---------|---------|---------|--------|
| Overall | 23.30% | 31.7% | 39.0% | 49.5% | 80%+ |
| async_task.rs | 0% | 83% | 83% | 83% | 80%+ |
| ai_arbiter.rs | 5% | 80% | 80% | 80% | 80%+ |
| orchestrator.rs | 63.93% | 63.93% | 63.93% | 75%+ | 80%+ |
| LLM modules | 0% | 0% | 60%+ | 60%+ | 60%+ |

### Test Quality Targets

| Metric | Current | Target |
|--------|---------|--------|
| Total Tests | 42 | 90+ |
| Pass Rate | 100% | 100% |
| Warnings | 0 | <5 |
| Test Runtime | <0.5s | <2s |

---

## Risk Mitigation

### Risk 1: Async Test Complexity ⚠️

**Probability:** Medium  
**Impact:** High  
**Mitigation:**
- Use tokio test utilities (`tokio::test`, `time::pause`)
- Start with simple async tests, add complexity gradually
- Document timing assumptions clearly
- Use `--test-threads=1` for reliability

### Risk 2: LLM Mock Server Reliability ⚠️

**Probability:** Medium  
**Impact:** Medium  
**Mitigation:**
- Use established mock server libraries (mockito, wiremock)
- Test mock server separately before using in tests
- Add retry logic for flaky network tests
- Use in-memory mocks where possible

### Risk 3: Coverage Tool Limitations ⚠️

**Probability:** Low  
**Impact:** Medium  
**Mitigation:**
- Tarpaulin has known limitations with async code
- Cross-validate with manual code review
- Focus on critical paths over 100% coverage
- Document known coverage gaps

### Risk 4: Time Overrun 🟡

**Probability:** Medium  
**Impact:** Low  
**Mitigation:**
- Phases are independent (can skip Phase 3)
- Phases 1-2 get to 39% (significant improvement)
- Phase 3 is polish (not critical)
- Can extend timeline if needed

---

## Alternative Approach: Integration-First Strategy

If unit test coverage is insufficient, consider integration tests:

**Pros:**
- Tests real behavior end-to-end
- Higher confidence in functionality
- Less mocking required

**Cons:**
- Slower test runtime
- More complex setup
- Harder to debug failures

**Recommendation:** Use hybrid approach - unit tests for 70%, integration for remaining 10%

---

## Maintenance Plan

### Post-80% Coverage

1. **Add CI coverage checks:**
   ```yaml
   - name: Coverage Check
     run: |
       cargo tarpaulin -p astraweave-ai --lib --out Stdout | grep "coverage"
       # Fail if < 75% (allow 5% buffer)
   ```

2. **Monthly coverage reviews:**
   - Generate HTML report
   - Review uncovered lines
   - Add tests for critical paths

3. **New feature policy:**
   - All new functions require tests
   - Minimum 80% coverage for new modules
   - PR checks enforce coverage

4. **Coverage debt tracking:**
   - Document known gaps in `COVERAGE_GAPS.md`
   - Prioritize gaps in critical modules
   - Quarterly reviews to reduce debt

---

## Conclusion

**Feasibility:** ✅ Achievable with focused effort  
**Timeline:** 8-12 hours over 2-3 weeks  
**Risk:** 🟡 Medium (async complexity)  
**Impact:** 🔥 High (doubles coverage, validates critical paths)

**Recommended Approach:**
1. Start with Phase 1 (async infrastructure) - highest impact
2. Evaluate results after Phase 1
3. Proceed with Phase 2 if time permits
4. Phase 3 is optional polish

**Success Definition:**
- ✅ async_task.rs: 80%+
- ✅ ai_arbiter.rs: 80%+
- ✅ orchestrator.rs: 75%+
- ✅ Overall: 35-40% (realistic) or 80%+ (aspirational)

---

**Next Steps:**
1. Review and approve this plan
2. Begin Phase 1 (AsyncTask tests)
3. Create tracking issue for each phase
4. Schedule weekly progress reviews

**Document Location:** `docs/AI_CRATE_80_PERCENT_FOLLOWUP_PLAN.md`  
**Related:** `docs/AI_CRATE_COVERAGE_REPORT.md`, `docs/AI_CRATE_STRATEGIC_ROADMAP.md`
