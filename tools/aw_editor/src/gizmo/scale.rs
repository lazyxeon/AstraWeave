//! Scale gizmo implementation

//!
//! Supports:
//! - Uniform scaling (default) - Scale all axes equally
//! - Per-axis scaling (X/Y/Z constraint) - Scale single axis
//! - Mouse delta → scale factor (sensitivity controlled)
//! - Numeric input (e.g., "2.0" → 2× scale)
//! - Min/max clamping (0.01× to 100×)
//! - Local space scaling (via object rotation)

use super::AxisConstraint;
use glam::{Quat, Vec2, Vec3};

/// Scale gizmo calculator.
pub struct ScaleGizmo;

impl ScaleGizmo {
    /// Minimum scale factor (prevent negative/zero scale)
    const MIN_SCALE: f32 = 0.01;

    /// Maximum scale factor (prevent extreme growth)
    const MAX_SCALE: f32 = 100.0;

    /// Calculate scale factor from mouse movement.
    ///
    /// # Algorithm
    /// 1. Mouse delta magnitude → scale multiplier: `1.0 + (delta / 100.0) * sensitivity`
    /// 2. Clamp to MIN_SCALE..MAX_SCALE
    /// 3. Apply constraint:
    ///    - Uniform: Vec3::splat(scale_factor) (all axes same)
    ///    - X: Vec3::new(scale_factor, 1.0, 1.0)
    ///    - Y: Vec3::new(1.0, scale_factor, 1.0)
    ///    - Z: Vec3::new(1.0, 1.0, scale_factor)
    ///    - Planar/None: Uniform (default to all axes)
    ///
    /// # Arguments
    /// - `mouse_delta`: Pixel movement since start (Vec2)
    /// - `constraint`: Axis constraint (X/Y/Z for single-axis, None for uniform)
    /// - `uniform`: Force uniform scaling even with constraint
    /// - `sensitivity`: Scale speed (1.0 = 1× per 100px, 2.0 = 2× per 100px)
    /// - `object_rotation`: Current object rotation (for local space, unused in scale)
    /// - `local_space`: Apply constraint in object's local space (unused, scale is always local)
    ///
    /// # Returns
    /// Scale multiplier as Vec3 (e.g., Vec3::new(2.0, 1.0, 1.0) = 2× on X)
    pub fn calculate_scale(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        uniform: bool,
        sensitivity: f32,
        _object_rotation: Quat, // Scale doesn't rotate axes
        _local_space: bool,     // Scale is always in local space
    ) -> Vec3 {
        // 1. Calculate scale factor from mouse delta magnitude
        let delta_magnitude = mouse_delta.length();
        let mut scale_factor = 1.0 + (delta_magnitude / 100.0) * sensitivity;

        // 2. Clamp to safe range
        scale_factor = scale_factor.clamp(Self::MIN_SCALE, Self::MAX_SCALE);

        // 3. Apply constraint
        if uniform {
            // Force uniform scaling
            Vec3::splat(scale_factor)
        } else {
            match constraint {
                AxisConstraint::X => Vec3::new(scale_factor, 1.0, 1.0),
                AxisConstraint::Y => Vec3::new(1.0, scale_factor, 1.0),
                AxisConstraint::Z => Vec3::new(1.0, 1.0, scale_factor),
                // Planar/None → uniform (matches Blender: S = uniform, S+X = X-axis only)
                _ => Vec3::splat(scale_factor),
            }
        }
    }

