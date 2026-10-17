//! Presentation-only two-bone IK. The sim never reads these joint positions.

use glam::Vec3;

/// Move the middle joint so the end reaches `target` when the chain can.
/// Returns `(mid, end)`.
pub fn solve_two_bone(root: Vec3, mid: Vec3, end: Vec3, target: Vec3) -> (Vec3, Vec3) {
    let len_ab = (mid - root).length().max(1e-4);
    let len_bc = (end - mid).length().max(1e-4);
    let mut to_target = target - root;
    let reach = len_ab + len_bc;
    let dist = to_target.length();
    if dist < 1e-4 {
        return (mid, root);
    }
    if dist >= reach {
        let dir = to_target / dist;
        let new_mid = root + dir * len_ab;
        let new_end = root + dir * reach;
        return (new_mid, new_end);
    }
    to_target /= dist;
    let adjacent = (len_ab * len_ab - len_bc * len_bc + dist * dist) / (2.0 * dist);
    let height_sq = (len_ab * len_ab - adjacent * adjacent).max(0.0);
    let height = height_sq.sqrt();
    let pole = {
        let bend = (mid - root).cross(to_target);
        if bend.length_squared() < 1e-6 {
            Vec3::Y
        } else {
            bend.normalize()
        }
    };
    let new_mid = root + to_target * adjacent + pole * height;
    let new_end = target;
    (new_mid, new_end)
}

/// Pelvis lift so a foot planted at `ground_y` meets the hip chain.
pub fn foot_ground_offset(hip_y: f32, foot_y: f32, ground_y: f32) -> f32 {
    let plant = ground_y - foot_y;
    if hip_y + plant < ground_y {
        ground_y - hip_y
    } else {
        plant
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn hand_ik_reaches_the_climb_anchor() {
        let root = Vec3::new(0.0, 1.4, 0.0);
        let mid = Vec3::new(0.2, 1.2, 0.1);
        let end = Vec3::new(0.4, 1.0, 0.2);
        let target = Vec3::new(0.3, 1.5, 0.4);
        let (_mid, solved) = solve_two_bone(root, mid, end, target);
        let gap = (solved - target).length();
        assert!(gap < 1e-3, "hand missed the anchor by {gap}");
    }

    #[test]
    fn foot_ik_lifts_to_the_ground_sample() {
        let offset = foot_ground_offset(1.0, 0.0, 0.2);
        assert!((offset - 0.2).abs() < 1e-4);
    }
}
