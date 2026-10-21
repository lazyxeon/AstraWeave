# Week 4 Action 14: Terrain Streaming Phase 2 - Implementation Plan

**Date**: October 13, 2025  
**Status**: 🔄 IN PROGRESS (Full Optimization)  
**Current Issue**: Soak test failing (9.37ms p99 vs 2.0ms target)

---

## Root Cause Analysis

### Performance Bottlenecks Identified

**1. Synchronous Chunk Generation** (PRIMARY BOTTLENECK)
- `WorldGenerator::generate_chunk()` requires `&mut self` (exclusive lock)
- Blocks all concurrent generation tasks while one runs
- **Impact**: 4 concurrent tasks → effectively 1 (serialized)
- **Fix**: Make generation lock-free (pure function, no `&mut`)

**2. Inadequate Concurrency**
- `max_concurrent_loads: 4` (too conservative)
- Modern CPUs have 8-16 threads
- **Impact**: Underutilizes CPU, queue backs up
- **Fix**: Increase to 8, add adaptive throttling

**3. Missing LOD Mesh Caching**
- LOD transitions regenerate meshes from heightmap
- **Impact**: Spike when camera moves between LOD bands
- **Fix**: Cache 3 LOD levels (L0, L1, L2) per chunk

**4. No Adaptive Throttling**
- Always runs at max capacity even when frame time high
- **Impact**: Frame hitches compound
- **Fix**: Reduce concurrent loads when frame >10ms

---

## Implementation Strategy

### Phase 1: Lock-Free Generation (4-5 hours) - CRITICAL

**Goal**: Remove `&mut self` requirement from `generate_chunk()`

**Current Signature**:
```rust
pub fn generate_chunk(&mut self, chunk_id: ChunkId) -> Result<TerrainChunk>
```

**Target Signature**:
```rust
// Static method - no self needed
pub fn generate_chunk_pure(
    config: &WorldConfig,
    noise: &TerrainNoise,  // Immutable noise gen
    climate: &ClimateMap,  // Immutable climate
    chunk_id: ChunkId
) -> Result<TerrainChunk>
```

**Changes Required**:
1. Make `TerrainNoise` immutable (remove internal state if any)
2. Make `ClimateMap` immutable
3. Convert `WorldGenerator` to hold `Arc` references
4. Update `background_loader.rs` to clone `Arc`s instead of locking

**Files to Modify**:
- `astraweave-terrain/src/lib.rs` (WorldGenerator)
- `astraweave-terrain/src/noise_gen.rs` (ensure stateless)
- `astraweave-terrain/src/climate.rs` (ensure stateless)
- `astraweave-terrain/src/background_loader.rs` (remove write lock)

**Expected Impact**: 4× speedup (4 tasks truly parallel)

---

### Phase 2: Increase Concurrency + Adaptive Throttling (1-2 hours)

**StreamingConfig Changes**:
```rust
pub struct StreamingConfig {
    pub max_loaded_chunks: usize,
    pub view_distance: u32,
    pub prefetch_distance: u32,
    pub max_concurrent_loads: usize,        // 4 → 8
    pub chunk_size: f32,
    
    // NEW FIELDS
    pub adaptive_throttle_threshold_ms: f32,  // 10.0ms
    pub throttled_concurrent_loads: usize,    // 2 (when throttling)
}
```

**Adaptive Logic**:
```rust
// In process_load_queue()
let current_concurrent_loads = if last_frame_time_ms > config.adaptive_throttle_threshold_ms {
    config.throttled_concurrent_loads  // Reduce to 2
} else {
    config.max_concurrent_loads  // Use full 8
};
```

**Files to Modify**:
- `astraweave-terrain/src/background_loader.rs` (StreamingConfig, adaptive logic)
- `astraweave-terrain/tests/streaming_integrity.rs` (pass frame time to loader)

**Expected Impact**: 2× speedup + prevent compounding hitches

---

### Phase 3: LOD Mesh Caching (2-3 hours)

**Goal**: Store pre-generated LOD meshes, reuse on transitions

**Current**: Regenerate mesh on LOD change
```rust
// Every LOD transition
let new_mesh = generate_mesh_at_lod(chunk, lod_level);  // EXPENSIVE
```

**Target**: Cache and retrieve
```rust
struct ChunkLodCache {
    l0_mesh: Option<TerrainMesh>,  // Full detail
    l1_mesh: Option<TerrainMesh>,  // Half resolution
    l2_mesh: Option<TerrainMesh>,  // Quarter resolution
}

// On first load
cache.l0_mesh = Some(generate_mesh_at_lod(chunk, LodLevel::Full));
cache.l1_mesh = Some(generate_mesh_at_lod(chunk, LodLevel::Half));

// On LOD transition
let mesh = cache.l1_mesh.clone();  // FAST - just clone Arc<MeshData>
```

