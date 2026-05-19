//! Camera types — backward-compatibility shim.
//!
//! The canonical types now live in `astraweave-camera`. This shim re-exports
//! them under their historical names to preserve compile-time compatibility
//! for callers that haven't migrated to the canonical paths yet.
//!
//! This shim exists for the duration of the Unified Camera campaign:
//!
//! - **C.3.A (this commit)**: shim created; both old and new APIs functional.
//! - **C.3.B**: every caller of the deprecated `Renderer::update_camera` /
//!   `Renderer::update_camera_matrices` migrates to `Renderer::update_view`.
//! - **C.3.C**: this shim is deleted, alongside the deprecated APIs.
//!
//! Do not add new callers that depend on this shim — import directly from
//! `astraweave_camera` instead. The `#[deprecated]` attributes on the legacy
//! `Renderer` methods make migration-required call sites visible at compile
//! time.
//!
//! See `docs/current/CAMERA_CONVENTIONS.md` for the canonical convention
//! reference.

pub use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};
