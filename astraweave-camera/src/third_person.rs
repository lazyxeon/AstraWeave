//! Third-person follow camera.
//!
//! The eye sits `distance` behind [`FreeFly::dir`] so yaw 0 / pitch 0 looks
//! along +X, matching the free-fly convention. `CameraProducer::to_render_view`
//! is world-relative, the same split as [`FreeFly`]: the default renderer
//! upload does not offset geometry. [`ThirdPersonCamera::to_render_view_camera_relative`]
//! is the opt-in large-world form.

use glam::{Mat4, Vec3};

use crate::freefly::FreeFly;
use crate::{CameraProducer, Projection, RenderView};

/// Near plane of the Aravath third-person camera. The heavy-scene budget uses it.
pub const GAME_ZNEAR: f32 = 0.1;

/// Far plane of the Aravath third-person camera. The heavy-scene budget uses it.
pub const GAME_ZFAR: f32 = 500.0;

/// Follow camera orbiting a pivot (usually the character center).
pub struct ThirdPersonCamera {
    pub pivot: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Desired eye distance before occlusion.
    pub distance: f32,
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl ThirdPersonCamera {
    pub fn new(pivot: Vec3) -> Self {
        Self {
            pivot,
            yaw: 0.0,
            pitch: -0.35,
            distance: 6.0,
            fovy: 60_f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: GAME_ZNEAR,
            zfar: GAME_ZFAR,
        }
    }

    /// Direction the camera looks (from the eye toward and past the pivot).
    pub fn look_dir(&self) -> Vec3 {
        FreeFly::dir(self.yaw, self.pitch)
    }

    /// Direction from the pivot to the eye.
    pub fn eye_offset_dir(&self) -> Vec3 {
        -self.look_dir()
    }

    pub fn eye_position(&self, distance: f32) -> Vec3 {
        self.pivot + self.eye_offset_dir() * distance
    }

    /// Shorten `distance` when a hit lies along the pivot-to-eye ray.
    ///
    /// `hit_toi` is the physics ray distance. `None` or a hit past the
    /// desired distance leaves the distance unchanged. A nearer hit backs
    /// the eye off the surface by a small margin.
    pub fn distance_after_hit(&self, hit_toi: Option<f32>) -> f32 {
        match hit_toi {
            Some(toi) if toi < self.distance => (toi - 0.25).clamp(0.4, self.distance),
            _ => self.distance,
        }
    }

    pub fn apply_look(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        self.yaw += dx * sensitivity;
        self.pitch = (self.pitch - dy * sensitivity).clamp(-1.2, 1.2);
    }

    fn view_at(&self, eye: Vec3, camera_relative: bool) -> RenderView {
        let projection = Projection::perspective(self.fovy, self.aspect, self.znear, self.zfar);
        let origin = if camera_relative { Vec3::ZERO } else { eye };
        let view = Mat4::look_to_rh(origin, self.look_dir(), Vec3::Y);
        RenderView::new(view, &projection, eye, self.look_dir())
    }

    /// World-relative view. This is what [`CameraProducer::to_render_view`] returns.
    pub fn to_render_view_at(&self, distance: f32) -> RenderView {
        self.view_at(self.eye_position(distance), false)
    }

    /// Camera-relative view: matrices treat the eye as the origin, and
    /// `RenderView::position` stays the world-space eye.
    pub fn to_render_view_camera_relative(&self) -> RenderView {
        self.view_at(self.eye_position(self.distance), true)
    }
}

impl CameraProducer for ThirdPersonCamera {
    fn to_render_view(&self) -> RenderView {
        self.to_render_view_at(self.distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_zero_eye_sits_behind_plus_x() {
        let mut cam = ThirdPersonCamera::new(Vec3::new(10.0, 2.0, 4.0));
        cam.pitch = 0.0;
        cam.distance = 5.0;
        let eye = cam.eye_position(cam.distance);
        assert!((eye.x - 5.0).abs() < 1e-4, "eye.x {}", eye.x);
        assert!((eye.y - 2.0).abs() < 1e-4);
        assert!((eye.z - 4.0).abs() < 1e-4);
        let look = cam.look_dir();
        assert!(look.x > 0.9);
    }

    #[test]
    fn nearer_hit_shortens_distance() {
        let mut cam = ThirdPersonCamera::new(Vec3::ZERO);
        cam.distance = 8.0;
        assert!((cam.distance_after_hit(None) - 8.0).abs() < 1e-6);
        assert!((cam.distance_after_hit(Some(12.0)) - 8.0).abs() < 1e-6);
        let shortened = cam.distance_after_hit(Some(3.0));
        assert!(shortened < 8.0);
        assert!((shortened - 2.75).abs() < 1e-4);
    }

    #[test]
    fn camera_relative_view_has_zero_translation() {
        let cam = ThirdPersonCamera::new(Vec3::new(100.0, 4.0, -20.0));
        let view = cam.to_render_view_camera_relative();
        let translation = view.view.w_axis.truncate();
        assert!(
            translation.length() < 1e-3,
            "camera-relative view translation {translation:?}"
        );
        let eye = cam.eye_position(cam.distance);
        assert!((view.position - eye).length() < 1e-4);
    }
}
