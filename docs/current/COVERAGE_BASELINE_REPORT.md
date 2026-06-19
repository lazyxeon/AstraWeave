# AstraWeave Test Coverage Baseline Report

**Version**: 1.0.0  
**Date**: December 6, 2025  
**Measured With**: cargo-llvm-cov 0.6.21  
**Total Production Crates**: 47

---

## Executive Summary

This report provides a comprehensive baseline measurement of test coverage across all 47 production crates in the AstraWeave game engine. Coverage was measured using `cargo llvm-cov --lib` which provides source-level line coverage.

### Overall Statistics

| Metric | Value |
|--------|-------|
| **Total Crates** | 47 |
| **Excellent (≥90%)** | 11 (23.4%) |
| **Good (70-89%)** | 6 (12.8%) |
| **Needs Work (50-69%)** | 4 (8.5%) |
| **Critical (<50%)** | 26 (55.3%) |
| **Weighted Average** | ~53% (estimated) |

### Key Findings

1. **Core Infrastructure is Strong**: ECS, math, physics, and core systems have excellent coverage (78-98%)
2. **LLM Support is Mixed**: Some crates excellent (embeddings 98%, memory 93%), others critical (persona 14%, rag 24%)
3. **Rendering is Underserved**: Main render crate at 36%, UI at 20%
4. **New/Experimental Crates Need Tests**: 6 crates at 0% coverage
5. **Network Layer Critical**: Both net (24%) and net-ecs (38%) need attention

---

## Coverage by Category

### Tier 1: Excellent Coverage (≥90%)

These crates meet or exceed the coverage standards.

| Crate | Line Coverage | Functions | Lines | Notes |
|-------|--------------|-----------|-------|-------|
| **astraweave-profiling** | 100.00% | 11/11 | 40/40 | Minimal crate, perfect coverage |
| **astraweave-cinematics** | 98.75% | 22/22 | 340/342 | Excellent, mature crate |
| **astraweave-embeddings** | 98.13% | 206/210 | 1624/1660 | LLM embedding support, excellent |
| **astraweave-math** | 98.05% | 66/66 | 545/559 | SIMD math, well-tested |
| **astraweave-ecs** | 97.10% | 399/416 | 3413/3525 | Core ECS, critical path covered |
| **astraweave-input** | 96.21% | 117/120 | 1233/1297 | Input handling, comprehensive |
| **astraweave-weaving** | 94.71% | 745/799 | 6601/7033 | Fate weaving system, excellent |
| **astraweave-pcg** | 94.78% | 44/49 | 357/382 | Procedural generation |
| **astraweave-prompts** | 93.98% | 286/297 | 2056/2183 | LLM prompt management |
| **astraweave-memory** | 93.58% | 547/598 | 5961/6373 | LLM memory management |
| **astraweave-nav** | 91.29% | 80/82 | 1207/1314 | Navigation/pathfinding |

**Total: 11 crates (23.4%)**

### Tier 2: Good Coverage (70-89%)

These crates have reasonable coverage but could benefit from additional tests.

| Crate | Line Coverage | Functions | Lines | Priority |
|-------|--------------|-----------|-------|----------|
| **astraweave-materials** | 88.18% | 27/29 | 359/384 | Low - close to excellent |
| **astraweave-core** | 84.29% | 494/656 | 5304/6510 | Medium - critical path |
| **astraweave-physics** | 78.86% | 342/432 | 3508/4346 | Medium - core system |
| **astraweave-persistence-player** | 76.74% | 33/49 | 297/392 | Low - adequate |
| **astraweave-security** | 70.63% | 225/378 | 2137/3222 | High - security critical |
| **astraweave-ai** | 70.41% | 234/421 | 3317/4569 | High - core AI system |

**Total: 6 crates (12.8%)**

### Tier 3: Needs Work (50-69%)

These crates have partial coverage and should be prioritized.

| Crate | Line Coverage | Functions | Lines | Gap to 70% |
|-------|--------------|-----------|-------|------------|
| **astraweave-behavior** | 66.83% | 174/331 | 1540/2443 | +78 lines |
| **astraweave-llm** | 65.84% | 670/1099 | 6879/10466 | +437 lines |
| **astraweave-assets** | 51.66% | 115/192 | 1378/2372 | +435 lines |
| **astraweave-terrain** | 50.67% | 590/1296 | 5531/11208 | +2165 lines |

**Total: 4 crates (8.5%)**

### Tier 4: Critical Coverage (<50%)

These crates require significant test investment.

