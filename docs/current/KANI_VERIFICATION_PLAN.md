# AstraWeave Kani Verification Plan

**Version**: 1.0.0  
**Date**: February 3, 2026  
**Status**: 📋 PLANNING - Installation Pending  
**Author**: GitHub Copilot (AI-generated, zero human-written code)

---

## Executive Summary

This document outlines a comprehensive plan to apply **Kani** (a bit-precise model checker for Rust) to formally verify critical systems in AstraWeave. While Miri validates runtime undefined behavior, Kani provides **formal mathematical proofs** of correctness properties that go beyond testing.

### Kani vs Miri Comparison

| Aspect | Miri | Kani |
|--------|------|------|
| **Type** | Runtime UB detector | Formal model checker |
| **Approach** | Executes test cases | Exhaustive symbolic analysis |
| **Coverage** | Test-dependent | All possible inputs |
| **Proves** | No UB in tested paths | Mathematical correctness |
| **Speed** | Moderate (~100× slowdown) | Slow (minutes per proof) |
| **Best For** | Memory safety validation | Algorithm correctness proofs |

### Key Insight

**Miri finds bugs. Kani proves their absence.**

---

## Systems Identified for Kani Verification

### Tier 1: Critical Unsafe Code (High Priority)

These systems contain unsafe code with complex invariants that benefit most from formal verification:

#### 1. BlobVec (astraweave-ecs/src/blob_vec.rs)

**Priority**: 🔴 CRITICAL  
**Lines**: 789  
**Unsafe Operations**: ~30+ blocks

**Properties to Verify**:
- Memory allocation never overflows (capacity × item_size)
- No out-of-bounds access in `push`, `get`, `swap_remove`
- Drop function called exactly once per element
- No double-free on Drop implementation
- Reallocation preserves existing data
- `clone_fn` called with valid aligned pointers

**Example Proof Harness**:
```rust
#[kani::proof]
fn blob_vec_push_get_roundtrip() {
    let mut blob = BlobVec::new::<u32>();
    let value: u32 = kani::any();  // Symbolic value
    
    unsafe { blob.push(value); }
    
    let retrieved = unsafe { blob.get::<u32>(0) };
    kani::assert(retrieved == Some(&value), "Push/get roundtrip must preserve value");
}
```

#### 2. SparseSet (astraweave-ecs/src/sparse_set.rs)

**Priority**: 🔴 CRITICAL  
**Lines**: 638  
**Unsafe Operations**: 0 (but uses Entity::from_raw in tests)

**Properties to Verify**:
- Insert returns correct dense index
- Get always returns valid index or None
- Remove maintains sparse↔dense consistency
- No index overflow in sparse array expansion
- Swap-remove preserves other entity indices

**Example Proof Harness**:
```rust
#[kani::proof]
fn sparse_set_insert_get_consistency() {
    let mut set = SparseSet::new();
    let id: u32 = kani::any();
    kani::assume(id < 10000);  // Bound for tractability
    
    let entity = unsafe { Entity::from_raw(id as u64) };
    let idx = set.insert(entity);
    
    kani::assert(set.get(entity) == Some(idx), "Insert must return queryable index");
    kani::assert(set.contains(entity), "Contains must return true after insert");
}
```

#### 3. EntityAllocator (astraweave-ecs/src/entity_allocator.rs)

**Priority**: 🔴 CRITICAL  
**Lines**: 555

**Properties to Verify**:
- Generational indices always increment on despawn
- `is_alive` returns false for despawned entities
- Free list maintains valid entity IDs
- No entity ID reuse without generation increment
- `Entity::from_raw()` / `to_raw()` roundtrip is lossless

**Example Proof Harness**:
```rust
#[kani::proof]
fn entity_roundtrip_lossless() {
    let id: u32 = kani::any();
    let gen: u32 = kani::any();
    
    let entity = Entity::new(id, gen);
    let raw = entity.to_raw();
    let recovered = unsafe { Entity::from_raw(raw) };
    
    kani::assert(recovered.id() == id, "ID must survive roundtrip");
    kani::assert(recovered.generation() == gen, "Generation must survive roundtrip");
}
```

