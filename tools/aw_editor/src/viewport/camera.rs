//! Orbit Camera Controller
//!
//! Professional camera controller using spherical coordinates for smooth
//! orbit, pan, and zoom operations. Designed for 3D editing workflows.
//!
//! # Features
//!
//! - **Orbit**: Rotate around focal point (left mouse drag)
//! - **Pan**: Move focal point in screen space (middle mouse drag)
//! - **Zoom**: Change distance from focal point (scroll wheel)
//! - **Frame**: Center camera on selected entity (F key)
//! - **Constraints**: Min/max distance, pitch limits
//!
//! # Example
//!
//! ```
//! use aw_editor_lib::viewport::OrbitCamera;
//! use glam::Vec3;
//!
//! let mut camera = OrbitCamera::default();
//!
//! // Orbit camera
//! camera.orbit(10.0, 5.0);
//!
//! // Zoom in
//! camera.zoom(5.0);
//!
//! // Frame entity
//! camera.frame_entity(Vec3::new(5.0, 0.0, 5.0), 2.0);
//! ```

use astraweave_camera::{CameraProducer, Projection, RenderView};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Professional orbit camera controller
///
/// Uses spherical coordinates (distance, yaw, pitch) around a focal point.
/// Provides smooth, predictable camera controls for 3D editing.
///
/// # Coordinate System
///
/// - **Yaw**: Rotation around Y axis (horizontal), in radians
/// - **Pitch**: Rotation around X axis (vertical), in radians, constrained to [-π/2, π/2]
/// - **Distance**: Radius from focal point, constrained to [min_distance, max_distance]
///
/// # Field of view
///
/// `fovy` stores the **vertical field of view in radians** per
/// `CAMERA_CONVENTIONS.md` §2.1 (canonical convention; matches engine
/// `FreeFly` and the `RenderView` upload contract). The editor's user-
/// facing API ([`set_fov`], [`fov_degrees`]) keeps degrees as the UI
/// boundary unit (humans think in degrees; the FOV slider is degree-
/// scaled) — conversion happens at the boundary methods, not at consumer
/// sites. This boundary discipline is the same pattern Decision 1 of
/// sub-phase C.4.B locked in.
///
/// # Serialization backward-compat
///
/// On deserialization, [`OrbitCamera`] accepts both `fov` (legacy
/// degrees, pre-C.4.B) and `fovy` (canonical radians, post-C.4.B) field
/// names via the [`OrbitCameraSerde`] shadow type and
/// [`From<OrbitCameraSerde>`] implementation. Legacy `fov` values are
/// converted to radians at deserialization. Serialization always emits
/// `fovy`; legacy files are migrated forward on the first save after
/// upgrade.
///
/// # Performance
///
/// Camera updates are O(1) and typically take <0.1ms per frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "OrbitCameraSerde")]
pub struct OrbitCamera {
    /// Focal point (what camera orbits around)
    focal_point: Vec3,

    /// Distance from focal point (meters)
    distance: f32,

    /// Yaw angle (rotation around Y axis, radians)
    yaw: f32,

    /// Pitch angle (rotation around X axis, radians)
    pitch: f32,

    /// Vertical field of view (**radians** per `CAMERA_CONVENTIONS.md` §2.1).
    /// Renamed from pre-C.4.B `fov: f32` (which stored degrees) in
    /// sub-phase C.4.B; the serde shadow type [`OrbitCameraSerde`]
    /// accepts both names on deserialization for backward compatibility
    /// with saved `.editor_preferences.json` files. UI/external API
    /// keeps degrees as the boundary unit ([`set_fov`], [`fov_degrees`]).
    pub(crate) fovy: f32,

    /// Aspect ratio (width / height)
    pub(crate) aspect: f32,

    /// Near clip plane (meters)
    pub(crate) near: f32,

    /// Far clip plane (meters)
    pub(crate) far: f32,

    /// Minimum distance from focal point (meters)
    min_distance: f32,

    /// Maximum distance from focal point (meters)
    max_distance: f32,

    /// Minimum pitch angle (radians, slightly above -π/2 to prevent gimbal lock)
    min_pitch: f32,

    /// Maximum pitch angle (radians, slightly below π/2 to prevent gimbal lock)
    max_pitch: f32,

    /// Target distance for smooth zoom animation
    zoom_target: f32,

    /// Target focal point for smooth camera framing transitions
    focal_point_target: Vec3,

    /// Target pitch for smooth camera framing transitions
    pitch_target: f32,

    /// Target yaw for smooth orbit transitions
    yaw_target: f32,
}

