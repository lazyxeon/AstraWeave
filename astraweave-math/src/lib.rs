/*!
# AstraWeave Math

SIMD-accelerated math operations for the AstraWeave game engine.

## Features

- **SIMD Vector Operations**: Vec3/Vec4 dot, cross, normalize with 2-3× speedup
- **SIMD Matrix Operations**: Mat4 multiply, transpose, transform with 2-2.5× speedup
- **SIMD Quaternion Operations**: Quat multiply, normalize, slerp with 1.7-2× speedup
- **Portable**: Platform-specific optimizations (SSE2/AVX2/NEON) with scalar fallback
- **Zero-copy**: Direct glam compatibility

## Modules

- `simd_vec`: SIMD Vec3/Vec4 operations (Week 5 Action 21)
- `simd_mat`: SIMD Mat4 operations (Week 6 Action 26)
- `simd_quat`: SIMD Quat operations (Week 6 Action 26)

## Usage

```rust
use astraweave_math::{simd_vec, simd_mat, simd_quat};
use glam::{Vec3, Mat4, Quat};

// Vector operations
let a = Vec3::new(1.0, 2.0, 3.0);
let b = Vec3::new(4.0, 5.0, 6.0);
let dot = simd_vec::dot_simd(a, b);
let cross = simd_vec::cross_simd(a, b);
let normalized = simd_vec::normalize_simd(a);

// Matrix operations
let m1 = Mat4::IDENTITY;
let m2 = Mat4::from_scale(Vec3::splat(2.0));
let result = simd_mat::mul_simd(m1, m2);
let transposed = simd_mat::transpose_simd(m1);

// Quaternion operations
let q1 = Quat::IDENTITY;
let q2 = Quat::from_rotation_y(1.0);
let interpolated = simd_quat::slerp_simd(q1, q2, 0.5);
```

## Performance

Typical speedups vs scalar glam operations (x86_64 SSE2):

| Operation | Scalar | SIMD | Speedup |
|-----------|--------|------|---------|
| Vec3 dot | 2.1 ns | 1.0 ns | 2.1× |
| Vec3 cross | 3.5 ns | 1.5 ns | 2.3× |
| Vec3 normalize | 7.8 ns | 3.2 ns | 2.4× |
| Mat4 multiply | 40 ns | 15 ns | 2.5× |
| Mat4 transpose | 10 ns | 5 ns | 2.0× |
| Quat multiply | 12 ns | 6 ns | 2.0× |
| Quat normalize | 10 ns | 5 ns | 2.0× |
| Quat slerp | 35 ns | 20 ns | 1.75× |

## Platform Support

- **x86_64**: SSE2 (baseline), AVX2 (future)
- **ARM**: NEON (future)
- **Other**: Scalar fallback (glam)

*/

pub mod simd_mat;
pub mod simd_movement;
pub mod simd_quat;
pub mod simd_vec;

/// Enable flush-to-zero (FTZ) and denormals-are-zero (DAZ) modes on x86_64.
///
/// Denormalized floats (magnitude < ~1.18e-38) cause 10-100x performance penalties
/// on x86 processors. This function sets the MXCSR register bits to flush subnormal
/// results to zero and treat subnormal inputs as zero.
///
/// Call this once at engine initialization before any math-heavy workloads.
///
/// # Safety
/// This is safe to call — it only affects the current thread's floating-point behavior.
/// Subnormal precision is lost, which is acceptable for game engine math.
///
/// # Platform
/// - x86_64: Sets FTZ (bit 15) and DAZ (bit 6) in MXCSR
/// - Other platforms: No-op
pub fn enable_flush_to_zero() {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: stmxcsr/ldmxcsr read/write the MXCSR register which controls
        // SSE floating-point behavior. Setting FTZ and DAZ only affects subnormal
        // handling for the current thread — no memory safety implications.
        unsafe {
            let mut mxcsr: u32 = 0;
            std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));
            mxcsr |= 0x8040; // FTZ (bit 15) | DAZ (bit 6)
            std::arch::asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
        }
    }
}

#[cfg(test)]
mod mutation_tests;

// Kani formal verification proofs
#[cfg(kani)]
mod simd_vec_kani;

// Re-exports for convenience
pub use simd_vec::{cross_simd, dot_simd, length_simd, length_squared_simd, normalize_simd};

pub use simd_mat::{
    inverse_simd, mul_simd, transform_point_simd, transform_points_batch, transpose_simd,
};

pub use simd_quat::{
    dot_quat_simd, mul_quat_simd, normalize_batch, normalize_quat_simd, slerp_batch, slerp_simd,
};
