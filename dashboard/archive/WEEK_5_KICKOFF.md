# Week 5 Kickoff: GPU Optimization & SIMD Math Sprint

**Week**: 5 (October 13-15, 2025)  
**Focus**: Performance optimization, code quality, asset pipeline  
**Status**: 📋 **PLANNING**  
**Estimated Time**: 25-34 hours (3 days)  
**Priority**: 🔴 High-impact performance wins

---

## Context

**Week 4 Complete**: All 6 actions delivered (**100%**), adding **2,397 LOC** with **4-50× performance improvements**. Phase A complete in **3 days** (431% efficiency vs 3-week plan).

**Week 5 Strategy**: Focus on **rendering/math performance** (GPU mesh, SIMD), **code quality** (unwrap remediation), and **developer experience** (asset pipeline). Target **5-6 actions** over **3 days**.

---

## Proposed Actions (5-6 total)

### 🔴 Priority 1: GPU Mesh Optimization (Action 19)

**Goal**: Vertex compression, LOD generation, GPU instancing  
**Impact**: 50% memory reduction, 2× draw call reduction  
**Estimated Time**: 6-8 hours  

**Scope**:
- **Vertex Compression**: Pack normals (octahedral), UVs (half-float)
- **LOD Generation**: Automated mesh simplification (quadric error metrics)
- **GPU Instancing**: Batch identical meshes (grass, rocks, trees)
- **Benchmarks**: Memory usage, draw call counts, render time

**Deliverables**:
- `astraweave-render/src/vertex_compression.rs` (+150 LOC)
- `astraweave-render/src/lod_generator.rs` (+200 LOC)
- `astraweave-render/src/instancing.rs` (+100 LOC)
- `WEEK_5_ACTION_19_COMPLETE.md` (completion report)

**Acceptance Criteria**:
1. ✅ Vertex memory reduced by 40-60% (compressed normals/UVs)
2. ✅ LOD generation produces 3-5 levels (target poly reduction: 25%, 50%, 75%)
3. ✅ Instancing batches 100+ identical meshes into 1 draw call
4. ✅ Benchmarks show 2× draw call reduction
5. ✅ Zero rendering artifacts (visual validation)

**Dependencies**: wgpu 25.0.2, meshopt (optional, Rust bindings)

---

### 🔴 Priority 2: Unwrap Remediation Phase 4 (Action 20)

**Goal**: Fix 40-50 unwraps in context (34), terrain (27), llm (27) crates  
**Impact**: Production code safety, actionable error messages  
**Estimated Time**: 3-4 hours  

**Scope**:
- **Target Crates**: astraweave-context, astraweave-terrain, astraweave-llm
- **Total Unwraps**: 88 (34 + 27 + 27)
- **Remediation Target**: 40-50 unwraps (50-57% of total)
- **Safe Patterns**: `anyhow::Result`, `.context()`, `unwrap_or()`, `?` operator

**Deliverables**:
- Modified files: `context/src/*.rs`, `terrain/src/*.rs`, `llm/src/*.rs` (~10-15 files)
- Updated `unwrap_audit_report.csv` (579 → ~530, 8-9% reduction)
- `WEEK_5_ACTION_20_COMPLETE.md` (completion report)

**Acceptance Criteria**:
1. ✅ 40-50 unwraps remediated (88 → 38-48 remaining)
2. ✅ All remediations use established safe patterns
3. ✅ `cargo check` passes for all modified crates
4. ✅ `cargo test` passes (no regressions)
5. ✅ CSV updated with remediation status

**Example Remediations**:

**Pattern A: Config Parsing**
```rust
// Before:
let config: Config = toml::from_str(content).unwrap();

// After:
let config: Config = toml::from_str(content)
    .context("Failed to parse config.toml")?;
```

**Pattern B: HashMap Lookups**
```rust
// Before:
let value = map.get(&key).unwrap();

// After:
let value = map.get(&key)
    .ok_or_else(|| anyhow!("Key '{}' not found in map", key))?;
```

**Pattern C: Lock Poisoning**
```rust
// Before:
let guard = mutex.lock().unwrap();

// After:
let guard = mutex.lock()
    .map_err(|e| anyhow!("Mutex poisoned: {}", e))?;
```

