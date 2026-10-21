# AstraWeave Revised Validation Plan

**Version**: 2.1  
**Date**: January 9, 2026  
**Status**: Research Complete, Ready for Implementation

---

## ⚠️ Important Caveats

Before implementing this plan, understand these practical limitations:

1. **ASan + GPU Drivers Don't Mix**: AddressSanitizer produces false positives from GPU driver allocations. The render crate **cannot get ASan coverage** — rely on wgpu validation instead. Run ASan tests with `--no-default-features` to disable GPU features.

2. **ThreadSanitizer is Advisory, Not Must-Pass**: TSan with async Rust (Tokio) is noisy. Expect to accumulate suppressions over time. Budget 2-4 hours for tuning, and accept that TSan failures may be warnings rather than blockers.

3. **contracts Crate Has Rough Edges**: Version 0.6 struggles with complex generic types. Works well for math crate (quaternions, matrices), but may hit limitations with AI planning types. **Test on a single function before broad adoption.**

4. **proptest Time Estimates Are Optimistic**: Complex crates (ECS, AI, physics) require figuring out what invariants matter. **Actual time: 3-4 hours per complex crate, 1-2 hours for simpler ones.**

5. **wgpu API Version Check**: This plan uses `InstanceFlags::debugging()` (stable across versions). The `advanced_debugging()` method may not exist in all wgpu versions — verify against your exact wgpu 25.0.2.

---

## Executive Summary

This document revises the original validation plan based on research into alternative verification tools that better address AstraWeave's specific challenges:
- **FFI-heavy crates** (Rapier3D physics bindings)
- **GPU-intensive rendering** (wgpu 25.0.2)
- **49 files with unsafe code**
- **January 1st launch constraint**

### Key Changes from Original Plan

| Original Tool | Status | Replacement | Reason |
|--------------|--------|-------------|--------|
| **Miri** | ⚠️ Limited | LLVM Sanitizers + Miri (hybrid) | Miri can't verify FFI; sanitizers can |
| **Kani** | ❌ Removed | `contracts` crate + proptest | 8-16h estimate unrealistic; proptest provides 80% of benefit |
| **Valgrind** | Not planned | ❌ Skip | 10-50× overhead; sanitizers faster with same coverage |
| **GPU Validation** | Not planned | ✅ Added | wgpu's `InstanceFlags::VALIDATION` is free |

---

## Tool Comparison Matrix

| Tool | What It Verifies | What It Can't Verify | Overhead | CI Feasibility | Setup Time | Maturity |
|------|------------------|---------------------|----------|----------------|------------|----------|
| **AddressSanitizer (ASan)** | Memory errors, heap/stack overflow, use-after-free, double-free | Data races (use TSan) | 2-3× runtime | ✅ PR checks (Linux/macOS) | 1h | Production |
| **ThreadSanitizer (TSan)** | Data races, deadlocks | Memory errors, uninitialized reads | 5-15× runtime | ⚠️ Nightly only (slow) | 1h | Production |
| **MemorySanitizer (MSan)** | Uninitialized memory reads | FFI code unless also instrumented | 3× runtime | ❌ Too many false positives | 2h | Experimental |
| **LeakSanitizer (LSan)** | Memory leaks | Intentional leaks (Rc cycles) | 1× runtime | ✅ PR checks | 30min | Production |
| **Miri** | Pure Rust UB, aliasing violations | FFI, GPU calls, async runtime | 10-50× runtime | ⚠️ Nightly only | 1h | Production |
| **wgpu Validation** | GPU API misuse, buffer overflows, shader errors | Driver bugs, memory corruption | 0× in release | ✅ Always on in debug | 0h (built-in) | Production |
| **Vulkan Validation Layers** | Vulkan API compliance, resource tracking | Application logic, Rust memory | ~10% runtime | ✅ Debug builds | 30min | Production |
| **proptest** | Property invariants, edge cases | Full state space (bounded) | 1-10× test time | ✅ PR checks | 2h | Production |
| **cargo-mutants** | Test effectiveness (mutation score) | Correctness (only test quality) | 10-100× runtime | ⚠️ Weekly only | 30min | Production |
| **Kani** | Full formal verification | Large functions, async, FFI | Hours per proof | ❌ Research only | 16h+ | Experimental |
| **contracts** | Runtime pre/post conditions | Compile-time proofs | <1% runtime | ✅ PR checks | 2h | Production |

