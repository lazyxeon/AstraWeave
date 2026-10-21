# Issue: GPU Compute Skinning & Pipelines

**Type**: Enhancement  
**Priority**: Medium  
**Effort**: 2-3 days  
**Milestone**: Phase 2 Polish

---

## Summary

Complete the GPU compute dispatch integration for skeletal animation skinning. The GPU pipeline structure exists (Phase D), but the compute shader dispatch wiring needs to be completed to enable full GPU-accelerated skinning.

---

## Context

**Current State** (from Task 5):
- ✅ GPU structures implemented (`JointPalette`, `JointMatrixGPU`)
- ✅ Compute shader compiled and validated
- ✅ Buffer upload/readback logic tested
- ✅ Feature flag `skinning-gpu` defined
- ⏳ Compute dispatch integration pending

**Current Limitation**:
- GPU parity tests are placeholders (compare CPU against itself)
- Compute shader exists but not dispatched in render pipeline
- GPU feature flag compiles but doesn't accelerate skinning yet

---

## Goals

1. **Wire Compute Dispatch**: Integrate compute shader into render pipeline
2. **Enable GPU Parity Tests**: Remove `#[ignore]` from 3 GPU comparison tests
3. **Validate Performance**: Measure GPU vs CPU for high vertex counts
4. **Document Usage**: Update docs with GPU feature flag benefits

---

## Acceptance Criteria

- [ ] Compute shader dispatches correctly in render pipeline
- [ ] GPU parity tests pass without `#[ignore]` (3 tests: rest pose, animated, blending)
- [ ] CPU↔GPU comparison within 0.01 units tolerance
- [ ] Performance benchmark shows GPU > CPU for 10K+ vertices
- [ ] Feature flag `--features skinning-gpu` fully functional
- [ ] Demo toggles between CPU/GPU with 'G' key (visible performance difference)
- [ ] Documentation updated with GPU usage examples

---

## Implementation Plan

### Step 1: Compute Pipeline Integration (1 day)

**File**: `astraweave-render/src/skinning_gpu.rs`

```rust
pub struct SkinningComputePipeline {
    pipeline: wgpu::ComputePipeline,
    joint_buffer: wgpu::Buffer,
    vertex_input_buffer: wgpu::Buffer,
    vertex_output_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SkinningComputePipeline {
    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        vertex_count: u32,
    ) {
        let workgroup_size = 256;
        let workgroup_count = (vertex_count + workgroup_size - 1) / workgroup_size;
        
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Skeletal Skinning Compute"),
        });
        
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }
}
```

**Integration Point**: Call before main render pass in `Renderer::render_with`

### Step 2: Update GPU Parity Tests (0.5 days)

**File**: `astraweave-render/tests/skinning_parity_cpu_vs_gpu.rs`

**Changes**:
1. Remove `#[ignore]` from `test_parity_rest_pose`, `test_parity_animated_frame`, `test_parity_weighted_blending`
2. Replace CPU-vs-CPU placeholders with actual GPU compute dispatch
3. Add GPU buffer readback to compare results
4. Keep tolerance at 0.001-0.01 based on complexity

**Example**:
```rust
#[test]
#[cfg(feature = "skinning-gpu")]  // Still feature-gated, but not ignored
fn test_parity_rest_pose() {
    let (device, queue) = create_test_device();
    
    // CPU path
    let cpu_result = skin_vertices_cpu(&skeleton, &matrices, &vertices);
    
    // GPU path
    let gpu_pipeline = SkinningComputePipeline::new(&device, &skeleton);
    gpu_pipeline.upload_matrices(&queue, &matrices);
    gpu_pipeline.upload_vertices(&queue, &vertices);
    gpu_pipeline.dispatch(&mut encoder, vertices.len() as u32);
    let gpu_result = gpu_pipeline.readback_vertices(&device, &queue);
    
    // Compare
    assert_vertices_close(&cpu_result, &gpu_result, 0.001);  // Tight tolerance for rest pose
}
```

### Step 3: Performance Benchmarking (0.5 days)

**File**: `astraweave-render/benches/skinning_perf.rs`

**Benchmarks**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_cpu_skinning_1k(c: &mut Criterion) {
    c.bench_function("cpu_skinning_1k_vertices", |b| {
        b.iter(|| skin_vertices_cpu(black_box(&vertices_1k)));
    });
}