### Tier 2: SIMD/Math Correctness (High Priority)

#### 4. SIMD Vector Operations (astraweave-math/src/simd_vec.rs)

**Priority**: 🟠 HIGH  
**Lines**: 374  
**Unsafe Operations**: ~10 blocks (SSE2 intrinsics)

**Properties to Verify**:
- Dot product is symmetric: `dot(a, b) == dot(b, a)`
- Dot product with zero: `dot(a, ZERO) == 0`
- Cross product anticommutativity: `cross(a, b) == -cross(b, a)`
- Cross product orthogonality: `dot(cross(a, b), a) == 0`
- Normalize produces unit vectors: `length(normalize(v)) ≈ 1.0`
- SIMD matches scalar fallback (within epsilon)

**Example Proof Harness**:
```rust
#[kani::proof]
fn dot_product_symmetric() {
    let ax: f32 = kani::any();
    let ay: f32 = kani::any();
    let az: f32 = kani::any();
    let bx: f32 = kani::any();
    let by: f32 = kani::any();
    let bz: f32 = kani::any();
    
    kani::assume(!ax.is_nan() && !ay.is_nan() && !az.is_nan());
    kani::assume(!bx.is_nan() && !by.is_nan() && !bz.is_nan());
    
    let a = Vec3::new(ax, ay, az);
    let b = Vec3::new(bx, by, bz);
    
    let dot_ab = dot_simd(a, b);
    let dot_ba = dot_simd(b, a);
    
    kani::assert((dot_ab - dot_ba).abs() < 1e-6, "Dot product must be symmetric");
}
```

#### 5. SIMD Matrix Operations (astraweave-math/src/simd_mat.rs)

**Priority**: 🟠 HIGH  
**Lines**: ~400  
**Unsafe Operations**: ~8 blocks

**Properties to Verify**:
- Matrix multiply associativity: `(A × B) × C == A × (B × C)`
- Identity multiplication: `A × I == A`
- Transpose involution: `transpose(transpose(A)) == A`
- Transform point with identity: `transform(I, p) == p`

#### 6. SIMD Quaternion Operations (astraweave-math/src/simd_quat.rs)

**Priority**: 🟠 HIGH  
**Lines**: ~350  
**Unsafe Operations**: ~8 blocks

**Properties to Verify**:
- Quaternion normalization: `length(normalize(q)) ≈ 1.0`
- Identity quaternion: `mul(q, IDENTITY) == q`
- Slerp endpoints: `slerp(a, b, 0.0) == a` and `slerp(a, b, 1.0) == b`
- Dot product range: `-1.0 ≤ dot(a, b) ≤ 1.0` for unit quaternions

### Tier 3: FFI/C ABI Safety (High Priority)

#### 7. SDK C ABI Functions (astraweave-sdk/src/lib.rs)

**Priority**: 🟠 HIGH  
**Lines**: 597  
**Unsafe Operations**: ~20 blocks

**Properties to Verify**:
- `aw_version_string` never writes beyond buffer
- Null pointer checks before dereference
- CString conversions never panic
- World lifecycle (create/destroy) is leak-free
- Callback invocation is panic-safe

**Example Proof Harness**:
```rust
#[kani::proof]
fn version_string_no_buffer_overflow() {
    let len: usize = kani::any();
    kani::assume(len <= 1024);  // Reasonable bound
    
    let mut buf = vec![0u8; len];
    let required = unsafe { aw_version_string(buf.as_mut_ptr(), len) };
    
    if len > 0 {
        // Check null terminator is within bounds
        let null_pos = buf.iter().position(|&b| b == 0);
        kani::assert(null_pos.is_some(), "Must write null terminator");
        kani::assert(null_pos.unwrap() < len, "Null terminator must be in bounds");
    }
}
```

### Tier 4: Algorithm Correctness (Medium Priority)

