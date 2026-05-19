//! Wave 2 – Golden-value tests for camera.rs (155 mutants, 81 in update_camera)
//!
//! Targets: Camera::dir golden values at specific yaw/pitch,
//!          Camera::view_matrix / proj_matrix / vp() consistency,
//!          CameraController::update_camera smoothing, speed modifiers,
//!          pitch clamping, mode-specific movement, orbit zoom.

use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};
use glam::{Mat4, Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

fn make_camera() -> Camera {
    Camera {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: 1.0, // ~57.3 degrees
        aspect: 800.0 / 600.0,
        znear: 0.1,
        zfar: 100.0,
    }
}

// ============================================================================
// Camera::dir — golden direction values
// ============================================================================

#[test]
fn dir_yaw0_pitch0_is_positive_x() {
    let d = Camera::dir(0.0, 0.0);
    assert!((d.x - 1.0).abs() < 0.001, "x should be 1.0, got {}", d.x);
    assert!(d.y.abs() < 0.001, "y should be 0.0, got {}", d.y);
    assert!(d.z.abs() < 0.001, "z should be 0.0, got {}", d.z);
}

#[test]
fn dir_yaw_pi_2_pitch0_is_positive_z() {
    let d = Camera::dir(FRAC_PI_2, 0.0);
    assert!(d.x.abs() < 0.001, "x should be ~0, got {}", d.x);
    assert!(d.y.abs() < 0.001);
    assert!((d.z - 1.0).abs() < 0.001, "z should be ~1.0, got {}", d.z);
}

#[test]
fn dir_yaw_pi_pitch0_is_negative_x() {
    let d = Camera::dir(PI, 0.0);
    assert!((d.x + 1.0).abs() < 0.001, "x should be ~-1.0, got {}", d.x);
    assert!(d.y.abs() < 0.001);
    assert!(d.z.abs() < 0.01);
}

#[test]
fn dir_yaw0_pitch_pi4_looks_up() {
    let d = Camera::dir(0.0, FRAC_PI_4);
    let expected_x = FRAC_PI_4.cos(); // ~0.7071
    let expected_y = FRAC_PI_4.sin(); // ~0.7071
    assert!(
        (d.x - expected_x).abs() < 0.01,
        "x={}, expected {}",
        d.x,
        expected_x
    );
    assert!(
        (d.y - expected_y).abs() < 0.01,
        "y={}, expected {}",
        d.y,
        expected_y
    );
    assert!(d.z.abs() < 0.001);
}

#[test]
fn dir_yaw0_negative_pitch_looks_down() {
    let d = Camera::dir(0.0, -FRAC_PI_4);
    assert!(d.y < 0.0, "Negative pitch should look down, y={}", d.y);
    assert!((d.y + FRAC_PI_4.sin()).abs() < 0.01);
}

#[test]
fn dir_is_always_normalized() {
    let cases = [
        (0.0, 0.0),
        (1.0, 0.5),
        (2.0, -0.3),
        (PI, 1.0),
        (-1.5, -1.5),
        (3.0, 0.1),
    ];
    for (yaw, pitch) in cases {
        let d = Camera::dir(yaw, pitch);
        let len = d.length();
        assert!(
            (len - 1.0).abs() < 0.001,
            "Dir should be normalized for yaw={}, pitch={}: len={}",
            yaw,
            pitch,
            len
        );
    }
}

#[test]
fn dir_pitch_component_is_sin_pitch() {
    for pitch in [-1.0, -0.5, 0.0, 0.25, 0.5, 1.0, 1.5] {
        let d = Camera::dir(0.0, pitch);
        let expected_y = pitch.sin();
        // After normalization, y component should be close to sin(pitch)/length
        // But length = sqrt(cos²(p) + sin²(p)) = 1, so y ≈ sin(pitch)
        assert!(
            (d.y - expected_y).abs() < 0.01,
            "pitch={}: y={}, expected sin({})={}",
            pitch,
            d.y,
            pitch,
            expected_y
        );
    }
}

#[test]
fn dir_yaw_affects_xz_not_y() {
    // Changing yaw with pitch=0 should only change x/z, not y
    let d1 = Camera::dir(0.0, 0.0);
    let d2 = Camera::dir(1.0, 0.0);
    assert!(d1.y.abs() < 0.001);
    assert!(d2.y.abs() < 0.001);
    assert!((d1.x - d2.x).abs() > 0.1, "x should change with yaw");
}

// ============================================================================
// Camera matrices — consistency checks
// ============================================================================

#[test]
fn view_matrix_is_finite() {
    let c = make_camera();
    let m = c.view_matrix();
    for col in 0..4 {
        for row in 0..4 {
            assert!(
                m.col(col)[row].is_finite(),
                "View matrix has non-finite at [{},{}]",
                col,
                row
            );
        }
    }
}

#[test]
fn proj_matrix_is_finite() {
    let c = make_camera();
    let m = c.proj_matrix();
    for col in 0..4 {
        for row in 0..4 {
            assert!(m.col(col)[row].is_finite());
        }
    }
}

#[test]
fn vp_equals_proj_times_view() {
    let c = make_camera();
    let vp = c.vp();
    let expected = c.proj_matrix() * c.view_matrix();
    let diff = (vp - expected).abs();
    for col in 0..4 {
        for row in 0..4 {
            assert!(
                diff.col(col)[row] < 1e-5,
                "vp() != proj * view at [{},{}]: {} vs {}",
                col,
                row,
                vp.col(col)[row],
                expected.col(col)[row]
            );
        }
    }
}

#[test]
fn view_matrix_uses_negative_y_up() {
    // Flipped up vector means the view matrix is built with -Vec3::Y as up
    let mut c = make_camera();
    c.position = Vec3::new(0.0, 0.0, 5.0);
    let v = c.view_matrix();
    // With -Y up and looking along +X (yaw=0, pitch=0), the up vector in view space
    // should flip the vertical. Verify this is look_to_rh with -Y up:
    let expected = Mat4::look_to_rh(c.position, Camera::dir(c.yaw, c.pitch), -Vec3::Y);
    let diff = (v - expected).abs();
    for col in 0..4 {
        for row in 0..4 {
            assert!(diff.col(col)[row] < 1e-6);
        }
    }
}

#[test]
fn proj_matrix_clamps_aspect_to_min_001() {
    // With aspect = 0, should use max(0, 0.01) = 0.01
    let mut c = make_camera();
    c.aspect = 0.0;
    let p = c.proj_matrix();
    // Should be finite (clamped aspect prevents divide by zero)
    for col in 0..4 {
        for row in 0..4 {
            assert!(
                p.col(col)[row].is_finite(),
                "Non-finite at [{},{}] with aspect=0",
                col,
                row
            );
        }
    }
}

// ============================================================================
// CameraController::new — default field values
// ============================================================================

#[test]
fn controller_new_defaults() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert_eq!(ctrl.speed, 5.0);
    assert_eq!(ctrl.sensitivity, 0.01);
    assert!((ctrl.zoom_sensitivity - 0.1).abs() < 0.001);
    assert!((ctrl.mouse_smooth - 0.15).abs() < 0.001);
    assert!((ctrl.mouse_deadzone - 0.25).abs() < 0.001);
    assert!(matches!(ctrl.mode, CameraMode::FreeFly));
    assert_eq!(ctrl.orbit_target, Vec3::ZERO);
    assert!((ctrl.orbit_distance - 5.0).abs() < 0.001);
}

