# AstraWeave Coverage Gap Analysis

**Version**: 1.0.0  
**Date**: December 6, 2025  
**Based On**: COVERAGE_BASELINE_REPORT.md v1.0.0

---

## Executive Summary

This gap analysis identifies patterns in test coverage gaps and provides strategic recommendations for test improvement. The analysis reveals that 55% of crates fall below 50% coverage, with clear patterns in GPU-dependent, network, and LLM integration code.

### Coverage Distribution

```
Excellent (≥90%): ████████████ 11 crates (23%)
Good (70-89%):    ██████ 6 crates (13%)
Needs Work:       ████ 4 crates (9%)
Critical (<50%):  ██████████████████████████ 26 crates (55%)
```

---

## Gap Patterns Identified

### Pattern 1: Zero Coverage Crates (6 crates)

**Crates**: dialogue, npc, secrets, fluids, author, ipc

**Root Cause Analysis**:
- `astraweave-fluids`: GPU-dependent simulation, no mock infrastructure
- `astraweave-author`: Rhai Sync trait compilation issues preventing test execution
- `astraweave-dialogue`: New/stub crate, only 15 lines of code
- `astraweave-ipc`: Inter-process communication, requires OS mocking
- `astraweave-npc`: New crate, 229 lines, no tests added yet
- `astraweave-secrets`: Security-sensitive, may need careful test design

**Remediation Strategy**:
1. **Quick Wins** (dialogue, npc, secrets): Add basic unit tests - 2-4 hours each
2. **Infrastructure** (fluids): Create GPU mock context - 8-16 hours
3. **Fix Blocking Issues** (author): Resolve Rhai Sync trait - 4-8 hours
4. **Design Carefully** (ipc): Add integration tests with process isolation - 8-12 hours

### Pattern 2: LLM Integration Gaps (5 crates)

**Crates**: persona (14%), rag (24%), llm-eval (9%), context (38%), observability (22%)

**Root Cause Analysis**:
- Dependency on external LLM services for integration tests
- Complex async/streaming code patterns
- State management across conversations

**Remediation Strategy**:
1. **Mock LLM Client**: Create comprehensive MockLlm with configurable responses
2. **Conversation Mocks**: Pre-recorded conversation fixtures
3. **Deterministic Embeddings**: Mock embedding client for reproducible tests
4. **Token Counting Tests**: Pure unit tests for token budget management

**Estimated Coverage Caps**:
| Crate | Current | Realistic Cap | Gap |
|-------|---------|---------------|-----|
| persona | 14% | 70% | 56% |
| rag | 24% | 75% | 51% |
| llm-eval | 9% | 60% | 51% |
| context | 38% | 75% | 37% |
| observability | 22% | 70% | 48% |

### Pattern 3: GPU/Rendering Gaps (3 crates)

**Crates**: render (36%), fluids (0%), audio (22%)

**Root Cause Analysis**:
- Require GPU context initialization
- Hardware-dependent code paths
- Real-time processing requirements

**Remediation Strategy**:
1. **Headless Testing**: wgpu supports headless backend for CI
2. **Mock Device**: Create MockDevice trait for unit tests
3. **Split Architecture**: Separate pure logic from GPU code
4. **Shader Validation**: Compile-time shader validation tests

**Estimated Coverage Caps**:
| Crate | Current | Realistic Cap | Gap |
|-------|---------|---------------|-----|
| render | 36% | 60% | 24% |
| fluids | 0% | 70% | 70% |
| audio | 22% | 50% | 28% |

### Pattern 4: Network/Async Gaps (3 crates)

**Crates**: net (24%), net-ecs (38%), ipc (0%)

**Root Cause Analysis**:
- Network I/O mocking complexity
- Async runtime requirements
- Protocol-level testing needs

**Remediation Strategy**:
1. **MockSocket**: Create socket abstraction with mock implementation
2. **Loopback Tests**: Use localhost for integration tests
3. **Protocol Unit Tests**: Test serialization/deserialization in isolation
4. **Timeout Testing**: Ensure timeout handling is covered

**Estimated Coverage Caps**:
| Crate | Current | Realistic Cap | Gap |
|-------|---------|---------------|-----|
| net | 24% | 60% | 36% |
| net-ecs | 38% | 70% | 32% |
| ipc | 0% | 70% | 70% |

### Pattern 5: Gameplay/Integration Gaps (4 crates)

**Crates**: gameplay (36%), scripting (31%), persistence-ecs (27%), scene (33%)

**Root Cause Analysis**:
- Cross-system dependencies
- Complex state management
- Integration test infrastructure missing

**Remediation Strategy**:
1. **Test Worlds**: Create pre-configured World fixtures
2. **Component Mocks**: Mock dependent components
3. **Scenario Tests**: Test specific gameplay scenarios
4. **Save/Load Round-trips**: Serialization tests

**Estimated Coverage Caps**:
| Crate | Current | Realistic Cap | Gap |
|-------|---------|---------------|-----|
| gameplay | 36% | 80% | 44% |
| scripting | 31% | 70% | 39% |
| persistence-ecs | 27% | 80% | 53% |
| scene | 33% | 70% | 37% |

---

## Uncovered Code Patterns

### 1. Error Handling Paths

Many crates have untested error paths:
```rust
// Pattern: Error handling often uncovered
fn load_asset(path: &Path) -> Result<Asset> {
    let data = fs::read(path)?;  // Error path untested
    parse_asset(&data)
}
```

**Recommendation**: Add tests with invalid inputs, missing files, corrupted data.

