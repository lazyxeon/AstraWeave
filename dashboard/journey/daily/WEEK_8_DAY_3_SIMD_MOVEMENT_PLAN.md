# Week 8 Day 3: SIMD Movement Optimization - Implementation Plan

**Date**: October 12, 2025  
**Phase**: Phase B - Month 4 - Week 8 - Day 3  
**Status**: 🚀 **IN PROGRESS**  
**Estimated Time**: 6-8 hours  

---

## Executive Summary

**Goal**: Optimize the movement system using SIMD (Single Instruction, Multiple Data) vectorization to process 4-8 entities per CPU instruction, reducing movement time from **861 µs → 430-600 µs** (-30-50%).

**Current Performance** (from Day 2):
- **movement**: 861 µs per frame (30% of 2.87 ms total)
- **Entity Count**: 1,000 entities
- **Per-Entity Cost**: 0.861 µs per entity

**Target Performance**:
- **movement**: 430-600 µs per frame (-30-50%)
- **Frame Time**: 2.87 ms → 2.3-2.5 ms (-13-20%)
- **FPS**: 348 → 400-435 FPS (+15-25%)

---

## SIMD Background

### What is SIMD?

**SIMD (Single Instruction, Multiple Data)**: CPU instructions that process multiple data elements in parallel using wide registers (128-512 bits).

**AVX2 (Advanced Vector Extensions 2)**:
- **Register Width**: 256 bits (8 × 32-bit floats, or 4 × 64-bit doubles)
- **Operations**: Add, multiply, FMA (fused multiply-add), shuffle, etc.
- **Supported**: Intel Haswell+ (2013), AMD Excavator+ (2015)
- **Rust Support**: `std::arch::x86_64` intrinsics, or libraries like `glam` with SIMD features

**Performance Potential**:
- **Theoretical**: 8× speedup (8 floats per instruction)
- **Practical**: 2-4× speedup (due to memory bandwidth, alignment, control flow)

### Movement System Analysis

**Current Implementation** (profiling_demo):
```rust
// Naive scalar loop
for (entity, mut pos, vel) in query.iter_mut() {
    pos.x += vel.x * dt;  // 3 scalar operations
    pos.y += vel.y * dt;
    pos.z += vel.z * dt;
}
```

**Per-Entity Cost**:
- 3 × multiply (3 CPU cycles)
- 3 × add (3 CPU cycles)
- Memory: 2 × Vec3 loads + 1 × Vec3 store (6-12 cycles)
- **Total**: ~15-20 CPU cycles per entity

**At 1,000 entities**: 15,000-20,000 cycles = **~6.25-8.3 µs @ 2.4 GHz**

**Wait, that's only 6-8 µs, but Tracy shows 861 µs!**

**Missing overhead**:
- ECS iteration (archetype lookup, component access)
- Cache misses (sparse memory access)
- Branch mispredictions
- Other operations in movement span

**Measured**: 861 µs / 1,000 entities = **0.861 µs per entity** (realistic)

---

## SIMD Optimization Strategy

### Approach 1: Manual SIMD (AVX2 Intrinsics)

**Pros**:
- Maximum control over vectorization
- Can handle AoS (Array of Structs) → SoA (Struct of Arrays) conversion
- Explicit alignment and prefetching

**Cons**:
- Complex, error-prone code
- Platform-specific (requires runtime CPU detection)
- Harder to maintain

**Example**:
```rust
use std::arch::x86_64::*;

#[target_feature(enable = "avx2")]
unsafe fn update_positions_simd(positions: &mut [Vec3], velocities: &[Vec3], dt: f32) {
    let dt_vec = _mm256_set1_ps(dt);  // Broadcast dt to 8 lanes
    
    for chunk in 0..(positions.len() / 8) {
        // Load 8 Vec3s (24 floats) - requires SoA layout!
        let pos_x = _mm256_loadu_ps(&positions_x[chunk * 8]);
        let vel_x = _mm256_loadu_ps(&velocities_x[chunk * 8]);
        
        // pos_x += vel_x * dt (8 entities at once)
        let new_pos_x = _mm256_fmadd_ps(vel_x, dt_vec, pos_x);
        
        _mm256_storeu_ps(&mut positions_x[chunk * 8], new_pos_x);
        // Repeat for y, z...
    }
}
```

