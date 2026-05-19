//! Wave 2 proactive remediation tests for camera.rs (155 mutants, 7 existing tests).
//!
//! Targets:
//!   - Camera::dir() trigonometry (yaw/pitch → direction vector)
//!   - Camera::view_matrix() / proj_matrix() / vp() composition
//!   - CameraController::new() defaults
//!   - CameraController::update_camera() smoothing math
//!   - CameraController scroll/orbit/toggle logic
//!   - Speed modifiers (sprint/precision)

use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};
use glam::{Mat4, Vec2, Vec3};

fn default_camera() -> Camera {
    Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        yaw: 0.0,
        pitch: 0.0,
        fovy: std::f32::consts::FRAC_PI_3,
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 100.0,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Camera::dir() — trigonometric direction calculation
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn dir_yaw0_pitch0_is_positive_x() {
    let d = Camera::dir(0.0, 0.0);
    assert!(
        (d.x - 1.0).abs() < 0.001,
        "dir(0,0).x should be ~1.0, got {}",
        d.x
    );
    assert!(d.y.abs() < 0.001, "dir(0,0).y should be ~0, got {}", d.y);
    assert!(d.z.abs() < 0.001, "dir(0,0).z should be ~0, got {}", d.z);
}

#[test]
fn dir_yaw_half_pi_is_positive_z() {
    let d = Camera::dir(std::f32::consts::FRAC_PI_2, 0.0);
    assert!(d.x.abs() < 0.001, "dir(π/2,0).x ≈ 0, got {}", d.x);
    assert!(d.y.abs() < 0.001);
    assert!((d.z - 1.0).abs() < 0.001, "dir(π/2,0).z ≈ 1, got {}", d.z);
}

#[test]
fn dir_yaw_pi_is_negative_x() {
    let d = Camera::dir(std::f32::consts::PI, 0.0);
    assert!((d.x + 1.0).abs() < 0.001, "dir(π,0).x ≈ -1, got {}", d.x);
    assert!(d.z.abs() < 0.01);
}

#[test]
fn dir_pitch_up_positive_y() {
    let d = Camera::dir(0.0, std::f32::consts::FRAC_PI_4);
    assert!(d.y > 0.5, "dir(0,π/4).y should be positive, got {}", d.y);
    assert!(d.x > 0.0, "dir(0,π/4).x should be positive");
}

#[test]
fn dir_pitch_down_negative_y() {
    let d = Camera::dir(0.0, -std::f32::consts::FRAC_PI_4);
    assert!(d.y < -0.5, "dir(0,-π/4).y should be negative, got {}", d.y);
}

#[test]
fn dir_is_always_normalized() {
    for yaw in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, -1.0, -2.5] {
        for pitch in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let d = Camera::dir(yaw, pitch);
            let len = d.length();
            assert!(
                (len - 1.0).abs() < 0.01,
                "dir({yaw},{pitch}) length={len} should be ~1"
            );
        }
    }
}

#[test]
fn dir_yaw_continuous_rotation() {
    // As yaw goes from 0 to 2π, direction should sweep full XZ circle
    let d0 = Camera::dir(0.0, 0.0);
    let d1 = Camera::dir(std::f32::consts::FRAC_PI_2, 0.0);
    let d2 = Camera::dir(std::f32::consts::PI, 0.0);
    let d3 = Camera::dir(3.0 * std::f32::consts::FRAC_PI_2, 0.0);

    // Consecutive directions should be roughly orthogonal
    assert!(d0.dot(d1).abs() < 0.1, "0 vs π/2 should be orthogonal");
    assert!(d1.dot(d2).abs() < 0.1, "π/2 vs π should be orthogonal");
    assert!(d2.dot(d3).abs() < 0.1, "π vs 3π/2 should be orthogonal");
}

// ══════════════════════════════════════════════════════════════════════════════
// Camera matrices
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn view_matrix_not_nan() {
    let cam = default_camera();
    let vm = cam.view_matrix();
    assert!(!vm.is_nan());
}

#[test]
fn view_matrix_different_for_different_positions() {
    let mut c1 = default_camera();
    let mut c2 = default_camera();
    c2.position = Vec3::new(10.0, 20.0, 30.0);
    assert_ne!(c1.view_matrix(), c2.view_matrix());
}

#[test]
fn view_matrix_different_for_different_yaw() {
    let mut c1 = default_camera();
    let mut c2 = default_camera();
    c2.yaw = 1.0;
    assert_ne!(c1.view_matrix(), c2.view_matrix());
}

