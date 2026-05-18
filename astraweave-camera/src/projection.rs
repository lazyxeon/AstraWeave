//! Projection types for canonical camera-system uploads.
//!
//! Conventions enforced by `docs/current/CAMERA_CONVENTIONS.md` §2.1 (fovy
//! radians), §2.2 (wgpu [0, 1] depth), §2.3 (`.max(0.01)` aspect floor), §2.6
//! (`Mat4::perspective_rh` only).

use glam::Mat4;

/// Perspective projection with both derived matrix and original parameters.
///
/// Carries the matrix and the values that produced it so consumers can:
///
/// - re-derive matrix variants (jittered for TAA, etc.) without recovering
///   parameters from the matrix;
/// - interrogate `fovy` / `aspect` / `znear` / `zfar` for shader uses like
///   depth linearization and FOV-aware post-effects;
/// - debug-inspect the projection without matrix decomposition.
///
/// Construction enforces conventions: `aspect` is floored at `.max(0.01)` per
/// §2.3 before producing the matrix, so a zero or NaN aspect does not
/// propagate to a NaN matrix.
///
/// Orthographic projection is deferred until a use case lands. The `Projection`
/// struct is `enum`-less by design — adding orthographic later is an additive
/// transition (likely by promoting `Projection` to an enum and offering
/// `Projection::perspective(...) -> Projection::Perspective { ... }`), not a
/// breaking change to the perspective path.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Projection {
    /// Projection matrix. View-space → clip-space.
    ///
    /// Produced by `Mat4::perspective_rh(fovy, aspect.max(0.01), znear, zfar)`
    /// per §2.6. wgpu-compatible `[0, 1]` depth range per §2.2.
    pub matrix: Mat4,

    /// Vertical field of view in radians. Per §2.1.
    pub fovy: f32,

    /// Aspect ratio (`width / height`). Stored pre-floor; `matrix` had
    /// `.max(0.01)` applied per §2.3. To get the actual aspect used in the
    /// matrix, callers can recompute `self.aspect.max(0.01)`.
    pub aspect: f32,

    /// Near plane distance (view-space, positive value). Per §2.2, must be `> 0`.
    pub znear: f32,

    /// Far plane distance (view-space, positive value). Per §2.2, must be
    /// `> znear`.
    pub zfar: f32,
}

impl Projection {
    /// Construct a perspective projection from canonical parameters.
    ///
    /// # Convention enforcement
    ///
    /// - §2.2: `znear` must be `> 0` and `zfar` must be `> znear`. Both are
    ///   `debug_assert`ed (panic in debug builds, silent in release — the
    ///   `.max(0.01)` aspect floor below is the only defense-in-depth check
    ///   that runs in release).
    /// - §2.3: `aspect` is floored at `.max(0.01)` before matrix construction.
    ///   The stored `self.aspect` field preserves the input value (callers can
    ///   inspect what was passed, even if it was zero or NaN).
    /// - §2.6: matrix is constructed via `Mat4::perspective_rh` — wgpu's
    ///   `[0, 1]` depth convention, never `perspective_rh_gl`'s `[-1, 1]`.
    pub fn perspective(fovy: f32, aspect: f32, znear: f32, zfar: f32) -> Self {
        debug_assert!(
            znear > 0.0,
            "Projection::perspective: znear must be > 0 per CAMERA_CONVENTIONS.md §2.2 (got {})",
            znear
        );
        debug_assert!(
            zfar > znear,
            "Projection::perspective: zfar must be > znear per CAMERA_CONVENTIONS.md §2.2 (got znear={}, zfar={})",
            znear,
            zfar
        );
        let matrix = Mat4::perspective_rh(fovy, aspect.max(0.01), znear, zfar);
        Self {
            matrix,
            fovy,
            aspect,
            znear,
            zfar,
        }
    }
}