**Problem**: ECS uses AoS (Vec3 struct), but SIMD wants SoA (separate x, y, z arrays). Conversion overhead may negate gains!

### Approach 2: SIMD-Friendly Libraries (glam with SIMD)

**Pros**:
- `glam` crate has AVX2 support via `scalar-math` feature flag
- Transparent SIMD (compiler chooses when to vectorize)
- No unsafe code
- Works with AoS layout

**Cons**:
- Less control (compiler may not vectorize)
- May not achieve 4× speedup

**Current Usage**: profiling_demo uses `glam::Vec3` already!

**Check if SIMD is enabled**:
```rust
// In Cargo.toml
glam = { version = "0.29", features = ["scalar-math"] }  // Disables SIMD
glam = { version = "0.29" }  // Enables SIMD by default on supported platforms
```

### Approach 3: Hybrid (Manual Batching + SIMD Math)

**Best of both worlds**:
1. Batch entities into groups of 4-8
2. Use `glam::Vec3A` (aligned Vec3 for SIMD)
3. Unroll loop manually to help compiler vectorize
4. Keep ECS iteration unchanged (no SoA conversion)

**Example**:
```rust
const BATCH_SIZE: usize = 4;

for chunk in positions.chunks_exact_mut(BATCH_SIZE) {
    // Process 4 entities per iteration
    chunk[0] += velocities[0] * dt;  // Compiler may vectorize this
    chunk[1] += velocities[1] * dt;
    chunk[2] += velocities[2] * dt;
    chunk[3] += velocities[3] * dt;
}
```

---

## Recommended Approach: Hybrid with glam SIMD

**Why**:
- ✅ Minimal code changes (no AoS → SoA conversion)
- ✅ Safe Rust (no unsafe intrinsics)
- ✅ Portable (works on all platforms)
- ✅ Measurable (Tracy will show improvement)
- ✅ Realistic 2-3× speedup (vs 8× theoretical)

**Implementation Plan**:

### Step 1: Verify glam SIMD Configuration (15 min)

**Check profiling_demo Cargo.toml**:
```bash
grep -A 2 "glam" examples/profiling_demo/Cargo.toml
```

**Expected**: `glam.workspace = true` (inherits from workspace)

**Check workspace Cargo.toml**:
```bash
grep -A 2 "glam" Cargo.toml
```

**If SIMD disabled**, enable it:
```toml
[workspace.dependencies]
glam = "0.29"  # SIMD enabled by default
```

### Step 2: Create SIMD Movement Module (2-3h)

**File**: `crates/astraweave-math/src/simd_movement.rs`

**API**:
```rust
/// SIMD-optimized movement update
pub fn update_positions_simd(
    positions: &mut [Vec3],
    velocities: &[Vec3],
    dt: f32,
) {
    // Batch processing with unrolled loop
    const BATCH_SIZE: usize = 4;
    
    let (batches, remainder) = positions.split_at_mut(
        positions.len() - (positions.len() % BATCH_SIZE)
    );
    
    // Process 4 entities per iteration (SIMD-friendly)
    for i in (0..batches.len()).step_by(BATCH_SIZE) {
        positions[i + 0] += velocities[i + 0] * dt;
        positions[i + 1] += velocities[i + 1] * dt;
        positions[i + 2] += velocities[i + 2] * dt;
        positions[i + 3] += velocities[i + 3] * dt;
    }
    
    // Handle remainder (< 4 entities)
    for i in batches.len()..positions.len() {
        positions[i] += velocities[i] * dt;
    }
}
```