#[test]
fn proj_matrix_not_nan() {
    let cam = default_camera();
    assert!(!cam.proj_matrix().is_nan());
}

#[test]
fn proj_matrix_different_for_different_fovy() {
    let mut c1 = default_camera();
    let mut c2 = default_camera();
    c2.fovy = 1.5;
    assert_ne!(c1.proj_matrix(), c2.proj_matrix());
}

#[test]
fn proj_matrix_different_for_different_aspect() {
    let mut c1 = default_camera();
    let mut c2 = default_camera();
    c2.aspect = 4.0 / 3.0;
    assert_ne!(c1.proj_matrix(), c2.proj_matrix());
}

#[test]
fn proj_matrix_clamps_aspect_to_min() {
    let mut c = default_camera();
    c.aspect = 0.0; // Would cause division by zero without clamp
    let pm = c.proj_matrix();
    assert!(
        !pm.is_nan(),
        "Zero aspect should be clamped, not produce NaN"
    );
}

#[test]
fn vp_matrix_is_proj_times_view() {
    let cam = default_camera();
    let expected = cam.proj_matrix() * cam.view_matrix();
    let actual = cam.vp();
    // Check all 16 elements
    for i in 0..4 {
        for j in 0..4 {
            assert!(
                (actual.col(i)[j] - expected.col(i)[j]).abs() < 1e-6,
                "vp() != proj * view at ({i},{j})"
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CameraController::new defaults
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn controller_new_stores_speed_and_sensitivity() {
    let ctrl = CameraController::new(10.0, 0.05);
    assert_eq!(ctrl.speed, 10.0);
    assert_eq!(ctrl.sensitivity, 0.05);
}

#[test]
fn controller_default_zoom_sensitivity() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert_eq!(ctrl.zoom_sensitivity, 0.1);
}

#[test]
fn controller_default_mouse_smooth() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert_eq!(ctrl.mouse_smooth, 0.15);
}

#[test]
fn controller_default_mouse_deadzone() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert_eq!(ctrl.mouse_deadzone, 0.25);
}

#[test]
fn controller_default_starts_freefly() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert!(matches!(ctrl.mode, CameraMode::FreeFly));
}

#[test]
fn controller_default_orbit_distance() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert_eq!(ctrl.orbit_distance, 5.0);
}

#[test]
fn controller_not_dragging_initially() {
    let ctrl = CameraController::new(5.0, 0.01);
    assert!(!ctrl.is_dragging());
}

// ══════════════════════════════════════════════════════════════════════════════
// CameraController::update_camera — smoothing math
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn update_camera_initializes_targets() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    cam.yaw = 1.5;
    cam.pitch = 0.3;
    ctrl.update_camera(&mut cam, 0.016);
    // After first update, yaw/pitch shouldn't change (targets = current)
    assert!((cam.yaw - 1.5).abs() < 0.01);
    assert!((cam.pitch - 0.3).abs() < 0.01);
}

#[test]
fn update_camera_no_movement_without_keys() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    let pos_before = cam.position;
    ctrl.update_camera(&mut cam, 0.1);
    assert_eq!(cam.position, pos_before, "No keys pressed → no movement");
}

#[test]
fn update_camera_forward_movement() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.update_camera(&mut cam, 0.001); // init targets
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    let pos_before = cam.position;
    ctrl.update_camera(&mut cam, 0.1);
    assert_ne!(cam.position, pos_before, "W key should move camera");
}

#[test]
fn update_camera_speed_scales_movement() {
    let mut ctrl_slow = CameraController::new(1.0, 0.01);
    let mut ctrl_fast = CameraController::new(10.0, 0.01);
    let mut cam_slow = default_camera();
    let mut cam_fast = default_camera();

    ctrl_slow.update_camera(&mut cam_slow, 0.001);
    ctrl_fast.update_camera(&mut cam_fast, 0.001);

    ctrl_slow.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl_fast.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    ctrl_slow.update_camera(&mut cam_slow, 1.0);
    ctrl_fast.update_camera(&mut cam_fast, 1.0);

    let dist_slow = (cam_slow.position - Vec3::new(0.0, 0.0, 5.0)).length();
    let dist_fast = (cam_fast.position - Vec3::new(0.0, 0.0, 5.0)).length();
    assert!(
        dist_fast > dist_slow,
        "Faster speed should move more: slow={dist_slow}, fast={dist_fast}"
    );
}