/// Deserialization shadow type for [`OrbitCamera`] handling the C.4.B
/// field rename (`fov: degrees` → `fovy: radians`) with backward
/// compatibility for pre-C.4.B saved files.
///
/// Both `fov` (legacy) and `fovy` (canonical) are accepted via
/// `Option<f32>` fields; [`From<OrbitCameraSerde>`] resolves which value
/// to use:
///
/// 1. If `fovy` is present, use it directly (canonical radians).
/// 2. Else if `fov` is present, treat as legacy degrees and convert via
///    `.to_radians()`.
/// 3. Else fall back to the default (60° converted to radians).
///
/// Serialization is unaffected — `OrbitCamera` derives `Serialize`
/// directly and emits only `fovy`. The shadow type is deserialization-
/// only.
#[derive(Deserialize)]
struct OrbitCameraSerde {
    #[serde(default)]
    focal_point: Vec3,
    #[serde(default = "default_distance")]
    distance: f32,
    #[serde(default = "default_yaw")]
    yaw: f32,
    #[serde(default = "default_pitch")]
    pitch: f32,
    /// Legacy pre-C.4.B field name. If present, interpreted as degrees
    /// and converted to radians.
    #[serde(default)]
    fov: Option<f32>,
    /// Canonical post-C.4.B field name. Stores radians.
    #[serde(default)]
    fovy: Option<f32>,
    #[serde(default = "default_aspect")]
    aspect: f32,
    #[serde(default = "default_near")]
    near: f32,
    #[serde(default = "default_far")]
    far: f32,
    #[serde(default = "default_min_distance")]
    min_distance: f32,
    #[serde(default = "default_max_distance")]
    max_distance: f32,
    #[serde(default = "default_min_pitch")]
    min_pitch: f32,
    #[serde(default = "default_max_pitch")]
    max_pitch: f32,
    #[serde(default = "default_distance")]
    zoom_target: f32,
    #[serde(default)]
    focal_point_target: Vec3,
    #[serde(default = "default_pitch")]
    pitch_target: f32,
    #[serde(default = "default_yaw")]
    yaw_target: f32,
}

// Default helpers for `OrbitCameraSerde` — mirror the values in
// `OrbitCamera::default`. Centralizing them as named functions lets the
// serde derives reference them without repetition.
fn default_distance() -> f32 {
    25.0
}
fn default_yaw() -> f32 {
    std::f32::consts::PI / 4.0
}
fn default_pitch() -> f32 {
    std::f32::consts::PI / 6.0
}
fn default_aspect() -> f32 {
    16.0 / 9.0
}
fn default_near() -> f32 {
    0.5
}
fn default_far() -> f32 {
    5000.0
}
fn default_min_distance() -> f32 {
    0.02
}
fn default_max_distance() -> f32 {
    20000.0
}
fn default_min_pitch() -> f32 {
    -std::f32::consts::PI / 2.0 + 0.01
}
fn default_max_pitch() -> f32 {
    std::f32::consts::PI / 2.0 - 0.01
}

impl From<OrbitCameraSerde> for OrbitCamera {
    fn from(s: OrbitCameraSerde) -> Self {
        // Resolve the FOV: prefer canonical `fovy` (radians); fall back
        // to legacy `fov` (degrees) with conversion; else default 60°.
        let fovy = match (s.fovy, s.fov) {
            (Some(fovy_rad), _) => fovy_rad,
            (None, Some(fov_deg)) => fov_deg.to_radians(),
            (None, None) => 60_f32.to_radians(),
        };
        Self {
            focal_point: s.focal_point,
            distance: s.distance,
            yaw: s.yaw,
            pitch: s.pitch,
            fovy,
            aspect: s.aspect,
            near: s.near,
            far: s.far,
            min_distance: s.min_distance,
            max_distance: s.max_distance,
            min_pitch: s.min_pitch,
            max_pitch: s.max_pitch,
            zoom_target: s.zoom_target,
            focal_point_target: s.focal_point_target,
            pitch_target: s.pitch_target,
            yaw_target: s.yaw_target,
        }
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focal_point: Vec3::ZERO,
            distance: 25.0,                  // Start further back to see more entities
            yaw: std::f32::consts::PI / 4.0, // 45° angle (diagonal view)
            pitch: std::f32::consts::PI / 6.0, // 30° pitch (shallower, see more horizon/sky)
            // 60° vertical FOV in radians (post-C.4.B canonical units per
            // CAMERA_CONVENTIONS.md §2.1; pre-C.4.B was `fov: 60.0` storing
            // degrees with conversion at projection time).
            fovy: 60_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.5,
            far: 5000.0,
            min_distance: 0.02, // Allow camera to get very close (2cm from focal point)
            max_distance: 20000.0, // Allow zooming out to see full terrain from high altitude
            min_pitch: -std::f32::consts::PI / 2.0 + 0.01, // Prevent gimbal lock
            max_pitch: std::f32::consts::PI / 2.0 - 0.01, // Prevent gimbal lock
            zoom_target: 25.0,
            focal_point_target: Vec3::ZERO,
            pitch_target: std::f32::consts::PI / 6.0,
            yaw_target: std::f32::consts::PI / 4.0,
        }
    }
}

impl OrbitCamera {
    /// Create camera with custom parameters
    ///
    /// # Arguments
    ///
    /// * `focal_point` - Initial focal point
    /// * `distance` - Initial distance from focal point
    /// * `yaw` - Initial yaw angle (radians)
    /// * `pitch` - Initial pitch angle (radians)
    pub fn new(focal_point: Vec3, distance: f32, yaw: f32, pitch: f32) -> Self {
        Self {
            focal_point,
            distance,
            yaw,
            pitch,
            zoom_target: distance,
            focal_point_target: focal_point,
            pitch_target: pitch,
            yaw_target: yaw,
            ..Default::default()
        }
    }