**Optimizations**:
- Loop unrolling (compiler sees 4× pattern → SIMD)
- Aligned access (use `Vec3A` if available)
- Prefetching hints (for large batches)

### Step 3: Integrate with profiling_demo (1h)

**Modify `examples/profiling_demo/src/main.rs`**:

```rust
use astraweave_math::simd_movement::update_positions_simd;

fn movement(world: &mut World, dt: f32) {
    tracy_zone!("movement");
    
    // Collect positions and velocities into contiguous arrays
    let mut query = world.query::<(&mut Position, &Velocity)>();
    let count = query.iter().count();
    
    let mut positions: Vec<Vec3> = Vec::with_capacity(count);
    let mut velocities: Vec<Vec3> = Vec::with_capacity(count);
    let mut entities: Vec<Entity> = Vec::with_capacity(count);
    
    for (entity, (pos, vel)) in query.iter() {
        positions.push(pos.0);
        velocities.push(vel.0);
        entities.push(entity);
    }
    
    // SIMD update (this is the hot path)
    update_positions_simd(&mut positions, &velocities, dt);
    
    // Write back to ECS (unavoidable overhead)
    for (i, entity) in entities.iter().enumerate() {
        if let Some(mut pos) = world.get_mut::<Position>(*entity) {
            pos.0 = positions[i];
        }
    }
}
```

**Note**: Collecting into arrays has overhead (~100-200 µs), but SIMD savings should exceed it!

### Step 4: Benchmark (30 min)

**Create**: `crates/astraweave-math/benches/simd_movement.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use glam::Vec3;
use astraweave_math::simd_movement::update_positions_simd;

fn naive_movement(positions: &mut [Vec3], velocities: &[Vec3], dt: f32) {
    for i in 0..positions.len() {
        positions[i] += velocities[i] * dt;
    }
}

fn benchmark_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("movement");
    
    for count in [100, 1000, 10000] {
        let mut positions = vec![Vec3::ZERO; count];
        let velocities = vec![Vec3::ONE; count];
        let dt = 0.016;
        
        group.bench_function(&format!("naive_{}", count), |b| {
            b.iter(|| {
                naive_movement(black_box(&mut positions), black_box(&velocities), black_box(dt))
            });
        });
        
        group.bench_function(&format!("simd_{}", count), |b| {
            b.iter(|| {
                update_positions_simd(black_box(&mut positions), black_box(&velocities), black_box(dt))
            });
        });
    }
}

criterion_group!(benches, benchmark_movement);
criterion_main!(benches);
```

**Run**:
```bash
cargo bench -p astraweave-math --bench simd_movement
```

**Expected**:
- Naive: ~300-400 ns per 1,000 entities (pure math)
- SIMD: ~100-150 ns per 1,000 entities (2-4× faster)

### Step 5: Tracy Validation (30 min)

**Rebuild profiling_demo**:
```bash
cargo build -p profiling_demo --features profiling --release
```

**Run and capture**:
```bash
cargo run -p profiling_demo --features profiling --release -- --entities 1000
```

**Save trace**: `baseline_1000_simd_movement.tracy`

**Expected**:
- movement: 861 µs → 430-600 µs (-30-50%)
- Frame time: 2.87 ms → 2.3-2.5 ms (-13-20%)

### Step 6: Documentation (1-2h)

**Create**:
- `WEEK_8_DAY_3_SIMD_MOVEMENT_COMPLETE.md` (detailed results)
- `WEEK_8_DAY_3_SUMMARY.md` (executive summary)

---

## Alternative: ECS-Native SIMD (If Above Fails)

**If array collection overhead is too high**, try **in-place SIMD**:

```rust
fn movement_simd_in_place(world: &mut World, dt: f32) {
    tracy_zone!("movement");
    
    // Get raw component arrays (requires ECS API support)
    let positions = world.components_mut::<Position>();
    let velocities = world.components::<Velocity>();
    
    // Direct SIMD on component arrays (no copy)
    for i in 0..positions.len() {
        positions[i].0 += velocities[i].0 * dt;
    }
}
```