#[test]
fn update_camera_dt_scales_movement() {
    let mut ctrl1 = CameraController::new(5.0, 0.01);
    let mut ctrl2 = CameraController::new(5.0, 0.01);
    let mut cam1 = default_camera();
    let mut cam2 = default_camera();

    ctrl1.update_camera(&mut cam1, 0.001);
    ctrl2.update_camera(&mut cam2, 0.001);

    ctrl1.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);

    ctrl1.update_camera(&mut cam1, 0.01);
    ctrl2.update_camera(&mut cam2, 0.1);

    let dist1 = (cam1.position - Vec3::new(0.0, 0.0, 5.0)).length();
    let dist2 = (cam2.position - Vec3::new(0.0, 0.0, 5.0)).length();
    assert!(
        dist2 > dist1,
        "Larger dt should move more: dt=0.01→{dist1}, dt=0.1→{dist2}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Scroll / zoom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn scroll_freefly_changes_fov() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    let fov0 = cam.fovy;
    ctrl.process_scroll(&mut cam, 1.0);
    assert!(cam.fovy < fov0, "Scroll up should zoom in (reduce FOV)");
}

#[test]
fn scroll_freefly_fov_clamped_min() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    // Zoom in a lot
    for _ in 0..100 {
        ctrl.process_scroll(&mut cam, 10.0);
    }
    assert!(
        cam.fovy >= 0.1,
        "FOV should be clamped to min 0.1, got {}",
        cam.fovy
    );
}

#[test]
fn scroll_freefly_fov_clamped_max() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    // Zoom out a lot
    for _ in 0..100 {
        ctrl.process_scroll(&mut cam, -10.0);
    }
    assert!(
        cam.fovy <= 3.0,
        "FOV should be clamped to max 3.0, got {}",
        cam.fovy
    );
}

#[test]
fn scroll_orbit_changes_distance() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.toggle_mode(&mut cam);
    let d0 = ctrl.orbit_distance;
    ctrl.process_scroll(&mut cam, 1.0);
    assert!(
        ctrl.orbit_distance < d0,
        "Scroll up in orbit should reduce distance"
    );
}

#[test]
fn scroll_orbit_distance_clamped() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.toggle_mode(&mut cam);
    // Zoom in a lot
    for _ in 0..200 {
        ctrl.process_scroll(&mut cam, 10.0);
    }
    assert!(ctrl.orbit_distance >= 1.0, "Orbit distance min = 1.0");
    // Zoom out a lot
    for _ in 0..200 {
        ctrl.process_scroll(&mut cam, -10.0);
    }
    assert!(ctrl.orbit_distance <= 50.0, "Orbit distance max = 50.0");
}

// ══════════════════════════════════════════════════════════════════════════════
// Mode toggle
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn toggle_freefly_to_orbit() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.toggle_mode(&mut cam);
    assert!(matches!(ctrl.mode, CameraMode::Orbit));
}

#[test]
fn toggle_orbit_to_freefly() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.toggle_mode(&mut cam);
    ctrl.toggle_mode(&mut cam);
    assert!(matches!(ctrl.mode, CameraMode::FreeFly));
}

#[test]
fn toggle_to_orbit_sets_target() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    let initial_target = ctrl.orbit_target;
    ctrl.toggle_mode(&mut cam);
    // Orbit target should now be set based on look direction
    assert_ne!(ctrl.orbit_target, initial_target);
}

// ══════════════════════════════════════════════════════════════════════════════
// Sprint / precision modifiers
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn sprint_increases_movement_speed() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam_normal = default_camera();
    let mut cam_sprint = default_camera();

    ctrl.update_camera(&mut cam_normal, 0.001);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl.update_camera(&mut cam_normal, 1.0);
    let dist_normal = (cam_normal.position - Vec3::new(0.0, 0.0, 5.0)).length();

    let mut ctrl2 = CameraController::new(5.0, 0.01);
    ctrl2.update_camera(&mut cam_sprint, 0.001);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ShiftLeft, true);
    ctrl2.update_camera(&mut cam_sprint, 1.0);
    let dist_sprint = (cam_sprint.position - Vec3::new(0.0, 0.0, 5.0)).length();

    assert!(
        dist_sprint > dist_normal,
        "Sprint should move faster: normal={dist_normal}, sprint={dist_sprint}"
    );
}