**Dependencies**: None (code quality work)

---

### 🔴 Priority 3: SIMD Math Optimization (Action 21)

**Goal**: SIMD vector operations, matrix multiplication  
**Impact**: 2-4× math performance (physics, transforms, lighting)  
**Estimated Time**: 6-8 hours  

**Scope**:
- **SIMD Vec3/Vec4**: SSE/AVX2 dot product, cross product, normalization
- **SIMD Mat4**: Matrix multiplication, inverse, transpose
- **Auto-vectorization**: Compiler hints, manual intrinsics
- **Benchmarks**: Math operations (vector ops, matrix ops)

**Deliverables**:
- `astraweave-math/src/simd_vec.rs` (+200 LOC)
- `astraweave-math/src/simd_mat.rs` (+250 LOC)
- `astraweave-math/benches/simd_benchmarks.rs` (+150 LOC)
- `WEEK_5_ACTION_21_COMPLETE.md` (completion report)

**Acceptance Criteria**:
1. ✅ SIMD Vec3 dot product 2-4× faster than scalar
2. ✅ SIMD Mat4 multiply 2-4× faster than scalar
3. ✅ Benchmarks show consistent speedup across operations
4. ✅ Tests verify correctness (SIMD == scalar results)
5. ✅ Portable (fallback to scalar on non-SIMD platforms)

**Target Performance** (from benchmarks):

| Operation | Scalar | SIMD (SSE/AVX2) | Speedup |
|-----------|--------|-----------------|---------|
| **Vec3 dot** | 10 ns | 3-5 ns | 2-3× |
| **Vec3 cross** | 15 ns | 5-7 ns | 2-3× |
| **Vec3 normalize** | 20 ns | 7-10 ns | 2-3× |
| **Mat4 multiply** | 100 ns | 30-40 ns | 2.5-3× |
| **Mat4 inverse** | 200 ns | 60-80 ns | 2.5-3× |

**Dependencies**: `std::arch` (x86/x86_64 intrinsics), `wide` crate (optional)

---

### 🟡 Priority 4: LLM Prompt Optimization (Action 22)

**Goal**: Fine-tune prompts, reduce token usage  
**Impact**: 20-30% token reduction, improved accuracy  
**Estimated Time**: 4-6 hours  

**Scope**:
- **Prompt Templates**: Refine 15 existing templates (more concise, focused)
- **Token Reduction**: Remove redundancy, compress instructions
- **Few-Shot Examples**: Add 3-5 examples per template
- **A/B Testing**: Compare old vs new prompts (accuracy, tokens)

**Deliverables**:
- Modified `astraweave-llm/src/prompt_library.rs` (~200 LOC changes)
- `WEEK_5_ACTION_22_COMPLETE.md` (A/B test results, token savings)

**Acceptance Criteria**:
1. ✅ 20-30% token reduction (avg 400 tokens → 280-320)
2. ✅ Accuracy maintained or improved (>95% task success)
3. ✅ A/B tests show statistical significance (10+ runs per prompt)
4. ✅ Few-shot examples improve zero-shot performance
5. ✅ Documentation updated with optimization tips

**Example Optimization**:

**Before** (400 tokens):
```
You are an AI assistant controlling a game character. Your goal is to help the player by performing actions in the game world. You have access to various tools that allow you to interact with objects, navigate the environment, and communicate with other characters. When given a task, carefully analyze the current situation, consider the available tools, and select the most appropriate action. Always explain your reasoning before taking action. Be creative and helpful!

Current Task: [task here]
Available Tools: [tools here]
```

**After** (280 tokens):
```
Control game character. Use tools to complete task.

Task: [task here]
Tools: [tools here]

Think step-by-step, then act.
```

**Token Reduction**: 400 → 280 (**30% savings**)

**Dependencies**: None (prompt engineering)

---

### 🟡 Priority 5: Asset Pipeline Automation (Action 23)

**Goal**: Texture compression, mesh optimization, CI validation  
**Impact**: Faster iteration, consistent quality, automated checks  
**Estimated Time**: 6-8 hours  

