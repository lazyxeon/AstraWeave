//! Render view types — the canonical upload contract from camera producers to
//! the renderer.
//!
//! Conventions enforced by `docs/current/CAMERA_CONVENTIONS.md` §2.4 (right-
//! handed, +Y up), §2.8 (+X forward at yaw=0), §2.9 (`RenderView` is the
//! minimum upload contract).

use glam::{Mat4, Vec3};

use crate::Projection;

/// Canonical render-view upload contract.
///
/// Every camera producer (FreeFly, Orbit, Follow, Cinematic, Debug, and any
/// future producers) emits a `RenderView` via
/// [`crate::CameraProducer::to_render_view`]. The renderer consumes
/// `RenderView` exclusively — no per-producer-type renderer APIs (per §2.9).
///
/// # Camera-relative rendering
///
/// Camera-relative rendering is the **producer's responsibility**, not encoded
/// in `RenderView` field layout. A producer that wants camera-relative
/// rendering builds `view` with position pre-subtracted (camera at origin in
/// view-construction space) and reports `position` accordingly. Consumers
/// don't know whether the view is world-relative or camera-relative — that's
/// the producer's contract.
///
/// (UE5 carries both `ViewMatrix` and `TranslatedViewMatrix` simultaneously to
/// support both modes per draw. This design instead commits per producer call.
/// If a single producer needs to emit both flavors, it produces two
/// `RenderView`s.)
///
/// # Deferred fields
///
/// - **TAA jitter** (`unjittered_projection`, `unjittered_view_proj`,
///   `previous_view_proj`, jitter offsets): deferred per §2.7. No production
///   TAA in the current codebase. Will be added additively when TAA enters.
/// - **Basis vector fields** (`right`, `up`): deferred per Decision 3.
///   Shaders that need them extract from `inverse_view` columns:
///
///   | Vector | Source |
///   |---|---|
///   | `right` | `inverse_view.col(0).xyz` |
///   | `up` | `inverse_view.col(1).xyz` |
///   | `forward` (= `-view-space +Z`) | `-inverse_view.col(2).xyz` (also exposed as [`Self::view_dir`]) |
///
///   If shader-side column extraction proves costly, basis fields can be
///   added later — additive, not restructuring.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderView {
    /// World → view transformation (the conventional view matrix).
    ///
    /// Equivalent to `Mat4::look_to_rh(position, view_dir, Vec3::Y)` or
    /// `Mat4::look_at_rh(position, position + view_dir, Vec3::Y)` per §2.5.
    pub view: Mat4,

    /// View → clip transformation (the conventional projection matrix).
    ///
    /// Per §2.6, produced by `Mat4::perspective_rh`. wgpu `[0, 1]` depth.
    pub projection: Mat4,

    /// Precomputed `projection * view` for shader convenience.
    ///
    /// Shaders use this directly instead of multiplying per fragment. CPU
    /// computes it once; GPU consumes it many times.
    pub view_proj: Mat4,

    /// Inverse of `view`. Transforms view space back to world space.
    ///
    /// Camera world-space position appears in column 3 (use [`Self::position`]
    /// for direct access). Basis vectors appear in columns 0–2.
    pub inverse_view: Mat4,

    /// Inverse of `view_proj`. Reconstructs world-space positions from
    /// clip-space coordinates.
    ///
    /// Used in deferred shading, depth-buffer unprojection, and screen-space
    /// picking (canonical ray-from-screen path).
    pub inverse_view_proj: Mat4,

    /// Camera world-space position.
    ///
    /// Equivalent to `inverse_view.col(3).xyz` but precomputed for shader
    /// convenience. For camera-relative rendering, this is the producer's
    /// reported world position (the `view` matrix already has it subtracted).
    pub position: Vec3,

    /// Camera forward direction (unit vector, world space).
    ///
    /// At yaw=0, pitch=0, equals `Vec3::X` per §2.8. Equivalent to
    /// `-inverse_view.col(2).xyz` for right-handed cameras (the negation is
    /// because view space looks down `-Z` by convention, so world-space
    /// forward is the negation of view-space `+Z` column).
    pub view_dir: Vec3,

    /// Vertical field of view in radians. Mirrors [`Projection::fovy`].
    pub fovy: f32,

    /// Aspect ratio (`width / height`). Mirrors [`Projection::aspect`].
    pub aspect: f32,

    /// Near plane distance. Mirrors [`Projection::znear`].
    pub znear: f32,

    /// Far plane distance. Mirrors [`Projection::zfar`].
    pub zfar: f32,
}

impl RenderView {
    /// Construct a `RenderView` from a view matrix, a projection, a position,
    /// and a forward direction.
    ///
    /// Computes derived matrices (`view_proj`, `inverse_view`,
    /// `inverse_view_proj`) once on the CPU; shaders consume the precomputed
    /// versions.
    ///
    /// # Caller responsibilities
    ///
    /// - `position` should equal `inverse_view.col(3).xyz` (or the equivalent
    ///   for camera-relative rendering where position is subtracted from view
    ///   construction and reported separately).
    /// - `view_dir` should equal `-inverse_view.col(2).xyz`.
    /// - `projection` should already comply with §2.2/§2.3/§2.6 conventions
    ///   (constructed via [`Projection::perspective`] or equivalent).
    pub fn new(view: Mat4, projection: &Projection, position: Vec3, view_dir: Vec3) -> Self {
        let view_proj = projection.matrix * view;
        let inverse_view = view.inverse();
        let inverse_view_proj = view_proj.inverse();
        Self {
            view,
            projection: projection.matrix,
            view_proj,
            inverse_view,
            inverse_view_proj,
            position,
            view_dir,
            fovy: projection.fovy,
            aspect: projection.aspect,
            znear: projection.znear,
            zfar: projection.zfar,
        }
    }
}