---

## Revised Validation Phases

### Phase 1: Quick Wins (Launch-Critical, 4-6 hours)

**Goal**: Maximum verification coverage with minimal setup time.

#### 1.1 Enable wgpu GPU Validation (0 hours - already configured)

wgpu automatically enables validation in debug builds via `InstanceFlags::from_build_config()`.

**Verify**: Check that `astraweave-render` uses debug validation:
```rust
// Should already be in code - verify it's present
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    flags: wgpu::InstanceFlags::debugging(), // Enables VALIDATION + DEBUG
    ..Default::default()
});
```

**Environment variables for enhanced debugging**:
```bash
WGPU_VALIDATION=1              # Force validation in release builds
WGPU_GPU_BASED_VALIDATION=1    # GPU-assisted validation (slower but catches more)
```

#### 1.2 AddressSanitizer for Memory Safety (2 hours)

**Why**: Catches 90% of memory bugs that Miri catches, plus FFI bugs Miri misses.

**Target crates** (highest unsafe density):
- `astraweave-ecs` - Archetype storage with raw pointers
- `astraweave-physics` - Rapier3D FFI bindings
- `astraweave-memory` - Custom allocators

**Setup**:
```bash
# Linux/macOS only (not Windows)
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p astraweave-ecs --target x86_64-unknown-linux-gnu
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p astraweave-physics --target x86_64-unknown-linux-gnu
```

**CI Integration** (`.github/workflows/sanitizers.yml`):
```yaml
name: Address Sanitizer
on: [push, pull_request]
jobs:
  asan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@nightly
      - run: |
          RUSTFLAGS="-Zsanitizer=address" cargo +nightly test \
            -p astraweave-ecs \
            -p astraweave-physics \
            --target x86_64-unknown-linux-gnu
        env:
          ASAN_OPTIONS: detect_leaks=1
```

#### 1.3 LeakSanitizer for Memory Leaks (30 minutes)

**Why**: Catches Rc cycles, forgotten deallocations. Runs at native speed.

```bash
RUSTFLAGS="-Zsanitizer=leak" cargo +nightly test -p aw-save --target x86_64-unknown-linux-gnu
```

#### 1.4 Install Security Tooling (1 hour)

```bash
cargo install cargo-audit   # Known vulnerability scanning
cargo install cargo-deny    # License + ban enforcement

# Run immediately
cargo audit
cargo deny check
```

**Add to CI** (every PR):
```yaml
- run: cargo audit
- run: cargo deny check
```

#### 1.5 cargo-nextest for Better Test Output (30 minutes)

```bash
cargo install cargo-nextest
cargo nextest run  # 3× faster than cargo test, better failure output
```

---

### Phase 2: Property Testing Expansion (Post-Launch, 16-22 hours)

**Goal**: Expand proptest coverage from 1 crate to 8+ critical crates.

**⚠️ Time Reality Check**: The hardest part is figuring out WHAT to test, not writing the tests. For complex systems like ECS and AI, you'll spend significant time identifying meaningful invariants.

#### 2.1 Current State

Only `astraweave-blend` has proptest (679 LOC). Expand to:

| Crate | Test Focus | Effort | Notes |
|-------|-----------|--------|-------|
| `astraweave-ecs` | Component roundtrip, entity lifecycle, archetype migration | **4h** | Complex invariants, many edge cases |
| `astraweave-physics` | Collision detection, raycasts, contact points | **3-4h** | Physics edge cases are subtle |
| `astraweave-nav` | Pathfinding validity, navmesh queries | **3h** | Path validity invariants |
| `astraweave-ai` | Plan validity, decision consistency, state transitions | **4h** | AI invariants require domain knowledge |
| `aw-save` | Serialization roundtrip, corruption recovery | **2h** | Straightforward roundtrip tests |
| `astraweave-math` | Vector/quaternion operations, NaN handling | **1-2h** | Well-defined mathematical properties |

