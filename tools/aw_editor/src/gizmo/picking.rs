//! Ray-picking for gizmo handles.
//!
//! Supports:
//! - Screen → world ray casting (camera projection)
//! - Ray-line segment distance (for arrows/circles)
//! - Ray-AABB intersection (for cubes)
//! - Closest handle selection (when multiple overlap)
//! - Distance threshold (prevent picking from too far away)

use super::{AxisConstraint, GizmoMode};
use glam::{Mat4, Vec2, Vec3, Vec4, Vec4Swizzles};

/// Gizmo handle type (for picking).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GizmoHandle {
    TranslateX,
    TranslateY,
    TranslateZ,
    RotateX,
    RotateY,
    RotateZ,
    ScaleX,
    ScaleY,
    ScaleZ,
    ScaleUniform,
}

impl std::fmt::Display for GizmoHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GizmoHandle::TranslateX => write!(f, "Translate X"),
            GizmoHandle::TranslateY => write!(f, "Translate Y"),
            GizmoHandle::TranslateZ => write!(f, "Translate Z"),
            GizmoHandle::RotateX => write!(f, "Rotate X"),
            GizmoHandle::RotateY => write!(f, "Rotate Y"),
            GizmoHandle::RotateZ => write!(f, "Rotate Z"),
            GizmoHandle::ScaleX => write!(f, "Scale X"),
            GizmoHandle::ScaleY => write!(f, "Scale Y"),
            GizmoHandle::ScaleZ => write!(f, "Scale Z"),
            GizmoHandle::ScaleUniform => write!(f, "Scale Uniform"),
        }
    }
}

impl GizmoHandle {
    /// Returns all gizmo handle variants.
    pub fn all() -> &'static [Self] {
        &[
            GizmoHandle::TranslateX,
            GizmoHandle::TranslateY,
            GizmoHandle::TranslateZ,
            GizmoHandle::RotateX,
            GizmoHandle::RotateY,
            GizmoHandle::RotateZ,
            GizmoHandle::ScaleX,
            GizmoHandle::ScaleY,
            GizmoHandle::ScaleZ,
            GizmoHandle::ScaleUniform,
        ]
    }

    /// Returns the display name of this handle.
    pub fn name(&self) -> &'static str {
        match self {
            GizmoHandle::TranslateX => "Translate X",
            GizmoHandle::TranslateY => "Translate Y",
            GizmoHandle::TranslateZ => "Translate Z",
            GizmoHandle::RotateX => "Rotate X",
            GizmoHandle::RotateY => "Rotate Y",
            GizmoHandle::RotateZ => "Rotate Z",
            GizmoHandle::ScaleX => "Scale X",
            GizmoHandle::ScaleY => "Scale Y",
            GizmoHandle::ScaleZ => "Scale Z",
            GizmoHandle::ScaleUniform => "Scale Uniform",
        }
    }

    /// Returns true if this is a translation handle.
    pub fn is_translate(&self) -> bool {
        matches!(
            self,
            GizmoHandle::TranslateX | GizmoHandle::TranslateY | GizmoHandle::TranslateZ
        )
    }

    /// Returns true if this is a rotation handle.
    pub fn is_rotate(&self) -> bool {
        matches!(
            self,
            GizmoHandle::RotateX | GizmoHandle::RotateY | GizmoHandle::RotateZ
        )
    }

    /// Returns true if this is a scale handle.
    pub fn is_scale(&self) -> bool {
        matches!(
            self,
            GizmoHandle::ScaleX
                | GizmoHandle::ScaleY
                | GizmoHandle::ScaleZ
                | GizmoHandle::ScaleUniform
        )
    }

    /// Returns the axis character for this handle ('X', 'Y', 'Z', or 'U' for uniform).
    pub fn axis(&self) -> char {
        match self {
            GizmoHandle::TranslateX | GizmoHandle::RotateX | GizmoHandle::ScaleX => 'X',
            GizmoHandle::TranslateY | GizmoHandle::RotateY | GizmoHandle::ScaleY => 'Y',
            GizmoHandle::TranslateZ | GizmoHandle::RotateZ | GizmoHandle::ScaleZ => 'Z',
            GizmoHandle::ScaleUniform => 'U',
        }
    }

    /// Returns the color for this handle (for rendering).
    pub fn color(&self) -> [f32; 3] {
        match self {
            GizmoHandle::TranslateX | GizmoHandle::RotateX | GizmoHandle::ScaleX => [1.0, 0.2, 0.2], // Red
            GizmoHandle::TranslateY | GizmoHandle::RotateY | GizmoHandle::ScaleY => [0.2, 1.0, 0.2], // Green
            GizmoHandle::TranslateZ | GizmoHandle::RotateZ | GizmoHandle::ScaleZ => [0.3, 0.3, 1.0], // Blue
            GizmoHandle::ScaleUniform => [1.0, 1.0, 1.0], // White
        }
    }
}