    /// Orbit camera (rotate around focal point)
    ///
    /// # Arguments
    ///
    /// * `delta_x` - Horizontal mouse movement (pixels)
    /// * `delta_y` - Vertical mouse movement (pixels)
    ///
    /// # Performance
    ///
    /// O(1), typically <0.01ms
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.005; // Radians per pixel

        // Accumulate into targets; smooth_update() interpolates toward them.
        self.yaw_target -= delta_x * SENSITIVITY;
        self.pitch_target =
            (self.pitch_target - delta_y * SENSITIVITY).clamp(self.min_pitch, self.max_pitch);
    }

    /// Pan camera (move focal point in screen space)
    ///
    /// # Arguments
    ///
    /// * `delta_x` - Horizontal mouse movement (pixels)
    /// * `delta_y` - Vertical mouse movement (pixels)
    ///
    /// Pan speed scales with distance from focal point (further = faster pan).
    ///
    /// # Performance
    ///
    /// O(1), typically <0.05ms (involves cross products)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.005;

        // Calculate right and up vectors in world space
        let forward = self.forward();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        // Pan speed scales with distance (more zoom = slower pan)
        let pan_speed = self.distance * SENSITIVITY;
        // Accumulate into target; smooth_update() interpolates toward it.
        self.focal_point_target -= right * delta_x * pan_speed;
        self.focal_point_target += up * delta_y * pan_speed;
    }

    /// Zoom camera (change distance from focal point)
    ///
    /// # Arguments
    ///
    /// * `delta` - Scroll delta in points (egui raw_scroll_delta.y).
    ///   Standard mice: ~24 pts per notch. Trackpads: smaller, high-frequency.
    ///
    /// Applies zoom directly in log-space for perceptually uniform feel.
    ///
    /// # Performance
    ///
    /// O(1), typically <0.01ms
    pub fn zoom(&mut self, delta: f32) {
        // egui 0.32 raw_scroll_delta is in points (~24 per mouse notch).
        // Normalize so one standard mouse notch ≈ 0.15 in log-space.
        let zoom_amount = delta * (0.15 / 24.0);
        let log_target = self.zoom_target.ln();
        self.zoom_target = (log_target - zoom_amount)
            .exp()
            .clamp(self.min_distance, self.max_distance);
    }

    /// Smoothly animate distance toward zoom target. Call once per frame.
    ///
    /// Returns `true` if zoom animation is still in progress (caller should
    /// request a repaint).
    pub fn smooth_update(&mut self, dt: f32) -> bool {
        let dt_clamped = dt.clamp(0.001, 0.1);
        let mut animating = false;

        // Smooth zoom (distance)
        if (self.distance - self.zoom_target).abs() > 0.0001 {
            // Interpolate in log-space for perceptually smooth zoom at all distances.
            // High decay rate (k=25) for snappy response — settles in ~3 frames at 60fps.
            let log_dist = self.distance.ln();
            let log_target = self.zoom_target.ln();
            let log_diff = log_target - log_dist;

            if log_diff.abs() < 0.0002 {
                self.distance = self.zoom_target;
            } else {
                let factor = 1.0 - (-25.0 * dt_clamped).exp();
                let new_log = log_dist + log_diff * factor;
                self.distance = new_log.exp().clamp(self.min_distance, self.max_distance);
                animating = true;
            }
        }

        // Smooth focal point transition (for frame_terrain / frame_entity)
        let fp_diff = self.focal_point_target - self.focal_point;
        if fp_diff.length_squared() > 0.0001 {
            let factor = 1.0 - (-8.0 * dt_clamped).exp(); // k=8: smoother ~0.3s settle
            self.focal_point += fp_diff * factor;
            animating = true;
        } else if fp_diff.length_squared() > 0.0 {
            self.focal_point = self.focal_point_target;
        }

        // Smooth pitch transition
        let pitch_diff = self.pitch_target - self.pitch;
        if pitch_diff.abs() > 0.0001 {
            let orbit_factor = 1.0 - (-20.0 * dt_clamped).exp(); // k=20: snappy ~50ms settle
            self.pitch += pitch_diff * orbit_factor;
            animating = true;
        } else if pitch_diff.abs() > 0.0 {
            self.pitch = self.pitch_target;
        }

        // Smooth yaw transition
        let yaw_diff = self.yaw_target - self.yaw;
        if yaw_diff.abs() > 0.0001 {
            let orbit_factor = 1.0 - (-20.0 * dt_clamped).exp();
            self.yaw += yaw_diff * orbit_factor;
            animating = true;
        } else if yaw_diff.abs() > 0.0 {
            self.yaw = self.yaw_target;
        }

        animating
    }

    /// Translate camera (FPS-style WASD movement)
    ///
    /// Moves both the camera position and focal point by the given delta.
    pub fn translate(&mut self, delta: Vec3) {
        self.focal_point += delta;
        self.focal_point_target = self.focal_point;
    }

    /// Frame entity (set focal point and distance to nicely view entity)
    ///
    /// # Arguments
    ///
    /// * `entity_pos` - Entity world position
    /// * `entity_radius` - Entity bounding radius (meters)
    ///
    /// Sets focal point to entity center and distance to 2.5× radius for nice framing.
    pub fn frame_entity(&mut self, entity_pos: Vec3, entity_radius: f32) {
        self.focal_point_target = entity_pos;
        self.zoom_target = (entity_radius * 2.5).clamp(self.min_distance, self.max_distance);
    }

    /// Adjust the camera so generated terrain is visible.
    ///
    /// Sets focal_point.y to the average terrain height and pulls the camera
    /// back far enough to see the full height range.
    pub fn frame_terrain(&mut self, min_height: f32, max_height: f32, avg_height: f32) {
        // Animate toward terrain view instead of snapping instantly.
        // The update() tick lerps focal_point/pitch toward these targets.
        self.focal_point_target = Vec3::new(0.0, avg_height, 0.0);
        let height_range = (max_height - min_height).max(10.0);
        let target_dist = (height_range * 1.8).clamp(self.min_distance, self.max_distance);
        self.zoom_target = target_dist;
        // Set pitch target to ~30° above horizontal so the camera ends up well
        // above the terrain looking down.
        self.pitch_target = std::f32::consts::PI / 6.0;
    }

    /// Reset camera to default starting position
    ///
    /// Returns the camera to origin (0,0,0) with default distance and angles.
    /// Useful for recovering from "lost in void" scenarios.
    ///
    /// # Default Values
    ///
    /// - Focal point: (0, 0, 0)
    /// - Distance: 25 meters
    /// - Yaw: 45° (diagonal view)
    /// - Pitch: 30° (looking slightly down)
    pub fn reset_to_origin(&mut self) {
        self.focal_point = Vec3::ZERO;
        self.focal_point_target = Vec3::ZERO;
        self.distance = 25.0;
        self.zoom_target = 25.0;
        self.yaw = std::f32::consts::PI / 4.0; // 45°
        self.yaw_target = self.yaw;
        self.pitch = std::f32::consts::PI / 6.0; // 30°
        self.pitch_target = self.pitch;
    }

    /// Set camera to front view (looking along -Z axis)
    pub fn set_view_front(&mut self) {
        self.yaw_target = 0.0;
        self.pitch_target = 0.0;
    }

    /// Set camera to right view (looking along -X axis)
    pub fn set_view_right(&mut self) {
        self.yaw_target = std::f32::consts::FRAC_PI_2; // 90°
        self.pitch_target = 0.0;
    }

    /// Set camera to top view (looking along -Y axis)
    pub fn set_view_top(&mut self) {
        self.yaw_target = 0.0;
        self.pitch_target = self.max_pitch; // Nearly straight down
    }

    /// Set camera to back view (looking along +Z axis)
    pub fn set_view_back(&mut self) {
        self.yaw_target = std::f32::consts::PI; // 180°
        self.pitch_target = 0.0;
    }

    /// Set camera to perspective view (isometric-like diagonal)
    pub fn set_view_perspective(&mut self) {
        self.yaw_target = std::f32::consts::PI / 4.0; // 45°
        self.pitch_target = std::f32::consts::PI / 6.0; // 30°
    }

    /// Update aspect ratio (call when viewport resizes)
    pub fn set_aspect(&mut self, width: f32, height: f32) {
        if height > 0.0 {
            self.aspect = width / height;
        }
    }

    /// Get camera position in world space
    ///
    /// Calculated from spherical coordinates (distance, yaw, pitch).
    ///
    /// # Performance
    ///
    /// O(1), involves trigonometry (~0.01ms)
    pub fn position(&self) -> Vec3 {
        // Convert spherical to Cartesian coordinates
        let x = self.distance * self.yaw.cos() * self.pitch.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.yaw.sin() * self.pitch.cos();

        self.focal_point + Vec3::new(x, y, z)
    }

    /// Get focal point (what camera orbits around)
    pub fn target(&self) -> Vec3 {
        self.focal_point
    }

    /// Get distance from focal point (meters)
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Get yaw angle (radians)
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Get pitch angle (radians)
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Get focal point (for bookmark save)
    pub fn focal_point(&self) -> Vec3 {
        self.focal_point
    }

    /// Set focal point (for bookmark restore)
    pub fn set_focal_point(&mut self, focal_point: Vec3) {
        self.focal_point = focal_point;
        self.focal_point_target = focal_point;
    }

    /// Set distance (for bookmark restore)
    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.max(self.min_distance);
        self.zoom_target = self.distance;
    }

    /// Set yaw angle (for bookmark restore)
    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
    }

    /// Set pitch angle (for bookmark restore)
    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
    }

    /// Set the vertical field of view, taking **degrees** per the
    /// editor's UI convention.
    ///
    /// Decision 1 of sub-phase C.4.B locked the API boundary as
    /// degrees-taking (humans think in degrees; the FOV slider widget is
    /// degree-scaled). Internally, the value is converted to radians and
    /// stored in [`OrbitCamera::fovy`] per `CAMERA_CONVENTIONS.md` §2.1.
    /// Input is clamped to `[10°, 170°]` to prevent degenerate projections.
    pub fn set_fov(&mut self, degrees: f32) {
        self.fovy = degrees.clamp(10.0, 170.0).to_radians();
    }

    /// Get the vertical field of view in **degrees** (the editor's UI
    /// boundary unit).
    ///
    /// Internally [`OrbitCamera::fovy`] stores radians per
    /// `CAMERA_CONVENTIONS.md` §2.1; this method converts at the read
    /// boundary so the UI slider widget reads degrees directly.
    pub fn fov_degrees(&self) -> f32 {
        self.fovy.to_degrees()
    }

    /// Get the vertical field of view in **radians** (the canonical
    /// internal unit per `CAMERA_CONVENTIONS.md` §2.1).
    ///
    /// Use this when interacting with the canonical pipeline (e.g.,
    /// passing to `Mat4::perspective_rh` or `Projection::perspective`).
    /// For UI surfaces, prefer [`fov_degrees`].
    pub fn fovy(&self) -> f32 {
        self.fovy
    }

    /// Get camera forward vector (normalized)
    pub fn forward(&self) -> Vec3 {
        (self.focal_point - self.position()).normalize()
    }

    /// Get camera right vector (normalized)
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Get camera up vector (normalized)
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    /// Get view matrix (world → camera space)
    ///
    /// Right-handed coordinate system (OpenGL/wgpu convention).
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.focal_point, Vec3::Y)
    }

    /// Get projection matrix (camera → clip space)
    ///
    /// Perspective projection with vertical FOV. `self.fovy` already
    /// stores radians per the post-C.4.B canonical convention; no
    /// conversion needed at the boundary.
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.near, self.far)
    }

    /// Get combined view-projection matrix
    ///
    /// Transforms vertices from world space directly to clip space.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Get camera-relative view matrix (eye at origin).
    ///
    /// Eliminates f32 precision loss at large world coordinates by placing
    /// the camera at the origin. Geometry must be offset by −camera_pos
    /// before being transformed by this matrix.
    pub fn view_matrix_relative(&self) -> Mat4 {
        // Spherical offset from focal point (always small, high precision)
        let x = self.distance * self.yaw.cos() * self.pitch.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.yaw.sin() * self.pitch.cos();
        let eye_offset = Vec3::new(x, y, z);
        // Camera at origin, looking toward −offset direction
        Mat4::look_at_rh(Vec3::ZERO, -eye_offset, Vec3::Y)
    }

    /// Get camera-relative view-projection matrix.
    ///
    /// Use this for rendering to avoid f32 jitter far from the origin.
    /// All world-space positions must be offset by −camera_pos before
    /// being multiplied by this matrix.
    pub fn view_projection_matrix_relative(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix_relative()
    }

    /// Get inverse view-projection matrix
    ///
    /// Transforms from clip space back to world space.
    /// Used for ray casting and unprojection.
    pub fn inverse_view_projection_matrix(&self) -> Mat4 {
        self.view_projection_matrix().inverse()
    }

    /// Create ray from screen position (for picking)
    ///
    /// # Arguments
    ///
    /// * `screen_pos` - Mouse position in viewport (top-left origin)
    /// * `viewport_size` - Viewport dimensions (width, height)
    ///
    /// # Returns
    ///
    /// Ray with origin at near plane and direction toward far plane, both in
    /// world space. Suitable for ray-casting against scene geometry.
    ///
    /// # Coordinate-space discipline
    ///
    /// **Precision-stable inversion**: this function inverts the
    /// camera-relative view-projection ([`Self::view_projection_matrix_relative`])
    /// rather than the absolute one ([`Self::view_projection_matrix`]). The
    /// camera-relative matrix carries translations near zero (the spherical
    /// offset from the focal point), so its inverse is precision-stable
    /// regardless of camera world position. The absolute matrix's translation
    /// column grows with `|position()|`, and its inversion loses precision at
    /// large camera world positions (the C.0 §3.2 audit finding).
    ///
    /// **World-space output**: the unprojected camera-relative points are
    /// translated by `position()` to produce the world-space ray origin.
    /// Ray direction is invariant under translation, so it's computed in
    /// camera-relative space and used as-is. This matches the discipline
    /// of [`Self::unproject_depth_to_world`], which inverts the same
    /// camera-relative VP and adds `position()` to produce a world-space
    /// point. Both picking paths now agree at any camera world position.
    ///
    /// Pre-C.4 (Unified Camera campaign), this function used the absolute
    /// VP and diverged from `unproject_depth_to_world` at large camera
    /// world positions — see `CAMERA_CONVENTIONS.md` §3 migration row
    /// "Editor `OrbitCamera::ray_from_screen` vs `unproject_depth_to_world`
    /// VP mismatch" (closed in C.4).
    pub fn ray_from_screen(&self, screen_pos: egui::Pos2, viewport_size: egui::Vec2) -> Ray {
        // Convert screen pos to NDC [-1, 1]
        let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0; // Flip Y

        // Invert the precision-stable camera-relative VP. wgpu uses [0, 1]
        // depth (near=0, far=1).
        let inv_vp_rel = self.view_projection_matrix_relative().inverse();
        let near_point_rel = inv_vp_rel.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far_point_rel = inv_vp_rel.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        // Translate the origin to world space. Direction is invariant under
        // translation, so we compute it in camera-relative space and use it
        // as-is — no redundant translation.
        let position = self.position();
        Ray {
            origin: near_point_rel + position,
            direction: (far_point_rel - near_point_rel).normalize(),
        }
    }

    pub fn extract_frustum(&self) -> Frustum {
        Frustum::from_view_projection(self.view_projection_matrix())
    }

    /// Unproject a screen pixel + GPU depth value to world space.
    ///
    /// The depth buffer was produced with `view_projection_matrix_relative()`,
    /// so we invert that matrix and then add back `camera_pos` to get the
    /// absolute world coordinate.
    ///
    /// * `px`, `py` — pixel coordinates in viewport (top-left origin)
    /// * `vp_width`, `vp_height` — viewport dimensions in pixels
    /// * `depth` — depth value from the GPU depth buffer, range [0, 1]
    pub fn unproject_depth_to_world(
        &self,
        px: f32,
        py: f32,
        vp_width: f32,
        vp_height: f32,
        depth: f32,
    ) -> Vec3 {
        // Pixel center → NDC
        let ndc_x = ((px + 0.5) / vp_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((py + 0.5) / vp_height) * 2.0;

        // Invert the camera-relative view-projection
        let inv_vp_rel = self.view_projection_matrix_relative().inverse();
        let world_rel = inv_vp_rel.project_point3(Vec3::new(ndc_x, ndc_y, depth));

        // Add camera position to go from camera-relative → absolute world
        world_rel + self.position()
    }

    /// Validate and sanitize camera state after deserialization.
    /// Ensures all fields have reasonable values to prevent broken camera behavior.
    pub fn sanitize(&mut self) {
        let defaults = OrbitCamera::default();

        // Ensure min/max constraints are sane
        if self.min_distance <= 0.0 || self.min_distance.is_nan() {
            self.min_distance = defaults.min_distance;
        }
        if self.max_distance <= self.min_distance || self.max_distance.is_nan() {
            self.max_distance = defaults.max_distance;
        }

        // Clamp distance
        if self.distance.is_nan() || self.distance <= 0.0 {
            self.distance = defaults.distance;
        }
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
        self.zoom_target = self.distance;

        // Validate pitch constraints
        if self.min_pitch.is_nan() || self.max_pitch.is_nan() || self.min_pitch >= self.max_pitch {
            self.min_pitch = defaults.min_pitch;
            self.max_pitch = defaults.max_pitch;
        }

        // Clamp pitch/yaw
        if self.pitch.is_nan() {
            self.pitch = defaults.pitch;
        }
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        self.pitch_target = self.pitch;

        if self.yaw.is_nan() {
            self.yaw = defaults.yaw;
        }
        self.yaw_target = self.yaw;

        // Validate focal point
        if self.focal_point.x.is_nan() || self.focal_point.y.is_nan() || self.focal_point.z.is_nan()
        {
            self.focal_point = Vec3::ZERO;
        }

        // Validate FOV. Clamp to [10°, 170°] expressed in radians per
        // the post-C.4.B canonical units. The degree-based range matches
        // the editor's UI slider boundary (humans think in degrees);
        // internal storage is the radian equivalent.
        let min_fovy = 10_f32.to_radians();
        let max_fovy = 170_f32.to_radians();
        if self.fovy.is_nan() || self.fovy < min_fovy || self.fovy > max_fovy {
            self.fovy = defaults.fovy;
        }

        // Validate aspect
        if self.aspect.is_nan() || self.aspect <= 0.0 {
            self.aspect = defaults.aspect;
        }

        // Validate clip planes
        if self.near.is_nan() || self.near <= 0.0 {
            self.near = defaults.near;
        }
        if self.far.is_nan() || self.far <= self.near {
            self.far = defaults.far;
        }
    }

    /// Camera-relative `RenderView` from OrbitCamera state.
    ///
    /// Builds the view matrix with camera position pre-subtracted (eye at
    /// origin in view-construction space); consuming pipelines transform
    /// world geometry by `-self.position()` before applying this matrix.
    /// Used by the editor's main render path to mitigate large-world
    /// float precision artifacts.
    ///
    /// Per `CAMERA_CONVENTIONS.md` §2.9, this is **not** on the
    /// [`CameraProducer`] trait — it's an opt-in concrete capability that
    /// mirrors [`astraweave_camera::FreeFly::to_render_view_camera_relative`].
    /// Trait callers (cinematics blenders, generic camera managers) get the
    /// world-relative view from [`CameraProducer::to_render_view`]; the
    /// editor's render adapter calls this method directly on the concrete
    /// `OrbitCamera`.
    ///
    /// The returned `RenderView::position` is the original world position
    /// (consumers reconstruct world-space positions via this field for fog
    /// distance, picking, etc.); only the matrices are camera-relative.
    pub fn to_render_view_camera_relative(&self) -> RenderView {
        let projection = Projection::perspective(
            self.fovy,
            self.aspect,
            self.near,
            self.far,
        );
        let view = self.view_matrix_relative();
        let position = self.position();
        let view_dir = (self.focal_point - position).normalize();
        RenderView::new(view, &projection, position, view_dir)
    }
}