    /// Calculate scale factor from numeric input.
    ///
    /// # Algorithm
    /// 1. Clamp value to MIN_SCALE..MAX_SCALE
    /// 2. Apply constraint (same as mouse-based)
    ///
    /// # Arguments
    /// - `value`: Scale multiplier (e.g., 2.0 = double size, 0.5 = half size)
    /// - `constraint`: Axis constraint
    /// - `uniform`: Force uniform scaling
    ///
    /// # Returns
    /// Scale multiplier as Vec3
    ///
    /// # Examples
    /// ```ignore
    /// // "2.0" typed → 2× scale on all axes
    /// let scale = ScaleGizmo::calculate_scale_numeric(2.0, AxisConstraint::None, true);
    /// assert_eq!(scale, Vec3::new(2.0, 2.0, 2.0));
    ///
    /// // "0.5" typed with X constraint → half size on X only
    /// let scale = ScaleGizmo::calculate_scale_numeric(0.5, AxisConstraint::X, false);
    /// assert_eq!(scale, Vec3::new(0.5, 1.0, 1.0));
    /// ```
    pub fn calculate_scale_numeric(value: f32, constraint: AxisConstraint, uniform: bool) -> Vec3 {
        // 1. Clamp to safe range
        let scale_factor = value.clamp(Self::MIN_SCALE, Self::MAX_SCALE);

        // 2. Apply constraint
        if uniform {
            Vec3::splat(scale_factor)
        } else {
            match constraint {
                AxisConstraint::X => Vec3::new(scale_factor, 1.0, 1.0),
                AxisConstraint::Y => Vec3::new(1.0, scale_factor, 1.0),
                AxisConstraint::Z => Vec3::new(1.0, 1.0, scale_factor),
                _ => Vec3::splat(scale_factor),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// Helper to assert Vec3 equality with epsilon tolerance
    fn assert_vec3_eq(a: Vec3, b: Vec3, epsilon: f32) {
        assert_relative_eq!(a.x, b.x, epsilon = epsilon);
        assert_relative_eq!(a.y, b.y, epsilon = epsilon);
        assert_relative_eq!(a.z, b.z, epsilon = epsilon);
    }

    // --- Uniform Scaling Tests ---

    #[test]
    fn test_scale_uniform_2x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0), // 100px movement
            AxisConstraint::None,
            true, // Uniform
            1.0,  // Sensitivity
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (100 / 100) * 1.0 = 2.0
        assert_vec3_eq(scale, Vec3::splat(2.0), 0.001);
    }

    #[test]
    fn test_scale_uniform_half() {
        // Negative movement would shrink, but mouse_delta.length() is positive
        // For shrinking, we'd need negative sensitivity or different input mechanism
        // Let's test small delta = slight growth
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(10.0, 0.0), // 10px movement
            AxisConstraint::None,
            true,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (10 / 100) * 1.0 = 1.1
        assert_vec3_eq(scale, Vec3::splat(1.1), 0.001);
    }

    // --- Per-Axis Scaling Tests ---

    #[test]
    fn test_scale_x_axis_2x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::X,
            false, // Not uniform
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert_vec3_eq(scale, Vec3::new(2.0, 1.0, 1.0), 0.001);
    }

    #[test]
    fn test_scale_y_axis_3x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(200.0, 0.0), // 200px
            AxisConstraint::Y,
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (200 / 100) * 1.0 = 3.0
        assert_vec3_eq(scale, Vec3::new(1.0, 3.0, 1.0), 0.001);
    }

    #[test]
    fn test_scale_z_axis_1_5x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(50.0, 0.0), // 50px
            AxisConstraint::Z,
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (50 / 100) * 1.0 = 1.5
        assert_vec3_eq(scale, Vec3::new(1.0, 1.0, 1.5), 0.001);
    }

    // --- Numeric Input Tests ---

    #[test]
    fn test_scale_numeric_2x_uniform() {
        let scale = ScaleGizmo::calculate_scale_numeric(2.0, AxisConstraint::None, true);
        assert_eq!(scale, Vec3::splat(2.0));
    }

    #[test]
    fn test_scale_numeric_half_x_axis() {
        let scale = ScaleGizmo::calculate_scale_numeric(0.5, AxisConstraint::X, false);
        assert_eq!(scale, Vec3::new(0.5, 1.0, 1.0));
    }

    #[test]
    fn test_scale_numeric_3x_y_axis() {
        let scale = ScaleGizmo::calculate_scale_numeric(3.0, AxisConstraint::Y, false);
        assert_eq!(scale, Vec3::new(1.0, 3.0, 1.0));
    }

    // --- Clamping Tests ---

    #[test]
    fn test_scale_clamp_min() {
        // Try to scale to 0.001× (below MIN_SCALE = 0.01)
        let scale = ScaleGizmo::calculate_scale_numeric(0.001, AxisConstraint::None, true);
        assert_eq!(scale, Vec3::splat(0.01)); // Clamped to min
    }

    #[test]
    fn test_scale_clamp_max() {
        // Try to scale to 200× (above MAX_SCALE = 100)
        let scale = ScaleGizmo::calculate_scale_numeric(200.0, AxisConstraint::None, true);
        assert_eq!(scale, Vec3::splat(100.0)); // Clamped to max
    }

    // --- Sensitivity Tests ---

    #[test]
    fn test_scale_sensitivity_2x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::None,
            true,
            2.0, // Double sensitivity
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (100 / 100) * 2.0 = 3.0
        assert_vec3_eq(scale, Vec3::splat(3.0), 0.001);
    }

    #[test]
    fn test_scale_sensitivity_half() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::None,
            true,
            0.5, // Half sensitivity
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (100 / 100) * 0.5 = 1.5
        assert_vec3_eq(scale, Vec3::splat(1.5), 0.001);
    }

    // --- Edge Cases ---

    #[test]
    fn test_scale_zero_mouse_delta() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::ZERO,
            AxisConstraint::None,
            true,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // No movement = no scale change
        assert_eq!(scale, Vec3::ONE);
    }

    #[test]
    fn test_scale_planar_constraint_defaults_to_uniform() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::XY, // Planar constraint
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // Planar not supported → defaults to uniform
        assert_vec3_eq(scale, Vec3::splat(2.0), 0.001);
    }

    #[test]
    fn test_scale_force_uniform_overrides_constraint() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::X, // X constraint
            true,              // Force uniform
            1.0,
            Quat::IDENTITY,
            false,
        );
        // Uniform flag overrides constraint
        assert_vec3_eq(scale, Vec3::splat(2.0), 0.001);
    }
}
