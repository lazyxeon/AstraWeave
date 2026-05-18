//! Canonical camera types for the AstraWeave engine.
//!
//! See [`docs/current/CAMERA_CONVENTIONS.md`](../../docs/current/CAMERA_CONVENTIONS.md)
//! for the authoritative convention reference. Any divergence between this
//! crate's behavior and the conventions doc is a bug in the crate, not the
//! doc.
//!
//! # Layering
//!
//! - [`Projection`] — perspective projection with derived matrix and original
//!   parameters (fovy, aspect, znear, zfar). Convention enforcement: aspect
//!   floored at `.max(0.01)` at matrix construction (§2.3); matrix uses
//!   `Mat4::perspective_rh` (§2.6); wgpu `[0, 1]` depth (§2.2).
//! - [`RenderView`] — the canonical upload contract from camera producers to
//!   the renderer. Carries forward and inverse matrices precomputed, position,
//!   view direction, and projection parameters. See §2.9.
//! - [`CameraProducer`] — minimal trait every camera producer implements; one
//!   method `to_render_view()`. See §2.9. The renderer's update path is NOT
//!   generic over this trait — it takes `&RenderView` directly.
//!
//! # Status (as of C.2)
//!
//! Types and trait exist; no production caller has migrated yet. C.3 migrates
//! engine `Camera` + `CameraController` (the FreeFly producer); C.4 migrates
//! editor `OrbitCamera`; C.5–C.7 handle remaining implementations per the
//! `CAMERA_CONVENTIONS.md` §3 migration tracking table.
//!
//! # Dependencies
//!
//! `glam` only (plus optional `serde` behind a feature flag). No `wgpu`
//! coupling — this crate produces pure math types; GPU upload is the
//! renderer crate's responsibility.

pub mod producer;
pub mod projection;
pub mod render_view;

pub use producer::CameraProducer;
pub use projection::Projection;
pub use render_view::RenderView;