#### 8. Archetype Component Storage (astraweave-ecs/src/archetype.rs)

**Priority**: 🟡 MEDIUM  
**Lines**: ~800

**Properties to Verify**:
- Component lookup returns correct type
- Entity → archetype mapping is consistent
- Archetype transitions preserve all components
- No component data corruption during moves

#### 9. Component Meta Registry (astraweave-ecs/src/component_meta.rs)

**Priority**: 🟡 MEDIUM

**Properties to Verify**:
- TypeId uniqueness per component type
- Layout alignment is power of 2
- Clone/drop function pointers are valid

### Tier 5: Mathematical Properties (Medium Priority)

#### 10. Physics Calculations (astraweave-physics)

**Priority**: 🟡 MEDIUM

**Properties to Verify**:
- Velocity integration doesn't cause NaN/Inf
- Position clamping respects world bounds
- Collision response conserves momentum (where applicable)

#### 11. Fluid Simulation (astraweave-fluids) — ⚠ REMOVED in W.1

> **⚠ STALENESS BANNER (added W-series W.2 Phase 2, 2026-06-21).** These targets
> verify the SPH solver — particle density, pressure, SPH-kernel normalization —
> which was **deleted** in W.1 (commit `1a57fdd41`; recovery tag
> `w0-pre-deprecation`). The general fluid *simulation* no longer exists; water is
> now a layered *rendering* system (`astraweave-render::WaterRenderer` + the
> `astraweave-water` `WaterQuery` facade). This section targets removed code — do
> not pursue. Current authority: `docs/campaigns/water-successor/`. (The other
> tiers/sections in this plan are unaffected.)

**Priority**: 🟡 MEDIUM

**Properties to Verify**:
- Particle density is always positive
- Pressure calculation doesn't overflow
- SPH kernel sums to 1.0 (normalization)

---

## Implementation Plan

### Phase 1: Environment Setup (Day 1)

**Duration**: 1-2 hours

1. **Install Kani**:
   ```powershell
   cargo install --locked kani-verifier
   cargo kani setup
   ```

2. **Platform Consideration**:
   - Kani officially supports Linux and macOS
   - Windows: Requires WSL2 for full functionality
   - Alternative: Use GitHub Actions for Kani CI

3. **Create Kani Configuration**:
   ```toml
   # Cargo.toml additions
   [dev-dependencies]
   kani = "0.x"  # Add Kani macros
   ```

### Phase 2: Proof Harness Development (Days 2-4)

**Duration**: 8-12 hours

#### Day 2: BlobVec & EntityAllocator (4 hours)
- Write 5-7 proof harnesses for BlobVec
- Write 3-4 proof harnesses for EntityAllocator
- Verify basic memory safety properties

#### Day 3: SparseSet & Math (4 hours)
- Write 3-4 proof harnesses for SparseSet
- Write 5-6 proof harnesses for SIMD operations
- Verify mathematical identities

#### Day 4: SDK FFI (2-4 hours)
- Write 4-5 proof harnesses for C ABI functions
- Verify buffer overflow protection
- Verify null pointer handling

### Phase 3: CI Integration (Day 5)

**Duration**: 2-4 hours

1. **Create GitHub Actions Workflow**:
   ```yaml
   # .github/workflows/kani.yml
   name: Kani Verification
   on: [push, pull_request]
   
   jobs:
     kani:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - uses: model-checking/kani-github-action@v1
           with:
             args: --tests
   ```

2. **Create Verification Script**:
   ```powershell
   # scripts/kani_verify.ps1
   $crates = @("astraweave-ecs", "astraweave-math", "astraweave-sdk")
   foreach ($crate in $crates) {
       cargo kani --package $crate --harness-timeout 300
   }
   ```

### Phase 4: Documentation & Maintenance (Day 5-6)

**Duration**: 2-3 hours

1. Update BULLETPROOF_VALIDATION_PLAN.md with Kani integration
2. Create KANI_VERIFICATION_REPORT.md with results
3. Add Kani badge to README.md

