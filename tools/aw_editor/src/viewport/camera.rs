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
/// # Performance
///
/// Camera updates are O(1) and typically take <0.1ms per frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitCamera {
    /// Focal point (what camera orbits around)
    focal_point: Vec3,

    /// Distance from focal point (meters)
    distance: f32,

    /// Yaw angle (rotation around Y axis, radians)
    yaw: f32,

    /// Pitch angle (rotation around X axis, radians)
    pitch: f32,

    /// Field of view (degrees)
    fov: f32,

    /// Aspect ratio (width / height)
    aspect: f32,

    /// Near clip plane (meters)
    near: f32,

    /// Far clip plane (meters)
    far: f32,

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
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focal_point: Vec3::ZERO,
            distance: 25.0,                  // Start further back to see more entities
            yaw: std::f32::consts::PI / 4.0, // 45° angle (diagonal view)
            pitch: std::f32::consts::PI / 6.0, // 30° pitch (shallower, see more horizon/sky)
            fov: 60.0,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 50000.0,
            min_distance: 0.02, // Allow camera to get very close (2cm from focal point)
            max_distance: 20000.0, // Allow zooming out to see full terrain from high altitude
            min_pitch: -std::f32::consts::PI / 2.0 + 0.01, // Prevent gimbal lock
            max_pitch: std::f32::consts::PI / 2.0 - 0.01, // Prevent gimbal lock
            zoom_target: 25.0,
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

        self.yaw -= delta_x * SENSITIVITY;
        self.pitch = (self.pitch - delta_y * SENSITIVITY).clamp(self.min_pitch, self.max_pitch);
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
        self.focal_point -= right * delta_x * pan_speed;
        self.focal_point += up * delta_y * pan_speed;
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
        if (self.distance - self.zoom_target).abs() < 0.0001 {
            self.distance = self.zoom_target;
            return false;
        }

        // Interpolate in log-space for perceptually smooth zoom at all distances.
        // High decay rate (k=25) for snappy response — settles in ~3 frames at 60fps.
        let log_dist = self.distance.ln();
        let log_target = self.zoom_target.ln();
        let log_diff = log_target - log_dist;

        // Snap when close enough to prevent micro-oscillation
        if log_diff.abs() < 0.0002 {
            self.distance = self.zoom_target;
            return false;
        }

        // Frame-rate independent exponential decay.
        // k=25: at 60fps factor≈0.34, at 35fps factor≈0.51 — fast and consistent.
        let dt_clamped = dt.clamp(0.001, 0.1);
        let factor = 1.0 - (-25.0 * dt_clamped).exp();
        let new_log = log_dist + log_diff * factor;
        self.distance = new_log.exp().clamp(self.min_distance, self.max_distance);
        true
    }

    /// Translate camera (FPS-style WASD movement)
    ///
    /// Moves both the camera position and focal point by the given delta.
    pub fn translate(&mut self, delta: Vec3) {
        self.focal_point += delta;
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
        self.focal_point = entity_pos;
        self.distance = (entity_radius * 2.5).clamp(self.min_distance, self.max_distance);
        self.zoom_target = self.distance;
    }

    /// Adjust the camera so generated terrain is visible.
    ///
    /// Sets focal_point.y to the average terrain height and pulls the camera
    /// back far enough to see the full height range.
    pub fn frame_terrain(&mut self, min_height: f32, max_height: f32, avg_height: f32) {
        self.focal_point.y = avg_height;
        let height_range = (max_height - min_height).max(10.0);
        self.distance = (height_range * 1.8).clamp(self.min_distance, self.max_distance);
        self.zoom_target = self.distance;
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
        self.distance = 25.0;
        self.zoom_target = 25.0;
        self.yaw = std::f32::consts::PI / 4.0; // 45°
        self.pitch = std::f32::consts::PI / 6.0; // 30°
    }

    /// Set camera to front view (looking along -Z axis)
    pub fn set_view_front(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
    }

    /// Set camera to right view (looking along -X axis)
    pub fn set_view_right(&mut self) {
        self.yaw = std::f32::consts::FRAC_PI_2; // 90°
        self.pitch = 0.0;
    }

    /// Set camera to top view (looking along -Y axis)
    pub fn set_view_top(&mut self) {
        self.yaw = 0.0;
        self.pitch = self.max_pitch; // Nearly straight down
    }

    /// Set camera to back view (looking along +Z axis)
    pub fn set_view_back(&mut self) {
        self.yaw = std::f32::consts::PI; // 180°
        self.pitch = 0.0;
    }

    /// Set camera to perspective view (isometric-like diagonal)
    pub fn set_view_perspective(&mut self) {
        self.yaw = std::f32::consts::PI / 4.0; // 45°
        self.pitch = std::f32::consts::PI / 6.0; // 30°
    }

    /// Update aspect ratio (call when viewport resizes)
    pub fn set_aspect(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
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
    /// Perspective projection with vertical FOV.
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), self.aspect, self.near, self.far)
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
    /// Ray with origin at near plane and direction towards far plane.
    /// Suitable for ray-casting against scene geometry.
    pub fn ray_from_screen(&self, screen_pos: egui::Pos2, viewport_size: egui::Vec2) -> Ray {
        // Convert screen pos to NDC [-1, 1]
        let ndc_x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0; // Flip Y

        // Unproject to world space
        let inv_vp = self.view_projection_matrix().inverse();
        let near_point = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, -1.0));
        let far_point = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

        Ray {
            origin: near_point,
            direction: (far_point - near_point).normalize(),
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

        if self.yaw.is_nan() {
            self.yaw = defaults.yaw;
        }

        // Validate focal point
        if self.focal_point.x.is_nan() || self.focal_point.y.is_nan() || self.focal_point.z.is_nan()
        {
            self.focal_point = Vec3::ZERO;
        }

        // Validate FOV
        if self.fov.is_nan() || self.fov < 10.0 || self.fov > 170.0 {
            self.fov = defaults.fov;
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

    #[cfg(feature = "astraweave-render")]
    pub fn to_engine_camera(&self) -> astraweave_render::camera::Camera {
        astraweave_render::camera::Camera {
            position: self.position(),
            yaw: self.yaw,
            pitch: self.pitch,
            fovy: self.fov.to_radians(),
            aspect: self.aspect,
            znear: self.near,
            zfar: self.far,
        }
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

        assert_eq!(camera.focal_point, entity_pos);
        assert_eq!(camera.distance, 5.0); // 2.0 * 2.5
    }

    #[test]
    fn test_orbit_pitch_clamp() {
        let mut camera = OrbitCamera::default();

        // Try to orbit beyond max pitch
        camera.orbit(0.0, -10000.0);
        assert_relative_eq!(camera.pitch, camera.max_pitch, epsilon = 0.01);

        // Try to orbit below min pitch
        camera.orbit(0.0, 10000.0);
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