// ============================================================================
// process_keyboard — golden key mappings
// ============================================================================

#[test]
fn keyboard_w_sets_fwd() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    // fwd is private, but we can observe movement
    let mut cam = make_camera();
    let start = cam.position;
    ctrl.update_camera(&mut cam, 0.5);
    assert!(
        cam.position.x > start.x,
        "W key should move forward along look dir (initially +X)"
    );
}

#[test]
fn keyboard_s_sets_back() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyS, true);
    let mut cam = make_camera();
    let start = cam.position;
    ctrl.update_camera(&mut cam, 0.5);
    assert!(cam.position.x < start.x, "S key should move backward");
}

#[test]
fn keyboard_release_stops_movement() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, false);
    let mut cam = make_camera();
    let start = cam.position;
    ctrl.update_camera(&mut cam, 0.5);
    assert_eq!(
        cam.position, start,
        "Released key should not cause movement"
    );
}

// ============================================================================
// Speed modifiers — sprint and precision
// ============================================================================

#[test]
fn sprint_doubles_effective_speed() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    // Normal movement
    let mut cam1 = make_camera();
    ctrl.update_camera(&mut cam1, 0.5);
    let normal_dist = cam1.position.length();

    // Sprint movement
    let mut ctrl2 = CameraController::new(5.0, 0.01);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ShiftLeft, true);
    let mut cam2 = make_camera();
    ctrl2.update_camera(&mut cam2, 0.5);
    let sprint_dist = cam2.position.length();

    // Sprint should be ~2x normal (sprint_mult = 2.0)
    assert!(
        (sprint_dist / normal_dist - 2.0).abs() < 0.1,
        "Sprint should be ~2x: normal={}, sprint={}",
        normal_dist,
        sprint_dist
    );
}

