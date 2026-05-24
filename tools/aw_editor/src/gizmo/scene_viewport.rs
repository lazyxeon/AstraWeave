//! Scene viewport types.
//!
//! Post-C.6.A (Unified Camera campaign), this module retains only the
//! [`Transform`] type used by `transform_panel`, `scene_state`, and the
//! mutation-resistant tests. The pre-C.6.A `CameraController` and
//! `SceneViewport` types were dormant in production (declared by
//! `TransformPanel.camera` but never read; zero `SceneViewport`
//! production callers) per C.5 audit finding L.5.1 and were deleted
//! along with their benchmarks.
//!
//! The module name `scene_viewport` is preserved for now to keep the
//! existing `Transform` import paths stable; if a future sub-phase
//! relocates `Transform`, the rename should land separately to keep
//! C.6.A's deletion proof shape pure.

use glam::{Mat4, Quat, Vec3};

/// Transform state for ECS entity.
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Get transform matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}
