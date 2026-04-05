//! Constraint application helpers.

use super::AxisConstraint;
use glam::Vec3;

/// Apply axis constraint to a vector.
pub fn apply_constraint(value: Vec3, constraint: AxisConstraint) -> Vec3 {
    let mask = constraint.axis_vector();
    value * mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_constraint_x() {
        let input = Vec3::new(5.0, 3.0, 2.0);
        let result = apply_constraint(input, AxisConstraint::X);
        assert_eq!(result, Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn test_apply_constraint_xy() {
        let input = Vec3::new(5.0, 3.0, 2.0);
        let result = apply_constraint(input, AxisConstraint::XY);
        assert_eq!(result, Vec3::new(5.0, 3.0, 0.0));
    }

    #[test]
    fn test_apply_constraint_none() {
        let input = Vec3::new(5.0, 3.0, 2.0);
        let result = apply_constraint(input, AxisConstraint::None);
        assert_eq!(result, input);
    }
}