fn bench_gpu_skinning_1k(c: &mut Criterion) {
    c.bench_function("gpu_skinning_1k_vertices", |b| {
        b.iter(|| gpu_pipeline.dispatch(black_box(1000)));
    });
}

fn bench_cpu_skinning_100k(c: &mut Criterion) {
    c.bench_function("cpu_skinning_100k_vertices", |b| {
        b.iter(|| skin_vertices_cpu(black_box(&vertices_100k)));
    });
}

fn bench_gpu_skinning_100k(c: &mut Criterion) {
    c.bench_function("gpu_skinning_100k_vertices", |b| {
        b.iter(|| gpu_pipeline.dispatch(black_box(100000)));
    });
}

criterion_group!(benches, 
    bench_cpu_skinning_1k, 
    bench_gpu_skinning_1k,
    bench_cpu_skinning_100k,
    bench_gpu_skinning_100k
);
criterion_main!(benches);
```

**Expected Results**:
- CPU: O(n) scaling (1K → 100K = 100× slower)
- GPU: O(1) scaling (1K → 100K = ~1.5× slower due to dispatch overhead)

### Step 4: Documentation Updates (1 day)

**Files to Update**:
1. `docs/PHASE2_TASK5_IMPLEMENTATION_SUMMARY.md`: Add GPU integration section
2. `examples/skinning_demo/README.md`: Update GPU toggle instructions
3. `astraweave-render/src/skinning_gpu.rs`: Add module-level docs with examples

**Example Documentation**:
```markdown
## GPU Skinning

GPU skinning is enabled with the `skinning-gpu` feature flag:

\`\`\`powershell
cargo run -p skinning_demo --features skinning-gpu
\`\`\`

### When to Use GPU Skinning

**Use GPU** for:
- High-poly characters (>10K vertices)
- Crowd rendering (many instances)
- Production builds (maximum performance)

**Use CPU** for:
- CI/testing (deterministic)
- Low-poly characters (<5K vertices)
- Single character focus

### Performance Comparison

| Vertices | CPU Time | GPU Time | Speedup |
|----------|----------|----------|---------|
| 1K       | 0.1ms    | 0.05ms   | 2×      |
| 10K      | 1.0ms    | 0.08ms   | 12×     |
| 100K     | 10.0ms   | 0.15ms   | 66×     |
\`\`\`

---

## Testing Plan

### Unit Tests
```powershell
# GPU pipeline tests (should all pass)
cargo test -p astraweave-render --lib skinning_gpu --features skinning-gpu
```

### GPU Parity Tests
```powershell
# Should pass without --ignored
cargo test -p astraweave-render --test skinning_parity_cpu_vs_gpu --features skinning-gpu
```

### Performance Benchmarks
```powershell
# Run criterion benchmarks
cargo bench --bench skinning_perf --features skinning-gpu
```

### Demo Validation
```powershell
# Run demo and toggle GPU with 'G' key
cargo run -p skinning_demo --features skinning-gpu --release

# Expected: FPS increases when GPU enabled (especially for complex models)
```

---

## Affected Crates

- **astraweave-render**: Compute dispatch, GPU pipeline integration
- **skinning_demo**: GPU toggle functionality

---

## Dependencies

**Blocked By**: None (Task 5 Phase D complete)  
**Blocks**: None (enhancement only)

---

## Feature Flags

```toml
[features]
skinning-cpu = []  # Default, deterministic
skinning-gpu = []  # Requires hardware, this issue completes this flag
```

---

## Done When

- [ ] Compute shader dispatches in render pipeline
- [ ] 3 GPU parity tests pass (no `#[ignore]`)
- [ ] Tolerance within 0.001-0.01 validated
- [ ] Performance benchmarks show GPU advantage
- [ ] Demo 'G' key toggles GPU with visible perf difference
- [ ] Documentation updated with usage and performance data
- [ ] All tests green: `cargo test --workspace --features skinning-gpu`
- [ ] PR merged with updated docs

---

**Issue Created**: October 1, 2025  
**Labels**: `enhancement`, `gpu`, `rendering`, `phase-2-polish`  
**Milestone**: Phase 2 Polish