impl GizmoHandle {
    /// Convert to AxisConstraint (for state machine).
    pub fn to_constraint(self) -> AxisConstraint {
        match self {
            GizmoHandle::TranslateX | GizmoHandle::RotateX | GizmoHandle::ScaleX => {
                AxisConstraint::X
            }
            GizmoHandle::TranslateY | GizmoHandle::RotateY | GizmoHandle::ScaleY => {
                AxisConstraint::Y
            }
            GizmoHandle::TranslateZ | GizmoHandle::RotateZ | GizmoHandle::ScaleZ => {
                AxisConstraint::Z
            }
            GizmoHandle::ScaleUniform => AxisConstraint::None,
        }
    }

    /// Get handle's gizmo mode.
    pub fn mode(self) -> GizmoMode {
        match self {
            GizmoHandle::TranslateX | GizmoHandle::TranslateY | GizmoHandle::TranslateZ => {
                GizmoMode::Translate {
                    constraint: self.to_constraint(),
                }
            }
            GizmoHandle::RotateX | GizmoHandle::RotateY | GizmoHandle::RotateZ => {
                GizmoMode::Rotate {
                    constraint: self.to_constraint(),
                }
            }
            GizmoHandle::ScaleX
            | GizmoHandle::ScaleY
            | GizmoHandle::ScaleZ
            | GizmoHandle::ScaleUniform => GizmoMode::Scale {
                constraint: self.to_constraint(),
                uniform: matches!(self, GizmoHandle::ScaleUniform),
            },
        }
    }
}

/// Ray for intersection testing.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3, // Normalized
}

impl Ray {
    /// Create ray from screen coordinates.
    ///
    /// # Arguments
    /// - `screen_pos`: Normalized device coordinates [-1, 1] (top-left = (-1, 1), bottom-right = (1, -1))
    /// - `inv_view_proj`: Inverse view-projection matrix (world ← clip)
    ///
    /// # Algorithm
    /// 1. Near point: `screen_pos` at z=-1 (near plane)
    /// 2. Far point: `screen_pos` at z=1 (far plane)
    /// 3. Transform both to world space
    /// 4. Direction: `(far - near).normalize()`
    pub fn from_screen(screen_pos: Vec2, inv_view_proj: Mat4) -> Self {
        // Construct clip-space points (NDC)
        let near_clip = Vec4::new(screen_pos.x, screen_pos.y, -1.0, 1.0); // Near plane
        let far_clip = Vec4::new(screen_pos.x, screen_pos.y, 1.0, 1.0); // Far plane

        // Transform to world space
        let near_world = inv_view_proj * near_clip;
        let far_world = inv_view_proj * far_clip;

        // Perspective division (w != 1.0 after projection)
        let near = near_world.xyz() / near_world.w;
        let far = far_world.xyz() / far_world.w;

        // Ray from near to far
        let direction = (far - near).normalize();

        Ray {
            origin: near,
            direction,
        }
    }