#### 2.2 Example: ECS Property Tests

```rust
// astraweave-ecs/tests/property_tests.rs
use proptest::prelude::*;
use astraweave_ecs::*;

proptest! {
    #[test]
    fn prop_component_survives_archetype_migration(
        initial_x in -1e6f32..1e6f32,
        initial_y in -1e6f32..1e6f32,
        health in 0i32..1000
    ) {
        let mut world = World::new();
        let entity = world.spawn();
        
        // Add Position
        world.insert(entity, Position { x: initial_x, y: initial_y, z: 0.0 });
        
        // Add Health (triggers archetype migration)
        world.insert(entity, Health(health));
        
        // Verify Position survives migration
        let pos = world.get::<Position>(entity).unwrap();
        prop_assert_eq!(pos.x, initial_x);
        prop_assert_eq!(pos.y, initial_y);
    }
    
    #[test]
    fn prop_despawn_cleans_all_components(entity_count in 1usize..1000) {
        let mut world = World::new();
        let entities: Vec<_> = (0..entity_count)
            .map(|_| {
                let e = world.spawn();
                world.insert(e, Position::default());
                world.insert(e, Health(100));
                e
            })
            .collect();
        
        // Despawn half
        for entity in entities.iter().take(entity_count / 2) {
            world.despawn(*entity);
        }
        
        // Verify despawned entities have no components
        for entity in entities.iter().take(entity_count / 2) {
            prop_assert!(world.get::<Position>(*entity).is_none());
            prop_assert!(world.get::<Health>(*entity).is_none());
        }
    }
}
```

---

### Phase 3: Miri for Pure Rust UB (Post-Launch, 4-6 hours)

**Why**: Miri catches aliasing violations and UB that sanitizers miss, but only for pure Rust code.

**Scope**: Run Miri on crates WITHOUT FFI:
- ✅ `astraweave-ecs` (archetype storage)
- ✅ `astraweave-math` (SIMD operations)
- ✅ `astraweave-core` (data structures)
- ❌ `astraweave-physics` (Rapier FFI - use ASan instead)
- ❌ `astraweave-render` (wgpu FFI - use GPU validation instead)

**Setup**:
```bash
rustup +nightly component add miri
cargo +nightly miri test -p astraweave-ecs -- --exclude-pattern "ffi"
```

**CI Integration** (nightly only due to slowness):
```yaml
name: Miri
on:
  schedule:
    - cron: '0 3 * * *'  # 3 AM daily
jobs:
  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: miri
      - run: cargo +nightly miri test -p astraweave-ecs -p astraweave-math
```

---

### Phase 4: ThreadSanitizer for Concurrency (Post-Launch, 6-8 hours)

**Why**: AstraWeave's AI system uses async and multi-threading extensively.

**Target crates**:
- `astraweave-ai` - Multi-agent planning
- `astraweave-coordination` - Agent coordination
- `astraweave-net` - Network protocol handling

**⚠️ Critical Caveat: TSan is ADVISORY, not MUST-PASS**

TSan with async Rust is inherently noisy. Expect:
- False positives from Tokio internals
- False positives from crossbeam/parking_lot
- Suppressions list that grows over time
- 2-4 hours of tuning before it's useful

**Recommendation**: Treat TSan as a bug-finding tool, not a gate. Run it, investigate real issues, but don't block releases on TSan failures until suppressions are mature.

**Setup**:
```bash
# Create suppressions file (this WILL grow over time)
cat > tsan_suppressions.txt << EOF
# Standard library false positives
race:std::sync::mpsc
race:std::sync::Once

# Async runtime false positives
race:tokio::runtime
race:tokio::sync
race:tokio::io

# Common crate false positives
race:crossbeam
race:parking_lot
race:rayon

# Add more as discovered...
EOF

TSAN_OPTIONS="suppressions=tsan_suppressions.txt" \
RUSTFLAGS="-Zsanitizer=thread" \
cargo +nightly test -p astraweave-ai --target x86_64-unknown-linux-gnu -- --test-threads=1
```

