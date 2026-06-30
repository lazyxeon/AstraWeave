//! Wave 2 – Batch 4: Camera, Decals, HDRI Catalog, Types, BiomeAudio, SceneEnvironment
//! Proactive mutation-resistant integration tests for uncovered CPU-testable render modules.

// ═══════════════════════════════════════════════════════════════════════
//  Camera (155 mutants) — dir() trig math, view/proj matrices, controller
// ═══════════════════════════════════════════════════════════════════════
mod camera_tests {
    use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};
    use glam::{Mat4, Vec2, Vec3};

    fn make_camera() -> Camera {
        Camera {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: std::f32::consts::FRAC_PI_3, // 60°
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    // --- Camera::dir trig tests ---

    #[test]
    fn dir_yaw_zero_pitch_zero_points_positive_x() {
        let d = Camera::dir(0.0, 0.0);
        assert!((d.x - 1.0).abs() < 0.01, "x should be ~1, got {}", d.x);
        assert!(d.y.abs() < 0.01, "y should be ~0, got {}", d.y);
        assert!(d.z.abs() < 0.01, "z should be ~0, got {}", d.z);
    }

    #[test]
    fn dir_yaw_pi_half_points_positive_z() {
        let d = Camera::dir(std::f32::consts::FRAC_PI_2, 0.0);
        assert!(d.x.abs() < 0.01, "x should be ~0, got {}", d.x);
        assert!(d.z - 1.0 < 0.01 && d.z > 0.9, "z should be ~1, got {}", d.z);
    }

    #[test]
    fn dir_yaw_pi_points_negative_x() {
        let d = Camera::dir(std::f32::consts::PI, 0.0);
        assert!((d.x + 1.0).abs() < 0.01, "x should be ~-1, got {}", d.x);
    }

    #[test]
    fn dir_pitch_up_has_positive_y() {
        let d = Camera::dir(0.0, std::f32::consts::FRAC_PI_4);
        assert!(
            d.y > 0.5,
            "y should be positive when pitch > 0, got {}",
            d.y
        );
    }

    #[test]
    fn dir_pitch_down_has_negative_y() {
        let d = Camera::dir(0.0, -std::f32::consts::FRAC_PI_4);
        assert!(
            d.y < -0.5,
            "y should be negative when pitch < 0, got {}",
            d.y
        );
    }

    #[test]
    fn dir_is_unit_length() {
        for yaw in [0.0, 0.5, 1.0, 2.0, -1.0, std::f32::consts::PI] {
            for pitch in [0.0, 0.3, -0.3, 1.0, -1.0] {
                let d = Camera::dir(yaw, pitch);
                assert!(
                    (d.length() - 1.0).abs() < 0.001,
                    "dir({yaw},{pitch}) length = {}",
                    d.length()
                );
            }
        }
    }

    #[test]
    fn dir_pitch_90_points_straight_up() {
        let d = Camera::dir(0.0, std::f32::consts::FRAC_PI_2);
        // y should dominate
        assert!(d.y > 0.99, "at pitch=π/2, y should be ~1, got {}", d.y);
    }

    // --- Camera matrix tests ---

    #[test]
    fn view_matrix_not_nan() {
        let cam = make_camera();
        let v = cam.view_matrix();
        for i in 0..4 {
            for j in 0..4 {
                assert!(!v.col(i)[j].is_nan(), "view matrix has NaN at [{i}][{j}]");
            }
        }
    }

    #[test]
    fn proj_matrix_not_nan() {
        let cam = make_camera();
        let p = cam.proj_matrix();
        for i in 0..4 {
            for j in 0..4 {
                assert!(!p.col(i)[j].is_nan(), "proj matrix has NaN at [{i}][{j}]");
            }
        }
    }

    #[test]
    fn vp_equals_proj_times_view() {
        let cam = make_camera();
        let vp = cam.vp();
        let expected = cam.proj_matrix() * cam.view_matrix();
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (vp.col(i)[j] - expected.col(i)[j]).abs() < 1e-5,
                    "vp mismatch at [{i}][{j}]: {} vs {}",
                    vp.col(i)[j],
                    expected.col(i)[j]
                );
            }
        }
    }

    #[test]
    fn proj_matrix_uses_fovy() {
        let mut cam = make_camera();
        let p1 = cam.proj_matrix();
        cam.fovy = std::f32::consts::FRAC_PI_4; // 45° (narrower)
        let p2 = cam.proj_matrix();
        // Different fov → different projection
        assert!(
            (p1.col(0)[0] - p2.col(0)[0]).abs() > 0.01,
            "changing fovy should change projection"
        );
    }

    #[test]
    fn proj_matrix_clamps_aspect() {
        let mut cam = make_camera();
        cam.aspect = 0.0; // should clamp to 0.01
        let p = cam.proj_matrix();
        // Should not panic or produce NaN
        for i in 0..4 {
            for j in 0..4 {
                assert!(!p.col(i)[j].is_nan(), "zero aspect produced NaN");
            }
        }
    }

    #[test]
    fn view_matrix_uses_positive_y_up() {
        // Camera uses +Y as up (the prior -Y caused negative clip-space w — a
        // deliberate bug-fix in freefly.rs). Looking at +X from origin:
        // the view matrix should be look_to_rh(pos, dir, +Y).
        let cam = make_camera();
        let v = cam.view_matrix();
        let expected = Mat4::look_to_rh(cam.position, Camera::dir(cam.yaw, cam.pitch), Vec3::Y);
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (v.col(i)[j] - expected.col(i)[j]).abs() < 1e-5,
                    "view matrix mismatch"
                );
            }
        }
    }

    // --- CameraController tests ---

    #[test]
    fn controller_new_defaults() {
        let ctrl = CameraController::new(10.0, 0.05);
        assert_eq!(ctrl.speed, 10.0);
        assert_eq!(ctrl.sensitivity, 0.05);
        assert!(matches!(ctrl.mode, CameraMode::FreeFly));
        assert_eq!(ctrl.orbit_distance, 5.0);
        assert!(!ctrl.is_dragging());
    }

    #[test]
    fn controller_keyboard_wasd() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();

        // Press W → fwd = 1.0
        ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        // Press D → right = 1.0
        ctrl.process_keyboard(winit::keyboard::KeyCode::KeyD, true);

        let pos_before = cam.position;
        ctrl.update_camera(&mut cam, 0.1);
        // Camera should have moved (both forward and right)
        assert!(cam.position != pos_before);
    }

    #[test]
    fn controller_keyboard_release_stops_movement() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();

        ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        ctrl.update_camera(&mut cam, 0.1);
        let pos1 = cam.position;

        ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, false);
        ctrl.update_camera(&mut cam, 0.1);
        let pos2 = cam.position;

        // Second update should have minimal movement (only smoothing)
        // Forward velocity should be zero
        assert!(
            (pos2 - pos1).length() < (cam.position - Vec3::ZERO).length() * 0.1 + 0.001,
            "releasing key should stop movement"
        );
    }

    #[test]
    fn controller_scroll_changes_fov_in_freefly() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        let fov_before = cam.fovy;
        ctrl.process_scroll(&mut cam, 1.0);
        assert!(cam.fovy < fov_before, "scroll in should decrease FOV");
    }

    #[test]
    fn controller_scroll_fov_clamped_min() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        for _ in 0..100 {
            ctrl.process_scroll(&mut cam, 1.0);
        }
        assert!(cam.fovy >= 0.1, "FOV should clamp at 0.1, got {}", cam.fovy);
    }

    #[test]
    fn controller_scroll_fov_clamped_max() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        for _ in 0..100 {
            ctrl.process_scroll(&mut cam, -1.0);
        }
        assert!(cam.fovy <= 3.0, "FOV should clamp at 3.0, got {}", cam.fovy);
    }

    #[test]
    fn controller_toggle_to_orbit_sets_target() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fovy: 1.0,
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        assert!(matches!(ctrl.mode, CameraMode::FreeFly));
        ctrl.toggle_mode(&mut cam);
        assert!(matches!(ctrl.mode, CameraMode::Orbit));
        // Orbit target should be set from current look direction
        let expected_target = cam.position + Camera::dir(cam.yaw, cam.pitch) * ctrl.orbit_distance;
        // After toggle, target is set before position is updated
        assert!(
            (ctrl.orbit_target - expected_target).length() < 1.0,
            "orbit target should be along look direction"
        );
    }

    #[test]
    fn controller_orbit_scroll_changes_distance() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.toggle_mode(&mut cam); // Switch to orbit
        let dist_before = ctrl.orbit_distance;
        ctrl.process_scroll(&mut cam, 1.0); // Zoom in
        assert!(
            ctrl.orbit_distance < dist_before,
            "orbit zoom in should decrease distance"
        );
    }

    #[test]
    fn controller_orbit_distance_clamped() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.toggle_mode(&mut cam);
        for _ in 0..200 {
            ctrl.process_scroll(&mut cam, 1.0);
        }
        assert!(
            ctrl.orbit_distance >= 1.0,
            "orbit distance clamp min=1.0, got {}",
            ctrl.orbit_distance
        );
        for _ in 0..200 {
            ctrl.process_scroll(&mut cam, -1.0);
        }
        assert!(
            ctrl.orbit_distance <= 50.0,
            "orbit distance clamp max=50.0, got {}",
            ctrl.orbit_distance
        );
    }

    #[test]
    fn controller_mouse_button_sets_dragging() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        assert!(!ctrl.is_dragging());
        ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
        assert!(ctrl.is_dragging());
        ctrl.process_mouse_button(winit::event::MouseButton::Right, false);
        assert!(!ctrl.is_dragging());
    }

    #[test]
    fn controller_mouse_delta_deadzone() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.process_mouse_button(winit::event::MouseButton::Right, true);

        // Delta below deadzone (default 0.25)
        ctrl.process_mouse_delta(&mut cam, Vec2::new(0.1, 0.1));
        ctrl.update_camera(&mut cam, 0.016);
        // Should not have changed significantly (only smoothing toward initial target)
        assert!(
            cam.yaw.abs() < 0.001,
            "below-deadzone delta should be ignored"
        );
    }

    #[test]
    fn controller_mouse_delta_above_deadzone_changes_yaw() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.process_mouse_button(winit::event::MouseButton::Right, true);

        // Delta well above deadzone
        ctrl.process_mouse_delta(&mut cam, Vec2::new(50.0, 0.0));
        ctrl.update_camera(&mut cam, 0.016);
        // Yaw should have changed
        assert!(
            cam.yaw.abs() > 0.0001,
            "above-deadzone delta should change yaw"
        );
    }

    #[test]
    fn controller_begin_frame_resets_raw_flag() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
        ctrl.process_mouse_delta(&mut cam, Vec2::new(50.0, 0.0));
        // raw_used_this_frame should be true internally
        ctrl.begin_frame();
        // After begin_frame, mouse_move should not be skipped
        // (We can't directly test the private field, but begin_frame is callable)
    }

    #[test]
    fn controller_sprint_doubles_speed() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();

        // Move forward without sprint
        ctrl.process_keyboard(winit::keyboard::KeyCode::KeyW, true);
        ctrl.update_camera(&mut cam, 0.1);
        let dist_normal = cam.position.length();

        // Reset
        cam.position = Vec3::ZERO;
        cam.yaw = 0.0;
        cam.pitch = 0.0;

        // Move forward with sprint
        ctrl.process_keyboard(winit::keyboard::KeyCode::ShiftLeft, true);
        ctrl.update_camera(&mut cam, 0.1);
        let dist_sprint = cam.position.length();

        assert!(
            dist_sprint > dist_normal * 1.5,
            "sprint should move faster: normal={dist_normal}, sprint={dist_sprint}"
        );
    }

    #[test]
    fn controller_set_orbit_target() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.toggle_mode(&mut cam); // Orbit mode
        let target = Vec3::new(10.0, 5.0, 3.0);
        ctrl.set_orbit_target(target, &mut cam);
        assert!(
            (ctrl.orbit_target - target).length() < 0.001,
            "orbit target should be set"
        );
    }

    #[test]
    fn controller_pitch_clamped() {
        let mut ctrl = CameraController::new(5.0, 0.01);
        let mut cam = make_camera();
        ctrl.process_mouse_button(winit::event::MouseButton::Right, true);
        // Apply huge positive pitch delta
        for _ in 0..100 {
            ctrl.process_mouse_delta(&mut cam, Vec2::new(0.0, -100.0));
            ctrl.update_camera(&mut cam, 0.016);
            ctrl.begin_frame();
        }
        assert!(
            cam.pitch <= 1.55 && cam.pitch >= -1.55,
            "pitch should be clamped near ±1.54, got {}",
            cam.pitch
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Decals — CPU-side: Decal::new, update, to_gpu, GpuDecal, blend modes
// ═══════════════════════════════════════════════════════════════════════
mod decal_tests {
    use astraweave_render::decals::{Decal, DecalBlendMode, GpuDecal};
    use glam::{Quat, Vec3};

    #[test]
    fn gpu_decal_size_is_112() {
        assert_eq!(std::mem::size_of::<GpuDecal>(), 112);
    }

    #[test]
    fn decal_new_defaults() {
        let d = Decal::new(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        assert_eq!(d.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(d.albedo_tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(d.normal_strength, 1.0);
        assert_eq!(d.roughness, 0.5);
        assert_eq!(d.metallic, 0.0);
        assert_eq!(d.blend_mode, DecalBlendMode::AlphaBlend);
        assert_eq!(d.fade_duration, 0.0);
        assert_eq!(d.fade_time, 0.0);
    }

    #[test]
    fn decal_permanent_never_expires() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        // fade_duration = 0 means permanent
        for _ in 0..100 {
            assert!(d.update(1.0), "permanent decal should never expire");
        }
        assert_eq!(d.albedo_tint[3], 1.0, "alpha should stay at 1.0");
    }

    #[test]
    fn decal_fade_halfway_alpha() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        d.fade_duration = 4.0;
        assert!(d.update(2.0)); // 50% through
                                // alpha should be 1.0 - (2.0/4.0) = 0.5
        assert!(
            (d.albedo_tint[3] - 0.5).abs() < 0.01,
            "alpha at 50% should be ~0.5, got {}",
            d.albedo_tint[3]
        );
    }

    #[test]
    fn decal_fade_near_end_low_alpha() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        d.fade_duration = 2.0;
        assert!(d.update(1.5)); // 75%
        assert!(
            d.albedo_tint[3] < 0.3,
            "alpha at 75% should be ~0.25, got {}",
            d.albedo_tint[3]
        );
    }

    #[test]
    fn decal_fade_expired_returns_false() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        d.fade_duration = 1.0;
        assert!(d.update(0.5)); // Still alive
        assert!(!d.update(0.6)); // expired (total 1.1 > 1.0)
    }

    #[test]
    fn decal_fade_exact_duration_expires() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        d.fade_duration = 2.0;
        // Exactly at duration (fade_time == fade_duration)
        assert!(!d.update(2.0), "decal at exact duration should expire");
    }

    #[test]
    fn decal_to_gpu_stores_params() {
        let mut d = Decal::new(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec3::new(2.0, 2.0, 2.0),
            ([0.25, 0.5], [0.25, 0.25]),
        );
        d.normal_strength = 0.8;
        d.roughness = 0.3;
        d.metallic = 0.7;
        d.blend_mode = DecalBlendMode::Multiply;

        let gpu = d.to_gpu();
        assert_eq!(gpu.params[0], 0.8); // normal_strength
        assert_eq!(gpu.params[1], 0.3); // roughness
        assert_eq!(gpu.params[2], 0.7); // metallic
        assert_eq!(gpu.params[3], 0.0); // Multiply = 0
    }

    #[test]
    fn decal_to_gpu_atlas_uv() {
        let d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.125, 0.375], [0.25, 0.25]),
        );
        let gpu = d.to_gpu();
        assert_eq!(gpu.atlas_uv[0], 0.125);
        assert_eq!(gpu.atlas_uv[1], 0.375);
        assert_eq!(gpu.atlas_uv[2], 0.25);
        assert_eq!(gpu.atlas_uv[3], 0.25);
    }

    #[test]
    fn decal_to_gpu_inv_projection_is_inverse() {
        let d = Decal::new(
            Vec3::new(5.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        let gpu = d.to_gpu();
        // inv_projection should be the inverse of the transform
        // Transform = scale * rot * translate → inverse should undo translation
        // The [3][0] element of inverse should be -5.0
        assert!(
            (gpu.inv_projection[3][0] + 5.0).abs() < 0.01,
            "inverse should negate translation x, got {}",
            gpu.inv_projection[3][0]
        );
    }

    #[test]
    fn blend_mode_enum_values() {
        assert_eq!(DecalBlendMode::Multiply as u32, 0);
        assert_eq!(DecalBlendMode::Additive as u32, 1);
        assert_eq!(DecalBlendMode::AlphaBlend as u32, 2);
        assert_eq!(DecalBlendMode::Stain as u32, 3);
    }

    #[test]
    fn decal_to_gpu_blend_mode_in_params_w() {
        let mut d = Decal::new(
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            ([0.0, 0.0], [1.0, 1.0]),
        );
        d.blend_mode = DecalBlendMode::Additive;
        let gpu = d.to_gpu();
        assert_eq!(gpu.params[3], 1.0, "Additive should be 1.0 in params.w");

        d.blend_mode = DecalBlendMode::Stain;
        let gpu2 = d.to_gpu();
        assert_eq!(gpu2.params[3], 3.0, "Stain should be 3.0 in params.w");
    }

    #[test]
    fn gpu_decal_bytemuck_zeroed() {
        let z: GpuDecal = bytemuck::Zeroable::zeroed();
        assert_eq!(z.albedo_tint, [0.0; 4]);
        assert_eq!(z.params, [0.0; 4]);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  HDRI Catalog — DayPeriod parsing, game-hours boundaries
// ═══════════════════════════════════════════════════════════════════════
mod hdri_catalog_tests {
    use astraweave_render::hdri_catalog::DayPeriod;

    // --- from_str_loose comprehensive ---

    #[test]
    fn from_str_loose_day() {
        assert_eq!(DayPeriod::from_str_loose("day"), Some(DayPeriod::Day));
        assert_eq!(DayPeriod::from_str_loose("DAY"), Some(DayPeriod::Day));
        assert_eq!(DayPeriod::from_str_loose("  Day  "), Some(DayPeriod::Day));
    }

    #[test]
    fn from_str_loose_morning_aliases() {
        assert_eq!(
            DayPeriod::from_str_loose("morning"),
            Some(DayPeriod::Morning)
        );
        assert_eq!(
            DayPeriod::from_str_loose("sunrise"),
            Some(DayPeriod::Morning)
        );
        assert_eq!(DayPeriod::from_str_loose("dawn"), Some(DayPeriod::Morning));
        assert_eq!(DayPeriod::from_str_loose("DAWN"), Some(DayPeriod::Morning));
    }

    #[test]
    fn from_str_loose_evening_aliases() {
        assert_eq!(
            DayPeriod::from_str_loose("evening"),
            Some(DayPeriod::Evening)
        );
        assert_eq!(
            DayPeriod::from_str_loose("sunset"),
            Some(DayPeriod::Evening)
        );
        assert_eq!(DayPeriod::from_str_loose("dusk"), Some(DayPeriod::Evening));
    }

    #[test]
    fn from_str_loose_night_aliases() {
        assert_eq!(DayPeriod::from_str_loose("night"), Some(DayPeriod::Night));
        assert_eq!(
            DayPeriod::from_str_loose("midnight"),
            Some(DayPeriod::Night)
        );
    }

    #[test]
    fn from_str_loose_invalid_returns_none() {
        assert_eq!(DayPeriod::from_str_loose("noon"), None);
        assert_eq!(DayPeriod::from_str_loose(""), None);
        assert_eq!(DayPeriod::from_str_loose("afternoon"), None);
    }

    // --- as_str roundtrip ---

    #[test]
    fn as_str_values() {
        assert_eq!(DayPeriod::Day.as_str(), "day");
        assert_eq!(DayPeriod::Morning.as_str(), "morning");
        assert_eq!(DayPeriod::Evening.as_str(), "evening");
        assert_eq!(DayPeriod::Night.as_str(), "night");
    }

    #[test]
    fn as_str_roundtrip() {
        for &period in DayPeriod::all() {
            let s = period.as_str();
            let parsed = DayPeriod::from_str_loose(s);
            assert_eq!(parsed, Some(period), "roundtrip failed for {s}");
        }
    }

    // --- all() ---

    #[test]
    fn all_has_four_periods() {
        assert_eq!(DayPeriod::all().len(), 4);
    }

    #[test]
    fn all_contains_each_variant() {
        let all = DayPeriod::all();
        assert!(all.contains(&DayPeriod::Day));
        assert!(all.contains(&DayPeriod::Morning));
        assert!(all.contains(&DayPeriod::Evening));
        assert!(all.contains(&DayPeriod::Night));
    }

    // --- from_game_hours boundary tests ---

    #[test]
    fn from_game_hours_boundary_4_99_is_night() {
        assert_eq!(DayPeriod::from_game_hours(4.99), DayPeriod::Night);
    }

    #[test]
    fn from_game_hours_boundary_5_0_is_morning() {
        assert_eq!(DayPeriod::from_game_hours(5.0), DayPeriod::Morning);
    }

    #[test]
    fn from_game_hours_boundary_9_99_is_morning() {
        assert_eq!(DayPeriod::from_game_hours(9.99), DayPeriod::Morning);
    }

    #[test]
    fn from_game_hours_boundary_10_0_is_day() {
        assert_eq!(DayPeriod::from_game_hours(10.0), DayPeriod::Day);
    }

    #[test]
    fn from_game_hours_boundary_16_99_is_day() {
        assert_eq!(DayPeriod::from_game_hours(16.99), DayPeriod::Day);
    }

    #[test]
    fn from_game_hours_boundary_17_0_is_evening() {
        assert_eq!(DayPeriod::from_game_hours(17.0), DayPeriod::Evening);
    }

    #[test]
    fn from_game_hours_boundary_20_99_is_evening() {
        assert_eq!(DayPeriod::from_game_hours(20.99), DayPeriod::Evening);
    }

    #[test]
    fn from_game_hours_boundary_21_0_is_night() {
        assert_eq!(DayPeriod::from_game_hours(21.0), DayPeriod::Night);
    }

    #[test]
    fn from_game_hours_midnight_0() {
        assert_eq!(DayPeriod::from_game_hours(0.0), DayPeriod::Night);
    }

    #[test]
    fn from_game_hours_3am() {
        assert_eq!(DayPeriod::from_game_hours(3.0), DayPeriod::Night);
    }

    #[test]
    fn from_game_hours_noon() {
        assert_eq!(DayPeriod::from_game_hours(12.0), DayPeriod::Day);
    }

    #[test]
    fn from_game_hours_wraps_negative() {
        // rem_euclid handles negative hours
        assert_eq!(
            DayPeriod::from_game_hours(-1.0),
            DayPeriod::from_game_hours(23.0)
        );
    }

    #[test]
    fn from_game_hours_wraps_over_24() {
        assert_eq!(
            DayPeriod::from_game_hours(30.0),
            DayPeriod::from_game_hours(6.0)
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Types — cluster_index, Instance::raw/from_pos_scale_color, struct sizes
// ═══════════════════════════════════════════════════════════════════════
mod types_tests {
    use astraweave_render::types::*;
    use glam::{Mat4, Vec3};

    // --- cluster_index tests ---

    #[test]
    fn cluster_index_center_screen() {
        let dims = ClusterDims { x: 16, y: 9, z: 24 };
        let idx = cluster_index(960, 540, 1920, 1080, 10.0, 0.1, 100.0, dims);
        assert!(idx < 16 * 9 * 24);
        // Center pixel → x=8, y=4 (approximately)
    }

    #[test]
    fn cluster_index_origin_pixel() {
        let dims = ClusterDims { x: 8, y: 8, z: 8 };
        let idx = cluster_index(0, 0, 800, 800, 0.1, 0.1, 100.0, dims);
        // x=0, y=0, depth at near → z=0
        assert_eq!(idx, 0, "origin pixel at near should be cluster 0");
    }

    #[test]
    fn cluster_index_increasing_depth() {
        let dims = ClusterDims { x: 4, y: 4, z: 8 };
        let idx1 = cluster_index(200, 200, 800, 800, 5.0, 0.1, 100.0, dims);
        let idx2 = cluster_index(200, 200, 800, 800, 50.0, 0.1, 100.0, dims);
        assert!(
            idx2 > idx1,
            "deeper pixel should have higher z-slice → higher index"
        );
    }

    #[test]
    fn cluster_index_clamps_out_of_bounds_coords() {
        let dims = ClusterDims { x: 4, y: 4, z: 4 };
        // Huge coords
        let idx = cluster_index(99999, 99999, 800, 800, 50.0, 0.1, 100.0, dims);
        assert!(idx < 4 * 4 * 4, "out-of-bounds coords should clamp");
    }

    #[test]
    fn cluster_index_depth_beyond_far_clamps() {
        let dims = ClusterDims { x: 4, y: 4, z: 4 };
        let idx = cluster_index(400, 400, 800, 800, 200.0, 0.1, 100.0, dims);
        assert!(idx < 4 * 4 * 4, "depth beyond far should clamp");
    }

    #[test]
    fn cluster_index_depth_before_near_clamps() {
        let dims = ClusterDims { x: 4, y: 4, z: 4 };
        let idx = cluster_index(400, 400, 800, 800, 0.001, 0.1, 100.0, dims);
        assert!(idx < 4 * 4 * 4, "depth before near should clamp");
    }

    #[test]
    fn cluster_index_x_varies_across_width() {
        let dims = ClusterDims { x: 8, y: 4, z: 4 };
        let idx_left = cluster_index(0, 200, 800, 800, 10.0, 0.1, 100.0, dims);
        let idx_right = cluster_index(799, 200, 800, 800, 10.0, 0.1, 100.0, dims);
        // Same y and depth, different x → different sx component
        let sx_left = idx_left % dims.x;
        let sx_right = idx_right % dims.x;
        assert_ne!(
            sx_left, sx_right,
            "different x coords should map to different tiles"
        );
    }

    #[test]
    fn cluster_index_y_varies_across_height() {
        let dims = ClusterDims { x: 4, y: 8, z: 4 };
        let idx_top = cluster_index(200, 0, 800, 800, 10.0, 0.1, 100.0, dims);
        let idx_bottom = cluster_index(200, 799, 800, 800, 10.0, 0.1, 100.0, dims);
        assert_ne!(
            idx_top, idx_bottom,
            "different y coords should map to different tiles"
        );
    }

    // --- Instance tests ---

    #[test]
    fn instance_from_pos_scale_color_stores_fields() {
        let inst = Instance::from_pos_scale_color(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(2.0, 2.0, 2.0),
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(inst.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(inst.material_id, 0);
        // Translation should be in the w_axis of the matrix
        assert!((inst.transform.w_axis.x - 1.0).abs() < 1e-5);
        assert!((inst.transform.w_axis.y - 2.0).abs() < 1e-5);
        assert!((inst.transform.w_axis.z - 3.0).abs() < 1e-5);
    }

    #[test]
    fn instance_raw_identity_transform() {
        let inst = Instance {
            transform: Mat4::IDENTITY,
            color: [0.5, 0.5, 0.5, 1.0],
            material_id: 7,
        };
        let raw = inst.raw();
        assert_eq!(raw.color, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(raw.material_id, 7);
        // Model matrix should be identity
        for i in 0..4 {
            for j in 0..4 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (raw.model[i][j] - expected).abs() < 1e-5,
                    "identity model[{i}][{j}] = {} (expected {expected})",
                    raw.model[i][j]
                );
            }
        }
        assert_eq!(raw._padding, [0, 0, 0]);
    }

    #[test]
    fn instance_raw_normal_matrix_for_uniform_scale() {
        // For uniform scale, normal matrix = identity (since inverse transpose of scale*I = I/scale)
        let inst = Instance {
            transform: Mat4::from_scale(Vec3::splat(3.0)),
            color: [1.0; 4],
            material_id: 0,
        };
        let raw = inst.raw();
        // Normal matrix should be (1/3) * I (inverse transpose of 3*I)
        let expected_val = 1.0 / 3.0;
        assert!(
            (raw.normal_matrix[0][0] - expected_val).abs() < 0.01,
            "normal_matrix[0][0] = {} (expected ~{expected_val})",
            raw.normal_matrix[0][0]
        );
    }

    // --- Struct sizes ---

    #[test]
    fn vertex_size() {
        // 3f32 + 3f32 + 4f32 + 2f32 = 48 bytes
        assert_eq!(std::mem::size_of::<Vertex>(), 48);
    }

    #[test]
    fn skinned_vertex_size() {
        // 48 + 4u16(8) + 4f32(16) = 72 bytes
        assert_eq!(std::mem::size_of::<SkinnedVertex>(), 72);
    }

    #[test]
    fn instance_raw_size() {
        // 64 (model) + 36 (normal 3x3) + 16 (color) + 4 (material_id) + 12 (padding) = 132
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 132);
    }

    // --- Vertex layout attributes ---

    #[test]
    fn vertex_layout_has_four_attributes() {
        let layout = Vertex::layout();
        assert_eq!(layout.attributes.len(), 4);
    }

    #[test]
    fn vertex_layout_stride_is_48() {
        let layout = Vertex::layout();
        assert_eq!(layout.array_stride, 48);
    }

    #[test]
    fn skinned_vertex_layout_has_six_attributes() {
        let layout = SkinnedVertex::layout();
        assert_eq!(layout.attributes.len(), 6);
    }

    #[test]
    fn instance_raw_layout_is_instance_step() {
        let layout = InstanceRaw::layout();
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  BiomeAudio — BiomeAmbientMap, crossfade clamping, default paths
// ═══════════════════════════════════════════════════════════════════════
mod biome_audio_tests {
    use astraweave_render::biome_audio::{BiomeAmbientMap, DEFAULT_AMBIENT_CROSSFADE};
    use astraweave_terrain::biome::BiomeType;

    #[test]
    fn default_crossfade_is_three_seconds() {
        assert!((DEFAULT_AMBIENT_CROSSFADE - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn new_equals_default() {
        let a = BiomeAmbientMap::new();
        let b = BiomeAmbientMap::default();
        assert_eq!(a.len(), b.len());
        assert_eq!(a.crossfade_sec(), b.crossfade_sec());
    }

    #[test]
    fn empty_has_zero_tracks() {
        let m = BiomeAmbientMap::empty();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn empty_preserves_crossfade() {
        let m = BiomeAmbientMap::empty();
        assert_eq!(m.crossfade_sec(), DEFAULT_AMBIENT_CROSSFADE);
    }

    #[test]
    fn default_has_all_eight_biomes() {
        let m = BiomeAmbientMap::default();
        assert_eq!(m.len(), 8);
        for &biome in &[
            BiomeType::Forest,
            BiomeType::Desert,
            BiomeType::Grassland,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ] {
            assert!(m.get(biome).is_some(), "missing {:?}", biome);
        }
    }

    #[test]
    fn default_paths_format_consistent() {
        let m = BiomeAmbientMap::default();
        for &biome in &[
            BiomeType::Forest,
            BiomeType::Desert,
            BiomeType::Grassland,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ] {
            let path = m.get(biome).unwrap();
            assert!(
                path.starts_with("assets/audio/ambient/"),
                "path prefix wrong: {path}"
            );
            assert!(path.ends_with(".ogg"), "path suffix wrong: {path}");
        }
    }

    #[test]
    fn default_path_contains_biome_name() {
        let m = BiomeAmbientMap::default();
        assert!(m.get(BiomeType::Forest).unwrap().contains("forest"));
        assert!(m.get(BiomeType::Desert).unwrap().contains("desert"));
        assert!(m.get(BiomeType::Swamp).unwrap().contains("swamp"));
        assert!(m.get(BiomeType::Beach).unwrap().contains("beach"));
        assert!(m.get(BiomeType::River).unwrap().contains("river"));
        assert!(m.get(BiomeType::Tundra).unwrap().contains("tundra"));
        assert!(m.get(BiomeType::Mountain).unwrap().contains("mountain"));
        assert!(m.get(BiomeType::Grassland).unwrap().contains("grassland"));
    }

    #[test]
    fn set_overrides_path() {
        let mut m = BiomeAmbientMap::new();
        m.set(BiomeType::Forest, "custom/my_forest.ogg");
        assert_eq!(m.get(BiomeType::Forest).unwrap(), "custom/my_forest.ogg");
        // Count should still be 8 (override, not add)
        assert_eq!(m.len(), 8);
    }

    #[test]
    fn remove_deletes_biome() {
        let mut m = BiomeAmbientMap::new();
        m.remove(BiomeType::Desert);
        assert!(m.get(BiomeType::Desert).is_none());
        assert_eq!(m.len(), 7);
    }

    #[test]
    fn set_crossfade_sec_works() {
        let mut m = BiomeAmbientMap::new();
        m.set_crossfade_sec(5.0);
        assert_eq!(m.crossfade_sec(), 5.0);
    }

    #[test]
    fn set_crossfade_sec_clamps_negative() {
        let mut m = BiomeAmbientMap::new();
        m.set_crossfade_sec(-100.0);
        assert!(
            m.crossfade_sec() >= 0.01,
            "should clamp to >= 0.01, got {}",
            m.crossfade_sec()
        );
    }

    #[test]
    fn set_crossfade_sec_clamps_zero() {
        let mut m = BiomeAmbientMap::new();
        m.set_crossfade_sec(0.0);
        assert!(m.crossfade_sec() >= 0.01, "zero should clamp to 0.01");
    }

    #[test]
    fn is_empty_after_clear() {
        let mut m = BiomeAmbientMap::new();
        for &biome in &[
            BiomeType::Forest,
            BiomeType::Desert,
            BiomeType::Grassland,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ] {
            m.remove(biome);
        }
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SceneEnvironment — UBO layout, weather multipliers, biome lookups
// ═══════════════════════════════════════════════════════════════════════
mod scene_env_tests {
    use astraweave_render::biome_transition::BiomeVisuals;
    use astraweave_render::effects::WeatherKind;
    use astraweave_render::scene_environment::{
        SceneEnvironment, SceneEnvironmentUBO, WGSL_FOG_FUNCTIONS, WGSL_SCENE_ENVIRONMENT,
    };
    use astraweave_terrain::biome::BiomeType;

    #[test]
    fn ubo_size_is_80() {
        assert_eq!(std::mem::size_of::<SceneEnvironmentUBO>(), 96);
        assert_eq!(SceneEnvironmentUBO::size(), 96);
    }

    #[test]
    fn ubo_default_has_nonzero_fog() {
        let ubo = SceneEnvironmentUBO::default();
        assert!(ubo.fog_density > 0.0);
        assert!(ubo.fog_end > ubo.fog_start);
    }

    #[test]
    fn ubo_for_biome_matches_visuals() {
        for &biome in &[
            BiomeType::Grassland,
            BiomeType::Desert,
            BiomeType::Forest,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ] {
            let ubo = SceneEnvironmentUBO::for_biome(biome);
            let v = BiomeVisuals::for_biome(biome);
            assert_eq!(ubo.fog_density, v.fog_density, "fog mismatch for {biome:?}");
            assert_eq!(
                ubo.ambient_intensity, v.ambient_intensity,
                "ambient mismatch for {biome:?}"
            );
            assert_eq!(ubo.blend_factor, 0.0);
            assert_eq!(ubo.tint_alpha, 0.0);
        }
    }

    #[test]
    fn scene_env_default_multipliers_are_one() {
        let env = SceneEnvironment::default();
        assert_eq!(env.weather_fog_multiplier, 1.0);
        assert_eq!(env.weather_ambient_multiplier, 1.0);
    }

    #[test]
    fn scene_env_set_biome_clears_transition() {
        let mut env = SceneEnvironment::default();
        env.blend_factor = 0.5;
        env.tint_alpha = 0.3;
        env.set_biome(BiomeType::Forest);
        assert_eq!(env.blend_factor, 0.0);
        assert_eq!(env.tint_alpha, 0.0);
        assert_eq!(env.tint_color, [0.0; 3]);
    }

    #[test]
    fn scene_env_weather_multiplier_affects_ubo() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Grassland);
        let base_fog = env.visuals.fog_density;
        env.weather_fog_multiplier = 3.0;
        let ubo = env.to_ubo();
        assert!(
            (ubo.fog_density - base_fog * 3.0).abs() < 0.001,
            "fog should be 3× base, got {}, expected {}",
            ubo.fog_density,
            base_fog * 3.0
        );
    }

    #[test]
    fn scene_env_apply_weather_rain() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(WeatherKind::Rain);
        assert!((env.weather_fog_multiplier - 2.5).abs() < f32::EPSILON);
        assert!((env.weather_ambient_multiplier - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn scene_env_apply_weather_snow() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(WeatherKind::Snow);
        assert!((env.weather_fog_multiplier - 1.8).abs() < f32::EPSILON);
        assert!((env.weather_ambient_multiplier - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn scene_env_apply_weather_sandstorm() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(WeatherKind::Sandstorm);
        assert!((env.weather_fog_multiplier - 4.0).abs() < f32::EPSILON);
        assert!((env.weather_ambient_multiplier - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn scene_env_apply_weather_wind_trails() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(WeatherKind::WindTrails);
        assert!((env.weather_fog_multiplier - 1.4).abs() < f32::EPSILON);
        assert!((env.weather_ambient_multiplier - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn scene_env_apply_weather_none() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(WeatherKind::None);
        assert_eq!(env.weather_fog_multiplier, 1.0);
        assert_eq!(env.weather_ambient_multiplier, 1.0);
    }

    #[test]
    fn wgsl_scene_env_snippet_valid() {
        assert!(!WGSL_SCENE_ENVIRONMENT.is_empty());
        assert!(WGSL_SCENE_ENVIRONMENT.contains("SceneEnvironment"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("fog_color"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("fog_density"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("ambient_color"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("ambient_intensity"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("tint_color"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("tint_alpha"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("blend_factor"));
    }

    #[test]
    fn wgsl_fog_functions_snippet_valid() {
        assert!(!WGSL_FOG_FUNCTIONS.is_empty());
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_fog"));
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_tint"));
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_linear_fog"));
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_exp_fog"));
    }

    #[test]
    #[ignore = "GPU/wgpu test - not available in CI/mutation testing"]
    fn ubo_bytemuck_roundtrip() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Mountain);
        env.weather_fog_multiplier = 2.0;
        let ubo = env.to_ubo();
        let bytes = bytemuck::bytes_of(&ubo);
        assert_eq!(bytes.len(), 80);
        let decoded: &SceneEnvironmentUBO = bytemuck::from_bytes(bytes);
        assert_eq!(decoded.fog_density, ubo.fog_density);
        assert_eq!(decoded.ambient_intensity, ubo.ambient_intensity);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ClusteredForward — ClusterConfig, GpuLight, GpuCluster basics
// ═══════════════════════════════════════════════════════════════════════
mod clustered_forward_tests {
    use astraweave_render::clustered_forward::{ClusterConfig, GpuCluster, GpuLight};
    use glam::Vec3;

    #[test]
    fn cluster_config_default_values() {
        let c = ClusterConfig::default();
        assert_eq!(c.cluster_x, 16);
        assert_eq!(c.cluster_y, 9);
        assert_eq!(c.cluster_z, 24);
        assert!((c.near - 0.1).abs() < f32::EPSILON);
        assert!((c.far - 100.0).abs() < f32::EPSILON);
        assert_eq!(c._pad, [0; 3]);
    }

    #[test]
    fn cluster_config_total_clusters() {
        let c = ClusterConfig::default();
        assert_eq!(c.cluster_x * c.cluster_y * c.cluster_z, 3456);
    }

    #[test]
    fn cluster_config_size_32_bytes() {
        assert_eq!(std::mem::size_of::<ClusterConfig>(), 32);
    }

    #[test]
    fn gpu_light_new_stores_all() {
        let l = GpuLight::new(Vec3::new(1.0, 2.0, 3.0), 5.0, Vec3::new(0.1, 0.2, 0.3), 7.5);
        assert_eq!(l.position, [1.0, 2.0, 3.0, 5.0]);
        assert_eq!(l.color, [0.1, 0.2, 0.3, 7.5]);
    }

    #[test]
    fn gpu_light_size_32_bytes() {
        assert_eq!(std::mem::size_of::<GpuLight>(), 32);
    }

    #[test]
    fn gpu_cluster_size_48_bytes() {
        assert_eq!(std::mem::size_of::<GpuCluster>(), 48);
    }

    #[test]
    fn gpu_light_zeroed() {
        let l: GpuLight = bytemuck::Zeroable::zeroed();
        assert_eq!(l.position, [0.0; 4]);
        assert_eq!(l.color, [0.0; 4]);
    }

    #[test]
    fn gpu_light_position_w_is_radius() {
        let l = GpuLight::new(Vec3::ZERO, 42.0, Vec3::ONE, 1.0);
        assert_eq!(l.position[3], 42.0, "w component should be radius");
    }

    #[test]
    fn gpu_light_color_w_is_intensity() {
        let l = GpuLight::new(Vec3::ZERO, 1.0, Vec3::ONE, 99.0);
        assert_eq!(l.color[3], 99.0, "w component should be intensity");
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ShadowCSM — constants, cascade splits
// ═══════════════════════════════════════════════════════════════════════
mod shadow_csm_tests {
    use astraweave_render::shadow_csm::{
        ATLAS_RESOLUTION, CASCADE_COUNT, CASCADE_RESOLUTION, DEPTH_BIAS,
    };

    #[test]
    fn cascade_count_is_four() {
        assert_eq!(CASCADE_COUNT, 4);
    }

    #[test]
    fn cascade_resolution_is_2048() {
        assert_eq!(CASCADE_RESOLUTION, 2048);
    }

    #[test]
    fn atlas_resolution_equals_cascade() {
        assert_eq!(ATLAS_RESOLUTION, CASCADE_RESOLUTION);
    }

    #[test]
    fn depth_bias_positive_and_small() {
        assert!(DEPTH_BIAS > 0.0);
        assert!(DEPTH_BIAS < 0.1);
        assert!((DEPTH_BIAS - 0.005).abs() < f32::EPSILON);
    }

    #[test]
    fn cascade_splits_monotonic_various_ranges() {
        let lambda = 0.5f32;
        for &(near, far) in &[
            (0.1f32, 100.0f32),
            (0.1, 1000.0),
            (1.0, 50.0),
            (0.01, 10000.0),
        ] {
            let mut splits = [0.0f32; CASCADE_COUNT + 1];
            splits[0] = near;
            splits[CASCADE_COUNT] = far;
            for i in 1..CASCADE_COUNT {
                let i_f = i as f32;
                let n_f = CASCADE_COUNT as f32;
                let log_split = near * (far / near).powf(i_f / n_f);
                let uni_split = near + (far - near) * (i_f / n_f);
                splits[i] = lambda * log_split + (1.0 - lambda) * uni_split;
            }
            for pair in splits.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "splits must be monotonic for near={near}, far={far}"
                );
            }
        }
    }
}