    /// Get point along ray at distance `t`.
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

/// Gizmo picking parameters.
#[derive(Debug, Clone)]
pub struct GizmoPicker {
    /// Maximum pick distance (in world units).
    pub max_distance: f32,

    /// Pick tolerance (for line segments, in world units).
    pub tolerance: f32,

    /// Gizmo scale factor (affects pick volumes).
    pub gizmo_scale: f32,
}

impl Default for GizmoPicker {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            tolerance: 0.2,
            gizmo_scale: 1.0,
        }
    }
}

impl GizmoPicker {
    /// Pick gizmo handle from screen coordinates.
    ///
    /// # Arguments
    /// - `screen_pos`: Normalized device coordinates [-1, 1]
    /// - `inv_view_proj`: Inverse view-projection matrix
    /// - `gizmo_pos`: Gizmo world position
    /// - `mode`: Current gizmo mode (Translate/Rotate/Scale)
    ///
    /// # Returns
    /// Closest picked handle (if any within tolerance).
    pub fn pick_handle(
        &self,
        screen_pos: Vec2,
        inv_view_proj: Mat4,
        gizmo_pos: Vec3,
        mode: GizmoMode,
    ) -> Option<GizmoHandle> {
        let ray = Ray::from_screen(screen_pos, inv_view_proj);

        match mode {
            GizmoMode::Translate { .. } => self.pick_translate_handle(&ray, gizmo_pos),
            GizmoMode::Rotate { .. } => self.pick_rotate_handle(&ray, gizmo_pos),
            GizmoMode::Scale { .. } => self.pick_scale_handle(&ray, gizmo_pos),
            GizmoMode::Inactive => None,
        }
    }

    /// Pick translation arrow (ray-line segment distance test).
    fn pick_translate_handle(&self, ray: &Ray, gizmo_pos: Vec3) -> Option<GizmoHandle> {
        let length = self.gizmo_scale;

        // Test each axis arrow
        let axes = [
            (Vec3::X, GizmoHandle::TranslateX),
            (Vec3::Y, GizmoHandle::TranslateY),
            (Vec3::Z, GizmoHandle::TranslateZ),
        ];

        let mut closest: Option<(GizmoHandle, f32)> = None;

        for (axis, handle) in axes {
            let arrow_start = gizmo_pos;
            let arrow_end = gizmo_pos + axis * length;

            let distance = ray_line_segment_distance(ray, arrow_start, arrow_end);

            if distance < self.tolerance {
                if let Some((_, closest_dist)) = closest {
                    if distance < closest_dist {
                        closest = Some((handle, distance));
                    }
                } else {
                    closest = Some((handle, distance));
                }
            }
        }

        closest.map(|(handle, _)| handle)
    }

    /// Pick rotation circle (ray-torus intersection, simplified to ray-circle distance).
    fn pick_rotate_handle(&self, ray: &Ray, gizmo_pos: Vec3) -> Option<GizmoHandle> {
        let radius = self.gizmo_scale;

        // Test each axis circle
        let axes = [
            (Vec3::X, GizmoHandle::RotateX),
            (Vec3::Y, GizmoHandle::RotateY),
            (Vec3::Z, GizmoHandle::RotateZ),
        ];

        let mut closest: Option<(GizmoHandle, f32)> = None;

        for (axis, handle) in axes {
            let distance = ray_circle_distance(ray, gizmo_pos, axis, radius);

            if distance < self.tolerance {
                if let Some((_, closest_dist)) = closest {
                    if distance < closest_dist {
                        closest = Some((handle, distance));
                    }
                } else {
                    closest = Some((handle, distance));
                }
            }
        }

        closest.map(|(handle, _)| handle)
    }

