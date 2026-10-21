# Week 8 Day 4 Summary

**Date**: October 12, 2025  
**Duration**: 4 hours  
**Status**: ✅ Complete (Day 3 confirmed optimal)

---

## What We Tried

### 3 Optimization Strategies Tested:

1. **Rayon Full Parallel**: 4.93 ms (**+82% slower**) ❌
   - Overhead (50-100 µs) >> work (1 µs SIMD)
   - Lock contention ~1.5 ms

2. **Hybrid Threshold (500)**: 3.95 ms (**+46% slower**) ❌
   - Per-frame overhead still present
   - Branch misprediction cost

3. **Direct ECS Mutation**: 4.10 ms (**+52% slower**) ❌
   - `get_mut()` 1,000× slower than batched `collect()`
   - Lost SIMD cache locality

---

## Why Day 3 is Optimal

**Root Cause**: Movement bottleneck is **59% ECS overhead** (collection + writeback), **not** the 0.15% SIMD core.

| Component | Time | Optimization Potential |
|-----------|------|------------------------|
| Collection | 200 µs | ❌ Can't avoid (ECS query) |
| SIMD Core | 1 µs | ❌ Already 2.05× optimized |
| Writeback | 200 µs | ❌ Batched beats scattered |
| Bounds | 150 µs | ⚠️ Minor gain possible |
| Overhead | 124 µs | ❌ Memory ops |

**Amdahl's Law**: Only 0.15-22.4% parallelizable → max 1.24× speedup (Rayon overhead erases this).

**Batching Wins**: `collect() → SIMD → writeback` is **3-5× faster** than scattered `get_mut()`.

---

## Week 8 Final Results

| Metric | Day 1 | Day 3 | Change |
|--------|-------|-------|--------|
| **Frame Time** | 3.09 ms | 2.70 ms | **-12.6%** ✅ |
| **FPS** | 323 | 370 | **+14.6%** ✅ |
| **movement** | ~1,054 µs | 675 µs | **-36.0%** ✅ |

**Assessment**:
- ✅ 12.6% improvement in 3 days
- ✅ 84% headroom vs 60 FPS budget
- ✅ 2.70 ms is 8% from 2.5 ms stretch target

**Decision**: **Accept Day 3 as final Week 8 result**

---

## Lessons Learned

1. **Measure before parallelizing**: Work must be >>100× overhead
2. **Amdahl's Law is ruthless**: 59% sequential = max 2.4× speedup
3. **Batched > Scattered**: ECS collect/writeback beats per-entity `get_mut()`
4. **SIMD already optimal**: glam auto-vectorization is 80-85% of hand-written AVX2

---

## Next Steps

**Week 8 Day 5** (Final Validation):
1. Comprehensive Tracy capture (2,000+ frames)
2. Regression tests
3. Benchmarks
4. Create `WEEK_8_OPTIMIZATION_COMPLETE.md`

**Future Optimization** (Phase B/C):
- Collision: 1,100 µs → 500-700 µs (flat grid array)
- Rendering: 380 µs → 200-300 µs (instancing)
- Parallel ECS: Chunked iteration (~200-400 µs gain)

**Combined Potential**: 2.70 ms → 1.6-1.9 ms (-30-41%)

---

**Full Details**: See `WEEK_8_DAY_4_COMPLETE.md` (15,000 words)
