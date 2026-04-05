//! Rotation gizmo implementation

use super::{AxisConstraint, SnappingConfig};
use glam::{Quat, Vec2, Vec3};

/// Rotation gizmo calculator.
pub struct RotateGizmo;

impl RotateGizmo {
    /// Snap rotation using snapping configuration.
    pub fn snap_rotation(rotation: Quat, snapping: &SnappingConfig) -> Quat {
        snapping.snap_rotation(rotation)
    }

    /// Calculate rotation from mouse movement.
    ///
    /// # Arguments
    /// * `mouse_delta` - Screen-space mouse movement (pixels)
    /// * `constraint` - Axis constraint (X/Y/Z for single-axis rotation)
    /// * `sensitivity` - Rotation sensitivity (radians per 100 pixels)
    /// * `snap_enabled` - If true, snap to 15° increments
    /// * `object_rotation` - Object's current rotation (for local space)
    /// * `local_space` - If true, use object-local coordinates
    ///
    /// # Returns
    /// Quaternion representing the rotation delta
    pub fn calculate_rotation(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        sensitivity: f32,
        snap_enabled: bool,
        object_rotation: Quat,
        local_space: bool,
    ) -> Quat {
        // Map mouse delta to rotation based on axis
        // Use both horizontal and vertical components to support drag in any direction
        // This allows intuitive rotation regardless of drag direction
        let rotation_delta = match constraint {
            AxisConstraint::X | AxisConstraint::Y | AxisConstraint::Z => {
                // Use the component with larger magnitude (primary drag direction)
                if mouse_delta.x.abs() > mouse_delta.y.abs() {
                    mouse_delta.x
                } else {
                    mouse_delta.y
                }
            }
            _ => 0.0, // Will return IDENTITY below
        };

        // Calculate rotation angle from mouse delta
        // Sensitivity is in radians per 100 pixels
        // rotation_delta / 100.0 gives normalized distance, multiplied by sensitivity gives radians
        let mut angle = rotation_delta * (sensitivity / 100.0);

        // Apply snapping if enabled (15° increments = π/12 radians)
        if snap_enabled {
            let snap_increment = std::f32::consts::PI / 12.0; // 15°
            angle = (angle / snap_increment).round() * snap_increment;
        }

        // Determine rotation axis based on constraint
        let axis = match constraint {
            AxisConstraint::X => Vec3::X,
            AxisConstraint::Y => Vec3::Y,
            AxisConstraint::Z => Vec3::Z,
            AxisConstraint::None => return Quat::IDENTITY, // None constraint not supported for rotation
            _ => return Quat::IDENTITY, // Planar constraints not supported for rotation
        };

        // Create rotation quaternion

        if local_space {
            // Rotate axis to local space
            let local_axis = object_rotation * axis;
            Quat::from_axis_angle(local_axis, angle)
        } else {
            // Direct world-space rotation
            Quat::from_axis_angle(axis, angle)
        }
    }

    /// Calculate rotation from numeric input value (in degrees).
    ///
    /// # Arguments
    /// * `degrees` - Rotation angle in degrees (e.g., "90" → 90°)
    /// * `constraint` - Axis constraint (X/Y/Z for single-axis rotation)
    /// * `object_rotation` - Object's rotation (for local space)
    /// * `local_space` - If true, use object-local coordinates
    ///
    /// # Returns
    /// Quaternion representing the exact rotation
    pub fn calculate_rotation_numeric(
        degrees: f32,
        constraint: AxisConstraint,
        object_rotation: Quat,
        local_space: bool,
    ) -> Quat {
        // Convert degrees to radians
        let angle = degrees.to_radians();

        // Determine rotation axis
        let axis = match constraint {
            AxisConstraint::X => Vec3::X,
            AxisConstraint::Y => Vec3::Y,
            AxisConstraint::Z => Vec3::Z,
            _ => return Quat::IDENTITY, // Planar constraints not supported
        };

        // Create rotation quaternion
        if local_space {
            let local_axis = object_rotation * axis;
            Quat::from_axis_angle(local_axis, angle)
        } else {
            Quat::from_axis_angle(axis, angle)
        }
    }

    /// Get rotation angle in degrees from quaternion (for display).
    ///
    /// # Arguments
    /// * `rotation` - Rotation quaternion
    /// * `axis` - Axis to measure rotation around
    ///
    /// # Returns
    /// Angle in degrees (preserves sign)
    pub fn get_rotation_angle(rotation: Quat, axis: Vec3) -> f32 {
        let (axis_result, angle) = rotation.to_axis_angle();

        // Check if rotation axis matches (within tolerance)
        let dot = axis_result.dot(axis);
        if dot > 0.99 {
            // Same direction
            angle.to_degrees()
        } else if dot < -0.99 {
            // Opposite direction (negative rotation)
            -angle.to_degrees()
        } else {
            // Different axis - no rotation around this axis
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rotation_x_axis_90_degrees() {
        // Rotate 90° around X axis
        // Mouse delta magnitude: 100px * (π/2) / sensitivity
        let sensitivity = std::f32::consts::PI / 2.0; // 90° per 100px
        let mouse_delta = Vec2::new(100.0, 0.0); // 100px horizontal

        let rotation = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::X,
            sensitivity,
            false, // no snap
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::X);
        assert_relative_eq!(angle, 90.0, epsilon = 0.1);
    }

    #[test]
    fn test_rotation_y_axis_45_degrees() {
        let sensitivity = std::f32::consts::PI / 4.0; // 45° per 100px
        let mouse_delta = Vec2::new(0.0, 100.0); // 100px vertical

        let rotation = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::Y,
            sensitivity,
            false,
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::Y);
        assert_relative_eq!(angle.abs(), 45.0, epsilon = 0.1);
    }