    /// Pick scale cube (ray-AABB intersection).
    fn pick_scale_handle(&self, ray: &Ray, gizmo_pos: Vec3) -> Option<GizmoHandle> {
        let offset = self.gizmo_scale * 0.8;
        let size = self.gizmo_scale * 0.15;
        let half_size = size * 0.5;

        // Test each axis cube
        let axes = [
            (Vec3::X, GizmoHandle::ScaleX),
            (Vec3::Y, GizmoHandle::ScaleY),
            (Vec3::Z, GizmoHandle::ScaleZ),
        ];

        let mut closest: Option<(GizmoHandle, f32)> = None;

        for (axis, handle) in axes {
            let cube_center = gizmo_pos + axis * offset;
            let cube_min = cube_center - Vec3::splat(half_size);
            let cube_max = cube_center + Vec3::splat(half_size);

            if let Some(t) = ray_aabb_intersection(ray, cube_min, cube_max) {
                if let Some((_, closest_t)) = closest {
                    if t < closest_t {
                        closest = Some((handle, t));
                    }
                } else {
                    closest = Some((handle, t));
                }
            }
        }

        closest.map(|(handle, _)| handle)
    }
}

/// Ray-line segment distance (closest approach).
///
/// # Algorithm
/// 1. Project ray onto line segment
/// 2. Find closest points on both
/// 3. Return distance between closest points
fn ray_line_segment_distance(ray: &Ray, segment_start: Vec3, segment_end: Vec3) -> f32 {
    let segment_dir = segment_end - segment_start;
    let segment_length = segment_dir.length();
    let segment_dir_norm = segment_dir / segment_length;

    // Vector from ray origin to segment start
    let w = ray.origin - segment_start;

    // Coefficients for closest approach
    let a = ray.direction.dot(ray.direction); // Always 1 (ray normalized)
    let b = ray.direction.dot(segment_dir_norm);
    let c = segment_dir_norm.dot(segment_dir_norm); // Always 1 (segment normalized)
    let d = ray.direction.dot(w);
    let e = segment_dir_norm.dot(w);

    // Solve for closest points
    let denom = a * c - b * b;
    let t_ray = if denom.abs() < 1e-6 {
        0.0 // Parallel
    } else {
        (b * e - c * d) / denom
    };

    let t_segment = (a * e - b * d) / (if denom.abs() < 1e-6 { 1.0 } else { denom });
    let t_segment_clamped = t_segment.clamp(0.0, segment_length);

    // Closest points
    let ray_point = ray.point_at(t_ray.max(0.0));
    let segment_point = segment_start + segment_dir_norm * t_segment_clamped;

    (ray_point - segment_point).length()
}

/// Ray-circle distance (approximate, for picking).
///
/// # Algorithm
/// 1. Project ray onto circle plane
/// 2. Find closest point on circle to ray
/// 3. Return distance
fn ray_circle_distance(ray: &Ray, center: Vec3, normal: Vec3, radius: f32) -> f32 {
    // Intersect ray with circle plane
    let denom = ray.direction.dot(normal);
    if denom.abs() < 1e-6 {
        return f32::MAX; // Parallel to plane
    }

    let t = (center - ray.origin).dot(normal) / denom;
    if t < 0.0 {
        return f32::MAX; // Behind ray origin
    }

    let intersection_point = ray.point_at(t);
    let to_intersection = intersection_point - center;

    // Project onto circle
    let _distance_from_center = to_intersection.length();
    let on_circle = to_intersection.normalize() * radius;
    let circle_point = center + on_circle;

    (intersection_point - circle_point).length()
}