**Scope**:
- **Texture Compression**: Auto-convert PNG → BC7/ASTC (CI step)
- **Mesh Optimization**: Auto-simplify high-poly meshes (vertex cache, overdraw)
- **Validation**: CI checks for texture sizes, mesh poly counts, naming conventions
- **CLI Tool**: `aw_asset_cli` enhancements (batch processing)

**Deliverables**:
- `tools/aw_asset_cli/src/texture_processor.rs` (+150 LOC)
- `tools/aw_asset_cli/src/mesh_optimizer.rs` (+200 LOC)
- `.github/workflows/asset_validation.yml` (+100 LOC)
- `WEEK_5_ACTION_23_COMPLETE.md` (completion report)

**Acceptance Criteria**:
1. ✅ Auto-compress textures to BC7 (Windows/Linux) or ASTC (Mobile)
2. ✅ Mesh optimization reduces vertex count by 10-30%
3. ✅ CI validates asset sizes (max 2048×2048 textures, 10K poly meshes)
4. ✅ Batch processing handles 100+ assets in <5 minutes
5. ✅ Documentation for asset authoring guidelines

**Asset Validation Rules**:
- **Textures**: Max 2048×2048, BC7/ASTC format, power-of-2 dimensions
- **Meshes**: Max 10K triangles (LOD0), vertex cache optimized, indexed
- **Naming**: snake_case, descriptive (e.g., `grass_albedo.png`, `tree_oak_lod0.glb`)

**Dependencies**: `image` crate, `meshopt` bindings, CI (GitHub Actions)

---

### 🟢 Priority 6 (OPTIONAL): Debug Toolkit Enhancements (Action 24)

**Goal**: Performance overlay, entity inspector, console  
**Impact**: Developer experience, real-time debugging  
**Estimated Time**: 4-6 hours  

**Scope**:
- **Performance Overlay**: FPS, frame time, memory usage (egui)
- **Entity Inspector**: Select entity, view components, edit values
- **Console**: Command input, output history, autocomplete

**Deliverables**:
- `tools/aw_debug/src/perf_overlay.rs` (+100 LOC)
- `tools/aw_debug/src/entity_inspector.rs` (+150 LOC)
- `tools/aw_debug/src/console.rs` (+200 LOC)
- `WEEK_5_ACTION_24_COMPLETE.md` (completion report)

**Acceptance Criteria**:
1. ✅ Performance overlay shows FPS, frame time, memory
2. ✅ Entity inspector displays components, allows editing
3. ✅ Console supports 10+ commands (spawn, delete, teleport, etc.)
4. ✅ Keyboard shortcuts (F1 toggle overlay, F2 inspector, F3 console)
5. ✅ Zero performance impact when disabled

**Dependencies**: egui 0.32, astraweave-ecs (query API)

**Note**: **Deferred to Week 6+** if time constraints (lower priority than performance/quality work).

---

## Recommended Schedule

### Day 1: October 13 (Monday)

**Action 19: GPU Mesh Optimization** (6-8 hours)

**Morning** (3-4 hours):
- Implement vertex compression (octahedral normals, half-float UVs)
- Write unit tests (compression/decompression round-trip)

**Afternoon** (3-4 hours):
- Implement LOD generation (quadric error metrics, target poly counts)
- Implement GPU instancing (batch identical meshes)
- Write benchmarks (memory, draw calls, render time)
- Create completion report

**Deliverables**: Vertex compression, LOD generation, instancing, benchmarks

---

### Day 2: October 14 (Tuesday)

**Morning: Action 20: Unwrap Remediation** (3-4 hours)

**Tasks**:
- Run `audit_unwrap.ps1` on context/terrain/llm crates
- Apply safe patterns to 40-50 unwraps
- Run `cargo check` and `cargo test` for validation
- Update CSV, create completion report

**Afternoon: Action 21: SIMD Math** (4 hours)

**Tasks**:
- Implement SIMD Vec3 operations (dot, cross, normalize)
- Implement SIMD Mat4 multiply
- Write benchmarks (scalar vs SIMD)
- Write tests (correctness verification)

**Deliverables**: Unwrap remediation, SIMD math (partial)

---

### Day 3: October 15 (Wednesday)

**Morning: Action 21 (continued)** (2-4 hours)

