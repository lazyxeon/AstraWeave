# Week 4 Action 16 Progress: Unwrap Remediation Phase 3

**Status**: 🟡 **IN PROGRESS**  
**Date**: October 10, 2025  
**Target**: 40+ production unwraps fixed  
**Current**: 1 fixed, 39+ to go

---

## Investigation Summary

### Audit Findings

**Total `.unwrap()` calls**: 571  
**Production unwraps** (excluding test/bench/example files): 412

**Key Discovery**: Most unwraps in target crates (render, scene, nav) are in `#[test]` or `#[cfg(test)]` modules. The audit script counts them as production code because they're in `src/` files, but they're actually test unwraps (P3-Low priority).

### Actual Production Unwraps by Crate

| Crate | Total | Production | Test/Bench | Notes |
|-------|-------|------------|------------|-------|
| **astraweave-terrain** | 29 | ~6 | ~23 | Real production unwraps found in streaming_diagnostics.rs, noise_simd.rs |
| **astraweave-render** | 47 | ~2 | ~45 | Most in `#[test]` modules (graph.rs, material.rs tests) |
| **astraweave-scene** | 47 | ~2 | ~45 | All in `#[test]` modules (lib.rs tests) |
| **astraweave-nav** | 2 | 0 | 2 | Minimal unwraps |
| **astraweave-context** | 34 | ~3 | ~31 | Mostly test unwraps |
| **astraweave-core** | 19 | ~8 | ~11 | ECS/memory code |
| **astraweave-behavior** | 11 | ~5 | ~6 | GOAP/BT planners |

### Priority Re-Assessment

Since target crates have fewer production unwraps than expected (~13 vs. 40), expanding scope to include:
- ✅ **astraweave-terrain** (6 production unwraps)
- ✅ **astraweave-core** (8 production unwraps)
- ✅ **astraweave-behavior** (5 production unwraps)
- ✅ **astraweave-llm** (23 production unwraps - high count)
- ✅ **astraweave-embeddings** (22 production unwraps)
- ✅ **astraweave-memory** (20 production unwraps)

**New Target**: Fix 40+ production unwraps across high-impact crates (terrain, core, behavior, llm, embeddings, memory)

---

## Remediation Plan

### Phase 1: Streaming Diagnostics (terrain) ✅
**File**: `astraweave-terrain/src/streaming_diagnostics.rs`  
**Location**: Line 97  
**Issue**: `.unwrap()` on `partial_cmp` (can panic on NaN)  
**Fix**: Replace with `unwrap_or(std::cmp::Ordering::Equal)`  
**Status**: ✅ **FIXED**

```rust
// Before:
sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

// After:
sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
```

### Phase 2: Remaining Terrain Unwraps (5 unwraps)
**Files**:
- `noise_simd.rs` - Lines 135, 152, 159, 195, 198 (5 unwraps in test code)
- `climate.rs` - Line 382 (test unwrap)
- `chunk.rs` - Line 297 (test unwrap)

**Note**: After manual review, these are ALL in `#[cfg(test)]` modules. Skipping.

### Phase 3: Core Crate Unwraps (8 production unwraps)
**Target**: `astraweave-core/src/**/*.rs`  
**Strategy**: Find actual production unwraps, apply safe patterns

### Phase 4: LLM/Embeddings/Memory Unwraps (65 production unwraps)
**High-Value Target**: These crates have significant production unwraps in AI systems  
**Strategy**: Batch fix with documented expect() for known-safe cases

---

## Safe Pattern Reference

| Pattern | Use Case | Example |
|---------|----------|---------|
| **unwrap_or(default)** | Fallback value acceptable | `option.unwrap_or(0)` |
| **unwrap_or_else(\|\| default)** | Lazy default computation | `option.unwrap_or_else(\|\| compute())` |
| **expect("reason")** | Known-safe with documentation | `option.expect("config validated at startup")` |
| **? operator** | Error propagation | `let x = fallible()?;` |
| **if let Some(x)** | Graceful skip | `if let Some(x) = opt { use(x); }` |
| **let Some(x) else { return }** | Early return | `let Some(x) = opt else { return; }` |
| **match** | Multiple outcomes | `match opt { Some(x) => ..., None => ... }` |

### Ordering::Equal for NaN handling
**Use Case**: Sorting floats with potential NaN values  
**Pattern**: `.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)`  
**Rationale**: NaN values compare as "equal" (stable sort position), preventing panic

---

## Next Steps

1. ✅ Fix streaming_diagnostics.rs unwrap (COMPLETE)
2. Find actual production unwraps in core/behavior/llm crates
3. Batch fix with appropriate patterns
4. Test affected crates
5. Update audit CSV
6. Document velocity in completion report

---

**Current Count**: 1/40 fixed (2.5%)  
**Time Spent**: 1 hour (investigation + 1 fix)  
**Est. Remaining**: 3-5 hours (need to find real production unwraps)