impl CameraProducer for OrbitCamera {
    /// World-relative `RenderView` from OrbitCamera state.
    ///
    /// Per `CAMERA_CONVENTIONS.md` §2.9, this is the canonical trait
    /// implementation: world-space view matrix, no camera-relative
    /// translation. For camera-relative rendering (used by the editor's
    /// main render path to mitigate float precision at large camera world
    /// positions), call [`OrbitCamera::to_render_view_camera_relative`]
    /// instead — that's a concrete-type capability, not a trait obligation.
    ///
    /// Post-C.4.B, [`OrbitCamera::fovy`] stores radians directly per
    /// `CAMERA_CONVENTIONS.md` §2.1 (canonical convention); the producer
    /// boundary no longer converts. UI/external API surfaces ([`set_fov`],
    /// [`fov_degrees`]) keep degrees as the user-facing unit.
    ///
    /// The `view_dir` is derived from `focal_point - position()` (the
    /// orbit camera looks from `position()` toward `focal_point`), matching
    /// the semantic of `view_matrix()` which uses `look_at_rh(position(),
    /// focal_point, Vec3::Y)`.
    fn to_render_view(&self) -> RenderView {
        let projection = Projection::perspective(
            self.fovy,
            self.aspect,
            self.near,
            self.far,
        );
        let view = self.view_matrix();
        let position = self.position();
        let view_dir = (self.focal_point - position).normalize();
        RenderView::new(view, &projection, position, view_dir)
    }
}