---

## Proof Harness Organization

### File Structure

```
astraweave-ecs/
└── src/
    ├── blob_vec.rs
    ├── blob_vec_kani.rs       # Kani proofs for BlobVec
    ├── sparse_set.rs
    ├── sparse_set_kani.rs     # Kani proofs for SparseSet
    ├── entity_allocator.rs
    └── entity_allocator_kani.rs

astraweave-math/
└── src/
    ├── simd_vec.rs
    ├── simd_vec_kani.rs       # Kani proofs for SIMD vectors
    ├── simd_mat.rs
    └── simd_mat_kani.rs

astraweave-sdk/
└── src/
    ├── lib.rs
    └── lib_kani.rs            # Kani proofs for FFI
```

### Proof Naming Convention

```rust
#[kani::proof]
fn <module>_<property>_<scenario>() {
    // Example: blob_vec_push_no_overflow()
    //          sparse_set_insert_idempotent()
    //          entity_roundtrip_lossless()
}
```

---

## Expected Outcomes

### Verification Coverage

| Crate | Properties | Proofs | Status |
|-------|------------|--------|--------|
| astraweave-ecs | 15-20 | 12-15 | Planned |
| astraweave-math | 10-12 | 8-10 | Planned |
| astraweave-sdk | 6-8 | 5-7 | Planned |
| **Total** | **31-40** | **25-32** | Planned |

### Success Criteria

- ✅ All proof harnesses pass (VERIFICATION:- SUCCESSFUL)
- ✅ No memory safety violations found
- ✅ Mathematical properties proven correct
- ✅ FFI boundaries verified safe
- ✅ CI integration operational

### Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Windows compatibility | Medium | Use WSL2 or GitHub Actions |
| Long verification times | Medium | Use `--harness-timeout`, bound inputs |
| Complex proofs timeout | High | Decompose into smaller properties |
| Floating-point precision | Medium | Use epsilon comparisons, not exact equality |

---

## Platform Considerations

### Windows Limitation

⚠️ **Kani does NOT have native Windows support.** 

Options:
1. **WSL2** (Recommended): Run Kani inside Windows Subsystem for Linux
2. **GitHub Actions**: Run Kani only in CI (Linux runner)
3. **Docker**: Use Kani Docker image

### WSL2 Setup

```powershell
# Install WSL2 if not already installed
wsl --install -d Ubuntu

# Inside WSL
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo install --locked kani-verifier
cargo kani setup
```

---

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| **Phase 1**: Setup | 1-2 hours | Kani installed, configured |
| **Phase 2**: Proofs | 8-12 hours | 25-32 proof harnesses |
| **Phase 3**: CI | 2-4 hours | GitHub Actions workflow |
| **Phase 4**: Docs | 2-3 hours | Reports, README update |
| **Total** | **13-21 hours** | Comprehensive verification |

---

## Next Steps

1. **Immediate**: Install Kani via WSL2 or configure GitHub Actions
2. **Week 1**: Develop proof harnesses for Tier 1 systems (BlobVec, SparseSet, EntityAllocator)
3. **Week 2**: Develop proof harnesses for Tier 2-3 systems (SIMD, SDK FFI)
4. **Week 3**: CI integration and documentation

---

## Conclusion

Kani verification will elevate AstraWeave from **Miri-validated** (no UB in tested paths) to **formally verified** (mathematical proofs of correctness).

**Combined Verification Stack**:
- ✅ **Miri**: Runtime UB detection (1,059 tests, 0 violations)
- 🎯 **Kani**: Formal correctness proofs (25-32 properties planned)
- ✅ **Tests**: 3,040+ passing tests (94.57% coverage)
- ✅ **Clippy**: Zero warnings across workspace

---

**Version**: 1.0.0  
**Date**: February 3, 2026  
**Status**: 📋 PLANNING - Ready for Implementation

*This plan is part of the AstraWeave AI-Native Gaming Engine project, developed entirely through AI collaboration with zero human-written code.*