**Tasks**:
- Complete SIMD Mat4 operations (inverse, transpose)
- Add portable fallback (non-SIMD platforms)
- Finalize benchmarks and tests
- Create completion report

**Afternoon: Action 22 OR 23** (4-6 hours)

**Option A - LLM Prompt Optimization** (faster, higher value):
- Refine 15 prompt templates (token reduction)
- Add few-shot examples
- Run A/B tests (accuracy, token usage)
- Create completion report

**Option B - Asset Pipeline Automation** (longer, infrastructure):
- Implement texture compression (BC7/ASTC)
- Implement mesh optimization
- Add CI validation workflow
- Create completion report

**Recommendation**: **Action 22** (LLM optimization) for faster completion, higher immediate value.

**Deliverables**: SIMD math complete, LLM prompt optimization

---

## Week 5 Timeline Summary

| Day | Actions | Hours | Deliverables |
|-----|---------|-------|--------------|
| **Oct 13** | Action 19 (GPU Mesh) | 6-8 | Vertex compression, LOD, instancing |
| **Oct 14** | Actions 20 (Unwrap) + 21 (SIMD) | 7-8 | 40-50 unwraps fixed, SIMD Vec3/Mat4 |
| **Oct 15** | Actions 21 (SIMD) + 22 (LLM) | 6-10 | SIMD complete, prompt optimization |
| **TOTAL** | **3-4 actions** | **19-26h** | **GPU perf, code quality, LLM** |

**Optional**: Add Action 23 (Asset Pipeline) on Oct 16 if time permits (6-8 hours).

---

## Success Metrics

### Performance Targets

| Subsystem | Current | Target | Improvement |
|-----------|---------|--------|-------------|
| **Vertex Memory** | 100% | 50-60% | 40-50% reduction |
| **Draw Calls** | 100% | 50% | 2× reduction |
| **Vec3 Dot Product** | 10 ns | 3-5 ns | 2-3× faster |
| **Mat4 Multiply** | 100 ns | 30-40 ns | 2.5-3× faster |
| **LLM Tokens** | 400 avg | 280-320 | 20-30% reduction |

---

### Quality Targets

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| **Unwraps (context)** | 34 | 10-17 | 50% reduction |
| **Unwraps (terrain)** | 27 | 8-14 | 50% reduction |
| **Unwraps (llm)** | 27 | 8-14 | 50% reduction |
| **Total Unwraps** | 579 | ~530 | 8-9% reduction |

---

### Acceptance Criteria (Week 5)

**Mandatory** (Actions 19-21):
1. ✅ GPU mesh optimization: 40-50% memory reduction, 2× draw call reduction
2. ✅ Unwrap remediation: 40-50 unwraps fixed, CSV updated
3. ✅ SIMD math: 2-4× performance improvement, tests verify correctness

**Optional** (Actions 22-23):
4. 🟡 LLM prompt optimization: 20-30% token reduction, accuracy maintained
5. 🟡 Asset pipeline: Automated compression, CI validation

**Overall Target**: 3/5 mandatory + 1-2/2 optional = **4-5 actions** (80-100%)

---

## Dependencies & Risks

### Technical Dependencies

**Action 19 (GPU Mesh)**:
- ✅ wgpu 25.0.2 (installed)
- 🟡 meshopt Rust bindings (optional, can use quadric error metrics manually)

**Action 21 (SIMD)**:
- ✅ `std::arch` (stable Rust)
- 🟡 `wide` crate (optional, can use raw intrinsics)

**Action 23 (Asset Pipeline)**:
- ✅ `image` crate (installed)
- 🟡 meshopt bindings (optional)
- ✅ GitHub Actions (available)

---

### Risks & Mitigations

**Risk 1: SIMD Portability**
- **Impact**: Non-x86 platforms (ARM, WASM) may lack SIMD support
- **Mitigation**: Add scalar fallback, feature-gate SIMD (default on x86)
- **Priority**: 🟢 Low (most targets are x86)

**Risk 2: LOD Generation Complexity**
- **Impact**: Quadric error metrics are complex, may take >8 hours
- **Mitigation**: Use existing crate (`meshopt` bindings) or defer to Week 6
- **Priority**: 🟡 Medium (can simplify to uniform decimation)