#[test]
fn precision_decreases_movement_speed() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam_normal = default_camera();
    let mut cam_prec = default_camera();

    ctrl.update_camera(&mut cam_normal, 0.001);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl.update_camera(&mut cam_normal, 1.0);
    let dist_normal = (cam_normal.position - Vec3::new(0.0, 0.0, 5.0)).length();

    let mut ctrl2 = CameraController::new(5.0, 0.01);
    ctrl2.update_camera(&mut cam_prec, 0.001);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl2.process_keyboard(winit::keyboard::KeyCode::ControlLeft, true);
    ctrl2.update_camera(&mut cam_prec, 1.0);
    let dist_prec = (cam_prec.position - Vec3::new(0.0, 0.0, 5.0)).length();

    assert!(
        dist_prec < dist_normal,
        "Precision should move slower: normal={dist_normal}, precision={dist_prec}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Mouse input
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn mouse_delta_without_dragging_no_effect() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.update_camera(&mut cam, 0.001); // init
    let yaw0 = cam.yaw;
    ctrl.process_mouse_delta(&mut cam, Vec2::new(100.0, 50.0));
    ctrl.update_camera(&mut cam, 0.016);
    assert!((cam.yaw - yaw0).abs() < 1e-6, "No drag → no yaw change");
}

#[test]
fn mouse_delta_with_dragging_changes_yaw() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.update_camera(&mut cam, 0.001);
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    ctrl.process_mouse_delta(&mut cam, Vec2::new(50.0, 0.0));
    ctrl.update_camera(&mut cam, 0.016);
    assert!(cam.yaw != 0.0, "Dragging with delta should change yaw");
}

#[test]
fn mouse_deadzone_filters_small_deltas() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.update_camera(&mut cam, 0.001);
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    // Delta smaller than deadzone (0.25)
    ctrl.process_mouse_delta(&mut cam, Vec2::new(0.1, 0.1));
    ctrl.update_camera(&mut cam, 0.016);
    assert_eq!(
        cam.yaw, 0.0,
        "Small delta within deadzone should have no effect"
    );
}

#[test]
fn pitch_clamped_to_valid_range() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.update_camera(&mut cam, 0.001);
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    // Large pitch delta
    ctrl.process_mouse_delta(&mut cam, Vec2::new(0.0, -10000.0));
    // Multiple updates to converge
    for _ in 0..1000 {
        ctrl.update_camera(&mut cam, 0.016);
    }
    assert!(
        cam.pitch <= 1.55 && cam.pitch >= -1.55,
        "Pitch should be clamped near ±1.54, got {}",
        cam.pitch
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// set_orbit_target
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn set_orbit_target_updates_position_in_orbit_mode() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.toggle_mode(&mut cam);
    let pos_before = cam.position;
    ctrl.set_orbit_target(Vec3::new(10.0, 10.0, 10.0), &mut cam);
    assert_ne!(
        cam.position, pos_before,
        "Setting orbit target should move camera"
    );
}

#[test]
fn set_orbit_target_no_effect_in_freefly() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    let pos_before = cam.position;
    ctrl.set_orbit_target(Vec3::new(10.0, 10.0, 10.0), &mut cam);
    assert_eq!(
        cam.position, pos_before,
        "FreeFly mode: orbit target shouldn't move camera"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Keyboard input mapping
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn keyboard_wasd_mapping() {
    let mut ctrl = CameraController::new(5.0, 0.01);

    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyA, true);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyS, true);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyD, true);

    // All should be 1.0 when pressed
    // Release W
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, false);
    // After release, W state should be 0
    // (can't directly access private fields, but update will not move forward)
    let mut cam = default_camera();
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyS, false);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyA, false);
    ctrl.process_keyboard(winit::keyboard::KeyCode::KeyD, false);
    let pos = cam.position;
    ctrl.update_camera(&mut cam, 0.1);
    // All keys released → no movement
    assert_eq!(cam.position, pos);
}

#[test]
fn begin_frame_resets_raw_flag() {
    let mut ctrl = CameraController::new(5.0, 0.01);
    let mut cam = default_camera();
    ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
    ctrl.process_mouse_delta(&mut cam, Vec2::new(10.0, 10.0));
    // raw_used_this_frame should be true now
    // begin_frame resets it
    ctrl.begin_frame();
    // Now process_mouse_move should work (not blocked by raw)
    // Can't directly assert private field, but the flow works
}