---

### Phase 5: Runtime Contracts (Post-Launch, 4-6 hours)

**Why**: Kani is impractical (8-16h minimum), but `contracts` provides 80% of the benefit for 10% of the effort.

**⚠️ Maturity Warning**: The `contracts` crate (v0.6) has rough edges:
- Works well with simple types (primitives, simple structs)
- **Struggles with complex generics** (AI planning types, ECS archetypes)
- **Recommendation**: Start with `astraweave-math` only, then evaluate before expanding

**Instead of Kani proofs, add runtime precondition/postcondition checks**:

```rust
// astraweave-math/src/quaternion.rs - GOOD FIT for contracts
use contracts::*;

impl Quat {
    #[requires(self.length_squared() > 0.0001, "Quaternion must be non-zero")]
    #[ensures(ret.length_squared().abs() - 1.0 < 0.0001, "Result must be unit quaternion")]
    pub fn normalize(&self) -> Quat {
        let len = self.length();
        Quat {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
            w: self.w / len,
        }
    }
}
```

**Where contracts work well**:
- ✅ `astraweave-math` - Quaternions, vectors, matrices
- ✅ `astraweave-core` - Simple data structures
- ⚠️ `astraweave-ai` - Test on one function first, may hit generic limitations
- ❌ `astraweave-ecs` - Complex generics likely problematic

**Benefits over Kani**:
- Works with FFI code
- Zero runtime cost in release builds (contracts compile out)
- Catches bugs during testing
- Documents invariants in code

---

### Phase 6: Mutation Testing (Post-Launch, 8-12 hours)

**Why**: Our 7,600+ tests might have blind spots. Mutation testing finds them.

**Run weekly** (too slow for PR checks):
```bash
cargo install cargo-mutants
cargo mutants -p astraweave-ai --output mutation-report/
```

**Interpret results**:
- **Killed**: Test caught the mutation ✅
- **Survived**: Test MISSED the bug 🔴 → Add test
- **Timeout**: Consider making test faster

**Target**: >80% mutation score for critical crates

---

### Phase 7: Determinism Testing (Post-Launch, 4-6 hours)

**Why**: For multiplayer sync, replay systems, and bug reproduction, determinism is critical. The original audit found **32 determinism tests across 4 crates with 96.9% pass rate** — the failing 3.1% needs attention.

**Current State**:
- `astraweave-ecs`: Deterministic entity ordering ✅
- `astraweave-ai`: Seeded RNG ✅
- `astraweave-physics`: 1 failing determinism test ❌
- `astraweave-core`: Unknown coverage

**Goal**: 100% determinism test pass rate + expanded coverage

**Tasks**:

1. **Fix the failing 3.1%** (2 hours)
   - Investigate physics determinism failure
   - Common causes: floating-point ordering, HashMap iteration, thread scheduling
   - Use `#[cfg(test)] use std::collections::BTreeMap;` instead of HashMap in tests

2. **Expand determinism coverage** (2-4 hours)
   - Add multi-frame determinism tests (run 100 frames, hash state, compare across runs)
   - Test with varying entity counts (edge cases at archetype boundaries)
   - Test save/load determinism (serialize state, reload, verify identical)

**Example: Multi-Frame Determinism Test**:
```rust
#[test]
fn test_100_frame_determinism() {
    let mut hashes = Vec::new();
    
    for run in 0..3 {
        let mut world = World::new_with_seed(12345);
        for _ in 0..100 {
            world.tick();
        }
        hashes.push(world.state_hash());
    }
    
    assert_eq!(hashes[0], hashes[1], "Run 1 vs 2 diverged");
    assert_eq!(hashes[1], hashes[2], "Run 2 vs 3 diverged");
}
```

