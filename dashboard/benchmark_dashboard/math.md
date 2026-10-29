---
layout: default
title: Math Subsystem
---

# SIMD Math (astraweave-math)

AstraWeave's math library provides SIMD-accelerated vector, matrix, and quaternion operations built on **glam** with SSE2 intrinsics and scalar fallbacks. The crate has **109 tests** validated under Miri with **zero undefined behavior**, plus Kani formal verification proofs.

## Modules

| Module | Lines | Description |
|--------|-------|-------------|
| `simd_vec` | 378 | Vec3 SIMD operations (SSE2 intrinsics) |
| `simd_mat` | 407 | Mat4 SIMD operations (SSE2 intrinsics) |
| `simd_quat` | 440 | Quaternion SIMD operations (SSE2 intrinsics) |
| `simd_movement` | 275 | Batch position updates (auto-vectorized) |
| `simd_vec_kani` | 279 | Kani formal verification proofs |
| `mutation_tests` | 1,346 | Mutation testing suite |

## Performance

| Operation | Module | Measured Speedup |
|-----------|--------|-----------------|
| Vec3 dot/cross/normalize | `simd_vec` | 2.1× over scalar |
| Mat4 multiply | `simd_mat` | 2.5× over scalar |
| Quaternion slerp | `simd_quat` | 1.75× over scalar |
| Batch position updates | `simd_movement` | 2.08× over scalar |

**Key insight**: glam auto-vectorization achieves 80–85% of hand-written AVX2 performance.

## API Reference

### simd_vec — Vec3 Operations

SSE2 intrinsics with compile-time `#[cfg(target_feature = "sse2")]` guard:

```rust
use astraweave_math::simd_vec::*;

let d = dot_simd(a, b);              // f32 dot product
let c = cross_simd(a, b);            // Vec3 cross product
let n = normalize_simd(v);           // normalized Vec3
let l = length_simd(v);              // magnitude
let l2 = length_squared_simd(v);     // squared magnitude
```

### simd_mat — Mat4 Operations

SSE2 intrinsics with runtime `is_x86_feature_detected!("sse2")` detection:

```rust
use astraweave_math::simd_mat::*;

let result = mul_simd(a, b);                  // Mat4 × Mat4
let t = transpose_simd(m);                    // transpose
let inv = inverse_simd(m);                    // inverse (glam delegate)
let p = transform_point_simd(m, point);       // Mat4 × Vec3 (homogeneous)
let pts = transform_points_batch(m, &points); // batch transform
```

### simd_quat — Quaternion Operations

SSE2 intrinsics with runtime detection:

```rust
use astraweave_math::simd_quat::*;

let q = mul_quat_simd(a, b);                 // Quat × Quat
let n = normalize_quat_simd(q);              // normalized Quat
let s = slerp_simd(a, b, 0.5);              // spherical interpolation
let d = dot_quat_simd(a, b);                // Quat dot product
let normed = normalize_batch(&quats);        // batch normalize
let slerped = slerp_batch(&pairs, t);        // batch slerp
```

### simd_movement — Batch Position Updates

Compiler auto-vectorized (no raw intrinsics) with batch-of-4 loop unrolling:

```rust
use astraweave_math::simd_movement::update_positions_simd;

// 2.08× faster than scalar at 10K entities
update_positions_simd(&mut positions[..], &velocities[..], dt);

// Also available: scalar baseline for comparison
update_positions_naive(&mut positions[..], &velocities[..], dt);
```

## SSE2 Fallback Strategy

All SIMD operations have scalar fallbacks for non-x86_64 targets:

| Module | Detection | Fallback |
|--------|-----------|----------|
| `simd_vec` | Compile-time `#[cfg(target_feature = "sse2")]` | Inline scalar math |
| `simd_mat` | Runtime `is_x86_feature_detected!` | glam native operations |
| `simd_quat` | Runtime `is_x86_feature_detected!` | glam native operations |
| `simd_movement` | Compiler auto-vectorization | Same code path (no intrinsics) |

## Formal Verification

### Miri (109 tests, 0 UB)

- SSE2 intrinsic safety via symbolic alignment checks
- Scalar fallback correctness against glam reference
- Edge cases: NaN, infinity, subnormal floats, zero vectors

### Kani Proofs (`simd_vec_kani.rs`)

Formal mathematical property verification:

| Property | Description |
|----------|-------------|
| Dot symmetry | `dot(a, b) == dot(b, a)` |
| Cross anticommutativity | `cross(a, b) == -cross(b, a)` |
| Normalization | `‖normalize(v)‖ ≈ 1.0` for non-zero v |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `glam` | workspace | Core Vec3/Mat4/Quat types |
| `bytemuck` | 1.x | Zero-copy transmutes |

## Best Practice: ECS Batching

**DO** (3–5× faster):
```rust
// Collect into contiguous buffer, then SIMD batch
let batch: Vec<_> = query.iter().collect();
update_positions_simd(&mut batch, &velocities, dt);
// Write back to ECS
```

**DON'T** (slow — archetype lookup per entity):
```rust
for entity in query.iter() {
    let pos = world.get_mut::<Position>(entity); // O(log n) lookup each time
    pos.x += vel.dx * dt;
}
```

Only parallelize workloads >5 ms (Rayon overhead ~50–100 µs).

[← Back to Home](index.html) · [Architecture](architecture.html) · [ECS](ecs.html)
