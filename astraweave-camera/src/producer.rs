//! Camera producer trait — the interface every camera type implements to
//! commit to a render view.

use crate::RenderView;

/// Trait implemented by every camera producer (FreeFly, Orbit, Follow,
/// Cinematic, Debug) to commit to a [`RenderView`] for renderer upload.
///
/// # Design
///
/// The trait is intentionally minimal:
///
/// - One method: `to_render_view(&self) -> RenderView`.
/// - No associated types, no producer-specific methods.
/// - Producer-specific ergonomic surfaces (orbit-camera picking, follow-rig
///   target accessors, free-fly mouse handlers) live on the concrete types,
///   not on this trait.
///
/// # Renderer integration
///
/// The renderer's update path is **NOT** generic over this trait — it takes
/// `&RenderView` directly. This trait exists for caller-side polymorphism:
///
/// - a camera manager holding `Box<dyn CameraProducer>`;
/// - a cinematics blender combining two producers' views into one
///   intermediate `RenderView`;
/// - tests exercising multiple producers against the same renderer.
///
/// Per §2.9, the renderer's update contract is `update_view(&RenderView)` (or
/// equivalent). Callers convert producer → `RenderView` via this trait, then
/// hand the result to the renderer.
///
/// # Multi-view scenarios
///
/// Multi-view scenarios (one producer producing N views for shadow cascades,
/// cubemap faces, or split-screen) can be addressed by adding
/// `to_render_views(&self) -> Vec<RenderView>` (or a `SmallVec` for stack
/// allocation) as an additive trait method — not currently in scope. The
/// shadow CSM cascade computation currently derives subview matrices inside
/// the renderer from a single primary `RenderView`; that stays unchanged
/// through C.9.
pub trait CameraProducer {
    /// Commit the producer's current state to a [`RenderView`].
    ///
    /// Camera-relative vs world-relative rendering is the producer's
    /// decision (encoded in producer state or by separate methods on the
    /// concrete type). Consumers don't see the difference.
    fn to_render_view(&self) -> RenderView;
}