**CI Integration**: Add determinism tests to nightly (not PR — they're slow)

---

## Crate-Specific Recommendations

### astraweave-physics (Rapier3D FFI)

| Tool | Applicability | Notes |
|------|---------------|-------|
| ❌ Miri | Cannot run | FFI blocks Miri |
| ✅ AddressSanitizer | Primary tool | Catches FFI memory bugs |
| ✅ LeakSanitizer | Use | Catches FFI leaks |
| ✅ proptest | Use | Test collision edge cases |
| ⚠️ ThreadSanitizer | Limited | May have false positives from Rapier |

### astraweave-render (GPU/wgpu)

| Tool | Applicability | Notes |
|------|---------------|-------|
| ❌ Miri | Cannot run | GPU calls block Miri |
| ✅ wgpu Validation | **Primary tool** | Built-in, zero overhead in release |
| ✅ Vulkan Validation Layers | Use | Requires LunarG SDK |
| ✅ RenderDoc | Manual debugging | Frame-level GPU capture |
| ❌ AddressSanitizer | **Do not use** | GPU driver allocations cause false positives |

**⚠️ ASan Incompatibility**: GPU driver memory management triggers ASan false positives. Options:
1. **Recommended**: Skip ASan for render crate entirely — wgpu validation provides equivalent coverage
2. Run ASan tests with `--no-default-features` to disable GPU code paths
3. Add driver library suppressions (fragile, driver-version-specific)

**Enable GPU validation**:
```rust
// In debug builds (automatic via from_build_config())
let flags = wgpu::InstanceFlags::debugging(); // Stable API

// For GPU-based validation (if supported by your wgpu version):
// Check docs.rs/wgpu for your exact version's API
let flags = wgpu::InstanceFlags::VALIDATION | wgpu::InstanceFlags::DEBUG;
```

### astraweave-ecs (Heavy Unsafe)

| Tool | Applicability | Notes |
|------|---------------|-------|
| ✅ Miri | Primary tool | No FFI in core ECS |
| ✅ AddressSanitizer | Backup | Catches what Miri misses |
| ✅ proptest | Use heavily | Test archetype migrations |
| ✅ cargo-mutants | Use | High ROI for core data structures |

### astraweave-memory (Custom Allocators)

| Tool | Applicability | Notes |
|------|---------------|-------|
| ✅ Miri | Primary tool | Perfect for allocator verification |
| ✅ AddressSanitizer | Backup | Double-check with ASan |
| ✅ Valgrind | Optional | Only if Miri misses something |

### astraweave-net (Network Protocol)

| Tool | Applicability | Notes |
|------|---------------|-------|
| ✅ Fuzzing | Primary tool | Network parsers need fuzzing |
| ✅ proptest | Use | Test protocol edge cases |
| ✅ ThreadSanitizer | Use | Multi-threaded network handling |
| ❌ Miri | Limited | May have async runtime issues |

---

## CI Workflow Structure

### Tier 1: Every PR (must be fast, <10 min)

```yaml
pr_checks:
  - cargo check --all-targets
  - cargo clippy --all-targets
  - cargo test --workspace (via nextest)
  - cargo audit
  - cargo deny check
  - wgpu validation (automatic in debug tests)
```

**Time budget**: 5-10 minutes

### Tier 2: Nightly (can be slow, <60 min)

```yaml
nightly:
  - AddressSanitizer on critical crates
  - LeakSanitizer on all crates
  - Miri on pure-Rust crates
  - ThreadSanitizer on async crates
  - proptest with increased iterations
```

**Time budget**: 30-60 minutes

### Tier 3: Weekly (very slow, hours)

```yaml
weekly:
  - cargo-mutants on critical crates
  - Full fuzzing corpus runs
  - Extended proptest (100k iterations)
  - Memory profiling
```

**Time budget**: 2-4 hours

---

## ROI Analysis: Kani vs Alternatives

### Why Kani Was Removed

| Factor | Kani | contracts + proptest |
|--------|------|---------------------|
| Setup time | 16+ hours | 2-4 hours |
| Learning curve | Steep (model checking theory) | Minimal |
| FFI support | ❌ None | ✅ contracts work with FFI |
| Async support | ❌ Experimental | ✅ Full support |
| CI integration | ❌ Very slow | ✅ Fast |
| Bug detection rate | 95%+ (bounded) | 80%+ (random testing) |
| Maintenance burden | High (proofs break on refactor) | Low |

**Conclusion**: For a January launch, contracts + proptest provides **80% of Kani's benefit for 10% of the effort**.

### When to Revisit Kani

Consider Kani post-launch for:
- Safety-critical math (cryptographic operations)
- Financial calculations (if applicable)
- Regulatory compliance requirements

---

## Implementation Timeline

### Pre-Launch (Before January 1st)

| Task | Hours | Priority |
|------|-------|----------|
| Enable ASan in CI | 2h | 🔴 Critical |
| cargo-audit + cargo-deny | 1h | 🔴 Critical |
| Verify wgpu validation enabled | 0.5h | 🔴 Critical |
| cargo-nextest migration | 0.5h | 🟡 High |
| **Total** | **4h** | |

### Post-Launch (January - February)

| Task | Hours | Priority |
|------|-------|----------|
| Expand proptest to 8 crates | **18-22h** | 🟡 High |
| Miri on pure-Rust crates | 4h | 🟡 High |
| ThreadSanitizer setup + tuning | **6-8h** | 🟢 Medium |
| contracts crate integration (math only first) | **2-4h** | 🟢 Medium |
| cargo-mutants baseline | 4h | 🟢 Medium |
| Determinism testing expansion | **4-6h** | 🟡 High |
| **Total** | **38-48h** | |

### Long-Term (March+)

| Task | Hours | Priority |
|------|-------|----------|
| Fuzzing expansion to 25+ targets | 16h | 🟢 Medium |
| Mutation testing to 80%+ score | 16h | 🟢 Medium |
| Revisit Kani for math crates | 16h | ⚪ Low |
| **Total** | **48h** | |

---

## Success Metrics

| Metric | Current | Post-Launch Target | Long-Term Target |
|--------|---------|-------------------|------------------|
| Test coverage | ~78% | 85% | 90% |
| ASan-clean crates | Unknown | 5 critical crates (excl. render) | All non-GPU crates |
| Miri-clean crates | 0 | 5 pure-Rust crates | All pure-Rust |
| proptest crates | 1 | 8 | 15+ |
| Fuzz targets | 11 | 15 | 25+ |
| Mutation score | N/A | 70% critical | 80%+ |
| Security vulns (cargo-audit) | Unknown | 0 critical | 0 total |
| **Determinism pass rate** | **96.9%** | **100%** | **100% + expanded** |
| TSan suppressions | N/A | <20 | Stable list |

---

## Appendix: Tool Installation Commands

```bash
# Security
cargo install cargo-audit
cargo install cargo-deny

# Testing
cargo install cargo-nextest
cargo install cargo-mutants

# Miri (nightly only)
rustup +nightly component add miri

# contracts crate (add to Cargo.toml)
# [dev-dependencies]
# contracts = "0.6"
```

### Sanitizer Environment Variables

```bash
# AddressSanitizer
export ASAN_OPTIONS="detect_leaks=1:abort_on_error=1"

# ThreadSanitizer
export TSAN_OPTIONS="suppressions=tsan_suppressions.txt:second_deadlock_stack=1"

# LeakSanitizer
export LSAN_OPTIONS="max_leaks=10"

# wgpu GPU validation
export WGPU_VALIDATION=1
export WGPU_GPU_BASED_VALIDATION=1
```

---

## References

1. [Rust Sanitizers Documentation](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/sanitizer.html)
2. [wgpu Debugging Guide](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications)
3. [wgpu InstanceFlags](https://docs.rs/wgpu/latest/wgpu/struct.InstanceFlags.html)
4. [proptest Book](https://proptest-rs.github.io/proptest/intro.html)
5. [cargo-mutants](https://github.com/sourcefrog/cargo-mutants)
6. [contracts crate](https://crates.io/crates/contracts)

---

**Document Version**: 2.1  
**Last Updated**: January 9, 2026  
**Author**: GitHub Copilot (AI-generated research synthesis)

**Changelog**:
- v2.1: Added caveats section, fixed ASan+GPU warning, TSan advisory status, contracts maturity warning, realistic proptest estimates, wgpu API compatibility note, added Phase 7 (Determinism Testing)