    #[test]
    fn test_rotation_z_axis_180_degrees() {
        let sensitivity = std::f32::consts::PI; // 180° per 100px
        let mouse_delta = Vec2::new(100.0, 0.0);

        let rotation = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::Z,
            sensitivity,
            false,
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::Z);
        assert_relative_eq!(angle.abs(), 180.0, epsilon = 0.1);
    }

    #[test]
    fn test_rotation_snap_15_degrees() {
        // Test 15° snapping
        let sensitivity = 1.0; // 1 radian per 100px ≈ 57° per 100px
        let mouse_delta = Vec2::new(30.0, 0.0); // Small movement

        let rotation = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::X,
            sensitivity,
            true, // snap enabled
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::X);

        // Should snap to nearest 15° increment
        let remainder = angle % 15.0;
        assert!(
            remainder.abs() < 0.1 || (15.0 - remainder.abs()) < 0.1,
            "Angle {} should snap to 15° increments",
            angle
        );
    }

    #[test]
    fn test_rotation_snap_90_degrees() {
        // Test snapping to 90° (6 × 15°)
        let sensitivity = std::f32::consts::PI / 2.0; // 90° per 100px
        let mouse_delta = Vec2::new(100.0, 0.0);

        let rotation = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::Y,
            sensitivity,
            true, // snap enabled
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::Y);
        assert_relative_eq!(angle, 90.0, epsilon = 0.1);
    }

    #[test]
    fn test_rotation_numeric_90_degrees() {
        // Numeric input: "90" → 90° rotation
        let rotation =
            RotateGizmo::calculate_rotation_numeric(90.0, AxisConstraint::X, Quat::IDENTITY, false);

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::X);
        assert_relative_eq!(angle, 90.0, epsilon = 0.001);
    }

    #[test]
    fn test_rotation_numeric_negative_45_degrees() {
        let rotation = RotateGizmo::calculate_rotation_numeric(
            -45.0,
            AxisConstraint::Y,
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::Y);
        assert_relative_eq!(angle, -45.0, epsilon = 0.001);
    }

    #[test]
    fn test_rotation_numeric_180_degrees() {
        let rotation = RotateGizmo::calculate_rotation_numeric(
            180.0,
            AxisConstraint::Z,
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::Z);
        assert_relative_eq!(angle.abs(), 180.0, epsilon = 0.001);
    }

    #[test]
    fn test_rotation_planar_constraint_returns_identity() {
        // Planar constraints (XY, XZ, YZ) not supported for rotation
        let rotation = RotateGizmo::calculate_rotation(
            Vec2::new(100.0, 0.0),
            AxisConstraint::XY, // Planar
            1.0,
            false,
            Quat::IDENTITY,
            false,
        );

        assert_eq!(rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_rotation_none_constraint_returns_identity() {
        // None constraint not supported for rotation
        let rotation = RotateGizmo::calculate_rotation(
            Vec2::new(100.0, 0.0),
            AxisConstraint::None,
            1.0,
            false,
            Quat::IDENTITY,
            false,
        );

        assert_eq!(rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_rotation_zero_mouse_delta() {
        // No mouse movement = no rotation
        let rotation = RotateGizmo::calculate_rotation(
            Vec2::ZERO,
            AxisConstraint::X,
            1.0,
            false,
            Quat::IDENTITY,
            false,
        );

        let angle = RotateGizmo::get_rotation_angle(rotation, Vec3::X);
        assert_relative_eq!(angle, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_rotation_sensitivity_scaling() {
        // Higher sensitivity = more rotation per pixel
        let low_sensitivity = 0.1;
        let high_sensitivity = 1.0;
        let mouse_delta = Vec2::new(50.0, 0.0);

        let rot_low = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::X,
            low_sensitivity,
            false,
            Quat::IDENTITY,
            false,
        );

        let rot_high = RotateGizmo::calculate_rotation(
            mouse_delta,
            AxisConstraint::X,
            high_sensitivity,
            false,
            Quat::IDENTITY,
            false,
        );

        let angle_low = RotateGizmo::get_rotation_angle(rot_low, Vec3::X);
        let angle_high = RotateGizmo::get_rotation_angle(rot_high, Vec3::X);

        // High sensitivity should produce 10× larger rotation
        assert_relative_eq!(angle_high / angle_low, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_rotation_local_space_rotated_object() {
        // Rotate object 90° around Y, then rotate around local X
        let object_rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let rotation = RotateGizmo::calculate_rotation_numeric(
            45.0,
            AxisConstraint::X, // Local X axis
            object_rotation,
            true, // local space
        );

        // Rotation should be applied around object's local X axis
        // (which is world Z after 90° Y rotation)
        assert_ne!(rotation, Quat::IDENTITY);
    }
}