**Files to Modify**:
- `astraweave-terrain/src/lod_manager.rs` (add ChunkLodCache)
- `astraweave-terrain/src/background_loader.rs` (integrate cache)
- `astraweave-terrain/src/meshing.rs` (ensure Arc<MeshData> for cheap cloning)

**Expected Impact**: Eliminates LOD transition spikes (0.5-2ms → <0.1ms)

---

### Phase 4: SIMD Meshing Optimization (2-3 hours) - OPTIONAL

**Goal**: Use AVX2 SIMD for marching cubes

**Current**: Scalar marching cubes
```rust
for cell in chunk.cells() {
    let config_index = calculate_config(cell);  // 8 comparisons
    let vertices = generate_vertices(config_index);  // lots of float ops
}
```

**Target**: SIMD batching
```rust
use std::arch::x86_64::*;

// Process 8 cells at once with AVX2
unsafe {
    let configs = _mm256_set1_epi32(/* ... */);
    // Vectorized comparisons and vertex generation
}
```

**Files to Modify**:
- `astraweave-terrain/src/meshing.rs` (add simd module)
- `astraweave-terrain/Cargo.toml` (feature flag `simd-meshing`)

**Expected Impact**: 1.5-2× speedup for mesh generation

**Risk**: High complexity, may defer to Week 5 if time-constrained

---

## Acceptance Criteria

| Criterion | Target | Current | After Phase 1-3 | Status |
|-----------|--------|---------|-----------------|--------|
| **p99 Frame Time** | <2.0ms | 9.37ms | ~1.8ms | ⏳ IN PROGRESS |
| **Missing Chunks** | 0 | 273,894 | 0 | ⏳ IN PROGRESS |
| **Memory Delta** | <6% | Unknown | <4% | ⏳ NOT TESTED |
| **Hitch Count** | <10 | Unknown | <5 | ⏳ NOT TESTED |
| **Avg Frame Time** | N/A | Unknown | <0.8ms | ⏳ NOT TESTED |

---

## Implementation Timeline

### Day 1 (October 13, Evening)

**Hours 1-2**: Phase 1 Setup
- Analyze `TerrainNoise` and `ClimateMap` for statefulness
- Design lock-free API
- Begin refactoring `WorldGenerator`

**Hours 3-5**: Phase 1 Implementation
- Convert `generate_chunk()` to pure function
- Update `background_loader.rs` to use Arc cloning
- Fix compilation errors
- Run quick test (64 ticks)

### Day 2 (October 14, Morning)

**Hours 6-7**: Phase 2 Implementation
- Update `StreamingConfig` with new fields
- Implement adaptive throttling logic
- Integrate with streaming integrity test

**Hours 8-9**: Phase 3 Implementation
- Add `ChunkLodCache` struct
- Implement cache in `lod_manager.rs`
- Wire into background loader

### Day 2 (October 14, Afternoon)

**Hours 10-11**: Validation
- Run soak test (1024 ticks)
- Analyze results
- Debug any remaining issues

**Hours 12-14**: Documentation (if passing) OR Phase 4 SIMD (if needed)
- If soak test passes: Write completion report
- If still failing: Implement SIMD optimization

---

## Risk Assessment

### HIGH RISK: Lock-Free Refactoring

**Challenge**: `TerrainNoise` or `ClimateMap` may have internal mutable state

**Mitigation**:
- Audit both structs for `&mut` methods
- If state exists, make it `Send + Sync` with `Arc<RwLock<>>`
- Fallback: Keep write lock but reduce critical section

**Worst Case**: If lock-free impossible, optimize critical section:
```rust
// BEFORE: Hold lock entire generation
let chunk = world_gen.write().await.generate_chunk(id)?;

// AFTER: Lock only for config access, generate outside lock
let (config, noise, climate) = {
    let gen = world_gen.read().await;
    (gen.config.clone(), gen.noise.clone(), gen.climate.clone())
};
let chunk = generate_chunk_pure(&config, &noise, &climate, id)?;
```

### MEDIUM RISK: Adaptive Throttling Overhead

**Challenge**: Checking frame time adds overhead

**Mitigation**:
- Only check every 4th frame (60 FPS → 15 checks/sec)
- Use atomic for lock-free frame time access

### LOW RISK: LOD Cache Memory

**Challenge**: 3× mesh storage per chunk

**Mitigation**:
- L3 (skybox) doesn't need cache (minimal geometry)
- Evict cache when chunk unloaded
- Compression: Store indices as u16, vertices as f16

---

## Next Steps

**Immediate**: Begin Phase 1 (lock-free generation)
1. Audit `TerrainNoise` for mutable state
2. Audit `ClimateMap` for mutable state
3. Design pure generation API
4. Implement refactoring

**Estimated Completion**: October 14, 6:00 PM (24 hours from now)

---

**Version**: 1.0  
**Status**: Implementation Plan Complete, Awaiting Execution  
**Author**: AstraWeave Copilot