**Requires**: ECS to expose contiguous component slices (may not be available in current ECS).

---

## Performance Targets

### Conservative (2× SIMD speedup)

| Metric | Baseline | Target | Improvement |
|--------|----------|--------|-------------|
| **movement** | 861 µs | **430 µs** | **-50%** |
| **Frame Time** | 2.87 ms | **2.44 ms** | **-15%** |
| **FPS** | 348 | **410** | **+18%** |

### Optimistic (4× SIMD speedup, low overhead)

| Metric | Baseline | Target | Improvement |
|--------|----------|--------|-------------|
| **movement** | 861 µs | **300 µs** | **-65%** |
| **Frame Time** | 2.87 ms | **2.31 ms** | **-19.5%** |
| **FPS** | 348 | **433** | **+24%** |

### Realistic (3× SIMD, +100 µs overhead)

| Metric | Baseline | Target | Improvement |
|--------|----------|--------|-------------|
| **movement** | 861 µs | **550 µs** | **-36%** |
| **Frame Time** | 2.87 ms | **2.56 ms** | **-11%** |
| **FPS** | 348 | **391** | **+12%** |

---

## Success Criteria

✅ **Minimum Acceptable**:
- movement: < 600 µs (-30%)
- Frame time: < 2.5 ms (-13%)
- FPS: > 400 (+15%)

⭐ **Target**:
- movement: 430-550 µs (-37-50%)
- Frame time: 2.3-2.5 ms (-13-20%)
- FPS: 400-435 (+15-25%)

⭐⭐⭐ **Stretch**:
- movement: < 400 µs (-54%)
- Frame time: < 2.3 ms (-20%)
- FPS: > 435 (+25%)

---

## Risk Assessment

### High Risk ⚠️

1. **Array Collection Overhead**: Copying to/from Vec may cost 100-200 µs
   - **Mitigation**: Pre-allocate, reuse buffers, or use in-place SIMD

2. **ECS Access Patterns**: Sparse archetype iteration may prevent vectorization
   - **Mitigation**: Batch processing, manual unrolling

3. **Memory Bandwidth**: 1,000 Vec3s = 12 KB (fits L1), but random access hurts
   - **Mitigation**: Sequential processing, prefetching

### Medium Risk ⚠️

4. **Compiler Optimization**: May not auto-vectorize despite hints
   - **Mitigation**: Check assembly (`cargo asm`), use explicit SIMD if needed

5. **Platform Differences**: AVX2 not available on all machines
   - **Mitigation**: Runtime CPU detection, fallback to scalar

### Low Risk ✅

6. **Correctness**: SIMD bugs are rare with high-level APIs like glam
   - **Mitigation**: Unit tests comparing SIMD vs naive results

---

## Timeline

**Total**: 6-8 hours

| Task | Duration | Priority |
|------|----------|----------|
| 1. Verify glam SIMD config | 15 min | 🔴 High |
| 2. Create simd_movement.rs | 2-3h | 🔴 High |
| 3. Integrate with profiling_demo | 1h | 🔴 High |
| 4. Benchmark | 30 min | 🟡 Medium |
| 5. Tracy validation | 30 min | 🔴 High |
| 6. Documentation | 1-2h | 🟡 Medium |
| **Total** | **6-8h** | - |

---

## Next Steps (Immediate)

1. ✅ **Check glam configuration** in workspace Cargo.toml
2. ✅ **Create simd_movement.rs** module skeleton
3. ✅ **Implement batched SIMD update** with loop unrolling
4. ✅ **Write unit tests** (correctness validation)
5. ✅ **Integrate with profiling_demo**
6. ⏳ **Benchmark and validate**

**Ready to start implementation!** 🚀

---

**Status**: Plan complete, beginning Step 1 (glam SIMD verification)...