| Crate | Line Coverage | Functions | Lines | Realistic Cap |
|-------|--------------|-----------|-------|---------------|
| **astraweave-asset** | 49.22% | 141/265 | 1311/2582 | 80% (async I/O) |
| **astraweave-context** | 38.47% | 289/846 | 2513/7466 | 75% (LLM integration) |
| **astraweave-net-ecs** | 38.97% | 63/207 | 550/1422 | 70% (network) |
| **astraweave-gameplay** | 36.98% | 249/815 | 3150/8144 | 80% |
| **astraweave-render** | 36.27% | 756/1998 | 8685/25342 | 60% (GPU) |
| **astraweave-scene** | 33.73% | 119/330 | 861/2664 | 70% |
| **astraweave-scripting** | 31.75% | 131/564 | 1207/4771 | 70% (rhai integration) |
| **astraweave-persistence-ecs** | 27.36% | 49/283 | 514/1941 | 80% |
| **astraweave-net** | 24.72% | 50/298 | 622/2620 | 60% (network I/O) |
| **astraweave-rag** | 24.19% | 123/692 | 1360/5577 | 75% (LLM) |
| **astraweave-observability** | 22.76% | 54/248 | 493/1889 | 70% |
| **astraweave-audio** | 22.05% | 114/799 | 1530/7145 | 50% (audio hardware) |
| **astraweave-sdk** | 20.48% | 50/297 | 415/2253 | 60% (FFI) |
| **astraweave-ui** | 20.34% | 223/886 | 1919/8581 | 60% (GUI) |
| **astraweave-stress-test** | 16.25% | 28/207 | 234/1418 | 50% (load testing) |
| **astraweave-persona** | 14.05% | 165/1145 | 1553/9862 | 70% |
| **astraweave-llm-eval** | 9.45% | 23/401 | 292/2844 | 60% (evaluation) |
| **astraweave-fluids** | 0.00% | 0/3 | 0/290 | 70% (GPU simulation) |
| **astraweave-author** | 0.00% | 0/7 | 0/72 | 60% (rhai issues) |
| **astraweave-dialogue** | 0.00% | 0/3 | 0/15 | 90% (simple) |
| **astraweave-ipc** | 0.00% | 0/6 | 0/41 | 70% (IPC) |
| **astraweave-npc** | 0.00% | 0/15 | 0/229 | 80% |
| **astraweave-secrets** | 0.00% | 0/16 | 0/51 | 80% |

**Total: 26 crates (55.3%)**

---

## Coverage Caps Analysis

Some crates have structural limitations that prevent achieving 90%+ coverage:

### GPU-Dependent Crates (Cap: 50-60%)

- **astraweave-render**: Requires GPU context for integration tests
- **astraweave-fluids**: GPU simulation, limited testability
- **astraweave-audio**: Hardware audio dependencies

### Network I/O Crates (Cap: 60-70%)

- **astraweave-net**: Async network operations
- **astraweave-net-ecs**: Network + ECS integration
- **astraweave-ipc**: Inter-process communication

### LLM Integration Crates (Cap: 70-75%)

- **astraweave-rag**: Depends on embedding models
- **astraweave-context**: LLM context management
- **astraweave-llm-eval**: Evaluation requires LLM responses

### FFI/SDK Crates (Cap: 60%)

- **astraweave-sdk**: C FFI bindings
- **astraweave-scripting**: Rhai integration

---

## Fixes Applied During Audit

### 1. astraweave-observability: Async Deadlock Fix

**Issue**: `RequestTracker::complete()` held DashMap guard across `.await` point  
**Fix**: Extract data before async operations  
**File**: `astraweave-observability/src/llm_telemetry.rs`

```rust
// Before: Guard held across await (deadlock)
let active_request = self.telemetry.active_requests.get(&self.request_id)?;
// ... create trace using active_request ...
self.telemetry.record_request(trace).await  // DEADLOCK

// After: Extract data first
let prompt_tokens = {
    let active_request = self.telemetry.active_requests.get(&self.request_id)?;
    active_request.prompt_tokens
}; // Guard dropped
// ... create trace using prompt_tokens ...
self.telemetry.record_request(trace).await  // OK
```

### 2. astraweave-ai: Environment Variable Test Isolation

**Issue**: Tests manipulating env vars racing in parallel  
**Fix**: Added `serial_test` crate and `#[serial]` attribute  
**File**: `astraweave-ai/src/orchestrator.rs`

```rust
use serial_test::serial;

#[test]
#[serial]
fn system_orchestrator_config_respects_ollama_model_env() {
    std::env::set_var("OLLAMA_MODEL", "llama3:70b");
    // ...
}
```

---

## Recommendations

### Immediate Actions (Week 1)

1. **Add tests to 0% crates**: dialogue, npc, secrets (simple crates, quick wins)
2. **Fix astraweave-author**: Resolve rhai Sync trait issues
3. **Boost astraweave-behavior**: Close to 70% threshold