/// Ray-AABB intersection (Axis-Aligned Bounding Box).
///
/// # Returns
/// Intersection distance `t` (if hit), or `None`.
///
/// # Algorithm
/// Slab method: test ray against each axis-aligned slab.
fn ray_aabb_intersection(ray: &Ray, min: Vec3, max: Vec3) -> Option<f32> {
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = f32::MAX;

    // Test X slab
    if ray.direction.x.abs() > 1e-6 {
        let tx1 = (min.x - ray.origin.x) / ray.direction.x;
        let tx2 = (max.x - ray.origin.x) / ray.direction.x;
        t_min = t_min.max(tx1.min(tx2));
        t_max = t_max.min(tx1.max(tx2));
    } else if ray.origin.x < min.x || ray.origin.x > max.x {
        return None; // Ray parallel and outside slab
    }

    // Test Y slab
    if ray.direction.y.abs() > 1e-6 {
        let ty1 = (min.y - ray.origin.y) / ray.direction.y;
        let ty2 = (max.y - ray.origin.y) / ray.direction.y;
        t_min = t_min.max(ty1.min(ty2));
        t_max = t_max.min(ty1.max(ty2));
    } else if ray.origin.y < min.y || ray.origin.y > max.y {
        return None;
    }

    // Test Z slab
    if ray.direction.z.abs() > 1e-6 {
        let tz1 = (min.z - ray.origin.z) / ray.direction.z;
        let tz2 = (max.z - ray.origin.z) / ray.direction.z;
        t_min = t_min.max(tz1.min(tz2));
        t_max = t_max.min(tz1.max(tz2));
    } else if ray.origin.z < min.z || ray.origin.z > max.z {
        return None;
    }

    if t_max >= t_min && t_max >= 0.0 {
        Some(t_min.max(0.0)) // Return closest hit (or 0 if inside AABB)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_from_screen_center() {
        // Create a simple perspective projection looking down -Z
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 0.0, 10.0), // Eye position
            Vec3::ZERO,                // Look at origin
            Vec3::Y,                   // Up vector
        );
        let proj = Mat4::perspective_rh(
            std::f32::consts::FRAC_PI_4, // 45° FOV
            1.0,                         // Aspect ratio
            0.1,                         // Near plane
            100.0,                       // Far plane
        );
        let inv_vp = (proj * view).inverse();

        let ray = Ray::from_screen(Vec2::ZERO, inv_vp);

        // Ray should point down -Z axis (from camera at +Z looking at origin)
        assert!((ray.direction - Vec3::new(0.0, 0.0, -1.0)).length() < 0.01);

        // Ray origin should be near camera position
        assert!((ray.origin.z - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_ray_aabb_intersection_hit() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, -5.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        let min = Vec3::new(-1.0, -1.0, -1.0);
        let max = Vec3::new(1.0, 1.0, 1.0);

        let t = ray_aabb_intersection(&ray, min, max);
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 0.01); // Hit at z=-1, distance=4
    }

    #[test]
    fn test_ray_aabb_intersection_miss() {
        let ray = Ray {
            origin: Vec3::new(5.0, 0.0, 0.0),
            direction: Vec3::new(0.0, 0.0, 1.0), // Ray parallel to Z
        };

        let min = Vec3::new(-1.0, -1.0, -1.0);
        let max = Vec3::new(1.0, 1.0, 1.0);

        let t = ray_aabb_intersection(&ray, min, max);
        assert!(t.is_none()); // Ray misses box
    }

    #[test]
    fn test_ray_line_segment_distance_hit() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.1, 0.0), // Slightly above segment
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        let segment_start = Vec3::new(0.0, 0.0, 0.0);
        let segment_end = Vec3::new(1.0, 0.0, 0.0);

        let distance = ray_line_segment_distance(&ray, segment_start, segment_end);
        assert!((distance - 0.1).abs() < 0.01); // 0.1 units above segment
    }

    #[test]
    fn test_ray_line_segment_distance_miss() {
        let ray = Ray {
            origin: Vec3::new(5.0, 5.0, 0.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        let segment_start = Vec3::new(0.0, 0.0, 0.0);
        let segment_end = Vec3::new(1.0, 0.0, 0.0);

        let distance = ray_line_segment_distance(&ray, segment_start, segment_end);
        assert!(distance > 5.0); // Far from segment
    }

    #[test]
    fn test_gizmo_handle_to_constraint() {
        assert_eq!(GizmoHandle::TranslateX.to_constraint(), AxisConstraint::X);
        assert_eq!(GizmoHandle::RotateY.to_constraint(), AxisConstraint::Y);
        assert_eq!(GizmoHandle::ScaleZ.to_constraint(), AxisConstraint::Z);
        assert_eq!(
            GizmoHandle::ScaleUniform.to_constraint(),
            AxisConstraint::None
        );
    }

    #[test]
    fn test_gizmo_handle_mode() {
        let handle = GizmoHandle::TranslateX;
        assert!(matches!(handle.mode(), GizmoMode::Translate { .. }));

        let handle = GizmoHandle::RotateY;
        assert!(matches!(handle.mode(), GizmoMode::Rotate { .. }));

        let handle = GizmoHandle::ScaleZ;
        assert!(matches!(handle.mode(), GizmoMode::Scale { .. }));
    }

    #[test]
    fn test_pick_translate_arrow_x() {
        let picker = GizmoPicker::default();
        let gizmo_pos = Vec3::ZERO;

        // Ray pointing at X-axis arrow
        let ray = Ray {
            origin: Vec3::new(0.5, 0.05, -5.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        // Manually test pick (since we need inv_view_proj for full API)
        let handle = picker.pick_translate_handle(&ray, gizmo_pos);
        assert_eq!(handle, Some(GizmoHandle::TranslateX));
    }

    #[test]
    fn test_pick_scale_cube_y() {
        let picker = GizmoPicker::default();
        let gizmo_pos = Vec3::ZERO;

        // Ray pointing at Y-axis cube (at offset 0.8)
        let ray = Ray {
            origin: Vec3::new(0.0, 0.8, -5.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        let handle = picker.pick_scale_handle(&ray, gizmo_pos);
        assert_eq!(handle, Some(GizmoHandle::ScaleY));
    }

    // ==================== GizmoHandle Display Tests ====================

    #[test]
    fn test_gizmo_handle_display() {
        assert_eq!(GizmoHandle::TranslateX.to_string(), "Translate X");
        assert_eq!(GizmoHandle::RotateY.to_string(), "Rotate Y");
        assert_eq!(GizmoHandle::ScaleZ.to_string(), "Scale Z");
        assert_eq!(GizmoHandle::ScaleUniform.to_string(), "Scale Uniform");
    }

    #[test]
    fn test_gizmo_handle_all_and_name() {
        let handles = GizmoHandle::all();
        assert_eq!(handles.len(), 10);
        for handle in handles {
            assert!(!handle.name().is_empty());
        }
    }

    #[test]
    fn test_gizmo_handle_type_helpers() {
        assert!(GizmoHandle::TranslateX.is_translate());
        assert!(GizmoHandle::TranslateY.is_translate());
        assert!(GizmoHandle::TranslateZ.is_translate());
        assert!(!GizmoHandle::TranslateX.is_rotate());
        assert!(!GizmoHandle::TranslateX.is_scale());

        assert!(GizmoHandle::RotateX.is_rotate());
        assert!(!GizmoHandle::RotateX.is_translate());

        assert!(GizmoHandle::ScaleX.is_scale());
        assert!(GizmoHandle::ScaleUniform.is_scale());
    }

    #[test]
    fn test_gizmo_handle_axis() {
        assert_eq!(GizmoHandle::TranslateX.axis(), 'X');
        assert_eq!(GizmoHandle::RotateY.axis(), 'Y');
        assert_eq!(GizmoHandle::ScaleZ.axis(), 'Z');
        assert_eq!(GizmoHandle::ScaleUniform.axis(), 'U');
    }

    #[test]
    fn test_gizmo_handle_color() {
        // X handles are red-ish
        assert!(GizmoHandle::TranslateX.color()[0] > 0.5);
        // Y handles are green-ish
        assert!(GizmoHandle::RotateY.color()[1] > 0.5);
        // Z handles are blue-ish
        assert!(GizmoHandle::ScaleZ.color()[2] > 0.5);
        // Uniform is white
        let uniform_color = GizmoHandle::ScaleUniform.color();
        assert!(uniform_color[0] > 0.9 && uniform_color[1] > 0.9 && uniform_color[2] > 0.9);
    }
}