/// Ray for picking (origin + direction)
///
/// Used for ray-casting to select entities in 3D viewport.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Ray origin (world space)
    pub origin: Vec3,

    /// Ray direction (world space, normalized)
    pub direction: Vec3,
}

impl Ray {
    /// Create new ray
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get point along ray at distance t
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrustumPlane {
    pub normal: Vec3,
    pub distance: f32,
}

impl FrustumPlane {
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [FrustumPlane; 6],
}

impl Frustum {
    pub fn from_view_projection(vp: Mat4) -> Self {
        let m = vp.to_cols_array_2d();
        let planes = [
            Self::extract_plane(m, 0, true),  // Left:   row3 + row0
            Self::extract_plane(m, 0, false), // Right:  row3 - row0
            Self::extract_plane(m, 1, true),  // Bottom: row3 + row1
            Self::extract_plane(m, 1, false), // Top:    row3 - row1
            Self::extract_near_plane(m),      // Near:   row2 (wgpu [0,1] depth)
            Self::extract_plane(m, 2, false), // Far:    row3 - row2
        ];
        Self { planes }
    }

    fn extract_plane(m: [[f32; 4]; 4], row: usize, negative: bool) -> FrustumPlane {
        let sign = if negative { 1.0 } else { -1.0 };
        let a = m[0][3] + sign * m[0][row];
        let b = m[1][3] + sign * m[1][row];
        let c = m[2][3] + sign * m[2][row];
        let d = m[3][3] + sign * m[3][row];
        let len = (a * a + b * b + c * c).sqrt();
        if len > 1e-6 {
            FrustumPlane::new(Vec3::new(a / len, b / len, c / len), d / len)
        } else {
            FrustumPlane::new(Vec3::ZERO, 0.0)
        }
    }