### Short-Term (Weeks 2-4)

1. **LLM Support Sprint**: Raise persona (14%), rag (24%), llm-eval (9%)
2. **Persistence Sprint**: persistence-ecs (27%), asset (49%)
3. **Network Sprint**: net (24%), net-ecs (38%)

### Medium-Term (Months 2-3)

1. **Rendering Tests**: Add mock GPU context infrastructure
2. **Audio Tests**: Abstract hardware dependencies
3. **Integration Tests**: Cross-crate test suites

---

## Test Count by Crate

| Crate | Tests | Pass Rate |
|-------|-------|-----------|
| astraweave-ai | 103 | 100% |
| astraweave-ecs | 136 | 100% |
| astraweave-physics | ~60 | 100% |
| astraweave-weaving | 64 | 100% |
| astraweave-observability | 5 | 100% |
| astraweave-core | ~50 | 100% |
| ... | ... | ... |

---

## Methodology

### Measurement Tool

```bash
cargo llvm-cov --lib -p <crate>
```

### Coverage Metrics

- **Regions**: Code regions/branches
- **Functions**: Function entry points
- **Lines**: Source code lines
- **Branches**: Not measured (llvm-cov limitation)

### Exclusions

- Test code (`#[cfg(test)]`)
- Generated code
- Examples and benchmarks

---

## Appendix: Raw Data

### Full Coverage Table

| Crate | Regions | Functions | Lines |
|-------|---------|-----------|-------|
| astraweave-profiling | 100.00% | 100.00% | 100.00% |
| astraweave-cinematics | 98.75% | 100.00% | 99.42% |
| astraweave-embeddings | 98.13% | 98.10% | 97.83% |
| astraweave-math | 98.05% | 100.00% | 97.50% |
| astraweave-ecs | 97.10% | 95.91% | 96.82% |
| astraweave-input | 96.21% | 97.50% | 95.07% |
| astraweave-weaving | 94.71% | 93.24% | 93.86% |
| astraweave-pcg | 94.78% | 89.80% | 93.46% |
| astraweave-prompts | 93.98% | 96.30% | 94.18% |
| astraweave-memory | 93.58% | 91.47% | 93.54% |
| astraweave-nav | 91.29% | 97.56% | 91.86% |
| astraweave-materials | 88.18% | 93.10% | 93.49% |
| astraweave-core | 84.29% | 75.30% | 81.47% |
| astraweave-physics | 78.86% | 79.17% | 80.72% |
| astraweave-persistence-player | 76.74% | 67.35% | 75.77% |
| astraweave-security | 70.63% | 59.52% | 66.33% |
| astraweave-ai | 70.41% | 55.58% | 72.60% |
| astraweave-behavior | 66.83% | 52.57% | 63.04% |
| astraweave-llm | 65.84% | 60.96% | 65.73% |
| astraweave-assets | 51.66% | 59.90% | 58.09% |
| astraweave-terrain | 50.67% | 45.52% | 49.35% |
| astraweave-asset | 49.22% | 53.21% | 50.77% |
| astraweave-context | 38.47% | 34.16% | 33.66% |
| astraweave-net-ecs | 38.97% | 30.43% | 38.68% |
| astraweave-gameplay | 36.98% | 30.55% | 38.68% |
| astraweave-render | 36.27% | 37.84% | 34.27% |
| astraweave-scene | 33.73% | 36.06% | 32.32% |
| astraweave-scripting | 31.75% | 23.23% | 25.30% |
| astraweave-persistence-ecs | 27.36% | 17.31% | 26.48% |
| astraweave-net | 24.72% | 16.78% | 23.74% |
| astraweave-rag | 24.19% | 17.77% | 24.39% |
| astraweave-observability | 22.76% | 21.77% | 26.10% |
| astraweave-audio | 22.05% | 14.27% | 21.41% |
| astraweave-sdk | 20.48% | 16.84% | 18.42% |
| astraweave-ui | 20.34% | 25.17% | 22.36% |
| astraweave-stress-test | 16.25% | 13.53% | 16.50% |
| astraweave-persona | 14.05% | 14.41% | 15.75% |
| astraweave-llm-eval | 9.45% | 5.74% | 10.27% |
| astraweave-fluids | 0.00% | 0.00% | 0.00% |
| astraweave-author | 0.00% | 0.00% | 0.00% |
| astraweave-dialogue | 0.00% | 0.00% | 0.00% |
| astraweave-ipc | 0.00% | 0.00% | 0.00% |
| astraweave-npc | 0.00% | 0.00% | 0.00% |
| astraweave-secrets | 0.00% | 0.00% | 0.00% |

---

**Report Generated**: December 6, 2025  
**Next Update**: After remediation sprint completion