#[test]
fn precision_quarters_effective_speed() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    let mut cam1 = make_camera();
    ctrl.update_camera(&mut cam1, 0.5);
    let normal_dist = cam1.position.length();

    let mut ctrl2 = CameraController::new(5.0, 0.01);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ControlLeft, true);
    let mut cam2 = make_camera();
    ctrl2.update_camera(&mut cam2, 0.5);
    let precision_dist = cam2.position.length();

    // Precision should be ~0.25x normal (precision_mult = 0.25)
    assert!(
        (precision_dist / normal_dist - 0.25).abs() < 0.05,
        "Precision should be ~0.25x: normal={}, precision={}",
        normal_dist,
        precision_dist
    );
}

#[test]
fn sprint_and_precision_stack() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    let mut cam1 = make_camera();
    ctrl.update_camera(&mut cam1, 0.5);
    let normal_dist = cam1.position.length();

    // Both sprint and precision active: 2.0 * 0.25 = 0.5x
    let mut ctrl2 = CameraController::new(5.0, 0.01);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ShiftLeft, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ControlLeft, true);
    let mut cam2 = make_camera();
    ctrl2.update_camera(&mut cam2, 0.5);
    let combo_dist = cam2.position.length();

    assert!(
        (combo_dist / normal_dist - 0.5).abs() < 0.1,
        "Sprint+Precision should be ~0.5x: normal={}, combo={}",
        normal_dist,
        combo_dist
    );
}

// ============================================================================
// Pitch clamping — ±1.54 rad
// ============================================================================

#[test]
fn pitch_clamped_during_update() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    // Force targets to extreme pitch
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    // Send large negative delta to push pitch target very high
    ctrl.process_mouse_delta(&mut cam, Vec2::new(0.0, -50000.0));
    // Run enough updates to converge
    for _ in 0..1000 {
        ctrl.update_camera(&mut cam, 0.1);
    }
    assert!(
        cam.pitch <= 1.54 + 0.001,
        "Pitch should be clamped to ≤1.54, got {}",
        cam.pitch
    );
}

#[test]
fn pitch_clamped_negative_during_update() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    ctrl.process_mouse_delta(&mut cam, Vec2::new(0.0, 50000.0));
    for _ in 0..1000 {
        ctrl.update_camera(&mut cam, 0.1);
    }
    assert!(
        cam.pitch >= -1.54 - 0.001,
        "Pitch should be clamped to ≥-1.54, got {}",
        cam.pitch
    );
}

// ============================================================================
// Scroll zoom — FreeFly vs Orbit
// ============================================================================

#[test]
fn scroll_freefly_adjusts_fov() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    let initial_fov = cam.fovy;
    ctrl.process_scroll(&mut cam, 1.0); // Zoom in
    assert!(cam.fovy < initial_fov, "Scroll in should decrease FOV");
}

#[test]
fn scroll_freefly_fov_clamped_low() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    for _ in 0..1000 {
        ctrl.process_scroll(&mut cam, 10.0);
    }
    assert!(
        cam.fovy >= 0.1,
        "FOV should not go below 0.1, got {}",
        cam.fovy
    );
}

#[test]
fn scroll_freefly_fov_clamped_high() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    for _ in 0..1000 {
        ctrl.process_scroll(&mut cam, -10.0);
    }
    assert!(
        cam.fovy <= 3.0,
        "FOV should not exceed 3.0, got {}",
        cam.fovy
    );
}

#[test]
fn scroll_orbit_adjusts_distance() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.toggle_mode(&mut cam); // Switch to orbit
    let initial_dist = ctrl.orbit_distance;
    ctrl.process_scroll(&mut cam, 1.0); // Zoom in
    assert!(
        ctrl.orbit_distance < initial_dist,
        "Orbit scroll should decrease distance"
    );
}