### 2. Edge Cases in Collections

Empty/single/large collection cases often missed:
```rust
// Pattern: Collection edge cases
fn process_entities(entities: &[Entity]) {
    if entities.is_empty() { return; }  // Often untested
    // ...
}
```

**Recommendation**: Add `_empty`, `_single`, `_large` test variants.

### 3. Async Timeout Paths

Timeout handling rarely tested:
```rust
// Pattern: Timeout handling
async fn fetch_with_timeout() {
    tokio::time::timeout(Duration::from_secs(5), fetch())
        .await
        .map_err(|_| TimeoutError)?  // Rarely tested
}
```

**Recommendation**: Use `tokio::time::pause()` for deterministic timeout tests.

### 4. Feature-Gated Code

Code behind feature flags often untested:
```rust
#[cfg(feature = "gpu-tests")]
fn test_gpu_rendering() { ... }  // Only runs with specific features
```

**Recommendation**: CI matrix should include all feature combinations.

---

## Coverage Goals

### Phase 1: Quick Wins (Weeks 1-2)
Target: Raise 6 zero-coverage crates to ≥50%

| Crate | Current | Target | Effort |
|-------|---------|--------|--------|
| dialogue | 0% | 80% | 2h |
| npc | 0% | 70% | 4h |
| secrets | 0% | 70% | 3h |
| ipc | 0% | 50% | 6h |
| author | 0% | 50% | 8h (fix rhai) |
| fluids | 0% | 40% | 12h |

**Total Effort**: ~35 hours

### Phase 2: Critical Systems (Weeks 3-6)
Target: Raise critical crates to ≥50%

| Crate | Current | Target | Effort |
|-------|---------|--------|--------|
| persona | 14% | 60% | 16h |
| rag | 24% | 60% | 14h |
| llm-eval | 9% | 50% | 12h |
| gameplay | 36% | 60% | 10h |
| scripting | 31% | 60% | 12h |
| persistence-ecs | 27% | 60% | 10h |

**Total Effort**: ~74 hours

### Phase 3: Rendering Infrastructure (Weeks 7-10)
Target: Establish GPU test infrastructure and reach 50%+ on render crates

| Crate | Current | Target | Effort |
|-------|---------|--------|--------|
| render | 36% | 55% | 24h |
| scene | 33% | 55% | 12h |
| ui | 20% | 50% | 16h |
| audio | 22% | 45% | 14h |

**Total Effort**: ~66 hours

### Phase 4: Excellence (Ongoing)
Target: Maintain 70%+ on all crates, 90%+ on critical paths

Focus areas:
- Core systems (core, ecs, ai)
- Security-critical (security, net)
- Business-critical (weaving, gameplay)

---

## Risk Assessment

### High Risk (Immediate Action Required)

| Crate | Risk | Impact |
|-------|------|--------|
| net | Security vulnerabilities untested | Critical |
| security | Auth/crypto paths untested | Critical |
| persistence-ecs | Data corruption untested | High |

### Medium Risk (Address in Phase 2)

| Crate | Risk | Impact |
|-------|------|--------|
| gameplay | Game logic bugs | Medium |
| scripting | Script injection | Medium |
| llm | Prompt injection | Medium |

### Low Risk (Address in Phase 3+)

| Crate | Risk | Impact |
|-------|------|--------|
| render | Visual bugs | Low |
| audio | Audio glitches | Low |
| fluids | Simulation errors | Low |

---

## Metrics Dashboard

### Current State

```
Overall Coverage:    ████████████░░░░░░░░ ~53%
Crates ≥90%:         ████████████░░░░░░░░ 23% (11/47)
Crates ≥70%:         ██████████████████░░ 36% (17/47)
Crates ≥50%:         █████████████████████ 45% (21/47)
Crates <50%:         ██████████████████████████ 55% (26/47)
```

### Target State (3 months)

```
Overall Coverage:    ██████████████████░░ ~75%
Crates ≥90%:         ████████████████░░░░ 30% (14/47)
Crates ≥70%:         ████████████████████████ 60% (28/47)
Crates ≥50%:         █████████████████████████████ 90% (42/47)
Crates <50%:         ██ 10% (5/47, GPU-capped)
```

---

## Appendix: Coverage Heat Map

```
HIGH COVERAGE (Green, ≥90%):
┌─────────────────────────────────────────────────────────┐
│ profiling │ cinematics │ embeddings │ math │ ecs       │
│ input     │ weaving    │ pcg        │ prompts │ memory │
│ nav       │            │            │         │        │
└─────────────────────────────────────────────────────────┘

GOOD COVERAGE (Yellow, 70-89%):
┌─────────────────────────────────────────────────────────┐
│ materials │ core │ physics │ persistence-player        │
│ security  │ ai   │         │                           │
└─────────────────────────────────────────────────────────┘

NEEDS WORK (Orange, 50-69%):
┌─────────────────────────────────────────────────────────┐
│ behavior │ llm │ assets │ terrain                      │
└─────────────────────────────────────────────────────────┘

CRITICAL (Red, <50%):
┌─────────────────────────────────────────────────────────┐
│ asset     │ context   │ net-ecs │ gameplay │ render    │
│ scene     │ scripting │ persist │ net      │ rag       │
│ observ    │ audio     │ sdk     │ ui       │ stress    │
│ persona   │ llm-eval  │ fluids  │ author   │ dialogue  │
│ ipc       │ npc       │ secrets │          │           │
└─────────────────────────────────────────────────────────┘
```

---

**Next Document**: COVERAGE_REMEDIATION_PLAN.md