    /// Near plane for wgpu/Vulkan [0,1] depth: z_ndc >= 0 => row2 · P >= 0
    fn extract_near_plane(m: [[f32; 4]; 4]) -> FrustumPlane {
        let a = m[0][2];
        let b = m[1][2];
        let c = m[2][2];
        let d = m[3][2];
        let len = (a * a + b * b + c * c).sqrt();
        if len > 1e-6 {
            FrustumPlane::new(Vec3::new(a / len, b / len, c / len), d / len)
        } else {
            FrustumPlane::new(Vec3::ZERO, 0.0)
        }
    }

    /// Test whether a sphere is inside (or intersecting) the frustum.
    ///
    /// Includes a built-in guard band of 5 world-units on every plane to
    /// prevent objects from popping in/out at frustum edges due to
    /// bounding-sphere approximation errors.
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        const GUARD_BAND: f32 = 5.0;
        for plane in &self.planes {
            if plane.distance_to_point(center) < -(radius + GUARD_BAND) {
                return false;
            }
        }
        true
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        self.contains_sphere(point, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_orbit_camera_default() {
        let camera = OrbitCamera::default();
        assert_eq!(camera.focal_point, Vec3::ZERO);
        assert_eq!(camera.distance, 25.0); // Default is 25.0 for better initial view
                                           // Default pitch is PI/6 (30°) for shallower angle to see more horizon/sky
        assert_relative_eq!(camera.pitch, std::f32::consts::PI / 6.0);
    }

    #[test]
    fn test_orbit_camera_position() {
        let camera = OrbitCamera::default();
        let pos = camera.position();

        // Position should be ~25 units from focal point (default distance)
        let dist = (pos - camera.focal_point).length();
        assert_relative_eq!(dist, 25.0, epsilon = 0.01);
    }

    #[test]
    fn test_orbit_camera_zoom() {
        let mut camera = OrbitCamera::default();
        let initial_dist = camera.distance; // 25.0

        // Zoom in (positive delta = closer, ~24 pts per notch in egui)
        camera.zoom(24.0);
        for _ in 0..60 {
            camera.smooth_update(1.0 / 60.0);
        }
        assert!(
            camera.distance < initial_dist,
            "Zoom in should decrease distance"
        );
        let after_zoom_in = camera.distance;

        // Zoom out
        camera.zoom(-24.0);
        for _ in 0..60 {
            camera.smooth_update(1.0 / 60.0);
        }
        assert!(
            camera.distance > after_zoom_in,
            "Zoom out should increase distance"
        );
    }

    #[test]
    fn test_orbit_camera_zoom_clamp() {
        let mut camera = OrbitCamera::default();

        // Zoom in fully (raw_scroll_delta is ~24 pts per notch in egui 0.32)
        for _ in 0..500 {
            camera.zoom(24.0); // Zoom in
            camera.smooth_update(1.0 / 60.0);
        }
        for _ in 0..120 {
            camera.smooth_update(1.0 / 60.0);
        } // let animation settle
        assert_relative_eq!(camera.distance, camera.min_distance, epsilon = 0.01);

        // Zoom out fully
        for _ in 0..500 {
            camera.zoom(-24.0); // Zoom out
            camera.smooth_update(1.0 / 60.0);
        }
        for _ in 0..120 {
            camera.smooth_update(1.0 / 60.0);
        }
        assert_relative_eq!(camera.distance, camera.max_distance, epsilon = 0.01);
    }

    #[test]
    fn test_frame_entity() {
        let mut camera = OrbitCamera::default();
        let entity_pos = Vec3::new(5.0, 2.0, 5.0);
        let entity_radius = 2.0;

        camera.frame_entity(entity_pos, entity_radius);

        // frame_entity sets targets; run smooth_update to converge
        for _ in 0..300 {
            camera.smooth_update(1.0 / 60.0);
        }

        assert_relative_eq!(camera.focal_point.x, entity_pos.x, epsilon = 0.01);
        assert_relative_eq!(camera.focal_point.y, entity_pos.y, epsilon = 0.01);
        assert_relative_eq!(camera.focal_point.z, entity_pos.z, epsilon = 0.01);
        assert_relative_eq!(camera.distance, 5.0, epsilon = 0.01); // 2.0 * 2.5
    }

    #[test]
    fn test_orbit_pitch_clamp() {
        let mut camera = OrbitCamera::default();

        // Try to orbit beyond max pitch — orbit now sets pitch_target
        camera.orbit(0.0, -10000.0);
        assert_relative_eq!(camera.pitch_target, camera.max_pitch, epsilon = 0.01);

        // Settle the animation
        for _ in 0..60 {
            camera.smooth_update(1.0 / 60.0);
        }
        assert_relative_eq!(camera.pitch, camera.max_pitch, epsilon = 0.01);

        // Try to orbit below min pitch
        camera.orbit(0.0, 10000.0);
        assert_relative_eq!(camera.pitch_target, camera.min_pitch, epsilon = 0.01);

        for _ in 0..60 {
            camera.smooth_update(1.0 / 60.0);
        }
        assert_relative_eq!(camera.pitch, camera.min_pitch, epsilon = 0.01);
    }

    #[test]
    fn test_camera_vectors() {
        let camera = OrbitCamera::default();

        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();

        // Vectors should be normalized
        assert_relative_eq!(forward.length(), 1.0, epsilon = 0.01);
        assert_relative_eq!(right.length(), 1.0, epsilon = 0.01);
        assert_relative_eq!(up.length(), 1.0, epsilon = 0.01);

        // Vectors should be orthogonal
        assert_relative_eq!(forward.dot(right), 0.0, epsilon = 0.01);
        assert_relative_eq!(forward.dot(up), 0.0, epsilon = 0.01);
        assert_relative_eq!(right.dot(up), 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_ray_at() {
        let ray = Ray::new(Vec3::ZERO, Vec3::X);
        assert_eq!(ray.at(0.0), Vec3::ZERO);
        assert_eq!(ray.at(5.0), Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn test_ray_direction_normalized() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0));
        assert_relative_eq!(ray.direction.length(), 1.0, epsilon = 0.01);
    }
}