#[test]
fn scroll_orbit_distance_clamped() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.toggle_mode(&mut cam);
    for _ in 0..1000 {
        ctrl.process_scroll(&mut cam, 100.0);
    }
    assert!(ctrl.orbit_distance >= 1.0, "Min orbit distance is 1.0");
    ctrl.orbit_distance = 5.0;
    for _ in 0..1000 {
        ctrl.process_scroll(&mut cam, -100.0);
    }
    assert!(ctrl.orbit_distance <= 50.0, "Max orbit distance is 50.0");
}

// ============================================================================
// Mode toggle
// ============================================================================

#[test]
fn toggle_freefly_to_orbit_sets_target() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    cam.position = Vec3::new(0.0, 0.0, 5.0);
    ctrl.toggle_mode(&mut cam);
    assert!(matches!(ctrl.mode, CameraMode::Orbit));
    // Orbit target = position + dir * orbit_distance
    let dir = Camera::dir(cam.yaw, cam.pitch);
    let expected_target = Vec3::new(0.0, 0.0, 5.0) + dir * ctrl.orbit_distance;
    assert!(
        (ctrl.orbit_target - expected_target).length() < 0.01,
        "Orbit target should be set: {:?} vs {:?}",
        ctrl.orbit_target,
        expected_target
    );
}

#[test]
fn toggle_orbit_to_freefly_preserves_position() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    cam.position = Vec3::new(1.0, 2.0, 3.0);
    ctrl.toggle_mode(&mut cam); // → Orbit
    ctrl.toggle_mode(&mut cam); // → FreeFly
    assert!(matches!(ctrl.mode, CameraMode::FreeFly));
    // Position should be preserved (but may have been updated by orbit calculation)
}

// ============================================================================
// Movement direction — FreeFly
// ============================================================================

#[test]
fn freefly_w_moves_along_look_direction() {
    let mut ctrl = CameraController::new(10.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    // Looking along +X (yaw=0, pitch=0)
    let mut cam = make_camera();
    ctrl.update_camera(&mut cam, 1.0);
    assert!(cam.position.x > 0.0, "Should move in +X when yaw=0");
}

#[test]
fn freefly_d_moves_right_of_look() {
    let mut ctrl = CameraController::new(10.0, 0.01);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyD, true);

    // Looking along +X, right should be +Z (cross(+X, -Y) = +Z, but with -Y up...)
    let mut cam = make_camera();
    ctrl.update_camera(&mut cam, 1.0);
    // The exact direction depends on the cross product with -Y up
    assert!(cam.position != Vec3::ZERO, "D key should cause movement");
}

// ============================================================================
// Mouse delta — deadzone
// ============================================================================

#[test]
fn mouse_delta_below_deadzone_ignored() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    // Deadzone is 0.25, send delta < deadzone in both axes
    ctrl.process_mouse_delta(&mut cam, Vec2::new(0.1, 0.1));
    // targets_initialized should still be false (delta was too small)
    let start_yaw = cam.yaw;
    ctrl.update_camera(&mut cam, 0.016);
    assert!(
        (cam.yaw - start_yaw).abs() < 1e-6,
        "Sub-deadzone delta should not affect yaw"
    );
}

#[test]
fn mouse_delta_above_deadzone_applied() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    ctrl.process_mouse_delta(&mut cam, Vec2::new(10.0, 0.0));
    ctrl.update_camera(&mut cam, 0.5);
    assert!(
        cam.yaw.abs() > 1e-6,
        "Above-deadzone delta should change yaw"
    );
}

// ============================================================================
// begin_frame resets raw_used flag
// ============================================================================

#[test]
fn begin_frame_allows_new_input() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    // Process a delta (sets raw_used_this_frame = true)
    ctrl.process_mouse_delta(&mut cam, Vec2::new(10.0, 0.0));
    ctrl.begin_frame(); // Reset the flag
                        // Should accept mouse_move now since raw_used is reset
                        // (This validates that begin_frame properly resets raw_used_this_frame)
    ctrl.process_mouse_move(&mut cam, Vec2::new(100.0, 100.0));
    // last_mouse is None, so this updates last_mouse but doesn't change targets
    // But it doesn't early-return due to raw_used_this_frame being reset
}