**Risk 3: LLM Prompt Accuracy**
- **Impact**: Token reduction may hurt accuracy
- **Mitigation**: A/B test before deployment, revert if accuracy drops >5%
- **Priority**: 🟡 Medium (measure before commit)

**Risk 4: Asset Pipeline Scope Creep**
- **Impact**: Texture compression + mesh optimization + CI = 10-12 hours
- **Mitigation**: Defer Action 23 to Week 6 if Actions 19-22 take longer
- **Priority**: 🟢 Low (Action 23 is optional)

---

## Knowledge Transfer

### Safe Pattern Reference (Action 20)

**Pattern A: Configuration Parsing**
```rust
// Before:
let config: Config = toml::from_str(content).unwrap();

// After:
let config: Config = toml::from_str(content)
    .context("Failed to parse config.toml")?;
```

**Pattern B: HashMap Lookups**
```rust
// Before:
let value = map.get(&key).unwrap();

// After:
let value = map.get(&key)
    .ok_or_else(|| anyhow!("Key '{}' not found in map", key))?;
```

**Pattern C: Lock Poisoning**
```rust
// Before:
let guard = mutex.lock().unwrap();

// After:
let guard = mutex.lock()
    .map_err(|e| anyhow!("Mutex poisoned: {}", e))?;
```

**Pattern D: Option Chaining**
```rust
// Before:
let result = option.unwrap().process().unwrap();

// After:
let result = option
    .ok_or_else(|| anyhow!("Option was None"))?
    .process()
    .context("Process failed")?;
```

---

### SIMD Math Reference (Action 21)

**Vec3 Dot Product** (SSE):
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn dot_simd(a: Vec3, b: Vec3) -> f32 {
    unsafe {
        let a_vec = _mm_set_ps(0.0, a.z, a.y, a.x);
        let b_vec = _mm_set_ps(0.0, b.z, b.y, b.x);
        let mul = _mm_mul_ps(a_vec, b_vec);
        let sum = _mm_hadd_ps(mul, mul);
        let sum = _mm_hadd_ps(sum, sum);
        _mm_cvtss_f32(sum)
    }
}
```

**Mat4 Multiply** (SSE):
```rust
pub fn mul_simd(a: Mat4, b: Mat4) -> Mat4 {
    unsafe {
        let mut result = Mat4::identity();
        for i in 0..4 {
            let row = _mm_set_ps(a[i][3], a[i][2], a[i][1], a[i][0]);
            for j in 0..4 {
                let col = _mm_set_ps(b[3][j], b[2][j], b[1][j], b[0][j]);
                let mul = _mm_mul_ps(row, col);
                let sum = _mm_hadd_ps(mul, mul);
                let sum = _mm_hadd_ps(sum, sum);
                result[i][j] = _mm_cvtss_f32(sum);
            }
        }
        result
    }
}
```

---

## Conclusion

**Week 5 targets high-impact performance wins** (GPU mesh, SIMD math) and **code quality improvements** (unwrap remediation). Estimated **19-26 hours** over **3 days** (October 13-15) for **3-4 mandatory actions** plus **1-2 optional**.

**Priority Ranking**:
1. 🔴 **Action 19**: GPU Mesh Optimization (highest performance impact)
2. 🔴 **Action 20**: Unwrap Remediation (code quality, safety)
3. 🔴 **Action 21**: SIMD Math (broad performance benefit)
4. 🟡 **Action 22**: LLM Prompt Optimization (cost savings, faster)
5. 🟡 **Action 23**: Asset Pipeline (infrastructure, longer)
6. 🟢 **Action 24**: Debug Toolkit (developer experience, optional)

**Recommendation**: Complete **Actions 19-21** (mandatory) + **Action 22** (high-value optional) = **4 actions** in **3 days** for **Week 5 completion**.

---

**Status**: 📋 **PLANNING COMPLETE**  
**Next**: Start **Action 19 (GPU Mesh Optimization)** on October 13, 2025  
**Version**: 1.0  
**Author**: AstraWeave Copilot  
**Date**: October 10, 2025, 11:45 PM
